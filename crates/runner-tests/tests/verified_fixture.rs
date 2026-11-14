//! 命令替身的封闭派发域；IO 行为仍由独立 Rust 边界预期核验。
#![allow(unexpected_cfgs)]
#[path = "../../worktree-tests/tests/support/cargo_model.rs"]
mod cargo;
#[cfg(verus_keep_ghost)]
#[path = "support/cli_child.rs"]
mod cli_child;
#[cfg(verus_keep_ghost)]
#[path = "../../worktree-tests/tests/support/fixture_coordinator.rs"]
mod custody;
#[path = "../../distribution-tests/tests/support/bootstrap.rs"]
mod distribution_bootstrap;
#[path = "../../ci-tests/tests/support/generated_command.rs"]
mod generated_command;
#[path = "support/git_model.rs"]
mod git;
#[path = "support/git_fixture.rs"]
mod git_fixture;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[path = "support/isolated_capability.rs"]
mod isolation;
#[cfg(verus_keep_ghost)]
#[path = "protocol_v1.rs"]
mod protocol;
#[cfg(verus_keep_ghost)]
#[path = "retention.rs"]
mod retention;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;
verus! {
#[cfg(verus_keep_ghost)]
fn bounded_fixture_recovery_preserves_the_original_eight_round_limit() {
    expect(retention::fixture_recovery_has_room(0));
    expect(retention::fixture_recovery_has_room(7));
    expect(!retention::fixture_recovery_has_room(8));
    expect(!retention::fixture_recovery_has_room(usize::MAX));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn custody_release_requires_original_child_result_and_owner_retirement() {
    expect(isolation::release_custody(true, true));
    expect(!isolation::release_custody(false, true));
    expect(!isolation::release_custody(true, false));
    expect(!isolation::release_custody(false, false));
}
#[cfg_attr(not(verus_keep_ghost), test)]
fn isolation_requires_complete_test_identity() {
    proof {
        reveal_strlit("case"); reveal_strlit("case-extra"); reveal_strlit("");
        assert("case"@.len() != "case-extra"@.len());
        assert("case"@.len() != ""@.len());
    }
    expect(isolation::same_test("case",Some("case")));
    expect(!isolation::same_test("case",Some("case-extra")));
    expect(!isolation::same_test("case",Some("")));
    expect(!isolation::same_test("case",None));
}
#[cfg_attr(not(verus_keep_ghost), test)]
fn generated_command_has_closed_targets() {
    proof {
        reveal_strlit("chrono-harness"); reveal_strlit("$CHRONO_BASE");
        reveal_strlit("/usr/bin/printf"); reveal_strlit("/bin/sh"); reveal_strlit("");
        assert("/bin/sh"@.len() != "chrono-harness"@.len());
        assert("/bin/sh"@.len() != "$CHRONO_BASE"@.len());
        assert("/bin/sh"@.len() != "/usr/bin/printf"@.len());
        assert("/bin/sh"@.len() != ""@.len());
        assert(""@.len() != "chrono-harness"@.len());
        assert(""@.len() != "$CHRONO_BASE"@.len());
        assert(""@.len() != "/usr/bin/printf"@.len());
    }
    expect(generated_command::known_target("chrono-harness"));
    expect(generated_command::known_target("$CHRONO_BASE"));
    expect(generated_command::known_target("/usr/bin/printf"));
    expect(!generated_command::known_target("/bin/sh"));
    expect(!generated_command::known_target(""));
}
#[cfg_attr(not(verus_keep_ghost), test)]
fn distribution_bootstrap_has_closed_modes() {
    proof {
        reveal_strlit("install"); reveal_strlit("adopt"); reveal_strlit("shell"); reveal_strlit("");
        assert("shell"@[0] != "adopt"@[0]);
        assert("shell"@.len() != "install"@.len());
        assert("shell"@.len() != ""@.len());
        assert(""@.len() != "install"@.len() && ""@.len() != "adopt"@.len());
    }
    expect(distribution_bootstrap::known_mode("install"));
    expect(distribution_bootstrap::known_mode("adopt"));
    expect(!distribution_bootstrap::known_mode("shell"));
    expect(!distribution_bootstrap::known_mode(""));
}
fn expect(actual:bool) requires actual, { if !actual {panic!();} }
#[cfg_attr(not(verus_keep_ghost), test)]
fn known_git_command_independent_boundaries() {
    proof {
        reveal_strlit("tag"); reveal_strlit("apply");
        reveal_strlit("--version"); reveal_strlit("check-ref-format"); reveal_strlit("init"); reveal_strlit("clone"); reveal_strlit("add"); reveal_strlit("commit"); reveal_strlit("commit-tree"); reveal_strlit("rev-parse"); reveal_strlit("show"); reveal_strlit("ls-tree"); reveal_strlit("cat-file"); reveal_strlit("ls-files"); reveal_strlit("config"); reveal_strlit("diff"); reveal_strlit("update-index"); reveal_strlit("update-ref"); reveal_strlit("reset"); reveal_strlit("restore"); reveal_strlit("checkout"); reveal_strlit("switch"); reveal_strlit("status"); reveal_strlit("symbolic-ref"); reveal_strlit("branch"); reveal_strlit("show-ref"); reveal_strlit("for-each-ref"); reveal_strlit("remote"); reveal_strlit("ls-remote"); reveal_strlit("push"); reveal_strlit("fetch"); reveal_strlit("merge-base"); reveal_strlit("rev-list"); reveal_strlit("merge"); reveal_strlit("write-tree"); reveal_strlit("read-tree"); reveal_strlit("mv"); reveal_strlit("rm"); reveal_strlit("replace"); reveal_strlit("reflog"); reveal_strlit("worktree"); reveal_strlit(""); reveal_strlit("unknown command"); reveal_strlit("BUILD");
        assert(""@.len() == 0);
        assert("BUILD"@.len() != "clone"@.len() || "BUILD"@[0] != "clone"@[0]);
        assert("BUILD"@.len() != "reset"@.len() || "BUILD"@[0] != "reset"@[0]);
        assert("BUILD"@.len() != "merge"@.len() || "BUILD"@[0] != "merge"@[0]);
        assert("BUILD"@.len() != "fetch"@.len() || "BUILD"@[0] != "fetch"@[0]);
        assert("unknown command"@.len() != "--version"@.len() || "unknown command"@[0] != "--version"@[0]);
        assert("unknown command"@.len() != "check-ref-format"@.len() || "unknown command"@[0] != "check-ref-format"@[0]);
        assert("unknown command"@.len() != "init"@.len() || "unknown command"@[0] != "init"@[0]);
        assert("unknown command"@.len() != "clone"@.len() || "unknown command"@[0] != "clone"@[0]);
        assert("unknown command"@.len() != "add"@.len() || "unknown command"@[0] != "add"@[0]);
        assert("unknown command"@.len() != "commit"@.len() || "unknown command"@[0] != "commit"@[0]);
        assert("unknown command"@.len() != "commit-tree"@.len() || "unknown command"@[0] != "commit-tree"@[0]);
        assert("unknown command"@.len() != "rev-parse"@.len() || "unknown command"@[0] != "rev-parse"@[0]);
        assert("unknown command"@.len() != "show"@.len() || "unknown command"@[0] != "show"@[0]);
        assert("unknown command"@.len() != "ls-tree"@.len() || "unknown command"@[0] != "ls-tree"@[0]);
        assert("unknown command"@.len() != "cat-file"@.len() || "unknown command"@[0] != "cat-file"@[0]);
        assert("unknown command"@.len() != "ls-files"@.len() || "unknown command"@[0] != "ls-files"@[0]);
        assert("unknown command"@.len() != "config"@.len() || "unknown command"@[0] != "config"@[0]);
        assert("unknown command"@.len() != "diff"@.len() || "unknown command"@[0] != "diff"@[0]);
        assert("unknown command"@.len() != "update-index"@.len() || "unknown command"@[0] != "update-index"@[0]);
        assert("unknown command"@.len() != "update-ref"@.len() || "unknown command"@[0] != "update-ref"@[0]);
        assert("unknown command"@.len() != "reset"@.len() || "unknown command"@[0] != "reset"@[0]);
        assert("unknown command"@.len() != "restore"@.len() || "unknown command"@[0] != "restore"@[0]);
        assert("unknown command"@.len() != "checkout"@.len() || "unknown command"@[0] != "checkout"@[0]);
        assert("unknown command"@.len() != "switch"@.len() || "unknown command"@[0] != "switch"@[0]);
        assert("unknown command"@.len() != "status"@.len() || "unknown command"@[0] != "status"@[0]);
        assert("unknown command"@.len() != "symbolic-ref"@.len() || "unknown command"@[0] != "symbolic-ref"@[0]);
        assert("unknown command"@.len() != "branch"@.len() || "unknown command"@[0] != "branch"@[0]);
        assert("unknown command"@.len() != "show-ref"@.len() || "unknown command"@[0] != "show-ref"@[0]);
        assert("unknown command"@.len() != "for-each-ref"@.len() || "unknown command"@[0] != "for-each-ref"@[0]);
        assert("unknown command"@.len() != "remote"@.len() || "unknown command"@[0] != "remote"@[0]);
        assert("unknown command"@.len() != "ls-remote"@.len() || "unknown command"@[0] != "ls-remote"@[0]);
        assert("unknown command"@.len() != "push"@.len() || "unknown command"@[0] != "push"@[0]);
        assert("unknown command"@.len() != "fetch"@.len() || "unknown command"@[0] != "fetch"@[0]);
        assert("unknown command"@.len() != "merge-base"@.len() || "unknown command"@[0] != "merge-base"@[0]);
        assert("unknown command"@.len() != "rev-list"@.len() || "unknown command"@[0] != "rev-list"@[0]);
        assert("unknown command"@.len() != "merge"@.len() || "unknown command"@[0] != "merge"@[0]);
        assert("unknown command"@.len() != "write-tree"@.len() || "unknown command"@[0] != "write-tree"@[0]);
        assert("unknown command"@.len() != "read-tree"@.len() || "unknown command"@[0] != "read-tree"@[0]);
        assert("unknown command"@.len() != "mv"@.len() || "unknown command"@[0] != "mv"@[0]);
        assert("unknown command"@.len() != "rm"@.len() || "unknown command"@[0] != "rm"@[0]);
        assert("unknown command"@.len() != "replace"@.len() || "unknown command"@[0] != "replace"@[0]);
        assert("unknown command"@.len() != "reflog"@.len() || "unknown command"@[0] != "reflog"@[0]);
        assert("unknown command"@.len() != "worktree"@.len() || "unknown command"@[0] != "worktree"@[0]);
        assert("BUILD"@[0] != "--version"@[0] && "BUILD"@[0] != "check-ref-format"@[0]
            && "BUILD"@[0] != "init"@[0] && "BUILD"@[0] != "clone"@[0]
            && "BUILD"@[0] != "add"@[0] && "BUILD"@[0] != "commit"@[0]
            && "BUILD"@[0] != "commit-tree"@[0] && "BUILD"@[0] != "rev-parse"@[0]
            && "BUILD"@[0] != "show"@[0] && "BUILD"@[0] != "ls-tree"@[0]
            && "BUILD"@[0] != "cat-file"@[0] && "BUILD"@[0] != "ls-files"@[0]
            && "BUILD"@[0] != "config"@[0] && "BUILD"@[0] != "diff"@[0]
            && "BUILD"@[0] != "update-index"@[0] && "BUILD"@[0] != "update-ref"@[0]
            && "BUILD"@[0] != "restore"@[0] && "BUILD"@[0] != "reset"@[0] && "BUILD"@[0] != "checkout"@[0]
            && "BUILD"@[0] != "switch"@[0] && "BUILD"@[0] != "status"@[0]
            && "BUILD"@[0] != "symbolic-ref"@[0] && "BUILD"@[0] != "branch"@[0]
            && "BUILD"@[0] != "show-ref"@[0] && "BUILD"@[0] != "for-each-ref"@[0]
            && "BUILD"@[0] != "remote"@[0] && "BUILD"@[0] != "ls-remote"@[0]
            && "BUILD"@[0] != "push"@[0] && "BUILD"@[0] != "fetch"@[0]
            && "BUILD"@[0] != "merge-base"@[0] && "BUILD"@[0] != "rev-list"@[0]
            && "BUILD"@[0] != "merge"@[0] && "BUILD"@[0] != "write-tree"@[0]
            && "BUILD"@[0] != "read-tree"@[0] && "BUILD"@[0] != "mv"@[0]
            && "BUILD"@[0] != "rm"@[0] && "BUILD"@[0] != "replace"@[0]
            && "BUILD"@[0] != "reflog"@[0] && "BUILD"@[0] != "worktree"@[0]);
    }
    expect(git::known_git_command("--version"));
    expect(git::known_git_command("worktree"));
    expect(git::known_git_command("tag"));
    expect(git::known_git_command("apply"));
    expect(git::known_git_command("restore"));
    expect(!git::known_git_command(""));
    expect(!git::known_git_command("unknown command"));
    expect(!git::known_git_command("BUILD"));
}
#[cfg_attr(not(verus_keep_ghost), test)]
fn known_cargo_command_independent_boundaries() {
    proof {
        reveal_strlit("--native-holder"); reveal_strlit("--version"); reveal_strlit("build"); reveal_strlit("test"); reveal_strlit("generate-lockfile"); reveal_strlit(""); reveal_strlit("unknown command"); reveal_strlit("BUILD");
        assert(""@.len() != "--native-holder"@.len() && ""@.len() != "--version"@.len() && ""@.len() != "build"@.len() && ""@.len() != "test"@.len() && ""@.len() != "generate-lockfile"@.len());
        assert("unknown command"@[0] != "--native-holder"@[0]);
        assert("BUILD"@[0] != "build"@[0]);
    }
    expect(cargo::known_cargo_command("--native-holder"));
    expect(cargo::known_cargo_command("generate-lockfile"));
    expect(!cargo::known_cargo_command(""));
    expect(!cargo::known_cargo_command("unknown command"));
    expect(!cargo::known_cargo_command("BUILD"));
}
#[cfg(verus_keep_ghost)]
fn known_custody_producer_independent_boundaries() {
    proof {
        reveal_strlit("ci-test-host"); reveal_strlit("projects-test-host"); reveal_strlit("workflow-test-tools"); reveal_strlit("worktree-test-host"); reveal_strlit(""); reveal_strlit("unknown command"); reveal_strlit("BUILD");
        assert(""@.len() != "ci-test-host"@.len() && ""@.len() != "projects-test-host"@.len()
            && ""@.len() != "workflow-test-tools"@.len() && ""@.len() != "worktree-test-host"@.len());
        assert("unknown command"@[0] != "ci-test-host"@[0]
            && "unknown command"@[0] != "projects-test-host"@[0]
            && "unknown command"@[0] != "workflow-test-tools"@[0]
            && "unknown command"@[0] != "worktree-test-host"@[0]);
        assert("BUILD"@[0] != "ci-test-host"@[0]
            && "BUILD"@[0] != "projects-test-host"@[0]
            && "BUILD"@[0] != "workflow-test-tools"@[0]
            && "BUILD"@[0] != "worktree-test-host"@[0]);
    }
    expect(custody::known_custody_producer("ci-test-host"));
    expect(custody::known_custody_producer("worktree-test-host"));
    expect(!custody::known_custody_producer(""));
    expect(!custody::known_custody_producer("unknown command"));
    expect(!custody::known_custody_producer("BUILD"));
}
}

#[cfg(not(verus_keep_ghost))]
#[path = "support/native_executable.rs"]
mod native_executable;

#[cfg(not(verus_keep_ghost))]
mod git_status_io {
    use super::git_fixture;
    use std::{fs, process::Command};

    #[test]
    fn selected_git_mock_status_skips_ignored_subtrees_but_checks_tracked_files() {
        use std::os::unix::ffi::OsStringExt;

        if !git_fixture::isolated(
            "git_status_io::selected_git_mock_status_skips_ignored_subtrees_but_checks_tracked_files",
        ) {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let call = |args: &[&str]| {
            Command::new(git_fixture::program())
                .current_dir(root)
                .args(args)
                .output()
                .unwrap()
        };
        let ok = |args: &[&str]| {
            let output = call(args);
            assert!(output.status.success(), "{args:?}: {output:?}");
            assert!(output.stderr.is_empty(), "{args:?}: {output:?}");
            output.stdout
        };
        ok(&["init", "-q", "-b", "dev"]);
        fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
        fs::write(root.join(".git/info/exclude"), "excluded/\n").unwrap();
        fs::create_dir(root.join("ignored")).unwrap();
        fs::write(root.join("ignored/tracked"), b"before").unwrap();
        ok(&["add", ".gitignore"]);
        ok(&["add", "-f", "ignored/tracked"]);
        ok(&["commit", "-qm", "base"]);
        fs::write(root.join("ignored/tracked"), b"after").unwrap();
        fs::write(root.join("visible"), b"new").unwrap();
        fs::create_dir(root.join("excluded")).unwrap();
        // These unsupported names make an unwanted traversal fail deterministically;
        // status without --ignored has no consumer for either subtree's contents.
        let name = std::ffi::OsString::from_vec(b"private-\xff".to_vec());
        for subtree in ["ignored", "excluded"] {
            fs::write(root.join(subtree).join(&name), b"private").unwrap();
        }
        assert_eq!(
            ok(&["status", "--porcelain", "-z"]),
            b" M ignored/tracked\0?? visible\0"
        );
        for subtree in ["ignored", "excluded"] {
            fs::remove_file(root.join(subtree).join(&name)).unwrap();
            fs::write(root.join(subtree).join("untracked"), b"private").unwrap();
        }
        let output = ok(&["status", "--porcelain", "--ignored", "-z"]);
        let mut records: Vec<_> = output.split(|byte| *byte == 0).collect();
        records.sort();
        assert_eq!(
            records,
            [
                b"".as_slice(),
                b" M ignored/tracked",
                b"!! excluded/untracked",
                b"!! ignored/untracked",
                b"?? visible",
            ]
        );
    }

    #[test]
    fn selected_git_mock_status_keeps_visible_and_explicit_ignored_io_errors() {
        use std::os::unix::fs::PermissionsExt;

        if !git_fixture::isolated(
            "git_status_io::selected_git_mock_status_keeps_visible_and_explicit_ignored_io_errors",
        ) {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let call = |args: &[&str]| {
            Command::new(git_fixture::program())
                .current_dir(root)
                .args(args)
                .output()
                .unwrap()
        };
        for args in [
            vec!["init", "-q", "-b", "dev"],
            vec!["commit", "--allow-empty", "-qm", "base"],
        ] {
            let output = call(&args);
            assert!(output.status.success(), "{args:?}: {output:?}");
        }
        fs::write(root.join(".gitignore"), "blocked/\n").unwrap();
        let blocked = root.join("blocked");
        fs::create_dir(&blocked).unwrap();
        fs::write(blocked.join("payload"), b"private").unwrap();
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).unwrap();
        let denial = fs::read_dir(&blocked).err();
        let ordinary = call(&["status", "--porcelain", "-z"]);
        let explicit = call(&["status", "--porcelain", "--ignored", "-z"]);
        fs::write(root.join(".gitignore"), "").unwrap();
        let visible = call(&["status", "--porcelain", "-z"]);
        // Restore access before inspecting results so failed assertions retain a
        // removable private fixture. Privileged hosts cannot exercise this denial.
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o700)).unwrap();
        let Some(denial) = denial else {
            eprintln!("permission denial is unavailable for this process identity");
            return;
        };
        assert!(ordinary.status.success(), "{ordinary:?}");
        assert_eq!(ordinary.stdout, b"?? .gitignore\0");
        assert!(ordinary.stderr.is_empty(), "{ordinary:?}");
        for output in [explicit, visible] {
            assert_eq!(output.status.code(), Some(93), "{output:?}");
            assert!(output.stdout.is_empty(), "{output:?}");
            let error = String::from_utf8(output.stderr).unwrap();
            assert!(error.contains(blocked.to_str().unwrap()), "{error}");
            assert!(error.contains(&denial.to_string()), "{error}");
        }
    }
}
