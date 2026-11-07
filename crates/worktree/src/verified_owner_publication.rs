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

// The JSON/IO adapter provides separate observed and declared values. All
// decisions about those values and finite report-name sets live in this root.
pub enum StageRequirement {
    Equal { actual: Vec<u8>, expected: Vec<u8> },
    Names { names: Vec<String>, nonempty: bool },
    Process { exit: Option<i64>, pid: Option<u64> },
    Digest { bytes: Vec<u8> },
    AbsolutePath { bytes: Vec<u8> },
}

pub open spec fn distinct_stage_names(names: Seq<String>, nonempty: bool) -> bool {
    (forall|i: int| 0 <= i < names.len() ==> (!nonempty || names[i]@.len() > 0))
        && (forall|i: int, j: int| 0 <= i < j < names.len()
            ==> names[i]@ != names[j]@)
}

pub fn valid_stage_names(names: &Vec<String>, nonempty: bool) -> (valid: bool)
    ensures valid <==> distinct_stage_names(names@, nonempty),
{
    let mut i = 0;
    while i < names.len()
        invariant
            0 <= i <= names.len(),
            forall|j: int| 0 <= j < i ==> (!nonempty || names[j]@.len() > 0),
            forall|j: int, k: int| 0 <= j < k < i ==> names[j]@ != names[k]@,
        decreases names.len() - i,
    {
        if nonempty && equal_text(names[i].as_str(), "") { return false; }
        let mut j = 0;
        while j < i
            invariant
                0 <= j <= i < names.len(),
                forall|k: int| 0 <= k < j ==> names[k]@ != names[i as int]@,
            decreases i - j,
        {
            if equal_text(names[j].as_str(), names[i].as_str()) { return false; }
            j += 1;
        }
        i += 1;
    }
    true
}

pub open spec fn stage_digest(bytes: Seq<u8>) -> bool {
    bytes.len() == 64 && forall|i: int| 0 <= i < bytes.len()
        ==> identifiers::oid_byte(#[trigger] bytes[i])
}

pub fn valid_stage_digest(bytes: &[u8]) -> (valid: bool)
    ensures valid <==> stage_digest(bytes@),
{
    if bytes.len() != 64 { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant
            bytes.len() == 64, 0 <= i <= bytes.len(),
            forall|j: int| 0 <= j < i ==> identifiers::oid_byte(#[trigger] bytes[j]),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if !((48 <= byte && byte <= 57) || (97 <= byte && byte <= 102)) { return false; }
        i += 1;
    }
    true
}

pub open spec fn stage_requirement(check: &StageRequirement) -> bool {
    match check {
        StageRequirement::Equal { actual, expected } => actual@ == expected@,
        StageRequirement::Names { names, nonempty } => distinct_stage_names(names@, *nonempty),
        StageRequirement::Process { exit, pid } => *exit == Some(0i64)
            && pid is Some && pid.unwrap() > 0,
        StageRequirement::Digest { bytes } => stage_digest(bytes@),
        StageRequirement::AbsolutePath { bytes } => bytes@.len() > 0 && bytes@[0] == 47,
    }
}

pub fn valid_stage_requirement(check: &StageRequirement) -> (valid: bool)
    ensures valid <==> stage_requirement(check),
{
    match check {
        StageRequirement::Equal { actual, expected } => equal_bytes(actual, expected),
        StageRequirement::Names { names, nonempty } => valid_stage_names(names, *nonempty),
        StageRequirement::Process { exit, pid } => match (exit, pid) {
            (Some(code), Some(identity)) => *code == 0 && *identity > 0,
            _ => false,
        },
        StageRequirement::Digest { bytes } => valid_stage_digest(bytes),
        StageRequirement::AbsolutePath { bytes } => bytes.len() > 0 && bytes[0] == 47,
    }
}

pub fn first_stage_failure(checks: &Vec<StageRequirement>) -> (failed: Option<usize>)
    ensures
        failed is None <==> forall|i: int| 0 <= i < checks.len()
            ==> stage_requirement(&checks[i]),
        failed is Some ==> (failed.unwrap() < checks.len()
            && !stage_requirement(&checks[failed.unwrap() as int])
            && forall|i: int| 0 <= i < failed.unwrap() ==> stage_requirement(&checks[i])),
{
    let mut i = 0;
    while i < checks.len()
        invariant
            0 <= i <= checks.len(),
            forall|j: int| 0 <= j < i ==> stage_requirement(&checks[j]),
        decreases checks.len() - i,
    {
        if !valid_stage_requirement(&checks[i]) { return Some(i); }
        i += 1;
    }
    None
}

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
