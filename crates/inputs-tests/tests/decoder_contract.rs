#[path = "fixtures/transition.rs"]
mod transition;

use serde_json::{Value, json};

fn original_bytes(request: &mut Value) {
    let original = request["original"].as_object().unwrap().clone();
    for (path, value) in original {
        request["original_bytes"][path] = json!(serde_json::to_vec(&value).unwrap());
    }
}

#[test]
fn migration_fixture_contract() {
    let config = json!({"schema_version":1,"environment":{"inputs":[{"id":"empty","location":"x","sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}]}});
    let mut upgraded = config.clone();
    upgraded["schema_version"] = json!(2);
    upgraded["environment"]["inputs"][0]["presence"] = json!("present");
    assert_eq!(transition::upgrade_config(&config).unwrap(), upgraded);
    assert_eq!(config["schema_version"], 1);
    assert!(config["environment"]["inputs"][0].get("presence").is_none());
    let mut bad = config.clone();
    bad["environment"]["inputs"][0]["sha256"] = Value::Null;
    assert!(transition::upgrade_config(&bad).is_err());
    assert!(transition::upgrade_config(&upgraded).is_err());

    let mut direct = json!({"schema":"chrono-historical-decode/v2","config_path":"config.json","original":{"config.json":config},"original_bytes":{},"candidate":{"config.json":upgraded},"profile":{"mappings":[{"from":"old","to":"new"}]}});
    original_bytes(&mut direct);
    let unchanged = direct.clone();
    assert_eq!(
        transition::convert_direct(&direct).unwrap(),
        json!({"values":direct["original"],"historical":{},"mappings":[{"from":"old","to":"new"}]})
    );
    assert_eq!(direct, unchanged);
    for case in ["protocol", "bytes", "candidate"] {
        let mut bad = direct.clone();
        match case {
            "protocol" => bad["schema"] = json!("unsupported"),
            "bytes" => bad["original_bytes"]["config.json"] = json!(b"{}".to_vec()),
            _ => {
                bad["candidate"]["config.json"]["environment"]["inputs"][0]["presence"] =
                    json!("absent")
            }
        }
        assert!(transition::convert_direct(&bad).is_err(), "{case}");
    }

    let workflow = json!({"schema_version":1,"migrations":[]});
    assert_eq!(
        transition::upgrade_workflow(&workflow).unwrap(),
        json!({"schema_version":3,"migrations":[],"historical_profiles":[]})
    );
    assert_eq!(workflow, json!({"schema_version":1,"migrations":[]}));
    assert!(transition::upgrade_workflow(&json!({"schema_version":3})).is_err());

    let identity = json!({"entry_path":"selector.json","effective_path":"effective.json","selection":{"path":"selector.json","platform":"native","config_path":"effective.json"}});
    let effective = json!({"schema_version":3,"registries":{"workflow":"workflow.json"},"environment":{"values":{"EMPTY":""}}});
    let mut selected = json!({"schema":"chrono-historical-decode/v3","config_path":"selector.json","config_bindings":{"base":identity,"candidate":identity},"original":{"selector.json":{"platforms":{"native":"effective.json"}},"effective.json":effective,"workflow.json":workflow},"original_bytes":{},"candidate":{"selector.json":{"platforms":{"native":"effective.json"}},"effective.json":effective,"workflow.json":{"schema_version":3,"migrations":[{"id":"new"}],"historical_profiles":[{"id":"profile"}],"integration":{"tests":["compatibility"]}}},"profile":{"mappings":[]}});
    original_bytes(&mut selected);
    let unchanged = selected.clone();
    assert_eq!(
        transition::convert_selected(&selected).unwrap(),
        json!({"values":selected["original"],"historical":{},"mappings":[]})
    );
    assert_eq!(selected, unchanged);
    for case in [
        "base",
        "candidate",
        "selection",
        "bytes",
        "effective",
        "workflow",
    ] {
        let mut bad = selected.clone();
        match case {
            "base" | "candidate" => {
                bad["config_bindings"].as_object_mut().unwrap().remove(case);
            }
            "selection" => {
                bad["config_bindings"]["base"]["selection"]["config_path"] = json!("other.json")
            }
            "bytes" => bad["original_bytes"]["workflow.json"] = json!(b"{}".to_vec()),
            "effective" => {
                bad["candidate"]["effective.json"]["environment"]["values"]["EMPTY"] =
                    json!("rewritten")
            }
            _ => bad["candidate"]["workflow.json"]["unexpected"] = json!(true),
        }
        assert!(transition::convert_selected(&bad).is_err(), "{case}");
    }
}
