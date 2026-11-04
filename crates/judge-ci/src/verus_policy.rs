//! The executable admission predicate and its Verus specification share this source.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// Empty inventories cannot establish coverage. For a nonempty inventory,
/// return the first unsatisfied obligation, retaining its caller-owned identity.
pub fn first_unverified(obligations: &Vec<bool>) -> (result: Option<usize>)
    ensures
        result.is_none() <==> (forall|i: int| 0 <= i < obligations.len() ==> obligations[i]),
        result.is_some() ==> (
            0 <= result.unwrap() < obligations.len()
            && !obligations[result.unwrap() as int]
            && forall|i: int| 0 <= i < result.unwrap() ==> obligations[i]
        ),
{
    let mut i = 0;
    while i < obligations.len()
        invariant
            0 <= i <= obligations.len(),
            forall|j: int| 0 <= j < i ==> obligations[j],
        decreases obligations.len() - i,
    {
        if !obligations[i] {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Absence of obligations is a registration failure, not a passing proof.
pub fn complete(obligations: &Vec<bool>) -> (result: bool)
    ensures result <==> (
        obligations.len() > 0
        && forall|i: int| 0 <= i < obligations.len() ==> obligations[i]
    ),
{
    !obligations.is_empty() && first_unverified(obligations).is_none()
}

}
