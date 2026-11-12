//! 正式版只消费显式选定、已成功发布并安装的测试版原件。
//! 不改写原回执；GitHub artifact 的不可变身份与原包内容分别核对。
use crate::{
    delivery::{self, Package},
    execution::Environment,
    identity::{self, Channel, Identity},
    publication::{self, Asset, Commands},
    receipt::Selection,
    recipe::{self, Loaded},
};
use chrono_harness::{file_identity, no_symlink_parents};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::Path,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub workflow_id: u64,
    pub workflow_config: String,
}

pub struct Prepared {
    pub identity: Identity,
    pub packages: BTreeMap<String, Package>,
    pub selections: BTreeMap<String, Selection>,
    pub observation: Value,
}

fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_PROMOTION: {message}")
}
fn positive(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == value)
        .ok_or_else(|| error("explicit positive preview run and attempt required"))
}
fn env<'a>(environment: &'a Environment, key: &str) -> Result<&'a str, String> {
    environment
        .get(OsStr::new(key))
        .and_then(|v| v.to_str())
        .ok_or_else(|| error(format!("missing {key}")))
}

/// 固定 attempt 的成功终态包括原发布及两平台安装；不能只核 source SHA。
fn verify_run(
    run: &Value,
    repository: &str,
    workflow_id: u64,
    path: &str,
    id: u64,
    attempt: u64,
) -> Result<(), String> {
    if run["id"] != id
        || run["run_attempt"] != attempt
        || run["workflow_id"] != workflow_id
        || run["path"] != path
        || run["repository"]["full_name"] != repository
        || run["status"] != "completed"
        || run["conclusion"] != "success"
        || !matches!(
            run["event"].as_str(),
            Some("workflow_run" | "workflow_dispatch")
        )
        || !run["head_sha"]
            .as_str()
            .is_some_and(|s| chrono_identifiers::valid_oid(s.as_bytes()))
    {
        return Err(error(
            "original preview workflow attempt is not successful or has another identity",
        ));
    }
    Ok(())
}

/// API 精确名称查询必须只有一个未过期、不可变的工件，再按其同一 run/name 下载。
fn select_artifact(
    list: &Value,
    name: &str,
    run: u64,
    head: &str,
    expected: Option<&str>,
) -> Result<Value, String> {
    let rows = list["artifacts"]
        .as_array()
        .ok_or_else(|| error("missing artifacts"))?;
    if list["total_count"] != 1 || rows.len() != 1 {
        return Err(error("original artifact is missing or ambiguous"));
    }
    let artifact = &rows[0];
    let id = artifact["id"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or_else(|| error("invalid artifact ID"))?;
    if artifact["name"] != name
        || artifact["expired"] != false
        || artifact["workflow_run"]["id"] != run
        || artifact["workflow_run"]["head_sha"] != head
        || artifact["size_in_bytes"].as_u64().is_none_or(|n| n == 0)
        || expected.is_some_and(|expected| expected != id.to_string())
    {
        return Err(error(
            "original artifact identity, expiry or selected producer differs",
        ));
    }
    Ok(artifact.clone())
}

fn download(
    repository: &str,
    run: &Value,
    name: &str,
    destination: &Path,
    expected: Option<&str>,
    commands: &mut impl Commands,
) -> Result<Value, String> {
    if destination.exists() {
        return Err(error("original download destination already exists"));
    }
    let id = run["id"].as_u64().ok_or_else(|| error("missing run ID"))?;
    let list = publication::read_api(
        commands,
        format!("repos/{repository}/actions/runs/{id}/artifacts?name={name}&per_page=100"),
    )?;
    let artifact = select_artifact(
        &list,
        name,
        id,
        run["head_sha"]
            .as_str()
            .ok_or_else(|| error("missing workflow head"))?,
        expected,
    )?;
    let reply = commands.execute(
        "gh",
        &[
            "run".into(),
            "download".into(),
            id.to_string(),
            "--repo".into(),
            repository.into(),
            "--name".into(),
            name.into(),
            "--dir".into(),
            destination
                .to_str()
                .ok_or_else(|| error("invalid download path"))?
                .into(),
        ],
    )?;
    if reply.exit != 0 {
        return Err(error(format!(
            "original artifact download exited {}",
            reply.exit
        )));
    }
    // CLI 的退出不能代替下载结果；后续逐个核原回执、原流及包字节。
    if !destination.is_dir() {
        return Err(error("original artifact download produced no directory"));
    }
    Ok(artifact)
}

fn job<'a>(workflow: &'a Value, id: &str) -> Result<&'a Value, String> {
    let rows = workflow["jobs"]
        .as_array()
        .ok_or_else(|| error("missing workflow jobs"))?;
    let found = rows
        .iter()
        .filter(|row| row["id"] == id)
        .collect::<Vec<_>>();
    if found.len() != 1 {
        return Err(error(format!("missing or ambiguous registered job {id}")));
    }
    Ok(found[0])
}
fn artifact_name(
    workflow: &Value,
    job_id: &str,
    run: &str,
    attempt: &str,
) -> Result<String, String> {
    let prefix = job(workflow, job_id)?["artifact_name"]
        .as_str()
        .ok_or_else(|| error("missing artifact name"))?;
    if !prefix
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        || prefix.is_empty()
    {
        return Err(error("artifact prefix must be an explicit literal name"));
    }
    Ok(format!("{prefix}-{run}-{attempt}"))
}

pub fn prepare(
    root: &Path,
    output: &Path,
    config: &delivery::Config,
    loaded: &Loaded,
    stable: &Identity,
    environment: &Environment,
    commands: &mut impl Commands,
) -> Result<Prepared, String> {
    if stable.channel != Channel::Stable {
        return Err(error("only stable releases promote previews"));
    }
    let promotion = config
        .promotion
        .as_ref()
        .ok_or_else(|| error("missing promotion registration"))?;
    if promotion.workflow_id == 0 {
        return Err(error("missing release workflow identity"));
    }
    let workflow = recipe::read_json(root, &promotion.workflow_config)?;
    let path = workflow["workflow_path"]
        .as_str()
        .ok_or_else(|| error("missing workflow path"))?;
    let publisher = workflow["delivery"]["publisher"]
        .as_str()
        .ok_or_else(|| error("missing publisher"))?;
    let run_id = env(environment, "CHRONO_RELEASE_PREVIEW_RUN")?;
    let attempt = env(environment, "CHRONO_RELEASE_PREVIEW_ATTEMPT")?;
    let (run_number, attempt_number) = (positive(run_id)?, positive(attempt)?);
    if run_id == env(environment, "CHRONO_RELEASE_RUN")? {
        return Err(error("preview must be a completed earlier run"));
    }
    let run = publication::read_api(
        commands,
        format!(
            "repos/{}/actions/runs/{run_id}/attempts/{attempt}",
            config.publication.repository
        ),
    )?;
    verify_run(
        &run,
        &config.publication.repository,
        promotion.workflow_id,
        path,
        run_number,
        attempt_number,
    )?;
    let workflow_identity = identity::verify_workflow_commits(
        path,
        &stable.source_commit,
        run["head_sha"].as_str().unwrap(),
        commands,
    )?;
    let mut preview_environment = environment.clone();
    preview_environment.insert("CHRONO_RELEASE_CHANNEL".into(), "preview".into());
    preview_environment.remove(OsStr::new("CHRONO_RELEASE_VERSION"));
    let preview = identity::resolve(
        &loaded.plan.version,
        &stable.source_commit,
        &stable.source_tree,
        &preview_environment,
    )?;
    let origin = output.join("origin");
    fs::create_dir(&origin).map_err(error)?;
    let original_publication = origin.join("publication");
    let name = artifact_name(&workflow, publisher, run_id, attempt)?;
    let publication_artifact = download(
        &config.publication.repository,
        &run,
        &name,
        &original_publication,
        None,
        commands,
    )?;
    let published = recipe::read_json(&original_publication, "receipt.json")?;
    if published["schema"] != "chrono-release-publication/v1"
        || published["status"] != "passed"
        || published["identity"] != serde_json::to_value(&preview).map_err(error)?
        || published["lineage"]
            != json!({"kind":"github","run_id":run_id,"attempt":attempt,"job":publisher})
        || published["result"]["release"]["draft"] != false
        || published["result"]["release"]["prerelease"] != true
        || published["result"]["release"]["tag_name"] != preview.version
    {
        return Err(error(
            "original publication is not the matching successful preview",
        ));
    }
    delivery::verify_publication_processes(
        &original_publication,
        &published,
        &preview,
        &config.publication.repository,
    )?;
    let mut packages = BTreeMap::new();
    let mut selections = BTreeMap::new();
    let mut package_artifacts = BTreeMap::new();
    fs::create_dir(origin.join("platforms")).map_err(error)?;
    for (native, package) in &config.packages {
        let job_id = &loaded.recipe.native_jobs[native][&loaded.recipe.collector.id];
        let selection = delivery::select(job_id, &published["dependencies"], run_id, attempt)?;
        let name = artifact_name(&workflow, job_id, run_id, &selection.attempt)?;
        let destination = origin.join("platforms").join(native);
        let artifact = download(
            &config.publication.repository,
            &run,
            &name,
            &destination,
            Some(&selection.artifact_id),
            commands,
        )?;
        packages.insert(
            native.clone(),
            Package {
                platform: package.platform.clone(),
                directory: destination
                    .strip_prefix(root)
                    .map_err(error)?
                    .to_str()
                    .ok_or_else(|| error("invalid origin path"))?
                    .into(),
            },
        );
        selections.insert(native.clone(), selection);
        package_artifacts.insert(native.clone(), artifact);
    }
    // 公开版本、完整资产、标签和原 CI 均沿现有发布者的只读判法复核。
    let asset_root = original_publication.join("release");
    let declared = published["result"]["assets"]
        .as_object()
        .ok_or_else(|| error("missing original publication assets"))?;
    let mut names = BTreeSet::from(["install.sh".to_owned(), "release.json".to_owned()]);
    for package in config.packages.values() {
        for asset in loaded.plan.assets.keys() {
            names.insert(format!("{asset}-{}", package.platform));
        }
    }
    if declared.keys().collect::<BTreeSet<_>>() != names.iter().collect() {
        return Err(error("original publication asset membership differs"));
    }
    let mut assets = BTreeMap::new();
    for (name, declaration) in declared {
        let path = no_symlink_parents(&asset_root, name)?;
        let (sha256, size) = file_identity(&path)?;
        if declaration["sha256"] != sha256 || declaration["size"] != size {
            return Err(error("original publication asset bytes differ"));
        }
        assets.insert(
            name.clone(),
            Asset {
                path: path
                    .to_str()
                    .ok_or_else(|| error("invalid original path"))?
                    .into(),
                sha256,
                size,
            },
        );
    }
    let release = recipe::read_json(&asset_root, "release.json")?;
    if release["schema"] != "chrono-release/v1"
        || release["version"] != preview.version
        || release["source_commit"] != preview.source_commit
        || release["source_tree"] != preview.source_tree
        || release["platforms"].as_object().is_none_or(|p| {
            p.keys().collect::<BTreeSet<_>>()
                != config.packages.values().map(|p| &p.platform).collect()
        })
    {
        return Err(error(
            "original public manifest identity or platforms differ",
        ));
    }
    for package in packages.values() {
        let package_release = recipe::read_json(
            &no_symlink_parents(root, &package.directory)?,
            "release.json",
        )?;
        if release["platforms"][&package.platform]
            != package_release["platforms"][&package.platform]
        {
            return Err(error(
                "original package differs from the public preview binaries",
            ));
        }
    }
    let public = publication::verify_published(&config.publication, &preview, &assets, commands)?;
    if public["release"]["id"] != published["result"]["release"]["id"] {
        return Err(error("original public release was replaced"));
    }
    let (receipt_sha256, receipt_size) = file_identity(&original_publication.join("receipt.json"))?;
    Ok(Prepared {
        identity: preview.clone(),
        packages,
        selections,
        observation: json!({
            "schema":"chrono-release-promotion/v1","identity":preview,"run":run,"workflow":workflow_identity,
            "publication_artifact":publication_artifact,"package_artifacts":package_artifacts,
            "publication_receipt":{"path":"origin/publication/receipt.json","sha256":receipt_sha256,"size":receipt_size},
            "verified_publication":public,
        }),
    })
}
