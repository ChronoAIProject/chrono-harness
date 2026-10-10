use super::*;

fn bind_installed_planner(plan: &mut Value) {
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cache/target/debug/chrono-cache");
    plan["planner"] = json!({
        "executable": fs::canonicalize(&binary).unwrap(),
        "sha256": format!("{:x}", Sha256::digest(fs::read(&binary).unwrap())),
        "bytes": fs::metadata(&binary).unwrap().len(),
        "version": env!("CARGO_PKG_VERSION"),
    });
}

fn recovery_host() -> (Host, Value, Value) {
    let (root, mut config) = fixture();
    adopt_consumer_contract(root.path(), &mut config);
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

fn recover(root: &Path, steps: &Value) -> std::process::Output {
    Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../cache/target/debug/chrono-cache"))
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

fn result(root: &Path) -> Value {
    let pointer: Value = serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/state/restore/recovery.json")).unwrap(),
    )
    .unwrap();
    let raw = fs::read(root.join(pointer["path"].as_str().unwrap())).unwrap();
    assert_eq!(pointer["sha256"], format!("{:x}", Sha256::digest(&raw)));
    serde_json::from_slice(&raw).unwrap()
}

fn transport(root: &Path, steps: &Value) -> std::process::Output {
    Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../cache/target/debug/chrono-cache"))
        .args([
            "report",
            "--host-root",
            root.to_str().unwrap(),
            "--plan",
            ".chrono-harness/state/restore/plan.json",
            "--report-directory",
            ".chrono-harness/cache/release-result/",
            "--steps-env",
            "CACHE_RECOVERY_STEPS",
            "--work",
            "work",
        ])
        .env(
            "CACHE_RECOVERY_STEPS",
            serde_json::to_string(steps).unwrap(),
        )
        .output()
        .unwrap()
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
    let mut final_steps = steps.clone();
    final_steps["work"] = json!({"outcome":"failure"});
    let delivered = transport(root.path(), &final_steps);
    assert!(
        delivered.status.success(),
        "{}",
        String::from_utf8_lossy(&delivered.stderr)
    );
    let delivered: Value = serde_json::from_slice(&delivered.stdout).unwrap();
    assert_eq!(delivered["observation"]["work"]["outcome"], "failure");
    assert_eq!(
        delivered["observation"]["caches"]["project"]["restore"]["status"],
        "error"
    );
    let recovery = &delivered["observation"]["recovery"];
    assert_eq!(recovery["status"], "recovered");
    let uploaded = fs::read(
        root.path()
            .join(recovery["original"]["path"].as_str().unwrap()),
    )
    .unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&uploaded).unwrap(), report);
    let uploaded_steps = fs::read(
        root.path()
            .join(recovery["native_steps"]["path"].as_str().unwrap()),
    )
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&uploaded_steps).unwrap(),
        steps
    );
}

#[test]
fn missing_or_damaged_recovery_evidence_stops_report_production() {
    for damaged in [false, true] {
        let (root, _, _) = recovery_host();
        let steps = json!({"cache_0_restore":{"outcome":"success"},"work":{"outcome":"success"}});
        if damaged {
            assert!(recover(root.path(), &steps).status.success());
            let pointer: Value = serde_json::from_slice(
                &fs::read(
                    root.path()
                        .join(".chrono-harness/state/restore/recovery.json"),
                )
                .unwrap(),
            )
            .unwrap();
            fs::write(
                root.path().join(pointer["path"].as_str().unwrap()),
                b"damaged original",
            )
            .unwrap();
        }
        let output = transport(root.path(), &steps);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(if damaged {
                "digest mismatch"
            } else {
                "missing recovery result"
            })
        );
    }
}

#[test]
fn changed_restore_observations_stop_transport_and_preserve_both_originals() {
    for fault in ["missing", "outcome", "key"] {
        let (root, _, prepared) = recovery_host();
        let restore = if fault == "key" {
            json!({"outcome":"success","outputs":{"cache-matched-key":prepared["caches"]["project"]["key"]}})
        } else {
            json!({"outcome":"failure","conclusion":"success","outputs":{}})
        };
        let initial = json!({"cache_0_restore":restore});
        assert!(recover(root.path(), &initial).status.success());
        let recovery = result(root.path());
        let mut final_steps = initial.clone();
        final_steps["work"] = json!({"outcome":"failure"});
        match fault {
            "missing" => {
                final_steps
                    .as_object_mut()
                    .unwrap()
                    .remove("cache_0_restore");
            }
            "outcome" => final_steps["cache_0_restore"]["outcome"] = json!("success"),
            "key" => {
                final_steps["cache_0_restore"]["outputs"]["cache-matched-key"] =
                    json!("different-key");
            }
            _ => unreachable!(),
        }
        let output = transport(root.path(), &final_steps);
        assert!(
            !output.status.success(),
            "{fault}: contradictory restore observations were accepted: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("E_CACHE_REPORT"), "{error}");
        assert!(error.contains("project"), "{error}");
        assert!(error.contains("restore observation"), "{error}");
        assert_eq!(result(root.path()), recovery);
        let upload = root.path().join(".chrono-harness/cache/release-result");
        for (prefix, expected) in [
            ("restore-steps-", &initial),
            ("native-steps-", &final_steps),
        ] {
            let originals: Vec<_> = fs::read_dir(&upload)
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.file_name().unwrap().to_str().unwrap().starts_with(prefix))
                .collect();
            assert_eq!(originals.len(), 1, "{fault}: {prefix}");
            let raw = fs::read(&originals[0]).unwrap();
            assert_eq!(&serde_json::from_slice::<Value>(&raw).unwrap(), expected);
            assert!(error.contains(originals[0].file_name().unwrap().to_str().unwrap()));
        }
        assert!(fs::read_dir(&upload).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .starts_with("cache-transport-")
        }));
        assert_eq!(fs::read(root.path().join("source")).unwrap(), b"source-one");
    }
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
        let mut steps = json!({"cache_0_restore":restore});
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
        steps["work"] = json!({"outcome":"failure"});
        let transported = transport(root.path(), &steps);
        assert!(
            transported.status.success(),
            "{}",
            String::from_utf8_lossy(&transported.stderr)
        );
        let transported: Value = serde_json::from_slice(&transported.stdout).unwrap();
        let observed = &transported["observation"];
        assert_eq!(observed["work"]["outcome"], "failure");
        assert_eq!(observed["caches"]["project"]["restore"]["status"], reason);
        assert_eq!(
            observed["caches"]["project"]["restore"]["observation"],
            restore
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
        "owner":"project","path":"out/socket","kind":"compilation","external":false
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
    let socket_root = tempfile::tempdir().unwrap();
    let socket_path = socket_root.path().join("socket");
    // The actual special-file fixture must work under the adopted absolute
    // TMPDIR, even when that root exceeds the kernel socket-address limit.
    // Bind a short relative address in an explicitly rooted Rust child; the
    // test process never changes its cwd or its allocation environment.
    let bound = std::process::Command::new(env!("CARGO_BIN_EXE_chrono-cache-test-probe"))
        .current_dir(socket_root.path())
        .arg("socket")
        .output()
        .unwrap();
    assert!(bound.status.success(), "native socket fixture: {bound:?}");
    use std::os::unix::fs::FileTypeExt;
    assert!(fs::metadata(&socket_path).unwrap().file_type().is_socket());
    fs::rename(socket_path, root.path().join("out/socket")).unwrap();
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
    let delivered = transport(root.path(), &steps);
    assert!(
        delivered.status.success(),
        "{}",
        String::from_utf8_lossy(&delivered.stderr)
    );
    let delivered: Value = serde_json::from_slice(&delivered.stdout).unwrap();
    assert_eq!(delivered["observation"]["recovery"]["status"], "failed");
    assert_eq!(
        delivered["observation"]["recovery"]["error"],
        original["error"]
    );
}
