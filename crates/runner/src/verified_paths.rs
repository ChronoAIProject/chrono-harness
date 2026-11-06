//! Lexical POSIX relative-path admission. Callers supply UTF-8 path bytes;
//! filesystem identity, canonical spelling and glob policy remain separate.
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

#[verifier::inline]
pub open spec fn current_component_at(path: Seq<u8>, i: int) -> bool {
    0 <= i < path.len()
        && (i == 0 || path[i - 1] == 47)
        && path[i] == 46
        && (i + 1 == path.len() || path[i + 1] == 47)
}

/// Matches the runner's POSIX lexical domain: no root, current-dir or parent-dir
/// component, NUL, LF, CR or backslash. Repeated and trailing separators do not
/// change admission, matching `Path::components` on the Unix hosts.
#[verifier::inline]
pub open spec fn relative_path(path: Seq<u8>) -> bool {
    path.len() > 0 && path[0] != 47
        && (forall|i: int| 0 <= i < path.len() ==> (
            path_byte(#[trigger] path[i])
                && !current_component_at(path, i)
                && !parent_component_at(path, i)
        ))
}

pub fn valid_relative_path(path: &[u8]) -> (valid: bool)
    ensures valid == relative_path(path@),
{
    if path.len() == 0 || path[0] == 47
    {
        return false;
    }
    let mut i = 0;
    while i < path.len()
        invariant
            0 <= i <= path.len(),
            path.len() > 0 && path[0] != 47,
            forall|j: int| 0 <= j < i ==> (
                path_byte(#[trigger] path[j])
                    && !current_component_at(path@, j)
                    && !parent_component_at(path@, j)
            ),
        decreases path.len() - i,
    {
        let byte = path[i];
        if byte == 0 || byte == 10 || byte == 13 || byte == 92 {
            return false;
        }
        if byte == 46 && (i == 0 || path[i - 1] == 47)
            && (i + 1 == path.len() || path[i + 1] == 47)
        {
            return false;
        }
        if byte == 46 && (i == 0 || path[i - 1] == 47)
            && i + 1 < path.len() && path[i + 1] == 46
            && (i + 2 == path.len() || path[i + 2] == 47)
        {
            return false;
        }
        i += 1;
    }
    true
}

/// A retained evidence reference must point below the ignored host state
/// directory.  The prefix is part of the storage contract; it is not inferred
/// from a caller's directory layout.
#[verifier::inline]
pub open spec fn state_path(path: Seq<u8>) -> bool {
    relative_path(path)
        && path.len() > 22
        && path[0] == 46 && path[1] == 99 && path[2] == 104 && path[3] == 114
        && path[4] == 111 && path[5] == 110 && path[6] == 111 && path[7] == 45
        && path[8] == 104 && path[9] == 97 && path[10] == 114 && path[11] == 110
        && path[12] == 101 && path[13] == 115 && path[14] == 115 && path[15] == 47
        && path[16] == 115 && path[17] == 116 && path[18] == 97 && path[19] == 116
        && path[20] == 101 && path[21] == 47
}

pub fn valid_state_path(path: &[u8]) -> (valid: bool)
    ensures valid == state_path(path@),
{
    let relative = valid_relative_path(path);
    if !relative || path.len() <= 22 {
        return false;
    }
    if path[0] != 46 || path[1] != 99 || path[2] != 104 || path[3] != 114
        || path[4] != 111 || path[5] != 110 || path[6] != 111 || path[7] != 45
        || path[8] != 104 || path[9] != 97 || path[10] != 114 || path[11] != 110
        || path[12] != 101 || path[13] != 115 || path[14] != 115 || path[15] != 47
        || path[16] != 115 || path[17] != 116 || path[18] != 97 || path[19] != 116
        || path[20] != 101 || path[21] != 47
    {
        return false;
    }
    true
}

}
