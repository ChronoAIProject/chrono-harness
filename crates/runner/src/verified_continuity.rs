//! 固定两端的源码连续性。连续性保留历史分类，不产生证明资格。
use super::{append_bytes, equal_bytes};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// 来自同一固定 projects 库存的原 ID 和根；不从目录发现项目。
#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
pub struct ProjectRoot {
    pub id: String,
    // projects 允许无源码根的登记；空值只记录缺失，不能参与搬迁匹配。
    #[cfg_attr(not(verus_keep_ghost), serde(default))]
    pub root: String,
}

/// JSON/树适配器只投影原字段；metadata 是 owner、surface、cost、edges、
/// symlink、projection 的原值编码，不含 path 和独立核对的 verification。
pub struct FileObservation {
    pub path: String,
    pub metadata: Vec<u8>,
    pub mode: String,
    pub kind: String,
    pub registered: bool,
    /// 原登记已有分类时只追踪来源；源码连续性仅供 unknown 使用。
    pub classified: bool,
}

/// 宿主显式声明的单文件引用语法；只改变两端的确切包围字节，目标仍须对应。
/// base: 0 为源码目录，1 为项目根，2 为仓库根。
#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct ReferenceSyntax {
    pub file: String,
    pub before: String,
    pub after: String,
    pub trailing: String,
    pub suffix_before: String,
    pub suffix_after: String,
    pub base: u8,
}

pub struct Endpoint {
    pub projects: Vec<ProjectRoot>,
    pub files: Vec<FileObservation>,
    pub outputs: Vec<String>,
    pub reference_syntax: Vec<ReferenceSyntax>,
}

pub trait Read {
    spec fn contents(&self, base: bool, path: Seq<char>) -> Result<Seq<u8>, Seq<char>>;
    fn blob(&self, base: bool, path: &str) -> (out: Result<Vec<u8>, String>)
        ensures match out { Ok(value) => self.contents(base, path@) == Ok::<Seq<u8>, Seq<char>>(value@),
            Err(error) => self.contents(base, path@) == Err::<Seq<u8>, Seq<char>>(error@) };
}

pub open spec fn under(path: Seq<u8>, root: Seq<u8>) -> bool {
    root.len() < path.len() && path.take(root.len() as int) == root
        && path[root.len() as int] == 47
}

pub fn below(path: &[u8], root: &[u8]) -> (yes: bool)
    ensures yes == under(path@, root@),
{
    root.len() < path.len() && super::matches_at(path, 0, root) && path[root.len()] == 47
}

pub open spec fn project_match(
    base: Seq<ProjectRoot>, current: Seq<ProjectRoot>, i: int, j: int, path: Seq<u8>,
) -> bool {
    0 <= i < base.len() && 0 <= j < current.len()
        && base[i].root@.len() > 0 && current[j].root@.len() > 0
        && base[i].id@ == current[j].id@
        && base[i].root@ != current[j].root@
        && (path == vstd::utf8::encode_utf8(base[i].root@)
            || under(path, vstd::utf8::encode_utf8(base[i].root@)))
}

pub open spec fn moved_target(
    base: Seq<ProjectRoot>, current: Seq<ProjectRoot>, path: Seq<u8>, i: int, j: int,
) -> Seq<u8> {
    vstd::utf8::encode_utf8(current[j].root@)
        + path.skip(vstd::utf8::encode_utf8(base[i].root@).len() as int)
}

/// 歧义不选第一项；相同项目 ID 的两个实际根才可建立路径对应。
pub fn relocate(path: &[u8], base: &Vec<ProjectRoot>, current: &Vec<ProjectRoot>)
    -> (out: Option<Vec<u8>>)
    ensures (match out { None => None, Some(value) => Some(value@) }) == relocation(base@, current@, path@),
        match out {
        None => exists|i: int, j: int, k: int, l: int|
            project_match(base@, current@, i, j, path@)
            && project_match(base@, current@, k, l, path@) && (i != k || j != l),
        Some(value) => {
            (forall|i: int, j: int, k: int, l: int|
                project_match(base@, current@, i, j, path@)
                && project_match(base@, current@, k, l, path@) ==> i == k && j == l)
            && (forall|i: int, j: int| project_match(base@, current@, i, j, path@)
                ==> value@ == moved_target(base@, current@, path@, i, j))
            && ((!exists|i: int, j: int| project_match(base@, current@, i, j, path@))
                ==> value@ == path@)
        },
    },
{
    let mut found = false;
    let mut out = super::copy_range(path, 0, path.len());
    let mut i = 0;
    while i < base.len()
        invariant 0 <= i <= base.len(),
            found == (exists|a: int, b: int| 0 <= a < i && 0 <= b < current.len()
                && project_match(base@, current@, a, b, path@)),
            !found ==> out@ == path@,
            forall|a: int, b: int| 0 <= a < i && 0 <= b < current.len()
                && project_match(base@, current@, a, b, path@)
                ==> out@ == moved_target(base@, current@, path@, a, b),
            forall|a: int, b: int, c: int, d: int|
                0 <= a < i && 0 <= c < i
                && project_match(base@, current@, a, b, path@)
                && project_match(base@, current@, c, d, path@) ==> a == c && b == d,
        decreases base.len() - i,
    {
        let mut j = 0;
        while j < current.len()
            invariant 0 <= i < base.len(), 0 <= j <= current.len(),
                found == (exists|a: int, b: int| 0 <= a <= i && 0 <= b < current.len()
                    && (a < i || b < j) && project_match(base@, current@, a, b, path@)),
                !found ==> out@ == path@,
                forall|a: int, b: int| 0 <= a <= i && 0 <= b < current.len()
                    && (a < i || b < j) && project_match(base@, current@, a, b, path@)
                    ==> out@ == moved_target(base@, current@, path@, a, b),
                forall|a: int, b: int, c: int, d: int|
                    0 <= a <= i && 0 <= c <= i && (a < i || b < j) && (c < i || d < j)
                    && project_match(base@, current@, a, b, path@)
                    && project_match(base@, current@, c, d, path@) ==> a == c && b == d,
            decreases current.len() - j,
        {
            if super::super::equal_text(base[i].id.as_str(), current[j].id.as_str())
                && !base[i].root.as_str().is_empty() && !current[j].root.as_str().is_empty()
                && !super::super::equal_text(base[i].root.as_str(), current[j].root.as_str())
                && (equal_bytes(path, base[i].root.as_str().as_bytes()) || below(path, base[i].root.as_str().as_bytes())) {
                proof {
                    assert(project_match(base@, current@, i as int, j as int, path@));
                }
                if found {
                    proof {
                        let (a, b) = choose|a: int, b: int|
                            0 <= a <= i && 0 <= b < current.len()
                            && (a < i || b < j)
                            && project_match(base@, current@, a, b, path@);
                        assert(a != i || b != j);
                        assert(project_match(base@, current@, a, b, path@));
                        assert(relocation(base@, current@, path@).is_none());
                    }
                    return None;
                }
                found = true;
                out = Vec::new();
                append_bytes(&mut out, current[j].root.as_str().as_bytes());
                append_bytes(&mut out, super::copy_range(path, base[i].root.as_str().len(), path.len()).as_slice());
                assert(exists|a: int, b: int| 0 <= a <= i && 0 <= b < current.len()
                    && (a < i || b < j + 1) && project_match(base@, current@, a, b, path@));
            }
            j += 1;
        }
        i += 1;
    }
    proof {
        if found {
            let (a, b) = choose|a: int, b: int| 0 <= a < base.len() && 0 <= b < current.len()
                && project_match(base@, current@, a, b, path@);
            assert(out@ == moved_target(base@, current@, path@, a, b));
            assert(exists|pair: (int, int)| project_match(base@, current@, pair.0, pair.1, path@)) by {
                assert(project_match(base@, current@, (a, b).0, (a, b).1, path@));
            }
        } else {
            assert(!exists|a: int, b: int| project_match(base@, current@, a, b, path@));
        }
    }
    Some(out)
}

pub open spec fn unique_file(files: Seq<FileObservation>, path: Seq<u8>, index: int) -> bool {
    0 <= index < files.len() && vstd::utf8::encode_utf8(files[index].path@) == path
        && forall|j: int| 0 <= j < files.len() && j != index
            ==> vstd::utf8::encode_utf8((#[trigger] files[j]).path@) != path
}

pub fn find_file(files: &Vec<FileObservation>, path: &[u8]) -> (out: Option<usize>)
    ensures (match out { None => None, Some(i) => Some(i as int) }) == file_index(files@, path@),
        match out {
        None => !exists|i: int| unique_file(files@, path@, i),
        Some(i) => unique_file(files@, path@, i as int),
    },
{
    let mut index: Option<usize> = None;
    let mut i = 0;
    while i < files.len()
        invariant 0 <= i <= files.len(),
            match index {
                None => forall|j: int| 0 <= j < i ==> vstd::utf8::encode_utf8((#[trigger] files[j]).path@) != path@,
                Some(k) => 0 <= k < i && vstd::utf8::encode_utf8(files[k as int].path@) == path@
                    && forall|j: int| 0 <= j < i && j != k
                        ==> vstd::utf8::encode_utf8((#[trigger] files[j]).path@) != path@,
            },
        decreases files.len() - i,
    {
        if equal_bytes(files[i].path.as_str().as_bytes(), path) {
            if index.is_some() { return None; }
            index = Some(i);
        }
        i += 1;
    }
    proof {
        match index {
            Some(k) => {
                assert(unique_file(files@, path@, k as int));
                assert forall|j: int| unique_file(files@, path@, j) implies j == k by {
                    if j != k { assert(vstd::utf8::encode_utf8(files[k as int].path@) != path@); }
                }
            },
            None => {
                assert forall|j: int| !unique_file(files@, path@, j) by { }
            },
        }
    }
    index
}

pub open spec fn regular(file: &FileObservation) -> bool {
    (file.mode@ == "100644"@ || file.mode@ == "100755"@) && file.kind@ == "blob"@
}

pub fn is_regular(file: &FileObservation) -> (yes: bool)
    ensures yes == regular(file),
{
    (super::super::equal_text(file.mode.as_str(), "100644")
        || super::super::equal_text(file.mode.as_str(), "100755"))
        && super::super::equal_text(file.kind.as_str(), "blob")
}

/// dirname 的字节合同；空目录表示仓库根，不访问宿主路径解析器。
pub open spec fn directory(path: Seq<u8>) -> Seq<u8>
    decreases path.len(),
{
    if path.len() == 0 { Seq::empty() }
    else if path.last() == 47 { path.drop_last() }
    else { directory(path.drop_last()) }
}

pub fn dirname(path: &[u8]) -> (out: Vec<u8>)
    ensures out@ == directory(path@),
{
    let mut i = path.len();
    assert(path@.take(path.len() as int) =~= path@);
    while i > 0
        invariant 0 <= i <= path.len(), directory(path@) == directory(path@.take(i as int)),
        decreases i,
    {
        if path[i - 1] == 47 {
            assert(path@.take(i as int).last() == 47);
            assert(path@.take(i as int).drop_last() =~= path@.take(i - 1));
            return super::copy_range(path, 0, i - 1);
        }
        assert(path@.take(i as int).drop_last() =~= path@.take(i - 1));
        i -= 1;
    }
    Vec::new()
}

pub open spec fn join_reference(base: Seq<u8>, path: Seq<u8>) -> Seq<u8> {
    base + if base.len() == 0 { Seq::empty() } else { seq![47u8] } + path
}

/// 规范化允许结果为仓库根；绝对路径、逃逸和反斜杠仍拒绝。
pub open spec fn resolved(base: Seq<u8>, path: Seq<u8>) -> Option<Seq<u8>> {
    if path.len() == 0 || path[0] == 47 { None }
    else {
        match super::path_scan(join_reference(base, path), Seq::empty(), Seq::empty()) {
            None => None,
            Some(parts) => Some(super::joined_components(parts)),
        }
    }
}

pub fn resolve(base: &[u8], path: &[u8]) -> (out: Option<Vec<u8>>)
    ensures match out {
        None => resolved(base@, path@).is_none(),
        Some(value) => resolved(base@, path@) == Some(value@),
    },
{
    if path.len() == 0 || path[0] == 47 { return None; }
    let mut joined = Vec::new();
    append_bytes(&mut joined, base);
    if base.len() > 0 { joined.push(47); }
    append_bytes(&mut joined, path);
    let mut parts: Vec<Vec<u8>> = Vec::new();
    let mut word: Vec<u8> = Vec::new();
    let mut i = 0;
    assert(super::dependency_values(parts@) =~= Seq::<Seq<u8>>::empty());
    assert(joined@.skip(0) =~= joined@);
    while i < joined.len()
        invariant 0 <= i <= joined.len(), joined@ == join_reference(base@, path@),
            super::path_scan(joined@, Seq::empty(), Seq::empty())
                == super::path_scan(joined@.skip(i as int), super::dependency_values(parts@), word@),
            forall|j: int| 0 <= j < parts.len() ==> super::normal_component((#[trigger] parts[j])@),
            forall|j: int| 0 <= j < word.len() ==> super::dependency_byte(#[trigger] word[j]) && word[j] != 47,
        decreases joined.len() - i,
    {
        let byte = joined[i];
        if byte < 32 || byte == 92 || byte == 127 { return None; }
        if byte == 47 {
            if !super::finish_component(&mut parts, word) { return None; }
            word = Vec::new();
        } else { word.push(byte); }
        assert(joined@.skip(i as int).drop_first() =~= joined@.skip(i + 1));
        i += 1;
    }
    assert(joined@.skip(i as int) =~= Seq::<u8>::empty());
    if !super::finish_component(&mut parts, word) { return None; }
    let mut out = Vec::new();
    let mut j = 0;
    assert(super::dependency_values(parts@.take(0)) =~= Seq::<Seq<u8>>::empty());
    while j < parts.len()
        invariant 0 <= j <= parts.len(),
            out@ == super::joined_components(super::dependency_values(parts@.take(j as int))),
        decreases parts.len() - j,
    {
        if j > 0 { out.push(47); }
        append_bytes(&mut out, parts[j].as_slice());
        proof {
            let p = super::dependency_values(parts@.take(j + 1));
            assert(p.drop_last() =~= super::dependency_values(parts@.take(j as int)));
            assert(p.last() == parts@[j as int]@);
        }
        j += 1;
    }
    assert(parts@.take(j as int) =~= parts@);
    Some(out)
}

/// 五种语言引用由通用实现负责；其余语法只能来自当前固定宿主的显式声明。
pub open spec fn builtin_prefix(kind: u16) -> Seq<u8> {
    if kind == 1 { seq![35u8, 91u8, 112u8, 97u8, 116u8, 104u8, 32u8, 61u8, 32u8, 34u8] }
    else if kind == 2 { seq![105u8, 110u8, 99u8, 108u8, 117u8, 100u8, 101u8, 95u8, 98u8, 121u8, 116u8, 101u8, 115u8, 33u8, 40u8, 34u8] }
    else if kind == 3 { seq![112u8, 97u8, 116u8, 104u8, 32u8, 61u8, 32u8, 34u8] }
    else if kind == 4 { seq![80u8, 97u8, 116u8, 104u8, 58u8, 58u8, 110u8, 101u8, 119u8, 40u8, 101u8, 110u8, 118u8, 33u8, 40u8, 34u8, 67u8, 65u8, 82u8, 71u8, 79u8, 95u8, 77u8, 65u8, 78u8, 73u8, 70u8, 69u8, 83u8, 84u8, 95u8, 68u8, 73u8, 82u8, 34u8, 41u8, 41u8, 46u8, 106u8, 111u8, 105u8, 110u8, 40u8, 34u8] }
    else { seq![80u8, 97u8, 116u8, 104u8, 58u8, 58u8, 110u8, 101u8, 119u8, 40u8, 101u8, 110u8, 118u8, 33u8, 40u8, 34u8, 67u8, 65u8, 82u8, 71u8, 79u8, 95u8, 77u8, 65u8, 78u8, 73u8, 70u8, 69u8, 83u8, 84u8, 95u8, 68u8, 73u8, 82u8, 34u8, 41u8, 41u8, 10u8, 32u8, 32u8, 32u8, 32u8, 32u8, 32u8, 32u8, 32u8, 46u8, 106u8, 111u8, 105u8, 110u8, 40u8, 34u8] }
}

pub fn builtin_prefix_bytes(kind: u16) -> (out: &'static [u8])
    requires 1 <= kind <= 5,
    ensures out@ == builtin_prefix(kind),
{
    if kind == 1 { &[35u8, 91u8, 112u8, 97u8, 116u8, 104u8, 32u8, 61u8, 32u8, 34u8] }
    else if kind == 2 { &[105u8, 110u8, 99u8, 108u8, 117u8, 100u8, 101u8, 95u8, 98u8, 121u8, 116u8, 101u8, 115u8, 33u8, 40u8, 34u8] }
    else if kind == 3 { &[112u8, 97u8, 116u8, 104u8, 32u8, 61u8, 32u8, 34u8] }
    else if kind == 4 { &[80u8, 97u8, 116u8, 104u8, 58u8, 58u8, 110u8, 101u8, 119u8, 40u8, 101u8, 110u8, 118u8, 33u8, 40u8, 34u8, 67u8, 65u8, 82u8, 71u8, 79u8, 95u8, 77u8, 65u8, 78u8, 73u8, 70u8, 69u8, 83u8, 84u8, 95u8, 68u8, 73u8, 82u8, 34u8, 41u8, 41u8, 46u8, 106u8, 111u8, 105u8, 110u8, 40u8, 34u8] }
    else { &[80u8, 97u8, 116u8, 104u8, 58u8, 58u8, 110u8, 101u8, 119u8, 40u8, 101u8, 110u8, 118u8, 33u8, 40u8, 34u8, 67u8, 65u8, 82u8, 71u8, 79u8, 95u8, 77u8, 65u8, 78u8, 73u8, 70u8, 69u8, 83u8, 84u8, 95u8, 68u8, 73u8, 82u8, 34u8, 41u8, 41u8, 10u8, 32u8, 32u8, 32u8, 32u8, 32u8, 32u8, 32u8, 32u8, 46u8, 106u8, 111u8, 105u8, 110u8, 40u8, 34u8] }
}

pub open spec fn known_kind(current: &Endpoint, kind: u16) -> bool {
    1 <= kind <= 5 || (6 <= kind <= 255 && kind - 6 < current.reference_syntax.len())
}

#[verifier::opaque]
pub open spec fn prefix(kind: u16, current: &Endpoint, right: bool) -> Seq<u8> {
    if 1 <= kind <= 5 { builtin_prefix(kind) }
    else if known_kind(current, kind) {
        let form = &current.reference_syntax[kind - 6];
        vstd::utf8::encode_utf8(if right { form.after@ } else { form.before@ })
    } else { Seq::empty() }
}

pub fn prefix_bytes<'a>(kind: u16, current: &'a Endpoint, right: bool) -> (out: &'a [u8])
    requires known_kind(current, kind),
    ensures out@ == prefix(kind, current, right),
{
    reveal(prefix);
    if kind <= 5 { builtin_prefix_bytes(kind) }
    else {
        let form = &current.reference_syntax[(kind - 6) as usize];
        if right { form.after.as_str().as_bytes() } else { form.before.as_str().as_bytes() }
    }
}

#[verifier::opaque]
pub open spec fn suffix(kind: u16, current: &Endpoint, right: bool) -> Seq<u8> {
    if 6 <= kind && known_kind(current, kind) {
        let form = &current.reference_syntax[kind - 6];
        vstd::utf8::encode_utf8(if right { form.suffix_after@ } else { form.suffix_before@ })
    } else { Seq::empty() }
}

pub fn suffix_bytes<'a>(kind: u16, current: &'a Endpoint, right: bool) -> (out: &'a [u8])
    requires known_kind(current, kind),
    ensures out@ == suffix(kind, current, right),
{
    reveal(suffix);
    if kind <= 5 { &[] }
    else {
        let form = &current.reference_syntax[(kind - 6) as usize];
        if right { form.suffix_after.as_str().as_bytes() } else { form.suffix_before.as_str().as_bytes() }
    }
}

#[verifier::opaque]
pub open spec fn trailing(kind: u16, current: &Endpoint) -> Seq<u8> {
    if 6 <= kind && known_kind(current, kind) {
        vstd::utf8::encode_utf8(current.reference_syntax[kind - 6].trailing@)
    } else { Seq::empty() }
}

pub fn trailing_bytes<'a>(kind: u16, current: &'a Endpoint) -> (out: &'a [u8])
    requires known_kind(current, kind),
    ensures out@ == trailing(kind, current),
{
    reveal(trailing);
    if kind <= 5 { &[] }
    else { current.reference_syntax[(kind - 6) as usize].trailing.as_str().as_bytes() }
}

pub open spec fn eligible(kind: u16, current: &Endpoint, file: Seq<u8>, manifest: bool) -> bool {
    known_kind(current, kind) && (kind != 3 || manifest)
        && (kind <= 5 || vstd::utf8::encode_utf8(current.reference_syntax[kind - 6].file@) == file)
}

pub open spec fn anchor(input: Seq<u8>, at: int, kind: u16, manifest: bool,
    current: &Endpoint, file: Seq<u8>, right: bool) -> bool {
    eligible(kind, current, file, manifest) && prefix(kind, current, right).len() > 0
        && super::prefix_at(input, at, prefix(kind, current, right))
}

pub fn anchor_at(input: &[u8], at: usize, manifest: bool, current: &Endpoint, file: &[u8]) -> (kind: u16)
    ensures kind == 0 <==> !exists|k: u16| anchor(input@, at as int, k, manifest, current, file@, false),
        kind != 0 ==> anchor(input@, at as int, kind, manifest, current, file@, false),
        kind != 0 ==> forall|j: u16| 1 <= j < kind ==> !anchor(input@, at as int, j, manifest, current, file@, false),
{
    let mut k: u16 = 1;
    while k <= 5 || (k <= 255 && ((k - 6) as usize) < current.reference_syntax.len())
        invariant 1 <= k <= 256,
            forall|j: u16| 1 <= j < k ==> !anchor(input@, at as int, j, manifest, current, file@, false),
        decreases 256 - k,
    {
        let selected = if k <= 5 { k != 3 || manifest }
            else { equal_bytes(current.reference_syntax[(k - 6) as usize].file.as_str().as_bytes(), file) };
        let p = prefix_bytes(k, current, false);
        if selected && p.len() > 0 && super::matches_at(input, at, p) {
            assert(anchor(input@, at as int, k, manifest, current, file@, false));
            assert(exists|j: u16| anchor(input@, at as int, j, manifest, current, file@, false));
            return k;
        }
        k += 1;
    }
    assert forall|j: u16| !anchor(input@, at as int, j, manifest, current, file@, false) by {
        if j >= k { assert(!known_kind(current, j)); }
    }
    0
}

pub open spec fn reference_end(input: Seq<u8>, at: int, kind: u16, current: &Endpoint, right: bool) -> int {
    super::next_byte(input, at + prefix(kind, current, right).len(), 34)
}

pub open spec fn closed_reference(input: Seq<u8>, at: int, kind: u16, current: &Endpoint, right: bool) -> bool {
    let x = reference_end(input, at, kind, current, right);
    at + prefix(kind, current, right).len() <= x < input.len()
        && trailing(kind, current).len() <= x - at - prefix(kind, current, right).len()
        && super::prefix_at(input, x - trailing(kind, current).len(), trailing(kind, current))
        && super::prefix_at(input, x + 1, suffix(kind, current, right))
}

pub fn closed_at(input: &[u8], at: usize, kind: u16, current: &Endpoint, right: bool)
    -> (out: Option<(usize, usize)>)
    requires at <= input.len(), known_kind(current, kind),
        prefix(kind, current, right).len() <= input.len() - at,
    ensures match out {
        None => !closed_reference(input@, at as int, kind, current, right),
        Some((literal, next)) => closed_reference(input@, at as int, kind, current, right)
            && literal == reference_end(input@, at as int, kind, current, right) - trailing(kind, current).len()
            && next == reference_end(input@, at as int, kind, current, right) + 1 + suffix(kind, current, right).len(),
    },
{
    let p = prefix_bytes(kind, current, right);
    let tail = trailing_bytes(kind, current);
    let rest = suffix_bytes(kind, current, right);
    let x = super::find_byte(input, at + p.len(), 34);
    if x == input.len() || tail.len() > x - at - p.len() { return None; }
    if !super::matches_at(input, x - tail.len(), tail) || !super::matches_at(input, x + 1, rest) { return None; }
    Some((x - tail.len(), x + 1 + rest.len()))
}

/// 路径引用用同一固定目标判断；根／输出目录的同一地址不要求伪造文件。
pub open spec fn target_exists(endpoint: &Endpoint, path: Seq<u8>) -> bool {
    path.len() == 0
        || (exists|i: int| unique_file(endpoint.files@, path, i) && regular(&endpoint.files[i]))
        || (exists|i: int| 0 <= i < endpoint.projects.len()
            && vstd::utf8::encode_utf8((#[trigger] endpoint.projects[i]).root@) == path)
        || (exists|i: int| 0 <= i < endpoint.outputs.len()
            && super::prefix_at(path, 0, vstd::utf8::encode_utf8((#[trigger] endpoint.outputs[i])@)))
}

pub fn exists_target(endpoint: &Endpoint, path: &[u8]) -> (yes: bool)
    ensures yes == target_exists(endpoint, path@),
{
    if path.len() == 0 { return true; }
    if let Some(i) = find_file(&endpoint.files, path) {
        if is_regular(&endpoint.files[i]) { return true; }
    }
    let mut i = 0;
    while i < endpoint.projects.len()
        invariant 0 <= i <= endpoint.projects.len(), path.len() > 0,
            !(exists|j: int| unique_file(endpoint.files@, path@, j) && regular(&endpoint.files[j])),
            forall|j: int| 0 <= j < i ==> vstd::utf8::encode_utf8((#[trigger] endpoint.projects[j]).root@) != path@,
        decreases endpoint.projects.len() - i,
    {
        if equal_bytes(endpoint.projects[i].root.as_str().as_bytes(), path) {
            assert(vstd::utf8::encode_utf8(endpoint.projects[i as int].root@) == path@);
            assert(exists|j: int| 0 <= j < endpoint.projects.len()
                && vstd::utf8::encode_utf8(endpoint.projects[j].root@) == path@);
            return true;
        }
        i += 1;
    }
    i = 0;
    while i < endpoint.outputs.len()
        invariant 0 <= i <= endpoint.outputs.len(), path.len() > 0,
            !(exists|j: int| unique_file(endpoint.files@, path@, j) && regular(&endpoint.files[j])),
            forall|j: int| 0 <= j < endpoint.projects.len() ==> vstd::utf8::encode_utf8((#[trigger] endpoint.projects[j]).root@) != path@,
            forall|j: int| 0 <= j < i ==> !super::prefix_at(path@, 0, vstd::utf8::encode_utf8((#[trigger] endpoint.outputs[j])@)),
        decreases endpoint.outputs.len() - i,
    {
        if super::matches_at(path, 0, endpoint.outputs[i].as_str().as_bytes()) {
            assert(super::prefix_at(path@, 0, vstd::utf8::encode_utf8(endpoint.outputs[i as int]@)));
            assert(exists|j: int| 0 <= j < endpoint.outputs.len()
                && super::prefix_at(path@, 0, vstd::utf8::encode_utf8(endpoint.outputs[j]@)));
            return true;
        }
        i += 1;
    }
    false
}

/// 每端的真实 CARGO_MANIFEST_DIR；缺少／歧义项目根时不猜。
pub open spec fn unique_project(projects: Seq<ProjectRoot>, path: Seq<u8>, i: int) -> bool {
    0 <= i < projects.len() && under(path, vstd::utf8::encode_utf8(projects[i].root@))
        && forall|j: int| 0 <= j < projects.len() && j != i
            ==> !under(path, vstd::utf8::encode_utf8((#[trigger] projects[j]).root@))
}

pub fn project_directory(projects: &Vec<ProjectRoot>, path: &[u8]) -> (out: Option<Vec<u8>>)
    ensures match out {
        None => !exists|i: int| unique_project(projects@, path@, i),
        Some(value) => exists|i: int| unique_project(projects@, path@, i)
            && value@ == vstd::utf8::encode_utf8(projects[i].root@),
    },
{
    let mut index: Option<usize> = None;
    let mut i = 0;
    while i < projects.len()
        invariant 0 <= i <= projects.len(),
            match index {
                None => forall|j: int| 0 <= j < i ==> !under(path@, vstd::utf8::encode_utf8((#[trigger] projects[j]).root@)),
                Some(k) => 0 <= k < i && under(path@, vstd::utf8::encode_utf8(projects[k as int].root@))
                    && forall|j: int| 0 <= j < i && j != k ==> !under(path@, vstd::utf8::encode_utf8((#[trigger] projects[j]).root@)),
            },
        decreases projects.len() - i,
    {
        if below(path, projects[i].root.as_str().as_bytes()) {
            if index.is_some() { return None; }
            index = Some(i);
        }
        i += 1;
    }
    match index {
        None => None,
        Some(k) => {
            assert(unique_project(projects@, path@, k as int));
            assert(exists|j: int| unique_project(projects@, path@, j)
                && vstd::utf8::encode_utf8(projects[j].root@) == vstd::utf8::encode_utf8(projects[k as int].root@));
            let value = super::copy_range(projects[k].root.as_str().as_bytes(), 0, projects[k].root.as_str().len());
            assert(value@ =~= vstd::utf8::encode_utf8(projects[k as int].root@));
            assert(exists|j: int| unique_project(projects@, path@, j)
                && value@ == vstd::utf8::encode_utf8(projects[j].root@));
            Some(value)
        },
    }
}

pub open spec fn reference_base(endpoint: &Endpoint, file: Seq<u8>, kind: u16, current: &Endpoint) -> Option<Seq<u8>> {
    let mode = if kind <= 3 { 0int } else if kind <= 5 { 1int }
        else if known_kind(current, kind) { current.reference_syntax[kind - 6].base as int } else { 3int };
    if mode == 0 { Some(directory(file)) }
    else if mode == 1 {
        if exists|i: int| unique_project(endpoint.projects@, file, i) {
            let i = choose|i: int| unique_project(endpoint.projects@, file, i);
            Some(vstd::utf8::encode_utf8(endpoint.projects[i].root@))
        } else { None }
    } else if mode == 2 { Some(Seq::empty()) } else { None }
}

pub fn base_for(endpoint: &Endpoint, file: &[u8], kind: u16, current: &Endpoint) -> (out: Option<Vec<u8>>)
    requires known_kind(current, kind),
    ensures match out { None => reference_base(endpoint, file@, kind, current).is_none(),
        Some(value) => reference_base(endpoint, file@, kind, current) == Some(value@) },
{
    let mode = if kind <= 3 { 0u8 } else if kind <= 5 { 1u8 }
        else { current.reference_syntax[(kind - 6) as usize].base };
    if mode == 0 { Some(dirname(file)) }
    else if mode == 1 { project_directory(&endpoint.projects, file) }
    else if mode == 2 { Some(Vec::new()) } else { None }
}

#[verifier::opaque]
pub open spec fn reference_pair(
    left: Seq<u8>, right: Seq<u8>, old_file: Seq<u8>, new_file: Seq<u8>,
    base: &Endpoint, current: &Endpoint, kind: u16,
) -> bool {
    reference_base(base, old_file, kind, current).is_some() && reference_base(current, new_file, kind, current).is_some()
        && resolved(reference_base(base, old_file, kind, current).unwrap(), left).is_some()
        && resolved(reference_base(current, new_file, kind, current).unwrap(), right).is_some()
        && {
            let a = resolved(reference_base(base, old_file, kind, current).unwrap(), left).unwrap();
            let b = resolved(reference_base(current, new_file, kind, current).unwrap(), right).unwrap();
            target_exists(base, a) && target_exists(current, b)
                && (if exists|i: int, j: int| project_match(base.projects@, current.projects@, i, j, a) {
                    (exists|i: int, j: int| project_match(base.projects@, current.projects@, i, j, a))
                    && (forall|i: int, j: int| project_match(base.projects@, current.projects@, i, j, a)
                        ==> moved_target(base.projects@, current.projects@, a, i, j) == b)
                    && (forall|i: int, j: int, k: int, l: int|
                        project_match(base.projects@, current.projects@, i, j, a)
                        && project_match(base.projects@, current.projects@, k, l, a) ==> i == k && j == l)
                } else { a == b })
        }
}

pub fn same_reference(
    left: &[u8], right: &[u8], old_file: &[u8], new_file: &[u8],
    base: &Endpoint, current: &Endpoint, kind: u16,
) -> (yes: bool)
    requires known_kind(current, kind),
    ensures yes == reference_pair(left@, right@, old_file@, new_file@, base, current, kind),
{
    reveal(reference_pair);
    let a = match base_for(base, old_file, kind, current) { None => { return false; }, Some(v) => v };
    let b = match base_for(current, new_file, kind, current) { None => { return false; }, Some(v) => v };
    let a = match resolve(a.as_slice(), left) { None => { return false; }, Some(v) => v };
    let b = match resolve(b.as_slice(), right) { None => { return false; }, Some(v) => v };
    if !exists_target(base, a.as_slice()) || !exists_target(current, b.as_slice()) { return false; }
    match relocate(a.as_slice(), &base.projects, &current.projects) {
        None => false,
        Some(mapped) => equal_bytes(mapped.as_slice(), b.as_slice()),
    }
}

pub open spec fn source_equal(
    left: Seq<u8>, right: Seq<u8>, a: int, b: int, old_file: Seq<u8>, new_file: Seq<u8>,
    base: &Endpoint, current: &Endpoint, manifest: bool,
) -> bool
    decreases left.len() - a,
{
    if a < 0 || b < 0 || a > left.len() || b > right.len() { false }
    else if a == left.len() { b == right.len() }
    else if b == right.len() { false }
    else if exists|k: u16| anchor(left, a, k, manifest, current, new_file, false) {
        let k = choose|k: u16| anchor(left, a, k, manifest, current, new_file, false)
            && forall|j: u16| 1 <= j < k ==> !anchor(left, a, j, manifest, current, new_file, false);
        let x = reference_end(left, a, k, current, false);
        let y = reference_end(right, b, k, current, true);
        anchor(right, b, k, manifest, current, new_file, true)
            && closed_reference(left, a, k, current, false) && closed_reference(right, b, k, current, true)
            && reference_pair(left.subrange(a + prefix(k, current, false).len(), x - trailing(k, current).len()),
                right.subrange(b + prefix(k, current, true).len(), y - trailing(k, current).len()),
                old_file, new_file, base, current, k)
            && source_equal(left, right, x + 1 + suffix(k, current, false).len(),
                y + 1 + suffix(k, current, true).len(), old_file, new_file, base, current, manifest)
    } else {
        left[a] == right[b]
            && source_equal(left, right, a + 1, b + 1, old_file, new_file, base, current, manifest)
    }
}

/// 单步保留完整余段等式；主循环只组合已核对的步，不展开内部语法。
pub fn source_step(
    left: &[u8], right: &[u8], a: usize, b: usize, old_file: &[u8], new_file: &[u8],
    base: &Endpoint, current: &Endpoint, manifest: bool,
) -> (out: Option<(usize, usize)>)
    requires a < left.len(), b <= right.len(),
    ensures match out {
        None => !source_equal(left@, right@, a as int, b as int, old_file@, new_file@, base, current, manifest),
        Some((x,y)) => a < x <= left.len() && b < y <= right.len()
            && source_equal(left@, right@, a as int, b as int, old_file@, new_file@, base, current, manifest)
                == source_equal(left@, right@, x as int, y as int, old_file@, new_file@, base, current, manifest),
    },
{
    reveal_with_fuel(source_equal, 2);
    if b == right.len() { return None; }
    let kind = anchor_at(left, a, manifest, current, new_file);
    if kind != 0 {
        proof {
            let chosen = choose|k: u16| anchor(left@, a as int, k, manifest, current, new_file@, false)
                && forall|j: u16| 1 <= j < k ==> !anchor(left@, a as int, j, manifest, current, new_file@, false);
            assert(chosen == kind);
        }
        let p = prefix_bytes(kind, current, false);
        let q = prefix_bytes(kind, current, true);
        if q.len() == 0 || !super::matches_at(right, b, q) { return None; }
        assert(anchor(right@, b as int, kind, manifest, current, new_file@, true));
        let (x, next_a) = match closed_at(left, a, kind, current, false) { None => { return None; }, Some(v) => v };
        let (y, next_b) = match closed_at(right, b, kind, current, true) { None => { return None; }, Some(v) => v };
        let l = super::copy_range(left, a + p.len(), x);
        let r = super::copy_range(right, b + q.len(), y);
        if !same_reference(l.as_slice(), r.as_slice(), old_file, new_file, base, current, kind) { return None; }
        Some((next_a, next_b))
    } else {
        if left[a] != right[b] { return None; }
        Some((a + 1, b + 1))
    }
}

/// 精确比较所有非引用字节；每个引用先核声明、词法解析、原目标和当前目标。
pub fn same_source(
    left: &[u8], right: &[u8], old_file: &[u8], new_file: &[u8],
    base: &Endpoint, current: &Endpoint, manifest: bool,
) -> (yes: bool)
    ensures yes == source_equal(left@, right@, 0, 0, old_file@, new_file@, base, current, manifest),
{
    let mut a = 0;
    let mut b = 0;
    while a < left.len()
        invariant 0 <= a <= left.len(), 0 <= b <= right.len(),
            source_equal(left@, right@, 0, 0, old_file@, new_file@, base, current, manifest)
                == source_equal(left@, right@, a as int, b as int, old_file@, new_file@, base, current, manifest),
        decreases left.len() - a,
    {
        match source_step(left, right, a, b, old_file, new_file, base, current, manifest) {
            None => { return false; },
            Some((next_a, next_b)) => { a = next_a; b = next_b; },
        }
    }
    reveal_with_fuel(source_equal, 1);
    b == right.len()
}


pub fn select_endpoint<'a>(base: bool, previous: &'a str, current: &'a str) -> (out: &'a str)
    ensures out@ == if base { previous@ } else { current@ },
{ if base { previous } else { current } }

#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
pub struct Record {
    previous: String,
    current: String,
    left: Vec<u8>,
    right: Vec<u8>,
    manifest: bool,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
pub struct Origin { previous: String, current: String }
#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
pub struct Assessment { origin: Option<Origin>, record: Option<Record> }
#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
pub struct Continuity { observations: Vec<Assessment> }

#[cfg(verus_keep_ghost)]
pub type RecordView = (Seq<char>, Seq<char>, Seq<u8>, Seq<u8>, bool);
#[cfg(verus_keep_ghost)]
pub type AssessmentView = (Option<(Seq<char>, Seq<char>)>, Option<RecordView>);

impl Assessment {
    pub closed spec fn view(&self) -> AssessmentView {
        (match &self.origin { None => None, Some(origin) => Some((origin.previous@, origin.current@)) },
            match &self.record { None => None, Some(record) => Some((record.previous@, record.current@,
                record.left@, record.right@, record.manifest)) })
    }
}

pub open spec fn first_origin(observations: Seq<AssessmentView>, path: Seq<char>, i: int) -> Option<Seq<char>>
    decreases observations.len() - i,
{
    if i < 0 || i >= observations.len() { None }
    else {
        match observations[i].0 {
            Some(origin) => if origin.1 == path { Some(origin.0) }
                else { first_origin(observations, path, i + 1) },
            None => first_origin(observations, path, i + 1),
        }
    }
}

impl Continuity {
    pub closed spec fn view(&self) -> Seq<AssessmentView> {
        self.observations@.map(|i: int, value: Assessment| value.view())
    }

    pub fn empty() -> (out: Self)
        ensures out.view().len() == 0,
            forall|path: Seq<char>| !out.preserves(path) && out.origin_value(path).is_none(),
    { Self { observations: Vec::new() } }

    pub open spec fn preserves(&self, path: Seq<char>) -> bool {
        exists|i: int| 0 <= i < self.view().len()
            && match (#[trigger] self.view()[i]).1 {
                None => false, Some(record) => record.1 == path,
            }
    }

    pub fn preserved(&self, path: &str) -> (yes: bool)
        ensures yes == self.preserves(path@),
    {
        reveal(Continuity::view);
        reveal(Assessment::view);
        let mut i = 0;
        while i < self.observations.len()
            invariant 0 <= i <= self.observations.len(),
                forall|j: int| 0 <= j < i ==> match &(#[trigger] self.observations[j]).record {
                    None => true, Some(record) => record.current@ != path@,
                },
            decreases self.observations.len() - i,
        {
            if let Some(record) = &self.observations[i].record {
                if super::super::equal_text(record.current.as_str(), path) {
                    assert(self.view()[i as int].1.is_some());
                    assert(self.view()[i as int].1.unwrap().1 == path@);
                    assert(self.preserves(path@));
                    return true;
                }
            }
            i += 1;
        }
        false
    }

    pub open spec fn origin_value(&self, path: Seq<char>) -> Option<Seq<char>> {
        first_origin(self.view(), path, 0)
    }

    pub fn origin<'a>(&'a self, path: &str) -> (out: Option<&'a str>)
        ensures match out { None => self.origin_value(path@).is_none(),
            Some(value) => self.origin_value(path@) == Some(value@) },
    {
        let mut i = 0;
        while i < self.observations.len()
            invariant 0 <= i <= self.observations.len(),
                self.origin_value(path@) == first_origin(self.view(), path@, i as int),
            decreases self.observations.len() - i,
        {
            if let Some(origin) = &self.observations[i].origin {
                if super::super::equal_text(origin.current.as_str(), path) { return Some(origin.previous.as_str()); }
            }
            i += 1;
        }
        None
    }
}

pub open spec fn absent(files: Seq<FileObservation>, path: Seq<u8>) -> bool {
    forall|i: int| 0 <= i < files.len() ==> vstd::utf8::encode_utf8((#[trigger] files[i]).path@) != path
}

pub fn path_absent(files: &Vec<FileObservation>, path: &[u8]) -> (yes: bool)
    ensures yes == absent(files@, path@),
{
    let mut i = 0;
    while i < files.len()
        invariant 0 <= i <= files.len(),
            forall|j: int| 0 <= j < i ==> vstd::utf8::encode_utf8((#[trigger] files[j]).path@) != path@,
        decreases files.len() - i,
    {
        if equal_bytes(files[i].path.as_str().as_bytes(), path) { return false; }
        i += 1;
    }
    true
}

pub open spec fn relocation(base: Seq<ProjectRoot>, current: Seq<ProjectRoot>, path: Seq<u8>) -> Option<Seq<u8>> {
    if exists|i: int, j: int, k: int, l: int| project_match(base, current, i, j, path)
        && project_match(base, current, k, l, path) && (i != k || j != l) { None }
    else if exists|i: int, j: int| project_match(base, current, i, j, path) {
        let pair = choose|pair: (int, int)| project_match(base, current, pair.0, pair.1, path);
        Some(moved_target(base, current, path, pair.0, pair.1))
    } else { Some(path) }
}

pub open spec fn file_index(files: Seq<FileObservation>, path: Seq<u8>) -> Option<int> {
    if exists|i: int| unique_file(files, path, i) { Some(choose|i: int| unique_file(files, path, i)) }
    else { None }
}

pub open spec fn manifest_path(path: Seq<u8>) -> bool {
    resolved(directory(path), seq![67u8, 97u8, 114u8, 103u8, 111u8, 46u8, 116u8, 111u8, 109u8, 108u8]) == Some(path)
}

pub open spec fn comparable(old: &FileObservation, new: &FileObservation, base: &Endpoint, current: &Endpoint) -> bool {
    old.registered && new.registered && regular(old) && regular(new)
        && old.mode@ == new.mode@ && old.metadata@ == new.metadata@
        && (old.path@ == new.path@ || (absent(base.files@, vstd::utf8::encode_utf8(new.path@))
            && absent(current.files@, vstd::utf8::encode_utf8(old.path@))))
}

/// 与实际判断共用读取条件；已分类、错误模式、元数据变化和复制均不读取源码。
pub fn needs_source(old: &FileObservation, new: &FileObservation, base: &Endpoint, current: &Endpoint)
    -> (yes: bool)
    ensures yes == (!new.classified && comparable(old, new, base, current)),
{
    !new.classified && old.registered && new.registered && is_regular(old) && is_regular(new)
        && super::super::equal_text(old.mode.as_str(), new.mode.as_str())
        && equal_bytes(old.metadata.as_slice(), new.metadata.as_slice())
        && (super::super::equal_text(old.path.as_str(), new.path.as_str())
            || (path_absent(&base.files, new.path.as_str().as_bytes())
                && path_absent(&current.files, old.path.as_str().as_bytes())))
}

/// 完整返回合同：每条选择路径的来源、拒绝、原 IO 错误及准入字节均由固定输入决定。
pub open spec fn assessment<I: Read>(base: &Endpoint, current: &Endpoint, path: Seq<u8>, io: &I)
    -> Result<AssessmentView, Seq<char>>
{
    match relocation(current.projects@, base.projects@, path) {
        None => Ok((None, None)),
        Some(origin) => match (file_index(base.files@, origin), file_index(current.files@, path)) {
            (Some(a), Some(b)) => {
                let old = &base.files[a]; let new = &current.files[b];
                let history = Some((old.path@, new.path@));
                if new.classified || !comparable(old, new, base, current) { Ok((history, None)) }
                else { match io.contents(true, old.path@) {
                    Err(error) => Err(error),
                    Ok(left) => match io.contents(false, new.path@) {
                        Err(error) => Err(error),
                        Ok(right) => {
                            if source_equal(left, right, 0, 0, vstd::utf8::encode_utf8(old.path@),
                                vstd::utf8::encode_utf8(new.path@), base, current, manifest_path(path)) {
                                Ok((history, Some((old.path@, new.path@, left, right, manifest_path(path)))))
                            } else { Ok((history, None)) }
                        },
                    },
                } }
            },
            _ => Ok((None, None)),
        },
    }
}

fn assess_path<I: Read>(base: &Endpoint, current: &Endpoint, path: &[u8], io: &I)
    -> (out: Result<Assessment, String>)
    ensures match out { Ok(value) => assessment(base, current, path@, io) == Ok::<AssessmentView, Seq<char>>(value.view()),
        Err(error) => assessment(base, current, path@, io) == Err::<AssessmentView, Seq<char>>(error@) },
{
    let origin = match relocate(path, &current.projects, &base.projects) {
        None => { return Ok(Assessment { origin: None, record: None }); }, Some(value) => value,
    };
    let a = find_file(&base.files, origin.as_slice());
    let b = find_file(&current.files, path);
    if let (Some(a), Some(b)) = (a, b) {
        let old = &base.files[a]; let new = &current.files[b];
        let history = Some(Origin { previous: old.path.clone(), current: new.path.clone() });
        if !needs_source(old, new, base, current) {
            return Ok(Assessment { origin: history, record: None });
        }
        let left = match io.blob(true, old.path.as_str()) { Ok(value) => value, Err(error) => { return Err(error); } };
        let right = match io.blob(false, new.path.as_str()) { Ok(value) => value, Err(error) => { return Err(error); } };
        let directory = dirname(path);
        let manifest = match resolve(directory.as_slice(), &[67u8, 97u8, 114u8, 103u8, 111u8, 46u8, 116u8, 111u8, 109u8, 108u8]) {
            None => false, Some(value) => equal_bytes(value.as_slice(), path),
        };
        let record = if same_source(left.as_slice(), right.as_slice(), origin.as_slice(), path, base, current, manifest) {
            Some(Record { previous: old.path.clone(), current: new.path.clone(), left, right, manifest })
        } else { None };
        Ok(Assessment { origin: history, record })
    } else { Ok(Assessment { origin: None, record: None }) }
}

/// 按选择路径原序处理；返回全部判断，或第一个原读取错误，不能默默漏掉合法搬迁。
pub fn assess<I: Read>(base: &Endpoint, current: &Endpoint, paths: &Vec<String>, io: &I)
    -> (out: Result<Continuity, String>)
    ensures match out {
        Ok(evidence) => evidence.view().len() == paths.len()
            && forall|i: int| 0 <= i < paths.len() ==> assessment(base, current,
                vstd::utf8::encode_utf8((#[trigger] paths[i])@), io) == Ok::<AssessmentView, Seq<char>>(evidence.view()[i]),
        Err(error) => exists|i: int| 0 <= i < paths.len()
            && assessment(base, current, vstd::utf8::encode_utf8(paths[i]@), io) == Err::<AssessmentView, Seq<char>>(error@)
            && forall|j: int| 0 <= j < i ==> assessment(base, current,
                vstd::utf8::encode_utf8((#[trigger] paths[j])@), io).is_ok(),
    },
{
    let mut observations: Vec<Assessment> = Vec::new();
    let mut i = 0;
    while i < paths.len()
        invariant 0 <= i <= paths.len(), observations.len() == i,
            forall|j: int| 0 <= j < i ==> assessment(base, current,
                vstd::utf8::encode_utf8((#[trigger] paths[j])@), io) == Ok::<AssessmentView, Seq<char>>(observations[j].view()),
        decreases paths.len() - i,
    {
        let value = match assess_path(base, current, paths[i].as_str().as_bytes(), io) {
            Ok(value) => value, Err(error) => { return Err(error); },
        };
        observations.push(value);
        i += 1;
    }
    Ok(Continuity { observations })
}

pub fn assess_optional<I: Read>(base: Option<&Endpoint>, current: &Endpoint, paths: &Vec<String>, io: &I)
    -> (out: Result<Continuity, String>)
    ensures match out { Ok(evidence) => match base {
        None => evidence.view().len() == 0,
        Some(previous) => evidence.view().len() == paths.len()
            && forall|i: int| 0 <= i < paths.len() ==> assessment(previous, current,
                vstd::utf8::encode_utf8((#[trigger] paths[i])@), io) == Ok::<AssessmentView, Seq<char>>(evidence.view()[i]),
    }, Err(_) => base.is_some() },
{
    match base { None => Ok(Continuity::empty()), Some(previous) => assess(previous, current, paths, io) }
}

pub open spec fn assembled_file(
    file: &FileObservation, rows: Seq<FileObservation>, tree: &FileObservation,
) -> bool {
    file.path@ == tree.path@ && file.mode@ == tree.mode@ && file.kind@ == tree.kind@
        && (match file.registered {
            true => exists|i: int| unique_file(rows, vstd::utf8::encode_utf8(tree.path@), i)
                && file.metadata@ == rows[i].metadata@ && file.classified == rows[i].classified,
            false => !file.classified
                && !exists|i: int| unique_file(rows, vstd::utf8::encode_utf8(tree.path@), i),
        })
}

/// 只按完整路径联接两份真实原库存；没有登记的树条目仍保留，不能冒充 absent。
pub fn assemble(
    projects: Vec<ProjectRoot>, rows: Vec<FileObservation>, tree: Vec<FileObservation>, outputs: Vec<String>,
) -> (out: Endpoint)
    ensures out.projects == projects, out.outputs == outputs, out.files.len() == tree.len(),
        forall|i: int| 0 <= i < tree.len() ==> assembled_file(#[trigger] &out.files[i], rows@, &tree[i]),
{
    let mut files: Vec<FileObservation> = Vec::new();
    let mut i = 0;
    while i < tree.len()
        invariant 0 <= i <= tree.len(), files.len() == i,
            forall|j: int| 0 <= j < i ==> assembled_file(#[trigger] &files[j], rows@, &tree[j]),
        decreases tree.len() - i,
    {
        let index = find_file(&rows, tree[i].path.as_str().as_bytes());
        let mut metadata = Vec::new();
        let mut classified = false;
        if let Some(index) = index {
            append_bytes(&mut metadata, rows[index].metadata.as_slice());
            classified = rows[index].classified;
        }
        files.push(FileObservation { path: tree[i].path.clone(), mode: tree[i].mode.clone(), kind: tree[i].kind.clone(),
            metadata, registered: index.is_some(), classified });
        i += 1;
    }
    Endpoint { projects, files, outputs, reference_syntax: Vec::new() }
}

/// 保留固定端点的所有原字段，仅附加该端原登记的引用声明。
pub fn with_reference_syntax(endpoint: Endpoint, reference_syntax: Vec<ReferenceSyntax>) -> (out: Endpoint)
    ensures out.projects == endpoint.projects, out.files == endpoint.files,
        out.outputs == endpoint.outputs, out.reference_syntax == reference_syntax,
{
    Endpoint { projects: endpoint.projects, files: endpoint.files, outputs: endpoint.outputs, reference_syntax }
}

} // verus!

/// 以下仅为固定 Reader、Serde 和树迭代的接口适配；准入不在这里产生。
#[cfg(not(verus_keep_ghost))]
pub struct FixedRead<'a> {
    pub reader: &'a crate::facts::Reader,
    pub root: &'a std::path::Path,
    pub base: &'a str,
    pub candidate: &'a str,
}

#[cfg(not(verus_keep_ghost))]
impl Read for FixedRead<'_> {
    fn blob(&self, base: bool, path: &str) -> Result<Vec<u8>, String> {
        self.reader.blob(
            self.root,
            select_endpoint(base, self.base, self.candidate),
            path,
        )
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(serde::Deserialize)]
struct DecodedFile {
    path: String,
    owner: serde_json::Value,
    surface: serde_json::Value,
    cost: serde_json::Value,
    edges: serde_json::Value,
    symlink: Option<serde_json::Value>,
    projection: Option<serde_json::Value>,
    verification: Option<super::SourceVerification>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(serde::Deserialize)]
struct DecodedFiles {
    files: Vec<DecodedFile>,
    #[serde(default)]
    reference_syntax: Vec<ReferenceSyntax>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(serde::Deserialize)]
struct DecodedProjects {
    projects: Vec<ProjectRoot>,
}

#[cfg(not(verus_keep_ghost))]
pub fn observe_endpoint(
    reader: &crate::facts::Reader,
    root: &std::path::Path,
    oid: &str,
    filemap: &serde_json::Value,
    projects: &serde_json::Value,
    outputs: Vec<String>,
) -> Result<Endpoint, String> {
    let files: DecodedFiles = serde_json::from_value(filemap.clone()).map_err(|e| e.to_string())?;
    let projects: DecodedProjects =
        serde_json::from_value(projects.clone()).map_err(|e| e.to_string())?;
    let rows = files
        .files
        .into_iter()
        .map(|row| {
            Ok(FileObservation {
                path: row.path,
                mode: String::new(),
                kind: String::new(),
                registered: false,
                classified: row.verification.is_some(),
                metadata: serde_json::to_vec(&(
                    row.owner,
                    row.surface,
                    row.cost,
                    row.edges,
                    row.symlink,
                    row.projection,
                ))
                .map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let tree = reader
        .tree(root, oid)?
        .into_iter()
        .map(|(path, entry)| FileObservation {
            path: path.as_str().to_owned(),
            mode: entry.mode,
            kind: entry.kind,
            registered: false,
            classified: false,
            metadata: Vec::new(),
        })
        .collect();
    Ok(with_reference_syntax(
        assemble(projects.projects, rows, tree, outputs),
        files.reference_syntax,
    ))
}

/// 固定 Reader 的输入适配：只预取实际比较需要的两端路径，随后由原判断消费缓存。
/// 批量传输失败保留原错误；这里不产生连续性或证明资格。
#[cfg(not(verus_keep_ghost))]
pub fn acquire_sources(
    base: &Endpoint,
    current: &Endpoint,
    paths: &Vec<String>,
    mut acquire: impl FnMut(bool, &[String]) -> Result<(), String>,
) -> Result<(), String> {
    let mut previous = Vec::new();
    let mut candidate = Vec::new();
    for path in paths {
        let Some(origin) = relocate(path.as_bytes(), &current.projects, &base.projects) else {
            continue;
        };
        let (Some(a), Some(b)) = (
            find_file(&base.files, &origin),
            find_file(&current.files, path.as_bytes()),
        ) else {
            continue;
        };
        let old = &base.files[a];
        let new = &current.files[b];
        if needs_source(old, new, base, current) {
            previous.push(old.path.clone());
            candidate.push(new.path.clone());
        }
    }
    if !previous.is_empty() {
        acquire(true, &previous)?;
        acquire(false, &candidate)?;
    }
    Ok(())
}

/// 所有调用方使用此固定两端接口；完整计划与分类库存共用它的同一结果。
#[cfg(not(verus_keep_ghost))]
pub fn observe(
    reader: &crate::facts::Reader,
    root: &std::path::Path,
    previous: Option<(&str, &serde_json::Value, &serde_json::Value, Vec<String>)>,
    candidate: (&str, &serde_json::Value, &serde_json::Value, Vec<String>),
    paths: &Vec<String>,
) -> Result<Continuity, String> {
    // 空选择没有连续性义务；不为无使用方的证据读取登记或 Git 树。
    if paths.is_empty() {
        return Ok(Continuity::empty());
    }
    let base = previous
        .as_ref()
        .map(|(oid, files, projects, outputs)| {
            observe_endpoint(reader, root, oid, files, projects, outputs.clone())
        })
        .transpose()?;
    let current = observe_endpoint(
        reader,
        root,
        candidate.0,
        candidate.1,
        candidate.2,
        candidate.3,
    )?;
    let io = FixedRead {
        reader,
        root,
        base: previous.map(|row| row.0).unwrap_or(candidate.0),
        candidate: candidate.0,
    };
    if let Some(base) = &base {
        if reader.can_reuse_oid(io.base) && reader.can_reuse_oid(io.candidate) {
            acquire_sources(base, &current, paths, |base, paths| {
                reader
                    .blobs(root, select_endpoint(base, io.base, io.candidate), paths)
                    .map(|_| ())
            })?;
        }
    }
    assess_optional(base.as_ref(), &current, paths, &io)
}
