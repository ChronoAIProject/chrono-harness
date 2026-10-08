//! Candidate-owned Git binding. Does not infer Git's delegated input closure.
use crate::{
    CommandSpec, ProcessResult, facts, no_symlink_parents, resolve_program, run_process_observed,
    sha256, wire,
};
use serde_json::{Value, json as value};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub struct Reader {
    bound: Option<Bound>,
    processes: RefCell<Vec<ProcessResult>>,
    // Exact immutable endpoint/path -> range of an original successful process. The root,
    // tool and environment belong to this Reader's binding, never a global cache.
    blobs: RefCell<BTreeMap<(String, String), BlobReceipt>>,
    trees: RefCell<BTreeMap<String, usize>>,
    verified_oids: RefCell<BTreeSet<String>>,
}
#[derive(Clone)]
struct BlobReceipt {
    process: usize,
    range: std::ops::Range<usize>,
    // Batch ranges also belong to the original metadata receipt and identity.
    object: Option<(usize, BlobObject)>,
}
#[derive(Clone)]
struct BlobObject {
    oid: String,
    length: usize,
}
impl BlobObject {
    fn header(&self) -> String {
        format!("{} blob {}\n", self.oid, self.length)
    }
    fn matches(&self, bytes: &[u8]) -> bool {
        use sha2::Digest;
        if bytes.len() != self.length {
            return false;
        }
        let header = format!("blob {}\0", self.length);
        let oid = if self.oid.len() == 40 {
            let mut hash = sha1::Sha1::new();
            hash.update(header.as_bytes());
            hash.update(bytes);
            format!("{:x}", hash.finalize())
        } else {
            let mut hash = sha2::Sha256::new();
            hash.update(header.as_bytes());
            hash.update(bytes);
            format!("{:x}", hash.finalize())
        };
        oid == self.oid
    }
}
/// One successful registered blob and its original acquisition coordinates.
/// Callers retain the processes; no synthetic process receipt is produced here.
pub struct RegistryBlob {
    pub path: String,
    pub bytes: Vec<u8>,
    receipt: BlobReceipt,
}
/// Acquire only explicit immutable registry paths through the caller's bounded
/// Git transport. Metadata partitions requests under the same output limit.
/// The callback returns its retained process index and exact stdout bytes.
pub fn acquire_registry_blobs(
    oid: &str,
    paths: &[String],
    limit: usize,
    mut git: impl FnMut(&[&str], &[u8]) -> Result<(usize, Vec<u8>), String>,
    error: impl Fn(&str) -> String,
) -> Result<Vec<RegistryBlob>, String> {
    facts::full_oid(oid)?;
    let mut unique = BTreeSet::new();
    for path in paths {
        crate::relative_path(path)?;
        if !unique.insert(path) {
            return Err(error("duplicate registered blob path"));
        }
    }
    let pending = paths;
    let mut bytes = BTreeMap::new();
    let mut receipts = vec![];
    // The longest supported metadata header is 64 hex bytes, a blob
    // type, a 20-digit length and LF. Tiny declared limits remain usable.
    if pending.len() < 3 || pending.len().saturating_mul(91) > limit {
        for path in pending {
            let (process, original) = git(&["show", &format!("{oid}:{path}")], &[])?;
            receipts.push((
                path.clone(),
                BlobReceipt {
                    process,
                    range: 0..original.len(),
                    object: None,
                },
            ));
            bytes.insert(path.clone(), original);
        }
    } else {
        let input = pending
            .iter()
            .map(|path| format!("{oid}:{path}\n"))
            .collect::<String>();
        let (metadata_process, metadata) = git(&["cat-file", "--batch-check"], input.as_bytes())?;
        let parse = || -> Result<Vec<BlobObject>, String> {
            let text = std::str::from_utf8(&metadata).map_err(|_| "non UTF-8 batch metadata")?;
            let rows = text
                .strip_suffix('\n')
                .ok_or("missing batch metadata terminator")?
                .split('\n')
                .collect::<Vec<_>>();
            if rows.len() != pending.len() {
                return Err("batch metadata count mismatch".into());
            }
            rows.iter()
                .map(|row| {
                    let fields = row.split(' ').collect::<Vec<_>>();
                    if fields.len() != 3 || fields[1] != "blob" || fields[0].len() != oid.len() {
                        return Err("expected exact blob metadata".into());
                    }
                    facts::full_oid(fields[0])?;
                    let length = fields[2]
                        .parse::<usize>()
                        .map_err(|_| "invalid blob length")?;
                    if length.to_string() != fields[2] {
                        return Err("noncanonical blob length".into());
                    }
                    Ok(BlobObject {
                        oid: fields[0].into(),
                        length,
                    })
                })
                .collect()
        };
        let objects = parse().map_err(|e| error(&e))?;
        let mut first = 0;
        while first < pending.len() {
            let mut end = first;
            let mut length = 0usize;
            while end < pending.len() {
                let object = &objects[end];
                let framed = object
                    .length
                    .checked_add(object.header().len())
                    .and_then(|n| n.checked_add(1));
                let Some(total) = framed
                    .and_then(|n| length.checked_add(n))
                    .filter(|n| *n <= limit)
                else {
                    break;
                };
                length = total;
                end += 1;
            }
            if end.saturating_sub(first) < 2 {
                let path = &pending[first];
                let (process, original) = git(&["show", &format!("{oid}:{path}")], &[])?;
                if !objects[first].matches(&original) {
                    return Err(error("blob bytes differ from batch metadata"));
                }
                receipts.push((
                    path.clone(),
                    BlobReceipt {
                        process,
                        range: 0..original.len(),
                        object: Some((metadata_process, objects[first].clone())),
                    },
                ));
                bytes.insert(path.clone(), original);
                first += 1;
                continue;
            }
            let input = objects[first..end]
                .iter()
                .map(|object| format!("{}\n", object.oid))
                .collect::<String>();
            let (process, original) = git(&["cat-file", "--batch"], input.as_bytes())?;
            let mut offset = 0;
            for i in first..end {
                let object = &objects[i];
                let header = object.header();
                if !original[offset..].starts_with(header.as_bytes()) {
                    return Err(error("batch blob header mismatch"));
                }
                offset += header.len();
                let stop = offset
                    .checked_add(object.length)
                    .ok_or_else(|| error("batch blob length overflow"))?;
                let content = original
                    .get(offset..stop)
                    .ok_or_else(|| error("truncated batch blob"))?;
                if original.get(stop) != Some(&b'\n') || !object.matches(content) {
                    return Err(error("batch blob bytes/terminator mismatch"));
                }
                receipts.push((
                    pending[i].clone(),
                    BlobReceipt {
                        process,
                        range: offset..stop,
                        object: Some((metadata_process, object.clone())),
                    },
                ));
                bytes.insert(pending[i].clone(), content.to_vec());
                offset = stop + 1;
            }
            if offset != original.len() {
                return Err(error("trailing batch blob output"));
            }
            first = end;
        }
    }
    Ok(receipts
        .into_iter()
        .map(|(path, receipt)| RegistryBlob {
            bytes: bytes.remove(&path).expect("acquired explicit registry"),
            path,
            receipt,
        })
        .collect())
}

/// Read one explicitly declared immutable input. Metadata remains diagnostic
/// traffic; only the object's exact stdout receives the declared data bound.
/// Both original processes are retained by the caller, including failures.
pub fn acquire_immutable_input(
    oid: &str,
    path: &str,
    limit: usize,
    mut git: impl FnMut(&[&str], &[u8], Option<usize>) -> Result<Vec<u8>, String>,
) -> Result<Vec<u8>, String> {
    facts::full_oid(oid)?;
    crate::relative_path(path)?;
    if limit == 0 || limit > 64 * 1024 * 1024 {
        return Err("invalid immutable input limit".into());
    }
    // NUL-framed queries preserve literal registered paths, including newlines.
    let query = format!("{oid}:{path}\0");
    let metadata = git(&["cat-file", "--batch-check", "-z"], query.as_bytes(), None)?;
    let text = std::str::from_utf8(&metadata).map_err(|_| "invalid immutable input metadata")?;
    let fields: Vec<_> = text
        .strip_suffix('\n')
        .ok_or("missing immutable input metadata terminator")?
        .split(' ')
        .collect();
    if fields.len() != 3 || fields[1] != "blob" || fields[0].len() != oid.len() {
        return Err("expected exact immutable blob metadata".into());
    }
    facts::full_oid(fields[0])?;
    let length = fields[2]
        .parse::<usize>()
        .map_err(|_| "invalid immutable input length")?;
    if length.to_string() != fields[2] {
        return Err("noncanonical immutable input length".into());
    }
    if length > limit {
        return Err(format!(
            "immutable input {path} has {length} bytes; declared limit is {limit}"
        ));
    }
    let object = BlobObject {
        oid: fields[0].into(),
        length,
    };
    // Bind the data read to the observed object identity, never a moving ref.
    // Empty blobs still need one byte of transport capacity to detect excess.
    let bytes = git(&["cat-file", "blob", &object.oid], &[], Some(length.max(1)))?;
    if !object.matches(&bytes) {
        return Err("immutable input bytes differ from object metadata".into());
    }
    Ok(bytes)
}

/// Original binding observations when construction fails, without decoding diagnostics.
#[derive(Debug)]
pub struct OpenFailure {
    pub message: String,
    pub observation: Value,
}
struct Bound {
    root: PathBuf,
    config_path: String,
    config_bytes: Vec<u8>,
    binding: Value,
    spec: CommandSpec,
    environment: Value,
    guard: Option<crate::facts_inputs::Guard>,
    selection: Option<crate::facts_configs::Selection>,
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("missing {key}"))
}
/// Validate the new link without interpreting any historical config as v3.
pub fn declaration(config: &Value) -> Result<(&Value, &Value), String> {
    if !matches!(config["schema_version"].as_u64(), Some(3 | 4)) {
        return Err("facts_git requires config schema_version 3 or 4".into());
    }
    let link = config["facts_git"]
        .as_object()
        .ok_or("missing facts_git object")?;
    if !link.contains_key("tool")
        || !link.contains_key("input")
        || link
            .keys()
            .any(|key| !matches!(key.as_str(), "tool" | "input" | "guard"))
    {
        return Err("facts_git requires tool and input, with optional guard".into());
    }
    let unique = |collection: &Value, id: &str| -> Result<usize, String> {
        let items = collection
            .as_array()
            .ok_or("binding collection must be array")?;
        let hits: Vec<_> = items
            .iter()
            .enumerate()
            .filter(|(_, v)| v["id"] == id)
            .collect();
        if hits.len() != 1 {
            return Err(format!("missing/ambiguous facts_git reference {id}"));
        }
        Ok(hits[0].0)
    };
    let tool = &config["tools"][unique(&config["tools"], text(&config["facts_git"], "tool")?)?];
    let input = &config["environment"]["inputs"][unique(
        &config["environment"]["inputs"],
        text(&config["facts_git"], "input")?,
    )?];
    text(tool, "program")?;
    text(tool, "expected_version")?;
    text(input, "location")?;
    if tool["resolution"] != "PATH-once"
        || input["presence"] != "present"
        || !input["sha256"].as_str().is_some_and(wire::is_digest)
    {
        return Err("facts_git requires a present digest-bound input and PATH-once tool".into());
    }
    let args: Vec<String> =
        serde_json::from_value(tool["version_argv"].clone()).map_err(|e| e.to_string())?;
    if args.is_empty() {
        return Err("facts_git requires version_argv".into());
    }
    crate::facts_inputs::declaration(config)?;
    Ok((tool, input))
}

impl Reader {
    pub fn legacy() -> Self {
        Self {
            bound: None,
            processes: RefCell::new(vec![]),
            blobs: RefCell::new(BTreeMap::new()),
            trees: RefCell::new(BTreeMap::new()),
            verified_oids: RefCell::new(BTreeSet::new()),
        }
    }
    pub fn for_config(root: &Path, config: &str) -> Result<Self, String> {
        Self::open(root, config, None, None, None)
    }
    pub fn for_config_observed(root: &Path, config: &str) -> Result<Self, OpenFailure> {
        let mut observation = Value::Null;
        Self::open(root, config, None, None, Some(&mut observation)).map_err(|message| {
            OpenFailure {
                message,
                observation,
            }
        })
    }
    pub fn for_request(req: &wire::Request) -> Result<Self, String> {
        req.validate()?;
        Self::from_observations(
            &req.candidate.root,
            &req.config_path,
            &req.candidate.commit,
            &req.observations,
        )
    }
    pub fn from_observations(
        root: &Path,
        config: &str,
        candidate: &str,
        observations: &Value,
    ) -> Result<Self, String> {
        let reader = Self::open(
            root,
            config,
            Some(&observations["environment"]),
            Some(&observations["git_facts"]),
            None,
        )?;
        reader.verify_config(root, candidate)?;
        Ok(reader)
    }
    fn open(
        root: &Path,
        config: &str,
        observed_env: Option<&Value>,
        prior: Option<&Value>,
        opening_observation: Option<&mut Value>,
    ) -> Result<Self, String> {
        let result = (|| {
            let (config_path, bytes, cfg, selection) = crate::facts_configs::load(root, config)?;
            if !matches!(cfg["schema_version"].as_u64(), Some(3 | 4)) {
                if cfg.get("facts_git").is_some() || prior.is_some_and(|v| !v.is_null()) {
                    return Err("legacy configuration cannot contain a Git facts binding".into());
                }
                return Ok(Self::legacy());
            }
            let (tool, input) = declaration(&cfg)?;
            let credentials = if cfg["schema_version"] == 4 {
                crate::prepared::credential_environment(&cfg)?
            } else {
                Default::default()
            };
            let mut inherited = BTreeMap::<String, Option<String>>::new();
            let mut effective = BTreeMap::<String, String>::new();
            for key in cfg["environment"]["inherit"]
                .as_array()
                .ok_or("environment.inherit")?
            {
                let key = key.as_str().ok_or("environment key")?;
                let val = if credentials.contains(key) {
                    None
                } else if let Some(env) = observed_env {
                    serde_json::from_value(
                        env["inherited"]
                            .get(key)
                            .ok_or("missing inherited observation")?
                            .clone(),
                    )
                    .map_err(|e| e.to_string())?
                } else {
                    match std::env::var(key) {
                        Ok(v) => Some(v),
                        Err(std::env::VarError::NotPresent) => None,
                        Err(_) => return Err(format!("non UTF-8 inherited variable {key}")),
                    }
                };
                if inherited.insert(key.into(), val.clone()).is_some() {
                    return Err("duplicate inherited variable".into());
                }
                if let Some(v) = val {
                    effective.insert(key.into(), v);
                }
            }
            for (k, v) in cfg["environment"]["values"]
                .as_object()
                .ok_or("environment.values")?
            {
                effective.insert(k.clone(), v.as_str().ok_or("environment value")?.into());
            }
            let mut environment = value!({"inherited":inherited,"effective":effective});
            if cfg["schema_version"] == 4 {
                environment["omitted_credentials"] = value!(credentials);
            }
            if observed_env.is_some_and(|v| *v != environment) {
                return Err("Git facts environment observation mismatch".into());
            }
            // These variables redirect the selected checkout. Reject rather than silently discard declarations.
            for key in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
                if effective.contains_key(key) {
                    return Err(format!("Git facts environment redirects checkout: {key}"));
                }
            }
            let program = text(tool, "program")?;
            if !program.contains('/') && !effective.contains_key("PATH") {
                return Err("facts_git requires declared PATH for tool resolution".into());
            }
            let path = resolve_program(
                root,
                program,
                Some(effective.get("PATH").map(String::as_str).unwrap_or("")),
            )?;
            let location = Path::new(text(input, "location")?);
            let location = if location.is_absolute() {
                location.to_path_buf()
            } else {
                no_symlink_parents(root, location.to_str().ok_or("input UTF-8")?)?
            };
            if fs::canonicalize(&location).map_err(|e| e.to_string())?
                != fs::canonicalize(&path).map_err(|e| e.to_string())?
            {
                return Err(
                    "facts_git input location does not identify selected executable".into(),
                );
            }
            let hash = sha256(&fs::read(&path).map_err(|e| e.to_string())?);
            if input["sha256"] != hash {
                return Err("facts_git declared digest mismatch".into());
            }
            let timeout = cfg["protocol"]["timeout_seconds"]
                .as_u64()
                .filter(|n| *n > 0)
                .ok_or("invalid facts timeout")?;
            let limit = cfg["protocol"]["stdout_limit_bytes"]
                .as_u64()
                .filter(|n| *n > 0 && *n <= 64 * 1024 * 1024)
                .ok_or("invalid facts output bound")? as usize;
            let binding = value!({"tool":tool["id"],"input":input["id"],"location":location,
                "program":program,"path":path,"sha256":hash,"expected_version":tool["expected_version"],"version_argv":tool["version_argv"]});
            let spec = CommandSpec {
                program: path.to_str().ok_or("tool UTF-8")?.into(),
                args: vec![],
                env: effective,
                timeout_seconds: timeout,
                output_limit_bytes: limit,
            };
            let reader = Self {
                bound: Some(Bound {
                    root: fs::canonicalize(root).map_err(|e| e.to_string())?,
                    config_path,
                    config_bytes: bytes,
                    binding,
                    spec,
                    environment,
                    guard: crate::facts_inputs::Guard::open(
                        &fs::canonicalize(root).map_err(|e| e.to_string())?,
                        &cfg,
                    )?,
                    selection,
                }),
                processes: RefCell::new(vec![]),
                blobs: RefCell::new(BTreeMap::new()),
                trees: RefCell::new(BTreeMap::new()),
                verified_oids: RefCell::new(BTreeSet::new()),
            };
            if let Some(prior) = prior {
                let current = reader.observation();
                for key in [
                    "schema",
                    "config_path",
                    "config_sha256",
                    "binding",
                    "environment",
                    "inputs",
                    "selection",
                ] {
                    if prior.get(key) != current.get(key) {
                        return Err(
                            reader.error(&format!("missing/mismatched request Git facts {key}"))
                        );
                    }
                }
            }
            let argv: Vec<String> =
                serde_json::from_value(tool["version_argv"].clone()).map_err(|e| e.to_string())?;
            let result = reader.invoke(root, argv, &[], false);
            if let Some(observation) = opening_observation {
                *observation = reader.observation();
            }
            let bytes = result?;
            if std::str::from_utf8(&bytes)
                .map_err(|_| reader.error("non UTF-8 version"))?
                .trim_end()
                != text(tool, "expected_version")?
            {
                return Err(reader.error("Git facts version mismatch"));
            }
            Ok(reader)
        })();
        result.map_err(|e: String| {
            if e.starts_with("E_GIT_FACTS:") {
                e
            } else {
                format!("E_GIT_FACTS: {e}")
            }
        })
    }
    pub fn observation(&self) -> Value {
        match &self.bound {
            None => Value::Null,
            Some(b) => {
                let processes: Vec<_> = self
                    .processes
                    .borrow()
                    .iter()
                    .map(|process| {
                        crate::full::compact_process(&value!(process))
                            .expect("process engine preserves original stream identities")
                    })
                    .collect();
                let mut observed = value!({"schema":"chrono-git-facts/v1","config_path":b.config_path,
                "config_sha256":sha256(&b.config_bytes),"binding":b.binding,"environment":b.environment,
                "processes":processes,"input_closure_complete":false});
                if let Some(guard) = &b.guard {
                    observed["inputs"] = guard.observation.clone();
                }
                if let Some(selection) = &b.selection {
                    observed["selection"] = selection.observation.clone();
                }
                observed
            }
        }
    }
    pub fn is_bound(&self) -> bool {
        self.bound.is_some()
    }
    pub fn record(&self, response: &mut wire::Response) {
        if self.bound.is_some() {
            response
                .outputs
                .insert("git_facts".into(), self.observation());
        }
    }
    fn error(&self, message: &str) -> String {
        format!(
            "E_GIT_FACTS: {}",
            value!({"message":message,"observation":self.observation()})
        )
    }
    fn unchanged(&self) -> Result<(), String> {
        let b = self.bound.as_ref().ok_or("missing binding")?;
        if let Some(selection) = &b.selection {
            selection.unchanged(&b.root)?;
        }
        if fs::read(no_symlink_parents(&b.root, &b.config_path)?).map_err(|e| e.to_string())?
            != b.config_bytes
        {
            return Err("Git facts configuration changed".into());
        }
        let path = resolve_program(
            &b.root,
            text(&b.binding, "program")?,
            Some(b.spec.env.get("PATH").map(String::as_str).unwrap_or("")),
        )?;
        if path != Path::new(&b.spec.program)
            || sha256(&fs::read(&path).map_err(|e| e.to_string())?) != b.binding["sha256"]
            || fs::canonicalize(text(&b.binding, "location")?).map_err(|e| e.to_string())?
                != fs::canonicalize(&path).map_err(|e| e.to_string())?
        {
            return Err("Git facts executable/input changed".into());
        }
        if let Some(guard) = &b.guard {
            guard.unchanged(&b.root)?;
        }
        Ok(())
    }
    fn check_root(&self, root: &Path) -> Result<(), String> {
        let b = self.bound.as_ref().ok_or("missing binding")?;
        if fs::canonicalize(root).map_err(|e| self.error(&e.to_string()))? != b.root {
            return Err(self.error("Git facts root mismatch"));
        }
        Ok(())
    }
    pub(crate) fn blob_with_reuse(
        &self,
        root: &Path,
        oid: &str,
        path: &str,
    ) -> Result<Vec<u8>, String> {
        // Legacy environments are not bound. Refs and expressions must always
        // be observed anew, retaining the existing Git success/error semantics.
        if !self.can_reuse_oid(oid) {
            return self.git(root, &["show", &format!("{oid}:{path}")]);
        }
        self.check_root(root)?;
        let key = (oid.to_string(), path.to_string());
        let prior = self.blobs.borrow().get(&key).cloned();
        if let Some(receipt) = prior {
            self.unchanged().map_err(|e| self.error(&e))?;
            let processes = self.processes.borrow();
            let bytes = processes[receipt.process].stdout_bytes[receipt.range].to_vec();
            if let Some((metadata_process, object)) = &receipt.object {
                // Both originals remain owned by this Reader, including the
                // metadata identity that framed this exact content range.
                debug_assert!(*metadata_process < receipt.process);
                debug_assert!(object.matches(&bytes));
            }
            self.check_root(root)?;
            self.unchanged().map_err(|e| self.error(&e))?;
            return Ok(bytes);
        }
        let bytes = self.git(root, &["show", &format!("{oid}:{path}")])?;
        // invoke retains the real receipt and completes both guards before
        // returning success. Never index failures, absence or drifted reads.
        let index = self.processes.borrow().len() - 1;
        self.blobs.borrow_mut().insert(
            key,
            BlobReceipt {
                process: index,
                range: 0..bytes.len(),
                object: None,
            },
        );
        Ok(bytes)
    }
    pub(crate) fn tree_with_reuse(&self, root: &Path, oid: &str) -> Result<facts::Tree, String> {
        let eligible = self.can_reuse_oid(oid);
        if eligible {
            self.check_root(root)?;
            let prior = self.trees.borrow().get(oid).copied();
            if let Some(process) = prior {
                self.unchanged().map_err(|e| self.error(&e))?;
                let tree = facts::parse_tree(&self.processes.borrow()[process].stdout_bytes)?;
                self.check_root(root)?;
                self.unchanged().map_err(|e| self.error(&e))?;
                return Ok(tree);
            }
        }
        let raw = self.git(root, &["ls-tree", "-rz", "--full-tree", oid])?;
        let tree = facts::parse_tree(&raw)?;
        if eligible {
            // Keep only the original successful process address. Mutable index
            // and physical checkout observations never enter this cache.
            self.trees
                .borrow_mut()
                .insert(oid.into(), self.processes.borrow().len() - 1);
        }
        Ok(tree)
    }
    pub(crate) fn can_reuse_oid(&self, oid: &str) -> bool {
        self.bound.is_some() && self.verified_oids.borrow().contains(oid)
    }
    /// Resolved registry or snapshot paths in an already verified immutable
    /// commit retain framing, guards and original process receipts.
    pub(crate) fn immutable_blobs(
        &self,
        root: &Path,
        oid: &str,
        paths: &[String],
    ) -> Result<Vec<(String, Vec<u8>)>, String> {
        let mut bytes = BTreeMap::new();
        let mut pending = vec![];
        for path in paths {
            crate::relative_path(path)?;
            if self
                .blobs
                .borrow()
                .contains_key(&(oid.into(), path.clone()))
            {
                bytes.insert(path.clone(), self.blob_with_reuse(root, oid, path)?);
            } else {
                pending.push(path.clone());
            }
        }
        let limit = self
            .bound
            .as_ref()
            .ok_or("missing binding")?
            .spec
            .output_limit_bytes;
        let acquired = acquire_registry_blobs(
            oid,
            &pending,
            limit,
            |args, input| {
                let bytes = self.git_input(root, args, input, false)?;
                Ok((self.processes.borrow().len() - 1, bytes))
            },
            |message| self.error(message),
        )?;
        for blob in acquired {
            self.blobs
                .borrow_mut()
                .insert((oid.into(), blob.path.clone()), blob.receipt);
            bytes.insert(blob.path, blob.bytes);
        }
        Ok(paths
            .iter()
            .map(|path| {
                (
                    path.clone(),
                    bytes.remove(path).expect("acquired explicit registry"),
                )
            })
            .collect())
    }
    pub(crate) fn record_verified_oid(&self, oid: &str) {
        // Only verify_oid's existing successful commit/tree observations may
        // enable reuse. Never acquire another identity just to populate this set.
        if self.bound.is_some() {
            self.verified_oids.borrow_mut().insert(oid.into());
        }
    }
    fn invoke(
        &self,
        root: &Path,
        args: Vec<String>,
        input: &[u8],
        literal_inventory: bool,
    ) -> Result<Vec<u8>, String> {
        let b = self.bound.as_ref().ok_or("missing binding")?;
        self.check_root(root)?;
        self.unchanged().map_err(|e| self.error(&e))?;
        let mut spec = CommandSpec {
            args,
            ..b.spec.clone()
        };
        if literal_inventory {
            for key in facts::LITERAL_PATHSPEC_CONTROLS {
                spec.env.insert(key.into(), "0".into());
            }
        }
        let result = run_process_observed(root, &spec, input, text(&b.binding, "sha256")?)
            .map_err(|e| self.error(&e))?;
        let failure = result.failure.clone().or_else(|| {
            (result.exit_code != 0).then(|| format!("Git facts process exit {}", result.exit_code))
        });
        let bytes = result.stdout_bytes.clone();
        self.processes.borrow_mut().push(result);
        self.check_root(root)?;
        self.unchanged().map_err(|e| self.error(&e))?;
        if let Some(failure) = failure {
            return Err(self.error(&failure));
        }
        Ok(bytes)
    }
    pub fn verify_config(&self, root: &Path, candidate: &str) -> Result<(), String> {
        if let Some(b) = &self.bound {
            facts::full_oid(candidate)?;
            if let Some(selection) = &b.selection {
                if self.blob(root, candidate, &selection.path)? != selection.bytes {
                    return Err(self.error("Git config selector differs from fixed candidate"));
                }
            }
            if self.blob(root, candidate, &b.config_path)? != b.config_bytes {
                return Err(self.error("Git facts config differs from fixed candidate"));
            }
        }
        Ok(())
    }
    pub fn git(&self, root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
        if self.bound.is_none() {
            return facts::git(root, args);
        }
        self.git_input(root, args, &[], false)
    }
    pub(crate) fn literal_inventory(&self, root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
        if self.bound.is_none() {
            return facts::git_inventory(root, args, true);
        }
        self.git_input(root, args, &[], true)
    }
    /// Only an exact successful batch response proves absence. Process errors,
    /// wrong types and malformed responses are never converted to missing history.
    pub fn commit_available(&self, root: &Path, oid: &str) -> Result<bool, String> {
        facts::full_oid(oid)?;
        let bytes = self.git_input(
            root,
            &["cat-file", "--batch-check=%(objectname) %(objecttype)"],
            format!("{oid}\n").as_bytes(),
            false,
        )?;
        if bytes == format!("{oid} commit\n").as_bytes() {
            Ok(true)
        } else if bytes == format!("{oid} missing\n").as_bytes() {
            Ok(false)
        } else {
            Err(self.error("expected exact commit or missing object response"))
        }
    }
    fn git_input(
        &self,
        root: &Path,
        args: &[&str],
        input: &[u8],
        literal_inventory: bool,
    ) -> Result<Vec<u8>, String> {
        let bound = self.bound.as_ref().ok_or("missing binding")?;
        let argv = [
            "--no-optional-locks",
            "--no-replace-objects",
            "-C",
            bound.root.to_str().ok_or("root UTF-8")?,
        ]
        .into_iter()
        .chain(args.iter().copied())
        .map(str::to_string)
        .collect();
        self.invoke(root, argv, input, literal_inventory)
    }
}
