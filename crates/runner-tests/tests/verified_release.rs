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
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect_true(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn runtime_expectations_keep_their_failure_branch() {
    expect_true(false);
}

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
    expect_true(!e); expect_true(a); expect_true(!z); expect_true(!d); expect_true(b); expect_true(!over);
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
    expect_true(complete); expect_true(!short); expect_true(!extra);
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
    expect_true(remaining.len() == 2);
    assert(remaining[0]@ == "kept"@ && remaining[1]@ == "last"@);
    let kept = policy::contains(&remaining,"kept");
    let last = policy::contains(&remaining,"last");
    expect_true(kept && last);
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
    expect_true(accepted); expect_true(!live); expect_true(!empty); expect_true(!reason); expect_true(!size); expect_true(!bad);
    let global = policy::global_maintenance(None);
    let selected = vec!["output-a".to_owned()];
    let scoped = policy::global_maintenance(Some(&selected));
    expect_true(global && !scoped);
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn object_order_and_returned_command_have_independent_boundaries() {
    let reserve = policy::observation_owner(false,true,true);
    let ordinary = policy::observation_owner(false,false,true);
    let unsealed = policy::observation_result(true,false,true,true,true);
    let replaced = policy::observation_result(true,true,true,true,false);
    let live = policy::settled_pr_command(true,false,true);
    let create = policy::settled_pr_command(false,true,true);
    let storage = policy::settled_pr_command(false,false,false);
    let failed_read = policy::settled_pr_command(false,false,true);
    expect_true(reserve && !ordinary && !unsealed && !replaced);
    expect_true(!live && !create && !storage && failed_read);
    let first = policy::next_observation(0);
    let last = policy::next_observation(u64::MAX);
    expect_true(first.is_some()); expect_true(first.unwrap() == 1); expect_true(last.is_none());
    let pending_wrong_epoch = policy::observation_epoch(true,false,1,3);
    let completed_new_epoch = policy::observation_epoch(false,false,9,1);
    expect_true(!pending_wrong_epoch && completed_new_epoch);
    let intact = policy::observation_order(64,3,2,true,true);
    let future = policy::observation_order(64,3,4,true,true);
    let absent = policy::observation_order(64,3,2,true,false);
    expect_true(intact && !future && !absent);
    let ordinary_edit = policy::observation_edit(true,false,false);
    let owner_edit = policy::observation_edit(true,true,false);
    expect_true(!ordinary_edit && owner_edit);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn actual_pr_facts_do_not_certify_a_failed_or_unverified_command() {
    proof { reveal_strlit("OPEN"); reveal_strlit("CLOSED"); reveal_strlit("MERGED"); reveal_strlit("unknown"); reveal_strlit("pr"); reveal_strlit("create"); reveal_strlit("checks"); }
    proof {
        assert("OPEN"@[0] != "CLOSED"@[0] && "OPEN"@[0] != "MERGED"@[0]);
        assert("unknown"@[0] != "OPEN"@[0] && "unknown"@[0] != "CLOSED"@[0] && "unknown"@[0] != "MERGED"@[0]);
        assert("create"@[1] != "checks"@[1]);
    }
    let open = policy::open_pr("OPEN");
    let closed = policy::observed_pr("CLOSED",false);
    let merged = policy::observed_pr("MERGED",false);
    let wrong = policy::observed_pr("unknown",false);
    let contradictory = policy::observed_pr("OPEN",false);
    expect_true(open && closed && merged && !wrong && !contradictory);
    let create = policy::creating_pr("pr","create");
    let query = policy::creating_pr("pr","checks");
    expect_true(create && !query);
    let failed = policy::continued_pr_handoff(true,false,true,true);
    let stale = policy::continued_pr_handoff(true,true,true,false);
    let passed = policy::continued_pr_handoff(true,true,true,true);
    expect_true(!failed && !stale && passed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn older_observation_preserves_current_and_every_unrelated_pending_member() {
    broadcast use vstd::array::group_array_axioms;
    proof { reveal_strlit("unknown"); reveal_strlit("current"); reveal_strlit("request"); }
    let members = vec!["unknown".to_owned(),"current".to_owned(),"request".to_owned()];
    let id = "request".to_owned();
    let current = Some("current".to_owned());
    let old = policy::settle_observation_members(&members,&id,&current,9,Some((8,true)));
    let terminal = policy::settle_observation_members(&members,&id,&current,9,Some((10,false)));
    let active = policy::settle_observation_members(&members,&id,&current,9,Some((10,true)));
    proof {
        reveal_with_fuel(policy::removed,5);
        assert(members[0]@ == "unknown"@ && members[1]@ == "current"@ && members[2]@ == "request"@);
        assert("unknown"@[0] != "request"@[0] && "current"@[0] != "request"@[0]);
        assert("unknown"@[0] != "current"@[0] && "request"@[0] != "current"@[0]);
    }
    assert(old.accepted == 9 && policy::current_value(old.current) == Some("current"@));
    assert(terminal.accepted == 10 && terminal.current.is_none());
    assert(active.accepted == 10 && policy::current_value(active.current) == Some("request"@));
    expect_true(old.accepted == 9 && old.current.is_some());
    expect_true(terminal.accepted == 10 && terminal.current.is_none());
    expect_true(active.accepted == 10 && active.current.is_some());
    proof {
        assert(old.members@ =~= seq![members[0], members[1]]);
        assert(terminal.members@ =~= seq![members[0]]);
        assert(active.members@ =~= seq![members[0], id]);
    }
    let old_unknown = policy::contains(&old.members,"unknown");
    let old_current = policy::contains(&old.members,"current");
    let old_request = policy::contains(&old.members,"request");
    let closed_unknown = policy::contains(&terminal.members,"unknown");
    let closed_current = policy::contains(&terminal.members,"current");
    let active_request = policy::contains(&active.members,"request");
    expect_true(old_unknown && old_current && !old_request);
    expect_true(closed_unknown && !closed_current && active_request);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observation_binding_requires_the_actual_role_output_epoch_and_nonzero_attempt() {
    proof {
        reveal_strlit("role"); reveal_strlit("else"); reveal_strlit("");
        assert("role"@[0] != "else"@[0]); assert("role"@.len() == 4 && ""@.len() == 0);
    }
    let request = policy::ObservationBinding { role:"role", epoch:"role", id:"role",
        path:"role", physical:"role", serial:9, attempt:3 };
    let correct = policy::ObservationBinding { role:"role", epoch:"role", id:"role",
        path:"role", physical:"role", serial:8, attempt:2 };
    let other_role = policy::ObservationBinding { role:"else", ..correct };
    let other_epoch = policy::ObservationBinding { epoch:"else", ..correct };
    let other_id = policy::ObservationBinding { id:"else", ..correct };
    let other_path = policy::ObservationBinding { path:"else", ..correct };
    let replaced = policy::ObservationBinding { physical:"else", ..correct };
    let future = policy::ObservationBinding { serial:10, ..correct };
    let zero = policy::ObservationBinding { attempt:0, ..correct };
    let match_ok = policy::matching_observation(&request,&correct);
    let role = policy::matching_observation(&request,&other_role);
    let epoch = policy::matching_observation(&request,&other_epoch);
    let id = policy::matching_observation(&request,&other_id);
    let path = policy::matching_observation(&request,&other_path);
    let physical = policy::matching_observation(&request,&replaced);
    let newer = policy::matching_observation(&request,&future);
    let invalid_attempt = policy::matching_observation(&request,&zero);
    expect_true(match_ok && !role && !epoch && !id && !path && !physical && !newer && !invalid_attempt);
    let maximum = policy::observation_role("role",256,1);
    let too_long = policy::observation_role("role",257,1);
    let blank = policy::observation_role("",0,1);
    let no_attempt = policy::observation_token("role",4,1,0);
    let no_serial = policy::observation_token("role",4,0,1);
    expect_true(maximum && !too_long && !blank && !no_attempt && !no_serial);
    let continued = policy::continued_observation(&correct,"role","role");
    let rebound = policy::continued_observation(&correct,"role","else");
    expect_true(continued && !rebound);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn replay_or_missing_observation_keeps_the_current_original() {
    broadcast use vstd::array::group_array_axioms;
    proof { reveal_strlit("current"); reveal_strlit("unknown"); assert("current"@[0] != "unknown"@[0]); }
    let id = "current".to_owned();
    let members = vec!["unknown".to_owned(),id.clone()];
    let current = Some(id.clone());
    let replay = policy::settle_observation_members(&members,&id,&current,9,Some((9,false)));
    let failed = policy::settle_observation_members(&members,&id,&current,9,None);
    assert(replay.members@ == members@ && failed.members@ == members@);
    expect_true(replay.accepted == 9 && failed.accepted == 9);
    assert(policy::current_value(replay.current) == Some(id@));
    assert(policy::current_value(failed.current) == Some(id@));
    let replay_unknown = policy::contains(&replay.members,"unknown");
    let replay_current = policy::contains(&replay.members,"current");
    let failed_unknown = policy::contains(&failed.members,"unknown");
    let failed_current = policy::contains(&failed.members,"current");
    expect_true(replay.members.len() == 2 && replay_unknown && replay_current);
    expect_true(failed.members.len() == 2 && failed_unknown && failed_current);
    expect_true(replay.current.is_some() && failed.current.is_some());
    let producer = policy::observation_producer(true,true,true);
    let unreturned = policy::unknown_pr_operation(true,false);
    let unverified_create = policy::unknown_pr_operation(false,true);
    let complete = policy::unknown_pr_operation(false,false);
    expect_true(producer && unreturned && unverified_create && !complete);
    let valid_state = policy::object_state("chrono-pr-object-state/v1",false);
    let unknown_write = policy::object_state("chrono-pr-object-state/v1",true);
    expect_true(valid_state && !unknown_write);
}

}
