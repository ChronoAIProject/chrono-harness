use super::*;

impl Host {
    pub(super) fn maintain(&self, operation: &str, plan: Value) -> (i32, Value, String) {
        self.received(self.maintenance_output(operation, plan))
    }
    pub(super) fn maintenance_output(&self, operation: &str, plan: Value) -> std::process::Output {
        let path = ".chrono-harness/state/maintenance.json";
        fs::create_dir_all(self.root.join(".chrono-harness/state")).unwrap();
        fs::write(self.root.join(path), serde_json::to_vec(&plan).unwrap()).unwrap();
        Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
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
            .unwrap()
    }
    pub(crate) fn recovery(&self, report: &Value, target: &Path) -> Value {
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
fn raw_checkout_cleanup_preserves_mode_changes_hidden_by_git_configuration() {
    use std::os::unix::fs::PermissionsExt;
    let h = Host::new("payload");
    let target = h.parent.join("unsaved mode");
    assert_eq!(h.invoke("feature", "unsaved-mode", &target).0, 0);
    let plan = h.cleanup(&target);
    git(&h.root, &["config", "core.filemode", "false"]);
    fs::set_permissions(target.join("payload"), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(git(&target, &["status", "--porcelain"]).is_empty());
    let (code, report, error) = h.maintain("cleanup", plan);
    assert_ne!(code, 0, "{report} {error}");
    assert!(report["error"].as_str().unwrap().contains("clean snapshot"));
    assert!(target.join("payload").exists());
    assert_eq!(
        fs::metadata(target.join("payload"))
            .unwrap()
            .permissions()
            .mode()
            & 0o100,
        0o100
    );
}

#[test]
fn raw_checkout_recovery_keeps_lock_until_physical_work_matches_saved_index() {
    use std::os::unix::fs::PermissionsExt;
    let h = Host::new("payload");
    h.hook("exit 17");
    let target = h.parent.join("unreconciled mode");
    let (code, original, _) = h.invoke("feature", "unreconciled-mode", &target);
    assert_ne!(code, 0);
    let plan = h.recovery(&original, &target);
    git(&h.root, &["config", "core.filemode", "false"]);
    fs::set_permissions(target.join("payload"), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(git(&target, &["diff", "--name-only"]).is_empty());
    let (code, report, error) = h.maintain("recover", plan);
    assert_ne!(code, 0, "{report} {error}");
    assert!(report["error"].as_str().unwrap().contains("clean snapshot"));
    assert!(git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    git(&target, &["update-index", "--chmod=+x", "payload"]);
    let (code, report, error) = h.maintain("recover", h.recovery(&original, &target));
    assert_eq!(code, 0, "{report} {error}");
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

impl Host {
    fn failed_fetch(&self, after_fetch: bool) -> Value {
        use std::os::unix::fs::PermissionsExt;
        let real = Command::new("/bin/sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap();
        let real = String::from_utf8(real.stdout)
            .unwrap()
            .trim()
            .replace('\'', "'\\''");
        let wrapper = self.parent.join("fetch-git-wrapper");
        let body = r#"if [ "$2" = fetch ] && [ "$(cat "$HOME/fetch-action")" = fetch-block ]; then
 REAL "$@" || exit $?; echo failure-after-fetch >&2; exit 72
fi
if [ "$2" = update-ref ] && [ "$4" = -d ]; then
 case "$5" in refs/chrono-harness/fetch/*)
  case "$(cat "$HOME/fetch-action")" in
   block) echo original-fetch-ref-removal-failure >&2; exit 71;;
   race) REAL update-ref "$5" "$(cat "$HOME/fetch-new-head")" || exit $?;;
   remove-fail) REAL "$@" || exit $?; echo failure-after-ref-removal >&2; exit 73;;
  esac
 esac
fi
exec REAL "$@"
"#;
        fs::write(
            &wrapper,
            format!("#!/bin/sh\n{}", body.replace("REAL", &format!("'{real}'"))),
        )
        .unwrap();
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(
            self.parent.join("fetch-action"),
            if after_fetch { "fetch-block" } else { "block" },
        )
        .unwrap();
        self.policy(|p| p["git"]["program"] = value!(wrapper));
        let target = self.parent.join("never-created");
        let (code, original, error) = self.invoke("feature", "fetch-failure", &target);
        assert_ne!(code, 0, "{original} {error}");
        assert_eq!(original["status"], "failed");
        assert_eq!(original["fetch_ref_removed"], false);
        let fetched = git(
            &self.root,
            &["rev-parse", original["fetch_ref"].as_str().unwrap()],
        );
        if after_fetch {
            assert!(original.get("base").is_none());
        } else {
            assert_eq!(fetched, original["base"]);
        }
        assert!(!target.exists());
        fs::write(self.parent.join("fetch-action"), "normal").unwrap();
        original
    }
    fn fetch_cleanup(&self, original: &Value) -> Value {
        let path = original["report_path"].as_str().unwrap();
        value!({"schema":"chrono-worktree-maintenance/v1","operation":"cleanup-fetch",
            "receipt":{"path":path,"sha256":sha256(&fs::read(self.root.join(path)).unwrap())},
            "head":git(&self.root, &["rev-parse", original["fetch_ref"].as_str().unwrap()]),"retained_ref":"refs/heads/dev",
            "retained_commit":git(&self.root,&["rev-parse","refs/heads/dev"]),"allow_absent_ref":false})
    }
}

#[test]
fn fetch_cleanup_removes_retained_receipt_ref_and_supports_explicit_retry() {
    for after_fetch in [false, true] {
        let h = Host::new("different/program.go");
        let original = h.failed_fetch(after_fetch);
        let path = h.root.join(original["report_path"].as_str().unwrap());
        let bytes = fs::read(&path).unwrap();
        let expected = git(
            &h.root,
            &["rev-parse", original["fetch_ref"].as_str().unwrap()],
        );
        let unrelated = "refs/chrono-harness/fetch/keep-unrelated";
        git(&h.root, &["update-ref", unrelated, "HEAD"]);
        fs::write(h.root.join("different/program.go"), "unsaved source work\n").unwrap();
        let source_diff = git(&h.root, &["diff", "--binary"]);
        let mut plan = h.fetch_cleanup(&original);
        let (code, r, error) = h.maintain("cleanup-fetch", plan.clone());
        assert_eq!(code, 0, "{r} {error}");
        assert_eq!(r["status"], "cleaned");
        assert_eq!(r["fetch_ref_removed"], true);
        assert_eq!(r["fetch_ref_removal"], "verified-absent");
        assert_eq!(r["prior_report"]["report"], original);
        assert_eq!(r["prior_report"]["sha256"], sha256(&bytes));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(git(&h.root, &["rev-parse", unrelated]), expected);
        assert_eq!(git(&h.root, &["diff", "--binary"]), source_diff);
        assert!(
            !git(&h.root, &["for-each-ref", "--format=%(refname)"])
                .lines()
                .any(|x| x == original["fetch_ref"].as_str().unwrap())
        );
        assert_ne!(h.maintain("cleanup-fetch", plan.clone()).0, 0);
        plan["allow_absent_ref"] = value!(true);
        let (code, r, error) = h.maintain("cleanup-fetch", plan);
        assert_eq!(code, 0, "{r} {error}");
        assert_eq!(r["fetch_ref_removed"], false);
        assert_eq!(r["fetch_ref_removal"], "already-absent");
    }
}

#[test]
fn fetch_cleanup_rejects_receipt_identity_ref_and_retention_changes() {
    for fault in [
        "digest",
        "source",
        "namespace",
        "removed",
        "base",
        "head",
        "ref-changed",
        "retention-moved",
        "unretained",
        "symbolic",
    ] {
        let h = Host::new("unusual/source.ts");
        let mut original = h.failed_fetch(false);
        let fetch_ref = original["fetch_ref"].as_str().unwrap().to_string();
        let mut plan = h.fetch_cleanup(&original);
        match fault {
            "digest" => plan["receipt"]["sha256"] = value!("0".repeat(64)),
            "source" => original["source_root"] = value!(h.parent),
            "namespace" => original["fetch_ref"] = value!("refs/heads/dev"),
            "removed" => original["fetch_ref_removed"] = value!(true),
            "base" => original["base"] = value!("0".repeat(40)),
            "head" => plan["head"] = value!(git(&h.root, &["rev-parse", "HEAD^"])),
            "ref-changed" => {
                git(&h.root, &["update-ref", &fetch_ref, "HEAD^"]);
            }
            "retention-moved" => {
                git(&h.root, &["update-ref", "refs/heads/dev", "HEAD^"]);
            }
            "unretained" => {
                git(&h.root, &["update-ref", "refs/heads/old", "HEAD^"]);
                plan["retained_ref"] = value!("refs/heads/old");
                plan["retained_commit"] = value!(git(&h.root, &["rev-parse", "refs/heads/old"]));
            }
            _ => {
                git(&h.root, &["symbolic-ref", &fetch_ref, "refs/heads/dev"]);
            }
        }
        if ["source", "namespace", "removed", "base"].contains(&fault) {
            let path = h.root.join(original["report_path"].as_str().unwrap());
            fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
            plan["receipt"]["sha256"] = value!(sha256(&fs::read(path).unwrap()));
        }
        let before = git(
            &h.root,
            &[
                "for-each-ref",
                "--format=%(refname) %(objectname) %(symref)",
            ],
        );
        let (code, r, error) = h.maintain("cleanup-fetch", plan);
        assert_ne!(code, 0, "{fault}: {r} {error}");
        assert_eq!(
            r["fetch_ref_removal"], "not-attempted",
            "{fault}: {r} {error}"
        );
        assert_eq!(
            git(
                &h.root,
                &[
                    "for-each-ref",
                    "--format=%(refname) %(objectname) %(symref)"
                ]
            ),
            before
        );
    }
}

#[test]
fn fetch_cleanup_preserves_concurrent_ref_and_original_failure() {
    for action in ["block", "race", "remove-fail"] {
        let h = Host::new("payload");
        let original = h.failed_fetch(false);
        let plan = h.fetch_cleanup(&original);
        let old = fs::read(h.root.join(original["report_path"].as_str().unwrap())).unwrap();
        fs::write(
            h.parent.join("fetch-new-head"),
            git(&h.root, &["rev-parse", "HEAD^"]),
        )
        .unwrap();
        fs::write(h.parent.join("fetch-action"), action).unwrap();
        let (code, r, error) = h.maintain("cleanup-fetch", plan);
        assert_ne!(code, 0, "{action}: {r} {error}");
        assert_eq!(
            r["fetch_ref_removal"], "attempted-unverified",
            "{action}: {r} {error}"
        );
        assert_eq!(r["fetch_ref_removed"], false);
        assert_eq!(
            fs::read(h.root.join(original["report_path"].as_str().unwrap())).unwrap(),
            old
        );
        let fetch_ref = original["fetch_ref"].as_str().unwrap();
        match action {
            "race" => assert_eq!(
                git(&h.root, &["rev-parse", fetch_ref]),
                git(&h.root, &["rev-parse", "HEAD^"])
            ),
            "block" => {
                assert_eq!(git(&h.root, &["rev-parse", fetch_ref]), original["base"]);
                assert!(
                    r["processes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|x| x["process"]["exit_code"] == 71)
                );
            }
            _ => {
                assert!(
                    !git(&h.root, &["for-each-ref", "--format=%(refname)"])
                        .lines()
                        .any(|x| x == fetch_ref)
                );
                assert!(
                    r["processes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|x| x["process"]["exit_code"] == 73)
                );
            }
        }
    }
}

#[path = "interruption.rs"]
mod interruption;

#[path = "remote.rs"]
mod remote;

fn artifact_removal_git(h: &Host, condition: &str) {
    use std::os::unix::fs::PermissionsExt;
    let real = Command::new("/bin/sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    let real = String::from_utf8(real.stdout).unwrap().trim().to_string();
    let wrapper = h.parent.join("artifact-git");
    fs::write(
        &wrapper,
        format!("#!/bin/sh\nif [ \"$2\" = worktree ] && [ \"$3\" = remove ]; then\n{condition}\nfi\nexec '{real}' \"$@\"\n"),
    ).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    h.policy(|p| p["git"]["program"] = value!(wrapper));
}

#[test]
fn cleanup_artifact_disposal_precedes_git_removal() {
    use std::os::unix::fs::symlink;
    let h = Host::new("payload");
    artifact_removal_git(
        &h,
        "if [ -e \"$6/.chrono-harness/state/output\" ]; then echo artifacts-reached-git-removal >&2; exit 71; fi",
    );
    let target = h.parent.join("artifact disposal λ");
    assert_eq!(h.invoke("feature", "artifacts", &target).0, 0);
    let outside = h.parent.join("preserved outside");
    fs::write(&outside, b"not disposable").unwrap();
    fs::create_dir_all(target.join(".chrono-harness/state/nested")).unwrap();
    fs::write(target.join(".chrono-harness/state/output"), b"generated").unwrap();
    symlink(&outside, target.join(".chrono-harness/state/nested/link")).unwrap();
    let mut plan = h.cleanup(&target);
    plan["discard_artifacts"] = value!([".chrono-harness/state/", ".chrono-harness/bin/"]);
    let (code, report, error) = h.maintain("cleanup", plan);
    assert_eq!(code, 0, "{} {error}", report["error"]);
    assert_eq!(
        report["artifact_disposals"],
        value!([
            {"path":".chrono-harness/state/", "status":"verified-absent"},
            {"path":".chrono-harness/bin/", "status":"already-absent"}
        ])
    );
    assert_eq!(report["worktree_removal"], "verified-absent");
    assert_eq!(fs::read(outside).unwrap(), b"not disposable");
    assert!(!target.exists());
}

#[test]
fn cleanup_artifact_disposal_rejects_tracked_content_before_any_effect() {
    let h = Host::new("owned/payload");
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["artifacts"].as_array_mut().unwrap().push(value!({
        "path":"owned/", "owner":"host", "kind":"cache", "tracked":false
    }));
    fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
    h.policy(|_| {});
    let target = h.parent.join("tracked artifact conflict");
    assert_eq!(h.invoke("feature", "tracked-artifact", &target).0, 0);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(
        target.join(".chrono-harness/state/output"),
        b"keep on preflight failure",
    )
    .unwrap();
    let original = fs::read(target.join("owned/payload")).unwrap();
    let mut plan = h.cleanup(&target);
    plan["discard_artifacts"] = value!([".chrono-harness/state/", "owned/"]);
    let (code, report, _) = h.maintain("cleanup", plan);
    assert_ne!(code, 0, "tracked artifact contents must prevent disposal");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("contains tracked paths")
    );
    assert_eq!(fs::read(target.join("owned/payload")).unwrap(), original);
    assert_eq!(
        fs::read(target.join(".chrono-harness/state/output")).unwrap(),
        b"keep on preflight failure"
    );
    assert_eq!(report["worktree_removal"], "not-attempted");
    assert_eq!(report["artifact_disposals"], value!([]));
}

#[test]
fn cleanup_artifact_disposal_retains_partial_failure_and_explicit_retry() {
    let h = Host::new("payload");
    artifact_removal_git(
        &h,
        "if [ -e \"$HOME/fail-remove\" ]; then echo original-remove-failure >&2; exit 71; fi",
    );
    let target = h.parent.join("partial artifact cleanup");
    assert_eq!(h.invoke("feature", "partial-artifacts", &target).0, 0);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(target.join(".chrono-harness/state/output"), b"disposable").unwrap();
    fs::write(h.parent.join("fail-remove"), b"one failed attempt").unwrap();
    let payload = fs::read(target.join("payload")).unwrap();
    let mut plan = h.cleanup(&target);
    plan["discard_artifacts"] = value!([".chrono-harness/state/"]);
    let (code, failed, _) = h.maintain("cleanup", plan.clone());
    assert_ne!(code, 0);
    assert!(
        !target.join(".chrono-harness/state").exists(),
        "selected artifacts must be disposed before the failing Git command"
    );
    assert_eq!(fs::read(target.join("payload")).unwrap(), payload);
    assert_eq!(failed["artifact_disposals"][0]["status"], "verified-absent");
    assert_eq!(failed["worktree_removal"], "attempted-unverified");
    assert!(failed["processes"].as_array().unwrap().iter().any(|row| {
        row["process"]["exit_code"] == 71
            && row["process"]["stderr"]
                .as_str()
                .unwrap()
                .contains("original-remove-failure")
    }));
    let receipt = h.root.join(failed["report_path"].as_str().unwrap());
    let original = fs::read(&receipt).unwrap();
    fs::remove_file(h.parent.join("fail-remove")).unwrap();
    let (code, repaired, error) = h.maintain("cleanup", plan);
    assert_eq!(code, 0, "{} {error}", repaired["error"]);
    assert_eq!(
        repaired["artifact_disposals"][0]["status"],
        "already-absent"
    );
    assert_eq!(repaired["worktree_removal"], "verified-absent");
    assert_eq!(fs::read(receipt).unwrap(), original);
}
