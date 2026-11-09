//! GitHub 发布状态机。命令边界可替换；同一版本只续传相同资产。
use crate::identity::{Channel, Identity};
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
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
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
    if reply.exit == 0 {
        return Ok(Some(value));
    }
    if absent && reply.exit == 1 && value["status"] == "404" && value["message"] == "Not Found" {
        return Ok(None);
    }
    Err(error(format!(
        "original GitHub API exited {}: {}",
        reply.exit, args[3]
    )))
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
    if run["id"] != ci.run_id
        || run["run_attempt"] != ci.attempt
        || run["workflow_id"] != config.workflow_id
        || run["path"] != config.workflow_path
        || run["event"] != "push"
        || run["head_branch"] != config.branch
        || run["head_sha"] != identity.source_commit
        || run["status"] != "completed"
        || run["conclusion"] != "success"
        || run["repository"]["full_name"] != config.repository
    {
        return Err(error(
            "original landing CI does not qualify this fixed source",
        ));
    }
    Ok(run)
}

fn release_header(release: &Value, identity: &Identity) -> Result<u64, String> {
    let preview = identity.channel == Channel::Preview;
    if release["tag_name"] != identity.version
        || release["target_commitish"] != identity.source_commit
        || release["prerelease"] != preview
        || !release["draft"].is_boolean()
    {
        return Err(error("existing release has another source, tag or channel"));
    }
    release["id"]
        .as_u64()
        .filter(|v| *v > 0)
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
            || asset["state"] != "uploaded"
            || asset["size"] != expected.size
            || asset["digest"] != format!("sha256:{}", expected.sha256)
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
    if release["draft"] == false && !missing.is_empty() {
        return Err(error(
            "published version has missing assets; cannot repair a public version",
        ));
    }
    if release["draft"] == true {
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
        || release["draft"] != false
        || observed["draft"] != false
        || !remote_assets(config, id, assets, commands)?.is_empty()
    {
        return Err(error("public release verification failed"));
    }
    verify_tag(config, identity, commands, false)?;
    Ok(json!({"release":observed,"verified_ci":ci,"assets":assets}))
}
