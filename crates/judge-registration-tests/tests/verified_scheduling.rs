//! 独立边界：缺少参与操作的声明须拒绝，额外声明和值不影响覆盖。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../judge-registration/src/verified_scheduling.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_judge_registration::verified_scheduling as policy;
use std::collections::{BTreeMap, BTreeSet};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn coverage_requires_all_participants_and_allows_production_claims() {
    let empty: BTreeSet<usize> = BTreeSet::new();
    let mut operations: BTreeSet<usize> = BTreeSet::new();
    operations.insert(2);
    operations.insert(4);
    let mut claims: BTreeMap<usize, bool> = BTreeMap::new();

    let empty_covered = policy::claims_cover_operations(&empty, &claims);
    assert(empty_covered);
    let absent = policy::claims_cover_operations(&operations, &claims);
    assert(!absent);

    claims.insert(2, true);
    let missing_last = policy::claims_cover_operations(&operations, &claims);
    assert(!missing_last);
    claims.insert(9, false);
    let equal_count_wrong_keys = policy::claims_cover_operations(&operations, &claims);
    assert(!equal_count_wrong_keys);

    claims.insert(4, true);
    let extra_production_claim = policy::claims_cover_operations(&operations, &claims);
    assert(extra_production_claim);
    let no_participants = policy::claims_cover_operations(&empty, &claims);
    assert(no_participants);
    operations.insert(9);
    let false_value_is_still_a_claim = policy::claims_cover_operations(&operations, &claims);
    assert(false_value_is_still_a_claim);

    operations.insert(1);
    let missing_first = policy::claims_cover_operations(&operations, &claims);
    assert(!missing_first);
}

}
