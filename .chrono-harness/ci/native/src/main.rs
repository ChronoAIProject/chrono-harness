//! 本宿主原生 bootstrap 入口；原操作、两流和所有者阶段均由实际生产者发布。
use chrono_host_bootstrap::{
    bindings,
    construction::{BuildFailure, Construction},
    download, lifecycle, original, owner, profile, shared,
};
use serde_json::json;
use std::{path::Path, process::ExitCode};

fn bootstrap(host: &str, config: &str, stage_only: bool) -> Result<(), BuildFailure> {
    let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
    let environment = std::env::vars_os().collect();
    let native = bindings::native_platform()?;
    let loaded = profile::Loaded::read(&root, config, &native, &environment)?;
    if stage_only {
        owner::require_capabilities(&root, &environment)?;
        if loaded.profile.owner_stage.is_none() {
            return Err("owner stage declaration is missing".into());
        }
    } else if lifecycle::participate(&root, config, &loaded.profile, &native, &environment)? {
        return Ok(());
    }
    let previous = loaded
        .profile
        .owner_stage
        .as_ref()
        .map(|stage| {
            owner::require_capabilities(&root, &environment)?;
            owner::file_binding(&profile::path(
                &root,
                &stage.selected,
                b".chrono-harness/bin/",
            )?)
        })
        .transpose()?;
    let stage = previous.is_some();
    let mut construction = Construction::begin(&root, loaded, &environment)?;
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let outcome = (|| {
        construction.prerequisites(&mut stdout, &mut stderr)?;
        construction.select()?;
        construction.operations_reusing_owner(&native, &mut stdout, &mut stderr)?;
        let installations: Vec<_> = construction
            .profile
            .install
            .iter()
            .map(|item| (item.source.clone(), item.destination.clone()))
            .collect();
        for (source, destination) in installations {
            construction.install_tool(&root.join(source), &destination)?;
        }
        if let Some(previous) = previous {
            construction.stage_owner(&previous)?;
        }
        Ok(())
    })();
    let published = construction.finish(outcome)?;
    eprintln!(
        "bootstrap original report: {}",
        published.original.display()
    );
    published.outcome?;
    if stage {
        let reference = published
            .original
            .strip_prefix(&root)
            .map_err(|error| error.to_string())?
            .to_str()
            .ok_or("non UTF-8 owner original report")?;
        let digest = chrono_harness::file_identity(&published.original)?.0;
        println!(
            "chrono-owner-stage-result {}",
            json!({"path":reference,"sha256":digest})
        );
    }
    Ok(())
}

fn run() -> Result<(), BuildFailure> {
    let arguments: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|value| {
            value
                .into_string()
                .map_err(|_| BuildFailure::from("non UTF-8 bootstrap argument"))
        })
        .collect::<Result<_, _>>()?;
    let environment = std::env::vars_os().collect();
    let native = bindings::native_platform()?;
    match arguments.as_slice() {
        [command, host, checksum, curl, headers, url] if command == "download" => {
            let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
            let archive =
                download::prepare(&root, checksum, Path::new(curl), headers, url, &environment)?;
            println!(
                "{}",
                serde_json::to_string(&archive).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        [host, config, command] if command == "produce" => {
            let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
            shared::Shared::read(&root, config, &native)?.produce(&root, &environment, &native)
        }
        [host, config, command, selected] if command == "install" || command == "verify" => {
            let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
            let shared = shared::Shared::read(&root, config, &native)?;
            if command == "install" {
                shared.install(&root, &environment, &native, selected)
            } else {
                shared.verify(&root, &environment, &native, selected)
            }
        }
        [command, host, reference] if command == "validate-original-execution" => {
            let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
            let raw = profile::read(&root, reference, b".chrono-harness/state/")?;
            let report = original::validate_fixed_execution(&root, &environment, &native, &raw)?;
            println!(
                "{}",
                json!({"schema":"chrono-bootstrap-original-execution-validation/v1",
                "path":reference, "sha256":chrono_harness::sha256(&raw),
                "source":report.source, "operations":report.operations.len()})
            );
            Ok(())
        }
        [command, host, config] if command == "construct-owner-stage" => {
            bootstrap(host, config, true)
        }
        [command, host, config] if command == "bootstrap" => bootstrap(host, config, false),
        [host, config] => bootstrap(host, config, false),
        _ => Err("unknown native bootstrap command or arguments".into()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("E_BOOTSTRAP: {}", error.message);
            ExitCode::from(
                error
                    .exit_code
                    .and_then(|code| u8::try_from(code).ok())
                    .filter(|code| *code != 0)
                    .unwrap_or(1),
            )
        }
    }
}
