//! 原进程与完整 Verus 输出的独立成功、失败和边界预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_verifier.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_verifier as policy;
use policy::{CompilerReport, CompilerVersion, OutputContract, Verification};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

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
    assert(ignored.len() == 0);
    let ordinary = policy::admit_output(&None, &None, &Vec::new(), i32::MIN, true);
    assert(ordinary.is_ok());
    let selected = policy::proof_stdout(&contract(), &raw);
    assert(selected@ == raw@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn full_crate_zero_error_report_and_successful_process_are_accepted() {
    let accepted = policy::admit_output(&contract(), &report(verification()),
        &arguments(), 0, false);
    assert(accepted.is_ok());
    let mut maximum = verification();
    maximum.verified = u64::MAX;
    let at_maximum = policy::admit_output(&contract(), &report(maximum),
        &arguments(), 0, false);
    assert(at_maximum.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_report_or_failed_process_cannot_be_success() {
    let missing = policy::admit_output(&contract(), &None, &arguments(), 0, false);
    assert(missing.is_err());
    let nonzero = policy::admit_output(&contract(), &report(verification()), &arguments(), 1, false);
    assert(nonzero.is_err());
    let negative = policy::admit_output(&contract(), &report(verification()), &arguments(), i32::MIN, false);
    assert(negative.is_err());
    let failed = policy::admit_output(&contract(), &report(verification()), &arguments(), 0, true);
    assert(failed.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn each_compiler_failure_flag_and_count_independently_rejects_the_report() {
    let mut unsuccessful = verification(); unsuccessful.success = false;
    let a = policy::admit_output(&contract(), &report(unsuccessful), &arguments(), 0, false);
    assert(a.is_err());
    let mut error = verification(); error.encountered_error = true;
    let b = policy::admit_output(&contract(), &report(error), &arguments(), 0, false);
    assert(b.is_err());
    let mut vir_error = verification(); vir_error.encountered_vir_error = true;
    let c = policy::admit_output(&contract(), &report(vir_error), &arguments(), 0, false);
    assert(c.is_err());
    let mut errors = verification(); errors.errors = 1;
    let d = policy::admit_output(&contract(), &report(errors), &arguments(), 0, false);
    assert(d.is_err());
    let mut empty = verification(); empty.verified = 0;
    let e = policy::admit_output(&contract(), &report(empty), &arguments(), 0, false);
    assert(e.is_err());
    let mut partial = verification(); partial.is_verifying_entire_crate = false;
    let f = policy::admit_output(&contract(), &report(partial), &arguments(), 0, false);
    assert(f.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn version_and_commit_must_match_one_explicit_nonempty_contract() {
    proof {
        reveal_strlit("v"); reveal_strlit("c"); reveal_strlit("");
        assert("v"@[0] != "c"@[0]);
    }
    let wrong_version = Some(OutputContract { version: "c".to_string(), commit: "c".to_string() });
    let a = policy::admit_output(&wrong_version, &report(verification()), &arguments(), 0, false);
    assert(a.is_err());
    let wrong_commit = Some(OutputContract { version: "v".to_string(), commit: "v".to_string() });
    let b = policy::admit_output(&wrong_commit, &report(verification()), &arguments(), 0, false);
    assert(b.is_err());
    let empty_version = Some(OutputContract { version: "".to_string(), commit: "c".to_string() });
    let c = policy::validate_contract(&empty_version); assert(c.is_err());
    let empty_commit = Some(OutputContract { version: "v".to_string(), commit: "".to_string() });
    let d = policy::validate_contract(&empty_commit); assert(d.is_err());
    let original = OutputContract { version: "v".to_string(), commit: "c".to_string() };
    let cloned = original.clone();
    let preserved = cloned.eq(&original); assert(preserved);
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
    assert(a.is_err());
    let mut extra = arguments(); extra.push("--verify-function=one".to_string());
    let b = policy::canonical_verifier_arguments(&extra); assert(!b);
    let mut unchecked = arguments(); unchecked[1] = "--no-verify".to_string();
    assert(unchecked[1]@ != "--no-cheating"@);
    let c = policy::canonical_verifier_arguments(&unchecked); assert(!c);
    let mut rebound = arguments(); rebound[10] = "other".to_string();
    assert(rebound[7]@.len() == 30 && rebound[10]@.len() == 5);
    let d = policy::canonical_verifier_arguments(&rebound); assert(!d);
    let missing = Vec::new();
    let e = policy::canonical_verifier_arguments(&missing); assert(!e);
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
    assert(good.is_ok() && large.is_ok());
    assert(empty.is_err() && missing.is_err() && failed.is_err());
    let mut wrong_path = library(1); wrong_path.path.push(0u8);
    assert(wrong_path.path@.len() != vstd::utf8::encode_utf8("out"@).len());
    let rebound = policy::admit_artifact_observation(&contract(), &Some(Ok(wrong_path)), &args);
    assert(rebound.is_err());
    let mut bad_digest = library(1); bad_digest.sha256 = vec![48u8];
    let short = policy::admit_artifact_observation(&contract(), &Some(Ok(bad_digest)), &args);
    assert(short.is_err());
    let mut invalid = library(1); invalid.sha256.set(17, 65u8);
    assert(invalid.sha256[17] == 65u8);
    assert(!policy::identifiers::digest_format(invalid.sha256@));
    let uppercase = policy::admit_artifact_observation(&contract(), &Some(Ok(invalid)), &args);
    assert(uppercase.is_err());
    let no_args = policy::admit_artifact_observation(&contract(), &Some(Ok(library(1))), &Vec::new());
    let ordinary = policy::admit_artifact_observation(&None, &None, &Vec::new());
    let extra = policy::admit_artifact_observation(&None, &Some(Ok(library(1))), &args);
    assert(no_args.is_err() && ordinary.is_ok() && extra.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn library_observation_keeps_original_io_error_and_rejects_failed_process() {
    let io = UnavailableSources;
    let ordinary = policy::observe_artifact(&None, &Vec::new(), 0, false, &io);
    let failed = policy::observe_artifact(&contract(), &arguments(), 7, false, &io);
    let interrupted = policy::observe_artifact(&contract(), &arguments(), 0, true, &io);
    let invalid = policy::observe_artifact(&contract(), &Vec::new(), 0, false, &io);
    let unavailable = policy::observe_artifact(&contract(), &arguments(), 0, false, &io);
    assert(ordinary.is_none());
    assert(failed.is_some() && failed.unwrap().is_err());
    assert(interrupted.is_some() && interrupted.unwrap().is_err());
    assert(invalid.is_some() && invalid.unwrap().is_err());
    // trait 获取结果没有预先保证；这里只核原观察始终拒绝成功准入。
    let admitted = policy::admit_artifact_observation(&contract(), &unavailable, &arguments());
    if unavailable.is_some() && unavailable.unwrap().is_err() { assert(admitted.is_err()); }
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
    assert(accepted.is_err());
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
    assert(missing.is_err());
    let failure = Some(Err("original read error\0é".to_string()));
    let failed = policy::admit_observed_output(&contract(), &report, &args, 0, false, &failure, 1, &Some(Ok(library(1))));
    assert(failed.is_err());
    let original = Some(Ok(vec![0u8, 255u8]));
    let too_large = policy::admit_observed_output(&contract(), &report, &args, 0, false, &original, 1, &Some(Ok(library(1))));
    let exact = policy::admit_observed_output(&contract(), &report, &args, 0, false, &original, 2, &Some(Ok(library(1))));
    let zero_bound = policy::admit_observed_output(&contract(), &report, &args, 0, false, &original, 0, &Some(Ok(library(1))));
    let over_bound = policy::admit_observed_output(&contract(), &report, &args, 0, false, &original, 67_108_865, &Some(Ok(library(1))));
    assert(too_large.is_err() && exact.is_ok() && zero_bound.is_err() && over_bound.is_err());
    let ordinary = policy::admit_observed_output(&None, &None, &Vec::new(), 0, false, &None, 0, &None);
    let unexpected = policy::admit_observed_output(&None, &None, &Vec::new(), 0, false, &original, 2, &None);
    assert(ordinary.is_ok() && unexpected.is_err());
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
    assert(ordinary.is_none());
    let nonzero = policy::sources::observe_dependency(&contract(), &arguments(), 7, false, 1, &io);
    let process_failure = policy::sources::observe_dependency(&contract(), &arguments(), 0, true, 1, &io);
    let no_bound = policy::sources::observe_dependency(&contract(), &arguments(), 0, false, 0, &io);
    let huge_bound = policy::sources::observe_dependency(&contract(), &arguments(), 0, false, usize::MAX, &io);
    assert(nonzero.is_some() && nonzero.unwrap().is_err());
    assert(process_failure.is_some() && process_failure.unwrap().is_err());
    assert(no_bound.is_some() && no_bound.unwrap().is_err());
    assert(huge_bound.is_some() && huge_bound.unwrap().is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_argv_keeps_every_argument_and_missing_success_receipt_rejects() {
    let original = vec!["verus\0é".to_string(), "root".to_string(), "root".to_string()];
    let args = policy::sources::process_arguments(&original).unwrap();
    assert(args.len() == 2 && args[0]@ == "root"@ && args[1]@ == "root"@);
    let empty = policy::sources::process_arguments(&Vec::new());
    assert(empty.is_err());
    let io = UnavailableSources;
    let registry = Err("registry read error".to_string());
    let missing = policy::sources::bind_process_sources("passed", None, &registry, &io);
    assert(missing.is_err());
    let blocked = policy::sources::bind_process_sources("blocked", None, &registry, &io);
    let failed = policy::sources::bind_process_sources("failed", None, &registry, &io);
    proof {
        reveal_strlit("blocked"); reveal_strlit("failed"); reveal_strlit("passed");
        assert("blocked"@[0] != "passed"@[0]); assert("failed"@[0] != "passed"@[0]);
    }
    assert(blocked.is_ok() && failed.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn explicit_file_membership_uses_full_original_text() {
    proof {
        reveal_strlit("a"); reveal_strlit("A"); reveal_strlit("a\0");
        vstd::string::is_ascii_spec_bytes("a"); vstd::string::is_ascii_spec_bytes("A");
        vstd::string::is_ascii_spec_bytes("a\0");
    }
    let files = vec![policy::sources::SourceFile { path: "a".to_string() }];
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
    assert(exact);
    assert(!wrong_case);
    assert(!suffix);
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
    assert(accepted);
    let changed_candidate = vec![metadata_fixture(vec![97u8], 8)];
    let changed_length = policy::sources::admit_source_metadata(&sources, &changed_candidate, &root);
    assert(!changed_length);
    let mut changed = metadata_fixture(vec![46u8, 47, 97], 7);
    changed.sha256[0] = 65u8;
    let changed_sources = vec![changed];
    assert(changed_sources[0].sha256[0] == 65u8);
    assert(candidate[0].sha256[0] == 48u8);
    assert(!policy::sources::source_pair_matches(&changed_sources[0], &candidate[0]));
    let changed_digest = policy::sources::admit_source_metadata(&changed_sources, &candidate, &root);
    assert(!changed_digest);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_metadata_rejects_missing_roots_and_empty_inventories() {
    reveal_with_fuel(policy::sources::path_scan, 4);
    reveal_with_fuel(policy::sources::joined_components, 3);
    let sources = vec![metadata_fixture(vec![97u8], 0)];
    let candidate = vec![metadata_fixture(vec![97u8], 0)];
    let missing_root = policy::sources::admit_source_metadata(&sources, &candidate, &[98u8]);
    assert(!missing_root);
    let empty: Vec<policy::sources::SourceBinding> = Vec::new();
    let no_sources = policy::sources::admit_source_metadata(&empty, &empty, &[97u8]);
    let unequal = policy::sources::admit_source_metadata(&sources, &empty, &[97u8]);
    assert(!no_sources && !unequal);
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
    assert(accepted);
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
    assert(!accepted);
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
    assert(candidate[0].sha256[0] == 48u8 && candidate[1].sha256[0] == 49u8);
    assert(!policy::sources::source_aliases_consistent(candidate@, 1));
    let accepted = policy::sources::admit_source_metadata(&sources, &candidate, &[97u8]);
    assert(!accepted);
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
    let dotted = [46u8, 47, 97, 47, 47, 46, 47, 98, 47];
    assert(dotted@ =~= seq![46u8, 47u8, 97u8, 47u8, 47u8, 46u8, 47u8, 98u8, 47u8]);
    assert(policy::sources::relative_components(seq![46u8, 47u8, 97u8, 47u8, 47u8, 46u8, 47u8, 98u8, 47u8])
        == Some(seq![seq![97u8], seq![98u8]])) by (compute);
    assert(policy::sources::joined_components(seq![seq![97u8], seq![98u8]]) == seq![97u8, 47u8, 98u8]) by (compute);
    let normalized = policy::sources::normalize_relative_path(&dotted).unwrap();
    assert(normalized@ == seq![97u8, 47u8, 98u8]);
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
    assert(absolute.is_none() && empty.is_none() && dot.is_none());
    assert(parent.is_none() && cancelled.is_none() && escape.is_none());
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
    let nul = policy::sources::normalize_relative_path(&[97u8, 0]);
    let lf = policy::sources::normalize_relative_path(&[97u8, 10]);
    let del = policy::sources::normalize_relative_path(&[97u8, 127]);
    let backslash = policy::sources::normalize_relative_path(&[97u8, 92, 98]);
    assert(policy::sources::relative_components(seq![97u8, 0u8]).is_none()) by (compute);
    assert(policy::sources::relative_components(seq![97u8, 10u8]).is_none()) by (compute);
    assert(policy::sources::relative_components(seq![97u8, 127u8]).is_none()) by (compute);
    assert(policy::sources::relative_components(seq![97u8, 92u8, 98u8]).is_none()) by (compute);
    assert(nul.is_none() && lf.is_none() && del.is_none() && backslash.is_none());
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
    assert(sources.len() == 1);
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
    assert(matched);

    let missing = policy::sources::copy_range(&input, 0, 17);
    let missing_checksum = policy::sources::matches_dependency_content(missing.as_slice(), &output, &sources);
    assert(missing@.skip(17) =~= Seq::<u8>::empty());
    assert(!missing_checksum);

    let truncated = policy::sources::copy_range(&input, 0, 49);
    let no_final_lf = policy::sources::matches_dependency_content(truncated.as_slice(), &output, &sources);
    assert(truncated@.skip(17) =~= seq![35u8, 32u8, 99u8, 104u8, 101u8, 99u8, 107u8, 115u8, 117u8, 109u8, 58u8, 115u8, 104u8, 97u8, 50u8, 53u8, 54u8, 61u8, 102u8, 32u8, 102u8, 105u8, 108u8, 101u8, 95u8, 108u8, 101u8, 110u8, 58u8, 49u8, 32u8, 97u8]);
    proof { policy::sources::next_byte_at(truncated@.skip(17), 0, 32, 10u8); }
    assert(!no_final_lf);

    let empty: Vec<policy::sources::SourceBinding> = Vec::new();
    let no_sources = policy::sources::matches_dependency_content(&input, &output, &empty);
    assert(!no_sources);
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
    assert(parsed.is_some());
    let paths = parsed.unwrap();
    assert(policy::sources::dependency_values(paths@) == seq![seq![97u8], seq![98u8]]);
    assert(paths.len() == 2 && paths[0]@ == seq![97u8] && paths[1]@ == seq![98u8]);
    let wrong_output = [112u8];
    let wrong = policy::sources::compiler_dependency_paths(&input, &wrong_output);
    assert(wrong_output@ =~= seq![112u8]);
    assert(input@.subrange(0, 5)[0] != policy::sources::target_prefix(wrong_output@)[0]);
    assert(!policy::sources::prefix_at(input@, 0, policy::sources::target_prefix(wrong_output@)));
    assert(wrong.is_none());
    let unterminated = [111u8, 46u8, 100u8, 58u8, 32u8, 97u8];
    assert(unterminated@ =~= seq![111u8, 46u8, 100u8, 58u8, 32u8, 97u8]);
    let missing_lf = policy::sources::compiler_dependency_paths(&unterminated, &output);
    assert(missing_lf.is_none());
    let empty = [111u8, 46u8, 100u8, 58u8, 32u8, 10u8];
    assert(empty@ =~= seq![111u8, 46u8, 100u8, 58u8, 32u8, 10u8]);
    let empty_list = policy::sources::compiler_dependency_paths(&empty, &output);
    assert(empty_list.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn decimal_serialization_includes_zero_and_digit_boundary() {
    let zero = policy::sources::decimal_bytes(0);
    let nine = policy::sources::decimal_bytes(9);
    let ten = policy::sources::decimal_bytes(10);
    reveal_with_fuel(policy::sources::decimal_text, 3);
    assert(zero@ =~= seq![48u8]);
    assert(nine@ =~= seq![57u8]);
    assert(ten@ =~= seq![49u8, 48u8]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn decimal_serialization_includes_u64_maximum() {
    let maximum = policy::sources::decimal_bytes(18446744073709551615u64);
    reveal_with_fuel(policy::sources::decimal_text, 21);
    assert(maximum@ =~= seq![49u8, 56u8, 52u8, 52u8, 54u8, 55u8, 52u8, 52u8, 48u8, 55u8,
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
    assert(exact);
    assert(!wrong_hash && !wrong_length && !wrong_path && !leading_zero && !extra);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_decode_only_escaped_spaces() {
    let encoded = [97u8, 92u8, 32u8, 98u8];
    let parsed = policy::sources::parse_dependency_paths(&encoded);
    reveal_with_fuel(policy::sources::dependency_scan, 5);
    assert(parsed.is_some());
    let paths = parsed.unwrap();
    assert(paths.len() == 1);
    assert(paths[0]@ == seq![97u8, 32u8, 98u8]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_preserve_order_and_duplicates() {
    reveal_with_fuel(policy::sources::dependency_scan, 6);
    let encoded = [97u8, 32u8, 98u8, 32u8, 97u8];
    let parsed = policy::sources::parse_dependency_paths(&encoded);
    assert(parsed.is_some());
    let paths = parsed.unwrap();
    assert(encoded@ =~= seq![97u8, 32u8, 98u8, 32u8, 97u8]);
    assert(policy::sources::dependency_scan(encoded@, Seq::empty(), Seq::empty())
        == Some(seq![seq![97u8], seq![98u8], seq![97u8]]));
    assert(policy::sources::dependency_values(paths@) == seq![seq![97u8], seq![98u8], seq![97u8]]);
    assert(paths.len() == 3);
    assert(paths[0]@ == seq![97u8]);
    assert(paths[1]@ == seq![98u8]);
    assert(paths[2]@ == seq![97u8]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_preserve_relative_components_and_utf8_bytes() {
    reveal_with_fuel(policy::sources::dependency_scan, 5);
    let relative = [46u8, 46u8, 47u8, 99u8];
    let parsed = policy::sources::parse_dependency_paths(&relative);
    assert(parsed.is_some());
    let paths = parsed.unwrap();
    assert(paths.len() == 1 && paths[0]@ == relative@);
    let unicode = [206u8, 187u8];
    let unchanged = policy::sources::parse_dependency_paths(&unicode);
    assert(unchanged.is_some());
    let unchanged = unchanged.unwrap();
    assert(unchanged.len() == 1 && unchanged[0]@ == unicode@);
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
    assert(missing.is_none() && leading.is_none() && trailing.is_none());
    assert(repeated.is_none() && slash.is_none() && escape.is_none());
    assert(nul.is_none() && tab.is_none() && cr.is_none() && lf.is_none() && del.is_none());
}


}
