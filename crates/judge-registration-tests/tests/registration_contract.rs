use chrono_harness::{facts, sha256};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

#[test]
fn retained_input_duplicates_check_each_descriptor_and_refresh_on_each_validation() {
    use chrono_judge_registration::{Registrations, inputs};
    let h = Host::new();
    let root = fs::canonicalize(h.root()).unwrap();
    let config_path = ".chrono-harness/config.json";
    let mut values = std::collections::BTreeMap::new();
    for name in ["config", "judges", "projects", "FILEMAP", "workflow"] {
        let path = format!(".chrono-harness/{name}.json");
        values.insert(
            path.clone(),
            chrono_harness::json(&fs::read(root.join(path)).unwrap()).unwrap(),
        );
    }
    let live = root.join(".chrono-harness/state/live-input");
    let blob = ".chrono-harness/state/retained-input";
    fs::create_dir_all(live.parent().unwrap()).unwrap();
    let original = b"original";
    fs::write(&live, original).unwrap();
    fs::write(root.join(blob), original).unwrap();
    let config = values.get_mut(config_path).unwrap();
    config["environment"] = json!({"inherit":[],"values":{},"inputs":[{"id":"data","location":live,"sha256":sha256(original)}]});
    let digest = chrono_harness::wire::digest(config).unwrap();
    let r = Registrations::load(&values, config_path).unwrap();
    let snapshot = |commit: &str| json!({"schema":"chrono-input-snapshot/v1","commit":commit,"config_path":config_path,"config_digest":digest,"environment":{},"files":{"data":{"blob":blob,"sha256":sha256(original),"length":original.len()}}});
    let req: chrono_harness::wire::Request = serde_json::from_value(json!({
        "protocol":"chrono-judge/v1","request_id":"","judge_id":"registration","mode":"evaluate",
        "base":{"commit":h.base,"tree":"c".repeat(40),"root":root},
        "candidate":{"commit":h.candidate,"tree":"d".repeat(40),"root":root},
        "delta":[],"registries":{"base":root,"candidate":root,"digest":"e".repeat(64)},
        "context":{"path":root.join("context.json"),"sha256":"f".repeat(64)},
        "impact":{"seeds":[],"edges":[],"tests":[],"retired_tests":[]},"prior_results":[],
        "config_path":config_path,"checkout":{"head":h.candidate,"tracked":[],"untracked":[],"index_flags":[]},
        "runner":{"path":"runner","sha256":"e".repeat(64),"version":"0.1.0"},
        "observations":{"retained":{"base":snapshot(&h.base),"candidate":snapshot(&h.candidate)},"environment":{"inherited":{},"effective":{}}}
    })).unwrap();
    let validate = |request: &chrono_harness::wire::Request| inputs::validate(request, &r, &r);
    assert!(validate(&req).is_ok());
    let originals = json!({blob:{"schema":"chrono-retained-blob/v1","storage":blob,"sha256":sha256(original),"length":original.len()}});
    let validate_original = |request: &chrono_harness::wire::Request| {
        inputs::validate_retained(request, &r, &r, &originals)
    };
    assert!(validate_original(&req).is_ok());
    // The base reference is observed first; the candidate points at that same path.
    for field in ["sha256", "length"] {
        let mut bad = req.clone();
        bad.observations["retained"]["candidate"]["files"]["data"][field] = if field == "sha256" {
            json!("0".repeat(64))
        } else {
            json!(original.len() + 1)
        };
        let error = validate(&bad).unwrap_err();
        assert!(
            error.contains("candidate: retained input data") && error.contains("identity mismatch"),
            "{field}: {error}"
        );
        let error = validate_original(&bad).unwrap_err();
        assert!(
            error.contains("candidate: retained input data")
                && error.contains("original input differs"),
            "{field}: {error}"
        );
    }
    let other = ".chrono-harness/state/different-input";
    fs::write(root.join(other), b"corrupt!").unwrap();
    let mut bad = req.clone();
    bad.observations["retained"]["candidate"]["files"]["data"]["blob"] = json!(other);
    assert!(validate(&bad).unwrap_err().contains("identity mismatch"));
    // A completed validation never supplies observations to the next call.
    fs::write(root.join(blob), b"corrupt!").unwrap();
    assert!(validate(&req).unwrap_err().contains("identity mismatch"));
    assert!(
        validate_original(&req)
            .unwrap_err()
            .contains("digest/length")
    );
    fs::remove_file(root.join(blob)).unwrap();
    assert!(validate(&req).is_err());
    assert!(validate_original(&req).is_err());
    std::os::unix::fs::symlink(&live, root.join(blob)).unwrap();
    assert!(validate(&req).unwrap_err().contains("symlink"));
    assert!(validate_original(&req).unwrap_err().contains("symlink"));
    fs::remove_file(root.join(blob)).unwrap();
    fs::write(root.join(blob), original).unwrap();
    assert!(validate(&req).is_ok());
    assert!(validate_original(&req).is_ok());
    fs::write(&live, b"corrupt!").unwrap();
    assert!(
        validate(&req)
            .unwrap_err()
            .contains("candidate input changed: data")
    );
}
#[path = "git_facts.rs"]
mod git_facts;
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
fn bind_actual_git(config: &mut Value, root: &Path) {
    let path = chrono_harness::resolve_program(root, "git", None).unwrap();
    let version = Command::new(&path).arg("--version").output().unwrap();
    assert!(
        version.status.success(),
        "git --version failed: {version:?}"
    );
    let program = path.to_str().unwrap().to_owned();
    let expected_version = String::from_utf8(version.stdout)
        .unwrap()
        .trim_end()
        .to_owned();
    let digest = sha256(&fs::read(&path).unwrap());

    let tool = config["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|tool| tool["id"] == "git")
        .expect("product config must retain its Git tool");
    tool["program"] = program.clone().into();
    tool["expected_version"] = expected_version.into();
    let input = config["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|input| input["id"] == "git-executable")
        .expect("product config must retain its Git executable input");
    input["location"] = program.into();
    input["sha256"] = digest.into();
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
        let v = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
            json!({
                "stdout": String::from_utf8_lossy(&out.stdout),
                "stderr": String::from_utf8_lossy(&out.stderr)
            })
        });
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

fn initial_host() -> Host {
    let mut h = Host::new();
    git(h.root(), &["checkout", "--orphan", "initial-fixture"]);
    for name in ["config", "judges", "projects", "FILEMAP", "workflow"] {
        let path = format!(".chrono-harness/{name}.json");
        let mut v: Value =
            serde_json::from_slice(&fs::read(h.root().join(&path)).unwrap()).unwrap();
        v["status"] = json!("proposed");
        if name == "config" {
            v["enforcement"] = json!("not-implemented");
            v["input_closure"] = json!({"status":"incomplete","unresolved":["toolchain"]});
        }
        if name == "FILEMAP" {
            v["files"].as_array_mut().unwrap().extend([
                file("historical-unregistered.txt"),
                file(".chrono-harness/initial.json"),
            ]);
        }
        write(h.root(), &path, &v);
    }
    let binary = h
        .root()
        .join(".chrono-harness/bin/chrono-judge-registration");
    write(
        h.root(),
        ".chrono-harness/initial.json",
        &json!({
            "schema":"chrono-initial-check/v1", "host_config":".chrono-harness/config.json",
            "timeout_seconds":30, "stdout_limit_bytes":1048576,
            "judges":[{"id":"initial-registration","executable":".chrono-harness/bin/chrono-judge-registration",
                "sha256":sha256(&fs::read(binary).unwrap()),"version":"0.1.0",
                "argv":["--protocol","chrono-initial-judge/v1"],"selector":"every-initial","after":[],"modes":["inventory"]}]
        }),
    );
    h.candidate = commit(h.root());
    h
}

fn initial_run(h: &Host) -> (i32, Value) {
    let o = Command::new(h.root().join(".chrono-harness/bin/chrono-harness"))
        .current_dir("/")
        .env_remove("HOME")
        .args([
            "check",
            "--config",
            h.root()
                .join(".chrono-harness/initial.json")
                .to_str()
                .unwrap(),
            "--candidate",
            &h.candidate,
            "--initial",
        ])
        .output()
        .unwrap();
    let report = serde_json::from_slice(&o.stdout)
        .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&o.stderr)}));
    (o.status.code().unwrap(), report)
}

fn amend_initial(h: &mut Host) {
    git(h.root(), &["add", "."]);
    git(
        h.root(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--amend",
            "--no-gpg-sign",
            "-qm",
            "fixed initial fixture",
        ],
    );
    h.candidate = git(h.root(), &["rev-parse", "HEAD"]);
}

#[test]
fn initial_inventory_checks_semantic_field_references_without_old_history() {
    let mut h = initial_host();
    let path = ".chrono-harness/config.json";
    let mut cfg: Value = serde_json::from_slice(&fs::read(h.root().join(path)).unwrap()).unwrap();
    cfg["semantic_fields"] =
        json!([{"path":"missing.json","pointers":["/policy"],"on":"add-modify-delete"}]);
    write(h.root(), path, &cfg);
    amend_initial(&mut h);
    let (exit, r) = initial_run(&h);
    assert_ne!(exit, 0, "{r}");
    assert!(finding(&r, "E_REFERENCE"), "{r}");
}

#[test]
fn initial_inventory_reads_original_parents_through_git_replace_overlays() {
    let mut h = initial_host();
    let root = h.candidate.clone();
    git(
        h.root(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--allow-empty",
            "--no-gpg-sign",
            "-qm",
            "same tree child",
        ],
    );
    h.candidate = git(h.root(), &["rev-parse", "HEAD"]);
    git(h.root(), &["replace", &h.candidate, &root]);
    assert!(
        !git(h.root(), &["cat-file", "-p", &h.candidate])
            .lines()
            .take_while(|l| !l.is_empty())
            .any(|l| l.starts_with("parent "))
    );
    let (exit, r) = initial_run(&h);
    assert_ne!(exit, 0, "{r}");
    assert!(r["stderr"].as_str().unwrap().contains("parentless"), "{r}");
    assert!(
        !h.root()
            .join(".chrono-harness/state/initial-report.json")
            .exists()
    );
}

#[test]
fn initial_inventory_uses_only_declared_dag_and_blocks_failed_dependents() {
    use std::os::unix::fs::PermissionsExt;
    let mut h = initial_host();
    let script = ".chrono-harness/inventory-plugin.py";
    let text = r#"#!/usr/bin/python3
import json, pathlib, sys
r = json.load(sys.stdin)
assert 'base' not in r and 'delta' not in r
if r['judge_id'] == 'dependent':
    assert len(r['prior_results']) == 1
    assert r['prior_results'][0]['outputs']['inventory']['scope'] == 'schema-references-checkout'
else:
    assert r['prior_results'] == []
p = pathlib.Path('.chrono-harness/state')
p.mkdir(parents=True, exist_ok=True)
(p / (r['judge_id'] + '-ran')).write_text('ran')
print(json.dumps({'protocol':r['protocol'],'request_id':r['request_id'],'judge_id':r['judge_id'],
 'status':'pass','findings':[],'evidence':[],'outputs':{'plugin':True}}))
"#;
    fs::write(h.root().join(script), text).unwrap();
    fs::set_permissions(h.root().join(script), fs::Permissions::from_mode(0o755)).unwrap();
    let fm = ".chrono-harness/FILEMAP.json";
    let mut map: Value = serde_json::from_slice(&fs::read(h.root().join(fm)).unwrap()).unwrap();
    map["files"].as_array_mut().unwrap().push(file(script));
    write(h.root(), fm, &map);
    let path = ".chrono-harness/initial.json";
    let mut profile: Value =
        serde_json::from_slice(&fs::read(h.root().join(path)).unwrap()).unwrap();
    for (id, after) in [
        ("dependent", vec!["initial-registration"]),
        ("independent", vec![]),
    ] {
        profile["judges"].as_array_mut().unwrap().push(json!({"id":id,"executable":script,"version":"fixture",
            "sha256":sha256(text.as_bytes()),"argv":[],"selector":"every-initial","after":after,"modes":["inventory"]}));
    }
    write(h.root(), path, &profile);
    amend_initial(&mut h);
    let (exit, r) = initial_run(&h);
    assert_eq!(exit, 0, "{r}");
    let records = r["judges"].as_array().unwrap();
    assert_eq!(records.len(), 3);
    assert!(
        records
            .iter()
            .all(|r| r["state"] == "executed" && r["process"]["exit_code"] == 0)
    );
    for id in ["dependent", "independent"] {
        let path = h.root().join(format!(".chrono-harness/state/{id}-ran"));
        assert_eq!(fs::read(&path).unwrap(), b"ran");
        fs::remove_file(path).unwrap();
    }
    fs::write(h.root().join("unregistered.txt"), "break first judge").unwrap();
    amend_initial(&mut h);
    let (exit, r) = initial_run(&h);
    assert_ne!(exit, 0, "{r}");
    assert!(finding(&r, "E_UNREGISTERED"), "{r}");
    let records = r["judges"].as_array().unwrap();
    let dependent = records.iter().find(|r| r["id"] == "dependent").unwrap();
    assert_eq!(dependent["state"], "blocked");
    assert_eq!(dependent["blocked_by"], json!(["initial-registration"]));
    assert!(dependent["process"].is_null());
    assert!(
        !h.root()
            .join(".chrono-harness/state/dependent-ran")
            .exists()
    );
    assert_eq!(
        fs::read(h.root().join(".chrono-harness/state/independent-ran")).unwrap(),
        b"ran"
    );
}

#[test]
fn initial_inventory_rejects_unregistered_dangling_and_malformed_declarations() {
    for case in [
        "unregistered",
        "dangling",
        "missing-file",
        "schema",
        "activation",
    ] {
        let mut h = initial_host();
        let mut path = ".chrono-harness/FILEMAP.json";
        let mut value: Value =
            serde_json::from_slice(&fs::read(h.root().join(path)).unwrap()).unwrap();
        let expected = match case {
            "unregistered" => {
                fs::write(h.root().join("extra.txt"), "not registered").unwrap();
                "E_UNREGISTERED"
            }
            "dangling" => {
                value["project_edges"] =
                    json!([{"from":"file:document.txt","kind":"compile","to":"project:missing"}]);
                "E_DANGLING_EDGE"
            }
            "missing-file" => {
                value["files"]
                    .as_array_mut()
                    .unwrap()
                    .push(file("missing.txt"));
                "E_REFERENCE"
            }
            "schema" => {
                value["unknown"] = json!(true);
                "E_INITIAL_INPUT"
            }
            "activation" => {
                path = ".chrono-harness/config.json";
                value = serde_json::from_slice(&fs::read(h.root().join(path)).unwrap()).unwrap();
                value["status"] = json!("active");
                value["enforcement"] = json!("enabled");
                "E_INITIAL_INPUT"
            }
            _ => unreachable!(),
        };
        write(h.root(), path, &value);
        amend_initial(&mut h);
        let (exit, r) = initial_run(&h);
        assert_ne!(exit, 0, "{case}: {r}");
        assert!(finding(&r, expected), "{case}: {r}");
        assert_ne!(r["status"], "complete");
    }
}

#[test]
fn initial_inventory_rejects_dirty_hidden_and_wrong_checkouts() {
    for case in ["tracked", "ignored", "assume-unchanged", "head"] {
        let h = initial_host();
        match case {
            "tracked" => fs::write(h.root().join("document.txt"), "dirty").unwrap(),
            "ignored" => fs::write(h.root().join("ignored.txt"), "dirty").unwrap(),
            "assume-unchanged" => {
                git(
                    h.root(),
                    &["update-index", "--assume-unchanged", "document.txt"],
                );
            }
            "head" => {
                git(h.root(), &["update-ref", "HEAD", &h.base]);
            }
            _ => unreachable!(),
        }
        let (exit, r) = initial_run(&h);
        assert_ne!(exit, 0, "{case}: {r}");
        assert!(finding(&r, "E_INITIAL_INPUT"), "{case}: {r}");
        assert_ne!(r["status"], "complete");
    }
}

#[test]
fn initial_inventory_rejects_real_parents_even_in_shallow_history_and_cannot_activate_delta() {
    let mut h = initial_host();
    h.base = h.candidate.clone();
    fs::write(h.root().join("document.txt"), "later change").unwrap();
    h.candidate = commit(h.root());
    let (exit, r) = initial_run(&h);
    assert_ne!(exit, 0, "{r}");
    assert!(r["stderr"].as_str().unwrap().contains("parentless"), "{r}");
    let (exit, r) = h.run();
    assert_ne!(exit, 0, "{r}");
    assert!(finding(&r, "E_ACTIVATION"), "{r}");
    let dir = tempfile::tempdir().unwrap();
    let clone = dir.path().join("shallow host");
    git(
        dir.path(),
        &[
            "clone",
            "--quiet",
            "--depth",
            "1",
            &format!("file://{}", h.root().display()),
            clone.to_str().unwrap(),
        ],
    );
    assert_eq!(git(&clone, &["rev-list", "--count", "HEAD"]), "1");
    fs::create_dir_all(clone.join(".chrono-harness/bin")).unwrap();
    for binary in ["chrono-harness", "chrono-judge-registration"] {
        fs::copy(
            h.root().join(format!(".chrono-harness/bin/{binary}")),
            clone.join(format!(".chrono-harness/bin/{binary}")),
        )
        .unwrap();
    }
    let o = Command::new(clone.join(".chrono-harness/bin/chrono-harness"))
        .current_dir(&clone)
        .args([
            "check",
            "--config",
            ".chrono-harness/initial.json",
            "--candidate",
            &h.candidate,
            "--initial",
        ])
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("parentless"));
    assert!(
        !clone
            .join(".chrono-harness/state/initial-report.json")
            .exists()
    );
}

#[test]
fn explicit_initial_inventory_runs_candidate_judge_without_a_delta_or_activation_claim() {
    let h = initial_host();
    let (exit, r) = initial_run(&h);
    assert_eq!(exit, 0, "{r}");
    assert_eq!(r["scope"], "initial-inventory");
    assert_eq!(r["status"], "complete");
    assert!(r["base"].is_null() && r["delta"].is_null());
    assert_eq!(r["candidate"], h.candidate);
    assert_eq!(r["governance"], "not-evaluated");
    assert_eq!(r["previous_enforcement"], "none");
    assert_eq!(r["judges"][0]["process"]["exit_code"], 0);
    assert_eq!(
        r["judges"][0]["response"]["outputs"]["inventory"]["scope"],
        "schema-references-checkout"
    );
    let stored: Value = serde_json::from_slice(
        &fs::read(h.root().join(".chrono-harness/state/initial-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(r, stored);
    let cfg: Value =
        serde_json::from_slice(&fs::read(h.root().join(".chrono-harness/config.json")).unwrap())
            .unwrap();
    assert_eq!(cfg["status"], "proposed");
    assert_eq!(cfg["input_closure"]["status"], "incomplete");
}

#[test]
fn selected_initial_inventory_keeps_entry_identity_and_runs_the_effective_policy() {
    let mut h = initial_host();
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let selector = ".chrono-harness/platforms.json";
    let git_path = "/usr/bin/git";
    let git_version = String::from_utf8(
        Command::new(git_path)
            .arg("--version")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim_end()
    .to_string();
    let mut config: Value =
        serde_json::from_slice(&fs::read(h.root().join(".chrono-harness/config.json")).unwrap())
            .unwrap();
    config["schema_version"] = 3.into();
    config["facts_git"] = json!({"tool":"git","input":"git-binary"});
    config["tools"] = json!([{"id":"git","program":git_path,"resolution":"PATH-once","version_argv":["--version"],"expected_version":git_version}]);
    config["environment"]["inputs"] = json!([{"id":"git-binary","location":git_path,"presence":"present","sha256":sha256(&fs::read(git_path).unwrap())}]);
    config["canonical_check"]["argv"][3] = selector.into();
    write(h.root(), ".chrono-harness/config.json", &config);
    write(
        h.root(),
        selector,
        &json!({"schema":"chrono-git-configs/v1","platforms":{platform: ".chrono-harness/config.json"}}),
    );
    let profile_path = ".chrono-harness/initial.json";
    let mut profile: Value =
        serde_json::from_slice(&fs::read(h.root().join(profile_path)).unwrap()).unwrap();
    profile["host_config"] = selector.into();
    write(h.root(), profile_path, &profile);
    let filemap_path = ".chrono-harness/FILEMAP.json";
    let mut filemap: Value =
        serde_json::from_slice(&fs::read(h.root().join(filemap_path)).unwrap()).unwrap();
    filemap["files"]
        .as_array_mut()
        .unwrap()
        .push(file(selector));
    write(h.root(), filemap_path, &filemap);
    amend_initial(&mut h);

    let (exit, report) = initial_run(&h);
    assert_eq!(exit, 0, "{report}");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["git_facts"]["selection"]["path"], selector);
    assert_eq!(
        report["git_facts"]["selection"]["config_path"],
        ".chrono-harness/config.json"
    );
    assert_eq!(report["judges"][0]["response"]["status"], "pass");
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
fn input_closure_bindings_are_v3_only() {
    for schema_version in [1, 2] {
        let mut h = Host::new();
        h.edit(".chrono-harness/config.json", |v| {
            v["schema_version"] = schema_version.into();
            v["input_closure"]["bindings"] = json!([{
                "id": "legacy-closure",
                "consumer": "judge:registration",
                "kind": "toolchain",
                "inputs": ["tool:python"]
            }]);
        });
        let (exit, r) = h.run();
        assert_eq!(exit, 2, "schema {schema_version}: {r:#}");
        assert!(finding(&r, "E_SCHEMA"), "schema {schema_version}: {r:#}");
    }
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
    let explicit_config: Value =
        serde_json::from_slice(&fs::read(h.root().join(".chrono-harness/config.json")).unwrap())
            .unwrap();
    for name in ["config", "projects", "FILEMAP", "workflow"] {
        let bytes = fs::read(source().join(format!(".chrono-harness/{name}.json"))).unwrap();
        fs::write(h.root().join(format!(".chrono-harness/{name}.json")), bytes).unwrap();
    }
    // This host supplies its own fixed endpoints/context. Register that explicit
    // v3 invocation while retaining the current host's proposed governance data;
    // v4's short entry requires its registered automatic input producers.
    let config_path = ".chrono-harness/config.json";
    let mut config: Value =
        serde_json::from_slice(&fs::read(h.root().join(config_path)).unwrap()).unwrap();
    config["schema_version"] = json!(3);
    config["canonical_check"] = explicit_config["canonical_check"].clone();
    config["environment"]
        .as_object_mut()
        .unwrap()
        .remove("credential_environment");
    bind_actual_git(&mut config, h.root());
    write(h.root(), config_path, &config);
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

#[test]
fn explicit_same_project_group_bindings_and_legacy_scripts_have_one_owner() {
    use chrono_judge_registration::execution::test_bindings;
    let legacy = json!({"projects":[{"id":"t","kind":"test","actions":{"execute":{"operation":"old"},"alpha":{"operation":"a"},"beta":{"operation":"b"}}}],"scripts":[{"id":"s","tests_for":"producer","actions":{"execute":{"operation":"script"}}}]});
    let bindings = test_bindings(&legacy).unwrap();
    assert_eq!(bindings["test:t"][0].operation, "old");
    assert_eq!(bindings["test:s"][0].owner, "script:s");
    let mut grouped = legacy.clone();
    grouped["projects"][0]["test_groups"] = json!({"t":"alpha","opaque/suffix":"beta"});
    let bindings = test_bindings(&grouped).unwrap();
    assert_eq!(bindings["test:t"][0].operation, "a");
    assert_eq!(bindings["test:opaque/suffix"][0].owner, "project:t");
    assert_eq!(bindings["test:opaque/suffix"][0].action, "beta");
    assert_eq!(
        grouped["projects"][0]["actions"]["execute"]["operation"],
        "old"
    );
    for mode in 0..7 {
        let mut bad = grouped.clone();
        match mode {
            0 => bad["projects"][0]["test_groups"] = json!({}),
            1 => bad["projects"][0]["test_groups"] = json!({"other":"alpha"}),
            2 => bad["projects"][0]["test_groups"]["t"] = json!("foreign-action"),
            3 => bad["projects"][0]["test_groups"]["opaque/suffix"] = json!("alpha"),
            4 => bad["projects"][0]["kind"] = json!("production"),
            5 => bad["scripts"][0]["test_groups"] = json!({"s":"execute"}),
            _ => bad["projects"][0]["test_groups"]["s"] = json!("execute"),
        }
        assert!(
            test_bindings(&bad).unwrap_err().contains("E_TEST_BINDING"),
            "mode {mode}"
        );
    }
}

#[test]
fn scheduling_requires_complete_unambiguous_explicit_claims() {
    use chrono_judge_registration::execution::scheduling;
    let fm = json!({"execution_plans":{"test:t":{"operations":["run"],"timeout_seconds":30,"output_limit_bytes":4096}},"execution_scheduling":{"max_running":2,"resources":["db"],"claims":{"run":{"resources":["db"],"outputs":["out/"]}}}});
    let pr = json!({"projects":[{"id":"t","actions":{"execute":{"operation":"run","tool":"sh","argv":[]}}}],"scripts":[]});
    let artifacts = json!([{"path":"out/"}]);
    assert!(scheduling(&fm, &pr, &artifacts).unwrap().is_some());
    let mut old = fm.clone();
    old.as_object_mut().unwrap().remove("execution_scheduling");
    assert!(scheduling(&old, &pr, &artifacts).unwrap().is_none());
    for mode in 0..9 {
        let mut bad = fm.clone();
        let mut p = pr.clone();
        let mut a = artifacts.clone();
        match mode {
            0 => bad["execution_scheduling"]["max_running"] = json!(0),
            1 => {
                bad["execution_scheduling"]["claims"]
                    .as_object_mut()
                    .unwrap()
                    .remove("run");
            }
            2 => {
                bad["execution_scheduling"]["claims"]["extra"] =
                    json!({"resources":[],"outputs":[]})
            }
            3 => bad["execution_scheduling"]["claims"]["run"]["resources"] = json!(["unknown"]),
            4 => bad["execution_scheduling"]["claims"]["run"]["outputs"] = json!(["unknown/"]),
            5 => {
                let row = p["projects"][0].clone();
                p["projects"].as_array_mut().unwrap().push(row);
            }
            6 => bad["execution_scheduling"]["claims"]["run"] = json!({"resources":[]}),
            7 => bad["execution_scheduling"]["claims"]["run"]["outputs"] = json!(["../out/"]),
            _ => a.as_array_mut().unwrap().push(json!({"path":"out"})),
        }
        assert!(
            scheduling(&bad, &p, &a).is_err(),
            "accepted invalid declaration {mode}"
        );
    }
}
