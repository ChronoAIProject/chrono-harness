//! Input-coverage state boundaries directly exercise the production Verus policy.
#![allow(unexpected_cfgs)]
#[path = "../../judge-registration/src/verified_input_coverage.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn bound_and_absent_require_at_least_one_binding() {
    proof {
        reveal_strlit("bound"); reveal_strlit("absent"); reveal_strlit("draft");
        reveal_strlit("declared-complete");
        assert("bound"@.len() != "absent"@.len());
    }
    let bound = policy::row_state_valid("bound", 1, "draft");
    assert(bound);
    let absent = policy::row_state_valid("absent", usize::MAX, "draft");
    assert(absent);
    let empty_bound = policy::row_state_valid("bound", 0, "draft");
    assert(!empty_bound);
    let empty_absent = policy::row_state_valid("absent", 0, "draft");
    assert(!empty_absent);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn non_applicable_and_draft_unresolved_carry_no_bindings() {
    proof {
        reveal_strlit("bound"); reveal_strlit("absent");
        reveal_strlit("not-applicable"); reveal_strlit("unresolved");
        reveal_strlit("incomplete"); reveal_strlit("declared-complete");
        assert("not-applicable"@.len() != "bound"@.len());
        assert("not-applicable"@.len() != "absent"@.len());
        assert("not-applicable"@.len() != "unresolved"@.len());
        assert("unresolved"@.len() != "bound"@.len());
        assert("unresolved"@.len() != "absent"@.len());
        assert("incomplete"@.len() != "declared-complete"@.len());
    }
    let not_applicable = policy::row_state_valid("not-applicable", 0, "incomplete");
    assert(not_applicable);
    let unresolved = policy::row_state_valid("unresolved", 0, "incomplete");
    assert(unresolved);
    let bound_not_applicable = policy::row_state_valid("not-applicable", 1, "incomplete");
    assert(!bound_not_applicable);
    let bound_unresolved = policy::row_state_valid("unresolved", 1, "incomplete");
    assert(!bound_unresolved);
    let complete_unresolved = policy::row_state_valid("unresolved", 0, "declared-complete");
    assert(!complete_unresolved);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn state_and_completion_comparisons_are_exact() {
    proof {
        reveal_strlit("bound"); reveal_strlit("BOUND"); reveal_strlit("bound\0");
        reveal_strlit("absent"); reveal_strlit("absent\0");
        reveal_strlit("not-applicable"); reveal_strlit("unresolved");
        reveal_strlit("declared-complete"); reveal_strlit("declared-complete\0");
        assert("bound"@[0] != "BOUND"@[0]);
        assert("bound"@.len() != "bound\0"@.len());
        assert("bound"@.len() != "absent"@.len());
        assert("absent"@.len() != "absent\0"@.len());
        assert("bound\0"@[0] != "absent"@[0]);
        assert("bound\0"@.len() != "not-applicable"@.len());
        assert("bound\0"@.len() != "unresolved"@.len());
    }
    let uppercase = policy::row_state_valid("BOUND", 1, "incomplete");
    assert(!uppercase);
    let nul_state = policy::row_state_valid("bound\0", 1, "incomplete");
    assert(!nul_state);
    let absent = policy::is_absent("absent");
    assert(absent);
    let nul_absent = policy::is_absent("absent\0");
    assert(!nul_absent);
    let other = policy::is_absent("not-applicable");
    assert(!other);
}

}
