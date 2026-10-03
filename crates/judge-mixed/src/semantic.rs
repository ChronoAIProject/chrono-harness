//! Stable row identities are separate from physical JSON pointers.
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
fn escape(s: &str) -> String {
    s.replace('~', "~0").replace('/', "~1")
}
fn tokens(pointer: &str) -> Result<Vec<String>, String> {
    let tail = pointer
        .strip_prefix('/')
        .ok_or("E_SEMANTIC_POINTER: expected / prefix")?;
    tail.split('/')
        .map(|part| {
            let mut decoded = String::new();
            let mut chars = part.chars();
            while let Some(c) = chars.next() {
                decoded.push(if c == '~' {
                    match chars.next() {
                        Some('0') => '~',
                        Some('1') => '/',
                        _ => {
                            return Err(format!("E_SEMANTIC_POINTER: invalid escape in {pointer}"));
                        }
                    }
                } else {
                    c
                });
            }
            Ok(decoded)
        })
        .collect()
}
fn row_ids(rows: &[Value], workflow_retirements: bool) -> Result<Option<Vec<String>>, String> {
    if !rows
        .iter()
        .any(|r| r.get("id").is_some() || r.get("path").is_some())
    {
        return Ok(None);
    }
    let mut seen = BTreeSet::new();
    let mut ids = vec![];
    for row in rows {
        let key = if row.get("id").is_some() {
            "id"
        } else {
            "path"
        };
        let value = row[key]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("E_SEMANTIC_IDENTITY: mixed or invalid path/id rows")?;
        // The workflow contract identifies retirements by (kind, id). Other
        // collections retain their original id/path uniqueness contract.
        let id = if workflow_retirements {
            let kind = row["kind"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("E_SEMANTIC_IDENTITY: invalid retirement kind")?;
            if key != "id" {
                return Err("E_SEMANTIC_IDENTITY: retirement requires id".into());
            }
            format!("retirement:{}", json!([kind, value]))
        } else {
            format!("{key}:{value}")
        };
        if !seen.insert(id.clone()) {
            return Err(format!("E_SEMANTIC_IDENTITY: duplicate {id}"));
        }
        ids.push(id);
    }
    Ok(Some(ids))
}
// Kind tags keep user objects distinct from the internal row-map representation.
fn normalized(value: &Value, pointer: &str, workflow: bool) -> Result<Value, String> {
    match value {
        Value::Object(map) => Ok(json!([
            "object",
            map.iter()
                .map(|(k, v)| Ok((
                    k.clone(),
                    normalized(v, &format!("{pointer}/{}", escape(k)), workflow)?
                )))
                .collect::<Result<BTreeMap<_, _>, String>>()?
        ])),
        Value::Array(rows) => {
            let values = rows
                .iter()
                .enumerate()
                .map(|(i, v)| normalized(v, &format!("{pointer}/{i}"), workflow))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(
                if let Some(ids) = row_ids(rows, workflow && pointer == "/retirements")? {
                    json!([
                        "rows",
                        ids.into_iter().zip(values).collect::<BTreeMap<_, _>>()
                    ])
                } else {
                    json!(["array", values])
                },
            )
        }
        _ => Ok(json!(["scalar", value])),
    }
}
struct Selection {
    pointer: String,
    value: Value,
    normalized: Value,
}
type Selections = BTreeMap<Vec<String>, Selection>;
fn select(
    value: &Value,
    rest: &[String],
    pointer: String,
    identity: Vec<String>,
    out: &mut Selections,
    workflow: bool,
) -> Result<(), String> {
    if rest.is_empty() {
        let entry = Selection {
            normalized: normalized(value, &pointer, workflow)?,
            pointer,
            value: value.clone(),
        };
        if out.insert(identity, entry).is_some() {
            return Err("E_SEMANTIC_IDENTITY: ambiguous selection".into());
        }
        return Ok(());
    }
    let token = &rest[0];
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if token == "*" || token == key {
                    let mut next = identity.clone();
                    next.push(format!("key:{key}"));
                    select(
                        child,
                        &rest[1..],
                        format!("{pointer}/{}", escape(key)),
                        next,
                        out,
                        workflow,
                    )?;
                }
            }
        }
        Value::Array(rows) => {
            let ids = row_ids(rows, workflow && pointer == "/retirements")?;
            for (i, child) in rows.iter().enumerate() {
                if token == "*" || token == &i.to_string() {
                    let mut next = identity.clone();
                    next.push(
                        ids.as_ref()
                            .map(|ids| ids[i].clone())
                            .unwrap_or_else(|| format!("index:{i}")),
                    );
                    select(
                        child,
                        &rest[1..],
                        format!("{pointer}/{i}"),
                        next,
                        out,
                        workflow,
                    )?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn changes(
    before: Option<&Value>,
    after: Option<&Value>,
    pattern: &str,
    on: &str,
    before_workflow: bool,
    after_workflow: bool,
) -> Result<Vec<Value>, String> {
    let parts = tokens(pattern)?;
    let mut old = Selections::new();
    let mut new = Selections::new();
    if let Some(v) = before {
        select(v, &parts, String::new(), vec![], &mut old, before_workflow)?;
    }
    if let Some(v) = after {
        select(v, &parts, String::new(), vec![], &mut new, after_workflow)?;
    }
    let keys: BTreeSet<_> = old.keys().chain(new.keys()).collect();
    let mut changes = vec![];
    for key in keys {
        let a = old.get(key);
        let b = new.get(key);
        if on == "modify-delete-existing" && a.is_none() {
            continue;
        }
        if a.map(|s| &s.normalized) == b.map(|s| &s.normalized) {
            continue;
        }
        let encode = |s: &Selection| json!({"pointer":s.pointer,"value":s.value});
        changes.push(json!({"pattern":pattern,"on":on,"identity":key,"before":a.map(encode),"after":b.map(encode)}));
    }
    Ok(changes)
}
