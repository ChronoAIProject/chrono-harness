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

fn expect_inventory_error(out: Result<(), String>, expected: &str)
    requires match out { Err(message) => message@ == expected@, _ => false },
{
    match out {
        Ok(()) => { panic!(); },
        Err(message) => {
            let same = policy::equal_text(message.as_str(), expected);
            expect_identifier_true(same);
        },
    }
}

fn expect_inventory_success(out: Result<(), String>)
    requires out.is_ok(),
{
    match out {
        Ok(()) => {},
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_inventory_termination_checks_only_the_final_byte() {
    let empty: [u8; 0] = [];
    expect_inventory_success(policy::git_inventory_termination(&empty));
    let empty_rows = [0u8, 0];
    expect_inventory_success(policy::git_inventory_termination(&empty_rows));
    let invalid_utf8_but_terminated = [255u8, 0];
    expect_inventory_success(policy::git_inventory_termination(&invalid_utf8_but_terminated));
    let no_nul = [97u8];
    expect_inventory_error(policy::git_inventory_termination(&no_nul), "incomplete Git inventory");
    let embedded_nul = [97u8, 0, 98];
    expect_inventory_error(policy::git_inventory_termination(&embedded_nul), "incomplete Git inventory");
}

// 对任意解码文本核完整借用；解码失败必须保持原诊断。
fn git_inventory_decoding_preserves_the_entire_text(text: &str) {
    match policy::git_inventory_text(Some(text)) {
        Ok(actual) => {
            let same = policy::equal_text(actual, text);
            expect_identifier_true(same);
        },
        Err(_) => { panic!(); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_inventory_decoding_boundaries() {
    git_inventory_decoding_preserves_the_entire_text("");
    git_inventory_decoding_preserves_the_entire_text("路径\t\0\r\n");
    match policy::git_inventory_text(None) {
        Ok(_) => { panic!(); },
        Err(message) => {
            let same = policy::equal_text(message.as_str(), "non UTF-8 Git inventory");
            expect_identifier_true(same);
        },
    }
}

// 独立预期直接列完整原字节；普通运行也比较每个字节。
fn expect_inventory_row(row: &str, meta: &str, path: &str)
    requires ({
        let end = policy::ascii_separator_end(row.spec_bytes(), 9);
        policy::first_ascii_separator(row.spec_bytes(), 9, end)
            && end < row.spec_bytes().len()
            && meta.spec_bytes() == row.spec_bytes().take(end)
            && path.spec_bytes() == row.spec_bytes().skip(end + 1)
    }),
{
    match policy::git_inventory_row(row, "invalid Git tree row") {
        Ok((actual_meta, actual_path)) => {
            let same_meta = policy::equal_bytes(actual_meta.as_bytes(), meta.as_bytes());
            let same_path = policy::equal_bytes(actual_path.as_bytes(), path.as_bytes());
            expect_identifier_true(same_meta && same_path);
        },
        Err(_) => { panic!(); },
    }
}

fn expect_missing_inventory_row(row: &str, expected: &str)
    requires forall|i: int| 0 <= i < row.spec_bytes().len() ==> row.spec_bytes()[i] != 9,
{
    proof { policy::ascii_separator_end_is(row.spec_bytes(), 9, row.spec_bytes().len() as int); }
    match policy::git_inventory_row(row, expected) {
        Ok(_) => { panic!(); },
        Err(message) => {
            let same = policy::equal_text(message.as_str(), expected);
            expect_identifier_true(same);
        },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_inventory_row_boundaries() {
    proof {
        reveal_strlit(""); reveal_strlit("\t"); reveal_strlit("m\tp\tq\r\n");
        reveal_strlit("m"); reveal_strlit("p\tq\r\n"); reveal_strlit("missing");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("\t")
            && vstd::string::is_ascii("m\tp\tq\r\n") && vstd::string::is_ascii("m")
            && vstd::string::is_ascii("p\tq\r\n") && vstd::string::is_ascii("missing"));
        policy::ascii_separator_end_is("\t".spec_bytes(), 9, 0);
        policy::ascii_separator_end_is("m\tp\tq\r\n".spec_bytes(), 9, 1);
        assert(policy::first_ascii_separator("\t".spec_bytes(), 9, 0));
        assert("\t".spec_bytes().take(0) =~= "".spec_bytes());
        assert("\t".spec_bytes().skip(1) =~= "".spec_bytes());
        assert(policy::first_ascii_separator("m\tp\tq\r\n".spec_bytes(), 9, 1));
        assert("m\tp\tq\r\n".spec_bytes().take(1) =~= "m".spec_bytes());
        assert("m\tp\tq\r\n".spec_bytes().skip(2) =~= "p\tq\r\n".spec_bytes());
    }
    expect_inventory_row("\t", "", "");
    expect_inventory_row("m\tp\tq\r\n", "m", "p\tq\r\n");
    expect_missing_inventory_row("", "invalid Git tree row");
    expect_missing_inventory_row("missing", "invalid Git index row");
}

fn expect_inventory_records(actual: &Vec<&str>, expected: &[&str])
    requires policy::inventory_record_views(actual@) == policy::inventory_record_views(expected@),
{
    proof {
        assert(policy::inventory_record_views(actual@).len() == actual@.len());
        assert(policy::inventory_record_views(expected@).len() == expected@.len());
    }
    expect_identifier_true(actual.len() == expected.len());
    let mut i = 0usize;
    while i < actual.len()
        invariant i <= actual.len(), actual.len() == expected.len(),
            policy::inventory_record_views(actual@) == policy::inventory_record_views(expected@),
        decreases actual.len() - i,
    {
        proof {
            assert(policy::inventory_record_views(actual@)[i as int]
                == actual@[i as int].spec_bytes());
            assert(policy::inventory_record_views(expected@)[i as int]
                == expected@[i as int].spec_bytes());
        }
        let same = policy::equal_bytes(actual[i].as_bytes(), expected[i].as_bytes());
        expect_identifier_true(same);
        i += 1;
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_inventory_records_skip_empty_keep_order_repeats_and_control_bytes() {
    let raw = "\0a\0\0b\tc\r\n\0a\0";
    let expected = ["a", "b\tc\r\n", "a"];
    proof {
        reveal_strlit("\0a\0\0b\tc\r\n\0a\0");
        reveal_strlit("a"); reveal_strlit("b\tc\r\n");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii(raw) && vstd::string::is_ascii("a")
            && vstd::string::is_ascii("b\tc\r\n"));
        let bytes = raw.spec_bytes();
        policy::ascii_separator_end_is(bytes, 0, 0);
        policy::ascii_separator_end_is(bytes.skip(1), 0, 1);
        policy::ascii_separator_end_is(bytes.skip(3), 0, 0);
        policy::ascii_separator_end_is(bytes.skip(4), 0, 5);
        policy::ascii_separator_end_is(bytes.skip(10), 0, 1);
        policy::ascii_separator_end_is(bytes.skip(12), 0, 0);
        vstd::seq_lib::lemma_seq_skip_of_skip(bytes, 10, 2);
        vstd::seq_lib::lemma_seq_skip_of_skip(bytes, 4, 6);
        vstd::seq_lib::lemma_seq_skip_of_skip(bytes, 3, 1);
        vstd::seq_lib::lemma_seq_skip_of_skip(bytes, 1, 2);
        reveal_with_fuel(policy::ascii_split_records, 1);
        assert(policy::ascii_split_records(bytes.skip(12), 0, false) == Seq::<Seq<u8>>::empty());
        assert(bytes.skip(10).take(1) =~= "a".spec_bytes());
        assert(policy::ascii_split_records(bytes.skip(10), 0, false) == seq!["a".spec_bytes()]);
        assert(bytes.skip(4).take(5) =~= "b\tc\r\n".spec_bytes());
        assert(policy::ascii_split_records(bytes.skip(4), 0, false)
            == seq!["b\tc\r\n".spec_bytes(), "a".spec_bytes()]);
        assert(policy::ascii_split_records(bytes.skip(3), 0, false)
            == seq!["b\tc\r\n".spec_bytes(), "a".spec_bytes()]);
        assert(bytes.skip(1).take(1) =~= "a".spec_bytes());
        assert(policy::ascii_split_records(bytes.skip(1), 0, false)
            == seq!["a".spec_bytes(), "b\tc\r\n".spec_bytes(), "a".spec_bytes()]);
        assert(policy::inventory_record_views(expected@)
            =~= seq!["a".spec_bytes(), "b\tc\r\n".spec_bytes(), "a".spec_bytes()]);
        assert(policy::ascii_split_records(bytes, 0, false)
            =~= policy::inventory_record_views(expected@));
    }
    let out = policy::git_inventory_records(raw);
    expect_inventory_records(&out, &expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_inventory_records_empty_and_no_delimiter_boundaries() {
    let empty: [&str; 0] = [];
    proof {
        reveal_strlit(""); reveal_strlit("\0\0"); reveal_strlit("last");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("") && vstd::string::is_ascii("\0\0")
            && vstd::string::is_ascii("last"));
        policy::ascii_separator_end_is("".spec_bytes(), 0, 0);
        policy::ascii_separator_end_is("\0\0".spec_bytes(), 0, 0);
        policy::ascii_separator_end_is("\0\0".spec_bytes().skip(1), 0, 0);
        policy::ascii_separator_end_is("\0\0".spec_bytes().skip(2), 0, 0);
        policy::ascii_separator_end_is("last".spec_bytes(), 0, 4);
        vstd::seq_lib::lemma_seq_skip_of_skip("\0\0".spec_bytes(), 1, 1);
        reveal_with_fuel(policy::ascii_split_records, 1);
        assert(policy::ascii_split_records("".spec_bytes(), 0, false) == Seq::<Seq<u8>>::empty());
        assert(policy::ascii_split_records("\0\0".spec_bytes().skip(2), 0, false)
            == Seq::<Seq<u8>>::empty());
        assert(policy::ascii_split_records("\0\0".spec_bytes().skip(1), 0, false)
            == Seq::<Seq<u8>>::empty());
        assert(policy::ascii_split_records("\0\0".spec_bytes(), 0, false)
            == Seq::<Seq<u8>>::empty());
        assert(policy::inventory_record_views(empty@) =~= Seq::<Seq<u8>>::empty());
        assert("last".spec_bytes().take(4) =~= "last".spec_bytes());
        assert(policy::ascii_split_records("last".spec_bytes(), 0, false)
            == seq!["last".spec_bytes()]);
    }
    expect_inventory_records(&policy::git_inventory_records(""), &empty);
    expect_inventory_records(&policy::git_inventory_records("\0\0"), &empty);
    expect_inventory_records(&policy::git_inventory_records("last"), &["last"]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_inventory_fields_keep_all_empty_fields_and_only_split_ascii_space() {
    let raw = " a  b ";
    let expected = ["", "a", "", "b", ""];
    proof {
        reveal_strlit(" a  b "); reveal_strlit(""); reveal_strlit("a"); reveal_strlit("b");
        reveal_strlit("a\tb\r\n");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii(raw) && vstd::string::is_ascii("")
            && vstd::string::is_ascii("a") && vstd::string::is_ascii("b")
            && vstd::string::is_ascii("a\tb\r\n"));
        let bytes = raw.spec_bytes();
        policy::ascii_separator_end_is(bytes, 32, 0);
        policy::ascii_separator_end_is(bytes.skip(1), 32, 1);
        policy::ascii_separator_end_is(bytes.skip(3), 32, 0);
        policy::ascii_separator_end_is(bytes.skip(4), 32, 1);
        policy::ascii_separator_end_is(bytes.skip(6), 32, 0);
        policy::ascii_separator_end_is("".spec_bytes(), 32, 0);
        policy::ascii_separator_end_is("a\tb\r\n".spec_bytes(), 32, 5);
        vstd::seq_lib::lemma_seq_skip_of_skip(bytes, 4, 2);
        vstd::seq_lib::lemma_seq_skip_of_skip(bytes, 3, 1);
        vstd::seq_lib::lemma_seq_skip_of_skip(bytes, 1, 2);
        reveal_with_fuel(policy::ascii_split_records, 1);
        assert(bytes.take(0) =~= "".spec_bytes());
        assert(bytes.skip(3).take(0) =~= "".spec_bytes());
        assert(bytes.skip(6).take(0) =~= "".spec_bytes());
        assert(bytes.skip(4).take(1) =~= "b".spec_bytes());
        assert(bytes.skip(1).take(1) =~= "a".spec_bytes());
        assert(policy::ascii_split_records(bytes.skip(6), 32, true) == seq!["".spec_bytes()]);
        assert(policy::ascii_split_records(bytes.skip(4), 32, true)
            == seq!["b".spec_bytes(), "".spec_bytes()]);
        assert(policy::ascii_split_records(bytes.skip(3), 32, true)
            == seq!["".spec_bytes(), "b".spec_bytes(), "".spec_bytes()]);
        assert(policy::ascii_split_records(bytes.skip(1), 32, true)
            == seq!["a".spec_bytes(), "".spec_bytes(), "b".spec_bytes(), "".spec_bytes()]);
        assert(policy::inventory_record_views(expected@)
            =~= seq!["".spec_bytes(), "a".spec_bytes(), "".spec_bytes(), "b".spec_bytes(), "".spec_bytes()]);
        assert(policy::ascii_split_records(bytes, 32, true)
            =~= policy::inventory_record_views(expected@));
        assert("".spec_bytes().take(0) =~= "".spec_bytes());
        assert(policy::ascii_split_records("".spec_bytes(), 32, true) == seq!["".spec_bytes()]);
        assert("a\tb\r\n".spec_bytes().take(5) =~= "a\tb\r\n".spec_bytes());
        assert(policy::ascii_split_records("a\tb\r\n".spec_bytes(), 32, true)
            == seq!["a\tb\r\n".spec_bytes()]);
    }
    expect_inventory_records(&policy::git_inventory_fields(raw), &expected);
    expect_inventory_records(&policy::git_inventory_fields(""), &[""]);
    expect_inventory_records(&policy::git_inventory_fields("a\tb\r\n"), &["a\tb\r\n"]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_inventory_splitting_keeps_complete_multibyte_utf8() {
    let raw = "é\0é\0";
    proof {
        reveal_strlit("é"); reveal_strlit("é\0é\0");
        reveal_strlit("é\té\t"); reveal_strlit("é\t"); reveal_strlit("é é");
        assert("é"@ =~= seq!['é']);
        assert("é\0é\0"@ =~= seq!['é', '\0', 'é', '\0']);
        assert("é\té\t"@ =~= seq!['é', '\t', 'é', '\t']);
        assert("é\t"@ =~= seq!['é', '\t']);
        assert("é é"@ =~= seq!['é', ' ', 'é']);
        assert('é' as u32 == 233 && '\0' as u32 == 0 && '\t' as u32 == 9 && ' ' as u32 == 32);
        assert(vstd::utf8::encode_scalar(233) == seq![195u8, 169]) by (compute);
        assert(vstd::utf8::encode_scalar(0) == seq![0u8]) by (compute);
        assert(vstd::utf8::encode_scalar(9) == seq![9u8]) by (compute);
        assert(vstd::utf8::encode_scalar(32) == seq![32u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 5);
        assert("é".spec_bytes() =~= seq![195u8, 169]);
        assert(raw.spec_bytes() =~= seq![195u8, 169, 0, 195, 169, 0]);
        assert("é\té\t".spec_bytes() =~= seq![195u8, 169, 9, 195, 169, 9]);
        assert("é\t".spec_bytes() =~= seq![195u8, 169, 9]);
        assert("é é".spec_bytes() =~= seq![195u8, 169, 32, 195, 169]);
        policy::ascii_separator_end_is(raw.spec_bytes(), 0, 2);
        policy::ascii_separator_end_is(raw.spec_bytes().skip(3), 0, 2);
        policy::ascii_separator_end_is(raw.spec_bytes().skip(6), 0, 0);
        policy::ascii_separator_end_is("é".spec_bytes(), 32, 2);
        policy::ascii_separator_end_is("é\té\t".spec_bytes(), 9, 2);
        policy::ascii_separator_end_is("é é".spec_bytes(), 32, 2);
        policy::ascii_separator_end_is("é é".spec_bytes().skip(3), 32, 2);
        vstd::seq_lib::lemma_seq_skip_of_skip(raw.spec_bytes(), 3, 3);
        reveal_with_fuel(policy::ascii_split_records, 1);
        assert(policy::ascii_split_records(raw.spec_bytes().skip(6), 0, false)
            == Seq::<Seq<u8>>::empty());
        assert(raw.spec_bytes().skip(3).take(2) =~= "é".spec_bytes());
        assert(policy::ascii_split_records(raw.spec_bytes().skip(3), 0, false)
            == seq!["é".spec_bytes()]);
        assert(raw.spec_bytes().take(2) =~= "é".spec_bytes());
        assert(policy::ascii_split_records(raw.spec_bytes(), 0, false)
            == seq!["é".spec_bytes(), "é".spec_bytes()]);
        assert("é".spec_bytes().take(2) =~= "é".spec_bytes());
        assert(policy::ascii_split_records("é".spec_bytes(), 32, true) == seq!["é".spec_bytes()]);
        assert("é é".spec_bytes().skip(3) =~= "é".spec_bytes());
        assert("é é".spec_bytes().take(2) =~= "é".spec_bytes());
        assert(policy::ascii_split_records("é é".spec_bytes(), 32, true)
            == seq!["é".spec_bytes(), "é".spec_bytes()]);
        assert(policy::first_ascii_separator("é\té\t".spec_bytes(), 9, 2));
        assert("é\té\t".spec_bytes().take(2) =~= "é".spec_bytes());
        assert("é\té\t".spec_bytes().skip(3) =~= "é\t".spec_bytes());
    }
    expect_inventory_records(&policy::git_inventory_records(raw), &["é", "é"]);
    expect_inventory_records(&policy::git_inventory_fields("é"), &["é"]);
    expect_inventory_records(&policy::git_inventory_fields("é é"), &["é", "é"]);
    expect_inventory_row("é\té\t", "é", "é\t");
    expect_missing_inventory_row("é", "invalid Git tree row");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_tree_metadata_accepts_exactly_three_fields_including_empty_fields() {
    let empty: [&str; 0] = [];
    expect_inventory_error(policy::git_tree_metadata(&empty), "invalid Git tree metadata");
    let two = ["100644", "blob"];
    expect_inventory_error(policy::git_tree_metadata(&two), "invalid Git tree metadata");
    let three = ["", "", ""];
    expect_inventory_success(policy::git_tree_metadata(&three));
    let four = ["100644", "blob", "oid", ""];
    expect_inventory_error(policy::git_tree_metadata(&four), "invalid Git tree metadata");
}

// stage 必须恰为一个 ASCII 字节，任意其余字段的内容不在此层加强校验。
fn git_index_metadata_stage(stage: &str, expected: bool)
    requires expected == (stage.spec_bytes().len() == 1
        && 48 <= stage.spec_bytes()[0] <= 51),
{
    let fields = ["", "", stage];
    let out = policy::git_index_metadata(&fields);
    if expected {
        expect_inventory_success(out);
    } else {
        expect_inventory_error(out, "invalid Git index metadata");
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn git_index_metadata_rejects_wrong_count_and_nonexact_stages() {
    proof {
        reveal_strlit("0"); reveal_strlit("1"); reveal_strlit("2"); reveal_strlit("3");
        reveal_strlit(""); reveal_strlit("/"); reveal_strlit("4"); reveal_strlit("00");
        reveal_strlit("0\0"); reveal_strlit("0\n");
        assert(vstd::string::is_ascii("0"));
        assert(vstd::string::is_ascii("1"));
        assert(vstd::string::is_ascii("2"));
        assert(vstd::string::is_ascii("3"));
        assert(vstd::string::is_ascii(""));
        assert(vstd::string::is_ascii("/"));
        assert(vstd::string::is_ascii("4"));
        assert(vstd::string::is_ascii("00"));
        assert(vstd::string::is_ascii("0\0"));
        assert(vstd::string::is_ascii("0\n"));
        vstd::string::is_ascii_spec_bytes("0");
        vstd::string::is_ascii_spec_bytes("1");
        vstd::string::is_ascii_spec_bytes("2");
        vstd::string::is_ascii_spec_bytes("3");
        vstd::string::is_ascii_spec_bytes("");
        vstd::string::is_ascii_spec_bytes("/");
        vstd::string::is_ascii_spec_bytes("4");
        vstd::string::is_ascii_spec_bytes("00");
        vstd::string::is_ascii_spec_bytes("0\0");
        vstd::string::is_ascii_spec_bytes("0\n");
    }
    git_index_metadata_stage("0", true);
    git_index_metadata_stage("1", true);
    git_index_metadata_stage("2", true);
    git_index_metadata_stage("3", true);
    git_index_metadata_stage("", false);
    git_index_metadata_stage("/", false);
    git_index_metadata_stage("4", false);
    git_index_metadata_stage("00", false);
    git_index_metadata_stage("0\0", false);
    git_index_metadata_stage("0\n", false);
    let empty: [&str; 0] = [];
    expect_inventory_error(policy::git_index_metadata(&empty), "invalid Git index metadata");
    let short = ["100644", "oid"];
    expect_inventory_error(policy::git_index_metadata(&short), "invalid Git index metadata");
    let long = ["100644", "oid", "0", ""];
    expect_inventory_error(policy::git_index_metadata(&long), "invalid Git index metadata");
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
