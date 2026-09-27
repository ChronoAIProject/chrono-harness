use chrono_harness::{json, sha256, wire};
use serde_json::json as value;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn published_rfc8785_vectors() {
    // RFC 8785 §3.2.2 and §3.2.3, not produced by this implementation.
    let v =
        json(br#"{"numbers":[333333333.33333329,1E30,4.50,2e-3,0.000000000000000000000000001]}"#)
            .unwrap();
    assert_eq!(
        String::from_utf8(wire::canonical(&v).unwrap()).unwrap(),
        r#"{"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27]}"#
    );
    let v = value!({"\u{20ac}":1,"\r":2,"\u{fb33}":3,"1":4,"\u{1f600}":5,"\u{80}":6,"ö":7});
    assert_eq!(
        String::from_utf8(wire::canonical(&v).unwrap()).unwrap(),
        "{\"\\r\":2,\"1\":4,\"\u{80}\":6,\"ö\":7,\"€\":1,\"😀\":5,\"דּ\":3}"
    );
    assert_eq!(
        wire::canonical(&value!([-0.0, 1e-6, 1e21])).unwrap(),
        b"[0,0.000001,1e+21]"
    );
}
fn fixture(script: &str) -> (tempfile::TempDir, wire::Request, wire::Binding) {
    let dir = tempfile::Builder::new()
        .prefix("v1 host spaces ")
        .tempdir()
        .unwrap();
    let path = dir.path().join("judge");
    fs::write(&path, format!("#!/usr/bin/python3\n{script}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let mut r: wire::Request = serde_json::from_value(value!({
        "protocol":"chrono-judge/v1", "request_id":"", "judge_id":"fixture", "mode":"evaluate",
        "base":{"commit":"a".repeat(40),"tree":"c".repeat(40),"root":dir.path()},
        "candidate":{"commit":"b".repeat(40),"tree":"d".repeat(40),"root":dir.path()},
        "delta":[],"registries":{"base":dir.path(),"candidate":dir.path(),"digest":"e".repeat(64)},
        "context":{"path":dir.path().join("context.json"),"sha256":"f".repeat(64)},
        "impact":{"seeds":[],"edges":[],"tests":[],"retired_tests":[]},"prior_results":[],
        "config_path":".chrono-harness/config.json",
        "checkout":{"head":"b".repeat(40),"tracked":[],"untracked":[],"index_flags":[]},
        "runner":{"path":"runner","sha256":"e".repeat(64),"version":"0.1.0"}
    }))
    .unwrap();
    r.seal().unwrap();
    let binding = wire::Binding {
        id: "fixture".into(),
        executable: "judge".into(),
        version: "fixture".into(),
        sha256: Some(sha256(&fs::read(path).unwrap())),
        argv: vec![],
        selector: "every-delta".into(),
        after: vec![],
        modes: vec!["evaluate".into()],
    };
    (dir, r, binding)
}
const RESPONSE: &str = "import json,sys\nr=json.load(sys.stdin)\ns={'protocol':r['protocol'],'request_id':r['request_id'],'judge_id':r['judge_id'],'status':'pass','findings':[],'evidence':[],'outputs':{}}\n";
fn invoke(script: &str) -> Result<wire::Response, String> {
    let (_dir, r, b) = fixture(script);
    wire::invoke(&r, &b, &Default::default(), 5, 8192).map(|v| v.0)
}
#[test]
fn four_statuses_exact_exits_and_identity() {
    for (status, exit) in [("pass", 0), ("warn", 0), ("fail", 1), ("error", 2)] {
        let findings = if status == "pass" {
            "[]"
        } else {
            "[{'code':'FIXTURE','level':'error' if s['status'] in ['fail','error'] else 'warning','message':'actual fixture result','delta_refs':['/fixture'],'causes':[]}]"
        };
        let script = format!(
            "{RESPONSE}s['status']='{status}'\ns['findings']={findings}\nprint(json.dumps(s))\nsys.exit({exit})"
        );
        assert_eq!(invoke(&script).unwrap().status.exit_code(), exit);
        assert!(invoke(&script.replace(&format!("sys.exit({exit})"), "sys.exit(9)")).is_err());
    }
    for change in [
        "s['request_id']='wrong'",
        "s['judge_id']='wrong'",
        "s['protocol']='wrong'",
        "s['status']='unknown'",
        "s['extra']=True",
    ] {
        assert!(invoke(&format!("{RESPONSE}{change}\nprint(json.dumps(s))")).is_err());
    }
}
#[test]
fn malformed_stdout_crash_and_bounds() {
    for tail in [
        "pass",
        "print('no json')",
        "print('{}')",
        "print(json.dumps(s));print('{}')",
        "print('{\"protocol\":1,\"protocol\":2}')",
        "sys.stdout.buffer.write(b'\\xff')",
        "sys.exit(9)",
        "print('x'*10000)",
    ] {
        assert!(invoke(&format!("{RESPONSE}{tail}")).is_err(), "{tail}");
    }
    let (_dir, r, b) = fixture("import time\ntime.sleep(60)");
    assert!(
        wire::invoke(&r, &b, &Default::default(), 1, 8192)
            .unwrap_err()
            .contains("timed out")
    );
}
#[test]
fn evidence_exists_and_is_bound_to_bytes() {
    let (dir, r, b) = fixture(&format!(
        "{RESPONSE}s['evidence']=[{{'path':'.chrono-harness/state/proof','sha256':'{}','kind':'fixture'}}]\nprint(json.dumps(s))",
        sha256(b"proof")
    ));
    let call = || wire::invoke(&r, &b, &Default::default(), 5, 8192);
    assert!(call().is_err());
    fs::create_dir_all(dir.path().join(".chrono-harness/state")).unwrap();
    fs::write(dir.path().join(".chrono-harness/state/proof"), "tampered").unwrap();
    assert!(call().is_err());
    fs::write(dir.path().join(".chrono-harness/state/proof"), "proof").unwrap();
    assert!(call().is_ok());
}
#[test]
fn digest_checked_before_launch_environment_cleared_and_literal_argv() {
    let literal = "literal $HOME `whoami` ' \"";
    let (dir, r, mut b) = fixture(&format!(
        "{RESPONSE}import os\nassert 'HOME' not in os.environ\nassert os.environ['EXPLICIT']=='yes'\nassert sys.argv[1]=={}\nassert os.getcwd()==os.path.realpath(r['candidate']['root'])\nprint(json.dumps(s))",
        serde_json::to_string(literal).unwrap()
    ));
    b.argv = vec![literal.into()];
    let env = std::collections::BTreeMap::from([("EXPLICIT".into(), "yes".into())]);
    let outcome = wire::invoke(&r, &b, &env, 5, 8192);
    assert!(outcome.is_ok(), "{outcome:?}");
    fs::write(dir.path().join("judge"), "#!/bin/sh\ntouch launched\n").unwrap();
    assert!(
        wire::invoke(&r, &b, &env, 5, 8192)
            .unwrap_err()
            .contains("digest")
    );
    assert!(!dir.path().join("launched").exists());
}
#[test]
fn endpoint_argv_placeholders_replace_only_complete_arguments() {
    let literals = [
        "",
        "prefix{base}",
        "{candidate}suffix",
        "{base}{candidate}",
        " {base}",
        "{candidate} ",
        "{{base}}",
        "{BASE}",
        "{tree}",
        "literal with spaces\nand a newline",
        "é 😀",
        "$HOME ${base} `whoami` $(touch expanded) ; & | > < * ? ' \" \\",
    ];
    let script = format!("{RESPONSE}s['outputs']['argv']=sys.argv[1:]\nprint(json.dumps(s))");
    let (dir, r, mut b) = fixture(&script);
    b.argv = vec!["{candidate}".into(), "{base}".into()];
    b.argv.extend(literals.iter().map(|s| s.to_string()));
    b.argv.push("{base}".into());
    let (response, process) = wire::invoke(&r, &b, &Default::default(), 5, 8192).unwrap();
    assert_eq!(process.exit_code, 0);
    let mut expected = vec![r.candidate.commit.clone(), r.base.commit.clone()];
    expected.extend(literals.iter().map(|s| s.to_string()));
    expected.push(r.base.commit.clone());
    assert_eq!(response.outputs["argv"], value!(expected));
    assert!(!dir.path().join("expanded").exists());
}
#[test]
fn request_identity_is_deterministic_and_binds_facts() {
    let (_dir, mut r, _b) = fixture("");
    let old = r.request_id.clone();
    r.seal().unwrap();
    assert_eq!(r.request_id, old);
    r.config_path = ".chrono-harness/other.json".into();
    assert!(r.validate().is_err());
    r.seal().unwrap();
    assert_ne!(r.request_id, old);
}

#[test]
fn dag_forwards_only_direct_predecessors_and_named_outputs() {
    let script = format!(
        "{RESPONSE}expected={{'a':[], 'b':['a'], 'c':['b'], 'independent':[]}}\nassert [p['judge_id'] for p in r['prior_results']]==expected[r['judge_id']]\nif r['judge_id']=='a': s['outputs']={{'impact':{{'seeds':['file:example'],'edges':[],'tests':[],'retired_tests':[]}},'named':42}}\nif r['judge_id']=='b':\n assert r['impact']['seeds']==['file:example']\n assert r['prior_results'][0]['outputs']['named']==42\nif r['judge_id']=='c': assert r['impact'] is None\nprint(json.dumps(s))"
    );
    let (_dir, mut r, b) = fixture(&script);
    r.impact = serde_json::Value::Null;
    let binding = |id: &str, after: &[&str]| {
        let mut v = b.clone();
        v.id = id.into();
        v.after = after.iter().map(|s| s.to_string()).collect();
        v
    };
    let plan = vec![
        binding("c", &["b"]),
        binding("independent", &[]),
        binding("b", &["a"]),
        binding("a", &[]),
    ];
    let (status, records) =
        chrono_harness::full::execute(&r, &plan, &Default::default(), 5, 8192).unwrap();
    assert_eq!(status, wire::Status::Pass, "{records:#?}");
    assert_eq!(records.len(), 4);
    assert!(records.iter().all(|r| r["state"] == "executed"));
}
#[test]
fn dag_blocks_dependents_and_continues_independent_branch() {
    let script = format!(
        "{RESPONSE}assert r['judge_id']!='blocked'\nif r['judge_id']=='bad':\n s['status']='error'\n s['findings']=[{{'code':'E_FIXTURE','level':'error','message':'known error','delta_refs':['/fixture'],'causes':[]}}]\nprint(json.dumps(s))\nsys.exit(2 if r['judge_id']=='bad' else 0)"
    );
    let (_dir, r, b) = fixture(&script);
    let binding = |id: &str, after: &[&str]| {
        let mut v = b.clone();
        v.id = id.into();
        v.after = after.iter().map(|s| s.to_string()).collect();
        v
    };
    let plan = vec![
        binding("blocked", &["bad"]),
        binding("independent", &[]),
        binding("bad", &[]),
    ];
    let (status, records) =
        chrono_harness::full::execute(&r, &plan, &Default::default(), 5, 8192).unwrap();
    assert_eq!(status, wire::Status::Error);
    assert_eq!(records[1]["state"], "blocked");
    assert_eq!(records[2]["id"], "independent");
    assert_eq!(records[2]["response"]["status"], "pass");
    let mut broken = plan.clone();
    broken[0].after = vec!["missing".into()];
    assert!(chrono_harness::full::schedule(&broken).is_err());
    broken[0].after = vec!["blocked".into()];
    assert!(chrono_harness::full::schedule(&broken).is_err());
    assert!(chrono_harness::full::schedule(&[]).is_err());
}

#[test]
fn invalid_response_retains_actual_process_exit_in_report() {
    let (_dir, r, b) = fixture("import sys\nprint('invalid JSON')\nsys.exit(9)");
    let (status, records) =
        chrono_harness::full::execute(&r, &[b], &Default::default(), 5, 8192).unwrap();
    assert_eq!(status, wire::Status::Error);
    assert_eq!(records[0]["state"], "executed");
    assert_eq!(records[0]["exit_code"], 9);
    assert_eq!(records[0]["process"]["stdout"], "invalid JSON\n");
}

#[test]
fn embedded_invalid_utf8_is_protocol_error_and_valid_replacement_passes() {
    for (bytes, accepted) in [("bytes([239,191,189])", true), ("bytes([255])", false)] {
        let script = format!(
            "{RESPONSE}s['outputs']['note']='MARKER'\nsys.stdout.buffer.write(json.dumps(s).encode().replace(b'MARKER',{bytes}))"
        );
        let (_dir, r, b) = fixture(&script);
        let (status, records) =
            chrono_harness::full::execute(&r, &[b], &Default::default(), 5, 8192).unwrap();
        assert_eq!(
            status,
            if accepted {
                wire::Status::Pass
            } else {
                wire::Status::Error
            },
            "original protocol bytes must determine UTF-8 validity"
        );
        let raw: Vec<u8> =
            serde_json::from_value(records[0]["process"]["stdout_bytes"].clone()).unwrap();
        assert_eq!(std::str::from_utf8(&raw).is_ok(), accepted);
        assert_eq!(records[0]["process"]["exit_code"], 0);
        if accepted {
            assert_eq!(records[0]["response"]["outputs"]["note"], "�");
        }
    }
}

#[test]
fn later_judge_receives_actual_prior_process_identity_not_configured_metadata() {
    let script = format!(
        "{RESPONSE}import hashlib\nif r['judge_id']=='consumer':\n o=r['observations']['judges']\n assert len(o)==1 and o[0]['id']=='producer'\n p=o[0]['process']\n assert o[0]['request_digest']==p['stdin_sha256']\n assert hashlib.sha256(bytes(p['stdout_bytes'])).hexdigest()==p['stdout_sha256']\n assert json.loads(bytes(p['stdout_bytes']))==r['prior_results'][0]\n assert o[0]['request_id']==r['prior_results'][0]['request_id']\n assert p['exit_code']==0 and p['failure'] is None\nprint(json.dumps(s))"
    );
    let (_dir, req, mut producer) = fixture(&script);
    producer.id = "producer".into();
    let mut consumer = producer.clone();
    consumer.id = "consumer".into();
    consumer.after = vec!["producer".into()];
    let (status, rows) =
        chrono_harness::full::execute(&req, &[consumer, producer], &Default::default(), 5, 16384)
            .unwrap();
    assert_eq!(status, wire::Status::Pass, "{rows:#?}");
    assert_eq!(rows.len(), 2);
}
