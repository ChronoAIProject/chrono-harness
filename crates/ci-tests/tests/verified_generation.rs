//! Independent complete call traces and original errors, in Rust and Verus.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../ci/src/verified_generation.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_ci::verified_generation as policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
#[cfg(verus_keep_ghost)]
use policy::identifiers;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
fn expect(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

fn object(identity: &str, sha256: &str, bytes: u64) -> (out: policy::Object)
    ensures out.identity@ == identity@, out.sha256@ == sha256@, out.bytes == bytes,
{
    policy::Object { identity: identity.to_string(), sha256: sha256.to_string(), bytes }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn generation_objects_preserve_presence_identity_digest_and_length() {
    proof {
        reveal_strlit("16777231:42:file");
        reveal_strlit("16777231:43:file");
        reveal_strlit("original-digest");
        reveal_strlit("different-digest");
        reveal_strlit("目录\0原件\n");
        reveal_strlit("目录");
        assert("16777231:42:file"@[10] != "16777231:43:file"@[10]);
        assert("original-digest"@[0] != "different-digest"@[0]);
        assert("目录\0原件\n"@.len() != "目录"@.len());
    }
    let original = object("16777231:42:file", "original-digest", 7);
    let same = object("16777231:42:file", "original-digest", 7);
    let replaced = object("16777231:43:file", "original-digest", 7);
    let changed = object("16777231:42:file", "different-digest", 7);
    let resized = object("16777231:42:file", "original-digest", 8);
    expect(policy::same_object(None, None));
    expect(!policy::same_object(Some(&original), None));
    expect(!policy::same_object(None, Some(&original)));
    expect(policy::same_object(Some(&original), Some(&same)));
    expect(!policy::same_object(Some(&original), Some(&replaced)));
    expect(!policy::same_object(Some(&original), Some(&changed)));
    expect(!policy::same_object(Some(&original), Some(&resized)));

    let empty = object("", "", 0);
    let empty_same = object("", "", 0);
    let boundary = object("目录\0原件\n", "é\0摘要", u64::MAX);
    let boundary_same = object("目录\0原件\n", "é\0摘要", u64::MAX);
    let split_fields = object("目录", "原件\n\0é\0摘要", u64::MAX);
    expect(policy::same_object(Some(&empty), Some(&empty_same)));
    expect(policy::same_object(Some(&boundary), Some(&boundary_same)));
    expect(!policy::same_object(Some(&boundary), Some(&split_fields)));
}

spec fn original_bytes(empty: bool) -> Seq<u8> {
    if empty { Seq::empty() } else { seq![0u8, 255u8, 7u8] }
}

fn original_fixture(empty: bool) -> (bytes: Vec<u8>)
    ensures bytes@ == original_bytes(empty),
{
    let mut bytes = Vec::new();
    if !empty { bytes.push(0); bytes.push(255); bytes.push(7); }
    bytes
}

spec fn original_digest(bytes: Seq<u8>) -> Seq<char> {
    if bytes.len() == 0 { "empty\0摘要"@ }
    else if bytes.len() == 3 && bytes[0] == 0 && bytes[1] == 255 && bytes[2] == 7 {
        "full\0摘要"@
    } else { "wrong original bytes"@ }
}

fn original_read_case(empty: bool, read_fails: bool, limit: u64, identity_fails: bool, drift: bool) {
    proof {
        reveal_strlit("identity\0原件\n"); reveal_strlit("changed\0原件\n");
        assert("identity\0原件\n"@[0] != "changed\0原件\n"@[0]);
    }
    let read = |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<Vec<u8>, String>))
        ensures out.0@ == trace@.push(1),
            match out.1 {
                Err(error) => read_fails && error@ == "read\0error\n"@,
                Ok(bytes) => !read_fails && bytes@ == original_bytes(empty),
            },
    {
        trace.push(1);
        (trace, if read_fails { Err("read\0error\n".to_owned()) } else { Ok(original_fixture(empty)) })
    };
    let identity = |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<String, String>))
        ensures out.0@ == trace@.push(2),
            match out.1 {
                Err(error) => identity_fails && error@ == "identity\0error\n"@,
                Ok(actual) => !identity_fails
                    && actual@ == if drift { "changed\0原件\n"@ } else { "identity\0原件\n"@ },
            },
    {
        trace.push(2);
        (trace, if identity_fails { Err("identity\0error\n".to_owned()) }
            else { Ok(if drift { "changed\0原件\n".to_owned() } else { "identity\0原件\n".to_owned() }) })
    };
    let hash = |mut trace: Vec<u8>, bytes: &Vec<u8>| -> (out: (Vec<u8>, String))
        ensures out.0@ == trace@.push(3), out.1@ == original_digest(bytes@),
    {
        trace.push(3);
        let digest = if bytes.is_empty() { "empty\0摘要" }
            else if bytes.len() == 3 && bytes[0] == 0 && bytes[1] == 255 && bytes[2] == 7 {
                "full\0摘要"
            } else { "wrong original bytes" };
        (trace, digest.to_owned())
    };
    let changed = |mut trace: Vec<u8>| -> (out: (Vec<u8>, String))
        ensures out.0@ == trace@.push(4), out.1@ == "changed\0error\n"@,
    {
        trace.push(4);
        (trace, "changed\0error\n".to_owned())
    };
    let before = "identity\0原件\n".to_owned();
    let initial: Vec<u8> = Vec::new();
    let ghost start = initial;
    let ghost original_before = before;
    let (trace, result) = policy::read_present_original(initial, before, limit, read, identity, hash, changed);
    proof {
        let (after_read, observed) = choose|state: Vec<u8>, observed: Result<Vec<u8>, String>|
            #[trigger] read.ensures((start,), (state, observed)) && match observed {
                Err(error) => (trace, result) == (state, Err::<policy::OriginalBytes, String>(error)),
                Ok(bytes) => policy::original_bytes_result(state, original_before, bytes, limit,
                    identity, hash, changed, (trace, result)),
            };
        if !read_fails {
            let bytes = observed->Ok_0;
            if bytes@.len() <= limit {
                let (after_identity, actual) = choose|state: Vec<u8>, actual: Result<String, String>|
                    #[trigger] identity.ensures((after_read,), (state, actual)) && match actual {
                        Err(error) => (trace, result) == (state, Err::<policy::OriginalBytes, String>(error)),
                        Ok(current) => if original_before@ != current@ {
                            match result {
                                Err(error) => changed.ensures((state,), (trace, error)),
                                _ => false,
                            }
                        } else {
                            exists|next: Vec<u8>, digest: String|
                                #[trigger] hash.ensures((state, &bytes), (next, digest))
                                && trace == next && policy::original_snapshot(original_before, bytes, digest, result)
                        },
                    };
                if !identity_fails && !drift {
                    let (next, digest) = choose|next: Vec<u8>, digest: String|
                        #[trigger] hash.ensures((after_identity, &bytes), (next, digest))
                        && trace == next && policy::original_snapshot(original_before, bytes, digest, result);
                }
            }
        }
    }
    expect(trace[0] == 1);
    if read_fails {
        expect(trace.len() == 1);
        match result { Err(error) => expect(identifiers::equal_text(&error, "read\0error\n")), _ => expect(false) }
    } else if (!empty && limit < 3) {
        expect(trace.len() == 1);
        match result { Err(error) => expect(identifiers::equal_text(&error, "generation original exceeds byte limit")), _ => expect(false) }
    } else if identity_fails {
        expect(trace.len() == 2); expect(trace[1] == 2);
        match result { Err(error) => expect(identifiers::equal_text(&error, "identity\0error\n")), _ => expect(false) }
    } else if drift {
        expect(trace.len() == 3); expect(trace[1] == 2); expect(trace[2] == 4);
        match result { Err(error) => expect(identifiers::equal_text(&error, "changed\0error\n")), _ => expect(false) }
    } else {
        expect(trace.len() == 3); expect(trace[1] == 2); expect(trace[2] == 3);
        match result {
            Ok((Some(object), Some(bytes))) => {
                expect(identifiers::equal_text(&object.identity, "identity\0原件\n"));
                expect(object.bytes == if empty { 0 } else { 3 });
                expect(bytes.len() == if empty { 0 } else { 3 });
                expect(identifiers::equal_text(&object.sha256, if empty { "empty\0摘要" } else { "full\0摘要" }));
                if !empty { expect(bytes[0] == 0); expect(bytes[1] == 255); expect(bytes[2] == 7); }
            },
            _ => expect(false),
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_reading_keeps_bytes_identity_order_and_first_failure() {
    original_read_case(false, false, 3, false, false);
    original_read_case(false, false, u64::MAX, false, false);
    original_read_case(true, false, 0, false, false);
    original_read_case(true, false, u64::MAX, false, false);
    original_read_case(false, true, 0, true, true);
    original_read_case(false, true, u64::MAX, true, true);
    original_read_case(false, false, 0, true, true);
    original_read_case(false, false, 2, true, true);
    original_read_case(false, false, 3, true, true);
    original_read_case(false, false, 3, false, true);
    original_read_case(true, false, 0, true, true);
    original_read_case(true, false, 0, false, true);
}

fn case(prepare_fails: bool, apply_fails: bool) {
    let prepare = |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<u64, (u64, u64)>))
        ensures out.0@ == trace@.push(1),
            out.1 == if prepare_fails { Err::<u64, (u64, u64)>((u64::MAX, 0)) } else { Ok::<u64, (u64, u64)>(u64::MAX) },
    {
        trace.push(1);
        (trace, if prepare_fails { Err((u64::MAX, 0)) } else { Ok(u64::MAX) })
    };
    let apply = |mut trace: Vec<u8>, plan: u64| -> (out: (Vec<u8>, Result<u64, (u64, u64)>))
        ensures out.0@ == trace@.push(2),
            out.1 == if apply_fails { Err::<u64, (u64, u64)>((plan, 1)) } else { Ok::<u64, (u64, u64)>(plan) },
    {
        trace.push(2);
        (trace, if apply_fails { Err((plan, 1)) } else { Ok(plan) })
    };
    let initial: Vec<u8> = Vec::new();
    let ghost before = initial;
    let (trace, evaluated) = policy::prepare_then_apply(initial, prepare, apply);
    proof {
        let (middle, prepared) = choose|state: Vec<u8>, prepared: Result<u64, (u64, u64)>|
            #[trigger] prepare.ensures((before,), (state, prepared))
            && match prepared {
                Err(error) => trace == state && evaluated.result == Err::<u64, (u64, u64)>(error)
                    && evaluated.effects is NotStarted,
                Ok(plan) => exists|next: Vec<u8>, result: Result<u64, (u64, u64)>|
                    #[trigger] apply.ensures((state, plan), (next, result))
                    && trace == next && evaluated.result == result
                    && match result {
                        Ok(_) => evaluated.effects is Complete,
                        Err(_) => evaluated.effects is Unknown,
                    },
            };
        assert(middle@ == seq![1u8]);
        if !prepare_fails {
            assert(prepared == Ok::<u64, (u64, u64)>(u64::MAX));
            let (next, result) = choose|next: Vec<u8>, result: Result<u64, (u64, u64)>|
                #[trigger] apply.ensures((middle, u64::MAX), (next, result))
                && trace == next && evaluated.result == result
                && match result {
                    Ok(_) => evaluated.effects is Complete,
                    Err(_) => evaluated.effects is Unknown,
                };
        }
    }
    expect(trace.len() == if prepare_fails { 1 } else { 2 });
    expect(trace[0] == 1);
    if !prepare_fails { expect(trace[1] == 2); }
    if prepare_fails {
        expect(matches!(evaluated.effects, policy::Effects::NotStarted));
    } else if apply_fails {
        expect(matches!(evaluated.effects, policy::Effects::Unknown));
    } else {
        expect(matches!(evaluated.effects, policy::Effects::Complete));
    }
    expect(evaluated.effects.can_release() == (prepare_fails || !apply_fails));
    match evaluated.result {
        Err((code, source)) => {
            expect(prepare_fails || apply_fails);
            expect(code == u64::MAX);
            expect(source == if prepare_fails { 0 } else { 1 });
        },
        Ok(value) => { expect(!prepare_fails && !apply_fails); expect(value == u64::MAX); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn preparation_success_and_failure_keep_order_and_original_result() {
    case(false, false);
    case(false, true);
    case(true, false);
    case(true, true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn effect_labels_keep_unknown_distinct_from_no_work_and_completed_work() {
    let untouched = policy::Effects::NotStarted;
    let uncertain = policy::Effects::Unknown;
    let completed = policy::Effects::Complete;
    expect(identifiers::equal_text(untouched.label(), "not-started"));
    expect(identifiers::equal_text(uncertain.label(), "unknown"));
    expect(identifiers::equal_text(completed.label(), "complete"));
    expect(untouched.can_release());
    expect(!uncertain.can_release());
    expect(completed.can_release());
    proof {
        reveal_strlit(""); reveal_strlit("unknown");
        reveal_strlit("UNKNOWN"); reveal_strlit("unknown\0");
        assert("unknown"@[0] != "UNKNOWN"@[0]);
        assert("unknown"@.len() != "unknown\0"@.len());
        assert("unknown"@.len() != ""@.len());
    }
    expect(policy::reported_unknown(Some("unknown")));
    expect(!policy::reported_unknown(None));
    expect(!policy::reported_unknown(Some("")));
    expect(!policy::reported_unknown(Some("UNKNOWN")));
    expect(!policy::reported_unknown(Some("unknown\0")));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_completion_requires_one_released_and_sealed_record() {
    proof {
        reveal_strlit("released"); reveal_strlit("released-extra");
        reveal_strlit("releasing");
        assert("released"@.len() != "released-extra"@.len());
        assert("released"@.len() != "releasing"@.len());
    }
    expect(policy::sealed_record(1, Some("released"), Some(true)));
    expect(!policy::sealed_record(0, Some("released"), Some(true)));
    expect(!policy::sealed_record(2, Some("released"), Some(true)));
    expect(!policy::sealed_record(usize::MAX, Some("released"), Some(true)));
    expect(!policy::sealed_record(1, Some("released"), Some(false)));
    expect(!policy::sealed_record(1, Some("released"), None));
    expect(!policy::sealed_record(1, None, Some(true)));
    expect(!policy::sealed_record(1, Some("released-extra"), Some(true)));
    expect(!policy::sealed_record(1, Some("releasing"), Some(true)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_uses_the_complete_original_id_and_path_and_rejects_missing_fields() {
    proof {
        reveal_strlit("对象\0甲"); reveal_strlit("对象\0乙");
        reveal_strlit("目录/\0原件\n"); reveal_strlit("目录/\0原件");
        assert("对象\0甲"@[3] != "对象\0乙"@[3]);
        assert("目录/\0原件\n"@.len() != "目录/\0原件"@.len());
    }
    let id = "对象\0甲";
    let path = "目录/\0原件\n";
    expect(policy::recovery_reference_matches(Some(id), Some(id), Some(path), path));
    expect(!policy::recovery_reference_matches(Some(id), Some("对象\0乙"), Some(path), path));
    expect(!policy::recovery_reference_matches(Some(id), Some(id), Some("目录/\0原件"), path));
    expect(!policy::recovery_reference_matches(None, Some(id), Some(path), path));
    expect(!policy::recovery_reference_matches(Some(id), None, Some(path), path));
    expect(!policy::recovery_reference_matches(Some(id), Some(id), None, path));
    expect(!policy::recovery_reference_matches(None, None, Some(path), path));
    expect(!policy::recovery_reference_matches(None, Some(id), None, path));
    expect(!policy::recovery_reference_matches(Some(id), None, None, path));
    expect(!policy::recovery_reference_matches(None, None, None, path));
    expect(policy::recovery_reference_matches(Some(""), Some(""), Some(""), ""));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_records_require_the_exact_schema_attempt_digest_and_null_exit() {
    proof {
        reveal_strlit("chrono-ci-generation-intent/v1");
        reveal_strlit("chrono-ci-generation-result/v1");
        reveal_strlit("chrono-ci-generation-intent/v1-extra");
        reveal_strlit("chrono-ci-generation-result/v1-extra");
        reveal_strlit("目录/原件\0\n"); reveal_strlit("目录/原件\0");
        reveal_strlit("digest"); reveal_strlit("Digest");
        reveal_strlit("");
        assert("chrono-ci-generation-intent/v1"@.len()
            != "chrono-ci-generation-intent/v1-extra"@.len());
        assert("chrono-ci-generation-result/v1"@.len()
            != "chrono-ci-generation-result/v1-extra"@.len());
        assert("目录/原件\0\n"@.len() != "目录/原件\0"@.len());
        assert("digest"@[0] != "Digest"@[0]);
    }
    let intent = Some("chrono-ci-generation-intent/v1");
    let result = Some("chrono-ci-generation-result/v1");
    let path = "目录/原件\0\n";
    expect(policy::intent_address_matches(intent, Some(path), path));
    expect(!policy::intent_address_matches(None, Some(path), path));
    expect(!policy::intent_address_matches(intent, None, path));
    expect(!policy::intent_address_matches(None, None, path));
    expect(!policy::intent_address_matches(
        Some("chrono-ci-generation-intent/v1-extra"), Some(path), path));
    expect(!policy::intent_address_matches(intent, Some("目录/原件\0"), path));
    expect(policy::intent_address_matches(intent, Some(""), ""));

    expect(policy::result_address_matches(result, Some(path), Some("digest"), path, "digest", true));
    expect(!policy::result_address_matches(None, Some(path), Some("digest"), path, "digest", true));
    expect(!policy::result_address_matches(result, None, Some("digest"), path, "digest", true));
    expect(!policy::result_address_matches(result, Some(path), None, path, "digest", true));
    expect(!policy::result_address_matches(None, None, None, path, "digest", true));
    expect(!policy::result_address_matches(
        Some("chrono-ci-generation-result/v1-extra"), Some(path), Some("digest"), path, "digest", true));
    expect(!policy::result_address_matches(result, Some("目录/原件\0"), Some("digest"), path, "digest", true));
    expect(!policy::result_address_matches(result, Some(path), Some("Digest"), path, "digest", true));
    expect(!policy::result_address_matches(result, Some(path), Some("digest"), path, "digest", false));
    expect(policy::result_address_matches(result, Some(""), Some(""), "", "", true));
}

fn installation_case(count: usize, fail_at: usize, write_at: usize)
    requires count <= 3, fail_at <= count, write_at <= count,
{
    let apply = |mut trace: Vec<usize>, index: usize| -> (out: (Vec<usize>, Result<bool, (usize, u64)>))
        requires index < count,
        ensures out.0@ == trace@.push(index),
            out.1 == if index == fail_at {
                Err::<bool, (usize, u64)>((index, u64::MAX))
            } else {
                Ok::<bool, (usize, u64)>(index == write_at)
            },
    {
        trace.push(index);
        (trace, if index == fail_at { Err((index, u64::MAX)) } else { Ok(index == write_at) })
    };
    let initial: Vec<usize> = Vec::new();
    let ghost before = initial;
    let (trace, result) = policy::install_targets(initial, count, apply);
    proof {
        let (states, changed) = choose|states: Seq<Vec<usize>>, changed: Seq<bool>|
            policy::installed_prefix(before, states, changed, apply) && changed.len() <= count
            && match result {
                Ok(any_changed) => changed.len() == count && trace == states.last()
                    && any_changed == changed.contains(true),
                Err(error) => changed.len() < count
                    && apply.ensures((states.last(), changed.len() as usize),
                        (trace, Err::<bool, (usize, u64)>(error))),
            };
        assert(states[0]@ == Seq::<usize>::empty());
        assert forall|index: int| 0 <= index < changed.len() implies
            changed[index] == (index == write_at) && index != fail_at by {
            assert(apply.ensures((states[index], index as usize),
                (states[index + 1], Ok::<bool, (usize, u64)>(changed[index]))));
        }
        if changed.len() > 0 {
            assert(apply.ensures((states[0], 0usize),
                (states[1], Ok::<bool, (usize, u64)>(changed[0]))));
            assert(states[1]@ == states[0]@.push(0usize));
            assert(states[1]@ == seq![0usize]);
        }
        if changed.len() > 1 {
            assert(apply.ensures((states[1], 1usize),
                (states[2], Ok::<bool, (usize, u64)>(changed[1]))));
            assert(states[2]@ == states[1]@.push(1usize));
            assert(states[2]@ == seq![0usize, 1usize]);
        }
        if changed.len() > 2 {
            assert(apply.ensures((states[2], 2usize),
                (states[3], Ok::<bool, (usize, u64)>(changed[2]))));
            assert(states[3]@ == states[2]@.push(2usize));
            assert(states[3]@ == seq![0usize, 1usize, 2usize]);
        }
        match result {
            Ok(any_changed) => {
                if fail_at < count { assert(changed[fail_at as int] == (fail_at == write_at)); }
                assert(fail_at == count);
                if write_at < count { assert(changed[write_at as int]); }
                assert(any_changed == (write_at < count));
                if count == 0 { assert(trace@ == Seq::<usize>::empty()); }
                if count == 1 { assert(trace@ == seq![0usize]); }
                if count == 2 { assert(trace@ == seq![0usize, 1usize]); }
                if count == 3 { assert(trace@ == seq![0usize, 1usize, 2usize]); }
            },
            Err(error) => {
                assert(apply.ensures((states.last(), changed.len() as usize),
                    (trace, Err::<bool, (usize, u64)>(error))));
                assert(changed.len() == fail_at);
                assert(error == (fail_at, u64::MAX));
                assert(fail_at < count);
                if fail_at == 0 { assert(trace@ == seq![0usize]); }
                if fail_at == 1 { assert(trace@ == seq![0usize, 1usize]); }
                if fail_at == 2 { assert(trace@ == seq![0usize, 1usize, 2usize]); }
            },
        }
    }
    expect(trace.len() == if fail_at < count { fail_at + 1 } else { count });
    if !trace.is_empty() { expect(trace[0] == 0); }
    if trace.len() > 1 { expect(trace[1] == 1); }
    if trace.len() > 2 { expect(trace[2] == 2); }
    match result {
        Err((index, source)) => {
            expect(fail_at < count);
            expect(index == fail_at);
            expect(source == u64::MAX);
        },
        Ok(changed) => {
            expect(fail_at == count);
            expect(changed == (write_at < count));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn target_installation_keeps_order_first_error_and_any_change_without_skipping_unchanged_targets() {
    installation_case(0, 0, 0);
    installation_case(1, 1, 0);
    installation_case(1, 1, 1);
    installation_case(1, 0, 0);
    installation_case(2, 2, 1);
    installation_case(3, 3, 0);
    installation_case(3, 3, 1);
    installation_case(3, 3, 2);
    installation_case(3, 3, 3);
    installation_case(3, 0, 0);
    installation_case(3, 1, 0);
    installation_case(3, 1, 2);
    installation_case(3, 2, 0);
    installation_case(3, 2, 2);
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn runtime_expectation_keeps_its_failure_branch() { expect(false); }
}
