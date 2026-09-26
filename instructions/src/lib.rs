//! Explicit host instruction generation. No judge execution or host discovery.

mod transaction;

use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use transaction::{Change, Fault, Snapshot};

const DIRECTORY: &str = ".chrono-harness/instructions";
const MANIFEST: &str = ".chrono-harness/instructions/manifest.json";
const METHOD: &str = ".chrono-harness/instructions/methodology.md";
const CONTEXT: &str = ".chrono-harness/instructions/host-context.md";
const BEGIN: &[u8] = b"<!-- chrono-instructions:begin -->";
const END: &[u8] = b"<!-- chrono-instructions:end -->";
const RESERVED: &[u8] = b"<!-- chrono-instructions";
const FRAME: &str = include_str!("../../assets/entrypoint.md");
const DEFAULT_METHOD: &str = include_str!("../../assets/methodology.md");

struct InitSources {
    methodology: Option<Vec<u8>>,
    context: Option<Vec<u8>>,
}

impl InitSources {
    fn read(methodology: Option<&Path>, context: Option<&Path>) -> Result<Self, String> {
        Ok(Self {
            methodology: methodology.map(input).transpose()?,
            context: context.map(input).transpose()?,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Source {
    role: String,
    path: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Registration {
    schema_version: u32,
    producer: String,
    render: String,
    sources: Vec<Source>,
    outputs: Vec<Source>,
}

fn registration() -> Registration {
    let source = |role: &str, path: &str| Source {
        role: role.into(),
        path: path.into(),
    };
    Registration {
        schema_version: 1,
        producer: concat!("chrono-instructions/", env!("CARGO_PKG_VERSION")).into(),
        render: "literal-core/relative-alias/v2".into(),
        sources: vec![
            source("methodology", METHOD),
            source("host-context", CONTEXT),
        ],
        outputs: vec![
            source("claude-guide", "CLAUDE.md"),
            source("agents-relative-alias", "AGENTS.md"),
        ],
    }
}

// Only this exact earlier identity is a forward migration input.
fn legacy_registration() -> Registration {
    let mut old = registration();
    old.producer = "chrono-instructions/0.1.0".into();
    old.render = "read-both/v1".into();
    old.outputs = vec![
        Source {
            role: "agents-entrypoint".into(),
            path: "AGENTS.md".into(),
        },
        Source {
            role: "claude-entrypoint".into(),
            path: "CLAUDE.md".into(),
        },
    ];
    old
}

fn manifest_bytes() -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(&registration()).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn managed_block(method: &[u8]) -> Vec<u8> {
    let mut block = BEGIN.to_vec();
    block.push(b'\n');
    block.extend_from_slice(FRAME.as_bytes());
    block.push(b'\n');
    block.extend_from_slice(method);
    // Framing belongs to the projection; the method bytes are never trimmed.
    block.push(b'\n');
    block.extend_from_slice(END);
    block
}

fn error(path: &Path, message: impl std::fmt::Display) -> String {
    format!("{}: {message}", path.display())
}

fn metadata(path: &Path) -> Result<Option<fs::Metadata>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(meta)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(error(path, e)),
    }
}

fn regular(path: &Path) -> Result<Option<Snapshot>, String> {
    let Some(meta) = metadata(path)? else {
        return Ok(None);
    };
    if !meta.file_type().is_file() {
        return Err(error(
            path,
            "expected a regular file (aliases are not supported here)",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.nlink() != 1 {
            return Err(error(
                path,
                "hard-linked retained files/outputs are unsupported",
            ));
        }
    }
    Ok(Some(Snapshot {
        bytes: fs::read(path).map_err(|e| error(path, e))?,
        permissions: meta.permissions(),
    }))
}

fn utf8(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::str::from_utf8(bytes)
        .map(|_| ())
        .map_err(|e| error(path, format!("expected UTF-8: {e}")))
}

fn input(path: &Path) -> Result<Vec<u8>, String> {
    // Explicit external inputs may use aliases; retained host files may not.
    let meta = fs::metadata(path).map_err(|e| error(path, e))?;
    if !meta.is_file() {
        return Err(error(path, "expected a readable regular input file"));
    }
    let bytes = fs::read(path).map_err(|e| error(path, e))?;
    utf8(path, &bytes)?;
    Ok(bytes)
}

fn positions(bytes: &[u8], needle: &[u8]) -> Vec<usize> {
    bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(i, part)| (part == needle).then_some(i))
        .collect()
}

fn line_end(bytes: &[u8], offset: usize) -> bool {
    offset == bytes.len()
        || bytes[offset..].starts_with(b"\n")
        || bytes[offset..].starts_with(b"\r\n")
}

fn render(path: &Path, bytes: &[u8], block: &[u8]) -> Result<Vec<u8>, String> {
    let reserved = positions(bytes, RESERVED);
    if reserved.is_empty() {
        let mut result = bytes.to_vec();
        if !result.is_empty() && !result.ends_with(b"\n") {
            result.push(b'\n');
        }
        result.extend_from_slice(block);
        result.push(b'\n');
        return Ok(result);
    }
    let starts = positions(bytes, BEGIN);
    let ends = positions(bytes, END);
    if reserved.len() != 2 || starts.len() != 1 || ends.len() != 1 {
        return Err(error(path, "malformed or duplicate managed delimiters"));
    }
    let (start, end) = (starts[0], ends[0]);
    if start >= end
        || (start > 0 && bytes[start - 1] != b'\n')
        || (end > 0 && bytes[end - 1] != b'\n')
        || !line_end(bytes, start + BEGIN.len())
        || !line_end(bytes, end + END.len())
    {
        return Err(error(
            path,
            "managed delimiters must be ordered standalone lines",
        ));
    }
    let mut result = bytes[..start].to_vec();
    result.extend_from_slice(block);
    result.extend_from_slice(&bytes[end + END.len()..]);
    Ok(result)
}

fn prepare(
    host: &Path,
    selected: Option<InitSources>,
) -> Result<(Vec<Change>, Vec<PathBuf>), String> {
    if !cfg!(unix) {
        return Err(
            "root guide publication requires Unix symlinks; this platform is unsupported".into(),
        );
    }
    let root = fs::canonicalize(host).map_err(|e| error(host, e))?;
    if !root.is_dir() {
        return Err(error(&root, "host root must be an existing directory"));
    }
    let mut directories = Vec::new();
    for relative in [".chrono-harness", DIRECTORY] {
        let path = root.join(relative);
        match metadata(&path)? {
            Some(meta) if !meta.file_type().is_dir() => {
                return Err(error(
                    &path,
                    "expected a directory, not a symlink or other type",
                ));
            }
            None => directories.push(path),
            _ => {}
        }
    }
    let mut changes = Vec::new();
    let manifest_path = root.join(MANIFEST);
    let manifest = regular(&manifest_path)?;
    let method_path = root.join(METHOD);
    let context_path = root.join(CONTEXT);
    let method = regular(&method_path)?;
    let context = regular(&context_path)?;
    let mut manifest_change = None;
    let method_bytes;
    if let Some(existing) = manifest {
        let parsed: Registration = serde_json::from_slice(&existing.bytes)
            .map_err(|e| error(&manifest_path, format!("invalid registration: {e}")))?;
        if parsed == legacy_registration() {
            manifest_change = Some(Change::new(
                manifest_path.clone(),
                Some(existing),
                manifest_bytes()?,
            ));
        } else if parsed != registration() {
            return Err(error(
                &manifest_path,
                "unsupported registration: expected exact literal-core/relative-alias/v2 or read-both/v1 migration identity",
            ));
        }
        let method = method.ok_or_else(|| error(&method_path, "registered source is missing"))?;
        let context =
            context.ok_or_else(|| error(&context_path, "registered source is missing"))?;
        utf8(&method_path, &method.bytes)?;
        utf8(&context_path, &context.bytes)?;
        if let Some(selected) = selected {
            if selected
                .methodology
                .is_some_and(|bytes| bytes != method.bytes)
                || selected.context.is_some_and(|bytes| bytes != context.bytes)
            {
                return Err(error(
                    &manifest_path,
                    "init inputs differ from retained canonical sources; edit retained sources deliberately and use generate",
                ));
            }
        }
        method_bytes = method.bytes;
    } else {
        let selected = selected.ok_or_else(|| {
            error(
                &manifest_path,
                "missing registration; run explicit init first",
            )
        })?;
        for (path, exists) in [
            (&method_path, method.is_some()),
            (&context_path, context.is_some()),
        ] {
            if exists {
                return Err(error(
                    path,
                    "reserved source collision without registration",
                ));
            }
        }
        method_bytes = selected
            .methodology
            .unwrap_or_else(|| DEFAULT_METHOD.as_bytes().to_vec());
        changes.push(Change::new(method_path.clone(), None, method_bytes.clone()));
        changes.push(Change::new(
            context_path,
            None,
            selected.context.unwrap_or_default(),
        ));
        manifest_change = Some(Change::new(manifest_path, None, manifest_bytes()?));
    }
    if !positions(&method_bytes, RESERVED).is_empty() {
        return Err(error(
            &method_path,
            "methodology contains reserved marker prefix <!-- chrono-instructions; edit the source before generation",
        ));
    }
    let block = managed_block(&method_bytes);
    let agents = root.join("AGENTS.md");
    let claude = root.join("CLAUDE.md");
    let alias = metadata(&agents)?.is_some_and(|m| m.file_type().is_symlink());
    let claude_before = regular(&claude)?;
    let agents_before = if alias {
        if fs::read_link(&agents)
            .map_err(|e| error(&agents, e))?
            .as_os_str()
            != OsStr::new("CLAUDE.md")
            || claude_before.is_none()
        {
            return Err(error(
                &agents,
                "only the relative alias AGENTS.md -> CLAUDE.md with a regular existing target is supported",
            ));
        }
        None
    } else {
        regular(&agents)?
    };
    // Sole AGENTS is a donor. For two regular roots, compare prospective bytes,
    // so stale owned blocks do not cause a conflict but different host text does.
    let donor = claude_before.as_ref().or(agents_before.as_ref());
    let donor_path = if claude_before.is_none() && agents_before.is_some() {
        &agents
    } else {
        &claude
    };
    let after = render(
        donor_path,
        donor.map_or(&[], |s| s.bytes.as_slice()),
        &block,
    )?;
    if claude_before.is_some() {
        if let Some(before) = &agents_before {
            if render(&agents, &before.bytes, &block)? != after {
                return Err(format!(
                    "{} and {}: conflicting host-owned content; deliberately consolidate preserved text in CLAUDE.md, then align AGENTS.md or remove the redundant regular AGENTS.md before retrying",
                    agents.display(),
                    claude.display()
                ));
            }
        }
    }
    let permissions = donor.map(|s| s.permissions.clone());
    let mut guide = Change::new(claude, claude_before, after);
    guide.permissions = permissions;
    changes.push(guide);
    if !alias {
        changes.push(Change::alias(agents, agents_before));
    }
    // The target precedes its alias; registration commits the new layout last.
    if let Some(change) = manifest_change {
        changes.push(change);
    }
    changes.retain(|c| !c.unchanged());
    Ok((changes, directories))
}

/// Retain exact selected UTF-8 bytes and publish the literal root guide and relative alias.
/// Repeated init refuses differing retained sources. The host must already exist.
pub fn init(host: &Path, methodology: &Path, context: &Path) -> Result<Vec<PathBuf>, String> {
    init_with_defaults(host, Some(methodology), Some(context))
}

/// Initialize with independently optional source files. For a new host, omissions
/// select the embedded general core and empty context. For a registered host,
/// omissions retain validated canonical bytes; explicit conflicts still fail.
pub fn init_with_defaults(
    host: &Path,
    methodology: Option<&Path>,
    context: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    let selected = InitSources::read(methodology, context)?;
    let (changes, directories) = prepare(host, Some(selected))?;
    transaction::publish(changes, directories, Fault::default())
}

/// Validate registered sources and refresh the guide and alias while preserving host-owned bytes.
pub fn generate(host: &Path) -> Result<Vec<PathBuf>, String> {
    let (changes, directories) = prepare(host, None)?;
    transaction::publish(changes, directories, Fault::default())
}

/// Bounded IO-failure injection available only in the dedicated test build.
#[cfg(feature = "test-support")]
pub mod test_support {
    use super::*;

    /// Fail before publication `after` (zero based); optionally fail one rollback.
    /// No production CLI or environment switch enables this hook.
    pub fn init_with_failure(
        host: &Path,
        method: &Path,
        context: &Path,
        after: usize,
        rollback: Option<usize>,
    ) -> Result<Vec<PathBuf>, String> {
        let selected = InitSources::read(Some(method), Some(context))?;
        let (changes, directories) = prepare(host, Some(selected))?;
        transaction::publish(
            changes,
            directories,
            Fault {
                after: Some(after),
                rollback,
            },
        )
    }
}

/// CLI transport result; exit 0 complete, 2 usage, 1 input/generation/IO failure.
#[derive(Debug)]
pub struct CliOutput {
    pub exit_code: u8,
    pub stdout: String,
    pub stderr: String,
}

/// Accept OS paths without shell interpolation or UTF-8 conversion of arguments.
pub fn dispatch(args: &[OsString]) -> CliOutput {
    let reply = |exit_code, stdout: String, stderr: String| CliOutput {
        exit_code,
        stdout,
        stderr,
    };
    if args.is_empty()
        || (args.len() == 1 && ["help", "--help", "-h"].iter().any(|a| args[0] == *a))
    {
        return reply(0, "Usage:\n  chrono-instructions init --host-root H [--methodology M] [--host-context C]\n  chrono-instructions generate --host-root H\nH must be an existing directory. New init defaults to the bundled general core and empty context. Registered init retains existing canonical sources for omitted options; explicit differing inputs fail. Optional M/C must be readable UTF-8 regular files. Edit retained sources deliberately, then generate to update CLAUDE.md and AGENTS.md -> CLAUDE.md. Unix only. No host inference or runtime checkout required. Exit 0 complete, 2 usage, 1 generation/IO failure. No judges run.\n".into(), String::new());
    }
    if args.len() == 1 && args[0] == "--version" {
        return reply(
            0,
            concat!("chrono-instructions ", env!("CARGO_PKG_VERSION"), "\n").into(),
            String::new(),
        );
    }
    let usage = || {
        reply(
            2,
            String::new(),
            "E_USAGE: use --help; --host-root is required, each option may occur only once and needs a value.\n".into(),
        )
    };
    let is_init = args[0] == "init";
    if !is_init && args[0] != "generate" {
        return usage();
    }
    let mut host = None;
    let mut method = None;
    let mut context = None;
    let mut parts = args[1..].chunks_exact(2);
    for pair in &mut parts {
        let slot = if pair[0] == "--host-root" {
            &mut host
        } else if is_init && pair[0] == "--methodology" {
            &mut method
        } else if is_init && pair[0] == "--host-context" {
            &mut context
        } else {
            return usage();
        };
        if slot.is_some() || pair[1].is_empty() {
            return usage();
        }
        *slot = Some(PathBuf::from(&pair[1]));
    }
    if !parts.remainder().is_empty() {
        return usage();
    }
    let Some(host) = host else {
        return usage();
    };
    let result = if is_init {
        init_with_defaults(&host, method.as_deref(), context.as_deref())
    } else {
        generate(&host)
    };
    match result {
        Ok(paths) => reply(
            0,
            format!(
                "complete: {} file(s) changed; no judges executed\n",
                paths.len()
            ),
            String::new(),
        ),
        Err(e) => reply(1, String::new(), format!("E_GENERATION: {e}\n")),
    }
}
