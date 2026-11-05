//! Verified success, rejection and boundary expectations for the actual predicate.
#![allow(unexpected_cfgs)]
#[path = "../../judge-ci/src/verus_policy.rs"]
mod policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;

verus! {

broadcast use vstd::string::group_string_axioms;

#[cfg_attr(not(verus_keep_ghost), test)]
fn root_declarations_reject_missing_empty_and_malformed_lists() {
    proof { reveal_strlit("root"); reveal_strlit("verify.root"); }
    let registered = vec!["root"];
    let routes = vec![policy::RootRoute {
        root: "root", operation: "verify.root", globally_selected: false, locally_required: false,
    }];
    let empty: Vec<Option<&str>> = Vec::new();
    let malformed = vec![Some("root"), None];
    let missing = policy::admit_declared_roots(None, &registered, &routes);
    let empty_result = policy::admit_declared_roots(Some(&empty), &registered, &routes);
    let malformed_result = policy::admit_declared_roots(Some(&malformed), &registered, &routes);
    assert(!missing && !empty_result && !malformed_result);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn declared_roots_require_both_registered_files_and_unique_routes() {
    proof { reveal_strlit("root"); reveal_strlit("verify.root"); }
    let declared = vec![Some("root")];
    let registered = vec!["root"];
    let no_files: Vec<&str> = Vec::new();
    let no_routes: Vec<policy::RootRoute> = Vec::new();
    let routes = vec![policy::RootRoute {
        root: "root", operation: "verify.root", globally_selected: false, locally_required: false,
    }];
    let ambiguous = vec![
        policy::RootRoute { root: "root", operation: "verify.root", globally_selected: false, locally_required: false },
        policy::RootRoute { root: "root", operation: "verify.root", globally_selected: true, locally_required: true },
    ];
    let no_file = policy::admit_declared_roots(Some(&declared), &no_files, &routes);
    let no_route = policy::admit_declared_roots(Some(&declared), &registered, &no_routes);
    let multiple = policy::admit_declared_roots(Some(&declared), &registered, &ambiguous);
    assert(!no_file && !no_route && !multiple);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn root_declaration_membership_uses_unicode_values_and_allows_duplicate_mentions() {
    proof { reveal_strlit("证明根"); reveal_strlit("操作"); }
    let root = "证明根".to_owned();
    let declared = vec![Some(root.as_str()), Some("证明根")];
    let registered = vec!["证明根", root.as_str()];
    let routes = vec![policy::RootRoute {
        root: "证明根", operation: "操作", globally_selected: false, locally_required: false,
    }];
    assert(policy::unique_root_route(routes@, "证明根"@, 0));
    assert(policy::text_member(registered@, "证明根"@));
    assert forall|i: int| 0 <= i < declared.len()
        implies policy::declared_root_valid(#[trigger] declared@[i], registered@, routes@) by {
        assert(i == 0 || i == 1);
        assert(declared@[i].unwrap()@ == "证明根"@);
    }
    let valid = policy::admit_declared_roots(Some(&declared), &registered, &routes);
    assert(valid);
    // Declaration validity does not claim DELTA execution or proof admission.
    let no_proofs: Vec<policy::RootProof> = Vec::new();
    let covered = policy::covers_root_delta(&routes, &no_proofs, "证明根");
    assert(!covered);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unit_verifiers_require_the_units_own_action_operation() {
    proof { reveal_strlit("root"); reveal_strlit("操作"); reveal_strlit("other"); }
    let routes = vec![policy::RootRoute {
        root: "root", operation: "操作", globally_selected: true, locally_required: true,
    }];
    let empty: Vec<Option<&str>> = Vec::new();
    let unrelated = vec![None, Some("root"), Some("other")];
    let operation = "操作".to_owned();
    let own = vec![None, Some(operation.as_str()), Some("操作")];
    let no_routes: Vec<policy::RootRoute> = Vec::new();
    assert("root"@.len() != "操作"@.len());
    assert("other"@.len() != "操作"@.len());
    let absent = policy::admits_unit_verifier(&empty, &routes);
    let wrong = policy::admits_unit_verifier(&unrelated, &routes);
    let unregistered = policy::admits_unit_verifier(&own, &no_routes);
    let valid = policy::admits_unit_verifier(&own, &routes);
    assert(!absent && !wrong && !unregistered && valid);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn root_plan_binding_requires_registered_operations_without_inferring_selection() {
    proof { reveal_strlit("test:root"); reveal_strlit("verify.root"); reveal_strlit("root"); }
    let empty: Vec<&str> = Vec::new();
    let no_plans: Vec<policy::RootPlan> = Vec::new();
    let absent = policy::bind_root_route("root", "verify.root", &no_plans, &empty, &empty);
    assert(absent.is_none());
    let plans = vec![policy::RootPlan { test: "test:root", operations: vec!["verify.root"] }];
    let unselected = policy::bind_root_route("root", "verify.root", &plans, &empty, &empty);
    assert(unselected.is_some());
    let route = unselected.unwrap();
    assert(route.root@ == "root"@ && route.operation@ == "verify.root"@);
    assert(!route.globally_selected && !route.locally_required);
    let operation_name = vec!["verify.root"];
    assert("verify.root"@[0] != "test:root"@[0]);
    let wrong_selection = policy::bind_root_route("root", "verify.root", &plans, &operation_name, &operation_name);
    assert(wrong_selection.is_some());
    let route = wrong_selection.unwrap();
    assert(!route.globally_selected && !route.locally_required);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn root_plan_binding_separates_shared_local_and_other_unit_operations() {
    proof {
        reveal_strlit("alpha"); reveal_strlit("beta");
        reveal_strlit("shared"); reveal_strlit("local"); reveal_strlit("absent");
    }
    let plans = vec![
        policy::RootPlan { test: "alpha", operations: vec!["shared", "local"] },
        policy::RootPlan { test: "beta", operations: vec!["shared"] },
    ];
    let global = vec!["beta"];
    let local = vec!["alpha"];
    assert("alpha"@.len() != "beta"@.len());
    assert("shared"@.len() != "local"@.len());
    assert("absent"@[0] != "shared"@[0] && "absent"@[0] != "local"@[0]);
    let shared = policy::bind_root_route("root", "shared", &plans, &global, &local).unwrap();
    let local_only = policy::bind_root_route("root", "local", &plans, &global, &local).unwrap();
    let absent = policy::bind_root_route("root", "absent", &plans, &global, &local);
    assert(shared.globally_selected && shared.locally_required);
    assert(!local_only.globally_selected && local_only.locally_required);
    assert(absent.is_none());
    let empty: Vec<&str> = Vec::new();
    let deferred = policy::bind_root_route("root", "shared", &plans, &global, &empty).unwrap();
    assert(deferred.globally_selected && !deferred.locally_required);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn root_plan_binding_preserves_unicode_value_membership_and_duplicate_declarations() {
    proof { reveal_strlit("单元"); reveal_strlit("证明"); reveal_strlit("根"); }
    let unit = "单元".to_owned();
    let operation = "证明".to_owned();
    let plans = vec![
        policy::RootPlan { test: unit.as_str(), operations: vec![operation.as_str(), "证明"] },
        policy::RootPlan { test: "单元", operations: vec!["证明"] },
    ];
    let selected = vec!["单元", unit.as_str()];
    let bound = policy::bind_root_route("根", "证明", &plans, &selected, &selected).unwrap();
    assert(bound.root@ == "根"@ && bound.operation@ == "证明"@);
    assert(bound.globally_selected && bound.locally_required);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn root_delta_requires_global_selection_and_the_locally_owned_proof() {
    proof { reveal_strlit("root"); reveal_strlit("verify.root"); }
    let none: Vec<policy::RootProof> = Vec::new();
    let unselected = vec![policy::RootRoute {
        root: "root", operation: "verify.root", globally_selected: false, locally_required: false,
    }];
    let deferred = vec![policy::RootRoute {
        root: "root", operation: "verify.root", globally_selected: true, locally_required: false,
    }];
    let required = vec![policy::RootRoute {
        root: "root", operation: "verify.root", globally_selected: true, locally_required: true,
    }];
    assert(policy::unique_root_route(deferred@, "root"@, 0));
    assert(policy::unique_root_route(required@, "root"@, 0));
    let missing_selection = policy::covers_root_delta(&unselected, &none, "root");
    let other_unit = policy::covers_root_delta(&deferred, &none, "root");
    let missing_proof = policy::covers_root_delta(&required, &none, "root");
    let proof = vec![policy::RootProof { operation: "verify.root".to_owned(), admitted: true }];
    assert(policy::root_proofs_admitted(proof@, "verify.root"@));
    let complete = policy::covers_root_delta(&required, &proof, "root");
    assert(!missing_selection && other_unit && !missing_proof && complete);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn root_routes_reject_absence_and_ambiguity_even_with_successful_proofs() {
    proof { reveal_strlit("root"); reveal_strlit("verify.root"); }
    let no_routes: Vec<policy::RootRoute> = Vec::new();
    let proofs = vec![policy::RootProof { operation: "verify.root".to_owned(), admitted: true }];
    let absent = policy::covers_root_delta(&no_routes, &proofs, "root");
    let ambiguous = vec![
        policy::RootRoute { root: "root", operation: "verify.root", globally_selected: true, locally_required: true },
        policy::RootRoute { root: "root", operation: "verify.root", globally_selected: true, locally_required: true },
    ];
    let rejected = policy::covers_root_delta(&ambiguous, &proofs, "root");
    assert(!absent && !rejected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn root_proofs_require_exact_operation_and_preserve_shared_original_failures() {
    proof { reveal_strlit("verify.root"); reveal_strlit("other"); }
    assert("verify.root"@.len() != "other"@.len());
    let unrelated = vec![policy::RootProof { operation: "other".to_owned(), admitted: true }];
    let absent = policy::admits_root_proofs(&unrelated, "verify.root");
    let shared = vec![
        policy::RootProof { operation: "verify.root".to_owned(), admitted: true },
        policy::RootProof { operation: "other".to_owned(), admitted: false },
        policy::RootProof { operation: "verify.root".to_owned(), admitted: true },
    ];
    let accepted = policy::admits_root_proofs(&shared, "verify.root");
    let mixed = vec![
        policy::RootProof { operation: "verify.root".to_owned(), admitted: true },
        policy::RootProof { operation: "verify.root".to_owned(), admitted: false },
    ];
    let failure = policy::admits_root_proofs(&mixed, "verify.root");
    assert(!absent && accepted && !failure);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn root_identity_uses_unicode_value_and_accepts_proof_outside_route_order() {
    proof { reveal_strlit("根"); reveal_strlit("检查"); reveal_strlit("other"); }
    assert("根"@.len() != "other"@.len());
    let owned = "根".to_owned();
    let routes = vec![
        policy::RootRoute { root: "other", operation: "other", globally_selected: false, locally_required: false },
        policy::RootRoute { root: owned.as_str(), operation: "检查", globally_selected: true, locally_required: true },
    ];
    let proofs = vec![
        policy::RootProof { operation: "检查".to_owned(), admitted: true },
        policy::RootProof { operation: "other".to_owned(), admitted: false },
    ];
    assert(policy::unique_root_route(routes@, "根"@, 1));
    assert(policy::root_proofs_admitted(proofs@, "检查"@));
    let accepted = policy::covers_root_delta(&routes, &proofs, "根");
    let other = policy::covers_root_delta(&routes, &proofs, "other");
    assert(accepted && !other);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observation_identity_binds_candidate_root_and_both_artifact_labels() {
    let candidate = [97u8];
    let root = [98u8];
    let output = [99u8];
    let dependency = [99u8, 46, 100];
    let observed = policy::sources::ObservationIdentity {
        candidate: &candidate, root: &root, library: &output, dependency: &dependency,
    };
    assert(dependency@ =~= output@ + seq![46u8, 100u8]);
    let accepted = policy::sources::matches_observation_identity(&observed, &candidate, &root, &output);
    let changed_candidate = policy::sources::matches_observation_identity(&observed, &[100u8], &root, &output);
    let changed_root = policy::sources::matches_observation_identity(&observed, &candidate, &[100u8], &output);
    let changed_output = policy::sources::matches_observation_identity(&observed, &candidate, &root, &[100u8]);
    assert(accepted && !changed_candidate && !changed_root && !changed_output);
    let missing_suffix = policy::sources::ObservationIdentity {
        candidate: &candidate, root: &root, library: &output, dependency: &output,
    };
    let rejected = policy::sources::matches_observation_identity(&missing_suffix, &candidate, &root, &output);
    assert(!rejected);
    let empty: [u8; 0] = [];
    let suffix = [46u8, 100];
    let empty_observed = policy::sources::ObservationIdentity {
        candidate: &empty, root: &empty, library: &empty, dependency: &suffix,
    };
    assert(suffix@ =~= empty@ + seq![46u8, 100u8]);
    let empty_equal = policy::sources::matches_observation_identity(&empty_observed, &empty, &empty, &empty);
    assert(empty_equal); // Identity equality alone makes no eligibility claim.
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_requires_inventory_operations_and_adopted_coverage() {
    proof {
        reveal_strlit("ci.inventory");
        reveal_strlit("verify.root");
        reveal_strlit("test:other");
        reveal_strlit("ci.verus-coverage");
    }
    let operations = vec!["verify.root"];
    let not_required = vec!["test:other"];
    let complete = vec!["test:other", "ci.verus-coverage", "verify.root", "ci.inventory"];
    assert(policy::text_values(operations@) =~= seq!["verify.root"@]);
    assert(policy::text_values(not_required@) =~= seq!["test:other"@]);
    assert(policy::expected_result_ids(operations@, not_required@, true)
        =~= seq!["ci.inventory"@, "verify.root"@, "test:other"@, "ci.verus-coverage"@]);
    let accepted = policy::matches_result_ids(&complete, &operations, &not_required, true, Some((true, None)));
    assert("ci.inventory"@[0] != "verify.root"@[0]);
    assert("ci.inventory"@[0] != "test:other"@[0]);
    assert("ci.inventory"@.len() != "ci.verus-coverage"@.len());
    assert(accepted);
    let no_coverage = vec!["ci.inventory", "verify.root", "test:other"];
    let missing = policy::matches_result_ids(&no_coverage, &operations, &not_required, true, None);
    let unadopted = policy::matches_result_ids(&no_coverage, &operations, &not_required, false, None);
    let extra_coverage = policy::matches_result_ids(&complete, &operations, &not_required, false, None);
    assert(!missing && unadopted && !extra_coverage);
    let no_inventory = vec!["verify.root", "test:other", "ci.verus-coverage"];
    let missing_inventory = policy::matches_result_ids(&no_inventory, &operations, &not_required, true, Some((true, None)));
    assert(!missing_inventory);
    let absent = policy::matches_result_ids(&complete, &operations, &not_required, true, None);
    let unpassed = policy::matches_result_ids(&complete, &operations, &not_required, true, Some((false, None)));
    let process_exit = policy::matches_result_ids(&complete, &operations, &not_required, true, Some((true, Some(0))));
    assert(!absent && !unpassed && !process_exit);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_preserves_set_semantics_unicode_and_empty_boundaries() {
    proof { reveal_strlit("ci.inventory"); }
    let empty: Vec<&str> = Vec::new();
    let empty_membership = policy::matches_text_membership(&empty, &empty);
    let missing_inventory = policy::matches_result_ids(&empty, &empty, &empty, false, None);
    let inventory = vec!["ci.inventory"];
    assert(inventory[0]@ == "ci.inventory"@);
    assert(policy::expected_result_ids(empty@, empty@, false) =~= seq!["ci.inventory"@]);
    assert(policy::expected_result_ids(empty@, empty@, false)[0] == "ci.inventory"@);
    assert(!policy::text_member(empty@, "ci.inventory"@));
    let inventory_only = policy::matches_result_ids(&inventory, &empty, &empty, false, None);
    assert(empty_membership);
    assert(!missing_inventory);
    assert(inventory_only);
    let values = vec!["接口", ""];
    let reordered_duplicates = vec!["", "接口", "接口"];
    let same = policy::matches_text_membership(&values, &reordered_duplicates);
    let absent = policy::matches_text_membership(&values, &empty);
    let extra = policy::matches_text_membership(&empty, &values);
    assert(same && !absent && !extra);
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
fn source_metadata_requires_normalized_identity_digest_length_root_and_claims() {
    reveal_with_fuel(policy::sources::path_scan, 8);
    reveal_with_fuel(policy::sources::joined_components, 3);
    let raw = metadata_fixture(vec![46u8, 47, 97], 7);
    let fixed = metadata_fixture(vec![97u8], 7);
    let sources = vec![raw];
    let candidate = vec![fixed];
    let claims = vec![vec![97u8]];
    let root = [97u8];
    assert(sources[0].path@ == seq![46u8, 47u8, 97u8]);
    assert(candidate[0].path@ == seq![97u8]);
    assert(sources[0].sha256@ =~= candidate[0].sha256@);
    assert(policy::sources::source_pair_matches(&sources[0], &candidate[0]));
    assert(policy::sources::listed_source_path(candidate@, root@, 1));
    assert(!policy::sources::listed_source_path(candidate@, candidate[0].path@, 0));
    assert(policy::sources::source_metadata_admitted(sources@, candidate@, root@, claims@));
    let accepted = policy::sources::admit_source_metadata(&sources, &candidate, &root, &claims);
    assert(accepted);
    let changed_candidate = vec![metadata_fixture(vec![97u8], 8)];
    let changed_length = policy::sources::admit_source_metadata(&sources, &changed_candidate, &root, &claims);
    assert(!changed_length);
    let mut changed = metadata_fixture(vec![46u8, 47, 97], 7);
    changed.sha256[0] = 65u8;
    let changed_sources = vec![changed];
    assert(changed_sources[0].sha256[0] == 65u8);
    assert(candidate[0].sha256[0] == 48u8);
    assert(!policy::sources::source_pair_matches(&changed_sources[0], &candidate[0]));
    let changed_digest = policy::sources::admit_source_metadata(&changed_sources, &candidate, &root, &claims);
    assert(!changed_digest);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_metadata_rejects_missing_claims_roots_ambiguity_and_empty_inventories() {
    reveal_with_fuel(policy::sources::path_scan, 4);
    reveal_with_fuel(policy::sources::joined_components, 3);
    let mut sources = vec![metadata_fixture(vec![97u8], 0)];
    let mut candidate = vec![metadata_fixture(vec![97u8], 0)];
    let claims = vec![vec![98u8]];
    let empty_claims: Vec<Vec<u8>> = Vec::new();
    let missing_claim = policy::sources::admit_source_metadata(&sources, &candidate, &[97u8], &claims);
    let missing_root = policy::sources::admit_source_metadata(&sources, &candidate, &[98u8], &empty_claims);
    assert(!missing_claim && !missing_root);
    sources.push(metadata_fixture(vec![46u8, 47, 97], 0));
    candidate.push(metadata_fixture(vec![97u8], 0));
    assert(candidate[0].path@ == candidate[1].path@);
    assert(policy::sources::listed_source_path(candidate@, candidate[1].path@, 1));
    let duplicate = policy::sources::admit_source_metadata(&sources, &candidate, &[97u8], &empty_claims);
    assert(!duplicate);
    let empty: Vec<policy::sources::SourceBinding> = Vec::new();
    let no_sources = policy::sources::admit_source_metadata(&empty, &empty, &[97u8], &empty_claims);
    let unequal = policy::sources::admit_source_metadata(&sources, &empty, &[97u8], &empty_claims);
    assert(!no_sources && !unequal);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_sha256_text_requires_exact_lowercase_hex_length() {
    let mut hash: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < 64
        invariant 0 <= i <= 64, hash.len() == i,
            forall|j: int| 0 <= j < hash.len() ==> (#[trigger] hash[j]) == 48u8,
        decreases 64 - i,
    {
        hash.push(48u8);
        i += 1;
    }
    let zero = policy::sources::valid_sha256_text(hash.as_slice());
    assert(zero);
    let _ = hash.pop();
    let short = policy::sources::valid_sha256_text(hash.as_slice());
    assert(!short);
    hash.push(65u8);
    let uppercase = policy::sources::valid_sha256_text(hash.as_slice());
    assert(hash@[63] == 65u8);
    assert(!uppercase);
    hash[63] = 102u8;
    let hex_end = policy::sources::valid_sha256_text(hash.as_slice());
    assert(hex_end);
    hash[63] = 103u8;
    let beyond_hex = policy::sources::valid_sha256_text(hash.as_slice());
    assert(hash@[63] == 103u8);
    assert(!beyond_hex);
    hash.push(48u8);
    let long = policy::sources::valid_sha256_text(hash.as_slice());
    assert(!long);
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
    reveal_with_fuel(policy::sources::path_scan, 10);
    reveal(policy::sources::component_step);
    reveal_with_fuel(policy::sources::joined_components, 4);
    let path = [97u8, 32, 47, 206, 187];
    assert(path@ =~= seq![97u8, 32u8, 47u8, 206u8, 187u8]);
    let normalized = policy::sources::normalize_relative_path(&path).unwrap();
    assert(normalized@ == path@);
    let nul = policy::sources::normalize_relative_path(&[97u8, 0]);
    let lf = policy::sources::normalize_relative_path(&[97u8, 10]);
    let del = policy::sources::normalize_relative_path(&[97u8, 127]);
    let backslash = policy::sources::normalize_relative_path(&[97u8, 92, 98]);
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
    reveal_with_fuel(policy::dependency_escape, 3);
    reveal_with_fuel(policy::dependency_encoding, 3);
    reveal_with_fuel(policy::sources::phony_rules, 3);
    assert(policy::sources::source_paths(sources@) =~= seq![seq![97u8]]);
    assert(sources@.drop_last() =~= Seq::<policy::sources::SourceBinding>::empty());
    assert(policy::dependency_encoding(policy::sources::source_paths(sources@)) =~= seq![97u8]);
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
    reveal_with_fuel(policy::dependency_scan, 5);
    assert(input@.subrange(0, 5) =~= policy::sources::target_prefix(output@));
    assert(input@.subrange(5, 8) =~= seq![97u8, 32u8, 98u8]);
    assert(policy::dependency_scan(seq![97u8, 32u8, 98u8], Seq::empty(), Seq::empty())
        == Some(seq![seq![97u8], seq![98u8]]));
    assert(policy::sources::compiler_paths(input@, output@) == Some(seq![seq![97u8], seq![98u8]]));
    assert(parsed.is_some());
    let paths = parsed.unwrap();
    assert(policy::dependency_values(paths@) == seq![seq![97u8], seq![98u8]]);
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
    reveal_with_fuel(policy::dependency_escape, 4);
    assert(policy::sources::decimal_text(10) =~= seq![49u8, 48u8]);
    assert(policy::dependency_escape(source.path@) =~= seq![97u8, 92u8, 32u8, 98u8]);
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
fn complete_inventory_is_accepted() {
    let mut ledger = policy::Ledger::new();
    ledger.record("first".to_owned(), true);
    ledger.record("last".to_owned(), true);
    let coverage = ledger.finish();
    assert(coverage.obligations == 2);
    assert(coverage.complete);
    assert(coverage.missing.len() == 0);
    let first = coverage.first_missing();
    assert(first.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_inventory_is_rejected() {
    let ledger = policy::Ledger::new();
    let coverage = ledger.finish();
    assert(coverage.obligations == 0);
    assert(!coverage.complete);
    assert(coverage.missing.len() == 0);
    let first = coverage.first_missing();
    assert(first.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn every_failed_identity_is_retained_in_order() {
    let mut ledger = policy::Ledger::new();
    ledger.record("first".to_owned(), false);
    ledger.record("accepted".to_owned(), true);
    ledger.record("middle".to_owned(), false);
    ledger.record("last".to_owned(), false);
    let coverage = ledger.finish();
    reveal_with_fuel(policy::rejected_identities, 5);
    assert(coverage.obligations == 4);
    assert(!coverage.complete);
    assert(coverage.missing.len() == 3);
    assert(coverage.missing[0]@ == "first"@);
    assert(coverage.missing[1]@ == "middle"@);
    assert(coverage.missing[2]@ == "last"@);
    let first = coverage.first_missing();
    assert(first.is_some() && first.unwrap()@ == "first"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn duplicate_and_empty_identities_are_not_conflated() {
    let mut ledger = policy::Ledger::new();
    ledger.record("same".to_owned(), false);
    ledger.record("".to_owned(), false);
    ledger.record("same".to_owned(), true);
    ledger.record("same".to_owned(), false);
    let coverage = ledger.finish();
    reveal_with_fuel(policy::rejected_identities, 5);
    assert(coverage.obligations == 4);
    assert(!coverage.complete);
    assert(coverage.missing.len() == 3);
    assert(coverage.missing[0]@ == "same"@);
    assert(coverage.missing[1]@ == ""@);
    assert(coverage.missing[2]@ == "same"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn appending_records_preserves_prior_decisions() {
    let mut ledger = policy::Ledger::new();
    ledger.record("before".to_owned(), false);
    let ghost before = ledger.view();
    ledger.record("after".to_owned(), true);
    assert(ledger.view().take(1) =~= before);
    let coverage = ledger.finish();
    reveal_with_fuel(policy::rejected_identities, 3);
    assert(coverage.obligations == 2);
    assert(!coverage.complete);
    assert(coverage.missing.len() == 1);
    assert(coverage.missing[0]@ == "before"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registered_text_requires_exact_identity() {
    proof {
        reveal_strlit("verus");
        reveal_strlit("git");
        reveal_strlit("Verus");
        reveal_strlit("ver");
        reveal_strlit("");
        reveal_strlit("接口");
    }
    let registered = vec!["verus", "git", "verus"];
    let present = policy::contains_text(&registered, "verus");
    let absent = policy::contains_text(&registered, "Verus");
    let prefix = policy::contains_text(&registered, "ver");
    let empty: Vec<&str> = Vec::new();
    let no_entries = policy::contains_text(&empty, "");
    assert(registered[0]@ == "verus"@);
    assert("verus"@[0] != "Verus"@[0]);
    assert("git"@[0] != "Verus"@[0]);
    assert("verus"@.len() != "ver"@.len());
    assert("git"@[0] != "ver"@[0]);
    assert(present);
    assert(!absent);
    assert(!prefix);
    assert(!no_entries);
    let unicode = vec!["接口", ""];
    let exact_unicode = policy::contains_text(&unicode, "接口");
    let exact_empty = policy::contains_text(&unicode, "");
    assert(exact_unicode && exact_empty);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_decode_only_escaped_spaces() {
    let encoded = [97u8, 92u8, 32u8, 98u8];
    let parsed = policy::parse_dependency_paths(&encoded);
    reveal_with_fuel(policy::dependency_scan, 5);
    assert(parsed.is_some());
    let paths = parsed.unwrap();
    assert(paths.len() == 1);
    assert(paths[0]@ == seq![97u8, 32u8, 98u8]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_preserve_order_and_duplicates() {
    reveal_with_fuel(policy::dependency_scan, 6);
    let encoded = [97u8, 32u8, 98u8, 32u8, 97u8];
    let parsed = policy::parse_dependency_paths(&encoded);
    assert(parsed.is_some());
    let paths = parsed.unwrap();
    assert(encoded@ =~= seq![97u8, 32u8, 98u8, 32u8, 97u8]);
    assert(policy::dependency_scan(encoded@, Seq::empty(), Seq::empty())
        == Some(seq![seq![97u8], seq![98u8], seq![97u8]]));
    assert(policy::dependency_values(paths@) == seq![seq![97u8], seq![98u8], seq![97u8]]);
    assert(paths.len() == 3);
    assert(paths[0]@ == seq![97u8]);
    assert(paths[1]@ == seq![98u8]);
    assert(paths[2]@ == seq![97u8]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_preserve_relative_components_and_utf8_bytes() {
    reveal_with_fuel(policy::dependency_scan, 5);
    let relative = [46u8, 46u8, 47u8, 99u8];
    let parsed = policy::parse_dependency_paths(&relative);
    assert(parsed.is_some());
    let paths = parsed.unwrap();
    assert(paths.len() == 1 && paths[0]@ == relative@);
    let unicode = [206u8, 187u8];
    let unchanged = policy::parse_dependency_paths(&unicode);
    assert(unchanged.is_some());
    let unchanged = unchanged.unwrap();
    assert(unchanged.len() == 1 && unchanged[0]@ == unicode@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_dependency_paths_reject_empty_words_controls_and_unknown_escapes() {
    reveal_with_fuel(policy::dependency_scan, 8);
    let empty: [u8; 0] = [];
    let missing = policy::parse_dependency_paths(&empty);
    let leading = policy::parse_dependency_paths(&[32u8, 97u8]);
    let trailing = policy::parse_dependency_paths(&[97u8, 32u8]);
    let repeated = policy::parse_dependency_paths(&[97u8, 32u8, 32u8, 98u8]);
    let slash = policy::parse_dependency_paths(&[97u8, 92u8]);
    let escape = policy::parse_dependency_paths(&[97u8, 92u8, 98u8]);
    let nul = policy::parse_dependency_paths(&[97u8, 0u8]);
    let tab = policy::parse_dependency_paths(&[97u8, 9u8]);
    let cr = policy::parse_dependency_paths(&[97u8, 13u8]);
    let lf = policy::parse_dependency_paths(&[97u8, 10u8]);
    let del = policy::parse_dependency_paths(&[97u8, 127u8]);
    assert(missing.is_none() && leading.is_none() && trailing.is_none());
    assert(repeated.is_none() && slash.is_none() && escape.is_none());
    assert(nul.is_none() && tab.is_none() && cr.is_none() && lf.is_none() && del.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn requirements_need_selected_tool_and_explicit_nonempty_interfaces() {
    proof {
        reveal_strlit("verus");
        reveal_strlit("unregistered");
        reveal_strlit("chrono-verus/v1");
        reveal_strlit("chrono-verus/v2");
        reveal_strlit("pinned");
        reveal_strlit("identity");
        reveal_strlit("os");
        reveal_strlit("library");
        reveal_strlit("registered bytes");
        reveal_strlit("pinned contract");
        reveal_strlit("");
    }
    let tools = vec!["verus"];
    let mut fields = policy::RequirementFields {
        schema: "chrono-verus/v1",
        tool: "verus",
        version: "pinned",
        commit: "identity",
        interfaces: vec![("os", "registered bytes"), ("library", "pinned contract")],
    };
    let accepted = policy::valid_requirement_fields(&fields, &tools);
    assert(accepted);
    fields.tool = "unregistered";
    let wrong_tool = policy::valid_requirement_fields(&fields, &tools);
    assert("unregistered"@.len() != "verus"@.len());
    assert(!wrong_tool);
    fields.tool = "verus";
    fields.schema = "chrono-verus/v2";
    let wrong_schema = policy::valid_requirement_fields(&fields, &tools);
    assert("chrono-verus/v1"@[14] != "chrono-verus/v2"@[14]);
    assert(!wrong_schema);
    fields.schema = "chrono-verus/v1";
    fields.version = "";
    let no_version = policy::valid_requirement_fields(&fields, &tools);
    assert(!no_version);
    fields.version = "pinned";
    fields.commit = "";
    let no_commit = policy::valid_requirement_fields(&fields, &tools);
    assert(!no_commit);
    fields.commit = "identity";
    fields.interfaces[1] = ("library", "");
    let later_empty_assumption = policy::valid_requirement_fields(&fields, &tools);
    assert(fields.interfaces[1].1@.len() == 0);
    assert(!later_empty_assumption);
    fields.interfaces[1] = ("", "pinned contract");
    let later_empty_name = policy::valid_requirement_fields(&fields, &tools);
    assert(fields.interfaces[1].0@.len() == 0);
    assert(!later_empty_name);
    fields.interfaces = Vec::new();
    let no_interfaces = policy::valid_requirement_fields(&fields, &tools);
    assert(!no_interfaces);
}

fn canonical_arguments() -> (args: Vec<String>)
    ensures policy::verifier_arguments(args@),
{
    proof {
        reveal_strlit("--emit=link=proof.rlib,dep-info=proof.rlib.d");
        reveal_strlit("--emit=link=");
        reveal_strlit("proof.rlib");
        reveal_strlit(",dep-info=");
        reveal_strlit(".d");
        assert("--emit=link=proof.rlib,dep-info=proof.rlib.d"@
            =~= "--emit=link="@ + "proof.rlib"@ + ",dep-info="@ + "proof.rlib"@ + ".d"@);
    }
    vec![
        "--crate-type=lib".to_owned(), "--no-cheating".to_owned(),
        "--no-external-by-default".to_owned(), "--num-threads".to_owned(),
        "2".to_owned(), "--output-json".to_owned(),
        "--compile".to_owned(), "--emit=link=proof.rlib,dep-info=proof.rlib.d".to_owned(), "root.rs".to_owned(),
        "-o".to_owned(), "proof.rlib".to_owned(),
        "-Zchecksum-hash-algorithm=sha256".to_owned(),
    ]
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn verifier_arguments_cannot_disable_checks_or_select_partial_functions() {
    proof {
        reveal_strlit("--no-cheating");
        reveal_strlit("--no-cheating=false");
        reveal_strlit("--no-external-by-default");
        reveal_strlit("--external-by-default");
        reveal_strlit("--verify-function");
        reveal_strlit("-o");
    }
    let mut args = canonical_arguments();
    let accepted = policy::canonical_verifier_arguments(&args);
    assert(accepted);
    args[1] = "--no-cheating=false".to_owned();
    let cheating = policy::canonical_verifier_arguments(&args);
    assert("--no-cheating=false"@.len() != "--no-cheating"@.len());
    assert(!cheating);
    args[1] = "--no-cheating".to_owned();
    args[2] = "--external-by-default".to_owned();
    let external = policy::canonical_verifier_arguments(&args);
    assert("--external-by-default"@[2] != "--no-external-by-default"@[2]);
    assert(!external);
    args[2] = "--no-external-by-default".to_owned();
    args[9] = "--verify-function".to_owned();
    let partial = policy::canonical_verifier_arguments(&args);
    assert("--verify-function"@.len() != "-o"@.len());
    assert(!partial);
    let mut extended = canonical_arguments();
    extended.push("--verify-function=one".to_owned());
    let extra = policy::canonical_verifier_arguments(&extended);
    assert(!extra);
    let empty: Vec<String> = Vec::new();
    let absent = policy::canonical_verifier_arguments(&empty);
    assert(!absent);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn verifier_arguments_require_source_checksums() {
    proof {
        reveal_strlit("-Zchecksum-hash-algorithm=sha256");
        reveal_strlit("-Zchecksum-hash-algorithm=sha1");
    }
    let mut args = canonical_arguments();
    let accepted = policy::canonical_verifier_arguments(&args);
    assert(accepted);
    args[11] = "-Zchecksum-hash-algorithm=sha1".to_owned();
    let weaker = policy::canonical_verifier_arguments(&args);
    assert("-Zchecksum-hash-algorithm=sha256"@.len() != "-Zchecksum-hash-algorithm=sha1"@.len());
    assert(!weaker);
    args.pop();
    let missing = policy::canonical_verifier_arguments(&args);
    assert(!missing);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn proof_identity_requires_equal_bytes_and_lengths() {
    let first = [0u8, 255u8, 7u8];
    let same = [0u8, 255u8, 7u8];
    let changed = [0u8, 255u8, 8u8];
    let shorter = [0u8, 255u8];
    let empty: [u8; 0] = [];
    let a = policy::equal_bytes(&first, &same);
    let b = policy::equal_bytes(&first, &changed);
    let c = policy::equal_bytes(&first, &shorter);
    let d = policy::equal_bytes(&empty, &empty);
    assert(first@ =~= same@);
    assert(first@[2] != changed@[2]);
    assert(first@.len() == 3 && shorter@.len() == 2);
    assert(a);
    assert(!b);
    assert(!c);
    assert(d);
}

fn successful_compiler_result<'a>(version: &'a [u8], commit: &'a [u8])
    -> (result: policy::CompilerResult<'a>)
    ensures
        result.exit_code == Some(0i64),
        result.success == Some(true),
        result.encountered_error == Some(false),
        result.encountered_vir_error == Some(false),
        result.errors == Some(0u64),
        result.verified == Some(1u64),
        result.entire_crate == Some(true),
        result.version == Some(version),
        result.commit == Some(commit),
{
    policy::CompilerResult {
        exit_code: Some(0i64),
        success: Some(true),
        encountered_error: Some(false),
        encountered_vir_error: Some(false),
        errors: Some(0u64),
        verified: Some(1u64),
        entire_crate: Some(true),
        version: Some(version),
        commit: Some(commit),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn whole_crate_proof_with_exact_nonempty_identity_is_accepted() {
    let version = [118u8];
    let commit = [99u8];
    let mut result = successful_compiler_result(&version, &commit);
    let accepted = policy::accept_compiler_result(&result, &version, &commit);
    assert(accepted);
    result.verified = Some(u64::MAX);
    let largest_count = policy::accept_compiler_result(&result, &version, &commit);
    assert(largest_count);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn partial_failed_empty_or_different_proofs_are_rejected() {
    let version = [118u8];
    let commit = [99u8];
    let other = [100u8];
    let empty: [u8; 0] = [];
    let mut result = successful_compiler_result(&version, &commit);
    let stale_version = policy::accept_compiler_result(&result, &other, &commit);
    let stale_commit = policy::accept_compiler_result(&result, &version, &other);
    let no_pin = policy::accept_compiler_result(&result, &empty, &commit);
    assert(!stale_version && !stale_commit && !no_pin);
    result.entire_crate = Some(false);
    let partial = policy::accept_compiler_result(&result, &version, &commit);
    assert(!partial);
    result.entire_crate = Some(true);
    result.verified = Some(0u64);
    let zero = policy::accept_compiler_result(&result, &version, &commit);
    assert(!zero);
    result.verified = Some(1u64);
    result.errors = Some(1u64);
    let error = policy::accept_compiler_result(&result, &version, &commit);
    assert(!error);
    result.errors = Some(0u64);
    result.exit_code = Some(1i64);
    let failed = policy::accept_compiler_result(&result, &version, &commit);
    assert(!failed);
    result.exit_code = Some(0i64);
    result.encountered_error = Some(true);
    let encountered = policy::accept_compiler_result(&result, &version, &commit);
    assert(!encountered);
    result.encountered_error = Some(false);
    result.encountered_vir_error = Some(true);
    let vir = policy::accept_compiler_result(&result, &version, &commit);
    assert(!vir);
    result.encountered_vir_error = Some(false);
    result.success = Some(false);
    let unsuccessful = policy::accept_compiler_result(&result, &version, &commit);
    assert(!unsuccessful);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_or_wrongly_typed_compiler_fields_cannot_become_success() {
    let version = [118u8];
    let commit = [99u8];
    let mut result = successful_compiler_result(&version, &commit);
    result.verified = None;
    let a = policy::accept_compiler_result(&result, &version, &commit);
    assert(!a);
    result.verified = Some(1u64);
    result.version = None;
    let b = policy::accept_compiler_result(&result, &version, &commit);
    assert(!b);
    result.version = Some(&version);
    result.commit = None;
    let c = policy::accept_compiler_result(&result, &version, &commit);
    assert(!c);
    result.commit = Some(&commit);
    result.errors = None;
    let d = policy::accept_compiler_result(&result, &version, &commit);
    assert(!d);
    result.errors = Some(0u64);
    result.exit_code = None;
    let e = policy::accept_compiler_result(&result, &version, &commit);
    assert(!e);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn oid_boundaries() {
    let zero_bytes = [48u8; 40];
    let zero = policy::valid_oid(&zero_bytes);
    assert(!zero);
    let nonzero_bytes = [97u8; 40];
    let nonzero = policy::valid_oid(&nonzero_bytes);
    assert(nonzero_bytes@[0] == 97);
    assert(nonzero);
    let uppercase_bytes = [65u8; 40];
    let uppercase = policy::valid_oid(&uppercase_bytes);
    assert(!uppercase);
    let empty: [u8; 0] = [];
    let too_short = policy::valid_oid(&empty);
    assert(!too_short);
    let long_bytes = [102u8; 64];
    let long = policy::valid_oid(&long_bytes);
    assert(long_bytes@[0] == 102);
    assert(long);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn identifier_ascii_boundaries() {
    let bytes = [48u8, 57u8, 65u8, 90u8, 97u8, 122u8, 46u8, 95u8, 45u8];
    let accepted = policy::valid_id(&bytes);
    assert(accepted);
    let empty: [u8; 0] = [];
    let missing = policy::valid_id(&empty);
    assert(!missing);
    let slash = [47u8];
    let invalid_separator = policy::valid_id(&slash);
    assert(!invalid_separator);
    let non_ascii = [128u8];
    let invalid_unicode = policy::valid_id(&non_ascii);
    assert(!invalid_unicode);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_artifact_identity_and_budget_boundaries() {
    let digest = [48u8; 64];
    let changed = [49u8; 64];
    let invalid = [47u8; 64];
    assert(digest@[0] == 48);
    assert(changed@[0] == 49);
    assert(invalid@[0] == 47);
    let exact = policy::sources::admit_artifact(&digest, &digest, 1, 1, 1);
    assert(exact);
    let empty = policy::sources::admit_artifact(&digest, &digest, 0, 0, 1);
    assert(!empty);
    let over_budget = policy::sources::admit_artifact(&digest, &digest, 2, 2, 1);
    assert(!over_budget);
    let wrong_length = policy::sources::admit_artifact(&digest, &digest, 1, 2, 2);
    assert(!wrong_length);
    let wrong_digest = policy::sources::admit_artifact(&changed, &digest, 1, 1, 1);
    assert(!wrong_digest);
    let malformed = policy::sources::admit_artifact(&invalid, &invalid, 1, 1, 1);
    assert(!malformed);
    let maximum = policy::sources::admit_artifact(
        &digest, &digest, 18446744073709551615u64,
        18446744073709551615u64, 18446744073709551615u64,
    );
    assert(maximum);
}

}
