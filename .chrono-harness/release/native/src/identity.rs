//! 同一次发布的版本、固定源码及原 CI 身份；不读取会漂移的分支。
use crate::{execution::Environment, verified_recipe::delivery as policy};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Channel {
    Build,
    Preview,
    Stable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedCi {
    pub run_id: u64,
    pub attempt: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub channel: Channel,
    pub version: String,
    pub source_commit: String,
    pub source_tree: String,
    pub verified_ci: Option<VerifiedCi>,
}

fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_IDENTITY: {message}")
}

fn text<'a>(environment: &'a Environment, name: &str) -> Result<&'a str, String> {
    environment
        .get(OsStr::new(name))
        .map(|v| v.to_str().ok_or_else(|| error(format!("non UTF-8 {name}"))))
        .unwrap_or(Ok(""))
}

fn positive(environment: &Environment, name: &str) -> Result<u64, String> {
    let value = text(environment, name)?;
    if !policy::decimal_text(value.as_bytes(), false) {
        return Err(error(format!("{name} must be a positive decimal identity")));
    }
    value.parse().map_err(error)
}

fn core(version: &str) -> Result<&str, String> {
    let core = version.split('-').next().unwrap_or("");
    let numbers: Vec<_> = core.strip_prefix('v').unwrap_or("").split('.').collect();
    if !policy::version_numbers(&numbers)
    {
        return Err(error(
            "release plan needs a vMAJOR.MINOR.PATCH version core",
        ));
    }
    Ok(core)
}

pub fn resolve(
    planned_version: &str,
    source_commit: &str,
    source_tree: &str,
    environment: &Environment,
) -> Result<Identity, String> {
    if !chrono_identifiers::valid_oid(source_commit.as_bytes())
        || !chrono_identifiers::valid_oid(source_tree.as_bytes())
    {
        return Err(error(
            "source observation must contain complete lowercase commit/tree OIDs",
        ));
    }
    let selected_channel = policy::channel(text(environment, "CHRONO_RELEASE_CHANNEL")?);
    let channel = match selected_channel {
        0 => Channel::Build,
        1 => Channel::Preview,
        2 => Channel::Stable,
        _ => return Err(error("unknown release channel")),
    };
    let mut identity = Identity {
        channel,
        version: planned_version.into(),
        source_commit: source_commit.into(),
        source_tree: source_tree.into(),
        verified_ci: None,
    };
    if identity.channel == Channel::Build {
        return Ok(identity);
    }
    if !policy::text_fields(&[(Some(text(environment, "CHRONO_RELEASE_SOURCE")?), source_commit)]) {
        return Err(error(
            "requested source differs from the observed fixed commit",
        ));
    }
    let ci = VerifiedCi {
        run_id: positive(environment, "CHRONO_RELEASE_CI_RUN_ID")?,
        attempt: positive(environment, "CHRONO_RELEASE_CI_ATTEMPT")?,
    };
    let version_core = core(planned_version)?;
    let requested_version = text(environment, "CHRONO_RELEASE_VERSION")?;
    match identity.channel {
        Channel::Preview => {
            if !policy::requested_version(selected_channel, requested_version, version_core, planned_version) {
                return Err(error(
                    "preview version is derived from its CI and source identities",
                ));
            }
            identity.version = format!(
                "{version_core}-dev.{}.{}.g{source_commit}",
                ci.run_id, ci.attempt
            );
        }
        Channel::Stable => {
            if !policy::requested_version(selected_channel, requested_version, version_core, planned_version) {
                return Err(error(
                    "stable delivery needs an explicit final version matching the fixed plan",
                ));
            }
        }
        Channel::Build => unreachable!("build returned before publication bindings"),
    }
    identity.verified_ci = Some(ci);
    Ok(identity)
}
