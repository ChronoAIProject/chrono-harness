//! Real seven-judge short entry, existing local producers and native unit transport.
use super::*;
const PROVIDER: &str = ".chrono-harness/ci/units.json";
const WORKTREE: &str = ".chrono-harness/worktree.json";
fn short_host(native: bool) -> (Host, tempfile::TempDir) {
    configured_short_host(native, None)
}
fn configured_short_host(native: bool, age_seconds: Option<f64>) -> (Host, tempfile::TempDir) {
    configured_short_host_with_governance(native, age_seconds, false)
}
fn configured_short_host_with_governance(
    native: bool,
    age_seconds: Option<f64>,
    governance: bool,
) -> (Host, tempfile::TempDir) {
    let (h, tools) = rust_host();
    adopt_short_host(h, tools, native, age_seconds, governance, false)
}
fn adopt_short_host(
    mut h: Host,
    tools: tempfile::TempDir,
    native: bool,
    age_seconds: Option<f64>,
    governance: bool,
    migration: bool,
) -> (Host, tempfile::TempDir) {
    let root = h.root();
    for (project, binary) in [("ci", "chrono-ci"), ("worktree", "chrono-worktree")] {
        fs::copy(
            source().join(format!("crates/{project}/target/debug/{binary}")),
            root.join(format!(".chrono-harness/bin/{binary}")),
        )
        .unwrap();
    }
    let git_bin = chrono_harness::resolve_program(&root, "git", None).unwrap();
    let cfg = h.values.get_mut(CONFIG).unwrap();
    // Retain only the execution PATH this real Rust fixture needs, rather than
    // unrelated editor/agent paths repeated in every original Git observation.
    let cargo = chrono_harness::resolve_program(&root, "cargo", None).unwrap();
    cfg["environment"]["values"]["PATH"] = json!(format!(
        "{}:/usr/bin:/bin",
        cargo.parent().unwrap().display()
    ));
    cfg["schema_version"] = json!(4);
    cfg["canonical_check"] = json!({"operation":"validate.delta","argv":[".chrono-harness/bin/chrono-harness","check"],"profile":CONFIG,"inputs":{"local":{"operation":"prepare.local","tool":"chrono-worktree","argv":["check-inputs","--config",WORKTREE]},"ci":{"operation":"prepare.ci","tool":"chrono-ci","argv":["check-inputs","--config",PROVIDER,"--event-env","GITHUB_EVENT_NAME","--payload-env","GITHUB_EVENT_PATH","--revision-env","CHRONO_WORKFLOW_REVISION","--repository-env","GITHUB_REPOSITORY"]}}});
    cfg["environment"]["inherit"]
        .as_array_mut()
        .unwrap()
        .push(json!("CHRONO_CHECK_SOURCE"));
    let mut added_edges = vec![];
    for id in ["chrono-ci", "chrono-worktree"] {
        cfg["tools"].as_array_mut().unwrap().push(json!({"id":id,"program":format!(".chrono-harness/bin/{id}"),"resolution":"PATH-once","version_argv":["--version"],"expected_version":format!("{id} 0.1.0")}));
        let node = format!("tool:{id}");
        cfg["input_closure"]["bindings"][0]["inputs"]
            .as_array_mut()
            .unwrap()
            .push(json!(node));
        added_edges.push(edge(&node, "runtime-input", "judge:registration"));
    }
    if native {
        for key in [
            "GITHUB_EVENT_NAME",
            "GITHUB_EVENT_PATH",
            "CHRONO_WORKFLOW_REVISION",
            "GITHUB_REPOSITORY",
        ] {
            cfg["environment"]["inherit"]
                .as_array_mut()
                .unwrap()
                .push(json!(key));
            cfg["input_closure"]["bindings"][0]["inputs"]
                .as_array_mut()
                .unwrap()
                .push(json!(format!("environment:{key}")));
            added_edges.push(edge(
                &format!("environment:{key}"),
                "runtime-input",
                "judge:registration",
            ));
        }
    }
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .extend(added_edges);
    let cfg = h.values.get_mut(CONFIG).unwrap();
    cfg["input_closure"]["bindings"][0]["inputs"]
        .as_array_mut()
        .unwrap()
        .push(json!("environment:CHRONO_CHECK_SOURCE"));
    cfg["execution_units"]["report_path"] = json!(".chrono-harness/state/collection/check.json");
    for id in ["one", "two"] {
        cfg["execution_units"]["units"][id]["report_path"] =
            json!(format!(".chrono-harness/state/{id}/check.json"));
    }
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(edge(
            "environment:CHRONO_CHECK_SOURCE",
            "runtime-input",
            "judge:registration",
        ));
    let worktree = json!({"schema":"chrono-worktree-config/v2","host_config":CONFIG,"remote":"origin","git":{"program":git_bin,"expected_version":null,"sha256":sha256(&fs::read(&git_bin).unwrap())},"environment":{"inherit":["HOME"],"values":{"PATH":h.values[CONFIG]["environment"]["values"]["PATH"],"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"}},"timeout_seconds":30,"output_limit_bytes":1048576,"report_directory":".chrono-harness/state/worktrees/","check_inputs":{"origin_path":".chrono-harness/state/origin.json","context_path":".chrono-harness/state/local/context.json","collection_manifest":".chrono-harness/state/collection/manifest.json","roles":{"integration":"integration","feature":"delivery"}}});
    let common = json!({"schema":"chrono-github-ci/v4","workflow_path":".github/workflows/collection.yml","name":"Full collection","runs_on":"fixture-native","push_branches":["dev","integration/**"],"pull_request_branches":["dev"],"branch_creation_base_ref":"refs/heads/dev","checkout_action":"actions/checkout@11d5960a326750d5838078e36cf38b85af677262","upload_artifact_action":"actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02","timeout_minutes":20,"bootstrap":["/bin/true"],"runner":".chrono-harness/bin/chrono-harness","generator":".chrono-harness/bin/chrono-ci","check_config":CONFIG,"facts_config":CONFIG,"context_path":".chrono-harness/state/collection/event.json","artifact_directory":".chrono-harness/state/collection/"});
    let mut provider = json!({"schema":"chrono-github-units/v2","collection":common,"units":{},"full_contexts":{"collection":".chrono-harness/state/collection/full.json","units":{}},"gather":{"program":tools.path().join("mock-gh"),"inherit_environment":[],"credential_environment":[],"environment":{},"timeout_seconds":30,"output_limit_bytes":1048576,"wait_seconds":1,"poll_seconds":1,"manifest_path":".chrono-harness/state/collection/manifest.json","download_directory":".chrono-harness/state/collection/downloads/","report_path":".chrono-harness/state/collection/gather.json"}});
    for id in ["one", "two"] {
        provider["units"][id] = json!({"workflow_path":format!(".github/workflows/{id}.yml"),"name":format!("Full {id}"),"runs_on":"fixture-native","timeout_minutes":20,"bootstrap":["/bin/true"],"context_path":format!(".chrono-harness/state/{id}/event.json"),"artifact_directory":format!(".chrono-harness/state/{id}/")});
        provider["full_contexts"]["units"][id] =
            json!(format!(".chrono-harness/state/{id}/full.json"));
    }
    for (path, value) in [(WORKTREE, worktree), (PROVIDER, provider)] {
        fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        fs::write(root.join(path), serde_json::to_vec(&value).unwrap()).unwrap();
        let mut f = file(path, json!([]));
        f["surface"] = json!("documentation");
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(f);
    }
    h.save();
    let out = Command::new(root.join(".chrono-harness/bin/chrono-ci"))
        .current_dir(&root)
        .args(["generate", "--host-root", ".", "--config", PROVIDER])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for id in ["one", "two", "collection"] {
        let mut f = file(&format!(".github/workflows/{id}.yml"), json!([]));
        f["surface"] = json!("documentation");
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(f);
    }
    for prod in if migration { vec![] } else { vec!["p", "p2"] } {
        fs::write(
            root.join(format!("{prod}/src/lib.rs")),
            "pub fn double(n:i32)->i32 {n*2}\n",
        )
        .unwrap();
    }
    if governance {
        let path = ".chrono-harness/ci/governance-data";
        fs::write(root.join(path), b"genuine base governance H0").unwrap();
        h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"].as_array_mut().unwrap().push(
            json!({"id":"governance-data","location":path,"presence":"present","sha256":sha256(b"genuine base governance H0")}),
        );
        h.values.get_mut(CONFIG).unwrap()["input_closure"]["bindings"][0]["inputs"]
            .as_array_mut()
            .unwrap()
            .push(json!("input:governance-data"));
        let mut f = file(path, json!([]));
        f["surface"] = json!("documentation");
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(f);
        h.values.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .push(edge(
                "input:governance-data",
                "runtime-input",
                "judge:registration",
            ));
    }
    if let Some(seconds) = age_seconds {
        h.values.get_mut(WORKFLOW).unwrap()["staleness"]["max_age_hours"] = json!(seconds / 3600.0);
    }
    // Both endpoints retain the same short-entry producer registrations.
    let current = h.values.clone();
    if migration {
        let fm = h.values.get_mut(FM).unwrap();
        fm["schema_version"] = json!(1);
        fm.as_object_mut().unwrap().remove("execution_plans");
    }
    h.save();
    h.base = h.candidate.clone();
    h.values = current;
    // A local filesystem remote is the existing producer's registered fixed target.
    let remote = tools.path().join("target.git");
    git(&root, &["branch", "-f", "dev", &h.base]);
    git(
        &root,
        &[
            "clone",
            "--bare",
            "-q",
            root.to_str().unwrap(),
            remote.to_str().unwrap(),
        ],
    );
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    for prod in if migration { vec![] } else { vec!["p", "p2"] } {
        fs::write(
            root.join(format!("{prod}/src/lib.rs")),
            "pub fn double(n:i32)->i32 {n+n}\n",
        )
        .unwrap();
    }
    h.save();
    (h, tools)
}
// Read the registered publication and verify its immutable original. Short
// stdout is the landed human projection, not a JSON report protocol.
fn short_report(
    root: &Path,
    args: &[&str],
    output: &std::process::Output,
    source: Option<&str>,
) -> Value {
    if output.stdout.is_empty() {
        assert!(!output.status.success());
        return json!({"stderr":String::from_utf8_lossy(&output.stderr)});
    }
    assert!(output.stderr.is_empty());
    let config = chrono_harness::json(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    let path = match args {
        ["check", "--unit", unit] => config["execution_units"]["units"][*unit]["report_path"]
            .as_str()
            .unwrap()
            .to_owned(),
        ["check", "--collect"] => config["execution_units"]["report_path"]
            .as_str()
            .unwrap()
            .to_owned(),
        ["check"] if source == Some("ci") => {
            let provider = chrono_harness::json(&fs::read(root.join(PROVIDER)).unwrap()).unwrap();
            format!(
                "{}report.json",
                provider["collection"]["artifact_directory"]
                    .as_str()
                    .unwrap()
            )
        }
        ["check"] => ".chrono-harness/state/report.json".to_owned(),
        _ => panic!("unexpected short fixture arguments: {args:?}"),
    };
    let bytes = fs::read(root.join(path)).unwrap();
    let report = chrono_harness::json(&bytes).unwrap();
    let original = report["report_path"].as_str().unwrap();
    assert_eq!(bytes, fs::read(root.join(original)).unwrap());
    let console = String::from_utf8_lossy(&output.stdout);
    assert!(
        console.starts_with(&format!(
            "check: {} (exit {})\n",
            report["status"].as_str().unwrap(),
            output.status.code().unwrap()
        )),
        "{console}"
    );
    assert!(
        console.contains(&format!("Original report: {original}\n")),
        "{console}"
    );
    assert!(output.stdout.len() < 16_384, "{console}");
    report
}
fn short(root: &Path, args: &[&str], source: Option<&str>) -> (i32, Value) {
    let root = fs::canonicalize(root).unwrap();
    let mut command = Command::new(root.join(".chrono-harness/bin/chrono-harness"));
    command
        .current_dir(&root)
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .env_remove("CHRONO_CHECK_SOURCE")
        .args(args);
    if let Some(source) = source {
        command.env("CHRONO_CHECK_SOURCE", source);
    }
    let output = command.output().unwrap();
    if !output.status.success() {
        println!("SHORT_FAILURE {}", String::from_utf8_lossy(&output.stderr));
        if source == Some("ci") && args == ["check", "--collect"] && output.stdout.is_empty() {
            let retained = tempfile::Builder::new()
                .prefix("chrono-native-collection-failure-")
                .tempdir()
                .unwrap()
                .keep();
            copy_tree(&root.join(".chrono-harness/state"), &retained.join("state"));
            fs::write(retained.join("stdout"), &output.stdout).unwrap();
            fs::write(retained.join("stderr"), &output.stderr).unwrap();
            fs::write(
                retained.join("process.json"),
                serde_json::to_vec(&json!({
                    "exit_code":output.status.code(),"status":output.status.to_string()
                }))
                .unwrap(),
            )
            .unwrap();
            eprintln!(
                "original native collection process evidence: {}",
                retained.display()
            );
        }
    }
    let report = short_report(&root, args, &output, source);
    println!(
        "SHORT {:?} source={:?} exit={} bytes={}",
        args,
        source,
        output.status.code().unwrap(),
        output.stdout.len()
    );
    (output.status.code().unwrap(), report)
}

fn local_collection_observation(root: &Path, report: &Value) {
    let preparation = &report["preparation"];
    let endpoint = &preparation["request"]["prepared"];
    let result = &preparation["result"];
    let observation = &endpoint["evidence"]["current_observation"];
    assert!(observation.is_object(), "endpoint observation missing");
    assert_eq!(result["evidence"]["current_observation"], *observation);
    assert_eq!(result["context"], endpoint["context"]);
    assert_eq!(observation["context_digest"], report["context_digest"]);
    let read = |evidence: &Value| {
        let bytes = fs::read(root.join(evidence["report_path"].as_str().unwrap())).unwrap();
        assert_eq!(sha256(&bytes), evidence["report_sha256"]);
        chrono_harness::json(&bytes).unwrap()
    };
    let original_endpoint = read(&endpoint["evidence"]);
    let original_collection = read(&result["evidence"]);
    assert_eq!(original_endpoint["current_observation"], *observation);
    assert_eq!(original_collection["current_observation"], *observation);
    assert_eq!(
        original_collection["endpoint_evidence"],
        endpoint["evidence"]
    );
    let receipts = preparation["receipts"].as_array().unwrap();
    assert_eq!(receipts.len(), 2);
    for (index, receipt) in receipts.iter().enumerate() {
        let raw = fs::read(root.join(receipt["path"].as_str().unwrap())).unwrap();
        assert_eq!(sha256(&raw), receipt["sha256"]);
        let receipt = chrono_harness::json(&raw).unwrap();
        let stdout: Vec<u8> =
            serde_json::from_value(receipt["process"]["stdout_bytes"].clone()).unwrap();
        let produced = chrono_harness::json(&stdout).unwrap();
        assert_eq!(produced, if index == 0 { endpoint } else { result }.clone());
        assert_eq!(receipt["process"]["exit_code"], 0);
        assert_eq!(receipt["process"]["sha256"], receipt["tool"]["sha256"]);
        if index == 1 {
            assert_eq!(receipt["request"], preparation["request"]);
            assert_eq!(receipt["action"]["tool"], "chrono-ci");
        }
    }
    for original in endpoint["originals"].as_array().unwrap() {
        assert!(result["originals"].as_array().unwrap().contains(original));
        assert_eq!(
            sha256(&fs::read(root.join(original["path"].as_str().unwrap())).unwrap()),
            original["sha256"]
        );
    }
}
#[test]
fn full_provider_report_aliases_fail_before_generation_effects() {
    let (h, tools) = short_host(false);
    let root = h.root();
    let config = fs::read(root.join(CONFIG)).unwrap();
    let workflows: Vec<_> = ["one", "two", "collection"]
        .into_iter()
        .map(|id| {
            let path = root.join(format!(".github/workflows/{id}.yml"));
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    for case in ["unit-context", "collection-manifest"] {
        let mut cfg: Value = chrono_harness::json(&config).unwrap();
        if case == "unit-context" {
            cfg["execution_units"]["units"]["one"]["report_path"] =
                json!(".chrono-harness/state/one//event.json");
        } else {
            cfg["execution_units"]["report_path"] =
                json!(".chrono-harness/state/collection//manifest.json");
        }
        fs::write(root.join(CONFIG), serde_json::to_vec(&cfg).unwrap()).unwrap();
        let input = root.join(if case == "unit-context" {
            ".chrono-harness/state/one/event.json"
        } else {
            ".chrono-harness/state/collection/manifest.json"
        });
        fs::create_dir_all(input.parent().unwrap()).unwrap();
        fs::write(&input, b"original input bytes").unwrap();
        let before = marker(tools.path());
        let out = Command::new(root.join(".chrono-harness/bin/chrono-ci"))
            .current_dir(&root)
            .args(["generate", "--host-root", ".", "--config", PROVIDER])
            .output()
            .unwrap();
        assert!(!out.status.success(), "accepted {case}");
        assert_eq!(fs::read(input).unwrap(), b"original input bytes");
        assert_eq!(marker(tools.path()), before);
        for (path, bytes) in &workflows {
            assert_eq!(&fs::read(path).unwrap(), bytes);
        }
    }
}
fn local_short_lane(age_seconds: Option<f64>) -> (Host, tempfile::TempDir, std::path::PathBuf) {
    local_short_lane_with_governance(age_seconds, false)
}
fn local_short_lane_with_governance(
    age_seconds: Option<f64>,
    governance: bool,
) -> (Host, tempfile::TempDir, std::path::PathBuf) {
    let (h, tools) = configured_short_host_with_governance(false, age_seconds, governance);
    make_local_lane(h, tools, "integration")
}
fn make_local_lane(
    h: Host,
    tools: tempfile::TempDir,
    kind: &str,
) -> (Host, tempfile::TempDir, std::path::PathBuf) {
    let root = h.root();
    // Produce the fixed endpoint snapshots before starting the real branch
    // clock. Its short freshness window measures producer behavior, not fixture
    // snapshot setup competing with the other registered tests.
    git_facts::prepare_bound(&h, "integration", None);
    // short(..., None) explicitly removes this selector from the real caller.
    // Its retained snapshots must describe that local invocation even when the
    // test package itself was launched by the native CI check.
    let inputs_path = root.join(".chrono-harness/state/inputs.json");
    let mut inputs: Value = chrono_harness::json(&fs::read(&inputs_path).unwrap()).unwrap();
    for endpoint in ["base", "candidate"] {
        inputs[endpoint]["environment"]["CHRONO_CHECK_SOURCE"] = Value::Null;
    }
    fs::write(inputs_path, serde_json::to_vec(&inputs).unwrap()).unwrap();
    // Real birth producer in an isolated fixture; no hand-filled successful birth.
    git(&root, &["checkout", "-q", "dev"]);
    let lane = tools.path().join("local-lane");
    let out = Command::new(root.join(".chrono-harness/bin/chrono-worktree"))
        .current_dir(&root)
        .args([
            "start",
            "--host-root",
            root.to_str().unwrap(),
            "--config",
            WORKTREE,
            "--kind",
            kind,
            "--name",
            "short-full",
            "--path",
            lane.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // These declared fixture executables are immutable. Link their bytes into
    // the new lane instead of copying every binary after the branch clock starts.
    let mut programs = vec![
        h.values[CONFIG]["runner"]["path"].as_str().unwrap(),
        ".chrono-harness/bin/chrono-ci",
        ".chrono-harness/bin/chrono-worktree",
    ];
    programs.extend(
        h.values[JUDGES]["judges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| j["executable"].as_str().unwrap()),
    );
    for path in programs {
        let target = lane.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::hard_link(root.join(path), target).unwrap();
    }
    // The candidate is already a real descendant of the declared fork. Reuse
    // its fixed commit and snapshots instead of rebuilding them after birth.
    git(&lane, &["reset", "--hard", &h.candidate]);
    copy_tree(
        &root.join(".chrono-harness/state"),
        &lane.join(".chrono-harness/state"),
    );
    let origin_path = lane.join(".chrono-harness/state/origin.json");
    let mut origin: Value = chrono_harness::json(&fs::read(&origin_path).unwrap()).unwrap();
    origin["retained_inputs"] = json!(".chrono-harness/state/inputs.json");
    fs::write(&origin_path, serde_json::to_vec(&origin).unwrap()).unwrap();
    (h, tools, lane)
}
#[test]
fn real_short_full_local_units_collection_and_delivery() {
    let (_h, tools, lane) = local_short_lane(None);
    let origin_path = lane.join(".chrono-harness/state/origin.json");
    let mut origin: Value = chrono_harness::json(&fs::read(&origin_path).unwrap()).unwrap();
    let before = marker(tools.path());
    let (exit, one) = short(&lane, &["check", "--unit", "one"], None);
    passed(exit, &one);
    assert_eq!(one["judges"].as_array().unwrap().len(), 7);
    assert!(!marker(tools.path())[before.len()..].contains("p2/Cargo.toml"));
    let (exit, _) = short(&lane, &["check", "--collect"], None);
    assert_ne!(exit, 0);
    let (exit, two) = short(&lane, &["check", "--unit", "two"], None);
    passed(exit, &two);
    assert_eq!(one["context_digest"], two["context_digest"]);
    assert_eq!(
        one["preparation"]["result"]["context"]["raw"],
        two["preparation"]["result"]["context"]["raw"]
    );
    let first_observation =
        one["preparation"]["result"]["evidence"]["current_observation"]["unix_timestamp_nanos"]
            .as_str()
            .unwrap()
            .parse::<i128>()
            .unwrap();
    let second_observation =
        two["preparation"]["result"]["evidence"]["current_observation"]["unix_timestamp_nanos"]
            .as_str()
            .unwrap()
            .parse::<i128>()
            .unwrap();
    assert!(second_observation > first_observation);
    assert_eq!(
        two["preparation"]["result"]["evidence"]["current_observation"]["context_digest"],
        two["context_digest"]
    );
    let business = tools.path().join("cargo-fixture");
    let bytes = fs::read(&business).unwrap();
    fs::remove_file(&business).unwrap();
    let unchanged: Vec<_> = [
        origin_path.clone(),
        lane.join(origin["birth_report"].as_str().unwrap()),
        lane.join(".chrono-harness/state/one/check.json"),
        lane.join(".chrono-harness/state/two/check.json"),
    ]
    .into_iter()
    .map(|path| {
        let bytes = fs::read(&path).unwrap();
        (path, bytes)
    })
    .collect();
    let before = marker(tools.path());
    let (exit, collection) = short(&lane, &["check", "--collect"], None);
    passed(exit, &collection);
    for (path, bytes) in &unchanged {
        assert_eq!(fs::read(path).unwrap(), *bytes);
    }
    local_collection_observation(&lane, &collection);
    let receipt = &collection["preparation"]["receipts"][0];
    let receipt =
        chrono_harness::json(&fs::read(lane.join(receipt["path"].as_str().unwrap())).unwrap())
            .unwrap();
    assert_eq!(
        receipt["tool"]["path"],
        json!(
            fs::canonicalize(&lane)
                .unwrap()
                .join(".chrono-harness/bin/chrono-worktree")
        )
    );
    assert_eq!(
        receipt["tool"]["sha256"],
        sha256(&fs::read(lane.join(".chrono-harness/bin/chrono-worktree")).unwrap())
    );
    assert_eq!(
        workflow(&collection)["branch"]["current_observation"],
        collection["preparation"]["result"]["evidence"]["current_observation"]
    );
    let endpoint = &collection["preparation"]["request"]["prepared"];
    let original_worktree = chrono_harness::json(
        &fs::read(lane.join(endpoint["evidence"]["report_path"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    assert_eq!(original_worktree["status"], "prepared");
    assert_eq!(
        original_worktree["origin"]["birth_report"],
        origin["birth_report"]
    );
    for field in ["raw", "sha256", "semantic_digest"] {
        assert_eq!(
            endpoint["context"][field],
            one["preparation"]["result"]["context"][field]
        );
    }
    println!(
        "LOCAL_OBSERVATION genuine_worktree=true second_producer=true retained_report=true final_response=true context_unchanged=true"
    );
    assert_eq!(marker(tools.path()), before);
    assert_eq!(collection["tests"]["executed"], json!([]));
    assert_eq!(
        collection["tests"]["completion"]["required_units"],
        json!(["one", "two"])
    );
    assert!(collection["preparation"]["result"]["context"].is_object());
    let certificate = fs::read(lane.join(".chrono-harness/state/integration.json")).unwrap();
    fs::write(&business, &bytes).unwrap();
    fs::set_permissions(&business, fs::Permissions::from_mode(0o755)).unwrap();
    origin["run_kind"] = json!("delivery");
    origin["integration_evidence"] = json!(sha256(&certificate));
    fs::write(&origin_path, serde_json::to_vec(&origin).unwrap()).unwrap();
    let (exit, delivery) = short(&lane, &["check"], None);
    passed(exit, &delivery);
    assert_eq!(workflow(&delivery)["mode"], "delivery");
    assert_eq!(
        fs::read(lane.join(".chrono-harness/state/integration.json")).unwrap(),
        certificate
    );
}

// Deterministic times are fixture input data, not native observations. The
// registered producer fixture uses the same request/response transport as the
// existing CI short fixtures; the second producer and all judges are real.
fn fixture_endpoint(h: &Host, context: &Value, nanos: i128) {
    use chrono_harness::prepared::{self, Context, PreparedCheck};
    let root = h.root();
    let raw = serde_json::to_vec(context).unwrap();
    let original = prepared::retain_original(
        &root,
        ".chrono-harness/state/preparation/",
        "fixture-context",
        &raw,
    )
    .unwrap();
    let observation = json!({"context_digest":wire::digest(context).unwrap(),"unix_timestamp_nanos":nanos.to_string()});
    let evidence =
        json!({"schema":"fixture-local-observation/v1","current_observation":observation});
    let report = prepared::retain_original(
        &root,
        ".chrono-harness/state/preparation/",
        "fixture-endpoint",
        &serde_json::to_vec(&evidence).unwrap(),
    )
    .unwrap();
    let p = PreparedCheck {
        schema: prepared::RESPONSE.into(),
        request_sha256: String::new(),
        source: "local".into(),
        profile: CONFIG.into(),
        base: Some(h.base.clone()),
        candidate: h.candidate.clone(),
        initial: false,
        scope: None,
        context: Some(Context {
            path: original.path.clone(),
            sha256: original.sha256.clone(),
            semantic_digest: wire::digest(context).unwrap(),
            raw,
        }),
        evidence: json!({"report_path":report.path,"report_sha256":report.sha256,"current_observation":observation}),
        originals: vec![original, report],
    };
    fs::write(
        root.join(".chrono-harness/state/fixture-prepared.json"),
        serde_json::to_vec(&p).unwrap(),
    )
    .unwrap();
}

#[test]
fn local_collection_retry_preserves_observation_and_expires_older_producer() {
    let (mut h, tools) = short_host(false);
    let root = h.root();
    let fixture = root.join(".chrono-harness/bin/fixture-producer");
    // Reuse the registered producer-fixture capability, without a production
    // clock override or a direct injection into the workflow judge request.
    fs::write(&fixture, r#"#!/bin/sh
if [ "$1" = --version ]; then printf 'chrono-worktree 0.1.0\n'; exit 0; fi
exec /usr/bin/python3 -c '
import json,sys,hashlib
raw=sys.stdin.buffer.read(); q=json.loads(raw)
p=json.load(open(".chrono-harness/state/fixture-prepared.json"))
p["request_sha256"]=hashlib.sha256(raw).hexdigest()
s=q["selection"]
p["scope"] = None if s["kind"]=="all" else (dict(kind="unit",unit=s["unit"]) if s["kind"]=="unit" else dict(kind="collect",manifest=".chrono-harness/state/collection/manifest.json"))
print(json.dumps(p))'
"#).unwrap();
    fs::set_permissions(&fixture, fs::Permissions::from_mode(0o755)).unwrap();
    h.values.get_mut(CONFIG).unwrap()["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "chrono-worktree")
        .unwrap()["program"] = json!(fixture);
    h.save();
    h.base = h.candidate.clone();
    for prod in ["p", "p2"] {
        fs::write(
            root.join(format!("{prod}/src/lib.rs")),
            "pub fn double(n:i32)->i32 {2*n}\n",
        )
        .unwrap();
    }
    h.save();
    git_facts::prepare_bound(&h, "integration", None);
    let inputs_path = root.join(".chrono-harness/state/inputs.json");
    let mut inputs = chrono_harness::json(&fs::read(&inputs_path).unwrap()).unwrap();
    for endpoint in ["base", "candidate"] {
        inputs[endpoint]["environment"]["CHRONO_CHECK_SOURCE"] = Value::Null;
    }
    fs::write(inputs_path, serde_json::to_vec(&inputs).unwrap()).unwrap();
    let mut producer =
        chrono_harness::json(&fs::read(root.join(".chrono-harness/state/context.json")).unwrap())
            .unwrap();
    assert_eq!(producer["branch_started_at"], "2026-01-01T00:00:00Z");
    assert_eq!(h.values[WORKFLOW]["staleness"]["max_age_hours"], 24);
    producer["observed_at"] = json!("2026-01-01T23:00:00Z");
    let t23 = 1767308400000000000_i128;
    fixture_endpoint(&h, &producer, t23);
    for unit in ["one", "two"] {
        let (exit, report) = short(&root, &["check", "--unit", unit], None);
        passed(exit, &report);
    }
    let (exit, produced) = short(&root, &["check", "--collect"], None);
    passed(exit, &produced);
    let certificate_path = root.join(".chrono-harness/state/integration.json");
    let certificate_raw = fs::read(&certificate_path).unwrap();
    let mut consumer = producer.clone();
    consumer["run_kind"] = json!("delivery");
    consumer["branch_ref"] = json!("feature/independent-birth");
    consumer["branch_started_at"] = json!("2026-01-01T02:00:00Z");
    consumer["integration_evidence"] = json!(sha256(&certificate_raw));
    fixture_endpoint(&h, &consumer, t23);
    for unit in ["one", "two"] {
        let (exit, report) = short(&root, &["check", "--unit", unit], None);
        passed(exit, &report);
    }
    let mut originals = vec![(certificate_path, certificate_raw)];
    for path in [
        ".chrono-harness/state/context.json",
        ".chrono-harness/state/one/check.json",
        ".chrono-harness/state/two/check.json",
    ] {
        let path = root.join(path);
        originals.push((path.clone(), fs::read(path).unwrap()));
    }
    for report in [
        &produced,
        &chrono_harness::json(
            &fs::read(root.join(".chrono-harness/state/one/check.json")).unwrap(),
        )
        .unwrap(),
        &chrono_harness::json(
            &fs::read(root.join(".chrono-harness/state/two/check.json")).unwrap(),
        )
        .unwrap(),
    ] {
        for path in [
            report["report_path"].as_str().unwrap(),
            report["preparation"]["result"]["context"]["path"]
                .as_str()
                .unwrap(),
        ] {
            let path = root.join(path);
            originals.push((path.clone(), fs::read(path).unwrap()));
        }
    }
    for (nanos, pass, age) in [
        (1767315600000000000_i128, false, "90000000000000"), // producer25h, consumer23h
        (t23, true, ""),
        (1767312000000000000, true, ""), // producer24h equality
        (1767312000000000001, false, "86400000000001"),
    ] {
        fixture_endpoint(&h, &consumer, nanos);
        let before = marker(tools.path());
        let (exit, report) = short(&root, &["check", "--collect"], None);
        println!(
            "LOCAL_RETRY_ACTUAL fixture_nanos={nanos} exit={exit} observation={} findings={}",
            report["preparation"]["result"]["evidence"]["current_observation"], report["findings"]
        );
        assert_eq!(
            exit == 0,
            pass,
            "observation={nanos}: {}",
            report["findings"]
        );
        local_collection_observation(&root, &report);
        assert_eq!(
            report["preparation"]["result"]["evidence"]["current_observation"]["unix_timestamp_nanos"],
            nanos.to_string()
        );
        if pass {
            passed(exit, &report);
            assert_eq!(workflow(&report)["mode"], "delivery");
        } else {
            let findings = report["findings"].to_string();
            assert!(
                findings.contains("E_INTEGRATION_MISMATCH")
                    && findings.contains("E_BRANCH_STALE")
                    && findings.contains(age),
                "{findings}"
            );
        }
        assert_eq!(marker(tools.path()), before);
        assert_eq!(report["tests"]["executed"], json!([]));
        for (path, raw) in &originals {
            assert_eq!(fs::read(path).unwrap(), *raw);
        }
        println!(
            "LOCAL_RETRY fixture_nanos={nanos} pass={pass} original_bytes_unchanged=true collector_business=0 second_producer=true"
        );
    }
}
fn native_command(root: &Path, id: &str, candidate: &str, payload: &Path) -> (i32, Value) {
    let workflow = fs::read_to_string(root.join(format!(".github/workflows/{id}.yml"))).unwrap();
    let command = workflow
        .lines()
        .find(|line| {
            line.trim_start()
                .starts_with("'.chrono-harness/bin/chrono-harness' 'check'")
        })
        .expect("generated short command")
        .trim();
    let out = Command::new("/bin/sh")
        .current_dir(root)
        .args(["-c", command])
        .env("CHRONO_CHECK_SOURCE", "ci")
        .env("GITHUB_EVENT_NAME", "push")
        .env("GITHUB_EVENT_PATH", payload)
        .env("CHRONO_WORKFLOW_REVISION", candidate)
        .env("GITHUB_REPOSITORY", "fixture/full")
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .output()
        .unwrap();
    if !out.status.success() {
        println!("GENERATED_FAILURE {}", String::from_utf8_lossy(&out.stderr));
    }
    println!(
        "GENERATED {id} exit={} bytes={}",
        out.status.code().unwrap(),
        out.stdout.len()
    );
    let report = if id == "collection" {
        short_report(root, &["check", "--collect"], &out, Some("ci"))
    } else {
        short_report(root, &["check", "--unit", id], &out, Some("ci"))
    };
    (out.status.code().unwrap(), report)
}
#[test]
fn generated_automatic_full_units_gather_originals_on_separate_collection() {
    let (h, tools) = short_host(true);
    let root = h.root();
    let payload = tools.path().join("push.json");
    fs::write(&payload,serde_json::to_vec(&json!({"before":h.base,"after":h.candidate,"ref":"refs/heads/integration/fixture","created":false,"deleted":false})).unwrap()).unwrap();
    git_facts::prepare_bound(&h, "integration", None);
    let inputs_path = root.join(".chrono-harness/state/inputs.json");
    let mut inputs: Value = chrono_harness::json(&fs::read(&inputs_path).unwrap()).unwrap();
    for endpoint in ["base", "candidate"] {
        for (key, value) in [
            ("CHRONO_CHECK_SOURCE", json!("ci")),
            ("GITHUB_EVENT_NAME", json!("push")),
            ("GITHUB_EVENT_PATH", json!(payload)),
            ("CHRONO_WORKFLOW_REVISION", json!(h.candidate)),
            ("GITHUB_REPOSITORY", json!("fixture/full")),
        ] {
            inputs[endpoint]["environment"][key] = value;
        }
    }
    fs::write(&inputs_path, serde_json::to_vec(&inputs).unwrap()).unwrap();
    let context = fs::read(root.join(".chrono-harness/state/context.json")).unwrap();
    for id in ["one", "two", "collection"] {
        let path = root.join(format!(".chrono-harness/state/{id}/full.json"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, &context).unwrap();
    }
    let one = clone_host(&h);
    let two = clone_host(&h);
    let collector = clone_host(&h);
    let mut failed = Vec::new();
    for (id, unit_root) in [("one", one.path()), ("two", two.path())] {
        if id == "two" {
            fs::write(unit_root.join(".chrono-harness/state/fail"), "").unwrap();
            let (exit, report) = native_command(unit_root, id, &h.candidate, &payload);
            assert_ne!(exit, 0);
            failed =
                fs::read(unit_root.join(format!(".chrono-harness/state/{id}/check.json"))).unwrap();
            assert_ne!(report["status"], "pass");
            fs::remove_file(unit_root.join(".chrono-harness/state/fail")).unwrap();
        }
        let before = marker(tools.path());
        let (exit, report) = native_command(unit_root, id, &h.candidate, &payload);
        passed(exit, &report);
        assert_eq!(report["judges"].as_array().unwrap().len(), 7);
        assert_eq!(report["request"]["scope"], json!({"kind":"unit","unit":id}));
        let other = if id == "one" {
            "p2/Cargo.toml"
        } else {
            "p/Cargo.toml"
        };
        assert!(!marker(tools.path())[before.len()..].contains(other));
        copy_tree(
            &unit_root.join(format!(".chrono-harness/state/{id}")),
            &tools.path().join(format!("uploads/{id}")),
        );
        if id == "two" {
            let originals = fs::read_dir(unit_root.join(".chrono-harness/state/two"))
                .unwrap()
                .map(|e| fs::read(e.unwrap().path()).unwrap_or_default())
                .collect::<Vec<_>>();
            assert!(
                originals.contains(&failed),
                "failed original was not retained after retry"
            );
        }
    }
    let mock = tools.path().join("mock-gh");
    let data = tools.path().join("transport.json");
    fs::write(&data,serde_json::to_vec(&json!({"candidate":h.candidate,"uploads":tools.path().join("uploads"),"workflow_root":collector.path()})).unwrap()).unwrap();
    let python = chrono_harness::resolve_program(&root, "python3", None).unwrap();
    fs::write(&mock,format!(r#"#!{python}
import sys,json,pathlib,shutil
args=sys.argv[1:];data=json.loads(pathlib.Path({data:?}).read_text())
def run(unit):
 return dict(id=1 if unit=='one' else 2,run_attempt=1,status='completed',conclusion='success',head_sha=data['candidate'],event='push',path='.github/workflows/'+unit+'.yml')
if args[0]=='api':
 endpoint=args[1]
 if '/actions/workflows/' in endpoint:
  unit=endpoint.split('/actions/workflows/')[1].split('.yml')[0];print(json.dumps([dict(workflow_runs=[run(unit)])]))
 elif '/contents/' in endpoint:
  path=endpoint.split('/contents/')[1].split('?')[0];sys.stdout.buffer.write((pathlib.Path(data['workflow_root'])/path).read_bytes())
 else:
  unit='one' if endpoint.endswith('/1') else 'two';print(json.dumps(run(unit)))
else:
 unit='one' if args[2]=='1' else 'two';dest=pathlib.Path(args[args.index('--dir')+1]);shutil.copytree(pathlib.Path(data['uploads'])/unit,dest,dirs_exist_ok=True)
"#,python=python.display(),data=data.to_str().unwrap())).unwrap();
    fs::set_permissions(&mock, fs::Permissions::from_mode(0o755)).unwrap();
    one.close().unwrap();
    two.close().unwrap();
    fs::remove_file(tools.path().join("cargo-fixture")).unwrap();
    let before = marker(tools.path());
    let (exit, collected) = native_command(collector.path(), "collection", &h.candidate, &payload);
    passed(exit, &collected);
    assert_eq!(marker(tools.path()), before);
    assert_eq!(collected["tests"]["executed"], json!([]));
    assert_eq!(
        collected["tests"]["completion"]["required_units"],
        json!(["one", "two"])
    );
    let manifest: chrono_harness::units::FullManifest = chrono_harness::decode(
        &fs::read(
            collector
                .path()
                .join(".chrono-harness/state/collection/manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest.reports.len(), 2);
    assert!(manifest.reports.iter().all(|r| r.artifacts.is_some()));
    assert!(collected["request"]["observations"]["preparation"]["result"]["context"].is_object());
    // A changed original remains a failure, even with fresh gathering and valid current reports.
    let source = tools.path().join("uploads/one");
    let receipt = manifest.reports[0].artifacts.as_ref().unwrap();
    assert_eq!(receipt.source_directory, ".chrono-harness/state/one/");
    let original: Value =
        chrono_harness::json(&fs::read(source.join("check.json")).unwrap()).unwrap();
    let pointer = original["preparation"]["receipts"][0]["path"]
        .as_str()
        .unwrap()
        .strip_prefix(".chrono-harness/state/one/")
        .unwrap();
    fs::write(source.join(pointer), b"changed original acquisition").unwrap();
    let before = marker(tools.path());
    let (exit, report) = native_command(collector.path(), "collection", &h.candidate, &payload);
    assert_ne!(exit, 0, "{}", report["findings"]);
    assert_eq!(marker(tools.path()), before);
}

#[test]
fn local_short_round_expires_with_fresh_observation_and_preserves_originals() {
    let freshness_seconds = 30.0;
    let (h, tools, lane) = local_short_lane(Some(freshness_seconds));
    let (e, one) = short(&lane, &["check", "--unit", "one"], None);
    passed(e, &one);
    let old_context = one["request"]["context"]["path"].as_str().unwrap();
    let original = fs::read(old_context).unwrap();
    let birth = one["preparation"]["result"]["evidence"]["origin"]["birth_report"]
        .as_str()
        .unwrap();
    let birth_bytes = fs::read(lane.join(birth)).unwrap();
    let first_context = chrono_harness::json(&original).unwrap();
    let observed = first_context["observed_at"].clone();
    // Allow real setup work, then cross the registered deadline from the first
    // successful observation's measured age, with a margin for strict excess.
    let age_seconds = chrono_judge_workflow::observation_age(&first_context, &h.values[WORKFLOW])
        .unwrap()
        .as_seconds_f64();
    let wait = std::time::Duration::from_secs_f64(freshness_seconds - age_seconds + 1.0);
    println!(
        "EXPIRY initial_age_seconds={age_seconds} freshness_seconds={freshness_seconds} wait_seconds={}",
        wait.as_secs_f64()
    );
    std::thread::sleep(wait);
    let (e, two) = short(&lane, &["check", "--unit", "two"], None);
    assert_ne!(e, 0, "accepted an expired round");
    assert!(
        two["findings"].to_string().contains("E_BRANCH_STALE"),
        "{two:#}"
    );
    assert_ne!(two["context_digest"], one["context_digest"]);
    let refreshed = chrono_harness::json(
        &fs::read(lane.join(".chrono-harness/state/local/context.json")).unwrap(),
    )
    .unwrap();
    assert_ne!(refreshed["observed_at"], observed);
    let before = marker(tools.path());
    let (e, collection) = short(&lane, &["check", "--collect"], None);
    assert_ne!(e, 0, "old contributions completed the refreshed round");
    assert_eq!(marker(tools.path()), before);
    assert_eq!(fs::read(old_context).unwrap(), original);
    assert_eq!(fs::read(lane.join(birth)).unwrap(), birth_bytes);
    assert_ne!(collection["context_digest"], one["context_digest"]);
}

#[test]
fn malformed_cached_local_contexts_return_producer_errors() {
    let (_h, tools, lane) = local_short_lane(None);
    let (e, one) = short(&lane, &["check", "--unit", "one"], None);
    passed(e, &one);
    let path = lane.join(".chrono-harness/state/local/context.json");
    let before = marker(tools.path());
    for malformed in [
        json!([]),
        json!("cached string"),
        json!({"schema_version":1}),
    ] {
        fs::write(&path, serde_json::to_vec(&malformed).unwrap()).unwrap();
        let (e, r) = short(&lane, &["check", "--unit", "two"], None);
        assert_ne!(e, 0);
        let text = r.to_string();
        assert!(text.contains("E_LOCAL_CONTEXT"), "{text}");
        assert!(!text.contains("panicked"), "{text}");
        assert_eq!(marker(tools.path()), before);
    }
}

#[test]
fn ordinary_feature_short_check_accepts_null_certificate_but_stability_requires_it() {
    let (mut h, tools) = configured_short_host(false, None);
    git(
        &h.root(),
        &["checkout", &h.base, "--", "p/src/lib.rs", "p2/src/lib.rs"],
    );
    fs::write(h.root().join("doc.txt"), "ordinary documentation delta\n").unwrap();
    h.save();
    let (_h, tools, lane) = make_local_lane(h, tools, "feature");
    let (exit, report) = short(&lane, &["check"], None);
    passed(exit, &report);
    assert_eq!(workflow(&report)["mode"], "ordinary_delta");
    let raw: Vec<u8> =
        serde_json::from_value(report["preparation"]["result"]["context"]["raw"].clone()).unwrap();
    assert!(chrono_harness::json(&raw).unwrap()["integration_evidence"].is_null());
    assert!(!lane.join(".chrono-harness/state/integration.json").exists());
    let before = marker(tools.path());
    fs::write(
        lane.join("p/src/lib.rs"),
        "pub fn double(n:i32)->i32 {n+n}\n",
    )
    .unwrap();
    commit(&lane);
    let origin_path = lane.join(".chrono-harness/state/origin.json");
    let mut origin: Value = chrono_harness::json(&fs::read(&origin_path).unwrap()).unwrap();
    // Obtain genuine current captures while retaining the original historical base snapshot.
    let inputs_path = lane.join(".chrono-harness/state/inputs.json");
    let mut inputs: Value = chrono_harness::json(&fs::read(&inputs_path).unwrap()).unwrap();
    let captured = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
        .current_dir(&lane)
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .env_remove("CHRONO_CHECK_SOURCE")
        .args([
            "capture",
            "--host-root",
            ".",
            "--config",
            CONFIG,
            "--commit",
            &git(&lane, &["rev-parse", "HEAD"]),
            "--output",
            ".chrono-harness/state/feature-current.json",
        ])
        .output()
        .unwrap();
    assert!(
        captured.status.success(),
        "{}",
        String::from_utf8_lossy(&captured.stderr)
    );
    inputs["candidate"] = chrono_harness::json(&captured.stdout).unwrap();
    fs::write(&inputs_path, serde_json::to_vec(&inputs).unwrap()).unwrap();
    origin["integration_evidence"] = Value::Null;
    fs::write(origin_path, serde_json::to_vec(&origin).unwrap()).unwrap();
    let (exit, rejected) = short(&lane, &["check"], None);
    assert_ne!(exit, 0);
    assert!(
        rejected["findings"]
            .to_string()
            .contains("E_INTEGRATION_REQUIRED"),
        "{rejected}"
    );
    assert!(marker(tools.path()).len() >= before.len());
}

#[test]
fn genuine_lineage_early_seed_and_forwarding_bind_real_native_receipts() {
    let (mut h, tools, lane) = local_short_lane_with_governance(None, true);
    // Capture the original at its actual checkout and H0 location, before H1
    // changes that same location. No historical judge is executed.
    let base_original_path = ".chrono-harness/state/inputs/governance-base.json";
    git(&lane, &["checkout", "--detach", &h.base]);
    let old_capture = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
        .current_dir(&lane)
        .env("CHRONO_CHECK_SOURCE", "ci")
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .args([
            "capture",
            "--host-root",
            ".",
            "--config",
            CONFIG,
            "--commit",
            &h.base,
            "--output",
            base_original_path,
        ])
        .output()
        .unwrap();
    assert!(
        old_capture.status.success(),
        "exit={} stdout={} stderr={}",
        old_capture.status,
        String::from_utf8_lossy(&old_capture.stdout),
        String::from_utf8_lossy(&old_capture.stderr)
    );
    let base_original_raw = fs::read(lane.join(base_original_path)).unwrap();
    git(&lane, &["checkout", "integration/short-full"]);
    fs::write(
        lane.join(".chrono-harness/ci/governance-data"),
        b"genuine current governance H1",
    )
    .unwrap();
    h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["id"] == "governance-data")
        .unwrap()["sha256"] = json!(sha256(b"genuine current governance H1"));
    let origin: Value =
        chrono_harness::json(&fs::read(lane.join(".chrono-harness/state/origin.json")).unwrap())
            .unwrap();
    let lineage_bytes = fs::read(lane.join(origin["birth_report"].as_str().unwrap())).unwrap();
    let lineage_path = ".chrono-harness/ci/lineage.json";
    fs::write(lane.join(lineage_path), &lineage_bytes).unwrap();
    let system_python =
        chrono_harness::resolve_program(&lane, "python3", Some("/usr/bin:/bin")).unwrap();
    // Exercise the Linux layout even when the host's interpreter is a regular file.
    let python_directory = fs::canonicalize(tools.path())
        .unwrap()
        .join("native-python");
    fs::create_dir(&python_directory).unwrap();
    let python_link = python_directory.join("python3");
    std::os::unix::fs::symlink(&system_python, &python_link).unwrap();
    assert!(fs::symlink_metadata(&python_link).unwrap().is_symlink());
    let python =
        chrono_harness::resolve_program(&lane, "python3", Some(python_directory.to_str().unwrap()))
            .unwrap();
    let link_version = Command::new(&python).arg("--version").output().unwrap();
    let link_digest = sha256(&fs::read(&python).unwrap());
    // Ordinary file inputs require the actual regular executable, not its alias.
    let python = fs::canonicalize(python).unwrap();
    let version = Command::new(&python).arg("--version").output().unwrap();
    assert_eq!(version.status, link_version.status);
    assert_eq!(version.stdout, link_version.stdout);
    assert_eq!(version.stderr, link_version.stderr);
    assert_eq!(sha256(&fs::read(&python).unwrap()), link_digest);
    let cfg = h.values.get_mut(CONFIG).unwrap();
    cfg["canonical_check"]["inputs"]["ci"] = json!({"operation":"prepare.ci","tool":"native-python","argv":[".chrono-harness/ci/native.py","forward","--config",PROVIDER]});
    cfg["tools"].as_array_mut().unwrap().push(json!({"id":"native-python","program":python,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}));
    cfg["environment"]["inputs"].as_array_mut().unwrap().push(json!({"id":"native-python","location":python,"presence":"present","sha256":sha256(&fs::read(&python).unwrap())}));
    cfg["input_closure"]["bindings"][0]["inputs"]
        .as_array_mut()
        .unwrap()
        .extend([json!("tool:native-python"), json!("input:native-python")]);
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .extend([
            edge("tool:native-python", "runtime-input", "judge:registration"),
            edge("input:native-python", "runtime-input", "judge:registration"),
        ]);
    for path in [lineage_path, ".chrono-harness/ci/native.py"] {
        let mut f = file(path, json!([]));
        f["surface"] = json!("documentation");
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(f);
    }
    write_values(&lane, &h.values);
    let mut provider: Value =
        chrono_harness::json(&fs::read(lane.join(PROVIDER)).unwrap()).unwrap();
    let seed_directory = ".chrono-harness/state/shared-seed/";
    provider["job_gating"] = json!({"schema":"chrono-job-gating/v1","detector":{"runs_on":"fixture-native","timeout_minutes":20,"bootstrap":["/bin/true"],"sparse_checkout":[".chrono-harness/",".github/workflows/collection.yml"]}});
    h.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| {
            !matches!(
                f["path"].as_str(),
                Some(".github/workflows/one.yml" | ".github/workflows/two.yml")
            )
        });
    write_values(&lane, &h.values);
    provider["native_adoption"] = json!({"schema":"chrono-native-adoption/v1","lineage":{"path":lineage_path,"sha256":sha256(&lineage_bytes)},"adapter_path":".chrono-harness/ci/native.py","interpreter":python,"inputs_program":source().join("crates/inputs/target/debug/chrono-inputs"),"seed_directory":seed_directory,"seed_artifact":"chrono-context","retained_inputs":".chrono-harness/state/inputs.json","composition_sources":[],"push_roles":{"refs/heads/integration/":"integration","refs/heads/dev":"delivery"},"pull_request_role":"delivery","integration_evidence":null,"integration_evidence_path":".chrono-harness/state/integration.json","integration_transport":{"source_directory":".chrono-harness/state/collection/","directory":".chrono-harness/state/cert-download/"}});
    provider["native_adoption"]["composition_sources"] =
        json!([{"path":base_original_path,"sha256":sha256(&base_original_raw)}]);
    let mock = tools.path().join("mock-gh");
    let metadata = tools.path().join("seed-api.json");
    let uploads = tools.path().join("shared-upload");
    fs::write(&mock,format!(r#"#!/usr/bin/python3
import json,pathlib,sys,shutil
args=sys.argv[1:]; data=json.loads(pathlib.Path({metadata:?}).read_text()); run=dict(id=data.get('seed_run',900),run_attempt=data.get('attempt',1),status='in_progress',head_sha=data['candidate'],event=data.get('event','push'),path='.github/workflows/collection.yml',repository=dict(full_name='fixture/native'))
case=data.get('case','valid')
if case=='run-race' and args[:1]==['api'] and args[1].endswith('/runs/900'):run['run_attempt']=2
if args[:2]==['run','download']:
 name=args[args.index('--name')+1]
 unit=name[len('chrono-unit-'):].rsplit('-',2)[0] if name.startswith('chrono-unit-') else None
 dest=pathlib.Path(args[args.index('--dir')+1]);shutil.copytree(pathlib.Path(data['units'][unit]['upload'] if unit else data['upload']),dest,dirs_exist_ok=True)
 if case in ['wrong-base','wrong-attempt','wrong-workflow','wrong-digest','wrong-fork','stale','nested-loss','wrong-lineage','wrong-branch','wrong-repository','wrong-candidate']:
  binding=json.loads((dest/'binding.json').read_bytes()); context=json.loads((dest/'context.json').read_bytes())
  if case=='wrong-base':binding['base']='f'*40
  if case=='wrong-attempt':binding['attempt']=2
  if case=='wrong-workflow':binding['workflow_revision']='e'*40
  if case=='wrong-digest':binding['context_sha256']='0'*64
  if case=='wrong-repository':binding['repository']='other/repository'
  if case=='wrong-candidate':binding['candidate']='e'*40
  if case=='wrong-lineage':(dest/'lineage.json').write_bytes(b'original byte drift')
  if case=='wrong-branch':context['branch_ref']='feature/wrong'
  if case=='wrong-fork':context['fork_point']='d'*40
  if case=='stale':context['observed_at']='2020-01-01T00:00:00Z'
  if case=='nested-loss':
   first=next(iter(binding['originals'].values()));(dest/first['file']).unlink()
  if case in ['wrong-fork','stale','wrong-branch']:
   import hashlib
   raw=json.dumps(context,separators=(',',':')).encode();(dest/'context.json').write_bytes(raw);binding['context_sha256']=hashlib.sha256(raw).hexdigest()
  (dest/'binding.json').write_text(json.dumps(binding))
elif '/contents/' in args[1]:
 filename=args[1].split('/contents/')[1].split('?')[0]
 sys.stdout.buffer.write((pathlib.Path(data['workflow']).parents[2]/filename).read_bytes())
elif '/artifacts?' in args[1]:print(json.dumps([dict(artifacts=[] if case=='missing' else [dict(name='chrono-context-'+str(run['id'])+'-1',expired=False)]),dict(artifacts=[dict(name='chrono-context-'+str(run['id'])+'-1',expired=False)] if case=='ambiguous' else [])]))
elif '/jobs?' in args[1]:
 rows=[dict(id=200,run_id=run['id'],run_attempt=run['run_attempt'],name='Detect registered DELTA',status='completed',conclusion='success',head_sha=run['head_sha'])]
 provider=json.loads((pathlib.Path(data['workflow']).parents[2]/'.chrono-harness/ci/units.json').read_bytes())
 for i,unit in enumerate(sorted(data.get('units',{{}}))):rows.append(dict(id=201+i,run_id=run['id'],run_attempt=run['run_attempt'] if data.get('carried') else data['units'][unit].get('attempt',1),name=provider['units'][unit]['name'],status='completed',conclusion='success',head_sha=run['head_sha']))
 print(json.dumps([dict(jobs=rows[:2]),dict(jobs=rows[2:])]))
else:print(json.dumps(run))
"#,metadata=metadata.to_str().unwrap())).unwrap();
    fs::set_permissions(&mock, fs::Permissions::from_mode(0o755)).unwrap();
    provider["gather"]["program"] = json!(mock);
    fs::write(lane.join(PROVIDER), serde_json::to_vec(&provider).unwrap()).unwrap();
    let generated = Command::new(lane.join(".chrono-harness/bin/chrono-ci"))
        .current_dir(&lane)
        .args(["generate", "--host-root", ".", "--config", PROVIDER])
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    h.candidate = commit(&lane);
    let payload = tools.path().join("authentic-event.json");
    fs::write(&payload,serde_json::to_vec(&json!({"before":h.base,"after":h.candidate,"ref":"refs/heads/integration/short-full","created":false,"deleted":false})).unwrap()).unwrap();
    let detection_path = tools.path().join("actual-detection.json");
    let detect = |root: &Path, run: &str| {
        let event = if chrono_harness::json(&fs::read(&payload).unwrap())
            .unwrap()
            .get("pull_request")
            .is_some()
        {
            "pull_request"
        } else {
            "push"
        };
        let out = Command::new(root.join(".chrono-harness/bin/chrono-ci"))
            .current_dir(root)
            .env("GITHUB_EVENT_NAME", event)
            .env("GITHUB_EVENT_PATH", &payload)
            .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
            .env("GITHUB_REPOSITORY", "fixture/native")
            .env("GITHUB_RUN_ID", run)
            .env("GITHUB_RUN_ATTEMPT", "1")
            .env("GITHUB_JOB", "detect")
            .args([
                "detect",
                "--host-root",
                ".",
                "--config",
                PROVIDER,
                "--github-output",
                tools.path().join("detect-output").to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let detected = chrono_harness::json(&out.stdout).unwrap()["detection"].clone();
        fs::write(&detection_path, serde_json::to_vec(&detected).unwrap()).unwrap();
        detected
    };
    let detected = detect(&lane, "900");
    let native = |root: &Path, operation: &str, unit: Option<&str>, run: &str| {
        let mut c = Command::new(&python);
        c.current_dir(root)
            .env("DECLARED_EMPTY", "")
            .env_remove("DECLARED_ABSENT")
            .env(
                "GITHUB_EVENT_NAME",
                if chrono_harness::json(&fs::read(&payload).unwrap())
                    .unwrap()
                    .get("pull_request")
                    .is_some()
                {
                    "pull_request"
                } else {
                    "push"
                },
            )
            .env("GITHUB_EVENT_PATH", &payload)
            .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
            .env("GITHUB_REPOSITORY", "fixture/native")
            .env("GITHUB_RUN_ID", run)
            .env(
                "GITHUB_JOB",
                if operation == "publish" {
                    "detect".to_owned()
                } else {
                    unit.map(fixture_job_id).unwrap_or("aggregate".into())
                },
            )
            .env(
                "CHRONO_CI_DETECTION",
                fs::read_to_string(&detection_path).unwrap(),
            )
            .env(
                "GITHUB_RUN_ATTEMPT",
                if operation == "publish" {
                    "1".to_owned()
                } else {
                    fs::read(&metadata)
                        .ok()
                        .and_then(|r| chrono_harness::json(&r).ok())
                        .and_then(|v| v["attempt"].as_u64())
                        .unwrap_or(1)
                        .to_string()
                },
            )
            .args([
                ".chrono-harness/ci/native.py",
                operation,
                "--config",
                PROVIDER,
            ]);
        if unit.is_none() && operation == "acquire" {
            let data = chrono_harness::json(&fs::read(&metadata).unwrap()).unwrap();
            if let Some(needs) = data.get("needs") {
                c.env("CHRONO_CI_NEEDS", serde_json::to_string(needs).unwrap());
            }
        }
        if let Some(unit) = unit {
            c.args(["--unit", unit]);
        }
        let out = c.output().unwrap();
        if operation == "publish" && !out.status.success() {
            let retained = tempfile::Builder::new()
                .prefix("chrono-native-publication-failure-")
                .tempdir()
                .unwrap()
                .keep();
            copy_tree(&root.join(".chrono-harness/state"), &retained.join("state"));
            fs::write(retained.join("stdout"), &out.stdout).unwrap();
            fs::write(retained.join("stderr"), &out.stderr).unwrap();
            fs::write(
                retained.join("process.json"),
                serde_json::to_vec(&json!({
                    "exit_code":out.status.code(),"status":out.status.to_string()
                }))
                .unwrap(),
            )
            .unwrap();
            eprintln!(
                "original native publication process evidence: {}",
                retained.display()
            );
        }
        out
    };
    let published = native(&lane, "publish", None, "900");
    assert!(
        published.status.success(),
        "{}",
        String::from_utf8_lossy(&published.stderr)
    );
    let seed = fs::read(lane.join(format!("{seed_directory}context.json"))).unwrap();
    let governance_pair =
        chrono_harness::json(&fs::read(lane.join(format!("{seed_directory}inputs.json"))).unwrap())
            .unwrap();
    assert_eq!(
        governance_pair["base"]["files"]["governance-data"]["sha256"],
        sha256(b"genuine base governance H0")
    );
    assert_eq!(
        governance_pair["candidate"]["files"]["governance-data"]["sha256"],
        sha256(b"genuine current governance H1")
    );
    assert_eq!(
        fs::read(lane.join(base_original_path)).unwrap(),
        base_original_raw
    );
    let ctx = chrono_harness::json(&seed).unwrap();
    let birth = chrono_harness::json(&lineage_bytes).unwrap();
    assert_eq!(ctx["branch_started_at"], birth["branch_started_at"]);
    assert_eq!(ctx["fork_point"], birth["base"]);
    assert_eq!(
        fs::read(lane.join(format!("{seed_directory}lineage.json"))).unwrap(),
        lineage_bytes
    );
    let generated_workflow =
        fs::read_to_string(lane.join(".github/workflows/collection.yml")).unwrap();
    assert!(
        generated_workflow
            .find("Publish shared native context")
            .unwrap()
            < generated_workflow.find("Canonical harness check").unwrap()
    );
    assert!(
        !generated_workflow.contains("Gather independent unit reports"),
        "gather remains inside short check"
    );
    assert!(generated_workflow.contains("actions: read"));
    copy_tree(&lane.join(seed_directory), &uploads);
    fs::write(&metadata,serde_json::to_vec(&json!({"candidate":h.candidate,"upload":uploads,"workflow":lane.join(".github/workflows/collection.yml")})).unwrap()).unwrap();
    let unit_root = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["clone", "--quiet", "--no-hardlinks"])
            .arg(&lane)
            .arg(unit_root.path())
            .output()
            .unwrap()
            .status
            .success()
    );
    copy_tree(
        &lane.join(".chrono-harness/bin"),
        &unit_root.path().join(".chrono-harness/bin"),
    );
    let before = marker(tools.path());
    for case in [
        "missing",
        "ambiguous",
        "wrong-base",
        "wrong-attempt",
        "wrong-workflow",
        "wrong-digest",
        "wrong-fork",
        "stale",
        "nested-loss",
        "wrong-lineage",
        "wrong-branch",
        "wrong-repository",
        "wrong-candidate",
        "run-race",
    ] {
        let value = json!({"candidate":h.candidate,"upload":uploads,"workflow":lane.join(".github/workflows/collection.yml"),"case":case});
        fs::write(&metadata, serde_json::to_vec(&value).unwrap()).unwrap();
        let path = unit_root.path().join(".chrono-harness/state/one/seed");
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
        }
        let rejected = native(unit_root.path(), "acquire", Some("one"), "900");
        assert!(!rejected.status.success(), "accepted acquisition {case}");
        assert_eq!(
            marker(tools.path()),
            before,
            "acquisition launches no business/SDK processes"
        );
    }
    fs::write(&metadata,serde_json::to_vec(&json!({"candidate":h.candidate,"upload":uploads,"workflow":lane.join(".github/workflows/collection.yml")})).unwrap()).unwrap();
    if unit_root
        .path()
        .join(".chrono-harness/state/one/seed")
        .exists()
    {
        fs::remove_dir_all(unit_root.path().join(".chrono-harness/state/one/seed")).unwrap();
    }
    let acquired = native(unit_root.path(), "acquire", Some("one"), "900");
    assert!(
        acquired.status.success(),
        "{}",
        String::from_utf8_lossy(&acquired.stderr)
    );
    assert_eq!(
        fs::read(unit_root.path().join(".chrono-harness/state/one/full.json")).unwrap(),
        seed
    );
    // Capture the current endpoint in the real business environment with acquisition fields absent.
    let capture = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
        .current_dir(unit_root.path())
        .env("CHRONO_CHECK_SOURCE", "ci")
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
            "--unit",
            "one",
            "--output",
            ".chrono-harness/state/candidate-real.json",
        ])
        .output()
        .unwrap();
    assert!(
        capture.status.success(),
        "{}",
        String::from_utf8_lossy(&capture.stderr)
    );
    // The historical original was captured at its explicit checked-out endpoint.
    git(&lane, &["checkout", "--detach", &h.base]);
    let base = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
        .current_dir(&lane)
        .env("CHRONO_CHECK_SOURCE", "ci")
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .args([
            "capture",
            "--host-root",
            ".",
            "--config",
            CONFIG,
            "--commit",
            &h.base,
            "--unit",
            "one",
            "--output",
            ".chrono-harness/state/base-real.json",
        ])
        .output()
        .unwrap();
    assert!(
        base.status.success(),
        "{}",
        String::from_utf8_lossy(&base.stderr)
    );
    git(&lane, &["checkout", "--detach", &h.candidate]);
    let paired = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
        .current_dir(unit_root.path())
        .args([
            "pair",
            "--host-root",
            ".",
            "--base-root",
            lane.to_str().unwrap(),
            "--base-snapshot",
            ".chrono-harness/state/base-real.json",
            "--candidate-snapshot",
            ".chrono-harness/state/candidate-real.json",
            "--output",
            ".chrono-harness/state/inputs.json",
        ])
        .output()
        .unwrap();
    assert!(
        paired.status.success(),
        "{}",
        String::from_utf8_lossy(&paired.stderr)
    );
    let (exit, report) = short(unit_root.path(), &["check", "--unit", "one"], Some("ci"));
    passed(exit, &report);
    let request: chrono_harness::prepared::InputRequest =
        serde_json::from_value(report["preparation"]["request"].clone()).unwrap();
    assert_eq!(request.native_artifacts.unwrap().config_path, PROVIDER);
    assert_eq!(
        h.values[CONFIG]["canonical_check"]["inputs"]["ci"]["argv"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| *v == "--config")
            .count(),
        1
    );
    let original = report["preparation"]["result"]["evidence"]["report_path"]
        .as_str()
        .unwrap();
    let receipt =
        chrono_harness::json(&fs::read(unit_root.path().join(original)).unwrap()).unwrap();
    let process = receipt["forwarding"]["process"]["path"].as_str().unwrap();
    let process = chrono_harness::json(&fs::read(unit_root.path().join(process)).unwrap()).unwrap();
    let outer_ref = &report["preparation"]["receipts"][0];
    let outer = chrono_harness::json(
        &fs::read(unit_root.path().join(outer_ref["path"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    assert!(
        outer["process"]["environment"]
            .get("GITHUB_EVENT_NAME")
            .is_none()
    );
    assert_eq!(process["environment"]["GITHUB_EVENT_NAME"], sha256(b"push"));
    let original_input = fs::read(
        unit_root
            .path()
            .join(process["stdin"]["path"].as_str().unwrap()),
    )
    .unwrap();
    assert_eq!(sha256(&original_input), outer["process"]["stdin_sha256"]);
    let stdout = fs::read(
        unit_root
            .path()
            .join(process["stdout"]["path"].as_str().unwrap()),
    )
    .unwrap();
    let child: chrono_harness::prepared::PreparedCheck = chrono_harness::decode(&stdout).unwrap();
    chrono_harness::prepared::validate_result(
        &serde_json::from_slice(&original_input).unwrap(),
        &child,
    )
    .unwrap();
    // A second independent checkout contributes its own genuine endpoint pair and unit run.
    let second = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["clone", "--quiet", "--no-hardlinks"])
            .arg(&lane)
            .arg(second.path())
            .output()
            .unwrap()
            .status
            .success()
    );
    copy_tree(
        &lane.join(".chrono-harness/bin"),
        &second.path().join(".chrono-harness/bin"),
    );
    let acquired = native(second.path(), "acquire", Some("two"), "900");
    assert!(
        acquired.status.success(),
        "{}",
        String::from_utf8_lossy(&acquired.stderr)
    );
    assert_eq!(
        fs::read(second.path().join(".chrono-harness/state/two/full.json")).unwrap(),
        seed
    );
    let capture_two = |root: &Path, endpoint: &str, output: &str| {
        let p = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
            .current_dir(root)
            .env("CHRONO_CHECK_SOURCE", "ci")
            .env("DECLARED_EMPTY", "")
            .env_remove("DECLARED_ABSENT")
            .args([
                "capture",
                "--host-root",
                ".",
                "--config",
                CONFIG,
                "--commit",
                endpoint,
                "--unit",
                "two",
                "--output",
                output,
            ])
            .output()
            .unwrap();
        assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
    };
    capture_two(
        second.path(),
        &h.candidate,
        ".chrono-harness/state/candidate-two.json",
    );
    git(&lane, &["checkout", "--detach", &h.base]);
    capture_two(&lane, &h.base, ".chrono-harness/state/base-two.json");
    git(&lane, &["checkout", "--detach", &h.candidate]);
    let p = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
        .current_dir(second.path())
        .args([
            "pair",
            "--host-root",
            ".",
            "--base-root",
            lane.to_str().unwrap(),
            "--base-snapshot",
            ".chrono-harness/state/base-two.json",
            "--candidate-snapshot",
            ".chrono-harness/state/candidate-two.json",
            "--output",
            ".chrono-harness/state/inputs.json",
        ])
        .output()
        .unwrap();
    assert!(p.status.success(), "{}", String::from_utf8_lossy(&p.stderr));
    let (exit, two) = short(second.path(), &["check", "--unit", "two"], Some("ci"));
    passed(exit, &two);
    let production = |root: &Path, unit: &str| {
        let out = Command::new(root.join(".chrono-harness/bin/chrono-ci"))
            .current_dir(root)
            .env("GITHUB_REPOSITORY", "fixture/native")
            .env("GITHUB_RUN_ID", "900")
            .env("GITHUB_RUN_ATTEMPT", "1")
            .env("GITHUB_JOB", fixture_job_id(unit))
            .args([
                "production",
                "--host-root",
                ".",
                "--config",
                PROVIDER,
                "--unit",
                unit,
                "--github-output",
                tools.path().join("unit-output").to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    let mut outputs = json!({"detection":serde_json::to_string(&detected).unwrap()});
    let mut needs = json!({"detect":{"result":"success"}});
    for (id, root) in [("one", unit_root.path()), ("two", second.path())] {
        outputs[fixture_job_id(id)] = json!("true");
        needs[fixture_job_id(id)] =
            json!({"result":"success","outputs":{"production":production(root,id)}});
    }
    needs["detect"]["outputs"] = outputs;
    fs::write(
        &metadata,
        serde_json::to_vec(&json!({"candidate":h.candidate,"upload":uploads,
        "workflow":lane.join(".github/workflows/collection.yml"),"needs":needs,"units":{
        "one":{"upload":unit_root.path().join(".chrono-harness/state/one")},
        "two":{"upload":second.path().join(".chrono-harness/state/two")}}}))
        .unwrap(),
    )
    .unwrap();
    let acquired = native(&lane, "acquire", None, "900");
    assert!(
        acquired.status.success(),
        "{}",
        String::from_utf8_lossy(&acquired.stderr)
    );
    // Collection delegates real gather and composition through the adapter, with the SDK gone.
    let before_collection = marker(tools.path());
    let sdk_original = fs::read(tools.path().join("cargo-fixture")).unwrap();
    fs::remove_file(tools.path().join("cargo-fixture")).unwrap();
    fs::remove_file(lane.join(base_original_path)).unwrap();
    let (exit, collected) = short(&lane, &["check", "--collect"], Some("ci"));
    passed(exit, &collected);
    assert_eq!(collected["tests"]["executed"], json!([]));
    assert_eq!(marker(tools.path()), before_collection);
    assert_eq!(workflow(&collected)["mode"], "integration_run");
    assert!(!lane.join(base_original_path).exists());
    let result_report = collected["preparation"]["result"]["evidence"]["report_path"]
        .as_str()
        .unwrap();
    let result = chrono_harness::json(&fs::read(lane.join(result_report)).unwrap()).unwrap();
    assert!(
        result["originals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["path"].as_str().unwrap().contains("composition-process"))
    );
    // A fresh aggregate-only rerun retains original detector/unit production and
    // applies a genuinely later current observation to the same context bytes.
    let repeated_root = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["clone", "--quiet", "--no-hardlinks"])
            .arg(&lane)
            .arg(repeated_root.path())
            .output()
            .unwrap()
            .status
            .success()
    );
    copy_tree(
        &lane.join(".chrono-harness/bin"),
        &repeated_root.path().join(".chrono-harness/bin"),
    );
    let mut rerun: Value = chrono_harness::json(&fs::read(&metadata).unwrap()).unwrap();
    rerun["attempt"] = json!(2);
    rerun["carried"] = json!(true);
    fs::write(&metadata, serde_json::to_vec(&rerun).unwrap()).unwrap();
    let acquired = native(repeated_root.path(), "acquire", None, "900");
    assert!(
        acquired.status.success(),
        "{}",
        String::from_utf8_lossy(&acquired.stderr)
    );
    let (exit, repeated) = short(repeated_root.path(), &["check", "--collect"], Some("ci"));
    passed(exit, &repeated);
    assert_eq!(repeated["context_digest"], collected["context_digest"]);
    assert_eq!(repeated["tests"]["executed"], json!([]));
    assert_eq!(marker(tools.path()), before_collection);
    assert_eq!(
        fs::read(
            repeated_root
                .path()
                .join(".chrono-harness/state/collection/full.json")
        )
        .unwrap(),
        seed
    );
    let old_observation = &collected["preparation"]["result"]["evidence"]["current_observation"]["unix_timestamp_nanos"];
    let now_observation = &repeated["preparation"]["result"]["evidence"]["current_observation"]["unix_timestamp_nanos"];
    assert!(
        now_observation.as_str().unwrap().parse::<u128>().unwrap()
            > old_observation.as_str().unwrap().parse::<u128>().unwrap()
    );
    // A PR seed binds the actual original certificate bytes and consumes its relocated closure.
    // Publication's bootstrap supplies the same captured original bytes again;
    // collection above succeeded after the detector's source path was removed.
    fs::write(lane.join(base_original_path), &base_original_raw).unwrap();
    let delivery_root = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["clone", "--quiet", "--no-hardlinks"])
            .arg(&lane)
            .arg(delivery_root.path())
            .output()
            .unwrap()
            .status
            .success()
    );
    copy_tree(
        &lane.join(".chrono-harness/bin"),
        &delivery_root.path().join(".chrono-harness/bin"),
    );
    copy_tree(
        &lane.join(".chrono-harness/state/collection"),
        &delivery_root
            .path()
            .join(".chrono-harness/state/cert-download"),
    );
    copy_tree(
        &lane.join(".chrono-harness/state/inputs"),
        &delivery_root.path().join(".chrono-harness/state/inputs"),
    );
    for path in [
        ".chrono-harness/state/inputs.json",
        ".chrono-harness/state/integration.json",
    ] {
        fs::copy(lane.join(path), delivery_root.path().join(path)).unwrap();
    }
    fs::remove_dir_all(lane.join(".chrono-harness/state/collection")).unwrap();
    fs::write(tools.path().join("cargo-fixture"), &sdk_original).unwrap();
    fs::set_permissions(
        tools.path().join("cargo-fixture"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::write(&payload, serde_json::to_vec(&json!({"pull_request":{"base":{"sha":h.base},"head":{"sha":h.candidate,"ref":"integration/short-full"}}})).unwrap()).unwrap();
    detect(delivery_root.path(), "930");
    let published = native(delivery_root.path(), "publish", None, "930");
    assert!(
        published.status.success(),
        "{}",
        String::from_utf8_lossy(&published.stderr)
    );
    let delivered_seed = chrono_harness::json(
        &fs::read(
            delivery_root
                .path()
                .join(format!("{seed_directory}context.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        delivered_seed["integration_evidence"],
        sha256(
            &fs::read(
                delivery_root
                    .path()
                    .join(".chrono-harness/state/integration.json")
            )
            .unwrap()
        )
    );
    let delivery_upload = tools.path().join("delivery-seed-upload");
    copy_tree(&delivery_root.path().join(seed_directory), &delivery_upload);
    fs::write(&metadata, serde_json::to_vec(&json!({"candidate":h.candidate,"seed_run":930,"event":"pull_request",
        "upload":delivery_upload,"workflow":delivery_root.path().join(".github/workflows/collection.yml")})).unwrap()).unwrap();
    fs::remove_file(
        delivery_root
            .path()
            .join(".chrono-harness/state/integration.json"),
    )
    .unwrap();
    let acquired = native(delivery_root.path(), "acquire", Some("one"), "930");
    assert!(
        acquired.status.success(),
        "{}",
        String::from_utf8_lossy(&acquired.stderr)
    );
    assert_eq!(
        delivered_seed["integration_evidence"],
        sha256(
            &fs::read(
                delivery_root
                    .path()
                    .join(".chrono-harness/state/integration.json")
            )
            .unwrap()
        )
    );
    let acquired = native(delivery_root.path(), "acquire", None, "930");
    assert!(
        acquired.status.success(),
        "{}",
        String::from_utf8_lossy(&acquired.stderr)
    );
    let (exit, delivered) = short(delivery_root.path(), &["check"], Some("ci"));
    passed(exit, &delivered);
    assert_eq!(workflow(&delivered)["mode"], "delivery");
    println!(
        "NATIVE_PROVISION genuine_birth=true early_publish=true shared_raw=true acquisition_negatives=14 real_forwarded_prepared_consumer=pass provider_config_pairs=1 real_collection_composition=pass collector_effects=0 original_certificate_handoff=pass relocated_delivery=pass"
    );
}

fn fixture_job_id(unit: &str) -> String {
    format!("unit_{unit}")
}

// These regressions exercise the value-less local second-producer handoff with
// real, distinct runner/CI binaries. Optional task-local capture preserves exact
// original processes and report bytes; ordinary dedicated-suite runs need no flag.
fn repair_short(root: &Path, name: &str, args: &[&str]) -> (i32, Value) {
    let root = fs::canonicalize(root).unwrap();
    let output = Command::new(root.join(".chrono-harness/bin/chrono-harness"))
        .current_dir(&root)
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .env_remove("CHRONO_CHECK_SOURCE")
        .args(args)
        .output()
        .unwrap();
    let runner = sha256(&fs::read(root.join(".chrono-harness/bin/chrono-harness")).unwrap());
    let ci = sha256(&fs::read(root.join(".chrono-harness/bin/chrono-ci")).unwrap());
    assert_ne!(runner, ci, "fixture must have distinct actual producers");
    if let Some(directory) = std::env::var_os("CHRONO_COLLECTION_REPAIR_EVIDENCE") {
        let directory = std::path::PathBuf::from(directory).join(name);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("stdout"), &output.stdout).unwrap();
        fs::write(directory.join("stderr"), &output.stderr).unwrap();
        fs::write(directory.join("process.json"), serde_json::to_vec_pretty(&json!({
            "argv":std::iter::once(".chrono-harness/bin/chrono-harness").chain(args.iter().copied()).collect::<Vec<_>>(),
            "cwd":root,"source":"local","exit_code":output.status.code(),
            "runner_sha256":runner,"ci_sha256":ci,
            "stdout_sha256":sha256(&output.stdout),"stderr_sha256":sha256(&output.stderr),
            "head":git(&root, &["rev-parse", "HEAD"]),
            "expected_pre_repair_rejection":name.contains("collect")
        })).unwrap()).unwrap();
        copy_tree(
            &root.join(".chrono-harness/state"),
            &directory.join("state"),
        );
        println!("REPAIR_ORIGINAL {}", directory.display());
    }
    println!(
        "REPAIR_PROCESS name={name} exit={} runner={runner} ci={ci} stderr={}",
        output.status.code().unwrap(),
        String::from_utf8_lossy(&output.stderr)
    );
    (
        output.status.code().unwrap(),
        short_report(&root, args, &output, None),
    )
}
fn migration_local_short_lane() -> (Host, tempfile::TempDir, std::path::PathBuf) {
    let mut h = git_facts::bind_host(super::super::migration_host());
    let tools = tempfile::tempdir().unwrap();
    let wrapper = fs::canonicalize(tools.path())
        .unwrap()
        .join("python-migration");
    let python = fs::canonicalize(&h.tool).unwrap();
    let bytes = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec '{}' \"$@\"\n",
        tools.path().join("business-launches").display(),
        python.display()
    );
    fs::write(&wrapper, &bytes).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    h.tool = wrapper.clone();
    let cfg = h.values.get_mut(CONFIG).unwrap();
    cfg["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "python")
        .unwrap()["program"] = json!(wrapper);
    let input = cfg["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["id"] == "interpreter")
        .unwrap();
    input["location"] = json!(wrapper);
    input["sha256"] = json!(sha256(bytes.as_bytes()));
    cfg["protocol"]["timeout_seconds"] = json!(180);
    cfg["execution_units"] = json!({"units":{"one":{"tests":["test:t"],"report_path":".chrono-harness/state/one/check.json"},"two":{"tests":["test:decoder-tests"],"report_path":".chrono-harness/state/two/check.json"}},"shared_operations":{},"collection_limits":{"manifest_bytes":1048576,"report_bytes":67108864},"report_path":".chrono-harness/state/collection/check.json"});
    // This real local birth baseline is readable by worktree's registration
    // owner. Historical FILEMAP plans still require the candidate decoder;
    // malformed retired scripts belong to the existing explicit-entry fixture.
    let workflow = h.values.get_mut(WORKFLOW).unwrap();
    workflow["historical_profiles"][0]["legacy_records"] = json!([]);
    workflow["historical_profiles"][0]["mappings"] = json!([]);
    workflow["migrations"][0]["mappings"] = json!([]);
    workflow["retirements"] = json!([]);
    let profile_path = h.root().join(".chrono-harness/profile.json");
    let mut profile = chrono_harness::json(&fs::read(&profile_path).unwrap()).unwrap();
    profile["policy"]["bindings"]
        .as_object_mut()
        .unwrap()
        .remove("test:legacy");
    fs::write(profile_path, serde_json::to_vec(&profile).unwrap()).unwrap();
    let (h, tools) = adopt_short_host(h, tools, false, None, false, true);
    make_local_lane(h, tools, "integration")
}
fn repair_collected(root: &Path, name: &str, expected_red: &str) -> Option<Value> {
    let (exit, report) = repair_short(root, name, &["check", "--collect"]);
    if std::env::var_os("CHRONO_COLLECTION_REPAIR_RED").is_some() {
        assert_eq!(exit, 2, "expected sealed-source rejection: {report}");
        assert!(
            report["stderr"].as_str().unwrap().contains(expected_red),
            "{report}"
        );
        None
    } else {
        passed(exit, &report);
        Some(report)
    }
}
#[test]
fn repair_q1_local_full_migration_preserves_registered_runner_and_retained_conversion() {
    let (_h, tools, lane) = migration_local_short_lane();
    let mut originals = vec![];
    for unit in ["one", "two"] {
        let (exit, report) = repair_short(
            &lane,
            &format!("q1-unit-{unit}"),
            &["check", "--unit", unit],
        );
        passed(exit, &report);
        let registration = row(&mut report.clone(), "registration");
        assert!(report["judges"][registration]["response"]["outputs"]["registration_view"]["conversion"].is_object());
        let path = lane.join(format!(".chrono-harness/state/{unit}/check.json"));
        originals.push((path.clone(), fs::read(path).unwrap()));
    }
    let before = marker(tools.path());
    // No decoder/version or business execution is available to the collector.
    fs::remove_file(tools.path().join("python-migration")).unwrap();
    let report = repair_collected(&lane, "q1-collect", "original conversion bootstrap binding");
    assert_eq!(marker(tools.path()), before);
    for (path, bytes) in &originals {
        assert_eq!(fs::read(path).unwrap(), *bytes);
    }
    let Some(report) = report else {
        return;
    };
    assert_eq!(
        report["tests"]["completion"]["required_units"],
        json!(["one", "two"])
    );
    local_collection_observation(&lane, &report);
    for (index, (path, original)) in originals.iter().enumerate() {
        for kind in ["runner", "context", "endpoint", "judge", "conversion"] {
            let mut invalid: Value = chrono_harness::json(original).unwrap();
            let registration = row(&mut invalid, "registration");
            match kind {
                "runner" => invalid["request"]["runner"]["sha256"] = json!("0".repeat(64)),
                "context" => invalid["request"]["context"]["sha256"] = json!("0".repeat(64)),
                "endpoint" => {
                    invalid["request"]["candidate"]["commit"] =
                        invalid["request"]["base"]["commit"].clone()
                }
                "judge" => {
                    invalid["judges"][registration]["process"]["sha256"] = json!("0".repeat(64))
                }
                "conversion" => {
                    invalid["judges"][registration]["response"]["outputs"]["registration_view"]["conversion"]
                        ["input_digest"] = json!("0".repeat(64))
                }
                _ => unreachable!(),
            }
            fs::write(path, serde_json::to_vec(&invalid).unwrap()).unwrap();
            let (exit, _) = repair_short(
                &lane,
                &format!("q1-selected-{index}-{kind}"),
                &["check", "--collect"],
            );
            assert_ne!(exit, 0, "accepted selected original {kind} mismatch");
            assert_eq!(marker(tools.path()), before);
            fs::write(path, original).unwrap();
        }
    }
}
#[test]
fn repair_q2_local_full_one_and_empty_delta_ignore_unrelated_malformed_and_stale_conversion() {
    // Genuine old conversion bytes come from a separate real successful unit,
    // with different fixed endpoints/context, never a synthesized report.
    let (_old, old_tools, old_lane) = migration_local_short_lane();
    let (exit, old_two) = repair_short(&old_lane, "q2-old-unit-two", &["check", "--unit", "two"]);
    passed(exit, &old_two);
    let stale = fs::read(old_lane.join(".chrono-harness/state/two/check.json")).unwrap();
    let (exit, old_one) = repair_short(&old_lane, "q2-old-unit-one", &["check", "--unit", "one"]);
    passed(exit, &old_one);
    let selected_stale = fs::read(old_lane.join(".chrono-harness/state/one/check.json")).unwrap();
    assert!(
        old_two["judges"][0]["response"]["outputs"]["registration_view"]["conversion"].is_object()
    );
    for required in ["one", "empty"] {
        let (mut h, tools) = short_host(false);
        // Fix the registered integration obligation before the baseline: this
        // fixture intentionally has one required unit, rather than the broader
        // two-unit stability obligation used by the other full Rust fixtures.
        h.values.get_mut(WORKFLOW).unwrap()["integration"]["tests"] = json!(["t"]);
        for prod in ["p", "p2"] {
            fs::write(
                h.root().join(format!("{prod}/src/lib.rs")),
                "pub fn double(n:i32)->i32 {n*2}\n",
            )
            .unwrap();
        }
        h.save();
        h.base = h.candidate.clone();
        git(&h.root(), &["branch", "-f", "dev", &h.base]);
        git(&h.root(), &["push", "-q", "origin", "dev"]);
        if required == "one" {
            fs::write(
                h.root().join("p/src/lib.rs"),
                "pub fn double(n:i32)->i32 {n+n}\n",
            )
            .unwrap();
        }
        h.save();
        let (_h, tools, lane) = make_local_lane(h, tools, "integration");
        let required_path = lane.join(".chrono-harness/state/one/check.json");
        let original = if required == "one" {
            let (exit, report) =
                repair_short(&lane, "q2-current-unit-one", &["check", "--unit", "one"]);
            passed(exit, &report);
            let impact = report["judges"]
                .as_array()
                .unwrap()
                .iter()
                .find(|j| j["id"] == "filemap")
                .unwrap();
            assert_eq!(
                impact["response"]["outputs"]["impact"]["tests"],
                json!(["test:t"])
            );
            Some(fs::read(&required_path).unwrap())
        } else {
            None
        };
        let before = marker(tools.path());
        for (case, bytes, red) in [
            ("malformed", b"{}".as_slice(), "original judges"),
            (
                "stale-conversion",
                stale.as_slice(),
                "original conversion bootstrap binding",
            ),
        ] {
            let path = lane.join(".chrono-harness/state/two/check.json");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, bytes).unwrap();
            if let Some(report) =
                repair_collected(&lane, &format!("q2-{required}-{case}-collect"), red)
            {
                assert_eq!(
                    report["tests"]["completion"]["required_units"],
                    if required == "one" {
                        json!(["one"])
                    } else {
                        json!([])
                    }
                );
                assert_eq!(report["judges"].as_array().unwrap().len(), 7);
                local_collection_observation(&lane, &report);
            }
            assert_eq!(fs::read(path).unwrap(), bytes);
            assert_eq!(marker(tools.path()), before);
        }
        if std::env::var_os("CHRONO_COLLECTION_REPAIR_RED").is_none() {
            if let Some(original) = original {
                for case in ["missing", "malformed", "stale"] {
                    match case {
                        "missing" => fs::remove_file(&required_path).unwrap(),
                        "malformed" => fs::write(&required_path, b"{}").unwrap(),
                        "stale" => fs::write(&required_path, &selected_stale).unwrap(),
                        _ => unreachable!(),
                    }
                    let (exit, _) = repair_short(
                        &lane,
                        &format!("q2-selected-{case}-collect"),
                        &["check", "--collect"],
                    );
                    assert_ne!(exit, 0, "accepted selected {case} report");
                    assert_eq!(marker(tools.path()), before);
                    fs::write(&required_path, &original).unwrap();
                }
            }
        }
    }
    assert!(!marker(old_tools.path()).is_empty());
}
