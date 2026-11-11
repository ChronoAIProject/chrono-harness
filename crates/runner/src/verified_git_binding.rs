//! Git 事实读取的声明与可选预期；实际文件／进程观察归适配器。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

verus! {

pub enum GitConfigSchema {
    Direct,
    V1,
    V2,
    V3,
    Unsupported,
}

impl GitConfigSchema {
    pub fn is_selector(&self) -> (valid: bool)
        ensures valid == (self is V1 || self is V2 || self is V3),
    {
        matches!(self, Self::V1 | Self::V2 | Self::V3)
    }

    pub fn has_identity(&self) -> (valid: bool)
        ensures valid == (self is V2 || self is V3),
    {
        matches!(self, Self::V2 | Self::V3)
    }

    pub fn has_registry_path(&self) -> (valid: bool)
        ensures valid == (self is V3),
    {
        matches!(self, Self::V3)
    }
}

pub open spec fn text_has_prefix(text: &str, prefix: &str) -> bool {
    text.spec_bytes().len() >= prefix.spec_bytes().len()
        && text.spec_bytes().take(prefix.spec_bytes().len() as int) == prefix.spec_bytes()
}

fn has_prefix(text: &str, prefix: &str) -> (valid: bool)
    ensures valid == text_has_prefix(text, prefix),
{
    let bytes = text.as_bytes();
    let prefix = prefix.as_bytes();
    bytes.len() >= prefix.len() && identifiers::equal_bytes(&bytes[0..prefix.len()], prefix)
}

pub open spec fn git_config_schema(schema: Option<&str>) -> GitConfigSchema {
    match schema {
        None => GitConfigSchema::Direct,
        Some(text) => {
            if text@ == "chrono-git-configs/v1"@ { GitConfigSchema::V1 }
            else if text@ == "chrono-git-configs/v2"@ { GitConfigSchema::V2 }
            else if text@ == "chrono-git-configs/v3"@ { GitConfigSchema::V3 }
            else if text_has_prefix(text, "chrono-git-configs/") {
                GitConfigSchema::Unsupported
            } else { GitConfigSchema::Direct }
        },
    }
}

/// 完整版本先于家族前缀；缺失或非文本观察使用直接配置分支。
pub fn classify_git_config_schema(schema: Option<&str>) -> (out: GitConfigSchema)
    ensures out == git_config_schema(schema),
{
    let Some(text) = schema else { return GitConfigSchema::Direct; };
    if identifiers::equal_text(text, "chrono-git-configs/v1") { GitConfigSchema::V1 }
    else if identifiers::equal_text(text, "chrono-git-configs/v2") { GitConfigSchema::V2 }
    else if identifiers::equal_text(text, "chrono-git-configs/v3") { GitConfigSchema::V3 }
    else if has_prefix(text, "chrono-git-configs/") {
        GitConfigSchema::Unsupported
    } else { GitConfigSchema::Direct }
}

pub fn admit_selector_object(fields: Option<usize>, has_platforms: bool)
    -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => fields == Some(2usize) && has_platforms,
        Err(error) => if fields.is_none() {
            error@ == "Git config selector object required"@
        } else {
            (fields != Some(2usize) || !has_platforms)
                && error@ == "Git config selector requires only schema and platforms"@
        },
    },
{
    match fields {
        None => Err("Git config selector object required"),
        Some(2) if has_platforms => Ok(()),
        Some(_) => Err("Git config selector requires only schema and platforms"),
    }
}

pub fn admit_selector_platforms<T>(object: Option<(&T, usize)>)
    -> (out: Result<&T, &'static str>)
    ensures match out {
        Ok(platforms) => match object {
            Some((original, count)) => count > 0 && platforms == original,
            None => false,
        },
        Err(error) => (match object { Some((_, count)) => count == 0, None => true })
            && error@ == "Git config selector requires nonempty platforms"@,
    },
{
    match object {
        Some((platforms, count)) if count > 0 => Ok(platforms),
        _ => Err("Git config selector requires nonempty platforms"),
    }
}

/// 标准 trim 只提供原观察；非空与完整文本一致性由本函数判定。
pub fn admit_selector_platform_name(name: &str, trimmed: &str)
    -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => name@.len() > 0 && name@ == trimmed@,
        Err(error) => (name@.len() == 0 || name@ != trimmed@)
            && error@ == "Git config selector platform must be nonempty and literal"@,
    },
{
    if name.is_empty() || !identifiers::equal_text(name, trimmed) {
        Err("Git config selector platform must be nonempty and literal")
    } else { Ok(()) }
}

pub fn admit_selector_bytes(actual: &[u8], captured: &[u8])
    -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => actual@ == captured@,
        Err(error) => actual@ != captured@
            && error@ == "Git config selector changed"@,
    },
{
    if identifiers::equal_bytes(actual, captured) { Ok(()) }
    else { Err("Git config selector changed") }
}

/// 相对路径由现有路径准入负责；这里先判宿主范围，再拒绝目标指向入口自身。
pub fn admit_selector_host_path(path: &str, entry: Option<&str>)
    -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => text_has_prefix(path, ".chrono-harness/")
            && match entry { None => true, Some(origin) => origin@ != path@ },
        Err(error) => if !text_has_prefix(path, ".chrono-harness/") {
            error@ == "Git config selector paths must reside in .chrono-harness/"@
        } else {
            (exists|origin: &str| entry == Some(origin) && origin@ == path@)
                && error@ == "Git config selector cannot select itself"@
        },
    },
{
    if !has_prefix(path, ".chrono-harness/") {
        return Err("Git config selector paths must reside in .chrono-harness/");
    }
    if let Some(origin) = entry {
        if identifiers::equal_text(origin, path) {
            return Err("Git config selector cannot select itself");
        }
    }
    Ok(())
}

/// 选择器只能指向直接 v3／v4；schema 键即使为 null 也算存在。
pub fn admit_git_selector_target(version: Option<u64>, schema_present: bool)
    -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => !schema_present && (version == Some(3u64) || version == Some(4u64)),
        Err(message) => (schema_present || (version != Some(3u64) && version != Some(4u64)))
            && message@ == "Git config selector requires a direct full-v3 policy or v4 policy, never another selector"@,
    },
{
    if !schema_present && matches!(version, Some(3 | 4)) { Ok(()) }
    else { Err("Git config selector requires a direct full-v3 policy or v4 policy, never another selector") }
}

pub open spec fn required_text_matches(observed: Option<&str>, expected: &str) -> bool {
    match observed {
        Some(text) => text@ == expected@,
        None => false,
    }
}

fn matches_required(observed: Option<&str>, expected: &str) -> (valid: bool)
    ensures valid == required_text_matches(observed, expected),
{
    match observed {
        Some(text) => identifiers::equal_text(text, expected),
        None => false,
    }
}

pub open spec fn binding_text_present(observed: Option<&str>) -> bool {
    match observed { Some(text) => text@.len() > 0, None => false }
}

fn has_binding_text(observed: Option<&str>) -> (present: bool)
    ensures present == binding_text_present(observed),
{
    match observed { Some(text) => !text.is_empty(), None => false }
}

/// 缺失和空文本使用原字段名诊断；成功保留完整原切片。
pub struct MissingBindingText<'a> {
    pub prefix: &'static str,
    pub field: &'a str,
}

pub fn required_binding_text<'a, 'f>(observed: Option<&'a str>, field: &'f str)
    -> (out: Result<&'a str, MissingBindingText<'f>>)
    ensures match out {
        Ok(text) => observed == Some(text) && text@.len() > 0,
        Err(message) => !binding_text_present(observed)
            && message.prefix@ == "missing "@ && message.field@ == field@,
    },
{
    if has_binding_text(observed) { return Ok(observed.unwrap()); }
    Err(MissingBindingText { prefix: "missing ", field })
}

/// 目标策略必须声明执行物；缺声明先于任一字段不匹配。
pub fn admit_selector_binding(
    program: &str,
    digest: &str,
    tool: Option<&str>,
    input: Option<(Option<&str>, Option<&str>)>,
) -> (out: Result<(), &'static str>)
    ensures match input {
        None => out == Err::<(), &'static str>("Git identity selector requires a declared executable input"),
        Some((location, sha256)) => {
            if required_text_matches(tool, program) && required_text_matches(location, program)
                && required_text_matches(sha256, digest)
            {
                out == Ok::<(), &'static str>(())
            } else {
                out == Err::<(), &'static str>("Git config selector identity differs from selected policy binding")
            }
        },
    },
{
    let Some((location, sha256)) = input else {
        return Err("Git identity selector requires a declared executable input");
    };
    if matches_required(tool, program) && matches_required(location, program)
        && matches_required(sha256, digest)
    {
        Ok(())
    } else {
        Err("Git config selector identity differs from selected policy binding")
    }
}

pub fn valid_link(schema: u64, tool: bool, input: bool, unknown: bool) -> (valid: bool)
    ensures valid <==> ((schema == 3 || schema == 4) && tool && !unknown
        && (schema == 4 || input)),
{
    (schema == 3 || schema == 4) && tool && !unknown && (schema == 4 || input)
}

/// 有序集合的外部接口只观察完整文本的成员与插入。
pub trait SelectorSeen<'a> {
    spec fn seen(&self) -> Set<Seq<char>>;

    fn record(&mut self, text: &'a str) -> (fresh: bool)
        ensures self.seen() == old(self).seen().insert(text@),
            fresh == !old(self).seen().contains(text@);
}

pub struct SelectorRow<'a> {
    pub fields: Option<usize>,
    pub digest: Option<&'a str>,
    pub path_present: bool,
    pub path: Option<&'a str>,
    pub registry_path_present: bool,
}

pub open spec fn identity_row_shape(row: &SelectorRow, v3: bool) -> bool {
    row.fields == Some(if v3 { 3usize } else { 2usize })
        && row.path_present && (!v3 || row.registry_path_present)
}

pub open spec fn identity_row_error(
    row: &SelectorRow, v3: bool, hashes: Set<Seq<char>>, paths: Set<Seq<char>>,
) -> Option<Seq<char>> {
    if row.fields.is_none() {
        Some("Git selector configuration object required"@)
    } else if !matches!(row.digest, Some(text) if identifiers::digest_format(text.spec_bytes())) {
        Some("Git selector configuration digest required"@)
    } else if !identity_row_shape(row, v3) || hashes.contains(row.digest.unwrap()@) {
        Some("Git selector configuration requires a unique digest/path and the declared v3 registry_path only"@)
    } else {
        match row.path {
            None => Some("Git selector configuration path required"@),
            Some(path) => if paths.contains(path@) {
                Some("Git selector configuration paths must be unique"@)
            } else { None },
        }
    }
}

pub open spec fn identity_row_hashes(
    row: &SelectorRow, v3: bool, hashes: Set<Seq<char>>,
) -> Set<Seq<char>> {
    if identity_row_shape(row, v3)
        && matches!(row.digest, Some(text) if identifiers::digest_format(text.spec_bytes()))
    { hashes.insert(row.digest.unwrap()@) } else { hashes }
}

pub fn admit_identity_row<'a, S: SelectorSeen<'a>>(
    row: SelectorRow<'a>, v3: bool, hashes: &mut S, paths: &mut S,
) -> (out: Result<&'a str, &'static str>)
    ensures hashes.seen() == identity_row_hashes(&row, v3, old(hashes).seen()),
    match out {
        Ok(path) => row.path == Some(path)
            && identity_row_error(&row, v3, old(hashes).seen(), old(paths).seen()).is_none()
            && paths.seen() == old(paths).seen().insert(path@),
        Err(error) => identity_row_error(&row, v3, old(hashes).seen(), old(paths).seen()) == Some(error@)
            && paths.seen() == old(paths).seen(),
    },
{
    if row.fields.is_none() {
        return Err("Git selector configuration object required");
    }
    let Some(digest) = row.digest else {
        return Err("Git selector configuration digest required");
    };
    if !identifiers::valid_digest_text(digest) {
        return Err("Git selector configuration digest required");
    }
    if row.fields != Some(if v3 { 3 } else { 2 })
        || !row.path_present || (v3 && !row.registry_path_present)
        || !hashes.record(digest)
    {
        return Err("Git selector configuration requires a unique digest/path and the declared v3 registry_path only");
    }
    let Some(path) = row.path else {
        return Err("Git selector configuration path required");
    };
    if !paths.record(path) {
        return Err("Git selector configuration paths must be unique");
    }
    Ok(path)
}

/// JSON 适配器只交付原数组的长度及指定行的原字段观察。
pub trait SelectorRows<'a> {
    spec fn rows(&self) -> Seq<SelectorRow<'a>>;

    fn len(&self) -> (count: usize)
        ensures count == self.rows().len();

    fn row(&self, index: usize) -> (observed: SelectorRow<'a>)
        requires index < self.rows().len(),
        ensures observed == self.rows()[index as int];
}

pub open spec fn identity_prefix_hashes(rows: Seq<SelectorRow>, count: int) -> Set<Seq<char>>
    decreases count,
{
    if count <= 0 { Set::empty() }
    else {
        let previous = identity_prefix_hashes(rows, count - 1);
        match rows[count - 1].digest {
            Some(text) => previous.insert(text@),
            None => previous,
        }
    }
}

pub open spec fn identity_prefix_paths(rows: Seq<SelectorRow>, count: int) -> Set<Seq<char>>
    decreases count,
{
    if count <= 0 { Set::empty() }
    else {
        let previous = identity_prefix_paths(rows, count - 1);
        match rows[count - 1].path {
            Some(text) => previous.insert(text@),
            None => previous,
        }
    }
}

pub open spec fn identity_prefix_valid(rows: Seq<SelectorRow>, v3: bool, count: int) -> bool {
    0 <= count <= rows.len()
        && forall|i: int| 0 <= i < count ==> #[trigger] identity_row_error(
            &rows[i], v3, identity_prefix_hashes(rows, i), identity_prefix_paths(rows, i),
        ) == None::<Seq<char>>
}

pub open spec fn identity_failure_at(
    rows: Seq<SelectorRow>, v3: bool, index: int, error: Seq<char>,
) -> bool {
    0 <= index < rows.len() && identity_prefix_valid(rows, v3, index)
        && identity_row_error(&rows[index], v3,
            identity_prefix_hashes(rows, index), identity_prefix_paths(rows, index),
        ) == Some(error)
}

pub open spec fn identity_walk_failure(
    rows: Seq<SelectorRow>, v3: bool, hashes: Set<Seq<char>>, paths: Set<Seq<char>>,
    error: Seq<char>,
) -> bool {
    exists|i: int| #[trigger] identity_failure_at(rows, v3, i, error)
        && hashes == identity_row_hashes(&rows[i], v3, identity_prefix_hashes(rows, i))
        && paths == identity_prefix_paths(rows, i)
}

/// 按原顺序收集全部目标；失败精确对应首个不合格行及该行产生的集合状态。
pub fn collect_identity_paths<'a, R: SelectorRows<'a>, S: SelectorSeen<'a>>(
    rows: &R, v3: bool, hashes: &mut S, paths: &mut S,
) -> (out: Result<Vec<&'a str>, &'static str>)
    requires old(hashes).seen() == Set::<Seq<char>>::empty(),
        old(paths).seen() == Set::<Seq<char>>::empty(),
    ensures match out {
        Ok(collected) => identity_prefix_valid(rows.rows(), v3, rows.rows().len() as int)
            && collected@.len() == rows.rows().len()
            && (forall|i: int| 0 <= i < collected@.len() ==>
                rows.rows()[i].path == Some(#[trigger] collected@[i]))
            && hashes.seen() == identity_prefix_hashes(rows.rows(), rows.rows().len() as int)
            && paths.seen() == identity_prefix_paths(rows.rows(), rows.rows().len() as int),
        Err(error) => identity_walk_failure(rows.rows(), v3, hashes.seen(), paths.seen(), error@),
    },
{
    let count = rows.len();
    let mut collected = Vec::new();
    let mut i = 0;
    while i < count
        invariant i <= count, count == rows.rows().len(),
            identity_prefix_valid(rows.rows(), v3, i as int),
            collected@.len() == i,
            forall|j: int| 0 <= j < i ==>
                rows.rows()[j].path == Some(#[trigger] collected@[j]),
            hashes.seen() == identity_prefix_hashes(rows.rows(), i as int),
            paths.seen() == identity_prefix_paths(rows.rows(), i as int),
        decreases count - i,
    {
        let observed = rows.row(i);
        let ghost before_hashes = hashes.seen();
        let ghost before_paths = paths.seen();
        match admit_identity_row(observed, v3, hashes, paths) {
            Ok(path) => {
                proof {
                    assert(identity_row_error(&rows.rows()[i as int], v3,
                        before_hashes, before_paths).is_none());
                    assert(identity_row_shape(&rows.rows()[i as int], v3));
                    assert(match rows.rows()[i as int].digest {
                        Some(text) => identifiers::digest_format(text.spec_bytes()),
                        None => false,
                    });
                }
                collected.push(path);
            },
            Err(error) => {
                assert(identity_failure_at(rows.rows(), v3, i as int, error@));
                assert(hashes.seen() == identity_row_hashes(&rows.rows()[i as int], v3,
                    identity_prefix_hashes(rows.rows(), i as int)));
                assert(paths.seen() == identity_prefix_paths(rows.rows(), i as int));
                assert(identity_walk_failure(rows.rows(), v3, hashes.seen(), paths.seen(), error@));
                return Err(error);
            },
        }
        i += 1;
    }
    Ok(collected)
}

/// 只观察捕获的快照是否包含完整路径，不选择目标或读取现役执行物。
pub trait RegistryMembership {
    spec fn has_path(&self, path: &str) -> bool;

    fn contains_path(&self, path: &str) -> (present: bool)
        ensures present == self.has_path(path);
}

trait UniqueMatchRows {
    spec fn count(&self) -> nat;
    spec fn matches(&self, index: int) -> bool;

    fn len(&self) -> (count: usize)
        ensures count == self.count();

    fn matched(&self, index: usize) -> (present: bool)
        requires index < self.count(),
        ensures present == self.matches(index as int);
}

spec fn unique_match_index<R: UniqueMatchRows>(rows: &R, index: int) -> bool {
    0 <= index < rows.count() && rows.matches(index)
        && forall|j: int| 0 <= j < rows.count() && j != index ==> !#[trigger] rows.matches(j)
}

fn unique_matching_index<R: UniqueMatchRows>(rows: &R) -> (out: Option<usize>)
    ensures match out {
        Some(index) => unique_match_index(rows, index as int),
        None => forall|i: int| !#[trigger] unique_match_index(rows, i),
    },
{
    let count = rows.len();
    let mut selected: Option<usize> = None;
    let mut i = 0;
    while i < count
        invariant i <= count, count == rows.count(),
            match selected { Some(s) => s < i, None => true },
            forall|j: int| 0 <= j < i ==>
                (#[trigger] rows.matches(j) == (selected == Some(j as usize))),
        decreases count - i,
    {
        if rows.matched(i) {
            if let Some(_first) = selected {
                assert(rows.matches(_first as int));
                assert forall|j: int| !#[trigger] unique_match_index(rows, j) by {
                    if j == i { assert(_first < j && rows.matches(_first as int)); }
                    else { assert(rows.matches(i as int)); }
                }
                return None;
            }
            selected = Some(i);
        }
        i += 1;
    }
    match selected {
        Some(_index) => { assert(unique_match_index(rows, _index as int)); },
        None => {
            assert forall|j: int| !#[trigger] unique_match_index(rows, j) by {
                if 0 <= j < rows.count() { assert(!rows.matches(j)); }
            }
        },
    }
    selected
}

/// 外部 JSON 观察仅提供原数组长度及指定行的完整 id 文本。
pub trait BindingReferenceRows {
    spec fn count(&self) -> nat;
    spec fn id_at(&self, index: int) -> Option<&str>;

    fn len(&self) -> (count: usize)
        ensures count == self.count();

    fn id(&self, index: usize) -> (text: Option<&str>)
        requires index < self.count(),
        ensures text == self.id_at(index as int);
}

pub open spec fn binding_reference_matches<R: BindingReferenceRows>(
    rows: &R, index: int, id: &str,
) -> bool {
    match rows.id_at(index) { Some(text) => text@ == id@, None => false }
}

pub open spec fn unique_binding_reference_index<R: BindingReferenceRows>(
    rows: &R, id: &str, index: int,
) -> bool {
    0 <= index < rows.count() && binding_reference_matches(rows, index, id)
        && forall|j: int| 0 <= j < rows.count() && j != index ==>
            !#[trigger] binding_reference_matches(rows, j, id)
}

struct BindingMatches<'a, R: BindingReferenceRows> { rows: &'a R, id: &'a str }

impl<R: BindingReferenceRows> UniqueMatchRows for BindingMatches<'_, R> {
    spec fn count(&self) -> nat { self.rows.count() }
    spec fn matches(&self, index: int) -> bool {
        binding_reference_matches(self.rows, index, self.id)
    }
    fn len(&self) -> usize { self.rows.len() }
    fn matched(&self, index: usize) -> bool {
        match self.rows.id(index) {
            Some(text) => identifiers::equal_text(text, self.id),
            None => false,
        }
    }
}

/// 缺数组先报原错误；成功索引唯一且保留原位置，零项／多项均返回无唯一引用。
pub fn unique_binding_reference<R: BindingReferenceRows>(rows: Option<&R>, id: &str)
    -> (out: Result<Option<usize>, &'static str>)
    ensures
        out.is_err() == rows.is_none(),
        rows.is_none() ==> out == Err::<Option<usize>, &'static str>("binding collection must be array"),
        match out {
            Ok(selected) => match selected {
                Some(index) => unique_binding_reference_index(rows.unwrap(), id, index as int),
                None => forall|i: int| !#[trigger] unique_binding_reference_index(rows.unwrap(), id, i),
            },
            Err(_) => true,
        },
        match rows {
            Some(rows) => forall|i: usize| #[trigger] unique_binding_reference_index(rows, id, i as int)
                ==> out == Ok::<Option<usize>, &'static str>(Some(i)),
            None => true,
        },
{
    let Some(rows) = rows else { return Err("binding collection must be array"); };
    let observed = BindingMatches { rows, id };
    proof {
        reveal(unique_match_index);
        assert(observed.count() == rows.count());
        assert forall|i: int| #[trigger] observed.matches(i)
            == binding_reference_matches(rows, i, id) by {}
        assert forall|i: int| #[trigger] unique_match_index(&observed, i)
            == unique_binding_reference_index(rows, id, i) by {
            if unique_match_index(&observed, i) {
                assert forall|j: int| 0 <= j < rows.count() && j != i implies
                    !#[trigger] binding_reference_matches(rows, j, id) by {
                    assert(!observed.matches(j));
                }
            }
            if unique_binding_reference_index(rows, id, i) {
                assert forall|j: int| 0 <= j < observed.count() && j != i implies
                    !#[trigger] observed.matches(j) by {
                    assert(!binding_reference_matches(rows, j, id));
                }
            }
        }
    }
    let selected = unique_matching_index(&observed);
    proof {
        if selected.is_none() {
            assert forall|i: int| !#[trigger] unique_binding_reference_index(rows, id, i) by {
                assert(!unique_match_index(&observed, i));
            }
        }
        assert forall|i: usize| #[trigger] unique_binding_reference_index(rows, id, i as int)
            implies selected == Some(i) by {
            match selected {
                Some(actual) => {
                    assert(unique_binding_reference_index(rows, id, actual as int));
                    if actual != i {
                        assert(binding_reference_matches(rows, i as int, id));
                        assert(!binding_reference_matches(rows, i as int, id));
                    }
                },
                None => { assert(!unique_binding_reference_index(rows, id, i as int)); },
            }
        }
    }
    Ok(selected)
}

struct CapturedMatches<'a, 'p, M: RegistryMembership> {
    paths: &'a [&'p str], values: &'a M,
}

impl<M: RegistryMembership> UniqueMatchRows for CapturedMatches<'_, '_, M> {
    spec fn count(&self) -> nat { self.paths@.len() }
    spec fn matches(&self, index: int) -> bool { self.values.has_path(self.paths@[index]) }
    fn len(&self) -> usize { self.paths.len() }
    fn matched(&self, index: usize) -> bool { self.values.contains_path(self.paths[index]) }
}

pub open spec fn unique_registry_index<M: RegistryMembership>(
    paths: Seq<&str>, values: &M, index: int,
) -> bool {
    0 <= index < paths.len() && values.has_path(paths[index])
        && forall|j: int| 0 <= j < paths.len() && j != index ==> !values.has_path(#[trigger] paths[j])
}

/// 从原登记路径中借出快照唯一包含的一项；缺失或歧义保持原错误。
pub fn select_captured_target<'a, M: RegistryMembership>(paths: &[&'a str], values: &M)
    -> (out: Result<&'a str, &'static str>)
    ensures match out {
        Ok(path) => exists|i: int| #[trigger] unique_registry_index(paths@, values, i)
            && paths@[i]@ == path@,
        Err(message) => message@ == "registry values require exactly one registered effective policy"@
            && forall|i: int| !#[trigger] unique_registry_index(paths@, values, i),
    },
{
    let message = "registry values require exactly one registered effective policy";
    let observed = CapturedMatches { paths, values };
    proof {
        reveal(unique_match_index);
        assert(observed.count() == paths@.len());
        assert forall|i: int| 0 <= i < paths@.len() implies #[trigger] observed.matches(i)
            == values.has_path(paths@[i]) by {}
        assert forall|i: int| #[trigger] unique_match_index(&observed, i)
            == unique_registry_index(paths@, values, i) by {
            if unique_match_index(&observed, i) {
                assert forall|j: int| 0 <= j < paths@.len() && j != i implies
                    !values.has_path(#[trigger] paths@[j]) by {
                    assert(!observed.matches(j));
                }
            }
            if unique_registry_index(paths@, values, i) {
                assert forall|j: int| 0 <= j < observed.count() && j != i implies
                    !#[trigger] observed.matches(j) by {
                    assert(!values.has_path(paths@[j]));
                }
            }
        }
    }
    let selected = unique_matching_index(&observed);
    match selected {
        Some(index) => {
            assert(unique_registry_index(paths@, values, index as int));
            Ok(paths[index])
        },
        None => {
            assert forall|j: int| !#[trigger] unique_registry_index(paths@, values, j) by {
                assert(!unique_match_index(&observed, j));
            }
            Err(message)
        },
    }
}

pub fn admit_identity_object(fields: Option<usize>, program: bool, configurations: bool)
    -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => fields == Some(2usize) && program && configurations,
        Err(error) => if fields.is_none() {
            error@ == "Git selector identity object required"@
        } else {
            (fields != Some(2usize) || !program || !configurations)
                && error@ == "Git selector identity requires only program and configurations"@
        },
    },
{
    match fields {
        None => Err("Git selector identity object required"),
        Some(2) if program && configurations => Ok(()),
        Some(_) => Err("Git selector identity requires only program and configurations"),
    }
}

pub open spec fn literal_program(text: &str, absolute: bool) -> bool {
    absolute && forall|i: int| 0 <= i < text.spec_bytes().len() ==>
        (#[trigger] text.spec_bytes()[i] != 0 && text.spec_bytes()[i] != 10
            && text.spec_bytes()[i] != 13)
}

pub fn admit_identity_program(program: Option<&str>, absolute: bool)
    -> (out: Result<&str, &'static str>)
    ensures match out {
        Ok(text) => program == Some(text) && text@.len() > 0 && literal_program(text, absolute),
        Err(error) => if (match program { Some(text) => text@.len() > 0, None => false }) {
            !literal_program(program.unwrap(), absolute)
                && error@ == "Git selector program must be an absolute literal path"@
        } else { error@ == "Git selector program required"@ },
    },
{
    let Some(text) = program else { return Err("Git selector program required"); };
    if text.is_empty() { return Err("Git selector program required"); }
    if !absolute { return Err("Git selector program must be an absolute literal path"); }
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len()
        invariant i <= bytes.len(), absolute, program == Some(text), text@.len() > 0,
            bytes@ == text.spec_bytes(),
            forall|j: int| 0 <= j < i ==> (#[trigger] bytes[j] != 0
                && bytes[j] != 10 && bytes[j] != 13),
        decreases bytes.len() - i,
    {
        if bytes[i] == 0 || bytes[i] == 10 || bytes[i] == 13 {
            assert(!literal_program(text, absolute));
            return Err("Git selector program must be an absolute literal path");
        }
        i += 1;
    }
    Ok(text)
}

pub fn admit_identity_configurations<T>(rows: Option<(&T, usize)>)
    -> (out: Result<&T, &'static str>)
    ensures match out {
        Ok(actual) => match rows { Some((original, count)) => count > 0 && actual == original, None => false },
        Err(error) => (match rows { Some((_, count)) => count == 0, None => true })
            && error@ == "Git selector requires nonempty configurations"@,
    },
{
    match rows {
        Some((rows, count)) if count > 0 => Ok(rows),
        _ => Err("Git selector requires nonempty configurations"),
    }
}

pub fn identity_registry_path(path: Option<&str>) -> (out: Result<&str, &'static str>)
    ensures match out {
        Ok(actual) => path == Some(actual),
        Err(error) => path.is_none() && error@ == "Git selector registry_path required"@,
    },
{
    path.ok_or("Git selector registry_path required")
}

pub fn admit_identity_registry_target(path: &str, targets: &[&str])
    -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => exists|i: int| 0 <= i < targets.len() && targets@[i]@ == path@,
        Err(error) => (forall|i: int| 0 <= i < targets.len() ==> targets@[i]@ != path@)
            && error@ == "Git selector registry_path must name a registered policy target"@,
    },
{
    let mut i = 0;
    while i < targets.len()
        invariant i <= targets.len(),
            forall|j: int| 0 <= j < i ==> #[trigger] targets@[j]@ != path@,
        decreases targets.len() - i,
    {
        if identifiers::equal_text(path, targets[i]) {
            assert(exists|j: int| 0 <= j < targets.len() && targets@[j]@ == path@);
            return Ok(());
        }
        i += 1;
    }
    Err("Git selector registry_path must name a registered policy target")
}

pub open spec fn binding_version(schema: u64, present: bool, null: bool, text: Option<&str>) -> bool {
    present && match text {
        Some(value) => value@.len() > 0,
        None => schema == 4 && null,
    }
}

pub fn valid_version(schema: u64, present: bool, null: bool, text: Option<&str>) -> (valid: bool)
    ensures valid == binding_version(schema, present, null, text),
{
    present && match text {
        Some(value) => !value.is_empty(),
        None => schema == 4 && null,
    }
}

pub open spec fn binding_tool_error(
    schema: u64, program: Option<&str>, version: (bool, bool, Option<&str>),
    resolution: Option<&str>, input: Option<(Option<&str>, Option<&str>, Option<&str>)>,
) -> Option<Seq<char>> {
    if !binding_text_present(program) { Some("missing program"@) }
    else if !binding_version(schema, version.0, version.1, version.2) {
        Some("facts_git requires an expected version, or null in v4"@)
    } else if !required_text_matches(resolution, "PATH-once") {
        Some("facts_git requires a PATH-once tool"@)
    } else {
        match input {
            None => None,
            Some((location, presence, digest)) => {
                if !binding_text_present(location) { Some("missing location"@) }
                else if !required_text_matches(presence, "present") || !(match digest {
                    Some(text) => identifiers::digest_format(text.spec_bytes()),
                    None => false,
                }) {
                    Some("facts_git requires a present digest-bound input when declared"@)
                } else { None }
            },
        }
    }
}

/// 工具、版本、解析方式及可选输入按原顺序准入，返回第一个原诊断。
pub fn admit_binding_tool(
    schema: u64, program: Option<&str>, version: (bool, bool, Option<&str>),
    resolution: Option<&str>, input: Option<(Option<&str>, Option<&str>, Option<&str>)>,
) -> (out: Result<(), &'static str>)
    ensures match binding_tool_error(schema, program, version, resolution, input) {
        None => out.is_ok(),
        Some(expected) => match out { Err(message) => message@ == expected, Ok(()) => false },
    },
{
    if !has_binding_text(program) { return Err("missing program"); }
    if !valid_version(schema, version.0, version.1, version.2) {
        return Err("facts_git requires an expected version, or null in v4");
    }
    if !matches_required(resolution, "PATH-once") {
        return Err("facts_git requires a PATH-once tool");
    }
    if let Some((location, presence, digest)) = input {
        if !has_binding_text(location) { return Err("missing location"); }
        if !matches_required(presence, "present") || !(match digest {
            Some(text) => identifiers::valid_digest_text(text),
            None => false,
        }) {
            return Err("facts_git requires a present digest-bound input when declared");
        }
    }
    Ok(())
}

pub fn admit_binding_version_argv(count: usize) -> (out: Result<(), &'static str>)
    ensures out.is_ok() == (count > 0),
        match out {
            Err(message) => message@ == "facts_git requires version_argv"@,
            Ok(()) => true,
        },
{
    if count == 0 { Err("facts_git requires version_argv") } else { Ok(()) }
}

/// 无预期时保留真实观察；给出预期时必须完整相等。
pub fn matches_expected(actual: &str, expected: Option<&str>) -> (valid: bool)
    ensures valid <==> match expected {
        None => true,
        Some(value) => actual@ == value@,
    },
{
    match expected {
        None => true,
        Some(value) => identifiers::equal_text(actual, value),
    }
}

pub enum GitTargetField { Digest, Path }

/// 只观察原数组长度与指定槽位的完整字符串，不在接口内选择目标。
pub trait GitTargetRows {
    spec fn count(&self) -> nat;
    spec fn text_at(&self, index: int, field: GitTargetField) -> Option<&str>;

    fn len(&self) -> (out: usize)
        ensures out == self.count();

    fn text(&self, index: usize, field: &GitTargetField) -> (out: Option<&str>)
        requires index < self.count(),
        ensures out == self.text_at(index as int, *field);
}

pub open spec fn target_matches<R: GitTargetRows>(
    rows: &R, index: int, field: GitTargetField, expected: Option<&str>,
) -> bool {
    match (rows.text_at(index, field), expected) {
        (Some(actual), Some(expected)) => actual@ == expected@,
        _ => false,
    }
}

pub open spec fn first_target<R: GitTargetRows>(
    rows: &R, field: GitTargetField, expected: Option<&str>, out: Option<usize>,
) -> bool {
    match out {
        Some(i) => i < rows.count() && target_matches(rows, i as int, field, expected)
            && forall|j: int| 0 <= j < i ==> !target_matches(rows, j, field, expected),
        None => forall|j: int| 0 <= j < rows.count() ==> !target_matches(rows, j, field, expected),
    }
}

fn find_target<R: GitTargetRows>(rows: &R, field: GitTargetField, expected: Option<&str>)
    -> (out: Option<usize>)
    ensures first_target(rows, field, expected, out),
{
    let Some(text) = expected else { return None; };
    let len = rows.len();
    let mut i = 0;
    while i < len
        invariant
            i <= len, len == rows.count(), expected == Some(text),
            forall|j: int| 0 <= j < i ==> !target_matches(rows, j, field, expected),
        decreases len - i,
    {
        if let Some(actual) = rows.text(i, &field) {
            if identifiers::equal_text(actual, text) { return Some(i); }
        }
        assert(!target_matches(rows, i as int, field, expected));
        i += 1;
    }
    None
}

pub open spec fn preferred_target<R: GitTargetRows>(
    rows: &R, digest: Option<&str>, path: Option<&str>, out: Option<usize>,
) -> bool {
    (out.is_some() && first_target(rows, GitTargetField::Digest, digest, out))
    || (first_target(rows, GitTargetField::Digest, digest, None)
        && first_target(rows, GitTargetField::Path, path, out))
}

/// 优先选摘要首个匹配；只有摘要无匹配才按已给出的登记路径选首项。
pub fn select_git_target<R: GitTargetRows>(rows: &R, digest: Option<&str>, path: Option<&str>)
    -> (out: Option<usize>)
    ensures preferred_target(rows, digest, path, out),
{
    match find_target(rows, GitTargetField::Digest, digest) {
        Some(index) => Some(index),
        None => find_target(rows, GitTargetField::Path, path),
    }
}

}
