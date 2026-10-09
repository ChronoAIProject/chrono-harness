use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};

fn declaration(h: &Host, damaged: bool) -> (Value, PathBuf, PathBuf, Vec<u8>) {
    let target = h.parent.join("orphan λ checkout");
    assert_eq!(h.invoke("feature", "orphan", &target).0, 0);
    let head = git(&target, &["rev-parse", "HEAD"]);
    fs::write(target.join("payload"), b"staged\n").unwrap();
    git(&target, &["add", "payload"]);
    let tree = git(&target, &["write-tree"]);
    fs::write(target.join("payload"), b"unstaged\0bytes\xff").unwrap();
    fs::set_permissions(target.join("payload"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(target.join("unknown neighbor"), b"preserve untracked\n").unwrap();
    symlink("payload", target.join("literal alias")).unwrap();
    let metadata = PathBuf::from(git(&target, &["rev-parse", "--absolute-git-dir"]));
    let gitfile = fs::read(target.join(".git")).unwrap();
    if damaged {
        fs::write(metadata.join("index"), b"damaged original index\0\xff").unwrap();
    } else {
        fs::remove_dir_all(&metadata).unwrap();
    }
    // This case deliberately has no old receipt or intent to consult.
    fs::remove_dir_all(h.root.join(".chrono-harness/state")).unwrap();
    let plan = value!({"schema":"chrono-worktree-maintenance/v1","operation":"inspect-rebind","head":head,
        "binding":{"path":target,"branch":"feature/orphan","index_tree":tree,
            "metadata_id":metadata.file_name().unwrap().to_str().unwrap(),
            "backup":".chrono-harness/state/original-metadata","donor":h.parent.join("empty donor λ"),"expected":null}});
    (plan, target, metadata, gitfile)
}
fn inspect(h: &Host, plan: Value) -> Value {
    let (code, report, error) = h.maintain("inspect-rebind", plan);
    assert_eq!(code, 0, "inspection must succeed: {report} {error}");
    assert_eq!(report["status"], "observed");
    assert_eq!(report["proposed_plan"]["operation"], "rebind");
    report["proposed_plan"].clone()
}
fn visible(target: &Path) {
    assert_eq!(
        fs::read(target.join("payload")).unwrap(),
        b"unstaged\0bytes\xff"
    );
    assert_eq!(
        fs::metadata(target.join("payload"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert_eq!(
        fs::read(target.join("unknown neighbor")).unwrap(),
        b"preserve untracked\n"
    );
    assert_eq!(
        fs::read_link(target.join("literal alias")).unwrap(),
        Path::new("payload")
    );
}

#[test]
fn metadata_rebind_preserves_visible_work_and_handles_lost_receipts() {
    for absent_gitfile in [false, true] {
        let h = Host::new("payload");
        let source_head = git(&h.root, &["rev-parse", "HEAD"]);
        let (declaration, target, metadata, old_gitfile) = declaration(&h, false);
        if absent_gitfile {
            fs::remove_file(target.join(".git")).unwrap();
            fs::remove_dir(metadata.parent().unwrap()).unwrap();
        }
        let plan = inspect(&h, declaration);
        visible(&target);
        let (code, report, error) = h.maintain("rebind", plan.clone());
        assert_eq!(code, 0, "{report} {error}");
        assert_eq!(report["status"], "rebound");
        assert_eq!(report["original_outcome"], "unknown");
        assert_eq!(report["index_origin"], "explicit-plan");
        assert_eq!(git(&target, &["rev-parse", "HEAD"]), plan["head"]);
        assert_eq!(git(&target, &["write-tree"]), plan["binding"]["index_tree"]);
        assert_eq!(
            git(&target, &["branch", "--show-current"]),
            "feature/orphan"
        );
        assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), source_head);
        assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
        assert!(!h.parent.join("empty donor λ").exists());
        visible(&target);
        let backup = h.root.join(".chrono-harness/state/original-metadata");
        if absent_gitfile {
            assert!(!backup.join("gitfile").exists());
        } else {
            assert_eq!(fs::read(backup.join("gitfile")).unwrap(), old_gitfile);
        }
        assert_eq!(report["visible_before"], report["visible_after"]);
        assert!(report["rebind_intent"]["sha256"].is_string());
    }
}

#[test]
fn metadata_rebind_preserves_damaged_metadata_and_original_gitfile() {
    let h = Host::new("payload");
    let (decl, target, metadata, old_gitfile) = declaration(&h, true);
    let old_head = fs::read(metadata.join("HEAD")).unwrap();
    let plan = inspect(&h, decl);
    let (code, report, error) = h.maintain("rebind", plan);
    assert_eq!(code, 0, "{report} {error}");
    let backup = h.root.join(".chrono-harness/state/original-metadata");
    assert_eq!(
        fs::read(backup.join("metadata/index")).unwrap(),
        b"damaged original index\0\xff"
    );
    assert_eq!(fs::read(backup.join("metadata/HEAD")).unwrap(), old_head);
    assert_eq!(fs::read(backup.join("gitfile")).unwrap(), old_gitfile);
    assert!(!metadata.exists());
    visible(&target);
}

#[test]
fn metadata_rebind_refuses_changed_inputs_and_backup_collisions() {
    for fault in [
        "visible",
        "gitfile",
        "metadata",
        "backup",
        "donor",
        "plan-extra",
    ] {
        let h = Host::new("payload");
        let (decl, target, metadata, _) = declaration(&h, true);
        let mut plan = inspect(&h, decl);
        match fault {
            "visible" => fs::write(target.join("new file"), "later work").unwrap(),
            "gitfile" => fs::write(target.join(".git"), "changed gitfile").unwrap(),
            "metadata" => fs::write(metadata.join("new field"), "later metadata").unwrap(),
            "backup" => {
                fs::create_dir(h.root.join(".chrono-harness/state/original-metadata")).unwrap()
            }
            "donor" => fs::create_dir(h.parent.join("empty donor λ")).unwrap(),
            _ => plan["binding"]["undeclared"] = value!(true),
        }
        let (code, report, _) = h.maintain("rebind", plan);
        assert_ne!(code, 0, "{fault}: {report}");
        assert!(metadata.exists());
        visible(&target);
        assert!(
            !report["processes"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|p| {
                    p["argv"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|v| v == "--no-checkout"))
                })
        );
    }
}

#[test]
fn metadata_rebind_rejects_owned_or_overlapping_destinations_and_ref_drift() {
    for fault in [
        "source",
        "donor",
        "allocation",
        "healthy",
        "wrong-metadata",
        "branch",
        "tree",
    ] {
        let h = Host::new("payload");
        let (decl, target, metadata, _) = declaration(&h, true);
        let mut plan = inspect(&h, decl);
        match fault {
            "source" => plan["binding"]["path"] = value!(h.root),
            "donor" => plan["binding"]["donor"] = value!(target),
            "allocation" => {
                let parent = h.parent.join("separate donor parent");
                fs::create_dir(&parent).unwrap();
                plan["binding"]["donor"] = value!(parent.join(metadata.file_name().unwrap()));
            }
            "healthy" => {
                fs::remove_file(metadata.join("index")).unwrap();
                git(
                    &target,
                    &["read-tree", plan["binding"]["index_tree"].as_str().unwrap()],
                );
            }
            "wrong-metadata" => {
                let other = h.parent.join("other owner");
                assert_eq!(h.invoke("feature", "other", &other).0, 0);
                let other_metadata =
                    PathBuf::from(git(&other, &["rev-parse", "--absolute-git-dir"]));
                fs::remove_file(target.join(".git")).unwrap();
                plan["operation"] = value!("inspect-rebind");
                plan["binding"]["expected"] = Value::Null;
                plan["binding"]["metadata_id"] =
                    value!(other_metadata.file_name().unwrap().to_str().unwrap());
                assert_ne!(h.maintain("inspect-rebind", plan).0, 0);
                assert!(other_metadata.exists());
                assert_eq!(git(&other, &["branch", "--show-current"]), "feature/other");
                continue;
            }
            "branch" => {
                fs::write(h.root.join("payload"), "new source commit\n").unwrap();
                let changed = commit(&h.root);
                git(
                    &h.root,
                    &["update-ref", "refs/heads/feature/orphan", &changed],
                );
            }
            _ => plan["binding"]["index_tree"] = value!("0".repeat(40)),
        }
        assert_ne!(h.maintain("rebind", plan).0, 0, "{fault}");
        visible(&target);
        assert!(metadata.exists());
        assert!(
            !h.root
                .join(".chrono-harness/state/original-metadata")
                .exists()
        );
    }
}

#[test]
fn metadata_rebind_retains_failed_git_process_and_original_metadata() {
    let h = Host::new("payload");
    h.remote_git("index-failure");
    let (decl, target, _, gitfile) = declaration(&h, true);
    let plan = inspect(&h, decl);
    let (code, report, _) = h.maintain("rebind", plan);
    assert_ne!(code, 0);
    assert_eq!(report["phase"], "donor-created");
    let process = &report["processes"].as_array().unwrap().last().unwrap()["process"];
    assert_eq!(process["exit_code"], 29);
    assert_eq!(process["stderr_bytes"][0], 255);
    assert_eq!(fs::read(target.join(".git")).unwrap(), gitfile);
    assert_eq!(
        fs::read(
            h.root
                .join(".chrono-harness/state/original-metadata/metadata/index")
        )
        .unwrap(),
        b"damaged original index\0\xff"
    );
    assert!(h.parent.join("empty donor λ").exists());
    visible(&target);
}

#[test]
fn metadata_rebind_checkpoint_precedes_effects_and_preserves_partial_results() {
    let h = Host::new("payload");
    h.remote_git("donor-failure");
    let (decl, target, _, gitfile) = declaration(&h, true);
    let plan = inspect(&h, decl);
    let (code, report, _) = h.maintain("rebind", plan);
    assert_ne!(code, 0);
    let last = &report["processes"].as_array().unwrap().last().unwrap()["process"];
    assert_eq!(last["exit_code"], 31, "{report}");
    assert!(h.parent.join("empty donor λ/.git").exists());
    assert_eq!(fs::read(target.join(".git")).unwrap(), gitfile);
    visible(&target);
    let intent_path = h
        .root
        .join(report["rebind_intent"]["path"].as_str().unwrap());
    let bytes = fs::read(intent_path).unwrap();
    assert_eq!(sha256(&bytes), report["rebind_intent"]["sha256"]);
    let intent = json(&bytes).unwrap();
    assert_eq!(intent["head"], report["head"]);
    assert_eq!(intent["index_tree"], report["index_tree"]);
    assert_eq!(intent["plan_sha256"], report["maintenance_plan"]["sha256"]);
    assert!(!intent.as_object().unwrap().contains_key("status"));
}

#[test]
fn metadata_rebind_accepts_relative_or_damaged_pointer_with_owned_backlink() {
    for relative in [true, false] {
        let h = Host::new("payload");
        let (decl, target, metadata, _) = declaration(&h, true);
        let pointer = if relative {
            format!(
                "gitdir: ../{}/.git/worktrees/../worktrees/{}\n",
                h.root.file_name().unwrap().to_str().unwrap(),
                metadata.file_name().unwrap().to_str().unwrap()
            )
            .into_bytes()
        } else {
            b"broken pointer\xff\n".to_vec()
        };
        fs::write(target.join(".git"), &pointer).unwrap();
        let plan = inspect(&h, decl);
        let (code, report, error) = h.maintain("rebind", plan);
        assert_eq!(code, 0, "{report} {error}");
        assert_eq!(
            fs::read(
                h.root
                    .join(".chrono-harness/state/original-metadata/gitfile")
            )
            .unwrap(),
            pointer
        );
        visible(&target);
    }
}

#[test]
fn metadata_rebind_failed_attachment_uses_existing_reconciled_recovery() {
    let h = Host::new("payload");
    h.remote_git("repair-failure");
    let (decl, target, _, _) = declaration(&h, true);
    let plan = inspect(&h, decl);
    let (code, failed, _) = h.maintain("rebind", plan);
    assert_ne!(code, 0);
    assert_eq!(failed["phase"], "pointer-published");
    let original = fs::read(h.root.join(failed["report_path"].as_str().unwrap())).unwrap();
    visible(&target);
    // Explicit caller reconciliation is still required by ordinary recover.
    git(
        &target,
        &["add", "payload", "unknown neighbor", "literal alias"],
    );
    let plan = h.recovery(&failed, &target);
    let (code, recovered, error) = h.maintain("recover", plan);
    assert_eq!(code, 0, "{recovered} {error}");
    assert_eq!(recovered["prior_report"]["report"], failed);
    assert_eq!(
        fs::read(h.root.join(failed["report_path"].as_str().unwrap())).unwrap(),
        original
    );
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    // Existing recovery never silently deletes a remaining donor or original backup.
    assert!(h.parent.join("empty donor λ/.git").exists());
    assert!(
        h.root
            .join(".chrono-harness/state/original-metadata/metadata/index")
            .exists()
    );
    visible(&target);
}

#[test]
fn metadata_rebind_detects_backup_mutation_before_success() {
    let h = Host::new("payload");
    h.remote_git("backup-change");
    let (decl, target, _, _) = declaration(&h, true);
    let plan = inspect(&h, decl);
    let (code, report, _) = h.maintain("rebind", plan);
    assert_ne!(code, 0);
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("preserved metadata identity changed"),
        "{report}"
    );
    visible(&target);
}

#[path = "rebind_resume.rs"]
mod resume;
