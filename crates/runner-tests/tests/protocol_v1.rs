#[path = "support/native_executable.rs"]
mod native_executable;

use chrono_harness::{json, sha256, wire};
use serde_json::json as value;
use std::fs;

// Isolated launch helpers reject allocator use between fork and exec, including
// error paths. This observes real child execution, independent of its diagnostics.
struct ForkCheckedAllocator;
static LAUNCH_PARENT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
unsafe extern "C" {
    fn getpid() -> i32;
    fn _exit(status: i32) -> !;
}
fn check_fork_allocator() {
    let parent = LAUNCH_PARENT.load(std::sync::atomic::Ordering::Relaxed);
    if parent != 0 && unsafe { getpid() } != parent {
        unsafe { _exit(97) };
    }
}
fn check_launch_allocations() {
    LAUNCH_PARENT.store(unsafe { getpid() }, std::sync::atomic::Ordering::Relaxed);
}
unsafe impl std::alloc::GlobalAlloc for ForkCheckedAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        check_fork_allocator();
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        check_fork_allocator();
        unsafe { std::alloc::System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: std::alloc::Layout) {
        check_fork_allocator();
        unsafe { std::alloc::System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        check_fork_allocator();
        unsafe { std::alloc::System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: ForkCheckedAllocator = ForkCheckedAllocator;

// Ordinary protocol cases test responses, not startup latency. Real child
// startup has exceeded five seconds on the shared host. Deliberate timeout
// cases below keep their separate two-second bound and thirty-second stall.
const FIXTURE_TIMEOUT_SECONDS: u64 = 30;
const PROCESS_FIXTURE: &str = env!("CARGO_BIN_EXE_chrono-test-process-lifecycle");
const PROTOCOL_FIXTURE: &str = env!("CARGO_BIN_EXE_chrono-test-protocol-judge");

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
fn fixture(case: serde_json::Value) -> (tempfile::TempDir, wire::Request, wire::Binding) {
    let dir = tempfile::Builder::new()
        .prefix("v1 host spaces ")
        .tempdir_in(std::path::Path::new(PROTOCOL_FIXTURE).parent().unwrap())
        .unwrap();
    let path = dir.path().join("judge");
    // The protocol requires a regular file with the completed build bytes;
    // installation never opens an executable for writing.
    native_executable::install(PROTOCOL_FIXTURE, &path).unwrap();
    fs::write(
        dir.path().join("judge-case.json"),
        serde_json::to_vec(&case).unwrap(),
    )
    .unwrap();
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

fn replace_judge(directory: &std::path::Path, executable: &str) {
    let path = directory.join("judge");
    fs::remove_file(&path).unwrap();
    native_executable::install(executable, path).unwrap();
}
fn initial_fixture(
    case: serde_json::Value,
) -> (
    tempfile::TempDir,
    chrono_harness::initial::Request,
    wire::Binding,
) {
    let (dir, old, mut binding) = fixture(case);
    binding.selector = "every-initial".into();
    binding.modes = vec!["inventory".into()];
    let mut request = chrono_harness::initial::Request {
        protocol: chrono_harness::initial::PROTOCOL.into(),
        request_id: String::new(),
        judge_id: old.judge_id,
        mode: "inventory".into(),
        candidate: old.candidate,
        profile_path: ".chrono-harness/initial.json".into(),
        profile_sha256: "a".repeat(64),
        config_path: old.config_path,
        registry_digest: old.registries.digest,
        checkout: old.checkout,
        runner: old.runner,
        observations: value!({}),
        prior_results: vec![],
    };
    request.seal().unwrap();
    (dir, request, binding)
}

#[test]
fn initial_transport_has_one_endpoint_and_preserves_real_response_failures() {
    for (name, mut case) in [
        ("pass", value!({})),
        (
            "invalid-json",
            value!({"kind":"raw", "consume_request":true, "stdout":b"invalid json\n"}),
        ),
        (
            "wrong-protocol",
            value!({"fields":{"protocol":"chrono-judge/v1"}}),
        ),
        ("nonzero", value!({"exit":3})),
    ] {
        case["initial_only"] = value!(true);
        let (dir, request, binding) = initial_fixture(case);
        let result = chrono_harness::initial::invoke(
            &request,
            &binding,
            &Default::default(),
            FIXTURE_TIMEOUT_SECONDS,
            8192,
        );
        if name == "pass" {
            let (response, process) = result.unwrap();
            assert_eq!(response.status, wire::Status::Pass);
            assert_eq!(process.exit_code, 0);
            assert_eq!(response.protocol, "chrono-initial-judge/v1");
        } else {
            let failure = result.unwrap_err();
            assert!(failure.process.is_some(), "{failure:?}");
        }
        let mut value = serde_json::to_value(&request).unwrap();
        value["base"] = value!({"commit":"f".repeat(40),"root":dir.path(),"tree":"e".repeat(40)});
        assert!(serde_json::from_value::<chrono_harness::initial::Request>(value).is_err());
    }
}

#[test]
fn initial_transport_rejects_wrong_mode_digest_and_base_placeholder_before_launch() {
    let (dir, request, original) = initial_fixture(value!({"marker":"ran"}));
    for case in ["mode", "digest", "base", "request"] {
        let mut binding = original.clone();
        let mut request = request.clone();
        match case {
            "mode" => binding.modes = vec!["evaluate".into()],
            "digest" => binding.sha256 = Some("0".repeat(64)),
            "base" => binding.argv.push("{base}".into()),
            "request" => request.candidate.commit = "f".repeat(40),
            _ => unreachable!(),
        }
        assert!(
            chrono_harness::initial::invoke(
                &request,
                &binding,
                &Default::default(),
                FIXTURE_TIMEOUT_SECONDS,
                8192
            )
            .is_err(),
            "{case}"
        );
        assert!(!dir.path().join("ran").exists(), "{case}");
    }
}
fn configure_bound_fixture(
    dir: &std::path::Path,
    binding: &mut wire::Binding,
    protocol: &str,
    request_id: &str,
    timeout: bool,
) {
    let response = value!({"protocol":protocol,"request_id":request_id,"judge_id":"fixture",
        "status":"pass","findings":[],"evidence":[],"outputs":{}});
    fs::write(
        dir.join("judge-case.json"),
        serde_json::to_vec(&value!({
            "kind":"bound", "response":response, "timeout":timeout
        }))
        .unwrap(),
    )
    .unwrap();
    binding.sha256 = Some(sha256(&fs::read(dir.join("judge")).unwrap()));
}
fn assert_bound_evidence(failure: wire::TransportFailure, timeout: bool) {
    let expected = if timeout {
        "process timed out"
    } else {
        "process output limit exceeded"
    };
    assert!(failure.message.contains(expected), "{failure:?}");
    let process = failure
        .process
        .expect("a launched bounded judge must retain its process evidence");
    assert_eq!(process.failure.as_deref(), Some(expected));
    assert_eq!(process.stdout_sha256, sha256(&process.stdout_bytes));
    assert_eq!(process.stderr_sha256, sha256(&process.stderr_bytes));
    assert!(process.stdout_bytes.len() <= 1024 && process.stderr_bytes.len() <= 1024);
    if timeout {
        // A timeout can precede child output. Preserve the actual record without
        // assuming wall-clock progress of the child.
        assert_eq!(process.exit_code, -1);
        if !process.stdout_bytes.is_empty() {
            assert_eq!(json(&process.stdout_bytes).unwrap()["status"], "pass");
        }
    } else {
        assert!(
            process
                .stderr_bytes
                .starts_with(b"diagnostic-before-bound\n")
        );
        assert_eq!(
            json(&process.stdout_bytes).unwrap()["status"],
            "pass",
            "a valid-looking response cannot hide a process bound"
        );
        assert_eq!(process.stderr_bytes.len(), 1024);
    }
}
#[test]
fn bounded_delta_transport_retains_process_evidence() {
    for timeout in [true, false] {
        let (dir, request, mut binding) = fixture(value!({}));
        configure_bound_fixture(
            dir.path(),
            &mut binding,
            &request.protocol,
            &request.request_id,
            timeout,
        );
        // Match ordinary protocol fixtures' startup guard when testing output;
        // the deliberately shorter timeout is only the timeout case's subject.
        let seconds = if timeout { 2 } else { FIXTURE_TIMEOUT_SECONDS };
        let failure = wire::invoke_detailed(&request, &binding, &Default::default(), seconds, 1024)
            .expect_err("a process bound is an error despite a valid response");
        assert_bound_evidence(failure, timeout);
    }
}
#[test]
fn bounded_initial_transport_retains_process_evidence() {
    for timeout in [true, false] {
        let (dir, request, mut binding) = initial_fixture(value!({}));
        configure_bound_fixture(
            dir.path(),
            &mut binding,
            &request.protocol,
            &request.request_id,
            timeout,
        );
        let seconds = if timeout { 2 } else { FIXTURE_TIMEOUT_SECONDS };
        let failure =
            chrono_harness::initial::invoke(&request, &binding, &Default::default(), seconds, 1024)
                .expect_err("an initial process bound cannot complete inventory");
        assert_bound_evidence(failure, timeout);
    }
}
fn invoke(case: serde_json::Value) -> Result<wire::Response, String> {
    let (_dir, r, b) = fixture(case);
    wire::invoke(&r, &b, &Default::default(), FIXTURE_TIMEOUT_SECONDS, 8192).map(|v| v.0)
}
#[test]
fn four_statuses_exact_exits_and_identity() {
    for (status, exit) in [("pass", 0), ("warn", 0), ("fail", 1), ("error", 2)] {
        let findings = if status == "pass" {
            value!([])
        } else {
            value!([{"code":"FIXTURE", "level":if ["fail","error"].contains(&status) {"error"} else {"warning"},
                "message":"actual fixture result", "delta_refs":["/fixture"], "causes":[]}])
        };
        let mut case = value!({"fields":{"status":status,"findings":findings},"exit":exit});
        assert_eq!(invoke(case.clone()).unwrap().status.exit_code(), exit);
        case["exit"] = value!(9);
        assert!(invoke(case).is_err());
    }
    for (field, value) in [
        ("request_id", value!("wrong")),
        ("judge_id", value!("wrong")),
        ("protocol", value!("wrong")),
        ("status", value!("unknown")),
        ("extra", value!(true)),
    ] {
        let mut case = value!({"fields":{}});
        case["fields"][field] = value;
        assert!(invoke(case).is_err());
    }
}

#[test]
fn malformed_stdout_crash_and_bounds() {
    for (name, case) in [
        (
            "empty",
            value!({"kind":"raw", "consume_request":true,"stdout":[]}),
        ),
        (
            "invalid-json",
            value!({"kind":"raw", "consume_request":true,"stdout":b"no json\n"}),
        ),
        (
            "missing-fields",
            value!({"kind":"raw", "consume_request":true,"stdout":b"{}\n"}),
        ),
        ("extra-json", value!({"suffix":b"{}\n"})),
        (
            "duplicate-keys",
            value!({"kind":"raw", "consume_request":true,"stdout":b"{\"protocol\":1,\"protocol\":2}\n"}),
        ),
        (
            "invalid-utf8",
            value!({"kind":"raw", "consume_request":true,"stdout":[255]}),
        ),
        (
            "crash",
            value!({"kind":"raw", "consume_request":true,"stdout":[],"exit":9}),
        ),
        (
            "output-limit",
            value!({"kind":"raw", "consume_request":true,"stdout":format!("{}\n", "x".repeat(10000)).as_bytes()}),
        ),
    ] {
        assert!(invoke(case).is_err(), "{name}");
    }
    let (_dir, r, b) = fixture(value!({"kind":"sleep","seconds":60}));
    assert!(
        wire::invoke(&r, &b, &Default::default(), 1, 8192)
            .unwrap_err()
            .contains("timed out")
    );
}
#[test]
fn evidence_exists_and_is_bound_to_bytes() {
    let (dir, r, b) = fixture(value!({"fields":{"evidence":[{
        "path":".chrono-harness/state/proof", "sha256":sha256(b"proof"), "kind":"fixture"
    }]}}));
    let call = || wire::invoke(&r, &b, &Default::default(), FIXTURE_TIMEOUT_SECONDS, 8192);
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
    let (dir, r, mut b) = fixture(value!({"kind":"environment", "argument":literal}));
    b.argv = vec![literal.into()];
    let env = std::collections::BTreeMap::from([("EXPLICIT".into(), "yes".into())]);
    let outcome = wire::invoke(&r, &b, &env, FIXTURE_TIMEOUT_SECONDS, 8192);
    assert!(outcome.is_ok(), "{outcome:?}");
    let replacement_digest = sha256(&fs::read(PROCESS_FIXTURE).unwrap());
    assert_ne!(b.sha256.as_deref(), Some(replacement_digest.as_str()));
    replace_judge(dir.path(), PROCESS_FIXTURE);
    b.argv = vec!["marker".into(), "launched".into()];
    assert!(
        wire::invoke(&r, &b, &env, FIXTURE_TIMEOUT_SECONDS, 8192)
            .unwrap_err()
            .contains("digest")
    );
    assert!(!dir.path().join("launched").exists());
    // The substituted native executable can actually produce the forbidden
    // effect; an invalid executable must not make the negative check vacuous.
    b.sha256 = Some(replacement_digest);
    assert!(wire::invoke(&r, &b, &env, FIXTURE_TIMEOUT_SECONDS, 8192).is_err());
    assert_eq!(fs::read(dir.path().join("launched")).unwrap(), b"yes");
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
    let (dir, r, mut b) = fixture(value!({"kind":"argv"}));
    b.argv = vec!["{candidate}".into(), "{base}".into()];
    b.argv.extend(literals.iter().map(|s| s.to_string()));
    b.argv.push("{base}".into());
    let (response, process) =
        wire::invoke(&r, &b, &Default::default(), FIXTURE_TIMEOUT_SECONDS, 8192).unwrap();
    assert_eq!(process.exit_code, 0);
    let mut expected = vec![r.candidate.commit.clone(), r.base.commit.clone()];
    expected.extend(literals.iter().map(|s| s.to_string()));
    expected.push(r.base.commit.clone());
    assert_eq!(response.outputs["argv"], value!(expected));
    assert!(!dir.path().join("expanded").exists());
}
#[test]
fn request_identity_is_deterministic_and_binds_facts() {
    let (_dir, mut r, _b) = fixture(value!({}));
    let old = r.request_id.clone();
    r.seal().unwrap();
    assert_eq!(r.request_id, old);
    r.config_path = ".chrono-harness/other.json".into();
    assert!(r.validate().is_err());
    r.seal().unwrap();
    assert_ne!(r.request_id, old);
}

#[test]
fn request_identity_matches_original_projection_for_scopes_and_opaque_evidence() {
    let (_dir, mut request, _binding) = fixture(value!({}));
    request.observations = value!({
        "opaque":{"😀":"escaped \\\"\n", "דּ":[-0.0, 1e-27, 1e30, null, true]},
        "bytes":(0..=255).collect::<Vec<u8>>()
    });
    for scope in [
        None,
        Some(chrono_harness::units::Scope::Unit { unit: "one".into() }),
        Some(chrono_harness::units::Scope::Collect {
            manifest: ".chrono-harness/state/m.json".into(),
        }),
    ] {
        request.scope = scope;
        let mut original = serde_json::to_value(&request).unwrap();
        original.as_object_mut().unwrap().remove("request_id");
        let expected = wire::digest(&original).unwrap();
        request.seal().unwrap();
        assert_eq!(request.request_id, expected);
        assert_eq!(
            request.canonical().unwrap(),
            wire::canonical(&request).unwrap()
        );
        request.validate().unwrap();
        let mut changed = request.clone();
        changed.observations["bytes"][255] = value!(254);
        assert!(changed.validate().is_err());
    }
}

#[test]
fn canonical_request_preserves_number_boundaries_unicode_and_original_streams() {
    let (_dir, mut request, _binding) = fixture(value!({}));
    // Independent RFC vectors and the fixed generic JCS implementation remain
    // the oracle, including values outside the exact-integer fast path.
    let mut numbers = vec![
        value!(i64::MIN),
        value!(i64::MAX),
        value!(u64::MAX),
        value!(-9007199254740993_i64),
        value!(-9007199254740992_i64),
        value!(9007199254740992_u64),
        value!(9007199254740993_u64),
        value!(-0.0),
        value!(1e-6),
        value!(1e21),
        value!(1e-27),
    ];
    let mut bits = 0x123456789abcdef0_u64;
    for _ in 0..1024 {
        bits = bits.wrapping_mul(6364136223846793005).wrapping_add(1);
        let number = f64::from_bits(bits);
        if number.is_finite() {
            numbers.push(value!(number));
        }
    }
    request.observations = value!({
        "numbers": numbers,
        "object": {"\u{ffff}": [], "😀": {}, "\r": null, "€": true},
        "strings": ["\u{0000}\u{0001}\u{0008}\u{000c}\n\r\t\\\"/", "中文 λ 😀"],
        "stdout_bytes": (0..=255).cycle().take(65536).collect::<Vec<u8>>()
    });
    let mut prior = request.response(wire::Status::Pass);
    prior
        .outputs
        .insert("original".into(), request.observations.clone());
    request.prior_results.push(prior);
    for scope in [
        None,
        Some(chrono_harness::units::Scope::Unit { unit: "one".into() }),
    ] {
        request.scope = scope;
        let mut identity = serde_json::to_value(&request).unwrap();
        identity.as_object_mut().unwrap().remove("request_id");
        request.seal().unwrap();
        assert_eq!(request.request_id, wire::digest(&identity).unwrap());
        assert_eq!(
            request.canonical().unwrap(),
            wire::canonical(&request).unwrap()
        );
        request.validate().unwrap();
        let mut changed = request.clone();
        changed.prior_results[0]
            .outputs
            .get_mut("original")
            .unwrap()["stdout_bytes"][65535] = value!(0);
        assert!(changed.validate().is_err());
    }
}

#[test]
fn prepared_live_dag_rejects_invalid_headers_before_process_effects() {
    let (dir, request, binding) = fixture(value!({"marker":"ran"}));
    for field in ["protocol", "mode"] {
        let mut bad = request.clone();
        match field {
            "protocol" => bad.protocol = "wrong".into(),
            "mode" => bad.mode = "inventory".into(),
            _ => unreachable!(),
        }
        let (status, rows) = chrono_harness::full::execute(
            &bad,
            &[binding.clone()],
            &Default::default(),
            FIXTURE_TIMEOUT_SECONDS,
            16384,
        )
        .unwrap();
        assert_eq!(status, wire::Status::Error);
        assert_eq!(rows[0]["state"], "error");
        assert!(
            rows[0]["transport_failure"]
                .as_str()
                .unwrap()
                .contains("E_PROTOCOL")
        );
        assert!(!dir.path().join("ran").exists());
    }
}

#[test]
fn dag_forwards_only_direct_predecessors_and_named_outputs() {
    let (_dir, mut r, b) = fixture(value!({"kind":"dag"}));
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
    let (status, records) = chrono_harness::full::execute(
        &r,
        &plan,
        &Default::default(),
        FIXTURE_TIMEOUT_SECONDS,
        8192,
    )
    .unwrap();
    assert_eq!(status, wire::Status::Pass, "{records:#?}");
    assert_eq!(records.len(), 4);
    assert!(records.iter().all(|r| r["state"] == "executed"));
}

#[test]
fn execution_self_diagnostic_rejects_mutated_records_and_status() {
    let (_dir, request, binding) = fixture(value!({"kind":"response"}));
    let (status, records) = chrono_harness::full::execute(
        &request,
        std::slice::from_ref(&binding),
        &Default::default(),
        FIXTURE_TIMEOUT_SECONDS,
        8192,
    )
    .unwrap();
    chrono_harness::full::validate_execution(&request, &[binding.clone()], &status, &records)
        .unwrap();

    let mut response_mutation = records.clone();
    response_mutation[0]["response"]["judge_id"] = value!("forged");
    let error = chrono_harness::full::validate_execution(
        &request,
        &[binding.clone()],
        &status,
        &response_mutation,
    )
    .unwrap_err();
    assert!(error.starts_with("E_SELF_DIAGNOSTIC:"), "{error}");

    let mut status_mutation = records.clone();
    status_mutation[0]["response"]["status"] = value!("fail");
    let error = chrono_harness::full::validate_execution(
        &request,
        &[binding],
        &wire::Status::Pass,
        &status_mutation,
    )
    .unwrap_err();
    assert!(error.starts_with("E_SELF_DIAGNOSTIC:"), "{error}");
}

#[test]
fn dag_blocks_dependents_and_continues_independent_branch() {
    let (_dir, r, b) = fixture(value!({"kind":"blocked-dag"}));
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
    let (status, records) = chrono_harness::full::execute(
        &r,
        &plan,
        &Default::default(),
        FIXTURE_TIMEOUT_SECONDS,
        8192,
    )
    .unwrap();
    assert_eq!(status, wire::Status::Error);
    assert_eq!(records[1]["state"], "blocked");
    assert_eq!(records[2]["id"], "independent");
    assert_eq!(records[2]["response"]["status"], "pass");
    chrono_harness::full::validate_execution(&r, &plan, &status, &records).unwrap();
    let mut broken = plan.clone();
    broken[0].after = vec!["missing".into()];
    assert!(chrono_harness::full::schedule(&broken).is_err());
    broken[0].after = vec!["blocked".into()];
    assert!(chrono_harness::full::schedule(&broken).is_err());
    assert!(chrono_harness::full::schedule(&[]).is_err());
}

#[test]
fn invalid_response_retains_actual_process_exit_in_report() {
    let (_dir, r, b) = fixture(value!({"kind":"raw", "stdout":b"invalid JSON\n", "exit":9}));
    let (status, records) =
        chrono_harness::full::execute(&r, &[b], &Default::default(), FIXTURE_TIMEOUT_SECONDS, 8192)
            .unwrap();
    assert_eq!(status, wire::Status::Error);
    assert_eq!(records[0]["state"], "executed");
    assert_eq!(records[0]["exit_code"], 9);
    assert_eq!(records[0]["process"]["stdout"], "invalid JSON\n");
}

#[test]
fn embedded_invalid_utf8_is_protocol_error_and_valid_replacement_passes() {
    for (bytes, accepted) in [(vec![239, 191, 189], true), (vec![255], false)] {
        let (_dir, r, b) = fixture(value!({
            "fields":{"outputs":{"note":"MARKER"}}, "replace_marker":bytes
        }));
        let (status, records) = chrono_harness::full::execute(
            &r,
            &[b],
            &Default::default(),
            FIXTURE_TIMEOUT_SECONDS,
            8192,
        )
        .unwrap();
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
    let (_dir, req, mut producer) = fixture(value!({"kind":"prior-process"}));
    producer.id = "producer".into();
    let mut consumer = producer.clone();
    consumer.id = "consumer".into();
    consumer.after = vec!["producer".into()];
    let (status, rows) = chrono_harness::full::execute(
        &req,
        &[consumer, producer],
        &Default::default(),
        FIXTURE_TIMEOUT_SECONDS,
        16384,
    )
    .unwrap();
    assert_eq!(status, wire::Status::Pass, "{rows:#?}");
    assert_eq!(rows.len(), 2);
}

#[test]
fn original_hex_bytes_cover_empty_and_every_byte_and_reject_corruption() {
    let bytes: Vec<u8> = (0..=255).collect();
    let expected = concat!(
        "000102030405060708090a0b0c0d0e0f",
        "101112131415161718191a1b1c1d1e1f",
        "202122232425262728292a2b2c2d2e2f",
        "303132333435363738393a3b3c3d3e3f",
        "404142434445464748494a4b4c4d4e4f",
        "505152535455565758595a5b5c5d5e5f",
        "606162636465666768696a6b6c6d6e6f",
        "707172737475767778797a7b7c7d7e7f",
        "808182838485868788898a8b8c8d8e8f",
        "909192939495969798999a9b9c9d9e9f",
        "a0a1a2a3a4a5a6a7a8a9aaabacadaeaf",
        "b0b1b2b3b4b5b6b7b8b9babbbcbdbebf",
        "c0c1c2c3c4c5c6c7c8c9cacbcccdcecf",
        "d0d1d2d3d4d5d6d7d8d9dadbdcdddedf",
        "e0e1e2e3e4e5e6e7e8e9eaebecedeeef",
        "f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff"
    );
    for (bytes, hex) in [(&bytes[..], expected), (&[][..], "")] {
        let artifact = chrono_harness::full::artifact(bytes);
        assert_eq!(artifact["hex"], hex);
        assert_eq!(artifact["length"], bytes.len());
        assert_eq!(artifact["sha256"], sha256(bytes));
        let originals = value!({"original":artifact});
        assert_eq!(
            chrono_harness::full::artifact_bytes(&originals, "original").unwrap(),
            bytes
        );
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            chrono_harness::retained_artifacts::bytes(root.path(), &originals, "original", None)
                .unwrap(),
            bytes
        );
        assert_eq!(
            chrono_harness::retained_artifacts::identity(root.path(), &originals, "original", None)
                .unwrap(),
            (sha256(bytes), bytes.len() as u64)
        );
        for field in ["hex", "length", "sha256"] {
            let mut corrupt = originals.clone();
            corrupt["original"][field] = match field {
                "hex" => "0g".into(),
                "length" => (bytes.len() + 1).into(),
                _ => "0".repeat(64).into(),
            };
            assert!(chrono_harness::full::artifact_bytes(&corrupt, "original").is_err());
            assert!(
                chrono_harness::retained_artifacts::identity(
                    root.path(),
                    &corrupt,
                    "original",
                    None
                )
                .is_err()
            );
        }
    }
}

#[test]
fn scoped_predecessors_keep_exact_originals_through_live_and_retained_dag() {
    let (_dir, mut req, mut first) = fixture(value!({"kind":"retained-process"}));
    req.candidate.root = fs::canonicalize(&req.candidate.root).unwrap();
    req.base.root = req.candidate.root.clone();
    req.scope = Some(chrono_harness::units::Scope::Unit { unit: "one".into() });
    req.observations = value!({"environment":{"effective":{}}});
    first.id = "a".into();
    let bindings: Vec<_> = [
        ("a", vec![]),
        ("b", vec!["a"]),
        ("c", vec!["b"]),
        ("d", vec!["a"]),
    ]
    .into_iter()
    .map(|(id, after)| {
        let mut binding = first.clone();
        binding.id = id.into();
        binding.after = after.into_iter().map(str::to_owned).collect();
        binding
    })
    .collect();
    let template = chrono_harness::full::judge_request(&req, &first, &[]).unwrap();
    let (status, records) = chrono_harness::full::execute(
        &template,
        &bindings,
        &Default::default(),
        FIXTURE_TIMEOUT_SECONDS,
        16384,
    )
    .unwrap();
    assert_eq!(status, wire::Status::Pass, "{records:#?}");
    let parsed = chrono_harness::full::retained_judges(&template, &bindings, &records).unwrap();
    assert_eq!(parsed.len(), 4);
    for (index, binding) in bindings.iter().enumerate() {
        let rebuilt =
            chrono_harness::full::judge_request(&template, binding, &records[..index]).unwrap();
        assert_eq!(records[index]["request_id"], rebuilt.request_id);
        assert_eq!(
            records[index]["request_digest"],
            wire::digest(&rebuilt).unwrap()
        );
        assert_eq!(
            parsed[&binding.id].3.stderr_bytes,
            (0..=255).collect::<Vec<u8>>()
        );
    }
    for field in [
        "stdin_sha256",
        "stdout_bytes",
        "stderr_bytes",
        "environment",
        "exit_code",
    ] {
        let mut corrupt = records.clone();
        corrupt[0]["process"][field] = match field {
            "stdout_bytes" | "stderr_bytes" => value!([1]),
            "environment" => value!({"changed":"yes"}),
            "exit_code" => value!(7),
            _ => value!("0".repeat(64)),
        };
        assert!(
            chrono_harness::full::retained_judges(&template, &bindings, &corrupt).is_err(),
            "{field}"
        );
    }
}
// Real nested runner launch: the outer process must finish only after its owned
// operation has been terminated. The PID is produced by that actual operation.
#[test]
fn owned_nested_helper() {
    let Ok(root) = std::env::var("CHRONO_NESTED_FIXTURE") else {
        return;
    };
    let root = std::path::Path::new(&root);
    let spec = chrono_harness::CommandSpec {
        program: PROCESS_FIXTURE.into(),
        args: vec!["nested".into()],
        env: Default::default(),
        timeout_seconds: 30,
        output_limit_bytes: 4096,
    };
    let _ = chrono_harness::run_process_observed(
        root,
        &spec,
        &[],
        &sha256(&fs::read(PROCESS_FIXTURE).unwrap()),
    );
}

#[test]
fn outer_timeout_contains_nested_owned_operation() {
    for helper in ["owned_nested_helper", "owned_nested_middle_helper"] {
        let d = tempfile::tempdir().unwrap();
        let exe = std::env::current_exe().unwrap();
        let spec = chrono_harness::CommandSpec {
            program: exe.to_string_lossy().into_owned(),
            args: vec!["--exact".into(), helper.into(), "--nocapture".into()],
            env: [
                (
                    "CHRONO_NESTED_FIXTURE".into(),
                    d.path().to_string_lossy().into_owned(),
                ),
                ("PATH".into(), std::env::var("PATH").unwrap()),
            ]
            .into(),
            timeout_seconds: 2,
            output_limit_bytes: 4096,
        };
        let p = chrono_harness::run_process_observed(
            d.path(),
            &spec,
            &[],
            &chrono_harness::sha256(&std::fs::read(&exe).unwrap()),
        )
        .unwrap();
        assert_eq!(p.failure.as_deref(), Some("process timed out"), "{p:?}");
        let pid = std::fs::read_to_string(d.path().join("nested.pid")).unwrap();
        let pid: libc::pid_t = pid.parse().unwrap();
        let state = unsafe { libc::kill(pid, 0) };
        let error = std::io::Error::last_os_error();
        if state == 0 {
            // Always terminate the exact fixture child before reporting a failure.
            assert_eq!(unsafe { libc::kill(pid, libc::SIGKILL) }, 0);
        }
        assert_eq!(state, -1, "nested process survived outer completion");
        assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
    }
}

#[test]
fn owned_nested_middle_helper() {
    let Ok(root) = std::env::var("CHRONO_NESTED_FIXTURE") else {
        return;
    };
    let exe = std::env::current_exe().unwrap();
    let spec = chrono_harness::CommandSpec {
        program: exe.to_string_lossy().into_owned(),
        args: vec![
            "--exact".into(),
            "owned_nested_helper".into(),
            "--nocapture".into(),
        ],
        env: std::env::vars().collect(),
        timeout_seconds: 30,
        output_limit_bytes: 4096,
    };
    let _ = chrono_harness::run_process_observed(
        std::path::Path::new(&root),
        &spec,
        &[],
        &chrono_harness::sha256(&std::fs::read(exe).unwrap()),
    );
}
#[test]
fn owned_timeout_helper_keeps_unrelated_sibling() {
    let Ok(root) = std::env::var("CHRONO_SIBLING_FIXTURE") else {
        return;
    };
    let root = std::path::Path::new(&root);
    let digest = sha256(&fs::read(PROCESS_FIXTURE).unwrap());
    std::thread::scope(|scope| {
        let slow = scope.spawn(|| {
            chrono_harness::run_process_observed(
                root,
                &chrono_harness::CommandSpec {
                    program: PROCESS_FIXTURE.into(),
                    args: vec!["slow".into()],
                    env: Default::default(),
                    timeout_seconds: 1,
                    output_limit_bytes: 4096,
                },
                &[],
                &digest,
            )
            .unwrap()
        });
        let sibling = chrono_harness::run_process_observed(
            root,
            &chrono_harness::CommandSpec {
                program: PROCESS_FIXTURE.into(),
                args: vec!["sibling".into()],
                env: Default::default(),
                timeout_seconds: 5,
                output_limit_bytes: 4096,
            },
            &[],
            &digest,
        )
        .unwrap();
        assert_eq!(
            slow.join().unwrap().failure.as_deref(),
            Some("process timed out")
        );
        assert_eq!(sibling.exit_code, 0);
        assert!(sibling.failure.is_none());
        assert_eq!(sibling.stdout_bytes, b"original\n");
    });
}
#[test]
fn nested_operation_timeout_does_not_cancel_owned_sibling() {
    let d = tempfile::tempdir().unwrap();
    let exe = std::env::current_exe().unwrap();
    let spec = chrono_harness::CommandSpec {
        program: exe.to_string_lossy().into_owned(),
        args: vec![
            "--exact".into(),
            "owned_timeout_helper_keeps_unrelated_sibling".into(),
            "--nocapture".into(),
        ],
        env: [
            (
                "CHRONO_SIBLING_FIXTURE".into(),
                d.path().to_string_lossy().into_owned(),
            ),
            ("PATH".into(), std::env::var("PATH").unwrap()),
        ]
        .into(),
        timeout_seconds: 5,
        output_limit_bytes: 4096,
    };
    let p = chrono_harness::run_process_observed(
        d.path(),
        &spec,
        &[],
        &chrono_harness::sha256(&std::fs::read(exe).unwrap()),
    )
    .unwrap();
    assert_eq!(p.exit_code, 0, "{p:?}");
    assert!(p.failure.is_none());
    assert_eq!(
        std::fs::read_to_string(d.path().join("sibling")).unwrap(),
        "joined"
    );
    assert_eq!(p.environment, spec.env);
}

#[test]
fn interrupted_launch_child_owner_helper() {
    let Ok(root) = std::env::var("CHRONO_INTERRUPTED_LAUNCH") else {
        return;
    };
    let root = std::path::Path::new(&root);
    let complete = std::env::var("CHRONO_INTERRUPTED_COMPLETE").unwrap();
    let _ = chrono_harness::run_process_observed(
        root,
        &chrono_harness::CommandSpec {
            program: PROCESS_FIXTURE.into(),
            args: vec!["interrupted-child".into(), complete],
            env: Default::default(),
            timeout_seconds: 30,
            output_limit_bytes: 4096,
        },
        &[],
        &sha256(&fs::read(PROCESS_FIXTURE).unwrap()),
    );
    panic!("the child must interrupt this launcher");
}

#[test]
fn interrupted_launch_survivor_helper() {
    use std::os::unix::process::ExitStatusExt;
    let Ok(root) = std::env::var("CHRONO_INTERRUPTED_LAUNCH") else {
        return;
    };
    let original = std::path::Path::new(&root).join("original");
    let bytes = fs::File::create(&original).unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "interrupted_launch_child_owner_helper",
            "--nocapture",
        ])
        .stdout(bytes.try_clone().unwrap())
        .stderr(bytes)
        .stdin(std::process::Stdio::null())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    assert_eq!(status.signal(), Some(9));
    fs::write(
        std::path::Path::new(&root).join("interruption"),
        "signal 9 joined by surviving parent",
    )
    .unwrap();
    if std::env::var("CHRONO_INTERRUPTED_COMPLETE").unwrap() == "yes" {
        let start = std::time::Instant::now();
        while !std::path::Path::new(&root).join("completed").exists() {
            assert!(start.elapsed() < std::time::Duration::from_secs(3));
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    print!("{}", fs::read_to_string(original).unwrap());
    if let Ok(bytes) = fs::read_to_string(std::path::Path::new(&root).join("child-original")) {
        print!("{bytes}");
    }
}

fn interrupted_launch(complete: bool) -> (tempfile::TempDir, chrono_harness::ProcessResult) {
    let d = tempfile::tempdir().unwrap();
    let exe = std::env::current_exe().unwrap();
    let spec = chrono_harness::CommandSpec {
        program: exe.to_string_lossy().into_owned(),
        args: vec![
            "--exact".into(),
            "interrupted_launch_survivor_helper".into(),
            "--nocapture".into(),
        ],
        env: [
            (
                "CHRONO_INTERRUPTED_LAUNCH".into(),
                d.path().to_string_lossy().into_owned(),
            ),
            (
                "CHRONO_INTERRUPTED_COMPLETE".into(),
                if complete { "yes" } else { "no" }.into(),
            ),
            ("PATH".into(), std::env::var("PATH").unwrap()),
        ]
        .into(),
        timeout_seconds: 5,
        output_limit_bytes: 4096,
    };
    // An independent process binds the child's live kernel identity before the
    // interruption and observes its actual exit. File readiness alone cannot
    // satisfy this oracle, including the abandoned-child cleanup case.
    let observer = std::process::Command::new(PROCESS_FIXTURE)
        .arg("interrupted-observer")
        .current_dir(d.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let result = chrono_harness::run_process_observed(
        d.path(),
        &spec,
        &[],
        &sha256(&fs::read(exe).unwrap()),
    );
    let observed = observer.wait_with_output().unwrap();
    assert!(
        observed.status.success(),
        "{}",
        String::from_utf8_lossy(&observed.stderr)
    );
    assert_eq!(
        fs::read(d.path().join("ready")).unwrap(),
        fs::read(d.path().join("kernel-terminal")).unwrap()
    );
    let p = result.unwrap();
    assert_eq!(p.environment, spec.env);
    assert_eq!(p.argv[1..], spec.args);
    (d, p)
}

#[test]
fn interrupted_nested_launcher_with_terminal_child_completes_enclosing_owner() {
    let (d, p) = interrupted_launch(true);
    assert!(d.path().join("ready").exists());
    assert_eq!(
        fs::read_to_string(d.path().join("interruption")).unwrap(),
        "signal 9 joined by surviving parent"
    );
    assert_eq!(
        fs::read_to_string(d.path().join("completed")).unwrap(),
        "child reached exit"
    );
    assert!(p.stdout.contains("original child bytes"));
    assert_eq!(p.exit_code, 0, "{p:?}");
    assert!(p.failure.is_none(), "{p:?}");
}

#[test]
fn interrupted_nested_launcher_with_live_child_retains_cleanup_failure() {
    let (d, p) = interrupted_launch(false);
    assert!(d.path().join("ready").exists());
    assert_eq!(p.exit_code, 0, "{p:?}");
    assert!(
        p.failure
            .as_deref()
            .is_some_and(|s| s.contains("joined completion")),
        "{p:?}"
    );
    assert!(!d.path().join("escaped").exists());
}

#[test]
fn owned_launch_burst_helper() {
    let Ok(root) = std::env::var("CHRONO_LAUNCH_BURST") else {
        return;
    };
    check_launch_allocations();
    let root = std::path::Path::new(&root);
    let digest = sha256(&fs::read(PROCESS_FIXTURE).unwrap());
    std::thread::scope(|scope| {
        let mut tasks = Vec::new();
        for index in 0..64 {
            let digest = &digest;
            tasks.push(scope.spawn(move || {
                let spec = chrono_harness::CommandSpec {
                    program: PROCESS_FIXTURE.into(),
                    args: vec!["burst".into(), index.to_string()],
                    env: Default::default(),
                    timeout_seconds: 30,
                    output_limit_bytes: 4096,
                };
                let p = chrono_harness::run_process_observed(root, &spec, &[], digest).unwrap();
                assert_eq!(p.exit_code, 0);
                assert!(p.failure.is_none(), "{p:?}");
                assert_eq!(p.stdout, format!("original-{index}\n"));
            }));
        }
        for task in tasks {
            task.join().unwrap();
        }
    });
}
#[test]
fn concurrent_live_identity_handoffs_complete_each_real_child_once() {
    let d = tempfile::tempdir().unwrap();
    let exe = std::env::current_exe().unwrap();
    let spec = chrono_harness::CommandSpec {
        program: exe.to_string_lossy().into(),
        args: vec![
            "--exact".into(),
            "owned_launch_burst_helper".into(),
            "--nocapture".into(),
        ],
        env: [
            (
                "CHRONO_LAUNCH_BURST".into(),
                d.path().to_string_lossy().into(),
            ),
            ("PATH".into(), std::env::var("PATH").unwrap()),
        ]
        .into(),
        timeout_seconds: 30,
        output_limit_bytes: 65536,
    };
    let p = chrono_harness::run_process_observed(
        d.path(),
        &spec,
        &[],
        &sha256(&fs::read(exe).unwrap()),
    )
    .unwrap();
    assert_eq!(p.exit_code, 0, "{} {} {:?}", p.stdout, p.stderr, p.failure);
    assert!(p.failure.is_none());
    for index in 0..64 {
        assert_eq!(
            fs::read_to_string(d.path().join(format!("child-{index}"))).unwrap(),
            "once"
        );
    }
}

#[cfg(unix)]
#[test]
fn delayed_monitor_helper() {
    let Ok(root) = std::env::var("CHRONO_DELAYED_MONITOR") else {
        return;
    };
    let root = std::path::Path::new(&root);
    let executable = PROCESS_FIXTURE;
    let digest = sha256(&fs::read(executable).unwrap());
    let spec = chrono_harness::CommandSpec {
        program: executable.into(),
        args: vec!["child".into()],
        env: Default::default(),
        timeout_seconds: 1,
        output_limit_bytes: 4096,
    };
    // Complete this launcher's first owned process handoff before recording the
    // short experiment's conservative start. The probe uses the same executable,
    // ownership engine and bounds, and leaves no child or readiness marker.
    let mut readiness = spec.clone();
    readiness.args = vec!["exit".into(), "0".into()];
    let ready = chrono_harness::run_process_observed(root, &readiness, &[], &digest).unwrap();
    assert_eq!(ready.exit_code, 0, "{ready:?}");
    assert!(ready.failure.is_none(), "{ready:?}");
    assert!(ready.stdout_bytes.is_empty(), "{ready:?}");
    assert!(ready.stderr_bytes.is_empty(), "{ready:?}");
    let mut now: libc::timespec = unsafe { std::mem::zeroed() };
    #[cfg(target_vendor = "apple")]
    let clock = libc::CLOCK_UPTIME_RAW;
    #[cfg(not(target_vendor = "apple"))]
    let clock = libc::CLOCK_MONOTONIC;
    assert_eq!(unsafe { libc::clock_gettime(clock, &mut now) }, 0);
    fs::write(
        root.join("launcher"),
        serde_json::to_vec(&value!({
            "pid": std::process::id(),
            "started": now.tv_sec as f64 + now.tv_nsec as f64 / 1e9,
        }))
        .unwrap(),
    )
    .unwrap();
    let result = chrono_harness::run_process_observed(root, &spec, &[], &digest).unwrap();
    fs::write(
        root.join("result.json"),
        serde_json::to_vec(&result).unwrap(),
    )
    .unwrap();
}

#[cfg(unix)]
fn delayed_monitor(case: &str) -> chrono_harness::ProcessResult {
    let root = tempfile::tempdir().unwrap();
    let exe = std::env::current_exe().unwrap();
    let digest = sha256(&fs::read(&exe).unwrap());
    // Only the nested launcher is suspended. The child and independent kernel
    // observer remain runnable; the late-exit control rejects a monitor that
    // merely tests try_wait before checking its deadline.
    let mut controller = std::process::Command::new(PROCESS_FIXTURE)
        .args(["controller", case])
        .current_dir(root.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    // The independent observer publishes readiness before the timed launch.
    // The launcher timestamp conservatively precedes the actual timer start.
    let preparing = std::time::Instant::now();
    while !root.path().join("controller-ready").exists() {
        if preparing.elapsed() >= std::time::Duration::from_secs(5)
            || controller.try_wait().unwrap().is_some()
        {
            let _ = controller.kill();
            let original = controller.wait_with_output().unwrap();
            panic!("controller did not become ready: {original:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let spec = chrono_harness::CommandSpec {
        program: exe.to_string_lossy().into(),
        args: vec![
            "--exact".into(),
            "delayed_monitor_helper".into(),
            "--nocapture".into(),
        ],
        env: [
            (
                "CHRONO_DELAYED_MONITOR".into(),
                root.path().to_string_lossy().into(),
            ),
            ("PATH".into(), std::env::var("PATH").unwrap()),
        ]
        .into(),
        timeout_seconds: 8,
        output_limit_bytes: 16384,
    };
    let result = chrono_harness::run_process_observed(root.path(), &spec, &[], &digest);
    let observed = controller.wait_with_output().unwrap();
    assert!(
        observed.status.success(),
        "{}",
        String::from_utf8_lossy(&observed.stderr)
    );
    let result = result.unwrap();
    assert_eq!(result.exit_code, 0, "{result:?}");
    assert!(result.failure.is_none(), "{result:?}");
    let observed: serde_json::Value =
        serde_json::from_slice(&fs::read(root.path().join("kernel-terminal.json")).unwrap())
            .unwrap();
    eprintln!("delayed monitor kernel evidence ({case}): {observed}");
    assert!(observed["resumed"].as_f64().unwrap() - observed["stopped"].as_f64().unwrap() > 1.0);
    let result: chrono_harness::ProcessResult =
        serde_json::from_slice(&fs::read(root.path().join("result.json")).unwrap()).unwrap();
    assert_eq!(result.exit_code, 7, "{result:?}; kernel: {observed}");
    if case == "overflow" {
        assert_eq!(result.stdout_bytes, vec![b'x'; 4096]);
    } else {
        assert!(result.stdout_bytes.is_empty());
    }
    assert!(result.stderr_bytes.is_empty());
    result
}

#[cfg(unix)]
#[test]
fn delayed_monitor_preserves_on_time_child_exit() {
    let result = delayed_monitor("on-time");
    assert!(result.failure.is_none(), "{result:?}");
}

#[cfg(unix)]
#[test]
fn delayed_monitor_retains_late_child_timeout() {
    let result = delayed_monitor("late");
    assert_eq!(
        result.failure.as_deref(),
        Some("process timed out"),
        "{result:?}"
    );
}

#[cfg(unix)]
#[test]
fn delayed_monitor_preserves_output_bound() {
    let result = delayed_monitor("overflow");
    assert_eq!(
        result.failure.as_deref(),
        Some("process output limit exceeded"),
        "{result:?}"
    );
}

#[cfg(unix)]
#[test]
fn closed_output_streams_preserve_child_exit_and_timeout() {
    for (delay, expected_exit) in [(50, Some(7)), (20_000, None)] {
        let root = tempfile::tempdir().unwrap();
        let spec = chrono_harness::CommandSpec {
            program: PROCESS_FIXTURE.into(),
            args: vec!["closed-streams".into(), delay.to_string()],
            env: Default::default(),
            timeout_seconds: 1,
            output_limit_bytes: 4096,
        };
        let result = chrono_harness::run_process_observed(
            root.path(),
            &spec,
            &[],
            &sha256(&fs::read(&spec.program).unwrap()),
        )
        .unwrap();
        assert!(result.stdout_bytes.is_empty());
        assert!(result.stderr_bytes.is_empty());
        let markers = (
            fs::read_to_string(root.path().join("started")),
            fs::read_to_string(root.path().join("completed")),
        );
        if let Some(exit) = expected_exit {
            assert_eq!(result.exit_code, exit, "{result:?}; markers={markers:?}");
            assert!(result.failure.is_none(), "{result:?}; markers={markers:?}");
            assert!(root.path().join("completed").exists());
        } else {
            assert_eq!(
                result.failure.as_deref(),
                Some("process timed out"),
                "{result:?}; markers={markers:?}"
            );
            assert!(!root.path().join("completed").exists());
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn unexpected_exec_helper() {
    if std::env::var_os("CHRONO_UNEXPECTED_EXEC").is_some() {
        fs::File::create_new("unexpected-exec").unwrap();
    }
}

#[cfg(target_os = "macos")]
#[test]
fn failed_live_identity_registration_helper() {
    use std::os::fd::AsRawFd;
    let Ok(root) = std::env::var("CHRONO_FAILED_HANDOFF") else {
        return;
    };
    check_launch_allocations();
    // The independent Rust parent fixes this process's descriptor budget.
    // Leave the ownership transport free, while exhausting the observer's
    // remaining kernel descriptor when it receives the live handoff.
    let mut held = Vec::new();
    loop {
        let file = fs::File::open("/dev/null").unwrap();
        let fd = file.as_raw_fd();
        held.push(file);
        if fd >= 195 {
            break;
        }
    }
    let spec = chrono_harness::CommandSpec {
        program: std::env::current_exe().unwrap().to_str().unwrap().into(),
        args: vec![
            "--exact".into(),
            "unexpected_exec_helper".into(),
            "--nocapture".into(),
        ],
        env: [("CHRONO_UNEXPECTED_EXEC".into(), "1".into())].into(),
        timeout_seconds: 30,
        output_limit_bytes: 4096,
    };
    let error = chrono_harness::run_process_observed(
        std::path::Path::new(&root),
        &spec,
        &[],
        &sha256(&fs::read(&spec.program).unwrap()),
    )
    .expect_err("kernel registration exhaustion must prevent exec");
    println!("{error}");
    assert!(
        error.contains("process ownership live exit registration"),
        "{error}"
    );
    assert!(error.contains("os error 24"), "{error}");
    assert!(
        error.split(';').next().unwrap().contains("os error 24"),
        "spawn must preserve the actual errno independently of its diagnostic: {error}"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn failed_live_identity_registration_preserves_actual_kernel_error_before_exec() {
    failed_live_identity_handoff(210, "failed_live_identity_registration_helper");
}

#[cfg(target_os = "macos")]
#[test]
fn failed_handoff_receive_preserves_original_transport_failure_before_exec() {
    failed_live_identity_handoff(256, "failed_handoff_receive_helper");
}

#[cfg(target_os = "macos")]
fn failed_live_identity_handoff(descriptor_limit: libc::rlim_t, helper: &str) {
    use std::os::unix::process::CommandExt;
    let root = tempfile::tempdir().unwrap();
    let descriptors: Vec<i32> = std::env::var("CHRONO_PROCESS_FDS")
        .ok()
        .map(|value| value.split(',').map(|fd| fd.parse().unwrap()).collect())
        .unwrap_or_default();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", helper, "--nocapture"])
        .env("CHRONO_FAILED_HANDOFF", root.path())
        .env_remove("CHRONO_PROCESS_FDS");
    unsafe {
        command.pre_exec(move || {
            // Keep inherited leases open above the fixed allocation budget.
            // F_DUPFD creates inheritable copies; this helper omits their carrier.
            for fd in &descriptors {
                if libc::fcntl(*fd, libc::F_DUPFD, descriptor_limit as i32) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            for fd in &descriptors {
                if libc::close(*fd) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            for fd in [198, 199, 200] {
                if libc::close(fd) < 0 {
                    let error = std::io::Error::last_os_error();
                    if error.raw_os_error() != Some(libc::EBADF) {
                        return Err(error);
                    }
                }
            }
            let limit = libc::rlimit {
                rlim_cur: descriptor_limit,
                rlim_max: descriptor_limit,
            };
            if libc::setrlimit(libc::RLIMIT_NOFILE, &limit) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let out = command.output().unwrap();
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!root.path().join("unexpected-exec").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn fixed_descriptor_exhaustion_keeps_inherited_capabilities_outside_its_budget() {
    use std::os::fd::AsFd;
    let root = tempfile::tempdir().unwrap();
    let capability = fs::File::open(root.path()).unwrap();
    let _ownership = chrono_harness::process_fds::Scope::new(&[capability.as_fd()]).unwrap();
    let executable = std::env::current_exe().unwrap();
    let spec = chrono_harness::CommandSpec {
        program: executable.to_str().unwrap().into(),
        args: vec![
            "--exact".into(),
            "failed_live_identity_registration_preserves_actual_kernel_error_before_exec".into(),
            "--nocapture".into(),
        ],
        env: Default::default(),
        timeout_seconds: FIXTURE_TIMEOUT_SECONDS,
        output_limit_bytes: 4096,
    };
    let result = chrono_harness::run_process_observed(
        root.path(),
        &spec,
        &[],
        &sha256(&fs::read(&executable).unwrap()),
    )
    .unwrap();
    assert!(result.ownership_fds.is_some(), "{result:?}");
    assert!(result.failure.is_none(), "{result:?}");
    assert_eq!(result.exit_code, 0, "{result:?}");
    assert!(result.stdout.contains("1 passed; 0 failed"), "{result:?}");
}

#[test]
fn signal_termination_preserves_actual_pid_and_streams_without_reclassifying_normal_exits() {
    let dir = tempfile::tempdir().unwrap();
    let spec = chrono_harness::CommandSpec {
        program: PROCESS_FIXTURE.into(),
        args: vec!["terminated".into(), "streams".into()],
        env: Default::default(),
        timeout_seconds: FIXTURE_TIMEOUT_SECONDS,
        output_limit_bytes: 4096,
    };
    let identity = sha256(&fs::read(PROCESS_FIXTURE).unwrap());
    let result = chrono_harness::run_process_observed(dir.path(), &spec, &[], &identity).unwrap();
    let pid = fs::read_to_string(dir.path().join("terminated.pid")).unwrap();
    assert!(pid.parse::<u32>().unwrap() > 0);
    assert_eq!(result.exit_code, -1);
    assert_eq!(
        result.failure,
        Some(format!("process terminated by signal 9 (pid {pid})"))
    );
    assert_eq!(result.stdout_bytes, b"original stdout\xff");
    assert_eq!(result.stderr_bytes, b"original stderr\xfe");
    assert_eq!(result.stdout_sha256, sha256(&result.stdout_bytes));
    assert_eq!(result.stderr_sha256, sha256(&result.stderr_bytes));
    for code in [0, 7] {
        let mut ordinary = spec.clone();
        ordinary.args = vec!["exit".into(), code.to_string()];
        let result =
            chrono_harness::run_process_observed(dir.path(), &ordinary, &[], &identity).unwrap();
        assert_eq!(result.exit_code, code);
        assert_eq!(result.failure, None);
    }
}

#[test]
fn signal_termination_precedes_json_decode_for_delta_and_initial_transport() {
    let (dir, request, mut binding) = fixture(value!({}));
    replace_judge(dir.path(), PROCESS_FIXTURE);
    binding.sha256 = Some(sha256(&fs::read(PROCESS_FIXTURE).unwrap()));
    binding.argv = vec!["terminated".into(), "empty".into()];
    let failure = wire::invoke_detailed(
        &request,
        &binding,
        &Default::default(),
        FIXTURE_TIMEOUT_SECONDS,
        4096,
    )
    .unwrap_err();
    let pid = fs::read_to_string(dir.path().join("terminated.pid")).unwrap();
    let expected = format!("process terminated by signal 9 (pid {pid})");
    assert!(failure.message.contains(&expected), "{failure:?}");
    let process = failure.process.unwrap();
    assert_eq!(process.failure.as_deref(), Some(expected.as_str()));
    assert_eq!(process.exit_code, -1);
    assert!(process.stdout_bytes.is_empty());

    let (dir, request, mut binding) = initial_fixture(value!({}));
    replace_judge(dir.path(), PROCESS_FIXTURE);
    binding.sha256 = Some(sha256(&fs::read(PROCESS_FIXTURE).unwrap()));
    binding.argv = vec!["terminated".into(), "empty".into()];
    let failure = chrono_harness::initial::invoke(
        &request,
        &binding,
        &Default::default(),
        FIXTURE_TIMEOUT_SECONDS,
        4096,
    )
    .unwrap_err();
    let pid = fs::read_to_string(dir.path().join("terminated.pid")).unwrap();
    let expected = format!("process terminated by signal 9 (pid {pid})");
    assert!(failure.message.contains(&expected), "{failure:?}");
    let process = failure.process.unwrap();
    assert_eq!(process.failure.as_deref(), Some(expected.as_str()));
    assert_eq!(process.exit_code, -1);
    assert!(process.stdout_bytes.is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn failed_handoff_receive_helper() {
    use std::os::unix::fs::FileExt;
    let Ok(root) = std::env::var("CHRONO_FAILED_HANDOFF") else {
        return;
    };
    let root = std::path::Path::new(&root);
    let spec = chrono_harness::CommandSpec {
        program: std::env::current_exe().unwrap().to_str().unwrap().into(),
        args: vec![
            "--exact".into(),
            "failed_handoff_receive_child_helper".into(),
            "--nocapture".into(),
        ],
        env: [(
            "CHRONO_FAILED_HANDOFF".into(),
            root.to_string_lossy().into(),
        )]
        .into(),
        timeout_seconds: 30,
        output_limit_bytes: 65536,
    };
    let digest = sha256(&fs::read(&spec.program).unwrap());
    let release = fs::File::create_new(root.join("release")).unwrap();
    let result = std::thread::scope(|scope| {
        let task = scope.spawn(|| chrono_harness::run_process_observed(root, &spec, &[], &digest));
        let started = std::time::Instant::now();
        while !root.join("ready").exists() {
            assert!(started.elapsed().as_secs() < 10, "child readiness missing");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        // Only the observer process exhausts descriptors. The already-running
        // nested launcher retains its independent descriptor budget.
        let mut held = Vec::new();
        loop {
            match fs::File::open("/dev/null") {
                Ok(file) => held.push(file),
                Err(error) => {
                    assert_eq!(error.raw_os_error(), Some(libc::EMFILE));
                    break;
                }
            }
        }
        release.write_all_at(b"go", 0).unwrap();
        let result = task.join().unwrap();
        drop(held);
        result
    })
    .unwrap();
    assert_eq!(
        result.exit_code, 0,
        "{} {} {:?}",
        result.stdout, result.stderr, result.failure
    );
    assert!(result.failure.is_none(), "{result:?}");
    assert!(!root.join("unexpected-exec").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn failed_handoff_receive_child_helper() {
    let Ok(root) = std::env::var("CHRONO_FAILED_HANDOFF") else {
        return;
    };
    check_launch_allocations();
    let root = std::path::Path::new(&root);
    fs::File::create_new(root.join("ready")).unwrap();
    let started = std::time::Instant::now();
    while fs::metadata(root.join("release")).unwrap().len() == 0 {
        assert!(started.elapsed().as_secs() < 10, "release missing");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let spec = chrono_harness::CommandSpec {
        program: std::env::current_exe().unwrap().to_str().unwrap().into(),
        args: vec![
            "--exact".into(),
            "unexpected_exec_helper".into(),
            "--nocapture".into(),
        ],
        env: [("CHRONO_UNEXPECTED_EXEC".into(), "1".into())].into(),
        timeout_seconds: 30,
        output_limit_bytes: 4096,
    };
    let error = chrono_harness::run_process_observed(
        root,
        &spec,
        &[],
        &sha256(&fs::read(&spec.program).unwrap()),
    )
    .expect_err("unreceived identity must prevent exec");
    println!("{error}");
    assert!(error.contains("poll timeout"), "{error}");
    assert!(
        error.contains("observer receive"),
        "missing receiving endpoint diagnostic: {error}"
    );
    // macOS reports EMSGSIZE when it cannot externalize SCM_RIGHTS into
    // the exhausted receiving descriptor table; retain that original errno.
    assert!(
        error.contains(&format!("os error {}", libc::EMSGSIZE)),
        "missing original recvmsg errno: {error}"
    );
    assert!(error.contains("observer incomplete handoffs"), "{error}");
    assert!(
        error.contains("not attributed to this child"),
        "shared transport observations must not become a child-specific cause: {error}"
    );
    assert!(
        error.split(';').next().unwrap().contains("os error 22"),
        "observer diagnostics must preserve the actual spawn errno: {error}"
    );
}
