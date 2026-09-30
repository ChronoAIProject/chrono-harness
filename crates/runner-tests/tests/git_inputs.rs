use chrono_harness::{facts::Reader, sha256};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

struct Host {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    config: Value,
}
impl Host {
    fn new(body: &str) -> Self {
        let temporary = tempfile::Builder::new()
            .prefix("Git inputs λ ")
            .tempdir()
            .unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
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
        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\n{}\nexec {} \"$@\"\n",
            quote(root.join("trace").to_str().unwrap()),
            body,
            quote(git.to_str().unwrap())
        );
        fs::write(root.join("git-wrapper"), &script).unwrap();
        fs::set_permissions(root.join("git-wrapper"), fs::Permissions::from_mode(0o755)).unwrap();
        let config = json!({
            "schema_version":3,
            "facts_git":{"tool":"git","input":"git-binary","guard":{
                "schema":"chrono-git-inputs/v1","inputs":["git-config","no-extra-config"]}},
            "tools":[{"id":"git","program":"git-wrapper","resolution":"PATH-once",
                "version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}],
            "environment":{"inherit":[],"values":{"PATH":root,"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"},
                "inputs":[{"id":"git-binary","location":"git-wrapper","presence":"present","sha256":sha256(script.as_bytes())},
                    {"id":"git-config","location":".git/config","presence":"present","sha256":sha256(&fs::read(root.join(".git/config")).unwrap())},
                    {"id":"no-extra-config","location":".git/config.worktree","presence":"absent"}]},
            "protocol":{"timeout_seconds":30,"stdout_limit_bytes":1048576}
        });
        Self {
            _temporary: temporary,
            root,
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

#[test]
fn explicit_git_configuration_is_checked_before_each_real_read() {
    let h = Host::new("");
    let reader = h.open().unwrap_or_else(|e| panic!("{}", e.message));
    let bytes = reader
        .git(
            &h.root,
            &["config", "--local", "--get", "core.repositoryformatversion"],
        )
        .unwrap();
    assert_eq!(bytes, b"0\n");
    let observed = reader.observation();
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
        reader.observation()["processes"].as_array().unwrap().len(),
        count
    );
}

#[test]
fn artifact_inventory_preserves_noncanonical_paths_and_checks_bound_inputs() {
    let h = Host::new("");
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
    let count = reader.observation()["processes"].as_array().unwrap().len();
    fs::write(h.root.join(".git/config.worktree"), "changed").unwrap();
    let error = reader
        .untracked_excluding(&h.root, &["cache[1]/allowed/"])
        .unwrap_err();
    assert!(error.contains("no-extra-config"), "{error}");
    assert_eq!(
        reader.observation()["processes"].as_array().unwrap().len(),
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
        let mut h = Host::new("");
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
    let h = Host::new(
        "case \"$*\" in *rev-parse*) printf 'changed' > .git/config.worktree; printf 'original output'; printf 'original error' >&2; exit 17;; esac",
    );
    let reader = h.open().unwrap_or_else(|e| panic!("{}", e.message));
    let error = reader
        .git(&h.root, &["rev-parse", "--show-toplevel"])
        .unwrap_err();
    assert!(error.contains("no-extra-config"), "{error}");
    let observed = reader.observation();
    let process = observed["processes"].as_array().unwrap().last().unwrap();
    assert_eq!(process["exit_code"], 17);
    assert_eq!(process["stdout"], "original output");
    assert_eq!(process["stderr"], "original error");
    assert_eq!(process["stdout_sha256"], sha256(b"original output"));
}

#[test]
fn legacy_binding_has_no_guard_and_current_observations_cannot_omit_it() {
    let mut h = Host::new("");
    h.config["facts_git"]
        .as_object_mut()
        .unwrap()
        .remove("guard");
    let reader = h.open().unwrap_or_else(|e| panic!("{}", e.message));
    assert!(reader.observation().get("inputs").is_none());
    fs::write(h.root.join(".git/config.worktree"), "legacy semantics").unwrap();
    assert!(
        reader
            .git(&h.root, &["rev-parse", "--show-toplevel"])
            .is_ok()
    );

    let h = Host::new("");
    let reader = h.open().unwrap_or_else(|e| panic!("{}", e.message));
    let mut observation = reader.observation();
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
    let h = Host::new("");
    let mut selector = selection();
    // Another registered platform need not exist on this machine.
    selector["platforms"]["other-system"] = json!(".chrono-harness/elsewhere.json");
    select(&h, &selector);
    let candidate = commit(&h);
    let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
    assert!(reader.is_bound());
    reader.verify_config(&h.root, &candidate).unwrap();
    let observed = reader.observation();
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
        let mut h = Host::new("");
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
        let h = Host::new("");
        select(&h, &selection());
        let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
        assert!(reader.is_bound());
        let before = h.root.join(path);
        let bytes = fs::read(&before).unwrap();
        let count = reader.observation()["processes"].as_array().unwrap().len();
        fs::write(&before, b"{}").unwrap();
        assert!(
            reader
                .git(&h.root, &["rev-parse", "--show-toplevel"])
                .is_err()
        );
        assert_eq!(
            reader.observation()["processes"].as_array().unwrap().len(),
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
    let h = Host::new(
        "case \"$*\" in *rev-parse*) printf changed > .chrono-harness/git-platforms.json; printf original; exit 17;; esac",
    );
    select(&h, &selection());
    let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
    assert!(reader.is_bound());
    let error = reader
        .git(&h.root, &["rev-parse", "--show-toplevel"])
        .unwrap_err();
    assert!(error.contains("selector changed"), "{error}");
    let observation = reader.observation();
    let process = observation["processes"].as_array().unwrap().last().unwrap();
    assert_eq!(process["exit_code"], 17);
    assert_eq!(process["stdout_sha256"], sha256(b"original"));
}

#[test]
fn platform_selection_must_match_both_fixed_candidate_blobs() {
    for path in [SELECTOR, CONFIG] {
        let mut h = Host::new("");
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
