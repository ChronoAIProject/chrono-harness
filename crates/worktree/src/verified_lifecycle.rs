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

pub fn rolling_disposal(owned: bool, expired: bool, released: bool, exclusive: bool) -> (allowed: bool)
    ensures allowed <==> (owned && expired && released && exclusive),
{
    owned && expired && released && exclusive
}

pub fn rolling_keep(finished: bool, failed: bool, diagnostics: bool,
    success: u64, failure: u64, interrupted: u64) -> (seconds: u64)
    ensures
        !finished ==> seconds == interrupted,
        finished && (failed || diagnostics) ==> seconds == failure,
        finished && !failed && !diagnostics ==> seconds == success,
{
    if !finished { interrupted } else if failed || diagnostics { failure } else { success }
}

}
