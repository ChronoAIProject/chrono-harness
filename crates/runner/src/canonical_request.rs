//! Canonical JSON for the already typed request projection. Keep large
//! observations borrowed and write containers directly into one output buffer.
use serde::Serialize;
use serde_json::Value;

pub(crate) fn encode<T: Serialize>(header: &T, observations: &Value) -> Result<Vec<u8>, String> {
    let header = serde_json::to_value(header).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    object(
        header
            .as_object()
            .ok_or("request projection must be an object")?
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .chain(std::iter::once(("observations", observations))),
        &mut out,
    )?;
    Ok(out)
}
fn object<'a>(
    entries: impl Iterator<Item = (&'a str, &'a Value)>,
    out: &mut Vec<u8>,
) -> Result<(), String> {
    let mut entries: Vec<_> = entries.collect();
    // RFC 8785 orders unescaped property names by UTF-16 code units.
    entries.sort_unstable_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
    out.push(b'{');
    for (i, (key, value)) in entries.into_iter().enumerate() {
        if i != 0 {
            out.push(b',');
        }
        serde_json::to_writer(&mut *out, key).map_err(|e| e.to_string())?;
        out.push(b':');
        write(value, out)?;
    }
    out.push(b'}');
    Ok(())
}
fn write(value: &Value, out: &mut Vec<u8>) -> Result<(), String> {
    match value {
        Value::Object(values) => object(values.iter().map(|(k, v)| (k.as_str(), v)), out),
        Value::Array(values) => {
            out.push(b'[');
            for (i, value) in values.iter().enumerate() {
                if i != 0 {
                    out.push(b',');
                }
                write(value, out)?;
            }
            out.push(b']');
            Ok(())
        }
        // These integers have exactly the same decimal spelling in JSON and
        // ECMAScript. In particular, original byte arrays avoid per-element
        // floating point formatting and the JCS formatter's boxed writers.
        Value::Number(n)
            if n.as_i64()
                .is_some_and(|n| (-9007199254740992..=9007199254740992).contains(&n)) =>
        {
            serde_json::to_writer(out, n).map_err(|e| e.to_string())
        }
        // The fixed dependency still owns every floating point/large integer
        // normalization, including negative zero and precision boundaries.
        Value::Number(_) => serde_jcs::to_writer(out, value).map_err(|e| e.to_string()),
        _ => serde_json::to_writer(out, value).map_err(|e| e.to_string()),
    }
}
