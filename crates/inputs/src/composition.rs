//! Offline composition of explicitly addressed original endpoint captures.
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{
    prepared::{ArtifactTransport, Original},
    retained_artifacts, sha256,
};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy)]
pub enum CompositionEndpoint { Base, Candidate }

impl CompositionEndpoint {
    pub fn name(self) -> (name: &'static str)
        ensures name@ == match self { Self::Base => "base"@, Self::Candidate => "candidate"@ },
    {
        match self { Self::Base => "base", Self::Candidate => "candidate" }
    }
}

/// 仅观察原声明数组与 ID 字段；匹配由本证明根完成。
pub trait CompositionInputRows {
    spec fn count(&self) -> nat;
    spec fn id_at(&self, index: int) -> Option<&str>;

    fn len(&self) -> (out: usize)
        ensures out == self.count();

    fn id(&self, index: usize) -> (out: Option<&str>)
        requires index < self.count(),
        ensures out == self.id_at(index as int);
}

pub open spec fn composition_input_matches<R: CompositionInputRows>(
    rows: &R, index: int, expected: Seq<char>) -> bool
{
    match rows.id_at(index) { Some(id) => id@ == expected, None => false }
}

pub open spec fn first_composition_input<R: CompositionInputRows>(
    rows: &R, expected: Seq<char>, out: Option<usize>) -> bool
{
    match out {
        Some(index) => index < rows.count() && composition_input_matches(rows, index as int, expected)
            && forall|j: int| 0 <= j < index ==> !composition_input_matches(rows, j, expected),
        None => forall|j: int| 0 <= j < rows.count() ==> !composition_input_matches(rows, j, expected),
    }
}

pub fn find_composition_input<R: CompositionInputRows>(rows: &R, expected: &str)
    -> (out: Option<usize>)
    ensures first_composition_input(rows, expected@, out),
{
    let count = rows.len();
    let mut index = 0usize;
    while index < count
        invariant index <= count, count == rows.count(),
            forall|j: int| 0 <= j < index ==> !composition_input_matches(rows, j, expected@),
        decreases count - index,
    {
        if let Some(id) = rows.id(index) {
            if identifiers::equal_text(id, expected) { return Some(index); }
        }
        assert(!composition_input_matches(rows, index as int, expected@));
        index += 1;
    }
    None
}

pub enum CompositionImportError<'a> {
    Unregistered(&'a str),
    Blob(String),
    Admission(&'a str, String),
}

pub open spec fn composition_import_error_parts(error: CompositionImportError, endpoint: Seq<char>,
    parts: [&str; 6]) -> bool
{
    parts@[0]@ == "E_COMPOSE_INPUT: "@ && parts@[1]@ == endpoint && match error {
        CompositionImportError::Unregistered(id) => parts@[2]@ == " unregistered "@
            && parts@[3]@ == id@ && parts@[4]@ == ""@ && parts@[5]@ == ""@,
        CompositionImportError::Blob(_) => false,
        CompositionImportError::Admission(id, error) => parts@[2]@ == " "@
            && parts@[3]@ == error@ && parts@[4]@ == " "@ && parts@[5]@ == id@,
    }
}

impl<'a> CompositionImportError<'a> {
    pub fn render<F>(self, endpoint: &str, render: &F) -> (out: String)
        where F: Fn([&str; 6]) -> String,
        requires forall|parts: [&str; 6]| composition_import_error_parts(self, endpoint@, parts)
            ==> #[trigger] render.requires((parts,)),
        ensures match self {
            Self::Blob(error) => out == error,
            _ => exists|parts: [&str; 6]| composition_import_error_parts(self, endpoint@, parts)
                && #[trigger] render.ensures((parts,), out),
        },
    {
        let ghost initial = self;
        match self {
            Self::Unregistered(id) => {
                let parts = ["E_COMPOSE_INPUT: ", endpoint, " unregistered ", id, "", ""];
                assert(composition_import_error_parts(initial, endpoint@, parts));
                render(parts)
            },
            Self::Blob(error) => error,
            Self::Admission(id, error) => {
                let detail = error.as_str();
                let parts = ["E_COMPOSE_INPUT: ", endpoint, " ", detail, " ", id];
                assert(composition_import_error_parts(initial, endpoint@, parts));
                render(parts)
            },
        }
    }
}

#[verifier::opaque]
pub open spec fn composition_import_file_result<'a, S, V: 'a, D, O, L, I, A, W>(
    initial: S, id: &'a str, value: &'a V, lookup: &L, import: &I, admit: &A, write: &W,
    out: (S, Result<(), CompositionImportError<'a>>)) -> bool
    where L: Fn(S, &'a str) -> (S, Option<D>),
        I: Fn(S, &'a V) -> (S, Result<O, String>),
        A: Fn(S, D, &O) -> (S, Result<(), String>),
        W: Fn(S, &'a str, O) -> S,
{
    exists|located: S, declaration: Option<D>|
        #[trigger] lookup.ensures((initial, id), (located, declaration))
        && match declaration {
            None => out.0 == located
                && out.1 == Err::<(), CompositionImportError<'a>>(CompositionImportError::Unregistered(id)),
            Some(declared) => exists|loaded: S, importing: Result<O, String>|
                #[trigger] import.ensures((located, value), (loaded, importing))
                && match importing {
                    Err(error) => out.0 == loaded
                        && out.1 == Err::<(), CompositionImportError<'a>>(CompositionImportError::Blob(error)),
                    Ok(observed) => exists|admitted: S, decision: Result<(), String>|
                        #[trigger] admit.ensures((loaded, declared, &observed), (admitted, decision))
                        && match decision {
                            Err(error) => out.0 == admitted
                                && out.1 == Err::<(), CompositionImportError<'a>>(
                                    CompositionImportError::Admission(id, error)),
                            Ok(()) => out.1 == Ok::<(), CompositionImportError<'a>>(())
                                && write.ensures((admitted, id, observed), out.0),
                        },
                },
        }
}

/// 查声明、导入、准入后才写入；每个失败保留该阶段状态与原错误。
pub fn import_composition_file<'a, S, V: 'a, D, O, L, I, A, W>(
    initial: S, id: &'a str, value: &'a V, lookup: &L, import: &I, admit: &A, write: &W,
) -> (out: (S, Result<(), CompositionImportError<'a>>))
    where L: Fn(S, &'a str) -> (S, Option<D>),
        I: Fn(S, &'a V) -> (S, Result<O, String>),
        A: Fn(S, D, &O) -> (S, Result<(), String>),
        W: Fn(S, &'a str, O) -> S,
    requires forall|state: S, id: &'a str| lookup.requires((state, id)),
        forall|state: S, value: &'a V| import.requires((state, value)),
        forall|state: S, declared: D, observed: &O| admit.requires((state, declared, observed)),
        forall|state: S, id: &'a str, observed: O| write.requires((state, id, observed)),
    ensures composition_import_file_result(initial, id, value, lookup, import, admit, write, out),
{
    reveal(composition_import_file_result);
    let ghost start = initial;
    let (state, declaration) = lookup(initial, id);
    let ghost located = state;
    let ghost locating = declaration;
    assert(lookup.ensures((start, id), (located, locating)));
    let declared = match declaration {
        None => return (state, Err(CompositionImportError::Unregistered(id))),
        Some(declared) => declared,
    };
    let (state, importing) = import(state, value);
    let ghost loaded = state;
    let ghost loading = importing;
    assert(import.ensures((located, value), (loaded, loading)));
    let observed = match importing {
        Err(error) => return (state, Err(CompositionImportError::Blob(error))),
        Ok(observed) => observed,
    };
    let ghost observation = observed;
    let (state, decision) = admit(state, declared, &observed);
    let ghost admitted = state;
    let ghost admitting = decision;
    assert(admit.ensures((loaded, declared, &observation), (admitted, admitting)));
    match decision {
        Err(error) => (state, Err(CompositionImportError::Admission(id, error))),
        Ok(()) => {
            let next = write(state, id, observed);
            assert(write.ensures((admitted, id, observation), next));
            (next, Ok(()))
        },
    }
}

pub open spec fn composition_import_files_prefix<'a, S, V: 'a, D, O, L, I, A, W>(
    initial: S, rows: Seq<(&'a str, &'a V)>, states: Seq<S>,
    lookup: &L, import: &I, admit: &A, write: &W) -> bool
    where L: Fn(S, &'a str) -> (S, Option<D>),
        I: Fn(S, &'a V) -> (S, Result<O, String>),
        A: Fn(S, D, &O) -> (S, Result<(), String>),
        W: Fn(S, &'a str, O) -> S,
{
    0 < states.len() <= rows.len() + 1 && states[0] == initial
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] composition_import_file_result(states[i], rows[i].0, rows[i].1,
                lookup, import, admit, write, (states[i + 1], Ok::<(), CompositionImportError<'a>>(())))
}

/// 保留原文件顺序及成功前缀；首错后不再处理或写入后续项。
pub fn import_composition_files<'a, S, V: 'a, D, O, L, I, A, W>(
    initial: S, rows: &[(&'a str, &'a V)], lookup: &L, import: &I, admit: &A, write: &W,
) -> (out: (S, Result<(), CompositionImportError<'a>>))
    where L: Fn(S, &'a str) -> (S, Option<D>),
        I: Fn(S, &'a V) -> (S, Result<O, String>),
        A: Fn(S, D, &O) -> (S, Result<(), String>),
        W: Fn(S, &'a str, O) -> S,
    requires forall|state: S, id: &'a str| lookup.requires((state, id)),
        forall|state: S, value: &'a V| import.requires((state, value)),
        forall|state: S, declared: D, observed: &O| admit.requires((state, declared, observed)),
        forall|state: S, id: &'a str, observed: O| write.requires((state, id, observed)),
    ensures exists|states: Seq<S>|
        #[trigger] composition_import_files_prefix(initial, rows@, states, lookup, import, admit, write)
        && match out.1 {
            Ok(()) => states.len() == rows.len() + 1 && out.0 == states.last(),
            Err(error) => states.len() <= rows.len()
                && composition_import_file_result(states.last(), rows@[states.len() - 1].0,
                    rows@[states.len() - 1].1, lookup, import, admit, write,
                    (out.0, Err::<(), CompositionImportError<'a>>(error))),
        },
{
    let ghost start = initial;
    let mut current = initial;
    let ghost states = seq![start];
    let mut i = 0usize;
    while i < rows.len()
        invariant start == initial, i <= rows.len(), states.len() == i + 1,
            states.last() == current,
            composition_import_files_prefix(start, rows@, states, lookup, import, admit, write),
            forall|state: S, id: &'a str| lookup.requires((state, id)),
            forall|state: S, value: &'a V| import.requires((state, value)),
            forall|state: S, declared: D, observed: &O| admit.requires((state, declared, observed)),
            forall|state: S, id: &'a str, observed: O| write.requires((state, id, observed)),
        decreases rows.len() - i,
    {
        let ghost before = current;
        let (id, value) = rows[i];
        let (next, result) = import_composition_file(current, id, value, lookup, import, admit, write);
        let ghost outcome = result;
        assert(composition_import_file_result(before, id, value, lookup, import, admit, write,
            (next, outcome)));
        match result {
            Err(error) => {
                let out = (next, Err(error));
                assert(states.len() <= rows.len());
                assert(composition_import_files_prefix(initial, rows@, states, lookup, import, admit, write));
                assert(composition_import_file_result(states.last(), rows@[states.len() - 1].0,
                    rows@[states.len() - 1].1, lookup, import, admit, write,
                    (out.0, Err::<(), CompositionImportError<'a>>(error))));
                return out;
            },
            Ok(unit) => {
                assert(unit == ());
                assert(outcome == Ok::<(), CompositionImportError<'a>>(()));
                assert(before == states.last());
                proof {
                    let previous = states;
                    states = states.push(next);
                    assert forall|j: int| 0 <= j < states.len() - 1 implies
                        #[trigger] composition_import_file_result(states[j], rows@[j].0, rows@[j].1,
                            lookup, import, admit, write,
                            (states[j + 1], Ok::<(), CompositionImportError<'a>>(()))) by {
                        if j < previous.len() - 1 {
                            assert(composition_import_file_result(previous[j], rows@[j].0, rows@[j].1,
                                lookup, import, admit, write,
                                (previous[j + 1], Ok::<(), CompositionImportError<'a>>(()))));
                        }
                    }
                }
                current = next;
            },
        }
        i += 1;
    }
    (current, Ok(()))
}

pub enum CompositionMergeError<'a> {
    Environment(CompositionEndpoint),
    Files(String),
    Conflict(CompositionEndpoint, &'a str),
}

pub open spec fn composition_merge_files_prefix<'a, S, V: 'a, P, W>(
    initial: S, endpoint: CompositionEndpoint, rows: Seq<(&'a str, &'a V)>,
    states: Seq<S>, prior: &P, write: &W) -> bool
    where P: Fn(&S, CompositionEndpoint, &'a str, &'a V) -> Option<bool>,
        W: Fn(S, CompositionEndpoint, &'a str, &'a V) -> S,
{
    0 < states.len() <= rows.len() + 1 && states[0] == initial
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            (exists|same: Option<bool>|
                #[trigger] prior.ensures((&states[i], endpoint, rows[i].0, rows[i].1), same)
                && same != Some(false))
            && #[trigger] write.ensures((states[i], endpoint, rows[i].0, rows[i].1), states[i + 1])
}

pub open spec fn composition_merge_files_result<'a, S, V: 'a, P, W>(
    initial: S, endpoint: CompositionEndpoint, rows: Seq<(&'a str, &'a V)>,
    prior: &P, write: &W, out: (S, Result<(), CompositionMergeError<'a>>)) -> bool
    where P: Fn(&S, CompositionEndpoint, &'a str, &'a V) -> Option<bool>,
        W: Fn(S, CompositionEndpoint, &'a str, &'a V) -> S,
{
    exists|states: Seq<S>|
        #[trigger] composition_merge_files_prefix(initial, endpoint, rows, states, prior, write)
        && out.0 == states.last() && match out.1 {
            Ok(()) => states.len() == rows.len() + 1,
            Err(error) => states.len() <= rows.len()
                && error == CompositionMergeError::Conflict(endpoint, rows[states.len() - 1].0)
                && prior.ensures((&states.last(), endpoint, rows[states.len() - 1].0,
                    rows[states.len() - 1].1), Some(false)),
        }
}

/// Merge in original map order, preserving successful writes and stopping before the first conflict.
pub fn merge_composition_files<'a, S, V: 'a, P, W>(
    initial: S, endpoint: CompositionEndpoint, rows: &[(&'a str, &'a V)],
    prior: &P, write: &W) -> (out: (S, Result<(), CompositionMergeError<'a>>))
    where P: Fn(&S, CompositionEndpoint, &'a str, &'a V) -> Option<bool>,
        W: Fn(S, CompositionEndpoint, &'a str, &'a V) -> S,
    requires forall|state: &S, endpoint: CompositionEndpoint, id: &'a str, value: &'a V|
            prior.requires((state, endpoint, id, value)),
        forall|state: S, endpoint: CompositionEndpoint, id: &'a str, value: &'a V|
            write.requires((state, endpoint, id, value)),
    ensures composition_merge_files_result(initial, endpoint, rows@, prior, write, out),
{
    let mut state = initial;
    let ghost states = seq![initial];
    let mut i = 0usize;
    while i < rows.len()
        invariant i <= rows.len(), states.len() == i + 1, states.last() == state,
            composition_merge_files_prefix(initial, endpoint, rows@, states, prior, write),
            forall|state: &S, endpoint: CompositionEndpoint, id: &'a str, value: &'a V|
                prior.requires((state, endpoint, id, value)),
            forall|state: S, endpoint: CompositionEndpoint, id: &'a str, value: &'a V|
                write.requires((state, endpoint, id, value)),
        decreases rows.len() - i,
    {
        let (id, value) = rows[i];
        let same = prior(&state, endpoint, id, value);
        assert(prior.ensures((&state, endpoint, id, value), same));
        if same == Some(false) {
            let out = (state, Err(CompositionMergeError::Conflict(endpoint, id)));
            assert(composition_merge_files_result(initial, endpoint, rows@, prior, write, out));
            return out;
        }
        let ghost before = state;
        assert(before == states.last());
        let next = write(state, endpoint, id, value);
        assert(write.ensures((before, endpoint, id, value), next));
        proof {
            let previous = states;
            states = states.push(next);
            assert forall|j: int| 0 <= j < states.len() - 1 implies
                (exists|same: Option<bool>|
                    #[trigger] prior.ensures((&states[j], endpoint, rows@[j].0, rows@[j].1), same)
                    && same != Some(false))
                && #[trigger] write.ensures((states[j], endpoint, rows@[j].0, rows@[j].1), states[j + 1]) by {
                if j < previous.len() - 1 {
                    assert(write.ensures((previous[j], endpoint, rows@[j].0, rows@[j].1), previous[j + 1]));
                    assert(exists|same: Option<bool>|
                        #[trigger] prior.ensures((&previous[j], endpoint, rows@[j].0, rows@[j].1), same)
                        && same != Some(false));
                } else {
                    assert(j == i);
                    assert(before == previous[j]);
                    assert(prior.ensures((&before, endpoint, rows@[j].0, rows@[j].1), same));
                }
            }
            assert(composition_merge_files_prefix(initial, endpoint, rows@, states, prior, write));
        }
        state = next;
        i += 1;
    }
    (state, Ok(()))
}

pub open spec fn composition_merge_initialization<'a, S, T, P, I>(
    initial: S, endpoint: CompositionEndpoint, snapshot: &'a T,
    present: &P, initialize: &I, out: S) -> bool
    where P: Fn(&S, CompositionEndpoint) -> bool,
        I: Fn(S, CompositionEndpoint, &'a T) -> S,
{
    exists|already: bool| #[trigger] present.ensures((&initial, endpoint), already)
        && if already { out == initial }
            else { initialize.ensures((initial, endpoint, snapshot), out) }
}

#[verifier::opaque]
pub open spec fn composition_merge_endpoint_result<'a, S, T, V: 'a, P, I, E, F, R, W>(
    initial: S, endpoint: CompositionEndpoint, snapshot: Option<&'a T>,
    present: &P, initialize: &I, environment: &E, files: &F, prior: &R, write: &W,
    out: (S, Result<(), CompositionMergeError<'a>>)) -> bool
    where P: Fn(&S, CompositionEndpoint) -> bool,
        I: Fn(S, CompositionEndpoint, &'a T) -> S,
        E: Fn(&S, CompositionEndpoint, &'a T) -> bool,
        F: Fn(&'a T) -> Result<Vec<(&'a str, &'a V)>, String>,
        R: Fn(&S, CompositionEndpoint, &'a str, &'a V) -> Option<bool>,
        W: Fn(S, CompositionEndpoint, &'a str, &'a V) -> S,
{
    match snapshot {
        None => out.0 == initial && out.1 == Ok::<(), CompositionMergeError<'a>>(()),
        Some(snapshot) => exists|initialized: S|
            #[trigger] composition_merge_initialization(initial, endpoint, snapshot, present, initialize, initialized)
            && exists|same: bool|
                #[trigger] environment.ensures((&initialized, endpoint, snapshot), same)
                && if !same {
                    out.0 == initialized && out.1 == Err::<(), CompositionMergeError<'a>>(
                        CompositionMergeError::Environment(endpoint))
                } else { exists|rows: Result<Vec<(&'a str, &'a V)>, String>|
                    #[trigger] files.ensures((snapshot,), rows) && match rows {
                        Err(error) => out.0 == initialized && out.1 == Err::<(), CompositionMergeError<'a>>(
                            CompositionMergeError::Files(error)),
                        Ok(rows) => composition_merge_files_result(initialized, endpoint, rows@, prior, write, out),
                    }
                },
    }
}

/// Missing endpoints are skipped; initialize before environment admission and file iteration.
pub fn merge_composition_endpoint<'a, S, T, V: 'a, P, I, E, F, R, W>(
    initial: S, endpoint: CompositionEndpoint, snapshot: Option<&'a T>,
    present: &P, initialize: &I, environment: &E, files: &F, prior: &R, write: &W)
    -> (out: (S, Result<(), CompositionMergeError<'a>>))
    where P: Fn(&S, CompositionEndpoint) -> bool,
        I: Fn(S, CompositionEndpoint, &'a T) -> S,
        E: Fn(&S, CompositionEndpoint, &'a T) -> bool,
        F: Fn(&'a T) -> Result<Vec<(&'a str, &'a V)>, String>,
        R: Fn(&S, CompositionEndpoint, &'a str, &'a V) -> Option<bool>,
        W: Fn(S, CompositionEndpoint, &'a str, &'a V) -> S,
    requires forall|state: &S, endpoint: CompositionEndpoint| present.requires((state, endpoint)),
        forall|state: S, endpoint: CompositionEndpoint, snapshot: &'a T|
            initialize.requires((state, endpoint, snapshot)),
        forall|state: &S, endpoint: CompositionEndpoint, snapshot: &'a T|
            environment.requires((state, endpoint, snapshot)),
        forall|snapshot: &'a T| files.requires((snapshot,)),
        forall|state: &S, endpoint: CompositionEndpoint, id: &'a str, value: &'a V|
            prior.requires((state, endpoint, id, value)),
        forall|state: S, endpoint: CompositionEndpoint, id: &'a str, value: &'a V|
            write.requires((state, endpoint, id, value)),
    ensures composition_merge_endpoint_result(initial, endpoint, snapshot,
        present, initialize, environment, files, prior, write, out),
{
    reveal(composition_merge_endpoint_result);
    let snapshot = match snapshot {
        None => return (initial, Ok(())),
        Some(snapshot) => snapshot,
    };
    let already = present(&initial, endpoint);
    assert(present.ensures((&initial, endpoint), already));
    let state = if already { initial } else { initialize(initial, endpoint, snapshot) };
    assert(already || initialize.ensures((initial, endpoint, snapshot), state));
    assert(composition_merge_initialization(initial, endpoint, snapshot, present, initialize, state));
    let same = environment(&state, endpoint, snapshot);
    assert(environment.ensures((&state, endpoint, snapshot), same));
    if !same { return (state, Err(CompositionMergeError::Environment(endpoint))); }
    let rows = files(snapshot);
    assert(files.ensures((snapshot,), rows));
    match rows {
        Err(error) => (state, Err(CompositionMergeError::Files(error))),
        Ok(rows) => merge_composition_files(state, endpoint, rows.as_slice(), prior, write),
    }
}

/// Base completes before candidate; an error retains the exact first endpoint result.
pub fn merge_composition_part<'a, S, T, V: 'a, P, I, E, F, R, W>(
    initial: S, base: Option<&'a T>, candidate: Option<&'a T>,
    present: &P, initialize: &I, environment: &E, files: &F, prior: &R, write: &W)
    -> (out: (S, Result<(), CompositionMergeError<'a>>))
    where P: Fn(&S, CompositionEndpoint) -> bool,
        I: Fn(S, CompositionEndpoint, &'a T) -> S,
        E: Fn(&S, CompositionEndpoint, &'a T) -> bool,
        F: Fn(&'a T) -> Result<Vec<(&'a str, &'a V)>, String>,
        R: Fn(&S, CompositionEndpoint, &'a str, &'a V) -> Option<bool>,
        W: Fn(S, CompositionEndpoint, &'a str, &'a V) -> S,
    requires forall|state: &S, endpoint: CompositionEndpoint| present.requires((state, endpoint)),
        forall|state: S, endpoint: CompositionEndpoint, snapshot: &'a T|
            initialize.requires((state, endpoint, snapshot)),
        forall|state: &S, endpoint: CompositionEndpoint, snapshot: &'a T|
            environment.requires((state, endpoint, snapshot)),
        forall|snapshot: &'a T| files.requires((snapshot,)),
        forall|state: &S, endpoint: CompositionEndpoint, id: &'a str, value: &'a V|
            prior.requires((state, endpoint, id, value)),
        forall|state: S, endpoint: CompositionEndpoint, id: &'a str, value: &'a V|
            write.requires((state, endpoint, id, value)),
    ensures exists|state: S, result: Result<(), CompositionMergeError<'a>>|
        #[trigger] composition_merge_endpoint_result(initial, CompositionEndpoint::Base, base,
            present, initialize, environment, files, prior, write, (state, result))
        && match result {
            Err(error) => out.0 == state && out.1 == Err::<(), CompositionMergeError<'a>>(error),
            Ok(()) => composition_merge_endpoint_result(state, CompositionEndpoint::Candidate, candidate,
                present, initialize, environment, files, prior, write, out),
        },
{
    let (state, result) = merge_composition_endpoint(initial, CompositionEndpoint::Base, base,
        present, initialize, environment, files, prior, write);
    match result {
        Err(error) => (state, Err(error)),
        Ok(()) => merge_composition_endpoint(state, CompositionEndpoint::Candidate, candidate,
            present, initialize, environment, files, prior, write),
    }
}

pub open spec fn optional_text_is(text: Option<&str>, expected: Seq<char>) -> bool {
    match text { Some(text) => text@ == expected, None => false, }
}

fn text_is(text: Option<&str>, expected: &str) -> (same: bool)
    ensures same == optional_text_is(text, expected@),
{
    match text { Some(text) => identifiers::equal_text(text, expected), None => false, }
}

pub fn admit_json_original(length: u64) -> (out: Result<(), String>)
    ensures out.is_ok() == (length <= 64 * 1024 * 1024),
        match out { Err(error) => error@ == "E_COMPOSE_BOUND: JSON original exceeds bound"@, _ => true, },
{
    if length > 64 * 1024 * 1024 {
        Err("E_COMPOSE_BOUND: JSON original exceeds bound".to_string())
    } else { Ok(()) }
}

pub fn admit_manifest(schema: &str, count: usize) -> (out: Result<(), String>)
    ensures out.is_ok() == (schema@ == "chrono-input-composition/v1"@ && 0 < count <= 4096),
        match out { Err(error) => error@ == "E_COMPOSE_MANIFEST: schema/count"@, _ => true, },
{
    if !identifiers::equal_text(schema, "chrono-input-composition/v1") || count == 0 || count > 4096 {
        Err("E_COMPOSE_MANIFEST: schema/count".to_string())
    } else { Ok(()) }
}

pub fn admit_output_paths(output: &str, receipt: &str) -> (out: Result<(), String>)
    ensures out.is_ok() == (output@ != receipt@),
        match out { Err(error) => error@ == "E_COMPOSE_PATH: output/receipt collision"@, _ => true, },
{
    if identifiers::equal_text(output, receipt) {
        Err("E_COMPOSE_PATH: output/receipt collision".to_string())
    } else { Ok(()) }
}

pub fn admit_pair_digest(actual: &str, expected: &str) -> (out: Result<(), String>)
    ensures out.is_ok() == (actual@ == expected@),
        match out { Err(error) => error@ == "E_COMPOSE_ORIGINAL: pair digest differs"@, _ => true, },
{
    if identifiers::equal_text(actual, expected) { Ok(()) }
    else { Err("E_COMPOSE_ORIGINAL: pair digest differs".to_string()) }
}

pub fn admit_single_endpoint(commit: Option<&str>, base: &str, candidate: &str)
    -> (out: Result<(), String>)
    ensures out.is_ok() == (optional_text_is(commit, base@) || optional_text_is(commit, candidate@)),
        match out { Err(error) => error@ == "E_COMPOSE_HEADER: snapshot outside fixed endpoints"@, _ => true, },
{
    if text_is(commit, base) || text_is(commit, candidate) { Ok(()) }
    else { Err("E_COMPOSE_HEADER: snapshot outside fixed endpoints".to_string()) }
}

pub fn admit_pair_fields(count: Option<usize>, has_base: bool, has_candidate: bool)
    -> (out: Result<(), String>)
    ensures out.is_ok() == (count == Some(2usize) && has_base && has_candidate),
        match out { Err(error) => error@ == "E_COMPOSE_PAIR: exact endpoints required"@, _ => true, },
{
    if count == Some(2usize) && has_base && has_candidate { Ok(()) }
    else { Err("E_COMPOSE_PAIR: exact endpoints required".to_string()) }
}

pub struct SnapshotHeader<'a> {
    pub digest: Option<&'a str>,
    pub selection_present: bool,
    pub effective_present: bool,
    pub effective: Option<&'a str>,
}

pub fn admit_snapshot_identity(schema: Option<&str>, commit: Option<&str>, config: Option<&str>,
    expected_schema: &str, expected_commit: &str, expected_config: &str)
    -> (out: Result<(), String>)
    ensures out.is_ok() == (optional_text_is(schema, expected_schema@)
        && optional_text_is(commit, expected_commit@) && optional_text_is(config, expected_config@)),
        match out { Err(error) => error@ == "configuration/endpoint/selector mismatch"@, _ => true, },
{
    if text_is(schema, expected_schema) && text_is(commit, expected_commit) && text_is(config, expected_config) {
        Ok(())
    } else { Err("configuration/endpoint/selector mismatch".to_string()) }
}

pub open spec fn snapshot_header_admitted(observed: SnapshotHeader, digest: Seq<char>, selected: bool,
    selection_equal: bool, effective: Seq<char>) -> bool
{
    optional_text_is(observed.digest, digest)
        && if selected { selection_equal && optional_text_is(observed.effective, effective) }
        else { !observed.selection_present && !observed.effective_present }
}

/// JSON selector equality is an observation; this root owns its admission together with every header.
pub fn admit_snapshot_header(observed: &SnapshotHeader, digest: &str,
    selected: bool, selection_equal: bool, effective: &str)
    -> (out: Result<(), String>)
    ensures out.is_ok() == snapshot_header_admitted(*observed, digest@, selected, selection_equal, effective@),
        match out { Err(error) => error@ == "configuration/endpoint/selector mismatch"@, _ => true, },
{
    let identity = text_is(observed.digest, digest);
    let selector = if selected { selection_equal && text_is(observed.effective, effective) }
        else { !observed.selection_present && !observed.effective_present };
    if identity && selector { Ok(()) }
    else { Err("configuration/endpoint/selector mismatch".to_string()) }
}

pub fn admit_imported_blob(actual: Option<&str>, actual_length: Option<u64>,
    expected: &str, expected_length: u64) -> (out: Result<(), String>)
    ensures out.is_ok() == (optional_text_is(actual, expected@) && actual_length == Some(expected_length)),
        match out { Err(error) => error@ == "E_COMPOSE_BLOB: original identity differs"@, _ => true, },
{
    let matches = match (actual, actual_length) {
        (Some(actual), Some(length)) => super::blob_identity_matches(actual, length, expected, expected_length),
        _ => false,
    };
    if matches { Ok(()) }
    else { Err("E_COMPOSE_BLOB: original identity differs".to_string()) }
}

/// Error text is the original suffix; the caller only renders the endpoint and input ID.
pub fn admit_imported_input(absent: bool, presence: Option<&str>,
    digest_equal: bool) -> (out: Result<(), String>)
    ensures out.is_ok() == (if absent { optional_text_is(presence, "absent"@) }
            else { !optional_text_is(presence, "absent"@) && digest_equal }),
        match out { Err(error) => error@ == (if absent { "unexpected absence"@ }
            else { "digest/presence mismatch"@ }), _ => true, },
{
    if absent {
        if text_is(presence, "absent") { Ok(()) } else { Err("unexpected absence".to_string()) }
    } else if text_is(presence, "absent") || !digest_equal {
        Err("digest/presence mismatch".to_string())
    } else { Ok(()) }
}
pub struct CompositionSources<S, O> {
    pub state: S,
    pub originals: Vec<O>,
    pub seen: BTreeSet<String>,
}

impl<S, O> CompositionSources<S, O> {
    pub fn new(state: S) -> (out: Self)
        ensures out.state == state, out.originals@ == Seq::<O>::empty(),
            out.seen@ == Set::<String>::empty(),
    {
        Self { state, originals: Vec::new(), seen: BTreeSet::new() }
    }
}

#[verifier::opaque]
pub open spec fn composition_source_step_result<S, X, T, O, H, I, M>(
    initial: CompositionSources<S, O>, row: &X, hash: &H, import: &I, merge: &M,
    out: (CompositionSources<S, O>, Result<(), String>)) -> bool
    where H: Fn(S, &X) -> (S, Result<String, String>),
        I: Fn(S, &X) -> (S, Result<(T, O), String>),
        M: Fn(S, T) -> (S, Result<(), String>),
{
    exists|hashed: S, digest: Result<String, String>|
        #[trigger] hash.ensures((initial.state, row), (hashed, digest))
        && match digest {
            Err(error) => out.0.state == hashed
                && out.0.originals@ == initial.originals@
                && out.0.seen@ == initial.seen@
                && out.1 == Err::<(), String>(error),
            Ok(key) => if initial.seen@.contains(key) {
                out.0.state == hashed && out.0.originals@ == initial.originals@
                    && out.0.seen@ == initial.seen@
                    && match out.1 {
                        Err(error) => error@ == "E_COMPOSE_ORIGINAL: duplicate source"@,
                        _ => false,
                    }
            } else {
                out.0.seen@ == initial.seen@.insert(key)
                    && exists|imported: S, importing: Result<(T, O), String>|
                        #[trigger] import.ensures((hashed, row), (imported, importing))
                        && match importing {
                            Err(error) => out.0.state == imported
                                && out.0.originals@ == initial.originals@
                                && out.1 == Err::<(), String>(error),
                            Ok((part, original)) => out.0.originals@
                                == initial.originals@.push(original)
                                && merge.ensures((imported, part), (out.0.state, out.1)),
                        }
            },
        }
}

/// 按原顺序计算摘要、拒绝重复、导入、记录原件，再合并；保留每阶段首错与状态。
pub fn compose_source_step<S, X, T, O, H, I, M>(
    progress: CompositionSources<S, O>, row: &X, hash: &H, import: &I, merge: &M,
) -> (out: (CompositionSources<S, O>, Result<(), String>))
    where H: Fn(S, &X) -> (S, Result<String, String>),
        I: Fn(S, &X) -> (S, Result<(T, O), String>),
        M: Fn(S, T) -> (S, Result<(), String>),
    requires vstd::laws_cmp::obeys_cmp::<String>(),
        forall|state: S, row: &X| hash.requires((state, row)),
        forall|state: S, row: &X| import.requires((state, row)),
        forall|state: S, part: T| merge.requires((state, part)),
    ensures composition_source_step_result(progress, row, hash, import, merge, out),
{
    reveal(composition_source_step_result);
    let ghost initial = progress;
    let CompositionSources { state, mut originals, mut seen } = progress;
    let (state, digest) = hash(state, row);
    let ghost hashed = state;
    let ghost hashing = digest;
    assert(hash.ensures((initial.state, row), (hashed, hashing)));
    let key = match digest {
        Err(error) => return (CompositionSources { state, originals, seen }, Err(error)),
        Ok(key) => key,
    };
    let ghost identity = key;
    if !seen.insert(key) {
        assert(seen@ =~= initial.seen@);
        return (CompositionSources { state, originals, seen },
            Err("E_COMPOSE_ORIGINAL: duplicate source".to_string()));
    }
    let (state, importing) = import(state, row);
    let ghost imported = state;
    let ghost import_result = importing;
    assert(import.ensures((hashed, row), (imported, import_result)));
    let (part, original) = match importing {
        Err(error) => return (CompositionSources { state, originals, seen }, Err(error)),
        Ok(value) => value,
    };
    originals.push(original);
    let (state, result) = merge(state, part);
    assert(merge.ensures((imported, part), (state, result)));
    (CompositionSources { state, originals, seen }, result)
}

pub open spec fn composition_sources_prefix<S, X, T, O, H, I, M>(
    rows: Seq<X>, states: Seq<CompositionSources<S, O>>,
    initial: CompositionSources<S, O>, hash: &H, import: &I, merge: &M) -> bool
    where H: Fn(S, &X) -> (S, Result<String, String>),
        I: Fn(S, &X) -> (S, Result<(T, O), String>),
        M: Fn(S, T) -> (S, Result<(), String>),
{
    0 < states.len() <= rows.len() + 1 && states[0] == initial
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] composition_source_step_result(states[i], &rows[i], hash, import, merge,
                (states[i + 1], Ok::<(), String>(())))
}

/// 完整遍历实际来源库存；只在全部来源成功后返回成功，首错不消费后续来源。
pub fn compose_sources<S, X, T, O, H, I, M>(
    progress: CompositionSources<S, O>, rows: &[X], hash: &H, import: &I, merge: &M,
) -> (out: (CompositionSources<S, O>, Result<(), String>))
    where H: Fn(S, &X) -> (S, Result<String, String>),
        I: Fn(S, &X) -> (S, Result<(T, O), String>),
        M: Fn(S, T) -> (S, Result<(), String>),
    requires vstd::laws_cmp::obeys_cmp::<String>(),
        forall|state: S, row: &X| hash.requires((state, row)),
        forall|state: S, row: &X| import.requires((state, row)),
        forall|state: S, part: T| merge.requires((state, part)),
    ensures exists|states: Seq<CompositionSources<S, O>>|
        #[trigger] composition_sources_prefix(rows@, states, progress, hash, import, merge)
        && match out.1 {
            Ok(()) => states.len() == rows.len() + 1 && out.0 == states.last(),
            Err(error) => states.len() <= rows.len()
                && composition_source_step_result(states.last(), &rows@[states.len() - 1],
                    hash, import, merge, (out.0, Err::<(), String>(error))),
        },
{
    let ghost initial = progress;
    let mut current = progress;
    let ghost states = seq![initial];
    let mut i = 0usize;
    while i < rows.len()
        invariant initial == progress, i <= rows.len(), states.len() == i + 1,
            states.last() == current,
            composition_sources_prefix(rows@, states, initial, hash, import, merge),
            vstd::laws_cmp::obeys_cmp::<String>(),
            forall|state: S, row: &X| hash.requires((state, row)),
            forall|state: S, row: &X| import.requires((state, row)),
            forall|state: S, part: T| merge.requires((state, part)),
        decreases rows.len() - i,
    {
        let ghost before = current;
        let (next, result) = compose_source_step(current, &rows[i], hash, import, merge);
        let ghost original = result;
        assert(composition_source_step_result(before, &rows@[i as int], hash, import, merge,
            (next, original)));
        match result {
            Err(error) => {
                let out = (next, Err(error));
                assert(states.len() <= rows.len());
                assert(composition_sources_prefix(rows@, states, progress, hash, import, merge));
                assert(composition_source_step_result(states.last(), &rows@[states.len() - 1],
                    hash, import, merge, (out.0, Err::<(), String>(error))));
                assert(out.1 == Err::<(), String>(error));
                return out;
            },
            Ok(value) => {
                assert(value == ());
                assert(original == Ok::<(), String>(value));
                assert(original == Ok::<(), String>(()));
                assert(before == states.last());
                assert(composition_source_step_result(states.last(), &rows@[i as int],
                    hash, import, merge, (next, Ok::<(), String>(()))) );
                proof {
                    let previous = states;
                    states = states.push(next);
                    assert forall|j: int| 0 <= j < states.len() - 1 implies
                        #[trigger] composition_source_step_result(states[j], &rows@[j],
                            hash, import, merge, (states[j + 1], Ok::<(), String>(()))) by {
                        if j < previous.len() - 1 {
                            assert(composition_source_step_result(previous[j], &rows@[j],
                                hash, import, merge, (previous[j + 1], Ok::<(), String>(()))) );
                        }
                    }
                }
                current = next;
            },
        }
        i += 1;
    }
    (current, Ok(()))
}

}

#[cfg(not(verus_keep_ghost))]
impl CompositionInputRows for Vec<Value> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn id(&self, index: usize) -> Option<&str> {
        self[index]["id"].as_str()
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub pair: Original,
    #[serde(default)]
    pub transport: Option<ArtifactTransport>,
    #[serde(default)]
    pub artifacts: Option<Value>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub sources: Vec<Source>,
}
#[cfg(not(verus_keep_ghost))]
fn bounded(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    let _input = retained_artifacts::retention::read_guard(root, path)?;
    let p = no_symlink_parents(root, path)?;
    admit_json_original(fs::metadata(&p).map_err(|e| e.to_string())?.len())?;
    fs::read(p).map_err(|e| e.to_string())
}
#[cfg(not(verus_keep_ghost))]
fn source_bytes(root: &Path, source: &Source) -> Result<Vec<u8>, String> {
    let raw = if let Some(a) = &source.artifacts {
        retained_artifacts::bytes(root, a, &source.pair.path, source.transport.as_ref())?
    } else {
        return chrono_harness::prepared::read_original(
            root,
            &source.pair,
            source.transport.as_ref(),
        );
    };
    admit_pair_digest(&sha256(&raw), &source.pair.sha256)?;
    Ok(raw)
}
#[cfg(not(verus_keep_ghost))]
fn import_blob(root: &Path, f: &Value, source: &Source) -> Result<Value, String> {
    let (path, digest, length) = match retained_shape(f)? {
        Retained::Blob {
            path,
            digest,
            length,
        } => (path, digest, length),
        Retained::Absent => return Ok(f.clone()),
        Retained::Bytes(_) => {
            return Err("E_COMPOSE_ORIGINAL: capture blob reference required".into());
        }
    };
    let Some(a) = &source.artifacts else {
        let original = Original {
            path: path.into(),
            sha256: digest.into(),
        };
        let p = chrono_harness::prepared::original_path(&original, source.transport.as_ref())?;
        let _input = retained_artifacts::retention::read_guard(root, &p)?;
        let observed = store(root, &no_symlink_parents(root, &p)?)?;
        admit_imported_blob(
            observed["sha256"].as_str(),
            observed["length"].as_u64(),
            digest,
            length,
        )?;
        return Ok(observed);
    };
    let (stage, actual, copied) = custody::StagedBlob::write(root, length, |output| {
        retained_artifacts::copy_to(root, a, path, source.transport.as_ref(), output)
    })?;
    admit_imported_blob(Some(&actual), Some(copied), digest, length)?;
    publish_blob(root, &stage.path, actual, copied)
}
#[cfg(not(verus_keep_ghost))]
fn manifest(root: &Path, path: &str) -> Result<(Vec<u8>, Manifest), String> {
    let raw = bounded(root, path)?;
    let manifest: Manifest = chrono_harness::decode(&raw)?;
    admit_manifest(&manifest.schema, manifest.sources.len())?;
    Ok((raw, manifest))
}
/// Import addressed capture data, never execute its historical producer/judges.
/// A source is an original snapshot at either endpoint or an exact endpoint pair.
#[cfg(not(verus_keep_ghost))]
fn import_source(
    root: &Path,
    source: &Source,
    config: &str,
    base: &str,
    candidate: &str,
    old: &Registrations,
    new: &Registrations,
) -> Result<(Value, Value), String> {
    let bytes = source_bytes(root, source)?;
    let v = json(&bytes)?;
    let single = v.get("schema").is_some();
    if single {
        snapshot_shape(&v)?;
        admit_single_endpoint(v["commit"].as_str(), base, candidate)?;
    } else {
        admit_pair_fields(
            v.as_object().map(|m| m.len()),
            v.get("base").is_some(),
            v.get("candidate").is_some(),
        )?;
    }
    let originals = ".chrono-harness/state/inputs/originals";
    fs::create_dir_all(state(root, originals)?).map_err(|e| e.to_string())?;
    let retained_path = format!("{originals}/pair-{}.json", sha256(&bytes));
    publish_bytes(root, &retained_path, &bytes)?;
    let retained = Original {
        path: retained_path,
        sha256: sha256(&bytes),
    };
    let original = value!({"source":source.pair,"retained":retained,"transport":source.transport});
    let mut pair = value!({});
    for (name, commit, r) in [("base", base, old), ("candidate", candidate, new)] {
        if single && v["commit"] != commit {
            continue;
        }
        let snapshot = if single { &v } else { &v[name] };
        if !single {
            snapshot_shape(snapshot)?;
        }
        admit_snapshot_identity(
            snapshot["schema"].as_str(),
            snapshot["commit"].as_str(),
            snapshot["config_path"].as_str(),
            snapshot_schema_for(r),
            commit,
            config,
        )
        .map_err(|error| format!("E_COMPOSE_HEADER: {name} {error}"))?;
        admit_snapshot_header(
            &SnapshotHeader {
                digest: snapshot["config_digest"].as_str(),
                selection_present: snapshot.get("selection").is_some(),
                effective_present: snapshot.get("effective_config_path").is_some(),
                effective: snapshot["effective_config_path"].as_str(),
            },
            &wire::digest(r.config())?,
            r.selection().is_some(),
            Some(&snapshot["selection"]) == r.selection(),
            r.effective_config_path(),
        )
        .map_err(|error| format!("E_COMPOSE_HEADER: {name} {error}"))?;
        chrono_judge_registration::inputs::environment(r.config(), &snapshot["environment"])?;
        pair[name] = snapshot.clone();
        let files: Vec<_> = snapshot["files"]
            .as_object()
            .ok_or("snapshot files")?
            .iter()
            .map(|(id, file)| (id.as_str(), file))
            .collect();
        let (next, imported) = import_composition_files(
            pair,
            files.as_slice(),
            &|state, id| {
                let inputs = r.config()["environment"]["inputs"].as_array().unwrap();
                (
                    state,
                    find_composition_input(inputs, id).map(|index| &inputs[index]),
                )
            },
            &|state, file| (state, import_blob(root, file, source)),
            &|state, declared: &Value, observed: &Value| {
                (
                    state,
                    admit_imported_input(
                        observed["absent"] == true,
                        declared["presence"].as_str(),
                        observed["sha256"] == declared["sha256"],
                    ),
                )
            },
            &|mut state: Value, id, observed| {
                state[name]["files"][id] = observed;
                state
            },
        );
        pair = next;
        imported.map_err(|error| error.render(name, &|parts: [&str; 6]| parts.concat()))?;
    }
    Ok((pair, original))
}

/// No live business inputs, SDK probes or inferred dependencies are consulted.
#[cfg(not(verus_keep_ghost))]
pub fn compose(
    root: &Path,
    config: &str,
    base: &str,
    candidate: &str,
    manifest_path: &str,
    output: &str,
    receipt: &str,
) -> Result<Produced, String> {
    let read_scope = retained_artifacts::retention::read_scope(root)?;
    output_path(root, output)?;
    output_path(root, receipt)?;
    admit_output_paths(output, receipt)?;
    let (raw, manifest) = manifest(root, manifest_path)?;
    let reader = facts::Reader::for_config(root, config)?;
    let old = Registrations::load(&reader.registry_values(root, base, config)?, config)?;
    let new = Registrations::load(&reader.registry_values(root, candidate, config)?, config)?;
    let selected = if chrono_harness::units::full_execution_units(new.config())?.is_some() {
        let delta = facts::delta(&reader.tree(root, base)?, &reader.tree(root, candidate)?);
        let (impact, findings) = chrono_judge_filemap::produce_for_endpoints(
            root, base, candidate, &delta, &old, &new, &reader,
        )?;
        if findings.iter().any(|f| f.level == "error") {
            return Err(format!(
                "E_COMPOSE_SELECTION: {}",
                serde_json::to_string(&findings).map_err(|e| e.to_string())?
            ));
        }
        Some(chrono_judge_routes::global_selection(
            &old,
            &new,
            &impact,
            chrono_judge_registration::downstream_validator(&new, "routes"),
        )?)
    } else {
        None
    };
    let (composition, composed) = compose_sources(
        CompositionSources::new(value!({})),
        &manifest.sources,
        &|pair, source| {
            let digest = serde_json::to_value(source)
                .map_err(|error| error.to_string())
                .and_then(|value| wire::digest(&value));
            (pair, digest)
        },
        &|pair, source| {
            (
                pair,
                import_source(root, source, config, base, candidate, &old, &new),
            )
        },
        &|pair, part: Value| {
            let (pair, merged) = merge_composition_part(
                pair,
                part.get("base"),
                part.get("candidate"),
                &|pair: &Value, endpoint: CompositionEndpoint| pair.get(endpoint.name()).is_some(),
                &|mut pair: Value, endpoint: CompositionEndpoint, snapshot: &Value| {
                    pair[endpoint.name()] = snapshot.clone();
                    pair[endpoint.name()]["files"] = value!({});
                    pair
                },
                &|pair: &Value, endpoint: CompositionEndpoint, snapshot: &Value| {
                    pair[endpoint.name()]["environment"] == snapshot["environment"]
                },
                &|snapshot: &Value| {
                    snapshot["files"]
                        .as_object()
                        .ok_or_else(|| "snapshot files".to_string())
                        .map(|files| files.iter().map(|(id, file)| (id.as_str(), file)).collect())
                },
                &|pair: &Value, endpoint: CompositionEndpoint, id: &str, file: &Value| {
                    pair[endpoint.name()]["files"]
                        .get(id)
                        .map(|prior| prior == file)
                },
                &|mut pair: Value, endpoint: CompositionEndpoint, id: &str, file: &Value| {
                    pair[endpoint.name()]["files"][id] = file.clone();
                    pair
                },
            );
            let merged = merged.map_err(|error| match error {
                CompositionMergeError::Environment(endpoint) => format!(
                    "E_COMPOSE_ENVIRONMENT: {} conflicting originals",
                    endpoint.name()
                ),
                CompositionMergeError::Files(error) => error,
                CompositionMergeError::Conflict(endpoint, id) => format!(
                    "E_COMPOSE_CONFLICT: {} overlapping input {id}",
                    endpoint.name()
                ),
            });
            (pair, merged)
        },
    );
    composed?;
    let pair = composition.state;
    let originals = composition.originals;
    for (name, r) in [("base", &old), ("candidate", &new)] {
        if pair.get(name).is_none() {
            return Err(format!(
                "E_COMPOSE_COVERAGE: {name} missing endpoint original"
            ));
        }
        let required = if let Some(selected) = &selected {
            chrono_judge_registration::inputs::collection_required_files(
                name, &old, &new, selected,
            )?
        } else {
            chrono_judge_registration::inputs::required_files(r, None)?
        };
        for id in required {
            if pair[name]["files"].get(&id).is_none() {
                return Err(format!("E_COMPOSE_COVERAGE: {name} missing input {id}"));
            }
        }
    }
    publish(root, output, &pair)?;
    let mut blobs = custody::blobs(&pair["base"]);
    blobs.extend(custody::blobs(&pair["candidate"]));
    blobs.sort();
    blobs.dedup();
    custody::dependencies(root, output, &blobs)?;
    let provenance = value!({"schema":"chrono-input-composition-result/v1","manifest":{"path":manifest_path,"sha256":sha256(&raw)},"base":base,"candidate":candidate,"config_path":config,"output":{"path":output,"sha256":sha256(&wire::canonical(&pair)?)},"originals":originals,"completeness_proven":false});
    publish(root, receipt, &provenance)?;
    let mut dependencies = vec![output.to_owned(), manifest_path.to_owned()];
    dependencies.extend(
        provenance["originals"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|original| original["retained"]["path"].as_str().map(str::to_owned)),
    );
    custody::dependencies(root, receipt, &dependencies)?;
    Produced::new(root, receipt, pair, read_scope)
}
