use super::*;

fn host() -> Host {
    let h = Host::new();
    h.write("other.txt", "one\n");
    h.write(
        "other.sh",
        "mkdir -p .chrono-harness/state\nprintf other >> .chrono-harness/state/other\n",
    );
    h.change_registry(".chrono-harness/projects.json", |v| {
        v["owners"].as_array_mut().unwrap().push(json!("other"));
        v["projects"].as_array_mut().unwrap().push(json!({"id":"other","actions":{"execute":{"operation":"test.other","tool":"sh","argv":["other.sh"]}}}));
    });
    h.change_registry(".chrono-harness/FILEMAP.json", |v| {
        for path in ["other.txt", "other.sh"] {
            v["files"].as_array_mut().unwrap().push(json!({"path":path,"owner":"other","surface":"product","cost":"unmeasured","edges":[{"kind":"test-execution","to":"test:other"}]}));
        }
    });
    h.change_registry(CONFIG, |v| {
        v["schema"] = json!("chrono-ci-check/v3");
        v["policy"]["bindings"]["test:other"] = json!(["test.other"]);
        v["policy"]["units"] = json!({
            "alpha":{"tests":["test:suite"],"report_path":".chrono-harness/state/alpha/check.json"},
            "beta":{"tests":["test:other"],"report_path":".chrono-harness/state/beta/check.json"}
        });
        v["policy"]["shared_operations"] = json!({});
    });
    h.commit();
    h
}

fn unit(h: &Host, base: &str, candidate: &str, unit: &str) -> Response {
    let mut req = serde_json::to_value(h.request(Some(base), candidate)).unwrap();
    req["protocol"] = json!("chrono-ci-judge/v2");
    req["scope"] = json!({"kind":"unit","unit":unit});
    judge(&chrono_harness::decode(&serde_json::to_vec(&req).unwrap()).unwrap())
}

#[test]
fn registered_unit_runs_only_its_selected_plan() {
    let h = host();
    let b = h.head();
    h.write("src.txt", "two");
    h.write("other.txt", "two");
    let c = h.commit();
    let r = unit(&h, &b, &c, "alpha");
    pass(&r);
    assert_eq!(r.evidence["selected"], json!(["test:suite"]));
    assert_eq!(
        r.evidence["global_selected"],
        json!(["test:other", "test:suite"])
    );
    assert_eq!(r.evidence["assigned_elsewhere"], json!(["test:other"]));
    assert_eq!(r.evidence["operations"], json!(["test.suite"]));
    assert!(!h.root().join(".chrono-harness/state/other").exists());
    assert_eq!(h.calls(), 1);
}

#[test]
fn independent_unit_survives_another_units_failure() {
    let h = host();
    let b = h.head();
    h.write("check.sh", "exit 19\n");
    h.write("other.txt", "two");
    let c = h.commit();
    fail(&unit(&h, &b, &c, "alpha"), "19");
    let r = unit(&h, &b, &c, "beta");
    pass(&r);
    assert_eq!(r.evidence["operations"], json!(["test.other"]));
    assert_eq!(
        fs::read_to_string(h.root().join(".chrono-harness/state/other")).unwrap(),
        "other"
    );
    assert!(
        !r.evidence["not_required"]
            .as_array()
            .unwrap()
            .contains(&json!("test:suite"))
    );
}

#[test]
fn unit_without_delta_retains_other_unit_obligations() {
    let h = host();
    let b = h.head();
    h.write("other.txt", "two");
    let c = h.commit();
    let r = unit(&h, &b, &c, "alpha");
    pass(&r);
    assert_eq!(r.evidence["selected"], json!([]));
    assert_eq!(r.evidence["assigned_elsewhere"], json!(["test:other"]));
    assert_eq!(r.evidence["required_units"], json!(["beta"]));
    assert_eq!(r.evidence["not_required"], json!(["test:suite"]));
    assert_eq!(h.calls(), 0);
}

#[test]
fn missing_duplicate_unknown_assignments_fail_before_effects() {
    for mode in ["missing", "duplicate", "unknown"] {
        let h = host();
        let b = h.head();
        h.change_registry(CONFIG, |v| match mode {
            "missing" => {
                v["policy"]["units"].as_object_mut().unwrap().remove("beta");
            }
            "duplicate" => {
                v["policy"]["units"]["beta"]["tests"] = json!(["test:other", "test:suite"])
            }
            _ => v["policy"]["units"]["beta"]["tests"] = json!(["test:unknown"]),
        });
        let c = h.commit();
        fail(&unit(&h, &b, &c, "alpha"), mode);
        assert_eq!(h.calls(), 0);
    }
    let h = host();
    fail(&unit(&h, &h.head(), &h.head(), "absent"), "unregistered");
}

#[test]
fn shared_prerequisite_requires_exact_explicit_replication() {
    let h = host();
    let b = h.head();
    h.change_registry(CONFIG, |v| {
        v["policy"]["bindings"]["test:other"] = json!(["test.suite", "test.other"])
    });
    let c = h.commit();
    fail(&unit(&h, &b, &c, "beta"), "shared operations");
    assert_eq!(h.calls(), 0);
    h.change_registry(CONFIG, |v| {
        v["policy"]["shared_operations"] = json!({"test.suite":["alpha","beta"]})
    });
    let c = h.commit();
    let r = unit(&h, &b, &c, "beta");
    pass(&r);
    assert_eq!(
        r.evidence["operations"],
        json!(["test.suite", "test.other"])
    );
}

#[test]
fn unit_filter_does_not_hide_global_cycles() {
    let h = host();
    let b = h.head();
    h.change_registry(CONFIG, |v| {
        v["policy"]["bindings"] = json!({"test:suite":["test.suite","test.other"],"test:other":["test.other","test.suite"]});
        v["policy"]["shared_operations"] = json!({"test.suite":["alpha","beta"],"test.other":["alpha","beta"]});
    });
    let c = h.commit();
    fail(&unit(&h, &b, &c, "alpha"), "CYCLE");
    assert_eq!(h.calls(), 0);
    assert!(!h.root().join(".chrono-harness/state/other").exists());
}

fn report_host() -> Host {
    let h = host();
    h.change_registry(CONFIG, |v| v["judge"] = json!({"program":"/bin/cat","args":[".chrono-harness/state/producer.json"],"timeout_seconds":5,"output_limit_bytes":1048576}));
    h.commit();
    h
}

// The transport fixture records actual cat process bytes; unit results come from the real judge.
// Native runner/judge integration is exercised separately by ci-tests.
fn write_report(h: &Host, b: &str, c: &str, unit: &str) -> Value {
    let mut request = serde_json::to_value(h.request(Some(b), c)).unwrap();
    request["protocol"] = json!("chrono-ci-judge/v2");
    request["scope"] = json!({"kind":"unit","unit":unit});
    let mut request: Request =
        chrono_harness::decode(&serde_json::to_vec(&request).unwrap()).unwrap();
    request.host_root = fs::canonicalize(h.root()).unwrap();
    let response = judge(&request);
    h.json(
        ".chrono-harness/state/producer.json",
        &serde_json::to_value(&response).unwrap(),
    );
    let config = chrono_harness::load_config(&h.root().join(CONFIG)).unwrap();
    let process = chrono_harness::run_process(
        h.root(),
        &config.judge,
        &serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let runner = sha256(&fs::read("/bin/sh").unwrap());
    let judge = process.sha256.clone();
    let path = format!(".chrono-harness/state/{unit}/check.json");
    h.json(&path, &json!({"schema":"chrono-check-report/v1","request":request,"response":response,"judge":process,"runner":{"sha256":runner}}));
    json!({"unit":unit,"path":path,"sha256":sha256(&fs::read(h.root().join(&path)).unwrap()),"runner_sha256":runner,"judge_sha256":judge})
}

fn collect(h: &Host, b: &str, c: &str, reports: Value) -> Response {
    let manifest = ".chrono-harness/state/collection.json";
    h.json(
        manifest,
        &json!({"schema":"chrono-ci-collection/v1","reports":reports}),
    );
    let mut req = serde_json::to_value(h.request(Some(b), c)).unwrap();
    req["protocol"] = json!("chrono-ci-judge/v2");
    req["scope"] = json!({"kind":"collect","manifest":manifest});
    judge(&chrono_harness::decode(&serde_json::to_vec(&req).unwrap()).unwrap())
}

#[test]
fn collection_verifies_complete_reports_without_executing_tests() {
    let h = report_host();
    let b = h.head();
    h.write("src.txt", "two");
    h.write("other.txt", "two");
    let c = h.commit();
    let a = write_report(&h, &b, &c, "alpha");
    let other = write_report(&h, &b, &c, "beta");
    assert_eq!(h.calls(), 1);
    let r = collect(&h, &b, &c, json!([a, other]));
    pass(&r);
    assert_eq!(r.evidence["required_units"], json!(["alpha", "beta"]));
    assert_eq!(r.evidence["executed"], json!([]));
    assert_eq!(h.calls(), 1);
    assert_eq!(
        fs::read_to_string(h.root().join(".chrono-harness/state/other")).unwrap(),
        "other"
    );
    pass(&collect(&h, &c, &c, json!([])));
    assert_eq!(h.calls(), 1);
}

fn pad_report(h: &Host, mut report: Value, length: usize) -> Value {
    let path = h.root().join(report["path"].as_str().unwrap());
    let mut bytes = fs::read(&path).unwrap();
    assert!(bytes.len() < length);
    bytes.resize(length, b' ');
    fs::write(path, &bytes).unwrap();
    report["sha256"] = json!(sha256(&bytes));
    report
}

#[test]
fn collection_report_limit_accepts_exact_bytes_and_rejects_one_more() {
    let h = report_host();
    let limit = 2 * 1024 * 1024;
    h.change_registry(CONFIG, |v| {
        v["policy"]["collection_limits"] = json!({"manifest_bytes":4096,"report_bytes":limit});
    });
    let b = h.commit();
    h.write("src.txt", "two");
    let c = h.commit();
    let report = pad_report(&h, write_report(&h, &b, &c, "alpha"), limit);
    pass(&collect(&h, &b, &c, json!([report.clone()])));
    let too_large = pad_report(&h, report, limit + 1);
    let r = collect(&h, &b, &c, json!([too_large]));
    fail(&r, "collection input exceeds report_bytes");
    fail(&r, "2097153 > 2097152");
    fail(&r, ".chrono-harness/state/alpha/check.json");
    assert_eq!(h.calls(), 1);
}

#[test]
fn collection_can_adopt_large_original_reports_without_rewriting_evidence() {
    let length = 64 * 1024 * 1024 + 1;
    for explicit in [false, true] {
        let h = report_host();
        if explicit {
            h.change_registry(CONFIG, |v| {
                v["policy"]["collection_limits"] =
                    json!({"manifest_bytes":4096,"report_bytes":length});
            });
        }
        let b = h.commit();
        h.write("src.txt", "two");
        let c = h.commit();
        let report = pad_report(&h, write_report(&h, &b, &c, "alpha"), length);
        let original = report["sha256"].clone();
        let path = h.root().join(report["path"].as_str().unwrap());
        let r = collect(&h, &b, &c, json!([report]));
        if explicit {
            pass(&r);
            assert_eq!(r.evidence["reports"][0]["sha256"], original);
            assert_eq!(r.evidence["collection_limits"]["report_bytes"], length);
        } else {
            fail(&r, "collection input exceeds report_bytes");
        }
        assert_eq!(
            json!(chrono_harness::file_identity(&path).unwrap().0),
            original
        );
        assert_eq!(h.calls(), 1);
    }
}

#[test]
fn collection_manifest_limit_is_independent_and_checks_exact_bytes() {
    let h = report_host();
    h.change_registry(CONFIG, |v| {
        v["policy"]["collection_limits"] = json!({"manifest_bytes":128,"report_bytes":1});
    });
    let c = h.commit();
    let path = ".chrono-harness/state/collection.json";
    let mut request = serde_json::to_value(h.request(Some(&c), &c)).unwrap();
    request["protocol"] = json!("chrono-ci-judge/v2");
    request["scope"] = json!({"kind":"collect","manifest":path});
    let request: Request = serde_json::from_value(request).unwrap();
    for length in [128, 129] {
        let mut bytes = br#"{"schema":"chrono-ci-collection/v1","reports":[]}"#.to_vec();
        bytes.resize(length, b' ');
        h.write(path, std::str::from_utf8(&bytes).unwrap());
        let r = judge(&request);
        if length == 128 {
            pass(&r);
        } else {
            fail(&r, "collection input exceeds manifest_bytes");
            fail(&r, "129 > 128");
        }
    }
    assert_eq!(h.calls(), 0);
}

#[test]
fn collection_limits_reject_invalid_values_before_unit_operations() {
    for limits in [
        json!(null),
        json!({"manifest_bytes":0,"report_bytes":1024}),
        json!({"manifest_bytes":1024,"report_bytes":0}),
        json!({"manifest_bytes":1024,"report_bytes":u64::MAX}),
        json!({"manifest_bytes":1024,"report_bytes":-1}),
        json!({"manifest_bytes":1024}),
        json!({"manifest_bytes":1024,"report_bytes":1024,"unknown":1}),
    ] {
        let h = report_host();
        let b = h.head();
        h.change_registry(CONFIG, |v| v["policy"]["collection_limits"] = limits);
        h.write("src.txt", "two");
        let c = h.commit();
        let r = unit(&h, &b, &c, "alpha");
        assert_eq!(r.status, Status::Failed, "{r:?}");
        assert_eq!(h.calls(), 0);
    }
    let h = report_host();
    let b = h.head();
    h.change_registry(CONFIG, |v| {
        v["schema"] = json!("chrono-ci-check/v1");
        v["policy"].as_object_mut().unwrap().remove("units");
        v["policy"]
            .as_object_mut()
            .unwrap()
            .remove("shared_operations");
        v["policy"]["collection_limits"] = json!({"manifest_bytes":1024,"report_bytes":1024});
    });
    h.write("src.txt", "two");
    let c = h.commit();
    fail(
        &judge(&h.request(Some(&b), &c)),
        "CI units require chrono-ci-check/v3",
    );
    assert_eq!(h.calls(), 0);
}

#[test]
fn collection_rejects_missing_duplicate_stale_and_corrupt_originals() {
    let h = report_host();
    let b = h.head();
    h.write("src.txt", "two");
    h.write("other.txt", "two");
    let c = h.commit();
    let a = write_report(&h, &b, &c, "alpha");
    let other = write_report(&h, &b, &c, "beta");
    fail(&collect(&h, &b, &c, json!([a])), "missing required");
    fail(&collect(&h, &b, &c, json!([a, a, other])), "duplicate");
    fail(&collect(&h, &c, &c, json!([a, other])), "endpoints");
    for field in ["sha256", "runner_sha256", "judge_sha256"] {
        let mut changed = a.clone();
        changed[field] = json!("0".repeat(64));
        fail(&collect(&h, &b, &c, json!([changed, other])), "mismatch");
    }
    let path = a["path"].as_str().unwrap();
    let original = h.read(path);
    h.change_registry(path, |v| v["judge"]["stdout_bytes"][0] = json!(32));
    let mut changed = a.clone();
    changed["sha256"] = json!(sha256(&fs::read(h.root().join(path)).unwrap()));
    fail(
        &collect(&h, &b, &c, json!([changed, other])),
        "process bytes",
    );
    h.json(path, &original);
    pass(&collect(&h, &b, &c, json!([a, other])));
    assert_eq!(h.calls(), 1);
}

#[test]
fn collection_rechecks_candidate_plan_and_environment_beyond_report_hashes() {
    let h = report_host();
    h.change_registry(CONFIG, |v| {
        v["policy"]["environment"] = json!({"FIXTURE_LITERAL":"fixed"})
    });
    let b = h.commit();
    h.write("src.txt", "two");
    let c = h.commit();
    let input = write_report(&h, &b, &c, "alpha");
    let path = input["path"].as_str().unwrap();
    let original = h.read(path);
    for (mode, needle) in [
        ("environment", "environment differs"),
        ("method", "plan differs"),
        ("partition", "partition differs"),
        ("missing-process", "coverage missing"),
    ] {
        let mut report = original.clone();
        let e = &mut report["response"]["evidence"];
        match mode {
            "environment" => e["plan"]["environment"]["FIXTURE_LITERAL"] = json!("changed"),
            "method" => e["plan"]["operations"][0]["method"]["argv"] = json!(["other.sh"]),
            "partition" => e["assigned_elsewhere"] = json!(["test:other"]),
            _ => e["executed"] = json!([]),
        }
        let mut plan = e["plan"].clone();
        plan.as_object_mut().unwrap().remove("identity");
        e["plan"]["identity"] = json!(chrono_harness::wire::digest(&plan).unwrap());
        let stdout = serde_json::to_vec(&report["response"]).unwrap();
        report["judge"]["stdout_bytes"] = json!(stdout);
        report["judge"]["stdout"] = json!(String::from_utf8(stdout.clone()).unwrap());
        report["judge"]["stdout_sha256"] = json!(sha256(&stdout));
        h.json(path, &report);
        let mut changed = input.clone();
        changed["sha256"] = json!(sha256(&fs::read(h.root().join(path)).unwrap()));
        fail(&collect(&h, &b, &c, json!([changed])), needle);
    }
    h.json(path, &original);
    pass(&collect(&h, &b, &c, json!([input])));
    assert_eq!(h.calls(), 1);
}

#[test]
fn collection_supports_explicit_judge_resolved_through_recorded_path() {
    let h = report_host();
    h.change_registry(CONFIG, |v| v["judge"]["program"] = json!("cat"));
    let b = h.commit();
    h.write("src.txt", "two");
    let c = h.commit();
    let report = write_report(&h, &b, &c, "alpha");
    pass(&collect(&h, &b, &c, json!([report])));
    assert_eq!(h.calls(), 1);
}
