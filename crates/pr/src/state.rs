//! PR-owned immutable operation originals, protected by the existing output owner.
use chrono_harness::ownership::Lease;
use chrono_harness::retained_artifacts::retention::{
    Inventory, Publication, read_guard, unique_path,
};
use chrono_harness::{
    CommandSpec, ProcessResult, file_identity, no_symlink_parents, sha256, validate_launch_evidence,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(crate) struct State {
    root: PathBuf,
    producer: String,
    pub(crate) path: String,
    publication: Publication,
    calls: usize,
    unreturned: bool,
    inputs: Vec<(String, Lease)>,
    continuation: Option<Continuation>,
}

struct Continuation {
    path: String,
    result_original: Value,
    _reader: Lease,
}

impl Continuation {
    fn load(
        root: &Path,
        inventory: &Inventory,
        config: &super::Config,
        config_bytes: &[u8],
        args: &super::Args,
    ) -> Result<Option<Self>, String> {
        let Some(path) = args.values.get("--continue-operation") else {
            return Ok(None);
        };
        chrono_harness::relative_path(path)?;
        if !path.starts_with(&format!("{}/operation-", config.state_directory)) {
            return Err("continued operation is outside the configured PR state scope".into());
        }
        let reader =
            read_guard(root, path)?.ok_or("continued operation is unavailable or not enrolled")?;
        let read = |name: &str, limit: usize| -> Result<(Value, Value), String> {
            let member = format!("{path}/{name}");
            let bytes = super::read_bounded(&no_symlink_parents(root, &member)?, limit)?;
            let digest = sha256(&bytes);
            if !inventory.published_tree_member(
                &config.state_producer,
                path,
                &member,
                &digest,
                bytes.len() as u64,
            )? {
                return Err("continued operation has no matching sealed producer result; preserve originals".into());
            }
            Ok((
                chrono_harness::json(&bytes)?,
                json!({"path":member,"sha256":digest,"length":bytes.len()}),
            ))
        };
        let (intent, _) = read("intent.json", 1048576)?;
        let (old_config, _) = read("config.json", 1048576)?;
        if old_config != chrono_harness::json(config_bytes)? {
            return Err("continued operation config differs from the current binding".into());
        }
        let (result, result_original) = read("result.json", 16777216)?;
        if intent["schema"] != "chrono-pr-operation/v1"
            || intent["producer"] != "chrono-pr"
            || intent["operation"] != *path
            || intent["object"]["repository"] != config.repository
            || intent["object"]["base"] != config.base
            || intent["object"]["expected_head"] != args.get("--head-sha")?
            || result["schema"] != "chrono-pr-result/v2"
            || result["operation"] != *path
            || result["operation_state"] != "completed"
            || !result["command_exit_code"].is_u64()
            || result["repository"] != config.repository
            || result["base"] != config.base
            || result["pr"] != args.number()?
            || result["expected_head"] != args.get("--head-sha")?
            || result.get("storage_error").is_some()
        {
            return Err("continued operation identity or local completion is not established; preserve originals".into());
        }
        Ok(Some(Self {
            path: path.clone(),
            result_original,
            _reader: reader,
        }))
    }
}

pub(crate) fn now() -> Result<u128, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis())
}

impl State {
    pub(crate) fn begin(
        root: &Path,
        config: &super::Config,
        config_bytes: &[u8],
        args: &super::Args,
    ) -> Result<Self, String> {
        let inventory = Inventory::adopted(root)?
            .ok_or("PR operations require the registered output retention owner")?;
        let directory = no_symlink_parents(root, &config.state_directory)?;
        if !inventory.accepts(
            &config.state_producer,
            &format!("{}/operation-", config.state_directory),
        ) {
            return Err("PR state producer/directory is not registered in retention policy".into());
        }
        let continuation = Continuation::load(root, &inventory, config, config_bytes, args)?;
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let absolute = unique_path(&directory, "operation-", "")?;
        let path = absolute
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("PR state path UTF-8")?
            .to_owned();
        let publication = inventory.plan(
            &config.state_producer,
            &path,
            true,
            json!({"kind":"pr-operation","operation":path,"state":"intent","result":null}),
        )?;
        publication.materialize_for_reference(&Self::recovery_role(&path))?;
        let state = Self {
            root: root.into(),
            producer: config.state_producer.clone(),
            path,
            publication,
            calls: 0,
            unreturned: false,
            inputs: Vec::new(),
            continuation,
        };
        let saved = (|| {
            let config_original = state.original("config.json", config_bytes)?;
            state.write("intent.json", &json!({
                "schema":"chrono-pr-operation/v1", "operation":state.path, "producer":"chrono-pr",
                "command":args.command, "arguments":args.values, "draft":args.draft,
                "config_original":config_original,
                "continuation":state.continuation.as_ref().map(|c| json!({"operation":c.path,"result_original":c.result_original,"method":"one read-only current PR observation; release only this prior root after sealed handoff"})),
                "object":{"repository":config.repository,"base":config.base,"pr":args.number().ok(),"expected_head":args.get("--head-sha")?},
                "state":"intent", "updated_at_unix_millis":now()?, "handle":null, "result":null,
                "recovery":{"method":"inspect exact operation originals before any retry; missing result remains unknown", "consumer":Self::recovery_role(&state.path), "release":"only after matching terminal object evidence and recovery consumers end"}
            }))
        })();
        saved.map_err(|e| format!("PR operation {} intent publication failed: {e}", state.path))?;
        Ok(state)
    }

    pub(crate) fn continued_path(&self) -> Option<&str> {
        self.continuation.as_ref().map(|c| c.path.as_str())
    }

    fn recovery_role(path: &str) -> String {
        format!("pr-recovery:{}", sha256(path.as_bytes()))
    }

    fn write(&self, name: &str, value: &Value) -> Result<(), String> {
        self.write_bytes(name, &serde_json::to_vec(value).map_err(|e| e.to_string())?)
    }

    fn write_bytes(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let directory = self.root.join(&self.path);
        self.publication.write_file(&directory.join(name), bytes)?;
        fs::File::open(directory)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| e.to_string())
    }

    fn original(&self, name: &str, bytes: &[u8]) -> Result<Value, String> {
        let path = format!("{}/{name}", self.path);
        self.write_bytes(name, bytes)?;
        Ok(json!({"path":path,"sha256":sha256(bytes),"length":bytes.len()}))
    }

    pub(crate) fn process_paths(&self, n: usize) -> [PathBuf; 3] {
        ["stdout", "stderr", "launch.json"]
            .map(|suffix| self.root.join(format!("{}.call-{n}.{suffix}", self.path)))
    }

    pub(crate) fn operation_state(&self) -> &'static str {
        if self.unreturned {
            "unknown"
        } else {
            "completed"
        }
    }

    pub(crate) fn call_intent(
        &mut self,
        spec: &CommandSpec,
        input: &[u8],
        digest: &str,
        timeout: Duration,
    ) -> Result<usize, String> {
        self.calls += 1;
        let n = self.calls;
        let paths = self.process_paths(n);
        let inventory = Inventory::adopted(&self.root)?.ok_or("PR retention owner disappeared")?;
        let originals = paths
            .iter()
            .map(|path| {
                path.strip_prefix(&self.root)
                    .unwrap()
                    .to_str()
                    .ok_or("process original path UTF-8")
                    .map(str::to_owned)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if originals
            .iter()
            .any(|path| !inventory.accepts("check-original", path))
        {
            return Err(
                "PR process originals require registered check-original retention scopes".into(),
            );
        }
        let input_original = self.original(&format!("call-{n}.stdin"), input)?;
        self.write(&format!("call-{n}.intent.json"), &json!({
            "schema":"chrono-pr-call/v1", "operation":self.path, "attempt":n, "producer":"chrono-pr",
            "argv":std::iter::once(&spec.program).chain(spec.args.iter()).collect::<Vec<_>>(),
            "executable_sha256":digest, "stdin_original":input_original,
            "process_originals":{"stdout":originals[0],"stderr":originals[1],"launch":originals[2]},
            "environment_sha256":sha256(&serde_json::to_vec(&spec.env).map_err(|e| e.to_string())?),
            "timeout_millis":timeout.as_millis(), "output_limit_bytes":spec.output_limit_bytes,
            "state":"intent", "updated_at_unix_millis":now()?, "handle":null, "result":null,
            "recovery":"no terminal result or live launch publication means unknown; do not infer completion or blindly recreate"
        }))?;
        self.unreturned = true;
        Ok(n)
    }

    pub(crate) fn receipt(&mut self, n: usize, process: &ProcessResult) -> Result<Value, String> {
        self.unreturned = false; // Actual engine returned; storage failure keeps its complete embedded result.
        let paths = self.process_paths(n);
        let inventory = Inventory::adopted(&self.root)?.ok_or("PR retention owner disappeared")?;
        let mut originals = Vec::new();
        for path in &paths {
            let relative = path
                .strip_prefix(&self.root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("process original path UTF-8")?
                .to_owned();
            let reader =
                read_guard(&self.root, &relative)?.ok_or("process original is not enrolled")?;
            let (digest, length) = file_identity(path)?;
            if !inventory.published_original(&relative, &digest, length)? {
                return Err("process original is not durably published; preserve originals".into());
            }
            originals.push(json!({"path":relative,"sha256":digest,"length":length}));
            self.inputs.push((relative, reader));
        }
        validate_launch_evidence(&paths[2], process)?;
        for (index, (digest, length)) in [
            (&process.stdout_sha256, process.stdout_bytes.len()),
            (&process.stderr_sha256, process.stderr_bytes.len()),
        ]
        .into_iter()
        .enumerate()
        {
            if originals[index]["sha256"] != *digest || originals[index]["length"] != length as u64
            {
                return Err("process original stream differs from returned bytes".into());
            }
        }
        let mut receipt = Self::process_fields(process);
        receipt["stdout_original"] = originals[0].clone();
        receipt["stderr_original"] = originals[1].clone();
        receipt["launch_original"] = originals[2].clone();
        receipt["updated_at_unix_millis"] = json!(now()?);
        self.write(&format!("call-{n}.result.json"), &receipt)?;
        Ok(receipt)
    }

    pub(crate) fn process_fields(process: &ProcessResult) -> Value {
        json!({"argv":process.argv,"executable":process.executable,"sha256":process.sha256,
            "exit_code":process.exit_code,"exit_signal":process.exit_signal,"failure":process.failure,
            "child_pid":process.child_pid,"launcher_pid":process.launcher_pid,
            "stdout_complete":process.stdout_complete,"stderr_complete":process.stderr_complete,
            "timeout_seconds":process.timeout_seconds,"stdout_sha256":process.stdout_sha256,"stderr_sha256":process.stderr_sha256})
    }

    pub(crate) fn finish(self, result: &Value) -> Result<(), String> {
        self.write("result.json", result)?;
        let inputs = self
            .inputs
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        let inventory = Inventory::adopted(&self.root)?.ok_or("PR retention owner disappeared")?;
        inventory.dependencies_paths(&self.path, &inputs)?;
        let terminal_object = matches!(result["pr_state"].as_str(), Some("CLOSED" | "MERGED"));
        // Only an established result with no still-active PR can release this
        // attempt's recovery reference. Other attempts and objects are untouched.
        let release = result["error"].is_null()
            && result["outcome"] != "stale"
            && (terminal_object || result["pr_state"].is_null());
        let root = self.root.clone();
        let path = self.path.clone();
        self.publication.complete(json!({"kind":"pr-operation","operation":path,"command_exit_code":result["command_exit_code"],"pr_state":result["pr_state"],"result":"result.json"}))?;
        let inventory = Inventory::adopted(&root)?.ok_or("PR retention owner disappeared")?;
        inventory.retain_diagnostic_path(&path)?;
        if !inputs.is_empty() {
            let _reader =
                read_guard(&root, &path)?.ok_or("PR result disappeared before handoff")?;
            inventory.settle_publication_inputs(&path, &inputs)?;
        }
        if let Some(previous) = self.continuation {
            // Keep both recovery roots on every failed/stale query or incomplete
            // publication. A current PR observation does not finish an old task.
            if result["error"].is_null()
                && matches!(
                    result["pr_state"].as_str(),
                    Some("OPEN" | "CLOSED" | "MERGED")
                )
                && result["observed_head"] == result["expected_head"]
            {
                let _current_reader = read_guard(&root, &path)?
                    .ok_or("continued result disappeared before handoff")?;
                let result_bytes = serde_json::to_vec(result).map_err(|e| e.to_string())?;
                if !inventory.published_tree_member(
                    &self.producer,
                    &path,
                    &format!("{path}/result.json"),
                    &sha256(&result_bytes),
                    result_bytes.len() as u64,
                )? {
                    return Err(
                        "continued result is not sealed; both recovery roots remain protected"
                            .into(),
                    );
                }
                inventory.retain_diagnostic_path(&previous.path)?;
                inventory.reference_paths(&Self::recovery_role(&previous.path), &[])?;
            }
        }
        if release {
            inventory.reference_paths(&Self::recovery_role(&path), &[])?;
        }
        Ok(())
    }
}
