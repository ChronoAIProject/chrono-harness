//! 实际 CI／协议结果状态及 finding 一致性。Serde 适配仍由外部库生成。
use core::cmp::Ordering;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "verified_identifiers.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub enum Status {
    Passed,
    Failed,
    NotRequired,
    Blocked,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "lowercase"))]
pub enum WireStatus {
    Pass,
    Warn,
    Fail,
    Error,
}

impl WireStatus {
    pub open spec fn rank_spec(&self) -> u8 {
        match self { Self::Pass => 0, Self::Warn => 1, Self::Fail => 2, Self::Error => 3 }
    }

    fn rank(&self) -> (rank: u8)
        ensures rank == self.rank_spec(),
    {
        match self { Self::Pass => 0, Self::Warn => 1, Self::Fail => 2, Self::Error => 3 }
    }

    pub open spec fn order_spec(&self, other: &Self) -> Ordering {
        if self.rank_spec() < other.rank_spec() { Ordering::Less }
        else if self.rank_spec() > other.rank_spec() { Ordering::Greater }
        else { Ordering::Equal }
    }

    pub open spec fn exit_spec(&self) -> i32 {
        match self { Self::Pass | Self::Warn => 0, Self::Fail => 1, Self::Error => 2 }
    }

    pub fn exit_code(&self) -> (exit: i32)
        ensures exit == self.exit_spec(),
    {
        match self { Self::Pass | Self::Warn => 0, Self::Fail => 1, Self::Error => 2 }
    }
}

pub proof fn wire_status_order_laws(a: WireStatus, b: WireStatus, c: WireStatus)
    ensures
        (a.rank_spec() == b.rank_spec()) == (a == b),
        (a.order_spec(&b) == Ordering::Equal) == (a == b),
        (a.order_spec(&b) == Ordering::Less) == (b.order_spec(&a) == Ordering::Greater),
        a.order_spec(&b) == Ordering::Less && b.order_spec(&c) == Ordering::Less
            ==> a.order_spec(&c) == Ordering::Less,
{}

impl Clone for WireStatus {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        match self { Self::Pass => Self::Pass, Self::Warn => Self::Warn,
            Self::Fail => Self::Fail, Self::Error => Self::Error }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for WireStatus {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { *self == *other }
}

impl PartialEq for WireStatus {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (*self == *other),
    { self.rank() == other.rank() }
}

impl Eq for WireStatus {}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::OrdSpecImpl for WireStatus {
    open spec fn obeys_cmp_spec() -> bool { true }
    open spec fn cmp_spec(&self, other: &Self) -> Ordering { self.order_spec(other) }
}

impl Ord for WireStatus {
    fn cmp(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        let rank = self.rank();
        let other_rank = other.rank();
        if rank < other_rank { Ordering::Less }
        else if rank > other_rank { Ordering::Greater }
        else { Ordering::Equal }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialOrdSpecImpl for WireStatus {
    open spec fn obeys_partial_cmp_spec() -> bool { true }
    open spec fn partial_cmp_spec(&self, other: &Self) -> Option<Ordering> {
        Some(self.order_spec(other))
    }
}

impl PartialOrd for WireStatus {
    fn partial_cmp(&self, other: &Self) -> (order: Option<Ordering>)
        ensures order == Some(self.order_spec(other)),
    { Some(self.cmp(other)) }
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Finding {
    pub code: String,
    pub level: String,
    pub message: String,
    pub delta_refs: Vec<String>,
    pub causes: Vec<String>,
}

impl Clone for Finding {
    fn clone(&self) -> (out: Self)
        ensures out.code == self.code, out.level == self.level, out.message == self.message,
            out.delta_refs@ == self.delta_refs@, out.causes@ == self.causes@,
    {
        Self { code: self.code.clone(), level: self.level.clone(), message: self.message.clone(),
            delta_refs: self.delta_refs.clone(), causes: self.causes.clone() }
    }
}

pub open spec fn finding_level_present(findings: Seq<Finding>, level: Seq<char>) -> bool {
    exists|i: int| 0 <= i < findings.len() && (#[trigger] findings[i]).level@ == level
}

pub open spec fn finding_status_agreement(status: &WireStatus, findings: Seq<Finding>) -> bool {
    let errors = finding_level_present(findings, "error"@);
    let warnings = finding_level_present(findings, "warning"@);
    (matches!(status, WireStatus::Fail | WireStatus::Error) == errors)
        && (*status == WireStatus::Pass ==> !warnings)
        && (*status == WireStatus::Warn ==> warnings)
}

/// 直接检查真实 finding；字段有效性和响应身份另由各自所有者检查。
pub fn findings_match_status(status: &WireStatus, findings: &[Finding]) -> (matches: bool)
    ensures matches == finding_status_agreement(status, findings@),
{
    let mut at = 0;
    let mut errors = false;
    let mut warnings = false;
    while at < findings.len()
        invariant at <= findings.len(),
            errors == finding_level_present(findings@.take(at as int), "error"@),
            warnings == finding_level_present(findings@.take(at as int), "warning"@),
        decreases findings.len() - at,
    {
        let level = findings[at].level.as_str();
        errors = errors || identifiers::equal_text(level, "error");
        warnings = warnings || identifiers::equal_text(level, "warning");
        proof {
            assert forall|i: int| 0 <= i < at + 1 implies
                (#[trigger] findings@.take(at + 1)[i]).level@ == findings@[i].level@ by { }
        }
        at += 1;
    }
    assert(findings@.take(at as int) =~= findings@);
    match status {
        WireStatus::Pass => !errors && !warnings,
        WireStatus::Warn => !errors && warnings,
        WireStatus::Fail | WireStatus::Error => errors,
    }
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
