//! Real seven-judge short entry, existing local producers and native unit transport.
use super::*;
const PROVIDER: &str = ".chrono-harness/ci/units.json";
const WORKTREE: &str = ".chrono-harness/worktree.json";
fn short_host(native: bool) -> (Host, tempfile::TempDir) {
    configured_short_host(native, None)
}
fn configured_short_host(native: bool, age_seconds: Option<f64>) -> (Host, tempfile::TempDir) {
    let (mut h, tools) = rust_host();
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
    let worktree = json!({"schema":"chrono-worktree-config/v2","host_config":CONFIG,"remote":"origin","git":{"program":git_bin,"expected_version":null,"sha256":sha256(&fs::read(&git_bin).unwrap())},"environment":{"inherit":["HOME"],"values":{"PATH":h.values[CONFIG]["environment"]["values"]["PATH"],"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"}},"timeout_seconds":30,"output_limit_bytes":1048576,"report_directory":".chrono-harness/state/worktrees/","check_inputs":{"origin_path":".chrono-harness/state/origin.json","context_path":".chrono-harness/state/local/context.json","collection_manifest":".chrono-harness/state/collection/manifest.json","roles":{"integration":"integration","feature":"integration"}}});
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
    for prod in ["p", "p2"] {
        fs::write(
            root.join(format!("{prod}/src/lib.rs")),
            "pub fn double(n:i32)->i32 {n*2}\n",
        )
        .unwrap();
    }
    if let Some(seconds) = age_seconds {
        h.values.get_mut(WORKFLOW).unwrap()["staleness"]["max_age_hours"] = json!(seconds / 3600.0);
    }
    h.save();
    h.base = h.candidate.clone();
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
    for prod in ["p", "p2"] {
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
fn short_report(root: &Path, args: &[&str], output: &std::process::Output) -> Value {
    if output.stdout.is_empty() {
        assert!(!output.status.success());
        return json!({"stderr":String::from_utf8_lossy(&output.stderr)});
    }
    assert!(output.stderr.is_empty());
    let config = chrono_harness::json(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    let path = match args {
        ["check", "--unit", unit] => config["execution_units"]["units"][*unit]["report_path"]
            .as_str()
            .unwrap(),
        ["check", "--collect"] => config["execution_units"]["report_path"].as_str().unwrap(),
        ["check"] => ".chrono-harness/state/report.json",
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
    let mut command = Command::new(root.join(".chrono-harness/bin/chrono-harness"));
    command
        .current_dir(root)
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
    }
    let report = short_report(root, args, &output);
    println!(
        "SHORT {:?} source={:?} exit={} bytes={}",
        args,
        source,
        output.status.code().unwrap(),
        output.stdout.len()
    );
    (output.status.code().unwrap(), report)
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
    let (h, tools) = configured_short_host(false, age_seconds);
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
            "integration",
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
    let business = tools.path().join("cargo-fixture");
    let bytes = fs::read(&business).unwrap();
    fs::remove_file(&business).unwrap();
    let before = marker(tools.path());
    let (exit, collection) = short(&lane, &["check", "--collect"], None);
    passed(exit, &collection);
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
        short_report(root, &["check", "--collect"], &out)
    } else {
        short_report(root, &["check", "--unit", id], &out)
    };
    (out.status.code().unwrap(), report)
}
#[test]
fn generated_automatic_full_units_gather_originals_on_separate_collection() {
    generated_full_transport(false);
}
#[test]
fn generated_full_units_acquire_external_preparation_originals() {
    generated_full_transport(true);
}
fn generated_full_transport(external_originals: bool) {
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
        if external_originals {
            // Lossless transport fixture: exercise the actual prepared binding
            // consumers using exact context and acquisition bytes, including
            // short originals beneath a narrow native upload root.
            let mut portable = report.clone();
            let context = portable["request"]["context"]["path"]
                .as_str()
                .unwrap()
                .to_owned();
            let receipt = portable["preparation"]["receipts"][0]["path"]
                .as_str()
                .unwrap()
                .to_owned();
            for address in [context, receipt] {
                let bytes = chrono_harness::full::artifact_bytes_at(
                    unit_root,
                    &portable["artifacts"],
                    &address,
                )
                .unwrap();
                let raw = unit_root.join(format!(".chrono-harness/state/{id}/transport-original"));
                fs::write(&raw, &bytes).unwrap();
                let (original, length) = chrono_harness::prepared::retain_blob(
                    unit_root,
                    &format!(".chrono-harness/state/{id}/blobs/"),
                    &raw,
                )
                .unwrap();
                portable["artifacts"][address] =
                    json!({"blob":original.path,"sha256":original.sha256,"length":length});
            }
            fs::write(
                unit_root.join(format!(".chrono-harness/state/{id}/check.json")),
                serde_json::to_vec(&portable).unwrap(),
            )
            .unwrap();
        }
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
    if external_originals {
        // The bounded collector preserved external preparation originals in its
        // own closure; the original unit upload/read locations are unavailable.
        for input in &manifest.reports {
            let original_bytes = fs::read(collector.path().join(&input.path)).unwrap();
            let retained = collected["tests"]["completion"]["reports"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["unit"] == input.unit)
                .unwrap();
            let retained_bytes = chrono_harness::full::artifact_bytes_at(
                collector.path(),
                &collected["artifacts"],
                retained["retained_path"].as_str().unwrap(),
            )
            .unwrap();
            assert_eq!(retained_bytes, original_bytes);
        }
        println!(
            "FULL_EXTERNAL_TRANSPORT scope=local_mock_github run_attempts_preserved=true prepared_context_and_acquisition_exact=true business_reexecution=0"
        );
    }
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
