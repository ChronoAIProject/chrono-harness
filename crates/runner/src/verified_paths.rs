//! POSIX 相对路径的词法准入。调用方提供 UTF-8 路径字节；
//! 文件系统身份、规范拼写和 glob 政策另由各自所有者处理。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

/// 只接受带末尾斜杠、分量非空且不含单独 . 或 .. 的声明，不归一化输入。
pub open spec fn artifact_component_ok(path: Seq<u8>, i: int) -> bool {
    (i == 0 || path[i - 1] == 47) ==> path[i] != 47
        && !(path[i] == 46 && i + 1 < path.len() && path[i + 1] == 47)
        && !(path[i] == 46 && i + 2 < path.len()
            && path[i + 1] == 46 && path[i + 2] == 47)
}

pub open spec fn artifact_directory(path: Seq<u8>) -> bool {
    path.len() > 0 && path.last() == 47
        && forall|i: int| 0 <= i < path.len() ==> artifact_component_ok(path, i)
}

pub fn valid_artifact_directory(path: &[u8]) -> (valid: bool)
    ensures valid == artifact_directory(path@),
{
    if path.len() == 0 || path[path.len() - 1] != 47 { return false; }
    let mut i = 0;
    while i < path.len()
        invariant path.len() > 0, path@[path.len() - 1] == 47, i <= path.len(),
            forall|j: int| 0 <= j < i ==> artifact_component_ok(path@, j),
        decreases path.len() - i,
    {
        if i == 0 || path[i - 1] == 47 {
            if path[i] == 47
                || (path[i] == 46 && path.len() - i >= 2 && path[i + 1] == 47)
                || (path[i] == 46 && path.len() - i >= 3
                    && path[i + 1] == 46 && path[i + 2] == 47)
            {
                assert(!artifact_component_ok(path@, i as int));
                return false;
            }
        }
        i += 1;
    }
    true
}

pub open spec fn literal_directory(path: &str) -> Option<&str> {
    if artifact_directory(path.spec_bytes()) { Some(path) }
    else { None }
}

/// 完整过滤保持输入顺序、重复和原文借用；字面前缀由原标准格式接口输出。
pub fn literal_artifact_directories<'a>(paths: &[&'a str]) -> (out: Vec<&'a str>)
    ensures out@ == paths@.filter_map(|path: &str| literal_directory(path)),
{
    let mut out = Vec::new();
    let mut i = 0;
    while i < paths.len()
        invariant i <= paths.len(),
            out@ == paths@.take(i as int).filter_map(|path: &str| literal_directory(path)),
        decreases paths.len() - i,
    {
        proof {
            paths@.lemma_filter_map_take_succ(|path: &str| literal_directory(path), i as int);
        }
        let path = paths[i];
        if valid_artifact_directory(path.as_bytes()) {
            out.push(path);
        }
        i += 1;
    }
    assert(paths@.take(paths.len() as int) =~= paths@);
    out
}

#[verifier::inline]
pub open spec fn path_byte(byte: u8) -> bool {
    byte != 0 && byte != 10 && byte != 13 && byte != 92
}

#[verifier::inline]
pub open spec fn parent_component_at(path: Seq<u8>, i: int) -> bool {
    0 <= i < path.len()
        && (i == 0 || path[i - 1] == 47)
        && path[i] == 46
        && i + 1 < path.len() && path[i + 1] == 46
        && (i + 2 == path.len() || path[i + 2] == 47)
}

/// 精确匹配 runner 的 POSIX 词法范围：拒绝根、开头的当前目录、父目录分量、
/// NUL、LF、CR 和反斜杠；重复／末尾分隔符及非开头的当前目录分量不影响准入。
#[verifier::inline]
pub open spec fn relative_path(path: Seq<u8>) -> bool {
    path.len() > 0 && path[0] != 47
        && !(path[0] == 46 && (path.len() == 1 || path[1] == 47))
        && (forall|i: int| 0 <= i < path.len() ==> (
            path_byte(#[trigger] path[i]) && !parent_component_at(path, i)
        ))
}

pub fn valid_relative_path(path: &[u8]) -> (valid: bool)
    ensures valid == relative_path(path@),
{
    if path.len() == 0 || path[0] == 47
        || (path[0] == 46 && (path.len() == 1 || path[1] == 47))
    {
        return false;
    }
    let mut i = 0;
    while i < path.len()
        invariant
            0 <= i <= path.len(),
            path.len() > 0 && path[0] != 47,
            !(path[0] == 46 && (path.len() == 1 || path[1] == 47)),
            forall|j: int| 0 <= j < i ==> (
                path_byte(#[trigger] path[j]) && !parent_component_at(path@, j)
            ),
        decreases path.len() - i,
    {
        let byte = path[i];
        if byte == 0 || byte == 10 || byte == 13 || byte == 92 {
            return false;
        }
        if byte == 46 && (i == 0 || path[i - 1] == 47)
            && path.len() - i >= 2 && path[i + 1] == 46
            && (path.len() - i == 2 || path[i + 2] == 47)
        {
            return false;
        }
        i += 1;
    }
    true
}

#[verifier::inline]
pub open spec fn host_state_prefix() -> Seq<u8> {
    seq![46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47]
}

/// state 根之后须有非空字节后缀；不增加规范拼写或文件系统身份约束。
#[verifier::inline]
pub open spec fn host_state_path(path: Seq<u8>) -> bool {
    relative_path(path) && path.len() > 22
        && (forall|i: int| 0 <= i < 22 ==> #[trigger] path[i] == host_state_prefix()[i])
}

pub fn valid_host_state_path(path: &[u8]) -> (valid: bool)
    ensures valid == host_state_path(path@),
{
    if path.len() <= 22 {
        return false;
    }
    let prefix = [46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47];
    let mut i = 0;
    while i < 22
        invariant
            0 <= i <= 22,
            path.len() > 22,
            prefix@ == host_state_prefix(),
            forall|j: int| 0 <= j < i ==> #[trigger] path[j] == host_state_prefix()[j],
        decreases 22 - i,
    {
        if path[i] != prefix[i] {
            return false;
        }
        i += 1;
    }
    valid_relative_path(path)
}

}
