//! 实际 CI／协议状态的独立比较、退出码及 finding 边界预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_status.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_status as policy;
use core::cmp::Ordering;
use policy::{Status, WireStatus};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

fn expect(a: &Status, b: &Status, expected: bool)
    requires expected == (*a == *b),
{
    let observed = a.eq(b);
    assert(observed == expected);
    let cloned = a.clone();
    assert(cloned == *a);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_status_pairs_preserve_variant_identity_and_cloning() {
    expect(&Status::Passed, &Status::Passed, true);
    expect(&Status::Passed, &Status::Failed, false);
    expect(&Status::Passed, &Status::NotRequired, false);
    expect(&Status::Passed, &Status::Blocked, false);
    expect(&Status::Failed, &Status::Passed, false);
    expect(&Status::Failed, &Status::Failed, true);
    expect(&Status::Failed, &Status::NotRequired, false);
    expect(&Status::Failed, &Status::Blocked, false);
    expect(&Status::NotRequired, &Status::Passed, false);
    expect(&Status::NotRequired, &Status::Failed, false);
    expect(&Status::NotRequired, &Status::NotRequired, true);
    expect(&Status::NotRequired, &Status::Blocked, false);
    expect(&Status::Blocked, &Status::Passed, false);
    expect(&Status::Blocked, &Status::Failed, false);
    expect(&Status::Blocked, &Status::NotRequired, false);
    expect(&Status::Blocked, &Status::Blocked, true);
}

fn protocol_pair(a: &WireStatus, b: &WireStatus, expected: Ordering)
    requires expected == a.order_spec(b),
        (expected == Ordering::Equal) == (*a == *b),
{
    let cmp = a.cmp(b);
    let partial = a.partial_cmp(b);
    let equal = a.eq(b);
    let cloned = a.clone();
    assert(cmp == expected);
    assert(partial == Some(expected));
    assert(equal == (expected == Ordering::Equal));
    assert(cloned == *a);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn protocol_state_order_keeps_all_sixteen_pairs_and_exact_exits() {
    protocol_pair(&WireStatus::Pass, &WireStatus::Pass, Ordering::Equal);
    protocol_pair(&WireStatus::Pass, &WireStatus::Warn, Ordering::Less);
    protocol_pair(&WireStatus::Pass, &WireStatus::Fail, Ordering::Less);
    protocol_pair(&WireStatus::Pass, &WireStatus::Error, Ordering::Less);
    protocol_pair(&WireStatus::Warn, &WireStatus::Pass, Ordering::Greater);
    protocol_pair(&WireStatus::Warn, &WireStatus::Warn, Ordering::Equal);
    protocol_pair(&WireStatus::Warn, &WireStatus::Fail, Ordering::Less);
    protocol_pair(&WireStatus::Warn, &WireStatus::Error, Ordering::Less);
    protocol_pair(&WireStatus::Fail, &WireStatus::Pass, Ordering::Greater);
    protocol_pair(&WireStatus::Fail, &WireStatus::Warn, Ordering::Greater);
    protocol_pair(&WireStatus::Fail, &WireStatus::Fail, Ordering::Equal);
    protocol_pair(&WireStatus::Fail, &WireStatus::Error, Ordering::Less);
    protocol_pair(&WireStatus::Error, &WireStatus::Pass, Ordering::Greater);
    protocol_pair(&WireStatus::Error, &WireStatus::Warn, Ordering::Greater);
    protocol_pair(&WireStatus::Error, &WireStatus::Fail, Ordering::Greater);
    protocol_pair(&WireStatus::Error, &WireStatus::Error, Ordering::Equal);
    let pass = WireStatus::Pass.exit_code();
    let warn = WireStatus::Warn.exit_code();
    let fail = WireStatus::Fail.exit_code();
    let error = WireStatus::Error.exit_code();
    assert(pass == 0 && warn == 0 && fail == 1 && error == 2);
}

// 任意完整文本和任意 i32 退出码；预期直接列出协议四态的数字合同。
fn header_exits(protocol: &str, id: &str, judge: &str, exit: i32) {
    let pass = policy::wire_header_matches(
        protocol, id, judge, &WireStatus::Pass, exit, protocol, id, judge);
    let warn = policy::wire_header_matches(
        protocol, id, judge, &WireStatus::Warn, exit, protocol, id, judge);
    let fail = policy::wire_header_matches(
        protocol, id, judge, &WireStatus::Fail, exit, protocol, id, judge);
    let error = policy::wire_header_matches(
        protocol, id, judge, &WireStatus::Error, exit, protocol, id, judge);
    assert(pass == (exit == 0) && warn == (exit == 0)
        && fail == (exit == 1) && error == (exit == 2));
}

fn mismatched_headers(a: &str, b: &str)
    requires a@ != b@,
{
    let protocol = policy::wire_header_matches(
        b, a, a, &WireStatus::Pass, 0, a, a, a);
    let request = policy::wire_header_matches(
        a, b, a, &WireStatus::Pass, 0, a, a, a);
    let judge = policy::wire_header_matches(
        a, a, b, &WireStatus::Pass, 0, a, a, a);
    assert(!protocol && !request && !judge);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn response_header_keeps_empty_full_text_and_integer_boundaries() {
    header_exits("", "", "", 0);
    header_exits("chrono-judge/v1", "请求\0", "é", 0);
    header_exits("chrono-judge/v1", "请求\0", "é", 1);
    header_exits("chrono-judge/v1", "请求\0", "é", 2);
    header_exits("chrono-judge/v1", "请求\0", "é", -1);
    header_exits("chrono-judge/v1", "请求\0", "é", i32::MIN);
    header_exits("chrono-judge/v1", "请求\0", "é", i32::MAX);
    proof {
        reveal_strlit("request"); reveal_strlit("request\0");
        reveal_strlit("é"); reveal_strlit("e\u{301}");
        assert("request"@.len() == 7 && "request\0"@.len() == 8);
        assert("é"@.len() == 1 && "e\u{301}"@.len() == 2);
    }
    mismatched_headers("request", "request\0");
    mismatched_headers("é", "e\u{301}");
}

fn finding(level: &str) -> (out: policy::Finding)
    ensures out.level@ == level@,
{
    policy::Finding { code: "fixture".to_owned(), level: level.to_owned(),
        message: "independent expectation".to_owned(), delta_refs: Vec::new(), causes: Vec::new() }
}

fn expected_statuses(findings: &[policy::Finding], pass: bool, warn: bool, fail: bool, error: bool)
    requires
        pass == policy::finding_status_agreement(&WireStatus::Pass, findings@),
        warn == policy::finding_status_agreement(&WireStatus::Warn, findings@),
        fail == policy::finding_status_agreement(&WireStatus::Fail, findings@),
        error == policy::finding_status_agreement(&WireStatus::Error, findings@),
{
    let observed_pass = policy::findings_match_status(&WireStatus::Pass, findings);
    let observed_warn = policy::findings_match_status(&WireStatus::Warn, findings);
    let observed_fail = policy::findings_match_status(&WireStatus::Fail, findings);
    let observed_error = policy::findings_match_status(&WireStatus::Error, findings);
    assert(observed_pass == pass && observed_warn == warn
        && observed_fail == fail && observed_error == error);
}

proof fn level_projection(rows: Seq<policy::Finding>, levels: Seq<Seq<char>>, level: Seq<char>)
    requires rows.len() == levels.len(),
        forall|i: int| 0 <= i < rows.len() ==> (#[trigger] rows[i]).level@ == levels[i],
    ensures policy::finding_level_present(rows, level) == levels.contains(level),
{
    if policy::finding_level_present(rows, level) {
        let i = choose|i: int| 0 <= i < rows.len() && (#[trigger] rows[i]).level@ == level;
        assert(levels[i] == level);
        assert(levels.contains(level));
    }
    if levels.contains(level) {
        let i = choose|i: int| 0 <= i < levels.len() && #[trigger] levels[i] == level;
        assert(rows[i].level@ == level);
        assert(policy::finding_level_present(rows, level));
    }
}

proof fn known_levels(rows: Seq<policy::Finding>, levels: Seq<Seq<char>>)
    requires rows.len() == levels.len(),
        forall|i: int| 0 <= i < rows.len() ==> (#[trigger] rows[i]).level@ == levels[i],
    ensures
        policy::finding_level_present(rows, "error"@) == levels.contains("error"@),
        policy::finding_level_present(rows, "warning"@) == levels.contains("warning"@),
{
    level_projection(rows, levels, "error"@);
    level_projection(rows, levels, "warning"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn protocol_findings_cover_the_complete_presence_truth_table() {
    proof {
        reveal_strlit("info"); reveal_strlit("warning"); reveal_strlit("error");
        assert("info"@.len() == 4 && "warning"@.len() == 7 && "error"@.len() == 5);
    }
    let empty: Vec<policy::Finding> = Vec::new();
    expected_statuses(&empty, true, false, false, false);
    let only_info = vec![finding("info"), finding("info")];
    proof {
        known_levels(only_info@, seq!["info"@, "info"@]);
        assert(!policy::finding_level_present(only_info@, "error"@));
        assert(!policy::finding_level_present(only_info@, "warning"@));
        assert(policy::finding_status_agreement(&WireStatus::Pass, only_info@));
    }
    expected_statuses(&only_info, true, false, false, false);
    let warning = vec![finding("info"), finding("warning"), finding("warning")];
    proof { known_levels(warning@, seq!["info"@, "warning"@, "warning"@]); }
    expected_statuses(&warning, false, true, false, false);
    let error = vec![finding("error"), finding("info"), finding("error")];
    proof { known_levels(error@, seq!["error"@, "info"@, "error"@]); }
    expected_statuses(&error, false, false, true, true);
    let both = vec![finding("warning"), finding("error"), finding("info")];
    proof { known_levels(both@, seq!["warning"@, "error"@, "info"@]); }
    expected_statuses(&both, false, false, true, true);
    let reversed = vec![finding("error"), finding("warning")];
    proof { known_levels(reversed@, seq!["error"@, "warning"@]); }
    expected_statuses(&reversed, false, false, true, true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn protocol_level_matching_uses_the_entire_real_field() {
    proof {
        reveal_strlit("info"); reveal_strlit("warning"); reveal_strlit("error");
        reveal_strlit("ERROR"); reveal_strlit("warning\0"); reveal_strlit("警告"); reveal_strlit("");
        assert("info"@.len() == 4 && "warning"@.len() == 7 && "error"@.len() == 5);
        assert("ERROR"@.len() == 5 && "warning\0"@.len() == 8
            && "警告"@.len() == 2 && ""@.len() == 0);
        assert("ERROR"@[0] != "error"@[0]);
    }
    let irrelevant = vec![finding("ERROR"), finding("warning\0"), finding("警告"), finding("")];
    proof { known_levels(irrelevant@, seq!["ERROR"@, "warning\0"@, "警告"@, ""@]); }
    expected_statuses(&irrelevant, true, false, false, false);
    let misleading_fields = policy::Finding {
        code: "error".to_owned(), level: "info".to_owned(), message: "warning".to_owned(),
        delta_refs: vec!["error".to_owned()], causes: vec!["warning".to_owned()],
    };
    let rows = vec![misleading_fields];
    proof { known_levels(rows@, seq!["info"@]); }
    expected_statuses(&rows, true, false, false, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn actual_finding_clone_keeps_complete_fields_and_reference_order() {
    let original = policy::Finding {
        code: "根\0".to_owned(), level: "warning".to_owned(), message: "é\0".to_owned(),
        delta_refs: vec!["/根\0".to_owned(), "".to_owned(), "/根\0".to_owned()],
        causes: vec!["z".to_owned(), "原因\0".to_owned()],
    };
    let copied = original.clone();
    assert(copied.code@ == "根\0"@ && copied.level@ == "warning"@ && copied.message@ == "é\0"@);
    assert(copied.delta_refs.len() == 3 && copied.causes.len() == 2);
    assert(copied.delta_refs@[0]@ == "/根\0"@ && copied.delta_refs@[1]@ == ""@
        && copied.delta_refs@[2]@ == "/根\0"@);
    assert(copied.causes@[0]@ == "z"@ && copied.causes@[1]@ == "原因\0"@);
}

}
