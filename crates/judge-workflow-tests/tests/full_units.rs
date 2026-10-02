use super::*;
use chrono_harness::wire::{self, Binding, Request};
use std::{os::unix::fs::PermissionsExt, path::Path, process::Command, time::Instant};

fn rust_host() -> (Host, tempfile::TempDir) {
    let mut h = git_facts::bound_host();
    let tools = tempfile::tempdir().unwrap();
    let tool_root = fs::canonicalize(tools.path()).unwrap();
    let root = h.root();
    let wrapper = tool_root.join("cargo-fixture");
    let marker = tool_root.join("business-launches");
    let cargo = chrono_harness::resolve_program(&root, "cargo", None).unwrap();
    let quote = |p: &Path| format!("'{}'", p.to_str().unwrap().replace('\'', "'\\''"));
    let bytes = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\nif [ \"$1\" = fixture-prepare ]; then printf s >> .chrono-harness/state/order; exit 0; fi\nif [ -f .chrono-harness/state/fail ] && [ \"$3\" = t2/Cargo.toml ]; then exit 7; fi\nexec {} \"$@\"\n",
        quote(&marker),
        quote(&cargo)
    );
    fs::write(&wrapper, &bytes).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    // The declared Git producer remains available independently of all unit roots.
    let git_tool = tool_root.join("facts-git");
    let real_git = chrono_harness::resolve_program(&root, "git", None).unwrap();
    fs::write(
        &git_tool,
        format!(
            "#!/bin/sh\n[ \"$FACTS_SENTINEL\" = declared ] || exit 81\nexec {} \"$@\"\n",
            quote(&real_git)
        ),
    )
    .unwrap();
    fs::set_permissions(&git_tool, fs::Permissions::from_mode(0o755)).unwrap();
    h.tool = wrapper.clone();
    let cfg = h.values.get_mut(CONFIG).unwrap();
    for t in cfg["tools"].as_array_mut().unwrap() {
        if t["id"] == "python" {
            t["program"] = json!(wrapper);
        }
        if t["id"] == "facts-git" {
            t["program"] = json!(git_tool);
        }
    }
    let version = Command::new(&cargo).arg("--version").output().unwrap();
    cfg["tools"][0]["expected_version"] =
        json!(String::from_utf8(version.stdout).unwrap().trim_end());
    // The fixture needs Cargo and system tools, not the caller's editor/agent
    // directories repeated in every retained Git process observation.
    cfg["environment"]["values"]["PATH"] = json!(format!(
        "{}:/usr/bin:/bin",
        cargo.parent().unwrap().display()
    ));
    cfg["environment"]["values"]["HOME"] = json!(std::env::var("HOME").unwrap());
    // Native bootstrap selects Cargo through rustup with this variable. Keep
    // the same selection in the cleared judge/business environment, including
    // its absence locally, rather than falling back to a different default.
    cfg["environment"]["inherit"]
        .as_array_mut()
        .unwrap()
        .push(json!("RUSTUP_TOOLCHAIN"));
    cfg["input_closure"]["bindings"][1]["inputs"]
        .as_array_mut()
        .unwrap()
        .extend([
            json!("environment:HOME"),
            json!("environment:RUSTUP_TOOLCHAIN"),
        ]);
    for i in cfg["environment"]["inputs"].as_array_mut().unwrap() {
        if i["id"] == "interpreter" {
            i["location"] = json!(wrapper);
            i["sha256"] = json!(sha256(bytes.as_bytes()));
        }
        if i["id"] == "git-bytes" {
            i["location"] = json!(git_tool);
            i["sha256"] = json!(sha256(&fs::read(&git_tool).unwrap()));
        }
    }
    cfg["protocol"]["timeout_seconds"] = json!(180);
    cfg["semantic_fields"] = json!([{ "path":PROJECTS,"pointers":["/projects/*/actions"],"on":"add-modify-delete"},{"path":"policy.json","pointers":["/limit"],"on":"add-modify-delete"}]);
    cfg["execution_units"] = json!({"units":{"one":{"tests":["test:t"],"report_path":".chrono-harness/state/unit-one.json"},"two":{"tests":["test:t2"],"report_path":".chrono-harness/state/unit-two.json"}},"shared_operations":{"shared.prepare":["one","two"]},"collection_limits":{"manifest_bytes":1048576,"report_bytes":67108864},"report_path":".chrono-harness/state/collected.json"});
    for id in ["p", "t", "p2", "t2"] {
        cfg["artifacts"].as_array_mut().unwrap().push(json!({"path":format!("{id}/target/"),"owner":id,"kind":"cargo-output","tracked":false}));
    }
    let projects = h.values.get_mut(PROJECTS).unwrap();
    projects["owners"] = json!(["host", "p", "t", "p2", "t2"]);
    projects["scripts"] = json!([]);
    projects["projects"] = json!([]);
    for (prod, test) in [("p", "t"), ("p2", "t2")] {
        let mut production = project(prod, "production", test);
        production["actions"] = json!({"build":{"operation":format!("build.{prod}"),"tool":"python","argv":["build","--manifest-path",format!("{prod}/Cargo.toml"),"--locked","--offline"]}});
        if prod == "p2" {
            production["actions"]["check"] =
                json!({"operation":"shared.prepare","tool":"python","argv":["fixture-prepare"]});
        }
        let mut testing = project(test, "test", prod);
        testing["actions"] = json!({"execute":{"operation":format!("execute.{test}"),"tool":"python","argv":["test","--manifest-path",format!("{test}/Cargo.toml"),"--locked","--offline"]}});
        projects["projects"]
            .as_array_mut()
            .unwrap()
            .extend([production, testing]);
    }
    let fm = h.values.get_mut(FM).unwrap();
    fm["files"].as_array_mut().unwrap().retain(|f| {
        !f["path"].as_str().unwrap().starts_with("p/")
            && !f["path"].as_str().unwrap().starts_with("t/")
    });
    fm["project_edges"].as_array_mut().unwrap().push(edge(
        "environment:HOME",
        "runtime-input",
        "judge:projects",
    ));
    fm["project_edges"].as_array_mut().unwrap().push(edge(
        "environment:RUSTUP_TOOLCHAIN",
        "runtime-input",
        "judge:projects",
    ));
    fm["project_edges"].as_array_mut().unwrap().retain(|e| {
        !e["from"].as_str().unwrap().starts_with("script:")
            && !e["to"].as_str().unwrap().starts_with("script:")
    });
    for (prod, test) in [("p", "t"), ("p2", "t2")] {
        fm["project_edges"].as_array_mut().unwrap().extend([
            edge(
                &format!("project:{prod}"),
                "compile",
                &format!("project:{test}"),
            ),
            edge(
                &format!("project:{prod}"),
                "test-execution",
                &format!("test:{test}"),
            ),
            edge(
                &format!("project:{test}"),
                "test-execution",
                &format!("test:{test}"),
            ),
        ]);
        for id in [prod, test] {
            fs::create_dir_all(root.join(format!("{id}/src"))).unwrap();
            for suffix in ["Cargo.toml", "Cargo.lock", "src/lib.rs"] {
                let mut f = file(
                    &format!("{id}/{suffix}"),
                    json!([{"kind":"compile","to":format!("project:{id}")} ]),
                );
                f["owner"] = json!(id);
                fm["files"].as_array_mut().unwrap().push(f);
            }
        }
        let prod_manifest = format!("[package]\nname='{prod}'\nversion='0.1.0'\nedition='2024'\n");
        let test_manifest = format!(
            "[package]\nname='{test}'\nversion='0.1.0'\nedition='2024'\n[dev-dependencies]\n{prod}={{path='../{prod}'}}\n"
        );
        fs::write(root.join(format!("{prod}/Cargo.toml")), prod_manifest).unwrap();
        fs::write(root.join(format!("{test}/Cargo.toml")), test_manifest).unwrap();
        fs::write(
            root.join(format!("{prod}/src/lib.rs")),
            "pub fn double(n:i32)->i32 { n*2 }\n",
        )
        .unwrap();
        fs::write(
            root.join(format!("{test}/src/lib.rs")),
            format!("#[test] fn real_implementation() {{ assert_eq!({prod}::double(21),42); }}\n"),
        )
        .unwrap();
        for id in [prod, test] {
            assert!(
                Command::new(&cargo)
                    .args(["generate-lockfile", "--offline", "--manifest-path"])
                    .arg(root.join(format!("{id}/Cargo.toml")))
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
    }
    fm["execution_plans"] = json!({"test:t":{"operations":["shared.prepare","build.p","execute.t"],"timeout_seconds":30,"output_limit_bytes":32768},"test:t2":{"operations":["shared.prepare","build.p2","execute.t2"],"timeout_seconds":30,"output_limit_bytes":32768}});
    fm["test_costs"] = json!([{"test":"t","cost":"unknown"},{"test":"t2","cost":"unknown"}]);
    h.values.get_mut(WORKFLOW).unwrap()["integration"]["tests"] = json!(["t", "t2"]);
    h.values.get_mut(WORKFLOW).unwrap()["stability"] = json!([
        {"id":"p","paths":["p/src/lib.rs"],"tests":["t"],"reason":"explicit Rust stability obligation"},
        {"id":"p2","paths":["p2/src/lib.rs"],"tests":["t2"],"reason":"explicit Rust stability obligation"}
    ]);
    fs::write(root.join(".gitignore"),".chrono-harness/bin/\n.chrono-harness/state/\np/target/\nt/target/\np2/target/\nt2/target/\n").unwrap();
    // Replace the old script fixture completely before fixing the Rust baseline.
    for path in ["p/product.py", "t/check.py"] {
        fs::remove_file(root.join(path)).unwrap();
    }
    h.save();
    h.base = h.candidate.clone();
    for prod in ["p", "p2"] {
        fs::write(
            root.join(format!("{prod}/src/lib.rs")),
            "pub fn double(n:i32)->i32 { n+n }\n",
        )
        .unwrap();
    }
    fs::write(root.join("policy.json"), "{\"limit\":3}").unwrap();
    h.save();
    (h, tools)
}
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &to.join(e.file_name()));
        } else {
            fs::copy(e.path(), to.join(e.file_name())).unwrap();
        }
    }
}
fn clone_host(h: &Host) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["clone", "--quiet", "--no-hardlinks"])
            .arg(h.root())
            .arg(dir.path())
            .output()
            .unwrap()
            .status
            .success()
    );
    copy_tree(
        &h.root().join(".chrono-harness/bin"),
        &dir.path().join(".chrono-harness/bin"),
    );
    copy_tree(
        &h.root().join(".chrono-harness/state"),
        &dir.path().join(".chrono-harness/state"),
    );
    dir
}
fn launch(root: &Path, h: &Host, suffix: &[&str]) -> (i32, Value) {
    launch_endpoints(root, &h.base, &h.candidate, suffix)
}
fn launch_entry(root: &Path, h: &Host, cwd: &Path, config: &str, suffix: &[&str]) -> (i32, Value) {
    let root = fs::canonicalize(root).unwrap();
    let argv = vec![
        "check".to_owned(),
        "--config".into(),
        config.into(),
        "--base".into(),
        h.base.clone(),
        "--candidate".into(),
        h.candidate.clone(),
        "--context".into(),
        root.join(".chrono-harness/state/context.json")
            .to_str()
            .unwrap()
            .into(),
    ];
    let out = Command::new(root.join(".chrono-harness/bin/chrono-harness"))
        .current_dir(cwd)
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .args(&argv)
        .args(suffix)
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&out.stderr)}));
    (out.status.code().unwrap(), report)
}
fn launch_endpoints(root: &Path, base: &str, candidate: &str, suffix: &[&str]) -> (i32, Value) {
    let root_buf = fs::canonicalize(root).unwrap();
    let root = root_buf.as_path();
    let argv = vec![
        "check".to_owned(),
        "--config".into(),
        root.join(CONFIG).to_str().unwrap().into(),
        "--base".into(),
        base.into(),
        "--candidate".into(),
        candidate.into(),
        "--context".into(),
        root.join(".chrono-harness/state/context.json")
            .to_str()
            .unwrap()
            .into(),
    ];
    let started = Instant::now();
    let out = Command::new(root.join(".chrono-harness/bin/chrono-harness"))
        .current_dir("/")
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .args(&argv)
        .args(suffix)
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&out.stderr)}));
    println!(
        "COMMAND {} argv={} exit={} seconds={:.3} report_bytes={}",
        root.join(".chrono-harness/bin/chrono-harness").display(),
        json!(
            argv.iter()
                .cloned()
                .chain(suffix.iter().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        ),
        out.status.code().unwrap(),
        started.elapsed().as_secs_f64(),
        out.stdout.len()
    );
    println!(
        "PROCESS_OUTPUT exit={} stdout_bytes={} stderr_bytes={} stderr_sha256={} stderr={:?}",
        out.status.code().unwrap(),
        out.stdout.len(),
        out.stderr.len(),
        sha256(&out.stderr),
        String::from_utf8_lossy(&out.stderr),
    );
    if !out.status.success() {
        println!("FAILURE findings={} stderr={:?} transport={}", report["findings"], report["stderr"],json!(report["judges"].as_array().into_iter().flatten().filter(|j|j["transport_failure"].is_string()).map(|j|json!({"id":j["id"],"state":j["state"],"transport_failure":j["transport_failure"],"exit_code":j["exit_code"]})).collect::<Vec<_>>()));
    }
    (out.status.code().unwrap(), report)
}
fn manifest(root: &Path, units: &[&str]) {
    let reports: Vec<_> = units
        .iter()
        .map(|unit| {
            let path = format!(".chrono-harness/state/unit-{unit}.json");
            json!({"unit":unit,"path":path,"sha256":sha256(&fs::read(root.join(&path)).unwrap())})
        })
        .collect();
    fs::write(
        root.join(".chrono-harness/state/manifest.json"),
        serde_json::to_vec(&json!({"schema":"chrono-full-collection/v1","reports":reports}))
            .unwrap(),
    )
    .unwrap();
}
fn marker(tools: &Path) -> String {
    fs::read_to_string(tools.join("business-launches")).unwrap_or_default()
}
fn collected(root: &Path, h: &Host) -> (i32, Value) {
    launch(
        root,
        h,
        &["--collect", ".chrono-harness/state/manifest.json"],
    )
}

#[test]
fn invalid_unit_input_preserves_original_failure_and_retained_report_without_business_effects() {
    let mut h = git_facts::bound_host();
    h.values.get_mut(CONFIG).unwrap()["execution_units"] = json!({
        "units":{"one":{"tests":["test:t"],"report_path":".chrono-harness/state/unit-one.json"}},
        "shared_operations":{},
        "collection_limits":{"manifest_bytes":1048576,"report_bytes":67108864},
        "report_path":".chrono-harness/state/collected.json"
    });
    h.save();
    git_facts::prepare_bound(&h, "integration", None);
    let root = h.root();
    let path = root.join(".chrono-harness/state/inputs.json");
    let mut inputs: Value = chrono_harness::json(&fs::read(&path).unwrap()).unwrap();
    inputs["base"]["files"]["interpreter"]["blob"] = json!("/outside");
    fs::write(path, serde_json::to_vec(&inputs).unwrap()).unwrap();

    let (exit, report) = launch(&root, &h, &["--unit", "one"]);
    assert_eq!(exit, 2, "{report}");
    let registration = &report["judges"][0];
    assert_eq!(registration["id"], "registration", "{report}");
    assert_eq!(registration["response"]["status"], "error");
    let findings = registration["response"]["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{report}");
    assert_eq!(findings[0]["code"], "E_EVIDENCE_UNRESOLVED");
    assert_eq!(
        findings[0]["message"],
        "E_INPUT_BLOB: reference must be within host state"
    );
    assert_eq!(report["findings"], registration["response"]["findings"]);
    assert_eq!(report["status"], "error");
    assert!(report["tests"].is_null());
    assert!(
        report["judges"].as_array().unwrap()[1..]
            .iter()
            .all(|judge| judge["state"] == "blocked")
    );
    assert!(!root.join(".chrono-harness/state/order").exists());
    assert!(!root.join(".chrono-harness/state/integration.json").exists());
    assert!(report["artifacts"].get("/outside").is_none());
    assert_eq!(
        report["artifact_failures"]["/outside"],
        "invalid relative path: \"/outside\""
    );
    assert!(report["unresolved"]["/artifacts"].is_string());
    for path in [
        ".chrono-harness/state/unit-one.json",
        report["report_path"].as_str().unwrap(),
    ] {
        let stored = chrono_harness::json(&fs::read(root.join(path)).unwrap()).unwrap();
        assert_eq!(stored, report, "failed report must be retained at {path}");
    }
    // Available originals retain their real bytes even when another address is invalid.
    let context = report["request"]["context"]["path"].as_str().unwrap();
    assert_eq!(
        chrono_harness::full::artifact_bytes(&report["artifacts"], context).unwrap(),
        fs::read(context).unwrap()
    );
}

#[test]
fn independent_rust_pairs_retry_offline_collection_and_delivery() {
    let (h, tools) = rust_host();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    fs::remove_file(h.root().join(".chrono-harness/state/integration.json")).unwrap();
    let two = clone_host(&h);
    let collector = clone_host(&h);
    let nested_cwd = h.root().join("nested-caller");
    fs::create_dir_all(&nested_cwd).unwrap();
    let before = marker(tools.path());
    let (e, one) = launch_entry(
        &h.root(),
        &h,
        &nested_cwd,
        "../.chrono-harness/config.json",
        &["--unit", "one"],
    );
    passed(e, &one);
    assert_eq!(
        one["request"]["observations"]["entry"]["argv"][3],
        "../.chrono-harness/config.json"
    );
    assert_eq!(
        one["request"]["observations"]["entry"]["resolved_paths"]["paths"][3]["actual"],
        one["request"]["observations"]["entry"]["resolved_paths"]["paths"][3]["expected"]
    );
    assert_eq!(one["unit_evidence"]["own"], json!(["test:t"]));
    assert_eq!(
        one["unit_evidence"]["assigned_elsewhere"],
        json!(["test:t2"])
    );
    assert_eq!(workflow(&one)["mode"], "contribution");
    assert!(
        !h.root()
            .join(".chrono-harness/state/integration.json")
            .exists()
    );
    let after_one = marker(tools.path());
    assert_eq!(after_one[before.len()..].lines().count(), 4);
    assert!(!after_one[before.len()..].contains("p2/Cargo.toml"));
    fs::write(two.path().join(".chrono-harness/state/fail"), "").unwrap();
    let (e, failed) = launch(two.path(), &h, &["--unit", "two"]);
    assert_ne!(e, 0);
    assert_ne!(failed["status"], "pass");
    assert_eq!(
        fs::read(h.root().join(".chrono-harness/state/unit-one.json")).unwrap(),
        serde_json::to_vec(&one)
            .unwrap()
            .into_iter()
            .chain([b'\n'])
            .collect::<Vec<_>>()
    );
    fs::remove_file(two.path().join(".chrono-harness/state/fail")).unwrap();
    let retry_before = marker(tools.path());
    let (e, two_report) = launch(two.path(), &h, &["--unit", "two"]);
    passed(e, &two_report);
    assert!(
        two_report["request"]["observations"]["entry"]["argv"][3]
            .as_str()
            .unwrap()
            .starts_with("/")
    );
    let retry = marker(tools.path());
    assert_eq!(retry[retry_before.len()..].lines().count(), 4);
    assert!(!retry[retry_before.len()..].contains("p/Cargo.toml"));
    for (root, unit) in [(h.root(), "one"), (two.path().to_path_buf(), "two")] {
        fs::copy(
            root.join(format!(".chrono-harness/state/unit-{unit}.json")),
            collector
                .path()
                .join(format!(".chrono-harness/state/unit-{unit}.json")),
        )
        .unwrap();
    }
    manifest(collector.path(), &["one", "two"]);
    let old_one = h.root();
    let old_two = two.path().to_path_buf();
    fs::remove_dir_all(&old_one).unwrap();
    fs::remove_dir_all(&old_two).unwrap();
    fs::remove_file(tools.path().join("cargo-fixture")).unwrap();
    assert!(!old_one.exists() && !old_two.exists());
    let before = marker(tools.path());
    let (e, report) = collected(collector.path(), &h);
    passed(e, &report);
    assert_eq!(
        marker(tools.path()),
        before,
        "collection must launch zero business processes, including --version"
    );
    assert_eq!(report["tests"]["executed"], json!([]));
    assert_eq!(
        report["tests"]["completion"]["required_units"],
        json!(["one", "two"])
    );
    assert_eq!(workflow(&report)["mode"], "integration_run");
    println!("EFFECTS one=4 failed_two=4 retry_two=4 collection=0 shared_per_unit=1");
    // Delivery transports the finalized report's complete original closure.
    // Mutable unit paths and the collector's separate imported copies are no longer needed.
    for source in report["tests"]["completion"]["reports"].as_array().unwrap() {
        for key in ["path", "retained_path"] {
            fs::remove_file(collector.path().join(source[key].as_str().unwrap())).unwrap();
        }
    }
    let certificate_path = collector
        .path()
        .join(".chrono-harness/state/integration.json");
    let cert_bytes = fs::read(&certificate_path).unwrap();
    let digest = sha256(&cert_bytes);
    let mut ctx: Value = serde_json::from_slice(
        &fs::read(collector.path().join(".chrono-harness/state/context.json")).unwrap(),
    )
    .unwrap();
    ctx["run_kind"] = "delivery".into();
    ctx["branch_ref"] = "feature/full-units".into();
    ctx["integration_evidence"] = json!(digest);
    fs::write(
        collector.path().join(".chrono-harness/state/context.json"),
        serde_json::to_vec(&ctx).unwrap(),
    )
    .unwrap();
    // Restore the explicit business tool only for the delivery's direct run.
    let request: Request = serde_json::from_value(one["request"].clone()).unwrap();
    let file = &request.observations["retained"]["candidate"]["files"]["interpreter"];
    let bytes =
        chrono_harness::full::artifact_bytes(&one["artifacts"], file["blob"].as_str().unwrap())
            .unwrap();
    fs::write(tools.path().join("cargo-fixture"), bytes).unwrap();
    fs::set_permissions(
        tools.path().join("cargo-fixture"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let (e, delivery) = launch(collector.path(), &h, &[]);
    passed(e, &delivery);
    assert_eq!(workflow(&delivery)["mode"], "delivery");
    // Transported finalized proof contains originals; losing any of those bytes rejects delivery.
    let cert: Value = serde_json::from_slice(&cert_bytes).unwrap();
    let path = collector
        .path()
        .join(cert["producer"]["report_path"].as_str().unwrap());
    let original = fs::read(&path).unwrap();
    for case in [
        "missing-originals",
        "failed-collector-process",
        "provisional-report",
        "oversized-report",
    ] {
        let mut final_report: Value = serde_json::from_slice(&original).unwrap();
        match case {
            "missing-originals" => {
                let address = final_report["tests"]["completion"]["reports"][0]["retained_path"]
                    .as_str()
                    .unwrap()
                    .to_owned();
                assert!(
                    final_report["artifacts"]
                        .as_object_mut()
                        .unwrap()
                        .remove(&address)
                        .is_some()
                );
            }
            "failed-collector-process" => {
                final_report["judges"][0]["process"]["exit_code"] = json!(7)
            }
            "provisional-report" => {
                final_report["judges"].as_array_mut().unwrap().pop();
            }
            "oversized-report" => {}
            _ => unreachable!(),
        }
        fs::write(&path, serde_json::to_vec(&final_report).unwrap()).unwrap();
        if case == "oversized-report" {
            fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .set_len(64 * 1024 * 1024 + 1)
                .unwrap();
        }
        let (e, rejected) = launch(collector.path(), &h, &[]);
        assert_ne!(e, 0);
        assert!(
            rejected["findings"]
                .to_string()
                .contains("E_INTEGRATION_MISMATCH")
        );
        if case == "oversized-report" {
            assert!(
                rejected["findings"]
                    .to_string()
                    .contains("exceeds registered bound")
            );
        }
        if case == "missing-originals" {
            assert!(
                rejected["findings"]
                    .to_string()
                    .contains("missing original artifact .chrono-harness/state/import-")
            );
        }
        println!("DELIVERY_REJECTION case={case} exit={e}");
    }
    fs::write(&path, original).unwrap();
}

// Coherently reseal downstream stdin/response bytes, so semantic mutations cannot
// pass merely because the surrounding manifest and self-digests agree.
fn reseal(report: &mut Value) {
    for r in report["judges"].as_array_mut().unwrap() {
        *r = chrono_harness::full::expand_record(r).unwrap();
    }
    let mut template: Request = serde_json::from_value(report["request"].clone()).unwrap();
    template.seal().unwrap();
    report["request"] = json!(template);
    report["request_digest"] = json!(wire::digest(&template).unwrap());
    let mut history = vec![];
    for row in report["judges"].as_array_mut().unwrap() {
        let binding: Binding = serde_json::from_value(row["binding"].clone()).unwrap();
        let request = chrono_harness::full::judge_request(&template, &binding, &history).unwrap();
        row["request_id"] = json!(request.request_id);
        row["request_digest"] = json!(wire::digest(&request).unwrap());
        row["process"]["stdin_sha256"] = row["request_digest"].clone();
        row["response"]["request_id"] = row["request_id"].clone();
        let stdout = serde_json::to_vec(&row["response"]).unwrap();
        row["process"]["stdout_bytes"] = json!(stdout);
        row["process"]["stdout"] = json!(String::from_utf8(stdout.clone()).unwrap());
        row["process"]["stdout_sha256"] = json!(sha256(&stdout));
        history.push(chrono_harness::full::predecessor(row));
    }
    let projects = report["judges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"] == "projects")
        .unwrap();
    report["tests"] = projects["response"]["outputs"]["tests"].clone();
}
fn row(report: &mut Value, id: &str) -> usize {
    report["judges"]
        .as_array()
        .unwrap()
        .iter()
        .position(|j| j["id"] == id)
        .unwrap()
}
fn identity(value: &Value, key: &str) -> String {
    let mut v = value.clone();
    v.as_object_mut().unwrap().remove(key);
    wire::digest(&v).unwrap()
}
fn reseal_plan(report: &mut Value) {
    let r = row(report, "routes");
    let p = row(report, "projects");
    let plan = &mut report["judges"][r]["response"]["outputs"]["execution_plan"];
    plan["identity"] = json!(identity(plan, "identity"));
    let plan = plan.clone();
    let results = &mut report["judges"][p]["response"]["outputs"]["tests"];
    results["plan"] = plan["identity"].clone();
    for op in results["executed"].as_array_mut().unwrap() {
        op["receipt"]["plan"] = plan["identity"].clone();
        op["receipt"]["digest"] = json!(identity(&op["receipt"], "digest"));
    }
}
fn change_environment(report: &mut Value) {
    report["request"]["observations"]["environment"]["effective"]["FACTS_SENTINEL"] =
        "different".into();
    report["environment"] = report["request"]["observations"]["environment"].clone();
    for r in report["judges"].as_array_mut().unwrap() {
        r["process"]["environment"]["FACTS_SENTINEL"] = "different".into();
        r["process"]["environment_digest"] =
            json!(wire::digest(&r["process"]["environment"]).unwrap());
    }
    reseal(report);
}

#[test]
fn collection_rejects_consistently_resealed_environment() {
    let (h, tools) = rust_host();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    let (e, mut report) = launch(&h.root(), &h, &["--unit", "one"]);
    passed(e, &report);
    for row in report["judges"].as_array_mut().unwrap() {
        *row = chrono_harness::full::expand_record(row).unwrap();
    }
    change_environment(&mut report);
    let before = marker(tools.path());
    fs::write(
        h.root().join(".chrono-harness/state/unit-one.json"),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
    manifest(&h.root(), &["one"]);
    let (e, rejected) = collected(&h.root(), &h);
    assert_eq!(e, 2);
    assert!(
        rejected["findings"]
            .to_string()
            .contains("original report/root/run/runner summary differs")
    );
    assert!(
        !rejected["findings"]
            .to_string()
            .contains("E_PROCESS_EVIDENCE")
    );
    assert_eq!(marker(tools.path()), before);
    println!("REJECTION coherently_resealed_environment=declared_semantics business_launches=0");
}

#[test]
fn collection_rejects_resealed_semantics_artifacts_and_manifest_failures() {
    let (h, tools) = rust_host();
    git_facts::prepare_bound(&h, "integration", None);
    for unit in ["one", "two"] {
        let (e, r) = launch(&h.root(), &h, &["--unit", unit]);
        passed(e, &r);
    }
    let root = h.root();
    let path = root.join(".chrono-harness/state/unit-one.json");
    let original = fs::read(&path).unwrap();
    let original_report: Value = serde_json::from_slice(&original).unwrap();
    manifest(&root, &["one", "two"]);
    let (e, r) = collected(&root, &h);
    passed(e, &r);
    let before = marker(tools.path());
    let semantic_cases = [
        "summary",
        "failed-judge",
        "blocked-judge",
        "receipt-exit",
        "receipt-failure",
        "wrong-plan",
        "wrong-method",
        "wrong-global-graph",
        "wrong-input",
        "wrong-tool",
        "wrong-version",
        "wrong-context",
        "wrong-entry",
        "wrong-runner-invocation",
        "wrong-runner-resolved-path",
        "predecessor",
        "missing-artifact",
        "replaced-artifact",
        "incomplete-contribution",
        "shared-repeat",
        "stale-endpoint",
        "wrong-registry",
        "wrong-environment",
    ];
    for case in semantic_cases {
        let mut report = original_report.clone();
        for r in report["judges"].as_array_mut().unwrap() {
            *r = chrono_harness::full::expand_record(r).unwrap();
        }
        let ri = row(&mut report, "routes");
        let pi = row(&mut report, "projects");
        let wi = row(&mut report, "workflow");
        match case {
            "summary" => report["status"] = "pass".into(),
            "failed-judge" => {
                report["judges"][wi]["response"]["status"] = "error".into();
                report["judges"][wi]["response"]["findings"] = json!([{"code":"E_FIXTURE","message":"actual failed workflow","level":"error","delta_refs":["/request"],"causes":[]}]);
                report["judges"][wi]["process"]["exit_code"] = json!(2);
                report["judges"][wi]["exit_code"] = json!(2);
                reseal(&mut report);
                report["status"] = "pass".into();
            }
            "blocked-judge" => report["judges"][wi]["state"] = "blocked".into(),
            "receipt-exit" | "receipt-failure" => {
                let receipt = &mut report["judges"][pi]["response"]["outputs"]["tests"]["executed"]
                    [0]["receipt"];
                if case == "receipt-exit" {
                    receipt["process"]["exit_code"] = json!(7);
                } else {
                    receipt["process"]["failure"] = json!("resource failure");
                }
                receipt["digest"] = json!(identity(receipt, "digest"));
                reseal(&mut report);
            }
            "wrong-plan" => {
                report["judges"][ri]["response"]["outputs"]["execution_plan"]["binding"]["run"] =
                    "wrong-run".into();
                reseal_plan(&mut report);
                reseal(&mut report);
            }
            "wrong-method" => {
                report["judges"][ri]["response"]["outputs"]["execution_plan"]["operations"][0]["timeout_seconds"] =
                    json!(29);
                reseal_plan(&mut report);
                reseal(&mut report);
            }
            "wrong-global-graph" => {
                report["judges"][ri]["response"]["outputs"]["global_graph"]["operations"][0]["timeout_seconds"] =
                    json!(29);
                reseal(&mut report);
            }
            "wrong-version" => {
                let version = &mut report["judges"][ri]["response"]["outputs"]["execution_plan"]["tools"]
                    ["python"]["version"];
                let bytes = b"different version\n".to_vec();
                version["stdout_bytes"] = json!(bytes);
                version["stdout"] = json!("different version\n");
                version["stdout_sha256"] = json!(sha256(&bytes));
                reseal_plan(&mut report);
                reseal(&mut report);
            }
            "wrong-input" => {
                report["request"]["observations"]["retained"]["candidate"]["files"]["data"] =
                    json!({"bytes":b"other input\n".to_vec()});
                let snapshot =
                    serde_json::to_vec(&report["request"]["observations"]["retained"]).unwrap();
                report["artifacts"][".chrono-harness/state/inputs.json"] =
                    chrono_harness::full::artifact(&snapshot);
                reseal(&mut report);
            }
            "wrong-tool" => {
                report["judges"][ri]["response"]["outputs"]["execution_plan"]["tools"]["python"]
                    ["program"] = "/different/tool".into();
                reseal_plan(&mut report);
                reseal(&mut report);
            }
            "wrong-context" => {
                let address = report["request"]["context"]["path"]
                    .as_str()
                    .unwrap()
                    .to_owned();
                let mut ctx: Value = serde_json::from_slice(
                    &chrono_harness::full::artifact_bytes(&report["artifacts"], &address).unwrap(),
                )
                .unwrap();
                ctx["observed_at"] = "2026-01-01T02:00:00Z".into();
                report["request"]["context"]["sha256"] = json!(wire::digest(&ctx).unwrap());
                report["artifacts"][&address] =
                    chrono_harness::full::artifact(&serde_json::to_vec(&ctx).unwrap());
                reseal(&mut report);
            }
            "wrong-entry" => {
                report["request"]["observations"]["entry"]["argv"][3] =
                    ".chrono-harness/state/other-config.json".into();
            }
            "wrong-runner-invocation" | "wrong-runner-resolved-path" => {
                report["request"]["runner"]["path"] = json!("/outside/chrono-harness");
                if case == "wrong-runner-resolved-path" {
                    let entry = &mut report["request"]["observations"]["entry"];
                    entry["argv"][0] = json!("/outside/chrono-harness");
                    entry["resolved_paths"]["raw_digest"] = json!(
                        wire::digest(&json!({"argv":entry["argv"],"cwd":entry["cwd"]})).unwrap()
                    );
                    entry["resolved_paths"]["paths"][0] = json!({
                        "actual":"/outside/chrono-harness", "expected":"/outside/chrono-harness"
                    });
                }
                reseal(&mut report);
            }
            "stale-endpoint" => {
                report["request"]["candidate"]["commit"] = json!("0".repeat(40));
                report["candidate"] = report["request"]["candidate"]["commit"].clone();
                reseal(&mut report);
            }
            "wrong-registry" => {
                report["request"]["observations"]["registry_bindings"]["candidate"]["effective_path"] =
                    "other.json".into();
                reseal(&mut report);
            }
            "wrong-environment" => {
                change_environment(&mut report);
            }
            "predecessor" => {
                let template: Request = serde_json::from_value(report["request"].clone()).unwrap();
                let binding: Binding =
                    serde_json::from_value(report["judges"][wi]["binding"].clone()).unwrap();
                let history: Vec<_> = report["judges"].as_array().unwrap()[..wi]
                    .iter()
                    .map(chrono_harness::full::predecessor)
                    .collect();
                let mut request =
                    chrono_harness::full::judge_request(&template, &binding, &history).unwrap();
                request.observations["judges"][0]["process"]["argv"] = json!(["wrong"]);
                request.seal().unwrap();
                report["judges"][wi]["request"] = json!(request);
            }
            "missing-artifact" => report["artifacts"].as_object_mut().unwrap().clear(),
            "replaced-artifact" => {
                let address=report["request"]["observations"]["retained"]["candidate"]["files"]["interpreter"]["blob"].as_str().unwrap().to_owned();
                report["artifacts"][&address] =
                    chrono_harness::full::artifact(b"different executable");
            }
            "incomplete-contribution" => {
                report["judges"][wi]["response"]["outputs"]["workflow"]["mode"] =
                    "integration_run".into();
                reseal(&mut report);
            }
            "shared-repeat" => {
                let op =
                    report["judges"][pi]["response"]["outputs"]["tests"]["executed"][0].clone();
                report["judges"][pi]["response"]["outputs"]["tests"]["executed"]
                    .as_array_mut()
                    .unwrap()
                    .push(op);
                reseal(&mut report);
            }
            _ => unreachable!(),
        }
        fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
        manifest(&root, &["one", "two"]);
        let (e, rejected) = collected(&root, &h);
        assert_ne!(e, 0, "accepted {case}");
        assert_ne!(rejected["status"], "pass");
        if matches!(
            case,
            "wrong-runner-invocation" | "wrong-runner-resolved-path"
        ) {
            assert!(
                rejected["findings"]
                    .to_string()
                    .contains("E_COLLECTION_INPUT: original runner invocation differs"),
                "runner mutation did not reach its identity owner: {rejected}"
            );
        }
        assert!(
            !rejected["findings"]
                .to_string()
                .contains("bounded collection input rejected"),
            "semantic mutation hit the size gate: {case}"
        );
        println!("REJECTION case={case} exit={e}");
        assert_eq!(marker(tools.path()), before);
    }
    fs::write(&path, &original).unwrap();
    for case in [
        "missing-unit",
        "unknown-unit",
        "duplicate-unit",
        "digest-mismatch",
        "report-limit",
        "manifest-limit",
    ] {
        manifest(&root, &["one", "two"]);
        let mp = root.join(".chrono-harness/state/manifest.json");
        let mut m: Value = serde_json::from_slice(&fs::read(&mp).unwrap()).unwrap();
        match case {
            "missing-unit" => {
                m["reports"].as_array_mut().unwrap().pop();
            }
            "unknown-unit" => m["reports"][0]["unit"] = "unknown".into(),
            "duplicate-unit" => {
                let one = m["reports"][0].clone();
                m["reports"].as_array_mut().unwrap().push(one);
            }
            "digest-mismatch" => m["reports"][0]["sha256"] = json!("0".repeat(64)),
            "report-limit" => {
                fs::write(&path, vec![b' '; 67108865]).unwrap();
                m["reports"][0]["sha256"] = json!(sha256(&fs::read(&path).unwrap()));
            }
            "manifest-limit" => {}
            _ => unreachable!(),
        }
        fs::write(&mp, serde_json::to_vec(&m).unwrap()).unwrap();
        if case == "manifest-limit" {
            fs::write(&mp, vec![b' '; 1048577]).unwrap();
        }
        let (e, _) = collected(&root, &h);
        assert_ne!(e, 0, "accepted {case}");
        println!("REJECTION case={case} exit={e}");
        fs::write(&path, &original).unwrap();
        assert_eq!(marker(tools.path()), before);
    }
    manifest(&root, &["one", "two"]);
    let (e, r) = collected(&root, &h);
    passed(e, &r);
    assert_eq!(marker(tools.path()), before);
    println!(
        "REJECTIONS semantic={} manifest_and_bounds=6 business_launches=0 unit_bytes={}",
        semantic_cases.len(),
        original.len()
    );
}

#[test]
fn global_cycles_assignments_and_reserved_paths_precede_business_effects() {
    let (mut h, tools) = rust_host();
    let cfg = h.values[CONFIG].clone();
    let fm = h.values[FM].clone();
    let canonical = h.root().join(".chrono-harness/state/report.json");
    fs::write(&canonical, b"previous full report").unwrap();
    h.values.get_mut(CONFIG).unwrap()["execution_units"]["units"]["one"]["report_path"] =
        json!(".chrono-harness/state/report.json");
    h.save();
    let before = marker(tools.path());
    let (e, _) = git_facts::run_bound(&h, "integration", None);
    assert_ne!(e, 0);
    assert_eq!(fs::read(&canonical).unwrap(), b"previous full report");
    assert_eq!(marker(tools.path()), before);
    h.values.insert(CONFIG.into(), cfg.clone());
    h.values.get_mut(CONFIG).unwrap()["execution_units"]["units"]
        .as_object_mut()
        .unwrap()
        .remove("two");
    h.save();
    let (e, _) = git_facts::run_bound(&h, "integration", None);
    assert_ne!(e, 0);
    assert_eq!(marker(tools.path()), before);
    h.values.insert(CONFIG.into(), cfg);
    h.values.get_mut(FM).unwrap()["execution_plans"]["test:t2"]["operations"] =
        json!(["build.p", "shared.prepare", "build.p2", "execute.t2"]);
    h.values.get_mut(CONFIG).unwrap()["execution_units"]["shared_operations"]["build.p"] =
        json!(["one", "two"]);
    h.save();
    let (e, _) = git_facts::run_bound(&h, "integration", None);
    assert_ne!(e, 0);
    let (e, rejected) = launch(&h.root(), &h, &["--unit", "one"]);
    assert_ne!(e, 0);
    assert!(rejected["findings"].to_string().contains("E_PLAN_CYCLE"));
    assert_eq!(marker(tools.path()), before);
    h.values.insert(FM.into(), fm);
    h.values.get_mut(CONFIG).unwrap()["execution_units"]["shared_operations"]
        .as_object_mut()
        .unwrap()
        .remove("build.p");
    h.save();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    let before = marker(tools.path());
    let (e, _) = launch(&h.root(), &h, &["--unit", "unknown"]);
    assert_ne!(e, 0);
    assert_eq!(marker(tools.path()), before);
    println!(
        "GLOBAL_REJECTIONS collision=1 assignments=1 global_cycle=2 unknown=1 invalid_business_launches=0"
    );
}
#[test]
fn delivery_contributions_and_empty_collection_are_not_completed_unit_proofs() {
    let (mut h, tools) = rust_host();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    let (mut cert, _) = certificate(&h);
    fs::remove_file(h.root().join(".chrono-harness/state/integration.json")).unwrap();
    let cp = h.root().join(".chrono-harness/state/context.json");
    let mut ctx: Value = serde_json::from_slice(&fs::read(&cp).unwrap()).unwrap();
    ctx["run_kind"] = "delivery".into();
    ctx["branch_ref"] = "feature/contribution".into();
    ctx["integration_evidence"] = Value::Null;
    fs::write(&cp, serde_json::to_vec(&ctx).unwrap()).unwrap();
    let (e, unit) = launch(&h.root(), &h, &["--unit", "one"]);
    passed(e, &unit);
    assert_eq!(workflow(&unit)["mode"], "contribution");
    assert!(
        !h.root()
            .join(".chrono-harness/state/integration.json")
            .exists()
    );
    // A contribution cannot replace completed integration evidence, even with new enclosing digests.
    let ri = unit["judges"]
        .as_array()
        .unwrap()
        .iter()
        .position(|r| r["id"] == "routes")
        .unwrap();
    cert["proof"]["plan"] = unit["judges"][ri]["response"]["outputs"]["execution_plan"].clone();
    cert["proof"]["results"] = unit["tests"].clone();
    cert["results_digest"] = json!(wire::digest(&unit["tests"]).unwrap());
    let bytes = wire::canonical(&cert).unwrap();
    fs::write(
        h.root().join(".chrono-harness/state/integration.json"),
        &bytes,
    )
    .unwrap();
    ctx["integration_evidence"] = json!(sha256(&bytes));
    fs::write(&cp, serde_json::to_vec(&ctx).unwrap()).unwrap();
    let (e, rejected) = launch(&h.root(), &h, &[]);
    assert_ne!(e, 0);
    assert!(
        rejected["findings"]
            .to_string()
            .contains("E_INTEGRATION_MISMATCH")
    );
    // Fixed identical endpoints have no DELTA and no required unit. All seven nonbusiness judges still execute.
    h.base = h.candidate.clone();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    let before = marker(tools.path());
    manifest(&h.root(), &[]);
    let (e, empty) = collected(&h.root(), &h);
    passed(e, &empty);
    assert_eq!(empty["tests"]["selected"], json!([]));
    assert_eq!(empty["judges"].as_array().unwrap().len(), 7);
    assert_eq!(marker(tools.path()), before);
    // Supplied nonrequired originals are still validated.
    let (e, empty_unit) = launch(&h.root(), &h, &["--unit", "one"]);
    passed(e, &empty_unit);
    manifest(&h.root(), &["one"]);
    let (e, r) = collected(&h.root(), &h);
    passed(e, &r);
    let p = h.root().join(".chrono-harness/state/unit-one.json");
    let mut invalid = empty_unit;
    invalid["status"] = "warn".into();
    fs::write(&p, serde_json::to_vec(&invalid).unwrap()).unwrap();
    manifest(&h.root(), &["one"]);
    let (e, _) = collected(&h.root(), &h);
    assert_ne!(e, 0);
    println!(
        "EMPTY global_judges=7 business_launches=0 nonrequired_validated=true delivery_contribution_without_certificate=true"
    );
}
#[test]
fn cross_unit_replacement_and_joint_retirement_complete_at_collection() {
    let (mut h, _tools) = rust_host();
    // Move the replacement to the other explicitly assigned unit while retaining the unaffected pair.
    let projects = h.values.get_mut(PROJECTS).unwrap();
    projects["projects"][0]["test_project"] = "replacement".into();
    projects["projects"][1]["id"] = "replacement".into();
    projects["projects"][1]["actions"]["execute"]["operation"] = "execute.replacement".into();
    projects["owners"]
        .as_array_mut()
        .unwrap()
        .push(json!("replacement"));
    let fm = h.values.get_mut(FM).unwrap();
    for f in fm["files"].as_array_mut().unwrap() {
        if f["owner"] == "t" {
            f["owner"] = "replacement".into();
        }
        for edge in f["edges"].as_array_mut().unwrap() {
            if edge["to"] == "project:t" {
                edge["to"] = "project:replacement".into();
            }
        }
    }
    for e in fm["project_edges"].as_array_mut().unwrap() {
        if e["from"] == "project:t" {
            e["from"] = "project:replacement".into();
        }
        if e["to"] == "project:t" {
            e["to"] = "project:replacement".into();
        }
        if e["to"] == "test:t" {
            e["to"] = "test:replacement".into();
        }
    }
    let mut plan = fm["execution_plans"]
        .as_object_mut()
        .unwrap()
        .remove("test:t")
        .unwrap();
    plan["operations"][2] = "execute.replacement".into();
    fm["execution_plans"]["test:replacement"] = plan;
    fm["test_costs"][0]["test"] = "replacement".into();
    h.values.get_mut(CONFIG).unwrap()["execution_units"]["units"]["one"]["tests"] =
        json!(["test:t2"]);
    h.values.get_mut(CONFIG).unwrap()["execution_units"]["units"]["two"]["tests"] =
        json!(["test:replacement"]);
    h.values.get_mut(WORKFLOW).unwrap()["stability"][0]["tests"] = json!(["replacement"]);
    h.values.get_mut(WORKFLOW).unwrap()["integration"]["tests"] = json!(["replacement", "t2"]);
    h.values.get_mut(WORKFLOW).unwrap()["retirements"] = json!([{"kind":"test","id":"t","replacement":"replacement","reason":"same implementation purpose in separately assigned replacement"}]);
    h.save();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    for unit in ["one", "two"] {
        let (e, r) = launch(&h.root(), &h, &["--unit", unit]);
        passed(e, &r);
        assert_eq!(r["tests"]["removed"]["test:t"], "test:replacement");
        assert_eq!(workflow(&r)["completion"], "pending collection");
    }
    manifest(&h.root(), &["one", "two"]);
    let (e, r) = collected(&h.root(), &h);
    passed(e, &r);
    assert_eq!(r["tests"]["removed"]["test:t"], "test:replacement");
    // Start the joint-retirement DELTA from that fixed replacement candidate.
    h.base = h.candidate.clone();
    let projects = h.values.get_mut(PROJECTS).unwrap();
    projects["projects"]
        .as_array_mut()
        .unwrap()
        .retain(|p| p["id"] != "p" && p["id"] != "replacement");
    let fm = h.values.get_mut(FM).unwrap();
    fm["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["owner"] != "p" && f["owner"] != "replacement");
    fm["project_edges"].as_array_mut().unwrap().retain(|e| {
        !["project:p", "project:replacement"]
            .iter()
            .any(|id| e["from"] == *id || e["to"] == *id)
            && e["to"] != "test:replacement"
    });
    fm["execution_plans"]
        .as_object_mut()
        .unwrap()
        .remove("test:replacement");
    fm["test_costs"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["test"] != "replacement");
    for p in ["p", "t"] {
        fs::remove_dir_all(h.root().join(p)).unwrap();
    }
    h.values.get_mut(CONFIG).unwrap()["execution_units"]["units"] =
        json!({"one":{"tests":["test:t2"],"report_path":".chrono-harness/state/unit-one.json"}});
    h.values.get_mut(CONFIG).unwrap()["execution_units"]["shared_operations"] = json!({});
    h.values.get_mut(WORKFLOW).unwrap()["stability"]
        .as_array_mut()
        .unwrap()
        .retain(|s| s["id"] != "p");
    h.values.get_mut(WORKFLOW).unwrap()["integration"]["tests"] = json!(["t2"]);
    h.values.get_mut(WORKFLOW).unwrap()["retirements"] = json!([{"kind":"test","id":"replacement","replacement":null,"reason":"joint pair retirement"},{"kind":"project","id":"p","replacement":null,"reason":"joint pair retirement"}]);
    h.save();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    let (e, r) = launch(&h.root(), &h, &["--unit", "one"]);
    passed(e, &r);
    assert_eq!(r["tests"]["removed"], json!({"test:replacement":null}));
    manifest(&h.root(), &["one"]);
    let (e, r) = collected(&h.root(), &h);
    passed(e, &r);
    assert_eq!(r["tests"]["removed"], json!({"test:replacement":null}));
    println!("TRANSITIONS cross_unit_replacement=passed joint_retirement=passed");
}

#[test]
fn migration_units_defer_decision_and_collection_reuses_original_conversion() {
    let original = super::migration_host();
    let baseline = original.base.clone();
    let mut h = git_facts::bind_host(original);
    h.base = baseline;
    let tools = tempfile::tempdir().unwrap();
    let tool_root = fs::canonicalize(tools.path()).unwrap();
    let wrapper = tool_root.join("python-migration");
    let python = fs::canonicalize(&h.tool).unwrap();
    let launches = tool_root.join("business-launches");
    let bytes = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec '{}' \"$@\"\n",
        launches.display(),
        python.display()
    );
    fs::write(&wrapper, &bytes).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    h.tool = wrapper.clone();
    h.values.get_mut(CONFIG).unwrap()["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "python")
        .unwrap()["program"] = json!(wrapper);
    let input = h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["id"] == "interpreter")
        .unwrap();
    input["location"] = json!(wrapper);
    input["sha256"] = json!(sha256(bytes.as_bytes()));
    h.values.get_mut(CONFIG).unwrap()["protocol"]["timeout_seconds"] = json!(180);
    h.values.get_mut(CONFIG).unwrap()["execution_units"] = json!({"units":{"one":{"tests":["test:t"],"report_path":".chrono-harness/state/unit-one.json"},"two":{"tests":["test:decoder-tests"],"report_path":".chrono-harness/state/unit-two.json"}},"shared_operations":{},"collection_limits":{"manifest_bytes":1048576,"report_bytes":67108864},"report_path":".chrono-harness/state/collected.json"});
    let mut migration = h.values[WORKFLOW]["migrations"][0].clone();
    migration["to_version"] = json!(3);
    migration["reason"] = "candidate bound facts configuration migration".into();
    h.values.get_mut(WORKFLOW).unwrap()["migrations"]
        .as_array_mut()
        .unwrap()
        .push(migration);
    h.save();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    for unit in ["one", "two"] {
        let (e, r) = launch(&h.root(), &h, &["--unit", unit]);
        passed(e, &r);
        assert_eq!(workflow(&r)["completion"], "pending collection");
    }
    manifest(&h.root(), &["one", "two"]);
    let one_bytes = fs::metadata(h.root().join(".chrono-harness/state/unit-one.json"))
        .unwrap()
        .len();
    let two_bytes = fs::metadata(h.root().join(".chrono-harness/state/unit-two.json"))
        .unwrap()
        .len();
    println!(
        "MIGRATION_COLLECTION_INPUT unit_bytes=[{one_bytes},{two_bytes}] retained_hex_bytes={} report_bound={}",
        2 * (one_bytes + two_bytes),
        64 * 1024 * 1024,
    );
    fs::remove_file(&wrapper).unwrap();
    let before = marker(&tool_root);
    let (e, collected_report) = collected(&h.root(), &h);
    passed(e, &collected_report);
    assert_eq!(
        marker(&tool_root),
        before,
        "collector launched the retained conversion tool"
    );
    assert_eq!(
        workflow(&collected_report)["transitions"]["migrations"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    // Conversion provenance must come from an original successful process, not a summary.
    let path = h.root().join(".chrono-harness/state/unit-one.json");
    let original = fs::read(&path).unwrap();
    let mut invalid: Value = serde_json::from_slice(&original).unwrap();
    let registration = row(&mut invalid, "registration");
    invalid["judges"][registration]["response"]["outputs"]["registration_view"]["conversion"]["input_digest"] =
        json!("0".repeat(64));
    reseal(&mut invalid);
    fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    manifest(&h.root(), &["one", "two"]);
    let (e, _) = collected(&h.root(), &h);
    assert_ne!(e, 0);
    assert_eq!(marker(&tool_root), before);
    fs::write(&path, original).unwrap();
    let path = h.root().join(".chrono-harness/state/unit-two.json");
    let mut invalid: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let registration = row(&mut invalid, "registration");
    invalid["judges"][registration]["response"]["outputs"]["registration_view"]["conversion"]["process"]
        ["exit_code"] = json!(7);
    reseal(&mut invalid);
    fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    manifest(&h.root(), &["one", "two"]);
    let (e, rejected) = collected(&h.root(), &h);
    assert_ne!(e, 0);
    assert!(
        rejected["findings"]
            .to_string()
            .contains("E_PROCESS_EVIDENCE")
    );
    assert_eq!(marker(&tool_root), before);
    println!(
        "MIGRATION units=2 completed_decisions=2 retained_conversion=true additional_business_launches=0 invalid_conversion=2_rejected"
    );
}

#[test]
fn collection_manifest_must_not_be_its_output_destination() {
    let (h, tools) = rust_host();
    for path in [
        ".chrono-harness/state/collected.json",
        ".chrono-harness/state//collected.json",
        ".chrono-harness/state//unit-one.json",
    ] {
        let destination = h.root().join(path);
        let input = b"original manifest bytes must remain unchanged";
        fs::write(&destination, input).unwrap();
        let before = marker(tools.path());
        let (e, report) = launch(&h.root(), &h, &["--collect", path]);
        assert_eq!(e, 2, "{report}");
        assert!(
            report["stderr"]
                .as_str()
                .unwrap()
                .contains("collection manifest overlaps")
        );
        assert_eq!(fs::read(&destination).unwrap(), input);
        assert_eq!(marker(tools.path()), before);
    }
}

#[test]
fn unit_output_symlink_collision_must_reject_before_business_effects() {
    let (h, tools) = rust_host();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    fs::remove_file(h.root().join(".chrono-harness/state/integration.json")).unwrap();
    let state = h.root().join(".chrono-harness/state");
    std::os::unix::fs::symlink("report.json", state.join("unit-one.json")).unwrap();
    let original = fs::read(state.join("report.json")).unwrap();
    let before = marker(tools.path());
    let (e, _report) = launch(&h.root(), &h, &["--unit", "one"]);
    assert_ne!(e, 0);
    assert_eq!(marker(tools.path()), before);
    assert_eq!(original, fs::read(state.join("report.json")).unwrap());
}

#[test]
fn nested_full_unit_and_collection_destinations_are_prepared_before_publish() {
    let (mut h, _tools) = rust_host();
    h.values.get_mut(CONFIG).unwrap()["execution_units"]["units"]["one"]["report_path"] =
        ".chrono-harness/state/nested/one/report.json".into();
    h.values.get_mut(CONFIG).unwrap()["execution_units"]["report_path"] =
        ".chrono-harness/state/nested/collection/report.json".into();
    h.save();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    let (e, unit) = launch(&h.root(), &h, &["--unit", "one"]);
    passed(e, &unit);
    let unit_path = h
        .root()
        .join(".chrono-harness/state/nested/one/report.json");
    assert!(unit_path.is_file());
    let (e, unit_two) = launch(&h.root(), &h, &["--unit", "two"]);
    passed(e, &unit_two);
    let unit_two_path = h.root().join(".chrono-harness/state/unit-two.json");
    let manifest_path = h.root().join(".chrono-harness/state/manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec(&json!({
            "schema":"chrono-full-collection/v1",
            "reports":[
                {"unit":"one","path":".chrono-harness/state/nested/one/report.json","sha256":sha256(&fs::read(&unit_path).unwrap())},
                {"unit":"two","path":".chrono-harness/state/unit-two.json","sha256":sha256(&fs::read(&unit_two_path).unwrap())}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let (e, collected_report) = launch(
        &h.root(),
        &h,
        &["--collect", ".chrono-harness/state/manifest.json"],
    );
    passed(e, &collected_report);
    assert!(
        h.root()
            .join(".chrono-harness/state/nested/collection/report.json")
            .is_file()
    );
}

#[test]
fn collection_rejects_raw_entry_mismatch_without_business_effects() {
    let (mut h, tools) = rust_host();
    h.values.get_mut(WORKFLOW).unwrap()["integration"]["tests"] = json!(["t"]);
    h.save();
    let (e, r) = git_facts::run_bound(&h, "integration", None);
    passed(e, &r);
    let (e, mut unit) = launch(&h.root(), &h, &["--unit", "one"]);
    passed(e, &unit);
    unit["request"]["observations"]["entry"]["argv"][3] =
        ".chrono-harness/state/other-config.json".into();
    let unit_path = h.root().join(".chrono-harness/state/unit-one.json");
    fs::write(&unit_path, serde_json::to_vec(&unit).unwrap()).unwrap();
    manifest(&h.root(), &["one"]);
    let before = marker(tools.path());
    let (e, report) = collected(&h.root(), &h);
    assert_ne!(e, 0);
    assert!(!report["findings"].as_array().unwrap().is_empty());
    assert_eq!(marker(tools.path()), before);
}

#[path = "full_short.rs"]
mod full_short;

#[test]
fn candidate_unit_projection_preserves_historical_plan_inputs() {
    use chrono_harness::units::Scope;
    use chrono_judge_registration::{Registrations, inputs::project_effective};
    let (h, _tools) = rust_host();
    let mut old_values = h.values.clone();
    for (id, consumer) in [
        ("historical-sdk", "project:t"),
        ("selected-sdk", "project:t2"),
    ] {
        let cfg = old_values.get_mut(CONFIG).unwrap();
        cfg["environment"]["inputs"].as_array_mut().unwrap().push(json!({
            "id":id,"location":format!("/declared/{id}"),"presence":"present","sha256":sha256(id.as_bytes())
        }));
        cfg["input_closure"]["bindings"].as_array_mut().unwrap().push(json!({
            "id":id,"consumer":consumer,"kind":"explicit-fixture","inputs":[format!("input:{id}")]
        }));
        old_values.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .push(edge(&format!("input:{id}"), "runtime-input", consumer));
    }
    let old = Registrations::load(&old_values, CONFIG).unwrap();
    for case in ["rename", "introduction", "reassignment"] {
        let mut new_values = old_values.clone();
        let units = new_values.get_mut(CONFIG).unwrap()["execution_units"]["units"]
            .as_object_mut()
            .unwrap();
        let unit = if case == "reassignment" {
            units.get_mut("one").unwrap()["tests"] = json!(["test:t2"]);
            units.get_mut("two").unwrap()["tests"] = json!(["test:t"]);
            "one"
        } else {
            let mut definition = units.remove("one").unwrap();
            definition["report_path"] = json!(".chrono-harness/state/unit-new.json");
            units.insert("new".into(), definition);
            if case == "introduction" {
                units.insert(
                    "one".into(),
                    json!({"tests":[],"report_path":".chrono-harness/state/unit-one.json"}),
                );
            }
            "new"
        };
        if case != "reassignment" {
            new_values.get_mut(CONFIG).unwrap()["execution_units"]["shared_operations"]["shared.prepare"] =
                json!(["new", "two"]);
        }
        let new = Registrations::load(&new_values, CONFIG).unwrap();
        let mut evidence = json!({"endpoints":{"base":{"files":{}},"candidate":{"files":{}}}});
        for endpoint in ["base", "candidate"] {
            for input in old.config()["environment"]["inputs"].as_array().unwrap() {
                evidence["endpoints"][endpoint]["files"][input["id"].as_str().unwrap()] =
                    json!({"sha256":input["sha256"]});
            }
        }
        let scope = Some(Scope::Unit { unit: unit.into() });
        // Missing unrelated inputs are independent at both endpoints.
        if case != "reassignment" {
            for endpoint in ["base", "candidate"] {
                evidence["endpoints"][endpoint]["files"]
                    .as_object_mut()
                    .unwrap()
                    .remove("selected-sdk");
            }
        }
        evidence["identity"] = json!(wire::digest(&evidence["endpoints"]).unwrap());
        let projected = project_effective(&evidence, &old, &new, scope.as_ref())
            .unwrap_or_else(|e| panic!("{case}: {e}"));
        if case == "reassignment" {
            assert!(
                projected["endpoints"]["base"]["files"]
                    .get("selected-sdk")
                    .is_some()
            );
            assert!(
                projected["endpoints"]["candidate"]["files"]
                    .get("historical-sdk")
                    .is_none()
            );
            let mut missing = evidence.clone();
            missing["endpoints"]["base"]["files"]
                .as_object_mut()
                .unwrap()
                .remove("selected-sdk");
            missing["identity"] = json!(wire::digest(&missing["endpoints"]).unwrap());
            assert!(
                project_effective(&missing, &old, &new, scope.as_ref())
                    .unwrap_err()
                    .contains("missing effective input base: selected-sdk")
            );
        }
        assert!(
            projected["endpoints"]["base"]["files"]
                .get("historical-sdk")
                .is_some()
        );
        let selected = if case == "reassignment" {
            "selected-sdk"
        } else {
            "historical-sdk"
        };
        assert!(
            projected["endpoints"]["candidate"]["files"]
                .get(selected)
                .is_some()
        );
        for (endpoint, id) in [("base", "historical-sdk"), ("candidate", selected)] {
            let mut missing = evidence.clone();
            missing["endpoints"][endpoint]["files"]
                .as_object_mut()
                .unwrap()
                .remove(id);
            missing["identity"] = json!(wire::digest(&missing["endpoints"]).unwrap());
            let error = project_effective(&missing, &old, &new, scope.as_ref()).unwrap_err();
            assert!(
                error.contains(&format!("missing effective input {endpoint}: {id}")),
                "{case}: {error}"
            );
        }
    }
}

#[test]
fn collection_binds_canonical_runner_resolution_to_original_declaration() {
    let (h, tools) = rust_host();
    git_facts::prepare_bound(&h, "integration", None);
    for unit in ["one", "two"] {
        let (e, r) = launch(&h.root(), &h, &["--unit", unit]);
        passed(e, &r);
    }
    let collector = clone_host(&h);
    fs::remove_dir_all(h.root()).unwrap();
    fs::remove_file(tools.path().join("cargo-fixture")).unwrap();
    let before = marker(tools.path());
    manifest(collector.path(), &["one", "two"]);
    let (e, r) = collected(collector.path(), &h);
    passed(e, &r);
    let path = collector.path().join(".chrono-harness/state/unit-one.json");
    let original = fs::read(&path).unwrap();
    let mut report: Value = serde_json::from_slice(&original).unwrap();
    for record in report["judges"].as_array_mut().unwrap() {
        *record = chrono_harness::full::expand_record(record).unwrap();
    }
    assert_eq!(
        report["request"]["runner"]["path"],
        json!(
            Path::new(report["request"]["candidate"]["root"].as_str().unwrap())
                .join(".chrono-harness/bin/chrono-harness")
        )
    );
    report["request"]["observations"]["entry"]["resolved_paths"]["paths"][0] = json!({
        "actual":"/outside/chrono-harness","expected":"/outside/chrono-harness"
    });
    let ri = row(&mut report, "routes");
    report["judges"][ri]["response"]["outputs"]["execution_plan"]["binding"]["entry"] =
        report["request"]["observations"]["entry"].clone();
    reseal_plan(&mut report);
    reseal(&mut report);
    let request: Request = serde_json::from_value(report["request"].clone()).unwrap();
    let bindings: Vec<Binding> =
        serde_json::from_value(h.values[JUDGES]["judges"].clone()).unwrap();
    chrono_harness::full::retained_judges(
        &request,
        &bindings,
        report["judges"].as_array().unwrap(),
    )
    .unwrap();
    fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
    manifest(collector.path(), &["one", "two"]);
    let (e, rejected) = collected(collector.path(), &h);
    assert_ne!(e, 0, "accepted canonical contradictory resolution");
    assert!(
        rejected["findings"]
            .to_string()
            .contains("E_COLLECTION_INPUT: original runner invocation differs"),
        "{rejected}"
    );
    assert_eq!(marker(tools.path()), before);
    fs::write(path, original).unwrap();
}

#[test]
fn unit_inputs_are_local_but_shared_and_governance_inputs_remain_required() {
    let (mut h, tools) = rust_host();
    let root = h.root();
    for (id, consumer) in [
        ("one-sdk", "project:t"),
        ("two-sdk", "project:t2"),
        ("shared-sdk", "project:p2"),
        ("governance-data", "judge:registration"),
    ] {
        let path = fs::canonicalize(tools.path()).unwrap().join(id);
        fs::write(&path, id.as_bytes()).unwrap();
        let cfg = h.values.get_mut(CONFIG).unwrap();
        cfg["environment"]["inputs"].as_array_mut().unwrap().push(
            json!({"id":id,"location":path,"presence":"present","sha256":sha256(id.as_bytes())}),
        );
        cfg["input_closure"]["bindings"].as_array_mut().unwrap().push(json!({"id":id,"consumer":consumer,"kind":"explicit-fixture","inputs":[format!("input:{id}")]}));
        h.values.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .push(edge(&format!("input:{id}"), "runtime-input", consumer));
    }
    h.save();
    h.base = h.candidate.clone();
    // Candidate introduction/rename must project the old plans, although the
    // base already has units and has never declared this candidate unit ID.
    let cfg = h.values.get_mut(CONFIG).unwrap();
    let definition = cfg["execution_units"]["units"]
        .as_object_mut()
        .unwrap()
        .remove("one")
        .unwrap();
    cfg["execution_units"]["units"]["renamed"] = definition;
    cfg["execution_units"]["shared_operations"]["shared.prepare"][0] = json!("renamed");
    fs::write(
        root.join("p/src/lib.rs"),
        "pub fn double(n:i32)->i32 {2*n}\n",
    )
    .unwrap();
    fs::write(
        root.join("p2/src/lib.rs"),
        "pub fn double(n:i32)->i32 {2*n}\n",
    )
    .unwrap();
    h.save();
    git_facts::prepare_bound(&h, "integration", None);
    let path = root.join(".chrono-harness/state/inputs.json");
    let mut snapshots: Value = chrono_harness::json(&fs::read(&path).unwrap()).unwrap();
    for endpoint in ["base", "candidate"] {
        for id in ["one-sdk", "two-sdk", "shared-sdk", "governance-data"] {
            snapshots[endpoint]["files"][id] = json!({"bytes":id.as_bytes()});
        }
    }
    fs::write(&path, serde_json::to_vec(&snapshots).unwrap()).unwrap();
    let original = snapshots.clone();
    fs::remove_file(tools.path().join("two-sdk")).unwrap();
    // A wrong-shaped unrelated live location must never be opened by capture.
    fs::create_dir(tools.path().join("two-sdk")).unwrap();
    let capture = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
        .current_dir(&root)
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .args([
            "capture",
            "--host-root",
            ".",
            "--config",
            CONFIG,
            "--commit",
            &h.candidate,
            "--output",
            ".chrono-harness/state/one-captured.json",
            "--unit",
            "renamed",
        ])
        .output()
        .unwrap();
    assert!(
        capture.status.success(),
        "{}",
        String::from_utf8_lossy(&capture.stderr)
    );
    let captured = chrono_harness::json(&capture.stdout).unwrap();
    assert!(captured["files"].get("two-sdk").is_none());
    for id in ["one-sdk", "shared-sdk", "governance-data"] {
        assert!(captured["files"].get(id).is_some(), "capture omitted {id}");
    }
    fs::remove_dir(tools.path().join("two-sdk")).unwrap();
    for endpoint in ["base", "candidate"] {
        snapshots[endpoint]["files"]
            .as_object_mut()
            .unwrap()
            .remove("two-sdk");
    }
    fs::write(&path, serde_json::to_vec(&snapshots).unwrap()).unwrap();
    let (e, one) = launch(&root, &h, &["--unit", "renamed"]);
    passed(e, &one);
    let original_one_report = fs::read(root.join(".chrono-harness/state/unit-one.json")).unwrap();
    for endpoint in ["base", "candidate"] {
        let mut missing = snapshots.clone();
        missing[endpoint]["files"]
            .as_object_mut()
            .unwrap()
            .remove("one-sdk");
        fs::write(&path, serde_json::to_vec(&missing).unwrap()).unwrap();
        let before = marker(tools.path());
        let (e, rejected) = launch(&root, &h, &["--unit", "renamed"]);
        assert_ne!(e, 0, "accepted missing {endpoint} input");
        assert!(
            rejected["findings"]
                .to_string()
                .contains(&format!("{endpoint}: missing retained input one-sdk")),
            "{rejected}"
        );
        assert_eq!(marker(tools.path()), before);
    }
    fs::write(&path, serde_json::to_vec(&snapshots).unwrap()).unwrap();
    assert!(
        one["effective_inputs"]["endpoints"]["candidate"]["files"]
            .get("two-sdk")
            .is_none()
    );
    for id in ["one-sdk", "shared-sdk", "governance-data"] {
        let bytes = fs::read(tools.path().join(id)).unwrap();
        fs::remove_file(tools.path().join(id)).unwrap();
        let before = marker(tools.path());
        let (e, r) = launch(&root, &h, &["--unit", "renamed"]);
        assert_ne!(e, 0, "missing required {id}");
        assert!(r["findings"].to_string().contains(id), "{r:#}");
        assert_eq!(marker(tools.path()), before);
        fs::write(tools.path().join(id), bytes).unwrap();
    }
    // Original per-unit evidence may have different input projections.
    fs::write(tools.path().join("two-sdk"), b"two-sdk").unwrap();
    snapshots = original.clone();
    for endpoint in ["base", "candidate"] {
        snapshots[endpoint]["files"]
            .as_object_mut()
            .unwrap()
            .remove("one-sdk");
    }
    fs::write(&path, serde_json::to_vec(&snapshots).unwrap()).unwrap();
    fs::remove_file(tools.path().join("one-sdk")).unwrap();
    let (e, two) = launch(&root, &h, &["--unit", "two"]);
    passed(e, &two);
    assert!(
        two["effective_inputs"]["endpoints"]["candidate"]["files"]
            .get("one-sdk")
            .is_none()
    );
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    // Transport the original successful contribution after the failed retry probes.
    fs::write(
        root.join(".chrono-harness/state/unit-one.json"),
        original_one_report,
    )
    .unwrap();
    let reports: Vec<_> = [("renamed", "one"), ("two", "two")]
        .into_iter()
        .map(|(unit, coordinate)| {
            let path = format!(".chrono-harness/state/unit-{coordinate}.json");
            json!({"unit":unit,"path":path,"sha256":sha256(&fs::read(root.join(&path)).unwrap())})
        })
        .collect();
    fs::write(
        root.join(".chrono-harness/state/manifest.json"),
        serde_json::to_vec(&json!({"schema":"chrono-full-collection/v1","reports":reports}))
            .unwrap(),
    )
    .unwrap();
    fs::remove_file(tools.path().join("two-sdk")).unwrap();
    fs::remove_file(tools.path().join("cargo-fixture")).unwrap();
    let before = marker(tools.path());
    let (e, collection) = collected(&root, &h);
    passed(e, &collection);
    assert_eq!(marker(tools.path()), before);
    assert_eq!(collection["tests"]["executed"], json!([]));
    // Known omitted inputs, unknown inputs and malformed inputs are still failures.
    for case in ["missing", "unknown", "malformed"] {
        let mut broken = original.clone();
        match case {
            "missing" => {
                broken["candidate"]["files"]
                    .as_object_mut()
                    .unwrap()
                    .remove("one-sdk");
            }
            "unknown" => {
                broken["candidate"]["files"]["unknown-sdk"] = json!({"absent":true});
            }
            _ => {
                broken["candidate"]["files"]["one-sdk"] = json!([]);
            }
        }
        fs::write(&path, serde_json::to_vec(&broken).unwrap()).unwrap();
        let (e, _) = collected(&root, &h);
        assert_ne!(e, 0, "accepted {case}");
        assert_eq!(marker(tools.path()), before);
    }
}

#[test]
fn collection_missing_named_outputs_reaches_checked_semantic_validator() {
    let (h, tools) = rust_host();
    git_facts::prepare_bound(&h, "integration", None);
    for unit in ["one", "two"] {
        let (e, r) = launch(&h.root(), &h, &["--unit", unit]);
        passed(e, &r);
    }
    let path = h.root().join(".chrono-harness/state/unit-one.json");
    let original = fs::read(&path).unwrap();
    let before = marker(tools.path());
    for (judge, output) in [
        ("registration", "registration_view"),
        ("routes", "execution_plan"),
        ("projects", "tests"),
    ] {
        let mut report: Value = chrono_harness::json(&original).unwrap();
        for r in report["judges"].as_array_mut().unwrap() {
            *r = chrono_harness::full::expand_record(r).unwrap();
        }
        let index = row(&mut report, judge);
        report["judges"][index]["response"]["outputs"]
            .as_object_mut()
            .unwrap()
            .remove(output);
        reseal(&mut report);
        let request: Request = serde_json::from_value(report["request"].clone()).unwrap();
        let bindings: Vec<Binding> =
            serde_json::from_value(h.values[JUDGES]["judges"].clone()).unwrap();
        chrono_harness::full::retained_judges(
            &request,
            &bindings,
            report["judges"].as_array().unwrap(),
        )
        .expect("consistent original transport must reach the named output consumer");
        fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
        manifest(&h.root(), &["one", "two"]);
        let (e, rejected) = collected(&h.root(), &h);
        assert_ne!(e, 0);
        let expected =
            format!("E_COLLECTION_INPUT: {judge} judge missing required output {output}");
        assert!(
            rejected["findings"].to_string().contains(&expected),
            "{rejected:#}"
        );
        assert_eq!(marker(tools.path()), before);
    }
    fs::write(&path, original).unwrap();
}
