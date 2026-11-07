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

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_checksum_preserves_the_full_path_hash_and_decimal_length() {
    let line = b"# checksum:sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef file_len:0 crates/space name/\xe4\xb8\xad.rs";
    let parsed = policy::sources::parse_checksum_line(line);
    assert(parsed.is_ok());
    let record = parsed.unwrap();
    assert(record.path@ == b"crates/space name/\xe4\xb8\xad.rs"@);
    assert(record.sha256@ == b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"@);
    assert(record.length@ == b"0"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_checksum_never_truncates_large_lengths_or_parent_components() {
    let line = b"# checksum:sha256=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff file_len:184467440737095516160000 crates/a/../../other.rs";
    let parsed = policy::sources::parse_checksum_line(line);
    assert(parsed.is_ok());
    let record = parsed.unwrap();
    assert(record.path@ == b"crates/a/../../other.rs"@);
    assert(record.length@ == b"184467440737095516160000"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn malformed_compiler_checksum_fields_are_rejected() {
    let missing = policy::sources::parse_checksum_line(b"# checksum:sha256=");
    assert(missing.is_err());
    let uppercase = policy::sources::parse_checksum_line(b"# checksum:sha256=FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF file_len:1 x");
    assert(uppercase.is_err());
    let zero_prefix = policy::sources::parse_checksum_line(b"# checksum:sha256=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff file_len:01 x");
    assert(zero_prefix.is_err());
    let signed = policy::sources::parse_checksum_line(b"# checksum:sha256=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff file_len:-1 x");
    assert(signed.is_err());
    let no_path = policy::sources::parse_checksum_line(b"# checksum:sha256=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff file_len:1 ");
    assert(no_path.is_err());
    let nul = policy::sources::parse_checksum_line(b"# checksum:sha256=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff file_len:1 x\0y");
    assert(nul.is_err());
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

}
