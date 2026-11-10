//! 本宿主的 bootstrap 编排；原所有者继续负责租约、部署、报告保留和清理。
use crate::{
    bindings::{self, Environment, NativePlatform},
    construction::BuildFailure,
    original, owner,
    profile::{self, Profile},
    source,
};
use chrono_harness::{
    ProcessResult, decode, file_identity, json,
    retained_artifacts::retention::{self, Consumer, Inventory, Publication},
    sha256,
};
use serde::Deserialize;
use serde_json::Value;
use std::{ffi::OsStr, fs, io::Write, os::unix::fs::PermissionsExt, path::Path};

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut stream| stream.write_all(bytes))
        .map_err(|error| error.to_string())
}

fn write_attempt(
    publication: Option<&Publication>,
    path: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    match publication {
        Some(publication) => publication.write_file(path, bytes),
        None => write_new(path, bytes),
    }
}

fn retained_process(value: &Value) -> Result<ProcessResult, String> {
    if !matches!(
        value["encoding"].as_str(),
        Some("chrono-retained-process/v1" | "chrono-retained-process/v2")
    ) {
        return Err("unsupported lifecycle process encoding".into());
    }
    serde_json::from_value(chrono_harness::full::expand_process(value)?)
        .map_err(|error| error.to_string())
}

fn forward(process: &ProcessResult) -> Result<(), String> {
    std::io::stdout()
        .write_all(&process.stdout_bytes)
        .map_err(|error| error.to_string())?;
    std::io::stderr()
        .write_all(&process.stderr_bytes)
        .map_err(|error| error.to_string())
}

fn process_outcome(process: &ProcessResult) -> Result<(), BuildFailure> {
    if process.failure.is_some() || process.exit_code != 0 {
        return Err(BuildFailure {
            message: format!("bootstrap process failed: {:?}", process.failure),
            exit_code: Some(process.exit_code),
        });
    }
    Ok(())
}

pub fn run_owner(
    root: &Path,
    environment: &Environment,
    program: &Path,
    arguments: &[&str],
    directory: &Path,
    name: &str,
    publication: Option<&Publication>,
) -> Result<Value, BuildFailure> {
    let consumer = format!(
        "bootstrap-owner:{}:{name}",
        directory
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or("invalid owner attempt")?
    );
    let mut command = bindings::command(root, program, arguments, environment)?;
    let output = command
        .args(["--report-consumer", &consumer])
        .output()
        .map_err(|error| error.to_string())?;
    write_attempt(
        publication,
        &directory.join(format!("{name}.stdout")),
        &output.stdout,
    )?;
    write_attempt(
        publication,
        &directory.join(format!("{name}.stderr")),
        &output.stderr,
    )?;
    use std::os::unix::process::ExitStatusExt;
    let exit = crate::verified_bindings::process_exit(output.status.code(), output.status.signal());
    write_attempt(
        publication,
        &directory.join(format!("{name}.exit")),
        format!("{}\n", exit.unwrap_or(1)).as_bytes(),
    )?;
    std::io::stderr()
        .write_all(&output.stderr)
        .map_err(|error| error.to_string())?;
    let report = match json(&output.stdout) {
        Ok(report) => report,
        Err(error) => {
            std::io::stdout()
                .write_all(&output.stdout)
                .map_err(|error| error.to_string())?;
            return Err(BuildFailure {
                message: format!("owner phase has no original JSON report: {error}"),
                exit_code: exit.filter(|code| *code != 0),
            });
        }
    };
    let declaration = &report["report_consumer"];
    let originals = declaration["originals"]
        .as_array()
        .ok_or("owner phase report consumer originals are missing")?;
    if declaration["schema"] != "chrono-retention-consumer/v1"
        || declaration["consumer"] != consumer
        || !report["report_path"].is_string()
        || originals.len() != 1
        || originals[0]["path"] != report["report_path"]
    {
        return Err("owner phase report consumer differs from the original".into());
    }
    write_attempt(
        publication,
        &directory.join(format!("{name}-consumer.json")),
        &serde_json::to_vec(declaration).map_err(|error| error.to_string())?,
    )?;
    if let Some(publication) = publication {
        let consumer: Consumer =
            decode(&serde_json::to_vec(declaration).map_err(|error| error.to_string())?)?;
        publication.accept_consumer(&consumer)?;
    }
    if let Some(value) = report.get("managed_process") {
        let process = retained_process(value)?;
        forward(&process)?;
        process_outcome(&process)?;
    }
    if !output.status.success() {
        return Err(BuildFailure {
            message: format!("owner phase failed: {}", output.status),
            exit_code: exit,
        });
    }
    Ok(report)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Upgrade {
    schema: String,
    selected_original: String,
    stage_config: String,
}

pub fn upgrade(
    root: &Path,
    config: &str,
    profile: &Profile,
    native: &NativePlatform,
    environment: &Environment,
) -> Result<(), BuildFailure> {
    let declaration: Upgrade = decode(&profile::read(
        root,
        profile
            .owner_upgrade
            .as_ref()
            .ok_or("owner upgrade is missing")?,
        b".chrono-harness/ci/",
    )?)?;
    if declaration.schema != "chrono-bootstrap-owner-upgrade/v1" {
        return Err("unsupported owner upgrade declaration".into());
    }
    profile::path(
        root,
        &declaration.selected_original,
        b".chrono-harness/state/",
    )?;
    profile::path(root, &declaration.stage_config, b".chrono-harness/ci/")?;
    let observed = source::observe(root, environment);
    if observed.dirty != Some(false) || observed.error.is_some() {
        return Err("owner upgrade requires clean committed candidate source".into());
    }
    let selected = profile::path(
        root,
        &profile.participation.program,
        b".chrono-harness/bin/",
    )?;
    let current_path = profile::path(root, owner::CURRENT, b".chrono-harness/state/")?;
    if current_path.exists() {
        let current = json(&fs::read(&current_path).map_err(|error| error.to_string())?)?;
        if current["schema"] == "chrono-worktree-owner-adoption/v1"
            && current["source_commit"].as_str() == observed.commit.as_deref()
            && current["installed"]
                == serde_json::to_value(owner::file_binding(&selected)?)
                    .map_err(|error| error.to_string())?
        {
            owner::qualified_publication(
                root,
                environment,
                native,
                &current,
                &profile.participation.program,
            )?;
            return Ok(());
        }
    }
    let original = profile.report_destination(root)?;
    if original.exists() {
        let raw = fs::read(original).map_err(|error| error.to_string())?;
        let report: crate::construction::Report = decode(&raw)?;
        if report.source.as_ref().is_some_and(|identity| {
            identity.commit == observed.commit && identity.tree == observed.tree
        }) {
            original::Producer::read(config, native, &|path| {
                profile::read(root, path, b".chrono-harness/")
            })?
            .validate(&raw, &observed, &|path| {
                profile::read(root, path, b".chrono-harness/state/")
            })?;
            return Ok(());
        }
    }
    let qualification = owner::qualified_selected_original(
        root,
        environment,
        native,
        &declaration.selected_original,
        &profile.participation.program,
    )?;
    let original = profile::read(root, &qualification.path, b".chrono-harness/state/")?;
    let bytes = fs::read(&selected).map_err(|error| error.to_string())?;
    if sha256(&original) != qualification.sha256
        || sha256(&bytes) != qualification.selected.sha256
        || bytes.len() as u64 != qualification.selected.size
        || !crate::verified_bindings::owner_publication::binding_equal(
            &owner::file_binding(&selected)?,
            &qualification.selected,
        )
    {
        return Err("qualified selected owner changed before construction".into());
    }
    let parent = profile::path(
        root,
        ".chrono-harness/state/bootstrap-owner-upgrade",
        b".chrono-harness/state/",
    )?;
    fs::create_dir_all(&parent).map_err(|error| error.to_string())?;
    let directory = retention::unique_path(&parent, "attempt-", "")?;
    let publication = retention::create_adopted(
        root,
        "check-original",
        &directory,
        true,
        serde_json::json!({"status":"started","completion":"not-established"}),
    )?;
    let relative = directory
        .strip_prefix(root)
        .map_err(|error| error.to_string())?
        .to_str()
        .ok_or("non UTF-8 owner attempt")?;
    if publication.is_some() {
        Inventory::adopted(root)?
            .ok_or("owner attempt retention policy disappeared")?
            .retain_diagnostic_path(relative)?;
    }
    let outcome = (|| -> Result<(), BuildFailure> {
        write_attempt(
            publication.as_ref(),
            &directory.join("selected-producer.json"),
            &original,
        )?;
        write_attempt(
            publication.as_ref(),
            &directory.join("selected-qualification.json"),
            &serde_json::to_vec(&qualification).map_err(|error| error.to_string())?,
        )?;
        let carrier = directory.join("construction-owner");
        write_attempt(publication.as_ref(), &carrier, &bytes)?;
        fs::set_permissions(&carrier, fs::Permissions::from_mode(0o755))
            .map_err(|error| error.to_string())?;
        if !crate::verified_bindings::owner_publication::binding_equal(
            &owner::file_binding(&selected)?,
            &qualification.selected,
        ) || file_identity(&carrier)?.0 != qualification.selected.sha256
        {
            return Err("qualified selected owner changed before construction".into());
        }
        write_attempt(
            publication.as_ref(),
            &directory.join("upgrade.json"),
            &profile::read(
                root,
                profile.owner_upgrade.as_ref().unwrap(),
                b".chrono-harness/ci/",
            )?,
        )?;
        let host = root.to_str().ok_or("non UTF-8 host root")?;
        let common = [
            "--host-root",
            host,
            "--config",
            profile.participation.config.as_str(),
        ];
        run_owner(
            root,
            environment,
            &carrier,
            &[
                "migrate", common[0], common[1], common[2], common[3], "--path", host,
            ],
            &directory,
            "migration",
            publication.as_ref(),
        )?;
        let built = run_owner(
            root,
            environment,
            &carrier,
            &[
                "bootstrap",
                common[0],
                common[1],
                common[2],
                common[3],
                "--bootstrap-config",
                &declaration.stage_config,
            ],
            &directory,
            "construction",
            publication.as_ref(),
        )?;
        let process = retained_process(&built["managed_process"])?;
        let last = process
            .stdout_bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .next_back()
            .ok_or("construction lacks its original stage result")?;
        let descriptor: owner::Reference = decode(
            last.strip_prefix(b"chrono-owner-stage-result ")
                .ok_or("construction lacks its original stage result")?,
        )?;
        let qualified = owner::stage_report(
            root,
            environment,
            native,
            &descriptor.path,
            Some(&descriptor.sha256),
            None,
        )?;
        let staged = profile::path(root, &qualified.stage.staged.path, owner::STAGES.as_bytes())?;
        run_owner(
            root,
            environment,
            &staged,
            &[
                "deploy-owner",
                common[0],
                common[1],
                common[2],
                common[3],
                "--bootstrap-config",
                &declaration.stage_config,
                "--stage-result",
                &descriptor.path,
                "--stage-sha256",
                &descriptor.sha256,
            ],
            &directory,
            "deployment",
            publication.as_ref(),
        )?;
        let adopted = json(&fs::read(current_path).map_err(|error| error.to_string())?)?;
        if adopted["source_commit"].as_str() != observed.commit.as_deref()
            || adopted["installed"]
                != serde_json::to_value(owner::file_binding(&selected)?)
                    .map_err(|error| error.to_string())?
        {
            return Err("owner deployment did not publish current adoption".into());
        }
        Ok(())
    })();
    if let Some(publication) = publication {
        let completion = publication.complete(serde_json::json!({
            "status":if outcome.is_ok() {"completed"} else {"failed"},
            "error":outcome.as_ref().err().map(|error| &error.message),
            "exit_code":outcome.as_ref().err().and_then(|error| error.exit_code),
        }));
        let diagnostic = if outcome.is_err() || completion.is_err() {
            Inventory::adopted(root)?
                .ok_or("owner attempt retention policy disappeared")?
                .retain_diagnostic_path(relative)
        } else {
            Ok(())
        };
        if let Err(error) = completion.and(diagnostic) {
            return Err(BuildFailure {
                message: match &outcome {
                    Ok(()) => error,
                    Err(primary) => {
                        format!("{}; owner attempt retention: {error}", primary.message)
                    }
                },
                exit_code: outcome.as_ref().err().and_then(|error| error.exit_code),
            });
        }
    }
    outcome
}

/// 返回 true 表示实际生命周期子进程已经完成本次 bootstrap。
pub fn participate(
    root: &Path,
    config: &str,
    profile: &Profile,
    native: &NativePlatform,
    environment: &Environment,
) -> Result<bool, BuildFailure> {
    if let Some(marker) = environment.get(OsStr::new("CHRONO_WORKTREE_BOOTSTRAP")) {
        if Path::new(marker) != root {
            return Err("bootstrap participation root mismatch".into());
        }
        if profile.owner_upgrade.is_some() || profile.owner_stage.is_some() {
            owner::require_capabilities(root, environment)?;
        }
        return Ok(false);
    }
    let anchor = owner::coordinator_root(root, profile, environment)?;
    let local = profile::path(
        root,
        &profile.participation.program,
        b".chrono-harness/bin/",
    )?;
    let local_present = match fs::symlink_metadata(&local) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.to_string().into()),
    };
    let program =
        if crate::verified_bindings::bootstrap_uses_local_owner(anchor == root, local_present) {
            local
        } else {
            profile::path(
                &anchor,
                &profile.participation.program,
                b".chrono-harness/bin/",
            )?
        };
    if anchor == root && root.join(".git").is_dir() && !program.exists() {
        return Ok(false);
    }
    if anchor == root && profile.owner_upgrade.is_some() {
        upgrade(root, config, profile, native, environment)?;
    }
    let output = bindings::command(
        root,
        &program,
        &[
            "bootstrap",
            "--host-root",
            root.to_str().ok_or("non UTF-8 host root")?,
            "--config",
            &profile.participation.config,
            "--bootstrap-config",
            config,
        ],
        environment,
    )?
    .output()
    .map_err(|error| error.to_string())?;
    let report = json(&output.stdout).map_err(|error| {
        format!(
            "bootstrap ownership refused: {error}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })?;
    let process = retained_process(&report["managed_process"])?;
    forward(&process)?;
    std::io::stderr()
        .write_all(&output.stderr)
        .map_err(|error| error.to_string())?;
    eprintln!(
        "bootstrap ownership original report: {}",
        anchor
            .join(
                report["report_path"]
                    .as_str()
                    .ok_or("bootstrap ownership original path is missing")?
            )
            .display()
    );
    if report["status"] != "used" && report["managed_command_failed"] != true {
        return Err(format!("bootstrap lifecycle failed: {}", report["error"]).into());
    }
    process_outcome(&process)?;
    if !output.status.success() {
        return Err("bootstrap lifecycle wrapper failed".into());
    }
    Ok(true)
}
