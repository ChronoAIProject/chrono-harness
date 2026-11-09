//! 本宿主原生 bootstrap 入口；原操作、两流和所有者阶段均由实际生产者发布。
use chrono_host_bootstrap::{
    bindings,
    construction::{BuildFailure, Construction},
    download, lifecycle, owner, profile, proved_build, shared,
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
        [command] if command == "--version" => {
            println!("chrono-host-bootstrap {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        [command, host, config] if command == "compile" => {
            let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
            proved_build::build(
                &root,
                config,
                &environment,
                &mut std::io::stdout(),
                &mut std::io::stderr(),
            )
        }
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
        [command, host, config] if command == "construct-owner-stage" => {
            bootstrap(host, config, true)
        }
        [command, host, config] if command == "bootstrap" => bootstrap(host, config, false),
        [host, config] => bootstrap(host, config, false),
        _ => Err("unknown native bootstrap command or arguments".into()),
    }
}

fn execute() -> u8 {
    match run() {
        Ok(()) => 0,
        Err(error) => {
            chrono_harness::log_cli_error(&format!("E_BOOTSTRAP: {}", error.message));
            eprintln!("E_BOOTSTRAP: {}", error.message);
            error
                .exit_code
                .and_then(|code| u8::try_from(code).ok())
                .filter(|code| *code != 0)
                .unwrap_or(1)
        }
    }
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let (host, operation) = match arguments.as_slice() {
        [host, _, operation] if operation.to_str() == Some("produce") => (Some(host), "produce"),
        [host, _, operation, _] if matches!(operation.to_str(), Some("install" | "verify")) => {
            (Some(host), operation.to_str().unwrap())
        }
        [operation, host, _]
            if matches!(
                operation.to_str(),
                Some("bootstrap" | "construct-owner-stage" | "compile")
            ) =>
        {
            (Some(host), operation.to_str().unwrap())
        }
        [host, _] => (Some(host), "bootstrap"),
        _ => (None, ""),
    };
    ExitCode::from(match host {
        Some(host) => chrono_harness::logged_main_at(
            Path::new(host),
            "chrono-host-bootstrap",
            operation,
            execute,
        ),
        None => chrono_harness::logged_main("chrono-host-bootstrap", execute),
    })
}
