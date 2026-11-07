//! POSIX 相对路径的词法准入。调用方提供 UTF-8 路径字节；
//! 文件系统身份、规范拼写和 glob 政策另由各自所有者处理。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

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
