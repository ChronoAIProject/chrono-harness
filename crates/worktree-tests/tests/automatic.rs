use super::*;
const AUTO_POLICY: &str = ".chrono-harness/cleanup.json";
const STATE: &str = ".chrono-harness/state/automatic-cleanup/ledger.json";

impl Host {
    pub(super) fn automatic(&self, disposition: &str) {
        let mut config = json(&fs::read(self.root.join(CONFIG)).unwrap()).unwrap();
        for artifact in [
            value!({"path":"output λ/","owner":"host","kind":"arbitrary-output","tracked":false}),
            value!({"path":"nested/cache/","owner":"host","kind":"another-layout","tracked":false}),
        ] {
            if !config["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["path"] == artifact["path"])
            {
                config["artifacts"].as_array_mut().unwrap().push(artifact);
            }
        }
        fs::write(
            self.root.join(CONFIG),
            serde_json::to_vec_pretty(&config).unwrap(),
        )
        .unwrap();
        let mut fm = json(&fs::read(self.root.join(FM)).unwrap()).unwrap();
        fm["files"]
            .as_array_mut()
            .unwrap()
            .push(file(AUTO_POLICY, value!([])));
        fs::write(self.root.join(FM), serde_json::to_vec_pretty(&fm).unwrap()).unwrap();
        fs::write(
            self.root.join(AUTO_POLICY),
            serde_json::to_vec_pretty(&value!({
                "schema":"chrono-worktree-automatic-cleanup/v1",
                "coordinator_root":self.root,
                "state_directory":".chrono-harness/state/automatic-cleanup/",
                "retained_ref":"refs/heads/dev", "retention":"ancestor", "remove_branch":false,
                "allow_evidence_disposal":true,
                "artifacts":[
                    {"path":"output λ/","disposition":"dispose"},
                    {"path":"nested/cache/","disposition":"dispose"},
                    {"path":".chrono-harness/state/","disposition":disposition},
                    {"path":".chrono-harness/bin/","disposition":"retain"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            self.root.join(".gitignore"),
            ".chrono-harness/state/\n.chrono-harness/bin/\noutput λ/\nnested/cache/\n",
        )
        .unwrap();
        self.policy(|p| p["automatic_cleanup"] = value!(AUTO_POLICY));
    }
    pub(super) fn auto_command(&self, command: &str, extra: &[&str]) -> Command {
        let mut c = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"));
        c.current_dir(&self.root)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args([
                command,
                "--host-root",
                self.root.to_str().unwrap(),
                "--config",
                POLICY,
            ])
            .args(extra);
        c
    }
    pub(super) fn auto(&self, command: &str, extra: &[&str]) -> (i32, Value, String) {
        self.received(self.auto_command(command, extra).output().unwrap())
    }
    pub(super) fn ledger(&self) -> Value {
        json(&fs::read(self.root.join(STATE)).unwrap()).unwrap()
    }
}
fn output(target: &Path) {
    fs::create_dir_all(target.join("output λ")).unwrap();
    fs::write(target.join("output λ/data"), vec![42u8; 8192]).unwrap();
}

#[test]
fn automatic_birth_enrollment_and_active_retained_checkout_survives_start() {
    let h = Host::new("some language/input.odd");
    h.automatic("evidence-retain");
    let first = h.parent.join("active");
    let (code, r, e) = h.invoke("feature", "active", &first);
    assert_eq!(code, 0, "{r} {e}");
    output(&first);
    let ledger = h.ledger();
    let entry = ledger["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["path"] == value!(first))
        .unwrap();
    assert_eq!(entry["status"], "active");
    assert_eq!(entry["enrollment"]["kind"], "birth");
    assert_eq!(h.invoke("feature", "next", &h.parent.join("next")).0, 0);
    assert!(first.join("output λ/data").exists());
}

#[test]
fn automatic_finish_reclaims_real_checkout_and_reports_measured_outputs() {
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    let target = h.parent.join("finished");
    assert_eq!(h.invoke("integration", "finished", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(
        target.join(".chrono-harness/state/evidence"),
        "explicitly settled",
    )
    .unwrap();
    let (code, r, e) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    let effect = &r["drain"][0]["report"];
    assert_eq!(effect["worktree_removal"], "verified-absent");
    assert_eq!(effect["artifact_disposals"][0]["bytes_before"], 8192);
    assert_eq!(effect["artifact_disposals"][0]["bytes_after"], 0);
    assert!(git(&h.root, &["show-ref"]).contains("refs/heads/integration/finished"));
    assert_eq!(h.auto("maintain", &[]).0, 0);
}

fn entry_receipts(h: &Host, entry: &Value) -> Vec<(PathBuf, Vec<u8>)> {
    let evidence = std::env::var("CHRONO_WORKTREE_TEST_RECEIPTS")
        .ok()
        .map(|p| PathBuf::from(p).join("original-lifecycle-state"));
    if let Some(directory) = &evidence {
        fs::create_dir_all(directory).unwrap();
        let ledger = fs::read(h.root.join(STATE)).unwrap();
        fs::write(
            directory.join(format!("ledger-{}.json", sha256(&ledger))),
            ledger,
        )
        .unwrap();
    }
    let mut receipts = vec![entry["terminal"]["receipt"].clone()];
    if !entry["enrollment"]["sealed_receipt"].is_null() {
        receipts.push(entry["enrollment"]["sealed_receipt"].clone());
    }
    receipts.extend(
        entry["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["receipt"].clone()),
    );
    receipts
        .iter()
        .map(|r| {
            let path = h.root.join(r["path"].as_str().unwrap());
            let bytes = fs::read(&path).unwrap();
            assert_eq!(r["sha256"], sha256(&bytes));
            if let Some(directory) = &evidence {
                fs::write(
                    directory.join(format!("receipt-{}.json", sha256(&bytes))),
                    &bytes,
                )
                .unwrap();
            }
            (path, bytes)
        })
        .collect()
}

#[test]
fn automatic_disposed_path_reuse_targets_new_birth_and_import_generations() {
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    consuming_operation(
        &h,
        "mkdir -p 'output λ'\nprintf managed-output > 'output λ/use'\ngit symbolic-ref --short HEAD\n",
    );
    let target = h.parent.join("reused");
    let args = ["--path", target.to_str().unwrap(), "--dispose-evidence"];
    let (code, r, e) = h.invoke("feature", "first-task", &target);
    assert_eq!(code, 0, "{r} {e}");
    output(&target);
    let (code, r, e) = h.auto("finish", &args);
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    assert_eq!(
        r["drain"][0]["report"]["artifact_disposals"][0]["bytes_before"],
        8192
    );
    assert_eq!(
        r["drain"][0]["report"]["artifact_disposals"][0]["bytes_after"],
        0
    );
    let first = h.ledger()["entries"][0].clone();
    assert_eq!(first["status"], "disposed");
    assert!(!first["attempts"].as_array().unwrap().is_empty());
    let mut originals = entry_receipts(&h, &first);

    let (code, r, e) = h.invoke("feature", "second-task", &target);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["drain"], value!([]));
    let ledger = h.ledger();
    assert_eq!(ledger["entries"].as_array().unwrap().len(), 2);
    assert_eq!(ledger["entries"][0], first);
    assert_eq!(ledger["entries"][1]["branch"], "feature/second-task");
    assert_ne!(
        ledger["entries"][1]["enrollment"]["receipt"],
        first["enrollment"]["receipt"]
    );
    for branch in ["second-task", "third-task"] {
        let (code, r, e) = h.auto(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        );
        assert_eq!(code, 0, "{r} {e}");
        assert_eq!(
            r["managed_process"]["stdout"],
            format!("feature/{branch}\n")
        );
        assert_eq!(
            fs::read(target.join("output λ/use")).unwrap(),
            b"managed-output"
        );
        originals.push((
            h.root
                .join(r["managed_use_receipt"]["path"].as_str().unwrap()),
            fs::read(
                h.root
                    .join(r["managed_use_receipt"]["path"].as_str().unwrap()),
            )
            .unwrap(),
        ));
        let (code, r, e) = h.auto("finish", &args);
        assert_eq!(code, 0, "{r} {e}");
        assert!(!target.exists());
        assert_eq!(r["drain"].as_array().unwrap().len(), 1);
        assert_eq!(
            r["drain"][0]["report"]["branch_ref"],
            format!("feature/{branch}")
        );
        assert_eq!(
            r["drain"][0]["report"]["worktree_removal"],
            "verified-absent"
        );
        assert_eq!(h.ledger()["entries"][0], first);
        let entries = h.ledger()["entries"].as_array().unwrap().clone();
        let retired = entries.last().unwrap().clone();
        assert_eq!(retired["status"], "disposed");
        assert_eq!(retired["uses"], value!([]));
        originals.extend(entry_receipts(&h, &retired));
        let (code, r, e) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{r} {e}");
        assert_eq!(r["drain"], value!([]));
        assert_eq!(h.ledger()["entries"], value!(entries));
        for (path, bytes) in &originals {
            assert_eq!(&fs::read(path).unwrap(), bytes);
        }
        if branch == "second-task" {
            git(
                &h.root,
                &[
                    "worktree",
                    "add",
                    "-b",
                    "feature/third-task",
                    target.to_str().unwrap(),
                    "dev",
                ],
            );
            let (code, r, e) = h.auto("import", &["--path", target.to_str().unwrap()]);
            assert_eq!(code, 0, "{r} {e}");
            assert_eq!(h.ledger()["entries"][2]["enrollment"]["kind"], "import");
            assert_eq!(
                &h.ledger()["entries"].as_array().unwrap()[..2],
                entries.as_slice()
            );
        }
    }
}

#[test]
fn automatic_absent_active_enrollment_conflict_preflights_without_creation() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let target = h.parent.join("unresolved");
    assert_eq!(h.invoke("feature", "original-task", &target).0, 0);
    let original = h.ledger();
    // External removal does not establish a terminal handoff in this owner.
    git(
        &h.root,
        &["worktree", "remove", "--force", target.to_str().unwrap()],
    );
    assert!(!target.exists());
    let refs = git(&h.root, &["show-ref"]);
    let inventory = git(&h.root, &["worktree", "list", "--porcelain"]);
    let (code, r, e) = h.invoke("feature", "conflicting-task", &target);
    assert_ne!(code, 0, "{r} {e}");
    assert!(
        r["error"].as_str().unwrap().contains("already enrolled"),
        "{r} {e}"
    );
    assert!(
        !target.exists(),
        "registration conflict must precede checkout creation: {r}"
    );
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    assert_eq!(
        git(&h.root, &["worktree", "list", "--porcelain"]),
        inventory
    );
    assert_eq!(h.ledger(), original);
}

#[test]
fn automatic_cache_only_preserves_evidence_dirty_and_unretained_work() {
    for reason in ["evidence", "dirty", "unretained"] {
        let h = Host::new("payload");
        h.automatic("evidence-retain");
        let target = h.parent.join("preserved");
        assert_eq!(h.invoke("feature", "preserved", &target).0, 0);
        output(&target);
        match reason {
            "evidence" => {
                fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
                fs::write(target.join(".chrono-harness/state/receipt"), "needed").unwrap();
            }
            "dirty" => fs::write(target.join("payload"), "unsaved").unwrap(),
            _ => {
                fs::write(target.join("payload"), "unretained commit").unwrap();
                commit(&target);
            }
        }
        let args = if reason == "evidence" {
            vec!["--path", target.to_str().unwrap()]
        } else {
            vec!["--path", target.to_str().unwrap(), "--dispose-evidence"]
        };
        let (code, r, e) = h.auto("finish", &args);
        assert_eq!(code, 0, "{reason}: {r} {e}");
        assert!(target.exists());
        assert!(!target.join("output λ").exists());
        assert!(r["drain"][0]["report"]["preserved_reason"].is_string());
        if reason == "evidence" {
            assert!(target.join(".chrono-harness/state/receipt").exists());
        }
    }
}

#[test]
fn automatic_import_is_explicit_and_not_a_historical_birth() {
    let h = Host::new("payload");
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["artifacts"].as_array_mut().unwrap().extend([
        value!({"path":"output λ/","owner":"host","kind":"arbitrary-output","tracked":false}),
        value!({"path":"nested/cache/","owner":"host","kind":"another-layout","tracked":false}),
    ]);
    fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
    h.policy(|_| ());
    let target = h.parent.join("legacy");
    assert_eq!(h.invoke("feature", "legacy", &target).0, 0);
    h.automatic("evidence-retain");
    // Import the unchanged legacy checkout, without inventing a birth or rewriting its source.
    assert!(!target.join(AUTO_POLICY).exists());
    assert_ne!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    let (code, r, e) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(h.ledger()["entries"][0]["enrollment"]["kind"], "import");
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert!(!target.exists());
}

fn blocked_remove(h: &Host, after_effect: bool) {
    use std::os::unix::fs::PermissionsExt;
    let real = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let wrapper = h.parent.join("automatic-git");
    let effect = if after_effect {
        format!("'{}' \"$@\" || exit $?;", real.display())
    } else {
        String::new()
    };
    fs::write(&wrapper,format!("#!/bin/sh\nif [ -f \"$HOME/fail-remove\" ] && [ \"$2\" = worktree ] && [ \"$3\" = remove ]; then\n{effect}\nprintf 'original automatic removal failure\\n' >&2\nexit 71\nfi\nexec '{}' \"$@\"\n",real.display())).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    h.policy(|p| p["git"]["program"] = value!(wrapper));
}

#[test]
fn automatic_partial_failure_original_reporting_and_later_start_retry() {
    for after_effect in [false, true] {
        let h = Host::new("payload");
        blocked_remove(&h, after_effect);
        h.automatic("evidence-retain");
        let target = h.parent.join("pending");
        assert_eq!(h.invoke("feature", "pending", &target).0, 0);
        output(&target);
        fs::write(h.parent.join("fail-remove"), "one failure").unwrap();
        let (code, failed, e) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_ne!(code, 0, "{failed} {e}");
        let original = &failed["drain"][0]["report"];
        assert_eq!(original["status"], "failed");
        assert_eq!(original["worktree_removal"], "attempted-unverified");
        assert_eq!(original["worktree_removed"], false);
        assert_eq!(
            original["artifact_disposals"][0]["status"],
            "verified-absent"
        );
        assert_eq!(target.exists(), !after_effect);
        assert!(original["processes"].as_array().unwrap().iter().any(|p| {
            p["process"]["exit_code"] == 71
                && p["process"]["stderr"]
                    .as_str()
                    .unwrap_or("")
                    .contains("original automatic removal failure")
        }));
        let receipt = h.root.join(original["report_path"].as_str().unwrap());
        let bytes = fs::read(&receipt).unwrap();
        let next = h.parent.join("next");
        if !after_effect {
            let independent = h.parent.join("independent");
            let (code, admitted, error) = h.invoke("feature", "independent", &independent);
            assert_eq!(code, 0, "{} {error}", admitted["error"]);
            assert!(independent.exists());
            assert!(!admitted["cleanup_failures"].as_array().unwrap().is_empty());
            assert_eq!(admitted["drain"][0]["report"]["status"], "failed");
            assert_eq!(fs::read(&receipt).unwrap(), bytes);
        }
        fs::remove_file(h.parent.join("fail-remove")).unwrap();
        let (code, r, e) = h.invoke("feature", "next", &next);
        assert_eq!(code, 0, "{r} {e}");
        assert!(!target.exists());
        assert!(next.exists());
        assert_eq!(fs::read(receipt).unwrap(), bytes);
        assert_eq!(h.auto("maintain", &[]).0, 0);
    }
}

#[test]
fn automatic_symlink_containment_and_internal_external_target_safety() {
    use std::os::unix::fs::symlink;
    for fault in ["internal", "ancestor", "selected-root"] {
        let h = Host::new("payload");
        h.automatic("evidence-retain");
        let target = h.parent.join("links");
        assert_eq!(h.invoke("feature", "links", &target).0, 0);
        output(&target);
        let outside = h.parent.join("external-data");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep"), "outside").unwrap();
        match fault {
            "internal" => symlink(&outside, target.join("output λ/link")).unwrap(),
            "ancestor" => symlink(&outside, target.join("nested")).unwrap(),
            _ => {
                fs::remove_dir_all(target.join("output λ")).unwrap();
                symlink(&outside, target.join("output λ")).unwrap();
            }
        }
        let (code, r, e) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_eq!(fs::read(outside.join("keep")).unwrap(), b"outside");
        if fault == "internal" {
            assert_eq!(code, 0, "{r} {e}");
            assert!(!target.exists());
        } else {
            assert_ne!(code, 0, "{r} {e}");
            assert!(target.exists());
            if fault == "ancestor" {
                assert!(
                    target.join("output λ/data").exists(),
                    "whole selection must preflight before deletion"
                );
            }
        }
    }
}

pub(super) fn consuming_operation(h: &Host, body: &str) {
    let mut projects = json(&fs::read(h.root.join(PROJECTS)).unwrap()).unwrap();
    projects["owners"]
        .as_array_mut()
        .unwrap()
        .extend([value!("consumer"), value!("consumer-tests")]);
    projects["scripts"] = value!([
        {"id":"consumer","path":"s.sh","test_script":"consumer-tests","actions":{"execute":{"operation":"use.consumer","tool":"sh","argv":["s.sh"]}}},
        {"id":"consumer-tests","path":"st.sh","tests_for":"consumer","actions":{"execute":{"operation":"test.consumer","tool":"sh","argv":["st.sh"]}}}
    ]);
    fs::write(
        h.root.join(PROJECTS),
        serde_json::to_vec(&projects).unwrap(),
    )
    .unwrap();
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    fm["files"]
        .as_array_mut()
        .unwrap()
        .extend([file("s.sh", value!([])), file("st.sh", value!([]))]);
    fm["project_edges"] = value!([edge(
        "script:consumer",
        "test-execution",
        "test:consumer-tests"
    )]);
    fm["test_costs"] = value!([{"test":"consumer-tests","cost":"unknown"}]);
    fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    fs::write(h.root.join("s.sh"), body).unwrap();
    fs::write(h.root.join("st.sh"), "exit 0\n").unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
}

#[test]
fn automatic_managed_real_child_blocks_finish_and_exit_releases_use() {
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    consuming_operation(
        &h,
        "mkdir -p .chrono-harness/state\nprintf ready > .chrono-harness/state/ready\nwhile [ ! -f .chrono-harness/state/release ]; do sleep 0.02; done\nprintf joined-child\n",
    );
    let target = h.parent.join("in-use");
    assert_eq!(h.invoke("feature", "in-use", &target).0, 0);
    output(&target);
    let mut child = h
        .auto_command(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let began = Instant::now();
    while !target.join(".chrono-harness/state/ready").exists()
        && began.elapsed() < Duration::from_secs(10)
    {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let ready = target.join(".chrono-harness/state/ready").exists();
    // Join the wrapper before asserting, including its failure path. A missing
    // readiness marker must expose the real wrapper result and retain its receipt.
    let finish = ready.then(|| {
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        )
    });
    let output_retained = target.join("output λ/data").exists();
    let uses = h.ledger()["entries"][0]["uses"].clone();
    let release = fs::write(target.join(".chrono-harness/state/release"), "join now");
    let (code, r, e) = h.received(child.wait_with_output().unwrap());
    assert!(
        ready,
        "managed child did not become ready: exit {code}; {r} {e}"
    );
    release.unwrap();
    let (finish_code, finish_report, finish_error) = finish.unwrap();
    assert_ne!(finish_code, 0, "{finish_report} {finish_error}");
    assert!(output_retained);
    assert_eq!(uses.as_array().unwrap().len(), 1);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["managed_process"]["stdout"], "joined-child");
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    let (code, r, e) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
}

#[test]
fn automatic_terminal_rejects_changed_head_ref_policy_attachment_and_git_locks() {
    for fault in [
        "head",
        "ref",
        "policy",
        "target-policy",
        "target-config",
        "attachment",
        "git-lock",
        "worktree-lock",
    ] {
        let h = Host::new("payload");
        blocked_remove(&h, false);
        h.automatic("evidence-retain");
        let target = h.parent.join("drift");
        assert_eq!(h.invoke("feature", "drift", &target).0, 0);
        output(&target);
        fs::write(h.parent.join("fail-remove"), "fail").unwrap();
        assert_ne!(
            h.auto(
                "finish",
                &["--path", target.to_str().unwrap(), "--dispose-evidence"]
            )
            .0,
            0
        );
        fs::remove_file(h.parent.join("fail-remove")).unwrap();
        output(&target);
        match fault {
            "head" => {
                fs::write(target.join("payload"), "changed head").unwrap();
                commit(&target);
            }
            "ref" => {
                git(
                    &h.root,
                    &[
                        "-c",
                        "user.name=fixture",
                        "-c",
                        "user.email=fixture@example.invalid",
                        "commit",
                        "--allow-empty",
                        "-qm",
                        "advanced reference",
                    ],
                );
            }
            "policy" => {
                let mut p = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
                p["remove_branch"] = value!(true);
                fs::write(h.root.join(AUTO_POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            }
            "target-policy" => {
                let mut p = json(&fs::read(target.join(AUTO_POLICY)).unwrap()).unwrap();
                p["remove_branch"] = value!(true);
                fs::write(target.join(AUTO_POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            }
            "target-config" => {
                let mut p = json(&fs::read(target.join(POLICY)).unwrap()).unwrap();
                p["output_limit_bytes"] = value!(8192);
                fs::write(target.join(POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            }
            "attachment" => {
                fs::rename(target.join(".git"), h.parent.join("original-gitfile")).unwrap();
                fs::write(
                    target.join(".git"),
                    fs::read(h.parent.join("original-gitfile")).unwrap(),
                )
                .unwrap();
            }
            "git-lock" => {
                let metadata = git(&target, &["rev-parse", "--absolute-git-dir"]);
                fs::write(Path::new(&metadata).join("index.lock"), "owned").unwrap();
            }
            _ => {
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
        }
        let (code, r, e) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{fault}: {r} {e}");
        assert!(target.exists());
        assert!(target.join("output λ/data").exists());
    }
}

#[test]
fn automatic_reconstruct_enrolls_only_success_and_unresolved_evidence_can_later_settle() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let head = git(&h.root, &["rev-parse", "HEAD"]);
    let (code, r, e) = h.reconstruct(reconstruction(&head, &head, value!([])), "reborn");
    assert_eq!(code, 0, "{r} {e}");
    let target = h.parent.join("reborn");
    output(&target);
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["receipt"]["operation"],
        "reconstruct"
    );
    assert_eq!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    assert!(target.exists());
    assert_eq!(h.ledger()["entries"][0]["status"], "retained");
    let (code, r, e) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["drain"], value!([]));
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert!(!target.exists());
}

#[test]
fn automatic_same_tree_retention_remains_exact_and_branch_removal_is_opt_in() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let mut policy = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
    policy["retention"] = value!("same-tree");
    policy["remove_branch"] = value!(true);
    fs::write(
        h.root.join(AUTO_POLICY),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("squash");
    assert_eq!(h.invoke("integration", "squash", &target).0, 0);
    fs::write(target.join("payload"), "landed work").unwrap();
    let candidate = commit(&target);
    fs::write(h.root.join("payload"), "landed work").unwrap();
    git(&h.root, &["add", "payload"]);
    git(
        &h.root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-qm",
            "verified squash landing",
        ],
    );
    let landed = git(&h.root, &["rev-parse", "HEAD"]);
    assert_ne!(candidate, landed);
    output(&target);
    let (code, r, e) = h.auto(
        "finish",
        &[
            "--path",
            target.to_str().unwrap(),
            "--dispose-evidence",
            "--retained-commit",
            &landed,
        ],
    );
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    assert!(
        !git(&h.root, &["for-each-ref", "--format=%(refname)"])
            .contains("refs/heads/integration/squash")
    );
    assert_eq!(git(&h.root, &["rev-parse", "dev"]), landed);
}

#[test]
fn automatic_filesystem_partial_failure_retries_under_original_owned_lock() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let target = h.parent.join("filesystem-failure");
    assert_eq!(h.invoke("feature", "filesystem-failure", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(target.join("nested/cache/keep-until-retry"), "cache").unwrap();
    let special = target.join("nested/cache/special");
    assert!(
        Command::new("mkfifo")
            .arg(&special)
            .status()
            .unwrap()
            .success()
    );
    let (code, r, e) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_ne!(code, 0, "{r} {e}");
    let original = &r["drain"][0]["report"];
    assert_eq!(
        original["artifact_disposals"][0]["status"],
        "verified-absent"
    );
    assert_eq!(original["worktree_removal"], "not-attempted");
    assert!(target.exists());
    assert!(!target.join("output λ").exists());
    assert!(special.exists());
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"])
            .contains(original["lock_reason"].as_str().unwrap())
    );
    let receipt = h.root.join(original["report_path"].as_str().unwrap());
    let bytes = fs::read(&receipt).unwrap();
    fs::remove_file(special).unwrap();
    let (code, r, e) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    assert_eq!(fs::read(receipt).unwrap(), bytes);
    assert_eq!(
        r["drain"][0]["report"]["artifact_disposals"][0]["status"],
        "already-absent"
    );
}

#[test]
fn automatic_failed_birth_and_missing_state_are_protected() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    h.hook("printf original-birth-failure >&2\nexit 17");
    let failed = h.parent.join("failed");
    assert_ne!(h.invoke("feature", "failed", &failed).0, 0);
    assert!(failed.exists());
    assert_eq!(h.ledger()["entries"], value!([]));
    assert_ne!(h.auto("finish", &["--path", failed.to_str().unwrap()]).0, 0);
    assert!(failed.exists());
    fs::remove_file(h.root.join(".git/hooks/post-checkout")).unwrap();
    let active = h.parent.join("active");
    assert_eq!(h.invoke("feature", "active", &active).0, 0);
    output(&active);
    fs::remove_file(h.root.join(STATE)).unwrap();
    let next = h.parent.join("next");
    assert_ne!(h.invoke("feature", "next", &next).0, 0);
    assert!(!next.exists());
    assert!(active.join("output λ/data").exists());
}

#[test]
fn automatic_explicit_main_anchor_short_commands_and_finish_from_source_then_start() {
    let h = Host::new("layout/source.ext");
    h.automatic("evidence-retain");
    let mut policy = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
    policy["coordinator_root"] = value!("git-main-worktree");
    fs::write(
        h.root.join(AUTO_POLICY),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("self-finish");
    assert_eq!(h.invoke("feature", "self-finish", &target).0, 0);
    output(&target);
    let out = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
        .current_dir(&target)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args(["finish", "--dispose-evidence"])
        .output()
        .unwrap();
    let (code, r, e) = h.received(out);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(
        r["drain"][0]["preserved_reason"],
        "invoking source or destination"
    );
    assert!(target.exists());
    let next = h.parent.join("fresh");
    let (code, r, e) = h.invoke("feature", "fresh", &next);
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    let (code, r, e) = h.received(
        Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .current_dir(&h.root)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .arg("maintain")
            .output()
            .unwrap(),
    );
    assert_eq!(code, 0, "{r} {e}");
    assert!(next.exists());
}

#[test]
fn automatic_staged_source_inside_artifact_and_unauthorized_evidence_remain() {
    for fault in ["staged-source", "evidence-authorization", "gate"] {
        let h = Host::new("payload");
        h.automatic("evidence-retain");
        if fault == "evidence-authorization" {
            let mut p = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
            p["allow_evidence_disposal"] = value!(false);
            fs::write(h.root.join(AUTO_POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            commit(&h.root);
            git(&h.root, &["push", "-q", "warehouse", "dev"]);
        }
        let target = h.parent.join("protected");
        assert_eq!(h.invoke("feature", "protected", &target).0, 0);
        output(&target);
        if fault == "staged-source" {
            git(&target, &["add", "-f", "output λ/data"]);
        }
        if fault == "gate" {
            fs::create_dir(
                h.root
                    .join(".chrono-harness/state/automatic-cleanup/gate.lock"),
            )
            .unwrap();
        }
        let (code, r, e) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_ne!(code, 0, "{fault}: {r} {e}");
        assert!(target.join("output λ/data").exists());
    }
}

#[test]
fn automatic_interrupted_managed_wrapper_keeps_unknown_use_protected() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    consuming_operation(
        &h,
        "mkdir -p .chrono-harness/state\nprintf '%s' \"$$\" > .chrono-harness/state/ready\nwhile [ ! -f .chrono-harness/state/release ]; do sleep 0.02; done\n",
    );
    let target = h.parent.join("unknown-use");
    assert_eq!(h.invoke("feature", "unknown-use", &target).0, 0);
    output(&target);
    let mut wrapper = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        ),
        &h.root,
    );
    wrapper.await_file(&target.join(".chrono-harness/state/ready"));
    let child_pid = fs::read_to_string(target.join(".chrono-harness/state/ready")).unwrap();
    wrapper.kill().unwrap();
    let out = wrapper.wait_with_output().unwrap();
    assert_ne!(h.received(out).0, 0);
    // Join exactly the test-owned process group; no process scanning or age heuristic.
    assert!(
        Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{child_pid}")])
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(
        h.ledger()["entries"][0]["uses"].as_array().unwrap().len(),
        1
    );
    assert_ne!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/data").exists());
}

#[test]
fn automatic_concurrent_managed_completions_release_both_uses() {
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    consuming_operation(
        &h,
        "mkdir -p .chrono-harness/state\nwhile [ ! -f .chrono-harness/state/release ]; do sleep 0.02; done\nprintf completed\n",
    );
    let target = h.parent.join("parallel-use");
    assert_eq!(h.invoke("feature", "parallel-use", &target).0, 0);
    let mut children = vec![];
    for _ in 0..2 {
        children.push(
            h.auto_command(
                "use",
                &[
                    "--path",
                    target.to_str().unwrap(),
                    "--operation",
                    "use.consumer",
                ],
            )
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
        );
    }
    let began = Instant::now();
    while h.ledger()["entries"][0]["uses"].as_array().unwrap().len() != 2
        && began.elapsed() < Duration::from_secs(30)
    {
        thread::sleep(Duration::from_millis(20));
    }
    let admitted = h.ledger()["entries"][0]["uses"].as_array().unwrap().len();
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(target.join(".chrono-harness/state/release"), "joined").unwrap();
    for child in children {
        let (code, r, e) = h.received(child.wait_with_output().unwrap());
        assert_eq!(code, 0, "{r} {e}");
    }
    assert_eq!(
        admitted, 2,
        "both live consumers must enroll before release"
    );
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert!(!target.exists());
}

#[test]
fn automatic_use_from_checkout_publishes_at_surviving_anchor() {
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    consuming_operation(&h, "printf finished\n");
    let target = h.parent.join("local-use");
    assert_eq!(h.invoke("feature", "local-use", &target).0, 0);
    let out = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
        .current_dir(&target)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args(["use", "--operation", "use.consumer"])
        .output()
        .unwrap();
    let (code, r, e) = h.received(out);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["source_root"], value!(h.root));
    assert_eq!(r["invoking_root"], value!(target));
    let receipt = h.root.join(r["report_path"].as_str().unwrap());
    let bytes = fs::read(&receipt).unwrap();
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert!(!target.exists());
    assert_eq!(fs::read(receipt).unwrap(), bytes);
}
