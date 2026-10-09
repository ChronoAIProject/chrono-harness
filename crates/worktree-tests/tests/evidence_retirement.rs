use super::*;

fn fixture() -> (Host, PathBuf, Value) {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("historical evidence");
    assert_eq!(h.invoke("integration", "historical-evidence", &target).0, 0);
    let dir = target.join(".chrono-harness/state/custody");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("authority"),
        "current operator release; past failure remains failed",
    )
    .unwrap();
    let mut objects = vec![];
    for id in ["first", "second", "third"] {
        let bytes = format!("original {id} failure evidence\n");
        fs::write(dir.join(format!("{id}-copy")), &bytes).unwrap();
        fs::write(dir.join(format!("{id}-original")), &bytes).unwrap();
        objects.push(value!({"original":{"path":format!(".chrono-harness/state/custody/{id}-copy"),
            "sha256":sha256(bytes.as_bytes()),"length":bytes.len()},
            "survivor":format!(".chrono-harness/state/custody/{id}-original"),
            "owner":"explicit-test-producer","reason":"copied path assertions ended; original diagnosis continues"}));
    }
    let authority = fs::read(dir.join("authority")).unwrap();
    let plan = value!({"schema":"chrono-worktree-maintenance/v1","operation":"retire-evidence",
        "head":git(&target,&["rev-parse","HEAD"]).trim(),
        "custody":{"schema":"chrono-evidence-custody/v1","root":target,
            "branch":git(&target,&["symbolic-ref","HEAD"]).trim(),
            "current_consumers_released":true,"reason":"joined finite historical consumption; current custody",
            "authority":{"path":".chrono-harness/state/custody/authority","sha256":sha256(&authority),"length":authority.len()},
            "exclusion":{"coordinator_root":h.root,"config_path":POLICY,"state_directory":".chrono-harness/state/automatic-cleanup/"},
            "objects":objects}});
    (h, target, plan)
}
fn retire(target: &Path, plan: &Value) -> (i32, Value) {
    let path = ".chrono-harness/state/custody/plan.json";
    fs::write(target.join(path), serde_json::to_vec(plan).unwrap()).unwrap();
    let out = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args([
            "retire-evidence",
            "--host-root",
            target.to_str().unwrap(),
            "--config",
            POLICY,
            "--plan",
            path,
        ])
        .output()
        .unwrap();
    let report = json(&out.stdout).unwrap_or(Value::Null);
    (out.status.code().unwrap_or(-1), report)
}
#[test]
fn released_aliases_retire_without_rewriting_outcomes_source_or_enrollment() {
    let (h, target, plan) = fixture();
    let enrollment = h.ledger();
    let refs = git(&h.root, &["show-ref"]);
    let source = fs::read(target.join("payload")).unwrap();
    let (code, report) = retire(&target, &plan);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["evidence_retirement"]["removed"], 3);
    assert!(report["maintenance_plan"].get("input_bytes").is_none());
    for id in ["first", "second", "third"] {
        assert!(
            !target
                .join(format!(".chrono-harness/state/custody/{id}-copy"))
                .exists()
        );
        assert_eq!(
            fs::read_to_string(target.join(format!(".chrono-harness/state/custody/{id}-original")))
                .unwrap(),
            format!("original {id} failure evidence\n")
        );
    }
    assert_eq!(h.ledger(), enrollment);
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    assert_eq!(fs::read(target.join("payload")).unwrap(), source);
    let receipt = fs::read(target.join(report["report_path"].as_str().unwrap())).unwrap();
    let (code, retry) = retire(&target, &plan);
    assert_eq!(code, 0, "{retry}");
    assert_eq!(retry["evidence_retirement"]["removed"], 0);
    assert_eq!(retry["evidence_retirement"]["observed_absent"], 3);
    assert_eq!(
        fs::read(target.join(report["report_path"].as_str().unwrap())).unwrap(),
        receipt
    );
}
#[test]
fn independent_effects_and_original_failure_survive_partial_retry() {
    let (_h, target, plan) = fixture();
    let missing = target.join(".chrono-harness/state/custody/second-original");
    let raw = fs::read(&missing).unwrap();
    fs::remove_file(&missing).unwrap();
    let (code, failed) = retire(&target, &plan);
    assert_eq!(code, 2, "{failed}");
    assert_eq!(failed["evidence_retirement"]["removed"], 2);
    assert_eq!(failed["evidence_retirement"]["failed"], 1);
    assert!(
        target
            .join(".chrono-harness/state/custody/second-copy")
            .exists()
    );
    let effects = target.join(
        failed["evidence_retirement"]["effects_path"]
            .as_str()
            .unwrap(),
    );
    let original_effects = fs::read(&effects).unwrap();
    fs::write(&missing, raw).unwrap();
    let (code, retry) = retire(&target, &plan);
    assert_eq!(code, 0, "{retry}");
    assert_eq!(retry["evidence_retirement"]["removed"], 1);
    assert_eq!(retry["evidence_retirement"]["observed_absent"], 2);
    assert_eq!(fs::read(effects).unwrap(), original_effects);
}
#[test]
fn live_kernel_consumers_prevent_retirement_until_the_final_holder_joins() {
    let (h, target, plan) = fixture();
    let entry = h.ledger()["entries"][0].clone();
    let held = fs::File::open(h.root.join(entry["ownership"]["path"].as_str().unwrap())).unwrap();
    held.lock_shared().unwrap();
    let (code, report) = retire(&target, &plan);
    assert_eq!(code, 2, "{report}");
    assert!(report["error"].as_str().unwrap().contains("live consumers"));
    assert!(
        target
            .join(".chrono-harness/state/custody/first-copy")
            .exists()
    );
    drop(held);
    assert_eq!(retire(&target, &plan).0, 0);
}
#[test]
fn wrong_identity_release_and_survivor_omissions_preserve_affected_members() {
    for mode in ["release", "hash", "survivor", "source", "staged"] {
        let (_h, target, mut plan) = fixture();
        match mode {
            "release" => plan["custody"]["current_consumers_released"] = value!(false),
            "hash" => fs::write(
                target.join(".chrono-harness/state/custody/first-copy"),
                "changed since release",
            )
            .unwrap(),
            "survivor" => {
                plan["custody"]["objects"][0]["survivor"] =
                    plan["custody"]["objects"][1]["original"]["path"].clone()
            }
            "source" => plan["custody"]["objects"][0]["original"]["path"] = value!("payload"),
            "staged" => {
                git(
                    &target,
                    &["add", "-f", ".chrono-harness/state/custody/first-copy"],
                );
            }
            _ => unreachable!(),
        }
        let (code, report) = retire(&target, &plan);
        assert_eq!(code, 2, "{mode}: {report}");
        assert!(
            target
                .join(".chrono-harness/state/custody/first-copy")
                .exists()
        );
        assert!(target.join("payload").exists());
    }
}
