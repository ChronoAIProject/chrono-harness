//! PR-owned immutable operation originals, protected by the existing output owner.
use chrono_harness::retained_artifacts::retention::{Inventory, Publication, unique_path};
use chrono_harness::{CommandSpec, ProcessResult, no_symlink_parents, sha256};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(crate) struct State {
    root: PathBuf,
    pub(crate) path: String,
    publication: Publication,
    calls: usize,
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
            path,
            publication,
            calls: 0,
        };
        let saved = (|| {
            let config_original = state.original("config.json", config_bytes)?;
            state.write("intent.json", &json!({
                "schema":"chrono-pr-operation/v1", "operation":state.path, "producer":"chrono-pr",
                "command":args.command, "arguments":args.values, "draft":args.draft,
                "config_original":config_original,
                "object":{"repository":config.repository,"base":config.base,"pr":args.number().ok(),"expected_head":args.get("--head-sha")?},
                "state":"intent", "updated_at_unix_millis":now()?, "handle":null, "result":null,
                "recovery":{"method":"inspect exact operation originals before any retry; missing result remains unknown", "consumer":Self::recovery_role(&state.path), "release":"only after matching terminal object evidence and recovery consumers end"}
            }))
        })();
        saved.map_err(|e| format!("PR operation {} intent publication failed: {e}", state.path))?;
        Ok(state)
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

    pub(crate) fn call_intent(
        &mut self,
        spec: &CommandSpec,
        input: &[u8],
        digest: &str,
        timeout: Duration,
    ) -> Result<usize, String> {
        self.calls += 1;
        let n = self.calls;
        let input_original = self.original(&format!("call-{n}.stdin"), input)?;
        self.write(&format!("call-{n}.intent.json"), &json!({
            "schema":"chrono-pr-call/v1", "operation":self.path, "attempt":n, "producer":"chrono-pr",
            "argv":std::iter::once(&spec.program).chain(spec.args.iter()).collect::<Vec<_>>(),
            "executable_sha256":digest, "stdin_original":input_original,
            "environment_sha256":sha256(&serde_json::to_vec(&spec.env).map_err(|e| e.to_string())?),
            "timeout_millis":timeout.as_millis(), "output_limit_bytes":spec.output_limit_bytes,
            "state":"intent", "updated_at_unix_millis":now()?, "handle":null, "result":null,
            "recovery":"no terminal result or live launch publication means unknown; do not infer completion or blindly recreate"
        }))?;
        Ok(n)
    }

    pub(crate) fn receipt(&self, n: usize, process: &ProcessResult) -> Result<Value, String> {
        let stdout = self.original(&format!("call-{n}.stdout"), &process.stdout_bytes)?;
        let stderr = self.original(&format!("call-{n}.stderr"), &process.stderr_bytes)?;
        let mut receipt = Self::process_fields(process);
        receipt["stdout_original"] = stdout;
        receipt["stderr_original"] = stderr;
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
        let terminal_object = matches!(result["pr_state"].as_str(), Some("CLOSED" | "MERGED"));
        // Only an established result with no still-active PR can release this
        // attempt's recovery reference. Other attempts and objects are untouched.
        let release =
            result["error"].is_null() && (terminal_object || result["pr_state"].is_null());
        let root = self.root.clone();
        let path = self.path.clone();
        self.publication.complete(json!({"kind":"pr-operation","operation":path,"command_exit_code":result["command_exit_code"],"pr_state":result["pr_state"],"result":"result.json"}))?;
        let inventory = Inventory::adopted(&root)?.ok_or("PR retention owner disappeared")?;
        inventory.retain_diagnostic_path(&path)?;
        if release {
            inventory.reference_paths(&Self::recovery_role(&path), &[])?;
        }
        Ok(())
    }
}
