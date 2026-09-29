//! Migrate explicit owned projections while preserving the host's next configuration.
use super::*;
use std::collections::BTreeMap;

fn source_path(path: &str) -> Result<(), String> {
    relative_path(path)?;
    if !path.starts_with(".chrono-harness/ci/") {
        return Err("CI source must be an explicit host CI path".into());
    }
    Ok(())
}

fn previous(root: &Path, input: &str, source: &str) -> Result<BTreeMap<String, String>, String> {
    match load_projection(&no_symlink_parents(root, input)?)? {
        Projection::Units(c) => units::render(&c, source),
        Projection::Check(c) if c.schema == "chrono-github-ci/v1" => {
            Ok(BTreeMap::from([(c.workflow_path.clone(), render(&c, source)?)]))
        }
        _ => Err("migration supports scoped v1 and unit sources; initial/full/release contracts require their own explicit migration".into()),
    }
}

fn exact(root: &Path, path: &str, expected: &[u8]) -> Result<(), String> {
    let actual = fs::read(no_symlink_parents(root, path)?).map_err(|e| format!("{path}: {e}"))?;
    if actual != expected {
        return Err(format!(
            "previous projection differs; preserve host edits: {path}"
        ));
    }
    Ok(())
}

pub fn migrate(root: &Path, input: &str, old_source: &str, source: &str) -> Result<String, String> {
    relative_path(input)?;
    source_path(old_source)?;
    source_path(source)?;
    if !input.starts_with(".chrono-harness/") {
        return Err("previous input must be an explicit host harness path".into());
    }
    let old_bytes = fs::read(no_symlink_parents(root, input)?).map_err(|e| e.to_string())?;
    let source_bytes = fs::read(no_symlink_parents(root, source)?).map_err(|e| e.to_string())?;
    let old = previous(root, input, old_source)?;
    let Projection::Units(next) = load_projection(&no_symlink_parents(root, source)?)? else {
        return Err("migration target must be an explicitly configured unit provider".into());
    };
    units::profile(root, &next)?;
    let new = units::render(&next, source)?;

    // A prior declaration is authority only for its exact still-present projection.
    // Check the complete old and new output sets before the first write or removal.
    for (path, text) in &old {
        exact(root, path, text.as_bytes())?;
    }
    let mut writes = vec![];
    for (path, text) in &new {
        let target = no_symlink_parents(root, path)?;
        match fs::read(&target) {
            Ok(bytes) if bytes == text.as_bytes() => {}
            Ok(_) if old.contains_key(path) => writes.push((path, text)),
            Ok(_) => {
                return Err(format!(
                    "new workflow collision; preserve host file: {path}"
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => writes.push((path, text)),
            Err(e) => return Err(format!("{path}: {e}")),
        }
    }
    let retire: Vec<_> = old
        .keys()
        .filter(|p| !new.contains_key(*p))
        .cloned()
        .collect();
    let retire_source = (old_source != source).then_some(old_source);
    if let Some(path) = retire_source {
        exact(root, path, &old_bytes)?;
    }
    exact(root, source, &source_bytes)?;
    exact(root, input, &old_bytes)?;

    let mut written = vec![];
    let mut removed = vec![];
    let result: Result<(), String> = (|| {
        for (path, text) in writes {
            write_file(root, path, text.as_bytes())?;
            written.push(path.clone());
        }
        for path in &retire {
            exact(root, path, old[path].as_bytes())?;
            fs::remove_file(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
            removed.push(path.clone());
        }
        if let Some(path) = retire_source {
            exact(root, path, &old_bytes)?;
            fs::remove_file(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    result.map_err(|e| {
        format!("migration stopped after writing {written:?} and retiring {removed:?}: {e}")
    })?;
    Ok(serde_json::to_string(&json!({
        "schema":"chrono-ci-migration/v1", "status":"migrated",
        "previous_input":input,"previous_config":old_source,"config":source,
        "previous_input_sha256":chrono_harness::sha256(&old_bytes),
        "config_sha256":chrono_harness::sha256(&source_bytes),
        "written_workflows":written,"retired_workflows":removed,"retired_source":retire_source,
        "governance":"not-evaluated","atomicity":"not-a-multifile-transaction"
    }))
    .map_err(|e| e.to_string())?
        + "\n")
}
