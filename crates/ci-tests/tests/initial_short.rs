//! Real standalone bare checks with empty business inventory and both input owners.
use super::*;
use chrono_harness::{prepared, sha256};
use std::path::PathBuf;

struct Host {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    remote: PathBuf,
    candidate: String,
}
impl Host {
    fn new(unregistered: bool, binding: bool) -> Self {
        let temporary = tempfile::Builder::new()
            .prefix("empty initial λ ")
            .tempdir()
            .unwrap();
        let parent = fs::canonicalize(temporary.path()).unwrap();
        let root = parent.join("independent host");
        let remote = parent.join("registered target.git");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&remote).unwrap();
        git(&root, &["init", "-q", "-b", "dev"]);
        git(&remote, &["init", "--bare", "-q", "-b", "dev"]);
        git(
            &root,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        let product = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
        for (project, name) in [
            ("runner", "chrono-harness"),
            ("ci", "chrono-ci"),
            ("worktree", "chrono-worktree"),
            ("judge-registration", "chrono-judge-registration"),
            ("judge-ci", "chrono-judge-ci"),
        ] {
            fs::copy(
                product.join(format!("crates/{project}/target/debug/{name}")),
                root.join(format!(".chrono-harness/bin/{name}")),
            )
            .expect("registered production prerequisite");
        }
        let git_program = fixture_git();
        let version = Command::new(&git_program)
            .arg("--version")
            .output()
            .unwrap();
        let mut cfg: Value =
            serde_json::from_slice(&fs::read(product.join(".chrono-harness/config.json")).unwrap())
                .unwrap();
        cfg["tools"] = json!([
            {"id":"git","program":git_program,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()},
            {"id":"chrono-worktree","program":".chrono-harness/bin/chrono-worktree","resolution":"PATH-once","version_argv":["--version"],"expected_version":"chrono-worktree 0.1.0"},
            {"id":"chrono-ci","program":".chrono-harness/bin/chrono-ci","resolution":"PATH-once","version_argv":["--version"],"expected_version":"chrono-ci 0.1.0"}
        ]);
        cfg["facts_git"] = json!({"tool":"git","input":"git-bytes"});
        cfg["canonical_check"] = json!({"operation":"validate.delta","argv":[".chrono-harness/bin/chrono-harness","check"],"profile":".chrono-harness/ci/check.json","inputs":{
            "local":{"operation":"prepare.local","tool":"chrono-worktree","argv":["check-inputs","--config",".chrono-harness/worktree.json"]},
            "ci":{"operation":"prepare.ci","tool":"chrono-ci","argv":["check-inputs","--config",".chrono-harness/ci/github.json","--event-env","GITHUB_EVENT_NAME","--payload-env","GITHUB_EVENT_PATH","--revision-env","CHRONO_WORKFLOW_REVISION"]}
        }});
        if binding {
            cfg["canonical_check"]["initial_profile"] =
                json!(".chrono-harness/ci/root inventory.json");
        }
        cfg["semantic_fields"] = json!([]);
        cfg["environment"] = json!({"inherit":["PATH","CHRONO_CHECK_SOURCE","GITHUB_EVENT_NAME","GITHUB_EVENT_PATH","CHRONO_WORKFLOW_REVISION"],"values":{"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"},"inputs":[{"id":"git-bytes","location":git_program,"presence":"present","sha256":sha256(&fs::read(&git_program).unwrap())}]});
        cfg["protocol"]["timeout_seconds"] = json!(30);
        cfg["artifacts"] = json!([
            {"path":".chrono-harness/bin/","owner":"repository","kind":"executable","tracked":false},
            {"path":".chrono-harness/state/","owner":"repository","kind":"evidence","tracked":false}
        ]);
        write(&root.join(".chrono-harness/config.json"), &cfg);
        let mut workflow: Value = serde_json::from_slice(
            &fs::read(product.join(".chrono-harness/workflow.json")).unwrap(),
        )
        .unwrap();
        for name in [
            "stability",
            "retirements",
            "migrations",
            "historical_profiles",
        ] {
            workflow[name] = json!([]);
        }
        workflow["integration"]["tests"] = json!([]);
        write(&root.join(".chrono-harness/workflow.json"), &workflow);
        write(
            &root.join(".chrono-harness/projects.json"),
            &json!({"schema_version":1,"status":"proposed","owners":["repository"],"projects":[],"scripts":[]}),
        );
        let judge_hash =
            sha256(&fs::read(root.join(".chrono-harness/bin/chrono-judge-registration")).unwrap());
        let mut provider = source();
        provider["schema"] = json!("chrono-github-ci/v4");
        provider["facts_config"] = json!(".chrono-harness/config.json");
        provider["initial_inventory"]["profile"]["host_config"] =
            json!(".chrono-harness/config.json");
        provider["initial_inventory"]["profile"]["judges"][0]["executable"] =
            json!(".chrono-harness/bin/chrono-judge-registration");
        provider["initial_inventory"]["profile"]["judges"][0]["sha256"] = json!(judge_hash);
        provider["artifact_directory"] = json!(".chrono-harness/state/native/");
        provider["context_path"] = json!(".chrono-harness/state/native/context.json");
        write(&root.join(".chrono-harness/ci/github.json"), &provider);
        let mut judge = provider["initial_inventory"]["profile"]["judges"][0].clone();
        judge["selector"] = json!("every-delta");
        judge["modes"] = json!(["evaluate"]);
        judge["argv"] = json!(["--protocol", "chrono-judge/v1"]);
        write(
            &root.join(".chrono-harness/judges.json"),
            &json!({"schema_version":1,"status":"proposed","migration_validator":"inventory","judges":[judge]}),
        );
        write(
            &root.join(".chrono-harness/worktree.json"),
            &json!({"schema":"chrono-worktree-config/v2","host_config":".chrono-harness/config.json","remote":"origin","git":{"program":git_program,"expected_version":null,"sha256":sha256(&fs::read(&git_program).unwrap())},"environment":{"inherit":["PATH"],"values":{"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"}},"timeout_seconds":30,"output_limit_bytes":1048576,"report_directory":".chrono-harness/state/worktrees/"}),
        );
        let files: Vec<_> = [".gitignore","README.md",".chrono-harness/config.json",".chrono-harness/FILEMAP.json",".chrono-harness/projects.json",".chrono-harness/judges.json",".chrono-harness/workflow.json",".chrono-harness/worktree.json",".chrono-harness/ci/check.json",".chrono-harness/ci/github.json",".chrono-harness/ci/root inventory.json",".github/workflows/chrono-ci.yml"]
            .into_iter().map(|path|json!({"path":path,"owner":"repository","surface":"documentation","cost":"unknown","edges":[]})).collect();
        write(
            &root.join(".chrono-harness/FILEMAP.json"),
            &json!({"schema_version":2,"status":"proposed","files":files,"project_edges":[],"test_costs":[],"execution_plans":{},"cost_models":{"unknown":{"cpu_ms":null,"wall_ms":null,"peak_rss_bytes":null,"io_bytes":null,"basis":"fixture unknown"}}}),
        );
        write(
            &root.join(".chrono-harness/ci/check.json"),
            &json!({"schema":"chrono-ci-check/v2","judge":{"program":".chrono-harness/bin/chrono-judge-ci","args":[],"timeout_seconds":30,"output_limit_bytes":8388608},"report_path":".chrono-harness/state/native/check.json","policy":{"filemap":".chrono-harness/FILEMAP.json","projects":".chrono-harness/projects.json","registration_config":".chrono-harness/config.json","facts_config":".chrono-harness/config.json","tools":{},"artifacts":[".chrono-harness/bin/",".chrono-harness/state/"],"required_inputs":[".chrono-harness/ci/check.json",".chrono-harness/ci/github.json",".chrono-harness/ci/root inventory.json",".chrono-harness/worktree.json",".github/workflows/chrono-ci.yml"],"adoption_base":null,"operation_timeout_seconds":30,"operation_output_limit_bytes":1048576,"environment":{}}}),
        );
        fs::write(
            root.join(".gitignore"),
            ".chrono-harness/state/\n.chrono-harness/bin/\n",
        )
        .unwrap();
        fs::write(root.join("README.md"), "initial host\n").unwrap();
        if unregistered {
            fs::write(
                root.join("unregistered.txt"),
                "initial inventory must fail\n",
            )
            .unwrap();
        }
        generate(&root, ".chrono-harness/ci/github.json", false).unwrap();
        let candidate = commit(&root);
        Self {
            _temporary: temporary,
            root,
            remote,
            candidate,
        }
    }
    fn command(&self) -> Command {
        let mut c = Command::new(self.root.join(".chrono-harness/bin/chrono-harness"));
        c.current_dir(&self.root)
            .arg("check")
            .env_remove(prepared::SOURCE);
        c
    }
    fn native(&self, payload: Value) -> std::process::Output {
        let payload_path = self.root.join(".chrono-harness/state/event.json");
        write(&payload_path, &payload);
        self.command()
            .env(prepared::SOURCE, "ci")
            .env("GITHUB_EVENT_NAME", "push")
            .env("GITHUB_EVENT_PATH", payload_path)
            .env("CHRONO_WORKFLOW_REVISION", &self.candidate)
            .output()
            .unwrap()
    }
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
            "fixture",
        ],
    );
    git(root, &["rev-parse", "HEAD"])
}
fn report(root: &Path, path: &str, out: &std::process::Output, exit: i32) -> Value {
    assert_eq!(
        out.status.code(),
        Some(exit),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let raw = fs::read(root.join(path)).unwrap();
    let r: Value = serde_json::from_slice(&raw).unwrap();
    let console = String::from_utf8_lossy(&out.stdout);
    let original = r["retained_report"].as_str().unwrap_or(path);
    assert!(console.contains(original), "{console}");
    assert!(out.stdout.len() < 16384);
    assert_eq!(fs::read(root.join(original)).unwrap(), raw);
    r
}
#[test]
fn genuine_root_bare_local_and_generated_native_preserve_inventory_and_originals() {
    for unregistered in [false, true] {
        let h = Host::new(unregistered, true);
        let local = report(
            &h.root,
            ".chrono-harness/state/initial-report.json",
            &h.command().output().unwrap(),
            if unregistered { 1 } else { 0 },
        );
        let native=report(&h.root,".chrono-harness/state/native/initial-report.json",&h.native(json!({"ref":"refs/heads/dev","before":"0".repeat(40),"after":h.candidate,"created":true})),if unregistered {1}else{0});
        for r in [&local, &native] {
            assert_eq!(r["scope"], "initial-inventory");
            assert_eq!(r["governance"], "not-evaluated");
            assert!(r["base"].is_null() && r["delta"].is_null());
            assert_eq!(r["candidate"], h.candidate);
            assert_eq!(r["entry"]["argv"].as_array().unwrap().len(), 2);
            let binding = &r["preparation"];
            assert_eq!(
                binding["result"]["profile"],
                ".chrono-harness/ci/root inventory.json"
            );
            assert_eq!(
                binding["request"]["initial_profile"]["sha256"],
                r["profile_sha256"]
            );
            prepared::validate_portable_binding(&h.root, binding, None).unwrap();
            let receipt: Value = serde_json::from_slice(
                &fs::read(
                    h.root
                        .join(binding["receipts"][0]["path"].as_str().unwrap()),
                )
                .unwrap(),
            )
            .unwrap();
            let original_request: prepared::InputRequest =
                serde_json::from_value(receipt["request"].clone()).unwrap();
            assert_eq!(
                receipt["process"]["stdin_sha256"],
                sha256(&serde_json::to_vec(&original_request).unwrap())
            );
            assert_eq!(
                serde_json::from_slice::<Value>(
                    &serde_json::from_value::<Vec<u8>>(receipt["process"]["stdout_bytes"].clone())
                        .unwrap()
                )
                .unwrap(),
                binding["result"]
            );
        }
        let producer: Value = serde_json::from_slice(
            &fs::read(
                h.root.join(
                    local["preparation"]["result"]["evidence"]["report_path"]
                        .as_str()
                        .unwrap(),
                ),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(producer["parents"].as_array().unwrap().is_empty());
        assert!(producer.get("fetch_ref").is_none());
        assert!(
            !h.remote.join("refs/heads/dev").exists(),
            "root check must not need a remote baseline"
        );
        let missing = local["preparation"]["result"]["originals"][0]["path"]
            .as_str()
            .unwrap();
        fs::remove_file(h.root.join(missing)).unwrap();
        assert!(prepared::validate_portable_binding(&h.root, &local["preparation"], None).is_err());
    }
}
#[test]
fn nonroot_bare_local_and_native_use_delta_even_with_initial_inventory_registered() {
    let mut h = Host::new(false, true);
    let base = h.candidate.clone();
    git(&h.root, &["push", "-q", "origin", "dev"]);
    fs::write(h.root.join("README.md"), "ordinary DELTA\n").unwrap();
    h.candidate = commit(&h.root);
    for source in ["local", "ci"] {
        let out = if source == "local" {
            h.command().output().unwrap()
        } else {
            h.native(
                json!({"ref":"refs/heads/dev","before":base,"after":h.candidate,"created":false}),
            )
        };
        let r = report(&h.root, ".chrono-harness/state/native/check.json", &out, 0);
        assert_eq!(r["request"]["base"], base);
        assert_eq!(r["request"]["candidate"], h.candidate);
        assert_eq!(r["request"]["initial"], false);
        assert_eq!(
            r["request"]["observations"]["preparation"]["request"]["source"],
            source
        );
        assert_eq!(
            r["request"]["observations"]["preparation"]["result"]["profile"],
            ".chrono-harness/ci/check.json"
        );
        assert!(
            !h.root
                .join(".chrono-harness/state/initial-report.json")
                .exists()
        );
    }
}
#[test]
fn absent_or_incorrect_initial_binding_and_shallow_parented_creation_fail() {
    let h = Host::new(false, false);
    let out = h.command().output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("initial profile binding missing"));
    assert!(
        !h.root
            .join(".chrono-harness/state/initial-report.json")
            .exists()
    );
    let native = h.native(
        json!({"ref":"refs/heads/dev","before":"0".repeat(40),"after":h.candidate,"created":true}),
    );
    assert_eq!(native.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&native.stderr).contains("native initial profile binding mismatch")
    );
    let mut h = Host::new(false, true);
    let parent = h.candidate.clone();
    fs::write(h.root.join("README.md"), "parented\n").unwrap();
    h.candidate = commit(&h.root);
    fs::write(h.root.join(".git/shallow"), format!("{}\n", h.candidate)).unwrap();
    assert_eq!(
        git(&h.root, &["rev-list", "--parents", "-1", "HEAD"]),
        h.candidate
    );
    assert_eq!(
        chrono_harness::facts::parents(&h.root, &h.candidate).unwrap(),
        vec![parent]
    );
    let local = h.command().output().unwrap();
    assert_eq!(local.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&local.stderr).contains("Original acquisition:"));
    assert!(
        !h.root
            .join(".chrono-harness/state/initial-report.json")
            .exists()
    );
    let native = h.native(
        json!({"ref":"refs/heads/dev","before":"0".repeat(40),"after":h.candidate,"created":true}),
    );
    assert_eq!(native.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&native.stderr).contains("candidate has parents"));
}
