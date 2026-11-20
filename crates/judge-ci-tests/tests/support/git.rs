//! The diagnostic test's Git model. Only allocation probes may address the
//! enclosing checkout. Cleanup of existing paths belongs to the host owner.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
#[path = "../../../runner-tests/tests/support/git_model.rs"]
mod git_model;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
fn fresh_batch_probe(index: bool, staged_deletions: bool, absent: bool) -> (admitted: bool)
    ensures admitted == (absent && (index || staged_deletions)),
{
    absent && (index || staged_deletions)
}
}

#[cfg(not(verus_keep_ghost))]
fn main() {
    use std::{env, fs, io::ErrorKind, path::Path};
    let argv: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = argv.iter().map(String::as_str).collect();
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    if let [
        "--no-optional-locks",
        "--no-replace-objects",
        "-C",
        root,
        "--literal-pathspecs",
        rest @ ..,
    ] = args.as_slice()
        && Path::new(root) == checkout
    {
        let Some((relative, command)) = rest.split_last() else {
            panic!("missing diagnostic allocation probe");
        };
        let path = Path::new(relative);
        assert_eq!(
            path.parent(),
            Some(Path::new(".chrono-harness/state/judge-ci-test-failures"))
        );
        let suffix = path
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("diagnostic-"))
            .unwrap();
        let mut fields = suffix.split('-');
        assert!(
            (0..3).all(|_| fields
                .next()
                .is_some_and(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit())))
                && fields.next().is_none()
        );
        let missing = fs::symlink_metadata(checkout.join(path))
            .is_err_and(|error| error.kind() == ErrorKind::NotFound);
        assert!(
            fresh_batch_probe(
                command == ["ls-files", "-z", "--"],
                command
                    == [
                        "diff",
                        "--cached",
                        "--name-only",
                        "-z",
                        "--no-renames",
                        "--ignore-submodules=none",
                        "--diff-filter=D",
                        "--"
                    ],
                missing,
            ),
            "only a fresh diagnostic allocation can use the checkout probe"
        );
        return;
    }
    git_model::dispatch(&argv);
}
