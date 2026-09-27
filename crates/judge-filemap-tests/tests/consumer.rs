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
            observations: Value::Null,
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
fn script_host() -> Host {
    let mut h = Host::new();
    script_pair(&mut h.values, "st");
    for path in ["s.sh", "st.sh"] {
        fs::write(h.root().join(path), "exit 0\n").unwrap();
    }
    h.save();
    h.base = h.candidate.clone();
    h
}
#[test]
fn full_changed_alias_collision_is_not_a_unique_executable() {
    let mut h = script_host();
    script_pair(&mut h.values, "t");
    fs::write(h.root().join("src.bin"), "changed").unwrap();
    h.save();
    let (exit, r) = h.run();
    assert_ne!(
        exit, 0,
        "same-ID executable aliases must fail; status={}",
        r["status"]
    );
    assert_collision(&r, &["candidate"]);
}
fn assert_collision(r: &Value, endpoints: &[&str]) {
    assert_eq!(r["judges"][0]["response"]["status"], "pass", "{r:#}");
    assert_eq!(r["judges"][1]["response"]["status"], "fail", "{r:#}");
    let findings = r["findings"].as_array().unwrap();
    assert_eq!(findings.len(), endpoints.len(), "{r:#}");
    for (f, endpoint) in findings.iter().zip(endpoints) {
        assert_eq!(f["code"], "E_NODE_AMBIGUOUS");
        assert!(
            f["message"]
                .as_str()
                .unwrap()
                .starts_with(&format!("{endpoint}:"))
        );
        let reference = f["delta_refs"][0].as_str().unwrap();
        assert!(
            reference == "src.bin"
                || r["impact"]["changes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| c["record"] == reference)
        );
        let causes = f["causes"].as_array().unwrap();
        assert_eq!(causes.last().unwrap(), "test:t");
        assert!(
            r["impact"]["seed_causes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["reference"] == reference && s["node"] == causes[0])
        );
        // Independently specified fixture edges; a direct alias record seed has a zero-edge witness.
        for pair in causes.windows(2) {
            assert!(
                (pair[0] == "file:src.bin" && pair[1] == "project:p")
                    || (pair[0] == "project:p" && pair[1] == "test:t")
                    || (pair[0] == "script:s" && pair[1] == "test:t"),
                "{f:#}"
            );
        }
        let node = &r["impact"]["nodes"]["test:t"];
        assert!(
            node[endpoint].is_null(),
            "ambiguous value must never resolve to a winner"
        );
        let defs = &node[format!("{endpoint}_definitions")];
        assert_eq!(defs.as_array().unwrap().len(), 2);
        assert_eq!(defs[0]["identity"], "project:t");
        assert_eq!(
            defs[0]["value"]["actions"]["execute"]["operation"],
            "execute.t"
        );
        assert_eq!(defs[1]["identity"], "script:t");
        assert_eq!(
            defs[1]["value"]["actions"]["execute"]["operation"],
            "script.st"
        );
        for (record, targets) in [
            ("projects", json!(["project:t", "test:t"])),
            ("scripts", json!(["script:t", "test:t"])),
        ] {
            assert_eq!(
                r["impact"]["records"][format!("/records/projects/{record}/t")][endpoint]["targets"],
                targets
            );
        }
    }
    assert_eq!(
        r["sources"]["/impact"],
        json!(["/judges/1/response/outputs/impact"])
    );
}
#[test]
fn full_distinct_aliases_preserve_both_executables() {
    let mut h = script_host();
    script_pair(&mut h.values, "st2");
    fs::write(h.root().join("src.bin"), "changed").unwrap();
    h.save();
    let (exit, r) = h.run();
    assert_eq!(exit, 0, "{r:#}");
    assert_eq!(
        r["impact"]["nodes"]["test:t"]["candidate"]["actions"]["execute"]["operation"],
        "execute.t"
    );
    assert_eq!(
        r["impact"]["nodes"]["test:st2"]["candidate"]["actions"]["execute"]["operation"],
        "script.st"
    );
    assert_eq!(
        r["impact"]["records"]["/records/projects/projects/t"]["candidate"]["targets"],
        json!(["project:t", "test:t"])
    );
    assert_eq!(
        r["impact"]["records"]["/records/projects/scripts/st2"]["candidate"]["targets"],
        json!(["script:st2", "test:st2"])
    );
    assert_eq!(r["impact"]["tests"], json!(["test:st2", "test:t"]));
    assert_eq!(r["impact"]["retired_tests"], json!(["test:st"]));
}
#[test]
fn full_alias_change_without_source_change_is_causal() {
    let mut h = script_host();
    script_pair(&mut h.values, "t");
    h.save();
    let (exit, r) = h.run();
    assert_eq!(exit, 1, "{r:#}");
    assert_collision(&r, &["candidate"]);
    assert!(
        r["delta"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["path"] == FM || d["path"] == PROJECTS)
    );
}
#[test]
fn full_historical_collision_is_only_judged_when_reached() {
    let mut h = script_host();
    script_pair(&mut h.values, "t");
    h.save();
    h.base = h.candidate.clone();
    for changed in [false, true] {
        if changed {
            fs::write(h.root().join("doc.txt"), "unrelated").unwrap();
            h.candidate = commit(h.root());
        }
        let (exit, r) = h.run();
        assert_eq!(exit, 0, "{r:#}");
        assert_eq!(r["findings"], json!([]));
        assert_eq!(r["impact"]["tests"], json!([]));
        assert_eq!(
            r["impact"]["historical_ambiguities"],
            json!([
                {"node":"test:t","endpoint":"base","definitions":["project:t","script:t"]},
                {"node":"test:t","endpoint":"candidate","definitions":["project:t","script:t"]}
            ])
        );
        for endpoint in ["base_definitions", "candidate_definitions"] {
            assert_eq!(
                r["impact"]["nodes"]["test:t"][endpoint]
                    .as_array()
                    .unwrap()
                    .len(),
                2
            );
        }
    }
    h.base = h.candidate.clone();
    fs::write(h.root().join("src.bin"), "reaches collision").unwrap();
    h.candidate = commit(h.root());
    let (exit, r) = h.run();
    assert_eq!(exit, 1, "{r:#}");
    assert_collision(&r, &["base", "candidate"]);
    for f in r["findings"].as_array().unwrap() {
        assert_eq!(f["delta_refs"], json!(["src.bin"]));
        assert_eq!(f["causes"], json!(["file:src.bin", "project:p", "test:t"]));
    }
}
#[test]
fn full_colliding_project_record_change_preserves_its_alias_cause() {
    let mut h = script_host();
    script_pair(&mut h.values, "t");
    h.save();
    h.base = h.candidate.clone();
    h.values.get_mut(PROJECTS).unwrap()["projects"][1]["actions"]["execute"]["argv"] =
        json!(["-c", "exit 1"]);
    h.save();
    let (exit, r) = h.run();
    assert_eq!(exit, 1, "{r:#}");
    assert_collision(&r, &["base", "candidate"]);
    assert_eq!(r["delta"].as_array().unwrap().len(), 1);
    assert_eq!(r["delta"][0]["path"], PROJECTS);
    for f in r["findings"].as_array().unwrap() {
        assert_eq!(f["delta_refs"], json!(["/records/projects/projects/t"]));
        assert_eq!(f["causes"], json!(["test:t"]));
    }
    // Resolving only the candidate leaves the affected base ambiguity as a fact and finding.
    script_pair(&mut h.values, "st2");
    h.save();
    let (exit, r) = h.run();
    assert_eq!(exit, 1, "{r:#}");
    assert_collision(&r, &["base"]);
    assert_eq!(
        r["impact"]["nodes"]["test:t"]["candidate_definitions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
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
    assert_eq!(r["impact"]["tests"], json!(["test:t"]));
    assert_eq!(r["impact"]["retired_tests"], json!([]));
    assert_eq!(
        r["impact"]["edges"],
        json!([
            {"from":"file:src.bin","kind":"compile","to":"project:p","origin":"both"},
            {"from":"project:p","kind":"test-execution","to":"test:t","origin":"both"}
        ])
    );
    assert_eq!(
        r["impact"]["seeds"],
        json!([
            "file:.chrono-harness/judges.json",
            "file:src.bin",
            "judge:filemap",
            "judge:registration"
        ])
    );
    assert!(
        r["impact"]["seed_causes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["node"] == "file:src.bin"
                && s["reference"] == "src.bin"
                && s["reason"] == "delta-path")
    );
    assert_eq!(
        impact
            .closure
            .witness(
                &impact
                    .seed_causes
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
fn full_report_removed_test_is_selected_without_execution_or_retirement_approval() {
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
    assert_eq!(r["impact"]["tests"], json!([]));
    assert_eq!(r["impact"]["retired_tests"], json!(["test:t"]));
    assert!(
        r["impact"]["nodes"]["test:t"]["candidate_definitions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        r["impact"]["nodes"]["test:t"]["base_definitions"][0]["identity"],
        "project:t"
    );
    assert!(r["impact"]["required_tests"][0].get("approved").is_none());
    assert!(r["impact"]["required_tests"][0].get("executed").is_none());
    assert_eq!(h.values[WORKFLOW]["retirements"], json!([]));
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
