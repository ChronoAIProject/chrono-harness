//! 执行器、登记和路由共用的资源上限判断。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "maintenance_signal/verified_send.rs"]
pub mod maintenance_signal;

verus! {

/// Socket observation does not authorize a strict footprint or any effect.
pub fn observable_socket(observe_sockets: bool, is_socket: bool) -> (allowed: bool)
    ensures allowed == (observe_sockets && is_socket),
{ observe_sockets && is_socket }

pub const MAX_OUTPUT_BYTES: usize = 67_108_864;

// One low bit distinguishes child and launcher events; twelve bits identify
// the existing 4096 launch slots. The remaining bits belong to the generation.
pub const LAUNCH_CAPACITY: usize = 4096;
pub const MAX_LAUNCH_GENERATION: usize = (usize::MAX - 8191) / 8192;

pub fn next_launch_generation(current: usize) -> (next: Option<usize>)
    ensures
        next.is_some() <==> 0 < current && current <= MAX_LAUNCH_GENERATION,
        next.is_some() ==> next.unwrap() == current + 1,
{
    if current == 0 || current > MAX_LAUNCH_GENERATION {
        None
    } else {
        Some(current + 1)
    }
}

/// The even cookie leaves its following odd value for the launcher event.
pub fn launch_cookie(index: usize, generation: usize) -> (cookie: Option<usize>)
    ensures
        cookie.is_some() <==> index < LAUNCH_CAPACITY
            && 0 < generation && generation <= MAX_LAUNCH_GENERATION,
        cookie.is_some() ==> {
            &&& cookie.unwrap() as int == generation as int * 8192 + index as int * 2
            &&& cookie.unwrap() < usize::MAX
            &&& (cookie.unwrap() as int) % 2 == 0
            &&& ((cookie.unwrap() as int) / 2) % (LAUNCH_CAPACITY as int) == index as int
            &&& (cookie.unwrap() as int) / (LAUNCH_CAPACITY as int * 2) == generation as int
            &&& ((cookie.unwrap() as int + 1) / 2) % (LAUNCH_CAPACITY as int) == index as int
            &&& (cookie.unwrap() as int + 1) / (LAUNCH_CAPACITY as int * 2) == generation as int
        },
{
    if index >= LAUNCH_CAPACITY || generation == 0 || generation > MAX_LAUNCH_GENERATION {
        return None;
    }
    Some(generation * 8192 + index * 2)
}

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
