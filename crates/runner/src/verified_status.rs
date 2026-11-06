//! 实际 CI 结果状态的身份、比较与克隆。Serde 适配仍由外部库生成。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub enum Status {
    Passed,
    Failed,
    NotRequired,
    Blocked,
}

impl Clone for Status {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        match self {
            Self::Passed => Self::Passed,
            Self::Failed => Self::Failed,
            Self::NotRequired => Self::NotRequired,
            Self::Blocked => Self::Blocked,
        }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Status {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { *self == *other }
}

impl PartialEq for Status {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (*self == *other),
    {
        match self {
            Self::Passed => matches!(other, Self::Passed),
            Self::Failed => matches!(other, Self::Failed),
            Self::NotRequired => matches!(other, Self::NotRequired),
            Self::Blocked => matches!(other, Self::Blocked),
        }
    }
}

impl Eq for Status {}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct CheckResult {
    pub id: String,
    pub status: Status,
    pub cause: String,
    pub exit_code: Option<i32>,
}

impl Clone for CheckResult {
    fn clone(&self) -> (out: Self)
        ensures out.id@ == self.id@, out.status == self.status,
            out.cause@ == self.cause@, out.exit_code == self.exit_code,
    {
        Self {
            id: self.id.clone(),
            status: self.status.clone(),
            cause: self.cause.clone(),
            exit_code: self.exit_code,
        }
    }
}

}
