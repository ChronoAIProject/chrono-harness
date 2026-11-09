use serde_json::{Value, json};
use std::{fs, process::Command};
use tempfile::TempDir;

const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn snapshot() -> Value {
    serde_json::from_str(include_str!("fixtures/snapshot.json")).unwrap()
}
fn api(value: Value) -> Value {
    json!({"prefix":["api","graphql"],"stdout":value.to_string()})
}
fn watch(exit: i32) -> Value {
    json!({"prefix":["pr","checks"],"stdout":"native wait result\n","exit":exit})
}
fn pending() -> Value {
    let mut s = snapshot();
    s["data"]["repository"]["object"]["statusCheckRollup"]["contexts"]["nodes"][0]["status"] =
        json!("IN_PROGRESS");
    s["data"]["repository"]["object"]["statusCheckRollup"]["contexts"]["nodes"][0]["conclusion"] =
        Value::Null;
    s
}

struct Host {
    root: TempDir,
}
impl Host {
    fn new(steps: Vec<Value>) -> Self {
        let host = Self {
            root: tempfile::tempdir().unwrap(),
        };
        fs::create_dir(host.root.path().join(".chrono-harness")).unwrap();
        fs::write(host.root.path().join(".chrono-harness/retention.json"), json!({
            "schema":"chrono-output-retention/v2", "producers":{"pr-original":[".chrono-harness/state/pr-operations/"]},
            "keep_seconds":0,"diagnostic_keep_seconds":86400,"max_related_outputs":32,"max_summaries":32,
            "max_nodes_per_round":256,"max_bytes_per_round":1048576,"max_millis_per_round":1000
        }).to_string()).unwrap();
        host.write_config(json!({"schema":"chrono-pr-config/v2","repository":"example/host","base":"dev",
            "gh":env!("CARGO_BIN_EXE_chrono-pr-test-gh"),"required_checks":["chrono / collection"],
            "api_timeout_seconds":5,"watch_timeout_seconds":5,"watch_interval_seconds":60,"output_limit_bytes":1048576,"state_directory":".chrono-harness/state/pr-operations","state_producer":"pr-original"}));
        fs::write(
            host.root.path().join("steps.json"),
            json!(steps).to_string(),
        )
        .unwrap();
        fs::write(
            host.root.path().join("body.md"),
            "literal `echo no` $(touch nope)\n中文\n",
        )
        .unwrap();
        host
    }
    fn write_config(&self, value: Value) {
        fs::write(
            self.root.path().join(".chrono-harness/pr.json"),
            value.to_string(),
        )
        .unwrap();
    }
    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_chrono-pr-test-cli"))
            .args(args)
            .arg("--host-root")
            .arg(self.root.path())
            .current_dir("/")
            .env("GH_TOKEN", "fixture-secret-never-retain")
            .output()
            .unwrap();
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["schema"], "chrono-pr-result/v2");
        (output.status.code().unwrap(), result)
    }
    fn status(&self) -> (i32, Value) {
        self.run(&["pr-status", "--pr", "42", "--head-sha", HEAD])
    }
    fn calls(&self) -> Vec<Value> {
        fs::read_to_string(self.root.path().join("calls.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }
}

#[test]
fn status_passes_only_for_the_bound_head_and_required_success() {
    let h = Host::new(vec![api(snapshot())]);
    let (code, r) = h.status();
    assert_eq!(code, 0);
    assert_eq!(r["outcome"], "passed");
    assert_eq!(r["expected_head"], HEAD);
    assert_eq!(r["observed_head"], HEAD);
    assert_eq!(r["checks"][0]["state"], "SUCCESS");
    assert_eq!(h.calls().len(), 1);
    assert_eq!(r["receipts"][0]["exit_code"], 0);
    assert!(r["receipts"][0].get("environment").is_none());
}

#[test]
fn failure_skipped_and_missing_checks_cannot_pass() {
    for state in [
        "FAILURE",
        "CANCELLED",
        "TIMED_OUT",
        "ACTION_REQUIRED",
        "STARTUP_FAILURE",
        "STALE",
        "NEUTRAL",
        "SKIPPED",
    ] {
        let mut s = snapshot();
        s["data"]["repository"]["object"]["statusCheckRollup"]["contexts"]["nodes"][0]["conclusion"] =
            json!(state);
        let (code, r) = Host::new(vec![api(s)]).status();
        assert_eq!(code, 1, "{state}: {r}");
        assert_eq!(r["outcome"], "failed");
    }
    let mut s = snapshot();
    s["data"]["repository"]["object"]["statusCheckRollup"] = Value::Null;
    let (code, r) = Host::new(vec![api(s)]).status();
    assert_eq!(code, 3);
    assert_eq!(r["checks"][0]["state"], "MISSING");
}

#[test]
fn stale_and_closed_are_distinct_from_check_failure() {
    let mut s = snapshot();
    s["data"]["repository"]["pullRequest"]["headRefOid"] = json!(OTHER);
    let (code, r) = Host::new(vec![api(s)]).status();
    assert_eq!(code, 4);
    assert_eq!(r["outcome"], "stale");
    let mut s = snapshot();
    s["data"]["repository"]["pullRequest"]["state"] = json!("CLOSED");
    let (code, r) = Host::new(vec![api(s)]).status();
    assert_eq!(code, 1);
    assert_eq!(r["outcome"], "closed");
}

#[test]
fn rejects_incomplete_or_ambiguous_remote_facts() {
    for pointer in [
        "/data/repository/nameWithOwner",
        "/data/repository/pullRequest/number",
        "/data/repository/object/oid",
        "/data/repository/pullRequest/baseRefName",
        "/data/repository/pullRequest/headRepository/nameWithOwner",
    ] {
        let mut s = snapshot();
        *s.pointer_mut(pointer).unwrap() = json!("wrong");
        let (code, _) = Host::new(vec![api(s)]).status();
        assert_eq!(code, 2, "{pointer}");
    }
    for pointer in [
        "/data/repository/object/statusCheckRollup/contexts/pageInfo/hasNextPage",
        "/data/repository/object/statusCheckRollup/contexts/nodes/0/checkSuite/checkRuns/pageInfo/hasNextPage",
    ] {
        let mut s = snapshot();
        *s.pointer_mut(pointer).unwrap() = json!(true);
        assert_eq!(Host::new(vec![api(s)]).status().0, 2);
    }
    let mut s = snapshot();
    let nodes = s["data"]["repository"]["object"]["statusCheckRollup"]["contexts"]["nodes"]
        .as_array_mut()
        .unwrap();
    nodes.push(nodes[0].clone());
    assert_eq!(Host::new(vec![api(s)]).status().0, 2);
    let mut s = snapshot();
    s["errors"] = json!([{"message":"partial result"}]);
    assert_eq!(Host::new(vec![api(s)]).status().0, 2);
}

#[test]
fn latest_rerun_and_legacy_status_are_supported() {
    let mut s = snapshot();
    let nodes = s["data"]["repository"]["object"]["statusCheckRollup"]["contexts"]["nodes"]
        .as_array_mut()
        .unwrap();
    let mut old = nodes[0].clone();
    old["databaseId"] = json!(99);
    old["conclusion"] = json!("FAILURE");
    nodes.push(old);
    assert_eq!(Host::new(vec![api(s)]).status().0, 0);
    let mut s = snapshot();
    s["data"]["repository"]["object"]["statusCheckRollup"]["contexts"]["nodes"] = json!([
        {"__typename":"StatusContext","context":"chrono / collection","state":"SUCCESS","commit":{"oid":HEAD}}
    ]);
    assert_eq!(Host::new(vec![api(s)]).status().0, 0);
}

#[test]
fn unknown_states_wrong_commits_and_incomplete_latest_sets_are_errors() {
    for (pointer, value) in [
        (
            "/data/repository/object/statusCheckRollup/contexts/nodes/0/conclusion",
            json!("FUTURE_STATE"),
        ),
        (
            "/data/repository/object/statusCheckRollup/contexts/nodes/0/conclusion",
            json!("PENDING"),
        ),
        (
            "/data/repository/object/statusCheckRollup/contexts/nodes/0/checkSuite/commit/oid",
            json!(OTHER),
        ),
        (
            "/data/repository/object/statusCheckRollup/contexts/nodes/0/checkSuite/checkRuns/nodes",
            json!([{"databaseId":100},{"databaseId":101}]),
        ),
        (
            "/data/repository/object/statusCheckRollup/contexts/nodes/0/status",
            json!("UNKNOWN"),
        ),
    ] {
        let mut s = snapshot();
        *s.pointer_mut(pointer).unwrap() = value;
        assert_eq!(Host::new(vec![api(s)]).status().0, 2, "{pointer}");
    }
}

#[test]
fn draft_creation_is_explicit_and_readback_failure_preserves_the_pr_number() {
    let mut draft = snapshot();
    draft["data"]["repository"]["pullRequest"]["isDraft"] = json!(true);
    let h = Host::new(vec![
        resolve(),
        list(json!([])),
        json!({"prefix":["pr","create"],"stdout":"https://github.com/example/host/pull/42\n"}),
        api(draft),
    ]);
    let mut args = open_args();
    args.push("--draft");
    assert_eq!(h.run(&args).0, 0);
    assert!(
        h.calls()[2]["args"]
            .as_array()
            .unwrap()
            .contains(&json!("--draft"))
    );
    let h = Host::new(vec![
        resolve(),
        list(json!([])),
        json!({"prefix":["pr","create"],"stdout":"https://github.com/example/host/pull/42\n"}),
        json!({"prefix":["api","graphql"],"stderr":"HTTP 502","exit":1}),
    ]);
    let (code, result) = h.run(&open_args());
    assert_eq!(code, 2);
    assert_eq!(result["pr"], 42);
    assert_eq!(result["outcome"], "unknown");
}

#[test]
fn native_auth_failure_and_failed_readback_remain_unknown() {
    let h = Host::new(vec![api(pending()), watch(4)]);
    let (code, r) = h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]);
    assert_eq!(code, 2);
    assert_eq!(r["outcome"], "unknown");
    assert_eq!(h.calls().len(), 2);
    let h = Host::new(vec![
        api(pending()),
        watch(0),
        json!({"prefix":["api","graphql"],"stdout":"bad"}),
    ]);
    assert_eq!(h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]).0, 2);
}

#[test]
fn watch_joins_one_native_wait_and_rechecks_identity() {
    let h = Host::new(vec![api(pending()), watch(0), api(snapshot())]);
    let (code, r) = h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]);
    assert_eq!(code, 0);
    assert_eq!(r["outcome"], "passed");
    let calls = h.calls();
    assert_eq!(calls.len(), 3);
    assert!(
        calls[1]["args"]
            .as_array()
            .unwrap()
            .contains(&json!("--watch"))
    );
    assert!(
        calls[1]["args"]
            .as_array()
            .unwrap()
            .windows(2)
            .any(|w| w == [json!("--interval"), json!("60")])
    );
    let mut changed = snapshot();
    changed["data"]["repository"]["pullRequest"]["headRefOid"] = json!(OTHER);
    let h = Host::new(vec![api(pending()), watch(0), api(changed)]);
    assert_eq!(h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]).0, 4);
}

#[test]
fn native_exit_alone_never_certifies_success_and_terminal_needs_no_wait() {
    let h = Host::new(vec![api(pending()), watch(0), api(pending())]);
    let (code, r) = h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]);
    assert_eq!(code, 3);
    assert_eq!(r["outcome"], "pending");
    let h = Host::new(vec![api(snapshot())]);
    assert_eq!(h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]).0, 0);
    assert_eq!(h.calls().len(), 1);
    let h = Host::new(vec![api(pending()), watch(1), api(snapshot())]);
    assert_eq!(h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]).0, 0);
}

#[test]
fn api_failure_retains_original_output_without_retry() {
    let h = Host::new(vec![
        json!({"prefix":["api","graphql"],"stdout":"partial\n","stderr":"HTTP 403\n","exit":1}),
    ]);
    let (code, r) = h.status();
    assert_eq!(code, 2);
    assert_eq!(r["outcome"], "unknown");
    assert_eq!(h.calls().len(), 1);
    assert_eq!(
        fs::read(
            h.root.path().join(
                r["receipts"][0]["stderr_original"]["path"]
                    .as_str()
                    .unwrap()
            )
        )
        .unwrap(),
        b"HTTP 403\n"
    );
    let h = Host::new(vec![json!({"prefix":["api","graphql"],"stdout":"{broken"})]);
    assert_eq!(h.status().0, 2);
}

fn resolve() -> Value {
    api(
        json!({"data":{"repository":{"nameWithOwner":"example/host","ref":{"target":{"oid":HEAD}}}}}),
    )
}
fn list(items: Value) -> Value {
    json!({"prefix":["pr","list"],"stdout":items.to_string()})
}
fn open_args() -> Vec<&'static str> {
    vec![
        "pr-open",
        "--head",
        "integration/change",
        "--head-sha",
        HEAD,
        "--title",
        "literal $(no) `no` 中文",
        "--body-file",
        "body.md",
    ]
}

#[test]
fn opens_once_with_literal_text_and_verifies_the_created_pr() {
    let h = Host::new(vec![
        resolve(),
        list(json!([])),
        json!({"prefix":["pr","create"],"stdin":"literal `echo no` $(touch nope)\n中文\n","stdout":"https://github.com/example/host/pull/42\n"}),
        api(snapshot()),
    ]);
    let (code, r) = h.run(&open_args());
    assert_eq!(code, 0);
    assert_eq!(r["outcome"], "opened");
    assert_eq!(r["pr"], 42);
    let calls = h.calls();
    assert_eq!(calls.len(), 4);
    let args = calls[2]["args"].as_array().unwrap();
    assert!(
        args.windows(2)
            .any(|w| w == [json!("--title"), json!("literal $(no) `no` 中文")])
    );
    assert!(
        args.windows(2)
            .any(|w| w == [json!("--body-file"), json!("-")])
    );
    assert!(!h.root.path().join("nope").exists());
}

#[test]
fn rerunning_open_reuses_existing_pr_without_mutation() {
    let h = Host::new(vec![
        resolve(),
        list(json!([{"number":42}])),
        api(snapshot()),
    ]);
    let (code, r) = h.run(&open_args());
    assert_eq!(code, 0);
    assert_eq!(r["outcome"], "existing");
    assert_eq!(h.calls().len(), 3);
    let mut wrong = resolve();
    wrong["stdout"]=json!(json!({"data":{"repository":{"nameWithOwner":"example/host","ref":{"target":{"oid":OTHER}}}}}).to_string());
    let h = Host::new(vec![wrong]);
    assert_eq!(h.run(&open_args()).0, 4);
    assert_eq!(h.calls().len(), 1);
}

#[test]
fn ambiguous_creation_failures_and_changed_readback_keep_identity() {
    let h = Host::new(vec![
        resolve(),
        list(json!([])),
        json!({"prefix":["pr","create"],"stderr":"connection reset after POST","exit":1}),
    ]);
    let (code, r) = h.run(&open_args());
    assert_eq!(code, 2);
    assert_eq!(r["outcome"], "unknown");
    assert_eq!(r["expected_head"], HEAD);
    assert_eq!(h.calls().len(), 3);
    let mut s = snapshot();
    s["data"]["repository"]["pullRequest"]["headRefName"] = json!("wrong");
    let h = Host::new(vec![resolve(), list(json!([{"number":42}])), api(s)]);
    assert_eq!(h.run(&open_args()).0, 2);
    let h = Host::new(vec![resolve(), list(json!([{"number":42},{"number":43}]))]);
    assert_eq!(h.run(&open_args()).0, 2);
}

#[test]
fn invalid_arguments_and_config_do_no_remote_work() {
    for args in [
        vec!["pr-status", "--pr", "0", "--head-sha", HEAD],
        vec!["pr-status", "--pr", "42", "--head-sha", "bad"],
        vec!["pr-status", "--pr", "42", "--head-sha", HEAD, "--unknown"],
        vec!["pr-status", "--pr", "42", "--pr", "43", "--head-sha", HEAD],
        vec!["pr-open", "--head", "--base", "--head-sha", HEAD],
        vec!["pr-watch", "--pr", "42", "--head-sha", HEAD, "--draft"],
    ] {
        let h = Host::new(vec![]);
        assert_eq!(h.run(&args).0, 2);
        assert!(h.calls().is_empty());
    }
    for (key, value) in [
        ("required_checks", json!([])),
        ("api_timeout_seconds", json!(0)),
        ("watch_interval_seconds", json!(1)),
        ("unexpected", json!(true)),
    ] {
        let h = Host::new(vec![]);
        let mut c: Value = serde_json::from_slice(
            &fs::read(h.root.path().join(".chrono-harness/pr.json")).unwrap(),
        )
        .unwrap();
        c[key] = value;
        h.write_config(c);
        assert_eq!(h.status().0, 2, "{key}");
        assert!(h.calls().is_empty());
    }
}

#[test]
fn timeout_and_output_limits_preserve_the_original_local_receipt() {
    let h = Host::new(vec![
        api(pending()),
        json!({"prefix":["pr","checks"],"stdout":"waiting\n","sleep_ms":1800}),
    ]);
    let mut c: Value =
        serde_json::from_slice(&fs::read(h.root.path().join(".chrono-harness/pr.json")).unwrap())
            .unwrap();
    c["watch_timeout_seconds"] = json!(1);
    h.write_config(c);
    let (code, r) = h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]);
    assert_eq!(code, 3);
    assert_eq!(r["outcome"], "pending");
    assert_eq!(h.calls().len(), 2);
    assert!(r["receipts"][1]["failure"].is_string());
    let h = Host::new(vec![
        json!({"prefix":["api","graphql"],"stdout":"x".repeat(100000)}),
    ]);
    let mut c: Value =
        serde_json::from_slice(&fs::read(h.root.path().join(".chrono-harness/pr.json")).unwrap())
            .unwrap();
    c["output_limit_bytes"] = json!(4096);
    h.write_config(c);
    assert_eq!(h.status().0, 2);
}

#[test]
fn completed_command_keeps_durable_intent_originals_and_open_object_state() {
    let h = Host::new(vec![api(snapshot())]);
    let (code, result) = h.status();
    assert_eq!(code, 0, "{result}");
    let operation = result["operation"]
        .as_str()
        .expect("durable operation identity");
    let directory = h.root.path().join(operation);
    let intent: Value =
        serde_json::from_slice(&fs::read(directory.join("intent.json")).unwrap()).unwrap();
    assert_eq!(intent["command"], "pr-status");
    assert_eq!(intent["object"]["repository"], "example/host");
    assert_eq!(intent["object"]["pr"], 42);
    assert_eq!(intent["object"]["expected_head"], HEAD);
    let saved: Value =
        serde_json::from_slice(&fs::read(directory.join("result.json")).unwrap()).unwrap();
    assert_eq!(saved, result);
    assert_eq!(saved["operation_state"], "completed");
    assert_eq!(saved["pr_state"], "OPEN");
    assert_eq!(saved["merge_commit"], Value::Null);
    let call: Value =
        serde_json::from_slice(&fs::read(directory.join("call-1.intent.json")).unwrap()).unwrap();
    assert_eq!(call["result"], Value::Null);
    assert_eq!(call["handle"], Value::Null);
    assert_eq!(call["state"], "intent");
    assert!(call.get("environment").is_none());
    let receipt = &saved["receipts"][0];
    let original = &receipt["stdout_original"];
    assert_eq!(
        fs::read(h.root.path().join(original["path"].as_str().unwrap())).unwrap(),
        snapshot().to_string().as_bytes()
    );
    assert!(receipt.get("stdout_bytes").is_none());
    for file in [
        "intent.json",
        "config.json",
        "call-1.intent.json",
        "call-1.result.json",
        "result.json",
    ] {
        let bytes = fs::read(directory.join(file)).unwrap();
        assert!(
            !bytes
                .windows(b"fixture-secret-never-retain".len())
                .any(|w| w == b"fixture-secret-never-retain")
        );
    }
}

#[test]
fn absent_or_unregistered_state_never_dispatches() {
    for failure in [
        "missing-policy",
        "unregistered-producer",
        "old-config",
        "blocked-state",
    ] {
        let h = Host::new(vec![api(snapshot())]);
        match failure {
            "missing-policy" => {
                fs::remove_file(h.root.path().join(".chrono-harness/retention.json")).unwrap()
            }
            "blocked-state" => {
                fs::write(h.root.path().join(".chrono-harness/state"), b"block").unwrap()
            }
            _ => {
                let mut config: Value = serde_json::from_slice(
                    &fs::read(h.root.path().join(".chrono-harness/pr.json")).unwrap(),
                )
                .unwrap();
                if failure == "old-config" {
                    config["schema"] = json!("chrono-pr-config/v1");
                } else {
                    config["state_producer"] = json!("unregistered");
                }
                h.write_config(config);
            }
        }
        let (code, result) = h.status();
        assert_eq!(code, 2, "{failure}: {result}");
        assert!(
            h.calls().is_empty(),
            "{failure}: dispatched despite missing durable state"
        );
    }
}

#[test]
fn final_publication_failure_keeps_original_process_but_cannot_pass() {
    let mut step = api(snapshot());
    step["block_result"] = json!(true);
    let h = Host::new(vec![step]);
    let (code, result) = h.status();
    assert_eq!(code, 2, "{result}");
    assert_eq!(result["outcome"], "unknown");
    assert_eq!(result["operation_state"], "unknown");
    assert!(result["storage_error"].is_string());
    assert_eq!(h.calls().len(), 1);
    assert_eq!(result["receipts"][0]["exit_code"], 0);
    let path = result["operation"].as_str().unwrap();
    let receipt: Value = serde_json::from_slice(
        &fs::read(h.root.path().join(path).join("call-1.result.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt, result["receipts"][0]);
    assert_eq!(
        fs::read(
            h.root
                .path()
                .join(receipt["stdout_original"]["path"].as_str().unwrap())
        )
        .unwrap(),
        snapshot().to_string().as_bytes()
    );
}

#[test]
fn merged_state_requires_and_retains_the_actual_merge_identity() {
    let mut merged = snapshot();
    merged["data"]["repository"]["pullRequest"]["state"] = json!("MERGED");
    merged["data"]["repository"]["pullRequest"]["mergeCommit"] = json!({"oid":OTHER});
    let h = Host::new(vec![api(merged.clone())]);
    let (code, result) = h.status();
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["pr_state"], "MERGED");
    assert_eq!(result["merge_commit"], OTHER);
    for value in [Value::Null, json!({"oid":"invalid"})] {
        merged["data"]["repository"]["pullRequest"]["mergeCommit"] = value;
        assert_eq!(Host::new(vec![api(merged.clone())]).status().0, 2);
    }
}

#[test]
fn stream_publication_failure_returns_the_actual_unsaved_bytes() {
    let mut step = api(snapshot());
    step["block_receipt"] = json!(true);
    let h = Host::new(vec![step]);
    let (code, result) = h.status();
    assert_eq!(code, 2, "{result}");
    assert_eq!(result["outcome"], "unknown");
    assert!(result["receipts"][0]["storage_error"].is_string());
    assert_eq!(result["receipts"][0]["exit_code"], 0);
    assert_eq!(
        result["receipts"][0]["stdout_bytes"],
        json!(snapshot().to_string().into_bytes())
    );
    assert!(result["receipts"][0].get("stdout_original").is_none());
    let saved: Value = serde_json::from_slice(
        &fs::read(
            h.root
                .path()
                .join(result["operation"].as_str().unwrap())
                .join("result.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(saved, result);
}
