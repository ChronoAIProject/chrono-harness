//! 原进程与完整 Verus 输出的独立成功、失败和边界预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_verifier.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_verifier as policy;
#[cfg(verus_keep_ghost)]
use policy::identifiers;
use policy::sources::SourceRole;
use policy::{CompilerReport, CompilerVersion, OutputContract, Verification};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

/// 普通测试执行失败分支；同一证明根要求每个原布尔预期成立。
fn expect_verifier_true(actual: bool)
    requires actual,
{
    if !actual {
        panic!();
    }
}

fn expect_verifier_bytes(actual: &[u8], expected: &[u8])
    requires actual@ == expected@,
{
    let equal = identifiers::equal_bytes(actual, expected);
    expect_verifier_true(equal);
}

fn contract() -> (out: Option<OutputContract>)
    ensures out.is_some(), out.unwrap().version@ == "v"@, out.unwrap().commit@ == "c"@,
        policy::valid_contract(&out),
{
    proof { reveal_strlit("v"); reveal_strlit("c"); }
    Some(OutputContract { version: "v".to_string(), commit: "c".to_string() })
}

fn verification() -> (out: Verification)
    ensures out.success, !out.encountered_error, !out.encountered_vir_error,
        out.verified == 1, out.errors == 0, out.is_verifying_entire_crate,
{
    Verification { success: true, encountered_error: false, encountered_vir_error: false,
        verified: 1, errors: 0, is_verifying_entire_crate: true }
}

fn report(verification: Verification) -> (out: Option<CompilerReport>)
    ensures out.is_some(), out.unwrap().verification == verification,
        out.unwrap().verus.version@ == "v"@, out.unwrap().verus.commit@ == "c"@,
{
    Some(CompilerReport { verus: CompilerVersion {
        version: "v".to_string(), commit: "c".to_string(),
    }, verification })
}

fn arguments() -> (out: Vec<String>)
    ensures policy::verifier_arguments(out@),
        out[7]@ == "--emit=link=out,dep-info=out.d"@,
        out[8]@ == "root"@, out[10]@ == "out"@,
{
    proof {
        reveal_strlit("--emit=link="); reveal_strlit(",dep-info=");
        reveal_strlit("out"); reveal_strlit(".d"); reveal_strlit("root");
        reveal_strlit("--emit=link=out,dep-info=out.d");
        assert("--emit=link=out,dep-info=out.d"@ =~=
            "--emit=link="@ + "out"@ + ",dep-info="@ + "out"@ + ".d"@);
    }
    vec!["--crate-type=lib".to_string(), "--no-cheating".to_string(),
        "--no-external-by-default".to_string(), "--num-threads".to_string(),
        "2".to_string(), "--output-json".to_string(), "--compile".to_string(),
        "--emit=link=out,dep-info=out.d".to_string(), "root".to_string(),
        "-o".to_string(), "out".to_string(), "-Zchecksum-hash-algorithm=sha256".to_string()]
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ordinary_operations_do_not_decode_their_business_output() {
    let raw = [0u8, 255, 17];
    let ignored = policy::proof_stdout(&None, &raw);
    expect_verifier_true(ignored.len() == 0);
    let ordinary = policy::admit_output(&None, &None, &Vec::new(), i32::MIN, true);
    expect_verifier_true(ordinary.is_ok());
    let selected = policy::proof_stdout(&contract(), &raw);
    assert(selected@ == raw@);
    expect_verifier_bytes(&selected, &raw);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn full_crate_zero_error_report_and_successful_process_are_accepted() {
    let accepted = policy::admit_output(&contract(), &report(verification()),
        &arguments(), 0, false);
    expect_verifier_true(accepted.is_ok());
    let mut maximum = verification();
    maximum.verified = u64::MAX;
    let at_maximum = policy::admit_output(&contract(), &report(maximum),
        &arguments(), 0, false);
    expect_verifier_true(at_maximum.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_report_or_failed_process_cannot_be_success() {
    let missing = policy::admit_output(&contract(), &None, &arguments(), 0, false);
    expect_verifier_true(missing.is_err());
    let nonzero = policy::admit_output(&contract(), &report(verification()), &arguments(), 1, false);
    expect_verifier_true(nonzero.is_err());
    let negative = policy::admit_output(&contract(), &report(verification()), &arguments(), i32::MIN, false);
    expect_verifier_true(negative.is_err());
    let failed = policy::admit_output(&contract(), &report(verification()), &arguments(), 0, true);
    expect_verifier_true(failed.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn each_compiler_failure_flag_and_count_independently_rejects_the_report() {
    let mut unsuccessful = verification(); unsuccessful.success = false;
    let a = policy::admit_output(&contract(), &report(unsuccessful), &arguments(), 0, false);
    expect_verifier_true(a.is_err());
    let mut error = verification(); error.encountered_error = true;
    let b = policy::admit_output(&contract(), &report(error), &arguments(), 0, false);
    expect_verifier_true(b.is_err());
    let mut vir_error = verification(); vir_error.encountered_vir_error = true;
    let c = policy::admit_output(&contract(), &report(vir_error), &arguments(), 0, false);
    expect_verifier_true(c.is_err());
    let mut errors = verification(); errors.errors = 1;
    let d = policy::admit_output(&contract(), &report(errors), &arguments(), 0, false);
    expect_verifier_true(d.is_err());
    let mut empty = verification(); empty.verified = 0;
    let e = policy::admit_output(&contract(), &report(empty), &arguments(), 0, false);
    expect_verifier_true(e.is_err());
    let mut partial = verification(); partial.is_verifying_entire_crate = false;
    let f = policy::admit_output(&contract(), &report(partial), &arguments(), 0, false);
    expect_verifier_true(f.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn version_and_commit_must_match_one_explicit_nonempty_contract() {
    proof {
        reveal_strlit("v"); reveal_strlit("c"); reveal_strlit("");
        assert("v"@[0] != "c"@[0]);
    }
    let wrong_version = Some(OutputContract { version: "c".to_string(), commit: "c".to_string() });
    let a = policy::admit_output(&wrong_version, &report(verification()), &arguments(), 0, false);
    expect_verifier_true(a.is_err());
    let wrong_commit = Some(OutputContract { version: "v".to_string(), commit: "v".to_string() });
    let b = policy::admit_output(&wrong_commit, &report(verification()), &arguments(), 0, false);
    expect_verifier_true(b.is_err());
    let empty_version = Some(OutputContract { version: "".to_string(), commit: "c".to_string() });
    let c = policy::validate_contract(&empty_version); expect_verifier_true(c.is_err());
    let empty_commit = Some(OutputContract { version: "v".to_string(), commit: "".to_string() });
    let d = policy::validate_contract(&empty_commit); expect_verifier_true(d.is_err());
    let original = OutputContract { version: "v".to_string(), commit: "c".to_string() };
    let cloned = original.clone();
    let preserved = cloned.eq(&original); expect_verifier_true(preserved);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn extra_filtered_or_rebound_verifier_arguments_are_rejected() {
    proof {
        reveal_strlit("--compile"); reveal_strlit("--verify-function=one");
        reveal_strlit("out"); reveal_strlit("other");
        reveal_strlit("--no-cheating"); reveal_strlit("--no-verify");
        reveal_strlit("--emit=link="); reveal_strlit(",dep-info="); reveal_strlit(".d");
        reveal_strlit("--emit=link=out,dep-info=out.d");
        assert("--compile"@.len() == 9 && "--verify-function=one"@.len() == 21);
        assert("--no-cheating"@.len() == 13 && "--no-verify"@.len() == 11);
        assert("out"@.len() == 3 && "other"@.len() == 5);
        assert("--emit=link="@.len() == 12 && ",dep-info="@.len() == 10 && ".d"@.len() == 2);
        assert("--emit=link=out,dep-info=out.d"@.len() == 30);
        assert("--compile"@ != "--verify-function=one"@);
        assert("--no-cheating"@ != "--no-verify"@);
    }
    let mut filtered = arguments(); filtered[6] = "--verify-function=one".to_string();
    assert(filtered[6]@ != "--compile"@);
    let a = policy::admit_output(&contract(), &report(verification()), &filtered, 0, false);
    expect_verifier_true(a.is_err());
    let mut extra = arguments(); extra.push("--verify-function=one".to_string());
    let b = policy::canonical_verifier_arguments(&extra); expect_verifier_true(!b);
    let mut unchecked = arguments(); unchecked[1] = "--no-verify".to_string();
    assert(unchecked[1]@ != "--no-cheating"@);
    let c = policy::canonical_verifier_arguments(&unchecked); expect_verifier_true(!c);
    let mut rebound = arguments(); rebound[10] = "other".to_string();
    assert(rebound[7]@.len() == 30 && rebound[10]@.len() == 5);
    let d = policy::canonical_verifier_arguments(&rebound); expect_verifier_true(!d);
    let missing = Vec::new();
    let e = policy::canonical_verifier_arguments(&missing); expect_verifier_true(!e);
}

struct UnavailableSources;

fn library(length: u64) -> (out: policy::CompiledArtifact)
    ensures out.length == length,
        out.path@ == vstd::utf8::encode_utf8("out"@),
        policy::identifiers::digest_format(out.sha256@),
{
    let path = "out".as_bytes();
    let original = metadata_fixture(policy::sources::copy_range(path, 0, path.len()), length);
    policy::CompiledArtifact { path: original.path, sha256: original.sha256, length }
}

/// 任意观察（包括尚未准入的空值、Unicode、NUL 或无效摘要）都原样复制。
fn any_library_clone_preserves_every_field(artifact: &policy::CompiledArtifact) {
    let copied = artifact.clone();
    assert(copied.path@ == artifact.path@);
    assert(copied.sha256@ == artifact.sha256@);
    assert(copied.length == artifact.length);
}

impl policy::ArtifactRead for UnavailableSources {
    fn identity(&self, path: &[u8]) -> Result<(Vec<u8>, u64), String> {
        Err("original library read error\0é".to_string())
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn library_admission_requires_original_path_nonempty_length_and_digest() {
    proof { reveal_strlit("out"); }
    let args = arguments();
    let good = policy::admit_artifact_observation(&contract(), &Some(Ok(library(1))), &args);
    let large = policy::admit_artifact_observation(&contract(), &Some(Ok(library(u64::MAX))), &args);
    let empty = policy::admit_artifact_observation(&contract(), &Some(Ok(library(0))), &args);
    let missing = policy::admit_artifact_observation(&contract(), &None, &args);
    let failed = policy::admit_artifact_observation(
        &contract(), &Some(Err("original error".to_string())), &args,
    );
    expect_verifier_true(good.is_ok() && large.is_ok());
    expect_verifier_true(empty.is_err() && missing.is_err() && failed.is_err());
    let mut wrong_path = library(1); wrong_path.path.push(0u8);
    assert(wrong_path.path@.len() != vstd::utf8::encode_utf8("out"@).len());
    let rebound = policy::admit_artifact_observation(&contract(), &Some(Ok(wrong_path)), &args);
    expect_verifier_true(rebound.is_err());
    let mut bad_digest = library(1); bad_digest.sha256 = vec![48u8];
    let short = policy::admit_artifact_observation(&contract(), &Some(Ok(bad_digest)), &args);
    expect_verifier_true(short.is_err());
    let mut invalid = library(1); invalid.sha256[17] = 65u8;
    expect_verifier_true(invalid.sha256[17] == 65u8);
    assert(!policy::identifiers::digest_format(invalid.sha256@));
    let uppercase = policy::admit_artifact_observation(&contract(), &Some(Ok(invalid)), &args);
    expect_verifier_true(uppercase.is_err());
    let no_args = policy::admit_artifact_observation(&contract(), &Some(Ok(library(1))), &Vec::new());
    let ordinary = policy::admit_artifact_observation(&None, &None, &Vec::new());
    let extra = policy::admit_artifact_observation(&None, &Some(Ok(library(1))), &args);
    expect_verifier_true(no_args.is_err() && ordinary.is_ok() && extra.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn failed_process_or_invalid_arguments_cannot_create_successful_library_observation() {
    let io = UnavailableSources;
    let ordinary = policy::observe_artifact(&None, &Vec::new(), 0, false, &io);
    let failed = policy::observe_artifact(&contract(), &arguments(), 7, false, &io);
    let interrupted = policy::observe_artifact(&contract(), &arguments(), 0, true, &io);
    let invalid = policy::observe_artifact(&contract(), &Vec::new(), 0, false, &io);
    expect_verifier_true(ordinary.is_none());
    expect_verifier_true(failed.is_some() && failed.unwrap().is_err());
    expect_verifier_true(interrupted.is_some() && interrupted.unwrap().is_err());
    expect_verifier_true(invalid.is_some() && invalid.unwrap().is_err());
}

fn any_accepted_compiler_receipt_has_the_same_library(
    contract: &Option<OutputContract>, report: &Option<CompilerReport>, args: &Vec<String>,
    exit: i32, failed: bool, dependency: &Option<Result<Vec<u8>, String>>, limit: usize,
    artifact: &Option<Result<policy::CompiledArtifact, String>>,
)
    requires contract.is_some(),
{
    let accepted = policy::admit_observed_output(contract, report, args, exit, failed,
        dependency, limit, artifact);
    if accepted.is_ok() {
        assert(exit == 0 && !failed);
        assert(artifact.is_some() && artifact.unwrap().is_ok());
        assert(artifact.unwrap().unwrap().path@ == vstd::utf8::encode_utf8(args[10]@));
        assert(artifact.unwrap().unwrap().length > 0);
        assert(policy::identifiers::digest_format(artifact.unwrap().unwrap().sha256@));
    }
}

/// 成功报告与 dep-info 不能替代实际编译库；当前尚无库观察。
#[cfg_attr(not(verus_keep_ghost), test)]
fn successful_report_without_compiled_library_is_rejected() {
    let original = Some(Ok(vec![0u8, 255u8]));
    let accepted = policy::admit_observed_output(
        &contract(), &report(verification()), &arguments(), 0, false, &original, 2, &None,
    );
    expect_verifier_true(accepted.is_err());
}

impl policy::sources::DependencyRead for UnavailableSources {
    fn read(&self, path: &[u8], limit: u64) -> Result<Vec<u8>, String> {
        Err("original read error\0é".to_string())
    }
}

impl policy::sources::CandidateSourceRead for UnavailableSources {
    fn text(&self, path: &[u8]) -> Result<String, String> { Err("decode".to_string()) }
    fn mode(&self, path: &str) -> Option<String> { None }
    fn blob(&self, path: &str) -> Result<Vec<u8>, String> { Err("blob".to_string()) }
    fn digest(&self, bytes: &[u8]) -> Vec<u8> { Vec::new() }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_success_report_cannot_bypass_missing_failed_or_oversize_sources() {
    let args = arguments();
    let report = report(verification());
    let missing = policy::admit_observed_output(&contract(), &report, &args, 0, false, &None, 1, &Some(Ok(library(1))));
    expect_verifier_true(missing.is_err());
    let failure = Some(Err("original read error\0é".to_string()));
    let failed = policy::admit_observed_output(&contract(), &report, &args, 0, false, &failure, 1, &Some(Ok(library(1))));
    expect_verifier_true(failed.is_err());
    let original = Some(Ok(vec![0u8, 255u8]));
    let too_large = policy::admit_observed_output(&contract(), &report, &args, 0, false, &original, 1, &Some(Ok(library(1))));
    let exact = policy::admit_observed_output(&contract(), &report, &args, 0, false, &original, 2, &Some(Ok(library(1))));
    let zero_bound = policy::admit_observed_output(&contract(), &report, &args, 0, false, &original, 0, &Some(Ok(library(1))));
    let over_bound = policy::admit_observed_output(&contract(), &report, &args, 0, false, &original, 67_108_865, &Some(Ok(library(1))));
    expect_verifier_true(too_large.is_err() && exact.is_ok() && zero_bound.is_err() && over_bound.is_err());
    let ordinary = policy::admit_observed_output(&None, &None, &Vec::new(), 0, false, &None, 0, &None);
    let unexpected = policy::admit_observed_output(&None, &None, &Vec::new(), 0, false, &original, 2, &None);
    expect_verifier_true(ordinary.is_ok() && unexpected.is_err());
}

/// 与源码绑定组合：声明证明合同的成功回执一定带有同一原观察，不能走 None 快径。
fn any_admitted_proof_receipt_has_original_dependency(
    contract: &Option<OutputContract>, report: &Option<CompilerReport>, args: &Vec<String>,
    exit: i32, failed: bool, original: &Option<Result<Vec<u8>, String>>, limit: usize,
)
    requires contract.is_some(),
{
    let admitted = policy::admit_observed_output(contract, report, args, exit, failed, original, limit, &Some(Ok(library(1))));
    if admitted.is_ok() {
        assert(original.is_some());
        assert(original.unwrap().is_ok());
        assert(original.unwrap().unwrap().len() <= limit);
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observation_cannot_turn_failed_process_or_invalid_bound_into_source_evidence() {
    let io = UnavailableSources;
    let ordinary = policy::sources::observe_dependency(&None, &Vec::new(), 0, false, 1, &io);
    expect_verifier_true(ordinary.is_none());
    let nonzero = policy::sources::observe_dependency(&contract(), &arguments(), 7, false, 1, &io);
    let process_failure = policy::sources::observe_dependency(&contract(), &arguments(), 0, true, 1, &io);
    let no_bound = policy::sources::observe_dependency(&contract(), &arguments(), 0, false, 0, &io);
    let huge_bound = policy::sources::observe_dependency(&contract(), &arguments(), 0, false, usize::MAX, &io);
    expect_verifier_true(nonzero.is_some() && nonzero.unwrap().is_err());
    expect_verifier_true(process_failure.is_some() && process_failure.unwrap().is_err());
    expect_verifier_true(no_bound.is_some() && no_bound.unwrap().is_err());
    expect_verifier_true(huge_bound.is_some() && huge_bound.unwrap().is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_argv_keeps_every_argument_and_empty_original_rejects() {
    let original = vec!["verus\0é".to_string(), "root".to_string(), "root".to_string()];
    let args = policy::sources::process_arguments(&original).unwrap();
    assert(args.len() == 2 && args[0]@ == "root"@ && args[1]@ == "root"@);
    expect_verifier_true(args.len() == 2);
    let first = identifiers::equal_text(&args[0], "root");
    let second = identifiers::equal_text(&args[1], "root");
    expect_verifier_true(first && second);
    let empty = policy::sources::process_arguments(&Vec::new());
    expect_verifier_true(empty.is_err());
}

/// 缺方法、歧义和首个完整文本命中分别定义，不接受前缀或大小写折叠。
#[cfg_attr(not(verus_keep_ghost), test)]
fn proof_invocation_requires_one_exact_registered_operation() {
    let args = arguments();
    let ordinary = None;
    let proof_contract = contract();
    let methods = vec![
        policy::ProofInvocation { operation: "proof", argv: &args,
            output_contract: &proof_contract, output_limit_bytes: 1024 },
        policy::ProofInvocation { operation: "Proof", argv: &args,
            output_contract: &ordinary, output_limit_bytes: 1024 },
    ];
    proof {
        reveal_strlit("proof"); reveal_strlit("Proof"); reveal_strlit("proof\0");
        assert("proof"@[0] != "Proof"@[0]);
        assert("proof"@.len() != "proof\0"@.len());
        assert forall|j: int| 0 <= j < methods.len()
            && (#[trigger] methods[j]).operation@ == "proof"@ implies j == 0 by {
            if j != 0 { assert(j == 1); }
        }
        assert forall|j: int| 0 <= j < methods.len()
            && (#[trigger] methods[j]).operation@ == "Proof"@ implies j == 1 by {
            if j != 1 { assert(j == 0); }
        }
        assert(policy::unique_proof_invocation(methods@, "proof"@, 0));
        assert(exists|j: int| policy::unique_proof_invocation(methods@, "proof"@, j));
        assert(policy::unique_proof_invocation(methods@, "Proof"@, 1));
        assert(exists|j: int| policy::unique_proof_invocation(methods@, "Proof"@, j));
    }
    let exact = policy::find_proof_invocation(&methods, "proof");
    expect_verifier_true(exact.is_ok() && exact.unwrap() == 0);
    let other_case = policy::find_proof_invocation(&methods, "Proof");
    expect_verifier_true(other_case.is_ok() && other_case.unwrap() == 1);
    let suffix = policy::find_proof_invocation(&methods, "proof\0");
    expect_verifier_true(suffix.is_err());
    let duplicates = vec![
        policy::ProofInvocation { operation: "proof", argv: &args,
            output_contract: &proof_contract, output_limit_bytes: 1024 },
        policy::ProofInvocation { operation: "proof", argv: &args,
            output_contract: &ordinary, output_limit_bytes: 1024 },
    ];
    let ambiguous = policy::find_proof_invocation(&duplicates, "proof");
    expect_verifier_true(ambiguous.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_arguments_preserve_order_duplicates_and_full_text() {
    let first = vec!["a\0é".to_string(), "a".to_string(), "a".to_string()];
    let same = vec!["a\0é".to_string(), "a".to_string(), "a".to_string()];
    let reordered = vec!["a".to_string(), "a\0é".to_string(), "a".to_string()];
    let shortened = vec!["a\0é".to_string(), "a".to_string()];
    proof {
        reveal_strlit("a\0é"); reveal_strlit("a");
        assert("a\0é"@.len() != "a"@.len());
    }
    let exact = policy::equal_arguments(&first, &same);
    let order = policy::equal_arguments(&first, &reordered);
    let count = policy::equal_arguments(&first, &shortened);
    expect_verifier_true(exact && !order && !count);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unified_admission_does_not_invent_a_proof_for_blocked_or_missing_receipts() {
    let registry = Err("original registry error".to_string());
    let methods = Vec::new();
    let blocked = policy::bind_operation_output("blocked", "proof", &methods,
        None, &registry, &UnavailableSources, |message: &str, _path: &str| message.to_string());
    let failed = policy::bind_operation_output("failed", "proof", &methods,
        None, &registry, &UnavailableSources, |message: &str, _path: &str| message.to_string());
    proof {
        reveal_strlit("blocked"); reveal_strlit("failed"); reveal_strlit("passed");
        assert("blocked"@[0] != "passed"@[0]); assert("failed"@[0] != "passed"@[0]);
    }
    expect_verifier_true(blocked.is_ok() && blocked.unwrap().is_none());
    expect_verifier_true(failed.is_ok() && failed.unwrap().is_none());
    let missing = policy::bind_operation_output("passed", "proof", &methods,
        None, &registry, &UnavailableSources, |message: &str, _path: &str| message.to_string());
    expect_verifier_true(missing.is_err());
}

/// 普通操作的成功不能产生证明条目；其 stdout 保持业务字节，不要求解析。
#[cfg_attr(not(verus_keep_ghost), test)]
fn unified_ordinary_success_has_no_proof_and_failed_process_is_rejected() {
    let args = Vec::new();
    let contract = None;
    let methods = vec![policy::ProofInvocation { operation: "ordinary", argv: &args,
        output_contract: &contract, output_limit_bytes: 1 }];
    let argv = vec!["tool".to_string()];
    let dependency = None;
    let artifact = None;
    let registry = Err("registry not required".to_string());
    let raw = [255u8, 0u8];
    let original = Some(policy::ProofProcess { operation: "ordinary", argv: &argv, stdout: &raw,
        exit: 0, failed: false, dependency: &dependency, artifact: &artifact });
    proof {
        assert(policy::same_arguments(argv@.skip(1), args@));
        assert(policy::unique_proof_invocation(methods@, "ordinary"@, 0));
        assert(policy::ordinary_process_bound(&methods[0], &original.unwrap()));
        assert(policy::ordinary_output_accepted("ordinary"@, methods@, &original));
    }
    let ordinary = policy::bind_operation_output("passed", "ordinary", &methods,
        original, &registry, &UnavailableSources, |message: &str, _path: &str| message.to_string());
    expect_verifier_true(ordinary.is_ok() && ordinary.unwrap().is_none());
    let failed = policy::bind_operation_output("passed", "ordinary", &methods,
        Some(policy::ProofProcess { operation: "ordinary", argv: &argv, stdout: &raw,
            exit: 0, failed: true, dependency: &dependency, artifact: &artifact }),
        &registry, &UnavailableSources, |message: &str, _path: &str| message.to_string());
    expect_verifier_true(failed.is_err());
}

impl policy::CompilerReportRead for UnavailableSources {
    open spec fn decoded(&self, bytes: Seq<u8>) -> Option<CompilerReport> { None }
    fn decode(&self, bytes: &[u8]) -> Option<CompilerReport> { None }
}

/// 对任意原件，任何实际返回的证明都包含同一份报告、源码和库的准入关系。
fn any_unified_proof_has_a_unique_method_and_complete_original_evidence<I>(
    operation: &str, methods: &Vec<policy::ProofInvocation>,
    original: Option<policy::ProofProcess>,
    registry: &Result<policy::sources::SourceRegistry, String>, io: &I,
)
    where I: policy::sources::CandidateSourceRead + policy::CompilerReportRead,
{
    let admitted = policy::bind_operation_output("passed", operation, methods, original, registry, io, |message: &str, _path: &str| message.to_string());
    if let Ok(Some(proof)) = admitted {
        let ghost index = choose|i: int|
            policy::unique_proof_invocation(methods@, operation@, i)
                && policy::proof_bound(&proof, &methods[i], &original.unwrap(), &registry.unwrap());
        assert(policy::proof_bound(&proof, &methods[index], &original.unwrap(), &registry.unwrap()));
        assert(registry.is_ok());
        assert(original.is_some());
        assert(original.unwrap().exit == 0 && !original.unwrap().failed);
        assert(original.unwrap().dependency.is_some());
        assert(original.unwrap().artifact.is_some());
        assert(proof.report.verification.success);
        assert(proof.report.verification.errors == 0);
        assert(proof.report.verification.verified > 0);
        assert(proof.report.verification.is_verifying_entire_crate);
        assert(proof.sources.candidate.len() > 0);
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unified_proof_rejects_missing_report_and_different_original_identity() {
    let args = arguments();
    let proof_contract = contract();
    let methods = vec![policy::ProofInvocation { operation: "proof", argv: &args,
        output_contract: &proof_contract, output_limit_bytes: 1024 }];
    let mut argv = vec!["verus".to_string()];
    let mut i = 0;
    while i < args.len()
        invariant 0 <= i <= args.len(), argv.len() == i + 1,
            forall|j: int| 0 <= j < i ==> (#[trigger] argv[j + 1])@ == args[j]@,
        decreases args.len() - i,
    { argv.push(args[i].clone()); i += 1; }
    let dependency = Some(Ok(vec![0u8]));
    let artifact = Some(Ok(library(1)));
    let registry = Err("original registry error".to_string());
    let missing_report = policy::bind_operation_output("passed", "proof", &methods,
        Some(policy::ProofProcess { operation: "proof", argv: &argv, stdout: &[],
            exit: 0, failed: false, dependency: &dependency, artifact: &artifact }),
        &registry, &UnavailableSources, |message: &str, _path: &str| message.to_string());
    expect_verifier_true(missing_report.is_err());
    let wrong_operation = policy::bind_operation_output("passed", "proof", &methods,
        Some(policy::ProofProcess { operation: "Proof", argv: &argv, stdout: &[],
            exit: 0, failed: false, dependency: &dependency, artifact: &artifact }),
        &registry, &UnavailableSources, |message: &str, _path: &str| message.to_string());
    expect_verifier_true(wrong_operation.is_err());
    let wrong_argv = vec!["verus".to_string()];
    let wrong_arguments = policy::bind_operation_output("passed", "proof", &methods,
        Some(policy::ProofProcess { operation: "proof", argv: &wrong_argv, stdout: &[],
            exit: 0, failed: false, dependency: &dependency, artifact: &artifact }),
        &registry, &UnavailableSources, |message: &str, _path: &str| message.to_string());
    expect_verifier_true(wrong_arguments.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn explicit_file_membership_uses_full_original_text() {
    proof {
        reveal_strlit("a"); reveal_strlit("A"); reveal_strlit("a\0");
        vstd::string::is_ascii_spec_bytes("a"); vstd::string::is_ascii_spec_bytes("A");
        vstd::string::is_ascii_spec_bytes("a\0");
    }
    let files = vec![policy::sources::SourceFile { path: "a".to_string(), verification: None }];
    assert(files.len() == 1 && files[0].path@ == "a"@);
    assert(vstd::utf8::encode_utf8(files[0].path@) =~= seq![97u8]);
    assert(policy::sources::registered_source(files@, seq![97u8]));
    let exact_input = [97u8];
    let wrong_input = [65u8];
    let suffix_input = [97u8, 0u8];
    assert(exact_input@ == seq![97u8]);
    assert(wrong_input@ == seq![65u8]);
    assert(suffix_input@ == seq![97u8, 0u8]);
    let exact = policy::sources::source_is_registered(&files, &exact_input);
    let wrong_case = policy::sources::source_is_registered(&files, &wrong_input);
    let suffix = policy::sources::source_is_registered(&files, &suffix_input);
    assert forall|i: int| 0 <= i < files.len() implies
        vstd::utf8::encode_utf8((#[trigger] files[i]).path@) != seq![65u8] by {
        assert(i == 0); assert(seq![97u8][0] != seq![65u8][0]);
        assert(seq![97u8] != seq![65u8]);
    }
    assert forall|i: int| 0 <= i < files.len() implies
        vstd::utf8::encode_utf8((#[trigger] files[i]).path@) != seq![97u8, 0u8] by {
        assert(i == 0); assert(seq![97u8].len() != seq![97u8, 0u8].len());
    }
    assert(!policy::sources::registered_source(files@, seq![65u8]));
    assert(!policy::sources::registered_source(files@, seq![97u8, 0u8]));
    expect_verifier_true(exact);
    expect_verifier_true(!wrong_case);
    expect_verifier_true(!suffix);
}


fn declared_source_root(path: &str, operation: &str) -> (root: policy::sources::SourceProofRoot)
    ensures root.path@ == path@, root.operation@ == operation@,
{
    policy::sources::SourceProofRoot { path: path.to_string(), operation: operation.to_string() }
}

fn source_declaration(role: policy::sources::SourceRole, roots: Vec<policy::sources::SourceProofRoot>)
    -> (file: policy::sources::SourceFile)
    ensures file.path@ == "file"@, file.verification.is_some(),
        file.verification.unwrap().role == role,
        file.verification.unwrap().roots@ == roots@,
        forall|i: int| 0 <= i < roots.len() ==>
            file.verification.unwrap().roots[i].path@ == roots@[i].path@
                && file.verification.unwrap().roots[i].operation@ == roots@[i].operation@,
{
    policy::sources::SourceFile { path: "file".to_string(),
        verification: Some(policy::sources::SourceVerification { role, roots }) }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn explicit_program_specification_and_data_have_distinct_root_obligations() {
    proof { reveal(policy::sources::source_declaration_valid); }
    let unknown = policy::sources::SourceFile { path: "file".to_string(), verification: None };
    expect_verifier_true(unknown.verification.is_none());
    let absent = policy::sources::validate_source_declaration(&unknown);
    expect_verifier_true(absent.is_ok());
    let data = source_declaration(SourceRole::Data, Vec::new());
    let data_result = policy::sources::validate_source_declaration(&data);
    expect_verifier_true(data_result.is_ok());
    let program = source_declaration(SourceRole::Program, Vec::new());
    let program_result = policy::sources::validate_source_declaration(&program);
    expect_verifier_true(program_result.is_err());
    let specification = source_declaration(SourceRole::Specification, Vec::new());
    let specification_result = policy::sources::validate_source_declaration(&specification);
    expect_verifier_true(specification_result.is_err());
    let data_with_root = source_declaration(SourceRole::Data,
        vec![declared_source_root("r", "verify.r")]);
    let invalid_data = policy::sources::validate_source_declaration(&data_with_root);
    expect_verifier_true(invalid_data.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn declared_roots_require_canonical_paths_and_nonempty_operation_text() {
    proof {
        reveal(policy::sources::source_declaration_valid);
        reveal_strlit(""); reveal_strlit("r"); reveal_strlit("verify.r"); reveal_strlit("./r");
        vstd::string::is_ascii_spec_bytes("r"); vstd::string::is_ascii_spec_bytes("./r");
        reveal_with_fuel(policy::sources::path_scan, 8);
        reveal_with_fuel(policy::sources::joined_components, 4);
    }
    let program = source_declaration(SourceRole::Program,
        vec![declared_source_root("r", "verify.r")]);
    assert(policy::sources::canonical_source_path(vstd::utf8::encode_utf8("r"@)));
    let valid_program = policy::sources::validate_source_declaration(&program);
    expect_verifier_true(valid_program.is_ok());
    let specification = source_declaration(SourceRole::Specification,
        vec![declared_source_root("r", "verify.r")]);
    let valid_specification = policy::sources::validate_source_declaration(&specification);
    expect_verifier_true(valid_specification.is_ok());
    let noncanonical = source_declaration(SourceRole::Program,
        vec![declared_source_root("./r", "verify.r")]);
    assert(!policy::sources::canonical_source_path(vstd::utf8::encode_utf8("./r"@)));
    let rejected_path = policy::sources::validate_source_declaration(&noncanonical);
    expect_verifier_true(rejected_path.is_err());
    let empty_operation = source_declaration(SourceRole::Program,
        vec![declared_source_root("r", "")]);
    assert(empty_operation.verification.unwrap().roots@[0].operation@.len() == 0);
    let rejected_operation = policy::sources::validate_source_declaration(&empty_operation);
    expect_verifier_true(rejected_operation.is_err());
    let empty_path = source_declaration(SourceRole::Program,
        vec![declared_source_root("", "verify.r")]);
    assert(!policy::sources::canonical_source_path(vstd::utf8::encode_utf8(""@)));
    let rejected_empty_path = policy::sources::validate_source_declaration(&empty_path);
    expect_verifier_true(rejected_empty_path.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn duplicate_root_paths_are_rejected_even_with_different_operations() {
    proof {
        reveal(policy::sources::source_declaration_valid);
        reveal(policy::sources::distinct_proof_root);
        reveal_strlit("r"); reveal_strlit("R"); reveal_strlit("verify.r"); reveal_strlit("verify.R");
        vstd::string::is_ascii_spec_bytes("r"); vstd::string::is_ascii_spec_bytes("R");
        reveal_with_fuel(policy::sources::path_scan, 8);
        reveal_with_fuel(policy::sources::joined_components, 4);
    }
    let duplicate = source_declaration(SourceRole::Program,
        vec![declared_source_root("r", "verify.r"), declared_source_root("r", "other")]);
    assert(!policy::sources::distinct_proof_root(duplicate.verification.unwrap().roots@, 1));
    let rejected_duplicate = policy::sources::validate_source_declaration(&duplicate);
    expect_verifier_true(rejected_duplicate.is_err());
    let different_case = source_declaration(SourceRole::Program,
        vec![declared_source_root("r", "verify.r"), declared_source_root("R", "verify.R")]);
    assert("r"@[0] != "R"@[0]);
    assert("r"@ != "R"@);
    assert(policy::sources::distinct_proof_root(different_case.verification.unwrap().roots@, 1));
    let distinct = policy::sources::validate_source_declaration(&different_case);
    expect_verifier_true(distinct.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn proof_roots_must_name_registered_execution_operations() {
    proof {
        reveal(policy::sources::proof_roots_use_registered_operations);
        reveal(policy::sources::proof_root_operation_registered);
        reveal_strlit(""); reveal_strlit("other"); reveal_strlit("verify.r");
    }
    let valid = source_declaration(SourceRole::Program,
        vec![declared_source_root("r", "verify.r")]);
    let operations = vec!["other".to_string(), "verify.r".to_string()];
    let accepted = policy::sources::proof_roots_have_registered_operations(&valid, &operations);
    expect_verifier_true(accepted);
    let missing = vec!["other".to_string()];
    assert(valid.verification.unwrap().roots@[0].operation@ == "verify.r"@);
    assert(missing@[0]@ == "other"@);
    assert(missing@[0]@ != valid.verification.unwrap().roots@[0].operation@);
    let rejected = policy::sources::proof_roots_have_registered_operations(&valid, &missing);
    expect_verifier_true(!rejected);
    let unknown = policy::sources::SourceFile { path: "file".to_string(), verification: None };
    expect_verifier_true(policy::sources::proof_roots_have_registered_operations(&unknown, &missing));
}

fn planned_source() -> (file: policy::sources::SourceFile)
    ensures file.path@ == "file"@, file.verification.is_some(),
        file.verification.unwrap().roots.len() == 1,
        file.verification.unwrap().roots[0].path@ == "root"@,
        file.verification.unwrap().roots[0].operation@ == "proof"@,
        policy::sources::source_declaration_valid(&file),
{
    proof {
        reveal_strlit("root"); reveal_strlit("proof");
        vstd::string::is_ascii_spec_bytes("root");
        reveal_with_fuel(policy::sources::path_scan, 8);
        reveal_with_fuel(policy::sources::joined_components, 4);
        assert("root"@ =~= seq!['r', 'o', 'o', 't']);
        vstd::utf8::is_ascii_chars_encode_utf8("root"@);
        assert(vstd::utf8::encode_utf8("root"@) =~= seq![114u8, 111u8, 111u8, 116u8]);
        assert(policy::sources::canonical_source_path(seq![114u8, 111u8, 111u8, 116u8])) by (compute);
        assert(policy::sources::canonical_source_path(vstd::utf8::encode_utf8("root"@)));
    }
    let file = source_declaration(SourceRole::Program, vec![declared_source_root("root", "proof")]);
    assert(policy::sources::proof_root_declaration_valid(&file.verification.unwrap().roots[0]));
    assert(policy::sources::distinct_proof_root(file.verification.unwrap().roots@, 0));
    file
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selected_plan_requires_every_affected_declared_proof() {
    proof {
        reveal_with_fuel(policy::sources::selected_plan_error, 3);
        reveal_with_fuel(policy::sources::file_plan_error, 3);
        reveal(policy::sources::file_affected);
        reveal(policy::sources::root_planned);
        reveal(policy::unique_proof_invocation);
        reveal_strlit("file:"); reveal_strlit("file"); reveal_strlit("file:file");
        assert("file:file"@ =~= "file:"@ + "file"@);
    }
    let files = vec![planned_source()];
    let affected = vec!["file:file".to_string()];
    let missing = policy::sources::validate_selected_roots(&files, &affected, &Vec::new());
    expect_verifier_true(missing.is_err());
    let args = arguments();
    let output = contract();
    let methods = vec![policy::ProofInvocation { operation: "proof", argv: &args,
        output_contract: &output, output_limit_bytes: 4096 }];
    assert(policy::unique_proof_invocation(methods@, "proof"@, 0));
    assert(policy::sources::root_planned(&files[0].verification.unwrap().roots[0], methods@));
    let admitted = policy::sources::validate_selected_roots(&files, &affected, &methods);
    expect_verifier_true(admitted.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selected_ordinary_operation_cannot_satisfy_a_declared_proof() {
    proof {
        reveal_with_fuel(policy::sources::selected_plan_error, 3);
        reveal_with_fuel(policy::sources::file_plan_error, 3);
        reveal(policy::sources::file_affected);
        reveal(policy::sources::root_planned);
        reveal(policy::unique_proof_invocation);
        reveal_strlit("file:"); reveal_strlit("file"); reveal_strlit("file:file");
        assert("file:file"@ =~= "file:"@ + "file"@);
    }
    let files = vec![planned_source()];
    let affected = vec!["file:file".to_string()];
    let args = arguments();
    let ordinary = None;
    let methods = vec![policy::ProofInvocation { operation: "proof", argv: &args,
        output_contract: &ordinary, output_limit_bytes: 4096 }];
    let rejected = policy::sources::validate_selected_roots(&files, &affected, &methods);
    expect_verifier_true(rejected.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selected_proof_rejects_duplicate_methods_wrong_roots_and_noncanonical_arguments() {
    proof {
        reveal_with_fuel(policy::sources::selected_plan_error, 3);
        reveal_with_fuel(policy::sources::file_plan_error, 3);
        reveal(policy::sources::file_affected);
        reveal(policy::sources::root_planned);
        reveal(policy::unique_proof_invocation);
        reveal_strlit("file:"); reveal_strlit("file"); reveal_strlit("file:file");
        reveal_strlit("root"); reveal_strlit("Root");
        assert("file:file"@ =~= "file:"@ + "file"@);
        assert("root"@[0] != "Root"@[0]);
    }
    let files = vec![planned_source()];
    let affected = vec!["file:file".to_string()];
    let args = arguments();
    let output = contract();
    let duplicates = vec![
        policy::ProofInvocation { operation: "proof", argv: &args,
            output_contract: &output, output_limit_bytes: 4096 },
        policy::ProofInvocation { operation: "proof", argv: &args,
            output_contract: &output, output_limit_bytes: 4096 }];
    let ambiguous = policy::sources::validate_selected_roots(&files, &affected, &duplicates);
    expect_verifier_true(ambiguous.is_err());
    let mut other_root = arguments();
    other_root[8] = "Root".to_string();
    let wrong = vec![policy::ProofInvocation { operation: "proof", argv: &other_root,
        output_contract: &output, output_limit_bytes: 4096 }];
    let mismatch = policy::sources::validate_selected_roots(&files, &affected, &wrong);
    expect_verifier_true(mismatch.is_err());
    let invalid = Vec::new();
    let malformed = vec![policy::ProofInvocation { operation: "proof", argv: &invalid,
        output_contract: &output, output_limit_bytes: 4096 }];
    let rejected = policy::sources::validate_selected_roots(&files, &affected, &malformed);
    expect_verifier_true(rejected.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn proof_plan_preserves_explicit_scope_data_and_unknown_boundaries() {
    proof {
        reveal_with_fuel(policy::sources::selected_plan_error, 5);
        reveal_with_fuel(policy::sources::file_plan_error, 3);
        reveal(policy::sources::file_affected);
        reveal(policy::sources::source_declaration_valid);
        reveal_strlit("file:"); reveal_strlit("file"); reveal_strlit("file:file");
        reveal_strlit("file:other");
        assert("file:file"@ =~= "file:"@ + "file"@);
        assert("file:other"@ != "file:"@ + "file"@);
    }
    let files = vec![planned_source()];
    expect_verifier_true(policy::sources::can_validate_selected_plan(&Vec::new()));
    let blocked = vec!["missing required input".to_string()];
    expect_verifier_true(!policy::sources::can_validate_selected_plan(&blocked));
    let unrelated = vec!["file:other".to_string()];
    let unselected = policy::sources::validate_selected_roots(&files, &unrelated, &Vec::new());
    expect_verifier_true(unselected.is_ok());
    let unknown = policy::sources::SourceFile { path: "file".to_string(), verification: None };
    let data = source_declaration(SourceRole::Data, Vec::new());
    let inventory = vec![unknown, data];
    let affected = vec!["file:file".to_string()];
    let no_proof_claim = policy::sources::validate_selected_roots(&inventory, &affected, &Vec::new());
    expect_verifier_true(no_proof_claim.is_ok());
    let invalid = source_declaration(SourceRole::Program, Vec::new());
    let bad = vec![invalid];
    let rejected = policy::sources::validate_selected_roots(&bad, &affected, &Vec::new());
    expect_verifier_true(rejected.is_err());
}

fn metadata_fixture(path: Vec<u8>, length: u64) -> (source: policy::sources::SourceBinding)
    ensures source.path@ == path@, source.length == length,
        source.sha256.len() == 64,
        forall|i: int| 0 <= i < 64 ==> (#[trigger] source.sha256[i]) == 48u8,
{
    let mut hash = Vec::new();
    let mut i = 0;
    while i < 64
        invariant 0 <= i <= 64, hash.len() == i,
            forall|j: int| 0 <= j < i ==> (#[trigger] hash[j]) == 48u8,
        decreases 64 - i,
    { hash.push(48u8); i += 1; }
    policy::sources::SourceBinding { path, sha256: hash, length }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_metadata_requires_normalized_identity_digest_length_and_root() {
    reveal_with_fuel(policy::sources::path_scan, 8);
    reveal_with_fuel(policy::sources::joined_components, 3);
    let raw = metadata_fixture(vec![46u8, 47, 97], 7);
    let fixed = metadata_fixture(vec![97u8], 7);
    let sources = vec![raw];
    let candidate = vec![fixed];
    let root = [97u8];
    assert(sources[0].path@ == seq![46u8, 47u8, 97u8]);
    assert(candidate[0].path@ == seq![97u8]);
    assert(sources[0].sha256@ =~= candidate[0].sha256@);
    assert(policy::sources::source_pair_matches(&sources[0], &candidate[0]));
    assert(policy::sources::listed_source_path(candidate@, root@, 1));
    assert(!policy::sources::listed_source_path(candidate@, candidate[0].path@, 0));
    assert(policy::sources::source_metadata_admitted(sources@, candidate@, root@));
    let accepted = policy::sources::admit_source_metadata(&sources, &candidate, &root);
    expect_verifier_true(accepted);
    let changed_candidate = vec![metadata_fixture(vec![97u8], 8)];
    let changed_length = policy::sources::admit_source_metadata(&sources, &changed_candidate, &root);
    expect_verifier_true(!changed_length);
    let mut changed = metadata_fixture(vec![46u8, 47, 97], 7);
    changed.sha256[0] = 65u8;
    let changed_sources = vec![changed];
    expect_verifier_true(changed_sources[0].sha256[0] == 65u8);
    expect_verifier_true(candidate[0].sha256[0] == 48u8);
    assert(!policy::sources::source_pair_matches(&changed_sources[0], &candidate[0]));
    let changed_digest = policy::sources::admit_source_metadata(&changed_sources, &candidate, &root);
    expect_verifier_true(!changed_digest);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_metadata_rejects_missing_roots_and_empty_inventories() {
    reveal_with_fuel(policy::sources::path_scan, 4);
    reveal_with_fuel(policy::sources::joined_components, 3);
    let sources = vec![metadata_fixture(vec![97u8], 0)];
    let candidate = vec![metadata_fixture(vec![97u8], 0)];
    let missing_root = policy::sources::admit_source_metadata(&sources, &candidate, &[98u8]);
    expect_verifier_true(!missing_root);
    let empty: Vec<policy::sources::SourceBinding> = Vec::new();
    let no_sources = policy::sources::admit_source_metadata(&empty, &empty, &[97u8]);
    let unequal = policy::sources::admit_source_metadata(&sources, &empty, &[97u8]);
    expect_verifier_true(!no_sources && !unequal);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_metadata_accepts_consistent_path_aliases() {
    reveal_with_fuel(policy::sources::path_scan, 8);
    reveal_with_fuel(policy::sources::joined_components, 3);
    let sources = vec![metadata_fixture(vec![97u8], 7), metadata_fixture(vec![46u8, 47, 97], 7)];
    let candidate = vec![metadata_fixture(vec![97u8], 7), metadata_fixture(vec![97u8], 7)];
    assert(sources[0].path@ == seq![97u8]);
    assert(candidate[0].path@ == seq![97u8]);
    assert(sources[1].path@ == seq![46u8, 47u8, 97u8]);
    assert(candidate[1].path@ == seq![97u8]);
    assert(sources[0].sha256@ =~= candidate[0].sha256@);
    assert(sources[1].sha256@ =~= candidate[1].sha256@);
    assert(candidate[0].sha256@ =~= candidate[1].sha256@);
    assert(policy::sources::source_pair_matches(&sources[0], &candidate[0]));
    assert(policy::sources::source_pair_matches(&sources[1], &candidate[1]));
    assert(policy::sources::source_aliases_consistent(candidate@, 0));
    assert(policy::sources::source_aliases_consistent(candidate@, 1));
    let accepted = policy::sources::admit_source_metadata(&sources, &candidate, &[97u8]);
    expect_verifier_true(accepted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_metadata_rejects_alias_length_conflicts_even_when_each_pair_matches() {
    reveal_with_fuel(policy::sources::path_scan, 8);
    reveal_with_fuel(policy::sources::joined_components, 3);
    let sources = vec![metadata_fixture(vec![97u8], 7), metadata_fixture(vec![46u8, 47, 97], 8)];
    let candidate = vec![metadata_fixture(vec![97u8], 7), metadata_fixture(vec![97u8], 8)];
    assert(sources[0].path@ == seq![97u8]);
    assert(candidate[0].path@ == seq![97u8]);
    assert(sources[1].path@ == seq![46u8, 47u8, 97u8]);
    assert(candidate[1].path@ == seq![97u8]);
    assert(sources[0].sha256@ =~= candidate[0].sha256@);
    assert(sources[1].sha256@ =~= candidate[1].sha256@);
    assert(policy::sources::source_pair_matches(&sources[0], &candidate[0]));
    assert(policy::sources::source_pair_matches(&sources[1], &candidate[1]));
    assert(!policy::sources::source_aliases_consistent(candidate@, 1));
    let accepted = policy::sources::admit_source_metadata(&sources, &candidate, &[97u8]);
    expect_verifier_true(!accepted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_metadata_rejects_alias_digest_conflicts_even_when_each_pair_matches() {
    reveal_with_fuel(policy::sources::path_scan, 8);
    reveal_with_fuel(policy::sources::joined_components, 3);
    let mut raw = metadata_fixture(vec![46u8, 47, 97], 7);
    let mut fixed = metadata_fixture(vec![97u8], 7);
    raw.sha256[0] = 49u8;
    fixed.sha256[0] = 49u8;
    let sources = vec![metadata_fixture(vec![97u8], 7), raw];
    let candidate = vec![metadata_fixture(vec![97u8], 7), fixed];
    assert(sources[0].path@ == seq![97u8]);
    assert(candidate[0].path@ == seq![97u8]);
    assert(sources[1].path@ == seq![46u8, 47u8, 97u8]);
    assert(candidate[1].path@ == seq![97u8]);
    assert(sources[0].sha256@ =~= candidate[0].sha256@);
    assert(sources[1].sha256@ =~= candidate[1].sha256@);
    assert(policy::sources::source_pair_matches(&sources[0], &candidate[0]));
    assert(policy::sources::source_pair_matches(&sources[1], &candidate[1]));
    expect_verifier_true(candidate[0].sha256[0] == 48u8 && candidate[1].sha256[0] == 49u8);
    assert(!policy::sources::source_aliases_consistent(candidate@, 1));
    let accepted = policy::sources::admit_source_metadata(&sources, &candidate, &[97u8]);
    expect_verifier_true(!accepted);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn relative_source_paths_normalize_components_without_escaping_root() {
    let path = [97u8, 47, 98, 47, 46, 46, 47, 46, 46, 47, 99];
    assert(path@ =~= seq![97u8, 47u8, 98u8, 47u8, 46u8, 46u8, 47u8, 46u8, 46u8, 47u8, 99u8]);
    assert(policy::sources::relative_components(seq![97u8, 47u8, 98u8, 47u8, 46u8, 46u8, 47u8, 46u8, 46u8, 47u8, 99u8])
        == Some(seq![seq![99u8]])) by (compute);
    assert(policy::sources::joined_components(seq![seq![99u8]]) == seq![99u8]) by (compute);
    let normalized = policy::sources::normalize_relative_path(&path).unwrap();
    assert(normalized@ == seq![99u8]);
    let expected = [99u8];
    assert(expected@ == seq![99u8]);
    expect_verifier_bytes(&normalized, &expected);
    let dotted = [46u8, 47, 97, 47, 47, 46, 47, 98, 47];
    assert(dotted@ =~= seq![46u8, 47u8, 97u8, 47u8, 47u8, 46u8, 47u8, 98u8, 47u8]);
    assert(policy::sources::relative_components(seq![46u8, 47u8, 97u8, 47u8, 47u8, 46u8, 47u8, 98u8, 47u8])
        == Some(seq![seq![97u8], seq![98u8]])) by (compute);
    assert(policy::sources::joined_components(seq![seq![97u8], seq![98u8]]) == seq![97u8, 47u8, 98u8]) by (compute);
    let normalized = policy::sources::normalize_relative_path(&dotted).unwrap();
    assert(normalized@ == seq![97u8, 47u8, 98u8]);
    expect_verifier_bytes(&normalized, &[97u8, 47u8, 98u8]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn relative_source_paths_reject_absolute_empty_and_parent_escape() {
    assert(policy::sources::relative_components(seq![46u8]).is_none()) by (compute);
    assert(policy::sources::relative_components(seq![46u8, 46u8, 47u8, 97u8]).is_none()) by (compute);
    assert(policy::sources::relative_components(seq![97u8, 47u8, 46u8, 46u8]).is_none()) by (compute);
    assert(policy::sources::relative_components(seq![97u8, 47u8, 46u8, 46u8, 47u8, 46u8, 46u8, 47u8, 98u8]).is_none()) by (compute);
    let absolute_input = [47u8, 97];
    let empty_input: [u8; 0] = [];
    let dot_input = [46u8];
    let parent_input = [46u8, 46, 47, 97];
    let cancelled_input = [97u8, 47, 46, 46];
    let escape_input = [97u8, 47, 46, 46, 47, 46, 46, 47, 98];
    assert(absolute_input@ =~= seq![47u8, 97u8]);
    assert(empty_input@ =~= Seq::<u8>::empty());
    assert(dot_input@ =~= seq![46u8]);
    assert(parent_input@ =~= seq![46u8, 46u8, 47u8, 97u8]);
    assert(cancelled_input@ =~= seq![97u8, 47u8, 46u8, 46u8]);
    assert(escape_input@ =~= seq![97u8, 47u8, 46u8, 46u8, 47u8, 46u8, 46u8, 47u8, 98u8]);
    let absolute = policy::sources::normalize_relative_path(&absolute_input);
    let empty = policy::sources::normalize_relative_path(&empty_input);
    let dot = policy::sources::normalize_relative_path(&dot_input);
    let parent = policy::sources::normalize_relative_path(&parent_input);
    let cancelled = policy::sources::normalize_relative_path(&cancelled_input);
    let escape = policy::sources::normalize_relative_path(&escape_input);
    expect_verifier_true(absolute.is_none() && empty.is_none() && dot.is_none());
    expect_verifier_true(parent.is_none() && cancelled.is_none() && escape.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn relative_source_paths_preserve_spaces_unicode_and_reject_control_bytes() {
    let path = [97u8, 32, 47, 206, 187];
    assert(path@ =~= seq![97u8, 32u8, 47u8, 206u8, 187u8]);
    assert(policy::sources::relative_components(seq![97u8, 32u8, 47u8, 206u8, 187u8])
        == Some(seq![seq![97u8, 32u8], seq![206u8, 187u8]])) by (compute);
    assert(policy::sources::joined_components(seq![seq![97u8, 32u8], seq![206u8, 187u8]])
        == seq![97u8, 32u8, 47u8, 206u8, 187u8]) by (compute);
    let normalized = policy::sources::normalize_relative_path(&path).unwrap();
    assert(normalized@ == path@);
    expect_verifier_bytes(&normalized, &path);
    let nul = policy::sources::normalize_relative_path(&[97u8, 0]);
    let lf = policy::sources::normalize_relative_path(&[97u8, 10]);
    let del = policy::sources::normalize_relative_path(&[97u8, 127]);
    let backslash = policy::sources::normalize_relative_path(&[97u8, 92, 98]);
    assert(policy::sources::relative_components(seq![97u8, 0u8]).is_none()) by (compute);
    assert(policy::sources::relative_components(seq![97u8, 10u8]).is_none()) by (compute);
    assert(policy::sources::relative_components(seq![97u8, 127u8]).is_none()) by (compute);
    assert(policy::sources::relative_components(seq![97u8, 92u8, 98u8]).is_none()) by (compute);
    expect_verifier_true(nul.is_none() && lf.is_none() && del.is_none() && backslash.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn dependency_content_requires_every_checksum_and_final_newline() {
    proof {
        reveal_strlit("# checksum:sha256="); reveal_strlit(" file_len:");
        reveal_strlit("# env-dep:");
        assert(vstd::string::is_ascii("# checksum:sha256="));
        assert(vstd::string::is_ascii(" file_len:"));
        assert(vstd::string::is_ascii("# env-dep:"));
        vstd::string::is_ascii_spec_bytes("# checksum:sha256=");
        vstd::string::is_ascii_spec_bytes(" file_len:");
        vstd::string::is_ascii_spec_bytes("# env-dep:");
    }
    let input = [111u8, 46u8, 100u8, 58u8, 32u8, 97u8, 10u8, 10u8, 111u8, 58u8, 32u8, 97u8, 10u8, 10u8, 97u8, 58u8, 10u8, 35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 32u8, 97u8, 10u8];
    let output = [111u8];
    assert(input@ =~= seq![111u8, 46u8, 100u8, 58u8, 32u8, 97u8, 10u8, 10u8, 111u8, 58u8, 32u8, 97u8, 10u8, 10u8, 97u8, 58u8, 10u8, 35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 32u8, 97u8, 10u8]);
    assert(output@ =~= seq![111u8]);
    let source = policy::sources::SourceBinding {
        path: vec![97u8], sha256: vec![102u8], length: 1,
    };
    let mut sources = Vec::new();
    sources.push(source);
    expect_verifier_true(sources.len() == 1);
    assert(sources[0].path@ == seq![97u8]);
    assert(sources[0].sha256@ == seq![102u8]);
    let matched = policy::sources::matches_dependency_content(&input, &output, &sources);
    reveal_with_fuel(policy::sources::dependency_escape, 3);
    reveal_with_fuel(policy::sources::dependency_encoding, 3);
    reveal_with_fuel(policy::sources::phony_rules, 3);
    assert(policy::sources::source_paths(sources@) =~= seq![seq![97u8]]);
    assert(sources@.drop_last() =~= Seq::<policy::sources::SourceBinding>::empty());
    assert(policy::sources::dependency_encoding(policy::sources::source_paths(sources@)) =~= seq![97u8]);
    assert(policy::sources::phony_rules(sources@) =~= seq![97u8, 58u8, 10u8]);
    assert(policy::sources::dependency_header(output@, sources@) =~= input@.take(17));
    assert(input@.subrange(0, 17) =~= input@.take(17));
    assert(policy::sources::checksum_line(&sources[0]) =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 32u8, 97u8]);
    assert(input@.skip(17) =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 32u8, 97u8, 10u8]);
    proof { policy::sources::next_byte_at(input@.skip(17), 0, 32, 10u8); }
    reveal_with_fuel(policy::sources::checksum_trailer, 3);
    assert(input@.skip(17).skip(33) =~= Seq::<u8>::empty());
    assert(input@.skip(17).take(32) =~= policy::sources::checksum_line(&sources[0]));
    assert(input@.skip(17).take(32)[2] == 99u8);
    assert(vstd::utf8::encode_utf8("# env-dep:"@)[2] == 101u8);
    assert(!policy::sources::prefix_at(input@.skip(17).take(32), 0, vstd::utf8::encode_utf8("# env-dep:"@)));
    expect_verifier_true(matched);

    let missing = policy::sources::copy_range(&input, 0, 17);
    let missing_checksum = policy::sources::matches_dependency_content(missing.as_slice(), &output, &sources);
    assert(missing@.skip(17) =~= Seq::<u8>::empty());
    expect_verifier_true(!missing_checksum);

    let truncated = policy::sources::copy_range(&input, 0, 49);
    let no_final_lf = policy::sources::matches_dependency_content(truncated.as_slice(), &output, &sources);
    assert(truncated@.skip(17) =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 32u8, 97u8]);
    proof { policy::sources::next_byte_at(truncated@.skip(17), 0, 32, 10u8); }
    expect_verifier_true(!no_final_lf);

    let empty: Vec<policy::sources::SourceBinding> = Vec::new();
    let no_sources = policy::sources::matches_dependency_content(&input, &output, &empty);
    expect_verifier_true(!no_sources);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn dependency_target_requires_exact_output_and_terminated_nonempty_list() {
    let input = [111u8, 46u8, 100u8, 58u8, 32u8, 97u8, 32u8, 98u8, 10u8];
    let output = [111u8];
    assert(input@ =~= seq![111u8, 46u8, 100u8, 58u8, 32u8, 97u8, 32u8, 98u8, 10u8]);
    assert(output@ =~= seq![111u8]);
    let parsed = policy::sources::compiler_dependency_paths(&input, &output);
    reveal_with_fuel(policy::sources::next_byte, 5);
    reveal_with_fuel(policy::sources::dependency_scan, 5);
    assert(input@.subrange(0, 5) =~= policy::sources::target_prefix(output@));
    assert(input@.subrange(5, 8) =~= seq![97u8, 32u8, 98u8]);
    assert(policy::sources::dependency_scan(seq![97u8, 32u8, 98u8], Seq::empty(), Seq::empty())
        == Some(seq![seq![97u8], seq![98u8]]));
    assert(policy::sources::compiler_paths(input@, output@) == Some(seq![seq![97u8], seq![98u8]]));
    expect_verifier_true(parsed.is_some());
    let paths = parsed.unwrap();
    assert(policy::sources::dependency_values(paths@) == seq![seq![97u8], seq![98u8]]);
    assert(paths.len() == 2 && paths[0]@ == seq![97u8] && paths[1]@ == seq![98u8]);
    expect_verifier_true(paths.len() == 2);
    let expected = [97u8];
    assert(expected@ == seq![97u8]);
    expect_verifier_bytes(&paths[0], &expected);
    let expected = [98u8];
    assert(expected@ == seq![98u8]);
    expect_verifier_bytes(&paths[1], &expected);
    let wrong_output = [112u8];
    let wrong = policy::sources::compiler_dependency_paths(&input, &wrong_output);
    assert(wrong_output@ =~= seq![112u8]);
    assert(input@.subrange(0, 5)[0] != policy::sources::target_prefix(wrong_output@)[0]);
    assert(!policy::sources::prefix_at(input@, 0, policy::sources::target_prefix(wrong_output@)));
    expect_verifier_true(wrong.is_none());
    let unterminated = [111u8, 46u8, 100u8, 58u8, 32u8, 97u8];
    assert(unterminated@ =~= seq![111u8, 46u8, 100u8, 58u8, 32u8, 97u8]);
    let missing_lf = policy::sources::compiler_dependency_paths(&unterminated, &output);
    expect_verifier_true(missing_lf.is_none());
    let empty = [111u8, 46u8, 100u8, 58u8, 32u8, 10u8];
    assert(empty@ =~= seq![111u8, 46u8, 100u8, 58u8, 32u8, 10u8]);
    let empty_list = policy::sources::compiler_dependency_paths(&empty, &output);
    expect_verifier_true(empty_list.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn decimal_serialization_includes_zero_and_digit_boundary() {
    let zero = policy::sources::decimal_bytes(0);
    let nine = policy::sources::decimal_bytes(9);
    let ten = policy::sources::decimal_bytes(10);
    reveal_with_fuel(policy::sources::decimal_text, 3);
    assert(zero@ =~= seq![48u8]);
    let expected = [48u8];
    assert(expected@ == seq![48u8]);
    expect_verifier_bytes(&zero, &expected);
    assert(nine@ =~= seq![57u8]);
    let expected = [57u8];
    assert(expected@ == seq![57u8]);
    expect_verifier_bytes(&nine, &expected);
    assert(ten@ =~= seq![49u8, 48u8]);
    expect_verifier_bytes(&ten, &[49u8, 48u8]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn decimal_serialization_includes_u64_maximum() {
    let maximum = policy::sources::decimal_bytes(18446744073709551615u64);
    reveal_with_fuel(policy::sources::decimal_text, 21);
    assert(maximum@ =~= seq![49u8, 56u8, 52u8, 52u8, 54u8, 55u8, 52u8, 52u8, 48u8, 55u8,
                            51u8, 55u8, 48u8, 57u8, 53u8, 53u8, 49u8, 54u8, 49u8, 53u8]);
    expect_verifier_bytes(&maximum, &[49u8, 56u8, 52u8, 52u8, 54u8, 55u8, 52u8, 52u8, 48u8, 55u8,
                                     51u8, 55u8, 48u8, 57u8, 53u8, 53u8, 49u8, 54u8, 49u8, 53u8]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn checksum_lines_bind_path_hash_and_source_length() {
    proof {
        reveal_strlit("# checksum:sha256="); reveal_strlit(" file_len:");

    }
    let source = policy::sources::SourceBinding {
        path: vec![97u8, 32u8, 98u8], sha256: vec![102u8], length: 10,
    };
    let exact_input = [35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 98u8];
    assert(exact_input@ =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 98u8]);
    let exact = policy::sources::matches_checksum(
        &exact_input, &source);
    let wrong_hash_input = [35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 101u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 98u8];
    assert(wrong_hash_input@ =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 101u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 98u8]);
    let wrong_hash = policy::sources::matches_checksum(
        &wrong_hash_input, &source);
    let wrong_length_input = [35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 49u8, 32u8, 97u8, 92u8, 32u8, 98u8];
    assert(wrong_length_input@ =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 49u8, 32u8, 97u8, 92u8, 32u8, 98u8]);
    let wrong_length = policy::sources::matches_checksum(
        &wrong_length_input, &source);
    let wrong_path_input = [35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 99u8];
    assert(wrong_path_input@ =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 99u8]);
    let wrong_path = policy::sources::matches_checksum(
        &wrong_path_input, &source);
    let leading_zero_input = [35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 48u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 98u8];
    assert(leading_zero_input@ =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 48u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 98u8]);
    let leading_zero = policy::sources::matches_checksum(&leading_zero_input, &source);
    let extra_input = [35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 98u8, 120u8];
    assert(extra_input@ =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 98u8, 120u8]);
    let extra = policy::sources::matches_checksum(
        &extra_input, &source);
    reveal_with_fuel(policy::sources::decimal_text, 3);
    reveal_with_fuel(policy::sources::dependency_escape, 4);
    assert(policy::sources::decimal_text(10) =~= seq![49u8, 48u8]);
    assert(policy::sources::dependency_escape(source.path@) =~= seq![97u8, 92u8, 32u8, 98u8]);
    assert(vstd::string::is_ascii("# checksum:sha256="));
    assert(vstd::string::is_ascii(" file_len:"));
    proof {
        vstd::string::is_ascii_spec_bytes("# checksum:sha256=");
        vstd::string::is_ascii_spec_bytes(" file_len:");
    }
    assert(vstd::utf8::encode_utf8("# checksum:sha256="@) =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8]);
    assert(vstd::utf8::encode_utf8(" file_len:"@) =~= seq![32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8]);
    assert(policy::sources::checksum_line(&source) =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 48u8, 32u8, 97u8, 92u8, 32u8, 98u8]);
    expect_verifier_true(exact);
    expect_verifier_true(!wrong_hash && !wrong_length && !wrong_path && !leading_zero && !extra);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_decode_only_escaped_spaces() {
    let encoded = [97u8, 92u8, 32u8, 98u8];
    let parsed = policy::sources::parse_dependency_paths(&encoded);
    reveal_with_fuel(policy::sources::dependency_scan, 5);
    expect_verifier_true(parsed.is_some());
    let paths = parsed.unwrap();
    expect_verifier_true(paths.len() == 1);
    assert(paths[0]@ == seq![97u8, 32u8, 98u8]);
    expect_verifier_bytes(&paths[0], &[97u8, 32u8, 98u8]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_preserve_order_and_duplicates() {
    reveal_with_fuel(policy::sources::dependency_scan, 6);
    let encoded = [97u8, 32u8, 98u8, 32u8, 97u8];
    let parsed = policy::sources::parse_dependency_paths(&encoded);
    expect_verifier_true(parsed.is_some());
    let paths = parsed.unwrap();
    assert(encoded@ =~= seq![97u8, 32u8, 98u8, 32u8, 97u8]);
    assert(policy::sources::dependency_scan(encoded@, Seq::empty(), Seq::empty())
        == Some(seq![seq![97u8], seq![98u8], seq![97u8]]));
    assert(policy::sources::dependency_values(paths@) == seq![seq![97u8], seq![98u8], seq![97u8]]);
    expect_verifier_true(paths.len() == 3);
    assert(paths[0]@ == seq![97u8]);
    let expected = [97u8];
    assert(expected@ == seq![97u8]);
    expect_verifier_bytes(&paths[0], &expected);
    assert(paths[1]@ == seq![98u8]);
    let expected = [98u8];
    assert(expected@ == seq![98u8]);
    expect_verifier_bytes(&paths[1], &expected);
    assert(paths[2]@ == seq![97u8]);
    let expected = [97u8];
    assert(expected@ == seq![97u8]);
    expect_verifier_bytes(&paths[2], &expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_preserve_relative_components_and_utf8_bytes() {
    reveal_with_fuel(policy::sources::dependency_scan, 5);
    let relative = [46u8, 46u8, 47u8, 99u8];
    let parsed = policy::sources::parse_dependency_paths(&relative);
    expect_verifier_true(parsed.is_some());
    let paths = parsed.unwrap();
    assert(paths.len() == 1 && paths[0]@ == relative@);
    expect_verifier_true(paths.len() == 1);
    expect_verifier_bytes(&paths[0], &relative);
    let unicode = [206u8, 187u8];
    let unchanged = policy::sources::parse_dependency_paths(&unicode);
    expect_verifier_true(unchanged.is_some());
    let unchanged = unchanged.unwrap();
    assert(unchanged.len() == 1 && unchanged[0]@ == unicode@);
    expect_verifier_true(unchanged.len() == 1);
    expect_verifier_bytes(&unchanged[0], &unicode);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_reject_empty_words_controls_and_unknown_escapes() {
    reveal_with_fuel(policy::sources::dependency_scan, 8);
    let empty: [u8; 0] = [];
    let missing = policy::sources::parse_dependency_paths(&empty);
    let leading = policy::sources::parse_dependency_paths(&[32u8, 97u8]);
    let trailing = policy::sources::parse_dependency_paths(&[97u8, 32u8]);
    let repeated = policy::sources::parse_dependency_paths(&[97u8, 32u8, 32u8, 98u8]);
    let slash = policy::sources::parse_dependency_paths(&[97u8, 92u8]);
    let escape = policy::sources::parse_dependency_paths(&[97u8, 92u8, 98u8]);
    let nul = policy::sources::parse_dependency_paths(&[97u8, 0u8]);
    let tab = policy::sources::parse_dependency_paths(&[97u8, 9u8]);
    let cr = policy::sources::parse_dependency_paths(&[97u8, 13u8]);
    let lf = policy::sources::parse_dependency_paths(&[97u8, 10u8]);
    let del = policy::sources::parse_dependency_paths(&[97u8, 127u8]);
    expect_verifier_true(missing.is_none() && leading.is_none() && trailing.is_none());
    expect_verifier_true(repeated.is_none() && slash.is_none() && escape.is_none());
    expect_verifier_true(nul.is_none() && tab.is_none() && cr.is_none() && lf.is_none() && del.is_none());
}


fn declared_file(path: &str, root: &str, operation: &str) -> (file: policy::sources::SourceFile)
    ensures file.path@ == path@, file.verification.is_some(),
        file.verification.unwrap().roots.len() == 1,
        file.verification.unwrap().roots[0].path@ == root@,
        file.verification.unwrap().roots[0].operation@ == operation@,
{
    policy::sources::SourceFile { path: path.to_string(), verification: Some(policy::sources::SourceVerification {
        role: policy::sources::SourceRole::Program,
        roots: vec![policy::sources::SourceProofRoot {
            path: root.to_string(), operation: operation.to_string(),
        }],
    }) }
}

fn source(path: &str) -> (binding: policy::sources::SourceBinding)
    ensures binding.path@ == vstd::utf8::encode_utf8(path@),
{
    let bytes = path.as_bytes();
    policy::sources::SourceBinding { path: policy::sources::copy_range(bytes, 0, bytes.len()), sha256: Vec::new(), length: 0 }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn matching_root_and_compiled_file_are_accepted() {
    let files = vec![declared_file("source é.rs", "root.rs", "proof")];
    let candidate = vec![source("source é.rs")];
    proof { reveal_with_fuel(policy::sources::binding_error, 2); reveal_with_fuel(policy::sources::file_binding_error, 2); }
    let accepted = policy::sources::bind_registered_roots(&files, "proof", "root.rs", &candidate);
    expect_verifier_true(accepted.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn different_root_is_rejected_with_complete_error() {
    let files = vec![declared_file("source", "Root", "proof")];
    let candidate = vec![source("source")];
    proof {
        reveal_strlit("Root"); reveal_strlit("root");
        assert("Root"@[0] != "root"@[0]);
        reveal_with_fuel(policy::sources::binding_error, 2); reveal_with_fuel(policy::sources::file_binding_error, 2);
    }
    let rejected = policy::sources::bind_registered_roots(&files, "proof", "root", &candidate);
    expect_verifier_true(rejected.is_err());
    let error = rejected.unwrap_err();
    let same = identifiers::equal_text(error.message,
        "E_VERUS_COVERAGE: declared root differs from the invoked root: ");
    expect_verifier_true(same);
    let path = identifiers::equal_text(error.path, "source");
    expect_verifier_true(path);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_compiled_file_and_error_priority_are_independent() {
    let files = vec![declared_file("missing", "root", "proof")];
    let candidate = Vec::new();
    proof { reveal_with_fuel(policy::sources::binding_error, 2); reveal_with_fuel(policy::sources::file_binding_error, 2); }
    let rejected = policy::sources::bind_registered_roots(&files, "proof", "root", &candidate);
    expect_verifier_true(rejected.is_err());
    let error = rejected.unwrap_err();
    let same = identifiers::equal_text(error.message,
        "E_VERUS_COVERAGE: declared file is absent from compiler sources: ");
    expect_verifier_true(same);
    let path = identifiers::equal_text(error.path, "missing");
    expect_verifier_true(path);
    let both = vec![declared_file("missing", "Root", "proof")];
    proof {
        reveal_strlit("Root"); reveal_strlit("root");
        assert("Root"@[0] != "root"@[0]);
        reveal_with_fuel(policy::sources::binding_error, 2); reveal_with_fuel(policy::sources::file_binding_error, 2);
    }
    let rejected = policy::sources::bind_registered_roots(&both, "proof", "root", &candidate);
    expect_verifier_true(rejected.is_err());
    let error = rejected.unwrap_err();
    let same = identifiers::equal_text(error.message,
        "E_VERUS_COVERAGE: declared root differs from the invoked root: ");
    expect_verifier_true(same);
    let path = identifiers::equal_text(error.path, "missing");
    expect_verifier_true(path);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unrelated_operations_and_missing_declarations_do_not_claim_coverage() {
    let files = vec![
        policy::sources::SourceFile { path: "undeclared".to_string(), verification: None },
        declared_file("missing", "wrong", "proof\0"),
    ];
    proof {
        reveal_strlit("proof"); reveal_strlit("proof\0");
        assert("proof"@.len() != "proof\0"@.len());
        reveal_with_fuel(policy::sources::binding_error, 3); reveal_with_fuel(policy::sources::file_binding_error, 2);
    }
    let ignored = policy::sources::bind_registered_roots(&files, "proof", "root", &Vec::new());
    expect_verifier_true(ignored.is_ok());
    let empty_files = Vec::new();
    let empty = policy::sources::bind_registered_roots(&empty_files, "proof", "root", &Vec::new());
    expect_verifier_true(empty.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn first_file_error_and_later_roots_are_not_skipped() {
    let files = vec![declared_file("missing", "root", "proof"),
        declared_file("missing", "Root", "proof")];
    proof { reveal_with_fuel(policy::sources::binding_error, 2); reveal_with_fuel(policy::sources::file_binding_error, 2); }
    let rejected = policy::sources::bind_registered_roots(&files, "proof", "root", &Vec::new());
    expect_verifier_true(rejected.is_err());
    let error = rejected.unwrap_err();
    let same = identifiers::equal_text(error.message,
        "E_VERUS_COVERAGE: declared file is absent from compiler sources: ");
    expect_verifier_true(same);
    let path = identifiers::equal_text(error.path, "missing");
    expect_verifier_true(path);
    let files = vec![policy::sources::SourceFile { path: "source".to_string(),
        verification: Some(policy::sources::SourceVerification {
            role: policy::sources::SourceRole::Program,
            roots: vec![
                policy::sources::SourceProofRoot { path: "root".to_string(), operation: "proof".to_string() },
                policy::sources::SourceProofRoot { path: "Root".to_string(), operation: "proof".to_string() },
            ],
        }),
    }];
    let candidate = vec![source("source")];
    proof {
        reveal_strlit("Root"); reveal_strlit("root");
        assert("Root"@[0] != "root"@[0]);
        reveal_with_fuel(policy::sources::binding_error, 2); reveal_with_fuel(policy::sources::file_binding_error, 3);
    }
    let rejected = policy::sources::bind_registered_roots(&files, "proof", "root", &candidate);
    expect_verifier_true(rejected.is_err());
    let error = rejected.unwrap_err();
    let same = identifiers::equal_text(error.message,
        "E_VERUS_COVERAGE: declared root differs from the invoked root: ");
    expect_verifier_true(same);
    let path = identifiers::equal_text(error.path, "source");
    expect_verifier_true(path);
}
}
#[cfg(not(verus_keep_ghost))]
struct BoundSources;

#[cfg(not(verus_keep_ghost))]
impl policy::sources::CandidateSourceRead for BoundSources {
    fn text(&self, path: &[u8]) -> Result<String, String> {
        String::from_utf8(path.to_vec()).map_err(|error| error.to_string())
    }
    fn mode(&self, path: &str) -> Option<String> {
        (path == "root").then(|| "100644".to_string())
    }
    fn blob(&self, path: &str) -> Result<Vec<u8>, String> {
        if path == "root" {
            Ok(b"source".to_vec())
        } else {
            Err("absent".to_string())
        }
    }
    fn digest(&self, bytes: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(bytes)).into_bytes()
    }
}

#[cfg(not(verus_keep_ghost))]
impl policy::CompilerReportRead for BoundSources {
    fn decode(&self, bytes: &[u8]) -> Option<CompilerReport> {
        serde_json::from_slice(bytes).ok()
    }
}

#[cfg(not(verus_keep_ghost))]
fn unified_bound_result(file: &str, root: &str) -> Result<Option<policy::ProofAdmission>, String> {
    use policy::sources::CandidateSourceRead;
    let args = arguments();
    let contract = contract();
    let methods = vec![policy::ProofInvocation {
        operation: "proof",
        argv: &args,
        output_contract: &contract,
        output_limit_bytes: 1024,
    }];
    let mut argv = vec!["verus".to_string()];
    argv.extend(args.iter().cloned());
    let mut files = vec![declared_file(file, root, "proof")];
    if file != "root" {
        files.push(policy::sources::SourceFile {
            path: "root".to_string(),
            verification: None,
        });
    }
    let registry = Ok(policy::sources::SourceRegistry { files });
    let digest = String::from_utf8(BoundSources.digest(b"source")).unwrap();
    let dependency = Some(Ok(format!(
        "out.d: root\n\nout: root\n\nroot:\n\n# checksum:sha256={digest} file_len:6 root\n"
    )
    .into_bytes()));
    let artifact = Some(Ok(library(1)));
    let stdout = serde_json::to_vec(&serde_json::json!({
        "verus": { "version": "v", "commit": "c" },
        "verification-results": { "success": true, "encountered-error": false,
            "encountered-vir-error": false, "verified": 1, "errors": 0,
            "is-verifying-entire-crate": true }
    }))
    .unwrap();
    policy::bind_operation_output(
        "passed",
        "proof",
        &methods,
        Some(policy::ProofProcess {
            operation: "proof",
            argv: &argv,
            stdout: &stdout,
            exit: 0,
            failed: false,
            dependency: &dependency,
            artifact: &artifact,
        }),
        &registry,
        &BoundSources,
        |message, path| format!("{message}{path}"),
    )
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn unified_proof_accepts_the_declared_root_and_compiled_file() {
    let admitted = unified_bound_result("root", "root").unwrap().unwrap();
    assert!(admitted.report.verification.success);
    assert_eq!(admitted.sources.candidate.len(), 1);
    assert_eq!(admitted.sources.candidate[0].path, b"root");
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn unified_proof_rejects_a_declared_root_that_was_not_invoked() {
    match unified_bound_result("root", "other") {
        Err(error) => assert_eq!(
            error,
            "E_VERUS_COVERAGE: declared root differs from the invoked root: root"
        ),
        Ok(_) => panic!("an unrelated root must not receive proof admission"),
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn unified_proof_rejects_a_declared_file_missing_from_actual_compilation() {
    match unified_bound_result("file", "root") {
        Err(error) => assert_eq!(
            error,
            "E_VERUS_COVERAGE: declared file is absent from compiler sources: file"
        ),
        Ok(_) => panic!("an uncompiled file must not receive proof admission"),
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn coverage_diagnostic_preserves_the_complete_utf8_and_nul_path() {
    let files = vec![declared_file("路径 é\0suffix", "other", "proof")];
    let error = policy::sources::bind_registered_roots(&files, "proof", "root", &Vec::new())
        .err()
        .unwrap();
    let message = error.render(|message, path| format!("{message}{path}"));
    assert_eq!(
        message,
        "E_VERUS_COVERAGE: declared root differs from the invoked root: 路径 é\0suffix"
    );
}
