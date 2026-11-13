//! 参数原文保全、首错和两种位置的独立预期，同时执行 Verus 与 Rust。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_cli.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_cli as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

broadcast use {vstd::string::axiom_str_literal_len, vstd::string::axiom_str_literal_get_char};

fn expect_true(value: bool)
    requires value,
{
    if !value { panic!(); }
}

fn expect_text(actual: &str, expected: &str)
    requires actual@ == expected@,
{
    expect_true(actual.to_owned() == expected.to_owned());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_valid_arguments_keep_empty_unicode_replacement_and_duplicates() {
    let input = [Some("tool"), Some(""), Some("目录/é/�"), Some("same"), Some("same")];
    let output = policy::decoded_arguments(&input);
    match output {
        Ok(texts) => {
            expect_true(texts.len() == 5);
            expect_text(texts[0], "tool");
            expect_text(texts[1], "");
            expect_text(texts[2], "目录/é/�");
            expect_text(texts[3], "same");
            expect_text(texts[4], "same");
        },
        Err(_) => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_input_and_empty_program_name_remain_valid_text() {
    match policy::decoded_arguments(&[]) {
        Ok(texts) => expect_true(texts.len() == 0),
        Err(_) => panic!(),
    }
    match policy::decoded_arguments(&[Some("")]) {
        Ok(texts) => {
            expect_true(texts.len() == 1);
            expect_text(texts[0], "");
        },
        Err(_) => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn invalid_program_name_has_no_user_argument_index() {
    match policy::decoded_arguments(&[None, Some("--help"), None]) {
        Err(error) => {
            expect_true(error.argv_index == 0);
            expect_true(error.argument_index.is_none());
        },
        Ok(_) => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn first_invalid_user_argument_wins_over_later_failures() {
    match policy::decoded_arguments(&[Some("tool"), Some("check"), None, Some(""), None]) {
        Err(error) => {
            expect_true(error.argv_index == 2);
            expect_true(error.argument_index == Some(1));
        },
        Ok(_) => panic!(),
    }
    match policy::decoded_arguments(&[Some("tool"), Some(""), Some("�"), None]) {
        Err(error) => {
            expect_true(error.argv_index == 3);
            expect_true(error.argument_index == Some(2));
        },
        Ok(_) => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn judge_arguments_require_the_complete_exact_protocol_or_version() {
    proof {
        reveal_strlit("--version"); reveal_strlit("--protocol"); reveal_strlit("chrono-judge/v1");
        reveal_strlit("--Version"); reveal_strlit("--version\0");
        reveal_strlit("chrono-judge/v1\n"); reveal_strlit("chrono-judge/v1\0");
        reveal_strlit("chrono-judge/v10"); reveal_strlit("Chrono-judge/v1"); reveal_strlit("目录/é");
        assert("--version"@.len() == 9); assert("--protocol"@.len() == 10);
        assert("--version\0"@.len() == 10); assert("chrono-judge/v1"@.len() == 15);
        assert("chrono-judge/v1\n"@.len() == 16); assert("chrono-judge/v1\0"@.len() == 16);
        assert("chrono-judge/v10"@.len() == 16); assert("目录/é"@.len() == 4);
        assert("Chrono-judge/v1"@[0] != "chrono-judge/v1"@[0]);
        assert("--version"@ != "--protocol"@);
        assert("--Version"@[2] != "--version"@[2]);
        assert("--Version"@ != "--version"@);
    }
    expect_true(matches!(policy::judge_mode(&["--version"]), policy::JudgeMode::Version));
    expect_true(matches!(policy::judge_mode(&["--protocol", "chrono-judge/v1"]), policy::JudgeMode::Protocol));
    expect_true(matches!(policy::judge_mode(&[]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_mode(&["--version", "extra"]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_mode(&["--Version"]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_mode(&["--version\0"]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_mode(&["--protocol"]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_mode(&["--protocol", "chrono-judge/v1", ""]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_mode(&["--protocol", "chrono-judge/v1\n"]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_mode(&["--protocol", "chrono-judge/v1\0"]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_mode(&["--protocol", "chrono-judge/v10"]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_mode(&["--protocol", "Chrono-judge/v1"]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_mode(&["--protocol", "目录/é"]), policy::JudgeMode::Usage));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn judge_read_bound_is_inclusive_and_a_native_read_error_always_rejects() {
    expect_true(policy::MAX_JUDGE_REQUEST_BYTES == 67108864);
    expect_true(policy::JUDGE_READ_LIMIT == 67108865);
    expect_true(policy::judge_request_admissible(true, 0));
    expect_true(policy::judge_request_admissible(true, 67108863));
    expect_true(policy::judge_request_admissible(true, 67108864));
    expect_true(!policy::judge_request_admissible(true, 67108865));
    expect_true(!policy::judge_request_admissible(true, usize::MAX));
    expect_true(!policy::judge_request_admissible(false, 0));
    expect_true(!policy::judge_request_admissible(false, 67108864));
    expect_true(!policy::judge_request_admissible(false, usize::MAX));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn judge_owned_arguments_keep_the_original_count_and_complete_text() {
    proof {
        reveal_strlit("--version"); reveal_strlit("--protocol");
        assert("--version"@.len() == 9); assert("--protocol"@.len() == 10);
    }
    expect_true(matches!(policy::judge_owned_mode(&[]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_owned_mode(&["--version".to_owned()]), policy::JudgeMode::Version));
    expect_true(matches!(policy::judge_owned_mode(&["--protocol".to_owned(), "chrono-judge/v1".to_owned()]), policy::JudgeMode::Protocol));
    expect_true(matches!(policy::judge_owned_mode(&["--version".to_owned(), "".to_owned()]), policy::JudgeMode::Usage));
    expect_true(matches!(policy::judge_owned_mode(&["--protocol".to_owned(), "chrono-judge/v1".to_owned(), "目录\0\né".to_owned()]), policy::JudgeMode::Usage));
}

struct Probe {
    stop: u8,
    exit: u8,
    trace: Vec<u8>,
    published: Vec<u8>,
}

spec fn step(before: Probe, after: Probe, tag: u8) -> bool {
    after.stop == before.stop && after.exit == before.exit
        && after.trace@ == before.trace@.push(tag)
}

spec fn payload(bytes: Seq<u8>) -> bool { bytes == seq![0u8, 255, 10, 195, 169] }

spec fn probe_result(stop: u8, exit: u8, out: (Probe, policy::JudgeOutcome<String>)) -> bool {
    out.0.stop == stop && out.0.exit == exit
        && out.0.trace@ == seq![1u8, 2, 3, 4, 5].subrange(0,
            if stop == 1 { 1 } else if stop == 2 { 2 } else if stop == 3 { 4 } else { 5 })
        && (if stop == 1 || stop == 2 || stop == 3 { out.0.published@.len() == 0 }
            else { payload(out.0.published@) })
        && match out.1 {
            policy::JudgeOutcome::ReadBound => stop == 1,
            policy::JudgeOutcome::Codec(error) =>
                (stop == 2 && error@ == "解码\0\né"@)
                || (stop == 3 && error@ == "编码\0\né"@),
            policy::JudgeOutcome::OutputFailed => stop == 4,
            policy::JudgeOutcome::Delivered(code) => stop != 1 && stop != 2 && stop != 3 && stop != 4 && code == exit,
            _ => false,
        }
}

fn run_probe(args: &[&str], stop: u8, exit: u8) -> (out: (Probe, policy::JudgeOutcome<String>))
    ensures match policy::judge_mode_spec(args@) {
        policy::JudgeMode::Protocol => probe_result(stop, exit, out),
        policy::JudgeMode::Version => out.0.trace@.len() == 0 && out.0.published@.len() == 0
            && out.1 == policy::JudgeOutcome::<String>::Version,
        policy::JudgeMode::Usage => out.0.trace@.len() == 0 && out.0.published@.len() == 0
            && out.1 == policy::JudgeOutcome::<String>::Usage,
    },
{
    let initial = Probe { stop, exit, trace: Vec::new(), published: Vec::new() };
    let ghost before = initial;
    let read = |mut state: Probe| -> (out: (Probe, Result<Vec<u8>, u8>))
        ensures step(state, out.0, 1), out.0.published@ == state.published@,
            match out.1 { Err(error) => state.stop == 1 && error == 71,
                Ok(bytes) => state.stop != 1 && payload(bytes@) },
    {
        state.trace.push(1);
        let result = if state.stop == 1 { Err(71) } else { Ok(vec![0, 255, 10, 195, 169]) };
        (state, result)
    };
    let decode = |mut state: Probe, bytes: Vec<u8>| -> (out: (Probe, Result<Vec<u8>, String>))
        ensures step(state, out.0, 2), out.0.published@ == state.published@,
            match out.1 { Err(error) => state.stop == 2 && error@ == "解码\0\né"@,
                Ok(request) => state.stop != 2 && request == bytes },
    {
        state.trace.push(2);
        let result = if state.stop == 2 { Err("解码\0\né".to_owned()) } else { Ok(bytes) };
        (state, result)
    };
    let judge = |mut state: Probe, request: Vec<u8>| -> (out: (Probe, Vec<u8>, u8))
        ensures step(state, out.0, 3), out.0.published@ == state.published@,
            out.1 == request, out.2 == state.exit,
    {
        state.trace.push(3);
        let exit = state.exit;
        (state, request, exit)
    };
    let encode = |mut state: Probe, response: Vec<u8>| -> (out: (Probe, Result<Vec<u8>, String>))
        ensures step(state, out.0, 4), out.0.published@ == state.published@,
            match out.1 { Err(error) => state.stop == 3 && error@ == "编码\0\né"@,
                Ok(bytes) => state.stop != 3 && bytes == response },
    {
        state.trace.push(4);
        let result = if state.stop == 3 { Err("编码\0\né".to_owned()) } else { Ok(response) };
        (state, result)
    };
    let publish = |mut state: Probe, bytes: Vec<u8>| -> (out: (Probe, Result<(), u8>))
        ensures step(state, out.0, 5), out.0.published == bytes,
            match out.1 { Err(error) => state.stop == 4 && error == 72,
                Ok(_) => state.stop != 4 },
    {
        state.trace.push(5);
        state.published = bytes;
        let result = if state.stop == 4 { Err(72) } else { Ok(()) };
        (state, result)
    };
    let out = policy::run_judge_entry(initial, policy::judge_mode(args), read, decode, judge, encode, publish);
    proof {
        if policy::judge_mode_spec(args@) == policy::JudgeMode::Protocol {
            let (s1, input) = choose|s1: Probe, input: Result<Vec<u8>, u8>|
                read.ensures((before,), (s1, input)) && match input {
                    Err(_) => out == (s1, policy::JudgeOutcome::<String>::ReadBound),
                    Ok(bytes) => if bytes.len() > 67108864 { out == (s1, policy::JudgeOutcome::<String>::ReadBound) }
                        else { policy::judge_decode_result(s1, bytes, decode, judge, encode, publish, out) },
                };
            assert(step(before, s1, 1));
            if let Ok(bytes) = input {
                assert(payload(bytes@));
                let (s2, decoded) = choose|s2: Probe, decoded: Result<Vec<u8>, String>|
                    decode.ensures((s1, bytes), (s2, decoded)) && match decoded {
                        Err(error) => out == (s2, policy::JudgeOutcome::Codec(error)),
                        Ok(request) => policy::judge_evaluation_result(s2, request, judge, encode, publish, out),
                    };
                assert(step(s1, s2, 2));
                if let Ok(request) = decoded {
                    let (s3, response, code) = choose|s3: Probe, response: Vec<u8>, code: u8|
                        judge.ensures((s2, request), (s3, response, code))
                        && policy::judge_encode_result(s3, response, code, encode, publish, out);
                    assert(step(s2, s3, 3));
                    let (s4, encoded) = choose|s4: Probe, encoded: Result<Vec<u8>, String>|
                        encode.ensures((s3, response), (s4, encoded)) && match encoded {
                            Err(error) => out == (s4, policy::JudgeOutcome::Codec(error)),
                            Ok(bytes) => policy::judge_publish_result(s4, bytes, code, publish, out),
                        };
                    assert(step(s3, s4, 4));
                    if let Ok(bytes) = encoded {
                        let (s5, result) = choose|s5: Probe, result: Result<(), u8>|
                            publish.ensures((s4, bytes), (s5, result)) && out.0 == s5
                            && out.1 == match result { Err(_) => policy::JudgeOutcome::<String>::OutputFailed,
                                Ok(_) => policy::JudgeOutcome::<String>::Delivered(code) };
                        assert(step(s4, s5, 5));
                        assert(payload(out.0.published@));
                    }
                }
            }
            assert(out.0.trace@ =~= seq![1u8, 2, 3, 4, 5].subrange(0,
                if stop == 1 { 1 } else if stop == 2 { 2 } else if stop == 3 { 4 } else { 5 }));
        }
    }
    out
}

fn expect_prefix(trace: &[u8], length: usize)
    requires length <= 5, trace@ == seq![1u8, 2, 3, 4, 5].subrange(0, length as int),
{
    expect_true(trace.len() == length);
    let mut i = 0;
    while i < length
        invariant i <= length, length <= 5,
            trace@ == seq![1u8, 2, 3, 4, 5].subrange(0, length as int),
        decreases length - i,
    {
        expect_true(trace[i] == (i + 1) as u8);
        i += 1;
    }
}

fn expect_payload(bytes: &[u8])
    requires payload(bytes@),
{
    expect_true(bytes.len() == 5);
    expect_true(bytes[0] == 0);
    expect_true(bytes[1] == 255);
    expect_true(bytes[2] == 10);
    expect_true(bytes[3] == 195);
    expect_true(bytes[4] == 169);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn judge_pipeline_stops_at_the_first_failure_and_keeps_complete_errors() {
    let (state, outcome) = run_probe(&["--protocol", "chrono-judge/v1"], 1, 0);
    expect_prefix(&state.trace, 1);
    expect_true(state.published.len() == 0);
    expect_true(matches!(outcome, policy::JudgeOutcome::ReadBound));
    let (state, outcome) = run_probe(&["--protocol", "chrono-judge/v1"], 2, 1);
    expect_prefix(&state.trace, 2);
    expect_true(state.published.len() == 0);
    match outcome { policy::JudgeOutcome::Codec(error) => expect_text(&error, "解码\0\né"), _ => panic!() }
    let (state, outcome) = run_probe(&["--protocol", "chrono-judge/v1"], 3, 2);
    expect_prefix(&state.trace, 4);
    expect_true(state.published.len() == 0);
    match outcome { policy::JudgeOutcome::Codec(error) => expect_text(&error, "编码\0\né"), _ => panic!() }
    let (state, outcome) = run_probe(&["--protocol", "chrono-judge/v1"], 4, 0);
    expect_prefix(&state.trace, 5);
    expect_payload(&state.published);
    expect_true(matches!(outcome, policy::JudgeOutcome::OutputFailed));
}

fn delivered_case(exit: u8) {
    let (state, outcome) = run_probe(&["--protocol", "chrono-judge/v1"], 0, exit);
    expect_prefix(&state.trace, 5);
    expect_payload(&state.published);
    match outcome { policy::JudgeOutcome::Delivered(code) => expect_true(code == exit), _ => panic!() }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn judge_success_keeps_the_original_domain_exit_and_output_bytes() {
    delivered_case(0);
    delivered_case(1);
    delivered_case(2);
    delivered_case(255);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn judge_version_and_usage_do_not_read_decode_or_evaluate() {
    let (state, outcome) = run_probe(&["--version"], 1, 255);
    expect_true(state.trace.len() == 0);
    expect_true(state.published.len() == 0);
    expect_true(matches!(outcome, policy::JudgeOutcome::Version));
    let (state, outcome) = run_probe(&[], 4, 255);
    expect_true(state.trace.len() == 0);
    expect_true(state.published.len() == 0);
    expect_true(matches!(outcome, policy::JudgeOutcome::Usage));
}

fn finish_probe(outcome: policy::JudgeOutcome<String>) -> (out: (Vec<u8>, String, u8))
    ensures match outcome {
        policy::JudgeOutcome::Version => out.0@ == seq![7u8] && out.1@ == ""@ && out.2 == 0,
        policy::JudgeOutcome::Usage => out.0@ == seq![8u8] && out.1@ == ""@ && out.2 == 2,
        policy::JudgeOutcome::ReadBound => out.0@ == seq![9u8] && out.1@ == ""@ && out.2 == 2,
        policy::JudgeOutcome::Codec(error) => out.0@ == seq![10u8] && out.1 == error && out.2 == 2,
        policy::JudgeOutcome::OutputFailed => out.0@.len() == 0 && out.1@ == ""@ && out.2 == 2,
        policy::JudgeOutcome::Delivered(exit) => out.0@.len() == 0 && out.1@ == ""@ && out.2 == exit,
    },
{
    let version = |mut state: (Vec<u8>, String)| -> (out: (Vec<u8>, String))
        ensures out.0@ == state.0@.push(7), out.1 == state.1,
    { state.0.push(7); state };
    let usage = |mut state: (Vec<u8>, String)| -> (out: (Vec<u8>, String))
        ensures out.0@ == state.0@.push(8), out.1 == state.1,
    { state.0.push(8); state };
    let bound = |mut state: (Vec<u8>, String)| -> (out: (Vec<u8>, String))
        ensures out.0@ == state.0@.push(9), out.1 == state.1,
    { state.0.push(9); state };
    let codec = |mut state: (Vec<u8>, String), error: String| -> (out: (Vec<u8>, String))
        ensures out.0@ == state.0@.push(10), out.1 == error,
    { state.0.push(10); (state.0, error) };
    let ((trace, error), exit) = policy::finish_judge_entry(
        (Vec::new(), "".to_owned()), outcome, version, usage, bound, codec,
    );
    (trace, error, exit)
}

fn expect_emission(trace: &[u8], tag: u8)
    requires trace@ == seq![tag],
{
    expect_true(trace.len() == 1);
    expect_true(trace[0] == tag);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn judge_finish_emits_only_the_selected_diagnostic_and_keeps_exit_codes() {
    proof { reveal_strlit(""); }
    let (trace, error, exit) = finish_probe(policy::JudgeOutcome::Version);
    expect_emission(&trace, 7); expect_text(&error, ""); expect_true(exit == 0);
    let (trace, error, exit) = finish_probe(policy::JudgeOutcome::Usage);
    expect_emission(&trace, 8); expect_text(&error, ""); expect_true(exit == 2);
    let (trace, error, exit) = finish_probe(policy::JudgeOutcome::ReadBound);
    expect_emission(&trace, 9); expect_text(&error, ""); expect_true(exit == 2);
    let (trace, error, exit) = finish_probe(policy::JudgeOutcome::Codec("完整错误\0\né".to_owned()));
    expect_emission(&trace, 10); expect_text(&error, "完整错误\0\né"); expect_true(exit == 2);
    let (trace, error, exit) = finish_probe(policy::JudgeOutcome::OutputFailed);
    expect_true(trace.len() == 0); expect_text(&error, ""); expect_true(exit == 2);
    let (trace, error, exit) = finish_probe(policy::JudgeOutcome::Delivered(1));
    expect_true(trace.len() == 0); expect_text(&error, ""); expect_true(exit == 1);
}

}
