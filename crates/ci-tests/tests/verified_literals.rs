//! CI 字面文本判断的独立成功、失败与边界预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../ci/src/verified_literals.rs"]
mod policy;
#[cfg(verus_keep_ghost)]
#[path = "release.rs"]
mod release_expectations;
#[cfg(not(verus_keep_ghost))]
use chrono_ci::verified_literals as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_job_terminal_outcomes_never_select_another_detail() {
    proof {
        reveal_strlit("completed"); reveal_strlit("success"); reveal_strlit("failure");
        assert("success"@[0] != "failure"@[0]);
    }
    let passed = policy::native_job_state(Some("completed"),Some("success"),Some(false),None);
    let failed = policy::native_job_state(Some("completed"),Some("failure"),Some(false),Some(1));
    assert(matches!(passed, policy::NativeJobState::Succeeded));
    assert(matches!(failed, policy::NativeJobState::Rejected));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_job_detail_requires_an_explicit_null_and_positive_exact_id() {
    proof {
        reveal_strlit("in_progress"); reveal_strlit("completed"); reveal_strlit("queued");
        reveal_strlit("waiting"); reveal_strlit("pending"); reveal_strlit("requested");
        assert("in_progress"@.len() != "completed"@.len());
    }
    let detail = policy::native_job_state(Some("in_progress"),None,Some(true),Some(91));
    let missing = policy::native_job_state(Some("in_progress"),None,None,Some(91));
    let valued = policy::native_job_state(Some("in_progress"),None,Some(false),Some(91));
    let zero = policy::native_job_state(Some("in_progress"),None,Some(true),Some(0));
    let absent = policy::native_job_state(Some("in_progress"),None,Some(true),None);
    match detail { policy::NativeJobState::ReadDetail { id } => assert(id == 91), _ => assert(false) }
    assert(matches!(missing,policy::NativeJobState::Rejected));
    assert(matches!(valued,policy::NativeJobState::Rejected));
    assert(matches!(zero,policy::NativeJobState::Rejected));
    assert(matches!(absent,policy::NativeJobState::Rejected));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_job_fields_keep_absent_distinct_from_present_and_compare_all_bytes() {
    let one = [1u8,2];
    let different = [1u8,3];
    proof { assert(one@ =~= seq![1u8,2]); assert(different@ =~= seq![1u8,3]); }
    let same = policy::observed_field_equal(Some(&one),Some(&one));
    let changed = policy::observed_field_equal(Some(&one),Some(&different));
    let absent = policy::observed_field_equal(None,Some(&one));
    let both_absent = policy::observed_field_equal(None,None);
    assert(same && !changed && !absent && both_absent);
    let short = policy::native_job_binding(&[(None,None)]);
    assert(!short);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_publisher_lookup_keeps_the_first_full_match() {
    proof {
        reveal_strlit("build"); reveal_strlit("publish"); reveal_strlit("publishing");
        assert("build"@.len() != "publish"@.len());
        assert("publishing"@.len() != "publish"@.len());
    }
    let jobs = ["build","publish","publish"];
    proof { assert(jobs@ =~= seq!["build","publish","publish"]); }
    let selected = policy::publisher_index(&jobs,"publish");
    let prefix = policy::publisher_index(&jobs,"publishing");
    assert(selected == Some(1usize) && prefix == None::<usize>);
    let dependencies = policy::publisher_dependencies(1,false);
    let missing = policy::publisher_dependencies(0,false);
    let always = policy::publisher_dependencies(1,true);
    assert(dependencies && !missing && !always);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_requires_v2_and_the_successful_workflow_trigger() {
    proof {
        reveal_strlit("chrono-github-release/v2"); reveal_strlit("chrono-github-release/v1");
        assert("chrono-github-release/v2"@[23] != "chrono-github-release/v1"@[23]);
    }
    let valid = policy::delivery_mode("chrono-github-release/v2",0);
    let push = policy::delivery_mode("chrono-github-release/v2",1);
    let legacy = policy::delivery_mode("chrono-github-release/v1",0);
    assert(valid && !push && !legacy);
}

#[cfg(verus_keep_ghost)]
fn delivery_checkout_uses_the_landing_source_and_build_keeps_its_source() {
    let delivery = policy::delivery_source(true);
    let build = policy::delivery_source(false);
    assert(delivery@ == "${{ github.event.workflow_run.head_sha || inputs.source || github.sha }}"@);
    assert(build@ == "${{ inputs.source || github.sha }}"@);
}

// The native workflow test checks the emitted guard at runtime.
#[cfg(verus_keep_ghost)]
fn push_only_cache_saving_has_an_exact_native_event_guard() {
    let push = policy::cache_save_event_guard(true);
    let every_event = policy::cache_save_event_guard(false);
    assert(push@ == "github.event_name == 'push' && "@);
    assert(every_event@ == ""@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn download_contracts_reject_each_missing_obligation() {
    let good = policy::valid_download_entry(1, 1, 1, 1, true, true, true, true);
    assert(good);
    let absent_paths = policy::valid_download_entry(0, 1, 1, 1, true, true, true, true);
    let absent_domain = policy::valid_download_entry(1, 0, 1, 1, true, true, true, true);
    let absent_sources = policy::valid_download_entry(1, 1, 0, 1, true, true, true, true);
    let absent_consumer = policy::valid_download_entry(1, 1, 1, 0, true, true, true, true);
    let duplicate = policy::valid_download_entry(1, 1, 1, 1, false, true, true, true);
    let bad_writer = policy::valid_download_entry(1, 1, 1, 1, true, false, true, true);
    let incompatible = policy::valid_download_entry(1, 1, 1, 1, true, true, false, true);
    let no_purpose = policy::valid_download_entry(1, 1, 1, 1, true, true, true, false);
    assert(!absent_paths && !absent_domain && !absent_sources && !absent_consumer
        && !duplicate && !bad_writer && !incompatible && !no_purpose);
    let missing = policy::single_download_binding(0);
    let one = policy::single_download_binding(1);
    let ambiguous = policy::single_download_binding(2);
    assert(!missing && one && !ambiguous);
    let empty = policy::valid_download_path(&[]);
    let path = policy::valid_download_path(&[97u8, 47, 98, 46, 108, 111, 99, 107]);
    let wildcard = policy::valid_download_path(&[97u8, 47, 42]);
    let quote = policy::valid_download_path(&[97u8, 39]);
    assert(!empty && path && !wildcard && !quote);
    let artifact = policy::valid_download_artifact(Some("p"), "p", Some("o"), "o", Some(false), Some("download-cache"));
    let tracked_result = policy::valid_download_artifact(Some("p"), "p", Some("o"), "o", Some(true), Some("download-cache"));
    let missing_kind = policy::valid_download_artifact(Some("p"), "p", Some("o"), "o", Some(false), None);
    let missing_owner = policy::valid_download_artifact(Some("p"), "p", None, "o", Some(false), Some("download-cache"));
    let wrong_owner = policy::valid_download_artifact(Some("p"), "p", Some("p"), "o", Some(false), Some("download-cache"));
    let evidence = policy::valid_download_artifact(Some("p"), "p", Some("o"), "o", Some(false), Some("execution-evidence"));
    proof {
        reveal_strlit("p"); reveal_strlit("o");
        reveal_strlit("download-cache"); reveal_strlit("execution-evidence");
        assert("p"@[0] != "o"@[0]);
        assert("download-cache"@.len() != "execution-evidence"@.len());
    }
    assert(artifact && !tracked_result && !missing_kind && !missing_owner && !wrong_owner && !evidence);
}

fn expect_cache_registry(schema: Option<&str>, primary_checkout: bool, expected: bool)
    requires expected == policy::cache_registry(schema, primary_checkout),
{
    let actual = policy::valid_cache_registry(schema, primary_checkout);
    assert(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_registry_requires_current_schema_and_primary_checkout() {
    proof {
        reveal_strlit("chrono-cache/v2"); reveal_strlit("chrono-cache/v1");
        reveal_strlit("chrono-cache/v20"); reveal_strlit("chrono-cache/v2\n");
        reveal_strlit("");
        assert("chrono-cache/v2"@[14] != "chrono-cache/v1"@[14]);
        assert("chrono-cache/v2"@.len() != "chrono-cache/v20"@.len());
        assert("chrono-cache/v2"@.len() != "chrono-cache/v2\n"@.len());
        assert("chrono-cache/v2"@.len() != ""@.len());
    }
    expect_cache_registry(Some("chrono-cache/v2"), true, true);
    expect_cache_registry(Some("chrono-cache/v2"), false, false);
    expect_cache_registry(Some("chrono-cache/v1"), true, false);
    expect_cache_registry(Some("chrono-cache/v20"), true, false);
    expect_cache_registry(Some("chrono-cache/v2\n"), true, false);
    expect_cache_registry(Some(""), true, false);
    expect_cache_registry(None, true, false);
    expect_cache_registry(None, false, false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_names_keep_the_ascii_alphabet_and_128_byte_limit() {
    let alphabet = policy::valid_cache_name(&[48u8, 57, 65, 90, 97, 122, 45, 95, 46, 58]);
    let longest = policy::valid_cache_name(&[97u8; 128]);
    let too_long = policy::valid_cache_name(&[97u8; 129]);
    let empty = policy::valid_cache_name(&[]);
    let slash = policy::valid_cache_name(&[97u8, 47, 98]);
    let space = policy::valid_cache_name(&[97u8, 32]);
    let unicode = policy::valid_cache_name(&[228u8, 184, 173]);
    let nul = policy::valid_cache_name(&[97u8, 0]);
    assert(alphabet && longest && !too_long && !empty && !slash && !space && !unicode && !nul);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_text_membership_preserves_full_fields_and_nonadjacent_duplicates() {
    proof {
        reveal_strlit("a"); reveal_strlit("bb"); reveal_strlit("ccc");
        assert("a"@.len() != "bb"@.len());
        assert("a"@.len() != "ccc"@.len());
        assert("bb"@.len() != "ccc"@.len());
    }
    let values = ["a", "bb", "ccc"];
    let duplicate = ["a", "bb", "a"];
    proof {
        assert(values@ =~= seq!["a", "bb", "ccc"]);
        assert(duplicate@ =~= seq!["a", "bb", "a"]);
    }
    let found = policy::cache_has_text(&values, "bb");
    let absent = policy::cache_has_text(&["bb", "ccc"], "a");
    let unique = policy::cache_has_unique_text(&values);
    let repeated = policy::cache_has_unique_text(&duplicate);
    let empty_unique = policy::cache_has_unique_text(&[]);
    let reordered = policy::cache_is_subset(&["ccc", "a"], &values);
    let outside = policy::cache_is_subset(&["a"], &["bb", "ccc"]);
    let no_saves = policy::cache_is_subset(&[], &values);
    assert(found && !absent && unique && !repeated && empty_unique && reordered && !outside && no_saves);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cache_preparation_keeps_empty_arguments_but_requires_a_literal_program() {
    proof {
        reveal_strlit("a"); reveal_strlit(""); reveal_strlit("${{");
        reveal_strlit("a\n"); reveal_strlit("a\r"); reveal_strlit("a\0");
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("a") && vstd::string::is_ascii("")
            && vstd::string::is_ascii("${{") && vstd::string::is_ascii("a\n")
            && vstd::string::is_ascii("a\r") && vstd::string::is_ascii("a\0"));
        assert("a".spec_bytes() =~= seq![97u8]);
        assert("".spec_bytes() =~= Seq::<u8>::empty());
        assert("${{".spec_bytes() =~= seq![36u8, 123, 123]);
        assert("a\n".spec_bytes() =~= seq![97u8, 10]);
        assert("a\r".spec_bytes() =~= seq![97u8, 13]);
        assert("a\0".spec_bytes() =~= seq![97u8, 0]);
    }
    let valid = policy::valid_cache_preparation(&["a", ""]);
    let no_program = policy::valid_cache_preparation(&[]);
    let empty_program = policy::valid_cache_preparation(&["", "a"]);
    let template = policy::valid_cache_preparation(&["a", "${{"]);
    let newline = policy::valid_cache_preparation(&["a", "a\n"]);
    let carriage = policy::valid_cache_preparation(&["a", "a\r"]);
    let nul = policy::valid_cache_preparation(&["a", "a\0"]);
    assert(valid && !no_program && !empty_program && !template && !newline && !carriage && !nul);
}

fn expect_native_cache_consumer(
    known_job: bool, name: &str, need: &str, output_use: &str, preparation: &[&str],
    caches: &[&str], saves: &[&str], after_bootstrap: &[&str], expected: bool,
)
    requires expected == policy::cache_consumer(known_job, name, need, output_use,
        preparation@, caches@, saves@, after_bootstrap@),
{
    let actual = policy::valid_cache_consumer(known_job, name, need, output_use,
        preparation, caches, saves, after_bootstrap);
    assert(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_cache_consumers_require_each_obligation_and_keep_read_only_jobs() {
    proof {
        reveal_strlit("a"); reveal_strlit("bb"); reveal_strlit("");
        assert("a"@.len() != "bb"@.len());
        broadcast use vstd::string::is_ascii_spec_bytes;
        assert(vstd::string::is_ascii("a") && vstd::string::is_ascii("bb")
            && vstd::string::is_ascii(""));
        assert("a".spec_bytes() =~= seq![97u8]);
        assert("bb".spec_bytes() =~= seq![98u8, 98]);
        assert("".spec_bytes() =~= Seq::<u8>::empty());
    }
    expect_native_cache_consumer(true, "a", "a", "a", &["a", ""], &["a", "bb"], &["bb", "a"], &["a"], true);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a"], &[], &[], true);
    expect_native_cache_consumer(false, "a", "a", "a", &["a"], &["a"], &[], &[], false);
    expect_native_cache_consumer(true, "", "a", "a", &["a"], &["a"], &[], &[], false);
    expect_native_cache_consumer(true, "a", "", "a", &["a"], &["a"], &[], &[], false);
    expect_native_cache_consumer(true, "a", "a", "", &["a"], &["a"], &[], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &[], &["a"], &[], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &[], &[], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a", "a"], &[], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a"], &["a", "a"], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a"], &["bb"], &[], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a"], &["a"], &["a", "a"], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a", "bb"], &["a"], &["bb"], false);
    expect_native_cache_consumer(true, "a", "a", "a", &["a"], &["a"], &[], &["a"], false);
}

fn expect(bytes: &[u8], expected: bool)
    requires expected == policy::literal(bytes@),
{
    let actual = policy::valid_literal(bytes);
    assert(actual == expected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_and_ordinary_utf8_text_are_accepted() {
    expect(&[], true);
    expect(&[97u8, 32, 9, 127], true);
    expect(&[228u8, 184, 173, 230, 150, 135], true);
    expect(&[36u8, 72, 79, 77, 69, 32, 96, 97, 96], true);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn forbidden_bytes_fail_at_both_ends_and_inside_text() {
    expect(&[0u8], false);
    expect(&[10u8], false);
    expect(&[13u8], false);
    expect(&[0u8, 97], false);
    expect(&[97u8, 10, 98], false);
    expect(&[97u8, 13], false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_template_openers_fail_at_every_boundary() {
    expect(&[36u8, 123, 123], false);
    expect(&[36u8, 123, 123, 97], false);
    expect(&[97u8, 36, 123, 123], false);
    expect(&[97u8, 36, 123, 123, 98], false);
    expect(&[36u8, 36, 123, 123], false);
    expect(&[36u8, 123, 36, 123, 123], false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn partial_openers_and_nonadjacent_bytes_are_accepted() {
    expect(&[36u8], true);
    expect(&[36u8, 123], true);
    expect(&[123u8, 123], true);
    expect(&[36u8, 123, 97, 123], true);
    expect(&[36u8, 123, 36, 123], true);
    expect(&[36u8, 123, 32, 123], true);
}

}
