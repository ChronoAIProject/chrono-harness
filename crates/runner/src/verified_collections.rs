//! 标准树集合的顺序不变量和全文查询／插入；生产入口与严格预期共用。
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::{borrow::BorrowSpec, cmp::OrdSpec, iter::IteratorSpec};

verus! {

/// 只构造标准借用迭代器，不遍历或复制库存。
pub fn btree_map_compare_invariant<K: Ord, V>(map: &BTreeMap<K, V>)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures vstd::std_specs::btree::btree_map_inv(map),
{
    let iterator = map.iter();
    proof {
        let rows = iterator.remaining();
        let keys = rows.map_values(|row: (&K, &V)| *row.0);
        vstd::std_specs::btree::axiom_increasing_seq_meaning(keys);
        reveal(vstd::laws_cmp::obeys_cmp_ord);
        reveal(vstd::laws_cmp::obeys_partial_cmp_spec_properties);
        assert forall|a: K, b: K| #![trigger map@.contains_key(a), map@.contains_key(b)]
            map@.contains_key(a) && map@.contains_key(b) && a != b
                implies !(a.cmp_spec(&b) is Equal) by {
            assert(rows.unref().to_set().contains((a, map@[a])));
            assert(rows.unref().to_set().contains((b, map@[b])));
            let i = choose|i: int| 0 <= i < rows.len() && rows.unref()[i] == (a, map@[a]);
            let j = choose|j: int| 0 <= j < rows.len() && rows.unref()[j] == (b, map@[b]);
            if i < j { assert(keys[i].cmp_spec(&keys[j]) is Less); }
            else if j < i { assert(keys[j].cmp_spec(&keys[i]) is Less); }
        }
    }
}

pub fn btree_set_compare_invariant<K: Ord>(set: &BTreeSet<K>)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures vstd::std_specs::btree::btree_set_inv(set),
{
    let iterator = set.iter();
    proof {
        let rows = iterator.remaining();
        vstd::std_specs::btree::axiom_increasing_seq_meaning(rows);
        reveal(vstd::laws_cmp::obeys_cmp_ord);
        reveal(vstd::laws_cmp::obeys_partial_cmp_spec_properties);
        assert forall|a: K, b: K| #![trigger set@.contains(a), set@.contains(b)]
            set@.contains(a) && set@.contains(b) && a != b
                implies !(a.cmp_spec(&b) is Equal) by {
            let i = choose|i: int| 0 <= i < rows.len() && rows.unref()[i] == a;
            let j = choose|j: int| 0 <= j < rows.len() && rows.unref()[j] == b;
            if i < j { assert(rows[i].cmp_spec(&rows[j]) is Less); }
            else if j < i { assert(rows[j].cmp_spec(&rows[i]) is Less); }
        }
    }
}

pub fn contains_string_key<V>(map: &BTreeMap<String, V>, key: &str) -> (out: bool)
    requires vstd::laws_cmp::obeys_cmp::<String>(),
        vstd::laws_cmp::obeys_cmp::<&str>(),
        vstd::std_specs::cmp::borrowed_key_ordering_matches::<String, str>(),
    ensures out <==> exists|stored: String|
        #[trigger] map@.contains_key(stored) && stored@ == key@,
{
    btree_map_compare_invariant(map);
    let found = map.contains_key(key);
    proof {
        reveal(vstd::laws_cmp::obeys_cmp_ord);
        reveal(vstd::laws_cmp::obeys_cmp_partial_ord);
        assert forall|stored: String| #[trigger] map@.contains_key(stored) implies
            (key.cmp_spec(<String as BorrowSpec<str>>::borrow_spec(&stored)) is Equal
                <==> stored@ == key@) by {
            vstd::std_specs::borrow::axiom_string_borrow(&stored);
        }
    }
    found
}

pub fn insert_str_key<'a>(set: &mut BTreeSet<&'a str>, text: &'a str) -> (fresh: bool)
    requires vstd::laws_cmp::obeys_cmp::<&str>(),
    ensures
        final(set).deep_view() == old(set).deep_view().insert(text@),
        fresh == !old(set).deep_view().contains(text@),
        !fresh ==> final(set)@ == old(set)@,
{
    btree_set_compare_invariant(set);
    let ghost before = *set;
    proof {
        reveal(vstd::laws_cmp::obeys_cmp_ord);
        reveal(vstd::laws_cmp::obeys_cmp_partial_ord);
        assert(vstd::laws_cmp::obeys_cmp_deep_eq::<&str>());
    }
    let fresh = set.insert(text);
    proof {
        vstd::std_specs::btree::lemma_btree_set_insert_ensures_if_obeys_cmp_deep_eq(
            &before, set, text, fresh);
    }
    fresh
}

} // verus!
