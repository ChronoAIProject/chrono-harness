//! Finite union/closure mechanics. Callers own endpoint validation, seeds and extra selections.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeKind {
    Compile,
    BuildInput,
    RuntimeInput,
    TestExecution,
    JudgeTrigger,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub kind: EdgeKind,
    pub to: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Base,
    Candidate,
    Both,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnionEdge {
    #[serde(flatten)]
    pub edge: Edge,
    pub origin: Origin,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Seed {
    pub id: String,
    pub node: String,
    pub reference: String,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraversedEdge {
    pub edge: Edge,
    pub seeds: BTreeSet<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Closure {
    pub reached: BTreeMap<String, BTreeSet<String>>,
    /// One deterministic shortest predecessor per (seed, node); null denotes the seed root.
    pub predecessors: BTreeMap<String, BTreeMap<String, Option<Edge>>>,
    /// All traversed edges and contributing seeds, including alternate paths and cycle edges.
    pub traversed: Vec<TraversedEdge>,
}
pub fn union(base: &BTreeSet<Edge>, candidate: &BTreeSet<Edge>) -> Vec<UnionEdge> {
    base.union(candidate)
        .map(|edge| UnionEdge {
            edge: edge.clone(),
            origin: match (base.contains(edge), candidate.contains(edge)) {
                (true, true) => Origin::Both,
                (true, false) => Origin::Base,
                _ => Origin::Candidate,
            },
        })
        .collect()
}
pub fn closure(edges: &[UnionEdge], seeds: &[Seed]) -> Closure {
    let mut out = Closure {
        reached: BTreeMap::new(),
        predecessors: BTreeMap::new(),
        traversed: vec![],
    };
    let mut adjacency: BTreeMap<&str, BTreeSet<&Edge>> = BTreeMap::new();
    for e in edges {
        adjacency.entry(&e.edge.from).or_default().insert(&e.edge);
    }
    let mut traversed: BTreeMap<Edge, BTreeSet<String>> = BTreeMap::new();
    for seed in seeds {
        let mut parents = BTreeMap::from([(seed.node.clone(), None)]);
        let mut queue = VecDeque::from([seed.node.clone()]);
        while let Some(node) = queue.pop_front() {
            out.reached
                .entry(node.clone())
                .or_default()
                .insert(seed.id.clone());
            for edge in adjacency.get(node.as_str()).into_iter().flatten() {
                traversed
                    .entry((*edge).clone())
                    .or_default()
                    .insert(seed.id.clone());
                if !parents.contains_key(&edge.to) {
                    parents.insert(edge.to.clone(), Some((*edge).clone()));
                    queue.push_back(edge.to.clone());
                }
            }
        }
        out.predecessors.insert(seed.id.clone(), parents);
    }
    out.traversed = traversed
        .into_iter()
        .map(|(edge, seeds)| TraversedEdge { edge, seeds })
        .collect();
    out
}
impl Closure {
    pub fn selected_tests(&self) -> BTreeSet<String> {
        self.traversed
            .iter()
            .filter(|e| e.edge.kind == EdgeKind::TestExecution)
            .map(|e| e.edge.to.clone())
            .collect()
    }
    pub fn witness(&self, seed: &str, node: &str) -> Option<Vec<Edge>> {
        let parents = self.predecessors.get(seed)?;
        let mut node = node;
        let mut path = vec![];
        while let Some(edge) = parents.get(node)? {
            path.push(edge.clone());
            node = &edge.from;
        }
        path.reverse();
        Some(path)
    }
}

/// A deterministic, declaration-only structural view of the registered graph.
///
/// This is deliberately an observation rather than an optimiser: it reports the
/// facts which an autonomous caller needs in order to decide whether a real
/// refactoring is worthwhile.  No edge is inferred from source text, names or
/// language metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureAnalysis {
    pub schema: String,
    pub base: GraphSnapshot,
    pub candidate: GraphSnapshot,
    pub union: GraphSnapshot,
    pub affected_components: Vec<String>,
    pub execution: ExecutionAnalysis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub node_count: usize,
    pub edge_count: usize,
    pub direct_relations: BTreeMap<String, usize>,
    pub cross_owner_edges: Vec<CrossOwnerEdge>,
    pub components: Vec<StrongComponent>,
    pub component_depth: BTreeMap<String, usize>,
    pub max_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossOwnerEdge {
    pub edge: Edge,
    pub from_owners: Vec<String>,
    pub to_owners: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrongComponent {
    pub id: String,
    pub nodes: Vec<String>,
    pub cyclic: bool,
    pub witness: Vec<Edge>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionAnalysis {
    pub base: ExecutionSnapshot,
    pub candidate: ExecutionSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionSnapshot {
    pub operations: Vec<String>,
    pub edges: Vec<ExecutionEdge>,
    pub cycles: Vec<Vec<String>>,
    pub depth: BTreeMap<String, usize>,
    pub resource_conflicts: Vec<ExecutionConflict>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionEdge {
    pub from: String,
    pub to: String,
    pub tests: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionConflict {
    pub left: String,
    pub right: String,
    pub resources: Vec<String>,
    pub outputs: Vec<String>,
}

/// Analyse an explicit endpoint union.  `all_nodes` and `owners` are supplied
/// by registration's interpreted inventory; this function never discovers
/// either from the checkout.
pub struct AnalysisInput<'a> {
    pub base: &'a BTreeSet<Edge>,
    pub candidate: &'a BTreeSet<Edge>,
    pub union: &'a [UnionEdge],
    pub base_nodes: &'a BTreeSet<String>,
    pub candidate_nodes: &'a BTreeSet<String>,
    pub union_nodes: &'a BTreeSet<String>,
    pub owners: &'a BTreeMap<String, BTreeSet<String>>,
    pub base_filemap: &'a Value,
    pub candidate_filemap: &'a Value,
    pub reached: &'a BTreeMap<String, BTreeSet<String>>,
}

pub fn analyze(input: AnalysisInput<'_>) -> StructureAnalysis {
    let base_snapshot = graph_snapshot(input.base, input.base_nodes, input.owners);
    let candidate_snapshot = graph_snapshot(input.candidate, input.candidate_nodes, input.owners);
    let union_edges: BTreeSet<_> = input.union.iter().map(|e| e.edge.clone()).collect();
    let union_snapshot = graph_snapshot(&union_edges, input.union_nodes, input.owners);
    let mut affected = BTreeSet::new();
    for component in &union_snapshot.components {
        if component
            .nodes
            .iter()
            .any(|node| input.reached.contains_key(node))
        {
            affected.insert(component.id.clone());
        }
    }
    let execution = ExecutionAnalysis {
        base: execution_snapshot(input.base_filemap),
        candidate: execution_snapshot(input.candidate_filemap),
    };
    StructureAnalysis {
        schema: "chrono-filemap-structure/v1".into(),
        base: base_snapshot,
        candidate: candidate_snapshot,
        union: union_snapshot,
        affected_components: affected.into_iter().collect(),
        execution,
    }
}

fn graph_snapshot(
    edges: &BTreeSet<Edge>,
    all_nodes: &BTreeSet<String>,
    owners: &BTreeMap<String, BTreeSet<String>>,
) -> GraphSnapshot {
    let mut nodes = all_nodes.clone();
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut reverse: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut direct_relations = BTreeMap::new();
    for edge in edges {
        nodes.insert(edge.from.clone());
        nodes.insert(edge.to.clone());
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .insert(edge.to.clone());
        reverse
            .entry(edge.to.clone())
            .or_default()
            .insert(edge.from.clone());
        *direct_relations.entry(edge.from.clone()).or_insert(0) += 1;
    }
    for node in &nodes {
        direct_relations.entry(node.clone()).or_insert(0);
    }
    let components = strongly_connected(edges, &nodes, &adjacency, &reverse);
    let component_of: BTreeMap<_, _> = components
        .iter()
        .flat_map(|component| {
            component
                .nodes
                .iter()
                .map(move |node| (node.clone(), component.id.clone()))
        })
        .collect();
    let mut predecessors: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for component in &components {
        predecessors.entry(component.id.clone()).or_default();
    }
    let mut cross_owner_edges = vec![];
    for edge in edges {
        let from = component_of.get(&edge.from).unwrap();
        let to = component_of.get(&edge.to).unwrap();
        if from != to {
            predecessors
                .entry(to.clone())
                .or_default()
                .insert(from.clone());
        }
        let from_owners: Vec<_> = owners
            .get(&edge.from)
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        let to_owners: Vec<_> = owners
            .get(&edge.to)
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        if !from_owners.is_empty()
            && !to_owners.is_empty()
            && from_owners.iter().all(|owner| !to_owners.contains(owner))
        {
            cross_owner_edges.push(CrossOwnerEdge {
                edge: edge.clone(),
                from_owners,
                to_owners,
            });
        }
    }
    let mut component_depth = BTreeMap::new();
    for component in &components {
        let depth = longest_component_depth(&component.id, &predecessors, &mut component_depth);
        component_depth.insert(component.id.clone(), depth);
    }
    let max_depth = component_depth.values().copied().max().unwrap_or(0);
    GraphSnapshot {
        node_count: nodes.len(),
        edge_count: edges.len(),
        direct_relations,
        cross_owner_edges,
        components,
        component_depth,
        max_depth,
    }
}

fn longest_component_depth(
    component: &str,
    predecessors: &BTreeMap<String, BTreeSet<String>>,
    memo: &mut BTreeMap<String, usize>,
) -> usize {
    if let Some(depth) = memo.get(component) {
        return *depth;
    }
    let depth = predecessors
        .get(component)
        .into_iter()
        .flatten()
        .map(|parent| longest_component_depth(parent, predecessors, memo) + 1)
        .max()
        .unwrap_or(0);
    memo.insert(component.into(), depth);
    depth
}

fn strongly_connected(
    edges: &BTreeSet<Edge>,
    nodes: &BTreeSet<String>,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
    reverse: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<StrongComponent> {
    let mut visited = BTreeSet::new();
    let mut order = vec![];
    for start in nodes {
        if visited.contains(start) {
            continue;
        }
        let mut stack = vec![(start.clone(), false)];
        while let Some((node, expanded)) = stack.pop() {
            if expanded {
                order.push(node);
                continue;
            }
            if !visited.insert(node.clone()) {
                continue;
            }
            stack.push((node.clone(), true));
            for next in adjacency.get(&node).into_iter().flatten().rev() {
                if !visited.contains(next) {
                    stack.push((next.clone(), false));
                }
            }
        }
    }
    let mut assigned = BTreeSet::new();
    let mut groups = vec![];
    for start in order.into_iter().rev() {
        if !assigned.insert(start.clone()) {
            continue;
        }
        let mut group = vec![start.clone()];
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            for next in reverse.get(&node).into_iter().flatten().rev() {
                if assigned.insert(next.clone()) {
                    group.push(next.clone());
                    stack.push(next.clone());
                }
            }
        }
        group.sort();
        groups.push(group);
    }
    groups.sort();
    groups
        .into_iter()
        .map(|nodes| {
            let id = format!("scc:{}", nodes[0]);
            let cyclic = nodes.len() > 1
                || edges
                    .iter()
                    .any(|edge| edge.from == nodes[0] && edge.to == nodes[0]);
            let witness = if cyclic {
                cycle_witness(edges, &nodes)
            } else {
                vec![]
            };
            StrongComponent {
                id,
                nodes,
                cyclic,
                witness,
            }
        })
        .collect()
}

fn cycle_witness(edges: &BTreeSet<Edge>, nodes: &[String]) -> Vec<Edge> {
    let allowed: BTreeSet<_> = nodes.iter().cloned().collect();
    for first in edges
        .iter()
        .filter(|edge| allowed.contains(&edge.from) && allowed.contains(&edge.to))
    {
        if first.from == first.to {
            return vec![first.clone()];
        }
        let mut predecessors: BTreeMap<String, Edge> = BTreeMap::new();
        let mut queue = VecDeque::from([first.to.clone()]);
        let mut seen = BTreeSet::from([first.to.clone()]);
        while let Some(node) = queue.pop_front() {
            for edge in edges
                .iter()
                .filter(|edge| edge.from == node && allowed.contains(&edge.to))
            {
                if seen.insert(edge.to.clone()) {
                    predecessors.insert(edge.to.clone(), edge.clone());
                    queue.push_back(edge.to.clone());
                }
            }
        }
        if seen.contains(&first.from) {
            let mut path = vec![];
            let mut at = first.from.clone();
            while at != first.to {
                let edge = predecessors.get(&at).cloned().unwrap();
                at = edge.from.clone();
                path.push(edge);
            }
            path.reverse();
            let mut result = vec![first.clone()];
            result.extend(path);
            return result;
        }
    }
    vec![]
}

fn execution_snapshot(filemap: &Value) -> ExecutionSnapshot {
    let mut operations = BTreeSet::new();
    let mut edge_tests: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    if let Some(plans) = filemap.get("execution_plans").and_then(Value::as_object) {
        for (test, plan) in plans {
            let ids: Vec<_> = plan["operations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            operations.extend(ids.iter().cloned());
            for pair in ids.windows(2) {
                edge_tests
                    .entry((pair[0].clone(), pair[1].clone()))
                    .or_default()
                    .insert(test.clone());
            }
        }
    }
    let edges: Vec<_> = edge_tests
        .into_iter()
        .map(|((from, to), tests)| ExecutionEdge {
            from,
            to,
            tests: tests.into_iter().collect(),
        })
        .collect();
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edge in &edges {
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .insert(edge.to.clone());
    }
    let cycles = string_cycles(&operations, &adjacency);
    let mut predecessors: BTreeMap<String, BTreeSet<String>> = operations
        .iter()
        .map(|id| (id.clone(), BTreeSet::new()))
        .collect();
    for edge in &edges {
        predecessors
            .entry(edge.to.clone())
            .or_default()
            .insert(edge.from.clone());
    }
    let mut depth = BTreeMap::new();
    // A cycle is itself the useful diagnostic.  Do not recurse through it while
    // manufacturing a depth value; routes will reject the same malformed plan
    // before any operation is launched.
    if cycles.is_empty() {
        for operation in &operations {
            let value = longest_component_depth(operation, &predecessors, &mut depth);
            depth.insert(operation.clone(), value);
        }
    }
    let resource_conflicts = filemap
        .get("execution_scheduling")
        .and_then(|v| v.get("claims"))
        .and_then(Value::as_object)
        .map(|claims| {
            let ids: Vec<_> = claims.keys().cloned().collect();
            let mut conflicts = vec![];
            for (index, left) in ids.iter().enumerate() {
                for right in ids.iter().skip(index + 1) {
                    let left_value = &claims[left];
                    let right_value = &claims[right];
                    let resources: Vec<_> = left_value["resources"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .filter(|resource| {
                            right_value["resources"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .any(|other| other.as_str() == Some(resource))
                        })
                        .map(str::to_string)
                        .collect();
                    let outputs: Vec<_> =
                        left_value["outputs"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                            .filter(|output| {
                                right_value["outputs"].as_array().into_iter().flatten().any(
                                    |other| {
                                        other.as_str().is_some_and(|other| {
                                            chrono_harness::units::overlap(output, other)
                                        })
                                    },
                                )
                            })
                            .map(str::to_string)
                            .collect();
                    if !resources.is_empty() || !outputs.is_empty() {
                        conflicts.push(ExecutionConflict {
                            left: left.clone(),
                            right: right.clone(),
                            resources,
                            outputs,
                        });
                    }
                }
            }
            conflicts
        })
        .unwrap_or_default();
    ExecutionSnapshot {
        operations: operations.into_iter().collect(),
        edges,
        cycles,
        depth,
        resource_conflicts,
    }
}

fn string_cycles(
    nodes: &BTreeSet<String>,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<Vec<String>> {
    let mut result = vec![];
    let mut visiting = BTreeSet::new();
    let mut finished = BTreeSet::new();
    fn visit(
        node: &str,
        adjacency: &BTreeMap<String, BTreeSet<String>>,
        visiting: &mut BTreeSet<String>,
        finished: &mut BTreeSet<String>,
        stack: &mut Vec<String>,
        result: &mut Vec<Vec<String>>,
    ) {
        if finished.contains(node) {
            return;
        }
        if visiting.contains(node) {
            if let Some(index) = stack.iter().position(|value| value == node) {
                let mut cycle = stack[index..].to_vec();
                cycle.push(node.into());
                result.push(cycle);
            }
            return;
        }
        visiting.insert(node.into());
        stack.push(node.into());
        for next in adjacency.get(node).into_iter().flatten() {
            visit(next, adjacency, visiting, finished, stack, result);
        }
        stack.pop();
        visiting.remove(node);
        finished.insert(node.into());
    }
    for node in nodes {
        visit(
            node,
            adjacency,
            &mut visiting,
            &mut finished,
            &mut vec![],
            &mut result,
        );
    }
    result
}
