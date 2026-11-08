//! 生命周期选择直接供清理所有者调用；文件存在性与租约由外部接口提供。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_identifiers.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
use identifiers::equal_text;

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub enum CleanupStage {
    Pending,
    InProgress,
    WaitingForConsumers,
    Retained,
    Failed,
    CachesReleased,
    Disposed,
}

pub fn cleanup_result(cache: bool, status: &str, removal: &str) -> (stage: CleanupStage)
    ensures
        status@ != "cleaned"@ ==> stage == CleanupStage::Failed,
        status@ == "cleaned"@ && cache ==> stage == CleanupStage::CachesReleased,
        status@ == "cleaned"@ && !cache ==>
            (stage == CleanupStage::Disposed <==>
                removal@ == "verified-absent"@ || removal@ == "already-absent"@),
        status@ == "cleaned"@ && !cache
            && removal@ != "verified-absent"@ && removal@ != "already-absent"@
            ==> stage == CleanupStage::Retained,
{
    if !equal_text(status, "cleaned") { CleanupStage::Failed }
    else if cache { CleanupStage::CachesReleased }
    else if equal_text(removal, "verified-absent") || equal_text(removal, "already-absent") {
        CleanupStage::Disposed
    } else { CleanupStage::Retained }
}

pub fn select_cleanup(status: &str, cache_pending: bool, maintenance: bool, artifacts_only: bool) -> (selected: bool)
    ensures selected <==> (status@ == "terminal"@
        || (status@ == "active"@ && cache_pending)
        || (status@ == "retained"@ && maintenance && !artifacts_only)),
{
    equal_text(status, "terminal")
        || (equal_text(status, "active") && cache_pending)
        || (equal_text(status, "retained") && maintenance && !artifacts_only)
}

pub fn retain_evidence(disposal_authorized: bool, evidence_present: bool) -> (retain: bool)
    ensures retain <==> (!disposal_authorized && evidence_present),
{
    !disposal_authorized && evidence_present
}

pub fn accepts_enrollment_branch(kind: &str, registered: bool) -> (accepted: bool)
    ensures accepted <==> (kind@ == "import"@
        || (kind@ == "birth"@ && registered)),
{
    equal_text(kind, "import") || (equal_text(kind, "birth") && registered)
}

pub fn accepts_cleanup_branch(kind: &str, registered: bool) -> (accepted: bool)
    ensures accepted <==> (kind@ == "import"@ || registered),
{
    equal_text(kind, "import") || registered
}

pub fn pending_for_agent(creator: &str, current: &str, disposed: bool) -> (pending: bool)
    ensures pending <==> (creator@ == current@ && creator@.len() > 0 && !disposed),
{
    equal_text(creator, current) && !equal_text(creator, "") && !disposed
}

pub fn retire_history(disposed: bool, consumers_released: bool,
    confirmed_at: Option<u64>, observed: u64, keep_seconds: u64, over_limit: bool) -> (retire: bool)
    ensures retire <==> (disposed && consumers_released
        && confirmed_at.is_some()
        && confirmed_at.unwrap() <= observed
        && (over_limit || observed - confirmed_at.unwrap() >= keep_seconds)),
{
    match confirmed_at {
        Some(confirmed) => disposed && consumers_released
            && confirmed <= observed && (over_limit || observed - confirmed >= keep_seconds),
        None => false,
    }
}

}
