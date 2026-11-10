//! Integration certificate binding admission owned by a proved predicate.
#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// A certificate binding is admissible only when its cardinality and exact
/// field-set checks both hold in the concrete adapter.
pub open spec fn binding_admissible(
    field_count: bool,
    exact_fields: bool,
) -> bool {
    field_count && exact_fields
}

/// Consume the adapted binding facts through the proved certificate rule.
pub fn admit_binding(
    field_count: bool,
    exact_fields: bool,
) -> (admitted: bool)
    ensures admitted == binding_admissible(field_count, exact_fields),
{
    field_count && exact_fields
}

}
