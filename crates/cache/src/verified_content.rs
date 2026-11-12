//! Cache artifact observations, complete missing lists and publication selection.
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
use std::collections::BTreeMap;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub type ArtifactRow<'a> = (&'a str, Option<&'a str>);
pub type MissingArtifacts<'a> = BTreeMap<&'a str, &'a str>;

pub open spec fn rows_view<'a>(rows: Option<&[ArtifactRow<'a>]>)
    -> Option<Seq<ArtifactRow<'a>>>
{
    match rows { None => None, Some(rows) => Some(rows@) }
}

/// Each interface returns its actual state and result. Error kinds are raw tags,
/// not an adapter-computed acceptance decision.
pub struct ArtifactIo<R, O, T, F> {
    pub resolve: R,
    pub observe: O,
    pub error_text: T,
    pub render: F,
}

pub open spec fn callable<S, P, E, R, O, T, F>(io: &ArtifactIo<R, O, T, F>) -> bool
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
{
    (forall|state: S, path: &str| #[trigger] io.resolve.requires((state, path)))
        && (forall|state: S, path: &P| #[trigger] io.observe.requires((state, path)))
        && (forall|error: E| #[trigger] io.error_text.requires((error,)))
        && (forall|parts: [&str; 4]| #[trigger] io.render.requires((parts,)))
}

pub open spec fn formatted<F>(relative: &str, detail: &str, render: &F, error: String) -> bool
    where F: Fn([&str; 4]) -> String,
{
    render.ensures((["E_CACHE_CONTENT: ", relative, ": ", detail],), error)
}

pub fn content_error<F>(relative: &str, detail: &str, render: &F) -> (error: String)
    where F: Fn([&str; 4]) -> String,
    requires forall|parts: [&str; 4]| render.requires((parts,)),
    ensures formatted(relative, detail, render, error),
{
    render(["E_CACHE_CONTENT: ", relative, ": ", detail])
}

pub open spec fn artifact_result<S, P, E, R, O, T, F>(
    initial: S, relative: &str, missing_kind: u32, io: &ArtifactIo<R, O, T, F>,
    out: (S, Result<bool, String>),
) -> bool
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
{
    exists|resolved: (S, Result<P, String>)|
        #[trigger] io.resolve.ensures((initial, relative), resolved)
        && match resolved.1 {
            Err(error) => out == (resolved.0, Err::<bool, String>(error)),
            Ok(target) => exists|observed: (S, Result<bool, (u32, E)>)|
                #[trigger] io.observe.ensures((resolved.0, &target), observed)
                && out.0 == observed.0
                && match observed.1 {
                    Ok(true) => out.1 == Ok::<bool, String>(true),
                    Ok(false) => match out.1 {
                        Err(error) => formatted(relative,
                            "cache artifact must be a physical directory", &io.render, error),
                        _ => false,
                    },
                    Err((kind, original)) => if kind == missing_kind {
                        out.1 == Ok::<bool, String>(false)
                    } else {
                        exists|detail: String|
                            #[trigger] io.error_text.ensures((original,), detail)
                            && exists|text: &str| text@ == detail@
                                && match out.1 {
                                    Err(error) => formatted(relative, text, &io.render, error),
                                    _ => false,
                                }
                    },
                },
        }
}

fn check_artifact<S, P, E, R, O, T, F>(
    initial: S, relative: &str, missing_kind: u32, io: &ArtifactIo<R, O, T, F>,
) -> (out: (S, Result<bool, String>))
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
    requires callable(io),
    ensures artifact_result(initial, relative, missing_kind, io, out),
{
    let resolved = (io.resolve)(initial, relative);
    let ghost original_resolved = resolved;
    assert(io.resolve.ensures((initial, relative), original_resolved));
    let (state, target) = resolved;
    let target = match target {
        Err(error) => {
            let out = (state, Err(error));
            assert(artifact_result(initial, relative, missing_kind, io, out));
            return out;
        },
        Ok(target) => target,
    };
    let observed = (io.observe)(state, &target);
    let ghost original_observed = observed;
    assert(io.observe.ensures((state, &target), original_observed));
    let (state, observation) = observed;
    let result = match observation {
        Ok(true) => Ok(true),
        Ok(false) => Err(content_error(relative,
            "cache artifact must be a physical directory", &io.render)),
        Err((kind, error)) => if kind == missing_kind { Ok(false) } else {
            let detail = (io.error_text)(error);
            Err(content_error(relative, detail.as_str(), &io.render))
        },
    };
    let out = (state, result);
    assert(artifact_result(initial, relative, missing_kind, io, out));
    out
}

pub open spec fn row_result<S, P, E, R, O, T, F>(
    initial: S, row: ArtifactRow, missing_kind: u32, io: &ArtifactIo<R, O, T, F>,
    out: (S, Result<bool, String>),
) -> bool
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
{
    match row.1 {
        None => out.0 == initial && match out.1 {
            Err(error) => error@ == "E_CACHE_CONTENT: artifact path"@,
            _ => false,
        },
        Some(relative) => artifact_result(initial, relative, missing_kind, io, out),
    }
}

pub open spec fn missing_map<'a>(
    rows: Seq<ArtifactRow<'a>>, present: Seq<bool>,
) -> Map<&'a str, &'a str>
    recommends present.len() <= rows.len(),
    decreases present.len(),
{
    if present.len() == 0 { Map::empty() }
    else {
        let previous = missing_map(rows, present.drop_last());
        let row = rows[present.len() - 1];
        match row.1 {
            Some(path) => if present.last() { previous } else { previous.insert(row.0, path) },
            None => previous,
        }
    }
}

pub open spec fn scan_prefix<S, P, E, R, O, T, F>(
    rows: Seq<ArtifactRow>, initial: S, states: Seq<S>, present: Seq<bool>,
    missing_kind: u32, io: &ArtifactIo<R, O, T, F>,
) -> bool
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
{
    present.len() <= rows.len() && states.len() == present.len() + 1
        && states.first() == initial
        && forall|i: int| 0 <= i < present.len() ==>
            #[trigger] row_result(states[i], rows[i], missing_kind, io,
                (states[i + 1], Ok::<bool, String>(present[i])))
}

pub open spec fn scan_result<'a, S, P, E, R, O, T, F>(
    initial: S, rows: Seq<ArtifactRow<'a>>, states: Seq<S>, present: Seq<bool>,
    missing_kind: u32, io: &ArtifactIo<R, O, T, F>,
    out: (S, Result<MissingArtifacts<'a>, String>),
) -> bool
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
{
    scan_prefix(rows, initial, states, present, missing_kind, io)
        && match out.1 {
            Ok(absent) => present.len() == rows.len() && out.0 == states.last()
                && absent@ == missing_map(rows, present),
            Err(error) => present.len() < rows.len()
                && row_result(states.last(), rows[present.len() as int], missing_kind, io,
                    (out.0, Err::<bool, String>(error))),
        }
}

pub open spec fn preflight_result<'a, S, P, E, R, O, T, F>(
    initial: S, rows: Seq<ArtifactRow<'a>>, missing_kind: u32,
    io: &ArtifactIo<R, O, T, F>, out: (S, Result<MissingArtifacts<'a>, String>),
) -> bool
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
{
    exists|states: Seq<S>, present: Seq<bool>|
        #[trigger] scan_result(initial, rows, states, present, missing_kind, io, out)
}

proof fn finish_scan<'a, S, P, E, R, O, T, F>(
    initial: S, rows: Seq<ArtifactRow<'a>>, states: Seq<S>, present: Seq<bool>,
    missing_kind: u32, io: &ArtifactIo<R, O, T, F>,
    out: (S, Result<MissingArtifacts<'a>, String>),
)
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
    requires scan_result(initial, rows, states, present, missing_kind, io, out),
    ensures preflight_result(initial, rows, missing_kind, io, out),
{
    reveal(preflight_result);
    assert(exists|final_states: Seq<S>, final_present: Seq<bool>|
        #[trigger] scan_result::<S, P, E, R, O, T, F>(initial, rows, final_states, final_present,
            missing_kind, io, out));
}

pub fn preflight<'a, S, P, E, R, O, T, F>(
    initial: S, rows: &[ArtifactRow<'a>], missing_kind: u32,
    io: &ArtifactIo<R, O, T, F>,
) -> (out: (S, Result<MissingArtifacts<'a>, String>))
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
    requires callable(io), vstd::laws_cmp::obeys_cmp::<&str>(),
    ensures preflight_result(initial, rows@, missing_kind, io, out),
{
    let mut state = initial;
    let mut absent = BTreeMap::new();
    let mut index = 0;
    let ghost states = seq![initial];
    let ghost present = Seq::<bool>::empty();
    while index < rows.len()
        invariant index <= rows.len(), states.len() == index as int + 1,
            present.len() == index, state == states.last(),
            scan_prefix(rows@, initial, states, present, missing_kind, io),
            absent@ == missing_map(rows@, present), callable(io),
            vstd::laws_cmp::obeys_cmp::<&str>(),
        decreases rows.len() - index,
    {
        let row = rows[index];
        let observed = match row.1 {
            None => (state, Err("E_CACHE_CONTENT: artifact path".to_owned())),
            Some(relative) => check_artifact(state, relative, missing_kind, io),
        };
        let ghost original = observed;
        assert(row_result(state, row, missing_kind, io, original));
        let (next, result) = observed;
        let exists = match result {
            Err(error) => {
                assert(original == (next, Err::<bool, String>(error)));
                let out = (next, Err(error));
                assert(scan_result(initial, rows@, states, present, missing_kind, io, out));
                proof { finish_scan(initial, rows@, states, present, missing_kind, io, out); }
                assert(preflight_result(initial, rows@, missing_kind, io, out));
                return out;
            },
            Ok(exists) => exists,
        };
        let ghost previous_absent = absent@;
        if !exists {
            match row.1 {
                Some(path) => { absent.insert(row.0, path); },
                None => { assert(false); },
            }
        }
        proof {
            let previous_states = states;
            let previous_present = present;
            states = states.push(next);
            present = present.push(exists);
            assert(present.drop_last() == previous_present);
            assert(present.last() == exists);
            assert(present.len() - 1 == index);
            assert(rows@[present.len() - 1] == row);
            assert(missing_map(rows@, present) == match row.1 {
                Some(path) => if exists { previous_absent }
                    else { previous_absent.insert(row.0, path) },
                None => previous_absent,
            });
            assert(absent@ == missing_map(rows@, present));
            assert forall|i: int| 0 <= i < present.len() implies
                #[trigger] row_result(states[i], rows@[i], missing_kind, io,
                    (states[i + 1], Ok::<bool, String>(present[i]))) by {
                if i < previous_present.len() {
                    assert(row_result(previous_states[i], rows@[i], missing_kind, io,
                        (previous_states[i + 1], Ok::<bool, String>(previous_present[i]))));
                } else {
                    assert(i == index);
                    assert(row_result(state, row, missing_kind, io, original));
                }
            }
        }
        state = next;
        index += 1;
    }
    let out = (state, Ok(absent));
    assert(scan_result(initial, rows@, states, present, missing_kind, io, out));
    proof { finish_scan(initial, rows@, states, present, missing_kind, io, out); }
    assert(preflight_result(initial, rows@, missing_kind, io, out));
    out
}

pub open spec fn dispatch_result<'a, S, P, E, A, R, O, T, F, N, Y>(
    initial: S, rows: Seq<ArtifactRow<'a>>, missing_kind: u32,
    io: &ArtifactIo<R, O, T, F>, unavailable: N, available: Y,
    out: (S, Result<A, String>),
) -> bool
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
        N: FnOnce(S, MissingArtifacts<'a>) -> (S, Result<A, String>),
        Y: FnOnce(S) -> (S, Result<A, String>),
{
    exists|checked: (S, Result<MissingArtifacts<'a>, String>)|
        #[trigger] preflight_result(initial, rows, missing_kind, io, checked)
            && match checked.1 {
                Err(error) => out == (checked.0, Err::<A, String>(error)),
                Ok(absent) => if absent@.is_empty() { available.ensures((checked.0,), out) }
                    else { unavailable.ensures((checked.0, absent), out) },
            }
}

/// Select a single producer only after every artifact has been checked. A later
/// real error still wins over an earlier absence; it never authorizes a marker.
pub fn dispatch_present<'a, S, P, E, A, R, O, T, F, N, Y>(
    initial: S, rows: &[ArtifactRow<'a>], missing_kind: u32,
    io: &ArtifactIo<R, O, T, F>, unavailable: N, available: Y,
) -> (out: (S, Result<A, String>))
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
        N: FnOnce(S, MissingArtifacts<'a>) -> (S, Result<A, String>),
        Y: FnOnce(S) -> (S, Result<A, String>),
    requires callable(io), vstd::laws_cmp::obeys_cmp::<&str>(),
        forall|state: S, absent: MissingArtifacts<'a>| unavailable.requires((state, absent)),
        forall|state: S| available.requires((state,)),
    ensures dispatch_result(initial, rows@, missing_kind,
        io, unavailable, available, out),
{
    let checked = preflight(initial, rows, missing_kind, io);
    let ghost original = checked;
    assert(preflight_result(initial, rows@, missing_kind, io, original));
    let (state, result) = checked;
    let out = match result {
        Err(error) => (state, Err(error)),
        Ok(absent) => if absent.is_empty() {
            assert(absent@.is_empty());
            let out = available(state);
            assert(available.ensures((state,), out));
            out
        } else {
            assert(!absent@.is_empty());
            let ghost original_absent = absent;
            let out = unavailable(state, absent);
            assert(unavailable.ensures((state, original_absent), out));
            out
        },
    };
    assert(match original.1 {
        Err(error) => out == (original.0, Err::<A, String>(error)),
        Ok(absent) => if absent@.is_empty() { available.ensures((original.0,), out) }
            else { unavailable.ensures((original.0, absent), out) },
    });
    assert(exists|checked: (S, Result<MissingArtifacts<'a>, String>)|
        #[trigger] preflight_result(initial, rows@, missing_kind, io, checked)
            && match checked.1 {
                Err(error) => out == (checked.0, Err::<A, String>(error)),
                Ok(absent) => if absent@.is_empty() { available.ensures((checked.0,), out) }
                    else { unavailable.ensures((checked.0, absent), out) },
            });
    assert(dispatch_result(initial, rows@, missing_kind,
        io, unavailable, available, out));
    out
}

pub open spec fn with_artifacts_result<'a, S, P, E, A, R, O, T, F, N, Y>(
    initial: S, rows: Option<Seq<ArtifactRow<'a>>>, missing_kind: u32,
    io: &ArtifactIo<R, O, T, F>, unavailable: N, available: Y,
    out: (S, Result<A, String>),
) -> bool
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
        N: FnOnce(S, MissingArtifacts<'a>) -> (S, Result<A, String>),
        Y: FnOnce(S) -> (S, Result<A, String>),
{
    match rows {
        None => out.0 == initial && match out.1 {
            Err(error) => error@ == "E_CACHE_CONTENT: artifacts missing"@,
            _ => false,
        },
        Some(rows) => dispatch_result(initial, rows, missing_kind, io, unavailable, available, out),
    }
}

pub fn with_artifacts<'a, S, P, E, A, R, O, T, F, N, Y>(
    initial: S, rows: Option<&[ArtifactRow<'a>]>, missing_kind: u32,
    io: &ArtifactIo<R, O, T, F>, unavailable: N, available: Y,
) -> (out: (S, Result<A, String>))
    where R: Fn(S, &str) -> (S, Result<P, String>),
        O: Fn(S, &P) -> (S, Result<bool, (u32, E)>), T: Fn(E) -> String,
        F: Fn([&str; 4]) -> String,
        N: FnOnce(S, MissingArtifacts<'a>) -> (S, Result<A, String>),
        Y: FnOnce(S) -> (S, Result<A, String>),
    requires callable(io), vstd::laws_cmp::obeys_cmp::<&str>(),
        forall|state: S, absent: MissingArtifacts<'a>| unavailable.requires((state, absent)),
        forall|state: S| available.requires((state,)),
    ensures with_artifacts_result(initial, rows_view(rows), missing_kind,
        io, unavailable, available, out),
{
    match rows {
        None => (initial, Err("E_CACHE_CONTENT: artifacts missing".to_owned())),
        Some(rows) => dispatch_present(initial, rows, missing_kind, io, unavailable, available),
    }
}

}
