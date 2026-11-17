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
pub use identifiers::equal_text;

#[cfg(verus_keep_ghost)]
#[path = "maintenance.rs"]
mod maintenance_verification;
#[cfg(not(verus_keep_ghost))]
pub use crate::maintenance::retained_reference_ancestor_required;
#[cfg(verus_keep_ghost)]
pub use maintenance_verification::retained_reference_ancestor_required;

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
    MissingCacheObservation, absence_probe_required, absent_attachment_conflict, admission_handoff,
    known_absent_enrollment, missing_cache_observation, publish_enrollment_record,
    retain_cleanup_attempt,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::{
    MissingCacheObservation, absence_probe_required, absent_attachment_conflict, admission_handoff,
    known_absent_enrollment, missing_cache_observation, publish_enrollment_record,
    retain_cleanup_attempt,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::age::{age_preservation_required, reusable_release};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::age::{age_preservation_required, reusable_release};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::inventory::append_action_index;
#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::migration::{
    adopted_policy_binding_error, checkout_migration_ready, policy_migration_decision,
    policy_state_conflict, policy_state_present, store_current_policy,
};
#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::service::{
    maintenance_exclusions, maintenance_idle_exit, maintenance_retention_phase,
    validate_activation_sequence,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::inventory::append_action_index;
#[cfg(verus_keep_ghost)]
pub use automatic_verification::migration::{
    adopted_policy_binding_error, checkout_migration_ready, policy_migration_decision,
    policy_state_conflict, policy_state_present, store_current_policy,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::service::{
    maintenance_exclusions, maintenance_idle_exit, maintenance_retention_phase,
    validate_activation_sequence,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::producer::rolling::{rolling_disposal, rolling_keep};
#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::producer::{
    ProducerDeclarations, ProducerLookupError, ProducerNamespaceMember, find_producer_declaration,
    producer_namespace_member, producer_namespace_retired, take_resolved_store,
    visit_producer_stores,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::producer::rolling::{rolling_disposal, rolling_keep};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::producer::{
    ProducerDeclarations, ProducerLookupError, ProducerNamespaceMember, find_producer_declaration,
    producer_namespace_member, producer_namespace_retired, producer_visit_prefix,
    take_resolved_store, visit_producer_stores,
};

#[cfg(not(verus_keep_ghost))]
pub use crate::automatic::retirement::{retire_cache_history, retire_history};
#[cfg(not(verus_keep_ghost))]
pub use crate::recovery::{
    ImportedPolicyReceipt, imported_policy_receipt_valid, imported_policy_source_commit,
};
#[cfg(verus_keep_ghost)]
pub use automatic_verification::retirement::{retire_cache_history, retire_history};
#[cfg(verus_keep_ghost)]
#[path = "recovery.rs"]
#[rustfmt::skip]
mod recovery_verification;
#[cfg(verus_keep_ghost)]
pub use recovery_verification::{
    ImportedPolicyReceipt, imported_policy_receipt_valid, imported_policy_source_commit,
    required_text,
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

/// 普通消费交接不改变观察、导入、迁移或部署的现有资格边界。
pub fn ordinary_admission(operation: &str) -> (selected: bool)
    ensures selected <==> (operation@ == "use"@ || operation@ == "check"@
        || operation@ == "bootstrap"@),
{
    equal_text(operation, "use") || equal_text(operation, "check")
        || equal_text(operation, "bootstrap")
}

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

pub open spec fn cleanup_stage(cache: bool, status: Seq<char>, removal: Seq<char>) -> CleanupStage {
    if status != "cleaned"@ { CleanupStage::Failed }
    else if cache { CleanupStage::CachesReleased }
    else if removal == "verified-absent"@ || removal == "already-absent"@ {
        CleanupStage::Disposed
    } else { CleanupStage::Retained }
}

pub fn cleanup_result(cache: bool, status: &str, removal: &str) -> (stage: CleanupStage)
    ensures stage == cleanup_stage(cache, status@, removal@),
{
    if !equal_text(status, "cleaned") { CleanupStage::Failed }
    else if cache { CleanupStage::CachesReleased }
    else if equal_text(removal, "verified-absent") || equal_text(removal, "already-absent") {
        CleanupStage::Disposed
    } else { CleanupStage::Retained }
}

pub struct CleanupOutcome {
    pub cache: bool,
    pub cleaned: bool,
    pub stage: CleanupStage,
}

pub open spec fn cleanup_field(value: Option<&str>) -> Seq<char> {
    match value { Some(text) => text@, None => ""@ }
}

/// 原 JSON 观察只投影字段；缓存资格、成功与终态优先级由同一证明根决定。
pub fn observed_cleanup_result(cache: bool, enrollment_status: &str,
    age_outcome: Option<&str>, status: Option<&str>, removal: Option<&str>)
    -> (out: CleanupOutcome)
    ensures out.cache == (cleanup_field(age_outcome) == "reusable-source-preserved"@
            || cleanup_field(age_outcome) == "no-disposable-output-retained-checkout"@
            || (cache && cleanup_field(age_outcome)
                != "expired-checkout-verified-absent-source-branches-retained"@)),
        out.cleaned == (cleanup_field(status) == "cleaned"@),
        out.stage == cleanup_stage(out.cache && enrollment_status@ != "terminal"@,
            cleanup_field(status), cleanup_field(removal)),
{
    let age = match age_outcome { Some(value) => value, None => "" };
    let cache_outcome = equal_text(age, "reusable-source-preserved")
        || equal_text(age, "no-disposable-output-retained-checkout")
        || (cache && !equal_text(age, "expired-checkout-verified-absent-source-branches-retained"));
    let status = match status { Some(value) => value, None => "" };
    let removal = match removal { Some(value) => value, None => "" };
    let cleaned = equal_text(status, "cleaned");
    let stage = cleanup_result(cache_outcome && !equal_text(enrollment_status, "terminal"),
        status, removal);
    CleanupOutcome { cache: cache_outcome, cleaned, stage }
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
