//! Git 事实读取的声明与可选预期；实际文件／进程观察归适配器。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

verus! {

pub enum GitConfigSchema {
    Direct,
    V1,
    V2,
    V3,
    Unsupported,
}

impl GitConfigSchema {
    pub fn is_selector(&self) -> (valid: bool)
        ensures valid == (self is V1 || self is V2 || self is V3),
    {
        matches!(self, Self::V1 | Self::V2 | Self::V3)
    }

    pub fn has_identity(&self) -> (valid: bool)
        ensures valid == (self is V2 || self is V3),
    {
        matches!(self, Self::V2 | Self::V3)
    }

    pub fn has_registry_path(&self) -> (valid: bool)
        ensures valid == (self is V3),
    {
        matches!(self, Self::V3)
    }
}

pub open spec fn git_config_schema(schema: Option<&str>) -> GitConfigSchema {
    match schema {
        None => GitConfigSchema::Direct,
        Some(text) => {
            let bytes = text.spec_bytes();
            let prefix = "chrono-git-configs/".spec_bytes();
            if text@ == "chrono-git-configs/v1"@ { GitConfigSchema::V1 }
            else if text@ == "chrono-git-configs/v2"@ { GitConfigSchema::V2 }
            else if text@ == "chrono-git-configs/v3"@ { GitConfigSchema::V3 }
            else if bytes.len() >= prefix.len() && bytes.take(prefix.len() as int) == prefix {
                GitConfigSchema::Unsupported
            } else { GitConfigSchema::Direct }
        },
    }
}

/// 完整版本先于家族前缀；缺失或非文本观察使用直接配置分支。
pub fn classify_git_config_schema(schema: Option<&str>) -> (out: GitConfigSchema)
    ensures out == git_config_schema(schema),
{
    let Some(text) = schema else { return GitConfigSchema::Direct; };
    let bytes = text.as_bytes();
    let prefix = "chrono-git-configs/".as_bytes();
    if identifiers::equal_text(text, "chrono-git-configs/v1") { GitConfigSchema::V1 }
    else if identifiers::equal_text(text, "chrono-git-configs/v2") { GitConfigSchema::V2 }
    else if identifiers::equal_text(text, "chrono-git-configs/v3") { GitConfigSchema::V3 }
    else if bytes.len() >= prefix.len() && identifiers::equal_bytes(&bytes[0..prefix.len()], prefix) {
        GitConfigSchema::Unsupported
    } else { GitConfigSchema::Direct }
}

/// 选择器只能指向直接 v3／v4；schema 键即使为 null 也算存在。
pub fn admit_git_selector_target(version: Option<u64>, schema_present: bool)
    -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => !schema_present && (version == Some(3u64) || version == Some(4u64)),
        Err(message) => (schema_present || (version != Some(3u64) && version != Some(4u64)))
            && message@ == "Git config selector requires a direct full-v3 policy or v4 policy, never another selector"@,
    },
{
    if !schema_present && matches!(version, Some(3 | 4)) { Ok(()) }
    else { Err("Git config selector requires a direct full-v3 policy or v4 policy, never another selector") }
}

pub fn valid_link(schema: u64, tool: bool, input: bool, unknown: bool) -> (valid: bool)
    ensures valid <==> ((schema == 3 || schema == 4) && tool && !unknown
        && (schema == 4 || input)),
{
    (schema == 3 || schema == 4) && tool && !unknown && (schema == 4 || input)
}

pub fn valid_version(schema: u64, present: bool, null: bool, text: Option<&str>) -> (valid: bool)
    ensures valid <==> (present && match text {
        Some(value) => value@.len() > 0,
        None => schema == 4 && null,
    }),
{
    present && match text {
        Some(value) => !value.is_empty(),
        None => schema == 4 && null,
    }
}

/// 无预期时保留真实观察；给出预期时必须完整相等。
pub fn matches_expected(actual: &str, expected: Option<&str>) -> (valid: bool)
    ensures valid <==> match expected {
        None => true,
        Some(value) => actual@ == value@,
    },
{
    match expected {
        None => true,
        Some(value) => identifiers::equal_text(actual, value),
    }
}

}
