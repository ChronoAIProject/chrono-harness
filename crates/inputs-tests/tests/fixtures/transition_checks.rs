use super::transition;
use serde_json::{Value, json};

pub fn direct() {
    let mut source = json!({"schema_version":1,"environment":{"inputs":[{"id":"empty","location":"x","sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}]}});
    let expected = json!({"schema_version":2,"environment":{"inputs":[{"id":"empty","location":"x","sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","presence":"present"}]}});
    assert_eq!(transition::upgrade_config(&source).unwrap(), expected);
    assert_eq!(source["schema_version"], 1);
    assert!(source["environment"]["inputs"][0].get("presence").is_none());
    source["environment"]["inputs"][0]["sha256"] = Value::Null;
    assert!(transition::upgrade_config(&source).is_err());
    println!("config migration compatibility checked");
}

pub fn selected(request: &Value) {
    let source = json!({"schema_version":1,"migrations":[]});
    assert_eq!(
        transition::upgrade_workflow(&source).unwrap(),
        json!({"schema_version":3,"migrations":[],"historical_profiles":[]})
    );
    assert_eq!(source, json!({"schema_version":1,"migrations":[]}));
    assert!(transition::upgrade_workflow(&json!({"schema_version":3})).is_err());
    assert_eq!(
        transition::convert_selected(request).unwrap()["values"],
        request["original"]
    );
    for endpoint in ["base", "candidate"] {
        let mut bad = request.clone();
        bad["config_bindings"]
            .as_object_mut()
            .unwrap()
            .remove(endpoint);
        assert!(transition::convert_selected(&bad).is_err());
    }
    println!("selected workflow migration compatibility checked");
}
