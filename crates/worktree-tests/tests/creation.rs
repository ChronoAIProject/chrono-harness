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
        self.received(out)
    }
    fn received(&self, out: std::process::Output) -> (i32, Value, String) {
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

impl Host {
    fn reconstruct(&self, plan: Value, name: &str) -> (i32, Value, String) {
        let path = ".chrono-harness/state/reconstruction.json";
        fs::create_dir_all(self.root.join(".chrono-harness/state")).unwrap();
        fs::write(self.root.join(path), serde_json::to_vec(&plan).unwrap()).unwrap();
        let output = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .current_dir("/")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args([
                "reconstruct",
                "--host-root",
                self.root.to_str().unwrap(),
                "--config",
                POLICY,
                "--kind",
                "integration",
                "--name",
                name,
                "--path",
                self.parent.join(name).to_str().unwrap(),
                "--plan",
                path,
            ])
            .output()
            .unwrap();
        self.received(output)
    }
    fn upstream(&self, path: &str, bytes: &[u8]) -> String {
        let other = self.parent.join("other writer");
        git(
            &self.root,
            &[
                "clone",
                "-q",
                self.remote.to_str().unwrap(),
                other.to_str().unwrap(),
            ],
        );
        fs::write(other.join(path), bytes).unwrap();
        let head = commit(&other);
        git(&other, &["push", "-q", "origin", "dev"]);
        head
    }
}
fn reconstruction(base: &str, candidate: &str, changes: Value) -> Value {
    value!({"schema":"chrono-worktree-reconstruction/v1", "base":base,"candidate":candidate,"changes":changes})
}

#[test]
fn reconstructs_explicit_choices_on_advanced_target_without_merging_old_branch() {
    let h = Host::new("source.ts");
    fs::write(h.root.join("obsolete.data"), "original").unwrap();
    fs::write(h.root.join("remote-only"), "original").unwrap();
    let base = commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    git(&h.root, &["checkout", "-b", "feature/old"]);
    fs::write(h.root.join("source.ts"), "carried work").unwrap();
    fs::write(h.root.join("obsolete.data"), "retired work").unwrap();
    let candidate = commit(&h.root);
    let latest = h.upstream("remote-only", b"advanced target");
    let plan = reconstruction(
        &base,
        &candidate,
        value!([
            {"path":"source.ts","action":"carry"},
            {"path":"obsolete.data","action":"retire","reason":"superseded requirement"}
        ]),
    );
    let (code, r, error) = h.reconstruct(plan, "fresh");
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["status"], "reconstructed");
    assert_eq!(r["base"], latest);
    assert!(r["context"]["candidate"].is_null());
    let target = h.parent.join("fresh");
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), latest);
    assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), candidate);
    assert_eq!(git(&h.root, &["branch", "--show-current"]), "feature/old");
    assert_eq!(fs::read(target.join("source.ts")).unwrap(), b"carried work");
    assert_eq!(fs::read(target.join("obsolete.data")).unwrap(), b"original");
    assert_eq!(
        fs::read(target.join("remote-only")).unwrap(),
        b"advanced target"
    );
    assert_eq!(
        r["reconstruction"]["index_tree"],
        git(&target, &["write-tree"])
    );
    assert_eq!(r["reconstruction"]["staged_paths"], value!(["source.ts"]));
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    let apply = r["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["argv"].as_array().unwrap().contains(&value!("apply")))
        .unwrap();
    assert_eq!(
        apply["process"]["stdin_sha256"],
        r["reconstruction"]["patch_sha256"]
    );
}

#[test]
fn reconstruction_preserves_binary_modes_symlinks_adds_deletes_and_literal_paths() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    let h = Host::new("entry.go");
    for path in [
        "literal[1].bin",
        "literal1.bin",
        "removed",
        "executable",
        "alias",
    ] {
        fs::write(h.root.join(path), b"original\0bytes").unwrap();
    }
    let base = commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    fs::write(h.root.join("literal[1].bin"), b"new\0binary\xff").unwrap();
    fs::write(h.root.join("literal1.bin"), "retired").unwrap();
    fs::remove_file(h.root.join("removed")).unwrap();
    fs::remove_file(h.root.join("alias")).unwrap();
    symlink("entry.go", h.root.join("alias")).unwrap();
    fs::set_permissions(h.root.join("executable"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(h.root.join("added λ"), "new file").unwrap();
    let candidate = commit(&h.root);
    let mut changes: Vec<_> = [
        "literal[1].bin",
        "removed",
        "alias",
        "executable",
        "added λ",
    ]
    .iter()
    .map(|p| value!({"path":p,"action":"carry"}))
    .collect();
    changes.push(value!({"path":"literal1.bin","action":"retire","reason":"not selected"}));
    let (code, r, error) =
        h.reconstruct(reconstruction(&base, &candidate, value!(changes)), "binary");
    assert_eq!(code, 0, "{r} {error}");
    let target = h.parent.join("binary");
    assert_eq!(
        fs::read(target.join("literal[1].bin")).unwrap(),
        b"new\0binary\xff"
    );
    assert_eq!(
        fs::read(target.join("literal1.bin")).unwrap(),
        b"original\0bytes"
    );
    assert_eq!(
        fs::read_link(target.join("alias")).unwrap(),
        Path::new("entry.go")
    );
    assert_eq!(
        fs::metadata(target.join("executable"))
            .unwrap()
            .permissions()
            .mode()
            & 0o111,
        0o111
    );
    assert!(!target.join("removed").exists());
    assert_eq!(fs::read(target.join("added λ")).unwrap(), b"new file");
}

#[test]
fn reconstruction_conflict_preserves_both_worktrees_original_exit_and_owned_lock() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("payload"), "local change\n").unwrap();
    let candidate = commit(&h.root);
    let latest = h.upstream("payload", b"remote change\n");
    let (code, r, _) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"carry"}]),
        ),
        "conflict",
    );
    assert_ne!(code, 0, "{r}");
    assert_eq!(r["status"], "failed");
    let target = h.parent.join("conflict");
    assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), candidate);
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), latest);
    assert_eq!(fs::read(h.root.join("payload")).unwrap(), b"local change\n");
    assert!(!git(&target, &["ls-files", "--unmerged"]).is_empty());
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"])
            .contains(r["lock_reason"].as_str().unwrap())
    );
    let process = &r["processes"].as_array().unwrap().last().unwrap()["process"];
    assert_eq!(process["exit_code"], 1);
    assert!(r["context"]["candidate"].is_null());
    assert!(r["reconstruction"].get("index_tree").is_none());
}

#[test]
fn invalid_reconstruction_choices_or_dirty_source_never_create_a_destination() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("payload"), "committed").unwrap();
    let candidate = commit(&h.root);
    let changes = [
        value!([]),
        value!([{"path":"foreign","action":"carry"}]),
        value!([{"path":"payload","action":"retire"}]),
        value!([{"path":"payload","action":"carry"},{"path":"payload","action":"carry"}]),
    ];
    for (n, change) in changes.into_iter().enumerate() {
        let name = format!("invalid-{n}");
        let (code, _, _) = h.reconstruct(reconstruction(&base, &candidate, change), &name);
        assert_ne!(code, 0);
        assert!(!h.parent.join(name).exists());
    }
    let valid = value!([{"path":"payload","action":"carry"}]);
    assert_ne!(
        h.reconstruct(
            reconstruction(&base, &base, valid.clone()),
            "wrong-candidate"
        )
        .0,
        0
    );
    assert!(!h.parent.join("wrong-candidate").exists());
    fs::write(h.root.join("payload"), "unsaved work").unwrap();
    assert_ne!(
        h.reconstruct(reconstruction(&base, &candidate, valid), "dirty")
            .0,
        0
    );
    assert!(!h.parent.join("dirty").exists());
    assert_eq!(fs::read(h.root.join("payload")).unwrap(), b"unsaved work");
}

#[test]
fn retired_only_reconstruction_keeps_new_target_without_applying_a_patch() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("payload"), "old work").unwrap();
    let candidate = commit(&h.root);
    let (code, r, error) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"retire","reason":"already unnecessary"}]),
        ),
        "retired",
    );
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["reconstruction"]["staged_paths"], value!([]));
    assert_eq!(
        r["reconstruction"]["index_tree"],
        git(&h.root, &["rev-parse", &format!("{base}^{{tree}}")])
    );
    assert!(
        !r["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["argv"].as_array().unwrap().contains(&value!("apply")))
    );
    assert_eq!(fs::read(h.root.join("payload")).unwrap(), b"old work");
}

#[test]
fn source_mutation_during_reconstruction_fails_and_preserves_recovery_work() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("payload"), "old work").unwrap();
    let candidate = commit(&h.root);
    h.hook("echo concurrent-work > \"$HOME/source with spaces/payload\"\n");
    let (code, r, _) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"carry"}]),
        ),
        "mutated",
    );
    assert_ne!(code, 0, "{r}");
    assert_eq!(r["status"], "failed");
    assert_eq!(
        fs::read(h.root.join("payload")).unwrap(),
        b"concurrent-work\n"
    );
    assert!(h.parent.join("mutated").is_dir());
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"])
            .contains(r["lock_reason"].as_str().unwrap())
    );
}

#[test]
fn reconstruction_patch_bound_fails_before_creating_work() {
    let h = Host::new("payload");
    h.policy(|p| p["output_limit_bytes"] = value!(16384));
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    let data: String = (0..10000)
        .map(|n| format!("line {n:08} explicit reconstruction payload\n"))
        .collect();
    fs::write(h.root.join("payload"), &data).unwrap();
    let candidate = commit(&h.root);
    let (code, r, _) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"carry"}]),
        ),
        "bounded",
    );
    assert_ne!(code, 0, "{r}");
    assert!(!h.parent.join("bounded").exists());
    assert_eq!(fs::read_to_string(h.root.join("payload")).unwrap(), data);
    let last = r["processes"].as_array().unwrap().last().unwrap();
    assert!(
        last["argv"]
            .as_array()
            .unwrap()
            .contains(&value!("--binary"))
    );
    assert!(last["process"]["failure"].is_string());
}

#[test]
fn empty_reconstruction_and_existing_destination_keep_explicit_boundaries() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    let plan = reconstruction(&base, &base, value!([]));
    let (code, r, error) = h.reconstruct(plan.clone(), "empty");
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["reconstruction"]["staged_paths"], value!([]));
    fs::write(h.parent.join("empty/sentinel"), "preserved").unwrap();
    assert_ne!(h.reconstruct(plan, "empty").0, 0);
    assert_eq!(
        fs::read(h.parent.join("empty/sentinel")).unwrap(),
        b"preserved"
    );
    assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), base);
}
