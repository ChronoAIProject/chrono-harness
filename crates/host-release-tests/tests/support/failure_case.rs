use chrono_harness::retained_artifacts::retention::{self, Inventory, Publication};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// One directory per release test. The release failure collector reads these
/// batches while their diagnostic window is open.
pub struct FailureCase {
    path: PathBuf,
    relative: String,
    owner: Inventory,
    publication: Option<Publication>,
}

impl FailureCase {
    pub fn new(prefix: &str) -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        Self::allocate(&root, prefix).unwrap()
    }

    pub fn allocate(root: &Path, prefix: &str) -> Result<Self, String> {
        let directory = root.join(".chrono-harness/state/release-test-failures");
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let path = retention::unique_path(&directory, prefix, "")?;
        let relative = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_str()
            .ok_or("release fixture path is not UTF-8")?
            .to_owned();
        let owner =
            Inventory::adopted(root)?.ok_or("missing release verification retention policy")?;
        if !owner.accepts("check-original", &relative) {
            return Err("release verification directory is outside check-original".into());
        }
        let publication = retention::create_adopted(
            root,
            "check-original",
            &path,
            true,
            json!({"operation":"test.host-release-native-tests","fixture":"started"}),
        )?
        .ok_or("release verification directory was not enrolled")?;
        // Also protect a failed construction or an interrupted test process.
        owner.retain_diagnostic_path(&relative)?;
        Ok(Self {
            path,
            relative,
            owner,
            publication: Some(publication),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn finish(&mut self, failed: bool) -> Result<(), String> {
        let publication = self
            .publication
            .take()
            .expect("release fixture is closed once");
        // Keep the original available between publication and the failure
        // collector's diagnostic window. Success uses the ordinary expiry.
        let _reader = if failed {
            Some(self.owner.consume(publication.id())?)
        } else {
            None
        };
        publication
            .complete(json!({"operation":"test.host-release-native-tests","panicked":failed}))?;
        if failed {
            self.owner.retain_diagnostic_path(&self.relative)?;
        }
        Ok(())
    }
}

impl Drop for FailureCase {
    fn drop(&mut self) {
        let failed = std::thread::panicking();
        if failed {
            eprintln!(
                "original release test fixture retained: {}",
                self.path.display()
            );
        }
        if let Err(error) = self.finish(failed) {
            if failed {
                eprintln!(
                    "cannot finish release fixture {}: {error}",
                    self.path.display()
                );
            } else {
                panic!(
                    "cannot finish release fixture {}: {error}",
                    self.path.display()
                );
            }
        }
    }
}
