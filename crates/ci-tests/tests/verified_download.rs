//! 下载状态与停止条件的独立预期，同时由 Verus 和普通 Rust 测试执行。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../ci/src/verified_download.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_ci::verified_download as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_complete_ascii_status_tokens_are_accepted() {
    let empty = policy::http_status(&[]);
    let short = policy::http_status(&[72u8, 84, 84, 80, 32, 53, 48, 51]);
    let valid = policy::http_status(&[72u8, 84, 84, 80, 32, 53, 48, 51, 58]);
    let wrong_prefix = policy::http_status(&[104u8, 84, 84, 80, 32, 53, 48, 51, 58]);
    let not_a_digit = policy::http_status(&[72u8, 84, 84, 80, 32, 53, 255, 51, 58]);
    let wrong_suffix = policy::http_status(&[72u8, 84, 84, 80, 32, 53, 48, 51, 59]);
    assert(empty == None && short == None && valid == Some(503)
        && wrong_prefix == None && not_a_digit == None && wrong_suffix == None);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn repeated_statuses_are_unambiguous_but_conflicting_statuses_are_rejected() {
    let repeated = policy::http_status(&[
        72u8, 84, 84, 80, 32, 53, 48, 51, 58,
        72, 84, 84, 80, 32, 53, 48, 51, 58,
    ]);
    let conflict = policy::http_status(&[
        72u8, 84, 84, 80, 32, 53, 48, 51, 58,
        72, 84, 84, 80, 32, 52, 48, 49, 58,
    ]);
    let embedded = policy::http_status(&[255u8, 72, 84, 84, 80, 32, 48, 48, 48, 58, 0]);
    let maximum = policy::http_status(&[72u8, 84, 84, 80, 32, 57, 57, 57, 58]);
    assert(repeated == Some(503) && conflict == None && embedded == Some(0) && maximum == Some(999));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_missing_retry_obligation_stops_another_download() {
    let allowed = policy::retry(false, false, true, true, 1, 1, true);
    let successful = policy::retry(true, false, true, true, 1, 1, true);
    let failed_process = policy::retry(false, true, true, true, 1, 1, true);
    let exhausted = policy::retry(false, false, false, true, 1, 1, true);
    let unlisted = policy::retry(false, false, true, false, 1, 1, true);
    let no_stdout = policy::retry(false, false, true, true, 0, 1, true);
    let no_stderr = policy::retry(false, false, true, true, 1, 0, true);
    let deadline = policy::retry(false, false, true, true, 1, 1, false);
    assert(allowed && !successful && !failed_process && !exhausted && !unlisted
        && !no_stdout && !no_stderr && !deadline);
}

}
