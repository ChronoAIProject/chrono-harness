//! 实际身份实现的边界证明。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_identifiers as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
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

}
