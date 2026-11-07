//! Finite union/closure mechanics. Callers own endpoint validation, seeds and extra selections.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub use crate::verified_graph::{
    CauseSet, Edge, EdgeKind, Origin, Predecessor, PredecessorTree, Seed, TraversedEdge, UnionEdge,
    union,
};

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
impl std::fmt::Debug for Seed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Seed")
            .field("id", &self.id)
            .field("node", &self.node)
            .field("reference", &self.reference)
            .field("reason", &self.reason)
            .finish()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(remote = "Seed", rename = "Seed")]
struct SeedFields {
    id: String,
    node: String,
    reference: String,
    reason: String,
}
impl Serialize for Seed {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SeedFields::serialize(self, serializer)
    }
}
impl<'de> Deserialize<'de> for Seed {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        SeedFields::deserialize(deserializer)
    }
}
impl std::fmt::Debug for PredecessorTree {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_map()
            .entries(self.entries().iter().map(|row| (&row.node, &row.parent)))
            .finish()
    }
}
impl Serialize for PredecessorTree {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.entries().len()))?;
        for row in self.entries() {
            map.serialize_entry(&row.node, &row.parent)?;
        }
        map.end()
    }
}
impl<'de> Deserialize<'de> for PredecessorTree {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows = BTreeMap::<String, Option<Edge>>::deserialize(deserializer)?
            .into_iter()
            .map(|(node, parent)| Predecessor { node, parent })
            .collect();
        Self::new(rows)
            .ok_or_else(|| serde::de::Error::custom("predecessor nodes must be strictly ordered"))
    }
}
impl std::fmt::Debug for CauseSet {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_set().entries(self.entries()).finish()
    }
}
impl Serialize for CauseSet {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.entries().serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for CauseSet {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let names = Vec::<String>::deserialize(deserializer)?;
        Ok(Self::from_names(&names))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Closure {
    pub reached: BTreeMap<String, CauseSet>,
    /// One deterministic shortest predecessor per (seed, node); null denotes the seed root.
    pub predecessors: BTreeMap<String, PredecessorTree>,
    /// All traversed edges and contributing seeds, including alternate paths and cycle edges.
    pub traversed: Vec<TraversedEdge>,
}
pub fn closure(edges: &[UnionEdge], seeds: &[Seed]) -> Closure {
    let mut out = Closure {
        reached: BTreeMap::new(),
        predecessors: BTreeMap::new(),
        traversed: vec![],
    };
    let topology = crate::verified_graph::seed_topology(edges, seeds);
    let mut traversed: BTreeMap<Edge, CauseSet> = BTreeMap::new();
    let propagations = crate::verified_graph::SeedPropagations::new(&topology, seeds);
    for item in propagations {
        let seed = item.seed;
        let propagation = item.propagation;
        for row in propagation.rows {
            out.reached
                .entry(row.node.to_owned())
                .or_default()
                .insert(seed.id.as_str());
            for edge in row.outgoing {
                traversed
                    .entry((*edge).clone())
                    .or_default()
                    .insert(seed.id.as_str());
            }
        }
        out.predecessors
            .insert(seed.id.clone(), propagation.predecessors);
    }
    out.traversed = traversed
        .into_iter()
        .map(|(edge, seeds)| TraversedEdge { edge, seeds })
        .collect();
    out
}
impl Closure {
    pub fn selected_tests(&self) -> BTreeSet<String> {
        BTreeSet::from_iter(crate::verified_graph::selected_test_targets(
            &self.traversed,
        ))
    }
    pub fn witness(&self, seed: &str, node: &str) -> Option<Vec<Edge>> {
        self.predecessors.get(seed)?.witness(node)
    }
}
