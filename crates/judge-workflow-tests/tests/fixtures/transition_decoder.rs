use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing {key}"))
}

fn text(value: &Value) -> Result<&str, String> {
    value.as_str().ok_or_else(|| "expected string".into())
}

fn original_json(value: &Value) -> Result<Value, String> {
    let bytes: Vec<u8> = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub fn convert(request: &Value) -> Result<Value, String> {
    if field(request, "schema")? != "chrono-historical-decode/v1" {
        return Err("unsupported decoder protocol".into());
    }
    let profile = field(request, "profile")?;
    let original = field(request, "original")?;
    let config = field(original, text(field(request, "config_path")?)?)?;
    let registries = field(config, "registries")?;
    let filemap = text(field(registries, "filemap")?)?;
    let projects = text(field(registries, "projects")?)?;
    let scoped = field(request, "profile_value")?;
    let policy = field(scoped, "policy")?;
    if field(field(original, filemap)?, "schema_version")? != 1
        || field(scoped, "schema")? != field(profile, "id")?
        || field(profile, "id")? != "chrono-ci-check/v1"
        || field(policy, "filemap")? != filemap
        || field(policy, "projects")? != projects
    {
        return Err("unsupported version/profile pair".into());
    }
    if original_json(field(request, "profile_bytes")?)? != *scoped {
        return Err("profile bytes mismatch".into());
    }
    for (path, value) in original.as_object().ok_or("original must be an object")? {
        if original_json(field(field(request, "original_bytes")?, path)?)? != *value {
            return Err("original bytes mismatch".into());
        }
    }
    let mut values = original.clone();
    let mut historical = Map::new();
    for record in field(profile, "legacy_records")?
        .as_array()
        .ok_or("legacy records must be an array")?
    {
        let id = text(field(record, "id")?)?;
        let collection = text(field(record, "collection")?)?;
        let definition = field(record, "definition")?;
        let entries = values
            .get_mut(projects)
            .and_then(|p| p.get_mut(collection))
            .and_then(Value::as_array_mut)
            .ok_or("historical collection must be an array")?;
        let mut matches = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            if field(entry, "id")? == id {
                matches.push(index);
            }
        }
        if matches.len() != 1 || entries[matches[0]] != *definition {
            return Err("historical definition mismatch".into());
        }
        let retired = entries.remove(matches[0]);
        for node in field(record, "nodes")?
            .as_array()
            .ok_or("historical nodes must be an array")?
        {
            historical
                .entry(text(node)?.to_owned())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .unwrap()
                .push(json!([format!("script:{id}"), retired]));
        }
    }
    let mut plans = Map::new();
    for (test, operations) in field(policy, "bindings")?
        .as_object()
        .ok_or("historical bindings must be an object")?
    {
        let sequence = operations
            .as_array()
            .ok_or("historical ordered binding must be an array")?;
        let unique: BTreeSet<_> = sequence.iter().map(text).collect::<Result<_, _>>()?;
        if sequence.is_empty() || unique.len() != sequence.len() {
            return Err("invalid historical ordered binding".into());
        }
        plans.insert(
            test.clone(),
            json!({
                "operations":operations,
                "timeout_seconds":field(policy, "operation_timeout_seconds")?,
                "output_limit_bytes":field(policy, "operation_output_limit_bytes")?
            }),
        );
    }
    values[filemap]["schema_version"] = json!(2);
    values[filemap]["execution_plans"] = Value::Object(plans);
    Ok(json!({"values":values,"historical":historical,"mappings":field(profile, "mappings")?}))
}
