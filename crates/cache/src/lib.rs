//! Explicit cache key preparation. A plan is not a cache hit or a build verdict.
use chrono_harness::{decode, file_identity, no_symlink_parents, sha256, wire};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

pub const SCHEMA: &str = "chrono-cache/v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub namespace: String,
    pub artifact_registry: String,
    pub inputs: BTreeMap<String, Input>,
    pub artifacts: BTreeMap<String, Artifact>,
    pub caches: BTreeMap<String, Cache>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Input {
    File { path: String, presence: Presence },
    Environment { name: String, presence: Presence },
    Literal { value: Value },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Presence {
    Present,
    Absent,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Dependencies,
    Compilation,
    ExecutableCandidate,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub owner: String,
    pub path: String,
    pub kind: Kind,
    pub external: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Cache {
    pub owner: String,
    pub producer: String,
    pub consumers: Vec<String>,
    pub artifacts: Vec<String>,
    pub compatibility_inputs: Vec<String>,
    pub source_inputs: Vec<String>,
    pub restore: Restore,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Restore {
    Exact,
    Compatible,
}

fn name(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(&c))
    {
        return Err(format!("E_CACHE_CONFIG: invalid registered name {value:?}"));
    }
    Ok(())
}

fn unique(values: &[String]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        name(value)?;
        if !seen.insert(value) {
            return Err(format!("E_CACHE_CONFIG: duplicate reference {value}"));
        }
    }
    Ok(())
}

fn path(root: &Path, value: &str) -> Result<PathBuf, String> {
    if value.is_empty() || value.contains(['\0', '\n', '\r', '\\', '*', '?', '[', ']']) {
        return Err("E_CACHE_PATH: expected a literal path".into());
    }
    let value = Path::new(value);
    if value
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::RootDir))
    {
        return Err("E_CACHE_PATH: nonliteral path component".into());
    }
    if value.is_absolute() {
        no_symlink_parents(
            Path::new("/"),
            value
                .strip_prefix("/")
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("E_CACHE_PATH: non-UTF8 path")?,
        )
    } else {
        no_symlink_parents(root, value.to_str().ok_or("E_CACHE_PATH: non-UTF8 path")?)
    }
}

pub fn validate(c: &Config) -> Result<(), String> {
    if c.schema != SCHEMA || c.caches.is_empty() {
        return Err("E_CACHE_CONFIG: expected nonempty chrono-cache/v1 registration".into());
    }
    name(&c.namespace)?;
    if !c.artifact_registry.starts_with(".chrono-harness/") {
        return Err("E_CACHE_CONFIG: artifact registry must belong to host .chrono-harness".into());
    }
    chrono_harness::relative_path(&c.artifact_registry)?;
    for (id, input) in &c.inputs {
        name(id)?;
        if let Input::Environment { name, .. } = input {
            if name.is_empty() || name.contains(['=', '\0']) {
                return Err("E_CACHE_CONFIG: invalid environment name".into());
            }
        }
    }
    for (id, artifact) in &c.artifacts {
        name(id)?;
        name(&artifact.owner)?;
        if Path::new(&artifact.path).is_absolute() != artifact.external {
            return Err(format!(
                "E_CACHE_CONFIG: artifact {id} external/path disagreement"
            ));
        }
        if artifact.external && artifact.kind != Kind::Dependencies {
            return Err("E_CACHE_CONFIG: only explicitly external dependency caches may be outside the host".into());
        }
    }
    for (id, cache) in &c.caches {
        name(id)?;
        name(&cache.owner)?;
        name(&cache.producer)?;
        if cache.consumers.is_empty()
            || cache.artifacts.is_empty()
            || cache.compatibility_inputs.is_empty()
            || cache.source_inputs.is_empty()
        {
            return Err(format!(
                "E_CACHE_CONFIG: cache {id} requires consumers, artifacts and both input classes"
            ));
        }
        unique(&cache.consumers)?;
        unique(&cache.artifacts)?;
        unique(&cache.compatibility_inputs)?;
        unique(&cache.source_inputs)?;
        let mut inputs = BTreeSet::new();
        for input in cache
            .compatibility_inputs
            .iter()
            .chain(&cache.source_inputs)
        {
            if !c.inputs.contains_key(input) || !inputs.insert(input) {
                return Err(format!(
                    "E_CACHE_CONFIG: missing or repeated input {input} in {id}"
                ));
            }
        }
        for artifact in &cache.artifacts {
            let a = c
                .artifacts
                .get(artifact)
                .ok_or_else(|| format!("E_CACHE_CONFIG: unknown artifact {artifact}"))?;
            if a.owner != cache.owner {
                return Err(format!(
                    "E_CACHE_CONFIG: artifact {artifact} owner differs from cache {id}"
                ));
            }
            if a.kind == Kind::ExecutableCandidate && cache.restore != Restore::Exact {
                return Err(
                    "E_CACHE_CONFIG: executable candidates require exact restoration".into(),
                );
            }
        }
    }
    Ok(())
}

fn observe(root: &Path, input: &Input) -> Result<Value, String> {
    let observation = match input {
        Input::File {
            path: value,
            presence,
        } => {
            let file = path(root, value)?;
            match fs::symlink_metadata(&file) {
                Err(e)
                    if e.kind() == std::io::ErrorKind::NotFound
                        && *presence == Presence::Absent =>
                {
                    json!({"presence":"absent"})
                }
                Err(e) => return Err(format!("E_CACHE_INPUT: {value}: {e}")),
                Ok(_) if *presence == Presence::Absent => {
                    return Err(format!("E_CACHE_INPUT: expected absent file {value}"));
                }
                Ok(_) => {
                    let (digest, length) = file_identity(&file)?;
                    json!({"presence":"present","sha256":digest,"length":length})
                }
            }
        }
        Input::Environment { name, presence } => match std::env::var(name) {
            Err(std::env::VarError::NotPresent) if *presence == Presence::Absent => {
                json!({"presence":"absent"})
            }
            Err(std::env::VarError::NotPresent) => {
                return Err(format!("E_CACHE_INPUT: missing variable {name}"));
            }
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(format!("E_CACHE_INPUT: non-UTF8 variable {name}"));
            }
            Ok(_) if *presence == Presence::Absent => {
                return Err(format!("E_CACHE_INPUT: expected absent variable {name}"));
            }
            Ok(value) => {
                json!({"presence":"present","sha256":sha256(value.as_bytes()),"length":value.len()})
            }
        },
        Input::Literal { value } => json!({"declaration_sha256":wire::digest(value)?}),
    };
    Ok(json!({"declaration":input,"observation":observation}))
}

/// Read only the selected consumer's explicitly named inputs. Literal values are
/// declarations; hashing them does not turn them into toolchain observations.
pub fn prepare(root: &Path, c: &Config, consumer: &str) -> Result<Value, String> {
    validate(c)?;
    name(consumer)?;
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let selected: BTreeMap<_, _> = c
        .caches
        .iter()
        .filter(|(_, cache)| cache.consumers.iter().any(|v| v == consumer))
        .collect();
    if selected.is_empty() {
        return Err(format!("E_CACHE_CONSUMER: no registration for {consumer}"));
    }
    let registry: Value = decode(
        &fs::read(no_symlink_parents(&root, &c.artifact_registry)?).map_err(|e| e.to_string())?,
    )?;
    let registered = registry["artifacts"]
        .as_array()
        .ok_or("E_CACHE_CONFIG: artifact registry needs an artifacts array")?;
    let mut observations = BTreeMap::new();
    let mut paths = BTreeSet::new();
    let mut entries = BTreeMap::new();
    for (id, cache) in selected {
        let mut artifacts = BTreeMap::new();
        let mut bindings = BTreeMap::new();
        for artifact_id in &cache.artifacts {
            let artifact = &c.artifacts[artifact_id];
            if !artifact.external {
                let found: Vec<_> = registered
                    .iter()
                    .filter(|row| row["path"] == artifact.path)
                    .collect();
                if found.len() != 1
                    || found[0]["owner"] != artifact.owner
                    || found[0]["tracked"] != false
                    || found[0]["kind"] == "execution-evidence"
                {
                    return Err(format!(
                        "E_CACHE_ARTIFACT: {artifact_id} requires one matching untracked, nonevidence artifact registration"
                    ));
                }
                bindings.insert(artifact_id, found[0]);
            }
            let absolute = path(&root, &artifact.path)?;
            if !absolute.starts_with(&root) && !artifact.external {
                return Err("E_CACHE_PATH: host artifact escaped its root".into());
            }
            for protected in [root.join(".git"), root.join(".chrono-harness/state")] {
                if absolute.starts_with(&protected) || protected.starts_with(&absolute) {
                    return Err(
                        "E_CACHE_PATH: Git metadata and execution state cannot be cached".into(),
                    );
                }
            }
            if paths
                .iter()
                .any(|p: &PathBuf| p.starts_with(&absolute) || absolute.starts_with(p))
            {
                return Err("E_CACHE_PATH: selected cache artifacts overlap".into());
            }
            paths.insert(absolute);
            artifacts.insert(artifact_id, artifact);
        }
        for input_id in cache
            .compatibility_inputs
            .iter()
            .chain(&cache.source_inputs)
        {
            if !observations.contains_key(input_id) {
                observations.insert(input_id.clone(), observe(&root, &c.inputs[input_id])?);
            }
        }
        let values = |ids: &[String]| -> BTreeMap<_, _> {
            ids.iter()
                .map(|id| (id.clone(), observations[id].clone()))
                .collect()
        };
        let compatibility =
            wire::digest(&json!({"schema":SCHEMA,"namespace":c.namespace,"cache":id,
            "owner":cache.owner,"producer":cache.producer,
            "consumers":cache.consumers.iter().collect::<BTreeSet<_>>(),
            "restore":cache.restore,"artifacts":artifacts,
            "artifact_registry":c.artifact_registry,"artifact_bindings":bindings,
            "inputs":values(&cache.compatibility_inputs)}))?;
        let source = wire::digest(&values(&cache.source_inputs))?;
        let prefix = format!("chrono-v1-{compatibility}-");
        entries.insert(
            id,
            json!({"owner":cache.owner,"producer":cache.producer,
            "artifacts":artifacts,"key":format!("{prefix}{source}"),
            "restore_keys":if cache.restore == Restore::Compatible {vec![prefix]} else {vec![]},
            "handling":"registered-build-required","status":"prepared-unrestored"}),
        );
    }
    for id in observations.keys() {
        if let Input::File { path: input, .. } = &c.inputs[id] {
            let input = path(&root, input)?;
            if paths.iter().any(|artifact| input.starts_with(artifact)) {
                return Err(format!(
                    "E_CACHE_PATH: registered input {id} is inside a selected cache artifact"
                ));
            }
        }
    }
    Ok(
        json!({"schema":"chrono-plan/v1","consumer":consumer,"caches":entries,
        "inputs":observations,"input_completeness_proven":false,"execution":"not-started"}),
    )
}

pub fn dispatch(args: &[String]) -> Result<String, String> {
    if args.first().map(String::as_str) != Some("plan") {
        return Err("expected chrono-cache plan".into());
    }
    let mut options = BTreeMap::new();
    for pair in args.get(1..).unwrap_or_default().chunks(2) {
        if pair.len() != 2
            || !["--host-root", "--config", "--consumer"].contains(&pair[0].as_str())
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("plan requires unique host-root/config/consumer options".into());
        }
    }
    if options.len() != 3 {
        return Err("plan requires host-root/config/consumer".into());
    }
    let root = Path::new(options["--host-root"]);
    let config = options["--config"];
    if !config.starts_with(".chrono-harness/") {
        return Err("E_CACHE_CONFIG: registration must belong to host .chrono-harness".into());
    }
    let c: Config =
        decode(&fs::read(no_symlink_parents(root, config)?).map_err(|e| e.to_string())?)?;
    Ok(
        serde_json::to_string(&prepare(root, &c, options["--consumer"])?)
            .map_err(|e| e.to_string())?
            + "\n",
    )
}
