//! Candidate FILEMAP impact policy. No operation execution, retirement or cost verdicts.
pub mod graph;
mod records;
use chrono_harness::{
    facts,
    wire::{Delta, Finding, Request, Response, Status},
};
use chrono_judge_registration::{NodeKind, Registrations};
use graph::{Closure, Edge, EdgeKind, Origin, Seed, UnionEdge};
pub use records::{Change, Record, RecordPair};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const IMPACT_SCHEMA: &str = "chrono-filemap-impact/v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodePair {
    /// Only uniquely resolved values. Absent and ambiguous endpoints are distinguished by definitions.
    pub base: Option<Value>,
    pub candidate: Option<Value>,
    pub base_definitions: Vec<NodeDefinition>,
    pub candidate_definitions: Vec<NodeDefinition>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub identity: String,
    pub value: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeAmbiguity {
    pub node: String,
    pub endpoint: String,
    pub definitions: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequiredTest {
    pub node: String,
    pub candidate_present: bool,
    pub candidate_ambiguous: bool,
    pub execution_edges: Vec<Edge>,
    pub base_cost: Option<Value>,
    pub candidate_cost: Option<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JudgeSelection {
    pub node: String,
    pub every_delta: bool,
    pub triggers: Vec<Edge>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeProblem {
    pub edge: Edge,
    pub endpoint: String,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Impact {
    pub schema: String,
    pub delta: Vec<Delta>,
    pub records: BTreeMap<String, RecordPair>,
    pub changes: Vec<Change>,
    pub nodes: BTreeMap<String, NodePair>,
    pub edges: Vec<UnionEdge>,
    pub seeds: Vec<String>,
    pub seed_causes: Vec<Seed>,
    pub closure: Closure,
    pub tests: Vec<String>,
    pub retired_tests: Vec<String>,
    pub required_tests: Vec<RequiredTest>,
    pub judges: Vec<JudgeSelection>,
    pub historical_context: Vec<EdgeProblem>,
    pub historical_ambiguities: Vec<NodeAmbiguity>,
    pub limits: Vec<String>,
}
fn edges(r: &Registrations) -> BTreeSet<Edge> {
    let mut out = BTreeSet::new();
    let mut add = |from: &str, e: &Value| {
        out.insert(Edge {
            from: from.into(),
            kind: serde_json::from_value(e["kind"].clone()).unwrap(),
            to: e["to"].as_str().unwrap().into(),
        });
    };
    for f in r.filemap()["files"].as_array().unwrap() {
        for e in f["edges"].as_array().unwrap() {
            add(&format!("file:{}", f["path"].as_str().unwrap()), e);
        }
    }
    for e in r.filemap()["project_edges"].as_array().unwrap() {
        add(e["from"].as_str().unwrap(), e);
    }
    out
}
fn valid_types(kind: EdgeKind, from: NodeKind, to: NodeKind) -> bool {
    use NodeKind::*;
    match kind {
        EdgeKind::Compile => matches!(from, File | Project) && to == Project,
        EdgeKind::BuildInput | EdgeKind::RuntimeInput => {
            matches!(from, File | Project | Script | Input | Tool | Environment)
                && matches!(to, Project | Script)
        }
        EdgeKind::TestExecution => {
            matches!(from, File | Project | Script | Input | Tool | Environment) && to == Test
        }
        EdgeKind::JudgeTrigger => matches!(from, File | Project) && to == Judge,
    }
}
fn cost(r: &Registrations, test: &str) -> Option<Value> {
    r.filemap()["test_costs"].as_array().unwrap().iter().find(|c| c["test"].as_str() == test.strip_prefix("test:")).map(|c| {
        json!({"reference": c["cost"], "value": r.filemap()["cost_models"].get(c["cost"].as_str().unwrap())})
    })
}
/// Pure declaration-level impact. External input records are compared, not retained input bytes.
/// Callers must supply factual, complete DELTA; the judge entrypoint binds it to fixed Git objects.
pub fn produce(
    base: &Registrations,
    candidate: &Registrations,
    config_path: &str,
    delta: &[Delta],
) -> (Impact, Vec<Finding>) {
    produce_environment(base, candidate, config_path, delta, None)
}
/// Effective-input impact for validated retained endpoint facts. Declarations and
/// effective values use the same explicit environment nodes; no edges are inferred.
pub fn produce_with_inputs(
    base: &Registrations,
    candidate: &Registrations,
    config_path: &str,
    delta: &[Delta],
    effective_inputs: &Value,
) -> Result<(Impact, Vec<Finding>), String> {
    let environments = chrono_judge_registration::inputs::effective_environments(
        effective_inputs,
        base,
        candidate,
    )?;
    Ok(produce_environment(
        base,
        candidate,
        config_path,
        delta,
        environments.as_ref(),
    ))
}
fn produce_environment(
    base: &Registrations,
    candidate: &Registrations,
    config_path: &str,
    delta: &[Delta],
    environments: Option<&[BTreeMap<String, String>; 2]>,
) -> (Impact, Vec<Finding>) {
    let an = base.node_data();
    let bn = candidate.node_data();
    let a = records::inventory(base, config_path, &an, environments.map(|p| &p[0]));
    let b = records::inventory(candidate, config_path, &bn, environments.map(|p| &p[1]));
    let mut records = BTreeMap::new();
    let mut changes = vec![];
    let mut seeds = BTreeSet::new();
    let seed = |node: String, reference: String, reason: &str| Seed {
        id: serde_json::to_string(&json!([reference, reason, node])).unwrap(),
        node,
        reference,
        reason: reason.into(),
    };
    for d in delta {
        seeds.insert(seed(
            format!("file:{}", d.path),
            d.path.clone(),
            "delta-path",
        ));
    }
    for id in a.keys().chain(b.keys()).collect::<BTreeSet<_>>() {
        let av = a.get(id);
        let bv = b.get(id);
        let mut fields = vec![];
        records::changed_fields(av.map(|r| &r.value), bv.map(|r| &r.value), "", &mut fields);
        // Registry relocation is an input change even when the record's value is identical.
        if av.zip(bv).is_some_and(|(a, b)| a.path != b.path) {
            fields.push("/$registry_path".into());
        }
        if !fields.is_empty() {
            for record in av.into_iter().chain(bv) {
                for target in &record.targets {
                    seeds.insert(seed(target.clone(), id.clone(), "record-change"));
                }
            }
            changes.push(Change {
                record: id.clone(),
                fields,
            });
        }
        records.insert(
            id.clone(),
            RecordPair {
                base: av.cloned(),
                candidate: bv.cloned(),
            },
        );
    }
    let ae = edges(base);
    let be = edges(candidate);
    let edges = graph::union(&ae, &be);
    for e in ae.symmetric_difference(&be) {
        let file_record = e
            .from
            .strip_prefix("file:")
            .map(|p| format!("/records/filemap/files/{}", records::escape(p)));
        let declared_on_file = [base, candidate].iter().any(|r| {
            r.filemap()["files"].as_array().unwrap().iter().any(|f| {
                format!("file:{}", f["path"].as_str().unwrap()) == e.from
                    && f["edges"].as_array().unwrap().iter().any(|row| {
                        row["to"] == e.to && row["kind"] == serde_json::to_value(e.kind).unwrap()
                    })
            })
        });
        let reference = file_record.filter(|_| declared_on_file).unwrap_or_else(|| {
            format!(
                "/records/filemap/project_edges/{}",
                records::escape(&serde_json::to_string(&json!([e.from, e.kind, e.to])).unwrap())
            )
        });
        for node in [&e.from, &e.to] {
            seeds.insert(seed(node.clone(), reference.clone(), "edge-change"));
        }
    }
    let seeds: Vec<_> = seeds.into_iter().collect();
    let closure = graph::closure(&edges, &seeds);
    let definitions = |view: Option<&chrono_judge_registration::NodeView<'_>>| {
        view.into_iter()
            .flat_map(|v| &v.definitions)
            .map(|d| NodeDefinition {
                identity: d.identity.clone(),
                value: d.value.clone(),
            })
            .collect()
    };
    let nodes = an
        .keys()
        .chain(bn.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|id| {
            (
                id.clone(),
                NodePair {
                    base: an.get(id).and_then(|n| n.unique()).map(|d| d.value.clone()),
                    candidate: bn.get(id).and_then(|n| n.unique()).map(|d| d.value.clone()),
                    base_definitions: definitions(an.get(id)),
                    candidate_definitions: definitions(bn.get(id)),
                },
            )
        })
        .collect();
    let mut findings = vec![];
    let mut historical_context = vec![];
    let mut historical_ambiguities = vec![];
    for (endpoint, inventory) in [("base", &an), ("candidate", &bn)] {
        for (node, view) in inventory {
            if view.definitions.len() <= 1 {
                continue;
            }
            let ambiguity = NodeAmbiguity {
                node: node.clone(),
                endpoint: endpoint.into(),
                definitions: view
                    .definitions
                    .iter()
                    .map(|d| d.identity.clone())
                    .collect(),
            };
            if let Some(causes) = closure.reached.get(node) {
                if endpoint == "base"
                    && chrono_judge_registration::ambiguity_repaired(
                        candidate,
                        node,
                        &ambiguity.definitions,
                    )
                {
                    historical_ambiguities.push(ambiguity);
                    continue;
                }
                let cause = causes.first().unwrap();
                let seed = seeds.iter().find(|s| &s.id == cause).unwrap();
                let mut witness = vec![seed.node.clone()];
                witness.extend(
                    closure
                        .witness(cause, node)
                        .unwrap()
                        .iter()
                        .map(|e| e.to.clone()),
                );
                findings.push(Finding {
                    code: "E_NODE_AMBIGUOUS".into(),
                    level: "error".into(),
                    message: format!(
                        "{endpoint}: ambiguous node {node}: {:?}",
                        ambiguity.definitions
                    ),
                    delta_refs: vec![seed.reference.clone()],
                    causes: witness,
                });
            } else {
                historical_ambiguities.push(ambiguity);
            }
        }
    }
    for e in &edges {
        for (endpoint, inventory) in [("base", &an), ("candidate", &bn)] {
            if matches!(
                (e.origin, endpoint),
                (Origin::Base, "candidate") | (Origin::Candidate, "base")
            ) {
                continue;
            }
            let edge = &e.edge;
            let message = match (inventory.get(&edge.from), inventory.get(&edge.to)) {
                (Some(from), Some(to)) if !valid_types(edge.kind, from.kind, to.kind) => {
                    Some("invalid typed edge endpoints".to_string())
                }
                (None, _) => Some(format!("missing source node {}", edge.from)),
                (_, None) => Some(format!("missing target node {}", edge.to)),
                _ => None,
            };
            if let Some(message) = message {
                let problem = EdgeProblem {
                    edge: edge.clone(),
                    endpoint: endpoint.into(),
                    message: message.clone(),
                };
                let reached = closure
                    .reached
                    .get(&edge.from)
                    .map(|s| (&edge.from, s))
                    .or_else(|| closure.reached.get(&edge.to).map(|s| (&edge.to, s)));
                if let Some((node, causes)) = reached {
                    let cause = causes.first().unwrap();
                    let seed = seeds.iter().find(|s| &s.id == cause).unwrap();
                    let path = closure.witness(cause, node).unwrap();
                    let mut witness = vec![seed.node.clone()];
                    witness.extend(path.iter().map(|e| e.to.clone()));
                    if node == &edge.from {
                        witness.push(edge.to.clone());
                    }
                    findings.push(Finding {
                        code: if message.starts_with("missing") {
                            "E_DANGLING_EDGE"
                        } else {
                            "E_EDGE_TYPE"
                        }
                        .into(),
                        level: "error".into(),
                        message: format!("{endpoint}: {message}: {edge:?}"),
                        delta_refs: vec![seed.reference.clone()],
                        causes: witness,
                    });
                } else {
                    historical_context.push(problem);
                }
            }
        }
    }
    let required_tests: Vec<_> = closure
        .selected_tests()
        .into_iter()
        .map(|node| RequiredTest {
            candidate_present: bn.get(&node).is_some_and(|n| n.kind == NodeKind::Test),
            candidate_ambiguous: bn.get(&node).is_some_and(|n| n.definitions.len() > 1),
            execution_edges: closure
                .traversed
                .iter()
                .filter(|e| e.edge.kind == EdgeKind::TestExecution && e.edge.to == node)
                .map(|e| e.edge.clone())
                .collect(),
            base_cost: cost(base, &node),
            candidate_cost: cost(candidate, &node),
            node,
        })
        .collect();
    let mut judges: Vec<_> = candidate.judges()["judges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| {
            let node = format!("judge:{}", j["id"].as_str().unwrap());
            JudgeSelection {
                every_delta: j["selector"] == "every-delta",
                triggers: closure
                    .traversed
                    .iter()
                    .filter(|e| e.edge.kind == EdgeKind::JudgeTrigger && e.edge.to == node)
                    .map(|e| e.edge.clone())
                    .collect(),
                node,
            }
        })
        .collect();
    judges.sort_by(|a, b| a.node.cmp(&b.node));
    (
        Impact {
            schema: IMPACT_SCHEMA.into(),
            delta: delta.to_vec(),
            records,
            changes,
            nodes,
            edges,
            seeds: seeds
                .iter()
                .map(|s| s.node.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            seed_causes: seeds,
            closure,
            tests: required_tests
                .iter()
                .filter(|t| t.candidate_present)
                .map(|t| t.node.clone())
                .collect(),
            retired_tests: required_tests
                .iter()
                .filter(|t| !t.candidate_present)
                .map(|t| t.node.clone())
                .collect(),
            required_tests,
            judges,
            historical_context,
            historical_ambiguities,
            limits: vec![
                "Declaration impact only; no test execution, retirement approval or cost verdict"
                    .into(),
                "External input declaration changes are not retained input evidence".into(),
                if environments.is_some() {
                    "Retained effective environments compared; completeness remains unproven".into()
                } else {
                    "Declaration-only API: retained effective environment differences not evaluated"
                        .into()
                },
                "Actual input completeness and locality are unproven; unknown costs remain null"
                    .into(),
            ],
        },
        findings,
    )
}
pub fn judge(req: &Request) -> Response {
    let mut response = req.response(Status::Pass);
    match evaluate(req) {
        Ok((impact, findings, view)) => {
            if !findings.is_empty() {
                response.status = Status::Fail;
            }
            response.findings = findings;
            response.outputs.insert("registration_view".into(), view);
            response
                .outputs
                .insert("impact".into(), serde_json::to_value(impact).unwrap());
        }
        Err(message) => {
            response.status = Status::Error;
            response.findings.push(Finding {
                code: "E_INPUT".into(),
                level: "error".into(),
                message,
                delta_refs: vec!["/request".into()],
                causes: vec![],
            });
        }
    }
    response
}
fn evaluate(req: &Request) -> Result<(Impact, Vec<Finding>, Value), String> {
    req.validate()?;
    let root = &req.candidate.root;
    if facts::verify_oid(root, &req.base.commit)? != req.base.tree
        || facts::verify_oid(root, &req.candidate.commit)? != req.candidate.tree
    {
        return Err("fixed endpoint tree mismatch".into());
    }
    if facts::delta(
        &facts::tree(root, &req.base.commit)?,
        &facts::tree(root, &req.candidate.commit)?,
    ) != req.delta
    {
        return Err("DELTA differs from fixed endpoints".into());
    }
    let (a, b, view) = chrono_judge_registration::views(req)?;
    let inputs = chrono_judge_registration::inputs::validate(req, &a, &b)?;
    let (impact, findings) = produce_with_inputs(&a, &b, &req.config_path, &req.delta, &inputs)?;
    Ok((impact, findings, view))
}
