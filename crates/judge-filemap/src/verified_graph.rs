//! Indexed FIFO graph traversal. The caller owns the name/edge-index adapter.
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub struct Search {
    /// The graph size denotes an unreachable node; all other values are hop counts.
    pub distances: Vec<usize>,
    /// Source node and adjacency slot, preserving parallel edge identity.
    pub parents: Vec<Option<(usize, usize)>>,
    pub order: Vec<usize>,
}

/// First discovery owns a node permanently. Slots are examined in their supplied
/// order; previously seen targets leave all three state sequences unchanged.
pub open spec fn discovery(
    g: Seq<Vec<usize>>, node: usize, slot: int,
    d: Seq<usize>, p: Seq<Option<(usize, usize)>>, q: Seq<usize>,
) -> (Seq<usize>, Seq<Option<(usize, usize)>>, Seq<usize>) {
    if node < g.len() && 0 <= slot < g[node as int].len() && node < d.len() {
        let target = g[node as int]@[slot];
        if target < d.len() && target < p.len() && d[target as int] == g.len()
            && d[node as int] < usize::MAX
        {
            (d.update(target as int, (d[node as int] + 1) as usize),
                p.update(target as int, Some((node, slot as usize))), q.push(target))
        } else { (d, p, q) }
    } else { (d, p, q) }
}

#[verifier::opaque]
pub open spec fn ordered_slots(
    g: Seq<Vec<usize>>, node: usize, slot: int,
    d: Seq<usize>, p: Seq<Option<(usize, usize)>>, q: Seq<usize>,
) -> (Seq<usize>, Seq<Option<(usize, usize)>>, Seq<usize>)
    decreases if node < g.len() { g[node as int].len() - slot } else { 0 },
{
    if node < g.len() && 0 <= slot < g[node as int].len() {
        let next = discovery(g, node, slot, d, p, q);
        ordered_slots(g, node, slot + 1, next.0, next.1, next.2)
    } else { (d, p, q) }
}

#[verifier::opaque]
pub open spec fn fifo_nodes(
    g: Seq<Vec<usize>>, front: int,
    d: Seq<usize>, p: Seq<Option<(usize, usize)>>, q: Seq<usize>,
) -> (Seq<usize>, Seq<Option<(usize, usize)>>, Seq<usize>)
    decreases g.len() - front,
{
    if 0 <= front < g.len() && front < q.len() {
        let next = ordered_slots(g, q[front], 0, d, p, q);
        fifo_nodes(g, front + 1, next.0, next.1, next.2)
    } else { (d, p, q) }
}

/// Only the seed is discovered initially; every other distance is the sentinel.
pub open spec fn initial_state(n: nat, root: usize)
    -> (Seq<usize>, Seq<Option<(usize, usize)>>, Seq<usize>)
{
    (Seq::new(n, |i: int| if i == root { 0usize } else { n as usize }),
        Seq::new(n, |i: int| None), seq![root])
}

/// A deterministic reference contract: FIFO nodes, ordered adjacency slots and
/// immutable first predecessors. The bound is the number of distinct nodes;
/// the executable traversal separately proves it never leaves queued work out.
pub open spec fn fifo(g: Seq<Vec<usize>>, root: usize)
    -> (Seq<usize>, Seq<Option<(usize, usize)>>, Seq<usize>)
{
    if root < g.len() && g.len() <= usize::MAX {
        let start = initial_state(g.len(), root);
        fifo_nodes(g, 0, start.0, start.1, start.2)
    } else { (Seq::empty(), Seq::empty(), Seq::empty()) }
}

proof fn ordered_slot_step(
    g: Seq<Vec<usize>>, node: usize, slot: int,
    d: Seq<usize>, p: Seq<Option<(usize, usize)>>, q: Seq<usize>,
)
    requires node < g.len(), 0 <= slot < g[node as int].len(),
    ensures ordered_slots(g, node, slot, d, p, q) == {
        let next = discovery(g, node, slot, d, p, q);
        ordered_slots(g, node, slot + 1, next.0, next.1, next.2)
    },
{
    reveal(ordered_slots);
}

pub open spec fn valid_graph(g: Seq<Vec<usize>>) -> bool {
    forall|u: int, k: int| 0 <= u < g.len() && 0 <= k < g[u].len()
        ==> (#[trigger] g[u]@[k]) < g.len()
}

pub open spec fn walk(g: Seq<Vec<usize>>, w: Seq<usize>) -> bool {
    w.len() > 0
        && (forall|i: int| 0 <= i < w.len() ==> (#[trigger] w[i]) < g.len())
        && (forall|i: int| 0 <= i < w.len() - 1
            ==> (#[trigger] g[w[i] as int])@.contains(w[i + 1]))
}

pub open spec fn tree(
    g: Seq<Vec<usize>>, root: usize, d: Seq<usize>, p: Seq<Option<(usize, usize)>>,
) -> bool {
    root < g.len() && d.len() == g.len() && p.len() == g.len()
        && d[root as int] == 0 && p[root as int].is_none()
        && (forall|v: int| 0 <= v < g.len() ==> (#[trigger] d[v]) <= g.len())
        && forall|v: int| #![trigger p[v]] 0 <= v < g.len() ==> (
            if d[v] == g.len() { p[v].is_none() }
                else if v != root {
                    p[v].is_some() && {
                        let (u, k) = p[v].unwrap();
                        u < g.len() && k < g[u as int].len() && g[u as int]@[k as int] == v
                            && d[u as int] < d[v] && d[v] == d[u as int] + 1
                    }
                } else { true }
        )
}

pub open spec fn queue_state(
    g: Seq<Vec<usize>>, root: usize, d: Seq<usize>, p: Seq<Option<(usize, usize)>>,
    q: Seq<usize>,
) -> bool {
    tree(g, root, d, p) && q.len() > 0 && q[0] == root && q.no_duplicates()
        && (forall|i: int| 0 <= i < q.len() ==> (
            (#[trigger] q[i]) < g.len() && d[q[i] as int] < g.len() && d[q[i] as int] <= i
        ))
        && (forall|v: int| 0 <= v < g.len()
            ==> ((#[trigger] d[v]) < g.len() <==> q.contains(v as usize)))
        && (forall|i: int, j: int| 0 <= i <= j < q.len()
            ==> d[#[trigger] q[i] as int] <= d[#[trigger] q[j] as int])
}

pub open spec fn processed(
    g: Seq<Vec<usize>>, d: Seq<usize>, q: Seq<usize>, front: int, slot: int,
) -> bool {
    forall|i: int, k: int| 0 <= i < q.len() && 0 <= k < g[q[i] as int].len()
        && (i < front || (i == front && k < slot)) ==> {
            let v = (#[trigger] g[q[i] as int]@[k]);
            d[v as int] < g.len() && d[v as int] <= d[q[i] as int] + 1
        }
}

pub proof fn bounded_queue(q: Seq<usize>, n: usize)
    requires q.no_duplicates(), forall|i: int| 0 <= i < q.len() ==> (#[trigger] q[i]) < n,
    ensures q.len() <= n,
{
    use vstd::set_lib::FiniteRange;
    q.unique_seq_to_set();
    vstd::set_lib::range_set_properties::<usize>(0, n);
    assert(q.to_set().subset_of(usize::range_set(0, n))) by {
        assert forall|v: usize| q.to_set().contains(v) implies usize::range_set(0, n).contains(v) by {
            let i = choose|i: int| 0 <= i < q.len() && q[i] == v;
        }
    }
    vstd::set_lib::lemma_len_subset(q.to_set(), usize::range_set(0, n));
}

/// A finite predecessor chain supplies an actual path of the recorded length.
pub proof fn tree_path(
    g: Seq<Vec<usize>>, root: usize, d: Seq<usize>, p: Seq<Option<(usize, usize)>>, v: usize,
) -> (w: Seq<usize>)
    requires tree(g, root, d, p), v < g.len(), d[v as int] < g.len(),
    ensures walk(g, w), w[0] == root, w.last() == v, w.len() == d[v as int] + 1,
    decreases d[v as int],
{
    if v == root {
        seq![root]
    } else {
        let (u, k) = p[v as int].unwrap();
        let prefix = tree_path(g, root, d, p, u);
        let w = prefix.push(v);
        assert forall|i: int| 0 <= i < w.len() - 1
            implies (#[trigger] g[w[i] as int])@.contains(w[i + 1]) by {
            if i == prefix.len() - 1 {
                assert(g[u as int]@[k as int] == v);
            }
        }
        w
    }
}

pub open spec fn closed(g: Seq<Vec<usize>>, d: Seq<usize>) -> bool {
    forall|u: int, k: int| 0 <= u < g.len() && d[u] < g.len() && 0 <= k < g[u].len()
        ==> {
            let v = (#[trigger] g[u]@[k]);
            d[v as int] < g.len() && d[v as int] <= d[u] + 1
        }
}

/// Edge closure gives reachability completeness and a lower bound for every walk.
pub proof fn walk_bound(g: Seq<Vec<usize>>, d: Seq<usize>, root: usize, w: Seq<usize>)
    requires valid_graph(g), d.len() == g.len(), root < g.len(), d[root as int] == 0,
        closed(g, d), walk(g, w), w[0] == root,
    ensures d[w.last() as int] < g.len(), d[w.last() as int] <= w.len() - 1,
    decreases w.len(),
{
    if w.len() > 1 {
        let prefix = w.drop_last();
        assert(walk(g, prefix));
        walk_bound(g, d, root, prefix);
        let u = prefix.last();
        let v = w.last();
        let k = choose|k: int| 0 <= k < g[u as int].len() && g[u as int]@[k] == v;
        assert(d[v as int] <= d[u as int] + 1);
    }
}

#[verifier::opaque]
pub open spec fn complete(g: Seq<Vec<usize>>, root: usize, s: Search) -> bool {
    queue_state(g, root, s.distances@, s.parents@, s.order@)
        && closed(g, s.distances@)
        && (forall|v: usize| v < g.len() ==> (
            s.distances@[v as int] < g.len() <==>
                exists|w: Seq<usize>| walk(g, w) && w[0] == root && w.last() == v
        ))
        && (forall|w: Seq<usize>| walk(g, w) && w[0] == root
            ==> s.distances@[w.last() as int] <= w.len() - 1)
}

/// Validate once before traversing several roots of the same immutable graph.
pub fn valid_indices(g: &[Vec<usize>]) -> (valid: bool)
    ensures valid == valid_graph(g@),
{
    let n = g.len();
    let mut u = 0;
    while u < n
        invariant n == g.len(), u <= n,
            forall|i: int, k: int| 0 <= i < u && 0 <= k < g[i].len()
                ==> (#[trigger] g[i]@[k]) < n,
        decreases n - u,
    {
        let mut k = 0;
        while k < g[u].len()
            invariant n == g.len(), u < n, k <= g[u as int].len(),
                forall|i: int, j: int| 0 <= i < u && 0 <= j < g[i].len()
                    ==> (#[trigger] g[i]@[j]) < n,
                forall|j: int| 0 <= j < k ==> (#[trigger] g[u as int]@[j]) < n,
            decreases g[u as int].len() - k,
        {
            if g[u][k] >= n { return false; }
            k += 1;
        }
        u += 1;
    }
    true
}

/// One adjacency transition, separated from loop bookkeeping for local proofs.
fn expand_edge(
    g: &[Vec<usize>], _root: usize, d: &mut Vec<usize>,
    p: &mut Vec<Option<(usize, usize)>>, q: &mut Vec<usize>, front: usize, slot: usize,
)
    requires
        valid_graph(g@), queue_state(g@, _root, old(d)@, old(p)@, old(q)@),
        front < old(q).len(), slot < g[old(q)@[front as int] as int].len(),
        forall|i: int| 0 <= i < old(q).len()
            ==> old(d)@[#[trigger] old(q)@[i] as int] <= old(d)@[old(q)@[front as int] as int] + 1,
        processed(g@, old(d)@, old(q)@, front as int, slot as int),
    ensures
        queue_state(g@, _root, d@, p@, q@), q.len() <= g.len(), front < q.len(),
        (d@, p@, q@) == discovery(g@, old(q)@[front as int], slot as int,
            old(d)@, old(p)@, old(q)@),
        q@[front as int] == old(q)@[front as int],
        forall|i: int| 0 <= i < q.len()
            ==> d@[#[trigger] q@[i] as int] <= d@[q@[front as int] as int] + 1,
        processed(g@, d@, q@, front as int, slot as int + 1),
{
    let n = g.len();
    let node = q[front];
    let v = g[node][slot];
    if d[v] == n {
        let ghost old_d = d@;
        let ghost old_p = p@;
        let ghost old_q = q@;
        proof {
            assert(!q@.contains(v));
            bounded_queue(q@.push(v), n);
        }
        let depth = d[node] + 1;
        d[v] = depth;
        p[v] = Some((node, slot));
        q.push(v);
        proof {
            assert(q@.no_duplicates());
            assert(tree(g@, _root, d@, p@));
            assert forall|i: int| 0 <= i < q.len() implies (
                (#[trigger] q[i]) < n && d[q[i] as int] < n && d[q[i] as int] <= i
            ) by {
                if i < old_q.len() { assert(old_q[i] != v); }
            }
            assert forall|x: int| 0 <= x < n implies
                ((#[trigger] d[x]) < n <==> q@.contains(x as usize)) by {
                vstd::seq_lib::lemma_seq_contains_after_push(old_q, v, x as usize);
                if x != v { assert(d[x] == old_d[x]); }
            }
            assert forall|i: int, j: int| 0 <= i <= j < q.len()
                implies d[#[trigger] q[i] as int] <= d[#[trigger] q[j] as int] by {
                if j < old_q.len() { assert(old_q[i] != v && old_q[j] != v); }
                else if i < old_q.len() { assert(old_q[i] != v); }
            }
            assert(processed(g@, d@, q@, front as int, slot as int));
        }
    }
    proof {
        bounded_queue(q@, n);
        assert forall|i: int, k: int| 0 <= i < q.len() && 0 <= k < g[q[i] as int].len()
            && (i < front || (i == front && k < slot + 1)) implies {
                let target = (#[trigger] g[q[i] as int]@[k]);
                d[target as int] < n && d[target as int] <= d[q[i] as int] + 1
            } by {
            if i == front && k == slot { assert(g[q[i] as int]@[k] == v); }
        }
    }
}

/// FIFO traversal of explicitly validated immutable adjacency. The caller must
/// establish valid_indices once; an out-of-range root is rejected here.
pub fn breadth_first(g: &[Vec<usize>], root: usize) -> (result: Option<Search>)
    requires valid_graph(g@),
    ensures
        result.is_some() == (root < g.len()),
        result.is_some() ==> complete(g@, root, result.unwrap()),
        result.is_some() ==> {
            let search = result.unwrap();
            (search.distances@, search.parents@, search.order@) == fifo(g@, root)
        },
{
    reveal(complete);
    let n = g.len();
    if root >= n { return None; }
    let mut d: Vec<usize> = Vec::new();
    let mut p: Vec<Option<(usize, usize)>> = Vec::new();
    while d.len() < n
        invariant n == g.len(), root < n, valid_graph(g@),
            d.len() <= n, p.len() == d.len(),
            forall|i: int| 0 <= i < d.len() ==> (#[trigger] d[i]) == n && p[i].is_none(),
        decreases n - d.len(),
    {
        d.push(n);
        p.push(None);
    }
    proof { assert(p[root as int].is_none()) by { assert(d[root as int] == n); } }
    d[root] = 0;
    let mut q = Vec::new();
    q.push(root);
    proof {
        assert(q@ == seq![root]);
        assert(d.len() == g.len() && p.len() == g.len());
        assert(d[root as int] == 0 && p[root as int].is_none());
        assert forall|x: int| 0 <= x < n implies
            d[x] == (if x == root { 0usize } else { n }) && p[x].is_none() by {}
        assert forall|x: int| 0 <= x < n implies
            ((#[trigger] d[x]) < n <==> q@.contains(x as usize)) by {
            if x == root { assert(q@[0] == x); }
        }
        assert(tree(g@, root, d@, p@));
        assert(q@.no_duplicates());
        assert forall|i: int| 0 <= i < q.len() implies (
            (#[trigger] q[i]) < n && d[q[i] as int] < n && d[q[i] as int] <= i
        ) by {}
        assert forall|i: int, j: int| 0 <= i <= j < q.len()
            implies d[#[trigger] q[i] as int] <= d[#[trigger] q[j] as int] by {}
        assert(queue_state(g@, root, d@, p@, q@));
        assert(g@.len() == n && g@.len() <= usize::MAX);
        let start = initial_state(g@.len(), root);
        assert(d@ =~= start.0);
        assert(p@ =~= start.1);
        assert(q@ == start.2);
        reveal(fifo);
        assert(fifo_nodes(g@, 0, d@, p@, q@) == fifo(g@, root));
    }
    let mut front = 0;
    while front < q.len()
        invariant
            root < n, n == g.len(), valid_graph(g@),
            queue_state(g@, root, d@, p@, q@), front <= q.len(), q.len() <= n,
            front < q.len() ==> (forall|i: int| 0 <= i < q.len()
                ==> d[#[trigger] q[i] as int] <= d[q[front as int] as int] + 1),
            processed(g@, d@, q@, front as int, 0),
            fifo_nodes(g@, front as int, d@, p@, q@) == fifo(g@, root),
        decreases n - front,
    {
        let node = q[front];
        let ghost row_result = ordered_slots(g@, node, 0, d@, p@, q@);
        proof {
            reveal(fifo_nodes);
            assert(fifo_nodes(g@, front as int + 1, row_result.0, row_result.1, row_result.2)
                == fifo(g@, root));
        }
        let mut slot = 0;
        while slot < g[node].len()
            invariant
                root < n, n == g.len(), valid_graph(g@),
                queue_state(g@, root, d@, p@, q@), front < q.len(), q.len() <= n,
                node == q[front as int], slot <= g[node as int].len(),
                forall|i: int| 0 <= i < q.len()
                    ==> d[#[trigger] q[i] as int] <= d[node as int] + 1,
                processed(g@, d@, q@, front as int, slot as int),
                ordered_slots(g@, node, slot as int, d@, p@, q@) == row_result,
                fifo_nodes(g@, front as int + 1, row_result.0, row_result.1, row_result.2)
                    == fifo(g@, root),
            decreases g[node as int].len() - slot,
        {
            proof { ordered_slot_step(g@, node, slot as int, d@, p@, q@); }
            expand_edge(g, root, &mut d, &mut p, &mut q, front, slot);
            slot += 1;
        }
        proof {
            reveal(ordered_slots);
            assert((d@, p@, q@) == row_result);
        }
        front += 1;
    }
    proof {
        reveal(fifo_nodes);
        assert((d@, p@, q@) == fifo(g@, root));
        assert(closed(g@, d@)) by {
            assert forall|u: int, k: int| 0 <= u < n && d[u] < n && 0 <= k < g[u].len()
                implies {
                    let v = (#[trigger] g[u]@[k]);
                    d[v as int] < n && d[v as int] <= d[u] + 1
                } by {
                let i = choose|i: int| 0 <= i < q.len() && q[i] == u;
            }
        }
        assert forall|v: usize| v < n implies (
            d[v as int] < n <==> exists|w: Seq<usize>| walk(g@, w) && w[0] == root && w.last() == v
        ) by {
            if d[v as int] < n {
                let w = tree_path(g@, root, d@, p@, v);
            } else if exists|w: Seq<usize>| walk(g@, w) && w[0] == root && w.last() == v {
                let w = choose|w: Seq<usize>| walk(g@, w) && w[0] == root && w.last() == v;
                walk_bound(g@, d@, root, w);
            }
        }
        assert forall|w: Seq<usize>| walk(g@, w) && w[0] == root
            implies d[w.last() as int] <= w.len() - 1 by {
            walk_bound(g@, d@, root, w);
        }
    }
    Some(Search { distances: d, parents: p, order: q })
}

}
