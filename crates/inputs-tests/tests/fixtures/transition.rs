use serde_json::{Value, json};

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing {key}"))
}

fn text(value: &Value) -> Result<&str, String> {
    value.as_str().ok_or_else(|| "expected string".into())
}

fn originals(request: &Value) -> Result<&Value, String> {
    let original = field(request, "original")?;
    for (path, value) in original.as_object().ok_or("original must be an object")? {
        let bytes: Vec<u8> =
            serde_json::from_value(field(field(request, "original_bytes")?, path)?.clone())
                .map_err(|error| error.to_string())?;
        let decoded: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if decoded != *value {
            return Err("original bytes mismatch".into());
        }
    }
    Ok(original)
}

fn result(request: &Value) -> Result<Value, String> {
    Ok(json!({
        "values":field(request, "original")?,
        "historical":{},
        "mappings":field(field(request, "profile")?, "mappings")?
    }))
}

pub fn upgrade_config(source: &Value) -> Result<Value, String> {
    if field(source, "schema_version")? != 1 {
        return Err("source config must be v1".into());
    }
    let mut out = source.clone();
    out["schema_version"] = json!(2);
    let inputs = out
        .get_mut("environment")
        .and_then(|environment| environment.get_mut("inputs"))
        .and_then(Value::as_array_mut)
        .ok_or("config inputs must be an array")?;
    for input in inputs {
        if field(input, "sha256")?.is_null() {
            return Err("unbound digest is not absence".into());
        }
        input["presence"] = json!("present");
    }
    Ok(out)
}

pub fn upgrade_workflow(source: &Value) -> Result<Value, String> {
    if field(source, "schema_version")? != 1 {
        return Err("source workflow must be v1".into());
    }
    let mut out = source.clone();
    out["schema_version"] = json!(3);
    out["historical_profiles"] = json!([]);
    Ok(out)
}

pub fn convert_direct(request: &Value) -> Result<Value, String> {
    if field(request, "schema")? != "chrono-historical-decode/v2" {
        return Err("unsupported decoder protocol".into());
    }
    let original = originals(request)?;
    let config = text(field(request, "config_path")?)?;
    if upgrade_config(field(original, config)?)? != *field(field(request, "candidate")?, config)? {
        return Err("config transition differs".into());
    }
    result(request)
}

fn selected_effective<'a>(
    request: &'a Value,
    endpoint: &str,
    view: &str,
) -> Result<&'a Value, String> {
    let identity = field(field(request, "config_bindings")?, endpoint)?;
    let entry = text(field(identity, "entry_path")?)?;
    let selection = field(identity, "selection")?;
    let effective = text(field(identity, "effective_path")?)?;
    let values = field(request, view)?;
    let platform = text(field(selection, "platform")?)?;
    if field(request, "config_path")? != entry
        || field(selection, "path")? != entry
        || field(field(field(values, entry)?, "platforms")?, platform)? != effective
        || field(selection, "config_path")? != effective
    {
        return Err("selected endpoint binding differs".into());
    }
    let config = field(values, effective)?;
    if field(config, "schema_version")? != 3 {
        return Err("selected effective config must be v3".into());
    }
    Ok(config)
}

pub fn convert_selected(request: &Value) -> Result<Value, String> {
    if field(request, "schema")? != "chrono-historical-decode/v3" {
        return Err("unsupported decoder protocol".into());
    }
    let base = selected_effective(request, "base", "original")?;
    let candidate = selected_effective(request, "candidate", "candidate")?;
    let original = originals(request)?;
    if base != candidate {
        return Err("effective config transition differs".into());
    }
    let old_path = text(field(field(base, "registries")?, "workflow")?)?;
    let new_path = text(field(field(candidate, "registries")?, "workflow")?)?;
    let old = field(original, old_path)?;
    let new = field(field(request, "candidate")?, new_path)?;
    let mut expected = upgrade_workflow(old)?;
    for key in ["historical_profiles", "migrations", "integration"] {
        expected[key] = field(new, key)?.clone();
    }
    if expected != *new {
        return Err("workflow transition differs".into());
    }
    result(request)
}
