//! 执行器、登记和路由共用的资源上限判断。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub const MAX_OUTPUT_BYTES: usize = 67_108_864;

pub open spec fn output_limit(bytes: usize) -> bool {
    0 < bytes && bytes <= MAX_OUTPUT_BYTES
}

pub open spec fn execution_bounds(timeout_seconds: u64, output_bytes: usize) -> bool {
    0 < timeout_seconds && output_limit(output_bytes)
}

pub fn valid_output_limit(bytes: usize) -> (valid: bool)
    ensures valid == output_limit(bytes),
{
    bytes > 0 && bytes <= MAX_OUTPUT_BYTES
}

pub fn valid_execution_bounds(timeout_seconds: u64, output_bytes: usize) -> (valid: bool)
    ensures valid == execution_bounds(timeout_seconds, output_bytes),
{
    timeout_seconds > 0 && valid_output_limit(output_bytes)
}

}
