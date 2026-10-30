#[path = "support/history.rs"]
mod history;
#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
#[path = "support/temporary.rs"]
mod temporary;
use chrono_harness::{facts, observation, wire};
use chrono_judge_registration::{execution, interpret};
use chrono_judge_routes::{execute_actions, prepare_scoped};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    process::Command,
    sync::OnceLock,
};
use support::*;
const HISTORY: &str = ".chrono-harness/migrations/projects-history.json";
static HISTORICAL_INPUT: OnceLock<Result<history::Snapshot, String>> = OnceLock::new();

fn retain_history(processes: &[chrono_harness::ProcessResult], message: &str) -> String {
    let directory = source().join(".chrono-harness/state/migration-tests");
    fs::create_dir_all(&directory).unwrap();
    let (file, path) = tempfile::Builder::new()
        .prefix("history-input-")
        .suffix(".json")
        .tempfile_in(directory)
        .unwrap()
        .keep()
        .unwrap();
    serde_json::to_writer(file, &json!({"message":message,"processes":processes})).unwrap();
    format!(
        "{message}; original Git processes retained at {}",
        path.display()
    )
}

const TEST_HOSTS: &str = ".chrono-harness/migrations/test-hosts.json";
static FIXTURE_TOOLS: OnceLock<Result<FixtureTools, String>> = OnceLock::new();

struct FixtureTools {
    git: Value,
    git_input: Value,
    python: Value,
    observed: observation::Tool,
}

fn fixture_path<'a>(hosts: &'a Value, platform: &str) -> Result<&'a str, String> {
    hosts[platform]
        .as_str()
        .ok_or_else(|| format!("unregistered migration test platform: {platform}"))
}

fn fixture_data() -> Value {
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "Darwin/arm64".to_owned(),
        ("linux", "x86_64") => "Linux/x86_64".to_owned(),
        (os, arch) => format!("{os}/{arch}"),
    };
    let hosts: Value =
        serde_json::from_slice(&fs::read(source().join(TEST_HOSTS)).unwrap()).unwrap();
    let path = fixture_path(&hosts, &platform).unwrap();
    serde_json::from_slice(
        &fs::read(chrono_harness::no_symlink_parents(&source(), path).unwrap()).unwrap(),
    )
    .unwrap()
}

fn bind_fixture_tools(data: &Value) -> Result<FixtureTools, String> {
    let root = source();
    let declaration = &data["historical_git"];
    let program = declaration["program"]
        .as_str()
        .ok_or("fixture Git program")?;
    if !std::path::Path::new(program).is_absolute() {
        return Err("fixture Git must have an explicit absolute path".into());
    }
    let environment = history_environment();
    let observed = observation::tool(
        &root,
        program,
        &serde_json::from_value::<Vec<String>>(declaration["version_argv"].clone())
            .map_err(|e| e.to_string())?,
        &environment,
        120,
        1048576,
    )?;
    let identities = declaration["identities"]
        .as_array()
        .ok_or("fixture Git identities")?;
    // The platform explicitly admits a finite set of host identities. An
    // observation selects exactly one existing declaration; it creates none.
    let matches = identities
        .iter()
        .filter(|identity| {
            identity["sha256"] == observed.sha256
                && identity["expected_version"] == observed.version.stdout.trim_end()
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 || observation::process_success(&observed.version).is_err() {
        return Err(retain_history(
            &[observed.version],
            &format!(
                "historical fixture Git binding mismatch: expected {declaration}; observed sha256 {}",
                observed.sha256
            ),
        ));
    }
    let identity = matches[0];
    let git = json!({"id":"git", "program":program, "resolution":"PATH-once",
        "version_argv":declaration["version_argv"], "expected_version":identity["expected_version"]});
    let git_input = json!({"id":"git-executable", "location":program,
        "presence":"present", "sha256":identity["sha256"]});
    let python = data["tools"]
        .as_array()
        .ok_or("fixture tools")?
        .iter()
        .find(|tool| tool["id"] == "python3")
        .ok_or("fixture Python binding")?
        .clone();
    let python_observed = observation::tool(
        &root,
        python["program"].as_str().ok_or("fixture Python program")?,
        &serde_json::from_value::<Vec<String>>(python["version_argv"].clone())
            .map_err(|e| e.to_string())?,
        &environment,
        30,
        1048576,
    )?;
    if python_observed.version.stdout.trim_end() != python["expected_version"]
        || observation::process_success(&python_observed.version).is_err()
    {
        return Err(retain_history(
            &[python_observed.version],
            &format!("historical fixture Python binding mismatch: expected {python}"),
        ));
    }
    retain_history(
        &[observed.version.clone(), python_observed.version],
        &format!(
            "historical fixture host selected: {}; expected {git}; expected input {git_input}",
            identity["host"]
        ),
    );
    Ok(FixtureTools {
        git,
        git_input,
        python,
        observed,
    })
}

fn fixture_tools() -> &'static FixtureTools {
    FIXTURE_TOOLS
        .get_or_init(|| bind_fixture_tools(&fixture_data()))
        .as_ref()
        .unwrap_or_else(|e| panic!("historical fixture tool prerequisite failed: {e}"))
}

fn history_environment() -> BTreeMap<String, String> {
    ["PATH", "HOME", "SSH_AUTH_SOCK"]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok().map(|value| (key.into(), value)))
        .collect()
}

#[test]
fn historical_platform_bindings_reject_unknown_wrong_and_ambiguous_identities() {
    let hosts: Value =
        serde_json::from_slice(&fs::read(source().join(TEST_HOSTS)).unwrap()).unwrap();
    assert!(
        fixture_path(&hosts, "unregistered/platform")
            .unwrap_err()
            .contains("unregistered")
    );
    let fixture = fixture_data();
    bind_fixture_tools(&fixture).unwrap_or_else(|e| panic!("declared host rejected: {e}"));
    for field in ["sha256", "expected_version", "duplicate"] {
        let mut wrong = fixture.clone();
        let identities = wrong["historical_git"]["identities"]
            .as_array_mut()
            .unwrap();
        if field == "duplicate" {
            identities.extend(identities.clone());
        } else {
            for identity in identities {
                identity[field] = json!("deliberately wrong binding");
            }
        }
        let error = bind_fixture_tools(&wrong)
            .err()
            .expect("invalid fixed binding was adopted");
        assert!(
            error.contains("historical fixture Git binding mismatch"),
            "{error}"
        );
        assert!(
            error.contains("expected") && error.contains("observed sha256"),
            "{error}"
        );
    }
}

fn historical_input() -> &'static history::Snapshot {
    HISTORICAL_INPUT
        .get_or_init(|| {
            let root = source();
            let tools = fixture_tools();
            let observed = tools.observed.clone();
            let environment = history_environment();
            let binding: history::Binding =
                serde_json::from_slice(&fs::read(root.join(HISTORY)).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let spec = chrono_harness::CommandSpec {
                program: observed
                    .path
                    .to_str()
                    .ok_or("fixture Git path UTF-8")?
                    .into(),
                args: vec![],
                env: environment,
                timeout_seconds: binding.timeout_seconds,
                output_limit_bytes: binding.output_limit_bytes,
            };
            match history::acquire(&root, &binding, spec, &observed.sha256) {
                Ok(mut snapshot) => {
                    snapshot.processes.insert(0, observed.version);
                    retain_history(&snapshot.processes, "historical fixture input acquired");
                    Ok(snapshot)
                }
                Err(mut failure) => {
                    failure.processes.insert(0, observed.version);
                    Err(retain_history(&failure.processes, &failure.message))
                }
            }
        })
        .as_ref()
        .unwrap_or_else(|error| panic!("historical fixture prerequisite failed: {error}"))
}

fn history_spec() -> (chrono_harness::CommandSpec, String) {
    let tools = fixture_tools();
    let program = tools.git["program"].as_str().unwrap();
    let digest = tools.git_input["sha256"].as_str().unwrap().to_owned();
    let spec = chrono_harness::CommandSpec {
        program: program.into(),
        args: vec![],
        env: ["PATH", "HOME"]
            .into_iter()
            .filter_map(|key| std::env::var(key).ok().map(|value| (key.into(), value)))
            .collect(),
        timeout_seconds: 10,
        output_limit_bytes: 1048576,
    };
    (spec, digest)
}

fn partial_history_fixture() -> (tempfile::TempDir, tempfile::TempDir, history::Binding) {
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "-q"]);
    git(remote.path(), &["config", "uploadpack.allowFilter", "true"]);
    fs::write(remote.path().join("original bytes λ"), [0, 255, 10, 42]).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("original bytes λ", remote.path().join("alias")).unwrap();
    let commit = commit(remote.path());
    let tree = git(remote.path(), &["rev-parse", &format!("{commit}^{{tree}}")]);
    let local = tempfile::tempdir().unwrap();
    git(local.path(), &["init", "-q"]);
    // Seed only commit and tree objects: a real promisor checkout with missing
    // fixture blobs, without cloning or fetching unrelated history.
    let (mut spec, digest) = history_spec();
    for (kind, oid) in [("commit", &commit), ("tree", &tree)] {
        spec.args = vec!["cat-file".into(), kind.into(), oid.clone()];
        let original =
            chrono_harness::run_process_observed(remote.path(), &spec, &[], &digest).unwrap();
        assert_eq!(original.exit_code, 0);
        spec.args = vec![
            "hash-object".into(),
            "-w".into(),
            "-t".into(),
            kind.into(),
            "--stdin".into(),
        ];
        let seeded = chrono_harness::run_process_observed(
            local.path(),
            &spec,
            &original.stdout_bytes,
            &digest,
        )
        .unwrap();
        assert_eq!(seeded.exit_code, 0);
        assert_eq!(seeded.stdout.trim(), oid);
    }
    git(local.path(), &["update-ref", "HEAD", &commit]);
    git(
        local.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    git(local.path(), &["config", "remote.origin.promisor", "true"]);
    git(
        local.path(),
        &["config", "remote.origin.partialclonefilter", "blob:none"],
    );
    let binding = history::Binding {
        schema: "chrono-test-history/v1".into(),
        commit,
        tree,
        remote: "origin".into(),
        timeout_seconds: 10,
        output_limit_bytes: 1048576,
    };
    (remote, local, binding)
}

fn fetch_count(processes: &[chrono_harness::ProcessResult]) -> usize {
    processes
        .iter()
        .filter(|p| p.argv.iter().any(|arg| arg == "fetch"))
        .count()
}

#[test]
fn historical_fixture_acquires_missing_blobs_once_then_consumes_without_network() {
    let (remote, local, binding) = partial_history_fixture();
    let missing_remote = remote.path().join("unavailable-origin");
    git(
        local.path(),
        &[
            "remote",
            "set-url",
            "origin",
            missing_remote.to_str().unwrap(),
        ],
    );
    let (mut spec, digest) = history_spec();
    spec.args = vec![
        "show".into(),
        format!("{}:original bytes λ", binding.commit),
    ];
    let before = chrono_harness::run_process_observed(local.path(), &spec, &[], &digest).unwrap();
    assert_ne!(before.exit_code, 0);
    assert!(before.failure.is_none(), "{:?}", before.failure);
    assert!(
        before.stderr.contains(missing_remote.to_str().unwrap()),
        "{}",
        before.stderr
    );
    retain_history(
        &[before],
        "regression: original lazy historical read failed",
    );
    git(
        local.path(),
        &[
            "remote",
            "set-url",
            "origin",
            remote.path().to_str().unwrap(),
        ],
    );
    let (spec, digest) = history_spec();
    let snapshot = history::acquire(local.path(), &binding, spec.clone(), &digest).unwrap();
    // Retain originals before asserting: a regression must not lose its probes.
    retain_history(
        &snapshot.processes,
        "regression: explicit historical acquisition observed",
    );
    assert_eq!(fetch_count(&snapshot.processes), 1);
    assert!(
        snapshot
            .processes
            .iter()
            .filter(|p| !p.argv.iter().any(|a| a == "fetch"))
            .all(|p| p.stderr_bytes.is_empty())
    );
    let fetch = snapshot
        .processes
        .iter()
        .find(|p| p.argv.iter().any(|arg| arg == "fetch"))
        .unwrap();
    assert!(fetch.argv.iter().any(|arg| arg == "--no-filter"));
    assert!(
        !fetch
            .argv
            .iter()
            .any(|arg| arg == &binding.commit || arg == &binding.tree)
    );
    assert!(
        snapshot
            .processes
            .iter()
            .filter(|p| p.argv.iter().any(|arg| arg.starts_with("--git-dir=")))
            .all(|p| !p.environment.contains_key("GIT_NO_LAZY_FETCH"))
    );
    // Ordinary reentry needs neither a working remote nor a replacement source.
    git(
        local.path(),
        &[
            "remote",
            "set-url",
            "origin",
            missing_remote.to_str().unwrap(),
        ],
    );
    let warm = history::acquire(local.path(), &binding, spec, &digest).unwrap();
    retain_history(
        &warm.processes,
        "regression: warm consumption with unavailable origin",
    );
    assert_eq!(fetch_count(&warm.processes), 0);
    assert_eq!(warm.tree, snapshot.tree);
    assert_eq!(warm.blobs, snapshot.blobs);
    assert!(warm.processes.iter().all(|p| p.stderr_bytes.is_empty()));
    assert_eq!(git(local.path(), &["rev-parse", "HEAD"]), binding.commit);
    let exported = tempfile::tempdir().unwrap();
    warm.export(exported.path()).unwrap();
    assert_eq!(
        fs::read(exported.path().join("original bytes λ")).unwrap(),
        [0, 255, 10, 42]
    );
    #[cfg(unix)]
    assert_eq!(
        fs::read_link(exported.path().join("alias")).unwrap(),
        std::path::Path::new("original bytes λ")
    );
    assert!(
        fs::metadata(exported.path().join("original bytes λ"))
            .unwrap()
            .permissions()
            .readonly()
    );
    let cold = tempfile::tempdir().unwrap();
    git(cold.path(), &["init", "-q"]);
    git(
        cold.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    let (spec, digest) = history_spec();
    let cold_snapshot = history::acquire(cold.path(), &binding, spec, &digest).unwrap();
    retain_history(
        &cold_snapshot.processes,
        "regression: missing pinned commit acquired explicitly",
    );
    assert_eq!(cold_snapshot.tree, snapshot.tree);
    assert_eq!(cold_snapshot.blobs, snapshot.blobs);
    assert_eq!(fetch_count(&cold_snapshot.processes), 2);
    let (spec, digest) = history_spec();
    git(
        cold.path(),
        &[
            "remote",
            "set-url",
            "origin",
            missing_remote.to_str().unwrap(),
        ],
    );
    let cold_warm = history::acquire(cold.path(), &binding, spec, &digest).unwrap();
    retain_history(
        &cold_warm.processes,
        "regression: fetched metadata and blobs consumed offline",
    );
    assert_eq!(fetch_count(&cold_warm.processes), 0);
    assert_eq!(cold_warm.tree, snapshot.tree);
    assert_eq!(cold_warm.blobs, snapshot.blobs);
    assert!(
        cold_warm
            .processes
            .iter()
            .all(|p| p.stderr_bytes.is_empty())
    );
    assert!(!cold.path().join(".git/shallow").exists());
    assert!(!cold.path().join(".git/FETCH_HEAD").exists());
    assert!(
        cold_snapshot
            .processes
            .iter()
            .any(|p| p.argv.iter().any(|arg| arg == "--filter=blob:none"))
    );
}

#[test]
fn historical_fixture_retains_failed_fetch_and_rejects_identity_drift() {
    let (remote, local, binding) = partial_history_fixture();
    git(
        local.path(),
        &[
            "remote",
            "set-url",
            "origin",
            remote.path().join("unavailable-origin").to_str().unwrap(),
        ],
    );
    let (spec, digest) = history_spec();
    let failure = history::acquire(local.path(), &binding, spec.clone(), &digest)
        .err()
        .unwrap();
    retain_history(
        &failure.processes,
        "regression: unavailable historical origin acquisition observed",
    );
    assert_eq!(fetch_count(&failure.processes), 1);
    assert!(failure.message.contains("Git failed"));
    let failed = failure.processes.last().unwrap();
    assert_ne!(failed.exit_code, 0);
    assert!(!failed.stderr_bytes.is_empty());
    assert!(
        failure.processes[..failure.processes.len() - 1]
            .iter()
            .all(|p| p.stderr_bytes.is_empty())
    );
    assert!(
        failure
            .processes
            .iter()
            .filter(|p| p.argv.iter().any(|arg| arg.starts_with("--git-dir=")))
            .all(|p| !p.environment.contains_key("GIT_NO_LAZY_FETCH"))
    );
    let mut wrong = binding.clone();
    wrong.tree = "a".repeat(40);
    let failure = history::acquire(local.path(), &wrong, spec.clone(), &digest)
        .err()
        .unwrap();
    retain_history(&failure.processes, "regression: wrong pinned tree rejected");
    assert!(failure.message.contains("identity mismatch"));
    assert_eq!(fetch_count(&failure.processes), 0);
    let mut wrong_kind = binding.clone();
    wrong_kind.commit = binding.tree.clone();
    let failure = history::acquire(local.path(), &wrong_kind, spec.clone(), &digest)
        .err()
        .unwrap();
    retain_history(
        &failure.processes,
        "regression: tree cannot substitute for pinned commit",
    );
    assert!(
        failure.message.contains("expected exact historical commit"),
        "{}",
        failure.message
    );
    assert_eq!(fetch_count(&failure.processes), 0);
    // A cold metadata failure has its own original probe/fetch diagnostics;
    // success from the warm blob path cannot stand in for this boundary.
    let cold = tempfile::tempdir().unwrap();
    git(cold.path(), &["init", "-q"]);
    git(
        cold.path(),
        &[
            "remote",
            "add",
            "origin",
            remote.path().join("unavailable-origin").to_str().unwrap(),
        ],
    );
    let failure = history::acquire(cold.path(), &binding, spec.clone(), &digest)
        .err()
        .unwrap();
    retain_history(
        &failure.processes,
        "regression: unavailable pinned metadata acquisition observed",
    );
    assert_eq!(fetch_count(&failure.processes), 1);
    let probe = failure
        .processes
        .iter()
        .find(|p| {
            p.argv
                .iter()
                .any(|arg| arg == "--batch-check=%(objectname) %(objecttype)")
        })
        .unwrap();
    assert_eq!(
        probe.stdout_bytes,
        format!("{} missing\n", binding.commit).as_bytes()
    );
    let failed = failure.processes.last().unwrap();
    assert!(failed.argv.iter().any(|arg| arg == "--filter=blob:none"));
    assert_ne!(failed.exit_code, 0);
    assert!(!failed.stderr_bytes.is_empty());
    wrong = binding;
    wrong.commit = "b".repeat(40);
    let failure = history::acquire(local.path(), &wrong, spec, &digest)
        .err()
        .unwrap();
    retain_history(
        &failure.processes,
        "regression: wrong pinned commit remains failed",
    );
    assert_eq!(fetch_count(&failure.processes), 1);
    assert_ne!(failure.processes.last().unwrap().exit_code, 0);
}
fn adopt_fixture_bindings(_root: &std::path::Path, config: &mut Value) {
    let fixture = fixture_tools();
    for declaration in [&fixture.git, &fixture.python] {
        let tool = config["tools"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|tool| tool["id"] == declaration["id"])
            .unwrap();
        *tool = declaration.clone();
    }
    let input = config["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|input| input["id"] == fixture.git_input["id"])
        .unwrap();
    *input = fixture.git_input.clone();
}
fn migration_host() -> (
    temporary::FixtureDirectory,
    std::path::PathBuf,
    String,
    String,
) {
    migration_host_with_alias_collision(false)
}
fn copy_nested_receipts(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    let kind = fs::symlink_metadata(from)?.file_type();
    if kind.is_file() {
        fs::copy(from, to)?;
    } else if kind.is_dir() {
        fs::create_dir(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_nested_receipts(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        return Err(std::io::Error::other(format!(
            "unsupported nested receipt entry: {}",
            from.display()
        )));
    }
    Ok(())
}
fn execution_failure(
    root: &std::path::Path,
    reason: &str,
    result: &chrono_judge_projects::Results,
) -> String {
    let protection = temporary::retain_diagnostic(root, reason);
    let directory = source().join(".chrono-harness/state/migration-tests");
    let retained = (|| -> Result<String, Box<dyn std::error::Error>> {
        fs::create_dir_all(&directory)?;
        let (mut file, path) = tempfile::Builder::new()
            .prefix("unexpected-execution-")
            .suffix(".json")
            .tempfile_in(&directory)?
            .keep()?;
        let mut summary = match serde_json::to_writer(&mut file, result) {
            Ok(()) => format!("original execution retained at {}", path.display()),
            Err(error) => format!(
                "original execution retention incomplete at {}: {error}",
                path.display()
            ),
        };
        // The inner CI owner has already recorded its real process result and
        // inputs. Preserve those bytes before this temporary host is dropped.
        let nested = root.join(".chrono-harness/state/ci-test-failures");
        let destination = path.with_extension("ci-test-failures");
        let evidence = match copy_nested_receipts(&nested, &destination) {
            Ok(()) => format!("nested CI evidence retained at {}", destination.display()),
            Err(error) => format!(
                "nested CI evidence retention incomplete at {}: {error}",
                destination.display()
            ),
        };
        summary.push_str(&format!("; {evidence}"));
        Ok(summary)
    })()
    .unwrap_or_else(|error| format!("original execution retention failed: {error}"));
    let mut summary = format!("{reason}; {retained}");
    if let Err(error) = protection {
        summary.push_str(&format!("; diagnostic reference failed: {error}"));
    }
    for row in result.executed.iter().chain(&result.blocked) {
        if row.status == "passed" {
            continue;
        }
        let error: String = row
            .error
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(240)
            .collect();
        summary.push_str(&format!("\n{}: {} {error}", row.operation, row.status));
        if let Some(receipt) = &row.receipt {
            summary.push_str(&format!(
                " (exit {}, stdout {} bytes, stderr {} bytes)",
                receipt.process.exit_code,
                receipt.process.stdout_bytes.len(),
                receipt.process.stderr_bytes.len()
            ));
        }
    }
    summary
}
fn migration_host_with_alias_collision(
    collision: bool,
) -> (
    temporary::FixtureDirectory,
    std::path::PathBuf,
    String,
    String,
) {
    let source = fs::canonicalize(source()).unwrap();
    let dir = temporary::migration_directory("migration actual host ");
    let root = fs::canonicalize(dir.path()).unwrap();
    historical_input().export(&root).unwrap();
    if collision {
        // A controlled regression input, committed in this temporary host before
        // decoding, preserves both real original definitions under one alias.
        let mut projects: Value =
            serde_json::from_slice(&fs::read(root.join(PROJECTS)).unwrap()).unwrap();
        let mut project = projects["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["kind"] == "test")
            .unwrap()
            .clone();
        project["id"] = json!("ci-verify");
        project["actions"] = json!({"execute":{"operation":"collision.execute","tool":"cargo","argv":["--version"]}});
        projects["projects"].as_array_mut().unwrap().push(project);
        let mut permissions = fs::metadata(root.join(PROJECTS)).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(root.join(PROJECTS), permissions).unwrap();
        fs::write(
            root.join(PROJECTS),
            serde_json::to_vec_pretty(&projects).unwrap(),
        )
        .unwrap();
    }
    git(&root, &["init", "-q"]);
    let base = commit(&root);
    let map: Value = serde_json::from_slice(&fs::read(source.join(FM)).unwrap()).unwrap();
    let historical: Value = serde_json::from_slice(&fs::read(root.join(FM)).unwrap()).unwrap();
    let current_paths: BTreeSet<_> = map["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    // Overlaying the current candidate must include its declared retirements.
    // The fixed historical snapshot remains untouched at base.
    for file in historical["files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        if !current_paths.contains(path) && path != "AGENTS.md" {
            let target = root.join(path);
            if target.exists() {
                fs::remove_file(target).unwrap();
            }
        }
    }
    for file in map["files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        if path == "AGENTS.md" {
            continue;
        }
        let dest = root.join(path);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        if dest.exists() {
            let mut permissions = fs::metadata(&dest).unwrap().permissions();
            permissions.set_readonly(false);
            fs::set_permissions(&dest, permissions).unwrap();
        }
        fs::copy(source.join(path), &dest)
            .unwrap_or_else(|e| panic!("copy declared fixture {path}: {e}"));
    }
    fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
    for (project, name) in [
        ("ci", "chrono-ci"),
        ("runner", "chrono-harness"),
        ("judge-ci", "chrono-judge-ci"),
    ] {
        let original = source.join(format!("crates/{project}/target/debug/{name}"));
        let destination = root.join(format!(".chrono-harness/bin/{name}"));
        temporary::copied_fixture_input(&root, &original, &destination);
        fs::copy(original, destination).unwrap();
    }
    // The copied candidate is a test host on the executing native platform.
    // Preserve the historical snapshot; install only the explicitly selected
    // platform fixture declarations in the new host. The copied host's Git digest/version can reject at Reader::for_config
    // before either historical-method or wrong-Python-version diagnostics.
    // Rebind a declared direct policy as fixture data, preserving the fixed
    // historical snapshot and leaving production selector admission to Rust.
    let mut config: Value = serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/git/pinned-macos.json")).unwrap(),
    )
    .unwrap();
    adopt_fixture_bindings(&root, &mut config);
    fs::write(
        root.join(CONFIG),
        serde_json::to_vec_pretty(&config).unwrap(),
    )
    .unwrap();
    let workflow = ".github/workflows/chrono-ci.yml";
    let mut bytes = fs::read(root.join(workflow)).unwrap();
    bytes.extend_from_slice(b"\n# real workflow drift\n");
    fs::write(root.join(workflow), bytes).unwrap();
    let candidate = commit(&root);
    (dir, root, base, candidate)
}

#[test]
fn decoded_historical_bindings_preserve_original_aliases_and_ambiguity() {
    for collision in [false, true] {
        let (_dir, root, base, candidate) = migration_host_with_alias_collision(collision);
        let raw = facts::registry_values(&root, &base, CONFIG).unwrap();
        let new = facts::registry_values(&root, &candidate, CONFIG).unwrap();
        let (old, current, view) = interpret(
            &root,
            &base,
            &candidate,
            CONFIG,
            raw.clone(),
            new,
            &std::env::vars().collect(),
        )
        .unwrap();
        assert_eq!(view["conversion"]["input"]["original"], json!(raw));
        let nodes = old.node_data();
        let node = &nodes["test:ci-verify"];
        assert_eq!(node.definitions.len(), if collision { 2 } else { 1 });
        let bindings = old.test_bindings();
        let historical = &bindings["test:ci-verify"];
        assert_eq!(historical.len(), node.definitions.len());
        assert!(historical.iter().any(|b| b.owner == "script:ci-verify"
            && b.action == "execute"
            && b.operation == "ci.verify"));
        if collision {
            assert!(
                historical
                    .iter()
                    .any(|b| b.owner == "project:ci-verify" && b.operation == "collision.execute")
            );
            assert!(node.unique().is_none());
            assert!(!execute_actions(&old).contains_key("test:ci-verify"));
            let error = chrono_judge_routes::order(
                &BTreeSet::from(["test:ci-verify".into()]),
                &execution::plans(old.filemap()).unwrap(),
                &execution::methods(current.projects()).unwrap(),
                &execute_actions(&old),
            )
            .unwrap_err();
            assert!(error.contains("unique execute action"), "{error}");
        } else {
            assert_eq!(execute_actions(&old)["test:ci-verify"], "ci.verify");
        }
        assert!(!execute_actions(&current).contains_key("test:ci-verify"));
        assert_eq!(execute_actions(&current)["test:ci-tests"], "test.ci-tests");
    }
}
#[test]
fn real_historical_profile_repair_preserves_obligations_and_verify_detects_drift() {
    let (_dir, root, base, candidate) = migration_host();
    let raw = facts::registry_values(&root, &base, CONFIG).unwrap();
    let new = facts::registry_values(&root, &candidate, CONFIG).unwrap();
    let mut missing_mapping = new.clone();
    missing_mapping.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]["mappings"] = json!([]);
    let mut env: BTreeMap<String, String> = std::env::vars().collect();
    env.insert("RUSTUP_TOOLCHAIN".into(), "1.95.0".into());
    env.insert("CARGO_TERM_COLOR".into(), "never".into());
    // The historical decoder retains the original method. The current host
    // explicitly replaces its provider; standalone consumers may not infer it.
    let mut missing_method = new.clone();
    missing_method.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]
        .as_object_mut()
        .unwrap()
        .remove("method_replacements");
    let error = interpret(
        &root,
        &base,
        &candidate,
        CONFIG,
        raw.clone(),
        missing_method.clone(),
        &env,
    )
    .err()
    .expect("missing method replacement unexpectedly accepted");
    assert!(
        error.contains("historical method changed"),
        "unexpected historical method error: {error}"
    );
    let old_action = raw[PROJECTS]["scripts"][0]["actions"]["execute"].clone();
    missing_method.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["id"] == "ci")
        .unwrap()["actions"]["execute"] = old_action;
    interpret(
        &root,
        &base,
        &candidate,
        CONFIG,
        raw.clone(),
        missing_method,
        &env,
    )
    .unwrap_or_else(|error| {
        panic!("omitted replacements must retain the original strict unchanged-method contract: {error}")
    });
    for (pointer, value) in [
        ("/from/argv", json!(["invented historical method"])),
        ("/from/owner", json!("script:missing")),
        ("/to/argv", json!(["stale candidate method"])),
        ("/to/tool", json!("missing-tool")),
        ("/to/owner", json!("project:runner")),
        ("/to/operation", json!("renamed.operation")),
        ("/to/argv", json!("not-an-array")),
        ("/reason", json!("")),
    ] {
        let mut invalid = new.clone();
        *invalid.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]["method_replacements"][0]
            .pointer_mut(pointer)
            .unwrap() = value;
        assert!(
            interpret(&root, &base, &candidate, CONFIG, raw.clone(), invalid, &env).is_err(),
            "unbound replacement accepted: {pointer}"
        );
    }
    for variant in [
        "duplicate",
        "unknown-field",
        "unchanged",
        "unused",
        "ambiguous-target",
    ] {
        let mut invalid = new.clone();
        let rows =
            invalid.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]["method_replacements"]
                .as_array_mut()
                .unwrap();
        match variant {
            "duplicate" => rows.push(rows[0].clone()),
            "unknown-field" => rows[0]["waiver"] = json!(true),
            "unchanged" => rows[0]["to"] = rows[0]["from"].clone(),
            "unused" => {
                rows[0]["from"]["operation"] = json!("unused.operation");
                rows[0]["to"]["operation"] = json!("unused.operation");
            }
            "ambiguous-target" => {
                let action = invalid.get_mut(PROJECTS).unwrap()["projects"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|p| p["id"] == "ci")
                    .unwrap()["actions"]
                    .as_object_mut()
                    .unwrap();
                action.insert("duplicate".into(), action["execute"].clone());
            }
            _ => unreachable!(),
        }
        assert!(
            interpret(&root, &base, &candidate, CONFIG, raw.clone(), invalid, &env).is_err(),
            "invalid method replacement accepted: {variant}"
        );
    }
    let (old, current, view) =
        interpret(&root, &base, &candidate, CONFIG, raw.clone(), new, &env).unwrap();
    assert!(
        interpret(
            &root,
            &base,
            &candidate,
            CONFIG,
            raw.clone(),
            missing_mapping,
            &env
        )
        .err()
        .unwrap()
        .contains("mapping")
    );
    assert_eq!(
        old.node_data()["test:ci-verify"].definitions[0].value,
        &raw[PROJECTS]["scripts"][0]
    );
    assert_eq!(old.filemap()["project_edges"], raw[FM]["project_edges"]);
    assert_eq!(old.filemap()["test_costs"], raw[FM]["test_costs"]);
    assert_eq!(
        view["conversion"]["input_digest"],
        wire::digest(&view["conversion"]["input"]).unwrap()
    );
    assert_eq!(view["conversion"]["input"]["original"], json!(raw));
    assert_eq!(
        chrono_judge_registration::replacements(&current).unwrap()["test:ci-verify"],
        "test:ci-tests"
    );
    let plans = execution::plans(current.filemap()).unwrap();
    for operation in &execution::plans(old.filemap()).unwrap()["test:ci-verify"].operations {
        assert!(plans["test:ci-tests"].operations.contains(operation));
    }
    let selected = BTreeSet::from(["test:ci-tests".into()]);
    let plan = prepare_scoped(
        &root,
        json!({"base":base,"candidate":candidate,"run":"real-ci-consumer"}),
        &selected,
        &plans,
        &execution::methods(current.projects()).unwrap(),
        &execute_actions(&current),
        &current.config()["tools"],
        env,
        &chrono_judge_registration::reused_tools(&view).unwrap(),
    )
    .unwrap();
    let result = chrono_judge_projects::execute(&plan).unwrap();
    let verify = result
        .executed
        .iter()
        .find(|r| r.operation == "ci.verify")
        .unwrap_or_else(|| panic!("{}", execution_failure(&root, "ci.verify blocked", &result)));
    assert_eq!(
        verify.status,
        "failed",
        "{}",
        execution_failure(&root, "ci.verify must reject drift", &result)
    );
    assert!(
        verify
            .receipt
            .as_ref()
            .unwrap()
            .process
            .stderr
            .contains("drift"),
        "{}",
        execution_failure(&root, "ci.verify did not diagnose drift", &result)
    );
    let output = temporary::capture_native(
        &root,
        Command::new(root.join(".chrono-harness/bin/chrono-ci")).args([
            "generate",
            "--host-root",
            root.to_str().unwrap(),
            "--config",
            ".chrono-harness/ci/units.json",
        ]),
    );
    assert!(
        output.status.success(),
        "workflow repair failed: {output:?}"
    );
    // The repair changes only workflow bytes; rerun its single registered method with a new input binding.
    let action = current.projects()["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "ci")
        .unwrap()["actions"]["execute"]
        .clone();
    let tool = &plan.tools[action["tool"].as_str().unwrap()];
    let spec = chrono_harness::CommandSpec {
        program: tool.path.to_str().unwrap().into(),
        args: serde_json::from_value(action["argv"].clone()).unwrap(),
        env: plan.environment.clone(),
        timeout_seconds: 30,
        output_limit_bytes: 4096,
    };
    let verified = chrono_harness::run_process_observed(&root, &spec, &[], &tool.sha256).unwrap();
    assert_eq!(verified.exit_code, 0, "{}", verified.stderr);
    assert!(
        verified.failure.is_none(),
        "workflow verify failed: {verified:?}"
    );
    // These consumers have joined and the repaired workflow has been consumed.
    // This distinct Git host owns its nested fixtures; the outer allocation's
    // drain cannot reach them. Keep the owner's original result before judging
    // its exact member disposal/protection outcomes. Earlier failures retain the
    // fixture through the existing diagnostic guard without this handoff.
    let argv = vec![
        "maintain".to_owned(),
        "--host-root".to_owned(),
        root.to_str().unwrap().to_owned(),
    ];
    let owner = chrono_worktree::run(&argv);
    let directory = source().join(".chrono-harness/state/migration-tests");
    fs::create_dir_all(&directory).unwrap();
    let (file, original) = tempfile::Builder::new()
        .prefix("inner-owner-handoff-")
        .suffix(".json")
        .tempfile_in(directory)
        .unwrap()
        .keep()
        .unwrap();
    serde_json::to_writer(
        file,
        &json!({"argv":argv,"exit_code":owner.exit_code,
            "stdout_bytes":owner.stdout.as_bytes(),"stderr_bytes":owner.stderr.as_bytes()}),
    )
    .unwrap();
    assert_eq!(
        owner.exit_code,
        0,
        "inner owner handoff failed; original result at {}",
        original.display()
    );
    let handoff: Value = serde_json::from_str(&owner.stdout).unwrap();
    assert_eq!(handoff["status"], "maintained");
    assert_eq!(handoff["invoking_root"], json!(root));
    assert_eq!(handoff["lifecycle_coordinator_root"], json!(root));
    assert!(
        handoff["producer_objects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|member| member["producer"] == "ci-test-host"
                && member["path"].is_string()
                && member["status"].is_string()),
        "inner fixture member outcomes missing; original result at {}",
        original.display()
    );
}

#[test]
fn migration_binding_failure_retains_expected_and_observed_version() {
    let (_dir, root, base, candidate) = migration_host();
    let raw = facts::registry_values(&root, &base, CONFIG).unwrap();
    let mut new = facts::registry_values(&root, &candidate, CONFIG).unwrap();
    let tool = new.get_mut(CONFIG).unwrap()["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "python3")
        .unwrap();
    let observed = temporary::capture_native(
        &root,
        Command::new(tool["program"].as_str().unwrap()).arg("--version"),
    );
    tool["expected_version"] = json!("Python deliberately-wrong");
    let env: BTreeMap<String, String> = std::env::vars().collect();
    let error = interpret(&root, &base, &candidate, CONFIG, raw, new, &env)
        .err()
        .unwrap();
    assert!(
        error.starts_with("E_TOOL_BINDING: migration version mismatch"),
        "unexpected migration binding error: {error}"
    );
    assert!(
        error.contains("Python deliberately-wrong"),
        "missing expected version: {error}"
    );
    let diagnostic: Value = serde_json::from_str(
        error
            .split_once("; observation=")
            .expect("missing observed binding")
            .1,
    )
    .unwrap();
    assert_eq!(
        diagnostic["tool"]["version"]["stdout_bytes"],
        json!(observed.stdout)
    );
    assert_eq!(
        diagnostic["tool"]["version"]["stderr_bytes"],
        json!(observed.stderr)
    );
    assert_eq!(
        diagnostic["tool"]["version"]["exit_code"],
        observed.status.code().unwrap()
    );
    assert!(
        diagnostic["tool"]["path"]
            .as_str()
            .unwrap()
            .starts_with('/')
    );
    assert!(diagnostic["tool"]["sha256"].as_str().unwrap().len() == 64);
}

#[test]
fn migration_copied_git_binding_rejects_mismatch_before_decoder() {
    let (_dir, root, base, candidate) = migration_host();
    let raw = facts::registry_values(&root, &base, CONFIG).unwrap();
    let config = facts::registry_values(&root, &candidate, CONFIG).unwrap()[CONFIG].clone();
    // A real forwarding executable has the same Git version but different bytes.
    // Moving a copied host declaration to it must reject the stale digest, then
    // pass with an independently declared digest of the wrapper's source bytes.
    let original_git = config["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["id"] == "git")
        .unwrap()["program"]
        .as_str()
        .unwrap();
    let wrapper = root.join(".chrono-harness/bin/fixture-git");
    let wrapper_bytes = format!(
        "#!/bin/sh\nexec '{}' \"$@\"\n",
        original_git.replace('\'', "'\\''")
    )
    .into_bytes();
    let expected_wrapper_sha256 = chrono_harness::sha256(&wrapper_bytes);
    fs::write(&wrapper, &wrapper_bytes).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let env: BTreeMap<String, String> = std::env::vars().collect();
    for field in ["digest", "version"] {
        let mut wrong = config.clone();
        if field == "digest" {
            wrong["tools"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|tool| tool["id"] == "git")
                .unwrap()["program"] = json!(wrapper);
            wrong["environment"]["inputs"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|input| input["id"] == "git-executable")
                .unwrap()["location"] = json!(wrapper);
        } else {
            wrong["tools"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|tool| tool["id"] == "git")
                .unwrap()["expected_version"] = json!("git version deliberately-wrong-platform");
        }
        fs::write(
            root.join(CONFIG),
            serde_json::to_vec_pretty(&wrong).unwrap(),
        )
        .unwrap();
        let wrong_candidate = commit(&root);
        let new = facts::registry_values(&root, &wrong_candidate, CONFIG).unwrap();
        let error = interpret(
            &root,
            &base,
            &wrong_candidate,
            CONFIG,
            raw.clone(),
            new,
            &env,
        )
        .err()
        .expect("copied mismatched Git binding unexpectedly accepted");
        if field == "digest" {
            assert_eq!(error, "E_GIT_FACTS: facts_git declared digest mismatch");
        } else {
            let diagnostic: Value =
                serde_json::from_str(error.strip_prefix("E_GIT_FACTS: ").expect(&error))
                    .unwrap_or_else(|parse| {
                        panic!("invalid Git binding diagnostic: {parse}; {error}")
                    });
            assert_eq!(
                diagnostic["message"], "Git facts version mismatch",
                "{error}"
            );
            assert_eq!(
                diagnostic["observation"]["binding"]["expected_version"],
                "git version deliberately-wrong-platform",
                "{error}"
            );
            assert_eq!(
                chrono_harness::full::expand_process(&diagnostic["observation"]["processes"][0])
                    .unwrap()["stdout"],
                format!(
                    "{}\n",
                    config["tools"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|tool| tool["id"] == "git")
                        .unwrap()["expected_version"]
                        .as_str()
                        .unwrap()
                ),
                "{error}"
            );
        }
        adopt_fixture_bindings(&root, &mut wrong);
        if field == "digest" {
            // Keep the real forwarding executable as the consumer. Its expected
            // identity comes from the declared fixture bytes, not observation.
            wrong["tools"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|tool| tool["id"] == "git")
                .unwrap()["program"] = json!(wrapper);
            let input = wrong["environment"]["inputs"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|input| input["id"] == "git-executable")
                .unwrap();
            input["location"] = json!(wrapper);
            input["sha256"] = json!(expected_wrapper_sha256);
        }

        fs::write(
            root.join(CONFIG),
            serde_json::to_vec_pretty(&wrong).unwrap(),
        )
        .unwrap();
        let repaired_candidate = commit(&root);
        let mut new = facts::registry_values(&root, &repaired_candidate, CONFIG).unwrap();
        new.get_mut(CONFIG).unwrap()["tools"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|tool| tool["id"] == "python3")
            .unwrap()["expected_version"] = json!("Python deliberately-wrong");
        let error = interpret(
            &root,
            &base,
            &repaired_candidate,
            CONFIG,
            raw.clone(),
            new,
            &env,
        )
        .err()
        .expect("wrong Python binding unexpectedly accepted after adopting Git");
        assert!(
            error.starts_with("E_TOOL_BINDING: migration version mismatch"),
            "{error}"
        );
    }
}
