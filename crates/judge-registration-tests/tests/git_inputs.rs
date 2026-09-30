use super::*;

#[test]
fn full_consumer_adopts_guard_without_rewriting_history_and_rejects_drift_before_git() {
    let mut h = BoundHost::new(false, "");
    let root = fs::canonicalize(h.host.root()).unwrap();
    let config_file = root.join(".chrono-harness/state/global.gitconfig");
    fs::write(&config_file, "[core]\n  abbrev = 12\n").unwrap();
    let old = facts::blob(h.host.root(), &h.host.base, CONFIG).unwrap();
    h.host.edit(CONFIG, |config| {
        config["facts_git"]["guard"] = json!({"schema":"chrono-git-inputs/v1","inputs":["global-config"]});
        config["environment"]["inputs"].as_array_mut().unwrap().push(json!({
            "id":"global-config","location":config_file,"presence":"present","sha256":sha256(&fs::read(&config_file).unwrap())}));
        config["environment"]["values"]["GIT_CONFIG_GLOBAL"] = json!(config_file);
        config["input_closure"]["bindings"][0]["inputs"].as_array_mut().unwrap().push(json!("input:global-config"));
    });
    h.host.edit(".chrono-harness/FILEMAP.json", |map| {
        map["project_edges"].as_array_mut().unwrap().push(json!({
            "from":"input:global-config","kind":"runtime-input","to":"judge:registration"}));
    });
    let (exit, report) = run_with_endpoint_inputs(&h);
    assert_eq!(exit, 0, "{}", report["findings"]);
    let inputs = &report["git_facts"]["inputs"];
    assert_eq!(
        inputs["files"]["global-config"]["sha256"],
        sha256(&fs::read(&config_file).unwrap())
    );
    assert_eq!(inputs["completeness_proven"], false);
    assert_eq!(
        report["judges"][0]["response"]["outputs"]["git_facts"]["inputs"],
        *inputs
    );
    assert_eq!(
        facts::blob(h.host.root(), &h.host.base, CONFIG).unwrap(),
        old
    );

    fs::write(&config_file, "[core]\n  abbrev = 9\n").unwrap();
    fs::write(&h.trace, "").unwrap();
    let (exit, report) = run_with_endpoint_inputs(&h);
    assert_eq!(exit, 2, "{report}");
    assert!(report.to_string().contains("global-config"), "{report}");
    assert_eq!(h.trace(), "", "mismatched input launched the selected Git");
}

#[test]
fn initial_inventory_uses_the_same_git_input_guard() {
    let mut h = BoundHost::new(true, "");
    let mut config: Value =
        serde_json::from_slice(&fs::read(h.host.root().join(CONFIG)).unwrap()).unwrap();
    config["facts_git"]["guard"] =
        json!({"schema":"chrono-git-inputs/v1","inputs":["facts-git-bytes"]});
    write(h.host.root(), CONFIG, &config);
    amend_initial(&mut h.host);
    let (exit, report, error) = h.run(true);
    assert_eq!(exit, 0, "{error} {}", report["findings"]);
    assert_eq!(report["governance"], "not-evaluated");
    assert_eq!(
        report["git_facts"]["inputs"]["files"]["facts-git-bytes"]["presence"],
        "present"
    );
}
