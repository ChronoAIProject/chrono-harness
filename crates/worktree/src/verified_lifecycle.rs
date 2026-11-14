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

pub struct ImportedPolicyReceipt<'a> {
    pub schema: Option<&'a str>,
    pub operation: Option<&'a str>,
    pub status: Option<&'a str>,
    pub source_root: Option<&'a str>,
    pub report_path: Option<&'a str>,
    pub config_path: Option<&'a str>,
}

pub open spec fn required_text(observed: Option<&str>, expected: &str) -> bool {
    match observed { Some(text) => text@ == expected@, None => false }
}

fn matches_required(observed: Option<&str>, expected: &str) -> (valid: bool)
    ensures valid == required_text(observed, expected),
{
    match observed { Some(text) => equal_text(text, expected), None => false }
}

pub fn imported_policy_receipt_valid(receipt: &ImportedPolicyReceipt, published: bool,
    root: &str, path: &str, config: &str) -> (valid: bool)
    ensures valid <==> (published
        && required_text(receipt.schema, "chrono-worktree-report/v1")
        && required_text(receipt.operation, "import")
        && required_text(receipt.status, "imported")
        && required_text(receipt.source_root, root)
        && required_text(receipt.report_path, path)
        && required_text(receipt.config_path, config)),
{
    published
        && matches_required(receipt.schema, "chrono-worktree-report/v1")
        && matches_required(receipt.operation, "import")
        && matches_required(receipt.status, "imported")
        && matches_required(receipt.source_root, root)
        && matches_required(receipt.report_path, path)
        && matches_required(receipt.config_path, config)
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

pub fn retire_cache_history(completed: bool, generation: u64, current_generation: u64,
    consumers_released: bool, confirmed_at: Option<u64>, observed: u64,
    keep_seconds: u64, over_limit: bool) -> (retire: bool)
    ensures retire <==> (completed && generation < current_generation && consumers_released
        && confirmed_at.is_some() && confirmed_at.unwrap() <= observed
        && (over_limit || observed - confirmed_at.unwrap() >= keep_seconds)),
{
    retire_history(completed && generation < current_generation, consumers_released,
        confirmed_at, observed, keep_seconds, over_limit)
}

}
