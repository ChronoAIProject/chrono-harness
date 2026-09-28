use super::*;

impl Host {
    fn maintain(&self, operation: &str, plan: Value) -> (i32, Value, String) {
        let path = ".chrono-harness/state/maintenance.json";
        fs::create_dir_all(self.root.join(".chrono-harness/state")).unwrap();
        fs::write(self.root.join(path), serde_json::to_vec(&plan).unwrap()).unwrap();
        let output = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .current_dir("/")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args([
                operation,
                "--host-root",
                self.root.to_str().unwrap(),
                "--config",
                POLICY,
                "--plan",
                path,
            ])
            .output()
            .unwrap();
        self.received(output)
    }
    fn recovery(&self, report: &Value, target: &Path) -> Value {
        let path = report["report_path"].as_str().unwrap();
        value!({"schema":"chrono-worktree-maintenance/v1","operation":"recover",
            "receipt":{"path":path,"sha256":sha256(&fs::read(self.root.join(path)).unwrap())},
            "head":git(target,&["rev-parse","HEAD"]),"index_tree":git(target,&["write-tree"])})
    }
    fn cleanup(&self, target: &Path) -> Value {
        value!({"schema":"chrono-worktree-maintenance/v1","operation":"cleanup",
            "path":target,"branch":git(target,&["branch","--show-current"]),
            "head":git(target,&["rev-parse","HEAD"]),
            "retained_ref":"refs/heads/dev","retained_commit":git(&self.root,&["rev-parse","refs/heads/dev"]),
            "retention":"ancestor","discard_artifacts":[],"remove_branch":true,"allow_absent_worktree":false})
    }
}

#[test]
fn maintenance_recovers_failed_hook_without_rewriting_original_failure() {
    let h = Host::new("arbitrary/input.go");
    h.hook("echo original-hook-failure >&2\nexit 17");
    let target = h.parent.join("failed λ");
    let (code, original, _) = h.invoke("feature", "failed", &target);
    assert_ne!(code, 0);
    let path = h.root.join(original["report_path"].as_str().unwrap());
    let bytes = fs::read(&path).unwrap();
    let (code, recovered, error) = h.maintain("recover", h.recovery(&original, &target));
    assert_eq!(code, 0, "{recovered} {error}");
    assert_eq!(recovered["status"], "recovered");
    assert_eq!(recovered["prior_report"]["sha256"], sha256(&bytes));
    assert_eq!(recovered["prior_report"]["report"], original);
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    assert!(
        recovered["processes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| !r["argv"].as_array().unwrap().contains(&value!("hook")))
    );
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), original["base"]);
}

#[test]
fn maintenance_recovers_reconciled_conflict_with_explicit_index_tree() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("payload"), "local\n").unwrap();
    let candidate = commit(&h.root);
    h.upstream("payload", b"remote\n");
    let (code, original, _) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"carry"}]),
        ),
        "conflict",
    );
    assert_ne!(code, 0);
    let target = h.parent.join("conflict");
    let path = original["report_path"].as_str().unwrap();
    let unresolved = value!({"schema":"chrono-worktree-maintenance/v1","operation":"recover",
        "receipt":{"path":path,"sha256":sha256(&fs::read(h.root.join(path)).unwrap())},
        "head":git(&target,&["rev-parse","HEAD"]),"index_tree":git(&target,&["rev-parse","HEAD^{tree}"])});
    assert_ne!(h.maintain("recover", unresolved).0, 0);
    assert!(!git(&target, &["ls-files", "--unmerged"]).is_empty());
    fs::write(target.join("payload"), "reconciled\n").unwrap();
    git(&target, &["add", "--", "payload"]);
    let plan = h.recovery(&original, &target);
    let (code, recovered, error) = h.maintain("recover", plan.clone());
    assert_eq!(code, 0, "{recovered} {error}");
    assert_eq!(recovered["index_tree"], plan["index_tree"]);
    assert_eq!(recovered["context"]["candidate"], Value::Null);
    assert_eq!(fs::read(target.join("payload")).unwrap(), b"reconciled\n");
    assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), candidate);
    assert_eq!(recovered["prior_report"]["report"]["status"], "failed");
}

#[test]
fn maintenance_recovery_rejects_identity_lock_receipt_and_unresolved_state() {
    for fault in ["digest", "head", "tree", "lock", "unstaged", "unknown"] {
        let h = Host::new("payload");
        h.hook("exit 17");
        let target = h.parent.join("failed");
        let (_, original, _) = h.invoke("feature", "failed", &target);
        let mut plan = h.recovery(&original, &target);
        match fault {
            "digest" => plan["receipt"]["sha256"] = value!("0".repeat(64)),
            "head" => plan["head"] = value!("0".repeat(40)),
            "tree" => plan["index_tree"] = value!("0".repeat(40)),
            "lock" => {
                git(&h.root, &["worktree", "unlock", target.to_str().unwrap()]);
                git(
                    &h.root,
                    &[
                        "worktree",
                        "lock",
                        "--reason",
                        "another-owner",
                        target.to_str().unwrap(),
                    ],
                );
            }
            "unstaged" => fs::write(target.join("payload"), "not staged").unwrap(),
            _ => fs::write(target.join("unknown"), "not registered").unwrap(),
        }
        let (code, r, _) = h.maintain("recover", plan);
        assert_ne!(code, 0, "{fault}: {r}");
        assert!(target.exists());
        assert!(git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    }
}

#[test]
fn maintenance_cleanup_preserves_saved_work_and_declared_artifact_boundaries() {
    let h = Host::new("unusual/source.mts");
    let target = h.parent.join("saved λ");
    assert_eq!(h.invoke("feature", "saved", &target).0, 0);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(target.join(".chrono-harness/state/output"), "disposable").unwrap();
    let mut plan = h.cleanup(&target);
    plan["path"] = value!("../saved λ");
    let (code, _, _) = h.maintain("cleanup", plan.clone());
    assert_ne!(
        code, 0,
        "an artifact not explicitly selected for disposal must remain"
    );
    assert!(target.exists());
    plan["discard_artifacts"] = value!([".chrono-harness/state/"]);
    let (code, r, error) = h.maintain("cleanup", plan.clone());
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["status"], "cleaned");
    assert_eq!(r["worktree_removed"], true);
    assert_eq!(r["branch_removed"], true);
    assert_eq!(r["worktree_removal"], "verified-absent");
    assert_eq!(r["branch_removal"], "verified-absent");
    assert!(!target.exists());
    assert_eq!(
        git(&h.root, &["rev-parse", "refs/heads/dev"]),
        plan["retained_commit"]
    );
    assert!(
        !git(&h.root, &["for-each-ref", "--format=%(refname)"])
            .contains("refs/heads/feature/saved")
    );
}

#[test]
fn maintenance_cleanup_accepts_squash_tree_preservation_and_explicit_retry() {
    let h = Host::new("payload");
    let target = h.parent.join("saved");
    assert_eq!(h.invoke("integration", "saved", &target).0, 0);
    fs::write(target.join("payload"), "new work").unwrap();
    let candidate = commit(&target);
    fs::write(h.root.join("payload"), "new work").unwrap();
    git(&h.root, &["add", "--", "payload"]);
    git(
        &h.root,
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-m",
            "Different squash identity",
        ],
    );
    let landing = git(&h.root, &["rev-parse", "HEAD"]);
    assert_ne!(candidate, landing);
    let mut plan = h.cleanup(&target);
    assert_ne!(h.maintain("cleanup", plan.clone()).0, 0);
    plan["retention"] = value!("same-tree");
    plan["remove_branch"] = value!(false);
    let (code, r, error) = h.maintain("cleanup", plan.clone());
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["worktree_removal"], "verified-absent");
    assert_eq!(r["branch_removal"], "not-requested");
    assert_eq!(
        git(&h.root, &["rev-parse", "refs/heads/integration/saved"]),
        candidate
    );
    assert_ne!(h.maintain("cleanup", plan.clone()).0, 0);
    plan["allow_absent_worktree"] = value!(true);
    plan["remove_branch"] = value!(true);
    let (code, r, error) = h.maintain("cleanup", plan.clone());
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["worktree_removed"], false);
    assert_eq!(r["branch_removed"], true);
    assert_eq!(r["worktree_removal"], "already-absent");
    assert_eq!(r["branch_removal"], "verified-absent");
    let (code, r, error) = h.maintain("cleanup", plan);
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["branch_removed"], false);
    assert_eq!(r["worktree_removal"], "already-absent");
    assert_eq!(r["branch_removal"], "already-absent");
}

#[test]
fn maintenance_cleanup_rejects_wrong_dirty_locked_main_and_nested_worktrees() {
    for fault in [
        "head", "branch", "retained", "dirty", "ignored", "lock", "main", "nested", "artifact",
    ] {
        let h = Host::new("payload");
        let target = h.parent.join("saved");
        assert_eq!(h.invoke("feature", "saved", &target).0, 0);
        let mut plan = h.cleanup(&target);
        match fault {
            "head" => plan["head"] = value!("0".repeat(40)),
            "branch" => plan["branch"] = value!("feature/other"),
            "retained" => plan["retained_commit"] = value!("0".repeat(40)),
            "dirty" => fs::write(target.join("payload"), "unsaved").unwrap(),
            "ignored" => {
                fs::write(h.root.join(".git/info/exclude"), "unknown\n").unwrap();
                fs::write(target.join("unknown"), "ignored").unwrap();
            }
            "lock" => {
                git(
                    &h.root,
                    &[
                        "worktree",
                        "lock",
                        "--reason",
                        "foreign",
                        target.to_str().unwrap(),
                    ],
                );
            }
            "main" => {
                git(&h.root, &["checkout", "-b", "feature/main"]);
                plan["path"] = value!(h.root);
                plan["branch"] = value!("feature/main");
            }
            "nested" => {
                git(
                    &h.root,
                    &[
                        "worktree",
                        "add",
                        "-b",
                        "feature/nested",
                        target.join("nested").to_str().unwrap(),
                        "HEAD",
                    ],
                );
            }
            _ => plan["discard_artifacts"] = value!(["payload/"]),
        }
        let (code, r, _) = h.maintain("cleanup", plan);
        assert_ne!(code, 0, "{fault}: {r}");
        assert!(target.exists());
        assert!(h.root.exists());
    }
}

#[test]
fn maintenance_cleanup_retains_actual_remove_failure_and_concurrent_branch_update() {
    use std::os::unix::fs::PermissionsExt;
    for concurrent in [false, true] {
        let h = Host::new("payload");
        let real_git = Command::new("/bin/sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap();
        let real_git = String::from_utf8(real_git.stdout)
            .unwrap()
            .trim()
            .to_string();
        let wrapper = h.parent.join("git-wrapper");
        let body = if concurrent {
            format!(
                "if [ \"$2\" = update-ref ] && [ \"$4\" = -d ]; then\n '{real_git}' update-ref refs/heads/feature/saved \"$(cat \"$HOME/new-head\")\"\nfi\n"
            )
        } else {
            "if [ \"$2\" = worktree ] && [ \"$3\" = remove ]; then echo original-remove-failure >&2; exit 71; fi\n".into()
        };
        fs::write(
            &wrapper,
            format!("#!/bin/sh\n{body}exec '{real_git}' \"$@\"\n"),
        )
        .unwrap();
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
        h.policy(|p| p["git"]["program"] = value!(wrapper));
        let target = h.parent.join("saved");
        assert_eq!(h.invoke("feature", "saved", &target).0, 0);
        let plan = h.cleanup(&target);
        fs::write(h.root.join("payload"), "new retained work").unwrap();
        let later = commit(&h.root);
        fs::write(h.parent.join("new-head"), &later).unwrap();
        let mut plan = plan;
        plan["retained_commit"] = value!(later);
        let (code, r, error) = h.maintain("cleanup", plan);
        assert_ne!(code, 0, "{r} {error}");
        assert_eq!(r["status"], "failed");
        assert_eq!(r["worktree_removed"], concurrent);
        assert_eq!(r["branch_removed"], false);
        assert_eq!(
            r["worktree_removal"],
            if concurrent {
                "verified-absent"
            } else {
                "attempted-unverified"
            }
        );
        assert_eq!(
            r["branch_removal"],
            if concurrent {
                "attempted-unverified"
            } else {
                "not-attempted"
            }
        );
        if concurrent {
            assert!(!target.exists());
            assert_eq!(
                git(&h.root, &["rev-parse", "refs/heads/feature/saved"]),
                later
            );
        } else {
            assert!(target.exists());
            assert!(r["processes"].as_array().unwrap().iter().any(|x| {
                x["process"]["exit_code"] == 71
                    && x["process"]["stderr"]
                        .as_str()
                        .unwrap()
                        .contains("original-remove-failure")
            }));
        }
    }
}

#[test]
fn maintenance_rejects_malformed_plans_before_mutation() {
    let h = Host::new("payload");
    let target = h.parent.join("saved");
    assert_eq!(h.invoke("feature", "saved", &target).0, 0);
    for fault in ["schema", "extra", "duplicate", "operation"] {
        let mut plan = h.cleanup(&target);
        match fault {
            "schema" => plan["schema"] = value!("unknown"),
            "extra" => plan["invented"] = value!(true),
            "duplicate" => {
                plan["discard_artifacts"] =
                    value!([".chrono-harness/state/", ".chrono-harness/state/"])
            }
            _ => plan["operation"] = value!("recover"),
        }
        let (code, _, _) = h.maintain("cleanup", plan);
        assert_ne!(code, 0, "{fault}");
        assert!(target.exists());
    }
}

#[test]
fn maintenance_recovers_cleanup_lock_after_retention_race() {
    use std::os::unix::fs::PermissionsExt;
    let h = Host::new("payload");
    let real = Command::new("/bin/sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    let real = String::from_utf8(real.stdout).unwrap().trim().to_string();
    let wrapper = h.parent.join("git-wrapper");
    fs::write(&wrapper, format!("#!/bin/sh\nif [ \"$2\" = worktree ] && [ \"$3\" = lock ]; then\n '{real}' \"$@\" || exit $?\n '{real}' update-ref refs/heads/dev \"$(cat \"$HOME/new-head\")\"\n exit $?\nfi\nexec '{real}' \"$@\"\n")).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    h.policy(|p| p["git"]["program"] = value!(wrapper));
    let target = h.parent.join("saved");
    assert_eq!(h.invoke("feature", "saved", &target).0, 0);
    let tree = git(&h.root, &["rev-parse", "HEAD^{tree}"]);
    let new_head = git(
        &h.root,
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit-tree",
            &tree,
            "-p",
            "HEAD",
            "-m",
            "Concurrent retained update",
        ],
    );
    fs::write(h.parent.join("new-head"), &new_head).unwrap();
    let (code, original, _) = h.maintain("cleanup", h.cleanup(&target));
    assert_ne!(code, 0);
    assert_eq!(original["worktree_removed"], false);
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"])
            .contains(original["lock_reason"].as_str().unwrap())
    );
    let (code, recovered, error) = h.maintain("recover", h.recovery(&original, &target));
    assert_eq!(code, 0, "{recovered} {error}");
    assert_eq!(recovered["status"], "recovered");
    assert_eq!(recovered["prior_report"]["report"]["operation"], "cleanup");
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    let (code, cleaned, error) = h.maintain("cleanup", h.cleanup(&target));
    assert_eq!(code, 0, "{cleaned} {error}");
    assert!(!target.exists());
    assert_eq!(git(&h.root, &["rev-parse", "refs/heads/dev"]), new_head);
}

#[test]
fn maintenance_rechecks_work_after_releasing_owned_locks() {
    use std::os::unix::fs::PermissionsExt;
    for operation in ["cleanup", "recover"] {
        let h = Host::new("payload");
        let real = Command::new("/bin/sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap();
        let real = String::from_utf8(real.stdout).unwrap().trim().to_string();
        let wrapper = h.parent.join("git-wrapper");
        fs::write(&wrapper, format!("#!/bin/sh\nif [ -f \"$HOME/arm\" ] && [ \"$2\" = worktree ] && [ \"$3\" = unlock ]; then\n '{real}' \"$@\" || exit $?\n printf 'new unsaved work' > \"$4/payload\"\n exit 0\nfi\nexec '{real}' \"$@\"\n")).unwrap();
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
        h.policy(|p| p["git"]["program"] = value!(wrapper));
        let target = h.parent.join("saved");
        if operation == "recover" {
            h.hook("exit 17");
        }
        let (code, original, _) = h.invoke("feature", "saved", &target);
        assert_eq!(code == 0, operation == "cleanup");
        fs::write(h.parent.join("arm"), "").unwrap();
        let plan = if operation == "cleanup" {
            h.cleanup(&target)
        } else {
            h.recovery(&original, &target)
        };
        let (code, report, _) = h.maintain(operation, plan);
        assert_ne!(code, 0, "new unsaved work must survive: {report}");
        if operation == "cleanup" {
            assert_eq!(report["worktree_removed"], false);
        }
        assert!(target.exists());
        assert_eq!(
            fs::read(target.join("payload")).unwrap(),
            b"new unsaved work"
        );
    }
}
