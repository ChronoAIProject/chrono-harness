#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::sha256;
use serde_json::{Value, json};
use std::{fs, process::Command};
use support::*;
#[path = "support/host.rs"]
mod host;
use host::Host;

fn check_pass(code: i32, v: &Value) {
    assert_eq!(code, 0, "{}", v.get("findings").unwrap_or(v));
    assert_eq!(v["tests"]["tests"]["test:t"], "passed");
}
#[test]
fn committed_project_and_script_hosts_execute_actual_four_judge_chain() {
    for script in [false, true] {
        let h = Host::new(script);
        let (code, v) = h.run(|_| {});
        check_pass(code, &v);
        assert_eq!(
            fs::read_to_string(h.root().join(".chrono-harness/state/order")).unwrap(),
            "pt"
        );
        assert_eq!(v["tests"]["executed"].as_array().unwrap().len(), 2);
        let p = &v["tests"]["executed"][1]["receipt"]["process"];
        let data: Value = serde_json::from_str(p["stdout"].as_str().unwrap()).unwrap();
        assert_eq!(data[0], json!(["$HOME `literal` <tag>", "", "line\nnext"]));
        assert_eq!(data[1], json!(h.root()));
        assert_eq!(data[2], "");
        assert!(data[3].is_null());
    }
}
#[test]
fn retained_missing_changed_and_wrong_endpoint_cannot_pass() {
    let h = Host::new(true);
    for mode in 0..3 {
        let (code, v) = h.run(|i| match mode {
            0 => i["base"]["files"].as_object_mut().unwrap().clear(),
            1 => i["candidate"]["files"]["data"]["bytes"] = json!([0]),
            _ => i["base"]["commit"] = json!("wrong"),
        });
        assert_ne!(code, 0);
        assert!(v.to_string().contains("E_EVIDENCE_UNRESOLVED"));
        assert!(!h.root().join(".chrono-harness/state/order").exists());
    }
    fs::write(h.external.path(), "changed").unwrap();
    let (code, v) = h.run(|_| {});
    assert_ne!(code, 0);
    assert!(v.to_string().contains("candidate input changed"));
}
#[test]
fn docs_and_unrelated_historical_pair_defect_launch_zero_operations() {
    let mut h = Host::new(false);
    h.values.get_mut(PROJECTS).unwrap()["projects"][0]["test_project"] =
        json!("missing-historical");
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("doc.txt"), "docs changed").unwrap();
    h.candidate = commit(&h.root());
    let (code, v) = h.run(|_| {});
    assert_eq!(code, 0, "{}", v["findings"]);
    assert_eq!(v["tests"]["executed"], json!([]));
    assert!(!h.root().join(".chrono-harness/state/order").exists());
}
#[test]
fn pair_edge_manifest_and_route_counterexamples_launch_nothing() {
    for mode in 0..5 {
        let mut h = Host::new(false);
        match mode {
            0 => h.values.get_mut(PROJECTS).unwrap()["projects"][0]["test_project"] = json!("p"),
            1 => h.values.get_mut(FM).unwrap()["project_edges"]
                .as_array_mut()
                .unwrap()
                .retain(|e| e["from"] != "project:p" || e["kind"] != "test-execution"),
            2 => h.values.get_mut(FM).unwrap()["project_edges"]
                .as_array_mut()
                .unwrap()
                .retain(|e| e["kind"] != "compile"),
            3 => {
                h.values.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]["execute"] =
                    h.values[PROJECTS]["projects"][1]["actions"]["execute"].clone()
            }
            _ => {
                h.values.get_mut(CONFIG).unwrap()["tools"][0]["expected_version"] =
                    json!("wrong version")
            }
        };
        h.save();
        let (code, v) = h.run(|_| {});
        assert_ne!(code, 0, "mode {mode}");
        assert!(
            v.to_string().contains(match mode {
                0 | 1 => "E_TEST_PAIR",
                2 => "E_DANGLING_EDGE",
                3 => "E_ROUTE_AMBIGUOUS",
                _ => "E_TOOL_BINDING",
            }),
            "mode {mode}: {}",
            v["findings"]
        );
        assert!(!h.root().join(".chrono-harness/state/order").exists());
    }
}
#[test]
fn failed_test_and_candidate_mutation_are_visible() {
    for mutation in [false, true] {
        let mut h = Host::new(true);
        fs::write(
            h.root().join("t/check.py"),
            if mutation {
                "open('doc.txt','w').write('mutation')\n"
            } else {
                "print('PASS')\nraise SystemExit(9)\n"
            },
        )
        .unwrap();
        h.candidate = commit(&h.root());
        let (code, v) = h.run(|_| {});
        assert_ne!(code, 0);
        assert!(v.to_string().contains(if mutation {
            "E_SNAPSHOT_DIRTY"
        } else {
            "E_EXECUTION"
        }));
        if !mutation {
            assert_eq!(
                v["tests"]["executed"][1]["receipt"]["process"]["exit_code"],
                9
            );
        }
    }
}

#[test]
fn resource_failure_is_infrastructure_error_with_observed_output() {
    let mut h = Host::new(true);
    fs::write(h.root().join("t/check.py"), "print('x'*8000)\n").unwrap();
    h.candidate = commit(&h.root());
    let (code, v) = h.run(|_| {});
    assert_eq!(
        code, 2,
        "resource failure must be protocol error: {}",
        v["findings"]
    );
    assert_eq!(v["status"], "error");
    let result = &v["tests"]["executed"][1];
    assert_eq!(result["status"], "error");
    assert_eq!(
        result["receipt"]["process"]["stdout_bytes"]
            .as_array()
            .unwrap()
            .len(),
        4096
    );
    assert!(
        result["receipt"]["process"]["failure"]
            .as_str()
            .unwrap()
            .contains("output limit")
    );
}

#[test]
fn scoped_v2_adapter_uses_observed_canonical_invocation_and_shared_execution() {
    let mut h = Host::new(true);
    let root = h.root();
    let profile = ".chrono-harness/scoped.json";
    fs::copy(
        source().join("crates/judge-ci/target/debug/chrono-judge-ci"),
        root.join(".chrono-harness/bin/chrono-judge-ci"),
    )
    .unwrap();
    h.values.get_mut(CONFIG).unwrap()["canonical_check"]["argv"] = json!([
        ".chrono-harness/bin/chrono-harness",
        "check",
        "--config",
        profile,
        "--base",
        "{base}",
        "--candidate",
        "{candidate}"
    ]);
    let artifacts: Vec<_> = h.values[CONFIG]["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["path"].clone())
        .collect();
    let config = json!({"schema":"chrono-ci-check/v1","judge":{"program":".chrono-harness/bin/chrono-judge-ci","args":[],"timeout_seconds":30,"output_limit_bytes":1048576},"report_path":".chrono-harness/state/scoped.json","policy":{"filemap":FM,"projects":PROJECTS,"registration_config":CONFIG,"tools":{"python":h.tool},"artifacts":artifacts,"required_inputs":[profile],"operation_timeout_seconds":15,"operation_output_limit_bytes":4096,"environment":{}}});
    fs::write(root.join(profile), serde_json::to_vec(&config).unwrap()).unwrap();
    h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(file(profile, json!([])));
    h.save();
    h.base = h.candidate.clone();
    fs::write(root.join("p/product.py"), "def double(n): return 2*n\n").unwrap();
    h.candidate = commit(&root);
    fs::create_dir_all(root.join(".chrono-harness/state")).unwrap();
    let call = |reordered: bool| {
        let config = root.join(profile);
        let args = if reordered {
            vec![
                "check",
                "--base",
                &h.base,
                "--config",
                config.to_str().unwrap(),
                "--candidate",
                &h.candidate,
            ]
        } else {
            vec![
                "check",
                "--config",
                config.to_str().unwrap(),
                "--base",
                &h.base,
                "--candidate",
                &h.candidate,
            ]
        };
        let output = Command::new(root.join(".chrono-harness/bin/chrono-harness"))
            .current_dir("/")
            .args(args)
            .output()
            .unwrap();
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        (output.status.code().unwrap(), report)
    };
    let (code, report) = call(false);
    assert_eq!(code, 0, "{}", report["response"]);
    assert_eq!(
        report["response"]["evidence"]["selected"],
        json!(["test:t"])
    );
    assert_eq!(
        report["response"]["evidence"]["executed"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        fs::read_to_string(root.join(".chrono-harness/state/order")).unwrap(),
        "pt"
    );
    let (code, report) = call(true);
    assert_ne!(code, 0);
    assert!(
        report["response"]
            .to_string()
            .contains("canonical argv differs")
    );
    assert_eq!(
        fs::read_to_string(root.join(".chrono-harness/state/order")).unwrap(),
        "pt",
        "unregistered invocation launched nothing"
    );
}

#[test]
fn mapped_removed_obligation_requires_successful_replacement_and_preserves_old_method() {
    for mapping in [true, false] {
        let mut h = Host::new(true);
        h.values.get_mut(PROJECTS).unwrap()["scripts"][0]["test_script"] = json!("replacement");
        h.values.get_mut(PROJECTS).unwrap()["scripts"][1]["id"] = json!("replacement");
        h.values.get_mut(PROJECTS).unwrap()["owners"]
            .as_array_mut()
            .unwrap()
            .push(json!("replacement"));
        for f in h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
        {
            if f["owner"] == "t" {
                f["owner"] = json!("replacement");
            }
            for e in f["edges"].as_array_mut().unwrap() {
                if e["to"] == "script:t" {
                    e["to"] = json!("script:replacement");
                }
            }
        }
        for e in h.values.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
        {
            if e["from"] == "script:t" {
                e["from"] = json!("script:replacement");
            }
            if e["to"] == "test:t" {
                e["to"] = json!("test:replacement");
            }
        }
        let old = h.values.get_mut(FM).unwrap()["execution_plans"]
            .as_object_mut()
            .unwrap()
            .remove("test:t")
            .unwrap();
        h.values.get_mut(FM).unwrap()["execution_plans"]["test:replacement"] = old;
        h.values.get_mut(FM).unwrap()["test_costs"][0]["test"] = json!("replacement");
        if mapping {
            h.values.get_mut(WORKFLOW).unwrap()["retirements"] = json!([{"kind":"test","id":"t","replacement":"replacement","reason":"preserve original execute method"}]);
        }
        h.save();
        let (code, v) = h.run(|_| {});
        if mapping {
            assert_eq!(code, 0, "{}", v["findings"]);
            assert_eq!(v["tests"]["removed"]["test:t"], "test:replacement");
            assert_eq!(v["tests"]["tests"]["test:replacement"], "passed");
        } else {
            assert_ne!(code, 0);
            assert!(v.to_string().contains("E_REQUIRED_TEST_REMOVED"));
            assert!(!h.root().join(".chrono-harness/state/order").exists());
        }
    }
}

#[test]
fn finite_old_ambiguity_mapping_reaches_real_execution_without_dropping_either_method() {
    let mut h = Host::new(false);
    h.values.get_mut(PROJECTS).unwrap()["scripts"] = json!([
      {"id":"s","path":"s.py","test_script":"t","actions":{"execute":{"operation":"script.s","tool":"python","argv":["-B","s.py"]}}},
      {"id":"t","path":"st.py","tests_for":"s","actions":{"execute":{"operation":"script.st","tool":"python","argv":["-B","st.py"]}}}
    ]);
    h.values.get_mut(PROJECTS).unwrap()["owners"]
        .as_array_mut()
        .unwrap()
        .push(json!("s"));
    for (path, owner, code) in [
        ("s.py", "s", "def value(): return 42\n"),
        (
            "st.py",
            "t",
            "import runpy\nassert runpy.run_path('s.py')['value']()==42\nprint('old script obligation executed')\n",
        ),
    ] {
        let mut f = file(path, json!([]));
        f["owner"] = json!(owner);
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(f);
        fs::write(h.root().join(path), code).unwrap();
    }
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("script:s", "test-execution", "test:t"));
    h.save();
    h.base = h.candidate.clone();
    h.values.get_mut(PROJECTS).unwrap()["scripts"][0]["test_script"] = json!("st");
    h.values.get_mut(PROJECTS).unwrap()["scripts"][1]["id"] = json!("st");
    h.values.get_mut(PROJECTS).unwrap()["owners"]
        .as_array_mut()
        .unwrap()
        .push(json!("st"));
    for f in h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
    {
        if f["path"] == "st.py" {
            f["owner"] = json!("st");
        }
    }
    for e in h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
    {
        if e["from"] == "script:s" {
            e["to"] = json!("test:st");
        }
    }
    h.values.get_mut(FM).unwrap()["execution_plans"]["test:t"]["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!("script.st"));
    h.values.get_mut(FM).unwrap()["execution_plans"]["test:st"] =
        json!({"operations":["script.st"],"timeout_seconds":15,"output_limit_bytes":4096});
    h.values.get_mut(FM).unwrap()["test_costs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"test":"st","cost":"unknown"}));
    // This v2->v2 repair consumes mappings only; no schema decoder is run or certified.
    h.values.get_mut(WORKFLOW).unwrap()["schema_version"] = json!(2);
    h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"] = json!([{"id":"chrono-ci-check/v1","filemap_version":1,"profile_path":"profile.json","script":"s","test":"st","mappings":[{"from":"script:t","to":"script:st"}],"legacy_records":[],"ambiguities":[{"node":"test:t","definitions":["project:t","script:t"],"replacement":"test:t"}]}]);
    h.save();
    let (code, v) = h.run(|_| {});
    assert_eq!(code, 0, "{}", v["findings"]);
    assert_eq!(v["tests"]["tests"]["test:t"], "passed");
    assert_eq!(v["tests"]["tests"]["test:st"], "passed");
    assert_eq!(
        v["impact"]["nodes"]["test:t"]["base_definitions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        v["tests"]["executed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["operation"] == "script.st")
    );
}

#[test]
fn operation_mutating_external_input_is_rejected_after_receipt() {
    let mut h = Host::new(true);
    fs::write(
        h.root().join("t/check.py"),
        format!(
            "open({:?},'w').write('changed external input')\n",
            h.external.path().to_str().unwrap()
        ),
    )
    .unwrap();
    h.candidate = commit(&h.root());
    let (code, v) = h.run(|_| {});
    assert_ne!(code, 0);
    assert!(
        v.to_string()
            .contains("post-execution candidate input changed")
    );
    assert_eq!(
        v["tests"]["executed"][1]["receipt"]["process"]["exit_code"],
        0
    );
}

fn environment_host(connected: bool, overriding: bool) -> Host {
    let mut h = Host::new(true);
    if connected {
        for name in ["DECLARED_EMPTY", "DECLARED_ABSENT"] {
            h.values.get_mut(FM).unwrap()["project_edges"]
                .as_array_mut()
                .unwrap()
                .push(edge(
                    &format!("environment:{name}"),
                    "runtime-input",
                    "script:p",
                ));
        }
    }
    if overriding {
        h.values.get_mut(CONFIG).unwrap()["environment"]["values"]["DECLARED_ABSENT"] =
            json!("fixed");
    }
    fs::write(h.root().join("t/check.py"), "import os\nopen('.chrono-harness/state/order','a').write('t')\nprint(repr((os.environ.get('DECLARED_ABSENT'),os.environ.get('DECLARED_EMPTY'))))\nraise SystemExit(9)\n").unwrap();
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("doc.txt"), "environment comparison only\n").unwrap();
    h.candidate = commit(&h.root());
    h
}
#[test]
fn retained_present_to_absent_selects_and_executes_failing_consumer() {
    let h = environment_host(true, false);
    let (code, v) = h.run(|i| i["base"]["environment"]["DECLARED_ABSENT"] = json!("old"));
    assert_eq!(
        code, 1,
        "changed effective input must execute consumer: {}",
        v["findings"]
    );
    assert_eq!(v["tests"]["selected"], json!(["test:t"]));
    assert_eq!(
        v["tests"]["executed"][1]["receipt"]["process"]["exit_code"],
        9
    );
    assert_eq!(
        fs::read_to_string(h.root().join(".chrono-harness/state/order")).unwrap(),
        "pt"
    );
    assert!(
        v["impact"]["seeds"]
            .as_array()
            .unwrap()
            .contains(&json!("environment:DECLARED_ABSENT"))
    );
}
#[test]
fn retained_absent_to_empty_selects_and_executes_failing_consumer() {
    let h = environment_host(true, false);
    let (code, v) = h.run(|i| i["base"]["environment"]["DECLARED_EMPTY"] = Value::Null);
    assert_eq!(code, 1, "absent differs from empty: {}", v["findings"]);
    assert_eq!(
        v["tests"]["executed"][1]["receipt"]["process"]["exit_code"],
        9
    );
    assert_eq!(
        fs::read_to_string(h.root().join(".chrono-harness/state/order")).unwrap(),
        "pt"
    );
}
#[test]
fn unchanged_disconnected_and_overridden_environment_do_not_execute() {
    for (connected, overriding, changed) in [
        (true, false, false),
        (false, false, true),
        (true, true, true),
    ] {
        let h = environment_host(connected, overriding);
        let (code, v) = h.run(|i| {
            if changed {
                i["base"]["environment"]["DECLARED_ABSENT"] = json!("old");
            }
        });
        assert_eq!(code, 0, "{}", v["findings"]);
        assert_eq!(v["tests"]["executed"], json!([]));
        assert!(!h.root().join(".chrono-harness/state/order").exists());
    }
}
#[test]
fn unrepresented_retained_environment_is_not_no_work_success() {
    let h = environment_host(false, false);
    let (code, v) = h.run(|i| i["base"]["environment"]["UNDECLARED"] = json!("old"));
    assert_ne!(code, 0, "unrepresented input cannot silently disappear");
    assert!(v.to_string().contains("unregistered retained environment"));
    assert!(!h.root().join(".chrono-harness/state/order").exists());
}

#[test]
fn registered_intermediate_workspace_rejected_before_operations_and_standalone_passes() {
    for workspace in [false, true] {
        let mut h = Host::new(false);
        let root = h.root();
        fs::create_dir(root.join("group")).unwrap();
        for id in ["p", "t"] {
            fs::rename(root.join(id), root.join(format!("group/{id}"))).unwrap();
        }
        for p in h.values.get_mut(PROJECTS).unwrap()["projects"]
            .as_array_mut()
            .unwrap()
        {
            for field in ["root", "manifest", "lockfile"] {
                p[field] = json!(format!("group/{}", p[field].as_str().unwrap()));
            }
        }
        h.values.get_mut(PROJECTS).unwrap()["projects"][1]["actions"]["execute"]["argv"][1] =
            json!("group/t/check.py");
        for f in h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
        {
            let path = f["path"].as_str().unwrap();
            if path.starts_with("p/") || path.starts_with("t/") {
                f["path"] = json!(format!("group/{path}"));
            }
        }
        for a in h.values.get_mut(CONFIG).unwrap()["artifacts"]
            .as_array_mut()
            .unwrap()
        {
            if a["kind"] == "cargo-output" {
                a["path"] = json!(format!("group/{}", a["path"].as_str().unwrap()));
            }
        }
        let check = fs::read_to_string(root.join("group/t/check.py"))
            .unwrap()
            .replace("p/product.py", "group/p/product.py");
        fs::write(root.join("group/t/check.py"), check).unwrap();
        if workspace {
            fs::write(
                root.join("group/Cargo.toml"),
                "[workspace]\nmembers=['p','t']\n",
            )
            .unwrap();
            h.values.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .push(file("group/Cargo.toml", json!([])));
        }
        h.save();
        h.base = h.candidate.clone();
        fs::write(root.join("doc.txt"), "docs only\n").unwrap();
        h.candidate = commit(&root);
        let (code, v) = h.run(|_| {});
        assert_eq!(
            code, 0,
            "docs do not adjudicate historical Cargo data: {}",
            v["findings"]
        );
        assert_eq!(v["tests"]["executed"], json!([]));
        fs::write(
            root.join("group/p/product.py"),
            "def double(n): return 2*n\n",
        )
        .unwrap();
        h.candidate = commit(&root);
        let (code, v) = h.run(|_| {});
        if workspace {
            assert_ne!(
                code, 0,
                "registered intermediate workspace must be rejected"
            );
            assert!(v.to_string().contains("E_TEST_PAIR"), "{}", v["findings"]);
            assert!(v.to_string().contains("group/Cargo.toml"));
            assert!(!root.join(".chrono-harness/state/order").exists());
        } else {
            check_pass(code, &v);
            assert_eq!(
                fs::read_to_string(root.join(".chrono-harness/state/order")).unwrap(),
                "pt"
            );
        }
    }
}

#[test]
fn full_cli_embedded_invalid_utf8_is_protocol_error_and_valid_replacement_passes() {
    for (bytes, expected_exit) in [("bytes([239,191,189])", 0), ("bytes([255])", 2)] {
        let mut h = Host::new(true);
        let path = ".chrono-harness/bin/encoding-judge";
        let script = format!(
            "#!/usr/bin/python3\nimport json,sys\nr=json.load(sys.stdin)\ns={{'protocol':r['protocol'],'request_id':r['request_id'],'judge_id':r['judge_id'],'status':'pass','findings':[],'evidence':[],'outputs':{{'note':'MARKER'}}}}\nsys.stdout.buffer.write(json.dumps(s).encode().replace(b'MARKER',{bytes}))\n"
        );
        fs::write(h.root().join(path), &script).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(h.root().join(path), fs::Permissions::from_mode(0o755)).unwrap();
        h.values.get_mut(JUDGES).unwrap()["judges"] = json!([{"id":"encoding","executable":path,"version":"fixture","sha256":sha256(script.as_bytes()),"argv":[],"selector":"every-delta","after":[],"modes":["evaluate"]}]);
        h.values.get_mut(JUDGES).unwrap()["migration_validator"] = json!("encoding");
        h.save();
        let (code, report) = h.run(|_| {});
        assert_eq!(
            code, expected_exit,
            "full CLI must reject original invalid bytes"
        );
        let record = &report["judges"][0];
        let raw: Vec<u8> =
            serde_json::from_value(record["process"]["stdout_bytes"].clone()).unwrap();
        assert_eq!(std::str::from_utf8(&raw).is_ok(), expected_exit == 0);
        assert_eq!(record["process"]["exit_code"], 0);
        if expected_exit == 0 {
            assert_eq!(record["response"]["outputs"]["note"], "�");
        }
    }
}
