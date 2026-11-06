//! Boundary proofs for the shared execution limit predicate.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_limits.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_limits as policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

fn accepted_limits() {
    assert(policy::execution_bounds([1u64, 1]@));
    assert(policy::execution_bounds([30u64, 67_108_864]@));
}

fn rejected_limits() {
    assert(!policy::execution_bounds([0u64, 1]@));
    assert(!policy::execution_bounds([1u64, 0]@));
    assert(!policy::execution_bounds([1u64, 67_108_865]@));
    assert(!policy::execution_bounds([1u64]@));
}

}

#[cfg(not(verus_keep_ghost))]
mod runtime {
    use super::policy;

    #[test]
    fn accepted_limits() {
        assert!(policy::valid_execution_bounds(&[1u64, 1]));
        assert!(policy::valid_execution_bounds(&[30u64, 67_108_864]));
    }

    #[test]
    fn rejected_limits() {
        assert!(!policy::valid_execution_bounds(&[0u64, 1]));
        assert!(!policy::valid_execution_bounds(&[1u64, 0]));
        assert!(!policy::valid_execution_bounds(&[1u64, 67_108_865]));
        assert!(!policy::valid_execution_bounds(&[1u64]));
    }
}
