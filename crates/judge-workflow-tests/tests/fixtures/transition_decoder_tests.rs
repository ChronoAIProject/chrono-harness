use super::transition_decoder::convert;
use serde_json::Value;

pub fn check(request: &Value) {
    let result =
        convert(request).expect("current transition fixture must convert its actual input");
    let filemap = &result["values"][".chrono-harness/FILEMAP.json"];
    assert_eq!(filemap["schema_version"], 2);
    assert_eq!(
        filemap["execution_plans"]["test:t"]["operations"],
        serde_json::json!(["prepare.p", "execute.t"])
    );
    let mut invalid = request.clone();
    invalid["profile_value"]["schema"] = "unsupported".into();
    assert!(
        convert(&invalid).is_err(),
        "unsupported profile was accepted"
    );
}

#[cfg(test)]
mod contract {
    use super::*;
    use serde_json::json;

    fn request() -> Value {
        let filemap = ".chrono-harness/FILEMAP.json";
        let projects = ".chrono-harness/projects.json";
        let config = ".chrono-harness/config.json";
        let legacy = json!({"id":"legacy","path":"legacy","actions":{}});
        let original = json!({
            config: {"registries":{"filemap":filemap,"projects":projects}},
            filemap: {"schema_version":1,"files":[]},
            projects: {"scripts":[legacy]}
        });
        let original_bytes: serde_json::Map<_, _> = original
            .as_object()
            .unwrap()
            .iter()
            .map(|(path, value)| (path.clone(), json!(serde_json::to_vec(value).unwrap())))
            .collect();
        let scoped = json!({"schema":"chrono-ci-check/v1","policy":{
            "filemap":filemap,"projects":projects,
            "bindings":{"test:t":["prepare.p","execute.t"],"test:legacy":["legacy.old"]},
            "operation_timeout_seconds":15,"operation_output_limit_bytes":4096
        }});
        json!({
            "schema":"chrono-historical-decode/v1","config_path":config,
            "original":original,"original_bytes":original_bytes,
            "profile_value":scoped,"profile_bytes":serde_json::to_vec(&scoped).unwrap(),
            "profile":{
                "id":"chrono-ci-check/v1",
                "legacy_records":[{"id":"legacy","collection":"scripts","definition":legacy,
                    "nodes":["script:legacy","test:legacy"]}],
                "mappings":[{"from":"script:legacy","to":"script:decoder"}]
            }
        })
    }

    #[test]
    fn converts_ordered_plans_and_retains_original_definitions_without_mutation() {
        let input = request();
        let before = input.clone();
        check(&input);
        let result = convert(&input).unwrap();
        assert_eq!(input, before);
        assert_eq!(
            result["values"][".chrono-harness/projects.json"]["scripts"],
            json!([])
        );
        let legacy = &input["profile"]["legacy_records"][0]["definition"];
        for node in ["script:legacy", "test:legacy"] {
            assert_eq!(
                result["historical"][node],
                json!([["script:legacy", legacy]])
            );
        }
        assert_eq!(result["mappings"], input["profile"]["mappings"]);
        assert_eq!(
            result["values"][".chrono-harness/FILEMAP.json"]["execution_plans"]["test:t"],
            json!({"operations":["prepare.p","execute.t"],"timeout_seconds":15,"output_limit_bytes":4096})
        );
    }

    #[test]
    fn rejects_bad_protocol_bytes_definitions_and_ordered_bindings() {
        let cases = [
            ("/schema", json!("unsupported")),
            ("/profile/id", json!("unsupported")),
            ("/profile_bytes", json!([b'{'])),
            (
                "/original_bytes/.chrono-harness~1FILEMAP.json",
                json!([b'{']),
            ),
            ("/profile/legacy_records/0/definition/id", json!("other")),
            ("/profile_value/policy/bindings/test:t", json!([])),
            (
                "/profile_value/policy/bindings/test:t",
                json!(["prepare.p", "prepare.p"]),
            ),
        ];
        for (pointer, value) in cases {
            let mut input = request();
            *input.pointer_mut(pointer).unwrap() = value;
            if pointer.starts_with("/profile_value/") {
                input["profile_bytes"] =
                    json!(serde_json::to_vec(&input["profile_value"]).unwrap());
            }
            assert!(convert(&input).is_err(), "accepted {pointer}");
        }
    }
}
