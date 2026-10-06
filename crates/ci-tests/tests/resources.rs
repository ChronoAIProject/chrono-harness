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
