//! Snapshot transport, not dependency discovery or a governance verdict.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub mod identifiers;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
#[derive(Clone, Copy)]
pub enum InputKind { Capture, Compose, Pair }

pub enum InputAction<'a> {
    Version,
    Help,
    Command(InputCommand<'a>),
}

/// The validated command borrows its original argv; no second option inventory is built.
pub struct InputCommand<'a> {
    pub kind: InputKind,
    pub arguments: &'a [String],
}

pub struct MissingInputOption<'a> {
    pub key: &'a str,
}

pub open spec fn input_kind(text: Seq<char>) -> Option<InputKind> {
    if text == "capture"@ { Some(InputKind::Capture) }
    else if text == "compose"@ { Some(InputKind::Compose) }
    else if text == "pair"@ { Some(InputKind::Pair) }
    else { None }
}

pub open spec fn input_key_allowed(kind: InputKind, key: Seq<char>) -> bool {
    key == "--host-root"@ || key == "--output"@ || match kind {
        InputKind::Capture => key == "--config"@ || key == "--commit"@ || key == "--unit"@,
        InputKind::Compose => key == "--config"@ || key == "--base"@ || key == "--candidate"@
            || key == "--manifest"@ || key == "--receipt"@,
        InputKind::Pair => key == "--base-snapshot"@ || key == "--candidate-snapshot"@
            || key == "--base-root"@ || key == "--candidate-root"@,
    }
}

fn allowed_input_key(kind: InputKind, key: &str) -> (allowed: bool)
    ensures allowed == input_key_allowed(kind, key@),
{
    identifiers::equal_text(key, "--host-root") || identifiers::equal_text(key, "--output")
        || match kind {
            InputKind::Capture => identifiers::equal_text(key, "--config")
                || identifiers::equal_text(key, "--commit") || identifiers::equal_text(key, "--unit"),
            InputKind::Compose => identifiers::equal_text(key, "--config")
                || identifiers::equal_text(key, "--base") || identifiers::equal_text(key, "--candidate")
                || identifiers::equal_text(key, "--manifest") || identifiers::equal_text(key, "--receipt"),
            InputKind::Pair => identifiers::equal_text(key, "--base-snapshot")
                || identifiers::equal_text(key, "--candidate-snapshot")
                || identifiers::equal_text(key, "--base-root") || identifiers::equal_text(key, "--candidate-root"),
        }
}

pub open spec fn input_prefix_valid(args: Seq<String>, kind: InputKind, pairs: int) -> bool {
    0 <= pairs <= (args.len() - 1) / 2 && forall|i: int| 0 <= i < pairs ==>
        input_key_allowed(kind, (#[trigger] args[1 + 2 * i])@)
        && forall|j: int| 0 <= j < i ==> (#[trigger] args[1 + 2 * j])@ != args[1 + 2 * i]@
}

pub open spec fn input_command_valid(command: InputCommand) -> bool {
    let args = command.arguments@;
    args.len() > 0 && args.len() % 2 == 1 && input_kind(args[0]@) == Some(command.kind)
        && input_prefix_valid(args, command.kind, (args.len() - 1) / 2)
}

pub open spec fn input_command_result(args: Seq<String>) -> Option<Seq<char>> {
    if args.len() == 0 { Some("E_USAGE: capture or pair required"@) }
    else if args.len() == 1 && (args[0]@ == "--version"@ || args[0]@ == "--help"@) { None }
    else { match input_kind(args[0]@) {
        None => Some("E_USAGE: unknown command"@),
        Some(kind) => if !input_prefix_valid(args, kind, (args.len() - 1) / 2) {
            Some("E_USAGE: unknown or duplicate option"@)
        } else if args.len() % 2 == 0 { Some("E_USAGE: option needs a value"@) }
        else { None },
    } }
}

/// Return the first matching complete key/value pair from the specified original prefix.
fn find_input_option(args: &[String], pairs: usize, key: &str) -> (found: Option<usize>)
    requires args.len() > 0, pairs <= (args.len() - 1) / 2,
    ensures match found {
        Some(i) => i < pairs && args@[1 + 2 * i]@ == key@
            && forall|j: int| 0 <= j < i ==> (#[trigger] args@[1 + 2 * j])@ != key@,
        None => forall|j: int| 0 <= j < pairs ==> (#[trigger] args@[1 + 2 * j])@ != key@,
    },
{
    let mut i = 0;
    while i < pairs
        invariant args.len() > 0, pairs <= (args.len() - 1) / 2, i <= pairs,
            forall|j: int| 0 <= j < i ==> (#[trigger] args@[1 + 2 * j])@ != key@,
        decreases pairs - i,
    {
        if identifiers::equal_text(args[1 + 2 * i].as_str(), key) { return Some(i); }
        i += 1;
    }
    None
}

impl<'a> InputCommand<'a> {
    pub fn option(&self, key: &str) -> (value: Option<&'a str>)
        requires input_command_valid(*self),
        ensures match value {
            Some(value) => exists|i: int| 0 <= i < (self.arguments@.len() - 1) / 2
                && (#[trigger] self.arguments@[1 + 2 * i])@ == key@ && value@ == self.arguments@[2 + 2 * i]@,
            None => forall|i: int| 0 <= i < (self.arguments@.len() - 1) / 2
                ==> (#[trigger] self.arguments@[1 + 2 * i])@ != key@,
        },
    {
        match find_input_option(self.arguments, (self.arguments.len() - 1) / 2, key) {
            Some(i) => Some(self.arguments[2 + 2 * i].as_str()),
            None => None,
        }
    }

    pub fn required<'k>(&self, key: &'k str) -> (value: Result<&'a str, MissingInputOption<'k>>)
        requires input_command_valid(*self),
        ensures match value {
            Ok(value) => exists|i: int| 0 <= i < (self.arguments@.len() - 1) / 2
                && (#[trigger] self.arguments@[1 + 2 * i])@ == key@ && value@ == self.arguments@[2 + 2 * i]@,
            Err(error) => error.key@ == key@
                && forall|i: int| 0 <= i < (self.arguments@.len() - 1) / 2
                    ==> (#[trigger] self.arguments@[1 + 2 * i])@ != key@,
        },
    {
        match self.option(key) {
            Some(value) => Ok(value),
            None => Err(MissingInputOption { key }),
        }
    }
}

pub fn parse_input_command(args: &[String]) -> (out: Result<InputAction<'_>, String>)
    ensures match out {
        Err(error) => input_command_result(args@) == Some(error@),
        Ok(action) => input_command_result(args@).is_none() && match action {
            InputAction::Version => args@.len() == 1 && args@[0]@ == "--version"@,
            InputAction::Help => args@.len() == 1 && args@[0]@ == "--help"@,
            InputAction::Command(command) => input_command_valid(command) && command.arguments@ == args@,
        },
    },
{
    if args.len() == 0 { return Err("E_USAGE: capture or pair required".to_string()); }
    let first = args[0].as_str();
    if args.len() == 1 && identifiers::equal_text(first, "--version") { return Ok(InputAction::Version); }
    if args.len() == 1 && identifiers::equal_text(first, "--help") { return Ok(InputAction::Help); }
    let kind = if identifiers::equal_text(first, "capture") { InputKind::Capture }
        else if identifiers::equal_text(first, "compose") { InputKind::Compose }
        else if identifiers::equal_text(first, "pair") { InputKind::Pair }
        else { return Err("E_USAGE: unknown command".to_string()); };
    let pairs = (args.len() - 1) / 2;
    let mut i = 0;
    while i < pairs
        invariant args.len() > 0, pairs == (args.len() - 1) / 2, i <= pairs,
            input_kind(args@[0]@) == Some(kind), input_prefix_valid(args@, kind, i as int),
        decreases pairs - i,
    {
        let key = args[1 + 2 * i].as_str();
        if !allowed_input_key(kind, key) || find_input_option(args, i, key).is_some() {
            assert(!input_prefix_valid(args@, kind, pairs as int));
            return Err("E_USAGE: unknown or duplicate option".to_string());
        }
        proof {
            assert forall|j: int| 0 <= j < i + 1 implies
                input_key_allowed(kind, (#[trigger] args@[1 + 2 * j])@)
                    && forall|k: int| 0 <= k < j ==> (#[trigger] args@[1 + 2 * k])@ != args@[1 + 2 * j]@ by {
                if j < i { assert(input_key_allowed(kind, args@[1 + 2 * j]@)); }
            }
        }
        i += 1;
    }
    if args.len() % 2 == 0 { return Err("E_USAGE: option needs a value".to_string()); }
    Ok(InputAction::Command(InputCommand { kind, arguments: args }))
}

/// All current capture and composition paths use the same exact blob identity.
pub fn blob_identity_matches(actual: &str, actual_length: u64,
    expected: &str, expected_length: u64) -> (matches: bool)
    ensures matches == (actual@ == expected@ && actual_length == expected_length),
{
    identifiers::equal_text(actual, expected) && actual_length == expected_length
}
}

pub mod composition;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{copy_hashed, facts, file_identity, json, no_symlink_parents, wire};
#[cfg(not(verus_keep_ghost))]
pub use chrono_judge_registration::inputs::{
    SNAPSHOT_SCHEMA, SNAPSHOT_SCHEMA_V2, SNAPSHOT_SCHEMA_V3,
};
#[cfg(not(verus_keep_ghost))]
use chrono_judge_registration::{
    Registrations,
    inputs::{Retained, observe_file, retained_shape, snapshot_schema_for, snapshot_shape},
};
#[cfg(not(verus_keep_ghost))]
pub use composition::{Manifest as CompositionManifest, Source as CompositionSource, compose};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json as value};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
#[cfg(not(verus_keep_ghost))]
const BLOB_DIRECTORY: &str = ".chrono-harness/state/inputs/blobs";

#[cfg(not(verus_keep_ghost))]
fn state(root: &Path, path: &str) -> Result<PathBuf, String> {
    if !chrono_harness::verified_paths::valid_host_state_path(path.as_bytes()) {
        return Err("E_INPUT_PATH: output/reference must be within host state".into());
    }
    no_symlink_parents(root, path)
}
#[cfg(not(verus_keep_ghost))]
fn publish(root: &Path, path: &str, v: &Value) -> Result<(), String> {
    let target = state(root, path)?;
    let bytes = wire::canonical(v)?;
    if target.exists() {
        if fs::read(&target).map_err(|e| e.to_string())? == bytes {
            return Ok(());
        }
        return Err("E_SNAPSHOT_EXISTS: retain old snapshot; choose a new output path".into());
    }
    let parent = target.parent().ok_or("snapshot parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    tmp.write_all(&bytes).map_err(|e| e.to_string())?;
    tmp.persist_noclobber(&target).map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
fn store(root: &Path, input: &Path) -> Result<Value, String> {
    let dir = state(root, BLOB_DIRECTORY)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if !fs::metadata(input)
        .map_err(|e| format!("E_INPUT_READ: {}: {e}", input.display()))?
        .is_file()
    {
        return Err("E_INPUT_READ: not a regular file".into());
    }
    let file =
        fs::File::open(input).map_err(|e| format!("E_INPUT_READ: {}: {e}", input.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("E_INPUT_READ: not a regular file".into());
    }
    let mut tmp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    let (sha256, length) = copy_hashed(file, &mut tmp)?;
    publish_blob(root, tmp, sha256, length)
}
// The caller hashes all bytes written to its private temporary file before publication.
#[cfg(not(verus_keep_ghost))]
fn publish_blob(
    root: &Path,
    tmp: tempfile::NamedTempFile,
    sha256: String,
    length: u64,
) -> Result<Value, String> {
    let path = format!("{BLOB_DIRECTORY}/{sha256}");
    let target = state(root, &path)?;
    if target.exists() {
        let observed = file_identity(&target)?;
        if !blob_identity_matches(&observed.0, observed.1, &sha256, length) {
            return Err("E_INPUT_BLOB: existing content does not match its address".into());
        }
    } else if let Err(error) = tmp.persist_noclobber(&target) {
        if error.error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(format!("E_INPUT_BLOB: {error}"));
        }
        let observed = file_identity(&target)?;
        if !blob_identity_matches(&observed.0, observed.1, &sha256, length) {
            return Err(format!("E_INPUT_BLOB: {error}"));
        }
    }
    Ok(value!({"blob":path,"sha256":sha256,"length":length}))
}
#[cfg(not(verus_keep_ghost))]
pub fn capture(root: &Path, config: &str, commit: &str, output: &str) -> Result<Value, String> {
    capture_selected(
        root,
        config,
        commit,
        output,
        &chrono_harness::prepared::Selection::All,
    )
}
#[cfg(not(verus_keep_ghost))]
pub fn capture_selected(
    root: &Path,
    config: &str,
    commit: &str,
    output: &str,
    selection: &chrono_harness::prepared::Selection,
) -> Result<Value, String> {
    use chrono_harness::{prepared::Selection, units::Scope};
    let scope = match selection {
        Selection::All => None,
        Selection::Unit { unit } => Some(Scope::Unit { unit: unit.clone() }),
        Selection::Collect => return Err("E_INPUT_SCOPE: collection consumes original snapshots; it does not capture live inputs".into()),
    };
    state(root, output)?;
    let reader = facts::Reader::for_config(root, config)?;
    reader.verify_config(root, commit)?;
    reader.verify_oid(root, commit)?;
    if facts::utf8(reader.git(root, &["rev-parse", "HEAD"])?)?.trim() != commit {
        return Err("E_SNAPSHOT_IDENTITY: capture requires the explicit checked-out commit".into());
    }
    let bytes = reader.blob(root, commit, config)?;
    if fs::read(no_symlink_parents(root, config)?).map_err(|e| e.to_string())? != bytes {
        return Err("E_SNAPSHOT_IDENTITY: config differs from the fixed commit".into());
    }
    let values = reader.registry_values(root, commit, config)?;
    let r = Registrations::load(&values, config)?;
    let required = chrono_judge_registration::inputs::required_files(&r, scope.as_ref())?;
    let credentials = if r.config()["schema_version"] == 4 {
        chrono_harness::prepared::credential_environment(r.config())?
    } else {
        Default::default()
    };
    let mut environment = serde_json::Map::new();
    for key in r.config()["environment"]["inherit"].as_array().unwrap() {
        let key = key.as_str().ok_or("invalid inherited variable")?;
        let v = if credentials.contains(key) {
            Value::Null
        } else {
            match std::env::var(key) {
                Ok(v) => Value::String(v),
                Err(std::env::VarError::NotPresent) => Value::Null,
                Err(_) => return Err(format!("E_INPUT_ENVIRONMENT: non-UTF8 value for {key}")),
            }
        };
        environment.insert(key.into(), v);
    }
    let mut files = serde_json::Map::new();
    for input in r.config()["environment"]["inputs"].as_array().unwrap() {
        if !required.contains(input["id"].as_str().unwrap()) {
            continue;
        }
        let path = root.join(input["location"].as_str().unwrap());
        let retained = if matches!(r.config()["schema_version"].as_u64(), Some(2 | 3 | 4)) {
            match observe_file(&path)? {
                None => value!({"absent":true}),
                Some(identity) => {
                    let observed = store(root, &path)?;
                    if observed["sha256"] != identity.0 || observed["length"] != identity.1 {
                        return Err("E_INPUT_READ: input changed during capture".into());
                    }
                    observed
                }
            }
        } else {
            store(root, &path)?
        };
        files.insert(input["id"].as_str().unwrap().into(), retained);
    }
    let mut snapshot = value!({"schema":snapshot_schema_for(&r),"commit":commit,"config_path":config,"config_digest":wire::digest(r.config())?,"environment":environment,"files":files});
    if let Some(selection) = r.selection() {
        snapshot["effective_config_path"] = value!(r.effective_config_path());
        snapshot["selection"] = selection.clone();
    }
    publish(root, output, &snapshot)?;
    Ok(snapshot)
}

#[cfg(not(verus_keep_ghost))]
fn transport(from: &Path, snapshot: &str, to: &Path) -> Result<Value, String> {
    let _snapshot = chrono_harness::retained_artifacts::retention::read_guard(from, snapshot)?;
    let bytes = fs::read(state(from, snapshot)?).map_err(|e| e.to_string())?;
    let mut v = json(&bytes)?;
    snapshot_shape(&v)?;
    let files = v["files"]
        .as_object_mut()
        .ok_or("E_INPUT_SNAPSHOT: missing files")?;
    for f in files.values_mut() {
        let blob = match retained_shape(f)? {
            Retained::Absent => continue,
            Retained::Blob { path, .. } => path,
            Retained::Bytes(_) => {
                return Err("E_INPUT_BLOB: pair requires captured blob references".into());
            }
        };
        let _blob = chrono_harness::retained_artifacts::retention::read_guard(from, blob)?;
        let path = state(from, blob)?;
        let observed = store(to, &path)?;
        if observed["sha256"] != f["sha256"] || observed["length"] != f["length"] {
            return Err("E_INPUT_BLOB: retained bytes differ from snapshot".into());
        }
        *f = observed;
    }
    Ok(v)
}
#[cfg(not(verus_keep_ghost))]
pub fn pair(
    root: &Path,
    base_root: &Path,
    base: &str,
    candidate_root: &Path,
    candidate: &str,
    output: &str,
) -> Result<Value, String> {
    // Source capabilities survive both transports and the destination handoff.
    let _base = chrono_harness::retained_artifacts::retention::read_scope(base_root)?;
    let _candidate = chrono_harness::retained_artifacts::retention::read_scope(candidate_root)?;
    state(root, output)?;
    let a = transport(base_root, base, root)?;
    let b = transport(candidate_root, candidate, root)?;
    let v = value!({"base":a,"candidate":b});
    publish(root, output, &v)?;
    Ok(v)
}
#[cfg(not(verus_keep_ghost))]
pub fn run(cwd: &Path, args: &[String]) -> Result<String, String> {
    let command = match parse_input_command(args)? {
        InputAction::Version => return Ok(format!("chrono-inputs {}\n", env!("CARGO_PKG_VERSION"))),
        InputAction::Help => return Ok("chrono-inputs capture --host-root H --config P --commit OID --output P [--unit ID]\nchrono-inputs pair --host-root H --base-snapshot P --candidate-snapshot P --output P [--base-root H] [--candidate-root H]\nchrono-inputs compose --host-root H --config P --base OID --candidate OID --manifest P --output P --receipt P\nExplicit input transport only; snapshots are not governance verdicts.\n".into()),
        InputAction::Command(command) => command,
    };
    let required = |key: &str| {
        command
            .required(key)
            .map_err(|missing| format!("E_USAGE: missing {}", missing.key))
    };
    let root = fs::canonicalize(cwd.join(required("--host-root")?)).map_err(|e| e.to_string())?;
    let result = match command.kind {
        InputKind::Capture => capture_selected(
            &root,
            required("--config")?,
            required("--commit")?,
            required("--output")?,
            &command
                .option("--unit")
                .map_or(chrono_harness::prepared::Selection::All, |unit| {
                    chrono_harness::prepared::Selection::Unit { unit: unit.into() }
                }),
        )?,
        InputKind::Compose => compose(
            &root,
            required("--config")?,
            required("--base")?,
            required("--candidate")?,
            required("--manifest")?,
            required("--output")?,
            required("--receipt")?,
        )?,
        InputKind::Pair => {
            let location = |key: &str| -> Result<PathBuf, String> {
                match command.option(key) {
                    Some(p) => fs::canonicalize(cwd.join(p)).map_err(|e| e.to_string()),
                    None => Ok(root.clone()),
                }
            };
            pair(
                &root,
                &location("--base-root")?,
                required("--base-snapshot")?,
                &location("--candidate-root")?,
                required("--candidate-snapshot")?,
                required("--output")?,
            )?
        }
    };
    Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())? + "\n")
}
