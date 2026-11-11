//! PR-owned immutable operation originals, protected by the existing output owner.
use chrono_harness::ownership::Lease;
use chrono_harness::retained_artifacts::retention::{
    Inventory, ObservationToken, Publication, read_guard, unique_path,
};
use chrono_harness::verified_release as verified;
use chrono_harness::{
    CommandSpec, ProcessResult, file_identity, no_symlink_parents, sha256, validate_launch_evidence,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectState {
    schema: String,
    request: Option<ObservationToken>,
    observed: Option<Observation>,
    unresolved_create: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    token: ObservationToken,
    active: bool,
}

pub(crate) struct State {
    repository: String,
    object: ObjectState,
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
    object: ObjectState,
    path: String,
    result_original: Value,
    _reader: Lease,
    inputs: Vec<(String, Lease)>,
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
        let (object, _) = read("object-state.json", 1048576)?;
        let object: ObjectState = serde_json::from_value(object).map_err(|e| e.to_string())?;
        if !verified::object_state(&object.schema, object.unresolved_create) {
            return Err("continued operation has unresolved object state".into());
        }
        if let Some(request) = &object.request {
            if !verified::continued_observation(
                &request.binding(),
                &State::object_role(&config.repository, args.number()?),
                path,
            ) {
                return Err("continued observation binding differs".into());
            }
        }
        if let Some(observed) = &object.observed {
            let request = object
                .request
                .as_ref()
                .ok_or("continued observation has no request")?;
            if !verified::matching_observation(&request.binding(), &observed.token.binding())
                || !verified::observed_pr(
                    result["pr_state"].as_str().unwrap_or(""),
                    observed.active,
                )
            {
                return Err("continued observation result differs".into());
            }
        }
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
        let receipts = result["receipts"]
            .as_array()
            .ok_or("continued operation has no original process receipts")?;
        let mut inputs = Vec::new();
        for (index, receipt) in receipts.iter().enumerate() {
            let (saved, _) = read(&format!("call-{}.result.json", index + 1), 16777216)?;
            if saved != *receipt {
                return Err("continued process receipt differs from the sealed original".into());
            }
            for (field, suffix) in [
                ("stdout_original", "stdout"),
                ("stderr_original", "stderr"),
                ("launch_original", "launch.json"),
            ] {
                let original = &receipt[field];
                let expected = format!("{path}.call-{}.{suffix}", index + 1);
                if original["path"] != expected {
                    return Err(
                        "continued process original address differs from its operation".into(),
                    );
                }
                let guard = read_guard(root, &expected)?
                    .ok_or("continued process original is unavailable or not enrolled")?;
                let (digest, length) = file_identity(&no_symlink_parents(root, &expected)?)?;
                if original["sha256"] != digest
                    || original["length"] != length
                    || !inventory.published_original(&expected, &digest, length)?
                {
                    return Err(
                        "continued process original changed or is not sealed; preserve originals"
                            .into(),
                    );
                }
                inputs.push((expected, guard));
            }
        }
        Ok(Some(Self {
            object,
            path: path.clone(),
            result_original,
            _reader: reader,
            inputs,
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
            repository: config.repository.clone(),
            object: ObjectState {
                schema: "chrono-pr-object-state/v1".into(),
                request: None,
                observed: None,
                unresolved_create: false,
            },
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
                "recovery":{"method":"inspect exact operation originals before any retry; missing result remains unknown", "consumer":Self::recovery_role(&state.path), "release":"only after joined command, sealed result and protected per-object handoff; unknown writes remain protected"}
            }))
        })();
        saved.map_err(|e| format!("PR operation {} intent publication failed: {e}", state.path))?;
        Ok(state)
    }

    pub(crate) fn continued_path(&self) -> Option<&str> {
        self.continuation.as_ref().map(|c| c.path.as_str())
    }

    fn object_role(repository: &str, number: u64) -> String {
        format!(
            "pr-object:{}",
            sha256(&serde_json::to_vec(&(repository, number)).unwrap())
        )
    }

    pub(crate) fn reserve_snapshot(&mut self, number: u64) -> Result<(), String> {
        let inventory = Inventory::adopted(&self.root)?.ok_or("PR retention owner disappeared")?;
        let token = inventory.reserve_observation(
            &Self::object_role(&self.repository, number),
            &self.path,
            (self.calls + 1) as u64,
        )?;
        self.object.request = Some(token);
        Ok(())
    }

    pub(crate) fn observed_snapshot(&mut self, active: bool) -> Result<(), String> {
        self.object.observed = Some(Observation {
            token: self
                .object
                .request
                .clone()
                .ok_or("snapshot has no reserved observation")?,
            active,
        });
        Ok(())
    }

    pub(crate) fn confirm_create(&mut self) {
        self.object.unresolved_create = false;
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
        if verified::unknown_pr_operation(self.unreturned, self.object.unresolved_create) {
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
            "observation":self.object.request.as_ref().filter(|token| token.attempt == n as u64),
            "argv":std::iter::once(&spec.program).chain(spec.args.iter()).collect::<Vec<_>>(),
            "executable_sha256":digest, "stdin_original":input_original,
            "process_originals":{"stdout":originals[0],"stderr":originals[1],"launch":originals[2]},
            "environment_sha256":sha256(&serde_json::to_vec(&spec.env).map_err(|e| e.to_string())?),
            "timeout_millis":timeout.as_millis(), "output_limit_bytes":spec.output_limit_bytes,
            "state":"intent", "updated_at_unix_millis":now()?, "handle":null, "result":null,
            "recovery":"no terminal result or live launch publication means unknown; do not infer completion or blindly recreate"
        }))?;
        if self
            .object
            .request
            .as_ref()
            .is_some_and(|token| token.attempt == n as u64)
        {
            // This existing call intent binds the reservation before dispatch.
            // Transfer initial recovery to the actual per-object pending root.
            inventory.reference_paths(&Self::recovery_role(&self.path), &[])?;
        }
        self.unreturned = true;
        if verified::creating_pr(
            spec.args.first().map(String::as_str).unwrap_or(""),
            spec.args.get(1).map(String::as_str).unwrap_or(""),
        ) {
            self.object.unresolved_create = true;
        }
        Ok(n)
    }

    pub(crate) fn receipt(&mut self, n: usize, process: &ProcessResult) -> Result<Value, String> {
        self.unreturned = !verified::process_exit_known(process.exit_code, process.exit_signal);
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
        self.write(
            "object-state.json",
            &serde_json::to_value(&self.object).map_err(|e| e.to_string())?,
        )?;
        self.write("result.json", result)?;
        let inputs = self
            .inputs
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        let inventory = Inventory::adopted(&self.root)?.ok_or("PR retention owner disappeared")?;
        inventory.dependencies_paths(&self.path, &inputs)?;
        let root = self.root.clone();
        let path = self.path.clone();
        self.publication.complete_sealed(json!({"kind":"pr-operation","operation":path,"command_exit_code":result["command_exit_code"],"pr_state":result["pr_state"],"result":"result.json"}))?;
        let inventory = Inventory::adopted(&root)?.ok_or("PR retention owner disappeared")?;
        inventory.retain_diagnostic_path(&path)?;
        let _reader = read_guard(&root, &path)?.ok_or("PR result disappeared before handoff")?;
        if !inputs.is_empty() {
            inventory.settle_publication_inputs(&path, &inputs)?;
        }
        let complete = verified::settled_pr_command(
            self.unreturned,
            self.object.unresolved_create,
            result["receipts"]
                .as_array()
                .is_some_and(|rs| rs.iter().all(|r| r.get("storage_error").is_none())),
        );
        if let Some(previous) = self.continuation {
            if verified::continued_pr_handoff(
                complete,
                result["error"].is_null(),
                self.object.observed.is_some(),
                result["observed_head"] == result["expected_head"],
            ) {
                let result_bytes = serde_json::to_vec(result).map_err(|e| e.to_string())?;
                if !inventory.published_tree_member(
                    &self.producer,
                    &path,
                    &format!("{path}/result.json"),
                    &sha256(&result_bytes),
                    result_bytes.len() as u64,
                )? {
                    return Err("continued result is not sealed; preserve prior originals".into());
                }
                let prior_inputs = previous
                    .inputs
                    .iter()
                    .map(|(path, _)| path.clone())
                    .collect::<Vec<_>>();
                if !prior_inputs.is_empty() {
                    inventory.settle_publication_inputs(&previous.path, &prior_inputs)?;
                }
                inventory.retain_diagnostic_path(&previous.path)?;
                if let Some(request) = &previous.object.request {
                    inventory.settle_observation(
                        request,
                        previous
                            .object
                            .observed
                            .as_ref()
                            .map(|o| (&o.token, o.active)),
                    )?;
                }
                inventory.reference_paths(&Self::recovery_role(&previous.path), &[])?;
            }
        }
        if complete {
            if let Some(request) = &self.object.request {
                inventory.settle_observation(
                    request,
                    self.object.observed.as_ref().map(|o| (&o.token, o.active)),
                )?;
            }
            inventory.reference_paths(&Self::recovery_role(&path), &[])?;
        }
        Ok(())
    }
}
