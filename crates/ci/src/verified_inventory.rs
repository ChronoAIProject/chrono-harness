//! libtest 清单判断的同源 Verus 定义。
#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

verus! {

pub open spec fn binary_hash_suffix(bytes: Seq<u8>) -> bool {
    bytes.len() > 0 && forall|i: int| 0 <= i < bytes.len() ==> (
        (48 <= #[trigger] bytes[i] && bytes[i] <= 57)
            || (97 <= bytes[i] && bytes[i] <= 102)
    )
}

/// Cargo 二进制后缀必须是非空的小写十六进制串。
pub fn valid_binary_hash_suffix(bytes: &[u8]) -> (valid: bool)
    ensures valid == binary_hash_suffix(bytes@),
{
    if bytes.is_empty() { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant 0 <= i <= bytes.len(), bytes.len() > 0,
            forall|j: int| 0 <= j < i ==> (
                (48 <= #[trigger] bytes@[j] && bytes@[j] <= 57)
                    || (97 <= bytes@[j] && bytes@[j] <= 102)
            ),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if !((48 <= byte && byte <= 57) || (97 <= byte && byte <= 102)) {
            return false;
        }
        i += 1;
    }
    true
}

pub open spec fn same_names(first: Seq<String>, second: Seq<String>) -> bool {
    first.len() == second.len()
        && forall|i: int| 0 <= i < first.len() ==> first[i]@ == second[i]@
}

pub fn same_name_sequence(first: &[String], second: &[String]) -> (same: bool)
    ensures same == same_names(first@, second@),
{
    if first.len() != second.len() { return false; }
    let mut i = 0;
    while i < first.len()
        invariant first.len() == second.len(), i <= first.len(),
            forall|j: int| 0 <= j < i ==> first@[j]@ == second@[j]@,
        decreases first.len() - i,
    {
        if !identifiers::equal_text(first[i].as_str(), second[i].as_str()) { return false; }
        i += 1;
    }
    true
}

pub open spec fn suite_counts_valid(
    tests: nat, benches: nat, names: nat, distinct_names: nat,
) -> bool {
    benches == 0 && tests == names && distinct_names == names
}

pub fn valid_suite_counts(
    tests: usize, benches: usize, names: usize, distinct_names: usize,
) -> (valid: bool)
    ensures valid == suite_counts_valid(tests as nat, benches as nat, names as nat, distinct_names as nat),
{
    benches == 0 && tests == names && distinct_names == names
}

pub open spec fn complete_listing(names_empty: bool, suites: nat, binaries: nat) -> bool {
    names_empty && suites == binaries
}

pub fn valid_complete_listing(names_empty: bool, suites: usize, binaries: usize) -> (valid: bool)
    ensures valid == complete_listing(names_empty, suites as nat, binaries as nat),
{
    names_empty && suites == binaries
}

}
