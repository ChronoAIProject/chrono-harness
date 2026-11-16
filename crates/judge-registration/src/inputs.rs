//! Completeness declarations and retained endpoint bytes; a digest is not a closure proof.
use super::schema::{self, ObjectFieldError, object_error_parts, validate_object_fields};
#[cfg(verus_keep_ghost)]
use super::{retained_paths as paths, source_verifier::identifiers};
#[cfg(not(verus_keep_ghost))]
use crate::Registrations;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{
    facts, file_identity, no_symlink_parents, relative_path, sha256,
    wire::{self, Request},
};
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verified_paths as paths, verus};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{collections::BTreeMap, path::Path};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[cfg(not(verus_keep_ghost))]
pub fn snapshot_schema(config: &Value) -> &'static str {
    select_snapshot_schema(config["schema_version"].as_u64(), false)
}
#[cfg(not(verus_keep_ghost))]
pub fn snapshot_schema_for(r: &Registrations) -> &'static str {
    select_snapshot_schema(
        r.config()["schema_version"].as_u64(),
        r.selection().is_some(),
    )
}

#[cfg(not(verus_keep_ghost))]
pub use chrono_harness::input_file::observe_file;

verus! {

pub const SNAPSHOT_SCHEMA: &'static str = "chrono-input-snapshot/v1";
pub const SNAPSHOT_SCHEMA_V2: &'static str = "chrono-input-snapshot/v2";
pub const SNAPSHOT_SCHEMA_V3: &'static str = "chrono-input-snapshot/v3";

pub open spec fn snapshot_format_version(schema: Option<&str>) -> int {
    match schema {
        Some(value) => {
            if value@ == "chrono-input-snapshot/v1"@ { 1 }
            else if value@ == "chrono-input-snapshot/v2"@ { 2 }
            else if value@ == "chrono-input-snapshot/v3"@ { 3 }
            else { 0 }
        },
        None => 0,
    }
}

fn classify_snapshot_schema(schema: Option<&str>) -> (version: u8)
    ensures version as int == snapshot_format_version(schema),
{
    match schema {
        Some(value) => {
            if identifiers::equal_text(value, SNAPSHOT_SCHEMA) { 1 }
            else if identifiers::equal_text(value, SNAPSHOT_SCHEMA_V2) { 2 }
            else if identifiers::equal_text(value, SNAPSHOT_SCHEMA_V3) { 3 }
            else { 0 }
        },
        None => 0,
    }
}

/// 只有完整 v3 schema 才选择 selector 内容和端点绑定分支。
pub fn snapshot_uses_selector(schema: Option<&str>) -> (selected: bool)
    ensures selected == match schema {
        Some(text) => text@ == "chrono-input-snapshot/v3"@,
        None => false,
    },
{
    match schema {
        Some(text) => identifiers::equal_text(text, SNAPSHOT_SCHEMA_V3),
        None => false,
    }
}

/// 先拒绝未知 schema；只有 v3 可以携带 selector 字段。
pub fn admit_snapshot_header(schema: Option<&str>, effective: bool, selection: bool)
    -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => snapshot_format_version(schema) != 0
            && (snapshot_format_version(schema) == 3 || (!effective && !selection)),
        Err(message) => {
            (snapshot_format_version(schema) == 0
                && message@ == "E_INPUT_SNAPSHOT: unsupported snapshot schema"@)
            || (snapshot_format_version(schema) != 0 && snapshot_format_version(schema) != 3
                && (effective || selection)
                && message@ == "E_INPUT_SNAPSHOT: selection binding requires snapshot v3"@)
        },
    },
{
    let version = classify_snapshot_schema(schema);
    if version == 0 { return Err("E_INPUT_SNAPSHOT: unsupported snapshot schema"); }
    if version != 3 && (effective || selection) {
        return Err("E_INPUT_SNAPSHOT: selection binding requires snapshot v3");
    }
    Ok(())
}

pub open spec fn snapshot_selector_binding_valid(
    schema: Option<&str>, platform: Option<(&str, &str)>,
    entry_matches: bool, selected_matches: bool, self_link: bool,
) -> bool {
    (match schema { Some(text) => text@ == "chrono-registry-selection/v1"@, None => false })
        && (match platform {
            Some((text, trimmed)) => text@.len() > 0 && text@ == trimmed@,
            None => false,
        }) && entry_matches && selected_matches && !self_link
}

/// JSON 全值相等与 trim 只提供观察，完整准入和原错误由本函数决定。
pub fn admit_snapshot_selector_binding(
    schema: Option<&str>, platform: Option<(&str, &str)>,
    entry_matches: bool, selected_matches: bool, self_link: bool,
) -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => snapshot_selector_binding_valid(schema, platform, entry_matches, selected_matches, self_link),
        Err(message) => !snapshot_selector_binding_valid(schema, platform, entry_matches, selected_matches, self_link)
            && message@ == "E_INPUT_SNAPSHOT: invalid selector binding schema"@,
    },
{
    let schema_ok = match schema {
        Some(text) => identifiers::equal_text(text, "chrono-registry-selection/v1"),
        None => false,
    };
    let platform_ok = match platform {
        Some((text, trimmed)) => !text.is_empty() && identifiers::equal_text(text, trimmed),
        None => false,
    };
    if schema_ok && platform_ok && entry_matches && selected_matches && !self_link {
        Ok(())
    } else { Err("E_INPUT_SNAPSHOT: invalid selector binding schema") }
}

/// 选择器优先使用 v3；直接配置仅在版本 2、3、4 时使用 v2。
pub fn select_snapshot_schema(version: Option<u64>, selected: bool) -> (schema: &'static str)
    ensures schema@ == if selected { "chrono-input-snapshot/v3"@ }
        else if version == Some(2u64) || version == Some(3u64) || version == Some(4u64) {
            "chrono-input-snapshot/v2"@
        } else { "chrono-input-snapshot/v1"@ },
{
    if selected { SNAPSHOT_SCHEMA_V3 }
    else {
        match version {
            Some(2) | Some(3) | Some(4) => SNAPSHOT_SCHEMA_V2,
            _ => SNAPSHOT_SCHEMA,
        }
    }
}

/// Parse the retained representation without reading or repairing its contents.
pub enum Retained<'a, B = Vec<u8>> {
    Absent,
    Bytes(B),
    Blob {
        path: &'a str,
        digest: &'a str,
        length: u64,
    },
}

/// 只检查缺失表示与版本的相容性；其余表示保留原内容校验义务。
pub fn admit_snapshot_retained<B>(schema: Option<&str>, retained: &Retained<B>)
    -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => match *retained {
            Retained::Absent => snapshot_format_version(schema) == 2 || snapshot_format_version(schema) == 3,
            _ => true,
        },
        Err(message) => matches!(*retained, Retained::Absent)
            && snapshot_format_version(schema) != 2 && snapshot_format_version(schema) != 3
            && message@ == "E_INPUT_SNAPSHOT: absence requires snapshot v2"@,
    },
{
    match *retained {
        Retained::Absent => {
            let version = classify_snapshot_schema(schema);
            if version != 2 && version != 3 {
                return Err("E_INPUT_SNAPSHOT: absence requires snapshot v2");
            }
            Ok(())
        },
        _ => Ok(()),
    }
}

/// 同一 JSON 的原观察；缺键与非对应类型分别保留。
pub struct RetainedObservation<'a, B> {
    pub object: bool,
    pub keys: Vec<&'a str>,
    pub absent: Option<Option<bool>>,
    pub bytes: Option<B>,
    pub blob: Option<Option<&'a str>>,
    pub digest: Option<&'a str>,
    pub length: Option<u64>,
}

pub enum RetainedError<'a> {
    Fields(ObjectFieldError<'a>),
    Representation,
    Absent,
    Bytes,
    Path,
    Digest,
    Length,
}

#[verifier::inline]
pub open spec fn retained_mode<B>(observed: RetainedObservation<B>) -> int {
    if observed.absent.is_some() { 0 }
    else if observed.bytes.is_some() && observed.blob.is_none() { 1 }
    else if observed.bytes.is_none() && observed.blob.is_some() { 2 }
    else { 3 }
}

#[verifier::inline]
pub open spec fn retained_required(mode: int) -> Seq<&'static str> {
    if mode == 0 { seq!["absent"] }
    else if mode == 1 { seq!["bytes"] }
    else { seq!["blob", "sha256", "length"] }
}

#[verifier::inline]
pub open spec fn retained_fields_ok<B>(observed: RetainedObservation<B>) -> bool {
    observed.object
        && schema::required_fields_present(retained_required(retained_mode(observed)), observed.keys@)
        && schema::known_fields_only(retained_required(retained_mode(observed)), seq![], observed.keys@)
}

#[verifier::inline]
pub open spec fn retained_field_failure<B>(
    observed: RetainedObservation<B>, error: ObjectFieldError,
) -> bool {
    match error {
        ObjectFieldError::ExpectedObject => !observed.object,
        ObjectFieldError::Missing(name) => observed.object && schema::first_missing_field(
            retained_required(retained_mode(observed)), observed.keys@, name@,
        ),
        ObjectFieldError::Unknown(name) => observed.object
            && schema::required_fields_present(retained_required(retained_mode(observed)), observed.keys@)
            && schema::first_unknown_field(retained_required(retained_mode(observed)), seq![], observed.keys@, name@),
    }
}

pub open spec fn retained_path_valid(path: Option<Option<&str>>) -> bool {
    match path {
        Some(Some(path)) => paths::host_state_path(path.spec_bytes()),
        _ => false,
    }
}

pub open spec fn retained_digest_valid(digest: Option<&str>) -> bool {
    match digest {
        Some(digest) => identifiers::digest_format(digest.spec_bytes()),
        None => false,
    }
}

pub open spec fn retained_selection<B>(
    observed: RetainedObservation<B>, result: Result<Retained<B>, RetainedError>,
) -> bool {
    match result {
        Ok(Retained::Absent) => retained_mode(observed) == 0 && retained_fields_ok(observed)
            && observed.absent == Some(Some(true)),
        Ok(Retained::Bytes(bytes)) => retained_mode(observed) == 1 && retained_fields_ok(observed)
            && observed.bytes == Some(bytes),
        Ok(Retained::Blob { path, digest, length }) => retained_mode(observed) == 2
            && retained_fields_ok(observed) && retained_path_valid(observed.blob)
            && retained_digest_valid(observed.digest)
            && observed.blob == Some(Some(path)) && observed.digest == Some(digest)
            && observed.length == Some(length),
        Err(RetainedError::Representation) => retained_mode(observed) == 3,
        Err(RetainedError::Fields(error)) => retained_mode(observed) != 3
            && retained_field_failure(observed, error),
        Err(RetainedError::Absent) => retained_mode(observed) == 0 && retained_fields_ok(observed)
            && observed.absent != Some(Some(true)),
        Err(RetainedError::Path) => retained_mode(observed) == 2 && retained_fields_ok(observed)
            && !retained_path_valid(observed.blob),
        Err(RetainedError::Digest) => retained_mode(observed) == 2 && retained_fields_ok(observed)
            && retained_path_valid(observed.blob) && !retained_digest_valid(observed.digest),
        Err(RetainedError::Length) => retained_mode(observed) == 2 && retained_fields_ok(observed)
            && retained_path_valid(observed.blob) && retained_digest_valid(observed.digest)
            && observed.length.is_none(),
        Err(RetainedError::Bytes) => false,
    }
}

/// absent 键优先；先结构，再逐项检查；字节解码另由下方同源驱动调用。
pub fn select_retained<'a, B>(observed: RetainedObservation<'a, B>)
    -> (out: Result<Retained<'a, B>, RetainedError<'a>>)
    ensures retained_selection(observed, out),
{
    let object = if observed.object { Some(()) } else { None };
    if observed.absent.is_some() {
        let required = ["absent"];
        proof {
            assert(retained_mode(observed) == 0);
            assert(required@ == retained_required(retained_mode(observed)));
        }
        match validate_object_fields(object, &required, &[], &observed.keys) {
            Err(error) => {
                proof { assert(retained_field_failure(observed, error)); }
                return Err(RetainedError::Fields(error));
            },
            Ok(_) => {},
        }
        return match observed.absent {
            Some(Some(true)) => Ok(Retained::Absent),
            _ => Err(RetainedError::Absent),
        };
    }
    match (observed.bytes, observed.blob) {
        (Some(bytes), None) => {
            let required = ["bytes"];
            proof {
                assert(retained_mode(observed) == 1);
                assert(required@ == retained_required(retained_mode(observed)));
            }
            match validate_object_fields(object, &required, &[], &observed.keys) {
                Err(error) => {
                    proof { assert(retained_field_failure(observed, error)); }
                    return Err(RetainedError::Fields(error));
                },
                Ok(_) => {},
            }
            Ok(Retained::Bytes(bytes))
        },
        (None, Some(blob)) => {
            let required = ["blob", "sha256", "length"];
            proof {
                assert(retained_mode(observed) == 2);
                assert(required@ == retained_required(retained_mode(observed)));
            }
            match validate_object_fields(object, &required, &[], &observed.keys) {
                Err(error) => {
                    proof { assert(retained_field_failure(observed, error)); }
                    return Err(RetainedError::Fields(error));
                },
                Ok(_) => {},
            }
            let path = match blob {
                Some(path) => {
                    if !paths::valid_host_state_path(path.as_bytes()) { return Err(RetainedError::Path); }
                    path
                },
                None => return Err(RetainedError::Path),
            };
            let digest = match observed.digest {
                Some(digest) => {
                    if !identifiers::valid_digest_text(digest) { return Err(RetainedError::Digest); }
                    digest
                },
                None => return Err(RetainedError::Digest),
            };
            let length = match observed.length {
                Some(length) => length,
                None => return Err(RetainedError::Length),
            };
            Ok(Retained::Blob { path, digest, length })
        },
        _ => Err(RetainedError::Representation),
    }
}

pub open spec fn retained_decoding<S, B, D>(
    state: S, selected: Result<Retained<B>, RetainedError>, decode: D,
    out: (S, Result<Retained, RetainedError>),
) -> bool
    where D: FnOnce(S, B) -> (S, Result<Vec<u8>, ()>),
{
    match selected {
        Err(error) => out.0 == state && out.1 == Err::<Retained, RetainedError>(error),
        Ok(Retained::Absent) => out.0 == state && out.1 == Ok::<Retained, RetainedError>(Retained::Absent),
        Ok(Retained::Blob { path, digest, length }) =>
            out.0 == state && out.1 == Ok::<Retained, RetainedError>(Retained::Blob { path, digest, length }),
        Ok(Retained::Bytes(bytes)) => exists|decoded: (S, Result<Vec<u8>, ()>)|
            decode.ensures((state, bytes), decoded) && out.0 == decoded.0
            && match decoded.1 {
                Ok(value) => out.1 == Ok::<Retained, RetainedError>(Retained::Bytes(value)),
                Err(()) => out.1 == Err::<Retained, RetainedError>(RetainedError::Bytes),
            },
    }
}

/// 仅合格 bytes 调用一次原解码器；其它情况保留初态且零调用。
pub fn parse_retained<'a, S, B, D>(state: S, observed: RetainedObservation<'a, B>, decode: D)
    -> (out: (S, Result<Retained<'a>, RetainedError<'a>>))
    where D: FnOnce(S, B) -> (S, Result<Vec<u8>, ()>),
    requires forall|state: S, bytes: B| decode.requires((state, bytes)),
    ensures exists|selected: Result<Retained<'a, B>, RetainedError<'a>>|
        retained_selection(observed, selected) && retained_decoding(state, selected, decode, out),
{
    let ghost initial = state;
    let selected = select_retained(observed);
    let ghost selection = selected;
    let out = match selected {
        Err(error) => (state, Err(error)),
        Ok(Retained::Absent) => (state, Ok(Retained::Absent)),
        Ok(Retained::Blob { path, digest, length }) =>
            (state, Ok(Retained::Blob { path, digest, length })),
        Ok(Retained::Bytes(bytes)) => {
            let ghost input = bytes;
            let decoded = decode(state, bytes);
            let ghost original = decoded;
            let (state, result) = decoded;
            let out = match result {
                Ok(bytes) => (state, Ok(Retained::Bytes(bytes))),
                Err(()) => (state, Err(RetainedError::Bytes)),
            };
            proof {
                assert(decode.ensures((initial, input), original));
                assert(retained_decoding(initial, selection, decode, out));
            }
            out
        },
    };
    proof {
        assert(retained_selection(observed, selection));
        assert(retained_decoding(initial, selection, decode, out));
        assert(exists|selected: Result<Retained<'a, B>, RetainedError<'a>>|
            retained_selection(observed, selected) && retained_decoding(initial, selected, decode, out));
    }
    out
}

pub open spec fn retained_error_head(error: RetainedError) -> Seq<char> {
    match error {
        RetainedError::Fields(ObjectFieldError::ExpectedObject) => "expected object"@,
        RetainedError::Fields(ObjectFieldError::Missing(_)) => "missing field "@,
        RetainedError::Fields(ObjectFieldError::Unknown(_)) => "unknown field "@,
        RetainedError::Representation => "expected exactly one bytes or blob representation"@,
        RetainedError::Absent => "absent must be true"@,
        RetainedError::Bytes => "invalid retained byte array"@,
        RetainedError::Path => "reference must be within host state"@,
        RetainedError::Digest => "invalid digest"@,
        RetainedError::Length => "invalid length"@,
    }
}

pub open spec fn retained_error_suffix(error: RetainedError) -> Seq<char> {
    match error {
        RetainedError::Fields(ObjectFieldError::Missing(name)) => name@,
        RetainedError::Fields(ObjectFieldError::Unknown(name)) => name@,
        _ => ""@,
    }
}

pub open spec fn retained_error_message(error: RetainedError) -> Seq<char> {
    "E_INPUT_BLOB: "@ + (retained_error_head(error) + retained_error_suffix(error))
}

impl<'a> RetainedError<'a> {
    pub fn parts(self) -> (out: [&'a str; 3])
        ensures out[0]@ == "E_INPUT_BLOB: "@,
            out[1]@ == retained_error_head(self),
            out[2]@ == retained_error_suffix(self),
            out[0]@ + (out[1]@ + out[2]@) == retained_error_message(self),
    {
        let tail = match self {
            Self::Fields(error) => object_error_parts(error),
            Self::Representation => ["expected exactly one bytes or blob representation", ""],
            Self::Absent => ["absent must be true", ""],
            Self::Bytes => ["invalid retained byte array", ""],
            Self::Path => ["reference must be within host state", ""],
            Self::Digest => ["invalid digest", ""],
            Self::Length => ["invalid length", ""],
        };
        ["E_INPUT_BLOB: ", tail[0], tail[1]]
    }
}

} // verus!

#[cfg(not(verus_keep_ghost))]
pub fn retained_shape(value: &Value) -> Result<Retained<'_>, String> {
    let observed = RetainedObservation {
        object: value.is_object(),
        keys: value
            .as_object()
            .into_iter()
            .flat_map(|map| map.keys().map(String::as_str))
            .collect(),
        absent: value.get("absent").map(Value::as_bool),
        bytes: value.get("bytes"),
        blob: value.get("blob").map(Value::as_str),
        digest: value["sha256"].as_str(),
        length: value["length"].as_u64(),
    };
    let (_, parsed) = parse_retained((), observed, |(), bytes: &Value| {
        (
            (),
            serde_json::from_value::<Vec<u8>>(bytes.clone()).map_err(|_| ()),
        )
    });
    parsed.map_err(|error| error.parts().concat())
}

/// Snapshot format shared by the producer and consumer; endpoint policy is separate.
#[cfg(not(verus_keep_ghost))]
pub fn snapshot_shape(value: &Value) -> Result<(), String> {
    crate::schema::object(
        value,
        &[
            "schema",
            "commit",
            "config_path",
            "config_digest",
            "environment",
            "files",
        ],
        &["effective_config_path", "selection"],
    )
    .map_err(|e| format!("E_INPUT_SNAPSHOT: {e}"))?;
    admit_snapshot_header(
        value["schema"].as_str(),
        value.get("effective_config_path").is_some(),
        value.get("selection").is_some(),
    )
    .map_err(str::to_string)?;
    if snapshot_uses_selector(value["schema"].as_str()) {
        let effective = value["effective_config_path"]
            .as_str()
            .ok_or("E_INPUT_SNAPSHOT: missing effective config path")?;
        relative_path(effective)?;
        let selection = crate::schema::object(
            &value["selection"],
            &["schema", "path", "platform", "config_path"],
            &["registry_path"],
        )
        .map_err(|e| format!("E_INPUT_SNAPSHOT: invalid selector binding: {e}"))?;
        admit_snapshot_selector_binding(
            selection["schema"].as_str(),
            selection["platform"]
                .as_str()
                .map(|text| (text, text.trim())),
            selection["path"] == value["config_path"],
            selection["config_path"] == value["effective_config_path"],
            selection["path"] == selection["config_path"],
        )
        .map_err(str::to_string)?;
        for key in ["path", "config_path"] {
            let path = selection[key]
                .as_str()
                .ok_or("E_INPUT_SNAPSHOT: selector path must be a string")?;
            relative_path(path)?;
        }
        if let Some(path) = selection.get("registry_path") {
            relative_path(
                path.as_str()
                    .ok_or("E_INPUT_SNAPSHOT: selector registry path must be a string")?,
            )?;
        }
    }
    facts::full_oid(
        value["commit"]
            .as_str()
            .ok_or("E_INPUT_SNAPSHOT: invalid commit")?,
    )?;
    relative_path(
        value["config_path"]
            .as_str()
            .ok_or("E_INPUT_SNAPSHOT: invalid config path")?,
    )?;
    if !value["config_digest"].as_str().is_some_and(wire::is_digest) {
        return Err("E_INPUT_SNAPSHOT: invalid config digest".into());
    }
    let environment = value["environment"]
        .as_object()
        .ok_or("E_INPUT_SNAPSHOT: invalid environment")?;
    if environment
        .iter()
        .any(|(name, v)| name.is_empty() || !(v.is_null() || v.is_string()))
    {
        return Err("E_INPUT_SNAPSHOT: environment values must be strings or null".into());
    }
    for (id, file) in value["files"]
        .as_object()
        .ok_or("E_INPUT_SNAPSHOT: invalid files")?
    {
        if id.is_empty() {
            return Err("E_INPUT_SNAPSHOT: empty file ID".into());
        }
        admit_snapshot_retained(value["schema"].as_str(), &retained_shape(file)?)
            .map_err(str::to_string)?;
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
fn retained_identity(root: &Path, value: &Value) -> Result<Option<(String, u64)>, String> {
    match retained_shape(value)? {
        Retained::Absent => Ok(None),
        Retained::Bytes(bytes) => Ok(Some((sha256(&bytes), bytes.len() as u64))),
        Retained::Blob {
            path,
            digest,
            length,
        } => {
            let _input = chrono_harness::retained_artifacts::retention::read_guard(root, path)
                .map_err(|e| format!("E_INPUT_BLOB: {e}"))?;
            let path = no_symlink_parents(root, path).map_err(|e| format!("E_INPUT_BLOB: {e}"))?;
            let observed = file_identity(&path).map_err(|e| format!("E_INPUT_BLOB: {e}"))?;
            if observed != (digest.into(), length) {
                return Err("E_INPUT_BLOB: retained content identity mismatch".into());
            }
            Ok(Some(observed))
        }
    }
}
#[cfg(not(verus_keep_ghost))]
pub fn environment(config: &Value, snapshot: &Value) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    let credentials = if config["schema_version"] == 4 {
        chrono_harness::prepared::credential_environment(config)?
    } else {
        Default::default()
    };
    if let Some(values) = snapshot.as_object() {
        for key in values.keys() {
            if !config["environment"]["inherit"]
                .as_array()
                .ok_or("inherit missing")?
                .contains(&json!(key))
                && config["environment"]["values"].get(key).is_none()
            {
                return Err(format!("unregistered retained environment variable {key}"));
            }
        }
    }
    for key in config["environment"]["inherit"]
        .as_array()
        .ok_or("inherit missing")?
    {
        let key = key.as_str().ok_or("inherit key")?;
        if credentials.contains(key) && snapshot.get(key) != Some(&Value::Null) {
            return Err(
                "acquisition credentials cannot be retained as business environment inputs".into(),
            );
        }
        match snapshot
            .get(key)
            .ok_or_else(|| format!("missing retained environment variable {key}"))?
        {
            Value::Null => {}
            Value::String(value) => {
                out.insert(key.into(), value.clone());
            }
            _ => {
                return Err(
                    "environment must distinguish absent/null and string (including empty)".into(),
                );
            }
        }
    }
    for (key, value) in config["environment"]["values"]
        .as_object()
        .ok_or("environment values")?
    {
        out.insert(
            key.clone(),
            value.as_str().ok_or("environment value")?.into(),
        );
    }
    Ok(out)
}
/// Required declared files for one unit, including explicit operation owners,
/// their FILEMAP prerequisites, shared operations and governance consumers.
/// Collection starts with governance inputs; routes supplies its DELTA obligations
/// after FILEMAP impact. No IO discovery or report-derived selection.
#[cfg(not(verus_keep_ghost))]
pub fn required_files(
    r: &Registrations,
    scope: Option<&chrono_harness::units::Scope>,
) -> Result<std::collections::BTreeSet<String>, String> {
    let selected = match scope {
        Some(chrono_harness::units::Scope::Unit { unit }) => Some(assigned_plans(r, unit)?),
        Some(chrono_harness::units::Scope::Collect { .. }) => Some(Default::default()),
        _ => None,
    };
    required_plan_files(r, selected.as_ref())
}

#[cfg(not(verus_keep_ghost))]
fn assigned_plans(
    r: &Registrations,
    unit: &str,
) -> Result<std::collections::BTreeSet<String>, String> {
    use chrono_harness::units::{self, Unit};
    let units: BTreeMap<String, Unit> =
        serde_json::from_value(r.config()["execution_units"]["units"].clone())
            .map_err(|e| format!("E_UNIT_ASSIGNMENT: {e}"))?;
    let plans = crate::execution::plans(r.filemap())?;
    units::select(&units, &plans.keys().cloned().collect(), unit)
}

#[cfg(not(verus_keep_ghost))]
pub fn required_plan_files(
    r: &Registrations,
    selected: Option<&std::collections::BTreeSet<String>>,
) -> Result<std::collections::BTreeSet<String>, String> {
    use crate::NodeKind;
    use std::collections::BTreeSet;
    let mut all = BTreeSet::new();
    for input in r.config()["environment"]["inputs"]
        .as_array()
        .ok_or("inputs missing")?
    {
        all.insert(input["id"].as_str().ok_or("input id missing")?.to_owned());
    }
    let Some(selected) = selected else {
        return Ok(all);
    };
    if let Some(finding) = crate::input_closure_findings(r).first() {
        return Err(format!("{}: {}", finding.code, finding.message));
    }
    let plans = crate::execution::plans(r.filemap())?;
    let methods = crate::execution::methods(r.projects())?;
    let nodes = r.node_data();
    let mut needed = selected.clone();
    for test in selected {
        let definition = nodes
            .get(test)
            .and_then(|n| n.unique())
            .ok_or_else(|| format!("E_REFERENCE: missing/ambiguous input consumer {test}"))?;
        needed.insert(definition.identity.clone());
        for operation in &plans[test].operations {
            let method = methods
                .get(operation)
                .filter(|m| m.len() == 1)
                .ok_or_else(|| format!("E_REFERENCE: missing/ambiguous operation {operation}"))?;
            needed.insert(method[0].owner.clone());
            needed.insert(format!("tool:{}", method[0].tool));
        }
    }
    needed.extend(
        nodes
            .iter()
            .filter(|(_, n)| n.kind == NodeKind::Judge)
            .map(|(id, _)| id.clone()),
    );
    if let Some(id) = r.config()["facts_git"]["input"].as_str() {
        needed.insert(format!("input:{id}"));
    }
    // The whitelist carries dependencies only in their declared direction.
    // Reverse it to obtain the prerequisites of the chosen consumers.
    let mut edges = Vec::new();
    for file in r.filemap()["files"].as_array().unwrap() {
        for edge in file["edges"].as_array().unwrap() {
            edges.push((
                format!("file:{}", file["path"].as_str().unwrap()),
                edge.clone(),
            ));
        }
    }
    for edge in r.filemap()["project_edges"].as_array().unwrap() {
        edges.push((edge["from"].as_str().unwrap().to_owned(), edge.clone()));
    }
    for (from, edge) in &edges {
        for node in [from.as_str(), edge["to"].as_str().unwrap()] {
            if !nodes.contains_key(node) {
                return Err(format!("E_REFERENCE: unknown input dependency {node}"));
            }
        }
    }
    loop {
        let size = needed.len();
        for (from, edge) in &edges {
            if needed.contains(edge["to"].as_str().unwrap())
                && matches!(
                    edge["kind"].as_str(),
                    Some("compile" | "build-input" | "runtime-input" | "test-execution")
                )
            {
                needed.insert(from.clone());
            }
        }
        // Closure bindings are validated against explicit FILEMAP edges above.
        for binding in r.config()["input_closure"]["bindings"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if needed.contains(binding["consumer"].as_str().unwrap()) {
                needed.extend(
                    binding["inputs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap().to_owned()),
                );
            }
        }
        if needed.len() == size {
            break;
        }
    }
    Ok(all
        .into_iter()
        .filter(|id| needed.contains(&format!("input:{id}")))
        .collect())
}

/// Project routes' selected obligations through the existing explicit unit map.
/// Retain the old assignments as well as candidate plans that existed at base.
#[cfg(not(verus_keep_ghost))]
pub fn collection_required_files(
    endpoint: &str,
    old: &Registrations,
    new: &Registrations,
    selected: &std::collections::BTreeSet<String>,
) -> Result<std::collections::BTreeSet<String>, String> {
    let units = chrono_harness::units::full_execution_units(new.config())?
        .ok_or("collection execution units missing")?;
    let units: BTreeMap<String, chrono_harness::units::Unit> =
        serde_json::from_value(units["units"].clone()).map_err(|e| e.to_string())?;
    let mut plans = std::collections::BTreeSet::new();
    for unit in chrono_harness::units::required(&units, selected) {
        plans.extend(assigned_plans(new, &unit)?);
        if endpoint == "base"
            && old.config()["execution_units"]["units"]
                .get(&unit)
                .is_some()
        {
            plans.extend(assigned_plans(old, &unit)?);
        }
    }
    let r = if endpoint == "base" { old } else { new };
    if endpoint == "base" {
        let existing = crate::execution::plans(old.filemap())?;
        plans.retain(|p| existing.contains_key(p));
    }
    required_plan_files(r, Some(&plans))
}

#[cfg(not(verus_keep_ghost))]
pub fn endpoint_required_files(
    endpoint: &str,
    old: &Registrations,
    new: &Registrations,
    scope: Option<&chrono_harness::units::Scope>,
) -> Result<std::collections::BTreeSet<String>, String> {
    if endpoint != "base" {
        return required_files(new, scope);
    }
    let Some(chrono_harness::units::Scope::Unit { unit }) = scope else {
        return required_files(old, scope);
    };
    // Candidate IDs need not exist historically. Project their declared plans
    // onto the base, retaining also any plans assigned to this ID at the base.
    // Both projections use the same input closure; no global-input fallback.
    let plans = crate::execution::plans(old.filemap())?;
    let mut selected = assigned_plans(new, unit)?
        .into_iter()
        .filter(|test| plans.contains_key(test))
        .collect::<std::collections::BTreeSet<_>>();
    if old.config()["execution_units"]["units"].get(unit).is_some() {
        selected.extend(assigned_plans(old, unit)?);
    }
    required_plan_files(old, Some(&selected))
}

/// Compare original per-unit evidence with the same projection of the current
/// collection obligations, retaining exact environment and endpoint bindings.
#[cfg(not(verus_keep_ghost))]
pub fn project_effective(
    evidence: &Value,
    old: &Registrations,
    new: &Registrations,
    scope: Option<&chrono_harness::units::Scope>,
) -> Result<Value, String> {
    if evidence.is_null() {
        return Ok(Value::Null);
    }
    if evidence["identity"] != wire::digest(&evidence["endpoints"])? {
        return Err("effective input identity mismatch".into());
    }
    let mut projected = evidence.clone();
    for endpoint in ["base", "candidate"] {
        let required = endpoint_required_files(endpoint, old, new, scope)?;
        let files = projected["endpoints"][endpoint]["files"]
            .as_object_mut()
            .ok_or("effective input files missing")?;
        for id in &required {
            if !files.contains_key(id) {
                return Err(format!("missing effective input {endpoint}: {id}"));
            }
        }
        files.retain(|id, _| required.contains(id));
    }
    projected["identity"] = json!(wire::digest(&projected["endpoints"])?);
    Ok(projected)
}

#[cfg(not(verus_keep_ghost))]
pub fn validate(req: &Request, old: &Registrations, new: &Registrations) -> Result<Value, String> {
    validate_inner(
        req,
        old,
        new,
        None,
        !matches!(
            req.scope,
            Some(chrono_harness::units::Scope::Collect { .. })
        ),
    )
}
/// Routes is the selection owner. It checks collection coverage before preparing
/// the zero-business collection plan, without a registration/filemap cycle.
#[cfg(not(verus_keep_ghost))]
pub fn validate_collection(
    req: &Request,
    old: &Registrations,
    new: &Registrations,
    selected: &std::collections::BTreeSet<String>,
) -> Result<Value, String> {
    for endpoint in ["base", "candidate"] {
        for id in collection_required_files(endpoint, old, new, selected)? {
            if req.observations["retained"][endpoint]["files"]
                .get(&id)
                .is_none()
            {
                return Err(format!("{endpoint}: missing retained input {id}"));
            }
        }
    }
    validate(req, old, new)
}
#[cfg(not(verus_keep_ghost))]
pub fn validate_retained_at(
    req: &Request,
    old: &Registrations,
    new: &Registrations,
    artifacts: &Value,
    root: &Path,
    transport: Option<&chrono_harness::prepared::ArtifactTransport>,
) -> Result<Value, String> {
    validate_inner(req, old, new, Some((artifacts, root, transport)), false)
}
#[cfg(not(verus_keep_ghost))]
fn validate_inner(
    req: &Request,
    old: &Registrations,
    new: &Registrations,
    artifacts: Option<(
        &Value,
        &Path,
        Option<&chrono_harness::prepared::ArtifactTransport>,
    )>,
    live: bool,
) -> Result<Value, String> {
    if new.filemap()["schema_version"] == 1
        && old.config()["schema_version"] == 1
        && new.config()["schema_version"] == 1
        && old.config()["environment"]["inputs"]
            .as_array()
            .unwrap()
            .is_empty()
        && new.config()["environment"]["inputs"]
            .as_array()
            .unwrap()
            .is_empty()
        && req.observations["retained"].is_null()
    {
        // Historical registration-only profiles have no retained-input success claim.
        return Ok(Value::Null);
    }
    let mut endpoints = serde_json::Map::new();
    for (name, endpoint, r) in [("base", &req.base, old), ("candidate", &req.candidate, new)] {
        let cfg = r.config();
        let required = endpoint_required_files(name, old, new, req.scope.as_ref())?;
        let retained = &req.observations["retained"][name];
        if matches!(cfg["schema_version"].as_u64(), Some(2 | 3 | 4))
            && retained["schema"] != snapshot_schema_for(r)
        {
            return Err(if r.selection().is_some() {
                format!("{name}: selected config requires snapshot v3")
            } else {
                format!("{name}: config v2 requires snapshot v2")
            });
        }
        if retained.get("schema").is_some() {
            snapshot_shape(retained)?;
            if retained["schema"] != snapshot_schema_for(r)
                || retained["config_path"] != req.config_path
                || retained["config_digest"] != wire::digest(cfg)?
            {
                return Err(format!("{name}: snapshot configuration binding mismatch"));
            }
            if snapshot_uses_selector(retained["schema"].as_str())
                && (retained["effective_config_path"] != r.effective_config_path()
                    || retained["selection"] != r.selection().cloned().unwrap_or(Value::Null))
            {
                return Err(format!("{name}: snapshot selection binding mismatch"));
            }
        }
        let files = cfg["environment"]["inputs"]
            .as_array()
            .ok_or("inputs missing")?;
        let strict_environment = new.filemap()["schema_version"] == 2;
        if !crate::verified_readiness::endpoint_snapshot_matches(
            retained.get("schema").is_some(),
            files.len(),
            strict_environment,
            cfg["environment"]["inherit"].as_array().unwrap().len(),
            retained["commit"].as_str(),
            &endpoint.commit,
        ) {
            return Err(format!(
                "{name}: retained input snapshot absent or wrong endpoint"
            ));
        }
        let inherited = if retained.is_object() {
            &retained["environment"]
        } else if name == "candidate" {
            &req.observations["environment"]["inherited"]
        } else {
            &Value::Null
        };
        let effective = environment(cfg, inherited);
        if strict_environment && effective.is_err() {
            return Err(format!("{name}: {}", effective.unwrap_err()));
        }
        if name == "candidate"
            && strict_environment
            && serde_json::to_value(effective.as_ref().unwrap()).unwrap()
                != req.observations["environment"]["effective"]
        {
            return Err("candidate environment differs from entry observation".into());
        }
        let mut evidence = BTreeMap::new();
        if let Some(retained_files) = retained["files"].as_object() {
            for id in retained_files.keys() {
                if !files.iter().any(|i| i["id"] == *id) {
                    return Err(format!("{name}: unregistered retained input {id}"));
                }
            }
        }
        for input in files {
            let id = input["id"].as_str().unwrap();
            if !required.contains(id) && retained["files"].get(id).is_none() {
                continue;
            }
            let value = retained["files"]
                .get(id)
                .ok_or_else(|| format!("{name}: missing retained input {id}"))?;
            let identity = (|| match (artifacts, retained_shape(value)?) {
                (
                    Some((artifacts, root, transport)),
                    Retained::Blob {
                        path,
                        digest,
                        length,
                    },
                ) => {
                    let identity = chrono_harness::retained_artifacts::identity(
                        root, artifacts, path, transport,
                    )?;
                    if identity != (digest.to_owned(), length) {
                        return Err("E_INPUT_BLOB: original input differs".into());
                    }
                    Ok(Some((digest.to_owned(), length)))
                }
                _ => retained_identity(&req.candidate.root, value),
            })()
            .map_err(|e| format!("{name}: retained input {id}: {e}"))?;
            match &identity {
                None if matches!(cfg["schema_version"].as_u64(), Some(2 | 3 | 4))
                    && input["presence"] == "absent" => {}
                Some((digest, _))
                    if input["presence"] != "absent"
                        && input["sha256"].as_str() == Some(digest) => {}
                _ => {
                    return Err(format!(
                        "{name}: retained input digest/presence mismatch: {id}"
                    ));
                }
            }
            if name == "candidate" && live && required.contains(id) {
                let path = Path::new(input["location"].as_str().unwrap());
                let path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    req.candidate.root.join(path)
                };
                let actual = if matches!(cfg["schema_version"].as_u64(), Some(2 | 3 | 4)) {
                    observe_file(&path)
                } else {
                    file_identity(&path).map(Some)
                };
                if actual.map_err(|e| format!("candidate input {id}: {e}"))? != identity {
                    return Err(format!("candidate input changed: {id}"));
                }
            }
            let mut fact = match identity {
                None => json!({"presence":"absent","location":input["location"]}),
                Some((digest, length)) => {
                    json!({"sha256":digest,"length":length,"location":input["location"]})
                }
            };
            if matches!(cfg["schema_version"].as_u64(), Some(2 | 3 | 4))
                && fact.get("presence").is_none()
            {
                fact["presence"] = json!("present");
            }
            if required.contains(id)
                || matches!(
                    req.scope,
                    Some(chrono_harness::units::Scope::Collect { .. })
                )
            {
                evidence.insert(id.to_string(), fact);
            }
        }
        endpoints.insert(name.into(),json!({"commit":endpoint.commit,"environment":effective.ok(),"inherited":inherited,"files":evidence}));
    }
    let endpoints = Value::Object(endpoints);
    Ok(
        json!({"schema":effective_schema(old,new),"identity":wire::digest(&endpoints)?,"endpoints":endpoints,"completeness_proven":false}),
    )
}

#[cfg(not(verus_keep_ghost))]
fn effective_schema(old: &Registrations, new: &Registrations) -> &'static str {
    if matches!(old.config()["schema_version"].as_u64(), Some(2 | 3 | 4))
        || matches!(new.config()["schema_version"].as_u64(), Some(2 | 3 | 4))
    {
        "chrono-effective-inputs/v2"
    } else {
        "chrono-effective-inputs/v1"
    }
}

/// Consume retained facts through the same environment semantics as validation.
/// Null is supported only by the historical registration-only v1 contract.
#[cfg(not(verus_keep_ghost))]
pub fn effective_environments(
    evidence: &Value,
    old: &Registrations,
    new: &Registrations,
) -> Result<Option<[BTreeMap<String, String>; 2]>, String> {
    if evidence.is_null()
        && new.filemap()["schema_version"] == 1
        && old.config()["schema_version"] == 1
        && new.config()["schema_version"] == 1
    {
        return Ok(None);
    }
    if evidence["schema"] != effective_schema(old, new)
        || evidence["identity"] != wire::digest(&evidence["endpoints"])?
    {
        return Err("effective input identity missing/mismatched".into());
    }
    let mut pair = [BTreeMap::new(), BTreeMap::new()];
    for (index, name, r) in [(0, "base", old), (1, "candidate", new)] {
        let endpoint = &evidence["endpoints"][name];
        let effective = environment(r.config(), &endpoint["inherited"])?;
        if serde_json::to_value(&effective).unwrap() != endpoint["environment"] {
            return Err(format!(
                "{name}: effective environment differs from declared retained inputs"
            ));
        }
        pair[index] = effective;
    }
    Ok(Some(pair))
}
