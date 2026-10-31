//! Selected projects/workflow fixture publishers share the existing custody owner.
use chrono_worktree::{RetainedArtifact, TemporaryHost};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};

const PRODUCER: &str = "projects-test-host";
static HOSTS: OnceLock<Result<(PathBuf, Vec<String>), String>> = OnceLock::new();
static TOOLS: OnceLock<Result<(PathBuf, Vec<String>), String>> = OnceLock::new();
static MIGRATIONS: OnceLock<Result<(PathBuf, Vec<String>), String>> = OnceLock::new();

fn store(producer: &str) -> &'static (PathBuf, Vec<String>) {
    let selected = match producer {
        PRODUCER => &HOSTS,
        "workflow-test-tools" => &TOOLS,
        "projects-migration-host" => &MIGRATIONS,
        _ => panic!("undeclared fixture producer: {producer}"),
    };
    selected
        .get_or_init(|| {
            let source = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap();
            TemporaryHost::registered_store(source, ".chrono-harness/worktree.json", producer)
        })
        .as_ref()
        .unwrap_or_else(|error| panic!("fixture custody prerequisite: {error}"))
}

pub struct FixtureDirectory {
    host: TemporaryHost,
    producer: &'static str,
    root: PathBuf,
}
impl FixtureDirectory {
    pub fn path(&self) -> &Path {
        &self.root
    }
    pub fn temporary_path(&self) -> &Path {
        self.host.path()
    }
    pub fn capture_output(&self, command: &mut Command) -> Result<Output, String> {
        self.host.capture_output(command)
    }
    pub fn capture_input(&self, command: &mut Command, input: &[u8]) -> Result<Output, String> {
        self.host.bind_command(command)?;
        TemporaryHost::capture_bound_input(self.host.path(), command, input)
    }
    pub fn retain_diagnostic(&self, reason: &str) -> Result<(), String> {
        retain_diagnostic(self.host.path(), reason)
    }
    pub fn make_checkout_unavailable(&self) -> std::io::Result<PathBuf> {
        let retained = self.host.path().join("retained-offline-checkout");
        if retained.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "offline source already retained",
            ));
        }
        TemporaryHost::relocate_copies(
            &store(self.producer).0,
            self.producer,
            &self.root,
            &retained,
        )
        .map_err(std::io::Error::other)?;
        fs::rename(&self.root, &retained)?;
        Ok(retained)
    }
    pub fn close(self) -> std::io::Result<()> {
        self.make_checkout_unavailable().map(|_| ())
    }
}
impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        if std::thread::panicking()
            || std::env::var_os("CHRONO_FULL_TEST_RECEIPTS").is_some()
            || std::env::var_os("CHRONO_COLLECTION_REPAIR_EVIDENCE").is_some()
        {
            if let Err(error) = self.retain_diagnostic("failed fixture or requested original-evidence consumer; explicit diagnostic release required") {
                eprintln!(
                    "fixture diagnostic reference unavailable at {}: {error}",
                    self.host.path().display()
                );
            }
        }
    }
}
pub fn fixture_directory(prefix: &str) -> FixtureDirectory {
    allocate(PRODUCER, prefix)
}
pub fn tool_directory(prefix: &str) -> FixtureDirectory {
    allocate("workflow-test-tools", prefix)
}
pub fn migration_directory(prefix: &str) -> FixtureDirectory {
    allocate("projects-migration-host", prefix)
}
fn allocate(producer: &'static str, prefix: &str) -> FixtureDirectory {
    let (directory, outputs) = store(producer);
    let host = TemporaryHost::allocate(directory, producer, outputs, prefix).unwrap();
    let root = host.path().join("fixture");
    fs::create_dir(&root).unwrap();
    FixtureDirectory {
        host,
        producer,
        root,
    }
}

/// Configure arguments/environment before this single native binding.
/// Existing thread scopes supply held allocations; Command owns its copies.
pub fn capture_native(root: &Path, command: &mut Command) -> Output {
    let (_, directory) = binding(root).unwrap();
    let relative = root.strip_prefix(directory).unwrap();
    let allocation = directory.join(relative.components().next().unwrap().as_os_str());
    // Clone destinations must remain empty until the real Git clone. Captures
    // and incidental temporary files belong beside that checkout, not inside it.
    command.env("TMPDIR", &allocation);
    chrono_harness::process_fds::forward_command(command, &[]).unwrap();
    TemporaryHost::capture_bound_output(&allocation, command).unwrap()
}

/// Only explicit executable-copy publishers call this; evidence copies do not.
pub fn copied_fixture_input(root: &Path, source: &Path, destination: &Path) {
    let (producer, directory) = binding(destination).unwrap();
    TemporaryHost::register_copy(
        directory,
        producer,
        source,
        destination,
        Some(root),
        "actual projects/workflow fixture executable copy or hard link",
    )
    .unwrap();
}

pub fn retain_diagnostic(path: &Path, reason: &str) -> Result<(), String> {
    let (producer, directory) = binding(path)?;
    let relative = path
        .strip_prefix(directory)
        .map_err(|error| error.to_string())?;
    let allocation = relative
        .components()
        .next()
        .ok_or("diagnostic allocation missing")?;
    RetainedArtifact::resume(directory, producer, &directory.join(allocation.as_os_str()))?
        .acquire("fixture-diagnostic", reason)
}

// Consult only stores actually initialized by these explicit publishers. The
// owner still verifies exact allocation membership; no filesystem discovery.
fn binding(path: &Path) -> Result<(&'static str, &'static Path), String> {
    for (producer, selected) in [
        (PRODUCER, &HOSTS),
        ("workflow-test-tools", &TOOLS),
        ("projects-migration-host", &MIGRATIONS),
    ] {
        if let Some(Ok((directory, _))) = selected.get() {
            if path.starts_with(directory) {
                return Ok((producer, directory));
            }
        }
    }
    Err(format!(
        "path has no initialized fixture publisher: {}",
        path.display()
    ))
}
