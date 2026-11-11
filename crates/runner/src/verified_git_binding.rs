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

pub open spec fn required_text_matches(observed: Option<&str>, expected: &str) -> bool {
    match observed {
        Some(text) => text@ == expected@,
        None => false,
    }
}

fn matches_required(observed: Option<&str>, expected: &str) -> (valid: bool)
    ensures valid == required_text_matches(observed, expected),
{
    match observed {
        Some(text) => identifiers::equal_text(text, expected),
        None => false,
    }
}

/// 目标策略必须声明执行物；缺声明先于任一字段不匹配。
pub fn admit_selector_binding(
    program: &str,
    digest: &str,
    tool: Option<&str>,
    input: Option<(Option<&str>, Option<&str>)>,
) -> (out: Result<(), &'static str>)
    ensures match input {
        None => out == Err::<(), &'static str>("Git identity selector requires a declared executable input"),
        Some((location, sha256)) => {
            if required_text_matches(tool, program) && required_text_matches(location, program)
                && required_text_matches(sha256, digest)
            {
                out == Ok::<(), &'static str>(())
            } else {
                out == Err::<(), &'static str>("Git config selector identity differs from selected policy binding")
            }
        },
    },
{
    let Some((location, sha256)) = input else {
        return Err("Git identity selector requires a declared executable input");
    };
    if matches_required(tool, program) && matches_required(location, program)
        && matches_required(sha256, digest)
    {
        Ok(())
    } else {
        Err("Git config selector identity differs from selected policy binding")
    }
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

pub enum GitTargetField { Digest, Path }

/// 只观察原数组长度与指定槽位的完整字符串，不在接口内选择目标。
pub trait GitTargetRows {
    spec fn count(&self) -> nat;
    spec fn text_at(&self, index: int, field: GitTargetField) -> Option<&str>;

    fn len(&self) -> (out: usize)
        ensures out == self.count();

    fn text(&self, index: usize, field: &GitTargetField) -> (out: Option<&str>)
        requires index < self.count(),
        ensures out == self.text_at(index as int, *field);
}

pub open spec fn target_matches<R: GitTargetRows>(
    rows: &R, index: int, field: GitTargetField, expected: Option<&str>,
) -> bool {
    match (rows.text_at(index, field), expected) {
        (Some(actual), Some(expected)) => actual@ == expected@,
        _ => false,
    }
}

pub open spec fn first_target<R: GitTargetRows>(
    rows: &R, field: GitTargetField, expected: Option<&str>, out: Option<usize>,
) -> bool {
    match out {
        Some(i) => i < rows.count() && target_matches(rows, i as int, field, expected)
            && forall|j: int| 0 <= j < i ==> !target_matches(rows, j, field, expected),
        None => forall|j: int| 0 <= j < rows.count() ==> !target_matches(rows, j, field, expected),
    }
}

fn find_target<R: GitTargetRows>(rows: &R, field: GitTargetField, expected: Option<&str>)
    -> (out: Option<usize>)
    ensures first_target(rows, field, expected, out),
{
    let Some(text) = expected else { return None; };
    let len = rows.len();
    let mut i = 0;
    while i < len
        invariant
            i <= len, len == rows.count(), expected == Some(text),
            forall|j: int| 0 <= j < i ==> !target_matches(rows, j, field, expected),
        decreases len - i,
    {
        if let Some(actual) = rows.text(i, &field) {
            if identifiers::equal_text(actual, text) { return Some(i); }
        }
        assert(!target_matches(rows, i as int, field, expected));
        i += 1;
    }
    None
}

pub open spec fn preferred_target<R: GitTargetRows>(
    rows: &R, digest: Option<&str>, path: Option<&str>, out: Option<usize>,
) -> bool {
    (out.is_some() && first_target(rows, GitTargetField::Digest, digest, out))
    || (first_target(rows, GitTargetField::Digest, digest, None)
        && first_target(rows, GitTargetField::Path, path, out))
}

/// 优先选摘要首个匹配；只有摘要无匹配才按已给出的登记路径选首项。
pub fn select_git_target<R: GitTargetRows>(rows: &R, digest: Option<&str>, path: Option<&str>)
    -> (out: Option<usize>)
    ensures preferred_target(rows, digest, path, out),
{
    match find_target(rows, GitTargetField::Digest, digest) {
        Some(index) => Some(index),
        None => find_target(rows, GitTargetField::Path, path),
    }
}

}
