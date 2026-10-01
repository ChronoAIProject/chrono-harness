use super::*;
use chrono_harness::{prepared, sha256};
use std::path::PathBuf;

struct ShortHost {
    _dir: tempfile::TempDir,
    root: PathBuf,
    remote: PathBuf,
    base: String,
    candidate: String,
}
impl ShortHost {
    fn new() -> Self {
        let (old, _, _) = consumer();
        let dir = tempfile::Builder::new()
            .prefix("short host λ ")
            .tempdir()
            .unwrap();
        let parent = fs::canonicalize(dir.path()).unwrap();
        let root = parent.join("arbitrary non Cargo layout");
        fs::create_dir(&root).unwrap();
        git(&root, &["clone", "-q", old.path().to_str().unwrap(), "."]);
        install(&root);
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../worktree/target/debug/chrono-worktree"),
            root.join(".chrono-harness/bin/chrono-worktree"),
        )
        .unwrap();
        git(&root, &["config", "user.name", "Fixture"]);
        git(&root, &["config", "user.email", "fixture@example.invalid"]);
        let remote = parent.join("explicit target.git");
        fs::create_dir(&remote).unwrap();
        git(&remote, &["init", "--bare", "-q", "-b", "dev"]);
        git(
            &root,
            &["remote", "set-url", "origin", remote.to_str().unwrap()],
        );
        let git_bin = chrono_harness::resolve_program(&root, "git", None).unwrap();
        let version = Command::new(&git_bin).arg("--version").output().unwrap();
        let native = json!({"operation":"prepare.check.ci","tool":"chrono-ci","argv":["check-inputs","--config",".chrono-harness/ci/units.json","--event-env","GITHUB_EVENT_NAME","--payload-env","GITHUB_EVENT_PATH","--revision-env","CHRONO_WORKFLOW_REVISION","--repository-env","GITHUB_REPOSITORY"]});
        let cfg = json!({"schema_version":4,"status":"proposed","enforcement":"not-implemented","runner":{"path":".chrono-harness/bin/chrono-harness","version":"0.1.0","sha256":null},"registries":{"judges":".chrono-harness/judges.json","projects":".chrono-harness/projects.json","filemap":".chrono-harness/FILEMAP.json","workflow":".chrono-harness/workflow.json"},"canonical_check":{"operation":"validate.delta","argv":[".chrono-harness/bin/chrono-harness","check"],"profile":".chrono-harness/ci/check.json","inputs":{"local":{"operation":"prepare.check.local","tool":"chrono-worktree","argv":["check-inputs","--config",".chrono-harness/worktree.json"]},"ci":native}},"facts_git":{"tool":"git","input":"git-bytes"},"tools":[{"id":"git","program":git_bin,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()},{"id":"chrono-worktree","program":".chrono-harness/bin/chrono-worktree","resolution":"PATH-once","version_argv":["--version"],"expected_version":"chrono-worktree 0.1.0"},{"id":"chrono-ci","program":".chrono-harness/bin/chrono-ci","resolution":"PATH-once","version_argv":["--version"],"expected_version":"chrono-ci 0.1.0"},{"id":"sh","program":"/bin/sh","resolution":"PATH-once","version_argv":["-c","printf shell"],"expected_version":"shell"}],"protocol":{"id":"chrono-judge/v1","timeout_seconds":30,"stdout_limit_bytes":67108864,"encoding":"UTF-8"},"environment":{"inherit":["PATH","HOME","CHRONO_CHECK_SOURCE","GITHUB_EVENT_NAME","GITHUB_EVENT_PATH","CHRONO_WORKFLOW_REVISION","GITHUB_REPOSITORY"],"values":{"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"},"inputs":[{"id":"git-bytes","location":git_bin,"presence":"present","sha256":sha256(&fs::read(&git_bin).unwrap())}]},"input_closure":{"status":"incomplete","unresolved":["fixture closure"]},"semantic_fields":[],"artifacts":[{"path":".chrono-harness/state/","owner":"host","kind":"evidence","tracked":false},{"path":".chrono-harness/bin/","owner":"host","kind":"executable","tracked":false}]});
        json_file(&root, ".chrono-harness/config.json", &cfg);
        json_file(
            &root,
            ".chrono-harness/judges.json",
            &json!({"schema_version":1,"status":"proposed","migration_validator":"registration","judges":[{"id":"registration","executable":".chrono-harness/bin/chrono-judge-registration","version":"0.1.0","sha256":null,"argv":["--protocol","chrono-judge/v1"],"selector":"every-delta","after":[],"modes":["evaluate"]}]}),
        );
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.chrono-harness/workflow.json");
        let mut wf: Value = serde_json::from_slice(&fs::read(source).unwrap()).unwrap();
        wf["historical_profiles"] = json!([]);
        wf["migrations"] = json!([]);
        wf["retirements"] = json!([]);
        json_file(&root, ".chrono-harness/workflow.json", &wf);
        json_file(
            &root,
            ".chrono-harness/worktree.json",
            &json!({"schema":"chrono-worktree-config/v2","host_config":".chrono-harness/config.json","remote":"origin","git":{"program":git_bin,"expected_version":null,"sha256":sha256(&fs::read(&git_bin).unwrap())},"environment":{"inherit":["PATH","HOME"],"values":{"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"}},"timeout_seconds":15,"output_limit_bytes":1048576,"report_directory":".chrono-harness/state/worktrees/","check_inputs":{"origin_path":".chrono-harness/state/origin.json","context_path":".chrono-harness/state/local/context.json","collection_manifest":".chrono-harness/state/collection/manifest.json","roles":{"integration":"integration","feature":"integration"}}}),
        );
        let mut projects: Value =
            serde_json::from_slice(&fs::read(root.join(".chrono-harness/projects.json")).unwrap())
                .unwrap();
        projects["status"] = json!("proposed");
        projects["owners"] = json!(["host", "a", "b", "a-prod", "b-prod"]);
        for p in projects["projects"].as_array_mut().unwrap() {
            p["kind"] = json!("test");
            p["tests_for"] = json!(format!("{}-prod", p["id"].as_str().unwrap()));
        }
        for id in ["a", "b"] {
            projects["projects"].as_array_mut().unwrap().push(json!({"id":format!("{id}-prod"),"kind":"production","test_project":id,"actions":{"execute":{"operation":format!("prod.{id}"),"tool":"sh","argv":["-c","exit 0"]}}}));
        }
        json_file(&root, ".chrono-harness/projects.json", &projects);
        let mut fm: Value =
            serde_json::from_slice(&fs::read(root.join(".chrono-harness/FILEMAP.json")).unwrap())
                .unwrap();
        fm["schema_version"] = json!(2);
        fm["status"] = json!("proposed");
        fm["cost_models"] = json!({"unmeasured":{"cpu_ms":null,"wall_ms":null,"peak_rss_bytes":null,"io_bytes":null,"basis":"unknown"}});
        fm["test_costs"] =
            json!([{"test":"a","cost":"unmeasured"},{"test":"b","cost":"unmeasured"}]);
        fm["execution_plans"] = json!({"test:a":{"operations":["test.a"],"timeout_seconds":5,"output_limit_bytes":4096},"test:b":{"operations":["test.b"],"timeout_seconds":5,"output_limit_bytes":4096}});
        for path in [
            ".chrono-harness/config.json",
            ".chrono-harness/judges.json",
            ".chrono-harness/workflow.json",
            ".chrono-harness/worktree.json",
        ] {
            fm["files"].as_array_mut().unwrap().push(json!({"path":path,"owner":"host","surface":"judge-policy","cost":"unmeasured","edges":[]}));
        }
        json_file(&root, ".chrono-harness/FILEMAP.json", &fm);
        let mut check: Value =
            serde_json::from_slice(&fs::read(root.join(".chrono-harness/ci/check.json")).unwrap())
                .unwrap();
        check["judge"]["output_limit_bytes"] = json!(67108864);
        check["policy"]["registration_config"] = json!(".chrono-harness/config.json");
        check["policy"]["facts_config"] = json!(".chrono-harness/config.json");
        check["policy"].as_object_mut().unwrap().remove("bindings");
        json_file(&root, ".chrono-harness/ci/check.json", &check);
        let mut provider: Value =
            serde_json::from_slice(&fs::read(root.join(".chrono-harness/ci/units.json")).unwrap())
                .unwrap();
        provider["collection"]["schema"] = json!("chrono-github-ci/v4");
        provider["collection"]["facts_config"] = json!(".chrono-harness/config.json");
        for unit in ["alpha", "beta"] {
            provider["units"][unit]["artifact_directory"] =
                json!(format!(".chrono-harness/state/{unit}/"));
        }
        json_file(&root, ".chrono-harness/ci/units.json", &provider);
        generate(&root, ".chrono-harness/ci/units.json", false).unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "short adoption"]);
        let base = git(&root, &["rev-parse", "HEAD"]);
        git(&root, &["push", "-q", "origin", "dev"]);
        fs::write(root.join("a.txt"), "three").unwrap();
        fs::write(root.join("b.txt"), "three").unwrap();
        git(&root, &["commit", "-qam", "business delta"]);
        let candidate = git(&root, &["rev-parse", "HEAD"]);
        Self {
            _dir: dir,
            root,
            remote,
            base,
            candidate,
        }
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(self.root.join(".chrono-harness/bin/chrono-harness"));
        c.current_dir(&self.root)
            .env_remove(prepared::SOURCE)
            .args(args);
        c
    }
    fn run(&self, args: &[&str], exit: i32) -> Value {
        let out = self.command(args).output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(exit),
            "{}\n{}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        if args.contains(&"--config") {
            return serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
        }
        if out.stdout.is_empty() {
            return Value::Null;
        }
        let unit = args.windows(2).find(|a| a[0] == "--unit").map(|a| a[1]);
        published_report(&self.root, unit, &out)
    }

    fn revise(&mut self) {
        git(&self.root, &["add", "."]);
        git(&self.root, &["commit", "-qm", "fixture revision"]);
        self.candidate = git(&self.root, &["rev-parse", "HEAD"]);
    }
    fn modify(&mut self, path: &str, f: impl FnOnce(&mut Value)) {
        let mut v: Value =
            serde_json::from_slice(&fs::read(self.root.join(path)).unwrap()).unwrap();
        f(&mut v);
        json_file(&self.root, path, &v);
        self.revise();
    }
}
#[test]
fn real_short_check_tracks_explicit_remote_and_head_and_preserves_entry() {
    let mut h = ShortHost::new();
    let r = h.run(&["check"], 0);
    let explicit = h
        .command(&["check"])
        .env(prepared::SOURCE, "local")
        .output()
        .unwrap();
    assert!(explicit.status.success());
    assert_eq!(r["request"]["base"], h.base);
    assert_eq!(r["request"]["candidate"], h.candidate);
    assert_eq!(
        r["request"]["observations"]["entry"]["argv"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        r["request"]["observations"]["preparation"]["result"]["base"],
        h.base
    );
    assert!(h.root.join(".chrono-harness/state/calls-a").exists());
    h.run(&["check"], 0);
    git(&h.root, &["push", "-q", "origin", "dev"]);
    let old = h.candidate.clone();
    fs::write(h.root.join("a.txt"), "four").unwrap();
    h.revise();
    let r = h.run(&["check"], 0);
    assert_eq!(r["request"]["base"], old);
    assert_eq!(r["request"]["candidate"], h.candidate);
    let receipt: Value = serde_json::from_slice(
        &fs::read(
            h.root.join(
                r["request"]["observations"]["preparation"]["receipts"][0]["path"]
                    .as_str()
                    .unwrap(),
            ),
        )
        .unwrap(),
    )
    .unwrap();
    let observed = receipt["process"]["argv"].as_array().unwrap();
    assert!(observed.iter().any(|a| a == "check-inputs"));
    assert_ne!(h.remote, h.root);
}
#[test]
fn short_units_are_independent_and_value_less_collection_runs_zero_business() {
    let h = ShortHost::new();
    let first = h.run(&["check", "--unit", "alpha"], 0);
    h.run(&["check", "--unit", "alpha"], 0);
    assert_eq!(
        serde_json::from_slice::<Value>(
            &fs::read(h.root.join(first["retained_report"].as_str().unwrap())).unwrap()
        )
        .unwrap(),
        first
    );
    assert!(h.root.join(".chrono-harness/state/calls-a").exists());
    assert!(!h.root.join(".chrono-harness/state/calls-b").exists());
    h.run(&["check", "--collect"], 2);
    h.run(&["check", "--unit", "beta"], 0);
    let a = fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap();
    let b = fs::read(h.root.join(".chrono-harness/state/calls-b")).unwrap();
    h.run(&["check", "--collect"], 0);
    h.run(&["check", "--collect"], 0);
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap(),
        a
    );
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/state/calls-b")).unwrap(),
        b
    );
}
#[test]
fn short_preserves_actual_business_and_producer_failures() {
    let mut h = ShortHost::new();
    fs::write(h.root.join("a.sh"), "exit 7\n").unwrap();
    h.revise();
    let r = h.run(&["check"], 1);
    assert!(
        r["response"]["evidence"]["executed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["process"]["exit_code"] == 7)
    );
    git(
        &h.root,
        &["remote", "set-url", "origin", "/missing-explicit-remote"],
    );
    let out = h.command(&["check"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("Git fetch exited"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
fn short_rejects_dirty_missing_unknown_binding_selector_and_long_spelling() {
    let mut h = ShortHost::new();
    fs::write(h.root.join("a.txt"), "dirty").unwrap();
    h.run(&["check"], 2);
    git(&h.root, &["restore", "a.txt"]);
    for value in ["", "native", "CI"] {
        let o = h
            .command(&["check"])
            .env(prepared::SOURCE, value)
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&o.stderr).contains("CHRONO_CHECK_SOURCE"));
    }
    let o = h
        .command(&["check"])
        .env(prepared::SOURCE, "ci")
        .env_remove("GITHUB_EVENT_NAME")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("native input"));
    h.run(
        &[
            "check",
            "--config",
            ".chrono-harness/ci/check.json",
            "--base",
            &h.base,
            "--candidate",
            &h.candidate,
        ],
        1,
    );
    h.run(&["check", "--base", &h.base], 2);
    h.run(&["check", "--collect", "some.json"], 2);
    h.modify(".chrono-harness/config.json", |c| {
        c["canonical_check"]["inputs"]["local"]["tool"] = json!("missing")
    });
    h.run(&["check"], 2);
}
#[test]
fn generated_native_check_step_executes_same_short_command_and_event_rules() {
    let h = ShortHost::new();
    let event = h.root.join(".chrono-harness/state/event.json");
    json_file(
        &h.root,
        ".chrono-harness/state/event.json",
        &json!({"ref":"refs/heads/dev","before":h.base,"after":h.candidate,"created":false,"deleted":false}),
    );
    let workflow = fs::read_to_string(h.root.join(".github/workflows/alpha.yml")).unwrap();
    assert!(!workflow.contains("Prepare fixed event inputs"));
    assert!(!workflow.contains("--base"));
    let line = workflow
        .lines()
        .find(|l| l.contains("'check' '--unit' 'alpha'"))
        .unwrap()
        .trim();
    let out = Command::new("/bin/bash")
        .current_dir(&h.root)
        .args(["-c", line])
        .env(prepared::SOURCE, "ci")
        .env("GITHUB_EVENT_NAME", "push")
        .env("GITHUB_EVENT_PATH", &event)
        .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let r = published_report(&h.root, Some("alpha"), &out);
    assert_eq!(
        r["request"]["observations"]["preparation"]["result"]["evidence"]["source"],
        "push-before-after"
    );
    assert_eq!(
        r["request"]["observations"]["entry"]["argv"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    for (event_name, payload, expected_base) in [
        (
            "pull_request",
            json!({"pull_request":{"base":{"sha":h.base},"head":{"sha":h.candidate}}}),
            h.base.clone(),
        ),
        (
            "workflow_dispatch",
            json!({"inputs":{"base":h.base,"candidate":h.candidate,"initial":false}}),
            h.base.clone(),
        ),
        (
            "push",
            json!({"ref":"refs/heads/integration/topic","before":h.candidate,"after":h.candidate,"created":false,"deleted":false}),
            h.base.clone(),
        ),
    ] {
        // The fixture's v4 provider retains its registered push rule when explicitly configured.
        json_file(&h.root, ".chrono-harness/state/event.json", &payload);
        let o = h
            .command(&["check", "--unit", "alpha"])
            .env(prepared::SOURCE, "ci")
            .env("GITHUB_EVENT_NAME", event_name)
            .env("GITHUB_EVENT_PATH", &event)
            .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
            .output()
            .unwrap();
        assert_eq!(
            o.status.code(),
            Some(0),
            "{} {}",
            String::from_utf8_lossy(&o.stderr),
            String::from_utf8_lossy(&o.stdout)
        );
        let r = published_report(&h.root, Some("alpha"), &o);
        if event_name != "push" {
            assert_eq!(r["request"]["base"], expected_base);
        }
    }
}
#[test]
fn local_collection_rejects_stale_failed_and_wrong_pin_reports_without_execution() {
    let mut h = ShortHost::new();
    h.run(&["check", "--unit", "alpha"], 0);
    h.run(&["check", "--unit", "beta"], 0);
    let path = ".chrono-harness/state/alpha/check.json";
    let raw = fs::read(h.root.join(path)).unwrap();
    let mut r: Value = serde_json::from_slice(&raw).unwrap();
    r["runner"]["sha256"] = json!("0".repeat(64));
    json_file(&h.root, path, &r);
    h.run(&["check", "--collect"], 1);
    fs::write(h.root.join(path), &raw).unwrap();
    fs::write(h.root.join("a.txt"), "advanced").unwrap();
    h.revise();
    h.run(&["check", "--collect"], 1);
    h.run(&["check", "--unit", "beta"], 0);
    fs::write(h.root.join("a.sh"), "exit 9\n").unwrap();
    h.revise();
    h.run(&["check", "--unit", "alpha"], 1);
    h.run(&["check", "--unit", "beta"], 0);
    h.run(&["check", "--collect"], 1);
}
#[test]
fn short_schema_and_native_selector_fail_closed_on_missing_ambiguous_and_drifted_inputs() {
    let mut h = ShortHost::new();
    h.modify(".chrono-harness/config.json", |c| c["extra"] = json!(true));
    let o = h.command(&["check"]).output().unwrap();
    assert!(!o.status.success());
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("unknown field extra"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    h.modify(".chrono-harness/config.json", |c| {
        c.as_object_mut().unwrap().remove("extra");
        let t = c["tools"][1].clone();
        c["tools"].as_array_mut().unwrap().push(t);
    });
    let o = h.command(&["check"]).output().unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("ambiguous input producer tool"));
    h.modify(".chrono-harness/config.json", |c| {
        c["tools"].as_array_mut().unwrap().pop();
        c["tools"][1]["expected_version"] = json!("wrong producer version");
    });
    let o = h.command(&["check"]).output().unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("producer version mismatch"));
    h.modify(".chrono-harness/config.json", |c| {
        c["tools"][1]["expected_version"] = json!("chrono-worktree 0.1.0")
    });
    fs::remove_file(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap();
    h.run(&["check"], 2);
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../worktree/target/debug/chrono-worktree"),
        h.root.join(".chrono-harness/bin/chrono-worktree"),
    )
    .unwrap();
    fs::write(h.root.join(".chrono-harness/config.json"), "{}").unwrap();
    h.run(&["check"], 2);
}
#[test]
fn configured_selector_resolves_v4_without_legacy_git_and_has_no_platform_fallback() {
    let mut h = ShortHost::new();
    let cfg: Value =
        serde_json::from_slice(&fs::read(h.root.join(".chrono-harness/config.json")).unwrap())
            .unwrap();
    let target = ".chrono-harness/native policy.json";
    json_file(&h.root, target, &cfg);
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    json_file(
        &h.root,
        ".chrono-harness/config.json",
        &json!({"schema":"chrono-git-configs/v1","platforms":{&platform:target}}),
    );
    h.modify(".chrono-harness/FILEMAP.json",|f|f["files"].as_array_mut().unwrap().push(json!({"path":target,"owner":"host","surface":"judge-policy","cost":"unmeasured","edges":[]})));
    let r = h.run(&["check", "--unit", "alpha"], 0);
    assert_eq!(
        r["request"]["observations"]["preparation"]["request"]["effective_config"],
        target
    );
    assert_eq!(
        r["response"]["evidence"]["git_facts"]["binding"]["tool"],
        "git"
    );
    h.modify(".chrono-harness/config.json", |c| {
        c["platforms"] = json!({"unsupported-architecture":target})
    });
    let o = h.command(&["check"]).output().unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("no registered platform"));
}
fn adjudicate(h: &ShortHost, req: &Value) -> Value {
    use std::io::Write;
    let mut child = Command::new(h.root.join(".chrono-harness/bin/chrono-judge-ci"))
        .current_dir(&h.root)
        .env_remove(prepared::SOURCE)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(req).unwrap())
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    serde_json::from_slice(&o.stdout).unwrap()
}
#[test]
fn actual_consumer_rejects_fabricated_expanded_entry_and_source_disagreement() {
    let h = ShortHost::new();
    let r = h.run(&["check", "--unit", "alpha"], 0);
    let bytes = fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap();
    let mut req = r["request"].clone();
    req["observations"]["entry"]["argv"] = json!([
        h.root.join(".chrono-harness/bin/chrono-harness"),
        "check",
        "--config",
        ".chrono-harness/ci/check.json",
        "--base",
        h.base,
        "--candidate",
        h.candidate,
        "--unit",
        "alpha"
    ]);
    let response = adjudicate(&h, &req);
    assert!(
        response["results"][0]["cause"]
            .as_str()
            .unwrap()
            .contains("canonical argv differs")
    );
    let mut req = r["request"].clone();
    req["observations"]["preparation"]["environment"]["inherited"][prepared::SOURCE] = json!("ci");
    let response = adjudicate(&h, &req);
    assert!(
        response["results"][0]["cause"]
            .as_str()
            .unwrap()
            .contains("source selector disagreement")
    );
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap(),
        bytes
    );
}
#[test]
fn native_short_initial_requires_explicit_parentless_event_and_canonical_entry() {
    let mut h = ShortHost::new();
    let tree = git(&h.root, &["rev-parse", "HEAD^{tree}"]);
    let candidate = git(&h.root, &["commit-tree", &tree, "-m", "parentless fixture"]);
    git(&h.root, &["checkout", "--detach", "-q", &candidate]);
    h.candidate = candidate;
    let path = h.root.join(".chrono-harness/state/event.json");
    json_file(
        &h.root,
        ".chrono-harness/state/event.json",
        &json!({"inputs":{"base":"","candidate":h.candidate,"initial":true}}),
    );
    let o = h
        .command(&["check", "--unit", "alpha"])
        .env(prepared::SOURCE, "ci")
        .env("GITHUB_EVENT_NAME", "workflow_dispatch")
        .env("GITHUB_EVENT_PATH", &path)
        .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&o.stderr),
        String::from_utf8_lossy(&o.stdout)
    );
    let r = published_report(&h.root, Some("alpha"), &o);
    assert_eq!(r["request"]["initial"], true);
    assert!(r["request"]["base"].is_null());
    assert_eq!(
        r["request"]["observations"]["entry"]["argv"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}
#[test]
fn collection_rejects_duplicate_registered_report_paths() {
    let mut h = ShortHost::new();
    h.modify(".chrono-harness/ci/check.json", |c| {
        c["policy"]["units"]["beta"]["report_path"] =
            c["policy"]["units"]["alpha"]["report_path"].clone()
    });
    let o = h.command(&["check", "--unit", "alpha"]).output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    let r = published_report(&h.root, Some("alpha"), &o);
    assert!(
        r["response"]["results"][0]["cause"]
            .as_str()
            .unwrap()
            .contains("collision")
    );
}

#[test]
fn short_scoped_binding_never_falls_into_legacy_unbound_git() {
    let mut h = ShortHost::new();
    h.modify(".chrono-harness/ci/check.json", |c| {
        c["policy"].as_object_mut().unwrap().remove("facts_config");
    });
    let o = h.command(&["check"]).output().unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("bound facts_config"));
}

#[test]
fn preparation_retains_environment_identities_without_credential_values() {
    let mut h = ShortHost::new();
    h.modify(".chrono-harness/config.json", |c| {
        c["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .push(json!("GH_TOKEN"));
        c["environment"]["credential_environment"] = json!(["GH_TOKEN"]);
    });
    let token = "fixture-credential-must-not-be-published";
    let out = h
        .command(&["check"])
        .env("GH_TOKEN", token)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report = published_report(&h.root, None, &out);
    assert!(!String::from_utf8_lossy(&out.stdout).contains(token));
    let binding = &report["request"]["observations"]["preparation"];
    assert_eq!(binding["environment"]["representation"], "sha256");
    assert_eq!(
        binding["environment"]["inherited"]["GH_TOKEN"],
        sha256(token.as_bytes())
    );
    json_file(
        &h.root,
        ".chrono-harness/state/event.json",
        &json!({"ref":"refs/heads/dev","before":h.base,"after":h.candidate,"created":false,"deleted":false}),
    );
    let out = h
        .command(&["check", "--unit", "alpha"])
        .env("GH_TOKEN", token)
        .env(prepared::SOURCE, "ci")
        .env("GITHUB_EVENT_NAME", "push")
        .env(
            "GITHUB_EVENT_PATH",
            h.root.join(".chrono-harness/state/event.json"),
        )
        .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!String::from_utf8_lossy(&out.stdout).contains(token));
    for entry in fs::read_dir(h.root.join(".chrono-harness/state/preparation")).unwrap() {
        let bytes = fs::read(entry.unwrap().path()).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(token));
    }
    let facts =
        chrono_harness::facts::Reader::for_config(&h.root, ".chrono-harness/config.json").unwrap();
    assert_eq!(
        facts.observation()["environment"]["omitted_credentials"],
        json!(["GH_TOKEN"])
    );
}

#[test]
fn generated_native_collect_gathers_inside_short_check_and_repeats_without_business() {
    let mut h = ShortHost::new();
    h.modify(".chrono-harness/config.json", |c| {
        c["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .push(json!("GH_TOKEN"));
        c["environment"]["credential_environment"] = json!(["GH_TOKEN"]);
    });
    h.modify(".chrono-harness/ci/units.json", |c| {
        c["gather"]["program"] = json!(".chrono-harness/bin/mock-gh");
        c["gather"]["wait_seconds"] = json!(1);
        c["gather"]["poll_seconds"] = json!(1);
    });
    install_mock_transport(&h.root, "success", &h.candidate);
    let event = h.root.join(".chrono-harness/state/event.json");
    json_file(
        &h.root,
        ".chrono-harness/state/event.json",
        &json!({"ref":"refs/heads/dev","before":h.base,"after":h.candidate,"created":false,"deleted":false}),
    );
    let command = |args: &[&str]| {
        let mut c = h.command(args);
        c.env(prepared::SOURCE, "ci")
            .env("GITHUB_EVENT_NAME", "push")
            .env("GITHUB_EVENT_PATH", &event)
            .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
            .env("GITHUB_REPOSITORY", "owner/host")
            .env("GH_TOKEN", "fixture-native-secret");
        c
    };
    for unit in ["alpha", "beta"] {
        let out = command(&["check", "--unit", unit]).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let target = h
            .root
            .join(format!(".chrono-harness/state/mock-artifacts/{unit}"));
        fs::create_dir_all(&target).unwrap();
        copy_artifacts(
            &h.root.join(format!(".chrono-harness/state/{unit}")),
            &target,
        );
    }
    let a = fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap();
    let b = fs::read(h.root.join(".chrono-harness/state/calls-b")).unwrap();
    let workflow = fs::read_to_string(h.root.join(".github/workflows/collection.yml")).unwrap();
    let line = workflow
        .lines()
        .find(|l| l.contains("'check' '--collect'"))
        .unwrap()
        .trim();
    for _ in 0..2 {
        let out = Command::new("/bin/bash")
            .current_dir(&h.root)
            .args(["-c", line])
            .env(prepared::SOURCE, "ci")
            .env("GITHUB_EVENT_NAME", "push")
            .env("GITHUB_EVENT_PATH", &event)
            .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
            .env("GITHUB_REPOSITORY", "owner/host")
            .env("GH_TOKEN", "fixture-native-secret")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{} {}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
                .chars()
                .take(500)
                .collect::<String>()
        );
        let r = published_report(&h.root, None, &out);
        assert_eq!(r["response"]["evidence"]["executed"], json!([]));
        assert!(!String::from_utf8_lossy(&out.stdout).contains("fixture-native-secret"));
    }
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap(),
        a
    );
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/state/calls-b")).unwrap(),
        b
    );
    h.modify(".chrono-harness/config.json", |c| {
        c["protocol"]["timeout_seconds"] = json!(5)
    });
    h.modify(".chrono-harness/ci/units.json", |c| {
        c["gather"]["wait_seconds"] = json!(6)
    });
    json_file(
        &h.root,
        ".chrono-harness/state/event.json",
        &json!({"ref":"refs/heads/dev","before":h.base,"after":h.candidate,"created":false,"deleted":false}),
    );
    let rejected = h
        .command(&["check", "--collect"])
        .env(prepared::SOURCE, "ci")
        .env("GITHUB_EVENT_NAME", "push")
        .env("GITHUB_EVENT_PATH", &event)
        .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
        .env("GITHUB_REPOSITORY", "owner/host")
        .output()
        .unwrap();
    assert_eq!(rejected.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("registered acquisition timeout"),
        "{}",
        String::from_utf8_lossy(&rejected.stderr)
    );
}

#[test]
fn local_collection_only_requires_delta_units_and_ignores_unrelated_absent_or_stale_reports() {
    let mut h = ShortHost::new();
    h.run(&["check", "--unit", "beta"], 0);
    let base_b = git(&h.root, &["show", &format!("{}:b.txt", h.base)]);
    fs::write(h.root.join("b.txt"), base_b).unwrap();
    h.revise();
    h.run(&["check", "--unit", "alpha"], 0);
    let calls = fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap();
    for absent in [false, true] {
        if absent {
            fs::remove_file(h.root.join(".chrono-harness/state/beta/check.json")).unwrap();
        }
        let r = h.run(&["check", "--collect"], 0);
        assert_eq!(
            r["response"]["evidence"]["required_units"],
            json!(["alpha"])
        );
        assert_eq!(
            r["response"]["evidence"]["reports"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(r["response"]["evidence"]["executed"], json!([]));
    }
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap(),
        calls
    );
    fs::remove_file(h.root.join(".chrono-harness/state/alpha/check.json")).unwrap();
    h.run(&["check", "--collect"], 2);
}

#[test]
fn local_collection_empty_delta_needs_no_reports_or_business() {
    let h = ShortHost::new();
    git(&h.root, &["push", "-q", "origin", "dev"]);
    let r = h.run(&["check", "--collect"], 0);
    assert_eq!(r["response"]["evidence"]["required_units"], json!([]));
    assert_eq!(r["response"]["evidence"]["reports"], json!([]));
    assert_eq!(r["response"]["evidence"]["executed"], json!([]));
    assert!(!h.root.join(".chrono-harness/state/calls-a").exists());
    assert!(!h.root.join(".chrono-harness/state/calls-b").exists());
}

#[test]
fn local_collection_shared_obligations_require_each_assigned_unit() {
    let mut h = ShortHost::new();
    h.modify(".chrono-harness/FILEMAP.json", |fm| {
        fm["execution_plans"]["test:b"]["operations"] = json!(["test.a", "test.b"]);
        for f in fm["files"].as_array_mut().unwrap() {
            if f["path"] == "a.txt" {
                f["edges"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"kind":"test-execution","to":"test:b"}));
            }
        }
    });
    h.modify(".chrono-harness/ci/check.json", |c| {
        c["policy"]["shared_operations"] = json!({"test.a":["alpha","beta"]})
    });
    git(&h.root, &["push", "-q", "origin", "dev"]);
    fs::write(h.root.join("a.txt"), "shared delta").unwrap();
    h.revise();
    h.run(&["check", "--unit", "alpha"], 0);
    h.run(&["check", "--collect"], 2);
    h.run(&["check", "--unit", "beta"], 0);
    let calls = fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap();
    let r = h.run(&["check", "--collect"], 0);
    assert_eq!(
        r["response"]["evidence"]["required_units"],
        json!(["alpha", "beta"])
    );
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap(),
        calls
    );
}

fn copy_artifacts(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for item in fs::read_dir(source).unwrap() {
        let item = item.unwrap();
        if item.file_type().unwrap().is_dir() {
            copy_artifacts(&item.path(), &target.join(item.file_name()));
        } else {
            fs::copy(item.path(), target.join(item.file_name())).unwrap();
        }
    }
}

#[test]
fn declared_ssh_agent_input_reaches_actual_git_owner_with_absence_and_identity() {
    use std::os::unix::fs::PermissionsExt;
    let adopted: Value = serde_json::from_slice(
        &fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.chrono-harness/config.json"))
            .unwrap(),
    )
    .unwrap();
    assert!(
        adopted["environment"]["inherit"]
            .as_array()
            .unwrap()
            .contains(&json!("SSH_AUTH_SOCK"))
    );
    let mut h = ShortHost::new();
    let real_git = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let wrapper = h.root.join("declared-git.sh");
    fs::write(&wrapper,format!("#!/bin/sh\nfor arg in \"$@\"; do\nif [ \"$arg\" = fetch ]; then\n  [ -z \"${{UNREGISTERED_TRANSPORT_INPUT+x}}\" ] || exit 71\n  printf '%s' \"${{SSH_AUTH_SOCK-ABSENT}}\" > .chrono-harness/state/git-agent-observed\nfi\ndone\nexec '{}' \"$@\"\n", real_git.display())).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let hash = sha256(&fs::read(&wrapper).unwrap());
    h.modify(".chrono-harness/FILEMAP.json",|fm|fm["files"].as_array_mut().unwrap().push(json!({"path":"declared-git.sh","owner":"host","surface":"product","cost":"unmeasured","edges":[]})));
    h.modify(".chrono-harness/config.json", |c| {
        c["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .push(json!("SSH_AUTH_SOCK"));
        c["tools"][0]["program"] = json!(wrapper);
        c["environment"]["inputs"][0]["location"] = json!(wrapper);
        c["environment"]["inputs"][0]["sha256"] = json!(hash);
    });
    h.modify(".chrono-harness/worktree.json", |c| {
        c["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .push(json!("SSH_AUTH_SOCK"));
        c["git"]["program"] = json!(wrapper);
        c["git"]["sha256"] = json!(hash);
    });
    for value in [None, Some("/declared fixture agent/socket λ")] {
        let mut command = h.command(&["check"]);
        command
            .env_remove("SSH_AUTH_SOCK")
            .env("UNREGISTERED_TRANSPORT_INPUT", "must be cleared");
        if let Some(v) = value {
            command.env("SSH_AUTH_SOCK", v);
        }
        let out = command.output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let r = published_report(&h.root, None, &out);
        assert_eq!(
            r["request"]["observations"]["preparation"]["environment"]["inherited"]["SSH_AUTH_SOCK"],
            value
                .map(|v| json!(sha256(v.as_bytes())))
                .unwrap_or(Value::Null)
        );
        assert_eq!(
            fs::read_to_string(h.root.join(".chrono-harness/state/git-agent-observed")).unwrap(),
            value.unwrap_or("ABSENT")
        );
        let path = r["request"]["observations"]["preparation"]["result"]["evidence"]["report_path"]
            .as_str()
            .unwrap();
        let report: Value = serde_json::from_slice(&fs::read(h.root.join(path)).unwrap()).unwrap();
        let fetch = report["processes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["argv"].as_array().unwrap().contains(&json!("fetch")))
            .unwrap();
        assert_eq!(
            fetch["process"]["environment"]["SSH_AUTH_SOCK"],
            value.map(|v| json!(v)).unwrap_or(Value::Null)
        );
    }
}

#[test]
fn narrow_spaced_native_uploads_close_original_evidence_on_a_separate_consumer() {
    let mut h = ShortHost::new();
    let prefix = ".chrono-harness/state/custom evidence λ/";
    h.modify(".chrono-harness/ci/units.json", |c| {
        c["collection"]["artifact_directory"] = json!(format!("{prefix}collection/"));
        c["collection"]["context_path"] = json!(format!("{prefix}collection/context.json"));
        for unit in ["alpha", "beta"] {
            c["units"][unit]["artifact_directory"] = json!(format!("{prefix}{unit}/"));
            c["units"][unit]["context_path"] = json!(format!("{prefix}{unit}/context.json"));
        }
        c["gather"]["manifest_path"] = json!(format!("{prefix}collection/manifest.json"));
        c["gather"]["report_path"] = json!(format!("{prefix}collection/gather.json"));
        c["gather"]["download_directory"] = json!(format!("{prefix}collection/downloads/"));
        c["gather"]["program"] = json!(".chrono-harness/bin/mock-gh");
        c["gather"]["wait_seconds"] = json!(1);
        c["gather"]["poll_seconds"] = json!(1);
    });
    h.modify(".chrono-harness/ci/check.json", |c| {
        c["report_path"] = json!(format!("{prefix}collection/check.json"));
        for unit in ["alpha", "beta"] {
            c["policy"]["units"][unit]["report_path"] = json!(format!("{prefix}{unit}/check.json"));
        }
    });
    h.modify(".chrono-harness/worktree.json", |c| {
        c["check_inputs"]["collection_manifest"] =
            json!(format!("{prefix}collection/manifest.json"))
    });
    generate(&h.root, ".chrono-harness/ci/units.json", false).unwrap();
    h.revise();
    git(&h.root, &["push", "-q", "origin", "dev"]);
    h.base = h.candidate.clone();
    fs::write(h.root.join("a.txt"), "portable a").unwrap();
    fs::write(h.root.join("b.txt"), "portable b").unwrap();
    h.revise();
    let payload = json!({"ref":"refs/heads/dev","before":h.base,"after":h.candidate,"created":false,"deleted":false});
    json_file(&h.root, ".chrono-harness/state/event.json", &payload);
    for unit in ["alpha", "beta"] {
        let out = h
            .command(&["check", "--unit", unit])
            .env(prepared::SOURCE, "ci")
            .env("GITHUB_EVENT_NAME", "push")
            .env(
                "GITHUB_EVENT_PATH",
                h.root.join(".chrono-harness/state/event.json"),
            )
            .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let report = published_report(&h.root, Some(unit), &out);
        assert!(
            report["retained_report"]
                .as_str()
                .unwrap()
                .starts_with(&format!("{prefix}{unit}/"))
        );
        let binding = &report["request"]["observations"]["preparation"];
        prepared::validate_portable_binding(&h.root, binding, None).unwrap();
        assert!(binding["receipts"][0].get("process").is_none());
        println!(
            "MEASURE native-{unit} stdout_bytes={} report_bytes={} receipts_bytes={}",
            out.stdout.len(),
            fs::metadata(h.root.join(report["retained_report"].as_str().unwrap()))
                .unwrap()
                .len(),
            fs::metadata(
                h.root
                    .join(binding["receipts"][0]["path"].as_str().unwrap())
            )
            .unwrap()
            .len()
        );
    }
    let consumer = h._dir.path().join("separate artifact consumer λ");
    fs::create_dir(&consumer).unwrap();
    git(&consumer, &["clone", "-q", h.root.to_str().unwrap(), "."]);
    install(&consumer);
    git(&consumer, &["config", "user.name", "Fixture"]);
    git(
        &consumer,
        &["config", "user.email", "fixture@example.invalid"],
    );
    for unit in ["alpha", "beta"] {
        copy_artifacts(
            &h.root.join(format!("{prefix}{unit}")),
            &consumer.join(format!(".chrono-harness/state/mock-artifacts/{unit}")),
        );
    }
    // Original absolute paths are unavailable. Only the selected uploaded roots are delivered.
    fs::rename(&h.root, h._dir.path().join("unavailable original checkout")).unwrap();
    install_mock_transport(&consumer, "success", &h.candidate);
    json_file(&consumer, ".chrono-harness/state/event.json", &payload);
    let command = || {
        let mut c = Command::new(consumer.join(".chrono-harness/bin/chrono-harness"));
        c.current_dir(&consumer)
            .args(["check", "--collect"])
            .env(prepared::SOURCE, "ci")
            .env("GITHUB_EVENT_NAME", "push")
            .env(
                "GITHUB_EVENT_PATH",
                consumer.join(".chrono-harness/state/event.json"),
            )
            .env("CHRONO_WORKFLOW_REVISION", &h.candidate)
            .env("GITHUB_REPOSITORY", "owner/host");
        c
    };
    let out = command().output().unwrap();
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
            .chars()
            .take(300)
            .collect::<String>()
    );
    let report = published_report(&consumer, None, &out);
    assert_eq!(report["response"]["evidence"]["executed"], json!([]));
    prepared::validate_portable_binding(
        &consumer,
        &report["request"]["observations"]["preparation"],
        None,
    )
    .unwrap();
    println!(
        "MEASURE native-collect stdout_bytes={} report_bytes={}",
        out.stdout.len(),
        fs::metadata(consumer.join(report["retained_report"].as_str().unwrap()))
            .unwrap()
            .len()
    );
    // Admission consumes the exact manifest produced for this invocation.
    let manifest = consumer.join(report["request"]["scope"]["manifest"].as_str().unwrap());
    let manifest_bytes = fs::read(&manifest).unwrap();
    fs::write(
        &manifest,
        br#"{"schema":"chrono-ci-collection/v1","reports":[]}"#,
    )
    .unwrap();
    {
        use std::io::Write;
        let mut judge = Command::new(consumer.join(".chrono-harness/bin/chrono-judge-ci"))
            .current_dir(&consumer)
            .env(prepared::SOURCE, "ci")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        judge
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&report["request"]).unwrap())
            .unwrap();
        let bad = judge.wait_with_output().unwrap();
        assert_eq!(bad.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&bad.stdout)
                .contains("prepared collection manifest differs from original producer")
        );
    }
    fs::write(manifest, manifest_bytes).unwrap();
    // Missing original payload and altered acquisition must fail the real collector, even with the check/context reports present.
    let path = format!(".chrono-harness/state/mock-artifacts/alpha/check.json");
    let original: Value = serde_json::from_slice(&fs::read(consumer.join(path)).unwrap()).unwrap();
    let binding = &original["request"]["observations"]["preparation"];
    let receipt = binding["receipts"][0]["path"]
        .as_str()
        .unwrap()
        .strip_prefix(&format!("{prefix}alpha/"))
        .unwrap();
    let receipt = consumer.join(format!(
        ".chrono-harness/state/mock-artifacts/alpha/{receipt}"
    ));
    let bytes = fs::read(&receipt).unwrap();
    fs::write(&receipt, b"{}").unwrap();
    let bad = command().output().unwrap();
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stdout).contains("original evidence digest mismatch"));
    fs::write(&receipt, bytes).unwrap();
    let payload = binding["result"]["evidence"]["payload"]["path"]
        .as_str()
        .unwrap()
        .strip_prefix(&format!("{prefix}alpha/"))
        .unwrap();
    fs::remove_file(consumer.join(format!(
        ".chrono-harness/state/mock-artifacts/alpha/{payload}"
    )))
    .unwrap();
    let bad = command().output().unwrap();
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stdout).contains("missing original evidence"));
    assert!(!consumer.join(".chrono-harness/state/calls-a").exists());
    assert!(!consumer.join(".chrono-harness/state/calls-b").exists());
}

// Read the existing canonical path, then verify its immutable original. Console
// text is a human projection, not a second report protocol.
fn published_report(root: &Path, unit: Option<&str>, out: &std::process::Output) -> Value {
    let config: Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/ci/check.json")).unwrap())
            .unwrap();
    let path = unit
        .map(|id| &config["policy"]["units"][id]["report_path"])
        .unwrap_or(&config["report_path"])
        .as_str()
        .unwrap();
    let bytes = fs::read(root.join(path)).unwrap();
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    let retained = report["retained_report"].as_str().unwrap();
    assert_eq!(bytes, fs::read(root.join(retained)).unwrap());
    let console = String::from_utf8_lossy(&out.stdout);
    assert!(console.contains(retained), "{console}");
    assert!(console.starts_with("check: "), "{console}");
    assert!(
        out.stdout.len() < 16_384,
        "routine console must not scale with evidence"
    );
    report
}

#[test]
fn short_console_large_original_success_has_constant_output_and_no_extra_operations() {
    let mut h = ShortHost::new();
    // A large genuine business process receipt must stay in the original report.
    fs::write(h.root.join("a.sh"), "mkdir -p .chrono-harness/state\nprintf alpha >> .chrono-harness/state/calls-a\n/usr/bin/python3 -c 'print(\"large-original-evidence\" * 50000)'\n").unwrap();
    h.modify(".chrono-harness/FILEMAP.json", |f| {
        f["execution_plans"]["test:a"]["output_limit_bytes"] = json!(2_000_000)
    });
    let out = h.command(&["check", "--unit", "alpha"]).output().unwrap();
    let original: Value = serde_json::from_slice(
        &fs::read(h.root.join(".chrono-harness/state/alpha/check.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{} transport={} judge_failure={} results={}",
        String::from_utf8_lossy(&out.stderr),
        original["transport_failure"],
        original["judge"]["failure"],
        original["response"]["results"]
    );
    assert!(
        out.stdout.len() < 2048,
        "large original leaked into console: {} bytes",
        out.stdout.len()
    );
    let report = published_report(&h.root, Some("alpha"), &out);
    let retained = h.root.join(report["retained_report"].as_str().unwrap());
    let original = fs::read(&retained).unwrap();
    assert!(original.len() > 1_000_000);
    let process = &report["response"]["evidence"]["executed"][0]["process"];
    let bytes: Vec<u8> = serde_json::from_value(process["stdout_bytes"].clone()).unwrap();
    assert_eq!(process["stdout_sha256"], sha256(&bytes));
    assert!(String::from_utf8_lossy(&bytes).contains("large-original-evidence"));
    assert!(!String::from_utf8_lossy(&out.stdout).contains("large-original-evidence"));
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/state/calls-a")).unwrap(),
        b"alpha"
    );
    assert!(!h.root.join(".chrono-harness/state/calls-b").exists());
    println!(
        "MEASURE large-scoped stdout_bytes={} report_bytes={} report_sha256={} business_operations=1",
        out.stdout.len(),
        original.len(),
        sha256(&original)
    );
    // A subsequent check must preserve the first original byte for byte.
    h.run(&["check", "--unit", "alpha"], 0);
    assert_eq!(fs::read(retained).unwrap(), original);
}

fn script(root: &Path, path: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(root.join(path), body).unwrap();
    fs::set_permissions(root.join(path), fs::Permissions::from_mode(0o755)).unwrap();
}

fn retained_receipt(root: &Path, prefix: &str) -> (String, Vec<u8>, Value) {
    let dir = root.join(".chrono-harness/state/preparation");
    let entry = fs::read_dir(dir)
        .unwrap()
        .map(Result::unwrap)
        .find(|e| e.file_name().to_str().unwrap().starts_with(prefix))
        .unwrap();
    let path = entry
        .path()
        .strip_prefix(root)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = fs::read(entry.path()).unwrap();
    let receipt = serde_json::from_slice(&bytes).unwrap();
    assert!(path.contains(&sha256(&bytes)));
    (path, bytes, receipt)
}

#[test]
fn short_producer_failure_version_decode_and_validation_keep_originals() {
    for case in ["failed", "version", "decode", "validation"] {
        let mut h = ShortHost::new();
        let body = if case == "version" {
            "#!/bin/sh\nprintf probe >> .chrono-harness/state/producer-calls\n/usr/bin/python3 -c 'print(\"wrong-probe-version\" * 20000)'\n"
        } else {
            "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'chrono-worktree 0.1.0\\n'; exit 0; fi\nprintf acquisition >> .chrono-harness/state/producer-calls\nexec /usr/bin/python3 .chrono-harness/state/producer.py\n"
        };
        script(&h.root, ".chrono-harness/bin/fixture-producer", body);
        let code = match case {
            "failed" => "import sys\nprint('producer-stdout-original' * 20000)\nprint('E_FIXTURE_ACQUISITION: fetch failed ' + 'stderr-original' * 20000, file=sys.stderr)\nsys.exit(9)\n".to_string(),
            "decode" => "print('not-json-original' * 20000)\n".into(),
            "validation" => format!("import json,sys,hashlib\nr=sys.stdin.buffer.read(); q=json.loads(r)\nprint(json.dumps(dict(schema='chrono-check-inputs/v1',request_sha256=hashlib.sha256(r).hexdigest(),source=q['source'],profile=q['profile'],base='{}',candidate='{}',initial=False,scope=None,context=None,originals=[],evidence=dict(note='original-validation-evidence'*20000))))\n", h.base, "b".repeat(40)),
            _ => String::new(),
        };
        json_file(
            &h.root,
            ".chrono-harness/state/placeholder.json",
            &json!({}),
        );
        fs::write(h.root.join(".chrono-harness/state/producer.py"), code).unwrap();
        h.modify(".chrono-harness/config.json", |c| {
            c["tools"][1]["program"] = json!(".chrono-harness/bin/fixture-producer")
        });
        // Invalid identity must be rejected, independent of the presentation.
        if case == "validation" {
            let code = fs::read_to_string(h.root.join(".chrono-harness/state/producer.py"))
                .unwrap()
                .replace("chrono-check-inputs/v1", "wrong-input-schema");
            fs::write(h.root.join(".chrono-harness/state/producer.py"), code).unwrap();
        }
        let out = h.command(&["check"]).output().unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(out.stderr.len() < 2048, "{case}: {}", out.stderr.len());
        let stderr = String::from_utf8_lossy(&out.stderr);
        let prefix = if case == "version" {
            "producer-version-"
        } else {
            "acquisition-"
        };
        let (path, raw, receipt) = retained_receipt(&h.root, prefix);
        assert!(stderr.contains(&path), "{stderr}");
        let process = if case == "version" {
            &receipt["version"]
        } else {
            &receipt["process"]
        };
        let bytes: Vec<u8> = serde_json::from_value(process["stdout_bytes"].clone()).unwrap();
        assert_eq!(process["stdout_sha256"], sha256(&bytes));
        assert_eq!(process["stdout"], std::str::from_utf8(&bytes).unwrap());
        assert!(raw.len() > 100_000);
        if case == "failed" {
            assert!(stderr.contains("E_FIXTURE_ACQUISITION"));
            assert!(stderr.contains("exit 9"));
            assert!(stderr.contains("text omitted"));
            let bytes: Vec<u8> = serde_json::from_value(process["stderr_bytes"].clone()).unwrap();
            assert_eq!(process["stderr_sha256"], sha256(&bytes));
        } else {
            assert!(
                stderr.contains(match case {
                    "version" => "version mismatch",
                    "decode" => "decode failed",
                    _ => "validation failed",
                }),
                "{stderr}"
            );
        }
        assert_eq!(
            fs::read_to_string(h.root.join(".chrono-harness/state/producer-calls")).unwrap(),
            if case == "version" {
                "probe"
            } else {
                "acquisition"
            }
        );
        assert!(!h.root.join(".chrono-harness/state/calls-a").exists());
        println!(
            "MEASURE producer-{case} stderr_bytes={} original_bytes={}",
            out.stderr.len(),
            raw.len()
        );
    }
}

#[test]
fn short_outer_failure_keeps_original_error_and_retention_failure_is_honest() {
    let mut h = ShortHost::new();
    let long_parent = h
        ._dir
        .path()
        .join(format!("target-local-{}", "x".repeat(220)));
    fs::create_dir_all(&long_parent).unwrap();
    let long_root = long_parent.join("host");
    fs::rename(&h.root, &long_root).unwrap();
    h.root = long_root;
    h.modify(".chrono-harness/ci/check.json", |c| {
        c["report_path"] = json!(".chrono-harness/state/blocked/check.json")
    });
    fs::create_dir_all(h.root.join(".chrono-harness/state/elsewhere")).unwrap();
    std::os::unix::fs::symlink("elsewhere", h.root.join(".chrono-harness/state/blocked")).unwrap();
    let out = h.command(&["check"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let error = fs::read_dir(h.root.join(".chrono-harness/state/preparation"))
        .unwrap()
        .map(Result::unwrap)
        .find(|e| e.file_name().to_str().unwrap().starts_with("check-error-"))
        .unwrap();
    let original = fs::read_to_string(error.path()).unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let clipped: String = original.chars().take(240).collect();
    assert!(stderr.contains(&clipped), "{stderr}");
    assert!(stderr.contains("[text omitted; see original]"), "{stderr}");
    assert!(stderr.contains(&sha256(original.as_bytes())), "{stderr}");
    assert!(
        stderr.contains(
            error
                .path()
                .strip_prefix(&h.root)
                .unwrap()
                .to_str()
                .unwrap()
        )
    );
    assert!(stderr.contains("Original check error:"));
    assert!(!stderr.contains("Original report:"));
    assert!(!h.root.join(".chrono-harness/state/calls-a").exists());

    // Force the evidence owner to become unavailable after acquisition, in the
    // judge process. Publication and outer-error retention must both fail.
    fs::remove_file(h.root.join(".chrono-harness/state/blocked")).unwrap();
    script(
        &h.root,
        ".chrono-harness/bin/retention-failure",
        "#!/bin/sh\nmv .chrono-harness/state/preparation .chrono-harness/state/saved-preparation\nln -s saved-preparation .chrono-harness/state/preparation\nexec /usr/bin/python3 -c 'import json,sys; r=json.load(sys.stdin); print(json.dumps(dict(protocol=r[\"protocol\"],request_id=r[\"request_id\"],status=\"passed\",results=[dict(id=\"fixture\",status=\"passed\",cause=\"actual\",exit_code=0)],evidence=dict(actual=True))))'\n",
    );
    h.modify(".chrono-harness/ci/check.json", |c| {
        c["report_path"] = json!(".chrono-harness/state/check.json");
        c["judge"]["program"] = json!(".chrono-harness/bin/retention-failure");
    });
    let out = h.command(&["check"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("symlink"), "{stderr}");
    assert!(
        stderr.contains("Original check error retention failed:"),
        "{stderr}"
    );
    assert!(!stderr.contains("Original check error:"));
    assert!(!stderr.contains("Original report:"));
}

#[test]
fn short_prelaunch_failures_retain_actual_errors_without_business_launches() {
    let missing_program = format!(
        ".chrono-harness/bin/{}/{}/missing-tail-producer",
        "a".repeat(160),
        "b".repeat(166)
    );
    let expected_error = format!("executable not found: {missing_program}");
    assert!(expected_error.chars().count() > 240);
    let mut missing = ShortHost::new();
    missing.modify(".chrono-harness/config.json", |c| {
        c["tools"][1]["program"] = json!(missing_program)
    });
    let out = missing.command(&["check"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(out.stderr.len() < 2048, "{}", out.stderr.len());
    assert!(!missing.root.join(".chrono-harness/state/calls-a").exists());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("executable not found"), "{stderr}");
    assert!(stderr.contains("[text omitted; see original]"), "{stderr}");
    assert!(stderr.contains("Original version probe:"), "{stderr}");
    let entry = fs::read_dir(missing.root.join(".chrono-harness/state/preparation"))
        .unwrap()
        .map(Result::unwrap)
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("producer-version-")
        })
        .unwrap();
    let raw = fs::read(entry.path()).unwrap();
    assert_eq!(
        sha256(&raw),
        entry.file_name().to_string_lossy()["producer-version-".len()..].trim_end_matches(".json")
    );
    assert_eq!(String::from_utf8_lossy(&raw), expected_error);
    assert!(stderr.contains(&sha256(&raw)), "{stderr}");
    assert!(
        stderr.contains(
            entry
                .path()
                .strip_prefix(&missing.root)
                .unwrap()
                .to_str()
                .unwrap()
        ),
        "{stderr}"
    );

    let mut acquisition = ShortHost::new();
    script(
        &acquisition.root,
        ".chrono-harness/bin/mutating-producer",
        "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'chrono-worktree 0.1.0\\n'; printf '# changed after version\\n' >> \"$0\"; exit 0; fi\nexit 0\n",
    );
    acquisition.modify(".chrono-harness/config.json", |c| {
        c["tools"][1]["program"] = json!(".chrono-harness/bin/mutating-producer")
    });
    let out = acquisition.command(&["check"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        !acquisition
            .root
            .join(".chrono-harness/state/calls-a")
            .exists()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("prelaunch executable digest mismatch"),
        "{stderr}"
    );
    assert!(stderr.contains("Original acquisition:"), "{stderr}");
    let entry = fs::read_dir(acquisition.root.join(".chrono-harness/state/preparation"))
        .unwrap()
        .map(Result::unwrap)
        .find(|e| e.file_name().to_string_lossy().starts_with("acquisition-"))
        .unwrap();
    let raw = fs::read(entry.path()).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&raw),
        "prelaunch executable digest mismatch"
    );
    assert!(entry.file_name().to_string_lossy().contains(&sha256(&raw)));

    let mut setup = ShortHost::new();
    setup.modify(".chrono-harness/config.json", |c| {
        c["protocol"]["timeout_seconds"] = json!(0)
    });
    let out = setup.command(&["check"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(!setup.root.join(".chrono-harness/state/calls-a").exists());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid command/bounds"), "{stderr}");
    assert!(stderr.contains("Original version probe:"), "{stderr}");

    let mut persistence = ShortHost::new();
    let state = persistence.root.join(".chrono-harness/state");
    fs::create_dir_all(&state).unwrap();
    let preparation = state.join("preparation");
    if preparation.exists() {
        fs::remove_dir_all(&preparation).unwrap();
    }
    fs::create_dir(persistence.root.join(".chrono-harness/state/elsewhere")).unwrap();
    std::os::unix::fs::symlink("elsewhere", &preparation).unwrap();
    persistence.modify(".chrono-harness/config.json", |c| {
        c["tools"][1]["program"] = json!(missing_program)
    });
    let out = persistence.command(&["check"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(
        !persistence
            .root
            .join(".chrono-harness/state/calls-a")
            .exists()
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("retention failed"), "{stderr}");
    assert!(stderr.contains(&expected_error), "{stderr}");
    assert!(stderr.contains("missing-tail-producer"), "{stderr}");
    assert!(!stderr.contains("Original version probe:"), "{stderr}");
    assert!(!stderr.contains("Original check error:"), "{stderr}");
    assert!(!stderr.contains("Original report:"), "{stderr}");
    assert!(!stderr.contains("omitted"), "{stderr}");
    assert!(
        !persistence
            .root
            .join(".chrono-harness/state/producer-calls")
            .exists()
    );
    assert!(
        !persistence
            .root
            .join(".chrono-harness/state/calls-b")
            .exists()
    );
    assert!(
        fs::read_dir(state.join("elsewhere"))
            .unwrap()
            .next()
            .is_none()
    );
    println!(
        "MEASURE prelaunch-retention missing_program_chars={} original_error_chars={} original_error_bytes={} failed_retention_stderr_bytes={} exit={}",
        missing_program.chars().count(),
        expected_error.chars().count(),
        expected_error.len(),
        out.stderr.len(),
        out.status.code().unwrap()
    );
}

#[test]
fn short_console_failed_blocked_and_transport_diagnostics_are_bounded_and_visible() {
    for case in ["failed", "blocked", "transport"] {
        let mut h = ShortHost::new();
        let body = format!(
            r#"#!/bin/sh
printf judge >> .chrono-harness/state/judge-calls
exec /usr/bin/python3 -c 'import json,sys
r=json.load(sys.stdin)
case="{case}"
if case=="transport":
 print("original-malformed-transport"*10000)
else:
 results=[dict(id="identified-operation-"+str(i)+"x"*1000,status=case,cause="actionable-cause-"+str(i)+"z"*10000,exit_code=9 if case=="failed" else None) for i in range(30)]
 print(json.dumps(dict(protocol=r["protocol"],request_id=r["request_id"],status=case,results=results,evidence=dict(original="complete-original-evidence"))))
sys.exit(9)'
"#
        );
        script(&h.root, ".chrono-harness/bin/diagnostic-judge", &body);
        h.modify(".chrono-harness/ci/check.json", |c| {
            c["judge"]["program"] = json!(".chrono-harness/bin/diagnostic-judge")
        });
        let out = h.command(&["check", "--unit", "alpha"]).output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(if case == "transport" { 2 } else { 1 })
        );
        let report = published_report(&h.root, Some("alpha"), &out);
        let console = String::from_utf8_lossy(&out.stdout);
        if case == "transport" {
            assert!(console.contains("check: unavailable (exit 2)"));
            assert!(console.contains("transport |"));
            assert!(report["transport_failure"].is_string());
        } else {
            assert!(console.contains(&format!("check: {case} (exit 1)")));
            assert!(console.contains("identified-operation-0"));
            assert!(console.contains("actionable-cause-0"));
            assert!(console.contains("18 diagnostic rows omitted"));
            assert!(console.contains("text omitted"));
            assert_eq!(report["response"]["results"].as_array().unwrap().len(), 30);
            assert!(
                report["response"]["results"][29]["cause"]
                    .as_str()
                    .unwrap()
                    .len()
                    > 10000
            );
        }
        assert!(!console.contains("stdout_bytes"));
        assert!(!console.contains("complete-original-evidence"));
        assert_eq!(
            fs::read(h.root.join(".chrono-harness/state/judge-calls")).unwrap(),
            b"judge"
        );
        assert!(!h.root.join(".chrono-harness/state/calls-a").exists());
    }
}

#[test]
fn short_console_zero_delta_preserves_large_report_and_executes_zero_business() {
    let h = ShortHost::new();
    git(&h.root, &["push", "-q", "origin", "dev"]);
    let out = h.command(&["check"]).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report = published_report(&h.root, None, &out);
    assert_eq!(report["request"]["base"], h.candidate);
    assert_eq!(report["request"]["candidate"], h.candidate);
    assert_eq!(report["response"]["evidence"]["executed"], json!([]));
    assert!(!h.root.join(".chrono-harness/state/calls-a").exists());
    assert!(!h.root.join(".chrono-harness/state/calls-b").exists());
    assert!(out.stdout.len() < 2048);
    let bytes = fs::read(h.root.join(report["retained_report"].as_str().unwrap())).unwrap();
    println!(
        "MEASURE zero-delta-scoped stdout_bytes={} report_bytes={} report_sha256={} business_operations=0",
        out.stdout.len(),
        bytes.len(),
        sha256(&bytes)
    );
}

#[test]
fn native_short_version_failure_retains_probe_in_selected_unit_upload_directory() {
    let mut h = ShortHost::new();
    h.modify(".chrono-harness/config.json", |c| {
        c["tools"][2]["expected_version"] = json!("wrong-native-version")
    });
    let out = h
        .command(&["check", "--unit", "alpha"])
        .env(prepared::SOURCE, "ci")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("input producer version mismatch: chrono-ci"),
        "{stderr}"
    );
    let entry = fs::read_dir(h.root.join(".chrono-harness/state/alpha/preparation"))
        .unwrap()
        .map(Result::unwrap)
        .find(|e| {
            e.file_name()
                .to_str()
                .unwrap()
                .starts_with("producer-version-")
        })
        .unwrap();
    let path = entry.path();
    assert!(stderr.contains(path.strip_prefix(&h.root).unwrap().to_str().unwrap()));
    let raw = fs::read(path).unwrap();
    let receipt: Value = serde_json::from_slice(&raw).unwrap();
    let stdout: Vec<u8> =
        serde_json::from_value(receipt["version"]["stdout_bytes"].clone()).unwrap();
    assert_eq!(receipt["version"]["stdout_sha256"], sha256(&stdout));
    assert_eq!(stdout, b"chrono-ci 0.1.0\n");
    assert!(!h.root.join(".chrono-harness/state/preparation").exists());
    assert!(!h.root.join(".chrono-harness/state/calls-a").exists());
}

#[test]
fn short_console_does_not_revalidate_deep_accepted_evidence_or_change_verdict() {
    let mut h = ShortHost::new();
    script(
        &h.root,
        ".chrono-harness/bin/deep-judge",
        r#"#!/bin/sh
printf judge >> .chrono-harness/state/judge-calls
exec /usr/bin/python3 -c 'import json,sys
r=json.load(sys.stdin)
e={}
for i in range(125): e=dict(nested=e)
print(json.dumps(dict(protocol=r["protocol"],request_id=r["request_id"],status="passed",results=[dict(id="deep",status="passed",cause="actual result",exit_code=0)],evidence=e)))'
"#,
    );
    h.modify(".chrono-harness/ci/check.json", |c| {
        c["judge"]["program"] = json!(".chrono-harness/bin/deep-judge")
    });
    let out = h.command(&["check", "--unit", "alpha"]).output().unwrap();
    // The original acquisition/judge decoder accepted this evidence. Rendering
    // must not add another verdict-bearing depth limit around its report.
    let bytes = fs::read(h.root.join(".chrono-harness/state/alpha/check.json")).unwrap();
    let consumed = Command::new("/usr/bin/python3").args(["-c", "import json,sys; r=json.load(open(sys.argv[1])); assert r['response']['status']=='passed' and r['judge']['exit_code']==0 and 'transport_failure' not in r; print(r['retained_report'])", h.root.join(".chrono-harness/state/alpha/check.json").to_str().unwrap()]).output().unwrap();
    assert!(
        consumed.status.success(),
        "{}",
        String::from_utf8_lossy(&consumed.stderr)
    );
    let retained = String::from_utf8(consumed.stdout)
        .unwrap()
        .trim()
        .to_owned();
    assert_eq!(fs::read(h.root.join(&retained)).unwrap(), bytes);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("check: passed (exit 0)"));
    assert!(String::from_utf8_lossy(&out.stdout).contains(&retained));
    assert_eq!(
        fs::read(h.root.join(".chrono-harness/state/judge-calls")).unwrap(),
        b"judge"
    );
}
