//! Verus authority for operation resource limits.
//!
//! The registration and route consumers use this predicate before observing
//! tools or launching work.  The surrounding crates own parsing and transport;
//! this module owns the shared admission rule.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub const MAX_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;

#[verifier::inline]
pub open spec fn execution_bounds(limits: Seq<u64>) -> bool {
    limits.len() == 2
        && limits[0] > 0
        && limits[1] > 0
        && limits[1] <= MAX_OUTPUT_BYTES
}

/// Validate the limits consumed by both registered plans and route plans.
pub fn valid_execution_bounds(limits: &[u64]) -> (valid: bool)
    ensures valid == execution_bounds(limits@),
{
    let mut i = 0;
    let mut valid = true;
    while i < limits.len()
        invariant
            0 <= i <= limits.len(),
            valid == (forall|j: int| 0 <= j < i ==> (
                if j == 0 {
                    limits[j] > 0
                } else {
                    limits[j] > 0 && limits[j] <= MAX_OUTPUT_BYTES
                }
            )),
        decreases limits.len() - i,
    {
        valid = valid && if i == 0 {
            limits[i] > 0
        } else {
            limits[i] > 0 && limits[i] <= MAX_OUTPUT_BYTES
        };
        i += 1;
    }
    valid && limits.len() == 2
}

}
