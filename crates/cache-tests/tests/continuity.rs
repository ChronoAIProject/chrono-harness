use super::*;

fn probe_config(root: &Path, config: &mut Value, args: Value) {
    config["inputs"]["compiler"] = json!({"kind":"command","command":{
        "program":env!("CARGO_BIN_EXE_chrono-cache-test-probe"),"args":args,"env":{},
        "timeout_seconds":5,"output_limit_bytes":4096},"inherit":[],"result":"stdout"});
    fs::write(root.join(".chrono-harness/cache.json"), serde_json::to_vec(config).unwrap()).unwrap();
}

fn plan_args(root: &Path, output: &str) -> Vec<String> {
    ["plan", "--host-root", root.to_str().unwrap(), "--config", ".chrono-harness/cache.json",
        "--consumer", "check.one", "--plan-output", output].map(str::to_owned).to_vec()
}

#[test]
fn exact_plan_attempt_reentry_does_not_dispatch_another_probe() {
    let (root, mut config) = fixture();
    let count = root.path().join("invocations");
    probe_config(root.path(), &mut config, json!(["count", count]));
    let args = plan_args(root.path(), ".chrono-harness/state/cache/one/plan.json");
    let binary = env!("CARGO_BIN_EXE_chrono-cache-test-cli");
    let first = Command::new(binary).args(&args).output().unwrap();
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let second = Command::new(binary).args(&args).output().unwrap();
    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));
    assert_eq!(fs::read(&count).unwrap(), b"x");
    assert_eq!(first.stdout, second.stdout);
}
