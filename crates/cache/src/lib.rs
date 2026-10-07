//! Explicit cache key preparation. A plan is not a cache hit or a build verdict.
mod backend;
mod probe;
mod recovery;
mod report;
pub use backend::Config as Backend;
use chrono_harness::{decode, file_identity, no_symlink_parents, sha256, wire};
pub use report::transport_report;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

pub const SCHEMA: &str = "chrono-cache/v1";
pub const CONSUMER_SCHEMA: &str = "chrono-cache/v2";

pub(crate) fn current_executable_identity() -> Result<Value, String> {
    let executable =
        fs::canonicalize(std::env::current_exe().map_err(|e| format!("E_CACHE_EXECUTABLE: {e}"))?)
            .map_err(|e| format!("E_CACHE_EXECUTABLE: {e}"))?;
    let (sha256, bytes) = file_identity(&executable)?;
    Ok(json!({
        "executable": executable,
        "sha256": sha256,
        "bytes": bytes,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub namespace: String,
    pub artifact_registry: String,
    pub inputs: BTreeMap<String, Input>,
    pub artifacts: BTreeMap<String, Artifact>,
    pub caches: BTreeMap<String, Cache>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub consumer_operations: BTreeMap<String, Vec<OperationSource>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub require_primary_checkout: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<Backend>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recover_failed_restores: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub recover_unconfirmed_restores: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSource {
    pub registry: String,
    pub pointer: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Input {
    File {
        path: String,
        presence: Presence,
    },
    JsonValue {
        path: String,
        pointer: String,
    },
    Environment {
        name: String,
        presence: Presence,
    },
    Literal {
        value: Value,
    },
    Command {
        command: chrono_harness::CommandSpec,
        inherit: Vec<String>,
        result: probe::ResultKind,
    },
    RegisteredFiles {
        registry: String,
        node: String,
        edges: Vec<String>,
    },
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
    if ![SCHEMA, CONSUMER_SCHEMA].contains(&c.schema.as_str()) || c.caches.is_empty() {
        return Err("E_CACHE_CONFIG: expected nonempty chrono-cache/v1 or v2 registration".into());
    }
    if c.schema == SCHEMA && !c.consumer_operations.is_empty() {
        return Err("E_CACHE_CONFIG: consumer operation contracts require chrono-cache/v2".into());
    }
    if c.recover_failed_restores && (c.schema != CONSUMER_SCHEMA || !c.require_primary_checkout) {
        return Err(
            "E_CACHE_RECOVERY: failed-restore recovery requires v2 and primary-checkout protection"
                .into(),
        );
    }
    if c.recover_unconfirmed_restores && !c.recover_failed_restores {
        return Err(
            "E_CACHE_RECOVERY: unconfirmed-restore recovery requires adopted failed-restore recovery"
                .into(),
        );
    }
    if let Some(backend) = &c.backend {
        if c.schema != CONSUMER_SCHEMA {
            return Err("E_CACHE_CONFIG: backend observations require chrono-cache/v2".into());
        }
        backend::validate(backend)?;
    }
    if c.schema == CONSUMER_SCHEMA {
        let consumers: BTreeSet<_> = c.caches.values().flat_map(|c| &c.consumers).collect();
        for consumer in &consumers {
            if !c.consumer_operations.contains_key(*consumer) {
                return Err(format!(
                    "E_CACHE_CONSUMER: missing operation contract for {consumer}"
                ));
            }
        }
        for (consumer, sources) in &c.consumer_operations {
            name(consumer)?;
            if !consumers.contains(consumer) || sources.is_empty() {
                return Err(format!(
                    "E_CACHE_CONSUMER: unused or empty operation contract for {consumer}"
                ));
            }
            let mut seen = BTreeSet::new();
            for source in sources {
                chrono_harness::relative_path(&source.registry)?;
                if !source.registry.starts_with(".chrono-harness/")
                    || !source.pointer.starts_with('/')
                    || !seen.insert((&source.registry, &source.pointer))
                {
                    return Err(format!(
                        "E_CACHE_CONSUMER: invalid operation source for {consumer}"
                    ));
                }
            }
        }
    }
    name(&c.namespace)?;
    if !c.artifact_registry.starts_with(".chrono-harness/") {
        return Err("E_CACHE_CONFIG: artifact registry must belong to host .chrono-harness".into());
    }
    chrono_harness::relative_path(&c.artifact_registry)?;
    for (id, input) in &c.inputs {
        name(id)?;
        if let Input::JsonValue { pointer, .. } = input {
            if (!pointer.is_empty() && !pointer.starts_with('/'))
                || pointer
                    .split('~')
                    .skip(1)
                    .any(|part| !part.starts_with('0') && !part.starts_with('1'))
            {
                return Err("E_CACHE_CONFIG: json-value requires an RFC 6901 pointer".into());
            }
        }
        if let Input::RegisteredFiles {
            registry,
            node,
            edges,
        } = input
        {
            chrono_harness::relative_path(registry)?;
            if !registry.starts_with(".chrono-harness/") || node.is_empty() || edges.is_empty() {
                return Err("E_CACHE_CONFIG: registered file inputs need a host registry, node and edge kinds".into());
            }
            unique(edges)?;
        }
        if let Input::Command {
            command, inherit, ..
        } = input
        {
            chrono_harness::validate_command(command)?;
            let mut names = BTreeSet::new();
            for name in inherit {
                if name.is_empty()
                    || name.contains(['=', '\0'])
                    || !names.insert(name)
                    || command.env.contains_key(name)
                {
                    return Err("E_CACHE_CONFIG: invalid or ambiguous probe environment".into());
                }
            }
        }
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

fn consumer_operations(root: &Path, c: &Config, consumer: &str) -> Result<Value, String> {
    let mut operations = BTreeSet::new();
    let mut sources = Vec::new();
    let mut registries = BTreeMap::new();
    for source in &c.consumer_operations[consumer] {
        if !registries.contains_key(&source.registry) {
            let raw = fs::read(no_symlink_parents(root, &source.registry)?)
                .map_err(|e| format!("E_CACHE_CONSUMER: {}: {e}", source.registry))?;
            let value: Value = decode(&raw)?;
            registries.insert(source.registry.clone(), (sha256(&raw), value));
        }
        let (digest, registry) = &registries[&source.registry];
        let value = registry.pointer(&source.pointer).ok_or_else(|| {
            format!(
                "E_CACHE_CONSUMER: missing {}#{} for {consumer}",
                source.registry, source.pointer
            )
        })?;
        let values = if value.is_string() {
            vec![value]
        } else {
            value
                .as_array()
                .ok_or_else(|| {
                    format!(
                        "E_CACHE_CONSUMER: expected operation IDs at {}#{}",
                        source.registry, source.pointer
                    )
                })?
                .iter()
                .collect()
        };
        let mut selected = BTreeSet::new();
        for value in values {
            let operation = value
                .as_str()
                .ok_or("E_CACHE_CONSUMER: operation ID must be a string")?;
            name(operation).map_err(|e| format!("E_CACHE_CONSUMER: {e}"))?;
            if !selected.insert(operation.to_owned()) {
                return Err(format!("E_CACHE_CONSUMER: duplicate operation {operation}"));
            }
        }
        if selected.is_empty() {
            return Err("E_CACHE_CONSUMER: operation source is empty".into());
        }
        operations.extend(selected.iter().cloned());
        sources.push(json!({"registry":source.registry,"pointer":source.pointer,
            "sha256":digest,"operations":selected}));
    }
    for (id, cache) in &c.caches {
        if cache.consumers.iter().any(|v| v == consumer) && !operations.contains(&cache.producer) {
            return Err(format!(
                "E_CACHE_CONSUMER: {consumer} does not register producer {} for cache {id}",
                cache.producer
            ));
        }
    }
    Ok(json!({"status":"declared","operations":operations,"sources":sources}))
}

fn observe(root: &Path, input: &Input) -> Result<Value, String> {
    if let Input::JsonValue {
        path: value,
        pointer,
    } = input
    {
        let raw =
            fs::read(path(root, value)?).map_err(|e| format!("E_CACHE_INPUT: {value}: {e}"))?;
        let document: Value = decode(&raw)?;
        let selected = document
            .pointer(pointer)
            .ok_or_else(|| format!("E_CACHE_INPUT: missing JSON value {value}#{pointer}"))?;
        return Ok(json!({"declaration":input,
            "observation":{"sha256":wire::digest(selected)?},
            "source_file":{"sha256":sha256(&raw),"length":raw.len()}}));
    }
    if let Input::Command {
        command,
        inherit,
        result,
    } = input
    {
        let (observation, original) = probe::observe(root, command, inherit, result)?;
        return Ok(json!({"declaration":input,"observation":observation,"original":original}));
    }
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
        Input::RegisteredFiles {
            registry,
            node,
            edges,
        } => registered_files(root, registry, node, edges)?,
        Input::Command { .. } | Input::JsonValue { .. } => {
            unreachable!("command and JSON inputs return their observations above")
        }
    };
    Ok(json!({"declaration":input,"observation":observation}))
}

fn registered_files(
    root: &Path,
    registry: &str,
    node: &str,
    kinds: &[String],
) -> Result<Value, String> {
    let registry: Value =
        decode(&fs::read(no_symlink_parents(root, registry)?).map_err(|e| e.to_string())?)?;
    if registry["schema_version"] != 2 {
        return Err("E_CACHE_INPUT: registered files require FILEMAP v2".into());
    }
    let declared = registry["project_edges"]
        .as_array()
        .ok_or("E_CACHE_INPUT: project edges missing")?;
    let mut nodes = BTreeSet::from([node.to_owned()]);
    let mut edges = BTreeSet::new();
    loop {
        let before = nodes.len();
        for edge in declared {
            let kind = edge["kind"].as_str().ok_or("E_CACHE_INPUT: edge kind")?;
            let from = edge["from"].as_str().ok_or("E_CACHE_INPUT: edge source")?;
            let to = edge["to"]
                .as_str()
                .ok_or("E_CACHE_INPUT: edge destination")?;
            if kinds.iter().any(|v| v == kind) && nodes.contains(to) {
                nodes.insert(from.to_owned());
                edges.insert((from.to_owned(), kind.to_owned(), to.to_owned()));
            }
        }
        if before == nodes.len() {
            break;
        }
    }
    let mut files = BTreeMap::new();
    for file in registry["files"]
        .as_array()
        .ok_or("E_CACHE_INPUT: files missing")?
    {
        let file_edges = file["edges"]
            .as_array()
            .ok_or("E_CACHE_INPUT: file edges missing")?;
        let selected = file_edges
            .iter()
            .filter(|edge| {
                edge["kind"]
                    .as_str()
                    .is_some_and(|k| kinds.iter().any(|v| v == k))
                    && edge["to"].as_str().is_some_and(|n| nodes.contains(n))
            })
            .map(|edge| {
                (
                    edge["kind"].as_str().unwrap().to_owned(),
                    edge["to"].as_str().unwrap().to_owned(),
                )
            })
            .collect::<BTreeSet<_>>();
        if selected.is_empty() {
            continue;
        }
        let name = file["path"].as_str().ok_or("E_CACHE_INPUT: file path")?;
        let (digest, length) = file_identity(&no_symlink_parents(root, name)?)?;
        if files
            .insert(
                name.to_owned(),
                json!({"sha256":digest,"length":length,"edges":selected}),
            )
            .is_some()
        {
            return Err("E_CACHE_INPUT: duplicate selected file".into());
        }
    }
    if files.is_empty() {
        return Err(format!("E_CACHE_INPUT: no registered files for {node}"));
    }
    Ok(json!({"nodes":nodes,"edges":edges,"files":files}))
}

/// Read only the selected consumer's explicitly named inputs. Literal values are
/// declarations; hashing them does not turn them into toolchain observations.
pub fn prepare(root: &Path, c: &Config, consumer: &str) -> Result<Value, String> {
    prepare_with_inputs(root, c, consumer, None)
}

fn prepare_with_inputs(
    root: &Path,
    c: &Config,
    consumer: &str,
    prior: Option<&Value>,
) -> Result<Value, String> {
    validate(c)?;
    name(consumer)?;
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    if c.require_primary_checkout {
        let git = root.join(".git");
        let metadata = fs::symlink_metadata(&git).map_err(|e| format!("E_CACHE_OWNERSHIP: {e}"))?;
        let shared = match fs::symlink_metadata(git.join("commondir")) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(format!("E_CACHE_OWNERSHIP: {error}")),
        };
        if !metadata.is_dir() || shared {
            return Err(
                "E_CACHE_OWNERSHIP: this cache transport requires a primary Git checkout".into(),
            );
        }
    }
    let selected: BTreeMap<_, _> = c
        .caches
        .iter()
        .filter(|(_, cache)| cache.consumers.iter().any(|v| v == consumer))
        .collect();
    if selected.is_empty() {
        return Err(format!("E_CACHE_CONSUMER: no registration for {consumer}"));
    }
    let operation_contract = if c.schema == CONSUMER_SCHEMA {
        Some(consumer_operations(&root, c, consumer)?)
    } else {
        None
    };
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
            if c.recover_failed_restores && artifact.external {
                return Err(
                    "E_CACHE_RECOVERY: external artifacts require their own lifecycle protection"
                        .into(),
                );
            }
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
                let observed = if let Some(prior) = prior {
                    let value = prior
                        .get(input_id)
                        .ok_or("E_CACHE_RECOVERY: missing original input")?;
                    if value["declaration"]
                        != serde_json::to_value(&c.inputs[input_id]).map_err(|e| e.to_string())?
                    {
                        return Err("E_CACHE_RECOVERY: original input declaration differs".into());
                    }
                    value.clone()
                } else {
                    observe(&root, &c.inputs[input_id])?
                };
                observations.insert(input_id.clone(), observed);
            }
        }
        let values = |ids: &[String]| -> BTreeMap<_, _> {
            ids.iter()
                .map(|id| {
                    let mut value = observations[id].clone();
                    value.as_object_mut().unwrap().remove("original");
                    // Whole-file provenance is retained in the plan, while an
                    // explicit JSON pointer binds only its selected value.
                    value.as_object_mut().unwrap().remove("source_file");
                    (id.clone(), value)
                })
                .collect()
        };
        let mut identity = json!({"schema":c.schema,"namespace":c.namespace,"cache":id,
            "owner":cache.owner,"producer":cache.producer,
            "restore":cache.restore,"artifacts":artifacts,
            "artifact_registry":c.artifact_registry,"artifact_bindings":bindings,
            "inputs":values(&cache.compatibility_inputs)});
        if c.schema == SCHEMA {
            identity["consumers"] = json!(cache.consumers.iter().collect::<BTreeSet<_>>());
        }
        let compatibility = wire::digest(&identity)?;
        let source = wire::digest(&values(&cache.source_inputs))?;
        let version = if c.schema == CONSUMER_SCHEMA {
            "v2"
        } else {
            "v1"
        };
        let prefix = format!("chrono-{version}-{compatibility}-");
        entries.insert(
            id,
            json!({"owner":cache.owner,"producer":cache.producer,
            "artifacts":artifacts,"key":format!("{prefix}{source}"),
            "restore_keys":if cache.restore == Restore::Compatible {vec![prefix]} else {vec![]},
            "handling":"registered-build-required","status":"prepared-unrestored"}),
        );
    }
    if c.schema == CONSUMER_SCHEMA {
        for source in &c.consumer_operations[consumer] {
            let input = no_symlink_parents(&root, &source.registry)?;
            if paths.iter().any(|artifact| input.starts_with(artifact)) {
                return Err(format!(
                    "E_CACHE_PATH: consumer registry {} is inside a selected cache artifact",
                    source.registry
                ));
            }
        }
    }
    for id in observations.keys() {
        let mut inputs = Vec::new();
        if let Input::File { path: input, .. } | Input::JsonValue { path: input, .. } =
            &c.inputs[id]
        {
            inputs.push(path(&root, input)?);
        }
        if matches!(c.inputs[id], Input::Command { .. }) {
            for value in [
                &observations[id]["observation"]["executable"],
                &observations[id]["observation"]["file"]["path"],
            ] {
                if let Some(value) = value.as_str() {
                    inputs.push(fs::canonicalize(value).map_err(|e| e.to_string())?);
                }
            }
        }
        if matches!(c.inputs[id], Input::RegisteredFiles { .. }) {
            for name in observations[id]["observation"]["files"]
                .as_object()
                .ok_or("E_CACHE_INPUT: file observations")?
                .keys()
            {
                inputs.push(path(&root, name)?);
            }
        }
        for input in inputs {
            if paths.iter().any(|artifact| input.starts_with(artifact)) {
                return Err(format!(
                    "E_CACHE_PATH: registered input {id} is inside a selected cache artifact"
                ));
            }
        }
    }
    let mut plan = json!({"schema":if c.schema == CONSUMER_SCHEMA {"chrono-cache-plan/v2"} else {"chrono-cache-plan/v1"},
        "consumer":consumer,"caches":entries,"inputs":observations,
        "input_completeness_proven":false,"execution":"not-started",
        "planner":current_executable_identity()?});
    if let Some(contract) = operation_contract {
        plan["consumer_operations"] = contract;
    }
    if let Some(backend) = &c.backend {
        plan["backend"] = json!(backend);
    }
    if c.recover_failed_restores {
        plan["recovery"] = json!({"method":"discard-failed-host-restores","host_root":root,
            "registration_sha256":wire::digest(c)?});
        if c.recover_unconfirmed_restores {
            plan["recovery"]["unconfirmed"] = json!(true);
        }
    }
    Ok(plan)
}

pub fn dispatch(args: &[String]) -> Result<String, String> {
    if args.first().map(String::as_str) == Some("recover") {
        return recovery::dispatch(&args[1..]);
    }
    if args.first().map(String::as_str) == Some("report") {
        return report::dispatch(&args[1..]);
    }
    if args.first().map(String::as_str) != Some("plan") {
        return Err("expected chrono-cache plan".into());
    }
    let mut options = BTreeMap::new();
    for pair in args.get(1..).unwrap_or_default().chunks(2) {
        if pair.len() != 2
            || ![
                "--host-root",
                "--config",
                "--consumer",
                "--github-output",
                "--plan-output",
            ]
            .contains(&pair[0].as_str())
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err(
                "plan requires unique host-root/config/consumer and optional github-output".into(),
            );
        }
    }
    if ["--host-root", "--config", "--consumer"]
        .iter()
        .any(|name| !options.contains_key(name))
    {
        return Err("plan requires host-root/config/consumer".into());
    }
    let root = Path::new(options["--host-root"]);
    let config = options["--config"];
    if !config.starts_with(".chrono-harness/") {
        return Err("E_CACHE_CONFIG: registration must belong to host .chrono-harness".into());
    }
    let c: Config =
        decode(&fs::read(no_symlink_parents(root, config)?).map_err(|e| e.to_string())?)?;
    let mut plan = prepare(root, &c, options["--consumer"])?;
    if let Some(path) = options.get("--plan-output") {
        if ![".chrono-harness/state/", ".chrono-harness/cache/"]
            .iter()
            .any(|p| path.starts_with(p))
        {
            return Err("E_CACHE_OUTPUT: plan must belong to retained host state".into());
        }
        let target = no_symlink_parents(root, path)?;
        fs::create_dir_all(target.parent().ok_or("E_CACHE_OUTPUT: plan parent")?)
            .map_err(|e| e.to_string())?;
        let directory = path
            .rsplit_once('/')
            .ok_or("E_CACHE_OUTPUT: plan parent")?
            .0
            .to_owned()
            + "/";
        for input in plan["inputs"]
            .as_object_mut()
            .ok_or("E_CACHE_OUTPUT: inputs missing")?
            .values_mut()
        {
            if let Some(original) = input["original"].as_str() {
                let raw =
                    fs::read(no_symlink_parents(root, original)?).map_err(|e| e.to_string())?;
                input["original"] =
                    json!(report::retain_for_upload(root, &directory, "probe", &raw)?.path);
            }
        }
        let raw = serde_json::to_vec(&plan).map_err(|e| e.to_string())?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(mut file) => file.write_all(&raw).map_err(|e| e.to_string())?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if fs::read(target).map_err(|e| e.to_string())? != raw {
                    return Err("E_CACHE_OUTPUT: retained plan differs".into());
                }
            }
            Err(e) => return Err(format!("E_CACHE_OUTPUT: {e}")),
        }
    }
    if let Some(output) = options.get("--github-output") {
        let mut text = String::new();
        if c.recover_failed_restores {
            text.push_str("cache_recovery=discard-failed-host-restores\n");
        }
        let selected: BTreeMap<_, _> = plan["caches"]
            .as_object()
            .ok_or("E_CACHE_OUTPUT: caches object")?
            .iter()
            .collect();
        // The provider uses the same lexical ordering of the exact selected set.
        for (index, cache) in selected.values().enumerate() {
            let prefix = format!("cache_{index}");
            let paths = cache["artifacts"]
                .as_object()
                .ok_or("E_CACHE_OUTPUT: artifacts object")?
                .values()
                .map(|a| a["path"].as_str().ok_or("E_CACHE_OUTPUT: artifact path"))
                .collect::<Result<Vec<_>, _>>()?
                .join("\n");
            let restore = cache["restore_keys"]
                .as_array()
                .ok_or("E_CACHE_OUTPUT: restore keys")?
                .iter()
                .map(|v| v.as_str().ok_or("E_CACHE_OUTPUT: restore key"))
                .collect::<Result<Vec<_>, _>>()?
                .join("\n");
            for (suffix, value) in [
                ("key", cache["key"].as_str().ok_or("E_CACHE_OUTPUT: key")?),
                ("paths", paths.as_str()),
                ("restore", restore.as_str()),
            ] {
                let delimiter = format!("chrono_{}", sha256(value.as_bytes()));
                if value.lines().any(|line| line == delimiter) {
                    return Err("E_CACHE_OUTPUT: delimiter collision".into());
                }
                text.push_str(&format!(
                    "{prefix}_{suffix}<<{delimiter}\n{value}\n{delimiter}\n"
                ));
            }
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(output)
            .map_err(|e| format!("E_CACHE_OUTPUT: {e}"))?;
        file.write_all(text.as_bytes())
            .map_err(|e| format!("E_CACHE_OUTPUT: {e}"))?;
    }
    Ok(serde_json::to_string(&plan).map_err(|e| e.to_string())? + "\n")
}
