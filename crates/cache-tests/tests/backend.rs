use super::*;

fn backend_fixture() -> (Host, Value) {
    let (root, mut config) = fixture();
    adopt_consumer_contract(&root, &mut config);
    config["backend"] = backend();
    let mut prepared = plan(root.path(), &config, "check.one").unwrap();
    // The backend fixture calls the installed report binary below, so bind the
    // plan to that same executable just as the native workflow does.
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cache/target/debug/chrono-cache");
    let executable = fs::canonicalize(&binary).unwrap();
    prepared["planner"] = json!({
        "executable": executable,
        "sha256": format!("{:x}", Sha256::digest(fs::read(&binary).unwrap())),
        "bytes": fs::metadata(&binary).unwrap().len(),
        "version": env!("CARGO_PKG_VERSION"),
    });
    (root, prepared)
}

fn backend() -> Value {
    json!({"kind":"github","need":"Confirm the backend entry for native saves",
        "output_use":"Report cache availability separately from action success",
        "command":{"program":env!("CARGO_BIN_EXE_chrono-cache-test-probe"),
            "args":["github","{endpoint}","backend-response.json"],"env":{},
            "timeout_seconds":5,"output_limit_bytes":65536},
        "inherit":["CACHE_TEST_SECRET"],"credential_environment":["CACHE_TEST_SECRET"],
        "repository_environment":"CACHE_TEST_REPOSITORY","ref_environment":"CACHE_TEST_REF"})
}

#[test]
fn backend_policy_is_explicit_and_does_not_invalidate_build_keys_or_query_during_planning() {
    let (root, mut config) = fixture();
    adopt_consumer_contract(&root, &mut config);
    let before = plan(root.path(), &config, "check.one").unwrap();
    config["backend"] = backend();
    let after = plan(root.path(), &config, "check.one").unwrap();
    assert_eq!(before["caches"], after["caches"]);
    assert_eq!(after["backend"], config["backend"]);
    assert!(!root.path().join("backend-call.txt").exists());
    for (pointer, value) in [
        ("/schema", json!("chrono-cache/v1")),
        ("/backend/kind", json!("unknown")),
        ("/backend/need", json!("")),
        (
            "/backend/command/args",
            json!(["github", "prefix{endpoint}"]),
        ),
        ("/backend/command/args", json!(["{endpoint}", "{endpoint}"])),
        ("/backend/inherit", json!(["GH_TOKEN"])),
        ("/backend/credential_environment", json!(["undeclared"])),
        (
            "/backend/inherit",
            json!(["CACHE_TEST_SECRET", "CACHE_TEST_SECRET"]),
        ),
        ("/backend/repository_environment", json!("INVALID=NAME")),
        ("/backend/ref_environment", json!("CACHE_TEST_REPOSITORY")),
    ] {
        let mut invalid = config.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(
            plan(root.path(), &invalid, "check.one").is_err(),
            "{invalid}"
        );
    }
}

fn entry(key: &str) -> Value {
    json!({"id":41,"ref":"refs/pull/7/merge","key":key,"version":"backend-version",
        "size_in_bytes":123,"created_at":"2026-01-01T00:00:00Z","last_accessed_at":"2026-01-01T00:00:00Z"})
}

fn run_report(root: &Host, prepared: &Value, response: &Value, save: &str) -> Value {
    root.write(
        root.path().join("backend-response.json"),
        serde_json::to_vec(response).unwrap(),
    )
    .unwrap();
    let plan_path = ".chrono-harness/state/backend-plan.json";
    root.directory(root.path().join(".chrono-harness/state"))
        .unwrap();
    root.write(
        root.path().join(plan_path),
        serde_json::to_vec(prepared).unwrap(),
    )
    .unwrap();
    let steps = json!({"cache_0_restore":{"outcome":"success","outputs":{}},
        "cache_0_save":{"outcome":save,"outputs":{}},"work":{"outcome":"failure"}});
    let output = Command::new(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../cache/target/debug/chrono-cache"),
    )
    .args([
        "report",
        "--host-root",
        root.path().to_str().unwrap(),
        "--plan",
        plan_path,
        "--steps-env",
        "CACHE_TEST_STEPS",
        "--work",
        "work",
        "--report-directory",
        ".chrono-harness/state/backend/",
    ])
    .env("CACHE_TEST_STEPS", serde_json::to_string(&steps).unwrap())
    .env("CACHE_TEST_REPOSITORY", "owner/repository")
    .env("CACHE_TEST_REF", "refs/pull/7/merge")
    .env("CACHE_TEST_SECRET", "private-fixture-credential")
    .output()
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    let report = &result["observation"];
    let retained: Value = serde_json::from_slice(
        &fs::read(root.path().join(result["report"]["path"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    assert_eq!(report, &retained);
    assert_eq!(report["work"]["outcome"], "failure");
    let original = fs::read(
        root.path().join(
            report["backend"]["original"]["path"]
                .as_str()
                .expect("backend original must be retained"),
        ),
    )
    .unwrap();
    assert!(!String::from_utf8_lossy(&original).contains("private-fixture-credential"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private-fixture-credential"));
    report.clone()
}

#[test]
fn backend_presence_requires_original_exact_key_and_ref_evidence() {
    let (root, prepared) = backend_fixture();
    let key = prepared["caches"]["project"]["key"].as_str().unwrap();
    let report = run_report(
        &root,
        &prepared,
        &json!([{"total_count":1,"actions_caches":[entry(key)]}]),
        "success",
    );
    assert_eq!(
        report["caches"]["project"]["save"]["status"],
        "backend-entry-observed"
    );
    assert_eq!(report["caches"]["project"]["save"]["confirmed"], true);
    assert_eq!(report["caches"]["project"]["backend"]["status"], "present");
    assert_eq!(
        report["caches"]["project"]["backend"]["entries"][0]["id"],
        41
    );
    assert_eq!(
        report["caches"]["project"]["save"]["creator"],
        "unestablished"
    );
    assert!(
        report["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|w| w["code"] != "W_CACHE_SAVE_UNCONFIRMED")
    );
    assert_eq!(
        fs::read_to_string(root.path().join("backend-call.txt")).unwrap(),
        "repos/owner/repository/actions/caches?ref=refs%2Fpull%2F7%2Fmerge&per_page=100\ncredential=true"
    );
    assert_eq!(
        report["current_executable_verification"]["status"],
        "matched"
    );
    assert_eq!(
        report["caches"]["project"]["current_executable_verification"],
        "matched"
    );
}

#[test]
fn backend_absence_ambiguity_and_query_failure_never_confirm_save() {
    for case in [
        "absent",
        "wrong-key",
        "wrong-ref",
        "ambiguous",
        "malformed",
        "failed",
    ] {
        let (root, mut prepared) = backend_fixture();
        let key = prepared["caches"]["project"]["key"].as_str().unwrap();
        let mut row = entry(key);
        let response = match case {
            "absent" => json!([{"total_count":0,"actions_caches":[]}]),
            "wrong-key" => {
                row["key"] = json!("other");
                json!([{"total_count":1,"actions_caches":[row]}])
            }
            "wrong-ref" => {
                row["ref"] = json!("refs/heads/other");
                json!([{"total_count":1,"actions_caches":[row]}])
            }
            "ambiguous" => {
                let mut other = row.clone();
                other["id"] = json!(42);
                other["version"] = json!("different-version");
                json!([{"total_count":2,"actions_caches":[row,other]}])
            }
            "failed" => {
                prepared["backend"]["command"]["args"] = json!(["exit", "{endpoint}"]);
                json!([])
            }
            _ => json!({"not":"pages"}),
        };
        let report = run_report(&root, &prepared, &response, "success");
        assert_eq!(
            report["caches"]["project"]["save"]["confirmed"], false,
            "{case}: {report}"
        );
        assert_eq!(
            report["caches"]["project"]["save"]["status"], "unconfirmed",
            "{case}: {report}"
        );
        assert!(
            report["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w["code"] == "W_CACHE_SAVE_UNCONFIRMED")
        );
        if case == "failed" {
            assert_eq!(report["backend"]["process"]["exit_code"], 7);
        }
    }
}

#[test]
fn backend_is_not_queried_without_a_successful_native_save() {
    for outcome in ["failure", "skipped"] {
        let (root, prepared) = backend_fixture();
        let key = prepared["caches"]["project"]["key"].as_str().unwrap();
        let report = run_report(
            &root,
            &prepared,
            &json!([{"total_count":1,"actions_caches":[entry(key)]}]),
            outcome,
        );
        assert_eq!(
            report["caches"]["project"]["backend"]["status"],
            "not-requested"
        );
        assert_eq!(report["backend"]["status"], "not-required");
        assert_eq!(report["backend"]["process"], Value::Null);
        assert!(!root.path().join("backend-call.txt").exists());
        assert_eq!(report["caches"]["project"]["save"]["confirmed"], false);
        assert_eq!(
            report["caches"]["project"]["save"]["status"],
            if outcome == "failure" {
                "error"
            } else {
                "not-attempted"
            }
        );
    }
}

#[test]
fn backend_pagination_retains_original_bytes_and_rejects_conflicting_identity() {
    let (root, prepared) = backend_fixture();
    let key = prepared["caches"]["project"]["key"].as_str().unwrap();
    let first = entry("unrelated");
    let mut wanted = entry(key);
    wanted["id"] = json!(42);
    let pages = json!([
        {"total_count":2,"actions_caches":[first.clone()]},
        {"total_count":2,"actions_caches":[first.clone(),wanted.clone()]}
    ]);
    let report = run_report(&root, &prepared, &pages, "success");
    assert_eq!(
        report["caches"]["project"]["save"]["status"],
        "backend-entry-observed"
    );
    assert_eq!(report["backend"]["entries"].as_array().unwrap().len(), 2);
    let raw: Vec<u8> =
        serde_json::from_value(report["backend"]["process"]["stdout_bytes"].clone()).unwrap();
    assert_eq!(
        raw,
        fs::read(root.path().join("backend-response.json")).unwrap()
    );
    wanted["id"] = first["id"].clone();
    let conflict = run_report(
        &root,
        &prepared,
        &json!([
            {"total_count":2,"actions_caches":[first]},
            {"total_count":2,"actions_caches":[wanted]}
        ]),
        "success",
    );
    assert_eq!(conflict["backend"]["status"], "unavailable");
    assert!(
        conflict["backend"]["error"]
            .as_str()
            .unwrap()
            .contains("conflicting cache id")
    );
    assert_eq!(conflict["caches"]["project"]["save"]["confirmed"], false);
}

#[test]
fn backend_bounded_process_failures_preserve_diagnostics_and_never_confirm_save() {
    for case in ["timeout", "truncated", "missing-executable"] {
        let (root, mut prepared) = backend_fixture();
        match case {
            "timeout" => {
                prepared["backend"]["command"]["args"] = json!(["sleep", "{endpoint}"]);
                prepared["backend"]["command"]["timeout_seconds"] = json!(1);
            }
            "truncated" => {
                prepared["backend"]["command"]["args"] = json!(["flood", "{endpoint}"]);
                prepared["backend"]["command"]["output_limit_bytes"] = json!(64);
            }
            _ => {
                prepared["backend"]["command"]["program"] = json!(root.path().join("missing-tool"))
            }
        }
        let report = run_report(&root, &prepared, &json!([]), "success");
        assert_eq!(
            report["backend"]["status"], "unavailable",
            "{case}: {report}"
        );
        assert_eq!(report["caches"]["project"]["save"]["confirmed"], false);
        assert!(report["backend"]["error"].is_string());
        if case == "missing-executable" {
            assert!(report["backend"]["process"].is_null());
        } else {
            assert!(
                report["backend"]["process"]["failure"].is_string(),
                "{case}: {report}"
            );
            assert!(report["backend"]["process"]["stdout_bytes"].is_array());
        }
        assert!(
            report["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w["code"] == "W_CACHE_BACKEND_UNAVAILABLE")
        );
    }
}
