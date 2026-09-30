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
