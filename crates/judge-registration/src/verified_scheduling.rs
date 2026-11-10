//! 调度只要求参与计划的操作都有声明，真实生产声明可独立保留。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
use std::collections::{BTreeMap, BTreeSet};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub fn claims_cover_operations<K: Ord, V>(
    operations: &BTreeSet<K>, claims: &BTreeMap<K, V>,
) -> (covered: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures covered == operations@.subset_of(claims@.dom()),
{
    broadcast use vstd::std_specs::btree::axiom_contains_deref_key;
    let mut remaining = operations.iter();
    loop
        invariant vstd::laws_cmp::obeys_cmp::<K>(),
            remaining@.1.to_set() == operations@,
            0 <= remaining@.0 <= remaining@.1.len(),
            forall|i: int| 0 <= i < remaining@.0
                ==> claims@.contains_key(remaining@.1[i]),
        decreases remaining@.1.len() - remaining@.0 + 1,
    {
        let operation = match remaining.next() {
            Some(operation) => operation,
            None => {
                assert forall|key: K| #[trigger] operations@.contains(key)
                    implies claims@.contains_key(key) by {
                    let at = choose|i: int| 0 <= i < remaining@.1.len()
                        && remaining@.1[i] == key;
                }
                return true;
            },
        };
        if !claims.contains_key(operation) {
            assert(operations@.contains(*operation));
            return false;
        }
    }
}

}
