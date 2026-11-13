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

pub open spec fn installed_prefix<S, E, Apply>(
    initial: S, states: Seq<S>, changed: Seq<bool>, apply: Apply,
) -> bool
    where Apply: Fn(S, usize) -> (S, Result<bool, E>),
{
    states.len() == changed.len() + 1 && states.first() == initial
        && forall|index: int| #![trigger changed[index]] 0 <= index < changed.len() ==>
            apply.ensures((states[index], index as usize),
                (states[index + 1], Ok::<bool, E>(changed[index])))
}

pub open spec fn installation_result<S, E, Apply>(
    initial: S, count: usize, apply: Apply, out: (S, Result<bool, E>),
) -> bool
    where Apply: Fn(S, usize) -> (S, Result<bool, E>),
{
    exists|states: Seq<S>, changed: Seq<bool>|
        installed_prefix(initial, states, changed, apply) && changed.len() <= count
        && match out.1 {
            Ok(any_changed) => changed.len() == count && out.0 == states.last()
                && any_changed == changed.contains(true),
            Err(error) => changed.len() < count
                && apply.ensures((states.last(), changed.len() as usize),
                    (out.0, Err::<bool, E>(error))),
        }
}

/// Inspect every target in order, including unchanged ones, and stop at the first error.
/// Only ghost sequences retain the call history; runtime state is moved through each call.
pub fn install_targets<S, E, Apply>(initial: S, count: usize, apply: Apply)
    -> (out: (S, Result<bool, E>))
    where Apply: Fn(S, usize) -> (S, Result<bool, E>),
    requires forall|state: S, index: usize| index < count ==>
        #[trigger] apply.requires((state, index)),
    ensures installation_result(initial, count, apply, out),
{
    let mut state = initial;
    let mut index = 0usize;
    let mut any_changed = false;
    let ghost states = seq![initial];
    let ghost changed: Seq<bool> = Seq::empty();
    while index < count
        invariant index <= count,
            states.len() == index as int + 1,
            changed.len() == index,
            states.last() == state,
            any_changed == changed.contains(true),
            installed_prefix(initial, states, changed, apply),
            forall|state: S, index: usize| index < count ==>
                #[trigger] apply.requires((state, index)),
        decreases count - index,
    {
        let ghost before = state;
        let observed = apply(state, index);
        let ghost original = observed;
        let (next, result) = observed;
        assert(apply.ensures((before, index), original));
        match result {
            Err(error) => {
                let out = (next, Err(error));
                assert(installation_result(initial, count, apply, out));
                return out;
            },
            Ok(wrote) => {
                assert(original == (next, Ok::<bool, E>(wrote)));
                proof {
                    let old_states = states;
                    let old_changed = changed;
                    states = states.push(next);
                    changed = changed.push(wrote);
                    assert forall|position: int| 0 <= position < changed.len() implies
                        #[trigger] apply.ensures((states[position], position as usize),
                            (states[position + 1], Ok::<bool, E>(changed[position]))) by {
                        if position < old_changed.len() {
                            assert(apply.ensures((old_states[position], position as usize),
                                (old_states[position + 1], Ok::<bool, E>(old_changed[position]))));
                        } else {
                            assert(position == index);
                            assert(states[position] == before);
                            assert(states[position + 1] == next);
                        }
                    }
                    vstd::seq_lib::lemma_seq_contains_after_push(old_changed, wrote, true);
                    assert(changed.contains(true) == (old_changed.contains(true) || wrote));
                }
                any_changed = any_changed || wrote;
            },
        }
        state = next;
        index += 1;
    }
    let out = (state, Ok(any_changed));
    assert(installation_result(initial, count, apply, out));
    out
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
