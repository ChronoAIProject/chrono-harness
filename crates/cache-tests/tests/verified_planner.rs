//! 当前缓存格式与 planner 身份的独立成功、拒绝与完整文本边界预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../cache/src/verified_planner.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_cache::verified_planner as policy;
use policy::content_preflight as content;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

#[cfg(verus_keep_ghost)]
#[path = "continuity.rs"]
mod continuity;

#[cfg(verus_keep_ghost)]
#[path = "support/probe.rs"]
mod probe;

verus! {

enum ExpectedFinalError {
    Publication(Option<String>, String),
    Recovery(String, String),
    Encoding(String),
}

fn recovery_finalization_case(retain_ok: bool, pointer_ok: bool, recovery: Option<&str>,
    warn: bool, encode_ok: bool, path: &str, raw_error: &str)
{
    let retain = |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<String, String>))
        ensures out.0@ == trace@.push(0u8), match out.1 {
            Ok(original) => retain_ok && original@ == path@,
            Err(error) => !retain_ok && error@ == raw_error@,
        },
    {
        trace.push(0);
        (trace, if retain_ok { Ok(path.to_string()) } else { Err(raw_error.to_string()) })
    };
    let pointer = |mut trace: Vec<u8>, original: &String|
        -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1u8), match out.1 {
            Ok(()) => pointer_ok,
            Err(error) => !pointer_ok && error@ == raw_error@,
        },
    {
        trace.push(1);
        (trace, if pointer_ok { Ok(()) } else { Err(raw_error.to_string()) })
    };
    let warning = |mut trace: Vec<u8>, original: &String| -> (out: Vec<u8>)
        ensures out@ == trace@.push(2u8),
    {
        trace.push(2);
        trace
    };
    let encode = |mut trace: Vec<u8>, original: String|
        -> (out: (Vec<u8>, Result<String, ExpectedFinalError>))
        ensures out.0@ == trace@.push(3u8), match out.1 {
            Ok(text) => encode_ok && text == original,
            Err(error) => !encode_ok && match error {
                ExpectedFinalError::Encoding(error) => error@ == raw_error@,
                _ => false,
            },
        },
    {
        trace.push(3);
        (trace, if encode_ok { Ok(original) }
            else { Err(ExpectedFinalError::Encoding(raw_error.to_string())) })
    };
    let publication_error = |recovery: Option<String>, publication: String|
        -> (out: ExpectedFinalError)
        ensures out == ExpectedFinalError::Publication(recovery, publication),
    { ExpectedFinalError::Publication(recovery, publication) };
    let recovery_error = |error: String, original: String| -> (out: ExpectedFinalError)
        ensures out == ExpectedFinalError::Recovery(error, original),
    { ExpectedFinalError::Recovery(error, original) };
    let result = match recovery {
        Some(error) => Err(error.to_string()), None => Ok(()),
    };
    let (trace, result) = policy::finalize_recovery(Vec::new(), result, warn,
        retain, pointer, warning, encode, &publication_error, &recovery_error);
    assert(trace@ == if !retain_ok { seq![0u8] }
        else if !pointer_ok || recovery is Some { seq![0u8, 1u8] }
        else if warn { seq![0u8, 1u8, 2u8, 3u8] } else { seq![0u8, 1u8, 3u8] });
    expect_cache_true(trace[0] == 0);
    if !retain_ok {
        expect_cache_true(trace.len() == 1);
    } else {
        expect_cache_true(trace[1] == 1);
        if !pointer_ok || recovery.is_some() {
            expect_cache_true(trace.len() == 2);
        } else if warn {
            expect_cache_true(trace.len() == 4);
            expect_cache_true(trace[2] == 2);
            expect_cache_true(trace[3] == 3);
        } else {
            expect_cache_true(trace.len() == 3);
            expect_cache_true(trace[2] == 3);
        }
    }
    if !retain_ok || !pointer_ok {
        match result {
            Err(ExpectedFinalError::Publication(failed, publication)) => {
                expect_restore_text(publication.as_str(), raw_error);
                match failed {
                    Some(error) => match recovery {
                        Some(expected) => expect_restore_text(error.as_str(), expected),
                        None => expect_cache_true(false),
                    },
                    None => expect_cache_true(recovery.is_none()),
                }
            },
            _ => expect_cache_true(false),
        }
    } else if let Some(expected) = recovery {
        match result {
            Err(ExpectedFinalError::Recovery(error, original)) => {
                expect_restore_text(error.as_str(), expected);
                expect_restore_text(original.as_str(), path);
            },
            _ => expect_cache_true(false),
        }
    } else if !encode_ok {
        match result {
            Err(ExpectedFinalError::Encoding(error)) => expect_restore_text(error.as_str(), raw_error),
            _ => expect_cache_true(false),
        }
    } else {
        match result {
            Ok(text) => expect_restore_text(text.as_str(), path),
            _ => expect_cache_true(false),
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_finalization_stops_at_retention_or_pointer_failure() {
    recovery_finalization_case(false, false, None, false, false, "unused", "retain denied");
    recovery_finalization_case(false, true, Some("restore/λverus! {

\n"), true, true,
        "unused", "retain/λverus! {

\n");
    recovery_finalization_case(true, false, None, true, true, "original", "pointer denied");
    recovery_finalization_case(true, false, Some(""), false, false, "original/λverus! {

\n", "");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_finalization_preserves_recovery_error_after_publication() {
    recovery_finalization_case(true, true, Some("restore denied"), false, true,
        "original", "unused");
    recovery_finalization_case(true, true, Some("restore/λverus! {

\n"), true, false,
        "original/λverus! {

\n", "unused");
    recovery_finalization_case(true, true, Some(""), true, true, "", "");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_finalization_warns_before_encoding_and_keeps_encoding_failure() {
    recovery_finalization_case(true, true, None, false, true, "original", "unused");
    recovery_finalization_case(true, true, None, true, true, "original/λverus! {

\n", "unused");
    recovery_finalization_case(true, true, None, false, false, "original", "encode denied");
    recovery_finalization_case(true, true, None, true, false, "", "encode/λverus! {

\n");
}

pub open spec fn removal_event(trace: Seq<(u8, usize, String)>, previous: Seq<(u8, usize, String)>,
    kind: u8, index: usize, payload: Seq<char>) -> bool {
    trace.len() == previous.len() + 1 && trace.drop_last() == previous
        && trace.last().0 == kind && trace.last().1 == index && trace.last().2@ == payload
}

pub open spec fn removal_trace_prefix(trace: Seq<(u8, usize, String)>, count: int) -> bool {
    0 <= count && trace.len() == 1 + 2 * count
        && trace[0].0 == 0 && trace[0].1 == 0 && trace[0].2@ == ""@
        && forall|i: int| 0 <= i < count ==>
            #[trigger] trace[1 + 2 * i].0 == 1 && trace[1 + 2 * i].1 == i
                && trace[1 + 2 * i].2@ == ""@
                && trace[2 + 2 * i].0 == 2 && trace[2 + 2 * i].1 == i
                && trace[2 + 2 * i].2@ == if i % 2 == 0 { "removed"@ } else { "absent"@ }
}

pub open spec fn removal_success(next: Seq<(u8, usize, String)>,
    previous: Seq<(u8, usize, String)>, index: int) -> bool {
    next.len() == previous.len() + 2 && next.subrange(0, previous.len() as int) == previous
        && next[previous.len() as int].0 == 1 && next[previous.len() as int].1 == index
        && next[previous.len() as int].2@ == ""@
        && next[previous.len() as int + 1].0 == 2 && next[previous.len() as int + 1].1 == index
        && next[previous.len() as int + 1].2@ == if index % 2 == 0 { "removed"@ } else { "absent"@ }
}

proof fn removal_trace_step(previous: Seq<(u8, usize, String)>, next: Seq<(u8, usize, String)>, end: int)
    requires 0 < end, removal_trace_prefix(previous, end - 1),
        removal_success(next, previous, end - 1),
    ensures removal_trace_prefix(next, end),
{
        assert forall|i: int| 0 <= i < end implies
            #[trigger] next[1 + 2 * i].0 == 1 && next[1 + 2 * i].1 == i
                && next[1 + 2 * i].2@ == ""@
                && next[2 + 2 * i].0 == 2 && next[2 + 2 * i].1 == i
                && next[2 + 2 * i].2@ == if i % 2 == 0 { "removed"@ } else { "absent"@ } by {
            if i < end - 1 {
                assert(previous[1 + 2 * i].0 == 1);
                assert(previous[2 + 2 * i].2@ == if i % 2 == 0 { "removed"@ } else { "absent"@ });
                assert(next[1 + 2 * i] == previous[1 + 2 * i]);
                assert(next[2 + 2 * i] == previous[2 + 2 * i]);
            }
        }
}

fn recovery_removals_case(paths: &Vec<(String, usize, String)>, fail_at: usize,
    raw_error: &str, publish_ok: bool)
    requires paths.len() <= 3, fail_at <= paths.len(),
{
    let trace = Vec::<(u8, usize, String)>::new();
    let ghost initial = trace;
    let publish = |mut trace: Vec<(u8, usize, String)>|
        -> (out: (Vec<(u8, usize, String)>, Result<(), String>))
        ensures removal_event(out.0@, trace@, 0u8, 0usize, ""@),
            if publish_ok { out.1 == Ok::<(), String>(()) }
            else { match out.1 { Err(error) => error@ == raw_error@, _ => false } },
    {
        let ghost previous = trace@;
        trace.push((0, 0, "".to_string()));
        assert(trace@.drop_last() =~= previous);
        (trace, if publish_ok { Ok(()) } else { Err(raw_error.to_string()) })
    };
    let remove = |mut trace: Vec<(u8, usize, String)>, index: usize|
        -> (out: (Vec<(u8, usize, String)>, Result<&'static str, String>))
        requires index < paths.len(),
        ensures removal_event(out.0@, trace@, 1u8, index, ""@),
            if index == fail_at { match out.1 { Err(error) => error@ == raw_error@, _ => false } }
            else { match out.1 {
                Ok(status) => status@ == if index % 2 == 0 { "removed"@ } else { "absent"@ },
                _ => false,
            } },
    {
        let ghost previous = trace@;
        trace.push((1, index, "".to_string()));
        assert(trace@.drop_last() =~= previous);
        (trace, if index == fail_at { Err(raw_error.to_string()) }
            else if index % 2 == 0 { Ok("removed") } else { Ok("absent") })
    };
    let success = |mut trace: Vec<(u8, usize, String)>, index: usize, status: &'static str|
        -> (out: Vec<(u8, usize, String)>)
        ensures removal_event(out@, trace@, 2u8, index, status@),
    {
        let ghost previous = trace@;
        trace.push((2, index, status.to_string()));
        assert(trace@.drop_last() =~= previous);
        trace
    };
    let failure = |mut trace: Vec<(u8, usize, String)>, index: usize, error: &String|
        -> (out: Vec<(u8, usize, String)>)
        ensures removal_event(out@, trace@, 3u8, index, error@),
    {
        let ghost previous = trace@;
        trace.push((3, index, error.clone()));
        assert(trace@.drop_last() =~= previous);
        trace
    };
    let (trace, result) = policy::run_recovery_removals(
        trace, paths, publish, &remove, &success, &failure);
    proof {
        let (published, published_result) = choose|published: Vec<(u8, usize, String)>,
            published_result: Result<(), String>|
            #[trigger] publish.ensures((initial,), (published, published_result))
            && match published_result {
                Err(error) => trace == published && match result {
                    Err(parts) => parts.0@ == ""@ && parts.1 == error
                        && parts.2@ == ""@ && parts.3@ == ""@,
                    _ => false,
                },
                Ok(()) => policy::recovery_paths_result(paths@, published,
                    &remove, &success, &failure, (trace, result)),
            };
        if publish_ok {
            let states = choose|states: Seq<Vec<(u8, usize, String)>>|
                #[trigger] policy::recovery_removal_prefix(paths@, states, published,
                    &remove, &success, &failure)
                && match result {
                    Ok(()) => states.len() == paths.len() + 1 && trace == states.last(),
                    Err(error) => states.len() <= paths.len()
                        && policy::recovery_removal_step(states.last(),
                            (states.len() - 1) as usize, &paths@[states.len() - 1].2,
                            &remove, &success, &failure, (trace, Err::<(), policy::RecoveryRemovalError>(error))),
                };
            assert forall|i: int| 0 <= i < states.len() - 1 implies i != fail_at
                && removal_success(states[i + 1]@, #[trigger] states[i]@, i) by {
                assert(policy::recovery_removal_step(states[i], i as usize, &paths@[i].2,
                    &remove, &success, &failure, (states[i + 1], Ok::<(), policy::RecoveryRemovalError>(()))));
                let (observed, removed) = choose|observed: Vec<(u8, usize, String)>,
                    removed: Result<&'static str, String>|
                    #[trigger] remove.ensures((states[i], i as usize), (observed, removed))
                    && match removed {
                        Ok(status) => success.ensures((observed, i as usize, status), states[i + 1]),
                        Err(_) => false,
                    };
                assert(i != fail_at);
                assert(states[i + 1]@.subrange(0, states[i]@.len() as int) =~= states[i]@) by {
                    assert forall|k: int| 0 <= k < states[i]@.len() implies
                        states[i + 1]@[k] == states[i]@[k] by {
                        assert(observed@[k] == states[i]@[k]);
                        assert(states[i + 1]@[k] == observed@[k]);
                    }
                }
                assert(removal_success(states[i + 1]@, states[i]@, i));
            }
            let i = states.len() - 1;
            if i > 0 {
                assert(removal_success(states[1]@, states[0]@, 0));
                removal_trace_step(states[0]@, states[1]@, 1);
            }
            if i > 1 {
                assert(removal_success(states[2]@, states[1]@, 1));
                removal_trace_step(states[1]@, states[2]@, 2);
            }
            if i > 2 {
                assert(removal_success(states[3]@, states[2]@, 2));
                removal_trace_step(states[2]@, states[3]@, 3);
            }
            assert(removal_trace_prefix(states.last()@, i));
            match &result {
                Ok(()) => {
                    if fail_at < paths.len() {
                        assert(removal_success(states[fail_at as int + 1]@,
                            states[fail_at as int]@, fail_at as int));
                    }
                    assert(fail_at == paths.len());
                    assert(removal_trace_prefix(trace@, paths.len() as int));
                },
                Err(error) => {
                    let (observed, removed) = choose|observed: Vec<(u8, usize, String)>,
                        removed: Result<&'static str, String>|
                        #[trigger] remove.ensures((states.last(), i as usize), (observed, removed))
                        && match removed {
                            Ok(_) => false,
                            Err(original) => failure.ensures((observed, i as usize, &original), trace)
                                && error.0@ == "E_CACHE_RECOVERY: "@ && error.1 == paths@[i].2
                                && error.2@ == ": "@ && error.3 == original,
                        };
                    assert(i == fail_at);
                    assert(trace@.len() == 3 + 2 * fail_at);
                    assert(trace@[1 + 2 * fail_at].0 == 1);
                    assert(trace@[2 + 2 * fail_at].0 == 3);
                    assert(trace@[2 + 2 * fail_at].1 == fail_at);
                    assert(trace@[2 + 2 * fail_at].2@ == raw_error@);
                    assert forall|j: int| 0 <= j < fail_at implies
                        #[trigger] trace@[1 + 2 * j].0 == 1 && trace@[1 + 2 * j].1 == j
                            && trace@[2 + 2 * j].0 == 2 && trace@[2 + 2 * j].1 == j
                            && trace@[2 + 2 * j].2@ == if j % 2 == 0 { "removed"@ } else { "absent"@ } by {
                        assert(states.last()@[1 + 2 * j].0 == 1);
                        assert(states.last()@[2 + 2 * j].2@ == if j % 2 == 0 { "removed"@ } else { "absent"@ });
                        assert(trace@[1 + 2 * j] == states.last()@[1 + 2 * j]);
                        assert(trace@[2 + 2 * j] == states.last()@[2 + 2 * j]);
                    }
                },
            }
        }
    }
    if !publish_ok {
        expect_cache_true(trace.len() == 1);
        expect_cache_true(trace[0].0 == 0);
        match result { Err(parts) => {
                expect_restore_text(parts.0, "");
                expect_restore_text(parts.1.as_str(), raw_error);
                expect_restore_text(parts.2, "");
                expect_restore_text(parts.3.as_str(), "");
            },
            _ => expect_cache_true(false) }
    } else {
        let count = if fail_at < paths.len() { fail_at } else { paths.len() };
        let mut i = 0;
        while i < count
            invariant 0 <= i <= count, count <= paths.len(), count <= 3,
                trace@.len() >= 1 + 2 * count,
                forall|j: int| 0 <= j < count ==>
                    #[trigger] trace@[1 + 2 * j].0 == 1 && trace@[1 + 2 * j].1 == j
                        && trace@[2 + 2 * j].0 == 2 && trace@[2 + 2 * j].1 == j
                        && trace@[2 + 2 * j].2@ == if j % 2 == 0 { "removed"@ } else { "absent"@ },
            decreases count - i,
        {
            expect_cache_true(trace[1 + 2 * i].0 == 1);
            expect_cache_true(trace[1 + 2 * i].1 == i);
            expect_cache_true(trace[2 + 2 * i].0 == 2);
            expect_cache_true(trace[2 + 2 * i].1 == i);
            expect_restore_text(trace[2 + 2 * i].2.as_str(),
                if i % 2 == 0 { "removed" } else { "absent" });
            i += 1;
        }
        if fail_at < paths.len() {
            expect_cache_true(trace.len() == 3 + 2 * fail_at);
            expect_cache_true(trace[2 + 2 * fail_at].0 == 3);
            expect_cache_true(trace[2 + 2 * fail_at].1 == fail_at);
            expect_restore_text(trace[2 + 2 * fail_at].2.as_str(), raw_error);
            match result { Err(parts) => {
                    expect_restore_text(parts.0, "E_CACHE_RECOVERY: ");
                    expect_restore_text(parts.1.as_str(), paths[fail_at].2.as_str());
                    expect_restore_text(parts.2, ": ");
                    expect_restore_text(parts.3.as_str(), raw_error);
                },
                _ => expect_cache_true(false) }
        } else {
            expect_cache_true(trace.len() == 1 + 2 * paths.len());
            expect_cache_true(result.is_ok());
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_removals_publish_before_empty_or_complete_deletion() {
    let mut paths = Vec::new();
    recovery_removals_case(&paths, 0, "", true);
    paths.push(("first".to_string(), 9, "first/path".to_string()));
    recovery_removals_case(&paths, 1, "", true);
    paths.push(("second".to_string(), 0, "second/λ\0\n".to_string()));
    paths.push(("first".to_string(), 2, "".to_string()));
    recovery_removals_case(&paths, 3, "unused", true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_removals_stop_after_first_middle_or_last_failure() {
    let mut paths = Vec::new();
    paths.push(("first".to_string(), 9, "first/path".to_string()));
    paths.push(("second".to_string(), 0, "second/λ\0\n".to_string()));
    paths.push(("first".to_string(), 2, "".to_string()));
    recovery_removals_case(&paths, 0, "denied", true);
    recovery_removals_case(&paths, 1, "error/λ\0\n", true);
    recovery_removals_case(&paths, 2, "", true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_removals_do_not_delete_after_failed_intent_publication() {
    let mut paths = Vec::new();
    recovery_removals_case(&paths, 0, "publish/λ\0\n", false);
    paths.push(("first".to_string(), 0, "first".to_string()));
    recovery_removals_case(&paths, 0, "", false);
    recovery_removals_case(&paths, 1, "publish denied", false);
}

fn recovery_entry_case(
    output_probe: policy::RecoveryProbe, intent_probe: policy::RecoveryProbe,
    replay_result: Result<u64, String>, not_found: u32, intent_path: &str,
    wanted_call: u8, wanted: Result<Option<u64>, (&str, &str, &str)>,
)
    requires match output_probe {
        Err(error) => wanted_call == 0 && match wanted {
            Err(parts) => parts.0@ == ""@ && parts.1@ == error@ && parts.2@ == ""@,
            _ => false,
        },
        Ok(Ok(())) => wanted_call == 2 && match replay_result {
            Ok(value) => wanted == Ok::<Option<u64>, (&str, &str, &str)>(Some(value)),
            Err(error) => match wanted {
                Err(parts) => parts.0@ == ""@ && parts.1@ == error@ && parts.2@ == ""@,
                _ => false,
            },
        },
        Ok(Err((kind, error))) => if kind != not_found {
            wanted_call == 0 && match wanted {
                Err(parts) => parts.0@ == "E_CACHE_RECOVERY: result metadata: "@
                    && parts.1@ == error@ && parts.2@ == ""@,
                _ => false,
            }
        } else {
            wanted_call == 1 && match intent_probe {
                Err(error) => match wanted {
                    Err(parts) => parts.0@ == ""@ && parts.1@ == error@ && parts.2@ == ""@,
                    _ => false,
                },
                Ok(Ok(())) => match wanted {
                    Err(parts) => parts.0@ == "E_CACHE_RECOVERY: unfinished original intent "@
                        && parts.1@ == intent_path@ && parts.2@ == "; no automatic repeat"@,
                    _ => false,
                },
                Ok(Err((kind, error))) => if kind == not_found {
                    wanted == Ok::<Option<u64>, (&str, &str, &str)>(None)
                } else { match wanted {
                    Err(parts) => parts.0@ == "E_CACHE_RECOVERY: intent metadata: "@
                        && parts.1@ == error@ && parts.2@ == ""@,
                    _ => false,
                } },
            }
        },
    },
{
    let trace = Vec::<u8>::new();
    let ghost initial = trace;
    let output = |mut trace: Vec<u8>| -> (out: (Vec<u8>, policy::RecoveryProbe))
        ensures out.0@ == trace@.push(0u8), out.1 == output_probe,
    { trace.push(0); (trace, output_probe) };
    let intent = |mut trace: Vec<u8>| -> (out: (Vec<u8>, policy::RecoveryProbe))
        ensures out.0@ == trace@.push(1u8), out.1 == intent_probe,
    { trace.push(1); (trace, intent_probe) };
    let replay = |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<u64, String>))
        ensures out.0@ == trace@.push(2u8), out.1 == replay_result,
    { trace.push(2); (trace, replay_result) };
    let (trace, result) = policy::select_recovery_entry(
        trace, not_found, intent_path, output, intent, replay);
    proof {
        let (observed, observation, checked) = choose|observed: Vec<u8>,
            observation: policy::RecoveryProbe, checked: Result<bool, policy::RecoveryEntryError>|
            #[trigger] output.ensures((initial,), (observed, observation))
            && #[trigger] policy::recovery_probe_matches(observation, not_found,
                "E_CACHE_RECOVERY: result metadata: ", checked)
            && match checked {
                Err(error) => trace == observed
                    && result == Err::<Option<u64>, policy::RecoveryEntryError>(error),
                Ok(true) => exists|replayed: Result<u64, String>|
                    #[trigger] replay.ensures((observed,), (trace, replayed))
                    && match replayed {
                        Ok(value) => result == Ok::<Option<u64>, policy::RecoveryEntryError>(Some(value)),
                        Err(error) => result == Err::<Option<u64>, policy::RecoveryEntryError>(("", error, "")),
                    },
                Ok(false) => exists|probed: Vec<u8>, probe: policy::RecoveryProbe,
                    checked: Result<bool, policy::RecoveryEntryError>|
                    #[trigger] intent.ensures((observed,), (probed, probe))
                    && trace == probed && #[trigger] policy::recovery_probe_matches(probe, not_found,
                        "E_CACHE_RECOVERY: intent metadata: ", checked)
                    && match checked {
                        Err(error) => result == Err::<Option<u64>, policy::RecoveryEntryError>(error),
                        Ok(false) => result == Ok::<Option<u64>, policy::RecoveryEntryError>(None),
                        Ok(true) => match result {
                            Err(parts) => parts.0@ == "E_CACHE_RECOVERY: unfinished original intent "@
                                && parts.1@ == intent_path@
                                && parts.2@ == "; no automatic repeat"@,
                            _ => false,
                        },
                    },
            };
        assert(observation == output_probe);
    }
    expect_cache_true(trace.len() == if wanted_call == 0 { 1 } else { 2 });
    expect_cache_true(trace[0] == 0);
    if wanted_call != 0 { expect_cache_true(trace[1] == wanted_call); }
    match (result, wanted) {
        (Ok(actual), Ok(expected)) => expect_cache_true(actual == expected),
        (Err(actual), Err(expected)) => {
            expect_restore_text(actual.0, expected.0);
            expect_restore_text(actual.1.as_str(), expected.1);
            expect_restore_text(actual.2, expected.2);
        },
        _ => expect_cache_true(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_entry_replays_before_reading_any_intent() {
    recovery_entry_case(Ok(Ok(())), Ok(Ok(())), Ok(0), 7, "intent", 2, Ok(Some(0)));
    recovery_entry_case(Ok(Ok(())), Err("unreadable intent".to_string()),
        Ok(u64::MAX), 7, "intent", 2, Ok(Some(u64::MAX)));
    recovery_entry_case(Ok(Ok(())), Ok(Err((8, "intent metadata".to_string()))),
        Err("replay/λ\0\n".to_string()), 7, "intent", 2, Err(("", "replay/λ\0\n", "")));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_entry_requires_both_original_paths_to_be_absent() {
    recovery_entry_case(Ok(Err((7, "output absent".to_string()))),
        Ok(Err((7, "intent absent".to_string()))), Ok(42), 7, "intent", 1, Ok(None));
    recovery_entry_case(Ok(Err((u32::MAX, "".to_string()))),
        Ok(Err((u32::MAX, "".to_string()))), Err("unused replay".to_string()),
        u32::MAX, "", 1, Ok(None));
    recovery_entry_case(Ok(Err((7, "absent".to_string()))), Ok(Ok(())), Ok(42),
        7, "意图/λ\0\n", 1, Err(("E_CACHE_RECOVERY: unfinished original intent ",
            "意图/λ\0\n", "; no automatic repeat")));
    recovery_entry_case(Ok(Err((7, "absent".to_string()))), Ok(Ok(())), Ok(42),
        7, "", 1, Err(("E_CACHE_RECOVERY: unfinished original intent ",
            "", "; no automatic repeat")));
}

fn recovery_entry_preserves_original_errors(error: &str) {
    recovery_entry_case(Err(error.to_string()), Ok(Ok(())), Ok(42), 7, "intent", 0,
        Err(("", error, "")));
    recovery_entry_case(Ok(Err((0, error.to_string()))), Ok(Ok(())), Ok(42),
        u32::MAX, "intent", 0, Err(("E_CACHE_RECOVERY: result metadata: ", error, "")));
    recovery_entry_case(Ok(Err((7, "absent".to_string()))), Err(error.to_string()),
        Ok(42), 7, "intent", 1, Err(("", error, "")));
    recovery_entry_case(Ok(Err((7, "absent".to_string()))),
        Ok(Err((u32::MAX, error.to_string()))), Ok(42), 7, "intent", 1,
        Err(("E_CACHE_RECOVERY: intent metadata: ", error, "")));
    recovery_entry_case(Ok(Ok(())), Ok(Ok(())), Err(error.to_string()),
        7, "intent", 2, Err(("", error, "")));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_entry_preserves_empty_unicode_nul_and_newline_errors() {
    recovery_entry_preserves_original_errors("");
    recovery_entry_preserves_original_errors("原始错误/λ\0\n");
}

fn recovery_removal_case(
    observation: Result<(bool, bool), (u32, String)>, not_found: u32,
    directory_result: Result<(), String>, file_result: Result<(), String>,
    wanted_call: u8, wanted: Result<&str, &str>,
)
    requires match observation {
        Err((kind, error)) => wanted_call == 0 && if kind == not_found {
            wanted == Ok::<&str, &str>("absent")
        } else { match wanted { Err(text) => text@ == error@, _ => false } },
        Ok((is_directory, is_file)) => if !is_directory && !is_file {
            wanted_call == 0 && wanted == Err::<&str, &str>(
                "cache artifact is neither a regular file nor a directory")
        } else {
            wanted_call == (if is_directory { 1u8 } else { 2u8 })
                && match if is_directory { directory_result } else { file_result } {
                    Ok(()) => wanted == Ok::<&str, &str>("removed"),
                    Err(error) => match wanted { Err(text) => text@ == error@, _ => false },
                }
        },
    },
{
    let trace = Vec::<u8>::new();
    let ghost initial = trace;
    let metadata = |mut trace: Vec<u8>|
        -> (out: (Vec<u8>, Result<(bool, bool), (u32, String)>))
        ensures out.0@ == trace@.push(0u8), out.1 == observation,
    { trace.push(0); (trace, observation) };
    let directory = |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1u8), out.1 == directory_result,
    { trace.push(1); (trace, directory_result) };
    let file = |mut trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(2u8), out.1 == file_result,
    { trace.push(2); (trace, file_result) };
    let (trace, result) = policy::remove_recovery_output(
        trace, not_found, metadata, directory, file);
    proof {
        let (observed, original) = choose|observed: Vec<u8>,
            original: Result<(bool, bool), (u32, String)>|
            #[trigger] metadata.ensures((initial,), (observed, original))
                && match original {
                    Err((kind, error)) => trace == observed && if kind == not_found {
                        result == Ok::<&'static str, String>("absent")
                    } else { result == Err::<&'static str, String>(error) },
                    Ok((is_directory, is_file)) => if !is_directory && !is_file {
                        trace == observed && match result {
                            Err(error) => error@ ==
                                "cache artifact is neither a regular file nor a directory"@,
                            Ok(_) => false,
                        }
                    } else {
                        exists|removed: Result<(), String>|
                            (if is_directory {
                                directory.ensures((observed,), (trace, removed))
                            } else { file.ensures((observed,), (trace, removed)) })
                            && match removed {
                                Ok(()) => result == Ok::<&'static str, String>("removed"),
                                Err(error) => result == Err::<&'static str, String>(error),
                            }
                    },
                };
        assert(original == observation);
    }
    expect_cache_true(trace.len() == if wanted_call == 0 { 1 } else { 2 });
    expect_cache_true(trace[0] == 0);
    if wanted_call != 0 { expect_cache_true(trace[1] == wanted_call); }
    match (result, wanted) {
        (Ok(actual), Ok(expected)) => expect_restore_text(actual, expected),
        (Err(actual), Err(expected)) => expect_restore_text(actual.as_str(), expected),
        _ => expect_cache_true(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_removal_observes_before_selecting_one_operation() {
    recovery_removal_case(Ok((true, false)), 7, Ok(()), Err("unused file".to_string()),
        1, Ok("removed"));
    recovery_removal_case(Ok((false, true)), 7, Err("unused directory".to_string()), Ok(()),
        2, Ok("removed"));
    recovery_removal_case(Ok((true, true)), 7, Ok(()), Err("unused file".to_string()),
        1, Ok("removed"));
    recovery_removal_case(Ok((false, false)), 7, Ok(()), Ok(()), 0,
        Err("cache artifact is neither a regular file nor a directory"));
    recovery_removal_case(Err((7, "missing original".to_string())), 7,
        Err("unused directory".to_string()), Err("unused file".to_string()), 0, Ok("absent"));
    recovery_removal_case(Err((u32::MAX, "missing original".to_string())), u32::MAX,
        Ok(()), Ok(()), 0, Ok("absent"));
}

fn recovery_removal_preserves_the_complete_first_error(error: &str) {
    recovery_removal_case(Err((8, error.to_string())), 7, Ok(()), Ok(()), 0, Err(error));
    recovery_removal_case(Err((0, error.to_string())), u32::MAX,
        Err("unused directory".to_string()), Err("unused file".to_string()), 0, Err(error));
    recovery_removal_case(Ok((true, false)), 7, Err(error.to_string()), Ok(()), 1, Err(error));
    recovery_removal_case(Ok((false, true)), 7, Ok(()), Err(error.to_string()), 2, Err(error));
    recovery_removal_case(Ok((true, true)), 7,
        Err(error.to_string()), Ok(()), 1, Err(error));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_removal_keeps_empty_unicode_and_control_error_payloads() {
    recovery_removal_preserves_the_complete_first_error("");
    recovery_removal_preserves_the_complete_first_error("恢复/λ\0\n: 原错误");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_actions_keep_discard_and_refresh_have_distinct_effects() {
    let keep = policy::RecoveryAction::Keep;
    expect_cache_true(!keep.discards_outputs());
    expect_cache_true(!keep.refreshes_sources());
    expect_restore_text(keep.initial_status(), "not-required");
    let discard = policy::RecoveryAction::Discard;
    expect_cache_true(discard.discards_outputs());
    expect_cache_true(!discard.refreshes_sources());
    expect_restore_text(discard.initial_status(), "pending");
    let refresh = policy::RecoveryAction::Refresh;
    expect_cache_true(!refresh.discards_outputs());
    expect_cache_true(refresh.refreshes_sources());
    expect_restore_text(refresh.initial_status(), "not-required");
}

fn recovery_non_kept_actions_never_verify_content(method: Option<&str>, reason: &str) {
    let discard = policy::RecoveryAction::Discard;
    expect_cache_true(!discard.requires_content_verification(method, reason));
    let refresh = policy::RecoveryAction::Refresh;
    expect_cache_true(!refresh.requires_content_verification(method, reason));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_content_verification_requires_a_kept_registered_hit() {
    proof {
        reveal_strlit("producer-manifest/v1"); reveal_strlit("Producer-manifest/v1");
        reveal_strlit("producer-manifest/v1\0"); reveal_strlit("");
        reveal_strlit("exact"); reveal_strlit("compatible"); reveal_strlit("Exact");
        reveal_strlit("exact\0"); reveal_strlit("compatible\n");
        reveal_strlit("状态/λ\0\n");
        assert("producer-manifest/v1"@.len() == 20);
        assert(""@.len() == 0);
        assert("Producer-manifest/v1"@.len() == 20);
        assert("producer-manifest/v1"@[0] != "Producer-manifest/v1"@[0]);
        assert("producer-manifest/v1\0"@.len() == 21);
        assert("exact"@.len() == 5); assert("compatible"@.len() == 10);
        assert("Exact"@.len() == 5); assert("exact"@[0] != "Exact"@[0]);
        assert("exact\0"@.len() == 6); assert("compatible\n"@.len() == 11);
        assert("状态/λ\0\n"@.len() == 6);
    }
    let keep = policy::RecoveryAction::Keep;
    expect_cache_true(keep.requires_content_verification(Some("producer-manifest/v1"), "exact"));
    expect_cache_true(keep.requires_content_verification(Some("producer-manifest/v1"), "compatible"));
    expect_cache_true(!keep.requires_content_verification(None, "exact"));
    expect_cache_true(!keep.requires_content_verification(Some(""), "compatible"));
    expect_cache_true(!keep.requires_content_verification(Some("Producer-manifest/v1"), "exact"));
    expect_cache_true(!keep.requires_content_verification(Some("producer-manifest/v1\0"), "compatible"));
    expect_cache_true(!keep.requires_content_verification(Some("producer-manifest/v1"), "Exact"));
    expect_cache_true(!keep.requires_content_verification(Some("producer-manifest/v1"), "exact\0"));
    expect_cache_true(!keep.requires_content_verification(Some("producer-manifest/v1"), "compatible\n"));
    expect_cache_true(!keep.requires_content_verification(Some("producer-manifest/v1"), "状态/λ\0\n"));
    recovery_non_kept_actions_never_verify_content(None, "");
    recovery_non_kept_actions_never_verify_content(Some("producer-manifest/v1"), "exact");
    recovery_non_kept_actions_never_verify_content(Some("producer-manifest/v1"), "compatible");
    recovery_non_kept_actions_never_verify_content(Some("状态/λ\0\n"), "状态/λ\0\n");
}

fn recovery_decision_errors_keep_the_complete_id(id: &str) {
    let malformed = policy::recovery_decision_error(
        &policy::RecoveryDecisionError::MalformedOutputs, id);
    expect_restore_text(malformed[0], "E_CACHE_RECOVERY: malformed restore outputs for ");
    expect_restore_text(malformed[1], id);
    let unknown = policy::recovery_decision_error(
        &policy::RecoveryDecisionError::UnknownOutcome, id);
    expect_restore_text(unknown[0],
        "E_CACHE_RECOVERY: missing, cancelled or unknown restore outcome for ");
    expect_restore_text(unknown[1], id);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_decision_errors_select_both_prefixes_and_preserve_text_boundaries() {
    recovery_decision_errors_keep_the_complete_id("");
    recovery_decision_errors_keep_the_complete_id("cache-id");
    recovery_decision_errors_keep_the_complete_id("Cache-ID");
    recovery_decision_errors_keep_the_complete_id("缓存/λ\0\n");
    recovery_decision_errors_keep_the_complete_id("\0cache-id\0");
}

fn recovery_preserves_cache_status(status: Option<&str>)
    requires policy::recovery_text(status) != Some("pending"@),
{
    match policy::complete_recovery_cache(status) {
        None => (),
        Some(_) => panic!("non-pending cache status changed"),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_completes_only_the_exact_pending_cache_status() {
    proof {
        reveal_strlit("pending"); reveal_strlit("Pending"); reveal_strlit("pending\0");
        reveal_strlit(""); reveal_strlit("discarded"); reveal_strlit("状态/λ");
        assert("pending"@.len() == 7); assert("pending\0"@.len() == 8);
        assert(""@.len() == 0); assert("discarded"@.len() == 9);
        assert("状态/λ"@.len() == 4);
        assert("pending"@[0] != "Pending"@[0]);
    }
    match policy::complete_recovery_cache(Some("pending")) {
        Some(status) => expect_restore_text(status, "discarded"),
        None => panic!("completed pending cache left pending"),
    }
    recovery_preserves_cache_status(None);
    recovery_preserves_cache_status(Some(""));
    recovery_preserves_cache_status(Some("discarded"));
    recovery_preserves_cache_status(Some("Pending"));
    recovery_preserves_cache_status(Some("pending\0"));
    recovery_preserves_cache_status(Some("状态/λ"));
}

fn recovery_failure_precedes_cache_effects(summary: &policy::RecoverySummary) {
    expect_restore_text(summary.status(true), "failed");
}

fn recovery_observed_artifact<'a>(status: &'a Option<&'a str>) -> (out: Option<&'a str>)
    ensures out == *status,
{ *status }

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_aggregate_handles_no_caches_and_failed_cache_without_artifacts() {
    proof {
        reveal_strlit("failed"); reveal_strlit("recovered");
        assert("failed"@.len() == 6); assert("recovered"@.len() == 9);
    }
    let caches: [policy::RecoveryCacheObservation<Option<&str>>; 0] = [];
    let summary = policy::summarize_recovery(&caches,
        recovery_observed_artifact,
        Ghost(|status: Option<&str>| policy::recovery_text(status)));
    expect_restore_text(summary.status(false), "not-required");
    recovery_failure_precedes_cache_effects(&summary);
    if summary.warning(Some("recovered")) { panic!("empty recovery warned"); }

    let artifacts: [Option<&str>; 0] = [];
    let caches = [policy::RecoveryCacheObservation {
        status: Some("discarded"), outcome: Some("failure"), artifacts: &artifacts,
    }];
    let summary = policy::summarize_recovery(&caches,
        recovery_observed_artifact,
        Ghost(|status: Option<&str>| policy::recovery_text(status)));
    proof {
        assert(policy::recovery_cache_changed(caches@[0]));
        assert(policy::recovery_cache_warns(caches@[0],
            |status: Option<&str>| policy::recovery_text(status)));
    }
    expect_restore_text(summary.status(false), "recovered");
    recovery_failure_precedes_cache_effects(&summary);
    if !summary.warning(Some("recovered")) { panic!("failed empty cache did not warn"); }
    if summary.warning(Some("failed")) { panic!("failed recovery warned"); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_aggregate_combines_first_cache_change_and_last_artifact_removal() {
    let ghost model = |status: Option<&str>| policy::recovery_text(status);
    let first = [None, Some("absent")];
    let middle: [Option<&str>; 0] = [];
    let last = [Some("absent"), None, Some("removed")];
    let caches = [
        policy::RecoveryCacheObservation {
            status: Some("source-times-refreshed"), outcome: None, artifacts: &first,
        },
        policy::RecoveryCacheObservation {
            status: None, outcome: Some("success"), artifacts: &middle,
        },
        policy::RecoveryCacheObservation {
            status: Some("pending"), outcome: None, artifacts: &last,
        },
    ];
    let summary = policy::summarize_recovery(&caches,
        recovery_observed_artifact,
        Ghost(model));
    proof {
        assert(policy::recovery_cache_changed(caches@[0]));
        assert(model(caches@[2].artifacts@[2]) == Some("removed"@));
        assert(policy::recovery_cache_warns(caches@[2], model));
    }
    expect_restore_text(summary.status(false), "recovered");
    recovery_failure_precedes_cache_effects(&summary);
    if !summary.warning(Some("recovered")) { panic!("late artifact removal was lost"); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_aggregate_combines_first_artifact_removal_and_last_cache_change() {
    let ghost model = |status: Option<&str>| policy::recovery_text(status);
    let first = [Some("removed"), Some("absent")];
    let empty: [Option<&str>; 0] = [];
    let caches = [
        policy::RecoveryCacheObservation {
            status: None, outcome: None, artifacts: &first,
        },
        policy::RecoveryCacheObservation {
            status: Some("unknown"), outcome: None, artifacts: &empty,
        },
        policy::RecoveryCacheObservation {
            status: Some("discarded"), outcome: Some("success"), artifacts: &empty,
        },
    ];
    let summary = policy::summarize_recovery(&caches,
        recovery_observed_artifact,
        Ghost(model));
    proof {
        assert(policy::recovery_cache_changed(caches@[2]));
        assert(model(caches@[0].artifacts@[0]) == Some("removed"@));
        assert(policy::recovery_cache_warns(caches@[0], model));
    }
    expect_restore_text(summary.status(false), "recovered");
    if !summary.warning(Some("recovered")) { panic!("early artifact removal was lost"); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_aggregate_keeps_unknown_case_nul_and_unicode_text_exact() {
    proof {
        reveal_strlit("discarded"); reveal_strlit("Discarded"); reveal_strlit("discarded\0");
        reveal_strlit("source-times-refreshed"); reveal_strlit(""); reveal_strlit("状态/λ");
        reveal_strlit("failure"); reveal_strlit("Failure"); reveal_strlit("failure\0");
        reveal_strlit("removed"); reveal_strlit("Removed"); reveal_strlit("removed\0");
        assert("discarded"@.len() == 9); assert("discarded\0"@.len() == 10);
        assert("Discarded"@.len() == 9); assert("discarded"@[0] != "Discarded"@[0]);
        assert("source-times-refreshed"@.len() == 22); assert(""@.len() == 0);
        assert("状态/λ"@.len() == 4); assert("failure"@.len() == 7);
        assert("Failure"@.len() == 7); assert("failure\0"@.len() == 8);
        assert("failure"@[0] != "Failure"@[0]);
        assert("removed"@.len() == 7); assert("Removed"@.len() == 7);
        assert("removed\0"@.len() == 8); assert("removed"@[0] != "Removed"@[0]);
    }
    let artifacts = [None, Some(""), Some("Removed"), Some("removed\0"), Some("状态/λ")];
    let empty: [Option<&str>; 0] = [];
    let caches = [
        policy::RecoveryCacheObservation {
            status: Some("Discarded"), outcome: Some("Failure"), artifacts: &artifacts,
        },
        policy::RecoveryCacheObservation {
            status: Some("discarded\0"), outcome: Some("failure\0"), artifacts: &empty,
        },
        policy::RecoveryCacheObservation {
            status: Some("状态/λ"), outcome: Some(""), artifacts: &empty,
        },
    ];
    let summary = policy::summarize_recovery(&caches,
        recovery_observed_artifact,
        Ghost(|status: Option<&str>| policy::recovery_text(status)));
    expect_restore_text(summary.status(false), "not-required");
    if summary.warning(Some("recovered")) { panic!("inexact recovery text counted as an effect"); }
}

fn recovery_changed_cache_recovers(status: &str)
    requires status@ == "discarded"@ || status@ == "source-times-refreshed"@,
{
    let summary = policy::RecoverySummary::new()
        .observe_cache(Some(status), None)
        .observe_cache(None, None)
        .observe_cache(Some("unknown"), None)
        .observe_cache(Some(status), None);
    expect_restore_text(summary.status(false), "recovered");
    recovery_failure_precedes_cache_effects(&summary);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_report_prioritizes_failure_and_recognizes_both_completed_effects() {
    recovery_failure_precedes_cache_effects(&policy::RecoverySummary::new());
    recovery_changed_cache_recovers("discarded");
    recovery_changed_cache_recovers("source-times-refreshed");
    let summary = policy::RecoverySummary::new()
        .observe_cache(Some("source-times-refreshed"), None)
        .observe_cache(Some("discarded"), None)
        .observe_cache(None, None);
    expect_restore_text(summary.status(false), "recovered");
    recovery_failure_precedes_cache_effects(&summary);
}

fn recovery_unchanged_cache_requires_no_recovery(status: &str)
    requires status@ != "discarded"@, status@ != "source-times-refreshed"@,
{
    let summary = policy::RecoverySummary::new()
        .observe_cache(None, None)
        .observe_cache(Some(status), None)
        .observe_cache(Some(status), None);
    expect_restore_text(summary.status(false), "not-required");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_report_preserves_empty_unknown_case_and_nul_status_boundaries() {
    proof {
        reveal_strlit("discarded"); reveal_strlit("Discarded"); reveal_strlit("discarded\0");
        reveal_strlit("source-times-refreshed"); reveal_strlit("source-times-refreshed\0");
        reveal_strlit(""); reveal_strlit("pending"); reveal_strlit("状态/λ");
        assert("discarded"@.len() == 9); assert("discarded\0"@.len() == 10);
        assert("source-times-refreshed"@.len() == 22);
        assert("source-times-refreshed\0"@.len() == 23);
        assert(""@.len() == 0); assert("pending"@.len() == 7);
        assert("状态/λ"@.len() == 4);
        assert("discarded"@[0] != "Discarded"@[0]);
        assert("Discarded"@.len() == 9);
    }
    expect_restore_text(policy::RecoverySummary::new().status(false), "not-required");
    recovery_unchanged_cache_requires_no_recovery("");
    recovery_unchanged_cache_requires_no_recovery("pending");
    recovery_unchanged_cache_requires_no_recovery("Discarded");
    recovery_unchanged_cache_requires_no_recovery("discarded\0");
    recovery_unchanged_cache_requires_no_recovery("source-times-refreshed\0");
    recovery_unchanged_cache_requires_no_recovery("状态/λ");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_warning_accepts_failed_restores_or_removed_artifacts_after_recovery() {
    let failed = policy::RecoverySummary::new()
        .observe_cache(Some("discarded"), None)
        .observe_cache(None, Some("failure"));
    if !failed.warning(Some("recovered")) {
        panic!("recovered failed restore did not warn");
    }
    let removed = policy::RecoverySummary::new()
        .observe_cache(Some("source-times-refreshed"), None)
        .observe_artifact(Some("removed"))
        .observe_artifact(None);
    if !removed.warning(Some("recovered")) {
        panic!("recovered removed artifact did not warn");
    }
    let combined = policy::RecoverySummary::new()
        .observe_cache(Some("discarded"), Some("failure"))
        .observe_cache(None, Some("failure"))
        .observe_artifact(Some("removed"));
    if !combined.warning(Some("recovered")) {
        panic!("combined recovery effects did not warn");
    }
    if policy::RecoverySummary::new().warning(Some("recovered")) {
        panic!("recovery without a failed restore or removal warned");
    }
}

fn recovery_nonterminal_status_never_warns(status: Option<&str>)
    requires policy::recovery_text(status) != Some("recovered"@),
{
    let summary = policy::RecoverySummary::new()
        .observe_cache(Some("discarded"), Some("failure"))
        .observe_artifact(Some("removed"));
    if summary.warning(status) {
        panic!("unrecovered report warned despite failed restore and removal");
    }
}

fn recovery_other_effects_never_warn(outcome: Option<&str>, artifact: Option<&str>)
    requires policy::recovery_text(outcome) != Some("failure"@),
        policy::recovery_text(artifact) != Some("removed"@),
{
    let summary = policy::RecoverySummary::new()
        .observe_cache(None, None)
        .observe_cache(Some("source-times-refreshed"), outcome)
        .observe_artifact(artifact)
        .observe_artifact(None);
    if summary.warning(Some("recovered")) {
        panic!("unknown restore outcome or artifact status counted as recovery effect");
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_warning_preserves_report_outcome_and_artifact_text_boundaries() {
    proof {
        reveal_strlit("recovered"); reveal_strlit("Recovered"); reveal_strlit("recovered\0");
        reveal_strlit("failure"); reveal_strlit("Failure"); reveal_strlit("failure\0");
        reveal_strlit("removed"); reveal_strlit("Removed"); reveal_strlit("removed\0");
        reveal_strlit(""); reveal_strlit("状态/λ");
        assert("recovered"@.len() == 9); assert("recovered\0"@.len() == 10);
        assert("failure"@.len() == 7); assert("failure\0"@.len() == 8);
        assert("removed"@.len() == 7); assert("removed\0"@.len() == 8);
        assert(""@.len() == 0); assert("状态/λ"@.len() == 4);
        assert("recovered"@[0] != "Recovered"@[0]);
        assert("failure"@[0] != "Failure"@[0]);
        assert("removed"@[0] != "Removed"@[0]);
    }
    recovery_nonterminal_status_never_warns(None);
    recovery_nonterminal_status_never_warns(Some(""));
    recovery_nonterminal_status_never_warns(Some("Recovered"));
    recovery_nonterminal_status_never_warns(Some("recovered\0"));
    recovery_other_effects_never_warn(None, None);
    recovery_other_effects_never_warn(Some(""), Some(""));
    recovery_other_effects_never_warn(Some("Failure"), Some("Removed"));
    recovery_other_effects_never_warn(Some("failure\0"), Some("removed\0"));
    recovery_other_effects_never_warn(Some("状态/λ"), Some("状态/λ"));
}

fn replay_observation<'a>(path: &'a str, plan_hash: &'a str, steps_hash: &'a str,
    status: Option<&'a str>) -> (out: policy::RecoveryReplay<'a>)
    ensures out.schema == Some("chrono-cache-recovery/v1"), out.plan_fields == Some(2usize),
        out.plan_path == Some(path), out.plan_sha256 == Some(plan_hash),
        out.steps_sha256 == Some(steps_hash), out.status == status,
{
    policy::RecoveryReplay {
        schema: Some("chrono-cache-recovery/v1"), plan_fields: Some(2),
        plan_path: Some(path), plan_sha256: Some(plan_hash),
        steps_sha256: Some(steps_hash), status,
    }
}

fn replay_accepts(path: &str, plan_hash: &str, steps_hash: &str, original: &str,
    status: &str)
    requires status@ == "recovered"@ || status@ == "not-required"@,
{
    let report = replay_observation(path, plan_hash, steps_hash, Some(status));
    match policy::admit_recovery_replay(&report, path, plan_hash, steps_hash, original) {
        Ok(()) => (),
        Err(_) => panic!("matching completed recovery rejected"),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn replay_accepts_both_terminal_states_and_complete_binding_values() {
    replay_accepts("计划/λ\0\n", "摘要甲\0\n", "摘要乙/λ", "原件\0\n", "recovered");
    replay_accepts("", "", "", "", "not-required");
}

fn replay_wrong_inputs(report: &policy::RecoveryReplay, path: &str,
    plan_hash: &str, steps_hash: &str)
    requires !policy::recovery_replay_bound(report, path, plan_hash, steps_hash),
{
    match policy::admit_recovery_replay(report, path, plan_hash, steps_hash, "原件\0\n") {
        Ok(()) => panic!("different recovery inputs accepted"),
        Err(error) => {
            expect_restore_text(error[0],
                "E_CACHE_RECOVERY: previous recovery belongs to different inputs");
            expect_restore_text(error[1], "");
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn replay_rejects_each_missing_or_changed_binding_and_extra_plan_fields_first() {
    proof {
        reveal_strlit("chrono-cache-recovery/v1");
        reveal_strlit("CHRONO-cache-recovery/v1");
        reveal_strlit("plan"); reveal_strlit("plan\0");
        reveal_strlit("hash"); reveal_strlit("Hash");
        reveal_strlit("steps"); reveal_strlit("");
        assert("plan"@.len() == 4); assert("plan\0"@.len() == 5);
        assert("steps"@.len() == 5); assert(""@.len() == 0);
        assert("hash"@[0] != "Hash"@[0]);
        assert("chrono-cache-recovery/v1"@[0] != "CHRONO-cache-recovery/v1"@[0]);
    }
    let mut report = replay_observation("plan", "hash", "steps", Some("failed"));
    report.schema = None;
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.schema = Some("CHRONO-cache-recovery/v1");
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.schema = Some("chrono-cache-recovery/v1");
    report.plan_fields = None;
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.plan_fields = Some(0);
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.plan_fields = Some(1);
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.plan_fields = Some(3);
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.plan_fields = Some(2);
    report.plan_path = None;
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.plan_path = Some("plan\0");
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.plan_path = Some("plan");
    report.plan_sha256 = None;
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.plan_sha256 = Some("Hash");
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.plan_sha256 = Some("hash");
    report.steps_sha256 = None;
    replay_wrong_inputs(&report, "plan", "hash", "steps");
    report.steps_sha256 = Some("");
    replay_wrong_inputs(&report, "plan", "hash", "steps");
}

fn replay_failed(status: Option<&str>, original: &str)
    requires policy::recovery_text(status) != Some("recovered"@),
        policy::recovery_text(status) != Some("not-required"@),
{
    let report = replay_observation("plan", "hash", "steps", status);
    match policy::admit_recovery_replay(&report, "plan", "hash", "steps", original) {
        Ok(()) => panic!("failed or unknown previous recovery accepted"),
        Err(error) => {
            expect_restore_text(error[0], "E_CACHE_RECOVERY: previous attempt failed; original ");
            expect_restore_text(error[1], original);
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn replay_rejects_failed_missing_unknown_case_and_nul_statuses_with_original_path() {
    proof {
        reveal_strlit("recovered"); reveal_strlit("not-required");
        reveal_strlit("failed"); reveal_strlit("");
        reveal_strlit("Recovered"); reveal_strlit("recovered\0");
        assert("recovered"@.len() == 9); assert("not-required"@.len() == 12);
        assert("failed"@.len() == 6); assert(""@.len() == 0);
        assert("recovered\0"@.len() == 10);
        assert("recovered"@[0] != "Recovered"@[0]);
    }
    replay_failed(None, "原件/λ\0\n");
    replay_failed(Some("failed"), "原件/λ\0\n");
    replay_failed(Some(""), "");
    replay_failed(Some("Recovered"), "原件/λ\0\n");
    replay_failed(Some("recovered\0"), "原件/λ\0\n");
}

fn recovery_value(actual: Option<&str>, expected: &str)
    requires policy::recovery_text(actual) == Some(expected@),
{
    match actual {
        Some(text) => expect_restore_text(text, expected),
        None => panic!("missing recovery option"),
    }
}

fn recovery_missing(options: &policy::RecoveryOptions, key: &policy::RecoveryOption,
    expected: &str)
    requires options.value(key.slot()).is_none(), expected@ == key.missing(),
{
    match options.required(key) {
        Ok(_) => panic!("missing recovery option accepted"),
        Err(error) => expect_restore_text(error, expected),
    }
}

fn recovery_values(root: &str, config: &str, plan: &str, environment: &str) {
    let args = ["--plan".to_owned(), plan.to_owned(),
        "--host-root".to_owned(), root.to_owned(),
        "--steps-env".to_owned(), environment.to_owned(),
        "--config".to_owned(), config.to_owned()];
    proof {
        reveal_strlit("--host-root"); reveal_strlit("--config");
        reveal_strlit("--plan"); reveal_strlit("--steps-env");
        assert("--plan"@.len() == 6);
        assert("--config"@.len() == 8);
        assert("--host-root"@.len() == 11);
        assert("--steps-env"@.len() == 11);
        assert("--host-root"@[2] != "--steps-env"@[2]);
        assert(policy::recovery_pair_slot(args@, 0) == 2);
        assert(policy::recovery_pair_slot(args@, 1) == 0);
        assert(policy::recovery_pair_slot(args@, 2) == 3);
        assert(policy::recovery_pair_slot(args@, 3) == 1);
        assert forall|i: int| 0 <= i < 4 implies
            policy::recovery_pair_slot(args@, i) < 4 by { }
        assert forall|i: int, j: int| 0 <= i < j < 4 implies
            policy::recovery_pair_slot(args@, i) != policy::recovery_pair_slot(args@, j) by { }
        assert(policy::recovery_prefix_valid(args@, 4));
        assert(policy::recovery_args_valid(args@));
    }
    let options = match policy::parse_recovery_options(&args) {
        Ok(options) => options,
        Err(_) => panic!("complete recovery options rejected"),
    };
    proof {
        assert(policy::recovery_options_bound(args@, 4, &options));
        assert(options.value(0) == Some(root@));
        assert(options.value(1) == Some(config@));
        assert(options.value(2) == Some(plan@));
        assert(options.value(3) == Some(environment@));
    }
    recovery_value(options.get(&policy::RecoveryOption::HostRoot), root);
    recovery_value(options.get(&policy::RecoveryOption::Config), config);
    recovery_value(options.get(&policy::RecoveryOption::Plan), plan);
    recovery_value(options.get(&policy::RecoveryOption::StepsEnvironment), environment);
    match options.required(&policy::RecoveryOption::Plan) {
        Ok(actual) => expect_restore_text(actual, plan),
        Err(_) => panic!("present recovery option rejected"),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_options_preserve_reordered_complete_values() {
    recovery_values("根/λ\0\n", ".chrono-harness/配置\0\n",
        ".chrono-harness/state/计划\0\n", "环境\0\n");
    recovery_values("", "", "", "");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_empty_options_keep_all_four_missing_errors() {
    let options = match policy::parse_recovery_options(&[]) {
        Ok(options) => options,
        Err(_) => panic!("empty recovery options rejected before required lookup"),
    };
    recovery_missing(&options, &policy::RecoveryOption::HostRoot,
        "E_CACHE_RECOVERY: missing --host-root");
    recovery_missing(&options, &policy::RecoveryOption::Config,
        "E_CACHE_RECOVERY: missing --config");
    recovery_missing(&options, &policy::RecoveryOption::Plan,
        "E_CACHE_RECOVERY: missing --plan");
    recovery_missing(&options, &policy::RecoveryOption::StepsEnvironment,
        "E_CACHE_RECOVERY: missing --steps-env");
}

fn recovery_parse_error(args: &[String])
    requires !policy::recovery_args_valid(args@),
{
    match policy::parse_recovery_options(args) {
        Ok(_) => panic!("invalid recovery options accepted"),
        Err(error) => expect_restore_text(error,
            "E_CACHE_RECOVERY: invalid or duplicate recovery option"),
    }
}

fn recovery_duplicate(key: &str)
    requires policy::recovery_arg_slot(key@) < 4,
{
    let args = [key.to_owned(), "first\0\n".to_owned(),
        key.to_owned(), "second/λ".to_owned()];
    proof {
        assert(policy::recovery_pair_slot(args@, 0) == policy::recovery_arg_slot(key@));
        assert(policy::recovery_pair_slot(args@, 1) == policy::recovery_arg_slot(key@));
        assert(!policy::recovery_args_valid(args@)) by {
            if policy::recovery_args_valid(args@) {
                assert(policy::recovery_prefix_valid(args@, 2));
                assert(policy::recovery_pair_slot(args@, 0)
                    != policy::recovery_pair_slot(args@, 1));
            }
        }
    }
    recovery_parse_error(&args);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_rejects_each_duplicate_option() {
    proof {
        reveal_strlit("--host-root"); reveal_strlit("--config");
        reveal_strlit("--plan"); reveal_strlit("--steps-env");
    }
    recovery_duplicate("--host-root");
    recovery_duplicate("--config");
    recovery_duplicate("--plan");
    recovery_duplicate("--steps-env");
}

fn recovery_unknown(key: &str)
    requires policy::recovery_arg_slot(key@) == 4,
{
    let args = [key.to_owned(), "原值\0\n".to_owned()];
    proof {
        assert(policy::recovery_pair_slot(args@, 0) == 4);
        assert(!policy::recovery_args_valid(args@)) by {
            if policy::recovery_args_valid(args@) {
                assert(policy::recovery_prefix_valid(args@, 1));
                assert(policy::recovery_pair_slot(args@, 0) < 4);
            }
        }
    }
    recovery_parse_error(&args);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_rejects_unknown_case_and_nul_keys_and_odd_input() {
    proof {
        reveal_strlit("--host-root"); reveal_strlit("--config");
        reveal_strlit("--plan"); reveal_strlit("--steps-env");
        reveal_strlit("--PLAN"); reveal_strlit("--plan\0");
        reveal_strlit("");
        assert(""@.len() != "--host-root"@.len());
        assert(""@.len() != "--config"@.len());
        assert(""@.len() != "--plan"@.len());
        assert(""@.len() != "--steps-env"@.len());
        assert("--PLAN"@[2] != "--plan"@[2]);
    }
    recovery_unknown("");
    recovery_unknown("--PLAN");
    recovery_unknown("--plan\0");
    let args = ["--host-root".to_owned()];
    recovery_parse_error(&args);
    let args = ["--host-root".to_owned(), "根".to_owned(), "--config".to_owned()];
    recovery_parse_error(&args);
}

fn recovery_paths(config: &str, plan: &str, accepted: bool)
    requires accepted == (policy::restore_prefix_matches(config, Some(".chrono-harness/"))
        && policy::restore_prefix_matches(plan, Some(".chrono-harness/state/"))),
{
    match policy::admit_recovery_paths(config, plan) {
        Ok(()) => expect_cache_true(accepted),
        Err(error) => {
            expect_cache_true(!accepted);
            expect_restore_text(error, "E_CACHE_RECOVERY: expected host config and retained state plan");
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_paths_require_both_exact_prefixes() {
    proof {
        reveal_strlit(".chrono-harness/"); reveal_strlit(".chrono-harness/state/");
        reveal_strlit(".chrono-harness/配置\0\n");
        reveal_strlit(".chrono-harness/state/计划\0\n");
        reveal_strlit(".chrono-harness-other/config");
        reveal_strlit(".chrono-harness/states/plan");
        reveal_strlit(".chrono-harness"); reveal_strlit("");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii(".chrono-harness/")
            && vstd::string::is_ascii(".chrono-harness/state/")
            && vstd::string::is_ascii(".chrono-harness-other/config")
            && vstd::string::is_ascii(".chrono-harness/states/plan")
            && vstd::string::is_ascii(".chrono-harness")
            && vstd::string::is_ascii(""));
        reveal_with_fuel(vstd::utf8::encode_utf8, 32);
        assert(forall|i: int| 0 <= i < ".chrono-harness/".spec_bytes().len()
            ==> ".chrono-harness/配置\0\n".spec_bytes()[i] == ".chrono-harness/".spec_bytes()[i]);
        assert(forall|i: int| 0 <= i < ".chrono-harness/state/".spec_bytes().len()
            ==> ".chrono-harness/state/计划\0\n".spec_bytes()[i] == ".chrono-harness/state/".spec_bytes()[i]);
        assert(".chrono-harness-other/config".spec_bytes()[15]
            != ".chrono-harness/".spec_bytes()[15]);
        assert(".chrono-harness/states/plan".spec_bytes()[21]
            != ".chrono-harness/state/".spec_bytes()[21]);
    }
    recovery_paths(".chrono-harness/配置\0\n", ".chrono-harness/state/计划\0\n", true);
    recovery_paths(".chrono-harness/", ".chrono-harness/state/", true);
    recovery_paths(".chrono-harness-other/config", ".chrono-harness/state/", false);
    recovery_paths(".chrono-harness/", ".chrono-harness/states/plan", false);
    recovery_paths(".chrono-harness", ".chrono-harness/state/", false);
    recovery_paths("", "", false);
}

fn artifact_trace(mut state: Vec<u8>, event: u8) -> (out: Vec<u8>)
    ensures out@ == state@.push(event),
{
    state.push(event);
    state
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn artifact_error_renderer_receives_every_original_field() {
    let prefix = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[0]@,
        { parts[0].to_owned() };
    let path = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[1]@,
        { parts[1].to_owned() };
    let separator = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[2]@,
        { parts[2].to_owned() };
    let detail = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[3]@,
        { parts[3].to_owned() };
    expect_restore_text(content::content_error("缓存/λ\0\n", "原错\0\n", &prefix).as_str(), "E_CACHE_CONTENT: ");
    expect_restore_text(content::content_error("缓存/λ\0\n", "原错\0\n", &path).as_str(), "缓存/λ\0\n");
    expect_restore_text(content::content_error("缓存/λ\0\n", "原错\0\n", &separator).as_str(), ": ");
    expect_restore_text(content::content_error("缓存/λ\0\n", "原错\0\n", &detail).as_str(), "原错\0\n");
}

fn artifact_observation_case(raw: Result<bool, (u32, &str)>, expected: Result<bool, &str>)
    requires vstd::laws_cmp::obeys_cmp::<&str>(),
        match raw {
            Ok(true) => expected == Ok::<bool, &str>(true),
            Ok(false) => expected == Err::<bool, &str>("cache artifact must be a physical directory"),
            Err((tag, detail)) => if tag == 404 { expected == Ok::<bool, &str>(false) }
                else { expected == Err::<bool, &str>(detail) },
        },
{
    let resolve = |state: Vec<u8>, _path: &str| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == state@.push(1), out.1 == Ok::<(), String>(()),
        { (artifact_trace(state, 1), Ok(())) };
    let observe = |state: Vec<u8>, _target: &()| -> (out: (Vec<u8>, Result<bool, (u32, &str)>))
        ensures out.0@ == state@.push(2), out.1 == raw,
        { (artifact_trace(state, 2), raw) };
    let error_text = |text: &str| -> (out: String) ensures out@ == text@,
        { text.to_owned() };
    let render = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[3]@,
        { parts[3].to_owned() };
    let io = content::ArtifactIo { resolve, observe, error_text, render };
    let rows = [("缓存\0\n", Some("路径/λ\0\n"))];
    let initial = Vec::<u8>::new();
    let ghost initial_state = initial;
    let out = content::preflight(initial, &rows, 404, &io);
    let _ = (out, expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn artifact_raw_tags_keep_directories_absence_wrong_type_and_original_errors_distinct()
    requires vstd::laws_cmp::obeys_cmp::<&str>(),
{
    artifact_observation_case(Ok(true), Ok(true));
    artifact_observation_case(Ok(false), Err("cache artifact must be a physical directory"));
    artifact_observation_case(Err((404, "ignored missing detail")), Ok(false));
    artifact_observation_case(Err((0, "")), Err(""));
    artifact_observation_case(Err((u32::MAX, "原错\0\n")), Err("原错\0\n"));
}

fn artifact_early_error_case(no_artifacts: bool, no_path: bool)
    requires vstd::laws_cmp::obeys_cmp::<&str>(),
{
    let resolve = |state: Vec<u8>, _path: &str| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == state@.push(1), match out.1 {
            Err(error) => error@ == "原路径错误\0\n"@, _ => false },
        { (artifact_trace(state, 1), Err("原路径错误\0\n".to_owned())) };
    let observe = |state: Vec<u8>, _target: &()| -> (out: (Vec<u8>, Result<bool, (u32, &str)>))
        ensures out.0@ == state@.push(2), out.1 == Ok::<bool, (u32, &str)>(true),
        { (artifact_trace(state, 2), Ok(true)) };
    let error_text = |text: &str| -> (out: String) ensures out@ == text@,
        { text.to_owned() };
    let render = |parts: [&str; 4]| -> (out: String) ensures out@ == parts[3]@,
        { parts[3].to_owned() };
    let io = content::ArtifactIo { resolve, observe, error_text, render };
    let rows = [("a", if no_path { None } else { Some("root") })];
    let unavailable = |state: Vec<u8>, _absent: content::MissingArtifacts| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == state@.push(3), out.1 == Ok::<(), String>(()),
        { (artifact_trace(state, 3), Ok(())) };
    let available = |state: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == state@.push(4), out.1 == Ok::<(), String>(()),
        { (artifact_trace(state, 4), Ok(())) };
    let initial = Vec::<u8>::new();
    let ghost initial_state = initial;
    let out = content::with_artifacts(initial, if no_artifacts { None } else { Some(&rows) },
        404, &io, unavailable, available);
    let _ = (out, no_artifacts, no_path);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn artifact_input_and_path_failures_do_not_observe_or_publish()
    requires vstd::laws_cmp::obeys_cmp::<&str>(),
{
    artifact_early_error_case(true, false);
    artifact_early_error_case(true, true);
    artifact_early_error_case(false, true);
    artifact_early_error_case(false, false);
}

fn expect_restore_text(actual: &str, expected: &str)
    requires actual@ == expected@,
{
    expect_cache_true(actual.to_owned() == expected.to_owned());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn restore_outcome_precedes_any_key_or_prefix() {
    proof {
        reveal_strlit("success"); reveal_strlit("failure"); reveal_strlit("cancelled");
        reveal_strlit("skipped"); reveal_strlit("Success"); reveal_strlit("success\n");
        reveal_strlit("");
        assert("success"@[0] != "failure"@[0]);
        assert("success"@.len() != "cancelled"@.len());
        assert("success"@[1] != "skipped"@[1]);
        assert("failure"@.len() != "cancelled"@.len());
        assert("failure"@[0] != "skipped"@[0]);
        assert("cancelled"@.len() != "skipped"@.len());
        assert("Success"@[0] != "success"@[0]);
        assert("Success"@[0] != "failure"@[0]);
        assert("Success"@.len() != "cancelled"@.len());
        assert("Success"@[0] != "skipped"@[0]);
        assert("success\n"@.len() != "success"@.len());
        assert("success\n"@.len() != "failure"@.len());
        assert("success\n"@.len() != "cancelled"@.len());
        assert("success\n"@.len() != "skipped"@.len());
    }
    let prefixes = [Some("key")];
    expect_restore_text(policy::restore_status(Some("failure"), Some("key"), Some("key"), &prefixes), "error");
    expect_restore_text(policy::restore_status(Some("cancelled"), Some("key"), Some("key"), &prefixes), "error");
    expect_restore_text(policy::restore_status(Some("skipped"), Some("key"), Some("key"), &prefixes), "not-attempted");
    expect_restore_text(policy::restore_status(None, Some("key"), Some("key"), &prefixes), "unavailable");
    expect_restore_text(policy::restore_status(Some("Success"), Some("key"), Some("key"), &prefixes), "unavailable");
    expect_restore_text(policy::restore_status(Some("success\n"), Some("key"), Some("key"), &prefixes), "unavailable");
    expect_restore_text(policy::restore_status(Some(""), Some("key"), Some("key"), &prefixes), "unavailable");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn successful_restore_needs_a_nonempty_observed_key_and_exact_match_wins() {
    proof {
        reveal_strlit(""); reveal_strlit("key");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("key") && vstd::string::is_ascii(""));
        assert("key".spec_bytes().len() == 3);
        assert("".spec_bytes().len() == 0);
    }
    expect_restore_text(policy::restore_status(Some("success"), Some("key"), None, &[]), "miss-or-unavailable");
    expect_restore_text(policy::restore_status(Some("success"), Some(""), Some(""), &[Some("")]), "miss-or-unavailable");
    expect_restore_text(policy::restore_status(Some("success"), Some("key"), Some("key"), &[]), "exact");
    expect_restore_text(policy::restore_status(Some("success"), Some("key"), Some("key"), &[Some("key")]), "exact");
    expect_restore_text(policy::restore_status(Some("success"), None, Some("key"), &[]), "incompatible");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_prefix_is_examined_but_missing_empty_case_and_longer_prefixes_do_not_match() {
    proof {
        reveal_strlit(""); reveal_strlit("cache/revision"); reveal_strlit("other/");
        reveal_strlit("cache/"); reveal_strlit("Cache/");
        reveal_strlit("cache/revision-extra"); reveal_strlit("cache/\0");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("cache/revision")
            && vstd::string::is_ascii("other/") && vstd::string::is_ascii("cache/")
            && vstd::string::is_ascii("Cache/") && vstd::string::is_ascii("cache/revision-extra")
            && vstd::string::is_ascii("cache/\0"));
        assert("cache/revision".spec_bytes()[0] != "Cache/".spec_bytes()[0]);
        assert("cache/revision".spec_bytes()[6] != "cache/\0".spec_bytes()[6]);
        assert("cache/revision".spec_bytes()[0] != "other/".spec_bytes()[0]);
        assert("cache/revision-extra".spec_bytes().len() > "cache/revision".spec_bytes().len());
        assert(forall|i: int| 0 <= i < "cache/".spec_bytes().len()
            ==> "cache/revision".spec_bytes()[i] == "cache/".spec_bytes()[i]);
    }
    let key = Some("cache/revision");
    let prefixes = [None, Some(""), Some("other/"), Some("cache/")];
    proof { assert(policy::restore_prefix_matches("cache/revision", prefixes@[3])); }
    expect_restore_text(policy::restore_status(Some("success"), None, key, &prefixes), "compatible");
    expect_restore_text(policy::restore_status(Some("success"), None, key, &[None, Some("")]), "incompatible");
    expect_restore_text(policy::restore_status(Some("success"), None, key, &[Some("Cache/")]), "incompatible");
    expect_restore_text(policy::restore_status(Some("success"), None, key, &[Some("cache/revision-extra")]), "incompatible");
    expect_restore_text(policy::restore_status(Some("success"), None, key, &[Some("cache/\0")]), "incompatible");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn restore_identity_and_full_prefix_keep_unicode_nul_and_newline() {
    let key = "缓存/λ\0\n";
    proof {
        reveal_strlit("缓存/λ\0\n");
        vstd::utf8::encode_utf8_valid_utf8(key@);
        vstd::utf8::encode_utf8_decode_utf8(key@);
        assert(key.spec_bytes().len() > 0) by {
            if key.spec_bytes().len() == 0 {
                assert(key.spec_bytes() == Seq::<u8>::empty());
                reveal(vstd::utf8::decode_utf8);
            }
        }
        assert(forall|i: int| 0 <= i < key.spec_bytes().len()
            ==> key.spec_bytes()[i] == key.spec_bytes()[i]);
    }
    expect_restore_text(policy::restore_status(Some("success"), Some(key), Some(key), &[]), "exact");
    expect_restore_text(policy::restore_status(Some("success"), None, Some(key), &[Some(key)]), "compatible");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_observed_success_or_opted_in_failure_can_seal() {
    proof {
        reveal_strlit("success"); reveal_strlit("failure");
        reveal_strlit("skipped"); reveal_strlit("cancelled");
        reveal_strlit("success\n"); reveal_strlit("");
        assert("success"@[0] != "failure"@[0]);
        assert("success"@[1] != "skipped"@[1]);
        assert("failure"@[0] != "skipped"@[0]);
        assert("success"@.len() != "cancelled"@.len());
        assert("failure"@.len() != "cancelled"@.len());
        assert("success"@.len() != "success\n"@.len());
        assert("failure"@.len() != "success\n"@.len());
        assert("success"@.len() != ""@.len());
        assert("failure"@.len() != ""@.len());
    }
    expect_cache_true(policy::producer_can_seal(false, Some("success")));
    expect_cache_true(policy::producer_can_seal(true, Some("success")));
    expect_cache_true(!policy::producer_can_seal(false, Some("failure")));
    expect_cache_true(policy::producer_can_seal(true, Some("failure")));
    expect_cache_true(!policy::producer_can_seal(true, Some("skipped")));
    expect_cache_true(!policy::producer_can_seal(true, Some("cancelled")));
    expect_cache_true(!policy::producer_can_seal(true, Some("success\n")));
    expect_cache_true(!policy::producer_can_seal(true, Some("")));
    expect_cache_true(!policy::producer_can_seal(false, None));
    expect_cache_true(!policy::producer_can_seal(true, None));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_adopted_compatible_restores_refresh_inputs() {
    proof {
        reveal_strlit("compatible"); reveal_strlit("exact");
        reveal_strlit("error"); reveal_strlit("miss-or-unavailable");
        reveal_strlit("compatible\n"); reveal_strlit("");
        assert("compatible"@.len() != "exact"@.len());
        assert("compatible"@.len() != "error"@.len());
        assert("compatible"@.len() != "miss-or-unavailable"@.len());
        assert("compatible"@.len() != "compatible\n"@.len());
        assert("compatible"@.len() != ""@.len());
    }
    expect_cache_true(policy::refresh_sources(true, "compatible"));
    expect_cache_true(!policy::refresh_sources(false, "compatible"));
    expect_cache_true(!policy::refresh_sources(true, "exact"));
    expect_cache_true(!policy::refresh_sources(true, "error"));
    expect_cache_true(!policy::refresh_sources(true, "miss-or-unavailable"));
    expect_cache_true(!policy::refresh_sources(true, "compatible\n"));
    expect_cache_true(!policy::refresh_sources(true, ""));
}

fn recovery_action_tag(
    outcome: Option<&str>, reason: &str, unconfirmed: bool,
    outputs_is_null_or_object: bool, matched_key_is_null_or_string: bool,
    content_method: Option<&str>, content_status: Option<&str>, refresh_enabled: bool,
    expected: u8,
)
    requires match policy::recovery_action_spec(
        outcome, reason, unconfirmed, outputs_is_null_or_object,
        matched_key_is_null_or_string, content_method, content_status, refresh_enabled,
    ) {
        Ok(policy::RecoveryAction::Keep) => expected == 0,
        Ok(policy::RecoveryAction::Discard) => expected == 1,
        Ok(policy::RecoveryAction::Refresh) => expected == 2,
        Err(policy::RecoveryDecisionError::UnknownOutcome) => expected == 3,
        Err(policy::RecoveryDecisionError::MalformedOutputs) => expected == 4,
    },
{
    let actual = policy::recovery_action(
        outcome, reason, unconfirmed, outputs_is_null_or_object,
        matched_key_is_null_or_string, content_method, content_status, refresh_enabled,
    );
    let actual_tag = match actual {
        Ok(policy::RecoveryAction::Keep) => 0,
        Ok(policy::RecoveryAction::Discard) => 1,
        Ok(policy::RecoveryAction::Refresh) => 2,
        Err(policy::RecoveryDecisionError::UnknownOutcome) => 3,
        Err(policy::RecoveryDecisionError::MalformedOutputs) => 4,
    };
    assert(actual_tag == expected);
    expect_cache_true(actual_tag == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recovery_decision_covers_outcomes_content_and_refresh_boundaries() {
    proof {
        reveal_strlit("failure"); reveal_strlit("success"); reveal_strlit("skipped");
        reveal_strlit("miss-or-unavailable"); reveal_strlit("incompatible");
        reveal_strlit("exact"); reveal_strlit("compatible");
        reveal_strlit("producer-manifest/v1"); reveal_strlit("matched");
        reveal_strlit("unusable"); reveal_strlit("unknown"); reveal_strlit("");
        assert("failure"@[0] != "success"@[0]);
        assert("success"@[1] != "skipped"@[1]);
        assert("exact"@.len() != "compatible"@.len());
        assert("matched"@[0] != "unusable"@[0]);
        assert("producer-manifest/v1"@.len() != "matched"@.len());
        assert("unknown"@[0] != "success"@[0]);
    }
    // A failed provider always discards, regardless of optional content metadata.
    recovery_action_tag(
        Some("failure"), "error", false, true, true,
        Some("producer-manifest/v1"), Some("matched"), true, 1,
    );
    // An opted-in unconfirmed restore rejects malformed output shape first.
    recovery_action_tag(
        Some("success"), "exact", true, false, true,
        None, None, false, 4,
    );
    // Misses and incompatible keys discard before content verification.
    recovery_action_tag(
        Some("success"), "miss-or-unavailable", true, true, true,
        None, None, false, 1,
    );
    recovery_action_tag(
        Some("success"), "incompatible", true, true, true,
        None, None, false, 1,
    );
    // Exact content verification keeps a matched producer manifest and discards an unusable one.
    recovery_action_tag(
        Some("success"), "exact", false, true, true,
        Some("producer-manifest/v1"), Some("matched"), false, 0,
    );
    recovery_action_tag(
        Some("success"), "exact", false, true, true,
        Some("producer-manifest/v1"), Some("unusable"), false, 1,
    );
    // Compatible restores refresh only when the policy enables source refresh.
    recovery_action_tag(
        Some("success"), "compatible", false, true, true,
        None, None, false, 0,
    );
    recovery_action_tag(
        Some("success"), "compatible", false, true, true,
        None, None, true, 2,
    );
    // Skipped is retained; missing and unknown outcomes are rejected.
    recovery_action_tag(
        Some("skipped"), "not-attempted", false, true, true,
        None, None, false, 0,
    );
    recovery_action_tag(
        None, "unavailable", false, true, true,
        None, None, false, 3,
    );
    recovery_action_tag(
        Some("unknown"), "unavailable", false, true, true,
        None, None, false, 3,
    );
}

// 同一预期也在普通 Cargo 中实际执行。
fn expect_cache_true(actual: bool)
    requires actual,
{
    if !actual {
        panic!();
    }
}

fn expect_registration(schema: &str, expected: bool)
    requires expected == (schema@ == "chrono-cache/v2"@),
{
    let actual = policy::current_registration(schema);
    assert(actual == expected);
    expect_cache_true(actual == expected);
}

fn expect_plan(schema: Option<&str>, expected: bool)
    requires expected == policy::current_plan_schema(schema),
{
    let actual = policy::current_plan(schema);
    assert(actual == expected);
    expect_cache_true(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_current_registration_and_plan_schemas_are_accepted() {
    proof {
        reveal_strlit("chrono-cache/v2"); reveal_strlit("chrono-cache/v1");
        reveal_strlit("chrono-cache/v20"); reveal_strlit("chrono-cache/v2\n");
        reveal_strlit("chrono-cache-plan/v2"); reveal_strlit("chrono-cache-plan/v1");
        reveal_strlit("chrono-cache-plan/v20"); reveal_strlit("chrono-cache-plan/v2\n");
        reveal_strlit("");
        assert("chrono-cache/v2"@[14] != "chrono-cache/v1"@[14]);
        assert("chrono-cache/v2"@.len() != "chrono-cache/v20"@.len());
        assert("chrono-cache/v2"@.len() != "chrono-cache/v2\n"@.len());
        assert("chrono-cache/v2"@.len() != ""@.len());
        assert("chrono-cache-plan/v2"@[19] != "chrono-cache-plan/v1"@[19]);
        assert("chrono-cache-plan/v2"@.len() != "chrono-cache-plan/v20"@.len());
        assert("chrono-cache-plan/v2"@.len() != "chrono-cache-plan/v2\n"@.len());
        assert("chrono-cache-plan/v2"@.len() != ""@.len());
    }
    expect_registration("chrono-cache/v2", true);
    expect_registration("chrono-cache/v1", false);
    expect_registration("chrono-cache/v20", false);
    expect_registration("chrono-cache/v2\n", false);
    expect_registration("", false);
    expect_plan(Some("chrono-cache-plan/v2"), true);
    expect_plan(Some("chrono-cache-plan/v1"), false);
    expect_plan(Some("chrono-cache-plan/v20"), false);
    expect_plan(Some("chrono-cache-plan/v2\n"), false);
    expect_plan(Some(""), false);
    expect_plan(None, false);
}

fn expect(
    ds: Option<&str>, dv: Option<&str>, os: Option<&str>, ov: Option<&str>,
    expected: bool,
)
    requires expected == policy::planner_identity_matches(ds, dv, os, ov),
{
    let actual = policy::matches_planner_identity(ds, dv, os, ov);
    assert(actual == expected);
    expect_cache_true(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn matching_fields_accept_complete_text() {
    expect(Some("digest"), Some("1.0"), Some("digest"), Some("1.0"), true);
    expect(Some("摘要"), Some("版本\0\n"), Some("摘要"), Some("版本\0\n"), true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_missing_field_rejects() {
    expect(None, None, None, None, false);
    expect(None, None, Some("digest"), Some("1.0"), false);
    expect(None, Some("1.0"), Some("digest"), Some("1.0"), false);
    expect(Some("digest"), None, Some("digest"), Some("1.0"), false);
    expect(Some("digest"), Some("1.0"), None, Some("1.0"), false);
    expect(Some("digest"), Some("1.0"), Some("digest"), None, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn mismatches_do_not_accept_case_or_prefixes() {
    proof {
        reveal_strlit("digest"); reveal_strlit("Digest");
        reveal_strlit("digest\0"); reveal_strlit("1.0");
        reveal_strlit("1.1"); reveal_strlit("1.0\n");
        reveal_strlit("版本甲"); reveal_strlit("版本乙");
        assert("digest"@[0] != "Digest"@[0]);
        assert("digest"@.len() != "digest\0"@.len());
        assert("1.0"@[2] != "1.1"@[2]);
        assert("1.0"@.len() != "1.0\n"@.len());
        assert("版本甲"@[2] != "版本乙"@[2]);
    }
    expect(Some("digest"), Some("1.0"), Some("Digest"), Some("1.0"), false);
    expect(Some("digest"), Some("1.0"), Some("digest\0"), Some("1.0"), false);
    expect(Some("digest"), Some("1.0"), Some("digest"), Some("1.1"), false);
    expect(Some("digest"), Some("1.0"), Some("digest"), Some("1.0\n"), false);
    expect(Some("digest"), Some("版本甲"), Some("digest"), Some("版本乙"), false);
}

fn expect_freshness(
    current_newer: bool, first_newer: bool, second_newer: bool,
    second_attempted: bool, expected: u8,
)
    requires expected == match policy::freshness_decision_spec(
        current_newer, first_newer, second_newer, second_attempted,
    ) {
        policy::FreshnessDecision::AlreadyNewer => 0u8,
        policy::FreshnessDecision::Updated => 1u8,
        policy::FreshnessDecision::Retry => 2u8,
        policy::FreshnessDecision::Failed => 3u8,
    },
{
    let actual: u8 = match policy::freshness_decision(
        current_newer, first_newer, second_newer, second_attempted,
    ) {
        policy::FreshnessDecision::AlreadyNewer => 0u8,
        policy::FreshnessDecision::Updated => 1u8,
        policy::FreshnessDecision::Retry => 2u8,
        policy::FreshnessDecision::Failed => 3u8,
    };
    assert(actual == expected);
    expect_cache_true(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn freshness_transition_prioritizes_observed_state_and_retries_once() {
    expect_freshness(false, false, false, false, 2u8);
    expect_freshness(false, false, false, true, 3u8);
    expect_freshness(false, false, true, false, 2u8);
    expect_freshness(false, false, true, true, 1u8);
    expect_freshness(false, true, false, false, 1u8);
    expect_freshness(false, true, false, true, 1u8);
    expect_freshness(false, true, true, false, 1u8);
    expect_freshness(false, true, true, true, 1u8);
    expect_freshness(true, false, false, false, 0u8);
    expect_freshness(true, false, false, true, 0u8);
    expect_freshness(true, false, true, false, 0u8);
    expect_freshness(true, false, true, true, 0u8);
    expect_freshness(true, true, false, false, 0u8);
    expect_freshness(true, true, false, true, 0u8);
    expect_freshness(true, true, true, false, 0u8);
    expect_freshness(true, true, true, true, 0u8);
}

}
