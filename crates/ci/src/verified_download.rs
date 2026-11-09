//! 原生工件下载的状态码解析与有限重试判断；实际传输仍由 gather 负责。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub open spec fn token_at(bytes: Seq<u8>, i: int) -> bool {
    0 <= i && i + 8 < bytes.len()
        && bytes[i] == 72 && bytes[i + 1] == 84 && bytes[i + 2] == 84
        && bytes[i + 3] == 80 && bytes[i + 4] == 32 && bytes[i + 8] == 58
        && 48 <= bytes[i + 5] <= 57 && 48 <= bytes[i + 6] <= 57
        && 48 <= bytes[i + 7] <= 57
}

pub open spec fn code_at(bytes: Seq<u8>, i: int) -> int {
    100 * (bytes[i + 5] as int - 48)
        + 10 * (bytes[i + 6] as int - 48) + bytes[i + 7] as int - 48
}

pub open spec fn prefix(bytes: Seq<u8>, end: int, status: Option<u16>) -> bool {
    match status {
        None => forall|i: int| 0 <= i < end ==> !token_at(bytes, i),
        Some(code) =>
            (exists|i: int| 0 <= i < end && token_at(bytes, i) && code_at(bytes, i) == code)
            && (forall|i: int| 0 <= i < end && token_at(bytes, i) ==> code_at(bytes, i) == code),
    }
}

pub open spec fn parsed(bytes: Seq<u8>, status: Option<u16>) -> bool {
    match status {
        Some(_) => prefix(bytes, bytes.len() as int, status),
        None => prefix(bytes, bytes.len() as int, None)
            || exists|i: int, j: int| token_at(bytes, i) && token_at(bytes, j)
                && code_at(bytes, i) != code_at(bytes, j),
    }
}

fn token(bytes: &[u8], i: usize) -> (status: Option<u16>)
    requires i + 8 < bytes.len(),
    ensures match status {
        Some(code) => token_at(bytes@, i as int) && code_at(bytes@, i as int) == code,
        None => !token_at(bytes@, i as int),
    },
{
    if bytes[i] != 72 || bytes[i + 1] != 84 || bytes[i + 2] != 84
        || bytes[i + 3] != 80 || bytes[i + 4] != 32 || bytes[i + 8] != 58
        || bytes[i + 5] < 48 || bytes[i + 5] > 57
        || bytes[i + 6] < 48 || bytes[i + 6] > 57
        || bytes[i + 7] < 48 || bytes[i + 7] > 57 { return None; }
    Some(100 * (bytes[i + 5] - 48) as u16
        + 10 * (bytes[i + 6] - 48) as u16 + (bytes[i + 7] - 48) as u16)
}

/// 只接受完整的 HTTP nnn:；重复相同状态允许，任意矛盾状态拒绝。
pub fn http_status(bytes: &[u8]) -> (status: Option<u16>)
    ensures parsed(bytes@, status),
{
    if bytes.len() < 9 { return None; }
    let end = bytes.len() - 8;
    let mut i = 0;
    let mut status: Option<u16> = None;
    while i < end
        invariant bytes.len() >= 9, end == bytes.len() - 8,
            0 <= i <= end, prefix(bytes@, i as int, status),
        decreases end - i,
    {
        if let Some(current) = token(bytes, i) {
            if let Some(prior) = status {
                if prior != current {
                    proof {
                        let previous = choose|j: int| 0 <= j < i
                            && token_at(bytes@, j) && code_at(bytes@, j) == prior;
                        assert(token_at(bytes@, previous) && token_at(bytes@, i as int)
                            && code_at(bytes@, previous) != code_at(bytes@, i as int));
                    }
                    return None;
                }
            }
            status = Some(current);
        }
        i += 1;
    }
    status
}

/// 所有外部观察由调用方提供；失败、次数、流容量、状态白名单和期限缺一不可。
pub fn retry(
    exit_ok: bool, process_failed: bool, attempts_remaining: bool,
    status_allowed: bool, stdout_remaining: usize, stderr_remaining: usize,
    delay_fits: bool,
) -> (allowed: bool)
    ensures allowed == (!exit_ok && !process_failed && attempts_remaining
        && status_allowed && stdout_remaining > 0 && stderr_remaining > 0 && delay_fits),
{
    !exit_ok && !process_failed && attempts_remaining && status_allowed
        && stdout_remaining > 0 && stderr_remaining > 0 && delay_fits
}

}
