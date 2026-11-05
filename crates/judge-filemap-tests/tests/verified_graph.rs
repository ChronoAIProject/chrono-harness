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

verus! {

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
    // Exact tie-breaking is checked through the actual typed adapter's tests;
    // the proof contract here requires a real shortest predecessor edge.
}

}
