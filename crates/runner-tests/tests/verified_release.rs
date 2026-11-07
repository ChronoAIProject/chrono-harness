//! 交接边界的独立预期，直接消费真实 Verus 实现。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/retained_artifacts/verified_release.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_release as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn explicit_selection_is_nonempty_unique_and_bounded() {
    broadcast use vstd::array::group_array_axioms;
    proof { reveal_strlit("output-a"); reveal_strlit("output-b"); }
    let empty: Vec<String> = Vec::new();
    let one = vec!["output-a".to_owned()];
    let duplicate = vec!["output-a".to_owned(), "output-a".to_owned()];
    let two = vec!["output-a".to_owned(), "output-b".to_owned()];
    proof {
        assert("output-a"@[7] != "output-b"@[7]);
        assert(two[0]@ == "output-a"@ && two[1]@ == "output-b"@);
        assert forall|i: int, j: int| 0 <= i < j < two.len()
            implies two[i]@ != two[j]@ by { assert(i == 0 && j == 1); }
    }
    let e = policy::unique_selection(&empty, 512);
    let a = policy::unique_selection(&one, 1);
    let z = policy::unique_selection(&one, 0);
    let d = policy::unique_selection(&duplicate, 512);
    let b = policy::unique_selection(&two, 2);
    let over = policy::unique_selection(&two, 1);
    assert(!e); assert(a); assert(!z); assert(!d); assert(b); assert(!over);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn roles_are_complete_and_release_preserves_other_members() {
    broadcast use vstd::array::group_array_axioms;
    proof {
        reveal_strlit("recovery"); reveal_strlit("current");
        reveal_strlit("kept"); reveal_strlit("retired"); reveal_strlit("last");
        reveal_with_fuel(policy::removed, 4);
    }
    let roles = vec!["recovery".to_owned(), "current".to_owned()];
    let reversed = vec!["current".to_owned(), "recovery".to_owned()];
    let incomplete = vec!["recovery".to_owned()];
    proof {
        assert(roles[0]@ == "recovery"@ && roles[1]@ == "current"@);
        assert(incomplete[0]@ == "recovery"@);
        assert(policy::present(roles@, "current"@));
        assert(!policy::present(incomplete@, "current"@));
    }
    let complete = policy::same_members(&roles, &reversed);
    let short = policy::same_members(&roles, &incomplete);
    let extra = policy::same_members(&incomplete, &roles);
    assert(complete); assert(!short); assert(!extra);
    let members = vec!["kept".to_owned(), "retired".to_owned(), "last".to_owned()];
    let remaining = policy::without(&members, "retired");
    proof {
        reveal_strlit("kept"); reveal_strlit("retired"); reveal_strlit("last");
        assert("kept"@[0] != "retired"@[0] && "last"@[0] != "retired"@[0]);
        assert(members[0]@ == "kept"@ && members[1]@ == "retired"@ && members[2]@ == "last"@);
        reveal_with_fuel(policy::removed, 4);
        assert(members@.take(1).drop_last() =~= Seq::<String>::empty());
        assert(members@.take(1).last() == members@[0]);
        assert(members@.take(2).drop_last() =~= members@.take(1));
        assert(members@.drop_last() =~= members@.take(2));
        assert("kept"@ != "retired"@ && "last"@ != "retired"@);
        assert(policy::removed(Seq::<String>::empty(), "retired"@) =~= Seq::<String>::empty());
        assert(policy::removed(members@.take(1), "retired"@)
            == policy::removed(members@.take(1).drop_last(), "retired"@).push(members@.take(1).last()));
        assert(policy::removed(members@.take(1), "retired"@) =~= seq![members@[0]]);
        assert(policy::removed(members@.take(2), "retired"@) =~= seq![members@[0]]);
        assert(remaining@ =~= seq![members@[0], members@[2]]);
    }
    assert(remaining.len() == 2);
    assert(remaining[0]@ == "kept"@ && remaining[1]@ == "last"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_requires_closed_users_and_keeps_original_size_boundary() {
    broadcast use vstd::array::group_array_axioms;
    proof {
        reveal_strlit("recovery"); reveal_strlit("preserved"); reveal_strlit("");
        vstd::string::is_ascii_spec_bytes("recovery");
    }
    let roles = vec!["recovery".to_owned()];
    proof {
        assert(roles[0]@ == "recovery"@);
        assert forall|i: int| 0 <= i < "recovery"@.len()
            implies ("recovery"@[i] as u32) < 128 by {
            assert(i == 0 || i == 1 || i == 2 || i == 3
                || i == 4 || i == 5 || i == 6 || i == 7);
        }
        assert(vstd::string::is_ascii("recovery"));
        vstd::string::is_ascii_spec_bytes("recovery");
        assert(vstd::utf8::encode_utf8("recovery"@).len() == 8);
        assert(policy::bounded_roles(roles@));
    }
    let accepted = policy::valid_request(&roles, 1, "preserved", 4096, true, 1024, 1024);
    let live = policy::valid_request(&roles, 1, "preserved", 4096, false, 1024, 1024);
    let empty = policy::valid_request(&roles, 1, "", 0, true, 1024, 1024);
    let reason = policy::valid_request(&roles, 1, "preserved", 4097, true, 1024, 1024);
    let size = policy::valid_request(&roles, 1, "preserved", 4096, true, 1025, 1024);
    let bad_roles = vec!["".to_owned()];
    let bad = policy::valid_request(&bad_roles, 1, "preserved", 9, true, 0, 0);
    assert(accepted); assert(!live); assert(!empty); assert(!reason); assert(!size); assert(!bad);
    let global = policy::global_maintenance(None);
    let selected = vec!["output-a".to_owned()];
    let scoped = policy::global_maintenance(Some(&selected));
    assert(global && !scoped);
}

}
