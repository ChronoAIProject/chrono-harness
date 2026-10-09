use super::*;
use std::os::unix::process::ExitStatusExt;

const ORIGINAL: &str = ".chrono-harness/state/original-rebind.json";
const CONTINUATION: &str = ".chrono-harness/state/resume-rebind.json";
fn wrapper(h: &Host) {
    h.remote_git("rebind-interruption");
}
fn invoke(h: &Host, op: &str, path: &str, plan: &Value) -> std::process::Output {
    fs::write(h.root.join(path), serde_json::to_vec(plan).unwrap()).unwrap();
    Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
        .current_dir("/")
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args([
            op,
            "--host-root",
            h.root.to_str().unwrap(),
            "--config",
            POLICY,
            "--plan",
            path,
        ])
        .output()
        .unwrap()
}
fn interruption(
    h: &Host,
    plan: &Value,
    stage: &str,
    failed: bool,
) -> (Value, Value, Vec<u8>, Vec<u8>) {
    fs::write(h.parent.join("rebind-stage"), stage).unwrap();
    if failed {
        fs::write(h.parent.join("rebind-fail"), "fail").unwrap();
    }
    let out = invoke(h, "rebind", ORIGINAL, plan);
    if failed {
        assert_eq!(out.status.code(), Some(2), "{out:?}");
    } else {
        assert_eq!(out.status.signal(), Some(9), "{stage}: {out:?}");
    }
    fs::remove_file(h.parent.join("rebind-stage")).unwrap();
    if failed {
        fs::remove_file(h.parent.join("rebind-fail")).unwrap();
    }
    assert_eq!(
        fs::read(h.parent.join("rebind-interrupted")).unwrap(),
        b"stopped"
    );
    let intents: Vec<_> = fs::read_dir(h.root.join(".chrono-harness/state/worktrees"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_str().unwrap().ends_with(".rebind-intent.json"))
        .collect();
    assert_eq!(intents.len(), 1);
    let bytes = fs::read(&intents[0]).unwrap();
    let intent = json(&bytes).unwrap();
    let result = fs::read(h.root.join(intent["report_path"].as_str().unwrap())).unwrap();
    let resume = value!({"schema":"chrono-worktree-maintenance/v1","operation":"resume-rebind","head":plan["head"],
        "intent":{"path":intents[0].strip_prefix(&h.root).unwrap().to_str().unwrap(),"sha256":sha256(&bytes)},
        "result":{"presence":"present","sha256":sha256(&result)}});
    (resume, intent, bytes, result)
}
fn resumed(h: &Host, plan: &Value) -> (i32, Value, String) {
    h.received(invoke(h, "resume-rebind", CONTINUATION, plan))
}
fn assert_original(
    h: &Host,
    plan: &Value,
    intent: &Value,
    intent_bytes: &[u8],
    result: Option<&[u8]>,
) {
    assert_eq!(
        fs::read(h.root.join(plan["intent"]["path"].as_str().unwrap())).unwrap(),
        intent_bytes
    );
    assert_eq!(
        fs::read(h.root.join(intent["report_path"].as_str().unwrap()))
            .ok()
            .as_deref(),
        result
    );
}
#[test]
fn metadata_rebind_resumes_real_interruptions_at_all_git_boundaries() {
    for stage in [
        "before-preservation",
        "before-add",
        "after-add",
        "after-branch",
        "after-index",
        "before-repair",
        "after-repair",
        "before-unlock",
        "after-unlock",
    ] {
        let h = Host::new("payload");
        wrapper(&h);
        let (decl, target, _, old_gitfile) = declaration(&h, true);
        let original = inspect(&h, decl);
        let (plan, intent, intent_bytes, result) = interruption(&h, &original, stage, false);
        let original_bytes = fs::read(h.root.join(ORIGINAL)).unwrap();
        let (code, report, error) = resumed(&h, &plan);
        assert_eq!(code, 0, "{stage}: {report} {error}");
        assert_eq!(report["status"], "rebound");
        assert_eq!(report["original_outcome"], "unknown");
        assert_eq!(report["visible_before"], report["visible_after"]);
        assert_eq!(
            git(&target, &["write-tree"]),
            original["binding"]["index_tree"]
        );
        assert_eq!(
            git(&target, &["branch", "--show-current"]),
            "feature/orphan"
        );
        assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
        assert!(!h.parent.join("empty donor λ").exists());
        visible(&target);
        assert_original(&h, &plan, &intent, &intent_bytes, Some(&result));
        assert_eq!(fs::read(h.root.join(ORIGINAL)).unwrap(), original_bytes);
        assert_eq!(
            fs::read(
                h.root
                    .join(".chrono-harness/state/original-metadata/gitfile")
            )
            .unwrap(),
            old_gitfile
        );
        assert_eq!(
            fs::read(
                h.root
                    .join(".chrono-harness/state/original-metadata/metadata/index")
            )
            .unwrap(),
            b"damaged original index\0\xff"
        );
    }
}
#[test]
fn metadata_rebind_resume_preserves_absent_partial_and_failed_original_results() {
    for state in ["absent", "empty", "truncated", "failed"] {
        let h = Host::new("payload");
        wrapper(&h);
        let (decl, target, _, _) = declaration(&h, false);
        fs::remove_file(target.join(".git")).unwrap();
        let original = inspect(&h, decl);
        let (mut plan, intent, intent_bytes, mut result) =
            interruption(&h, &original, "after-add", state == "failed");
        let path = h.root.join(intent["report_path"].as_str().unwrap());
        if state == "absent" {
            fs::remove_file(&path).unwrap();
            plan["result"] = value!({"presence":"absent"});
        }
        if state == "truncated" {
            result = b"{\"status\":".to_vec();
            fs::write(&path, &result).unwrap();
            plan["result"]["sha256"] = value!(sha256(&result));
        }
        if state == "failed" {
            let failed = json(&result).unwrap();
            assert_eq!(failed["status"], "failed");
            assert_eq!(
                failed["processes"].as_array().unwrap().last().unwrap()["process"]["exit_code"],
                83
            );
        }
        let (code, r, e) = resumed(&h, &plan);
        assert_eq!(code, 0, "{state}: {r} {e}");
        assert_eq!(
            r["original_outcome"],
            if state == "failed" {
                "failed"
            } else {
                "unknown"
            }
        );
        assert_original(
            &h,
            &plan,
            &intent,
            &intent_bytes,
            if state == "absent" {
                None
            } else {
                Some(&result)
            },
        );
        visible(&target);
        assert_eq!(
            git(&target, &["write-tree"]),
            original["binding"]["index_tree"]
        );
    }
}
#[test]
fn metadata_rebind_resume_refuses_conflicting_state_before_effects() {
    for fault in [
        "visible",
        "backup-pointer",
        "backup-metadata",
        "donor-extra",
        "lock",
        "branch",
        "index",
        "plan",
        "intent",
        "result",
        "source",
        "profile",
        "git-lock",
    ] {
        let h = Host::new("payload");
        wrapper(&h);
        let (decl, target, _, _) = declaration(&h, true);
        let original = inspect(&h, decl);
        let (plan, intent, _, _) = interruption(&h, &original, "after-index", false);
        let donor = h.parent.join("empty donor λ");
        let metadata = PathBuf::from(git(&donor, &["rev-parse", "--absolute-git-dir"]));
        let backup = h.root.join(".chrono-harness/state/original-metadata");
        match fault {
            "visible" => fs::write(target.join("later work"), "preserve").unwrap(),
            "backup-pointer" => fs::write(backup.join("gitfile"), "changed").unwrap(),
            "backup-metadata" => fs::write(backup.join("metadata/index"), "changed").unwrap(),
            "donor-extra" => fs::write(donor.join("later work"), "preserve").unwrap(),
            "lock" => fs::write(metadata.join("locked"), "another owner\n").unwrap(),
            "branch" => fs::write(metadata.join("HEAD"), "ref: refs/heads/dev\n").unwrap(),
            "index" => {
                git(&donor, &["read-tree", original["head"].as_str().unwrap()]);
            }
            "plan" => fs::write(h.root.join(ORIGINAL), "{}").unwrap(),
            "intent" => {
                fs::write(h.root.join(plan["intent"]["path"].as_str().unwrap()), "{}").unwrap()
            }
            "result" => fs::write(
                h.root.join(intent["report_path"].as_str().unwrap()),
                "changed",
            )
            .unwrap(),
            "source" => {
                fs::write(h.root.join("payload"), "new commit").unwrap();
                commit(&h.root);
            }
            "profile" => {
                let mut p = json(&fs::read(h.root.join(POLICY)).unwrap()).unwrap();
                p["timeout_seconds"] = value!(31);
                fs::write(h.root.join(POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            }
            _ => fs::write(metadata.join("index.lock"), "preserve pending writer").unwrap(),
        }
        let old_gitfile = fs::read(target.join(".git")).unwrap();
        let old_index = fs::read(metadata.join("index")).unwrap();
        let (code, r, _) = resumed(&h, &plan);
        assert_ne!(code, 0, "{fault}: {r}");
        assert_eq!(fs::read(target.join(".git")).unwrap(), old_gitfile);
        assert_eq!(fs::read(metadata.join("index")).unwrap(), old_index);
        assert!(donor.exists());
        visible(&target);
        assert!(
            !r["processes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["argv"][1] == "worktree"
                    && [value!("repair"), value!("unlock"), value!("add")].contains(&p["argv"][2]))
        );
    }
}
#[test]
fn metadata_rebind_continuation_can_itself_be_interrupted_and_retried() {
    let h = Host::new("payload");
    wrapper(&h);
    let (decl, target, _, _) = declaration(&h, true);
    let original = inspect(&h, decl);
    let (plan, intent, bytes, result) = interruption(&h, &original, "after-add", false);
    for stage in [
        "after-branch",
        "after-index",
        "after-repair",
        "after-unlock",
    ] {
        fs::write(h.parent.join("rebind-stage"), stage).unwrap();
        let out = invoke(&h, "resume-rebind", CONTINUATION, &plan);
        assert_eq!(out.status.signal(), Some(9), "{stage}: {out:?}");
        fs::remove_file(h.parent.join("rebind-stage")).unwrap();
        visible(&target);
        assert_original(&h, &plan, &intent, &bytes, Some(&result));
    }
    let (code, r, e) = resumed(&h, &plan);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["original_outcome"], "unknown");
    assert_eq!(
        git(&target, &["write-tree"]),
        original["binding"]["index_tree"]
    );
    let (code, retry, e) = resumed(&h, &plan);
    assert_eq!(code, 0, "{retry} {e}");
    assert_original(&h, &plan, &intent, &bytes, Some(&result));
    visible(&target);
}
#[test]
fn metadata_rebind_resume_rejects_completed_results_and_binding_substitution() {
    for fault in [
        "completed",
        "wrong-head",
        "different-intent",
        "extra",
        "absent-payload",
    ] {
        let h = Host::new("payload");
        wrapper(&h);
        let (decl, target, _, _) = declaration(&h, true);
        let original = inspect(&h, decl);
        let (mut plan, intent, _, _) = interruption(&h, &original, "after-add", false);
        match fault {
            "completed" => {
                let (code, r, e) = resumed(&h, &plan);
                assert_eq!(code, 0, "{r} {e}");
                let b = serde_json::to_vec(&r).unwrap();
                fs::write(h.root.join(intent["report_path"].as_str().unwrap()), &b).unwrap();
                plan["result"]["sha256"] = value!(sha256(&b));
            }
            "wrong-head" => plan["head"] = value!("0".repeat(40)),
            "different-intent" => {
                let path = plan["intent"]["path"].as_str().unwrap();
                let mut i = intent.clone();
                i["donor_path"] = value!(h.parent.join("someone else"));
                let b = serde_json::to_vec(&i).unwrap();
                fs::write(h.root.join(path), &b).unwrap();
                plan["intent"]["sha256"] = value!(sha256(&b));
            }
            "extra" => plan["inferred"] = value!(true),
            _ => plan["result"] = value!({"presence":"absent","sha256":"0".repeat(64)}),
        }
        assert_ne!(resumed(&h, &plan).0, 0, "{fault}");
        visible(&target);
    }
}
