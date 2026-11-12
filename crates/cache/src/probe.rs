use chrono_harness::{
    CommandSpec, ProcessEvidence, file_identity, resolve_program, run_process_observed_retained,
    sha256, validate_launch_evidence,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultKind {
    Stdout,
    FilePath,
}

pub fn observe(
    root: &Path,
    command: &CommandSpec,
    inherit: &[String],
    result: &ResultKind,
    attempt: &super::attempt::Attempt,
    input_id: &str,
) -> Result<(Value, Value), String> {
    let directory = attempt.directory.join(input_id);
    attempt.publication.create_directory(&directory)?;
    fs::File::open(&attempt.directory)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    let original = format!("{}/{input_id}/process.json", attempt.relative);
    let intent = json!({"schema":"chrono-cache-probe-intent/v1","attempt":attempt.relative,
        "input":input_id,"command":command,"inherit":inherit,"result_kind":result,
        "stdin_sha256":sha256(&[]),"task_identity":null,"terminal_result":null,
        "launch":"launch.json","stdout":"stdout","stderr":"stderr",
        "wait":"original process engine native join; absent terminal result stays unknown"});
    attempt.record(&directory.join("intent.json"), &intent)?;
    let mut invoked = false;
    let evaluated = (|| {
        let mut command = command.clone();
        for name in inherit {
            match std::env::var(name) {
                Ok(value) => {
                    command.env.insert(name.clone(), value);
                }
                Err(std::env::VarError::NotPresent) => {}
                Err(e) => return Err(format!("environment {name}: {e}")),
            }
        }
        let executable = resolve_program(
            root,
            &command.program,
            Some(command.env.get("PATH").map(String::as_str).unwrap_or("")),
        )?;
        let (executable_sha256, _) = file_identity(&executable)?;
        command.program = executable
            .to_str()
            .ok_or("non-UTF8 executable path")?
            .to_owned();
        attempt.record(
            &directory.join("invocation.json"),
            &json!({"command":command,"executable_sha256":executable_sha256}),
        )?;
        invoked = true;
        attempt.unknown.set(true);
        run_process_observed_retained(
            root,
            &command,
            &[],
            &executable_sha256,
            &ProcessEvidence {
                stdout: &directory.join("stdout"),
                stderr: &directory.join("stderr"),
                launch: &directory.join("launch.json"),
                publication: Some(&attempt.publication),
            },
        )
    })();
    let process = match evaluated {
        Ok(process) => process,
        Err(error) => {
            let record = json!({"error":error,"engine_invoked":invoked,"returned_task_identity":null,
                "launch_original":if invoked {json!({"path":"launch.json","availability":"not-assessed"})} else {Value::Null},
                "terminal_result":null,"completion":if invoked {"unknown"} else {"not-dispatched"}});
            attempt
                .record(&directory.join("error.json"), &record)
                .map_err(|publication| {
                    format!(
                        "E_CACHE_PROBE: {error}; error publication: {publication}; attempt {}",
                        attempt.relative
                    )
                })?;
            return Err(format!(
                "E_CACHE_PROBE: {error}; original {}/{}",
                attempt.relative,
                format!("{input_id}/error.json")
            ));
        }
    };
    // Publish the actual receipt before evaluating launch, exit or file-path semantics.
    let process_value = serde_json::to_value(&process).map_err(|e| e.to_string())?;
    let original_raw = serde_json::to_vec(&process_value).map_err(|e| e.to_string())?;
    attempt.record(&directory.join("process.json"), &process_value).map_err(|e|
        format!("E_CACHE_PROBE: original receipt publication: {e}; attempt {}; observed exit {} signal {:?}; original {original}", attempt.publication.id(), process.exit_code, process.exit_signal))?;
    let launch = validate_launch_evidence(&directory.join("launch.json"), &process);
    let terminal = super::verified_planner::confirmed_probe(
        launch.is_ok(),
        process.exit_code,
        process.exit_signal,
        process.owned_work_joined,
    );
    attempt.unknown.set(!terminal);
    launch.map_err(|e| format!("E_CACHE_PROBE: {e}; original {original}"))?;
    if !terminal {
        return Err(format!(
            "E_CACHE_PROBE: original process terminality unknown; original {original}"
        ));
    }
    if process.exit_code != 0 || process.failure.is_some() {
        return Err(format!(
            "E_CACHE_PROBE: exit {}, failure {:?}; original {original}",
            process.exit_code, process.failure
        ));
    }
    let environment: BTreeMap<_, _> = process
        .environment
        .iter()
        .map(|(name, value)| {
            (
                name,
                json!({"sha256":sha256(value.as_bytes()),"length":value.len()}),
            )
        })
        .collect();
    let mut observed = json!({"executable":process.executable,"executable_sha256":process.sha256,
        "argv":process.argv,"environment":environment,"exit_code":process.exit_code,
        "stdout_sha256":process.stdout_sha256,"stdout_length":process.stdout_bytes.len(),
        "stderr_sha256":process.stderr_sha256,"stderr_length":process.stderr_bytes.len()});
    if matches!(result, ResultKind::FilePath) {
        let value = std::str::from_utf8(&process.stdout_bytes)
            .map_err(|e| format!("E_CACHE_PROBE: path encoding: {e}; original {original}"))?
            .trim_end_matches(['\n', '\r']);
        if value.is_empty() || value.contains(['\n', '\r', '\0']) || !Path::new(value).is_absolute()
        {
            return Err(format!(
                "E_CACHE_PROBE: expected one absolute file path; original {original}"
            ));
        }
        let actual = fs::canonicalize(value)
            .map_err(|e| format!("E_CACHE_PROBE: {e}; original {original}"))?;
        let (digest, length) = file_identity(&actual)
            .map_err(|e| format!("E_CACHE_PROBE: {e}; original {original}"))?;
        observed["file"] = json!({"path":actual,"sha256":digest,"length":length});
    }
    Ok((
        observed,
        json!({"path":original,"sha256":sha256(&original_raw),"attempt_id":attempt.publication.id()}),
    ))
}
