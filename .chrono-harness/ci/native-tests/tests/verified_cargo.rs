//! 完整 Cargo 输出的独立预期；普通执行和整 crate 证明消费同一组边界。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../../../crates/runner/src/verified_verifier.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as ids;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_verifier as policy, verus};
use policy::cargo::{Event, State};
#[cfg(verus_keep_ghost)]
use policy::identifiers as ids;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect(value: bool)
    requires value,
{ if !value { panic!(); } }

proof fn library_literal_bytes()
    ensures vstd::utf8::encode_utf8("d/libn-a.rlib"@) == seq![100u8,47,108,105,98,110,45,97,46,114,108,105,98],
        vstd::utf8::encode_utf8("d/libn-a.rmeta"@) == seq![100u8,47,108,105,98,110,45,97,46,114,109,101,116,97],
        vstd::utf8::encode_utf8("d/libn-b.rmeta"@) == seq![100u8,47,108,105,98,110,45,98,46,114,109,101,116,97],
{
        reveal_strlit("d/libn-a.rlib"); reveal_strlit("d/libn-a.rmeta");
        reveal_strlit("d/libn-b.rmeta");
        assert('d' as u32 == 100 && '/' as u32 == 47 && 'l' as u32 == 108 && 'i' as u32 == 105 && 'b' as u32 == 98 && 'n' as u32 == 110 && '-' as u32 == 45 && 'a' as u32 == 97 && '.' as u32 == 46 && 'r' as u32 == 114 && 'm' as u32 == 109 && 'e' as u32 == 101 && 't' as u32 == 116);
        assert(vstd::utf8::encode_scalar(100) == seq![100u8]) by (compute);
        assert(vstd::utf8::encode_scalar(47) == seq![47u8]) by (compute);
        assert(vstd::utf8::encode_scalar(108) == seq![108u8]) by (compute);
        assert(vstd::utf8::encode_scalar(105) == seq![105u8]) by (compute);
        assert(vstd::utf8::encode_scalar(98) == seq![98u8]) by (compute);
        assert(vstd::utf8::encode_scalar(110) == seq![110u8]) by (compute);
        assert(vstd::utf8::encode_scalar(45) == seq![45u8]) by (compute);
        assert(vstd::utf8::encode_scalar(97) == seq![97u8]) by (compute);
        assert(vstd::utf8::encode_scalar(46) == seq![46u8]) by (compute);
        assert(vstd::utf8::encode_scalar(114) == seq![114u8]) by (compute);
        assert(vstd::utf8::encode_scalar(109) == seq![109u8]) by (compute);
        assert(vstd::utf8::encode_scalar(101) == seq![101u8]) by (compute);
        assert(vstd::utf8::encode_scalar(116) == seq![116u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 24);
        assert(vstd::utf8::encode_utf8("d/libn-a.rlib"@) =~= seq![100u8,47,108,105,98,110,45,97,46,114,108,105,98]);
        assert(vstd::utf8::encode_utf8("d/libn-a.rmeta"@) =~= seq![100u8,47,108,105,98,110,45,97,46,114,109,101,116,97]);
        assert(vstd::utf8::encode_utf8("d/libn-b.rmeta"@) =~= seq![100u8,47,108,105,98,110,45,98,46,114,109,101,116,97]);
 }

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_library_paths_require_one_directory_and_one_fingerprint() {
    let mut files = vec!["d/libn-a.rlib".to_string(), "d/libn-a.rmeta".to_string()];
    proof {
        library_literal_bytes();
        assert(policy::cargo::library_hash(seq![100u8], seq![110u8], files@) =~= seq![97u8]);
        let prefix = policy::cargo::library_prefix(seq![100u8], seq![110u8]);
        assert(prefix + seq![97u8] + seq![46u8,114,108,105,98] =~= vstd::utf8::encode_utf8(files[0]@));
        assert(prefix + seq![97u8] + seq![46u8,114,109,101,116,97] =~= vstd::utf8::encode_utf8(files[1]@));
        assert(policy::cargo::library_shape(seq![100u8], seq![110u8], files@, false));
    }
    let directory = [100u8];
    let name = [110u8];
    proof { assert(directory@ == seq![100u8]); assert(name@ == seq![110u8]);
        assert(policy::cargo::library_shape(directory@, name@, files@, false)); }
    let matched = policy::cargo::compiler_library_paths(directory.as_slice(), name.as_slice(), &files, false);
    expect(matched.is_some());
    let output = matched.unwrap();
    let expected = [100u8, 47, 110, 45, 97, 46, 100];
    proof { assert(output.dependency@ =~= expected@); }
    let correct = ids::equal_bytes(output.dependency.as_slice(), &expected);
    expect(correct);
    expect(!policy::cargo::compiler_library_paths(&[100u8], &[110u8], &files, true).is_some());
    expect(!policy::cargo::compiler_library_paths(&[120u8], &[110u8], &files, false).is_some());
    files = vec!["d/libn-a.rlib".to_string(), "d/libn-b.rmeta".to_string()];
    proof { library_literal_bytes(); }
    expect(!policy::cargo::compiler_library_paths(&[100u8], &[110u8], &files, false).is_some());
    let empty: Vec<String> = Vec::new();
    expect(!policy::cargo::compiler_library_paths(&[100u8], &[110u8], &empty, false).is_some());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn library_names_and_fingerprints_cannot_escape_the_declared_output_directory() {
    expect(policy::cargo::library_name(&[97u8, 90, 48, 95]));
    expect(!policy::cargo::library_name(&[]));
    expect(!policy::cargo::library_name(&[97u8, 47, 98]));
    expect(!policy::cargo::library_name(&[46u8, 46]));
    expect(!policy::cargo::library_name(&[0u8]));
    expect(policy::cargo::fingerprint(&[48u8, 57, 97, 102]));
    expect(!policy::cargo::fingerprint(&[]));
    expect(!policy::cargo::fingerprint(&[65u8]));
    expect(!policy::cargo::fingerprint(&[48u8, 47]));
    let name = [97u8,45,48,102,46,100];
    let prefix = [97u8,45];
    proof {
        assert(name@.subrange(0,2) =~= prefix@);
        assert(name@.subrange(2,4) =~= seq![48u8,102]);
        assert(policy::cargo::dependency_name(name@, prefix@));
    }
    expect(policy::cargo::selected_dependency(name.as_slice(), prefix.as_slice()));
    expect(!policy::cargo::selected_dependency(&[97u8,45,46,100], &[97u8,45]));
    let bad = [97u8,45,65,46,100];
    proof {
        assert(bad@.subrange(2,3) =~= seq![65u8]);
        assert(!policy::cargo::fingerprint_byte(bad@.subrange(2,3)[0]));
        assert(!policy::cargo::dependency_name(bad@, prefix@));
    }
    expect(!policy::cargo::selected_dependency(bad.as_slice(), prefix.as_slice()));
    expect(!policy::cargo::selected_dependency(&[97u8,45,48,46,100,120], &[97u8,45]));
    expect(!policy::cargo::selected_dependency(&[98u8,45,48,46,100], &[97u8,45]));
    expect(!policy::cargo::selected_dependency(&[97u8,45,48,46,100], &[]));
    expect(policy::cargo::all_present(&vec![true, true]));
    expect(!policy::cargo::all_present(&vec![true, false]));
    expect(!policy::cargo::all_present(&Vec::new()));
    expect(policy::cargo::output_inventory_within_bound(4096));
    expect(!policy::cargo::output_inventory_within_bound(4097));
    expect(!policy::cargo::retain_attempt(true, false));
    expect(policy::cargo::retain_attempt(true, true));
    expect(policy::cargo::retain_attempt(false, false));
    expect(policy::cargo::retain_attempt(false, true));
    expect(policy::cargo::bounded_original(true, 8_388_608));
    expect(!policy::cargo::bounded_original(true, 8_388_609));
    expect(!policy::cargo::bounded_original(false, 0));
    expect(policy::cargo::content_matches("x", 0, "x", 0));
    expect(!policy::cargo::content_matches("x", 0, "x", 1));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registration_and_recovery_preserve_the_declared_source_and_retry_bound() {
    use policy::cargo::{registered_sources, recovery, Recovery};
    proof { reveal_strlit("chrono-proved-build/v1"); reveal_strlit("other");
        reveal_strlit("s"); reveal_strlit("x");
        assert("chrono-proved-build/v1"@.len() != "other"@.len());
        assert("s"@[0] != "x"@[0]); }
    let sources = vec!["s".to_string()];
    expect(registered_sources("chrono-proved-build/v1", "s", &sources));
    expect(!registered_sources("chrono-proved-build/v1", "x", &sources));
    expect(!registered_sources("other", "s", &sources));
    expect(!registered_sources("chrono-proved-build/v1", "s", &Vec::new()));
    expect(match recovery(false, false, false) { Recovery::Continue => true, _ => false });
    expect(match recovery(false, false, true) { Recovery::Continue => true, _ => false });
    expect(match recovery(false, true, false) { Recovery::Continue => true, _ => false });
    expect(match recovery(false, true, true) { Recovery::Continue => true, _ => false });
    expect(match recovery(true, true, false) { Recovery::Continue => true, _ => false });
    expect(match recovery(true, true, true) { Recovery::Continue => true, _ => false });
    expect(match recovery(true, false, true) { Recovery::Rebuild => true, _ => false });
    expect(match recovery(true, false, false) { Recovery::Reject => true, _ => false });
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_original_records_reject_errors_trailing_records_and_missing_outputs() {
    use policy::cargo::{Artifact, Target, TargetIdentity, BuildTargets, Record, Cargo, complete_records};
    proof {
        reveal_strlit("p"); reveal_strlit("s"); reveal_strlit("n"); reveal_strlit("b");
        reveal_strlit("lib"); reveal_strlit("bin"); reveal_strlit("out");
        reveal_strlit("error"); reveal_strlit("warning"); reveal_strlit("note"); reveal_strlit("help");
        assert("lib"@[0] != "bin"@[0]);
    }
    let targets = BuildTargets {
        library: TargetIdentity { manifest: "p".to_string(), source: "s".to_string(), name: "n".to_string() },
        binary: TargetIdentity { manifest: "p".to_string(), source: "s".to_string(), name: "b".to_string() },
        published: "out".to_string(),
    };
    let contract = policy::OutputContract { version: "v".to_string(), commit: "c".to_string() };
    let mut records = vec![
        Record::Cargo(Cargo::Artifact(Artifact { package_id: "id".to_string(), manifest_path: "p".to_string(),
            target: Target { name: "n".to_string(), src_path: "s".to_string(), kind: vec!["lib".to_string()], crate_types: vec!["lib".to_string()] },
            filenames: Vec::new(), executable: None, fresh: true })),
        Record::Cargo(Cargo::Artifact(Artifact { package_id: "id".to_string(), manifest_path: "p".to_string(),
            target: Target { name: "b".to_string(), src_path: "s".to_string(), kind: vec!["bin".to_string()], crate_types: vec!["bin".to_string()] },
            filenames: vec!["out".to_string()], executable: Some("out".to_string()), fresh: true })),
    ];
    expect(!complete_records(&records, &targets, &contract));
    records.push(Record::Cargo(Cargo::Finished { success: true }));
    proof { reveal_with_fuel(policy::cargo::trace, 5); }
    expect(complete_records(&records, &targets, &contract));
    records.push(Record::Cargo(Cargo::Finished { success: true }));
    proof { reveal_with_fuel(policy::cargo::trace, 6); }
    expect(!complete_records(&records, &targets, &contract));
    let empty = Vec::new();
    expect(!complete_records(&empty, &targets, &contract));
    let errors = vec![Record::Cargo(Cargo::Message { message: policy::cargo::Message { level: "error".to_string() } }),
        Record::Cargo(Cargo::Finished { success: true })];
    expect(!complete_records(&errors, &targets, &contract));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_proofs_bind_the_current_inputs_full_metadata_and_original_success() {
    let bytes = [1u8, 2, 3];
    let changed = [1u8, 2, 4];
    proof {
        reveal_strlit("chrono-library-proof/v1"); reveal_strlit("wrong");
        reveal_strlit("input"); reveal_strlit("other");
        assert("input"@[0] != "other"@[0]);
        assert("chrono-library-proof/v1"@.len() != "wrong"@.len());
    }
    expect(policy::cargo::retained_header("chrono-library-proof/v1", "input", "input", 0, false, &bytes, &bytes));
    expect(!policy::cargo::retained_header("wrong", "input", "input", 0, false, &bytes, &bytes));
    expect(!policy::cargo::retained_header("chrono-library-proof/v1", "input", "other", 0, false, &bytes, &bytes));
    expect(!policy::cargo::retained_header("chrono-library-proof/v1", "input", "input", 1, false, &bytes, &bytes));
    expect(!policy::cargo::retained_header("chrono-library-proof/v1", "input", "input", 0, true, &bytes, &bytes));
    expect(!policy::cargo::retained_header("chrono-library-proof/v1", "input", "input", 0, false, &bytes, &changed));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn artifact_selection_requires_the_entire_declared_target_identity() {
    use policy::cargo::{Artifact, Target, artifact_selected};
    proof {
        reveal_strlit("p"); reveal_strlit("s"); reveal_strlit("n"); reveal_strlit("other");
        reveal_strlit("lib"); reveal_strlit("bin");
        assert("lib"@[0] != "bin"@[0]);
        assert("p"@.len() != "other"@.len());
        assert("s"@.len() != "other"@.len());
        assert("n"@.len() != "other"@.len());
    }
    let mut artifact = Artifact { package_id: "id".to_string(), manifest_path: "p".to_string(),
        target: Target { name: "n".to_string(), src_path: "s".to_string(),
            kind: vec!["lib".to_string()], crate_types: vec!["lib".to_string()] },
        filenames: Vec::new(), executable: None, fresh: false };
    expect(artifact_selected(&artifact, "p", "s", "n", "lib"));
    expect(!artifact_selected(&artifact, "other", "s", "n", "lib"));
    expect(!artifact_selected(&artifact, "p", "other", "n", "lib"));
    expect(!artifact_selected(&artifact, "p", "s", "other", "lib"));
    expect(!artifact_selected(&artifact, "p", "s", "n", "bin"));
    artifact.target.kind.push("lib".to_string());
    expect(!artifact_selected(&artifact, "p", "s", "n", "lib"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_error_diagnostics_cannot_be_hidden_by_a_successful_terminal_record() {
    proof {
        reveal_strlit("warning"); reveal_strlit("error"); reveal_strlit("failure-note");
        reveal_strlit("note"); reveal_strlit("help");
        assert("error"@.len() != "note"@.len()); assert("error"@.len() != "help"@.len());
        assert("failure-note"@.len() != "warning"@.len()); assert("failure-note"@.len() != "note"@.len()); assert("failure-note"@.len() != "help"@.len());
        assert("warning"@.len() != "error"@.len());
        assert("warning"@.len() != "failure-note"@.len());
    }
    expect(policy::cargo::diagnostic_rejected("error"));
    expect(policy::cargo::diagnostic_rejected("failure-note"));
    expect(!policy::cargo::diagnostic_rejected("warning"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn an_empty_or_unfinished_build_never_proves_a_library() {
    let state = State::new();
    expect(!state.accepted());
    let selected = state.step(Event::Library { fresh: false, matches: true });
    expect(selected.is_err());
    let finished = state.step(Event::Finished { success: true });
    expect(finished.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn a_cold_build_requires_its_own_proof_library_and_final_binary() {
    let state = State::new();
    let proof = state.step(Event::Proof { accepted: true });
    expect(proof.is_ok());
    let library = proof.unwrap().step(Event::Library { fresh: false, matches: true });
    expect(library.is_ok());
    let binary = library.unwrap().step(Event::Binary { matches: true });
    expect(binary.is_ok());
    let finished = binary.unwrap().step(Event::Finished { success: true });
    expect(finished.is_ok());
    expect(finished.unwrap().accepted());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn fresh_artifacts_require_original_proof_and_matching_bytes() {
    let state = State::new();
    let missing = state.step(Event::Library { fresh: true, matches: false });
    expect(missing.is_err());
    let restored = state.step(Event::Library { fresh: true, matches: true });
    expect(restored.is_ok());
    let binary = restored.unwrap().step(Event::Binary { matches: true });
    expect(binary.is_ok());
    let finished = binary.unwrap().step(Event::Finished { success: true });
    expect(finished.is_ok());
    expect(finished.unwrap().accepted());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn duplicate_proofs_libraries_and_terminal_records_are_errors() {
    let proof = State::new().step(Event::Proof { accepted: true }).unwrap();
    expect(proof.step(Event::Proof { accepted: true }).is_err());
    expect(proof.step(Event::OtherArtifact).is_err());
    expect(proof.step(Event::Library { fresh: true, matches: true }).is_err());
    let library = proof.step(Event::Library { fresh: false, matches: true }).unwrap();
    expect(library.step(Event::Library { fresh: true, matches: true }).is_err());
    expect(library.step(Event::Finished { success: true }).is_err());
    let binary = library.step(Event::Binary { matches: true }).unwrap();
    expect(binary.step(Event::Binary { matches: true }).is_err());
    let done = binary.step(Event::Finished { success: true }).unwrap();
    expect(done.step(Event::Finished { success: true }).is_err());
    expect(done.step(Event::Note).is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn report_failure_diagnostic_errors_and_mismatched_artifacts_reject() {
    let initial = State::new();
    expect(initial.step(Event::Proof { accepted: false }).is_err());
    expect(initial.step(Event::Error).is_err());
    expect(initial.step(Event::Binary { matches: true }).is_err());
    let proof = initial.step(Event::Proof { accepted: true }).unwrap();
    expect(proof.step(Event::Library { fresh: false, matches: false }).is_err());
    let library = proof.step(Event::Library { fresh: false, matches: true }).unwrap();
    expect(library.step(Event::Binary { matches: false }).is_err());
    let binary = library.step(Event::Binary { matches: true }).unwrap();
    expect(binary.step(Event::Finished { success: false }).is_err());
}

}
