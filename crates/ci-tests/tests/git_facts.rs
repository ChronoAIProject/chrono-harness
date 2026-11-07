#[path = "support/tools.rs"]
mod tools;
use tools::fixture_git;
#[path = "full.rs"]
mod full;
use chrono_ci::{Config, generate, init, load, prepare};
use chrono_harness::sha256;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const SOURCE: &str = ".chrono-harness/ci/github.json";
const FACTS: &str = ".chrono-harness/event facts.json";

fn write(root: &Path, path: &str, value: &Value) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn read(root: &Path, path: &str) -> Value {
    serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap()
}
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new(fixture_git())
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn commit(root: &Path, amend: bool) -> String {
    git(root, &["add", "."]);
    let mut args = vec![
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "--no-gpg-sign",
        "-qm",
        "registered event fixture",
    ];
    if amend {
        args.push("--amend");
    }
    git(root, &args);
    git(root, &["rev-parse", "HEAD"])
}
struct Host {
    _dir: tempfile::TempDir,
    _inputs: tempfile::TempDir,
    root: PathBuf,
    trace: PathBuf,
    program: PathBuf,
    candidate: String,
}
impl Host {
    fn new(mode: &str) -> Self {
        let dir = tempfile::Builder::new()
            .prefix("event host λ ")
            .tempdir()
            .unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let inputs = tempfile::Builder::new()
            .prefix("event tools λ ")
            .tempdir()
            .unwrap();
        let external = fs::canonicalize(inputs.path()).unwrap();
        let program = external.join("chosen git");
        let trace = external.join("process.trace");
        let real = fixture_git();
        let version = Command::new(&real).arg("--version").output().unwrap();
        assert!(version.status.success());
        let helper = env!("CARGO_BIN_EXE_chrono-ci-test-git");
        fs::copy(helper, &program).unwrap();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut source = read(&repo, ".chrono-harness/ci/units.json")["collection"].clone();
        source["schema"] = "chrono-github-ci/v4".into();
        source["facts_config"] = FACTS.into();
        source["push_baselines"] = json!([]);
        write(&root, SOURCE, &source);
        let facts = json!({"schema_version":3,
            "facts_git":{"tool":"event-git","input":"event-git-bytes"},
            "tools":[{"id":"event-git","program":program,"resolution":"PATH-once",
                "version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}],
            "environment":{"inherit":[],"values":{"PATH":std::env::var("PATH").unwrap(),
                "BOUND_EVENT_VALUE":"declared","EVENT_TRACE":trace,
                "EVENT_REAL_GIT":real,"EVENT_GIT_MODE":mode,"CHRONO_TEST_GIT_PROFILE":"event",
                "GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"},
                "inputs":[{"id":"event-git-bytes","presence":"present","location":program,
                    "sha256":sha256(&fs::read(&program).unwrap())}]},
            "protocol":{"timeout_seconds":10,"stdout_limit_bytes":1048576}});
        write(&root, FACTS, &facts);
        git(&root, &["init", "-q", "-b", "dev"]);
        let candidate = commit(&root, false);
        Self {
            _dir: dir,
            _inputs: inputs,
            root,
            trace,
            program,
            candidate,
        }
    }
    fn config(&self) -> Config {
        load(&self.root.join(SOURCE)).expect("current v4 source must be accepted")
    }
    fn push(&self, base: &str) -> Result<Value, String> {
        prepare(
            &self.root,
            &self.config(),
            "push",
            &json!({"before":base,"after":self.candidate,"created":false}),
            &self.candidate,
        )
    }
    fn trace(&self) -> String {
        fs::read_to_string(&self.trace).unwrap_or_default()
    }
    fn revise(&mut self) {
        self.candidate = commit(&self.root, true);
    }
    fn remote(&self) -> (tempfile::TempDir, String) {
        let remote = tempfile::tempdir().unwrap();
        git(remote.path(), &["init", "-q", "-b", "stable"]);
        fs::write(remote.path().join("remote-only"), "distinct object\n").unwrap();
        let base = commit(remote.path(), false);
        git(
            &self.root,
            &["remote", "add", "origin", remote.path().to_str().unwrap()],
        );
        (remote, base)
    }
}
fn observed(error: &str) -> Value {
    let value: Value =
        serde_json::from_str(error.strip_prefix("E_CI_GIT: ").expect(error)).unwrap();
    value["git_facts"].clone()
}
fn processes(facts: &Value) -> &Vec<Value> {
    facts["processes"].as_array().unwrap()
}
fn is_probe(process: &Value) -> bool {
    process["argv"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg.as_str().unwrap().starts_with("--batch-check"))
}
fn bytes(p: &Value, stream: &str) -> Vec<u8> {
    serde_json::from_value(
        chrono_harness::full::expand_process(p).unwrap()[format!("{stream}_bytes")].clone(),
    )
    .unwrap()
}

#[test]
fn event_binding_is_versioned_explicit_and_generation_preserves_customization() {
    let h = Host::new("pass");
    let c = h.config();
    assert!(generate(&h.root, SOURCE, false).unwrap());
    assert!(!generate(&h.root, SOURCE, true).unwrap());
    assert!(!init(&h.root, &h.root.join(SOURCE)).unwrap());
    assert_eq!(load(&h.root.join(SOURCE)).unwrap().name, c.name);
    for case in [
        "v1-field", "v1-null", "v2-field", "missing", "null", "outside", "state",
    ] {
        let mut bad = read(&h.root, SOURCE);
        match case {
            "v1-field" | "v1-null" => {
                bad["schema"] = "chrono-github-ci/v1".into();
                if case == "v1-null" {
                    bad["facts_config"] = Value::Null;
                }
            }
            "v2-field" => bad["schema"] = "chrono-github-ci/v2".into(),
            "missing" => {
                bad.as_object_mut().unwrap().remove("facts_config");
            }
            "null" => bad["facts_config"] = Value::Null,
            "outside" => bad["facts_config"] = "elsewhere.json".into(),
            _ => bad["facts_config"] = ".chrono-harness/state/temporary.json".into(),
        }
        write(&h.root, ".chrono-harness/ci/bad.json", &bad);
        assert!(
            load(&h.root.join(".chrono-harness/ci/bad.json")).is_err(),
            "{case}"
        );
    }
}

#[test]
fn bound_event_preparation_ignores_ambient_git_and_retains_original_processes() {
    if isolated_event_test() {
        return;
    }
    let h = Host::new("pass");
    let payload = json!({"before":h.candidate,"after":h.candidate,"created":false});
    let report = prepare(&h.root, &h.config(), "push", &payload, &h.candidate).unwrap();
    let facts = &report["git_facts"];
    assert_eq!(facts["binding"]["path"], json!(h.program));
    assert_eq!(facts["input_closure_complete"], false);
    assert!(processes(facts).iter().any(is_probe));
    for p in processes(facts) {
        assert_eq!(p["argv"][0], json!(h.program));
        assert_eq!(p["exit_code"], 0);
        assert!(p["failure"].is_null());
        assert_eq!(p["environment"]["BOUND_EVENT_VALUE"], "declared");
        assert!(p["environment"].get("CHRONO_EVENT_AMBIENT").is_none());
        for stream in ["stdout", "stderr"] {
            assert_eq!(p[format!("{stream}_sha256")], sha256(&bytes(p, stream)));
        }
    }
    assert_eq!(
        report["canonical_argv"],
        json!([h.config().runner, "check"])
    );
}

// Re-execute the real test so API calls observe an isolated ambient environment
// without changing the environment of concurrently running tests.
fn isolated_event_test() -> bool {
    let name = std::thread::current().name().unwrap().to_owned();
    if std::env::var("CHRONO_EVENT_TEST_CHILD").as_deref() == Ok(name.as_str()) {
        return false;
    }
    let shadow = tempfile::tempdir().unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_chrono-ci-test-git"),
        shadow.path().join("git"),
    )
    .unwrap();
    let out = Command::new(std::env::current_exe().unwrap())
        .current_dir("/")
        .env_clear()
        .env("PATH", shadow.path())
        .env("CHRONO_EVENT_AMBIENT", "must-not-leak")
        .env("CHRONO_EVENT_TEST_CHILD", &name)
        .args(["--exact", &name, "--nocapture"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("1 passed; 0 failed;"),
        "{out:?}"
    );
    true
}

#[test]
fn native_platform_policy_serves_real_push_and_pr_acquisition() {
    if isolated_event_test() {
        return;
    }
    for event in ["push", "pull_request"] {
        let mut h = Host::new("pass");
        let chosen = ".chrono-harness/native event.json";
        write(&h.root, chosen, &read(&h.root, FACTS));
        let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
        write(
            &h.root,
            FACTS,
            &json!({"schema":"chrono-git-configs/v1",
            "platforms":{platform.clone():chosen}}),
        );
        generate(&h.root, SOURCE, false).unwrap();
        h.revise();
        let (remote, base) = h.remote();
        let payload = if event == "push" {
            json!({"before":base,"after":h.candidate,"created":false})
        } else {
            git(
                remote.path(),
                &[
                    "fetch",
                    "--no-write-fetch-head",
                    h.root.to_str().unwrap(),
                    &h.candidate,
                ],
            );
            let merge = tools::pr_workflow_merge(remote.path(), &base, &h.candidate);
            json!({"pull_request":{"base":{"sha":base},"head":{"sha":h.candidate},"merge_commit_sha":merge}})
        };
        let revision = payload["pull_request"]["merge_commit_sha"]
            .as_str()
            .unwrap_or(&h.candidate);
        let report = prepare(&h.root, &h.config(), event, &payload, revision).unwrap();
        assert_eq!(report["base"], base);
        assert_eq!(report["candidate"], h.candidate);
        let facts = &report["git_facts"];
        assert_eq!(facts["selection"]["platform"], platform);
        assert_eq!(facts["selection"]["path"], FACTS);
        assert_eq!(facts["config_path"], chosen);
        assert_eq!(facts["binding"]["path"], json!(h.program));
        assert!(
            processes(facts)
                .iter()
                .any(|p| p["argv"].as_array().unwrap().contains(&json!("fetch")))
        );
        assert_eq!(
            report["canonical_argv"],
            json!([h.config().runner, "check"])
        );
        let trace = h.trace();
        let mut drift = read(&h.root, FACTS);
        drift["platforms"]["additional-system"] = json!(".chrono-harness/future.json");
        write(&h.root, FACTS, &drift);
        assert!(
            h.push(&base)
                .unwrap_err()
                .contains("selector differs from fixed candidate")
        );
        assert!(!h.trace().strip_prefix(&trace).unwrap().contains(" fetch "));
    }
}

#[test]
fn missing_commit_is_fetched_and_reprobed_through_bound_git() {
    let h = Host::new("pass");
    let (_remote, base) = h.remote();
    let r = h.push(&base).unwrap();
    let p = processes(&r["git_facts"]);
    let missing = p
        .iter()
        .position(|p| is_probe(p) && bytes(p, "stdout") == format!("{base} missing\n").as_bytes())
        .unwrap();
    let fetch = p
        .iter()
        .position(|p| p["argv"].as_array().unwrap().contains(&json!("fetch")))
        .unwrap();
    let present = p
        .iter()
        .position(|p| is_probe(p) && bytes(p, "stdout") == format!("{base} commit\n").as_bytes())
        .unwrap();
    assert!(missing < fetch && fetch < present);
    assert_eq!(
        p[missing]["stdin_sha256"],
        sha256(format!("{base}\n").as_bytes())
    );
    assert_eq!(r["base"], base);
    assert_eq!(
        git(&h.root, &["rev-parse", &format!("{base}^{{commit}}")]),
        base
    );
}

#[test]
fn failed_probe_preserves_original_bytes_and_never_fetches() {
    let h = Host::new("probe-failure");
    let error = h.push(&h.candidate).unwrap_err();
    let facts = observed(&error);
    let failed = processes(&facts)
        .iter()
        .find(|p| p["exit_code"] == 73)
        .expect("original failed probe must be retained");
    assert_eq!(bytes(failed, "stderr"), b"\xfforiginal-probe\n");
    assert!(
        !h.trace().contains(" fetch "),
        "execution failure must not trigger fetch"
    );
}

#[test]
fn malformed_or_wrong_type_probe_does_not_trigger_fetch() {
    for mode in ["malformed-probe", "wrong-type-probe", "extra-probe-output"] {
        let h = Host::new(mode);
        let error = h.push(&h.candidate).unwrap_err();
        assert!(processes(&observed(&error)).iter().any(is_probe));
        assert!(!h.trace().contains(" fetch "));
    }
}

#[test]
fn absent_probe_and_failed_fetch_both_remain_observable() {
    let h = Host::new("pass");
    git(
        &h.root,
        &[
            "remote",
            "add",
            "origin",
            "/does-not-exist/chrono-test-remote",
        ],
    );
    let absent = "1".repeat(40);
    let error = h.push(&absent).unwrap_err();
    let facts = observed(&error);
    assert!(
        processes(&facts)
            .iter()
            .any(|p| is_probe(p) && bytes(p, "stdout") == format!("{absent} missing\n").as_bytes())
    );
    let fetch = processes(&facts)
        .iter()
        .find(|p| p["argv"].as_array().unwrap().contains(&json!("fetch")))
        .unwrap();
    assert_ne!(fetch["exit_code"], 0);
    assert!(!bytes(fetch, "stderr").is_empty());
}

#[test]
fn initial_pr_and_configured_baseline_routes_share_the_declared_reader() {
    let mut h = Host::new("pass");
    let mut c = read(&h.root, SOURCE);
    c["initial_inventory"] = json!({"path":".chrono-harness/ci/root.json","profile":{
        "schema":"chrono-initial-check/v1","host_config":FACTS,"timeout_seconds":10,"stdout_limit_bytes":1048576,
        "judges":[{"id":"inventory","executable":".chrono-harness/bin/judge","version":"0.1.0","sha256":"a".repeat(64),
            "argv":["--protocol","chrono-initial-judge/v1"],"selector":"every-initial","modes":["inventory"],"after":[]}]}});
    write(&h.root, SOURCE, &c);
    generate(&h.root, SOURCE, false).unwrap();
    h.revise();
    let initial = prepare(
        &h.root,
        &h.config(),
        "workflow_dispatch",
        &json!({"inputs":{"candidate":h.candidate,"initial":true}}),
        &h.candidate,
    )
    .unwrap();
    assert_eq!(
        initial["canonical_argv"],
        json!([h.config().runner, "check"])
    );
    assert!(h.trace().contains("cat-file commit"));
    let (_remote, base) = h.remote();
    let mut configured = h.config();
    configured.push_baselines = vec![chrono_ci::PushBaseline {
        ref_prefix: "refs/heads/verify/".into(),
        base_ref: "refs/heads/stable".into(),
    }];
    let integrated = prepare(
        &h.root,
        &configured,
        "push",
        &json!({"before":"0".repeat(40),"after":h.candidate,
        "ref":"refs/heads/verify/change","created":true}),
        &h.candidate,
    )
    .unwrap();
    assert_eq!(integrated["base"], base);
    assert_eq!(integrated["source"], "push-configured-baseline-ref");
    assert!(
        processes(&integrated["git_facts"])
            .iter()
            .any(|p| p["argv"].as_array().unwrap().contains(&json!("ls-remote")))
    );
    let merge = tools::pr_workflow_merge(&h.root, &base, &h.candidate);
    let pr = prepare(
        &h.root,
        &h.config(),
        "pull_request",
        &json!({"pull_request":{"base":{"sha":base},"head":{"sha":h.candidate}}}),
        &merge,
    )
    .unwrap();
    assert_eq!(pr["candidate"], h.candidate);
    assert_eq!(pr["workflow_source_revision"], merge);
}

#[test]
fn binding_source_and_executable_failures_precede_network_observation() {
    for case in [
        "source-drift",
        "digest",
        "version",
        "legacy-binding",
        "checkout",
    ] {
        let mut h = Host::new("pass");
        let mut facts = read(&h.root, FACTS);
        match case {
            "source-drift" => facts["environment"]["values"]["NEW"] = "changed".into(),
            "digest" => facts["environment"]["inputs"][0]["sha256"] = "0".repeat(64).into(),
            "version" => facts["tools"][0]["expected_version"] = "wrong Git".into(),
            "checkout" => {}
            _ => {
                facts["schema_version"] = 2.into();
                facts.as_object_mut().unwrap().remove("facts_git");
            }
        }
        write(&h.root, FACTS, &facts);
        if case != "source-drift" && case != "checkout" {
            h.revise();
        }
        if case == "checkout" {
            fs::write(h.root.join("later.txt"), "another checkout").unwrap();
            commit(&h.root, false);
        }
        let error = h.push(&"1".repeat(40)).unwrap_err();
        let expected = match case {
            "source-drift" => "config differs from fixed candidate",
            "digest" => "declared digest mismatch",
            "version" => "version mismatch",
            "legacy-binding" => "requires a full v3",
            _ => "checkout is not exact event candidate",
        };
        assert!(error.contains(expected), "{case}: {error}");
        assert!(
            !h.trace().contains(" fetch ") && !h.trace().contains("ls-remote"),
            "{case}: {error}"
        );
        assert!(!error.is_empty());
        if case == "digest" || case == "legacy-binding" {
            assert!(h.trace().is_empty());
        }
    }
}

#[test]
fn relative_host_root_uses_the_same_bound_checkout() {
    if isolated_event_test() {
        return;
    }
    let h = Host::new("pass");
    let report = prepare(
        h.root.strip_prefix("/").unwrap(),
        &h.config(),
        "push",
        &json!({"before":h.candidate,"after":h.candidate,"created":false}),
        &h.candidate,
    )
    .expect("relative host root must work");
    assert_eq!(report["candidate"], h.candidate);
    assert_eq!(report["base"], h.candidate);
    for process in processes(&report["git_facts"]) {
        let argv = process["argv"].as_array().unwrap();
        if let Some(i) = argv.iter().position(|v| v == "-C") {
            assert_eq!(argv[i + 1], json!(h.root));
        }
    }
}
