//! Preparation must return its durable plan before any projection effect is dispatched.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub(crate) mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

verus! {
#[cfg_attr(not(verus_keep_ghost), derive(Clone, Debug, serde::Serialize, serde::Deserialize))]
pub struct Object {
    pub identity: String,
    pub sha256: String,
    pub bytes: u64,
}

pub open spec fn same_object_spec(first: Option<&Object>, second: Option<&Object>) -> bool {
    match (first, second) {
        (None, None) => true,
        (Some(left), Some(right)) => left.identity@ == right.identity@
            && left.sha256@ == right.sha256@ && left.bytes == right.bytes,
        _ => false,
    }
}

/// A missing original and an observed file never identify the same object.
pub fn same_object(first: Option<&Object>, second: Option<&Object>) -> (same: bool)
    ensures same == same_object_spec(first, second),
{
    match (first, second) {
        (None, None) => true,
        (Some(left), Some(right)) => identifiers::equal_text(&left.identity, &right.identity)
            && identifiers::equal_text(&left.sha256, &right.sha256) && left.bytes == right.bytes,
        _ => false,
    }
}

pub enum Effects { NotStarted, Unknown, Complete }

impl Effects {
    pub fn label(&self) -> (out: &'static str)
        ensures match self {
            Self::NotStarted => out@ == "not-started"@,
            Self::Unknown => out@ == "unknown"@,
            Self::Complete => out@ == "complete"@,
        },
    {
        match self {
            Self::NotStarted => "not-started",
            Self::Unknown => "unknown",
            Self::Complete => "complete",
        }
    }

    pub fn can_release(&self) -> (release: bool)
        ensures release == !(self is Unknown),
    {
        match self { Self::Unknown => false, _ => true }
    }
}

/// The original evaluation and its effect knowledge are produced together.
pub struct Evaluation<T, E> {
    pub result: Result<T, E>,
    pub effects: Effects,
}

pub open spec fn text_is(actual: Option<&str>, expected: &str) -> bool {
    match actual { Some(text) => text@ == expected@, None => false }
}

pub fn reported_unknown(effects: Option<&str>) -> (unknown: bool)
    ensures unknown == text_is(effects, "unknown"),
{
    match effects { Some(text) => identifiers::equal_text(text, "unknown"), None => false }
}

pub fn sealed_record(count: usize, phase: Option<&str>, sealed: Option<bool>) -> (ready: bool)
    ensures ready == (count == 1 && text_is(phase, "released") && sealed == Some(true)),
{
    if count != 1 { return false; }
    match phase {
        Some(text) => identifiers::equal_text(text, "released") && sealed == Some(true),
        None => false,
    }
}

pub open spec fn recovery_reference_spec(
    recorded_id: Option<&str>, attached_id: Option<&str>, attached_path: Option<&str>, path: &str,
) -> bool {
    match (recorded_id, attached_id, attached_path) {
        (Some(recorded), Some(attached), Some(actual_path)) =>
            recorded@ == attached@ && actual_path@ == path@,
        _ => false,
    }
}

pub fn recovery_reference_matches(
    recorded_id: Option<&str>, attached_id: Option<&str>, attached_path: Option<&str>, path: &str,
) -> (bound: bool)
    ensures bound == recovery_reference_spec(recorded_id, attached_id, attached_path, path),
{
    match (recorded_id, attached_id, attached_path) {
        (Some(recorded), Some(attached), Some(actual_path)) =>
            identifiers::equal_text(recorded, attached) && identifiers::equal_text(actual_path, path),
        _ => false,
    }
}

pub fn intent_address_matches(schema: Option<&str>, attempt: Option<&str>, path: &str) -> (bound: bool)
    ensures bound == (text_is(schema, "chrono-ci-generation-intent/v1") && text_is(attempt, path)),
{
    match (schema, attempt) {
        (Some(schema), Some(attempt)) =>
            identifiers::equal_text(schema, "chrono-ci-generation-intent/v1")
                && identifiers::equal_text(attempt, path),
        _ => false,
    }
}

pub fn result_address_matches(
    schema: Option<&str>, attempt: Option<&str>, intent_sha256: Option<&str>,
    path: &str, digest: &str, exit_is_null: bool,
) -> (bound: bool)
    ensures bound == (text_is(schema, "chrono-ci-generation-result/v1")
        && text_is(attempt, path) && text_is(intent_sha256, digest) && exit_is_null),
{
    match (schema, attempt, intent_sha256) {
        (Some(schema), Some(attempt), Some(intent_sha256)) =>
            identifiers::equal_text(schema, "chrono-ci-generation-result/v1")
                && identifiers::equal_text(attempt, path)
                && identifiers::equal_text(intent_sha256, digest) && exit_is_null,
        _ => false,
    }
}

pub open spec fn evaluation<S, P, T, E, Prepare, Apply>(
    initial: S, prepare: Prepare, apply: Apply, out: (S, Evaluation<T, E>),
) -> bool
    where Prepare: FnOnce(S) -> (S, Result<P, E>),
          Apply: FnOnce(S, P) -> (S, Result<T, E>),
{
    exists|state: S, prepared: Result<P, E>|
        #[trigger] prepare.ensures((initial,), (state, prepared))
        && match prepared {
            Err(error) => out.0 == state && out.1.result == Err::<T, E>(error)
                && out.1.effects is NotStarted,
            Ok(plan) => exists|next: S, result: Result<T, E>|
                #[trigger] apply.ensures((state, plan), (next, result))
                && out.0 == next && out.1.result == result
                && match result {
                    Ok(_) => out.1.effects is Complete,
                    Err(_) => out.1.effects is Unknown,
                },
        }
}

pub fn prepare_then_apply<S, P, T, E, Prepare, Apply>(
    initial: S, prepare: Prepare, apply: Apply,
) -> (out: (S, Evaluation<T, E>))
    where Prepare: FnOnce(S) -> (S, Result<P, E>),
          Apply: FnOnce(S, P) -> (S, Result<T, E>),
    requires prepare.requires((initial,)),
        forall|state: S, plan: P| apply.requires((state, plan)),
    ensures evaluation(initial, prepare, apply, out),
{
    let ghost before = initial;
    let (state, prepared) = prepare(initial);
    let ghost middle = state;
    let ghost original = prepared;
    let out = match prepared {
        Err(error) => (state, Evaluation { result: Err(error), effects: Effects::NotStarted }),
        Ok(plan) => {
            let (next, result) = apply(state, plan);
            let effects = match &result {
                Ok(_) => Effects::Complete,
                Err(_) => Effects::Unknown,
            };
            (next, Evaluation { result, effects })
        },
    };
    assert(prepare.ensures((before,), (middle, original)));
    out
}
}
