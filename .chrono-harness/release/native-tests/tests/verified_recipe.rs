//! 独立列出既有发布配方的成功、失败和边界，不以新实现计算预期。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../native/src/verified_recipe.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_host_release::verified_recipe as policy;
use policy::{DependencyResult, Invocation};
use policy::delivery;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

// 同一预期在 Verus 中证明，在普通 Rust 中实际执行。
fn expect_recipe(actual: bool)
    requires actual,
{
    if !actual {
        panic!();
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_missing_fields_are_not_empty_or_zero_values() {
    proof { reveal_strlit(""); }
    let text = delivery::text_fields(&[(None, "")]);
    let number = delivery::number_fields(&[(None, 0)]);
    let boolean = delivery::boolean_fields(&[(None, false)]);
    let complete = delivery::number_fields(&[(Some(0), 0), (Some(u64::MAX), u64::MAX)]);
    let changed = delivery::number_fields(&[(Some(0), 0), (Some(1), u64::MAX)]);
    expect_recipe(!text && !number && !boolean && complete && !changed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_requires_all_full_text_fields() {
    proof { reveal_strlit("source"); reveal_strlit("source-extra"); }
    let exact = delivery::text_fields(&[(Some("source"), "source")]);
    let prefix = delivery::text_fields(&[(Some("source-extra"), "source")]);
    let missing = delivery::text_fields(&[(Some("source"), "source"), (None, "source")]);
    expect_recipe(exact && !prefix && !missing);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_decimal_ids_reject_zero_leading_zero_and_non_digits() {
    let positive = delivery::decimal_text(&[49u8,50], false);
    let component = delivery::decimal_text(&[48u8], true);
    let zero_id = delivery::decimal_text(&[48u8], false);
    let leading = delivery::decimal_text(&[48u8,49], true);
    let sign = delivery::decimal_text(&[45u8,49], false);
    let suffix = delivery::decimal_text(&[49u8,120], false);
    expect_recipe(positive && component && !zero_id && !leading && !sign && !suffix);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_public_versions_require_a_complete_draft_or_exact_existing_assets() {
    let draft = delivery::draft_state(Some(true), 99);
    let complete = delivery::draft_state(Some(false), 0);
    let incomplete = delivery::draft_state(Some(false), 1);
    let unknown = delivery::draft_state(None, 0);
    let zero = delivery::positive(Some(0));
    let maximum = delivery::positive(Some(u64::MAX));
    expect_recipe(draft == 1 && complete == 0 && incomplete == 2 && unknown == 2 && !zero && maximum);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_only_treats_the_original_explicit_404_as_absent() {
    proof { reveal_strlit("404"); reveal_strlit("Not Found"); reveal_strlit("403"); }
    let success = delivery::api_outcome(0, None, None, false);
    let absent = delivery::api_outcome(1, Some("404"), Some("Not Found"), true);
    let denied = delivery::api_outcome(1, Some("403"), Some("Not Found"), true);
    let forbidden_absence = delivery::api_outcome(1, Some("404"), Some("Not Found"), false);
    let failed = delivery::api_outcome(2, Some("404"), Some("Not Found"), true);
    expect_recipe(success == 0 && absent == 1 && denied == 2 && forbidden_absence == 2 && failed == 2);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_asset_names_do_not_allow_paths_or_parent_components() {
    let valid = delivery::name(&[97u8,45,49,46,95]);
    let parent = delivery::name(&[46u8,46]);
    let current = delivery::name(&[46u8]);
    let path = delivery::name(&[97u8,47,98]);
    let newline = delivery::name(&[97u8,10]);
    expect_recipe(valid && !parent && !current && !path && !newline);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_channel_and_requested_version_are_exact() {
    proof { reveal_strlit(""); reveal_strlit("build"); reveal_strlit("preview"); reveal_strlit("stable"); reveal_strlit("Preview"); reveal_strlit("v1.2.3"); reveal_strlit("v1.2.3-beta.1"); }
    let build = delivery::channel("");
    let preview = delivery::channel("preview");
    let stable = delivery::channel("stable");
    let unknown = delivery::channel("Preview");
    let derived = delivery::requested_version(1, "", "v1.2.3", "v1.2.3-beta.1");
    let override_version = delivery::requested_version(1, "v1.2.3", "v1.2.3", "v1.2.3");
    let final_version = delivery::requested_version(2, "v1.2.3", "v1.2.3", "v1.2.3");
    let prerelease = delivery::requested_version(2, "v1.2.3-beta.1", "v1.2.3", "v1.2.3-beta.1");
    expect_recipe(build == 0 && preview == 1 && stable == 2 && unknown == 3
        && derived && !override_version && final_version && !prerelease);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn frontend_header_requires_the_original_source_platform_and_github_role() {
    proof {
        reveal_strlit("chrono-release-frontend/v1"); reveal_strlit("other");
        reveal_strlit("commit"); reveal_strlit("other-commit");
        reveal_strlit("tree"); reveal_strlit("other-tree");
        reveal_strlit("system"); reveal_strlit("other-system");
        reveal_strlit("machine"); reveal_strlit("other-machine");
        assert("chrono-release-frontend/v1"@.len() != "other"@.len());
        assert("commit"@.len() != "other-commit"@.len());
        assert("tree"@.len() != "other-tree"@.len());
        assert("system"@.len() != "other-system"@.len());
        assert("machine"@.len() != "other-machine"@.len());
    }
    let same=policy::frontend_header("chrono-release-frontend/v1",true,"commit","tree","system","machine","commit","tree","system","machine");
    let schema=policy::frontend_header("other",true,"commit","tree","system","machine","commit","tree","system","machine");
    let role=policy::frontend_header("chrono-release-frontend/v1",false,"commit","tree","system","machine","commit","tree","system","machine");
    let commit=policy::frontend_header("chrono-release-frontend/v1",true,"other-commit","tree","system","machine","commit","tree","system","machine");
    let tree=policy::frontend_header("chrono-release-frontend/v1",true,"commit","other-tree","system","machine","commit","tree","system","machine");
    let system=policy::frontend_header("chrono-release-frontend/v1",true,"commit","tree","other-system","machine","commit","tree","system","machine");
    let machine=policy::frontend_header("chrono-release-frontend/v1",true,"commit","tree","system","other-machine","commit","tree","system","machine");
    expect_recipe(same && !schema && !role && !commit && !tree && !system && !machine);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn frontend_installation_requires_exactly_the_declared_tool_and_digest() {
    proof {
        reveal_strlit(".chrono-harness/bin/chrono-host-release"); reveal_strlit("other");
        reveal_strlit("hash"); reveal_strlit("other-hash");
        assert(".chrono-harness/bin/chrono-host-release"@.len() != "other"@.len());
        assert("hash"@.len() != "other-hash"@.len());
    }
    let same=policy::frontend_installation(1,1,".chrono-harness/bin/chrono-host-release",".chrono-harness/bin/chrono-host-release","hash","hash");
    let missing=policy::frontend_installation(1,0,".chrono-harness/bin/chrono-host-release",".chrono-harness/bin/chrono-host-release","hash","hash");
    let extra=policy::frontend_installation(2,1,".chrono-harness/bin/chrono-host-release",".chrono-harness/bin/chrono-host-release","hash","hash");
    let tool=policy::frontend_installation(1,1,"other","other","hash","hash");
    let path=policy::frontend_installation(1,1,".chrono-harness/bin/chrono-host-release","other","hash","hash");
    let hash=policy::frontend_installation(1,1,".chrono-harness/bin/chrono-host-release",".chrono-harness/bin/chrono-host-release","other-hash","hash");
    expect_recipe(same && !missing && !extra && !tool && !path && !hash);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn non_rust_verification_and_collection_do_not_require_rust() {
    let verification = policy::requires_rust(&Invocation::Verify { rust_toolchain: false });
    let collection = policy::requires_rust(&Invocation::Collect);
    expect_recipe(!verification && !collection);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn build_and_explicit_verification_require_rust() {
    let build = policy::requires_rust(&Invocation::Build);
    let verification = policy::requires_rust(&Invocation::Verify { rust_toolchain: true });
    expect_recipe(build && verification);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn executable_modes_require_a_regular_file_and_an_execute_bit() {
    proof {
        assert(493u32 & 73u32 == 73u32) by (bit_vector);
        assert(420u32 & 73u32 == 0u32) by (bit_vector);
        assert(1u32 & 73u32 == 1u32) by (bit_vector);
        assert(4095u32 & 73u32 == 73u32) by (bit_vector);
    }
    let normal = policy::executable_file(true, 493);
    let data = policy::executable_file(true, 420);
    let directory = policy::executable_file(false, 493);
    let lowest = policy::executable_file(true, 1);
    let special = policy::executable_file(true, 4095);
    let unknown = policy::executable_file(true, 4096);
    expect_recipe(normal && !data && !directory && lowest && special && !unknown);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn executable_identity_requires_the_full_digest_size_and_mode() {
    proof {
        reveal_strlit("hash"); reveal_strlit("hashx"); reveal_strlit("Hash");
        assert("hash"@.len() != "hashx"@.len());
        assert("hash"@[0] != "Hash"@[0]);
    }
    let same = policy::same_executable("hash", 7, 489, "hash", 7, 489);
    let digest = policy::same_executable("Hash", 7, 489, "hash", 7, 489);
    let prefix = policy::same_executable("hashx", 7, 489, "hash", 7, 489);
    let size = policy::same_executable("hash", 8, 489, "hash", 7, 489);
    let mode = policy::same_executable("hash", 7, 493, "hash", 7, 489);
    expect_recipe(same && !digest && !prefix && !size && !mode);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn tar_entry_requires_the_exact_regular_name_size_and_mode() {
    let name = [101u8,120,101,99,117,116,97,98,108,101];
    proof { assert(name@ =~= seq![101u8,120,101,99,117,116,97,98,108,101]); }
    let same = policy::executable_entry(&name, true, 7, 489, 7, 489);
    let link = policy::executable_entry(&name, false, 7, 489, 7, 489);
    let size = policy::executable_entry(&name, true, 8, 489, 7, 489);
    let mode = policy::executable_entry(&name, true, 7, 493, 7, 489);
    let traversal = [46u8,46,47,101,120,101,99,117,116,97,98,108,101];
    let path = policy::executable_entry(&traversal, true, 7, 489, 7, 489);
    let empty: [u8; 0] = [];
    proof { vstd::array::array_len_matches_n(&empty); }
    let absent = policy::executable_entry(&empty, true, 7, 489, 7, 489);
    expect_recipe(same);
    expect_recipe(!link);
    expect_recipe(!size);
    expect_recipe(!mode);
    expect_recipe(!path);
    expect_recipe(!absent);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn child_failure_wins_over_forwarding_retention_and_receipt_failures() {
    let ordinary = policy::result_exit(Some(41), false);
    let auxiliary = policy::result_exit(Some(41), true);
    let lowest = policy::result_exit(Some(1), true);
    let highest = policy::result_exit(Some(255), true);
    expect_recipe(ordinary == 41 && auxiliary == 41 && lowest == 1 && highest == 255);
    let success = policy::passed(Some(41), true);
    expect_recipe(!success);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn successful_child_fails_when_evidence_cannot_be_published() {
    let success = policy::result_exit(Some(0), false);
    let failure = policy::result_exit(Some(0), true);
    let passed = policy::passed(Some(0), false);
    let rejected = policy::passed(Some(0), true);
    expect_recipe(success == 0 && failure == 2 && passed && !rejected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_child_status_is_never_success() {
    let no_error = policy::result_exit(None, false);
    let with_error = policy::result_exit(None, true);
    let passed = policy::passed(None, false);
    expect_recipe(no_error == 2 && with_error == 2 && !passed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn child_signal_survives_auxiliary_io_failures() {
    let killed = policy::result_exit(Some(-9), false);
    let killed_with_io_failure = policy::result_exit(Some(-9), true);
    let first = policy::result_exit(Some(-1), true);
    let last = policy::result_exit(Some(-127), true);
    expect_recipe(killed == 137 && killed_with_io_failure == 137 && first == 129 && last == 255);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn old_success_cannot_hide_current_failed_cancelled_or_skipped_needs() {
    let failed = policy::selected_producer(&DependencyResult::Failure, "run", "run", 1, 2, "artifact");
    let cancelled = policy::selected_producer(&DependencyResult::Cancelled, "run", "run", 1, 2, "artifact");
    let skipped = policy::selected_producer(&DependencyResult::Skipped, "run", "run", 1, 2, "artifact");
    expect_recipe(!failed && !cancelled && !skipped);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn successful_current_needs_can_select_the_original_earlier_attempt() {
    proof { reveal_strlit("run"); reveal_strlit("artifact"); }
    let carried = policy::selected_producer(&DependencyResult::Success, "run", "run", 1, 2, "artifact");
    let same = policy::selected_producer(&DependencyResult::Success, "run", "run", 2, 2, "artifact");
    let large = policy::selected_producer(&DependencyResult::Success, "run", "run", u64::MAX, u64::MAX, "artifact");
    expect_recipe(carried && same && large);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_future_or_foreign_producers_are_rejected() {
    proof {
        reveal_strlit("run"); reveal_strlit("foreign"); reveal_strlit("");
        assert("run"@.len() != "foreign"@.len());
    }
    let zero = policy::selected_producer(&DependencyResult::Success, "run", "run", 0, 2, "artifact");
    let future = policy::selected_producer(&DependencyResult::Success, "run", "run", 3, 2, "artifact");
    let foreign = policy::selected_producer(&DependencyResult::Success, "foreign", "run", 1, 2, "artifact");
    let absent_run = policy::selected_producer(&DependencyResult::Success, "", "", 1, 2, "artifact");
    let absent_artifact = policy::selected_producer(&DependencyResult::Success, "run", "run", 1, 2, "");
    expect_recipe(!zero && !future && !foreign && !absent_run && !absent_artifact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn receipt_job_run_and_attempt_must_match_the_selected_producer() {
    proof {
        reveal_strlit("job"); reveal_strlit("other"); reveal_strlit("run"); reveal_strlit("changed");
        reveal_strlit("");
        assert("job"@.len() != "other"@.len());
        assert("run"@.len() != "changed"@.len());
    }
    let valid = policy::producer_lineage("job", "job", "run", "run", 1, 1);
    let job = policy::producer_lineage("other", "job", "run", "run", 1, 1);
    let run = policy::producer_lineage("job", "job", "changed", "run", 1, 1);
    let attempt = policy::producer_lineage("job", "job", "run", "run", 1, 2);
    let zero = policy::producer_lineage("job", "job", "run", "run", 0, 0);
    let empty_job = policy::producer_lineage("", "", "run", "run", 1, 1);
    let empty_run = policy::producer_lineage("job", "job", "", "", 1, 1);
    expect_recipe(valid && !job && !run && !attempt && !zero && !empty_job && !empty_run);
}

}
