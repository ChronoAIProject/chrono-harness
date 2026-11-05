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

}
