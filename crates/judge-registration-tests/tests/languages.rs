#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_judge_registration::Registrations;
use serde_json::{Value, json};

fn validate(projects: &Value) -> Result<(), String> {
    let mut values = support::values();
    values.insert(support::PROJECTS.into(), projects.clone());
    Registrations::load(&values, support::CONFIG).map(|_| ())
}

fn registry(version: u64) -> Value {
    let action = json!({"execute":{"operation":"host.action","tool":"host","argv":[]}});
    json!({
        "schema_version":version,"status":"active","owners":["p","t","s","st"],
        "projects":[
            {"id":"p","kind":"production","test_project":"t","actions":action},
            {"id":"t","kind":"test","tests_for":"p","actions":action}
        ],
        "scripts":[
            {"id":"s","path":"arbitrary/input.bin","test_script":"st","actions":action},
            {"id":"st","path":"elsewhere/assertions.data","tests_for":"s","actions":action}
        ]
    })
}

#[test]
fn v2_requires_language_for_every_project_and_script() {
    let mut values = registry(2);
    for collection in ["projects", "scripts"] {
        for row in values[collection].as_array_mut().unwrap() {
            row["language"] = json!("host-language");
        }
    }
    validate(&values).unwrap();
    for collection in ["projects", "scripts"] {
        for index in 0..2 {
            let mut missing = values.clone();
            missing[collection][index]
                .as_object_mut()
                .unwrap()
                .remove("language");
            assert!(
                validate(&missing)
                    .unwrap_err()
                    .contains("missing field language")
            );
            for bad in [Value::Null, json!(""), json!(["rust"]), json!(2)] {
                let mut invalid = values.clone();
                invalid[collection][index]["language"] = bad;
                assert!(validate(&invalid).unwrap_err().contains("language"));
            }
        }
    }
}

#[test]
fn v1_keeps_its_original_contract_and_rejects_v2_fields() {
    let mut values = registry(1);
    validate(&values).unwrap();
    values["projects"][0]["language"] = json!("rust");
    assert!(
        validate(&values)
            .unwrap_err()
            .contains("unknown field language")
    );
    values["schema_version"] = json!(3);
    assert!(
        validate(&values)
            .unwrap_err()
            .contains("unsupported schema_version")
    );
}
