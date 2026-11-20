#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// A failed cleanup must fail a successful test, while an existing panic
/// must keep its original diagnostic and avoid a second unwind.
pub(crate) fn propagate_cleanup_error(test_panicked: bool, cleanup_failed: bool) -> (raise: bool)
    ensures raise == (cleanup_failed && !test_panicked),
{
    cleanup_failed && !test_panicked
}
}

#[cfg(not(verus_keep_ghost))]
use chrono_harness::retained_artifacts::retention::{self, Inventory, Publication};
#[cfg(not(verus_keep_ghost))]
use serde_json::json;
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    path::{Path, PathBuf},
};

/// One test allocation, retained under its business directory on failure.
#[cfg(not(verus_keep_ghost))]
pub struct FailureCase {
    path: PathBuf,
    operation: &'static str,
    publication: Option<Publication>,
}

#[cfg(not(verus_keep_ghost))]
impl FailureCase {
    pub fn new(prefix: &str) -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        Self::allocate(&root, prefix).unwrap()
    }

    pub fn allocate(root: &Path, prefix: &str) -> Result<Self, String> {
        Self::allocate_for(
            root,
            ".chrono-harness/state/release-test-failures",
            prefix,
            "test.host-release-native-tests",
        )
    }

    pub fn allocate_for(
        root: &Path,
        family: &str,
        prefix: &str,
        operation: &'static str,
    ) -> Result<Self, String> {
        let directory = chrono_harness::no_symlink_parents(root, family)?;
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let path = retention::unique_path(&directory, prefix, "")?;
        let relative = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_str()
            .ok_or("fixture path is not UTF-8")?
            .to_owned();
        let owner = Inventory::adopted(root)?.ok_or("missing test output retention policy")?;
        let publication = owner.create_batch(
            "check-original",
            &relative,
            json!({"operation":operation,"fixture":"started"}),
        )?;
        Ok(Self {
            path,
            operation,
            publication: Some(publication),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn finish(&mut self, failed: bool) -> Result<(), String> {
        let publication = self.publication.take().expect("fixture is closed once");
        let outcome = json!({"operation":self.operation,"panicked":failed});
        if failed {
            publication.complete_diagnostic(outcome)
        } else {
            publication.complete(outcome)
        }
    }
}

#[cfg(not(verus_keep_ghost))]
impl Drop for FailureCase {
    fn drop(&mut self) {
        let failed = std::thread::panicking();
        if failed {
            eprintln!(
                "original {} fixture retained: {}",
                self.operation,
                self.path.display()
            );
        }
        let completion = self.finish(failed);
        let raise = propagate_cleanup_error(failed, completion.is_err());
        if let Err(error) = completion {
            if raise {
                panic!(
                    "cannot finish test fixture {}: {error}",
                    self.path.display()
                );
            }
            eprintln!(
                "cannot finish test fixture {}: {error}",
                self.path.display()
            );
        }
    }
}
