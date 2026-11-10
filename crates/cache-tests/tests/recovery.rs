use super::*;

fn bind_installed_planner(plan: &mut Value) {
    let binary = Path::new(env!("CARGO_BIN_EXE_chrono-cache-test-cli"));
    plan["planner"] = json!({
        "executable": fs::canonicalize(&binary).unwrap(),
        "sha256": format!("{:x}", Sha256::digest(fs::read(&binary).unwrap())),
        "bytes": fs::metadata(&binary).unwrap().len(),
        "version": env!("CARGO_PKG_VERSION"),
    });
}

fn recovery_host() -> (Host, Value, Value) {
    let (root, mut config) = fixture();
    fs::create_dir(root.path().join(".git")).unwrap();
    config["require_primary_checkout"] = json!(true);
    let before = plan(root.path(), &config, "check.one").unwrap();
    config["recover_failed_restores"] = json!(true);
    let mut prepared = plan(root.path(), &config, "check.one").unwrap();
    bind_installed_planner(&mut prepared);
    assert_eq!(before["caches"], prepared["caches"]);
    fs::create_dir_all(root.path().join(".chrono-harness/state/restore")).unwrap();
    fs::write(
        root.path().join(".chrono-harness/cache.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    fs::write(
        root.path().join(".chrono-harness/state/restore/plan.json"),
        serde_json::to_vec(&prepared).unwrap(),
    )
    .unwrap();
    fs::create_dir_all(root.path().join("out/project")).unwrap();
    fs::write(root.path().join("out/project/partial"), b"partial archive").unwrap();
    fs::create_dir_all(root.path().join("out/other")).unwrap();
    fs::write(root.path().join("out/other/keep"), b"other consumer").unwrap();
    (root, config, prepared)
}

pub(super) fn recover(root: &Path, steps: &Value) -> std::process::Output {
    Command::new(Path::new(env!("CARGO_BIN_EXE_chrono-cache-test-cli")))
        .args([
            "recover",
            "--host-root",
            root.to_str().unwrap(),
            "--config",
            ".chrono-harness/cache.json",
            "--plan",
            ".chrono-harness/state/restore/plan.json",
            "--steps-env",
            "CACHE_RECOVERY_STEPS",
        ])
        .env(
            "CACHE_RECOVERY_STEPS",
            serde_json::to_string(steps).unwrap(),
        )
        .output()
        .unwrap()
}

fn adopt_source_refresh(root: &Path, config: &mut Value) -> Value {
    config["caches"]["project"]["refresh_sources"] = json!(true);
    let mut prepared = plan(root, config, "check.one").unwrap();
    bind_installed_planner(&mut prepared);
    for (path, value) in [
        (".chrono-harness/cache.json", config.clone()),
        (".chrono-harness/state/restore/plan.json", prepared.clone()),
    ] {
        fs::write(root.join(path), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    prepared
}

#[test]
fn source_refresh_is_local_to_compatible_hits_and_replay_keeps_later_timestamps() {
    use std::time::{Duration, SystemTime};
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
    let future = SystemTime::now() + Duration::from_secs(7200);
    for reason in ["compatible", "exact", "skipped", "failure"] {
        let (root, mut config, original) = recovery_host();
        let prepared = adopt_source_refresh(root.path(), &mut config);
        assert_ne!(
            original["caches"]["project"]["restore_keys"],
            prepared["caches"]["project"]["restore_keys"]
        );
        let source = root.path().join("source");
        fs::File::open(&source).unwrap().set_modified(old).unwrap();
        fs::File::open(root.path().join("out/project/partial"))
            .unwrap()
            .set_modified(future)
            .unwrap();
        let compiler = root.path().join("compiler");
        let unchanged = fs::metadata(&compiler).unwrap().modified().unwrap();
        let key = if reason == "exact" {
            prepared["caches"]["project"]["key"]
                .as_str()
                .unwrap()
                .to_owned()
        } else {
            format!(
                "{}another-source",
                prepared["caches"]["project"]["restore_keys"][0]
                    .as_str()
                    .unwrap()
            )
        };
        let outcome = if matches!(reason, "compatible" | "exact") {
            "success"
        } else {
            reason
        };
        let steps =
            json!({"cache_0_restore":{"outcome":outcome,"outputs":{"cache-matched-key":key}}});
        let output = recover(root.path(), &steps);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let changed = fs::metadata(&source).unwrap().modified().unwrap();
        if reason == "compatible" {
            assert!(changed > future);
            assert!(root.path().join("out/project/partial").exists());
            assert_eq!(
                result(root.path())["caches"]["project"]["source_freshness"]["status"],
                "refreshed"
            );
            fs::File::open(&source).unwrap().set_modified(old).unwrap();
            assert!(recover(root.path(), &steps).status.success());
            assert_eq!(fs::metadata(&source).unwrap().modified().unwrap(), old);
        } else {
            assert_eq!(changed, old, "{reason}");
        }
        assert_eq!(fs::read(&source).unwrap(), b"source-one");
        assert_eq!(
            fs::metadata(&compiler).unwrap().modified().unwrap(),
            unchanged
        );
        assert_eq!(
            fs::read(root.path().join("out/other/keep")).unwrap(),
            b"other consumer"
        );
    }
}

#[test]
fn registered_source_refresh_rejects_changed_inputs_before_any_mutation() {
    for changed in [false, true] {
        let (root, mut config, _) = recovery_host();
        config["inputs"]["source"] = json!({"kind":"registered-files","registry":".chrono-harness/FILEMAP.json","node":"project:product","edges":["compile"]});
        fs::write(root.path().join(".chrono-harness/FILEMAP.json"), serde_json::to_vec(&json!({"schema_version":2,"project_edges":[],"files":[{"path":"source","edges":[{"kind":"compile","to":"project:product"}]}]})).unwrap()).unwrap();
        let prepared = adopt_source_refresh(root.path(), &mut config);
        if changed {
            fs::write(root.path().join("source"), b"new input after plan").unwrap();
        }
        let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1);
        fs::File::open(root.path().join("source"))
            .unwrap()
            .set_modified(old)
            .unwrap();
        let key = format!(
            "{}older-source",
            prepared["caches"]["project"]["restore_keys"][0]
                .as_str()
                .unwrap()
        );
        let output = recover(
            root.path(),
            &json!({"cache_0_restore":{"outcome":"success","outputs":{"cache-matched-key":key}}}),
        );
        assert_eq!(
            output.status.success(),
            !changed,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if changed {
            assert!(
                result(root.path())["error"]
                    .as_str()
                    .unwrap()
                    .contains("current input differs")
            );
            assert_eq!(
                fs::metadata(root.path().join("source"))
                    .unwrap()
                    .modified()
                    .unwrap(),
                old
            );
        } else {
            assert!(
                fs::metadata(root.path().join("source"))
                    .unwrap()
                    .modified()
                    .unwrap()
                    > old
            );
        }
        assert!(root.path().join("out/project/partial").exists());
    }
}

fn result(root: &Path) -> Value {
    let pointer: Value = serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/state/restore/recovery.json")).unwrap(),
    )
    .unwrap();
    let raw = fs::read(root.join(pointer["path"].as_str().unwrap())).unwrap();
    assert_eq!(pointer["sha256"], format!("{:x}", Sha256::digest(&raw)));
    serde_json::from_slice(&raw).unwrap()
}

#[test]
fn failed_restore_clears_only_its_registered_artifacts_and_does_not_repeat_after_build() {
    let (root, _, _) = recovery_host();
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        root.path().join("source"),
        root.path().join("out/project/source-alias"),
    )
    .unwrap();
    let steps =
        json!({"cache_0_restore":{"outcome":"failure","conclusion":"success","outputs":{}}});
    let recovered = recover(root.path(), &steps);
    assert!(
        recovered.status.success(),
        "{}",
        String::from_utf8_lossy(&recovered.stderr)
    );
    assert!(!root.path().join("out/project").exists());
    assert_eq!(
        fs::read(root.path().join("out/other/keep")).unwrap(),
        b"other consumer"
    );
    assert_eq!(fs::read(root.path().join("source")).unwrap(), b"source-one");
    let report = result(root.path());
    assert_eq!(report["status"], "recovered");
    assert_eq!(
        report["caches"]["project"]["restore"],
        steps["cache_0_restore"]
    );
    assert_eq!(
        report["caches"]["project"]["artifacts"][0]["status"],
        "removed"
    );
    let original = fs::read(
        root.path()
            .join(report["native_steps"]["path"].as_str().unwrap()),
    )
    .unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&original).unwrap(), steps);
    fs::create_dir_all(root.path().join("out/project")).unwrap();
    fs::write(root.path().join("out/project/new-build"), b"new build").unwrap();
    let replay = recover(root.path(), &steps);
    assert!(replay.status.success());
    assert_eq!(result(root.path()), report);
    assert_eq!(
        fs::read(root.path().join("out/project/new-build")).unwrap(),
        b"new build"
    );
}

#[test]
fn successful_or_skipped_restore_keeps_outputs_and_missing_observations_fail() {
    for outcome in [
        Some("success"),
        Some("skipped"),
        Some("cancelled"),
        Some("unknown"),
        None,
    ] {
        let (root, _, _) = recovery_host();
        let steps = outcome
            .map(|s| json!({"cache_0_restore":{"outcome":s}}))
            .unwrap_or(json!({}));
        let output = recover(root.path(), &steps);
        let allowed = matches!(outcome, Some("success" | "skipped"));
        assert_eq!(
            output.status.success(),
            allowed,
            "{outcome:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(root.path().join("out/project/partial")).unwrap(),
            b"partial archive"
        );
        assert_eq!(
            result(root.path())["status"],
            if allowed { "not-required" } else { "failed" }
        );
    }
}

fn adopt_unconfirmed_recovery(root: &Path, config: &mut Value, original: &Value) {
    config["recover_unconfirmed_restores"] = json!(true);
    let mut prepared = plan(root, config, "check.one").unwrap();
    bind_installed_planner(&mut prepared);
    assert_eq!(prepared["caches"], original["caches"]);
    assert_eq!(prepared["recovery"]["unconfirmed"], true);
    for (path, value) in [
        (".chrono-harness/cache.json", config.clone()),
        (".chrono-harness/state/restore/plan.json", prepared),
    ] {
        fs::write(root.join(path), serde_json::to_vec(&value).unwrap()).unwrap();
    }
}

#[test]
fn unconfirmed_success_discards_partial_outputs_and_retains_original_uncertainty() {
    for matched in [None, Some(""), Some("unregistered-key")] {
        let (root, mut config, prepared) = recovery_host();
        adopt_unconfirmed_recovery(root.path(), &mut config, &prepared);
        let mut restore = json!({"outcome":"success","conclusion":"success","outputs":{}});
        if let Some(key) = matched {
            restore["outputs"]["cache-matched-key"] = json!(key);
        }
        let steps = json!({"cache_0_restore":restore});
        let output = recover(root.path(), &steps);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!root.path().join("out/project").exists(), "{matched:?}");
        assert_eq!(
            fs::read(root.path().join("out/other/keep")).unwrap(),
            b"other consumer"
        );
        assert_eq!(fs::read(root.path().join("source")).unwrap(), b"source-one");
        let report = result(root.path());
        assert_eq!(report["status"], "recovered");
        assert_eq!(report["caches"]["project"]["restore"], restore);
        let reason = if matched == Some("unregistered-key") {
            "incompatible"
        } else {
            "miss-or-unavailable"
        };
        assert_eq!(report["caches"]["project"]["reason"], reason);
        fs::create_dir_all(root.path().join("out/project")).unwrap();
        fs::write(root.path().join("out/project/rebuilt"), b"rebuilt").unwrap();
        assert!(recover(root.path(), &steps).status.success());
        assert_eq!(
            fs::read(root.path().join("out/project/rebuilt")).unwrap(),
            b"rebuilt"
        );
        let original = fs::read(
            root.path()
                .join(report["native_steps"]["path"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&original).unwrap(),
            json!({"cache_0_restore":restore})
        );
    }
}

#[test]
fn unconfirmed_recovery_keeps_matching_hits_and_rejects_missing_or_malformed_observations() {
    for kind in [
        "exact",
        "compatible",
        "skipped",
        "missing",
        "cancelled",
        "malformed",
    ] {
        let (root, mut config, prepared) = recovery_host();
        adopt_unconfirmed_recovery(root.path(), &mut config, &prepared);
        let cache = &prepared["caches"]["project"];
        let mut restore = json!({"outcome":"success","outputs":{}});
        match kind {
            "exact" => restore["outputs"]["cache-matched-key"] = cache["key"].clone(),
            "compatible" => {
                restore["outputs"]["cache-matched-key"] = json!(format!(
                    "{}old-source",
                    cache["restore_keys"][0].as_str().unwrap()
                ))
            }
            "skipped" => restore["outcome"] = json!("skipped"),
            "missing" => restore = Value::Null,
            "cancelled" => restore["outcome"] = json!("cancelled"),
            "malformed" => restore["outputs"]["cache-matched-key"] = json!(123),
            _ => unreachable!(),
        }
        let output = recover(root.path(), &json!({"cache_0_restore":restore}));
        let allowed = matches!(kind, "exact" | "compatible" | "skipped");
        assert_eq!(
            output.status.success(),
            allowed,
            "{kind}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(root.path().join("out/project/partial")).unwrap(),
            b"partial archive"
        );
        assert_eq!(
            result(root.path())["status"],
            if allowed { "not-required" } else { "failed" }
        );
    }
}

#[test]
fn unconfirmed_recovery_requires_failed_restore_adoption_and_keeps_cold_miss_uncertain() {
    let (root, mut config, prepared) = recovery_host();
    config["recover_unconfirmed_restores"] = json!(true);
    config["recover_failed_restores"] = json!(false);
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("adopted failed-restore recovery")
    );
    config["recover_failed_restores"] = json!(true);
    config["require_primary_checkout"] = json!(false);
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("primary-checkout")
    );
    config["require_primary_checkout"] = json!(true);
    adopt_unconfirmed_recovery(root.path(), &mut config, &prepared);
    fs::remove_dir_all(root.path().join("out/project")).unwrap();
    let steps = json!({"cache_0_restore":{"outcome":"success","outputs":{}}});
    let output = recover(root.path(), &steps);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "a cold miss without leftover outputs is not a diagnosed restore error"
    );
    let report = result(root.path());
    assert_eq!(report["caches"]["project"]["reason"], "miss-or-unavailable");
    assert_eq!(
        report["caches"]["project"]["artifacts"][0]["status"],
        "absent"
    );
    assert!(!root.path().join("out/project").exists());
}

#[test]
fn changed_registration_and_symlinked_outputs_preserve_original_failure_without_discard() {
    for alter in ["registration", "output", "unfinished"] {
        let (root, mut config, _) = recovery_host();
        match alter {
            "registration" => {
                config["artifacts"]["target"]["path"] = json!("source");
                fs::write(
                    root.path().join(".chrono-harness/cache.json"),
                    serde_json::to_vec(&config).unwrap(),
                )
                .unwrap();
            }
            "output" => {
                fs::rename(
                    root.path().join("out/project"),
                    root.path().join("out/retained"),
                )
                .unwrap();
                #[cfg(unix)]
                std::os::unix::fs::symlink("retained", root.path().join("out/project")).unwrap();
            }
            "unfinished" => fs::write(
                root.path()
                    .join(".chrono-harness/state/restore/recovery-intent.json"),
                b"original interrupted intent",
            )
            .unwrap(),
            _ => unreachable!(),
        }
        let output = recover(
            root.path(),
            &json!({"cache_0_restore":{"outcome":"failure"}}),
        );
        assert!(!output.status.success(), "{alter}");
        assert_eq!(fs::read(root.path().join("source")).unwrap(), b"source-one");
        assert_eq!(
            fs::read(root.path().join("out/project/partial")).unwrap(),
            b"partial archive"
        );
        if alter != "unfinished" {
            assert_eq!(result(root.path())["status"], "failed");
        }
    }
}

#[cfg(unix)]
#[test]
fn partial_cleanup_failure_retains_each_result_and_refuses_a_second_deletion() {
    let (root, mut config, _) = recovery_host();
    config["artifacts"]["zz-socket"] = json!({
        "owner":"project","path":"out/socket"
    });
    config["caches"]["project"]["artifacts"] = json!(["target", "zz-socket"]);
    register_artifacts(root.path(), &config);
    let mut prepared = plan(root.path(), &config, "check.one").unwrap();
    bind_installed_planner(&mut prepared);
    fs::write(
        root.path().join(".chrono-harness/cache.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    fs::write(
        root.path().join(".chrono-harness/state/restore/plan.json"),
        serde_json::to_vec(&prepared).unwrap(),
    )
    .unwrap();
    // A relative socket name keeps the fixture within SUN_LEN even when the
    // registered temporary root is long. Only the child's cwd changes.
    let socket = Command::new(env!("CARGO_BIN_EXE_chrono-cache-test-probe"))
        .current_dir(root.path())
        .args(["socket", "out/socket"])
        .output()
        .unwrap();
    assert!(
        socket.status.success(),
        "{}",
        String::from_utf8_lossy(&socket.stderr)
    );
    let steps = json!({"cache_0_restore":{"outcome":"failure"}});
    let failed = recover(root.path(), &steps);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("neither a regular file"));
    assert!(!root.path().join("out/project").exists());
    assert!(root.path().join("out/socket").exists());
    let original = result(root.path());
    assert_eq!(original["status"], "failed");
    let artifacts = &original["caches"]["project"]["artifacts"];
    assert_eq!(artifacts[0]["status"], "removed");
    assert_eq!(artifacts[1]["status"], "failed");
    fs::create_dir(root.path().join("out/project")).unwrap();
    fs::write(root.path().join("out/project/new-build"), b"new build").unwrap();
    let repeated = recover(root.path(), &steps);
    assert!(!repeated.status.success());
    assert!(String::from_utf8_lossy(&repeated.stderr).contains("previous attempt failed"));
    assert_eq!(result(root.path()), original);
    assert_eq!(
        fs::read(root.path().join("out/project/new-build")).unwrap(),
        b"new build"
    );
}

fn verified_host() -> (Host, Value, Value) {
    let (root, mut config, _) = recovery_host();
    config["recover_unconfirmed_restores"] = json!(true);
    config["caches"]["project"]["verify_contents"] = json!(true);
    let mut prepared = plan(root.path(), &config, "check.one").unwrap();
    bind_installed_planner(&mut prepared);
    fs::write(
        root.path().join(".chrono-harness/cache.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    fs::write(
        root.path().join(".chrono-harness/state/restore/plan.json"),
        serde_json::to_vec(&prepared).unwrap(),
    )
    .unwrap();
    (root, config, prepared)
}

fn seal(root: &Path, steps: &Value) -> std::process::Output {
    Command::new(Path::new(env!("CARGO_BIN_EXE_chrono-cache-test-cli")))
        .args([
            "seal",
            "--host-root",
            root.to_str().unwrap(),
            "--config",
            ".chrono-harness/cache.json",
            "--plan",
            ".chrono-harness/state/restore/plan.json",
            "--cache",
            "project",
            "--steps-env",
            "CACHE_PRODUCER_STEPS",
            "--work",
            "work",
        ])
        .env(
            "CACHE_PRODUCER_STEPS",
            serde_json::to_string(steps).unwrap(),
        )
        .output()
        .unwrap()
}

#[test]
fn verified_hit_requires_producer_seal_and_retains_mismatched_content_before_recovery() {
    for mutation in [
        "none",
        "changed",
        "missing",
        "extra",
        "mode",
        "wrong-producer",
        "wrong-key",
        "malformed-manifest",
        "missing-manifest",
    ] {
        let (root, _, prepared) = verified_host();
        let sealed = seal(root.path(), &json!({"work":{"outcome":"success"}}));
        assert!(
            sealed.status.success(),
            "{}",
            String::from_utf8_lossy(&sealed.stderr)
        );
        let marker = root
            .path()
            .join("out/project/.chrono-cache-content-v1.json");
        match mutation {
            "changed" => {
                fs::write(root.path().join("out/project/partial"), b"altered content").unwrap()
            }
            "missing" => fs::remove_file(root.path().join("out/project/partial")).unwrap(),
            "extra" => fs::write(root.path().join("out/project/extra"), b"unbound output").unwrap(),
            "mode" => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(
                        root.path().join("out/project/partial"),
                        fs::Permissions::from_mode(0o755),
                    )
                    .unwrap();
                }
            }
            "wrong-producer" => {
                let mut value: Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
                value["producer"] = json!("other.operation");
                fs::write(&marker, serde_json::to_vec(&value).unwrap()).unwrap();
            }
            "wrong-key" => {
                let mut value: Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
                value["key"] = json!("incompatible-source-key");
                fs::write(&marker, serde_json::to_vec(&value).unwrap()).unwrap();
            }
            "malformed-manifest" => fs::write(&marker, b"partial manifest").unwrap(),
            "missing-manifest" => fs::remove_file(&marker).unwrap(),
            _ => (),
        }
        let steps = json!({"cache_0_restore":{"outcome":"success","outputs":{"cache-matched-key":prepared["caches"]["project"]["key"]}}});
        let recovered = recover(root.path(), &steps);
        assert!(
            recovered.status.success(),
            "{mutation}: {}",
            String::from_utf8_lossy(&recovered.stderr)
        );
        let report = result(root.path());
        assert_eq!(
            report["caches"]["project"]["restore"],
            steps["cache_0_restore"]
        );
        assert_eq!(
            report["caches"]["project"]["verification"]["status"],
            if mutation == "none" {
                "matched"
            } else {
                "unusable"
            }
        );
        assert_eq!(root.path().join("out/project").exists(), mutation == "none");
        if mutation != "none" {
            let original = &report["caches"]["project"]["verification"]["original"];
            assert!(
                root.path()
                    .join(original["path"].as_str().unwrap())
                    .is_file()
            );
            // Completed recovery must preserve the output of the subsequent registered build.
            fs::create_dir_all(root.path().join("out/project")).unwrap();
            fs::write(root.path().join("out/project/rebuilt"), b"candidate").unwrap();
            assert!(recover(root.path(), &steps).status.success());
            assert!(root.path().join("out/project/rebuilt").is_file());
        }
    }
}

#[test]
fn producer_seal_refuses_failure_and_compatible_restore_checks_original_source_key() {
    let (root, config, prepared) = verified_host();
    let rejected = seal(
        root.path(),
        &json!({"work":{"outcome":"failure","conclusion":"success"}}),
    );
    assert!(!rejected.status.success());
    assert!(
        !root
            .path()
            .join("out/project/.chrono-cache-content-v1.json")
            .exists()
    );
    assert!(
        seal(root.path(), &json!({"work":{"outcome":"success"}}))
            .status
            .success()
    );
    fs::write(root.path().join("source"), b"candidate-source-two").unwrap();
    let mut candidate = plan(root.path(), &config, "check.one").unwrap();
    bind_installed_planner(&mut candidate);
    assert_ne!(
        prepared["caches"]["project"]["key"],
        candidate["caches"]["project"]["key"]
    );
    assert_eq!(
        prepared["caches"]["project"]["restore_keys"],
        candidate["caches"]["project"]["restore_keys"]
    );
    fs::write(
        root.path().join(".chrono-harness/state/restore/plan.json"),
        serde_json::to_vec(&candidate).unwrap(),
    )
    .unwrap();
    let steps = json!({"cache_0_restore":{"outcome":"success","outputs":{"cache-matched-key":prepared["caches"]["project"]["key"]}}});
    assert!(recover(root.path(), &steps).status.success());
    assert_eq!(
        result(root.path())["caches"]["project"]["verification"]["status"],
        "matched"
    );
    assert!(root.path().join("out/project/partial").is_file());
}

#[test]
fn retired_acceptance_registration_and_cli_fail_without_changing_outputs() {
    let (root, mut config, _) = verified_host();
    config["acceptance_fixtures"] = json!({"native-unusable":{
        "method":"remove-producer-manifest/v1","consumer":"check.one","caches":["project"]}});
    assert!(serde_json::from_value::<chrono_cache::Config>(config).is_err());
    let output = Command::new(Path::new(env!("CARGO_BIN_EXE_chrono-cache-test-cli")))
        .args(["acceptance-fixture", "--host-root"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(root.path().join("out/project/partial").is_file());
}
