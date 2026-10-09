use chrono_harness::{
    CommandSpec, file_identity, no_symlink_parents, resolve_program, run_process_observed, sha256,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, io::Write, path::Path};

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
) -> Result<(Value, String), String> {
    let mut command = command.clone();
    for name in inherit {
        match std::env::var(name) {
            Ok(value) => {
                command.env.insert(name.clone(), value);
            }
            Err(std::env::VarError::NotPresent) => {}
            Err(e) => return Err(format!("E_CACHE_PROBE: environment {name}: {e}")),
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
        .ok_or("E_CACHE_PROBE: non-UTF8 executable path")?
        .to_owned();
    let process = run_process_observed(root, &command, &[], &executable_sha256)?;
    let raw = serde_json::to_vec(&process).map_err(|e| e.to_string())?;
    let original = format!(".chrono-harness/state/cache-probes/{}.json", sha256(&raw));
    let output = no_symlink_parents(root, &original)?;
    fs::create_dir_all(output.parent().ok_or("E_CACHE_PROBE: evidence parent")?)
        .map_err(|e| e.to_string())?;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
    {
        Ok(mut file) => file.write_all(&raw).map_err(|e| e.to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read(&output).map_err(|e| e.to_string())? != raw {
                return Err(format!(
                    "E_CACHE_PROBE: retained evidence differs: {original}"
                ));
            }
        }
        Err(e) => return Err(format!("E_CACHE_PROBE: cannot retain original: {e}")),
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
    Ok((observed, original))
}
