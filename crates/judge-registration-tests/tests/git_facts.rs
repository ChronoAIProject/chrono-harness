use super::*;
use std::os::unix::fs::PermissionsExt;
#[path = "git_inputs.rs"]
mod git_inputs;
#[path = "input_coverage.rs"]
mod input_coverage;

const CONFIG: &str = ".chrono-harness/config.json";

fn quoted(path: &Path) -> String {
    format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"))
}

struct BoundHost {
    host: Host,
    program: PathBuf,
    trace: PathBuf,
    shadow: PathBuf,
}
impl BoundHost {
    fn new(initial: bool, body: &str) -> Self {
        let mut h = if initial { initial_host() } else { Host::new() };
        let root = fs::canonicalize(h.root()).unwrap();
        let state = root.join(".chrono-harness/state");
        fs::create_dir_all(&state).unwrap();
        let program = state.join("chosen Git λ");
        let trace = state.join("chosen-git.trace");
        let real = chrono_harness::resolve_program(&root, "git", None).unwrap();
        let version = Command::new(&real).arg("--version").output().unwrap();
        assert!(version.status.success());
        let script = format!(
            "#!/bin/sh\n[ \"$BOUND_FACTS_VALUE\" = declared ] || exit 81\n[ -z \"$CHRONO_AMBIENT_PROBE\" ] || exit 82\nprintf '%s\\n' \"$*\" >> \"$CHRONO_FACTS_TRACE\" || exit $?\n{}\nexec {} \"$@\"\n",
            body.replace("REAL_GIT", &quoted(&real)),
            quoted(&real)
        );
        fs::write(&program, script).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        let shadow = state.join("ambient-bin");
        fs::create_dir(&shadow).unwrap();
        fs::write(
            shadow.join("git"),
            "#!/bin/sh\necho ambient-git-must-not-run >&2\nexit 83\n",
        )
        .unwrap();
        fs::set_permissions(shadow.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
        let mut config: Value =
            serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
        config["schema_version"] = json!(3);
        config["facts_git"] = json!({"tool":"facts-git","input":"facts-git-bytes"});
        config["tools"] = json!([{"id":"facts-git","program":program,"resolution":"PATH-once",
            "version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}]);
        config["environment"] = json!({"inherit":[],"values":{"PATH":std::env::var("PATH").unwrap(),
            "CHRONO_FACTS_TRACE":trace,"BOUND_FACTS_VALUE":"declared","GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"},
            "inputs":[{"id":"facts-git-bytes","location":program,"presence":"present","sha256":sha256(&fs::read(&program).unwrap())}]});
        config["input_closure"] = json!({
            "status": "declared-complete",
            "unresolved": [],
            "bindings": [{
                "id": "registration-git-facts",
                "consumer": "judge:registration",
                "kind": "git-facts",
                "inputs": [
                    "tool:facts-git",
                    "input:facts-git-bytes",
                    "environment:PATH",
                    "environment:CHRONO_FACTS_TRACE",
                    "environment:BOUND_FACTS_VALUE",
                    "environment:GIT_CONFIG_NOSYSTEM",
                    "environment:GIT_CONFIG_GLOBAL"
                ]
            }]
        });
        write(&root, CONFIG, &config);
        let path = ".chrono-harness/FILEMAP.json";
        let mut map: Value = serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap();
        map["project_edges"] = json!([
            {"from":"tool:facts-git","kind":"runtime-input","to":"judge:registration"},
            {"from":"input:facts-git-bytes","kind":"judge-trigger","to":"judge:registration"},
            {"from":"environment:PATH","kind":"runtime-input","to":"judge:registration"},
            {"from":"environment:CHRONO_FACTS_TRACE","kind":"runtime-input","to":"judge:registration"},
            {"from":"environment:BOUND_FACTS_VALUE","kind":"runtime-input","to":"judge:registration"},
            {"from":"environment:GIT_CONFIG_NOSYSTEM","kind":"runtime-input","to":"judge:registration"},
            {"from":"environment:GIT_CONFIG_GLOBAL","kind":"runtime-input","to":"judge:registration"}
        ]);
        write(&root, path, &map);
        if initial {
            amend_initial(&mut h);
        } else {
            h.base = commit(&root);
            fs::write(root.join("document.txt"), "new candidate using bound facts").unwrap();
            h.candidate = commit(&root);
        }
        Self {
            host: h,
            program,
            trace,
            shadow,
        }
    }
    fn run(&self, initial: bool) -> (i32, Value, String) {
        let h = &self.host;
        let mut context = h.context();
        let config: Value =
            serde_json::from_slice(&fs::read(h.root().join(CONFIG)).unwrap()).unwrap();
        let snapshot = |oid: &str| {
            json!({"schema":"chrono-input-snapshot/v2","commit":oid,
            "config_path":CONFIG,"config_digest":chrono_harness::wire::digest(&config).unwrap(),
            "environment":{},"files":{"facts-git-bytes":{"bytes":fs::read(&self.program).unwrap()}}})
        };
        write(
            h.root(),
            ".chrono-harness/state/retained.json",
            &json!({"base":snapshot(&h.base),"candidate":snapshot(&h.candidate)}),
        );
        context["retained_inputs"] = json!(".chrono-harness/state/retained.json");
        write(h.root(), ".chrono-harness/state/context.json", &context);
        let profile = if initial {
            ".chrono-harness/initial.json"
        } else {
            CONFIG
        };
        let mut command = Command::new(h.root().join(".chrono-harness/bin/chrono-harness"));
        command
            .current_dir("/")
            .env_clear()
            .env("PATH", &self.shadow)
            .env("CHRONO_AMBIENT_PROBE", "must-not-leak")
            .args([
                "check",
                "--config",
                h.root().join(profile).to_str().unwrap(),
                "--candidate",
                &h.candidate,
            ]);
        if initial {
            command.arg("--initial");
        } else {
            command.args([
                "--base",
                &h.base,
                "--context",
                ".chrono-harness/state/context.json",
            ]);
        }
        let output = command.output().unwrap();
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        let report = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
        (output.status.code().unwrap(), report, diagnostic)
    }
    fn trace(&self) -> String {
        fs::read_to_string(&self.trace).unwrap_or_default()
    }
}

// Retain each endpoint's own configuration. Adopting coverage must not rewrite
// the old v3 config or require executing the previous judge binary.
fn run_with_endpoint_inputs(h: &BoundHost) -> (i32, Value) {
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
fn full_check_uses_registered_git_for_acquisition_and_registration() {
    let h = BoundHost::new(false, "");
    let (exit, report, error) = h.run(false);
    assert_eq!(
        exit, 0,
        "registered facts must ignore ambient Git/environment: {error} {report}"
    );
    assert_eq!(report["status"], "pass");
    assert_eq!(report["git_facts"]["binding"]["path"], json!(h.program));
    assert_eq!(
        report["judges"][0]["response"]["outputs"]["git_facts"]["binding"]["path"],
        json!(h.program)
    );
    let trace = h.trace();
    assert!(
        trace.contains("rev-parse --show-toplevel"),
        "registration must recheck through chosen Git: {trace}"
    );
    assert!(
        trace.matches("ls-tree").count() >= 4,
        "both runner and registration must read both endpoints: {trace}"
    );
    for p in report["git_facts"]["processes"].as_array().unwrap() {
        assert_eq!(p["sha256"], json!(sha256(&fs::read(&h.program).unwrap())));
        assert_eq!(p["exit_code"], 0);
        for stream in ["stdout", "stderr"] {
            let bytes: Vec<u8> =
                serde_json::from_value(p[format!("{stream}_bytes")].clone()).unwrap();
            assert_eq!(p[format!("{stream}_sha256")], json!(sha256(&bytes)));
        }
    }
}

#[test]
fn initial_inventory_uses_registered_git_and_keeps_initial_scope() {
    let h = BoundHost::new(true, "");
    let (exit, report, error) = h.run(true);
    assert_eq!(
        exit, 0,
        "initial inventory must use declared Git: {error} {report}"
    );
    assert_eq!(report["governance"], "not-evaluated");
    assert_eq!(report["git_facts"]["binding"]["path"], json!(h.program));
    assert!(h.trace().contains("cat-file commit"));
    assert!(h.trace().contains("rev-parse --show-toplevel"));
}

#[test]
fn full_and_initial_bound_checks_reject_configuration_hidden_mode_changes() {
    for initial in [false, true] {
        let h = BoundHost::new(initial, "");
        git(h.host.root(), &["config", "core.filemode", "false"]);
        fs::set_permissions(
            h.host.root().join("document.txt"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(
            git(
                h.host.root(),
                &["diff", "--name-only", &h.host.candidate, "--"]
            )
            .is_empty()
        );
        let (exit, report, error) = h.run(initial);
        assert_ne!(exit, 0, "{initial}: {report} {error}");
        if initial {
            assert!(
                format!("{report} {error}")
                    .contains("initial candidate checkout is dirty or changed")
            );
        } else {
            assert!(
                report["findings"]
                    .as_array()
                    .is_some_and(|findings| findings
                        .iter()
                        .any(|f| f["code"] == "E_SNAPSHOT_DIRTY")),
                "findings={}, error={error}",
                report["findings"]
            );
        }
        assert!(h.trace().contains("ls-files --stage -z"));
        assert!(
            !h.trace().contains("diff"),
            "raw identity must not delegate to filters"
        );
    }
}

#[test]
fn registered_git_rejects_digest_and_version_before_object_acquisition() {
    for fault in ["digest", "version"] {
        let mut h = BoundHost::new(false, "");
        h.host.edit(CONFIG, |c| {
            if fault == "digest" {
                c["environment"]["inputs"][0]["sha256"] = json!("0".repeat(64));
            } else {
                c["tools"][0]["expected_version"] = json!("wrong declared Git version");
            }
        });
        let (exit, report, error) = h.run(false);
        assert_ne!(exit, 0, "{fault}: {report}");
        assert!(
            error.contains("E_GIT_FACTS"),
            "named binding failure required: {fault}: {error} {report}"
        );
        assert!(
            !h.trace().contains("rev-parse"),
            "no objects may be read after binding failure"
        );
        if fault == "digest" {
            assert!(
                h.trace().is_empty(),
                "digest mismatch must precede any invocation"
            );
        } else {
            assert_eq!(h.trace().trim(), "--version");
            assert!(error.contains("git version"));
        }
    }
}

#[test]
fn registered_git_rechecks_bound_bytes_after_successful_process() {
    let h = BoundHost::new(
        false,
        "case \"$*\" in *'rev-parse'*)\n REAL_GIT \"$@\" || exit $?\n printf '\\n# persistent drift\\n' >> \"$0\"\n exit 0;;\nesac",
    );
    let before = fs::read(&h.program).unwrap();
    let (exit, report, error) = h.run(false);
    assert_ne!(exit, 0, "{report}");
    assert!(
        error.contains("E_GIT_FACTS") && error.contains("changed"),
        "{error} {report}"
    );
    assert_ne!(
        fs::read(&h.program).unwrap(),
        before,
        "consumer must have caused the tested persistent change"
    );
    assert!(
        error.contains("\"exit_code\":0"),
        "retain the successful subprocess before postcondition failure: {error}"
    );
}

#[test]
fn registered_git_failure_retains_original_output_and_exit() {
    let h = BoundHost::new(
        false,
        "case \"$*\" in *'ls-tree'*) printf 'a\\000b'; printf 'selected-facts-failure' >&2; exit 17;; esac",
    );
    let (exit, report, error) = h.run(false);
    assert_ne!(exit, 0, "{report}");
    assert!(
        error.contains("E_GIT_FACTS") && error.contains("selected-facts-failure"),
        "{error}"
    );
    assert!(error.contains("\"exit_code\":17"), "{error}");
    assert!(
        error.contains("\"stdout_bytes\":[97,0,98]"),
        "raw process bytes must survive: {error}"
    );
}

#[test]
fn legacy_configs_reject_new_git_binding_without_changing_legacy_success() {
    for (version, binding) in [
        (1, Value::Null),
        (2, Value::Null),
        (1, json!({"tool":"invalid","input":"none"})),
        (2, json!({"tool":"invalid","input":"none"})),
    ] {
        let mut h = Host::new();
        assert_eq!(h.run().0, 0);
        h.edit(CONFIG, |c| {
            c["schema_version"] = json!(version);
            c["facts_git"] = binding;
        });
        let (exit, report) = h.run();
        assert_ne!(
            exit, 0,
            "old configuration must not adopt new semantics: {report}"
        );
    }
}

#[test]
fn request_binding_and_environment_cannot_be_missing_or_substituted() {
    let h = BoundHost::new(false, "");
    let reader = facts::Reader::for_config(h.host.root(), CONFIG).unwrap();
    let observation = reader.observation();
    let valid = json!({"git_facts":observation,"environment":observation["environment"]});
    facts::Reader::from_observations(h.host.root(), CONFIG, &h.host.candidate, &valid).unwrap();
    for fault in ["missing", "path", "digest", "config", "environment"] {
        let mut changed = valid.clone();
        match fault {
            "missing" => {
                changed.as_object_mut().unwrap().remove("git_facts");
            }
            "path" => changed["git_facts"]["binding"]["path"] = json!("/different/git"),
            "digest" => changed["git_facts"]["binding"]["sha256"] = json!("f".repeat(64)),
            "config" => changed["git_facts"]["config_sha256"] = json!("e".repeat(64)),
            _ => changed["environment"]["effective"]["BOUND_FACTS_VALUE"] = json!("ambient"),
        }
        let before = h.trace();
        let err =
            facts::Reader::from_observations(h.host.root(), CONFIG, &h.host.candidate, &changed)
                .err()
                .expect(fault);
        assert!(err.contains("E_GIT_FACTS"), "{fault}: {err}");
        assert_eq!(
            h.trace(),
            before,
            "invalid request must not launch Git: {fault}"
        );
    }
}

#[test]
fn invalid_git_references_fail_before_version_or_objects() {
    for fault in [
        "null",
        "missing-tool",
        "duplicate-tool",
        "absent-input",
        "different-location",
        "redirect",
    ] {
        let mut h = BoundHost::new(false, "");
        let other = h.program.with_extension("other");
        fs::copy(&h.program, &other).unwrap();
        h.host.edit(CONFIG, |cfg| match fault {
            "null" => cfg["facts_git"] = Value::Null,
            "missing-tool" => cfg["facts_git"]["tool"] = json!("no-such-tool"),
            "duplicate-tool" => {
                let row = cfg["tools"][0].clone();
                cfg["tools"].as_array_mut().unwrap().push(row);
            }
            "absent-input" => cfg["environment"]["inputs"][0]["presence"] = json!("absent"),
            "different-location" => cfg["environment"]["inputs"][0]["location"] = json!(other),
            _ => cfg["environment"]["values"]["GIT_INDEX_FILE"] = json!("/tmp/unrelated-index"),
        });
        let (exit, _, error) = h.run(false);
        assert_ne!(exit, 0, "{fault}");
        assert!(error.contains("E_GIT_FACTS"), "{fault}: {error}");
        assert!(h.trace().is_empty(), "invalid link ran Git: {fault}");
    }
}

#[test]
fn bounded_git_failure_and_fixed_candidate_config_are_observed() {
    for fault in ["timeout", "object-timeout", "output", "dirty-config"] {
        let body = match fault {
            "timeout" => "case \"$*\" in --version) printf 'before-timeout'; /bin/sleep 10;; esac",
            // Object acquisition keeps its normal startup guard. Its deliberate
            // stall is longer, so this separately checks the post-version path.
            "object-timeout" => {
                "case \"$*\" in *'ls-tree'*) printf 'before-timeout'; /bin/sleep 60;; esac"
            }
            "output" => {
                "case \"$*\" in *'ls-tree'*) i=0; while [ $i -lt 400 ]; do printf '0123456789012345678901234567890123456789'; i=$((i+1)); done;; esac"
            }
            _ => "",
        };
        let mut h = BoundHost::new(false, body);
        h.host.edit(CONFIG, |c| {
            // Keep the short version bound separate from object setup. Set
            // both explicitly; the historical full config defaults to 120s.
            c["protocol"]["timeout_seconds"] = json!(if fault == "timeout" { 5 } else { 30 });
            // The v3 binding is part of the retained Git input. Leave enough
            // room for its config snapshot so object-timeout cases reach the
            // intended post-version process; the output fixture remains above
            // this bound.
            c["protocol"]["stdout_limit_bytes"] = json!(8192);
        });
        if fault == "dirty-config" {
            let mut original = fs::read(h.host.root().join(CONFIG)).unwrap();
            original.push(b'\n');
            fs::write(h.host.root().join(CONFIG), original).unwrap();
        }
        let (exit, _, error) = h.run(false);
        assert_ne!(exit, 0, "{fault}");
        assert!(error.contains("E_GIT_FACTS"), "{fault}: {error}");
        let expected = match fault {
            "timeout" | "object-timeout" => "process timed out",
            "output" => "process output limit exceeded",
            _ => "config differs from fixed candidate",
        };
        assert!(error.contains(expected), "{fault}: {error}");
        if matches!(fault, "timeout" | "object-timeout") {
            let observed: Value =
                serde_json::from_str(error.trim().strip_prefix("E_CHECK: E_GIT_FACTS: ").unwrap())
                    .unwrap();
            let processes = observed["observation"]["processes"].as_array().unwrap();
            let process = processes.last().unwrap();
            if fault == "timeout" {
                assert_eq!(processes.len(), 1, "timeout must stop before object reads");
                assert_eq!(process["argv"], json!([h.program, "--version"]));
            } else {
                assert!(
                    process["argv"]
                        .as_array()
                        .unwrap()
                        .contains(&json!("ls-tree"))
                );
                assert_eq!(process["stdout"], "before-timeout");
            }
            assert_eq!(process["failure"], "process timed out");
            assert_eq!(process["exit_code"], -1);
            for stream in ["stdout", "stderr"] {
                let bytes: Vec<u8> =
                    serde_json::from_value(process[format!("{stream}_bytes")].clone()).unwrap();
                assert_eq!(process[format!("{stream}_sha256")], json!(sha256(&bytes)));
                assert_eq!(process[stream], String::from_utf8(bytes.clone()).unwrap());
                if stream == "stdout" {
                    // A wall-clock bound may fire before the script starts or
                    // during its first write. Retain exactly the actual prefix.
                    assert!(b"before-timeout".starts_with(&bytes), "{error}");
                } else {
                    assert!(bytes.is_empty(), "{error}");
                }
            }
            if fault == "timeout" {
                assert!(matches!(h.trace().as_str(), "" | "--version\n"));
            }
        }
        if fault == "output" {
            assert!(error.contains("0123456789"), "{error}");
        }
    }
}

#[test]
fn input_closure_bindings_accept_explicit_external_nodes() {
    let mut h = BoundHost::new(false, "");
    h.host.edit(CONFIG, |config| {
        config["input_closure"]["bindings"] = json!([{
            "id": "registration-git-facts",
            "consumer": "judge:registration",
            "kind": "git-facts-explicit",
            "inputs": [
                "tool:facts-git",
                "input:facts-git-bytes",
                "environment:PATH",
                "environment:CHRONO_FACTS_TRACE",
                "environment:BOUND_FACTS_VALUE",
                "environment:GIT_CONFIG_NOSYSTEM",
                "environment:GIT_CONFIG_GLOBAL"
            ]
        }]);
    });
    h.host.base = h.host.candidate.clone();
    fs::write(
        h.host.root().join("document.txt"),
        "candidate after closure binding",
    )
    .unwrap();
    h.host.candidate = commit(h.host.root());
    let (exit, report, error) = h.run(false);
    assert_eq!(
        exit, 0,
        "explicit closure binding must pass: {error} {report}"
    );
    assert_eq!(report["status"], "pass");
}

#[test]
fn declared_complete_v3_closure_requires_all_external_nodes() {
    let mut h = BoundHost::new(false, "");
    h.host.edit(CONFIG, |config| {
        config["input_closure"] = json!({
            "status": "declared-complete",
            "unresolved": [],
            "bindings": [{
                "id": "registration-git-facts",
                "consumer": "judge:registration",
                "kind": "git-facts",
                "inputs": ["input:facts-git-bytes"]
            }]
        });
    });
    h.host.base = h.host.candidate.clone();
    fs::write(
        h.host.root().join("document.txt"),
        "candidate after incomplete declared closure",
    )
    .unwrap();
    h.host.candidate = commit(h.host.root());
    let (exit, report, error) = h.run(false);
    assert_ne!(
        exit, 0,
        "omitted external nodes must fail: {report} {error}"
    );
    assert!(finding(&report, "E_INPUT_UNDECLARED"), "{report} {error}");
    assert!(format!("{report} {error}").contains("tool:facts-git"));
}

#[test]
fn declared_complete_v3_closure_accepts_explicit_external_inventory() {
    let mut h = BoundHost::new(false, "");
    // BoundHost already installs this complete v3 inventory. Retain the
    // fixture's candidate transition below so the test exercises the reader
    // without creating a no-op commit.
    h.host.base = h.host.candidate.clone();
    fs::write(
        h.host.root().join("document.txt"),
        "candidate after complete declared closure",
    )
    .unwrap();
    h.host.candidate = commit(h.host.root());
    let (exit, report, error) = h.run(false);
    assert_eq!(
        exit, 0,
        "explicit external inventory must pass: {report} {error}"
    );
    assert_eq!(report["status"], "pass");
}

#[test]
fn declared_complete_v3_input_closure_requires_bindings_field() {
    let mut h = BoundHost::new(false, "");
    h.host.edit(CONFIG, |config| {
        config["input_closure"] = json!({
            "status": "declared-complete",
            "unresolved": []
        });
    });
    h.host.base = h.host.candidate.clone();
    fs::write(
        h.host.root().join("document.txt"),
        "candidate after missing bindings field",
    )
    .unwrap();
    h.host.candidate = commit(h.host.root());
    let (exit, report, error) = h.run(false);
    assert_ne!(
        exit, 0,
        "missing bindings must fail schema validation: {report} {error}"
    );
    assert!(finding(&report, "E_SCHEMA"), "{report} {error}");
}

#[test]
fn input_closure_bindings_reject_unknown_nodes_without_discovery() {
    for (field, value) in [
        ("consumer", json!("project:unknown")),
        ("inputs", json!(["input:unknown"])),
    ] {
        let mut h = BoundHost::new(false, "");
        h.host.edit(CONFIG, |config| {
            config["input_closure"]["bindings"] = json!([{
                "id": "invalid-closure",
                "consumer": "judge:registration",
                "kind": "git-facts",
                "inputs": ["tool:facts-git"]
            }]);
            config["input_closure"]["bindings"][0][field] = value.clone();
        });
        h.host.base = h.host.candidate.clone();
        fs::write(
            h.host.root().join("document.txt"),
            "candidate after invalid closure binding",
        )
        .unwrap();
        h.host.candidate = commit(h.host.root());
        let (exit, report, error) = h.run(false);
        assert_ne!(exit, 0, "{field}: {report} {error}");
        assert!(finding(&report, "E_REFERENCE"), "{field}: {report} {error}");
    }
}

#[test]
fn input_closure_bindings_require_explicit_project_edges() {
    let mut h = BoundHost::new(false, "");
    h.host.edit(CONFIG, |config| {
        config["input_closure"]["bindings"] = json!([{
            "id": "registration-git-facts",
            "consumer": "judge:registration",
            "kind": "git-facts",
            "inputs": [
                "tool:facts-git",
                "input:facts-git-bytes",
                "environment:PATH"
            ]
        }]);
    });
    h.host.edit(".chrono-harness/FILEMAP.json", |filemap| {
        filemap["project_edges"] = json!([
            {"from":"input:facts-git-bytes","kind":"judge-trigger","to":"judge:registration"},
            {"from":"environment:PATH","kind":"runtime-input","to":"judge:registration"}
        ]);
    });
    h.host.base = h.host.candidate.clone();
    fs::write(
        h.host.root().join("document.txt"),
        "candidate after missing closure edge",
    )
    .unwrap();
    h.host.candidate = commit(h.host.root());
    let (exit, report, error) = h.run(false);
    assert_ne!(exit, 0, "missing input edge must fail: {report} {error}");
    assert!(finding(&report, "E_REFERENCE"), "{report} {error}");
    assert!(format!("{report} {error}").contains("tool:facts-git"));
}
