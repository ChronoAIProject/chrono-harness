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

/// 只观察捕获的快照是否包含完整路径，不选择目标或读取现役执行物。
pub trait RegistryMembership {
    spec fn has_path(&self, path: &str) -> bool;

    fn contains_path(&self, path: &str) -> (present: bool)
        ensures present == self.has_path(path);
}

pub open spec fn unique_registry_index<M: RegistryMembership>(
    paths: Seq<&str>, values: &M, index: int,
) -> bool {
    0 <= index < paths.len() && values.has_path(paths[index])
        && forall|j: int| 0 <= j < paths.len() && j != index ==> !values.has_path(#[trigger] paths[j])
}

/// 从原登记路径中借出快照唯一包含的一项；缺失或歧义保持原错误。
pub fn select_captured_target<'a, M: RegistryMembership>(paths: &[&'a str], values: &M)
    -> (out: Result<&'a str, &'static str>)
    ensures match out {
        Ok(path) => exists|i: int| #[trigger] unique_registry_index(paths@, values, i)
            && paths@[i]@ == path@,
        Err(message) => message@ == "registry values require exactly one registered effective policy"@
            && forall|i: int| !#[trigger] unique_registry_index(paths@, values, i),
    },
{
    let message = "registry values require exactly one registered effective policy";
    let mut selected: Option<usize> = None;
    let mut i = 0;
    while i < paths.len()
        invariant i <= paths.len(),
            message@ == "registry values require exactly one registered effective policy"@,
            match selected { Some(s) => s < i, None => true },
            forall|j: int| 0 <= j < i ==>
                (values.has_path(#[trigger] paths@[j]) == (selected == Some(j as usize))),
        decreases paths.len() - i,
    {
        if values.contains_path(paths[i]) {
            if let Some(first) = selected {
                assert(values.has_path(paths@[first as int]));
                assert forall|j: int| !#[trigger] unique_registry_index(paths@, values, j) by {
                    if j == i {
                        assert(first < j && values.has_path(paths@[first as int]));
                    } else {
                        assert(values.has_path(paths@[i as int]));
                    }
                }
                return Err(message);
            }
            selected = Some(i);
        }
        i += 1;
    }
    match selected {
        Some(index) => {
            assert(unique_registry_index(paths@, values, index as int));
            Ok(paths[index])
        },
        None => {
            assert forall|j: int| !#[trigger] unique_registry_index(paths@, values, j) by {
                if 0 <= j < paths.len() { assert(!values.has_path(paths@[j])); }
            }
            Err(message)
        },
    }
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
