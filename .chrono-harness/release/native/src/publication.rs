//! GitHub 发布状态机。命令边界可替换；同一版本只续传相同资产。
use crate::{
    identity::{Channel, Identity},
    verified_recipe::delivery as policy,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub repository: String,
    pub workflow_id: u64,
    pub workflow_path: String,
    pub branch: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

pub struct Reply {
    pub exit: i32,
    pub stdout: Vec<u8>,
}

/// 生产实现保留 argv、原流及退出；测试只在这个执行边界模拟 gh。
pub trait Commands {
    fn execute(&mut self, program: &str, args: &[String]) -> Result<Reply, String>;
    fn body(&mut self, value: &Value) -> Result<String, String>;
}

fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_PUBLICATION: {message}")
}

fn name(s: &str) -> bool {
    policy::name(s.as_bytes())
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        let parts: Vec<_> = self.repository.split('/').collect();
        if parts.len() != 2
            || !parts.into_iter().all(name)
            || self.workflow_id == 0
            || !self.workflow_path.starts_with(".github/workflows/")
            || !self.workflow_path.ends_with(".yml")
            || self.branch.is_empty()
        {
            return Err(error("invalid repository or landing workflow declaration"));
        }
        chrono_harness::relative_path(&self.workflow_path)?;
        Ok(())
    }
}

fn api(
    commands: &mut impl Commands,
    method: &str,
    endpoint: String,
    body: Option<Value>,
    absent: bool,
) -> Result<Option<Value>, String> {
    let mut args = vec!["api".into(), "--method".into(), method.into(), endpoint];
    if let Some(body) = body {
        args.extend(["--input".into(), commands.body(&body)?]);
    }
    let reply = commands.execute("gh", &args)?;
    let value: Value = chrono_harness::json(&reply.stdout)?;
    match policy::api_outcome(
        reply.exit,
        value["status"].as_str(),
        value["message"].as_str(),
        absent,
    ) {
        0 => return Ok(Some(value)),
        1 => return Ok(None),
        _ => {}
    }
    Err(error(format!(
        "original GitHub API exited {}: {}",
        reply.exit, args[3]
    )))
}

/// 安装消费者沿用发布 API 的缺失判法，只接受当前仓库、版本的两个 GET 查询。
pub(crate) fn original_absence(
    process: &Value,
    stdout: &[u8],
    repository: &str,
    identity: &Identity,
) -> bool {
    if process["exit_code"] != 1 || identity.channel == Channel::Build || process["tool"] != "gh" {
        return false;
    }
    let Some(argv) = process["argv"].as_array() else {
        return false;
    };
    if argv.len() != 5 || argv[1] != "api" || argv[2] != "--method" || argv[3] != "GET" {
        return false;
    }
    let Some(executable) = argv[0].as_str() else {
        return false;
    };
    if !std::path::Path::new(executable).is_absolute()
        || process["observation"]["executable"] != executable
    {
        return false;
    }
    let endpoints = [
        format!("repos/{repository}/releases/tags/{}", identity.version),
        format!("repos/{repository}/git/ref/tags/{}", identity.version),
    ];
    if !endpoints
        .iter()
        .any(|endpoint| argv[4] == endpoint.as_str())
    {
        return false;
    }
    let Ok(value) = chrono_harness::json(stdout) else {
        return false;
    };
    policy::api_outcome(1, value["status"].as_str(), value["message"].as_str(), true) == 1
}

pub fn verify_ci(
    config: &Config,
    identity: &Identity,
    commands: &mut impl Commands,
) -> Result<Value, String> {
    config.validate()?;
    let ci = identity
        .verified_ci
        .as_ref()
        .ok_or_else(|| error("publication needs original CI identity"))?;
    let run = api(
        commands,
        "GET",
        format!(
            "repos/{}/actions/runs/{}/attempts/{}",
            config.repository, ci.run_id, ci.attempt
        ),
        None,
        false,
    )?
    .ok_or("missing CI run")?;
    if !policy::number_fields(&[
        (run["id"].as_u64(), ci.run_id),
        (run["run_attempt"].as_u64(), ci.attempt),
        (run["workflow_id"].as_u64(), config.workflow_id),
    ]) || !policy::text_fields(&[
        (run["path"].as_str(), &config.workflow_path),
        (run["event"].as_str(), "push"),
        (run["head_branch"].as_str(), &config.branch),
        (run["head_sha"].as_str(), &identity.source_commit),
        (run["status"].as_str(), "completed"),
        (run["conclusion"].as_str(), "success"),
        (run["repository"]["full_name"].as_str(), &config.repository),
    ]) {
        return Err(error(
            "original landing CI does not qualify this fixed source",
        ));
    }
    Ok(run)
}

fn release_header(release: &Value, identity: &Identity) -> Result<u64, String> {
    let preview = identity.channel == Channel::Preview;
    if !policy::text_fields(&[
        (release["tag_name"].as_str(), &identity.version),
        (
            release["target_commitish"].as_str(),
            &identity.source_commit,
        ),
    ]) || !policy::boolean_fields(&[(release["prerelease"].as_bool(), preview)])
        || policy::draft_state(release["draft"].as_bool(), 0) == 2
    {
        return Err(error("existing release has another source, tag or channel"));
    }
    release["id"]
        .as_u64()
        .filter(|v| policy::positive(Some(*v)))
        .ok_or_else(|| error("missing release ID"))
}

fn remote_assets(
    config: &Config,
    id: u64,
    assets: &BTreeMap<String, Asset>,
    commands: &mut impl Commands,
) -> Result<Vec<String>, String> {
    let observed = api(
        commands,
        "GET",
        format!(
            "repos/{}/releases/{id}/assets?per_page=100",
            config.repository
        ),
        None,
        false,
    )?
    .ok_or("missing assets")?;
    let mut remaining = assets
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for asset in observed
        .as_array()
        .ok_or_else(|| error("invalid remote assets"))?
    {
        let name = asset["name"]
            .as_str()
            .ok_or_else(|| error("missing remote asset name"))?;
        let expected = assets
            .get(name)
            .ok_or_else(|| error(format!("unexpected remote asset {name}")))?;
        if !remaining.remove(name)
            || !policy::text_fields(&[
                (asset["state"].as_str(), "uploaded"),
                (
                    asset["digest"].as_str(),
                    &format!("sha256:{}", expected.sha256),
                ),
            ])
            || !policy::number_fields(&[(asset["size"].as_u64(), expected.size)])
        {
            return Err(error(format!("remote asset identity conflicts: {name}")));
        }
    }
    Ok(remaining.into_iter().collect())
}

fn verify_tag(
    config: &Config,
    identity: &Identity,
    commands: &mut impl Commands,
    allow_absent: bool,
) -> Result<(), String> {
    let reference = api(
        commands,
        "GET",
        format!(
            "repos/{}/git/ref/tags/{}",
            config.repository, identity.version
        ),
        None,
        allow_absent,
    )?;
    let Some(reference) = reference else {
        return Ok(());
    };
    let mut object = reference["object"].clone();
    for _ in 0..4 {
        let oid = object["sha"]
            .as_str()
            .filter(|s| chrono_identifiers::valid_oid(s.as_bytes()))
            .ok_or_else(|| error("invalid tag object identity"))?;
        match object["type"].as_str() {
            Some("commit") if oid == identity.source_commit => return Ok(()),
            Some("tag") => {
                object = api(
                    commands,
                    "GET",
                    format!("repos/{}/git/tags/{oid}", config.repository),
                    None,
                    false,
                )?
                .ok_or("missing tag object")?["object"]
                    .clone();
            }
            _ => return Err(error("release tag points at another source")),
        }
    }
    Err(error("release tag nesting exceeds supported bound"))
}

pub fn publish(
    config: &Config,
    identity: &Identity,
    assets: &BTreeMap<String, Asset>,
    commands: &mut impl Commands,
) -> Result<Value, String> {
    config.validate()?;
    if identity.channel == Channel::Build
        || !name(&identity.version)
        || assets.is_empty()
        || assets.len() >= 100
    {
        return Err(error(
            "publication requires a delivery channel and bounded explicit assets",
        ));
    }
    for (name, asset) in assets {
        if !self::name(name)
            || !chrono_identifiers::valid_digest_text(&asset.sha256)
            || asset.size == 0
        {
            return Err(error("invalid explicit asset identity"));
        }
    }
    let ci = verify_ci(config, identity, commands)?;
    let endpoint = format!(
        "repos/{}/releases/tags/{}",
        config.repository, identity.version
    );
    let mut release = match api(commands, "GET", endpoint.clone(), None, true)? {
        Some(release) => release,
        None => {
            verify_tag(config, identity, commands, true)?;
            api(commands,"POST",format!("repos/{}/releases",config.repository),Some(json!({
                "tag_name":identity.version,"target_commitish":identity.source_commit,
                "name":identity.version,"body":format!("Source: {}\nLanding CI: {} attempt {}",identity.source_commit,ci["id"],ci["run_attempt"]),
                "draft":true,"prerelease":identity.channel == Channel::Preview,"make_latest":"false"
            })),false)?.ok_or("missing created draft")?
        }
    };
    let id = release_header(&release, identity)?;
    let missing = remote_assets(config, id, assets, commands)?;
    let action = policy::draft_state(release["draft"].as_bool(), missing.len());
    if action == 2 {
        return Err(error(
            "published version has missing assets; cannot repair a public version",
        ));
    }
    if action == 1 {
        for name in missing {
            let reply = commands.execute(
                "gh",
                &[
                    "release".into(),
                    "upload".into(),
                    identity.version.clone(),
                    assets[&name].path.clone(),
                    "--repo".into(),
                    config.repository.clone(),
                ],
            )?;
            if reply.exit != 0 {
                return Err(error(format!(
                    "original asset upload exited {}: {name}",
                    reply.exit
                )));
            }
        }
        if !remote_assets(config, id, assets, commands)?.is_empty() {
            return Err(error("draft upload incomplete"));
        }
        verify_tag(config, identity, commands, true)?;
        release = api(commands,"PATCH",format!("repos/{}/releases/{id}",config.repository),Some(json!({"draft":false,"make_latest":if identity.channel == Channel::Stable {"true"} else {"false"}})),false)?.ok_or("missing published release")?;
    }
    // 重新读取实际公开状态；PATCH 成功不能代替落地核验。
    let observed = api(commands, "GET", endpoint, None, false)?.ok_or("missing public release")?;
    if release_header(&observed, identity)? != id
        || !policy::boolean_fields(&[
            (release["draft"].as_bool(), false),
            (observed["draft"].as_bool(), false),
        ])
        || !remote_assets(config, id, assets, commands)?.is_empty()
    {
        return Err(error("public release verification failed"));
    }
    verify_tag(config, identity, commands, false)?;
    Ok(json!({"release":observed,"verified_ci":ci,"assets":assets}))
}
