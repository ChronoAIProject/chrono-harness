//! Finite union/closure mechanics. Callers own endpoint validation, seeds and extra selections.
use serde::{Deserialize, Serialize};
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
