mod support;
use chrono_harness::{facts, sha256, wire};
use chrono_judge_filemap::{IMPACT_SCHEMA, Impact};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
use support::*;
struct Host {
    dir: tempfile::TempDir,
    base: String,
    candidate: String,
    values: Values,
}
impl Host {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("filemap full host with spaces ")
            .tempdir()
            .unwrap();
        let root = dir.path();
        git(root, &["init", "-q"]);
        fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
        for (project, binary) in [
            ("runner", "chrono-harness"),
            ("judge-registration", "chrono-judge-registration"),
            ("judge-filemap", "chrono-judge-filemap"),
        ] {
            let p = source().join(format!("crates/{project}/target/debug/{binary}"));
            assert!(
                p.is_file(),
                "build registered binary first: {}",
                p.display()
            );
            fs::copy(p, root.join(format!(".chrono-harness/bin/{binary}"))).unwrap();
        }
        let mut v = values();
        for f in v[FM]["files"].as_array().unwrap() {
            let p = root.join(f["path"].as_str().unwrap());
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, "fixture").unwrap();
        }
        fs::write(
            root.join(".gitignore"),
            ".chrono-harness/bin/\n.chrono-harness/state/\n",
        )
        .unwrap();
        v.get_mut(CONFIG).unwrap()["runner"]["sha256"] =
            sha256(&fs::read(root.join(".chrono-harness/bin/chrono-harness")).unwrap()).into();
        for j in v.get_mut(JUDGES).unwrap()["judges"].as_array_mut().unwrap() {
            j["executable"] = format!(
                ".chrono-harness/bin/absent-historical-{}",
                j["id"].as_str().unwrap()
            )
            .into();
            j["sha256"] = "a".repeat(64).into();
        }
        write_values(root, &v);
        let base = commit(root);
        for j in v.get_mut(JUDGES).unwrap()["judges"].as_array_mut().unwrap() {
            let p = format!(
                ".chrono-harness/bin/chrono-judge-{}",
                j["id"].as_str().unwrap()
            );
            j["sha256"] = sha256(&fs::read(root.join(&p)).unwrap()).into();
            j["executable"] = p.into();
        }
        fs::write(root.join("src.bin"), [0, 255, 42]).unwrap();
        write_values(root, &v);
        let candidate = commit(root);
        Self {
            dir,
            base,
            candidate,
            values: v,
        }
    }
    fn root(&self) -> &Path {
        self.dir.path()
    }
    fn save(&mut self) {
        write_values(self.root(), &self.values);
        self.candidate = commit(self.root());
    }
    fn run(&self) -> (i32, Value) {
        let ctx = json!({"schema_version":1,"base":self.base,"candidate":self.candidate,"dev_tip":self.base,"branch_ref":"integration/fixture","fork_point":self.base,"branch_started_at":"2026-01-01T00:00:00Z","observed_at":"2026-01-01T01:00:00Z","operation":"validate.delta","integration_evidence":null});
        fs::create_dir_all(self.root().join(".chrono-harness/state")).unwrap();
        fs::write(
            self.root().join(".chrono-harness/state/context.json"),
            serde_json::to_vec(&ctx).unwrap(),
        )
        .unwrap();
        let o = Command::new(self.root().join(".chrono-harness/bin/chrono-harness"))
            .current_dir("/")
            .env_remove("HOME")
            .args([
                "check",
                "--config",
                self.root().join(CONFIG).to_str().unwrap(),
                "--base",
                &self.base,
                "--candidate",
                &self.candidate,
                "--context",
                ".chrono-harness/state/context.json",
            ])
            .output()
            .unwrap();
        let v = serde_json::from_slice(&o.stdout)
            .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&o.stderr)}));
        (o.status.code().unwrap(), v)
    }
    fn request(&self) -> wire::Request {
        let a = facts::registry_values(self.root(), &self.base, CONFIG).unwrap();
        let b = facts::registry_values(self.root(), &self.candidate, CONFIG).unwrap();
        let mut r = wire::Request {
            protocol: wire::PROTOCOL.into(),
            request_id: String::new(),
            judge_id: "filemap".into(),
            mode: "evaluate".into(),
            base: wire::Endpoint {
                commit: self.base.clone(),
                tree: facts::verify_oid(self.root(), &self.base).unwrap(),
                root: self.root().join(".chrono-harness/state/base"),
            },
            candidate: wire::Endpoint {
                commit: self.candidate.clone(),
                tree: facts::verify_oid(self.root(), &self.candidate).unwrap(),
                root: self.root().into(),
            },
            delta: git_delta(self.root(), &self.base, &self.candidate),
            registries: wire::Registries {
                base: self
                    .root()
                    .join(".chrono-harness/state/base/.chrono-harness"),
                candidate: self.root().join(".chrono-harness"),
                digest: facts::registry_digest(&a, &b).unwrap(),
            },
            context: wire::Context {
                path: self.root().join(".chrono-harness/state/context.json"),
                sha256: "a".repeat(64),
            },
            impact: Value::Null,
            prior_results: vec![],
            config_path: CONFIG.into(),
            checkout: facts::checkout(self.root(), &self.candidate).unwrap(),
            runner: wire::Executable {
                path: "unused-by-filemap".into(),
                sha256: "a".repeat(64),
                version: "fixture".into(),
            },
        };
        r.seal().unwrap();
        r
    }
}
fn codes(v: &Value) -> String {
    serde_json::to_string(v).unwrap()
}
#[test]
fn actual_runner_registration_filemap_consumes_fixed_commits_and_attributes_impact() {
    let h = Host::new();
    let (exit, r) = h.run();
    assert_eq!(exit, 0, "{r:#}");
    assert_eq!(
        r["judges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| j["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["registration", "filemap"]
    );
    for j in r["judges"].as_array().unwrap() {
        assert_eq!(j["state"], "executed");
        assert_eq!(j["exit_code"], 0);
        assert_eq!(j["response"]["status"], "pass");
    }
    assert_eq!(
        r["sources"]["/impact"],
        json!(["/judges/1/response/outputs/impact"])
    );
    assert_eq!(
        r["impact"],
        r.pointer("/judges/1/response/outputs/impact")
            .unwrap()
            .clone()
    );
    let impact: Impact = serde_json::from_value(r["impact"].clone()).unwrap();
    assert_eq!(impact.schema, IMPACT_SCHEMA);
    assert_eq!(
        impact
            .required_tests
            .iter()
            .map(|t| t.node.as_str())
            .collect::<Vec<_>>(),
        ["test:t"]
    );
    assert!(impact.required_tests[0].candidate_present);
    assert_eq!(
        impact
            .closure
            .witness(
                &impact
                    .seeds
                    .iter()
                    .find(|s| s.reference == "src.bin")
                    .unwrap()
                    .id,
                "test:t"
            )
            .unwrap()
            .len(),
        2
    );
    assert!(r["tests"].is_null());
    assert!(r["costs"].is_null());
    assert!(
        !h.root()
            .join(".chrono-harness/bin/absent-historical-filemap")
            .exists()
    );
}
#[test]
fn full_report_removed_test_is_visible_not_executed_or_retired() {
    let mut h = Host::new();
    h.values.get_mut(PROJECTS).unwrap()["projects"] = json!([]);
    h.values.get_mut(FM).unwrap()["project_edges"] = json!([]);
    h.values.get_mut(FM).unwrap()["test_costs"] = json!([]);
    h.values.get_mut(FM).unwrap()["files"][0]["edges"] = json!([]);
    h.save();
    let (exit, r) = h.run();
    assert_eq!(exit, 0, "{r:#}");
    assert_eq!(r["impact"]["required_tests"][0]["node"], "test:t");
    assert_eq!(r["impact"]["required_tests"][0]["candidate_present"], false);
    assert!(r["tests"].is_null());
    assert!(r["impact"].get("retired_tests").is_none());
}
#[test]
fn full_check_missing_or_malformed_required_input_is_nonzero() {
    for mode in ["missing", "malformed", "structure"] {
        let mut h = Host::new();
        match mode {
            "missing" => fs::remove_file(h.root().join(FM)).unwrap(),
            "malformed" => fs::write(h.root().join(FM), b"{").unwrap(),
            _ => {
                h.values.get_mut(FM).unwrap()["files"] = json!({});
                write_values(h.root(), &h.values);
            }
        }
        h.candidate = commit(h.root());
        let (exit, r) = h.run();
        assert_ne!(exit, 0, "{mode}: {r:#}");
        assert!(r["impact"].is_null());
    }
}
#[test]
fn full_external_input_declaration_remains_unresolved_before_filemap() {
    let mut h = Host::new();
    h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"] =
        json!([{"id":"outside","location":"/not-retained","sha256":null}]);
    h.save();
    let (exit, r) = h.run();
    assert_ne!(exit, 0);
    assert!(codes(&r).contains("E_EVIDENCE_UNRESOLVED"));
    assert_eq!(r["judges"][1]["state"], "blocked");
    assert!(r["impact"].is_null());
}
#[test]
fn filemap_process_rejects_forged_empty_delta_missing_oid_and_registry_digest() {
    let h = Host::new();
    let binding: wire::Binding =
        serde_json::from_value(h.values[JUDGES]["judges"][1].clone()).unwrap();
    for mode in ["delta", "object", "digest", "schema"] {
        let mut req = h.request();
        match mode {
            "delta" => req.delta.clear(),
            "object" => req.base.commit = "a".repeat(40),
            "digest" => req.registries.digest = "b".repeat(64),
            _ => req.config_path = "missing.json".into(),
        };
        req.seal().unwrap();
        let (r, p) = wire::invoke(
            &req,
            &binding,
            &std::collections::BTreeMap::new(),
            30,
            8388608,
        )
        .unwrap();
        assert_eq!(p.exit_code, 2);
        assert_eq!(r.status, wire::Status::Error);
        assert!(!r.outputs.contains_key("impact"));
        assert_eq!(r.findings[0].code, "E_INPUT");
    }
}
#[test]
fn full_reached_historical_dangling_has_consecutive_path_cause() {
    let mut h = Host::new();
    // Persist defect at both endpoints so registration's direct-change policy leaves it to impact.
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("project:p", "compile", "project:missing"));
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("src.bin"), "next").unwrap();
    h.candidate = commit(h.root());
    let (exit, r) = h.run();
    assert_eq!(exit, 1, "{r:#}");
    assert_eq!(r["judges"][0]["response"]["status"], "pass");
    assert_eq!(r["judges"][1]["response"]["status"], "fail");
    for f in r["judges"][1]["response"]["findings"].as_array().unwrap() {
        assert_eq!(f["delta_refs"], json!(["src.bin"]));
        assert_eq!(
            f["causes"],
            json!(["file:src.bin", "project:p", "project:missing"])
        );
    }
    assert_eq!(
        r["sources"]["/impact"],
        json!(["/judges/1/response/outputs/impact"])
    );
}
