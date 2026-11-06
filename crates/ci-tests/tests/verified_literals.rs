//! Boundary proofs for CI literal input admission.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_literals.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_literals as policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

fn accepted_literals() {
    assert(policy::literal_byte(97));
    assert(policy::literal_byte(32));
    assert(policy::literal_byte(127));
    assert(!policy::template_at([36u8, 123]@, 0));
}

fn rejected_literals() {
    assert(!policy::literal_byte(0));
    assert(!policy::literal_byte(10));
    assert(!policy::literal_byte(13));
    assert(policy::template_at([36u8, 123, 123]@, 0));
    assert(policy::template_at([97u8, 36, 123, 123, 98]@, 1));
}

}

#[cfg(not(verus_keep_ghost))]
mod runtime {
    use super::policy;

    #[test]
    fn accepted_literals() {
        assert!(policy::valid_literal(&[97u8, 32, 127]));
        assert!(policy::valid_literal(&[]));
        assert!(policy::valid_literal(&[36u8, 123]));
    }

    #[test]
    fn rejected_literals() {
        assert!(!policy::valid_literal(&[0u8]));
        assert!(!policy::valid_literal(&[10u8]));
        assert!(!policy::valid_literal(&[13u8]));
        assert!(!policy::valid_literal(&[36u8, 123, 123]));
        assert!(!policy::valid_literal(&[97u8, 36, 123, 123, 98]));
    }
}
