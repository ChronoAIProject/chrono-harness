//! One registered Cargo entry: validate metadata, execute its declared consumer,
//! then check retained input identities. Generic execution needs no Cargo hook.
use chrono_harness::{
    CommandSpec, ProcessResult, facts, json, no_symlink_parents, observation, run_process_observed,
};
use chrono_judge_registration::Registrations;
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Serialize)]
pub struct Report {
    schema: &'static str,
    pub exit_code: u8,
    status: &'static str,
    error: Option<String>,
    operation_id: String,
    tool: Option<observation::Tool>,
    compiler: Option<observation::Tool>,
    metadata: Option<ProcessResult>,
    operation: Option<ProcessResult>,
    configuration: Option<crate::configuration::Evidence>,
    input_closure_complete: bool,
}
impl Report {
    fn fail(&mut self, code: u8, message: String) {
        self.exit_code = code;
        self.status = if code == 2 { "error" } else { "failed" };
        self.error = Some(message);
    }
}
pub fn run(root: &Path, config: &str, policy: &str, operation: &str) -> Report {
    let mut report = Report {
        schema: "chrono-cargo-run/v2",
        exit_code: 0,
        status: "passed",
        error: None,
        operation_id: operation.into(),
        tool: None,
        compiler: None,
        metadata: None,
        operation: None,
        configuration: None,
        input_closure_complete: false,
    };
    if let Err(error) = evaluate(root, config, policy, operation, &mut report) {
        report.fail(1, error);
    }
    report
}
fn evaluate(
    root: &Path,
    config: &str,
    policy: &str,
    operation: &str,
    report: &mut Report,
) -> Result<(), String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let config_value =
        json(&fs::read(no_symlink_parents(&root, config)?).map_err(|e| e.to_string())?)?;
    let values: BTreeMap<_, _> = facts::registry_paths(&config_value, config)?
        .into_iter()
        .map(|path| {
            Ok((
                path.clone(),
                json(&fs::read(no_symlink_parents(&root, &path)?).map_err(|e| e.to_string())?)?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let registrations = Registrations::load(&values, config)?;
    let snapshot: serde_json::Map<String, Value> = registrations.config()["environment"]["inherit"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| {
            let key = key.as_str().unwrap();
            let value = match std::env::var(key) {
                Ok(v) => Value::String(v),
                Err(std::env::VarError::NotPresent) => Value::Null,
                Err(e) => return Err(format!("E_CARGO_INPUT: environment {key}: {e}")),
            };
            Ok((key.to_string(), value))
        })
        .collect::<Result<_, String>>()?;
    let environment = chrono_judge_registration::inputs::environment(
        registrations.config(),
        &Value::Object(snapshot),
    )?;
    let check = crate::inputs::prepare(
        &root,
        &registrations,
        config,
        policy,
        operation,
        &environment,
    )?;
    report.configuration = Some(check.configuration.evidence.clone());
    let contract = &check.contract;
    let mut tools = BTreeMap::new();
    let ids = std::iter::once(contract.metadata.tool.as_str())
        .chain(contract.compiler.iter().map(|c| c.tool.as_str()));
    for id in ids {
        let declarations: Vec<_> = registrations.config()["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["id"] == id)
            .collect();
        if declarations.len() != 1 {
            return Err(format!(
                "E_CARGO_INPUT: tool {id} is not uniquely registered"
            ));
        }
        let declaration = declarations[0];
        let program = declaration["program"]
            .as_str()
            .ok_or("E_CARGO_INPUT: tool program missing")?;
        let version_argv: Vec<String> = serde_json::from_value(declaration["version_argv"].clone())
            .map_err(|e| e.to_string())?;
        if let Some(input) = check.tool_inputs.get(id) {
            let path = chrono_harness::resolve_program(
                &root,
                program,
                Some(environment.get("PATH").map(String::as_str).unwrap_or("")),
            )?;
            if path != input.path
                || chrono_harness::file_identity(&path)?.0 != input.sha256
                || declaration["expected_version"].as_str().is_none()
            {
                return Err(format!(
                    "E_CARGO_INPUT: tool {id} path/digest/version is not bound to its declared input"
                ));
            }
        }
        let tool = observation::tool(
            &root,
            program,
            &version_argv,
            &environment,
            contract.timeout_seconds,
            contract.output_limit_bytes,
        )?;
        if id == contract.metadata.tool {
            report.tool = Some(tool.clone());
        } else {
            report.compiler = Some(tool.clone());
        }
        chrono_judge_routes::validate_tool(
            &tool,
            program,
            &version_argv,
            &environment,
            &root,
            declaration["expected_version"].as_str(),
        )?;
        check.unchanged()?;
        if let Some(input) = check.tool_inputs.get(id) {
            if tool.path != input.path || tool.sha256 != input.sha256 {
                return Err(format!(
                    "E_CARGO_INPUT: tool {id} changed during observation"
                ));
            }
        }
        tools.insert(id, tool);
    }
    let tool = &tools[contract.metadata.tool.as_str()];
    let invoke = |argv: Vec<String>| {
        run_process_observed(
            &root,
            &CommandSpec {
                program: tool
                    .path
                    .to_str()
                    .ok_or("E_CARGO_INPUT: tool path UTF-8")?
                    .into(),
                args: argv,
                env: environment.clone(),
                timeout_seconds: contract.timeout_seconds,
                output_limit_bytes: contract.output_limit_bytes,
            },
            &[],
            &tool.sha256,
        )
    };
    let metadata = invoke(contract.metadata.argv.clone())?;
    report.metadata = Some(metadata);
    let metadata = report.metadata.as_ref().unwrap();
    if let Some(failure) = &metadata.failure {
        report.fail(2, format!("E_CARGO_PROCESS: metadata: {failure}"));
        return Ok(());
    }
    if metadata.exit_code != 0 {
        report.fail(
            u8::try_from(metadata.exit_code)
                .ok()
                .filter(|c| *c != 0)
                .unwrap_or(1),
            format!("E_CARGO_PROCESS: metadata exit {}", metadata.exit_code),
        );
        return Ok(());
    }
    check.validate(&json(&metadata.stdout_bytes)?)?;
    check.unchanged()?;
    let process = invoke(contract.operations[operation].argv.clone())?;
    report.operation = Some(process);
    check.unchanged()?;
    let process = report.operation.as_ref().unwrap();
    if let Some(failure) = &process.failure {
        report.fail(2, format!("E_CARGO_PROCESS: operation: {failure}"));
    } else if process.exit_code != 0 {
        report.fail(
            u8::try_from(process.exit_code)
                .ok()
                .filter(|c| *c != 0)
                .unwrap_or(1),
            format!("E_CARGO_PROCESS: operation exit {}", process.exit_code),
        );
    }
    Ok(())
}
