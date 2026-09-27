use chrono_harness::{facts, sha256};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;
struct Host {
    dir: TempDir,
    base: String,
    candidate: String,
}
fn source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn write(root: &Path, path: &str, value: &Value) {
    let p = root.join(path);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn commit(root: &Path) -> String {
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-qm",
            "fixed fixture",
        ],
    );
    git(root, &["rev-parse", "HEAD"])
}
fn file(path: &str) -> Value {
    json!({"path":path,"owner":"repository","surface":"documentation","cost":"unmeasured","edges":[]})
}
impl Host {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("registration host with spaces ")
            .tempdir()
            .unwrap();
        let root = dir.path();
        git(root, &["init", "-q"]);
        fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
        for (project, binary) in [
            ("runner", "chrono-harness"),
            ("judge-registration", "chrono-judge-registration"),
        ] {
            let p = source().join(format!("crates/{project}/target/debug/{binary}"));
            assert!(
                p.is_file(),
                "build registered production binaries first: {}",
                p.display()
            );
            fs::copy(p, root.join(format!(".chrono-harness/bin/{binary}"))).unwrap();
        }
        let mut values = std::collections::BTreeMap::new();
        for name in ["config", "judges", "projects", "FILEMAP", "workflow"] {
            let mut v: Value = serde_json::from_slice(
                &facts::blob(
                    &source(),
                    "4f08aef7ab40d7b0a3fb6ba42af2620600f18d98",
                    &format!(".chrono-harness/{name}.json"),
                )
                .unwrap(),
            )
            .unwrap();
            v["status"] = "active".into();
            values.insert(name, v);
        }
        let c = values.get_mut("config").unwrap();
        c["enforcement"] = "enabled".into();
        c["runner"]["sha256"] =
            sha256(&fs::read(root.join(".chrono-harness/bin/chrono-harness")).unwrap()).into();
        c["input_closure"] = json!({"status":"declared-complete","unresolved":[]});
        c["tools"] = json!([]);
        c["semantic_fields"] = json!([]);
        c["environment"] = json!({"inherit":["PATH"],"values":{},"inputs":[]});
        c["artifacts"] = json!([{"path":".chrono-harness/bin/","owner":"repository","kind":"executable","tracked":false},{"path":".chrono-harness/state/","owner":"repository","kind":"evidence","tracked":false}]);
        let j = values.get_mut("judges").unwrap();
        let mut reg = j["judges"][0].clone();
        reg["sha256"] =
            sha256(&fs::read(root.join(".chrono-harness/bin/chrono-judge-registration")).unwrap())
                .into();
        j["judges"] = json!([reg]);
        j["migration_validator"] = "registration".into();
        let p = values.get_mut("projects").unwrap();
        p["owners"] = json!(["repository"]);
        p["projects"] = json!([]);
        p["scripts"] = json!([]);
        let m = values.get_mut("FILEMAP").unwrap();
        m["files"] = json!([
            file(".gitignore"),
            file("document.txt"),
            file(".chrono-harness/config.json"),
            file(".chrono-harness/judges.json"),
            file(".chrono-harness/projects.json"),
            file(".chrono-harness/FILEMAP.json"),
            file(".chrono-harness/workflow.json")
        ]);
        m["project_edges"] = json!([]);
        m["test_costs"] = json!([]);
        let w = values.get_mut("workflow").unwrap();
        w["stability"] = json!([]);
        w["integration"]["tests"] = json!([]);
        for (name, v) in &values {
            write(root, &format!(".chrono-harness/{name}.json"), v);
        }
        fs::write(
            root.join(".gitignore"),
            ".chrono-harness/bin/\n.chrono-harness/state/\nignored.txt\n",
        )
        .unwrap();
        fs::write(root.join("document.txt"), "base document").unwrap();
        fs::write(
            root.join("historical-unregistered.txt"),
            "unrelated historical omission",
        )
        .unwrap();
        let base = commit(root);
        fs::write(root.join("document.txt"), "candidate document").unwrap();
        let candidate = commit(root);
        Self {
            dir,
            base,
            candidate,
        }
    }
    fn root(&self) -> &Path {
        self.dir.path()
    }
    fn edit(&mut self, path: &str, f: impl FnOnce(&mut Value)) {
        let mut v: Value =
            serde_json::from_slice(&fs::read(self.root().join(path)).unwrap()).unwrap();
        f(&mut v);
        write(self.root(), path, &v);
        self.candidate = commit(self.root());
    }
    fn context(&self) -> Value {
        json!({"schema_version":1,"base":self.base,"candidate":self.candidate,"dev_tip":self.base,"branch_ref":"integration/fixture","fork_point":self.base,"branch_started_at":"2026-01-01T00:00:00Z","observed_at":"2026-01-01T01:00:00Z","operation":"validate.delta","integration_evidence":null})
    }
    fn run_context(&self, context: Value) -> (i32, Value) {
        write(self.root(), ".chrono-harness/state/context.json", &context);
        let out = Command::new(self.root().join(".chrono-harness/bin/chrono-harness"))
            .current_dir("/")
            .env_remove("HOME")
            .args([
                "check",
                "--config",
                self.root()
                    .join(".chrono-harness/config.json")
                    .to_str()
                    .unwrap(),
                "--base",
                &self.base,
                "--candidate",
                &self.candidate,
                "--context",
                ".chrono-harness/state/context.json",
            ])
            .output()
            .unwrap();
        let v = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&out.stderr)}));
        (out.status.code().unwrap(), v)
    }
    fn run(&self) -> (i32, Value) {
        self.run_context(self.context())
    }
}
fn finding(v: &Value, code: &str) -> bool {
    v["judges"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|j| j["response"]["findings"].as_array().into_iter().flatten())
        .any(|f| f["code"] == code)
}
fn assert_report_contract(h: &Host, report: &Value) {
    // Required consumer fields come from SPEC §9, independently of serialization.
    let missing: Vec<_> = [
        "schema_version",
        "status",
        "base",
        "candidate",
        "candidate_tree",
        "context_digest",
        "registry_digest",
        "executables",
        "tools",
        "environment",
        "effective_inputs",
        "parity",
        "delta",
        "impact",
        "judges",
        "tests",
        "findings",
        "costs",
    ]
    .into_iter()
    .filter(|key| report.get(key).is_none())
    .collect();
    assert!(
        missing.is_empty(),
        "SPEC9 missing {missing:?}; base={}, candidate={}",
        h.base,
        h.candidate
    );
    assert_eq!(report["base"], h.base);
    assert_eq!(report["candidate"], h.candidate);
    assert_eq!(report["scope"], "configured-judges");
    assert_eq!(
        report["parity"],
        json!({"status":"unestablished","compared_report":null})
    );
    let stored: Value = serde_json::from_slice(
        &fs::read(h.root().join(".chrono-harness/state/report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(&stored, report);
    for (target, sources) in report["sources"].as_object().unwrap() {
        let actual = report.pointer(target).unwrap();
        for source in sources.as_array().unwrap() {
            let original = report
                .pointer(source.as_str().unwrap())
                .expect("source must resolve in the report");
            if !actual.is_null() {
                assert_eq!(actual, original, "source fidelity for {target}");
            }
        }
    }
}
fn assert_unresolved(report: &Value, field: &str) {
    assert!(
        report.get(field).is_some_and(Value::is_null),
        "{field}: {report:#}"
    );
    assert!(
        report["unresolved"][format!("/{field}")]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "missing reason for {field}: {report:#}"
    );
}
#[test]
fn full_report_required_fields_and_unknowns_for_bounded_host() {
    let h = Host::new();
    let (exit, report) = h.run();
    assert_eq!(exit, 0, "{report:#}");
    assert_report_contract(&h, &report);
    assert_eq!(report["status"], "pass");
    assert_eq!(report["findings"], json!([]));
    assert_eq!(
        report["judges"][0]["response"]["outputs"]["registration"]["scope"],
        "registration-only"
    );
    for field in ["tools", "effective_inputs", "impact", "tests", "costs"] {
        assert_unresolved(&report, field);
    }
}
#[test]
fn full_report_executable_list_distinguishes_observed_and_configured() {
    let h = Host::new();
    let (exit, report) = h.run();
    assert_eq!(exit, 0, "{report:#}");
    let executables = report["executables"].as_array().unwrap_or_else(|| {
        panic!(
            "SPEC9 executables must be a list; base={}, candidate={}; actual={}",
            h.base, h.candidate, report["executables"]
        )
    });
    assert_eq!(executables.len(), 2);
    for (entry, binary) in executables
        .iter()
        .zip(["chrono-harness", "chrono-judge-registration"])
    {
        assert_eq!(entry.as_object().unwrap().len(), 3);
        assert_eq!(
            fs::canonicalize(entry["path"].as_str().unwrap()).unwrap(),
            fs::canonicalize(h.root().join(format!(".chrono-harness/bin/{binary}"))).unwrap()
        );
        assert_eq!(
            entry["sha256"],
            sha256(&fs::read(entry["path"].as_str().unwrap()).unwrap())
        );
    }
    assert_eq!(executables[0]["version"], env!("CARGO_PKG_VERSION"));
    assert!(executables[1]["version"].is_null());
    assert!(
        report["unresolved"]["/executables/1/version"]
            .as_str()
            .unwrap()
            .contains("configured")
    );
    assert_eq!(report["judges"][0]["binding"]["version"], "0.1.0");
    assert_eq!(report["judges"][0]["executable_index"], 1);
}
#[test]
fn full_report_preserves_failure_findings_and_incomplete_status() {
    let mut h = Host::new();
    fs::write(h.root().join("new.txt"), "unregistered").unwrap();
    h.candidate = commit(h.root());
    let (exit, report) = h.run();
    assert_eq!(exit, 1, "{report:#}");
    assert_report_contract(&h, &report);
    assert_eq!(report["status"], "fail");
    assert_eq!(
        report["findings"],
        report["judges"][0]["response"]["findings"]
    );
    assert!(
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "E_UNREGISTERED")
    );
    assert_eq!(
        report["sources"]["/findings/0"],
        json!(["/judges/0/response/findings/0"])
    );
    h.edit(".chrono-harness/config.json", |v| {
        v["status"] = "proposed".into();
        v["input_closure"]["status"] = "incomplete".into();
    });
    let (exit, report) = h.run();
    assert_eq!(exit, 2, "{report:#}");
    assert_report_contract(&h, &report);
    assert_eq!(report["status"], "error");
    assert_eq!(
        report["findings"],
        report["judges"][0]["response"]["findings"]
    );
    assert!(finding(&report, "E_ACTIVATION"));
    assert!(finding(&report, "E_EVIDENCE_UNRESOLVED"));
    for field in ["tools", "effective_inputs", "impact", "tests", "costs"] {
        assert_unresolved(&report, field);
    }
}
fn install_report_fixture(h: &mut Host, script: &str, plan: &[(&str, Vec<&str>)]) {
    use std::os::unix::fs::PermissionsExt;
    let path = ".chrono-harness/bin/report-fixture";
    fs::write(
        h.root().join(path),
        format!("#!/usr/bin/python3\n{script}\n"),
    )
    .unwrap();
    fs::set_permissions(h.root().join(path), fs::Permissions::from_mode(0o755)).unwrap();
    let digest = sha256(&fs::read(h.root().join(path)).unwrap());
    h.edit(".chrono-harness/judges.json", |v| {
        v["judges"] = plan.iter().map(|(id, after)| json!({
            "id":id, "after":after, "executable":path, "sha256":digest,
            "version":"configured-version-only", "argv":[], "selector":"every-delta", "modes":["evaluate"]
        })).collect();
    });
}
#[test]
fn full_report_forwards_real_named_outputs_and_warn_without_inventing_observations() {
    let mut h = Host::new();
    install_report_fixture(
        &mut h,
        r#"
import hashlib,json,subprocess,sys
r=json.load(sys.stdin)
child=subprocess.run([sys.executable,'--version'],capture_output=True)
path='.chrono-harness/state/tool-version.txt'
with open(path,'wb') as f: f.write(child.stdout)
e={'path':path,'sha256':hashlib.sha256(child.stdout).hexdigest(),'kind':'fixture-tool-version'}
outputs={
 'tools':[{'path':sys.executable,'version':child.stdout.decode().strip(),'evidence':[e]}],
 'effective_inputs':{'scope':'fixture-context-only','context':r['context']},
 'impact':{'seeds':['file:'+d['path'] for d in r['delta']],'edges':[],'tests':[],'retired_tests':[]},
 'tests':[{'id':'fixture-version-command','state':'executed','exit_code':child.returncode,'evidence':[e]}],
 'costs':{'declared_before':None,'declared_after':None,'affected_tests':['fixture-version-command'],'retired_tests':[],'unknown':['fixture-version-command'],'measured':None},
 'fixture_extra':{'retained':True}
}
print(json.dumps({'protocol':r['protocol'],'request_id':r['request_id'],'judge_id':r['judge_id'],'status':'warn','findings':[{'code':'W_FIXTURE_COST','level':'warning','message':'fixture costs unmeasured','delta_refs':['/costs'],'causes':[]}],'evidence':[e],'outputs':outputs}))
"#,
        &[("observations", vec![])],
    );
    let (exit, report) = h.run();
    assert_eq!(exit, 0, "{report:#}");
    assert_report_contract(&h, &report);
    assert_eq!(report["status"], "warn");
    assert_eq!(report["judges"][0]["exit_code"], 0);
    for field in ["tools", "effective_inputs", "impact", "tests", "costs"] {
        assert_eq!(
            report[field],
            report["judges"][0]["response"]["outputs"][field]
        );
        assert_eq!(
            report["sources"][format!("/{field}")],
            json!([format!("/judges/0/response/outputs/{field}")])
        );
        assert!(report["unresolved"].get(format!("/{field}")).is_none());
    }
    assert_eq!(report["tests"][0]["state"], "executed");
    assert_eq!(report["tests"][0]["exit_code"], 0);
    let evidence = &report["tests"][0]["evidence"][0];
    let bytes = fs::read(h.root().join(evidence["path"].as_str().unwrap())).unwrap();
    assert_eq!(evidence["sha256"], sha256(&bytes));
    assert_eq!(
        report["tools"][0]["version"],
        String::from_utf8(bytes).unwrap().trim()
    );
    assert_eq!(
        report["effective_inputs"]["context"]["sha256"],
        report["context_digest"]
    );
    assert!(report["costs"]["measured"].is_null());
    assert_eq!(report["findings"][0]["code"], "W_FIXTURE_COST");
    assert_eq!(
        report["judges"][0]["binding"]["version"],
        "configured-version-only"
    );
    assert!(report["executables"][1]["version"].is_null());
    assert_eq!(
        report["judges"][0]["response"]["outputs"]["fixture_extra"]["retained"],
        true
    );
}
#[test]
fn full_report_missing_responses_and_conflicting_outputs_remain_unresolved() {
    let mut h = Host::new();
    install_report_fixture(
        &mut h,
        "import sys\nprint('invalid JSON')\nsys.exit(9)",
        &[("broken", vec![]), ("dependent", vec!["broken"])],
    );
    let (exit, report) = h.run();
    assert_eq!(exit, 2, "{report:#}");
    assert_report_contract(&h, &report);
    assert_eq!(report["status"], "error");
    assert_unresolved(&report, "findings");
    assert_eq!(report["judges"][0]["exit_code"], 9);
    assert_eq!(report["judges"][1]["state"], "blocked");
    assert!(report["judges"][1]["executable_index"].is_null());
    assert_eq!(report["executables"].as_array().unwrap().len(), 2);
    for field in ["tools", "effective_inputs", "impact", "tests", "costs"] {
        assert_unresolved(&report, field);
    }
    // A rejected binding is metadata, never an observed executable.
    h.edit(".chrono-harness/judges.json", |v| {
        v["judges"][0]["sha256"] = "0".repeat(64).into()
    });
    let (exit, report) = h.run();
    assert_eq!(exit, 2, "{report:#}");
    assert_report_contract(&h, &report);
    assert_eq!(report["executables"].as_array().unwrap().len(), 1);
    assert!(
        report["judges"][0]["transport_failure"]
            .as_str()
            .unwrap()
            .contains("digest mismatch")
    );
    assert_unresolved(&report, "findings");
    // Two independent producers cannot be silently resolved by traversal order.
    install_report_fixture(
        &mut h,
        r#"
import json,sys
r=json.load(sys.stdin)
print(json.dumps({'protocol':r['protocol'],'request_id':r['request_id'],'judge_id':r['judge_id'],'status':'pass','findings':[],'evidence':[],'outputs':{'impact':{'seeds':[r['judge_id']]},'tests':None,'costs':{'unknown':['fixture']}}}))
"#,
        &[("first", vec![]), ("second", vec![])],
    );
    let (exit, report) = h.run();
    assert_eq!(exit, 0, "configured statuses preserved: {report:#}");
    assert_report_contract(&h, &report);
    assert_unresolved(&report, "impact");
    assert!(
        report["unresolved"]["/impact"]
            .as_str()
            .unwrap()
            .contains("conflicting")
    );
    assert_unresolved(&report, "tests");
    assert_eq!(report["sources"]["/impact"].as_array().unwrap().len(), 2);
    assert_eq!(report["costs"], json!({"unknown":["fixture"]}));
    assert_eq!(report["sources"]["/costs"].as_array().unwrap().len(), 2);
}
fn paired_host(key: &str) -> Host {
    let mut h = Host::new();
    let (tool, rows) = if key == "scripts" {
        (
            "python3",
            json!([
                {"id":"work","path":"work.py","test_script":"work-test","actions":{"execute":{"operation":"run.work","tool":"python3","argv":["work.py"]}}},
                {"id":"work-test","path":"work-test.py","tests_for":"work","actions":{"execute":{"operation":"test.work","tool":"python3","argv":["work-test.py"]}}}
            ]),
        )
    } else {
        (
            "cargo",
            json!([
                {"id":"work","kind":"production","root":"work","manifest":"work/Cargo.toml","lockfile":"work/Cargo.lock","test_project":"work-test","actions":{"build":{"operation":"build.work","tool":"cargo","argv":["build","--locked","--manifest-path","work/Cargo.toml"]}}},
                {"id":"work-test","kind":"test","root":"work-test","manifest":"work-test/Cargo.toml","lockfile":"work-test/Cargo.lock","tests_for":"work","actions":{"execute":{"operation":"test.work","tool":"cargo","argv":["test","--locked","--manifest-path","work-test/Cargo.toml"]}}}
            ]),
        )
    };
    h.edit(".chrono-harness/config.json", |v| {
        v["tools"] = json!([{"id":tool,"program":tool,"resolution":"PATH-once","version_argv":["--version"],"expected_version":"fixture-version"}]);
    });
    let mut files = Vec::new();
    for row in rows.as_array().unwrap() {
        for field in ["path", "manifest", "lockfile"] {
            if let Some(path) = row[field].as_str() {
                let target = h.root().join(path);
                fs::create_dir_all(target.parent().unwrap()).unwrap();
                // Registration validates these explicit references, not their language contents.
                fs::write(target, "registered fixture input\n").unwrap();
                let mut entry = file(path);
                entry["owner"] = row["id"].clone();
                files.push(entry);
            }
        }
    }
    h.edit(".chrono-harness/projects.json", |v| {
        v["owners"] = json!(["repository", "work", "work-test"]);
        v[key] = rows;
    });
    h.edit(".chrono-harness/FILEMAP.json", |v| {
        v["files"].as_array_mut().unwrap().extend(files);
    });
    let (exit, response) = h.run();
    assert_eq!(exit, 0, "valid paired {key} host: {response:#}");
    assert_eq!(response["judges"][0]["response"]["status"], "pass");
    h.base = h.candidate.clone();
    h
}
fn remove_membership(h: &mut Host, path: &str) {
    h.edit(".chrono-harness/FILEMAP.json", |v| {
        v["files"]
            .as_array_mut()
            .unwrap()
            .retain(|f| f["path"] != path);
    });
}
fn assert_removed_input_wakes_consumer(key: &str, field: &str, path: &str) {
    let mut h = paired_host(key);
    remove_membership(&mut h, path);
    assert!(h.root().join(path).is_file());
    let (exit, response) = h.run();
    assert_eq!(exit, 1, "removed {field} membership: {response:#}");
    assert_eq!(response["judges"][0]["response"]["status"], "fail");
    assert_eq!(response["delta"].as_array().unwrap().len(), 1);
    assert_eq!(response["delta"][0]["path"], ".chrono-harness/FILEMAP.json");
    assert!(
        response["judges"][0]["response"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "E_REFERENCE"
                && f["message"] == format!("missing/unregistered {field}: {path}")),
        "{response:#}"
    );
}
#[test]
fn removed_script_filemap_membership_wakes_unchanged_script() {
    assert_removed_input_wakes_consumer("scripts", "path", "work.py");
}
#[test]
fn removed_manifest_filemap_membership_wakes_unchanged_project() {
    assert_removed_input_wakes_consumer("projects", "manifest", "work/Cargo.toml");
}
#[test]
fn removed_lockfile_filemap_membership_wakes_unchanged_project() {
    assert_removed_input_wakes_consumer("projects", "lockfile", "work/Cargo.lock");
}
#[test]
fn retiring_paired_consumers_and_filemap_memberships_together_is_valid() {
    for key in ["projects", "scripts"] {
        let mut h = paired_host(key);
        h.edit(".chrono-harness/projects.json", |v| {
            v[key] = json!([]);
            v["owners"] = json!(["repository"]);
        });
        h.edit(".chrono-harness/FILEMAP.json", |v| {
            v["files"]
                .as_array_mut()
                .unwrap()
                .retain(|f| f["owner"] == "repository");
        });
        let (exit, response) = h.run();
        assert_eq!(exit, 0, "retired {key}: {response:#}");
        assert_eq!(response["judges"][0]["response"]["status"], "pass");
    }
}
#[test]
fn unrelated_historical_project_and_script_membership_omissions_are_not_rejudged() {
    for (key, path) in [
        ("scripts", "work.py"),
        ("projects", "work/Cargo.toml"),
        ("projects", "work/Cargo.lock"),
    ] {
        let mut h = paired_host(key);
        remove_membership(&mut h, path);
        h.base = h.candidate.clone();
        fs::write(h.root().join("document.txt"), "unrelated later change").unwrap();
        h.candidate = commit(h.root());
        let (exit, response) = h.run();
        assert_eq!(exit, 0, "historical omission {path}: {response:#}");
        assert_eq!(response["judges"][0]["response"]["status"], "pass");
        assert_eq!(response["delta"].as_array().unwrap().len(), 1);
        assert_eq!(response["delta"][0]["path"], "document.txt");
    }
}
#[test]
fn external_registration_passes_bounded_host_without_judging_historical_omission() {
    let h = Host::new();
    let (exit, r) = h.run();
    assert_eq!(exit, 0, "{r:#}");
    assert_eq!(r["judges"][0]["response"]["status"], "pass");
    assert_eq!(
        r["judges"][0]["response"]["outputs"]["registration"]["scope"],
        "registration-only"
    );
    assert_eq!(r["delta"].as_array().unwrap().len(), 1);
}
#[test]
fn changed_unregistered_fails_and_explicit_registration_repairs() {
    let mut h = Host::new();
    fs::write(h.root().join("new.txt"), "changed").unwrap();
    h.candidate = commit(h.root());
    let (exit, r) = h.run();
    assert_eq!(exit, 1, "{r:#}");
    assert!(finding(&r, "E_UNREGISTERED"));
    h.edit(".chrono-harness/FILEMAP.json", |v| {
        v["files"].as_array_mut().unwrap().push(file("new.txt"))
    });
    assert_eq!(h.run().0, 0);
}
#[test]
fn ignored_untracked_staged_and_wrong_checkout_fail() {
    for kind in ["ignored", "untracked", "staged", "unstaged", "head"] {
        let h = Host::new();
        match kind {
            "ignored" => fs::write(h.root().join("ignored.txt"), "dirt").unwrap(),
            "untracked" => fs::write(h.root().join("new.txt"), "dirt").unwrap(),
            "staged" => {
                fs::write(h.root().join("document.txt"), "dirt").unwrap();
                git(h.root(), &["add", "document.txt"]);
                fs::write(h.root().join("document.txt"), "candidate document").unwrap();
            }
            "unstaged" => fs::write(h.root().join("document.txt"), "dirt").unwrap(),
            _ => {
                git(h.root(), &["checkout", "-q", &h.base]);
            }
        }
        let (exit, r) = h.run();
        assert_ne!(exit, 0, "{kind}");
        assert!(finding(&r, "E_SNAPSHOT_DIRTY"), "{kind}: {r:#}");
    }
}
#[test]
fn context_and_config_identity_mismatch() {
    let h = Host::new();
    let mut c = h.context();
    c["candidate"] = h.base.clone().into();
    let (exit, r) = h.run_context(c);
    assert_eq!(exit, 2);
    assert!(finding(&r, "E_CONTEXT_MISMATCH"), "{r:#}");
    let p = h.root().join(".chrono-harness/config.json");
    let mut bytes = fs::read(&p).unwrap();
    bytes.push(b' ');
    fs::write(&p, bytes).unwrap();
    let (exit, r) = h.run();
    assert_eq!(exit, 2);
    assert!(finding(&r, "E_CONFIG_MISMATCH"), "{r:#}");
}
#[test]
fn strict_five_schemas_duplicate_keys_ids_and_unknown_fields() {
    for name in ["config", "judges", "projects", "FILEMAP", "workflow"] {
        let mut h = Host::new();
        h.edit(&format!(".chrono-harness/{name}.json"), |v| {
            v["unknown"] = true.into();
        });
        let (exit, r) = h.run();
        assert_eq!(exit, 2);
        assert!(finding(&r, "E_SCHEMA"), "{name}: {r:#}");
    }
    let mut h = Host::new();
    h.edit(".chrono-harness/FILEMAP.json", |v| {
        let row = v["files"][0].clone();
        v["files"].as_array_mut().unwrap().push(row);
    });
    assert!(finding(&h.run().1, "E_SCHEMA"));
    let mut h = Host::new();
    let p = h.root().join(".chrono-harness/projects.json");
    let raw = fs::read_to_string(&p).unwrap().replacen(
        "\"status\":",
        "\"status\":\"active\",\"status\":",
        1,
    );
    fs::write(p, raw).unwrap();
    h.candidate = commit(h.root());
    let (exit, r) = h.run();
    assert_eq!(exit, 2);
    assert!(r.to_string().contains("duplicate JSON"));
}
#[test]
fn changed_references_fail_and_unrelated_historical_ones_are_not_rejudged() {
    let mut h = Host::new();
    h.edit(".chrono-harness/FILEMAP.json", |v| {
        v["files"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["path"] == "document.txt")
            .unwrap()["edges"] = json!([{"kind":"test-execution","to":"test:missing"}]);
    });
    let (exit, r) = h.run();
    assert_eq!(exit, 1);
    assert!(finding(&r, "E_DANGLING_EDGE"), "{r:#}");
}
#[test]
fn proposed_and_incomplete_never_receive_governance_success() {
    let mut h = Host::new();
    h.edit(".chrono-harness/config.json", |v| {
        v["status"] = "proposed".into();
        v["input_closure"]["status"] = "incomplete".into();
    });
    let (exit, r) = h.run();
    assert_eq!(exit, 2);
    assert!(finding(&r, "E_ACTIVATION"));
    assert!(finding(&r, "E_EVIDENCE_UNRESOLVED"));
}
#[test]
fn real_host_pseudo_script_and_tool_defects_are_concrete() {
    let mut h = Host::new();
    let host: Value = serde_json::from_slice(
        &chrono_harness::facts::blob(
            &source(),
            "4f08aef7ab40d7b0a3fb6ba42af2620600f18d98",
            ".chrono-harness/projects.json",
        )
        .unwrap(),
    )
    .unwrap();
    h.edit(".chrono-harness/projects.json", |v| {
        v["scripts"] = host["scripts"].clone()
    });
    let (exit, r) = h.run();
    assert_eq!(exit, 2);
    assert!(finding(&r, "E_SCHEMA"), "{r:#}");
    assert!(r.to_string().contains("path") || r.to_string().contains("kind"));
    // Repair only structure in the synthetic fixture, exposing the separate undeclared tool defect.
    fs::write(h.root().join("verify.py"), "fixture").unwrap();
    fs::write(h.root().join("verify-tests.py"), "fixture").unwrap();
    h.edit(".chrono-harness/projects.json",|v|v["scripts"]=json!([{"id":"ci-verify","path":"verify.py","test_script":"verify-tests","actions":host["scripts"][0]["actions"]},{"id":"verify-tests","path":"verify-tests.py","tests_for":"ci-verify","actions":{"execute":{"operation":"test.verify","tool":"chrono-ci","argv":[]}}}]));
    h.edit(".chrono-harness/FILEMAP.json", |v| {
        v["files"]
            .as_array_mut()
            .unwrap()
            .extend([file("verify.py"), file("verify-tests.py")])
    });
    let (exit, r) = h.run();
    assert_eq!(exit, 1, "{r:#}");
    assert!(finding(&r, "E_REFERENCE"));
    assert!(r.to_string().contains("undeclared tool"));
}
#[test]
fn actual_host_registries_cannot_pass_full_governance() {
    let mut h = Host::new();
    for name in ["config", "projects", "FILEMAP", "workflow"] {
        let bytes = fs::read(source().join(format!(".chrono-harness/{name}.json"))).unwrap();
        fs::write(h.root().join(format!(".chrono-harness/{name}.json")), bytes).unwrap();
    }
    h.candidate = commit(h.root());
    h.base = h.candidate.clone();
    fs::write(
        h.root().join("document.txt"),
        "current host negative closure",
    )
    .unwrap();
    h.candidate = commit(h.root());
    let (exit, r) = h.run();
    assert_eq!(exit, 2);
    assert_report_contract(&h, &r);
    for field in ["tools", "effective_inputs", "impact", "tests", "costs"] {
        assert_unresolved(&r, field);
    }
    assert_eq!(r["findings"], r["judges"][0]["response"]["findings"]);
    assert!(finding(&r, "E_ACTIVATION"), "{r:#}");
}
#[test]
fn rename_is_delete_add_and_no_delta_still_checks_inputs() {
    let mut h = Host::new();
    fs::rename(h.root().join("document.txt"), h.root().join("renamed.txt")).unwrap();
    h.edit(".chrono-harness/FILEMAP.json", |v| {
        let rows = v["files"].as_array_mut().unwrap();
        rows.retain(|f| f["path"] != "document.txt");
        rows.push(file("renamed.txt"));
    });
    let (exit, r) = h.run();
    assert_eq!(exit, 0, "{r:#}");
    assert!(
        r["delta"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["kind"] == "D" && d["path"] == "document.txt")
    );
    assert!(
        r["delta"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["kind"] == "A" && d["path"] == "renamed.txt")
    );
    h.base = h.candidate.clone();
    assert_eq!(h.run().1["delta"], json!([]));
    fs::write(h.root().join("ignored.txt"), "dirt").unwrap();
    assert!(finding(&h.run().1, "E_SNAPSHOT_DIRTY"));
}
#[test]
fn missing_oid_and_symbolic_endpoint_are_errors() {
    let h = Host::new();
    assert!(facts::verify_oid(h.root(), "HEAD").is_err());
    assert!(facts::verify_oid(h.root(), &"a".repeat(40)).is_err());
}

#[test]
fn hidden_index_flags_and_submodules_are_visible_errors() {
    for flag in ["--assume-unchanged", "--skip-worktree"] {
        let h = Host::new();
        git(h.root(), &["update-index", flag, "document.txt"]);
        fs::write(h.root().join("document.txt"), "hidden dirt").unwrap();
        let before = git(h.root(), &["ls-files", "-v"]);
        let (exit, r) = h.run();
        assert_eq!(exit, 2);
        assert!(finding(&r, "E_INPUT_UNSUPPORTED"), "{r:#}");
        assert_eq!(git(h.root(), &["ls-files", "-v"]), before);
    }
    let mut h = Host::new();
    git(
        h.root(),
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{},module", h.base),
        ],
    );
    git(
        h.root(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-qm",
            "submodule fixture",
        ],
    );
    h.candidate = git(h.root(), &["rev-parse", "HEAD"]);
    let (exit, r) = h.run();
    assert_eq!(exit, 2);
    assert!(finding(&r, "E_INPUT_UNSUPPORTED"), "{r:#}");
}
#[test]
fn removed_reference_targets_wake_unchanged_consumers() {
    let mut h = Host::new();
    h.edit(".chrono-harness/FILEMAP.json", |v| {
        v["files"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["path"] == "document.txt")
            .unwrap()["edges"] = json!([{"kind":"runtime-input","to":"file:.gitignore"}])
    });
    h.base = h.candidate.clone();
    h.edit(".chrono-harness/FILEMAP.json", |v| {
        v["files"]
            .as_array_mut()
            .unwrap()
            .retain(|f| f["path"] != ".gitignore")
    });
    let (exit, r) = h.run();
    assert_eq!(exit, 1);
    assert!(finding(&r, "E_DANGLING_EDGE"), "{r:#}");
}

#[test]
fn base_executable_binding_is_data_only() {
    let mut h = Host::new();
    let original: Value =
        serde_json::from_slice(&fs::read(h.root().join(".chrono-harness/judges.json")).unwrap())
            .unwrap();
    h.edit(".chrono-harness/judges.json", |v| {
        v["judges"][0]["executable"] = ".chrono-harness/bin/nonexistent-old-judge".into();
        v["judges"][0]["sha256"] = "a".repeat(64).into();
    });
    h.base = h.candidate.clone();
    h.edit(".chrono-harness/judges.json", |v| *v = original);
    let (exit, r) = h.run();
    assert_eq!(exit, 0, "{r:#}");
    assert_eq!(r["judges"][0]["response"]["judge_id"], "registration");
}

#[test]
fn unrelated_historical_dangling_record_is_not_rejudged() {
    let mut h = Host::new();
    h.edit(".chrono-harness/FILEMAP.json", |v| {
        let mut f = file("historical-unregistered.txt");
        f["edges"] = json!([{"kind":"runtime-input","to":"file:historical-missing"}]);
        v["files"].as_array_mut().unwrap().push(f);
    });
    h.base = h.candidate.clone();
    fs::write(h.root().join("document.txt"), "new bounded delta").unwrap();
    h.candidate = commit(h.root());
    let (exit, r) = h.run();
    assert_eq!(exit, 0, "{r:#}");
}
