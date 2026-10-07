use super::*;
use chrono_harness::prepared::{self, InputRequest, PreparedCheck, Selection};

pub(super) fn bind(h: &Host) {
    let git_bin = fixture_git();
    let version = Command::new(&git_bin).arg("--version").output().unwrap();
    let mut cfg = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    cfg["schema_version"] = value!(4);
    cfg["facts_git"] = value!({"tool":"git","input":"git-bytes"});
    cfg["canonical_check"] = value!({"operation":"validate.delta","argv":[".chrono-harness/bin/chrono-harness","check"],"profile":CONFIG,"inputs":{"local":{"operation":"prepare.local","tool":"chrono-worktree","argv":["check-inputs","--config",POLICY]}}});
    cfg["tools"] = value!([{"id":"git","program":git_bin,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()},{"id":"chrono-worktree","program":".chrono-harness/bin/chrono-worktree","resolution":"PATH-once","version_argv":["--version"],"expected_version":"chrono-worktree 0.1.0"}]);
    cfg["environment"]["inherit"] = value!(["PATH", "CHRONO_CHECK_SOURCE"]);
    cfg["environment"]["inputs"] = value!([{"id":"git-bytes","location":git_bin,"presence":"present","sha256":sha256(&fs::read(&git_bin).unwrap())}]);
    cfg["protocol"]["stdout_limit_bytes"] = value!(67108864);
    fs::write(
        h.root.join(CONFIG),
        serde_json::to_vec_pretty(&cfg).unwrap(),
    )
    .unwrap();
    let mut judges = json(&fs::read(h.root.join(JUDGES)).unwrap()).unwrap();
    judges["judges"] = value!([{"id":"registration","executable":".chrono-harness/bin/context-judge","version":"fixture","sha256":sha256(&fs::read(env!("CARGO_BIN_EXE_chrono-worktree-test-judge")).unwrap()),"argv":["context"],"selector":"every-delta","after":[],"modes":["evaluate"]}]);
    fs::write(h.root.join(JUDGES), serde_json::to_vec(&judges).unwrap()).unwrap();
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    fm["files"]
        .as_array_mut()
        .unwrap()
        .push(file("context-judge.rs", value!([])));
    fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    fs::write(
        h.root.join("context-judge.rs"),
        include_bytes!("support/context_judge.rs"),
    )
    .unwrap();
    h.policy(|p|{p["schema"]=value!("chrono-worktree-config/v2");p["check_inputs"]=value!({"origin_path":".chrono-harness/state/origin.json","context_path":".chrono-harness/state/local/context.json","collection_manifest":".chrono-harness/state/collection/manifest.json","roles":{"feature":"integration","integration":"integration"}});});
}
pub(super) fn install(root: &Path) {
    fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
    for (project, name) in [
        ("runner", "chrono-harness"),
        ("worktree", "chrono-worktree"),
    ] {
        install_readonly_executable(root, project, name);
    }
    install_readonly_file(
        root,
        Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-judge")),
        "context-judge",
        b"fixture\n",
    );
}
pub(super) fn install_readonly_executable(root: &Path, project: &str, name: &str) {
    let built = source().join(format!("crates/{project}/target/debug/{name}"));
    install_readonly_file(root, &built, name, format!("{name} 0.1.0\n").as_bytes());
}
pub(super) fn ready_version_program(root: &Path, program: &Path, expected: &[u8]) {
    let ready = Command::new(program)
        .current_dir(root)
        .env_clear()
        .arg("--version")
        .output()
        .unwrap();
    assert!(
        ready.status.success(),
        "fixture readiness {program:?}: {ready:?}"
    );
    assert_eq!(ready.stdout, expected, "fixture readiness {program:?}");
    assert!(
        ready.stderr.is_empty(),
        "fixture readiness {program:?}: {ready:?}"
    );
}
pub(super) fn install_readonly_file(root: &Path, built: &Path, name: &str, version: &[u8]) {
    let installed = root.join(format!(".chrono-harness/bin/{name}"));
    assert_eq!(
        fs::symlink_metadata(&installed).unwrap_err().kind(),
        std::io::ErrorKind::NotFound,
        "fixture executable destination must be absent"
    );
    // Each host owns its executable identity. A joined child owns writable
    // descriptors, so concurrent test forks cannot inherit writable executables.
    let copied = Command::new("/bin/cp")
        .arg(built)
        .arg(&installed)
        .output()
        .unwrap();
    assert!(
        copied.status.success(),
        "fixture executable copy: {copied:?}"
    );
    ready_version_program(root, &installed, version);
}
#[test]
fn fixture_executable_identity_survives_neighbor_teardown() {
    use std::os::unix::fs::MetadataExt;

    let built = Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"));
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for directory in [first.path(), second.path()] {
        fs::create_dir_all(directory.join(".chrono-harness/bin")).unwrap();
        install_readonly_file(directory, built, "consumer", b"fixture\n");
    }
    let first_path = first.path().join(".chrono-harness/bin/consumer");
    let second_path = second.path().join(".chrono-harness/bin/consumer");
    let identity = |path: &Path| {
        let metadata = fs::metadata(path).unwrap();
        (metadata.dev(), metadata.ino())
    };
    assert_ne!(identity(&first_path), identity(built));
    assert_ne!(identity(&second_path), identity(built));
    assert_ne!(identity(&first_path), identity(&second_path));
    assert_eq!(fs::read(&first_path).unwrap(), fs::read(built).unwrap());
    assert_eq!(fs::read(&second_path).unwrap(), fs::read(built).unwrap());
    drop(first);
    let output = Command::new(&second_path)
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"fixture\n");
    assert!(output.stderr.is_empty());
}
fn inputs(root: &Path) -> (std::process::Output, Option<PreparedCheck>) {
    let head = git(root, &["rev-parse", "HEAD"]);
    let req = InputRequest {
        schema: prepared::REQUEST.into(),
        host_root: root.into(),
        source: "local".into(),
        host_config: CONFIG.into(),
        host_config_sha256: sha256(&fs::read(root.join(CONFIG)).unwrap()),
        effective_config: CONFIG.into(),
        effective_config_sha256: sha256(&fs::read(root.join(CONFIG)).unwrap()),
        profile: CONFIG.into(),
        profile_sha256: sha256(&fs::read(root.join(CONFIG)).unwrap()),
        selection: Selection::All,
        native_artifacts: None,
        prepared: None,
    };
    let mut c = Command::new(root.join(".chrono-harness/bin/chrono-worktree"));
    c.current_dir(root)
        .env(prepared::SOURCE, "local")
        .args(["check-inputs", "--config", POLICY])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = c.spawn().unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&req).unwrap())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let p = chrono_harness::decode::<PreparedCheck>(&out.stdout).ok();
    if let Some(p) = &p {
        assert_eq!(p.candidate, head);
    }
    (out, p)
}
#[test]
fn creation_publishes_exact_birth_and_full_short_check_uses_original_fork() {
    // Native workflows inherit the CI selector, while this fixture exercises
    // the local producer and validator contract. Run the whole fixture in a
    // child with an explicit local selector so no test-global environment is
    // mutated while other tests execute.
    if std::env::var(prepared::SOURCE).as_deref() == Ok("ci") {
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "check_inputs::creation_publishes_exact_birth_and_full_short_check_uses_original_fork",
                "--nocapture",
            ])
            .env(prepared::SOURCE, "local")
            .status()
            .unwrap();
        assert!(status.success(), "isolated local fixture failed: {status}");
        return;
    }
    let h = Host::new("anything/data.txt");
    bind(&h);
    let dest = h.parent.join("new independent layout");
    let (exit, birth, err) = h.invoke("integration", "short", &dest);
    assert_eq!(exit, 0, "{} {err}", birth["error"]);
    install(&dest);
    let origin = json(&fs::read(dest.join(".chrono-harness/state/origin.json")).unwrap()).unwrap();
    let birth_bytes = fs::read(dest.join(origin["birth_report"].as_str().unwrap())).unwrap();
    assert_eq!(origin["birth_sha256"], sha256(&birth_bytes));
    assert_eq!(json(&birth_bytes).unwrap(), birth);
    fs::write(dest.join("anything/data.txt"), "changed").unwrap();
    commit(&dest);
    let out = Command::new(dest.join(".chrono-harness/bin/chrono-harness"))
        .current_dir(&dest)
        .env(prepared::SOURCE, "local")
        .arg("check")
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let report = published_full_report(&dest, ".chrono-harness/state/", &out);
    let producer_path = report["preparation"]["result"]["evidence"]["report_path"]
        .as_str()
        .unwrap();
    let producer = json(&fs::read(dest.join(producer_path)).unwrap()).unwrap();
    let identity = producer["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["argv"]
                .as_array()
                .unwrap()
                .iter()
                .any(|arg| arg == "--show-toplevel")
        })
        .unwrap();
    let original: chrono_harness::ProcessResult =
        serde_json::from_value(identity["process"].clone()).unwrap();
    chrono_harness::observation::process_success(&original).unwrap();
    assert!(
        original
            .stdout
            .lines()
            .any(|line| line == report["candidate"].as_str().unwrap()),
        "local preparation must bind root and candidate HEAD in the same original observation"
    );
    assert_eq!(original.stdout.lines().next(), dest.to_str());
    let context = &report["preparation"]["result"]["context"];
    let raw: Vec<u8> = serde_json::from_value(context["raw"].clone()).unwrap();
    let ctx = json(&raw).unwrap();
    assert_eq!(ctx["fork_point"], birth["base"]);
    assert_eq!(ctx["branch_started_at"], birth["branch_started_at"]);
    assert_eq!(ctx["run_kind"], "integration");
    assert_eq!(report["entry"]["argv"].as_array().unwrap().len(), 2);
    assert_eq!(report["judges"][0]["response"]["outputs"]["context"], ctx);
    let cfg = json(&fs::read(dest.join(CONFIG)).unwrap()).unwrap();
    let actual_context = chrono_harness::wire::Context {
        path: dest.join(context["path"].as_str().unwrap()),
        sha256: context["semantic_digest"].as_str().unwrap().into(),
    };
    let validate = |actual: &chrono_harness::wire::Context| {
        prepared::validate_binding(
            &dest,
            &cfg,
            CONFIG,
            report["base"].as_str(),
            report["candidate"].as_str().unwrap(),
            false,
            &None,
            Some(actual),
            &report["preparation"],
        )
    };
    validate(&actual_context).unwrap();
    let mut drift = actual_context.clone();
    drift.sha256 = "0".repeat(64);
    assert!(
        validate(&drift)
            .unwrap_err()
            .contains("actual check context")
    );
    // Identical bytes at another path do not replace the original producer binding.
    for scope in [vec!["check", "--unit", "any"], vec!["check", "--collect"]] {
        let rejected = Command::new(dest.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&dest)
            .env(prepared::SOURCE, "local")
            .args(scope)
            .output()
            .unwrap();
        assert_eq!(rejected.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("full independent scopes"));
    }
    let other = dest.join(".chrono-harness/state/other-context.json");
    fs::write(&other, &raw).unwrap();
    drift = actual_context.clone();
    drift.path = other;
    assert!(
        validate(&drift)
            .unwrap_err()
            .contains("actual check context")
    );
    let out = Command::new(dest.join(".chrono-harness/bin/chrono-harness"))
        .current_dir(&dest)
        .env(prepared::SOURCE, "local")
        .arg("check")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    fs::write(h.root.join("anything/data.txt"), "advanced dev").unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let (o, p) = inputs(&dest);
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let p = p.unwrap();
    let ctx = json(&p.context.unwrap().raw).unwrap();
    assert_eq!(ctx["fork_point"], birth["base"]);
    assert_eq!(ctx["base"], git(&h.root, &["rev-parse", "HEAD"]));
}
#[test]
fn local_preparation_rejects_clean_replacement_after_candidate_head_observation() {
    let h = Host::new("payload");
    bind(&h);
    let dest = h.parent.join("moving-head");
    let fixture = super::automatic::native_git(
        &h,
        "local-check-head-drift",
        value!({"target":dest,"replacement":"not-armed"}),
    );
    let (exit, birth, err) = h.invoke("integration", "moving-head", &dest);
    assert_eq!(exit, 0, "{} {err}", birth["error"]);
    install(&dest);
    let candidate = git(&dest, &["rev-parse", "HEAD"]);
    fs::write(dest.join("payload"), "replacement commit").unwrap();
    commit(&dest);
    let replacement = git(&dest, &["rev-parse", "HEAD"]);
    git(&dest, &["reset", "--hard", &candidate]);
    let mut config = json(&fs::read(&fixture).unwrap()).unwrap();
    config["replacement"] = value!(replacement);
    fs::write(&fixture, serde_json::to_vec(&config).unwrap()).unwrap();
    fs::write(h.parent.join("arm-head-drift"), "armed").unwrap();

    let (out, prepared) = inputs(&dest);
    assert!(h.parent.join("head-drift-applied").is_file());
    assert_eq!(git(&dest, &["rev-parse", "HEAD"]), replacement);
    assert_eq!(
        fs::read_to_string(dest.join("payload")).unwrap(),
        "replacement commit"
    );
    assert!(
        !out.status.success(),
        "a clean replacement must not validate the original candidate"
    );
    assert!(prepared.is_none());
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("checkout differs from requested clean snapshot"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
            .chars()
            .take(1800)
            .collect::<String>()
    );
}
#[test]
fn full_missing_origin_historical_snapshot_and_changed_branch_are_named_failures() {
    let h = Host::new("any/file");
    bind(&h);
    install(&h.root);
    let (out, p) = inputs(&h.root);
    assert!(p.is_none());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("missing full check origin receipt"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
            .chars()
            .take(1800)
            .collect::<String>()
    );
    let dest = h.parent.join("destination");
    let (exit, birth, err) = h.invoke("feature", "mapped", &dest);
    assert_eq!(exit, 0, "{} {err}", birth["error"]);
    install(&dest);
    let origin_path = dest.join(".chrono-harness/state/origin.json");
    let bytes = fs::read(&origin_path).unwrap();
    let mut origin = json(&bytes).unwrap();
    origin["retained_inputs"] = value!(".chrono-harness/state/absent-history.json");
    fs::write(&origin_path, serde_json::to_vec(&origin).unwrap()).unwrap();
    let (o, p) = inputs(&dest);
    assert!(p.is_none());
    assert!(String::from_utf8_lossy(&o.stderr).contains("absent-history.json"));
    fs::write(&origin_path, bytes).unwrap();
    let original = fs::read(&origin_path).unwrap();
    let mut delivery = json(&original).unwrap();
    delivery["run_kind"] = value!("delivery");
    fs::write(&origin_path, serde_json::to_vec(&delivery).unwrap()).unwrap();
    let (o, p) = inputs(&dest);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let context: Value = serde_json::from_slice(&p.unwrap().context.unwrap().raw).unwrap();
    assert!(context["integration_evidence"].is_null());
    fs::write(&origin_path, original).unwrap();
    git(&dest, &["branch", "-m", "unmapped-renamed"]);
    let (o, p) = inputs(&dest);
    assert!(p.is_none());
    assert!(String::from_utf8_lossy(&o.stderr).contains("original birth association"));
}
#[test]
fn full_input_references_preserve_birth_and_raw_producer_data() {
    let h = Host::new("independent/input");
    bind(&h);
    h.policy(|p| {
        p["check_inputs"]["roles"]["feature"] = value!("delivery");
        p["check_inputs"]["full_inputs"] =
            value!({"retained_inputs":".chrono-harness/state/current-inputs.json"});
    });
    let dest = h.parent.join("current references");
    let (exit, birth, err) = h.invoke("feature", "references", &dest);
    assert_eq!(exit, 0, "{} {err}", birth["error"]);
    install(&dest);
    let origin_path = dest.join(".chrono-harness/state/origin.json");
    let origin_bytes = fs::read(&origin_path).unwrap();
    let origin = json(&origin_bytes).unwrap();
    let birth_path = dest.join(origin["birth_report"].as_str().unwrap());
    let birth_bytes = fs::read(&birth_path).unwrap();
    assert!(origin["retained_inputs"].is_null());
    assert!(origin["integration_evidence"].is_null());
    let (out, prepared) = inputs(&dest);
    assert_eq!(out.status.code(), Some(2));
    assert!(prepared.is_none());
    assert!(String::from_utf8_lossy(&out.stderr).contains("current-inputs.json"));
    // Opaque producer inputs deliberately carry no governance success. The
    // seven-judge consumer tests separately validate real snapshots/certificates.
    let raw = b"{\n \"producer\":\"original data\"\n}\n";
    let input_path = dest.join(".chrono-harness/state/current-inputs.json");
    fs::write(&input_path, raw).unwrap();
    let (out, prepared) = inputs(&dest);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let first = prepared.unwrap();
    let ctx = json(&first.context.unwrap().raw).unwrap();
    let retained_path = dest.join(ctx["retained_inputs"].as_str().unwrap());
    assert_eq!(fs::read(&retained_path).unwrap(), raw);
    assert!(ctx["integration_evidence"].is_null());
    assert_eq!(
        first.evidence["full_inputs"]["retained_inputs"]["source"]["sha256"],
        sha256(raw)
    );
    assert!(
        first
            .originals
            .iter()
            .any(|r| r.path == ctx["retained_inputs"].as_str().unwrap() && r.sha256 == sha256(raw))
    );
    fs::write(&input_path, b"replacement data").unwrap();
    // Observing even a failed certificate is not admission; that belongs to workflow.
    let certificate = b"{\"status\":\"failed\"}\n";
    fs::write(
        dest.join(".chrono-harness/state/integration.json"),
        certificate,
    )
    .unwrap();
    let (out, prepared) = inputs(&dest);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let second = prepared.unwrap();
    let next = json(&second.context.unwrap().raw).unwrap();
    assert_ne!(next["retained_inputs"], ctx["retained_inputs"]);
    assert_eq!(next["integration_evidence"], sha256(certificate));
    assert_eq!(fs::read(&retained_path).unwrap(), raw);
    assert_eq!(fs::read(&origin_path).unwrap(), origin_bytes);
    assert_eq!(fs::read(&birth_path).unwrap(), birth_bytes);
    fs::remove_file(input_path).unwrap();
    let (out, prepared) = inputs(&dest);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        prepared.is_none(),
        "must not fall back to an earlier retained pair"
    );
}
#[test]
fn generated_full_native_short_step_preserves_exact_context_bytes() {
    let h = Host::new("non rust/input");
    bind(&h);
    install(&h.root);
    let mut cfg = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    cfg["canonical_check"]["inputs"]["ci"] = value!({"operation":"prepare.ci","tool":"chrono-ci","argv":["check-inputs","--config",".chrono-harness/ci/full.json","--event-env","GITHUB_EVENT_NAME","--payload-env","GITHUB_EVENT_PATH","--revision-env","CHRONO_WORKFLOW_REVISION"]});
    cfg["tools"].as_array_mut().unwrap().push(value!({"id":"chrono-ci","program":".chrono-harness/bin/chrono-ci","resolution":"PATH-once","version_argv":["--version"],"expected_version":"chrono-ci 0.1.0"}));
    cfg["environment"]["inherit"]
        .as_array_mut()
        .unwrap()
        .extend([
            value!("GITHUB_EVENT_NAME"),
            value!("GITHUB_EVENT_PATH"),
            value!("CHRONO_WORKFLOW_REVISION"),
        ]);
    fs::write(h.root.join(CONFIG), serde_json::to_vec(&cfg).unwrap()).unwrap();
    let provider = value!({"schema":"chrono-github-full-ci/v2","workflow_path":".github/workflows/full.yml","name":"native full fixture","runs_on":"macos-14","checkout_action":"actions/checkout@11d5960a326750d5838078e36cf38b85af677262","upload_artifact_action":"actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02","timeout_minutes":10,"bootstrap":["/bin/sh","explicit.sh"],"runner":".chrono-harness/bin/chrono-harness","generator":".chrono-harness/bin/chrono-ci","check_config":CONFIG,"context_path":".chrono-harness/state/native evidence λ/context.json","preparation_path":".chrono-harness/state/native evidence λ/preparation.json","artifact_directory":".chrono-harness/state/native evidence λ/"});
    fs::create_dir_all(h.root.join(".chrono-harness/ci")).unwrap();
    fs::write(
        h.root.join(".chrono-harness/ci/full.json"),
        serde_json::to_vec(&provider).unwrap(),
    )
    .unwrap();
    fs::copy(
        source().join("crates/ci/target/debug/chrono-ci"),
        h.root.join(".chrono-harness/bin/chrono-ci"),
    )
    .unwrap();
    let output = Command::new(h.root.join(".chrono-harness/bin/chrono-ci"))
        .current_dir(&h.root)
        .args([
            "generate",
            "--host-root",
            ".",
            "--config",
            ".chrono-harness/ci/full.json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    for path in [".chrono-harness/ci/full.json", ".github/workflows/full.yml"] {
        fm["files"]
            .as_array_mut()
            .unwrap()
            .push(file(path, value!([])));
    }
    fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let dest = h.parent.join("native destination");
    let (exit, birth, err) = h.invoke("integration", "native", &dest);
    assert_eq!(exit, 0, "{} {err}", birth["error"]);
    install(&dest);
    fs::copy(
        source().join("crates/ci/target/debug/chrono-ci"),
        dest.join(".chrono-harness/bin/chrono-ci"),
    )
    .unwrap();
    fs::write(dest.join("non rust/input"), "delta").unwrap();
    commit(&dest);
    let (o, p) = inputs(&dest);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let context = p.unwrap().context.unwrap();
    let raw = [
        b" \n\t".as_slice(),
        context.raw.as_slice(),
        b"\n ".as_slice(),
    ]
    .concat();
    let payload = dest.join(".chrono-harness/state/payload.json");
    fs::write(
        &payload,
        serde_json::to_vec(&value!({"inputs":{"context":String::from_utf8(raw.clone()).unwrap()}}))
            .unwrap(),
    )
    .unwrap();
    let revision = git(&dest, &["rev-parse", "HEAD"]);
    let workflow = fs::read_to_string(dest.join(".github/workflows/full.yml")).unwrap();
    assert!(!workflow.contains("--context"));
    assert!(!workflow.contains("Preserve fixed full context"));
    let line = workflow
        .lines()
        .find(|l| l.contains("'check'"))
        .unwrap()
        .trim();
    let o = Command::new("/bin/bash")
        .current_dir(&dest)
        .args(["-c", line])
        .env(prepared::SOURCE, "ci")
        .env("GITHUB_EVENT_NAME", "workflow_dispatch")
        .env("GITHUB_EVENT_PATH", &payload)
        .env("CHRONO_WORKFLOW_REVISION", &revision)
        .output()
        .unwrap();
    if !o.status.success() {
        // The console points to the complete producer evidence. Preserve that
        // specific failure before the fixture's temporary host is dropped.
        for line in String::from_utf8_lossy(&o.stderr).lines() {
            if let Some(path) = line.strip_prefix("Original acquisition: ") {
                let original = chrono_harness::no_symlink_parents(&dest, path)
                    .and_then(|path| fs::read(path).map_err(|error| error.to_string()));
                match original {
                    Ok(bytes) => eprintln!(
                        "ORIGINAL_ACQUISITION path={path} sha256={} bytes={}\n{}",
                        sha256(&bytes),
                        bytes.len(),
                        String::from_utf8_lossy(&bytes)
                    ),
                    Err(error) => eprintln!("ORIGINAL_ACQUISITION path={path} read_error={error}"),
                }
            }
        }
    }
    assert!(
        o.status.success(),
        "{} {}",
        String::from_utf8_lossy(&o.stderr),
        String::from_utf8_lossy(&o.stdout)
    );
    let r = published_full_report(&dest, ".chrono-harness/state/native evidence λ/", &o);
    let observed: Vec<u8> =
        serde_json::from_value(r["preparation"]["result"]["context"]["raw"].clone()).unwrap();
    assert_eq!(observed, raw);
    assert_eq!(
        fs::read(dest.join(".chrono-harness/state/native evidence λ/context.json")).unwrap(),
        raw
    );
    let original_path = dest.join(
        r["preparation"]["result"]["context"]["path"]
            .as_str()
            .unwrap(),
    );
    let next_raw = [raw.as_slice(), b"\n\t".as_slice()].concat();
    fs::write(
        &payload,
        serde_json::to_vec(
            &value!({"inputs":{"context":String::from_utf8(next_raw.clone()).unwrap()}}),
        )
        .unwrap(),
    )
    .unwrap();
    let repeated = Command::new("/bin/bash")
        .current_dir(&dest)
        .args(["-c", line])
        .env(prepared::SOURCE, "ci")
        .env("GITHUB_EVENT_NAME", "workflow_dispatch")
        .env("GITHUB_EVENT_PATH", &payload)
        .env("CHRONO_WORKFLOW_REVISION", &revision)
        .output()
        .unwrap();
    assert!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    assert_eq!(fs::read(&original_path).unwrap(), raw);
    let next = published_full_report(&dest, ".chrono-harness/state/native evidence λ/", &repeated);
    let retained = dest.join(
        next["preparation"]["result"]["context"]["path"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(fs::read(retained).unwrap(), next_raw);
    prepared::validate_portable_binding(&dest, &r["preparation"], None).unwrap();
    prepared::validate_portable_binding(&dest, &next["preparation"], None).unwrap();
    assert!(
        r["report_path"]
            .as_str()
            .unwrap()
            .starts_with(".chrono-harness/state/native evidence λ/")
    );
    let consumer = h.parent.join("separate full evidence consumer");
    fs::create_dir(&consumer).unwrap();
    git(&consumer, &["clone", "-q", dest.to_str().unwrap(), "."]);
    fn copy(source: &Path, target: &Path) {
        fs::create_dir_all(target).unwrap();
        for e in fs::read_dir(source).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                copy(&e.path(), &target.join(e.file_name()));
            } else {
                fs::copy(e.path(), target.join(e.file_name())).unwrap();
            }
        }
    }
    copy(
        &dest.join(".chrono-harness/state/native evidence λ"),
        &consumer.join(".chrono-harness/state/native evidence λ"),
    );
    fs::rename(&dest, h.parent.join("original full paths unavailable")).unwrap();
    prepared::validate_portable_binding(&consumer, &r["preparation"], None).unwrap();
    prepared::validate_portable_binding(&consumer, &next["preparation"], None).unwrap();
    println!(
        "MEASURE full-v2 stdout_bytes={} repeated_stdout_bytes={} context_bytes={} original_receipt_bytes={} report_bytes={}",
        o.stdout.len(),
        repeated.stdout.len(),
        raw.len(),
        fs::metadata(consumer.join(r["preparation"]["receipts"][0]["path"].as_str().unwrap()))
            .unwrap()
            .len(),
        fs::metadata(consumer.join(r["report_path"].as_str().unwrap()))
            .unwrap()
            .len()
    );
    let mut mismatch = r["preparation"].clone();
    mismatch["request"]["native_artifacts"]["directory"] = value!(".chrono-harness/state/wrong/");
    assert!(
        prepared::validate_portable_binding(&consumer, &mismatch, None)
            .unwrap_err()
            .contains("contract mismatch")
    );
    let context_path = consumer.join(
        r["preparation"]["result"]["context"]["path"]
            .as_str()
            .unwrap(),
    );
    fs::remove_file(context_path).unwrap();
    assert!(
        prepared::validate_portable_binding(&consumer, &r["preparation"], None)
            .unwrap_err()
            .contains("missing original evidence")
    );
}
#[test]
fn reconstructed_destination_publishes_its_real_new_birth_association() {
    let h = Host::new("plain input/data");
    bind(&h);
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("plain input/data"), "carried business delta").unwrap();
    let candidate = commit(&h.root);
    let fresh = h.upstream(
        ".gitignore",
        b".chrono-harness/state/\n.chrono-harness/bin/\n# newer target\n",
    );
    let (exit, birth, error) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"plain input/data","action":"carry"}]),
        ),
        "reconstructed-short",
    );
    assert_eq!(exit, 0, "{} {error}", birth["error"]);
    let root = h.parent.join("reconstructed-short");
    install(&root);
    commit(&root);
    let (out, p) = inputs(&root);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let context = json(&p.unwrap().context.unwrap().raw).unwrap();
    assert_eq!(context["fork_point"], fresh);
    assert_eq!(context["branch_started_at"], birth["branch_started_at"]);
    let origin = json(&fs::read(root.join(".chrono-harness/state/origin.json")).unwrap()).unwrap();
    assert_eq!(
        json(&fs::read(root.join(origin["birth_report"].as_str().unwrap())).unwrap()).unwrap(),
        birth
    );
}

fn published_full_report(root: &Path, directory: &str, out: &std::process::Output) -> Value {
    let bytes = fs::read(root.join(format!("{directory}report.json"))).unwrap();
    let report = json(&bytes).unwrap();
    let original = report["report_path"].as_str().unwrap();
    assert_eq!(fs::read(root.join(original)).unwrap(), bytes);
    let console = String::from_utf8_lossy(&out.stdout);
    assert!(console.contains(original), "{console}");
    assert!(
        console.contains(&format!(
            "check: {} (exit {})",
            report["status"].as_str().unwrap(),
            out.status.code().unwrap()
        )),
        "{console}"
    );
    assert!(out.stdout.len() < 16_384);
    report
}

#[test]
fn full_short_console_projects_warnings_failures_transport_and_blocked_from_originals() {
    for case in ["warn", "fail", "transport", "blocked"] {
        let h = Host::new("data.txt");
        bind(&h);
        let mut judges = json(&fs::read(h.root.join(JUDGES)).unwrap()).unwrap();
        judges["judges"][0]["argv"] = value!([case]);
        if case == "blocked" {
            let mut dependent = judges["judges"][0].clone();
            dependent["id"] = value!("dependent");
            dependent["after"] = value!(["registration"]);
            judges["judges"].as_array_mut().unwrap().push(dependent);
        }
        fs::write(h.root.join(JUDGES), serde_json::to_vec(&judges).unwrap()).unwrap();
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
        let dest = h.parent.join(format!("console-{case}"));
        let (code, _, error) = h.invoke("integration", case, &dest);
        assert_eq!(code, 0, "{error}");
        install(&dest);
        let out = Command::new(dest.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&dest)
            .env(prepared::SOURCE, "local")
            .arg("check")
            .output()
            .unwrap();
        let expected_exit = match case {
            "warn" => 0,
            "fail" => 1,
            _ => 2,
        };
        assert_eq!(
            out.status.code(),
            Some(expected_exit),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let report = published_full_report(&dest, ".chrono-harness/state/", &out);
        let console = String::from_utf8_lossy(&out.stdout);
        assert!(!console.contains("FULL_ORIGINAL"));
        if case == "transport" {
            assert_eq!(report["status"], "error");
            assert!(console.contains("transport | registration"));
            let original: Vec<u8> =
                serde_json::from_value(report["judges"][0]["process"]["stdout_bytes"].clone())
                    .unwrap();
            assert!(String::from_utf8_lossy(&original).contains("complete-malformed-original"));
        } else {
            assert!(console.contains("IDENTIFIED_0"));
            assert!(console.contains("actionable-0"));
            if case != "blocked" {
                assert!(console.contains("diagnostic rows omitted"));
            }
            assert!(console.contains("text omitted"));
            assert_eq!(
                report["findings"].as_array().unwrap().len(),
                if case == "blocked" { 2 } else { 30 }
            );
            assert_eq!(
                report["judges"][0]["response"]["outputs"]["original"]
                    .as_str()
                    .unwrap()
                    .len(),
                13 * 50000
            );
        }
        assert_eq!(
            fs::read(dest.join(".chrono-harness/state/judge-calls")).unwrap(),
            b"actual"
        );
        if case == "blocked" {
            assert_eq!(report["judges"][1]["state"], "blocked");
            assert!(console.contains("blocked | dependent | registration"));
            assert_eq!(report["status"], "error");
        }
        println!(
            "MEASURE full-{case} stdout_bytes={} report_bytes={} judge_launches=1",
            out.stdout.len(),
            fs::metadata(dest.join(report["report_path"].as_str().unwrap()))
                .unwrap()
                .len()
        );
    }
}

#[test]
fn full_short_outer_git_error_retains_large_original_or_preserves_it_when_writing_fails() {
    for retain in [true, false] {
        let h = Host::new("data.txt");
        bind(&h);
        let program = env!("CARGO_BIN_EXE_chrono-worktree-test-version-probe");
        let mut cfg = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
        cfg["tools"][0]["program"] = value!(program);
        cfg["environment"]["inputs"][0]["location"] = value!(program);
        cfg["environment"]["inputs"][0]["sha256"] = value!(sha256(&fs::read(program).unwrap()));
        cfg["environment"]["values"]["CHRONO_TEST_PROBE_RETAIN"] = value!(retain.to_string());
        fs::write(h.root.join(CONFIG), serde_json::to_vec(&cfg).unwrap()).unwrap();
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
        let dest = h.parent.join(if retain {
            "outer-original"
        } else {
            "outer-retention-failure"
        });
        let (code, _, error) = h.invoke("integration", "outer", &dest);
        assert_eq!(code, 0, "{error}");
        install(&dest);
        let out = Command::new(dest.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&dest)
            .env(prepared::SOURCE, "local")
            .arg("check")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("Git facts version mismatch"),
            "{}",
            stderr.chars().take(1000).collect::<String>()
        );
        assert!(!stderr.contains("Original report:"));
        assert!(!dest.join(".chrono-harness/state/report.json").exists());
        assert!(!dest.join(".chrono-harness/state/judge-calls").exists());
        if retain {
            assert!(out.stderr.len() < 2048);
            assert!(!stderr.contains("stdout_bytes"));
            let entry = fs::read_dir(dest.join(".chrono-harness/state/preparation"))
                .unwrap()
                .map(Result::unwrap)
                .find(|e| e.file_name().to_str().unwrap().starts_with("check-error-"))
                .unwrap();
            let raw = fs::read(entry.path()).unwrap();
            assert!(stderr.contains(entry.path().strip_prefix(&dest).unwrap().to_str().unwrap()));
            assert!(entry.file_name().to_str().unwrap().contains(&sha256(&raw)));
            let original = String::from_utf8(raw).unwrap();
            let evidence =
                json(original.strip_prefix("E_GIT_FACTS: ").unwrap().as_bytes()).unwrap();
            let process = &evidence["observation"]["processes"][0];
            let bytes: Vec<u8> = serde_json::from_value(process["stdout_bytes"].clone()).unwrap();
            assert_eq!(process["stdout_sha256"], sha256(&bytes));
            assert!(String::from_utf8_lossy(&bytes).contains("original-full-version-probe"));
            assert!(original.len() > 1_000_000);
            println!(
                "MEASURE full-outer-error stderr_bytes={} original_error_bytes={}",
                out.stderr.len(),
                original.len()
            );
        } else {
            assert!(stderr.contains("Original check error retention failed:"));
            assert!(stderr.contains("original-full-version-probe"));
            assert!(out.stderr.len() > 1_000_000);
            assert!(!stderr.contains("Original check error:"));
        }
    }
}
