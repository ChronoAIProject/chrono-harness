//! 发布配方的实际判断；文件、进程和原生 metadata 解码由各自 IO 边界供应。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../../../crates/identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_identifiers as identifiers;

#[path = "verified_delivery.rs"]
pub mod delivery;

verus! {

/// 调用者显式提供角色与选择；不根据项目语言、命令或资产数推断 SDK。
pub enum Invocation {
    Build,
    Verify { rust_toolchain: bool },
    Collect,
}

pub fn requires_rust(invocation: &Invocation) -> (required: bool)
    ensures required == match invocation {
        Invocation::Build => true,
        Invocation::Verify { rust_toolchain } => *rust_toolchain,
        Invocation::Collect => false,
    },
{
    match invocation {
        Invocation::Build => true,
        Invocation::Verify { rust_toolchain } => *rust_toolchain,
        Invocation::Collect => false,
    }
}

/// 内核退出码为 0..255，信号按原进程回执记为 -1..-127。
/// None 表示尚无真实子进程终态，不能冒充成功。
pub fn result_exit(child: Option<i32>, auxiliary_failed: bool) -> (exit: i32)
    requires child is Some ==> -127 <= child.unwrap() <= 255,
    ensures
        0 <= exit <= 255,
        child is None ==> exit == 2,
        child is Some && child.unwrap() > 0 ==> exit == child.unwrap(),
        child is Some && child.unwrap() < 0 ==> exit == 128 - child.unwrap(),
        child == Some(0i32) ==> exit == if auxiliary_failed { 2i32 } else { 0i32 },
{
    match child {
        None => 2,
        Some(code) => {
            if code > 0 { code }
            else if code < 0 { 128 - code }
            else if auxiliary_failed { 2 }
            else { 0 }
        },
    }
}

pub enum DependencyResult { Success, Failure, Cancelled, Skipped }

/// 工具产物与业务单元共享源码，但原生平台与实际生产者必须分别绑定。
pub fn frontend_header(
    schema: &str, github: bool, commit: &str, tree: &str, system: &str, machine: &str,
    expected_commit: &str, expected_tree: &str, expected_system: &str, expected_machine: &str,
) -> (valid: bool)
    ensures valid <==> (schema@ == "chrono-release-frontend/v1"@ && github
        && commit@ == expected_commit@ && tree@ == expected_tree@
        && system@ == expected_system@ && machine@ == expected_machine@),
{
    identifiers::equal_text(schema, "chrono-release-frontend/v1") && github
        && identifiers::equal_text(commit, expected_commit)
        && identifiers::equal_text(tree, expected_tree)
        && identifiers::equal_text(system, expected_system)
        && identifiers::equal_text(machine, expected_machine)
}

pub fn frontend_installation(
    planned: u64, reported: u64, destination: &str, installed: &str,
    hash: &str, expected_hash: &str,
) -> (valid: bool)
    ensures valid <==> (planned == 1 && reported == 1
        && destination@ == ".chrono-harness/bin/chrono-host-release"@
        && installed@ == destination@ && hash@ == expected_hash@),
{
    planned == 1 && reported == 1
        && identifiers::equal_text(destination, ".chrono-harness/bin/chrono-host-release")
        && identifiers::equal_text(installed, destination)
        && identifiers::equal_text(hash, expected_hash)
}

pub fn executable_file(regular: bool, mode: u32) -> (valid: bool)
    ensures valid == (regular && mode <= 4095 && mode & 73 != 0),
{
    regular && mode <= 4095 && mode & 73 != 0
}

pub fn same_content(hash: &str, size: u64, expected_hash: &str, expected_size: u64) -> (same: bool)
    ensures same == (hash@ == expected_hash@ && size == expected_size),
{
    identifiers::equal_text(hash, expected_hash) && size == expected_size
}

pub fn same_executable(
    hash: &str, size: u64, mode: u32,
    expected_hash: &str, expected_size: u64, expected_mode: u32,
) -> (same: bool)
    ensures same == (hash@ == expected_hash@ && size == expected_size && mode == expected_mode),
{
    same_content(hash, size, expected_hash, expected_size) && mode == expected_mode
}

/// tar 解码只提供原字段；成员必须是唯一的字面 executable 普通文件。
pub fn executable_entry(
    name: &[u8], regular: bool, size: u64, mode: u32,
    expected_size: u64, expected_mode: u32,
) -> (valid: bool)
    ensures valid == (regular
        && name@ == seq![101u8,120,101,99,117,116,97,98,108,101]
        && size == expected_size && mode == expected_mode),
{
    regular && identifiers::equal_bytes(name, &[101u8,120,101,99,117,116,97,98,108,101])
        && size == expected_size && mode == expected_mode
}

/// 先核当前 needs 的结果，再接受同一 run 的原生产 attempt。
/// 旧成功原件不能覆盖当前 failure／cancelled／skipped。
pub fn selected_producer(
    current: &DependencyResult,
    selected_run: &str,
    current_run: &str,
    selected_attempt: u64,
    current_attempt: u64,
    artifact_id: &str,
) -> (valid: bool)
    ensures valid <==> (current is Success
        && selected_run@ == current_run@ && selected_run@.len() > 0
        && 0 < selected_attempt <= current_attempt && artifact_id@.len() > 0),
{
    matches!(current, DependencyResult::Success)
        && identifiers::equal_text(selected_run, current_run)
        && !selected_run.is_empty()
        && selected_attempt > 0 && selected_attempt <= current_attempt
        && !artifact_id.is_empty()
}

/// 同一逻辑单元只消费已声明的原生产 job，不能借同一 run 替换来源。
pub fn producer_lineage(
    producing_job: &str,
    expected_job: &str,
    producing_run: &str,
    selected_run: &str,
    producing_attempt: u64,
    selected_attempt: u64,
) -> (valid: bool)
    ensures valid <==> (producing_job@ == expected_job@
        && producing_job@.len() > 0
        && producing_run@ == selected_run@
        && producing_run@.len() > 0
        && 0 < producing_attempt && producing_attempt == selected_attempt),
{
    identifiers::equal_text(producing_job, expected_job) && !producing_job.is_empty()
        && identifiers::equal_text(producing_run, selected_run)
        && !producing_run.is_empty()
        && producing_attempt > 0 && producing_attempt == selected_attempt
}

/// 原始操作已结束后，观察或证据失败只能补充失败，不能抹去真实子进程失败。
pub fn passed(child: Option<i32>, auxiliary_failed: bool) -> (success: bool)
    ensures success <==> (child == Some(0i32) && !auxiliary_failed),
{
    matches!(child, Some(0)) && !auxiliary_failed
}

}
