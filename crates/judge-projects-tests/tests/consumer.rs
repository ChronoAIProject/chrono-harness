#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::sha256;
use serde_json::{Value, json};
use std::{fs, process::Command};
use support::*;
#[path = "support/host.rs"]
mod host;
use host::Host;

// Original command bytes are retained separately; assertion messages must not
// embed nested reports and their byte arrays into the outer check response.
pub fn failure_summary(report: &Value) -> Value {
    fn field(value: &Value) -> String {
        let text = value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned);
        let mut chars = text.chars();
        let mut prefix: String = chars.by_ref().take(1024).collect();
        if chars.next().is_some() {
            prefix.push_str(" [truncated; original retained]");
        }
        prefix
    }
    fn process(value: &Value) -> Value {
        json!({"exit_code":field(&value["exit_code"]),
            "failure":field(&value["failure"]),"stderr":field(&value["stderr"])})
    }
    json!({
        "status":field(&report["status"]),
        "error":field(&report["error"]),
        "transport_failure":field(&report["transport_failure"]),
        "findings":field(&report["findings"]),
        "results":field(&report["response"]["results"]),
        "judge":process(&report["judge"]),
        "judges":report["judges"].as_array().into_iter().flatten().take(16)
            .map(|judge| json!({"id":field(&judge["id"]),
                "transport_failure":field(&judge["transport_failure"]),
                "process":process(&judge["process"])})).collect::<Vec<_>>(),
    })
}

#[test]
fn failure_diagnostics_keep_transport_cause_without_nested_output_amplification() {
    let report = json!({
        "status":"error",
        "transport_failure":"process timed out",
        "judges":[{"id":"registration","transport_failure":"process timed out",
            "process":{"exit_code":-1,"failure":"process timed out",
                "stdout":"x".repeat(1_000_000),"stdout_bytes":vec![255u8;1_000_000],
                "stderr":"λ".repeat(50_000)}}],
    });
    let summary = failure_summary(&report).to_string();
    assert!(summary.len() < 8192);
    assert!(summary.contains("process timed out"));
    assert!(summary.contains("registration"));
    assert!(summary.contains("original retained"));
    assert!(!summary.contains("stdout_bytes"));
    assert_eq!(report["judges"][0]["process"]["stdout_bytes"][999_999], 255);
}

fn check_pass(code: i32, v: &Value) {
    assert_eq!(code, 0, "{}", failure_summary(v));
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
    assert_eq!(code, 0, "{}", failure_summary(&v));
    assert_eq!(v["tests"]["executed"], json!([]));
    assert!(!h.root().join(".chrono-harness/state/order").exists());
}
#[test]
fn pair_edge_ownership_and_route_counterexamples_launch_nothing() {
    for mode in 0..5 {
        let mut h = Host::new(false);
        match mode {
            0 => h.values.get_mut(PROJECTS).unwrap()["projects"][0]["test_project"] = json!("p"),
            1 => h.values.get_mut(FM).unwrap()["project_edges"]
                .as_array_mut()
                .unwrap()
                .retain(|e| e["from"] != "project:p" || e["kind"] != "test-execution"),
            2 => {
                h.values.get_mut(FM).unwrap()["files"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|f| f["path"] == "p/product.py")
                    .unwrap()["owner"] = json!("t");
                h.values.get_mut(PROJECTS).unwrap()["projects"][0]["manifest"] =
                    json!("p/product.py");
            }
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
                2 => "E_TEST_PAIR",
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
    scoped_adapter(false, None);
}

#[test]
fn scoped_v2_git_binding_reuses_reader_through_full_registration_and_execution() {
    scoped_adapter(true, None);
}

#[test]
fn scoped_optional_direct_v3_registration_consumes_effective_policy() {
    scoped_adapter(true, Some(false));
}

#[test]
fn scoped_optional_selected_registration_consumes_effective_policy() {
    scoped_adapter(true, Some(true));
}

fn scoped_adapter(bound: bool, selected: Option<bool>) {
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
    let mut config = json!({"schema":"chrono-ci-check/v1","judge":{"program":".chrono-harness/bin/chrono-judge-ci","args":[],"timeout_seconds":30,"output_limit_bytes":1048576},"report_path":".chrono-harness/state/scoped.json","policy":{"filemap":FM,"projects":PROJECTS,"registration_config":CONFIG,"tools":{"python":h.tool},"artifacts":artifacts,"required_inputs":[profile],"operation_timeout_seconds":15,"operation_output_limit_bytes":4096,"environment":{}}});
    let trace = root.join(".chrono-harness/state/scoped-git.trace");
    let shadow = root.join(".chrono-harness/state/shadow");
    if bound {
        use std::os::unix::fs::PermissionsExt;
        let facts = ".chrono-harness/scoped-facts.json";
        let git = chrono_harness::resolve_program(&root, "git", None).unwrap();
        let version = Command::new(&git).arg("--version").output().unwrap();
        assert!(version.status.success());
        let chosen = root.join(".chrono-harness/bin/chosen git");
        let quoted = format!("'{}'", git.to_str().unwrap().replace('\'', "'\\''"));
        fs::write(&chosen, format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$SCOPED_TRACE\" || exit $?\nexec {quoted} \"$@\"\n")).unwrap();
        fs::set_permissions(&chosen, fs::Permissions::from_mode(0o755)).unwrap();
        fs::create_dir_all(&shadow).unwrap();
        fs::write(
            shadow.join("git"),
            "#!/bin/sh\necho unbound-registration-git >&2\nexit 83\n",
        )
        .unwrap();
        fs::set_permissions(shadow.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
        let facts_config = json!({"schema_version":3,
            "facts_git":{"tool":"git","input":"git-bytes"},
            "tools":[{"id":"git","program":chosen,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}],
            "environment":{"inherit":[],"values":{"SCOPED_TRACE":trace,"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"},
                "inputs":[{"id":"git-bytes","location":chosen,"presence":"present","sha256":chrono_harness::sha256(&fs::read(&chosen).unwrap())}]},
            "protocol":{"timeout_seconds":30,"stdout_limit_bytes":1048576}});
        h.values.insert(facts.into(), facts_config);
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(file(facts, json!([])));
        config["schema"] = json!("chrono-ci-check/v2");
        config["policy"]["facts_config"] = json!(facts);
        config["judge"]["timeout_seconds"] = json!(120);
        config["judge"]["output_limit_bytes"] = json!(8 * 1024 * 1024);
    }
    let target = ".chrono-harness/full scoped target.json";
    if let Some(selected) = selected {
        let facts = h.values[".chrono-harness/scoped-facts.json"].clone();
        let cfg = h.values.get_mut(CONFIG).unwrap();
        cfg["schema_version"] = json!(3);
        cfg["facts_git"] = facts["facts_git"].clone();
        cfg["input_closure"]["bindings"] = json!([]);
        cfg["tools"]
            .as_array_mut()
            .unwrap()
            .extend(facts["tools"].as_array().unwrap().iter().cloned());
        cfg["environment"]["inputs"].as_array_mut().unwrap().extend(
            facts["environment"]["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .cloned(),
        );
        cfg["environment"]["values"]["ONLY_FROM_TARGET"] = json!("selected environment");
        for input in cfg["environment"]["inputs"].as_array_mut().unwrap() {
            input["presence"] = json!("present");
        }
        // The full registration owns tool binding and environment for operations.
        config["policy"]["tools"]["python"] = json!("/missing/scoped-fallback");
        if selected {
            let cfg = h.values.remove(CONFIG).unwrap();
            h.values.insert(target.into(), cfg);
            h.values.insert(CONFIG.into(), json!({"schema":"chrono-git-configs/v1","platforms":{format!("{}-{}",std::env::consts::OS,std::env::consts::ARCH):target}}));
            h.values.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .push(file(target, json!([])));
        }
    }
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
    let call = |h: &Host, reordered: bool| {
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
        let mut command = Command::new(root.join(".chrono-harness/bin/chrono-harness"));
        if bound {
            command.env_clear().env("PATH", &shadow);
        }
        let output = command.current_dir("/").args(args).output().unwrap();
        h.retain_command_result(
            &format!("scoped-check-bound-{bound}-selected-{selected:?}-reordered-{reordered}"),
            &output,
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "scoped check returned invalid JSON: {error}; exit {}; stderr {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )
        });
        (output.status.code().unwrap(), report)
    };
    let (code, report) = call(&h, false);
    assert_eq!(code, 0, "{}", failure_summary(&report));
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
    if selected.is_some() {
        let process = &report["response"]["evidence"]["executed"][1]["receipt"]["process"];
        assert_eq!(
            process["environment"]["ONLY_FROM_TARGET"],
            "selected environment"
        );
        assert_eq!(process["environment"]["EMPTY"], "");
    }
    if bound {
        let facts = &report["response"]["evidence"]["git_facts"];
        let processes = facts["processes"].as_array().unwrap();
        let trace_bytes = fs::read_to_string(&trace).unwrap();
        assert_eq!(processes.len(), trace_bytes.lines().count());
        assert_eq!(
            trace_bytes
                .lines()
                .filter(|line| *line == "--version")
                .count(),
            1
        );
        assert!(trace_bytes.contains(CONFIG));
        for process in processes {
            for stream in ["stdout", "stderr"] {
                let bytes: Vec<u8> =
                    serde_json::from_value(process[format!("{stream}_bytes")].clone()).unwrap();
                assert_eq!(
                    process[format!("{stream}_sha256")],
                    json!(chrono_harness::sha256(&bytes))
                );
            }
        }
        fs::write(&trace, "").unwrap();
    }
    let (code, report) = call(&h, true);
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
    if bound {
        assert_eq!(
            report["response"]["evidence"]["git_facts"]["processes"]
                .as_array()
                .unwrap()
                .len(),
            fs::read_to_string(&trace).unwrap().lines().count()
        );
    }
    if let Some(selected) = selected {
        fs::remove_file(root.join(".chrono-harness/state/order")).unwrap();
        h.base = h.candidate.clone();
        fs::write(root.join("doc.txt"), "unrelated documentation\n").unwrap();
        h.candidate = commit(&root);
        let (code, report) = call(&h, false);
        assert_eq!(code, 0, "{}", failure_summary(&report));
        assert_eq!(report["response"]["evidence"]["executed"], json!([]));
        assert!(!root.join(".chrono-harness/state/order").exists());
        // Substituting the effective full target for the registered scoped entry
        // cannot authorize the same observed invocation.
        let effective = if selected { target } else { CONFIG };
        h.values.get_mut(effective).unwrap()["canonical_check"]["argv"][3] = json!(effective);
        h.save();
        let (code, report) = call(&h, false);
        assert_ne!(code, 0);
        assert!(
            report["response"]["results"]
                .to_string()
                .contains("canonical path differs"),
            "{}",
            report["response"]["results"]
        );
        assert!(!root.join(".chrono-harness/state/order").exists());
    }
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
            assert_eq!(code, 0, "{}", failure_summary(&v));
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
    assert_eq!(code, 0, "{}", failure_summary(&v));
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
        code,
        1,
        "changed effective input must execute consumer: {}",
        failure_summary(&v)
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
    assert_eq!(
        code,
        1,
        "absent differs from empty: {}",
        failure_summary(&v)
    );
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
        assert_eq!(code, 0, "{}", failure_summary(&v));
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

#[test]
fn manifest_free_projects_custom_operations_and_arbitrary_paths_execute_full_chain() {
    let mut h = Host::new(false);
    let root = h.root();
    for row in h.values.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
    {
        for field in ["manifest", "lockfile", "root"] {
            row.as_object_mut().unwrap().remove(field);
        }
    }
    let actions = h.values.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]
        .as_object_mut()
        .unwrap();
    let method = actions.remove("build").unwrap();
    actions.insert("hydrate-fixture".into(), method);
    h.values.get_mut(CONFIG).unwrap()["artifacts"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["owner"] != "p" && a["owner"] != "t");
    for (old, new) in [
        ("p/product.py", "different shelf/value.data"),
        ("t/check.py", "assays/contract.verify"),
    ] {
        fs::create_dir_all(root.join(new).parent().unwrap()).unwrap();
        fs::rename(root.join(old), root.join(new)).unwrap();
        for f in h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
        {
            if f["path"] == old {
                f["path"] = json!(new);
            }
        }
    }
    let test = fs::read_to_string(root.join("assays/contract.verify"))
        .unwrap()
        .replace("p/product.py", "different shelf/value.data");
    fs::write(root.join("assays/contract.verify"), test).unwrap();
    h.values.get_mut(PROJECTS).unwrap()["projects"][1]["actions"]["execute"]["argv"][1] =
        json!("assays/contract.verify");
    fs::write(root.join("p/Cargo.toml"), "opaque host bytes, not TOML").unwrap();
    h.save();
    let (code, report) = h.run(|_| {});
    check_pass(code, &report);
    assert_eq!(
        fs::read_to_string(root.join(".chrono-harness/state/order")).unwrap(),
        "pt"
    );
    assert_eq!(report["tests"]["selected"], json!(["test:t"]));
}

#[test]
fn opaque_legacy_paths_do_not_enable_language_or_directory_inference() {
    let mut h = Host::new(false);
    // A manifest-looking name does not opt the host into Cargo semantics.
    fs::write(h.root().join("p/Cargo.toml"), "host-owned opaque input\n").unwrap();
    fs::write(
        h.root().join("t/Cargo.toml"),
        "[workspace]\nmembers=['p']\n",
    )
    .unwrap();
    h.values.get_mut(PROJECTS).unwrap()["projects"][1]["root"] = json!("p");
    h.values.get_mut(CONFIG).unwrap()["artifacts"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["owner"] != "p" && a["owner"] != "t");
    h.save();
    let (code, v) = h.run(|_| {});
    check_pass(code, &v);
}

#[test]
fn overlapping_declared_outputs_fail_and_isolated_arbitrary_paths_pass() {
    for overlap in [false, true] {
        let mut h = Host::new(false);
        for row in h.values.get_mut(PROJECTS).unwrap()["projects"]
            .as_array_mut()
            .unwrap()
        {
            for key in ["manifest", "lockfile", "root"] {
                row.as_object_mut().unwrap().remove(key);
            }
        }
        for a in h.values.get_mut(CONFIG).unwrap()["artifacts"]
            .as_array_mut()
            .unwrap()
        {
            if a["owner"] == "p" {
                a["path"] = json!("output shelf/product/");
            }
            if a["owner"] == "t" {
                a["path"] = json!(if overlap {
                    "output shelf/product/test/"
                } else {
                    "assay output/"
                });
            }
        }
        h.save();
        let (code, v) = h.run(|_| {});
        if overlap {
            assert_ne!(code, 0);
            assert!(
                v.to_string().contains("overlapping registered outputs"),
                "{}",
                v["findings"]
            );
            assert!(!h.root().join(".chrono-harness/state/order").exists());
        } else {
            check_pass(code, &v);
        }
    }
}

#[test]
fn empty_action_name_and_duplicate_custom_operation_are_rejected() {
    for duplicate in [false, true] {
        let mut h = Host::new(false);
        let value = h.values[PROJECTS]["projects"][0]["actions"]["build"].clone();
        h.values.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]
            .as_object_mut()
            .unwrap()
            .insert(
                if duplicate {
                    "explicit-custom-operation"
                } else {
                    ""
                }
                .into(),
                value,
            );
        h.save();
        let (code, v) = h.run(|_| {});
        assert_ne!(code, 0);
        assert!(
            v.to_string().contains(if duplicate {
                "E_ROUTE_AMBIGUOUS"
            } else {
                "empty action name"
            }),
            "{}",
            v["findings"]
        );
        assert!(!h.root().join(".chrono-harness/state/order").exists());
    }
}

#[test]
fn output_registration_delta_alone_wakes_its_owner_and_checks_isolation() {
    let mut h = Host::new(false);
    h.base = h.candidate.clone();
    h.values.get_mut(CONFIG).unwrap()["artifacts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["owner"] == "t")
        .unwrap()["path"] = json!("p/target/test/");
    h.save();
    let (code, v) = h.run(|_| {});
    assert_ne!(
        code, 0,
        "an output-only overlap must not be classified as no work"
    );
    assert!(
        v.to_string().contains("overlapping registered outputs"),
        "{}",
        v["findings"]
    );
    assert!(!h.root().join(".chrono-harness/state/order").exists());
    h.values.get_mut(CONFIG).unwrap()["artifacts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["owner"] == "t")
        .unwrap()["path"] = json!("arbitrary assay outputs/");
    h.save();
    let (code, v) = h.run(|_| {});
    check_pass(code, &v);
}

#[test]
fn one_actual_cargo_test_project_runs_two_registered_groups_full_and_scoped() {
    let mut h = Host::new(false);
    let cargo = chrono_harness::resolve_program(&h.root(), "cargo", None).unwrap();
    let version = Command::new(&cargo).arg("--version").output().unwrap();
    assert!(version.status.success());
    let cargo_bytes = fs::read(&cargo).unwrap();
    let cargo_digest = sha256(&cargo_bytes);
    let cargo_blob = ".chrono-harness/state/cargo-original";
    fs::create_dir_all(h.root().join(".chrono-harness/state")).unwrap();
    fs::write(h.root().join(cargo_blob), &cargo_bytes).unwrap();
    h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"cargo-executable","location":cargo,"sha256":cargo_digest}));

    h.values.get_mut(CONFIG).unwrap()["tools"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"cargo", "program":cargo, "resolution":"PATH-once", "version_argv":["--version"],
            "expected_version":String::from_utf8(version.stdout).unwrap().trim_end()
        }));
    for key in ["PATH", "HOME", "RUSTUP_HOME", "CARGO_HOME"] {
        if let Ok(value) = std::env::var(key) {
            h.values.get_mut(CONFIG).unwrap()["environment"]["values"][key] = json!(value);
        }
    }
    h.values.get_mut(CONFIG).unwrap()["environment"]["values"]["RUSTUP_TOOLCHAIN"] =
        json!("1.95.0");
    let argv = json!(["test", "--locked", "--manifest-path", "t/Cargo.toml"]);
    h.values.get_mut(PROJECTS).unwrap()["projects"][1]["actions"] = json!({
        "execute":{"operation":"execute.t","tool":"cargo","argv":argv},
        "alpha":{"operation":"group.alpha","tool":"cargo","argv":["test","--locked","--manifest-path","t/Cargo.toml","alpha"]},
        "beta":{"operation":"group.beta","tool":"cargo","argv":["test","--locked","--manifest-path","t/Cargo.toml","beta"]}
    });
    h.values.get_mut(PROJECTS).unwrap()["projects"][1]["test_groups"] =
        json!({"t":"alpha","opaque":"beta"});
    h.values.get_mut(FM).unwrap()["execution_plans"] = json!({
        "test:t":{"operations":["group.alpha"],"timeout_seconds":30,"output_limit_bytes":65536},
        "test:opaque":{"operations":["group.beta"],"timeout_seconds":30,"output_limit_bytes":65536}
    });
    h.values.get_mut(FM).unwrap()["test_costs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"test":"opaque","cost":"unknown"}));
    for owner in ["p", "t"] {
        h.values.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .push(edge(
                &format!("project:{owner}"),
                "test-execution",
                "test:opaque",
            ));
    }
    for (path, owner, body) in [
        (
            "p/src/lib.rs",
            "p",
            "pub fn double(n: u32) -> u32 { n + n }\n",
        ),
        (
            "t/src/lib.rs",
            "t",
            "#[cfg(test)] mod tests { fn run(name: &str) { assert_eq!(p::double(21),42); let root=std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(); std::fs::write(root.join(\".chrono-harness/state\").join(name), name).unwrap(); } #[test] fn alpha() { run(\"alpha\"); } #[test] fn beta() { run(\"beta\"); } }\n",
        ),
    ] {
        let mut row = file(
            path,
            json!([{"kind":"compile","to":format!("project:{owner}")}]),
        );
        row["owner"] = json!(owner);
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(row);
        fs::create_dir_all(h.root().join(path).parent().unwrap()).unwrap();
        fs::write(h.root().join(path), body).unwrap();
    }
    let lock = Command::new(&cargo)
        .current_dir(h.root())
        .args(["generate-lockfile", "--manifest-path", "t/Cargo.toml"])
        .output()
        .unwrap();
    assert!(lock.status.success(), "{lock:?}");
    let scoped_path = ".chrono-harness/ci/group-check.json";
    let scoped_registration = ".chrono-harness/ci/group-registration.json";
    let mut registered = h.values[CONFIG].clone();
    registered["canonical_check"]["argv"] = json!([
        ".chrono-harness/bin/chrono-harness",
        "check",
        "--config",
        scoped_path,
        "--base",
        "{base}",
        "--candidate",
        "{candidate}"
    ]);
    h.values.insert(scoped_registration.into(), registered);
    h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(file(scoped_registration, json!([])));
    let profile = json!({"schema":"chrono-ci-check/v3", "judge":{"program":source().join("crates/judge-ci/target/debug/chrono-judge-ci"),"args":[],"timeout_seconds":120,"output_limit_bytes":8388608},
        "report_path":".chrono-harness/state/scoped.json", "policy":{
        "filemap":FM,"projects":PROJECTS,"registration_config":scoped_registration,
        "tools":{"cargo":cargo,"python":h.tool},
        "environment":h.values[CONFIG]["environment"]["values"],
        "artifacts":[".chrono-harness/state/",".chrono-harness/bin/","p/target/","t/target/"],
        "required_inputs":[],"adoption_base":null,"operation_timeout_seconds":30,"operation_output_limit_bytes":65536,
        "units":{"alpha":{"tests":["test:t"],"report_path":".chrono-harness/state/alpha.json"},"beta":{"tests":["test:opaque"],"report_path":".chrono-harness/state/beta.json"}},"shared_operations":{}}});
    h.values.insert(scoped_path.into(), profile);
    h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(file(scoped_path, json!([])));
    h.save();
    h.base = h.candidate.clone();
    fs::write(
        h.root().join("p/src/lib.rs"),
        "pub fn double(n:u32)->u32 { 2*n }\n",
    )
    .unwrap();
    h.candidate = commit(&h.root());
    let (code, full) = h.run(|inputs| {
        for endpoint in ["base", "candidate"] {
            inputs[endpoint]["files"]["cargo-executable"] =
                json!({"blob":cargo_blob,"sha256":cargo_digest,"length":cargo_bytes.len()});
        }
    });
    assert_eq!(code, 0, "{}", failure_summary(&full));
    assert_eq!(
        full["tests"]["tests"],
        json!({"test:t":"passed","test:opaque":"passed"})
    );
    assert_eq!(full["tests"]["executed"].as_array().unwrap().len(), 2);
    for (unit, own, other) in [
        ("alpha", "test:t", "beta"),
        ("beta", "test:opaque", "alpha"),
    ] {
        for name in ["alpha", "beta"] {
            fs::remove_file(h.root().join(".chrono-harness/state").join(name)).unwrap_or(());
        }
        let out = Command::new(h.root().join(".chrono-harness/bin/chrono-harness"))
            .current_dir(h.root())
            .args([
                "check",
                "--config",
                scoped_path,
                "--base",
                &h.base,
                "--candidate",
                &h.candidate,
                "--unit",
                unit,
            ])
            .output()
            .unwrap();
        h.retain_command_result(&format!("group-unit-{unit}"), &out);
        let report: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(out.status.success(), "{}", failure_summary(&report));
        assert_eq!(report["response"]["evidence"]["selected"], json!([own]));
        assert!(h.root().join(".chrono-harness/state").join(unit).is_file());
        assert!(!h.root().join(".chrono-harness/state").join(other).exists());
    }
    fs::write(
        h.root().join(".chrono-harness/state/missing.json"),
        r#"{"schema":"chrono-ci-collection/v1","reports":[]}"#,
    )
    .unwrap();
    let out = Command::new(h.root().join(".chrono-harness/bin/chrono-harness"))
        .current_dir(h.root())
        .args([
            "check",
            "--config",
            scoped_path,
            "--base",
            &h.base,
            "--candidate",
            &h.candidate,
            "--collect",
            ".chrono-harness/state/missing.json",
        ])
        .output()
        .unwrap();
    h.retain_command_result("group-missing-collection", &out);
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(!out.status.success(), "{}", failure_summary(&report));
    assert!(
        report
            .to_string()
            .contains("missing required CI unit report"),
        "{}",
        failure_summary(&report)
    );
}

#[test]
fn group_terminal_operation_and_producer_edge_are_required_before_effects() {
    for omit_edge in [false, true] {
        let mut h = Host::new(false);
        let terminal = h.values[PROJECTS]["projects"][1]["actions"]["execute"].clone();
        h.values.get_mut(PROJECTS).unwrap()["projects"][1]["actions"]["group"] = terminal;
        h.values.get_mut(PROJECTS).unwrap()["projects"][1]["actions"]["group"]["operation"] =
            json!("group.t");
        h.values.get_mut(PROJECTS).unwrap()["projects"][1]["test_groups"] = json!({"t":"group"});
        if omit_edge {
            h.values.get_mut(FM).unwrap()["execution_plans"]["test:t"]["operations"] =
                json!(["prepare.p", "group.t"]);
            h.values.get_mut(FM).unwrap()["project_edges"]
                .as_array_mut()
                .unwrap()
                .retain(|edge| edge["from"] != "project:p" || edge["kind"] != "test-execution");
        }
        h.save();
        let (exit, report) = h.run(|_| {});
        assert_ne!(exit, 0, "{}", failure_summary(&report));
        assert!(
            report.to_string().contains(if omit_edge {
                "E_TEST_PAIR"
            } else {
                "omits its execute action"
            }),
            "{}",
            failure_summary(&report)
        );
        assert!(!h.root().join(".chrono-harness/state/order").exists());
    }
}

#[test]
fn full_route_retains_registered_policy_and_rejects_incomplete_claims_before_effects() {
    for (script, priority) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut h = Host::new(script);
        let mut policy = json!({"max_running":2,"resources":["order"],"claims":{"prepare.p":{"resources":["order"],"outputs":[]},"execute.t":{"resources":["order"],"outputs":[]}}});
        if priority {
            policy["priority"] = json!(["execute.t", "prepare.p"]);
        }
        h.values.get_mut(FM).unwrap()["execution_scheduling"] = policy.clone();
        h.save();
        let (code, v) = h.run(|_| {});
        check_pass(code, &v);
        let route = v["judges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == "routes")
            .unwrap();
        let plan = &route["response"]["outputs"]["execution_plan"];
        assert_eq!(plan["scheduling"], policy);
        assert_eq!(v["tests"]["plan"], plan["identity"]);
        assert_eq!(
            fs::read_to_string(h.root().join(".chrono-harness/state/order")).unwrap(),
            "pt"
        );
        h.values.get_mut(FM).unwrap()["execution_scheduling"]["claims"]
            .as_object_mut()
            .unwrap()
            .remove("execute.t");
        h.save();
        let (code, v) = h.run(|_| {});
        assert_ne!(code, 0);
        assert!(serde_json::to_string(&v).unwrap().contains("E_SCHEDULING"));
        assert_eq!(
            fs::read_to_string(h.root().join(".chrono-harness/state/order")).unwrap(),
            "pt"
        );
    }
}
