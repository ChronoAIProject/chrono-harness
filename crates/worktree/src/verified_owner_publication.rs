//! 所有者发布的实际身份判断与状态选择；文件系统和编码由调用方提供。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_identifiers.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
use identifiers::{equal_bytes, equal_text};

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Binding {
    pub identity: String,
    pub sha256: String,
    pub size: u64,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Staged {
    pub path: String,
    pub identity: String,
    pub sha256: String,
    pub size: u64,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Stage {
    pub selected: String,
    pub staged: Staged,
    pub previous: Binding,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Intent {
    pub schema: String,
    pub stage: Stage,
    pub replacement: Binding,
}

pub open spec fn same_binding(a: &Binding, b: &Binding) -> bool {
    a.identity@ == b.identity@ && a.sha256@ == b.sha256@ && a.size == b.size
}

pub fn binding_equal(a: &Binding, b: &Binding) -> (equal: bool)
    ensures equal <==> same_binding(a, b),
{
    equal_text(a.identity.as_str(), b.identity.as_str())
        && equal_text(a.sha256.as_str(), b.sha256.as_str()) && a.size == b.size
}

pub open spec fn stage_binding(actual: &Binding, expected: &Staged) -> bool {
    actual.identity@ == expected.identity@ && actual.sha256@ == expected.sha256@
        && actual.size == expected.size
}

pub fn staged_binding_equal(actual: &Binding, expected: &Staged) -> (equal: bool)
    ensures equal <==> stage_binding(actual, expected),
{
    equal_text(actual.identity.as_str(), expected.identity.as_str())
        && equal_text(actual.sha256.as_str(), expected.sha256.as_str())
        && actual.size == expected.size
}

pub open spec fn stage_bytes(actual: &Binding, expected: &Staged) -> bool {
    actual.sha256@ == expected.sha256@ && actual.size == expected.size
}

pub fn staged_bytes_equal(actual: &Binding, expected: &Staged) -> (equal: bool)
    ensures equal <==> stage_bytes(actual, expected),
{
    equal_text(actual.sha256.as_str(), expected.sha256.as_str()) && actual.size == expected.size
}

pub open spec fn same_stage(a: &Stage, b: &Stage) -> bool {
    a.selected@ == b.selected@ && a.staged.path@ == b.staged.path@
        && a.staged.identity@ == b.staged.identity@ && a.staged.sha256@ == b.staged.sha256@
        && a.staged.size == b.staged.size
        && same_binding(&a.previous, &b.previous)
}

pub fn stage_equal(a: &Stage, b: &Stage) -> (equal: bool)
    ensures equal <==> same_stage(a, b),
{
    equal_text(a.selected.as_str(), b.selected.as_str())
        && equal_text(a.staged.path.as_str(), b.staged.path.as_str())
        && equal_text(a.staged.identity.as_str(), b.staged.identity.as_str())
        && equal_text(a.staged.sha256.as_str(), b.staged.sha256.as_str())
        && a.staged.size == b.staged.size
        && binding_equal(&a.previous, &b.previous)
}

pub open spec fn prefix(actual: Seq<u8>, expected: Seq<u8>) -> bool {
    expected.len() <= actual.len()
        && forall|i: int| 0 <= i < expected.len() ==> actual[i] == expected[i]
}

pub fn has_prefix(actual: &[u8], expected: &[u8]) -> (matches: bool)
    ensures matches <==> prefix(actual@, expected@),
{
    if actual.len() < expected.len() { return false; }
    let mut i = 0;
    while i < expected.len()
        invariant
            expected.len() <= actual.len(),
            0 <= i <= expected.len(),
            forall|j: int| 0 <= j < i ==> actual[j] == expected[j],
        decreases expected.len() - i,
    {
        if actual[i] != expected[i] { return false; }
        i += 1;
    }
    true
}

// ASCII bytes for `.chrono-harness/bin/owner-stages/` and `.chrono-harness/bin/`.
// The fixed Verus release accepts numeric byte arrays, not Rust byte literals.
pub fn retained_paths(staged: &[u8], selected: &[u8]) -> (valid: bool)
    ensures valid <==> (
        prefix(staged@, seq![46u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 104u8, 97u8, 114u8, 110u8, 101u8, 115u8, 115u8, 47u8, 98u8, 105u8, 110u8, 47u8, 111u8, 119u8, 110u8, 101u8, 114u8, 45u8, 115u8, 116u8, 97u8, 103u8, 101u8, 115u8, 47u8])
            && prefix(selected@, seq![46u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 104u8, 97u8, 114u8, 110u8, 101u8, 115u8, 115u8, 47u8, 98u8, 105u8, 110u8, 47u8])),
{
    has_prefix(staged, &[46u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 104u8, 97u8, 114u8, 110u8, 101u8, 115u8, 115u8, 47u8, 98u8, 105u8, 110u8, 47u8, 111u8, 119u8, 110u8, 101u8, 114u8, 45u8, 115u8, 116u8, 97u8, 103u8, 101u8, 115u8, 47u8])
        && has_prefix(selected, &[46u8, 99u8, 104u8, 114u8, 111u8, 110u8, 111u8, 45u8, 104u8, 97u8, 114u8, 110u8, 101u8, 115u8, 115u8, 47u8, 98u8, 105u8, 110u8, 47u8])
}

pub fn stable_observation(
    before_id: &str, after_id: &str, before_size: u64, after_size: u64,
    before_seconds: i64, after_seconds: i64, before_nanos: i64, after_nanos: i64,
) -> (stable: bool)
    ensures stable <==> (before_id@ == after_id@ && before_size == after_size
        && before_seconds == after_seconds && before_nanos == after_nanos),
{
    equal_text(before_id, after_id) && before_size == after_size
        && before_seconds == after_seconds && before_nanos == after_nanos
}

pub enum OriginalAction { Create, Keep, Reject }

pub fn preserve_original(
    exists: bool, selected: Option<&Binding>, expected: &Binding,
) -> (action: OriginalAction)
    ensures
        (action is Keep) <==> exists,
        (action is Create) <==> (!exists && selected is Some
            && same_binding(selected.unwrap(), expected)),
        (action is Reject) <==> (!exists && (selected is None
            || !same_binding(selected.unwrap(), expected))),
{
    if exists { return OriginalAction::Keep; }
    match selected {
        Some(actual) => {
            if binding_equal(actual, expected) { OriginalAction::Create }
            else { OriginalAction::Reject }
        },
        None => OriginalAction::Reject,
    }
}

pub fn immutable_original(exists: bool, original: &[u8], expected: &[u8])
    -> (action: OriginalAction)
    ensures
        (action is Create) <==> !exists,
        (action is Keep) <==> (exists && original@ == expected@),
        (action is Reject) <==> (exists && original@ != expected@),
{
    if !exists { OriginalAction::Create }
    else if equal_bytes(original, expected) { OriginalAction::Keep }
    else { OriginalAction::Reject }
}

pub fn valid_intent(intent: &Intent, expected: &Stage) -> (valid: bool)
    ensures valid <==> (
        intent.schema@ == "chrono-bootstrap-owner-publication-intent/v1"@
            && same_stage(&intent.stage, expected)),
{
    equal_text(intent.schema.as_str(), "chrono-bootstrap-owner-publication-intent/v1")
        && stage_equal(&intent.stage, expected)
}

pub enum SelectedAction { Replace, Keep, Reject }

pub enum IntentAction { Read, Prepare, Reject }

pub fn intent_action(intent_exists: bool, prepared_exists: bool) -> (action: IntentAction)
    ensures
        (action is Read) <==> intent_exists,
        (action is Prepare) <==> (!intent_exists && !prepared_exists),
        (action is Reject) <==> (!intent_exists && prepared_exists),
{
    if intent_exists { IntentAction::Read }
    else if prepared_exists { IntentAction::Reject }
    else { IntentAction::Prepare }
}

pub fn selected_action(current: &Binding, previous: &Binding, replacement: &Binding)
    -> (action: SelectedAction)
    ensures
        (action is Replace) <==> same_binding(current, previous),
        (action is Keep) <==> (!same_binding(current, previous)
            && same_binding(current, replacement)),
        (action is Reject) <==> (!same_binding(current, previous)
            && !same_binding(current, replacement)),
{
    if binding_equal(current, previous) { SelectedAction::Replace }
    else if binding_equal(current, replacement) { SelectedAction::Keep }
    else { SelectedAction::Reject }
}

pub fn valid_publication(
    installed: &Binding, replacement: &Binding, retained: &Binding, expected: &Stage,
) -> (valid: bool)
    ensures valid <==> (same_binding(installed, replacement)
        && stage_bytes(installed, &expected.staged)
        && same_binding(retained, &expected.previous)),
{
    binding_equal(installed, replacement) && staged_bytes_equal(installed, &expected.staged)
        && binding_equal(retained, &expected.previous)
}

}
