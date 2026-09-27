#[path = "../../judge-projects-tests/tests/support/host.rs"]
mod host;
#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::sha256;
use host::Host;
use serde_json::{Value, json};
use std::{fs, process::Command};
use support::*;

fn fixture() -> Host {
    let mut h = Host::new(true);
    for (id, deps) in [
        ("cost", vec!["registration", "filemap"]),
        ("mixed", vec!["registration", "filemap", "cost"]),
        (
            "workflow",
            vec![
                "registration",
                "filemap",
                "routes",
                "projects",
                "cost",
                "mixed",
            ],
        ),
    ] {
        let bytes =
            fs::read(source().join(format!("crates/judge-{id}/target/debug/chrono-judge-{id}")))
                .unwrap();
        let path = format!(".chrono-harness/bin/chrono-judge-{id}");
        fs::write(h.root().join(&path), &bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(h.root().join(&path), fs::Permissions::from_mode(0o755)).unwrap();
        }
        h.values.get_mut(JUDGES).unwrap()["judges"].as_array_mut().unwrap().push(json!({"id":id,"executable":path,"version":"0.1.0","sha256":sha256(&bytes),"argv":["--protocol","chrono-judge/v1"],"selector":"every-delta","after":deps,"modes":["evaluate"]}));
    }
    h.values.get_mut(JUDGES).unwrap()["migration_validator"] = "workflow".into();
    let w = h.values.get_mut(WORKFLOW).unwrap();
    w["integration"]["bind"] = json!([
        "base",
        "candidate_tree",
        "registry_digest",
        "executables",
        "tools",
        "environment",
        "effective_inputs",
        "required_tests",
        "results_digest"
    ]);
    w["integration"]["tests"] = json!(["t"]);
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("p/product.py"), "def double(n): return 2*n\n").unwrap();
    h.save();
    h
}
fn snapshots(h: &Host) -> Value {
    let invoke = |args: &[&str]| {
        let out = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
            .current_dir("/")
            .env("DECLARED_EMPTY", "")
            .env_remove("DECLARED_ABSENT")
            .args(args)
            .args(["--host-root", h.root().to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    git(&h.root(), &["checkout", "--detach", &h.base]);
    invoke(&[
        "capture",
        "--config",
        CONFIG,
        "--commit",
        &h.base,
        "--output",
        ".chrono-harness/state/base.json",
    ]);
    git(&h.root(), &["checkout", "--detach", &h.candidate]);
    invoke(&[
        "capture",
        "--config",
        CONFIG,
        "--commit",
        &h.candidate,
        "--output",
        ".chrono-harness/state/candidate.json",
    ]);
    invoke(&[
        "pair",
        "--base-snapshot",
        ".chrono-harness/state/base.json",
        "--candidate-snapshot",
        ".chrono-harness/state/candidate.json",
        "--output",
        ".chrono-harness/state/paired.json",
    ])
}
fn check(h: &Host, pair: &Value) -> (i32, Value) {
    h.run_with_context(
        |v| *v = pair.clone(),
        |ctx| {
            ctx["schema_version"] = 2.into();
            ctx["run_kind"] = "integration".into()
        },
    )
}
#[test]
fn produced_snapshots_drive_actual_seven_judge_execution_without_inline_input_bytes() {
    let h = fixture();
    let pair = snapshots(&h);
    assert!(pair["base"]["files"]["interpreter"].get("bytes").is_none());
    let (exit, r) = check(&h, &pair);
    assert_eq!(exit, 0, "{}", r["findings"]);
    assert_eq!(r["tests"]["tests"]["test:t"], "passed");
    assert!(
        h.root()
            .join(".chrono-harness/state/integration.json")
            .is_file()
    );
    let (exit, inline) = h.run_with_context(
        |_| {},
        |ctx| {
            ctx["schema_version"] = 2.into();
            ctx["run_kind"] = "integration".into()
        },
    );
    assert_eq!(exit, 0, "{}", inline["findings"]);
    assert_eq!(r["effective_inputs"], inline["effective_inputs"]);
}

#[test]
fn malformed_misbound_missing_and_changed_retained_inputs_fail_before_execution() {
    let h = fixture();
    let pair = snapshots(&h);
    let reject = |v: &Value, expected: &str| {
        let (exit, r) = check(&h, v);
        assert_ne!(exit, 0, "{r:#}");
        assert!(
            r["findings"].to_string().contains(expected),
            "expected {expected}: {}",
            r["findings"]
        );
        assert!(r["tests"].is_null());
        assert!(!h.root().join(".chrono-harness/state/order").exists());
    };
    let mut bad = pair.clone();
    bad["base"]["commit"] = h.candidate.clone().into();
    reject(&bad, "wrong endpoint");
    bad = pair.clone();
    bad["base"]["config_digest"] = "0".repeat(64).into();
    reject(&bad, "snapshot configuration binding");
    bad = pair.clone();
    bad["base"]["files"]["extra"] = bad["base"]["files"]["data"].clone();
    reject(&bad, "unregistered retained input extra");
    bad = pair.clone();
    bad["base"]["files"]["data"]["bytes"] = json!([]);
    reject(&bad, "exactly one bytes or blob");
    bad = pair.clone();
    bad["base"]["files"]["data"]["blob"] = "/outside".into();
    reject(&bad, "E_INPUT_BLOB");
    bad = pair.clone();
    bad["base"]["files"]["data"]["length"] = (-1).into();
    reject(&bad, "E_INPUT_BLOB");
    bad = pair.clone();
    bad["base"]["files"]["data"]["blob"] = ".chrono-harness/state/missing".into();
    reject(&bad, "E_INPUT_BLOB");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            h.external.path(),
            h.root().join(".chrono-harness/state/link"),
        )
        .unwrap();
        bad = pair.clone();
        bad["base"]["files"]["data"]["blob"] = ".chrono-harness/state/link".into();
        reject(&bad, "E_INPUT_BLOB");
    }
    let blob = h
        .root()
        .join(pair["base"]["files"]["data"]["blob"].as_str().unwrap());
    let old = fs::read(&blob).unwrap();
    fs::write(&blob, b"damaged").unwrap();
    reject(&pair, "E_INPUT_BLOB");
    fs::write(blob, old).unwrap();
    fs::write(h.external.path(), b"candidate changed after capture").unwrap();
    reject(&pair, "candidate input changed: data");
}

#[test]
fn input_larger_than_judge_json_bound_remains_a_small_request_and_is_actually_validated() {
    let mut h = fixture();
    // Independent SHA-256 oracle: Python hashlib over 65 successive 1 MiB zero blocks.
    let digest = "25631f11bd18756ec0029380ec886af0c8824dc6b2706bbdb1d9451c7cf45f42";
    let length = 65 * 1024 * 1024;
    fs::File::create(h.external.path())
        .unwrap()
        .set_len(length)
        .unwrap();
    h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|v| v["id"] == "data")
        .unwrap()["sha256"] = digest.into();
    h.values.get_mut(CONFIG).unwrap()["protocol"]["timeout_seconds"] = 120.into();
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("p/product.py"), "def double(n): return n+n\n").unwrap();
    h.save();
    let pair = snapshots(&h);
    assert_eq!(pair["base"]["files"]["data"]["length"], length);
    assert!(serde_json::to_vec(&pair).unwrap().len() < 16384);
    let (exit, r) = check(&h, &pair);
    assert_eq!(exit, 0, "{}", r["findings"]);
    assert_eq!(
        r["effective_inputs"]["endpoints"]["candidate"]["files"]["data"]["sha256"],
        digest
    );
    assert_eq!(r["tests"]["tests"]["test:t"], "passed");
}
