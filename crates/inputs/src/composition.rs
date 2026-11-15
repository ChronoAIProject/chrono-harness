//! Offline composition of explicitly addressed original endpoint captures.
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{
    prepared::{ArtifactTransport, Original},
    retained_artifacts, sha256,
};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub open spec fn optional_text_is(text: Option<&str>, expected: Seq<char>) -> bool {
    match text { Some(text) => text@ == expected, None => false, }
}

fn text_is(text: Option<&str>, expected: &str) -> (same: bool)
    ensures same == optional_text_is(text, expected@),
{
    match text { Some(text) => identifiers::equal_text(text, expected), None => false, }
}

pub fn admit_json_original(length: u64) -> (out: Result<(), String>)
    ensures out.is_ok() == (length <= 64 * 1024 * 1024),
        match out { Err(error) => error@ == "E_COMPOSE_BOUND: JSON original exceeds bound"@, _ => true, },
{
    if length > 64 * 1024 * 1024 {
        Err("E_COMPOSE_BOUND: JSON original exceeds bound".to_string())
    } else { Ok(()) }
}

pub fn admit_manifest(schema: &str, count: usize) -> (out: Result<(), String>)
    ensures out.is_ok() == (schema@ == "chrono-input-composition/v1"@ && 0 < count <= 4096),
        match out { Err(error) => error@ == "E_COMPOSE_MANIFEST: schema/count"@, _ => true, },
{
    if !identifiers::equal_text(schema, "chrono-input-composition/v1") || count == 0 || count > 4096 {
        Err("E_COMPOSE_MANIFEST: schema/count".to_string())
    } else { Ok(()) }
}

pub fn admit_output_paths(output: &str, receipt: &str) -> (out: Result<(), String>)
    ensures out.is_ok() == (output@ != receipt@),
        match out { Err(error) => error@ == "E_COMPOSE_PATH: output/receipt collision"@, _ => true, },
{
    if identifiers::equal_text(output, receipt) {
        Err("E_COMPOSE_PATH: output/receipt collision".to_string())
    } else { Ok(()) }
}

pub fn admit_pair_digest(actual: &str, expected: &str) -> (out: Result<(), String>)
    ensures out.is_ok() == (actual@ == expected@),
        match out { Err(error) => error@ == "E_COMPOSE_ORIGINAL: pair digest differs"@, _ => true, },
{
    if identifiers::equal_text(actual, expected) { Ok(()) }
    else { Err("E_COMPOSE_ORIGINAL: pair digest differs".to_string()) }
}

pub fn admit_single_endpoint(commit: Option<&str>, base: &str, candidate: &str)
    -> (out: Result<(), String>)
    ensures out.is_ok() == (optional_text_is(commit, base@) || optional_text_is(commit, candidate@)),
        match out { Err(error) => error@ == "E_COMPOSE_HEADER: snapshot outside fixed endpoints"@, _ => true, },
{
    if text_is(commit, base) || text_is(commit, candidate) { Ok(()) }
    else { Err("E_COMPOSE_HEADER: snapshot outside fixed endpoints".to_string()) }
}

pub fn admit_pair_fields(count: Option<usize>, has_base: bool, has_candidate: bool)
    -> (out: Result<(), String>)
    ensures out.is_ok() == (count == Some(2usize) && has_base && has_candidate),
        match out { Err(error) => error@ == "E_COMPOSE_PAIR: exact endpoints required"@, _ => true, },
{
    if count == Some(2usize) && has_base && has_candidate { Ok(()) }
    else { Err("E_COMPOSE_PAIR: exact endpoints required".to_string()) }
}

pub struct SnapshotHeader<'a> {
    pub digest: Option<&'a str>,
    pub selection_present: bool,
    pub effective_present: bool,
    pub effective: Option<&'a str>,
}

pub fn admit_snapshot_identity(schema: Option<&str>, commit: Option<&str>, config: Option<&str>,
    expected_schema: &str, expected_commit: &str, expected_config: &str)
    -> (out: Result<(), String>)
    ensures out.is_ok() == (optional_text_is(schema, expected_schema@)
        && optional_text_is(commit, expected_commit@) && optional_text_is(config, expected_config@)),
        match out { Err(error) => error@ == "configuration/endpoint/selector mismatch"@, _ => true, },
{
    if text_is(schema, expected_schema) && text_is(commit, expected_commit) && text_is(config, expected_config) {
        Ok(())
    } else { Err("configuration/endpoint/selector mismatch".to_string()) }
}

pub open spec fn snapshot_header_admitted(observed: SnapshotHeader, digest: Seq<char>, selected: bool,
    selection_equal: bool, effective: Seq<char>) -> bool
{
    optional_text_is(observed.digest, digest)
        && if selected { selection_equal && optional_text_is(observed.effective, effective) }
        else { !observed.selection_present && !observed.effective_present }
}

/// JSON selector equality is an observation; this root owns its admission together with every header.
pub fn admit_snapshot_header(observed: &SnapshotHeader, digest: &str,
    selected: bool, selection_equal: bool, effective: &str)
    -> (out: Result<(), String>)
    ensures out.is_ok() == snapshot_header_admitted(*observed, digest@, selected, selection_equal, effective@),
        match out { Err(error) => error@ == "configuration/endpoint/selector mismatch"@, _ => true, },
{
    let identity = text_is(observed.digest, digest);
    let selector = if selected { selection_equal && text_is(observed.effective, effective) }
        else { !observed.selection_present && !observed.effective_present };
    if identity && selector { Ok(()) }
    else { Err("configuration/endpoint/selector mismatch".to_string()) }
}

pub fn admit_imported_blob(actual: Option<&str>, actual_length: Option<u64>,
    expected: &str, expected_length: u64) -> (out: Result<(), String>)
    ensures out.is_ok() == (optional_text_is(actual, expected@) && actual_length == Some(expected_length)),
        match out { Err(error) => error@ == "E_COMPOSE_BLOB: original identity differs"@, _ => true, },
{
    let matches = match (actual, actual_length) {
        (Some(actual), Some(length)) => super::blob_identity_matches(actual, length, expected, expected_length),
        _ => false,
    };
    if matches { Ok(()) }
    else { Err("E_COMPOSE_BLOB: original identity differs".to_string()) }
}

/// Error text is the original suffix; the caller only renders the endpoint and input ID.
pub fn admit_imported_input(absent: bool, presence: Option<&str>,
    digest_equal: bool) -> (out: Result<(), String>)
    ensures out.is_ok() == (if absent { optional_text_is(presence, "absent"@) }
            else { !optional_text_is(presence, "absent"@) && digest_equal }),
        match out { Err(error) => error@ == (if absent { "unexpected absence"@ }
            else { "digest/presence mismatch"@ }), _ => true, },
{
    if absent {
        if text_is(presence, "absent") { Ok(()) } else { Err("unexpected absence".to_string()) }
    } else if text_is(presence, "absent") || !digest_equal {
        Err("digest/presence mismatch".to_string())
    } else { Ok(()) }
}
pub struct CompositionSources<S, O> {
    pub state: S,
    pub originals: Vec<O>,
    pub seen: BTreeSet<String>,
}

impl<S, O> CompositionSources<S, O> {
    pub fn new(state: S) -> (out: Self)
        ensures out.state == state, out.originals@ == Seq::<O>::empty(),
            out.seen@ == Set::<String>::empty(),
    {
        Self { state, originals: Vec::new(), seen: BTreeSet::new() }
    }
}

pub open spec fn composition_source_step_result<S, X, T, O, H, I, M>(
    initial: CompositionSources<S, O>, row: &X, hash: &H, import: &I, merge: &M,
    out: (CompositionSources<S, O>, Result<(), String>)) -> bool
    where H: Fn(S, &X) -> (S, Result<String, String>),
        I: Fn(S, &X) -> (S, Result<(T, O), String>),
        M: Fn(S, T) -> (S, Result<(), String>),
{
    exists|hashed: S, digest: Result<String, String>|
        #[trigger] hash.ensures((initial.state, row), (hashed, digest))
        && match digest {
            Err(error) => out.0.state == hashed
                && out.0.originals@ == initial.originals@
                && out.0.seen@ == initial.seen@
                && out.1 == Err::<(), String>(error),
            Ok(key) => if initial.seen@.contains(key) {
                out.0.state == hashed && out.0.originals@ == initial.originals@
                    && out.0.seen@ == initial.seen@
                    && match out.1 {
                        Err(error) => error@ == "E_COMPOSE_ORIGINAL: duplicate source"@,
                        _ => false,
                    }
            } else {
                out.0.seen@ == initial.seen@.insert(key)
                    && exists|imported: S, importing: Result<(T, O), String>|
                        #[trigger] import.ensures((hashed, row), (imported, importing))
                        && match importing {
                            Err(error) => out.0.state == imported
                                && out.0.originals@ == initial.originals@
                                && out.1 == Err::<(), String>(error),
                            Ok((part, original)) => out.0.originals@
                                == initial.originals@.push(original)
                                && merge.ensures((imported, part), (out.0.state, out.1)),
                        }
            },
        }
}

/// 按原顺序计算摘要、拒绝重复、导入、记录原件，再合并；保留每阶段首错与状态。
pub fn compose_source_step<S, X, T, O, H, I, M>(
    progress: CompositionSources<S, O>, row: &X, hash: &H, import: &I, merge: &M,
) -> (out: (CompositionSources<S, O>, Result<(), String>))
    where H: Fn(S, &X) -> (S, Result<String, String>),
        I: Fn(S, &X) -> (S, Result<(T, O), String>),
        M: Fn(S, T) -> (S, Result<(), String>),
    requires vstd::laws_cmp::obeys_cmp::<String>(),
        forall|state: S, row: &X| hash.requires((state, row)),
        forall|state: S, row: &X| import.requires((state, row)),
        forall|state: S, part: T| merge.requires((state, part)),
    ensures composition_source_step_result(progress, row, hash, import, merge, out),
{
    let ghost initial = progress;
    let CompositionSources { state, mut originals, mut seen } = progress;
    let (state, digest) = hash(state, row);
    let ghost hashed = state;
    let ghost hashing = digest;
    assert(hash.ensures((initial.state, row), (hashed, hashing)));
    let key = match digest {
        Err(error) => return (CompositionSources { state, originals, seen }, Err(error)),
        Ok(key) => key,
    };
    let ghost identity = key;
    if !seen.insert(key) {
        assert(seen@ =~= initial.seen@);
        return (CompositionSources { state, originals, seen },
            Err("E_COMPOSE_ORIGINAL: duplicate source".to_string()));
    }
    let (state, importing) = import(state, row);
    let ghost imported = state;
    let ghost import_result = importing;
    assert(import.ensures((hashed, row), (imported, import_result)));
    let (part, original) = match importing {
        Err(error) => return (CompositionSources { state, originals, seen }, Err(error)),
        Ok(value) => value,
    };
    originals.push(original);
    let (state, result) = merge(state, part);
    assert(merge.ensures((imported, part), (state, result)));
    (CompositionSources { state, originals, seen }, result)
}

pub open spec fn composition_sources_prefix<S, X, T, O, H, I, M>(
    rows: Seq<X>, states: Seq<CompositionSources<S, O>>,
    initial: CompositionSources<S, O>, hash: &H, import: &I, merge: &M) -> bool
    where H: Fn(S, &X) -> (S, Result<String, String>),
        I: Fn(S, &X) -> (S, Result<(T, O), String>),
        M: Fn(S, T) -> (S, Result<(), String>),
{
    0 < states.len() <= rows.len() + 1 && states[0] == initial
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] composition_source_step_result(states[i], &rows[i], hash, import, merge,
                (states[i + 1], Ok::<(), String>(())))
}

/// 完整遍历实际来源库存；只在全部来源成功后返回成功，首错不消费后续来源。
pub fn compose_sources<S, X, T, O, H, I, M>(
    progress: CompositionSources<S, O>, rows: &[X], hash: &H, import: &I, merge: &M,
) -> (out: (CompositionSources<S, O>, Result<(), String>))
    where H: Fn(S, &X) -> (S, Result<String, String>),
        I: Fn(S, &X) -> (S, Result<(T, O), String>),
        M: Fn(S, T) -> (S, Result<(), String>),
    requires vstd::laws_cmp::obeys_cmp::<String>(),
        forall|state: S, row: &X| hash.requires((state, row)),
        forall|state: S, row: &X| import.requires((state, row)),
        forall|state: S, part: T| merge.requires((state, part)),
    ensures exists|states: Seq<CompositionSources<S, O>>|
        #[trigger] composition_sources_prefix(rows@, states, progress, hash, import, merge)
        && match out.1 {
            Ok(()) => states.len() == rows.len() + 1 && out.0 == states.last(),
            Err(error) => states.len() <= rows.len()
                && composition_source_step_result(states.last(), &rows@[states.len() - 1],
                    hash, import, merge, (out.0, Err::<(), String>(error))),
        },
{
    let ghost initial = progress;
    let mut current = progress;
    let ghost states = seq![initial];
    let mut i = 0usize;
    while i < rows.len()
        invariant i <= rows.len(), states.len() == i + 1, states.last() == current,
            composition_sources_prefix(rows@, states, initial, hash, import, merge),
            vstd::laws_cmp::obeys_cmp::<String>(),
            forall|state: S, row: &X| hash.requires((state, row)),
            forall|state: S, row: &X| import.requires((state, row)),
            forall|state: S, part: T| merge.requires((state, part)),
        decreases rows.len() - i,
    {
        let ghost before = current;
        let (next, result) = compose_source_step(current, &rows[i], hash, import, merge);
        let ghost original = result;
        assert(composition_source_step_result(before, &rows@[i as int], hash, import, merge,
            (next, original)));
        match result {
            Err(error) => {
                let out = (next, Err(error));
                assert(states.len() <= rows.len());
                assert(composition_sources_prefix(rows@, states, progress, hash, import, merge));
                assert(composition_source_step_result(states.last(), &rows@[states.len() - 1],
                    hash, import, merge, (out.0, Err::<(), String>(error))));
                assert(out.1 == Err::<(), String>(error));
                return out;
            },
            Ok(value) => {
                assert(value == ());
                assert(original == Ok::<(), String>(value));
                assert(original == Ok::<(), String>(()));
                assert(before == states.last());
                assert(composition_source_step_result(states.last(), &rows@[i as int],
                    hash, import, merge, (next, Ok::<(), String>(()))) );
                proof {
                    let previous = states;
                    states = states.push(next);
                    assert forall|j: int| 0 <= j < states.len() - 1 implies
                        #[trigger] composition_source_step_result(states[j], &rows@[j],
                            hash, import, merge, (states[j + 1], Ok::<(), String>(()))) by {
                        if j < previous.len() - 1 {
                            assert(composition_source_step_result(previous[j], &rows@[j],
                                hash, import, merge, (previous[j + 1], Ok::<(), String>(()))) );
                        }
                    }
                }
                current = next;
            },
        }
        i += 1;
    }
    (current, Ok(()))
}

}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub pair: Original,
    #[serde(default)]
    pub transport: Option<ArtifactTransport>,
    #[serde(default)]
    pub artifacts: Option<Value>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub sources: Vec<Source>,
}
#[cfg(not(verus_keep_ghost))]
fn bounded(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    let p = no_symlink_parents(root, path)?;
    admit_json_original(fs::metadata(&p).map_err(|e| e.to_string())?.len())?;
    fs::read(p).map_err(|e| e.to_string())
}
#[cfg(not(verus_keep_ghost))]
fn source_bytes(root: &Path, source: &Source) -> Result<Vec<u8>, String> {
    let raw = if let Some(a) = &source.artifacts {
        retained_artifacts::bytes(root, a, &source.pair.path, source.transport.as_ref())?
    } else {
        return chrono_harness::prepared::read_original(
            root,
            &source.pair,
            source.transport.as_ref(),
        );
    };
    admit_pair_digest(&sha256(&raw), &source.pair.sha256)?;
    Ok(raw)
}
#[cfg(not(verus_keep_ghost))]
fn import_blob(root: &Path, f: &Value, source: &Source) -> Result<Value, String> {
    let (path, digest, length) = match retained_shape(f)? {
        Retained::Blob {
            path,
            digest,
            length,
        } => (path, digest, length),
        Retained::Absent => return Ok(f.clone()),
        Retained::Bytes(_) => {
            return Err("E_COMPOSE_ORIGINAL: capture blob reference required".into());
        }
    };
    let Some(a) = &source.artifacts else {
        let original = Original {
            path: path.into(),
            sha256: digest.into(),
        };
        let p = chrono_harness::prepared::original_path(&original, source.transport.as_ref())?;
        let observed = store(root, &no_symlink_parents(root, &p)?)?;
        admit_imported_blob(
            observed["sha256"].as_str(),
            observed["length"].as_u64(),
            digest,
            length,
        )?;
        return Ok(observed);
    };
    let dir = state(root, BLOB_DIRECTORY)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    let observed = retained_artifacts::copy_to(root, a, path, source.transport.as_ref(), &mut tmp)?;
    admit_imported_blob(Some(&observed.0), Some(observed.1), digest, length)?;
    publish_blob(root, tmp, observed.0, observed.1)
}
#[cfg(not(verus_keep_ghost))]
fn manifest(root: &Path, path: &str) -> Result<(Vec<u8>, Manifest), String> {
    let raw = bounded(root, path)?;
    let manifest: Manifest = chrono_harness::decode(&raw)?;
    admit_manifest(&manifest.schema, manifest.sources.len())?;
    Ok((raw, manifest))
}
/// Import addressed capture data, never execute its historical producer/judges.
/// A source is an original snapshot at either endpoint or an exact endpoint pair.
#[cfg(not(verus_keep_ghost))]
fn import_source(
    root: &Path,
    source: &Source,
    config: &str,
    base: &str,
    candidate: &str,
    old: &Registrations,
    new: &Registrations,
) -> Result<(Value, Value), String> {
    let bytes = source_bytes(root, source)?;
    let v = json(&bytes)?;
    let single = v.get("schema").is_some();
    if single {
        snapshot_shape(&v)?;
        admit_single_endpoint(v["commit"].as_str(), base, candidate)?;
    } else {
        admit_pair_fields(
            v.as_object().map(|m| m.len()),
            v.get("base").is_some(),
            v.get("candidate").is_some(),
        )?;
    }
    let retained = chrono_harness::prepared::retain_original(
        root,
        ".chrono-harness/state/inputs/originals/",
        "pair",
        &bytes,
    )?;
    let original = value!({"source":source.pair,"retained":retained,"transport":source.transport});
    let mut pair = value!({});
    for (name, commit, r) in [("base", base, old), ("candidate", candidate, new)] {
        if single && v["commit"] != commit {
            continue;
        }
        let snapshot = if single { &v } else { &v[name] };
        if !single {
            snapshot_shape(snapshot)?;
        }
        admit_snapshot_identity(
            snapshot["schema"].as_str(),
            snapshot["commit"].as_str(),
            snapshot["config_path"].as_str(),
            snapshot_schema_for(r),
            commit,
            config,
        )
        .map_err(|error| format!("E_COMPOSE_HEADER: {name} {error}"))?;
        admit_snapshot_header(
            &SnapshotHeader {
                digest: snapshot["config_digest"].as_str(),
                selection_present: snapshot.get("selection").is_some(),
                effective_present: snapshot.get("effective_config_path").is_some(),
                effective: snapshot["effective_config_path"].as_str(),
            },
            &wire::digest(r.config())?,
            r.selection().is_some(),
            Some(&snapshot["selection"]) == r.selection(),
            r.effective_config_path(),
        )
        .map_err(|error| format!("E_COMPOSE_HEADER: {name} {error}"))?;
        chrono_judge_registration::inputs::environment(r.config(), &snapshot["environment"])?;
        pair[name] = snapshot.clone();
        for (id, file) in snapshot["files"].as_object().ok_or("snapshot files")? {
            let declared = r.config()["environment"]["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["id"] == *id)
                .ok_or_else(|| format!("E_COMPOSE_INPUT: {name} unregistered {id}"))?;
            let observed = import_blob(root, file, source)?;
            admit_imported_input(
                observed["absent"] == true,
                declared["presence"].as_str(),
                observed["sha256"] == declared["sha256"],
            )
            .map_err(|error| format!("E_COMPOSE_INPUT: {name} {error} {id}"))?;
            pair[name]["files"][id] = observed;
        }
    }
    Ok((pair, original))
}

/// No live business inputs, SDK probes or inferred dependencies are consulted.
#[cfg(not(verus_keep_ghost))]
pub fn compose(
    root: &Path,
    config: &str,
    base: &str,
    candidate: &str,
    manifest_path: &str,
    output: &str,
    receipt: &str,
) -> Result<Value, String> {
    state(root, output)?;
    state(root, receipt)?;
    admit_output_paths(output, receipt)?;
    let (raw, manifest) = manifest(root, manifest_path)?;
    let reader = facts::Reader::for_config(root, config)?;
    let old = Registrations::load(&reader.registry_values(root, base, config)?, config)?;
    let new = Registrations::load(&reader.registry_values(root, candidate, config)?, config)?;
    let selected = if chrono_harness::units::full_execution_units(new.config())?.is_some() {
        let delta = facts::delta(&reader.tree(root, base)?, &reader.tree(root, candidate)?);
        let (impact, findings) = chrono_judge_filemap::produce_for_endpoints(
            root, base, candidate, &delta, &old, &new, &reader,
        )?;
        if findings.iter().any(|f| f.level == "error") {
            return Err(format!(
                "E_COMPOSE_SELECTION: {}",
                serde_json::to_string(&findings).map_err(|e| e.to_string())?
            ));
        }
        Some(chrono_judge_routes::global_selection(
            &old,
            &new,
            &impact,
            chrono_judge_registration::downstream_validator(&new, "routes"),
        )?)
    } else {
        None
    };
    let (composition, composed) = compose_sources(
        CompositionSources::new(value!({})),
        &manifest.sources,
        &|pair, source| {
            let digest = serde_json::to_value(source)
                .map_err(|error| error.to_string())
                .and_then(|value| wire::digest(&value));
            (pair, digest)
        },
        &|pair, source| {
            (
                pair,
                import_source(root, source, config, base, candidate, &old, &new),
            )
        },
        &|mut pair, part: Value| {
            let merged = (|| {
                for name in ["base", "candidate"] {
                    let Some(snapshot) = part.get(name) else {
                        continue;
                    };
                    if pair.get(name).is_none() {
                        pair[name] = snapshot.clone();
                        pair[name]["files"] = value!({});
                    }
                    if pair[name]["environment"] != snapshot["environment"] {
                        return Err(format!(
                            "E_COMPOSE_ENVIRONMENT: {name} conflicting originals"
                        ));
                    }
                    for (id, file) in snapshot["files"].as_object().ok_or("snapshot files")? {
                        let observed = file.clone();
                        if pair[name]["files"]
                            .get(id)
                            .is_some_and(|prior| prior != &observed)
                        {
                            return Err(format!(
                                "E_COMPOSE_CONFLICT: {name} overlapping input {id}"
                            ));
                        }
                        pair[name]["files"][id] = observed;
                    }
                }
                Ok(())
            })();
            (pair, merged)
        },
    );
    composed?;
    let pair = composition.state;
    let originals = composition.originals;
    for (name, r) in [("base", &old), ("candidate", &new)] {
        if pair.get(name).is_none() {
            return Err(format!(
                "E_COMPOSE_COVERAGE: {name} missing endpoint original"
            ));
        }
        let required = if let Some(selected) = &selected {
            chrono_judge_registration::inputs::collection_required_files(
                name, &old, &new, selected,
            )?
        } else {
            chrono_judge_registration::inputs::required_files(r, None)?
        };
        for id in required {
            if pair[name]["files"].get(&id).is_none() {
                return Err(format!("E_COMPOSE_COVERAGE: {name} missing input {id}"));
            }
        }
    }
    publish(root, output, &pair)?;
    let provenance = value!({"schema":"chrono-input-composition-result/v1","manifest":{"path":manifest_path,"sha256":sha256(&raw)},"base":base,"candidate":candidate,"config_path":config,"output":{"path":output,"sha256":sha256(&wire::canonical(&pair)?)},"originals":originals,"completeness_proven":false});
    publish(root, receipt, &provenance)?;
    Ok(pair)
}
