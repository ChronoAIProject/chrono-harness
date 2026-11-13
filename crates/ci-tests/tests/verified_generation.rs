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

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn runtime_expectation_keeps_its_failure_branch() { expect(false); }
}
