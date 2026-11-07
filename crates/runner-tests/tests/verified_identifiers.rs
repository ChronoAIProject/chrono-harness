//! 实际身份实现的边界证明。
#![allow(unexpected_cfgs)]
#[path = "../../runner/src/verified_identifiers.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn request_header_profiles_and_empty_judge() {
    proof {
        reveal_strlit("chrono-judge/v1");
        reveal_strlit("chrono-initial-judge/v1");
        reveal_strlit("evaluate");
        reveal_strlit("inventory");
        reveal_strlit("judge");
        reveal_strlit("");
    }
    let evaluation = policy::valid_request_header(
        "chrono-judge/v1", "evaluate", "judge", "chrono-judge/v1", "evaluate",
    );
    assert(evaluation);
    let inventory = policy::valid_request_header(
        "chrono-initial-judge/v1", "inventory", "judge",
        "chrono-initial-judge/v1", "inventory",
    );
    assert(inventory);
    let empty = policy::valid_request_header(
        "chrono-judge/v1", "evaluate", "", "chrono-judge/v1", "evaluate",
    );
    assert(!empty);
    let wrong_protocol = policy::valid_request_header(
        "chrono-initial-judge/v1", "evaluate", "judge", "chrono-judge/v1", "evaluate",
    );
    assert(!wrong_protocol);
    let wrong_mode = policy::valid_request_header(
        "chrono-initial-judge/v1", "evaluate", "judge",
        "chrono-initial-judge/v1", "inventory",
    );
    assert(!wrong_mode);
    let wrong_inventory = policy::valid_request_header(
        "chrono-judge/v1", "inventory", "judge", "chrono-judge/v1", "evaluate",
    );
    assert(!wrong_inventory);
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
    }
    let unicode_judge = policy::valid_request_header(
        "chrono-judge/v1", "evaluate", "判官/\0\n", "chrono-judge/v1", "evaluate",
    );
    assert(unicode_judge);
    let nul_protocol = policy::valid_request_header(
        "chrono-judge/v1\0", "evaluate", "judge", "chrono-judge/v1", "evaluate",
    );
    assert(!nul_protocol);
    let newline_mode = policy::valid_request_header(
        "chrono-judge/v1", "evaluate\n", "judge", "chrono-judge/v1", "evaluate",
    );
    assert(!newline_mode);
    let uppercase = policy::valid_request_header(
        "chrono-judge/v1", "Evaluate", "judge", "chrono-judge/v1", "evaluate",
    );
    assert(!uppercase);
    let suffix = policy::valid_request_header(
        "chrono-initial-judge/v10", "inventory", "judge",
        "chrono-initial-judge/v1", "inventory",
    );
    assert(!suffix);
}

// 任意非空名称都保留；头部校验不能偷偷加强为 ASCII 标识符策略。
fn request_header_accepts_any_nonempty_judge(judge: &str)
    requires judge@.len() > 0,
{
    let accepted = policy::valid_request_header(
        "chrono-judge/v1", "evaluate", judge, "chrono-judge/v1", "evaluate",
    );
    assert(accepted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn oid_boundaries() {
    let zero_bytes = [48u8; 40];
    let zero = policy::valid_oid(&zero_bytes);
    assert(!zero);
    let nonzero_bytes = [97u8; 40];
    let nonzero = policy::valid_oid(&nonzero_bytes);
    assert(nonzero_bytes@[0] == 97);
    assert(nonzero);
    let uppercase_bytes = [65u8; 40];
    let uppercase = policy::valid_oid(&uppercase_bytes);
    assert(!uppercase);
    let empty: [u8; 0] = [];
    let too_short = policy::valid_oid(&empty);
    assert(!too_short);
    let long_bytes = [102u8; 64];
    let long = policy::valid_oid(&long_bytes);
    assert(long_bytes@[0] == 102);
    assert(long);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identifier_ascii_boundaries() {
    let bytes = [48u8, 57u8, 65u8, 90u8, 97u8, 122u8, 46u8, 95u8, 45u8];
    let accepted = policy::valid_id(&bytes);
    assert(accepted);
    let empty: [u8; 0] = [];
    let missing = policy::valid_id(&empty);
    assert(!missing);
    let slash = [47u8];
    let invalid_separator = policy::valid_id(&slash);
    assert(!invalid_separator);
    let non_ascii = [128u8];
    let invalid_unicode = policy::valid_id(&non_ascii);
    assert(!invalid_unicode);
}

}
