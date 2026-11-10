//! 显式 Verus 输出合同、规范参数及原执行结果准入。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;
use identifiers::{equal_bytes, equal_text};

#[path = "verified_cargo.rs"]
pub mod cargo;
#[path = "verified_sources.rs"]
pub mod sources;

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

/// 原编译进程结束后取得的库文件身份；不在回执中复制库内容。
#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct CompiledArtifact {
    pub path: Vec<u8>,
    pub sha256: Vec<u8>,
    pub length: u64,
}

impl Clone for CompiledArtifact {
    fn clone(&self) -> (out: Self)
        ensures out.path@ == self.path@, out.sha256@ == self.sha256@,
            out.length == self.length,
    {
        Self { path: self.path.clone(), sha256: self.sha256.clone(), length: self.length }
    }
}

/// 外部文件／SHA-256 接口返回点名普通文件的实际摘要与长度或原错误。
pub trait ArtifactRead {
    fn identity(&self, path: &[u8]) -> Result<(Vec<u8>, u64), String>;
}

pub open spec fn artifact_matches(artifact: &CompiledArtifact, args: Seq<String>) -> bool {
    verifier_arguments(args)
        && artifact.path@ == vstd::utf8::encode_utf8(args[10]@)
        && identifiers::digest_format(artifact.sha256@) && artifact.length > 0
}

/// 失败进程与非规范参数不读取库；原 IO 错误仍进入同一回执。
pub fn observe_artifact<I: ArtifactRead>(
    contract: &Option<OutputContract>, args: &Vec<String>,
    exit: i32, process_failed: bool, io: &I,
) -> (observed: Option<Result<CompiledArtifact, String>>)
    ensures observed.is_none() == contract.is_none(),
        match observed {
            Some(Ok(artifact)) => exit == 0 && !process_failed && artifact_matches(&artifact, args@),
            _ => true,
        },
{
    if contract.is_none() { return None; }
    if exit != 0 || process_failed || !canonical_verifier_arguments(args) {
        return Some(Err("E_VERUS_ARTIFACT: original compiler process was not successful".to_string()));
    }
    let (sha256, length) = match io.identity(args[10].as_str().as_bytes()) {
        Ok(value) => value,
        Err(error) => { return Some(Err(error)); },
    };
    if length == 0 || !identifiers::valid_digest(sha256.as_slice()) {
        return Some(Err("E_VERUS_ARTIFACT: empty library or invalid observed digest".to_string()));
    }
    let mut path = Vec::new();
    append_bytes(&mut path, args[10].as_str().as_bytes());
    Some(Ok(CompiledArtifact { path, sha256, length }))
}

pub open spec fn artifact_observed(
    contract: &Option<OutputContract>, observed: &Option<Result<CompiledArtifact, String>>,
    args: Seq<String>,
) -> bool {
    match contract {
        None => observed.is_none(),
        Some(_) => match observed {
            Some(Ok(artifact)) => artifact_matches(artifact, args),
            _ => false,
        },
    }
}

pub fn admit_artifact_observation(
    contract: &Option<OutputContract>, observed: &Option<Result<CompiledArtifact, String>>,
    args: &Vec<String>,
) -> (out: Result<(), String>)
    ensures out.is_ok() == artifact_observed(contract, observed, args@),
{
    if contract.is_none() {
        return if observed.is_none() { Ok(()) }
            else { Err("E_VERUS_ARTIFACT: ordinary operation has compiler library evidence".to_string()) };
    }
    if !canonical_verifier_arguments(args) {
        return Err("E_VERUS_ARTIFACT: verifier arguments differ from the registered invocation".to_string());
    }
    let artifact = match observed {
        Some(Ok(artifact)) => artifact,
        _ => { return Err("E_VERUS_ARTIFACT: missing or failed original library observation".to_string()); },
    };
    if !equal_bytes(artifact.path.as_slice(), args[10].as_str().as_bytes())
        || !identifiers::valid_digest(artifact.sha256.as_slice()) || artifact.length == 0 {
        return Err("E_VERUS_ARTIFACT: library path, digest or length rejected".to_string());
    }
    Ok(())
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

/// 完整证明结果的准入；原进程和调用参数仍由入口核对。
pub fn accepts_report(report: &CompilerReport, contract: &OutputContract) -> (accepted: bool)
    ensures accepted == report_accepted(report, contract),
{
    equal_text(report.verus.version.as_str(), contract.version.as_str())
        && equal_text(report.verus.commit.as_str(), contract.commit.as_str())
        && report.verification.success && !report.verification.encountered_error
        && !report.verification.encountered_vir_error && report.verification.errors == 0
        && report.verification.verified > 0 && report.verification.is_verifying_entire_crate
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
    if !accepts_report(report, value) {
        return Err("E_VERUS_RESULT: incomplete, failed or mismatched verification".to_string());
    }
    Ok(())
}

/// 本地与离线回执共同消费原报告及原依赖观察。
pub fn admit_observed_output(
    contract: &Option<OutputContract>, decoded: &Option<CompilerReport>,
    args: &Vec<String>, exit: i32, process_failed: bool,
    dependency: &Option<Result<Vec<u8>, String>>, limit: usize,
    artifact: &Option<Result<CompiledArtifact, String>>,
) -> (out: Result<(), String>)
    ensures out.is_ok() == (output_accepted(contract, decoded, args@, exit, process_failed)
        && sources::dependency_observed(contract, dependency, limit)
        && artifact_observed(contract, artifact, args@)),
{
    match admit_output(contract, decoded, args, exit, process_failed) {
        Err(error) => { return Err(error); },
        Ok(()) => {},
    }
    match sources::admit_dependency_observation(contract, dependency, limit) {
        Err(error) => Err(error),
        Ok(()) => admit_artifact_observation(contract, artifact, args),
    }
}

/// 现役计划的借用投影；没有独立的准入标志或第二份方法权威。
pub struct ProofInvocation<'a> {
    pub operation: &'a str,
    pub argv: &'a Vec<String>,
    pub output_contract: &'a Option<OutputContract>,
    pub output_limit_bytes: usize,
}

/// 同一原回执的输入投影，库和源码观察不能来自另一次进程。
pub struct ProofProcess<'a> {
    pub operation: &'a str,
    pub argv: &'a Vec<String>,
    pub stdout: &'a [u8],
    pub exit: i32,
    pub failed: bool,
    pub dependency: &'a Option<Result<Vec<u8>, String>>,
    pub artifact: &'a Option<Result<CompiledArtifact, String>>,
}

/// 只负责原字节的标准 JSON 类型化解码；报告准入仍由本证明根负责。
pub trait CompilerReportRead {
    spec fn decoded(&self, bytes: Seq<u8>) -> Option<CompilerReport>;
    fn decode(&self, bytes: &[u8]) -> (out: Option<CompilerReport>)
        ensures out == self.decoded(bytes@);
}

pub struct ProofAdmission {
    pub report: CompilerReport,
    pub sources: sources::SourceAdmission,
}

pub open spec fn unique_proof_invocation(
    methods: Seq<ProofInvocation>, operation: Seq<char>, index: int,
) -> bool {
    0 <= index < methods.len() && methods[index].operation@ == operation
        && forall|i: int| 0 <= i < methods.len() && (#[trigger] methods[i]).operation@ == operation
            ==> i == index
}

pub fn find_proof_invocation(methods: &Vec<ProofInvocation>, operation: &str)
    -> (out: Result<usize, String>)
    ensures out.is_ok() == (exists|i: int| unique_proof_invocation(methods@, operation@, i)),
        out.is_ok() ==> unique_proof_invocation(methods@, operation@, out.unwrap() as int),
{
    let mut found: Option<usize> = None;
    let mut i = 0;
    while i < methods.len()
        invariant 0 <= i <= methods.len(),
            match found {
                None => forall|j: int| 0 <= j < i ==> (#[trigger] methods[j]).operation@ != operation@,
                Some(index) => 0 <= index < i && methods[index as int].operation@ == operation@
                    && forall|j: int| 0 <= j < i && (#[trigger] methods[j]).operation@ == operation@
                        ==> j == index,
            },
        decreases methods.len() - i,
    {
        if equal_text(methods[i].operation, operation) {
            if let Some(index) = found {
                assert(index != i);
                assert forall|j: int| unique_proof_invocation(methods@, operation@, j)
                    implies false by {
                    assert(j == index as int);
                    assert(j == i as int);
                }
                return Err("E_VERUS_RESULT: ambiguous registered operation".to_string());
            }
            found = Some(i);
        }
        i += 1;
    }
    match found {
        Some(index) => {
            assert(unique_proof_invocation(methods@, operation@, index as int));
            assert(exists|j: int| unique_proof_invocation(methods@, operation@, j));
            Ok(index)
        },
        None => {
            assert forall|j: int| unique_proof_invocation(methods@, operation@, j)
                implies false by {
                assert(0 <= j < methods.len());
                assert(methods[j].operation@ != operation@);
            }
            Err("E_VERUS_RESULT: operation has no registered invocation".to_string())
        },
    }
}

pub open spec fn same_arguments(first: Seq<String>, second: Seq<String>) -> bool {
    first.len() == second.len()
        && forall|i: int| 0 <= i < first.len() ==> (#[trigger] first[i])@ == second[i]@
}

pub fn equal_arguments(first: &Vec<String>, second: &Vec<String>) -> (same: bool)
    ensures same == same_arguments(first@, second@),
{
    if first.len() != second.len() { return false; }
    let mut i = 0;
    while i < first.len()
        invariant 0 <= i <= first.len(), first.len() == second.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] first[j])@ == second[j]@,
        decreases first.len() - i,
    {
        if !equal_text(first[i].as_str(), second[i].as_str()) { return false; }
        i += 1;
    }
    true
}

pub open spec fn proof_bound(
    proof: &ProofAdmission, method: &ProofInvocation, process: &ProofProcess,
    registry: &sources::SourceRegistry,
) -> bool {
    method.output_contract.is_some() && process.argv.len() > 0
        && process.operation@ == method.operation@
        && same_arguments(process.argv@.skip(1), method.argv@)
        && process.exit == 0 && !process.failed
        && output_accepted(method.output_contract, &Some(proof.report), process.argv@.skip(1),
            process.exit, process.failed)
        && sources::dependency_observed(method.output_contract, process.dependency, method.output_limit_bytes)
        && artifact_observed(method.output_contract, process.artifact, process.argv@.skip(1))
        && sources::binding_error(registry.files@, method.operation@, method.argv[8]@,
            proof.sources.candidate@, 0).is_none()
        && match process.dependency {
            Some(Ok(bytes)) => sources::admission_matches(&proof.sources,
                process.argv@.skip(1), bytes@, registry.files@),
            _ => false,
        }
}

/// 无证明合同的普通操作成功，仍须绑定唯一方法、完整原参数及无失败原进程。
pub open spec fn ordinary_process_bound(method: &ProofInvocation, process: &ProofProcess) -> bool {
    method.output_contract.is_none() && process.argv.len() > 0
        && process.operation@ == method.operation@
        && same_arguments(process.argv@.skip(1), method.argv@)
        && process.exit == 0 && !process.failed
        && process.dependency.is_none() && process.artifact.is_none()
}

pub open spec fn ordinary_output_accepted(
    operation: Seq<char>, methods: Seq<ProofInvocation>, original: &Option<ProofProcess>,
) -> bool {
    match original {
        Some(process) => exists|i: int| unique_proof_invocation(methods, operation, i)
            && ordinary_process_bound(&methods[i], process),
        None => false,
    }
}

/// 成功报告、原 argv、库、完整依赖和固定候选源码在此共同准入。
/// 普通成功与非 passed 操作只返回 None，不能用作全仓覆盖的证明条目。
pub fn bind_operation_output<I: sources::CandidateSourceRead + CompilerReportRead,
    F: Fn(&str, &str) -> String>(
    state: &str, operation: &str, methods: &Vec<ProofInvocation>,
    original: Option<ProofProcess>, registry: &Result<sources::SourceRegistry, String>, io: &I,
    format_error: F,
) -> (out: Result<Option<ProofAdmission>, String>)
    requires forall|message: &str, path: &str| format_error.requires((message, path)),
    ensures state@ != "passed"@ ==> out.is_ok() && out.unwrap().is_none(),
        state@ == "passed"@ ==> ((match out { Ok(None) => true, _ => false })
            == ordinary_output_accepted(operation@, methods@, &original)),
        match out {
            Ok(Some(proof)) => state@ == "passed"@ && original.is_some() && registry.is_ok()
                && exists|i: int| unique_proof_invocation(methods@, operation@, i)
                    && proof_bound(&proof, &methods[i], &original.unwrap(), &registry.unwrap())
                    && io.decoded(original.unwrap().stdout@) == Some(proof.report),
            Ok(None) => state@ != "passed"@ || exists|i: int|
                unique_proof_invocation(methods@, operation@, i) && methods[i].output_contract.is_none(),
            Err(_) => true,
        },
        state@ == "passed"@ && out.is_ok() ==> original.is_some()
            && original.unwrap().exit == 0 && !original.unwrap().failed,
{
    if !equal_text(state, "passed") { return Ok(None); }
    let process = match original {
        Some(process) => process,
        None => { return Err("E_VERUS_RESULT: successful operation has no original receipt".to_string()); },
    };
    let index = match find_proof_invocation(methods, operation) {
        Ok(index) => index,
        Err(error) => { return Err(error); },
    };
    let method = &methods[index];
    proof {
        if ordinary_output_accepted(operation@, methods@, &original) {
            let ordinary_index = choose|i: int| unique_proof_invocation(methods@, operation@, i)
                && ordinary_process_bound(&methods[i], &process);
            assert(ordinary_index == index as int);
            assert(ordinary_process_bound(method, &process));
        }
    }
    let args = match sources::process_arguments(process.argv) {
        Ok(args) => args,
        Err(error) => { return Err(error); },
    };
    if !equal_text(process.operation, operation) || !equal_arguments(&args, method.argv)
        || process.exit != 0 || process.failed {
        return Err("E_VERUS_RESULT: original operation, arguments or process rejected".to_string());
    }
    let decoded = io.decode(proof_stdout(method.output_contract, process.stdout));
    match admit_observed_output(method.output_contract, &decoded, &args,
        process.exit, process.failed, process.dependency, method.output_limit_bytes, process.artifact) {
        Err(error) => { return Err(error); },
        Ok(()) => {},
    }
    if method.output_contract.is_none() {
        assert(ordinary_process_bound(method, &process));
        assert(ordinary_output_accepted(operation@, methods@, &original));
        return Ok(None);
    }
    assert(!ordinary_output_accepted(operation@, methods@, &original));
    let source_admission = match sources::bind_compiler_sources(&args, process.dependency, registry, io) {
        Ok(Some(admission)) => admission,
        Err(error) => { return Err(error); },
        Ok(None) => { return Err("E_VERUS_RESULT: proof has no bound compiler sources".to_string()); },
    };
    let source_registry = match registry {
        Ok(registry) => registry,
        Err(error) => { return Err(error.clone()); },
    };
    match sources::bind_registered_roots(&source_registry.files, operation,
        args[8].as_str(), &source_admission.candidate) {
        Ok(()) => {},
        Err(error) => { return Err(error.render(format_error)); },
    }
    let report = match decoded {
        Some(report) => report,
        None => { return Err("E_VERUS_RESULT: proof has no original typed report".to_string()); },
    };
    let proof = ProofAdmission { report, sources: source_admission };
    assert(unique_proof_invocation(methods@, operation@, index as int));
    assert(proof_bound(&proof, method, &process, &registry.unwrap()));
    Ok(Some(proof))
}

}
