//! Independent boundaries for the executable graph search and its contract.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../judge-filemap/src/verified_graph.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_judge_filemap::verified_graph as policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_graph_and_invalid_roots() {
    let empty: Vec<Vec<usize>> = Vec::new();
    assert(policy::valid_indices(&empty));
    let result = policy::breadth_first(&empty, 0);
    assert(result.is_none());
    let g: Vec<Vec<usize>> = vec![vec![]];
    assert(policy::valid_indices(&g));
    let result = policy::breadth_first(&g, 1);
    assert(result.is_none());
    let result = policy::breadth_first(&g, usize::MAX);
    assert(result.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn malformed_destinations_fail_before_traversal() {
    let g: Vec<Vec<usize>> = vec![vec![1]];
    assert(!policy::valid_indices(&g));
    let g: Vec<Vec<usize>> = vec![vec![], vec![usize::MAX]];
    assert(!policy::valid_indices(&g));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn self_cycle_and_disconnected_node() {
    let g: Vec<Vec<usize>> = vec![vec![0], vec![]];
    assert(policy::valid_indices(&g));
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
    assert(policy::valid_indices(&g));
    let result = policy::breadth_first(&g, 0);
    assert(result.is_some());
    let search = result.unwrap();
    assert(search.distances[0] == 0);
    assert(search.distances[1] == 1);
    assert(search.distances[2] == 1);
    assert(search.distances[3] == 2);
    assert(search.order.len() == 4);
    // Exact tie-breaking is checked through the actual typed adapter's tests;
    // the proof contract here requires a real shortest predecessor edge.
}

}
