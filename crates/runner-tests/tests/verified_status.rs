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


// 记录实际回调参数和严格次序；此 fixture 不冒充 Serde JSON 对象的内部模型。
struct SelectionProjectionFixture {
    phase: u8,
    retained: String,
    removed: String,
    field: String,
    value: String,
}

fn selection_projection_remove(state: SelectionProjectionFixture, key: &str)
    -> (out: SelectionProjectionFixture)
    requires state.phase == 0,
    ensures out.phase == 1, out.retained == state.retained,
        out.removed@ == key@, out.field == state.field, out.value == state.value,
{
    SelectionProjectionFixture {
        phase: 1, retained: state.retained, removed: key.to_owned(),
        field: state.field, value: state.value,
    }
}

fn selection_projection_insert(state: SelectionProjectionFixture, key: &str, value: &str)
    -> (out: SelectionProjectionFixture)
    requires state.phase == 1,
    ensures out.phase == 2, out.retained == state.retained,
        out.removed == state.removed, out.field@ == key@, out.value@ == value@,
{
    SelectionProjectionFixture {
        phase: 2, retained: state.retained, removed: state.removed,
        field: key.to_owned(), value: value.to_owned(),
    }
}

fn selection_projection_never_remove(state: SelectionProjectionFixture, key: &str)
    -> (out: SelectionProjectionFixture)
    requires false,
    ensures false,
{
    panic!();
}

fn selection_projection_never_insert(state: SelectionProjectionFixture, key: &str, value: &str)
    -> (out: SelectionProjectionFixture)
    requires false,
    ensures false,
{
    panic!();
}

fn selection_projection_fixture(state: SelectionProjectionFixture)
    -> (out: SelectionProjectionFixture)
    requires state.phase == 0,
    ensures out.phase == 2, out.retained == state.retained,
        out.removed@ == "sha256"@, out.field@ == "schema"@,
        out.value@ == "chrono-registry-selection/v1"@,
{
    let result = policy::normalize_registry_selection_object(
        Some(state), selection_projection_remove, selection_projection_insert,
    );
    match result {
        None => { panic!(); },
        Some(out) => {
            proof {
                let (middle, removed, field, value) = choose|
                    middle: SelectionProjectionFixture, removed: &str, field: &str, value: &str|
                    removed@ == "sha256"@ && field@ == "schema"@
                    && value@ == "chrono-registry-selection/v1"@
                    && selection_projection_remove.ensures((state, removed), middle)
                    && selection_projection_insert.ensures((middle, field, value), out);
            }
            out
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selection_projection_without_object_never_calls_either_interface() {
    let out = policy::normalize_registry_selection_object(
        None::<SelectionProjectionFixture>,
        selection_projection_never_remove, selection_projection_never_insert,
    );
    expect_status_true(out.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selection_projection_preserves_empty_metadata_and_exact_schema() {
    let out = selection_projection_fixture(SelectionProjectionFixture {
        phase: 0, retained: "".to_owned(), removed: "old digest".to_owned(),
        field: "old field".to_owned(), value: "old schema".to_owned(),
    });
    expect_status_true(out.phase == 2);
    expect_status_text(out.retained.as_str(), "");
    expect_status_text(out.removed.as_str(), "sha256");
    expect_status_text(out.field.as_str(), "schema");
    expect_status_text(out.value.as_str(), "chrono-registry-selection/v1");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selection_projection_preserves_unicode_nul_and_lf_metadata() {
    let out = selection_projection_fixture(SelectionProjectionFixture {
        phase: 0, retained: "原始/é\0\n完整尾部".to_owned(), removed: "摘要\0\n".to_owned(),
        field: "schema\0suffix".to_owned(), value: "chrono-git-config-selection/v1".to_owned(),
    });
    expect_status_true(out.phase == 2);
    expect_status_text(out.retained.as_str(), "原始/é\0\n完整尾部");
    expect_status_text(out.removed.as_str(), "sha256");
    expect_status_text(out.field.as_str(), "schema");
    expect_status_text(out.value.as_str(), "chrono-registry-selection/v1");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selection_projection_already_matching_schema_still_performs_both_operations() {
    let out = selection_projection_fixture(SelectionProjectionFixture {
        phase: 0, retained: "retained".to_owned(), removed: "sha256".to_owned(),
        field: "schema".to_owned(), value: "chrono-registry-selection/v1".to_owned(),
    });
    expect_status_true(out.phase == 2);
    expect_status_text(out.retained.as_str(), "retained");
    expect_status_text(out.removed.as_str(), "sha256");
    expect_status_text(out.field.as_str(), "schema");
    expect_status_text(out.value.as_str(), "chrono-registry-selection/v1");
}

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

proof fn finding_reference_member_at(entries: &[policy::Delta], at: int, reference: Seq<char>)
    requires 0 <= at < entries.len(), entries[at].path@ == reference,
    ensures policy::reference_allowed(reference, Some(entries)),
{
    let delta = Some(entries);
    match delta {
        Some(rows) => {
            assert(rows == entries);
            assert(rows[at].path@ == reference);
            assert(exists|i: int| 0 <= i < rows.len() && #[trigger] rows[i].path@ == reference) by {
                assert(0 <= at < rows.len() && rows[at].path@ == reference);
            }
            assert(policy::reference_allowed(reference, delta));
        },
        None => {},
    }
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
        finding_reference_member_at(slice, 0, path@);
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
        finding_reference_member_at(slice, 0, "x"@);
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

fn expect_checkout_path(changes: &policy::CheckoutChanges, path: &str, wanted: bool)
    requires wanted == policy::checkout_change_has_path(changes@, path@),
{
    let paths = policy::checkout_changed_paths(changes);
    let mut i: usize = 0;
    let mut found = false;
    while i < paths.len()
        invariant 0 <= i <= paths.len(),
            found == (exists|j: int| 0 <= j < i && (#[trigger] paths@[j])@ == path@),
        decreases paths.len() - i,
    {
        found = found || scope_text::equal_text(paths[i].as_str(), path);
        i += 1;
    }
    expect_status_true(found == wanted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_changed_paths_preserve_full_order_and_exact_cardinality() {
    proof {
        reveal_strlit("a"); reveal_strlit("a\0"); reveal_strlit("A"); reveal_strlit("目录/é");
        assert("a"@.len() != "a\0"@.len()); assert("A"@[0] != "a"@[0]);
        assert("A"@[0] != "a\0"@[0]);
        assert("目录/é"@[0] != "a"@[0] && "目录/é"@[0] != "A"@[0]);
        assert("目录/é"@[0] != "a\0"@[0]);
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("A") && vstd::string::is_ascii("a")
            && vstd::string::is_ascii("a\0"));
        assert('目' as u32 == 30446u32);
        assert(vstd::utf8::encode_scalar(30446u32) == seq![231u8, 155u8, 174u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 2);
        assert("目录/é".spec_bytes()[0] == 231u8);
        assert(scope_text::byte_prefix("A".spec_bytes(), "a".spec_bytes(), 0));
        assert(scope_text::byte_prefix("a".spec_bytes(), "a\0".spec_bytes(), 1));
        assert(scope_text::byte_prefix("a\0".spec_bytes(), "目录/é".spec_bytes(), 0));
        assert(scope_text::byte_lt("A".spec_bytes(), "a".spec_bytes()));
        assert(scope_text::byte_lt("a".spec_bytes(), "a\0".spec_bytes()));
        assert(scope_text::byte_lt("a\0".spec_bytes(), "目录/é".spec_bytes()));
    }
    let mut changes = policy::CheckoutChanges::new();
    policy::record_checkout_change(&mut changes, "目录/é");
    policy::record_checkout_change(&mut changes, "a\0");
    policy::record_checkout_change(&mut changes, "a");
    policy::record_checkout_change(&mut changes, "A");
    let other = "a".to_owned();
    policy::record_checkout_change(&mut changes, other.as_str());
    let paths = policy::checkout_changed_paths(&changes);
    expect_status_true(paths.len() == 4);
    proof {
        assert(policy::checkout_change_has_path(changes@, "A"@));
        assert(policy::checkout_change_has_path(changes@, "a"@));
        assert(policy::checkout_change_has_path(changes@, "a\0"@));
        assert(policy::checkout_change_has_path(changes@, "目录/é"@));
        let a = choose|i: int| 0 <= i < paths.len() && (#[trigger] paths@[i])@ == "A"@;
        let b = choose|i: int| 0 <= i < paths.len() && (#[trigger] paths@[i])@ == "a"@;
        let c = choose|i: int| 0 <= i < paths.len() && (#[trigger] paths@[i])@ == "a\0"@;
        let d = choose|i: int| 0 <= i < paths.len() && (#[trigger] paths@[i])@ == "目录/é"@;
        scope_text::byte_lt_asymmetric("A".spec_bytes(), "a".spec_bytes());
        scope_text::byte_lt_asymmetric("a".spec_bytes(), "a\0".spec_bytes());
        scope_text::byte_lt_asymmetric("a\0".spec_bytes(), "目录/é".spec_bytes());
        assert(a < b && b < c && c < d);
        assert(a == 0 && b == 1 && c == 2 && d == 3);
    }
    expect_status_text(paths[0].as_str(), "A");
    expect_status_text(paths[1].as_str(), "a");
    expect_status_text(paths[2].as_str(), "a\0");
    expect_status_text(paths[3].as_str(), "目录/é");
}

fn expect_normalized_single_path(first: Vec<String>, second: Vec<String>, expected: &str)
    requires first@.len() + second@.len() > 0,
        forall|i: int| 0 <= i < first@.len() + second@.len()
            ==> (#[trigger] (first@ + second@)[i])@ == expected@,
{
    let ghost all = first@ + second@;
    let paths = policy::normalize_checkout_paths(first, second);
    proof {
        assert(policy::checkout_path_prefix(all, all.len() as int, expected@)) by {
            assert(all[0]@ == expected@);
        }
        assert(policy::checkout_path_prefix(paths@, paths.len() as int, expected@));
        assert(paths.len() > 0);
        assert forall|i: int| 0 <= i < paths.len() implies
            (#[trigger] paths@[i])@ == expected@ by {
            assert(policy::checkout_path_prefix(paths@, paths.len() as int, paths@[i]@));
            assert(policy::checkout_path_prefix(all, all.len() as int, paths@[i]@));
            let source = choose|j: int| 0 <= j < all.len() && (#[trigger] all[j])@ == paths@[i]@;
            assert(all[source]@ == expected@);
        }
        if paths.len() > 1 {
            assert(paths@[0]@ == expected@ && paths@[1]@ == expected@);
            assert(scope_text::byte_lt(vstd::utf8::encode_utf8(paths@[0]@),
                vstd::utf8::encode_utf8(paths@[1]@)));
            scope_text::byte_lt_asymmetric(expected.spec_bytes(), expected.spec_bytes());
            assert(scope_text::byte_lt(expected.spec_bytes(), expected.spec_bytes()));
        }
        assert(paths.len() == 1);
    }
    expect_status_true(paths.len() == 1);
    expect_status_text(paths[0].as_str(), expected);
}

fn expect_normalized_pair(left: &str, right: &str)
    requires scope_text::byte_lt(left.spec_bytes(), right.spec_bytes()),
{
    let first = vec![right.to_owned()];
    let second = vec![left.to_owned()];
    let ghost all = first@ + second@;
    let paths = policy::normalize_checkout_paths(first, second);
    proof {
        assert(all.len() == 2 && all[0]@ == right@ && all[1]@ == left@);
        assert(policy::checkout_path_prefix(all, 2, left@));
        assert(policy::checkout_path_prefix(all, 2, right@));
        assert(policy::checkout_path_prefix(paths@, paths.len() as int, left@));
        assert(policy::checkout_path_prefix(paths@, paths.len() as int, right@));
        let a = choose|i: int| 0 <= i < paths.len() && (#[trigger] paths@[i])@ == left@;
        let b = choose|i: int| 0 <= i < paths.len() && (#[trigger] paths@[i])@ == right@;
        scope_text::byte_lt_asymmetric(left.spec_bytes(), right.spec_bytes());
        assert(a < b);
        assert(paths.len() == 2 && a == 0 && b == 1);
    }
    expect_status_true(paths.len() == 2);
    expect_status_text(paths[0].as_str(), left);
    expect_status_text(paths[1].as_str(), right);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_path_normalization_keeps_empty_inventory_empty() {
    let first: Vec<String> = Vec::new();
    let second: Vec<String> = Vec::new();
    let paths = policy::normalize_checkout_paths(first, second);
    expect_status_true(paths.len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_path_normalization_keeps_either_inventory_and_empty_paths() {
    expect_normalized_single_path(vec!["".to_owned()], Vec::new(), "");
    expect_normalized_single_path(Vec::new(), vec!["目录/é\n ".to_owned()], "目录/é\n ");
}

fn expect_cross_inventory_duplicates(path: &str) {
    expect_normalized_single_path(vec![path.to_owned(), path.to_owned()], vec![path.to_owned()], path);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_path_normalization_deduplicates_full_text_across_inventories() {
    expect_cross_inventory_duplicates("");
    expect_cross_inventory_duplicates("a\0");
    expect_cross_inventory_duplicates("目录/é\n ");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_path_normalization_preserves_exact_byte_order_and_prefixes() {
    proof {
        reveal_strlit(""); reveal_strlit("A"); reveal_strlit("a"); reveal_strlit("a\0");
        reveal_strlit("a\n"); reveal_strlit("a "); reveal_strlit("目录/é");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("A")
            && vstd::string::is_ascii("a") && vstd::string::is_ascii("a\0")
            && vstd::string::is_ascii("a\n") && vstd::string::is_ascii("a "));
        assert('目' as u32 == 30446u32);
        assert(vstd::utf8::encode_scalar(30446u32) == seq![231u8, 155u8, 174u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 2);
        assert("目录/é".spec_bytes()[0] == 231u8);
        assert(scope_text::byte_prefix("".spec_bytes(), "A".spec_bytes(), 0));
        assert(scope_text::byte_prefix("A".spec_bytes(), "a".spec_bytes(), 0));
        assert(scope_text::byte_prefix("a".spec_bytes(), "a\0".spec_bytes(), 1));
        assert(scope_text::byte_prefix("a\n".spec_bytes(), "a ".spec_bytes(), 1));
        assert(scope_text::byte_prefix("a\0".spec_bytes(), "目录/é".spec_bytes(), 0));
        assert(scope_text::byte_lt("".spec_bytes(), "A".spec_bytes()));
        assert(scope_text::byte_lt("A".spec_bytes(), "a".spec_bytes()));
        assert(scope_text::byte_lt("a".spec_bytes(), "a\0".spec_bytes()));
        assert(scope_text::byte_lt("a\n".spec_bytes(), "a ".spec_bytes()));
        assert(scope_text::byte_lt("a\0".spec_bytes(), "目录/é".spec_bytes()));
    }
    expect_normalized_pair("", "A");
    expect_normalized_pair("A", "a");
    expect_normalized_pair("a", "a\0");
    expect_normalized_pair("a\n", "a ");
    expect_normalized_pair("a\0", "目录/é");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_index_changes_empty_and_exact_stage_zero_are_empty() {
    let mut tree = policy::Tree::new();
    let out = policy::insert_git_tree_entry(&mut tree,
        policy::TreePath::from_text("a"), "mode", "oid", "kind");
    expect_tree_insert_ok(out);
    let empty = policy::checkout_index_changes(&tree, &[]);
    expect_checkout_path(&empty, "a", false);
    let row = policy::GitIndexRow { path: "a", mode: "mode", oid: "oid", stage: "0" };
    let rows = vec![row];
    let changes = policy::checkout_index_changes(&tree, rows.as_slice());
    expect_checkout_path(&changes, "a", false);
    expect_checkout_path(&changes, "", false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_index_changes_keep_all_rows_and_deduplicate_full_paths() {
    proof {
        reveal_strlit("a"); reveal_strlit("b"); reveal_strlit("B");
        reveal_strlit("0"); reveal_strlit("1"); reveal_strlit("2");
        assert("a"@[0] != "b"@[0]); assert("B"@[0] != "b"@[0]);
    }
    let tree = policy::Tree::new();
    let text = "b".to_owned();
    let rows = vec![
        policy::GitIndexRow { path: "b", mode: "", oid: "", stage: "0" },
        policy::GitIndexRow { path: "a", mode: "", oid: "", stage: "1" },
        policy::GitIndexRow { path: text.as_str(), mode: "other", oid: "other", stage: "2" },
    ];
    let mut changes = policy::checkout_index_changes(&tree, rows.as_slice());
    expect_checkout_path(&changes, "a", true);
    expect_checkout_path(&changes, "b", true);
    expect_checkout_path(&changes, "B", false);
    policy::record_checkout_change(&mut changes, "a");
    policy::record_checkout_change(&mut changes, "B");
    expect_checkout_path(&changes, "B", true);
    expect_checkout_path(&changes, "b", true);
    expect_checkout_path(&changes, "a", true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_index_changes_compare_complete_mode_oid_and_path_text() {
    proof {
        reveal_strlit("a"); reveal_strlit("a\0"); reveal_strlit("A");
        reveal_strlit("mode"); reveal_strlit("mode\0");
        reveal_strlit("oid"); reveal_strlit("oid\0");
        reveal_strlit("0"); reveal_strlit("3");
        assert("a"@.len() != "a\0"@.len()); assert("A"@[0] != "a"@[0]);
        assert("mode"@.len() != "mode\0"@.len()); assert("oid"@.len() != "oid\0"@.len());
        assert("0"@[0] != "3"@[0]);
    }
    let mut tree = policy::Tree::new();
    let out = policy::insert_git_tree_entry(&mut tree,
        policy::TreePath::from_text("a"), "mode", "oid", "ignored kind");
    expect_tree_insert_ok(out);
    let mode_rows = vec![policy::GitIndexRow { path: "a", mode: "mode\0", oid: "oid", stage: "0" }];
    let changes = policy::checkout_index_changes(&tree, mode_rows.as_slice());
    expect_checkout_path(&changes, "a", true);
    let oid_rows = vec![policy::GitIndexRow { path: "a", mode: "mode", oid: "oid\0", stage: "0" }];
    let changes = policy::checkout_index_changes(&tree, oid_rows.as_slice());
    expect_checkout_path(&changes, "a", true);
    let path_rows = vec![
        policy::GitIndexRow { path: "a\0", mode: "mode", oid: "oid", stage: "0" },
        policy::GitIndexRow { path: "A", mode: "mode", oid: "oid", stage: "3" },
    ];
    let changes = policy::checkout_index_changes(&tree, path_rows.as_slice());
    expect_checkout_path(&changes, "a\0", true);
    expect_checkout_path(&changes, "A", true);
    expect_checkout_path(&changes, "a", false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_physical_plan_keeps_empty_tree_and_prior_changes() {
    let tree = policy::Tree::new();
    let stages = policy::GitIndexStages::new();
    let lookup = policy::snapshot_git_index_stages(&stages);
    let mut changed = policy::CheckoutChanges::new();
    policy::record_checkout_change(&mut changed, "keep");
    let targets = policy::checkout_physical_targets(&tree, &lookup, &mut changed);
    proof {
        assert(tree@.kv_pairs() =~= Set::empty());
        assert(policy::tree_rows_view(targets@).to_set() =~= Set::empty());
    }
    expect_status_true(targets.len() == 0);
    expect_checkout_path(&changed, "keep", true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_physical_plan_marks_missing_stage_zero_without_a_file_probe() {
    proof {
        reveal_strlit("a"); reveal_strlit("1"); reveal_strlit("0");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("1") && vstd::string::is_ascii("0"));
        assert("1".spec_bytes()[0] != "0".spec_bytes()[0]);
    }
    let mut tree = policy::Tree::new();
    let key = policy::TreePath::from_text("a");
    let ghost expected_key = key;
    expect_tree_insert_ok(policy::insert_git_tree_entry(&mut tree,
        key, "mode", "oid", "kind"));
    let mut stages = policy::GitIndexStages::new();
    expect_status_true(policy::record_git_index_stage(&mut stages, "a", "1").is_ok());
    let lookup = policy::snapshot_git_index_stages(&stages);
    let mut changed = policy::CheckoutChanges::new();
    let targets = policy::checkout_physical_targets(&tree, &lookup, &mut changed);
    proof {
        assert(tree@.kv_pairs() =~= set![(expected_key, tree@[expected_key])]);
        assert(!policy::checkout_stage_zero(lookup@, expected_key.bytes())) by {
            assert forall|key: policy::GitIndexStage| lookup@.to_set().contains(key)
                implies key.stage.spec_bytes() != "0".spec_bytes() by {
                assert(key.stage.spec_bytes() == "1".spec_bytes());
            }
        }
        assert(policy::tree_rows_view(targets@).to_set() =~= Set::empty());
        assert(tree@.kv_pairs().contains((expected_key, tree@[expected_key])));
    }
    expect_status_true(targets.len() == 0);
    expect_checkout_path(&changed, "a", true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_physical_plan_preserves_probe_order_and_complete_entry_fields() {
    proof {
        reveal_strlit("a"); reveal_strlit("b"); reveal_strlit("0");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("a") && vstd::string::is_ascii("b"));
        assert("a".spec_bytes()[0] < "b".spec_bytes()[0]);
        assert(scope_text::byte_prefix("a".spec_bytes(), "b".spec_bytes(), 0));
        assert(scope_text::byte_lt("a".spec_bytes(), "b".spec_bytes()));
        assert("a"@[0] != "b"@[0]);
    }
    let mut tree = policy::Tree::new();
    let key_b = policy::TreePath::from_text("b");
    let ghost kb = key_b;
    expect_tree_insert_ok(policy::insert_git_tree_entry(&mut tree,
        key_b, "b-mode", "b-oid", "b-kind"));
    let key_a = policy::TreePath::from_text("a");
    let ghost ka = key_a;
    expect_tree_insert_ok(policy::insert_git_tree_entry(&mut tree,
        key_a, "a-mode", "a-oid", "a-kind"));
    let mut stages = policy::GitIndexStages::new();
    expect_status_true(policy::record_git_index_stage(&mut stages, "b", "0").is_ok());
    expect_status_true(policy::record_git_index_stage(&mut stages, "a", "0").is_ok());
    let lookup = policy::snapshot_git_index_stages(&stages);
    let mut changed = policy::CheckoutChanges::new();
    let targets = policy::checkout_physical_targets(&tree, &lookup, &mut changed);
    proof {
        let a = (ka, tree@[ka]);
        let b = (kb, tree@[kb]);
        assert(tree@.kv_pairs() =~= set![a, b]);
        assert(stages@.contains_key(policy::GitIndexStage { path: "a", stage: "0" }));
        assert(stages@.contains_key(policy::GitIndexStage { path: "b", stage: "0" }));
        assert(lookup@.to_set().contains(policy::GitIndexStage { path: "a", stage: "0" }));
        assert(lookup@.to_set().contains(policy::GitIndexStage { path: "b", stage: "0" }));
        assert(policy::checkout_stage_zero(lookup@, ka.bytes()));
        assert(policy::checkout_stage_zero(lookup@, kb.bytes()));
        assert(policy::tree_rows_view(targets@).to_set() =~= set![a, b]);
        let expected = seq![a, b];
        assert(expected.no_duplicates());
        expected.unique_seq_to_set();
        assert(expected.to_set() =~= set![a, b]);
        assert(targets.len() == 2);
        let values = policy::tree_rows_view(targets@);
        assert(values.to_set().contains(values[0]));
        assert(values.to_set().contains(values[1]));
        if values[0] == b {
            assert(values[1] == a);
            assert(scope_text::byte_lt(kb.bytes(), ka.bytes()));
            scope_text::byte_lt_transitive(ka.bytes(), kb.bytes(), ka.bytes());
            scope_text::byte_lt_strict(ka.bytes(), ka.bytes());
        }
        assert(values[0] == a && values[1] == b);
    }
    expect_status_true(targets.len() == 2);
    expect_status_text(targets[0].0.as_str(), "a");
    expect_status_text(targets[0].1.mode.as_str(), "a-mode");
    expect_status_text(targets[0].1.oid.as_str(), "a-oid");
    expect_status_text(targets[0].1.kind.as_str(), "a-kind");
    expect_status_text(targets[1].0.as_str(), "b");
    expect_status_text(targets[1].1.mode.as_str(), "b-mode");
    expect_status_text(targets[1].1.oid.as_str(), "b-oid");
    expect_status_text(targets[1].1.kind.as_str(), "b-kind");
    expect_checkout_path(&changed, "a", false);
    expect_checkout_path(&changed, "b", false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_physical_observations_preserve_prior_changes_and_exact_failed_paths() {
    broadcast use vstd::array::group_array_axioms;
    proof {
        reveal_strlit("keep"); reveal_strlit("a"); reveal_strlit("a\0");
        reveal_strlit("目录/é"); reveal_strlit("目录/e");
        assert("a"@.len() != "a\0"@.len());
        assert("目录/é"@[3] != "目录/e"@[3]);
        assert("keep"@[0] != "目录/é"@[0]);
        assert("keep"@.len() != "a"@.len() && "keep"@.len() != "a\0"@.len());
        assert("a\0"@.len() != "目录/é"@.len());
    }
    let duplicate = "a\0".to_owned();
    let observations = vec![
        policy::CheckoutPhysicalObservation { path: "a", matches: true },
        policy::CheckoutPhysicalObservation { path: "a\0", matches: false },
        policy::CheckoutPhysicalObservation { path: "目录/é", matches: false },
        policy::CheckoutPhysicalObservation { path: duplicate.as_str(), matches: false },
        policy::CheckoutPhysicalObservation { path: "a\0", matches: true },
    ];
    let mut changed = policy::CheckoutChanges::new();
    policy::record_checkout_change(&mut changed, "keep");
    policy::record_checkout_physical_observations(&mut changed, observations.as_slice());
    expect_checkout_path(&changed, "keep", true);
    expect_checkout_path(&changed, "a", false);
    expect_checkout_path(&changed, "a\0", true);
    expect_checkout_path(&changed, "目录/é", true);
    expect_checkout_path(&changed, "目录/e", false);
    let empty: Vec<policy::CheckoutPhysicalObservation> = Vec::new();
    policy::record_checkout_physical_observations(&mut changed, empty.as_slice());
    expect_checkout_path(&changed, "keep", true);
    expect_checkout_path(&changed, "a\0", true);
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

fn expect_checkout_metadata(out: Result<Option<String>, String>, expected: &str)
    requires out matches Ok(Some(value)) && value@ == expected@,
{
    match out {
        Ok(Some(value)) => expect_status_text(value.as_str(), expected),
        _ => panic!(),
    }
}

fn expect_checkout_observation_error<T>(out: Result<T, String>, expected: &str)
    requires out matches Err(error) && error@ == expected@,
{
    match out {
        Err(error) => expect_status_text(error.as_str(), expected),
        Ok(_) => panic!(),
    }
}

fn expect_checkout_parent(out: Result<bool, String>, expected: bool)
    requires out matches Ok(value) && value == expected,
{
    match out { Ok(value) => expect_status_true(value == expected), Err(_) => panic!() }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_metadata_preserves_complete_success_even_when_classifier_says_missing() {
    let empty = policy::checkout_metadata_observation(
        Ok::<String, String>("".to_owned()), |_error: &String| true,
    );
    expect_checkout_metadata(empty, "");
    let full = policy::checkout_metadata_observation(
        Ok::<String, String>("é\0metadata\ntail".to_owned()), |_error: &String| false,
    );
    expect_checkout_metadata(full, "é\0metadata\ntail");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_metadata_only_collapses_the_observed_missing_error() {
    proof {
        reveal_strlit("missing"); reveal_strlit("missing\0detail"); reveal_strlit("é\npermission");
        assert("missing\0detail"@.len() != "missing"@.len());
        assert("é\npermission"@.len() != "missing"@.len());
    }
    let absent = policy::checkout_metadata_observation(
        Err::<String, String>("missing".to_owned()),
        |error: &String| -> (missing: bool)
            ensures missing == (error@ == "missing"@),
        { scope_text::equal_text(error.as_str(), "missing") },
    );
    match absent { Ok(None) => {}, _ => panic!() }
    let suffix = policy::checkout_metadata_observation(
        Err::<String, String>("missing\0detail".to_owned()),
        |error: &String| -> (missing: bool)
            ensures missing == (error@ == "missing"@),
        { scope_text::equal_text(error.as_str(), "missing") },
    );
    expect_checkout_observation_error(suffix, "missing\0detail");
    let full = policy::checkout_metadata_observation(
        Err::<String, String>("é\npermission".to_owned()),
        |error: &String| -> (missing: bool)
            ensures missing == (error@ == "missing"@),
        { scope_text::equal_text(error.as_str(), "missing") },
    );
    expect_checkout_observation_error(full, "é\npermission");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_parent_success_preserves_the_actual_directory_observation() {
    expect_checkout_parent(policy::checkout_parent_observation(
        Ok::<bool, String>(true), |_error: &String| true,
    ), true);
    expect_checkout_parent(policy::checkout_parent_observation(
        Ok::<bool, String>(false), |_error: &String| true,
    ), false);
    expect_checkout_parent(policy::checkout_parent_observation(
        Ok::<bool, String>(true), |_error: &String| false,
    ), true);
    expect_checkout_parent(policy::checkout_parent_observation(
        Ok::<bool, String>(false), |_error: &String| false,
    ), false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_parent_missing_stops_and_other_errors_preserve_the_complete_error() {
    proof {
        reveal_strlit("missing"); reveal_strlit("missing\0detail"); reveal_strlit("é\npermission");
        assert("missing\0detail"@.len() != "missing"@.len());
        assert("é\npermission"@.len() != "missing"@.len());
    }
    expect_checkout_parent(policy::checkout_parent_observation(
        Err::<bool, String>("missing".to_owned()),
        |error: &String| -> (missing: bool)
            ensures missing == (error@ == "missing"@),
        { scope_text::equal_text(error.as_str(), "missing") },
    ), false);
    let suffix = policy::checkout_parent_observation(
        Err::<bool, String>("missing\0detail".to_owned()),
        |error: &String| -> (missing: bool)
            ensures missing == (error@ == "missing"@),
        { scope_text::equal_text(error.as_str(), "missing") },
    );
    expect_checkout_observation_error(suffix, "missing\0detail");
    let full = policy::checkout_parent_observation(
        Err::<bool, String>("é\npermission".to_owned()),
        |error: &String| -> (missing: bool)
            ensures missing == (error@ == "missing"@),
        { scope_text::equal_text(error.as_str(), "missing") },
    );
    expect_checkout_observation_error(full, "é\npermission");
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

fn expect_checkout_sha1(out: Result<policy::CheckoutHashAlgorithm, String>)
    requires out matches Ok(policy::CheckoutHashAlgorithm::Sha1),
{
    match out { Ok(policy::CheckoutHashAlgorithm::Sha1) => {}, _ => panic!() }
}

fn expect_checkout_sha256(out: Result<policy::CheckoutHashAlgorithm, String>)
    requires out matches Ok(policy::CheckoutHashAlgorithm::Sha256),
{
    match out { Ok(policy::CheckoutHashAlgorithm::Sha256) => {}, _ => panic!() }
}

fn expect_checkout_algorithm_error(text: &str)
    requires !scope_text::oid(text.spec_bytes()),
{
    match policy::checkout_hash_algorithm(text) {
        Ok(_) => panic!(),
        Err(message) => expect_status_text(
            message.as_str(), "expected full nonzero lowercase commit OID",
        ),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_sha1_requires_a_complete_valid_forty_byte_oid() {
    proof {
        reveal_strlit("0123456789abcdef0123456789abcdef01234567");
        reveal_strlit("0000000000000000000000000000000000000001");
        assert(vstd::string::is_ascii("0123456789abcdef0123456789abcdef01234567"));
        assert(vstd::string::is_ascii("0000000000000000000000000000000000000001"));
        vstd::string::is_ascii_spec_bytes("0123456789abcdef0123456789abcdef01234567");
        vstd::string::is_ascii_spec_bytes("0000000000000000000000000000000000000001");
        assert("0123456789abcdef0123456789abcdef01234567".spec_bytes()[1] == 49);
        assert("0000000000000000000000000000000000000001".spec_bytes()[39] == 49);
    }
    expect_checkout_sha1(policy::checkout_hash_algorithm("0123456789abcdef0123456789abcdef01234567"));
    expect_checkout_sha1(policy::checkout_hash_algorithm("0000000000000000000000000000000000000001"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_sha256_requires_a_complete_valid_sixty_four_byte_oid() {
    proof {
        reveal_strlit("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        reveal_strlit("0000000000000000000000000000000000000000000000000000000000000001");
        assert(vstd::string::is_ascii("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"));
        assert(vstd::string::is_ascii("0000000000000000000000000000000000000000000000000000000000000001"));
        vstd::string::is_ascii_spec_bytes("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        vstd::string::is_ascii_spec_bytes("0000000000000000000000000000000000000000000000000000000000000001");
        assert("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".spec_bytes()[1] == 49);
        assert("0000000000000000000000000000000000000000000000000000000000000001".spec_bytes()[63] == 49);
    }
    expect_checkout_sha256(policy::checkout_hash_algorithm(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    ));
    expect_checkout_sha256(policy::checkout_hash_algorithm(
        "0000000000000000000000000000000000000000000000000000000000000001",
    ));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_algorithm_rejects_zero_wrong_lengths_case_and_interior_control_bytes() {
    proof {
        reveal_strlit("");
        reveal_strlit("0000000000000000000000000000000000000000");
        reveal_strlit("0000000000000000000000000000000000000000000000000000000000000000");
        reveal_strlit("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        reveal_strlit("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        reveal_strlit("aaaaaaaaaaaaaaaaaaaAaaaaaaaaaaaaaaaaaaaa");
        reveal_strlit("aaaaaaaaaaaaaaaaaaa\0aaaaaaaaaaaaaaaaaaaa");
        reveal_strlit("aaaaaaaaaaaaaaaaaaa\naaaaaaaaaaaaaaaaaaaa");
        assert(vstd::string::is_ascii(""));
        assert(vstd::string::is_ascii("0000000000000000000000000000000000000000"));
        assert(vstd::string::is_ascii("0000000000000000000000000000000000000000000000000000000000000000"));
        assert(vstd::string::is_ascii("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert(vstd::string::is_ascii("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert(vstd::string::is_ascii("aaaaaaaaaaaaaaaaaaaAaaaaaaaaaaaaaaaaaaaa"));
        assert(vstd::string::is_ascii("aaaaaaaaaaaaaaaaaaa\0aaaaaaaaaaaaaaaaaaaa"));
        assert(vstd::string::is_ascii("aaaaaaaaaaaaaaaaaaa\naaaaaaaaaaaaaaaaaaaa"));
        vstd::string::is_ascii_spec_bytes("");
        vstd::string::is_ascii_spec_bytes("0000000000000000000000000000000000000000");
        vstd::string::is_ascii_spec_bytes("0000000000000000000000000000000000000000000000000000000000000000");
        vstd::string::is_ascii_spec_bytes("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        vstd::string::is_ascii_spec_bytes("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        vstd::string::is_ascii_spec_bytes("aaaaaaaaaaaaaaaaaaaAaaaaaaaaaaaaaaaaaaaa");
        vstd::string::is_ascii_spec_bytes("aaaaaaaaaaaaaaaaaaa\0aaaaaaaaaaaaaaaaaaaa");
        vstd::string::is_ascii_spec_bytes("aaaaaaaaaaaaaaaaaaa\naaaaaaaaaaaaaaaaaaaa");
        assert("aaaaaaaaaaaaaaaaaaaAaaaaaaaaaaaaaaaaaaaa".spec_bytes()[19] == 65);
        assert("aaaaaaaaaaaaaaaaaaa\0aaaaaaaaaaaaaaaaaaaa".spec_bytes()[19] == 0);
        assert("aaaaaaaaaaaaaaaaaaa\naaaaaaaaaaaaaaaaaaaa".spec_bytes()[19] == 10);
    }
    expect_checkout_algorithm_error("");
    expect_checkout_algorithm_error("0000000000000000000000000000000000000000");
    expect_checkout_algorithm_error("0000000000000000000000000000000000000000000000000000000000000000");
    expect_checkout_algorithm_error("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    expect_checkout_algorithm_error("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    expect_checkout_algorithm_error("aaaaaaaaaaaaaaaaaaaAaaaaaaaaaaaaaaaaaaaa");
    expect_checkout_algorithm_error("aaaaaaaaaaaaaaaaaaa\0aaaaaaaaaaaaaaaaaaaa");
    expect_checkout_algorithm_error("aaaaaaaaaaaaaaaaaaa\naaaaaaaaaaaaaaaaaaaa");
}

fn expect_checkout_hash_count(out: Result<u64, String>, expected: u64)
    requires out matches Ok(value) && value == expected,
{
    match out { Ok(value) => expect_status_true(value == expected), Err(_) => panic!() }
}

fn blob_match_original_input(size: usize, bytes: &[u8])
    requires size == 4, bytes@ == seq![0u8, 255u8, 10u8, 13u8],
{
    expect_status_true(size == 4);
    expect_status_true(bytes.len() == 4);
    expect_status_true(bytes[0] == 0);
    expect_status_true(bytes[1] == 255);
    expect_status_true(bytes[2] == 10);
    expect_status_true(bytes[3] == 13);
}

fn blob_match_short_digest(size: usize, bytes: &[u8]) -> (out: String)
    requires size == 4, bytes@ == seq![0u8, 255u8, 10u8, 13u8],
    ensures out@ == "1111111111111111111111111111111111111111"@,
{
    blob_match_original_input(size, bytes);
    checkout_dispatch_short_oid().to_owned()
}

fn blob_match_long_digest(size: usize, bytes: &[u8]) -> (out: String)
    requires size == 4, bytes@ == seq![0u8, 255u8, 10u8, 13u8],
    ensures out@ == "1111111111111111111111111111111111111111111111111111111111111111"@,
{
    blob_match_original_input(size, bytes);
    checkout_dispatch_long_oid().to_owned()
}

fn blob_match_full_digest(size: usize, bytes: &[u8]) -> (out: String)
    requires size == 4, bytes@ == seq![0u8, 255u8, 10u8, 13u8],
    ensures out@ == "digest: é\0original\nend"@, out@[0] == 'd',
{
    blob_match_original_input(size, bytes);
    proof { reveal_strlit("digest: é\0original\nend"); }
    "digest: é\0original\nend".to_owned()
}

fn blob_match_empty_short(size: usize, bytes: &[u8]) -> (out: String)
    requires size == 0, bytes@.len() == 0,
    ensures out@ == "1111111111111111111111111111111111111111"@,
{
    expect_status_true(size == 0 && bytes.is_empty());
    checkout_dispatch_short_oid().to_owned()
}

fn blob_match_empty_long(size: usize, bytes: &[u8]) -> (out: String)
    requires size == 0, bytes@.len() == 0,
    ensures out@ == "1111111111111111111111111111111111111111111111111111111111111111"@,
{
    expect_status_true(size == 0 && bytes.is_empty());
    checkout_dispatch_long_oid().to_owned()
}

fn blob_match_never(_size: usize, _bytes: &[u8]) -> String
    requires false,
{ panic!() }

#[cfg_attr(not(verus_keep_ghost), test)]
fn blob_match_rejects_every_wrong_length_before_hashing() {
    let bytes = [0u8, 255u8, 10u8, 13u8];
    expect_status_true(!policy::git_blob_matches(checkout_dispatch_short_oid(), 0, &bytes,
        blob_match_never, blob_match_never));
    expect_status_true(!policy::git_blob_matches(checkout_dispatch_long_oid(), 5, &bytes,
        blob_match_never, blob_match_never));
    expect_status_true(!policy::git_blob_matches(checkout_dispatch_long_oid(), usize::MAX, &bytes,
        blob_match_never, blob_match_never));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn blob_match_sha1_keeps_original_length_bytes_and_complete_digest() {
    let bytes = [0u8, 255u8, 10u8, 13u8];
    expect_status_true(policy::git_blob_matches(checkout_dispatch_short_oid(), 4, &bytes,
        blob_match_short_digest, blob_match_never));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn blob_match_sha256_keeps_original_length_bytes_and_complete_digest() {
    let bytes = [0u8, 255u8, 10u8, 13u8];
    expect_status_true(policy::git_blob_matches(checkout_dispatch_long_oid(), 4, &bytes,
        blob_match_never, blob_match_long_digest));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn blob_match_sha1_rejects_a_different_complete_digest() {
    let bytes = [0u8, 255u8, 10u8, 13u8];
    expect_status_true(!policy::git_blob_matches(checkout_dispatch_short_oid(), 4, &bytes,
        blob_match_full_digest, blob_match_never));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn blob_match_sha256_rejects_a_different_complete_digest() {
    let bytes = [0u8, 255u8, 10u8, 13u8];
    expect_status_true(!policy::git_blob_matches(checkout_dispatch_long_oid(), 4, &bytes,
        blob_match_never, blob_match_full_digest));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn blob_match_empty_sha1_still_hashes_the_original_empty_body() {
    expect_status_true(policy::git_blob_matches(checkout_dispatch_short_oid(), 0, &[],
        blob_match_empty_short, blob_match_never));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn blob_match_empty_sha256_still_hashes_the_original_empty_body() {
    expect_status_true(policy::git_blob_matches(checkout_dispatch_long_oid(), 0, &[],
        blob_match_never, blob_match_empty_long));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn blob_match_preserves_complete_unicode_nul_and_line_text() {
    let bytes = [0u8, 255u8, 10u8, 13u8];
    let exact = "digest: é\0original\nend";
    let different = "digest: é\0original\nEnd";
    proof {
        reveal_strlit("digest: é\0original\nend");
        reveal_strlit("digest: é\0original\nEnd");
        assert(exact@[19] == 'e');
        assert(different@[19] == 'E');
        assert(exact@ != different@);
    }
    expect_status_true(policy::git_blob_matches(exact, 4, &bytes,
        blob_match_full_digest, blob_match_full_digest));
    expect_status_true(!policy::git_blob_matches(different, 4, &bytes,
        blob_match_full_digest, blob_match_full_digest));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn blob_match_keeps_the_original_nonforty_byte_fallback() {
    let bytes = [0u8, 255u8, 10u8, 13u8];
    let oid = "1";
    proof {
        reveal_strlit("1");
        assert(vstd::string::is_ascii(oid));
        vstd::string::is_ascii_spec_bytes(oid);
    }
    expect_status_true(!policy::git_blob_matches(oid, 4, &bytes,
        blob_match_never, blob_match_full_digest));
}

fn checkout_dispatch_success(size: u64, source: u8) -> (out: Result<String, String>)
    ensures if size == u64::MAX && source == 77 {
        out matches Ok(text) && text@ == "digest: é\0full\nend"@
    } else { out matches Err(error) && error@ == "wrong success arguments"@ },
{
    if size == u64::MAX && source == 77 { Ok("digest: é\0full\nend".to_owned()) }
    else { Err("wrong success arguments".to_owned()) }
}

fn checkout_dispatch_error(size: u64, source: u8) -> (out: Result<String, String>)
    ensures if size == 0 && source == 255 {
        out matches Err(error) && error@ == "read: é\0original\nend"@
    } else { out matches Ok(text) && text@ == "wrong error arguments"@ },
{
    if size == 0 && source == 255 { Err("read: é\0original\nend".to_owned()) }
    else { Ok("wrong error arguments".to_owned()) }
}

fn checkout_dispatch_never(_size: u64, _source: u8) -> Result<String, String>
    requires false,
{ panic!() }

fn checkout_dispatch_short_oid() -> (text: &'static str)
    ensures policy::identifiers::oid(text.spec_bytes()), text.spec_bytes().len() == 40,
        text@ == "1111111111111111111111111111111111111111"@, text@[0] == '1',
{
    let text = "1111111111111111111111111111111111111111";
    proof {
        reveal_strlit("1111111111111111111111111111111111111111");
        assert(vstd::string::is_ascii(text));
        vstd::string::is_ascii_spec_bytes(text);
        assert(text.spec_bytes()[0] == 49);
    }
    text
}

fn checkout_dispatch_long_oid() -> (text: &'static str)
    ensures policy::identifiers::oid(text.spec_bytes()), text.spec_bytes().len() == 64,
        text@ == "1111111111111111111111111111111111111111111111111111111111111111"@,
        text@[0] == '1',
{
    let text = "1111111111111111111111111111111111111111111111111111111111111111";
    proof {
        reveal_strlit("1111111111111111111111111111111111111111111111111111111111111111");
        assert(vstd::string::is_ascii(text));
        vstd::string::is_ascii_spec_bytes(text);
        assert(text.spec_bytes()[0] == 49);
    }
    text
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_dispatch_sha1_preserves_complete_source_size_and_digest() {
    let out = policy::checkout_object_hash(checkout_dispatch_short_oid(), u64::MAX, 77u8,
        checkout_dispatch_success, checkout_dispatch_never);
    match out {
        Ok(text) => expect_status_text(text.as_str(), "digest: é\0full\nend"),
        _ => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_dispatch_sha256_preserves_complete_source_size_and_digest() {
    let out = policy::checkout_object_hash(checkout_dispatch_long_oid(), u64::MAX, 77u8,
        checkout_dispatch_never, checkout_dispatch_success);
    match out {
        Ok(text) => expect_status_text(text.as_str(), "digest: é\0full\nend"),
        _ => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_dispatch_sha1_preserves_the_complete_hash_error() {
    let out = policy::checkout_object_hash(checkout_dispatch_short_oid(), 0, 255u8,
        checkout_dispatch_error, checkout_dispatch_never);
    expect_checkout_observation_error(out, "read: é\0original\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_dispatch_sha256_preserves_the_complete_hash_error() {
    let out = policy::checkout_object_hash(checkout_dispatch_long_oid(), 0, 255u8,
        checkout_dispatch_never, checkout_dispatch_error);
    expect_checkout_observation_error(out, "read: é\0original\nend");
}

fn checkout_dispatch_invalid_without_hash(text: &str)
    requires !policy::identifiers::oid(text.spec_bytes()),
{
    let out = policy::checkout_object_hash(text, u64::MAX, 0u8,
        checkout_dispatch_never, checkout_dispatch_never);
    expect_checkout_observation_error(out, "expected full nonzero lowercase commit OID");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_dispatch_rejects_invalid_oids_before_either_hash() {
    proof {
        reveal_strlit(""); reveal_strlit("1");
        reveal_strlit("0000000000000000000000000000000000000000");
        assert(vstd::string::is_ascii(""));
        assert(vstd::string::is_ascii("1"));
        assert(vstd::string::is_ascii("0000000000000000000000000000000000000000"));
        vstd::string::is_ascii_spec_bytes("");
        vstd::string::is_ascii_spec_bytes("1");
        vstd::string::is_ascii_spec_bytes("0000000000000000000000000000000000000000");
    }
    checkout_dispatch_invalid_without_hash("");
    checkout_dispatch_invalid_without_hash("1");
    checkout_dispatch_invalid_without_hash("0000000000000000000000000000000000000000");
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

// 小缓冲区保留未读取的尾字节；摘要 fixture 只有收到按顺序的单字节 0、255 才成功。
fn checkout_stream_fixture_view(buffer: &[u8; 4]) -> (bytes: &[u8])
    ensures bytes@ == buffer@,
{ buffer.as_slice() }

fn checkout_stream_fixture_read(state: u8, buffer: [u8; 4])
    -> (out: (u8, [u8; 4], Result<usize, String>))
    ensures out == if state == 0 { (1u8, [0u8, 11, 12, 13], Ok::<usize, String>(1)) }
        else if state == 1 { (2u8, [255u8, 21, 22, 23], Ok::<usize, String>(1)) }
        else { (state, buffer, Ok::<usize, String>(0)) },
{
    if state == 0 { (1, [0, 11, 12, 13], Ok(1)) }
    else if state == 1 { (2, [255, 21, 22, 23], Ok(1)) }
    else { (state, buffer, Ok(0)) }
}

fn checkout_stream_fixture_error(state: u8, buffer: [u8; 4])
    -> (out: (u8, [u8; 4], Result<usize, String>))
    ensures out.0 == state, out.1 == buffer,
        match out.2 { Err(error) => error@ == "read: é\0original\nend"@, Ok(_) => false },
{ (state, buffer, Err("read: é\0original\nend".to_owned())) }

fn checkout_stream_fixture_late_error(state: u8, buffer: [u8; 4])
    -> (out: (u8, [u8; 4], Result<usize, String>))
    ensures if state == 0 { out == (1u8, [0u8, 11, 12, 13], Ok::<usize, String>(1)) }
        else { out.0 == state && out.1 == buffer
            && match out.2 { Err(error) => error@ == "read: é\0original\nend"@, Ok(_) => false } },
{
    if state == 0 { (1, [0, 11, 12, 13], Ok(1)) }
    else { (state, buffer, Err("read: é\0original\nend".to_owned())) }
}

fn checkout_stream_fixture_eof(state: u8, buffer: [u8; 4])
    -> (out: (u8, [u8; 4], Result<usize, String>))
    ensures out == (state, buffer, Ok::<usize, String>(0)),
{ (state, buffer, Ok(0)) }

fn checkout_stream_fixture_update(digest: u8, bytes: &[u8]) -> (out: u8)
    ensures out == if digest == 0 && bytes@ == seq![0u8] { 1u8 }
        else if digest == 1 && bytes@ == seq![255u8] { 2u8 } else { 99u8 },
{
    if bytes.len() == 1 { assert(bytes@ =~= seq![bytes[0]]); }
    if bytes.len() == 1 && digest == 0 && bytes[0] == 0 { 1 }
    else if bytes.len() == 1 && digest == 1 && bytes[0] == 255 { 2 }
    else { 99 }
}

fn checkout_stream_fixture_finalize(digest: u8) -> (out: String)
    ensures out@ == if digest == 0 { "empty"@ }
        else if digest == 2 { "digest: \0ÿ"@ } else { "unexpected bytes or update order"@ },
{
    if digest == 0 { "empty".to_owned() }
    else if digest == 2 { "digest: \0ÿ".to_owned() }
    else { "unexpected bytes or update order".to_owned() }
}

fn expect_checkout_stream_success(out: Result<String, String>, expected: &str)
    requires match out { Ok(value) => value@ == expected@, Err(_) => false },
{
    match out {
        Ok(value) => expect_status_text(value.as_str(), expected),
        Err(_) => panic!(),
    }
}

fn expect_checkout_stream_failure(out: Result<String, String>, expected: &str)
    requires match out { Err(value) => value@ == expected@, Ok(_) => false },
{
    match out {
        Err(value) => expect_status_text(value.as_str(), expected),
        Ok(_) => panic!(),
    }
}

proof fn checkout_stream_prefix_at<S, B, D, R, V, U>(
    states: Seq<(S, B, D, u64)>, initial: (S, B, D, u64), size: u64,
    read: R, view: V, update: U, index: int,
)
    where R: Fn(S, B) -> (S, B, Result<usize, String>),
          V: Fn(&B) -> &[u8], U: Fn(D, &[u8]) -> D,
    requires policy::checkout_hash_stream_prefix(states, initial, size, read, view, update),
        0 <= index < states.len() - 1,
    ensures policy::checkout_hash_stream_step(states[index], states[index + 1], size, read, view, update),
{ }

proof fn checkout_stream_fixture_read_step(before: (u8, [u8; 4], u8, u64),
    after: (u8, [u8; 4], u8, u64), size: u64)
    requires before.0 < 2, before.2 == before.0, before.3 == before.0 as u64,
        policy::checkout_hash_stream_step(before, after, size,
            checkout_stream_fixture_read, checkout_stream_fixture_view, checkout_stream_fixture_update),
    ensures after.0 as int == before.0 as int + 1,
        after.2 == after.0, after.3 == after.0 as u64,
{
    reveal(policy::checkout_hash_stream_step);
    let (count, bytes, chunk) = choose|count: usize, bytes: &[u8], chunk: &[u8]|
        count > 0 && count <= bytes.len()
        && checkout_stream_fixture_read.ensures((before.0, before.1),
            (after.0, after.1, Ok::<usize, String>(count)))
        && checkout_stream_fixture_view.ensures((&after.1,), bytes)
        && chunk@ == bytes@.subrange(0, count as int)
        && policy::checkout_hash_advance_spec(before.3, size, count as u64)
            == Ok::<u64, Seq<char>>(after.3)
        && checkout_stream_fixture_update.ensures((before.2, chunk), after.2);
    assert(count == 1);
    if before.0 == 0 { assert(chunk@ =~= seq![0u8]); }
    else { assert(before.0 == 1); assert(chunk@ =~= seq![255u8]); }
}

proof fn checkout_stream_fixture_late_step(before: (u8, [u8; 4], u8, u64),
    after: (u8, [u8; 4], u8, u64), size: u64)
    requires policy::checkout_hash_stream_step(before, after, size,
        checkout_stream_fixture_late_error, checkout_stream_fixture_view, checkout_stream_fixture_update),
    ensures before.0 == 0, after.0 == 1, after.3 as int == before.3 as int + 1,
{ reveal(policy::checkout_hash_stream_step); }

proof fn checkout_stream_fixture_eof_cannot_advance(before: (u8, [u8; 4], u8, u64),
    after: (u8, [u8; 4], u8, u64), size: u64)
    requires policy::checkout_hash_stream_step(before, after, size,
        checkout_stream_fixture_eof, checkout_stream_fixture_view, checkout_stream_fixture_update),
    ensures false,
{ reveal(policy::checkout_hash_stream_step); }

proof fn checkout_stream_fixture_read_result(buffer: [u8; 4], size: u64, out: Result<String, String>)
    requires size <= 2,
        policy::checkout_hash_stream_result((0u8, buffer, 0u8, 0u64), size,
            checkout_stream_fixture_read, checkout_stream_fixture_view,
            checkout_stream_fixture_update, checkout_stream_fixture_finalize, out),
    ensures if size == 2 { match out { Ok(value) => value@ == "digest: \0ÿ"@, Err(_) => false } }
        else { match out { Err(value) => value@ == "checkout file grew during observation"@, Ok(_) => false } },
{
    let initial = (0u8, buffer, 0u8, 0u64);
    let (states, observation) = choose|states: Seq<(u8, [u8; 4], u8, u64)>,
        observation: (u8, [u8; 4], Result<usize, String>)|
        policy::checkout_hash_stream_prefix(states, initial, size,
            checkout_stream_fixture_read, checkout_stream_fixture_view, checkout_stream_fixture_update)
        && policy::checkout_hash_stream_terminal(states.last(), observation, size,
            checkout_stream_fixture_read, checkout_stream_fixture_finalize, out);
    assert(states.last().3 <= size);
    assert(1 <= states.len() <= 3);
    if states.len() >= 2 {
        checkout_stream_prefix_at(states, initial, size, checkout_stream_fixture_read,
            checkout_stream_fixture_view, checkout_stream_fixture_update, 0);
        checkout_stream_fixture_read_step(states[0], states[1], size);
    }
    if states.len() >= 3 {
        checkout_stream_prefix_at(states, initial, size, checkout_stream_fixture_read,
            checkout_stream_fixture_view, checkout_stream_fixture_update, 1);
        checkout_stream_fixture_read_step(states[1], states[2], size);
    }
    if states.len() == 1 { assert(states.last() == initial); }
    else if states.len() == 2 { assert(states.last() == states[1]); }
    else { assert(states.len() == 3 && states.last() == states[2]); }
}

proof fn checkout_stream_fixture_late_error_result(buffer: [u8; 4], size: u64, out: Result<String, String>)
    requires 1 <= size <= 2,
        policy::checkout_hash_stream_result((0u8, buffer, 0u8, 0u64), size,
            checkout_stream_fixture_late_error, checkout_stream_fixture_view,
            checkout_stream_fixture_update, checkout_stream_fixture_finalize, out),
    ensures match out { Err(value) => value@ == "read: é\0original\nend"@, Ok(_) => false },
{
    let initial = (0u8, buffer, 0u8, 0u64);
    let (states, observation) = choose|states: Seq<(u8, [u8; 4], u8, u64)>,
        observation: (u8, [u8; 4], Result<usize, String>)|
        policy::checkout_hash_stream_prefix(states, initial, size,
            checkout_stream_fixture_late_error, checkout_stream_fixture_view, checkout_stream_fixture_update)
        && policy::checkout_hash_stream_terminal(states.last(), observation, size,
            checkout_stream_fixture_late_error, checkout_stream_fixture_finalize, out);
    assert(states.last().3 <= size);
    assert(1 <= states.len() <= 3);
    if states.len() >= 2 {
        checkout_stream_prefix_at(states, initial, size, checkout_stream_fixture_late_error,
            checkout_stream_fixture_view, checkout_stream_fixture_update, 0);
        checkout_stream_fixture_late_step(states[0], states[1], size);
        assert(states[1].0 == 1 && states[1].3 == 1);
    }
    if states.len() == 3 {
        checkout_stream_prefix_at(states, initial, size, checkout_stream_fixture_late_error,
            checkout_stream_fixture_view, checkout_stream_fixture_update, 1);
        checkout_stream_fixture_late_step(states[1], states[2], size);
        assert(false);
    }
    if states.len() == 1 { assert(states.last() == initial); }
    else { assert(states.len() == 2 && states.last() == states[1]); }
}

proof fn checkout_stream_fixture_eof_result(buffer: [u8; 4], size: u64, out: Result<String, String>)
    requires size <= 1,
        policy::checkout_hash_stream_result((0u8, buffer, 0u8, 0u64), size,
            checkout_stream_fixture_eof, checkout_stream_fixture_view,
            checkout_stream_fixture_update, checkout_stream_fixture_finalize, out),
    ensures if size == 0 { match out { Ok(value) => value@ == "empty"@, Err(_) => false } }
        else { match out { Err(value) => value@ == "checkout file shrank during observation"@, Ok(_) => false } },
{
    let initial = (0u8, buffer, 0u8, 0u64);
    let (states, observation) = choose|states: Seq<(u8, [u8; 4], u8, u64)>,
        observation: (u8, [u8; 4], Result<usize, String>)|
        policy::checkout_hash_stream_prefix(states, initial, size,
            checkout_stream_fixture_eof, checkout_stream_fixture_view, checkout_stream_fixture_update)
        && policy::checkout_hash_stream_terminal(states.last(), observation, size,
            checkout_stream_fixture_eof, checkout_stream_fixture_finalize, out);
    if states.len() > 1 {
        checkout_stream_prefix_at(states, initial, size, checkout_stream_fixture_eof,
            checkout_stream_fixture_view, checkout_stream_fixture_update, 0);
        checkout_stream_fixture_eof_cannot_advance(states[0], states[1], size);
    }
    assert(states.len() == 1 && states.last() == initial);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_stream_preserves_chunk_bytes_and_update_order_until_eof() {
    let out_buffer = [77u8, 77, 77, 77];
    let ghost out_initial = out_buffer;
    let out = policy::checkout_hash_stream(2, 0u8, 0u8, out_buffer,
        checkout_stream_fixture_read, checkout_stream_fixture_view,
        checkout_stream_fixture_update, checkout_stream_fixture_finalize);
    proof { checkout_stream_fixture_read_result(out_initial, 2, out); }
    expect_checkout_stream_success(out, "digest: \0ÿ");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_stream_empty_eof_finalizes_the_initial_digest() {
    let out_buffer = [77u8, 77, 77, 77];
    let ghost out_initial = out_buffer;
    let out = policy::checkout_hash_stream(0, 0u8, 0u8, out_buffer,
        checkout_stream_fixture_eof, checkout_stream_fixture_view,
        checkout_stream_fixture_update, checkout_stream_fixture_finalize);
    proof { checkout_stream_fixture_eof_result(out_initial, 0, out); }
    expect_checkout_stream_success(out, "empty");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_stream_preserves_first_and_late_read_error_in_full() {
    let first_buffer = [77u8, 77, 77, 77];
    let ghost first_initial = first_buffer;
    let first = policy::checkout_hash_stream(0, 0u8, 0u8, first_buffer,
        checkout_stream_fixture_error, checkout_stream_fixture_view,
        checkout_stream_fixture_update, checkout_stream_fixture_finalize);
    expect_checkout_stream_failure(first, "read: é\0original\nend");
    let late_buffer = [77u8, 77, 77, 77];
    let ghost late_initial = late_buffer;
    let late = policy::checkout_hash_stream(2, 0u8, 0u8, late_buffer,
        checkout_stream_fixture_late_error, checkout_stream_fixture_view,
        checkout_stream_fixture_update, checkout_stream_fixture_finalize);
    proof { checkout_stream_fixture_late_error_result(late_initial, 2, late); }
    expect_checkout_stream_failure(late, "read: é\0original\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_stream_reads_again_after_expected_length_and_preserves_error_priority() {
    let out_buffer = [77u8, 77, 77, 77];
    let ghost out_initial = out_buffer;
    let out = policy::checkout_hash_stream(1, 0u8, 0u8, out_buffer,
        checkout_stream_fixture_late_error, checkout_stream_fixture_view,
        checkout_stream_fixture_update, checkout_stream_fixture_finalize);
    proof { checkout_stream_fixture_late_error_result(out_initial, 1, out); }
    expect_checkout_stream_failure(out, "read: é\0original\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_stream_rejects_growth_and_early_eof() {
    let grew_buffer = [77u8, 77, 77, 77];
    let ghost grew_initial = grew_buffer;
    let grew = policy::checkout_hash_stream(0, 0u8, 0u8, grew_buffer,
        checkout_stream_fixture_read, checkout_stream_fixture_view,
        checkout_stream_fixture_update, checkout_stream_fixture_finalize);
    proof { checkout_stream_fixture_read_result(grew_initial, 0, grew); }
    expect_checkout_stream_failure(grew, "checkout file grew during observation");
    let late_growth_buffer = [77u8, 77, 77, 77];
    let ghost late_growth_initial = late_growth_buffer;
    let late_growth = policy::checkout_hash_stream(1, 0u8, 0u8, late_growth_buffer,
        checkout_stream_fixture_read, checkout_stream_fixture_view,
        checkout_stream_fixture_update, checkout_stream_fixture_finalize);
    proof { checkout_stream_fixture_read_result(late_growth_initial, 1, late_growth); }
    expect_checkout_stream_failure(late_growth, "checkout file grew during observation");
    let shrank_buffer = [77u8, 77, 77, 77];
    let ghost shrank_initial = shrank_buffer;
    let shrank = policy::checkout_hash_stream(1, 0u8, 0u8, shrank_buffer,
        checkout_stream_fixture_eof, checkout_stream_fixture_view,
        checkout_stream_fixture_update, checkout_stream_fixture_finalize);
    proof { checkout_stream_fixture_eof_result(shrank_initial, 1, shrank); }
    expect_checkout_stream_failure(shrank, "checkout file shrank during observation");
}

fn checkout_parent_fixture_next(source: u8) -> (out: (u8, Option<u8>))
    ensures out == if source == 0 { (0u8, None) }
        else { ((source - 1) as u8, Some(if source == 1 { 0u8 } else { 1u8 })) },
{
    if source == 0 { (0, None) }
    else { (source - 1, Some(if source == 1 { 0 } else { 1 })) }
}

fn checkout_parent_fixture_append(mut path: Vec<u8>, component: u8) -> (out: Vec<u8>)
    ensures out@ == path@ + seq![component],
{ path.push(component); path }

spec fn checkout_parent_fixture_metadata_result(mode: u8, path: Seq<u8>, out: Result<bool, String>) -> bool {
    if mode == 0 { out == Ok::<bool, String>(true) }
    else if mode == 1 || mode == 2 {
        if path == seq![77u8, 0, 255, 1] { out == Ok::<bool, String>(mode == 1) }
        else { match out { Err(error) => error@ == "io: éfn index_nonzero_stage_always_changesdenied\nend"@, Ok(_) => false } }
    } else if mode == 3 {
        match out { Err(error) => error@ == "missing:fn index_nonzero_stage_always_changesé"@, Ok(_) => false }
    } else {
        match out { Err(error) => error@ == "missing:fn index_nonzero_stage_always_changesé/suffix"@, Ok(_) => false }
    }
}

fn checkout_parent_fixture_metadata(mode: u8, path: Vec<u8>) -> (out: (Vec<u8>, Result<bool, String>))
    ensures out.0 == path, checkout_parent_fixture_metadata_result(mode, path@, out.1),
{
    let observation = if mode == 0 { Ok(true) }
        else if mode == 1 || mode == 2 {
            if scope_text::equal_bytes(path.as_slice(), &[77u8, 0, 255, 1]) { Ok(mode == 1) }
            else { Err("io: éfn index_nonzero_stage_always_changesdenied\nend".to_owned()) }
        } else if mode == 3 { Err("missing:fn index_nonzero_stage_always_changesé".to_owned()) }
        else { Err("missing:fn index_nonzero_stage_always_changesé/suffix".to_owned()) };
    (path, observation)
}

fn checkout_parent_fixture_missing(error: &String) -> (missing: bool)
    ensures missing == (error@ == "missing:fn index_nonzero_stage_always_changesé"@),
{ scope_text::equal_text(error.as_str(), "missing:fn index_nonzero_stage_always_changesé") }

spec fn checkout_parent_fixture_expected(count: u8, mode: u8, out: Result<Option<Vec<u8>>, String>) -> bool {
    if count == 0 { match out { Ok(Some(path)) => path@ == seq![77u8, 0, 255], _ => false } }
    else if count == 1 { match out { Ok(Some(path)) => path@ == seq![77u8, 0, 255, 0], _ => false } }
    else if mode == 0 || (count == 2 && mode == 1) {
        match out { Ok(Some(path)) => path@ == if count == 2 { seq![77u8, 0, 255, 1, 0] }
            else { seq![77u8, 0, 255, 1, 1, 0] }, _ => false }
    } else if mode == 2 || mode == 3 { out == Ok::<Option<Vec<u8>>, String>(None) }
    else if mode == 1 { match out { Err(error) => error@ == "io: éfn index_nonzero_stage_always_changesdenied\nend"@, _ => false } }
    else { match out { Err(error) => error@ == "missing:fn index_nonzero_stage_always_changesé/suffix"@, _ => false } }
}

proof fn checkout_parent_fixture_prefix_at<S, P, C, E, N, A, M>(
    states: Seq<(S, P, C)>, first: (S, P, C), next: N, append: A, metadata: M, index: int,
)
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>),
    requires policy::checkout_physical_prefix::<S, P, C, E, N, A, M>(states, first, next, append, metadata),
        0 <= index < states.len() - 1,
    ensures policy::checkout_physical_step::<S, P, C, E, N, A, M>(
        states[index], states[index + 1], next, append, metadata),
{ }

proof fn checkout_parent_fixture_step<M>(before: (u8, Vec<u8>, u8), after: (u8, Vec<u8>, u8), mode: u8, metadata: M)
    where M: Fn(Vec<u8>) -> (Vec<u8>, Result<bool, String>),
    requires before.2 == if before.0 == 0 { 0u8 } else { 1u8 },
        policy::checkout_physical_step::<u8, Vec<u8>, u8, String, _, _, M>(
            before, after, checkout_parent_fixture_next, checkout_parent_fixture_append, metadata),
        forall|path: Vec<u8>, observed: (Vec<u8>, Result<bool, String>)|
            #[trigger] metadata.ensures((path,), observed) ==>
                observed.0 == path && checkout_parent_fixture_metadata_result(mode, path@, observed.1),
    ensures before.0 > 0, after.0 as int == before.0 as int - 1,
        after.2 == if after.0 == 0 { 0u8 } else { 1u8 },
        after.1@ == before.1@ + seq![1u8],
        checkout_parent_fixture_metadata_result(mode, after.1@, Ok::<bool, String>(true)),
{ reveal(policy::checkout_physical_step); }

proof fn checkout_parent_fixture_result<M>(count: u8, mode: u8, root: Vec<u8>, metadata: M,
    out: Result<Option<Vec<u8>>, String>)
    where M: Fn(Vec<u8>) -> (Vec<u8>, Result<bool, String>),
    requires count <= 3, mode <= 4, root@ == seq![77u8, 0, 255],
        forall|path: Vec<u8>, observed: (Vec<u8>, Result<bool, String>)|
            #[trigger] metadata.ensures((path,), observed) ==>
                observed.0 == path && checkout_parent_fixture_metadata_result(mode, path@, observed.1),
        policy::checkout_physical_result(count, root, checkout_parent_fixture_next, checkout_parent_fixture_append,
            metadata, checkout_parent_fixture_missing, out),
    ensures checkout_parent_fixture_expected(count, mode, out),
{
    reveal_strlit("io: éfn index_nonzero_stage_always_changesdenied\nend");
    reveal_strlit("missing:fn index_nonzero_stage_always_changesé");
    reveal_strlit("missing:fn index_nonzero_stage_always_changesé/suffix");
    let first = choose|first: (u8, Option<u8>)|
        checkout_parent_fixture_next.ensures((count,), first)
        && match first.1 {
            None => out == Ok::<Option<Vec<u8>>, String>(Some(root)),
            Some(component) => exists|states: Seq<(u8, Vec<u8>, u8)>, built: Vec<u8>,
                following: (u8, Option<u8>), observed: Option<(Vec<u8>, Result<bool, String>)>|
                policy::checkout_physical_prefix::<u8, Vec<u8>, u8, String, _, _, M>(states,
                    (first.0, root, component), checkout_parent_fixture_next, checkout_parent_fixture_append, metadata)
                && policy::checkout_physical_terminal(states.last(), built, following, observed,
                    checkout_parent_fixture_next, checkout_parent_fixture_append, metadata, checkout_parent_fixture_missing, out),
        };
    if count == 0 { assert(first.1.is_none()); return; }
    let component = first.1->Some_0;
    let initial = (first.0, root, component);
    let (states, built, following, observed) = choose|states: Seq<(u8, Vec<u8>, u8)>, built: Vec<u8>,
        following: (u8, Option<u8>), observed: Option<(Vec<u8>, Result<bool, String>)>|
        policy::checkout_physical_prefix::<u8, Vec<u8>, u8, String, _, _, M>(states,
            initial, checkout_parent_fixture_next, checkout_parent_fixture_append, metadata)
        && policy::checkout_physical_terminal(states.last(), built, following, observed,
            checkout_parent_fixture_next, checkout_parent_fixture_append, metadata, checkout_parent_fixture_missing, out);
    if states.len() > 1 {
        checkout_parent_fixture_prefix_at(states, initial, checkout_parent_fixture_next, checkout_parent_fixture_append, metadata, 0);
        checkout_parent_fixture_step(states[0], states[1], mode, metadata);
        assert(states[1].0 as int == count as int - 2);
        assert(states[1].1@ == seq![77u8, 0, 255, 1]);
    }
    if states.len() > 2 {
        checkout_parent_fixture_prefix_at(states, initial, checkout_parent_fixture_next, checkout_parent_fixture_append, metadata, 1);
        checkout_parent_fixture_step(states[1], states[2], mode, metadata);
        assert(states[2].0 == 0 && count == 3);
        assert(states[2].1@ == seq![77u8, 0, 255, 1, 1]);
    }
    if states.len() > 3 {
        checkout_parent_fixture_prefix_at(states, initial, checkout_parent_fixture_next, checkout_parent_fixture_append, metadata, 2);
        checkout_parent_fixture_step(states[2], states[3], mode, metadata);
        assert(false);
    }
    assert(1 <= states.len() <= 3);
    if states.len() == 1 { assert(states.last() == initial); }
    else if states.len() == 2 { assert(states.last() == states[1]); }
    else { assert(states.last() == states[2]); }
    let last = states.last();
    assert(last.0 as int == count as int - states.len());
    assert(last.2 == if last.0 == 0 { 0u8 } else { 1u8 });
    assert(built@ == last.1@ + seq![last.2]);
    if states.len() == 1 {
        if last.0 == 0 { assert(built@ =~= seq![77u8, 0, 255, 0]); }
        else { assert(built@ =~= seq![77u8, 0, 255, 1]); }
    } else if states.len() == 2 {
        assert(mode == 0 || mode == 1);
        if last.0 == 0 { assert(built@ =~= seq![77u8, 0, 255, 1, 0]); }
        else { assert(built@ =~= seq![77u8, 0, 255, 1, 1]); }
    } else {
        assert(mode == 0);
        assert(built@ =~= seq![77u8, 0, 255, 1, 1, 0]);
    }
    assert(checkout_parent_fixture_next.ensures((last.0,), following));
    match following.1 {
        None => {
            assert(last.0 == 0);
            assert(checkout_parent_fixture_expected(count, mode, out));
        },
        Some(_) => {
            let observation = observed->Some_0;
            assert(metadata.ensures((built,), observation));
            assert(checkout_parent_fixture_metadata_result(mode, built@, observation.1));
            assert(observation.0 == built);
            match observation.1 {
                Ok(_) => { assert(checkout_parent_fixture_expected(count, mode, out)); },
                Err(error) => {
                    assert(checkout_parent_fixture_missing.ensures((&error,), true)
                        || checkout_parent_fixture_missing.ensures((&error,), false));
                    if mode == 3 {
                        assert(error@ == "missing:fn index_nonzero_stage_always_changesé"@);
                        assert(out == Ok::<Option<Vec<u8>>, String>(None));
                    } else {
                        if mode == 1 || mode == 2 {
                            assert(error@ == "io: éfn index_nonzero_stage_always_changesdenied\nend"@);
                            assert(error@.len() != "missing:fn index_nonzero_stage_always_changesé"@.len());
                        } else {
                            assert(mode == 4);
                            assert(error@ == "missing:fn index_nonzero_stage_always_changesé/suffix"@);
                            assert(error@.len() != "missing:fn index_nonzero_stage_always_changesé"@.len());
                        }
                        assert(error@ != "missing:fn index_nonzero_stage_always_changesé"@);
                        assert(out == Err::<Option<Vec<u8>>, String>(error));
                    }
                    assert(checkout_parent_fixture_expected(count, mode, out));
                },
            }
        },
    }
}

fn checkout_parent_fixture_run(count: u8, mode: u8) -> (out: Result<Option<Vec<u8>>, String>)
    requires count <= 3, mode <= 4,
    ensures checkout_parent_fixture_expected(count, mode, out),
{
    let root = vec![77u8, 0, 255];
    let ghost original_root = root;
    let metadata = |path: Vec<u8>| -> (observed: (Vec<u8>, Result<bool, String>))
        ensures observed.0 == path, checkout_parent_fixture_metadata_result(mode, path@, observed.1),
    { checkout_parent_fixture_metadata(mode, path) };
    assert(policy::checkout_components_finite::<u8, u8, _>(checkout_parent_fixture_next, |source: u8| source as nat));
    assert(exists|rank: spec_fn(u8) -> nat| policy::checkout_components_finite::<u8, u8, _>(checkout_parent_fixture_next, rank));
    let out = policy::checkout_physical_path(count, root, checkout_parent_fixture_next, checkout_parent_fixture_append,
        metadata, checkout_parent_fixture_missing);
    proof { checkout_parent_fixture_result(count, mode, original_root, metadata, out); }
    out
}

fn expect_checkout_parent_path(out: Result<Option<Vec<u8>>, String>, expected: &[u8])
    requires out matches Ok(Some(path)) && path@ == expected@,
{
    match out {
        Ok(Some(path)) => expect_status_true(scope_text::equal_bytes(path.as_slice(), expected)),
        _ => panic!(),
    }
}

fn expect_checkout_parent_absent(out: Result<Option<Vec<u8>>, String>)
    requires out == Ok::<Option<Vec<u8>>, String>(None),
{
    match out { Ok(None) => {}, _ => panic!() }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_parents_empty_and_final_never_query_metadata() {
    let mut mode = 0u8;
    while mode < 5
        invariant mode <= 5,
        decreases 5 - mode,
    {
        expect_checkout_parent_path(checkout_parent_fixture_run(0, mode), &[77u8, 0, 255]);
        expect_checkout_parent_path(checkout_parent_fixture_run(1, mode), &[77u8, 0, 255, 0]);
        mode = mode + 1;
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_parents_append_and_query_in_component_order() {
    expect_checkout_parent_path(checkout_parent_fixture_run(2, 0), &[77u8, 0, 255, 1, 0]);
    expect_checkout_parent_path(checkout_parent_fixture_run(3, 0), &[77u8, 0, 255, 1, 1, 0]);
    expect_checkout_parent_path(checkout_parent_fixture_run(2, 1), &[77u8, 0, 255, 1, 0]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_parents_stop_at_first_non_directory() {
    expect_checkout_parent_absent(checkout_parent_fixture_run(2, 2));
    expect_checkout_parent_absent(checkout_parent_fixture_run(3, 2));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_parents_stop_at_first_missing_component() {
    expect_checkout_parent_absent(checkout_parent_fixture_run(2, 3));
    expect_checkout_parent_absent(checkout_parent_fixture_run(3, 3));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checkout_parents_preserve_full_initial_and_later_errors() {
    expect_checkout_observation_error(checkout_parent_fixture_run(2, 4), "missing:fn index_nonzero_stage_always_changesé/suffix");
    expect_checkout_observation_error(checkout_parent_fixture_run(3, 4), "missing:fn index_nonzero_stage_always_changesé/suffix");
    expect_checkout_observation_error(checkout_parent_fixture_run(3, 1), "io: éfn index_nonzero_stage_always_changesdenied\nend");
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

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_admission_preserves_first_offending_record_and_reason() {
    let flags = vec![
        policy::IndexFlag { tag: "H".to_string(), path: "ok".to_string() },
        policy::IndexFlag { tag: "S".to_string(), path: "目录/首个".to_string() },
        policy::IndexFlag { tag: "s".to_string(), path: "later".to_string() },
    ];
    proof {
        reveal_strlit("H"); reveal_strlit("S"); reveal_strlit("s"); reveal_strlit("h");
        assert(vstd::utf8::encode_scalar(72) == seq![72u8]) by (compute);
        assert(vstd::utf8::encode_scalar(83) == seq![83u8]) by (compute);
        assert(vstd::utf8::encode_scalar(104) == seq![104u8]) by (compute);
        assert(vstd::utf8::encode_scalar(115) == seq![115u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 2);
        assert("H".spec_bytes() =~= seq![72u8]);
        assert("S".spec_bytes() =~= seq![83u8]);
        assert("h".spec_bytes() =~= seq![104u8]);
        assert("s".spec_bytes() =~= seq![115u8]);
        assert(flags@[0].tag@ == "H"@);
        assert(flags@[1].tag@ == "S"@);
        assert(policy::index_flag_admission(vstd::utf8::encode_utf8(flags@[0].tag@)).is_none());
        assert(policy::index_flag_admission(vstd::utf8::encode_utf8(flags@[1].tag@))
            == Some(policy::UnsupportedIndexFlag::SkipWorktree));
    }
    let result = policy::first_unsupported_index_flag(flags.as_slice());
    let (entry, reason) = result.unwrap();
    proof {
        let i = choose|i: int| 0 <= i < flags.len() && (#[trigger] flags@[i]) == *entry
            && policy::index_flag_admission(vstd::utf8::encode_utf8(entry.tag@)) == Some(reason)
            && (forall|j: int| 0 <= j < i ==> policy::index_flag_admission(
                vstd::utf8::encode_utf8((#[trigger] flags@[j]).tag@)).is_none());
        if i > 1 {
            assert(policy::index_flag_admission(vstd::utf8::encode_utf8(flags@[1].tag@)).is_none());
        }
        assert(i == 1);
    }
    expect_status_text(entry.path.as_str(), "目录/首个");
    expect_status_text(reason.message(), "skip-worktree");
    let both = policy::unsupported_index_flag("s").unwrap();
    expect_status_text(both.message(), "assume-unchanged and skip-worktree");
    let assume = policy::unsupported_index_flag("h").unwrap();
    expect_status_text(assume.message(), "assume-unchanged");
    expect_status_true(policy::unsupported_index_flag("H").is_none());
    expect_status_true(policy::first_unsupported_index_flag(&[]).is_none());
}

fn expect_index_flag_error(rows: &[policy::GitIndexFlagObservation], first: usize, expected: &str)
    requires policy::index_flag_observations_bound(rows@),
        first < rows.len(),
        forall|j: int| 0 <= j < first ==> policy::index_flag_row_spec(&rows@[j]).is_ok(),
        policy::index_flag_row_spec(&rows@[first as int]) == Err::<policy::IndexFlagBytes, Seq<char>>(expected@),
{
    match policy::parse_git_index_flag_rows(rows) {
        Err(message) => {
            proof {
                let found = choose|i: int| 0 <= i < rows.len()
                    && (forall|j: int| 0 <= j < i ==> policy::index_flag_row_spec(&rows@[j]).is_ok())
                    && policy::index_flag_row_spec(&rows@[i]) == Err::<policy::IndexFlagBytes, Seq<char>>(message@);
                if found < first { assert(policy::index_flag_row_spec(&rows@[found]).is_ok()); }
                if first < found { assert(policy::index_flag_row_spec(&rows@[first as int]).is_ok()); }
                assert(found == first);
            }
            expect_status_text(message.as_str(), expected);
        },
        Ok(_) => { panic!(); },
    }
}

fn expect_index_flag_fields(flag: &policy::IndexFlag, tag: &str, path: &str)
    requires policy::index_flag_bytes(flag) == (tag.spec_bytes(), path.spec_bytes()),
{
    proof {
        vstd::utf8::encode_utf8_decode_utf8(flag.tag@);
        vstd::utf8::encode_utf8_decode_utf8(flag.path@);
        vstd::utf8::encode_utf8_decode_utf8(tag@);
        vstd::utf8::encode_utf8_decode_utf8(path@);
    }
    expect_status_text(flag.tag.as_str(), tag);
    expect_status_text(flag.path.as_str(), path);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_flags_keep_order_duplicates_case_and_complete_unicode_paths() {
    let first = "H 目录/é\n ";
    let second = "h A";
    proof {
        reveal_strlit("H 目录/é\n "); reveal_strlit("目录/é\n ");
        reveal_strlit("h A"); reveal_strlit("A"); reveal_strlit("H"); reveal_strlit("h");
        assert(first@.drop_first().drop_first() =~= "目录/é\n "@);
        assert(second@.drop_first().drop_first() =~= "A"@);
        assert(vstd::utf8::encode_scalar(72) == seq![72u8]) by (compute);
        assert(vstd::utf8::encode_scalar(104) == seq![104u8]) by (compute);
        assert(vstd::utf8::encode_scalar(32) == seq![32u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
        assert(first.spec_bytes() =~= seq![72u8, 32u8] + "目录/é\n ".spec_bytes());
        assert(second.spec_bytes() =~= seq![104u8, 32u8] + "A".spec_bytes());
        assert(first.spec_bytes().take(1) =~= "H".spec_bytes());
        assert(first.spec_bytes().skip(2) =~= "目录/é\n ".spec_bytes());
        assert(second.spec_bytes().take(1) =~= "h".spec_bytes());
        assert(second.spec_bytes().skip(2) =~= "A".spec_bytes());
    }
    let rows = vec![
        policy::GitIndexFlagObservation { raw: first.as_bytes(), text: Some(first) },
        policy::GitIndexFlagObservation { raw: second.as_bytes(), text: Some(second) },
        policy::GitIndexFlagObservation { raw: first.as_bytes(), text: Some(first) },
    ];
    match policy::parse_git_index_flag_rows(rows.as_slice()) {
        Ok(flags) => {
            expect_status_true(flags.len() == 3);
            expect_index_flag_fields(&flags[0], "H", "目录/é\n ");
            expect_index_flag_fields(&flags[1], "h", "A");
            expect_index_flag_fields(&flags[2], "H", "目录/é\n ");
            let copy = flags[0].clone();
            expect_status_true(copy.eq(&flags[0]));
            expect_status_true(!copy.eq(&flags[1]));
        },
        Err(_) => { panic!(); },
    }
}

fn expect_invalid_index_flag_header(text: &str)
    requires !policy::index_flag_header(text.spec_bytes()),
{
    let rows = vec![policy::GitIndexFlagObservation { raw: text.as_bytes(), text: Some(text) }];
    expect_index_flag_error(rows.as_slice(), 0, "invalid Git index flag record");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_flag_header_rejects_short_nonletters_and_wrong_separator() {
    proof {
        reveal_strlit(""); reveal_strlit("H"); reveal_strlit("H ");
        reveal_strlit("@ a"); reveal_strlit("[ a"); reveal_strlit("` a");
        reveal_strlit("{ a"); reveal_strlit("0 a"); reveal_strlit("é a"); reveal_strlit("H\ta");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("H")
            && vstd::string::is_ascii("H ") && vstd::string::is_ascii("@ a")
            && vstd::string::is_ascii("[ a") && vstd::string::is_ascii("` a")
            && vstd::string::is_ascii("{ a") && vstd::string::is_ascii("0 a")
            && vstd::string::is_ascii("H\ta"));
        reveal_with_fuel(vstd::utf8::encode_utf8, 2);
        assert(vstd::utf8::encode_scalar(233) == seq![195u8, 169u8]) by (compute);
    }
    expect_invalid_index_flag_header("");
    expect_invalid_index_flag_header("H");
    expect_invalid_index_flag_header("H ");
    expect_invalid_index_flag_header("@ a");
    expect_invalid_index_flag_header("[ a");
    expect_invalid_index_flag_header("` a");
    expect_invalid_index_flag_header("{ a");
    expect_invalid_index_flag_header("0 a");
    expect_invalid_index_flag_header("é a");
    expect_invalid_index_flag_header("H\ta");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_flag_format_and_utf8_errors_select_the_first_record() {
    let invalid_bytes = vec![72u8, 32u8, 255u8];
    let malformed_bytes = vec![255u8, 32u8, 120u8];
    let valid = "Z path";
    proof {
        reveal_strlit("Z path");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii(valid));
    }
    let rows = vec![
        policy::GitIndexFlagObservation { raw: valid.as_bytes(), text: Some(valid) },
        policy::GitIndexFlagObservation { raw: invalid_bytes.as_slice(), text: None },
        policy::GitIndexFlagObservation { raw: malformed_bytes.as_slice(), text: None },
    ];
    proof {
        assert(policy::index_flag_row_spec(&rows@[0]).is_ok());
        assert(policy::index_flag_row_spec(&rows@[1]) ==
            Err::<policy::IndexFlagBytes, Seq<char>>("E_INPUT_UNSUPPORTED: non UTF-8 Git output"@));
    }
    expect_index_flag_error(rows.as_slice(), 1, "E_INPUT_UNSUPPORTED: non UTF-8 Git output");
    let reversed = vec![
        policy::GitIndexFlagObservation { raw: malformed_bytes.as_slice(), text: None },
        policy::GitIndexFlagObservation { raw: invalid_bytes.as_slice(), text: None },
    ];
    expect_index_flag_error(reversed.as_slice(), 0, "invalid Git index flag record");
}

fn expect_single_index_flag(text: &str, tag: &str, path: &str)
    requires policy::index_flag_header(text.spec_bytes()),
        text.spec_bytes().take(1) == tag.spec_bytes(),
        text.spec_bytes().skip(2) == path.spec_bytes(),
{
    let row = policy::GitIndexFlagObservation { raw: text.as_bytes(), text: Some(text) };
    match policy::parse_git_index_flag_row(&row) {
        Ok(flag) => { expect_index_flag_fields(&flag, tag, path); },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_flag_ascii_letter_endpoints_are_all_accepted() {
    proof {
        reveal_strlit("A a"); reveal_strlit("Z z"); reveal_strlit("a a"); reveal_strlit("z z");
        reveal_strlit("A"); reveal_strlit("Z"); reveal_strlit("a"); reveal_strlit("z");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("A a") && vstd::string::is_ascii("Z z")
            && vstd::string::is_ascii("a a") && vstd::string::is_ascii("z z")
            && vstd::string::is_ascii("A") && vstd::string::is_ascii("Z")
            && vstd::string::is_ascii("a") && vstd::string::is_ascii("z"));
        assert("A a".spec_bytes().take(1) =~= "A".spec_bytes());
        assert("A a".spec_bytes().skip(2) =~= "a".spec_bytes());
        assert("Z z".spec_bytes().take(1) =~= "Z".spec_bytes());
        assert("Z z".spec_bytes().skip(2) =~= "z".spec_bytes());
        assert("a a".spec_bytes().take(1) =~= "a".spec_bytes());
        assert("a a".spec_bytes().skip(2) =~= "a".spec_bytes());
        assert("z z".spec_bytes().take(1) =~= "z".spec_bytes());
        assert("z z".spec_bytes().skip(2) =~= "z".spec_bytes());
    }
    expect_single_index_flag("A a", "A", "a");
    expect_single_index_flag("Z z", "Z", "z");
    expect_single_index_flag("a a", "a", "a");
    expect_single_index_flag("z z", "z", "z");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn index_flag_framing_ignores_empty_records_and_keeps_unterminated_tail() {
    proof {
        reveal_strlit("\0H a\0\0h b\0H a"); reveal_strlit("H a"); reveal_strlit("h b");
        reveal_strlit("\0\0"); reveal_strlit("");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("\0H a\0\0h b\0H a")
            && vstd::string::is_ascii("H a") && vstd::string::is_ascii("h b")
            && vstd::string::is_ascii("\0\0") && vstd::string::is_ascii(""));
        let raw = "\0H a\0\0h b\0H a".spec_bytes();
        scope_text::ascii_separator_end_is(raw, 0, 0);
        scope_text::ascii_separator_end_is(raw.skip(1), 0, 3);
        scope_text::ascii_separator_end_is(raw.skip(5), 0, 0);
        scope_text::ascii_separator_end_is(raw.skip(6), 0, 3);
        scope_text::ascii_separator_end_is(raw.skip(10), 0, 3);
        assert(raw.skip(1).skip(4) =~= raw.skip(5));
        assert(raw.skip(5).skip(1) =~= raw.skip(6));
        assert(raw.skip(6).skip(4) =~= raw.skip(10));
        assert(raw.skip(1).take(3) =~= "H a".spec_bytes());
        assert(raw.skip(6).take(3) =~= "h b".spec_bytes());
        assert(raw.skip(10).take(3) =~= "H a".spec_bytes());
        scope_text::ascii_separator_end_is("\0\0".spec_bytes(), 0, 0);
        scope_text::ascii_separator_end_is("\0\0".spec_bytes().skip(1), 0, 0);
        scope_text::ascii_separator_end_is("".spec_bytes(), 0, 0);
        reveal_with_fuel(scope_text::ascii_split_records, 6);
        assert(scope_text::ascii_split_records(raw, 0, false)
            =~= seq!["H a".spec_bytes(), "h b".spec_bytes(), "H a".spec_bytes()]);
    }
    let records = policy::git_nul_records("\0H a\0\0h b\0H a".as_bytes());
    expect_status_true(records.len() == 3);
    expect_status_true(scope_text::equal_bytes(records[0], "H a".as_bytes()));
    expect_status_true(scope_text::equal_bytes(records[1], "h b".as_bytes()));
    expect_status_true(scope_text::equal_bytes(records[2], "H a".as_bytes()));
    expect_status_true(policy::git_nul_records("\0\0".as_bytes()).len() == 0);
    expect_status_true(policy::git_nul_records("".as_bytes()).len() == 0);
    let rows: Vec<policy::GitIndexFlagObservation> = Vec::new();
    match policy::parse_git_index_flag_rows(rows.as_slice()) {
        Ok(flags) => { expect_status_true(flags.len() == 0); },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_path_records_filter_only_empty_items_and_keep_original_bytes() {
    proof {
        reveal_strlit("\0z\0\0a\n \0z"); reveal_strlit("z"); reveal_strlit("a\n ");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("\0z\0\0a\n \0z")
            && vstd::string::is_ascii("z") && vstd::string::is_ascii("a\n "));
        let raw = "\0z\0\0a\n \0z".spec_bytes();
        scope_text::ascii_separator_end_is(raw, 0, 0);
        scope_text::ascii_separator_end_is(raw.skip(1), 0, 1);
        scope_text::ascii_separator_end_is(raw.skip(3), 0, 0);
        scope_text::ascii_separator_end_is(raw.skip(4), 0, 3);
        scope_text::ascii_separator_end_is(raw.skip(8), 0, 1);
        assert(raw.skip(1).skip(2) =~= raw.skip(3));
        assert(raw.skip(3).skip(1) =~= raw.skip(4));
        assert(raw.skip(4).skip(4) =~= raw.skip(8));
        assert(raw.skip(1).take(1) =~= "z".spec_bytes());
        assert(raw.skip(4).take(3) =~= "a\n ".spec_bytes());
        assert(raw.skip(8).take(1) =~= "z".spec_bytes());
        reveal_with_fuel(scope_text::ascii_split_records, 6);
        assert(scope_text::ascii_split_records(raw, 0, false)
            =~= seq!["z".spec_bytes(), "a\n ".spec_bytes(), "z".spec_bytes()]);
    }
    let records = policy::git_nul_records("\0z\0\0a\n \0z".as_bytes());
    expect_status_true(records.len() == 3);
    expect_status_true(scope_text::equal_bytes(records[0], "z".as_bytes()));
    expect_status_true(scope_text::equal_bytes(records[1], "a\n ".as_bytes()));
    expect_status_true(scope_text::equal_bytes(records[2], "z".as_bytes()));
    let raw = vec![255u8, 0u8, 254u8];
    proof {
        assert(raw@.take(1) =~= seq![255u8]);
        assert(raw@.skip(2) =~= seq![254u8]);
        assert(raw@.skip(2).take(1) =~= seq![254u8]);
        scope_text::ascii_separator_end_is(raw@, 0, 1);
        scope_text::ascii_separator_end_is(raw@.skip(2), 0, 1);
        reveal_with_fuel(scope_text::ascii_split_records, 3);
        assert(scope_text::ascii_split_records(raw@, 0, false) =~= seq![seq![255u8], seq![254u8]]);
    }
    let records = policy::git_nul_records(raw.as_slice());
    expect_status_true(records.len() == 2);
    expect_status_true(records[0].len() == 1 && records[0][0] == 255);
    expect_status_true(records[1].len() == 1 && records[1][0] == 254);
}

fn expect_complete_git_text(text: &str) {
    match policy::git_output_text(Some(text.to_owned())) {
        Ok(actual) => { expect_status_text(actual.as_str(), text); },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_output_text_keeps_empty_and_complete_decoded_output() {
    expect_complete_git_text("");
    expect_complete_git_text(" a\0目录/é\n ");
    match policy::git_output_text(None) {
        Ok(_) => { panic!(); },
        Err(message) => {
            expect_status_text(message.as_str(), "E_INPUT_UNSUPPORTED: non UTF-8 Git output");
        },
    }
}

fn expect_git_text_row_sequence(first: &str, second: &str) {
    let rows = vec![Some(first.to_owned()), Some(second.to_owned()), Some(first.to_owned())];
    match policy::parse_git_text_rows(rows) {
        Ok(paths) => {
            expect_status_true(paths.len() == 3);
            expect_status_text(paths[0].as_str(), first);
            expect_status_text(paths[1].as_str(), second);
            expect_status_text(paths[2].as_str(), first);
        },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_text_rows_keep_complete_order_duplicates_and_empty_text() {
    let rows: Vec<Option<String>> = Vec::new();
    match policy::parse_git_text_rows(rows) {
        Ok(paths) => { expect_status_true(paths.len() == 0); },
        Err(_) => { panic!(); },
    }
    expect_git_text_row_sequence("z", "a");
    expect_git_text_row_sequence("", "目录/é\0\n ");
}

fn expect_git_text_row_failure(rows: Vec<Option<String>>)
    requires exists|i: int| 0 <= i < rows.len() && (#[trigger] rows@[i]).is_none(),
{
    match policy::parse_git_text_rows(rows) {
        Ok(_) => { panic!(); },
        Err(message) => {
            expect_status_text(message.as_str(), "E_INPUT_UNSUPPORTED: non UTF-8 Git output");
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_text_rows_reject_missing_decodes_at_every_boundary() {
    expect_git_text_row_failure(vec![None]);
    expect_git_text_row_failure(vec![None, Some("later".to_owned()), None]);
    expect_git_text_row_failure(vec![Some("first".to_owned()), None, Some("later".to_owned())]);
    expect_git_text_row_failure(vec![Some("first".to_owned()), Some("second".to_owned()), None]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_inventory_keeps_entry_order_and_repeated_paths() {
    proof {
        registry_fixture_paths();
    }
    let rows = [Some("z"), Some("a//./"), Some("z"), Some("目录/é")];
    match policy::collect_registry_paths("", &rows) {
        Ok(paths) => {
            expect_status_true(paths.len() == 5);
            expect_status_text(paths[0].as_str(), "");
            expect_status_text(paths[1].as_str(), "z");
            expect_status_text(paths[2].as_str(), "a//./");
            expect_status_text(paths[3].as_str(), "z");
            expect_status_text(paths[4].as_str(), "目录/é");
        },
        Err(_) => { panic!(); },
    }
    let rows = [Some("a"); 4];
    match policy::collect_registry_paths("../entry\0", &rows) {
        Ok(paths) => {
            expect_status_true(paths.len() == 5);
            expect_status_text(paths[0].as_str(), "../entry\0");
        },
        Err(_) => { panic!(); },
    }
}

proof fn registry_fixture_paths()
    ensures policy::paths::relative_path("a".spec_bytes()),
        policy::paths::relative_path("z".spec_bytes()),
        policy::paths::relative_path("a//./".spec_bytes()),
        policy::paths::relative_path("目录/é".spec_bytes()),
{
    reveal_strlit("a"); reveal_strlit("z"); reveal_strlit("a//./");
    broadcast use vstd::string::is_ascii_spec_bytes;
    assert(vstd::string::is_ascii("a") && vstd::string::is_ascii("z")
        && vstd::string::is_ascii("a//./"));
    reveal_strlit("目录/é");
    assert("目录/é"@ =~= seq!['目', '录', '/', 'é']);
    assert('目' as u32 == 30446 && '录' as u32 == 24405
        && 'é' as u32 == 233 && '/' as u32 == 47);
    assert(vstd::utf8::encode_scalar(30446) == seq![231u8, 155, 174]) by (compute);
    assert(vstd::utf8::encode_scalar(24405) == seq![229u8, 189, 149]) by (compute);
    assert(vstd::utf8::encode_scalar(233) == seq![195u8, 169]) by (compute);
    assert(vstd::utf8::encode_scalar(47) == seq![47u8]) by (compute);
    reveal_with_fuel(vstd::utf8::encode_utf8, 5);
    assert("目录/é".spec_bytes() =~= seq![231u8, 155, 174, 229, 189, 149, 47, 195, 169]);
}

proof fn registry_error_is_first(error: &policy::RegistryPathError, rows: Seq<Option<&str>>, first: int)
    requires rows.len() == 4, 0 <= first < 4,
        !policy::registry_path_valid(rows[first]),
        forall|j: int| 0 <= j < first ==> policy::registry_path_valid(rows[j]),
        exists|i: int| 0 <= i < 4
            && (forall|j: int| 0 <= j < i ==> policy::registry_path_valid(rows[j]))
            && policy::registry_error_at(error, i, rows[i]),
    ensures policy::registry_error_at(error, first, rows[first]),
{
    let found = choose|i: int| 0 <= i < 4
        && (forall|j: int| 0 <= j < i ==> policy::registry_path_valid(rows[j]))
        && policy::registry_error_at(error, i, rows[i]);
    assert(!policy::registry_path_valid(rows[found]));
    if found < first { assert(policy::registry_path_valid(rows[found])); }
    if first < found { assert(policy::registry_path_valid(rows[first])); }
    assert(found == first);
}

fn expect_missing_registry(rows: [Option<&str>; 4], first: usize, expected: &str)
    requires first < 4, rows@[first as int].is_none(),
        forall|j: int| 0 <= j < first ==> policy::registry_path_valid(rows@[j]),
        policy::registry_missing_message(first as int) == expected@,
{
    match policy::collect_registry_paths("entry", &rows) {
        Err(error) => {
            proof { registry_error_is_first(&error, rows@, first as int); }
            match error {
                policy::RegistryPathError::Missing { message } => {
                    expect_status_text(message, expected);
                },
                _ => { panic!(); },
            }
        },
        Ok(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_inventory_reports_first_missing_field_in_fixed_order() {
    proof { registry_fixture_paths(); }
    expect_missing_registry([None, Some("../bad"), None, None], 0, "missing registry path judges");
    expect_missing_registry([Some("a"), None, Some("/bad"), None], 1, "missing registry path projects");
    expect_missing_registry([Some("a"), Some("a"), None, Some("")], 2, "missing registry path filemap");
    expect_missing_registry([Some("a"), Some("a"), Some("a"), None], 3, "missing registry path workflow");
}

fn expect_invalid_registry(path: &str)
    requires !policy::paths::relative_path(path.spec_bytes()),
{
    proof { registry_fixture_paths(); }
    let rows = [Some("a"), Some(path), None, Some("../later")];
    match policy::collect_registry_paths("entry", &rows) {
        Err(error) => {
            proof { registry_error_is_first(&error, rows@, 1); }
            match error {
                policy::RegistryPathError::Invalid { path: actual } => {
                    expect_status_text(actual, path);
                },
                _ => { panic!(); },
            }
        },
        _ => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_inventory_rejects_invalid_path_before_later_missing_field() {
    proof {
        reveal_strlit(""); reveal_strlit("/a"); reveal_strlit("./a"); reveal_strlit("a/../b");
        reveal_strlit("a\0b"); reveal_strlit("a\nb"); reveal_strlit("a\rb"); reveal_strlit("a\\b");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("/a")
            && vstd::string::is_ascii("./a") && vstd::string::is_ascii("a/../b")
            && vstd::string::is_ascii("a\0b") && vstd::string::is_ascii("a\nb")
            && vstd::string::is_ascii("a\rb") && vstd::string::is_ascii("a\\b"));
        assert(policy::paths::parent_component_at("a/../b".spec_bytes(), 2));
        assert("a/../b".spec_bytes()[2] == 46);
        assert("a\0b".spec_bytes()[1] == 0);
        assert("a\nb".spec_bytes()[1] == 10);
        assert("a\rb".spec_bytes()[1] == 13);
        assert("a\\b".spec_bytes()[1] == 92);
    }
    expect_invalid_registry("");
    expect_invalid_registry("/a");
    expect_invalid_registry("./a");
    expect_invalid_registry("a/../b");
    expect_invalid_registry("a\0b");
    expect_invalid_registry("a\nb");
    expect_invalid_registry("a\rb");
    expect_invalid_registry("a\\b");
}

fn expect_relative_admission(path: &str, valid: bool)
    requires valid == policy::paths::relative_path(path.spec_bytes()),
{
    match policy::admit_relative_path(path) {
        Ok(()) => { expect_status_true(valid); },
        Err(actual) => {
            expect_status_true(!valid);
            expect_status_text(actual, path);
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn relative_path_results_and_registry_renderer_keep_complete_text() {
    proof {
        registry_fixture_paths();
        reveal_strlit(""); reveal_strlit("./a"); reveal_strlit("a/../b");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("./a")
            && vstd::string::is_ascii("a/../b"));
        assert(policy::paths::parent_component_at("a/../b".spec_bytes(), 2));
        assert("a/../b".spec_bytes()[2] == 46);
    }
    expect_relative_admission("a//./", true);
    expect_relative_admission("目录/é", true);
    expect_relative_admission("", false);
    expect_relative_admission("./a", false);
    expect_relative_admission("a/../b", false);
    let invalid = policy::RegistryPathError::Invalid { path: "../目录\0\n" };
    let message = invalid.render(|path: &str| -> (out: String)
        ensures out@ == path@,
    { path.to_owned() });
    expect_status_text(message.as_str(), "../目录\0\n");
    let missing = policy::RegistryPathError::Missing { message: "missing registry path filemap" };
    let message = missing.render(|path: &str| -> (out: String)
        ensures out@ == path@,
    { path.to_owned() });
    expect_status_text(message.as_str(), "missing registry path filemap");
}

spec fn registry_decode_model(bytes: Seq<u8>) -> Result<u64, Seq<char>> {
    if bytes.len() == 0 { Err("空输入\0\n"@) }
    else if bytes[0] == 255 { Err("解码失败：é\0\n"@) }
    else { Ok(bytes[0] as u64) }
}

fn decode_registry_fixture(bytes: &[u8]) -> (out: Result<u64, String>)
    ensures policy::registry_decoded_view(out) == registry_decode_model(bytes@),
{
    if bytes.len() == 0 { Err("空输入\0\n".to_owned()) }
    else if bytes[0] == 255 { Err("解码失败：é\0\n".to_owned()) }
    else { Ok(bytes[0] as u64) }
}

fn never_decode_registry(bytes: &[u8]) -> (out: Result<u64, String>)
    requires false,
    ensures false,
{
    panic!();
}

// 同一被测函数；只有 Verus 调用携带不会进入普通 Rust 的证明模型。
fn registry_fixture_batch<P>(
    expected: &[String], originals: Vec<(String, Vec<u8>)>,
    bytes: std::collections::BTreeMap<String, Vec<u8>>,
    values: std::collections::BTreeMap<String, u64>, decode: P,
) -> (out: Result<(std::collections::BTreeMap<String, Vec<u8>>,
    std::collections::BTreeMap<String, u64>), String>)
    where P: Fn(&[u8]) -> Result<u64, String>,
    requires vstd::laws_cmp::obeys_cmp::<String>(),
        forall|i: int, raw: &[u8]| 0 <= i < originals.len()
            && policy::registry_batch_prefix(expected@, originals@, |raw: Seq<u8>| registry_decode_model(raw), i)
            && expected@[i]@ == originals@[i].0@ && raw@ == originals@[i].1@
            ==> decode.requires((raw,)),
        forall|raw: &[u8], result: Result<u64, String>|
            decode.requires((raw,)) && decode.ensures((raw,), result)
            ==> policy::registry_decoded_view(result) == registry_decode_model(raw@),
    ensures policy::registry_batch_view(out) == policy::registry_batch_spec(
        expected@, originals@, bytes@, values@, |raw: Seq<u8>| registry_decode_model(raw)),
{
    #[cfg(verus_keep_ghost)]
    let out = policy::registry_parse_batch(expected, originals, bytes, values, decode,
        Ghost(|raw: Seq<u8>| registry_decode_model(raw)));
    #[cfg(not(verus_keep_ghost))]
    let out = policy::registry_parse_batch(expected, originals, bytes, values, decode);
    out
}

fn expect_registry_batch_error(
    actual: Result<(std::collections::BTreeMap<String, Vec<u8>>,
        std::collections::BTreeMap<String, u64>), String>, expected: &str,
)
    requires actual matches Err(message) && message@ == expected@,
{
    match actual {
        Err(message) => { expect_status_text(message.as_str(), expected); },
        Ok(_) => { panic!(); },
    }
}

fn expect_registry_pair(
    bytes: &std::collections::BTreeMap<String, Vec<u8>>,
    values: &std::collections::BTreeMap<String, u64>,
    key: &String, original: &[u8], value: u64,
)
    requires vstd::laws_cmp::obeys_cmp::<String>(),
        bytes@.contains_key(*key), bytes@[*key]@ == original@,
        values@.contains_key(*key), values@[*key] == value,
{
    broadcast use {vstd::std_specs::btree::axiom_maps_deref_key_to_value,
        vstd::std_specs::btree::axiom_contains_deref_key};
    match bytes.get(key) {
        Some(actual) => { expect_status_true(scope_text::equal_bytes(actual.as_slice(), original)); },
        None => { panic!(); },
    }
    match values.get(key) {
        Some(actual) => { expect_status_true(*actual == value); },
        None => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_parse_batch_empty_preserves_both_existing_maps()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let key = "已有/é\0\n".to_owned();
    let mut bytes = std::collections::BTreeMap::new();
    let mut values = std::collections::BTreeMap::new();
    bytes.insert(key.clone(), vec![0u8, 255, 10]);
    values.insert(key.clone(), u64::MAX);
    let result = registry_fixture_batch(&[], Vec::new(), bytes, values,
        never_decode_registry);
    match result {
        Ok((bytes, values)) => {
            expect_status_true(bytes.len() == 1 && values.len() == 1);
            expect_registry_pair(&bytes, &values, &key, &[0u8, 255, 10], u64::MAX);
        },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_parse_batch_missing_rows_rejects_before_decoding()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let expected = vec!["a".to_owned()];
    let result = registry_fixture_batch(&expected, Vec::new(),
        std::collections::BTreeMap::new(), std::collections::BTreeMap::new(),
        never_decode_registry);
    expect_registry_batch_error(result, "registry acquisition count mismatch");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_parse_batch_extra_rows_rejects_before_path_or_decode()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let originals = vec![("unexpected".to_owned(), Vec::new())];
    let result = registry_fixture_batch(&[], originals,
        std::collections::BTreeMap::new(), std::collections::BTreeMap::new(),
        never_decode_registry);
    expect_registry_batch_error(result, "registry acquisition count mismatch");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_parse_batch_wrong_path_rejects_before_current_decode()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof {
        reveal_strlit("a"); reveal_strlit("a\0");
        assert("a"@.len() != "a\0"@.len());
    }
    let expected = vec!["a".to_owned()];
    let originals = vec![("a\0".to_owned(), Vec::new())];
    let result = registry_fixture_batch(&expected, originals,
        std::collections::BTreeMap::new(), std::collections::BTreeMap::new(),
        never_decode_registry);
    expect_registry_batch_error(result, "registry acquisition path mismatch");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_parse_batch_preserves_complete_empty_decode_error()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let key = "".to_owned();
    let expected = vec![key.clone()];
    let originals = vec![(key, Vec::new())];
    let result = registry_fixture_batch(&expected, originals,
        std::collections::BTreeMap::new(), std::collections::BTreeMap::new(),
        decode_registry_fixture);
    expect_registry_batch_error(result, "空输入\0\n");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_parse_batch_decode_error_precedes_later_wrong_path()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let key = "first".to_owned();
    let expected = vec![key.clone(), "later".to_owned()];
    let originals = vec![(key, vec![255u8, 0, 10]), ("wrong".to_owned(), Vec::new())];
    let result = registry_fixture_batch(&expected, originals,
        std::collections::BTreeMap::new(), std::collections::BTreeMap::new(),
        decode_registry_fixture);
    expect_registry_batch_error(result, "解码失败：é\0\n");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_parse_batch_path_error_after_success_precedes_decode_error()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof {
        reveal_strlit("a"); reveal_strlit("A");
        assert("a"@[0] != "A"@[0]);
        reveal_with_fuel(policy::registry_batch_fold, 3);
    }
    let first = "first".to_owned();
    let expected = vec![first.clone(), "a".to_owned()];
    let originals = vec![(first, vec![1u8]), ("A".to_owned(), vec![255u8])];
    let result = registry_fixture_batch(&expected, originals,
        std::collections::BTreeMap::new(), std::collections::BTreeMap::new(),
        decode_registry_fixture);
    expect_registry_batch_error(result, "registry acquisition path mismatch");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_parse_batch_success_preserves_full_bytes_and_unrelated_entries()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof {
        reveal_strlit("entry"); reveal_strlit("../目录/é\0\n");
        assert("entry"@.len() != "../目录/é\0\n"@.len());
        reveal_with_fuel(policy::registry_batch_fold, 3);
    }
    let old = "entry".to_owned();
    let key = "../目录/é\0\n".to_owned();
    let mut bytes = std::collections::BTreeMap::new();
    let mut values = std::collections::BTreeMap::new();
    bytes.insert(old.clone(), vec![9u8]);
    values.insert(old.clone(), u64::MAX);
    let expected = vec![key.clone()];
    let originals = vec![(key.clone(), vec![0u8, 255, 10, 13, 254])];
    let result = registry_fixture_batch(&expected, originals, bytes, values,
        decode_registry_fixture);
    match result {
        Ok((bytes, values)) => {
            expect_status_true(bytes.len() == 2 && values.len() == 2);
            expect_registry_pair(&bytes, &values, &old, &[9u8], u64::MAX);
            expect_registry_pair(&bytes, &values, &key, &[0u8, 255, 10, 13, 254], 0);
        },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_parse_batch_duplicate_keys_replace_both_maps_in_order()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof { reveal_with_fuel(policy::registry_batch_fold, 4); }
    let key = "same".to_owned();
    let mut bytes = std::collections::BTreeMap::new();
    let mut values = std::collections::BTreeMap::new();
    bytes.insert(key.clone(), vec![3u8]);
    values.insert(key.clone(), 3);
    let expected = vec![key.clone(), key.clone()];
    let originals = vec![(key.clone(), vec![4u8, 0]), (key.clone(), vec![254u8, 255, 0])];
    let result = registry_fixture_batch(&expected, originals, bytes, values,
        decode_registry_fixture);
    match result {
        Ok((bytes, values)) => {
            expect_status_true(bytes.len() == 1 && values.len() == 1);
            expect_registry_pair(&bytes, &values, &key, &[254u8, 255, 0], 254);
        },
        Err(_) => { panic!(); },
    }
}

spec fn registry_sequence_original(out: Result<Vec<u8>, String>) -> Result<Seq<u8>, Seq<char>> {
    match out { Ok(raw) => Ok(raw@), Err(error) => Err(error@) }
}

spec fn registry_sequence_read_model(state: u8, path: Seq<char>) -> Result<Seq<u8>, Seq<char>> {
    if state == 255 { Err("unexpected extra read"@) }
    else if path == "read-error"@ { Err("读取失败：é\0\n"@) }
    else if path == "empty"@ { Ok(Seq::empty()) }
    else if path == "decode-error"@ { Ok(seq![255u8, 0, 10]) }
    else if state == 0 { Ok(seq![1u8, 0, 255]) }
    else { Ok(seq![2u8, 255, 0]) }
}

fn registry_sequence_read(state: u8, path: &str) -> (out: (u8, Result<Vec<u8>, String>))
    ensures out.0 as int == if state == 255 { 255 } else { state as int + 1 },
        registry_sequence_original(out.1) == registry_sequence_read_model(state, path@),
{
    if state == 255 { return (state, Err("unexpected extra read".to_owned())); }
    let next = state + 1;
    if scope_text::equal_text(path, "read-error") { (next, Err("读取失败：é\0\n".to_owned())) }
    else if scope_text::equal_text(path, "empty") { (next, Ok(Vec::new())) }
    else if scope_text::equal_text(path, "decode-error") { (next, Ok(vec![255u8, 0, 10])) }
    else if state == 0 { (next, Ok(vec![1u8, 0, 255])) }
    else { (next, Ok(vec![2u8, 255, 0])) }
}

spec fn registry_sequence_bytes(bytes: Map<String, Vec<u8>>) -> Map<String, Seq<u8>> {
    Map::new(|key: String| bytes.contains_key(key), |key: String| bytes[key]@)
}

spec fn registry_sequence_view(out: (u8, Result<(std::collections::BTreeMap<String, Vec<u8>>,
    std::collections::BTreeMap<String, u64>), String>))
    -> (u8, Result<(Map<String, Seq<u8>>, Map<String, u64>), Seq<char>>)
{
    (out.0, match out.1 {
        Ok((bytes, values)) => Ok((registry_sequence_bytes(bytes@), values@)),
        Err(error) => Err(error@),
    })
}

// 独立读取器和解析器的参考结果；具体用例另外断言完整错误、读取次数和两个 map。
spec fn registry_sequence_expected(paths: Seq<String>, at: int,
    bytes: Map<String, Seq<u8>>, values: Map<String, u64>)
    -> (u8, Result<(Map<String, Seq<u8>>, Map<String, u64>), Seq<char>>)
    recommends 0 <= at <= paths.len() <= 3,
    decreases paths.len() - at,
{
    if at >= paths.len() { (at as u8, Ok((bytes, values))) }
    else {
        match registry_sequence_read_model(at as u8, paths[at]@) {
            Err(error) => ((at + 1) as u8, Err(error)),
            Ok(raw) => match registry_decode_model(raw) {
                Err(error) => ((at + 1) as u8, Err(error)),
                Ok(value) => registry_sequence_expected(paths, at + 1,
                    bytes.insert(paths[at], raw), values.insert(paths[at], value)),
            },
        }
    }
}

proof fn registry_sequence_bytes_insert(bytes: Map<String, Vec<u8>>, key: String, raw: Vec<u8>)
    ensures registry_sequence_bytes(bytes.insert(key, raw))
        == registry_sequence_bytes(bytes).insert(key, raw@),
{
    assert(registry_sequence_bytes(bytes.insert(key, raw))
        =~= registry_sequence_bytes(bytes).insert(key, raw@));
}

proof fn registry_sequence_bytes_facts(bytes: Map<String, Vec<u8>>)
    ensures registry_sequence_bytes(bytes).dom() == bytes.dom(),
        forall|key: String| #[trigger] bytes.contains_key(key)
            ==> registry_sequence_bytes(bytes)[key] == bytes[key]@,
{
    assert(registry_sequence_bytes(bytes).dom() =~= bytes.dom());
}

proof fn registry_sequence_fixture_step(paths: Seq<String>,
    before: (u8, Map<String, Vec<u8>>, Map<String, u64>),
    after: (u8, Map<String, Vec<u8>>, Map<String, u64>))
    requires before.0 < paths.len() <= 3,
        policy::registry_read_sequence_step(before, after, paths[before.0 as int],
            registry_sequence_read, decode_registry_fixture),
    ensures after.0 as int == before.0 as int + 1,
        registry_sequence_expected(paths, before.0 as int, registry_sequence_bytes(before.1), before.2)
            == registry_sequence_expected(paths, after.0 as int, registry_sequence_bytes(after.1), after.2),
{
    reveal(policy::registry_read_sequence_step);
    let (path, raw, input, value) = choose|path: &str, raw: Vec<u8>, input: &[u8], value: u64|
        path@ == paths[before.0 as int]@ && input@ == raw@
        && registry_sequence_read.ensures((before.0, path), (after.0, Ok::<Vec<u8>, String>(raw)))
        && decode_registry_fixture.ensures((input,), Ok::<u64, String>(value))
        && after.1 == before.1.insert(paths[before.0 as int], raw)
        && after.2 == before.2.insert(paths[before.0 as int], value);
    registry_sequence_bytes_insert(before.1, paths[before.0 as int], raw);
    reveal_with_fuel(registry_sequence_expected, 1);
}

proof fn registry_sequence_fixture_prefix(paths: Seq<String>,
    states: Seq<(u8, Map<String, Vec<u8>>, Map<String, u64>)>,
    initial: (u8, Map<String, Vec<u8>>, Map<String, u64>), at: int)
    requires initial.0 == 0, paths.len() <= 3, 0 <= at < states.len(),
        policy::registry_read_sequence_prefix(states, initial, paths,
            registry_sequence_read, decode_registry_fixture),
    ensures states[at].0 as int == at,
        registry_sequence_expected(paths, 0, registry_sequence_bytes(initial.1), initial.2)
            == registry_sequence_expected(paths, at, registry_sequence_bytes(states[at].1), states[at].2),
    decreases at,
{
    if at > 0 {
        registry_sequence_fixture_prefix(paths, states, initial, at - 1);
        assert(policy::registry_read_sequence_step(states[at - 1], states[at], paths[at - 1],
            registry_sequence_read, decode_registry_fixture));
        registry_sequence_fixture_step(paths, states[at - 1], states[at]);
    }
}

proof fn registry_sequence_fixture_result(paths: Seq<String>,
    bytes: Map<String, Vec<u8>>, values: Map<String, u64>,
    out: (u8, Result<(std::collections::BTreeMap<String, Vec<u8>>,
        std::collections::BTreeMap<String, u64>), String>))
    requires paths.len() <= 3,
        policy::registry_read_sequence_result((0u8, bytes, values), paths,
            registry_sequence_read, decode_registry_fixture, out),
    ensures registry_sequence_view(out)
        == registry_sequence_expected(paths, 0, registry_sequence_bytes(bytes), values),
{
    let initial = (0u8, bytes, values);
    let states = choose|states: Seq<(u8, Map<String, Vec<u8>>, Map<String, u64>)>|
        policy::registry_read_sequence_prefix(states, initial, paths,
            registry_sequence_read, decode_registry_fixture)
        && policy::registry_read_sequence_terminal(states.last(), states.len() - 1, paths,
            registry_sequence_read, decode_registry_fixture, out);
    registry_sequence_fixture_prefix(paths, states, initial, states.len() - 1);
    let at = states.len() - 1;
    let before = states.last();
    reveal(policy::registry_read_sequence_terminal);
    reveal_with_fuel(registry_sequence_expected, 1);
    match out.1 {
        Ok(_) => {},
        Err(error) => {
            let (path, observation) = choose|path: &str, observation: (u8, Result<Vec<u8>, String>)|
                path@ == paths[at]@
                && registry_sequence_read.ensures((before.0, path), observation)
                && out.0 == observation.0
                && match observation.1 {
                    Err(original) => error@ == original@,
                    Ok(raw) => exists|input: &[u8]| input@ == raw@
                        && decode_registry_fixture.ensures((input,), Err::<u64, String>(error)),
                };
            match observation.1 {
                Err(_) => {},
                Ok(raw) => {
                    let input = choose|input: &[u8]| input@ == raw@
                        && decode_registry_fixture.ensures((input,), Err::<u64, String>(error));
                },
            }
        },
    }
}

proof fn registry_sequence_fixture_texts()
    ensures "read-error"@.len() == 10, "empty"@.len() == 5, "decode-error"@.len() == 12,
        "first"@.len() == 5, "first"@ != "empty"@,
        "later"@.len() == 5, "later"@ != "empty"@, "same"@.len() == 4,
        "旧/é"@.len() == 3, "../目录/é\0\n"@.len() == 9,
{
    reveal_strlit("read-error"); reveal_strlit("empty"); reveal_strlit("decode-error");
    reveal_strlit("first"); reveal_strlit("later"); reveal_strlit("same");
    reveal_strlit("旧/é"); reveal_strlit("../目录/é\0\n");
    assert("first"@[0] != "empty"@[0]);
    assert("later"@[0] != "empty"@[0]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_read_sequence_empty_keeps_initial_maps_and_reader()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let key = "旧/é".to_owned();
    let mut bytes = std::collections::BTreeMap::new();
    let mut values = std::collections::BTreeMap::new();
    bytes.insert(key.clone(), vec![255u8, 0, 10]);
    values.insert(key.clone(), u64::MAX);
    let ghost initial = (bytes@, values@);
    let out = policy::registry_read_sequence(Vec::new(), 0u8, bytes, values,
        registry_sequence_read, decode_registry_fixture);
    proof { registry_sequence_fixture_result(Seq::empty(), initial.0, initial.1, out); }
    expect_status_true(out.0 == 0);
    match out.1 {
        Ok((bytes, values)) => {
            expect_status_true(bytes.len() == 1 && values.len() == 1);
            expect_registry_pair(&bytes, &values, &key, &[255u8, 0, 10], u64::MAX);
        },
        Err(_) => { panic!(); },
    }
}

fn expect_registry_sequence_fixture_error(paths: Vec<String>, count: u8, expected: &str)
    requires vstd::laws_cmp::obeys_cmp::<String>(), paths.len() <= 3,
        registry_sequence_expected(paths@, 0, Map::empty(), Map::empty())
            == (count, Err::<(Map<String, Seq<u8>>, Map<String, u64>), Seq<char>>(expected@)),
{
    let ghost requested = paths@;
    let out = policy::registry_read_sequence(paths, 0u8,
        std::collections::BTreeMap::new(), std::collections::BTreeMap::new(),
        registry_sequence_read, decode_registry_fixture);
    proof {
        assert(registry_sequence_bytes(Map::<String, Vec<u8>>::empty())
            =~= Map::<String, Seq<u8>>::empty());
        registry_sequence_fixture_result(requested, Map::empty(), Map::empty(), out);
    }
    expect_status_true(out.0 == count);
    expect_registry_batch_error(out.1, expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_read_sequence_first_read_error_stops_before_parse_or_later_read()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof { registry_sequence_fixture_texts(); reveal_with_fuel(registry_sequence_expected, 4); }
    expect_registry_sequence_fixture_error(vec!["read-error".to_owned(), "decode-error".to_owned()],
        1, "读取失败：é\0\n");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_read_sequence_empty_decode_error_stops_before_later_read()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof { registry_sequence_fixture_texts(); reveal_with_fuel(registry_sequence_expected, 4); }
    expect_registry_sequence_fixture_error(vec!["empty".to_owned(), "read-error".to_owned()],
        1, "空输入\0\n");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_read_sequence_decode_error_wins_over_later_read_error()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof { registry_sequence_fixture_texts(); reveal_with_fuel(registry_sequence_expected, 4); }
    expect_registry_sequence_fixture_error(vec!["decode-error".to_owned(), "read-error".to_owned()],
        1, "解码失败：é\0\n");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_read_sequence_read_error_after_success_discards_partial_maps()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof { registry_sequence_fixture_texts(); reveal_with_fuel(registry_sequence_expected, 4); }
    expect_registry_sequence_fixture_error(vec!["first".to_owned(), "read-error".to_owned(), "later".to_owned()],
        2, "读取失败：é\0\n");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_read_sequence_decode_error_after_success_discards_partial_maps()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof { registry_sequence_fixture_texts(); reveal_with_fuel(registry_sequence_expected, 4); }
    expect_registry_sequence_fixture_error(vec!["first".to_owned(), "decode-error".to_owned(), "later".to_owned()],
        2, "解码失败：é\0\n");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_read_sequence_preserves_original_bytes_and_unrelated_entries()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof { registry_sequence_fixture_texts(); reveal_with_fuel(registry_sequence_expected, 4); }
    let old = "旧/é".to_owned();
    let key = "../目录/é\0\n".to_owned();
    let mut bytes = std::collections::BTreeMap::new();
    let mut values = std::collections::BTreeMap::new();
    bytes.insert(old.clone(), vec![9u8]);
    values.insert(old.clone(), u64::MAX);
    let paths = vec![key.clone()];
    let ghost initial = (paths@, bytes@, values@);
    let out = policy::registry_read_sequence(paths, 0u8, bytes, values,
        registry_sequence_read, decode_registry_fixture);
    proof { registry_sequence_fixture_result(initial.0, initial.1, initial.2, out); }
    expect_status_true(out.0 == 1);
    match out.1 {
        Ok((bytes, values)) => {
            proof {
                registry_sequence_bytes_facts(initial.1);
                registry_sequence_bytes_facts(bytes@);
                assert(registry_sequence_bytes(bytes@)[old] == seq![9u8]);
                assert(bytes@[old]@ =~= seq![9u8]);
            }
            expect_status_true(bytes.len() == 2 && values.len() == 2);
            expect_registry_pair(&bytes, &values, &old, &[9u8], u64::MAX);
            let original = [1u8, 0, 255];
            proof {
                assert(registry_sequence_bytes(bytes@)[key] == seq![1u8, 0, 255]);
                assert(bytes@[key]@ =~= original@);
            }
            expect_registry_pair(&bytes, &values, &key, &original, 1);
        },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_read_sequence_duplicate_keys_replace_both_maps_in_order()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    proof { registry_sequence_fixture_texts(); reveal_with_fuel(registry_sequence_expected, 4); }
    let key = "same".to_owned();
    let paths = vec![key.clone(), key.clone()];
    let ghost requested = paths@;
    let out = policy::registry_read_sequence(paths, 0u8,
        std::collections::BTreeMap::new(), std::collections::BTreeMap::new(),
        registry_sequence_read, decode_registry_fixture);
    proof {
        assert(registry_sequence_bytes(Map::<String, Vec<u8>>::empty())
            =~= Map::<String, Seq<u8>>::empty());
        registry_sequence_fixture_result(requested, Map::empty(), Map::empty(), out);
    }
    expect_status_true(out.0 == 2);
    match out.1 {
        Ok((bytes, values)) => {
            proof { registry_sequence_bytes_facts(bytes@); }
            expect_status_true(bytes.len() == 1 && values.len() == 1);
            let original = [2u8, 255, 0];
            proof {
                assert(registry_sequence_bytes(bytes@)[key] == seq![2u8, 255, 0]);
                assert(bytes@[key]@ =~= original@);
            }
            expect_registry_pair(&bytes, &values, &key, &original, 2);
        },
        Err(_) => { panic!(); },
    }
}

fn expect_single_registry_blob(path: &str, bytes: Vec<u8>, expected: Vec<u8>)
    requires bytes@ == expected@,
{
    let rows = vec![(path.to_owned(), bytes)];
    match policy::single_registry_blob(path, rows) {
        Ok(actual) => {
            expect_status_true(scope_text::equal_bytes(actual.as_slice(), expected.as_slice()));
        },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_single_acquisition_keeps_full_path_and_original_bytes() {
    expect_single_registry_blob("", Vec::new(), Vec::new());
    expect_single_registry_blob("目录/é\0\n", vec![255u8, 0, 10, 13, 254], vec![255u8, 0, 10, 13, 254]);
    expect_single_registry_blob("../entry", vec![0u8], vec![0u8]);
}

fn expect_registry_blob_rejected(expected: &str, rows: Vec<(String, Vec<u8>)>)
    requires rows.len() != 1 || rows@[0].0@ != expected@,
{
    match policy::single_registry_blob(expected, rows) {
        Ok(_) => { panic!(); },
        Err(message) => { expect_status_text(message.as_str(), "registry acquisition path mismatch"); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_single_acquisition_rejects_missing_extra_and_wrong_paths() {
    proof {
        reveal_strlit("a"); reveal_strlit("a/"); reveal_strlit("A"); reveal_strlit("a\0");
        assert("a"@.len() != "a/"@.len() && "a"@.len() != "a\0"@.len());
        assert("a"@[0] != "A"@[0]);
    }
    expect_registry_blob_rejected("a", Vec::new());
    expect_registry_blob_rejected("a", vec![("a".to_owned(), vec![1u8]), ("a".to_owned(), vec![2u8])]);
    expect_registry_blob_rejected("a", vec![("a/".to_owned(), vec![1u8])]);
    expect_registry_blob_rejected("a", vec![("A".to_owned(), vec![1u8])]);
    expect_registry_blob_rejected("a", vec![("a\0".to_owned(), vec![1u8])]);
}

fn expect_registry_count(expected: usize, observed: usize, accepted: bool)
    requires accepted == (expected == observed),
{
    match policy::registry_acquisition_count(expected, observed) {
        Ok(()) => { expect_status_true(accepted); },
        Err(message) => {
            expect_status_true(!accepted);
            expect_status_text(message.as_str(), "registry acquisition count mismatch");
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_batch_count_accepts_empty_exact_and_integer_boundary_only() {
    expect_registry_count(0, 0, true);
    expect_registry_count(1, 1, true);
    expect_registry_count(5, 5, true);
    expect_registry_count(usize::MAX, usize::MAX, true);
    expect_registry_count(0, 1, false);
    expect_registry_count(1, 0, false);
    expect_registry_count(5, 4, false);
    expect_registry_count(5, 6, false);
    expect_registry_count(usize::MAX, usize::MAX - 1, false);
}

fn expect_registry_same_path(path: &str) {
    match policy::registry_acquisition_path(path, path) {
        Ok(()) => {},
        Err(_) => { panic!(); },
    }
}

fn expect_registry_wrong_path(expected: &str, observed: &str)
    requires expected@ != observed@,
{
    match policy::registry_acquisition_path(expected, observed) {
        Ok(()) => { panic!(); },
        Err(message) => { expect_status_text(message.as_str(), "registry acquisition path mismatch"); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_batch_path_keeps_full_text_without_normalization_or_path_admission() {
    proof {
        reveal_strlit("a"); reveal_strlit("a/"); reveal_strlit("A");
        reveal_strlit("a//b"); reveal_strlit("a/b"); reveal_strlit("a\0");
        reveal_strlit("é"); reveal_strlit("e\u{301}");
        assert("a"@.len() != "a/"@.len() && "a"@.len() != "a\0"@.len());
        assert("a"@[0] != "A"@[0]);
        assert("a//b"@.len() != "a/b"@.len());
        assert("é"@.len() != "e\u{301}"@.len());
    }
    expect_registry_same_path("");
    expect_registry_same_path("目录/é\0\n");
    expect_registry_same_path("../entry");
    expect_registry_wrong_path("a", "a/");
    expect_registry_wrong_path("a", "A");
    expect_registry_wrong_path("a//b", "a/b");
    expect_registry_wrong_path("a", "a\0");
    expect_registry_wrong_path("é", "e\u{301}");
}

fn expect_pending_paths(paths: Vec<String>, entry: &str, effective: &str, expected: Vec<String>)
    requires
        policy::registry_pending_prefix(paths@.map_values(|p: String| p@), paths@.len() as int,
            entry@, effective@) == expected@.map_values(|p: String| p@),
{
    let actual = policy::pending_registry_paths(paths, entry, effective);
    let ghost actual_text = actual@.map_values(|p: String| p@);
    let ghost expected_text = expected@.map_values(|p: String| p@);
    assert(actual_text.len() == actual.len() && expected_text.len() == expected.len());
    expect_status_true(actual.len() == expected.len());
    let mut i = 0usize;
    while i < expected.len()
        invariant
            i <= expected.len(), actual.len() == expected.len(),
            actual_text == expected_text,
            actual_text == actual@.map_values(|p: String| p@),
            expected_text == expected@.map_values(|p: String| p@),
        decreases expected.len() - i,
    {
        assert(actual_text[i as int] == actual@[i as int]@);
        assert(expected_text[i as int] == expected@[i as int]@);
        expect_status_text(actual[i].as_str(), expected[i].as_str());
        i += 1;
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_pending_keeps_first_unread_path_in_request_order() {
    let paths = vec!["entry".to_owned(), "z".to_owned(), "effective".to_owned(),
        "a".to_owned(), "z".to_owned()];
    let expected = vec!["z".to_owned(), "a".to_owned()];
    assert(policy::registry_pending_prefix(paths@.map_values(|p: String| p@), paths@.len() as int,
        "entry"@, "effective"@) == expected@.map_values(|p: String| p@)) by {
        reveal_with_fuel(policy::registry_pending_prefix, 6);
        reveal_strlit("entry"); reveal_strlit("effective"); reveal_strlit("z"); reveal_strlit("a");
        let input = paths@.map_values(|p: String| p@);
        assert(input.len() == 5);
        assert(input =~= seq!["entry"@, "z"@, "effective"@, "a"@, "z"@]);
        let a = policy::registry_pending_prefix(input, 1, "entry"@, "effective"@);
        let b = policy::registry_pending_prefix(input, 2, "entry"@, "effective"@);
        let c = policy::registry_pending_prefix(input, 3, "entry"@, "effective"@);
        let d = policy::registry_pending_prefix(input, 4, "entry"@, "effective"@);
        assert("z"@.len() == 1 && "entry"@.len() == 5 && "effective"@.len() == 9);
        assert("a"@.len() == 1 && "a"@[0] != "z"@[0]);
        assert(a =~= seq![]);
        assert(a.len() == 0 && !a.contains("z"@));
        assert(b =~= seq!["z"@]);
        assert(c =~= seq!["z"@]);
        assert(c.len() == 1 && c[0] == "z"@);
        assert(!c.contains("a"@));
        assert(d =~= seq!["z"@, "a"@]);
        assert(d.len() == 2 && d[0] == "z"@);
        assert(d.contains("z"@));
        assert(expected@.map_values(|p: String| p@) =~= d);
    }
    expect_pending_paths(paths, "entry", "effective", expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_pending_handles_empty_input_shared_entry_and_exact_path_text() {
    let paths = vec!["".to_owned(), "目录/é\0".to_owned(), "目录/é".to_owned()];
    let expected = vec!["目录/é\0".to_owned(), "目录/é".to_owned()];
    assert(policy::registry_pending_prefix(paths@.map_values(|p: String| p@), paths@.len() as int,
        ""@, ""@) == expected@.map_values(|p: String| p@)) by {
        reveal_with_fuel(policy::registry_pending_prefix, 4);
        reveal_strlit(""); reveal_strlit("目录/é\0"); reveal_strlit("目录/é");
        let input = paths@.map_values(|p: String| p@);
        assert(input.len() == 3);
        assert(input =~= seq![""@, "目录/é\0"@, "目录/é"@]);
        let a = policy::registry_pending_prefix(input, 1, ""@, ""@);
        let b = policy::registry_pending_prefix(input, 2, ""@, ""@);
        assert(""@.len() == 0 && "目录/é\0"@.len() == 5 && "目录/é"@.len() == 4);
        assert(a =~= seq![]);
        assert(a.len() == 0 && !a.contains("目录/é\0"@));
        assert(b =~= seq!["目录/é\0"@]);
        assert(b.len() == 1 && b[0] == "目录/é\0"@);
        assert(!b.contains("目录/é"@));
        assert(expected@.map_values(|p: String| p@) =~= seq!["目录/é\0"@, "目录/é"@]);
        assert(policy::registry_pending_prefix(input, 3, ""@, ""@) =~= seq!["目录/é\0"@, "目录/é"@]);
    }
    let empty: Vec<String> = Vec::new();
    assert(empty@.map_values(|p: String| p@) =~= seq![]);
    expect_pending_paths(empty, "entry", "effective", Vec::new());
    expect_pending_paths(paths, "", "", expected);
}

// 独立边界预期：未选中的 IO 替身不可调用，普通执行时调用即失败。

fn entry_match_no_locate() -> Result<Option<u8>, String> requires false, { panic!() }
fn entry_match_no_metadata(_path: &u8) -> Result<policy::CheckoutEntryMetadata, (bool, u8)> requires false, { panic!() }
fn entry_match_no_format(_error: u8) -> String requires false, { panic!() }
fn entry_match_no_link(_path: u8) -> Result<u32, String> requires false, { panic!() }
fn entry_match_no_link_hash(_target: u32) -> Result<String, String> requires false, { panic!() }
fn entry_match_no_open(_path: u8) -> Result<u16, String> requires false, { panic!() }
fn entry_match_no_opened(_file: &u16) -> Result<policy::CheckoutEntryMetadata, String> requires false, { panic!() }
fn entry_match_no_file_hash(_size: u64, _file: u16) -> Result<String, String> requires false, { panic!() }
fn entry_match_no_changed() -> String requires false, { panic!() }

fn entry_match_locate_none() -> (out: Result<Option<u8>, String>) ensures out == Ok::<Option<u8>, String>(None),
{ Ok(None) }

fn entry_match_locate_error() -> (out: Result<Option<u8>, String>)
    ensures out matches Err(error) && error@ == "path: é\0original\nend"@,
{ Err("path: é\0original\nend".to_owned()) }

fn entry_match_locate_seven() -> (out: Result<Option<u8>, String>) ensures out == Ok::<Option<u8>, String>(Some(7)),
{ Ok(Some(7)) }

fn entry_match_metadata_missing(path: &u8) -> (out: Result<policy::CheckoutEntryMetadata, (bool, u8)>)
    requires *path == 7,
    ensures out == Err::<policy::CheckoutEntryMetadata, (bool, u8)>((true, 42)),
{ Err((true, 42)) }

fn entry_match_metadata_error(path: &u8) -> (out: Result<policy::CheckoutEntryMetadata, (bool, u8)>)
    requires *path == 7,
    ensures out == Err::<policy::CheckoutEntryMetadata, (bool, u8)>((false, 42)),
{ Err((false, 42)) }

fn entry_match_format_metadata(error: u8) -> (out: String)
    requires error == 42,
    ensures out@ == "checkout file é\0name: original\nend"@,
{ "checkout file é\0name: original\nend".to_owned() }

fn entry_match_metadata_regular(path: &u8) -> (out: Result<policy::CheckoutEntryMetadata, (bool, u8)>)
    requires *path == 7,
    ensures out matches Ok(fields) && !fields.symbolic && fields.regular
        && fields.size == u64::MAX && fields.permissions == 0o644,
{ Ok(policy::CheckoutEntryMetadata { symbolic: false, regular: true, size: u64::MAX, permissions: 0o644 }) }

fn entry_match_open_regular(path: u8) -> (out: Result<u16, String>)
    requires path == 7,
    ensures out == Ok::<u16, String>(513),
{ Ok(513) }

fn entry_match_opened_regular(file: &u16) -> (out: Result<policy::CheckoutEntryMetadata, String>)
    requires *file == 513,
    ensures out matches Ok(fields) && !fields.symbolic && fields.regular
        && fields.size == u64::MAX && fields.permissions == 0o644,
{ Ok(policy::CheckoutEntryMetadata { symbolic: false, regular: true, size: u64::MAX, permissions: 0o644 }) }

fn entry_match_hash_regular(size: u64, file: u16) -> (out: Result<String, String>)
    requires size == u64::MAX, file == 513,
    ensures out matches Ok(text) && text@ == "digest: é\0full\nend"@,
{ Ok("digest: é\0full\nend".to_owned()) }

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_unsupported_entry_performs_no_observation() {
    proof {
        reveal_strlit("tree"); reveal_strlit("blob");
        assert("tree"@[0] == 't'); assert("blob"@[0] == 'b');
        assert("tree"@ != "blob"@);
        reveal(policy::checkout_entry_matches_callable);
    }
    let out = policy::checkout_entry_matches("tree", "100644", "digest", policy::CheckoutEntryIo {
        locate: entry_match_no_locate, metadata: entry_match_no_metadata, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_no_open,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    expect_checkout_parent(out, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_missing_physical_path_stops_before_metadata() {
    let out = policy::checkout_entry_matches("blob", "100644", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_none, metadata: entry_match_no_metadata, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_no_open,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    expect_checkout_parent(out, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_physical_error_preserves_complete_text_and_stops() {
    let out = policy::checkout_entry_matches("blob", "120000", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_error, metadata: entry_match_no_metadata, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_no_open,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    expect_checkout_observation_error(out, "path: é\0original\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_absent_metadata_does_not_format_or_continue() {
    let out = policy::checkout_entry_matches("blob", "100644", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_missing, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_no_open,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    expect_checkout_parent(out, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_metadata_error_formats_original_error_and_stops() {
    let out = policy::checkout_entry_matches("blob", "100644", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_error, format_metadata_error: entry_match_format_metadata,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_no_open,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    expect_checkout_observation_error(out, "checkout file é\0name: original\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_symbolic_mode_rejects_regular_metadata_before_read_link() {
    let out = policy::checkout_entry_matches("blob", "120000", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_regular, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_no_open,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    expect_checkout_parent(out, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_regular_mode_rejects_wrong_execute_bit_before_open() {
    proof { assert(0o644u32 & 0o100u32 == 0) by(bit_vector); }
    let out = policy::checkout_entry_matches("blob", "100755", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_regular, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_no_open,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    expect_checkout_parent(out, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_regular_match_preserves_path_file_size_and_complete_digest() {
    proof { entry_match_regular_mode_facts(); }
    let out = policy::checkout_entry_matches("blob", "100644", "digest: é\0full\nend", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_regular, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_open_regular,
        opened_metadata: entry_match_opened_regular, hash_file: entry_match_hash_regular, changed_error: entry_match_no_changed,
    });
    proof {
        reveal(policy::checkout_entry_matches_result); reveal(policy::checkout_entry_observed_result); reveal(policy::checkout_entry_opened_result); reveal(policy::checkout_digest_result);
    }
    expect_checkout_parent(out, true);
}

proof fn entry_match_regular_mode_facts()
    ensures "100644"@ != "120000"@, "100644"@ != "100755"@,
        0o644u32 & 0o100u32 == 0,
{
    reveal_strlit("100644"); reveal_strlit("100755"); reveal_strlit("120000");
    assert("100644"@[1] == '0' && "120000"@[1] == '2');
    assert("100644"@[3] == '6' && "100755"@[3] == '7');
    assert(0o644u32 & 0o100u32 == 0) by(bit_vector);
}

fn entry_match_metadata_symbolic(path: &u8) -> (out: Result<policy::CheckoutEntryMetadata, (bool, u8)>)
    requires *path == 7,
    ensures out matches Ok(fields) && fields.symbolic && !fields.regular
        && fields.size == 0 && fields.permissions == 0,
{ Ok(policy::CheckoutEntryMetadata { symbolic: true, regular: false, size: 0, permissions: 0 }) }

fn entry_match_read_target(path: u8) -> (out: Result<u32, String>)
    requires path == 7,
    ensures out == Ok::<u32, String>(u32::MAX),
{ Ok(u32::MAX) }

fn entry_match_read_error(path: u8) -> (out: Result<u32, String>)
    requires path == 7,
    ensures out matches Err(error) && error@ == "read link: é\0original\nend"@,
{ Err("read link: é\0original\nend".to_owned()) }

fn entry_match_hash_target(target: u32) -> (out: Result<String, String>)
    requires target == u32::MAX,
    ensures out matches Ok(text) && text@ == "digest: é\0full\nend"@,
{ Ok("digest: é\0full\nend".to_owned()) }

fn entry_match_hash_target_error(target: u32) -> (out: Result<String, String>)
    requires target == u32::MAX,
    ensures out matches Err(error) && error@ == "link hash: é\0original\nend"@,
{ Err("link hash: é\0original\nend".to_owned()) }

fn entry_match_open_error(path: u8) -> (out: Result<u16, String>)
    requires path == 7,
    ensures out matches Err(error) && error@ == "checkout file é\0name: open\nend"@,
{ Err("checkout file é\0name: open\nend".to_owned()) }

fn entry_match_opened_error(file: &u16) -> (out: Result<policy::CheckoutEntryMetadata, String>)
    requires *file == 513,
    ensures out matches Err(error) && error@ == "opened metadata: é\0original\nend"@,
{ Err("opened metadata: é\0original\nend".to_owned()) }

fn entry_match_changed_type(file: &u16) -> (out: Result<policy::CheckoutEntryMetadata, String>)
    requires *file == 513,
    ensures out matches Ok(fields) && !fields.regular
        && fields.size == u64::MAX && fields.permissions == 0o644,
{ Ok(policy::CheckoutEntryMetadata { symbolic: true, regular: false, size: u64::MAX, permissions: 0o644 }) }

fn entry_match_changed_size(file: &u16) -> (out: Result<policy::CheckoutEntryMetadata, String>)
    requires *file == 513,
    ensures out matches Ok(fields) && fields.regular
        && fields.size == 0 && fields.permissions == 0o644,
{ Ok(policy::CheckoutEntryMetadata { symbolic: false, regular: true, size: 0, permissions: 0o644 }) }

fn entry_match_changed_permissions(file: &u16) -> (out: Result<policy::CheckoutEntryMetadata, String>)
    requires *file == 513,
    ensures out matches Ok(fields) && fields.regular
        && fields.size == u64::MAX && fields.permissions == 0o600,
{ Ok(policy::CheckoutEntryMetadata { symbolic: false, regular: true, size: u64::MAX, permissions: 0o600 }) }

fn entry_match_changed_message() -> (out: String)
    ensures out@ == "checkout file identity changed while opening: é\0name\nend"@,
{ "checkout file identity changed while opening: é\0name\nend".to_owned() }

fn entry_match_hash_file_error(size: u64, file: u16) -> (out: Result<String, String>)
    requires size == u64::MAX, file == 513,
    ensures out matches Err(error) && error@ == "file hash: é\0original\nend"@,
{ Err("file hash: é\0original\nend".to_owned()) }

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_symbolic_success_preserves_target_and_complete_digest() {
    let out = policy::checkout_entry_matches("blob", "120000", "digest: é\0full\nend", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_symbolic, format_metadata_error: entry_match_no_format,
        read_link: entry_match_read_target, hash_link: entry_match_hash_target, open: entry_match_no_open,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    proof { reveal(policy::checkout_entry_matches_result); reveal(policy::checkout_entry_observed_result); reveal(policy::checkout_digest_result); }
    expect_checkout_parent(out, true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_symbolic_read_error_stops_before_hash_and_open() {
    let out = policy::checkout_entry_matches("blob", "120000", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_symbolic, format_metadata_error: entry_match_no_format,
        read_link: entry_match_read_error, hash_link: entry_match_no_link_hash, open: entry_match_no_open,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    expect_checkout_observation_error(out, "read link: é\0original\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_symbolic_hash_error_preserves_complete_text() {
    let out = policy::checkout_entry_matches("blob", "120000", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_symbolic, format_metadata_error: entry_match_no_format,
        read_link: entry_match_read_target, hash_link: entry_match_hash_target_error, open: entry_match_no_open,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    proof { reveal(policy::checkout_entry_matches_result); reveal(policy::checkout_entry_observed_result); reveal(policy::checkout_digest_result); }
    expect_checkout_observation_error(out, "link hash: é\0original\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_regular_open_error_stops_before_opened_metadata() {
    proof { entry_match_regular_mode_facts(); }
    let out = policy::checkout_entry_matches("blob", "100644", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_regular, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_open_error,
        opened_metadata: entry_match_no_opened, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    proof { reveal(policy::checkout_entry_matches_result); reveal(policy::checkout_entry_observed_result); reveal(policy::checkout_entry_opened_result); }
    expect_checkout_observation_error(out, "checkout file é\0name: open\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_regular_opened_metadata_error_stops_before_hash() {
    proof { entry_match_regular_mode_facts(); }
    let out = policy::checkout_entry_matches("blob", "100644", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_regular, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_open_regular,
        opened_metadata: entry_match_opened_error, hash_file: entry_match_no_file_hash, changed_error: entry_match_no_changed,
    });
    proof { reveal(policy::checkout_entry_matches_result); reveal(policy::checkout_entry_observed_result); reveal(policy::checkout_entry_opened_result); }
    expect_checkout_observation_error(out, "opened metadata: é\0original\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_regular_changed_type_stops_before_hash() {
    proof { entry_match_regular_mode_facts(); }
    let out = policy::checkout_entry_matches("blob", "100644", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_regular, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_open_regular,
        opened_metadata: entry_match_changed_type, hash_file: entry_match_no_file_hash, changed_error: entry_match_changed_message,
    });
    proof { reveal(policy::checkout_entry_matches_result); reveal(policy::checkout_entry_observed_result); reveal(policy::checkout_entry_opened_result); }
    expect_checkout_observation_error(out, "checkout file identity changed while opening: é\0name\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_regular_changed_size_stops_before_hash() {
    proof { entry_match_regular_mode_facts(); }
    let out = policy::checkout_entry_matches("blob", "100644", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_regular, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_open_regular,
        opened_metadata: entry_match_changed_size, hash_file: entry_match_no_file_hash, changed_error: entry_match_changed_message,
    });
    proof { reveal(policy::checkout_entry_matches_result); reveal(policy::checkout_entry_observed_result); reveal(policy::checkout_entry_opened_result); }
    expect_checkout_observation_error(out, "checkout file identity changed while opening: é\0name\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_regular_changed_permissions_stops_before_hash() {
    proof { entry_match_regular_mode_facts(); }
    let out = policy::checkout_entry_matches("blob", "100644", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_regular, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_open_regular,
        opened_metadata: entry_match_changed_permissions, hash_file: entry_match_no_file_hash, changed_error: entry_match_changed_message,
    });
    proof { reveal(policy::checkout_entry_matches_result); reveal(policy::checkout_entry_observed_result); reveal(policy::checkout_entry_opened_result); }
    expect_checkout_observation_error(out, "checkout file identity changed while opening: é\0name\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_regular_hash_error_preserves_complete_text() {
    proof { entry_match_regular_mode_facts(); }
    let out = policy::checkout_entry_matches("blob", "100644", "digest", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_regular, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_open_regular,
        opened_metadata: entry_match_opened_regular, hash_file: entry_match_hash_file_error, changed_error: entry_match_no_changed,
    });
    proof { reveal(policy::checkout_entry_matches_result); reveal(policy::checkout_entry_observed_result); reveal(policy::checkout_entry_opened_result); reveal(policy::checkout_digest_result); }
    expect_checkout_observation_error(out, "file hash: é\0original\nend");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn entry_match_digest_prefix_never_matches_full_oid() {
    proof {
        entry_match_regular_mode_facts();
        reveal_strlit("digest: é\0full\nend"); reveal_strlit("digest: é\0full\nend-suffix");
        assert("digest: é\0full\nend"@.len() < "digest: é\0full\nend-suffix"@.len());
        assert("digest: é\0full\nend"@ != "digest: é\0full\nend-suffix"@);
    }
    let out = policy::checkout_entry_matches("blob", "100644", "digest: é\0full\nend-suffix", policy::CheckoutEntryIo {
        locate: entry_match_locate_seven, metadata: entry_match_metadata_regular, format_metadata_error: entry_match_no_format,
        read_link: entry_match_no_link, hash_link: entry_match_no_link_hash, open: entry_match_open_regular,
        opened_metadata: entry_match_opened_regular, hash_file: entry_match_hash_regular, changed_error: entry_match_no_changed,
    });
    proof { reveal(policy::checkout_entry_matches_result); reveal(policy::checkout_entry_observed_result); reveal(policy::checkout_entry_opened_result); reveal(policy::checkout_digest_result); }
    expect_checkout_parent(out, false);
}



#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_snapshot_initial_same_path_keeps_entry_document()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let entry = ".chrono-harness/入口é\0\n.json".to_owned();
    let same_text = ".chrono-harness/入口é\0\n.json".to_owned();
    let snapshot = policy::registry_snapshot_initial(
        entry.clone(), vec![0u8, 255, 10], 11u64,
        same_text, vec![99u8], 22u64, None,
    );
    expect_status_text(snapshot.entry_path.as_str(), entry.as_str());
    expect_status_text(snapshot.effective_path.as_str(), entry.as_str());
    expect_status_true(snapshot.bytes.len() == 1 && snapshot.values.len() == 1);
    expect_registry_pair(&snapshot.bytes, &snapshot.values, &entry, &[0u8, 255, 10], 11u64);
    expect_status_true(snapshot.selection.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_snapshot_initial_distinct_paths_keep_both_complete_documents()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let entry = ".chrono-harness/入口\0\n".to_owned();
    let effective = ".chrono-harness/目标é\0\n".to_owned();
    proof {
        reveal_strlit(".chrono-harness/入口\0\n");
        reveal_strlit(".chrono-harness/目标é\0\n");
        assert(entry@.len() != effective@.len());
        assert(entry != effective);
    }
    let snapshot = policy::registry_snapshot_initial(
        entry.clone(), vec![0u8, 255, 10], u64::MAX,
        effective.clone(), vec![17u8, 0, 254], 0u64, Some(71u64),
    );
    expect_status_text(snapshot.entry_path.as_str(), entry.as_str());
    expect_status_text(snapshot.effective_path.as_str(), effective.as_str());
    expect_status_true(snapshot.bytes.len() == 2 && snapshot.values.len() == 2);
    expect_registry_pair(&snapshot.bytes, &snapshot.values, &entry, &[0u8, 255, 10], u64::MAX);
    expect_registry_pair(&snapshot.bytes, &snapshot.values, &effective, &[17u8, 0, 254], 0u64);
    match snapshot.selection {
        Some(value) => { expect_status_true(value == 71u64); },
        None => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_snapshot_initial_empty_identity_preserves_entry_and_selection()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let entry = "".to_owned();
    let original = Vec::new();
    let expected: &[u8] = &[];
    proof { assert(original@ =~= expected@); }
    let snapshot = policy::registry_snapshot_initial(
        entry.clone(), original, 0u64,
        "".to_owned(), vec![255u8], u64::MAX, Some(u64::MAX),
    );
    expect_status_text(snapshot.entry_path.as_str(), "");
    expect_status_text(snapshot.effective_path.as_str(), "");
    expect_status_true(snapshot.bytes.len() == 1 && snapshot.values.len() == 1);
    expect_registry_pair(&snapshot.bytes, &snapshot.values, &entry, expected, 0u64);
    match snapshot.selection {
        Some(value) => { expect_status_true(value == u64::MAX); },
        None => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_snapshot_initial_paths_remain_literal_and_case_sensitive()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let entry = "../A/é\0\n".to_owned();
    let effective = "../a/é\0\n".to_owned();
    let original = Vec::new();
    let effective_bytes = Vec::new();
    let expected: &[u8] = &[];
    proof {
        reveal_strlit("../A/é\0\n");
        reveal_strlit("../a/é\0\n");
        assert(entry@[3] == 'A' && effective@[3] == 'a');
        assert(entry != effective);
        assert(original@ =~= expected@);
        assert(effective_bytes@ =~= expected@);
    }
    let snapshot = policy::registry_snapshot_initial(
        entry.clone(), original, 13u64,
        effective.clone(), effective_bytes, 17u64, None,
    );
    expect_status_text(snapshot.entry_path.as_str(), "../A/é\0\n");
    expect_status_text(snapshot.effective_path.as_str(), "../a/é\0\n");
    expect_status_true(snapshot.bytes.len() == 2 && snapshot.values.len() == 2);
    expect_registry_pair(&snapshot.bytes, &snapshot.values, &entry, expected, 13u64);
    expect_registry_pair(&snapshot.bytes, &snapshot.values, &effective, expected, 17u64);
    expect_status_true(snapshot.selection.is_none());
}

}
