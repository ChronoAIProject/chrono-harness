#[path = "../../judge-projects-tests/tests/support/host.rs"]
mod host;
#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::sha256;
use host::Host;
use serde_json::{Value, json};
use std::fs;
use support::*;
const POLICY: &str = ".chrono-harness/cargo/projects.json";
fn attach(h: &mut Host, rows: Value) {
    let root = h.root();
    let binary = source().join("crates/judge-cargo/target/debug/chrono-judge-cargo");
    let path = ".chrono-harness/bin/chrono-judge-cargo";
    fs::copy(&binary, root.join(path)).unwrap();
    h.values.get_mut(JUDGES).unwrap()["judges"].as_array_mut().unwrap().push(json!({"id":"cargo","executable":path,"version":"0.1.0","sha256":sha256(&fs::read(&binary).unwrap()),"argv":["--protocol","chrono-judge/v1","--policy",POLICY],"selector":"every-delta","after":["registration","filemap"],"modes":["evaluate"]}));
    h.values.get_mut(JUDGES).unwrap()["judges"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|j| j["id"] == "projects")
        .unwrap()["after"] = json!(["routes", "cargo"]);
    h.values.get_mut(FM).unwrap()["files"].as_array_mut().unwrap().push(file(POLICY,json!([{"kind":"runtime-input","to":"project:p"},{"kind":"runtime-input","to":"project:t"}])));
    h.values.insert(
        POLICY.into(),
        json!({"schema":"chrono-cargo-projects/v1","projects":rows}),
    );
    h.save();
}
fn rows(prefix: &str, ancestors: Value) -> Value {
    json!([{"project":"p","manifest":format!("{prefix}p/Cargo.toml"),"lockfile":format!("{prefix}p/Cargo.lock"),"output":format!("{prefix}p/target/"),"ancestor_manifests":ancestors},
           {"project":"t","manifest":format!("{prefix}t/Cargo.toml"),"lockfile":format!("{prefix}t/Cargo.lock"),"output":format!("{prefix}t/target/"),"ancestor_manifests":ancestors}])
}
fn check_pass(code: i32, v: &Value) {
    assert_eq!(code, 0, "{}", v["findings"]);
    assert_eq!(v["tests"]["tests"]["test:t"], "passed");
}
#[test]
fn registered_intermediate_workspace_rejected_before_operations_and_standalone_passes() {
    for workspace in [false, true] {
        let mut h = Host::new(false);
        let root = h.root();
        fs::create_dir(root.join("group")).unwrap();
        for id in ["p", "t"] {
            fs::rename(root.join(id), root.join(format!("group/{id}"))).unwrap();
        }
        for p in h.values.get_mut(PROJECTS).unwrap()["projects"]
            .as_array_mut()
            .unwrap()
        {
            for field in ["root", "manifest", "lockfile"] {
                p[field] = json!(format!("group/{}", p[field].as_str().unwrap()));
            }
        }
        h.values.get_mut(PROJECTS).unwrap()["projects"][1]["actions"]["execute"]["argv"][1] =
            json!("group/t/check.py");
        for f in h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
        {
            let path = f["path"].as_str().unwrap();
            if path.starts_with("p/") || path.starts_with("t/") {
                f["path"] = json!(format!("group/{path}"));
            }
        }
        for a in h.values.get_mut(CONFIG).unwrap()["artifacts"]
            .as_array_mut()
            .unwrap()
        {
            if a["kind"] == "cargo-output" {
                a["path"] = json!(format!("group/{}", a["path"].as_str().unwrap()));
            }
        }
        let check = fs::read_to_string(root.join("group/t/check.py"))
            .unwrap()
            .replace("p/product.py", "group/p/product.py");
        fs::write(root.join("group/t/check.py"), check).unwrap();
        if workspace {
            fs::write(
                root.join("group/Cargo.toml"),
                "[workspace]\nmembers=['p','t']\n",
            )
            .unwrap();
            h.values.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .push(file("group/Cargo.toml", json!([])));
        }
        attach(
            &mut h,
            rows(
                "group/",
                if workspace {
                    json!(["group/Cargo.toml"])
                } else {
                    json!([])
                },
            ),
        );
        h.base = h.candidate.clone();
        fs::write(root.join("doc.txt"), "docs only\n").unwrap();
        h.candidate = commit(&root);
        let (code, v) = h.run(|_| {});
        assert_eq!(
            code, 0,
            "docs do not adjudicate historical Cargo data: {}",
            v["findings"]
        );
        assert_eq!(v["tests"]["executed"], json!([]));
        fs::write(
            root.join("group/p/product.py"),
            "def double(n): return 2*n\n",
        )
        .unwrap();
        h.candidate = commit(&root);
        let (code, v) = h.run(|_| {});
        if workspace {
            assert_ne!(
                code, 0,
                "registered intermediate workspace must be rejected"
            );
            assert!(v.to_string().contains("E_TEST_PAIR"), "{}", v["findings"]);
            assert!(v.to_string().contains("group/Cargo.toml"));
            assert!(!root.join(".chrono-harness/state/order").exists());
        } else {
            check_pass(code, &v);
            assert_eq!(
                fs::read_to_string(root.join(".chrono-harness/state/order")).unwrap(),
                "pt"
            );
        }
    }
}

#[test]
fn undeclared_path_dependency_edge_blocks_actual_operations_and_repair_runs_them() {
    let mut h = Host::new(false);
    attach(&mut h, rows("", json!([])));
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .retain(|e| e["kind"] != "compile");
    h.save();
    let (code, v) = h.run(|_| {});
    assert_ne!(code, 0);
    assert!(
        v.to_string().contains("E_DANGLING_EDGE"),
        "{}",
        v["findings"]
    );
    assert!(!h.root().join(".chrono-harness/state/order").exists());
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge("project:p", "compile", "project:t"));
    h.save();
    let (code, v) = h.run(|_| {});
    check_pass(code, &v);
}
