use super::*;

impl Host {
    fn remote_plan(&self, branch: &str) -> Value {
        let head = git(&self.root, &["rev-parse", "HEAD"]);
        git(
            &self.root,
            &[
                "push",
                "-q",
                "warehouse",
                &format!("{head}:refs/heads/{branch}"),
            ],
        );
        value!({"schema":"chrono-worktree-maintenance/v1","operation":"cleanup-remote",
            "head":head,"retained_commit":head,"branch":branch,"expected_url":self.remote,
            "retained_ref":"refs/heads/dev","retention":"ancestor","allow_absent_ref":false})
    }
    fn remote_refs(&self, branch: &str) -> String {
        git(
            &self.root,
            &["ls-remote", "warehouse", &format!("refs/heads/{branch}")],
        )
    }
    pub(crate) fn remote_git(&self, fault: &str) {
        native_git(self, "remote", value!({"fault":fault}));
    }
}

#[test]
fn remote_cleanup_preserves_local_work_and_uses_one_explicit_push_destination() {
    let h = Host::new("strange/payload.ts");
    let mut workflow = json(&fs::read(h.root.join(WORKFLOW)).unwrap()).unwrap();
    workflow["target_branch"] = value!("trunk");
    workflow["integration_prefix"] = value!("trial/");
    fs::write(
        h.root.join(WORKFLOW),
        serde_json::to_vec(&workflow).unwrap(),
    )
    .unwrap();
    let head = commit(&h.root);
    git(&h.root, &["branch", "trunk", &head]);
    git(&h.root, &["push", "-q", "warehouse", "trunk"]);
    let target = h.parent.join("local checkout");
    assert_eq!(h.invoke("integration", "owned", &target).0, 0);
    let mut plan = h.remote_plan("trial/owned");
    plan["retained_ref"] = value!("refs/heads/trunk");
    fs::write(target.join("strange/payload.ts"), "unsaved local work").unwrap();
    git(
        &h.root,
        &["config", "remote.warehouse.push", ":refs/heads/dev"],
    );
    let config = fs::read(h.root.join(".git/config")).unwrap();
    let (code, r, e) = h.maintain("cleanup-remote", plan.clone());
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["remote_branch_removed"], true);
    assert_eq!(r["remote_branch_removal"], "verified-absent");
    assert!(h.remote_refs("trial/owned").is_empty());
    assert_eq!(
        git(&h.remote, &["rev-parse", "refs/heads/trunk"]),
        plan["retained_commit"]
    );
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), plan["head"]);
    assert_eq!(
        fs::read(target.join("strange/payload.ts")).unwrap(),
        b"unsaved local work"
    );
    assert_eq!(fs::read(h.root.join(".git/config")).unwrap(), config);
    let push = r["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["argv"][1] == "push")
        .unwrap();
    assert!(push["argv"].as_array().unwrap().contains(&value!(format!(
        "--force-with-lease=refs/heads/trial/owned:{}",
        plan["head"].as_str().unwrap()
    ))));
    assert_eq!(
        push["argv"].as_array().unwrap().last().unwrap(),
        ":refs/heads/trial/owned"
    );
}

#[test]
fn remote_cleanup_checks_squash_retention_and_explicit_absence_retry() {
    let h = Host::new("payload");
    let mut plan = h.remote_plan("feature/finished");
    let parent = plan["head"].as_str().unwrap();
    let tree = git(&h.root, &["rev-parse", "HEAD^{tree}"]);
    let other = git(
        &h.root,
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit-tree",
            &tree,
            "-m",
            "squash root",
        ],
    );
    assert_ne!(other, parent);
    git(&h.root, &["update-ref", "refs/heads/dev", &other]);
    git(&h.root, &["push", "-q", "--force", "warehouse", "dev"]);
    plan["retained_commit"] = value!(other);
    assert_ne!(h.maintain("cleanup-remote", plan.clone()).0, 0);
    assert!(!h.remote_refs("feature/finished").is_empty());
    plan["retention"] = value!("same-tree");
    let (code, r, e) = h.maintain("cleanup-remote", plan.clone());
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["remote_branch_removal"], "verified-absent");
    assert_ne!(h.maintain("cleanup-remote", plan.clone()).0, 0);
    plan["allow_absent_ref"] = value!(true);
    let (code, r, e) = h.maintain("cleanup-remote", plan);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["remote_branch_removal"], "already-absent");
    assert_eq!(r["remote_branch_removed"], false);
    assert!(
        r["processes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["argv"][1] != "push")
    );
}

#[test]
fn remote_cleanup_rejects_endpoint_branch_and_retention_mismatches_before_push() {
    for fault in [
        "multiple-urls",
        "wrong-url",
        "wrong-local-retention",
        "wrong-remote-retention",
        "changed-head",
        "unregistered-branch",
        "target",
        "symbolic",
        "unknown-field",
    ] {
        let h = Host::new("payload");
        let mut plan = h.remote_plan("integration/owned");
        let head = plan["head"].as_str().unwrap().to_string();
        fs::write(h.root.join("payload"), "another commit").unwrap();
        let next = commit(&h.root);
        git(&h.root, &["update-ref", "refs/heads/dev", &head]);
        match fault {
            "multiple-urls" => {
                git(
                    &h.root,
                    &[
                        "config",
                        "--add",
                        "remote.warehouse.pushurl",
                        h.remote.to_str().unwrap(),
                    ],
                );
                git(
                    &h.root,
                    &[
                        "config",
                        "--add",
                        "remote.warehouse.pushurl",
                        h.remote.to_str().unwrap(),
                    ],
                );
            }
            "wrong-url" => plan["expected_url"] = value!(h.parent.join("another remote")),
            "wrong-local-retention" => {
                git(&h.root, &["update-ref", "refs/heads/dev", &next]);
            }
            "wrong-remote-retention" => {
                git(
                    &h.root,
                    &["push", "-q", "warehouse", &format!("{next}:refs/heads/dev")],
                );
            }
            "changed-head" => {
                git(
                    &h.root,
                    &[
                        "push",
                        "-q",
                        "warehouse",
                        &format!("{next}:refs/heads/integration/owned"),
                    ],
                );
            }
            "unregistered-branch" => plan["branch"] = value!("unowned/task"),
            "target" => plan["branch"] = value!("dev"),
            "symbolic" => {
                git(
                    &h.remote,
                    &[
                        "symbolic-ref",
                        "refs/heads/integration/owned",
                        "refs/heads/dev",
                    ],
                );
            }
            _ => plan["ignored-authority"] = value!(true),
        }
        let before = h.remote_refs("integration/owned");
        let (code, r, _) = h.maintain("cleanup-remote", plan);
        assert_ne!(code, 0, "{fault}: {r}");
        assert_eq!(h.remote_refs("integration/owned"), before, "{fault}");
        if !r.is_null() {
            assert!(
                r["processes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|p| p["argv"][1] != "push"),
                "{fault}"
            );
        }
    }
}

#[test]
fn remote_cleanup_lease_preserves_a_concurrent_remote_update() {
    let h = Host::new("payload");
    h.remote_git("remote-lease");
    let plan = h.remote_plan("integration/owned");
    fs::write(h.root.join("payload"), "concurrent remote work").unwrap();
    let next = commit(&h.root);
    git(
        &h.root,
        &[
            "push",
            "-q",
            "warehouse",
            &format!("{next}:refs/heads/keep-new"),
        ],
    );
    fs::write(h.parent.join("new-head"), &next).unwrap();
    git(
        &h.root,
        &[
            "update-ref",
            "refs/heads/dev",
            plan["head"].as_str().unwrap(),
        ],
    );
    let (code, r, _) = h.maintain("cleanup-remote", plan);
    assert_ne!(code, 0, "{r}");
    assert_eq!(r["remote_branch_removal"], "attempted-unverified");
    assert_eq!(r["remote_branch_removed"], false);
    assert_eq!(
        git(&h.remote, &["rev-parse", "refs/heads/integration/owned"]),
        next
    );
    let process = &r["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["argv"][1] == "push")
        .unwrap()["process"];
    assert_ne!(process["exit_code"], 0);
    assert!(process["stdout"].as_str().unwrap().contains("stale info"));
}

#[test]
fn remote_cleanup_retains_failure_after_deletion_and_allows_observed_retry() {
    let h = Host::new("payload");
    h.remote_git("post-delete");
    let mut plan = h.remote_plan("integration/owned");
    let (code, r, _) = h.maintain("cleanup-remote", plan.clone());
    assert_ne!(code, 0);
    assert!(h.remote_refs("integration/owned").is_empty());
    assert_eq!(r["remote_branch_removed"], false);
    assert_eq!(r["remote_branch_removal"], "attempted-unverified");
    let path = h.root.join(r["report_path"].as_str().unwrap());
    let original = fs::read(&path).unwrap();
    let process = &r["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["argv"][1] == "push")
        .unwrap()["process"];
    assert_eq!(process["exit_code"], 73);
    assert_eq!(process["stderr_bytes"][0], 255);
    plan["allow_absent_ref"] = value!(true);
    let (code, retry, e) = h.maintain("cleanup-remote", plan);
    assert_eq!(code, 0, "{retry} {e}");
    assert_eq!(retry["remote_branch_removal"], "already-absent");
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn remote_cleanup_does_not_claim_success_after_target_or_plan_changes() {
    for fault in ["target", "plan"] {
        let h = Host::new("payload");
        h.remote_git(if fault == "target" {
            "remote-target"
        } else {
            "remote-plan"
        });
        let plan = h.remote_plan("feature/owned");
        fs::write(h.root.join("payload"), "next target").unwrap();
        let next = commit(&h.root);
        git(
            &h.root,
            &[
                "push",
                "-q",
                "warehouse",
                &format!("{next}:refs/heads/keep-new"),
            ],
        );
        fs::write(h.parent.join("new-head"), &next).unwrap();
        git(
            &h.root,
            &[
                "update-ref",
                "refs/heads/dev",
                plan["head"].as_str().unwrap(),
            ],
        );
        let (code, r, _) = h.maintain("cleanup-remote", plan);
        assert_ne!(code, 0, "{fault}: {r}");
        assert!(h.remote_refs("feature/owned").is_empty());
        assert_eq!(r["remote_branch_removal"], "attempted-unverified");
        assert_eq!(r["remote_branch_removed"], false);
    }
}
