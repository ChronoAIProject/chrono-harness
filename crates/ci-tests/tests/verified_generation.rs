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

fn original_binding_case(intent_schema: bool, intent_attempt: bool, host: bool, result_equal: bool,
    result_schema: bool, result_attempt: bool, result_digest: bool, null_exit: bool,
    identity_fails: bool, missing_host: bool)
{
    proof {
        reveal_strlit("chrono-ci-generation-intent/v1");
        reveal_strlit("chrono-ci-generation-result/v1");
        reveal_strlit("invalid"); reveal_strlit("路径\0原件\n"); reveal_strlit("路径");
        reveal_strlit("host\0原件\n"); reveal_strlit("host");
        reveal_strlit("digest\0原件\n"); reveal_strlit("wrong");
        assert("chrono-ci-generation-intent/v1"@.len() != "invalid"@.len());
        assert("chrono-ci-generation-result/v1"@.len() != "invalid"@.len());
        assert("路径\0原件\n"@.len() != "路径"@.len());
        assert("host\0原件\n"@.len() != "host"@.len());
        assert("digest\0原件\n"@.len() != "wrong"@.len());
    }
    let fields = policy::OriginalBinding {
        intent_schema: Some(if intent_schema { "chrono-ci-generation-intent/v1" } else { "invalid" }),
        intent_attempt: Some(if intent_attempt { "路径\0原件\n" } else { "路径" }),
        intent_host: if missing_host { None } else { Some(if host { "host\0原件\n" } else { "host" }) },
        result_schema: Some(if result_schema { "chrono-ci-generation-result/v1" } else { "invalid" }),
        result_attempt: Some(if result_attempt { "路径\0原件\n" } else { "路径" }),
        result_intent_sha256: Some(if result_digest { "digest\0原件\n" } else { "wrong" }),
        result_exit_is_null: null_exit,
    };
    let identify = |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<String, String>))
        ensures out.0@ == trace@.push(1), match out.1 {
            Err(error) => identity_fails && error@ == "identity\0failure\n"@,
            Ok(actual) => !identity_fails && actual@ == "host\0原件\n"@,
        },
    {
        trace.push(1);
        (trace, if identity_fails { Err("identity\0failure\n".to_owned()) }
            else { Ok("host\0原件\n".to_owned()) })
    };
    let equal = |mut trace: Vec<u8>| -> (out: (Vec<u8>, bool))
        ensures out.0@ == trace@.push(2), out.1 == result_equal,
    {
        trace.push(2);
        (trace, result_equal)
    };
    let hash = |mut trace: Vec<u8>, bytes: &Vec<u8>| -> (out: (Vec<u8>, String))
        ensures out.0@ == trace@.push(3), out.1@ == if bytes@.len() == 3
            && bytes@[0] == 0 && bytes@[1] == 255 && bytes@[2] == 7 {
            "digest\0原件\n"@
        } else { "wrong"@ },
    {
        let valid = bytes.len() == 3 && bytes[0] == 0 && bytes[1] == 255 && bytes[2] == 7;
        trace.push(3);
        (trace, if valid { "digest\0原件\n".to_owned() } else { "wrong".to_owned() })
    };
    let bytes = original_fixture(false);
    let initial: Vec<u8> = Vec::new();
    let ghost start = initial;
    let (trace, result) = policy::bind_generation_originals(initial, &fields, "路径\0原件\n", &bytes, identify, equal, hash);
    proof {
        if intent_schema && intent_attempt {
            let (identified, actual) = choose|state: Vec<u8>, actual: Result<String, String>|
                #[trigger] identify.ensures((start,), (state, actual)) && match actual {
                    Err(error) => (trace, result) == (state, Err::<(), String>(error)),
                    Ok(actual) => if !policy::original_binding_host(&fields, actual) {
                        trace == state && policy::rejected_original_binding(result)
                    } else {
                        exists|next: Vec<u8>, matched: bool|
                            #[trigger] equal.ensures((state,), (next, matched))
                                && policy::original_binding_equality_result(next, &fields, "路径\0原件\n", &bytes,
                                    matched, hash, (trace, result))
                    },
                };
            if !identity_fails && !missing_host && host {
                let (compared, matched) = choose|state: Vec<u8>, matched: bool|
                    #[trigger] equal.ensures((identified,), (state, matched))
                        && policy::original_binding_equality_result(state, &fields, "路径\0原件\n", &bytes,
                            matched, hash, (trace, result));
                if result_equal {
                    let (hashed, digest) = choose|state: Vec<u8>, digest: String|
                        #[trigger] hash.ensures((compared, &bytes), (state, digest))
                            && trace == state && policy::original_binding_digest_result(&fields, "路径\0原件\n", digest@, result);
                }
            }
        }
    }
    // Independent oracle: address, identity, host/result, then digest; every later check is excluded on failure.
    let length = if !intent_schema || !intent_attempt { 0usize }
        else if identity_fails || missing_host || !host { 1usize }
        else if !result_equal { 2usize } else { 3usize };
    expect(trace.len() == length);
    if length > 0 { expect(trace[0] == 1); }
    if length > 1 { expect(trace[1] == 2); }
    if length > 2 { expect(trace[2] == 3); }
    let success = intent_schema && intent_attempt && !identity_fails && !missing_host && host
        && result_equal && result_schema && result_attempt && result_digest && null_exit;
    match result {
        Ok(()) => expect(success),
        Err(error) => {
            expect(!success);
            expect(identifiers::equal_text(&error, if intent_schema && intent_attempt && identity_fails {
                "identity\0failure\n"
            } else { "generation original result/intent binding mismatch" }));
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_binding_preserves_short_circuit_order_original_bytes_and_identity_failures() {
    let mut flags = 0u16;
    while flags < 256
        invariant flags <= 256,
        decreases 256 - flags,
    {
        original_binding_case(flags & 1 == 0, flags & 2 == 0, flags & 4 == 0, flags & 8 == 0,
            flags & 16 == 0, flags & 32 == 0, flags & 64 == 0, flags & 128 == 0, false, false);
        original_binding_case(flags & 1 == 0, flags & 2 == 0, flags & 4 == 0, flags & 8 == 0,
            flags & 16 == 0, flags & 32 == 0, flags & 64 == 0, flags & 128 == 0, true, false);
        original_binding_case(flags & 1 == 0, flags & 2 == 0, flags & 4 == 0, flags & 8 == 0,
            flags & 16 == 0, flags & 32 == 0, flags & 64 == 0, flags & 128 == 0, false, true);
        original_binding_case(flags & 1 == 0, flags & 2 == 0, flags & 4 == 0, flags & 8 == 0,
            flags & 16 == 0, flags & 32 == 0, flags & 64 == 0, flags & 128 == 0, true, true);
        flags += 1;
    }
}

type TargetFault = (u8, u64);

spec fn bound_payload(bytes: Seq<u8>, empty: bool) -> bool {
    if empty { bytes.len() == 0 }
    else { bytes.len() == 3 && bytes[0] == 0 && bytes[1] == 255 && bytes[2] == 7 }
}

fn bound_original_case(empty: bool, missing: bool, corrupt: bool, fail_at: u8)
    requires fail_at <= 3,
{
    proof {
        reveal_strlit("hash\0原件\n"); reveal_strlit("wrong");
        assert("hash\0原件\n"@.len() != "wrong"@.len());
    }
    let io = policy::BoundOriginalIo {
        validate: |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(1), out.1 == staging_step_result(1, true, fail_at),
        { staging_step(trace, 1, true, fail_at) },
        observe: |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<Option<Vec<u8>>, TargetFault>))
            ensures out.0@ == trace@.push(2), match out.1 {
                Err(error) => fail_at == 2 && error == (2u8, u64::MAX),
                Ok(None) => fail_at != 2 && missing,
                Ok(Some(bytes)) => fail_at != 2 && !missing && bytes@ == original_bytes(empty),
            },
        {
            trace.push(2);
            (trace, if fail_at == 2 { Err((2, u64::MAX)) }
                else if missing { Ok(None) } else { Ok(Some(original_fixture(empty))) })
        },
        missing: |mut trace: Vec<u8>| -> (out: (Vec<u8>, TargetFault))
            ensures out.0@ == trace@.push(3), out.1 == (3u8, u64::MAX),
        { trace.push(3); (trace, (3, u64::MAX)) },
        hash: |mut trace: Vec<u8>, bytes: &Vec<u8>| -> (out: (Vec<u8>, String))
            ensures out.0@ == trace@.push(4), out.1@ == if bound_payload(bytes@, empty) && !corrupt {
                "hash\0原件\n"@
            } else { "wrong"@ },
        {
            let valid = if empty { bytes.is_empty() }
                else { bytes.len() == 3 && bytes[0] == 0 && bytes[1] == 255 && bytes[2] == 7 };
            trace.push(4);
            (trace, if valid && !corrupt { "hash\0原件\n".to_owned() } else { "wrong".to_owned() })
        },
        changed: |mut trace: Vec<u8>| -> (out: (Vec<u8>, TargetFault))
            ensures out.0@ == trace@.push(5), out.1 == (5u8, u64::MAX),
        { trace.push(5); (trace, (5, u64::MAX)) },
    };
    let initial: Vec<u8> = Vec::new();
    let ghost start = initial;
    let (trace, result) = policy::read_bound_original(initial, "hash\0原件\n", &io);
    proof {
        let (validated_state, validated) = choose|state: Vec<u8>, validated: Result<(), TargetFault>|
            #[trigger] io.validate.ensures((start,), (state, validated)) && match validated {
                Err(error) => (trace, result) == (state, Err::<Vec<u8>, TargetFault>(error)),
                Ok(()) => policy::bound_observed_result(state, "hash\0原件\n", &io, (trace, result)),
            };
        if fail_at != 1 {
            let (observed_state, observed) = choose|state: Vec<u8>, observed: Result<Option<Vec<u8>>, TargetFault>|
                #[trigger] io.observe.ensures((validated_state,), (state, observed)) && match observed {
                    Err(error) => (trace, result) == (state, Err::<Vec<u8>, TargetFault>(error)),
                    Ok(None) => match result {
                        Err(error) => io.missing.ensures((state,), (trace, error)),
                        _ => false,
                    },
                    Ok(Some(bytes)) => policy::bound_hashed_result(state, bytes, "hash\0原件\n", &io, (trace, result)),
                };
            if fail_at != 2 && !missing {
                let bytes = observed->Ok_0->Some_0;
                let (hashed_state, digest) = choose|state: Vec<u8>, digest: String|
                    #[trigger] io.hash.ensures((observed_state, &bytes), (state, digest)) && if digest@ == "hash\0原件\n"@ {
                        (trace, result) == (state, Ok::<Vec<u8>, TargetFault>(bytes))
                    } else { match result {
                        Err(error) => io.changed.ensures((state,), (trace, error)),
                        _ => false,
                    } };
            }
        }
    }
    // Independent oracle: validate, observe, then missing or hash; changed bytes stop after hash.
    let failure = if fail_at == 1 { 1u8 } else if fail_at == 2 { 2u8 }
        else if missing { 3u8 } else if corrupt { 5u8 } else { 0u8 };
    let length = if failure == 1 { 1usize } else if failure == 2 { 2usize }
        else if failure == 5 { 4usize } else { 3usize };
    expect(trace.len() == length);
    expect(trace[0] == 1);
    if length > 1 { expect(trace[1] == 2); }
    if length > 2 { expect(trace[2] == if missing { 3u8 } else { 4u8 }); }
    if length > 3 { expect(trace[3] == 5); }
    match result {
        Err(error) => { expect(failure != 0); expect(error.0 == failure); expect(error.1 == u64::MAX); },
        Ok(bytes) => {
            expect(failure == 0); expect(bytes.len() == if empty { 0usize } else { 3usize });
            if !empty { expect(bytes[0] == 0); expect(bytes[1] == 255); expect(bytes[2] == 7); }
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn bound_original_reading_preserves_complete_bytes_and_first_error() {
    let mut flags = 0u8;
    while flags < 8
        invariant flags <= 8,
        decreases 8 - flags,
    {
        bound_original_case(flags & 1 != 0, flags & 2 != 0, flags & 4 != 0, 0);
        bound_original_case(flags & 1 != 0, flags & 2 != 0, flags & 4 != 0, 1);
        bound_original_case(flags & 1 != 0, flags & 2 != 0, flags & 4 != 0, 2);
        bound_original_case(flags & 1 != 0, flags & 2 != 0, flags & 4 != 0, 3);
        flags += 1;
    }
}

type AncestorPath = (u64, u64);
type AncestorEvent = (u8, u64, u64);
type AncestorState = (u8, Vec<AncestorEvent>);

fn expect_ancestor_event(actual: AncestorEvent, expected: AncestorEvent)
    requires actual == expected,
{
    expect(actual.0 == expected.0);
    expect(actual.1 == expected.1);
    expect(actual.2 == expected.2);
}

fn ancestor_case(count: u8, root: AncestorPath, fail: bool, fail_at: u64)
    requires count <= 3,
{
    let next = |state: AncestorState| -> (out: (AncestorState, Option<AncestorPath>))
        ensures out.0.0 == if state.0 > 0 { state.0 - 1 } else { 0 },
            out.0.1@ == state.1@.push((1u8, u64::MAX, state.0 as u64)),
            out.1 == if state.0 > 0 { Some((u64::MAX, state.0 as u64)) } else { None },
    {
        let (remaining, mut trace) = state;
        trace.push((1, u64::MAX, remaining as u64));
        if remaining > 0 { ((remaining - 1, trace), Some((u64::MAX, remaining as u64))) }
        else { ((0, trace), None) }
    };
    let same = |path: &AncestorPath, root: &AncestorPath| -> (out: bool)
        ensures out == (path.0 == root.0 && path.1 == root.1),
    { path.0 == root.0 && path.1 == root.1 };
    let flush = |state: AncestorState, path: AncestorPath| -> (out: (AncestorState, Result<(), AncestorPath>))
        ensures out.0.0 == state.0,
            out.0.1@ == state.1@.push((2u8, path.0, path.1)),
            out.1 == if fail && path.1 == fail_at { Err::<(), AncestorPath>(path) } else { Ok::<(), AncestorPath>(()) },
    {
        let (remaining, mut trace) = state;
        trace.push((2, path.0, path.1));
        ((remaining, trace), if fail && path.1 == fail_at { Err(path) } else { Ok(()) })
    };
    proof {
        let rank = |state: AncestorState| state.0 as nat;
        assert(policy::directory_steps_finite(next, flush, rank));
    }
    let initial: AncestorState = (count, Vec::new());
    let ghost start = initial;
    let (state, result) = policy::sync_ancestor_directories(initial, root, next, same, flush);
    proof {
        let (states, steps, following) = choose|states: Seq<AncestorState>, steps: Seq<(AncestorState, AncestorPath)>,
            following: (AncestorState, Option<AncestorPath>)|
            policy::directory_prefix(start, root, states, steps, next, same, flush)
                && policy::directory_terminal(states.last(), root, following, next, same, flush, (state, result));
        assert(states[0] == start);
        if steps.len() > 0 {
            assert(next.ensures((states[0],), (steps[0].0, Some(steps[0].1))));
            assert(flush.ensures((steps[0].0, steps[0].1), (states[1], Ok::<(), AncestorPath>(()))));
        }
        if steps.len() > 1 {
            assert(next.ensures((states[1],), (steps[1].0, Some(steps[1].1))));
            assert(flush.ensures((steps[1].0, steps[1].1), (states[2], Ok::<(), AncestorPath>(()))));
        }
        if steps.len() > 2 {
            assert(next.ensures((states[2],), (steps[2].0, Some(steps[2].1))));
            assert(flush.ensures((steps[2].0, steps[2].1), (states[3], Ok::<(), AncestorPath>(()))));
        }
        if steps.len() > 3 {
            assert(next.ensures((states[3],), (steps[3].0, Some(steps[3].1))));
            assert(false);
        }
        assert(steps.len() <= 3);
        if steps.len() == 0 { assert(states.last() == states[0]); }
        else if steps.len() == 1 { assert(states.last() == states[1]); }
        else if steps.len() == 2 { assert(states.last() == states[2]); }
        else { assert(states.last() == states[3]); }
    }
    // A closed-form oracle: ancestors descend from count to the first root or failure.
    let stop = if root.0 == u64::MAX && 1 <= root.1 && root.1 <= count as u64 { root.1 as usize } else { 0usize };
    let ancestor_error = fail && (stop as u64) < fail_at && fail_at <= count as u64;
    let visited = if ancestor_error { count as usize - fail_at as usize + 1 } else { count as usize - stop };
    let length = 2 * visited + if ancestor_error { 0usize } else { 2usize };
    let (remaining, trace) = state;
    expect(trace.len() == length);
    if visited > 0 { expect_ancestor_event(trace[0], (1u8, u64::MAX, count as u64)); expect_ancestor_event(trace[1], (2u8, u64::MAX, count as u64)); }
    if visited > 1 { expect_ancestor_event(trace[2], (1u8, u64::MAX, count as u64 - 1)); expect_ancestor_event(trace[3], (2u8, u64::MAX, count as u64 - 1)); }
    if visited > 2 { expect_ancestor_event(trace[4], (1u8, u64::MAX, count as u64 - 2)); expect_ancestor_event(trace[5], (2u8, u64::MAX, count as u64 - 2)); }
    if !ancestor_error {
        expect_ancestor_event(trace[2 * visited], (1u8, u64::MAX, stop as u64));
        expect_ancestor_event(trace[2 * visited + 1], (2u8, root.0, root.1));
    }
    expect(remaining as u64 == if ancestor_error { fail_at - 1 } else if stop > 0 { stop as u64 - 1 } else { 0u64 });
    match result {
        Err(path) => {
            expect(ancestor_error || (fail && root.1 == fail_at));
            let expected = if ancestor_error { (u64::MAX, fail_at) } else { root };
            expect(path.0 == expected.0);
            expect(path.1 == expected.1);
        },
        Ok(()) => { expect(!ancestor_error && !(fail && root.1 == fail_at)); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ancestor_sync_streams_complete_paths_stops_at_root_and_keeps_first_error() {
    ancestor_case(0, (u64::MAX, 17), false, 0);
    ancestor_case(1, (u64::MAX, 17), false, 0);
    ancestor_case(3, (u64::MAX, 17), false, 0);
    ancestor_case(3, (u64::MAX, 3), false, 0);
    ancestor_case(3, (u64::MAX, 2), false, 0);
    ancestor_case(3, (u64::MAX, 1), false, 0);
    ancestor_case(3, (u64::MAX - 1, 2), false, 0);
    ancestor_case(3, (u64::MAX, 17), true, 3);
    ancestor_case(3, (u64::MAX, 17), true, 2);
    ancestor_case(3, (u64::MAX, 17), true, 1);
    ancestor_case(3, (u64::MAX, 17), true, 17);
    ancestor_case(3, (u64::MAX, 2), true, 2);
    ancestor_case(3, (u64::MAX, 2), true, 1);
    ancestor_case(3, (u64::MAX, 2), true, 3);
    ancestor_case(3, (u64::MAX, 3), true, 2);
    ancestor_case(3, (u64::MAX, 0), true, 0);
    ancestor_case(3, (u64::MAX, u64::MAX), true, u64::MAX);
    ancestor_case(3, (0, u64::MAX), true, u64::MAX);
    ancestor_case(1, (u64::MAX, 1), false, 0);
}

spec fn staging_step_result(step: u8, valid: bool, fail_at: u8) -> Result<(), TargetFault> {
    if !valid { Err::<(), TargetFault>((9u8, u64::MAX)) }
    else if step == fail_at { Err::<(), TargetFault>((step, u64::MAX)) }
    else { Ok::<(), TargetFault>(()) }
}

fn staging_step(mut trace: Vec<u8>, step: u8, valid: bool, fail_at: u8)
    -> (out: (Vec<u8>, Result<(), TargetFault>))
    ensures out.0@ == trace@.push(step), out.1 == staging_step_result(step, valid, fail_at),
{
    if !valid { trace.push(step); (trace, Err((9, u64::MAX))) }
    else { target_step(trace, step, fail_at) }
}

fn staging_case(fail_at: u8)
    requires fail_at <= 7,
{
    let stage = (u64::MAX, 17u64);
    let write = |trace: Vec<u8>, stage: &(u64, u64)| -> (out: (Vec<u8>, Result<(), TargetFault>))
        ensures out.0@ == trace@.push(1),
            out.1 == staging_step_result(1, stage.0 == u64::MAX && stage.1 == 17, fail_at),
    { staging_step(trace, 1, stage.0 == u64::MAX && stage.1 == 17, fail_at) };
    let identity = |trace: Vec<u8>, stage: &(u64, u64)|
        -> (out: (Vec<u8>, Result<(u64, u64), TargetFault>))
        ensures out.0@ == trace@.push(2),
            match out.1 {
                Err(error) => staging_step_result(2, stage.0 == u64::MAX && stage.1 == 17, fail_at)
                    == Err::<(), TargetFault>(error),
                Ok(value) => staging_step_result(2, stage.0 == u64::MAX && stage.1 == 17, fail_at)
                    == Ok::<(), TargetFault>(()) && value.0 == u64::MAX - 1 && value.1 == 18,
            },
    {
        let (trace, result) = staging_step(trace, 2, stage.0 == u64::MAX && stage.1 == 17, fail_at);
        (trace, match result { Err(error) => Err(error), Ok(()) => Ok((u64::MAX - 1, 18)) })
    };
    let receipt = |trace: Vec<u8>, stage: &(u64, u64), identity: (u64, u64)|
        -> (out: (Vec<u8>, Result<(), TargetFault>))
        ensures out.0@ == trace@.push(3),
            out.1 == staging_step_result(3, stage.0 == u64::MAX && stage.1 == 17
                && identity.0 == u64::MAX - 1 && identity.1 == 18, fail_at),
    {
        staging_step(trace, 3, stage.0 == u64::MAX && stage.1 == 17
            && identity.0 == u64::MAX - 1 && identity.1 == 18, fail_at)
    };
    let parent = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(u64, u64), TargetFault>))
        ensures out.0@ == trace@.push(4),
            out.1 == if fail_at == 4 { Err::<(u64, u64), TargetFault>((4u8, u64::MAX)) }
                else { Ok::<(u64, u64), TargetFault>(((u64::MAX - 2) as u64, 19u64)) },
    {
        let (trace, result) = target_step(trace, 4, fail_at);
        (trace, match result { Err(error) => Err(error), Ok(()) => Ok((u64::MAX - 2, 19)) })
    };
    let create = |trace: Vec<u8>, parent: &(u64, u64)| -> (out: (Vec<u8>, Result<(), TargetFault>))
        ensures out.0@ == trace@.push(5),
            out.1 == staging_step_result(5, parent.0 == u64::MAX - 2 && parent.1 == 19, fail_at),
    { staging_step(trace, 5, parent.0 == u64::MAX - 2 && parent.1 == 19, fail_at) };
    let sync = |trace: Vec<u8>, parent: &(u64, u64)| -> (out: (Vec<u8>, Result<(), TargetFault>))
        ensures out.0@ == trace@.push(6),
            out.1 == staging_step_result(6, parent.0 == u64::MAX - 2 && parent.1 == 19, fail_at),
    { staging_step(trace, 6, parent.0 == u64::MAX - 2 && parent.1 == 19, fail_at) };
    let initial: Vec<u8> = Vec::new();
    let ghost start = initial;
    let (trace, result) = policy::stage_target(initial, stage, write, identity, receipt, parent, create, sync);
    proof {
        let (written, observed) = choose|state: Vec<u8>, observed: Result<(), TargetFault>|
            #[trigger] write.ensures((start, &stage), (state, observed)) && match observed {
                Err(error) => (trace, result) == (state, Err::<((u64, u64), (u64, u64)), TargetFault>(error)),
                Ok(()) => policy::staging_identity_result(state, stage, identity, receipt, parent, create, sync, (trace, result)),
            };
        if fail_at != 1 {
            let (identified, observed) = choose|state: Vec<u8>, observed: Result<(u64, u64), TargetFault>|
                #[trigger] identity.ensures((written, &stage), (state, observed)) && match observed {
                    Err(error) => (trace, result) == (state, Err::<((u64, u64), (u64, u64)), TargetFault>(error)),
                    Ok(value) => exists|next: Vec<u8>, recorded: Result<(), TargetFault>|
                        #[trigger] receipt.ensures((state, &stage, value), (next, recorded)) && match recorded {
                            Err(error) => (trace, result) == (next, Err::<((u64, u64), (u64, u64)), TargetFault>(error)),
                            Ok(()) => policy::staging_parent_result(next, stage, parent, create, sync, (trace, result)),
                        },
                };
            if fail_at != 2 {
                let value = observed->Ok_0;
                let (recorded, observed) = choose|state: Vec<u8>, observed: Result<(), TargetFault>|
                    #[trigger] receipt.ensures((identified, &stage, value), (state, observed)) && match observed {
                        Err(error) => (trace, result) == (state, Err::<((u64, u64), (u64, u64)), TargetFault>(error)),
                        Ok(()) => policy::staging_parent_result(state, stage, parent, create, sync, (trace, result)),
                    };
                if fail_at != 3 {
                    let (obtained, observed) = choose|state: Vec<u8>, observed: Result<(u64, u64), TargetFault>|
                        #[trigger] parent.ensures((recorded,), (state, observed)) && match observed {
                            Err(error) => (trace, result) == (state, Err::<((u64, u64), (u64, u64)), TargetFault>(error)),
                            Ok(path) => exists|next: Vec<u8>, created: Result<(), TargetFault>|
                                #[trigger] create.ensures((state, &path), (next, created)) && match created {
                                    Err(error) => (trace, result) == (next, Err::<((u64, u64), (u64, u64)), TargetFault>(error)),
                                    Ok(()) => exists|last: Vec<u8>, synced: Result<(), TargetFault>|
                                        #[trigger] sync.ensures((next, &path), (last, synced)) && match synced {
                                            Err(error) => (trace, result) == (last, Err::<((u64, u64), (u64, u64)), TargetFault>(error)),
                                            Ok(()) => (trace, result) == (last, Ok::<((u64, u64), (u64, u64)), TargetFault>((stage, path))),
                                        },
                                },
                        };
                    if fail_at != 4 {
                        let path = observed->Ok_0;
                        let (created, observed) = choose|state: Vec<u8>, observed: Result<(), TargetFault>|
                            #[trigger] create.ensures((obtained, &path), (state, observed)) && match observed {
                                Err(error) => (trace, result) == (state, Err::<((u64, u64), (u64, u64)), TargetFault>(error)),
                                Ok(()) => exists|last: Vec<u8>, synced: Result<(), TargetFault>|
                                    #[trigger] sync.ensures((state, &path), (last, synced)) && match synced {
                                        Err(error) => (trace, result) == (last, Err::<((u64, u64), (u64, u64)), TargetFault>(error)),
                                        Ok(()) => (trace, result) == (last, Ok::<((u64, u64), (u64, u64)), TargetFault>((stage, path))),
                                    },
                            };
                        if fail_at != 5 {
                            let (synced, observed) = choose|state: Vec<u8>, observed: Result<(), TargetFault>|
                                #[trigger] sync.ensures((created, &path), (state, observed)) && match observed {
                                    Err(error) => (trace, result) == (state, Err::<((u64, u64), (u64, u64)), TargetFault>(error)),
                                    Ok(()) => (trace, result) == (state, Ok::<((u64, u64), (u64, u64)), TargetFault>((stage, path))),
                                };
                        }
                    }
                }
            }
        }
    }
    // Independent oracle: every successful prefix is exactly 1..=6, cut at its first error.
    let length = if 1 <= fail_at && fail_at <= 6 { fail_at as usize } else { 6usize };
    expect(trace.len() == length);
    expect(trace[0] == 1);
    if length > 1 { expect(trace[1] == 2); }
    if length > 2 { expect(trace[2] == 3); }
    if length > 3 { expect(trace[3] == 4); }
    if length > 4 { expect(trace[4] == 5); }
    if length > 5 { expect(trace[5] == 6); }
    match result {
        Err(error) => { expect(1 <= fail_at && fail_at <= 6); expect(error.0 == fail_at); expect(error.1 == u64::MAX); },
        Ok((saved_stage, saved_parent)) => {
            expect(fail_at == 0 || fail_at == 7);
            expect(saved_stage.0 == u64::MAX); expect(saved_stage.1 == 17);
            expect(saved_parent.0 == u64::MAX - 2); expect(saved_parent.1 == 19);
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn target_staging_keeps_complete_handles_identity_and_first_error_in_order() {
    staging_case(0); staging_case(1); staging_case(2); staging_case(3);
    staging_case(4); staging_case(5); staging_case(6); staging_case(7);
}

spec fn original_return_matches(result: Result<(u64, u64), TargetFault>, successful: bool) -> bool {
    match result {
        Ok(value) => successful && value.0 == u64::MAX && value.1 == 17,
        Err(error) => !successful && error == (7u8, u64::MAX),
    }
}

fn return_step(mut trace: Vec<u8>, original: &Result<(u64, u64), TargetFault>, successful: bool,
    handle_valid: bool, step: u8, fail_at: u8) -> (out: (Vec<u8>, Result<(), TargetFault>))
    ensures out.0@ == trace@.push(step),
        out.1 == staging_step_result(step, original_return_matches(*original, successful) && handle_valid, fail_at),
{
    let valid = handle_valid && match original {
        Ok(value) => successful && value.0 == u64::MAX && value.1 == 17,
        Err(error) => !successful && error.0 == 7 && error.1 == u64::MAX,
    };
    trace.push(step);
    (trace, if !valid { Err((9, u64::MAX)) }
        else if step == fail_at { Err((step, u64::MAX)) } else { Ok(()) })
}

fn return_case(mode: u8, successful: bool, fail_at: u8)
    requires mode <= 2, fail_at <= 7,
{
    let effects = if mode == 0 { policy::Effects::NotStarted }
        else if mode == 1 { policy::Effects::Unknown } else { policy::Effects::Complete };
    let original: Result<(u64, u64), TargetFault> = if successful { Ok((u64::MAX, 17)) }
        else { Err((7, u64::MAX)) };
    let publication = (u64::MAX - 1, 23u64);
    let io = policy::ReturnIo {
        observe: |trace: Vec<u8>, original: &Result<(u64, u64), TargetFault>| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(1),
                out.1 == staging_step_result(1, original_return_matches(*original, successful), fail_at),
        { return_step(trace, original, successful, true, 1, fail_at) },
        publish: |trace: Vec<u8>, publication: &(u64, u64), original: &Result<(u64, u64), TargetFault>| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(3),
                out.1 == staging_step_result(3, original_return_matches(*original, successful)
                    && publication.0 == u64::MAX - 1 && publication.1 == 23, fail_at),
        { return_step(trace, original, successful, publication.0 == u64::MAX - 1 && publication.1 == 23, 3, fail_at) },
        seal: |trace: Vec<u8>, publication: (u64, u64), original: &Result<(u64, u64), TargetFault>| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(5),
                out.1 == staging_step_result(5, original_return_matches(*original, successful)
                    && publication.0 == u64::MAX - 1 && publication.1 == 23, fail_at),
        { return_step(trace, original, successful, publication.0 == u64::MAX - 1 && publication.1 == 23, 5, fail_at) },
        release: |trace: Vec<u8>, original: &Result<(u64, u64), TargetFault>| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(6),
                out.1 == staging_step_result(6, original_return_matches(*original, successful), fail_at),
        { return_step(trace, original, successful, true, 6, fail_at) },
        notify: |mut trace: Vec<u8>, point: policy::ReturnCheckpoint| -> (out: Vec<u8>)
            ensures out@ == trace@.push(match point {
                policy::ReturnCheckpoint::Observed => 2u8,
                policy::ReturnCheckpoint::Published => 4u8,
            }),
        {
            let step = match point {
                policy::ReturnCheckpoint::Observed => {
                    expect(identifiers::equal_text(point.label(), "evaluated")); 2u8
                },
                policy::ReturnCheckpoint::Published => {
                    expect(identifiers::equal_text(point.label(), "result")); 4u8
                },
            };
            trace.push(step);
            trace
        },
    };
    let initial: Vec<u8> = Vec::new();
    let ghost start = initial;
    let (trace, result) = policy::finish_generation_return(initial, publication, original, &effects, &io);
    proof {
        let (observed_state, observed) = choose|state: Vec<u8>, observed: Result<(), TargetFault>|
            #[trigger] io.observe.ensures((start, &original), (state, observed)) && match observed {
                Err(error) => (trace, result) == (state, Err::<(u64, u64), TargetFault>(error)),
                Ok(()) => exists|next: Vec<u8>|
                    #[trigger] io.notify.ensures((state, policy::ReturnCheckpoint::Observed), next)
                        && policy::return_published_result(next, publication, original, &effects, &io, (trace, result)),
            };
        if fail_at != 1 {
            let notified = choose|next: Vec<u8>|
                #[trigger] io.notify.ensures((observed_state, policy::ReturnCheckpoint::Observed), next)
                    && policy::return_published_result(next, publication, original, &effects, &io, (trace, result));
            let (published_state, published) = choose|state: Vec<u8>, published: Result<(), TargetFault>|
                #[trigger] io.publish.ensures((notified, &publication, &original), (state, published)) && match published {
                    Err(error) => (trace, result) == (state, Err::<(u64, u64), TargetFault>(error)),
                    Ok(()) => exists|next: Vec<u8>|
                        #[trigger] io.notify.ensures((state, policy::ReturnCheckpoint::Published), next)
                            && policy::return_sealed_result(next, publication, original, &effects, &io, (trace, result)),
                };
            if fail_at != 3 {
                let notified = choose|next: Vec<u8>|
                    #[trigger] io.notify.ensures((published_state, policy::ReturnCheckpoint::Published), next)
                        && policy::return_sealed_result(next, publication, original, &effects, &io, (trace, result));
                let (sealed_state, sealed) = choose|state: Vec<u8>, sealed: Result<(), TargetFault>|
                    #[trigger] io.seal.ensures((notified, publication, &original), (state, sealed)) && match sealed {
                        Err(error) => (trace, result) == (state, Err::<(u64, u64), TargetFault>(error)),
                        Ok(()) => if effects is Unknown { (trace, result) == (state, original) }
                            else { exists|next: Vec<u8>, released: Result<(), TargetFault>|
                                #[trigger] io.release.ensures((state, &original), (next, released)) && match released {
                                    Err(error) => (trace, result) == (next, Err::<(u64, u64), TargetFault>(error)),
                                    Ok(()) => (trace, result) == (next, original),
                                }
                            },
                    };
                if fail_at != 5 && mode != 1 {
                    let (released_state, released) = choose|state: Vec<u8>, released: Result<(), TargetFault>|
                        #[trigger] io.release.ensures((sealed_state, &original), (state, released)) && match released {
                            Err(error) => (trace, result) == (state, Err::<(u64, u64), TargetFault>(error)),
                            Ok(()) => (trace, result) == (state, original),
                        };
                }
            }
        }
    }
    // Independent oracle: checkpoints follow their successful operations, and unknown effects retain recovery.
    let length = if fail_at == 1 { 1usize } else if fail_at == 3 { 3usize }
        else if fail_at == 5 || mode == 1 { 5usize } else { 6usize };
    let failure = if fail_at == 1 || fail_at == 3 || fail_at == 5 || (mode != 1 && fail_at == 6) {
        fail_at
    } else if successful { 0u8 } else { 7u8 };
    expect(trace.len() == length);
    expect(trace[0] == 1);
    if length > 1 { expect(trace[1] == 2); }
    if length > 2 { expect(trace[2] == 3); }
    if length > 3 { expect(trace[3] == 4); }
    if length > 4 { expect(trace[4] == 5); }
    if length > 5 { expect(trace[5] == 6); }
    match result {
        Err(error) => { expect(failure != 0); expect(error.0 == failure); expect(error.1 == u64::MAX); },
        Ok(value) => { expect(failure == 0); expect(value.0 == u64::MAX); expect(value.1 == 17); },
    }
}

fn return_failures_for_mode(mode: u8)
    requires mode <= 2,
{
    return_case(mode, false, 0); return_case(mode, false, 1);
    return_case(mode, false, 2); return_case(mode, false, 3);
    return_case(mode, false, 4); return_case(mode, false, 5);
    return_case(mode, false, 6); return_case(mode, false, 7);
    return_case(mode, true, 0); return_case(mode, true, 1);
    return_case(mode, true, 2); return_case(mode, true, 3);
    return_case(mode, true, 4); return_case(mode, true, 5);
    return_case(mode, true, 6); return_case(mode, true, 7);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn generation_return_preserves_original_result_checkpoints_and_recovery_on_every_failure() {
    return_failures_for_mode(0); return_failures_for_mode(1); return_failures_for_mode(2);
}

fn resume_case(mode: u8, present: bool, fail_at: u8)
    requires mode <= 3, fail_at <= 7,
{
    let effects = if mode == 0 { None } else if mode == 1 { Some("unknown") }
        else if mode == 2 { Some("unknown\0tail") } else { Some("complete") };
    proof {
        reveal_strlit("unknown");
        reveal_strlit("unknown\0tail");
        reveal_strlit("complete");
        assert("unknown\0tail"@.len() != "unknown"@.len());
        assert("complete"@.len() != "unknown"@.len());
        assert(policy::text_is(effects, "unknown") == (mode == 1));
    }
    let expected = (u64::MAX, 17u64);
    let publication = (u64::MAX - 1, 23u64);
    let io = policy::ResumeIo {
        probe: |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<bool, TargetFault>))
            ensures out.0@ == trace@.push(1),
                out.1 == if fail_at == 1 { Err::<bool, TargetFault>((1u8, u64::MAX)) }
                    else { Ok::<bool, TargetFault>(present) },
        {
            trace.push(1);
            (trace, if fail_at == 1 { Err((1, u64::MAX)) } else { Ok(present) })
        },
        write: |trace: Vec<u8>, publication: &(u64, u64), expected: &(u64, u64)|
            -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(2),
                out.1 == staging_step_result(2, publication.0 == u64::MAX - 1
                    && publication.1 == 23 && expected.0 == u64::MAX && expected.1 == 17, fail_at),
        {
            staging_step(trace, 2, publication.0 == u64::MAX - 1 && publication.1 == 23
                && expected.0 == u64::MAX && expected.1 == 17, fail_at)
        },
        check: |trace: Vec<u8>, expected: &(u64, u64)| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(3),
                out.1 == staging_step_result(3, expected.0 == u64::MAX && expected.1 == 17, fail_at),
        { staging_step(trace, 3, expected.0 == u64::MAX && expected.1 == 17, fail_at) },
        seal: |trace: Vec<u8>, publication: (u64, u64), expected: &(u64, u64)|
            -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(4),
                out.1 == staging_step_result(4, publication.0 == u64::MAX - 1
                    && publication.1 == 23 && expected.0 == u64::MAX && expected.1 == 17, fail_at),
        {
            staging_step(trace, 4, publication.0 == u64::MAX - 1 && publication.1 == 23
                && expected.0 == u64::MAX && expected.1 == 17, fail_at)
        },
        release: |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(5), out.1 == staging_step_result(5, true, fail_at),
        { staging_step(trace, 5, true, fail_at) },
        status: |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<(u64, u64), TargetFault>))
            ensures out.0@ == trace@.push(6),
                match out.1 {
                    Err(error) => fail_at == 6 && error == (6u8, u64::MAX),
                    Ok(value) => fail_at != 6 && value.0 == u64::MAX - 2 && value.1 == 29,
                },
        {
            trace.push(6);
            (trace, if fail_at == 6 { Err((6, u64::MAX)) } else { Ok((u64::MAX - 2, 29)) })
        },
    };
    let initial: Vec<u8> = Vec::new();
    let ghost start = initial;
    let (trace, result) = policy::resume_generation_result(initial, publication, &expected, effects, &io);
    proof {
        let (probed_state, probed) = choose|state: Vec<u8>, probed: Result<bool, TargetFault>|
            #[trigger] io.probe.ensures((start,), (state, probed)) && match probed {
                Err(error) => (trace, result) == (state, Err::<(u64, u64), TargetFault>(error)),
                Ok(exists) => policy::resume_published_result(state, publication, &expected, exists,
                    effects, &io, (trace, result)),
            };
        if fail_at != 1 {
            let checked_start = if present { probed_state } else {
                let (written_state, written) = choose|state: Vec<u8>, written: Result<(), TargetFault>|
                    #[trigger] io.write.ensures((probed_state, &publication, &expected), (state, written)) && match written {
                        Err(error) => (trace, result) == (state, Err::<(u64, u64), TargetFault>(error)),
                        Ok(()) => policy::resume_checked_result(state, publication, &expected, effects, &io, (trace, result)),
                    };
                written_state
            };
            if present || fail_at != 2 {
                let (checked_state, checked) = choose|state: Vec<u8>, checked: Result<(), TargetFault>|
                    #[trigger] io.check.ensures((checked_start, &expected), (state, checked)) && match checked {
                        Err(error) => (trace, result) == (state, Err::<(u64, u64), TargetFault>(error)),
                        Ok(()) => policy::resume_sealed_result(state, publication, &expected, effects, &io, (trace, result)),
                    };
                if fail_at != 3 {
                    let (sealed_state, sealed) = choose|state: Vec<u8>, sealed: Result<(), TargetFault>|
                        #[trigger] io.seal.ensures((checked_state, publication, &expected), (state, sealed)) && match sealed {
                            Err(error) => (trace, result) == (state, Err::<(u64, u64), TargetFault>(error)),
                            Ok(()) => if policy::text_is(effects, "unknown") {
                                io.status.ensures((state,), (trace, result))
                            } else {
                                exists|next: Vec<u8>, released: Result<(), TargetFault>|
                                    #[trigger] io.release.ensures((state,), (next, released)) && match released {
                                        Err(error) => (trace, result) == (next, Err::<(u64, u64), TargetFault>(error)),
                                        Ok(()) => io.status.ensures((next,), (trace, result)),
                                    }
                            },
                        };
                    if fail_at != 4 && mode != 1 {
                        let (released_state, released) = choose|state: Vec<u8>, released: Result<(), TargetFault>|
                            #[trigger] io.release.ensures((sealed_state,), (state, released)) && match released {
                                Err(error) => (trace, result) == (state, Err::<(u64, u64), TargetFault>(error)),
                                Ok(()) => io.status.ensures((state,), (trace, result)),
                            };
                    }
                }
            }
        }
    }
    // Independent oracle: existing results skip write, exact unknown retains the reference.
    let failure = if 1 <= fail_at && fail_at <= 6
        && (fail_at != 2 || !present) && (fail_at != 5 || mode != 1) { fail_at } else { 0u8 };
    let length = if failure != 0 {
        failure as usize - if present && failure > 2 { 1usize } else { 0usize }
            - if mode == 1 && failure > 5 { 1usize } else { 0usize }
    } else { 6usize - if present { 1usize } else { 0usize } - if mode == 1 { 1usize } else { 0usize } };
    expect(trace.len() == length);
    expect(trace[0] == 1);
    if length > 1 { expect(trace[1] == if present { 3u8 } else { 2u8 }); }
    if length > 2 { expect(trace[2] == if present { 4u8 } else { 3u8 }); }
    if length > 3 { expect(trace[3] == if present && mode == 1 { 6u8 } else if present { 5u8 } else { 4u8 }); }
    if length > 4 { expect(trace[4] == if present || mode == 1 { 6u8 } else { 5u8 }); }
    if length > 5 { expect(trace[5] == 6); }
    match result {
        Err(error) => { expect(failure != 0); expect(error.0 == failure); expect(error.1 == u64::MAX); },
        Ok(value) => { expect(failure == 0); expect(value.0 == u64::MAX - 2); expect(value.1 == 29); },
    }
}

fn resume_failures_for_mode(mode: u8)
    requires mode <= 3,
{
    resume_case(mode, false, 0); resume_case(mode, false, 1);
    resume_case(mode, false, 2); resume_case(mode, false, 3);
    resume_case(mode, false, 4); resume_case(mode, false, 5);
    resume_case(mode, false, 6); resume_case(mode, false, 7);
    resume_case(mode, true, 0); resume_case(mode, true, 1);
    resume_case(mode, true, 2); resume_case(mode, true, 3);
    resume_case(mode, true, 4); resume_case(mode, true, 5);
    resume_case(mode, true, 6); resume_case(mode, true, 7);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn generation_resume_preserves_original_payload_and_first_error_without_replaying_effects() {
    resume_failures_for_mode(0); resume_failures_for_mode(1);
    resume_failures_for_mode(2); resume_failures_for_mode(3);
}

spec fn target_observation_step(length: nat) -> u8 {
    if length == 0 { 1u8 } else if length == 2 { 3u8 } else { 7u8 }
}

spec fn target_object(missing: bool, step: u8, bad_at: u8, field: u8, value: Option<policy::Object>) -> bool {
    if (step == bad_at && field == 1 && (step == 7 || !missing))
        || (step != 7 && step != bad_at && missing) {
        value is None
    } else {
        match value {
            None => false,
            Some(object) => if step == 7 {
                object.identity@ == "published\0id"@ && object.bytes == u64::MAX
                    && object.sha256@ == if step == bad_at { "other\0hash"@ } else { "after\0hash"@ }
            } else {
                object.identity@ == if step == bad_at && field == 2 { "other\0id"@ } else { "prior\0id"@ }
                    && object.sha256@ == if step == bad_at && field == 3 { "other\0hash"@ } else { "prior\0hash"@ }
                    && object.bytes == if step == bad_at && field == 4 { 8u64 } else { 7u64 }
            },
        }
    }
}

fn target_object_fixture(missing: bool, step: u8, bad_at: u8, field: u8) -> (out: Option<policy::Object>)
    ensures target_object(missing, step, bad_at, field, out),
{
    if (step == bad_at && field == 1 && (step == 7 || !missing))
        || (step != 7 && step != bad_at && missing) { return None; }
    if step == 7 {
        Some(object("published\0id", if step == bad_at { "other\0hash" } else { "after\0hash" }, u64::MAX))
    } else {
        Some(object(if step == bad_at && field == 2 { "other\0id" } else { "prior\0id" },
            if step == bad_at && field == 3 { "other\0hash" } else { "prior\0hash" },
            if step == bad_at && field == 4 { 8 } else { 7 }))
    }
}

fn target_step(mut trace: Vec<u8>, step: u8, fail_at: u8) -> (out: (Vec<u8>, Result<(), TargetFault>))
    ensures out.0@ == trace@.push(step),
        out.1 == if step == fail_at { Err::<(), TargetFault>((step, u64::MAX)) } else { Ok::<(), TargetFault>(()) },
{
    trace.push(step);
    (trace, if step == fail_at { Err((step, u64::MAX)) } else { Ok(()) })
}

fn target_parent_step(mut trace: Vec<u8>, parent: (u64, u64), step: u8, fail_at: u8)
    -> (out: (Vec<u8>, Result<(), TargetFault>))
    ensures out.0@ == trace@.push(step),
        out.1 == if parent.0 != u64::MAX || parent.1 != 17 {
            Err::<(), TargetFault>((9u8, u64::MAX))
        } else if step == fail_at { Err::<(), TargetFault>((step, u64::MAX)) } else { Ok::<(), TargetFault>(()) },
{
    if parent.0 != u64::MAX || parent.1 != 17 {
        trace.push(step);
        (trace, Err((9, u64::MAX)))
    } else { target_step(trace, step, fail_at) }
}

spec fn target_effect_object(value: Option<policy::Object>) -> bool {
    match value {
        Some(object) => object.identity@ == "published\0id"@ && object.sha256@ == "after\0hash"@
            && object.bytes == u64::MAX,
        None => false,
    }
}

spec fn target_mismatch_step(reason: policy::TargetMismatch) -> u8 {
    match reason { policy::TargetMismatch::Prepared => 11u8,
        policy::TargetMismatch::Rename => 12u8, policy::TargetMismatch::Published => 13u8 }
}

fn target_case(missing: bool, write: bool, fail_at: u8, bad_at: u8, field: u8)
    requires bad_at == 0 || bad_at == 1 || bad_at == 3 || bad_at == 7,
        1 <= field <= 4,
{
    proof {
        reveal_strlit("prior\0id"); reveal_strlit("other\0id");
        reveal_strlit("prior\0hash"); reveal_strlit("other\0hash"); reveal_strlit("after\0hash");
        assert("prior\0id"@[0] != "other\0id"@[0]);
        assert("prior\0hash"@[0] != "other\0hash"@[0]);
        assert("after\0hash"@[0] != "other\0hash"@[0]);
    }
    let io = policy::TargetIo {
        observe: |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<Option<policy::Object>, TargetFault>))
            ensures out.0@ == trace@.push(target_observation_step(trace@.len())),
                match out.1 {
                    Err(error) => fail_at == target_observation_step(trace@.len())
                        && error == (target_observation_step(trace@.len()), u64::MAX),
                    Ok(value) => fail_at != target_observation_step(trace@.len())
                        && target_object(missing, target_observation_step(trace@.len()), bad_at, field, value),
                },
        {
            let step = if trace.is_empty() { 1u8 } else if trace.len() == 2 { 3u8 } else { 7u8 };
            trace.push(step);
            (trace, if step == fail_at { Err((step, u64::MAX)) }
                else { Ok(target_object_fixture(missing, step, bad_at, field)) })
        },
        stage: |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(u64, u64), TargetFault>))
            ensures out.0@ == trace@.push(2),
                out.1 == if fail_at == 2 { Err::<(u64, u64), TargetFault>((2u8, u64::MAX)) }
                    else { Ok::<(u64, u64), TargetFault>((u64::MAX, 17u64)) },
        {
            let (trace, result) = target_step(trace, 2, fail_at);
            (trace, match result { Err(error) => Err(error), Ok(()) => Ok((u64::MAX, 17)) })
        },
        create: |trace: Vec<u8>, parent: &(u64, u64)| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(4),
                out.1 == if parent.0 != u64::MAX || parent.1 != 17 { Err::<(), TargetFault>((9u8, u64::MAX)) }
                    else if fail_at == 4 { Err::<(), TargetFault>((4u8, u64::MAX)) } else { Ok::<(), TargetFault>(()) },
        { target_parent_step(trace, *parent, 4, fail_at) },
        replace: |trace: Vec<u8>, parent: &(u64, u64)| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(5),
                out.1 == if parent.0 != u64::MAX || parent.1 != 17 { Err::<(), TargetFault>((9u8, u64::MAX)) }
                    else if fail_at == 5 { Err::<(), TargetFault>((5u8, u64::MAX)) } else { Ok::<(), TargetFault>(()) },
        { target_parent_step(trace, *parent, 5, fail_at) },
        sync: |trace: Vec<u8>, parent: (u64, u64)| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(6),
                out.1 == if parent.0 != u64::MAX || parent.1 != 17 { Err::<(), TargetFault>((9u8, u64::MAX)) }
                    else if fail_at == 6 { Err::<(), TargetFault>((6u8, u64::MAX)) } else { Ok::<(), TargetFault>(()) },
        { target_parent_step(trace, parent, 6, fail_at) },
        effect: |mut trace: Vec<u8>, value: Option<policy::Object>| -> (out: (Vec<u8>, Result<(), TargetFault>))
            ensures out.0@ == trace@.push(8),
                out.1 == if !target_effect_object(value) { Err::<(), TargetFault>((9u8, u64::MAX)) }
                    else if fail_at == 8 { Err::<(), TargetFault>((8u8, u64::MAX)) } else { Ok::<(), TargetFault>(()) },
        {
            let valid = match value.as_ref() {
                Some(value) => identifiers::equal_text(&value.identity, "published\0id")
                    && identifiers::equal_text(&value.sha256, "after\0hash") && value.bytes == u64::MAX,
                None => false,
            };
            if !valid { trace.push(8); (trace, Err((9, u64::MAX))) }
            else { target_step(trace, 8, fail_at) }
        },
        mismatch: |mut trace: Vec<u8>, reason: policy::TargetMismatch| -> (out: (Vec<u8>, TargetFault))
            ensures out.0@ == trace@.push(target_mismatch_step(reason)),
                out.1 == (target_mismatch_step(reason), u64::MAX),
        {
            let step = match reason { policy::TargetMismatch::Prepared => 11u8,
                policy::TargetMismatch::Rename => 12u8, policy::TargetMismatch::Published => 13u8 };
            trace.push(step);
            (trace, (step, u64::MAX))
        },
    };
    let before = if missing { None } else { Some(object("prior\0id", "prior\0hash", 7)) };
    let before_ref = before.as_ref();
    let initial: Vec<u8> = Vec::new();
    let ghost start = initial;
    let (trace, result) = policy::install_target(initial, before_ref, write, "after\0hash", &io);
    proof {
        let (first, observed) = choose|state: Vec<u8>, observed: Result<Option<policy::Object>, TargetFault>|
            #[trigger] io.observe.ensures((start,), (state, observed)) && match observed {
                Err(error) => (trace, result) == (state, Err::<bool, TargetFault>(error)),
                Ok(value) => if !policy::same_object_spec(policy::borrowed_object(&value), before_ref) {
                    match result { Err(error) => io.mismatch.ensures((state, policy::TargetMismatch::Prepared), (trace, error)), _ => false }
                } else if !write { (trace, result) == (state, Ok::<bool, TargetFault>(false)) }
                else { exists|next: Vec<u8>, staged: Result<(u64, u64), TargetFault>|
                    #[trigger] io.stage.ensures((state,), (next, staged)) && match staged {
                        Err(error) => (trace, result) == (next, Err::<bool, TargetFault>(error)),
                        Ok(parent) => policy::target_staged_result(next, parent, before_ref, "after\0hash", &io, (trace, result)),
                    }
                },
            };
        if fail_at != 1 && bad_at != 1 && write {
            let (staged_state, staged) = choose|next: Vec<u8>, staged: Result<(u64, u64), TargetFault>|
                #[trigger] io.stage.ensures((first,), (next, staged)) && match staged {
                    Err(error) => (trace, result) == (next, Err::<bool, TargetFault>(error)),
                    Ok(parent) => policy::target_staged_result(next, parent, before_ref, "after\0hash", &io, (trace, result)),
                };
            if fail_at != 2 {
                let parent = staged->Ok_0;
                let (checked, observed) = choose|state: Vec<u8>, observed: Result<Option<policy::Object>, TargetFault>|
                    #[trigger] io.observe.ensures((staged_state,), (state, observed)) && match observed {
                        Err(error) => (trace, result) == (state, Err::<bool, TargetFault>(error)),
                        Ok(value) => if !policy::same_object_spec(policy::borrowed_object(&value), before_ref) {
                            match result { Err(error) => io.mismatch.ensures((state, policy::TargetMismatch::Rename), (trace, error)), _ => false }
                        } else { policy::target_publish_result(state, parent, missing, "after\0hash", &io, (trace, result)) },
                    };
                if fail_at != 3 && bad_at != 3 {
                    let (published_state, published) = choose|state: Vec<u8>, published: Result<(), TargetFault>|
                        #[trigger] policy::target_publication_step(checked, parent, missing,
                            io.create, io.replace, (state, published))
                        && match published {
                            Err(error) => (trace, result) == (state, Err::<bool, TargetFault>(error)),
                            Ok(()) => exists|next: Vec<u8>, synced: Result<(), TargetFault>|
                                #[trigger] io.sync.ensures((state, parent), (next, synced)) && match synced {
                                    Err(error) => (trace, result) == (next, Err::<bool, TargetFault>(error)),
                                    Ok(()) => policy::target_final_result(next, "after\0hash", &io, (trace, result)),
                                },
                        };
                    if fail_at != (if missing { 4u8 } else { 5u8 }) {
                        let (synced_state, synced) = choose|next: Vec<u8>, synced: Result<(), TargetFault>|
                            #[trigger] io.sync.ensures((published_state, parent), (next, synced)) && match synced {
                                Err(error) => (trace, result) == (next, Err::<bool, TargetFault>(error)),
                                Ok(()) => policy::target_final_result(next, "after\0hash", &io, (trace, result)),
                            };
                        if fail_at != 6 {
                            let (last, observed) = choose|state: Vec<u8>, observed: Result<Option<policy::Object>, TargetFault>|
                                #[trigger] io.observe.ensures((synced_state,), (state, observed)) && match observed {
                                    Err(error) => (trace, result) == (state, Err::<bool, TargetFault>(error)),
                                    Ok(value) => if !policy::published_digest(policy::borrowed_object(&value), "after\0hash") {
                                        match result { Err(error) => io.mismatch.ensures((state, policy::TargetMismatch::Published), (trace, error)), _ => false }
                                    } else { exists|effect: (Vec<u8>, Result<(), TargetFault>)|
                                        #[trigger] io.effect.ensures((state, value), effect) && policy::target_effect_result(effect, (trace, result)) },
                                };
                            if fail_at != 7 && bad_at != 7 {
                                let value = observed->Ok_0;
                                let effect = choose|effect: (Vec<u8>, Result<(), TargetFault>)|
                                    #[trigger] io.effect.ensures((last, value), effect) && policy::target_effect_result(effect, (trace, result));
                            }
                        }
                    }
                }
            }
        }
    }
    let publish_step = if missing { 4u8 } else { 5u8 };
    let expected_error = if fail_at == 1 { 1u8 } else if bad_at == 1 { 11u8 }
        else if !write { 0u8 } else if fail_at == 2 { 2u8 }
        else if fail_at == 3 { 3u8 } else if bad_at == 3 { 12u8 }
        else if fail_at == publish_step { publish_step } else if fail_at == 6 { 6u8 }
        else if fail_at == 7 { 7u8 } else if bad_at == 7 { 13u8 }
        else if fail_at == 8 { 8u8 } else { 0u8 };
    let length = if expected_error == 1 || (expected_error == 0 && !write) { 1usize }
        else if expected_error == 11 || expected_error == 2 { 2usize }
        else if expected_error == 3 { 3usize }
        else if expected_error == 12 || expected_error == publish_step { 4usize }
        else if expected_error == 6 { 5usize } else if expected_error == 7 { 6usize } else { 7usize };
    expect(trace.len() == length); expect(trace[0] == 1);
    if length > 1 { expect(trace[1] == if expected_error == 11 { 11u8 } else { 2u8 }); }
    if length > 2 { expect(trace[2] == 3); }
    if length > 3 { expect(trace[3] == if expected_error == 12 { 12u8 } else { publish_step }); }
    if length > 4 { expect(trace[4] == 6); }
    if length > 5 { expect(trace[5] == 7); }
    if length > 6 { expect(trace[6] == if expected_error == 13 { 13u8 } else { 8u8 }); }
    match result {
        Err((step, original)) => { expect(expected_error != 0); expect(step == expected_error); expect(original == u64::MAX); },
        Ok(changed) => { expect(expected_error == 0); expect(changed == write); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn single_target_publication_checks_before_effects_and_keeps_complete_first_failure() {
    expect(identifiers::equal_text(policy::TargetMismatch::Prepared.label(), "generation output changed after preparation"));
    expect(identifiers::equal_text(policy::TargetMismatch::Rename.label(), "generation output changed before rename"));
    expect(identifiers::equal_text(policy::TargetMismatch::Published.label(), "generation output differs after publication"));
    target_case(true, true, 0, 0, 1); target_case(false, true, 0, 0, 1);
    target_case(true, false, 2, 7, 1); target_case(false, false, 2, 7, 1);
    target_case(false, false, 1, 1, 2); target_case(false, false, 0, 1, 2);
    target_case(true, true, 1, 1, 1); target_case(false, true, 2, 3, 1);
    target_case(true, true, 3, 3, 1); target_case(false, true, 3, 3, 2);
    target_case(true, true, 4, 0, 1); target_case(false, true, 5, 0, 1);
    target_case(true, true, 5, 0, 1); target_case(false, true, 4, 0, 1);
    target_case(true, true, 6, 7, 1); target_case(false, true, 6, 7, 3);
    target_case(true, true, 7, 7, 1); target_case(false, true, 7, 7, 3);
    target_case(true, true, 8, 0, 1); target_case(false, true, 8, 0, 1);
    target_case(true, true, 0, 1, 1); target_case(true, true, 0, 3, 1);
    target_case(false, true, 0, 1, 1); target_case(false, true, 0, 1, 2);
    target_case(false, true, 0, 1, 3); target_case(false, true, 0, 1, 4);
    target_case(false, true, 0, 3, 1); target_case(false, true, 0, 3, 2);
    target_case(false, true, 0, 3, 3); target_case(false, true, 0, 3, 4);
    target_case(true, true, 0, 7, 1); target_case(true, true, 0, 7, 3);
    target_case(false, true, 0, 7, 1); target_case(false, true, 0, 7, 3);
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
