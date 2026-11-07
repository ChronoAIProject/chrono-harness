//! 显式 Verus 输出合同、规范参数及原执行结果准入。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "verified_identifiers.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;
use identifiers::{equal_bytes, equal_text};

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct OutputContract {
    pub version: String,
    pub commit: String,
}

impl Clone for OutputContract {
    fn clone(&self) -> (out: Self)
        ensures out.version == self.version, out.commit == self.commit,
    { Self { version: self.version.clone(), commit: self.commit.clone() } }
}

pub open spec fn same_contract(first: &OutputContract, second: &OutputContract) -> bool {
    first.version@ == second.version@ && first.commit@ == second.commit@
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for OutputContract {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { same_contract(self, other) }
}

impl PartialEq for OutputContract {
    fn eq(&self, other: &Self) -> (same: bool)
        ensures same == same_contract(self, other),
    {
        equal_text(self.version.as_str(), other.version.as_str())
            && equal_text(self.commit.as_str(), other.commit.as_str())
    }
}

impl Eq for OutputContract {}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct RegisteredAction {
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    #[cfg_attr(not(verus_keep_ghost), serde(default))]
    pub output_contract: Option<OutputContract>,
}

/// 字段名和必需字段由外部 Serde 解码；这些真实类型也由证明根拥有。
#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
pub struct CompilerReport {
    pub verus: CompilerVersion,
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "verification-results"))]
    pub verification: Verification,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
pub struct CompilerVersion {
    pub version: String,
    pub commit: String,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub struct Verification {
    pub encountered_error: bool,
    pub encountered_vir_error: bool,
    pub success: bool,
    pub verified: u64,
    pub errors: u64,
    pub is_verifying_entire_crate: bool,
}

pub open spec fn valid_contract(contract: &Option<OutputContract>) -> bool {
    match contract {
        None => true,
        Some(value) => value.version@.len() > 0 && value.commit@.len() > 0,
    }
}

pub fn validate_contract(contract: &Option<OutputContract>) -> (out: Result<(), String>)
    ensures out.is_ok() == valid_contract(contract),
        out.is_err() ==> out.unwrap_err()@ == "E_VERUS_CONTRACT: version and commit must be nonempty"@,
{
    match contract {
        None => Ok(()),
        Some(value) => {
            if value.version.as_str().is_empty() || value.commit.as_str().is_empty() {
                Err("E_VERUS_CONTRACT: version and commit must be nonempty".to_string())
            } else { Ok(()) }
        },
    }
}

/// 普通操作不解析其业务输出；只向解码接口传空输入。
pub fn proof_stdout<'a>(contract: &Option<OutputContract>, raw: &'a [u8]) -> (out: &'a [u8])
    ensures out@ == if contract.is_some() { raw@ } else { Seq::<u8>::empty() },
{
    match contract { None => &[], Some(_) => raw }
}

pub proof fn encode_concat(first: Seq<char>, second: Seq<char>)
    ensures vstd::utf8::encode_utf8(first + second)
        == vstd::utf8::encode_utf8(first) + vstd::utf8::encode_utf8(second),
    decreases first.len(),
{
    if first.len() > 0 {
        assert((first + second).drop_first() =~= first.drop_first() + second);
        encode_concat(first.drop_first(), second);
    }
}

fn append_bytes(bytes: &mut Vec<u8>, suffix: &[u8])
    ensures bytes@ == old(bytes)@ + suffix@,
{
    let ghost before = bytes@;
    let mut i = 0;
    while i < suffix.len()
        invariant 0 <= i <= suffix.len(), bytes@ == before + suffix@.take(i as int),
        decreases suffix.len() - i,
    {
        bytes.push(suffix[i]);
        assert(suffix@.take(i + 1) =~= suffix@.take(i as int).push(suffix[i as int]));
        i += 1;
    }
    assert(suffix@.take(i as int) =~= suffix@);
}

pub fn compiler_outputs(output: &str) -> (argument: Vec<u8>)
    ensures argument@ == vstd::utf8::encode_utf8(
        "--emit=link="@ + output@ + ",dep-info="@ + output@ + ".d"@),
{
    let mut argument = Vec::new();
    append_bytes(&mut argument, "--emit=link=".as_bytes());
    append_bytes(&mut argument, output.as_bytes());
    append_bytes(&mut argument, ",dep-info=".as_bytes());
    append_bytes(&mut argument, output.as_bytes());
    append_bytes(&mut argument, ".d".as_bytes());
    proof {
        encode_concat("--emit=link="@, output@);
        encode_concat("--emit=link="@ + output@, ",dep-info="@);
        encode_concat("--emit=link="@ + output@ + ",dep-info="@, output@);
        encode_concat("--emit=link="@ + output@ + ",dep-info="@ + output@, ".d"@);
    }
    argument
}

pub open spec fn verifier_arguments(args: Seq<String>) -> bool {
    args.len() == 12
        && args[0]@ == "--crate-type=lib"@
        && args[1]@ == "--no-cheating"@
        && args[2]@ == "--no-external-by-default"@
        && args[3]@ == "--num-threads"@
        && args[4]@ == "2"@
        && args[5]@ == "--output-json"@
        && args[6]@ == "--compile"@
        && args[7]@ == "--emit=link="@ + args[10]@ + ",dep-info="@ + args[10]@ + ".d"@
        && args[8]@.len() > 0
        && args[9]@ == "-o"@
        && args[10]@.len() > 0
        && args[11]@ == "-Zchecksum-hash-algorithm=sha256"@
}

pub fn canonical_verifier_arguments(args: &Vec<String>) -> (valid: bool)
    ensures valid == verifier_arguments(args@),
{
    if args.len() != 12 { return false; }
    let outputs = compiler_outputs(args[10].as_str());
    let outputs_match = equal_bytes(args[7].as_str().as_bytes(), outputs.as_slice());
    proof {
        vstd::utf8::encode_utf8_decode_utf8(args[7]@);
        vstd::utf8::encode_utf8_decode_utf8(
            "--emit=link="@ + args[10]@ + ",dep-info="@ + args[10]@ + ".d"@);
    }
    equal_text(args[0].as_str(), "--crate-type=lib")
        && equal_text(args[1].as_str(), "--no-cheating")
        && equal_text(args[2].as_str(), "--no-external-by-default")
        && equal_text(args[3].as_str(), "--num-threads")
        && equal_text(args[4].as_str(), "2")
        && equal_text(args[5].as_str(), "--output-json")
        && equal_text(args[6].as_str(), "--compile") && outputs_match
        && !args[8].as_str().is_empty()
        && equal_text(args[9].as_str(), "-o") && !args[10].as_str().is_empty()
        && equal_text(args[11].as_str(), "-Zchecksum-hash-algorithm=sha256")
}

pub open spec fn report_accepted(report: &CompilerReport, contract: &OutputContract) -> bool {
    report.verus.version@ == contract.version@
        && report.verus.commit@ == contract.commit@
        && report.verification.success && !report.verification.encountered_error
        && !report.verification.encountered_vir_error
        && report.verification.errors == 0 && report.verification.verified > 0
        && report.verification.is_verifying_entire_crate
}

pub open spec fn output_accepted(
    contract: &Option<OutputContract>, decoded: &Option<CompilerReport>,
    args: Seq<String>, exit: i32, process_failed: bool,
) -> bool {
    match contract {
        None => true,
        Some(value) => valid_contract(contract) && verifier_arguments(args)
            && exit == 0 && !process_failed
            && match decoded { None => false, Some(report) => report_accepted(report, value) },
    }
}

pub open spec fn output_error(
    contract: &Option<OutputContract>, decoded: &Option<CompilerReport>,
    args: Seq<String>, exit: i32, process_failed: bool,
) -> Option<Seq<char>> {
    match contract {
        None => None,
        Some(value) => {
            if !valid_contract(contract) {
                Some("E_VERUS_CONTRACT: version and commit must be nonempty"@)
            } else if exit != 0 || process_failed || !verifier_arguments(args) {
                Some("E_VERUS_RESULT: process or verifier arguments rejected"@)
            } else {
                match decoded {
                    None => Some("E_VERUS_RESULT: missing or malformed typed report"@),
                    Some(report) => if report_accepted(report, value) { None }
                        else { Some("E_VERUS_RESULT: incomplete, failed or mismatched verification"@) },
                }
            }
        },
    }
}

/// 直接消费同一原进程的退出、失败、参数和一次解码得到的完整报告。
pub fn admit_output(
    contract: &Option<OutputContract>, decoded: &Option<CompilerReport>,
    args: &Vec<String>, exit: i32, process_failed: bool,
) -> (out: Result<(), String>)
    ensures out.is_ok() == output_accepted(contract, decoded, args@, exit, process_failed),
        match out {
            Ok(()) => output_error(contract, decoded, args@, exit, process_failed).is_none(),
            Err(message) => output_error(contract, decoded, args@, exit, process_failed) == Some(message@),
        },
{
    let value = match contract { None => { return Ok(()); }, Some(value) => value };
    match validate_contract(contract) {
        Ok(()) => {},
        Err(error) => { return Err(error); },
    }
    if exit != 0 || process_failed || !canonical_verifier_arguments(args) {
        return Err("E_VERUS_RESULT: process or verifier arguments rejected".to_string());
    }
    let report = match decoded {
        None => { return Err("E_VERUS_RESULT: missing or malformed typed report".to_string()); },
        Some(report) => report,
    };
    if !equal_text(report.verus.version.as_str(), value.version.as_str())
        || !equal_text(report.verus.commit.as_str(), value.commit.as_str())
        || !report.verification.success || report.verification.encountered_error
        || report.verification.encountered_vir_error || report.verification.errors != 0
        || report.verification.verified == 0 || !report.verification.is_verifying_entire_crate {
        return Err("E_VERUS_RESULT: incomplete, failed or mismatched verification".to_string());
    }
    Ok(())
}

}
