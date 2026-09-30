#[path = "full.rs"]
mod full;
use chrono_ci::{Config, generate, init, load, prepare};
use chrono_harness::sha256;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
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
    let out = Command::new("git")
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
fn quote(path: &Path) -> String {
    format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"))
}
struct Host {
    _dir: tempfile::TempDir,
    _inputs: tempfile::TempDir,
    root: PathBuf,
    trace: PathBuf,
    program: PathBuf,
    shadow: PathBuf,
    candidate: String,
}
impl Host {
    fn new(body: &str) -> Self {
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
        let real = chrono_harness::resolve_program(&root, "git", None).unwrap();
        let version = Command::new(&real).arg("--version").output().unwrap();
        assert!(version.status.success());
        let script = format!(
            "#!/bin/sh\n[ \"$BOUND_EVENT_VALUE\" = declared ] || exit 81\n[ -z \"$CHRONO_EVENT_AMBIENT\" ] || exit 82\nprintf '%s\\n' \"$*\" >> \"$EVENT_TRACE\" || exit $?\n{body}\nexec {} \"$@\"\n",
            quote(&real)
        );
        fs::write(&program, script).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        let shadow = external.join("ambient-bin");
        fs::create_dir(&shadow).unwrap();
        fs::write(
            shadow.join("git"),
            "#!/bin/sh\necho ambient-git >&2\nexit 83\n",
        )
        .unwrap();
        fs::set_permissions(shadow.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut source = read(&repo, ".chrono-harness/ci/units.json")["collection"].clone();
        source["schema"] = "chrono-github-ci/v3".into();
        source["facts_config"] = FACTS.into();
        source["push_baselines"] = json!([]);
        write(&root, SOURCE, &source);
        let facts = json!({"schema_version":3,
            "facts_git":{"tool":"event-git","input":"event-git-bytes"},
            "tools":[{"id":"event-git","program":program,"resolution":"PATH-once",
                "version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}],
            "environment":{"inherit":[],"values":{"PATH":std::env::var("PATH").unwrap(),
                "BOUND_EVENT_VALUE":"declared","EVENT_TRACE":trace,
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
            shadow,
            candidate,
        }
    }
    fn config(&self) -> Config {
        load(&self.root.join(SOURCE)).expect("explicit v3 source must be accepted")
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
    serde_json::from_value(p[format!("{stream}_bytes")].clone()).unwrap()
}

#[test]
fn event_binding_is_versioned_explicit_and_generation_preserves_customization() {
    let h = Host::new("");
    let c = h.config();
    assert!(generate(&h.root, SOURCE, false).unwrap());
    assert!(!generate(&h.root, SOURCE, true).unwrap());
    assert!(!init(&h.root, &h.root.join(SOURCE)).unwrap());
    assert_eq!(load(&h.root.join(SOURCE)).unwrap().name, c.name);
    for case in [
        "v1-field",
        "v1-null",
        "v2-field",
        "v3-missing",
        "v3-null",
        "v3-outside",
        "v3-state",
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
            "v3-missing" => {
                bad.as_object_mut().unwrap().remove("facts_config");
            }
            "v3-null" => bad["facts_config"] = Value::Null,
            "v3-outside" => bad["facts_config"] = "elsewhere.json".into(),
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
fn bound_event_cli_ignores_ambient_git_and_retains_original_processes() {
    let h = Host::new("");
    let payload = json!({"before":h.candidate,"after":h.candidate,"created":false});
    let report = prepare_cli(&h, "push", &payload);
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
        json!(chrono_harness::canonical_argv(
            &h.config().runner,
            &h.config().check_config,
            Some(&h.candidate),
            &h.candidate,
            false
        ))
    );
}

fn prepare_cli(h: &Host, event: &str, payload: &Value) -> Value {
    write(&h.root, ".chrono-harness/state/payload.json", &payload);
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ci/target/debug/chrono-ci");
    let out = Command::new(binary)
        .current_dir("/")
        .env_clear()
        .env("PATH", &h.shadow)
        .env("CHRONO_EVENT_AMBIENT", "must-not-leak")
        .args([
            "prepare",
            "--host-root",
            h.root.to_str().unwrap(),
            "--config",
            SOURCE,
            "--event",
            event,
            "--payload",
            h.root
                .join(".chrono-harness/state/payload.json")
                .to_str()
                .unwrap(),
            "--workflow-revision",
            &h.candidate,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(read(&h.root, &h.config().context_path), report);
    report
}

#[test]
fn native_platform_policy_serves_real_push_and_pr_cli_acquisition() {
    for event in ["push", "pull_request"] {
        let mut h = Host::new("");
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
        let (_remote, base) = h.remote();
        let payload = if event == "push" {
            json!({"before":base,"after":h.candidate,"created":false})
        } else {
            json!({"pull_request":{"base":{"sha":base},"head":{"sha":h.candidate}}})
        };
        let report = prepare_cli(&h, event, &payload);
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
            json!(chrono_harness::canonical_argv(
                &h.config().runner,
                &h.config().check_config,
                Some(&base),
                &h.candidate,
                false
            ))
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
    let h = Host::new("");
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
    let h = Host::new(
        "case \"$*\" in *--batch-check*) /bin/cat >/dev/null; printf '\\377original-probe\\n' >&2; exit 73;; esac",
    );
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
    for body in [
        "case \"$*\" in *--batch-check*) /bin/cat >/dev/null; printf 'not-the-request missing\\n'; exit 0;; esac",
        "case \"$*\" in *--batch-check*) read oid; printf '%s blob\\n' \"$oid\"; exit 0;; esac",
        "case \"$*\" in *--batch-check*) read oid; printf '%s commit\\nextra\\n' \"$oid\"; exit 0;; esac",
    ] {
        let h = Host::new(body);
        let error = h.push(&h.candidate).unwrap_err();
        assert!(processes(&observed(&error)).iter().any(is_probe));
        assert!(!h.trace().contains(" fetch "));
    }
}

#[test]
fn absent_probe_and_failed_fetch_both_remain_observable() {
    let h = Host::new("");
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
    let mut h = Host::new("");
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
    assert_eq!(initial["canonical_argv"][3], ".chrono-harness/ci/root.json");
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
    let pr = prepare(
        &h.root,
        &h.config(),
        "pull_request",
        &json!({"pull_request":{"base":{"sha":base},"head":{"sha":h.candidate}}}),
        &base,
    )
    .unwrap();
    assert_eq!(pr["candidate"], h.candidate);
    assert_eq!(pr["workflow_source_revision"], base);
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
        let mut h = Host::new("");
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
fn legacy_event_profile_remains_unbound() {
    let h = Host::new("");
    let mut source = read(&h.root, SOURCE);
    source["schema"] = "chrono-github-ci/v1".into();
    source.as_object_mut().unwrap().remove("facts_config");
    let c: Config = serde_json::from_value(source).unwrap();
    let report = prepare(
        &h.root,
        &c,
        "push",
        &json!({"before":h.candidate,"after":h.candidate}),
        &h.candidate,
    )
    .unwrap();
    assert!(report.get("git_facts").is_none());
    assert!(h.trace().is_empty());
}

#[test]
fn relative_host_root_uses_the_same_bound_checkout() {
    let h = Host::new("");
    write(
        &h.root,
        ".chrono-harness/state/relative-payload.json",
        &json!({"before":h.candidate,"after":h.candidate,"created":false}),
    );
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ci/target/debug/chrono-ci");
    let out = Command::new(binary)
        .current_dir(h.root.parent().unwrap())
        .env_clear()
        .env("PATH", &h.shadow)
        .args([
            "prepare",
            "--host-root",
            h.root.file_name().unwrap().to_str().unwrap(),
            "--config",
            SOURCE,
            "--event",
            "push",
            "--payload",
            h.root
                .join(".chrono-harness/state/relative-payload.json")
                .to_str()
                .unwrap(),
            "--workflow-revision",
            &h.candidate,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "relative host root must work: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["candidate"], h.candidate);
    assert_eq!(report["base"], h.candidate);
    for process in processes(&report["git_facts"]) {
        let argv = process["argv"].as_array().unwrap();
        if let Some(i) = argv.iter().position(|v| v == "-C") {
            assert_eq!(argv[i + 1], json!(h.root));
        }
    }
}
