//! Validate an explicit inventory against Cargo's configuration lookup rules.
//! Lookup and include parsing check declarations; they never register inputs.
use chrono_harness::file_identity;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};
type Result<T = ()> = std::result::Result<T, String>;
fn error(message: impl std::fmt::Display) -> String {
    format!("E_CARGO_CONFIGURATION: {message}")
}
#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Ancestors {
    Inventory,
    Absent,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Declaration {
    path: String,
    #[serde(deserialize_with = "required_input")]
    input: Option<String>,
}
// Missing is an error; explicit null declares absence.
fn required_input<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    Option::deserialize(d)
}
#[derive(Clone, Serialize)]
pub(crate) struct Evidence {
    files: Vec<File>,
    parsed: BTreeSet<PathBuf>,
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    ancestor_absences: BTreeSet<PathBuf>,
}
#[derive(Clone, Serialize)]
struct File {
    path: PathBuf,
    input: Option<String>,
    sha256: Option<String>,
}
pub(crate) struct Checked {
    pub evidence: Evidence,
    spellings: BTreeSet<PathBuf>,
}
fn normalized(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(error("configuration path must resolve to an absolute path"));
    }
    // Check the original spelling before collapsing `..`; a link before `..`
    // would otherwise let Cargo and this inventory resolve different files.
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(error(format!(
                    "configuration symlink: {}",
                    ancestor.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(error(format!("{}: {e}", ancestor.display()))),
        }
    }
    let mut result = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            Component::RootDir | Component::Normal(_) => result.push(c.as_os_str()),
            _ => return Err(error("unsupported configuration path prefix")),
        }
    }
    Ok(result)
}
// Missing ancestors are allowed for explicitly absent files. A link (including
// a dangling one) is never confused with absence or a regular configuration.
fn present(path: &Path) -> Result<bool> {
    for part in path.ancestors() {
        match fs::symlink_metadata(part) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(error(format!("configuration symlink: {}", part.display())));
            }
            Ok(m) if part == path && !m.is_file() => {
                return Err(error(format!(
                    "configuration is not a regular file: {}",
                    path.display()
                )));
            }
            Ok(m) if part != path && !m.is_dir() => {
                return Err(error(format!(
                    "configuration parent is not a directory: {}",
                    part.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(error(format!("{}: {e}", part.display()))),
        }
    }
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(error(e)),
    }
}
impl Checked {
    pub fn unchanged(&self) -> Result {
        for path in &self.spellings {
            normalized(path)?;
        }
        for row in &self.evidence.files {
            if present(&row.path)? != row.input.is_some() {
                return Err(error(format!(
                    "configuration presence changed during guarded operation: {}",
                    row.path.display()
                )));
            }
        }
        for path in &self.evidence.ancestor_absences {
            if present(path)? {
                return Err(error(format!(
                    "ancestor configuration presence changed during guarded operation: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Include {
    path: String,
    #[serde(default)]
    optional: bool,
}
fn includes(path: &Path) -> Result<Vec<Include>> {
    let config: toml::Value = fs::read_to_string(path)
        .map_err(error)?
        .parse()
        .map_err(error)?;
    config
        .get("include")
        .map(|value| {
            value
                .as_array()
                .ok_or_else(|| error("include must be an array"))?
                .iter()
                .map(|v| {
                    let include = if let Some(path) = v.as_str() {
                        Include {
                            path: path.into(),
                            optional: false,
                        }
                    } else {
                        v.clone().try_into::<Include>().map_err(error)?
                    };
                    if include.path.is_empty()
                        || include.path.contains('\0')
                        || Path::new(&include.path)
                            .extension()
                            .and_then(|s| s.to_str())
                            != Some("toml")
                    {
                        return Err(error("include path must name a .toml file"));
                    }
                    Ok(include)
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}
pub(crate) fn check(
    root: &Path,
    environment: &BTreeMap<String, String>,
    declarations: &[Declaration],
    ancestors: Option<Ancestors>,
    arguments: &[String],
    input: impl Fn(&str) -> Result<PathBuf>,
) -> Result<Checked> {
    let home = environment
        .get("CARGO_HOME")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            error("CARGO_HOME must be explicitly set; implicit HOME lookup is unsupported")
        })?;
    if ancestors.is_none() && !Path::new(home).is_absolute() {
        return Err(error("CARGO_HOME must be an explicit absolute directory"));
    }
    let home = root.join(home);
    let home_directory = normalized(&home)?;
    let mut files = BTreeMap::new();
    let mut evidence = Vec::new();
    let mut spellings = BTreeSet::from([home]);
    for row in declarations {
        if row.path.is_empty() || row.path.contains('\0') {
            return Err(error("empty/invalid configuration path"));
        }
        let spelling = root.join(&row.path);
        let path = normalized(&spelling)?;
        spellings.insert(spelling);
        if files.insert(path.clone(), row.input.is_some()).is_some() {
            return Err(error(format!(
                "duplicate configuration path: {}",
                path.display()
            )));
        }
        if present(&path)? != row.input.is_some() {
            return Err(error(format!(
                "configuration presence differs from declaration: {}",
                path.display()
            )));
        }
        let digest = match &row.input {
            Some(id) => {
                if input(id)? != path {
                    return Err(error(format!(
                        "configuration input location differs: {}",
                        path.display()
                    )));
                }
                Some(file_identity(&path).map_err(error)?.0)
            }
            None => None,
        };
        evidence.push(File {
            path,
            input: row.input.clone(),
            sha256: digest,
        });
    }
    let mut used = BTreeSet::new();
    let mut roots = Vec::new();
    let mut ancestor_absences = BTreeSet::new();
    if ancestors == Some(Ancestors::Absent) {
        for directory in root.ancestors().skip(1) {
            for name in ["config", "config.toml"] {
                let path = directory.join(".cargo").join(name);
                if present(&path)? {
                    return Err(error(format!(
                        "ancestor configuration must be absent: {}",
                        path.display()
                    )));
                }
                if files.contains_key(&path) {
                    return Err(error("ancestor absence policy overlaps an inventory row"));
                }
                ancestor_absences.insert(path);
            }
        }
    }
    for directory in root
        .ancestors()
        .take(if ancestors == Some(Ancestors::Absent) {
            1
        } else {
            usize::MAX
        })
        .map(|p| p.join(".cargo"))
        .chain([home_directory])
    {
        let old = directory.join("config");
        let new = directory.join("config.toml");
        for path in [&old, &new] {
            if !files.contains_key(path) {
                return Err(error(format!(
                    "missing configuration lookup declaration: {}",
                    path.display()
                )));
            }
            used.insert(path.clone());
        }
        if files[&old] {
            roots.push(old);
        } else if files[&new] {
            roots.push(new);
        }
    }
    for argument in arguments {
        let spelling = root.join(argument);
        let path = normalized(&spelling)?;
        spellings.insert(spelling);
        if files.get(&path) != Some(&true) {
            return Err(error(format!(
                "unregistered Cargo configuration argument: {}",
                path.display()
            )));
        }
        used.insert(path.clone());
        roots.push(path);
    }
    let mut parsed = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    let mut pending: Vec<_> = roots.into_iter().map(|p| (p, false)).collect();
    while let Some((path, leave)) = pending.pop() {
        if leave {
            visiting.remove(&path);
            parsed.insert(path);
            continue;
        }
        if parsed.contains(&path) {
            continue;
        }
        if !visiting.insert(path.clone()) {
            return Err(error(format!(
                "configuration include cycle: {}",
                path.display()
            )));
        }
        pending.push((path.clone(), true));
        for include in includes(&path)?.into_iter().rev() {
            let spelling = path.parent().unwrap().join(include.path);
            let target = normalized(&spelling)?;
            spellings.insert(spelling);
            used.insert(target.clone());
            match files.get(&target) {
                Some(true) => pending.push((target, false)),
                Some(false) if include.optional => {}
                Some(false) => {
                    return Err(error(format!(
                        "required configuration include is declared absent: {}",
                        target.display()
                    )));
                }
                None => {
                    return Err(error(format!(
                        "unregistered configuration include: {}",
                        target.display()
                    )));
                }
            }
        }
    }
    if used != files.keys().cloned().collect() {
        return Err(error(
            "configuration inventory contains paths outside lookup/argument/include closure",
        ));
    }
    Ok(Checked {
        evidence: Evidence {
            files: evidence,
            parsed,
            ancestor_absences,
        },
        spellings,
    })
}
