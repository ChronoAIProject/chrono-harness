use super::*;
use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};

impl Host {
    fn reject_result_shape(&self, result: Value, diagnostic: &str) {
        self.interrupting_git("printf invoked > \"$HOME/unexpected-plan-git\" || exit $?");
        let plan = value!({"schema":"chrono-worktree-maintenance/v1","operation":"recover-interrupted",
            "intent":{"path":".chrono-harness/state/missing.intent.json","sha256":"0".repeat(64)},
            "result":result,"head":git(&self.root,&["rev-parse","HEAD"]),
            "index_tree":git(&self.root,&["rev-parse","HEAD^{tree}"])});
        let output = self.maintenance_output("recover-interrupted", plan);
        assert_eq!(output.status.code(), Some(2), "{result}: {output:?}");
        assert!(
            !self.parent.join("unexpected-plan-git").exists(),
            "malformed result {result} must fail before invoking Git"
        );
        assert!(
            output.stdout.is_empty(),
            "parse failure must have empty stdout"
        );
        assert!(!self.root.join(".chrono-harness/state/worktrees").exists());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains(diagnostic), "{result}: {error}");
    }

    fn interrupting_git(&self, extra: &str) {
        let real = Command::new("/bin/sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap();
        let real = String::from_utf8(real.stdout)
            .unwrap()
            .trim()
            .replace('\'', "'\\''");
        let wrapper = self.parent.join("interrupting-git");
        let body = r#"if [ -f "$HOME/interrupt-stage" ] && [ "$(cat "$HOME/interrupt-stage")" = "$2 $3" ]; then
 REAL "$@" || exit $?
 printf completed > "$HOME/interruption-observed" || exit $?
 kill -KILL "$PPID"
 exit 0
fi
EXTRA
exec REAL "$@"
"#;
        fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\n{}",
                body.replace("REAL", &format!("'{real}'"))
                    .replace("EXTRA", extra)
            ),
        )
        .unwrap();
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
        self.policy(|p| p["git"]["program"] = value!(wrapper));
    }
    fn crash(&self, args: &[&str], stage: &str) {
        fs::write(self.parent.join("interrupt-stage"), stage).unwrap();
        let output = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .current_dir("/")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.signal(), Some(9), "{output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(
            fs::read(self.parent.join("interruption-observed")).unwrap(),
            b"completed"
        );
        fs::remove_file(self.parent.join("interrupt-stage")).unwrap();
    }
    fn intent(&self, operation: &str) -> (String, Value) {
        let prefix = format!("{operation}-");
        let entries: Vec<_> = fs::read_dir(self.root.join(".chrono-harness/state/worktrees"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(&prefix)
                    && p.to_str().unwrap().ends_with(".intent.json")
            })
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "interrupted operation must retain one immutable recovery intent"
        );
        let path = entries[0]
            .strip_prefix(&self.root)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let intent = json(&fs::read(&entries[0]).unwrap()).unwrap();
        assert_eq!(intent["schema"], "chrono-worktree-recovery-intent/v1");
        assert!(
            intent.get("status").is_none(),
            "intent must not invent a result"
        );
        (path, intent)
    }
    fn interruption_plan(&self, operation: &str, target: &Path) -> Value {
        let (path, intent) = self.intent(operation);
        let result = self.root.join(intent["report_path"].as_str().unwrap());
        let observation = match fs::read(&result) {
            Ok(bytes) => value!({"presence":"present","sha256":sha256(&bytes)}),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => value!({"presence":"absent"}),
            Err(e) => panic!("{e}"),
        };
        value!({"schema":"chrono-worktree-maintenance/v1","operation":"recover-interrupted",
            "intent":{"path":path,"sha256":sha256(&fs::read(self.root.join(&path)).unwrap())},
            "result":observation,"head":git(target,&["rev-parse","HEAD"]),"index_tree":git(target,&["write-tree"])})
    }
    fn crash_start(&self, target: &Path) {
        self.crash(
            &[
                "start",
                "--host-root",
                self.root.to_str().unwrap(),
                "--config",
                POLICY,
                "--kind",
                "integration",
                "--name",
                "interrupted",
                "--path",
                target.to_str().unwrap(),
            ],
            "worktree add",
        );
    }
}

#[test]
fn interrupted_recovery_rejects_payload_on_absent_result() {
    for result in [
        value!({"presence":"absent","sha256":"0".repeat(64)}),
        value!({"presence":"absent","sha256":null}),
        value!({"presence":"absent","path":"unexpected"}),
    ] {
        Host::new("payload").reject_result_shape(result, "unknown field");
    }
}

#[test]
fn interrupted_recovery_rejects_malformed_result_shapes() {
    for (result, diagnostic) in [
        (value!({}), "missing field"),
        (value!({"presence":"missing"}), "unknown variant"),
        (value!({"presence":null}), "invalid type"),
        (value!({"presence":"present"}), "missing field"),
        (value!({"presence":"present","sha256":null}), "invalid type"),
        (
            value!({"presence":"present","sha256":"0".repeat(64),"extra":true}),
            "unknown field",
        ),
    ] {
        Host::new("payload").reject_result_shape(result, diagnostic);
    }
}

#[test]
fn interrupted_start_retains_intent_and_recovers_missing_or_partial_result() {
    for state in ["absent", "empty", "truncated"] {
        let h = Host::new("payload");
        h.interrupting_git("");
        let source_head = git(&h.root, &["rev-parse", "HEAD"]);
        fs::write(h.root.join("payload"), "unsaved original source\n").unwrap();
        let target = h.parent.join("interrupted λ");
        h.crash_start(&target);
        let (intent_path, intent) = h.intent("start");
        let intent_bytes = fs::read(h.root.join(&intent_path)).unwrap();
        let result = h.root.join(intent["report_path"].as_str().unwrap());
        assert_eq!(fs::read(&result).unwrap(), b"");
        if state == "absent" {
            fs::remove_file(&result).unwrap();
        }
        if state == "truncated" {
            fs::write(&result, b"{\"status\":").unwrap();
        }
        let prior = fs::read(&result).ok();
        let plan = h.interruption_plan("start", &target);
        let (code, r, error) = h.maintain("recover-interrupted", plan.clone());
        assert_eq!(code, 0, "{state}: {r} {error}");
        assert_eq!(r["status"], "recovered");
        assert_eq!(r["original_outcome"], "unknown");
        assert_eq!(r["index_tree"], plan["index_tree"]);
        assert_eq!(r["context"]["candidate"], Value::Null);
        assert_eq!(r["prior_intent"]["input_bytes"], value!(intent_bytes));
        if let Some(bytes) = &prior {
            assert_eq!(r["prior_result"]["input_bytes"], value!(bytes));
        } else {
            assert_eq!(r["prior_result"]["presence"], "absent");
        }
        assert_eq!(fs::read(&result).ok(), prior);
        assert_eq!(fs::read(h.root.join(&intent_path)).unwrap(), intent_bytes);
        assert_eq!(
            fs::read(h.root.join("payload")).unwrap(),
            b"unsaved original source\n"
        );
        assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), source_head);
        assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    }
}

#[test]
fn interrupted_reconstruction_preserves_actual_index_and_original_source() {
    let h = Host::new("payload");
    h.interrupting_git("");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("payload"), "carried content\n").unwrap();
    let candidate = commit(&h.root);
    h.upstream(".gitignore", b".chrono-harness/state/\n# advanced target\n");
    let path = ".chrono-harness/state/reconstruction.json";
    fs::create_dir_all(h.root.join(".chrono-harness/state")).unwrap();
    fs::write(
        h.root.join(path),
        serde_json::to_vec(&reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"carry"}]),
        ))
        .unwrap(),
    )
    .unwrap();
    let target = h.parent.join("interrupted apply");
    h.crash(
        &[
            "reconstruct",
            "--host-root",
            h.root.to_str().unwrap(),
            "--config",
            POLICY,
            "--kind",
            "integration",
            "--name",
            "interrupted",
            "--path",
            target.to_str().unwrap(),
            "--plan",
            path,
        ],
        "apply --3way",
    );
    let plan = h.interruption_plan("reconstruct", &target);
    assert_ne!(
        plan["index_tree"],
        git(&target, &["rev-parse", "HEAD^{tree}"])
    );
    let (code, r, error) = h.maintain("recover-interrupted", plan.clone());
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["index_tree"], plan["index_tree"]);
    assert_eq!(r["original_outcome"], "unknown");
    assert_eq!(
        fs::read(target.join("payload")).unwrap(),
        b"carried content\n"
    );
    assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), candidate);
    assert_eq!(
        git(&target, &["diff", "--cached", "--name-only"]),
        "payload"
    );
}

#[test]
fn interrupted_cleanup_preserves_work_and_recovers_its_owned_lock() {
    let h = Host::new("payload");
    h.interrupting_git("");
    let target = h.parent.join("finished");
    assert_eq!(h.invoke("integration", "finished", &target).0, 0);
    let plan = h.cleanup(&target);
    let path = ".chrono-harness/state/cleanup.json";
    fs::write(h.root.join(path), serde_json::to_vec(&plan).unwrap()).unwrap();
    h.crash(
        &[
            "cleanup",
            "--host-root",
            h.root.to_str().unwrap(),
            "--config",
            POLICY,
            "--plan",
            path,
        ],
        "worktree lock",
    );
    let before = fs::read(target.join("payload")).unwrap();
    let (code, r, error) = h.maintain(
        "recover-interrupted",
        h.interruption_plan("cleanup", &target),
    );
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["original_outcome"], "unknown");
    assert_eq!(fs::read(target.join("payload")).unwrap(), before);
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    assert_eq!(h.maintain("cleanup", plan).0, 0);
}

#[test]
fn interrupted_recovery_rejects_terminal_result_and_bound_identity_drift() {
    for fault in ["terminal", "intent", "result", "lock", "tree", "config"] {
        let h = Host::new("payload");
        h.interrupting_git("");
        let target = h.parent.join("interrupted");
        if fault == "terminal" {
            h.hook("exit 17");
            assert_ne!(h.invoke("integration", "interrupted", &target).0, 0);
        } else {
            h.crash_start(&target);
        }
        let mut plan = h.interruption_plan("start", &target);
        let (_, intent) = h.intent("start");
        let result = h.root.join(intent["report_path"].as_str().unwrap());
        match fault {
            "intent" => plan["intent"]["sha256"] = value!("0".repeat(64)),
            "result" => {
                fs::write(&result, b"changed partial result").unwrap();
            }
            "tree" => plan["index_tree"] = value!("0".repeat(40)),
            "lock" => {
                git(&h.root, &["worktree", "unlock", target.to_str().unwrap()]);
                git(
                    &h.root,
                    &[
                        "worktree",
                        "lock",
                        "--reason",
                        "other-owner",
                        target.to_str().unwrap(),
                    ],
                );
            }
            "config" => h.policy(|p| p["timeout_seconds"] = value!(31)),
            _ => (),
        }
        let bytes = fs::read(&result).unwrap();
        let (code, r, error) = h.maintain("recover-interrupted", plan);
        assert_ne!(code, 0, "{fault}: {r} {error}");
        if fault == "terminal" {
            assert!(r["error"].as_str().unwrap().contains("terminal"), "{r}");
        }
        assert_eq!(fs::read(&result).unwrap(), bytes);
        assert!(target.exists());
        assert!(git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    }
}

#[test]
fn interrupted_intent_publication_never_overwrites_or_creates_checkout_on_failure() {
    let h = Host::new("payload");
    h.interrupting_git(
        r#"if [ "$2" = show-ref ] && [ "$5" = refs/heads/feature/new ]; then
 for result in .chrono-harness/state/worktrees/start-*.json; do
  printf occupied > "$result.intent.json"
 done
fi"#,
    );
    let target = h.parent.join("must remain absent");
    let (code, r, error) = h.invoke("feature", "new", &target);
    assert_ne!(code, 0, "{r} {error}");
    assert!(r["error"].as_str().unwrap().contains("intent"), "{r}");
    assert!(!target.exists());
    assert!(
        !git(&h.root, &["for-each-ref", "--format=%(refname)"]).contains("refs/heads/feature/new")
    );
    let file = format!("{}.intent.json", r["report_path"].as_str().unwrap());
    assert_eq!(fs::read(h.root.join(file)).unwrap(), b"occupied");
}
