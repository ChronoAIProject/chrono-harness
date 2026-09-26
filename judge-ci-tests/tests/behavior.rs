use chrono_harness::{PROTOCOL, Request, Response, Status, sha256};
use chrono_judge_ci::judge;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
const CONFIG: &str = ".chrono-harness/ci/check.json";
struct Host {
    dir: TempDir,
}
impl Host {
    fn new() -> Self {
        let h = Self {
            dir: tempfile::Builder::new()
                .prefix("judge host ")
                .tempdir()
                .unwrap(),
        };
        h.git(&["init", "-q"]);
        h.git(&["config", "user.email", "test@example.invalid"]);
        h.git(&["config", "user.name", "Test"]);
        h.write(".gitignore", ".chrono-harness/state/\n");
        h.write("src.txt", "one\n");
        h.write("doc.txt", "one\n");
        h.write(
            "check.sh",
            "mkdir -p .chrono-harness/state\nprintf 'called\\n' >> .chrono-harness/state/calls\n",
        );
        h.json(".chrono-harness/projects.json",&json!({"schema_version":1,"owners":["host","lib","suite"],"projects":[{"id":"lib","actions":{}},{"id":"suite","actions":{"execute":{"operation":"test.suite","tool":"sh","argv":["check.sh"]}}}],"scripts":[]}));
        let mut files = vec![];
        for p in [
            ".gitignore",
            "src.txt",
            "doc.txt",
            "check.sh",
            ".chrono-harness/projects.json",
            ".chrono-harness/FILEMAP.json",
            CONFIG,
        ] {
            files.push(json!({"path":p,"owner":"host","surface":"product","cost":"unmeasured","edges":if p=="src.txt"{json!([{"kind":"compile","to":"project:lib"}])}else if p=="check.sh"{json!([{"kind":"test-execution","to":"test:suite"}])}else{json!([])}}));
        }
        h.json(".chrono-harness/FILEMAP.json",&json!({"schema_version":1,"files":files,"project_edges":[{"from":"project:lib","kind":"test-execution","to":"test:suite"}]}));
        h.json(CONFIG,&json!({"schema":"chrono-ci-check/v1","judge":{"program":"unused","args":[],"timeout_seconds":5,"output_limit_bytes":4096},"report_path":".chrono-harness/state/check.json","policy":{"filemap":".chrono-harness/FILEMAP.json","projects":".chrono-harness/projects.json","tools":{"sh":"/bin/sh"},"bindings":{"test:suite":["test.suite"]},"artifacts":[".chrono-harness/state/"],"required_inputs":[CONFIG,"check.sh"],"adoption_base":null,"operation_timeout_seconds":3,"operation_output_limit_bytes":4096}}));
        h.commit();
        h
    }
    fn root(&self) -> &Path {
        self.dir.path()
    }
    fn git(&self, args: &[&str]) -> String {
        let o = Command::new("git")
            .args(args)
            .current_dir(self.root())
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8(o.stdout).unwrap().trim().to_string()
    }
    fn write(&self, p: &str, text: &str) {
        let p = self.root().join(p);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    fn json(&self, p: &str, v: &Value) {
        self.write(p, &serde_json::to_string_pretty(v).unwrap())
    }
    fn read(&self, p: &str) -> Value {
        serde_json::from_slice(&fs::read(self.root().join(p)).unwrap()).unwrap()
    }
    fn commit(&self) -> String {
        self.git(&["add", "."]);
        self.git(&["commit", "-qm", "fixture", "--allow-empty"]);
        self.git(&["rev-parse", "HEAD"])
    }
    fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"])
    }
    fn request(&self, b: Option<&str>, c: &str) -> Request {
        Request {
            protocol: PROTOCOL.into(),
            request_id: "fixture-request".into(),
            host_root: self.root().to_path_buf(),
            config_path: CONFIG.into(),
            config_sha256: sha256(&fs::read(self.root().join(CONFIG)).unwrap()),
            base: b.map(str::to_string),
            candidate: c.into(),
            initial: b.is_none(),
        }
    }
    fn check(&self, b: &str, c: &str) -> Response {
        judge(&self.request(Some(b), c))
    }
    fn change_registry(&self, p: &str, f: impl FnOnce(&mut Value)) {
        let mut v = self.read(p);
        f(&mut v);
        self.json(p, &v)
    }
    fn calls(&self) -> usize {
        fs::read_to_string(self.root().join(".chrono-harness/state/calls"))
            .unwrap_or_default()
            .lines()
            .count()
    }
}
fn pass(r: &Response) {
    assert_eq!(r.status, Status::Passed, "{r:#?}")
}
fn fail(r: &Response, needle: &str) {
    assert_eq!(r.status, Status::Failed, "{r:#?}");
    assert!(serde_json::to_string(r).unwrap().contains(needle), "{r:#?}")
}
#[test]
fn source_closure_executes_real_command() {
    let h = Host::new();
    let b = h.head();
    h.write("src.txt", "two");
    let c = h.commit();
    let r = h.check(&b, &c);
    pass(&r);
    assert_eq!(h.calls(), 1);
    assert_eq!(r.evidence["selected"], json!(["test:suite"]));
    assert!(
        r.evidence["executed"][0]["process"]["sha256"]
            .as_str()
            .unwrap()
            .len()
            == 64
    );
}
#[test]
fn documentation_outside_closure_has_no_project_checks() {
    let h = Host::new();
    let b = h.head();
    h.write("doc.txt", "two");
    let c = h.commit();
    let r = h.check(&b, &c);
    pass(&r);
    assert_eq!(h.calls(), 0);
    assert_eq!(r.evidence["selected"], json!([]));
    assert_eq!(r.evidence["not_required"], json!(["test:suite"]));
}
#[test]
fn identical_commit_is_honest_noop() {
    let h = Host::new();
    let r = h.check(&h.head(), &h.head());
    pass(&r);
    assert_eq!(h.calls(), 0);
    assert_eq!(r.evidence["changed_paths"], json!([]));
}
#[test]
fn deletion_keeps_old_consumer() {
    let h = Host::new();
    let b = h.head();
    fs::remove_file(h.root().join("src.txt")).unwrap();
    h.change_registry(".chrono-harness/FILEMAP.json", |v| {
        v["files"]
            .as_array_mut()
            .unwrap()
            .retain(|f| f["path"] != "src.txt")
    });
    let c = h.commit();
    pass(&h.check(&b, &c));
    assert_eq!(h.calls(), 1);
}
#[test]
fn rename_is_delete_plus_add() {
    let h = Host::new();
    let b = h.head();
    fs::rename(h.root().join("src.txt"), h.root().join("new name.txt")).unwrap();
    h.change_registry(".chrono-harness/FILEMAP.json", |v| {
        for f in v["files"].as_array_mut().unwrap() {
            if f["path"] == "src.txt" {
                f["path"] = "new name.txt".into()
            }
        }
    });
    let c = h.commit();
    let r = h.check(&b, &c);
    pass(&r);
    assert_eq!(h.calls(), 1);
    assert!(
        r.evidence["changed_paths"]
            .as_array()
            .unwrap()
            .contains(&json!("src.txt"))
    );
}
#[test]
fn removed_project_edge_still_selects_surviving_consumer() {
    let h = Host::new();
    let b = h.head();
    h.change_registry(".chrono-harness/FILEMAP.json", |v| {
        v["project_edges"] = json!([])
    });
    let c = h.commit();
    pass(&h.check(&b, &c));
    assert_eq!(h.calls(), 1);
}
#[test]
fn removed_file_edge_seeds_old_graph() {
    let h = Host::new();
    let b = h.head();
    h.change_registry(".chrono-harness/FILEMAP.json", |v| {
        for f in v["files"].as_array_mut().unwrap() {
            if f["path"] == "src.txt" {
                f["edges"] = json!([])
            }
        }
    });
    let c = h.commit();
    pass(&h.check(&b, &c));
    assert_eq!(h.calls(), 1);
}
#[test]
fn operation_only_change_is_selected_and_failure_is_real() {
    let h = Host::new();
    let b = h.head();
    h.change_registry(".chrono-harness/projects.json", |v| {
        v["projects"][1]["actions"]["execute"]["argv"] = json!(["-c", "exit 7"])
    });
    let c = h.commit();
    let r = h.check(&b, &c);
    fail(&r, "exit 7");
    assert_eq!(r.evidence["executed"][0]["process"]["exit_code"], 7);
}
#[test]
fn removed_affected_binding_fails_closed() {
    let h = Host::new();
    let b = h.head();
    h.change_registry(".chrono-harness/projects.json", |v| {
        v["projects"][1]["actions"] = json!({})
    });
    h.change_registry(CONFIG, |v| v["policy"]["bindings"] = json!({}));
    h.change_registry(".chrono-harness/FILEMAP.json", |v| {
        v["project_edges"] = json!([]);
        for f in v["files"].as_array_mut().unwrap() {
            f["edges"] = json!([])
        }
    });
    let c = h.commit();
    fail(&h.check(&b, &c), "affected binding removed");
    assert_eq!(h.calls(), 0);
}
#[test]
fn unknown_changed_path_never_green() {
    let h = Host::new();
    let b = h.head();
    h.write("unknown.txt", "new");
    let c = h.commit();
    fail(&h.check(&b, &c), "unregistered tracked path");
}
#[test]
fn absent_registered_path_never_green() {
    let h = Host::new();
    let b = h.head();
    fs::remove_file(h.root().join("src.txt")).unwrap();
    let c = h.commit();
    fail(&h.check(&b, &c), "registered path absent");
}
#[test]
fn duplicate_and_unresolved_registration_fail() {
    for duplicate in [true, false] {
        let h = Host::new();
        let b = h.head();
        h.change_registry(".chrono-harness/FILEMAP.json", |v| {
            if duplicate {
                let row = v["files"][0].clone();
                v["files"].as_array_mut().unwrap().push(row)
            } else {
                v["project_edges"][0]["to"] = "test:missing".into()
            }
        });
        let c = h.commit();
        fail(
            &h.check(&b, &c),
            if duplicate {
                "duplicate FILEMAP"
            } else {
                "unresolved edge"
            },
        );
    }
}
#[test]
fn missing_or_short_oid_and_checkout_mismatch_fail() {
    let h = Host::new();
    let b = h.head();
    h.write("doc.txt", "two");
    let c = h.commit();
    for (base, candidate, reason) in [
        (&b[..8], c.as_str(), "full"),
        (
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            c.as_str(),
            "git",
        ),
        (b.as_str(), b.as_str(), "HEAD"),
    ] {
        fail(&h.check(base, candidate), reason)
    }
}
#[test]
fn dirty_tracked_untracked_ignored_and_staged_fail() {
    for mode in ["tracked", "untracked", "ignored", "staged"] {
        let h = Host::new();
        let b = h.head();
        match mode {
            "tracked" => h.write("src.txt", "dirty"),
            "untracked" => h.write("extra", "dirty"),
            "ignored" => {
                h.write(".git/info/exclude", "extra\n");
                h.write("extra", "dirty")
            }
            _ => {
                h.write("src.txt", "dirty");
                h.git(&["add", "src.txt"]);
            }
        }
        fail(
            &h.check(&b, &b),
            if matches!(mode, "untracked" | "ignored") {
                "untracked nonartifact"
            } else {
                "dirty tracked"
            },
        );
    }
}
#[test]
fn artifact_outputs_are_allowed_but_tracked_artifacts_are_not() {
    let h = Host::new();
    let b = h.head();
    h.write(".chrono-harness/state/log", "out");
    pass(&h.check(&b, &b));
    h.git(&["add", "-f", ".chrono-harness/state/log"]);
    let c = h.commit();
    fail(&h.check(&b, &c), "unregistered tracked path");
}
#[test]
fn profile_bytes_and_request_digest_bind_candidate() {
    let h = Host::new();
    let b = h.head();
    let mut r = h.request(Some(&b), &b);
    r.config_sha256 = "bad".into();
    fail(&judge(&r), "config does not match");
    h.change_registry(CONFIG, |v| {
        v["policy"]["operation_timeout_seconds"] = 9.into()
    });
    fail(&h.check(&b, &b), "config does not match");
}
#[test]
fn initial_requires_parentless_commit_and_runs_declared_inventory() {
    let h = Host::new();
    let b = h.head();
    let r = judge(&h.request(None, &b));
    pass(&r);
    assert_eq!(r.evidence["mode"], "initial-inventory");
    assert_eq!(h.calls(), 1);
    let c = h.commit();
    let r = judge(&h.request(None, &c));
    fail(&r, "parentless");
    assert_eq!(r.evidence["executed"], json!([]));
    assert_eq!(h.calls(), 1);
}
#[test]
fn initial_rejects_actual_parents_in_full_and_shallow_history_before_operations() {
    for shallow in [false, true] {
        let h = Host::new();
        let b = h.head();
        h.write("src.txt", "two\n");
        let c = h.commit();
        if shallow {
            h.write(".git/shallow", &format!("{c}\n"));
            assert_eq!(h.git(&["rev-list", "--parents", "-n", "1", &c]), c);
        }
        assert_eq!(
            h.git(&["rev-parse", "--is-shallow-repository"]),
            shallow.to_string()
        );
        assert!(
            h.git(&["cat-file", "-p", &c])
                .lines()
                .take_while(|line| !line.is_empty())
                .any(|line| line == format!("parent {b}"))
        );
        assert!(h.git(&["status", "--porcelain"]).is_empty());
        let r = judge(&h.request(None, &c));
        fail(&r, "initial requires parentless commit");
        assert_eq!(r.results[0].id, "ci.inventory");
        assert_eq!(r.evidence["executed"], json!([]));
        assert_eq!(h.calls(), 0);

        let r = h.check(&b, &c);
        pass(&r);
        assert_eq!(r.evidence["mode"], "delta");
        assert_eq!(r.evidence["selected"], json!(["test:suite"]));
        assert_eq!(h.calls(), 1);
    }
}
#[test]
fn output_mutating_inputs_fails_after_execution() {
    let h = Host::new();
    let b = h.head();
    h.write("check.sh", "printf dirty > src.txt\n");
    let c = h.commit();
    fail(&h.check(&b, &c), "dirty tracked");
}
#[test]
fn mode_only_change_is_delta() {
    let h = Host::new();
    let b = h.head();
    h.git(&["update-index", "--chmod=+x", "src.txt"]);
    h.git(&["commit", "-qm", "mode"]);
    let c = h.head();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(h.root().join("src.txt"), fs::Permissions::from_mode(0o755)).unwrap();
    pass(&h.check(&b, &c));
    assert_eq!(h.calls(), 1);
}
#[test]
fn adoption_uses_real_base_and_requires_exact_opt_in() {
    let h = Host::new();
    h.git(&["rm", CONFIG]);
    h.change_registry(".chrono-harness/FILEMAP.json", |v| {
        v["files"]
            .as_array_mut()
            .unwrap()
            .retain(|f| f["path"] != CONFIG)
    });
    let b = h.commit();
    let prior = h.git(&["rev-parse", "HEAD~1"]);
    let old = h.git(&["show", &format!("{prior}:{CONFIG}")]);
    h.write(CONFIG, &old);
    h.change_registry(".chrono-harness/FILEMAP.json",|v|v["files"].as_array_mut().unwrap().push(json!({"path":CONFIG,"owner":"host","surface":"membership","cost":"unmeasured","edges":[]})));
    h.write("src.txt", "adopted");
    let c = h.commit();
    fail(&h.check(&b, &c), "adoption_base required");
    h.change_registry(CONFIG, |v| v["policy"]["adoption_base"] = b.clone().into());
    let c = h.commit();
    let r = h.check(&b, &c);
    pass(&r);
    assert_eq!(r.evidence["previous_enforcement"], "none");
    assert_eq!(h.calls(), 1);
}
#[test]
fn explicitly_registered_extension_executes_without_scheduler_changes() {
    let h = Host::new();
    let b = h.head();
    h.write(
        "extra.sh",
        "mkdir -p .chrono-harness/state\nprintf extra > .chrono-harness/state/extra\n",
    );
    h.change_registry(".chrono-harness/projects.json",|v|{v["owners"].as_array_mut().unwrap().push("extra".into());v["scripts"].as_array_mut().unwrap().push(json!({"id":"extra","actions":{"execute":{"operation":"extra.check","tool":"sh","argv":["extra.sh"]}}}));});
    h.change_registry(CONFIG, |v| {
        v["policy"]["bindings"]["test:extra"] = json!(["extra.check"])
    });
    h.change_registry(".chrono-harness/FILEMAP.json",|v|v["files"].as_array_mut().unwrap().push(json!({"path":"extra.sh","owner":"extra","surface":"test","cost":"unmeasured","edges":[{"kind":"test-execution","to":"test:extra"}]})));
    let c = h.commit();
    let r = h.check(&b, &c);
    pass(&r);
    assert_eq!(
        fs::read_to_string(h.root().join(".chrono-harness/state/extra")).unwrap(),
        "extra"
    );
}
#[test]
fn tool_or_environment_change_reexecutes_its_registered_bindings() {
    for env in [false, true] {
        let h = Host::new();
        let b = h.head();
        h.change_registry(CONFIG, |v| {
            if env {
                v["policy"]["environment"] = json!({"CHRONO_FIXTURE":"changed"})
            } else {
                v["policy"]["tools"]["sh"] = "sh".into()
            }
        });
        let c = h.commit();
        pass(&h.check(&b, &c));
        assert_eq!(h.calls(), 1);
    }
}
#[test]
fn index_changes_cannot_cancel_against_worktree_changes() {
    let h = Host::new();
    let base = h.head();
    h.write("src.txt", "staged change");
    h.git(&["add", "src.txt"]);
    h.write("src.txt", "one\n");
    fail(&h.check(&base, &base), "dirty tracked");
}

#[test]
fn hidden_worktree_bytes_fail_before_execution_without_changing_index() {
    for (flag, reason) in [
        ("--assume-unchanged", "assume-unchanged"),
        ("--skip-worktree", "skip-worktree"),
    ] {
        let h = Host::new();
        let base = h.head();
        // Tabs/spaces in a real tracked name must survive index fact parsing.
        let path = "source with\ttab.txt";
        h.git(&["mv", "src.txt", path]);
        h.change_registry(".chrono-harness/FILEMAP.json", |v| {
            for row in v["files"].as_array_mut().unwrap() {
                if row["path"] == "src.txt" {
                    row["path"] = path.into();
                }
            }
        });
        h.write(path, "bad\n");
        h.write(
            "check.sh",
            "test \"$(cat 'source with\ttab.txt')\" = good || exit 7\n",
        );
        let candidate = h.commit();
        let control = h.check(&base, &candidate);
        fail(&control, "exit 7");
        h.git(&["update-index", flag, "--", path]);
        h.write(path, "good\n");
        assert!(h.git(&["diff", "--raw", "-z"]).is_empty());
        let index = fs::read(h.root().join(".git/index")).unwrap();
        let result = h.check(&base, &candidate);
        fail(&result, reason);
        assert_eq!(result.results[0].id, "ci.inventory");
        assert!(result.results[0].cause.contains(path));
        assert_eq!(result.evidence["executed"], json!([]));
        assert_eq!(fs::read(h.root().join(".git/index")).unwrap(), index);
    }
}

#[test]
fn index_flags_created_by_operations_fail_the_post_snapshot_guard() {
    for (flag, reason) in [
        ("--assume-unchanged", "assume-unchanged"),
        ("--skip-worktree", "skip-worktree"),
    ] {
        let h = Host::new();
        let base = h.head();
        h.write(
            "check.sh",
            &format!("git update-index {flag} src.txt\nprintf hidden > src.txt\n"),
        );
        let candidate = h.commit();
        let result = h.check(&base, &candidate);
        fail(&result, reason);
        assert_eq!(result.evidence["executed"][0]["process"]["exit_code"], 0);
        let guard = result
            .results
            .iter()
            .find(|r| r.id == "ci.post-snapshot")
            .unwrap();
        assert_eq!(guard.status, Status::Failed);
        assert!(guard.cause.contains(reason));
    }
}
