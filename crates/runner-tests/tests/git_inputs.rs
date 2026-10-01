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

fn processes(reader: &Reader) -> usize {
    reader.observation()["processes"].as_array().unwrap().len()
}

#[test]
fn fixed_registry_reads_reuse_real_acquisitions_without_replaying_observations() {
    let mut h = Host::new("");
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
    let original = reader.observation();
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
        reader.observation()
    );
    assert_eq!(
        processes(&reader),
        7,
        "version, existing commit/tree observations, selector/config and two registry batch acquisitions"
    );
    assert_eq!(
        reader.observation(),
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
    let h = Host::new("");
    h.save();
    let path = "payload café\t.bin";
    let bytes = b"\0\xff\nbody\0";
    fs::write(h.root.join(path), bytes).unwrap();
    fs::write(h.root.join("other.bin"), b"other").unwrap();
    let old = commit(&h);
    let reader = h.open().unwrap();
    reader.verify_oid(&h.root, &old).unwrap();
    assert_eq!(reader.blob(&h.root, &old, path).unwrap(), bytes);
    let original = reader.observation();
    assert_eq!(reader.blob(&h.root, &old, path).unwrap(), bytes);
    assert_eq!(reader.observation(), original);
    assert_eq!(reader.blob(&h.root, &old, "other.bin").unwrap(), b"other");
    fs::write(h.root.join(path), b"new").unwrap();
    let new = commit(&h);
    reader.verify_oid(&h.root, &new).unwrap();
    assert_eq!(reader.blob(&h.root, &new, path).unwrap(), b"new");
    assert_eq!(reader.blob(&h.root, &old, path).unwrap(), bytes);
    let other = Host::new("");
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
fn cached_immutable_bytes_still_reject_every_bound_guard_drift_before_effects() {
    for case in [
        "selector",
        "config",
        "tool",
        "guard-present",
        "guard-absent",
        "symlink",
    ] {
        let h = Host::new("");
        select(&h, &selection());
        fs::write(h.root.join("payload"), b"original").unwrap();
        let oid = commit(&h);
        let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        assert_eq!(reader.blob(&h.root, &oid, "payload").unwrap(), b"original");
        let trace = fs::read(h.root.join("trace")).unwrap();
        let count = processes(&reader);
        match case {
            "selector" => fs::write(h.root.join(SELECTOR), b"{}").unwrap(),
            "config" => fs::write(h.root.join(CONFIG), b"{}").unwrap(),
            "tool" => fs::write(h.root.join("git-wrapper"), b"changed").unwrap(),
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
        assert_eq!(processes(&reader), count, "{case}");
        assert_eq!(fs::read(h.root.join("trace")).unwrap(), trace, "{case}");
    }
}

#[test]
fn failed_or_postprocess_drifted_blob_reads_are_never_reused() {
    for drift in [false, true] {
        let body = if drift {
            "case \"$*\" in *' show '*) if [ ! -e retry-ready ]; then printf changed > .git/config.worktree; fi;; esac"
        } else {
            "case \"$*\" in *' show '*) if [ ! -e retry-ready ]; then printf 'original output'; printf 'original error' >&2; exit 17; fi;; esac"
        };
        let h = Host::new(body);
        h.save();
        fs::write(h.root.join("payload"), b"original\0\xff").unwrap();
        let oid = commit(&h);
        let reader = h.open().unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        assert!(reader.blob(&h.root, &oid, "payload").is_err());
        let observed = reader.observation();
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
        let success = reader.observation();
        assert_eq!(
            reader.blob(&h.root, &oid, "payload").unwrap(),
            b"original\0\xff"
        );
        assert_eq!(reader.observation(), success);
        assert_eq!(success["processes"][failed_index], *failed);
        println!("FACTS_RETRY drift={drift} observation={success}");
    }
}

#[test]
fn mutable_refs_index_checkout_and_untracked_facts_remain_fresh() {
    let h = Host::new("");
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
    let mut h = Host::new("case \"$*\" in *' show '*) printf '%s' \"$BOUND_VALUE\"; exit 0;; esac");
    h.config["environment"]["values"]["BOUND_VALUE"] = json!("first");
    h.save();
    fs::write(h.root.join("payload"), b"committed").unwrap();
    let oid = commit(&h);
    let first = h.open().unwrap();
    first.verify_oid(&h.root, &oid).unwrap();
    assert_eq!(first.blob(&h.root, &oid, "payload").unwrap(), b"first");
    let original = first.observation();
    assert_eq!(first.blob(&h.root, &oid, "payload").unwrap(), b"first");
    assert_eq!(first.observation(), original);
    h.config["environment"]["values"]["BOUND_VALUE"] = json!("second");
    let second = h.open().unwrap();
    assert!(
        first
            .blob(&h.root, &oid, "payload")
            .unwrap_err()
            .contains("configuration changed")
    );
    assert_eq!(first.observation(), original);
    second.verify_oid(&h.root, &oid).unwrap();
    assert_eq!(second.blob(&h.root, &oid, "payload").unwrap(), b"second");
    let observed = second.observation();
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
    let h = Host::new("");
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
    println!("FACTS_HEX_REF observation={}", reader.observation());
}

#[test]
fn reads_before_a_successful_identity_observation_stay_fresh_without_extra_probes() {
    let h = Host::new("");
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
    let original = reader.observation();
    assert_eq!(reader.blob(&h.root, &oid, "payload").unwrap(), b"original");
    assert_eq!(reader.observation(), original);
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
        let h = Host::new(
            "case \"$*\" in *rev-parse*) if [ -e identity-fail ]; then printf 'original error' >&2; exit 17; fi; case \"$*\" in *'^{tree}'*) if [ -e tree-fail ]; then exit 17; fi; if [ -e tree-response ]; then /bin/cat tree-response; exit 0; fi;; esac; if [ -e object-response ]; then /bin/cat object-response; exit 0; fi;; esac",
        );
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
        let observed = reader.observation();
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
            reader.observation()
        );
    }
}

fn registry_host(
    body: &str,
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
    let (h, originals, oid) = registry_host("", 40);
    let reader = h.open().unwrap();
    reader.verify_oid(&h.root, &oid).unwrap();
    let start = std::time::Instant::now();
    let snapshot = reader.registry_snapshot(&h.root, &oid, CONFIG).unwrap();
    let elapsed = start.elapsed().as_secs_f64();
    for (path, bytes) in &originals {
        assert_eq!(&snapshot.bytes[path], bytes);
    }
    let observed = reader.observation();
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
        reader.observation(),
        observed,
        "reuse retains the original receipt"
    );
}

#[test]
fn registry_batch_partitions_within_original_limit_and_preserves_exact_limit_blobs() {
    for exact_limit in [false, true] {
        let (mut h, mut originals, _) = registry_host("", 700);
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
        for process in reader.observation()["processes"].as_array().unwrap() {
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
            let body = format!(
                "case \"$*\" in *' cat-file {phase}') if [ -e response ]; then /bin/cat response; exit 0; fi;; esac"
            );
            let (h, _, oid) = registry_host(&body, 20);
            let reader = h.open().unwrap();
            reader.verify_oid(&h.root, &oid).unwrap();
            fs::write(h.root.join("response"), response).unwrap();
            for _ in 0..2 {
                let before = processes(&reader);
                assert!(
                    reader.registry_snapshot(&h.root, &oid, CONFIG).is_err(),
                    "{phase}: {response:?}"
                );
                assert!(processes(&reader) > before);
                let observed = reader.observation();
                let last = observed["processes"].as_array().unwrap().last().unwrap();
                assert_eq!(last["stdout_bytes"], json!(response));
                assert_eq!(last["stdout_sha256"], sha256(response));
                assert_eq!(last["exit_code"], 0);
            }
            fs::remove_file(h.root.join("response")).unwrap();
            assert!(reader.registry_snapshot(&h.root, &oid, CONFIG).is_ok());
        }
    }
    for drift in [false, true] {
        let body = if drift {
            "case \"$*\" in *' cat-file --batch') if [ ! -e retry-ready ]; then printf changed > .git/config.worktree; fi;; esac"
        } else {
            "case \"$*\" in *' cat-file --batch') if [ ! -e retry-ready ]; then printf 'original partial'; printf 'original error' >&2; exit 17; fi;; esac"
        };
        let (h, _, oid) = registry_host(body, 20);
        let reader = h.open().unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        assert!(reader.registry_snapshot(&h.root, &oid, CONFIG).is_err());
        let observed = reader.observation();
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
        assert_eq!(reader.observation()["processes"][failed_index], failed);
    }
}

#[test]
fn registry_batch_rejects_corrupted_lengths_identities_payloads_and_framing() {
    let body = "case \"$*\" in *' cat-file --batch') if [ -e response ]; then /bin/cat response; exit 0; fi;; esac";
    let (h, _, oid) = registry_host(body, 30);
    let first = h.open().unwrap();
    first.verify_oid(&h.root, &oid).unwrap();
    first.registry_snapshot(&h.root, &oid, CONFIG).unwrap();
    let observed = first.observation();
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
            let observed = reader.observation();
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
            "case \"$*\" in *' cat-file --batch') /bin/cat oversized; exit 0;; esac"
        } else {
            "case \"$*\" in *' cat-file --batch') /bin/sleep 60;; esac"
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
        let observed = reader.observation();
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
        let (h, originals, _) = registry_host("", 20);
        select(&h, &selection());
        let oid = commit(&h);
        let reader = Reader::for_config(&h.root, SELECTOR).unwrap();
        reader.verify_oid(&h.root, &oid).unwrap();
        reader.registry_snapshot(&h.root, &oid, SELECTOR).unwrap();
        let original = reader.observation();
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
            "tool" => fs::write(h.root.join("git-wrapper"), b"changed").unwrap(),
            "guard-present" => fs::write(h.root.join(".git/config"), b"changed").unwrap(),
            "guard-absent" => fs::write(h.root.join(".git/config.worktree"), b"changed").unwrap(),
            "root" => {}
            _ => unreachable!(),
        }
        assert!(reader.blob(root, &oid, path).is_err(), "{case}");
        assert_eq!(
            reader.observation(),
            original,
            "{case}: reject before process effects"
        );
    }
}

#[test]
fn unverified_registry_endpoints_and_mutable_refs_keep_original_reads() {
    let (h, originals, old) = registry_host("", 20);
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
