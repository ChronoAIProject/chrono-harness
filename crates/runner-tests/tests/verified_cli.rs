//! 参数原文保全、首错和两种位置的独立预期，同时执行 Verus 与 Rust。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_cli.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_cli as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect_true(value: bool)
    requires value,
{
    if !value { panic!(); }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_valid_arguments_keep_empty_unicode_replacement_and_duplicates() {
    let input = [Some("tool"), Some(""), Some("目录/é/�"), Some("same"), Some("same")];
    let output = policy::decoded_arguments(&input);
    match output {
        Ok(texts) => {
            expect_true(texts.len() == 5);
            expect_true(texts[0] == "tool");
            expect_true(texts[1] == "");
            expect_true(texts[2] == "目录/é/�");
            expect_true(texts[3] == "same");
            expect_true(texts[4] == "same");
        },
        Err(_) => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_input_and_empty_program_name_remain_valid_text() {
    match policy::decoded_arguments(&[]) {
        Ok(texts) => expect_true(texts.len() == 0),
        Err(_) => panic!(),
    }
    match policy::decoded_arguments(&[Some("")]) {
        Ok(texts) => {
            expect_true(texts.len() == 1);
            expect_true(texts[0] == "");
        },
        Err(_) => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn invalid_program_name_has_no_user_argument_index() {
    match policy::decoded_arguments(&[None, Some("--help"), None]) {
        Err(error) => {
            expect_true(error.argv_index == 0);
            expect_true(error.argument_index.is_none());
        },
        Ok(_) => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn first_invalid_user_argument_wins_over_later_failures() {
    match policy::decoded_arguments(&[Some("tool"), Some("check"), None, Some(""), None]) {
        Err(error) => {
            expect_true(error.argv_index == 2);
            expect_true(error.argument_index == Some(1));
        },
        Ok(_) => panic!(),
    }
    match policy::decoded_arguments(&[Some("tool"), Some(""), Some("�"), None]) {
        Err(error) => {
            expect_true(error.argv_index == 3);
            expect_true(error.argument_index == Some(2));
        },
        Ok(_) => panic!(),
    }
}

}
