//! Finite union/closure mechanics. Callers own endpoint validation, seeds and extra selections.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub use crate::verified_graph::{Edge, EdgeKind, Origin, UnionEdge, union};

impl std::fmt::Debug for EdgeKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Compile => "Compile",
            Self::BuildInput => "BuildInput",
            Self::RuntimeInput => "RuntimeInput",
            Self::TestExecution => "TestExecution",
            Self::JudgeTrigger => "JudgeTrigger",
        })
    }
}
impl Serialize for EdgeKind {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.wire_name())
    }
}
impl<'de> Deserialize<'de> for EdgeKind {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KindVisitor;
        impl serde::de::Visitor<'_> for KindVisitor {
            type Value = EdgeKind;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a canonical FILEMAP edge kind")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<EdgeKind, E> {
                EdgeKind::from_wire(value)
                    .ok_or_else(|| E::invalid_value(serde::de::Unexpected::Str(value), &self))
            }
        }
        deserializer.deserialize_str(KindVisitor)
    }
}
impl std::fmt::Debug for Edge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Edge")
            .field("from", &self.from)
            .field("kind", &self.kind)
            .field("to", &self.to)
            .finish()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "Edge", rename = "Edge")]
struct EdgeFields {
    from: String,
    kind: EdgeKind,
    to: String,
}
impl Serialize for Edge {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        EdgeFields::serialize(self, serializer)
    }
}
impl<'de> Deserialize<'de> for Edge {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        EdgeFields::deserialize(deserializer)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "Origin", rename_all = "lowercase")]
enum OriginFields {
    Base,
    Candidate,
    Both,
}
impl Serialize for Origin {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        OriginFields::serialize(self, serializer)
    }
}
impl<'de> Deserialize<'de> for Origin {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        OriginFields::deserialize(deserializer)
    }
}
impl std::fmt::Debug for Origin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Base => "Base",
            Self::Candidate => "Candidate",
            Self::Both => "Both",
        })
    }
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "UnionEdge", rename = "UnionEdge")]
struct UnionEdgeFields {
    #[serde(flatten)]
    edge: Edge,
    origin: Origin,
}
impl Serialize for UnionEdge {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        UnionEdgeFields::serialize(self, serializer)
    }
}
impl<'de> Deserialize<'de> for UnionEdge {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        UnionEdgeFields::deserialize(deserializer)
    }
}
impl std::fmt::Debug for UnionEdge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UnionEdge")
            .field("edge", &self.edge)
            .field("origin", &self.origin)
            .finish()
    }
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
pub fn closure(edges: &[UnionEdge], seeds: &[Seed]) -> Closure {
    let mut out = Closure {
        reached: BTreeMap::new(),
        predecessors: BTreeMap::new(),
        traversed: vec![],
    };
    let seed_names: Vec<&str> = seeds.iter().map(|seed| seed.node.as_str()).collect();
    let nodes = crate::verified_graph::graph_names(edges, &seed_names);
    let edge_rows = crate::verified_graph::source_rows(&nodes, edges)
        .expect("edge sources belong to the explicit node inventory");
    let indices = crate::verified_graph::index_edges(&nodes, &edge_rows, &seed_names)
        .expect("edge destinations and seeds belong to the explicit node inventory");
    drop(seed_names);
    let mut traversed: BTreeMap<Edge, BTreeSet<String>> = BTreeMap::new();
    for (seed, root) in seeds.iter().zip(indices.roots) {
        let search = crate::verified_graph::breadth_first(&indices.rows, root)
            .expect("seed index belongs to the explicit node inventory");
        let mut parents = BTreeMap::new();
        for node in search.order {
            out.reached
                .entry(nodes[node].to_owned())
                .or_default()
                .insert(seed.id.clone());
            parents.insert(
                nodes[node].to_owned(),
                search.parents[node].map(|(from, slot)| edge_rows[from][slot].clone()),
            );
            for edge in &edge_rows[node] {
                traversed
                    .entry((*edge).clone())
                    .or_default()
                    .insert(seed.id.clone());
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
