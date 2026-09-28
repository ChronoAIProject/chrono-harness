#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::{json, sha256};
use serde_json::{Value, json as value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use support::*;
const POLICY: &str = ".chrono-harness/worktree.json";

struct Host {
    _dir: tempfile::TempDir,
    root: PathBuf,
    remote: PathBuf,
    parent: PathBuf,
}
impl Host {
    fn new(layout: &str) -> Self {
        let dir = tempfile::Builder::new()
            .prefix("worktree host λ ")
            .tempdir()
            .unwrap();
        let parent = fs::canonicalize(dir.path()).unwrap();
        let root = parent.join("source with spaces");
        let remote = parent.join("declared upstream.git");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&remote).unwrap();
        git(&remote, &["init", "--bare", "--initial-branch=dev", "-q"]);
        git(&root, &["init", "--initial-branch=dev", "-q"]);
        let mut v = values();
        for x in v.values_mut() {
            x["status"] = value!("proposed");
        }
        v.get_mut(CONFIG).unwrap()["enforcement"] = value!("not-implemented");
        v.get_mut(CONFIG).unwrap()["input_closure"] = value!({"status":"incomplete","unresolved":["fixture does not certify complete inputs"]});
        v.get_mut(PROJECTS).unwrap()["owners"] = value!(["host"]);
        v.get_mut(PROJECTS).unwrap()["projects"] = value!([]);
        let paths = [
            CONFIG,
            FM,
            PROJECTS,
            JUDGES,
            WORKFLOW,
            POLICY,
            ".gitignore",
            layout,
        ];
        v.get_mut(FM).unwrap()["files"] = value!(
            paths
                .iter()
                .map(|p| file(p, value!([])))
                .collect::<Vec<_>>()
        );
        v.get_mut(FM).unwrap()["project_edges"] = value!([]);
        v.get_mut(FM).unwrap()["test_costs"] = value!([]);
        write_values(&root, &v);
        fs::write(root.join(".gitignore"), ".chrono-harness/state/\n").unwrap();
        fs::create_dir_all(root.join(layout).parent().unwrap()).unwrap();
        fs::write(
            root.join(layout),
            "literal host input; no dependency inference\n",
        )
        .unwrap();
        let policy = value!({"schema":"chrono-worktree-config/v1","host_config":CONFIG,"remote":"warehouse","git":{"program":"git","expected_version":null,"sha256":null},"environment":{"inherit":["PATH"],"values":{"HOME":parent,"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null","GIT_TERMINAL_PROMPT":"0"}},"timeout_seconds":30,"output_limit_bytes":1048576,"report_directory":".chrono-harness/state/worktrees/"});
        fs::write(
            root.join(POLICY),
            serde_json::to_vec_pretty(&policy).unwrap(),
        )
        .unwrap();
        commit(&root);
        git(
            &root,
            &["remote", "add", "warehouse", remote.to_str().unwrap()],
        );
        git(&root, &["push", "-q", "warehouse", "dev"]);
        Self {
            _dir: dir,
            root,
            remote,
            parent,
        }
    }
    fn policy(&self, f: impl FnOnce(&mut Value)) {
        let mut p = json(&fs::read(self.root.join(POLICY)).unwrap()).unwrap();
        f(&mut p);
        fs::write(
            self.root.join(POLICY),
            serde_json::to_vec_pretty(&p).unwrap(),
        )
        .unwrap();
        commit(&self.root);
        git(&self.root, &["push", "-q", "warehouse", "dev"]);
    }
    fn invoke(&self, kind: &str, name: &str, target: &Path) -> (i32, Value, String) {
        let out = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .current_dir("/")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("UNDECLARED_WORKTREE_ENV", "must-not-leak")
            .args([
                "start",
                "--host-root",
                self.root.to_str().unwrap(),
                "--config",
                POLICY,
                "--kind",
                kind,
                "--name",
                name,
                "--path",
                target.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        let report = json(&out.stdout).unwrap_or(Value::Null);
        if !report.is_null() {
            assert_eq!(
                report,
                json(&fs::read(self.root.join(report["report_path"].as_str().unwrap())).unwrap())
                    .unwrap()
            );
            assert_eq!(report["governance"], "not-evaluated");
            assert_eq!(report["parity"], "unestablished");
            assert!(
                report["environment"]["effective"]
                    .get("UNDECLARED_WORKTREE_ENV")
                    .is_none()
            );
            for row in report["processes"].as_array().unwrap() {
                if row["process"].is_null() {
                    continue;
                }
                for stream in ["stdout", "stderr"] {
                    let bytes: Vec<u8> =
                        serde_json::from_value(row["process"][format!("{stream}_bytes")].clone())
                            .unwrap();
                    assert_eq!(row["process"][format!("{stream}_sha256")], sha256(&bytes));
                }
            }
        }
        (
            out.status.code().unwrap(),
            report,
            String::from_utf8_lossy(&out.stderr).into(),
        )
    }
    fn hook(&self, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        let p = self.root.join(".git/hooks/post-checkout");
        fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn fetches_advanced_registered_target_preserves_dirty_source_and_explicit_layouts() {
    for layout in ["deep/engine/rates.go", "unrelated/client/entry.mts"] {
        let h = Host::new(layout);
        let old = git(&h.root, &["rev-parse", "HEAD"]);
        let other = h.parent.join("other writer");
        git(
            &h.root,
            &[
                "clone",
                "-q",
                h.remote.to_str().unwrap(),
                other.to_str().unwrap(),
            ],
        );
        fs::write(other.join(layout), "new target content\n").unwrap();
        let latest = commit(&other);
        git(&other, &["push", "-q", "origin", "dev"]);
        fs::write(h.root.join(layout), "unsaved original work\n").unwrap();
        let target = h.parent.join("literal $(touch ignored) λ");
        let (code, r, e) = h.invoke("feature", "task", &target);
        assert_eq!(code, 0, "{r} {e}");
        assert_eq!(r["base"], latest);
        assert_eq!(r["context"]["candidate"], latest);
        assert_eq!(r["status"], "created");
        assert_eq!(r["fetch_ref_removed"], true);
        assert_eq!(git(&target, &["rev-parse", "HEAD"]), latest);
        assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), old);
        assert_eq!(
            fs::read_to_string(h.root.join(layout)).unwrap(),
            "unsaved original work\n"
        );
        assert_eq!(
            fs::read_to_string(target.join(layout)).unwrap(),
            "new target content\n"
        );
        assert_eq!(
            git(
                &h.root,
                &[
                    "for-each-ref",
                    "--format=%(refname)",
                    "refs/chrono-harness/fetch/"
                ]
            ),
            ""
        );
        assert!(!target.join("Cargo.toml").exists() && !target.join(".lake").exists());
    }
}
#[test]
fn uses_registered_prefix_and_disables_inherited_tracking_without_changing_config() {
    let h = Host::new("payload");
    let mut w = json(&fs::read(h.root.join(WORKFLOW)).unwrap()).unwrap();
    w["integration_prefix"] = value!("trial/");
    fs::write(h.root.join(WORKFLOW), serde_json::to_vec(&w).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    git(&h.root, &["config", "branch.autoSetupMerge", "always"]);
    git(&h.root, &["config", "branch.autoSetupRebase", "always"]);
    let config = fs::read(h.root.join(".git/config")).unwrap();
    let target = h.parent.join("new trial");
    let (code, r, e) = h.invoke("integration", "sample", &target);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["branch_ref"], "trial/sample");
    assert_eq!(
        git(&target, &["symbolic-ref", "HEAD"]),
        "refs/heads/trial/sample"
    );
    assert_eq!(fs::read(h.root.join(".git/config")).unwrap(), config);
    assert!(git(&h.root, &["worktree", "list", "--porcelain"]).contains("trial/sample"));
}
#[test]
fn preserves_existing_destination_and_branch_instead_of_resetting_them() {
    let h = Host::new("payload");
    let target = h.parent.join("keep");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("sentinel"), "keep").unwrap();
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert!(r["error"].as_str().unwrap().contains("already exists"));
    assert_eq!(fs::read_to_string(target.join("sentinel")).unwrap(), "keep");
    let first = h.parent.join("first");
    assert_eq!(h.invoke("feature", "task", &first).0, 0);
    let head = git(&first, &["rev-parse", "HEAD"]);
    fs::write(first.join("payload"), "dirty owned work").unwrap();
    git(&h.root, &["worktree", "lock", first.to_str().unwrap()]);
    let next = h.parent.join("next");
    let (code, r, _) = h.invoke("feature", "task", &next);
    assert_ne!(code, 0);
    assert!(
        r["error"]
            .as_str()
            .unwrap()
            .contains("branch already exists")
    );
    assert!(!next.exists());
    assert_eq!(git(&first, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        fs::read_to_string(first.join("payload")).unwrap(),
        "dirty owned work"
    );
}
#[test]
fn rejects_nested_destinations_and_unregistered_or_changed_policy() {
    let h = Host::new("payload");
    let (code, r, _) = h.invoke("feature", "task", &h.root.join("nested"));
    assert_ne!(code, 0);
    assert!(r["error"].as_str().unwrap().contains("overlaps"));
    let mut p = fs::read(h.root.join(POLICY)).unwrap();
    p.push(b' ');
    fs::write(h.root.join(POLICY), &p).unwrap();
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert!(r["error"].as_str().unwrap().contains("source commit"));
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    fm["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["path"] != POLICY);
    fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert!(r["error"].as_str().unwrap().contains("FILEMAP"));
    assert!(!target.exists());
}
#[test]
fn remote_policy_change_requires_adoption_before_creating_a_branch() {
    let h = Host::new("payload");
    let other = h.parent.join("writer");
    git(
        &h.root,
        &[
            "clone",
            "-q",
            h.remote.to_str().unwrap(),
            other.to_str().unwrap(),
        ],
    );
    let p = other.join(POLICY);
    let mut b = fs::read(&p).unwrap();
    b.push(b'\n');
    fs::write(p, b).unwrap();
    commit(&other);
    git(&other, &["push", "-q", "origin", "dev"]);
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert!(
        r["error"]
            .as_str()
            .unwrap()
            .contains("fetched target changed")
    );
    assert!(!target.exists());
}
#[test]
fn retains_actual_failed_fetch_and_version_observations() {
    let h = Host::new("payload");
    h.policy(|p| p["remote"] = value!("missing-remote"));
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    let rows = r["processes"].as_array().unwrap();
    let last = rows.last().unwrap();
    assert_eq!(last["argv"][1], "fetch");
    assert_ne!(last["process"]["exit_code"], 0);
    assert!(
        !last["process"]["stderr_bytes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!target.exists());
    h.policy(|p| {
        p["remote"] = value!("warehouse");
        p["git"]["expected_version"] = value!("not the actual Git version");
    });
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert_eq!(r["error"], "Git version mismatch");
    assert!(r["processes"].as_array().unwrap().is_empty());
    assert_eq!(r["tool"]["version"]["exit_code"], 0);
    assert!(
        r["tool"]["version"]["stdout"]
            .as_str()
            .unwrap()
            .starts_with("git version ")
    );
}
#[test]
fn failed_checkout_hook_keeps_work_and_owned_lock_with_original_exit() {
    let h = Host::new("payload");
    h.hook("echo recoverable > recovery-file\necho hook-failure >&2\nexit 37");
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("integration", "task", &target);
    assert_ne!(code, 0, "{r}");
    assert_eq!(
        fs::read_to_string(target.join("recovery-file")).unwrap(),
        "recoverable\n"
    );
    assert_eq!(
        git(&target, &["symbolic-ref", "HEAD"]),
        "refs/heads/integration/task"
    );
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"])
            .contains(r["lock_reason"].as_str().unwrap())
    );
    let process = &r["processes"].as_array().unwrap().last().unwrap()["process"];
    assert_eq!(process["exit_code"], 37);
    assert!(process["stderr"].as_str().unwrap().contains("hook-failure"));
}
#[test]
fn successful_hook_with_dirty_checkout_cannot_report_created() {
    let h = Host::new("payload");
    h.hook("echo changed > payload\nexit 0");
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0, "{r}");
    assert_eq!(r["status"], "failed");
    assert!(r["error"].as_str().unwrap().contains("clean snapshot"));
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "changed\n"
    );
}
#[test]
fn strict_cli_and_configuration_fail_without_creating_a_worktree() {
    let h = Host::new("payload");
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("unknown", "task", &target);
    assert_ne!(code, 0);
    assert!(r.is_null());
    h.policy(|p| p["inferred_layout"] = value!("forbidden"));
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert!(r.is_null());
    assert!(!target.exists());
    assert_eq!(chrono_worktree::run(&["--version".into()]).exit_code, 0);
    assert_ne!(chrono_worktree::run(&["start".into()]).exit_code, 0);
}

#[test]
fn configured_digest_mismatch_does_not_execute_the_wrong_git() {
    use std::os::unix::fs::PermissionsExt;
    let h = Host::new("payload");
    let script = h.parent.join("wrong-git");
    let marker = h.parent.join("must-not-exist");
    fs::write(
        &script,
        "#!/bin/sh\nprintf executed > \"$HOME/must-not-exist\"\n",
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    h.policy(|p| {
        p["git"]["program"] = value!(script);
        p["git"]["sha256"] = value!("0".repeat(64));
    });
    let (code, report, error) = h.invoke("feature", "task", &h.parent.join("target"));
    assert_ne!(code, 0);
    assert!(report.is_null());
    assert!(error.contains("before version probe"));
    assert!(!marker.exists());
}

#[test]
fn adopted_policy_runs_the_actual_creation_consumer() {
    let h = Host::new("host files/no-language-contract.data");
    let adopted = json(&fs::read(source().join(POLICY)).unwrap()).unwrap();
    h.policy(|p| {
        let environment = p["environment"].clone();
        *p = adopted;
        p["remote"] = value!("warehouse");
        p["environment"] = environment;
    });
    let target = h.parent.join("adopted");
    let (code, report, error) = h.invoke("integration", "adopted", &target);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(
        git(&target, &["rev-parse", "HEAD"]),
        git(&h.root, &["rev-parse", "HEAD"])
    );
}

#[test]
fn registered_artifacts_from_checkout_hooks_are_preserved() {
    let h = Host::new("payload");
    h.hook("mkdir -p .chrono-harness/state\necho generated > .chrono-harness/state/hook-result\n");
    let target = h.parent.join("target");
    let (code, report, error) = h.invoke("feature", "task", &target);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(
        fs::read_to_string(target.join(".chrono-harness/state/hook-result")).unwrap(),
        "generated\n"
    );
}
