#[path = "../../judge-projects-tests/tests/support/host.rs"]
mod host;
#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::sha256;
use host::Host;
use serde_json::{Value, json};
use std::{fs, process::Command};
use support::*;

fn fixture() -> Host {
    let mut h = Host::new(true);
    for (id, deps) in [
        ("cost", vec!["registration", "filemap"]),
        ("mixed", vec!["registration", "filemap", "cost"]),
        (
            "workflow",
            vec![
                "registration",
                "filemap",
                "routes",
                "projects",
                "cost",
                "mixed",
            ],
        ),
    ] {
        let bytes =
            fs::read(source().join(format!("crates/judge-{id}/target/debug/chrono-judge-{id}")))
                .unwrap();
        let path = format!(".chrono-harness/bin/chrono-judge-{id}");
        fs::write(h.root().join(&path), &bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(h.root().join(&path), fs::Permissions::from_mode(0o755)).unwrap();
        }
        h.values.get_mut(JUDGES).unwrap()["judges"].as_array_mut().unwrap().push(json!({"id":id,"executable":path,"version":"0.1.0","sha256":sha256(&bytes),"argv":["--protocol","chrono-judge/v1"],"selector":"every-delta","after":deps,"modes":["evaluate"]}));
    }
    h.values.get_mut(JUDGES).unwrap()["migration_validator"] = "workflow".into();
    let w = h.values.get_mut(WORKFLOW).unwrap();
    w["integration"]["bind"] = json!([
        "base",
        "candidate_tree",
        "registry_digest",
        "executables",
        "tools",
        "environment",
        "effective_inputs",
        "required_tests",
        "results_digest"
    ]);
    w["integration"]["tests"] = json!(["t"]);
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("p/product.py"), "def double(n): return 2*n\n").unwrap();
    h.save();
    h
}
const OLD_TARGET: &str = ".chrono-harness/native old.json";
const NEW_TARGET: &str = ".chrono-harness/native current.json";

fn native_v3(h: &mut Host) {
    v2(h);
    let git =
        fs::canonicalize(chrono_harness::resolve_program(&h.root(), "git", None).unwrap()).unwrap();
    let version = Command::new(&git).arg("--version").output().unwrap();
    assert!(version.status.success());
    let cfg = h.values.get_mut(CONFIG).unwrap();
    cfg["schema_version"] = json!(3);
    cfg["facts_git"] = json!({"tool":"git","input":"git-bytes"});
    cfg["tools"].as_array_mut().unwrap().push(json!({"id":"git","program":git,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim_end()}));
    cfg["environment"]["inputs"].as_array_mut().unwrap().push(json!({"id":"git-bytes","location":git,"presence":"present","sha256":sha256(&fs::read(&git).unwrap())}));
    cfg["environment"]["values"]["FACTS_GIT"] = json!(git);
    // The actual script test below consumes the declared Git, Python and data
    // observations. Use admissible runtime-input edges to that script, not
    // environment/input/tool-to-judge edges (which FILEMAP does not permit).
    let inputs = json!([
        "tool:git",
        "input:git-bytes",
        "tool:python",
        "input:interpreter",
        "input:data",
        "environment:FACTS_GIT",
        "environment:DECLARED_EMPTY",
        "environment:DECLARED_ABSENT",
        "environment:EMPTY",
        "environment:TMPDIR"
    ]);
    cfg["input_closure"]["bindings"] =
        json!([{"id":"script-inputs","consumer":"script:t","kind":"execution","inputs":inputs}]);
    h.values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .extend(
            inputs
                .as_array()
                .unwrap()
                .iter()
                .map(|input| edge(input.as_str().unwrap(), "runtime-input", "script:t")),
        );
    let test = h.root().join("t/check.py");
    let mut code = fs::read_to_string(&test).unwrap();
    code.push_str(&format!("\nimport subprocess\nassert subprocess.run([os.environ['FACTS_GIT'], '--version'], stdout=subprocess.PIPE).returncode == 0\nassert os.environ.get('DECLARED_EMPTY') == '' and 'DECLARED_ABSENT' not in os.environ\nprint('data-present', os.path.exists({:?}))\n", h.external.path().to_str().unwrap()));
    fs::write(test, code).unwrap();
}

fn select_config(h: &mut Host, target: &str) {
    let old = chrono_harness::facts::registry_identity(&h.values, CONFIG)
        .unwrap()
        .effective_path;
    let cfg = h.values.remove(&old).unwrap();
    if old != CONFIG {
        fs::remove_file(h.root().join(&old)).unwrap();
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .retain(|f| f["path"] != old);
    }
    if target == CONFIG {
        h.values.insert(CONFIG.into(), cfg);
    } else {
        h.values.insert(target.into(), cfg);
        h.values.insert(CONFIG.into(), json!({"schema":"chrono-git-configs/v1","platforms":{format!("{}-{}",std::env::consts::OS,std::env::consts::ARCH):target}}));
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(file(target, json!([])));
    }
}

fn pair_captures(h: &Host, base: &str, candidate: &str) -> Value {
    let out = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
        .current_dir("/")
        .args([
            "pair",
            "--host-root",
            h.root().to_str().unwrap(),
            "--base-snapshot",
            &format!(".chrono-harness/state/{base}.json"),
            "--candidate-snapshot",
            &format!(".chrono-harness/state/{candidate}.json"),
            "--output",
            &format!(".chrono-harness/state/paired-{base}-{candidate}.json"),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn snapshots(h: &Host) -> Value {
    let invoke = |args: &[&str]| {
        let out = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
            .current_dir("/")
            .env("DECLARED_EMPTY", "")
            .env_remove("DECLARED_ABSENT")
            .args(args)
            .args(["--host-root", h.root().to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    git(&h.root(), &["checkout", "--detach", &h.base]);
    invoke(&[
        "capture",
        "--config",
        CONFIG,
        "--commit",
        &h.base,
        "--output",
        ".chrono-harness/state/base.json",
    ]);
    git(&h.root(), &["checkout", "--detach", &h.candidate]);
    invoke(&[
        "capture",
        "--config",
        CONFIG,
        "--commit",
        &h.candidate,
        "--output",
        ".chrono-harness/state/candidate.json",
    ]);
    invoke(&[
        "pair",
        "--base-snapshot",
        ".chrono-harness/state/base.json",
        "--candidate-snapshot",
        ".chrono-harness/state/candidate.json",
        "--output",
        ".chrono-harness/state/paired.json",
    ])
}
fn check(h: &Host, pair: &Value) -> (i32, Value) {
    h.run_with_context(
        |v| *v = pair.clone(),
        |ctx| {
            ctx["schema_version"] = 2.into();
            ctx["run_kind"] = "integration".into()
        },
    )
}
#[test]
fn produced_snapshots_drive_actual_seven_judge_execution_without_inline_input_bytes() {
    let h = fixture();
    let pair = snapshots(&h);
    assert!(pair["base"]["files"]["interpreter"].get("bytes").is_none());
    let (exit, r) = check(&h, &pair);
    assert_eq!(exit, 0, "{}", r["findings"]);
    assert_eq!(r["tests"]["tests"]["test:t"], "passed");
    assert!(
        h.root()
            .join(".chrono-harness/state/integration.json")
            .is_file()
    );
    let (exit, inline) = h.run_with_context(
        |inputs| {
            let interpreter = json!({"bytes":fs::read(&h.tool).unwrap()});
            for endpoint in ["base", "candidate"] {
                inputs[endpoint]["files"]["interpreter"] = interpreter.clone();
            }
        },
        |ctx| {
            ctx["schema_version"] = 2.into();
            ctx["run_kind"] = "integration".into()
        },
    );
    assert_eq!(exit, 0, "{}", inline["findings"]);
    assert_eq!(r["effective_inputs"], inline["effective_inputs"]);
}

#[test]
fn malformed_misbound_missing_and_changed_retained_inputs_fail_before_execution() {
    let h = fixture();
    let pair = snapshots(&h);
    let reject = |v: &Value, expected: &str| {
        let (exit, r) = check(&h, v);
        assert_ne!(exit, 0, "{r:#}");
        assert!(r.to_string().contains(expected), "expected {expected}: {r}");
        assert!(r["tests"].is_null());
        assert!(!h.root().join(".chrono-harness/state/order").exists());
    };
    let mut bad = pair.clone();
    bad["base"]["commit"] = h.candidate.clone().into();
    reject(&bad, "wrong endpoint");
    bad = pair.clone();
    bad["base"]["config_digest"] = "0".repeat(64).into();
    reject(&bad, "snapshot configuration binding");
    bad = pair.clone();
    bad["base"]["files"]["extra"] = bad["base"]["files"]["data"].clone();
    reject(&bad, "unregistered retained input extra");
    bad = pair.clone();
    bad["base"]["files"]["data"]["bytes"] = json!([]);
    reject(&bad, "exactly one bytes or blob");
    bad = pair.clone();
    bad["base"]["files"]["data"]["blob"] = "/outside".into();
    reject(&bad, "E_INPUT_BLOB");
    bad = pair.clone();
    bad["base"]["files"]["data"]["length"] = (-1).into();
    reject(&bad, "E_INPUT_BLOB");
    bad = pair.clone();
    bad["base"]["files"]["data"]["blob"] = ".chrono-harness/state/missing".into();
    reject(&bad, "E_INPUT_BLOB");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            h.external.path(),
            h.root().join(".chrono-harness/state/link"),
        )
        .unwrap();
        bad = pair.clone();
        bad["base"]["files"]["data"]["blob"] = ".chrono-harness/state/link".into();
        reject(&bad, "E_INPUT_BLOB");
    }
    let blob = h
        .root()
        .join(pair["base"]["files"]["data"]["blob"].as_str().unwrap());
    let old = fs::read(&blob).unwrap();
    fs::write(&blob, b"damaged").unwrap();
    reject(&pair, "E_INPUT_BLOB");
    fs::write(blob, old).unwrap();
    fs::write(h.external.path(), b"candidate changed after capture").unwrap();
    reject(&pair, "candidate input changed: data");
}

#[test]
fn input_larger_than_judge_json_bound_remains_a_small_request_and_is_actually_validated() {
    let mut h = fixture();
    // Independent SHA-256 oracle: Python hashlib over 65 successive 1 MiB zero blocks.
    let digest = "25631f11bd18756ec0029380ec886af0c8824dc6b2706bbdb1d9451c7cf45f42";
    let length = 65 * 1024 * 1024;
    fs::File::create(h.external.path())
        .unwrap()
        .set_len(length)
        .unwrap();
    h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|v| v["id"] == "data")
        .unwrap()["sha256"] = digest.into();
    h.values.get_mut(CONFIG).unwrap()["protocol"]["timeout_seconds"] = 120.into();
    h.save();
    h.base = h.candidate.clone();
    fs::write(h.root().join("p/product.py"), "def double(n): return n+n\n").unwrap();
    h.save();
    let pair = snapshots(&h);
    assert_eq!(pair["base"]["files"]["data"]["length"], length);
    assert!(serde_json::to_vec(&pair).unwrap().len() < 16384);
    let (exit, r) = check(&h, &pair);
    assert_eq!(exit, 0, "{r}");
    assert_eq!(
        r["effective_inputs"]["endpoints"]["candidate"]["files"]["data"]["sha256"],
        digest
    );
    assert_eq!(r["tests"]["tests"]["test:t"], "passed");
}

fn v2(h: &mut Host) {
    h.tool = fs::canonicalize(&h.tool).unwrap();
    for tool in h.values.get_mut(CONFIG).unwrap()["tools"]
        .as_array_mut()
        .unwrap()
    {
        tool["program"] = json!(fs::canonicalize(tool["program"].as_str().unwrap()).unwrap());
    }
    h.values.get_mut(CONFIG).unwrap()["schema_version"] = json!(2);
    for row in h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        row["presence"] = json!("present");
        row["location"] = json!(fs::canonicalize(row["location"].as_str().unwrap()).unwrap());
    }
}
fn data_state(h: &mut Host, absent: bool) {
    let row = h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["id"] == "data")
        .unwrap();
    row["presence"] = json!(if absent { "absent" } else { "present" });
    if absent {
        row.as_object_mut().unwrap().remove("sha256");
        if h.external.path().exists() {
            fs::remove_file(h.external.path()).unwrap();
        }
    } else {
        row["sha256"] = json!(sha256(b""));
        fs::write(h.external.path(), b"").unwrap();
    }
}
fn capture(h: &Host, name: &str) -> Value {
    let out = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
        .env("DECLARED_EMPTY", "")
        .env_remove("DECLARED_ABSENT")
        .args([
            "capture",
            "--host-root",
            h.root().to_str().unwrap(),
            "--config",
            CONFIG,
            "--commit",
            &h.candidate,
            "--output",
            &format!(".chrono-harness/state/{name}.json"),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
#[test]
fn absence_to_empty_and_back_selects_only_explicit_input_consumers_including_old_edges() {
    for absent in [true, false] {
        let mut h = fixture();
        v2(&mut h);
        data_state(&mut h, absent);
        h.save();
        h.base = h.candidate.clone();
        let a = capture(&h, "before");
        data_state(&mut h, !absent);
        if !absent {
            // Removal keeps the old-only input dependency alive for this DELTA.
            h.values.get_mut(FM).unwrap()["project_edges"]
                .as_array_mut()
                .unwrap()
                .retain(|e| e["from"] != "input:data");
        }
        h.save();
        let b = capture(&h, "after");
        let (exit, r) = check(&h, &json!({"base":a,"candidate":b}));
        assert_eq!(exit, 0, "{}", r["findings"]);
        assert_eq!(r["tests"]["tests"]["test:t"], "passed");
        assert!(r["impact"].to_string().contains("input:data"));
        let files = &r["effective_inputs"]["endpoints"];
        assert_ne!(
            files["base"]["files"]["data"],
            files["candidate"]["files"]["data"]
        );
        assert_eq!(r["effective_inputs"]["completeness_proven"], false);
    }
}
#[test]
fn unchanged_absence_and_disconnected_changes_do_not_select_tests() {
    for disconnected in [false, true] {
        let mut h = fixture();
        v2(&mut h);
        data_state(&mut h, true);
        if disconnected {
            h.values.get_mut(FM).unwrap()["project_edges"]
                .as_array_mut()
                .unwrap()
                .retain(|e| e["from"] != "input:data");
        }
        h.save();
        h.base = h.candidate.clone();
        let a = capture(&h, "before");
        if disconnected {
            data_state(&mut h, false);
        } else {
            fs::write(h.root().join("doc.txt"), "documentation only\n").unwrap();
        }
        h.save();
        let b = capture(&h, "after");
        let (exit, r) = check(&h, &json!({"base":a,"candidate":b}));
        assert_eq!(exit, 0, "{}", r["findings"]);
        assert_eq!(r["tests"]["tests"], json!({}));
        assert!(!h.root().join(".chrono-harness/state/order").exists());
    }
}
#[test]
fn retained_absence_mismatch_missing_snapshot_and_v1_downgrade_fail_before_operations() {
    let mut h = fixture();
    v2(&mut h);
    data_state(&mut h, true);
    h.save();
    h.base = h.candidate.clone();
    let a = capture(&h, "before");
    fs::write(h.root().join("p/product.py"), "def double(n): return n+n\n").unwrap();
    h.save();
    let b = capture(&h, "after");
    let pair = json!({"base":a,"candidate":b});
    for case in [
        "retained-empty",
        "retained-malformed",
        "missing-schema",
        "v1-schema",
        "actual-present",
        "actual-directory",
        "actual-symlink",
    ] {
        let mut bad = pair.clone();
        match case {
            "retained-empty" => bad["base"]["files"]["data"] = json!({"bytes":[]}),
            "retained-malformed" => {
                bad["candidate"]["files"]["data"] = json!({"absent":true,"bytes":[]})
            }
            "missing-schema" => {
                bad["candidate"].as_object_mut().unwrap().remove("schema");
            }
            "v1-schema" => bad["candidate"]["schema"] = json!("chrono-input-snapshot/v1"),
            "actual-present" => fs::write(h.external.path(), []).unwrap(),
            "actual-directory" => fs::create_dir(h.external.path()).unwrap(),
            "actual-symlink" => {
                #[cfg(unix)]
                std::os::unix::fs::symlink(
                    h.external.path().with_extension("missing"),
                    h.external.path(),
                )
                .unwrap();
                #[cfg(not(unix))]
                continue;
            }
            _ => unreachable!(),
        }
        let (exit, r) = check(&h, &bad);
        assert_ne!(exit, 0, "{case}: {r}");
        assert!(r["tests"].is_null(), "{case}: {r}");
        assert!(!h.root().join(".chrono-harness/state/order").exists());
        if case == "actual-directory" {
            fs::remove_dir(h.external.path()).unwrap();
        } else if case.starts_with("actual-") {
            fs::remove_file(h.external.path()).unwrap();
        }
    }
    let (exit, r) = check(&h, &pair);
    assert_eq!(exit, 0, "{}", r["findings"]);
}
#[test]
fn operation_creating_an_absent_input_fails_post_execution_validation() {
    let mut h = fixture();
    v2(&mut h);
    data_state(&mut h, true);
    h.values.get_mut(PROJECTS).unwrap()["scripts"][1]["actions"]["execute"]["argv"] = json!([
        "-c",
        format!(
            "open({:?},'w').write('created')",
            h.external.path().to_str().unwrap()
        )
    ]);
    h.save();
    h.base = h.candidate.clone();
    let a = capture(&h, "before");
    fs::write(h.root().join("p/product.py"), "def double(n): return n+n\n").unwrap();
    h.save();
    let b = capture(&h, "after");
    let (exit, r) = check(&h, &json!({"base":a,"candidate":b}));
    assert_ne!(exit, 0, "{r}");
    assert!(
        r["findings"]
            .to_string()
            .contains("candidate input changed: data"),
        "{r}"
    );
    assert_eq!(fs::read(h.external.path()).unwrap(), b"created");
}

#[test]
fn legacy_null_and_unversioned_retained_absence_never_authorize_missing_input() {
    let mut h = fixture();
    h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["id"] == "data")
        .unwrap()["sha256"] = Value::Null;
    h.save();
    h.base = h.candidate.clone();
    fs::remove_file(h.external.path()).unwrap();
    let (exit, r) = h.run(|pair| {
        for endpoint in ["base", "candidate"] {
            pair[endpoint]["files"]["data"] = json!({"absent":true});
        }
    });
    assert_ne!(exit, 0, "{r}");
    assert!(
        r["findings"]
            .to_string()
            .contains("retained input digest/presence mismatch: data"),
        "{r}"
    );
    assert!(r["tests"].is_null());
}

#[test]
fn config_presence_upgrade_requires_registered_migration_evidence() {
    let mut h = fixture();
    h.base = h.candidate.clone();
    let a = capture(&h, "before");
    v2(&mut h);
    h.save();
    let b = capture(&h, "after");
    let (exit, r) = check(&h, &json!({"base":a,"candidate":b}));
    assert_ne!(exit, 0, "{r}");
    assert!(
        r["findings"].to_string().contains("E_MIGRATION_EVIDENCE"),
        "{}",
        r["findings"]
    );
    assert!(
        !h.root()
            .join(".chrono-harness/state/integration.json")
            .exists()
    );
}

// A host-owned decoder checks an exact v1 -> v2 config migration. The historical
// view stays v1: converting its inputs would invalidate the retained snapshot.
fn migration_fixture() -> (Host, Value) {
    migration_fixture_mode(false)
}
fn migration_fixture_mode(selected: bool) -> (Host, Value) {
    let mut h = fixture();
    v2(&mut h);
    h.values.get_mut(CONFIG).unwrap()["schema_version"] = json!(1);
    for input in h.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        input.as_object_mut().unwrap().remove("presence");
    }
    let decoder = r#"import copy,json,sys
def upgrade(config):
    if config['schema_version'] != 1:
        raise ValueError('source config must be v1')
    out = copy.deepcopy(config)
    out['schema_version'] = 2
    for item in out['environment']['inputs']:
        if item['sha256'] is None:
            raise ValueError('unbound digest is not absence')
        item['presence'] = 'present'
    return out
def convert(req):
    assert req['schema'] == 'chrono-historical-decode/v2'
    for path, value in req['original'].items():
        assert json.loads(bytes(req['original_bytes'][path])) == value
    assert upgrade(req['original'][req['config_path']]) == req['candidate'][req['config_path']]
    return dict(values=req['original'],historical={},mappings=req['profile']['mappings'])
if __name__ == '__main__':
    print(json.dumps(convert(json.load(sys.stdin))))
"#;
    let tests = r#"import runpy
upgrade = runpy.run_path('.chrono-harness/decoder.py')['upgrade']
source = dict(schema_version=1,environment=dict(inputs=[dict(id='empty',location='x',sha256='e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855')]))
expected = dict(schema_version=2,environment=dict(inputs=[dict(id='empty',location='x',sha256='e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',presence='present')]))
assert upgrade(source) == expected
assert source['schema_version'] == 1 and 'presence' not in source['environment']['inputs'][0]
source['environment']['inputs'][0]['sha256'] = None
try:
    upgrade(source)
except ValueError:
    pass
else:
    raise AssertionError('unbound digest became absence')
print('config migration compatibility checked')
"#;
    // Selected targets remain full-v3. The actual selected schema transition
    // is workflow v1 -> v3, certified by the candidate's finite decoder/test.
    let selected_decoder = r#"import copy,json,sys
def upgrade(workflow):
    if workflow['schema_version'] != 1:
        raise ValueError('source workflow must be v1')
    out = copy.deepcopy(workflow)
    out['schema_version'] = 3
    out['historical_profiles'] = []
    return out
def convert(req):
    assert req['schema'] == 'chrono-historical-decode/v3'
    bindings = req['config_bindings']
    for endpoint, key in [('base','original'),('candidate','candidate')]:
        identity = bindings[endpoint]
        assert identity['entry_path'] == req['config_path']
        entry = req[key][identity['entry_path']]
        selection = identity['selection']
        assert selection['path'] == identity['entry_path']
        assert entry['platforms'][selection['platform']] == identity['effective_path'] == selection['config_path']
        assert req[key][identity['effective_path']]['schema_version'] == 3
    for path, value in req['original'].items():
        assert json.loads(bytes(req['original_bytes'][path])) == value
    a = req['original'][bindings['base']['effective_path']]
    b = req['candidate'][bindings['candidate']['effective_path']]
    assert a == b
    old = req['original'][a['registries']['workflow']]
    new = req['candidate'][b['registries']['workflow']]
    expected = upgrade(old)
    for key in ['historical_profiles','migrations','integration']:
        expected[key] = new[key]
    assert expected == new
    return dict(values=req['original'],historical={},mappings=req['profile']['mappings'])
if __name__ == '__main__':
    req = json.load(sys.stdin)
    with open('.chrono-harness/state/decoder-input.json','w') as f:
        json.dump(req,f)
    print(json.dumps(convert(req)))
"#;
    let selected_tests = r#"import copy,json,runpy
module = runpy.run_path('.chrono-harness/decoder.py')
upgrade,convert = module['upgrade'],module['convert']
source = dict(schema_version=1,migrations=[])
assert upgrade(source) == dict(schema_version=3,migrations=[],historical_profiles=[])
assert source == dict(schema_version=1,migrations=[])
try:
    upgrade(dict(schema_version=3))
except ValueError:
    pass
else:
    raise AssertionError('accepted wrong historical schema')
# Exercise compatibility on the actual retained decoder input as well.
req = json.load(open('.chrono-harness/state/decoder-input.json'))
assert convert(req)['values'] == req['original']
for endpoint in ['base','candidate']:
    bad = copy.deepcopy(req)
    del bad['config_bindings'][endpoint]
    try:
        convert(bad)
    except KeyError:
        pass
    else:
        raise AssertionError('missing endpoint binding accepted')
print('selected workflow migration compatibility checked')
"#;
    let (decoder, tests) = if selected {
        native_v3(&mut h);
        (selected_decoder, selected_tests)
    } else {
        (decoder, tests)
    };
    for (id, path, code, pair) in [
        (
            "decoder",
            ".chrono-harness/decoder.py",
            decoder,
            json!({"test_script":"decoder-tests"}),
        ),
        (
            "decoder-tests",
            ".chrono-harness/decoder-tests.py",
            tests,
            json!({"tests_for":"decoder"}),
        ),
    ] {
        fs::write(h.root().join(path), code).unwrap();
        let mut f = file(
            path,
            json!([{"kind":"runtime-input","to":format!("script:{id}")}]),
        );
        f["owner"] = json!(id);
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(f);
        let p = h.values.get_mut(PROJECTS).unwrap();
        p["owners"].as_array_mut().unwrap().push(json!(id));
        let mut script = json!({"id":id,"path":path,"actions":{"execute":{"operation":id,"tool":"python","argv":["-B",path]}}});
        script
            .as_object_mut()
            .unwrap()
            .extend(pair.as_object().unwrap().clone());
        p["scripts"].as_array_mut().unwrap().push(script);
    }
    let f = h.values.get_mut(FM).unwrap();
    f["project_edges"].as_array_mut().unwrap().push(edge(
        "script:decoder",
        "test-execution",
        "test:decoder-tests",
    ));
    f["execution_plans"]["test:decoder-tests"] =
        json!({"operations":["decoder-tests"],"timeout_seconds":15,"output_limit_bytes":4096});
    f["test_costs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"test":"decoder-tests","cost":"unknown"}));
    fs::write(
        h.root().join(".chrono-harness/decoder.py"),
        "raise RuntimeError('historical decoder must never execute')\n",
    )
    .unwrap();
    if selected {
        select_config(&mut h, OLD_TARGET);
        // Keep an actual downstream request for view-integrity rejection tests.
        // The wrapper forwards original stdin/stdout to the real FILEMAP judge.
        let wrapper = ".chrono-harness/bin/record-filemap";
        let code = format!(
            "#!{}\nimport os,pathlib,subprocess,sys\nb=sys.stdin.buffer.read()\npathlib.Path('.chrono-harness/state/filemap-request.json').write_bytes(b)\nfds=tuple(int(fd) for fd in os.environ.get('CHRONO_PROCESS_FDS','').split(',') if fd)\nr=subprocess.run(['.chrono-harness/bin/chrono-judge-filemap']+sys.argv[1:],input=b,pass_fds=fds,stdout=subprocess.PIPE,stderr=subprocess.PIPE)\nsys.stdout.buffer.write(r.stdout)\nsys.stderr.buffer.write(r.stderr)\nsys.exit(r.returncode)\n",
            h.tool.display()
        );
        fs::write(h.root().join(wrapper), &code).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(h.root().join(wrapper), fs::Permissions::from_mode(0o755)).unwrap();
        }
        let judge = h.values.get_mut(JUDGES).unwrap()["judges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|j| j["id"] == "filemap")
            .unwrap();
        judge["executable"] = json!(wrapper);
        judge["sha256"] = json!(sha256(code.as_bytes()));
    }
    h.save();
    h.base = h.candidate.clone();
    let before = capture(&h, "before");
    fs::write(h.root().join(".chrono-harness/decoder.py"), decoder).unwrap();
    if selected {
        select_config(&mut h, NEW_TARGET);
    } else {
        v2(&mut h);
    }
    let w = h.values.get_mut(WORKFLOW).unwrap();
    w["schema_version"] = json!(3);
    w["historical_profiles"] = json!([{
        "id":"config-presence/v1-to-v2",
        "from_versions":{"config":1,"filemap":2,"projects":1,"judges":1,"workflow":1},
        "to_versions":{"config":2,"filemap":2,"projects":1,"judges":1,"workflow":3},
        "script":"decoder","test":"decoder-tests","mappings":[],"legacy_records":[],"ambiguities":[]
    }]);
    w["integration"]["tests"] = json!(["t", "decoder-tests"]);
    w["migrations"] = json!([
        {"from_version":1,"to_version":2,"script":"decoder","test":"decoder-tests","mappings":[],"reason":"explicit input presence"},
        {"from_version":1,"to_version":3,"script":"decoder","test":"decoder-tests","mappings":[],"reason":"explicit version selector"}
    ]);
    if selected {
        let w = h.values.get_mut(WORKFLOW).unwrap();
        w["historical_profiles"][0]["id"] = json!("selected-workflow/v1-to-v3");
        w["historical_profiles"][0]["from_versions"]["config"] = json!(3);
        w["historical_profiles"][0]["to_versions"]["config"] = json!(3);
        w["migrations"] = json!([{ "from_version":1,"to_version":3,"script":"decoder","test":"decoder-tests","mappings":[],"reason":"explicit workflow version selector" }]);
    }
    h.save();
    (h, before)
}
fn migration_check(h: &Host, before: &Value) -> (i32, Value) {
    check(
        h,
        &json!({"base":before,"candidate":capture(h, &format!("after-{}", h.candidate))}),
    )
}
fn conversion(report: &Value) -> &Value {
    &report["judges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"] == "registration")
        .unwrap()["response"]["outputs"]["registration_view"]["conversion"]
}

#[test]
fn explicit_version_migration_executes_candidate_decoder_and_preserves_v1_snapshot() {
    let (h, before) = migration_fixture();
    let (exit, r) = migration_check(&h, &before);
    assert_eq!(exit, 0, "{}", r["findings"]);
    assert_eq!(r["tests"]["tests"]["test:decoder-tests"], "passed");
    let c = conversion(&r);
    assert_eq!(c["input"]["schema"], "chrono-historical-decode/v2");
    assert_eq!(c["output"]["values"][CONFIG]["schema_version"], 1);
    assert_eq!(c["input"]["candidate"][CONFIG]["schema_version"], 2);
    assert_eq!(c["process"]["exit_code"], 0);
    assert_eq!(c["input_digest"], c["process"]["stdin_sha256"]);
    assert_eq!(c["output_digest"], c["process"]["stdout_sha256"]);
    assert_eq!(before["schema"], "chrono-input-snapshot/v1");
    assert!(
        r["effective_inputs"]["endpoints"]["base"]["files"]["data"]
            .get("presence")
            .is_none()
    );
    assert_eq!(
        r["effective_inputs"]["endpoints"]["candidate"]["files"]["data"]["presence"],
        "present"
    );
    // An unchanged v3 host must not run an old migration on a documentation DELTA.
    let mut h = h;
    h.base = h.candidate.clone();
    let before = capture(&h, "steady-before");
    fs::write(h.root().join("doc.txt"), "documentation only\n").unwrap();
    h.save();
    let (exit, r) = migration_check(&h, &before);
    assert_eq!(exit, 0, "{}", r["findings"]);
    assert!(conversion(&r).is_null());
    assert!(r["tests"]["executed"].as_array().unwrap().is_empty());
}

#[test]
fn version_migration_rejects_ambiguous_incomplete_and_unmatched_selectors() {
    let (mut h, before) = migration_fixture();
    let profile = h.values[WORKFLOW]["historical_profiles"][0].clone();
    let mut duplicate = profile.clone();
    duplicate["id"] = json!("ambiguous");
    h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"] = json!([profile, duplicate]);
    h.save();
    let (exit, r) = migration_check(&h, &before);
    assert_ne!(exit, 0);
    assert!(
        r["findings"].to_string().contains("E_MIGRATION_PROFILE"),
        "{r}"
    );
    assert!(r["tests"].is_null());
    h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"] = json!([profile]);
    h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]["from_versions"]
        .as_object_mut()
        .unwrap()
        .remove("config");
    let error = chrono_judge_registration::Registrations::load(&h.values, CONFIG)
        .err()
        .unwrap();
    assert!(error.contains("missing field config"), "{error}");
    for (key, value, expected) in [
        (
            "to_versions",
            profile["from_versions"].clone(),
            "must describe a transition",
        ),
        (
            "from_versions",
            json!({"config":0,"filemap":2,"projects":1,"judges":1,"workflow":1}),
            "positive integers",
        ),
        (
            "from_versions",
            json!({"config":1,"filemap":2,"projects":1,"judges":1,"workflow":1,"guessed":1}),
            "unknown field guessed",
        ),
    ] {
        h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"] = json!([profile]);
        h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"][0][key] = value;
        let error = chrono_judge_registration::Registrations::load(&h.values, CONFIG)
            .err()
            .unwrap();
        assert!(error.contains(expected), "{error}");
    }
    h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"] = json!([profile]);
    h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"][0]["to_versions"]["config"] =
        json!(1);
    h.save();
    let (exit, r) = migration_check(&h, &before);
    assert_ne!(exit, 0);
    assert!(
        r["findings"].to_string().contains("E_MIGRATION_EVIDENCE"),
        "{r}"
    );
    assert!(conversion(&r).is_null());
}

#[test]
fn version_migration_cannot_rewrite_historical_input_semantics() {
    let (mut h, before) = migration_fixture();
    let path = h.root().join(".chrono-harness/decoder.py");
    let source = fs::read_to_string(&path).unwrap();
    fs::write(path, source.replace(
        "return dict(values=req['original'],historical={},mappings=req['profile']['mappings'])",
        "req['original'][req['config_path']] = req['candidate'][req['config_path']]\n    return dict(values=req['original'],historical={},mappings=req['profile']['mappings'])",
    )).unwrap();
    h.save();
    let (exit, r) = migration_check(&h, &before);
    assert_ne!(exit, 0);
    assert!(
        r["findings"]
            .to_string()
            .contains("E_MIGRATION_INPUT_SEMANTICS"),
        "{r}"
    );
    assert!(r["tests"].is_null());
}

#[test]
fn version_migration_requires_its_actual_compatibility_test_and_each_changed_version() {
    let (mut h, before) = migration_fixture();
    let migrations = h.values[WORKFLOW]["migrations"].clone();
    h.values.get_mut(WORKFLOW).unwrap()["migrations"][0]["script"] = json!("p");
    h.values.get_mut(WORKFLOW).unwrap()["migrations"][0]["test"] = json!("t");
    h.save();
    let (exit, r) = migration_check(&h, &before);
    assert_ne!(exit, 0);
    assert_eq!(r["tests"]["tests"]["test:t"], "passed");
    assert!(
        r["findings"]
            .to_string()
            .contains("declared script/test differs"),
        "{r}"
    );
    h.values.get_mut(WORKFLOW).unwrap()["migrations"] = migrations.clone();
    h.values.get_mut(WORKFLOW).unwrap()["migrations"]
        .as_array_mut()
        .unwrap()
        .pop();
    h.save();
    let (exit, r) = migration_check(&h, &before);
    assert_ne!(exit, 0);
    assert!(
        r["findings"]
            .to_string()
            .contains("missing/ambiguous schema migration"),
        "{r}"
    );
    h.values.get_mut(WORKFLOW).unwrap()["migrations"] = migrations;
    fs::write(
        h.root().join(".chrono-harness/decoder-tests.py"),
        "raise AssertionError('incompatible migration')\n",
    )
    .unwrap();
    h.save();
    let (exit, r) = migration_check(&h, &before);
    assert_ne!(exit, 0);
    assert_eq!(r["tests"]["tests"]["test:decoder-tests"], "failed");
    assert!(
        !h.root()
            .join(".chrono-harness/state/integration.json")
            .exists()
    );
}

#[test]
fn selected_produced_pairs_preserve_endpoint_targets_old_edges_and_docs_locality() {
    for (base_target, candidate_target) in [
        (CONFIG, NEW_TARGET),
        (OLD_TARGET, CONFIG),
        (OLD_TARGET, NEW_TARGET),
    ] {
        let mut h = fixture();
        native_v3(&mut h);
        if base_target != CONFIG {
            select_config(&mut h, base_target);
        }
        let linked_entry = ".chrono-harness/linked entry.json";
        std::os::unix::fs::symlink("config.json", h.root().join(linked_entry)).unwrap();
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .push(file(linked_entry, json!([])));
        h.values.get_mut(FM).unwrap()["files"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["path"] == base_target)
            .unwrap()["edges"] = json!([{"kind":"test-execution","to":"test:t"}]);
        h.save();
        h.base = h.candidate.clone();
        capture(&h, "transition-before");
        select_config(&mut h, candidate_target);
        h.values.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .retain(|e| !(e["from"] == "input:data" && e["to"] == "script:p"));
        if base_target == CONFIG {
            h.values.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|f| f["path"] == CONFIG)
                .unwrap()["edges"] = json!([]);
        }
        let cfg = h.values.get_mut(candidate_target).unwrap();
        let data = cfg["environment"]["inputs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|i| i["id"] == "data")
            .unwrap();
        data["presence"] = json!("absent");
        data.as_object_mut().unwrap().remove("sha256");
        fs::remove_file(h.external.path()).unwrap();
        h.save();
        capture(&h, "transition-after");
        if candidate_target != CONFIG {
            let output = ".chrono-harness/state/linked-capture.json";
            let out = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
                .current_dir("/")
                .args([
                    "capture",
                    "--host-root",
                    h.root().to_str().unwrap(),
                    "--config",
                    linked_entry,
                    "--commit",
                    &h.candidate,
                    "--output",
                    output,
                ])
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(2));
            assert!(String::from_utf8_lossy(&out.stderr).contains("symlink path is not allowed:"));
            assert!(out.stdout.is_empty());
            assert!(!h.root().join(output).exists());
            assert!(!h.root().join(".chrono-harness/state/order").exists());
        }
        let pair = pair_captures(&h, "transition-before", "transition-after");
        assert!(pair["base"]["files"]["data"].get("blob").is_some());
        assert_eq!(pair["candidate"]["files"]["data"], json!({"absent":true}));
        if base_target != CONFIG {
            assert!(!h.root().join(base_target).exists());
        }
        let (exit, report) = check(&h, &pair);
        assert_eq!(
            exit, 0,
            "{base_target} -> {candidate_target}: {}",
            report["findings"]
        );
        assert_eq!(report["tests"]["tests"]["test:t"], "passed");
        assert!(
            conversion(&report).is_null(),
            "same-v3 wrapping is not a schema migration"
        );
        let view = registration_view(&report);
        assert_eq!(
            view["config_bindings"]["base"]["effective_path"],
            base_target
        );
        assert_eq!(
            view["config_bindings"]["candidate"]["effective_path"],
            candidate_target
        );
        let union = report["impact"]["edges"].as_array().unwrap();
        for (from, to) in [
            ("input:data".to_owned(), "script:p"),
            (format!("file:{base_target}"), "test:t"),
        ] {
            assert!(
                union
                    .iter()
                    .any(|e| e["from"] == from && e["to"] == to && e["origin"] == "base"),
                "missing old-only edge {from} -> {to}"
            );
            assert!(
                report["impact"]["closure"]["traversed"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["edge"]["from"] == from && e["edge"]["to"] == to),
                "old edge not traversed {from} -> {to}"
            );
        }
        fs::remove_file(h.root().join(".chrono-harness/state/order")).unwrap();
        h.base = h.candidate.clone();
        capture(&h, "docs-before");
        fs::write(
            h.root().join("doc.txt"),
            "unrelated selected documentation\n",
        )
        .unwrap();
        h.save();
        capture(&h, "docs-after");
        let (exit, report) = check(&h, &pair_captures(&h, "docs-before", "docs-after"));
        assert_eq!(exit, 0, "{}", report["findings"]);
        assert_eq!(report["tests"]["executed"], json!([]));
        assert!(!h.root().join(".chrono-harness/state/order").exists());
    }
}

#[test]
fn selected_produced_pair_rejects_missing_and_substituted_endpoint_binding() {
    let mut h = fixture();
    native_v3(&mut h);
    select_config(&mut h, OLD_TARGET);
    h.save();
    h.base = h.candidate.clone();
    capture(&h, "binding-before");
    select_config(&mut h, NEW_TARGET);
    h.save();
    capture(&h, "binding-after");
    let pair = pair_captures(&h, "binding-before", "binding-after");
    for endpoint in ["base", "candidate"] {
        for field in [
            "selection",
            "effective_config_path",
            "platform",
            "target",
            "entry",
        ] {
            let mut bad = pair.clone();
            match field {
                "selection" | "effective_config_path" => {
                    bad[endpoint].as_object_mut().unwrap().remove(field);
                }
                "platform" => bad[endpoint]["selection"]["platform"] = json!("wrong-platform"),
                "target" => bad[endpoint]["selection"]["config_path"] = json!(CONFIG),
                _ => bad[endpoint]["config_path"] = json!(NEW_TARGET),
            }
            let (exit, report) = check(&h, &bad);
            assert_ne!(exit, 0, "{endpoint}/{field}");
            assert!(
                report["findings"]
                    .to_string()
                    .contains("E_EVIDENCE_UNRESOLVED"),
                "{}",
                report["findings"]
            );
            assert!(report["tests"].is_null());
            assert!(!h.root().join(".chrono-harness/state/order").exists());
        }
    }
    let (exit, report) = check(&h, &pair);
    assert_eq!(exit, 0, "{}", report["findings"]);
}

fn registration_view(report: &Value) -> &Value {
    &report["judges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"] == "registration")
        .unwrap()["response"]["outputs"]["registration_view"]
}

#[test]
fn selected_migration_executes_v3_decoder_compatibility_and_downstream_view() {
    let (h, before) = migration_fixture_mode(true);
    assert_eq!(before["schema"], "chrono-input-snapshot/v3");
    capture(&h, "after");
    let pair = pair_captures(&h, "before", "after");
    let (exit, report) = check(&h, &pair);
    assert_eq!(exit, 0, "{}", report["findings"]);
    assert_eq!(report["tests"]["tests"]["test:decoder-tests"], "passed");
    let compatibility = report["tests"]["executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|op| op["operation"] == "decoder-tests")
        .unwrap();
    assert!(
        compatibility["receipt"]["process"]["stdout"]
            .as_str()
            .unwrap()
            .contains("selected workflow migration compatibility checked")
    );
    let view = registration_view(&report);
    assert_eq!(view["schema"], "chrono-registration-view/v2");
    let c = conversion(&report);
    assert_eq!(c["input"]["schema"], "chrono-historical-decode/v3");
    assert_eq!(c["input"]["config_bindings"], view["config_bindings"]);
    assert_eq!(
        c["input"]["config_bindings"]["base"]["effective_path"],
        OLD_TARGET
    );
    assert_eq!(
        c["input"]["config_bindings"]["candidate"]["effective_path"],
        NEW_TARGET
    );
    assert_eq!(c["input"]["original"][WORKFLOW]["schema_version"], 1);
    assert_eq!(c["input"]["candidate"][WORKFLOW]["schema_version"], 3);
    assert_eq!(c["process"]["exit_code"], 0);
    assert_eq!(c["input_digest"], c["process"]["stdin_sha256"]);
    assert_eq!(c["output_digest"], c["process"]["stdout_sha256"]);
    assert!(!h.root().join(OLD_TARGET).exists());
    for path in [CONFIG, OLD_TARGET] {
        let bytes = chrono_harness::facts::blob(&h.root(), &h.base, path).unwrap();
        assert_eq!(c["input"]["original_bytes"][path], json!(bytes));
        assert_eq!(
            view["base"][path],
            serde_json::from_slice::<Value>(&bytes).unwrap()
        );
    }
    // This is the real FILEMAP request after full registration, then resealed
    // counterexamples for the same downstream view consumer.
    let req: chrono_harness::wire::Request = serde_json::from_slice(
        &fs::read(h.root().join(".chrono-harness/state/filemap-request.json")).unwrap(),
    )
    .unwrap();
    let (_, _, accepted) = chrono_judge_registration::views(&req).unwrap();
    assert_eq!(accepted, *view);
    for case in [
        "missing-bindings",
        "substituted-binding",
        "missing-entry-binding",
        "missing-decoder-binding",
        "decoder-entry",
        "decoder-binding",
        "selector-bytes",
        "target-bytes",
        "original-selector",
        "original-target",
        "interpreted-target",
    ] {
        let mut bad = req.clone();
        let prior = bad
            .prior_results
            .iter_mut()
            .find(|r| r.outputs.contains_key("registration_view"))
            .unwrap();
        let v = prior.outputs.get_mut("registration_view").unwrap();
        match case {
            "missing-bindings" => {
                v.as_object_mut().unwrap().remove("config_bindings");
            }
            "substituted-binding" => {
                v["config_bindings"]["base"]["effective_path"] = json!(NEW_TARGET)
            }
            "missing-entry-binding" => {
                v["binding"].as_object_mut().unwrap().remove("entry_path");
            }
            "missing-decoder-binding" => {
                v["conversion"]["input"]
                    .as_object_mut()
                    .unwrap()
                    .remove("config_bindings");
            }
            "decoder-entry" => v["conversion"]["input"]["config_path"] = json!(NEW_TARGET),
            "decoder-binding" => {
                v["conversion"]["input"]["config_bindings"]["base"]["effective_path"] =
                    json!(NEW_TARGET)
            }
            "selector-bytes" => {
                v["conversion"]["input"]["original_bytes"][CONFIG] = json!(b"{}".to_vec())
            }
            "target-bytes" => {
                v["conversion"]["input"]["original_bytes"][OLD_TARGET] = json!(b"{}".to_vec())
            }
            "original-selector" => {
                v["conversion"]["input"]["original"][CONFIG] = v["candidate"][CONFIG].clone()
            }
            "original-target" => {
                v["conversion"]["input"]["original"][OLD_TARGET]["environment"]["values"]["EMPTY"] =
                    json!("rewritten")
            }
            _ => v["base"][OLD_TARGET]["environment"]["values"]["EMPTY"] = json!("rewritten"),
        }
        bad.seal().unwrap();
        let error = chrono_judge_registration::views(&bad)
            .err()
            .unwrap_or_else(|| panic!("accepted {case}"));
        assert!(
            error.contains("binding")
                || error.contains("original endpoint")
                || error.contains("E_MIGRATION_INPUT_SEMANTICS"),
            "{case}: {error}"
        );
    }
}

#[test]
fn selected_migration_requires_actual_conversion_and_passing_compatibility() {
    let (mut h, before) = migration_fixture_mode(true);
    let workflow = h.values[WORKFLOW].clone();
    h.values.get_mut(WORKFLOW).unwrap()["historical_profiles"] = json!([]);
    // A passing unrelated compatibility command cannot replace conversion.
    fs::write(
        h.root().join(".chrono-harness/decoder-tests.py"),
        "print('compatibility command passed without conversion')\n",
    )
    .unwrap();
    h.save();
    let (exit, report) = migration_check(&h, &before);
    assert_ne!(exit, 0);
    assert!(
        report["findings"]
            .to_string()
            .contains("E_MIGRATION_EVIDENCE"),
        "{}",
        report["findings"]
    );
    assert!(conversion(&report).is_null());
    h.values.insert(WORKFLOW.into(), workflow);
    fs::write(
        h.root().join(".chrono-harness/decoder-tests.py"),
        "raise AssertionError('selected compatibility failed')\n",
    )
    .unwrap();
    h.save();
    let (exit, report) = migration_check(&h, &before);
    assert_ne!(exit, 0);
    assert_eq!(report["tests"]["tests"]["test:decoder-tests"], "failed");
    assert!(
        !h.root()
            .join(".chrono-harness/state/integration.json")
            .exists()
    );
}

#[test]
fn selected_decoder_cannot_rewrite_original_effective_config() {
    let (mut h, before) = migration_fixture_mode(true);
    let path = h.root().join(".chrono-harness/decoder.py");
    let source = fs::read_to_string(&path).unwrap();
    fs::write(path, source.replace(
        "return dict(values=req['original'],historical={},mappings=req['profile']['mappings'])",
        "req['original'][bindings['base']['effective_path']]['environment']['values']['EMPTY'] = 'rewritten'\n    return dict(values=req['original'],historical={},mappings=req['profile']['mappings'])",
    )).unwrap();
    h.save();
    let (exit, report) = migration_check(&h, &before);
    assert_ne!(exit, 0);
    assert!(
        report["findings"]
            .to_string()
            .contains("E_MIGRATION_INPUT_SEMANTICS"),
        "{}",
        report["findings"]
    );
    assert!(report["tests"].is_null());
}
