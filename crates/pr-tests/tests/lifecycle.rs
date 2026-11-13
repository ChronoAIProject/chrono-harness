use serde_json::{Value, json};
use std::{fs, io::Write, process::Command};
use tempfile::TempDir;

#[path = "../../runner-tests/tests/support/isolated_capability.rs"]
mod isolated_capability;
use chrono_harness::retained_artifacts::retention::Inventory;
use isolated_capability::isolated_capability_test;

const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
#[cfg(unix)]
fn cli_read_only_stdout_returns_native_publication_failure() {
    let expected = chrono_pr::run(&["--help".into()]);
    assert_eq!(expected.exit_code, 0);
    assert!(!expected.stdout.is_empty());
    assert!(expected.stderr.is_empty());
    let mut root = tempfile::tempdir().unwrap();
    root.disable_cleanup(true);
    eprintln!(
        "CLI publication failure originals: {}",
        root.path().display()
    );
    let binary = env!("CARGO_BIN_EXE_chrono-pr-test-cli");
    let healthy = Command::new(binary)
        .arg("--help")
        .current_dir(root.path())
        .output()
        .unwrap();
    assert_eq!(healthy.status.code(), Some(0));
    assert_eq!(healthy.stdout, expected.stdout.as_bytes());
    assert!(healthy.stderr.is_empty());
    let descriptor = root.path().join("read-only-output");
    let original = b"caller supplied a read-only output descriptor\n";
    fs::write(&descriptor, original).unwrap();
    let actual = Command::new(binary)
        .arg("--help")
        .current_dir(root.path())
        .stdout(fs::File::open(&descriptor).unwrap())
        .output()
        .unwrap();
    fs::write(root.path().join("stderr.bin"), &actual.stderr).unwrap();
    fs::write(
        root.path().join("observation.json"),
        serde_json::to_vec(&json!({
            "argv":[binary,"--help"],
            "binary_sha256":chrono_harness::file_identity(std::path::Path::new(binary)).unwrap().0,
            "stdout_descriptor":"open O_RDONLY regular file",
            "stderr_descriptor":"writable capture pipe",
            "expected_exit":2,"actual_exit":actual.status.code(),
            "expected_stdout_bytes":expected.stdout.len(),
            "captured_stdout_bytes":actual.stdout.len(),
            "native_error":libc::EBADF
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(actual.status.code(), Some(2));
    assert!(actual.stdout.is_empty());
    assert_eq!(fs::read(descriptor).unwrap(), original);
    assert!(actual.stderr.ends_with(b"\n"));
    assert_eq!(
        std::str::from_utf8(&actual.stderr).unwrap().lines().count(),
        1
    );
    let event: Value = serde_json::from_slice(&actual.stderr).unwrap();
    assert_eq!(event["schema"], "chrono-log/v1");
    assert_eq!(event["event"], "runner.output_publication_failed");
    let error = &event["error"]["error"];
    assert_eq!(error["code"], "E_OUTPUT_PUBLISH");
    assert_eq!(error["context"]["stream"], "stdout");
    assert_eq!(error["context"]["phase"], "write");
    assert_eq!(error["context"]["operation_exit_code"], 0);
    assert_eq!(error["context"]["output_bytes"], expected.stdout.len());
    assert_eq!(error["context"]["published_bytes"], 0);
    assert_eq!(error["source"]["source"]["native_code"], libc::EBADF);
    root.disable_cleanup(false);
}

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
impl Drop for Host {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.root.disable_cleanup(true);
            let _ = writeln!(
                std::io::stderr(),
                "PR test fixture retained for failure diagnosis and regression acceptance: {}",
                self.root.path().display()
            );
        }
    }
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
fn adopted_runtime_logging_preserves_query_wait_and_api_failure_results() {
    for (command, steps, exit, outcome, calls) in [
        ("pr-status", vec![api(snapshot())], 0, "passed", 1),
        (
            "pr-watch",
            vec![api(pending()), watch(1), api(snapshot())],
            0,
            "passed",
            3,
        ),
        (
            "pr-status",
            vec![
                json!({"prefix":["api","graphql"],"stderr":"fixture authentication failed\n","exit":1}),
            ],
            2,
            "unknown",
            1,
        ),
    ] {
        let h = Host::new(steps);
        fs::write(
            h.root.path().join(".chrono-harness/logging.json"),
            json!({
                "schema":"chrono-runtime-logging/v1","level":"debug",
                "directory":".chrono-harness/state/logs/","max_bytes":1048576,
                "max_files":2,"stderr":false,
            })
            .to_string(),
        )
        .unwrap();
        let (code, result) = h.run(&[command, "--pr", "42", "--head-sha", HEAD]);
        assert_eq!(code, exit, "{result}");
        assert_eq!(result["outcome"], outcome);
        assert_eq!(h.calls().len(), calls);
        let mut events = Vec::new();
        for entry in fs::read_dir(h.root.path().join(".chrono-harness/state/logs")).unwrap() {
            let path = entry.unwrap().path();
            if path
                .extension()
                .is_some_and(|extension| extension == "jsonl")
            {
                let bytes = fs::read(path).unwrap();
                assert!(bytes.len() <= 1048576);
                let text = std::str::from_utf8(&bytes).unwrap();
                assert!(!text.contains("fixture-secret-never-retain"));
                events.extend(
                    text.lines()
                        .map(|line| serde_json::from_str::<Value>(line).unwrap()),
                );
            }
        }
        assert!(events.iter().all(|event| event["producer"] == "chrono-pr"));
        assert!(events.iter().any(|event| event["event"] == "cli.started"));
        assert!(
            events
                .iter()
                .any(|event| event["event"] == "process.preparing")
        );
        assert!(
            events
                .iter()
                .any(|event| event["event"] == "process.completed")
        );
        assert!(
            events
                .iter()
                .any(|event| event["event"] == "cli.finished"
                    && event["fields"]["exit_code"] == exit)
        );
    }
}

#[cfg(unix)]
#[test]
fn unavailable_child_exit_keeps_the_original_operation_unknown() {
    if !isolated_capability_test("unavailable_child_exit_keeps_the_original_operation_unknown") {
        return;
    }
    let h = Host::new(vec![api(snapshot()), api(snapshot())]);
    // This process alone relinquishes native child statuses to the kernel.
    // The real gh fixture still runs and publishes its actual response, but
    // neither those bytes nor a kernel exit observation supply Child::wait.
    let previous = unsafe { libc::signal(libc::SIGCHLD, libc::SIG_IGN) };
    assert_ne!(previous, libc::SIG_ERR);
    let output = chrono_pr::run(&[
        "pr-status".into(),
        "--pr".into(),
        "42".into(),
        "--head-sha".into(),
        HEAD.into(),
        "--host-root".into(),
        h.root.path().as_os_str().to_owned(),
    ]);
    assert_ne!(
        unsafe { libc::signal(libc::SIGCHLD, previous) },
        libc::SIG_ERR
    );
    let result: Value = serde_json::from_str(&output.stdout).unwrap();
    assert_eq!(output.exit_code, 2, "{result}");
    assert_eq!(h.calls().len(), 1, "the original gh actually ran");
    assert_eq!(result["receipts"].as_array().unwrap().len(), 1);
    assert_eq!(result["receipts"][0]["exit_code"], -1, "{result}");
    assert!(result["receipts"][0]["exit_signal"].is_null());
    assert!(result["receipts"][0]["failure"].is_string());
    assert_eq!(
        result["receipts"][0]["owned_work_joined"], false,
        "{result}"
    );
    assert_eq!(result["operation_state"], "unknown", "{result}");
    let path = result["operation"].as_str().unwrap();
    let saved = fs::read(h.root.path().join(path).join("result.json")).unwrap();
    assert_eq!(h.status().0, 0, "independent work can continue");
    maintain_after_diagnostics(&h);
    assert_eq!(
        fs::read(h.root.path().join(path).join("result.json")).unwrap(),
        saved,
        "a newer PR observation cannot establish this original exit"
    );
    let (code, continued) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        path,
    ]);
    assert_eq!(code, 2, "{continued}");
    assert_eq!(
        h.calls().len(),
        2,
        "unknown continuation never redispatches"
    );
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
    let (code, result) = h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]);
    assert_eq!(code, 2, "{result}");
}

#[test]
fn watch_joins_one_native_wait_and_rechecks_identity() {
    let h = Host::new(vec![api(pending()), watch(0), api(snapshot())]);
    let (code, r) = h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]);
    assert_eq!(code, 0, "{r}");
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
    for initial_timeout in [false, true] {
        let mut initial = api(pending());
        if initial_timeout {
            initial["sleep_ms"] = json!(1800);
        }
        let h = Host::new(vec![
            initial,
            json!({"prefix":["pr","checks"],"stdout":"waiting\n","sleep_ms":1800}),
        ]);
        let mut c: Value = serde_json::from_slice(
            &fs::read(h.root.path().join(".chrono-harness/pr.json")).unwrap(),
        )
        .unwrap();
        c["watch_timeout_seconds"] = json!(1);
        h.write_config(c);
        let (code, r) = h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]);
        let receipts = r["receipts"].as_array().unwrap();
        let first = &receipts[0];
        assert_eq!(r["expected_head"], HEAD, "{r}");
        assert_eq!(
            &first["argv"].as_array().unwrap()[1..3],
            &[json!("api"), json!("graphql")],
            "{r}"
        );
        // The same deadline also bounds the initial query. Without its real
        // successful exit, even response bytes cannot establish a pending PR.
        if initial_timeout || first["failure"] == "process timed out" {
            assert_eq!(code, 2, "{r}");
            assert_eq!(r["outcome"], "unknown", "{r}");
            assert_eq!(r["observed_head"], Value::Null, "{r}");
            assert_eq!(r["error"], "process timed out", "{r}");
            assert_eq!(receipts.len(), 1, "{r}");
            assert_eq!(first["failure"], "process timed out", "{r}");
            assert!(first["child_pid"].is_u64(), "{r}");
            continue;
        }
        assert_eq!(code, 3, "{r}");
        assert_eq!(r["outcome"], "pending", "{r}");
        assert_eq!(r["observed_head"], HEAD, "{r}");
        assert_eq!(first["exit_code"], 0, "{r}");
        assert_eq!(first["failure"], Value::Null, "{r}");
        // The producer's receipts count actual dispatches; an interrupted
        // fixture need not have finished publishing its own call log.
        match receipts.len() {
            1 => assert_eq!(
                r["error"], "operation deadline reached; remote result remains unverified",
                "{r}"
            ),
            2 => {
                assert_eq!(
                    &receipts[1]["argv"].as_array().unwrap()[1..3],
                    &[json!("pr"), json!("checks")],
                    "{r}"
                );
                assert_eq!(receipts[1]["failure"], "process timed out", "{r}");
                assert!(receipts[1]["child_pid"].is_u64(), "{r}");
            }
            count => panic!("unexpected dispatch count {count}: {r}"),
        }
    }
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
fn receipt_publication_failure_returns_the_actual_unsaved_receipt_bytes() {
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

#[test]
fn continues_a_known_operation_and_keeps_command_and_object_facts_separate() {
    let mut merged = snapshot();
    merged["data"]["repository"]["pullRequest"]["state"] = json!("MERGED");
    merged["data"]["repository"]["pullRequest"]["mergeCommit"] = json!({"oid":OTHER});
    let h = Host::new(vec![api(snapshot()), api(merged)]);
    let (first_code, first) = h.status();
    assert_eq!(first_code, 0, "{first}");
    assert_eq!(first["pr_state"], "OPEN");
    let previous = first["operation"].as_str().unwrap();
    let old_result = h.root.path().join(previous).join("result.json");
    let original = fs::read(&old_result).unwrap();
    let (code, continued) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        previous,
    ]);
    assert_eq!(code, 0, "{continued}");
    assert_eq!(continued["continued_operation"], previous);
    assert_eq!(continued["pr_state"], "MERGED");
    assert_eq!(continued["merge_commit"], OTHER);
    assert_ne!(continued["operation"], first["operation"]);
    assert_eq!(
        fs::read(&old_result).unwrap(),
        original,
        "new object state must not rewrite an old command result"
    );
    assert_eq!(
        h.calls().len(),
        2,
        "continuation uses one readback and never creates a PR or starts a second watcher"
    );
}

fn maintain_after_diagnostics(h: &Host) -> Value {
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 100000;
    let mut result = Value::Null;
    for _ in 0..8 {
        result = Inventory::adopted(h.root.path())
            .unwrap()
            .unwrap()
            .maintain_at(future)
            .unwrap();
    }
    result
}

#[test]
fn continuation_releases_only_its_prior_record_after_a_durable_handoff() {
    if !isolated_capability_test(
        "continuation_releases_only_its_prior_record_after_a_durable_handoff",
    ) {
        return;
    }
    let mut merged = snapshot();
    merged["data"]["repository"]["pullRequest"]["state"] = json!("MERGED");
    merged["data"]["repository"]["pullRequest"]["mergeCommit"] = json!({"oid":OTHER});
    let h = Host::new(vec![
        api(snapshot()),
        api(snapshot()),
        api(snapshot()),
        api(merged),
    ]);
    let (code, prior) = h.status();
    assert_eq!(code, 0, "{prior}");
    let (code, independent) = h.status();
    assert_eq!(code, 0, "{independent}");
    let owner = Inventory::adopted(h.root.path()).unwrap().unwrap();
    owner
        .reference_paths(
            "actual-independent-review",
            &[independent["operation"].as_str().unwrap().into()],
        )
        .unwrap();
    let previous = prior["operation"].as_str().unwrap();
    let (code, current) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        previous,
    ]);
    assert_eq!(code, 0, "{current}");
    assert_eq!(current["pr_state"], "OPEN");
    maintain_after_diagnostics(&h);
    assert!(
        !h.root.path().join(previous).exists(),
        "explicitly superseded local command can expire"
    );
    for receipt in prior["receipts"].as_array().unwrap() {
        for field in ["stdout_original", "stderr_original", "launch_original"] {
            assert!(
                !h.root
                    .path()
                    .join(receipt[field]["path"].as_str().unwrap())
                    .exists(),
                "superseded process original must expire: {field}"
            );
        }
    }
    for receipt in independent["receipts"].as_array().unwrap() {
        for field in ["stdout_original", "stderr_original", "launch_original"] {
            assert!(
                h.root
                    .path()
                    .join(receipt[field]["path"].as_str().unwrap())
                    .exists(),
                "independent process original remains protected: {field}"
            );
        }
    }
    assert!(
        h.root
            .path()
            .join(independent["operation"].as_str().unwrap())
            .exists(),
        "another caller's root remains protected"
    );
    let current_path = current["operation"].as_str().unwrap();
    assert!(
        h.root
            .path()
            .join(current_path)
            .join("result.json")
            .exists(),
        "current OPEN object remains recoverable"
    );
    let (code, absent) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        previous,
    ]);
    assert_eq!(code, 2, "{absent}");
    assert!(absent["error"].is_string());
    assert_eq!(
        h.calls().len(),
        3,
        "unavailable history never creates a new remote task"
    );
    let (code, terminal) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        current_path,
    ]);
    assert_eq!(code, 0, "{terminal}");
    assert_eq!(terminal["merge_commit"], OTHER);
    let maintenance = maintain_after_diagnostics(&h);
    assert!(
        !h.root.path().join(current_path).exists(),
        "terminal operation remains after maintenance: {maintenance}"
    );
    assert!(
        !h.root
            .path()
            .join(terminal["operation"].as_str().unwrap())
            .exists()
    );
    assert!(
        h.root
            .path()
            .join(independent["operation"].as_str().unwrap())
            .exists()
    );
    assert_eq!(h.calls().len(), 4);
    owner
        .reference_paths("actual-independent-review", &[])
        .unwrap();
    maintain_after_diagnostics(&h);
    assert!(
        !h.root
            .path()
            .join(independent["operation"].as_str().unwrap())
            .exists()
    );
}

#[test]
fn continuation_rejects_missing_changed_or_mismatched_originals_before_dispatch() {
    for case in ["pr", "head", "config", "missing", "changed", "unregistered"] {
        let h = Host::new(vec![api(snapshot())]);
        let (code, prior) = h.status();
        assert_eq!(code, 0, "{prior}");
        let previous = prior["operation"].as_str().unwrap();
        let result_path = h.root.path().join(previous).join("result.json");
        match case {
            "config" => {
                let p = h.root.path().join(".chrono-harness/pr.json");
                let mut c: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
                c["base"] = json!("main");
                h.write_config(c);
            }
            "missing" => fs::remove_file(&result_path).unwrap(),
            "changed" => fs::write(&result_path, prior.to_string() + " ").unwrap(),
            "unregistered" => {
                let copy = h
                    .root
                    .path()
                    .join(".chrono-harness/state/pr-operations/operation-unregistered");
                fs::create_dir(&copy).unwrap();
                for name in ["config.json", "intent.json", "result.json"] {
                    fs::copy(h.root.path().join(previous).join(name), copy.join(name)).unwrap();
                }
            }
            _ => (),
        }
        let (code, result) = h.run(&[
            "pr-status",
            "--pr",
            if case == "pr" { "43" } else { "42" },
            "--head-sha",
            if case == "head" { OTHER } else { HEAD },
            "--continue-operation",
            if case == "unregistered" {
                ".chrono-harness/state/pr-operations/operation-unregistered"
            } else {
                previous
            },
        ]);
        assert_eq!(code, 2, "{case}: {result}");
        assert!(result["error"].is_string(), "{case}: {result}");
        assert_eq!(h.calls().len(), 1, "{case} must reject before dispatch");
        assert!(h.root.path().join(previous).exists());
    }
}

#[test]
fn continuation_requires_joined_process_work_in_each_sealed_receipt_before_dispatch() {
    for joined in [
        Some(json!(true)),
        Some(json!(false)),
        None,
        Some(json!("true")),
    ] {
        let h = Host::new(vec![api(snapshot()), api(snapshot())]);
        let (code, mut prior) = h.status();
        assert_eq!(code, 0, "{prior}");
        let source = prior["operation"].as_str().unwrap().to_owned();
        let path = ".chrono-harness/state/pr-operations/operation-completion-fixture";
        let publication = Inventory::adopted(h.root.path())
            .unwrap()
            .unwrap()
            .plan("pr-original", path, true, json!({"state":"intent"}))
            .unwrap();
        publication
            .materialize_for_reference("fixture-completion-originals")
            .unwrap();
        // Publish the complete current contract through the real sealing owner.
        // A digest failure must not stand in for the completion decision.
        let write = |name: &str, bytes: &[u8]| {
            publication
                .write_file(&h.root.path().join(path).join(name), bytes)
                .unwrap();
        };
        for (index, receipt) in prior["receipts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            match &joined {
                Some(value) => receipt["owned_work_joined"] = value.clone(),
                None => {
                    receipt.as_object_mut().unwrap().remove("owned_work_joined");
                }
            }
            for field in ["stdout_original", "stderr_original", "launch_original"] {
                let original = receipt[field]["path"].as_str().unwrap();
                let member = original.strip_prefix(&format!("{source}/")).unwrap();
                write(member, &fs::read(h.root.path().join(original)).unwrap());
                receipt[field]["path"] = json!(format!("{path}/{member}"));
            }
            write(
                &format!("call-{}.result.json", index + 1),
                &serde_json::to_vec(receipt).unwrap(),
            );
        }
        let mut intent: Value = serde_json::from_slice(
            &fs::read(h.root.path().join(&source).join("intent.json")).unwrap(),
        )
        .unwrap();
        intent["operation"] = json!(path);
        write("intent.json", &serde_json::to_vec(&intent).unwrap());
        write(
            "config.json",
            &fs::read(h.root.path().join(".chrono-harness/pr.json")).unwrap(),
        );
        write(
            "object-state.json",
            &serde_json::to_vec(&json!({
                "schema":"chrono-pr-object-state/v1", "request":null,
                "observed":null, "unresolved_create":false
            }))
            .unwrap(),
        );
        prior["operation"] = json!(path);
        let original_result = serde_json::to_vec(&prior).unwrap();
        write("result.json", &original_result);
        publication.complete_sealed(json!({"exit":0})).unwrap();
        let (code, result) = h.run(&[
            "pr-status",
            "--pr",
            "42",
            "--head-sha",
            HEAD,
            "--continue-operation",
            path,
        ]);
        if joined == Some(json!(true)) {
            assert_eq!(code, 0, "{result}");
            assert_eq!(h.calls().len(), 2, "one current readback");
        } else {
            assert_eq!(code, 2, "joined={joined:?}: {result}");
            assert!(
                result["error"]
                    .as_str()
                    .unwrap()
                    .contains("continued process completion is not established"),
                "{result}"
            );
            assert_eq!(h.calls().len(), 1, "reject before another remote call");
        }
        assert_eq!(
            fs::read(h.root.path().join(path).join("result.json")).unwrap(),
            original_result
        );
    }
}

#[test]
fn failed_continuation_separates_auth_publication_and_actual_closed_object() {
    if !isolated_capability_test(
        "failed_continuation_separates_auth_publication_and_actual_closed_object",
    ) {
        return;
    }
    let mut stale = snapshot();
    stale["data"]["repository"]["pullRequest"]["headRefOid"] = json!(OTHER);
    stale["data"]["repository"]["pullRequest"]["state"] = json!("CLOSED");
    for (step, expected_exit, keep_prior, keep_current) in [
        (
            json!({"prefix":["api","graphql"],"stdout":"offline","exit":2}),
            2,
            true,
            false,
        ),
        (
            json!({"prefix":["api","graphql"],"stdout":snapshot().to_string(),"block_result":true}),
            2,
            true,
            true,
        ),
        (api(stale), 4, false, false),
    ] {
        let h = Host::new(vec![api(snapshot()), step]);
        let (code, prior) = h.status();
        assert_eq!(code, 0, "{prior}");
        let previous = prior["operation"].as_str().unwrap();
        let original = fs::read(h.root.path().join(previous).join("result.json")).unwrap();
        let (code, result) = h.run(&[
            "pr-status",
            "--pr",
            "42",
            "--head-sha",
            HEAD,
            "--continue-operation",
            previous,
        ]);
        assert_eq!(code, expected_exit, "{result}");
        assert_eq!(
            fs::read(h.root.path().join(previous).join("result.json")).unwrap(),
            original
        );
        maintain_after_diagnostics(&h);
        assert_eq!(
            h.root.path().join(previous).exists(),
            keep_prior,
            "{result}"
        );
        if keep_prior {
            assert_eq!(
                fs::read(h.root.path().join(previous).join("result.json")).unwrap(),
                original
            );
        }
        assert_eq!(
            h.root
                .path()
                .join(result["operation"].as_str().unwrap())
                .exists(),
            keep_current,
            "{result}"
        );
        assert_eq!(h.calls().len(), 2);
    }
}

#[test]
fn continuation_rejects_a_live_producer_even_when_result_bytes_exist() {
    if !isolated_capability_test(
        "continuation_rejects_a_live_producer_even_when_result_bytes_exist",
    ) {
        return;
    }
    let h = Host::new(vec![]);
    let path = ".chrono-harness/state/pr-operations/operation-live";
    fs::create_dir_all(h.root.path().join(".chrono-harness/state/pr-operations")).unwrap();
    let inventory = Inventory::adopted(h.root.path()).unwrap().unwrap();
    let publication = inventory
        .plan("pr-original", path, true, json!({"state":"intent"}))
        .unwrap();
    publication
        .materialize_for_reference("live-original-task")
        .unwrap();
    publication
        .write_file(&h.root.path().join(path).join("result.json"), b"{}")
        .unwrap();
    let (code, result) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        path,
    ]);
    assert_eq!(code, 2, "{result}");
    assert!(result["error"].is_string());
    assert!(h.calls().is_empty());
    assert!(h.root.path().join(path).exists());
    drop(publication);
}

#[test]
fn returned_pr_receipt_binds_durable_launch_without_retaining_credentials() {
    let h = Host::new(vec![api(snapshot())]);
    let (code, result) = h.status();
    assert_eq!(code, 0, "{result}");
    let receipt = &result["receipts"][0];
    assert_eq!(receipt["owned_work_joined"], true, "{receipt}");
    let operation = result["operation"].as_str().unwrap();
    let inventory = Inventory::adopted(h.root.path()).unwrap().unwrap();
    for field in ["stdout_original", "stderr_original", "launch_original"] {
        let original = &receipt[field];
        let path = original["path"].as_str().unwrap();
        assert!(path.starts_with(&format!("{operation}/")), "{original}");
        assert!(
            inventory
                .published_tree_member(
                    "pr-original",
                    operation,
                    path,
                    original["sha256"].as_str().unwrap(),
                    original["length"].as_u64().unwrap(),
                )
                .unwrap()
        );
    }
    let original = &receipt["launch_original"];
    let path = original["path"]
        .as_str()
        .expect("PR receipt must reference the actual persisted launch, not only its returned PID");
    let bytes = fs::read(h.root.path().join(path)).unwrap();
    assert_eq!(chrono_harness::sha256(&bytes), original["sha256"]);
    assert_eq!(bytes.len() as u64, original["length"].as_u64().unwrap());
    let launch: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(launch["schema"], "chrono-process-launch/v2");
    assert_eq!(launch["child_pid"], receipt["child_pid"]);
    assert_eq!(launch["launcher_pid"], receipt["launcher_pid"]);
    assert_eq!(launch["argv"], receipt["argv"]);
    assert_eq!(launch["sha256"], receipt["sha256"]);
    assert!(
        launch.get("environment").is_none(),
        "launch originals must not persist credentials"
    );
    assert!(
        !bytes
            .windows(b"fixture-secret-never-retain".len())
            .any(|w| w == b"fixture-secret-never-retain")
    );
    assert!(launch["environment_digest"].is_string());
    assert_eq!(h.calls().len(), 1);
}

#[test]
fn unregistered_process_originals_refuse_dispatch() {
    let h = Host::new(vec![api(snapshot())]);
    let path = h.root.path().join(".chrono-harness/retention.json");
    let mut policy: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    policy["producers"]
        .as_object_mut()
        .unwrap()
        .remove("pr-original");
    fs::write(path, policy.to_string()).unwrap();
    let (code, result) = h.status();
    assert_eq!(code, 2, "{result}");
    assert!(
        result["error"]
            .as_str()
            .unwrap()
            .contains("PR state producer")
    );
    assert!(h.calls().is_empty());
}

#[cfg(unix)]
#[test]
fn enclosing_interruption_preserves_live_launch_partial_streams_and_unknown_operation() {
    if !isolated_capability_test(
        "enclosing_interruption_preserves_live_launch_partial_streams_and_unknown_operation",
    ) {
        return;
    }
    use std::{
        io::{Read, Write},
        os::unix::net::{UnixListener, UnixStream},
        time::Duration,
    };
    let mut step = api(snapshot());
    // More than the supported platforms' stdout pipe capacity: reaching the
    // handshake requires the actual runner reader to drain some original bytes.
    step["stdout"] = json!("x".repeat(2 * 1024 * 1024));
    step["stderr"] = json!("original partial diagnostic\n");
    step["wait_for_interruption"] = json!(true);
    let h = Host::new(vec![step, api(snapshot())]);
    let config_path = h.root.path().join(".chrono-harness/pr.json");
    let mut config: Value = serde_json::from_slice(&fs::read(&config_path).unwrap()).unwrap();
    config["output_limit_bytes"] = json!(4 * 1024 * 1024);
    h.write_config(config);
    let listener = UnixListener::bind(h.root.path().join("ready.sock")).unwrap();
    let cli = env!("CARGO_BIN_EXE_chrono-pr-test-cli");
    let spec = chrono_harness::CommandSpec {
        program: cli.into(),
        args: vec![
            "pr-status".into(),
            "--pr".into(),
            "42".into(),
            "--head-sha".into(),
            HEAD.into(),
            "--host-root".into(),
            h.root.path().to_str().unwrap().into(),
        ],
        env: [
            ("GH_TOKEN".into(), "fixture-secret-never-retain".into()),
            // Completion must join the original result even when its diagnostic
            // representation exceeds a native notification socket's capacity.
            (
                "CHRONO_TEST_INTERRUPTION_INPUT".into(),
                "x".repeat(64 * 1024),
            ),
        ]
        .into_iter()
        .collect(),
        timeout_seconds: 30,
        output_limit_bytes: 1048576,
    };
    let digest = chrono_harness::file_identity(std::path::Path::new(cli))
        .unwrap()
        .0;
    let (operation, originals) = std::thread::scope(|scope| {
        let root = h.root.path();
        let task = scope.spawn(|| {
            let result = chrono_harness::run_process_observed_for(
                root,
                &spec,
                &[],
                &digest,
                Duration::from_secs(30),
            );
            // Wake the same native channel if launch fails before the fixture
            // handshake. Keep the notification bounded: after a real handshake,
            // this pending connection has no reader while the caller joins us.
            // The original result is transferred only by that join.
            if let Ok(mut ready) = UnixStream::connect(root.join("ready.sock")) {
                let bytes = br#"{"completed_before_handshake":true}"#;
                let _ = ready.write_all(&(bytes.len() as u32).to_be_bytes());
                let _ = ready.write_all(bytes);
            }
            result
        });
        let (mut ready, _) = listener.accept().unwrap();
        let mut length = [0; 4];
        ready.read_exact(&mut length).unwrap();
        let mut bytes = vec![0; u32::from_be_bytes(length) as usize];
        ready.read_exact(&mut bytes).unwrap();
        let message: Value = serde_json::from_slice(&bytes).unwrap();
        if message.get("completed_before_handshake").is_some() {
            panic!(
                "process completed before handshake: {:?}",
                task.join().unwrap()
            );
        }
        let operation = message["operation"].as_str().unwrap().to_owned();
        let call: Value = serde_json::from_slice(
            &fs::read(root.join(&operation).join("call-1.intent.json")).unwrap(),
        )
        .unwrap();
        let originals = call["process_originals"].clone();
        let launch_bytes = fs::read(root.join(originals["launch"].as_str().unwrap())).unwrap();
        let launch: Value = serde_json::from_slice(&launch_bytes).unwrap();
        assert_eq!(launch["schema"], "chrono-process-launch/v2");
        assert!(launch.get("environment").is_none());
        assert!(
            !root.join(&operation).join("result.json").exists(),
            "launch is live evidence, not a terminal result"
        );
        assert!(!root.join(&operation).join("call-1.result.json").exists());
        let partial = fs::read(root.join(originals["stdout"].as_str().unwrap())).unwrap();
        assert!(!partial.is_empty());
        assert!(partial.iter().all(|byte| *byte == b'x'));
        let launcher = launch["launcher_pid"].as_u64().unwrap() as i32;
        assert_eq!(unsafe { libc::kill(launcher, libc::SIGKILL) }, 0);
        let joined = task.join().unwrap().unwrap();
        assert_eq!(joined.child_pid, Some(launcher as u32));
        assert_eq!(joined.exit_signal, Some(libc::SIGKILL));
        (operation, originals)
    });
    // A successful independent observation cannot complete this interrupted task.
    let (code, current) = h.status();
    assert_eq!(code, 0, "{current}");
    assert_eq!(current["pr_state"], "OPEN");
    // Caller has joined its enclosing owned process. A later query still cannot
    // manufacture the missing original PR terminal receipt or release its root.
    maintain_after_diagnostics(&h);
    assert!(!h.root.path().join(&operation).join("result.json").exists());
    for field in ["stdout", "stderr", "launch"] {
        assert!(
            h.root
                .path()
                .join(originals[field].as_str().unwrap())
                .exists(),
            "unknown original remains protected: {field}"
        );
    }
    let (code, result) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        &operation,
    ]);
    assert_eq!(code, 2, "{result}");
    assert_eq!(
        h.calls().len(),
        2,
        "unknown continuation never dispatches; independent observation does not replace its task"
    );
}

#[test]
fn continuation_recovers_the_originals_handoff_before_releasing_the_prior_operation() {
    if !isolated_capability_test(
        "continuation_recovers_the_originals_handoff_before_releasing_the_prior_operation",
    ) {
        return;
    }
    let h = Host::new(vec![
        api(snapshot()),
        json!({"prefix":["api","graphql"],"exit":2,"stderr":"original readback failure"}),
        api(snapshot()),
    ]);
    let (code, previous) = h.status();
    assert_eq!(code, 0, "{previous}");
    let path = previous["operation"].as_str().unwrap();
    let saved_result = fs::read(h.root.path().join(path).join("result.json")).unwrap();
    // Recreate the operation's actual recovery protection after interruption
    // between sealing and custody handoff. All process originals belong to it.
    Inventory::adopted(h.root.path())
        .unwrap()
        .unwrap()
        .reference_paths(
            &format!("pr-recovery:{}", chrono_harness::sha256(path.as_bytes())),
            &[path.to_owned()],
        )
        .unwrap();
    let (code, failed) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        path,
    ]);
    assert_eq!(code, 2, "{failed}");
    maintain_after_diagnostics(&h);
    assert!(
        h.root.path().join(path).exists(),
        "failed query preserves prior recovery"
    );
    for field in ["stdout_original", "stderr_original", "launch_original"] {
        assert!(
            h.root
                .path()
                .join(previous["receipts"][0][field]["path"].as_str().unwrap())
                .exists(),
            "failed query preserves original producer protection: {field}"
        );
    }
    let (code, current) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        path,
    ]);
    assert_eq!(code, 0, "{current}");
    assert_eq!(h.calls().len(), 3);
    assert_eq!(
        fs::read(h.root.path().join(path).join("result.json")).unwrap(),
        saved_result
    );
    maintain_after_diagnostics(&h);
    assert!(
        !h.root.path().join(path).exists(),
        "accepted prior operation expires"
    );
    for field in ["stdout_original", "stderr_original", "launch_original"] {
        let original = previous["receipts"][0][field]["path"].as_str().unwrap();
        assert!(
            !h.root.path().join(original).exists(),
            "accepted original producer handoff must recover and expire: {field}"
        );
    }
    assert!(
        h.root
            .path()
            .join(current["operation"].as_str().unwrap())
            .exists(),
        "current OPEN observation remains protected"
    );
}

#[test]
fn continuation_requires_available_unchanged_actual_process_originals_before_dispatch() {
    for failure in ["missing", "modified", "replaced"] {
        let h = Host::new(vec![api(snapshot()), api(snapshot())]);
        let (code, previous) = h.status();
        assert_eq!(code, 0, "{previous}");
        let operation = previous["operation"].as_str().unwrap();
        let original = previous["receipts"][0]["stdout_original"]["path"]
            .as_str()
            .unwrap();
        let path = h.root.path().join(original);
        match failure {
            "missing" => fs::remove_file(&path).unwrap(),
            "modified" => fs::write(&path, b"different original output").unwrap(),
            "replaced" => {
                let saved = fs::read(&path).unwrap();
                fs::rename(&path, path.with_extension("saved-original")).unwrap();
                fs::write(&path, saved).unwrap();
            }
            _ => unreachable!(),
        }
        let (code, result) = h.run(&[
            "pr-status",
            "--pr",
            "42",
            "--head-sha",
            HEAD,
            "--continue-operation",
            operation,
        ]);
        assert_eq!(code, 2, "{failure}: {result}");
        assert!(result["error"].is_string(), "{failure}: {result}");
        assert_eq!(
            h.calls().len(),
            1,
            "{failure}: failed original qualification must not dispatch a new query"
        );
        assert!(h.root.path().join(operation).exists());
    }
}

#[test]
fn continuation_can_recheck_a_terminal_operation_after_its_original_handoff() {
    let mut merged = snapshot();
    merged["data"]["repository"]["pullRequest"]["state"] = json!("MERGED");
    merged["data"]["repository"]["pullRequest"]["mergeCommit"] = json!({"oid":OTHER});
    let h = Host::new(vec![api(merged.clone()), api(merged)]);
    let (code, previous) = h.status();
    assert_eq!(code, 0, "{previous}");
    let (code, current) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        previous["operation"].as_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{current}");
    assert_eq!(current["pr_state"], "MERGED");
    assert_eq!(current["merge_commit"], OTHER);
    assert_eq!(h.calls().len(), 2);
}

#[test]
fn object_custody_moves_across_head_changes_and_keeps_another_pr() {
    if !isolated_capability_test("object_custody_moves_across_head_changes_and_keeps_another_pr") {
        return;
    }
    let mut other = snapshot();
    other["data"]["repository"]["pullRequest"]["number"] = json!(43);
    other["data"]["repository"]["pullRequest"]["url"] =
        json!("https://github.com/example/host/pull/43");
    let newer: Value = serde_json::from_str(&snapshot().to_string().replace(HEAD, OTHER)).unwrap();
    let h = Host::new(vec![api(snapshot()), api(other), api(newer)]);
    let (code, old) = h.status();
    assert_eq!(code, 0, "{old}");
    let (code, independent) = h.run(&["pr-status", "--pr", "43", "--head-sha", HEAD]);
    assert_eq!(code, 0, "{independent}");
    let (code, current) = h.run(&["pr-status", "--pr", "42", "--head-sha", OTHER]);
    assert_eq!(code, 0, "{current}");
    assert_eq!(current["observed_head"], OTHER);
    let old_path = h.root.path().join(old["operation"].as_str().unwrap());
    let saved = fs::read(old_path.join("result.json")).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&saved).unwrap(), old);
    maintain_after_diagnostics(&h);
    assert!(
        !old_path.exists(),
        "completed old query can roll when the same PR advances to another head"
    );
    for result in [&current, &independent] {
        assert_eq!(
            serde_json::from_slice::<Value>(
                &fs::read(
                    h.root
                        .path()
                        .join(result["operation"].as_str().unwrap())
                        .join("result.json")
                )
                .unwrap()
            )
            .unwrap(),
            *result
        );
    }
    for receipt in old["receipts"].as_array().unwrap() {
        for field in ["stdout_original", "stderr_original", "launch_original"] {
            assert!(
                !h.root
                    .path()
                    .join(receipt[field]["path"].as_str().unwrap())
                    .exists(),
                "superseded process input rolls: {field}"
            );
        }
    }
    assert_eq!(h.calls().len(), 3);
    let (code, missing) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        old["operation"].as_str().unwrap(),
    ]);
    assert_eq!(code, 2, "{missing}");
    assert_eq!(h.calls().len(), 3);
}

#[test]
fn object_custody_preserves_a_real_reader_then_rolls_after_release() {
    if !isolated_capability_test("object_custody_preserves_a_real_reader_then_rolls_after_release")
    {
        return;
    }
    let h = Host::new(vec![api(snapshot()), api(snapshot())]);
    let (code, old) = h.status();
    assert_eq!(code, 0, "{old}");
    let old_path = old["operation"].as_str().unwrap().to_owned();
    let inventory = Inventory::adopted(h.root.path()).unwrap().unwrap();
    inventory
        .reference_paths("actual-review-consumer", &[old_path.clone()])
        .unwrap();
    let reader =
        chrono_harness::retained_artifacts::retention::read_guard(h.root.path(), &old_path)
            .unwrap()
            .unwrap();
    let original = fs::read(h.root.path().join(&old_path).join("result.json")).unwrap();
    let (code, current) = h.status();
    assert_eq!(code, 0, "{current}");
    maintain_after_diagnostics(&h);
    assert_eq!(
        fs::read(h.root.path().join(&old_path).join("result.json")).unwrap(),
        original
    );
    drop(reader);
    inventory
        .reference_paths("actual-review-consumer", &[])
        .unwrap();
    maintain_after_diagnostics(&h);
    assert!(
        !h.root.path().join(&old_path).exists(),
        "completed history rolls only after the actual consumer ends"
    );
    assert!(
        h.root
            .path()
            .join(current["operation"].as_str().unwrap())
            .join("result.json")
            .exists()
    );
    assert_eq!(h.calls().len(), 2);
}

#[cfg(unix)]
#[test]
fn object_custody_does_not_regress_when_an_older_query_returns_last() {
    if !isolated_capability_test("object_custody_does_not_regress_when_an_older_query_returns_last")
    {
        return;
    }
    use std::{
        io::{Read, Write},
        os::unix::net::{UnixListener, UnixStream},
    };
    let mut deferred = api(snapshot());
    deferred["pause_before_response"] = json!(true);
    let newer: Value = serde_json::from_str(&snapshot().to_string().replace(HEAD, OTHER)).unwrap();
    let h = Host::new(vec![deferred, api(newer)]);
    let listener = UnixListener::bind(h.root.path().join("response.sock")).unwrap();
    let (old, current) = std::thread::scope(|scope| {
        let task = scope.spawn(|| {
            let result = h.status();
            if let Ok(mut ready) = UnixStream::connect(h.root.path().join("response.sock")) {
                let bytes =
                    serde_json::to_vec(&json!({"completed_before_handshake":result})).unwrap();
                let _ = ready.write_all(&(bytes.len() as u32).to_be_bytes());
                let _ = ready.write_all(&bytes);
            }
            result
        });
        let (mut ready, _) = listener.accept().unwrap();
        let mut length = [0; 4];
        ready.read_exact(&mut length).unwrap();
        let mut bytes = vec![0; u32::from_be_bytes(length) as usize];
        ready.read_exact(&mut bytes).unwrap();
        let started: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            started.get("completed_before_handshake").is_none(),
            "{started}"
        );
        assert!(
            !h.root
                .path()
                .join(started["operation"].as_str().unwrap())
                .join("result.json")
                .exists()
        );
        let (code, current) = h.run(&["pr-status", "--pr", "42", "--head-sha", OTHER]);
        ready.write_all(&[1]).unwrap();
        let (old_code, old) = task.join().unwrap();
        assert_eq!(code, 0, "{current}");
        assert_eq!(old_code, 0, "{old}");
        assert_eq!(old["operation"], started["operation"]);
        (old, current)
    });
    maintain_after_diagnostics(&h);
    assert!(
        !h.root
            .path()
            .join(old["operation"].as_str().unwrap())
            .exists(),
        "older completed query must not replace newer same-object custody"
    );
    assert_eq!(
        serde_json::from_slice::<Value>(
            &fs::read(
                h.root
                    .path()
                    .join(current["operation"].as_str().unwrap())
                    .join("result.json")
            )
            .unwrap()
        )
        .unwrap(),
        current
    );
    assert_eq!(current["observed_head"], OTHER);
    assert_eq!(h.calls().len(), 2);
}

#[test]
fn object_custody_failed_complete_query_rolls_without_ending_the_open_pr() {
    if !isolated_capability_test(
        "object_custody_failed_complete_query_rolls_without_ending_the_open_pr",
    ) {
        return;
    }
    let h = Host::new(vec![
        api(snapshot()),
        json!({"prefix":["api","graphql"],"stdout":"native authentication refusal","exit":2}),
    ]);
    let (code, prior) = h.status();
    assert_eq!(code, 0, "{prior}");
    let previous = prior["operation"].as_str().unwrap();
    let original = fs::read(h.root.path().join(previous).join("result.json")).unwrap();
    let (code, failure) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        previous,
    ]);
    assert_eq!(code, 2, "{failure}");
    assert_eq!(failure["operation_state"], "completed");
    assert!(failure["error"].is_string());
    assert_eq!(failure["receipts"][0]["exit_code"], 2);
    let path = h.root.path().join(failure["operation"].as_str().unwrap());
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(path.join("result.json")).unwrap()).unwrap(),
        failure
    );
    maintain_after_diagnostics(&h);
    assert_eq!(
        fs::read(h.root.path().join(previous).join("result.json")).unwrap(),
        original,
        "a failed query cannot end the current OPEN object custody"
    );
    assert!(
        !path.exists(),
        "known failed command with sealed originals and no remaining consumer is terminal and can roll"
    );
    for receipt in failure["receipts"].as_array().unwrap() {
        for field in ["stdout_original", "stderr_original", "launch_original"] {
            assert!(
                !h.root
                    .path()
                    .join(receipt[field]["path"].as_str().unwrap())
                    .exists()
            );
        }
    }
    assert_eq!(h.calls().len(), 2);
}

#[test]
fn object_custody_failed_publication_stays_unknown_after_a_new_complete_observation() {
    if !isolated_capability_test(
        "object_custody_failed_publication_stays_unknown_after_a_new_complete_observation",
    ) {
        return;
    }
    let mut broken = api(snapshot());
    broken["block_result"] = json!(true);
    let h = Host::new(vec![api(snapshot()), broken, api(snapshot())]);
    let (code, prior) = h.status();
    assert_eq!(code, 0, "{prior}");
    let (code, failure) = h.status();
    assert_eq!(code, 2, "{failure}");
    assert!(!failure["storage_error"].is_null(), "{failure}");
    let unknown = h.root.path().join(failure["operation"].as_str().unwrap());
    assert!(
        unknown.join("result.json").is_dir(),
        "the actual publication failure remains unsealed"
    );
    let (code, current) = h.status();
    assert_eq!(code, 0, "{current}");
    maintain_after_diagnostics(&h);
    assert!(
        unknown.join("result.json").is_dir(),
        "a later successful query does not repair or complete the failed publication"
    );
    for receipt in failure["receipts"].as_array().unwrap() {
        for field in ["stdout_original", "stderr_original", "launch_original"] {
            assert!(
                h.root
                    .path()
                    .join(receipt[field]["path"].as_str().unwrap())
                    .exists(),
                "unknown publication keeps its actual original: {field}"
            );
        }
    }
    assert!(
        !h.root
            .path()
            .join(prior["operation"].as_str().unwrap())
            .exists(),
        "the known old command can roll independently of another unknown publication"
    );
    assert_eq!(
        serde_json::from_slice::<Value>(
            &fs::read(
                h.root
                    .path()
                    .join(current["operation"].as_str().unwrap())
                    .join("result.json")
            )
            .unwrap()
        )
        .unwrap(),
        current
    );
    assert_eq!(h.calls().len(), 3);
}

#[test]
fn watch_final_api_failure_keeps_the_valid_open_fact_until_newer_custody() {
    if !isolated_capability_test(
        "watch_final_api_failure_keeps_the_valid_open_fact_until_newer_custody",
    ) {
        return;
    }
    let h = Host::new(vec![
        api(pending()),
        watch(0),
        json!({"prefix":["api","graphql"],"exit":2,"stderr":"final authentication refusal"}),
        api(snapshot()),
    ]);
    let (code, failed) = h.run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]);
    assert_eq!(code, 2, "{failed}");
    assert_eq!(failed["operation_state"], "completed");
    assert_eq!(failed["pr_state"], "OPEN");
    let path = h.root.path().join(failed["operation"].as_str().unwrap());
    maintain_after_diagnostics(&h);
    assert!(
        path.join("result.json").exists(),
        "last valid OPEN observation survives later API failure"
    );
    let (code, current) = h.status();
    assert_eq!(code, 0, "{current}");
    maintain_after_diagnostics(&h);
    assert!(
        !path.exists(),
        "returned failed watch rolls after newer successful custody"
    );
    assert!(
        h.root
            .path()
            .join(current["operation"].as_str().unwrap())
            .exists()
    );
    assert_eq!(h.calls().len(), 4);
}

#[cfg(unix)]
#[test]
fn watch_final_query_is_ordered_after_a_query_during_its_native_wait() {
    if !isolated_capability_test(
        "watch_final_query_is_ordered_after_a_query_during_its_native_wait",
    ) {
        return;
    }
    use std::{
        io::{Read, Write},
        os::unix::net::{UnixListener, UnixStream},
    };
    let mut native = watch(0);
    native["pause_before_response"] = json!(true);
    let mut intervening = api(snapshot());
    intervening["pause_before_response"] = json!(true);
    let mut terminal = snapshot();
    terminal["data"]["repository"]["pullRequest"]["headRefOid"] = json!(OTHER);
    terminal["data"]["repository"]["pullRequest"]["state"] = json!("CLOSED");
    let h = Host::new(vec![api(pending()), native, intervening, api(terminal)]);
    let listener = UnixListener::bind(h.root.path().join("response.sock")).unwrap();
    let accept = || {
        let (mut ready, _) = listener.accept().unwrap();
        let mut length = [0; 4];
        ready.read_exact(&mut length).unwrap();
        let mut bytes = vec![0; u32::from_be_bytes(length) as usize];
        ready.read_exact(&mut bytes).unwrap();
        let started: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            started.get("completed_before_handshake").is_none(),
            "{started}"
        );
        (ready, started)
    };
    let run = |args: &[&str]| {
        // This isolated test exercises concurrent observations in the real
        // command implementation. Extra CLI launcher processes add no ordering
        // evidence; the native gh children and their originals remain real.
        let args = args
            .iter()
            .copied()
            .chain(["--host-root", h.root.path().to_str().unwrap()])
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();
        let output = chrono_pr::run(&args);
        assert!(output.stderr.is_empty(), "{}", output.stderr);
        let result = (
            output.exit_code,
            serde_json::from_str::<Value>(&output.stdout).unwrap(),
        );
        assert_eq!(result.1["schema"], "chrono-pr-result/v2");
        if let Ok(mut ready) = UnixStream::connect(h.root.path().join("response.sock")) {
            let bytes = serde_json::to_vec(&json!({"completed_before_handshake":result})).unwrap();
            let _ = ready.write_all(&(bytes.len() as u32).to_be_bytes());
            let _ = ready.write_all(&bytes);
        }
        result
    };
    let (watched, intermediate) = std::thread::scope(|scope| {
        let task = scope.spawn(|| run(&["pr-watch", "--pr", "42", "--head-sha", HEAD]));
        let (mut ready, started) = accept();
        assert_eq!(started["call"], 2);
        let query = scope.spawn(|| run(&["pr-status", "--pr", "42", "--head-sha", HEAD]));
        let (mut query_ready, query_started) = accept();
        assert_eq!(query_started["call"], 1);
        assert_ne!(query_started["operation"], started["operation"]);
        // The real intervening API call has already reserved its observation.
        // Release the native wait without waiting for that command's separate
        // result publication; both commands are joined before checking custody.
        let query_release = query_ready.write_all(&[1]);
        let watch_release = ready.write_all(&[1]);
        let (code, intermediate) = query.join().unwrap();
        let (watch_code, watched) = task.join().unwrap();
        assert!(query_release.is_ok(), "{query_release:?}: {intermediate}");
        assert!(watch_release.is_ok(), "{watch_release:?}: {watched}");
        assert_eq!(code, 0, "{intermediate}");
        assert_eq!(watch_code, 4, "{watched}");
        assert_eq!(watched["pr_state"], "CLOSED");
        assert_eq!(watched["observed_head"], OTHER);
        assert_eq!(watched["receipts"].as_array().unwrap().len(), 3);
        (watched, intermediate)
    });
    maintain_after_diagnostics(&h);
    for result in [&watched, &intermediate] {
        assert!(
            !h.root
                .path()
                .join(result["operation"].as_str().unwrap())
                .exists(),
            "actual terminal PR fact releases object custody despite stale command head"
        );
    }
    assert_eq!(
        h.calls().len(),
        4,
        "one native watcher and actual final query"
    );
}

#[test]
fn unverified_create_keeps_unknown_originals_after_an_independent_observation() {
    if !isolated_capability_test(
        "unverified_create_keeps_unknown_originals_after_an_independent_observation",
    ) {
        return;
    }
    let h = Host::new(vec![
        resolve(),
        list(json!([])),
        json!({"prefix":["pr","create"],"stdout":"https://github.com/example/host/pull/42\n"}),
        json!({"prefix":["api","graphql"],"exit":2}),
        api(snapshot()),
    ]);
    let (code, original) = h.run(&open_args());
    assert_eq!(code, 2, "{original}");
    assert_eq!(original["operation_state"], "unknown");
    let path = original["operation"].as_str().unwrap();
    let saved = fs::read(h.root.path().join(path).join("result.json")).unwrap();
    let (code, current) = h.status();
    assert_eq!(code, 0, "{current}");
    maintain_after_diagnostics(&h);
    assert_eq!(
        fs::read(h.root.path().join(path).join("result.json")).unwrap(),
        saved
    );
    for receipt in original["receipts"].as_array().unwrap() {
        for field in ["stdout_original", "stderr_original", "launch_original"] {
            assert!(
                h.root
                    .path()
                    .join(receipt[field]["path"].as_str().unwrap())
                    .exists()
            );
        }
    }
    let (code, continued) = h.run(&[
        "pr-status",
        "--pr",
        "42",
        "--head-sha",
        HEAD,
        "--continue-operation",
        path,
    ]);
    assert_eq!(code, 2, "{continued}");
    assert_eq!(
        h.calls().len(),
        5,
        "unknown write cannot be completed or recreated by another query"
    );
}
