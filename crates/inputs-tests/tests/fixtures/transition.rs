use serde_json::Value;

pub fn upgrade_config(_: &Value) -> Result<Value, String> {
    Err("Rust input decoder fixture is not implemented".into())
}

pub fn upgrade_workflow(_: &Value) -> Result<Value, String> {
    Err("Rust input decoder fixture is not implemented".into())
}

pub fn convert_direct(_: &Value) -> Result<Value, String> {
    Err("Rust input decoder fixture is not implemented".into())
}

pub fn convert_selected(_: &Value) -> Result<Value, String> {
    Err("Rust input decoder fixture is not implemented".into())
}
