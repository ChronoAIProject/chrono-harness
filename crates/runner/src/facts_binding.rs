//! Candidate-owned Git binding. Does not infer Git's delegated input closure.
use crate::{
    CommandSpec, ProcessResult, facts, no_symlink_parents, resolve_program, run_process_observed,
    sha256, wire,
};
use serde_json::{Value, json as value};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub struct Reader {
    bound: Option<Bound>,
    processes: RefCell<Vec<ProcessResult>>,
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
    if config["schema_version"] != 3 {
        return Err("facts_git requires config schema_version 3".into());
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
            if cfg["schema_version"] != 3 {
                if cfg.get("facts_git").is_some() || prior.is_some_and(|v| !v.is_null()) {
                    return Err("legacy configuration cannot contain a Git facts binding".into());
                }
                return Ok(Self::legacy());
            }
            let (tool, input) = declaration(&cfg)?;
            let mut inherited = BTreeMap::<String, Option<String>>::new();
            let mut effective = BTreeMap::<String, String>::new();
            for key in cfg["environment"]["inherit"]
                .as_array()
                .ok_or("environment.inherit")?
            {
                let key = key.as_str().ok_or("environment key")?;
                let val = if let Some(env) = observed_env {
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
            let environment = value!({"inherited":inherited,"effective":effective});
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
            let result = reader.invoke(root, argv, &[]);
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
                let mut observed = value!({"schema":"chrono-git-facts/v1","config_path":b.config_path,
                "config_sha256":sha256(&b.config_bytes),"binding":b.binding,"environment":b.environment,
                "processes":*self.processes.borrow(),"input_closure_complete":false});
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
    fn invoke(&self, root: &Path, args: Vec<String>, input: &[u8]) -> Result<Vec<u8>, String> {
        let b = self.bound.as_ref().ok_or("missing binding")?;
        if fs::canonicalize(root).map_err(|e| self.error(&e.to_string()))? != b.root {
            return Err(self.error("Git facts root mismatch"));
        }
        self.unchanged().map_err(|e| self.error(&e))?;
        let spec = CommandSpec {
            args,
            ..b.spec.clone()
        };
        let result = run_process_observed(root, &spec, input, text(&b.binding, "sha256")?)
            .map_err(|e| self.error(&e))?;
        let failure = result.failure.clone().or_else(|| {
            (result.exit_code != 0).then(|| format!("Git facts process exit {}", result.exit_code))
        });
        let bytes = result.stdout_bytes.clone();
        self.processes.borrow_mut().push(result);
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
        self.git_input(root, args, &[])
    }
    /// Only an exact successful batch response proves absence. Process errors,
    /// wrong types and malformed responses are never converted to missing history.
    pub fn commit_available(&self, root: &Path, oid: &str) -> Result<bool, String> {
        facts::full_oid(oid)?;
        let bytes = self.git_input(
            root,
            &["cat-file", "--batch-check=%(objectname) %(objecttype)"],
            format!("{oid}\n").as_bytes(),
        )?;
        if bytes == format!("{oid} commit\n").as_bytes() {
            Ok(true)
        } else if bytes == format!("{oid} missing\n").as_bytes() {
            Ok(false)
        } else {
            Err(self.error("expected exact commit or missing object response"))
        }
    }
    fn git_input(&self, root: &Path, args: &[&str], input: &[u8]) -> Result<Vec<u8>, String> {
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
        self.invoke(root, argv, input)
    }
}
