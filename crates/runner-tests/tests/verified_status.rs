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
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

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

fn finding_fields_preserve_nonempty_full_text(code: &str, message: &str, reference: &str)
    requires code@.len() > 0, message@.len() > 0,
{
    let refs = vec![reference.to_owned(), reference.to_owned()];
    let info = policy::Finding { code: code.to_owned(), message: message.to_owned(),
        level: "info".to_owned(), delta_refs: refs, causes: vec!["".to_owned()] };
    let accepted = policy::valid_finding_fields(&info);
    assert(accepted);
    let warning = policy::Finding { level: "warning".to_owned(), ..info.clone() };
    let accepted = policy::valid_finding_fields(&warning);
    assert(accepted);
    let error = policy::Finding { level: "error".to_owned(), ..info.clone() };
    let accepted = policy::valid_finding_fields(&error);
    assert(accepted);
    assert(info.delta_refs.len() == 2 && info.delta_refs@[0]@ == reference@
        && info.delta_refs@[1]@ == reference@);
}

fn empty_finding_fields_are_rejected(code: &str, message: &str, level: &str) {
    proof { reveal_strlit(""); }
    let refs = vec!["/records/x".to_owned()];
    let no_code = policy::Finding { code: "".to_owned(), message: message.to_owned(),
        level: level.to_owned(), delta_refs: refs.clone(), causes: Vec::new() };
    let accepted = policy::valid_finding_fields(&no_code);
    assert(!accepted);
    let no_message = policy::Finding { code: code.to_owned(), message: "".to_owned(),
        level: level.to_owned(), delta_refs: refs, causes: Vec::new() };
    let accepted = policy::valid_finding_fields(&no_message);
    assert(!accepted);
    let no_refs = policy::Finding { code: code.to_owned(), message: message.to_owned(),
        level: level.to_owned(), delta_refs: Vec::new(), causes: Vec::new() };
    let accepted = policy::valid_finding_fields(&no_refs);
    assert(!accepted);
}

fn noncanonical_finding_level_is_rejected(level: &str)
    requires level@ != "info"@, level@ != "warning"@, level@ != "error"@,
{
    let finding = policy::Finding { code: "E".to_owned(), message: "m".to_owned(),
        level: level.to_owned(), delta_refs: vec!["/x".to_owned()], causes: Vec::new() };
    let accepted = policy::valid_finding_fields(&finding);
    assert(!accepted);
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
    let empty = policy::valid_evidence_identity("", "report", zero);
    let short = policy::valid_evidence_identity(".chrono-harness/state", "report", zero);
    let similar = policy::valid_evidence_identity(".chrono-harness/states/x", "report", zero);
    let absolute = policy::valid_evidence_identity("/.chrono-harness/state/x", "report", zero);
    let case = policy::valid_evidence_identity(".chrono-harness/State/x", "report", zero);
    assert(!empty); assert(!short); assert(!similar); assert(!absolute); assert(!case);
    let no_kind = policy::valid_evidence_identity(".chrono-harness/state/x", "", zero);
    assert(!no_kind);
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
    assert(!no_digest); assert(!short_digest); assert(!long_digest); assert(!upper_digest);
}

}
