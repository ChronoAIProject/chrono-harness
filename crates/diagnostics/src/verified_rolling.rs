//! 日志文件数、单记录容量和轮转阈值的同源判断。
#[cfg(not(verus_keep_ghost))]
use crate::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub const MAX_FILES: usize = 64;

pub fn valid_bounds(max_bytes: u64, max_files: usize) -> (valid: bool)
    ensures valid == (max_bytes > 0 && 1 <= max_files && max_files <= MAX_FILES),
{
    max_bytes > 0 && 1 <= max_files && max_files <= MAX_FILES
}

pub fn record_fits(record_bytes: u64, max_bytes: u64) -> (fits: bool)
    ensures fits == (record_bytes <= max_bytes),
{
    record_bytes <= max_bytes
}

/// 用减法实现数学整数的完整长度和比较，避免实际 u64 加法溢出。
pub fn needs_rotation(length: u64, max_bytes: u64, record_bytes: u64) -> (rotate: bool)
    requires record_bytes <= max_bytes,
    ensures rotate == (length as int + record_bytes as int > max_bytes as int),
{
    length > max_bytes - record_bytes
}

}
