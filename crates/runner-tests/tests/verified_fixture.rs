//! 命令替身的封闭派发域；IO 行为仍由独立 Rust 边界预期核验。
#![allow(unexpected_cfgs)]
#[path = "../../worktree-tests/tests/support/cargo_model.rs"]
mod cargo;
#[cfg(verus_keep_ghost)]
#[path = "../../worktree-tests/tests/support/fixture_coordinator.rs"]
mod custody;
#[path = "../../distribution-tests/tests/support/bootstrap.rs"]
mod distribution_bootstrap;
#[path = "../../ci-tests/tests/support/generated_command.rs"]
mod generated_command;
#[path = "support/git_model.rs"]
mod git;
#[cfg(verus_keep_ghost)]
#[path = "support/git_fixture.rs"]
mod git_fixture;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[path = "support/isolated_capability.rs"]
mod isolation;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;
verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn custody_release_requires_original_child_result_and_owner_retirement() {
    expect(isolation::release_custody(true, true));
    expect(!isolation::release_custody(false, true));
    expect(!isolation::release_custody(true, false));
    expect(!isolation::release_custody(false, false));
}
#[cfg_attr(not(verus_keep_ghost), test)]
fn isolation_requires_complete_test_identity() {
    proof {reveal_strlit("case");reveal_strlit("case-extra");reveal_strlit("");}
    expect(isolation::same_test("case",Some("case")));
    expect(!isolation::same_test("case",Some("case-extra")));
    expect(!isolation::same_test("case",Some("")));
    expect(!isolation::same_test("case",None));
}
#[cfg_attr(not(verus_keep_ghost), test)]
fn generated_command_has_closed_targets() {
    proof {reveal_strlit("chrono-harness");reveal_strlit("$CHRONO_BASE");reveal_strlit("/usr/bin/printf");reveal_strlit("/bin/sh");reveal_strlit("");}
    expect(generated_command::known_target("chrono-harness"));
    expect(generated_command::known_target("$CHRONO_BASE"));
    expect(generated_command::known_target("/usr/bin/printf"));
    expect(!generated_command::known_target("/bin/sh"));
    expect(!generated_command::known_target(""));
}
#[cfg_attr(not(verus_keep_ghost), test)]
fn distribution_bootstrap_has_closed_modes() {
    proof {reveal_strlit("install");reveal_strlit("adopt");reveal_strlit("shell");reveal_strlit("");}
    expect(distribution_bootstrap::known_mode("install"));
    expect(distribution_bootstrap::known_mode("adopt"));
    expect(!distribution_bootstrap::known_mode("shell"));
    expect(!distribution_bootstrap::known_mode(""));
}
fn expect(actual:bool) requires actual, { if !actual {panic!();} }
#[cfg_attr(not(verus_keep_ghost), test)]
fn known_git_command_independent_boundaries() {
    proof { reveal_strlit("--version"); reveal_strlit("check-ref-format"); reveal_strlit("init"); reveal_strlit("clone"); reveal_strlit("add"); reveal_strlit("commit"); reveal_strlit("commit-tree"); reveal_strlit("rev-parse"); reveal_strlit("show"); reveal_strlit("ls-tree"); reveal_strlit("cat-file"); reveal_strlit("ls-files"); reveal_strlit("config"); reveal_strlit("diff"); reveal_strlit("update-index"); reveal_strlit("update-ref"); reveal_strlit("reset"); reveal_strlit("checkout"); reveal_strlit("switch"); reveal_strlit("status"); reveal_strlit("symbolic-ref"); reveal_strlit("branch"); reveal_strlit("show-ref"); reveal_strlit("for-each-ref"); reveal_strlit("remote"); reveal_strlit("ls-remote"); reveal_strlit("push"); reveal_strlit("fetch"); reveal_strlit("merge-base"); reveal_strlit("rev-list"); reveal_strlit("merge"); reveal_strlit("write-tree"); reveal_strlit("read-tree"); reveal_strlit("mv"); reveal_strlit("rm"); reveal_strlit("replace"); reveal_strlit("reflog"); reveal_strlit("worktree"); reveal_strlit(""); reveal_strlit("unknown command"); reveal_strlit("BUILD"); }
    expect(git::known_git_command("--version"));
    expect(git::known_git_command("worktree"));
    expect(!git::known_git_command(""));
    expect(!git::known_git_command("unknown command"));
    expect(!git::known_git_command("BUILD"));
}
#[cfg_attr(not(verus_keep_ghost), test)]
fn known_cargo_command_independent_boundaries() {
    proof { reveal_strlit("--native-holder"); reveal_strlit("--version"); reveal_strlit("build"); reveal_strlit("test"); reveal_strlit("generate-lockfile"); reveal_strlit(""); reveal_strlit("unknown command"); reveal_strlit("BUILD"); }
    expect(cargo::known_cargo_command("--native-holder"));
    expect(cargo::known_cargo_command("generate-lockfile"));
    expect(!cargo::known_cargo_command(""));
    expect(!cargo::known_cargo_command("unknown command"));
    expect(!cargo::known_cargo_command("BUILD"));
}
#[cfg(verus_keep_ghost)]
fn known_custody_producer_independent_boundaries() {
    proof { reveal_strlit("ci-test-host"); reveal_strlit("projects-test-host"); reveal_strlit("workflow-test-tools"); reveal_strlit("worktree-test-host"); reveal_strlit(""); reveal_strlit("unknown command"); reveal_strlit("BUILD"); }
    expect(custody::known_custody_producer("ci-test-host"));
    expect(custody::known_custody_producer("worktree-test-host"));
    expect(!custody::known_custody_producer(""));
    expect(!custody::known_custody_producer("unknown command"));
    expect(!custody::known_custody_producer("BUILD"));
}
}
