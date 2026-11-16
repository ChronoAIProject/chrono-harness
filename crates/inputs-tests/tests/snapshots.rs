#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../inputs/src/lib.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as composition_text;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_inputs as policy;
#[cfg(verus_keep_ghost)]
use policy::identifiers as composition_text;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
struct CompositionImportFixture {
    id: u64,
    blob_error: bool,
    admission_error: bool,
}

fn fixture_import_lookup(mut trace: CompositionTrace, id: &str)
    -> (out: (CompositionTrace, Option<usize>))
    ensures out.0@ == trace@.push((0u64, 1u8)),
        out.1 == if id@ == "缺失输入\0\n"@ { None } else { Some(7usize) },
{
    trace.push((0, 1));
    let declared = if composition_text::equal_text(id, "缺失输入\0\n") { None }
        else { Some(7) };
    (trace, declared)
}

fn fixture_import_blob(mut trace: CompositionTrace, file: &CompositionImportFixture)
    -> (out: (CompositionTrace, Result<(u64, bool), String>))
    ensures out.0@ == trace@.push((file.id, 2u8)),
        if file.blob_error { match out.1 {
            Err(error) => error@ == "blob 原错\0\n"@, _ => false,
        } } else { out.1 == Ok::<(u64, bool), String>((file.id, file.admission_error)) },
{
    trace.push((file.id, 2));
    (trace, if file.blob_error { Err("blob 原错\0\n".to_string()) }
        else { Ok((file.id, file.admission_error)) })
}

fn fixture_import_admit(mut trace: CompositionTrace, declared: usize, observed: &(u64, bool))
    -> (out: (CompositionTrace, Result<(), String>))
    ensures out.0@ == trace@.push((observed.0, 3u8)),
        if declared != 7 { match out.1 {
            Err(error) => error@ == "declaration 原错\0\n"@, _ => false,
        } } else if observed.1 { match out.1 {
            Err(error) => error@ == "admission 原错\0\n"@, _ => false,
        } } else { out.1 == Ok::<(), String>(()) },
{
    trace.push((observed.0, 3));
    (trace, if declared != 7 { Err("declaration 原错\0\n".to_string()) }
        else if observed.1 { Err("admission 原错\0\n".to_string()) }
        else { Ok(()) })
}

fn fixture_import_write(mut trace: CompositionTrace, _id: &str, observed: (u64, bool))
    -> (out: CompositionTrace)
    ensures out@ == trace@.push((observed.0, 4u8)),
{
    trace.push((observed.0, 4));
    trace
}

spec fn fixture_import_step_expected(initial: CompositionTrace, id: &str,
    file: &CompositionImportFixture,
    out: (CompositionTrace, Result<(), policy::composition::CompositionImportError>)) -> bool
{
    let missing = id@ == "缺失输入\0\n"@;
    let located = initial@.push((0u64, 1u8));
    let loaded = located.push((file.id, 2u8));
    let admitted = loaded.push((file.id, 3u8));
    out.0@ == if missing { located } else if file.blob_error { loaded }
        else if file.admission_error { admitted } else { admitted.push((file.id, 4u8)) }
    && if missing { match out.1 {
        Err(policy::composition::CompositionImportError::Unregistered(error_id)) => error_id@ == id@,
        _ => false,
    } } else if file.blob_error { match out.1 {
        Err(policy::composition::CompositionImportError::Blob(error)) => error@ == "blob 原错\0\n"@,
        _ => false,
    } } else if file.admission_error { match out.1 {
        Err(policy::composition::CompositionImportError::Admission(error_id, error)) =>
            error_id@ == id@ && error@ == "admission 原错\0\n"@,
        _ => false,
    } } else { out.1 == Ok::<(), policy::composition::CompositionImportError>(()) }
}

proof fn fixture_import_step_bound(initial: CompositionTrace, id: &str,
    file: &CompositionImportFixture,
    out: (CompositionTrace, Result<(), policy::composition::CompositionImportError>))
    requires policy::composition::composition_import_file_result(initial, id, file,
        &fixture_import_lookup, &fixture_import_blob, &fixture_import_admit, &fixture_import_write, out),
    ensures fixture_import_step_expected(initial, id, file, out),
{
    reveal(policy::composition::composition_import_file_result);
}

proof fn fixture_import_prefix_step<'a>(initial: CompositionTrace,
    rows: Seq<(&'a str, &'a CompositionImportFixture)>, states: Seq<CompositionTrace>, index: int)
    requires policy::composition::composition_import_files_prefix(initial, rows, states,
        &fixture_import_lookup, &fixture_import_blob, &fixture_import_admit, &fixture_import_write),
        0 <= index < states.len() - 1,
    ensures fixture_import_step_expected(states[index], rows[index].0, rows[index].1,
        (states[index + 1], Ok::<(), policy::composition::CompositionImportError<'a>>(()))),
{
    reveal(policy::composition::composition_import_files_prefix);
    fixture_import_step_bound(states[index], rows[index].0, rows[index].1,
        (states[index + 1], Ok::<(), policy::composition::CompositionImportError<'a>>(())));
}

spec fn fixture_import_successes(rows: Seq<(&str, &CompositionImportFixture)>, count: int)
    -> Seq<(u64, u8)>
    decreases count,
{
    if count <= 0 { Seq::empty() } else {
        fixture_import_successes(rows, count - 1)
            + seq![(0u64, 1u8), (rows[count - 1].1.id, 2u8),
                (rows[count - 1].1.id, 3u8), (rows[count - 1].1.id, 4u8)]
    }
}

proof fn fixture_import_prefix_trace(initial: CompositionTrace,
    rows: Seq<(&str, &CompositionImportFixture)>, states: Seq<CompositionTrace>, count: int)
    requires policy::composition::composition_import_files_prefix(initial, rows, states,
        &fixture_import_lookup, &fixture_import_blob, &fixture_import_admit, &fixture_import_write),
        0 <= count < states.len(),
    ensures states[count]@ == initial@ + fixture_import_successes(rows, count),
    decreases count,
{
    reveal(policy::composition::composition_import_files_prefix);
    if count > 0 {
        fixture_import_prefix_trace(initial, rows, states, count - 1);
        fixture_import_prefix_step(initial, rows, states, count - 1);
        assert(states[count]@ =~= initial@ + fixture_import_successes(rows, count));
    } else { assert(initial@ + Seq::<(u64, u8)>::empty() =~= initial@); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_import_empty_files_skip_every_stage() {
    use policy::composition::*;
    let rows: Vec<(&str, &CompositionImportFixture)> = Vec::new();
    let (trace, result) = import_composition_files(vec![(u64::MAX, 0u8)], rows.as_slice(),
        &fixture_import_lookup, &fixture_import_blob, &fixture_import_admit, &fixture_import_write);
    let expected = [(u64::MAX, 0u8)];
    assert(trace@ == expected@);
    assert(result.is_ok());
    composition_trace(&expected, trace.as_slice());
    if result.is_err() { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_import_file_preserves_successful_stage_order() {
    use policy::composition::*;
    reveal(composition_import_file_result);
    let file = CompositionImportFixture { id: u64::MAX, blob_error: false, admission_error: false };
    let (trace, result) = import_composition_file(vec![(29u64, 0u8)], "", &file,
        &fixture_import_lookup, &fixture_import_blob, &fixture_import_admit, &fixture_import_write);
    proof {
        reveal_strlit(""); reveal_strlit("缺失输入\0\n");
        assert(""@.len() != "缺失输入\0\n"@.len());
    }
    let expected = [(29u64, 0u8), (0u64, 1u8), (u64::MAX, 2u8), (u64::MAX, 3u8), (u64::MAX, 4u8)];
    assert(trace@ =~= expected@);
    assert(result.is_ok());
    composition_trace(&expected, trace.as_slice());
    if result.is_err() { panic!(); }
}

fn composition_import_files_case(stop: usize, failure: u8)
    requires stop < 3, failure <= 3,
{
    use policy::composition::*;
    let first = CompositionImportFixture { id: 11,
        blob_error: stop == 0 && (failure == 1 || failure == 2),
        admission_error: stop == 0 && failure != 0 };
    let middle = CompositionImportFixture { id: u64::MAX,
        blob_error: stop == 1 && (failure == 1 || failure == 2),
        admission_error: stop == 1 && failure != 0 };
    let last = CompositionImportFixture { id: 47,
        blob_error: stop == 2 && (failure == 1 || failure == 2),
        admission_error: stop == 2 && failure != 0 };
    let first_id = if stop == 0 && failure == 1 { "缺失输入\0\n" } else { "" };
    let middle_id = if stop == 1 && failure == 1 { "缺失输入\0\n" } else { "输入\0\n" };
    let last_id = if stop == 2 && failure == 1 { "缺失输入\0\n" } else { "缺失输入" };
    let rows = [(first_id, &first), (middle_id, &middle), (last_id, &last)];
    let initial = vec![(17u64, 0u8)];
    let ghost before = initial;
    let (trace, result) = import_composition_files(initial, &rows,
        &fixture_import_lookup, &fixture_import_blob, &fixture_import_admit, &fixture_import_write);
    let ghost outcome = result;
    proof {
        reveal_strlit(""); reveal_strlit("输入\0\n"); reveal_strlit("缺失输入");
        reveal_strlit("缺失输入\0\n");
        assert(""@.len() != "缺失输入\0\n"@.len());
        assert("输入\0\n"@.len() != "缺失输入\0\n"@.len());
        assert("缺失输入"@.len() != "缺失输入\0\n"@.len());
        assert forall|i: int| 0 <= i < 3 implies
            (rows@[i].0@ == "缺失输入\0\n"@) == (failure == 1 && i == stop)
            && rows@[i].1.blob_error == ((failure == 1 || failure == 2) && i == stop)
            && rows@[i].1.admission_error == (failure != 0 && i == stop) by {
            if i == 0 { } else if i == 1 { } else { assert(i == 2); }
        }
        let states = choose|states: Seq<CompositionTrace>|
            #[trigger] composition_import_files_prefix(before, rows@, states,
                &fixture_import_lookup, &fixture_import_blob, &fixture_import_admit, &fixture_import_write)
            && match outcome {
                Ok(()) => states.len() == rows@.len() + 1 && trace == states.last(),
                Err(error) => states.len() <= rows@.len()
                    && composition_import_file_result(states.last(), rows@[states.len() - 1].0,
                        rows@[states.len() - 1].1, &fixture_import_lookup, &fixture_import_blob,
                        &fixture_import_admit, &fixture_import_write,
                        (trace, Err::<(), CompositionImportError>(error))),
            };
        reveal(composition_import_files_prefix);
        let count = states.len() - 1;
        fixture_import_prefix_trace(before, rows@, states, count);
        if failure != 0 && count > stop {
            fixture_import_prefix_step(before, rows@, states, stop as int);
            assert(false);
        }
        match outcome {
            Ok(()) => { assert(failure == 0); },
            Err(error) => {
                fixture_import_step_bound(states.last(), rows@[count].0, rows@[count].1,
                    (trace, Err::<(), CompositionImportError>(error)));
                assert(failure != 0 && count == stop);
            },
        }
        reveal_with_fuel(fixture_import_successes, 4);
    }
    if failure == 0 {
        composition_trace(&[(17, 0), (0, 1), (11, 2), (11, 3), (11, 4),
            (0, 1), (u64::MAX, 2), (u64::MAX, 3), (u64::MAX, 4),
            (0, 1), (47, 2), (47, 3), (47, 4)], trace.as_slice());
    } else if stop == 0 {
        if failure == 1 { composition_trace(&[(17, 0), (0, 1)], trace.as_slice()); }
        else if failure == 2 { composition_trace(&[(17, 0), (0, 1), (11, 2)], trace.as_slice()); }
        else { composition_trace(&[(17, 0), (0, 1), (11, 2), (11, 3)], trace.as_slice()); }
    } else if stop == 1 {
        if failure == 1 { composition_trace(&[(17, 0), (0, 1), (11, 2), (11, 3), (11, 4),
            (0, 1)], trace.as_slice()); }
        else if failure == 2 { composition_trace(&[(17, 0), (0, 1), (11, 2), (11, 3), (11, 4),
            (0, 1), (u64::MAX, 2)], trace.as_slice()); }
        else { composition_trace(&[(17, 0), (0, 1), (11, 2), (11, 3), (11, 4),
            (0, 1), (u64::MAX, 2), (u64::MAX, 3)], trace.as_slice()); }
    } else {
        assert(stop == 2);
        if failure == 1 { composition_trace(&[(17, 0), (0, 1), (11, 2), (11, 3), (11, 4),
            (0, 1), (u64::MAX, 2), (u64::MAX, 3), (u64::MAX, 4), (0, 1)], trace.as_slice()); }
        else if failure == 2 { composition_trace(&[(17, 0), (0, 1), (11, 2), (11, 3), (11, 4),
            (0, 1), (u64::MAX, 2), (u64::MAX, 3), (u64::MAX, 4), (0, 1), (47, 2)], trace.as_slice()); }
        else { composition_trace(&[(17, 0), (0, 1), (11, 2), (11, 3), (11, 4),
            (0, 1), (u64::MAX, 2), (u64::MAX, 3), (u64::MAX, 4), (0, 1), (47, 2), (47, 3)], trace.as_slice()); }
    }
    match result {
        Ok(()) => { assert(failure == 0); if failure != 0 { panic!(); } },
        Err(CompositionImportError::Unregistered(id)) => {
            let same = composition_text::equal_text(id, rows[stop].0);
            assert(failure == 1 && same);
            if failure != 1 || !same { panic!(); }
        },
        Err(CompositionImportError::Blob(error)) => {
            let same = composition_text::equal_text(error.as_str(), "blob 原错\0\n");
            assert(failure == 2 && same);
            if failure != 2 || !same { panic!(); }
        },
        Err(CompositionImportError::Admission(id, error)) => {
            let same_id = composition_text::equal_text(id, rows[stop].0);
            let same_error = composition_text::equal_text(error.as_str(), "admission 原错\0\n");
            assert(failure == 3 && same_id && same_error);
            if failure != 3 || !same_id || !same_error { panic!(); }
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_import_files_keep_complete_success_prefix() {
    composition_import_files_case(1, 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_import_files_stop_at_every_first_failure() {
    let mut stop = 0usize;
    while stop < 3
        invariant stop <= 3,
        decreases 3 - stop,
    {
        composition_import_files_case(stop, 1);
        composition_import_files_case(stop, 2);
        composition_import_files_case(stop, 3);
        stop += 1;
    }
}

fn fixture_import_render(parts: [&str; 6], expected: [&str; 6], message: &str) -> (out: String)
    requires forall|i: int| 0 <= i < 6 ==> parts@[i]@ == expected@[i]@,
    ensures out@ == message@,
{
    let mut i = 0usize;
    while i < 6
        invariant i <= 6, forall|j: int| 0 <= j < 6 ==> parts@[j]@ == expected@[j]@,
        decreases 6 - i,
    {
        let same = composition_text::equal_text(parts[i], expected[i]);
        assert(same);
        if !same { panic!(); }
        i += 1;
    }
    message.to_string()
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_import_error_messages_keep_full_endpoint_id_and_suffix() {
    use policy::composition::*;
    let unregistered = CompositionImportError::Unregistered("输入\0\n").render("端点\0\n",
        &|parts: [&str; 6]| -> (out: String)
            requires parts@[0]@ == "E_COMPOSE_INPUT: "@, parts@[1]@ == "端点\0\n"@,
                parts@[2]@ == " unregistered "@, parts@[3]@ == "输入\0\n"@,
                parts@[4]@ == ""@, parts@[5]@ == ""@,
            ensures out@ == "E_COMPOSE_INPUT: 端点\0\n unregistered 输入\0\n"@,
        {
            fixture_import_render(parts, ["E_COMPOSE_INPUT: ", "端点\0\n", " unregistered ",
                "输入\0\n", "", ""], "E_COMPOSE_INPUT: 端点\0\n unregistered 输入\0\n")
        });
    let blob = CompositionImportError::Blob("blob 原错\0\n".to_string()).render("base",
        &|_parts: [&str; 6]| -> String requires false, { panic!(); });
    let admission = CompositionImportError::Admission("", "admission 原错\0\n".to_string())
        .render("candidate", &|parts: [&str; 6]| -> (out: String)
            requires parts@[0]@ == "E_COMPOSE_INPUT: "@, parts@[1]@ == "candidate"@,
                parts@[2]@ == " "@, parts@[3]@ == "admission 原错\0\n"@,
                parts@[4]@ == " "@, parts@[5]@ == ""@,
            ensures out@ == "E_COMPOSE_INPUT: candidate admission 原错\0\n "@,
        {
            fixture_import_render(parts, ["E_COMPOSE_INPUT: ", "candidate", " ",
                "admission 原错\0\n", " ", ""], "E_COMPOSE_INPUT: candidate admission 原错\0\n ")
        });
    let same_unregistered = composition_text::equal_text(unregistered.as_str(),
        "E_COMPOSE_INPUT: 端点\0\n unregistered 输入\0\n");
    let same_blob = composition_text::equal_text(blob.as_str(), "blob 原错\0\n");
    let same_admission = composition_text::equal_text(admission.as_str(),
        "E_COMPOSE_INPUT: candidate admission 原错\0\n ");
    assert(same_unregistered && same_blob && same_admission);
    if !same_unregistered || !same_blob || !same_admission { panic!(); }
}

pub struct CompositionInputFixture<'a> {
    pub ids: &'a [Option<&'a str>],
}

impl<'a> policy::composition::CompositionInputRows for CompositionInputFixture<'a> {
    open spec fn count(&self) -> nat { self.ids@.len() }
    open spec fn id_at(&self, index: int) -> Option<&str> { self.ids@[index] }

    fn len(&self) -> (out: usize)
        ensures out == self.count(),
    { self.ids.len() }

    fn id(&self, index: usize) -> (out: Option<&str>)
        ensures out == self.id_at(index as int),
    { self.ids[index] }
}

proof fn fixture_first_input<R: policy::composition::CompositionInputRows>(
    rows: &R, expected: Seq<char>, first: usize, actual: Option<usize>)
    requires policy::composition::first_composition_input(rows, expected, Some(first)),
        policy::composition::first_composition_input(rows, expected, actual),
    ensures actual == Some(first),
{
    use policy::composition::*;
    match actual {
        None => { assert(!composition_input_matches(rows, first as int, expected)); },
        Some(index) => {
            if index < first { assert(!composition_input_matches(rows, index as int, expected)); }
            else if first < index { assert(!composition_input_matches(rows, first as int, expected)); }
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_import_lookup_preserves_first_match_and_complete_id() {
    use policy::composition::*;
    let ids = [None, Some("input"), Some("输入\0\n"), Some("input"), Some("")];
    let rows = CompositionInputFixture { ids: &ids };
    let ordinary = find_composition_input(&rows, "input");
    let unicode = find_composition_input(&rows, "输入\0\n");
    let empty = find_composition_input(&rows, "");
    let case = find_composition_input(&rows, "Input");
    let prefix = find_composition_input(&rows, "输入");
    proof {
        reveal_strlit("input"); reveal_strlit("输入\0\n"); reveal_strlit("");
        reveal_strlit("Input"); reveal_strlit("输入");
        assert("input"@.len() == 5);
        assert("输入\0\n"@.len() == 4);
        assert(""@.len() == 0);
        assert("input"@ != "输入\0\n"@);
        assert("input"@ != ""@);
        assert("输入\0\n"@ != ""@);
        assert(composition_input_matches(&rows, 2, "输入\0\n"@));
        assert forall|j: int| 0 <= j < 2 implies
            !composition_input_matches(&rows, j, "输入\0\n"@) by {
            assert(rows.id_at(j) == rows.ids@[j]);
            if j == 0 { assert(rows.id_at(j).is_none()); }
            else { assert(j == 1); assert(rows.id_at(j).unwrap()@ == "input"@); }
        }
        assert(composition_input_matches(&rows, 4, ""@));
        assert forall|j: int| 0 <= j < 4 implies
            !composition_input_matches(&rows, j, ""@) by {
            assert(rows.id_at(j) == rows.ids@[j]);
            if j == 0 { assert(rows.id_at(j).is_none()); }
            else if j == 1 { assert(rows.id_at(j).unwrap()@ == "input"@); }
            else if j == 2 { assert(rows.id_at(j).unwrap()@ == "输入\0\n"@); }
            else { assert(j == 3); assert(rows.id_at(j).unwrap()@ == "input"@); }
        }
        fixture_first_input(&rows, "input"@, 1, ordinary);
        fixture_first_input(&rows, "输入\0\n"@, 2, unicode);
        fixture_first_input(&rows, ""@, 4, empty);
        assert("Input"@.len() == 5);
        assert("输入"@.len() == 2);
        assert("input"@ != "Input"@) by { assert("input"@[0] != "Input"@[0]); }
        assert forall|j: int| 0 <= j < 5 implies
            !composition_input_matches(&rows, j, "Input"@)
            && !composition_input_matches(&rows, j, "输入"@) by {
            assert(rows.id_at(j) == rows.ids@[j]);
            if j == 0 { assert(rows.id_at(j).is_none()); }
            else if j == 1 || j == 3 { assert(rows.id_at(j).unwrap()@ == "input"@); }
            else if j == 2 { assert(rows.id_at(j).unwrap()@ == "输入\0\n"@); }
            else { assert(j == 4); assert(rows.id_at(j).unwrap()@ == ""@); }
        }
        if let Some(index) = case { assert(!composition_input_matches(&rows, index as int, "Input"@)); }
        if let Some(index) = prefix { assert(!composition_input_matches(&rows, index as int, "输入"@)); }
    }
    assert(ordinary == Some(1usize));
    assert(unicode == Some(2usize));
    assert(empty == Some(4usize));
    assert(case.is_none());
    assert(prefix.is_none());
    if ordinary != Some(1) || unicode != Some(2) || empty != Some(4)
        || case.is_some() || prefix.is_some() { panic!(); }
    let no_ids: [Option<&str>; 0] = [];
    let no_rows = CompositionInputFixture { ids: &no_ids };
    let missing = find_composition_input(&no_rows, "input");
    assert(missing.is_none());
    if missing.is_some() { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_merge_rejects_environment_before_files() {
    use policy::composition::*;
    reveal(composition_merge_endpoint_result);
    let initial = 17u64;
    let snapshot = 29u64;
    let (state, result) = merge_composition_endpoint(initial, CompositionEndpoint::Base,
        Some(&snapshot),
        &|_state: &u64, _endpoint: CompositionEndpoint| -> (present: bool)
            ensures present,
        { true },
        &|state: u64, _endpoint: CompositionEndpoint, _snapshot: &u64| -> (out: u64)
            ensures out == state,
        { state },
        &|_state: &u64, _endpoint: CompositionEndpoint, _snapshot: &u64| -> (same: bool)
            ensures !same,
        { false },
        &|_snapshot: &u64| -> (rows: Result<Vec<(&str, &u64)>, String>)
            ensures match rows { Err(error) => error@ == "snapshot files"@, _ => false },
        { Err("snapshot files".to_string()) },
        &|_state: &u64, _endpoint: CompositionEndpoint, _id: &str, _value: &u64|
            -> (same: Option<bool>) ensures same == Some(false),
        { Some(false) },
        &|state: u64, _endpoint: CompositionEndpoint, _id: &str, _value: &u64|
            -> (out: u64) ensures out == state,
        { state });
    assert(state == 17);
    if state != 17 { panic!(); }
    match result {
        Err(CompositionMergeError::Environment(endpoint)) => {
            assert(endpoint == CompositionEndpoint::Base);
            if !matches!(endpoint, CompositionEndpoint::Base) { panic!(); }
        },
        _ => { assert(false); panic!(); },
    }
}

fn composition_merge_files_error_case(existing: bool) {
    use policy::composition::*;
    reveal(composition_merge_endpoint_result);
    let initial = vec![17u64];
    let snapshot = 29u64;
    let (state, result) = merge_composition_endpoint(initial, CompositionEndpoint::Candidate,
        Some(&snapshot),
        &|_state: &Vec<u64>, _endpoint: CompositionEndpoint| -> (present: bool)
            ensures present == existing,
        { existing },
        &|mut state: Vec<u64>, _endpoint: CompositionEndpoint, snapshot: &u64| -> (out: Vec<u64>)
            ensures out@ == state@.push(*snapshot),
        { state.push(*snapshot); state },
        &|_state: &Vec<u64>, _endpoint: CompositionEndpoint, _snapshot: &u64| -> (same: bool)
            ensures same,
        { true },
        &|_snapshot: &u64| -> (rows: Result<Vec<(&str, &u64)>, String>)
            ensures match rows { Err(error) => error@ == "snapshot files 原错\0\n"@, _ => false },
        { Err("snapshot files 原错\0\n".to_string()) },
        &|_state: &Vec<u64>, _endpoint: CompositionEndpoint, _id: &str, _value: &u64|
            -> (same: Option<bool>) ensures same == Some(false),
        { Some(false) },
        &|state: Vec<u64>, _endpoint: CompositionEndpoint, _id: &str, _value: &u64|
            -> (out: Vec<u64>) ensures out == state,
        { state });
    if existing { fixture_merge_trace(&[17], state.as_slice()); }
    else { fixture_merge_trace(&[17, 29], state.as_slice()); }
    match result {
        Err(CompositionMergeError::Files(error)) => {
            let same = composition_text::equal_text(error.as_str(), "snapshot files 原错\0\n");
            assert(same);
            if !same { panic!(); }
        },
        _ => { assert(false); panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_merge_initialization_and_original_files_error_are_preserved() {
    composition_merge_files_error_case(false);
    composition_merge_files_error_case(true);
}

fn fixture_merge_prior(_state: &Vec<u64>, _endpoint: policy::composition::CompositionEndpoint,
    _id: &str, value: &u64) -> (same: Option<bool>)
    ensures same == if *value == 0 { Some(false) } else if *value == 1 { Some(true) } else { None },
{
    if *value == 0 { Some(false) } else if *value == 1 { Some(true) } else { None }
}

fn fixture_merge_write(mut state: Vec<u64>, _endpoint: policy::composition::CompositionEndpoint,
    _id: &str, value: &u64) -> (out: Vec<u64>)
    ensures out@ == state@.push(*value),
{
    state.push(*value);
    state
}

spec fn fixture_merge_values(rows: Seq<(&str, &u64)>, count: int) -> Seq<u64>
    decreases count,
{
    if count <= 0 { Seq::empty() }
    else { fixture_merge_values(rows, count - 1).push(*rows[count - 1].1) }
}

proof fn fixture_merge_prefix_step<'a, S, V: 'a, P, W>(initial: S,
    endpoint: policy::composition::CompositionEndpoint, rows: Seq<(&'a str, &'a V)>,
    states: Seq<S>, index: int, prior: &P, write: &W)
    where P: Fn(&S, policy::composition::CompositionEndpoint, &'a str, &'a V) -> Option<bool>,
        W: Fn(S, policy::composition::CompositionEndpoint, &'a str, &'a V) -> S,
    requires policy::composition::composition_merge_files_prefix(initial, endpoint, rows, states, prior, write),
        0 <= index < states.len() - 1,
    ensures write.ensures((states[index], endpoint, rows[index].0, rows[index].1), states[index + 1]),
        exists|same: Option<bool>| #[trigger] prior.ensures(
            (&states[index], endpoint, rows[index].0, rows[index].1), same) && same != Some(false),
{
    reveal(policy::composition::composition_merge_files_prefix);
}

proof fn fixture_merge_prefix_state(initial: Vec<u64>,
    endpoint: policy::composition::CompositionEndpoint, rows: Seq<(&str, &u64)>,
    states: Seq<Vec<u64>>, index: int)
    requires policy::composition::composition_merge_files_prefix(initial, endpoint, rows, states,
            &fixture_merge_prior, &fixture_merge_write),
        0 <= index < states.len(),
    ensures states[index]@ == initial@ + fixture_merge_values(rows, index),
        forall|j: int| 0 <= j < index ==> *(#[trigger] rows[j]).1 != 0,
    decreases index,
{
    reveal(policy::composition::composition_merge_files_prefix);
    if index > 0 {
        fixture_merge_prefix_state(initial, endpoint, rows, states, index - 1);
        fixture_merge_prefix_step(initial, endpoint, rows, states, index - 1,
            &fixture_merge_prior, &fixture_merge_write);
        assert(fixture_merge_write.ensures((states[index - 1], endpoint,
            rows[index - 1].0, rows[index - 1].1), states[index]));
        assert(exists|same: Option<bool>| #[trigger] fixture_merge_prior.ensures(
            (&states[index - 1], endpoint, rows[index - 1].0, rows[index - 1].1), same)
            && same != Some(false));
        assert(states[index]@ == states[index - 1]@.push(*rows[index - 1].1));
        assert(states[index]@ =~= initial@ + fixture_merge_values(rows, index));
    }
}

fn fixture_merge_trace(expected: &[u64], actual: &[u64])
    requires expected@ =~= actual@,
{
    if expected.len() != actual.len() { panic!(); }
    let mut i = 0usize;
    while i < expected.len()
        invariant expected@ == actual@, i <= expected.len(),
        decreases expected.len() - i,
    {
        if expected[i] != actual[i] { panic!(); }
        i += 1;
    }
}

fn composition_merge_file_case(first: u64, second: u64, third: u64) {
    use policy::composition::*;
    let rows = [("首\0\n", &first), ("", &second), ("末\0\n", &third)];
    let initial = vec![u64::MAX];
    let ghost before = initial;
    let (state, result) = merge_composition_files(initial, CompositionEndpoint::Candidate,
        rows.as_slice(), &fixture_merge_prior, &fixture_merge_write);
    proof {
        reveal_with_fuel(fixture_merge_values, 4);
        let states = choose|states: Seq<Vec<u64>>|
            #[trigger] composition_merge_files_prefix(before, CompositionEndpoint::Candidate, rows@,
                states, &fixture_merge_prior, &fixture_merge_write)
            && state == states.last() && match result {
                Ok(()) => states.len() == rows@.len() + 1,
                Err(error) => states.len() <= rows@.len()
                    && error == CompositionMergeError::Conflict(CompositionEndpoint::Candidate,
                        rows@[states.len() - 1].0)
                    && fixture_merge_prior.ensures((&states.last(), CompositionEndpoint::Candidate,
                        rows@[states.len() - 1].0, rows@[states.len() - 1].1), Some(false)),
            };
        fixture_merge_prefix_state(before, CompositionEndpoint::Candidate, rows@,
            states, states.len() - 1);
        if first == 0 { assert(states.len() == 1); }
        else if second == 0 { assert(states.len() == 2); }
        else if third == 0 { assert(states.len() == 3); }
        else { assert(states.len() == 4); }
    }
    if first == 0 { fixture_merge_trace(&[u64::MAX], state.as_slice()); }
    else if second == 0 { fixture_merge_trace(&[u64::MAX, first], state.as_slice()); }
    else if third == 0 { fixture_merge_trace(&[u64::MAX, first, second], state.as_slice()); }
    else { fixture_merge_trace(&[u64::MAX, first, second, third], state.as_slice()); }
    if first == 0 || second == 0 || third == 0 {
        match result {
            Err(CompositionMergeError::Conflict(endpoint, id)) => {
                assert(endpoint == CompositionEndpoint::Candidate);
                if !matches!(endpoint, CompositionEndpoint::Candidate) { panic!(); }
                let expected = if first == 0 { "首\0\n" } else if second == 0 { "" } else { "末\0\n" };
                let same = composition_text::equal_text(id, expected);
                assert(same);
                if !same { panic!(); }
            },
            _ => { assert(false); panic!(); },
        }
    } else {
        assert(result.is_ok());
        if result.is_err() { panic!(); }
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_merge_keeps_successful_prefix_and_exact_first_conflict() {
    composition_merge_file_case(1, 2, u64::MAX);
    composition_merge_file_case(0, 0, 0);
    composition_merge_file_case(1, 0, 0);
    composition_merge_file_case(1, 2, 0);
    composition_merge_file_case(u64::MAX, 1, 2);
    use policy::composition::*;
    let initial = vec![u64::MAX];
    let rows: Vec<(&str, &u64)> = Vec::new();
    let (state, result) = merge_composition_files(initial, CompositionEndpoint::Base,
        rows.as_slice(), &fixture_merge_prior, &fixture_merge_write);
    assert(result.is_ok());
    if result.is_err() { panic!(); }
    fixture_merge_trace(&[u64::MAX], state.as_slice());
}

fn fixture_merge_present(_state: &Vec<u64>, _endpoint: policy::composition::CompositionEndpoint)
    -> (present: bool) ensures !present,
{ false }

fn fixture_merge_initialize(mut state: Vec<u64>, endpoint: policy::composition::CompositionEndpoint,
    _snapshot: &bool) -> (out: Vec<u64>)
    ensures out@ == state@.push(match endpoint {
        policy::composition::CompositionEndpoint::Base => 10u64,
        policy::composition::CompositionEndpoint::Candidate => 20u64,
    }),
{
    state.push(match endpoint {
        policy::composition::CompositionEndpoint::Base => 10,
        policy::composition::CompositionEndpoint::Candidate => 20,
    });
    state
}

fn fixture_merge_environment(_state: &Vec<u64>, _endpoint: policy::composition::CompositionEndpoint,
    snapshot: &bool) -> (same: bool) ensures same == *snapshot,
{ *snapshot }

fn fixture_merge_empty_files(_snapshot: &bool) -> (rows: Result<Vec<(&str, &u64)>, String>)
    ensures match rows { Ok(rows) => rows@.len() == 0, _ => false },
{ Ok(Vec::new()) }

proof fn fixture_merge_empty_endpoint(initial: Vec<u64>, endpoint: policy::composition::CompositionEndpoint,
    snapshot: Option<&bool>, out: (Vec<u64>, Result<(), policy::composition::CompositionMergeError>))
    requires policy::composition::composition_merge_endpoint_result(initial, endpoint, snapshot,
        &fixture_merge_present, &fixture_merge_initialize, &fixture_merge_environment,
        &fixture_merge_empty_files, &fixture_merge_prior, &fixture_merge_write, out),
    ensures out.0@ == if snapshot.is_some() { initial@.push(match endpoint {
            policy::composition::CompositionEndpoint::Base => 10u64,
            policy::composition::CompositionEndpoint::Candidate => 20u64,
        }) } else { initial@ },
        match snapshot {
            Some(value) => if !*value { out.1 == Err::<(), policy::composition::CompositionMergeError>(
                policy::composition::CompositionMergeError::Environment(endpoint)) } else { out.1.is_ok() },
            None => out.1.is_ok(),
        },
{
    reveal(policy::composition::composition_merge_endpoint_result);
}

fn composition_merge_endpoint_case(base: Option<&bool>, candidate: Option<&bool>) {
    use policy::composition::*;
    let initial = vec![u64::MAX];
    let ghost before = initial;
    let (state, result) = merge_composition_part(initial, base, candidate,
        &fixture_merge_present, &fixture_merge_initialize, &fixture_merge_environment,
        &fixture_merge_empty_files, &fixture_merge_prior, &fixture_merge_write);
    proof {
        let (middle, first) = choose|middle: Vec<u64>, first: Result<(), CompositionMergeError>|
            #[trigger] composition_merge_endpoint_result(before, CompositionEndpoint::Base, base,
                &fixture_merge_present, &fixture_merge_initialize, &fixture_merge_environment,
                &fixture_merge_empty_files, &fixture_merge_prior, &fixture_merge_write, (middle, first))
            && match first {
                Err(error) => state == middle && result == Err::<(), CompositionMergeError>(error),
                Ok(()) => composition_merge_endpoint_result(middle, CompositionEndpoint::Candidate, candidate,
                    &fixture_merge_present, &fixture_merge_initialize, &fixture_merge_environment,
                    &fixture_merge_empty_files, &fixture_merge_prior, &fixture_merge_write, (state, result)),
            };
        fixture_merge_empty_endpoint(before, CompositionEndpoint::Base, base, (middle, first));
        if first.is_ok() {
            fixture_merge_empty_endpoint(middle, CompositionEndpoint::Candidate, candidate, (state, result));
        }
    }
    match base {
        Some(value) => if !*value {
            fixture_merge_trace(&[u64::MAX, 10], state.as_slice());
            assert(result == Err::<(), CompositionMergeError>(CompositionMergeError::Environment(CompositionEndpoint::Base)));
        } else {
            match candidate {
                None => fixture_merge_trace(&[u64::MAX, 10], state.as_slice()),
                Some(_) => fixture_merge_trace(&[u64::MAX, 10, 20], state.as_slice()),
            }
        },
        None => match candidate {
            None => fixture_merge_trace(&[u64::MAX], state.as_slice()),
            Some(_) => fixture_merge_trace(&[u64::MAX, 20], state.as_slice()),
        },
    }
    let failure = match base { Some(value) => !*value, None => false }
        || match candidate { Some(value) => !*value, None => false };
    assert(result.is_err() == failure);
    if result.is_err() != failure { panic!(); }
    match result {
        Err(CompositionMergeError::Environment(endpoint)) => {
            let base_failed = match base { Some(value) => !*value, None => false };
            match endpoint {
                CompositionEndpoint::Base => { assert(base_failed); if !base_failed { panic!(); } },
                CompositionEndpoint::Candidate => { assert(!base_failed); if base_failed { panic!(); } },
            }
        },
        Err(_) => { assert(false); panic!(); },
        Ok(()) => {},
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_merge_initializes_skips_and_preserves_base_before_candidate() {
    let yes = true;
    let no = false;
    composition_merge_endpoint_case(None, None);
    composition_merge_endpoint_case(Some(&yes), None);
    composition_merge_endpoint_case(None, Some(&yes));
    composition_merge_endpoint_case(Some(&yes), Some(&yes));
    composition_merge_endpoint_case(Some(&no), Some(&no));
    composition_merge_endpoint_case(Some(&no), Some(&yes));
    composition_merge_endpoint_case(Some(&yes), Some(&no));
    composition_merge_endpoint_case(None, Some(&no));
}

proof fn input_fixture_prefix_member(args: Seq<String>, kind: policy::InputKind, pairs: int, index: int)
    requires policy::input_prefix_valid(args, kind, pairs), 0 <= index < pairs,
    ensures policy::input_key_allowed(kind, args[1 + 2 * index]@),
{
}

proof fn input_fixture_prefix_distinct(args: Seq<String>, kind: policy::InputKind, pairs: int,
    left: int, right: int)
    requires policy::input_prefix_valid(args, kind, pairs), 0 <= left < right < pairs,
    ensures args[1 + 2 * left]@ != args[1 + 2 * right]@,
{
    assert(policy::input_key_allowed(kind, args[1 + 2 * right]@));
}

fn input_parse_error(args: &[String], expected: &str)
    requires match policy::input_command_result(args@) {
        Some(error) => error == expected@,
        None => false,
    },
{
    match policy::parse_input_command(args) {
        Err(error) => {
            let same = composition_text::equal_text(error.as_str(), expected);
            assert(same);
            if !same { panic!(); }
        },
        Ok(_) => { assert(false); panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn input_commands_keep_exact_modes_and_first_option_error() {
    proof {
        reveal(policy::input_command_result);
        reveal(policy::input_kind);
        reveal(policy::input_prefix_valid);
        reveal(policy::input_key_allowed);
        reveal_strlit("capture"); reveal_strlit("compose"); reveal_strlit("pair"); reveal_strlit("Capture");
        reveal_strlit("--help"); reveal_strlit("--version"); reveal_strlit("--base");
        reveal_strlit("--host-root"); reveal_strlit("--config"); reveal_strlit("--commit");
        reveal_strlit("--output"); reveal_strlit("--unit"); reveal_strlit("--candidate");
        reveal_strlit("--manifest"); reveal_strlit("--receipt"); reveal_strlit("--unknown");
        reveal_strlit("--base-snapshot"); reveal_strlit("--candidate-snapshot");
        reveal_strlit("--base-root"); reveal_strlit("--candidate-root");
        assert("capture"@.len() == 7); assert("compose"@.len() == 7); assert("pair"@.len() == 4);
        assert("--version"@.len() == 9); assert("--help"@.len() == 6);
        assert("capture"@[0] == 'c'); assert("compose"@[0] == 'c');
        assert(policy::input_kind("--version"@).is_none());
        assert("Capture"@[0] == 'C'); assert("Capture"@.len() == 7);
        assert(policy::input_kind("Capture"@).is_none());
        assert("--host-root"@.len() == 11); assert("--config"@.len() == 8);
        assert("--commit"@.len() == 8); assert("--output"@.len() == 8);
        assert("--config"@[2] == 'c'); assert("--output"@[2] == 'o');
        assert("--unit"@.len() == 6); assert("--unit"@[2] == 'u');
        assert("--base"@.len() == 6); assert("--base"@[2] == 'b');
        assert("--base-root"@.len() == 11); assert("--candidate-root"@.len() == 16);
        assert("--base-snapshot"@.len() == 15); assert("--candidate-snapshot"@.len() == 20);
        reveal_strlit("capture\0"); assert("capture\0"@.len() == 8);
        reveal_strlit("--output\0"); assert("--output\0"@.len() == 9);
        assert(!policy::input_key_allowed(policy::InputKind::Capture, "--base"@));
        assert(!policy::input_key_allowed(policy::InputKind::Capture, "--output\0"@));
    }
    input_parse_error(&[], "E_USAGE: capture or pair required");
    input_parse_error(&["--version".to_string(), "x".to_string()], "E_USAGE: unknown command");
    input_parse_error(&["--help".to_string(), "x".to_string()], "E_USAGE: unknown command");
    input_parse_error(&["Capture".to_string()], "E_USAGE: unknown command");
    input_parse_error(&["capture\0".to_string()], "E_USAGE: unknown command");
    let args = ["capture".to_string(), "--base".to_string(), "x".to_string()];
    proof {
        if policy::input_prefix_valid(args@, policy::InputKind::Capture, 1) {
            input_fixture_prefix_member(args@, policy::InputKind::Capture, 1, 0);
        }
        assert(!policy::input_prefix_valid(args@, policy::InputKind::Capture, 1));
    }
    input_parse_error(&args, "E_USAGE: unknown or duplicate option");
    input_parse_error(&["capture".to_string(), "--base".to_string()],
        "E_USAGE: option needs a value");
    let args = ["capture".to_string(), "--output\0".to_string(), "x".to_string()];
    proof {
        if policy::input_prefix_valid(args@, policy::InputKind::Capture, 1) {
            input_fixture_prefix_member(args@, policy::InputKind::Capture, 1, 0);
        }
        assert(!policy::input_prefix_valid(args@, policy::InputKind::Capture, 1));
    }
    input_parse_error(&args, "E_USAGE: unknown or duplicate option");
    let args = ["pair".to_string(), "--config".to_string(), "x".to_string(), "--unknown".to_string()];
    proof {
        if policy::input_prefix_valid(args@, policy::InputKind::Pair, 1) {
            input_fixture_prefix_member(args@, policy::InputKind::Pair, 1, 0);
        }
        assert(!policy::input_prefix_valid(args@, policy::InputKind::Pair, 1));
    }
    input_parse_error(&args, "E_USAGE: unknown or duplicate option");
    let args = ["compose".to_string(), "--output".to_string(), "".to_string(),
        "--output".to_string(), "second".to_string(), "--unknown".to_string()];
    proof {
        assert("capture"@[1] == 'a'); assert("compose"@[1] == 'o');
        if policy::input_prefix_valid(args@, policy::InputKind::Compose, 2) {
            input_fixture_prefix_distinct(args@, policy::InputKind::Compose, 2, 0, 1);
        }
        assert(!policy::input_prefix_valid(args@, policy::InputKind::Compose, 2));
    }
    input_parse_error(&args, "E_USAGE: unknown or duplicate option");
}

fn input_valid_command(args: &[String], kind: policy::InputKind) -> (command: policy::InputCommand<'_>)
    requires args.len() > 0, args.len() % 2 == 1,
        policy::input_kind(args@[0]@) == Some(kind),
        policy::input_prefix_valid(args@, kind, (args.len() - 1) / 2),
    ensures policy::input_command_valid(command), command.arguments@ == args@, command.kind == kind,
{
    proof {
        reveal_strlit("capture"); reveal_strlit("compose"); reveal_strlit("pair");
        reveal_strlit("--version"); reveal_strlit("--help");
        reveal(policy::input_command_result);
        reveal(policy::input_kind);
        assert("capture"@.len() == 7); assert("compose"@.len() == 7); assert("pair"@.len() == 4);
        assert("--version"@.len() == 9); assert("--help"@.len() == 6);
        assert("capture"@[0] == 'c'); assert("compose"@[0] == 'c');
        assert(policy::input_kind("--version"@).is_none());
        assert(policy::input_kind("--help"@).is_none());
    }
    match policy::parse_input_command(args) {
        Ok(policy::InputAction::Command(command)) => command,
        _ => { assert(false); panic!(); },
    }
}

fn input_value(command: &policy::InputCommand, key: &str, expected: &str, index: usize)
    requires policy::input_command_valid(*command),
        index < (command.arguments@.len() - 1) / 2,
        command.arguments@[1 + 2 * index]@ == key@,
        command.arguments@[2 + 2 * index]@ == expected@,
{
    match command.required(key) {
        Ok(value) => {
            let same = composition_text::equal_text(value, expected);
            assert(same);
            if !same { panic!(); }
        },
        Err(_) => { assert(false); panic!(); },
    }
}

fn input_missing(command: &policy::InputCommand, key: &str)
    requires policy::input_command_valid(*command),
        forall|i: int| 0 <= i < (command.arguments@.len() - 1) / 2
            ==> (#[trigger] command.arguments@[1 + 2 * i])@ != key@,
{
    let absent = command.option(key).is_none();
    assert(absent);
    if !absent { panic!(); }
    match command.required(key) {
        Err(missing) => {
            let same = composition_text::equal_text(missing.key, key);
            assert(same);
            if !same { panic!(); }
        },
        Ok(_) => { assert(false); panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn input_commands_preserve_all_values_and_missing_keys() {
    use policy::InputKind;
    proof {
        reveal(policy::input_prefix_valid);
        reveal(policy::input_key_allowed);
        reveal_strlit("capture"); reveal_strlit("compose"); reveal_strlit("pair");
        assert("capture"@.len() == 7); assert("compose"@.len() == 7); assert("pair"@.len() == 4);
        assert("capture"@[1] == 'a'); assert("compose"@[1] == 'o');
        reveal_strlit("--help"); reveal_strlit("--version");
        reveal_strlit("--host-root"); reveal_strlit("--config"); reveal_strlit("--commit");
        reveal_strlit("--output"); reveal_strlit("--unit"); reveal_strlit("--base");
        reveal_strlit("--candidate"); reveal_strlit("--manifest"); reveal_strlit("--receipt");
        reveal_strlit("--base-snapshot"); reveal_strlit("--candidate-snapshot");
        reveal_strlit("--base-root"); reveal_strlit("--candidate-root"); reveal_strlit("缺键\0\n");
        assert("--host-root"@.len() == 11); assert("--host-root"@[2] == 'h'); assert("--host-root"@[4] == 's');
        assert("--config"@.len() == 8); assert("--config"@[2] == 'c'); assert("--config"@[4] == 'n');
        assert("--commit"@.len() == 8); assert("--commit"@[2] == 'c'); assert("--commit"@[4] == 'm');
        assert("--output"@.len() == 8); assert("--output"@[2] == 'o'); assert("--output"@[4] == 't');
        assert("--unit"@.len() == 6); assert("--unit"@[2] == 'u'); assert("--unit"@[4] == 'i');
        assert("--base"@.len() == 6); assert("--base"@[2] == 'b'); assert("--base"@[4] == 's');
        assert("--candidate"@.len() == 11); assert("--candidate"@[2] == 'c'); assert("--candidate"@[4] == 'n');
        assert("--manifest"@.len() == 10); assert("--manifest"@[2] == 'm'); assert("--manifest"@[4] == 'n');
        assert("--receipt"@.len() == 9); assert("--receipt"@[2] == 'r'); assert("--receipt"@[4] == 'c');
        assert("--base-snapshot"@.len() == 15); assert("--base-snapshot"@[2] == 'b'); assert("--base-snapshot"@[4] == 's');
        assert("--candidate-snapshot"@.len() == 20); assert("--candidate-snapshot"@[2] == 'c'); assert("--candidate-snapshot"@[4] == 'n');
        assert("--base-root"@.len() == 11); assert("--base-root"@[2] == 'b'); assert("--base-root"@[4] == 's');
        assert("--candidate-root"@.len() == 16); assert("--candidate-root"@[2] == 'c'); assert("--candidate-root"@[4] == 'n');
        assert("缺键\0\n"@.len() == 4);

    }
    let args = ["capture".to_string(), "--unit".to_string(), "--output".to_string(),
        "--commit".to_string(), "原值\0\n".to_string(), "--output".to_string(), "".to_string(),
        "--config".to_string(), "配置".to_string(), "--host-root".to_string(), "根".to_string()];
    let args: &[String] = &args;
    proof {
        assert(args@[1]@ == "--unit"@);
        assert(args@[3]@ == "--commit"@);
        assert(args@[5]@ == "--output"@);
        assert(args@[7]@ == "--config"@);
        assert(args@[9]@ == "--host-root"@);
        assert forall|i: int| 0 <= i < 5 implies
            policy::input_key_allowed(InputKind::Capture, (#[trigger] args@[1 + 2 * i])@)
                && forall|j: int| 0 <= j < i ==> (#[trigger] args@[1 + 2 * j])@ != args@[1 + 2 * i]@ by {
            assert(i == 0 || i == 1 || i == 2 || i == 3 || i == 4);
            assert forall|j: int| 0 <= j < i implies (#[trigger] args@[1 + 2 * j])@ != args@[1 + 2 * i]@ by {
                assert(j == 0 || j == 1 || j == 2 || j == 3);
            }
        }
    }
    let command = input_valid_command(args, InputKind::Capture);
    input_value(&command, "--unit", "--output", 0);
    input_value(&command, "--commit", "原值\0\n", 1);
    input_value(&command, "--output", "", 2);
    input_value(&command, "--config", "配置", 3);
    input_value(&command, "--host-root", "根", 4);
    input_missing(&command, "缺键\0\n");
    let args = ["compose".to_string(), "--receipt".to_string(), "r".to_string(),
        "--manifest".to_string(), "m".to_string(), "--candidate".to_string(), "c".to_string(),
        "--base".to_string(), "b".to_string(), "--output".to_string(), "o".to_string(),
        "--config".to_string(), "p".to_string(), "--host-root".to_string(), "h".to_string()];
    let args: &[String] = &args;
    proof {
        assert forall|i: int| 0 <= i < 7 implies
            policy::input_key_allowed(InputKind::Compose, (#[trigger] args@[1 + 2 * i])@)
                && forall|j: int| 0 <= j < i ==> (#[trigger] args@[1 + 2 * j])@ != args@[1 + 2 * i]@ by {
            assert(i == 0 || i == 1 || i == 2 || i == 3 || i == 4 || i == 5 || i == 6);
            assert forall|j: int| 0 <= j < i implies (#[trigger] args@[1 + 2 * j])@ != args@[1 + 2 * i]@ by {
                assert(j == 0 || j == 1 || j == 2 || j == 3 || j == 4 || j == 5);
            }
        }
    }
    let command = input_valid_command(args, InputKind::Compose);
    input_value(&command, "--receipt", "r", 0);
    input_value(&command, "--manifest", "m", 1);
    input_value(&command, "--candidate", "c", 2);
    input_value(&command, "--base", "b", 3);
    input_value(&command, "--output", "o", 4);
    input_value(&command, "--config", "p", 5);
    input_value(&command, "--host-root", "h", 6);
    input_missing(&command, "--unit");
    let args = ["pair".to_string(), "--candidate-root".to_string(), "cr".to_string(),
        "--base-root".to_string(), "br".to_string(), "--output".to_string(), "o".to_string(),
        "--candidate-snapshot".to_string(), "cs".to_string(), "--base-snapshot".to_string(), "bs".to_string(),
        "--host-root".to_string(), "h".to_string()];
    let args: &[String] = &args;
    proof {
        assert forall|i: int| 0 <= i < 6 implies
            policy::input_key_allowed(InputKind::Pair, (#[trigger] args@[1 + 2 * i])@)
                && forall|j: int| 0 <= j < i ==> (#[trigger] args@[1 + 2 * j])@ != args@[1 + 2 * i]@ by {
            assert(i == 0 || i == 1 || i == 2 || i == 3 || i == 4 || i == 5);
            assert forall|j: int| 0 <= j < i implies (#[trigger] args@[1 + 2 * j])@ != args@[1 + 2 * i]@ by {
                assert(j == 0 || j == 1 || j == 2 || j == 3 || j == 4);
            }
        }
    }
    let command = input_valid_command(args, InputKind::Pair);
    input_value(&command, "--candidate-root", "cr", 0);
    input_value(&command, "--base-root", "br", 1);
    input_value(&command, "--output", "o", 2);
    input_value(&command, "--candidate-snapshot", "cs", 3);
    input_value(&command, "--base-snapshot", "bs", 4);
    input_value(&command, "--host-root", "h", 5);
    input_missing(&command, "--config");
    let args = ["pair".to_string()];
    let command = input_valid_command(&args, InputKind::Pair);
    input_missing(&command, "--host-root");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn input_commands_accept_only_single_argument_help_and_version() {
    proof {
        reveal(policy::input_command_result);
        reveal(policy::input_kind);
        reveal_strlit("capture"); reveal_strlit("compose"); reveal_strlit("pair");
        reveal_strlit("--version"); reveal_strlit("--help");
        assert("capture"@.len() == 7); assert("compose"@.len() == 7); assert("pair"@.len() == 4);
        assert("--version"@.len() == 9); assert("--help"@.len() == 6);
        assert("capture"@[0] == 'c'); assert("compose"@[0] == 'c');
        assert(policy::input_kind("--version"@).is_none());
        assert(policy::input_kind("--help"@).is_none());
    }
    let args = ["--version".to_string()];
    assert(args@[0]@ == "--version"@);
    let version = matches!(policy::parse_input_command(&args), Ok(policy::InputAction::Version));
    assert(version);
    if !version { panic!(); }
    let args = ["--help".to_string()];
    let help = matches!(policy::parse_input_command(&args), Ok(policy::InputAction::Help));
    assert(help);
    if !help { panic!(); }
}

fn composition_result(result: Result<(), String>, success: bool, expected: &str)
    requires result.is_ok() == success,
        match result { Err(error) => error@ == expected@, _ => true, },
{
    let matches = match result {
        Ok(()) => success,
        Err(error) => !success && composition_text::equal_text(error.as_str(), expected),
    };
    assert(matches);
    if !matches { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_bounds_manifest_and_path_identity() {
    use policy::composition::*;
    proof {
        reveal_strlit("chrono-input-composition/v1"); reveal_strlit("chrono-input-composition/v1\0");
        reveal_strlit("原件\0\n"); reveal_strlit("原件\0\n ");
        assert("chrono-input-composition/v1"@.len() != "chrono-input-composition/v1\0"@.len());
        assert("原件\0\n"@.len() != "原件\0\n "@.len());
    }
    composition_result(admit_json_original(0), true, "");
    composition_result(admit_json_original(64 * 1024 * 1024), true, "");
    composition_result(admit_json_original(64 * 1024 * 1024 + 1), false,
        "E_COMPOSE_BOUND: JSON original exceeds bound");
    composition_result(admit_json_original(u64::MAX), false,
        "E_COMPOSE_BOUND: JSON original exceeds bound");
    composition_result(admit_manifest("chrono-input-composition/v1", 1), true, "");
    composition_result(admit_manifest("chrono-input-composition/v1", 4096), true, "");
    composition_result(admit_manifest("chrono-input-composition/v1", 0), false,
        "E_COMPOSE_MANIFEST: schema/count");
    composition_result(admit_manifest("chrono-input-composition/v1", 4097), false,
        "E_COMPOSE_MANIFEST: schema/count");
    composition_result(admit_manifest("chrono-input-composition/v1", usize::MAX), false,
        "E_COMPOSE_MANIFEST: schema/count");
    composition_result(admit_manifest("chrono-input-composition/v1\0", 1), false,
        "E_COMPOSE_MANIFEST: schema/count");
    composition_result(admit_output_paths("", ""), false, "E_COMPOSE_PATH: output/receipt collision");
    composition_result(admit_output_paths("原件\0\n", "原件\0\n"), false,
        "E_COMPOSE_PATH: output/receipt collision");
    composition_result(admit_output_paths("原件\0\n", "原件\0\n "), true, "");
    composition_result(admit_pair_digest("原件\0\n", "原件\0\n"), true, "");
    composition_result(admit_pair_digest("原件\0\n ", "原件\0\n"), false,
        "E_COMPOSE_ORIGINAL: pair digest differs");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_requires_exact_endpoints_and_both_pair_fields() {
    use policy::composition::*;
    proof { reveal_strlit("base"); reveal_strlit("candidate"); reveal_strlit("base\0");
        assert("base\0"@.len() != "base"@.len());
        assert("base\0"@.len() != "candidate"@.len()); }
    composition_result(admit_single_endpoint(Some("base"), "base", "candidate"), true, "");
    composition_result(admit_single_endpoint(Some("candidate"), "base", "candidate"), true, "");
    composition_result(admit_single_endpoint(None, "base", "candidate"), false,
        "E_COMPOSE_HEADER: snapshot outside fixed endpoints");
    composition_result(admit_single_endpoint(Some("base\0"), "base", "candidate"), false,
        "E_COMPOSE_HEADER: snapshot outside fixed endpoints");
    composition_result(admit_single_endpoint(Some(""), "", ""), true, "");
    composition_result(admit_pair_fields(Some(2), true, true), true, "");
    composition_result(admit_pair_fields(None, true, true), false, "E_COMPOSE_PAIR: exact endpoints required");
    composition_result(admit_pair_fields(Some(1), true, true), false, "E_COMPOSE_PAIR: exact endpoints required");
    composition_result(admit_pair_fields(Some(3), true, true), false, "E_COMPOSE_PAIR: exact endpoints required");
    composition_result(admit_pair_fields(Some(2), false, true), false, "E_COMPOSE_PAIR: exact endpoints required");
    composition_result(admit_pair_fields(Some(2), true, false), false, "E_COMPOSE_PAIR: exact endpoints required");
    composition_result(admit_pair_fields(Some(2), false, false), false, "E_COMPOSE_PAIR: exact endpoints required");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_header_binds_every_identity_and_selector_observation() {
    use policy::composition::*;
    proof { reveal_strlit("完整\0\n"); reveal_strlit("完整\0\n ");
        assert("完整\0\n"@.len() != "完整\0\n "@.len()); }
    composition_result(admit_snapshot_identity(Some("完整\0\n"), Some("完整\0\n"), Some("完整\0\n"),
        "完整\0\n", "完整\0\n", "完整\0\n"), true, "");
    composition_result(admit_snapshot_identity(None, Some("完整\0\n"), Some("完整\0\n"),
        "完整\0\n", "完整\0\n", "完整\0\n"), false, "configuration/endpoint/selector mismatch");
    composition_result(admit_snapshot_identity(Some("完整\0\n"), None, Some("完整\0\n"),
        "完整\0\n", "完整\0\n", "完整\0\n"), false, "configuration/endpoint/selector mismatch");
    composition_result(admit_snapshot_identity(Some("完整\0\n"), Some("完整\0\n"), None,
        "完整\0\n", "完整\0\n", "完整\0\n"), false, "configuration/endpoint/selector mismatch");
    composition_result(admit_snapshot_identity(Some("完整\0\n "), Some("完整\0\n"), Some("完整\0\n"),
        "完整\0\n", "完整\0\n", "完整\0\n"), false, "configuration/endpoint/selector mismatch");
    composition_result(admit_snapshot_identity(Some("完整\0\n"), Some("完整\0\n "), Some("完整\0\n"),
        "完整\0\n", "完整\0\n", "完整\0\n"), false, "configuration/endpoint/selector mismatch");
    composition_result(admit_snapshot_identity(Some("完整\0\n"), Some("完整\0\n"), Some("完整\0\n "),
        "完整\0\n", "完整\0\n", "完整\0\n"), false, "configuration/endpoint/selector mismatch");
    let mut header = SnapshotHeader { digest: Some("完整\0\n"), selection_present: false,
        effective_present: false, effective: None };
    composition_result(admit_snapshot_header(&header, "完整\0\n", false, false, ""), true, "");
    header.selection_present = true;
    composition_result(admit_snapshot_header(&header, "完整\0\n", false, true, ""), false,
        "configuration/endpoint/selector mismatch");
    header.selection_present = false; header.effective_present = true;
    composition_result(admit_snapshot_header(&header, "完整\0\n", false, true, ""), false,
        "configuration/endpoint/selector mismatch");
    header.selection_present = true; header.effective = Some("完整\0\n");
    composition_result(admit_snapshot_header(&header, "完整\0\n", true, true, "完整\0\n"), true, "");
    composition_result(admit_snapshot_header(&header, "完整\0\n", true, false, "完整\0\n"), false,
        "configuration/endpoint/selector mismatch");
    header.effective = Some("完整\0\n ");
    composition_result(admit_snapshot_header(&header, "完整\0\n", true, true, "完整\0\n"), false,
        "configuration/endpoint/selector mismatch");
    header.effective = None;
    composition_result(admit_snapshot_header(&header, "完整\0\n", true, true, "完整\0\n"), false,
        "configuration/endpoint/selector mismatch");
    header.effective = Some("完整\0\n"); header.digest = None;
    composition_result(admit_snapshot_header(&header, "完整\0\n", true, true, "完整\0\n"), false,
        "configuration/endpoint/selector mismatch");
    header.digest = Some("完整\0\n ");
    composition_result(admit_snapshot_header(&header, "完整\0\n", true, true, "完整\0\n"), false,
        "configuration/endpoint/selector mismatch");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_blob_identity_and_absence_keep_original_errors() {
    use policy::composition::*;
    proof { reveal_strlit("摘要\0"); reveal_strlit("摘要\0 "); reveal_strlit("absent"); reveal_strlit("Absent");
        assert("摘要\0"@.len() != "摘要\0 "@.len());
        assert("absent"@[0] != "Absent"@[0]); }
    composition_result(admit_imported_blob(Some("摘要\0"), Some(u64::MAX), "摘要\0", u64::MAX), true, "");
    composition_result(admit_imported_blob(Some("摘要\0"), Some(0), "摘要\0", 0), true, "");
    composition_result(admit_imported_blob(Some("摘要\0 "), Some(0), "摘要\0", 0), false,
        "E_COMPOSE_BLOB: original identity differs");
    composition_result(admit_imported_blob(Some("摘要\0"), Some(1), "摘要\0", 0), false,
        "E_COMPOSE_BLOB: original identity differs");
    composition_result(admit_imported_blob(None, Some(0), "摘要\0", 0), false,
        "E_COMPOSE_BLOB: original identity differs");
    composition_result(admit_imported_blob(Some("摘要\0"), None, "摘要\0", 0), false,
        "E_COMPOSE_BLOB: original identity differs");
    composition_result(admit_imported_input(true, Some("absent"), false), true, "");
    composition_result(admit_imported_input(true, Some("absent"), true), true, "");
    composition_result(admit_imported_input(true, None, true), false, "unexpected absence");
    composition_result(admit_imported_input(true, Some("Absent"), true), false, "unexpected absence");
    composition_result(admit_imported_input(false, Some("absent"), true), false, "digest/presence mismatch");
    composition_result(admit_imported_input(false, Some("absent"), false), false, "digest/presence mismatch");
    composition_result(admit_imported_input(false, None, false), false, "digest/presence mismatch");
    composition_result(admit_imported_input(false, None, true), true, "");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn blob_identity_requires_both_exact_digest_and_length() {
    proof {
        reveal_strlit("hash"); reveal_strlit("Hash"); reveal_strlit("");
        assert("hash"@[0] != "Hash"@[0]);
        assert("hash"@.len() != ""@.len());
    }
    let same = policy::blob_identity_matches("hash", 42, "hash", 42);
    let content = policy::blob_identity_matches("Hash", 42, "hash", 42);
    let size = policy::blob_identity_matches("hash", 41, "hash", 42);
    let absent = policy::blob_identity_matches("", 42, "hash", 42);
    let empty = policy::blob_identity_matches("hash", 0, "hash", 0);
    let largest = policy::blob_identity_matches("hash", u64::MAX, "hash", u64::MAX);
    assert(same && !content && !size && !absent && empty && largest);
    if !(same && !content && !size && !absent && empty && largest) { panic!(); }
}
}

#[path = "../../runner-tests/tests/support/isolated_capability.rs"]
#[cfg(not(verus_keep_ghost))]
mod chrono_git_boundary;
#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
#[cfg(not(verus_keep_ghost))]
mod support;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{json, sha256};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json as value};
#[cfg(not(verus_keep_ghost))]
use std::{fs, path::Path, process::Command};
#[cfg(not(verus_keep_ghost))]
use support::*;
#[cfg(not(verus_keep_ghost))]
fn invoke(root: &Path, args: &[&str]) -> (i32, Value, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_chrono-inputs-test-cli"));
    cmd.current_dir("/")
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .env("DECLARED_EMPTY", "")
        .env("SHOULD_NOT_LEAK", "undeclared");
    cmd.args(args).args(["--host-root", root.to_str().unwrap()]);
    let out = cmd.output().unwrap();
    (
        out.status.code().unwrap(),
        json(&out.stdout).unwrap_or(Value::Null),
        String::from_utf8_lossy(&out.stderr).into(),
    )
}
#[cfg(not(verus_keep_ghost))]
fn fixture() -> (tempfile::TempDir, tempfile::NamedTempFile, Values, String) {
    let d = tempfile::Builder::new()
        .prefix("retained input host ")
        .tempdir()
        .unwrap();
    git(d.path(), &["init", "-q"]);
    fs::write(d.path().join(".gitignore"), ".chrono-harness/state/\n").unwrap();
    let f = tempfile::NamedTempFile::new().unwrap();
    fs::write(f.path(), b"original\0binary\xff").unwrap();
    let mut v = values();
    v.get_mut(CONFIG).unwrap()["environment"] = value!({"inherit":["DECLARED_EMPTY","DECLARED_ABSENT"],"values":{"FIXED":"fixed"},"inputs":[{"id":"data","location":f.path(),"sha256":sha256(b"original\0binary\xff")},{"id":"same-bytes","location":f.path(),"sha256":sha256(b"original\0binary\xff")}]});
    write_values(d.path(), &v);
    let commit = commit(d.path());
    (d, f, v, commit)
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn captured_snapshot_and_shared_blobs_enter_the_declared_retention_owner() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
    let (host, _input, _values, commit) = fixture();
    fs::write(
        host.path().join(POLICY_PATH),
        serde_json::to_vec(&value!({
            "schema":"chrono-output-retention/v2",
            "producers":{"full-original":[".chrono-harness/state/"]},
            "keep_seconds":0,"metadata_keep_seconds":0
        }))
        .unwrap(),
    )
    .unwrap();
    let path = ".chrono-harness/state/retained-snapshot.json";
    let snapshot = chrono_inputs::capture(host.path(), CONFIG, &commit, path).unwrap();
    let owner = Inventory::adopted(host.path()).unwrap().unwrap();
    assert!(
        owner.enrolled(path).unwrap(),
        "published snapshot is outside the maintenance inventory"
    );
    let blob = snapshot["files"]["data"]["blob"].as_str().unwrap();
    assert_eq!(snapshot["files"]["data"], snapshot["files"]["same-bytes"]);
    assert!(
        owner.enrolled(blob).unwrap(),
        "shared blob is outside the maintenance inventory"
    );
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn capture_rejects_retired_endpoint_options_before_accessing_host_inputs() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for options in [
        vec!["--base", "base"],
        vec!["--candidate", "candidate"],
        vec!["--base", "base", "--candidate", "candidate"],
    ] {
        let args: Vec<String> = std::iter::once("capture")
            .chain(options)
            .map(str::to_owned)
            .collect();
        assert_eq!(
            chrono_inputs::run(Path::new("."), &args).unwrap_err(),
            "E_USAGE: unknown or duplicate option"
        );
    }
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn schema4_snapshots_exclude_acquisition_credentials_and_legacy_readers_stay_strict() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, _f, mut v, _) = fixture();
    let path_dir = tempfile::tempdir().unwrap();
    let ambient_git = chrono_harness::resolve_program(d.path(), "git", None).unwrap();
    let git_link = path_dir.path().join("git");
    std::os::unix::fs::symlink(&ambient_git, &git_link).unwrap();
    let path = format!(
        "{}:{}",
        path_dir.path().display(),
        std::env::var("PATH").unwrap()
    );
    let git =
        fs::canonicalize(chrono_harness::resolve_program(d.path(), "git", Some(&path)).unwrap())
            .unwrap();
    assert!(fs::metadata(&git).unwrap().is_file());
    let version = Command::new(&git).arg("--version").output().unwrap();
    let c = v.get_mut(CONFIG).unwrap();
    c["schema_version"] = value!(4);
    c["canonical_check"] = value!({"operation":"validate.delta","argv":[".chrono-harness/bin/chrono-harness","check"],"profile":CONFIG,"inputs":{"local":{"operation":"fixture.input","tool":"git","argv":["--version"]}}});
    c["facts_git"] = value!({"tool":"git","input":"git-bytes"});
    c["tools"].as_array_mut().unwrap().push(value!({"id":"git","program":git,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}));
    for i in c["environment"]["inputs"].as_array_mut().unwrap() {
        i["presence"] = value!("present");
        i["location"] = value!(fs::canonicalize(i["location"].as_str().unwrap()).unwrap());
    }
    c["environment"]["inputs"].as_array_mut().unwrap().push(value!({"id":"git-bytes","location":git,"presence":"present","sha256":sha256(&fs::read(&git).unwrap())}));
    c["environment"]["inherit"]
        .as_array_mut()
        .unwrap()
        .extend([value!("CHRONO_CHECK_SOURCE"), value!("GH_TOKEN")]);
    c["environment"]["credential_environment"] = value!(["GH_TOKEN"]);
    c["input_closure"] = value!({"status":"incomplete","unresolved":["fixture"]});
    write_values(d.path(), &v);
    let oid = commit(d.path());
    let out = Command::new(env!("CARGO_BIN_EXE_chrono-inputs-test-cli"))
        .env("GH_TOKEN", "fixture-secret-not-a-business-input")
        .env_remove("CHRONO_CHECK_SOURCE")
        .args([
            "capture",
            "--host-root",
            d.path().to_str().unwrap(),
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/credential-snapshot.json",
        ])
        .env("PATH", &path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!String::from_utf8_lossy(&out.stdout).contains("fixture-secret-not-a-business-input"));
    let snapshot = json(&out.stdout).unwrap();
    assert_eq!(snapshot["environment"]["GH_TOKEN"], Value::Null);
    chrono_judge_registration::inputs::environment(&v[CONFIG], &snapshot["environment"]).unwrap();
    let mut wrong = snapshot["environment"].clone();
    wrong["GH_TOKEN"] = value!("retained-by-mistake");
    assert!(
        chrono_judge_registration::inputs::environment(&v[CONFIG], &wrong)
            .unwrap_err()
            .contains("acquisition credentials")
    );
    let mut old = values();
    old.get_mut(CONFIG).unwrap()["environment"]["credential_environment"] = value!([]);
    assert!(
        chrono_judge_registration::Registrations::load(&old, CONFIG)
            .err()
            .unwrap()
            .contains("unknown field credential_environment")
    );
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn captures_only_declared_bytes_and_environment_and_deduplicates_content() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, f, _, oid) = fixture();
    let args = [
        "capture",
        "--config",
        CONFIG,
        "--commit",
        &oid,
        "--output",
        ".chrono-harness/state/one.json",
    ];
    let (code, s, err) = invoke(d.path(), &args);
    assert_eq!(code, 0, "{err}");
    assert_eq!(s["schema"], "chrono-input-snapshot/v1");
    assert_eq!(s["commit"], oid);
    assert_eq!(
        s["environment"],
        value!({"DECLARED_EMPTY":"","DECLARED_ABSENT":null})
    );
    assert_eq!(s["files"]["data"], s["files"]["same-bytes"]);
    assert!(s["files"]["data"].get("bytes").is_none());
    let blob = s["files"]["data"]["blob"].as_str().unwrap();
    assert_eq!(
        fs::read(d.path().join(blob)).unwrap(),
        fs::read(f.path()).unwrap()
    );
    let (code, again, err) = invoke(d.path(), &args);
    assert_eq!(code, 0, "{err}");
    assert_eq!(s, again);
    assert_eq!(
        fs::read_dir(d.path().join(".chrono-harness/state/inputs/blobs"))
            .unwrap()
            .count(),
        1
    );
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn captures_observed_mismatch_without_rewriting_registration_and_preserves_old_blob() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, f, _, oid) = fixture();
    let cfg = fs::read(d.path().join(CONFIG)).unwrap();
    let (code, old, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    fs::write(f.path(), b"changed").unwrap();
    let (code, new, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/two.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert_ne!(
        old["files"]["data"]["sha256"],
        new["files"]["data"]["sha256"]
    );
    assert_eq!(
        fs::read(
            d.path()
                .join(old["files"]["data"]["blob"].as_str().unwrap())
        )
        .unwrap(),
        b"original\0binary\xff"
    );
    assert_eq!(fs::read(d.path().join(CONFIG)).unwrap(), cfg);
    let (code, _, _) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_ne!(code, 0);
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn pairs_explicit_snapshots_without_recapturing_the_old_environment_or_file() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, f, _, oid) = fixture();
    let (code, a, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    fs::write(f.path(), b"new bytes").unwrap();
    let newer = commit(d.path());
    let (code, b, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &newer,
            "--output",
            ".chrono-harness/state/two.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    let (code, p, err) = invoke(
        d.path(),
        &[
            "pair",
            "--base-snapshot",
            ".chrono-harness/state/one.json",
            "--candidate-snapshot",
            ".chrono-harness/state/two.json",
            "--output",
            ".chrono-harness/state/pair.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(p, value!({"base":a,"candidate":b}));
}

#[cfg(not(verus_keep_ghost))]
fn enroll_snapshot_sources(
    root: &Path,
    snapshot: &Value,
) -> (
    chrono_harness::retained_artifacts::retention::Inventory,
    Vec<String>,
) {
    use chrono_harness::retained_artifacts::retention::Inventory;
    fs::write(
        root.join(".chrono-harness/retention.json"),
        serde_json::to_vec(&value!({
            "schema":"chrono-output-retention/v2",
            "producers":{"fixture":[".chrono-harness/state/"]},
            "keep_seconds":0,"metadata_keep_seconds":0,
            "max_related_outputs":32,"max_summaries":16,"max_nodes_per_round":256,
            "max_bytes_per_round":1048576,"max_millis_per_round":1000
        }))
        .unwrap(),
    )
    .unwrap();
    let owner = Inventory::adopted(root).unwrap().unwrap();
    let mut ids = Vec::new();
    for path in [
        ".chrono-harness/state/one.json",
        snapshot["files"]["data"]["blob"].as_str().unwrap(),
    ] {
        let publication = Inventory::adopted(root)
            .unwrap()
            .unwrap()
            .begin("fixture", path, value!({"kind":"explicit-test-adoption"}))
            .unwrap();
        ids.push(publication.id().to_owned());
        publication.complete(value!({"exit_code":0})).unwrap();
    }
    (owner, ids)
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn pairing_keeps_enrolled_sources_until_the_callers_handoff() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::read_scope;
    let (mut source, _input, _, oid) = fixture();
    source.disable_cleanup(true);
    eprintln!(
        "Original snapshot handoff fixture: {}",
        source.path().display()
    );
    let snapshot_path = ".chrono-harness/state/one.json";
    let (code, snapshot, error) = invoke(
        source.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            snapshot_path,
        ],
    );
    assert_eq!(code, 0, "{error}");
    let (owner, ids) = enroll_snapshot_sources(source.path(), &snapshot);
    let original = fs::read(source.path().join(snapshot_path)).unwrap();
    let blob = snapshot["files"]["data"]["blob"].as_str().unwrap();
    let bytes = fs::read(source.path().join(blob)).unwrap();
    let destination = tempfile::tempdir().unwrap();
    let caller = read_scope(source.path()).unwrap();
    let paired = chrono_inputs::pair(
        destination.path(),
        source.path(),
        snapshot_path,
        source.path(),
        snapshot_path,
        ".chrono-harness/state/pair.json",
    )
    .unwrap();
    assert_eq!(paired, value!({"base":snapshot,"candidate":snapshot}));
    let active = owner.maintain_originals_at(&ids, u64::MAX).unwrap();
    assert_eq!(
        fs::read(source.path().join(snapshot_path)).unwrap_or_else(|e| panic!("{e}; {active}")),
        original,
        "{active}"
    );
    assert_eq!(
        fs::read(source.path().join(blob)).unwrap_or_else(|e| panic!("{e}; {active}")),
        bytes,
        "{active}"
    );
    assert_eq!(fs::read(destination.path().join(blob)).unwrap(), bytes);
    drop(caller);
    let retired = owner.maintain_originals_at(&ids, u64::MAX).unwrap();
    assert!(!source.path().join(snapshot_path).exists(), "{retired}");
    assert!(!source.path().join(blob).exists(), "{retired}");
    source.disable_cleanup(false);
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn registration_keeps_an_enrolled_blob_until_the_callers_handoff() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::{retained_artifacts::retention::read_scope, wire};
    use chrono_judge_registration::{Registrations, inputs};
    let (mut source, _input, values, oid) = fixture();
    source.disable_cleanup(true);
    eprintln!("Original input reader fixture: {}", source.path().display());
    let (code, snapshot, error) = invoke(
        source.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_eq!(code, 0, "{error}");
    let root = fs::canonicalize(source.path()).unwrap();
    let request: wire::Request = serde_json::from_value(value!({
        "protocol":"chrono-judge/v1","request_id":"","judge_id":"registration","mode":"evaluate",
        "base":{"commit":oid,"tree":"c".repeat(40),"root":root},
        "candidate":{"commit":oid,"tree":"d".repeat(40),"root":root},
        "delta":[],"registries":{"base":root,"candidate":root,"digest":"e".repeat(64)},
        "context":{"path":root.join("context.json"),"sha256":"f".repeat(64)},
        "impact":null,"prior_results":[],"config_path":CONFIG,
        "checkout":{"head":oid,"tracked":[],"untracked":[],"index_flags":[]},
        "runner":{"path":"runner","sha256":"e".repeat(64),"version":"0.1.0"},
        "observations":{"retained":{"base":snapshot,"candidate":snapshot},
            "environment":{"inherited":snapshot["environment"],
                "effective":inputs::environment(&values[CONFIG], &snapshot["environment"]).unwrap()}}
    })).unwrap();
    let registrations = Registrations::load(&values, CONFIG).unwrap();
    let (owner, ids) = enroll_snapshot_sources(&root, &snapshot);
    let blob = snapshot["files"]["data"]["blob"].as_str().unwrap();
    let bytes = fs::read(root.join(blob)).unwrap();
    let caller = read_scope(&root).unwrap();
    let effective = inputs::validate(&request, &registrations, &registrations).unwrap();
    assert_eq!(
        effective["endpoints"]["candidate"]["files"]["data"]["sha256"],
        sha256(&bytes)
    );
    let active = owner.maintain_originals_at(&ids, u64::MAX).unwrap();
    assert_eq!(
        fs::read(root.join(blob)).unwrap_or_else(|e| panic!("{e}; {active}")),
        bytes,
        "{active}"
    );
    // The already decoded snapshot was not opened by this consumer.
    assert!(
        !root.join(".chrono-harness/state/one.json").exists(),
        "{active}"
    );
    drop(caller);
    let retired = owner.maintain_originals_at(&ids[1..], u64::MAX).unwrap();
    assert!(!root.join(blob).exists(), "{retired}");
    source.disable_cleanup(false);
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn wrong_checkout_dirty_config_missing_input_and_unsafe_output_never_publish_snapshot() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, f, _, oid) = fixture();
    for (commit, path) in [
        ("HEAD", ".chrono-harness/state/one.json"),
        (&oid, "outside.json"),
        (&oid, ".chrono-harness/state/"),
        (&oid, ".chrono-harness/state/../bad.json"),
    ] {
        let (code, _, _) = invoke(
            d.path(),
            &[
                "capture", "--config", CONFIG, "--commit", commit, "--output", path,
            ],
        );
        assert_ne!(code, 0);
    }
    let original = fs::read(d.path().join(CONFIG)).unwrap();
    fs::write(d.path().join(CONFIG), b"{}").unwrap();
    assert_ne!(
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/one.json"
            ]
        )
        .0,
        0
    );
    fs::write(d.path().join(CONFIG), original).unwrap();
    commit(d.path());
    assert_ne!(
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/wrong-checkout.json"
            ]
        )
        .0,
        0
    );
    git(d.path(), &["checkout", "--detach", &oid]);
    fs::remove_file(f.path()).unwrap();
    assert_ne!(
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/one.json"
            ]
        )
        .0,
        0
    );
    assert!(!d.path().join(".chrono-harness/state/one.json").exists());
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn transport_to_a_new_host_uses_retained_bytes_after_original_input_disappears() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, f, _, oid) = fixture();
    let (code, snapshot, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    fs::remove_file(f.path()).unwrap();
    let dest = tempfile::Builder::new()
        .prefix("new snapshot host ")
        .tempdir()
        .unwrap();
    let args = [
        "pair",
        "--base-root",
        d.path().to_str().unwrap(),
        "--candidate-root",
        d.path().to_str().unwrap(),
        "--base-snapshot",
        ".chrono-harness/state/one.json",
        "--candidate-snapshot",
        ".chrono-harness/state/one.json",
        "--output",
        ".chrono-harness/state/pair.json",
    ];
    let (code, pair, err) = invoke(dest.path(), &args);
    assert_eq!(code, 0, "{err}");
    assert_eq!(pair["base"], snapshot);
    assert_eq!(pair["candidate"], snapshot);
    let blob = snapshot["files"]["data"]["blob"].as_str().unwrap();
    assert_eq!(
        fs::read(dest.path().join(blob)).unwrap(),
        b"original\0binary\xff"
    );
    fs::write(d.path().join(blob), b"corrupt retained content").unwrap();
    let (code, _, err) = invoke(dest.path(), &args);
    assert_ne!(code, 0);
    assert!(err.contains("E_INPUT_BLOB"), "{err}");
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn corrupt_existing_content_address_and_symlink_output_are_rejected() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, _file, _, oid) = fixture();
    let args = [
        "capture",
        "--config",
        CONFIG,
        "--commit",
        &oid,
        "--output",
        ".chrono-harness/state/one.json",
    ];
    let (code, s, err) = invoke(d.path(), &args);
    assert_eq!(code, 0, "{err}");
    fs::write(
        d.path().join(s["files"]["data"]["blob"].as_str().unwrap()),
        b"wrong",
    )
    .unwrap();
    let (code, _, err) = invoke(d.path(), &args);
    assert_ne!(code, 0);
    assert!(err.contains("E_INPUT_BLOB"), "{err}");
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), d.path().join(".chrono-harness/state/alias"))
            .unwrap();
        let (code, _, err) = invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/alias/output.json",
            ],
        );
        assert_ne!(code, 0);
        assert!(err.contains("symlink"), "{err}");
        assert!(!outside.path().join("output.json").exists());
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn pairing_does_not_silently_normalize_conflicting_or_incomplete_snapshot_fields() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, _file, _, oid) = fixture();
    let (code, snapshot, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    let mut cases = Vec::new();
    let mut bad = snapshot.clone();
    bad["files"]["data"]["bytes"] = value!([1, 2]);
    cases.push(bad);
    bad = snapshot.clone();
    bad.as_object_mut().unwrap().remove("config_digest");
    cases.push(bad);
    bad = snapshot.clone();
    bad["environment"] = value!([]);
    cases.push(bad);
    bad = snapshot.clone();
    bad["environment"]["DECLARED_EMPTY"] = value!(false);
    cases.push(bad);
    bad = snapshot.clone();
    bad["files"]["data"]["length"] = value!(-1);
    cases.push(bad);
    bad = snapshot.clone();
    bad["files"]["data"]["sha256"] = value!("bad digest");
    cases.push(bad);
    bad = snapshot.clone();
    bad["files"]["data"]["blob"] = value!(".chrono-harness/state/../outside");
    cases.push(bad);
    bad = snapshot.clone();
    bad["config_path"] = value!("../config.json");
    cases.push(bad);
    bad = snapshot.clone();
    bad["unknown"] = value!(true);
    cases.push(bad);
    for data in cases {
        fs::write(
            d.path().join(".chrono-harness/state/bad.json"),
            serde_json::to_vec(&data).unwrap(),
        )
        .unwrap();
        let (code, _, err) = invoke(
            d.path(),
            &[
                "pair",
                "--base-snapshot",
                ".chrono-harness/state/bad.json",
                "--candidate-snapshot",
                ".chrono-harness/state/one.json",
                "--output",
                ".chrono-harness/state/must-not-publish.json",
            ],
        );
        assert_ne!(code, 0, "{err}");
        assert!(
            !d.path()
                .join(".chrono-harness/state/must-not-publish.json")
                .exists()
        );
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn explicit_absence_is_retained_transported_and_distinct_from_empty_bytes() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, f, mut v, _) = fixture();
    v.get_mut(CONFIG).unwrap()["schema_version"] = value!(2);
    for row in v.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        row["location"] = value!(fs::canonicalize(f.path()).unwrap());
        row["presence"] = value!("absent");
        row.as_object_mut().unwrap().remove("sha256");
    }
    write_values(d.path(), &v);
    let oid = commit(d.path());
    fs::remove_file(f.path()).unwrap();
    let capture = |out| {
        invoke(
            d.path(),
            &[
                "capture", "--config", CONFIG, "--commit", &oid, "--output", out,
            ],
        )
    };
    let (code, absent, err) = capture(".chrono-harness/state/absent.json");
    assert_eq!(code, 0, "{err}");
    assert_eq!(absent["schema"], "chrono-input-snapshot/v2");
    assert_eq!(absent["files"]["data"], value!({"absent":true}));
    fs::write(f.path(), []).unwrap();
    let (code, empty, err) = capture(".chrono-harness/state/empty.json");
    assert_eq!(
        code, 0,
        "capture observes even a declaration mismatch: {err}"
    );
    assert_eq!(empty["files"]["data"]["length"], 0);
    assert_eq!(empty["files"]["data"]["sha256"], sha256(b""));
    let target = tempfile::tempdir().unwrap();
    let (code, pair, err) = invoke(
        target.path(),
        &[
            "pair",
            "--base-root",
            d.path().to_str().unwrap(),
            "--candidate-root",
            d.path().to_str().unwrap(),
            "--base-snapshot",
            ".chrono-harness/state/absent.json",
            "--candidate-snapshot",
            ".chrono-harness/state/empty.json",
            "--output",
            ".chrono-harness/state/pair.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(pair["base"], absent);
    assert_eq!(pair["candidate"], empty);
    assert_eq!(
        fs::read_dir(target.path().join(".chrono-harness/state/inputs/blobs"))
            .unwrap()
            .count(),
        1
    );
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn v2_absence_rejects_symlink_directory_and_non_directory_ancestors() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, f, mut v, _) = fixture();
    let external = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(external.path()).unwrap();
    let bad = root.join("bad");
    v.get_mut(CONFIG).unwrap()["schema_version"] = value!(2);
    v.get_mut(CONFIG).unwrap()["environment"]["inputs"] =
        value!([{"id":"data","location":bad,"presence":"absent"}]);
    write_values(d.path(), &v);
    let oid = commit(d.path());
    let capture = || {
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/bad.json",
            ],
        )
    };
    fs::create_dir(&bad).unwrap();
    assert_ne!(capture().0, 0, "directory is not absent");
    fs::remove_dir(&bad).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("missing"), &bad).unwrap();
        assert_ne!(capture().0, 0, "dangling link is not absent");
        fs::remove_file(&bad).unwrap();
        std::os::unix::fs::symlink(f.path(), &bad).unwrap();
        assert_ne!(capture().0, 0, "live link is not a v2 input");
        fs::remove_file(&bad).unwrap();
    }
    fs::write(&bad, []).unwrap();
    v.get_mut(CONFIG).unwrap()["environment"]["inputs"][0]["location"] = value!(bad.join("child"));
    write_values(d.path(), &v);
    let oid = commit(d.path());
    assert_ne!(
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/non-directory.json"
            ]
        )
        .0,
        0
    );
    fs::remove_file(&bad).unwrap();
    let (code, observed, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/missing-parent.json",
        ],
    );
    assert_eq!(code, 0, "missing directory gives explicit absence: {err}");
    assert_eq!(observed["files"]["data"], value!({"absent":true}));
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn absence_versioning_rejects_ambiguous_declarations_and_legacy_null() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_judge_registration::{Registrations, inputs::snapshot_shape};
    let (d, f, mut v, _) = fixture();
    for row in v.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        row["sha256"] = Value::Null;
    }
    assert!(
        Registrations::load(&v, CONFIG).is_ok(),
        "v1 null remains an unbound draft"
    );
    write_values(d.path(), &v);
    let oid = commit(d.path());
    fs::remove_file(f.path()).unwrap();
    assert_ne!(
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/legacy.json"
            ]
        )
        .0,
        0
    );
    for row in [
        value!({"id":"data","location":"external","presence":"absent","sha256":null}),
        value!({"id":"data","location":"external","presence":"present"}),
        value!({"id":"data","location":"external","sha256":null}),
        value!({"id":"data","location":"external","presence":"unknown"}),
    ] {
        v.get_mut(CONFIG).unwrap()["schema_version"] = value!(2);
        v.get_mut(CONFIG).unwrap()["environment"]["inputs"] = value!([row]);
        assert!(Registrations::load(&v, CONFIG).is_err());
    }
    v.get_mut(CONFIG).unwrap()["schema_version"] = value!(1);
    v.get_mut(CONFIG).unwrap()["environment"]["inputs"] =
        value!([{"id":"data","location":"external","presence":"absent"}]);
    assert!(Registrations::load(&v, CONFIG).is_err());
    let mut empty = values();
    empty.get_mut(CONFIG).unwrap()["schema_version"] = value!(2);
    let r = Registrations::load(&empty, CONFIG).unwrap();
    assert!(
        chrono_judge_registration::inputs::effective_environments(&Value::Null, &r, &r).is_err()
    );
    let mut s = value!({"schema":"chrono-input-snapshot/v1","commit":oid,"config_path":CONFIG,"config_digest":"a".repeat(64),"environment":{},"files":{"data":{"absent":true}}});
    assert!(snapshot_shape(&s).is_err());
    s["schema"] = value!("chrono-input-snapshot/v2");
    assert!(snapshot_shape(&s).is_ok());
    s["files"]["data"]["absent"] = value!(false);
    assert!(snapshot_shape(&s).is_err());
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn v3_capture_uses_bound_git_and_keeps_v2_snapshot_semantics() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let (dir, _data, mut values, _) = fixture();
    let root = fs::canonicalize(dir.path()).unwrap();
    let state = root.join(".chrono-harness/state");
    fs::create_dir_all(&state).unwrap();
    let program = state.join("snapshot Git λ");
    let trace = state.join("git-trace");
    let real =
        fs::canonicalize(chrono_harness::resolve_program(&root, "git", None).unwrap()).unwrap();
    let version = Command::new(&real).arg("--version").output().unwrap();
    assert!(version.status.success());
    fs::copy(env!("CARGO_BIN_EXE_chrono-inputs-test-git"), &program).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let config = values.get_mut(CONFIG).unwrap();
    config["schema_version"] = value!(3);
    config["facts_git"] = value!({"tool":"chosen-git","input":"git-input"});
    config["tools"].as_array_mut().unwrap().push(value!({"id":"chosen-git","program":program,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim_end()}));
    config["environment"]["values"]["SNAPSHOT_GIT"] = value!("declared");
    config["environment"]["values"]["SNAPSHOT_TRACE"] = value!(trace);
    for input in config["environment"]["inputs"].as_array_mut().unwrap() {
        input["presence"] = value!("present");
        input["location"] = value!(fs::canonicalize(input["location"].as_str().unwrap()).unwrap());
    }
    config["environment"]["inputs"].as_array_mut().unwrap().push(value!({"id":"git-input","location":program,"presence":"present","sha256":sha256(&fs::read(&program).unwrap())}));
    config["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .push(value!({"id":"absent","location":state.join("absent-input"),"presence":"absent"}));
    // A v3 declared-complete config must carry the explicit external-input
    // inventory. Capture consumes the registration snapshot contract directly,
    // so keep every tool, retained file and environment value visible here.
    config["input_closure"] = value!({
        "status": "declared-complete",
        "unresolved": [],
        "bindings": [{
            "id": "snapshot-capture-inputs",
            "consumer": "judge:registration",
            "kind": "snapshot-capture",
            "inputs": [
                "tool:sh",
                "tool:chosen-git",
                "input:data",
                "input:same-bytes",
                "input:git-input",
                "input:absent",
                "environment:DECLARED_EMPTY",
                "environment:DECLARED_ABSENT",
                "environment:FIXED",
                "environment:SNAPSHOT_GIT",
                "environment:SNAPSHOT_TRACE"
            ]
        }]
    });
    let edges = values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap();
    for input in [
        "tool:sh",
        "tool:chosen-git",
        "input:data",
        "input:same-bytes",
        "input:git-input",
        "input:absent",
        "environment:DECLARED_EMPTY",
        "environment:DECLARED_ABSENT",
        "environment:FIXED",
        "environment:SNAPSHOT_GIT",
        "environment:SNAPSHOT_TRACE",
    ] {
        edges.push(value!({
            "from": input,
            "kind": "runtime-input",
            "to": "judge:registration"
        }));
    }
    write_values(&root, &values);
    let oid = commit(&root);
    let (exit, snapshot, error) = invoke(
        &root,
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/v3.json",
        ],
    );
    assert_eq!(exit, 0, "{error}");
    assert_eq!(snapshot["schema"], "chrono-input-snapshot/v2");
    assert_eq!(snapshot["files"]["absent"], value!({"absent":true}));
    assert_eq!(
        snapshot["config_digest"],
        chrono_harness::wire::digest(&values[CONFIG]).unwrap()
    );
    assert_eq!(
        snapshot["files"]["git-input"]["sha256"],
        sha256(&fs::read(&program).unwrap())
    );
    assert!(fs::read_to_string(trace).unwrap().contains("rev-parse"));

    // The same bound v3 policy can be reached through the native selector
    // entry. Capture publishes a versioned selection binding while retaining
    // the caller's entry path for later registration validation.
    let selector = ".chrono-harness/platforms.json";
    fs::write(
        root.join(selector),
        serde_json::to_vec(&value!({
            "schema": "chrono-git-configs/v3",
            "platforms": {format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH): {
                "program": program,
                "configurations": [{
                    "sha256": sha256(&fs::read(&program).unwrap()),
                    "path": CONFIG,
                    "registry_path": CONFIG
                }]
            }}
        }))
        .unwrap(),
    )
    .unwrap();
    let selected_base = commit(&root);
    let (exit, selected, error) = invoke(
        &root,
        &[
            "capture",
            "--config",
            selector,
            "--commit",
            &selected_base,
            "--output",
            ".chrono-harness/state/selected-base.json",
        ],
    );
    assert_eq!(exit, 0, "{error}");
    assert_eq!(selected["schema"], "chrono-input-snapshot/v3");
    assert_eq!(selected["config_path"], selector);
    assert_eq!(selected["effective_config_path"], CONFIG);
    assert_eq!(selected["selection"]["config_path"], CONFIG);
    assert_eq!(selected["selection"]["registry_path"], CONFIG);
    for path in [
        Value::Null,
        value!(false),
        value!("../outside.json"),
        value!("/absolute.json"),
    ] {
        let mut invalid = selected.clone();
        invalid["selection"]["registry_path"] = path;
        assert!(chrono_judge_registration::inputs::snapshot_shape(&invalid).is_err());
    }
    fs::write(root.join("src.bin"), b"selected candidate").unwrap();
    let selected_candidate = commit(&root);
    let (exit, _, error) = invoke(
        &root,
        &[
            "capture",
            "--config",
            selector,
            "--commit",
            &selected_candidate,
            "--output",
            ".chrono-harness/state/selected-candidate.json",
        ],
    );
    assert_eq!(exit, 0, "{error}");
    let (exit, paired, error) = invoke(
        &root,
        &[
            "pair",
            "--base-snapshot",
            ".chrono-harness/state/selected-base.json",
            "--candidate-snapshot",
            ".chrono-harness/state/selected-candidate.json",
            "--output",
            ".chrono-harness/state/selected-paired.json",
        ],
    );
    assert_eq!(exit, 0, "{error}");
    assert_eq!(paired["base"]["schema"], "chrono-input-snapshot/v3");
    assert_eq!(paired["candidate"]["effective_config_path"], CONFIG);
    let manifest = ".chrono-harness/state/selected-compose.json";
    let original = fs::read(root.join(".chrono-harness/state/selected-paired.json")).unwrap();
    fs::write(root.join(manifest), serde_json::to_vec(&value!({"schema":"chrono-input-composition/v1",
        "sources":[{"pair":{"path":".chrono-harness/state/selected-paired.json","sha256":sha256(&original)}}]})).unwrap()).unwrap();
    assert_eq!(
        chrono_inputs::compose(
            &root,
            selector,
            &selected_base,
            &selected_candidate,
            manifest,
            ".chrono-harness/state/selected-composed.json",
            ".chrono-harness/state/selected-receipt.json"
        )
        .unwrap(),
        paired
    );
    for field in ["effective_config_path", "selection", "registry_path"] {
        let mut wrong = paired.clone();
        if field == "registry_path" {
            wrong["candidate"]["selection"][field] = value!(".chrono-harness/wrong-config.json");
        } else {
            wrong["candidate"][field] = if field == "selection" {
                let mut v = wrong["candidate"][field].clone();
                v["platform"] = value!("wrong-platform");
                v
            } else {
                value!("wrong-config.json")
            };
        }
        if field == "effective_config_path" {
            wrong["candidate"]["selection"]["config_path"] = value!("wrong-config.json");
        }
        let path = format!(".chrono-harness/state/wrong-{field}.json");
        let raw = serde_json::to_vec(&wrong).unwrap();
        fs::write(root.join(&path), &raw).unwrap();
        fs::write(
            root.join(manifest),
            serde_json::to_vec(&value!({"schema":"chrono-input-composition/v1",
            "sources":[{"pair":{"path":path,"sha256":sha256(&raw)}}]}))
            .unwrap(),
        )
        .unwrap();
        let error = chrono_inputs::compose(
            &root,
            selector,
            &selected_base,
            &selected_candidate,
            manifest,
            &format!(".chrono-harness/state/reject-{field}.json"),
            &format!(".chrono-harness/state/reject-{field}.receipt.json"),
        )
        .unwrap_err();
        assert!(error.contains("E_COMPOSE_HEADER"), "{error}");
    }
    fs::copy(env!("CARGO_BIN_EXE_chrono-inputs-test-zero"), &program).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let (exit, _, error) = invoke(
        &root,
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/rejected.json",
        ],
    );
    assert_ne!(exit, 0);
    assert!(error.contains("E_GIT_FACTS"));
    assert!(!state.join("rejected.json").exists());
    assert!(state.join("v3.json").exists());
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn compose_genuine_captures_retains_originals_and_rejects_conflicts_and_missing_coverage() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let (d, input, mut v, base) = fixture();
    let captured = chrono_inputs::capture(
        d.path(),
        CONFIG,
        &base,
        ".chrono-harness/state/base-capture.json",
    )
    .unwrap();
    // Historical input bytes remain genuine after the live input and declaration change.
    fs::write(input.path(), b"new actual endpoint bytes").unwrap();
    for i in v.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        i["sha256"] = value!(sha256(b"new actual endpoint bytes"));
    }
    write_values(d.path(), &v);
    let candidate = commit(d.path());
    let current = chrono_inputs::capture(
        d.path(),
        CONFIG,
        &candidate,
        ".chrono-harness/state/candidate-capture.json",
    )
    .unwrap();
    let pair = chrono_inputs::pair(
        d.path(),
        d.path(),
        ".chrono-harness/state/base-capture.json",
        d.path(),
        ".chrono-harness/state/candidate-capture.json",
        ".chrono-harness/state/constituent.json",
    )
    .unwrap();
    assert_eq!(pair, value!({"base":captured,"candidate":current}));
    let original = fs::read(d.path().join(".chrono-harness/state/constituent.json")).unwrap();
    let source = value!({"pair":{"path":".chrono-harness/state/constituent.json","sha256":sha256(&original)}});
    let manifest = ".chrono-harness/state/composition.json";
    fs::write(
        d.path().join(manifest),
        serde_json::to_vec(&value!({"schema":"chrono-input-composition/v1","sources":[source]}))
            .unwrap(),
    )
    .unwrap();
    fs::remove_file(input.path()).unwrap();
    let (code, result, err) = invoke(
        d.path(),
        &[
            "compose",
            "--config",
            CONFIG,
            "--base",
            &base,
            "--candidate",
            &candidate,
            "--manifest",
            manifest,
            "--output",
            ".chrono-harness/state/composed.json",
            "--receipt",
            ".chrono-harness/state/composed-receipt.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(result, pair);
    assert_eq!(
        fs::read(d.path().join(".chrono-harness/state/constituent.json")).unwrap(),
        original
    );
    let receipt =
        json(&fs::read(d.path().join(".chrono-harness/state/composed-receipt.json")).unwrap())
            .unwrap();
    assert_eq!(receipt["completeness_proven"], false);
    let retained = receipt["originals"][0]["retained"]["path"]
        .as_str()
        .unwrap();
    assert_eq!(fs::read(d.path().join(retained)).unwrap(), original);
    // Imported descriptors use only their explicit relocated transport. The
    // source capture blobs are gone before composition publishes them again.
    let upload = ".chrono-harness/state/composition-upload/";
    let download = ".chrono-harness/state/composition-download/";
    let pair_path = ".chrono-harness/state/constituent.json";
    let mut artifacts = serde_json::Map::new();
    artifacts.insert(
        pair_path.into(),
        chrono_harness::retained_artifacts::stage(d.path(), &d.path().join(pair_path), upload)
            .unwrap(),
    );
    let mut blobs = Vec::new();
    for snapshot in [&captured, &current] {
        let blob = snapshot["files"]["data"]["blob"].as_str().unwrap();
        let bytes = fs::read(d.path().join(blob)).unwrap();
        artifacts.insert(
            blob.into(),
            chrono_harness::retained_artifacts::stage(d.path(), &d.path().join(blob), upload)
                .unwrap(),
        );
        fs::remove_file(d.path().join(blob)).unwrap();
        blobs.push((blob, bytes));
    }
    fs::rename(d.path().join(upload), d.path().join(download)).unwrap();
    fs::write(
        d.path().join(manifest),
        serde_json::to_vec(&value!({"schema":"chrono-input-composition/v1","sources":[{
            "pair":{"path":pair_path,"sha256":sha256(&original)},
            "artifacts":artifacts,
            "transport":{"source_directory":upload,"directory":download}
        }]}))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        chrono_inputs::compose(
            d.path(),
            CONFIG,
            &base,
            &candidate,
            manifest,
            ".chrono-harness/state/transport-composed.json",
            ".chrono-harness/state/transport-receipt.json",
        )
        .unwrap(),
        pair
    );
    for (blob, bytes) in &blobs {
        assert_eq!(fs::read(d.path().join(blob)).unwrap(), *bytes);
    }
    // Neither a damaged content address nor a damaged transported original can
    // be overwritten or published as a successful composition.
    let (blob, bytes) = &blobs[0];
    fs::write(d.path().join(blob), b"damaged destination").unwrap();
    let error = chrono_inputs::compose(
        d.path(),
        CONFIG,
        &base,
        &candidate,
        manifest,
        ".chrono-harness/state/collision-composed.json",
        ".chrono-harness/state/collision-receipt.json",
    )
    .unwrap_err();
    assert!(error.contains("E_INPUT_BLOB"), "{error}");
    assert_eq!(
        fs::read(d.path().join(blob)).unwrap(),
        b"damaged destination"
    );
    fs::remove_file(d.path().join(blob)).unwrap();
    let storage = artifacts[*blob]["storage"].as_str().unwrap();
    let transported = d.path().join(format!(
        "{download}{}",
        storage.strip_prefix(upload).unwrap()
    ));
    fs::write(&transported, b"damaged original").unwrap();
    let error = chrono_inputs::compose(
        d.path(),
        CONFIG,
        &base,
        &candidate,
        manifest,
        ".chrono-harness/state/damaged-composed.json",
        ".chrono-harness/state/damaged-receipt.json",
    )
    .unwrap_err();
    assert!(error.contains("original artifact digest/length"), "{error}");
    assert!(!d.path().join(blob).exists());
    for case in ["collision", "damaged"] {
        assert!(
            !d.path()
                .join(format!(".chrono-harness/state/{case}-composed.json"))
                .exists()
        );
        assert!(
            !d.path()
                .join(format!(".chrono-harness/state/{case}-receipt.json"))
                .exists()
        );
    }
    fs::write(transported, bytes).unwrap();
    fs::write(d.path().join(blob), bytes).unwrap();
    for (case, expected) in [
        ("coverage", "E_COMPOSE_COVERAGE"),
        ("environment", "E_COMPOSE_ENVIRONMENT"),
        ("config", "E_COMPOSE_HEADER"),
        ("absence", "E_INPUT_SNAPSHOT"),
        ("endpoint", "E_COMPOSE_HEADER"),
        ("blob", "E_COMPOSE_BLOB"),
    ] {
        let mut bad = pair.clone();
        match case {
            "coverage" => {
                bad["base"]["files"].as_object_mut().unwrap().remove("data");
            }
            "environment" => {
                bad["base"]["environment"]["DECLARED_EMPTY"] = value!("different actual capture")
            }
            "config" => bad["base"]["config_digest"] = value!("0".repeat(64)),
            "absence" => bad["base"]["files"]["data"] = value!({"absent":true}),
            "endpoint" => bad["base"]["commit"] = value!(candidate),
            _ => bad["base"]["files"]["data"]["length"] = value!(1),
        }
        let path = format!(".chrono-harness/state/{case}.json");
        let raw = serde_json::to_vec(&bad).unwrap();
        fs::write(d.path().join(&path), &raw).unwrap();
        let source = value!({"pair":{"path":path,"sha256":sha256(&raw)}});
        let sources = if case == "environment" {
            value!([{"pair":{"path":".chrono-harness/state/constituent.json","sha256":sha256(&original)}},source])
        } else {
            value!([source])
        };
        fs::write(
            d.path().join(manifest),
            serde_json::to_vec(&value!({"schema":"chrono-input-composition/v1","sources":sources}))
                .unwrap(),
        )
        .unwrap();
        let error = chrono_inputs::compose(
            d.path(),
            CONFIG,
            &base,
            &candidate,
            manifest,
            &format!(".chrono-harness/state/out-{case}.json"),
            &format!(".chrono-harness/state/receipt-{case}.json"),
        )
        .unwrap_err();
        assert!(error.contains(expected), "{case}: {error}");
        assert!(
            !d.path()
                .join(format!(".chrono-harness/state/out-{case}.json"))
                .exists()
        );
    }
}

verus! {
struct CompositionSourceFixture {
    digest: String,
    id: u64,
    failure: u8,
}

type CompositionTrace = Vec<(u64, u8)>;

fn fixture_source_hash(mut trace: CompositionTrace, row: &CompositionSourceFixture)
    -> (out: (CompositionTrace, Result<String, String>))
    ensures out.0@ == trace@.push((row.id, 1u8)),
        if row.failure == 1 { match out.1 {
            Err(error) => error@ == "hash 原错\0\n"@, _ => false,
        } } else { out.1 == Ok::<String, String>(row.digest) },
{
    trace.push((row.id, 1));
    let result = if row.failure == 1 { Err("hash 原错\0\n".to_string()) }
        else { Ok(row.digest.clone()) };
    (trace, result)
}

fn fixture_source_import(mut trace: CompositionTrace, row: &CompositionSourceFixture)
    -> (out: (CompositionTrace, Result<((u64, bool), u64), String>))
    ensures out.0@ == trace@.push((row.id, 2u8)),
        if row.failure == 2 { match out.1 {
            Err(error) => error@ == "import 原错\0\n"@, _ => false,
        } } else { out.1 == Ok::<((u64, bool), u64), String>(((row.id, row.failure == 3), row.id)) },
{
    trace.push((row.id, 2));
    let result = if row.failure == 2 { Err("import 原错\0\n".to_string()) }
        else { Ok(((row.id, row.failure == 3), row.id)) };
    (trace, result)
}

fn fixture_source_merge(mut trace: CompositionTrace, part: (u64, bool))
    -> (out: (CompositionTrace, Result<(), String>))
    ensures out.0@ == trace@.push((part.0, 3u8)),
        if part.1 { match out.1 {
            Err(error) => error@ == "merge 原错\0\n"@, _ => false,
        } } else { out.1 == Ok::<(), String>(()) },
{
    trace.push((part.0, 3));
    (trace, if part.1 { Err("merge 原错\0\n".to_string()) } else { Ok(()) })
}

spec fn fixture_source_step_expected(
    initial: policy::composition::CompositionSources<CompositionTrace, u64>,
    row: &CompositionSourceFixture,
    out: (policy::composition::CompositionSources<CompositionTrace, u64>, Result<(), String>),
) -> bool {
    let duplicate = initial.seen@.contains(row.digest);
    let stops_before_import = row.failure == 1 || duplicate;
    let stops_before_merge = stops_before_import || row.failure == 2;
    out.0.state@ == if stops_before_import { initial.state@.push((row.id, 1u8)) }
        else if stops_before_merge { initial.state@.push((row.id, 1u8)).push((row.id, 2u8)) }
        else { initial.state@.push((row.id, 1u8)).push((row.id, 2u8)).push((row.id, 3u8)) }
    && out.0.originals@ == if stops_before_merge { initial.originals@ }
        else { initial.originals@.push(row.id) }
    && out.0.seen@ == if row.failure == 1 { initial.seen@ }
        else { initial.seen@.insert(row.digest) }
    && if row.failure == 1 { match out.1 {
        Err(error) => error@ == "hash 原错\0\n"@, _ => false,
    } } else if duplicate { match out.1 {
        Err(error) => error@ == "E_COMPOSE_ORIGINAL: duplicate source"@, _ => false,
    } } else if row.failure == 2 { match out.1 {
        Err(error) => error@ == "import 原错\0\n"@, _ => false,
    } } else if row.failure == 3 { match out.1 {
        Err(error) => error@ == "merge 原错\0\n"@, _ => false,
    } } else { out.1 == Ok::<(), String>(()) }
}

proof fn fixture_source_step_bound(
    initial: policy::composition::CompositionSources<CompositionTrace, u64>,
    row: &CompositionSourceFixture,
    out: (policy::composition::CompositionSources<CompositionTrace, u64>, Result<(), String>),
)
    requires policy::composition::composition_source_step_result(initial, row,
        &fixture_source_hash, &fixture_source_import, &fixture_source_merge, out),
    ensures fixture_source_step_expected(initial, row, out),
{
    reveal(policy::composition::composition_source_step_result);
    if initial.seen@.contains(row.digest) {
        assert(initial.seen@.insert(row.digest) =~= initial.seen@);
    }
}

proof fn fixture_source_prefix_step(
    rows: Seq<CompositionSourceFixture>,
    states: Seq<policy::composition::CompositionSources<CompositionTrace, u64>>,
    initial: policy::composition::CompositionSources<CompositionTrace, u64>, i: int,
)
    requires policy::composition::composition_sources_prefix(rows, states, initial,
        &fixture_source_hash, &fixture_source_import, &fixture_source_merge),
        0 <= i < states.len() - 1,
    ensures policy::composition::composition_source_step_result(states[i], &rows[i],
        &fixture_source_hash, &fixture_source_import, &fixture_source_merge,
        (states[i + 1], Ok::<(), String>(()))) ,
{
    reveal(policy::composition::composition_sources_prefix);
}

fn composition_trace(expected: &[(u64, u8)], actual: &[(u64, u8)])
    requires actual@ =~= expected@,
{
    let same_length = actual.len() == expected.len();
    assert(same_length);
    if !same_length { panic!(); }
    let mut i = 0;
    while i < actual.len()
        invariant i <= actual.len(), actual@ == expected@,
        decreases actual.len() - i,
    {
        let same = actual[i].0 == expected[i].0 && actual[i].1 == expected[i].1;
        assert(same);
        if !same { panic!(); }
        i += 1;
    }
}

fn composition_sources_case(failure: u8, duplicate: bool)
    requires failure <= 3, vstd::laws_cmp::obeys_cmp::<String>(),
{
    use policy::composition::*;
    let first = "来源\0\n".to_string();
    let second = if duplicate { first.clone() } else { "不同来源\0\n".to_string() };
    let last = "后续完整来源\0\n".to_string();
    proof {
        reveal_strlit("来源\0\n"); reveal_strlit("不同来源\0\n");
        reveal_strlit("后续完整来源\0\n");
        assert("来源\0\n"@.len() != "不同来源\0\n"@.len());
        assert("来源\0\n"@.len() != "后续完整来源\0\n"@.len());
        assert("不同来源\0\n"@.len() != "后续完整来源\0\n"@.len());
        assert((first == second) == duplicate);
        assert(first != last && second != last);
    }
    let rows = vec![
        CompositionSourceFixture { digest: first, id: 11, failure: 0 },
        CompositionSourceFixture { digest: second, id: 29, failure },
        CompositionSourceFixture { digest: last, id: 47, failure: 0 },
    ];
    let initial = CompositionSources::new(Vec::new());
    let ghost original = initial;
    let (progress, result) = compose_sources(initial, rows.as_slice(),
        &fixture_source_hash, &fixture_source_import, &fixture_source_merge);
    let ghost outcome = result;
    proof {
        let states = choose|states: Seq<CompositionSources<CompositionTrace, u64>>|
            #[trigger] composition_sources_prefix(rows@, states, original,
                &fixture_source_hash, &fixture_source_import, &fixture_source_merge)
            && match outcome {
                Ok(()) => states.len() == rows.len() + 1 && progress == states.last(),
                Err(error) => states.len() <= rows.len()
                    && composition_source_step_result(states.last(), &rows@[states.len() - 1],
                        &fixture_source_hash, &fixture_source_import, &fixture_source_merge,
                        (progress, Err::<(), String>(error))),
            };
        reveal(composition_sources_prefix);
        assert(composition_sources_prefix(rows@, states, original,
            &fixture_source_hash, &fixture_source_import, &fixture_source_merge));
        assert(forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] composition_source_step_result(states[i], &rows@[i],
                &fixture_source_hash, &fixture_source_import, &fixture_source_merge,
                (states[i + 1], Ok::<(), String>(()))) );
        if states.len() == 1 {
            match outcome {
                Ok(()) => { assert(false); },
                Err(error) => {
                    fixture_source_step_bound(original, &rows@[0], (progress, Err::<(), String>(error)));
                    assert(false);
                },
            }
        }
        assert(states.len() >= 2);
        assert(states[0] == original);
        fixture_source_prefix_step(rows@, states, original, 0);
        assert(composition_source_step_result(states[0], &rows@[0],
            &fixture_source_hash, &fixture_source_import, &fixture_source_merge,
            (states[1], Ok::<(), String>(()))) );
        fixture_source_step_bound(original, &rows@[0], (states[1], Ok::<(), String>(()))) ;
        assert(states[1].state@ == seq![(11u64, 1u8), (11u64, 2u8), (11u64, 3u8)]);
        assert(states[1].originals@ == seq![11u64]);
        assert(states[1].seen@ == Set::<String>::empty().insert(rows@[0].digest));
        assert(states[1].seen@.contains(rows@[1].digest) == duplicate);
        if states.len() >= 3 {
            fixture_source_prefix_step(rows@, states, original, 1);
            fixture_source_step_bound(states[1], &rows@[1], (states[2], Ok::<(), String>(()))) ;
            assert(failure == 0 && !duplicate);
            assert(!states[2].seen@.contains(rows@[2].digest));
        }
        match outcome {
            Ok(()) => {
                assert(states.len() == 4);
                fixture_source_prefix_step(rows@, states, original, 2);
                fixture_source_step_bound(states[2], &rows@[2], (progress, Ok::<(), String>(()))) ;
            },
            Err(error) => {
                if states.len() == 3 {
                    fixture_source_step_bound(states[2], &rows@[2], (progress, Err::<(), String>(error)));
                    assert(false);
                }
                assert(states.len() == 2);
                fixture_source_step_bound(states[1], &rows@[1], (progress, Err::<(), String>(error)));
            },
        }
    }
    if failure == 1 || duplicate {
        composition_trace(&[(11, 1), (11, 2), (11, 3), (29, 1)], progress.state.as_slice());
    } else if failure == 2 {
        composition_trace(&[(11, 1), (11, 2), (11, 3), (29, 1), (29, 2)], progress.state.as_slice());
    } else if failure == 0 {
        composition_trace(&[(11, 1), (11, 2), (11, 3), (29, 1), (29, 2), (29, 3),
            (47, 1), (47, 2), (47, 3)], progress.state.as_slice());
    } else {
        assert(progress.state@ == seq![(11u64, 1u8), (11u64, 2u8), (11u64, 3u8),
            (29u64, 1u8), (29u64, 2u8), (29u64, 3u8)]);
        composition_trace(&[(11, 1), (11, 2), (11, 3), (29, 1), (29, 2), (29, 3)], progress.state.as_slice());
    }
    let expected_originals = if failure == 1 || duplicate || failure == 2 { 1usize }
        else if failure == 0 { 3usize } else { 2usize };
    let original_count_matches = progress.originals.len() == expected_originals;
    assert(original_count_matches);
    if !original_count_matches { panic!(); }
    let first_original_matches = progress.originals[0] == 11;
    assert(first_original_matches);
    if !first_original_matches { panic!(); }
    if expected_originals >= 2 {
        let second_original_matches = progress.originals[1] == 29;
        assert(second_original_matches);
        if !second_original_matches { panic!(); }
    }
    if expected_originals == 3 {
        let last_original_matches = progress.originals[2] == 47;
        assert(last_original_matches);
        if !last_original_matches { panic!(); }
    }
    let expected_error = if failure == 1 { "hash 原错\0\n" }
        else if duplicate { "E_COMPOSE_ORIGINAL: duplicate source" }
        else if failure == 2 { "import 原错\0\n" }
        else if failure == 3 { "merge 原错\0\n" }
        else { "" };
    composition_result(result, failure == 0 && !duplicate, expected_error);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_sources_preserve_order_originals_and_every_first_failure()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    composition_sources_case(0, false);
    composition_sources_case(1, false);
    composition_sources_case(2, false);
    composition_sources_case(3, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_sources_reject_duplicate_before_import_and_keep_hash_error_priority()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    composition_sources_case(0, true);
    composition_sources_case(1, true);
    composition_sources_case(2, true);
    composition_sources_case(3, true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn composition_sources_empty_inventory_keeps_exact_initial_state()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    use policy::composition::*;
    let mut trace = Vec::new();
    trace.push((u64::MAX, u8::MAX));
    let initial = CompositionSources::<CompositionTrace, u64>::new(trace);
    let ghost original = initial;
    assert(original.state@ == seq![(u64::MAX, u8::MAX)]);
    let rows: Vec<CompositionSourceFixture> = Vec::new();
    let (progress, result) = compose_sources(initial, rows.as_slice(),
        &fixture_source_hash, &fixture_source_import, &fixture_source_merge);
    let ghost outcome = result;
    proof {
        let states = choose|states: Seq<CompositionSources<CompositionTrace, u64>>|
            #[trigger] composition_sources_prefix(rows@, states, original,
                &fixture_source_hash, &fixture_source_import, &fixture_source_merge)
            && match outcome {
                Ok(()) => states.len() == rows.len() + 1 && progress == states.last(),
                Err(error) => states.len() <= rows.len()
                    && composition_source_step_result(states.last(), &rows@[states.len() - 1],
                        &fixture_source_hash, &fixture_source_import, &fixture_source_merge,
                        (progress, Err::<(), String>(error))),
            };
        assert(states.len() == 1 && progress == original);
        assert(progress.state@ == seq![(u64::MAX, u8::MAX)]);
    }
    composition_trace(&[(u64::MAX, u8::MAX)], progress.state.as_slice());
    let no_originals = progress.originals.len() == 0;
    assert(no_originals);
    if !no_originals { panic!(); }
    composition_result(result, true, "");
}

}
