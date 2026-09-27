#[path = "../../judge-cost-tests/tests/support/host.rs"]
mod host;
#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::sha256;
use host::Host;
use serde_json::{Value, json};
use std::fs;
use support::*;

fn install(h: &mut Host, name: &str, bytes: &[u8]) {
    let path = h.root().join(format!(".chrono-harness/bin/{name}"));
    fs::write(&path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}
fn host() -> Host {
    let mut h = Host::new(false);
    let bytes = fs::read(source().join("crates/judge-mixed/target/debug/chrono-judge-mixed"))
        .expect("build registered mixed executable");
    install(&mut h, "chrono-judge-mixed", &bytes);
    h.values.get_mut(JUDGES).unwrap()["judges"].as_array_mut().unwrap().push(json!({
        "id":"mixed","executable":".chrono-harness/bin/chrono-judge-mixed","version":"0.1.0","sha256":sha256(&bytes),
        "argv":["--protocol","chrono-judge/v1"],"selector":"every-delta","after":["registration","filemap","cost"],"modes":["evaluate"]}));
    for f in h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
    {
        f["surface"] = match f["path"].as_str().unwrap() {
            CONFIG | JUDGES | WORKFLOW => "judge-policy",
            FM | PROJECTS => "membership",
            "doc.txt" => "documentation",
            _ => "product",
        }
        .into();
    }
    h.values.get_mut(CONFIG).unwrap()["semantic_fields"] = json!([
        {"path":FM,"pointers":["/files/*/surface"],"on":"modify-delete-existing"},
        {"path":PROJECTS,"pointers":["/projects/*/actions"],"on":"add-modify-delete"}]);
    h.values.get_mut(WORKFLOW).unwrap()["mixed_change"]["code"] = "W_MIXED_JUDGE_PRODUCT".into();
    h.save();
    h.base = h.candidate.clone();
    h
}
fn mixed(r: &Value) -> &Value {
    &r["judges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"] == "mixed")
        .unwrap()["response"]
}
fn checked(h: &Host, expected: i32) -> Value {
    let (exit, r) = h.run();
    assert_eq!(
        exit, expected,
        "status={} findings={}",
        r["status"], r["findings"]
    );
    let j = r["judges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"] == "mixed")
        .unwrap();
    assert_eq!(j["state"], "executed");
    assert_eq!(j["process"]["exit_code"], expected);
    r
}

#[test]
fn actual_mixed_warning_is_zero_exit_with_both_groups_and_bound_cost_provenance() {
    let mut h = host();
    h.values.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]["execute"]["argv"] =
        json!(["-c", "exit 1"]);
    fs::write(h.root().join("src.bin"), "product after base").unwrap();
    h.save();
    let r = checked(&h, 0);
    let m = mixed(&r);
    assert_eq!(r["status"], "warn");
    assert_eq!(m["status"], "warn");
    assert_eq!(m["findings"][0]["code"], "W_MIXED_JUDGE_PRODUCT");
    let out = &m["outputs"]["rule_changes"];
    assert_eq!(out["rule_paths"], json!([PROJECTS]));
    assert_eq!(out["product_paths"], json!(["src.bin"]));
    assert_eq!(out["costs"], r["costs"]);
    assert_eq!(out["binding"]["candidate"], h.candidate);
    assert_eq!(out["costs"]["binding"], out["binding"]);
    assert_eq!(out["requires_acknowledgement"], false);
    assert_eq!(m["findings"][0]["causes"], json!(["/outputs/rule_changes"]));
    assert_eq!(
        out["changes"][0]["semantic"][0]["after"]["value"],
        h.values[PROJECTS]["projects"][0]["actions"]
    );
}

#[test]
fn actual_ordinary_membership_is_pass_and_rule_only_still_publishes_changes() {
    let mut h = host();
    fs::write(h.root().join("src.bin"), "another product").unwrap();
    fs::write(h.root().join("new.bin"), "registered new file").unwrap();
    h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(file("new.bin", json!([])));
    h.save();
    let r = checked(&h, 0);
    assert_eq!(r["status"], "pass");
    assert_eq!(
        mixed(&r)["outputs"]["rule_changes"]["rule_paths"],
        json!([])
    );
    h.base = h.candidate.clone();
    h.values.get_mut(PROJECTS).unwrap()["projects"][0]["actions"]["execute"]["argv"] =
        json!(["-c", "exit 2"]);
    h.save();
    let r = checked(&h, 0);
    assert_eq!(r["status"], "pass");
    assert_eq!(
        mixed(&r)["outputs"]["rule_changes"]["rule_paths"],
        json!([PROJECTS])
    );
    assert_eq!(mixed(&r)["outputs"]["rule_changes"]["mixed"], false);
}

#[test]
fn actual_missing_cost_predecessor_is_an_error() {
    let mut h = host();
    h.values.get_mut(JUDGES).unwrap()["judges"][3]["after"] = json!(["registration", "filemap"]);
    h.save();
    let r = checked(&h, 2);
    assert_eq!(mixed(&r)["findings"][0]["code"], "E_COST_INPUT");
    assert!(mixed(&r)["outputs"].as_object().unwrap().is_empty());
}

#[test]
fn actual_stale_cost_binding_is_rejected_without_recomputing_it() {
    let mut h = host();
    let script=b"#!/usr/bin/python3\nimport json,subprocess,sys\nr=json.load(sys.stdin)\np=subprocess.run([r['candidate']['root']+'/.chrono-harness/bin/chrono-judge-cost','--protocol','chrono-judge/v1'],input=json.dumps(r).encode(),capture_output=True)\nassert p.returncode==0, p.stderr\nv=json.loads(p.stdout)\nv['outputs']['costs']['binding']['candidate_tree']='0'*40\nprint(json.dumps(v))\n";
    install(&mut h, "stale-cost", script);
    h.values.get_mut(JUDGES).unwrap()["judges"][2]["executable"] =
        ".chrono-harness/bin/stale-cost".into();
    h.values.get_mut(JUDGES).unwrap()["judges"][2]["sha256"] = sha256(script).into();
    h.save();
    let r = checked(&h, 2);
    assert_eq!(mixed(&r)["findings"][0]["code"], "E_COST_INPUT");
    assert!(mixed(&r)["outputs"].as_object().unwrap().is_empty());
}

#[test]
fn actual_named_json_uses_fixed_git_bytes_and_rejects_invalid_utf8() {
    let mut h = host();
    let mut f = file("named.json", json!([]));
    f["surface"] = "membership".into();
    h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(f);
    h.values.get_mut(CONFIG).unwrap()["semantic_fields"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":"named.json","pointers":["/mode"],"on":"add-modify-delete"}));
    fs::write(h.root().join("named.json"), b"{\"mode\":\"before\"}").unwrap();
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("named.json"), b"{\"mode\":\"after\"}").unwrap();
    fs::write(h.root().join("src.bin"), "another version").unwrap();
    h.save();
    let r = checked(&h, 0);
    assert_eq!(
        mixed(&r)["outputs"]["rule_changes"]["rule_paths"],
        json!(["named.json"])
    );
    fs::write(h.root().join("named.json"), b"{\"mode\":\"\xff\"}").unwrap();
    h.save();
    let r = checked(&h, 2);
    assert_eq!(mixed(&r)["findings"][0]["code"], "E_SEMANTIC_INPUT");
}
