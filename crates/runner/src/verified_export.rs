//! 快照导出的选择、逐阶段状态、首错及容量合同；IO 由真实接口逐项转交。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
use super::{Tree, TreePath, TreeEntry, identifiers, checkout_symbolic_link};
#[cfg(verus_keep_ghost)]
use super::{tree_rows_ordered, tree_rows_view};
use super::tree_rows;

verus! {

pub struct SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F> {
    pub prefetch: B,
    pub locate: L,
    pub parent: A,
    pub create_parent: C,
    pub read: R,
    pub decode: D,
    pub symlink: Y,
    pub write: W,
    pub metadata: G,
    pub readonly: O,
    pub permissions: F,
}

pub open spec fn callable<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>) -> bool
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
{
    (forall|s: S, paths: &Vec<String>| #[trigger] io.prefetch.requires((s, paths)))
    && (forall|s: S, path: &String| #[trigger] io.locate.requires((s, path)))
    && (forall|s: S, path: &P| #[trigger] io.parent.requires((s, path)))
    && (forall|s: S, path: P| #[trigger] io.create_parent.requires((s, path)))
    && (forall|s: S, path: &String| #[trigger] io.read.requires((s, path)))
    && (forall|s: S, bytes: Vec<u8>| #[trigger] io.decode.requires((s, bytes)))
    && (forall|s: S, path: &P, text: String| #[trigger] io.symlink.requires((s, path, text)))
    && (forall|s: S, path: &P, bytes: Vec<u8>| #[trigger] io.write.requires((s, path, bytes)))
    && (forall|s: S, path: &P| #[trigger] io.metadata.requires((s, path)))
    && (forall|s: S, permissions: M, readonly: bool| #[trigger] io.readonly.requires((s, permissions, readonly)))
    && (forall|s: S, path: &P, permissions: M| #[trigger] io.permissions.requires((s, path, permissions)))
}

/// checked_add 的溢出优先于容量拒绝；相等允许写入。
pub fn byte_total(previous: u64, bytes: u64, limit: u64) -> (out: Result<u64, String>)
    ensures match out {
        Ok(total) => previous as int + bytes as int <= u64::MAX
            && total == previous as int + bytes as int && total <= limit,
        Err(error) => if previous as int + bytes as int > u64::MAX {
            error@ == "snapshot byte accounting overflow"@
        } else {
            previous as int + bytes as int > limit
                && error@ == "snapshot output byte capacity exceeded before writing"@
        },
    },
{
    let total = match previous.checked_add(bytes) {
        None => { return Err("snapshot byte accounting overflow".to_owned()); },
        Some(total) => total,
    };
    if total > limit {
        Err("snapshot output byte capacity exceeded before writing".to_owned())
    } else { Ok(total) }
}

pub open spec fn counted<S>(total: u64, observed: (S, Result<(), String>),
    out: (S, Result<u64, String>)) -> bool
{
    out.0 == observed.0 && match observed.1 {
        Err(error) => out.1 == Err::<u64, String>(error),
        Ok(()) => out.1 == Ok::<u64, String>(total),
    }
}

fn finish_count<S>(total: u64, observed: (S, Result<(), String>))
    -> (out: (S, Result<u64, String>))
    ensures counted(total, observed, out),
{
    let (state, result) = observed;
    (state, match result { Err(error) => Err(error), Ok(()) => Ok(total) })
}

pub open spec fn regular_result<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, path: &P, bytes: Vec<u8>, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
    out: (S, Result<(), String>)) -> bool
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
{
    exists|written: (S, Result<(), String>)| io.write.ensures((state, path, bytes), written)
        && match written.1 {
            Err(_) => out == written,
            Ok(()) => exists|observed: (S, Result<M, String>)|
                io.metadata.ensures((written.0, path), observed) && match observed.1 {
                    Err(error) => out == (observed.0, Err::<(), String>(error)),
                    Ok(permissions) => exists|changed: (S, M)|
                        io.readonly.ensures((observed.0, permissions, true), changed)
                        && io.permissions.ensures((changed.0, path, changed.1), out),
                },
        }
}

pub fn regular<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, path: &P, bytes: Vec<u8>, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
) -> (out: (S, Result<(), String>))
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
    requires callable(io),
    ensures regular_result(state, path, bytes, io, out),
{
    let observed_write = (io.write)(state, path, bytes);
    let ghost written = observed_write;
    let (state, result) = observed_write;
    match result { Err(error) => { return (state, Err(error)); }, Ok(()) => {} }
    let observed_metadata = (io.metadata)(state, path);
    let ghost metadata = observed_metadata;
    let (state, result) = observed_metadata;
    let permissions = match result {
        Err(error) => { return (state, Err(error)); },
        Ok(permissions) => permissions,
    };
    let changed = (io.readonly)(state, permissions, true);
    (io.permissions)(changed.0, path, changed.1)
}

pub open spec fn symbolic_result<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, path: &P, bytes: Vec<u8>, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
    out: (S, Result<(), String>)) -> bool
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
{
    if !supported {
        out.0 == state && match out.1 {
            Err(error) => error@ == "E_INPUT_UNSUPPORTED: symlink snapshot"@,
            _ => false,
        }
    } else {
        exists|decoded: (S, Result<String, String>)| io.decode.ensures((state, bytes), decoded)
            && match decoded.1 {
                Err(error) => out == (decoded.0, Err::<(), String>(error)),
                Ok(text) => io.symlink.ensures((decoded.0, path, text), out),
            }
    }
}

pub fn symbolic<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, path: &P, bytes: Vec<u8>, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
) -> (out: (S, Result<(), String>))
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
    requires callable(io),
    ensures symbolic_result(state, path, bytes, supported, io, out),
{
    if !supported {
        return (state, Err("E_INPUT_UNSUPPORTED: symlink snapshot".to_owned()));
    }
    let observed = (io.decode)(state, bytes);
    let ghost decoded = observed;
    let (state, result) = observed;
    let text = match result { Err(error) => { return (state, Err(error)); }, Ok(text) => text };
    (io.symlink)(state, path, text)
}

pub open spec fn payload_result<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, path: &P, mode: &String, bytes: Vec<u8>, previous: u64,
    limit: u64, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>, out: (S, Result<u64, String>),
) -> bool
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
{
    if previous as int + (bytes.len() as u64) as int > u64::MAX {
        out.0 == state && match out.1 {
            Err(error) => error@ == "snapshot byte accounting overflow"@,
            _ => false,
        }
    } else if previous as int + (bytes.len() as u64) as int > limit {
        out.0 == state && match out.1 {
            Err(error) => error@ == "snapshot output byte capacity exceeded before writing"@,
            _ => false,
        }
    } else {
        exists|written: (S, Result<(), String>)|
            #[trigger] counted((previous + bytes.len() as u64) as u64, written, out)
            && if mode@ == "120000"@ {
                symbolic_result(state, path, bytes, supported, io, written)
            } else { regular_result(state, path, bytes, io, written) }
    }
}

pub fn payload<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, path: &P, mode: &String, bytes: Vec<u8>, previous: u64,
    limit: u64, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
) -> (out: (S, Result<u64, String>))
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
    requires callable(io),
    ensures payload_result(state, path, mode, bytes, previous, limit, supported, io, out),
{
    let total = match byte_total(previous, bytes.len() as u64, limit) {
        Err(error) => { return (state, Err(error)); },
        Ok(total) => total,
    };
    assert(total == (previous + bytes.len() as u64) as u64);
    if checkout_symbolic_link(mode.as_str()) {
        let observed = symbolic(state, path, bytes, supported, io);
        let out = finish_count(total, observed);
        assert(counted(total, observed, out));
        assert(symbolic_result(state, path, bytes, supported, io, observed));
        assert(payload_result(state, path, mode, bytes, previous, limit, supported, io, out));
        out
    } else {
        let observed = regular(state, path, bytes, io);
        let out = finish_count(total, observed);
        assert(counted(total, observed, out));
        assert(regular_result(state, path, bytes, io, observed));
        assert(payload_result(state, path, mode, bytes, previous, limit, supported, io, out));
        out
    }
}

pub open spec fn read_result<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, path: &P, relative: &String, mode: &String, previous: u64,
    limit: u64, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>, out: (S, Result<u64, String>),
) -> bool
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
{
    exists|read: (S, Result<Vec<u8>, String>)| io.read.ensures((state, relative), read)
        && match read.1 {
            Err(error) => out == (read.0, Err::<u64, String>(error)),
            Ok(bytes) => payload_result(read.0, path, mode, bytes, previous, limit, supported, io, out),
        }
}

pub fn read_file<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, path: &P, relative: &String, mode: &String, previous: u64,
    limit: u64, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
) -> (out: (S, Result<u64, String>))
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
    requires callable(io),
    ensures read_result(state, path, relative, mode, previous, limit, supported, io, out),
{
    let observed = (io.read)(state, relative);
    let ghost read = observed;
    let (state, result) = observed;
    let bytes = match result { Err(error) => { return (state, Err(error)); }, Ok(bytes) => bytes };
    payload(state, path, mode, bytes, previous, limit, supported, io)
}

pub open spec fn parent_result<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, path: &P, relative: &String, mode: &String, previous: u64,
    limit: u64, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>, out: (S, Result<u64, String>),
) -> bool
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
{
    exists|parent: (S, Option<P>)| io.parent.ensures((state, path), parent)
        && match parent.1 {
            None => out.0 == parent.0 && match out.1 {
                Err(error) => error@ == "snapshot parent"@, _ => false,
            },
            Some(directory) => exists|created: (S, Result<(), String>)|
                io.create_parent.ensures((parent.0, directory), created) && match created.1 {
                    Err(error) => out == (created.0, Err::<u64, String>(error)),
                    Ok(()) => read_result(created.0, path, relative, mode, previous, limit, supported, io, out),
                },
        }
}

pub fn parent_file<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, path: &P, relative: &String, mode: &String, previous: u64,
    limit: u64, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
) -> (out: (S, Result<u64, String>))
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
    requires callable(io),
    ensures parent_result(state, path, relative, mode, previous, limit, supported, io, out),
{
    let observed_parent = (io.parent)(state, path);
    let ghost parent = observed_parent;
    let (state, result) = observed_parent;
    let directory = match result {
        None => { return (state, Err("snapshot parent".to_owned())); }, Some(directory) => directory,
    };
    let observed_create = (io.create_parent)(state, directory);
    let ghost created = observed_create;
    let (state, result) = observed_create;
    match result { Err(error) => { return (state, Err(error)); }, Ok(()) => {} }
    read_file(state, path, relative, mode, previous, limit, supported, io)
}

pub open spec fn file_result<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, relative: &String, kind: &String, mode: &String, previous: u64,
    limit: u64, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>, out: (S, Result<u64, String>),
) -> bool
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
{
    if kind@ != "blob"@ { out == (state, Ok::<u64, String>(previous)) }
    else {
        exists|located: (S, Result<P, String>)| io.locate.ensures((state, relative), located)
            && match located.1 {
                Err(error) => out == (located.0, Err::<u64, String>(error)),
                Ok(path) => parent_result(located.0, &path, relative, mode, previous, limit, supported, io, out),
            }
    }
}

pub fn file<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    state: S, relative: &String, kind: &String, mode: &String, previous: u64,
    limit: u64, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
) -> (out: (S, Result<u64, String>))
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
    requires callable(io),
    ensures file_result(state, relative, kind, mode, previous, limit, supported, io, out),
{
    if !identifiers::equal_text(kind.as_str(), "blob") { return (state, Ok(previous)); }
    let observed = (io.locate)(state, relative);
    let ghost located = observed;
    let (state, result) = observed;
    let path = match result { Err(error) => { return (state, Err(error)); }, Ok(path) => path };
    parent_file(state, &path, relative, mode, previous, limit, supported, io)
}

pub open spec fn blob_paths(rows: Seq<(&TreePath, &TreeEntry)>, end: int) -> Seq<Seq<char>>
    recommends 0 <= end <= rows.len(),
    decreases end,
{
    if end <= 0 { Seq::empty() }
    else if rows[end - 1].1.kind@ == "blob"@ {
        blob_paths(rows, end - 1).push(rows[end - 1].0@)
    } else { blob_paths(rows, end - 1) }
}

pub fn prefetch_paths(rows: &Vec<(&TreePath, &TreeEntry)>) -> (paths: Vec<String>)
    ensures paths@.map(|i: int, path: String| path@) == blob_paths(rows@, rows.len() as int),
{
    let mut paths: Vec<String> = Vec::new();
    let mut i = 0;
    while i < rows.len()
        invariant 0 <= i <= rows.len(),
            paths@.map(|i: int, path: String| path@) == blob_paths(rows@, i as int),
        decreases rows.len() - i,
    {
        let row = &rows[i];
        if identifiers::equal_text(row.1.kind.as_str(), "blob") {
            paths.push(row.0.text.clone());
        }
        i += 1;
    }
    paths
}

pub open spec fn prefix<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    rows: Seq<(&TreePath, &TreeEntry)>, states: Seq<(S, u64)>, source: S,
    limit: u64, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
) -> bool
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
{
    0 < states.len() <= rows.len() + 1 && states[0] == (source, 0u64)
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] file_result(states[i].0, &rows[i].0.text,
                &rows[i].1.kind, &rows[i].1.mode, states[i].1,
                limit, supported, io, (states[i + 1].0, Ok::<u64, String>(states[i + 1].1)))
}

pub open spec fn files_result<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    rows: Seq<(&TreePath, &TreeEntry)>, source: S, limit: u64, supported: bool,
    io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>, out: (S, Result<(), String>),
) -> bool
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
{
    exists|states: Seq<(S, u64)>| prefix(rows, states, source, limit, supported, io)
        && match out.1 {
            Ok(()) => states.len() == rows.len() + 1 && out.0 == states.last().0,
            Err(error) => states.len() <= rows.len()
                && file_result(states.last().0, &rows[states.len() - 1].0.text,
                    &rows[states.len() - 1].1.kind, &rows[states.len() - 1].1.mode,
                    states.last().1, limit, supported, io, (out.0, Err::<u64, String>(error))),
        }
}

pub fn files<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    rows: &Vec<(&TreePath, &TreeEntry)>, source: S, limit: u64, supported: bool,
    io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
) -> (out: (S, Result<(), String>))
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
    requires callable(io),
    ensures files_result(rows@, source, limit, supported, io, out),
{
    let mut state = source;
    let mut total = 0u64;
    let mut i = 0;
    let ghost states = seq![(source, 0u64)];
    while i < rows.len()
        invariant 0 <= i <= rows.len(), callable(io),
            states.len() == i + 1, states.last() == (state, total),
            prefix(rows@, states, source, limit, supported, io),
        decreases rows.len() - i,
    {
        let row = &rows[i];
        let observed = file(state, &row.0.text, &row.1.kind,
            &row.1.mode, total, limit, supported, io);
        let (next, result) = observed;
        match result {
            Err(error) => {
                let out = (next, Err(error));
                assert(files_result(rows@, source, limit, supported, io, out));
                return out;
            },
            Ok(count) => {
                proof {
                    let previous_states = states;
                    states = states.push((next, count));
                    assert forall|j: int| 0 <= j < states.len() - 1 implies
                        #[trigger] file_result(states[j].0, &rows@[j].0.text,
                            &rows@[j].1.kind, &rows@[j].1.mode, states[j].1,
                            limit, supported, io, (states[j + 1].0, Ok::<u64, String>(states[j + 1].1))) by {
                        if j < previous_states.len() - 1 {
                            assert(file_result(previous_states[j].0, &rows@[j].0.text,
                                &rows@[j].1.kind, &rows@[j].1.mode, previous_states[j].1,
                                limit, supported, io, (previous_states[j + 1].0, Ok::<u64, String>(previous_states[j + 1].1))));
                        }
                    }
                }
                state = next;
                total = count;
            },
        }
        i += 1;
    }
    let out = (state, Ok(()));
    assert(files_result(rows@, source, limit, supported, io, out));
    out
}

pub open spec fn rows_result<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    rows: Seq<(&TreePath, &TreeEntry)>, source: S, reusable: bool, limit: u64, supported: bool,
    io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>, out: (S, Result<(), String>),
) -> bool
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
{
    if !reusable { files_result(rows, source, limit, supported, io, out) }
    else {
        exists|paths: Vec<String>, prefetched: (S, Result<(), String>)|
            paths@.map(|i: int, path: String| path@) == blob_paths(rows, rows.len() as int)
            && io.prefetch.ensures((source, &paths), prefetched)
            && match prefetched.1 {
                Err(_) => out == prefetched,
                Ok(()) => files_result(rows, prefetched.0, limit, supported, io, out),
            }
    }
}

/// 真实 BTreeMap 的全部键值及严格字节序，与逐项首错和每次 IO 观察相连。
pub fn snapshot<S, P, M, B, L, A, C, R, D, Y, W, G, O, F>(
    source: S, tree: &Tree, reusable: bool, limit: u64, supported: bool, io: &SnapshotExportIo<B, L, A, C, R, D, Y, W, G, O, F>,
) -> (out: (S, Result<(), String>))
    where B: Fn(S, &Vec<String>) -> (S, Result<(), String>),
    L: Fn(S, &String) -> (S, Result<P, String>),
    A: Fn(S, &P) -> (S, Option<P>),
    C: Fn(S, P) -> (S, Result<(), String>),
    R: Fn(S, &String) -> (S, Result<Vec<u8>, String>),
    D: Fn(S, Vec<u8>) -> (S, Result<String, String>),
    Y: Fn(S, &P, String) -> (S, Result<(), String>),
    W: Fn(S, &P, Vec<u8>) -> (S, Result<(), String>),
    G: Fn(S, &P) -> (S, Result<M, String>),
    O: Fn(S, M, bool) -> (S, M),
    F: Fn(S, &P, M) -> (S, Result<(), String>)
    requires callable(io),
    ensures exists|rows: Seq<(&TreePath, &TreeEntry)>|
        tree_rows_view(rows).to_set() == tree@.kv_pairs() && tree_rows_ordered(rows)
            && rows_result(rows, source, reusable, limit, supported, io, out),
{
    let rows = tree_rows(tree);
    let ghost original_rows = rows@;
    let state = if reusable {
        let paths = prefetch_paths(&rows);
        let observed = (io.prefetch)(source, &paths);
        let ghost prefetched = observed;
        let (state, result) = observed;
        match result {
            Err(error) => {
                let out = (state, Err(error));
                assert(rows_result(original_rows, source, reusable, limit, supported, io, out));
                return out;
            },
            Ok(()) => state,
        }
    } else { source };
    let out = files(&rows, state, limit, supported, io);
    assert(rows_result(original_rows, source, reusable, limit, supported, io, out));
    out
}

}

