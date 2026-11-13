//! 实际 CI／协议结果状态及 finding 一致性。Serde 适配仍由外部库生成。
use core::cmp::Ordering;
use std::collections::BTreeMap;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::cmp::OrdSpec;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

#[cfg(verus_keep_ghost)]
#[path = "verified_paths.rs"]
pub mod paths;
#[cfg(not(verus_keep_ghost))]
use crate::verified_paths as paths;

#[path = "verified_scope.rs"]
pub mod scopes;

#[path = "verified_export.rs"]
pub mod snapshot_export;

verus! {

/// Only the synchronous owner releases a local directory operation on return.
pub fn release_local_directory(local: bool, owner_pid: u32, current_pid: u32) -> (out: bool)
    ensures out == (local && owner_pid == current_pid),
{
    local && owner_pid == current_pid
}

pub open spec fn oid_text_result(value: Result<String, String>) -> Result<Seq<char>, Seq<char>> {
    match value { Ok(text) => Ok(text@), Err(error) => Err(error@) }
}

pub open spec fn oid_read_result<S>(value: (S, Result<String, String>))
    -> (S, Result<Seq<char>, Seq<char>>)
{
    (value.0, oid_text_result(value.1))
}

pub open spec fn oid_commit_matches<S>(observed: (S, Result<String, String>), oid: &str) -> bool {
    match observed.1 { Ok(text) => text@ == oid@, Err(_) => false }
}

pub open spec fn oid_tree_valid<S>(observed: (S, Result<String, String>)) -> bool {
    match observed.1 {
        Ok(text) => identifiers::oid(vstd::utf8::encode_utf8(text@)), Err(_) => false,
    }
}

pub open spec fn oid_tree_relation<S, R>(
    oid: &str, observed: (S, Result<String, String>), out: (S, Result<String, String>), record: R,
) -> bool where R: Fn(S, &str) -> S {
    if oid_tree_valid(observed) {
        out.1 == observed.1 && record.ensures((observed.0, oid), out.0)
    } else { out == observed }
}

pub open spec fn oid_commit_relation<S, T, R>(
    oid: &str, observed: (S, Result<String, String>), out: (S, Result<String, String>),
    read_tree: T, record: R,
) -> bool where T: Fn(S, &str) -> (S, Result<String, String>), R: Fn(S, &str) -> S {
    match observed.1 {
        Err(_) => out == observed,
        Ok(text) => if text@ != oid@ {
            oid_read_result(out) == (observed.0, Err::<Seq<char>, Seq<char>>("OID is not a commit"@))
        } else {
            exists|second: (S, Result<String, String>)|
                read_tree.ensures((observed.0, oid), second)
                    && oid_tree_relation(oid, second, out, record)
        },
    }
}

/// 关系保留每次实际观察，允许同一输入取得不同 IO 结果。
pub open spec fn verify_oid_relation<S, C, T, R>(
    state: S, oid: &str, out: (S, Result<String, String>), read_commit: C, read_tree: T, record: R,
) -> bool where C: Fn(S, &str) -> (S, Result<String, String>),
    T: Fn(S, &str) -> (S, Result<String, String>), R: Fn(S, &str) -> S {
    if !identifiers::oid(vstd::utf8::encode_utf8(oid@)) {
        oid_read_result(out) == (state,
            Err::<Seq<char>, Seq<char>>("expected full nonzero lowercase commit OID"@))
    } else {
        exists|first: (S, Result<String, String>)| read_commit.ensures((state, oid), first)
            && oid_commit_relation(oid, first, out, read_tree, record)
    }
}

fn finish_oid_tree<S, R>(oid: &str, observed: (S, Result<String, String>), record: R)
    -> (out: (S, Result<String, String>)) where R: Fn(S, &str) -> S,
    requires oid_tree_valid(observed) ==> record.requires((observed.0, oid)),
    ensures oid_tree_relation(oid, observed, out, record),
{
    let (state, result) = observed;
    let tree = match result { Err(error) => { return (state, Err(error)); }, Ok(text) => text };
    if identifiers::full_oid(tree.as_str()).is_ok() {
        (record(state, oid), Ok(tree))
    } else { (state, Ok(tree)) }
}

fn finish_oid_commit<S, T, R>(
    oid: &str, observed: (S, Result<String, String>), read_tree: T, record: R,
) -> (out: (S, Result<String, String>))
    where T: Fn(S, &str) -> (S, Result<String, String>), R: Fn(S, &str) -> S,
    requires oid_commit_matches(observed, oid) ==> read_tree.requires((observed.0, oid)),
        forall|second: (S, Result<String, String>)| oid_commit_matches(observed, oid)
            && read_tree.ensures((observed.0, oid), second) && oid_tree_valid(second)
            ==> record.requires((second.0, oid)),
    ensures oid_commit_relation(oid, observed, out, read_tree, record),
{
    let (state, result) = observed;
    let commit = match result { Err(error) => { return (state, Err(error)); }, Ok(text) => text };
    if !identifiers::equal_text(commit.as_str(), oid) {
        return (state, Err("OID is not a commit".to_owned()));
    }
    let second = read_tree(state, oid);
    finish_oid_tree(oid, second, record)
}

/// 只要求实际可达阶段可调用；不要求观察回调具有确定性。
pub fn verify_oid<S, C, T, R>(state: S, oid: &str, read_commit: C, read_tree: T, record: R)
    -> (out: (S, Result<String, String>))
    where C: Fn(S, &str) -> (S, Result<String, String>),
        T: Fn(S, &str) -> (S, Result<String, String>), R: Fn(S, &str) -> S,
    requires identifiers::oid(vstd::utf8::encode_utf8(oid@)) ==> read_commit.requires((state, oid)),
        forall|first: (S, Result<String, String>)| identifiers::oid(vstd::utf8::encode_utf8(oid@))
            && read_commit.ensures((state, oid), first) && oid_commit_matches(first, oid)
            ==> read_tree.requires((first.0, oid)),
        forall|first: (S, Result<String, String>), second: (S, Result<String, String>)|
            identifiers::oid(vstd::utf8::encode_utf8(oid@))
                && read_commit.ensures((state, oid), first) && oid_commit_matches(first, oid)
                && read_tree.ensures((first.0, oid), second) && oid_tree_valid(second)
                ==> record.requires((second.0, oid)),
    ensures verify_oid_relation(state, oid, out, read_commit, read_tree, record),
{
    match identifiers::full_oid(oid) {
        Err(error) => {
            assert(!identifiers::oid(oid.spec_bytes()));
            assert(oid.spec_bytes() == vstd::utf8::encode_utf8(oid@));
            assert(error@ == "expected full nonzero lowercase commit OID"@);
            let out = (state, Err(error));
            assert(oid_read_result(out) == (state,
                Err::<Seq<char>, Seq<char>>("expected full nonzero lowercase commit OID"@)));
            assert(verify_oid_relation(state, oid, out, read_commit, read_tree, record));
            return out;
        },
        Ok(()) => {},
    }
    let first = read_commit(state, oid);
    finish_oid_commit(oid, first, read_tree, record)
}

/// 无绑定时不查询；有绑定时只向原缓存接口转交同一个完整键。
pub fn can_reuse_oid<S, Q>(bound: bool, cache: S, oid: &str, contains: Q)
    -> (out: (S, bool))
    where Q: Fn(S, &str) -> (S, bool),
    requires bound ==> contains.requires((cache, oid)),
    ensures if bound { contains.ensures((cache, oid), out) } else { out == (cache, false) },
{
    if bound { contains(cache, oid) } else { (cache, false) }
}

/// 只有绑定读取器才登记已验证的原键；无绑定时保持原缓存状态。
pub fn record_verified_oid<S, I>(bound: bool, cache: S, oid: &str, insert: I)
    -> (out: S)
    where I: Fn(S, &str) -> S,
    requires bound ==> insert.requires((cache, oid)),
    ensures if bound { insert.ensures((cache, oid), out) } else { out == cache },
{
    if bound { insert(cache, oid) } else { cache }
}

/// 一次固定读取的完整原字节、解析值及入口／有效路径；JSON 值由实际使用方提供。
pub struct RegistrySnapshot<V> {
    pub bytes: BTreeMap<String, Vec<u8>>,
    pub values: BTreeMap<String, V>,
    pub entry_path: String,
    pub effective_path: String,
    pub selection: Option<V>,
}

/// 两条真实 registry 组装路径共用的终态：保留完整错误，成功时组合原身份与实际双 map。
pub fn registry_snapshot_complete<V>(
    entry_path: String, effective_path: String, selection: Option<V>,
    documents: Result<(BTreeMap<String, Vec<u8>>, BTreeMap<String, V>), String>,
) -> (out: Result<RegistrySnapshot<V>, String>)
    ensures match documents {
        Err(error) => out == Err::<RegistrySnapshot<V>, String>(error),
        Ok((bytes, values)) => match out {
            Err(_) => false,
            Ok(snapshot) => snapshot.bytes@ == bytes@ && snapshot.values@ == values@
                && snapshot.entry_path == entry_path
                && snapshot.effective_path == effective_path
                && snapshot.selection == selection,
        },
    },
{
    match documents {
        Err(error) => Err(error),
        Ok((bytes, values)) => Ok(RegistrySnapshot {
            bytes, values, entry_path, effective_path, selection,
        }),
    }
}

/// 同路径保留入口文档；不同路径额外保留目标文档，不合并两个文档的身份。
pub fn registry_snapshot_initial<V>(
    entry_path: String, original: Vec<u8>, config: V,
    effective_path: String, effective_bytes: Vec<u8>, effective_config: V,
    selection: Option<V>,
) -> (out: RegistrySnapshot<V>)
    requires vstd::laws_cmp::obeys_cmp::<String>(),
    ensures
        out.entry_path == entry_path,
        out.effective_path == effective_path,
        out.selection == selection,
        out.bytes@ == if entry_path@ == effective_path@ {
            Map::empty().insert(entry_path, original)
        } else {
            Map::empty().insert(entry_path, original).insert(effective_path, effective_bytes)
        },
        out.values@ == if entry_path@ == effective_path@ {
            Map::empty().insert(entry_path, config)
        } else {
            Map::empty().insert(entry_path, config).insert(effective_path, effective_config)
        },
{
    let mut bytes = BTreeMap::new();
    let mut values = BTreeMap::new();
    bytes.insert(entry_path.clone(), original);
    values.insert(entry_path.clone(), config);
    if !identifiers::equal_text(entry_path.as_str(), effective_path.as_str()) {
        bytes.insert(effective_path.clone(), effective_bytes);
        values.insert(effective_path.clone(), effective_config);
    }
    RegistrySnapshot { bytes, values, entry_path, effective_path, selection }
}



/// 保留非对象；对象先删除原字节摘要，再写入结构化 registry selection schema。
pub fn normalize_registry_selection_object<M, R, I>(object: Option<M>, remove: R, insert: I)
    -> (out: Option<M>)
    where R: Fn(M, &str) -> M, I: Fn(M, &str, &str) -> M,
    requires match object {
        None => true,
        Some(initial) =>
            (forall|key: &str| key@ == "sha256"@ ==> remove.requires((initial, key)))
            && (forall|key: &str, middle: M, field: &str, value: &str|
                key@ == "sha256"@ && remove.ensures((initial, key), middle)
                && field@ == "schema"@ && value@ == "chrono-registry-selection/v1"@
                ==> insert.requires((middle, field, value))),
    },
    ensures match (object, out) {
        (None, None) => true,
        (Some(initial), Some(normalized)) =>
            exists|middle: M, removed: &str, field: &str, value: &str|
                removed@ == "sha256"@ && field@ == "schema"@
                && value@ == "chrono-registry-selection/v1"@
                && remove.ensures((initial, removed), middle)
                && insert.ensures((middle, field, value), normalized),
        _ => false,
    },
{
    match object {
        None => None,
        Some(initial) => {
            let removed = "sha256";
            let field = "schema";
            let value = "chrono-registry-selection/v1";
            let middle = remove(initial, removed);
            let normalized = insert(middle, field, value);
            proof {
                assert(object == Some(initial));
                assert(remove.ensures((initial, removed), middle));
                assert(insert.ensures((middle, field, value), normalized));
                assert(exists|observed: M, key: &str, target: &str, schema: &str|
                    key@ == "sha256"@ && target@ == "schema"@
                    && schema@ == "chrono-registry-selection/v1"@
                    && remove.ensures((initial, key), observed)
                    && insert.ensures((observed, target, schema), normalized));
            }
            let result = Some(normalized);
            proof {
                assert(result->Some_0 == normalized);
                assert(exists|observed: M, key: &str, target: &str, schema: &str|
                    key@ == "sha256"@ && target@ == "schema"@
                    && schema@ == "chrono-registry-selection/v1"@
                    && remove.ensures((object->Some_0, key), observed)
                    && insert.ensures((observed, target, schema), result->Some_0));
            }
            result
        },
    }
}

/// 标准 JSON 解码仅提供四个固定字段的原字符串观察。
pub enum RegistryPathError<'a> {
    Missing { message: &'static str },
    Invalid { path: &'a str },
}

impl<'a> RegistryPathError<'a> {
    /// 标准 Debug 格式化为外部接口；错误类别和完整原路径由本模块选择。
    pub fn render<F: Fn(&str) -> String>(self, format_invalid: F) -> (out: String)
        requires forall|path: &str| format_invalid.requires((path,)),
        ensures match self {
            Self::Missing { message } => out@ == message@,
            Self::Invalid { path } => format_invalid.ensures((path,), out),
        },
    {
        match self {
            Self::Missing { message } => message.to_owned(),
            Self::Invalid { path } => format_invalid(path),
        }
    }
}

pub fn admit_relative_path(value: &str) -> (out: Result<(), &str>)
    ensures match out {
        Ok(()) => paths::relative_path(value.spec_bytes()),
        Err(invalid) => invalid@ == value@ && !paths::relative_path(value.spec_bytes()),
    },
{
    if paths::valid_relative_path(value.as_bytes()) { Ok(()) }
    else { Err(value) }
}

pub open spec fn registry_path_valid(row: Option<&str>) -> bool {
    match row { Some(path) => paths::relative_path(path.spec_bytes()), None => false }
}

pub open spec fn registry_missing_message(index: int) -> Seq<char> {
    if index == 0 { "missing registry path judges"@ }
    else if index == 1 { "missing registry path projects"@ }
    else if index == 2 { "missing registry path filemap"@ }
    else { "missing registry path workflow"@ }
}

fn missing_registry_message(index: usize) -> (message: &'static str)
    requires index < 4,
    ensures message@ == registry_missing_message(index as int),
{
    if index == 0 { "missing registry path judges" }
    else if index == 1 { "missing registry path projects" }
    else if index == 2 { "missing registry path filemap" }
    else { "missing registry path workflow" }
}

pub open spec fn registry_error_at(error: &RegistryPathError, index: int, row: Option<&str>) -> bool {
    match error {
        RegistryPathError::Missing { message } => row.is_none()
            && message@ == registry_missing_message(index),
        RegistryPathError::Invalid { path } => match row {
            Some(original) => path@ == original@ && !paths::relative_path(original.spec_bytes()),
            None => false,
        },
    }
}

/// 入口原样在前；依次收取 judges、projects、filemap、workflow，严格保留首错。
pub fn collect_registry_paths<'a>(entry: &str, rows: &[Option<&'a str>; 4])
    -> (out: Result<Vec<String>, RegistryPathError<'a>>)
    ensures match out {
        Ok(paths) => paths.len() == 5 && paths@[0]@ == entry@
            && forall|i: int| 0 <= i < 4 ==> registry_path_valid(rows@[i])
                && match rows@[i] { Some(path) => paths@[i + 1]@ == path@, None => false },
        Err(error) => exists|i: int| 0 <= i < 4
            && (forall|j: int| 0 <= j < i ==> registry_path_valid(rows@[j]))
            && registry_error_at(&error, i, rows@[i]),
    },
{
    let mut paths = vec![entry.to_owned()];
    let mut i = 0usize;
    while i < 4
        invariant i <= 4, paths.len() == i + 1, paths@[0]@ == entry@,
            forall|j: int| 0 <= j < i ==> registry_path_valid(rows@[j])
                && match rows@[j] { Some(path) => paths@[j + 1]@ == path@, None => false },
        decreases 4 - i,
    {
        let path = match rows[i] {
            Some(path) => path,
            None => {
                let error = RegistryPathError::Missing { message: missing_registry_message(i) };
                assert(registry_error_at(&error, i as int, rows@[i as int]));
                return Err(error);
            },
        };
        match admit_relative_path(path) {
            Ok(()) => {},
            Err(invalid) => {
                let error = RegistryPathError::Invalid { path: invalid };
                assert(registry_error_at(&error, i as int, rows@[i as int]));
                return Err(error);
            },
        }
        paths.push(path.to_owned());
        i += 1;
    }
    Ok(paths)
}

/// 按前缀定义完整待读序列：排除已读路径，每个其余路径只保留首次出现。
pub open spec fn registry_pending_prefix(
    paths: Seq<Seq<char>>, count: int, entry: Seq<char>, effective: Seq<char>,
) -> Seq<Seq<char>>
    decreases count,
{
    if count <= 0 {
        Seq::empty()
    } else {
        let previous = registry_pending_prefix(paths, count - 1, entry, effective);
        let path = paths[count - 1];
        if path == entry || path == effective || previous.contains(path) { previous }
        else { previous.push(path) }
    }
}

fn pending_contains(paths: &Vec<String>, path: &str) -> (found: bool)
    ensures found == paths@.map_values(|p: String| p@).contains(path@),
{
    let mut i = 0usize;
    while i < paths.len()
        invariant
            i <= paths.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] paths@[j])@ != path@,
        decreases paths.len() - i,
    {
        if identifiers::equal_text(paths[i].as_str(), path) {
            assert(paths@.map_values(|p: String| p@)[i as int] == path@);
            return true;
        }
        i += 1;
    }
    let ghost texts = paths@.map_values(|p: String| p@);
    assert forall|j: int| 0 <= j < texts.len() implies (#[trigger] texts[j]) != path@ by {
        assert(texts[j] == paths@[j]@);
    }
    false
}

/// 按值移交首次出现的待读路径，不复制文本或改写路径。
pub fn pending_registry_paths(paths: Vec<String>, entry: &str, effective: &str) -> (out: Vec<String>)
    ensures out@.map_values(|p: String| p@)
        == registry_pending_prefix(paths@.map_values(|p: String| p@), paths@.len() as int, entry@, effective@),
{
    let ghost all = paths@;
    let ghost text = all.map_values(|p: String| p@);
    let mut iterator = paths.into_iter();
    let mut pending: Vec<String> = Vec::new();
    loop
        invariant
            all == paths@, text == all.map_values(|p: String| p@),
            iterator@.1 == all, 0 <= iterator@.0 <= all.len(),
            pending@.map_values(|p: String| p@)
                == registry_pending_prefix(text, iterator@.0, entry@, effective@),
        decreases all.len() - iterator@.0,
    {
        let ghost at = iterator@.0;
        match iterator.next() {
            None => { return pending; },
            Some(path) => {
                assert(text[at] == path@);
                if !identifiers::equal_text(path.as_str(), entry)
                    && !identifiers::equal_text(path.as_str(), effective)
                    && !pending_contains(&pending, path.as_str())
                {
                    let ghost previous = pending@.map_values(|p: String| p@);
                    let ghost original = path@;
                    pending.push(path);
                    assert(pending@.map_values(|p: String| p@) =~= previous.push(original));
                }
            },
        }
    }
}

/// 只核本次请求的数量，不推断路径、内容或解码结果。
pub fn registry_acquisition_count(expected: usize, observed: usize) -> (out: Result<(), String>)
    ensures match out {
        Ok(()) => expected == observed,
        Err(message) => expected != observed && message@ == "registry acquisition count mismatch"@,
    },
{
    if expected == observed { Ok(()) }
    else { Err("registry acquisition count mismatch".to_owned()) }
}

/// 返回项须精确对应当前请求；不做归一化或额外路径准入。
pub fn registry_acquisition_path(expected: &str, observed: &str) -> (out: Result<(), String>)
    ensures match out {
        Ok(()) => expected@ == observed@,
        Err(message) => expected@ != observed@ && message@ == "registry acquisition path mismatch"@,
    },
{
    if identifiers::equal_text(expected, observed) { Ok(()) }
    else { Err("registry acquisition path mismatch".to_owned()) }
}

/// 单条读取必须恰好返回所请求的一项；成功按值交出该项完整原字节。
pub fn single_registry_blob(expected: &str, mut rows: Vec<(String, Vec<u8>)>)
    -> (out: Result<Vec<u8>, String>)
    ensures match out {
        Ok(bytes) => rows.len() == 1 && rows@[0].0@ == expected@ && bytes@ == rows@[0].1@,
        Err(message) => (rows.len() != 1 || rows@[0].0@ != expected@)
            && message@ == "registry acquisition path mismatch"@,
    },
{
    if rows.len() != 1 { return Err("registry acquisition path mismatch".to_owned()); }
    let (path, bytes) = rows.remove(0);
    match registry_acquisition_path(expected, path.as_str()) {
        Ok(()) => Ok(bytes),
        Err(message) => Err(message),
    }
}

pub open spec fn registry_decoded_view<V>(decoded: Result<V, String>) -> Result<V, Seq<char>> {
    match decoded { Ok(value) => Ok(value), Err(message) => Err(message@) }
}

pub open spec fn registry_batch_view<V>(
    result: Result<(BTreeMap<String, Vec<u8>>, BTreeMap<String, V>), String>,
) -> Result<(Map<String, Vec<u8>>, Map<String, V>), Seq<char>> {
    match result { Ok((bytes, values)) => Ok((bytes@, values@)), Err(message) => Err(message@) }
}

/// 原顺序中此前所有路径与解析都已成功，才会到达当前项。
pub open spec fn registry_batch_prefix<V>(
    expected: Seq<String>, rows: Seq<(String, Vec<u8>)>,
    model: spec_fn(Seq<u8>) -> Result<V, Seq<char>>, end: int,
) -> bool {
    0 <= end <= rows.len() && expected.len() == rows.len()
        && forall|i: int| 0 <= i < end ==> (expected[i]@ == rows[i].0@
            && (#[trigger] model(rows[i].1@)).is_ok())
}

/// 双 map 只在当前路径和解析均成功后一起更新，重复键采用最后一项。
pub open spec fn registry_batch_fold<V>(
    expected: Seq<String>, rows: Seq<(String, Vec<u8>)>, at: int,
    bytes: Map<String, Vec<u8>>, values: Map<String, V>,
    model: spec_fn(Seq<u8>) -> Result<V, Seq<char>>,
) -> Result<(Map<String, Vec<u8>>, Map<String, V>), Seq<char>>
    recommends expected.len() == rows.len(), 0 <= at <= rows.len(),
    decreases rows.len() - at,
{
    if at >= rows.len() { Ok((bytes, values)) }
    else if expected[at]@ != rows[at].0@ { Err("registry acquisition path mismatch"@) }
    else {
        match model(rows[at].1@) {
            Err(message) => Err(message),
            Ok(value) => registry_batch_fold(expected, rows, at + 1,
                bytes.insert(rows[at].0, rows[at].1), values.insert(rows[at].0, value), model),
        }
    }
}

pub open spec fn registry_batch_spec<V>(
    expected: Seq<String>, rows: Seq<(String, Vec<u8>)>,
    bytes: Map<String, Vec<u8>>, values: Map<String, V>,
    model: spec_fn(Seq<u8>) -> Result<V, Seq<char>>,
) -> Result<(Map<String, Vec<u8>>, Map<String, V>), Seq<char>> {
    if expected.len() != rows.len() { Err("registry acquisition count mismatch"@) }
    else { registry_batch_fold(expected, rows, 0, bytes, values, model) }
}

/// 固定批量观察的完整校验、首错及双 map 组装；失败不发布部分状态。
pub fn registry_parse_batch<V, P>(
    expected: &[String], originals: Vec<(String, Vec<u8>)>,
    bytes: BTreeMap<String, Vec<u8>>, values: BTreeMap<String, V>, decode: P,
    #[cfg(verus_keep_ghost)] Ghost(model): Ghost<spec_fn(Seq<u8>) -> Result<V, Seq<char>>>,
) -> (out: Result<(BTreeMap<String, Vec<u8>>, BTreeMap<String, V>), String>)
    where P: Fn(&[u8]) -> Result<V, String>,
    requires
        vstd::laws_cmp::obeys_cmp::<String>(),
        forall|i: int, raw: &[u8]| 0 <= i < originals.len()
            && registry_batch_prefix(expected@, originals@, model, i)
            && expected@[i]@ == originals@[i].0@ && raw@ == originals@[i].1@
            ==> decode.requires((raw,)),
        forall|raw: &[u8], result: Result<V, String>|
            decode.requires((raw,)) && decode.ensures((raw,), result)
            ==> registry_decoded_view(result) == model(raw@),
    ensures registry_batch_view(out)
        == registry_batch_spec(expected@, originals@, bytes@, values@, model),
{
    let ghost all = originals@;
    let ghost initial_bytes = bytes@;
    let ghost initial_values = values@;
    let mut collected_bytes = bytes;
    let mut collected_values = values;
    match registry_acquisition_count(expected.len(), originals.len()) {
        Err(error) => { return Err(error); },
        Ok(()) => {},
    }
    let mut remaining = originals.into_iter();
    let mut index = 0usize;
    loop
        invariant
            vstd::laws_cmp::obeys_cmp::<String>(),
            all == originals@, all.len() == expected.len(),
            initial_bytes == bytes@, initial_values == values@,
            remaining@.1 == all, 0 <= remaining@.0 <= all.len(), index as int == remaining@.0,
            registry_batch_prefix(expected@, all, model, remaining@.0),
            registry_batch_spec(expected@, all, initial_bytes, initial_values, model)
                == registry_batch_fold(expected@, all, remaining@.0, collected_bytes@, collected_values@, model),
            forall|i: int, raw: &[u8]| 0 <= i < all.len()
                && registry_batch_prefix(expected@, all, model, i)
                && expected@[i]@ == all[i].0@ && raw@ == all[i].1@
                ==> decode.requires((raw,)),
            forall|raw: &[u8], result: Result<V, String>|
                decode.requires((raw,)) && decode.ensures((raw,), result)
                ==> registry_decoded_view(result) == model(raw@),
        decreases all.len() - remaining@.0,
    {
        let ghost at = remaining@.0;
        proof { reveal_with_fuel(registry_batch_fold, 1); }
        let (path, original) = match remaining.next() {
            None => { return Ok((collected_bytes, collected_values)); },
            Some(row) => row,
        };
        let ghost before_bytes = collected_bytes@;
        let ghost before_values = collected_values@;
        match registry_acquisition_path(expected[index].as_str(), path.as_str()) {
            Err(error) => { return Err(error); },
            Ok(()) => {},
        }
        let raw = original.as_slice();
        assert(decode.requires((raw,)));
        let decoded = decode(raw);
        assert(registry_decoded_view(decoded) == model(all[at].1@));
        let value = match decoded {
            Err(error) => { return Err(error); },
            Ok(value) => value,
        };
        collected_bytes.insert(path.clone(), original);
        collected_values.insert(path, value);
        assert(registry_batch_fold(expected@, all, at, before_bytes, before_values, model)
            == registry_batch_fold(expected@, all, at + 1, collected_bytes@, collected_values@, model));
        index += 1;
        assert(registry_batch_prefix(expected@, all, model, at + 1)) by {
            assert forall|i: int| 0 <= i < at + 1 implies
                expected@[i]@ == all[i].0@ && (#[trigger] model(all[i].1@)).is_ok() by {
                if i < at { assert(registry_batch_prefix(expected@, all, model, at)); }
            }
        }
    }
}

pub open spec fn registry_read_sequence_step<S, V, R, P>(
    before: (S, Map<String, Vec<u8>>, Map<String, V>),
    after: (S, Map<String, Vec<u8>>, Map<String, V>), path: String, read: R, decode: P,
) -> bool
    where R: Fn(S, &str) -> (S, Result<Vec<u8>, String>), P: Fn(&[u8]) -> Result<V, String>,
{
    exists|argument: &str, raw: Vec<u8>, input: &[u8], value: V|
        argument@ == path@ && input@ == raw@
        && read.ensures((before.0, argument), (after.0, Ok::<Vec<u8>, String>(raw)))
        && decode.ensures((input,), Ok::<V, String>(value))
        && after.1 == before.1.insert(path, raw)
        && after.2 == before.2.insert(path, value)
}

pub open spec fn registry_read_sequence_prefix<S, V, R, P>(
    states: Seq<(S, Map<String, Vec<u8>>, Map<String, V>)>,
    initial: (S, Map<String, Vec<u8>>, Map<String, V>), paths: Seq<String>, read: R, decode: P,
) -> bool
    where R: Fn(S, &str) -> (S, Result<Vec<u8>, String>), P: Fn(&[u8]) -> Result<V, String>,
{
    1 <= states.len() <= paths.len() + 1 && states[0] == initial
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] registry_read_sequence_step(states[i], states[i + 1], paths[i], read, decode)
}

pub open spec fn registry_read_sequence_terminal<S, V, R, P>(
    before: (S, Map<String, Vec<u8>>, Map<String, V>), at: int, paths: Seq<String>,
    read: R, decode: P,
    out: (S, Result<(BTreeMap<String, Vec<u8>>, BTreeMap<String, V>), String>),
) -> bool
    where R: Fn(S, &str) -> (S, Result<Vec<u8>, String>), P: Fn(&[u8]) -> Result<V, String>,
{
    match out.1 {
        Ok((bytes, values)) => at == paths.len() && out.0 == before.0
            && bytes@ == before.1 && values@ == before.2,
        Err(error) => 0 <= at < paths.len()
            && exists|argument: &str, observation: (S, Result<Vec<u8>, String>)|
                argument@ == paths[at]@
                && read.ensures((before.0, argument), observation)
                && out.0 == observation.0
                && match observation.1 {
                    Err(original) => error@ == original@,
                    Ok(raw) => exists|input: &[u8]| input@ == raw@
                        && decode.ensures((input,), Err::<V, String>(error)),
                },
    }
}

pub open spec fn registry_read_sequence_result<S, V, R, P>(
    initial: (S, Map<String, Vec<u8>>, Map<String, V>), paths: Seq<String>, read: R, decode: P,
    out: (S, Result<(BTreeMap<String, Vec<u8>>, BTreeMap<String, V>), String>),
) -> bool
    where R: Fn(S, &str) -> (S, Result<Vec<u8>, String>), P: Fn(&[u8]) -> Result<V, String>,
{
    exists|states: Seq<(S, Map<String, Vec<u8>>, Map<String, V>)>|
        registry_read_sequence_prefix(states, initial, paths, read, decode)
        && registry_read_sequence_terminal(states.last(), states.len() - 1, paths, read, decode, out)
}

/// 逐条读取后立即解析；首错终止，不预读后续项或发布部分 map。
pub fn registry_read_sequence<S, V, R, P>(
    paths: Vec<String>, input_source: S,
    bytes: BTreeMap<String, Vec<u8>>, values: BTreeMap<String, V>, read: R, decode: P,
) -> (out: (S, Result<(BTreeMap<String, Vec<u8>>, BTreeMap<String, V>), String>))
    where R: Fn(S, &str) -> (S, Result<Vec<u8>, String>), P: Fn(&[u8]) -> Result<V, String>,
    requires vstd::laws_cmp::obeys_cmp::<String>(),
        paths.len() > 0 ==> (forall|source: S, path: &str| #[trigger] read.requires((source, path))),
        paths.len() > 0 ==> (forall|raw: &[u8]| #[trigger] decode.requires((raw,))),
    ensures registry_read_sequence_result((input_source, bytes@, values@), paths@, read, decode, out),
{
    let ghost all = paths@;
    let ghost initial = (input_source, bytes@, values@);
    let ghost states = seq![initial];
    let mut source = input_source;
    let mut collected_bytes = bytes;
    let mut collected_values = values;
    let mut remaining = paths.into_iter();
    loop
        invariant vstd::laws_cmp::obeys_cmp::<String>(),
            all == paths@, initial == (input_source, bytes@, values@),
            remaining@.1 == all, 0 <= remaining@.0 <= all.len(),
            states.len() == remaining@.0 + 1,
            states.last() == (source, collected_bytes@, collected_values@),
            registry_read_sequence_prefix(states, initial, all, read, decode),
            all.len() > 0 ==> (forall|source: S, path: &str| #[trigger] read.requires((source, path))),
            all.len() > 0 ==> (forall|raw: &[u8]| #[trigger] decode.requires((raw,))),
        decreases all.len() - remaining@.0,
    {
        let ghost at = remaining@.0;
        let ghost before = states.last();
        let path = match remaining.next() {
            None => {
                let out = (source, Ok((collected_bytes, collected_values)));
                assert(registry_read_sequence_terminal(before, at, all, read, decode, out));
                assert(registry_read_sequence_result(initial, all, read, decode, out));
                return out;
            },
            Some(path) => path,
        };
        let argument = path.as_str();
        let observation = read(source, argument);
        let ghost observed = observation;
        assert(read.ensures((before.0, argument), observed));
        let (next_source, result) = observation;
        source = next_source;
        let original = match result {
            Err(error) => {
                let out = (source, Err(error));
                assert(registry_read_sequence_terminal(before, at, all, read, decode, out));
                assert(registry_read_sequence_result(initial, all, read, decode, out));
                return out;
            },
            Ok(original) => original,
        };
        let input = original.as_slice();
        let decoded = decode(input);
        assert(decode.ensures((input,), decoded));
        let value = match decoded {
            Err(error) => {
                let out = (source, Err(error));
                assert(registry_read_sequence_terminal(before, at, all, read, decode, out));
                assert(registry_read_sequence_result(initial, all, read, decode, out));
                return out;
            },
            Ok(value) => value,
        };
        let ghost key = path;
        let ghost raw = original;
        let ghost parsed = value;
        collected_bytes.insert(path.clone(), original);
        collected_values.insert(path, value);
        let ghost after = (source, collected_bytes@, collected_values@);
        assert(read.ensures((before.0, argument), (after.0, Ok::<Vec<u8>, String>(raw))));
        assert(decode.ensures((input,), Ok::<V, String>(parsed)));
        assert(registry_read_sequence_step(before, after, key, read, decode)) by {
            reveal(registry_read_sequence_step);
        }
        proof {
            let previous = states;
            states = states.push(after);
            assert forall|i: int| 0 <= i < states.len() - 1 implies
                #[trigger] registry_read_sequence_step(states[i], states[i + 1], all[i], read, decode) by {
                if i < previous.len() - 1 {
                    assert(registry_read_sequence_step(previous[i], previous[i + 1], all[i], read, decode));
                }
            }
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct IndexFlag {
    pub path: String,
    pub tag: String,
}

impl Clone for IndexFlag {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    { Self { path: self.path.clone(), tag: self.tag.clone() } }
}

pub open spec fn index_flags_equal(a: &IndexFlag, b: &IndexFlag) -> bool {
    a.path@ == b.path@ && a.tag@ == b.tag@
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for IndexFlag {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { index_flags_equal(self, other) }
}

impl PartialEq for IndexFlag {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == index_flags_equal(self, other),
    {
        identifiers::equal_text(self.path.as_str(), other.path.as_str())
            && identifiers::equal_text(self.tag.as_str(), other.tag.as_str())
    }
}
impl Eq for IndexFlag {}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Checkout {
    pub head: String,
    pub r#tracked: Vec<String>,
    pub untracked: Vec<String>,
    pub index_flags: Vec<IndexFlag>,
}

impl Clone for Checkout {
    fn clone(&self) -> (out: Self)
        ensures out.head == self.head, out.r#tracked@ == self.r#tracked@,
            out.untracked@ == self.untracked@, out.index_flags@ == self.index_flags@,
    {
        Self { head: self.head.clone(), r#tracked: self.r#tracked.clone(),
            untracked: self.untracked.clone(), index_flags: self.index_flags.clone() }
    }
}

pub open spec fn checkout_texts_equal(a: Seq<String>, b: Seq<String>) -> bool {
    a.len() == b.len() && (forall|i: int| 0 <= i < a.len() ==> (#[trigger] a[i])@ == b[i]@)
}

fn equal_checkout_texts(a: &[String], b: &[String]) -> (equal: bool)
    ensures equal == checkout_texts_equal(a@, b@),
{
    if a.len() != b.len() { return false; }
    let mut i = 0;
    while i < a.len()
        invariant a.len() == b.len(), 0 <= i <= a.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] a[j])@ == b[j]@,
        decreases a.len() - i,
    {
        if !identifiers::equal_text(a[i].as_str(), b[i].as_str()) { return false; }
        i += 1;
    }
    true
}

pub open spec fn checkout_equal(a: &Checkout, b: &Checkout) -> bool {
    a.head@ == b.head@ && checkout_texts_equal(a.r#tracked@, b.r#tracked@)
        && checkout_texts_equal(a.untracked@, b.untracked@)
        && a.index_flags.len() == b.index_flags.len()
        && (forall|i: int| 0 <= i < a.index_flags.len() ==>
            #[trigger] index_flags_equal(&a.index_flags[i], &b.index_flags[i]))
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Checkout {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { checkout_equal(self, other) }
}

impl PartialEq for Checkout {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == checkout_equal(self, other),
    {
        identifiers::equal_text(self.head.as_str(), other.head.as_str())
            && equal_checkout_texts(&self.r#tracked, &other.r#tracked)
            && equal_checkout_texts(&self.untracked, &other.untracked)
            && self.index_flags == other.index_flags
    }

    fn ne(&self, other: &Self) -> (different: bool)
        ensures different == !checkout_equal(self, other),
    {
        !self.eq(other)
    }
}
impl Eq for Checkout {}

pub open spec fn checkout_flags_result<S>(
    head: String, r#tracked: Vec<String>, untracked: Vec<String>,
    observed: (S, Result<Vec<IndexFlag>, String>), out: (S, Result<Checkout, String>),
) -> bool {
    out.0 == observed.0 && match observed.1 {
        Err(error) => out.1 == Err::<Checkout, String>(error),
        Ok(index_flags) => out.1 == Ok::<Checkout, String>(Checkout { head, r#tracked, untracked, index_flags }),
    }
}

pub open spec fn checkout_untracked_result<S, F>(
    head: String, r#tracked: Vec<String>, observed: (S, Result<Vec<String>, String>),
    flags: F, out: (S, Result<Checkout, String>),
) -> bool where F: Fn(S) -> (S, Result<Vec<IndexFlag>, String>) {
    match observed.1 {
        Err(error) => out == (observed.0, Err::<Checkout, String>(error)),
        Ok(untracked) => exists|last: (S, Result<Vec<IndexFlag>, String>)|
            flags.ensures((observed.0,), last)
                && checkout_flags_result(head, r#tracked, untracked, last, out),
    }
}

pub open spec fn checkout_tracked_result<S, U, F>(
    head: String, observed: (S, Result<Vec<String>, String>), untracked: U, flags: F,
    out: (S, Result<Checkout, String>),
) -> bool where U: Fn(S) -> (S, Result<Vec<String>, String>),
    F: Fn(S) -> (S, Result<Vec<IndexFlag>, String>) {
    match observed.1 {
        Err(error) => out == (observed.0, Err::<Checkout, String>(error)),
        Ok(r#tracked) => exists|next: (S, Result<Vec<String>, String>)|
            untracked.ensures((observed.0,), next)
                && checkout_untracked_result(head, r#tracked, next, flags, out),
    }
}

pub open spec fn checkout_head_result<S, T, U, F>(
    observed: (S, Result<String, String>), r#tracked: T, untracked: U, flags: F,
    out: (S, Result<Checkout, String>),
) -> bool where T: Fn(S) -> (S, Result<Vec<String>, String>),
    U: Fn(S) -> (S, Result<Vec<String>, String>), F: Fn(S) -> (S, Result<Vec<IndexFlag>, String>) {
    match observed.1 {
        Err(error) => out == (observed.0, Err::<Checkout, String>(error)),
        Ok(head) => exists|next: (S, Result<Vec<String>, String>)|
            r#tracked.ensures((observed.0,), next)
                && checkout_tracked_result(head, next, untracked, flags, out),
    }
}

fn finish_checkout_flags<S>(head: String, changed: Vec<String>, untracked: Vec<String>,
    observed: (S, Result<Vec<IndexFlag>, String>)) -> (out: (S, Result<Checkout, String>))
    ensures checkout_flags_result(head, changed, untracked, observed, out),
{
    let (state, result) = observed;
    match result {
        Err(error) => (state, Err(error)),
        Ok(index_flags) => (state, Ok(Checkout { head, r#tracked: changed, untracked, index_flags })),
    }
}

fn finish_checkout_untracked<S, F>(head: String, changed: Vec<String>,
    observed: (S, Result<Vec<String>, String>), flags: F) -> (out: (S, Result<Checkout, String>))
    where F: Fn(S) -> (S, Result<Vec<IndexFlag>, String>),
    requires forall|s: S| #[trigger] flags.requires((s,)),
    ensures checkout_untracked_result(head, changed, observed, flags, out),
{
    let (state, result) = observed;
    let untracked = match result { Err(error) => { return (state, Err(error)); }, Ok(value) => value };
    let last = flags(state);
    finish_checkout_flags(head, changed, untracked, last)
}

fn finish_checkout_tracked<S, U, F>(head: String,
    observed: (S, Result<Vec<String>, String>), untracked: U, flags: F)
    -> (out: (S, Result<Checkout, String>))
    where U: Fn(S) -> (S, Result<Vec<String>, String>), F: Fn(S) -> (S, Result<Vec<IndexFlag>, String>),
    requires forall|s: S| #[trigger] untracked.requires((s,)),
        forall|s: S| #[trigger] flags.requires((s,)),
    ensures checkout_tracked_result(head, observed, untracked, flags, out),
{
    let (state, result) = observed;
    let changed = match result { Err(error) => { return (state, Err(error)); }, Ok(value) => value };
    let next = untracked(state);
    finish_checkout_untracked(head, changed, next, flags)
}

fn finish_checkout_head<S, T, U, F>(observed: (S, Result<String, String>), changed: T,
    untracked: U, flags: F) -> (out: (S, Result<Checkout, String>))
    where T: Fn(S) -> (S, Result<Vec<String>, String>),
        U: Fn(S) -> (S, Result<Vec<String>, String>), F: Fn(S) -> (S, Result<Vec<IndexFlag>, String>),
    requires forall|s: S| #[trigger] changed.requires((s,)),
        forall|s: S| #[trigger] untracked.requires((s,)),
        forall|s: S| #[trigger] flags.requires((s,)),
    ensures checkout_head_result(observed, changed, untracked, flags, out),
{
    let (state, result) = observed;
    let head = match result { Err(error) => { return (state, Err(error)); }, Ok(value) => value };
    let next = changed(state);
    finish_checkout_tracked(head, next, untracked, flags)
}

/// 按 head、tracked、untracked、index flags 的实际观察顺序组装同一个状态。
/// 回调可以返回不同 IO 观察；首个错误保留完整原值，不启动之后的阶段。
pub fn collect_checkout<S, H, T, U, F>(source: S, head: H, r#tracked: T, untracked: U, flags: F)
    -> (out: (S, Result<Checkout, String>))
    where H: Fn(S) -> (S, Result<String, String>),
        T: Fn(S) -> (S, Result<Vec<String>, String>),
        U: Fn(S) -> (S, Result<Vec<String>, String>),
        F: Fn(S) -> (S, Result<Vec<IndexFlag>, String>),
    requires
        forall|s: S| #[trigger] head.requires((s,)),
        forall|s: S| #[trigger] r#tracked.requires((s,)),
        forall|s: S| #[trigger] untracked.requires((s,)),
        forall|s: S| #[trigger] flags.requires((s,)),
    ensures exists|first: (S, Result<String, String>)| head.ensures((source,), first)
        && checkout_head_result(first, r#tracked, untracked, flags, out),
{
    let first = head(source);
    finish_checkout_head(first, r#tracked, untracked, flags)
}

pub open spec fn checkout_paths_merged(out: Seq<String>, first: Seq<String>, second: Seq<String>) -> bool {
    (forall|path: Seq<char>| #[trigger] checkout_path_prefix(out, out.len() as int, path)
        == checkout_path_prefix(first + second, (first.len() + second.len()) as int, path))
        && (forall|i: int, j: int| 0 <= i < j < out.len()
            ==> identifiers::byte_lt(vstd::utf8::encode_utf8((#[trigger] out[i])@),
                vstd::utf8::encode_utf8((#[trigger] out[j])@)))
        && out.len() <= first.len() + second.len()
}

pub open spec fn checkout_ignored_result<S>(
    first: Vec<String>, observed: (S, Result<Vec<String>, String>),
    out: (S, Result<Vec<String>, String>),
) -> bool {
    out.0 == observed.0 && match observed.1 {
        Err(error) => out.1 == Err::<Vec<String>, String>(error),
        Ok(second) => match out.1 {
            Err(_) => false,
            Ok(paths) => checkout_paths_merged(paths@, first@, second@),
        },
    }
}

pub open spec fn checkout_inventory_result<S, I>(observed: (S, Result<Vec<String>, String>),
    ignored: I, out: (S, Result<Vec<String>, String>)) -> bool
    where I: Fn(S) -> (S, Result<Vec<String>, String>) {
    match observed.1 {
        Err(error) => out == (observed.0, Err::<Vec<String>, String>(error)),
        Ok(paths) => exists|second: (S, Result<Vec<String>, String>)|
            ignored.ensures((observed.0,), second) && checkout_ignored_result(paths, second, out),
    }
}

fn finish_checkout_ignored<S>(paths: Vec<String>, observed: (S, Result<Vec<String>, String>))
    -> (out: (S, Result<Vec<String>, String>))
    ensures checkout_ignored_result(paths, observed, out),
{
    let (state, result) = observed;
    match result {
        Err(error) => (state, Err(error)),
        Ok(ignored) => (state, Ok(normalize_checkout_paths(paths, ignored))),
    }
}

fn finish_checkout_inventory<S, I>(observed: (S, Result<Vec<String>, String>), ignored: I)
    -> (out: (S, Result<Vec<String>, String>))
    where I: Fn(S) -> (S, Result<Vec<String>, String>),
    requires forall|s: S| #[trigger] ignored.requires((s,)),
    ensures checkout_inventory_result(observed, ignored, out),
{
    let (state, result) = observed;
    let paths = match result { Err(error) => { return (state, Err(error)); }, Ok(value) => value };
    let second = ignored(state);
    finish_checkout_ignored(paths, second)
}

/// 普通 checkout 先读取未跟踪库存，再读取 ignored 库存，成功后复用原全文合并。
pub fn collect_checkout_untracked<S, U, I>(source: S, untracked: U, ignored: I)
    -> (out: (S, Result<Vec<String>, String>))
    where U: Fn(S) -> (S, Result<Vec<String>, String>),
        I: Fn(S) -> (S, Result<Vec<String>, String>),
    requires
        forall|s: S| #[trigger] untracked.requires((s,)),
        forall|s: S| #[trigger] ignored.requires((s,)),
    ensures exists|first: (S, Result<Vec<String>, String>)| untracked.ensures((source,), first)
        && checkout_inventory_result(first, ignored, out),
{
    let first = untracked(source);
    finish_checkout_inventory(first, ignored)
}

pub enum UnsupportedIndexFlag {
    AssumeUnchanged,
    SkipWorktree,
    Both,
}

impl UnsupportedIndexFlag {
    pub open spec fn message_spec(&self) -> Seq<char> {
        match self {
            Self::AssumeUnchanged => "assume-unchanged"@,
            Self::SkipWorktree => "skip-worktree"@,
            Self::Both => "assume-unchanged and skip-worktree"@,
        }
    }

    pub fn message(&self) -> (out: &'static str)
        ensures out@ == self.message_spec(),
    {
        match self {
            Self::AssumeUnchanged => "assume-unchanged",
            Self::SkipWorktree => "skip-worktree",
            Self::Both => "assume-unchanged and skip-worktree",
        }
    }
}

pub open spec fn index_flag_admission(tag: Seq<u8>) -> Option<UnsupportedIndexFlag> {
    if tag.len() == 1 && tag[0] == 115 { Some(UnsupportedIndexFlag::Both) }
    else if tag.len() == 1 && tag[0] == 83 { Some(UnsupportedIndexFlag::SkipWorktree) }
    else if tag.len() > 0 && 97 <= tag[0] <= 122 { Some(UnsupportedIndexFlag::AssumeUnchanged) }
    else { None }
}

pub fn unsupported_index_flag(tag: &str) -> (out: Option<UnsupportedIndexFlag>)
    ensures out == index_flag_admission(tag.spec_bytes()),
{
    let bytes = tag.as_bytes();
    // Git's -v lowercases assume-unchanged; S/s denotes skip-worktree.
    if bytes.len() == 1 && bytes[0] == 115 { Some(UnsupportedIndexFlag::Both) }
    else if bytes.len() == 1 && bytes[0] == 83 { Some(UnsupportedIndexFlag::SkipWorktree) }
    else if bytes.len() > 0 && 97 <= bytes[0] && bytes[0] <= 122 {
        Some(UnsupportedIndexFlag::AssumeUnchanged)
    } else { None }
}

pub fn first_unsupported_index_flag(flags: &[IndexFlag])
    -> (out: Option<(&IndexFlag, UnsupportedIndexFlag)>)
    ensures match out {
        Some((flag, reason)) => exists|i: int| 0 <= i < flags.len() && (#[trigger] flags[i]) == *flag
            && index_flag_admission(vstd::utf8::encode_utf8(flag.tag@)) == Some(reason)
            && (forall|j: int| 0 <= j < i ==> index_flag_admission(vstd::utf8::encode_utf8((#[trigger] flags[j]).tag@)).is_none()),
        None => forall|i: int| 0 <= i < flags.len() ==>
            index_flag_admission(vstd::utf8::encode_utf8((#[trigger] flags[i]).tag@)).is_none(),
    },
{
    let mut i = 0;
    while i < flags.len()
        invariant 0 <= i <= flags.len(),
            forall|j: int| 0 <= j < i ==> index_flag_admission(vstd::utf8::encode_utf8((#[trigger] flags[j]).tag@)).is_none(),
        decreases flags.len() - i,
    {
        let flag = &flags[i];
        if let Some(reason) = unsupported_index_flag(flag.tag.as_str()) {
            return Some((flag, reason));
        }
        i += 1;
    }
    None
}

/// 原字节与安全标准 UTF-8 解码的观察。适配层只搬运这两个值。
pub struct GitIndexFlagObservation<'a> {
    pub raw: &'a [u8],
    pub text: Option<&'a str>,
}

pub open spec fn index_flag_observation_bound(row: &GitIndexFlagObservation) -> bool {
    match row.text { Some(text) => text.spec_bytes() == row.raw@, None => true }
}

pub open spec fn index_flag_observations_bound(rows: Seq<GitIndexFlagObservation>) -> bool {
    forall|i: int| 0 <= i < rows.len() ==> #[trigger] index_flag_observation_bound(&rows[i])
}

#[cfg(verus_keep_ghost)]
pub type IndexFlagBytes = (Seq<u8>, Seq<u8>);

pub open spec fn index_flag_bytes(flag: &IndexFlag) -> IndexFlagBytes {
    (vstd::utf8::encode_utf8(flag.tag@), vstd::utf8::encode_utf8(flag.path@))
}

pub open spec fn index_flag_header(raw: Seq<u8>) -> bool {
    raw.len() >= 3 && raw[1] == 32
        && (65 <= raw[0] <= 90 || 97 <= raw[0] <= 122)
}

pub open spec fn index_flag_row_spec(row: &GitIndexFlagObservation)
    -> Result<IndexFlagBytes, Seq<char>>
{
    if !index_flag_header(row.raw@) { Err("invalid Git index flag record"@) }
    else { match row.text {
        None => Err("E_INPUT_UNSUPPORTED: non UTF-8 Git output"@),
        Some(_) => Ok((row.raw@.take(1), row.raw@.skip(2))),
    } }
}

/// 先核原字节头部，再采用 UTF-8 观察；首错次序与原 Git 接口一致。
pub fn parse_git_index_flag_row(row: &GitIndexFlagObservation) -> (out: Result<IndexFlag, String>)
    requires index_flag_observation_bound(row),
    ensures match out {
        Ok(flag) => index_flag_row_spec(row) == Ok::<IndexFlagBytes, Seq<char>>(index_flag_bytes(&flag)),
        Err(message) => index_flag_row_spec(row) == Err::<IndexFlagBytes, Seq<char>>(message@),
    },
{
    let raw = row.raw;
    if raw.len() < 3 || raw[1] != 32
        || !((65 <= raw[0] && raw[0] <= 90) || (97 <= raw[0] && raw[0] <= 122)) {
        return Err("invalid Git index flag record".to_owned());
    }
    let text = match row.text {
        None => { return Err("E_INPUT_UNSUPPORTED: non UTF-8 Git output".to_owned()); },
        Some(text) => text,
    };
    proof { identifiers::ascii_separator_end_is(text.spec_bytes(), 32, 1); }
    let (tag, rest) = identifiers::ascii_split_head(text, 32);
    match rest {
        Some(path) => Ok(IndexFlag { path: path.to_owned(), tag: tag.to_owned() }),
        None => Err("invalid Git index flag record".to_owned()),
    }
}

/// 完整行序列逐项解释；保留顺序和重复，错误严格来自最早失败行。
pub fn parse_git_index_flag_rows(rows: &[GitIndexFlagObservation]) -> (out: Result<Vec<IndexFlag>, String>)
    requires index_flag_observations_bound(rows@),
    ensures match out {
        Ok(flags) => flags.len() == rows.len()
            && forall|i: int| 0 <= i < rows.len() ==>
                index_flag_row_spec(&rows@[i]) == Ok::<IndexFlagBytes, Seq<char>>(index_flag_bytes(&flags@[i])),
        Err(message) => exists|i: int| 0 <= i < rows.len()
            && (forall|j: int| 0 <= j < i ==> index_flag_row_spec(&rows@[j]).is_ok())
            && index_flag_row_spec(&rows@[i]) == Err::<IndexFlagBytes, Seq<char>>(message@),
    },
{
    let mut flags: Vec<IndexFlag> = Vec::new();
    let mut i = 0usize;
    while i < rows.len()
        invariant i <= rows.len(), flags.len() == i,
            index_flag_observations_bound(rows@),
            forall|j: int| 0 <= j < i ==> index_flag_row_spec(&rows@[j])
                == Ok::<IndexFlagBytes, Seq<char>>(index_flag_bytes(&flags@[j])),
        decreases rows.len() - i,
    {
        let flag = match parse_git_index_flag_row(&rows[i]) {
            Ok(flag) => flag,
            Err(message) => { return Err(message); },
        };
        flags.push(flag);
        i += 1;
    }
    Ok(flags)
}

fn git_nul_record_head(raw: &[u8]) -> (out: (&[u8], Option<&[u8]>))
    ensures ({ let end = identifiers::ascii_separator_end(raw@, 0);
        identifiers::first_ascii_separator(raw@, 0, end)
            && out.0@ == raw@.take(end)
            && match out.1 {
                Some(tail) => end < raw.len() && tail@ == raw@.skip(end + 1),
                None => end == raw.len(),
            }
    }),
{
    let mut end = 0usize;
    while end < raw.len() && raw[end] != 0
        invariant end <= raw.len(), forall|i: int| 0 <= i < end ==> raw@[i] != 0,
        decreases raw.len() - end,
    { end += 1; }
    proof { identifiers::ascii_separator_end_is(raw@, 0, end as int); }
    if end == raw.len() { return (raw, None); }
    let (head, rest) = raw.split_at(end);
    let (_, tail) = rest.split_at(1);
    (head, Some(tail))
}

pub open spec fn git_nul_record_views(rows: Seq<&[u8]>) -> Seq<Seq<u8>> {
    rows.map(|i: int, row: &[u8]| row@)
}

/// 按原字节的 NUL 拆分；忽略空项，末尾无 NUL 仍保留，不提前解码后续行。
pub fn git_nul_records(raw: &[u8]) -> (out: Vec<&[u8]>)
    ensures git_nul_record_views(out@) == identifiers::ascii_split_records(raw@, 0, false),
        out.len() == identifiers::ascii_split_records(raw@, 0, false).len(),
        forall|i: int| 0 <= i < out.len() ==> (#[trigger] out@[i])@
            == identifiers::ascii_split_records(raw@, 0, false)[i],
{
    let mut remaining = raw;
    let mut out: Vec<&[u8]> = Vec::new();
    loop
        invariant git_nul_record_views(out@)
            + identifiers::ascii_split_records(remaining@, 0, false)
            == identifiers::ascii_split_records(raw@, 0, false),
        decreases remaining.len(),
    {
        let ghost previous = git_nul_record_views(out@);
        let (head, rest) = git_nul_record_head(remaining);
        proof {
            reveal_with_fuel(identifiers::ascii_split_records, 1);
            assert(identifiers::ascii_split_records(remaining@, 0, false)
                == (if head.len() > 0 { seq![head@] } else { seq![] })
                    + match rest {
                        Some(tail) => identifiers::ascii_split_records(tail@, 0, false),
                        None => seq![],
                    });
        }
        if head.len() > 0 {
            out.push(head);
            proof { assert(git_nul_record_views(out@) =~= previous + seq![head@]); }
        }
        match rest {
            None => { return out; },
            Some(tail) => { remaining = tail; },
        }
    }
}

/// 标准解码观察的成功值原样移交，失败只产生原有完整错误。
pub fn git_output_text(decoded: Option<String>) -> (out: Result<String, String>)
    ensures match (decoded, out) {
        (Some(original), Ok(text)) => text@ == original@,
        (None, Err(message)) => message@ == "E_INPUT_UNSUPPORTED: non UTF-8 Git output"@,
        _ => false,
    },
{
    match decoded {
        Some(text) => Ok(text),
        None => Err("E_INPUT_UNSUPPORTED: non UTF-8 Git output".to_owned()),
    }
}

/// 逐项消费完整解码观察；成功保留数量、顺序和重复，失败只来自首个缺失解码。
pub fn parse_git_text_rows(decoded: Vec<Option<String>>) -> (out: Result<Vec<String>, String>)
    ensures match out {
        Ok(paths) => paths.len() == decoded.len()
            && forall|i: int| 0 <= i < decoded.len() ==> match (#[trigger] decoded@[i]) {
                Some(text) => text@ == paths@[i]@,
                None => false,
            },
        Err(message) => message@ == "E_INPUT_UNSUPPORTED: non UTF-8 Git output"@
            && exists|i: int| 0 <= i < decoded.len() && (#[trigger] decoded@[i]).is_none()
                && forall|j: int| 0 <= j < i ==> (#[trigger] decoded@[j]).is_some(),
    },
{
    let ghost all = decoded@;
    let mut iterator = decoded.into_iter();
    let mut paths: Vec<String> = Vec::new();
    loop
        invariant all == decoded@,
            iterator@.1 == all, 0 <= iterator@.0 <= all.len(),
            paths.len() == iterator@.0,
            forall|i: int| 0 <= i < paths.len() ==> match (#[trigger] all[i]) {
                Some(text) => text@ == paths@[i]@,
                None => false,
            },
        decreases all.len() - iterator@.0,
    {
        let ghost at = iterator@.0;
        match iterator.next() {
            None => { return Ok(paths); },
            Some(row) => {
                let path = match git_output_text(row) {
                    Ok(path) => path,
                    Err(message) => {
                        proof {
                            assert(all[at].is_none());
                            assert forall|j: int| 0 <= j < at implies
                                (#[trigger] all[j]).is_some() by {
                                assert(match all[j] { Some(text) => text@ == paths@[j]@, None => false });
                            }
                        }
                        return Err(message);
                    },
                };
                paths.push(path);
            },
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub enum Status {
    Passed,
    Failed,
    NotRequired,
    Blocked,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "lowercase"))]
pub enum WireStatus {
    Pass,
    Warn,
    Fail,
    Error,
}

impl WireStatus {
    pub open spec fn rank_spec(&self) -> u8 {
        match self { Self::Pass => 0, Self::Warn => 1, Self::Fail => 2, Self::Error => 3 }
    }

    fn rank(&self) -> (rank: u8)
        ensures rank == self.rank_spec(),
    {
        match self { Self::Pass => 0, Self::Warn => 1, Self::Fail => 2, Self::Error => 3 }
    }

    pub open spec fn order_spec(&self, other: &Self) -> Ordering {
        if self.rank_spec() < other.rank_spec() { Ordering::Less }
        else if self.rank_spec() > other.rank_spec() { Ordering::Greater }
        else { Ordering::Equal }
    }

    pub open spec fn exit_spec(&self) -> i32 {
        match self { Self::Pass | Self::Warn => 0, Self::Fail => 1, Self::Error => 2 }
    }

    pub fn exit_code(&self) -> (exit: i32)
        ensures exit == self.exit_spec(),
    {
        match self { Self::Pass | Self::Warn => 0, Self::Fail => 1, Self::Error => 2 }
    }

    /// 汇总实际协议状态，精确保留严重程度较高的那个变体。
    pub fn maximum(self, other: Self) -> (out: Self)
        ensures out == (if self.rank_spec() >= other.rank_spec() { self } else { other }),
    {
        if self.rank() >= other.rank() { self } else { other }
    }
}

/// 普通响应和保留报告共用实际头部字段；不另加名称或协议内容限制。
pub fn wire_header_matches(
    protocol: &str, request_id: &str, judge_id: &str, status: &WireStatus, exit: i32,
    expected_protocol: &str, expected_request_id: &str, expected_judge_id: &str,
) -> (matches: bool)
    ensures matches <==> protocol@ == expected_protocol@
        && request_id@ == expected_request_id@ && judge_id@ == expected_judge_id@
        && exit == status.exit_spec(),
{
    identifiers::equal_text(protocol, expected_protocol)
        && identifiers::equal_text(request_id, expected_request_id)
        && identifiers::equal_text(judge_id, expected_judge_id)
        && exit == status.exit_code()
}

pub proof fn wire_status_order_laws(a: WireStatus, b: WireStatus, c: WireStatus)
    ensures
        (a.rank_spec() == b.rank_spec()) == (a == b),
        (a.order_spec(&b) == Ordering::Equal) == (a == b),
        (a.order_spec(&b) == Ordering::Less) == (b.order_spec(&a) == Ordering::Greater),
        a.order_spec(&b) == Ordering::Less && b.order_spec(&c) == Ordering::Less
            ==> a.order_spec(&c) == Ordering::Less,
{}

impl Clone for WireStatus {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        match self { Self::Pass => Self::Pass, Self::Warn => Self::Warn,
            Self::Fail => Self::Fail, Self::Error => Self::Error }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for WireStatus {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { *self == *other }
}

impl PartialEq for WireStatus {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (*self == *other),
    { self.rank() == other.rank() }
}

impl Eq for WireStatus {}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::OrdSpecImpl for WireStatus {
    open spec fn obeys_cmp_spec() -> bool { true }
    open spec fn cmp_spec(&self, other: &Self) -> Ordering { self.order_spec(other) }
}

impl Ord for WireStatus {
    fn cmp(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        let rank = self.rank();
        let other_rank = other.rank();
        if rank < other_rank { Ordering::Less }
        else if rank > other_rank { Ordering::Greater }
        else { Ordering::Equal }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialOrdSpecImpl for WireStatus {
    open spec fn obeys_partial_cmp_spec() -> bool { true }
    open spec fn partial_cmp_spec(&self, other: &Self) -> Option<Ordering> {
        Some(self.order_spec(other))
    }
}

impl PartialOrd for WireStatus {
    fn partial_cmp(&self, other: &Self) -> (order: Option<Ordering>)
        ensures order == Some(self.order_spec(other)),
    { Some(self.cmp(other)) }
}

/// 实际树键按完整 UTF-8 内容比较；不归一化路径。
#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
pub struct TreePath {
    pub text: String,
}

#[cfg(verus_keep_ghost)]
impl View for TreePath {
    type V = Seq<char>;
    open spec fn view(&self) -> Seq<char> { self.text@ }
}

impl TreePath {
    pub open spec fn bytes(&self) -> Seq<u8> { vstd::utf8::encode_utf8(self.text@) }
    pub open spec fn same(&self, other: &Self) -> bool { self.bytes() == other.bytes() }
    pub open spec fn order_spec(&self, other: &Self) -> Ordering {
        if self.same(other) { Ordering::Equal }
        else if identifiers::byte_lt(self.bytes(), other.bytes()) { Ordering::Less }
        else { Ordering::Greater }
    }
    pub fn as_str(&self) -> (out: &str)
        ensures out@ == self@, out.spec_bytes() == self.bytes(),
    { self.text.as_str() }
}

impl Clone for TreePath {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    { Self { text: self.text.clone() } }
}

impl TreePath {
    pub fn from_owned(text: String) -> (out: Self)
        ensures out.text == text,
    { Self { text } }

    pub fn from_text(text: &str) -> (out: Self)
        ensures out@ == text@, out.bytes() == text.spec_bytes(),
    { Self::from_owned(text.to_owned()) }
}

impl std::ops::Deref for TreePath {
    type Target = str;
    fn deref(&self) -> (out: &str)
        ensures out@ == self@, out.spec_bytes() == self.bytes(),
    { self.as_str() }
}

impl std::borrow::Borrow<str> for TreePath {
    fn borrow(&self) -> (out: &str)
        ensures ({ let text: &str = out; text@ == self@ && text.spec_bytes() == self.bytes() }),
    { self.as_str() }
}

impl std::borrow::Borrow<String> for TreePath {
    fn borrow(&self) -> (out: &String)
        ensures ({ let text: &String = out; *text == self.text }),
    { &self.text }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for TreePath {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self.same(other) }
}

impl PartialEq for TreePath {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == self.same(other),
    { identifiers::equal_bytes(self.as_str().as_bytes(), other.as_str().as_bytes()) }
}

impl Eq for TreePath {}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::OrdSpecImpl for TreePath {
    open spec fn obeys_cmp_spec() -> bool { true }
    open spec fn cmp_spec(&self, other: &Self) -> Ordering { self.order_spec(other) }
}

impl Ord for TreePath {
    fn cmp(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        let order = identifiers::compare_bytes(self.as_str().as_bytes(), other.as_str().as_bytes());
        proof {
            identifiers::byte_lt_asymmetric(self.bytes(), other.bytes());
            identifiers::byte_lt_asymmetric(other.bytes(), self.bytes());
        }
        if order == 0 { Ordering::Equal }
        else if order < 0 { Ordering::Less }
        else { Ordering::Greater }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialOrdSpecImpl for TreePath {
    open spec fn obeys_partial_cmp_spec() -> bool { true }
    open spec fn partial_cmp_spec(&self, other: &Self) -> Option<Ordering> {
        Some(self.order_spec(other))
    }
}

impl PartialOrd for TreePath {
    fn partial_cmp(&self, other: &Self) -> (order: Option<Ordering>)
        ensures order == Some(self.order_spec(other)),
    { Some(self.cmp(other)) }
}

pub proof fn tree_path_order_laws(a: &TreePath, b: &TreePath, c: &TreePath)
    ensures
        (a.order_spec(b) == Ordering::Equal) == a.same(b),
        (a.order_spec(b) == Ordering::Less) == (b.order_spec(a) == Ordering::Greater),
        a.order_spec(b) == Ordering::Less && b.order_spec(c) == Ordering::Less
            ==> a.order_spec(c) == Ordering::Less,
{
    identifiers::byte_trichotomy(a.bytes(), b.bytes());
    identifiers::byte_lt_asymmetric(a.bytes(), b.bytes());
    identifiers::byte_lt_asymmetric(b.bytes(), a.bytes());
    if identifiers::byte_lt(a.bytes(), b.bytes()) { identifiers::byte_lt_strict(a.bytes(), b.bytes()); }
    if identifiers::byte_lt(b.bytes(), a.bytes()) { identifiers::byte_lt_strict(b.bytes(), a.bytes()); }
    if a.order_spec(b) == Ordering::Less && b.order_spec(c) == Ordering::Less {
        identifiers::byte_lt_transitive(a.bytes(), b.bytes(), c.bytes());
        identifiers::byte_lt_strict(a.bytes(), c.bytes());
    }
}

pub proof fn tree_path_obeys_cmp()
    ensures vstd::laws_cmp::obeys_cmp::<TreePath>(),
{
    use vstd::std_specs::cmp::{PartialEqSpec, PartialOrdSpec, OrdSpec};
    reveal(vstd::laws_eq::obeys_eq_spec_properties);
    reveal(vstd::laws_cmp::obeys_partial_cmp_spec_properties);
    reveal(vstd::laws_cmp::obeys_cmp_partial_ord);
    reveal(vstd::laws_cmp::obeys_cmp_ord);
    assert forall|a: TreePath, b: TreePath|
        a.partial_cmp_spec(&b) == Some(Ordering::Less)
            <==> b.partial_cmp_spec(&a) == Some(Ordering::Greater) by {
        tree_path_order_laws(&a, &b, &b);
    }
    assert forall|a: TreePath, b: TreePath, c: TreePath|
        a.partial_cmp_spec(&b) == Some(Ordering::Less)
            && b.partial_cmp_spec(&c) == Some(Ordering::Less)
            implies a.partial_cmp_spec(&c) == Some(Ordering::Less) by {
        tree_path_order_laws(&a, &b, &c);
    }
    assert forall|a: TreePath, b: TreePath, c: TreePath|
        a.partial_cmp_spec(&b) == Some(Ordering::Greater)
            && b.partial_cmp_spec(&c) == Some(Ordering::Greater)
            implies a.partial_cmp_spec(&c) == Some(Ordering::Greater) by {
        tree_path_order_laws(&c, &b, &a);
    }
}

pub type Tree = BTreeMap<TreePath, TreeEntry>;

pub open spec fn tree_rows_view(rows: Seq<(&TreePath, &TreeEntry)>) -> Seq<(TreePath, TreeEntry)> {
    rows.map(|i: int, row: (&TreePath, &TreeEntry)| (*row.0, *row.1))
}

pub open spec fn tree_rows_ordered(rows: Seq<(&TreePath, &TreeEntry)>) -> bool {
    forall|i: int, j: int| 0 <= i < j < rows.len()
        ==> identifiers::byte_lt((#[trigger] rows[i]).0.bytes(), (#[trigger] rows[j]).0.bytes())
}

/// 借用真实树的全部原键和值，保留字节严格顺序。
fn tree_rows(tree: &Tree) -> (rows: Vec<(&TreePath, &TreeEntry)>)
    ensures
        tree_rows_view(rows@).to_set() == tree@.kv_pairs(),
        tree_rows_ordered(rows@),
{
    proof { tree_path_obeys_cmp(); }
    let mut iterator = tree.iter();
    let ghost all = iterator@.1;
    let mut rows: Vec<(&TreePath, &TreeEntry)> = Vec::new();
    loop
        invariant
            iterator@.1 == all,
            iterator@.0 == rows.len(),
            0 <= iterator@.0 <= all.len(),
            tree_rows_view(rows@) == all.take(iterator@.0),
            all.to_set() == tree@.kv_pairs(),
            vstd::std_specs::btree::increasing_seq(all.map(|i: int, row: (TreePath, TreeEntry)| row.0)),
        decreases all.len() - rows.len(),
    {
        match iterator.next() {
            None => {
                proof {
                    tree_path_obeys_cmp();
                    let keys = all.map(|i: int, row: (TreePath, TreeEntry)| row.0);
                    vstd::std_specs::btree::axiom_increasing_seq_meaning(keys);
                    assert(tree_rows_view(rows@) == all);
                    assert forall|i: int, j: int| 0 <= i < j < rows.len()
                        implies identifiers::byte_lt((#[trigger] rows@[i]).0.bytes(),
                            (#[trigger] rows@[j]).0.bytes()) by {
                        assert(keys[i] == all[i].0);
                        assert(keys[j] == all[j].0);
                        assert(keys[i].cmp_spec(&keys[j]) is Less);
                        assert(all[i].0.order_spec(&all[j].0) is Less);
                        identifiers::byte_lt_strict(all[i].0.bytes(), all[j].0.bytes());
                    }
                }
                return rows;
            }
            Some((key, value)) => {
                rows.push((key, value));
                proof { assert(tree_rows_view(rows@) =~= all.take(iterator@.0)); }
            }
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct TreeEntry {
    pub mode: String,
    pub oid: String,
    pub kind: String,
}

pub open spec fn tree_entry_equal(left: &TreeEntry, right: &TreeEntry) -> bool {
    left.mode@ == right.mode@ && left.oid@ == right.oid@ && left.kind@ == right.kind@
}

impl Clone for TreeEntry {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        Self { mode: self.mode.clone(), oid: self.oid.clone(), kind: self.kind.clone() }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for TreeEntry {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { tree_entry_equal(self, other) }
}

impl PartialEq for TreeEntry {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == tree_entry_equal(self, other),
    {
        identifiers::equal_text(self.mode.as_str(), other.mode.as_str())
            && identifiers::equal_text(self.oid.as_str(), other.oid.as_str())
            && identifiers::equal_text(self.kind.as_str(), other.kind.as_str())
    }
}

impl Eq for TreeEntry {}

pub open spec fn tree_entries_equal(old: Option<&TreeEntry>, new: Option<&TreeEntry>) -> bool {
    match (old, new) {
        (None, None) => true,
        (Some(a), Some(b)) => tree_entry_equal(a, b),
        _ => false,
    }
}

fn unchanged_tree_entries(old: Option<&TreeEntry>, new: Option<&TreeEntry>) -> (equal: bool)
    ensures equal == tree_entries_equal(old, new),
{
    match (old, new) {
        (None, None) => true,
        (Some(a), Some(b)) => a.eq(b),
        _ => false,
    }
}

pub open spec fn entry_blob(entry: Option<&TreeEntry>) -> Option<Seq<char>> {
    match entry { None => None, Some(entry) => Some(entry.oid@) }
}

pub open spec fn entry_mode(entry: Option<&TreeEntry>) -> Option<Seq<char>> {
    match entry { None => None, Some(entry) => Some(entry.mode@) }
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Delta {
    pub kind: String,
    pub path: String,
    pub old_blob: Option<String>,
    pub new_blob: Option<String>,
    pub old_mode: Option<String>,
    pub new_mode: Option<String>,
}

fn clone_optional_text(value: &Option<String>) -> (out: Option<String>)
    ensures out == *value,
{
    match value { None => None, Some(text) => Some(text.clone()) }
}

#[verifier::inline]
pub open spec fn optional_text(value: &Option<String>) -> Option<Seq<char>> {
    match value { None => None, Some(text) => Some(text@) }
}

/// 直接读取两端实际条目；kind 变化也属于修改，即使 blob 和 mode 相同。
pub fn delta_for_entry(path: &str, old: Option<&TreeEntry>, new: Option<&TreeEntry>) -> (out: Option<Delta>)
    ensures
        out.is_none() == tree_entries_equal(old, new),
        match out {
            None => true,
            Some(change) => change.path@ == path@
                && change.kind@ == (if old.is_none() { "A"@ }
                    else if new.is_none() { "D"@ } else { "M"@ })
                && optional_text(&change.old_blob) == entry_blob(old)
                && optional_text(&change.new_blob) == entry_blob(new)
                && optional_text(&change.old_mode) == entry_mode(old)
                && optional_text(&change.new_mode) == entry_mode(new),
        },
{
    if unchanged_tree_entries(old, new) { return None; }
    let kind = if old.is_none() { "A" } else if new.is_none() { "D" } else { "M" };
    Some(Delta {
        kind: kind.to_owned(), path: path.to_owned(),
        old_blob: match old { None => None, Some(entry) => Some(entry.oid.clone()) },
        new_blob: match new { None => None, Some(entry) => Some(entry.oid.clone()) },
        old_mode: match old { None => None, Some(entry) => Some(entry.mode.clone()) },
        new_mode: match new { None => None, Some(entry) => Some(entry.mode.clone()) },
    })
}

#[cfg(verus_keep_ghost)]
pub type DeltaFields = (Seq<char>, Seq<char>, Option<Seq<char>>, Option<Seq<char>>,
    Option<Seq<char>>, Option<Seq<char>>);

pub open spec fn delta_record_view(change: Delta) -> DeltaFields {
    (change.path@, change.kind@, optional_text(&change.old_blob), optional_text(&change.new_blob),
        optional_text(&change.old_mode), optional_text(&change.new_mode))
}

pub open spec fn delta_records_view(changes: Seq<Delta>) -> Seq<DeltaFields> {
    changes.map(|i: int, change: Delta| delta_record_view(change))
}

pub open spec fn entry_delta_fields(path: Seq<char>, old_entry: Option<&TreeEntry>,
    new_entry: Option<&TreeEntry>) -> Seq<DeltaFields>
{
    if tree_entries_equal(old_entry, new_entry) { seq![] }
    else { seq![(path,
        if old_entry.is_none() { "A"@ } else if new_entry.is_none() { "D"@ } else { "M"@ },
        entry_blob(old_entry), entry_blob(new_entry), entry_mode(old_entry), entry_mode(new_entry))] }
}

#[cfg(verus_keep_ghost)]
pub type TreeEntryFields = (Seq<char>, Seq<char>, Seq<char>);

pub open spec fn tree_entry_fields(entry: &TreeEntry) -> TreeEntryFields {
    (entry.mode@, entry.oid@, entry.kind@)
}

/// 标准迭代器的完整、严格有序库存连接实际 map 键与全文身份。
/// 只创建迭代器，不遍历或复制库存。
fn tree_keys_have_unique_text<V>(tree: &BTreeMap<TreePath, V>)
    ensures forall|a: TreePath, b: TreePath| #![trigger tree@.contains_key(a), tree@.contains_key(b)]
        tree@.contains_key(a) && tree@.contains_key(b) && a@ == b@ ==> a == b,
{
    proof { tree_path_obeys_cmp(); }
    let iterator = tree.iter();
    proof {
        let rows = iterator@.1;
        let keys = rows.map(|i: int, row: (TreePath, V)| row.0);
        vstd::std_specs::btree::axiom_increasing_seq_meaning(keys);
        assert forall|a: TreePath, b: TreePath| #![trigger tree@.contains_key(a), tree@.contains_key(b)]
            tree@.contains_key(a) && tree@.contains_key(b) && a@ == b@
                implies a == b by {
            assert(rows.to_set().contains((a, tree@[a])));
            assert(rows.to_set().contains((b, tree@[b])));
            let i = choose|i: int| 0 <= i < rows.len() && rows[i] == (a, tree@[a]);
            let j = choose|j: int| 0 <= j < rows.len() && rows[j] == (b, tree@[b]);
            if i < j {
                assert(keys[i].cmp_spec(&keys[j]) is Less);
                assert(a.order_spec(&b) is Less);
            } else if j < i {
                assert(keys[j].cmp_spec(&keys[i]) is Less);
                assert(b.order_spec(&a) is Less);
            }
        }
    }
}

/// Git tree 的实际插入状态：完整构造条目，原路径已存在时保留原重复诊断。
/// 沿用原 insert 的更新顺序；失败时该路径也已替换，调用方丢弃整棵候选树。
pub fn insert_git_tree_entry(
    tree: &mut Tree, path: TreePath, mode: &str, oid: &str, kind: &str,
) -> (out: Result<(), String>)
    ensures
        tree@.contains_key(path),
        tree@ == old(tree)@.insert(path, tree@[path]),
        tree_entry_fields(&tree@[path]) == (mode@, oid@, kind@),
        out.is_ok() == !old(tree)@.contains_key(path),
        out.is_ok() == !(exists|key: TreePath| (#[trigger] old(tree)@.contains_key(key)) && key@ == path@),
        match out {
            Ok(()) => true,
            Err(message) => message@ == "duplicate Git tree path"@,
        },
{
    proof { tree_path_obeys_cmp(); }
    let ghost before = tree@;
    let entry = TreeEntry {
        mode: mode.to_owned(), oid: oid.to_owned(), kind: kind.to_owned(),
    };
    let previous = tree.insert(path, entry);
    tree_keys_have_unique_text(tree);
    proof {
        if exists|key: TreePath| (#[trigger] before.contains_key(key)) && key@ == path@ {
            let key = choose|key: TreePath| (#[trigger] before.contains_key(key)) && key@ == path@;
            assert(tree@.contains_key(key));
            assert(tree@.contains_key(path));
            assert(key == path);
            assert(before.contains_key(path));
        }
        if before.contains_key(path) {
            assert(exists|key: TreePath| (#[trigger] before.contains_key(key)) && key@ == path@);
        }
        assert(before.contains_key(path) ==
            (exists|key: TreePath| (#[trigger] before.contains_key(key)) && key@ == path@));
    }
    match previous {
        Some(_) => Err("duplicate Git tree path".to_owned()),
        None => Ok(()),
    }
}

/// 与 Git 原 index 库存一样借用完整路径和 stage；不复制或归一化字段。
pub struct GitIndexStage<'a> {
    pub path: &'a str,
    pub stage: &'a str,
}

impl<'a> GitIndexStage<'a> {
    pub open spec fn same(&self, other: &Self) -> bool {
        self.path.spec_bytes() == other.path.spec_bytes()
            && self.stage.spec_bytes() == other.stage.spec_bytes()
    }

    pub open spec fn order_spec(&self, other: &Self) -> Ordering {
        if self.path.spec_bytes() == other.path.spec_bytes() {
            if self.stage.spec_bytes() == other.stage.spec_bytes() { Ordering::Equal }
            else if identifiers::byte_lt(self.stage.spec_bytes(), other.stage.spec_bytes()) {
                Ordering::Less
            } else { Ordering::Greater }
        } else if identifiers::byte_lt(self.path.spec_bytes(), other.path.spec_bytes()) {
            Ordering::Less
        } else { Ordering::Greater }
    }
}

#[cfg(verus_keep_ghost)]
impl<'a> vstd::std_specs::cmp::PartialEqSpecImpl for GitIndexStage<'a> {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self.same(other) }
}

impl<'a> PartialEq for GitIndexStage<'a> {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == self.same(other),
    {
        identifiers::equal_bytes(self.path.as_bytes(), other.path.as_bytes())
            && identifiers::equal_bytes(self.stage.as_bytes(), other.stage.as_bytes())
    }
}

impl<'a> Eq for GitIndexStage<'a> {}

#[cfg(verus_keep_ghost)]
impl<'a> vstd::std_specs::cmp::OrdSpecImpl for GitIndexStage<'a> {
    open spec fn obeys_cmp_spec() -> bool { true }
    open spec fn cmp_spec(&self, other: &Self) -> Ordering { self.order_spec(other) }
}

impl<'a> Ord for GitIndexStage<'a> {
    fn cmp(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        let path_order = identifiers::compare_bytes(self.path.as_bytes(), other.path.as_bytes());
        proof {
            identifiers::byte_lt_asymmetric(self.path.spec_bytes(), other.path.spec_bytes());
            identifiers::byte_lt_asymmetric(other.path.spec_bytes(), self.path.spec_bytes());
            identifiers::byte_lt_asymmetric(self.stage.spec_bytes(), other.stage.spec_bytes());
            identifiers::byte_lt_asymmetric(other.stage.spec_bytes(), self.stage.spec_bytes());
        }
        let order = if path_order == 0 {
            identifiers::compare_bytes(self.stage.as_bytes(), other.stage.as_bytes())
        } else { path_order };
        if order == 0 { Ordering::Equal }
        else if order < 0 { Ordering::Less }
        else { Ordering::Greater }
    }
}

#[cfg(verus_keep_ghost)]
impl<'a> vstd::std_specs::cmp::PartialOrdSpecImpl for GitIndexStage<'a> {
    open spec fn obeys_partial_cmp_spec() -> bool { true }
    open spec fn partial_cmp_spec(&self, other: &Self) -> Option<Ordering> {
        Some(self.order_spec(other))
    }
}

impl<'a> PartialOrd for GitIndexStage<'a> {
    fn partial_cmp(&self, other: &Self) -> (order: Option<Ordering>)
        ensures order == Some(self.order_spec(other)),
    { Some(self.cmp(other)) }
}

pub proof fn git_index_stage_order_laws(a: &GitIndexStage, b: &GitIndexStage, c: &GitIndexStage)
    ensures
        (a.order_spec(b) == Ordering::Equal) == a.same(b),
        (a.order_spec(b) == Ordering::Less) == (b.order_spec(a) == Ordering::Greater),
        a.order_spec(b) == Ordering::Less && b.order_spec(c) == Ordering::Less
            ==> a.order_spec(c) == Ordering::Less,
{
    identifiers::byte_trichotomy(a.path.spec_bytes(), b.path.spec_bytes());
    identifiers::byte_lt_asymmetric(a.path.spec_bytes(), b.path.spec_bytes());
    identifiers::byte_lt_asymmetric(b.path.spec_bytes(), a.path.spec_bytes());
    identifiers::byte_trichotomy(a.stage.spec_bytes(), b.stage.spec_bytes());
    identifiers::byte_lt_asymmetric(a.stage.spec_bytes(), b.stage.spec_bytes());
    identifiers::byte_lt_asymmetric(b.stage.spec_bytes(), a.stage.spec_bytes());
    if a.order_spec(b) == Ordering::Less && b.order_spec(c) == Ordering::Less {
        if a.path.spec_bytes() == b.path.spec_bytes() && b.path.spec_bytes() == c.path.spec_bytes() {
            identifiers::byte_lt_transitive(a.stage.spec_bytes(), b.stage.spec_bytes(), c.stage.spec_bytes());
            identifiers::byte_lt_strict(a.stage.spec_bytes(), c.stage.spec_bytes());
        } else if a.path.spec_bytes() != b.path.spec_bytes() && b.path.spec_bytes() != c.path.spec_bytes() {
            identifiers::byte_lt_transitive(a.path.spec_bytes(), b.path.spec_bytes(), c.path.spec_bytes());
            identifiers::byte_lt_strict(a.path.spec_bytes(), c.path.spec_bytes());
        } else {
            identifiers::byte_lt_strict(a.path.spec_bytes(), c.path.spec_bytes());
        }
    }
}

pub proof fn git_index_stage_obeys_cmp<'a>()
    ensures vstd::laws_cmp::obeys_cmp::<GitIndexStage<'a>>(),
{
    use vstd::std_specs::cmp::{PartialEqSpec, PartialOrdSpec, OrdSpec};
    reveal(vstd::laws_eq::obeys_eq_spec_properties);
    reveal(vstd::laws_cmp::obeys_partial_cmp_spec_properties);
    reveal(vstd::laws_cmp::obeys_cmp_partial_ord);
    reveal(vstd::laws_cmp::obeys_cmp_ord);
    assert forall|a: GitIndexStage<'a>, b: GitIndexStage<'a>|
        a.partial_cmp_spec(&b) == Some(Ordering::Less)
            <==> b.partial_cmp_spec(&a) == Some(Ordering::Greater) by {
        git_index_stage_order_laws(&a, &b, &b);
    }
    assert forall|a: GitIndexStage<'a>, b: GitIndexStage<'a>, c: GitIndexStage<'a>|
        a.partial_cmp_spec(&b) == Some(Ordering::Less)
            && b.partial_cmp_spec(&c) == Some(Ordering::Less)
            implies a.partial_cmp_spec(&c) == Some(Ordering::Less) by {
        git_index_stage_order_laws(&a, &b, &c);
    }
    assert forall|a: GitIndexStage<'a>, b: GitIndexStage<'a>, c: GitIndexStage<'a>|
        a.partial_cmp_spec(&b) == Some(Ordering::Greater)
            && b.partial_cmp_spec(&c) == Some(Ordering::Greater)
            implies a.partial_cmp_spec(&c) == Some(Ordering::Greater) by {
        git_index_stage_order_laws(&c, &b, &a);
    }
}

pub type GitIndexStages<'a> = BTreeMap<GitIndexStage<'a>, ()>;

pub open spec fn git_index_stages_ordered(keys: Seq<GitIndexStage>) -> bool {
    forall|i: int, j: int| 0 <= i < j < keys.len()
        ==> #[trigger] keys[i].order_spec(&keys[j]) == Ordering::Less
}

/// 解析完成后的只读键库存。只借用原字段；临时 BTreeMap 随解析返回释放。
pub struct GitIndexStageLookup<'a> {
    keys: Vec<GitIndexStage<'a>>,
}

impl<'a> GitIndexStageLookup<'a> {
    pub closed spec fn view(&self) -> Seq<GitIndexStage<'a>> { self.keys@ }

    pub open spec fn valid(&self) -> bool { git_index_stages_ordered(self.view()) }

    pub open spec fn contains_spec(&self, path: &str, stage: &str) -> bool {
        exists|key: GitIndexStage| (#[trigger] self.view().to_set().contains(key))
            && key.path.spec_bytes() == path.spec_bytes()
            && key.stage.spec_bytes() == stage.spec_bytes()
    }

    pub fn len(&self) -> (count: usize)
        ensures count == self.view().len(),
    { self.keys.len() }

    /// 全文二分查找；不同借用地址、UTF-8 和 NUL 都按完整原字节比较。
    pub fn contains(&self, path: &str, stage: &str) -> (found: bool)
        requires self.valid(),
        ensures found == self.contains_spec(path, stage),
    {
        let query = GitIndexStage { path, stage };
        let mut low: usize = 0;
        let mut high = self.keys.len();
        while low < high
            invariant
                0 <= low <= high <= self.keys.len(), self.valid(),
                query.path == path, query.stage == stage,
                forall|i: int| 0 <= i < low
                    ==> (#[trigger] self.keys@[i]).order_spec(&query) == Ordering::Less,
                forall|i: int| high <= i < self.keys.len()
                    ==> query.order_spec(&(#[trigger] self.keys@[i])) == Ordering::Less,
            decreases high - low,
        {
            let mid = low + (high - low) / 2;
            let order = self.keys[mid].cmp(&query);
            match order {
            Ordering::Equal => {
                proof {
                    git_index_stage_order_laws(&self.keys@[mid as int], &query, &query);
                    let key = self.keys@[mid as int];
                    assert(key.same(&query));
                    assert(key.path.spec_bytes() == path.spec_bytes());
                    assert(key.stage.spec_bytes() == stage.spec_bytes());
                    assert(self.view().to_set().contains(key));
                    assert(exists|key: GitIndexStage| (#[trigger] self.view().to_set().contains(key))
                        && key.path.spec_bytes() == path.spec_bytes()
                        && key.stage.spec_bytes() == stage.spec_bytes());
                }
                return true;
            },
            Ordering::Less => {
                proof {
                    assert forall|i: int| 0 <= i < mid + 1
                        implies (#[trigger] self.keys@[i]).order_spec(&query) == Ordering::Less by {
                        if low <= i < mid {
                            git_index_stage_order_laws(&self.keys@[i], &self.keys@[mid as int], &query);
                        }
                    }
                }
                low = mid + 1;
            },
            Ordering::Greater => {
                proof {
                    git_index_stage_order_laws(&query, &self.keys@[mid as int], &query);
                    assert forall|i: int| mid <= i < self.keys.len()
                        implies query.order_spec(&(#[trigger] self.keys@[i])) == Ordering::Less by {
                        if mid < i < high {
                            git_index_stage_order_laws(&query, &self.keys@[mid as int], &self.keys@[i]);
                        }
                    }
                }
                high = mid;
            },
            }
        }
        proof {
            if self.contains_spec(path, stage) {
                let key = choose|key: GitIndexStage| (#[trigger] self.view().to_set().contains(key))
                    && key.path.spec_bytes() == path.spec_bytes()
                    && key.stage.spec_bytes() == stage.spec_bytes();
                let i = choose|i: int| 0 <= i < self.keys.len() && self.keys@[i] == key;
                if i < low {
                    git_index_stage_order_laws(&key, &query, &query);
                } else {
                    git_index_stage_order_laws(&query, &key, &key);
                }
            }
        }
        false
    }
}

/// 标准 keys 迭代提供完整键和顺序；本函数证明复制、基数和终止。
pub fn snapshot_git_index_stages<'a>(stages: &GitIndexStages<'a>) -> (out: GitIndexStageLookup<'a>)
    ensures out.valid(), out.view().to_set() == stages@.dom(), out.view().len() == stages@.len(),
{
    proof { git_index_stage_obeys_cmp(); }
    let mut iterator = stages.keys();
    let ghost all = iterator@.1;
    proof { all.unique_seq_to_set(); }
    let mut keys: Vec<GitIndexStage<'a>> = Vec::new();
    loop
        invariant
            iterator@.1 == all, iterator@.0 == keys.len(),
            0 <= iterator@.0 <= all.len(), keys@ == all.take(iterator@.0),
            all.to_set() == stages@.dom(), all.len() == stages@.len(),
            vstd::std_specs::btree::increasing_seq(all),
        decreases all.len() - keys.len(),
    {
        match iterator.next() {
            None => {
                proof {
                    git_index_stage_obeys_cmp();
                    vstd::std_specs::btree::axiom_increasing_seq_meaning(all);
                    assert(keys@ == all);
                    assert forall|i: int, j: int| 0 <= i < j < keys.len()
                        implies #[trigger] keys@[i].order_spec(&keys@[j]) == Ordering::Less by {
                        assert(all[i].cmp_spec(&all[j]) == Ordering::Less);
                    }
                }
                return GitIndexStageLookup { keys };
            },
            Some(key) => {
                keys.push(GitIndexStage { path: key.path, stage: key.stage });
                proof { assert(keys@ =~= all.take(iterator@.0)); }
            },
        }
    }
}

fn git_index_stage_keys_have_unique_text(stages: &GitIndexStages)
    ensures forall|a: GitIndexStage, b: GitIndexStage|
        #![trigger stages@.contains_key(a), stages@.contains_key(b)]
        stages@.contains_key(a) && stages@.contains_key(b) && a.same(&b) ==> a == b,
{
    proof { git_index_stage_obeys_cmp(); }
    let iterator = stages.iter();
    proof {
        let rows = iterator@.1;
        let keys = rows.map(|i: int, row: (GitIndexStage, ())| row.0);
        vstd::std_specs::btree::axiom_increasing_seq_meaning(keys);
        assert forall|a: GitIndexStage, b: GitIndexStage|
            #![trigger stages@.contains_key(a), stages@.contains_key(b)]
            stages@.contains_key(a) && stages@.contains_key(b) && a.same(&b)
                implies a == b by {
            assert(rows.to_set().contains((a, ())));
            assert(rows.to_set().contains((b, ())));
            let i = choose|i: int| 0 <= i < rows.len() && rows[i] == (a, ());
            let j = choose|j: int| 0 <= j < rows.len() && rows[j] == (b, ());
            if i < j {
                assert(keys[i].cmp_spec(&keys[j]) is Less);
                assert(a.order_spec(&b) is Less);
            } else if j < i {
                assert(keys[j].cmp_spec(&keys[i]) is Less);
                assert(b.order_spec(&a) is Less);
            }
        }
    }
}

/// 实际插入保持原索引集合；仅完整路径和 stage 都重复时返回原错误。
pub fn record_git_index_stage<'a>(stages: &mut GitIndexStages<'a>, path: &'a str, stage: &'a str)
    -> (out: Result<(), String>)
    ensures
        stages@ == old(stages)@.insert(GitIndexStage { path, stage }, ()),
        out.is_ok() == !old(stages)@.contains_key(GitIndexStage { path, stage }),
        out.is_ok() == !(exists|key: GitIndexStage| (#[trigger] old(stages)@.contains_key(key))
            && key.path.spec_bytes() == path.spec_bytes()
            && key.stage.spec_bytes() == stage.spec_bytes()),
        out.is_err() ==> stages@ == old(stages)@,
        match out { Ok(()) => true, Err(message) => message@ == "duplicate Git index stage"@ },
{
    proof { git_index_stage_obeys_cmp(); }
    let key = GitIndexStage { path, stage };
    let ghost before = stages@;
    let previous = stages.insert(key, ());
    git_index_stage_keys_have_unique_text(stages);
    proof {
        if exists|old_key: GitIndexStage| (#[trigger] before.contains_key(old_key))
            && old_key.same(&key) {
            let old_key = choose|old_key: GitIndexStage| (#[trigger] before.contains_key(old_key))
                && old_key.same(&key);
            assert(stages@.contains_key(old_key));
            assert(stages@.contains_key(key));
            assert(old_key == key);
            assert(before.contains_key(key));
        }
        if before.contains_key(key) {
            assert(exists|old_key: GitIndexStage| (#[trigger] before.contains_key(old_key))
                && old_key.same(&key));
        }
    }
    match previous { None => Ok(()), Some(()) => Err("duplicate Git index stage".to_owned()) }
}

#[cfg(verus_keep_ghost)]
pub enum GitInventoryErrorView {
    Message(Seq<char>),
    InvalidPath(Seq<u8>),
}

pub enum GitInventoryError<'a> {
    Message(String),
    InvalidPath(&'a str),
}

impl<'a> GitInventoryError<'a> {
    pub open spec fn view(&self) -> GitInventoryErrorView {
        match self {
            Self::Message(message) => GitInventoryErrorView::Message(message@),
            Self::InvalidPath(path) => GitInventoryErrorView::InvalidPath(path.spec_bytes()),
        }
    }

    /// 唯一外部边界是标准 Debug 格式化；仓内分支及原路径转交在此证明。
    pub fn render<F: Fn(&str) -> String>(self, format_path: F) -> (out: String)
        requires forall|path: &str| format_path.requires((path,)),
        ensures match self {
            Self::Message(message) => out == message,
            Self::InvalidPath(path) => format_path.ensures((path,), out),
        },
    {
        match self {
            Self::Message(message) => message,
            Self::InvalidPath(path) => format_path(path),
        }
    }
}

pub struct GitTreeRow<'a> {
    pub path: &'a str,
    pub mode: &'a str,
    pub oid: &'a str,
    pub kind: &'a str,
}

#[cfg(verus_keep_ghost)]
pub type GitTreeRowBytes = (Seq<u8>, Seq<u8>, Seq<u8>, Seq<u8>);

impl<'a> GitTreeRow<'a> {
    pub open spec fn bytes(&self) -> GitTreeRowBytes {
        (self.path.spec_bytes(), self.mode.spec_bytes(), self.oid.spec_bytes(), self.kind.spec_bytes())
    }
}

/// 输入字节独立定义原准入次序：首 TAB、三字段、路径、完整 OID。
#[verifier::opaque]
pub open spec fn git_tree_row_spec(raw: Seq<u8>) -> Result<GitTreeRowBytes, GitInventoryErrorView> {
    let end = identifiers::ascii_separator_end(raw, 9);
    if end == raw.len() { Err(GitInventoryErrorView::Message("invalid Git tree row"@)) }
    else {
        let fields = identifiers::ascii_split_records(raw.take(end), 32, true);
        let path = raw.skip(end + 1);
        if fields.len() != 3 { Err(GitInventoryErrorView::Message("invalid Git tree metadata"@)) }
        else if !paths::relative_path(path) { Err(GitInventoryErrorView::InvalidPath(path)) }
        else if !identifiers::oid(fields[2]) {
            Err(GitInventoryErrorView::Message("expected full nonzero lowercase commit OID"@))
        } else { Ok((path, fields[0], fields[2], fields[1])) }
    }
}

pub fn parse_git_tree_row(raw: &str) -> (out: Result<GitTreeRow<'_>, GitInventoryError<'_>>)
    ensures match out {
        Ok(row) => git_tree_row_spec(raw.spec_bytes()) == Ok::<GitTreeRowBytes, GitInventoryErrorView>(row.bytes()),
        Err(error) => git_tree_row_spec(raw.spec_bytes()) == Err::<GitTreeRowBytes, GitInventoryErrorView>(error.view()),
    },
{
    proof { reveal(git_tree_row_spec); }
    let (meta, path) = match identifiers::git_inventory_row(raw, "invalid Git tree row") {
        Ok(parts) => parts,
        Err(message) => return Err(GitInventoryError::Message(message)),
    };
    let fields = identifiers::git_inventory_fields(meta);
    match identifiers::git_tree_metadata(&fields) {
        Ok(()) => {},
        Err(message) => return Err(GitInventoryError::Message(message)),
    }
    if !paths::valid_relative_path(path.as_bytes()) { return Err(GitInventoryError::InvalidPath(path)); }
    match identifiers::full_oid(fields[2]) {
        Ok(()) => {},
        Err(message) => return Err(GitInventoryError::Message(message)),
    }
    Ok(GitTreeRow { path, mode: fields[0], oid: fields[2], kind: fields[1] })
}

pub struct GitIndexRow<'a> {
    pub path: &'a str,
    pub mode: &'a str,
    pub oid: &'a str,
    pub stage: &'a str,
}

#[cfg(verus_keep_ghost)]
pub type GitIndexRowBytes = (Seq<u8>, Seq<u8>, Seq<u8>, Seq<u8>);

impl<'a> GitIndexRow<'a> {
    pub open spec fn bytes(&self) -> GitIndexRowBytes {
        (self.path.spec_bytes(), self.mode.spec_bytes(), self.oid.spec_bytes(), self.stage.spec_bytes())
    }
}

/// 实际 index 行的完整输入合同：首 TAB、三字段／单字节 stage、路径、完整 OID。
#[verifier::opaque]
pub open spec fn git_index_row_spec(raw: Seq<u8>) -> Result<GitIndexRowBytes, GitInventoryErrorView> {
    let end = identifiers::ascii_separator_end(raw, 9);
    if end == raw.len() { Err(GitInventoryErrorView::Message("invalid Git index row"@)) }
    else {
        let fields = identifiers::ascii_split_records(raw.take(end), 32, true);
        let path = raw.skip(end + 1);
        if fields.len() != 3 || fields[2].len() != 1 || fields[2][0] < 48 || fields[2][0] > 51 {
            Err(GitInventoryErrorView::Message("invalid Git index metadata"@))
        } else if !paths::relative_path(path) { Err(GitInventoryErrorView::InvalidPath(path)) }
        else if !identifiers::oid(fields[1]) {
            Err(GitInventoryErrorView::Message("expected full nonzero lowercase commit OID"@))
        } else { Ok((path, fields[0], fields[1], fields[2])) }
    }
}

pub fn parse_git_index_row(raw: &str) -> (out: Result<GitIndexRow<'_>, GitInventoryError<'_>>)
    ensures match out {
        Ok(row) => git_index_row_spec(raw.spec_bytes()) == Ok::<GitIndexRowBytes, GitInventoryErrorView>(row.bytes()),
        Err(error) => git_index_row_spec(raw.spec_bytes()) == Err::<GitIndexRowBytes, GitInventoryErrorView>(error.view()),
    },
{
    proof { reveal(git_index_row_spec); }
    let (meta, path) = match identifiers::git_inventory_row(raw, "invalid Git index row") {
        Ok(parts) => parts,
        Err(message) => return Err(GitInventoryError::Message(message)),
    };
    let fields = identifiers::git_inventory_fields(meta);
    match identifiers::git_index_metadata(&fields) {
        Ok(()) => {},
        Err(message) => return Err(GitInventoryError::Message(message)),
    }
    if !paths::valid_relative_path(path.as_bytes()) { return Err(GitInventoryError::InvalidPath(path)); }
    match identifiers::full_oid(fields[1]) {
        Ok(()) => {},
        Err(message) => return Err(GitInventoryError::Message(message)),
    }
    Ok(GitIndexRow { path, mode: fields[0], oid: fields[1], stage: fields[2] })
}

pub struct GitIndexInventory<'a> {
    pub rows: Vec<GitIndexRow<'a>>,
    pub stages: GitIndexStageLookup<'a>,
}

pub open spec fn git_index_row_at(rows: Seq<&str>, i: int) -> Result<GitIndexRowBytes, GitInventoryErrorView> {
    git_index_row_spec(rows[i].spec_bytes())
}

pub open spec fn git_index_same_stage(a: GitIndexRowBytes, b: GitIndexRowBytes) -> bool {
    a.0 == b.0 && a.3 == b.3
}

pub open spec fn git_index_prefix_valid(rows: Seq<&str>, count: int) -> bool {
    0 <= count <= rows.len()
        && (forall|i: int| 0 <= i < count ==> (#[trigger] git_index_row_at(rows, i)).is_ok())
        && (forall|i: int, j: int| 0 <= i < j < count ==>
            !git_index_same_stage((#[trigger] git_index_row_at(rows, i)->Ok_0),
                (#[trigger] git_index_row_at(rows, j)->Ok_0)))
}

/// 全部字段和顺序与输入逐项对应，不只保留 path／stage。
pub open spec fn git_index_prefix_rows(parsed: Seq<GitIndexRow>, rows: Seq<&str>, count: int) -> bool {
    parsed.len() == count && (forall|i: int| 0 <= i < count ==>
        (#[trigger] git_index_row_at(rows, i)) == Ok::<GitIndexRowBytes, GitInventoryErrorView>(parsed[i].bytes()))
}

/// 实际借用键与全部解析行双向对应；精确基数排除遗漏和额外键。
pub open spec fn git_index_inventory_stages(stages: Map<GitIndexStage, ()>, parsed: Seq<GitIndexRow>) -> bool {
    stages.len() == parsed.len()
        && (forall|key: GitIndexStage| stages.contains_key(key) ==> exists|i: int| 0 <= i < parsed.len()
            && key == GitIndexStage { path: parsed[i].path, stage: parsed[i].stage })
        && (forall|i: int| 0 <= i < parsed.len() ==> #[trigger] stages.contains_key(
            GitIndexStage { path: parsed[i].path, stage: parsed[i].stage }))
}

pub open spec fn git_index_lookup_inventory(stages: Seq<GitIndexStage>, parsed: Seq<GitIndexRow>) -> bool {
    stages.len() == parsed.len()
        && (forall|key: GitIndexStage| stages.to_set().contains(key) ==> exists|i: int| 0 <= i < parsed.len()
            && key == GitIndexStage { path: parsed[i].path, stage: parsed[i].stage })
        && (forall|i: int| 0 <= i < parsed.len() ==> #[trigger] stages.to_set().contains(
            GitIndexStage { path: parsed[i].path, stage: parsed[i].stage }))
}

pub open spec fn git_index_error_at(rows: Seq<&str>, i: int, error: GitInventoryErrorView) -> bool {
    match git_index_row_at(rows, i) {
        Err(expected) => error == expected,
        Ok(row) => error == GitInventoryErrorView::Message("duplicate Git index stage"@)
            && exists|j: int| 0 <= j < i && git_index_same_stage((git_index_row_at(rows, j)->Ok_0), row),
    }
}

pub proof fn git_index_error_blocks_prefix(rows: Seq<&str>, i: int,
    error: GitInventoryErrorView, count: int)
    requires 0 <= i < count <= rows.len(), git_index_error_at(rows, i, error),
    ensures !git_index_prefix_valid(rows, count),
{
    if git_index_prefix_valid(rows, count) {
        match git_index_row_at(rows, i) {
            Err(_) => {},
            Ok(row) => {
                let j = choose|j: int| 0 <= j < i && git_index_same_stage((git_index_row_at(rows, j)->Ok_0), row);
                assert(!git_index_same_stage((git_index_row_at(rows, j)->Ok_0), (git_index_row_at(rows, i)->Ok_0)));
            },
        }
    }
}

pub proof fn git_index_first_error_unique(rows: Seq<&str>, i: int, error: GitInventoryErrorView,
    j: int, other: GitInventoryErrorView)
    requires 0 <= i < rows.len(), git_index_prefix_valid(rows, i), git_index_error_at(rows, i, error),
        0 <= j < rows.len(), git_index_prefix_valid(rows, j), git_index_error_at(rows, j, other),
    ensures i == j, error == other,
{
    if i < j { git_index_error_blocks_prefix(rows, i, error, j); }
    if j < i { git_index_error_blocks_prefix(rows, j, other, i); }
}

proof fn git_index_next_stage_is_new(stages: Map<GitIndexStage, ()>, parsed: Seq<GitIndexRow>,
    rows: Seq<&str>, count: int, row: GitIndexRowBytes)
    requires git_index_prefix_rows(parsed, rows, count), git_index_inventory_stages(stages, parsed),
        !(exists|key: GitIndexStage| (#[trigger] stages.contains_key(key))
            && key.path.spec_bytes() == row.0 && key.stage.spec_bytes() == row.3),
    ensures forall|j: int| 0 <= j < count ==>
        !git_index_same_stage((#[trigger] git_index_row_at(rows, j)->Ok_0), row),
{
    assert forall|j: int| 0 <= j < count implies
        !git_index_same_stage((#[trigger] git_index_row_at(rows, j)->Ok_0), row) by {
        let key = GitIndexStage { path: parsed[j].path, stage: parsed[j].stage };
        assert(stages.contains_key(key));
        if git_index_same_stage((git_index_row_at(rows, j)->Ok_0), row) {
            assert(exists|key: GitIndexStage| (#[trigger] stages.contains_key(key))
                && key.path.spec_bytes() == row.0 && key.stage.spec_bytes() == row.3);
        }
    }
}

proof fn git_index_extend_valid_prefix(rows: Seq<&str>, count: int, row: GitIndexRowBytes)
    requires 0 <= count < rows.len(), git_index_prefix_valid(rows, count),
        git_index_row_at(rows, count) == Ok::<GitIndexRowBytes, GitInventoryErrorView>(row),
        forall|j: int| 0 <= j < count ==>
            !git_index_same_stage((#[trigger] git_index_row_at(rows, j)->Ok_0), row),
    ensures git_index_prefix_valid(rows, count + 1),
{
    assert forall|j: int| 0 <= j < count + 1 implies (#[trigger] git_index_row_at(rows, j)).is_ok() by {
        if j < count {} else { assert(j == count); }
    }
    assert forall|i: int, j: int| 0 <= i < j < count + 1 implies
        !git_index_same_stage((#[trigger] git_index_row_at(rows, i)->Ok_0),
            (#[trigger] git_index_row_at(rows, j)->Ok_0)) by {
        if j < count {} else { assert(j == count); }
    }
}

proof fn git_index_extend_inventory_stages(stages: Map<GitIndexStage, ()>, parsed: Seq<GitIndexRow>,
    row: GitIndexRow)
    requires git_index_inventory_stages(stages, parsed), stages.dom().finite(),
        !stages.contains_key(GitIndexStage { path: row.path, stage: row.stage }),
    ensures git_index_inventory_stages(stages.insert(GitIndexStage { path: row.path, stage: row.stage }, ()),
        parsed.push(row)),
{
    let inserted = GitIndexStage { path: row.path, stage: row.stage };
    let next = stages.insert(inserted, ());
    let all = parsed.push(row);
    assert(next.len() == all.len());
    assert forall|key: GitIndexStage| (#[trigger] next.contains_key(key)) implies exists|j: int|
        0 <= j < all.len() && key == GitIndexStage { path: all[j].path, stage: all[j].stage } by {
        if key == inserted {
            assert(key == GitIndexStage { path: all[parsed.len() as int].path, stage: all[parsed.len() as int].stage });
        } else {
            let j = choose|j: int| 0 <= j < parsed.len()
                && key == GitIndexStage { path: parsed[j].path, stage: parsed[j].stage };
            assert(key == GitIndexStage { path: all[j].path, stage: all[j].stage });
        }
    }
    assert forall|j: int| 0 <= j < all.len() implies #[trigger] next.contains_key(
        GitIndexStage { path: all[j].path, stage: all[j].stage }) by {
        if j < parsed.len() {
            assert(stages.contains_key(GitIndexStage { path: parsed[j].path, stage: parsed[j].stage }));
        } else { assert(j == parsed.len()); }
    }
}

proof fn git_index_duplicate_error(stages: Map<GitIndexStage, ()>, parsed: Seq<GitIndexRow>,
    rows: Seq<&str>, count: int, row: GitIndexRowBytes)
    requires 0 <= count < rows.len(), git_index_prefix_rows(parsed, rows, count),
        git_index_inventory_stages(stages, parsed),
        git_index_row_at(rows, count) == Ok::<GitIndexRowBytes, GitInventoryErrorView>(row),
        exists|key: GitIndexStage| (#[trigger] stages.contains_key(key))
            && key.path.spec_bytes() == row.0 && key.stage.spec_bytes() == row.3,
    ensures git_index_error_at(rows, count, GitInventoryErrorView::Message("duplicate Git index stage"@)),
{
    let key = choose|key: GitIndexStage| (#[trigger] stages.contains_key(key))
        && key.path.spec_bytes() == row.0 && key.stage.spec_bytes() == row.3;
    let j = choose|j: int| 0 <= j < parsed.len()
        && key == GitIndexStage { path: parsed[j].path, stage: parsed[j].stage };
    assert(git_index_same_stage((git_index_row_at(rows, j)->Ok_0), row));
}

/// 完整逐行解析和 stage 库存：只发布完整成功，错误保持首个失败及原字段。
pub fn parse_git_index_rows<'a>(rows: &[&'a str]) -> (out: Result<GitIndexInventory<'a>, GitInventoryError<'a>>)
    ensures match out {
        Ok(inventory) => git_index_prefix_valid(rows@, rows.len() as int)
            && git_index_prefix_rows(inventory.rows@, rows@, rows.len() as int)
            && inventory.stages.valid()
            && git_index_lookup_inventory(inventory.stages.view(), inventory.rows@),
        Err(error) => exists|i: int| 0 <= i < rows.len() && git_index_prefix_valid(rows@, i)
            && git_index_error_at(rows@, i, error.view()),
    },
{
    let mut parsed: Vec<GitIndexRow<'a>> = Vec::new();
    let mut stages = GitIndexStages::new();
    let mut i = 0;
    while i < rows.len()
        invariant
            0 <= i <= rows.len(),
            git_index_prefix_valid(rows@, i as int),
            git_index_prefix_rows(parsed@, rows@, i as int),
            git_index_inventory_stages(stages@, parsed@),
        decreases rows.len() - i,
    {
        let row = match parse_git_index_row(rows[i]) {
            Ok(row) => row,
            Err(error) => {
                proof { assert(git_index_error_at(rows@, i as int, error.view())); }
                return Err(error);
            },
        };
        let ghost before = stages@;
        proof { vstd::std_specs::btree::axiom_btree_map_view_finite_dom(stages); }
        match record_git_index_stage(&mut stages, row.path, row.stage) {
            Err(message) => {
                proof { git_index_duplicate_error(before, parsed@, rows@, i as int, row.bytes()); }
                return Err(GitInventoryError::Message(message));
            },
            Ok(()) => {},
        }
        proof {
            git_index_next_stage_is_new(before, parsed@, rows@, i as int, row.bytes());
            git_index_extend_valid_prefix(rows@, i as int, row.bytes());
            git_index_extend_inventory_stages(before, parsed@, row);
            assert(git_index_prefix_rows(parsed@.push(row), rows@, i as int + 1)) by {
                assert forall|j: int| 0 <= j < i + 1 implies
                    (#[trigger] git_index_row_at(rows@, j)) ==
                        Ok::<GitIndexRowBytes, GitInventoryErrorView>(parsed@.push(row)[j].bytes()) by {
                    if j < i {} else { assert(j == i); }
                }
            }
        }
        parsed.push(row);
        i += 1;
    }
    let lookup = snapshot_git_index_stages(&stages);
    Ok(GitIndexInventory { rows: parsed, stages: lookup })
}

/// 与原 checkout 一致：只比较 stage、mode 和 OID，不增加 kind／mode 准入。
pub fn git_index_entry_changed(row: &GitIndexRow<'_>, entry: Option<&TreeEntry>) -> (changed: bool)
    ensures changed == !(row.stage@ == "0"@ && match entry {
        Some(value) => value.mode@ == row.mode@ && value.oid@ == row.oid@,
        None => false,
    }),
{
    !identifiers::equal_text(row.stage, "0") || match entry {
        Some(value) => !identifiers::equal_text(value.mode.as_str(), row.mode)
            || !identifiers::equal_text(value.oid.as_str(), row.oid),
        None => true,
    }
}

pub type CheckoutChanges = BTreeMap<TreePath, ()>;

pub open spec fn checkout_change_has_path(changes: Map<TreePath, ()>, path: Seq<char>) -> bool {
    exists|key: TreePath| (#[trigger] changes.contains_key(key)) && key@ == path
}

pub open spec fn checkout_index_row_changed(tree: Map<TreePath, TreeEntry>, row: GitIndexRow) -> bool {
    !(row.stage@ == "0"@ && match map_entry_fields(tree, row.path@) {
        Some(fields) => fields.0 == row.mode@ && fields.1 == row.oid@,
        None => false,
    })
}

pub open spec fn checkout_index_prefix_changed(
    tree: Map<TreePath, TreeEntry>, rows: Seq<GitIndexRow>, end: int, path: Seq<char>,
) -> bool {
    exists|i: int| 0 <= i < end && (#[trigger] rows[i]).path@ == path
        && checkout_index_row_changed(tree, rows[i])
}

/// 只保存完整路径；重复标记不增加成员，原有路径始终保留。
pub fn record_checkout_owned_change(changes: &mut CheckoutChanges, path: String)
    ensures forall|text: Seq<char>| #[trigger] checkout_change_has_path(changes@, text)
        == (checkout_change_has_path(old(changes)@, text) || text == path@),
        changes@.len() == old(changes)@.len()
            + if checkout_change_has_path(old(changes)@, path@) { 0int } else { 1int },
{
    proof { tree_path_obeys_cmp(); }
    let key = TreePath::from_owned(path);
    let ghost before = changes@;
    changes.insert(key, ());
    tree_keys_have_unique_text(changes);
    proof {
        if checkout_change_has_path(before, path@) {
            let found = choose|found: TreePath| (#[trigger] before.contains_key(found)) && found@ == path@;
            assert(changes@.contains_key(found));
            assert(changes@.contains_key(key));
            assert(found == key);
            assert(before.contains_key(key));
        }
        if before.contains_key(key) { assert(checkout_change_has_path(before, path@)); }
        assert(before.contains_key(key) == checkout_change_has_path(before, path@));
        assert forall|text: Seq<char>| #[trigger] checkout_change_has_path(changes@, text)
            == (checkout_change_has_path(before, text) || text == path@) by {
            if checkout_change_has_path(changes@, text) {
                let found = choose|found: TreePath| (#[trigger] changes@.contains_key(found)) && found@ == text;
                if found != key { assert(before.contains_key(found)); }
            }
            if checkout_change_has_path(before, text) {
                let found = choose|found: TreePath| (#[trigger] before.contains_key(found)) && found@ == text;
                assert(changes@.contains_key(found));
            }
            if text == path@ { assert(changes@.contains_key(key)); }
        }
    }
}

pub fn record_checkout_change(changes: &mut CheckoutChanges, path: &str)
    ensures forall|text: Seq<char>| #[trigger] checkout_change_has_path(changes@, text)
        == (checkout_change_has_path(old(changes)@, text) || text == path@),
        changes@.len() == old(changes)@.len()
            + if checkout_change_has_path(old(changes)@, path@) { 0int } else { 1int },
{
    record_checkout_owned_change(changes, path.to_owned());
}

pub open spec fn checkout_path_prefix(paths: Seq<String>, end: int, path: Seq<char>) -> bool {
    exists|i: int| 0 <= i < end && i < paths.len() && (#[trigger] paths[i])@ == path
}

/// 完整合并两个实际库存，按全文字节排序去重；不归一化或过滤路径。
pub fn normalize_checkout_paths(first: Vec<String>, second: Vec<String>) -> (out: Vec<String>)
    ensures
        forall|path: Seq<char>| #[trigger] checkout_path_prefix(out@, out.len() as int, path)
            == checkout_path_prefix(first@ + second@, (first@.len() + second@.len()) as int, path),
        forall|i: int, j: int| 0 <= i < j < out.len()
            ==> identifiers::byte_lt(vstd::utf8::encode_utf8((#[trigger] out@[i])@),
                vstd::utf8::encode_utf8((#[trigger] out@[j])@)),
        out.len() <= first@.len() + second@.len(),
{
    let ghost all = first@ + second@;
    let mut pending = first;
    let mut additional = second;
    pending.append(&mut additional);
    let mut iterator = pending.into_iter();
    let mut changes = CheckoutChanges::new();
    loop
        invariant all == first@ + second@,
            iterator@.1 == all, 0 <= iterator@.0 <= all.len(),
            changes@.len() <= iterator@.0,
            forall|path: Seq<char>| #[trigger] checkout_change_has_path(changes@, path)
                == checkout_path_prefix(all, iterator@.0, path),
        decreases all.len() - iterator@.0,
    {
        let ghost at = iterator@.0;
        match iterator.next() {
            None => {
                assert(iterator@.0 == all.len());
                let out = checkout_changed_paths(&changes);
                proof {
                    assert forall|text: Seq<char>|
                        #[trigger] checkout_path_prefix(out@, out.len() as int, text)
                            == checkout_path_prefix(all, all.len() as int, text) by {
                        assert(checkout_change_has_path(changes@, text)
                            == checkout_path_prefix(all, all.len() as int, text));
                    }
                }
                return out;
            },
            Some(path) => {
                record_checkout_owned_change(&mut changes, path);
                proof {
                    assert forall|text: Seq<char>| #[trigger] checkout_change_has_path(changes@, text)
                        == checkout_path_prefix(all, iterator@.0, text) by {
                        assert(checkout_path_prefix(all, at + 1, text)
                            == (checkout_path_prefix(all, at, text) || all[at]@ == text));
                    }
                }
            },
        }
    }
}

/// 结果完整且按 UTF-8 字节严格排序；不把 map 中的键遗漏或重复输出。
pub fn checkout_changed_paths(changes: &CheckoutChanges) -> (paths: Vec<String>)
    ensures
        forall|text: Seq<char>| #[trigger] checkout_change_has_path(changes@, text)
            == (exists|i: int| 0 <= i < paths.len() && (#[trigger] paths@[i])@ == text),
        forall|i: int, j: int| 0 <= i < j < paths.len()
            ==> identifiers::byte_lt(vstd::utf8::encode_utf8((#[trigger] paths@[i])@),
                vstd::utf8::encode_utf8((#[trigger] paths@[j])@)),
        paths.len() == changes@.len(),
{
    proof { tree_path_obeys_cmp(); }
    let mut iterator = changes.keys();
    let ghost all = iterator@.1;
    proof { all.unique_seq_to_set(); }
    let mut paths: Vec<String> = Vec::new();
    loop
        invariant
            iterator@.1 == all, iterator@.0 == paths.len(),
            0 <= iterator@.0 <= all.len(), all.to_set() == changes@.dom(),
            all.len() == changes@.len(),
            vstd::std_specs::btree::increasing_seq(all),
            forall|i: int| 0 <= i < paths.len() ==> (#[trigger] paths@[i])@ == all[i]@,
        decreases all.len() - paths.len(),
    {
        match iterator.next() {
            Some(key) => { paths.push(key.text.clone()); },
            None => {
                proof {
                    tree_path_obeys_cmp();
                    vstd::std_specs::btree::axiom_increasing_seq_meaning(all);
                    assert forall|text: Seq<char>| #[trigger] checkout_change_has_path(changes@, text)
                        == (exists|i: int| 0 <= i < paths.len() && (#[trigger] paths@[i])@ == text) by {
                        if checkout_change_has_path(changes@, text) {
                            let key = choose|key: TreePath| (#[trigger] changes@.contains_key(key)) && key@ == text;
                            assert(all.contains(key));
                            let i = all.lemma_contains_to_index(key);
                            assert(paths@[i]@ == text);
                        }
                        if exists|i: int| 0 <= i < paths.len() && (#[trigger] paths@[i])@ == text {
                            let i = choose|i: int| 0 <= i < paths.len() && (#[trigger] paths@[i])@ == text;
                            assert(all.to_set().contains(all[i]));
                            assert(changes@.contains_key(all[i]));
                        }
                    }
                    assert forall|i: int, j: int| 0 <= i < j < paths.len()
                        implies identifiers::byte_lt(vstd::utf8::encode_utf8((#[trigger] paths@[i])@),
                            vstd::utf8::encode_utf8((#[trigger] paths@[j])@)) by {
                        assert(all[i].cmp_spec(&all[j]) is Less);
                        assert(all[i].order_spec(&all[j]) is Less);
                    }
                }
                return paths;
            },
        }
    }
}

proof fn tree_rows_entry_at(rows: Seq<(&TreePath, &TreeEntry)>, index: int)
    requires tree_rows_ordered(rows), 0 <= index < rows.len(),
    ensures rows_entry(rows, 0, rows[index].0@) == Some(tree_entry_fields(rows[index].1)),
{
    rows_entry_witness(rows, 0, rows[index].0@);
    assert(rows_has_path(rows, 0, rows[index].0@));
    let fields = rows_entry(rows, 0, rows[index].0@)->Some_0;
    let found = choose|found: int| 0 <= found < rows.len()
        && rows[found].0@ == rows[index].0@ && tree_entry_fields(rows[found].1) == fields;
    if found < index { identifiers::byte_lt_strict(rows[found].0.bytes(), rows[index].0.bytes()); }
    if index < found { identifiers::byte_lt_strict(rows[index].0.bytes(), rows[found].0.bytes()); }
    assert(found == index);
}

/// 查询实际树的借用库存；不采用缺少全文解释的异型借用 map 查询。
fn lookup_tree_rows<'a>(rows: &Vec<(&'a TreePath, &'a TreeEntry)>, path: &str)
    -> (entry: Option<&'a TreeEntry>)
    requires tree_rows_ordered(rows@),
    ensures optional_entry_fields(entry) == rows_entry(rows@, 0, path@),
{
    let mut low: usize = 0;
    let mut high = rows.len();
    while low < high
        invariant 0 <= low <= high <= rows.len(), tree_rows_ordered(rows@),
            forall|i: int| 0 <= i < low
                ==> identifiers::byte_lt((#[trigger] rows@[i]).0.bytes(), path.spec_bytes()),
            forall|i: int| high <= i < rows.len()
                ==> identifiers::byte_lt(path.spec_bytes(), (#[trigger] rows@[i]).0.bytes()),
        decreases high - low,
    {
        let mid = low + (high - low) / 2;
        let order = identifiers::compare_bytes(rows[mid].0.as_str().as_bytes(), path.as_bytes());
        if order == 0 {
            proof {
                vstd::utf8::encode_utf8_decode_utf8(rows@[mid as int].0@);
                vstd::utf8::encode_utf8_decode_utf8(path@);
                assert(rows@[mid as int].0@ == path@);
                tree_rows_entry_at(rows@, mid as int);
            }
            return Some(rows[mid].1);
        } else if order < 0 {
            proof {
                assert forall|i: int| 0 <= i < mid + 1
                    implies identifiers::byte_lt((#[trigger] rows@[i]).0.bytes(), path.spec_bytes()) by {
                    if low <= i < mid {
                        identifiers::byte_lt_transitive(rows@[i].0.bytes(), rows@[mid as int].0.bytes(), path.spec_bytes());
                    }
                }
            }
            low = mid + 1;
        } else {
            proof {
                assert forall|i: int| mid <= i < rows.len()
                    implies identifiers::byte_lt(path.spec_bytes(), (#[trigger] rows@[i]).0.bytes()) by {
                    if mid < i < high {
                        identifiers::byte_lt_transitive(path.spec_bytes(), rows@[mid as int].0.bytes(), rows@[i].0.bytes());
                    }
                }
            }
            high = mid;
        }
    }
    proof {
        assert forall|i: int| 0 <= i < rows.len() implies (#[trigger] rows@[i]).0@ != path@ by {
            if i < low { identifiers::byte_lt_strict(rows@[i].0.bytes(), path.spec_bytes()); }
            else { identifiers::byte_lt_strict(path.spec_bytes(), rows@[i].0.bytes()); }
        }
        rows_entry_absent(rows@, 0, path@);
    }
    None
}

/// 真实 index 的每一行参与判断；changed 集合精确包含至少有一条变化行的完整路径。
pub fn checkout_index_changes(tree: &Tree, rows: &[GitIndexRow<'_>]) -> (changes: CheckoutChanges)
    ensures forall|path: Seq<char>| #[trigger] checkout_change_has_path(changes@, path)
        == checkout_index_prefix_changed(tree@, rows@, rows.len() as int, path),
{
    let inventory = tree_rows(tree);
    let mut changes = CheckoutChanges::new();
    let mut i: usize = 0;
    while i < rows.len()
        invariant 0 <= i <= rows.len(), tree_rows_ordered(inventory@),
            tree_rows_view(inventory@).to_set() == tree@.kv_pairs(),
            forall|path: Seq<char>| #[trigger] checkout_change_has_path(changes@, path)
                == checkout_index_prefix_changed(tree@, rows@, i as int, path),
        decreases rows.len() - i,
    {
        let entry = lookup_tree_rows(&inventory, rows[i].path);
        proof { map_rows_entry(tree@, inventory@, rows@[i as int].path@); }
        let changed = git_index_entry_changed(&rows[i], entry);
        let ghost before = changes@;
        if changed { record_checkout_change(&mut changes, rows[i].path); }
        proof {
            assert(changed == checkout_index_row_changed(tree@, rows@[i as int]));
            assert forall|path: Seq<char>| #[trigger] checkout_change_has_path(changes@, path)
                == checkout_index_prefix_changed(tree@, rows@, i + 1, path) by {
                assert(checkout_index_prefix_changed(tree@, rows@, i + 1, path)
                    == (checkout_index_prefix_changed(tree@, rows@, i as int, path)
                        || (rows@[i as int].path@ == path && changed)));
            }
        }
        i += 1;
    }
    changes
}

pub open spec fn checkout_stage_zero(stages: Seq<GitIndexStage>, path: Seq<u8>) -> bool {
    exists|key: GitIndexStage| (#[trigger] stages.to_set().contains(key))
        && key.path.spec_bytes() == path && key.stage.spec_bytes() == "0".spec_bytes()
}

pub open spec fn checkout_missing_physical_prefix(
    rows: Seq<(&TreePath, &TreeEntry)>, stages: Seq<GitIndexStage>, end: int, path: Seq<char>,
) -> bool {
    exists|i: int| 0 <= i < end && (#[trigger] rows[i]).0@ == path
        && !checkout_stage_zero(stages, rows[i].0.bytes())
}

/// 缺 stage 0 的路径只记变化；其余原条目按完整路径顺序交给实际文件观察。
pub fn checkout_physical_targets<'a>(
    tree: &'a Tree, stages: &GitIndexStageLookup, changes: &mut CheckoutChanges,
) -> (targets: Vec<(&'a TreePath, &'a TreeEntry)>)
    requires stages.valid(),
    ensures
        tree_rows_ordered(targets@),
        tree_rows_view(targets@).no_duplicates(),
        targets.len() == tree_rows_view(targets@).to_set().len(),
        tree_rows_view(targets@).to_set() == tree@.kv_pairs().filter(
            |row: (TreePath, TreeEntry)| checkout_stage_zero(stages@, row.0.bytes())),
        forall|path: Seq<char>| #[trigger] checkout_change_has_path(changes@, path)
            == (checkout_change_has_path(old(changes)@, path)
                || exists|row: (TreePath, TreeEntry)| (#[trigger] tree@.kv_pairs().contains(row))
                    && row.0@ == path && !checkout_stage_zero(stages@, row.0.bytes())),
{
    let rows = tree_rows(tree);
    let ghost before = changes@;
    let mut targets: Vec<(&TreePath, &TreeEntry)> = Vec::new();
    let mut i: usize = 0;
    while i < rows.len()
        invariant
            0 <= i <= rows.len(), stages.valid(), tree_rows_ordered(rows@),
            tree_rows_view(rows@).to_set() == tree@.kv_pairs(),
            tree_rows_ordered(targets@),
            forall|j: int| #![trigger targets@[j]] 0 <= j < targets.len() ==> exists|k: int|
                0 <= k < i && targets@[j] == rows@[k],
            tree_rows_view(targets@).to_set() == tree_rows_view(rows@.take(i as int)).to_set().filter(
                |row: (TreePath, TreeEntry)| checkout_stage_zero(stages@, row.0.bytes())),
            forall|path: Seq<char>| #[trigger] checkout_change_has_path(changes@, path)
                == (checkout_change_has_path(before, path)
                    || checkout_missing_physical_prefix(rows@, stages@, i as int, path)),
        decreases rows.len() - i,
    {
        let (path, entry) = rows[i];
        let probe = stages.contains(path.as_str(), "0");
        let ghost prior = targets@;
        if probe {
            proof {
                assert forall|j: int| 0 <= j < targets.len()
                    implies identifiers::byte_lt((#[trigger] targets@[j]).0.bytes(), path.bytes()) by {
                    let k = choose|k: int| 0 <= k < i && targets@[j] == rows@[k];
                    assert(identifiers::byte_lt(rows@[k].0.bytes(), rows@[i as int].0.bytes()));
                }
            }
            targets.push((path, entry));
        } else {
            record_checkout_change(changes, path.as_str());
        }
        proof {
            assert(probe == checkout_stage_zero(stages@, path.bytes()));
            assert(tree_rows_view(targets@) =~= if probe {
                tree_rows_view(prior).push((*path, *entry))
            } else { tree_rows_view(prior) });
            assert(tree_rows_view(rows@.take(i + 1))
                =~= tree_rows_view(rows@.take(i as int)).push((*path, *entry)));
            tree_rows_view(rows@.take(i as int)).lemma_push_to_set_commute((*path, *entry));
            tree_rows_view(prior).lemma_push_to_set_commute((*path, *entry));
            assert(tree_rows_view(targets@).to_set()
                =~= tree_rows_view(rows@.take(i + 1)).to_set().filter(
                    |row: (TreePath, TreeEntry)| checkout_stage_zero(stages@, row.0.bytes()))) by {
                assert forall|row: (TreePath, TreeEntry)|
                    #[trigger] tree_rows_view(targets@).to_set().contains(row)
                    == tree_rows_view(rows@.take(i + 1)).to_set().filter(
                        |row: (TreePath, TreeEntry)| checkout_stage_zero(stages@, row.0.bytes())).contains(row) by {
                    assert(tree_rows_view(targets@).to_set().contains(row)
                        == (tree_rows_view(prior).to_set().contains(row) || (probe && row == (*path, *entry))));
                    assert(tree_rows_view(rows@.take(i + 1)).to_set().contains(row)
                        == (tree_rows_view(rows@.take(i as int)).to_set().contains(row) || row == (*path, *entry)));
                }
            }
            assert forall|j: int| #![trigger targets@[j]] 0 <= j < targets.len()
                implies exists|k: int| 0 <= k < i + 1 && targets@[j] == rows@[k] by {
                if j < prior.len() {
                    let k = choose|k: int| 0 <= k < i && prior[j] == rows@[k];
                    assert(targets@[j] == rows@[k]);
                } else { assert(j == prior.len() && targets@[j] == rows@[i as int]); }
            }
            assert forall|text: Seq<char>| #[trigger] checkout_change_has_path(changes@, text)
                == (checkout_change_has_path(before, text)
                    || checkout_missing_physical_prefix(rows@, stages@, i + 1, text)) by {
                assert(checkout_missing_physical_prefix(rows@, stages@, i + 1, text)
                    == (checkout_missing_physical_prefix(rows@, stages@, i as int, text)
                        || (path@ == text && !probe)));
            }
        }
        i += 1;
    }
    proof {
        assert(rows@.take(rows.len() as int) =~= rows@);
        assert(tree_rows_view(targets@).to_set() == tree@.kv_pairs().filter(
            |row: (TreePath, TreeEntry)| checkout_stage_zero(stages@, row.0.bytes())));
        assert forall|i: int, j: int| 0 <= i < j < targets.len()
            implies tree_rows_view(targets@)[i] != tree_rows_view(targets@)[j] by {
            identifiers::byte_lt_strict(targets@[i].0.bytes(), targets@[j].0.bytes());
        }
        tree_rows_view(targets@).unique_seq_to_set();
        assert forall|path: Seq<char>| checkout_missing_physical_prefix(rows@, stages@, rows.len() as int, path)
            == (exists|row: (TreePath, TreeEntry)| (#[trigger] tree@.kv_pairs().contains(row))
                && row.0@ == path && !checkout_stage_zero(stages@, row.0.bytes())) by {
            if checkout_missing_physical_prefix(rows@, stages@, rows.len() as int, path) {
                let k = choose|k: int| 0 <= k < rows.len() && rows@[k].0@ == path
                    && !checkout_stage_zero(stages@, rows@[k].0.bytes());
                assert(tree_rows_view(rows@).contains(tree_rows_view(rows@)[k]));
                assert(tree_rows_view(rows@)[k] == (*rows@[k].0, *rows@[k].1));
                assert(tree@.kv_pairs().contains(tree_rows_view(rows@)[k]));
            } else if exists|row: (TreePath, TreeEntry)| (#[trigger] tree@.kv_pairs().contains(row))
                && row.0@ == path && !checkout_stage_zero(stages@, row.0.bytes()) {
                let row = choose|row: (TreePath, TreeEntry)| (#[trigger] tree@.kv_pairs().contains(row))
                    && row.0@ == path && !checkout_stage_zero(stages@, row.0.bytes());
                assert(tree_rows_view(rows@).contains(row));
                let k = tree_rows_view(rows@).lemma_contains_to_index(row);
                assert(checkout_missing_physical_prefix(rows@, stages@, rows.len() as int, path));
            }
        }
    }
    targets
}

pub struct CheckoutPhysicalObservation<'a> {
    pub path: &'a str,
    pub matches: bool,
}

pub open spec fn checkout_failed_physical_prefix(
    observations: Seq<CheckoutPhysicalObservation>, end: int, path: Seq<char>,
) -> bool {
    exists|i: int| 0 <= i < end && (#[trigger] observations[i]).path@ == path
        && !observations[i].matches
}

/// 精确加入观察不匹配的原路径；匹配、空输入及重复都不清除既有变化。
pub fn record_checkout_physical_observations(
    changes: &mut CheckoutChanges, observations: &[CheckoutPhysicalObservation],
)
    ensures forall|path: Seq<char>| #[trigger] checkout_change_has_path(changes@, path)
        == (checkout_change_has_path(old(changes)@, path)
            || checkout_failed_physical_prefix(observations@, observations.len() as int, path)),
{
    let ghost before = changes@;
    let mut i: usize = 0;
    while i < observations.len()
        invariant 0 <= i <= observations.len(),
            forall|path: Seq<char>| #[trigger] checkout_change_has_path(changes@, path)
                == (checkout_change_has_path(before, path)
                    || checkout_failed_physical_prefix(observations@, i as int, path)),
        decreases observations.len() - i,
    {
        if !observations[i].matches { record_checkout_change(changes, observations[i].path); }
        proof {
            assert forall|path: Seq<char>| #[trigger] checkout_change_has_path(changes@, path)
                == (checkout_change_has_path(before, path)
                    || checkout_failed_physical_prefix(observations@, i + 1, path)) by {
                assert(checkout_failed_physical_prefix(observations@, i + 1, path)
                    == (checkout_failed_physical_prefix(observations@, i as int, path)
                        || (observations@[i as int].path@ == path && !observations@[i as int].matches)));
            }
        }
        i += 1;
    }
}

/// 外部接口只观察错误种类；成功保留原元数据，缺失转 None，其余错误原样转交。
pub fn checkout_metadata_observation<T, E, F>(observation: Result<T, E>, is_missing: F)
    -> (out: Result<Option<T>, E>)
    where F: Fn(&E) -> bool,
    requires match observation {
        Ok(_) => true,
        Err(error) => is_missing.requires((&error,)),
    },
    ensures match observation {
        Ok(metadata) => out == Ok::<Option<T>, E>(Some(metadata)),
        Err(error) => match out {
            Ok(None) => is_missing.ensures((&error,), true),
            Err(actual) => actual == error && is_missing.ensures((&error,), false),
            _ => false,
        },
    },
{
    match observation {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) => {
            if is_missing(&error) { Ok(None) }
            else { Err(error) }
        },
    }
}

/// 父路径只在真实目录观察为真时允许继续；缺失和非目录均停止，其他错误保留。
pub fn checkout_parent_observation<E, F>(observation: Result<bool, E>, is_missing: F)
    -> (out: Result<bool, E>)
    where F: Fn(&E) -> bool,
    requires match observation {
        Ok(_) => true,
        Err(error) => is_missing.requires((&error,)),
    },
    ensures match observation {
        Ok(is_directory) => out == Ok::<bool, E>(is_directory),
        Err(error) => match out {
            Ok(false) => is_missing.ensures((&error,), true),
            Err(actual) => actual == error && is_missing.ensures((&error,), false),
            _ => false,
        },
    },
{
    match checkout_metadata_observation(observation, is_missing) {
        Ok(Some(is_directory)) => Ok(is_directory),
        Ok(None) => Ok(false),
        Err(error) => Err(error),
    }
}

pub open spec fn checkout_components_finite<S, C, N>(next: N, rank: spec_fn(S) -> nat) -> bool
    where N: Fn(S) -> (S, Option<C>),
{
    paths::component_steps_finite::<S, C, N>(next, rank)
}

#[verifier::opaque]
pub open spec fn checkout_physical_step<S, P, C, E, N, A, M>(
    before: (S, P, C), after: (S, P, C), next: N, append: A, metadata: M,
) -> bool
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>),
{
    exists|built: P| append.ensures((before.1, before.2), built)
        && next.ensures((before.0,), (after.0, Some(after.2)))
        && metadata.ensures((built,), (after.1, Ok::<bool, E>(true)))
}

pub open spec fn checkout_physical_prefix<S, P, C, E, N, A, M>(
    states: Seq<(S, P, C)>, first: (S, P, C), next: N, append: A, metadata: M,
) -> bool
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>),
{
    states.len() > 0 && states[0] == first
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] checkout_physical_step::<S, P, C, E, N, A, M>(
                states[i], states[i + 1], next, append, metadata)
}

pub open spec fn checkout_physical_terminal<S, P, C, E, N, A, M, F>(
    before: (S, P, C), built: P, following: (S, Option<C>),
    observed: Option<(P, Result<bool, E>)>,
    next: N, append: A, metadata: M, is_missing: F, out: Result<Option<P>, E>,
) -> bool
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>), F: Fn(&E) -> bool,
{
    append.ensures((before.1, before.2), built)
        && next.ensures((before.0,), following)
        && match following.1 {
            None => observed.is_none() && out == Ok::<Option<P>, E>(Some(built)),
            Some(_) => match observed {
                None => false,
                Some(observed) => metadata.ensures((built,), observed)
                    && match observed.1 {
                        Ok(false) => out == Ok::<Option<P>, E>(None),
                        Ok(true) => false,
                        Err(error) => (out == Ok::<Option<P>, E>(None)
                                && is_missing.ensures((&error,), true))
                            || (out == Err::<Option<P>, E>(error)
                                && is_missing.ensures((&error,), false)),
                    },
            },
        }
}

pub open spec fn checkout_physical_result<S, P, C, E, N, A, M, F>(
    initial: S, root: P, next: N, append: A, metadata: M, is_missing: F,
    out: Result<Option<P>, E>,
) -> bool
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>), F: Fn(&E) -> bool,
{
    exists|first: (S, Option<C>)| next.ensures((initial,), first)
        && match first.1 {
            None => out == Ok::<Option<P>, E>(Some(root)),
            Some(component) => exists|states: Seq<(S, P, C)>, built: P, following: (S, Option<C>), observed: Option<(P, Result<bool, E>)>|
                checkout_physical_prefix::<S, P, C, E, N, A, M>(states,
                    (first.0, root, component), next, append, metadata)
                    && checkout_physical_terminal(states.last(), built, following, observed, next, append, metadata, is_missing, out),
        }
}

/// 逐组件构造路径；只查询父路径，首个非目录、缺失或错误立即终止。
pub fn checkout_physical_path<S, P, C, E, N, A, M, F>(
    initial: S, root: P, next: N, append: A, metadata: M, is_missing: F,
) -> (out: Result<Option<P>, E>)
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>), F: Fn(&E) -> bool,
    requires
        forall|source: S| #[trigger] next.requires((source,)),
        exists|rank: spec_fn(S) -> nat| checkout_components_finite::<S, C, N>(next, rank),
        forall|path: P, component: C| #[trigger] append.requires((path, component)),
        forall|path: P| #[trigger] metadata.requires((path,)),
        forall|error: &E| #[trigger] is_missing.requires((error,)),
    ensures checkout_physical_result(initial, root, next, append, metadata, is_missing, out),
{
    let ghost rank = choose|rank: spec_fn(S) -> nat| checkout_components_finite::<S, C, N>(next, rank);
    let first_observation = next(initial);
    let ghost first = first_observation;
    let (mut source, initial_part) = first_observation;
    let mut component = match initial_part {
        None => {
            assert(checkout_physical_result(initial, root, next, append, metadata, is_missing, Ok(Some(root))));
            return Ok(Some(root));
        },
        Some(part) => part,
    };
    let ghost first_state = (source, root, component);
    let ghost states = seq![first_state];
    let mut path = root;
    loop
        invariant
            next.ensures((initial,), first), first.0 == first_state.0,
            first.1 == Some(first_state.2), first_state.1 == root,
            states.last() == (source, path, component),
            checkout_physical_prefix::<S, P, C, E, N, A, M>(states, first_state, next, append, metadata),
            checkout_components_finite::<S, C, N>(next, rank),
            forall|source: S| #[trigger] next.requires((source,)),
            forall|path: P, component: C| #[trigger] append.requires((path, component)),
            forall|path: P| #[trigger] metadata.requires((path,)),
            forall|error: &E| #[trigger] is_missing.requires((error,)),
        decreases rank(source),
    {
        path = append(path, component);
        let ghost built = path;
        let following = next(source);
        let ghost following_observation = following;
        let (after, following_part) = following;
        source = after;
        component = match following_part {
            None => {
                assert(checkout_physical_terminal(states.last(), built, following_observation, None, next, append, metadata, is_missing, Ok(Some(path))));
                assert(checkout_physical_result(initial, root, next, append, metadata, is_missing, Ok(Some(path))));
                return Ok(Some(path));
            },
            Some(part) => part,
        };
        let observed = metadata(path);
        let ghost original_observation = observed;
        let (after_path, observation) = observed;
        path = after_path;
        let permitted = checkout_parent_observation(observation,
            |error: &E| -> (missing: bool)
                ensures is_missing.ensures((error,), missing),
            { is_missing(error) });
        match permitted {
            Err(error) => {
                assert(checkout_physical_terminal(states.last(), built, following_observation, Some(original_observation), next, append, metadata, is_missing, Err(error)));
                assert(checkout_physical_result(initial, root, next, append, metadata, is_missing, Err(error)));
                return Err(error);
            },
            Ok(false) => {
                assert(checkout_physical_terminal(states.last(), built, following_observation, Some(original_observation), next, append, metadata, is_missing, Ok(None)));
                assert(checkout_physical_result(initial, root, next, append, metadata, is_missing, Ok(None)));
                return Ok(None);
            },
            Ok(true) => {},
        }
        let ghost after = (source, path, component);
        assert(original_observation.1 == Ok::<bool, E>(true));
        assert(append.ensures((states.last().1, states.last().2), built));
        assert(next.ensures((states.last().0,), (source, Some(component))));
        assert(metadata.ensures((built,), (path, Ok::<bool, E>(true))));
        assert(checkout_physical_step::<S, P, C, E, N, A, M>(states.last(), after, next, append, metadata)) by {
            reveal(checkout_physical_step);
        }
        proof {
            let previous = states;
            states = states.push(after);
            assert forall|i: int| 0 <= i < states.len() - 1 implies
                #[trigger] checkout_physical_step::<S, P, C, E, N, A, M>(states[i], states[i + 1], next, append, metadata) by {
                if i < previous.len() - 1 {
                    assert(checkout_physical_step::<S, P, C, E, N, A, M>(previous[i], previous[i + 1], next, append, metadata));
                }
            }
        }
    }
}


/// checkout 只支持原有三种 blob 模式，完整文本决定支持与否。
pub fn checkout_entry_supported(kind: &str, mode: &str) -> (supported: bool)
    ensures supported == (kind@ == "blob"@
        && (mode@ == "100644"@ || mode@ == "100755"@ || mode@ == "120000"@)),
{
    identifiers::equal_text(kind, "blob") && (identifiers::equal_text(mode, "100644")
        || identifiers::equal_text(mode, "100755") || identifiers::equal_text(mode, "120000"))
}

pub fn checkout_symbolic_link(mode: &str) -> (symbolic: bool)
    ensures symbolic == (mode@ == "120000"@),
{ identifiers::equal_text(mode, "120000") }

/// 与原消费一致：只核普通文件与 owner execute 位，不增加其他权限准入。
pub fn checkout_regular_file_matches(is_file: bool, permissions: u32, mode: &str) -> (matches: bool)
    ensures matches == (is_file && ((permissions & 0o100 != 0) == (mode@ == "100755"@))),
{
    is_file && ((permissions & 0o100 != 0) == identifiers::equal_text(mode, "100755"))
}

/// 打开后必须仍是普通文件，长度和完整权限字必须与打开前相同。
pub fn checkout_opened_file_matches(
    is_file: bool, opened_size: u64, opened_permissions: u32,
    observed_size: u64, observed_permissions: u32,
) -> (matches: bool)
    ensures matches == (is_file && opened_size == observed_size
        && opened_permissions == observed_permissions),
{
    is_file && opened_size == observed_size && opened_permissions == observed_permissions
}


/// 同一次文件系统元数据的原始字段；判断由条目匹配流程拥有。
pub struct CheckoutEntryMetadata {
    pub symbolic: bool,
    pub regular: bool,
    pub size: u64,
    pub permissions: u32,
}

/// 实际 IO、已有生产入口及原错误格式；未选中分支无需可调用。
pub struct CheckoutEntryIo<L, M, FE, R, HL, O, FM, HF, C> {
    pub locate: L,
    pub metadata: M,
    pub format_metadata_error: FE,
    pub read_link: R,
    pub hash_link: HL,
    pub open: O,
    pub opened_metadata: FM,
    pub hash_file: HF,
    pub changed_error: C,
}

pub open spec fn checkout_digest_result(oid: Seq<char>, digest: Result<String, String>, out: Result<bool, String>) -> bool {
    match digest {
        Err(error) => out == Err::<bool, String>(error),
        Ok(text) => out == Ok::<bool, String>(text@ == oid),
    }
}

pub fn checkout_digest_matches(oid: &str, digest: Result<String, String>) -> (out: Result<bool, String>)
    ensures checkout_digest_result(oid@, digest, out),
{
    match digest {
        Err(error) => Err(error),
        Ok(text) => Ok(identifiers::equal_text(text.as_str(), oid)),
    }
}

pub open spec fn checkout_entry_opened_result<F, FM, HF, C>(
    oid: Seq<char>, observed: CheckoutEntryMetadata, opened: Result<F, String>,
    metadata: FM, hash: HF, changed: C, out: Result<bool, String>,
) -> bool
    where FM: Fn(&F) -> Result<CheckoutEntryMetadata, String>,
          HF: Fn(u64, F) -> Result<String, String>, C: Fn() -> String,
{
    match opened {
        Err(error) => out == Err::<bool, String>(error),
        Ok(file) => metadata.requires((&file,)) && (exists|actual: Result<CheckoutEntryMetadata, String>|
            metadata.ensures((&file,), actual) && match actual {
                Err(error) => out == Err::<bool, String>(error),
                Ok(fields) => {
                    if fields.regular && fields.size == observed.size && fields.permissions == observed.permissions {
                        hash.requires((fields.size, file)) && (exists|digest: Result<String, String>| hash.ensures((fields.size, file), digest)
                            && checkout_digest_result(oid, digest, out))
                    } else {
                        changed.requires(()) && (exists|error: String| changed.ensures((), error) && out == Err::<bool, String>(error))
                    }
                },
            }),
    }
}

pub open spec fn checkout_entry_observed_result<P, F, B, E, FE, R, HL, O, FM, HF, C>(
    mode: Seq<char>, oid: Seq<char>, path: P,
    observed: Result<CheckoutEntryMetadata, (bool, E)>, format_error: FE,
    read_link: R, hash_link: HL, open: O, opened_metadata: FM, hash_file: HF,
    changed_error: C, out: Result<bool, String>,
) -> bool
    where FE: Fn(E) -> String, R: Fn(P) -> Result<B, String>,
          HL: Fn(B) -> Result<String, String>, O: Fn(P) -> Result<F, String>,
          FM: Fn(&F) -> Result<CheckoutEntryMetadata, String>,
          HF: Fn(u64, F) -> Result<String, String>, C: Fn() -> String,
{
    match observed {
        Err((missing, error)) => {
            if missing { out == Ok::<bool, String>(false) }
            else { format_error.requires((error,)) && (exists|message: String| format_error.ensures((error,), message)
                && out == Err::<bool, String>(message)) }
        },
        Ok(fields) => {
            if mode == "120000"@ {
                if !fields.symbolic { out == Ok::<bool, String>(false) }
                else {
                    read_link.requires((path,)) && (exists|target: Result<B, String>| read_link.ensures((path,), target) && match target {
                        Err(error) => out == Err::<bool, String>(error),
                        Ok(bytes) => hash_link.requires((bytes,)) && (exists|digest: Result<String, String>|
                            hash_link.ensures((bytes,), digest) && checkout_digest_result(oid, digest, out)),
                    })
                }
            } else if !(fields.regular && ((fields.permissions & 0o100 != 0) == (mode == "100755"@))) {
                out == Ok::<bool, String>(false)
            } else {
                open.requires((path,)) && (exists|opened: Result<F, String>| open.ensures((path,), opened)
                    && checkout_entry_opened_result(oid, fields, opened, opened_metadata, hash_file, changed_error, out))
            }
        },
    }
}

pub open spec fn checkout_entry_matches_result<P, F, B, E, L, M, FE, R, HL, O, FM, HF, C>(
    kind: Seq<char>, mode: Seq<char>, oid: Seq<char>,
    io: CheckoutEntryIo<L, M, FE, R, HL, O, FM, HF, C>, out: Result<bool, String>,
) -> bool
    where L: Fn() -> Result<Option<P>, String>,
          M: Fn(&P) -> Result<CheckoutEntryMetadata, (bool, E)>, FE: Fn(E) -> String,
          R: Fn(P) -> Result<B, String>, HL: Fn(B) -> Result<String, String>,
          O: Fn(P) -> Result<F, String>, FM: Fn(&F) -> Result<CheckoutEntryMetadata, String>,
          HF: Fn(u64, F) -> Result<String, String>, C: Fn() -> String,
{
    if kind != "blob"@ || !(mode == "100644"@ || mode == "100755"@ || mode == "120000"@) {
        out == Ok::<bool, String>(false)
    } else {
        io.locate.requires(()) && (exists|located: Result<Option<P>, String>| io.locate.ensures((), located) && match located {
            Err(error) => out == Err::<bool, String>(error),
            Ok(None) => out == Ok::<bool, String>(false),
            Ok(Some(path)) => io.metadata.requires((&path,)) && (exists|observed: Result<CheckoutEntryMetadata, (bool, E)>|
                io.metadata.ensures((&path,), observed)
                && checkout_entry_observed_result(mode, oid, path, observed, io.format_metadata_error,
                    io.read_link, io.hash_link, io.open, io.opened_metadata,
                    io.hash_file, io.changed_error, out)),
        })
    }
}

pub open spec fn checkout_entry_opened_callable<F, FM, HF, C>(
    observed: CheckoutEntryMetadata, file: F, metadata: FM, hash: HF, changed: C,
) -> bool
    where FM: Fn(&F) -> Result<CheckoutEntryMetadata, String>,
          HF: Fn(u64, F) -> Result<String, String>, C: Fn() -> String,
{
    metadata.requires((&file,)) && (forall|actual: Result<CheckoutEntryMetadata, String>|
        #[trigger] metadata.ensures((&file,), actual) ==> match actual {
            Err(_) => true,
            Ok(fields) => {
                if fields.regular && fields.size == observed.size && fields.permissions == observed.permissions {
                    hash.requires((fields.size, file))
                } else { changed.requires(()) }
            },
        })
}

pub open spec fn checkout_entry_observed_callable<P, F, B, E, FE, R, HL, O, FM, HF, C>(
    mode: Seq<char>, path: P, observed: Result<CheckoutEntryMetadata, (bool, E)>,
    format_error: FE, read_link: R, hash_link: HL, open: O,
    opened_metadata: FM, hash_file: HF, changed_error: C,
) -> bool
    where FE: Fn(E) -> String, R: Fn(P) -> Result<B, String>,
          HL: Fn(B) -> Result<String, String>, O: Fn(P) -> Result<F, String>,
          FM: Fn(&F) -> Result<CheckoutEntryMetadata, String>,
          HF: Fn(u64, F) -> Result<String, String>, C: Fn() -> String,
{
    match observed {
        Err((missing, error)) => missing || format_error.requires((error,)),
        Ok(fields) => {
            if mode == "120000"@ {
                !fields.symbolic || (read_link.requires((path,))
                    && (forall|target: Result<B, String>| #[trigger] read_link.ensures((path,), target)
                        ==> match target { Err(_) => true, Ok(bytes) => hash_link.requires((bytes,)) }))
            } else if !(fields.regular && ((fields.permissions & 0o100 != 0) == (mode == "100755"@))) {
                true
            } else {
                open.requires((path,)) && (forall|opened: Result<F, String>|
                    #[trigger] open.ensures((path,), opened) ==> match opened {
                        Err(_) => true,
                        Ok(file) => checkout_entry_opened_callable(fields, file, opened_metadata, hash_file, changed_error),
                    })
            }
        },
    }
}

pub open spec fn checkout_entry_matches_callable<P, F, B, E, L, M, FE, R, HL, O, FM, HF, C>(
    kind: Seq<char>, mode: Seq<char>, io: CheckoutEntryIo<L, M, FE, R, HL, O, FM, HF, C>,
) -> bool
    where L: Fn() -> Result<Option<P>, String>,
          M: Fn(&P) -> Result<CheckoutEntryMetadata, (bool, E)>, FE: Fn(E) -> String,
          R: Fn(P) -> Result<B, String>, HL: Fn(B) -> Result<String, String>,
          O: Fn(P) -> Result<F, String>, FM: Fn(&F) -> Result<CheckoutEntryMetadata, String>,
          HF: Fn(u64, F) -> Result<String, String>, C: Fn() -> String,
{
    if kind != "blob"@ || !(mode == "100644"@ || mode == "100755"@ || mode == "120000"@) {
        true
    } else {
        io.locate.requires(()) && (forall|located: Result<Option<P>, String>|
            #[trigger] io.locate.ensures((), located) ==> match located {
                Err(_) => true,
                Ok(None) => true,
                Ok(Some(path)) => io.metadata.requires((&path,))
                    && (forall|observed: Result<CheckoutEntryMetadata, (bool, E)>|
                        #[trigger] io.metadata.ensures((&path,), observed)
                        ==> checkout_entry_observed_callable(mode, path, observed, io.format_metadata_error,
                            io.read_link, io.hash_link, io.open, io.opened_metadata,
                            io.hash_file, io.changed_error)),
            })
    }
}

/// 保持实际观察次序、首错与完整结果；证明快照不进入运行程序。
pub fn checkout_entry_matches<P, F, B, E, L, M, FE, R, HL, O, FM, HF, C>(
    kind: &str, mode: &str, oid: &str, io: CheckoutEntryIo<L, M, FE, R, HL, O, FM, HF, C>,
) -> (out: Result<bool, String>)
    where L: Fn() -> Result<Option<P>, String>,
          M: Fn(&P) -> Result<CheckoutEntryMetadata, (bool, E)>, FE: Fn(E) -> String,
          R: Fn(P) -> Result<B, String>, HL: Fn(B) -> Result<String, String>,
          O: Fn(P) -> Result<F, String>, FM: Fn(&F) -> Result<CheckoutEntryMetadata, String>,
          HF: Fn(u64, F) -> Result<String, String>, C: Fn() -> String,
    requires checkout_entry_matches_callable(kind@, mode@, io),
    ensures checkout_entry_matches_result(kind@, mode@, oid@, io, out),
{
    if !checkout_entry_supported(kind, mode) { return Ok(false); }
    let located = (io.locate)();
    let ghost located_snapshot = located;
    let path = match located {
        Err(error) => { return Err(error); },
        Ok(None) => { return Ok(false); },
        Ok(Some(path)) => path,
    };
    let ghost path_snapshot = path;
    let observation = (io.metadata)(&path);
    let ghost observation_snapshot = observation;
    let observed = match observation {
        Err((missing, error)) => {
            let out = if missing { Ok(false) }
                else { Err((io.format_metadata_error)(error)) };
            proof {
                assert(checkout_entry_observed_result(mode@, oid@, path, observation_snapshot,
                    io.format_metadata_error, io.read_link, io.hash_link, io.open,
                    io.opened_metadata, io.hash_file, io.changed_error, out));
                assert(checkout_entry_matches_result(kind@, mode@, oid@, io, out));
            }
            return out;
        },
        Ok(fields) => fields,
    };
    if checkout_symbolic_link(mode) {
        if !observed.symbolic {
            let out = Ok(false);
            proof {
                assert(checkout_entry_observed_result(mode@, oid@, path_snapshot, observation_snapshot,
                    io.format_metadata_error, io.read_link, io.hash_link, io.open,
                    io.opened_metadata, io.hash_file, io.changed_error, out));
                assert(checkout_entry_matches_result(kind@, mode@, oid@, io, out));
            }
            return out;
        }
        let target_result = (io.read_link)(path);
        let ghost target_snapshot = target_result;
        let target = match target_result {
            Err(error) => {
                let out = Err(error);
                proof {
                    assert(checkout_entry_observed_result(mode@, oid@, path_snapshot, observation_snapshot,
                        io.format_metadata_error, io.read_link, io.hash_link, io.open,
                        io.opened_metadata, io.hash_file, io.changed_error, out));
                    assert(checkout_entry_matches_result(kind@, mode@, oid@, io, out));
                }
                return out;
            },
            Ok(target) => target,
        };
        let out = checkout_digest_matches(oid, (io.hash_link)(target));
        proof {
            assert(checkout_entry_observed_result(mode@, oid@, path_snapshot, observation_snapshot,
                io.format_metadata_error, io.read_link, io.hash_link, io.open,
                io.opened_metadata, io.hash_file, io.changed_error, out));
            assert(checkout_entry_matches_result(kind@, mode@, oid@, io, out));
        }
        return out;
    }
    if !checkout_regular_file_matches(observed.regular, observed.permissions, mode) {
        let out = Ok(false);
        proof {
            assert(checkout_entry_observed_result(mode@, oid@, path_snapshot, observation_snapshot,
                io.format_metadata_error, io.read_link, io.hash_link, io.open,
                io.opened_metadata, io.hash_file, io.changed_error, out));
            assert(checkout_entry_matches_result(kind@, mode@, oid@, io, out));
        }
        return out;
    }
    let opened_result_value = (io.open)(path);
    let ghost opened_snapshot = opened_result_value;
    let file = match opened_result_value {
        Err(error) => {
            let out = Err(error);
            proof {
                assert(checkout_entry_opened_result(oid@, observed, opened_snapshot, io.opened_metadata,
                    io.hash_file, io.changed_error, out));
                assert(checkout_entry_observed_result(mode@, oid@, path_snapshot, observation_snapshot,
                    io.format_metadata_error, io.read_link, io.hash_link, io.open,
                    io.opened_metadata, io.hash_file, io.changed_error, out));
                assert(checkout_entry_matches_result(kind@, mode@, oid@, io, out));
            }
            return out;
        },
        Ok(file) => file,
    };
    let opened_observation = (io.opened_metadata)(&file);
    let ghost opened_observation_snapshot = opened_observation;
    let opened = match opened_observation {
        Err(error) => {
            let out = Err(error);
            proof {
                assert(checkout_entry_opened_result(oid@, observed, opened_snapshot, io.opened_metadata,
                    io.hash_file, io.changed_error, out));
                assert(checkout_entry_observed_result(mode@, oid@, path_snapshot, observation_snapshot,
                    io.format_metadata_error, io.read_link, io.hash_link, io.open,
                    io.opened_metadata, io.hash_file, io.changed_error, out));
                assert(checkout_entry_matches_result(kind@, mode@, oid@, io, out));
            }
            return out;
        },
        Ok(fields) => fields,
    };
    if !checkout_opened_file_matches(opened.regular, opened.size, opened.permissions,
        observed.size, observed.permissions) {
        let out = Err((io.changed_error)());
        proof {
            assert(checkout_entry_opened_result(oid@, observed, opened_snapshot, io.opened_metadata,
                io.hash_file, io.changed_error, out));
            assert(checkout_entry_observed_result(mode@, oid@, path_snapshot, observation_snapshot,
                io.format_metadata_error, io.read_link, io.hash_link, io.open,
                io.opened_metadata, io.hash_file, io.changed_error, out));
            assert(checkout_entry_matches_result(kind@, mode@, oid@, io, out));
        }
        return out;
    }
    let out = checkout_digest_matches(oid, (io.hash_file)(opened.size, file));
    proof {
        assert(checkout_entry_opened_result(oid@, observed, opened_snapshot, io.opened_metadata,
            io.hash_file, io.changed_error, out));
        assert(checkout_entry_observed_result(mode@, oid@, path_snapshot, observation_snapshot,
            io.format_metadata_error, io.read_link, io.hash_link, io.open,
            io.opened_metadata, io.hash_file, io.changed_error, out));
        assert(checkout_entry_matches_result(kind@, mode@, oid@, io, out));
    }
    out
}


pub open spec fn git_blob_match_result<'a, A, B>(
    oid: &str, size: usize, bytes: &'a [u8], sha1: A, sha256: B, matches: bool,
) -> bool
    where A: Fn(usize, &'a [u8]) -> String,
          B: Fn(usize, &'a [u8]) -> String,
{
    if bytes.len() != size { !matches }
    else if oid.spec_bytes().len() == 40 {
        exists|digest: String| sha1.ensures((size, bytes), digest)
            && matches == (digest@ == oid@)
    } else {
        exists|digest: String| sha256.ensures((size, bytes), digest)
            && matches == (digest@ == oid@)
    }
}

/// 声明长度先于哈希；只调用原 OID 字节宽度选中的接口，保留原参数及完整摘要比较。
pub fn git_blob_matches<'a, A, B>(
    oid: &str, size: usize, bytes: &'a [u8], sha1: A, sha256: B,
) -> (matches: bool)
    where A: Fn(usize, &'a [u8]) -> String,
          B: Fn(usize, &'a [u8]) -> String,
    requires if bytes.len() == size {
        if oid.spec_bytes().len() == 40 { sha1.requires((size, bytes)) }
        else { sha256.requires((size, bytes)) }
    } else { true },
    ensures git_blob_match_result(oid, size, bytes, sha1, sha256, matches),
{
    if bytes.len() != size { return false; }
    let digest = if oid.as_bytes().len() == 40 { sha1(size, bytes) } else { sha256(size, bytes) };
    let matches = identifiers::equal_text(digest.as_str(), oid);
    assert(git_blob_match_result(oid, size, bytes, sha1, sha256, matches));
    matches
}

pub enum CheckoutHashAlgorithm {
    Sha1,
    Sha256,
}

/// 先复用完整 OID 准入，再选择原有 Git 对象摘要算法；不截取或归一化输入。
pub fn checkout_hash_algorithm(text: &str) -> (out: Result<CheckoutHashAlgorithm, String>)
    ensures
        out.is_ok() == identifiers::oid(text.spec_bytes()),
        match out {
            Ok(CheckoutHashAlgorithm::Sha1) => text.spec_bytes().len() == 40,
            Ok(CheckoutHashAlgorithm::Sha256) => text.spec_bytes().len() == 64,
            Err(message) => message@ == "expected full nonzero lowercase commit OID"@,
        },
{
    match identifiers::full_oid(text) {
        Err(message) => Err(message),
        Ok(()) => {
            if text.len() == 40 { Ok(CheckoutHashAlgorithm::Sha1) }
            else { Ok(CheckoutHashAlgorithm::Sha256) }
        },
    }
}

/// 只调用已准入的哈希分支，并完整转交原输入与同一次实际结果。
pub fn checkout_object_hash<S, A, B>(
    oid: &str, size: u64, source: S, sha1: A, sha256: B,
) -> (out: Result<String, String>)
    where A: Fn(u64, S) -> Result<String, String>,
          B: Fn(u64, S) -> Result<String, String>,
    requires if identifiers::oid(oid.spec_bytes()) {
        if oid.spec_bytes().len() == 40 { sha1.requires((size, source)) }
        else { sha256.requires((size, source)) }
    } else { true },
    ensures if identifiers::oid(oid.spec_bytes()) {
        if oid.spec_bytes().len() == 40 { sha1.ensures((size, source), out) }
        else { sha256.ensures((size, source), out) }
    } else {
        out matches Err(error) && error@ == "expected full nonzero lowercase commit OID"@
    },
{
    match checkout_hash_algorithm(oid) {
        Err(error) => Err(error),
        Ok(CheckoutHashAlgorithm::Sha1) => sha1(size, source),
        Ok(CheckoutHashAlgorithm::Sha256) => sha256(size, source),
    }
}

pub open spec fn checkout_hash_advance_spec(total: u64, size: u64, chunk: u64)
    -> Result<u64, Seq<char>>
{
    let next = total as int + chunk as int;
    if next > u64::MAX { Err("checkout file size overflow"@) }
    else if next > size { Err("checkout file grew during observation"@) }
    else { Ok(next as u64) }
}

/// 先判计数溢出，再判相对本次原观察长度增长；保留原错误优先级。
pub fn checkout_hash_advance(total: u64, size: u64, chunk: u64) -> (out: Result<u64, String>)
    ensures match out {
        Ok(next) => checkout_hash_advance_spec(total, size, chunk) == Ok::<u64, Seq<char>>(next),
        Err(message) => checkout_hash_advance_spec(total, size, chunk) == Err::<u64, Seq<char>>(message@),
    },
{
    match total.checked_add(chunk) {
        None => Err("checkout file size overflow".to_owned()),
        Some(next) => {
            if next > size { Err("checkout file grew during observation".to_owned()) }
            else { Ok(next) }
        },
    }
}

pub fn checkout_hash_complete(total: u64, size: u64) -> (out: Result<(), String>)
    ensures
        out.is_ok() == (total == size),
        match out { Ok(()) => true, Err(message) => message@ == "checkout file shrank during observation"@ },
{
    if total == size { Ok(()) }
    else { Err("checkout file shrank during observation".to_owned()) }
}


#[verifier::opaque]
pub open spec fn checkout_hash_stream_step<S, B, D, R, V, U>(
    before: (S, B, D, u64), after: (S, B, D, u64), size: u64,
    read: R, view: V, update: U,
) -> bool
    where R: Fn(S, B) -> (S, B, Result<usize, String>),
          V: Fn(&B) -> &[u8], U: Fn(D, &[u8]) -> D,
{
    exists|count: usize, bytes: &[u8], chunk: &[u8]|
        count > 0 && count <= bytes.len()
        && read.ensures((before.0, before.1), (after.0, after.1, Ok::<usize, String>(count)))
        && view.ensures((&after.1,), bytes)
        && chunk@ == bytes@.subrange(0, count as int)
        && checkout_hash_advance_spec(before.3, size, count as u64) == Ok::<u64, Seq<char>>(after.3)
        && update.ensures((before.2, chunk), after.2)
}

pub open spec fn checkout_hash_stream_prefix<S, B, D, R, V, U>(
    states: Seq<(S, B, D, u64)>, initial: (S, B, D, u64), size: u64,
    read: R, view: V, update: U,
) -> bool
    where R: Fn(S, B) -> (S, B, Result<usize, String>),
          V: Fn(&B) -> &[u8], U: Fn(D, &[u8]) -> D,
{
    states.len() > 0 && states[0] == initial
        && states.len() <= states.last().3 as int + 1
        && (forall|i: int| 0 <= i < states.len() ==> (#[trigger] states[i]).3 <= size)
        && (forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] checkout_hash_stream_step(states[i], states[i + 1], size, read, view, update))
}

pub open spec fn checkout_hash_stream_terminal<S, B, D, R, F>(
    before: (S, B, D, u64), observation: (S, B, Result<usize, String>),
    size: u64, read: R, finalize: F, out: Result<String, String>,
) -> bool
    where R: Fn(S, B) -> (S, B, Result<usize, String>), F: Fn(D) -> String,
{
    read.ensures((before.0, before.1), observation)
        && match observation.2 {
            Err(error) => match out { Err(message) => message@ == error@, Ok(_) => false },
            Ok(count) => if count == 0 {
                if before.3 == size {
                    match out { Ok(value) => finalize.ensures((before.2,), value), Err(_) => false }
                } else {
                    match out { Err(message) => message@ == "checkout file shrank during observation"@, Ok(_) => false }
                }
            } else {
                match (checkout_hash_advance_spec(before.3, size, count as u64), out) {
                    (Err(expected), Err(message)) => message@ == expected,
                    _ => false,
                }
            },
        }
}

pub open spec fn checkout_hash_stream_result<S, B, D, R, V, U, F>(
    initial: (S, B, D, u64), size: u64,
    read: R, view: V, update: U, finalize: F, out: Result<String, String>,
) -> bool
    where R: Fn(S, B) -> (S, B, Result<usize, String>),
          V: Fn(&B) -> &[u8], U: Fn(D, &[u8]) -> D, F: Fn(D) -> String,
{
    exists|states: Seq<(S, B, D, u64)>, observation: (S, B, Result<usize, String>)|
        checkout_hash_stream_prefix(states, initial, size, read, view, update)
            && checkout_hash_stream_terminal(states.last(), observation, size, read, finalize, out)
}

// 真实读取、完整错误与摘要接口通过回调转交；轨迹只在证明中保留，没有运行时副本。
pub fn checkout_hash_stream<S, D, B, R, V, U, F>(
    size: u64, input_source: S, input_digest: D, input_buffer: B,
    read: R, view: V, update: U, finalize: F,
) -> (out: Result<String, String>)
    where R: Fn(S, B) -> (S, B, Result<usize, String>),
          V: Fn(&B) -> &[u8], U: Fn(D, &[u8]) -> D, F: Fn(D) -> String,
    requires
        forall|buffer: &B| #[trigger] view.requires((buffer,)),
        forall|state: S, buffer: B| #[trigger] read.requires((state, buffer)),
        forall|state: S, buffer: B, result: (S, B, Result<usize, String>)|
            #[trigger] read.ensures((state, buffer), result) ==>
                (forall|bytes: &[u8]| #[trigger] view.ensures((&result.1,), bytes)
                    ==> match result.2 { Ok(count) => count <= bytes.len(), Err(_) => true }),
        forall|state: D, bytes: &[u8]| #[trigger] update.requires((state, bytes)),
        forall|state: D| #[trigger] finalize.requires((state,)),
    ensures checkout_hash_stream_result((input_source, input_buffer, input_digest, 0), size, read, view, update, finalize, out),
{
    let ghost initial = (input_source, input_buffer, input_digest, 0u64);
    let ghost states = seq![initial];
    let mut source = input_source;
    let mut buffer = input_buffer;
    let mut digest = input_digest;
    let mut total = 0u64;
    loop
        invariant total <= size,
            initial == (input_source, input_buffer, input_digest, 0u64),
            states.last() == (source, buffer, digest, total),
            checkout_hash_stream_prefix(states, initial, size, read, view, update),
            forall|buffer: &B| #[trigger] view.requires((buffer,)),
            forall|state: S, buffer: B| #[trigger] read.requires((state, buffer)),
            forall|state: S, buffer: B, result: (S, B, Result<usize, String>)|
                #[trigger] read.ensures((state, buffer), result) ==>
                    (forall|bytes: &[u8]| #[trigger] view.ensures((&result.1,), bytes)
                        ==> match result.2 { Ok(count) => count <= bytes.len(), Err(_) => true }),
            forall|state: D, bytes: &[u8]| #[trigger] update.requires((state, bytes)),
            forall|state: D| #[trigger] finalize.requires((state,)),
        decreases size - total,
    {
        assert(checkout_hash_stream_prefix(states, initial, size, read, view, update));
        let observation = read(source, buffer);
        let ghost observed = observation;
        let (next_source, next_buffer, result) = observation;
        source = next_source;
        buffer = next_buffer;
        let count = match result {
            Err(error) => {
                assert(checkout_hash_stream_terminal(states.last(), observed, size, read, finalize, Err(error)));
                assert(checkout_hash_stream_result(initial, size, read, view, update, finalize, Err(error)));
                return Err(error);
            },
            Ok(count) => count,
        };
        if count == 0 {
            match checkout_hash_complete(total, size) {
                Err(error) => {
                    assert(checkout_hash_stream_terminal(states.last(), observed, size, read, finalize, Err(error)));
                    assert(checkout_hash_stream_result(initial, size, read, view, update, finalize, Err(error)));
                    return Err(error);
                },
                Ok(()) => {
                    let output = finalize(digest);
                    assert(checkout_hash_stream_terminal(states.last(), observed, size, read, finalize, Ok(output)));
                    assert(checkout_hash_stream_result(initial, size, read, view, update, finalize, Ok(output)));
                    return Ok(output);
                },
            }
        }
        let next = match checkout_hash_advance(total, size, count as u64) {
            Err(error) => {
                assert(checkout_hash_stream_terminal(states.last(), observed, size, read, finalize, Err(error)));
                assert(checkout_hash_stream_result(initial, size, read, view, update, finalize, Err(error)));
                return Err(error);
            },
            Ok(next) => next,
        };
        assert(next > total);
        let bytes = view(&buffer);
        #[cfg(verus_keep_ghost)]
        let chunk = vstd::slice::slice_subrange(bytes, 0, count);
        #[cfg(not(verus_keep_ghost))]
        let chunk = &bytes[0..count];
        digest = update(digest, chunk);
        let ghost after = (source, buffer, digest, next);
        assert(checkout_hash_stream_step(states.last(), after, size, read, view, update)) by {
            reveal(checkout_hash_stream_step);
        }
        proof {
            let previous = states;
            states = states.push(after);
            assert forall|i: int| 0 <= i < states.len() - 1 implies
                #[trigger] checkout_hash_stream_step(states[i], states[i + 1], size, read, view, update) by {
                if i < previous.len() - 1 {
                    assert(checkout_hash_stream_step(previous[i], previous[i + 1], size, read, view, update));
                }
            }
        }
        total = next;
    }
}


pub open spec fn copy_hash_advance_spec(total: u64, count: usize) -> Result<u64, Seq<char>> {
    if total as int + count as int > u64::MAX { Err("input length overflow"@) }
    else { Ok((total as int + count as int) as u64) }
}

pub fn copy_hash_advance(total: u64, count: usize) -> (out: Result<u64, String>)
    ensures match out {
        Ok(next) => copy_hash_advance_spec(total, count) == Ok::<u64, Seq<char>>(next),
        Err(error) => copy_hash_advance_spec(total, count) == Err::<u64, Seq<char>>(error@),
    },
{
    match total.checked_add(count as u64) {
        Some(next) => Ok(next),
        None => Err("input length overflow".to_owned()),
    }
}

#[verifier::opaque]
pub open spec fn copy_hash_stream_step<S, B, R, V, W, U>(
    before: (S, B, u64), after: (S, B, u64), read: R, view: V, write: W, update: U,
) -> bool
    where R: Fn(S, B) -> (S, B, Result<usize, String>), V: Fn(&B) -> &[u8],
          W: Fn(S, &[u8]) -> (S, Result<(), String>), U: Fn(S, &[u8]) -> S,
{
    exists|count: usize, observed: S, bytes: &[u8], chunk: &[u8], written: (S, Result<(), String>)|
        count > 0 && count <= bytes.len()
        && read.ensures((before.0, before.1), (observed, after.1, Ok::<usize, String>(count)))
        && view.ensures((&after.1,), bytes) && chunk@ == bytes@.subrange(0, count as int)
        && write.ensures((observed, chunk), written) && written.1.is_ok()
        && update.ensures((written.0, chunk), after.0)
        && before.2 as int + count as int == after.2 as int
}

pub open spec fn copy_hash_stream_prefix<S, B, R, V, W, U>(
    states: Seq<(S, B, u64)>, initial: (S, B, u64), read: R, view: V, write: W, update: U,
) -> bool
    where R: Fn(S, B) -> (S, B, Result<usize, String>), V: Fn(&B) -> &[u8],
          W: Fn(S, &[u8]) -> (S, Result<(), String>), U: Fn(S, &[u8]) -> S,
{
    states.len() > 0 && states[0] == initial
        && states.len() <= states.last().2 as int + 1
        && (forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] copy_hash_stream_step(states[i], states[i + 1], read, view, write, update))
}

pub open spec fn copy_hash_stream_terminal<S, B, R, V, W, U, F>(
    before: (S, B, u64), observation: (S, B, Result<usize, String>),
    read: R, view: V, write: W, update: U, finalize: F, out: Result<(String, u64), String>,
) -> bool
    where R: Fn(S, B) -> (S, B, Result<usize, String>), V: Fn(&B) -> &[u8],
          W: Fn(S, &[u8]) -> (S, Result<(), String>), U: Fn(S, &[u8]) -> S, F: Fn(S) -> String,
{
    read.ensures((before.0, before.1), observation)
        && match observation.2 {
            Err(error) => match out { Err(message) => message@ == error@, Ok(_) => false },
            Ok(count) => if count == 0 {
                match out {
                    Ok((digest, length)) => length == before.2 && finalize.ensures((observation.0,), digest),
                    Err(_) => false,
                }
            } else {
                exists|bytes: &[u8], chunk: &[u8], written: (S, Result<(), String>)|
                    count <= bytes.len() && view.ensures((&observation.1,), bytes)
                    && chunk@ == bytes@.subrange(0, count as int)
                    && write.ensures((observation.0, chunk), written)
                    && match written.1 {
                        Err(error) => match out { Err(message) => message@ == error@, Ok(_) => false },
                        Ok(()) => exists|hashed: S|
                            update.ensures((written.0, chunk), hashed)
                            && match (copy_hash_advance_spec(before.2, count), out) {
                                (Err(expected), Err(message)) => message@ == expected,
                                _ => false,
                            },
                    }
            },
        }
}

pub open spec fn copy_hash_stream_result<S, B, R, V, W, U, F>(
    initial: (S, B, u64), read: R, view: V, write: W, update: U, finalize: F,
    out: Result<(String, u64), String>,
) -> bool
    where R: Fn(S, B) -> (S, B, Result<usize, String>), V: Fn(&B) -> &[u8],
          W: Fn(S, &[u8]) -> (S, Result<(), String>), U: Fn(S, &[u8]) -> S, F: Fn(S) -> String,
{
    exists|states: Seq<(S, B, u64)>, observation: (S, B, Result<usize, String>)|
        copy_hash_stream_prefix(states, initial, read, view, write, update)
        && copy_hash_stream_terminal(states.last(), observation, read, view, write, update, finalize, out)
}

/// 读取原块后先完整写入，再更新摘要和计数；首错停止，EOF 才终结摘要。
/// 输入、输出与摘要共享一个搬运状态，证明中的轨迹不产生运行时副本。
pub fn copy_hash_stream<S, B, R, V, W, U, F>(
    input_state: S, input_buffer: B, read: R, view: V, write: W, update: U, finalize: F,
) -> (out: Result<(String, u64), String>)
    where R: Fn(S, B) -> (S, B, Result<usize, String>), V: Fn(&B) -> &[u8],
          W: Fn(S, &[u8]) -> (S, Result<(), String>), U: Fn(S, &[u8]) -> S, F: Fn(S) -> String,
    requires
        forall|buffer: &B| #[trigger] view.requires((buffer,)),
        forall|state: S, buffer: B| #[trigger] read.requires((state, buffer)),
        forall|state: S, buffer: B, result: (S, B, Result<usize, String>)|
            #[trigger] read.ensures((state, buffer), result) ==>
                (forall|bytes: &[u8]| #[trigger] view.ensures((&result.1,), bytes)
                    ==> match result.2 { Ok(count) => count <= bytes.len(), Err(_) => true }),
        forall|state: S, bytes: &[u8]| #[trigger] write.requires((state, bytes)),
        forall|state: S, bytes: &[u8]| #[trigger] update.requires((state, bytes)),
        forall|state: S| #[trigger] finalize.requires((state,)),
    ensures copy_hash_stream_result((input_state, input_buffer, 0), read, view, write, update, finalize, out),
{
    let ghost initial = (input_state, input_buffer, 0u64);
    let ghost states = seq![initial];
    let mut state = input_state;
    let mut buffer = input_buffer;
    let mut total = 0u64;
    loop
        invariant
            initial == (input_state, input_buffer, 0u64),
            states.last() == (state, buffer, total),
            copy_hash_stream_prefix(states, initial, read, view, write, update),
            forall|buffer: &B| #[trigger] view.requires((buffer,)),
            forall|state: S, buffer: B| #[trigger] read.requires((state, buffer)),
            forall|state: S, buffer: B, result: (S, B, Result<usize, String>)|
                #[trigger] read.ensures((state, buffer), result) ==>
                    (forall|bytes: &[u8]| #[trigger] view.ensures((&result.1,), bytes)
                        ==> match result.2 { Ok(count) => count <= bytes.len(), Err(_) => true }),
            forall|state: S, bytes: &[u8]| #[trigger] write.requires((state, bytes)),
            forall|state: S, bytes: &[u8]| #[trigger] update.requires((state, bytes)),
            forall|state: S| #[trigger] finalize.requires((state,)),
        decreases u64::MAX - total,
    {
        let observation = read(state, buffer);
        let ghost observed = observation;
        let (next_state, next_buffer, result) = observation;
        state = next_state;
        buffer = next_buffer;
        let count = match result {
            Err(error) => {
                assert(copy_hash_stream_terminal(states.last(), observed, read, view, write, update, finalize, Err(error)));
                assert(copy_hash_stream_result(initial, read, view, write, update, finalize, Err(error)));
                return Err(error);
            },
            Ok(count) => count,
        };
        if count == 0 {
            let digest = finalize(state);
            let out = Ok((digest, total));
            assert(copy_hash_stream_terminal(states.last(), observed, read, view, write, update, finalize, out));
            assert(copy_hash_stream_result(initial, read, view, write, update, finalize, out));
            return out;
        }
        let bytes = view(&buffer);
        #[cfg(verus_keep_ghost)]
        let chunk = vstd::slice::slice_subrange(bytes, 0, count);
        #[cfg(not(verus_keep_ghost))]
        let chunk = &bytes[0..count];
        let ghost write_input = state;
        assert(write_input == observed.0);
        let written = write(state, chunk);
        let ghost write_observation = written;
        assert(write.ensures((write_input, chunk), write_observation));
        let (written_state, result) = written;
        state = written_state;
        match result {
            Err(error) => {
                assert(copy_hash_stream_terminal(states.last(), observed, read, view, write, update, finalize, Err(error)));
                assert(copy_hash_stream_result(initial, read, view, write, update, finalize, Err(error)));
                return Err(error);
            },
            Ok(()) => { assert(write_observation.1.is_ok()); },
        }
        assert(state == write_observation.0);
        state = update(state, chunk);
        let next = match copy_hash_advance(total, count) {
            Err(error) => {
                assert(copy_hash_stream_terminal(states.last(), observed, read, view, write, update, finalize, Err(error)));
                assert(copy_hash_stream_result(initial, read, view, write, update, finalize, Err(error)));
                return Err(error);
            },
            Ok(next) => next,
        };
        assert(next > total);
        let ghost after = (state, buffer, next);
        assert(copy_hash_stream_step(states.last(), after, read, view, write, update)) by {
            reveal(copy_hash_stream_step);
            assert(count > 0 && count <= bytes.len());
            assert(read.ensures((states.last().0, states.last().1),
                (observed.0, after.1, Ok::<usize, String>(count))));
            assert(view.ensures((&after.1,), bytes));
            assert(chunk@ == bytes@.subrange(0, count as int));
            assert(write.ensures((observed.0, chunk), write_observation));
            assert(write_observation.1.is_ok());
            assert(update.ensures((write_observation.0, chunk), after.0));
            assert(states.last().2 as int + count as int == after.2 as int);
        }
        proof {
            let previous = states;
            states = states.push(after);
            assert forall|i: int| 0 <= i < states.len() - 1 implies
                #[trigger] copy_hash_stream_step(states[i], states[i + 1], read, view, write, update) by {
                if i < previous.len() - 1 {
                    assert(copy_hash_stream_step(previous[i], previous[i + 1], read, view, write, update));
                }
            }
        }
        total = next;
    }
}

pub open spec fn git_tree_row_at(rows: Seq<&str>, i: int) -> Result<GitTreeRowBytes, GitInventoryErrorView> {
    git_tree_row_spec(rows[i].spec_bytes())
}

pub open spec fn git_tree_prefix_valid(rows: Seq<&str>, count: int) -> bool {
    0 <= count <= rows.len()
        && (forall|i: int| 0 <= i < count ==> (#[trigger] git_tree_row_at(rows, i)).is_ok())
        && (forall|i: int, j: int| 0 <= i < j < count ==>
            ((#[trigger] git_tree_row_at(rows, i))->Ok_0).0
                != ((#[trigger] git_tree_row_at(rows, j))->Ok_0).0)
}

pub open spec fn git_tree_entry_matches(key: TreePath, entry: TreeEntry, row: GitTreeRowBytes) -> bool {
    key.bytes() == row.0 && vstd::utf8::encode_utf8(entry.mode@) == row.1
        && vstd::utf8::encode_utf8(entry.oid@) == row.2
        && vstd::utf8::encode_utf8(entry.kind@) == row.3
}

/// 双向完整对应排除遗漏与额外条目；数量是实际 map 的键数。
pub open spec fn git_tree_prefix_map(tree: Map<TreePath, TreeEntry>, rows: Seq<&str>, count: int) -> bool {
    tree.len() == count
        && (forall|key: TreePath| tree.contains_key(key) ==> exists|i: int| 0 <= i < count
            && git_tree_entry_matches(key, tree[key], (git_tree_row_at(rows, i)->Ok_0)))
        && (forall|i: int| #![trigger git_tree_row_at(rows, i)]
            0 <= i < count ==> exists|key: TreePath| tree.contains_key(key)
            && git_tree_entry_matches(key, tree[key], (git_tree_row_at(rows, i)->Ok_0)))
}

pub open spec fn git_tree_error_at(rows: Seq<&str>, i: int, error: GitInventoryErrorView) -> bool {
    match git_tree_row_at(rows, i) {
        Err(expected) => error == expected,
        Ok(row) => error == GitInventoryErrorView::Message("duplicate Git tree path"@)
            && exists|j: int| 0 <= j < i && (git_tree_row_at(rows, j)->Ok_0).0 == row.0,
    }
}

pub proof fn git_tree_error_blocks_prefix(rows: Seq<&str>, i: int,
    error: GitInventoryErrorView, count: int)
    requires 0 <= i < count <= rows.len(), git_tree_error_at(rows, i, error),
    ensures !git_tree_prefix_valid(rows, count),
{
    if git_tree_prefix_valid(rows, count) {
        match git_tree_row_at(rows, i) {
            Err(_) => {},
            Ok(row) => {
                let j = choose|j: int| 0 <= j < i && (git_tree_row_at(rows, j)->Ok_0).0 == row.0;
                assert((git_tree_row_at(rows, j)->Ok_0).0 != (git_tree_row_at(rows, i)->Ok_0).0);
            },
        }
    }
}

pub proof fn git_tree_first_error_unique(rows: Seq<&str>, i: int, error: GitInventoryErrorView,
    j: int, other: GitInventoryErrorView)
    requires 0 <= i < rows.len(), git_tree_prefix_valid(rows, i), git_tree_error_at(rows, i, error),
        0 <= j < rows.len(), git_tree_prefix_valid(rows, j), git_tree_error_at(rows, j, other),
    ensures i == j, error == other,
{
    if i < j { git_tree_error_blocks_prefix(rows, i, error, j); }
    if j < i { git_tree_error_blocks_prefix(rows, j, other, i); }
}

/// 已发布前缀的路径见证与下一次插入的全文判重使用同一身份。
proof fn git_tree_next_path_is_new(tree: Map<TreePath, TreeEntry>, rows: Seq<&str>,
    count: int, path: TreePath)
    requires
        git_tree_prefix_map(tree, rows, count),
        !(exists|key: TreePath| (#[trigger] tree.contains_key(key)) && key@ == path@),
    ensures forall|j: int| 0 <= j < count ==>
        (#[trigger] git_tree_row_at(rows, j)->Ok_0).0 != path.bytes(),
{
    assert forall|j: int| 0 <= j < count implies
        (#[trigger] git_tree_row_at(rows, j)->Ok_0).0 != path.bytes() by {
        let key = choose|key: TreePath| tree.contains_key(key) && git_tree_entry_matches(
            key, tree[key], (git_tree_row_at(rows, j)->Ok_0));
        if key.bytes() == path.bytes() {
            tree_path_same_text(&key, &path);
            assert(exists|key: TreePath| (#[trigger] tree.contains_key(key)) && key@ == path@);
        }
    }
}

proof fn git_tree_extend_valid_prefix(rows: Seq<&str>, count: int, row: GitTreeRowBytes)
    requires
        0 <= count < rows.len(), git_tree_prefix_valid(rows, count),
        git_tree_row_at(rows, count) == Ok::<GitTreeRowBytes, GitInventoryErrorView>(row),
        forall|j: int| 0 <= j < count ==> (#[trigger] git_tree_row_at(rows, j)->Ok_0).0 != row.0,
    ensures git_tree_prefix_valid(rows, count + 1),
{
    assert forall|j: int| 0 <= j < count + 1 implies (#[trigger] git_tree_row_at(rows, j)).is_ok() by {
        if j < count {} else { assert(j == count); }
    }
    assert forall|i: int, j: int| 0 <= i < j < count + 1 implies
        ((#[trigger] git_tree_row_at(rows, i))->Ok_0).0
            != ((#[trigger] git_tree_row_at(rows, j))->Ok_0).0 by {
        if j < count {} else { assert(j == count); }
    }
}

/// 精确 map 更新的双向完整性单独证明，不展开行字节解析。
proof fn git_tree_extend_prefix_map(tree: Map<TreePath, TreeEntry>, rows: Seq<&str>,
    count: int, inserted: TreePath, entry: TreeEntry, row: GitTreeRowBytes)
    requires
        0 <= count < rows.len(), git_tree_prefix_map(tree, rows, count), tree.dom().finite(),
        git_tree_row_at(rows, count) == Ok::<GitTreeRowBytes, GitInventoryErrorView>(row),
        git_tree_entry_matches(inserted, entry, row), !tree.contains_key(inserted),
    ensures git_tree_prefix_map(tree.insert(inserted, entry), rows, count + 1),
{
    let next = tree.insert(inserted, entry);
    assert(next.len() == count + 1);
    assert forall|key: TreePath| (#[trigger] next.contains_key(key)) implies exists|j: int|
        0 <= j < count + 1 && git_tree_entry_matches(
            key, next[key], (git_tree_row_at(rows, j)->Ok_0)) by {
        if key == inserted {
            assert(git_tree_entry_matches(key, next[key], (git_tree_row_at(rows, count)->Ok_0)));
        } else {
            assert(tree.contains_key(key));
            let j = choose|j: int| 0 <= j < count && git_tree_entry_matches(
                key, tree[key], (git_tree_row_at(rows, j)->Ok_0));
            assert(git_tree_entry_matches(key, next[key], (git_tree_row_at(rows, j)->Ok_0)));
        }
    }
    assert forall|j: int| #![trigger git_tree_row_at(rows, j)]
        0 <= j < count + 1 implies exists|key: TreePath|
        next.contains_key(key) && git_tree_entry_matches(
            key, next[key], (git_tree_row_at(rows, j)->Ok_0)) by {
        if j == count {
            assert(next.contains_key(inserted));
            assert(git_tree_entry_matches(inserted, next[inserted], (git_tree_row_at(rows, j)->Ok_0)));
        } else {
            let key = choose|key: TreePath| tree.contains_key(key) && git_tree_entry_matches(
                key, tree[key], (git_tree_row_at(rows, j)->Ok_0));
            assert(key != inserted);
            assert(next.contains_key(key));
            assert(git_tree_entry_matches(key, next[key], (git_tree_row_at(rows, j)->Ok_0)));
        }
    }
}

proof fn git_tree_duplicate_error(tree: Map<TreePath, TreeEntry>, rows: Seq<&str>,
    count: int, path: TreePath, row: GitTreeRowBytes)
    requires
        0 <= count < rows.len(), git_tree_prefix_map(tree, rows, count),
        git_tree_row_at(rows, count) == Ok::<GitTreeRowBytes, GitInventoryErrorView>(row),
        path.bytes() == row.0,
        exists|key: TreePath| (#[trigger] tree.contains_key(key)) && key@ == path@,
    ensures git_tree_error_at(rows, count, GitInventoryErrorView::Message("duplicate Git tree path"@)),
{
    let key = choose|key: TreePath| (#[trigger] tree.contains_key(key)) && key@ == path@;
    let j = choose|j: int| 0 <= j < count && git_tree_entry_matches(
        key, tree[key], (git_tree_row_at(rows, j)->Ok_0));
    assert((git_tree_row_at(rows, j)->Ok_0).0 == row.0);
    assert(exists|j: int| 0 <= j < count && (git_tree_row_at(rows, j)->Ok_0).0 == row.0);
}

/// 完整逐行解析：返回全部原字段，或首个失败；失败后的局部树不发布。
pub fn parse_git_tree_rows<'a>(rows: &[&'a str]) -> (out: Result<Tree, GitInventoryError<'a>>)
    ensures match out {
        Ok(tree) => git_tree_prefix_valid(rows@, rows.len() as int)
            && git_tree_prefix_map(tree@, rows@, rows.len() as int),
        Err(error) => exists|i: int| 0 <= i < rows.len() && git_tree_prefix_valid(rows@, i)
            && git_tree_error_at(rows@, i, error.view()),
    },
{
    let mut tree = Tree::new();
    let mut i = 0;
    while i < rows.len()
        invariant
            0 <= i <= rows.len(),
            git_tree_prefix_valid(rows@, i as int),
            git_tree_prefix_map(tree@, rows@, i as int),
        decreases rows.len() - i,
    {
        let row = match parse_git_tree_row(rows[i]) {
            Ok(row) => row,
            Err(error) => {
                proof { assert(git_tree_error_at(rows@, i as int, error.view())); }
                return Err(error);
            },
        };
        let key = TreePath::from_text(row.path);
        let ghost inserted = key;
        let ghost before = tree@;
        proof { vstd::std_specs::btree::axiom_btree_map_view_finite_dom(tree); }
        let result = insert_git_tree_entry(&mut tree, key, row.mode, row.oid, row.kind);
        match result {
            Err(message) => {
                proof {
                    git_tree_duplicate_error(before, rows@, i as int, inserted, row.bytes());
                }
                return Err(GitInventoryError::Message(message));
            },
            Ok(()) => {},
        }
        proof {
            git_tree_next_path_is_new(before, rows@, i as int, inserted);
            git_tree_extend_valid_prefix(rows@, i as int, row.bytes());
            assert(git_tree_entry_matches(inserted, tree@[inserted], row.bytes()));
            git_tree_extend_prefix_map(before, rows@, i as int, inserted, tree@[inserted], row.bytes());
        }
        i += 1;
    }
    Ok(tree)
}

pub open spec fn optional_entry_fields(entry: Option<&TreeEntry>) -> Option<TreeEntryFields> {
    match entry { None => None, Some(entry) => Some(tree_entry_fields(entry)) }
}

pub open spec fn fields_blob(entry: Option<TreeEntryFields>) -> Option<Seq<char>> {
    match entry { None => None, Some(entry) => Some(entry.1) }
}

pub open spec fn fields_mode(entry: Option<TreeEntryFields>) -> Option<Seq<char>> {
    match entry { None => None, Some(entry) => Some(entry.0) }
}

pub open spec fn entry_delta_values(path: Seq<char>, old_entry: Option<TreeEntryFields>,
    new_entry: Option<TreeEntryFields>) -> Seq<DeltaFields>
{
    if old_entry == new_entry { seq![] }
    else { seq![(path,
        if old_entry.is_none() { "A"@ } else if new_entry.is_none() { "D"@ } else { "M"@ },
        fields_blob(old_entry), fields_blob(new_entry), fields_mode(old_entry), fields_mode(new_entry))] }
}

pub open spec fn rows_entry(rows: Seq<(&TreePath, &TreeEntry)>, start: int,
    path: Seq<char>) -> Option<TreeEntryFields>
    decreases rows.len() - start,
{
    if !(0 <= start < rows.len()) { None }
    else if rows[start].0@ == path { Some(tree_entry_fields(rows[start].1)) }
    else { rows_entry(rows, start + 1, path) }
}

pub open spec fn rows_has_path(rows: Seq<(&TreePath, &TreeEntry)>, start: int,
    path: Seq<char>) -> bool
{
    exists|k: int| start <= k < rows.len() && (#[trigger] rows[k]).0@ == path
}

proof fn rows_entry_witness(rows: Seq<(&TreePath, &TreeEntry)>, start: int, path: Seq<char>)
    requires 0 <= start <= rows.len(),
    ensures rows_entry(rows, start, path).is_some() == rows_has_path(rows, start, path),
        match rows_entry(rows, start, path) {
            None => true,
            Some(fields) => exists|k: int| start <= k < rows.len()
                && (#[trigger] rows[k]).0@ == path && tree_entry_fields(rows[k].1) == fields,
        },
    decreases rows.len() - start,
{
    if start < rows.len() {
        if rows[start].0@ == path {
            assert(rows_has_path(rows, start, path));
        } else {
            rows_entry_witness(rows, start + 1, path);
            assert(rows_has_path(rows, start, path) == rows_has_path(rows, start + 1, path));
        }
    }
}

proof fn tree_path_same_text(a: &TreePath, b: &TreePath)
    requires a.same(b),
    ensures a@ == b@,
{
    vstd::utf8::encode_utf8_decode_utf8(a@);
    vstd::utf8::encode_utf8_decode_utf8(b@);
}

proof fn rows_entry_absent(rows: Seq<(&TreePath, &TreeEntry)>, start: int, path: Seq<char>)
    requires 0 <= start <= rows.len(),
        forall|k: int| start <= k < rows.len() ==> (#[trigger] rows[k]).0@ != path,
    ensures rows_entry(rows, start, path).is_none(),
    decreases rows.len() - start,
{
    if start < rows.len() { rows_entry_absent(rows, start + 1, path); }
}

proof fn rows_entry_before_head(rows: Seq<(&TreePath, &TreeEntry)>, start: int, key: &TreePath)
    requires tree_rows_ordered(rows), 0 <= start <= rows.len(),
        start == rows.len() || identifiers::byte_lt(key.bytes(), rows[start].0.bytes()),
    ensures rows_entry(rows, start, key@).is_none(),
        forall|k: int| start <= k < rows.len()
            ==> identifiers::byte_lt(key.bytes(), (#[trigger] rows[k]).0.bytes()),
{
    assert forall|k: int| start <= k < rows.len()
        implies identifiers::byte_lt(key.bytes(), (#[trigger] rows[k]).0.bytes())
            && rows[k].0@ != key@ by {
        if start < k {
            identifiers::byte_lt_transitive(key.bytes(), rows[start].0.bytes(), rows[k].0.bytes());
        }
        identifiers::byte_lt_strict(key.bytes(), rows[k].0.bytes());
    }
    rows_entry_absent(rows, start, key@);
}

proof fn entry_delta_projection(selected_path: Seq<char>, old_entry: Option<&TreeEntry>,
    new_entry: Option<&TreeEntry>, path: Seq<char>)
    ensures delta_at_path(entry_delta_fields(selected_path, old_entry, new_entry), path)
        == if selected_path == path {
            entry_delta_values(path, optional_entry_fields(old_entry), optional_entry_fields(new_entry))
        } else { seq![] },
{
    reveal_with_fuel(Seq::filter, 2);
}

pub open spec fn tree_merge_fields(a: Seq<(&TreePath, &TreeEntry)>,
    b: Seq<(&TreePath, &TreeEntry)>, i: int, j: int) -> Seq<DeltaFields>
    decreases a.len() - i + b.len() - j,
{
    if !(0 <= i <= a.len() && 0 <= j <= b.len()) { seq![] }
    else if i == a.len() && j == b.len() { seq![] }
    else if i == a.len() {
        entry_delta_fields(b[j].0@, None, Some(b[j].1)) + tree_merge_fields(a, b, i, j + 1)
    } else if j == b.len() || a[i].0.order_spec(b[j].0) is Less {
        entry_delta_fields(a[i].0@, Some(a[i].1), None) + tree_merge_fields(a, b, i + 1, j)
    } else if a[i].0.order_spec(b[j].0) is Greater {
        entry_delta_fields(b[j].0@, None, Some(b[j].1)) + tree_merge_fields(a, b, i, j + 1)
    } else {
        entry_delta_fields(a[i].0@, Some(a[i].1), Some(b[j].1))
            + tree_merge_fields(a, b, i + 1, j + 1)
    }
}

/// 对任意完整路径，输出恰为该路径实际两端字段的零或一条变化。
proof fn tree_merge_path_projection(a: Seq<(&TreePath, &TreeEntry)>,
    b: Seq<(&TreePath, &TreeEntry)>, i: int, j: int, path: Seq<char>)
    requires tree_rows_ordered(a), tree_rows_ordered(b),
        0 <= i <= a.len(), 0 <= j <= b.len(),
    ensures delta_at_path(tree_merge_fields(a, b, i, j), path)
        == entry_delta_values(path, rows_entry(a, i, path), rows_entry(b, j, path)),
    decreases a.len() - i + b.len() - j,
{
    reveal_with_fuel(tree_merge_fields, 1);
    if i == a.len() && j == b.len() {
        reveal_with_fuel(Seq::filter, 1);
        return;
    }
    if i < a.len() && j < b.len() {
        identifiers::byte_trichotomy(a[i].0.bytes(), b[j].0.bytes());
        if a[i].0.order_spec(b[j].0) is Equal {
            tree_path_same_text(a[i].0, b[j].0);
        }
    }
    let (key, old_entry, new_entry, ni, nj):
        (&TreePath, Option<&TreeEntry>, Option<&TreeEntry>, int, int) =
        if i == a.len() { (b[j].0, None, Some(b[j].1), i, j + 1) }
        else if j == b.len() || a[i].0.order_spec(b[j].0) is Less {
            (a[i].0, Some(a[i].1), None, i + 1, j)
        } else if a[i].0.order_spec(b[j].0) is Greater {
            (b[j].0, None, Some(b[j].1), i, j + 1)
        } else { (a[i].0, Some(a[i].1), Some(b[j].1), i + 1, j + 1) };
    assert(ni == a.len() || identifiers::byte_lt(key.bytes(), a[ni].0.bytes()));
    assert(nj == b.len() || identifiers::byte_lt(key.bytes(), b[nj].0.bytes()));
    tree_merge_path_projection(a, b, ni, nj, path);
    let head = entry_delta_fields(key@, old_entry, new_entry);
    let rest = tree_merge_fields(a, b, ni, nj);
    assert(tree_merge_fields(a, b, i, j) == head + rest);
    entry_delta_projection(key@, old_entry, new_entry, path);
    Seq::<DeltaFields>::filter_distributes_over_add(head, rest,
        delta_path_predicate(path));
    if path == key@ {
        rows_entry_before_head(a, ni, key);
        rows_entry_before_head(b, nj, key);
        assert(rows_entry(a, i, path) == optional_entry_fields(old_entry));
        assert(rows_entry(b, j, path) == optional_entry_fields(new_entry));
    } else {
        assert(rows_entry(a, i, path) == rows_entry(a, ni, path));
        assert(rows_entry(b, j, path) == rows_entry(b, nj, path));
    }
}

proof fn tree_merge_sources(a: Seq<(&TreePath, &TreeEntry)>,
    b: Seq<(&TreePath, &TreeEntry)>, i: int, j: int)
    requires tree_rows_ordered(a), tree_rows_ordered(b),
        0 <= i <= a.len(), 0 <= j <= b.len(),
    ensures forall|k: int| 0 <= k < tree_merge_fields(a, b, i, j).len()
        ==> rows_has_path(a, i, (#[trigger] tree_merge_fields(a, b, i, j)[k]).0)
            || rows_has_path(b, j, tree_merge_fields(a, b, i, j)[k].0),
{
    let changes = tree_merge_fields(a, b, i, j);
    assert forall|k: int| 0 <= k < changes.len()
        implies rows_has_path(a, i, (#[trigger] changes[k]).0)
            || rows_has_path(b, j, changes[k].0) by {
        let path = changes[k].0;
        changes.lemma_filter_contains(delta_path_predicate(path), k);
        tree_merge_path_projection(a, b, i, j, path);
        rows_entry_witness(a, i, path);
        rows_entry_witness(b, j, path);
        assert(entry_delta_values(path, rows_entry(a, i, path), rows_entry(b, j, path)).len() > 0);
    }
}

pub open spec fn delta_fields_ordered(changes: Seq<DeltaFields>) -> bool {
    forall|i: int, j: int| 0 <= i < j < changes.len()
        ==> identifiers::byte_lt(vstd::utf8::encode_utf8((#[trigger] changes[i]).0),
            vstd::utf8::encode_utf8((#[trigger] changes[j]).0))
}

proof fn tree_merge_ordered(a: Seq<(&TreePath, &TreeEntry)>,
    b: Seq<(&TreePath, &TreeEntry)>, i: int, j: int)
    requires tree_rows_ordered(a), tree_rows_ordered(b),
        0 <= i <= a.len(), 0 <= j <= b.len(),
    ensures delta_fields_ordered(tree_merge_fields(a, b, i, j)),
    decreases a.len() - i + b.len() - j,
{
    reveal_with_fuel(tree_merge_fields, 1);
    if i == a.len() && j == b.len() { return; }
    if i < a.len() && j < b.len() {
        identifiers::byte_trichotomy(a[i].0.bytes(), b[j].0.bytes());
    }
    let (key, old_entry, new_entry, ni, nj):
        (&TreePath, Option<&TreeEntry>, Option<&TreeEntry>, int, int) =
        if i == a.len() { (b[j].0, None, Some(b[j].1), i, j + 1) }
        else if j == b.len() || a[i].0.order_spec(b[j].0) is Less {
            (a[i].0, Some(a[i].1), None, i + 1, j)
        } else if a[i].0.order_spec(b[j].0) is Greater {
            (b[j].0, None, Some(b[j].1), i, j + 1)
        } else { (a[i].0, Some(a[i].1), Some(b[j].1), i + 1, j + 1) };
    assert(ni == a.len() || identifiers::byte_lt(key.bytes(), a[ni].0.bytes()));
    assert(nj == b.len() || identifiers::byte_lt(key.bytes(), b[nj].0.bytes()));
    tree_merge_ordered(a, b, ni, nj);
    tree_merge_sources(a, b, ni, nj);
    rows_entry_before_head(a, ni, key);
    rows_entry_before_head(b, nj, key);
    let head = entry_delta_fields(key@, old_entry, new_entry);
    let rest = tree_merge_fields(a, b, ni, nj);
    let changes = tree_merge_fields(a, b, i, j);
    assert(changes == head + rest);
    assert(head.len() <= 1);
    assert(head.len() > 0 ==> head[0].0 == key@);
    assert forall|k: int| 0 <= k < rest.len()
        implies identifiers::byte_lt(key.bytes(), vstd::utf8::encode_utf8((#[trigger] rest[k]).0)) by {
        if rows_has_path(a, ni, rest[k].0) {
            let index = choose|index: int| ni <= index < a.len() && a[index].0@ == rest[k].0;
            assert(a[index].0.bytes() == vstd::utf8::encode_utf8(rest[k].0));
        } else {
            let index = choose|index: int| nj <= index < b.len() && b[index].0@ == rest[k].0;
            assert(b[index].0.bytes() == vstd::utf8::encode_utf8(rest[k].0));
        }
    }
    assert forall|x: int, y: int| 0 <= x < y < changes.len()
        implies identifiers::byte_lt(vstd::utf8::encode_utf8((#[trigger] changes[x]).0),
            vstd::utf8::encode_utf8((#[trigger] changes[y]).0)) by {
        if x < head.len() {
            assert(x == 0 && head.len() == 1);
            assert(changes[y] == rest[y - head.len()]);
        } else {
            assert(changes[x] == rest[x - head.len()]);
            assert(changes[y] == rest[y - head.len()]);
        }
    }
}

fn push_tree_change(out: &mut Vec<Delta>, path: &str,
    old_entry: Option<&TreeEntry>, new_entry: Option<&TreeEntry>)
    ensures delta_records_view(out@) == delta_records_view(old(out)@)
        + entry_delta_fields(path@, old_entry, new_entry),
{
    match delta_for_entry(path, old_entry, new_entry) {
        None => {},
        Some(change) => { out.push(change); },
    }
    proof { assert(delta_records_view(out@) =~= delta_records_view(old(out)@)
        + entry_delta_fields(path@, old_entry, new_entry)); }
}

fn delta_for_rows(a: &Vec<(&TreePath, &TreeEntry)>, b: &Vec<(&TreePath, &TreeEntry)>)
    -> (out: Vec<Delta>)
    ensures delta_records_view(out@) == tree_merge_fields(a@, b@, 0, 0),
{
    let mut out: Vec<Delta> = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;
    while i < a.len() || j < b.len()
        invariant
            i <= a.len(), j <= b.len(),
            delta_records_view(out@) + tree_merge_fields(a@, b@, i as int, j as int)
                == tree_merge_fields(a@, b@, 0, 0),
        decreases a.len() - i + b.len() - j,
    {
        proof { reveal_with_fuel(tree_merge_fields, 1); }
        if i == a.len() {
            push_tree_change(&mut out, b[j].0.as_str(), None, Some(b[j].1));
            j += 1;
        } else if j == b.len() {
            push_tree_change(&mut out, a[i].0.as_str(), Some(a[i].1), None);
            i += 1;
        } else {
            let order = a[i].0.cmp(b[j].0);
            match order {
                Ordering::Less => {
                    push_tree_change(&mut out, a[i].0.as_str(), Some(a[i].1), None);
                    i += 1;
                },
                Ordering::Greater => {
                    push_tree_change(&mut out, b[j].0.as_str(), None, Some(b[j].1));
                    j += 1;
                },
                Ordering::Equal => {
                    push_tree_change(&mut out, a[i].0.as_str(), Some(a[i].1), Some(b[j].1));
                    i += 1;
                    j += 1;
                },
            }
        }
    }
    out
}

pub open spec fn map_has_path(tree: Map<TreePath, TreeEntry>, path: Seq<char>) -> bool {
    exists|key: TreePath| tree.contains_key(key) && key@ == path
}

pub open spec fn map_path_key(tree: Map<TreePath, TreeEntry>, path: Seq<char>) -> TreePath {
    choose|key: TreePath| tree.contains_key(key) && key@ == path
}

pub open spec fn map_entry_fields(tree: Map<TreePath, TreeEntry>, path: Seq<char>)
    -> Option<TreeEntryFields>
{
    if map_has_path(tree, path) {
        let key = map_path_key(tree, path);
        Some(tree_entry_fields(&tree[key]))
    } else { None }
}

pub open spec fn delta_path_predicate(path: Seq<char>) -> spec_fn(DeltaFields) -> bool {
    |change: DeltaFields| change.0 == path
}

pub open spec fn delta_at_path(changes: Seq<DeltaFields>, path: Seq<char>) -> Seq<DeltaFields> {
    changes.filter(delta_path_predicate(path))
}

proof fn map_rows_entry(tree: Map<TreePath, TreeEntry>, rows: Seq<(&TreePath, &TreeEntry)>,
    path: Seq<char>)
    requires tree_rows_view(rows).to_set() == tree.kv_pairs(), tree_rows_ordered(rows),
    ensures map_entry_fields(tree, path) == rows_entry(rows, 0, path),
{
    rows_entry_witness(rows, 0, path);
    if map_has_path(tree, path) {
        let key = map_path_key(tree, path);
        assert(tree.contains_key(key) && key@ == path);
        assert(tree.kv_pairs().contains((key, tree[key])));
        assert(tree_rows_view(rows).to_set().contains((key, tree[key])));
        assert(tree_rows_view(rows).contains((key, tree[key])));
        let index = tree_rows_view(rows).lemma_contains_to_index((key, tree[key]));
        assert(rows[index].0@ == path);
        assert(rows_has_path(rows, 0, path));
        match rows_entry(rows, 0, path) {
            None => { assert(false); },
            Some(fields) => {
                let found = choose|found: int| 0 <= found < rows.len()
                    && rows[found].0@ == path && tree_entry_fields(rows[found].1) == fields;
                if found < index {
                    identifiers::byte_lt_strict(rows[found].0.bytes(), rows[index].0.bytes());
                    assert(false);
                } else if index < found {
                    identifiers::byte_lt_strict(rows[index].0.bytes(), rows[found].0.bytes());
                    assert(false);
                }
                assert(found == index);
                assert(fields == tree_entry_fields(&tree[key]));
            },
        }
    } else {
        assert forall|k: int| 0 <= k < rows.len()
            implies (#[trigger] rows[k]).0@ != path by {
            assert(tree_rows_view(rows).contains(tree_rows_view(rows)[k]));
            assert(tree.kv_pairs().contains(tree_rows_view(rows)[k]));
            assert(tree.contains_key(*rows[k].0));
            if rows[k].0@ == path { assert(map_has_path(tree, path)); }
        }
        rows_entry_absent(rows, 0, path);
    }
}

pub fn delta_for_trees(a: &Tree, b: &Tree) -> (out: Vec<Delta>)
    ensures
        delta_fields_ordered(delta_records_view(out@)),
        forall|path: Seq<char>| #[trigger] delta_at_path(delta_records_view(out@), path)
            == entry_delta_values(path, map_entry_fields(a@, path), map_entry_fields(b@, path)),
{
    let left = tree_rows(a);
    let right = tree_rows(b);
    let out = delta_for_rows(&left, &right);
    proof {
        assert(delta_records_view(out@) == tree_merge_fields(left@, right@, 0, 0));
        tree_merge_ordered(left@, right@, 0, 0);
        assert forall|path: Seq<char>|
            #[trigger] delta_at_path(delta_records_view(out@), path)
                == entry_delta_values(path, map_entry_fields(a@, path), map_entry_fields(b@, path)) by {
            tree_merge_path_projection(left@, right@, 0, 0, path);
            map_rows_entry(a@, left@, path);
            map_rows_entry(b@, right@, path);
            assert(rows_entry(left@, 0, path) == map_entry_fields(a@, path));
            assert(rows_entry(right@, 0, path) == map_entry_fields(b@, path));
            assert(delta_at_path(tree_merge_fields(left@, right@, 0, 0), path)
                == entry_delta_values(path, rows_entry(left@, 0, path), rows_entry(right@, 0, path)));
            assert(delta_at_path(delta_records_view(out@), path)
                == entry_delta_values(path, map_entry_fields(a@, path), map_entry_fields(b@, path)));
        }
    }
    out
}

fn optional_text_equal(left: &Option<String>, right: &Option<String>) -> (equal: bool)
    ensures equal == (optional_text(left) == optional_text(right)),
{
    match (left, right) {
        (Some(a), Some(b)) => identifiers::equal_text(a.as_str(), b.as_str()),
        (None, None) => true,
        _ => false,
    }
}

impl Clone for Delta {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        Self { kind: self.kind.clone(), path: self.path.clone(),
            old_blob: clone_optional_text(&self.old_blob),
            new_blob: clone_optional_text(&self.new_blob),
            old_mode: clone_optional_text(&self.old_mode),
            new_mode: clone_optional_text(&self.new_mode) }
    }
}

#[verifier::inline]
pub open spec fn delta_equal(left: &Delta, right: &Delta) -> bool {
    left.kind@ == right.kind@ && left.path@ == right.path@
        && optional_text(&left.old_blob) == optional_text(&right.old_blob)
        && optional_text(&left.new_blob) == optional_text(&right.new_blob)
        && optional_text(&left.old_mode) == optional_text(&right.old_mode)
        && optional_text(&left.new_mode) == optional_text(&right.new_mode)
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Delta {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { delta_equal(self, other) }
}

impl PartialEq for Delta {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == delta_equal(self, other),
    {
        identifiers::equal_text(self.kind.as_str(), other.kind.as_str())
            && identifiers::equal_text(self.path.as_str(), other.path.as_str())
            && optional_text_equal(&self.old_blob, &other.old_blob)
            && optional_text_equal(&self.new_blob, &other.new_blob)
            && optional_text_equal(&self.old_mode, &other.old_mode)
            && optional_text_equal(&self.new_mode, &other.new_mode)
    }
}

impl Eq for Delta {}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Finding {
    pub code: String,
    pub level: String,
    pub message: String,
    pub delta_refs: Vec<String>,
    pub causes: Vec<String>,
}

/// 记录已经分类的问题；调用方仍负责选择严重程度和字段。
pub fn append_finding(
    status: &mut WireStatus, findings: &mut Vec<Finding>, severity: WireStatus, finding: Finding,
)
    ensures
        *status == (if old(status).rank_spec() >= severity.rank_spec() { *old(status) } else { severity }),
        findings@ == old(findings)@.push(finding),
{
    *status = status.clone().maximum(severity);
    findings.push(finding);
}

pub open spec fn finding_fields(finding: &Finding) -> bool {
    finding.code@.len() > 0 && finding.message@.len() > 0
        && (finding.level@ == "info"@ || finding.level@ == "warning"@
            || finding.level@ == "error"@)
        && finding.delta_refs@.len() > 0
}

/// 只判断原协议要求的字段形状；引用是否属于实际请求仍由该请求检查。
pub fn valid_finding_fields(finding: &Finding) -> (valid: bool)
    ensures valid == finding_fields(finding),
{
    !finding.code.as_str().is_empty() && !finding.message.as_str().is_empty()
        && (identifiers::equal_text(finding.level.as_str(), "info")
            || identifiers::equal_text(finding.level.as_str(), "warning")
            || identifiers::equal_text(finding.level.as_str(), "error"))
        && finding.delta_refs.len() > 0
}

#[verifier::inline]
pub open spec fn record_reference(reference: Seq<char>) -> bool {
    let bytes = vstd::utf8::encode_utf8(reference);
    bytes.len() > 0 && bytes[0] == 47
}

pub open spec fn delta_reference(reference: Seq<char>, entries: &[Delta]) -> bool {
    exists|i: int| 0 <= i < entries.len() && #[trigger] entries[i].path@ == reference
}

#[verifier::inline]
pub open spec fn reference_allowed(reference: Seq<char>, delta: Option<&[Delta]>) -> bool {
    record_reference(reference) || match delta {
        None => false,
        Some(entries) => delta_reference(reference, entries),
    }
}

/// None 对应初始库存；普通请求直接借用自己的完整 DELTA。
pub fn valid_finding_reference(reference: &str, delta: Option<&[Delta]>) -> (valid: bool)
    ensures valid == reference_allowed(reference@, delta),
{
    let bytes = reference.as_bytes();
    if bytes.len() > 0 && bytes[0] == 47 { return true; }
    match delta {
        None => false,
        Some(entries) => {
            let mut i = 0;
            while i < entries.len()
                invariant
                    0 <= i <= entries.len(),
                    delta == Some(entries),
                    !record_reference(reference@),
                    forall|j: int| 0 <= j < i ==> #[trigger] entries[j].path@ != reference@,
                decreases entries.len() - i,
            {
                if identifiers::equal_text(entries[i].path.as_str(), reference) {
                    assert(entries[i as int].path@ == reference@);
                    assert(exists|j: int| 0 <= j < entries.len()
                        && #[trigger] entries[j].path@ == reference@);
                    assert(delta_reference(reference@, entries));
                    assert(reference_allowed(reference@, delta));
                    return true;
                }
                i += 1;
            }
            false
        }
    }
}

pub open spec fn finding_references(finding: &Finding, delta: Option<&[Delta]>) -> bool {
    forall|i: int| 0 <= i < finding.delta_refs.len()
        ==> reference_allowed(#[trigger] finding.delta_refs[i]@, delta)
}

pub fn valid_finding_references(finding: &Finding, delta: Option<&[Delta]>) -> (valid: bool)
    ensures valid == finding_references(finding, delta),
{
    let mut i = 0;
    while i < finding.delta_refs.len()
        invariant
            0 <= i <= finding.delta_refs.len(),
            forall|j: int| 0 <= j < i
                ==> reference_allowed(#[trigger] finding.delta_refs[j]@, delta),
        decreases finding.delta_refs.len() - i,
    {
        if !valid_finding_reference(finding.delta_refs[i].as_str(), delta) {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Evidence {
    pub path: String,
    pub sha256: String,
    pub kind: String,
}

impl Clone for Evidence {
    fn clone(&self) -> (out: Self)
        ensures out.path == self.path, out.sha256 == self.sha256, out.kind == self.kind,
    {
        Self { path: self.path.clone(), sha256: self.sha256.clone(), kind: self.kind.clone() }
    }
}

pub open spec fn evidence_identity_bytes(path: Seq<u8>, kind: Seq<char>, sha256: Seq<u8>) -> bool {
    let prefix = ".chrono-harness/state/".spec_bytes();
    path.len() >= prefix.len() && path.take(prefix.len() as int) == prefix
        && kind.len() > 0 && identifiers::digest_format(sha256)
}

pub open spec fn evidence_record_identity(entry: &Evidence) -> bool {
    evidence_identity_bytes(vstd::utf8::encode_utf8(entry.path@), entry.kind@,
        vstd::utf8::encode_utf8(entry.sha256@))
}

pub open spec fn evidence_hash_result<S, H>(
    state: S, bytes: Vec<u8>, expected: Seq<char>, hash: H,
    out: (S, Result<(), String>),
) -> bool where H: Fn(S, &[u8]) -> (S, String) {
    exists|raw: &[u8], after: S, digest: String| raw@ == bytes@
        && hash.ensures((state, raw), (after, digest))
        && out.0 == after
        && match out.1 {
            Ok(()) => digest@ == expected,
            Err(error) => digest@ != expected
                && error@ == "E_PROTOCOL: missing/tampered evidence"@,
        }
}

fn check_evidence_hash<S, H>(state: S, bytes: Vec<u8>, expected: &str, hash: H)
    -> (out: (S, Result<(), String>))
    where H: Fn(S, &[u8]) -> (S, String),
    requires forall|s: S, raw: &[u8]| #[trigger] hash.requires((s, raw)),
    ensures evidence_hash_result(state, bytes, expected@, hash, out),
{
    let raw = bytes.as_slice();
    let (after, digest) = hash(state, raw);
    if identifiers::equal_text(digest.as_str(), expected) {
        (after, Ok(()))
    } else { (after, Err("E_PROTOCOL: missing/tampered evidence".to_owned())) }
}

pub open spec fn evidence_read_result<S, P, B, H>(
    state: S, path: &P, expected: Seq<char>, read: B, hash: H,
    out: (S, Result<(), String>),
) -> bool where B: Fn(S, &P) -> (S, Result<Vec<u8>, String>),
    H: Fn(S, &[u8]) -> (S, String),
{
    exists|after: S, observed: Result<Vec<u8>, String>|
        read.ensures((state, path), (after, observed))
        && match observed {
            Err(error) => out == (after, Err::<(), String>(error)),
            Ok(bytes) => evidence_hash_result(after, bytes, expected, hash, out),
        }
}

fn check_evidence_read<S, P, B, H>(
    state: S, path: &P, expected: &str, read: B, hash: H,
) -> (out: (S, Result<(), String>))
    where B: Fn(S, &P) -> (S, Result<Vec<u8>, String>),
        H: Fn(S, &[u8]) -> (S, String),
    requires read.requires((state, path)),
        forall|s: S, raw: &[u8]| #[trigger] hash.requires((s, raw)),
    ensures evidence_read_result(state, path, expected@, read, hash, out),
{
    let (after, observed) = read(state, path);
    match observed {
        Err(error) => (after, Err(error)),
        Ok(bytes) => check_evidence_hash(after, bytes, expected, hash),
    }
}

pub open spec fn evidence_file_result<S, P, F, B, H>(
    state: S, path: &P, expected: Seq<char>, is_file: F, read: B, hash: H,
    out: (S, Result<(), String>),
) -> bool where F: Fn(S, &P) -> (S, bool),
    B: Fn(S, &P) -> (S, Result<Vec<u8>, String>), H: Fn(S, &[u8]) -> (S, String),
{
    exists|after: S, regular: bool| is_file.ensures((state, path), (after, regular))
        && if regular { evidence_read_result(after, path, expected, read, hash, out) }
        else { out.0 == after && match out.1 {
            Ok(()) => false,
            Err(error) => error@ == "E_PROTOCOL: missing/tampered evidence"@,
        } }
}

fn check_evidence_file<S, P, F, B, H>(
    state: S, path: &P, expected: &str, is_file: F, read: B, hash: H,
) -> (out: (S, Result<(), String>))
    where F: Fn(S, &P) -> (S, bool),
        B: Fn(S, &P) -> (S, Result<Vec<u8>, String>), H: Fn(S, &[u8]) -> (S, String),
    requires is_file.requires((state, path)),
        forall|s: S, p: &P| #[trigger] read.requires((s, p)),
        forall|s: S, raw: &[u8]| #[trigger] hash.requires((s, raw)),
    ensures evidence_file_result(state, path, expected@, is_file, read, hash, out),
{
    let (after, regular) = is_file(state, path);
    if !regular {
        (after, Err("E_PROTOCOL: missing/tampered evidence".to_owned()))
    } else { check_evidence_read(after, path, expected, read, hash) }
}

pub open spec fn evidence_entry_result<S, P, R, F, B, H>(
    state: S, entry: &Evidence, resolve: R, is_file: F, read: B, hash: H,
    out: (S, Result<(), String>),
) -> bool where R: Fn(S, &str) -> (S, Result<P, String>), F: Fn(S, &P) -> (S, bool),
    B: Fn(S, &P) -> (S, Result<Vec<u8>, String>), H: Fn(S, &[u8]) -> (S, String),
{
    if !evidence_record_identity(entry) {
        out.0 == state && match out.1 {
            Ok(()) => false,
            Err(error) => error@ == "E_PROTOCOL: invalid evidence identity"@,
        }
    } else {
        exists|argument: &str, after: S, observed: Result<P, String>|
            argument@ == entry.path@
            && resolve.ensures((state, argument), (after, observed))
            && match observed {
                Err(error) => out == (after, Err::<(), String>(error)),
                Ok(path) => evidence_file_result(after, &path, entry.sha256@,
                    is_file, read, hash, out),
            }
    }
}

/// 各阶段只接收真实观察；首错停止，完整错误和全部状态归本函数维护。
pub fn check_evidence_entry<S, P, R, F, B, H>(
    state: S, entry: &Evidence, resolve: R, is_file: F, read: B, hash: H,
) -> (out: (S, Result<(), String>))
    where R: Fn(S, &str) -> (S, Result<P, String>), F: Fn(S, &P) -> (S, bool),
        B: Fn(S, &P) -> (S, Result<Vec<u8>, String>), H: Fn(S, &[u8]) -> (S, String),
    requires forall|s: S, path: &str| #[trigger] resolve.requires((s, path)),
        forall|s: S, path: &P| #[trigger] is_file.requires((s, path)),
        forall|s: S, path: &P| #[trigger] read.requires((s, path)),
        forall|s: S, raw: &[u8]| #[trigger] hash.requires((s, raw)),
    ensures evidence_entry_result(state, entry, resolve, is_file, read, hash, out),
{
    if !valid_evidence_identity(entry.path.as_str(), entry.kind.as_str(), entry.sha256.as_str()) {
        return (state, Err("E_PROTOCOL: invalid evidence identity".to_owned()));
    }
    let argument = entry.path.as_str();
    let (after, observed) = resolve(state, argument);
    match observed {
        Err(error) => (after, Err(error)),
        Ok(path) => check_evidence_file(after, &path, entry.sha256.as_str(), is_file, read, hash),
    }
}

pub open spec fn evidence_prefix<S, P, R, F, B, H>(
    states: Seq<S>, initial: S, entries: Seq<Evidence>, resolve: R, is_file: F, read: B, hash: H,
) -> bool where R: Fn(S, &str) -> (S, Result<P, String>), F: Fn(S, &P) -> (S, bool),
    B: Fn(S, &P) -> (S, Result<Vec<u8>, String>), H: Fn(S, &[u8]) -> (S, String),
{
    1 <= states.len() <= entries.len() + 1 && states[0] == initial
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] evidence_entry_result(states[i], &entries[i], resolve, is_file, read, hash,
                (states[i + 1], Ok::<(), String>(())))
}

pub open spec fn evidence_sequence_result<S, P, R, F, B, H>(
    initial: S, entries: Seq<Evidence>, resolve: R, is_file: F, read: B, hash: H,
    out: (S, Result<(), String>),
) -> bool where R: Fn(S, &str) -> (S, Result<P, String>), F: Fn(S, &P) -> (S, bool),
    B: Fn(S, &P) -> (S, Result<Vec<u8>, String>), H: Fn(S, &[u8]) -> (S, String),
{
    exists|states: Seq<S>| evidence_prefix(states, initial, entries, resolve, is_file, read, hash)
        && match out.1 {
            Ok(()) => states.len() == entries.len() + 1 && out.0 == states.last(),
            Err(_) => states.len() <= entries.len()
                && evidence_entry_result(states.last(), &entries[states.len() - 1],
                    resolve, is_file, read, hash, out),
        }
}

pub proof fn evidence_prefix_at<S, P, R, F, B, H>(
    states: Seq<S>, initial: S, entries: Seq<Evidence>, resolve: R, is_file: F, read: B, hash: H,
    at: int,
) where R: Fn(S, &str) -> (S, Result<P, String>), F: Fn(S, &P) -> (S, bool),
    B: Fn(S, &P) -> (S, Result<Vec<u8>, String>), H: Fn(S, &[u8]) -> (S, String),
    requires evidence_prefix(states, initial, entries, resolve, is_file, read, hash),
        0 <= at < states.len() - 1,
    ensures evidence_entry_result(states[at], &entries[at], resolve, is_file, read, hash,
        (states[at + 1], Ok::<(), String>(()))),
{
}

pub fn check_evidence_sequence<S, P, R, F, B, H>(
    state: S, entries: &[Evidence], resolve: R, is_file: F, read: B, hash: H,
) -> (out: (S, Result<(), String>))
    where R: Fn(S, &str) -> (S, Result<P, String>), F: Fn(S, &P) -> (S, bool),
        B: Fn(S, &P) -> (S, Result<Vec<u8>, String>), H: Fn(S, &[u8]) -> (S, String),
    requires forall|s: S, path: &str| #[trigger] resolve.requires((s, path)),
        forall|s: S, path: &P| #[trigger] is_file.requires((s, path)),
        forall|s: S, path: &P| #[trigger] read.requires((s, path)),
        forall|s: S, raw: &[u8]| #[trigger] hash.requires((s, raw)),
    ensures evidence_sequence_result(state, entries@, resolve, is_file, read, hash, out),
{
    let ghost initial = state;
    let ghost states = seq![state];
    let mut current = state;
    let mut i = 0;
    while i < entries.len()
        invariant 0 <= i <= entries.len(), states.len() == i + 1, initial == state,
            states.last() == current,
            evidence_prefix(states, initial, entries@, resolve, is_file, read, hash),
            forall|s: S, path: &str| #[trigger] resolve.requires((s, path)),
            forall|s: S, path: &P| #[trigger] is_file.requires((s, path)),
            forall|s: S, path: &P| #[trigger] read.requires((s, path)),
            forall|s: S, raw: &[u8]| #[trigger] hash.requires((s, raw)),
        decreases entries.len() - i,
    {
        let (after, result) = check_evidence_entry(current, &entries[i], &resolve, &is_file, &read, &hash);
        match result {
            Err(error) => {
                proof {
                    assert(states.len() <= entries.len());
                    assert(evidence_entry_result(states.last(), &entries@[states.len() - 1],
                        resolve, is_file, read, hash, (after, Err::<(), String>(error))));
                    assert(evidence_sequence_result(initial, entries@, resolve, is_file, read, hash,
                        (after, Err::<(), String>(error))));
                }
                return (after, Err(error));
            },
            Ok(()) => {},
        }
        proof {
            let previous = states;
            states = states.push(after);
            assert forall|j: int| 0 <= j < states.len() - 1 implies
                #[trigger] evidence_entry_result(states[j], &entries@[j], resolve, is_file, read, hash,
                    (states[j + 1], Ok::<(), String>(()))) by {
                if j < previous.len() - 1 {
                    assert(evidence_entry_result(previous[j], &entries@[j], resolve, is_file, read, hash,
                        (previous[j + 1], Ok::<(), String>(()))) );
                }
            }
        }
        current = after;
        i += 1;
    }
    (current, Ok(()))
}

/// 头部先验失败不观察证据；成功才逐条消费原证据列表。
pub fn validate_response_evidence<S, P, R, F, B, H>(
    header: Result<(), String>, state: S, entries: &[Evidence],
    resolve: R, is_file: F, read: B, hash: H,
) -> (out: (S, Result<(), String>))
    where R: Fn(S, &str) -> (S, Result<P, String>), F: Fn(S, &P) -> (S, bool),
        B: Fn(S, &P) -> (S, Result<Vec<u8>, String>), H: Fn(S, &[u8]) -> (S, String),
    requires forall|s: S, path: &str| #[trigger] resolve.requires((s, path)),
        forall|s: S, path: &P| #[trigger] is_file.requires((s, path)),
        forall|s: S, path: &P| #[trigger] read.requires((s, path)),
        forall|s: S, raw: &[u8]| #[trigger] hash.requires((s, raw)),
    ensures match header {
        Err(error) => out == (state, Err::<(), String>(error)),
        Ok(()) => evidence_sequence_result(state, entries@, resolve, is_file, read, hash, out),
    },
{
    match header {
        Err(error) => (state, Err(error)),
        Ok(()) => check_evidence_sequence(state, entries, resolve, is_file, read, hash),
    }
}

pub open spec fn evidence_identity(path: &str, kind: &str, sha256: &str) -> bool {
    evidence_identity_bytes(path.spec_bytes(), kind@, sha256.spec_bytes())
}

/// 保留原前缀合同，不在这里增加词法路径或文件存在条件。
pub fn valid_evidence_identity(path: &str, kind: &str, sha256: &str) -> (valid: bool)
    ensures valid == evidence_identity(path, kind, sha256),
{
    let bytes = path.as_bytes();
    let prefix = ".chrono-harness/state/".as_bytes();
    if bytes.len() < prefix.len() { return false; }
    let (head, _) = bytes.split_at(prefix.len());
    identifiers::equal_bytes(head, prefix) && !kind.is_empty()
        && identifiers::valid_digest_text(sha256)
}

impl Clone for Finding {
    fn clone(&self) -> (out: Self)
        ensures out.code == self.code, out.level == self.level, out.message == self.message,
            out.delta_refs@ == self.delta_refs@, out.causes@ == self.causes@,
    {
        Self { code: self.code.clone(), level: self.level.clone(), message: self.message.clone(),
            delta_refs: self.delta_refs.clone(), causes: self.causes.clone() }
    }
}

pub open spec fn finding_level_present(findings: Seq<Finding>, level: Seq<char>) -> bool {
    exists|i: int| 0 <= i < findings.len() && (#[trigger] findings[i]).level@ == level
}

proof fn finding_level_prefix_step(findings: Seq<Finding>, level: Seq<char>, at: int)
    requires 0 <= at < findings.len(),
    ensures finding_level_present(findings.take(at + 1), level)
        == (finding_level_present(findings.take(at), level) || findings[at].level@ == level),
{
    let before = findings.take(at);
    let after = findings.take(at + 1);
    if finding_level_present(after, level) {
        let i = choose|i: int| 0 <= i < after.len() && (#[trigger] after[i]).level@ == level;
        if i < at {
            assert(before[i].level@ == level);
            assert(finding_level_present(before, level));
        } else {
            assert(i == at && findings[at].level@ == level);
        }
    }
    if finding_level_present(before, level) {
        let i = choose|i: int| 0 <= i < before.len() && (#[trigger] before[i]).level@ == level;
        assert(after[i].level@ == level);
        assert(finding_level_present(after, level));
    }
    if findings[at].level@ == level {
        assert(after[at].level@ == level);
        assert(finding_level_present(after, level));
    }
}

pub open spec fn finding_status_agreement(status: &WireStatus, findings: Seq<Finding>) -> bool {
    let errors = finding_level_present(findings, "error"@);
    let warnings = finding_level_present(findings, "warning"@);
    (matches!(status, WireStatus::Fail | WireStatus::Error) == errors)
        && (*status == WireStatus::Pass ==> !warnings)
        && (*status == WireStatus::Warn ==> warnings)
}

/// 直接检查真实 finding；字段有效性和响应身份另由各自所有者检查。
pub fn findings_match_status(status: &WireStatus, findings: &[Finding]) -> (matches: bool)
    ensures matches == finding_status_agreement(status, findings@),
{
    let mut at = 0;
    let mut errors = false;
    let mut warnings = false;
    while at < findings.len()
        invariant at <= findings.len(),
            errors == finding_level_present(findings@.take(at as int), "error"@),
            warnings == finding_level_present(findings@.take(at as int), "warning"@),
        decreases findings.len() - at,
    {
        let level = findings[at].level.as_str();
        errors = errors || identifiers::equal_text(level, "error");
        warnings = warnings || identifiers::equal_text(level, "warning");
        proof {
            finding_level_prefix_step(findings@, "error"@, at as int);
            finding_level_prefix_step(findings@, "warning"@, at as int);
        }
        at += 1;
    }
    assert(findings@.take(at as int) =~= findings@);
    match status {
        WireStatus::Pass => !errors && !warnings,
        WireStatus::Warn => !errors && warnings,
        WireStatus::Fail | WireStatus::Error => errors,
    }
}

pub open spec fn response_findings_valid(findings: Seq<Finding>, delta: Option<&[Delta]>) -> bool {
    forall|i: int| 0 <= i < findings.len()
        ==> finding_fields(&(#[trigger] findings[i])) && finding_references(&findings[i], delta)
}

/// 同时核对每条真实 finding 的字段和引用，不接受预先计算的有效标志。
pub fn valid_response_findings(findings: &[Finding], delta: Option<&[Delta]>) -> (valid: bool)
    ensures valid == response_findings_valid(findings@, delta),
{
    let mut i = 0;
    while i < findings.len()
        invariant
            0 <= i <= findings.len(),
            forall|j: int| 0 <= j < i
                ==> finding_fields(&(#[trigger] findings@[j]))
                    && finding_references(&findings@[j], delta),
        decreases findings.len() - i,
    {
        if !valid_finding_fields(&findings[i]) || !valid_finding_references(&findings[i], delta) {
            return false;
        }
        i += 1;
    }
    true
}

pub open spec fn response_fields_error(
    protocol: &str, request_id: &str, judge_id: &str, status: &WireStatus, exit: i32,
    expected_protocol: &str, expected_request_id: &str, expected_judge_id: &str,
    findings: Seq<Finding>, delta: Option<&[Delta]>,
) -> Option<Seq<char>> {
    if protocol@ != expected_protocol@ || request_id@ != expected_request_id@
        || judge_id@ != expected_judge_id@ || exit != status.exit_spec() {
        Some("E_PROTOCOL: response identity/status/exit mismatch"@)
    } else if !response_findings_valid(findings, delta) {
        Some("E_PROTOCOL: invalid finding"@)
    } else if !finding_status_agreement(status, findings) {
        Some("E_PROTOCOL: finding/status disagreement"@)
    } else { None }
}

/// 普通与初始响应共用原错误优先级；成功后仍须由证据 IO 负责方完成验证。
pub fn validate_response_fields(
    protocol: &str, request_id: &str, judge_id: &str, status: &WireStatus, exit: i32,
    expected_protocol: &str, expected_request_id: &str, expected_judge_id: &str,
    findings: &[Finding], delta: Option<&[Delta]>,
) -> (out: Result<(), String>)
    ensures
        out.is_ok() == response_fields_error(protocol, request_id, judge_id, status, exit,
            expected_protocol, expected_request_id, expected_judge_id, findings@, delta).is_none(),
        match out {
            Ok(()) => true,
            Err(message) => response_fields_error(protocol, request_id, judge_id, status, exit,
                expected_protocol, expected_request_id, expected_judge_id, findings@, delta)
                    == Some(message@),
        },
{
    if !wire_header_matches(protocol, request_id, judge_id, status, exit,
        expected_protocol, expected_request_id, expected_judge_id) {
        return Err("E_PROTOCOL: response identity/status/exit mismatch".to_string());
    }
    if !valid_response_findings(findings, delta) {
        return Err("E_PROTOCOL: invalid finding".to_string());
    }
    if !findings_match_status(status, findings) {
        return Err("E_PROTOCOL: finding/status disagreement".to_string());
    }
    Ok(())
}

pub open spec fn record_response_error(
    protocol: &str, request_id: &str, judge_id: &str, status: &WireStatus, exit: i32,
    expected_protocol: &str, expected_request_id: &str, expected_judge_id: &str,
    findings: Seq<Finding>,
) -> Option<Seq<char>> {
    if protocol@ != expected_protocol@ || request_id@ != expected_request_id@
        || judge_id@ != expected_judge_id@ || exit != status.exit_spec() {
        Some("response identity/status/exit mismatch"@)
    } else if !finding_status_agreement(status, findings) {
        Some("response findings/status disagree"@)
    } else { None }
}

/// 留存报告的头部与状态审计；原件、JSON 与其它协议条件仍由其负责方处理。
pub fn validate_record_response(
    protocol: &str, request_id: &str, judge_id: &str, status: &WireStatus, exit: i32,
    expected_protocol: &str, expected_request_id: &str, expected_judge_id: &str,
    findings: &[Finding],
) -> (out: Result<(), String>)
    ensures
        out.is_ok() == record_response_error(protocol, request_id, judge_id, status, exit,
            expected_protocol, expected_request_id, expected_judge_id, findings@).is_none(),
        match out {
            Ok(()) => true,
            Err(message) => record_response_error(protocol, request_id, judge_id, status, exit,
                expected_protocol, expected_request_id, expected_judge_id, findings@)
                    == Some(message@),
        },
{
    if !wire_header_matches(protocol, request_id, judge_id, status, exit,
        expected_protocol, expected_request_id, expected_judge_id) {
        return Err("response identity/status/exit mismatch".to_string());
    }
    if !findings_match_status(status, findings) {
        return Err("response findings/status disagree".to_string());
    }
    Ok(())
}

impl Clone for Status {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        match self {
            Self::Passed => Self::Passed,
            Self::Failed => Self::Failed,
            Self::NotRequired => Self::NotRequired,
            Self::Blocked => Self::Blocked,
        }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Status {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { *self == *other }
}

impl PartialEq for Status {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (*self == *other),
    {
        match self {
            Self::Passed => matches!(other, Self::Passed),
            Self::Failed => matches!(other, Self::Failed),
            Self::NotRequired => matches!(other, Self::NotRequired),
            Self::Blocked => matches!(other, Self::Blocked),
        }
    }
}

impl Eq for Status {}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct CheckResult {
    pub id: String,
    pub status: Status,
    pub cause: String,
    pub exit_code: Option<i32>,
}

impl Clone for CheckResult {
    fn clone(&self) -> (out: Self)
        ensures out.id@ == self.id@, out.status == self.status,
            out.cause@ == self.cause@, out.exit_code == self.exit_code,
    {
        Self {
            id: self.id.clone(),
            status: self.status.clone(),
            cause: self.cause.clone(),
            exit_code: self.exit_code,
        }
    }
}

}
