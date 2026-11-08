//! 实际 CI 选择类型、完整字段身份与规范参数。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
use super::identifiers::equal_text;

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields))]
pub enum Scope {
    Unit { unit: String },
    Collect { manifest: String },
}

pub open spec fn same_scope(first: &Scope, second: &Scope) -> bool {
    match (first, second) {
        (Scope::Unit { unit: a }, Scope::Unit { unit: b }) => a@ == b@,
        (Scope::Collect { manifest: a }, Scope::Collect { manifest: b }) => a@ == b@,
        _ => false,
    }
}

impl Clone for Scope {
    fn clone(&self) -> (out: Self)
        ensures same_scope(&out, self),
    {
        match self {
            Self::Unit { unit } => Self::Unit { unit: unit.clone() },
            Self::Collect { manifest } => Self::Collect { manifest: manifest.clone() },
        }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Scope {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { same_scope(self, other) }
}

impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == same_scope(self, other),
    {
        match (self, other) {
            (Self::Unit { unit: first }, Self::Unit { unit: second }) =>
                equal_text(first.as_str(), second.as_str()),
            (Self::Collect { manifest: first }, Self::Collect { manifest: second }) =>
                equal_text(first.as_str(), second.as_str()),
            _ => false,
        }
    }
}

impl Eq for Scope {}

impl Scope {
    pub fn short_argv(&self) -> (args: Vec<String>)
        ensures match self {
            Self::Unit { unit } => args.len() == 2
                && args@[0]@ == "--unit"@ && args@[1]@ == unit@,
            Self::Collect { .. } => args.len() == 1 && args@[0]@ == "--collect"@,
        },
    {
        let mut args = Vec::new();
        match self {
            Self::Unit { unit } => {
                args.push("--unit".to_string());
                args.push(unit.clone());
            },
            Self::Collect { .. } => { args.push("--collect".to_string()); },
        }
        args
    }

    pub fn argv(&self) -> (args: [String; 2])
        ensures match self {
            Self::Unit { unit } => args@[0]@ == "--unit"@ && args@[1]@ == unit@,
            Self::Collect { manifest } => args@[0]@ == "--collect"@ && args@[1]@ == manifest@,
        },
    {
        match self {
            Self::Unit { unit } => ["--unit".to_string(), unit.clone()],
            Self::Collect { manifest } => ["--collect".to_string(), manifest.clone()],
        }
    }
}

}
