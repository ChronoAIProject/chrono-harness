//! Registered backend observations, separate from build and native action results.
use chrono_harness::{CommandSpec, decode, file_identity, resolve_program, run_process_observed};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub kind: String,
    pub need: String,
    pub output_use: String,
    pub command: CommandSpec,
    pub inherit: Vec<String>,
    pub credential_environment: Vec<String>,
    pub repository_environment: String,
    pub ref_environment: String,
}

pub fn validate(c: &Config) -> Result<(), String> {
    chrono_harness::validate_command(&c.command)?;
    let valid_name = |s: &str| !s.is_empty() && !s.contains(['=', '\0']);
    let inherited: BTreeSet<_> = c.inherit.iter().collect();
    let credentials: BTreeSet<_> = c.credential_environment.iter().collect();
    if c.kind != "github"
        || c.need.trim().is_empty()
        || c.output_use.trim().is_empty()
        || c.command
            .args
            .iter()
            .filter(|s| s.as_str() == "{endpoint}")
            .count()
            != 1
        || c.command
            .args
            .iter()
            .any(|s| s.contains("{endpoint}") && s != "{endpoint}")
        || inherited.len() != c.inherit.len()
        || credentials.len() != c.credential_environment.len()
        || c.inherit
            .iter()
            .any(|s| !valid_name(s) || c.command.env.contains_key(s))
        || credentials.iter().any(|s| !inherited.contains(s))
        || !valid_name(&c.repository_environment)
        || !valid_name(&c.ref_environment)
        || c.repository_environment == c.ref_environment
        || ["GH_TOKEN", "GITHUB_TOKEN"].iter().any(|name| {
            inherited.iter().any(|s| s.as_str() == *name)
                && !credentials.iter().any(|s| s.as_str() == *name)
        })
    {
        return Err("E_CACHE_BACKEND: invalid endpoint, purpose or environment declaration".into());
    }
    Ok(())
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn entries(raw: &[u8], reference: &str) -> Result<Vec<Value>, String> {
    let value: Value = decode(raw)?;
    let pages = value
        .as_array()
        .filter(|p| !p.is_empty())
        .ok_or("E_CACHE_BACKEND: expected nonempty paginated response")?;
    let mut entries = BTreeMap::new();
    for page in pages {
        if page["total_count"].as_u64().is_none() {
            return Err("E_CACHE_BACKEND: missing backend count".into());
        }
        for row in page["actions_caches"]
            .as_array()
            .ok_or("E_CACHE_BACKEND: missing cache entries")?
        {
            let id = row["id"]
                .as_u64()
                .filter(|id| *id > 0)
                .ok_or("E_CACHE_BACKEND: invalid cache id")?;
            if row["ref"] != reference
                || row["size_in_bytes"].as_u64().is_none()
                || ["key", "version", "created_at", "last_accessed_at"]
                    .iter()
                    .any(|key| row[*key].as_str().is_none_or(str::is_empty))
            {
                return Err("E_CACHE_BACKEND: invalid cache entry or ref mismatch".into());
            }
            if let Some(previous) = entries.insert(id, row.clone()) {
                if previous != *row {
                    return Err("E_CACHE_BACKEND: conflicting cache id observations".into());
                }
            }
        }
    }
    // Pagination is a sequence of observations, not an atomic inventory. An
    // unobserved key is never promoted to proof of absence.
    Ok(entries.into_values().collect())
}

fn observe(root: &Path, c: &Config, report: &mut Value) -> Result<(), String> {
    let repository = std::env::var(&c.repository_environment)
        .map_err(|e| format!("E_CACHE_BACKEND: repository: {e}"))?;
    let reference =
        std::env::var(&c.ref_environment).map_err(|e| format!("E_CACHE_BACKEND: ref: {e}"))?;
    let parts: Vec<_> = repository.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty()
                || [".", ".."].contains(part)
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
        || !reference.starts_with("refs/")
        || reference.len() <= 5
        || reference.chars().any(char::is_control)
    {
        return Err("E_CACHE_BACKEND: invalid repository or ref".into());
    }
    report["repository"] = json!(repository);
    report["ref"] = json!(reference);
    let endpoint = format!(
        "repos/{repository}/actions/caches?ref={}&per_page=100",
        encode(&reference)
    );
    report["endpoint"] = json!(endpoint);
    let mut command = c.command.clone();
    for arg in &mut command.args {
        if arg == "{endpoint}" {
            *arg = endpoint.clone();
        }
    }
    for name in &c.inherit {
        match std::env::var(name) {
            Ok(value) => {
                command.env.insert(name.clone(), value);
            }
            Err(std::env::VarError::NotPresent) => {}
            Err(e) => return Err(format!("E_CACHE_BACKEND: environment {name}: {e}")),
        }
    }
    let executable = resolve_program(
        root,
        &command.program,
        Some(command.env.get("PATH").map(String::as_str).unwrap_or("")),
    )?;
    let digest = file_identity(&executable)?.0;
    command.program = executable
        .to_str()
        .ok_or("E_CACHE_BACKEND: executable encoding")?
        .into();
    let p = run_process_observed(root, &command, &[], &digest)?;
    let environment: BTreeMap<_, _> = p
        .environment
        .iter()
        .filter(|(name, _)| !c.credential_environment.contains(name))
        .collect();
    report["process"] = json!({"argv":p.argv,"cwd":p.cwd,"executable":p.executable,"sha256":p.sha256,
        "environment":environment,"omitted_credentials":c.credential_environment,
        "original_environment_digest":p.environment_digest,"stdin_sha256":p.stdin_sha256,
        "stdout_bytes":p.stdout_bytes,"stderr_bytes":p.stderr_bytes,
        "stdout_sha256":p.stdout_sha256,"stderr_sha256":p.stderr_sha256,
        "exit_code":p.exit_code,"failure":p.failure});
    if p.exit_code != 0 || p.failure.is_some() {
        return Err("E_CACHE_BACKEND: original backend process failed".into());
    }
    report["entries"] = json!(entries(&p.stdout_bytes, &reference)?);
    report["status"] = json!("observed");
    Ok(())
}

pub(crate) fn apply(
    root: &Path,
    directory: &str,
    c: &Config,
    report: &mut Value,
) -> Result<(), String> {
    validate(c)?;
    let mut observed = json!({"schema":"chrono-cache-github-observation/v1","status":"unavailable",
        "policy":c,"process":null,"error":null,"entries":[],
        "limits":["paginated observations are not an atomic inventory", "backend presence does not identify the creating action or verify archive contents"]});
    let required = report["caches"]
        .as_object()
        .ok_or("E_CACHE_BACKEND: report caches missing")?
        .values()
        .any(|cache| cache["save"]["status"] == "unconfirmed");
    if !required {
        observed["status"] = json!("not-required");
    } else if let Err(error) = observe(root, c, &mut observed) {
        observed["error"] = json!(error);
    }
    let raw = serde_json::to_vec(&observed).map_err(|e| e.to_string())?;
    observed["original"] = json!(super::report::retain_for_upload(
        root,
        directory,
        "cache-backend",
        &raw
    )?);
    let available = observed["status"] == "observed";
    let mut confirmed = BTreeSet::new();
    let caches = report["caches"]
        .as_object_mut()
        .ok_or("E_CACHE_BACKEND: report caches missing")?;
    for (id, cache) in caches {
        let matched: Vec<_> = observed["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["key"] == cache["requested_key"])
            .cloned()
            .collect();
        let state = if !required {
            "not-requested"
        } else if !available {
            "unavailable"
        } else {
            match matched.len() {
                0 => "not-observed",
                1 => "present",
                _ => "ambiguous",
            }
        };
        cache["backend"] = json!({"status":state,"entries":matched});
        if state == "present" && cache["save"]["status"] == "unconfirmed" {
            cache["save"]["status"] = json!("backend-entry-observed");
            cache["save"]["confirmed"] = json!(true);
            cache["save"]["creator"] = json!("unestablished");
            confirmed.insert(id.clone());
        }
    }
    let warnings = report["warnings"]
        .as_array_mut()
        .ok_or("E_CACHE_BACKEND: report warnings missing")?;
    warnings.retain(|w| {
        !(w["code"] == "W_CACHE_SAVE_UNCONFIRMED"
            && w["cache"].as_str().is_some_and(|id| confirmed.contains(id)))
    });
    if required && !available {
        warnings.push(
            json!({"code":"W_CACHE_BACKEND_UNAVAILABLE","cache":"backend",
            "message":"backend availability was not established; original observation retained"}),
        );
    }
    report["schema"] = json!("chrono-cache-transport/v2");
    report["backend"] = observed;
    Ok(())
}
