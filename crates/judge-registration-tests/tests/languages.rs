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

#[test]
fn retirement_structure_accepts_joint_pair_removal_and_rejects_a_surviving_producer() {
    let old = support::values();
    let a = support::load(&old);
    let mut candidate = old.clone();
    candidate.get_mut(support::PROJECTS).unwrap()["projects"] = json!([]);
    candidate.get_mut(support::WORKFLOW).unwrap()["retirements"] = json!([
        {"kind":"test","id":"t","replacement":null,"reason":"pair retired"},
        {"kind":"project","id":"p","replacement":null,"reason":"product retired"}
    ]);
    let b = support::load(&candidate);
    assert_eq!(
        chrono_judge_registration::validate_test_retirement(&a, &b, "test:t").unwrap(),
        None
    );
    candidate.get_mut(support::PROJECTS).unwrap()["projects"] =
        json!([old[support::PROJECTS]["projects"][0]]);
    let b = support::load(&candidate);
    assert!(
        chrono_judge_registration::validate_test_retirement(&a, &b, "test:t")
            .unwrap_err()
            .contains("joint producer/test removal required")
    );
}

#[test]
fn retirement_structure_resolves_grouped_alias_owner_and_requires_producer_declaration() {
    let mut old = support::values();
    let mut grouped = old[support::PROJECTS]["projects"][1]["actions"]["execute"].clone();
    grouped["operation"] = json!("execute.grouped");
    old.get_mut(support::PROJECTS).unwrap()["projects"][1]["actions"]["grouped"] = grouped;
    old.get_mut(support::PROJECTS).unwrap()["projects"][1]["test_groups"] =
        json!({"t":"execute","opaque/suffix":"grouped"});
    let a = support::load(&old);
    let mut candidate = old.clone();
    candidate.get_mut(support::PROJECTS).unwrap()["projects"] = json!([]);
    candidate.get_mut(support::WORKFLOW).unwrap()["retirements"] = json!([
        {"kind":"test","id":"opaque/suffix","replacement":null,"reason":"pair retired"}
    ]);
    let b = support::load(&candidate);
    assert!(
        chrono_judge_registration::validate_test_retirement(&a, &b, "test:opaque/suffix")
            .unwrap_err()
            .contains("missing/ambiguous project:p")
    );
    candidate.get_mut(support::WORKFLOW).unwrap()["retirements"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"project","id":"p","replacement":null,"reason":"product retired"}));
    let b = support::load(&candidate);
    assert_eq!(
        chrono_judge_registration::validate_test_retirement(&a, &b, "test:opaque/suffix").unwrap(),
        None
    );
}

#[test]
fn retirement_structure_requires_a_declared_unique_current_replacement() {
    let old = support::values();
    let a = support::load(&old);
    let mut candidate = old.clone();
    candidate.get_mut(support::PROJECTS).unwrap()["owners"]
        .as_array_mut()
        .unwrap()
        .push(json!("current"));
    candidate.get_mut(support::PROJECTS).unwrap()["projects"][1]["id"] = json!("current");
    candidate.get_mut(support::PROJECTS).unwrap()["projects"][0]["test_project"] = json!("current");
    let b = support::load(&candidate);
    assert!(
        chrono_judge_registration::validate_test_retirement(&a, &b, "test:t")
            .unwrap_err()
            .contains("E_REQUIRED_TEST_REMOVED")
    );
    candidate.get_mut(support::WORKFLOW).unwrap()["retirements"] = json!([
        {"kind":"test","id":"t","replacement":"current","reason":"current consumer migrated"}
    ]);
    let b = support::load(&candidate);
    assert_eq!(
        chrono_judge_registration::validate_test_retirement(&a, &b, "test:t").unwrap(),
        Some("test:current".into())
    );
    let mut ambiguous = candidate.clone();
    support::script_pair(&mut ambiguous, "current");
    let b = support::load(&ambiguous);
    assert!(
        chrono_judge_registration::validate_test_retirement(&a, &b, "test:t")
            .unwrap_err()
            .contains("replacement missing/ambiguous")
    );
    candidate.get_mut(support::PROJECTS).unwrap()["projects"] = json!([]);
    let b = support::load(&candidate);
    assert!(
        chrono_judge_registration::validate_test_retirement(&a, &b, "test:t")
            .unwrap_err()
            .contains("replacement missing/ambiguous")
    );
}
