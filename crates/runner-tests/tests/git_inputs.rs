#[path = "support/native_executable.rs"]
mod native_executable;

use chrono_harness::{facts::Reader, sha256};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command};

struct Host {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    tool: PathBuf,
    config: Value,
}
impl Host {
    fn new(mut case: Value) -> Self {
        let temporary = tempfile::Builder::new()
            .prefix("Git inputs λ ")
            .tempdir_in(
                std::path::Path::new(env!("CARGO_BIN_EXE_chrono-test-git-input"))
                    .parent()
                    .unwrap(),
            )
            .unwrap();
        fs::create_dir(temporary.path().join("host")).unwrap();
        fs::create_dir(temporary.path().join("tools")).unwrap();
        let root = fs::canonicalize(temporary.path().join("host")).unwrap();
        let tools = fs::canonicalize(temporary.path().join("tools")).unwrap();
        let tool = tools.join("git-wrapper");
        fs::create_dir(root.join(".chrono-harness")).unwrap();
        let git = chrono_harness::resolve_program(&root, "git", None).unwrap();
        let result = Command::new(&git)
            .args(["init", "--quiet"])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(result.status.success());
        let version = Command::new(&git).arg("--version").output().unwrap();
        assert!(version.status.success());
        native_executable::install(env!("CARGO_BIN_EXE_chrono-test-git-input"), &tool).unwrap();
        case["git"] = json!(git);
        case["root"] = json!(root);
        fs::write(
            tools.join("git-case.json"),
            serde_json::to_vec(&case).unwrap(),
        )
        .unwrap();
        let config = json!({
            "schema_version":3,
            "facts_git":{"tool":"git","input":"git-binary","guard":{
                "schema":"chrono-git-inputs/v1","inputs":["git-config","no-extra-config"]}},
            "tools":[{"id":"git","program":"git-wrapper","resolution":"PATH-once",
                "version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}],
            "environment":{"inherit":[],"values":{"PATH":tools,"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"},
                "inputs":[{"id":"git-binary","location":tool,"presence":"present","sha256":sha256(&fs::read(&tool).unwrap())},
                    {"id":"git-config","location":".git/config","presence":"present","sha256":sha256(&fs::read(root.join(".git/config")).unwrap())},
                    {"id":"no-extra-config","location":".git/config.worktree","presence":"absent"}]},
            "protocol":{"timeout_seconds":30,"stdout_limit_bytes":1048576}
        });
        Self {
            _temporary: temporary,
            root,
            tool,
            config,
        }
    }
    fn save(&self) {
        fs::write(
            self.root.join(".chrono-harness/config.json"),
            serde_json::to_vec(&self.config).unwrap(),
        )
        .unwrap();
    }
    fn open(&self) -> Result<Reader, chrono_harness::facts::OpenFailure> {
        self.save();
        Reader::for_config_observed(&self.root, ".chrono-harness/config.json")
    }
}

// Byte assertions consume the codec through the actual public decoder.
fn expanded_observation(reader: &Reader) -> Value {
    let mut observed = reader.observation();
    for process in observed["processes"].as_array_mut().unwrap() {
        *process = chrono_harness::full::expand_process(process).unwrap();
    }
    observed
}

#[test]
fn actual_registry_test_inputs_have_explicit_runner_test_consumers() {
    let product = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = ".chrono-harness/config.json";
    let config: Value = serde_json::from_slice(&fs::read(product.join(path)).unwrap()).unwrap();
    let filemap: Value =
        serde_json::from_slice(&fs::read(product.join(".chrono-harness/FILEMAP.json")).unwrap())
            .unwrap();
    for input in chrono_harness::facts::registry_paths(&config, path).unwrap() {
        let declarations: Vec<_> = filemap["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|file| file["path"] == input)
            .collect();
        assert_eq!(declarations.len(), 1, "real test input {input}");
        let file = declarations[0];
        assert!(
            file["edges"].as_array().unwrap().iter().any(|edge| {
                edge["kind"] == "runtime-input" && edge["to"] == "project:runner-tests"
            }),
            "missing actual registry test consumer for {input}"
        );
    }
    assert!(
        filemap["project_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| {
                edge["from"] == "project:runner-tests"
                    && edge["kind"] == "test-execution"
                    && edge["to"] == "test:runner-tests"
            })
    );
}

#[test]
fn facts_observation_v2_is_lossless_and_request_consumers_gate_v1_before_git() {
    let h = Host::new(json!({"mode":"passthrough"}));
    h.save();
    let oid = commit(&h);
    let reader = h.open().unwrap();
    let current = reader.observation();
    assert_eq!(current["schema"], "chrono-git-facts/v2");
    let expanded = expanded_observation(&reader);
    for process in expanded["processes"].as_array().unwrap() {
        let raw: chrono_harness::ProcessResult = serde_json::from_value(process.clone()).unwrap();
        chrono_harness::observation::process_success(&raw).unwrap();
    }
    let environment = current["environment"].clone();
    let consumer = Reader::from_observations(
        &h.root,
        CONFIG,
        &oid,
        &json!({"environment":environment,"git_facts":current}),
    )
    .unwrap();
    assert_eq!(consumer.observation()["schema"], "chrono-git-facts/v2");
    let before = fs::read(h.root.join("trace")).unwrap();
    for schema in [
        Some(json!("chrono-git-facts/v1")),
        Some(json!("unknown")),
        Some(Value::Null),
        None,
    ] {
        let mut prior = expanded.clone();
        match &schema {
            Some(schema) => prior["schema"] = schema.clone(),
            None => {
                prior.as_object_mut().unwrap().remove("schema");
            }
        }
        let error = match Reader::from_observations(
            &h.root,
            CONFIG,
            &oid,
            &json!({"environment":environment,"git_facts":prior}),
        ) {
            Ok(_) => panic!("unsupported request schema accepted: {schema:?}"),
            Err(error) => error,
        };
        assert!(
            error.contains("unsupported request Git facts schema"),
            "{error}"
        );
        assert_eq!(fs::read(h.root.join("trace")).unwrap(), before);
    }
    assert!(Reader::legacy().observation().is_null());
}

#[test]
fn actual_host_registry_bytes_have_lossless_bounded_process_transport() {
    let mut h = Host::new(json!({"mode":"passthrough"}));
    let original = fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.chrono-harness/config.json"),
    )
    .unwrap();
    let path = ".chrono-harness/registry-source.json";
    let product = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config: Value = serde_json::from_slice(&original).unwrap();
    let mut originals = std::collections::BTreeMap::new();
    for registry in chrono_harness::facts::registry_paths(&config, path).unwrap() {
        let bytes = if registry == path {
            original.clone()
        } else {
            fs::read(product.join(&registry)).unwrap()
        };
        fs::create_dir_all(h.root.join(&registry).parent().unwrap()).unwrap();
        fs::write(h.root.join(&registry), &bytes).unwrap();
        originals.insert(registry, bytes);
    }
    h.config["protocol"]["stdout_limit_bytes"] = json!(8 * 1024 * 1024);
    h.save();
    let oid = commit(&h);
    let reader = h.open().unwrap();
    reader.verify_oid(&h.root, &oid).unwrap();
    let base = reader.registry_snapshot(&h.root, &oid, path).unwrap();
    assert_eq!(base.bytes, originals);
    fs::write(h.root.join("candidate-source"), "a real second endpoint").unwrap();
    let candidate = commit(&h);
    reader.verify_oid(&h.root, &candidate).unwrap();
    let current = reader.registry_snapshot(&h.root, &candidate, path).unwrap();
    assert_eq!(current.bytes, originals);
    let observed = reader.observation();
    let wire_bytes = serde_json::to_vec(&observed).unwrap();
    let product = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let retained = chrono_harness::prepared::retain_original(
        &product,
        ".chrono-harness/state/preparation/",
        "registry-transport",
        &wire_bytes,
    )
    .unwrap();
    println!(
        "ACTUAL_REGISTRY_TRANSPORT config_bytes={} registry_bytes_per_endpoint={} registries={} processes={} observation_bytes={} original={} sha256={}",
        original.len(),
        originals.values().map(Vec::len).sum::<usize>(),
        originals.len(),
        observed["processes"].as_array().unwrap().len(),
        wire_bytes.len(),
        retained.path,
        retained.sha256
    );
    assert!(
        wire_bytes.len() < 8 * 1024 * 1024,
        "actual host evidence must fit the existing check bound"
    );
    for row in observed["processes"].as_array().unwrap() {
        let expanded = chrono_harness::full::expand_process(row).unwrap();
        let process: chrono_harness::ProcessResult = serde_json::from_value(expanded).unwrap();
        chrono_harness::observation::process_success(&process).unwrap();
    }
    let last = observed["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["stdout_sha256"] == sha256(&original))
        .unwrap();
    assert_eq!(last["encoding"], "chrono-retained-process/v2");
    assert!(last.get("stdout").is_none() && last.get("stdout_bytes").is_none());
    let expanded = chrono_harness::full::expand_process(last).unwrap();
    let process: chrono_harness::ProcessResult = serde_json::from_value(expanded).unwrap();
    chrono_harness::observation::process_success(&process).unwrap();
    assert_eq!(process.stdout_bytes, original);
    let mut corrupt = last.clone();
    corrupt["stdout_sha256"] = json!("0".repeat(64));
    assert!(chrono_harness::full::expand_process(&corrupt).is_err());
}

#[test]
fn explicit_git_configuration_is_checked_before_each_real_read() {
    let h = Host::new(json!({"mode":"passthrough"}));
    let reader = h.open().unwrap_or_else(|e| panic!("{}", e.message));
    let bytes = reader
        .git(
            &h.root,
            &["config", "--local", "--get", "core.repositoryformatversion"],
        )
        .unwrap();
    assert_eq!(bytes, b"0\n");
    let observed = expanded_observation(&reader);
    assert_eq!(observed["inputs"]["schema"], "chrono-git-inputs-result/v1");
    assert_eq!(
        observed["inputs"]["files"]["no-extra-config"]["presence"],
        "absent"
    );
    assert_eq!(observed["input_closure_complete"], false);
    let count = observed["processes"].as_array().unwrap().len();
    fs::write(
        h.root.join(".git/config"),
        "[core]\nrepositoryformatversion = 0\n# changed\n",
    )
    .unwrap();
    let error = reader
        .git(&h.root, &["rev-parse", "--show-toplevel"])
        .unwrap_err();
    assert!(error.contains("git-config"), "{error}");
    assert_eq!(
        expanded_observation(&reader)["processes"]
            .as_array()
            .unwrap()
            .len(),
        count
    );
}

#[test]
fn artifact_inventory_preserves_noncanonical_paths_and_checks_bound_inputs() {
    let h = Host::new(json!({"mode":"passthrough"}));
    fs::create_dir_all(h.root.join("cache[1]/allowed")).unwrap();
    fs::write(h.root.join("cache[1]/allowed/output"), "generated").unwrap();
    let reader = h.open().unwrap_or_else(|e| panic!("{}", e.message));
    let raw = reader
        .git(&h.root, &["ls-files", "--others", "-z"])
        .unwrap();
    assert!(
        raw.split(|b| *b == 0)
            .any(|p| p == b"cache[1]/allowed/output")
    );
    let paths = reader
        .untracked_excluding(&h.root, &["cache[1]/allowed/"])
        .unwrap();
    assert!(!paths.iter().any(|p| p == "cache[1]/allowed/output"));
    for declaration in [
        "cache[1]//allowed/",
        "cache[1]/./allowed/",
        "./cache[1]/allowed/",
        "cache[1]/allowed",
        "/",
        "",
    ] {
        let paths = reader.untracked_excluding(&h.root, &[declaration]).unwrap();
        assert!(
            paths.iter().any(|p| p == "cache[1]/allowed/output"),
            "normalized {declaration:?}"
        );
    }
    let count = expanded_observation(&reader)["processes"]
        .as_array()
        .unwrap()
        .len();
    fs::write(h.root.join(".git/config.worktree"), "changed").unwrap();
    let error = reader
        .untracked_excluding(&h.root, &["cache[1]/allowed/"])
        .unwrap_err();
    assert!(error.contains("no-extra-config"), "{error}");
    assert_eq!(
        expanded_observation(&reader)["processes"]
            .as_array()
            .unwrap()
            .len(),
        count
    );
}

#[test]
fn mismatches_and_undeclared_guard_references_do_not_launch_git() {
    for case in [
        "digest",
        "presence",
        "missing",
        "duplicate",
        "empty",
        "unknown-field",
        "schema",
        "directory",
        "symlink",
    ] {
        let mut h = Host::new(json!({"mode":"passthrough"}));
        match case {
            "digest" => h.config["environment"]["inputs"][1]["sha256"] = json!("0".repeat(64)),
            "presence" => fs::write(h.root.join(".git/config.worktree"), "").unwrap(),
            "missing" => h.config["facts_git"]["guard"]["inputs"] = json!(["unknown"]),
            "duplicate" => {
                h.config["facts_git"]["guard"]["inputs"] = json!(["git-config", "git-config"])
            }
            "empty" => h.config["facts_git"]["guard"]["inputs"] = json!([]),
            "unknown-field" => h.config["facts_git"]["guard"]["discover"] = json!(true),
            "schema" => h.config["facts_git"]["guard"]["schema"] = json!("chrono-git-inputs/v999"),
            "directory" => fs::create_dir(h.root.join(".git/config.worktree")).unwrap(),
            "symlink" => {
                std::os::unix::fs::symlink("config", h.root.join(".git/config.worktree")).unwrap()
            }
            _ => unreachable!(),
        }
        let failure = match h.open() {
            Ok(_) => panic!("{case} accepted"),
            Err(e) => e,
        };
        assert!(
            failure.message.contains("E_GIT_FACTS"),
            "{case}: {}",
            failure.message
        );
        assert!(!h.root.join("trace").exists(), "{case} launched Git");
    }
}

#[test]
fn a_git_process_mutating_an_input_preserves_its_original_failed_receipt() {
    let h = Host::new(json!({"mode":"input-drift"}));
    let reader = h.open().unwrap_or_else(|e| panic!("{}", e.message));
    let error = reader
        .git(&h.root, &["rev-parse", "--show-toplevel"])
        .unwrap_err();
    assert!(error.contains("no-extra-config"), "{error}");
    let observed = expanded_observation(&reader);
    let process = observed["processes"].as_array().unwrap().last().unwrap();
    assert_eq!(process["exit_code"], 17);
    assert_eq!(process["stdout"], "original output");
    assert_eq!(process["stderr"], "original error");
    assert_eq!(process["stdout_sha256"], sha256(b"original output"));
}

#[test]
fn legacy_binding_has_no_guard_and_current_observations_cannot_omit_it() {
    let mut h = Host::new(json!({"mode":"passthrough"}));
    h.config["facts_git"]
        .as_object_mut()
        .unwrap()
        .remove("guard");
    let reader = h.open().unwrap_or_else(|e| panic!("{}", e.message));
    assert!(expanded_observation(&reader).get("inputs").is_none());
    fs::write(h.root.join(".git/config.worktree"), "legacy semantics").unwrap();
    assert!(
        reader
            .git(&h.root, &["rev-parse", "--show-toplevel"])
            .is_ok()
    );

    let h = Host::new(json!({"mode":"passthrough"}));
    let reader = h.open().unwrap_or_else(|e| panic!("{}", e.message));
    let mut observation = expanded_observation(&reader);
    let environment = observation["environment"].clone();
    observation.as_object_mut().unwrap().remove("inputs");
    let result = Reader::from_observations(
        &h.root,
        ".chrono-harness/config.json",
        &"a".repeat(40),
        &json!({"git_facts":observation,"environment":environment}),
    );
    let error = match result {
        Ok(_) => panic!("missing inputs accepted"),
        Err(e) => e,
    };
    assert!(error.contains("request Git facts inputs"), "{error}");
}

const SELECTOR: &str = ".chrono-harness/git-platforms.json";
const CONFIG: &str = ".chrono-harness/config.json";

fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}
fn select(h: &Host, selector: &Value) {
    h.save();
    fs::write(h.root.join(SELECTOR), serde_json::to_vec(selector).unwrap()).unwrap();
}
fn selection() -> Value {
    json!({"schema":"chrono-git-configs/v1","platforms":{platform():CONFIG}})
}
fn commit(h: &Host) -> String {
    let git = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    for args in [
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-qm",
            "registered platform facts",
        ],
    ] {
        assert!(
            Command::new(&git)
                .current_dir(&h.root)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    let output = Command::new(git)
        .current_dir(&h.root)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().into()
}

#[test]
fn selected_platform_binds_both_candidate_files_and_request_observation() {
    let h = Host::new(json!({"mode":"passthrough"}));
    let mut selector = selection();
    // Another registered platform need not exist on this machine.
    selector["platforms"]["other-system"] = json!(".chrono-harness/elsewhere.json");
    select(&h, &selector);
    let candidate = commit(&h);
    let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
    assert!(reader.is_bound());
    reader.verify_config(&h.root, &candidate).unwrap();
    let observed = expanded_observation(&reader);
    assert_eq!(observed["config_path"], CONFIG);
    assert_eq!(observed["selection"]["path"], SELECTOR);
    assert_eq!(observed["selection"]["platform"], platform());
    assert_eq!(
        observed["selection"]["sha256"],
        sha256(&fs::read(h.root.join(SELECTOR)).unwrap())
    );
    assert_eq!(observed["input_closure_complete"], false);
    let environment = observed["environment"].clone();
    Reader::from_observations(
        &h.root,
        SELECTOR,
        &candidate,
        &json!({"git_facts":observed,"environment":environment}),
    )
    .unwrap();
    for key in ["platform", "sha256", "path"] {
        let mut substituted = observed.clone();
        substituted["selection"][key] = json!("unbound");
        let result = Reader::from_observations(
            &h.root,
            SELECTOR,
            &candidate,
            &json!({"git_facts":substituted,"environment":environment}),
        );
        assert!(
            result
                .err()
                .unwrap()
                .contains("request Git facts selection")
        );
    }
    let mut missing = observed;
    missing.as_object_mut().unwrap().remove("selection");
    assert!(
        Reader::from_observations(
            &h.root,
            SELECTOR,
            &candidate,
            &json!({"git_facts":missing,"environment":environment})
        )
        .is_err()
    );
}

#[test]
fn invalid_platform_maps_and_unbound_targets_never_launch_git() {
    for case in [
        "empty",
        "unregistered",
        "outside",
        "self",
        "nested",
        "legacy",
        "missing",
        "unknown",
        "empty-key",
        "duplicate",
        "symlink",
    ] {
        let mut h = Host::new(json!({"mode":"passthrough"}));
        let mut selector = selection();
        match case {
            "empty" => selector["platforms"] = json!({}),
            "unregistered" => selector["platforms"] = json!({"another-system":CONFIG}),
            "outside" => selector["platforms"][platform()] = json!("../outside.json"),
            "self" => selector["platforms"][platform()] = json!(SELECTOR),
            "nested" => h.config = selection(),
            "legacy" => h.config["schema_version"] = json!(2),
            "missing" => selector["platforms"][platform()] = json!(".chrono-harness/missing.json"),
            "unknown" => selector["fallback"] = json!(CONFIG),
            "empty-key" => selector["platforms"][""] = json!(CONFIG),
            "duplicate" | "symlink" => {}
            _ => unreachable!(),
        }
        select(&h, &selector);
        if case == "duplicate" {
            fs::write(h.root.join(SELECTOR), format!(
                "{{\"schema\":\"chrono-git-configs/v1\",\"platforms\":{{\"{0}\":\"{1}\",\"{0}\":\"{1}\"}}}}",
                platform(), CONFIG)).unwrap();
        }
        if case == "symlink" {
            fs::rename(
                h.root.join(CONFIG),
                h.root.join(".chrono-harness/real.json"),
            )
            .unwrap();
            std::os::unix::fs::symlink("real.json", h.root.join(CONFIG)).unwrap();
        }
        assert!(
            Reader::for_config(&h.root, SELECTOR).is_err(),
            "{case} accepted"
        );
        assert!(!h.root.join("trace").exists(), "{case} launched Git");
    }
}

#[test]
fn platform_selector_and_selected_policy_drift_block_the_next_process() {
    for path in [SELECTOR, CONFIG] {
        let h = Host::new(json!({"mode":"passthrough"}));
        select(&h, &selection());
        let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
        assert!(reader.is_bound());
        let before = h.root.join(path);
        let bytes = fs::read(&before).unwrap();
        let count = expanded_observation(&reader)["processes"]
            .as_array()
            .unwrap()
            .len();
        fs::write(&before, b"{}").unwrap();
        assert!(
            reader
                .git(&h.root, &["rev-parse", "--show-toplevel"])
                .is_err()
        );
        assert_eq!(
            expanded_observation(&reader)["processes"]
                .as_array()
                .unwrap()
                .len(),
            count
        );
        fs::write(&before, bytes).unwrap();
        reader
            .git(&h.root, &["rev-parse", "--show-toplevel"])
            .unwrap();
    }
}

#[test]
fn selector_mutation_during_git_retains_the_original_failed_process() {
    let h = Host::new(json!({"mode":"selector-drift"}));
    select(&h, &selection());
    let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
    assert!(reader.is_bound());
    let error = reader
        .git(&h.root, &["rev-parse", "--show-toplevel"])
        .unwrap_err();
    assert!(error.contains("selector changed"), "{error}");
    let observation = expanded_observation(&reader);
    let process = observation["processes"].as_array().unwrap().last().unwrap();
    assert_eq!(process["exit_code"], 17);
    assert_eq!(process["stdout_sha256"], sha256(b"original"));
}

#[test]
fn platform_selection_must_match_both_fixed_candidate_blobs() {
    for path in [SELECTOR, CONFIG] {
        let mut h = Host::new(json!({"mode":"passthrough"}));
        let selector = selection();
        select(&h, &selector);
        let candidate = commit(&h);
        if path == SELECTOR {
            let mut changed = selector;
            changed["platforms"]["future-system"] = json!(".chrono-harness/future.json");
            select(&h, &changed);
        } else {
            h.config["environment"]["values"]["DECLARED_VALUE"] = json!("changed");
            h.save();
        }
        let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
        assert!(reader.is_bound());
        let error = reader.verify_config(&h.root, &candidate).unwrap_err();
        assert!(error.contains("fixed candidate"), "{path}: {error}");
    }
}

fn processes(reader: &Reader) -> usize {
    expanded_observation(&reader)["processes"]
        .as_array()
        .unwrap()
        .len()
}

#[test]
fn snapshot_export_batches_literal_blobs_and_preserves_original_bytes() {
    let h = Host::new(json!({"mode":"passthrough"}));
    h.save();
    fs::create_dir(h.root.join("objects")).unwrap();
    let mut originals = std::collections::BTreeMap::new();
    for n in 0..32u8 {
        let path = format!("objects/{n:02} café\t:key");
        let bytes = vec![n, 0xff, 0, b'\n'];
        fs::write(h.root.join(&path), &bytes).unwrap();
        originals.insert(path, bytes);
    }
    fs::write(h.root.join("empty"), b"").unwrap();
    std::os::unix::fs::symlink("objects/00 café\t:key", h.root.join("alias")).unwrap();
    let oid = commit(&h);
    let reader = h.open().unwrap();
    reader.verify_oid(&h.root, &oid).unwrap();
    let tree = reader.tree(&h.root, &oid).unwrap();
    // Already observed paths share their original receipt with the export.
    let first = originals.keys().next().unwrap();
    reader.blob(&h.root, &oid, first).unwrap();
    let before = processes(&reader);
    let dest = tempfile::tempdir().unwrap();
    reader.export(&h.root, &oid, &tree, dest.path()).unwrap();
    for (path, bytes) in &originals {
        assert_eq!(fs::read(dest.path().join(path)).unwrap(), *bytes);
        assert!(
            fs::metadata(dest.path().join(path))
                .unwrap()
                .permissions()
                .readonly()
        );
    }
    assert_eq!(fs::read(dest.path().join("empty")).unwrap(), b"");
    assert_eq!(
        fs::read_link(dest.path().join("alias")).unwrap(),
        PathBuf::from(first)
    );
    let acquired = processes(&reader) - before;
    println!(
        "SNAPSHOT_EXPORT blobs={} git_processes={acquired}",
        tree.len()
    );
    assert!(
        acquired <= 3,
        "small immutable blobs must not launch one Git process per file: {acquired}"
    );
    let observed = expanded_observation(&reader);
    for process in observed["processes"].as_array().unwrap() {
        let process: chrono_harness::ProcessResult =
            serde_json::from_value(process.clone()).unwrap();
        chrono_harness::observation::process_success(&process).unwrap();
    }
    let next = tempfile::tempdir().unwrap();
    reader.export(&h.root, &oid, &tree, next.path()).unwrap();
    assert_eq!(
        expanded_observation(&reader),
        observed,
        "reuse retains original processes"
    );
    fs::write(h.root.join(".git/config.worktree"), "drift").unwrap();
    let rejected = tempfile::tempdir().unwrap();
    assert!(
        reader
            .export(&h.root, &oid, &tree, rejected.path())
            .is_err()
    );
    assert_eq!(
        expanded_observation(&reader),
        observed,
        "cached bytes retain input guards"
    );
}

#[test]
fn snapshot_export_retains_failed_batch_and_reacquires_on_retry() {
    let (h, originals, oid) = registry_host(json!({"mode":"batch-fail"}), 20);
    let reader = h.open().unwrap();
    reader.verify_oid(&h.root, &oid).unwrap();
    let tree = reader.tree(&h.root, &oid).unwrap();
    let dest = tempfile::tempdir().unwrap();
    let error = reader
        .export(&h.root, &oid, &tree, dest.path())
        .unwrap_err();
    assert!(error.contains("Git facts process exit 17"), "{error}");
    let observed = expanded_observation(&reader);
    let failure = observed["processes"].as_array().unwrap().last().unwrap();
    assert_eq!(failure["exit_code"], 17);
    assert_eq!(failure["stdout_sha256"], sha256(b"original partial"));
    assert_eq!(failure["stderr_sha256"], sha256(b"original error"));
    fs::write(h.root.join("retry-ready"), b"").unwrap();
    reader.export(&h.root, &oid, &tree, dest.path()).unwrap();
    assert!(processes(&reader) > observed["processes"].as_array().unwrap().len());
    for (path, bytes) in originals {
        assert_eq!(fs::read(dest.path().join(path)).unwrap(), bytes);
    }
    let after = expanded_observation(&reader);
    assert_eq!(
        &after["processes"].as_array().unwrap()[..observed["processes"].as_array().unwrap().len()],
        observed["processes"].as_array().unwrap()
    );
}

#[test]
fn fixed_registry_reads_reuse_real_acquisitions_without_replaying_observations() {
    let mut h = Host::new(json!({"mode":"passthrough"}));
    let mut registries = serde_json::Map::new();
    for role in ["judges", "projects", "filemap", "workflow"] {
        let path = format!(".chrono-harness/{role} café.json");
        fs::write(h.root.join(&path), format!("{{ \"role\":\"{role}\" }}\n\n")).unwrap();
        registries.insert(role.into(), json!(path));
    }
    h.config["registries"] = registries.into();
    select(&h, &selection());
    let candidate = commit(&h);
    let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
    let start = std::time::Instant::now();
    reader.verify_oid(&h.root, &candidate).unwrap();
    reader.verify_config(&h.root, &candidate).unwrap();
    let first = reader
        .registry_snapshot(&h.root, &candidate, SELECTOR)
        .unwrap();
    let original = expanded_observation(&reader);
    let trace = fs::read(h.root.join("trace")).unwrap();
    for _ in 0..32 {
        reader.verify_config(&h.root, &candidate).unwrap();
        let next = reader
            .registry_snapshot(&h.root, &candidate, SELECTOR)
            .unwrap();
        assert_eq!(next.bytes, first.bytes);
        assert_eq!(next.values, first.values);
        assert_eq!(next.selection, first.selection);
        assert_eq!(next.effective_path, CONFIG);
    }
    println!(
        "FACTS_REUSE wall_seconds={} observation={}",
        start.elapsed().as_secs_f64(),
        expanded_observation(&reader)
    );
    assert_eq!(
        processes(&reader),
        7,
        "version, existing commit/tree observations, selector/config and two registry batch acquisitions"
    );
    assert_eq!(
        expanded_observation(&reader),
        original,
        "reuse is not a fresh process receipt"
    );
    assert_eq!(fs::read(h.root.join("trace")).unwrap(), trace);
    for process in original["processes"].as_array().unwrap() {
        let process = serde_json::from_value(process.clone()).unwrap();
        chrono_harness::observation::process_success(&process).unwrap();
    }
    let second = Reader::for_config(&h.root, SELECTOR).unwrap();
    second.verify_oid(&h.root, &candidate).unwrap();
    second.verify_config(&h.root, &candidate).unwrap();
    second
        .registry_snapshot(&h.root, &candidate, SELECTOR)
        .unwrap();
    assert_eq!(
        processes(&second),
        7,
        "a new Reader acquires its own evidence"
    );
    assert_ne!(fs::read(h.root.join("trace")).unwrap(), trace);
}

#[test]
fn immutable_reuse_separates_root_oid_and_exact_path_and_preserves_binary_bytes() {
    let h = Host::new(json!({"mode":"passthrough"}));
    h.save();
    let path = "payload café\t.bin";
    let bytes = b"\0\xff\nbody\0";
    fs::write(h.root.join(path), bytes).unwrap();
    fs::write(h.root.join("other.bin"), b"other").unwrap();
    let old = commit(&h);
    let reader = h.open().unwrap();
    reader.verify_oid(&h.root, &old).unwrap();
    assert_eq!(reader.blob(&h.root, &old, path).unwrap(), bytes);
    let original = expanded_observation(&reader);
    assert_eq!(reader.blob(&h.root, &old, path).unwrap(), bytes);
    assert_eq!(expanded_observation(&reader), original);
    assert_eq!(reader.blob(&h.root, &old, "other.bin").unwrap(), b"other");
    fs::write(h.root.join(path), b"new").unwrap();
    let new = commit(&h);
    reader.verify_oid(&h.root, &new).unwrap();
    assert_eq!(reader.blob(&h.root, &new, path).unwrap(), b"new");
    assert_eq!(reader.blob(&h.root, &old, path).unwrap(), bytes);
    let other = Host::new(json!({"mode":"passthrough"}));
    let count = processes(&reader);
    assert!(
        reader
            .blob(&other.root, &old, path)
            .unwrap_err()
            .contains("root mismatch")
    );
    assert!(reader.blob(&h.root, &old, "../payload").is_err());
    assert_eq!(processes(&reader), count);
    // Each eligible key is acquired only once, without rewriting the receipt bytes.
    assert_eq!(count, 8);
    let legacy = Reader::legacy();
    assert_eq!(legacy.blob(&h.root, &old, path).unwrap(), bytes);
    assert!(legacy.blob(&other.root, &old, path).is_err());
}

#[test]
fn immutable_tree_reuse_preserves_live_checkout_and_original_evidence() {
    let h = Host::new(json!({"mode":"passthrough"}));
    h.save();
    fs::write(h.root.join("payload"), b"original").unwrap();
    let old = commit(&h);
    let reader = h.open().unwrap();
    reader.verify_oid(&h.root, &old).unwrap();
    let tree = reader.tree(&h.root, &old).unwrap();
    let original = expanded_observation(&reader);
    assert_eq!(reader.tree(&h.root, &old).unwrap(), tree);
    assert_eq!(
        expanded_observation(&reader),
        original,
        "reuse must retain one original tree acquisition"
    );

    fs::write(h.root.join("payload"), b"changed").unwrap();
    assert!(
        reader
            .tracked_changes(&h.root, &old)
            .unwrap()
            .contains(&"payload".into())
    );
    let git = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    assert!(
        Command::new(git)
            .current_dir(&h.root)
            .args(["add", "payload"])
            .status()
            .unwrap()
            .success()
    );
    fs::write(h.root.join("payload"), b"original").unwrap();
    assert!(
        reader
            .tracked_changes(&h.root, &old)
            .unwrap()
            .contains(&"payload".into()),
        "a staged change is not cancelled by opposite worktree bytes"
    );
    let observed = expanded_observation(&reader);
    let reads = observed["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["argv"].as_array().unwrap().iter().any(|v| v == "ls-tree"))
        .count();
    assert_eq!(reads, 1);
    let other = Host::new(json!({"mode":"passthrough"}));
    let count = processes(&reader);
    assert!(
        reader
            .tree(&other.root, &old)
            .unwrap_err()
            .contains("root mismatch")
    );
    assert_eq!(processes(&reader), count);

    let fresh = h.open().unwrap();
    fresh.verify_oid(&h.root, &old).unwrap();
    let count = processes(&fresh);
    assert_eq!(fresh.tree(&h.root, &old).unwrap(), tree);
    assert_eq!(processes(&fresh), count + 1);
    let before = reader.tree(&h.root, "HEAD").unwrap();
    fs::write(h.root.join("payload"), b"next commit").unwrap();
    let new = commit(&h);
    assert_ne!(
        reader.tree(&h.root, "HEAD").unwrap()["payload"],
        before["payload"]
    );
    reader.verify_oid(&h.root, &new).unwrap();
    assert_ne!(
        reader.tree(&h.root, &new).unwrap()["payload"],
        tree["payload"]
    );
    assert_eq!(reader.tree(&h.root, &old).unwrap(), tree);
}

#[test]
fn invalid_or_drifted_tree_acquisition_is_retried_without_reusing_failure() {
    for drift in [false, true] {
        let h = Host::new(if drift {
            json!({"mode":"tree-drift"})
        } else {
            json!({"mode":"tree-malformed"})
        });
        h.save();
        fs::write(h.root.join("payload"), b"original").unwrap();
        let oid = commit(&h);
        let reader = h.open().unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        fs::write(h.root.join("fail-tree"), b"").unwrap();
        let count = processes(&reader);
        assert!(reader.tree(&h.root, &oid).is_err());
        let original = expanded_observation(&reader)["processes"][count].clone();
        fs::remove_file(h.root.join("fail-tree")).unwrap();
        if drift {
            fs::remove_file(h.root.join(".git/config.worktree")).unwrap();
        }
        assert!(reader.tree(&h.root, &oid).unwrap().contains_key("payload"));
        assert_eq!(processes(&reader), count + 2);
        assert_eq!(expanded_observation(&reader)["processes"][count], original);
    }
}

#[test]
fn cached_immutable_bytes_still_reject_every_bound_guard_drift_before_effects() {
    for case in [
        "selector",
        "config",
        "tool",
        "guard-present",
        "guard-absent",
        "symlink",
    ] {
        let h = Host::new(json!({"mode":"passthrough"}));
        select(&h, &selection());
        fs::write(h.root.join("payload"), b"original").unwrap();
        let oid = commit(&h);
        let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        assert_eq!(reader.blob(&h.root, &oid, "payload").unwrap(), b"original");
        assert!(reader.tree(&h.root, &oid).unwrap().contains_key("payload"));
        let trace = fs::read(h.root.join("trace")).unwrap();
        let count = processes(&reader);
        match case {
            "selector" => fs::write(h.root.join(SELECTOR), b"{}").unwrap(),
            "config" => fs::write(h.root.join(CONFIG), b"{}").unwrap(),
            "tool" => {
                fs::remove_file(&h.tool).unwrap();
                fs::write(&h.tool, b"changed").unwrap();
            }
            "guard-present" => fs::write(h.root.join(".git/config"), b"changed").unwrap(),
            "guard-absent" => fs::write(h.root.join(".git/config.worktree"), b"changed").unwrap(),
            "symlink" => {
                fs::rename(
                    h.root.join(CONFIG),
                    h.root.join(".chrono-harness/original.json"),
                )
                .unwrap();
                std::os::unix::fs::symlink("original.json", h.root.join(CONFIG)).unwrap();
            }
            _ => unreachable!(),
        }
        let error = reader.blob(&h.root, &oid, "payload").unwrap_err();
        assert!(error.contains("E_GIT_FACTS"), "{case}: {error}");
        assert!(
            reader
                .tree(&h.root, &oid)
                .unwrap_err()
                .contains("E_GIT_FACTS"),
            "{case}"
        );
        assert_eq!(processes(&reader), count, "{case}");
        assert_eq!(fs::read(h.root.join("trace")).unwrap(), trace, "{case}");
    }
}

#[test]
fn failed_or_postprocess_drifted_blob_reads_are_never_reused() {
    for drift in [false, true] {
        let body = if drift {
            json!({"mode":"blob-drift"})
        } else {
            json!({"mode":"blob-fail"})
        };
        let h = Host::new(body);
        h.save();
        fs::write(h.root.join("payload"), b"original\0\xff").unwrap();
        let oid = commit(&h);
        let reader = h.open().unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        assert!(reader.blob(&h.root, &oid, "payload").is_err());
        let observed = expanded_observation(&reader);
        let failed_index = processes(&reader) - 1;
        let failed = &observed["processes"][failed_index];
        assert_eq!(failed["exit_code"], if drift { 0 } else { 17 });
        assert_eq!(
            failed["stdout_sha256"],
            sha256(if drift {
                b"original\0\xff"
            } else {
                b"original output"
            })
        );
        if drift {
            fs::remove_file(h.root.join(".git/config.worktree")).unwrap();
        } else {
            assert!(reader.blob(&h.root, &oid, "payload").is_err());
            assert_eq!(processes(&reader), failed_index + 2);
            assert_eq!(failed["stderr_sha256"], sha256(b"original error"));
        }
        fs::write(h.root.join("retry-ready"), b"").unwrap();
        assert_eq!(
            reader.blob(&h.root, &oid, "payload").unwrap(),
            b"original\0\xff"
        );
        let success = expanded_observation(&reader);
        assert_eq!(
            reader.blob(&h.root, &oid, "payload").unwrap(),
            b"original\0\xff"
        );
        assert_eq!(expanded_observation(&reader), success);
        assert_eq!(success["processes"][failed_index], *failed);
        println!("FACTS_RETRY drift={drift} observation={success}");
    }
}

#[test]
fn mutable_refs_index_checkout_and_untracked_facts_remain_fresh() {
    let h = Host::new(json!({"mode":"passthrough"}));
    h.save();
    fs::write(h.root.join("payload"), b"old").unwrap();
    let old = commit(&h);
    let reader = h.open().unwrap();
    assert_eq!(reader.blob(&h.root, &old, "payload").unwrap(), b"old");
    assert_eq!(reader.blob(&h.root, "HEAD", "payload").unwrap(), b"old");
    let before = reader.checkout(&h.root, &old).unwrap();
    fs::write(h.root.join("payload"), b"new").unwrap();
    fs::write(h.root.join("new-untracked"), b"new").unwrap();
    let after = reader.checkout(&h.root, &old).unwrap();
    assert_ne!(before.tracked, after.tracked);
    assert!(after.untracked.iter().any(|path| path == "new-untracked"));
    let new = commit(&h);
    let count = processes(&reader);
    assert_eq!(reader.blob(&h.root, "HEAD", "payload").unwrap(), b"new");
    assert_eq!(reader.blob(&h.root, "HEAD", "payload").unwrap(), b"new");
    assert_eq!(processes(&reader), count + 2);
    assert_eq!(reader.checkout(&h.root, &new).unwrap().head, new);
    assert_eq!(reader.blob(&h.root, &old, "payload").unwrap(), b"old");
    let flags = reader.index_flags(&h.root).unwrap();
    let git = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    assert!(
        Command::new(git)
            .current_dir(&h.root)
            .args(["update-index", "--assume-unchanged", "payload"])
            .status()
            .unwrap()
            .success()
    );
    let changed_flags = reader.index_flags(&h.root).unwrap();
    assert_ne!(flags, changed_flags);
    assert!(
        changed_flags
            .iter()
            .any(|flag| flag.path == "payload" && flag.tag == "h")
    );
    for oid in ["f".repeat(40), "not-an-object".into()] {
        let count = processes(&reader);
        assert!(reader.blob(&h.root, &oid, "payload").is_err());
        assert!(reader.blob(&h.root, &oid, "payload").is_err());
        assert_eq!(
            processes(&reader),
            count + 2,
            "absence/failure is not cached"
        );
    }
}

#[test]
fn immutable_reuse_does_not_cross_reader_environment_or_binding_changes() {
    let mut h = Host::new(json!({"mode":"bound-value"}));
    h.config["environment"]["values"]["BOUND_VALUE"] = json!("first");
    h.save();
    fs::write(h.root.join("payload"), b"committed").unwrap();
    let oid = commit(&h);
    let first = h.open().unwrap();
    first.verify_oid(&h.root, &oid).unwrap();
    assert_eq!(first.blob(&h.root, &oid, "payload").unwrap(), b"first");
    let original = expanded_observation(&first);
    assert_eq!(first.blob(&h.root, &oid, "payload").unwrap(), b"first");
    assert_eq!(expanded_observation(&first), original);
    h.config["environment"]["values"]["BOUND_VALUE"] = json!("second");
    let second = h.open().unwrap();
    assert!(
        first
            .blob(&h.root, &oid, "payload")
            .unwrap_err()
            .contains("configuration changed")
    );
    assert_eq!(expanded_observation(&first), original);
    second.verify_oid(&h.root, &oid).unwrap();
    assert_eq!(second.blob(&h.root, &oid, "payload").unwrap(), b"second");
    let observed = expanded_observation(&second);
    assert_eq!(processes(&first), 4);
    assert_eq!(processes(&second), 4);
    assert_eq!(
        observed["processes"][3]["environment"]["BOUND_VALUE"],
        "second"
    );
    assert_eq!(observed["processes"][3]["stdout_sha256"], sha256(b"second"));
    println!("FACTS_BINDING first={original} second={observed}");
}

#[test]
fn oid_shaped_ref_for_another_object_format_is_still_mutable() {
    let h = Host::new(json!({"mode":"passthrough"}));
    h.save();
    fs::write(h.root.join("payload"), b"old").unwrap();
    let old = commit(&h);
    let git = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let format = Command::new(&git)
        .current_dir(&h.root)
        .args(["rev-parse", "--show-object-format"])
        .output()
        .unwrap();
    assert!(format.status.success());
    let refname = "a".repeat(if format.stdout == b"sha1\n" { 64 } else { 40 });
    let update = |oid: &str| {
        assert!(
            Command::new(&git)
                .current_dir(&h.root)
                .args(["update-ref", &format!("refs/heads/{refname}"), oid])
                .status()
                .unwrap()
                .success()
        );
    };
    update(&old);
    let reader = h.open().unwrap();
    assert!(reader.verify_oid(&h.root, &refname).is_err());
    assert_eq!(reader.blob(&h.root, &refname, "payload").unwrap(), b"old");
    fs::write(h.root.join("payload"), b"new").unwrap();
    let new = commit(&h);
    update(&new);
    assert_eq!(reader.blob(&h.root, &refname, "payload").unwrap(), b"new");
    println!(
        "FACTS_HEX_REF observation={}",
        expanded_observation(&reader)
    );
}

#[test]
fn reads_before_a_successful_identity_observation_stay_fresh_without_extra_probes() {
    let h = Host::new(json!({"mode":"passthrough"}));
    h.save();
    fs::write(h.root.join("payload"), b"original").unwrap();
    let oid = commit(&h);
    let reader = h.open().unwrap();
    for count in [2, 3] {
        assert_eq!(reader.blob(&h.root, &oid, "payload").unwrap(), b"original");
        assert_eq!(processes(&reader), count);
    }
    assert!(
        !fs::read_to_string(h.root.join("trace"))
            .unwrap()
            .contains("rev-parse")
    );
    reader.verify_oid(&h.root, &oid).unwrap();
    assert_eq!(reader.blob(&h.root, &oid, "payload").unwrap(), b"original");
    let original = expanded_observation(&reader);
    assert_eq!(reader.blob(&h.root, &oid, "payload").unwrap(), b"original");
    assert_eq!(expanded_observation(&reader), original);
    assert_eq!(processes(&reader), 6);
}

#[test]
fn malformed_or_failed_identity_observations_never_enable_reuse() {
    for case in [
        "type",
        "framing",
        "oid",
        "binary",
        "empty",
        "mismatch",
        "failed",
        "tree-failed",
        "tree-malformed",
        "invalid",
        "missing",
    ] {
        let h = Host::new(json!({"mode":"identity"}));
        h.save();
        fs::write(h.root.join("payload"), b"original").unwrap();
        let oid = commit(&h);
        let response = match case {
            "type" => Some(format!("{oid} unknown\n").into_bytes()),
            "framing" => Some(format!("{oid}\nextra\n").into_bytes()),
            "oid" => Some(b"not-an-object commit\n".to_vec()),
            "binary" => Some(vec![0xff, 0]),
            "empty" => Some(vec![]),
            "mismatch" => Some(format!("{}\n", "f".repeat(40)).into_bytes()),
            _ => None,
        };
        if let Some(bytes) = &response {
            fs::write(h.root.join("object-response"), bytes).unwrap();
        }
        if case == "failed" {
            fs::write(h.root.join("identity-fail"), b"").unwrap();
        }
        if case == "tree-failed" {
            fs::write(h.root.join("tree-fail"), b"").unwrap();
        }
        if case == "tree-malformed" {
            fs::write(h.root.join("tree-response"), b"not-a-tree\n").unwrap();
        }
        let reader = h.open().unwrap();
        let requested = match case {
            "invalid" => "invalid".into(),
            "missing" => "f".repeat(40),
            _ => oid.clone(),
        };
        let result = reader.verify_oid(&h.root, &requested);
        if case == "tree-malformed" {
            // Preserve the original verify_oid return contract, while refusing reuse.
            assert_eq!(result.unwrap(), "not-a-tree");
        } else {
            assert!(result.is_err(), "{case}");
        }
        let observed = expanded_observation(&reader);
        if let Some(bytes) = &response {
            let process = observed["processes"].as_array().unwrap().last().unwrap();
            assert_eq!(process["stdout_bytes"], json!(bytes));
            assert_eq!(process["stdin_sha256"], sha256(b""));
        }
        if case == "failed" {
            let failed = observed["processes"].as_array().unwrap().last().unwrap();
            assert_eq!(failed["exit_code"], 17);
            assert_eq!(failed["stderr_sha256"], sha256(b"original error"));
        }
        assert!(
            !fs::read_to_string(h.root.join("trace"))
                .unwrap()
                .contains(" show ")
        );
        let count = processes(&reader);
        for added in [1, 2] {
            assert_eq!(reader.blob(&h.root, &oid, "payload").unwrap(), b"original");
            assert_eq!(processes(&reader), count + added, "{case}");
        }
        println!(
            "FACTS_MALFORMED case={case} observation={}",
            expanded_observation(&reader)
        );
    }
}

fn registry_host(
    body: Value,
    payload_length: usize,
) -> (Host, std::collections::BTreeMap<String, Vec<u8>>, String) {
    let mut h = Host::new(body);
    let mut paths = serde_json::Map::new();
    let mut originals = std::collections::BTreeMap::new();
    for role in ["judges", "projects", "filemap", "workflow"] {
        let path = format!(".chrono-harness/{role} café\t:key.json");
        let bytes = format!(
            "{{ \"role\":\"{role}\",\"payload\":\"{}\" }}\n\n",
            "x".repeat(payload_length)
        )
        .into_bytes();
        fs::write(h.root.join(&path), &bytes).unwrap();
        paths.insert(role.into(), json!(path));
        originals.insert(path, bytes);
    }
    h.config["registries"] = paths.into();
    h.save();
    let oid = commit(&h);
    (h, originals, oid)
}

#[test]
fn registry_batch_retains_original_framed_processes_and_reuses_only_blob_ranges() {
    let (h, originals, oid) = registry_host(json!({"mode":"passthrough"}), 40);
    let reader = h.open().unwrap();
    reader.verify_oid(&h.root, &oid).unwrap();
    let start = std::time::Instant::now();
    let snapshot = reader.registry_snapshot(&h.root, &oid, CONFIG).unwrap();
    let elapsed = start.elapsed().as_secs_f64();
    for (path, bytes) in &originals {
        assert_eq!(&snapshot.bytes[path], bytes);
    }
    let observed = expanded_observation(&reader);
    println!("REGISTRY_ACQUISITION wall_seconds={elapsed} observation={observed}");
    assert_eq!(
        processes(&reader),
        6,
        "version, commit/tree, config, metadata and contents"
    );
    let process_list = observed["processes"].as_array().unwrap();
    let metadata: chrono_harness::ProcessResult =
        serde_json::from_value(process_list[4].clone()).unwrap();
    let contents: chrono_harness::ProcessResult =
        serde_json::from_value(process_list[5].clone()).unwrap();
    assert_eq!(metadata.argv.last().unwrap(), "--batch-check");
    assert_eq!(contents.argv.last().unwrap(), "--batch");
    let paths = chrono_harness::facts::registry_paths(&h.config, CONFIG).unwrap();
    let input = paths[1..]
        .iter()
        .map(|p| format!("{oid}:{p}\n"))
        .collect::<String>();
    assert_eq!(metadata.stdin_sha256, sha256(input.as_bytes()));
    let lines = metadata.stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 4);
    let content_input = lines
        .iter()
        .map(|line| format!("{}\n", line.split(' ').next().unwrap()))
        .collect::<String>();
    assert_eq!(contents.stdin_sha256, sha256(content_input.as_bytes()));
    let mut framed = vec![];
    for (line, path) in lines.iter().zip(&paths[1..]) {
        assert!(line.contains(" blob "));
        assert_eq!(
            line.split(' ').nth(2).unwrap().parse::<usize>().unwrap(),
            originals[path].len()
        );
        framed.extend_from_slice(format!("{line}\n").as_bytes());
        framed.extend_from_slice(&originals[path]);
        framed.push(b'\n');
    }
    assert_eq!(contents.stdout_bytes, framed);
    chrono_harness::observation::process_success(&metadata).unwrap();
    chrono_harness::observation::process_success(&contents).unwrap();
    for (path, bytes) in originals {
        assert_eq!(reader.blob(&h.root, &oid, &path).unwrap(), bytes);
    }
    assert_eq!(
        reader
            .registry_snapshot(&h.root, &oid, CONFIG)
            .unwrap()
            .bytes,
        snapshot.bytes
    );
    assert_eq!(
        expanded_observation(&reader),
        observed,
        "reuse retains the original receipt"
    );
}

#[test]
fn registry_batch_partitions_within_original_limit_and_preserves_exact_limit_blobs() {
    for exact_limit in [false, true] {
        let (mut h, mut originals, _) = registry_host(json!({"mode":"passthrough"}), 700);
        h.config["protocol"]["stdout_limit_bytes"] = json!(2048);
        if exact_limit {
            for (path, bytes) in &mut originals {
                *bytes = format!("{{\"p\":\"{}\"}}", "x".repeat(2040)).into_bytes();
                assert_eq!(bytes.len(), 2048);
                fs::write(h.root.join(path), &bytes).unwrap();
            }
        }
        h.save();
        let oid = commit(&h);
        let reader = h.open().unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        let snapshot = reader.registry_snapshot(&h.root, &oid, CONFIG).unwrap();
        for (path, bytes) in originals {
            assert_eq!(snapshot.bytes[&path], bytes);
            assert_eq!(reader.blob(&h.root, &oid, &path).unwrap(), bytes);
        }
        for process in expanded_observation(&reader)["processes"]
            .as_array()
            .unwrap()
        {
            let p: chrono_harness::ProcessResult = serde_json::from_value(process.clone()).unwrap();
            assert!(p.stdout_bytes.len() <= 2048);
            chrono_harness::observation::process_success(&p).unwrap();
        }
    }
}

#[test]
fn registry_batch_failures_and_malformed_originals_are_retained_and_never_reused() {
    for phase in ["--batch-check", "--batch"] {
        for response in [
            b"".as_slice(),
            b"not-an-object blob 1\n",
            b"0000000000000000000000000000000000000000 blob 0\n",
            b"tree tree 1\n",
            b"missing missing\n",
            b"malformed\0\xff",
            b"extra\n",
        ] {
            let body = json!({"mode":"batch-response","phase":phase});
            let (h, _, oid) = registry_host(body, 20);
            let reader = h.open().unwrap();
            reader.verify_oid(&h.root, &oid).unwrap();
            fs::write(h.root.join("response"), response).unwrap();
            for _ in 0..2 {
                let before = processes(&reader);
                let result = reader.registry_snapshot(&h.root, &oid, CONFIG);
                assert!(result.is_err(), "{phase}: {response:?}");
                let error = result.err().unwrap();
                assert!(processes(&reader) > before, "{phase}: {error}");
                let observed = expanded_observation(&reader);
                let last = observed["processes"].as_array().unwrap().last().unwrap();
                assert_eq!(last["stdout_bytes"], json!(response), "{phase}: {error}");
                assert_eq!(last["stdout_sha256"], sha256(response));
                assert_eq!(last["exit_code"], 0);
            }
            fs::remove_file(h.root.join("response")).unwrap();
            assert!(reader.registry_snapshot(&h.root, &oid, CONFIG).is_ok());
        }
    }
    for drift in [false, true] {
        let body = if drift {
            json!({"mode":"batch-drift"})
        } else {
            json!({"mode":"batch-fail"})
        };
        let (h, _, oid) = registry_host(body, 20);
        let reader = h.open().unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        assert!(reader.registry_snapshot(&h.root, &oid, CONFIG).is_err());
        let observed = expanded_observation(&reader);
        let failed_index = processes(&reader) - 1;
        let failed = observed["processes"][failed_index].clone();
        assert_eq!(failed["exit_code"], if drift { 0 } else { 17 });
        if drift {
            fs::remove_file(h.root.join(".git/config.worktree")).unwrap();
        } else {
            assert_eq!(failed["stdout_sha256"], sha256(b"original partial"));
            assert_eq!(failed["stderr_sha256"], sha256(b"original error"));
        }
        fs::write(h.root.join("retry-ready"), b"").unwrap();
        assert!(reader.registry_snapshot(&h.root, &oid, CONFIG).is_ok());
        assert_eq!(
            expanded_observation(&reader)["processes"][failed_index],
            failed
        );
    }
}

#[test]
fn registry_batch_rejects_corrupted_lengths_identities_payloads_and_framing() {
    let body = json!({"mode":"batch-response","phase":"--batch"});
    let (h, _, oid) = registry_host(body, 30);
    let first = h.open().unwrap();
    first.verify_oid(&h.root, &oid).unwrap();
    first.registry_snapshot(&h.root, &oid, CONFIG).unwrap();
    let observed = expanded_observation(&first);
    let valid: chrono_harness::ProcessResult = serde_json::from_value(
        observed["processes"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()
            .clone(),
    )
    .unwrap();
    let header_end = valid.stdout_bytes.iter().position(|b| *b == b'\n').unwrap();
    for case in [
        "oid",
        "type",
        "length",
        "payload",
        "terminator",
        "truncated",
        "trailing",
    ] {
        let mut raw = valid.stdout_bytes.clone();
        match case {
            "oid" => raw[0] = if raw[0] == b'a' { b'b' } else { b'a' },
            "type" => raw[41] = b't',
            "length" => raw[header_end - 1] = b'9',
            "payload" => raw[header_end + 1] = b'\0',
            "terminator" => *raw.last_mut().unwrap() = b'\0',
            "truncated" => {
                raw.truncate(header_end + 2);
            }
            "trailing" => raw.extend_from_slice(b"extra\n"),
            _ => unreachable!(),
        }
        fs::write(h.root.join("response"), &raw).unwrap();
        let reader = h.open().unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        for _ in 0..2 {
            let before = processes(&reader);
            assert!(
                reader.registry_snapshot(&h.root, &oid, CONFIG).is_err(),
                "{case}"
            );
            assert!(processes(&reader) > before);
            let observed = expanded_observation(&reader);
            let last = observed["processes"].as_array().unwrap().last().unwrap();
            assert_eq!(last["stdout_bytes"], json!(raw));
            assert_eq!(last["stdout_sha256"], sha256(&raw));
        }
    }
}

#[test]
fn registry_batch_keeps_configured_process_output_and_time_failures() {
    for case in ["output", "time"] {
        let body = if case == "output" {
            json!({"mode":"batch-output"})
        } else {
            json!({"mode":"batch-time"})
        };
        let (mut h, _, _) = registry_host(body, 20);
        h.config["protocol"]["stdout_limit_bytes"] = json!(2048);
        h.save();
        let oid = commit(&h);
        fs::write(h.root.join("oversized"), vec![b'x'; 4096]).unwrap();
        let reader = h.open().unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        assert!(
            reader.registry_snapshot(&h.root, &oid, CONFIG).is_err(),
            "{case}"
        );
        let observed = expanded_observation(&reader);
        let last = observed["processes"].as_array().unwrap().last().unwrap();
        assert_eq!(
            last["failure"],
            if case == "output" {
                "process output limit exceeded"
            } else {
                "process timed out"
            }
        );
        assert_eq!(
            last["stdout_bytes"].as_array().unwrap().len(),
            if case == "output" { 2048 } else { 0 }
        );
    }
}

#[test]
fn registry_batch_ranges_preserve_selector_root_and_all_bound_guards() {
    for case in [
        "selector",
        "config",
        "tool",
        "guard-present",
        "guard-absent",
        "root",
    ] {
        let (h, originals, _) = registry_host(json!({"mode":"passthrough"}), 20);
        select(&h, &selection());
        let oid = commit(&h);
        let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        reader.registry_snapshot(&h.root, &oid, SELECTOR).unwrap();
        let original = expanded_observation(&reader);
        let path = originals.keys().next().unwrap();
        let other = tempfile::tempdir().unwrap();
        let root = if case == "root" {
            other.path()
        } else {
            &h.root
        };
        match case {
            "selector" => fs::write(h.root.join(SELECTOR), b"{}").unwrap(),
            "config" => fs::write(h.root.join(CONFIG), b"{}").unwrap(),
            "tool" => {
                fs::remove_file(&h.tool).unwrap();
                fs::write(&h.tool, b"changed").unwrap();
            }
            "guard-present" => fs::write(h.root.join(".git/config"), b"changed").unwrap(),
            "guard-absent" => fs::write(h.root.join(".git/config.worktree"), b"changed").unwrap(),
            "root" => {}
            _ => unreachable!(),
        }
        assert!(reader.blob(root, &oid, path).is_err(), "{case}");
        assert_eq!(
            expanded_observation(&reader),
            original,
            "{case}: reject before process effects"
        );
    }
}

#[test]
fn unverified_registry_endpoints_and_mutable_refs_keep_original_reads() {
    let (h, originals, old) = registry_host(json!({"mode":"passthrough"}), 20);
    let reader = h.open().unwrap();
    for _ in 0..2 {
        let before = processes(&reader);
        let snapshot = reader.registry_snapshot(&h.root, &old, CONFIG).unwrap();
        assert_eq!(processes(&reader), before + 5);
        assert_eq!(
            snapshot.bytes[originals.keys().next().unwrap()],
            originals.values().next().unwrap().clone()
        );
    }
    reader.verify_oid(&h.root, &old).unwrap();
    let before = processes(&reader);
    reader.registry_snapshot(&h.root, "HEAD", CONFIG).unwrap();
    assert_eq!(processes(&reader), before + 5);
    let path = originals.keys().next().unwrap();
    fs::write(h.root.join(path), b"{\"changed\":true}").unwrap();
    let new = commit(&h);
    assert_ne!(new, old);
    assert_eq!(
        reader
            .registry_snapshot(&h.root, "HEAD", CONFIG)
            .unwrap()
            .values[path]["changed"],
        true
    );
    assert!(
        reader
            .registry_snapshot(&h.root, &old, CONFIG)
            .unwrap()
            .values[path]["changed"]
            .is_null()
    );
    assert!(
        Reader::legacy()
            .registry_snapshot(&h.root, &old, CONFIG)
            .is_ok()
    );
}

#[test]
fn immutable_input_contract_reads_complete_large_bytes_with_original_processes() {
    let h = Host::new(json!({"mode":"passthrough"}));
    let original = if let Some(path) = std::env::var_os("CHRONO_IMMUTABLE_INPUT_FIXTURE") {
        let bytes = fs::read(path).unwrap();
        assert_eq!(bytes.len(), 2_022_211);
        assert_eq!(
            sha256(&bytes),
            "15f1ec444ed15be8d5979257bf1e092d78a95081ec43aa77f5b973b25b955346"
        );
        bytes
    } else {
        vec![b'x'; 2_022_211]
    };
    let path = ".chrono-harness/immutable-input.json";
    fs::write(h.root.join(path), &original).unwrap();
    let oid = commit(&h);
    let git = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let digest = sha256(&fs::read(&git).unwrap());
    let mut spec = chrono_harness::CommandSpec {
        program: git.to_str().unwrap().into(),
        args: vec![
            "--no-replace-objects".into(),
            "show".into(),
            format!("{oid}:{path}"),
        ],
        env: [
            ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
            ("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()),
        ]
        .into(),
        timeout_seconds: 30,
        output_limit_bytes: 1_048_576,
    };
    let ordinary = chrono_harness::run_process_observed(&h.root, &spec, &[], &digest).unwrap();
    assert_eq!(
        ordinary.failure.as_deref(),
        Some("process output limit exceeded")
    );
    assert_eq!(ordinary.stdout_bytes.len(), 1_048_576);
    let mut receipts = vec![];
    let acquired = chrono_harness::facts::acquire_immutable_input(
        &oid,
        path,
        2_097_152,
        |args, input, limit| {
            spec.args = std::iter::once("--no-replace-objects".into())
                .chain(args.iter().map(|s| s.to_string()))
                .collect();
            let process = match limit {
                Some(limit) => {
                    chrono_harness::run_process_observed_data(&h.root, &spec, input, &digest, limit)
                }
                None => chrono_harness::run_process_observed(&h.root, &spec, input, &digest),
            }?;
            let result = chrono_harness::observation::process_success(&process)
                .map(|_| process.stdout_bytes.clone());
            receipts.push(process);
            result
        },
    )
    .unwrap();
    assert_eq!(acquired, original);
    assert_eq!(receipts.len(), 2);
    let content = &receipts[1];
    assert_eq!(content.stdout_bytes, original);
    assert_eq!(content.stdout_sha256, sha256(&original));
    assert_eq!(content.exit_code, 0);
    assert!(content.failure.is_none());
    assert!(content.stderr_bytes.is_empty());
    assert_eq!(content.stdin_sha256, sha256(b""));
    println!(
        "IMMUTABLE_INPUT bytes={} sha256={} diagnostic_limit={} data_declaration={} processes={}",
        acquired.len(),
        sha256(&acquired),
        spec.output_limit_bytes,
        2_097_152,
        receipts.len()
    );

    let mut metadata_calls = 0;
    let error = chrono_harness::facts::acquire_immutable_input(
        &oid,
        path,
        original.len() - 1,
        |args, input, limit| {
            assert_eq!(
                limit, None,
                "oversized data must be refused before acquisition"
            );
            metadata_calls += 1;
            spec.args = std::iter::once("--no-replace-objects".into())
                .chain(args.iter().map(|s| s.to_string()))
                .collect();
            let process = chrono_harness::run_process_observed(&h.root, &spec, input, &digest)?;
            chrono_harness::observation::process_success(&process)?;
            Ok(process.stdout_bytes)
        },
    )
    .unwrap_err();
    assert!(error.contains("declared limit"), "{error}");
    assert_eq!(metadata_calls, 1);

    let mut calls = 0;
    let mismatch = chrono_harness::facts::acquire_immutable_input(
        &oid,
        path,
        original.len(),
        |_, _, limit| {
            calls += 1;
            Ok(if limit.is_none() {
                receipts[0].stdout_bytes.clone()
            } else {
                vec![b'y'; original.len()]
            })
        },
    )
    .unwrap_err();
    assert_eq!(calls, 2);
    assert!(mismatch.contains("differ from object metadata"));
}

#[test]
fn immutable_input_data_transport_keeps_stderr_and_failure_bounds() {
    let h = Host::new(json!({"mode":"data-response"}));
    fs::write(h.root.join("data-stdout"), b"original partial\xff").unwrap();
    fs::write(h.root.join("data-stderr"), b"original error\xfe").unwrap();
    let spec = chrono_harness::CommandSpec {
        program: h.tool.to_str().unwrap().into(),
        args: vec![],
        env: Default::default(),
        timeout_seconds: 30,
        output_limit_bytes: 1024,
    };
    let digest = sha256(&fs::read(&h.tool).unwrap());
    for limit in [0, 64 * 1024 * 1024 + 1] {
        assert!(
            chrono_harness::run_process_observed_data(&h.root, &spec, &[], &digest, limit)
                .unwrap_err()
                .contains("transport bound")
        );
    }
    let failed =
        chrono_harness::run_process_observed_data(&h.root, &spec, &[], &digest, 2_097_152).unwrap();
    assert_eq!(failed.exit_code, 17);
    assert!(failed.failure.is_none());
    assert_eq!(failed.stdout_bytes, b"original partial\xff");
    assert_eq!(failed.stderr_bytes, b"original error\xfe");
    fs::write(h.root.join("data-stderr"), vec![b'e'; 2048]).unwrap();
    let bounded =
        chrono_harness::run_process_observed_data(&h.root, &spec, &[], &digest, 2_097_152).unwrap();
    assert_eq!(
        bounded.failure.as_deref(),
        Some("process output limit exceeded")
    );
    assert_eq!(bounded.stderr_bytes, vec![b'e'; 1024]);
    assert_eq!(bounded.stderr_sha256, sha256(&vec![b'e'; 1024]));
    fs::write(h.root.join("data-stderr"), b"").unwrap();
    fs::write(h.root.join("data-stdout"), vec![b'x'; 1025]).unwrap();
    let bounded =
        chrono_harness::run_process_observed_data(&h.root, &spec, &[], &digest, 1024).unwrap();
    assert_eq!(
        bounded.failure.as_deref(),
        Some("process output limit exceeded")
    );
    assert_eq!(bounded.stdout_bytes, vec![b'x'; 1024]);
    assert!(
        chrono_harness::run_process_observed_data(&h.root, &spec, &[], &"0".repeat(64), 2048)
            .unwrap_err()
            .contains("digest mismatch")
    );
}

#[test]
fn immutable_input_verifies_complete_sha1_and_sha256_blobs_including_empty_data() {
    for (original, sha1) in [
        (&b"hello\n"[..], "ce013625030ba8dba906f756967f9e9ca394464a"),
        (&b""[..], "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"),
    ] {
        let framed = [format!("blob {}\0", original.len()).as_bytes(), original].concat();
        for object in [sha1.to_string(), sha256(&framed)] {
            let commit = "a".repeat(object.len());
            let metadata = format!("{object} blob {}\n", original.len()).into_bytes();
            // A successful result requires both exact length and the Git object
            // hash, rather than just the process stream's SHA-256 receipt.
            let variants = [
                original.to_vec(),
                original.iter().copied().chain([b'x']).collect(),
                original
                    .get(..original.len().saturating_sub(1))
                    .unwrap()
                    .to_vec(),
                vec![b'y'; original.len()],
            ];
            for bytes in variants {
                let mut calls = 0;
                let result = chrono_harness::facts::acquire_immutable_input(
                    &commit,
                    "inputs/literal λ.json",
                    1024,
                    |args, input, bound| {
                        calls += 1;
                        if bound.is_none() {
                            assert_eq!(args, ["cat-file", "--batch-check", "-z"]);
                            assert_eq!(
                                input,
                                format!("{commit}:inputs/literal λ.json\0").as_bytes()
                            );
                            Ok(metadata.clone())
                        } else {
                            assert_eq!(bound, Some(original.len().max(1)));
                            assert_eq!(args, ["cat-file", "blob", object.as_str()]);
                            assert!(input.is_empty());
                            Ok(bytes.clone())
                        }
                    },
                );
                assert_eq!(calls, 2);
                if bytes == original {
                    assert_eq!(result.unwrap(), original);
                } else {
                    assert!(result.unwrap_err().contains("differ from object metadata"));
                }
            }
        }
    }
}

#[test]
fn immutable_input_rejects_invalid_metadata_and_preserves_acquisition_errors() {
    let commit = "a".repeat(40);
    let object = "ce013625030ba8dba906f756967f9e9ca394464a";
    for metadata in [
        vec![0xff],
        format!("{object} blob 6").into_bytes(),
        format!("{object} tree 6\n").into_bytes(),
        format!("{object} blob 06\n").into_bytes(),
        format!("{object} blob -1\n").into_bytes(),
        format!("{object} blob 999999999999999999999999\n").into_bytes(),
        format!("{object} blob 6\n{object} blob 6\n").into_bytes(),
        format!("{} blob 6\n", "f".repeat(64)).into_bytes(),
        format!("{} blob 6\n", "z".repeat(40)).into_bytes(),
        format!("{commit}:absent missing\n").into_bytes(),
    ] {
        let mut calls = 0;
        assert!(
            chrono_harness::facts::acquire_immutable_input(
                &commit,
                "input",
                1024,
                |_, _, bound| {
                    calls += 1;
                    assert_eq!(
                        bound, None,
                        "invalid metadata must not launch a content read"
                    );
                    Ok(metadata.clone())
                },
            )
            .is_err()
        );
        assert_eq!(calls, 1);
    }
    for fail_data in [false, true] {
        let mut calls = 0;
        let original_error = "original process failure: exit 17, binary stderr retained";
        let error = chrono_harness::facts::acquire_immutable_input(
            &commit,
            "input",
            1024,
            |_, _, bound| {
                calls += 1;
                if fail_data && bound.is_none() {
                    Ok(format!("{object} blob 6\n").into_bytes())
                } else {
                    Err(original_error.into())
                }
            },
        )
        .unwrap_err();
        assert_eq!(error, original_error);
        assert_eq!(calls, if fail_data { 2 } else { 1 });
    }
}
