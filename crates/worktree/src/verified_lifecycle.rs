//! 生命周期选择直接供清理所有者调用；文件存在性与租约由外部接口提供。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
use identifiers::equal_text;

#[cfg(verus_keep_ghost)]
#[path = "start.rs"]
mod start_verification;
#[cfg(not(verus_keep_ghost))]
pub use crate::start::select_report_input_scope;
#[cfg(verus_keep_ghost)]
pub use start_verification::select_report_input_scope;

#[cfg(verus_keep_ghost)]
#[path = "automatic.rs"]
// The ordinary automatic module owns formatting of its runtime child modules.
// This proof-only alias has a different module directory and must not traverse them.
#[rustfmt::skip]
mod automatic_verification;
#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::{
    MissingCacheObservation, absence_probe_required, absent_attachment_conflict,
    known_absent_enrollment, missing_cache_observation,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::{
    MissingCacheObservation, absence_probe_required, absent_attachment_conflict,
    known_absent_enrollment, missing_cache_observation,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::inventory::append_action_index;
#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::service::maintenance_idle_exit;
#[cfg(verus_keep_ghost)]
pub use automatic_verification::inventory::append_action_index;
#[cfg(verus_keep_ghost)]
pub use automatic_verification::service::maintenance_idle_exit;

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::producer::rolling::{rolling_disposal, rolling_keep};
#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::producer::{
    ProducerNamespaceMember, producer_namespace_member, producer_namespace_retired,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::producer::rolling::{rolling_disposal, rolling_keep};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::producer::{
    ProducerNamespaceMember, producer_namespace_member, producer_namespace_retired,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::retirement::{retire_cache_history, retire_history};
#[cfg(not(verus_keep_ghost))]
pub use crate::recovery::{ImportedPolicyReceipt, imported_policy_receipt_valid};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::retirement::{retire_cache_history, retire_history};
#[cfg(verus_keep_ghost)]
#[path = "recovery.rs"]
#[rustfmt::skip]
mod recovery_verification;
#[cfg(verus_keep_ghost)]
pub use recovery_verification::{
    ImportedPolicyReceipt, imported_policy_receipt_valid, required_text,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::successful_report_status;
#[cfg(verus_keep_ghost)]
#[path = "lib.rs"]
#[rustfmt::skip]
mod cli_status_owner;
#[cfg(verus_keep_ghost)]
pub use cli_status_owner::successful_report_status;

verus! {

/// An address left by failed admission has no checkout policy consumer.
pub fn enrolled_policy_required(entries: usize) -> (required: bool)
    ensures required <==> entries > 0,
{
    entries > 0
}

#[cfg_attr(not(verus_keep_ghost), derive(Clone, Debug, PartialEq, Eq, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub enum Disposition {
    Dispose,
    FinishDispose,
    Retain,
    EvidenceRetain,
}

pub fn select_artifact(disposition: &Disposition, release_reusable: bool,
    release_evidence: bool) -> (selected: bool)
    ensures selected <==> match *disposition {
        Disposition::Dispose => true,
        Disposition::FinishDispose => release_reusable,
        Disposition::Retain => false,
        Disposition::EvidenceRetain => release_evidence,
    },
{
    match disposition {
        Disposition::Dispose => true,
        Disposition::FinishDispose => release_reusable,
        Disposition::Retain => false,
        Disposition::EvidenceRetain => release_evidence,
    }
}

pub fn reusable_release(terminal_handoff: bool, generation: u64,
    requested_generation: Option<u64>) -> (release: bool)
    ensures release <==> (terminal_handoff || (requested_generation.is_some()
        && requested_generation.unwrap() == generation)),
{
    terminal_handoff || match requested_generation {
        Some(requested) => requested == generation,
        None => false,
    }
}

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

// The caller holds the original exclusive lease. Cleanup observations are
// diagnostic history; actual unjoined work still prevents checkout adoption.
pub fn checkout_migration_ready(status: &str, terminal: bool, uses_pending: bool,
    handoff_pending: bool) -> (ready: bool)
    ensures ready <==> (status@ == "active"@ && !terminal
        && !uses_pending && !handoff_pending),
{
    equal_text(status, "active") && !terminal && !uses_pending && !handoff_pending
}

pub fn accepts_cleanup_branch(kind: &str, registered: bool) -> (accepted: bool)
    ensures accepted <==> (kind@ == "import"@ || registered),
{
    equal_text(kind, "import") || registered
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, PartialEq, Eq))]
pub enum AdoptedPolicySource {
    ImportedOriginal,
    Current,
    RetainedTarget,
}

pub fn imported_policy_required(kind: &str, migrated: bool) -> (required: bool)
    ensures required <==> (kind@ == "import"@ && !migrated),
{
    equal_text(kind, "import") && !migrated
}

pub fn restore_present_import(imported: bool, maintenance: bool,
    managed_originals: bool) -> (restore: bool)
    ensures restore <==> (imported && maintenance && managed_originals),
{
    imported && maintenance && managed_originals
}

pub fn adopted_policy_source(kind: &str, migrated: bool, head: &str, current_head: &str,
    digest: &str, current_digest: &str) -> (source: AdoptedPolicySource)
    ensures
        (source == AdoptedPolicySource::ImportedOriginal) <==>
            (kind@ == "import"@ && !migrated),
        (source == AdoptedPolicySource::Current) <==>
            (!(kind@ == "import"@ && !migrated)
                && head@ == current_head@ && digest@ == current_digest@),
        (source == AdoptedPolicySource::RetainedTarget) <==>
            (!(kind@ == "import"@ && !migrated)
                && (head@ != current_head@ || digest@ != current_digest@)),
{
    if imported_policy_required(kind, migrated) { AdoptedPolicySource::ImportedOriginal }
    else if equal_text(head, current_head) && equal_text(digest, current_digest) {
        AdoptedPolicySource::Current
    } else { AdoptedPolicySource::RetainedTarget }
}






}
