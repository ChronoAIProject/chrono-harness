use super::*;

impl Host {
    fn fetch_intent(&self) -> (String, Value) {
        let paths: Vec<_> = fs::read_dir(self.root.join(".chrono-harness/state/worktrees"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.to_str().unwrap().ends_with(".fetch-intent.json"))
            .collect();
        assert_eq!(paths.len(), 1, "fetch must retain its pre-effect identity");
        let path = paths[0]
            .strip_prefix(&self.root)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let intent = json(&fs::read(&paths[0]).unwrap()).unwrap();
        assert_eq!(intent["schema"], "chrono-worktree-fetch-intent/v1");
        assert!(intent.get("status").is_none() && intent.get("base").is_none());
        (path, intent)
    }
    fn crash_fetch(&self, stage: &str) {
        self.crash_fetch_operation(stage, "start");
    }
    fn crash_fetch_operation(&self, stage: &str, operation: &str) {
        let target = self.parent.join("unused destination");
        let mut args = vec![
            operation,
            "--host-root",
            self.root.to_str().unwrap(),
            "--config",
            POLICY,
            "--kind",
            "integration",
            "--name",
            "interrupted-fetch",
            "--path",
            target.to_str().unwrap(),
        ];
        if operation == "reconstruct" {
            let path = ".chrono-harness/state/reconstruction.json";
            let head = git(&self.root, &["rev-parse", "HEAD"]);
            fs::create_dir_all(self.root.join(".chrono-harness/state")).unwrap();
            fs::write(
                self.root.join(path),
                serde_json::to_vec(&reconstruction(&head, &head, value!([]))).unwrap(),
            )
            .unwrap();
            args.extend(["--plan", path]);
        }
        self.crash(&args, stage);
        assert!(!target.exists());
    }
    fn interrupted_fetch_plan(&self) -> Value {
        let (path, intent) = self.fetch_intent();
        let result = self.root.join(intent["report_path"].as_str().unwrap());
        let observation = match fs::read(result) {
            Ok(bytes) => value!({"presence":"present","sha256":sha256(&bytes)}),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => value!({"presence":"absent"}),
            Err(e) => panic!("{e}"),
        };
        value!({"schema":"chrono-worktree-maintenance/v1","operation":"cleanup-fetch-interrupted",
            "intent":{"path":path,"sha256":sha256(&fs::read(self.root.join(&path)).unwrap())},
            "result":observation,"head":git(&self.root,&["rev-parse","HEAD"]),
            "retained_ref":"refs/heads/dev","retained_commit":git(&self.root,&["rev-parse","HEAD"]),
            "allow_absent_ref":false})
    }
}

#[test]
fn interrupted_fetch_cleans_ref_and_preserves_absent_or_partial_result() {
    for state in ["absent", "empty", "truncated", "reconstruct"] {
        let h = Host::new("payload");
        h.interrupting_git("");
        if state != "reconstruct" {
            fs::write(h.root.join("payload"), b"unsaved source").unwrap();
        }
        let source_bytes = fs::read(h.root.join("payload")).unwrap();
        h.crash_fetch_operation(
            "fetch --no-tags",
            if state == "reconstruct" {
                "reconstruct"
            } else {
                "start"
            },
        );
        let (path, intent) = h.fetch_intent();
        let intent_bytes = fs::read(h.root.join(&path)).unwrap();
        let result = h.root.join(intent["report_path"].as_str().unwrap());
        assert_eq!(fs::read(&result).unwrap(), b"");
        if state == "absent" {
            fs::remove_file(&result).unwrap();
        }
        if state == "truncated" {
            fs::write(&result, b"{\"status\":").unwrap();
        }
        let original = fs::read(&result).ok();
        let plan = h.interrupted_fetch_plan();
        let fetch_ref = intent["fetch_ref"].as_str().unwrap();
        assert_eq!(git(&h.root, &["rev-parse", fetch_ref]), plan["head"]);
        let (code, report, error) = h.maintain("cleanup-fetch-interrupted", plan.clone());
        assert_eq!(code, 0, "{state}: {report} {error}");
        assert_eq!(report["status"], "cleaned");
        assert_eq!(report["original_outcome"], "unknown");
        assert_eq!(report["fetch_ref_removal"], "verified-absent");
        assert_eq!(report["fetch_ref_removed"], true);
        assert_eq!(report["prior_intent"]["input_bytes"], value!(intent_bytes));
        assert_eq!(fs::read(h.root.join(path)).unwrap(), intent_bytes);
        assert_eq!(fs::read(&result).ok(), original);
        assert_eq!(fs::read(h.root.join("payload")).unwrap(), source_bytes);
        assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), plan["head"]);
        assert!(
            git(
                &h.root,
                &[
                    "for-each-ref",
                    "--format=%(refname)",
                    "refs/chrono-harness/fetch/"
                ]
            )
            .is_empty()
        );
    }
}

#[test]
fn interrupted_fetch_after_deletion_requires_explicit_absence_retry() {
    let h = Host::new("payload");
    h.interrupting_git("");
    h.crash_fetch("update-ref --no-deref");
    let mut plan = h.interrupted_fetch_plan();
    assert_ne!(h.maintain("cleanup-fetch-interrupted", plan.clone()).0, 0);
    plan["allow_absent_ref"] = value!(true);
    let (code, r, error) = h.maintain("cleanup-fetch-interrupted", plan);
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["fetch_ref_removal"], "already-absent");
    assert_eq!(r["fetch_ref_removed"], false);
    assert_eq!(r["original_outcome"], "unknown");
}

#[test]
fn interrupted_fetch_rejects_identity_drift_symbolic_refs_and_lost_retention() {
    for fault in [
        "intent",
        "result",
        "source",
        "ref",
        "config",
        "symbolic",
        "head",
        "retention",
    ] {
        let h = Host::new("payload");
        h.interrupting_git("");
        h.crash_fetch("fetch --no-tags");
        let (path, mut intent) = h.fetch_intent();
        let fetch_ref = intent["fetch_ref"].as_str().unwrap().to_string();
        let result = h.root.join(intent["report_path"].as_str().unwrap());
        let mut plan = h.interrupted_fetch_plan();
        match fault {
            "intent" => plan["intent"]["sha256"] = value!("0".repeat(64)),
            "result" => fs::write(&result, b"changed partial result").unwrap(),
            "config" => h.policy(|p| p["timeout_seconds"] = value!(31)),
            "symbolic" => {
                git(&h.root, &["symbolic-ref", &fetch_ref, "refs/heads/dev"]);
            }
            "head" | "retention" => {
                fs::write(h.root.join("payload"), b"new retained commit").unwrap();
                let next = commit(&h.root);
                if fault == "head" {
                    git(&h.root, &["update-ref", &fetch_ref, &next]);
                }
            }
            _ => {
                if fault == "source" {
                    intent["source_root"] = value!(h.parent);
                } else {
                    intent["fetch_ref"] = value!("refs/heads/dev");
                }
                let bytes = serde_json::to_vec(&intent).unwrap();
                fs::write(h.root.join(&path), &bytes).unwrap();
                plan["intent"]["sha256"] = value!(sha256(&bytes));
            }
        }
        let original = fs::read(&result).unwrap();
        let head = git(&h.root, &["rev-parse", &fetch_ref]);
        let (code, r, error) = h.maintain("cleanup-fetch-interrupted", plan);
        assert_ne!(code, 0, "{fault}: {r} {error}");
        assert_eq!(git(&h.root, &["rev-parse", &fetch_ref]), head);
        assert_eq!(fs::read(&result).unwrap(), original);
    }
}

#[test]
fn interrupted_fetch_terminal_result_requires_ordinary_receipt_cleanup() {
    let h = Host::new("payload");
    let original = h.failed_fetch(true);
    let plan = h.interrupted_fetch_plan();
    let (code, r, error) = h.maintain("cleanup-fetch-interrupted", plan.clone());
    assert_ne!(code, 0, "{r} {error}");
    assert!(r["error"].as_str().unwrap().contains("terminal"));
    let path = original["report_path"].as_str().unwrap();
    let bytes = fs::read(h.root.join(path)).unwrap();
    fs::write(h.parent.join("fetch-action"), "allow").unwrap();
    let ordinary = value!({"schema":"chrono-worktree-maintenance/v1","operation":"cleanup-fetch",
        "receipt":{"path":path,"sha256":sha256(&bytes)},"head":plan["head"],"retained_ref":plan["retained_ref"],
        "retained_commit":plan["retained_commit"],"allow_absent_ref":false});
    let (code, r, error) = h.maintain("cleanup-fetch", ordinary);
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(fs::read(h.root.join(path)).unwrap(), bytes);
}

#[test]
fn interrupted_fetch_publication_collision_prevents_fetch() {
    let h = Host::new("payload");
    h.interrupting_git(
        r#"if [ "$2 $3" = 'worktree list' ]; then
 for result in .chrono-harness/state/worktrees/start-*.json; do
  printf occupied > "$result.fetch-intent.json" || exit $?
 done
fi
if [ "$2" = fetch ]; then printf invoked > "$HOME/unexpected-fetch" || exit $?; fi"#,
    );
    let target = h.parent.join("absent destination");
    let (code, r, error) = h.invoke("integration", "collision", &target);
    assert_ne!(code, 0, "{r} {error}");
    assert!(!h.parent.join("unexpected-fetch").exists());
    assert!(!target.exists());
    let path = format!("{}.fetch-intent.json", r["report_path"].as_str().unwrap());
    assert_eq!(fs::read(h.root.join(path)).unwrap(), b"occupied");
}

#[test]
fn interrupted_fetch_rechecks_original_evidence_after_removal() {
    let h = Host::new("payload");
    h.interrupting_git(
        r#"if [ -f "$HOME/change-result" ] && [ "$2 $3 $4" = 'update-ref --no-deref -d' ]; then
 REAL "$@" || exit $?
 printf changed > "$(cat "$HOME/change-result")" || exit $?
 exit 0
fi"#,
    );
    h.crash_fetch("fetch --no-tags");
    let (_, intent) = h.fetch_intent();
    let plan = h.interrupted_fetch_plan();
    let result = h.root.join(intent["report_path"].as_str().unwrap());
    fs::write(h.parent.join("change-result"), result.to_str().unwrap()).unwrap();
    let (code, r, error) = h.maintain("cleanup-fetch-interrupted", plan);
    assert_ne!(code, 0, "{r} {error}");
    assert!(
        r["error"].as_str().unwrap().contains("evidence changed"),
        "actual error: {}",
        r["error"]
    );
    assert_eq!(r["fetch_ref_removal"], "verified-absent");
    assert_eq!(fs::read(result).unwrap(), b"changed");
}
