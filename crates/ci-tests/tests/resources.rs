//! Resource diagnostics exercise the actual parent collection transport.
use super::*;

#[test]
fn resource_observations_expose_cost_and_unknowns_without_extra_requests_or_business_work() {
    let mut h = GatedHost::new();
    let mut provider: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(PROVIDER)).unwrap()).unwrap();
    provider["gather"]["resource_observation"] = json!({"step_categories":{
        "Bootstrap registered tools":"bootstrap", "Canonical harness check":"check"
    },"summary_environment":"CHRONO_TEST_RESOURCE_SUMMARY"});
    json_file(&h.host.root, PROVIDER, &provider);
    let mut config: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(".chrono-harness/config.json")).unwrap())
            .unwrap();
    config["environment"]["inherit"]
        .as_array_mut()
        .unwrap()
        .push(json!("CHRONO_TEST_RESOURCE_SUMMARY"));
    json_file(&h.host.root, ".chrono-harness/config.json", &config);
    let summary = h
        .host
        .root
        .join(".chrono-harness/state/resource-summary.md");
    fs::create_dir_all(summary.parent().unwrap()).unwrap();
    fs::write(&summary, "").unwrap();
    h.host.revise();
    h.host.base = h.host.candidate.clone();
    git(&h.host.root, &["push", "-q", "origin", "dev"]);
    h.payload["before"] = json!(h.host.base);
    h.payload["after"] = json!(h.host.candidate);
    let d = h.detection();
    assert_eq!(d["required_units"], json!([]));
    let job = |id: u64, name: &str, steps: Value| {
        json!({"id":id,"run_id":101,"run_attempt":2,
        "name":name,"head_sha":d["candidate"],"status":"completed","conclusion":"success","steps":steps})
    };
    let step = |name: &str, start: &str, end: &str| {
        json!({"name":name,"status":"completed",
        "conclusion":"success","started_at":start,"completed_at":end})
    };
    let detector = job(
        200,
        "Detect registered DELTA",
        json!([
            step(
                "Bootstrap registered tools",
                "2026-10-06T23:59:55Z",
                "2026-10-07T00:00:05Z"
            ),
            step(
                "Canonical harness check",
                "2026-10-07T00:00:05Z",
                "2026-10-07T00:00:25Z"
            ),
            step(
                "New unregistered work",
                "2026-10-07T00:00:25Z",
                "2026-10-07T00:00:32Z"
            )
        ]),
    );
    let mut active = job(
        203,
        "collection",
        json!([
            {"name":"Canonical harness check","status":"in_progress","conclusion":null,
             "started_at":"2026-10-07T00:00:25Z","completed_at":null}
        ]),
    );
    active["status"] = json!("in_progress");
    let mut measured_job = detector.clone();
    measured_job["id"] = json!(201);
    measured_job["name"] = json!("unit alpha");
    let cases = [
        (
            json!([{"jobs":[job(200,"Detect registered DELTA",json!([])),measured_job.clone()]},{"jobs":[measured_job,active]}]),
            Some(37.0),
            1,
            1,
            "partial",
        ),
        (
            json!([{"jobs":[job(200,"Detect registered DELTA",json!([
                step("Bootstrap registered tools", "2026-10-07T00:00:05Z", "2026-10-07T00:00:04Z"),
                step("Canonical harness check", "bad", "2026-10-07T00:00:25Z")
            ]))]}]),
            None,
            2,
            0,
            "partial",
        ),
        (
            json!([{"jobs":[job(200,"Detect registered DELTA",Value::Null)]}]),
            None,
            0,
            0,
            "partial",
        ),
        (
            json!([{"jobs":[job(200,"Detect registered DELTA",json!([
                step("Bootstrap registered tools", "2026-10-07T08:00:00+08:00", "2026-10-07T00:00:02.500Z")
            ]))]}]),
            Some(2.5),
            0,
            0,
            "observed",
        ),
        (
            json!([{"jobs":[job(200,"Detect registered DELTA",json!([
                {"name":"Canonical harness check","status":"completed","conclusion":"skipped"}
            ]))]}]),
            None,
            0,
            0,
            "observed",
        ),
        (
            json!([{"jobs":[job(200,"Detect registered DELTA",json!([])),
                job(201,"other",json!([step("Bootstrap registered tools", "2026-10-07T00:00:00Z", "2026-10-07T00:00:10Z")])),
                job(201,"other",json!([step("Bootstrap registered tools", "2026-10-07T00:00:00Z", "2026-10-07T00:00:20Z")]))
            ]}]),
            None,
            0,
            0,
            "partial",
        ),
    ];
    for (pages, seconds, unknown, unclassified, status) in cases {
        json_file(
            &h.host.root,
            ".chrono-harness/state/parent-data.json",
            &json!({
                "detection":d,"mode":"resources","resource_pages":pages
            }),
        );
        let calls_path = h.host.root.join(".chrono-harness/state/parent-calls");
        let before = fs::read_to_string(&calls_path)
            .unwrap_or_default()
            .matches("/jobs?")
            .count();
        fs::write(&summary, "").unwrap();
        let out = h
            .command(&["check", "--collect"], &d, &needs(&h, &d))
            .env("CHRONO_TEST_RESOURCE_SUMMARY", &summary)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let report: Value = serde_json::from_slice(
            &fs::read(
                h.host
                    .root
                    .join(".chrono-harness/state/collection/gather.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let r = &report["resources"];
        assert_eq!(r["schema"], "chrono-ci-resources/v1");
        assert_eq!(r["status"], status);
        assert_eq!(r["known_step_seconds"], json!(seconds));
        assert_eq!(r["unknown_steps"], unknown);
        assert_eq!(r["unclassified_steps"], unclassified);
        assert_eq!(r["summary"]["status"], "written");
        let text = fs::read_to_string(&summary).unwrap();
        assert!(text.contains("not billing time"));
        assert!(text.contains(&format!("Status: **{status}**")));
        if seconds.is_none() {
            assert!(text.contains("**unknown seconds**"));
        }
        let source = r["source"]["process_index"].as_u64().unwrap() as usize;
        assert_eq!(
            r["source"]["stdout_sha256"],
            report["processes"][source]["stdout_sha256"]
        );
        let raw: Vec<u8> =
            serde_json::from_value(report["processes"][source]["stdout_bytes"].clone()).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&raw).unwrap(), pages);
        assert_eq!(
            fs::read_to_string(&calls_path)
                .unwrap()
                .matches("/jobs?")
                .count(),
            before + 1
        );
        assert!(!h.host.root.join(".chrono-harness/state/calls-a").exists());
        assert!(!h.host.root.join(".chrono-harness/state/calls-b").exists());
        if seconds == Some(37.0) {
            assert_eq!(r["jobs"].as_array().unwrap().len(), 3);
            assert_eq!(r["categories"]["bootstrap"]["known_step_seconds"], 10.0);
            assert_eq!(r["categories"]["check"]["known_step_seconds"], 20.0);
            assert_eq!(r["categories"]["unclassified"]["known_step_seconds"], 7.0);
        }
    }
    let mut failed = needs(&h, &d);
    failed["unit_alpha"]["result"] = json!("failure");
    let out = h
        .command(&["check", "--collect"], &d, &failed)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let report: Value = serde_json::from_slice(
        &fs::read(
            h.host
                .root
                .join(".chrono-harness/state/collection/gather.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report["resources"]["status"], "unavailable");
    assert!(report["resources"]["known_step_seconds"].is_null());
    assert!(report["resources"]["source"].is_null());
    assert_eq!(report["resources"]["summary"]["status"], "unavailable");
}

#[test]
fn resource_configuration_rejects_ambiguous_categories_and_unsupported_topology() {
    let h = GatedHost::new();
    let original: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(PROVIDER)).unwrap()).unwrap();
    for invalid in [
        Value::Null,
        json!({"step_categories":{}}),
        json!({"step_categories":{"known":"unclassified"}}),
        json!({"step_categories":{"known":" "}}),
        json!({"step_categories":{"known":"known"},"unknown_field":true}),
        json!({"step_categories":{"known":"known"},"summary_environment":"bad name"}),
    ] {
        let mut candidate = original.clone();
        candidate["gather"]["resource_observation"] = invalid;
        json_file(&h.host.root, PROVIDER, &candidate);
        assert!(generate(&h.host.root, PROVIDER, true).is_err());
    }
    let mut legacy = original;
    legacy.as_object_mut().unwrap().remove("job_gating");
    legacy["gather"]["resource_observation"] = json!({"step_categories":{"known":"known"}});
    json_file(&h.host.root, PROVIDER, &legacy);
    assert!(
        generate(&h.host.root, PROVIDER, true)
            .unwrap_err()
            .contains("resource observation requires")
    );
}

#[test]
fn resource_comparisons_expose_overhead_without_guessing_missing_measurements() {
    let mut h = GatedHost::new();
    let mut provider: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(PROVIDER)).unwrap()).unwrap();
    provider["gather"]["resource_observation"] = json!({
        "step_categories":{"prepare":"setup","execute":"work"},
        "summary_environment":"CHRONO_TEST_RESOURCE_SUMMARY",
        "comparisons":{"setup-cost":{
            "category":"setup","reference_category":"work","factor":1.0,
            "minimum_seconds":10
        },"scaled-cost":{
            "category":"setup","reference_category":"work","factor":10.0,
            "minimum_seconds":10
        }}
    });
    json_file(&h.host.root, PROVIDER, &provider);
    let config_path = ".chrono-harness/config.json";
    let mut config: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(config_path)).unwrap()).unwrap();
    config["environment"]["inherit"]
        .as_array_mut()
        .unwrap()
        .push(json!("CHRONO_TEST_RESOURCE_SUMMARY"));
    json_file(&h.host.root, config_path, &config);
    let summary = h
        .host
        .root
        .join(".chrono-harness/state/resource-summary.md");
    fs::create_dir_all(summary.parent().unwrap()).unwrap();
    fs::write(&summary, "").unwrap();
    h.host.revise();
    h.host.base = h.host.candidate.clone();
    git(&h.host.root, &["push", "-q", "origin", "dev"]);
    h.payload["before"] = json!(h.host.base);
    h.payload["after"] = json!(h.host.candidate);
    let d = h.detection();
    assert_eq!(d["required_units"], json!([]));
    let step = |name: &str, seconds: u64| {
        json!({"name":name,"status":"completed",
        "conclusion":"success","started_at":"2026-10-07T00:00:00Z",
        "completed_at":format!("2026-10-07T00:{:02}:{:02}Z", seconds / 60, seconds % 60)})
    };
    let job = |id: u64, steps: Value| {
        json!({"id":id,"run_id":101,"run_attempt":2,
        "name":if id == 200 {"Detect registered DELTA".into()} else {format!("job {id}")},"head_sha":d["candidate"],"status":"completed",
        "conclusion":"success","steps":steps})
    };
    let mut incomplete = job(204, json!([step("prepare", 120), step("execute", 20)]));
    incomplete["steps"][1]["completed_at"] = Value::Null;
    let mut running = job(205, json!([step("prepare", 120), step("execute", 20)]));
    running["status"] = json!("in_progress");
    let mut skipped = job(206, json!([]));
    skipped["conclusion"] = json!("skipped");
    let mut failed = job(209, json!([step("prepare", 120), step("execute", 20)]));
    failed["conclusion"] = json!("failure");
    failed["steps"][1]["conclusion"] = json!("failure");
    let measured = job(
        201,
        json!([
            step("prepare", 70),
            step("prepare", 50),
            step("execute", 20)
        ]),
    );
    let pages = json!([{"jobs":[
        job(200,json!([])),
        measured.clone(),
        job(210,json!([step("prepare",20),step("execute",20)])),
        job(202,json!([step("prepare",5),step("execute",1)])),
        job(203,json!([step("prepare",120),step("execute",0)])),
        incomplete,running,skipped,
        job(207,json!([step("prepare",120)])),
        job(208,json!([step("prepare",10),step("execute",20),step("unknown",500)])),
        failed
    ]},{"jobs":[measured]}]);
    json_file(
        &h.host.root,
        ".chrono-harness/state/parent-data.json",
        &json!({"detection":d,"mode":"resources","resource_pages":pages}),
    );
    let out = h
        .command(&["check", "--collect"], &d, &needs(&h, &d))
        .env("CHRONO_TEST_RESOURCE_SUMMARY", &summary)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(
        &fs::read(
            h.host
                .root
                .join(".chrono-harness/state/collection/gather.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let evaluations = report["resources"]["comparisons"].as_array().unwrap();
    assert_eq!(
        evaluations.len(),
        22,
        "duplicate API rows must not duplicate findings"
    );
    let scaled = evaluations
        .iter()
        .find(|v| v["job_id"] == 201 && v["rule_id"] == "scaled-cost")
        .unwrap();
    assert_eq!(
        scaled["status"], "within-threshold",
        "the host's factor must affect the comparison"
    );
    let evaluations: Vec<_> = evaluations
        .iter()
        .filter(|v| v["rule_id"] == "setup-cost")
        .collect();
    for (id, status, reason) in [
        (200, "unavailable", Some("category-not-observed")),
        (201, "exceeded", None),
        (210, "within-threshold", None),
        (202, "within-threshold", None),
        (203, "exceeded", None),
        (204, "unavailable", Some("category-measurement-incomplete")),
        (205, "unavailable", Some("job-not-completed")),
        (206, "not-applicable", Some("job-skipped")),
        (207, "unavailable", Some("category-not-observed")),
        (208, "within-threshold", None),
        (209, "exceeded", None),
    ] {
        let row = evaluations.iter().find(|v| v["job_id"] == id).unwrap();
        assert_eq!(row["status"], status, "{id}: {row}");
        assert_eq!(row["reason"], json!(reason));
        assert_eq!(row["rule_id"], "setup-cost");
        if status == "exceeded" {
            assert_eq!(row["code"], "W_CI_RESOURCE_COMPARISON");
            assert_eq!(row["category_seconds"], 120.0);
        } else {
            assert!(row["code"].is_null());
        }
    }
    let calls = fs::read_to_string(h.host.root.join(".chrono-harness/state/parent-calls")).unwrap();
    assert_eq!(calls.matches("/jobs?").count(), 1);
    assert!(!h.host.root.join(".chrono-harness/state/calls-a").exists());
    assert!(!h.host.root.join(".chrono-harness/state/calls-b").exists());
    let text = fs::read_to_string(&summary).unwrap();
    assert!(text.contains("4 warnings; 8 unavailable"), "{text}");
    assert!(
        text.contains("| setup-cost | job 201 / 201 / 2 | 120.0 | 20.0 | 1.0 |"),
        "{text}"
    );
    assert!(text.contains("do not establish unnecessary work or a failed check"));
}

#[test]
fn resource_comparison_rules_require_explicit_distinct_categories_and_finite_thresholds() {
    let h = GatedHost::new();
    let original: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(PROVIDER)).unwrap()).unwrap();
    let rule =
        json!({"category":"setup","reference_category":"work","factor":1.0,"minimum_seconds":10});
    for (field, value) in [
        ("category", json!("unknown")),
        ("reference_category", json!("setup")),
        ("factor", json!(0)),
        ("factor", json!(-1)),
        ("minimum_seconds", json!(-1)),
        ("extra", json!(true)),
    ] {
        let mut candidate = original.clone();
        let mut invalid = rule.clone();
        invalid[field] = value;
        candidate["gather"]["resource_observation"] = json!({
            "step_categories":{"prepare":"setup","execute":"work"},"comparisons":{"cost":invalid}});
        json_file(&h.host.root, PROVIDER, &candidate);
        assert!(generate(&h.host.root, PROVIDER, true).is_err(), "{field}");
    }
}
