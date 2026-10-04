use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const FACTS: &str = ".chrono-harness/scoped facts.json";

struct BoundHost {
    host: Host,
    program: PathBuf,
    trace: PathBuf,
    shadow: PathBuf,
    base: String,
    legacy_base: String,
}
fn source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn quote(path: &Path) -> String {
    format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"))
}
impl BoundHost {
    fn new(body: &str) -> Self {
        let host = Host::new();
        let legacy_base = host.head();
        let state = fs::canonicalize(host.root())
            .unwrap()
            .join(".chrono-harness/state");
        fs::create_dir_all(&state).unwrap();
        let program = state.join("chosen Git λ");
        let trace = state.join("git.trace");
        let real = chrono_harness::resolve_program(host.root(), "git", None).unwrap();
        let version = Command::new(&real).arg("--version").output().unwrap();
        assert!(version.status.success());
        fs::write(
            &program,
            format!(
                "#!/bin/sh\n[ \"$BOUND_SCOPED_VALUE\" = declared ] || exit 81\n[ -z \"$SCOPED_AMBIENT\" ] || exit 82\nprintf '%s\\n' \"$*\" >> \"$SCOPED_TRACE\" || exit $?\n{body}\nexec {} \"$@\"\n",
                quote(&real)
            ),
        )
        .unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        let shadow = state.join("ambient-bin");
        fs::create_dir(&shadow).unwrap();
        fs::write(
            shadow.join("git"),
            "#!/bin/sh\necho ambient-git >&2\nexit 83\n",
        )
        .unwrap();
        fs::set_permissions(shadow.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
        let path = std::env::var("PATH").unwrap();
        host.json(
            FACTS,
            &json!({"schema_version":3,
                "facts_git":{"tool":"scoped-git","input":"scoped-git-bytes"},
                "tools":[{"id":"scoped-git","program":program,"resolution":"PATH-once",
                    "version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}],
                "environment":{"inherit":[],"values":{"PATH":path,"BOUND_SCOPED_VALUE":"declared",
                    "SCOPED_TRACE":trace,"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null"},
                    "inputs":[{"id":"scoped-git-bytes","location":program,"presence":"present",
                        "sha256":sha256(&fs::read(&program).unwrap())}]},
                "protocol":{"timeout_seconds":30,"stdout_limit_bytes":1048576}}),
        );
        host.change_registry(CONFIG, |c| {
            c["schema"] = json!("chrono-ci-check/v2");
            c["judge"]["program"] =
                json!(source().join("crates/judge-ci/target/debug/chrono-judge-ci"));
            c["judge"]["timeout_seconds"] = json!(120);
            c["judge"]["output_limit_bytes"] = json!(4 * 1024 * 1024);
            c["policy"]["facts_config"] = json!(FACTS);
            c["policy"]["environment"] = json!({"PATH":path});
            c["policy"]["operation_timeout_seconds"] = json!(30);
        });
        host.change_registry(".chrono-harness/FILEMAP.json", |m| {
            m["files"].as_array_mut().unwrap().push(json!({
                "path":FACTS,"owner":"host","surface":"judge-policy","cost":"unmeasured","edges":[]}));
        });
        let base = host.commit();
        Self {
            host,
            program,
            trace,
            shadow,
            base,
            legacy_base,
        }
    }
    fn trace(&self) -> String {
        fs::read_to_string(&self.trace).unwrap_or_default()
    }
    fn cli(&self, candidate: &str) -> (i32, Value, String) {
        let output = Command::new(source().join("crates/runner/target/debug/chrono-harness"))
            .current_dir("/")
            .env_clear()
            .env("PATH", &self.shadow)
            .env("SCOPED_AMBIENT", "must-not-leak")
            .args([
                "check",
                "--config",
                self.host.root().join(CONFIG).to_str().unwrap(),
                "--base",
                &self.base,
                "--candidate",
                candidate,
            ])
            .output()
            .unwrap();
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap_or(Value::Null),
            String::from_utf8(output.stderr).unwrap(),
        )
    }
}

fn assert_process_bytes(observation: &Value) {
    let processes = observation["processes"]
        .as_array()
        .expect("bound process records");
    assert!(!processes.is_empty());
    for p in processes {
        for stream in ["stdout", "stderr"] {
            let bytes: Vec<u8> =
                serde_json::from_value(p[format!("{stream}_bytes")].clone()).unwrap();
            assert_eq!(p[format!("{stream}_sha256")], json!(sha256(&bytes)));
        }
    }
}

#[test]
fn scoped_v2_cli_binds_git_and_preserves_registered_execution() {
    let h = BoundHost::new("");
    h.host.write("src.txt", "candidate source");
    let candidate = h.host.commit();
    let (exit, report, error) = h.cli(&candidate);
    assert_eq!(exit, 0, "{error} {report}");
    let response = &report["response"];
    assert_eq!(response["status"], "passed");
    assert_eq!(response["evidence"]["scope"], "chrono-ci-check/v2");
    assert_eq!(response["evidence"]["selected"], json!(["test:suite"]));
    assert_eq!(h.host.calls(), 1);
    let facts = &response["evidence"]["git_facts"];
    assert_eq!(facts["binding"]["path"], json!(h.program));
    assert_process_bytes(facts);
    assert_eq!(
        facts["processes"].as_array().unwrap().len(),
        h.trace().lines().count()
    );
    assert!(h.trace().contains(&h.base) && h.trace().contains(&candidate));
    let trees = facts["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["argv"].as_array().unwrap().iter().any(|a| a == "ls-tree"))
        .collect::<Vec<_>>();
    assert_eq!(
        trees.len(),
        2,
        "one original tree acquisition per fixed endpoint"
    );
    for oid in [&h.base, &candidate] {
        assert_eq!(
            trees
                .iter()
                .filter(|p| p["argv"].as_array().unwrap().last() == Some(&json!(oid)))
                .count(),
            1
        );
    }
}

#[test]
fn scoped_v2_docs_delta_retains_both_snapshots_without_execution() {
    let h = BoundHost::new("");
    h.host.write("doc.txt", "candidate documentation");
    let candidate = h.host.commit();
    let response = h.host.check(&h.base, &candidate);
    pass(&response);
    assert_eq!(h.host.calls(), 0);
    assert_eq!(response.evidence["selected"], json!([]));
    assert_eq!(response.evidence["not_required"], json!(["test:suite"]));
    assert_process_bytes(&response.evidence["git_facts"]);
    assert!(h.trace().contains(&h.base) && h.trace().contains(&candidate));
}

#[test]
fn scoped_v2_filemap_plan_bounds_select_with_bound_git_facts() {
    for (key, value, reason) in [
        ("timeout_seconds", 31, "changed operation timeout"),
        ("output_limit_bytes", 8192, "changed operation output limit"),
    ] {
        let h = BoundHost::new("");
        explicit_plans(&h.host);
        let base = h.host.commit();
        h.host.change_registry(".chrono-harness/FILEMAP.json", |v| {
            v["execution_plans"]["test:suite"][key] = json!(value);
        });
        let candidate = h.host.commit();
        let r = h.host.check(&base, &candidate);
        assert_eq!(r.evidence["selected"], json!(["test:suite"]));
        pass(&r);
        assert_eq!(h.host.calls(), 1);
        assert_eq!(r.evidence["executed"].as_array().unwrap().len(), 1);
        assert_eq!(r.evidence["plan"]["operations"][0][key], json!(value));
        assert_eq!(
            r.evidence["selection_explanation"]["extra_selections"]["test:suite"],
            json!([reason])
        );
        assert_process_bytes(&r.evidence["git_facts"]);
        assert!(h.trace().contains(&base) && h.trace().contains(&candidate));
    }
}

fn select_platform(h: &BoundHost) {
    let selected = ".chrono-harness/native scoped.json";
    fs::copy(h.host.root().join(FACTS), h.host.root().join(selected)).unwrap();
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    h.host.json(
        FACTS,
        &json!({"schema":"chrono-git-configs/v1","platforms":{platform:selected}}),
    );
    h.host.change_registry(".chrono-harness/FILEMAP.json", |m| {
        m["files"]
            .as_array_mut()
            .unwrap()
            .push(json!({"path":selected,"owner":"host",
            "surface":"judge-policy","cost":"unmeasured","edges":[]}));
    });
}

#[test]
fn scoped_cli_platform_selection_keeps_execution_and_docs_locality() {
    for changed in ["src.txt", "doc.txt"] {
        let mut h = BoundHost::new("");
        select_platform(&h);
        h.base = h.host.commit();
        h.host.write(changed, "selected platform candidate");
        let candidate = h.host.commit();
        let (exit, report, error) = h.cli(&candidate);
        assert_eq!(exit, 0, "{error} {report}");
        let facts = &report["response"]["evidence"]["git_facts"];
        assert_eq!(facts["selection"]["path"], FACTS);
        assert_eq!(facts["config_path"], ".chrono-harness/native scoped.json");
        assert_process_bytes(facts);
        assert_eq!(h.host.calls(), usize::from(changed == "src.txt"));
        assert_eq!(facts["input_closure_complete"], false);
    }
}

#[test]
fn scoped_platform_selector_drift_rejects_before_business_execution() {
    let h = BoundHost::new("");
    select_platform(&h);
    h.host.write("src.txt", "candidate source");
    let candidate = h.host.commit();
    h.host.change_registry(FACTS, |selector| {
        selector["platforms"]["another-system"] = json!(".chrono-harness/other.json");
    });
    let (exit, report, _) = h.cli(&candidate);
    assert_ne!(exit, 0);
    assert!(
        report
            .to_string()
            .contains("selector differs from fixed candidate")
    );
    assert_eq!(h.host.calls(), 0);
}

#[test]
fn scoped_v2_failed_read_keeps_original_bytes_and_exit() {
    let h = BoundHost::new(
        "case \"$*\" in *'ls-tree'*) printf 'a\\000b'; printf 'chosen-failure' >&2; exit 17;; esac",
    );
    h.host.write("src.txt", "candidate source");
    let candidate = h.host.commit();
    let response = h.host.check(&h.base, &candidate);
    fail(&response, "Git facts process exit 17");
    assert_eq!(h.host.calls(), 0);
    let facts = &response.evidence["git_facts"];
    assert_process_bytes(facts);
    let last = facts["processes"].as_array().unwrap().last().unwrap();
    assert_eq!(last["exit_code"], 17);
    assert_eq!(last["stdout_bytes"], json!([97, 0, 98]));
    assert_eq!(last["stderr"], "chosen-failure");
}

#[test]
fn scoped_v2_rejects_bound_input_and_fixed_policy_drift() {
    for fault in ["digest", "version", "policy"] {
        let h = BoundHost::new("");
        if fault != "policy" {
            h.host.change_registry(FACTS, |f| {
                if fault == "digest" {
                    f["environment"]["inputs"][0]["sha256"] = json!("0".repeat(64));
                } else {
                    f["tools"][0]["expected_version"] = json!("not this version");
                }
            });
        }
        h.host.write("src.txt", "candidate source");
        let candidate = h.host.commit();
        if fault == "policy" {
            let path = h.host.root().join(FACTS);
            let mut bytes = fs::read(&path).unwrap();
            bytes.push(b'\n');
            fs::write(path, bytes).unwrap();
        }
        let response = h.host.check(&h.base, &candidate);
        let expected = match fault {
            "digest" => "digest",
            "version" => "version mismatch",
            _ => "config differs from fixed candidate",
        };
        fail(&response, expected);
        assert_eq!(h.host.calls(), 0);
        if fault == "digest" {
            assert!(h.trace().is_empty());
        } else {
            let facts = &response.evidence["git_facts"];
            assert_process_bytes(facts);
            if fault == "version" {
                let processes = facts["processes"].as_array().unwrap();
                assert_eq!(processes.len(), 1);
                assert_eq!(processes[0]["exit_code"], 0);
                assert!(
                    processes[0]["stdout"]
                        .as_str()
                        .unwrap()
                        .starts_with("git version ")
                );
            }
        }
    }
}

#[test]
fn scoped_v1_rejects_new_binding_field_including_null() {
    for value in [json!(FACTS), Value::Null] {
        let h = Host::new();
        let base = h.head();
        h.change_registry(CONFIG, |c| c["policy"]["facts_config"] = value);
        let candidate = h.commit();
        let response = h.check(&base, &candidate);
        fail(&response, "facts_config");
        assert_eq!(h.calls(), 0);
    }
}

#[test]
fn scoped_v2_requires_explicit_full_v3_facts_source() {
    for fault in [
        "missing",
        "null",
        "empty",
        "outside",
        "parent",
        "source-v2",
        "profile",
    ] {
        let h = BoundHost::new("");
        h.host.change_registry(CONFIG, |c| match fault {
            "missing" => {
                c["policy"].as_object_mut().unwrap().remove("facts_config");
            }
            "null" => c["policy"]["facts_config"] = Value::Null,
            "empty" => c["policy"]["facts_config"] = json!(""),
            "outside" => c["policy"]["facts_config"] = json!("facts.json"),
            "parent" => c["policy"]["facts_config"] = json!(".chrono-harness/../facts.json"),
            "profile" => c["schema"] = json!("chrono-ci-check/v99"),
            _ => (),
        });
        if fault == "source-v2" {
            h.host.change_registry(FACTS, |f| {
                f["schema_version"] = json!(2);
                f.as_object_mut().unwrap().remove("facts_git");
            });
        }
        let candidate = h.host.commit();
        let result = h.host.check(&h.base, &candidate);
        assert_eq!(result.status, Status::Failed, "accepted {fault}");
        assert_eq!(h.host.calls(), 0);
        assert!(h.trace().is_empty(), "Git ran for invalid {fault}");
    }
}

#[test]
fn scoped_v2_initial_reads_real_parents_even_when_shallow() {
    let h = BoundHost::new("");
    h.host.git(&["checkout", "--orphan", "new-root"]);
    let root = h.host.commit();
    let result = judge(&h.host.request(None, &root));
    pass(&result);
    assert_eq!(result.evidence["mode"], "initial-inventory");
    assert_eq!(result.evidence["previous_enforcement"], "none");
    assert_eq!(result.evidence["selected"], json!(["test:suite"]));
    assert_eq!(h.host.calls(), 1);
    assert_process_bytes(&result.evidence["git_facts"]);
    h.host.write("doc.txt", "child");
    let child = h.host.commit();
    for shallow in [false, true] {
        if shallow {
            h.host.write(".git/shallow", &format!("{child}\n"));
        }
        let result = judge(&h.host.request(None, &child));
        fail(&result, "initial requires parentless commit");
        assert_process_bytes(&result.evidence["git_facts"]);
        assert_eq!(h.host.calls(), 1);
    }
    assert!(h.trace().contains("cat-file"));
}

#[test]
fn scoped_v2_transition_and_old_only_deletion_keep_test_obligations() {
    let h = BoundHost::new("");
    h.host.write("src.txt", "migrated source");
    let candidate = h.host.commit();
    let result = h.host.check(&h.legacy_base, &candidate);
    pass(&result);
    assert_eq!(
        result.evidence["previous_enforcement"],
        "chrono-ci-check/v1"
    );
    assert_eq!(result.evidence["selected"], json!(["test:suite"]));
    assert_eq!(h.host.calls(), 1);
    fs::remove_file(h.host.root().join("src.txt")).unwrap();
    h.host.change_registry(".chrono-harness/FILEMAP.json", |m| {
        m["files"]
            .as_array_mut()
            .unwrap()
            .retain(|f| f["path"] != "src.txt");
        m["project_edges"] = json!([]);
    });
    let deleted = h.host.commit();
    let result = h.host.check(&candidate, &deleted);
    pass(&result);
    assert_eq!(
        result.evidence["previous_enforcement"],
        "chrono-ci-check/v2"
    );
    assert_eq!(result.evidence["selected"], json!(["test:suite"]));
    assert_eq!(h.host.calls(), 2);
    assert_process_bytes(&result.evidence["git_facts"]);
}

#[test]
fn scoped_v2_bound_reader_preserves_checkout_and_index_guards() {
    for fault in [
        "tracked",
        "staged",
        "untracked",
        "ignored",
        "assume",
        "skip",
        "checkout",
    ] {
        let h = BoundHost::new("");
        match fault {
            "tracked" => h.host.write("src.txt", "dirty"),
            "staged" => {
                h.host.write("src.txt", "dirty");
                h.host.git(&["add", "src.txt"]);
            }
            "untracked" => h.host.write("extra", "dirty"),
            "ignored" => {
                h.host.write(".git/info/exclude", "extra\n");
                h.host.write("extra", "dirty");
            }
            "checkout" => {
                h.host.write("doc.txt", "another commit");
                h.host.commit();
            }
            _ => {
                h.host.git(&[
                    "update-index",
                    if fault == "assume" {
                        "--assume-unchanged"
                    } else {
                        "--skip-worktree"
                    },
                    "src.txt",
                ]);
                h.host.write("src.txt", "hidden");
            }
        }
        let expected = match fault {
            "tracked" | "staged" => "dirty tracked",
            "untracked" | "ignored" => "untracked nonartifact",
            "checkout" => "HEAD does not equal candidate",
            _ => "unsupported index flags",
        };
        let index = fs::read(h.host.root().join(".git/index")).unwrap();
        let result = h.host.check(&h.base, &h.base);
        fail(&result, expected);
        assert_process_bytes(&result.evidence["git_facts"]);
        assert_eq!(h.host.calls(), 0);
        assert_eq!(fs::read(h.host.root().join(".git/index")).unwrap(), index);
    }
}

#[test]
fn scoped_v2_post_execution_rechecks_with_bound_reader() {
    let h = BoundHost::new("");
    h.host.write("check.sh", "printf changed > src.txt\n");
    let candidate = h.host.commit();
    let result = h.host.check(&h.base, &candidate);
    fail(&result, "dirty tracked");
    assert_eq!(result.evidence["executed"][0]["process"]["exit_code"], 0);
    assert!(
        result
            .results
            .iter()
            .any(|r| r.id == "ci.post-snapshot" && r.status == Status::Failed)
    );
    assert_process_bytes(&result.evidence["git_facts"]);
    assert_eq!(
        result.evidence["git_facts"]["processes"]
            .as_array()
            .unwrap()
            .len(),
        h.trace().lines().count()
    );
}

#[test]
fn scoped_v2_opening_and_bounded_failures_retain_structured_observations() {
    for fault in ["version-exit", "version-timeout", "read-limit"] {
        let body = match fault {
            "version-exit" => {
                "if [ \"$1\" = --version ]; then printf 'a\\000b'; printf version-failed >&2; exit 19; fi"
            }
            "version-timeout" => "if [ \"$1\" = --version ]; then exec /bin/sleep 60; fi",
            _ => "case \"$*\" in *'ls-tree'*) while :; do printf 0123456789abcdef; done;; esac",
        };
        let h = BoundHost::new(body);
        h.host.change_registry(FACTS, |f| {
            if fault == "version-timeout" {
                f["protocol"]["timeout_seconds"] = json!(1);
            }
            if fault == "read-limit" {
                f["protocol"]["stdout_limit_bytes"] = json!(4096);
            }
        });
        let candidate = h.host.commit();
        let result = h.host.check(&h.base, &candidate);
        let expected = match fault {
            "version-exit" => "Git facts process exit 19",
            "version-timeout" => "timed out",
            _ => "output limit",
        };
        fail(&result, expected);
        assert_eq!(h.host.calls(), 0);
        let facts = &result.evidence["git_facts"];
        assert_process_bytes(facts);
        let processes = facts["processes"].as_array().unwrap();
        let last = processes.last().unwrap();
        if fault == "version-exit" {
            assert_eq!(processes.len(), 1);
            assert_eq!(last["exit_code"], 19);
            assert_eq!(last["stdout_bytes"], json!([97, 0, 98]));
            assert_eq!(last["stderr"], "version-failed");
        } else {
            assert!(last["failure"].as_str().unwrap().contains(expected));
            if fault == "version-timeout" {
                assert_eq!(processes.len(), 1);
            } else {
                assert_eq!(last["stdout_bytes"].as_array().unwrap().len(), 4096);
                assert!(h.trace().contains("ls-tree"));
            }
        }
    }
}

#[test]
fn scoped_artifact_inventory_does_not_spend_git_output_bound_on_declared_outputs() {
    let h = BoundHost::new("");
    h.host.change_registry(FACTS, |f| {
        f["protocol"]["stdout_limit_bytes"] = json!(8192);
        f["environment"]["values"]["GIT_LITERAL_PATHSPECS"] = json!("1");
        f["environment"]["values"]["GIT_ICASE_PATHSPECS"] = json!("1");
    });
    h.host.change_registry(CONFIG, |c| {
        c["policy"]["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(json!("cache[1]/allowed/"));
    });
    h.host.write("src.txt", "changed");
    let candidate = h.host.commit();
    for i in 0..256 {
        h.host.write(
            &format!("cache[1]/allowed/{i:04}-{}", "x".repeat(80)),
            "output",
        );
    }
    let result = h.host.check(&h.base, &candidate);
    pass(&result);
    assert_eq!(h.host.calls(), 1);
    let facts = &result.evidence["git_facts"];
    assert_process_bytes(facts);
    let inventories: Vec<_> = facts["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| {
            p["argv"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a == "--others")
        })
        .collect();
    assert!(!inventories.is_empty());
    for inventory in inventories {
        assert!(inventory["stdout_bytes"].as_array().unwrap().is_empty());
        for key in [
            "GIT_LITERAL_PATHSPECS",
            "GIT_GLOB_PATHSPECS",
            "GIT_NOGLOB_PATHSPECS",
            "GIT_ICASE_PATHSPECS",
        ] {
            assert_eq!(inventory["environment"][key], "0");
        }
    }
}

#[test]
fn scoped_artifact_exclusion_keeps_literal_neighbors_ignored_files_and_root_types_visible() {
    use std::os::unix::fs::symlink;
    let h = BoundHost::new("");
    h.host.change_registry(FACTS, |f| {
        f["environment"]["values"]["GIT_LITERAL_PATHSPECS"] = json!("1");
        f["environment"]["values"]["GIT_ICASE_PATHSPECS"] = json!("1");
    });
    h.host.change_registry(CONFIG, |c| {
        c["policy"]["artifacts"].as_array_mut().unwrap().extend([
            json!("cache[1]/allowed/"),
            json!("CacheCase/allowed/"),
            json!("declared-directory/"),
        ]);
    });
    h.host.write(".git/info/exclude", "cache*/\nCacheCase/\n");
    let candidate = h.host.commit();
    h.host.write("cache[1]/allowed/output", "allowed");
    for path in [
        "cache[1]/outside",
        "cache1/allowed/foreign",
        "cachecase/allowed/foreign",
        "unknown",
    ] {
        h.host.write(path, "not registered");
        let result = h.host.check(&h.base, &candidate);
        fail(&result, "untracked nonartifact");
        assert_eq!(h.host.calls(), 0);
        fs::remove_file(h.host.root().join(path)).unwrap();
    }
    for link in [false, true] {
        let path = h.host.root().join("declared-directory");
        if link {
            symlink("cache[1]/allowed", &path).unwrap();
        } else {
            fs::write(&path, "not a directory").unwrap();
        }
        fail(&h.host.check(&h.base, &candidate), "untracked nonartifact");
        fs::remove_file(path).unwrap();
    }
    pass(&h.host.check(&h.base, &candidate));
}
