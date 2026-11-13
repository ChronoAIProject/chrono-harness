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

pub open spec fn borrowed_object<'a>(object: &'a Option<Object>) -> Option<&'a Object> {
    match object { Some(object) => Some(object), None => None }
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

pub type OriginalBytes = (Option<Object>, Option<Vec<u8>>);

pub open spec fn original_snapshot(
    before: String, bytes: Vec<u8>, digest: String, out: Result<OriginalBytes, String>,
) -> bool {
    match out {
        Ok((Some(object), Some(saved))) => object.identity@ == before@
            && object.sha256@ == digest@ && object.bytes == bytes@.len()
            && saved@ == bytes@,
        _ => false,
    }
}

pub open spec fn original_bytes_result<S, Identity, Hash, Changed>(
    initial: S, before: String, bytes: Vec<u8>, limit: u64,
    identity: Identity, hash: Hash, changed: Changed,
    out: (S, Result<OriginalBytes, String>),
) -> bool
    where Identity: FnOnce(S) -> (S, Result<String, String>),
          Hash: FnOnce(S, &Vec<u8>) -> (S, String),
          Changed: FnOnce(S) -> (S, String),
{
    if bytes@.len() > limit {
        out.0 == initial && match out.1 {
            Err(error) => error@ == "generation original exceeds byte limit"@,
            _ => false,
        }
    } else {
        exists|state: S, observed: Result<String, String>|
            #[trigger] identity.ensures((initial,), (state, observed)) && match observed {
                Err(error) => out == (state, Err::<OriginalBytes, String>(error)),
                Ok(actual) => if before@ != actual@ {
                    match out.1 {
                        Err(error) => changed.ensures((state,), (out.0, error)),
                        _ => false,
                    }
                } else {
                    exists|next: S, digest: String|
                        #[trigger] hash.ensures((state, &bytes), (next, digest))
                        && out.0 == next && original_snapshot(before, bytes, digest, out.1)
                },
            }
    }
}

fn finish_original_bytes<S, Identity, Hash, Changed>(
    state: S, before: String, bytes: Vec<u8>, limit: u64,
    identity: Identity, hash: Hash, changed: Changed,
) -> (out: (S, Result<OriginalBytes, String>))
    where Identity: FnOnce(S) -> (S, Result<String, String>),
          Hash: FnOnce(S, &Vec<u8>) -> (S, String),
          Changed: FnOnce(S) -> (S, String),
    requires forall|state: S| #[trigger] identity.requires((state,)),
        forall|state: S, bytes: &Vec<u8>| #[trigger] hash.requires((state, bytes)),
        forall|state: S| #[trigger] changed.requires((state,)),
    ensures original_bytes_result(state, before, bytes, limit, identity, hash, changed, out),
{
    if bytes.len() as u128 > limit as u128 {
        return (state, Err("generation original exceeds byte limit".to_owned()));
    }
    let ghost initial = state;
    let ghost original_bytes = bytes;
    let ghost original_before = before;
    let (state, observed) = identity(state);
    let ghost after_identity = state;
    let ghost original = observed;
    let out = match observed {
        Err(error) => (state, Err(error)),
        Ok(actual) => {
            if !identifiers::equal_text(&before, &actual) {
                let (state, error) = changed(state);
                (state, Err(error))
            } else {
                let (state, digest) = hash(state, &bytes);
                let object = Object { identity: before, sha256: digest, bytes: bytes.len() as u64 };
                (state, Ok((Some(object), Some(bytes))))
            }
        },
    };
    assert(identity.ensures((initial,), (after_identity, original)));
    assert(original_bytes_result(initial, original_before, original_bytes, limit,
        identity, hash, changed, out));
    out
}

pub open spec fn original_read_result<S, Read, Identity, Hash, Changed>(
    initial: S, before: String, limit: u64,
    read: Read, identity: Identity, hash: Hash, changed: Changed,
    out: (S, Result<OriginalBytes, String>),
) -> bool
    where Read: FnOnce(S) -> (S, Result<Vec<u8>, String>),
          Identity: FnOnce(S) -> (S, Result<String, String>),
          Hash: FnOnce(S, &Vec<u8>) -> (S, String),
          Changed: FnOnce(S) -> (S, String),
{
    exists|state: S, observed: Result<Vec<u8>, String>|
        #[trigger] read.ensures((initial,), (state, observed)) && match observed {
            Err(error) => out == (state, Err::<OriginalBytes, String>(error)),
            Ok(bytes) => original_bytes_result(state, before, bytes, limit, identity, hash, changed, out),
        }
}

/// Read first, reject oversized bytes before rechecking identity, then hash and retain the exact bytes.
pub fn read_present_original<S, Read, Identity, Hash, Changed>(
    state: S, before: String, limit: u64,
    read: Read, identity: Identity, hash: Hash, changed: Changed,
) -> (out: (S, Result<OriginalBytes, String>))
    where Read: FnOnce(S) -> (S, Result<Vec<u8>, String>),
          Identity: FnOnce(S) -> (S, Result<String, String>),
          Hash: FnOnce(S, &Vec<u8>) -> (S, String),
          Changed: FnOnce(S) -> (S, String),
    requires read.requires((state,)),
        forall|state: S| #[trigger] identity.requires((state,)),
        forall|state: S, bytes: &Vec<u8>| #[trigger] hash.requires((state, bytes)),
        forall|state: S| #[trigger] changed.requires((state,)),
    ensures original_read_result(state, before, limit, read, identity, hash, changed, out),
{
    let ghost initial = state;
    let (state, observed) = read(state);
    let ghost after_read = state;
    let ghost original = observed;
    let out = match observed {
        Err(error) => (state, Err(error)),
        Ok(bytes) => finish_original_bytes(state, before, bytes, limit, identity, hash, changed),
    };
    assert(read.ensures((initial,), (after_read, original)));
    out
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

pub enum TargetMismatch { Prepared, Rename, Published }

impl TargetMismatch {
    pub fn label(&self) -> (out: &'static str)
        ensures match self {
            Self::Prepared => out@ == "generation output changed after preparation"@,
            Self::Rename => out@ == "generation output changed before rename"@,
            Self::Published => out@ == "generation output differs after publication"@,
        },
    {
        match self {
            Self::Prepared => "generation output changed after preparation",
            Self::Rename => "generation output changed before rename",
            Self::Published => "generation output differs after publication",
        }
    }
}

pub struct TargetIo<O, W, N, R, D, F, M> {
    pub observe: O,
    pub stage: W,
    pub create: N,
    pub replace: R,
    pub sync: D,
    pub effect: F,
    pub mismatch: M,
}

pub open spec fn target_callable<S, P, E, O, W, N, R, D, F, M>(io: &TargetIo<O, W, N, R, D, F, M>) -> bool
    where O: Fn(S) -> (S, Result<Option<Object>, E>), W: Fn(S) -> (S, Result<P, E>),
          N: Fn(S, &P) -> (S, Result<(), E>), R: Fn(S, &P) -> (S, Result<(), E>),
          D: Fn(S, P) -> (S, Result<(), E>), F: Fn(S, Option<Object>) -> (S, Result<(), E>),
          M: Fn(S, TargetMismatch) -> (S, E),
{
    (forall|s: S| #[trigger] io.observe.requires((s,)))
        && (forall|s: S| #[trigger] io.stage.requires((s,)))
        && (forall|s: S, parent: &P| #[trigger] io.create.requires((s, parent)))
        && (forall|s: S, parent: &P| #[trigger] io.replace.requires((s, parent)))
        && (forall|s: S, parent: P| #[trigger] io.sync.requires((s, parent)))
        && (forall|s: S, object: Option<Object>| #[trigger] io.effect.requires((s, object)))
        && (forall|s: S, reason: TargetMismatch| #[trigger] io.mismatch.requires((s, reason)))
}

pub open spec fn published_digest(object: Option<&Object>, digest: &str) -> bool {
    match object { Some(object) => object.sha256@ == digest@, None => false }
}

pub open spec fn target_effect_result<S, E>(observed: (S, Result<(), E>), out: (S, Result<bool, E>)) -> bool {
    out.0 == observed.0 && match observed.1 {
        Err(error) => out.1 == Err::<bool, E>(error),
        Ok(()) => out.1 == Ok::<bool, E>(true),
    }
}

pub open spec fn target_final_result<S, P, E, O, W, N, R, D, F, M>(
    initial: S, digest: &str, io: &TargetIo<O, W, N, R, D, F, M>, out: (S, Result<bool, E>),
) -> bool
    where O: Fn(S) -> (S, Result<Option<Object>, E>), W: Fn(S) -> (S, Result<P, E>),
          N: Fn(S, &P) -> (S, Result<(), E>), R: Fn(S, &P) -> (S, Result<(), E>),
          D: Fn(S, P) -> (S, Result<(), E>), F: Fn(S, Option<Object>) -> (S, Result<(), E>),
          M: Fn(S, TargetMismatch) -> (S, E),
{
    exists|state: S, observed: Result<Option<Object>, E>|
        #[trigger] io.observe.ensures((initial,), (state, observed)) && match observed {
            Err(error) => out == (state, Err::<bool, E>(error)),
            Ok(object) => if !published_digest(borrowed_object(&object), digest) {
                match out.1 {
                    Err(error) => io.mismatch.ensures((state, TargetMismatch::Published), (out.0, error)),
                    _ => false,
                }
            } else {
                exists|effect: (S, Result<(), E>)|
                    #[trigger] io.effect.ensures((state, object), effect) && target_effect_result(effect, out)
            },
        }
}

fn finish_target<S, P, E, O, W, N, R, D, F, M>(
    state: S, digest: &str, io: &TargetIo<O, W, N, R, D, F, M>,
) -> (out: (S, Result<bool, E>))
    where O: Fn(S) -> (S, Result<Option<Object>, E>), W: Fn(S) -> (S, Result<P, E>),
          N: Fn(S, &P) -> (S, Result<(), E>), R: Fn(S, &P) -> (S, Result<(), E>),
          D: Fn(S, P) -> (S, Result<(), E>), F: Fn(S, Option<Object>) -> (S, Result<(), E>),
          M: Fn(S, TargetMismatch) -> (S, E),
    requires target_callable(io),
    ensures target_final_result(state, digest, io, out),
{
    let ghost initial = state;
    let (state, observed) = (io.observe)(state);
    let ghost after_observe = state;
    let ghost original = observed;
    let out = match observed {
        Err(error) => (state, Err(error)),
        Ok(object) => {
            let valid = match object.as_ref() {
                Some(object) => identifiers::equal_text(&object.sha256, digest),
                None => false,
            };
            if !valid {
                let (state, error) = (io.mismatch)(state, TargetMismatch::Published);
                (state, Err(error))
            } else {
                let effect = (io.effect)(state, object);
                let ghost original_effect = effect;
                let (state, result) = effect;
                let out = (state, match result { Err(error) => Err(error), Ok(()) => Ok(true) });
                assert(target_effect_result(original_effect, out));
                out
            }
        },
    };
    assert(io.observe.ensures((initial,), (after_observe, original)));
    out
}

pub open spec fn target_publication_step<S, P, E, N, R>(
    initial: S, parent: P, missing: bool, create: N, replace: R,
    observed: (S, Result<(), E>),
) -> bool
    where N: Fn(S, &P) -> (S, Result<(), E>), R: Fn(S, &P) -> (S, Result<(), E>),
{
    if missing { create.ensures((initial, &parent), observed) }
    else { replace.ensures((initial, &parent), observed) }
}

pub open spec fn target_publish_result<S, P, E, O, W, N, R, D, F, M>(
    initial: S, parent: P, missing: bool, digest: &str, io: &TargetIo<O, W, N, R, D, F, M>,
    out: (S, Result<bool, E>),
) -> bool
    where O: Fn(S) -> (S, Result<Option<Object>, E>), W: Fn(S) -> (S, Result<P, E>),
          N: Fn(S, &P) -> (S, Result<(), E>), R: Fn(S, &P) -> (S, Result<(), E>),
          D: Fn(S, P) -> (S, Result<(), E>), F: Fn(S, Option<Object>) -> (S, Result<(), E>),
          M: Fn(S, TargetMismatch) -> (S, E),
{
    exists|state: S, published: Result<(), E>|
        #[trigger] target_publication_step(initial, parent, missing, io.create, io.replace,
            (state, published))
        && match published {
            Err(error) => out == (state, Err::<bool, E>(error)),
            Ok(()) => exists|next: S, synced: Result<(), E>|
                #[trigger] io.sync.ensures((state, parent), (next, synced)) && match synced {
                    Err(error) => out == (next, Err::<bool, E>(error)),
                    Ok(()) => target_final_result(next, digest, io, out),
                },
        }
}

fn publish_target<S, P, E, O, W, N, R, D, F, M>(
    state: S, parent: P, missing: bool, digest: &str, io: &TargetIo<O, W, N, R, D, F, M>,
) -> (out: (S, Result<bool, E>))
    where O: Fn(S) -> (S, Result<Option<Object>, E>), W: Fn(S) -> (S, Result<P, E>),
          N: Fn(S, &P) -> (S, Result<(), E>), R: Fn(S, &P) -> (S, Result<(), E>),
          D: Fn(S, P) -> (S, Result<(), E>), F: Fn(S, Option<Object>) -> (S, Result<(), E>),
          M: Fn(S, TargetMismatch) -> (S, E),
    requires target_callable(io),
    ensures target_publish_result(state, parent, missing, digest, io, out),
{
    let ghost initial = state;
    let (state, published) = if missing { (io.create)(state, &parent) } else { (io.replace)(state, &parent) };
    let ghost after_publish = state;
    let ghost original_publish = published;
    assert(if missing { io.create.ensures((initial, &parent), (after_publish, original_publish)) }
        else { io.replace.ensures((initial, &parent), (after_publish, original_publish)) });
    assert(target_publication_step(initial, parent, missing, io.create, io.replace,
        (after_publish, original_publish)));
    let out = match published {
        Err(error) => {
            let out = (state, Err(error));
            assert(target_publish_result(initial, parent, missing, digest, io, out));
            out
        },
        Ok(()) => {
            let (state, synced) = (io.sync)(state, parent);
            let ghost after_sync = state;
            let ghost original_sync = synced;
            let out = match synced {
                Err(error) => (state, Err(error)),
                Ok(()) => finish_target(state, digest, io),
            };
            assert(io.sync.ensures((after_publish, parent), (after_sync, original_sync)));
            assert(exists|next: S, synced: Result<(), E>|
                #[trigger] io.sync.ensures((after_publish, parent), (next, synced)) && match synced {
                    Err(error) => out == (next, Err::<bool, E>(error)),
                    Ok(()) => target_final_result(next, digest, io, out),
                });
            assert(target_publish_result(initial, parent, missing, digest, io, out));
            out
        },
    };
    assert(target_publish_result(initial, parent, missing, digest, io, out));
    out
}

pub open spec fn target_staged_result<S, P, E, O, W, N, R, D, F, M>(
    initial: S, parent: P, before: Option<&Object>, digest: &str,
    io: &TargetIo<O, W, N, R, D, F, M>, out: (S, Result<bool, E>),
) -> bool
    where O: Fn(S) -> (S, Result<Option<Object>, E>), W: Fn(S) -> (S, Result<P, E>),
          N: Fn(S, &P) -> (S, Result<(), E>), R: Fn(S, &P) -> (S, Result<(), E>),
          D: Fn(S, P) -> (S, Result<(), E>), F: Fn(S, Option<Object>) -> (S, Result<(), E>),
          M: Fn(S, TargetMismatch) -> (S, E),
{
    exists|state: S, observed: Result<Option<Object>, E>|
        #[trigger] io.observe.ensures((initial,), (state, observed)) && match observed {
            Err(error) => out == (state, Err::<bool, E>(error)),
            Ok(object) => if !same_object_spec(borrowed_object(&object), before) {
                match out.1 {
                    Err(error) => io.mismatch.ensures((state, TargetMismatch::Rename), (out.0, error)),
                    _ => false,
                }
            } else { target_publish_result(state, parent, before.is_none(), digest, io, out) },
        }
}

fn install_staged_target<S, P, E, O, W, N, R, D, F, M>(
    state: S, parent: P, before: Option<&Object>, digest: &str, io: &TargetIo<O, W, N, R, D, F, M>,
) -> (out: (S, Result<bool, E>))
    where O: Fn(S) -> (S, Result<Option<Object>, E>), W: Fn(S) -> (S, Result<P, E>),
          N: Fn(S, &P) -> (S, Result<(), E>), R: Fn(S, &P) -> (S, Result<(), E>),
          D: Fn(S, P) -> (S, Result<(), E>), F: Fn(S, Option<Object>) -> (S, Result<(), E>),
          M: Fn(S, TargetMismatch) -> (S, E),
    requires target_callable(io),
    ensures target_staged_result(state, parent, before, digest, io, out),
{
    let ghost initial = state;
    let (state, observed) = (io.observe)(state);
    let ghost after_observe = state;
    let ghost original = observed;
    let out = match observed {
        Err(error) => (state, Err(error)),
        Ok(object) => {
            if !same_object(object.as_ref(), before) {
                let (state, error) = (io.mismatch)(state, TargetMismatch::Rename);
                (state, Err(error))
            } else { publish_target(state, parent, before.is_none(), digest, io) }
        },
    };
    assert(io.observe.ensures((initial,), (after_observe, original)));
    out
}

pub open spec fn target_installation_result<S, P, E, O, W, N, R, D, F, M>(
    initial: S, before: Option<&Object>, write: bool, digest: &str,
    io: &TargetIo<O, W, N, R, D, F, M>, out: (S, Result<bool, E>),
) -> bool
    where O: Fn(S) -> (S, Result<Option<Object>, E>), W: Fn(S) -> (S, Result<P, E>),
          N: Fn(S, &P) -> (S, Result<(), E>), R: Fn(S, &P) -> (S, Result<(), E>),
          D: Fn(S, P) -> (S, Result<(), E>), F: Fn(S, Option<Object>) -> (S, Result<(), E>),
          M: Fn(S, TargetMismatch) -> (S, E),
{
    exists|state: S, observed: Result<Option<Object>, E>|
        #[trigger] io.observe.ensures((initial,), (state, observed)) && match observed {
            Err(error) => out == (state, Err::<bool, E>(error)),
            Ok(object) => if !same_object_spec(borrowed_object(&object), before) {
                match out.1 {
                    Err(error) => io.mismatch.ensures((state, TargetMismatch::Prepared), (out.0, error)),
                    _ => false,
                }
            } else if !write { out == (state, Ok::<bool, E>(false)) }
            else {
                exists|next: S, staged: Result<P, E>|
                    #[trigger] io.stage.ensures((state,), (next, staged)) && match staged {
                        Err(error) => out == (next, Err::<bool, E>(error)),
                        Ok(parent) => target_staged_result(next, parent, before, digest, io, out),
                    }
            },
        }
}

/// Recheck even unchanged targets; stage, recheck, publish, sync and record in that order.
pub fn install_target<S, P, E, O, W, N, R, D, F, M>(
    state: S, before: Option<&Object>, write: bool, digest: &str, io: &TargetIo<O, W, N, R, D, F, M>,
) -> (out: (S, Result<bool, E>))
    where O: Fn(S) -> (S, Result<Option<Object>, E>), W: Fn(S) -> (S, Result<P, E>),
          N: Fn(S, &P) -> (S, Result<(), E>), R: Fn(S, &P) -> (S, Result<(), E>),
          D: Fn(S, P) -> (S, Result<(), E>), F: Fn(S, Option<Object>) -> (S, Result<(), E>),
          M: Fn(S, TargetMismatch) -> (S, E),
    requires target_callable(io),
    ensures target_installation_result(state, before, write, digest, io, out),
{
    let ghost initial = state;
    let (state, observed) = (io.observe)(state);
    let ghost after_observe = state;
    let ghost original = observed;
    let out = match observed {
        Err(error) => (state, Err(error)),
        Ok(object) => {
            if !same_object(object.as_ref(), before) {
                let (state, error) = (io.mismatch)(state, TargetMismatch::Prepared);
                (state, Err(error))
            } else if !write { (state, Ok(false)) }
            else {
                let (state, staged) = (io.stage)(state);
                let ghost after_stage = state;
                let ghost original_stage = staged;
                let out = match staged {
                    Err(error) => (state, Err(error)),
                    Ok(parent) => install_staged_target(state, parent, before, digest, io),
                };
                assert(io.stage.ensures((after_observe,), (after_stage, original_stage)));
                out
            }
        },
    };
    assert(io.observe.ensures((initial,), (after_observe, original)));
    out
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
