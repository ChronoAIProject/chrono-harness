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
#[derive(Clone, Copy)]
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

/// A report-side invariant violation found while the judge is assembling its
/// own structural evidence.  These checks deliberately consume only the
/// already bound declarations and the produced report; they never discover a
/// dependency or turn a diagnostic into an optimisation decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureIssue {
    pub message: String,
}

/// Validate the internal consistency of a structure report.
///
/// The analysis is useful only when its witnesses are trustworthy.  Keeping a
/// small independent checker here means a future edit that drops an edge,
/// mislabels an SCC, or reports an impossible affected component becomes an
/// explicit judge error instead of silently publishing a plausible report.
/// This is intentionally a consistency check, not a second dependency
/// discovery mechanism.
pub fn validate_structure(
    input: AnalysisInput<'_>,
    closure: &Closure,
    analysis: &StructureAnalysis,
) -> Vec<StructureIssue> {
    let mut issues = vec![];
    if analysis.schema != "chrono-filemap-structure/v1" {
        issues.push(StructureIssue {
            message: format!("unexpected structure schema {:?}", analysis.schema),
        });
    }
    validate_snapshot(
        "base",
        &analysis.base,
        input.base,
        input.base_nodes,
        input.owners,
        &mut issues,
    );
    validate_snapshot(
        "candidate",
        &analysis.candidate,
        input.candidate,
        input.candidate_nodes,
        input.owners,
        &mut issues,
    );
    let union_edges: BTreeSet<_> = input.union.iter().map(|edge| edge.edge.clone()).collect();
    validate_snapshot(
        "union",
        &analysis.union,
        &union_edges,
        input.union_nodes,
        input.owners,
        &mut issues,
    );

    let component_of: BTreeMap<_, _> = analysis
        .union
        .components
        .iter()
        .flat_map(|component| {
            component
                .nodes
                .iter()
                .map(move |node| (node.clone(), component.id.clone()))
        })
        .collect();
    let expected_affected: BTreeSet<_> = closure
        .reached
        .keys()
        .filter_map(|node| component_of.get(node))
        .cloned()
        .collect();
    let actual_affected: BTreeSet<_> = analysis.affected_components.iter().cloned().collect();
    if expected_affected != actual_affected {
        issues.push(StructureIssue {
            message: format!(
                "affected components disagree with closure: expected {:?}, observed {:?}",
                expected_affected, actual_affected
            ),
        });
    }
    if analysis
        .affected_components
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        issues.push(StructureIssue {
            message: "affected components are not sorted and unique".into(),
        });
    }
    validate_execution(
        &analysis.execution.base,
        input.base_filemap,
        "base",
        &mut issues,
    );
    validate_execution(
        &analysis.execution.candidate,
        input.candidate_filemap,
        "candidate",
        &mut issues,
    );
    issues
}

fn validate_snapshot(
    endpoint: &str,
    snapshot: &GraphSnapshot,
    edges: &BTreeSet<Edge>,
    declared_nodes: &BTreeSet<String>,
    owners: &BTreeMap<String, BTreeSet<String>>,
    issues: &mut Vec<StructureIssue>,
) {
    let mut nodes = declared_nodes.clone();
    let mut outgoing: BTreeMap<String, usize> = BTreeMap::new();
    for edge in edges {
        nodes.insert(edge.from.clone());
        nodes.insert(edge.to.clone());
        *outgoing.entry(edge.from.clone()).or_default() += 1;
    }
    if snapshot.node_count != nodes.len() {
        issues.push(StructureIssue {
            message: format!(
                "{endpoint} node count {} does not match {} bound nodes",
                snapshot.node_count,
                nodes.len()
            ),
        });
    }
    if snapshot.edge_count != edges.len() {
        issues.push(StructureIssue {
            message: format!(
                "{endpoint} edge count {} does not match {} bound edges",
                snapshot.edge_count,
                edges.len()
            ),
        });
    }
    let mut expected_direct = BTreeMap::new();
    for node in &nodes {
        expected_direct.insert(node.clone(), outgoing.get(node).copied().unwrap_or(0));
    }
    if snapshot.direct_relations != expected_direct {
        issues.push(StructureIssue {
            message: format!("{endpoint} direct relation counts disagree with bound edges"),
        });
    }
    let mut seen = BTreeSet::new();
    let mut component_of = BTreeMap::new();
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edge in edges {
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .insert(edge.to.clone());
    }
    for component in &snapshot.components {
        if component.nodes.is_empty() || component.nodes.windows(2).any(|pair| pair[0] >= pair[1]) {
            issues.push(StructureIssue {
                message: format!(
                    "{endpoint} component {:?} is empty or unsorted",
                    component.id
                ),
            });
        }
        if component
            .nodes
            .first()
            .is_some_and(|node| component.id != format!("scc:{node}"))
        {
            issues.push(StructureIssue {
                message: format!(
                    "{endpoint} component {:?} has an invalid identity",
                    component.id
                ),
            });
        }
        for node in &component.nodes {
            if !seen.insert(node.clone()) {
                issues.push(StructureIssue {
                    message: format!("{endpoint} node {node} occurs in multiple components"),
                });
            }
            component_of.insert(node.clone(), component.id.clone());
        }
        let expected_cyclic = component.nodes.len() > 1
            || component.nodes.first().is_some_and(|node| {
                edges
                    .iter()
                    .any(|edge| edge.from == *node && edge.to == *node)
            });
        if component.cyclic != expected_cyclic {
            issues.push(StructureIssue {
                message: format!(
                    "{endpoint} component {:?} cyclic flag disagrees with bound edges",
                    component.id
                ),
            });
        }
        validate_witness(endpoint, component, edges, issues);
    }
    if seen != nodes {
        issues.push(StructureIssue {
            message: format!("{endpoint} components do not partition the bound node set"),
        });
    }
    // Check the partition independently from the producer's SCC algorithm:
    // every reported component must be mutually reachable, and its
    // condensation graph must be acyclic.  This catches both a dropped cycle
    // and an accidental merge without reusing the producer's traversal.
    for component in &snapshot.components {
        for start in &component.nodes {
            let reachable = reachable_nodes(start, &adjacency);
            if component.nodes.iter().any(|node| !reachable.contains(node)) {
                issues.push(StructureIssue {
                    message: format!(
                        "{endpoint} component {:?} is not strongly connected",
                        component.id
                    ),
                });
                break;
            }
        }
    }
    let mut condensed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for component in &snapshot.components {
        condensed.entry(component.id.clone()).or_default();
    }
    for edge in edges {
        if let (Some(from), Some(to)) = (component_of.get(&edge.from), component_of.get(&edge.to))
            && from != to
        {
            condensed
                .entry(from.clone())
                .or_default()
                .insert(to.clone());
        }
    }
    if directed_cycle(&condensed) {
        issues.push(StructureIssue {
            message: format!("{endpoint} SCC condensation graph contains a cycle"),
        });
    }
    let component_ids: BTreeSet<_> = snapshot
        .components
        .iter()
        .map(|component| component.id.clone())
        .collect();
    if snapshot
        .component_depth
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        != component_ids
    {
        issues.push(StructureIssue {
            message: format!("{endpoint} component depths do not cover every component"),
        });
    }
    let expected_max = snapshot
        .component_depth
        .values()
        .copied()
        .max()
        .unwrap_or(0);
    if snapshot.max_depth != expected_max {
        issues.push(StructureIssue {
            message: format!("{endpoint} maximum depth is inconsistent with component depths"),
        });
    }
    if let Some(expected_depth) =
        expected_component_depths(&component_of, &snapshot.components, edges)
        && snapshot.component_depth != expected_depth
    {
        issues.push(StructureIssue {
            message: format!("{endpoint} component depths disagree with the condensed graph"),
        });
    }
    let expected_cross_owner = edges
        .iter()
        .filter_map(|edge| {
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
                Some(CrossOwnerEdge {
                    edge: edge.clone(),
                    from_owners,
                    to_owners,
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if snapshot.cross_owner_edges != expected_cross_owner {
        issues.push(StructureIssue {
            message: format!("{endpoint} cross-owner edge diagnostics disagree with bound owners"),
        });
    }
    for edge in edges {
        if !component_of.contains_key(&edge.from) || !component_of.contains_key(&edge.to) {
            issues.push(StructureIssue {
                message: format!("{endpoint} edge {:?} has no component witness", edge),
            });
        }
    }
}

fn expected_component_depths(
    component_of: &BTreeMap<String, String>,
    components: &[StrongComponent],
    edges: &BTreeSet<Edge>,
) -> Option<BTreeMap<String, usize>> {
    let mut predecessors: BTreeMap<String, BTreeSet<String>> = components
        .iter()
        .map(|component| (component.id.clone(), BTreeSet::new()))
        .collect();
    for edge in edges {
        let (Some(from), Some(to)) = (component_of.get(&edge.from), component_of.get(&edge.to))
        else {
            continue;
        };
        if from != to {
            predecessors
                .entry(to.clone())
                .or_default()
                .insert(from.clone());
        }
    }
    independent_depths(&predecessors)
}

/// Compute longest paths with a Kahn walk rather than the producer's recursive
/// helper.  This is intentionally duplicated at the validation boundary so a
/// defect in report construction cannot reproduce itself in the checker.
fn independent_depths(
    predecessors: &BTreeMap<String, BTreeSet<String>>,
) -> Option<BTreeMap<String, usize>> {
    let mut indegree: BTreeMap<_, _> = predecessors
        .iter()
        .map(|(node, parents)| (node.clone(), parents.len()))
        .collect();
    let mut children: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (node, parents) in predecessors {
        for parent in parents {
            children
                .entry(parent.clone())
                .or_default()
                .insert(node.clone());
        }
    }
    let mut ready: BTreeSet<_> = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(node.clone()))
        .collect();
    let mut depths: BTreeMap<_, _> = indegree.keys().cloned().map(|node| (node, 0)).collect();
    let mut processed = 0;
    while let Some(node) = ready.pop_first() {
        processed += 1;
        let parent_depth = depths[&node];
        for child in children.get(&node).into_iter().flatten() {
            let depth = depths.get_mut(child).expect("child is in predecessor map");
            *depth = (*depth).max(parent_depth + 1);
            let degree = indegree.get_mut(child).expect("child has an indegree");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(child.clone());
            }
        }
    }
    (processed == indegree.len()).then_some(depths)
}

fn reachable_nodes(
    start: &str,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut reached = BTreeSet::from([start.to_string()]);
    let mut queue = VecDeque::from([start.to_string()]);
    while let Some(node) = queue.pop_front() {
        for next in adjacency.get(node.as_str()).into_iter().flatten() {
            if reached.insert(next.clone()) {
                queue.push_back(next.clone());
            }
        }
    }
    reached
}

fn directed_cycle(adjacency: &BTreeMap<String, BTreeSet<String>>) -> bool {
    fn visit(
        node: &str,
        adjacency: &BTreeMap<String, BTreeSet<String>>,
        visiting: &mut BTreeSet<String>,
        finished: &mut BTreeSet<String>,
    ) -> bool {
        if visiting.contains(node) {
            return true;
        }
        if !finished.insert(node.to_string()) {
            return false;
        }
        visiting.insert(node.to_string());
        let cycle = adjacency
            .get(node)
            .into_iter()
            .flatten()
            .any(|next| visit(next, adjacency, visiting, finished));
        visiting.remove(node);
        cycle
    }
    let mut visiting = BTreeSet::new();
    let mut finished = BTreeSet::new();
    for node in adjacency.keys() {
        if visit(node, adjacency, &mut visiting, &mut finished) {
            return true;
        }
    }
    false
}

fn validate_witness(
    endpoint: &str,
    component: &StrongComponent,
    edges: &BTreeSet<Edge>,
    issues: &mut Vec<StructureIssue>,
) {
    if !component.cyclic {
        if !component.witness.is_empty() {
            issues.push(StructureIssue {
                message: format!(
                    "{endpoint} acyclic component {:?} has a cycle witness",
                    component.id
                ),
            });
        }
        return;
    }
    if component.witness.is_empty() {
        issues.push(StructureIssue {
            message: format!(
                "{endpoint} cyclic component {:?} has no witness",
                component.id
            ),
        });
        return;
    }
    for edge in &component.witness {
        if !edges.contains(edge)
            || !component.nodes.contains(&edge.from)
            || !component.nodes.contains(&edge.to)
        {
            issues.push(StructureIssue {
                message: format!(
                    "{endpoint} component {:?} witness uses an unbound edge",
                    component.id
                ),
            });
        }
    }
    if component
        .witness
        .windows(2)
        .any(|pair| pair[0].to != pair[1].from)
        || component.witness.first().map(|edge| &edge.from)
            != component.witness.last().map(|edge| &edge.to)
    {
        issues.push(StructureIssue {
            message: format!(
                "{endpoint} component {:?} witness is not a closed path",
                component.id
            ),
        });
    }
}

fn validate_execution(
    snapshot: &ExecutionSnapshot,
    filemap: &Value,
    endpoint: &str,
    issues: &mut Vec<StructureIssue>,
) {
    let operations: BTreeSet<_> = snapshot.operations.iter().cloned().collect();
    if operations.len() != snapshot.operations.len()
        || snapshot
            .operations
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        issues.push(StructureIssue {
            message: format!("{endpoint} execution operations are not sorted and unique"),
        });
    }
    let mut expected_operations = BTreeSet::new();
    let mut expected_adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Some(plans) = filemap.get("execution_plans").and_then(Value::as_object) {
        for plan in plans.values() {
            let ids: Vec<_> = plan["operations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            expected_operations.extend(ids.iter().map(|id| (*id).to_string()));
            for pair in ids.windows(2) {
                expected_adjacency
                    .entry(pair[0].to_string())
                    .or_default()
                    .insert(pair[1].to_string());
            }
        }
    }
    if operations != expected_operations {
        issues.push(StructureIssue {
            message: format!("{endpoint} execution operations disagree with registered plans"),
        });
    }
    let mut expected_edges = BTreeSet::new();
    if let Some(plans) = filemap.get("execution_plans").and_then(Value::as_object) {
        for (test, plan) in plans {
            let ids: Vec<_> = plan["operations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            for pair in ids.windows(2) {
                expected_edges.insert((pair[0].to_string(), pair[1].to_string(), test.clone()));
            }
        }
    }
    let actual_edges: BTreeSet<_> = snapshot
        .edges
        .iter()
        .flat_map(|edge| {
            edge.tests
                .iter()
                .map(move |test| (edge.from.clone(), edge.to.clone(), test.clone()))
        })
        .collect();
    if actual_edges != expected_edges {
        issues.push(StructureIssue {
            message: format!(
                "{endpoint} execution precedence edges disagree with registered plans"
            ),
        });
    }
    let actual_edge_keys: Vec<_> = snapshot
        .edges
        .iter()
        .map(|edge| (edge.from.clone(), edge.to.clone()))
        .collect();
    let unique_edge_keys: BTreeSet<_> = actual_edge_keys.iter().cloned().collect();
    if unique_edge_keys.len() != actual_edge_keys.len()
        || actual_edge_keys.windows(2).any(|pair| pair[0] >= pair[1])
    {
        issues.push(StructureIssue {
            message: format!("{endpoint} execution precedence edges are not sorted and unique"),
        });
    }
    for edge in &snapshot.edges {
        if edge.tests.is_empty()
            || edge.tests.windows(2).any(|pair| pair[0] >= pair[1])
            || edge.tests.windows(2).any(|pair| pair[0] == pair[1])
        {
            issues.push(StructureIssue {
                message: format!("{endpoint} execution edge has unsorted or empty test witnesses"),
            });
        }
    }
    for edge in &snapshot.edges {
        if !operations.contains(&edge.from) || !operations.contains(&edge.to) {
            issues.push(StructureIssue {
                message: format!(
                    "{endpoint} execution edge {:?} references an unknown operation",
                    edge
                ),
            });
        }
    }
    for cycle in &snapshot.cycles {
        if cycle.len() < 2 || cycle.first() != cycle.last() {
            issues.push(StructureIssue {
                message: format!("{endpoint} execution cycle is not closed"),
            });
        }
        if cycle.windows(2).any(|pair| {
            !snapshot
                .edges
                .iter()
                .any(|edge| edge.from == pair[0] && edge.to == pair[1])
        }) {
            issues.push(StructureIssue {
                message: format!("{endpoint} execution cycle uses an unbound precedence edge"),
            });
        }
    }
    let expected_cycles = string_cycles(&expected_operations, &expected_adjacency);
    if snapshot.cycles != expected_cycles {
        issues.push(StructureIssue {
            message: format!(
                "{endpoint} execution cycle diagnostics disagree with registered plans"
            ),
        });
    }
    let expected_depth = if expected_cycles.is_empty() {
        let mut predecessors: BTreeMap<String, BTreeSet<String>> = expected_operations
            .iter()
            .map(|operation| (operation.clone(), BTreeSet::new()))
            .collect();
        for edge in &expected_edges {
            predecessors
                .entry(edge.1.clone())
                .or_default()
                .insert(edge.0.clone());
        }
        independent_depths(&predecessors).unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    if snapshot.depth != expected_depth {
        issues.push(StructureIssue {
            message: format!("{endpoint} execution depths disagree with registered plans"),
        });
    }
    let expected_conflicts = independent_resource_conflicts(filemap);
    if canonical_conflicts(&snapshot.resource_conflicts) != canonical_conflicts(&expected_conflicts)
    {
        issues.push(StructureIssue {
            message: format!(
                "{endpoint} execution resource diagnostics disagree with registered claims"
            ),
        });
    }
}

fn canonical_conflicts(conflicts: &[ExecutionConflict]) -> Vec<ExecutionConflict> {
    let mut conflicts = conflicts.to_vec();
    for conflict in &mut conflicts {
        conflict.resources.sort();
        conflict.resources.dedup();
        conflict.outputs.sort();
        conflict.outputs.dedup();
    }
    conflicts.sort_by(|left, right| {
        (&left.left, &left.right, &left.resources, &left.outputs).cmp(&(
            &right.left,
            &right.right,
            &right.resources,
            &right.outputs,
        ))
    });
    conflicts
}

/// Independent set-based reconstruction of scheduling conflicts for the
/// report validator.  The producer preserves declaration order; the checker
/// compares canonical sets so it can catch missing/extra conflicts without
/// depending on that presentation detail.
fn independent_resource_conflicts(filemap: &Value) -> Vec<ExecutionConflict> {
    let Some(claims) = filemap
        .get("execution_scheduling")
        .and_then(|value| value.get("claims"))
        .and_then(Value::as_object)
    else {
        return vec![];
    };
    let ids: Vec<_> = claims.keys().cloned().collect();
    let mut conflicts = vec![];
    for (index, left) in ids.iter().enumerate() {
        for right in ids.iter().skip(index + 1) {
            let left_claim = &claims[left];
            let right_claim = &claims[right];
            let left_resources: BTreeSet<_> = left_claim["resources"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            let right_resources: BTreeSet<_> = right_claim["resources"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            let resources: Vec<_> = left_resources
                .intersection(&right_resources)
                .cloned()
                .collect();
            let left_outputs: Vec<_> = left_claim["outputs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let right_outputs: Vec<_> = right_claim["outputs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let outputs: BTreeSet<_> = left_outputs
                .iter()
                .filter(|left_output| {
                    right_outputs.iter().any(|right_output| {
                        chrono_harness::units::overlap(left_output, right_output)
                    })
                })
                .map(|output| (*output).to_string())
                .collect();
            if !resources.is_empty() || !outputs.is_empty() {
                conflicts.push(ExecutionConflict {
                    left: left.clone(),
                    right: right.clone(),
                    resources,
                    outputs: outputs.into_iter().collect(),
                });
            }
        }
    }
    conflicts
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
    let resource_conflicts = expected_resource_conflicts(filemap);
    ExecutionSnapshot {
        operations: operations.into_iter().collect(),
        edges,
        cycles,
        depth,
        resource_conflicts,
    }
}

fn expected_resource_conflicts(filemap: &Value) -> Vec<ExecutionConflict> {
    filemap
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
        .unwrap_or_default()
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
