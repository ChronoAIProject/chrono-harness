use super::*;

fn coverage() -> Value {
    json!({"schema":"chrono-input-coverage/v1",
    "required":[{"consumer":"judge:registration","domains":["git-facts","remote-response"]}],
    "rows":[
        {"consumer":"judge:registration","domain":"git-facts","state":"bound",
         "bindings":["registration-git-facts"],"reason":"registered executable and environment observations"},
        {"consumer":"judge:registration","domain":"remote-response","state":"not-applicable",
         "bindings":[],"reason":"fixture reads only local committed objects"}
    ]})
}

// Retain each endpoint's own configuration. Adopting coverage must not rewrite
// the old v3 config or require executing the previous judge binary.
fn run(h: &BoundHost) -> (i32, Value) {
    let snapshot = |oid: &str| {
        let config: Value =
            serde_json::from_slice(&facts::blob(h.host.root(), oid, CONFIG).unwrap()).unwrap();
        let mut files = serde_json::Map::new();
        for input in config["environment"]["inputs"].as_array().unwrap() {
            let observed = match fs::read(h.host.root().join(input["location"].as_str().unwrap())) {
                Ok(bytes) => json!({"bytes":bytes}),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({"absent":true}),
                Err(e) => panic!("snapshot input: {e}"),
            };
            files.insert(input["id"].as_str().unwrap().to_owned(), observed);
        }
        json!({"schema":"chrono-input-snapshot/v2","commit":oid,"config_path":CONFIG,
            "config_digest":chrono_harness::wire::digest(&config).unwrap(),"environment":{},"files":files})
    };
    write(
        h.host.root(),
        ".chrono-harness/state/coverage-inputs.json",
        &json!({"base":snapshot(&h.host.base),"candidate":snapshot(&h.host.candidate)}),
    );
    let mut context = h.host.context();
    context["retained_inputs"] = json!(".chrono-harness/state/coverage-inputs.json");
    h.host.run_context(context)
}

#[test]
fn optional_coverage_adopts_without_rewriting_legacy_v3_and_preserves_claim_boundary() {
    let mut h = BoundHost::new(false, "");
    let (exit, legacy, error) = h.run(false);
    assert_eq!(exit, 0, "{error}");
    assert!(
        legacy["judges"][0]["response"]["outputs"]
            .get("input_coverage")
            .is_none()
    );
    let old = facts::blob(h.host.root(), &h.host.base, CONFIG).unwrap();
    h.host.edit(CONFIG, |config| {
        config["input_closure"]["coverage"] = coverage()
    });
    let (exit, report) = run(&h);
    assert_eq!(exit, 0, "{}", report["findings"]);
    let result = &report["judges"][0]["response"]["outputs"]["input_coverage"];
    assert_eq!(result["scope"], "declared-domains");
    assert_eq!(result["declaration"], coverage());
    assert_eq!(result["completeness_proven"], false);
    assert_eq!(report["effective_inputs"]["completeness_proven"], false);
    assert_eq!(report["parity"]["status"], "unestablished");
    assert_eq!(
        facts::blob(h.host.root(), &h.host.base, CONFIG).unwrap(),
        old
    );
}

#[test]
fn incomplete_domain_accounting_cannot_claim_declared_complete() {
    for case in [
        "missing-row",
        "empty-binding",
        "empty-inputs",
        "unknown-binding",
        "wrong-consumer",
        "duplicate-row",
        "duplicate-binding",
        "missing-reason",
        "unresolved",
        "uncovered-binding",
        "empty-scope",
        "unknown-schema",
        "unknown-field",
        "duplicate-domain",
    ] {
        let mut h = BoundHost::new(false, "");
        h.host.edit(CONFIG, |config| {
            let mut c = coverage();
            match case {
                "missing-row" => {
                    c["rows"].as_array_mut().unwrap().pop();
                }
                "empty-binding" => c["rows"][0]["bindings"] = json!([]),
                "empty-inputs" => config["input_closure"]["bindings"][0]["inputs"] = json!([]),
                "unknown-binding" => c["rows"][0]["bindings"] = json!(["missing"]),
                "wrong-consumer" => {
                    config["input_closure"]["bindings"][0]["consumer"] = json!("test:other")
                }
                "duplicate-row" => {
                    let row = c["rows"][0].clone();
                    c["rows"].as_array_mut().unwrap().push(row);
                }
                "duplicate-binding" => {
                    c["rows"][0]["bindings"] =
                        json!(["registration-git-facts", "registration-git-facts"])
                }
                "missing-reason" => {
                    c["rows"][1].as_object_mut().unwrap().remove("reason");
                }
                "unresolved" => c["rows"][1]["state"] = json!("unresolved"),
                "uncovered-binding" => {
                    c["rows"][0]["state"] = json!("not-applicable");
                    c["rows"][0]["bindings"] = json!([]);
                }
                "empty-scope" => {
                    c["required"] = json!([]);
                    c["rows"] = json!([]);
                }
                "unknown-schema" => c["schema"] = json!("chrono-input-coverage/v999"),
                "unknown-field" => c["rows"][0]["complete"] = json!(true),
                "duplicate-domain" => {
                    c["required"][0]["domains"] = json!(["git-facts", "git-facts"])
                }
                _ => unreachable!(),
            }
            config["input_closure"]["coverage"] = c;
        });
        let (exit, report) = run(&h);
        assert_eq!(exit, 2, "{case}: {}", report["findings"]);
        assert!(
            finding(&report, "E_SCHEMA"),
            "{case}: {}",
            report["findings"]
        );
    }
}

#[test]
fn unresolved_draft_is_readable_but_not_activated_and_legacy_schemas_reject_coverage() {
    let mut h = BoundHost::new(false, "");
    h.host.edit(CONFIG, |config| {
        config["input_closure"]["coverage"] = coverage();
        config["input_closure"]["coverage"]["rows"][1]["state"] = json!("unresolved");
        config["input_closure"]["status"] = json!("incomplete");
        config["input_closure"]["unresolved"] = json!(["remote-response"]);
    });
    let (exit, report) = run(&h);
    assert_eq!(exit, 2, "{}", report["findings"]);
    assert!(finding(&report, "E_EVIDENCE_UNRESOLVED"));
    assert!(!finding(&report, "E_SCHEMA"), "{}", report["findings"]);
    for version in [1, 2] {
        let mut host = Host::new();
        host.edit(CONFIG, |config| {
            config["schema_version"] = json!(version);
            config["input_closure"]["coverage"] = coverage();
        });
        assert!(finding(&host.run().1, "E_SCHEMA"));
    }
}

#[test]
fn coverage_never_admits_unknown_consumers_or_invents_absence() {
    for case in ["unknown-consumer", "file-consumer", "false-absence"] {
        let mut h = BoundHost::new(false, "");
        h.host.edit(CONFIG, |config| {
            let mut c = coverage();
            if case == "false-absence" {
                c["rows"][0]["state"] = json!("absent");
            } else {
                let consumer = if case == "file-consumer" {
                    "file:document.txt"
                } else {
                    "judge:missing"
                };
                c["required"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"consumer":consumer,"domains":["none"]}));
                c["rows"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"consumer":consumer,"domain":"none",
                    "state":"not-applicable","bindings":[],"reason":"explicit fixture assertion"}));
            }
            config["input_closure"]["coverage"] = c;
        });
        let (exit, report) = run(&h);
        assert_eq!(exit, 1, "{case}: {}", report["findings"]);
        let code = if case == "false-absence" {
            "E_INPUT_UNDECLARED"
        } else {
            "E_REFERENCE"
        };
        assert!(finding(&report, code), "{case}: {}", report["findings"]);
    }
}

#[test]
fn absent_domain_uses_retained_file_absence_and_detects_new_bytes() {
    let mut h = BoundHost::new(false, "");
    let absent = fs::canonicalize(h.host.root())
        .unwrap()
        .join(".chrono-harness/state/credential.json");
    h.host.edit(CONFIG, |config| {
        config["environment"]["inputs"].as_array_mut().unwrap().push(json!({
            "id":"credential","location":absent,"presence":"absent"}));
        config["input_closure"]["bindings"].as_array_mut().unwrap().push(json!({
            "id":"credential-absence","consumer":"judge:registration","kind":"credential",
            "inputs":["input:credential"]}));
        let mut c = coverage();
        c["required"][0]["domains"].as_array_mut().unwrap().push(json!("credential"));
        c["rows"].as_array_mut().unwrap().push(json!({"consumer":"judge:registration","domain":"credential",
            "state":"absent","bindings":["credential-absence"],"reason":"no credential file is present"}));
        config["input_closure"]["coverage"] = c;
    });
    h.host.edit(".chrono-harness/FILEMAP.json", |map| {
        map["project_edges"].as_array_mut().unwrap().push(
            json!({"from":"input:credential","kind":"runtime-input","to":"judge:registration"}),
        )
    });
    let (exit, report) = run(&h);
    assert_eq!(exit, 0, "{}", report["findings"]);
    fs::write(&absent, b"not-secret-fixture").unwrap();
    let (exit, report) = run(&h);
    assert_eq!(exit, 2, "{}", report["findings"]);
    assert!(
        finding(&report, "E_EVIDENCE_UNRESOLVED"),
        "{}",
        report["findings"]
    );
}
