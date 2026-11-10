//! 实际 CI／协议状态的独立比较、退出码及 finding 边界预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_status.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as scope_text;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_status as policy;
use core::cmp::Ordering;
#[cfg(verus_keep_ghost)]
use policy::identifiers as scope_text;
use policy::{Status, WireStatus};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::cmp::OrdSpec;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

// 这些分支在普通 Rust 测试中也执行；证明不能替代真实调用的断言。
fn expect_status_true(actual: bool)
    requires actual,
{
    if !actual {
        panic!();
    }
}

fn expect_status_text(actual: &str, expected: &str)
    requires actual@ == expected@,
{
    let equal = scope_text::equal_text(actual, expected);
    expect_status_true(equal);
}

fn expect_optional_text(actual: &Option<String>, expected: &Option<String>)
    requires policy::optional_text(actual) == policy::optional_text(expected),
{
    match (actual, expected) {
        (Some(actual), Some(expected)) => {
            expect_status_text(actual.as_str(), expected.as_str());
        },
        (None, None) => {},
        _ => { panic!(); },
    }
}

fn expect_some_text(actual: &Option<String>, expected: &str)
    requires policy::optional_text(actual) == Some(expected@),
{
    match actual {
        Some(actual) => { expect_status_text(actual.as_str(), expected); },
        None => { panic!(); },
    }
}

fn expect_order(actual: Ordering, expected: Ordering)
    requires actual == expected,
{
    let equal = match (actual, expected) {
        (Ordering::Less, Ordering::Less) => true,
        (Ordering::Equal, Ordering::Equal) => true,
        (Ordering::Greater, Ordering::Greater) => true,
        _ => false,
    };
    expect_status_true(equal);
}

fn scope_preserves_full_text(text: &str) {
    let unit = policy::scopes::Scope::Unit { unit: text.to_owned() };
    let collect = policy::scopes::Scope::Collect { manifest: text.to_owned() };
    let cloned_unit = unit.clone();
    let cloned_collect = collect.clone();
    expect_status_true(cloned_unit.eq(&unit));
    expect_status_true(cloned_collect.eq(&collect));
    expect_status_true(!unit.eq(&collect));
    expect_status_true(!collect.eq(&unit));

    let short_unit = unit.short_argv();
    let full_unit = unit.argv();
    let short_collect = collect.short_argv();
    let full_collect = collect.argv();
    expect_status_true(short_unit.len() == 2);
    expect_status_true(scope_text::equal_text(short_unit[0].as_str(), "--unit"));
    expect_status_true(scope_text::equal_text(short_unit[1].as_str(), text));
    expect_status_true(scope_text::equal_text(full_unit[0].as_str(), "--unit"));
    expect_status_true(scope_text::equal_text(full_unit[1].as_str(), text));
    expect_status_true(short_collect.len() == 1);
    expect_status_true(scope_text::equal_text(short_collect[0].as_str(), "--collect"));
    expect_status_true(scope_text::equal_text(full_collect[0].as_str(), "--collect"));
    expect_status_true(scope_text::equal_text(full_collect[1].as_str(), text));
}

fn distinct_scope_texts(first: &str, second: &str)
    requires first@ != second@,
{
    let a = policy::scopes::Scope::Unit { unit: first.to_owned() };
    let b = policy::scopes::Scope::Unit { unit: second.to_owned() };
    let c = policy::scopes::Scope::Collect { manifest: first.to_owned() };
    let d = policy::scopes::Scope::Collect { manifest: second.to_owned() };
    expect_status_true(!a.eq(&b));
    expect_status_true(!b.eq(&a));
    expect_status_true(!c.eq(&d));
    expect_status_true(!d.eq(&c));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn scope_arguments_and_cloning_keep_complete_text_and_variant() {
    scope_preserves_full_text("");
    scope_preserves_full_text("runner");
    scope_preserves_full_text(".chrono-harness/state/with spaces/清单.json");
    scope_preserves_full_text("--collect\0\n\r");
    scope_preserves_full_text("é");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn scope_equality_rejects_prefix_case_and_unicode_normalization() {
    proof {
        reveal_strlit("unit"); reveal_strlit("unit\0");
        reveal_strlit("é"); reveal_strlit("e\u{301}");
        reveal_strlit("A"); reveal_strlit("a");
        assert("unit"@.len() == 4 && "unit\0"@.len() == 5);
        assert("é"@.len() == 1 && "e\u{301}"@.len() == 2);
        assert("A"@[0] != "a"@[0]);
    }
    distinct_scope_texts("unit", "unit\0");
    distinct_scope_texts("é", "e\u{301}");
    distinct_scope_texts("A", "a");
}

fn expect(a: &Status, b: &Status, expected: bool)
    requires expected == (*a == *b),
{
    let observed = a.eq(b);
    assert(observed == expected);
    expect_status_true(observed == expected);
    let cloned = a.clone();
    assert(cloned == *a);
    expect_status_true(cloned.eq(a));
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

#[cfg_attr(not(verus_keep_ghost), test)]
fn severity_maximum_keeps_all_sixteen_expected_results() {
    let pp = WireStatus::Pass.maximum(WireStatus::Pass);
    let pw = WireStatus::Pass.maximum(WireStatus::Warn);
    let pf = WireStatus::Pass.maximum(WireStatus::Fail);
    let pe = WireStatus::Pass.maximum(WireStatus::Error);
    assert(pp == WireStatus::Pass && pw == WireStatus::Warn
        && pf == WireStatus::Fail && pe == WireStatus::Error);
    expect_status_true(pp.eq(&WireStatus::Pass) && pw.eq(&WireStatus::Warn)
        && pf.eq(&WireStatus::Fail) && pe.eq(&WireStatus::Error));
    let wp = WireStatus::Warn.maximum(WireStatus::Pass);
    let ww = WireStatus::Warn.maximum(WireStatus::Warn);
    let wf = WireStatus::Warn.maximum(WireStatus::Fail);
    let we = WireStatus::Warn.maximum(WireStatus::Error);
    assert(wp == WireStatus::Warn && ww == WireStatus::Warn
        && wf == WireStatus::Fail && we == WireStatus::Error);
    expect_status_true(wp.eq(&WireStatus::Warn) && ww.eq(&WireStatus::Warn)
        && wf.eq(&WireStatus::Fail) && we.eq(&WireStatus::Error));
    let fp = WireStatus::Fail.maximum(WireStatus::Pass);
    let fw = WireStatus::Fail.maximum(WireStatus::Warn);
    let ff = WireStatus::Fail.maximum(WireStatus::Fail);
    let fe = WireStatus::Fail.maximum(WireStatus::Error);
    assert(fp == WireStatus::Fail && fw == WireStatus::Fail
        && ff == WireStatus::Fail && fe == WireStatus::Error);
    expect_status_true(fp.eq(&WireStatus::Fail) && fw.eq(&WireStatus::Fail)
        && ff.eq(&WireStatus::Fail) && fe.eq(&WireStatus::Error));
    let ep = WireStatus::Error.maximum(WireStatus::Pass);
    let ew = WireStatus::Error.maximum(WireStatus::Warn);
    let ef = WireStatus::Error.maximum(WireStatus::Fail);
    let ee = WireStatus::Error.maximum(WireStatus::Error);
    assert(ep == WireStatus::Error && ew == WireStatus::Error
        && ef == WireStatus::Error && ee == WireStatus::Error);
    expect_status_true(ep.eq(&WireStatus::Error) && ew.eq(&WireStatus::Error)
        && ef.eq(&WireStatus::Error) && ee.eq(&WireStatus::Error));
}

fn severity_maximum_keeps_idempotence_commutativity_and_associativity(
    a: WireStatus, b: WireStatus, c: WireStatus,
) {
    let aa = a.clone().maximum(a.clone());
    assert(aa == a);
    expect_status_true(aa.eq(&a));
    let ab = a.clone().maximum(b.clone());
    let ba = b.clone().maximum(a.clone());
    assert(ab == ba);
    expect_status_true(ab.eq(&ba));
    let left = ab.maximum(c.clone());
    let bc = b.maximum(c);
    let right = a.maximum(bc);
    assert(left == right);
    expect_status_true(left.eq(&right));
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
    expect_order(cmp, expected);
    assert(partial == Some(expected));
    match partial {
        Some(value) => { expect_order(value, expected); },
        None => { panic!(); },
    }
    assert(equal == (expected == Ordering::Equal));
    let expected_equal = match expected { Ordering::Equal => true, _ => false };
    expect_status_true(equal == expected_equal);
    assert(cloned == *a);
    expect_status_true(cloned.eq(a));
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
    expect_status_true(pass == 0 && warn == 0 && fail == 1 && error == 2);
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
    expect_status_true(pass == (exit == 0) && warn == (exit == 0)
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
    expect_status_true(!protocol && !request && !judge);
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
    ensures out.level@ == level@, out.code@ == "fixture"@,
        out.message@ == "independent expectation"@,
        out.delta_refs@ == Seq::<String>::empty(), out.causes@ == Seq::<String>::empty(),
{
    policy::Finding { code: "fixture".to_owned(), level: level.to_owned(),
        message: "independent expectation".to_owned(), delta_refs: Vec::new(), causes: Vec::new() }
}

fn append_case(mut status: WireStatus, severity: WireStatus, expected: WireStatus)
    requires expected == (if status.rank_spec() >= severity.rank_spec() { status } else { severity }),
{
    let mut rows: Vec<policy::Finding> = Vec::new();
    policy::append_finding(&mut status, &mut rows, severity, finding("error"));
    assert(status == expected);
    expect_status_true(status.eq(&expected));
    assert(rows.len() == 1 && rows@[0].level@ == "error"@);
    expect_status_true(rows.len() == 1);
    expect_status_text(rows[0].level.as_str(), "error");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recording_finding_keeps_the_highest_of_all_sixteen_status_pairs() {
    append_case(WireStatus::Pass, WireStatus::Pass, WireStatus::Pass);
    append_case(WireStatus::Pass, WireStatus::Warn, WireStatus::Warn);
    append_case(WireStatus::Pass, WireStatus::Fail, WireStatus::Fail);
    append_case(WireStatus::Pass, WireStatus::Error, WireStatus::Error);
    append_case(WireStatus::Warn, WireStatus::Pass, WireStatus::Warn);
    append_case(WireStatus::Warn, WireStatus::Warn, WireStatus::Warn);
    append_case(WireStatus::Warn, WireStatus::Fail, WireStatus::Fail);
    append_case(WireStatus::Warn, WireStatus::Error, WireStatus::Error);
    append_case(WireStatus::Fail, WireStatus::Pass, WireStatus::Fail);
    append_case(WireStatus::Fail, WireStatus::Warn, WireStatus::Fail);
    append_case(WireStatus::Fail, WireStatus::Fail, WireStatus::Fail);
    append_case(WireStatus::Fail, WireStatus::Error, WireStatus::Error);
    append_case(WireStatus::Error, WireStatus::Pass, WireStatus::Error);
    append_case(WireStatus::Error, WireStatus::Warn, WireStatus::Error);
    append_case(WireStatus::Error, WireStatus::Fail, WireStatus::Error);
    append_case(WireStatus::Error, WireStatus::Error, WireStatus::Error);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn recording_findings_keeps_history_duplicates_and_complete_fields() {
    let first = finding("info");
    let next = policy::Finding {
        code: "根\0".to_owned(), level: "warning".to_owned(), message: "é\0".to_owned(),
        delta_refs: vec!["/根\0".to_owned(), "".to_owned(), "/根\0".to_owned()],
        causes: vec!["z".to_owned(), "原因\0".to_owned()],
    };
    let mut rows = vec![first, next.clone()];
    let mut status = WireStatus::Error;
    policy::append_finding(&mut status, &mut rows, WireStatus::Warn, next);
    assert(status == WireStatus::Error && rows.len() == 3);
    expect_status_true(status.eq(&WireStatus::Error) && rows.len() == 3);
    assert(rows@[0].level@ == "info"@);
    expect_status_text(rows[0].level.as_str(), "info");
    assert(rows@[1].code@ == rows@[2].code@ && rows@[2].code@ == "根\0"@);
    expect_status_text(rows[1].code.as_str(), "根\0");
    expect_status_text(rows[2].code.as_str(), "根\0");
    assert(rows@[1].level@ == rows@[2].level@ && rows@[2].level@ == "warning"@);
    expect_status_text(rows[1].level.as_str(), "warning");
    expect_status_text(rows[2].level.as_str(), "warning");
    assert(rows@[1].message@ == rows@[2].message@ && rows@[2].message@ == "é\0"@);
    expect_status_text(rows[1].message.as_str(), "é\0");
    expect_status_text(rows[2].message.as_str(), "é\0");
    assert(rows@[1].delta_refs@ == rows@[2].delta_refs@);
    assert(rows@[2].delta_refs.len() == 3);
    expect_status_true(rows[1].delta_refs.len() == 3 && rows[2].delta_refs.len() == 3);
    assert(rows@[2].delta_refs@[0]@ == "/根\0"@ && rows@[2].delta_refs@[1]@ == ""@
        && rows@[2].delta_refs@[2]@ == "/根\0"@);
    expect_status_true(rows[1].delta_refs.len() == 3 && rows[2].delta_refs.len() == 3);
    expect_status_text(rows[1].delta_refs[0].as_str(), "/根\0");
    expect_status_text(rows[1].delta_refs[1].as_str(), "");
    expect_status_text(rows[1].delta_refs[2].as_str(), "/根\0");
    expect_status_text(rows[2].delta_refs[0].as_str(), "/根\0");
    expect_status_text(rows[2].delta_refs[1].as_str(), "");
    expect_status_text(rows[2].delta_refs[2].as_str(), "/根\0");
    assert(rows@[1].causes@ == rows@[2].causes@ && rows@[2].causes.len() == 2);
    assert(rows@[2].causes@[0]@ == "z"@ && rows@[2].causes@[1]@ == "原因\0"@);
    expect_status_true(rows[1].causes.len() == 2 && rows[2].causes.len() == 2);
    expect_status_text(rows[1].causes[0].as_str(), "z");
    expect_status_text(rows[1].causes[1].as_str(), "原因\0");
    expect_status_text(rows[2].causes[0].as_str(), "z");
    expect_status_text(rows[2].causes[1].as_str(), "原因\0");
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
    expect_status_true(observed_pass == pass && observed_warn == warn
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
    expect_status_text(copied.code.as_str(), "根\0");
    expect_status_text(copied.level.as_str(), "warning");
    expect_status_text(copied.message.as_str(), "é\0");
    assert(copied.delta_refs.len() == 3 && copied.causes.len() == 2);
    expect_status_true(copied.delta_refs.len() == 3 && copied.causes.len() == 2);
    assert(copied.delta_refs@[0]@ == "/根\0"@ && copied.delta_refs@[1]@ == ""@
        && copied.delta_refs@[2]@ == "/根\0"@);
    assert(copied.causes@[0]@ == "z"@ && copied.causes@[1]@ == "原因\0"@);
    expect_status_text(copied.delta_refs[0].as_str(), "/根\0");
    expect_status_text(copied.delta_refs[1].as_str(), "");
    expect_status_text(copied.delta_refs[2].as_str(), "/根\0");
    expect_status_text(copied.causes[0].as_str(), "z");
    expect_status_text(copied.causes[1].as_str(), "原因\0");
}

fn finding_fields_preserve_nonempty_full_text(code: &str, message: &str, reference: &str)
    requires code@.len() > 0, message@.len() > 0,
{
    let refs = vec![reference.to_owned(), reference.to_owned()];
    let info = policy::Finding { code: code.to_owned(), message: message.to_owned(),
        level: "info".to_owned(), delta_refs: refs, causes: vec!["".to_owned()] };
    let accepted = policy::valid_finding_fields(&info);
    assert(accepted);
    expect_status_true(accepted);
    let warning = policy::Finding { level: "warning".to_owned(), ..info.clone() };
    let accepted = policy::valid_finding_fields(&warning);
    assert(accepted);
    expect_status_true(accepted);
    let error = policy::Finding { level: "error".to_owned(), ..info.clone() };
    let accepted = policy::valid_finding_fields(&error);
    assert(accepted);
    expect_status_true(accepted);
    assert(info.delta_refs.len() == 2 && info.delta_refs@[0]@ == reference@
        && info.delta_refs@[1]@ == reference@);
    expect_status_true(info.delta_refs.len() == 2);
    expect_status_text(info.delta_refs[0].as_str(), reference);
    expect_status_text(info.delta_refs[1].as_str(), reference);
}

fn empty_finding_fields_are_rejected(code: &str, message: &str, level: &str) {
    proof { reveal_strlit(""); }
    let refs = vec!["/records/x".to_owned()];
    let no_code = policy::Finding { code: "".to_owned(), message: message.to_owned(),
        level: level.to_owned(), delta_refs: refs.clone(), causes: Vec::new() };
    let accepted = policy::valid_finding_fields(&no_code);
    assert(!accepted);
    expect_status_true(!accepted);
    let no_message = policy::Finding { code: code.to_owned(), message: "".to_owned(),
        level: level.to_owned(), delta_refs: refs, causes: Vec::new() };
    let accepted = policy::valid_finding_fields(&no_message);
    assert(!accepted);
    expect_status_true(!accepted);
    let no_refs = policy::Finding { code: code.to_owned(), message: message.to_owned(),
        level: level.to_owned(), delta_refs: Vec::new(), causes: Vec::new() };
    let accepted = policy::valid_finding_fields(&no_refs);
    assert(!accepted);
    expect_status_true(!accepted);
}

fn noncanonical_finding_level_is_rejected(level: &str)
    requires level@ != "info"@, level@ != "warning"@, level@ != "error"@,
{
    let finding = policy::Finding { code: "E".to_owned(), message: "m".to_owned(),
        level: level.to_owned(), delta_refs: vec!["/x".to_owned()], causes: Vec::new() };
    let accepted = policy::valid_finding_fields(&finding);
    assert(!accepted);
    expect_status_true(!accepted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn finding_fields_keep_exact_levels_and_independent_empty_boundaries() {
    proof {
        reveal_strlit("E"); reveal_strlit("message"); reveal_strlit("根\0");
        reveal_strlit("é\n"); reveal_strlit(" "); reveal_strlit("\0");
        reveal_strlit(""); reveal_strlit("info"); reveal_strlit("warning"); reveal_strlit("error");
        reveal_strlit("Info"); reveal_strlit("warning "); reveal_strlit("error\0");
        reveal_strlit("information");
        assert(""@.len() == 0 && "info"@.len() == 4 && "warning"@.len() == 7
            && "error"@.len() == 5);
        assert("Info"@.len() == 4 && "Info"@[0] != "info"@[0]);
        assert("warning "@.len() == 8 && "error\0"@.len() == 6 && "information"@.len() == 11);
    }
    finding_fields_preserve_nonempty_full_text("E", "message", "/records/x");
    finding_fields_preserve_nonempty_full_text("根\0", "é\n", "");
    finding_fields_preserve_nonempty_full_text(" ", "\0", "路径\0");
    empty_finding_fields_are_rejected("E", "message", "info");
    empty_finding_fields_are_rejected("", "", "warning");
    noncanonical_finding_level_is_rejected("");
    noncanonical_finding_level_is_rejected("Info");
    noncanonical_finding_level_is_rejected("warning ");
    noncanonical_finding_level_is_rejected("error\0");
    noncanonical_finding_level_is_rejected("information");
}

proof fn zero_evidence_digest_is_valid()
    ensures policy::identifiers::digest_format(vstd::utf8::encode_utf8(
        "0000000000000000000000000000000000000000000000000000000000000000"@)),
{
    reveal_strlit("0000000000000000000000000000000000000000000000000000000000000000");
    assert(vstd::string::is_ascii("0000000000000000000000000000000000000000000000000000000000000000"));
    vstd::string::is_ascii_spec_bytes("0000000000000000000000000000000000000000000000000000000000000000");
    assert("0000000000000000000000000000000000000000000000000000000000000000"@.len() == 64);
}

fn evidence_kind_keeps_arbitrary_nonempty_text(kind: &str)
    requires kind@.len() > 0,
{
    proof {
        zero_evidence_digest_is_valid(); unicode_evidence_prefix_encoding();
        assert(".chrono-harness/state/".spec_bytes().len() == 22);
        assert(".chrono-harness/state/".spec_bytes().take(22) =~= ".chrono-harness/state/".spec_bytes());
        assert(policy::identifiers::digest_format(
            "0000000000000000000000000000000000000000000000000000000000000000".spec_bytes()));
        assert(policy::evidence_identity(".chrono-harness/state/", kind,
            "0000000000000000000000000000000000000000000000000000000000000000"));
    }
    let accepted = policy::valid_evidence_identity(".chrono-harness/state/", kind,
        "0000000000000000000000000000000000000000000000000000000000000000");
    assert(accepted);
    expect_status_true(accepted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn evidence_identity_keeps_exact_prefix_and_nonempty_kind() {
    proof {
        reveal_strlit(".chrono-harness/state/"); reveal_strlit("report"); reveal_strlit("原件\0");
        reveal_strlit(".chrono-harness/state/../x");
        reveal_strlit(""); reveal_strlit(".chrono-harness/state"); reveal_strlit(".chrono-harness/states/x");
        reveal_strlit("/.chrono-harness/state/x"); reveal_strlit(".chrono-harness/State/x");
        reveal_strlit(".chrono-harness/state/x");
        zero_evidence_digest_is_valid(); unicode_evidence_prefix_encoding();
        assert(vstd::string::is_ascii(".chrono-harness/state/"));
        assert(vstd::string::is_ascii(""));
        assert(vstd::string::is_ascii(".chrono-harness/state"));
        assert(vstd::string::is_ascii(".chrono-harness/states/x"));
        assert(vstd::string::is_ascii("/.chrono-harness/state/x"));
        assert(vstd::string::is_ascii(".chrono-harness/State/x"));
        assert(vstd::string::is_ascii(".chrono-harness/state/x"));
        assert(vstd::string::is_ascii(".chrono-harness/state/../x"));
        assert(""@.len() == 0 && "report"@.len() == 6 && "原件\0"@.len() == 3);
        assert(".chrono-harness/state"@.len() == 21 && ".chrono-harness/states/x"@.len() == 24);
        assert("/.chrono-harness/state/x"@.len() == 24 && ".chrono-harness/State/x"@.len() == 23);
        vstd::string::is_ascii_spec_bytes(".chrono-harness/state/");
        vstd::string::is_ascii_spec_bytes("");
        vstd::string::is_ascii_spec_bytes(".chrono-harness/state");
        vstd::string::is_ascii_spec_bytes(".chrono-harness/states/x");
        vstd::string::is_ascii_spec_bytes("/.chrono-harness/state/x");
        vstd::string::is_ascii_spec_bytes(".chrono-harness/State/x");
        vstd::string::is_ascii_spec_bytes(".chrono-harness/state/x");
        vstd::string::is_ascii_spec_bytes(".chrono-harness/state/../x");
        assert(".chrono-harness/state/../x".spec_bytes().take(22)
            =~= ".chrono-harness/state/".spec_bytes());
        assert(".chrono-harness/states/x".spec_bytes()[21] == 115);
        assert(".chrono-harness/state/".spec_bytes()[21] == 47);
        assert("/.chrono-harness/state/x".spec_bytes()[0] == 47
            && ".chrono-harness/state/".spec_bytes()[0] == 46);
        assert(".chrono-harness/State/x".spec_bytes()[16] == 83
            && ".chrono-harness/state/".spec_bytes()[16] == 115);
    }
    let zero = "0000000000000000000000000000000000000000000000000000000000000000";
    evidence_kind_keeps_arbitrary_nonempty_text("report");
    evidence_kind_keeps_arbitrary_nonempty_text("原件\0");
    let accepted = policy::valid_evidence_identity(".chrono-harness/state/../x", "report", zero);
    assert(accepted);
    expect_status_true(accepted);
    let empty = policy::valid_evidence_identity("", "report", zero);
    let short = policy::valid_evidence_identity(".chrono-harness/state", "report", zero);
    let similar = policy::valid_evidence_identity(".chrono-harness/states/x", "report", zero);
    let absolute = policy::valid_evidence_identity("/.chrono-harness/state/x", "report", zero);
    let case = policy::valid_evidence_identity(".chrono-harness/State/x", "report", zero);
    assert(!empty);
    expect_status_true(!empty); assert(!short);
    expect_status_true(!short); assert(!similar);
    expect_status_true(!similar); assert(!absolute);
    expect_status_true(!absolute); assert(!case);
    expect_status_true(!case);
    let no_kind = policy::valid_evidence_identity(".chrono-harness/state/x", "", zero);
    assert(!no_kind);
    expect_status_true(!no_kind);
}

proof fn unicode_evidence_prefix_encoding()
    ensures vstd::utf8::encode_utf8(".chrono-harness/state/"@).len() == 22,
        vstd::utf8::encode_utf8(".chrono-harness/state/中文\0"@).len() >= 22,
        vstd::utf8::encode_utf8(".chrono-harness/state/中文\0"@).take(22)
            == vstd::utf8::encode_utf8(".chrono-harness/state/"@),
{
    reveal_strlit(".chrono-harness/state/");
    reveal_strlit(".chrono-harness/state/中文\0");
    reveal_with_fuel(vstd::utf8::encode_utf8, 24);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn evidence_identity_keeps_unicode_and_nul_suffix() {
    proof {
        unicode_evidence_prefix_encoding(); zero_evidence_digest_is_valid();
        reveal_strlit("report"); assert("report"@.len() == 6);
    }
    let accepted = policy::valid_evidence_identity(".chrono-harness/state/中文\0", "report",
        "0000000000000000000000000000000000000000000000000000000000000000");
    assert(accepted);
    expect_status_true(accepted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn evidence_identity_rejects_missing_short_long_and_uppercase_digest() {
    proof {
        reveal_strlit("");
        reveal_strlit("000000000000000000000000000000000000000000000000000000000000000");
        reveal_strlit("00000000000000000000000000000000000000000000000000000000000000000");
        reveal_strlit("A000000000000000000000000000000000000000000000000000000000000000");
        assert(vstd::string::is_ascii(""));
        assert(vstd::string::is_ascii("000000000000000000000000000000000000000000000000000000000000000"));
        assert(vstd::string::is_ascii("00000000000000000000000000000000000000000000000000000000000000000"));
        assert(vstd::string::is_ascii("A000000000000000000000000000000000000000000000000000000000000000"));
        assert(""@.len() == 0);
        assert("000000000000000000000000000000000000000000000000000000000000000"@.len() == 63);
        assert("00000000000000000000000000000000000000000000000000000000000000000"@.len() == 65);
        assert("A000000000000000000000000000000000000000000000000000000000000000"@.len() == 64);
        assert("A000000000000000000000000000000000000000000000000000000000000000"@[0] == 'A');
        vstd::string::is_ascii_spec_bytes("");
        vstd::string::is_ascii_spec_bytes("000000000000000000000000000000000000000000000000000000000000000");
        vstd::string::is_ascii_spec_bytes("00000000000000000000000000000000000000000000000000000000000000000");
        vstd::string::is_ascii_spec_bytes("A000000000000000000000000000000000000000000000000000000000000000");
        assert("A000000000000000000000000000000000000000000000000000000000000000".spec_bytes()[0] == 65);
        assert(!policy::identifiers::oid_byte(
            "A000000000000000000000000000000000000000000000000000000000000000".spec_bytes()[0]));
    }
    let no_digest = policy::valid_evidence_identity(".chrono-harness/state/x", "report", "");
    let short_digest = policy::valid_evidence_identity(".chrono-harness/state/x", "report",
        "000000000000000000000000000000000000000000000000000000000000000");
    let long_digest = policy::valid_evidence_identity(".chrono-harness/state/x", "report",
        "00000000000000000000000000000000000000000000000000000000000000000");
    let upper_digest = policy::valid_evidence_identity(".chrono-harness/state/x", "report",
        "A000000000000000000000000000000000000000000000000000000000000000");
    assert(!no_digest);
    expect_status_true(!no_digest); assert(!short_digest);
    expect_status_true(!short_digest); assert(!long_digest);
    expect_status_true(!long_digest); assert(!upper_digest);
    expect_status_true(!upper_digest);
}

fn delta_clone_keeps_every_field(value: &policy::Delta) {
    let copied = value.clone();
    assert(copied.kind == value.kind && copied.path == value.path);
    expect_status_text(copied.kind.as_str(), value.kind.as_str());
    expect_status_text(copied.path.as_str(), value.path.as_str());
    assert(copied.old_blob == value.old_blob && copied.new_blob == value.new_blob);
    expect_optional_text(&copied.old_blob, &value.old_blob);
    expect_optional_text(&copied.new_blob, &value.new_blob);
    assert(copied.old_mode == value.old_mode && copied.new_mode == value.new_mode);
    expect_optional_text(&copied.old_mode, &value.old_mode);
    expect_optional_text(&copied.new_mode, &value.new_mode);
    let equal = copied.eq(value);
    assert(equal);
    expect_status_true(equal);
}

fn delta_equality_distinguishes_each_complete_field(left: &str, right: &str)
    requires left@ != right@,
{
    let original = policy::Delta { kind: left.to_owned(), path: left.to_owned(),
        old_blob: Some(left.to_owned()), new_blob: Some(left.to_owned()),
        old_mode: Some(left.to_owned()), new_mode: Some(left.to_owned()) };
    delta_clone_keeps_every_field(&original);
    let kind = policy::Delta { kind: right.to_owned(), ..original.clone() };
    let path = policy::Delta { path: right.to_owned(), ..original.clone() };
    let old_blob = policy::Delta { old_blob: Some(right.to_owned()), ..original.clone() };
    let new_blob = policy::Delta { new_blob: Some(right.to_owned()), ..original.clone() };
    let old_mode = policy::Delta { old_mode: Some(right.to_owned()), ..original.clone() };
    let new_mode = policy::Delta { new_mode: Some(right.to_owned()), ..original.clone() };
    let equal = original.eq(&kind); assert(!equal);
    expect_status_true(!equal);
    let equal = original.eq(&path); assert(!equal);
    expect_status_true(!equal);
    let equal = original.eq(&old_blob); assert(!equal);
    expect_status_true(!equal);
    let equal = original.eq(&new_blob); assert(!equal);
    expect_status_true(!equal);
    let equal = original.eq(&old_mode); assert(!equal);
    expect_status_true(!equal);
    let equal = original.eq(&new_mode); assert(!equal);
    expect_status_true(!equal);
    let missing = policy::Delta { old_blob: None, ..original.clone() };
    let equal = original.eq(&missing); assert(!equal);
    expect_status_true(!equal);
    let missing = policy::Delta { new_blob: None, ..original.clone() };
    let equal = original.eq(&missing); assert(!equal);
    expect_status_true(!equal);
    let missing = policy::Delta { old_mode: None, ..original.clone() };
    let equal = original.eq(&missing); assert(!equal);
    expect_status_true(!equal);
    let missing = policy::Delta { new_mode: None, ..original.clone() };
    let equal = original.eq(&missing); assert(!equal);
    expect_status_true(!equal);
}

fn any_listed_path_is_accepted(path: &str, kind: &str, metadata: &str) {
    let entry = policy::Delta { kind: kind.to_owned(), path: path.to_owned(),
        old_blob: Some(metadata.to_owned()), new_blob: None,
        old_mode: None, new_mode: Some(metadata.to_owned()) };
    delta_clone_keeps_every_field(&entry);
    let entries = vec![entry];
    let slice = entries.as_slice();
    proof {
        assert(slice@.len() == 1 && slice@[0].path@ == path@);
        assert(exists|i: int| 0 <= i < slice.len() && #[trigger] slice[i].path@ == path@);
        assert(policy::reference_allowed(path@, Some(slice)));
    }
    let accepted = policy::valid_finding_reference(path, Some(slice));
    assert(accepted);
    expect_status_true(accepted);
}

fn unlisted_relative_reference_is_rejected(reference: &str, path: &str)
    requires
        reference.spec_bytes().len() == 0 || reference.spec_bytes()[0] != 47,
        reference@ != path@,
{
    let entries = vec![policy::Delta { kind: "".to_owned(), path: path.to_owned(),
        old_blob: None, new_blob: None, old_mode: None, new_mode: None }];
    let accepted = policy::valid_finding_reference(reference, Some(entries.as_slice()));
    assert(!accepted);
    expect_status_true(!accepted);
    let initial = policy::valid_finding_reference(reference, None);
    assert(!initial);
    expect_status_true(!initial);
    let empty: Vec<policy::Delta> = Vec::new();
    let accepted = policy::valid_finding_reference(reference, Some(empty.as_slice()));
    assert(!accepted);
    expect_status_true(!accepted);
}

fn record_pointer_is_accepted_in_both_modes(reference: &str)
    requires reference.spec_bytes().len() > 0, reference.spec_bytes()[0] == 47,
{
    let initial = policy::valid_finding_reference(reference, None);
    let empty: Vec<policy::Delta> = Vec::new();
    let ordinary = policy::valid_finding_reference(reference, Some(empty.as_slice()));
    assert(initial && ordinary);
    expect_status_true(initial && ordinary);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn reference_membership_keeps_exact_text_and_initial_scope() {
    proof {
        reveal_strlit(""); reveal_strlit("x"); reveal_strlit("X"); reveal_strlit("x/y");
        reveal_strlit("/"); reveal_strlit("/records/a");
        assert(vstd::string::is_ascii("")); assert(vstd::string::is_ascii("x"));
        assert(vstd::string::is_ascii("X")); assert(vstd::string::is_ascii("x/y"));
        assert(vstd::string::is_ascii("/")); assert(vstd::string::is_ascii("/records/a"));
        vstd::string::is_ascii_spec_bytes(""); vstd::string::is_ascii_spec_bytes("x");
        vstd::string::is_ascii_spec_bytes("X"); vstd::string::is_ascii_spec_bytes("x/y");
        vstd::string::is_ascii_spec_bytes("/"); vstd::string::is_ascii_spec_bytes("/records/a");
        assert(""@.len() == 0 && "x"@.len() == 1 && "x/y"@.len() == 3);
        assert("X"@[0] == 'X' && "x"@[0] == 'x');
        assert("x".spec_bytes()[0] == 120 && "x/y".spec_bytes()[0] == 120);
        assert("/".spec_bytes()[0] == 47 && "/records/a".spec_bytes()[0] == 47);
    }
    delta_equality_distinguishes_each_complete_field("x", "X");
    delta_equality_distinguishes_each_complete_field("", "x");
    any_listed_path_is_accepted("", "", "");
    any_listed_path_is_accepted("中文\0", "unknown", "arbitrary\0metadata");
    any_listed_path_is_accepted("é", "", "");
    any_listed_path_is_accepted("e\u{301}", "", "");
    unlisted_relative_reference_is_rejected("", "x");
    unlisted_relative_reference_is_rejected("x", "X");
    unlisted_relative_reference_is_rejected("x/y", "x");
    record_pointer_is_accepted_in_both_modes("/");
    record_pointer_is_accepted_in_both_modes("/records/a");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn finding_reference_list_checks_every_position_and_preserves_duplicates() {
    proof {
        reveal_strlit("x"); reveal_strlit("y"); reveal_strlit("/a");
        assert(vstd::string::is_ascii("x")); assert(vstd::string::is_ascii("y"));
        assert(vstd::string::is_ascii("/a"));
        vstd::string::is_ascii_spec_bytes("x"); vstd::string::is_ascii_spec_bytes("y");
        vstd::string::is_ascii_spec_bytes("/a");
        assert("x".spec_bytes()[0] == 120 && "y".spec_bytes()[0] == 121);
        assert("/a".spec_bytes()[0] == 47 && "x"@[0] != "y"@[0]);
    }
    let entries = vec![policy::Delta { kind: "".to_owned(), path: "x".to_owned(),
        old_blob: None, new_blob: None, old_mode: None, new_mode: None }];
    let finding = policy::Finding { code: "".to_owned(), level: "".to_owned(),
        message: "".to_owned(), delta_refs: vec!["/a".to_owned(), "x".to_owned(),
            "x".to_owned(), "/a".to_owned()], causes: Vec::new() };
    let slice = entries.as_slice();
    proof {
        assert(slice@.len() == 1 && slice@[0].path@ == "x"@);
        assert(finding.delta_refs@.len() == 4);
        assert(finding.delta_refs@[0]@ == "/a"@ && finding.delta_refs@[3]@ == "/a"@);
        assert(finding.delta_refs@[1]@ == "x"@ && finding.delta_refs@[2]@ == "x"@);
        assert(policy::record_reference("/a"@));
        assert(!policy::record_reference("x"@) && !policy::record_reference("y"@));
        assert(policy::reference_allowed("/a"@, Some(slice)));
        assert(policy::reference_allowed("x"@, Some(slice))) by {
            assert(slice[0].path@ == "x"@);
            assert(exists|j: int| 0 <= j < slice.len() && slice[j].path@ == "x"@);
        }
        assert(!policy::reference_allowed("y"@, Some(slice))) by {
            assert forall|j: int| 0 <= j < slice.len() implies slice[j].path@ != "y"@ by {
                assert(j == 0);
            }
        }
        assert forall|i: int| 0 <= i < finding.delta_refs@.len()
            implies policy::reference_allowed(#[trigger] finding.delta_refs@[i]@, Some(slice)) by {
            if i == 0 || i == 3 {
                assert(finding.delta_refs@[i]@ == "/a"@);
                assert(policy::record_reference(finding.delta_refs@[i]@));
            } else {
                assert(finding.delta_refs@[i]@ == slice@[0].path@);
                assert(exists|j: int| 0 <= j < slice@.len()
                    && slice@[j].path@ == finding.delta_refs@[i]@);
            }
        }
        assert(!policy::reference_allowed(finding.delta_refs@[1]@, None));
    }
    let accepted = policy::valid_finding_references(&finding, Some(slice));
    assert(accepted);
    expect_status_true(accepted);
    let initial = policy::valid_finding_references(&finding, None);
    assert(!initial);
    expect_status_true(!initial);
    let first = policy::Finding { delta_refs: vec!["y".to_owned(), "x".to_owned()], ..finding.clone() };
    let middle = policy::Finding { delta_refs: vec!["x".to_owned(), "y".to_owned(), "x".to_owned()], ..finding.clone() };
    let last = policy::Finding { delta_refs: vec!["x".to_owned(), "y".to_owned()], ..finding.clone() };
    proof {
        assert(first.delta_refs@.len() == 2 && first.delta_refs@[0]@ == "y"@);
        assert(middle.delta_refs@.len() == 3 && middle.delta_refs@[1]@ == "y"@);
        assert(last.delta_refs@.len() == 2 && last.delta_refs@[1]@ == "y"@);
        assert(!policy::reference_allowed(first.delta_refs@[0]@, Some(slice)));
        assert(!policy::reference_allowed(middle.delta_refs@[1]@, Some(slice)));
        assert(!policy::reference_allowed(last.delta_refs@[1]@, Some(slice)));
    }
    let accepted = policy::valid_finding_references(&first, Some(slice)); assert(!accepted);
    expect_status_true(!accepted);
    let accepted = policy::valid_finding_references(&middle, Some(slice)); assert(!accepted);
    expect_status_true(!accepted);
    let accepted = policy::valid_finding_references(&last, Some(slice)); assert(!accepted);
    expect_status_true(!accepted);
    let records = policy::Finding { delta_refs: vec!["/a".to_owned(), "/a".to_owned()], ..finding.clone() };
    let accepted = policy::valid_finding_references(&records, None); assert(accepted);
    expect_status_true(accepted);
    let empty = policy::Finding { delta_refs: Vec::new(), ..finding };
    let accepted = policy::valid_finding_references(&empty, None); assert(accepted);
    expect_status_true(accepted);
    let fields = policy::valid_finding_fields(&empty); assert(!fields);
    expect_status_true(!fields);
}

fn tree_entry(mode: &str, oid: &str, kind: &str) -> (out: policy::TreeEntry)
    ensures out.mode@ == mode@, out.oid@ == oid@, out.kind@ == kind@,
{
    policy::TreeEntry { mode: mode.to_owned(), oid: oid.to_owned(), kind: kind.to_owned() }
}

fn expect_tree_slot(
    tree: &policy::Tree, path: &policy::TreePath, mode: &str, oid: &str, kind: &str,
)
    requires tree@.contains_key(*path),
        policy::tree_entry_fields(&tree@[*path]) == (mode@, oid@, kind@),
{
    proof { policy::tree_path_obeys_cmp(); }
    match tree.get(path) {
        Some(entry) => {
            expect_status_text(entry.mode.as_str(), mode);
            expect_status_text(entry.oid.as_str(), oid);
            expect_status_text(entry.kind.as_str(), kind);
        },
        None => { panic!(); },
    }
}

fn expect_tree_insert_ok(out: Result<(), String>)
    requires out.is_ok(),
{
    match out { Ok(()) => {}, Err(_) => { panic!(); } }
}

fn expect_tree_duplicate(out: Result<(), String>)
    requires match out {
        Err(message) => message@ == "duplicate Git tree path"@,
        Ok(()) => false,
    },
{
    match out {
        Err(message) => { expect_status_text(message.as_str(), "duplicate Git tree path"); },
        Ok(()) => { panic!(); },
    }
}

// 预期独立列出完整字段、数量、失败文本及失败后的状态，不由生产函数生成。
fn git_tree_insert_preserves_other_paths_and_rejects_every_duplicate(a: &str, b: &str)
    requires a@ != b@,
{
    proof { policy::tree_path_obeys_cmp(); }
    let mut tree = policy::Tree::new();
    let first = policy::TreePath::from_text(a);
    let second = policy::TreePath::from_text(b);
    let inserted = first.clone();
    let out = policy::insert_git_tree_entry(&mut tree, inserted, "100644", "first\0", "blob");
    expect_tree_insert_ok(out);
    expect_status_true(tree.len() == 1);
    expect_tree_slot(&tree, &first, "100644", "first\0", "blob");
    let inserted = second.clone();
    let out = policy::insert_git_tree_entry(&mut tree, inserted, "", "", "tree");
    expect_tree_insert_ok(out);
    expect_status_true(tree.len() == 2);
    expect_tree_slot(&tree, &first, "100644", "first\0", "blob");
    expect_tree_slot(&tree, &second, "", "", "tree");

    // 重新构造及 clone 都有独立存储；重复按全文判断，不按地址或条目字段判断。
    let same_path = policy::TreePath::from_text(a);
    let out = policy::insert_git_tree_entry(&mut tree, same_path, "100755", "第二\0", "commit");
    expect_tree_duplicate(out);
    expect_status_true(tree.len() == 2);
    expect_tree_slot(&tree, &first, "100755", "第二\0", "commit");
    expect_tree_slot(&tree, &second, "", "", "tree");
    let same_path = first.clone();
    let out = policy::insert_git_tree_entry(&mut tree, same_path, "100755", "第二\0", "commit");
    expect_tree_duplicate(out);
    expect_status_true(tree.len() == 2);
    expect_tree_slot(&tree, &first, "100755", "第二\0", "commit");
    expect_tree_slot(&tree, &second, "", "", "tree");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_tree_insert_keeps_full_path_identity_and_exact_duplicate_state() {
    proof {
        reveal_strlit(""); reveal_strlit("a"); reveal_strlit("a\0");
        reveal_strlit("目录/é"); reveal_strlit("目录/e\u{301}");
        assert(""@.len() != "a"@.len());
        assert("a"@.len() != "a\0"@.len());
        assert("目录/é"@.len() != "目录/e\u{301}"@.len());
    }
    git_tree_insert_preserves_other_paths_and_rejects_every_duplicate("", "a");
    git_tree_insert_preserves_other_paths_and_rejects_every_duplicate("a", "a\0");
    git_tree_insert_preserves_other_paths_and_rejects_every_duplicate("目录/é", "目录/e\u{301}");
}

// 独立 fixture 明确列出字段及分隔位置；不调用生产解析来生成预期。
proof fn tree_fixture_row(raw: &str, path: &str)
    requires
        vstd::string::is_ascii(raw), vstd::string::is_ascii(path),
        raw@ == "100644 blob 0000000000000000000000000000000000000001\t"@ + path@,
    ensures policy::git_tree_row_spec(raw.spec_bytes()) ==
        if policy::paths::relative_path(path.spec_bytes()) {
            Ok((path.spec_bytes(), "100644".spec_bytes(),
                "0000000000000000000000000000000000000001".spec_bytes(), "blob".spec_bytes()))
        } else { Err(policy::GitInventoryErrorView::InvalidPath(path.spec_bytes())) },
{
    reveal(policy::git_tree_row_spec);
    let meta = "100644 blob 0000000000000000000000000000000000000001";
    let tail = "blob 0000000000000000000000000000000000000001";
    let oid = "0000000000000000000000000000000000000001";
    reveal_strlit("100644 blob 0000000000000000000000000000000000000001\t");
    reveal_strlit("100644 blob 0000000000000000000000000000000000000001");
    reveal_strlit("blob 0000000000000000000000000000000000000001");
    reveal_strlit("0000000000000000000000000000000000000001");
    reveal_strlit("100644"); reveal_strlit("blob");
    assert(vstd::string::is_ascii(meta) && vstd::string::is_ascii(tail)
        && vstd::string::is_ascii(oid) && vstd::string::is_ascii("100644")
        && vstd::string::is_ascii("blob"));
    vstd::string::is_ascii_spec_bytes(raw);
    vstd::string::is_ascii_spec_bytes(path);
    vstd::string::is_ascii_spec_bytes(meta);
    vstd::string::is_ascii_spec_bytes(tail);
    vstd::string::is_ascii_spec_bytes(oid);
    vstd::string::is_ascii_spec_bytes("100644");
    vstd::string::is_ascii_spec_bytes("blob");
    scope_text::ascii_separator_end_is(raw.spec_bytes(), 9, 52);
    assert(raw.spec_bytes().take(52) =~= meta.spec_bytes());
    assert(raw.spec_bytes().skip(53) =~= path.spec_bytes());
    scope_text::ascii_separator_end_is(meta.spec_bytes(), 32, 6);
    scope_text::ascii_separator_end_is(tail.spec_bytes(), 32, 4);
    scope_text::ascii_separator_end_is(oid.spec_bytes(), 32, 40);
    assert(meta.spec_bytes().take(6) =~= "100644".spec_bytes());
    assert(meta.spec_bytes().skip(7) =~= tail.spec_bytes());
    assert(tail.spec_bytes().take(4) =~= "blob".spec_bytes());
    assert(tail.spec_bytes().skip(5) =~= oid.spec_bytes());
    assert(oid.spec_bytes().take(40) =~= oid.spec_bytes());
    reveal_with_fuel(scope_text::ascii_split_records, 4);
    assert(scope_text::ascii_split_records(meta.spec_bytes(), 32, true)
        =~= seq!["100644".spec_bytes(), "blob".spec_bytes(), oid.spec_bytes()]);
    assert forall|i: int| 0 <= i < oid.spec_bytes().len()
        implies scope_text::oid_byte(#[trigger] oid.spec_bytes()[i]) by {
        assert(oid.spec_bytes()[i] == 48 || oid.spec_bytes()[i] == 49);
    }
    assert(oid.spec_bytes()[39] == 49);
    assert(scope_text::oid(oid.spec_bytes()));
}

fn expect_tree_parse_error(rows: &[&str], first: usize, expected: &str)
    requires first < rows.len(), policy::git_tree_prefix_valid(rows@, first as int),
        policy::git_tree_error_at(rows@, first as int, policy::GitInventoryErrorView::Message(expected@)),
{
    proof { policy::git_tree_error_blocks_prefix(rows@, first as int,
        policy::GitInventoryErrorView::Message(expected@), rows.len() as int); }
    match policy::parse_git_tree_rows(rows) {
        Err(error) => {
            proof {
                let j = choose|j: int| 0 <= j < rows.len() && policy::git_tree_prefix_valid(rows@, j)
                    && policy::git_tree_error_at(rows@, j, error.view());
                policy::git_tree_first_error_unique(rows@, first as int,
                    policy::GitInventoryErrorView::Message(expected@), j, error.view());
            }
            match error {
                policy::GitInventoryError::Message(message) => {
                    let rendered = policy::GitInventoryError::Message(message).render(copy_tree_fixture_path);
                    expect_status_text(rendered.as_str(), expected);
                },
                _ => { panic!(); },
            }
        },
        Ok(_) => { panic!(); },
    }
}

fn copy_tree_fixture_path(path: &str) -> (out: String)
    ensures out@ == path@,
{ path.to_owned() }

fn expect_tree_path_error(rows: &[&str], first: usize, expected: &str)
    requires first < rows.len(), policy::git_tree_prefix_valid(rows@, first as int),
        policy::git_tree_error_at(rows@, first as int, policy::GitInventoryErrorView::InvalidPath(expected.spec_bytes())),
{
    proof { policy::git_tree_error_blocks_prefix(rows@, first as int,
        policy::GitInventoryErrorView::InvalidPath(expected.spec_bytes()), rows.len() as int); }
    match policy::parse_git_tree_rows(rows) {
        Err(error) => {
            proof {
                let j = choose|j: int| 0 <= j < rows.len() && policy::git_tree_prefix_valid(rows@, j)
                    && policy::git_tree_error_at(rows@, j, error.view());
                policy::git_tree_first_error_unique(rows@, first as int,
                    policy::GitInventoryErrorView::InvalidPath(expected.spec_bytes()), j, error.view());
            }
            match error {
                policy::GitInventoryError::InvalidPath(path) => {
                    proof {
                        vstd::utf8::encode_utf8_decode_utf8(path@);
                        vstd::utf8::encode_utf8_decode_utf8(expected@);
                    }
                    expect_status_text(path, expected);
                    let rendered = policy::GitInventoryError::InvalidPath(path).render(copy_tree_fixture_path);
                    expect_status_text(rendered.as_str(), expected);
                },
                _ => { panic!(); },
            }
        },
        Ok(_) => { panic!(); },
    }
}

proof fn empty_metadata_tree_fixture(raw: &str, path: &str)
    requires vstd::string::is_ascii(raw), vstd::string::is_ascii(path), raw@ == "  \t"@ + path@,
    ensures policy::git_tree_row_spec(raw.spec_bytes()) ==
        if policy::paths::relative_path(path.spec_bytes()) {
            Err::<policy::GitTreeRowBytes, policy::GitInventoryErrorView>(
                policy::GitInventoryErrorView::Message("expected full nonzero lowercase commit OID"@))
        } else { Err(policy::GitInventoryErrorView::InvalidPath(path.spec_bytes())) },
{
    reveal(policy::git_tree_row_spec);
    reveal_strlit("  \t"); reveal_strlit("  "); reveal_strlit(" "); reveal_strlit("");
    broadcast use vstd::string::is_ascii_spec_bytes;
    assert(vstd::string::is_ascii("  ") && vstd::string::is_ascii(" ") && vstd::string::is_ascii(""));
    scope_text::ascii_separator_end_is(raw.spec_bytes(), 9, 2);
    assert(raw.spec_bytes().take(2) =~= "  ".spec_bytes());
    assert(raw.spec_bytes().skip(3) =~= path.spec_bytes());
    scope_text::ascii_separator_end_is("  ".spec_bytes(), 32, 0);
    scope_text::ascii_separator_end_is(" ".spec_bytes(), 32, 0);
    scope_text::ascii_separator_end_is("".spec_bytes(), 32, 0);
    assert("  ".spec_bytes().take(0) =~= "".spec_bytes());
    assert("  ".spec_bytes().skip(1) =~= " ".spec_bytes());
    assert(" ".spec_bytes().take(0) =~= "".spec_bytes());
    assert(" ".spec_bytes().skip(1) =~= "".spec_bytes());
    assert("".spec_bytes().take(0) =~= "".spec_bytes());
    reveal_with_fuel(scope_text::ascii_split_records, 4);
    assert(scope_text::ascii_split_records("  ".spec_bytes(), 32, true)
        =~= seq!["".spec_bytes(), "".spec_bytes(), "".spec_bytes()]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn tree_parser_keeps_first_error_and_exact_path() {
    let good = "100644 blob 0000000000000000000000000000000000000001\ta";
    let bad_path = "100644 blob 0000000000000000000000000000000000000001\ta/../b\0tail";
    proof {
        reveal(policy::git_tree_row_spec);
        reveal_strlit("100644 blob 0000000000000000000000000000000000000001\t");
        reveal_strlit("100644 blob 0000000000000000000000000000000000000001\ta");
        reveal_strlit("100644 blob 0000000000000000000000000000000000000001\ta/../b\0tail");
        reveal_strlit("a"); reveal_strlit("a/../b\0tail"); reveal_strlit("missing");
        reveal_strlit("bad\t"); reveal_strlit("bad"); reveal_strlit("");
        reveal_strlit("  \t"); reveal_strlit("  \ta"); reveal_strlit("  \t/"); reveal_strlit("/");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii(good) && vstd::string::is_ascii(bad_path)
            && vstd::string::is_ascii("a") && vstd::string::is_ascii("a/../b\0tail")
            && vstd::string::is_ascii("missing") && vstd::string::is_ascii("bad\t")
            && vstd::string::is_ascii("bad") && vstd::string::is_ascii(""));
        assert(vstd::string::is_ascii("  \t") && vstd::string::is_ascii("  \ta")
            && vstd::string::is_ascii("  \t/") && vstd::string::is_ascii("/"));
        assert(good@ =~= "100644 blob 0000000000000000000000000000000000000001\t"@ + "a"@);
        assert(bad_path@ =~= "100644 blob 0000000000000000000000000000000000000001\t"@ + "a/../b\0tail"@);
        tree_fixture_row(good, "a"); tree_fixture_row(bad_path, "a/../b\0tail");
        assert("  \t"@ =~= "  \t"@ + ""@);
        assert("  \ta"@ =~= "  \t"@ + "a"@);
        assert("  \t/"@ =~= "  \t"@ + "/"@);
        empty_metadata_tree_fixture("  \t", "");
        empty_metadata_tree_fixture("  \ta", "a");
        empty_metadata_tree_fixture("  \t/", "/");
        scope_text::ascii_separator_end_is("missing".spec_bytes(), 9, 7);
        scope_text::ascii_separator_end_is("bad\t".spec_bytes(), 9, 3);
        scope_text::ascii_separator_end_is("bad".spec_bytes(), 32, 3);
        assert("bad\t".spec_bytes().take(3) =~= "bad".spec_bytes());
        assert("bad\t".spec_bytes().skip(4) =~= "".spec_bytes());
        assert("bad".spec_bytes().take(3) =~= "bad".spec_bytes());
        assert(scope_text::ascii_split_records("bad".spec_bytes(), 32, true).len() == 1);
        assert(policy::paths::relative_path("a".spec_bytes()));
        assert("a/../b\0tail".spec_bytes()[6] == 0);
        assert(policy::paths::parent_component_at("a/../b\0tail".spec_bytes(), 2));
        assert(!policy::paths::relative_path("a/../b\0tail".spec_bytes()));
    }
    expect_tree_parse_error(&[good, "missing", bad_path], 1, "invalid Git tree row");
    expect_tree_parse_error(&["bad\t", "missing"], 0, "invalid Git tree metadata");
    expect_tree_path_error(&[good, bad_path, "missing"], 1, "a/../b\0tail");
    let duplicate = [good, good, "missing"];
    proof {
        assert(policy::git_tree_row_at(duplicate@, 0).is_ok());
        assert(policy::git_tree_row_at(duplicate@, 0) == policy::git_tree_row_at(duplicate@, 1));
        assert(exists|j: int| #![trigger policy::git_tree_row_at(duplicate@, j)]
            0 <= j < 1 && (policy::git_tree_row_at(duplicate@, j)->Ok_0).0
            == (policy::git_tree_row_at(duplicate@, 1)->Ok_0).0) by {
            assert((policy::git_tree_row_at(duplicate@, 0)->Ok_0).0
                == (policy::git_tree_row_at(duplicate@, 1)->Ok_0).0);
        }
        assert(policy::git_tree_error_at(duplicate@, 1,
            policy::GitInventoryErrorView::Message("duplicate Git tree path"@)));
    }
    expect_tree_parse_error(&duplicate, 1, "duplicate Git tree path");
    expect_tree_parse_error(&[good, "missing", good], 1, "invalid Git tree row");
    expect_tree_path_error(&["  \t", good], 0, "");
    expect_tree_path_error(&["  \t/", "missing"], 0, "/");
    expect_tree_parse_error(&["  \ta", "missing"], 0, "expected full nonzero lowercase commit OID");
}

fn expect_fixture_tree_member(tree: &policy::Tree, path: &str)
    requires exists|key: policy::TreePath| tree@.contains_key(key) && key@ == path@
        && policy::tree_entry_fields(&tree@[key]) == ("100644"@,
            "0000000000000000000000000000000000000001"@, "blob"@),
{
    proof { policy::tree_path_obeys_cmp(); }
    let mut iterator = tree.iter();
    let ghost all = iterator@.1;
    let ghost expected = choose|key: policy::TreePath| tree@.contains_key(key)
        && key@ == path@ && policy::tree_entry_fields(&tree@[key]) == ("100644"@,
            "0000000000000000000000000000000000000001"@, "blob"@);
    proof {
        assert(tree@.contains_key(expected));
        assert(tree@.kv_pairs().contains((expected, tree@[expected])));
        assert(all.to_set().contains((expected, tree@[expected])));
    }
    let ghost expected_index = choose|j: int| 0 <= j < all.len() && all[j] == (expected, tree@[expected]);
    let mut found = false;
    loop
        invariant
            iterator@.1 == all, 0 <= iterator@.0 <= all.len(),
            all.to_set() == tree@.kv_pairs(),
            0 <= expected_index < all.len(), all[expected_index].0@ == path@,
            policy::tree_entry_fields(&all[expected_index].1) == ("100644"@,
                "0000000000000000000000000000000000000001"@, "blob"@),
            vstd::std_specs::btree::increasing_seq(all.map(|i: int, row: (policy::TreePath, policy::TreeEntry)| row.0)),
            found == (exists|i: int| 0 <= i < iterator@.0 && (#[trigger] all[i]).0@ == path@),
        decreases all.len() - iterator@.0,
    {
        match iterator.next() {
            None => {
                proof {
                    assert(0 <= expected_index < iterator@.0);
                    assert(exists|i: int| 0 <= i < iterator@.0 && (#[trigger] all[i]).0@ == path@);
                }
                expect_status_true(found); return;
            },
            Some((key, entry)) => {
                if scope_text::equal_text(key.as_str(), path) {
                    // 所有同路径键属于实际唯一键；预期完整字段独立写在此处。
                    proof {
                        policy::tree_path_obeys_cmp();
                        let j = expected_index;
                        let k = iterator@.0 - 1;
                        let keys = all.map(|i: int, row: (policy::TreePath, policy::TreeEntry)| row.0);
                        vstd::std_specs::btree::axiom_increasing_seq_meaning(keys);
                        if j < k { assert(keys[j].cmp_spec(&keys[k]) is Less); }
                        if k < j { assert(keys[k].cmp_spec(&keys[j]) is Less); }
                        assert(j == k);
                    }
                    expect_status_text(entry.mode.as_str(), "100644");
                    expect_status_text(entry.oid.as_str(), "0000000000000000000000000000000000000001");
                    expect_status_text(entry.kind.as_str(), "blob");
                    found = true;
                }
            },
        }
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn tree_parser_returns_complete_map_and_preserves_literal_paths() {
    let a = "100644 blob 0000000000000000000000000000000000000001\ta";
    let b = "100644 blob 0000000000000000000000000000000000000001\ta//./b/";
    proof {
        policy::tree_path_obeys_cmp();
        reveal_strlit("100644 blob 0000000000000000000000000000000000000001\t");
        reveal_strlit("100644 blob 0000000000000000000000000000000000000001\ta");
        reveal_strlit("100644 blob 0000000000000000000000000000000000000001\ta//./b/");
        reveal_strlit("a"); reveal_strlit("a//./b/");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii(a) && vstd::string::is_ascii(b)
            && vstd::string::is_ascii("a") && vstd::string::is_ascii("a//./b/"));
        assert(a@ =~= "100644 blob 0000000000000000000000000000000000000001\t"@ + "a"@);
        assert(b@ =~= "100644 blob 0000000000000000000000000000000000000001\t"@ + "a//./b/"@);
        tree_fixture_row(a, "a"); tree_fixture_row(b, "a//./b/");
        assert(policy::paths::relative_path("a".spec_bytes()));
        assert(policy::paths::relative_path("a//./b/".spec_bytes()));
    }
    let rows = [b, a];
    match policy::parse_git_tree_rows(&rows) {
        Ok(tree) => {
            expect_status_true(tree.len() == 2);
            proof {
                assert forall|i: int| #![trigger policy::git_tree_row_at(rows@, i)]
                    0 <= i < rows.len() implies exists|key: policy::TreePath|
                    tree@.contains_key(key) && key@ == (if i == 0 { "a//./b/"@ } else { "a"@ })
                        && policy::tree_entry_fields(&tree@[key]) == ("100644"@,
                            "0000000000000000000000000000000000000001"@, "blob"@) by {
                    let key = choose|key: policy::TreePath| tree@.contains_key(key)
                        && policy::git_tree_entry_matches(key, tree@[key],
                            (policy::git_tree_row_at(rows@, i)->Ok_0));
                    vstd::utf8::encode_utf8_decode_utf8(key@);
                    vstd::utf8::encode_utf8_decode_utf8(tree@[key].mode@);
                    vstd::utf8::encode_utf8_decode_utf8(tree@[key].oid@);
                    vstd::utf8::encode_utf8_decode_utf8(tree@[key].kind@);
                    vstd::utf8::encode_utf8_decode_utf8("a//./b/"@);
                    vstd::utf8::encode_utf8_decode_utf8("a"@);
                    vstd::utf8::encode_utf8_decode_utf8("100644"@);
                    vstd::utf8::encode_utf8_decode_utf8("0000000000000000000000000000000000000001"@);
                    vstd::utf8::encode_utf8_decode_utf8("blob"@);
                }
            }
            proof {
                assert(policy::git_tree_row_at(rows@, 0).is_ok());
                assert(policy::git_tree_row_at(rows@, 1).is_ok());
            }
            expect_fixture_tree_member(&tree, "a");
            expect_fixture_tree_member(&tree, "a//./b/");
        },
        Err(_) => { panic!(); },
    }
    match policy::parse_git_tree_rows(&[]) {
        Ok(tree) => expect_status_true(tree.len() == 0),
        Err(_) => { panic!(); },
    }
}

fn tree_path_keeps_full_content(text: &str) {
    let path = policy::TreePath::from_text(text);
    let copied = path.clone();
    let from_owned = policy::TreePath::from_owned(text.to_owned());
    expect_status_true(scope_text::equal_text(path.as_str(), text));
    expect_status_true(scope_text::equal_text(copied.as_str(), text));
    expect_status_true(scope_text::equal_text(from_owned.as_str(), text));
    expect_status_true(path.eq(&copied));
    expect_status_true(matches!(path.cmp(&from_owned), Ordering::Equal));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn tree_path_preserves_empty_unicode_controls_and_distinct_allocations() {
    tree_path_keeps_full_content("");
    tree_path_keeps_full_content("a");
    tree_path_keeps_full_content("目录/é\0\n");
    tree_path_keeps_full_content("e\u{301}");
    tree_path_keeps_full_content("../a//b/");
}

fn tree_path_has_expected_order(a: &str, b: &str)
    requires scope_text::byte_lt(a.spec_bytes(), b.spec_bytes()),
{
    let a = policy::TreePath::from_text(a);
    let b = policy::TreePath::from_text(b);
    proof {
        scope_text::byte_lt_strict(a.bytes(), b.bytes());
        scope_text::byte_lt_asymmetric(a.bytes(), b.bytes());
    }
    expect_status_true(matches!(a.cmp(&b), Ordering::Less));
    expect_status_true(matches!(b.cmp(&a), Ordering::Greater));
    expect_status_true(!a.eq(&b));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn tree_path_orders_case_nul_and_complete_prefixes() {
    proof {
        reveal_strlit(""); reveal_strlit("a"); reveal_strlit("a\0");
        reveal_strlit("a0"); reveal_strlit("aa"); reveal_strlit("A");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("a")
            && vstd::string::is_ascii("a\0") && vstd::string::is_ascii("a0")
            && vstd::string::is_ascii("aa") && vstd::string::is_ascii("A"));
        assert(scope_text::byte_prefix("".spec_bytes(), "a".spec_bytes(), 0));
        assert(scope_text::byte_prefix("a".spec_bytes(), "a\0".spec_bytes(), 1));
        assert(scope_text::byte_prefix("a\0".spec_bytes(), "a0".spec_bytes(), 1));
        assert(scope_text::byte_prefix("a0".spec_bytes(), "aa".spec_bytes(), 1));
        assert(scope_text::byte_prefix("A".spec_bytes(), "a".spec_bytes(), 0));
        assert(scope_text::byte_lt("".spec_bytes(), "a".spec_bytes()));
        assert(scope_text::byte_lt("a".spec_bytes(), "a\0".spec_bytes()));
        assert(scope_text::byte_lt("a\0".spec_bytes(), "a0".spec_bytes()));
        assert(scope_text::byte_lt("a0".spec_bytes(), "aa".spec_bytes()));
        assert(scope_text::byte_lt("A".spec_bytes(), "a".spec_bytes()));
    }
    tree_path_has_expected_order("", "a");
    tree_path_has_expected_order("a", "a\0");
    tree_path_has_expected_order("a\0", "a0");
    tree_path_has_expected_order("a0", "aa");
    tree_path_has_expected_order("A", "a");
}

fn unchanged_entries_have_no_delta(path: &str, mode: &str, oid: &str, kind: &str) {
    let absent = policy::delta_for_entry(path, None, None);
    assert(absent.is_none());
    expect_status_true(absent.is_none());
    let old = tree_entry(mode, oid, kind);
    let new = tree_entry(mode, oid, kind);
    let equal = old.eq(&new);
    assert(equal);
    expect_status_true(equal);
    let copied = old.clone();
    assert(copied.mode@ == mode@ && copied.oid@ == oid@ && copied.kind@ == kind@);
    expect_status_text(copied.mode.as_str(), mode);
    expect_status_text(copied.oid.as_str(), oid);
    expect_status_text(copied.kind.as_str(), kind);
    let unchanged = policy::delta_for_entry(path, Some(&old), Some(&new));
    assert(unchanged.is_none());
    expect_status_true(unchanged.is_none());
}

fn additions_and_deletions_preserve_fields(path: &str, mode: &str, oid: &str, kind: &str) {
    let entry = tree_entry(mode, oid, kind);
    let added = policy::delta_for_entry(path, None, Some(&entry));
    match added {
        None => { assert(false); panic!(); },
        Some(change) => {
            assert(change.kind@ == "A"@ && change.path@ == path@);
            expect_status_text(change.kind.as_str(), "A");
            expect_status_text(change.path.as_str(), path);
            assert(change.old_blob.is_none() && change.old_mode.is_none());
            expect_status_true(change.old_blob.is_none() && change.old_mode.is_none());
            assert(policy::optional_text(&change.new_blob) == Some(oid@));
            expect_some_text(&change.new_blob, oid);
            assert(policy::optional_text(&change.new_mode) == Some(mode@));
            expect_some_text(&change.new_mode, mode);
        },
    }
    let deleted = policy::delta_for_entry(path, Some(&entry), None);
    match deleted {
        None => { assert(false); panic!(); },
        Some(change) => {
            assert(change.kind@ == "D"@ && change.path@ == path@);
            expect_status_text(change.kind.as_str(), "D");
            expect_status_text(change.path.as_str(), path);
            assert(policy::optional_text(&change.old_blob) == Some(oid@));
            expect_some_text(&change.old_blob, oid);
            assert(policy::optional_text(&change.old_mode) == Some(mode@));
            expect_some_text(&change.old_mode, mode);
            assert(change.new_blob.is_none() && change.new_mode.is_none());
            expect_status_true(change.new_blob.is_none() && change.new_mode.is_none());
        },
    }
}

fn modified_entries_preserve_both_sides(path: &str, old: &policy::TreeEntry, new: &policy::TreeEntry)
    requires old.mode@ != new.mode@ || old.oid@ != new.oid@ || old.kind@ != new.kind@,
{
    let changed = policy::delta_for_entry(path, Some(old), Some(new));
    match changed {
        None => { assert(false); panic!(); },
        Some(change) => {
            assert(change.kind@ == "M"@ && change.path@ == path@);
            expect_status_text(change.kind.as_str(), "M");
            expect_status_text(change.path.as_str(), path);
            assert(policy::optional_text(&change.old_blob) == Some(old.oid@));
            expect_some_text(&change.old_blob, old.oid.as_str());
            assert(policy::optional_text(&change.new_blob) == Some(new.oid@));
            expect_some_text(&change.new_blob, new.oid.as_str());
            assert(policy::optional_text(&change.old_mode) == Some(old.mode@));
            expect_some_text(&change.old_mode, old.mode.as_str());
            assert(policy::optional_text(&change.new_mode) == Some(new.mode@));
            expect_some_text(&change.new_mode, new.mode.as_str());
        },
    }
}

fn each_tree_field_changes_identity(path: &str, left: &str, right: &str)
    requires left@ != right@,
{
    let old = tree_entry(left, left, left);
    let mode = tree_entry(right, left, left);
    let oid = tree_entry(left, right, left);
    let kind = tree_entry(left, left, right);
    let equal = old.eq(&mode); assert(!equal);
    expect_status_true(!equal);
    let equal = old.eq(&oid); assert(!equal);
    expect_status_true(!equal);
    let equal = old.eq(&kind); assert(!equal);
    expect_status_true(!equal);
    modified_entries_preserve_both_sides(path, &old, &mode);
    modified_entries_preserve_both_sides(path, &old, &oid);
    modified_entries_preserve_both_sides(path, &old, &kind);
    modified_entries_preserve_both_sides(path, &mode, &old);
    modified_entries_preserve_both_sides(path, &oid, &old);
    modified_entries_preserve_both_sides(path, &kind, &old);
    let all = tree_entry(right, right, right);
    modified_entries_preserve_both_sides(path, &old, &all);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn tree_delta_preserves_absence_empty_fields_and_arbitrary_text() {
    unchanged_entries_have_no_delta("", "", "", "");
    unchanged_entries_have_no_delta("目录/é\0", "100644", "object\0", "blob");
    additions_and_deletions_preserve_fields("", "", "", "");
    additions_and_deletions_preserve_fields("目录/é\0", "mode\0", "对象\0", "任意\0");
    proof {
        reveal_strlit(""); reveal_strlit("x"); reveal_strlit("x\0");
        reveal_strlit("é"); reveal_strlit("e\u{301}");
        assert(""@.len() == 0 && "x"@.len() == 1 && "x\0"@.len() == 2);
        assert("é"@.len() == 1 && "e\u{301}"@.len() == 2);
    }
    each_tree_field_changes_identity("", "", "x");
    each_tree_field_changes_identity("a/b", "x", "x\0");
    each_tree_field_changes_identity("目录/é\0", "é", "e\u{301}");
}

// 独立预期按完整文本登记 fixture，不依赖生产合并递归决定预期。
spec fn fixture_tree(tree: Map<policy::TreePath, policy::TreeEntry>,
    entries: Map<Seq<char>, policy::TreeEntryFields>) -> bool {
    (forall|key: policy::TreePath| tree.contains_key(key) ==>
        entries.contains_key(key@) && policy::tree_entry_fields(&tree[key]) == entries[key@])
    && (forall|path: Seq<char>| entries.contains_key(path) ==>
        exists|key: policy::TreePath| tree.contains_key(key) && key@ == path)
}

fn insert_tree_fixture(tree: &mut policy::Tree, path: &str, mode: &str, oid: &str, kind: &str)
    ensures forall|entries: Map<Seq<char>, policy::TreeEntryFields>|
        fixture_tree(old(tree)@, entries) && !entries.contains_key(path@)
            ==> #[trigger] fixture_tree(tree@, entries.insert(path@, (mode@, oid@, kind@))),
{
    proof { policy::tree_path_obeys_cmp(); }
    let key = policy::TreePath::from_text(path);
    let ghost inserted = key;
    tree.insert(key, tree_entry(mode, oid, kind));
    proof {
        assert forall|entries: Map<Seq<char>, policy::TreeEntryFields>|
            fixture_tree(old(tree)@, entries) && !entries.contains_key(path@)
                implies #[trigger] fixture_tree(tree@, entries.insert(path@, (mode@, oid@, kind@))) by {
            let expected = entries.insert(path@, (mode@, oid@, kind@));
            assert forall|key: policy::TreePath| tree@.contains_key(key) implies
                expected.contains_key(key@) && policy::tree_entry_fields(&tree@[key]) == expected[key@] by {
                if key != inserted { assert(old(tree)@.contains_key(key)); }
            }
            assert forall|text: Seq<char>| expected.contains_key(text) implies
                exists|key: policy::TreePath| tree@.contains_key(key) && key@ == text by {
                if text == path@ {
                    assert(tree@.contains_key(inserted));
                } else {
                    let key = choose|key: policy::TreePath| old(tree)@.contains_key(key) && key@ == text;
                    assert(tree@.contains_key(key));
                }
            }
        }
    }
}

proof fn fixture_tree_projection(tree: Map<policy::TreePath, policy::TreeEntry>,
    entries: Map<Seq<char>, policy::TreeEntryFields>, path: Seq<char>)
    requires fixture_tree(tree, entries),
    ensures policy::map_entry_fields(tree, path)
        == if entries.contains_key(path) { Some(entries[path]) } else { None },
{
    if policy::map_has_path(tree, path) {
        let key = policy::map_path_key(tree, path);
        assert(tree.contains_key(key) && key@ == path);
    } else if entries.contains_key(path) {
        let key = choose|key: policy::TreePath| tree.contains_key(key) && key@ == path;
        assert(policy::map_has_path(tree, path));
    }
}

proof fn ordered_delta_head(changes: Seq<policy::DeltaFields>)
    requires changes.len() > 0, policy::delta_fields_ordered(changes),
    ensures policy::delta_at_path(changes, changes[0].0) == seq![changes[0]],
{
    let tail = changes.skip(1);
    let pred = policy::delta_path_predicate(changes[0].0);
    assert forall|i: int| 0 <= i < tail.len() implies !pred(#[trigger] tail[i]) by {
        scope_text::byte_lt_strict(vstd::utf8::encode_utf8(changes[0].0),
            vstd::utf8::encode_utf8(changes[i + 1].0));
    }
    tail.lemma_all_neg_filter_empty(pred);
    assert(changes == seq![changes[0]] + tail);
    tail.lemma_filter_prepend(changes[0], pred);
}

proof fn delta_projection_heads(actual: Seq<policy::DeltaFields>,
    expected: Seq<policy::DeltaFields>)
    requires actual.len() > 0,
        policy::delta_fields_ordered(actual), policy::delta_fields_ordered(expected),
        forall|path: Seq<char>| #[trigger] policy::delta_at_path(actual, path)
            == policy::delta_at_path(expected, path),
    ensures expected.len() > 0, actual[0] == expected[0],
{
    hide(scope_text::byte_lt);
    let path = actual[0].0;
    ordered_delta_head(actual);
    assert(policy::delta_at_path(expected, path) == seq![actual[0]]);
    assert(policy::delta_at_path(expected, path).contains(actual[0]));
    expected.lemma_filter_contains_rev(policy::delta_path_predicate(path), actual[0]);
    let i = expected.lemma_contains_to_index(actual[0]);
    ordered_delta_head(expected);
    assert(policy::delta_at_path(actual, expected[0].0) == seq![expected[0]]);
    assert(policy::delta_at_path(actual, expected[0].0).contains(expected[0]));
    actual.lemma_filter_contains_rev(policy::delta_path_predicate(expected[0].0), expected[0]);
    let j = actual.lemma_contains_to_index(expected[0]);
    if i > 0 {
        if j == 0 {
            scope_text::byte_lt_strict(vstd::utf8::encode_utf8(expected[0].0),
                vstd::utf8::encode_utf8(expected[i].0));
            assert(false);
        }
        assert(j > 0);
        scope_text::byte_lt_transitive(vstd::utf8::encode_utf8(actual[0].0),
            vstd::utf8::encode_utf8(expected[0].0), vstd::utf8::encode_utf8(actual[0].0));
        scope_text::byte_lt_strict(vstd::utf8::encode_utf8(actual[0].0),
            vstd::utf8::encode_utf8(actual[0].0));
        assert(false);
    }
    assert(actual[0] == expected[0]);
}

// 严格有序序列由全部逐路径投影唯一决定；首项与递归尾部各自证明。
proof fn delta_from_all_path_projections(actual: Seq<policy::DeltaFields>,
    expected: Seq<policy::DeltaFields>)
    requires policy::delta_fields_ordered(actual), policy::delta_fields_ordered(expected),
        forall|path: Seq<char>| #[trigger] policy::delta_at_path(actual, path)
            == policy::delta_at_path(expected, path),
    ensures actual == expected,
    decreases actual.len() + expected.len(),
{
    hide(scope_text::byte_lt);
    if actual.len() == 0 {
        if expected.len() > 0 {
            ordered_delta_head(expected);
            assert(policy::delta_at_path(actual, expected[0].0).len() == 0) by {
                reveal_with_fuel(Seq::filter, 1);
            }
        }
        assert(actual =~= expected);
        return;
    }
    delta_projection_heads(actual, expected);
    let a = actual.skip(1);
    let b = expected.skip(1);
    assert forall|path: Seq<char>| #[trigger] policy::delta_at_path(a, path)
        == policy::delta_at_path(b, path) by {
        let pred = policy::delta_path_predicate(path);
        a.lemma_filter_prepend(actual[0], pred);
        b.lemma_filter_prepend(expected[0], pred);
        let prefix = if pred(actual[0]) { seq![actual[0]] } else { seq![] };
        assert(actual == seq![actual[0]] + a);
        assert(expected == seq![expected[0]] + b);
        assert(policy::delta_at_path(actual, path) == policy::delta_at_path(expected, path));
        assert(prefix + a.filter(pred) == prefix + b.filter(pred));
        prefix.lemma_concat_injective(a.filter(pred), b.filter(pred));
    }
    assert(policy::delta_fields_ordered(a));
    assert(policy::delta_fields_ordered(b));
    delta_from_all_path_projections(a, b);
    assert(actual =~= seq![actual[0]] + a);
    assert(expected =~= seq![expected[0]] + b);
    assert(actual =~= expected);
}

proof fn full_delta_matches_fixture(changes: Seq<policy::Delta>,
    before: Map<policy::TreePath, policy::TreeEntry>, after: Map<policy::TreePath, policy::TreeEntry>,
    old_entries: Map<Seq<char>, policy::TreeEntryFields>, new_entries: Map<Seq<char>, policy::TreeEntryFields>,
    expected: Seq<policy::DeltaFields>)
    requires fixture_tree(before, old_entries), fixture_tree(after, new_entries),
        policy::delta_fields_ordered(policy::delta_records_view(changes)),
        policy::delta_fields_ordered(expected),
        forall|path: Seq<char>| #[trigger] policy::delta_at_path(policy::delta_records_view(changes), path)
            == policy::entry_delta_values(path, policy::map_entry_fields(before, path), policy::map_entry_fields(after, path)),
        forall|path: Seq<char>| #[trigger] policy::delta_at_path(expected, path)
            == policy::entry_delta_values(path,
                if old_entries.contains_key(path) { Some(old_entries[path]) } else { None },
                if new_entries.contains_key(path) { Some(new_entries[path]) } else { None }),
    ensures policy::delta_records_view(changes) == expected, changes.len() == expected.len(),
{
    assert forall|path: Seq<char>| #[trigger] policy::delta_at_path(policy::delta_records_view(changes), path)
        == policy::delta_at_path(expected, path) by {
        fixture_tree_projection(before, old_entries, path);
        fixture_tree_projection(after, new_entries, path);
    }
    delta_from_all_path_projections(policy::delta_records_view(changes), expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn full_tree_delta_of_empty_maps_is_empty() {
    let before = policy::Tree::new();
    let after = policy::Tree::new();
    let changes = policy::delta_for_trees(&before, &after);
    proof {
        reveal_with_fuel(Seq::filter, 1);
        full_delta_matches_fixture(changes@, before@, after@, Map::empty(), Map::empty(), seq![]);
    }
    expect_status_true(changes.len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn full_tree_delta_joins_both_maps_once_and_keeps_sorted_complete_records() {
    proof {
        policy::tree_path_obeys_cmp();
        reveal_strlit("a"); reveal_strlit("b"); reveal_strlit("c"); reveal_strlit("d");
        reveal_strlit("blob"); reveal_strlit("tree");
        assert("a"@[0] != "b"@[0] && "a"@[0] != "d"@[0] && "b"@[0] != "d"@[0]);
        assert("b"@[0] != "c"@[0] && "c"@[0] != "d"@[0]);
        assert("blob"@[0] != "tree"@[0]);
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("a") && vstd::string::is_ascii("b") && vstd::string::is_ascii("c"));
        assert("a".spec_bytes()[0] == 97 && "b".spec_bytes()[0] == 98 && "c".spec_bytes()[0] == 99);
        assert("a".spec_bytes().len() == 1 && "b".spec_bytes().len() == 1 && "c".spec_bytes().len() == 1);
        assert(scope_text::byte_prefix("a".spec_bytes(), "b".spec_bytes(), 0));
        assert(scope_text::byte_prefix("a".spec_bytes(), "c".spec_bytes(), 0));
        assert(scope_text::byte_prefix("b".spec_bytes(), "c".spec_bytes(), 0));
        assert(scope_text::byte_lt("a".spec_bytes(), "b".spec_bytes()));
        assert(scope_text::byte_lt("a".spec_bytes(), "c".spec_bytes()));
        assert(scope_text::byte_lt("b".spec_bytes(), "c".spec_bytes()));
    }
    let mut before = policy::Tree::new();
    let mut after = policy::Tree::new();
    let ghost mut old_entries = Map::empty();
    let ghost mut new_entries = Map::empty();
    insert_tree_fixture(&mut before, "a", "old mode", "old blob", "blob");
    proof { old_entries = old_entries.insert("a"@, ("old mode"@, "old blob"@, "blob"@)); }
    insert_tree_fixture(&mut before, "b", "same mode", "same blob", "blob");
    proof { old_entries = old_entries.insert("b"@, ("same mode"@, "same blob"@, "blob"@)); }
    insert_tree_fixture(&mut before, "d", "same", "same", "same");
    proof { old_entries = old_entries.insert("d"@, ("same"@, "same"@, "same"@)); }
    insert_tree_fixture(&mut after, "b", "same mode", "same blob", "tree");
    proof { new_entries = new_entries.insert("b"@, ("same mode"@, "same blob"@, "tree"@)); }
    insert_tree_fixture(&mut after, "c", "new mode", "new blob", "blob");
    proof { new_entries = new_entries.insert("c"@, ("new mode"@, "new blob"@, "blob"@)); }
    insert_tree_fixture(&mut after, "d", "same", "same", "same");
    proof { new_entries = new_entries.insert("d"@, ("same"@, "same"@, "same"@)); }
    let changes = policy::delta_for_trees(&before, &after);
    proof {
        let expected = seq![
            ("a"@, "D"@, Some("old blob"@), None, Some("old mode"@), None),
            ("b"@, "M"@, Some("same blob"@), Some("same blob"@), Some("same mode"@), Some("same mode"@)),
            ("c"@, "A"@, None, Some("new blob"@), None, Some("new mode"@))];
        assert forall|path: Seq<char>| #[trigger] policy::delta_at_path(expected, path)
            == policy::entry_delta_values(path,
                if old_entries.contains_key(path) { Some(old_entries[path]) } else { None },
                if new_entries.contains_key(path) { Some(new_entries[path]) } else { None }) by {
            reveal_with_fuel(Seq::filter, 4);
            if path == "a"@ {} else if path == "b"@ {} else if path == "c"@ {}
            else if path == "d"@ {} else {}
        }
        full_delta_matches_fixture(changes@, before@, after@, old_entries, new_entries, expected);
    }
    expect_status_true(changes.len() == 3);
    let deleted = &changes[0];
    expect_status_text(deleted.path.as_str(), "a");
    expect_status_text(deleted.kind.as_str(), "D");
    expect_some_text(&deleted.old_blob, "old blob");
    expect_some_text(&deleted.old_mode, "old mode");
    expect_status_true(deleted.new_blob.is_none() && deleted.new_mode.is_none());
    let modified = &changes[1];
    expect_status_text(modified.path.as_str(), "b");
    expect_status_text(modified.kind.as_str(), "M");
    expect_some_text(&modified.old_blob, "same blob");
    expect_some_text(&modified.new_blob, "same blob");
    expect_some_text(&modified.old_mode, "same mode");
    expect_some_text(&modified.new_mode, "same mode");
    let added = &changes[2];
    expect_status_text(added.path.as_str(), "c");
    expect_status_text(added.kind.as_str(), "A");
    expect_status_true(added.old_blob.is_none() && added.old_mode.is_none());
    expect_some_text(&added.new_blob, "new blob");
    expect_some_text(&added.new_mode, "new mode");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn full_tree_delta_preserves_empty_unicode_nul_and_reverse_changes() {
    proof {
        policy::tree_path_obeys_cmp();
        reveal_strlit(""); reveal_strlit("é"); reveal_strlit("a\0");
        assert(""@.len() == 0 && "é"@.len() == 1 && "a\0"@.len() == 2);
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("a\0"));
        assert("é"@ =~= seq!['é']);
        assert('é' as u32 == 233u32);
        assert(vstd::utf8::encode_scalar(233u32) == seq![195u8, 169u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 2);
        assert(vstd::utf8::encode_utf8("é"@) == seq![195u8, 169u8]);
        assert("".spec_bytes().len() == 0 && "é".spec_bytes().len() == 2 && "a\0".spec_bytes().len() == 2);
        assert("a\0".spec_bytes()[0] == 97 && "é".spec_bytes()[0] == 195);
        assert(scope_text::byte_prefix("".spec_bytes(), "a\0".spec_bytes(), 0));
        assert(scope_text::byte_prefix("".spec_bytes(), "é".spec_bytes(), 0));
        assert(scope_text::byte_prefix("a\0".spec_bytes(), "é".spec_bytes(), 0));
        assert(scope_text::byte_lt("".spec_bytes(), "a\0".spec_bytes()));
        assert(scope_text::byte_lt("".spec_bytes(), "é".spec_bytes()));
        assert(scope_text::byte_lt("a\0".spec_bytes(), "é".spec_bytes()));
    }
    let before = policy::Tree::new();
    let mut after = policy::Tree::new();
    let ghost mut entries = Map::empty();
    insert_tree_fixture(&mut after, "é", "", "", "");
    proof { entries = entries.insert("é"@, (""@, ""@, ""@)); }
    insert_tree_fixture(&mut after, "", "", "", "");
    proof { entries = entries.insert(""@, (""@, ""@, ""@)); }
    insert_tree_fixture(&mut after, "a\0", "mode\0", "对象\0", "blob");
    proof { entries = entries.insert("a\0"@, ("mode\0"@, "对象\0"@, "blob"@)); }
    let changes = policy::delta_for_trees(&before, &after);
    proof {
        let expected = seq![
            (""@, "A"@, None, Some(""@), None, Some(""@)),
            ("a\0"@, "A"@, None, Some("对象\0"@), None, Some("mode\0"@)),
            ("é"@, "A"@, None, Some(""@), None, Some(""@))];
        assert forall|path: Seq<char>| #[trigger] policy::delta_at_path(expected, path)
            == policy::entry_delta_values(path, None,
                if entries.contains_key(path) { Some(entries[path]) } else { None }) by {
            reveal_with_fuel(Seq::filter, 4);
            if path == ""@ {} else if path == "a\0"@ {} else if path == "é"@ {} else {}
        }
        full_delta_matches_fixture(changes@, before@, after@, Map::empty(), entries, expected);
    }
    expect_status_true(changes.len() == 3);
    expect_status_text(changes[0].path.as_str(), "");
    expect_status_text(changes[1].path.as_str(), "a\0");
    expect_status_text(changes[2].path.as_str(), "é");
    expect_status_text(changes[0].kind.as_str(), "A");
    expect_status_text(changes[1].kind.as_str(), "A");
    expect_status_text(changes[2].kind.as_str(), "A");
    expect_some_text(&changes[0].new_blob, "");
    expect_some_text(&changes[1].new_blob, "对象\0");
    let reverse = policy::delta_for_trees(&after, &before);
    proof {
        let expected = seq![
            (""@, "D"@, Some(""@), None, Some(""@), None),
            ("a\0"@, "D"@, Some("对象\0"@), None, Some("mode\0"@), None),
            ("é"@, "D"@, Some(""@), None, Some(""@), None)];
        assert forall|path: Seq<char>| #[trigger] policy::delta_at_path(expected, path)
            == policy::entry_delta_values(path,
                if entries.contains_key(path) { Some(entries[path]) } else { None }, None) by {
            reveal_with_fuel(Seq::filter, 4);
            if path == ""@ {} else if path == "a\0"@ {} else if path == "é"@ {} else {}
        }
        full_delta_matches_fixture(reverse@, after@, before@, entries, Map::empty(), expected);
    }
    expect_status_true(reverse.len() == 3);
    expect_status_text(reverse[0].path.as_str(), "");
    expect_status_text(reverse[1].path.as_str(), "a\0");
    expect_status_text(reverse[2].path.as_str(), "é");
    expect_status_text(reverse[0].kind.as_str(), "D");
    expect_status_text(reverse[1].kind.as_str(), "D");
    expect_status_text(reverse[2].kind.as_str(), "D");
    expect_some_text(&reverse[1].old_blob, "对象\0");
    expect_status_true(reverse[1].new_blob.is_none() && reverse[1].new_mode.is_none());
}

fn complete_finding(level: &str, reference: &str) -> (out: policy::Finding)
    ensures out.level@ == level@, out.code@ == "fixture"@,
        out.message@ == "independent expectation"@,
        out.delta_refs.len() == 1, out.delta_refs@[0]@ == reference@,
        out.causes@ == Seq::<String>::empty(),
{
    policy::Finding { delta_refs: vec![reference.to_owned()], ..finding(level) }
}

proof fn response_fixture_texts()
    ensures
        "fixture"@.len() > 0, "independent expectation"@.len() > 0, ""@.len() == 0,
        "/a".spec_bytes().len() > 0, "/a".spec_bytes()[0] == 47,
        "x".spec_bytes().len() > 0, "x".spec_bytes()[0] != 47,
        "info"@ != "warning"@, "info"@ != "error"@, "warning"@ != "error"@,
{
    reveal_strlit("fixture"); reveal_strlit("independent expectation"); reveal_strlit("");
    reveal_strlit("info"); reveal_strlit("warning"); reveal_strlit("error");
    assert("info"@[0] != "warning"@[0]);
    assert("info"@[0] != "error"@[0]);
    assert("warning"@[0] != "error"@[0]);
    reveal_strlit("/a"); reveal_strlit("x");
    assert(vstd::string::is_ascii("/a")); assert(vstd::string::is_ascii("x"));
    vstd::string::is_ascii_spec_bytes("/a"); vstd::string::is_ascii_spec_bytes("x");
}

fn expect_response_error(result: Result<(), String>, expected: &str)
    requires result.is_err(), result.unwrap_err()@ == expected@,
{
    assert(result.is_err());
    expect_status_true(result.is_err());
    match result {
        Err(message) => {
            assert(message@ == expected@);
            expect_status_text(message.as_str(), expected);
        },
        Ok(()) => { panic!(); },
    }
}

fn exact_header_accepts_empty_pass(protocol: &str, request: &str, judge: &str) {
    let findings: Vec<policy::Finding> = Vec::new();
    let accepted = policy::validate_response_fields(protocol, request, judge, &WireStatus::Pass, 0,
        protocol, request, judge, findings.as_slice(), None);
    assert(accepted.is_ok());
    expect_status_true(accepted.is_ok());
}

fn mismatched_header_precedes_bad_finding_and_status(a: &str, b: &str)
    requires a@ != b@,
{
    let invalid = vec![finding("error")];
    let protocol = policy::validate_response_fields(b, a, a, &WireStatus::Pass, 0,
        a, a, a, invalid.as_slice(), None);
    expect_response_error(protocol, "E_PROTOCOL: response identity/status/exit mismatch");
    let request = policy::validate_response_fields(a, b, a, &WireStatus::Pass, 0,
        a, a, a, invalid.as_slice(), None);
    expect_response_error(request, "E_PROTOCOL: response identity/status/exit mismatch");
    let judge = policy::validate_response_fields(a, a, b, &WireStatus::Pass, 0,
        a, a, a, invalid.as_slice(), None);
    expect_response_error(judge, "E_PROTOCOL: response identity/status/exit mismatch");
    let negative = policy::validate_response_fields(a, a, a, &WireStatus::Pass, i32::MIN,
        a, a, a, invalid.as_slice(), None);
    expect_response_error(negative, "E_PROTOCOL: response identity/status/exit mismatch");
    let large = policy::validate_response_fields(a, a, a, &WireStatus::Pass, i32::MAX,
        a, a, a, invalid.as_slice(), None);
    expect_response_error(large, "E_PROTOCOL: response identity/status/exit mismatch");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn response_preflight_keeps_exact_headers_and_error_priority() {
    exact_header_accepts_empty_pass("", "", "");
    exact_header_accepts_empty_pass("协议\0", "é", "e\u{301}");
    proof { reveal_strlit("a"); reveal_strlit("b"); assert("a"@[0] != "b"@[0]); }
    mismatched_header_precedes_bad_finding_and_status("a", "b");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn response_preflight_checks_actual_findings_before_status() {
    proof { response_fixture_texts(); }
    let good = complete_finding("info", "/a");
    let bad = policy::Finding { code: "".to_owned(), level: "warning".to_owned(), ..good.clone() };
    let first = vec![bad.clone(), good.clone()];
    let middle = vec![good.clone(), bad.clone(), good.clone()];
    let last = vec![good.clone(), good.clone(), bad];
    let a = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", first.as_slice(), None);
    assert(a.is_err());
    expect_status_true(a.is_err());
    expect_response_error(a, "E_PROTOCOL: invalid finding");
    let b = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", middle.as_slice(), None);
    expect_response_error(b, "E_PROTOCOL: invalid finding");
    let c = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", last.as_slice(), None);
    expect_response_error(c, "E_PROTOCOL: invalid finding");
    let empty_refs = vec![policy::Finding { delta_refs: Vec::new(), ..good }];
    let empty = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", empty_refs.as_slice(), None);
    expect_response_error(empty, "E_PROTOCOL: invalid finding");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn response_preflight_preserves_ordinary_and_initial_reference_scope() {
    proof { response_fixture_texts(); }
    let good = complete_finding("info", "/a");
    let relative = complete_finding("info", "x");
    let first = vec![relative.clone(), good.clone()];
    let middle = vec![good.clone(), relative.clone(), good.clone()];
    let last = vec![good.clone(), good, relative];
    let a = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", first.as_slice(), None);
    expect_response_error(a, "E_PROTOCOL: invalid finding");
    let b = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", middle.as_slice(), None);
    expect_response_error(b, "E_PROTOCOL: invalid finding");
    let c = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", last.as_slice(), None);
    expect_response_error(c, "E_PROTOCOL: invalid finding");
    let no_delta: Vec<policy::Delta> = Vec::new();
    let absent = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", first.as_slice(), Some(no_delta.as_slice()));
    expect_response_error(absent, "E_PROTOCOL: invalid finding");
    let delta = vec![policy::Delta { kind: "unexamined\0".to_owned(), path: "x".to_owned(),
        old_blob: Some("arbitrary\0".to_owned()), new_blob: None, old_mode: None, new_mode: None }];
    let slice = delta.as_slice();
    proof {
        assert(slice.len() == 1 && slice[0].path@ == "x"@);
        assert(exists|i: int| 0 <= i < slice.len() && #[trigger] slice[i].path@ == "x"@);
        assert(policy::reference_allowed("x"@, Some(slice)));
        assert forall|i: int| 0 <= i < first.len()
            implies policy::finding_fields(&(#[trigger] first@[i]))
                && policy::finding_references(&first@[i], Some(slice)) by {
            assert(first@[i].delta_refs.len() == 1);
            if i == 0 { assert(first@[i].delta_refs@[0]@ == "x"@); }
            else { assert(i == 1 && first@[i].delta_refs@[0]@ == "/a"@); }
        }
    }
    let allowed = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", first.as_slice(), Some(slice));
    assert(allowed.is_ok());
    expect_status_true(allowed.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn response_preflight_keeps_status_agreement_after_valid_fields() {
    proof { response_fixture_texts(); }
    let info = vec![complete_finding("info", "/a")];
    let warning = vec![complete_finding("warning", "/a")];
    let error = vec![complete_finding("error", "/a")];
    let pass = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", info.as_slice(), None);
    let warn = policy::validate_response_fields("p", "r", "j", &WireStatus::Warn, 0,
        "p", "r", "j", warning.as_slice(), None);
    let fail = policy::validate_response_fields("p", "r", "j", &WireStatus::Fail, 1,
        "p", "r", "j", error.as_slice(), None);
    let failure = policy::validate_response_fields("p", "r", "j", &WireStatus::Error, 2,
        "p", "r", "j", error.as_slice(), None);
    assert(pass.is_ok() && warn.is_ok() && fail.is_ok() && failure.is_ok());
    expect_status_true(pass.is_ok() && warn.is_ok() && fail.is_ok() && failure.is_ok());
    let bad_pass = policy::validate_response_fields("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", warning.as_slice(), None);
    expect_response_error(bad_pass, "E_PROTOCOL: finding/status disagreement");
    let bad_warn = policy::validate_response_fields("p", "r", "j", &WireStatus::Warn, 0,
        "p", "r", "j", info.as_slice(), None);
    expect_response_error(bad_warn, "E_PROTOCOL: finding/status disagreement");
    let bad_fail = policy::validate_response_fields("p", "r", "j", &WireStatus::Fail, 1,
        "p", "r", "j", warning.as_slice(), None);
    expect_response_error(bad_fail, "E_PROTOCOL: finding/status disagreement");
    let bad_error = policy::validate_response_fields("p", "r", "j", &WireStatus::Error, 2,
        "p", "r", "j", info.as_slice(), None);
    expect_response_error(bad_error, "E_PROTOCOL: finding/status disagreement");
}

fn exact_record_header_accepts_empty_pass(protocol: &str, request: &str, judge: &str) {
    let findings: Vec<policy::Finding> = Vec::new();
    let accepted = policy::validate_record_response(protocol, request, judge, &WireStatus::Pass, 0,
        protocol, request, judge, findings.as_slice());
    assert(accepted.is_ok());
    expect_status_true(accepted.is_ok());
}

fn record_header_errors_precede_status(a: &str, b: &str)
    requires a@ != b@,
{
    let findings = vec![complete_finding("warning", "/a")];
    let protocol = policy::validate_record_response(b, a, a, &WireStatus::Pass, 0,
        a, a, a, findings.as_slice());
    expect_response_error(protocol, "response identity/status/exit mismatch");
    let request = policy::validate_record_response(a, b, a, &WireStatus::Pass, 0,
        a, a, a, findings.as_slice());
    expect_response_error(request, "response identity/status/exit mismatch");
    let judge = policy::validate_record_response(a, a, b, &WireStatus::Pass, 0,
        a, a, a, findings.as_slice());
    expect_response_error(judge, "response identity/status/exit mismatch");
    let negative = policy::validate_record_response(a, a, a, &WireStatus::Pass, i32::MIN,
        a, a, a, findings.as_slice());
    expect_response_error(negative, "response identity/status/exit mismatch");
    let large = policy::validate_record_response(a, a, a, &WireStatus::Pass, i32::MAX,
        a, a, a, findings.as_slice());
    expect_response_error(large, "response identity/status/exit mismatch");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_response_keeps_exact_headers_and_error_priority() {
    exact_record_header_accepts_empty_pass("", "", "");
    exact_record_header_accepts_empty_pass("协议\0", "é", "e\u{301}");
    proof { reveal_strlit("a"); reveal_strlit("b"); assert("a"@[0] != "b"@[0]); }
    record_header_errors_precede_status("a", "b");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_response_keeps_four_success_states() {
    proof { response_fixture_texts(); }
    let info = vec![complete_finding("info", "/a")];
    let warning = vec![complete_finding("warning", "/a")];
    let error = vec![complete_finding("error", "/a")];
    let pass = policy::validate_record_response("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", info.as_slice());
    let warn = policy::validate_record_response("p", "r", "j", &WireStatus::Warn, 0,
        "p", "r", "j", warning.as_slice());
    let fail = policy::validate_record_response("p", "r", "j", &WireStatus::Fail, 1,
        "p", "r", "j", error.as_slice());
    let failure = policy::validate_record_response("p", "r", "j", &WireStatus::Error, 2,
        "p", "r", "j", error.as_slice());
    assert(pass.is_ok() && warn.is_ok() && fail.is_ok() && failure.is_ok());
    expect_status_true(pass.is_ok() && warn.is_ok() && fail.is_ok() && failure.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_response_preserves_exact_status_errors() {
    proof { response_fixture_texts(); }
    let info = vec![complete_finding("info", "/a")];
    let warning = vec![complete_finding("warning", "/a")];
    let bad_pass = policy::validate_record_response("p", "r", "j", &WireStatus::Pass, 0,
        "p", "r", "j", warning.as_slice());
    expect_response_error(bad_pass, "response findings/status disagree");
    let bad_warn = policy::validate_record_response("p", "r", "j", &WireStatus::Warn, 0,
        "p", "r", "j", info.as_slice());
    expect_response_error(bad_warn, "response findings/status disagree");
    let bad_fail = policy::validate_record_response("p", "r", "j", &WireStatus::Fail, 1,
        "p", "r", "j", warning.as_slice());
    expect_response_error(bad_fail, "response findings/status disagree");
    let bad_error = policy::validate_record_response("p", "r", "j", &WireStatus::Error, 2,
        "p", "r", "j", info.as_slice());
    expect_response_error(bad_error, "response findings/status disagree");
}

fn expect_index_stage_duplicate(out: Result<(), String>)
    requires match out {
        Err(message) => message@ == "duplicate Git index stage"@,
        Ok(()) => false,
    },
{
    match out {
        Err(message) => { expect_status_text(message.as_str(), "duplicate Git index stage"); },
        Ok(()) => { panic!(); },
    }
}

fn expect_index_stage_slot<'a>(stages: &policy::GitIndexStages<'a>, path: &'a str, stage: &'a str)
    requires stages@.contains_key(policy::GitIndexStage { path, stage }),
{
    proof { policy::git_index_stage_obeys_cmp(); }
    expect_status_true(stages.contains_key(&policy::GitIndexStage { path, stage }));
}

fn expect_index_lookup(stages: &policy::GitIndexStages<'_>, path: &str, stage: &str, expected: bool)
    requires expected == (exists|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
        && key.path.spec_bytes() == path.spec_bytes() && key.stage.spec_bytes() == stage.spec_bytes()),
{
    let lookup = policy::snapshot_git_index_stages(stages);
    let actual = lookup.contains(path, stage);
    expect_status_true(actual == expected);
    proof { policy::git_index_stage_obeys_cmp(); }
    expect_status_true(lookup.len() == stages.len());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_lookup_empty_rejects_every_text_key() {
    let stages = policy::GitIndexStages::new();
    expect_index_lookup(&stages, "", "", false);
    expect_index_lookup(&stages, "数据\0path", "0", false);
}

fn index_lookup_keeps_full_path_and_stage_text(path: &str) {
    let mut stages = policy::GitIndexStages::new();
    let out = policy::record_git_index_stage(&mut stages, path, "1");
    expect_status_true(out.is_ok());
    let out = policy::record_git_index_stage(&mut stages, path, "3");
    proof {
        reveal_strlit("0"); reveal_strlit("1"); reveal_strlit("2"); reveal_strlit("3");
        reveal_strlit("00"); reveal_strlit("1\0");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("0") && vstd::string::is_ascii("1")
            && vstd::string::is_ascii("2") && vstd::string::is_ascii("3")
            && vstd::string::is_ascii("00") && vstd::string::is_ascii("1\0"));
        assert("0".spec_bytes() == seq![48u8]);
        assert("1".spec_bytes() == seq![49u8]);
        assert("2".spec_bytes() == seq![50u8]);
        assert("3".spec_bytes() == seq![51u8]);
        assert("00".spec_bytes() == seq![48u8, 48u8]);
        assert("1\0".spec_bytes() == seq![49u8, 0u8]);
    }
    expect_status_true(out.is_ok());
    let copied = path.to_owned();
    let copied_path = copied.as_str();
    proof {
        assert(copied_path.spec_bytes() == path.spec_bytes());
        assert(stages@.contains_key(policy::GitIndexStage { path, stage: "1" }));
        assert(stages@.contains_key(policy::GitIndexStage { path, stage: "3" }));
        assert(exists|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
            && key.path.spec_bytes() == copied_path.spec_bytes() && key.stage.spec_bytes() == "1".spec_bytes());
        assert(exists|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
            && key.path.spec_bytes() == copied_path.spec_bytes() && key.stage.spec_bytes() == "3".spec_bytes());
        assert forall|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
            implies key == policy::GitIndexStage { path, stage: "1" }
                || key == policy::GitIndexStage { path, stage: "3" } by {};
        assert forall|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
            implies key.stage.spec_bytes() != "2".spec_bytes() by {
            assert(key == policy::GitIndexStage { path, stage: "1" }
                || key == policy::GitIndexStage { path, stage: "3" });
            assert(key.stage@ == "1"@ || key.stage@ == "3"@);
            assert(vstd::string::is_ascii(key.stage));
            vstd::string::is_ascii_spec_bytes(key.stage);
            assert(key.stage.spec_bytes()[0] == 49 || key.stage.spec_bytes()[0] == 51);
            assert("2".spec_bytes()[0] == 50);
        }
    }
    expect_index_lookup(&stages, copied_path, "1", true);
    expect_index_lookup(&stages, copied_path, "3", true);
    expect_index_lookup(&stages, copied_path, "0", false);
    expect_index_lookup(&stages, copied_path, "2", false);
    expect_index_lookup(&stages, copied_path, "00", false);
    expect_index_lookup(&stages, copied_path, "1\0", false);
    let out = policy::record_git_index_stage(&mut stages, path, "2");
    expect_status_true(out.is_ok());
    let out = policy::record_git_index_stage(&mut stages, path, "0");
    expect_status_true(out.is_ok());
    proof {
        assert(stages@.contains_key(policy::GitIndexStage { path, stage: "0" }));
        assert(stages@.contains_key(policy::GitIndexStage { path, stage: "1" }));
        assert(stages@.contains_key(policy::GitIndexStage { path, stage: "2" }));
        assert(stages@.contains_key(policy::GitIndexStage { path, stage: "3" }));
        assert(exists|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
            && key.path.spec_bytes() == copied_path.spec_bytes() && key.stage.spec_bytes() == "0".spec_bytes());
        assert(exists|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
            && key.path.spec_bytes() == copied_path.spec_bytes() && key.stage.spec_bytes() == "2".spec_bytes());
        assert(exists|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
            && key.path.spec_bytes() == copied_path.spec_bytes() && key.stage.spec_bytes() == "1".spec_bytes());
        assert(exists|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
            && key.path.spec_bytes() == copied_path.spec_bytes() && key.stage.spec_bytes() == "3".spec_bytes());
    }
    expect_index_lookup(&stages, copied_path, "0", true);
    expect_index_lookup(&stages, copied_path, "1", true);
    expect_index_lookup(&stages, copied_path, "2", true);
    expect_index_lookup(&stages, copied_path, "3", true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_lookup_nonzero_stages_do_not_supply_stage_zero() {
    index_lookup_keeps_full_path_and_stage_text("");
    index_lookup_keeps_full_path_and_stage_text("a");
    index_lookup_keeps_full_path_and_stage_text("数据\0path");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_lookup_distinguishes_full_paths_and_owns_a_stable_snapshot() {
    let mut stages = policy::GitIndexStages::new();
    let out = policy::record_git_index_stage(&mut stages, "a\0", "0");
    expect_status_true(out.is_ok());
    proof {
        reveal_strlit("a"); reveal_strlit("a\0"); reveal_strlit("A\0");
        reveal_strlit("a\0tail"); reveal_strlit("0"); reveal_strlit("3");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("a") && vstd::string::is_ascii("a\0")
            && vstd::string::is_ascii("A\0") && vstd::string::is_ascii("a\0tail")
            && vstd::string::is_ascii("0") && vstd::string::is_ascii("3"));
        assert("a\0".spec_bytes()[0] != "A\0".spec_bytes()[0]);
    }
    expect_index_lookup(&stages, "a", "0", false);
    expect_index_lookup(&stages, "A\0", "0", false);
    expect_index_lookup(&stages, "a\0tail", "0", false);
    let lookup = policy::snapshot_git_index_stages(&stages);
    let copied = "a\0".to_owned();
    let copied_path = copied.as_str();
    proof {
        assert(copied_path.spec_bytes() == "a\0".spec_bytes());
        let key = policy::GitIndexStage { path: "a\0", stage: "0" };
        assert(lookup.view().to_set().contains(key));
        assert(lookup.contains_spec(copied_path, "0"));
    }
    expect_status_true(lookup.contains(copied_path, "0"));
    let out = policy::record_git_index_stage(&mut stages, "a", "3");
    expect_status_true(out.is_ok());
    expect_status_true(!lookup.contains("a", "3"));
    expect_status_true(lookup.len() == 1);
}

// 独立列出五个预期键、数量、完整重复错误及失败后的原集合。
fn index_stage_insertion_keeps_all_paths_and_stages(a: &str, b: &str)
    requires a@ != b@,
{
    proof {
        policy::git_index_stage_obeys_cmp();
        vstd::utf8::encode_utf8_decode_utf8(a@);
        vstd::utf8::encode_utf8_decode_utf8(b@);
        reveal_strlit("0"); reveal_strlit("1"); reveal_strlit("2"); reveal_strlit("3");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("0") && vstd::string::is_ascii("1")
            && vstd::string::is_ascii("2") && vstd::string::is_ascii("3"));
    }
    let copied_a = a.to_owned();
    let copied_b = b.to_owned();
    let mut stages = policy::GitIndexStages::new();
    expect_status_true(stages.is_empty());
    let out = policy::record_git_index_stage(&mut stages, a, "0");
    expect_tree_insert_ok(out);
    let out = policy::record_git_index_stage(&mut stages, a, "1");
    expect_tree_insert_ok(out);
    let out = policy::record_git_index_stage(&mut stages, a, "2");
    expect_tree_insert_ok(out);
    let out = policy::record_git_index_stage(&mut stages, a, "3");
    expect_tree_insert_ok(out);
    let out = policy::record_git_index_stage(&mut stages, b, "0");
    expect_tree_insert_ok(out);
    expect_status_true(stages.len() == 5);
    expect_index_stage_slot(&stages, a, "0");
    expect_index_stage_slot(&stages, a, "1");
    expect_index_stage_slot(&stages, a, "2");
    expect_index_stage_slot(&stages, a, "3");
    expect_index_stage_slot(&stages, b, "0");

    let ghost before = stages@;
    let copied_path = copied_a.as_str();
    proof {
        assert(copied_path.spec_bytes() == a.spec_bytes());
        assert(stages@.contains_key(policy::GitIndexStage { path: a, stage: "0" }));
        assert(exists|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
            && key.path.spec_bytes() == copied_path.spec_bytes()
            && key.stage.spec_bytes() == "0".spec_bytes());
    }
    let out = policy::record_git_index_stage(&mut stages, copied_path, "0");
    expect_index_stage_duplicate(out);
    assert(stages@ == before);
    expect_status_true(stages.len() == 5);
    let copied_path = copied_b.as_str();
    proof {
        assert(copied_path.spec_bytes() == b.spec_bytes());
        assert(stages@.contains_key(policy::GitIndexStage { path: b, stage: "0" }));
        assert(exists|key: policy::GitIndexStage| (#[trigger] stages@.contains_key(key))
            && key.path.spec_bytes() == copied_path.spec_bytes()
            && key.stage.spec_bytes() == "0".spec_bytes());
    }
    let out = policy::record_git_index_stage(&mut stages, copied_path, "0");
    expect_index_stage_duplicate(out);
    assert(stages@ == before);
    let out = policy::record_git_index_stage(&mut stages, a, "3");
    expect_index_stage_duplicate(out);
    assert(stages@ == before);
    expect_index_stage_slot(&stages, a, "0");
    expect_index_stage_slot(&stages, a, "1");
    expect_index_stage_slot(&stages, a, "2");
    expect_index_stage_slot(&stages, a, "3");
    expect_index_stage_slot(&stages, b, "0");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_stage_state_preserves_full_identity_and_duplicate_state() {
    proof {
        reveal_strlit(""); reveal_strlit("a"); reveal_strlit("a\0");
        reveal_strlit("目录/é"); reveal_strlit("目录/e\u{301}");
        assert(""@.len() != "a"@.len());
        assert("a"@.len() != "a\0"@.len());
        assert("目录/é"@.len() != "目录/e\u{301}"@.len());
    }
    index_stage_insertion_keeps_all_paths_and_stages("", "a");
    index_stage_insertion_keeps_all_paths_and_stages("a", "a\0");
    index_stage_insertion_keeps_all_paths_and_stages("目录/é", "目录/e\u{301}");
}

fn index_stage_path_order_precedes_stage(a: &str, b: &str)
    requires scope_text::byte_lt(a.spec_bytes(), b.spec_bytes()),
{
    proof {
        scope_text::byte_lt_strict(a.spec_bytes(), b.spec_bytes());
        scope_text::byte_lt_asymmetric(a.spec_bytes(), b.spec_bytes());
    }
    let left = policy::GitIndexStage { path: a, stage: "3" };
    let right = policy::GitIndexStage { path: b, stage: "0" };
    expect_status_true(matches!(left.cmp(&right), Ordering::Less));
    expect_status_true(matches!(right.cmp(&left), Ordering::Greater));
    expect_status_true(!left.eq(&right));
}

fn index_stage_orders_complete_stage_after_equal_path(path: &str, a: &str, b: &str)
    requires scope_text::byte_lt(a.spec_bytes(), b.spec_bytes()),
{
    proof {
        scope_text::byte_lt_strict(a.spec_bytes(), b.spec_bytes());
        scope_text::byte_lt_asymmetric(a.spec_bytes(), b.spec_bytes());
    }
    let left = policy::GitIndexStage { path, stage: a };
    let right = policy::GitIndexStage { path, stage: b };
    expect_status_true(matches!(left.cmp(&right), Ordering::Less));
    expect_status_true(matches!(right.cmp(&left), Ordering::Greater));
    expect_status_true(!left.eq(&right));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_stage_order_keeps_path_priority_and_full_stage_bytes() {
    proof {
        reveal_strlit(""); reveal_strlit("a"); reveal_strlit("a\0");
        reveal_strlit("a0"); reveal_strlit("0"); reveal_strlit("0\0"); reveal_strlit("1");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("a")
            && vstd::string::is_ascii("a\0") && vstd::string::is_ascii("a0")
            && vstd::string::is_ascii("0") && vstd::string::is_ascii("0\0")
            && vstd::string::is_ascii("1"));
        assert(scope_text::byte_prefix("".spec_bytes(), "a".spec_bytes(), 0));
        assert(scope_text::byte_prefix("a".spec_bytes(), "a\0".spec_bytes(), 1));
        assert(scope_text::byte_prefix("a\0".spec_bytes(), "a0".spec_bytes(), 1));
        assert(scope_text::byte_prefix("0".spec_bytes(), "0\0".spec_bytes(), 1));
        assert(scope_text::byte_prefix("0\0".spec_bytes(), "1".spec_bytes(), 0));
        assert(scope_text::byte_lt("".spec_bytes(), "a".spec_bytes()));
        assert(scope_text::byte_lt("a".spec_bytes(), "a\0".spec_bytes()));
        assert(scope_text::byte_lt("a\0".spec_bytes(), "a0".spec_bytes()));
        assert(scope_text::byte_lt("0".spec_bytes(), "0\0".spec_bytes()));
        assert(scope_text::byte_lt("0\0".spec_bytes(), "1".spec_bytes()));
    }
    index_stage_path_order_precedes_stage("", "a");
    index_stage_path_order_precedes_stage("a", "a\0");
    index_stage_path_order_precedes_stage("a\0", "a0");
    index_stage_orders_complete_stage_after_equal_path("目录/é", "0", "0\0");
    index_stage_orders_complete_stage_after_equal_path("目录/é", "0\0", "1");
}

fn expect_index_row_message(raw: &str, expected: &str)
    requires policy::git_index_row_spec(raw.spec_bytes()) ==
        Err::<policy::GitIndexRowBytes, policy::GitInventoryErrorView>(
            policy::GitInventoryErrorView::Message(expected@)),
{
    match policy::parse_git_index_row(raw) {
        Err(policy::GitInventoryError::Message(message)) => {
            expect_status_text(message.as_str(), expected);
        },
        _ => { panic!(); },
    }
}

// 显式固定分隔位置和字段，预期不由生产解析器生成。
proof fn index_fixture_row(raw: &str, path: &str, stage: &str)
    requires
        vstd::string::is_ascii(raw), vstd::string::is_ascii(path), vstd::string::is_ascii(stage),
        stage@.len() == 1, '0' <= stage@[0] <= '3',
        raw@ == "100644 0000000000000000000000000000000000000001 "@ + stage@ + "\t"@ + path@,
    ensures policy::git_index_row_spec(raw.spec_bytes()) ==
        if policy::paths::relative_path(path.spec_bytes()) {
            Ok((path.spec_bytes(), "100644".spec_bytes(),
                "0000000000000000000000000000000000000001".spec_bytes(), stage.spec_bytes()))
        } else { Err(policy::GitInventoryErrorView::InvalidPath(path.spec_bytes())) },
{
    reveal(policy::git_index_row_spec);
    reveal_strlit("100644 0000000000000000000000000000000000000001 ");
    reveal_strlit("100644"); reveal_strlit("\t");
    reveal_strlit("0000000000000000000000000000000000000001");
    broadcast use vstd::string::is_ascii_spec_bytes;
    let oid = "0000000000000000000000000000000000000001";
    let ghost meta = raw.spec_bytes().take(49);
    let ghost tail = meta.skip(7);
    assert(vstd::string::is_ascii(oid) && vstd::string::is_ascii("100644"));
    scope_text::ascii_separator_end_is(raw.spec_bytes(), 9, 49);
    assert(raw.spec_bytes().skip(50) =~= path.spec_bytes());
    scope_text::ascii_separator_end_is(meta, 32, 6);
    scope_text::ascii_separator_end_is(tail, 32, 40);
    scope_text::ascii_separator_end_is(stage.spec_bytes(), 32, 1);
    assert(meta.take(6) =~= "100644".spec_bytes());
    assert(tail.take(40) =~= oid.spec_bytes());
    assert(tail.skip(41) =~= stage.spec_bytes());
    assert(stage.spec_bytes().take(1) =~= stage.spec_bytes());
    reveal_with_fuel(scope_text::ascii_split_records, 4);
    assert(scope_text::ascii_split_records(meta, 32, true)
        =~= seq!["100644".spec_bytes(), oid.spec_bytes(), stage.spec_bytes()]);
    assert forall|i: int| 0 <= i < oid.spec_bytes().len()
        implies scope_text::oid_byte(#[trigger] oid.spec_bytes()[i]) by {
        assert(oid.spec_bytes()[i] == 48 || oid.spec_bytes()[i] == 49);
    }
    assert(oid.spec_bytes()[39] == 49);
    assert(scope_text::oid(oid.spec_bytes()));
}

// 完整库存预期由原输入和独立字段给出，不调用生产解析器构造预期。
fn expect_index_inventory_rows(raw: &[&str], expected: &[policy::GitIndexRow])
    requires
        raw.len() == expected.len(), policy::git_index_prefix_valid(raw@, raw.len() as int),
        forall|i: int| 0 <= i < raw.len() ==> policy::git_index_row_at(raw@, i)
            == Ok::<policy::GitIndexRowBytes, policy::GitInventoryErrorView>(expected@[i].bytes()),
{
    proof { policy::git_index_stage_obeys_cmp(); }
    match policy::parse_git_index_rows(raw) {
        Ok(inventory) => {
            expect_status_true(inventory.rows.len() == expected.len());
            expect_status_true(inventory.stages.len() == expected.len());
            let mut i = 0;
            while i < expected.len()
                invariant
                    0 <= i <= expected.len(), raw.len() == expected.len(),
                    policy::git_index_prefix_rows(inventory.rows@, raw@, raw.len() as int),
                    inventory.stages.valid(),
                    policy::git_index_lookup_inventory(inventory.stages.view(), inventory.rows@),
                    forall|j: int| 0 <= j < raw.len() ==> policy::git_index_row_at(raw@, j)
                        == Ok::<policy::GitIndexRowBytes, policy::GitInventoryErrorView>(expected@[j].bytes()),
                decreases expected.len() - i,
            {
                let actual = &inventory.rows[i];
                let wanted = &expected[i];
                proof {
                    assert(policy::git_index_row_at(raw@, i as int)
                        == Ok::<policy::GitIndexRowBytes, policy::GitInventoryErrorView>(actual.bytes()));
                    assert(actual.bytes() == wanted.bytes());
                }
                expect_status_true(scope_text::equal_bytes(actual.path.as_bytes(), wanted.path.as_bytes())
                    && scope_text::equal_bytes(actual.mode.as_bytes(), wanted.mode.as_bytes())
                    && scope_text::equal_bytes(actual.oid.as_bytes(), wanted.oid.as_bytes())
                    && scope_text::equal_bytes(actual.stage.as_bytes(), wanted.stage.as_bytes()));
                proof {
                    assert(inventory.stages.view().to_set().contains(
                        policy::GitIndexStage { path: actual.path, stage: actual.stage }));
                }
                expect_status_true(inventory.stages.contains(actual.path, actual.stage));
                i += 1;
            }
        },
        Err(error) => {
            proof {
                let i = choose|i: int| 0 <= i < raw.len() && policy::git_index_prefix_valid(raw@, i)
                    && policy::git_index_error_at(raw@, i, error.view());
                policy::git_index_error_blocks_prefix(raw@, i, error.view(), raw.len() as int);
            }
            panic!();
        },
    }
}

fn expect_index_inventory_error(raw: &[&str], first: usize, expected: policy::GitInventoryError)
    requires first < raw.len(), policy::git_index_prefix_valid(raw@, first as int),
        policy::git_index_error_at(raw@, first as int, expected.view()),
{
    match policy::parse_git_index_rows(raw) {
        Err(error) => {
            proof {
                let i = choose|i: int| 0 <= i < raw.len() && policy::git_index_prefix_valid(raw@, i)
                    && policy::git_index_error_at(raw@, i, error.view());
                policy::git_index_first_error_unique(raw@, first as int, expected.view(), i, error.view());
            }
            match (error, expected) {
                (policy::GitInventoryError::Message(actual), policy::GitInventoryError::Message(wanted)) => {
                    expect_status_text(actual.as_str(), wanted.as_str());
                },
                (policy::GitInventoryError::InvalidPath(actual), policy::GitInventoryError::InvalidPath(wanted)) => {
                    expect_status_true(scope_text::equal_bytes(actual.as_bytes(), wanted.as_bytes()));
                },
                _ => { panic!(); },
            }
        },
        Ok(_) => {
            proof { policy::git_index_error_blocks_prefix(raw@, first as int, expected.view(), raw.len() as int); }
            panic!();
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_inventory_keeps_empty_input() {
    let raw: Vec<&str> = Vec::new();
    let expected: Vec<policy::GitIndexRow> = Vec::new();
    expect_index_inventory_rows(raw.as_slice(), expected.as_slice());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_inventory_preserves_order_fields_and_all_four_stages() {
    let zero = "100644 0000000000000000000000000000000000000001 0\ta";
    let one = "100644 0000000000000000000000000000000000000001 1\ta";
    let two = "100644 0000000000000000000000000000000000000001 2\ta";
    let three = "100644 0000000000000000000000000000000000000001 3\ta";
    let tail = "100644 0000000000000000000000000000000000000001 0\ta\tpath";
    proof {
        reveal_strlit("100644 0000000000000000000000000000000000000001 0\ta");
        reveal_strlit("100644 0000000000000000000000000000000000000001 1\ta");
        reveal_strlit("100644 0000000000000000000000000000000000000001 2\ta");
        reveal_strlit("100644 0000000000000000000000000000000000000001 3\ta");
        reveal_strlit("100644 0000000000000000000000000000000000000001 0\ta\tpath");
        reveal_strlit("100644 0000000000000000000000000000000000000001 ");
        reveal_strlit("a"); reveal_strlit("a\tpath"); reveal_strlit("\t");
        reveal_strlit("0"); reveal_strlit("1"); reveal_strlit("2"); reveal_strlit("3");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii(zero) && vstd::string::is_ascii(one)
            && vstd::string::is_ascii(two) && vstd::string::is_ascii(three)
            && vstd::string::is_ascii(tail));
        vstd::string::is_ascii_spec_bytes("a"); vstd::string::is_ascii_spec_bytes("a\tpath");
        vstd::string::is_ascii_spec_bytes("0"); vstd::string::is_ascii_spec_bytes("1");
        vstd::string::is_ascii_spec_bytes("2"); vstd::string::is_ascii_spec_bytes("3");
        assert(zero@ =~= "100644 0000000000000000000000000000000000000001 "@ + "0"@ + "\t"@ + "a"@);
        assert(one@ =~= "100644 0000000000000000000000000000000000000001 "@ + "1"@ + "\t"@ + "a"@);
        assert(two@ =~= "100644 0000000000000000000000000000000000000001 "@ + "2"@ + "\t"@ + "a"@);
        assert(three@ =~= "100644 0000000000000000000000000000000000000001 "@ + "3"@ + "\t"@ + "a"@);
        assert(tail@ =~= "100644 0000000000000000000000000000000000000001 "@ + "0"@ + "\t"@ + "a\tpath"@);
        index_fixture_row(zero, "a", "0"); index_fixture_row(one, "a", "1");
        index_fixture_row(two, "a", "2"); index_fixture_row(three, "a", "3");
        index_fixture_row(tail, "a\tpath", "0");
        assert(policy::paths::relative_path("a".spec_bytes()));
        assert(policy::paths::relative_path("a\tpath".spec_bytes()));
        assert("a".spec_bytes().len() != "a\tpath".spec_bytes().len());
        assert("0".spec_bytes()[0] == 48 && "1".spec_bytes()[0] == 49
            && "2".spec_bytes()[0] == 50 && "3".spec_bytes()[0] == 51);
    }
    // 顺序刻意不按 stage 排序；原路径中的第二个 TAB 必须保留。
    let raw = vec![three, zero, tail, two, one];
    let oid = "0000000000000000000000000000000000000001";
    let expected = vec![
        policy::GitIndexRow { path: "a", mode: "100644", oid, stage: "3" },
        policy::GitIndexRow { path: "a", mode: "100644", oid, stage: "0" },
        policy::GitIndexRow { path: "a\tpath", mode: "100644", oid, stage: "0" },
        policy::GitIndexRow { path: "a", mode: "100644", oid, stage: "2" },
        policy::GitIndexRow { path: "a", mode: "100644", oid, stage: "1" }];
    proof {
        assert forall|i: int, j: int| 0 <= i < j < raw.len() implies
            !policy::git_index_same_stage((policy::git_index_row_at(raw@, i)->Ok_0),
                (policy::git_index_row_at(raw@, j)->Ok_0)) by {
            if i == 0 {} else if i == 1 {} else if i == 2 {} else { assert(i == 3); }
            if j == 1 {} else if j == 2 {} else if j == 3 {} else { assert(j == 4); }
        }
    }
    expect_index_inventory_rows(raw.as_slice(), expected.as_slice());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_inventory_duplicate_precedes_later_malformed_row() {
    let zero = "100644 0000000000000000000000000000000000000001 0\ta";
    let copy = zero.to_owned();
    let other_storage = copy.as_str();
    let raw = vec![zero, other_storage, ""];
    proof {
        reveal_strlit("100644 0000000000000000000000000000000000000001 0\ta");
        reveal_strlit("100644 0000000000000000000000000000000000000001 ");
        reveal_strlit("a"); reveal_strlit("0"); reveal_strlit("\t");
        vstd::string::is_ascii_spec_bytes(zero);
        assert(vstd::string::is_ascii(zero) && vstd::string::is_ascii("a") && vstd::string::is_ascii("0"));
        assert(zero@ =~= "100644 0000000000000000000000000000000000000001 "@ + "0"@ + "\t"@ + "a"@);
        index_fixture_row(zero, "a", "0");
        assert(policy::paths::relative_path("a".spec_bytes()));
        assert(other_storage.spec_bytes() == zero.spec_bytes());
        assert(policy::git_index_row_at(raw@, 1) == policy::git_index_row_at(raw@, 0));
        assert(policy::git_index_same_stage((policy::git_index_row_at(raw@, 0)->Ok_0),
            (policy::git_index_row_at(raw@, 1)->Ok_0)));
        assert(exists|j: int| 0 <= j < 1 && policy::git_index_same_stage(
            (policy::git_index_row_at(raw@, j)->Ok_0), (policy::git_index_row_at(raw@, 1)->Ok_0)));
    }
    expect_index_inventory_error(raw.as_slice(), 1,
        policy::GitInventoryError::Message("duplicate Git index stage".to_owned()));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_inventory_first_row_error_precedes_later_duplicate() {
    let zero = "100644 0000000000000000000000000000000000000001 0\ta";
    let raw = vec!["", zero, zero];
    proof {
        reveal(policy::git_index_row_spec); reveal_strlit("");
        vstd::string::is_ascii_spec_bytes("");
        scope_text::ascii_separator_end_is("".spec_bytes(), 9, 0);
    }
    expect_index_inventory_error(raw.as_slice(), 0,
        policy::GitInventoryError::Message("invalid Git index row".to_owned()));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_inventory_keeps_invalid_path_before_oid_and_later_rows() {
    let raw = vec!["m x 0\t/a\0suffix", "m x 0\ta", ""];
    proof {
        reveal_strlit("m x 0\t/a\0suffix"); reveal_strlit("m x 0\t"); reveal_strlit("/a\0suffix");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("m x 0\t/a\0suffix") && vstd::string::is_ascii("/a\0suffix"));
        assert("m x 0\t/a\0suffix"@ =~= "m x 0\t"@ + "/a\0suffix"@);
        invalid_oid_index_fixture("m x 0\t/a\0suffix", "/a\0suffix");
        assert(!policy::paths::relative_path("/a\0suffix".spec_bytes()));
    }
    expect_index_inventory_error(raw.as_slice(), 0, policy::GitInventoryError::InvalidPath("/a\0suffix"));
}

fn expect_index_row_fields(raw: &str, path: &str, mode: &str, oid: &str, stage: &str)
    requires policy::git_index_row_spec(raw.spec_bytes()) ==
        Ok::<policy::GitIndexRowBytes, policy::GitInventoryErrorView>(
            (path.spec_bytes(), mode.spec_bytes(), oid.spec_bytes(), stage.spec_bytes())),
{
    match policy::parse_git_index_row(raw) {
        Ok(row) => {
            let same = scope_text::equal_bytes(row.path.as_bytes(), path.as_bytes())
                && scope_text::equal_bytes(row.mode.as_bytes(), mode.as_bytes())
                && scope_text::equal_bytes(row.oid.as_bytes(), oid.as_bytes())
                && scope_text::equal_bytes(row.stage.as_bytes(), stage.as_bytes());
            expect_status_true(same);
        },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_parser_preserves_all_fields_and_stage_boundaries() {
    let zero = "100644 0000000000000000000000000000000000000001 0\ta\tpath";
    let three = "100644 0000000000000000000000000000000000000001 3\ta\tpath";
    proof {
        reveal_strlit("100644 0000000000000000000000000000000000000001 0\ta\tpath");
        reveal_strlit("100644 0000000000000000000000000000000000000001 3\ta\tpath");
        reveal_strlit("100644 0000000000000000000000000000000000000001 ");
        reveal_strlit("a\tpath"); reveal_strlit("0"); reveal_strlit("3"); reveal_strlit("\t");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii(zero) && vstd::string::is_ascii(three)
            && vstd::string::is_ascii("a\tpath") && vstd::string::is_ascii("0")
            && vstd::string::is_ascii("3"));
        assert(zero@ =~= "100644 0000000000000000000000000000000000000001 "@ + "0"@ + "\t"@ + "a\tpath"@);
        assert(three@ =~= "100644 0000000000000000000000000000000000000001 "@ + "3"@ + "\t"@ + "a\tpath"@);
        index_fixture_row(zero, "a\tpath", "0");
        index_fixture_row(three, "a\tpath", "3");
        assert(policy::paths::relative_path("a\tpath".spec_bytes()));
    }
    expect_index_row_fields(zero, "a\tpath", "100644", "0000000000000000000000000000000000000001", "0");
    expect_index_row_fields(three, "a\tpath", "100644", "0000000000000000000000000000000000000001", "3");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_parser_keeps_row_and_metadata_errors_before_path_and_oid() {
    proof {
        reveal(policy::git_index_row_spec);
        reveal_strlit(""); reveal_strlit("bad\t/"); reveal_strlit("bad");
        reveal_strlit("m x 4\t/"); reveal_strlit("m x 4"); reveal_strlit("x 4");
        reveal_strlit("m"); reveal_strlit("x"); reveal_strlit("4");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("bad\t/")
            && vstd::string::is_ascii("m x 4\t/"));
        assert(vstd::string::is_ascii("bad") && vstd::string::is_ascii("m x 4")
            && vstd::string::is_ascii("x 4") && vstd::string::is_ascii("4")
            && vstd::string::is_ascii("m") && vstd::string::is_ascii("x"));
        scope_text::ascii_separator_end_is("".spec_bytes(), 9, 0);
        scope_text::ascii_separator_end_is("bad\t/".spec_bytes(), 9, 3);
        scope_text::ascii_separator_end_is("bad".spec_bytes(), 32, 3);
        scope_text::ascii_separator_end_is("m x 4\t/".spec_bytes(), 9, 5);
        scope_text::ascii_separator_end_is("m x 4".spec_bytes(), 32, 1);
        scope_text::ascii_separator_end_is("x 4".spec_bytes(), 32, 1);
        scope_text::ascii_separator_end_is("4".spec_bytes(), 32, 1);
        assert("m x 4".spec_bytes().take(1) =~= "m".spec_bytes());
        assert("m x 4".spec_bytes().skip(2) =~= "x 4".spec_bytes());
        assert("x 4".spec_bytes().take(1) =~= "x".spec_bytes());
        assert("x 4".spec_bytes().skip(2) =~= "4".spec_bytes());
        assert("m x 4\t/".spec_bytes().take(5) =~= "m x 4".spec_bytes());
        assert("bad\t/".spec_bytes().take(3) =~= "bad".spec_bytes());
        reveal_with_fuel(scope_text::ascii_split_records, 4);
    }
    expect_index_row_message("", "invalid Git index row");
    expect_index_row_message("bad\t/", "invalid Git index metadata");
    expect_index_row_message("m x 4\t/", "invalid Git index metadata");
}

proof fn invalid_oid_index_fixture(raw: &str, path: &str)
    requires vstd::string::is_ascii(raw), vstd::string::is_ascii(path),
        raw@ == "m x 0\t"@ + path@,
    ensures policy::git_index_row_spec(raw.spec_bytes()) ==
        if policy::paths::relative_path(path.spec_bytes()) {
            Err::<policy::GitIndexRowBytes, policy::GitInventoryErrorView>(
                policy::GitInventoryErrorView::Message("expected full nonzero lowercase commit OID"@))
        } else { Err(policy::GitInventoryErrorView::InvalidPath(path.spec_bytes())) },
{
    reveal(policy::git_index_row_spec);
    reveal_strlit("m x 0\t"); reveal_strlit("m x 0"); reveal_strlit("x 0");
    reveal_strlit("m"); reveal_strlit("x"); reveal_strlit("0");
    broadcast use vstd::string::is_ascii_spec_bytes;
    assert(vstd::string::is_ascii("m x 0") && vstd::string::is_ascii("x 0")
        && vstd::string::is_ascii("m") && vstd::string::is_ascii("x")
        && vstd::string::is_ascii("0"));
    scope_text::ascii_separator_end_is(raw.spec_bytes(), 9, 5);
    scope_text::ascii_separator_end_is("m x 0".spec_bytes(), 32, 1);
    scope_text::ascii_separator_end_is("x 0".spec_bytes(), 32, 1);
    scope_text::ascii_separator_end_is("0".spec_bytes(), 32, 1);
    assert(raw.spec_bytes().take(5) =~= "m x 0".spec_bytes());
    assert(raw.spec_bytes().skip(6) =~= path.spec_bytes());
    assert("m x 0".spec_bytes().take(1) =~= "m".spec_bytes());
    assert("m x 0".spec_bytes().skip(2) =~= "x 0".spec_bytes());
    assert("x 0".spec_bytes().take(1) =~= "x".spec_bytes());
    assert("x 0".spec_bytes().skip(2) =~= "0".spec_bytes());
    reveal_with_fuel(scope_text::ascii_split_records, 4);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_parser_keeps_path_error_before_oid_and_preserves_exact_error() {
    proof {
        reveal_strlit("m x 0\t"); reveal_strlit("m x 0\t/"); reveal_strlit("m x 0\ta");
        reveal_strlit("/"); reveal_strlit("a");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("m x 0\t/") && vstd::string::is_ascii("m x 0\ta")
            && vstd::string::is_ascii("/") && vstd::string::is_ascii("a"));
        assert("m x 0\t/"@ =~= "m x 0\t"@ + "/"@);
        assert("m x 0\ta"@ =~= "m x 0\t"@ + "a"@);
        invalid_oid_index_fixture("m x 0\t/", "/");
        invalid_oid_index_fixture("m x 0\ta", "a");
        assert(!policy::paths::relative_path("/".spec_bytes()));
        assert(policy::paths::relative_path("a".spec_bytes()));
    }
    match policy::parse_git_index_row("m x 0\t/") {
        Err(policy::GitInventoryError::InvalidPath(path)) => {
            let same = scope_text::equal_bytes(path.as_bytes(), "/".as_bytes());
            expect_status_true(same);
        },
        _ => { panic!(); },
    }
    expect_index_row_message("m x 0\ta", "expected full nonzero lowercase commit OID");
}

fn index_entry_uses_complete_mode_oid_and_ignores_path_kind(a: &str, b: &str)
    requires a@ != b@,
{
    let row = policy::GitIndexRow { path: b, mode: a, oid: a, stage: "0" };
    let same = policy::TreeEntry { mode: a.to_owned(), oid: a.to_owned(), kind: b.to_owned() };
    expect_status_true(!policy::git_index_entry_changed(&row, Some(&same)));
    expect_status_true(policy::git_index_entry_changed(&row, None));
    let other_mode = policy::TreeEntry { mode: b.to_owned(), oid: a.to_owned(), kind: b.to_owned() };
    let other_oid = policy::TreeEntry { mode: a.to_owned(), oid: b.to_owned(), kind: b.to_owned() };
    expect_status_true(policy::git_index_entry_changed(&row, Some(&other_mode)));
    expect_status_true(policy::git_index_entry_changed(&row, Some(&other_oid)));
}

fn checkout_rejects_other_kind(kind: &str)
    requires kind@ != "blob"@,
{
    expect_status_true(!policy::checkout_entry_supported(kind, "100644"));
    expect_status_true(!policy::checkout_entry_supported(kind, "100755"));
    expect_status_true(!policy::checkout_entry_supported(kind, "120000"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_support_preserves_complete_kind_and_mode() {
    proof {
        reveal_strlit(""); reveal_strlit("blob"); reveal_strlit("Blob");
        reveal_strlit("blob\0"); reveal_strlit("blob/目录");
        reveal_strlit("100644"); reveal_strlit("100755"); reveal_strlit("120000");
        reveal_strlit("100644\0"); reveal_strlit("0100644"); reveal_strlit("100755\n");
        reveal_strlit("120000/é"); reveal_strlit("160000");
        assert(""@.len() != "blob"@.len());
        assert("Blob"@[0] != "blob"@[0]);
        assert("blob\0"@.len() != "blob"@.len());
        assert("blob/目录"@.len() != "blob"@.len());
        assert("100644"@[1] != "120000"@[1]);
        assert("100755"@[1] != "120000"@[1]);
        assert("160000"@[1] != "100644"@[1]);
        assert("160000"@[1] != "100755"@[1]);
        assert("160000"@[1] != "120000"@[1]);
        assert(""@.len() != "100644"@.len());
        assert("100644\0"@.len() != "100644"@.len());
        assert("0100644"@.len() != "100644"@.len());
        assert("100755\n"@.len() != "100644"@.len());
        assert("120000/é"@.len() != "100644"@.len());
    }
    expect_status_true(policy::checkout_entry_supported("blob", "100644"));
    expect_status_true(policy::checkout_entry_supported("blob", "100755"));
    expect_status_true(policy::checkout_entry_supported("blob", "120000"));
    checkout_rejects_other_kind("");
    checkout_rejects_other_kind("Blob");
    checkout_rejects_other_kind("blob\0");
    checkout_rejects_other_kind("blob/目录");
    expect_status_true(!policy::checkout_entry_supported("blob", ""));
    expect_status_true(!policy::checkout_entry_supported("blob", "100644\0"));
    expect_status_true(!policy::checkout_entry_supported("blob", "0100644"));
    expect_status_true(!policy::checkout_entry_supported("blob", "100755\n"));
    expect_status_true(!policy::checkout_entry_supported("blob", "120000/é"));
    expect_status_true(!policy::checkout_entry_supported("blob", "160000"));
    expect_status_true(policy::checkout_symbolic_link("120000"));
    expect_status_true(!policy::checkout_symbolic_link("100644"));
    expect_status_true(!policy::checkout_symbolic_link("100755"));
    expect_status_true(!policy::checkout_symbolic_link("120000/é"));
    expect_status_true(!policy::checkout_symbolic_link(""));
}

fn checkout_non_file_never_matches(permissions: u32, mode: &str) {
    expect_status_true(!policy::checkout_regular_file_matches(false, permissions, mode));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_regular_permissions_keep_only_owner_execute_decision() {
    proof {
        reveal_strlit("100644"); reveal_strlit("100755");
        reveal_strlit("100755\0"); reveal_strlit("");
        assert("100644"@[3] != "100755"@[3]);
        assert("100755\0"@.len() != "100755"@.len());
        assert(""@.len() != "100755"@.len());
        assert((0u32 & 0o100) == 0) by(bit_vector);
        assert((0o644u32 & 0o100) == 0) by(bit_vector);
        assert((0o011u32 & 0o100) == 0) by(bit_vector);
        assert((0o100u32 & 0o100) != 0) by(bit_vector);
        assert((0o755u32 & 0o100) != 0) by(bit_vector);
        assert((u32::MAX & 0o100) != 0) by(bit_vector);
    }
    expect_status_true(policy::checkout_regular_file_matches(true, 0, "100644"));
    expect_status_true(policy::checkout_regular_file_matches(true, 0o644, "100644"));
    expect_status_true(policy::checkout_regular_file_matches(true, 0o011, "100644"));
    expect_status_true(!policy::checkout_regular_file_matches(true, 0o100, "100644"));
    expect_status_true(!policy::checkout_regular_file_matches(true, 0o755, "100644"));
    expect_status_true(!policy::checkout_regular_file_matches(true, 0, "100755"));
    expect_status_true(!policy::checkout_regular_file_matches(true, 0o644, "100755"));
    expect_status_true(policy::checkout_regular_file_matches(true, 0o100, "100755"));
    expect_status_true(policy::checkout_regular_file_matches(true, 0o755, "100755"));
    expect_status_true(policy::checkout_regular_file_matches(true, u32::MAX, "100755"));
    expect_status_true(!policy::checkout_regular_file_matches(true, u32::MAX, "100755\0"));
    expect_status_true(policy::checkout_regular_file_matches(true, 0, "100755\0"));
    expect_status_true(policy::checkout_regular_file_matches(true, 0, ""));
    checkout_non_file_never_matches(0, "100644");
    checkout_non_file_never_matches(0o100, "100755");
    checkout_non_file_never_matches(u32::MAX, "100755\0");
}

fn checkout_open_identity_cases(size: u64, other_size: u64, permissions: u32, other_permissions: u32)
    requires size != other_size, permissions != other_permissions,
{
    expect_status_true(policy::checkout_opened_file_matches(true, size, permissions, size, permissions));
    expect_status_true(!policy::checkout_opened_file_matches(false, size, permissions, size, permissions));
    expect_status_true(!policy::checkout_opened_file_matches(true, other_size, permissions, size, permissions));
    expect_status_true(!policy::checkout_opened_file_matches(true, size, other_permissions, size, permissions));
    expect_status_true(!policy::checkout_opened_file_matches(true, other_size, other_permissions, size, permissions));
    expect_status_true(!policy::checkout_opened_file_matches(false, other_size, permissions, size, permissions));
    expect_status_true(!policy::checkout_opened_file_matches(false, size, other_permissions, size, permissions));
    expect_status_true(!policy::checkout_opened_file_matches(false, other_size, other_permissions, size, permissions));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_open_identity_requires_full_permissions_and_size() {
    checkout_open_identity_cases(0, u64::MAX, 0, u32::MAX);
    checkout_open_identity_cases(u64::MAX, 0, u32::MAX, 0);
    checkout_open_identity_cases(7, 8, 0o644, 0o600);
    checkout_open_identity_cases(7, 8, 0o755, 0o754);
}

fn expect_checkout_hash_count(out: Result<u64, String>, expected: u64)
    requires out matches Ok(value) && value == expected,
{
    match out { Ok(value) => expect_status_true(value == expected), Err(_) => panic!() }
}

fn expect_checkout_hash_error(out: Result<u64, String>, expected: &str)
    requires out matches Err(message) && message@ == expected@,
{
    match out {
        Err(message) => expect_status_text(message.as_str(), expected),
        Ok(_) => panic!(),
    }
}

fn expect_checkout_hash_finished(out: Result<(), String>, expected: &str)
    requires out matches Err(message) && message@ == expected@,
{
    match out {
        Err(message) => expect_status_text(message.as_str(), expected),
        Ok(()) => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_hash_count_preserves_limits_and_error_priority() {
    expect_checkout_hash_count(policy::checkout_hash_advance(0, 0, 0), 0);
    expect_checkout_hash_count(policy::checkout_hash_advance(0, 8, 3), 3);
    expect_checkout_hash_count(policy::checkout_hash_advance(3, 8, 5), 8);
    expect_checkout_hash_count(policy::checkout_hash_advance(u64::MAX, u64::MAX, 0), u64::MAX);
    expect_checkout_hash_count(policy::checkout_hash_advance(u64::MAX - 1, u64::MAX, 1), u64::MAX);
    expect_checkout_hash_error(policy::checkout_hash_advance(0, 0, 1), "checkout file grew during observation");
    expect_checkout_hash_error(policy::checkout_hash_advance(8, 8, 1), "checkout file grew during observation");
    expect_checkout_hash_error(policy::checkout_hash_advance(9, 8, 0), "checkout file grew during observation");
    expect_checkout_hash_error(policy::checkout_hash_advance(u64::MAX - 1, u64::MAX - 1, 1),
        "checkout file grew during observation");
    expect_checkout_hash_error(policy::checkout_hash_advance(u64::MAX, u64::MAX, 1), "checkout file size overflow");
    expect_checkout_hash_error(policy::checkout_hash_advance(u64::MAX, 0, 1), "checkout file size overflow");
    expect_checkout_hash_error(policy::checkout_hash_advance(1, 0, u64::MAX), "checkout file size overflow");
    expect_checkout_hash_error(policy::checkout_hash_advance(u64::MAX, u64::MAX, u64::MAX),
        "checkout file size overflow");
    let out = policy::checkout_hash_complete(0, 0);
    expect_tree_insert_ok(out);
    let out = policy::checkout_hash_complete(u64::MAX, u64::MAX);
    expect_tree_insert_ok(out);
    expect_checkout_hash_finished(policy::checkout_hash_complete(0, 1), "checkout file shrank during observation");
    expect_checkout_hash_finished(policy::checkout_hash_complete(u64::MAX - 1, u64::MAX),
        "checkout file shrank during observation");
    expect_checkout_hash_finished(policy::checkout_hash_complete(u64::MAX, u64::MAX - 1),
        "checkout file shrank during observation");
}

fn index_nonzero_stage_always_changes(stage: &str, text: &str)
    requires stage@ != "0"@,
{
    let row = policy::GitIndexRow { path: text, mode: text, oid: text, stage };
    let same = policy::TreeEntry { mode: text.to_owned(), oid: text.to_owned(), kind: text.to_owned() };
    expect_status_true(policy::git_index_entry_changed(&row, None));
    expect_status_true(policy::git_index_entry_changed(&row, Some(&same)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_entry_matching_preserves_complete_identity_and_stage_decision() {
    proof {
        reveal_strlit(""); reveal_strlit("a"); reveal_strlit("a\0");
        reveal_strlit("é"); reveal_strlit("e\u{301}");
        reveal_strlit("0"); reveal_strlit("1"); reveal_strlit("2"); reveal_strlit("3");
        assert("a"@.len() != "a\0"@.len());
        assert("é"@.len() != "e\u{301}"@.len());
        assert("1"@[0] != "0"@[0]);
        assert("2"@[0] != "0"@[0]);
        assert("3"@[0] != "0"@[0]);
    }
    index_entry_uses_complete_mode_oid_and_ignores_path_kind("", "a");
    index_entry_uses_complete_mode_oid_and_ignores_path_kind("a", "a\0");
    index_entry_uses_complete_mode_oid_and_ignores_path_kind("é", "e\u{301}");
    index_nonzero_stage_always_changes("1", "");
    index_nonzero_stage_always_changes("2", "a\0");
    index_nonzero_stage_always_changes("3", "目录/é");
}

}
