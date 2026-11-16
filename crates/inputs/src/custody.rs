//! Input callers own their exact output capability until handoff or last use.
use super::*;
use chrono_harness::retained_artifacts::retention::{
    self, Consumer, Inventory, OriginalIdentity, Publication, ReadScope,
};

/// Keep this value alive while using its retained paths. Serialization copies
/// the snapshot, not its ownership; a different process needs an explicit handoff.
pub struct Produced {
    pub(crate) value: Value,
    root: PathBuf,
    primary: String,
    _scope: ReadScope,
}

impl std::ops::Deref for Produced {
    type Target = Value;
    fn deref(&self) -> &Value {
        &self.value
    }
}
impl std::fmt::Debug for Produced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.value, f)
    }
}
impl PartialEq for Produced {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl PartialEq<Value> for Produced {
    fn eq(&self, other: &Value) -> bool {
        &self.value == other
    }
}
impl PartialEq<Produced> for Value {
    fn eq(&self, other: &Produced) -> bool {
        self == &other.value
    }
}
impl serde::Serialize for Produced {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&self.value, serializer)
    }
}
impl Produced {
    pub(crate) fn new(
        root: &Path,
        primary: &str,
        value: Value,
        scope: ReadScope,
    ) -> Result<Self, String> {
        Ok(Self {
            value,
            root: fs::canonicalize(root).map_err(|e| e.to_string())?,
            primary: primary.into(),
            _scope: scope,
        })
    }

    /// The named caller accepts the primary original and its declared closure.
    /// It releases this declaration through the existing retention interface
    /// after its actual downstream use or durable handoff has finished.
    pub fn handoff(&self, consumer: &str) -> Result<Consumer, String> {
        let owner = Inventory::adopted(&self.root)?
            .ok_or("E_INPUT_RETENTION: caller handoff requires adopted retention")?;
        let status = owner.producer_status("full-original", Some(&self.primary))?;
        let records = status["records"]
            .as_array()
            .ok_or("input producer status")?;
        let [record] = records.as_slice() else {
            return Err(
                "E_INPUT_RETENTION: existing unknown output requires its original custodian".into(),
            );
        };
        let original = OriginalIdentity {
            id: record["id"].as_str().ok_or("input output id")?.into(),
            path: self.primary.clone(),
            identity: record["identity"]
                .as_str()
                .ok_or("input output identity")?
                .into(),
        };
        // Compose returns the pair but its primary original is the provenance
        // receipt. Qualify the actual original bytes under the live capability.
        let primary = state(&self.root, &self.primary)?;
        let (digest, length) = file_identity(&primary)?;
        if !owner.published_original(&self.primary, &digest, length)? {
            return Err("E_INPUT_RETENTION: input original is not sealed".into());
        }
        owner.protect_consumer(consumer, std::slice::from_ref(&original))?;
        Ok(Consumer {
            schema: "chrono-retention-consumer/v1".into(),
            consumer: consumer.into(),
            purpose: "调用方保管输入导出，直到后续使用完成或已交接，并释放此声明".into(),
            originals: vec![original],
        })
    }
}

pub(crate) fn dependencies(root: &Path, output: &str, paths: &[String]) -> Result<(), String> {
    if let Some(owner) = Inventory::adopted(root)? {
        owner.dependencies_paths(output, paths)?;
    }
    Ok(())
}

pub(crate) fn blobs(snapshot: &Value) -> Vec<String> {
    snapshot["files"]
        .as_object()
        .into_iter()
        .flat_map(|files| files.values())
        .filter_map(|file| file["blob"].as_str().map(str::to_owned))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) struct BoundedFile {
    file: fs::File,
    remaining: u64,
}
impl Write for BoundedFile {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(std::io::Error::other(
                "E_INPUT_BLOB: source exceeds declared length",
            ));
        }
        let written = self.file.write(bytes)?;
        self.remaining -= written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

/// A registered staging allocation survives interruption under the same finite
/// owner policy. Unadopted callers retain the ordinary RAII cleanup behavior.
pub(crate) struct StagedBlob {
    pub path: PathBuf,
    managed: bool,
    publication: Option<Publication>,
}
impl Drop for StagedBlob {
    fn drop(&mut self) {
        if !self.managed {
            let _ = fs::remove_file(&self.path);
        }
    }
}
impl StagedBlob {
    pub fn write(
        root: &Path,
        length: u64,
        write: impl FnOnce(&mut BoundedFile) -> Result<(String, u64), String>,
    ) -> Result<(Self, String, u64), String> {
        let directory = state(root, BLOB_DIRECTORY)?;
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let path = retention::unique_path(&directory, "input-stage", ".bytes")?;
        let publication = retention::create_adopted_with_limit(
            root,
            "full-original",
            &path,
            false,
            value!({"kind":"input-staging","outcome":"not-established"}),
            Some(length.max(1)),
        )?;
        let mut stage = Self {
            path,
            managed: publication.is_some(),
            publication,
        };
        let mut output = BoundedFile {
            file: fs::OpenOptions::new()
                .write(true)
                .open(&stage.path)
                .map_err(|e| e.to_string())?,
            remaining: length,
        };
        let (digest, copied) = write(&mut output)
            .map_err(|e| format!("{e}; input staging original: {}", stage.path.display()))?;
        if copied != length || output.remaining != 0 {
            return Err(format!(
                "E_INPUT_BLOB: source length changed; input staging original: {}",
                stage.path.display()
            ));
        }
        output.file.sync_all().map_err(|e| e.to_string())?;
        if let Some(publication) = stage.publication.take() {
            publication.complete_sealed(
                value!({"kind":"input-staging","sha256":digest,"length":copied}),
            )?;
        }
        Ok((stage, digest, copied))
    }
}
