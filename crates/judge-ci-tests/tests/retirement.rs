#[path = "../../runner-tests/tests/support/isolated_capability.rs"]
mod chrono_git_boundary;
use super::*;

const REGISTRATION: &str = ".chrono-harness/config.json";
const WORKFLOW: &str = ".chrono-harness/workflow.json";
const PROJECTS: &str = ".chrono-harness/projects.json";
const FILEMAP: &str = ".chrono-harness/FILEMAP.json";
const JUDGES: &str = ".chrono-harness/judges.json";

fn host() -> Host {
    let h = Host::new();
    h.change_registry(PROJECTS, |v| {
        v["status"] = json!("active");
        v["projects"][0]["kind"] = json!("production");
        v["projects"][0]["test_project"] = json!("suite");
        v["projects"][0]["actions"] = json!({"build":{
            "operation":"build.lib","tool":"fixture","argv":["production"]
        }});
        v["projects"][1]["kind"] = json!("test");
        v["projects"][1]["tests_for"] = json!("lib");
    });
    h.change_registry(FILEMAP, |v| {
        v["status"] = json!("active");
        v["cost_models"] = json!({"unmeasured":{
            "cpu_ms":null,"wall_ms":null,"peak_rss_bytes":null,"io_bytes":null,
            "basis":"unknown fixture cost"
        }});
        v["test_costs"] = json!([{"test":"suite","cost":"unmeasured"}]);
        for path in [REGISTRATION, WORKFLOW, JUDGES] {
            v["files"].as_array_mut().unwrap().push(json!({
                "path":path,"owner":"host","surface":"product","cost":"unmeasured","edges":[]
            }));
        }
    });
    h.json(REGISTRATION, &json!({
        "schema_version":1,"status":"active","enforcement":"enabled",
        "runner":{"path":".chrono-harness/bin/chrono-harness","version":"0.1.0","sha256":null},
        "registries":{"filemap":FILEMAP,"projects":PROJECTS,"judges":JUDGES,"workflow":WORKFLOW},
        "canonical_check":{"operation":"validate.delta","argv":[OPERATION_TOOL,"check","--base","{base}","--candidate","{candidate}"]},
        "tools":[{"id":"fixture","program":OPERATION_TOOL,"resolution":"PATH-once","version_argv":["--version"],"expected_version":"chrono-test-ci-diagnostics/v1"}],
        "environment":{"inherit":["PATH"],"values":{},"inputs":[]},
        "input_closure":{"status":"declared-complete","unresolved":[]},
        "protocol":{"id":"chrono-judge/v1","timeout_seconds":30,"stdout_limit_bytes":8388608,"encoding":"UTF-8"},
        "semantic_fields":[],
        "artifacts":[
            {"path":".chrono-harness/bin/","owner":"host","kind":"executable","tracked":false},
            {"path":".chrono-harness/state/","owner":"host","kind":"evidence","tracked":false}
        ]
    }));
    h.json(JUDGES, &json!({
        "schema_version":1,"status":"active","migration_validator":"registration",
        "judges":[{"id":"registration","executable":".chrono-harness/bin/chrono-judge-registration","version":"0.1.0","sha256":null,"argv":["--protocol","chrono-judge/v1"],"selector":"every-delta","after":[],"modes":["evaluate"]}]
    }));
    h.json(WORKFLOW, &json!({
        "schema_version":1,"status":"active","target_branch":"dev","feature_prefix":"feature/","integration_prefix":"integration/",
        "staleness":{"max_behind_commits":3,"max_age_hours":24,"combine":"any-exceeded"},
        "stability":[],"semantic_changes_require_integration":true,
        "mixed_change":{"level":"warning","code":"W_MIXED","requires_acknowledgement":false},
        "integration":{"tests":[],"bind":[],"evidence":".chrono-harness/state/integration.json"},
        "retirements":[],"migrations":[]
    }));
    h.change_registry(CONFIG, |v| {
        v["policy"]["registration_config"] = json!(REGISTRATION);
        v["policy"].as_object_mut().unwrap().remove("tools");
        v["policy"]["bindings"]["test:suite"] = json!(["build.lib", "test.suite"]);
    });
    h.commit();
    h
}

fn check(h: &Host, base: &str, candidate: &str) -> Response {
    let mut request = h.request(Some(base), candidate);
    request.observations = json!({"entry":{
        "argv":[OPERATION_TOOL,"check","--base",base,"--candidate",candidate],
        "cwd":h.root()
    }});
    judge(&request)
}

#[test]
fn registered_profile_removes_a_stale_duplicate_tool_table() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = host();
    h.change_registry(CONFIG, |v| {
        v["policy"]["tools"]["fixture"] = json!("unused-stale-program");
    });
    let base = h.commit();
    h.change_registry(CONFIG, |v| {
        v["policy"].as_object_mut().unwrap().remove("tools");
    });
    h.write("src.txt", "changed\n");
    let candidate = h.commit();
    pass(&check(&h, &base, &candidate));
    assert_eq!(h.calls(), 1, "the registered tool must execute the test");
}

#[test]
fn registered_tool_change_reexecutes_its_binding_without_a_profile_copy_change() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = host();
    let base = h.head();
    let tool = Path::new(OPERATION_TOOL);
    let alternate = format!(
        "{}/./{}",
        tool.parent().unwrap().display(),
        tool.file_name().unwrap().to_str().unwrap()
    );
    h.change_registry(REGISTRATION, |v| {
        v["tools"][0]["program"] = json!(alternate);
    });
    let candidate = h.commit();
    let result = check(&h, &base, &candidate);
    pass(&result);
    assert_eq!(
        h.calls(),
        1,
        "changing the actual tool must select its test"
    );
}

#[test]
fn registered_action_cannot_fall_back_to_a_duplicate_tool_table() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = host();
    h.change_registry(CONFIG, |v| {
        v["policy"]["tools"] = json!({"fixture": OPERATION_TOOL});
    });
    let base = h.commit();
    h.change_registry(REGISTRATION, |v| v["tools"] = json!([]));
    h.write("src.txt", "changed\n");
    let candidate = h.commit();
    let result = check(&h, &base, &candidate);
    assert_eq!(result.status, Status::Failed);
    assert_eq!(
        h.calls(),
        0,
        "missing registered tools must block execution"
    );
}

fn remove_pair(h: &Host, remove_producer: bool) {
    h.change_registry(PROJECTS, |v| {
        if remove_producer {
            v["projects"] = json!([]);
        } else {
            v["projects"].as_array_mut().unwrap().remove(1);
        }
    });
    h.change_registry(FILEMAP, |v| {
        v["project_edges"] = json!([]);
        v["test_costs"] = json!([]);
        for file in v["files"].as_array_mut().unwrap() {
            file["edges"] = json!([]);
        }
    });
    h.change_registry(CONFIG, |v| v["policy"]["bindings"] = json!({}));
    h.change_registry(WORKFLOW, |v| {
        v["retirements"] = json!([
            {"kind":"test","id":"suite","replacement":null,"reason":"product and test retired"},
            {"kind":"project","id":"lib","replacement":null,"reason":"product retired"}
        ]);
    });
}

#[test]
fn joint_pair_removal_keeps_old_costs_without_executing_old_commands() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = host();
    let base = h.head();
    remove_pair(&h, true);
    let candidate = h.commit();
    let result = check(&h, &base, &candidate);
    pass(&result);
    assert_eq!(h.calls(), 0);
    assert_eq!(result.evidence["removed"], json!({"test:suite":null}));
    assert_eq!(
        result.evidence["retirement_context"]["test:suite"]["cost"]["reference"],
        "unmeasured"
    );
    assert_eq!(
        result.evidence["retirement_context"]["test:suite"]["definitions"][0]["identity"],
        "project:suite"
    );
    assert_eq!(result.evidence["operations"], json!([]));
}

#[test]
fn null_retirement_with_surviving_producer_fails_before_business_effects() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = host();
    let base = h.head();
    remove_pair(&h, false);
    let candidate = h.commit();
    fail(
        &check(&h, &base, &candidate),
        "joint producer/test removal required",
    );
    assert_eq!(h.calls(), 0);
}

fn replace(h: &Host) {
    h.change_registry(PROJECTS, |v| {
        v["owners"].as_array_mut().unwrap().push(json!("current"));
        v["projects"][0]["test_project"] = json!("current");
        v["projects"][1]["id"] = json!("current");
        v["projects"][1]["actions"]["execute"]["operation"] = json!("test.current");
    });
    h.change_registry(FILEMAP, |v| {
        v["test_costs"] = json!([{"test":"current","cost":"unmeasured"}]);
        for edge in v["project_edges"].as_array_mut().unwrap() {
            edge["to"] = json!("test:current");
        }
        for file in v["files"].as_array_mut().unwrap() {
            for edge in file["edges"].as_array_mut().unwrap() {
                if edge["to"] == "test:suite" {
                    edge["to"] = json!("test:current");
                }
            }
        }
    });
    h.change_registry(CONFIG, |v| {
        v["policy"]["bindings"] = json!({"test:current":["build.lib","test.current"]});
    });
    h.change_registry(WORKFLOW, |v| {
        v["retirements"] = json!([
            {"kind":"test","id":"suite","replacement":"current","reason":"current test owns the changed contract"}
        ]);
    });
}

#[test]
fn replacement_executes_current_method_and_retains_removed_identity() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = host();
    let base = h.head();
    replace(&h);
    let candidate = h.commit();
    let result = check(&h, &base, &candidate);
    pass(&result);
    assert_eq!(h.calls(), 1);
    assert_eq!(
        result.evidence["removed"],
        json!({"test:suite":"test:current"})
    );
    assert_eq!(
        result.evidence["operations"],
        json!(["build.lib", "test.current"])
    );
}

#[test]
fn a_declared_replacement_that_fails_cannot_approve_retirement() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = host();
    let base = h.head();
    replace(&h);
    h.change_registry(PROJECTS, |v| {
        v["projects"][1]["actions"]["execute"]["argv"] = json!(["fail-seven"]);
    });
    let candidate = h.commit();
    let result = check(&h, &base, &candidate);
    fail(&result, "required replacement did not execute successfully");
    assert!(
        result
            .results
            .iter()
            .any(|r| r.id == "replacement:test:suite" && r.status == Status::Failed)
    );
}
