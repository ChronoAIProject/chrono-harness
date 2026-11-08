//! 发布判断的独立边界；证明读取生产源码，Cargo 使用同一生产实现。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../worktree/src/verified_owner_publication.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_worktree::verified_owner_publication as policy;
use policy::{Binding, Intent, Stage, Staged};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn policy_namespace_binds_protocol_and_physical_address() {
    broadcast use vstd::array::group_array_axioms;
    proof { reveal_strlit("v2"); reveal_strlit("v1"); reveal_strlit("state");
        reveal_strlit("other"); assert("v2"@[1] != "v1"@[1]);
        assert("state"@[0] != "other"@[0]); }
    let expected = policy::PolicyNamespace { schema: "v2".to_string(),
        coordinator: vec![47u8, 97], state_directory: "state".to_string() };
    let independent = policy::PolicyNamespace { schema: "v2".to_string(),
        coordinator: vec![47u8, 97], state_directory: "state".to_string() };
    let protocol = policy::PolicyNamespace { schema: "v1".to_string(),
        coordinator: vec![47u8, 97], state_directory: "state".to_string() };
    let coordinator = policy::PolicyNamespace { schema: "v2".to_string(),
        coordinator: vec![47u8, 98], state_directory: "state".to_string() };
    let directory = policy::PolicyNamespace { schema: "v2".to_string(),
        coordinator: vec![47u8, 97], state_directory: "other".to_string() };
    let same = policy::policy_namespace_matches(&expected, &independent);
    let old_protocol = policy::policy_namespace_matches(&expected, &protocol);
    let moved_owner = policy::policy_namespace_matches(&expected, &coordinator);
    let moved_state = policy::policy_namespace_matches(&expected, &directory);
    assert(same && !old_protocol && !moved_owner && !moved_state);
    if !same || old_protocol || moved_owner || moved_state { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn stage_report_value_drift_and_incomplete_processes_are_rejected() {
    broadcast use vstd::array::group_array_axioms;
    let same = policy::StageRequirement::Equal {
        actual: vec![0u8, 255], expected: vec![0u8, 255],
    };
    let changed = policy::StageRequirement::Equal {
        actual: vec![0u8, 254], expected: vec![0u8, 255],
    };
    let accepted = policy::valid_stage_requirement(&same);
    let rejected = policy::valid_stage_requirement(&changed);
    assert(accepted && !rejected);
    let complete = policy::valid_stage_requirement(
        &policy::StageRequirement::Process { exit: Some(0), pid: Some(1) });
    let failed = policy::valid_stage_requirement(
        &policy::StageRequirement::Process { exit: Some(1), pid: Some(1) });
    let absent_exit = policy::valid_stage_requirement(
        &policy::StageRequirement::Process { exit: None, pid: Some(1) });
    let absent_pid = policy::valid_stage_requirement(
        &policy::StageRequirement::Process { exit: Some(0), pid: None });
    let zero_pid = policy::valid_stage_requirement(
        &policy::StageRequirement::Process { exit: Some(0), pid: Some(0) });
    assert(complete && !failed && !absent_exit && !absent_pid && !zero_pid);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn stage_report_names_reject_missing_or_reused_originals() {
    broadcast use vstd::string::group_string_axioms;
    broadcast use vstd::array::group_array_axioms;
    let names = vec!["stdout".to_string(), "stderr".to_string()];
    proof {
        reveal_strlit(""); reveal_strlit("stdout"); reveal_strlit("stderr");
        assert("stdout"@[3] != "stderr"@[3]);
        assert(names[0]@ == "stdout"@);
        assert(names[1]@ == "stderr"@);
        assert("stdout"@.len() == 6 && "stderr"@.len() == 6 && ""@.len() == 0);
        assert(names[0]@.len() == 6 && names[1]@.len() == 6);
        assert forall|i: int| 0 <= i < names.len() implies names[i]@ != ""@ by {
            assert(i == 0 || i == 1);
            if i == 0 { assert(names[i]@ == names[0]@); }
            else { assert(names[i]@ == names[1]@); }
            assert(names[i]@.len() == 6);
        }
        assert forall|i: int, j: int| 0 <= i < j < names.len()
            implies names[i]@ != names[j]@ by { assert(i == 0 && j == 1); }
    }
    let unique = policy::StageRequirement::Names { names, nonempty: true };
    let duplicate = policy::StageRequirement::Names {
        names: vec!["stdout".to_string(), "stdout".to_string()], nonempty: false,
    };
    let empty = policy::StageRequirement::Names {
        names: vec!["".to_string()], nonempty: true,
    };
    proof { reveal_strlit(""); reveal_strlit("stdout"); reveal_strlit("stderr");
        assert("stdout"@[3] != "stderr"@[3]); }
    let valid = policy::valid_stage_requirement(&unique);
    let reused = policy::valid_stage_requirement(&duplicate);
    let unnamed = policy::valid_stage_requirement(&empty);
    assert(valid);
    assert(!reused);
    assert(!unnamed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn stage_report_collects_the_first_failed_requirement() {
    let checks = vec![
        policy::StageCheck { requirement: policy::StageRequirement::Process {
            exit: Some(0), pid: Some(1) }, diagnostic: "first".to_string() },
        policy::StageCheck { requirement: policy::StageRequirement::Process {
            exit: Some(0), pid: Some(0) }, diagnostic: "second".to_string() },
        policy::StageCheck { requirement: policy::StageRequirement::Process {
            exit: Some(1), pid: Some(1) }, diagnostic: "third".to_string() },
    ];
    let first = policy::first_stage_failure(&checks);
    assert(first == Some(1usize));
    let empty = policy::first_stage_failure(&Vec::new());
    assert(empty is None);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn stage_report_digest_path_and_relocation_boundaries() {
    broadcast use vstd::array::group_array_axioms;
    let zeros = [48u8; 64];
    let uppercase = [65u8; 64];
    let short = [48u8; 63];
    assert(zeros@.len() == 64 && uppercase@.len() == 64 && short@.len() == 63);
    assert forall|i: int| 0 <= i < zeros@.len() implies zeros@[i] == 48u8 by { }
    assert(uppercase@[0] == 65u8);
    let valid = policy::valid_stage_digest(&zeros);
    let bad_case = policy::valid_stage_digest(&uppercase);
    let bad_length = policy::valid_stage_digest(&short);
    assert(valid);
    assert(!bad_case);
    assert(!bad_length);
    let absolute = policy::valid_stage_requirement(
        &policy::StageRequirement::AbsolutePath { bytes: vec![47u8, 97] });
    let relative = policy::valid_stage_requirement(
        &policy::StageRequirement::AbsolutePath { bytes: vec![97u8] });
    let missing = policy::valid_stage_requirement(
        &policy::StageRequirement::AbsolutePath { bytes: Vec::new() });
    assert(absolute && !relative && !missing);
    let registry = policy::valid_stage_requirement(
        &policy::StageRequirement::BindingPath { registry: true, has_path: false });
    let inline_missing = policy::valid_stage_requirement(
        &policy::StageRequirement::BindingPath { registry: false, has_path: false });
    assert(registry && !inline_missing);
    let fixed = policy::valid_stage_requirement(&policy::StageRequirement::Relocation {
        allowed: false, actual: vec![47u8, 97], declared: Some(vec![47u8, 97]) });
    let drift = policy::valid_stage_requirement(&policy::StageRequirement::Relocation {
        allowed: false, actual: vec![47u8, 98], declared: Some(vec![47u8, 97]) });
    let moved = policy::valid_stage_requirement(&policy::StageRequirement::Relocation {
        allowed: true, actual: vec![47u8, 98], declared: None });
    let undeclared = policy::valid_stage_requirement(&policy::StageRequirement::Relocation {
        allowed: false, actual: vec![47u8, 98], declared: None });
    assert(fixed && !drift && moved && !undeclared);
}

fn binding(identity: &str, hash: &str, size: u64) -> (out: Binding)
    ensures out.identity@ == identity@, out.sha256@ == hash@, out.size == size,
{
    Binding { identity: identity.to_string(), sha256: hash.to_string(), size }
}

fn stage() -> (out: Stage)
    ensures out.selected@ == "target"@, out.staged.path@ == "stage"@,
        out.staged.identity@ == "stage-inode"@, out.staged.sha256@ == "new-hash"@,
        out.staged.size == 2, out.previous.identity@ == "old"@,
        out.previous.sha256@ == "old-hash"@, out.previous.size == 1,
{
    Stage {
        selected: "target".to_string(),
        staged: Staged { path: "stage".to_string(), identity: "stage-inode".to_string(),
            sha256: "new-hash".to_string(), size: 2 },
        previous: binding("old", "old-hash", 1),
    }
}

proof fn distinct_names()
    ensures "old"@ != "new"@, "old-hash"@ != "new-hash"@,
        "target"@ != "other"@, "stage"@ != "other"@,
        "stage-inode"@ != "other"@, "new-hash"@ != "other"@,
        "old"@ != "other"@, "new"@ != "other"@, "old-hash"@ != "other"@,
{
    reveal_strlit("old"); reveal_strlit("new"); reveal_strlit("old-hash");
    reveal_strlit("new-hash"); reveal_strlit("target"); reveal_strlit("stage");
    reveal_strlit("stage-inode"); reveal_strlit("other");
    assert("old"@[0] != "new"@[0]);
    assert("old-hash"@[0] != "new-hash"@[0]);
    assert("target"@.len() != "other"@.len());
    assert("stage"@[0] != "other"@[0]);
    assert("stage-inode"@.len() != "other"@.len());
    assert("new-hash"@.len() != "other"@.len());
    assert("old"@.len() != "other"@.len());
    assert("new"@.len() != "other"@.len());
    assert("old-hash"@.len() != "other"@.len());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_requires_identity_hash_and_size_but_payload_allows_another_inode() {
    proof { distinct_names(); }
    let expected = stage();
    let exact = binding("stage-inode", "new-hash", 2);
    let same = policy::staged_binding_equal(&exact, &expected.staged);
    assert(same);
    let different_inode = binding("other", "new-hash", 2);
    let identity = policy::staged_binding_equal(&different_inode, &expected.staged);
    assert(!identity);
    let payload = policy::staged_bytes_equal(&different_inode, &expected.staged);
    assert(payload);
    let different_hash = binding("stage-inode", "other", 2);
    let hash = policy::staged_binding_equal(&different_hash, &expected.staged);
    let hash_bytes = policy::staged_bytes_equal(&different_hash, &expected.staged);
    assert(!hash && !hash_bytes);
    let different_size = binding("stage-inode", "new-hash", 3);
    let size = policy::staged_binding_equal(&different_size, &expected.staged);
    let size_bytes = policy::staged_bytes_equal(&different_size, &expected.staged);
    assert(!size && !size_bytes);
    let equal = policy::binding_equal(&exact, &exact);
    let changed_identity = policy::binding_equal(&exact, &different_inode);
    let changed_hash = policy::binding_equal(&exact, &different_hash);
    let changed_size = policy::binding_equal(&exact, &different_size);
    assert(equal && !changed_identity && !changed_hash && !changed_size);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn preserving_original_requires_exact_selected_binding_only_when_absent() {
    proof { distinct_names(); }
    let old = binding("old", "old-hash", 1);
    let new = binding("new", "new-hash", 2);
    let create = policy::preserve_original(false, Some(&old), &old);
    assert(create is Create);
    let keep_missing = policy::preserve_original(true, None, &old);
    assert(keep_missing is Keep);
    let keep_new = policy::preserve_original(true, Some(&new), &old);
    assert(keep_new is Keep);
    let missing = policy::preserve_original(false, None, &old);
    assert(missing is Reject);
    let drift = policy::preserve_original(false, Some(&new), &old);
    assert(drift is Reject);
    // Keep 只保留已有原件；调用方随后仍核其完整绑定。
    let retained_drift = policy::binding_equal(&new, &old);
    assert(!retained_drift);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn immutable_original_preserves_all_bytes_including_nul_and_non_utf8() {
    broadcast use vstd::array::group_array_axioms;
    let original = [0u8, 255, 97];
    let same = [0u8, 255, 97];
    let changed = [0u8, 254, 97];
    let extra = [0u8, 255, 97, 0];
    assert(original@.len() == 3);
    assert(extra@.len() == 4);
    let absent = policy::immutable_original(false, &[], &original);
    assert(absent is Create);
    let keep = policy::immutable_original(true, &same, &original);
    assert(keep is Keep);
    let reject = policy::immutable_original(true, &changed, &original);
    assert(reject is Reject);
    let reject_extra = policy::immutable_original(true, &extra, &original);
    assert(reject_extra is Reject);
    let empty = policy::immutable_original(true, &[], &[]);
    assert(empty is Keep);
    let missing_bytes = policy::immutable_original(true, &[], &original);
    assert(missing_bytes is Reject);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn intent_presence_handles_first_publication_reentry_and_orphan_preparation() {
    let first = policy::intent_action(false, false);
    assert(first is Prepare);
    let orphan = policy::intent_action(false, true);
    assert(orphan is Reject);
    let prepared = policy::intent_action(true, true);
    assert(prepared is Read);
    let published = policy::intent_action(true, false);
    assert(published is Read);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selected_owner_requires_original_or_exact_published_binding() {
    proof { distinct_names(); }
    let old = binding("old", "old-hash", 1);
    let new = binding("new", "new-hash", 2);
    let alien = binding("other", "new-hash", 2);
    let replace = policy::selected_action(&old, &old, &new);
    assert(replace is Replace);
    let reentry = policy::selected_action(&new, &old, &new);
    assert(reentry is Keep);
    let drift = policy::selected_action(&alien, &old, &new);
    assert(drift is Reject);
    let same_bindings = policy::selected_action(&old, &old, &old);
    assert(same_bindings is Replace);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_stage_binding_rejects_each_independent_field_change() {
    proof { distinct_names(); }
    let expected = stage();
    let exact = stage();
    let accepted = policy::stage_equal(&exact, &expected);
    assert(accepted);
    let mut changed = stage();
    changed.selected = "other".to_string();
    let selected = policy::stage_equal(&changed, &expected);
    assert(!selected);
    let mut changed = stage();
    changed.staged.path = "other".to_string();
    let path = policy::stage_equal(&changed, &expected);
    assert(!path);
    let mut changed = stage();
    changed.staged.identity = "other".to_string();
    let inode = policy::stage_equal(&changed, &expected);
    assert(!inode);
    let mut changed = stage();
    changed.staged.sha256 = "other".to_string();
    let hash = policy::stage_equal(&changed, &expected);
    assert(!hash);
    let mut changed = stage();
    changed.staged.size = 3;
    let size = policy::stage_equal(&changed, &expected);
    assert(!size);
    let mut changed = stage();
    changed.previous.identity = "other".to_string();
    let previous_inode = policy::stage_equal(&changed, &expected);
    assert(!previous_inode);
    let mut changed = stage();
    changed.previous.sha256 = "other".to_string();
    let previous_hash = policy::stage_equal(&changed, &expected);
    assert(!previous_hash);
    let mut changed = stage();
    changed.previous.size = 0;
    let previous_size = policy::stage_equal(&changed, &expected);
    assert(!previous_size);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn intent_requires_current_schema_and_complete_stage() {
    proof {
        distinct_names();
        reveal_strlit("chrono-bootstrap-owner-publication-intent/v1");
        reveal_strlit("other");
        assert("chrono-bootstrap-owner-publication-intent/v1"@.len() != "other"@.len());
    }
    let expected = stage();
    let mut intent = Intent {
        schema: "chrono-bootstrap-owner-publication-intent/v1".to_string(),
        stage: stage(), replacement: binding("new", "new-hash", 2),
    };
    let accepted = policy::valid_intent(&intent, &expected);
    assert(accepted);
    intent.stage.previous.identity = "other".to_string();
    let wrong_stage = policy::valid_intent(&intent, &expected);
    assert(!wrong_stage);
    intent.stage = stage();
    intent.schema = "other".to_string();
    let wrong_schema = policy::valid_intent(&intent, &expected);
    assert(!wrong_schema);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observation_requires_all_metadata_fields_at_integer_boundaries() {
    proof { distinct_names(); }
    let stable = policy::stable_observation("old", "old", u64::MAX, u64::MAX,
        i64::MIN, i64::MIN, i64::MAX, i64::MAX);
    assert(stable);
    let id = policy::stable_observation("old", "new", 0, 0, 0, 0, 0, 0);
    assert(!id);
    let size = policy::stable_observation("old", "old", 0, u64::MAX, 0, 0, 0, 0);
    assert(!size);
    let seconds = policy::stable_observation("old", "old", 0, 0, i64::MIN, i64::MAX, 0, 0);
    assert(!seconds);
    let nanos = policy::stable_observation("old", "old", 0, 0, 0, 0, i64::MIN, i64::MAX);
    assert(!nanos);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn prefixes_cover_empty_short_exact_wrong_and_retained_locations() {
    let empty_prefix = policy::has_prefix(&[97u8], &[]);
    assert(empty_prefix);
    let short = policy::has_prefix(&[], &[97u8]);
    assert(!short);
    let exact = policy::has_prefix(&[97u8, 98], &[97u8, 98]);
    assert(exact);
    let longer = policy::has_prefix(&[97u8, 98, 99], &[97u8, 98]);
    assert(longer);
    let wrong = policy::has_prefix(&[97u8, 99], &[97u8, 98]);
    assert(!wrong);
    let retained = policy::retained_paths(&[46u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 104u8, 97u8, 114u8, 110u8, 101u8, 115u8, 115u8, 47u8, 98u8, 105u8, 110u8, 47u8, 111u8, 119u8, 110u8, 101u8, 114u8, 45u8, 115u8, 116u8, 97u8, 103u8, 101u8, 115u8, 47u8, 97u8, 47u8, 111u8, 119u8, 110u8, 101u8, 114u8],
        &[46u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 104u8, 97u8, 114u8, 110u8, 101u8, 115u8, 115u8, 47u8, 98u8, 105u8, 110u8, 47u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 119u8, 111u8, 114u8, 107u8, 116u8, 114u8, 101u8, 101u8]);
    assert(retained);
    let ephemeral = policy::retained_paths(&[46u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 104u8, 97u8, 114u8, 110u8, 101u8, 115u8, 115u8, 47u8, 99u8, 97u8, 99u8, 104u8, 101u8, 47u8, 111u8, 119u8, 110u8, 101u8, 114u8],
        &[46u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 104u8, 97u8, 114u8, 110u8, 101u8, 115u8, 115u8, 47u8, 98u8, 105u8, 110u8, 47u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 119u8, 111u8, 114u8, 107u8, 116u8, 114u8, 101u8, 101u8]);
    assert(!ephemeral);
    let wrong_selected = policy::retained_paths(&[46u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 104u8, 97u8, 114u8, 110u8, 101u8, 115u8, 115u8, 47u8, 98u8, 105u8, 110u8, 47u8, 111u8, 119u8, 110u8, 101u8, 114u8, 45u8, 115u8, 116u8, 97u8, 103u8, 101u8, 115u8, 47u8, 97u8, 47u8, 111u8, 119u8, 110u8, 101u8, 114u8], &[47u8, 116u8, 109u8, 112u8, 47u8, 111u8, 119u8, 110u8, 101u8, 114u8]);
    assert(!wrong_selected);
    let missing = policy::retained_paths(&[], &[]);
    assert(!missing);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn final_publication_requires_exact_installed_payload_and_retained_original() {
    proof { distinct_names(); }
    let expected = stage();
    let replacement = binding("new", "new-hash", 2);
    let installed = binding("new", "new-hash", 2);
    let old = binding("old", "old-hash", 1);
    let accepted = policy::valid_publication(&installed, &replacement, &old, &expected);
    assert(accepted);
    let alien = binding("other", "new-hash", 2);
    let inode = policy::valid_publication(&alien, &replacement, &old, &expected);
    assert(!inode);
    let wrong_payload = binding("new", "other", 2);
    let hash = policy::valid_publication(&wrong_payload, &wrong_payload, &old, &expected);
    assert(!hash);
    let wrong_size = binding("new", "new-hash", 3);
    let size = policy::valid_publication(&wrong_size, &wrong_size, &old, &expected);
    assert(!size);
    let retained = policy::valid_publication(&installed, &replacement, &alien, &expected);
    assert(!retained);
}

}
