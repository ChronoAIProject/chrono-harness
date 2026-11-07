//! Independent boundaries for the executable graph search and its contract.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../judge-filemap/src/verified_graph.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_judge_filemap::verified_graph as policy;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

broadcast use vstd::string::group_string_axioms;

/// One complete row: bits name File, Project, Script, Input, Test, Judge, Tool, Environment.
fn check_type_row(kind: policy::EdgeKind, from: policy::NodeKind, mask: u8)
    requires
        policy::permitted_types(kind, from, policy::NodeKind::File) == (mask & 1u8 != 0),
        policy::permitted_types(kind, from, policy::NodeKind::Project) == (mask & 2u8 != 0),
        policy::permitted_types(kind, from, policy::NodeKind::Script) == (mask & 4u8 != 0),
        policy::permitted_types(kind, from, policy::NodeKind::Input) == (mask & 8u8 != 0),
        policy::permitted_types(kind, from, policy::NodeKind::Test) == (mask & 16u8 != 0),
        policy::permitted_types(kind, from, policy::NodeKind::Judge) == (mask & 32u8 != 0),
        policy::permitted_types(kind, from, policy::NodeKind::Tool) == (mask & 64u8 != 0),
        policy::permitted_types(kind, from, policy::NodeKind::Environment) == (mask & 128u8 != 0),
{
    use policy::NodeKind::*;
    let file = policy::valid_types(kind, from, File);
    let project = policy::valid_types(kind, from, Project);
    let script = policy::valid_types(kind, from, Script);
    let input = policy::valid_types(kind, from, Input);
    let test = policy::valid_types(kind, from, Test);
    let judge = policy::valid_types(kind, from, Judge);
    let tool = policy::valid_types(kind, from, Tool);
    let environment = policy::valid_types(kind, from, Environment);
    assert(file == (mask & 1u8 != 0));
    assert(project == (mask & 2u8 != 0));
    assert(script == (mask & 4u8 != 0));
    assert(input == (mask & 8u8 != 0));
    assert(test == (mask & 16u8 != 0));
    assert(judge == (mask & 32u8 != 0));
    assert(tool == (mask & 64u8 != 0));
    assert(environment == (mask & 128u8 != 0));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_320_edge_type_combinations_match_the_independent_table() {
    use policy::EdgeKind::*;
    use policy::NodeKind::*;
    check_type_row(Compile, File, 2);
    check_type_row(Compile, Project, 2);
    check_type_row(Compile, Script, 0);
    check_type_row(Compile, Input, 0);
    check_type_row(Compile, Test, 0);
    check_type_row(Compile, Judge, 0);
    check_type_row(Compile, Tool, 0);
    check_type_row(Compile, Environment, 0);
    check_type_row(BuildInput, File, 6);
    check_type_row(BuildInput, Project, 6);
    check_type_row(BuildInput, Script, 6);
    check_type_row(BuildInput, Input, 6);
    check_type_row(BuildInput, Test, 0);
    check_type_row(BuildInput, Judge, 0);
    check_type_row(BuildInput, Tool, 6);
    check_type_row(BuildInput, Environment, 6);
    check_type_row(RuntimeInput, File, 38);
    check_type_row(RuntimeInput, Project, 38);
    check_type_row(RuntimeInput, Script, 38);
    check_type_row(RuntimeInput, Input, 38);
    check_type_row(RuntimeInput, Test, 0);
    check_type_row(RuntimeInput, Judge, 0);
    check_type_row(RuntimeInput, Tool, 38);
    check_type_row(RuntimeInput, Environment, 38);
    check_type_row(TestExecution, File, 16);
    check_type_row(TestExecution, Project, 16);
    check_type_row(TestExecution, Script, 16);
    check_type_row(TestExecution, Input, 16);
    check_type_row(TestExecution, Test, 0);
    check_type_row(TestExecution, Judge, 0);
    check_type_row(TestExecution, Tool, 16);
    check_type_row(TestExecution, Environment, 16);
    check_type_row(JudgeTrigger, File, 32);
    check_type_row(JudgeTrigger, Project, 32);
    check_type_row(JudgeTrigger, Script, 0);
    check_type_row(JudgeTrigger, Input, 32);
    check_type_row(JudgeTrigger, Test, 0);
    check_type_row(JudgeTrigger, Judge, 0);
    check_type_row(JudgeTrigger, Tool, 32);
    check_type_row(JudgeTrigger, Environment, 32);
}

fn edge(from: &str, kind: policy::EdgeKind, to: &str) -> (out: policy::Edge)
    ensures out.from_bytes() == from.spec_bytes(), out.kind == kind,
        out.to_bytes() == to.spec_bytes(),
{
    policy::Edge { from: from.to_owned(), kind, to: to.to_owned() }
}

fn edge_before(a: &policy::Edge, b: &policy::Edge)
    requires a.less(b), !a.same(b),
{
    use core::cmp::Ordering;
    proof { policy::edge_order_laws(a, b, b); }
    let forward = a.cmp(b);
    let reverse = b.cmp(a);
    let partial = a.partial_cmp(b);
    let equal = a == b;
    assert(forward == Ordering::Less);
    assert(reverse == Ordering::Greater);
    assert(partial == Some(Ordering::Less));
    assert(!equal);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn typed_union_preserves_empty_and_single_endpoint_boundaries() {
    use std::collections::BTreeSet;
    proof {
        policy::edge_obeys_cmp();
        broadcast use vstd::set::Set::lemma_set_map_insert_commute;
    }
    let empty: BTreeSet<policy::Edge> = BTreeSet::new();
    proof {
        assert(empty@.map(|e: policy::Edge| e.key()) =~= Set::<policy::EdgeKey>::empty());
        assert((empty@.map(|e: policy::Edge| e.key()) + empty@.map(|e: policy::Edge| e.key())) =~= Set::<policy::EdgeKey>::empty());
    }
    let out = policy::union(&empty, &empty);
    assert(out.len() == 0);
    let item = edge("根\0", policy::EdgeKind::JudgeTrigger, "");
    let same_bytes = edge("根\0", policy::EdgeKind::JudgeTrigger, "");
    let mut base = BTreeSet::new();
    base.insert(item.clone());
    base.insert(item.clone());
    assert(base@ == Set::<policy::Edge>::empty().insert(item));
    let mut candidate = BTreeSet::new();
    candidate.insert(same_bytes.clone());
    assert(candidate@ == Set::<policy::Edge>::empty().insert(same_bytes));
    proof {
        assert(base@.map(|e: policy::Edge| e.key()) =~= Set::<policy::EdgeKey>::empty().insert(item.key()));
        assert(candidate@.map(|e: policy::Edge| e.key()) =~= Set::<policy::EdgeKey>::empty().insert(item.key()));
        assert((base@.map(|e: policy::Edge| e.key()) + empty@.map(|e: policy::Edge| e.key())) =~= Set::<policy::EdgeKey>::empty().insert(item.key()));
        assert((empty@.map(|e: policy::Edge| e.key()) + candidate@.map(|e: policy::Edge| e.key())) =~= Set::<policy::EdgeKey>::empty().insert(item.key()));
        assert((base@.map(|e: policy::Edge| e.key()) + candidate@.map(|e: policy::Edge| e.key())) =~= Set::<policy::EdgeKey>::empty().insert(item.key()));
        assert(Set::<policy::EdgeKey>::empty().insert(item.key()).len() == 1);
    }
    let out = policy::union(&base, &empty);
    assert(out.len() == 1);
    assert(policy::union_edge_keys(out@).to_set().contains(out@[0].edge.key()));
    assert(out[0].edge.key() == item.key());
    assert(out[0].origin == policy::Origin::Base);
    let out = policy::union(&empty, &candidate);
    assert(out.len() == 1);
    assert(policy::union_edge_keys(out@).to_set().contains(out@[0].edge.key()));
    assert(out[0].edge.key() == item.key());
    assert(out[0].origin == policy::Origin::Candidate);
    let out = policy::union(&base, &candidate);
    assert(out.len() == 1);
    assert(policy::union_edge_keys(out@).to_set().contains(out@[0].edge.key()));
    assert(out[0].edge.key() == item.key());
    assert(out[0].origin == policy::Origin::Both);
    let cloned = out[0].clone();
    assert(cloned.edge.key() == item.key() && cloned.origin == policy::Origin::Both);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn typed_union_has_exact_sorted_slots_and_all_three_origins() {
    use std::collections::BTreeSet;
    use policy::EdgeKind::*;
    proof {
        policy::edge_obeys_cmp();
        broadcast use vstd::set::Set::lemma_set_map_insert_commute;
    }
    let first = edge("", Compile, "根\0");
    let shared = edge("", BuildInput, "根\0");
    let last = edge("", RuntimeInput, "根\0");
    let mut base = BTreeSet::new();
    base.insert(shared.clone());
    base.insert(first.clone());
    assert(base@ == Set::<policy::Edge>::empty().insert(shared).insert(first));
    let mut candidate = BTreeSet::new();
    candidate.insert(last.clone());
    candidate.insert(shared.clone());
    candidate.insert(last.clone());
    assert(candidate@ == Set::<policy::Edge>::empty().insert(last).insert(shared));
    proof {
        assert(base@.map(|e: policy::Edge| e.key()) =~= Set::<policy::EdgeKey>::empty().insert(first.key()).insert(shared.key()));
        assert(candidate@.map(|e: policy::Edge| e.key()) =~= Set::<policy::EdgeKey>::empty().insert(shared.key()).insert(last.key()));
        assert((base@.map(|e: policy::Edge| e.key()) + candidate@.map(|e: policy::Edge| e.key()))
            =~= Set::<policy::EdgeKey>::empty().insert(first.key()).insert(shared.key()).insert(last.key()));
        assert(Set::<policy::EdgeKey>::empty().insert(first.key()).insert(shared.key()).insert(last.key()).len() == 3);
    }
    let out = policy::union(&base, &candidate);
    assert(out.len() == 3);
    proof {
        reveal(policy::ordered_union);
        assert forall|i: int| 0 <= i < out.len() implies
            (#[trigger] out@[i]).edge.key() == first.key()
                || out@[i].edge.key() == shared.key() || out@[i].edge.key() == last.key() by {
            assert(policy::union_edge_keys(out@).to_set().contains(out@[i].edge.key()));
        }
        policy::edge_order_laws(&out@[0].edge, &out@[1].edge, &out@[2].edge);
        policy::edge_order_laws(&out@[1].edge, &out@[2].edge, &out@[2].edge);
        assert(out@[0].edge.kind.rank_spec() < out@[1].edge.kind.rank_spec());
        assert(out@[1].edge.kind.rank_spec() < out@[2].edge.kind.rank_spec());
    }
    assert(out[0].edge.key() == first.key());
    assert(out[1].edge.key() == shared.key());
    assert(out[2].edge.key() == last.key());
    assert(out[0].origin == policy::Origin::Base);
    assert(out[1].origin == policy::Origin::Both);
    assert(out[2].origin == policy::Origin::Candidate);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn typed_edge_indices_preserve_parallel_slots_cycles_and_duplicate_roots() {
    use policy::EdgeKind::*;
    proof {
        reveal_strlit("a"); reveal_strlit("b"); reveal_strlit("c");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("b"@);
        vstd::utf8::is_ascii_chars_encode_utf8("c"@);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    }
    let names: Vec<&str> = vec!["a", "b", "c"];
    proof {
        assert forall|i: int, j: int| 0 <= i < j < names.len()
            implies policy::byte_lt(#[trigger] names@[i].spec_bytes(), #[trigger] names@[j].spec_bytes()) by {
            assert(policy::byte_prefix(names@[i].spec_bytes(), names@[j].spec_bytes(), 0));
        }
    }
    let direct = edge("a", Compile, "c");
    let parallel_first = edge("a", TestExecution, "b");
    let parallel_next = edge("a", RuntimeInput, "b");
    let onward = edge("b", Compile, "c");
    let cycle = edge("c", JudgeTrigger, "a");
    let rows: Vec<Vec<&policy::Edge>> = vec![
        vec![&direct, &parallel_first, &parallel_next], vec![&onward], vec![&cycle],
    ];
    let roots: Vec<&str> = vec!["c", "a", "b", "b"];
    proof {
        assert(policy::name_members(names@).contains("a".spec_bytes()));
        assert(policy::name_members(names@).contains("b".spec_bytes()));
        assert(policy::name_members(names@).contains("c".spec_bytes()));
        assert forall|r: int, s: int| 0 <= r < rows.len() && 0 <= s < rows@[r].len()
            implies policy::name_members(names@).contains(#[trigger] rows@[r]@[s].to_bytes()) by {
            if r == 0 {
                assert(rows@[r]@[s] == &direct || rows@[r]@[s] == &parallel_first
                    || rows@[r]@[s] == &parallel_next);
            } else if r == 1 { assert(rows@[r]@[s] == &onward); }
            else { assert(rows@[r]@[s] == &cycle); }
        }
        assert(policy::edge_names_present(names@, rows@, roots@));
    }
    let result = policy::index_edges(&names, &rows, &roots);
    assert(result.is_some());
    let mapped = result.unwrap();
    assert(mapped.rows.len() == 3);
    assert(mapped.rows[0]@ == seq![2usize, 1, 1]) by {
        assert(policy::edge_indices_match(names@, rows@[0]@, mapped.rows@[0]@));
    }
    assert(mapped.rows[1]@ == seq![2usize]) by {
        assert(policy::edge_indices_match(names@, rows@[1]@, mapped.rows@[1]@));
    }
    assert(mapped.rows[2]@ == seq![0usize]) by {
        assert(policy::edge_indices_match(names@, rows@[2]@, mapped.rows@[2]@));
    }
    assert(mapped.roots@ == seq![2usize, 0, 1, 1]) by {
        assert(policy::name_indices_match(names@, roots@, mapped.roots@));
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn typed_edge_indices_preserve_empty_and_unicode_names_and_empty_rows() {
    proof {
        reveal_strlit(""); reveal_strlit("根");
        assert(vstd::utf8::encode_scalar(26681) == seq![230u8, 160, 185]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    }
    let names: Vec<&str> = vec!["", "根"];
    proof { assert(policy::byte_prefix(names@[0].spec_bytes(), names@[1].spec_bytes(), 0)); }
    let to_empty = edge("根", policy::EdgeKind::RuntimeInput, "");
    let self_loop = edge("根", policy::EdgeKind::Compile, "根");
    let rows: Vec<Vec<&policy::Edge>> = vec![vec![], vec![&to_empty, &self_loop]];
    let roots: Vec<&str> = vec!["根", ""];
    let result = policy::index_edges(&names, &rows, &roots);
    assert(result.is_some());
    let mapped = result.unwrap();
    assert(mapped.rows.len() == 2);
    assert(mapped.rows[0]@ == Seq::<usize>::empty());
    assert(mapped.rows[1]@ == seq![0usize, 1]);
    assert(mapped.roots@ == seq![1usize, 0]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn typed_edge_indices_reject_missing_targets_roots_and_row_counts() {
    use policy::EdgeKind::*;
    proof {
        reveal_strlit("a"); reveal_strlit("aa");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("aa"@);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    }
    let names: Vec<&str> = vec!["a"];
    let absent = edge("a", Compile, "aa");
    let rows: Vec<Vec<&policy::Edge>> = vec![vec![&absent]];
    let no_roots: Vec<&str> = vec![];
    let result = policy::index_edges(&names, &rows, &no_roots);
    assert(result.is_none());
    let rows: Vec<Vec<&policy::Edge>> = vec![vec![]];
    let roots: Vec<&str> = vec!["aa"];
    let result = policy::index_edges(&names, &rows, &roots);
    assert(result.is_none());
    let short: Vec<Vec<&policy::Edge>> = vec![];
    let result = policy::index_edges(&names, &short, &no_roots);
    assert(result.is_none());
    let long: Vec<Vec<&policy::Edge>> = vec![vec![], vec![]];
    let result = policy::index_edges(&names, &long, &no_roots);
    assert(result.is_none());
    let empty_names: Vec<&str> = vec![];
    let result = policy::index_edges(&empty_names, &short, &no_roots);
    assert(result.is_some());
    let mapped = result.unwrap();
    assert(mapped.rows.len() == 0 && mapped.roots.len() == 0);
    let result = policy::index_edges(&empty_names, &short, &roots);
    assert(result.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn full_edge_order_obeys_field_precedence_prefixes_and_unicode() {
    use policy::EdgeKind::*;
    proof {
        reveal_strlit(""); reveal_strlit("a"); reveal_strlit("a\0"); reveal_strlit("aa");
        reveal_strlit("é"); reveal_strlit("根");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("a\0"@);
        vstd::utf8::is_ascii_chars_encode_utf8("aa"@);
        assert(vstd::utf8::encode_scalar(233) == seq![195u8, 169]) by (compute);
        assert(vstd::utf8::encode_scalar(26681) == seq![230u8, 160, 185]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 4);
        assert("".spec_bytes() == Seq::<u8>::empty());
        assert("a".spec_bytes() == seq![97u8]);
        assert("a\0".spec_bytes() == seq![97u8, 0]);
        assert("aa".spec_bytes() == seq![97u8, 97]);
        assert("é".spec_bytes() == seq![195u8, 169]);
        assert("根".spec_bytes() == seq![230u8, 160, 185]);
    }
    let first = edge("", JudgeTrigger, "根");
    let next = edge("a", Compile, "");
    proof { assert(policy::byte_prefix(first.from_bytes(), next.from_bytes(), 0)); }
    edge_before(&first, &next);
    let first = edge("a", JudgeTrigger, "根");
    let next = edge("aa", Compile, "");
    proof { assert(policy::byte_prefix(first.from_bytes(), next.from_bytes(), 1)); }
    edge_before(&first, &next);
    let first = edge("é", JudgeTrigger, "根");
    let next = edge("根", Compile, "");
    proof { assert(policy::byte_prefix(first.from_bytes(), next.from_bytes(), 0)); }
    edge_before(&first, &next);
    let first = edge("a", Compile, "根");
    let next = edge("a", BuildInput, "");
    edge_before(&first, &next);
    let first = edge("a", Compile, "");
    let next = edge("a", Compile, "a");
    proof { assert(policy::byte_prefix(first.to_bytes(), next.to_bytes(), 0)); }
    edge_before(&first, &next);
    let first = edge("a", Compile, "a");
    let next = edge("a", Compile, "a\0");
    proof { assert(policy::byte_prefix(first.to_bytes(), next.to_bytes(), 1)); }
    edge_before(&first, &next);
    let first = edge("a", Compile, "a\0");
    let next = edge("a", Compile, "aa");
    proof { assert(policy::byte_prefix(first.to_bytes(), next.to_bytes(), 1)); }
    edge_before(&first, &next);
    let first = edge("a", Compile, "é");
    let next = edge("a", Compile, "根");
    proof { assert(policy::byte_prefix(first.to_bytes(), next.to_bytes(), 0)); }
    edge_before(&first, &next);
    let cloned = next.clone();
    let equal = next == cloned;
    let order = next.cmp(&cloned);
    assert(equal);
    assert(order == core::cmp::Ordering::Equal);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn edge_inventory_preserves_empty_rows_duplicates_and_exact_kind_slots() {
    use policy::EdgeKind::*;
    use std::collections::BTreeSet;
    proof {
        policy::edge_obeys_cmp();
        broadcast use vstd::laws_cmp::lemma_ref_obeys_cmp_spec;
        assert(vstd::laws_cmp::obeys_cmp::<&policy::Edge>());
    }
    let empty: BTreeSet<&policy::Edge> = BTreeSet::new();
    let out = policy::edge_inventory(&empty);
    assert(out.len() == 0);
    let a = edge("a", Compile, "根");
    let b = edge("a", BuildInput, "");
    let mut row = BTreeSet::new();
    row.insert(&b);
    row.insert(&a);
    row.insert(&b);
    assert(row@ == Set::<&policy::Edge>::empty().insert(&a).insert(&b));
    assert(row@.len() == 2);
    let out = policy::edge_inventory(&row);
    assert(out.len() == 2);
    proof {
        assert(out@.to_set().contains(&a));
        assert(out@.contains(&a));
        assert(out@.to_set().contains(out@[0]));
        assert(out@[0] == &a || out@[0] == &b);
        let i = choose|i: int| 0 <= i < out.len() && out@[i] == &a;
        if i == 1 {
            assert(out@[0] == &b);
            assert(out@[0].order_spec(out@[1]) != core::cmp::Ordering::Less);
        }
        assert(out@[0] == &a);
        assert(out@.to_set().contains(out@[1]));
        assert(out@[1] != out@[0]);
        assert(out@[1] == &b);
    }
    let first = out[0] == &a;
    let second = out[1] == &b;
    assert(first);
    assert(second);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn all_edge_kind_pairs_obey_the_declared_order_and_equality() {
    use policy::EdgeKind::*;
    use core::cmp::Ordering;
    let kinds = vec![Compile, BuildInput, RuntimeInput, TestExecution, JudgeTrigger];
    proof {
        assert forall|i: int| 0 <= i < kinds.len()
            implies (#[trigger] kinds@[i].rank_spec()) == i by {}
    }
    let mut i: usize = 0;
    while i < kinds.len()
        invariant i <= kinds.len(), kinds.len() == 5,
            forall|k: int| 0 <= k < kinds.len() ==> (#[trigger] kinds@[k].rank_spec()) == k,
        decreases kinds.len() - i,
    {
        let mut j: usize = 0;
        while j < kinds.len()
            invariant i < kinds.len(), j <= kinds.len(), kinds.len() == 5,
                forall|k: int| 0 <= k < kinds.len() ==> (#[trigger] kinds@[k].rank_spec()) == k,
            decreases kinds.len() - j,
        {
            proof { policy::edge_kind_order_laws(kinds@[i as int], kinds@[j as int], Compile); }
            let equal = kinds[i] == kinds[j];
            assert(equal == (i == j));
            let different = kinds[i] != kinds[j];
            assert(different == (i != j));
            let order = kinds[i].cmp(&kinds[j]);
            let partial = kinds[i].partial_cmp(&kinds[j]);
            if i < j {
                assert(order == Ordering::Less);
                assert(partial == Some(Ordering::Less));
            } else if i > j {
                assert(order == Ordering::Greater);
                assert(partial == Some(Ordering::Greater));
            } else {
                assert(order == Ordering::Equal);
                assert(partial == Some(Ordering::Equal));
            }
            j += 1;
        }
        i += 1;
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn canonical_edge_kind_names_have_exact_identity_and_roundtrip() {
    use policy::EdgeKind::*;
    let kinds = vec![Compile, BuildInput, RuntimeInput, TestExecution, JudgeTrigger];
    let names = vec!["compile", "build-input", "runtime-input", "test-execution", "judge-trigger"];
    proof {
        assert forall|i: int| 0 <= i < kinds.len()
            implies (#[trigger] kinds@[i].wire_spec()) == names@[i].spec_bytes() by {}
    }
    let mut i: usize = 0;
    while i < kinds.len()
        invariant i <= kinds.len(), kinds.len() == 5, names.len() == 5,
            forall|k: int| 0 <= k < kinds.len()
                ==> (#[trigger] kinds@[k].wire_spec()) == names@[k].spec_bytes(),
        decreases kinds.len() - i,
    {
        let name = kinds[i].wire_name();
        let order = policy::compare_bytes(name.as_bytes(), names[i].as_bytes());
        assert(order == 0);
        let decoded = policy::EdgeKind::from_wire(names[i]);
        proof { assert(exists|k: policy::EdgeKind| k.wire_spec() == names@[i as int].spec_bytes()); }
        assert(decoded.is_some());
        let decoded = decoded.unwrap();
        proof { policy::edge_kind_wire_unique(decoded, kinds@[i as int]); }
        let equal = decoded == kinds[i];
        assert(equal);
        i += 1;
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn edge_kind_parser_rejects_noncanonical_names() {
    proof {
        reveal_strlit("");
        reveal_strlit("Compile");
        reveal_strlit("compil");
        reveal_strlit("compilex");
        reveal_strlit("build_input");
        reveal_strlit(" compile");
        reveal_strlit("compile\0");
        vstd::string::is_ascii_spec_bytes("");
        vstd::string::is_ascii_spec_bytes("Compile");
        vstd::string::is_ascii_spec_bytes("compil");
        vstd::string::is_ascii_spec_bytes("compilex");
        vstd::string::is_ascii_spec_bytes("build_input");
        vstd::string::is_ascii_spec_bytes(" compile");
        vstd::string::is_ascii_spec_bytes("compile\0");
        // Establish the five canonical spellings independently of the parser.
        reveal_strlit("compile");
        reveal_strlit("build-input");
        reveal_strlit("runtime-input");
        reveal_strlit("test-execution");
        reveal_strlit("judge-trigger");
        vstd::string::is_ascii_spec_bytes("compile");
        vstd::string::is_ascii_spec_bytes("build-input");
        vstd::string::is_ascii_spec_bytes("runtime-input");
        vstd::string::is_ascii_spec_bytes("test-execution");
        vstd::string::is_ascii_spec_bytes("judge-trigger");
        assert("compile".spec_bytes().len() == 7);
        assert("build-input".spec_bytes().len() == 11);
        assert("runtime-input".spec_bytes().len() == 13);
        assert("test-execution".spec_bytes().len() == 14);
        assert("judge-trigger".spec_bytes().len() == 13);
        assert("".spec_bytes().len() == 0);
        assert("Compile".spec_bytes().len() == 7);
        assert("Compile".spec_bytes()[0] == 67);
        assert("compile".spec_bytes()[0] == 99);
        assert("compil".spec_bytes().len() == 6);
        assert("compilex".spec_bytes().len() == 8);
        assert("build_input".spec_bytes().len() == 11);
        assert("build_input".spec_bytes()[5] == 95);
        assert("build-input".spec_bytes()[5] == 45);
        assert(" compile".spec_bytes().len() == 8);
        assert("compile\0".spec_bytes().len() == 8);
    }
    let names = vec!["", "Compile", "compil", "compilex", "build_input", " compile", "compile\0"];
    proof {
        assert forall|i: int, k: policy::EdgeKind| 0 <= i < names.len()
            implies (#[trigger] names@[i].spec_bytes()) != (#[trigger] k.wire_spec()) by {
            if i == 0 {} else if i == 1 {} else if i == 2 {}
            else if i == 3 {} else if i == 4 {} else if i == 5 {} else {}
            match k {
                policy::EdgeKind::Compile => {},
                policy::EdgeKind::BuildInput => {},
                policy::EdgeKind::RuntimeInput => {},
                policy::EdgeKind::TestExecution => {},
                policy::EdgeKind::JudgeTrigger => {},
            }
        }
    }
    let mut i: usize = 0;
    while i < names.len()
        invariant i <= names.len(),
            forall|j: int, k: policy::EdgeKind| 0 <= j < names.len()
                ==> (#[trigger] names@[j].spec_bytes()) != (#[trigger] k.wire_spec()),
        decreases names.len() - i,
    {
        let parsed = policy::EdgeKind::from_wire(names[i]);
        assert(parsed.is_none());
        i += 1;
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn named_slots_keep_parallel_edges_cycles_and_root_positions() {
    proof {
        reveal_strlit("");
        reveal_strlit("a");
        reveal_strlit("根");
        vstd::utf8::is_ascii_chars_encode_utf8(""@);
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        assert(vstd::utf8::encode_scalar(26681) == seq![230u8, 160, 185]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    }
    let names: Vec<&str> = vec!["", "a", "根"];
    assert(names@[0].spec_bytes() == Seq::<u8>::empty());
    assert(names@[1].spec_bytes() == seq![97u8]);
    assert(names@[2].spec_bytes() == seq![230u8, 160, 185]);
    proof {
        assert forall|i: int, j: int| 0 <= i < j < names.len()
            implies policy::byte_lt(#[trigger] names@[i].spec_bytes(), #[trigger] names@[j].spec_bytes()) by {
            assert(policy::byte_prefix(names@[i].spec_bytes(), names@[j].spec_bytes(), 0));
        }
    }
    let rows: Vec<Vec<&str>> = vec![vec!["a", "a", ""], vec![""], vec![]];
    let roots: Vec<&str> = vec!["根", "", "根"];
    assert(policy::graph_names_present(names@, rows@, roots@));
    let found = policy::index_names(&names, &rows, &roots);
    assert(found.is_some());
    let mapped = found.unwrap();
    assert(policy::valid_graph(mapped.rows@));
    assert(mapped.rows.len() == 3);
    assert(mapped.rows@[0].len() == 3);
    assert(mapped.rows@[1].len() == 1);
    assert(mapped.rows@[2].len() == 0);
    assert(mapped.rows@[0]@ == seq![1usize, 1, 0]);
    assert(mapped.rows@[1]@ == seq![0usize]);
    assert(mapped.roots.len() == 3);
    assert(mapped.roots@ == seq![2usize, 0, 2]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn named_graph_rejects_missing_targets_roots_and_row_counts() {
    proof {
        reveal_strlit("a");
        reveal_strlit("b");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("b"@);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    }
    let names: Vec<&str> = vec!["a"];
    assert(names@[0].spec_bytes() == seq![97u8]);
    let valid = policy::valid_names(&names);
    assert(valid);
    let no_roots: Vec<&str> = vec![];
    let missing_target: Vec<Vec<&str>> = vec![vec!["b"]];
    assert(missing_target@[0]@[0].spec_bytes() == seq![98u8]);
    let found = policy::index_names(&names, &missing_target, &no_roots);
    assert(found.is_none());
    let rows: Vec<Vec<&str>> = vec![vec!["a"]];
    let missing_root: Vec<&str> = vec!["b"];
    let found = policy::index_names(&names, &rows, &missing_root);
    assert(found.is_none());
    let no_rows: Vec<Vec<&str>> = vec![];
    let found = policy::index_names(&names, &no_rows, &no_roots);
    assert(found.is_none());
    let extra_rows: Vec<Vec<&str>> = vec![vec![], vec![]];
    let found = policy::index_names(&names, &extra_rows, &no_roots);
    assert(found.is_none());
    let empty: Vec<&str> = vec![];
    let found = policy::index_names(&empty, &no_rows, &no_roots);
    assert(found.is_some());
    let found = policy::index_names(&empty, &no_rows, &missing_root);
    assert(found.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn built_inventory_is_ordered_unique_and_preserves_every_supplied_name() {
    let input: Vec<&str> = vec!["根", "b", "", "aa", "a", "b", "根", "a"];
    let names = policy::name_inventory(&input);
    let valid = policy::valid_names(&names);
    assert(valid);
    assert(policy::name_members(names@) == policy::name_members(input@));
    assert(input@[0].spec_bytes() == "根".spec_bytes());
    assert(policy::name_members(names@).contains("根".spec_bytes()));
    assert(input@[1].spec_bytes() == "b".spec_bytes());
    assert(policy::name_members(names@).contains("b".spec_bytes()));
    assert(input@[2].spec_bytes() == "".spec_bytes());
    assert(policy::name_members(names@).contains("".spec_bytes()));
    let found = policy::name_index(&names, "根");
    assert(found.is_some());
    let found = policy::name_index(&names, "b");
    assert(found.is_some());
    let found = policy::name_index(&names, "");
    assert(found.is_some());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn inventory_construction_handles_empty_and_repeated_singleton_inputs() {
    let empty: Vec<&str> = vec![];
    let names = policy::name_inventory(&empty);
    assert(names.len() == 0);
    let repeated: Vec<&str> = vec!["根", "根", "根"];
    assert(policy::name_members(repeated@) =~= Set::<Seq<u8>>::empty().insert("根".spec_bytes()));
    let names = policy::name_inventory(&repeated);
    assert(names.len() == 1);
    assert(policy::name_members(names@).contains(names@[0].spec_bytes()));
    assert(names@[0].spec_bytes() == "根".spec_bytes());
    let found = policy::name_index(&names, "根");
    assert(found == Some(0usize));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn descending_duplicate_input_has_exact_ascending_slots() {
    proof {
        reveal_strlit("a");
        reveal_strlit("b");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("b"@);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    }
    let input: Vec<&str> = vec!["b", "a", "b"];
    assert(input@[0].spec_bytes() == seq![98u8]);
    assert(input@[1].spec_bytes() == seq![97u8]);
    assert(input@[2].spec_bytes() == seq![98u8]);
    assert(policy::name_members(input@) =~= set![seq![97u8], seq![98u8]]);
    let names = policy::name_inventory(&input);
    assert(names.len() == 2);
    assert(policy::name_members(names@).contains(names@[0].spec_bytes()));
    assert(policy::name_members(names@).contains(names@[1].spec_bytes()));
    assert(!policy::byte_lt(seq![98u8], seq![97u8]));
    assert(!policy::byte_lt(seq![97u8], seq![97u8]));
    assert(!policy::byte_lt(seq![98u8], seq![98u8]));
    assert(names@[0].spec_bytes() == seq![97u8]);
    assert(names@[1].spec_bytes() == seq![98u8]);
    let found = policy::name_index(&names, "a");
    assert(found == Some(0usize));
    let found = policy::name_index(&names, "b");
    assert(found == Some(1usize));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn byte_order_handles_empty_prefixes_and_the_full_byte_range() {
    let empty: Vec<u8> = vec![];
    let zero: Vec<u8> = vec![0];
    let prefix: Vec<u8> = vec![0, 255];
    let high: Vec<u8> = vec![128];
    let highest: Vec<u8> = vec![255];
    let order = policy::compare_bytes(&empty, &empty);
    assert(order == 0);
    assert(!policy::byte_lt(zero@, empty@));
    let order = policy::compare_bytes(&empty, &zero);
    assert(order == -1);
    let order = policy::compare_bytes(&zero, &prefix);
    assert(order == -1);
    let order = policy::compare_bytes(&prefix, &zero);
    assert(order == 1);
    let order = policy::compare_bytes(&prefix, &high);
    assert(order == -1);
    let order = policy::compare_bytes(&highest, &high);
    assert(order == 1);
    let order = policy::compare_bytes(&prefix, &prefix);
    assert(order == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn name_inventory_rejects_duplicate_names_and_descending_bytes() {
    let duplicate: Vec<&str> = vec!["根", "根"];
    assert(duplicate@[0].spec_bytes() == duplicate@[1].spec_bytes());
    let valid = policy::valid_names(&duplicate);
    assert(!valid);
    proof {
        reveal_strlit("a");
        reveal_strlit("b");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("b"@);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    }
    let descending: Vec<&str> = vec!["b", "a"];
    assert(descending@[0].spec_bytes() == seq![98u8]);
    assert(descending@[1].spec_bytes() == seq![97u8]);
    let valid = policy::valid_names(&descending);
    assert(!valid);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn name_lookup_covers_empty_singleton_prefixes_unicode_and_absence() {
    proof {
        reveal_strlit("");
        reveal_strlit("a");
        reveal_strlit("aa");
        reveal_strlit("b");
        reveal_strlit("c");
        reveal_strlit("é");
        reveal_strlit("根");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("aa"@);
        vstd::utf8::is_ascii_chars_encode_utf8("b"@);
        vstd::utf8::is_ascii_chars_encode_utf8("c"@);
        assert(vstd::utf8::encode_scalar(233) == seq![195u8, 169]) by (compute);
        assert(vstd::utf8::encode_scalar(26681) == seq![230u8, 160, 185]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 4);
    }
    let empty: Vec<&str> = vec![];
    let valid = policy::valid_names(&empty);
    assert(valid);
    let found = policy::name_index(&empty, "a");
    assert(found.is_none());
    let one: Vec<&str> = vec!["根"];
    assert(one@[0].spec_bytes() == "根".spec_bytes());
    let valid = policy::valid_names(&one);
    assert(valid);
    let found = policy::name_index(&one, "根");
    assert(found == Some(0usize));
    let names: Vec<&str> = vec!["", "a", "aa", "b", "é", "根"];
    assert(names@[0].spec_bytes() == Seq::<u8>::empty());
    assert(names@[1].spec_bytes() == seq![97u8]);
    assert(names@[2].spec_bytes() == seq![97u8, 97]);
    assert(names@[3].spec_bytes() == seq![98u8]);
    assert(names@[4].spec_bytes() == seq![195u8, 169]);
    assert(names@[5].spec_bytes() == seq![230u8, 160, 185]);
    proof {
        assert forall|i: int, j: int| 0 <= i < j < names.len()
            implies policy::byte_lt(#[trigger] names@[i].spec_bytes(), #[trigger] names@[j].spec_bytes()) by {
            let a = names@[i].spec_bytes();
            let b = names@[j].spec_bytes();
            let first = if i == 1 && j == 2 { 1int } else { 0int };
            assert(policy::byte_prefix(a, b, first));
            assert(0 <= first <= a.len() && first < b.len()
                && (first == a.len() || a[first] < b[first]));
        }
    }
    let valid = policy::valid_names(&names);
    assert(valid);
    let found = policy::name_index(&names, "");
    assert(found == Some(0usize));
    let found = policy::name_index(&names, "aa");
    assert(found == Some(2usize));
    let found = policy::name_index(&names, "é");
    assert(found == Some(4usize));
    let found = policy::name_index(&names, "根");
    assert(found == Some(5usize));
    let found = policy::name_index(&names, "c");
    assert(found.is_none());
}

/// Independent paths give upper bounds; an independently supplied feasible
/// distance labeling gives lower bounds on every real predecessor path.
proof fn exact_distance(
    g: Seq<Vec<usize>>, root: usize, search: policy::Search,
    potential: Seq<usize>, v: usize, path: Seq<usize>,
)
    requires policy::valid_graph(g), policy::complete(g, root, search),
        potential.len() == g.len(), potential[root as int] == 0,
        policy::closed(g, potential), v < g.len(),
        policy::walk(g, path), path[0] == root, path.last() == v,
        potential[v as int] == path.len() - 1,
    ensures search.distances@[v as int] == potential[v as int], search.order@.contains(v),
{
    reveal(policy::complete);
    policy::walk_bound(g, search.distances@, root, path);
    let witness = policy::tree_path(g, root, search.distances@, search.parents@, v);
    policy::walk_bound(g, potential, root, witness);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_graph_and_invalid_roots() {
    let empty: Vec<Vec<usize>> = Vec::new();
    let valid = policy::valid_indices(&empty);
    assert(valid);
    let result = policy::breadth_first(&empty, 0);
    assert(result.is_none());
    let g: Vec<Vec<usize>> = vec![vec![]];
    let valid = policy::valid_indices(&g);
    assert(valid);
    let result = policy::breadth_first(&g, 1);
    assert(result.is_none());
    let result = policy::breadth_first(&g, usize::MAX);
    assert(result.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn malformed_destinations_fail_before_traversal() {
    let g: Vec<Vec<usize>> = vec![vec![1]];
    let valid = policy::valid_indices(&g);
    assert(!valid);
    let g: Vec<Vec<usize>> = vec![vec![], vec![usize::MAX]];
    let valid = policy::valid_indices(&g);
    assert(!valid);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn self_cycle_and_disconnected_node() {
    reveal(policy::complete);
    let g: Vec<Vec<usize>> = vec![vec![0], vec![]];
    let valid = policy::valid_indices(&g);
    assert(valid);
    let result = policy::breadth_first(&g, 0);
    assert(result.is_some());
    let search = result.unwrap();
    assert(search.distances[0] == 0);
    assert(search.parents[0].is_none());
    assert(search.distances[1] == 2);
    assert(search.parents[1].is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn diamond_cycle_and_parallel_edges() {
    let g: Vec<Vec<usize>> = vec![vec![1, 1, 2], vec![3], vec![3], vec![0]];
    let valid = policy::valid_indices(&g);
    assert(valid);
    let result = policy::breadth_first(&g, 0);
    assert(result.is_some());
    let search = result.unwrap();
    proof {
        let potential = seq![0usize, 1, 1, 2];
        assert(policy::closed(g@, potential));
        exact_distance(g@, 0, search, potential, 0, seq![0usize]);
        exact_distance(g@, 0, search, potential, 1, seq![0usize, 1]);
        exact_distance(g@, 0, search, potential, 2, seq![0usize, 2]);
        exact_distance(g@, 0, search, potential, 3, seq![0usize, 1, 3]);
    }
    assert(search.distances[0] == 0);
    assert(search.distances[1] == 1);
    assert(search.distances[2] == 1);
    assert(search.distances[3] == 2);
    assert(search.order@.contains(0) && search.order@.contains(1)
        && search.order@.contains(2) && search.order@.contains(3));
    // Separate examples below require the complete first-discovery result.
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn fifo_first_discovery_preserves_the_first_parallel_slot() {
    let g: Vec<Vec<usize>> = vec![vec![2, 1, 2], vec![3], vec![3], vec![0]];
    let valid = policy::valid_indices(&g);
    assert(valid);
    let result = policy::breadth_first(&g, 0);
    assert(result.is_some());
    let search = result.unwrap();
    proof {
        assert(policy::fifo(g@, 0) == (
            seq![0usize, 1, 1, 2],
            seq![None, Some((0usize, 1usize)), Some((0usize, 0usize)), Some((2usize, 0usize))],
            seq![0usize, 2, 1, 3],
        )) by {
            reveal_with_fuel(policy::fifo_nodes, 5);
            reveal_with_fuel(policy::ordered_slots, 5);
        }
    }
    assert(search.parents[2] == Some((0usize, 0usize)));
    assert(search.parents[1] == Some((0usize, 1usize)));
    assert(search.parents[3] == Some((2usize, 0usize)));
    assert(search.order@ == seq![0usize, 2, 1, 3]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn reversed_adjacency_changes_the_equal_length_parent() {
    let g: Vec<Vec<usize>> = vec![vec![1, 2, 1], vec![3], vec![3], vec![0]];
    let valid = policy::valid_indices(&g);
    assert(valid);
    let result = policy::breadth_first(&g, 0);
    assert(result.is_some());
    let search = result.unwrap();
    proof {
        assert(policy::fifo(g@, 0) == (
            seq![0usize, 1, 1, 2],
            seq![None, Some((0usize, 0usize)), Some((0usize, 1usize)), Some((1usize, 0usize))],
            seq![0usize, 1, 2, 3],
        )) by {
            reveal_with_fuel(policy::fifo_nodes, 5);
            reveal_with_fuel(policy::ordered_slots, 5);
        }
    }
    assert(search.parents[3] == Some((1usize, 0usize)));
    assert(search.parents[0].is_none());
    assert(search.order@ == seq![0usize, 1, 2, 3]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn later_longer_paths_do_not_replace_a_direct_parent() {
    let g: Vec<Vec<usize>> = vec![vec![1, 3], vec![2], vec![3], vec![0]];
    let valid = policy::valid_indices(&g);
    assert(valid);
    let result = policy::breadth_first(&g, 0);
    assert(result.is_some());
    let search = result.unwrap();
    proof {
        assert(policy::fifo(g@, 0) == (
            seq![0usize, 1, 2, 1],
            seq![None, Some((0usize, 0usize)), Some((1usize, 0usize)), Some((0usize, 1usize))],
            seq![0usize, 1, 3, 2],
        )) by {
            reveal_with_fuel(policy::fifo_nodes, 5);
            reveal_with_fuel(policy::ordered_slots, 5);
        }
    }
    assert(search.distances[3] == 1);
    assert(search.parents[3] == Some((0usize, 1usize)));
    assert(search.parents[0].is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_rows_cover_empty_isolated_unicode_and_missing_source_boundaries() {
    proof {
        reveal(policy::source_row_matches);
        reveal_strlit("孤"); reveal_strlit("根\0");
        assert(vstd::utf8::encode_scalar(23396) == seq![229u8, 173, 164]) by (compute);
        assert(vstd::utf8::encode_scalar(26681) == seq![230u8, 160, 185]) by (compute);
        assert(vstd::utf8::encode_scalar(0) == seq![0u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    }
    let none: Vec<&str> = vec![];
    let empty: Vec<policy::UnionEdge> = vec![];
    let result = policy::source_rows(&none, &empty);
    assert(result.is_some());
    let rows = result.unwrap();
    assert(rows.len() == 0);
    let isolated: Vec<&str> = vec!["孤"];
    let rows = policy::source_rows(&isolated, &empty).unwrap();
    assert(rows.len() == 1);
    proof {
        assert(policy::source_edge_keys(empty@, "孤".spec_bytes()) =~= Set::<policy::EdgeKey>::empty());
    }
    assert(rows[0].len() == 0);
    let names: Vec<&str> = vec!["根\0"];
    let item = edge("根\0", policy::EdgeKind::Compile, "target need not be present");
    let edges = vec![policy::UnionEdge { edge: item.clone(), origin: policy::Origin::Both }];
    proof {
        assert(policy::name_members(names@).contains(item.from_bytes()));
        assert(policy::union_edge_keys(edges@) =~= seq![item.key()]);
        Seq::<policy::EdgeKey>::empty().lemma_push_to_set_commute(item.key());
        assert(Seq::<policy::EdgeKey>::empty().to_set() =~= Set::<policy::EdgeKey>::empty());
        assert(policy::union_edge_keys(edges@).to_set() =~= Set::<policy::EdgeKey>::empty().insert(item.key()));
        assert(policy::source_edge_keys(edges@, "根\0".spec_bytes())
            =~= Set::<policy::EdgeKey>::empty().insert(item.key()));
    }
    let rows = policy::source_rows(&names, &edges).unwrap();
    assert(rows.len() == 1 && rows[0].len() == 1);
    proof {
        assert(policy::reference_edge_keys(rows@[0]@)[0] == rows@[0]@[0].key());
        assert(policy::reference_edge_keys(rows@[0]@).to_set().contains(rows@[0]@[0].key()));
    }
    assert(rows[0][0].key() == item.key());
    let rejected = policy::source_rows(&none, &edges);
    assert(rejected.is_none());
    let rejected = policy::source_rows(&isolated, &edges);
    assert(rejected.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_rows_group_unsorted_duplicate_edges_into_exact_typed_slots() {
    use policy::EdgeKind::*;
    proof {
        reveal_strlit("a"); reveal_strlit("b"); reveal_strlit("c");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("b"@);
        vstd::utf8::is_ascii_chars_encode_utf8("c"@);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    }
    let names: Vec<&str> = vec!["a", "b", "c"];
    proof {
        assert forall|i: int, j: int| 0 <= i < j < names.len()
            implies policy::byte_lt(#[trigger] names@[i].spec_bytes(), #[trigger] names@[j].spec_bytes()) by {
            assert(policy::byte_prefix(names@[i].spec_bytes(), names@[j].spec_bytes(), 0));
        }
    }
    let first = edge("a", Compile, "b");
    let next = edge("a", Compile, "c");
    let parallel = edge("a", RuntimeInput, "b");
    let cycle = edge("c", JudgeTrigger, "a");
    let duplicate = edge("a", Compile, "b");
    let edges = vec![
        policy::UnionEdge { edge: cycle.clone(), origin: policy::Origin::Base },
        policy::UnionEdge { edge: parallel.clone(), origin: policy::Origin::Both },
        policy::UnionEdge { edge: next.clone(), origin: policy::Origin::Candidate },
        policy::UnionEdge { edge: first.clone(), origin: policy::Origin::Base },
        policy::UnionEdge { edge: duplicate, origin: policy::Origin::Candidate },
    ];
    proof {
        assert(policy::name_members(names@).contains("a".spec_bytes()));
        assert(policy::name_members(names@).contains("c".spec_bytes()));
        assert forall|i: int| 0 <= i < edges.len() implies
            policy::name_members(names@).contains((#[trigger] edges@[i]).edge.from_bytes()) by {
            assert(edges@[i].edge.from_bytes() == "a".spec_bytes()
                || edges@[i].edge.from_bytes() == "c".spec_bytes());
        }
        assert(policy::union_edge_keys(edges@) =~= seq![cycle.key(), parallel.key(), next.key(), first.key(), first.key()]);
        assert(policy::union_edge_keys(edges@).to_set() =~=
            Set::<policy::EdgeKey>::empty().insert(cycle.key()).insert(parallel.key()).insert(next.key()).insert(first.key()));
        assert(policy::source_edge_keys(edges@, "a".spec_bytes()) =~=
            Set::<policy::EdgeKey>::empty().insert(first.key()).insert(next.key()).insert(parallel.key()));
        assert(policy::source_edge_keys(edges@, "b".spec_bytes()) =~= Set::<policy::EdgeKey>::empty());
        assert(policy::source_edge_keys(edges@, "c".spec_bytes()) =~=
            Set::<policy::EdgeKey>::empty().insert(cycle.key()));
        assert(Set::<policy::EdgeKey>::empty().insert(first.key()).insert(next.key()).insert(parallel.key()).len() == 3);
    }
    let rows = policy::source_rows(&names, &edges).unwrap();
    assert(rows.len() == 3);
    proof { reveal(policy::source_row_matches); }
    assert(rows[0].len() == 3 && rows[1].len() == 0 && rows[2].len() == 1);
    proof {
        reveal(policy::ordered_edges);
        assert forall|i: int| 0 <= i < rows@[0].len() implies
            (#[trigger] rows@[0]@[i]).key() == first.key()
                || rows@[0]@[i].key() == next.key() || rows@[0]@[i].key() == parallel.key() by {
            assert(policy::reference_edge_keys(rows@[0]@)[i] == rows@[0]@[i].key());
            assert(policy::reference_edge_keys(rows@[0]@).to_set().contains(rows@[0]@[i].key()));
        }
        assert(rows@[0]@[0].order_spec(rows@[0]@[1]) == core::cmp::Ordering::Less);
        assert(rows@[0]@[1].order_spec(rows@[0]@[2]) == core::cmp::Ordering::Less);
        policy::edge_order_laws(rows@[0]@[0], rows@[0]@[1], rows@[0]@[2]);
        policy::edge_order_laws(rows@[0]@[1], rows@[0]@[2], rows@[0]@[2]);
        assert(policy::reference_edge_keys(rows@[2]@)[0] == rows@[2]@[0].key());
        assert(policy::reference_edge_keys(rows@[2]@).to_set().contains(rows@[2]@[0].key()));
    }
    assert(rows[0][0].key() == first.key());
    assert(rows[0][1].key() == next.key());
    assert(rows[0][2].key() == parallel.key());
    assert(rows[2][0].key() == cycle.key());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn graph_names_preserve_empty_inputs() {
    let edges: Vec<policy::UnionEdge> = vec![];
    let roots: Vec<&str> = vec![];
    assert(policy::endpoint_names(edges@) =~= Set::<Seq<u8>>::empty());
    assert(policy::name_members(roots@) =~= Set::<Seq<u8>>::empty());
    assert((policy::endpoint_names(edges@) + policy::name_members(roots@)) =~= Set::<Seq<u8>>::empty());
    assert(Set::<Seq<u8>>::empty().len() == 0);
    let names = policy::graph_names(&edges, &roots);
    assert(names.len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn graph_names_preserve_duplicate_isolated_unicode_and_empty_roots() {
    proof {
        reveal_strlit("x"); reveal_strlit("q"); reveal_strlit("根\0");
        reveal_strlit("");
        vstd::utf8::is_ascii_chars_encode_utf8("x"@);
        vstd::utf8::is_ascii_chars_encode_utf8("q"@);
        assert(vstd::utf8::encode_scalar(26681) == seq![230u8, 160, 185]) by (compute);
        assert(vstd::utf8::encode_scalar(0) == seq![0u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
        assert("x".spec_bytes() == seq![120u8]);
        assert("q".spec_bytes() == seq![113u8]);
        assert("根\0".spec_bytes() == seq![230u8, 160, 185, 0]);
        assert("".spec_bytes() == Seq::<u8>::empty());
    }
    let edges: Vec<policy::UnionEdge> = vec![];
    let roots = vec!["x", "", "x", "根\0"];
    assert(policy::endpoint_names(edges@) =~= Set::<Seq<u8>>::empty());
    assert(policy::name_members(roots@) =~= set!["x".spec_bytes(), "".spec_bytes(), "根\0".spec_bytes()]);
    let names = policy::graph_names(&edges, &roots);
    assert(policy::name_members(names@) =~= set!["x".spec_bytes(), "".spec_bytes(), "根\0".spec_bytes()]);
    assert(!policy::name_members(names@).contains("q".spec_bytes()));
    let absent = policy::name_index(&names, "q");
    if let Some(i) = absent {
        assert(policy::name_members(names@).contains(names@[i as int].spec_bytes()));
    }
    assert(absent.is_none());
    let valid = policy::valid_names(&names);
    assert(valid);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn graph_names_preserve_both_endpoints_duplicate_kinds_origins_and_roots() {
    proof {
        reveal_strlit("a"); reveal_strlit("b"); reveal_strlit("q"); reveal_strlit("z");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("b"@);
        vstd::utf8::is_ascii_chars_encode_utf8("q"@);
        vstd::utf8::is_ascii_chars_encode_utf8("z"@);
        reveal_with_fuel(vstd::utf8::encode_utf8, 2);
        assert("a".spec_bytes() == seq![97u8]);
        assert("b".spec_bytes() == seq![98u8]);
        assert("q".spec_bytes() == seq![113u8]);
        assert("z".spec_bytes() == seq![122u8]);
    }
    let first = edge("z", policy::EdgeKind::Compile, "b");
    let second = edge("b", policy::EdgeKind::RuntimeInput, "a");
    let duplicate = edge("z", policy::EdgeKind::JudgeTrigger, "b");
    let edges = vec![
        policy::UnionEdge { edge: first, origin: policy::Origin::Base },
        policy::UnionEdge { edge: second, origin: policy::Origin::Both },
        policy::UnionEdge { edge: duplicate, origin: policy::Origin::Candidate },
    ];
    let roots = vec!["q", "a", "q"];
    assert(roots@[0].spec_bytes() == seq![113u8]);
    assert(roots@[1].spec_bytes() == seq![97u8]);
    assert(roots@[2].spec_bytes() == seq![113u8]);
    assert(policy::endpoint_names(edges@) =~= set![seq![122u8], seq![98u8], seq![97u8]]);
    assert(policy::name_members(roots@) =~= set![seq![113u8], seq![97u8]]);
    assert((policy::endpoint_names(edges@) + policy::name_members(roots@)) =~=
        set![seq![97u8], seq![98u8], seq![113u8], seq![122u8]]);
    assert(set![seq![97u8], seq![98u8], seq![113u8], seq![122u8]].len() == 4);
    let names = policy::graph_names(&edges, &roots);
    assert(names.len() == 4);
    assert(policy::name_members(names@) =~= set![seq![97u8], seq![98u8], seq![113u8], seq![122u8]]);
    assert(policy::source_names_present(names@, edges@));
    let valid = policy::valid_names(&names);
    assert(valid);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn topology_preserves_the_exact_empty_graph() {
    let edges: Vec<policy::UnionEdge> = vec![];
    let roots: Vec<&str> = vec![];
    assert(policy::endpoint_names(edges@) =~= Set::<Seq<u8>>::empty());
    assert(policy::name_members(roots@) =~= Set::<Seq<u8>>::empty());
    assert((policy::endpoint_names(edges@) + policy::name_members(roots@)) =~= Set::<Seq<u8>>::empty());
    assert(Set::<Seq<u8>>::empty().len() == 0);
    let graph = policy::topology(&edges, &roots);
    assert(graph.names.len() == 0);
    assert(graph.edge_rows.len() == 0);
    assert(graph.indices.rows.len() == 0);
    assert(graph.indices.roots.len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn topology_preserves_isolated_duplicate_unicode_nul_roots() {
    let edges: Vec<policy::UnionEdge> = vec![];
    let roots = vec!["根\0", "根\0"];
    assert(policy::endpoint_names(edges@) =~= Set::<Seq<u8>>::empty());
    assert(policy::name_members(roots@) =~= Set::<Seq<u8>>::empty().insert("根\0".spec_bytes()));
    assert((policy::endpoint_names(edges@) + policy::name_members(roots@)) =~=
        Set::<Seq<u8>>::empty().insert("根\0".spec_bytes()));
    assert(Set::<Seq<u8>>::empty().insert("根\0".spec_bytes()).len() == 1);
    let graph = policy::topology(&edges, &roots);
    assert(graph.names.len() == 1);
    assert(policy::name_members(graph.names@).contains(graph.names@[0].spec_bytes()));
    assert(graph.names@[0].spec_bytes() == "根\0".spec_bytes());
    proof {
        reveal(policy::source_row_matches);
        assert(policy::source_edge_keys(edges@, "根\0".spec_bytes()) =~= Set::<policy::EdgeKey>::empty());
    }
    assert(graph.edge_rows.len() == 1 && graph.edge_rows[0].len() == 0);
    assert(graph.indices.rows.len() == 1 && graph.indices.rows[0].len() == 0);
    assert(graph.indices.roots.len() == 2);
    assert(graph.indices.roots[0] == 0 && graph.indices.roots[1] == 0);
    let propagation = policy::propagate(&graph, graph.indices.roots[0]);
    proof {
        reveal_with_fuel(policy::fifo_nodes, 3);
        reveal_with_fuel(policy::ordered_slots, 2);
    }
    assert(propagation.rows.len() == 1);
    assert(propagation.rows[0].node.spec_bytes() == "根\0".spec_bytes());
    assert(propagation.rows[0].outgoing.len() == 0);
    assert(policy::predecessor_keys(propagation.predecessors.view()).to_set()
        =~= set![("根\0".spec_bytes(), None::<policy::EdgeKey>)]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn topology_preserves_exact_parallel_cycle_self_loop_and_root_slots() {
    use policy::EdgeKind::*;
    proof {
        reveal_strlit("a"); reveal_strlit("b");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("b"@);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    }
    let forward = edge("a", RuntimeInput, "b");
    let parallel = edge("a", TestExecution, "b");
    let backward = edge("b", Compile, "a");
    let self_loop = edge("b", Compile, "b");
    let edges = vec![
        policy::UnionEdge { edge: self_loop.clone(), origin: policy::Origin::Both },
        policy::UnionEdge { edge: parallel.clone(), origin: policy::Origin::Candidate },
        policy::UnionEdge { edge: backward.clone(), origin: policy::Origin::Base },
        policy::UnionEdge { edge: forward.clone(), origin: policy::Origin::Both },
        policy::UnionEdge { edge: forward.clone(), origin: policy::Origin::Base },
    ];
    let roots = vec!["b", "a", "b"];
    proof {
        assert(policy::endpoint_names(edges@) =~= set![seq![97u8], seq![98u8]]);
        assert(policy::name_members(roots@) =~= set![seq![97u8], seq![98u8]]);
        assert((policy::endpoint_names(edges@) + policy::name_members(roots@)) =~= set![seq![97u8], seq![98u8]]);
        assert(set![seq![97u8], seq![98u8]].len() == 2);
        assert(policy::union_edge_keys(edges@) =~=
            seq![self_loop.key(), parallel.key(), backward.key(), forward.key(), forward.key()]);
        assert(policy::union_edge_keys(edges@).to_set() =~=
            set![self_loop.key(), parallel.key(), backward.key(), forward.key()]);
        assert(policy::source_edge_keys(edges@, "a".spec_bytes()) =~= set![forward.key(), parallel.key()]);
        assert(policy::source_edge_keys(edges@, "b".spec_bytes()) =~= set![backward.key(), self_loop.key()]);
        assert(set![forward.key(), parallel.key()].len() == 2);
        assert(set![backward.key(), self_loop.key()].len() == 2);
    }
    let graph = policy::topology(&edges, &roots);
    assert(graph.names.len() == 2);
    assert(policy::name_members(graph.names@).contains(graph.names@[0].spec_bytes()));
    assert(policy::name_members(graph.names@).contains(graph.names@[1].spec_bytes()));
    assert(!policy::byte_lt(seq![98u8], seq![97u8]));
    assert(!policy::byte_lt(seq![97u8], seq![97u8]));
    assert(!policy::byte_lt(seq![98u8], seq![98u8]));
    assert(graph.names@[0].spec_bytes() == seq![97u8]);
    assert(graph.names@[1].spec_bytes() == seq![98u8]);
    proof {
        reveal(policy::source_row_matches);
        reveal(policy::ordered_edges);
    }
    assert(graph.edge_rows.len() == 2);
    assert(graph.edge_rows[0].len() == 2 && graph.edge_rows[1].len() == 2);
    proof {
        assert forall|slot: int| 0 <= slot < 2 implies {
            let key = (#[trigger] graph.edge_rows@[1]@[slot]).key();
            key == backward.key() || key == self_loop.key()
        } by {
            assert(policy::reference_edge_keys(graph.edge_rows@[1]@)[slot] == graph.edge_rows@[1]@[slot].key());
            assert(policy::reference_edge_keys(graph.edge_rows@[1]@).to_set().contains(graph.edge_rows@[1]@[slot].key()));
        }
        policy::edge_order_laws(graph.edge_rows@[1]@[0], graph.edge_rows@[1]@[1], graph.edge_rows@[1]@[1]);
        assert(graph.edge_rows@[1]@[0].key() == backward.key());
        assert(graph.edge_rows@[1]@[1].key() == self_loop.key());
        assert forall|slot: int| 0 <= slot < 2
            implies (#[trigger] graph.edge_rows@[0]@[slot]).to_bytes() == seq![98u8] by {
            assert(policy::reference_edge_keys(graph.edge_rows@[0]@)[slot] == graph.edge_rows@[0]@[slot].key());
            assert(policy::reference_edge_keys(graph.edge_rows@[0]@).to_set().contains(graph.edge_rows@[0]@[slot].key()));
        }
    }
    assert(graph.indices.rows[0][0] == 1 && graph.indices.rows[0][1] == 1);
    assert(graph.indices.rows[1][0] == 0 && graph.indices.rows[1][1] == 1);
    assert(graph.indices.roots.len() == 3);
    assert(graph.indices.roots[0] == 1 && graph.indices.roots[1] == 0 && graph.indices.roots[2] == 1);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn propagation_preserves_fifo_source_names_parallel_slots_cycles_and_self_loops() {
    use policy::EdgeKind::*;
    proof { predecessor_ascii_order(); }
    let forward = edge("a", RuntimeInput, "b");
    let parallel = edge("a", TestExecution, "b");
    let backward = edge("b", Compile, "a");
    let self_loop = edge("b", Compile, "b");
    let graph = policy::Topology {
        names: vec!["a", "b"],
        edge_rows: vec![vec![&forward, &parallel], vec![&backward, &self_loop]],
        indices: policy::NameIndices { rows: vec![vec![1, 1], vec![0, 1]], roots: vec![1] },
    };
    assert(policy::ordered_names(graph.names@));
    assert(policy::valid_graph(graph.indices.rows@));
    let propagation = policy::propagate(&graph, graph.indices.roots[0]);
    proof {
        reveal_with_fuel(policy::fifo_nodes, 4);
        reveal_with_fuel(policy::ordered_slots, 4);
        assert(policy::fifo(graph.indices.rows@, 1).2[0] == 1);
        assert(policy::fifo(graph.indices.rows@, 1).2[1] == 0);
    }
    assert(propagation.rows.len() == 2);
    assert(propagation.rows[0].node.spec_bytes() == "b".spec_bytes());
    assert(propagation.rows[1].node.spec_bytes() == "a".spec_bytes());
    assert(propagation.rows[0].outgoing.len() == 2);
    assert(propagation.rows[1].outgoing.len() == 2);
    assert(propagation.rows[0].outgoing[0].key() == backward.key());
    assert(propagation.rows[0].outgoing[1].key() == self_loop.key());
    assert(propagation.rows[1].outgoing[0].key() == forward.key());
    assert(propagation.rows[1].outgoing[1].key() == parallel.key());
    assert(policy::predecessor_keys(propagation.predecessors.view()).to_set()
        =~= set![("b".spec_bytes(), None::<policy::EdgeKey>),
            ("a".spec_bytes(), Some(backward.key()))]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn propagation_omits_unreachable_rows_and_keeps_first_parallel_and_shortest_parents() {
    use policy::EdgeKind::*;
    proof { predecessor_ascii_order(); }
    let first = edge("", Compile, "a");
    let parallel = edge("", RuntimeInput, "a");
    let direct = edge("", TestExecution, "b");
    let longer = edge("a", BuildInput, "b");
    let backward = edge("b", JudgeTrigger, "");
    let self_loop = edge("b", Compile, "b");
    let graph = policy::Topology {
        names: vec!["", "a", "b", "z"],
        edge_rows: vec![vec![&first, &parallel, &direct], vec![&longer], vec![&backward, &self_loop], vec![]],
        indices: policy::NameIndices { rows: vec![vec![1, 1, 2], vec![2], vec![0, 2], vec![]], roots: vec![0, 3] },
    };
    proof {
        assert forall|i: int, j: int| 0 <= i < j < graph.names.len()
            implies policy::byte_lt(#[trigger] graph.names@[i].spec_bytes(), #[trigger] graph.names@[j].spec_bytes()) by {
            assert(policy::byte_prefix(graph.names@[i].spec_bytes(), graph.names@[j].spec_bytes(), 0));
        }
    }
    assert(policy::valid_graph(graph.indices.rows@));
    let propagation = policy::propagate(&graph, graph.indices.roots[0]);
    proof {
        reveal_with_fuel(policy::fifo_nodes, 5);
        reveal_with_fuel(policy::ordered_slots, 5);
        assert(policy::fifo(graph.indices.rows@, 0).2[0] == 0);
        assert(policy::fifo(graph.indices.rows@, 0).2[1] == 1);
        assert(policy::fifo(graph.indices.rows@, 0).2[2] == 2);
    }
    assert(propagation.rows.len() == 3);
    assert(propagation.rows[0].node.spec_bytes() == "".spec_bytes());
    assert(propagation.rows[1].node.spec_bytes() == "a".spec_bytes());
    assert(propagation.rows[2].node.spec_bytes() == "b".spec_bytes());
    assert(propagation.rows[0].outgoing.len() == 3);
    assert(propagation.rows[1].outgoing.len() == 1);
    assert(propagation.rows[2].outgoing.len() == 2);
    assert(propagation.rows[1].outgoing[0].key() == longer.key());
    assert(policy::predecessor_keys(propagation.predecessors.view()).to_set()
        =~= set![("".spec_bytes(), None::<policy::EdgeKey>),
            ("a".spec_bytes(), Some(first.key())), ("b".spec_bytes(), Some(direct.key()))]);
    let isolated = policy::propagate(&graph, graph.indices.roots[1]);
    assert(isolated.rows.len() == 1);
    assert(isolated.rows[0].node.spec_bytes() == "z".spec_bytes());
    assert(isolated.rows[0].outgoing.len() == 0);
    assert(policy::predecessor_keys(isolated.predecessors.view()).to_set()
        =~= set![("z".spec_bytes(), None::<policy::EdgeKey>)]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cause_set_preserves_empty_new_duplicate_and_clone_membership() {
    proof { predecessor_ascii_order(); }
    let mut causes = policy::CauseSet::default();
    let len = causes.len();
    let first = causes.first();
    assert(len == 0);
    assert(first.is_none());
    let missing = causes.contains("a");
    assert(!missing);
    let added = causes.insert("a");
    let len = causes.len();
    assert(added && len == 1);
    let present = causes.contains("a");
    assert(present);
    let first = causes.first();
    let first = first.unwrap();
    assert(policy::cause_name(*first) == "a".spec_bytes());
    let same_bytes = "a".to_owned();
    let ghost before = causes.view();
    let duplicate = causes.insert(same_bytes.as_str());
    let len = causes.len();
    assert(!duplicate && len == 1 && causes.view() == before);
    let added = causes.insert("");
    let len = causes.len();
    assert(added && len == 2);
    let ghost original = causes.keys();
    let mut copy = causes.clone();
    assert(copy.keys() == original);
    let added = copy.insert("z");
    let copy_len = copy.len();
    let original_len = causes.len();
    assert(added && copy_len == 3);
    assert(causes.keys() == original && original_len == 2);
    let absent = causes.contains("z");
    let present = copy.contains("z");
    assert(!absent && present);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn cause_set_normalizes_unordered_unicode_nul_and_repeated_input() {
    let names = vec!["根\0".to_owned(), "".to_owned(), "é".to_owned(),
        "根\0".to_owned(), "a\0".to_owned()];
    let causes = policy::CauseSet::from_names(&names);
    proof {
        reveal(policy::cause_keys);
        assert(policy::cause_keys(names@) =~= seq!["根\0".spec_bytes(), "".spec_bytes(),
            "é".spec_bytes(), "根\0".spec_bytes(), "a\0".spec_bytes()]);
        assert(causes.keys().to_set() =~= set!["根\0".spec_bytes(), "".spec_bytes(),
            "é".spec_bytes(), "a\0".spec_bytes()]);
    }
    let present = causes.contains("根\0");
    assert(present);
    let copy = causes.clone();
    assert(copy.keys() == causes.keys());
    let entries = copy.entries();
    assert(policy::ordered_causes(entries@));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn seed_propagation_stream_keeps_every_position_and_all_seed_fields() {
    let edges: Vec<policy::UnionEdge> = vec![];
    let seeds = vec![seed("same", "根\0", "first", "one"),
        seed("same", "", "second", "two"), seed("same", "根\0", "first", "one")];
    let topology = policy::seed_topology(&edges, &seeds);
    let mut stream = policy::SeedPropagations::new(&topology, &seeds);
    assert(stream.cursor() == 0);
    let first = stream.next().unwrap();
    assert(stream.cursor() == 1);
    assert(first.seed.id@ == "same"@ && first.seed.node@ == "根\0"@);
    assert(first.seed.reference@ == "first"@ && first.seed.reason@ == "one"@);
    assert(policy::seed_propagation_matches(first, &topology, seeds@, 0));
    let second = stream.next().unwrap();
    assert(stream.cursor() == 2);
    assert(second.seed.id@ == "same"@ && second.seed.node@ == ""@);
    assert(second.seed.reference@ == "second"@ && second.seed.reason@ == "two"@);
    assert(policy::seed_propagation_matches(second, &topology, seeds@, 1));
    let third = stream.next().unwrap();
    assert(stream.cursor() == 3);
    assert(third.seed.key() == first.seed.key());
    assert(policy::seed_propagation_matches(third, &topology, seeds@, 2));
    let end = stream.next();
    assert(end.is_none() && stream.cursor() == 3);
    let stable_end = stream.next();
    assert(stable_end.is_none() && stream.cursor() == 3);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_seed_stream_stays_finished_and_mismatched_root_counts_are_inadmissible() {
    let edges: Vec<policy::UnionEdge> = vec![];
    let seeds: Vec<policy::Seed> = vec![];
    let topology = policy::seed_topology(&edges, &seeds);
    let mut stream = policy::SeedPropagations::new(&topology, &seeds);
    let end = stream.next();
    assert(end.is_none() && stream.cursor() == 0);
    let end_again = stream.next();
    assert(end_again.is_none() && stream.cursor() == 0);
    let other_seeds = vec![seed("other", "root", "ref", "reason")];
    assert(!policy::seed_topology_matches(&topology, other_seeds@));
    let other_topology = policy::seed_topology(&edges, &other_seeds);
    assert(!policy::seed_topology_matches(&other_topology, seeds@));
}

fn traversed(from: &str, kind: policy::EdgeKind, to: &str, cause: &str) -> (out: policy::TraversedEdge)
    ensures out.edge.key() == (from.spec_bytes(), kind.rank_spec(), to.spec_bytes()),
        out.seeds.keys().to_set() == Set::<Seq<u8>>::empty().insert(cause.spec_bytes()),
{
    let mut seeds = policy::CauseSet::default();
    seeds.insert(cause);
    policy::TraversedEdge { edge: edge(from, kind, to), seeds }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selection_rejects_every_nonexecution_kind_and_empty_inventory() {
    let empty: Vec<policy::TraversedEdge> = vec![];
    let no_targets = policy::selected_test_targets(&empty);
    proof {
        reveal_with_fuel(policy::selected_target_keys, 6);
        assert(policy::selected_target_keys(empty@) == Set::<Seq<u8>>::empty());
        empty_target_keys_mean_no_names(no_targets@);
    }
    assert(no_targets.len() == 0);
    let rows = vec![
        traversed("test:source", policy::EdgeKind::Compile, "test:t", "cause"),
        traversed("test:source", policy::EdgeKind::BuildInput, "test:t", "cause"),
        traversed("test:source", policy::EdgeKind::RuntimeInput, "test:t", "cause"),
        traversed("test:source", policy::EdgeKind::JudgeTrigger, "test:t", "cause"),
    ];
    let targets = policy::selected_test_targets(&rows);
    proof {
        reveal_with_fuel(policy::selected_target_keys, 6);
        assert(policy::selected_target_keys(rows@) == Set::<Seq<u8>>::empty());
        empty_target_keys_mean_no_names(targets@);
    }
    assert(targets.len() == 0);
}

proof fn empty_target_keys_mean_no_names(names: Seq<String>)
    requires policy::cause_keys(names).to_set() == Set::<Seq<u8>>::empty(),
    ensures names.len() == 0,
{
    reveal(policy::cause_keys);
    let keys = policy::cause_keys(names);
    assert(keys.len() == names.len());
    if names.len() > 0 {
        assert(keys.to_set().contains(keys[0]));
    }
}

proof fn selected_prefix_step(rows: Seq<policy::TraversedEdge>, at: int)
    requires 0 <= at < rows.len(),
    ensures policy::selected_target_keys(rows.take(at + 1)) ==
        if rows[at].edge.kind == policy::EdgeKind::TestExecution {
            policy::selected_target_keys(rows.take(at)).insert(rows[at].edge.to_bytes())
        } else { policy::selected_target_keys(rows.take(at)) },
{
    reveal(policy::selected_target_keys);
    assert(rows.take(at + 1).drop_last() =~= rows.take(at));
    assert(rows.take(at + 1).last() == rows[at]);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn selection_is_sorted_unique_and_keeps_complete_unicode_nul_targets() {
    let original = traversed("source\0", policy::EdgeKind::TestExecution, "根\0", "原因\0");
    let copied = original.clone();
    assert(copied.edge.key() == original.edge.key());
    assert(copied.seeds.keys() == original.seeds.keys());
    let rows = vec![original,
        traversed("other", policy::EdgeKind::TestExecution, "z", ""),
        traversed("other", policy::EdgeKind::TestExecution, "", "unrelated"),
        traversed("other", policy::EdgeKind::TestExecution, "é", "cause"),
        copied,
        traversed("test:ignored", policy::EdgeKind::Compile, "not-selected", "cause"),
    ];
    let targets = policy::selected_test_targets(&rows);
    proof {
        assert(policy::selected_target_keys(rows@.take(0)) == Set::<Seq<u8>>::empty()) by {
            reveal(policy::selected_target_keys);
        }
        selected_prefix_step(rows@, 0);
        selected_prefix_step(rows@, 1);
        selected_prefix_step(rows@, 2);
        selected_prefix_step(rows@, 3);
        selected_prefix_step(rows@, 4);
        selected_prefix_step(rows@, 5);
        assert(rows@.take(6) =~= rows@);
    }
    assert(policy::cause_keys(targets@).to_set() == Set::<Seq<u8>>::empty()
        .insert("".spec_bytes()).insert("z".spec_bytes())
        .insert("é".spec_bytes()).insert("根\0".spec_bytes()));
    assert(policy::ordered_causes(targets@));
}

fn seed(id: &str, node: &str, reference: &str, reason: &str) -> (out: policy::Seed)
    ensures out.key() == (id.spec_bytes(), node.spec_bytes(), reference.spec_bytes(), reason.spec_bytes()),
        out.id@ == id@, out.node@ == node@, out.reference@ == reference@, out.reason@ == reason@,
{
    policy::Seed { id: id.to_owned(), node: node.to_owned(),
        reference: reference.to_owned(), reason: reason.to_owned() }
}

fn seed_before(a: &policy::Seed, b: &policy::Seed)
    requires a.less(b), !a.same(b),
{
    use core::cmp::Ordering;
    proof { policy::seed_order_laws(a, b, b); }
    let forward = a.cmp(b);
    let reverse = b.cmp(a);
    let partial = a.partial_cmp(b);
    let equal = a == b;
    assert(forward == Ordering::Less && reverse == Ordering::Greater);
    assert(partial == Some(Ordering::Less));
    assert(!equal);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn seed_order_uses_complete_id_node_reference_reason_in_that_order() {
    proof {
        reveal_strlit(""); reveal_strlit("a"); reveal_strlit("z"); reveal_strlit("a\0");
        vstd::utf8::is_ascii_chars_encode_utf8("a"@);
        vstd::utf8::is_ascii_chars_encode_utf8("z"@);
        vstd::utf8::is_ascii_chars_encode_utf8("a\0"@);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
        assert("".spec_bytes() == Seq::<u8>::empty());
        assert("a".spec_bytes() == seq![97u8]);
        assert("z".spec_bytes() == seq![122u8]);
        assert("a\0".spec_bytes() == seq![97u8, 0]);
        assert(policy::byte_prefix("".spec_bytes(), "a".spec_bytes(), 0));
        assert(policy::byte_lt("".spec_bytes(), "a".spec_bytes()));
        assert(policy::byte_prefix("a".spec_bytes(), "a\0".spec_bytes(), 1));
        assert(policy::byte_lt("a".spec_bytes(), "a\0".spec_bytes()));
    }
    let low = seed("", "z", "z", "z");
    let high = seed("a", "", "", "");
    seed_before(&low, &high);
    let low = seed("a", "", "z", "z");
    let high = seed("a", "a", "", "");
    seed_before(&low, &high);
    let low = seed("a", "a", "", "z");
    let high = seed("a", "a", "a", "");
    seed_before(&low, &high);
    let low = seed("a", "a", "a", "a");
    let high = seed("a", "a", "a", "a\0");
    seed_before(&low, &high);
    let cloned = high.clone();
    assert(cloned.key() == high.key());
    let equal = cloned == high;
    let order = cloned.cmp(&high);
    assert(equal && order == core::cmp::Ordering::Equal);
    let kind = policy::EdgeKind::TestExecution;
    let cloned_kind = kind.clone();
    assert(cloned_kind == kind);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn seed_btree_membership_distinguishes_the_last_field_and_repeated_values() {
    use std::collections::BTreeSet;
    proof { policy::seed_obeys_cmp(); }
    let a = seed("same", "根\0", "ref", "");
    let b = seed("same", "根\0", "ref", "\0");
    let c = seed("same", "根\0", "ref", "\0x");
    proof {
        reveal_strlit(""); reveal_strlit("\0"); reveal_strlit("\0x");
        vstd::utf8::is_ascii_chars_encode_utf8("\0"@);
        vstd::utf8::is_ascii_chars_encode_utf8("\0x"@);
        reveal_with_fuel(vstd::utf8::encode_utf8, 3);
        assert(a.key().3 != b.key().3 && a.key().3 != c.key().3 && b.key().3 != c.key().3);
    }
    let mut values: BTreeSet<policy::Seed> = BTreeSet::new();
    values.insert(c.clone()); values.insert(a.clone()); values.insert(b.clone()); values.insert(a.clone());
    assert(values@ == Set::<policy::Seed>::empty().insert(c).insert(a).insert(b));
    assert(values.len() == 3);
    let present = values.contains(&a);
    assert(present);
    let present = values.contains(&b);
    assert(present);
    let present = values.contains(&c);
    assert(present);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn seed_roots_and_topology_preserve_actual_nodes_and_duplicate_positions() {
    let empty: Vec<policy::Seed> = vec![];
    let roots = policy::seed_roots(&empty);
    assert(roots.len() == 0);
    let edges: Vec<policy::UnionEdge> = vec![];
    assert(policy::endpoint_names(edges@) =~= Set::<Seq<u8>>::empty());
    assert(policy::seed_node_names(empty@) =~= Set::<Seq<u8>>::empty());
    assert((policy::endpoint_names(edges@) + policy::seed_node_names(empty@)) =~= Set::<Seq<u8>>::empty());
    let graph = policy::seed_topology(&edges, &empty);
    assert(graph.names.len() == 0 && graph.indices.roots.len() == 0);
    let first = seed("id differs", "", "reference differs", "reason differs");
    let next = seed("another id", "根\0", "another reference", "another reason");
    let seeds = vec![first.clone(), next, first];
    let roots = policy::seed_roots(&seeds);
    assert(roots.len() == 3);
    assert(roots@[0].spec_bytes() == "".spec_bytes());
    assert(roots@[1].spec_bytes() == "根\0".spec_bytes());
    assert(roots@[2].spec_bytes() == "".spec_bytes());
    let graph = policy::seed_topology(&edges, &seeds);
    assert(graph.indices.roots.len() == 3);
    assert(graph.names@[graph.indices.roots@[0] as int].spec_bytes() == "".spec_bytes());
    assert(graph.names@[graph.indices.roots@[1] as int].spec_bytes() == "根\0".spec_bytes());
    assert(graph.names@[graph.indices.roots@[2] as int].spec_bytes() == "".spec_bytes());
}

fn predecessor(node: &str, parent: Option<policy::Edge>) -> (out: policy::Predecessor)
    ensures out.name() == node.spec_bytes(), out.parent == parent,
{
    policy::Predecessor { node: node.to_owned(), parent }
}

proof fn predecessor_ascii_order()
    ensures policy::byte_lt("".spec_bytes(), "a".spec_bytes()),
        policy::byte_lt("".spec_bytes(), "b".spec_bytes()),
        policy::byte_lt("a".spec_bytes(), "b".spec_bytes()),
        "a".spec_bytes() == seq![97u8], "b".spec_bytes() == seq![98u8],
        "z".spec_bytes() == seq![122u8], "".spec_bytes() == Seq::<u8>::empty(),
{
    reveal_strlit(""); reveal_strlit("a"); reveal_strlit("b"); reveal_strlit("z");
    vstd::utf8::is_ascii_chars_encode_utf8("a"@);
    vstd::utf8::is_ascii_chars_encode_utf8("b"@);
    vstd::utf8::is_ascii_chars_encode_utf8("z"@);
    reveal_with_fuel(vstd::utf8::encode_utf8, 3);
    assert(policy::byte_prefix("a".spec_bytes(), "b".spec_bytes(), 0));
    assert(policy::byte_prefix("".spec_bytes(), "a".spec_bytes(), 0));
    assert(policy::byte_prefix("".spec_bytes(), "b".spec_bytes(), 0));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn predecessor_tree_preserves_empty_unicode_root_and_missing_boundaries() {
    let rows: Vec<policy::Predecessor> = vec![];
    let tree = policy::PredecessorTree::new(rows).unwrap();
    proof { reveal(policy::predecessor_chain); }
    let missing = tree.witness("");
    assert(missing.is_none());
    let rows = vec![predecessor("根\0", None)];
    let tree = policy::PredecessorTree::new(rows).unwrap();
    proof {
        policy::predecessor_unique(tree.view(), "根\0".spec_bytes(), 0);
        reveal(policy::predecessor_chain);
    }
    let root = tree.witness("根\0");
    assert(root.is_some() && root.unwrap().len() == 0);
    let cloned = tree.clone();
    let root = cloned.witness("根\0");
    assert(root.is_some() && root.unwrap().len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn predecessor_tree_returns_exact_root_to_target_order_and_edge_kinds() {
    proof { predecessor_ascii_order(); }
    let first = edge("", policy::EdgeKind::Compile, "a");
    let last = edge("a", policy::EdgeKind::TestExecution, "b");
    let rows = vec![predecessor("", None), predecessor("a", Some(first.clone())), predecessor("b", Some(last.clone()))];
    assert(policy::ordered_predecessors(rows@));
    let tree = policy::PredecessorTree::new(rows).unwrap();
    proof {
        policy::predecessor_unique(tree.view(), "".spec_bytes(), 0);
        policy::predecessor_unique(tree.view(), "a".spec_bytes(), 1);
        policy::predecessor_unique(tree.view(), "b".spec_bytes(), 2);
        reveal_with_fuel(policy::predecessor_chain, 4);
    }
    let path = tree.witness("b").unwrap();
    assert(path.len() == 2);
    assert(path@[0].key() == first.key() && path@[1].key() == last.key());
    assert(policy::predecessor_path(tree.view(), "b".spec_bytes(), policy::owned_edge_keys(path@)));
    let prefix = tree.witness("a").unwrap();
    assert(prefix.len() == 1 && prefix@[0].key() == first.key());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn predecessor_tree_rejects_unsorted_and_independently_allocated_duplicate_names() {
    proof { predecessor_ascii_order(); }
    let rows = vec![predecessor("b", None), predecessor("a", None)];
    assert(!policy::ordered_predecessors(rows@));
    let rejected = policy::PredecessorTree::new(rows);
    assert(rejected.is_none());
    let rows = vec![predecessor("a", None), predecessor("a", None)];
    assert(!policy::ordered_predecessors(rows@));
    let rejected = policy::PredecessorTree::new(rows);
    assert(rejected.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn predecessor_tree_rejects_cycles_and_self_loops_with_a_finite_budget() {
    proof { predecessor_ascii_order(); }
    let forward = edge("a", policy::EdgeKind::Compile, "b");
    let backward = edge("b", policy::EdgeKind::RuntimeInput, "a");
    let rows = vec![predecessor("a", Some(backward)), predecessor("b", Some(forward))];
    let tree = policy::PredecessorTree::new(rows).unwrap();
    proof {
        policy::predecessor_unique(tree.view(), "a".spec_bytes(), 0);
        policy::predecessor_unique(tree.view(), "b".spec_bytes(), 1);
        reveal_with_fuel(policy::predecessor_chain, 4);
    }
    let rejected = tree.witness("a");
    assert(rejected.is_none());
    let rejected = tree.witness("b");
    assert(rejected.is_none());
    let rows = vec![predecessor("a", Some(edge("a", policy::EdgeKind::Compile, "a")))];
    let tree = policy::PredecessorTree::new(rows).unwrap();
    proof {
        policy::predecessor_unique(tree.view(), "a".spec_bytes(), 0);
        reveal_with_fuel(policy::predecessor_chain, 3);
    }
    let rejected = tree.witness("a");
    assert(rejected.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn predecessor_tree_rejects_a_missing_parent_and_a_mismatched_target() {
    proof { predecessor_ascii_order(); }
    let rows = vec![predecessor("b", Some(edge("a", policy::EdgeKind::Compile, "b")))];
    let tree = policy::PredecessorTree::new(rows).unwrap();
    proof {
        policy::predecessor_unique(tree.view(), "b".spec_bytes(), 0);
        reveal_with_fuel(policy::predecessor_chain, 3);
    }
    let rejected = tree.witness("b");
    assert(rejected.is_none());
    let rows = vec![predecessor("a", None), predecessor("b", Some(edge("a", policy::EdgeKind::Compile, "z")))];
    let tree = policy::PredecessorTree::new(rows).unwrap();
    proof {
        policy::predecessor_unique(tree.view(), "b".spec_bytes(), 1);
        reveal_with_fuel(policy::predecessor_chain, 3);
    }
    let rejected = tree.witness("b");
    assert(rejected.is_none());
    let missing = tree.witness("z");
    assert(missing.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn sparse_predecessors_keep_exact_reached_nodes_and_typed_search_slots() {
    proof { predecessor_ascii_order(); }
    let first = edge("", policy::EdgeKind::Compile, "a");
    let parallel = edge("", policy::EdgeKind::TestExecution, "a");
    let names = vec!["", "a", "b"];
    let edges = vec![vec![&first, &parallel], vec![], vec![]];
    let search = policy::Search { distances: vec![0, 1, 3], parents: vec![None, Some((0, 1)), None], order: vec![0, 1] };
    let tree = policy::predecessors_from_search(&names, &edges, &search);
    assert(policy::predecessor_keys(tree.view()).to_set() =~=
        set![("".spec_bytes(), None::<policy::EdgeKey>), ("a".spec_bytes(), Some(parallel.key()))]);
    assert(policy::ordered_predecessors(tree.view()));
    let entries = tree.entries();
    assert forall|i: int| 0 <= i < entries.len() implies (#[trigger] entries@[i]).name() != "b".spec_bytes() by {
        assert(policy::predecessor_keys(entries@).to_set().contains(policy::predecessor_keys(entries@)[i]));
    }
}

}
