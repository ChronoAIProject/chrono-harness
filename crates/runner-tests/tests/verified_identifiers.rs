//! 实际身份实现的边界证明。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_identifiers.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

// 同一预期也在普通 Rust 中执行；Verus 的 assert 只检查证明。
fn expect_identifier_true(actual: bool)
    requires actual,
{
    if !actual {
        panic!();
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn request_header_profiles_and_empty_judge() {
    proof {
        reveal_strlit("chrono-judge/v1");
        reveal_strlit("chrono-initial-judge/v1");
        reveal_strlit("evaluate");
        reveal_strlit("inventory");
        reveal_strlit("judge");
        reveal_strlit("");
        assert("chrono-judge/v1"@.len() == 15);
        assert("chrono-initial-judge/v1"@.len() == 23);
        assert("evaluate"@.len() == 8 && "inventory"@.len() == 9);
        assert("judge"@.len() == 5 && ""@.len() == 0);
    }
    let evaluation = policy::valid_request_header(
        "chrono-judge/v1", "evaluate", "judge", "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(evaluation);
    let inventory = policy::valid_request_header(
        "chrono-initial-judge/v1", "inventory", "judge",
        "chrono-initial-judge/v1", "inventory",
    );
    expect_identifier_true(inventory);
    let empty = policy::valid_request_header(
        "chrono-judge/v1", "evaluate", "", "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(!empty);
    let empty_protocol = policy::valid_request_header(
        "", "evaluate", "judge", "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(!empty_protocol);
    let empty_mode = policy::valid_request_header(
        "chrono-judge/v1", "", "judge", "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(!empty_mode);
    let wrong_protocol = policy::valid_request_header(
        "chrono-initial-judge/v1", "evaluate", "judge", "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(!wrong_protocol);
    let wrong_mode = policy::valid_request_header(
        "chrono-initial-judge/v1", "evaluate", "judge",
        "chrono-initial-judge/v1", "inventory",
    );
    expect_identifier_true(!wrong_mode);
    let wrong_inventory = policy::valid_request_header(
        "chrono-judge/v1", "inventory", "judge", "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(!wrong_inventory);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn request_header_preserves_full_text_and_nonempty_name_contract() {
    proof {
        reveal_strlit("chrono-judge/v1");
        reveal_strlit("evaluate");
        reveal_strlit("判官/\0\n");
        reveal_strlit("chrono-judge/v1\0");
        reveal_strlit("evaluate\n");
        reveal_strlit("Evaluate");
        reveal_strlit("chrono-initial-judge/v10");
        reveal_strlit("chrono-initial-judge/v1");
        reveal_strlit("inventory");
        assert("chrono-judge/v1"@.len() == 15 && "chrono-judge/v1\0"@.len() == 16);
        assert("evaluate"@.len() == 8 && "evaluate\n"@.len() == 9);
        assert("Evaluate"@.len() == 8 && "Evaluate"@[0] != "evaluate"@[0]);
        assert("chrono-initial-judge/v1"@.len() == 23
            && "chrono-initial-judge/v10"@.len() == 24);
        assert("判官/\0\n"@.len() == 5);
    }
    let unicode_judge = policy::valid_request_header(
        "chrono-judge/v1", "evaluate", "判官/\0\n", "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(unicode_judge);
    let nul_protocol = policy::valid_request_header(
        "chrono-judge/v1\0", "evaluate", "judge", "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(!nul_protocol);
    let newline_mode = policy::valid_request_header(
        "chrono-judge/v1", "evaluate\n", "judge", "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(!newline_mode);
    let uppercase = policy::valid_request_header(
        "chrono-judge/v1", "Evaluate", "judge", "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(!uppercase);
    let suffix = policy::valid_request_header(
        "chrono-initial-judge/v10", "inventory", "judge",
        "chrono-initial-judge/v1", "inventory",
    );
    expect_identifier_true(!suffix);
}

// 任意非空名称都保留；头部校验不能偷偷加强为 ASCII 标识符策略。
fn request_header_accepts_any_nonempty_judge(judge: &str)
    requires judge@.len() > 0,
{
    let accepted = policy::valid_request_header(
        "chrono-judge/v1", "evaluate", judge, "chrono-judge/v1", "evaluate",
    );
    expect_identifier_true(accepted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn oid_boundaries() {
    let zero_bytes = [48u8; 40];
    let zero = policy::valid_oid(&zero_bytes);
    expect_identifier_true(!zero);
    let nonzero_bytes = [97u8; 40];
    let nonzero = policy::valid_oid(&nonzero_bytes);
    assert(nonzero_bytes@[0] == 97);
    expect_identifier_true(nonzero);
    let uppercase_bytes = [65u8; 40];
    let uppercase = policy::valid_oid(&uppercase_bytes);
    expect_identifier_true(!uppercase);
    let empty: [u8; 0] = [];
    let too_short = policy::valid_oid(&empty);
    expect_identifier_true(!too_short);
    let long_bytes = [102u8; 64];
    let long = policy::valid_oid(&long_bytes);
    assert(long_bytes@[0] == 102);
    expect_identifier_true(long);
}

fn expect_full_oid_success(text: &str)
    requires policy::oid(text.spec_bytes()),
{
    match policy::full_oid(text) {
        Ok(()) => {},
        Err(_) => { panic!(); },
    }
}

fn expect_full_oid_rejection(text: &str)
    requires !policy::oid(text.spec_bytes()),
{
    match policy::full_oid(text) {
        Ok(()) => { panic!(); },
        Err(message) => {
            let same = policy::equal_text(
                message.as_str(), "expected full nonzero lowercase commit OID",
            );
            expect_identifier_true(same);
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn full_oid_accepts_both_lengths_and_one_nonzero_digit() {
    proof {
        reveal_strlit("0123456789abcdef0123456789abcdef01234567");
        reveal_strlit("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        reveal_strlit("0000000000000000000000000000000000000001");
        assert(vstd::string::is_ascii("0123456789abcdef0123456789abcdef01234567"));
        assert(vstd::string::is_ascii("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"));
        assert(vstd::string::is_ascii("0000000000000000000000000000000000000001"));
        vstd::string::is_ascii_spec_bytes("0123456789abcdef0123456789abcdef01234567");
        vstd::string::is_ascii_spec_bytes("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        vstd::string::is_ascii_spec_bytes("0000000000000000000000000000000000000001");
        assert("0123456789abcdef0123456789abcdef01234567".spec_bytes()[1] == 49);
        assert("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".spec_bytes()[1] == 49);
        assert("0000000000000000000000000000000000000001".spec_bytes()[39] == 49);
    }
    expect_full_oid_success("0123456789abcdef0123456789abcdef01234567");
    expect_full_oid_success("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    expect_full_oid_success("0000000000000000000000000000000000000001");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn full_oid_rejects_zero_lengths_case_and_control_bytes_with_the_original_error() {
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
    expect_full_oid_rejection("");
    expect_full_oid_rejection("0000000000000000000000000000000000000000");
    expect_full_oid_rejection("0000000000000000000000000000000000000000000000000000000000000000");
    expect_full_oid_rejection("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    expect_full_oid_rejection("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    expect_full_oid_rejection("aaaaaaaaaaaaaaaaaaaAaaaaaaaaaaaaaaaaaaaa");
    expect_full_oid_rejection("aaaaaaaaaaaaaaaaaaa\0aaaaaaaaaaaaaaaaaaaa");
    expect_full_oid_rejection("aaaaaaaaaaaaaaaaaaa\naaaaaaaaaaaaaaaaaaaa");
}

// 任意位置的非法字节都必须拒绝，不能只检查首尾或截取一段文本。
fn full_oid_rejects_an_invalid_byte_anywhere(text: &str, at: usize)
    requires at < text.spec_bytes().len(), !policy::oid_byte(text.spec_bytes()[at as int]),
{
    expect_full_oid_rejection(text);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identifier_ascii_boundaries() {
    let bytes = [48u8, 57u8, 65u8, 90u8, 97u8, 122u8, 46u8, 95u8, 45u8];
    let accepted = policy::valid_id(&bytes);
    expect_identifier_true(accepted);
    let empty: [u8; 0] = [];
    let missing = policy::valid_id(&empty);
    expect_identifier_true(!missing);
    let slash = [47u8];
    let invalid_separator = policy::valid_id(&slash);
    expect_identifier_true(!invalid_separator);
    let non_ascii = [128u8];
    let invalid_unicode = policy::valid_id(&non_ascii);
    expect_identifier_true(!invalid_unicode);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn digest_lengths_and_allowed_bytes() {
    let empty: [u8; 0] = [];
    let missing = policy::valid_digest(&empty);
    expect_identifier_true(!missing);
    let oid_length = [97u8; 40];
    let only_oid = policy::valid_digest(&oid_length);
    expect_identifier_true(!only_oid);
    let short = [48u8; 63];
    let too_short = policy::valid_digest(&short);
    expect_identifier_true(!too_short);
    let long = [48u8; 65];
    let too_long = policy::valid_digest(&long);
    expect_identifier_true(!too_long);
    let zero = [48u8; 64];
    let zero_digest = policy::valid_digest(&zero);
    expect_identifier_true(zero_digest);
    let nine = [57u8; 64];
    let digit_end = policy::valid_digest(&nine);
    expect_identifier_true(digit_end);
    let a = [97u8; 64];
    let lower_start = policy::valid_digest(&a);
    expect_identifier_true(lower_start);
    let f = [102u8; 64];
    let lower_end = policy::valid_digest(&f);
    expect_identifier_true(lower_end);
}

fn reject_digest_byte_at(at: usize, byte: u8)
    requires at < 64,
        byte < 48 || (57 < byte && byte < 97) || 102 < byte,
{
    let mut bytes = [48u8; 64];
    bytes[at] = byte;
    let accepted = policy::valid_digest(&bytes);
    assert(bytes@[at as int] == byte);
    assert(!policy::oid_byte(bytes@[at as int]));
    expect_identifier_true(!accepted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn digest_rejects_bad_bytes_throughout_the_full_input() {
    reject_digest_byte_at(0, 47);
    reject_digest_byte_at(1, 58);
    reject_digest_byte_at(9, 64);
    reject_digest_byte_at(31, 65);
    reject_digest_byte_at(32, 96);
    reject_digest_byte_at(63, 103);
    reject_digest_byte_at(7, 0);
    reject_digest_byte_at(42, 10);
    reject_digest_byte_at(62, 128);
    reject_digest_byte_at(17, 195);
    reject_digest_byte_at(61, 255);
    let mut utf8 = [48u8; 64];
    utf8[0] = 195;
    utf8[1] = 169;
    let unicode = policy::valid_digest(&utf8);
    assert(utf8@[0] == 195);
    assert(!policy::oid_byte(utf8@[0]));
    expect_identifier_true(!unicode);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn digest_text_uses_the_complete_utf8_bytes() {
    let text = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    proof {
        reveal_strlit("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        assert(vstd::string::is_ascii(text));
        vstd::string::is_ascii_spec_bytes(text);
    }
    let accepted = policy::valid_digest_text(text);
    expect_identifier_true(accepted);
    proof {
        reveal_strlit("");
        vstd::string::is_ascii_spec_bytes("");
    }
    let empty = policy::valid_digest_text("");
    expect_identifier_true(!empty);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn merge_parent_identity_boundaries() {
    let base = [97u8; 40];
    let head = [98u8; 40];
    let wrong = [99u8; 40];
    let zero = [48u8; 40];
    assert(base@[0] == 97 && head@[0] == 98);
    let accepted = policy::valid_merge_parents(2, &base, &head, &head);
    expect_identifier_true(accepted);
    let missing_parent = policy::valid_merge_parents(1, &base, &head, &head);
    expect_identifier_true(!missing_parent);
    let extra_parent = policy::valid_merge_parents(3, &base, &head, &head);
    expect_identifier_true(!extra_parent);
    let unrelated_head = policy::valid_merge_parents(2, &base, &wrong, &head);
    expect_identifier_true(!unrelated_head);
    let invalid_base = policy::valid_merge_parents(2, &zero, &head, &head);
    expect_identifier_true(!invalid_base);
    let invalid_head = policy::valid_merge_parents(2, &base, &zero, &zero);
    expect_identifier_true(!invalid_head);
    let empty: [u8; 0] = [];
    let missing_head = policy::valid_merge_parents(2, &base, &empty, &head);
    expect_identifier_true(!missing_head);
    let base64 = [100u8; 64];
    let head64 = [101u8; 64];
    assert(base64@[0] == 100 && head64@[0] == 101);
    let accepted64 = policy::valid_merge_parents(2, &base64, &head64, &head64);
    expect_identifier_true(accepted64);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn commit_parent_selection_stops_at_the_first_empty_header_line() {
    let empty: [u8; 0] = [];
    proof {
        policy::header_line_end_is(empty@, 0);
        reveal_with_fuel(policy::commit_parent_lines, 1);
        assert(policy::commit_parent_lines(empty@).len() == 0);
    }
    let out = policy::commit_parent_records(&empty);
    expect_identifier_true(out.len() == 0);
    let raw = [10u8, 112, 97, 114, 101, 110, 116, 32, 97, 10];
    proof {
        policy::header_line_end_is(raw@, 0);
        reveal_with_fuel(policy::commit_parent_lines, 1);
        assert(policy::commit_parent_lines(raw@).len() == 0);
    }
    let out = policy::commit_parent_records(&raw);
    expect_identifier_true(out.len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn commit_parent_selection_keeps_complete_order_and_ignores_continuations_and_body() {
    let raw = [
        116u8, 114, 101, 101, 32, 120, 10, // tree x LF
        112, 97, 114, 101, 110, 116, 32, 97, 10, // parent a LF
        32, 112, 97, 114, 101, 110, 116, 32, 122, 10, // continuation
        112, 97, 114, 101, 110, 116, 32, 98, 10, // parent b LF
        10, 112, 97, 114, 101, 110, 116, 32, 99, 10, // body
    ];
    proof {
        policy::header_line_end_is(raw@, 6);
        policy::header_line_end_is(raw@.skip(7), 8);
        policy::header_line_end_is(raw@.skip(16), 9);
        policy::header_line_end_is(raw@.skip(26), 8);
        policy::header_line_end_is(raw@.skip(35), 0);
        vstd::seq_lib::lemma_seq_skip_of_skip(raw@, 7, 9);
        vstd::seq_lib::lemma_seq_skip_of_skip(raw@, 16, 10);
        vstd::seq_lib::lemma_seq_skip_of_skip(raw@, 26, 9);
        reveal_with_fuel(policy::commit_parent_lines, 1);
        assert(policy::commit_parent_lines(raw@.skip(35)) == Seq::<Seq<u8>>::empty());
        assert(raw@.skip(26).take(8).skip(7) =~= seq![98u8]);
        assert(policy::commit_parent_lines(raw@.skip(26)) == seq![seq![98u8]]);
        assert(policy::commit_parent_lines(raw@.skip(16)) == seq![seq![98u8]]);
        assert(raw@.skip(7).take(8).skip(7) =~= seq![97u8]);
        assert(policy::commit_parent_lines(raw@.skip(7)) == seq![seq![97u8], seq![98u8]]);
        assert(policy::commit_parent_lines(raw@).len() == 2);
        assert(policy::commit_parent_lines(raw@)[0] =~= seq![97u8]);
        assert(policy::commit_parent_lines(raw@)[1] =~= seq![98u8]);
    }
    let out = policy::commit_parent_records(&raw);
    expect_identifier_true(out.len() == 2);
    let first = out[0];
    let second = out[1];
    assert(first@ =~= seq![97u8]);
    assert(second@ =~= seq![98u8]);
    expect_identifier_true(first.len() == 1 && first[0] == 97);
    expect_identifier_true(second.len() == 1 && second[0] == 98);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn commit_parent_selection_preserves_empty_cr_nul_and_non_utf8_suffixes() {
    let raw = [112u8, 97, 114, 101, 110, 116, 32, 10,
        112, 97, 114, 101, 110, 116, 32, 97, 13, 10,
        112, 97, 114, 101, 110, 116, 32, 255, 0];
    proof {
        policy::header_line_end_is(raw@, 7);
        policy::header_line_end_is(raw@.skip(8), 9);
        policy::header_line_end_is(raw@.skip(18), 9);
        vstd::seq_lib::lemma_seq_skip_of_skip(raw@, 8, 10);
        reveal_with_fuel(policy::commit_parent_lines, 1);
        assert(raw@.skip(18).take(9).skip(7) =~= seq![255u8, 0u8]);
        assert(policy::commit_parent_lines(raw@.skip(18)) == seq![seq![255u8, 0u8]]);
        assert(raw@.skip(8).take(9).skip(7) =~= seq![97u8, 13u8]);
        assert(policy::commit_parent_lines(raw@.skip(8))
            == seq![seq![97u8, 13u8], seq![255u8, 0u8]]);
        assert(raw@.take(7).skip(7) =~= Seq::<u8>::empty());
        assert(policy::commit_parent_lines(raw@).len() == 3);
        assert(policy::commit_parent_lines(raw@)[0] =~= Seq::<u8>::empty());
        assert(policy::commit_parent_lines(raw@)[1] =~= seq![97u8, 13u8]);
        assert(policy::commit_parent_lines(raw@)[2] =~= seq![255u8, 0u8]);
    }
    let out = policy::commit_parent_records(&raw);
    expect_identifier_true(out.len() == 3);
    let first = out[0];
    let second = out[1];
    let third = out[2];
    assert(first@ =~= Seq::<u8>::empty());
    assert(second@ =~= seq![97u8, 13u8]);
    assert(third@ =~= seq![255u8, 0u8]);
    expect_identifier_true(first.len() == 0);
    expect_identifier_true(second.len() == 2 && second[0] == 97 && second[1] == 13);
    expect_identifier_true(third.len() == 2 && third[0] == 255 && third[1] == 0);
}

}
