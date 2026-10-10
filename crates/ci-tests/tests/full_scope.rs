//! Provider transport tests; the recording consumer does not judge full governance.
use super::*;
use chrono_harness::units::Scope;

fn scopes() -> [Value; 2] {
    [
        json!({"kind":"unit","unit":"compiler.arm64-1"}),
        json!({"kind":"collect","manifest":".chrono-harness/state/evidence/manifest.json"}),
    ]
}

fn scoped_config(scope: &Value) -> FullConfig {
    let mut value = serde_json::to_value(config()).unwrap();
    value["scope"] = scope.clone();
    serde_json::from_value(value).expect("strict typed provider scope")
}

fn local_argv(c: &FullConfig, scope: &Value, base: &str, candidate: &str) -> Vec<String> {
    let mut argv =
        chrono_harness::canonical_argv(&c.runner, &c.check_config, Some(base), candidate, false);
    argv.extend(["--context".into(), c.context_path.clone()]);
    argv.extend(
        serde_json::from_value::<Scope>(scope.clone())
            .unwrap()
            .argv(),
    );
    argv
}

#[test]
fn unscoped_source_identity_and_projection_are_stable() {
    let c = config();
    let source = serde_json::to_value(&c).unwrap();
    assert!(source.get("scope").is_none());
    assert!(source.get("include_hidden_files").is_none());
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<FullConfig>(source.clone()).unwrap())
            .unwrap(),
        source
    );
    assert_eq!(
        full::argv(&c, "base", "candidate"),
        vec![
            c.runner.clone(),
            "check".into(),
            "--config".into(),
            c.check_config.clone(),
            "--base".into(),
            "base".into(),
            "--candidate".into(),
            "candidate".into(),
            "--context".into(),
            c.context_path.clone(),
        ]
    );
    // The pre-follow-up v1 renderer's bytes are fixed, including the original ownership marker.
    assert_eq!(
        sha256(full::render(&c, SOURCE).unwrap().as_bytes()),
        "e8755f0e752cee5f45750b96398575d616df4472bb67fb48b190fadbe8904cd1"
    );
}

#[test]
fn scoped_render_and_recording_shell_use_exact_local_suffix_and_exit() {
    for scope in scopes().into_iter().chain([json!({"kind":"collect","manifest":".chrono-harness/state/'$CHRONO_BASE`literal`/manifest.json"})]) {
        let d = crate::tools::temporary_host("host λ ");
        let root = d.path();
        let mut c = scoped_config(&scope);
        c.runner = "$CHRONO_BASE".into();
        executable_alias(env!("CARGO_BIN_EXE_chrono-ci-test-transport"), root.join(&c.runner));
        let yaml = full::render(&c, SOURCE).unwrap();
        let out = Command::new("/bin/bash").current_dir(root).env("PATH", root)
            .env("CHRONO_TEST_RECORDING", "scope")
            .env("CHRONO_BASE", "base fixed").env("CHRONO_CANDIDATE", "candidate fixed")
            .args(["-e", "-c", &script(&yaml, "Canonical full harness check")]).output().unwrap();
        assert_eq!(out.status.code(), Some(23), "scope {scope}: {out:?}");
        assert_eq!(out.stderr, b"\xffscope-consumer-failure");
        let expected = local_argv(&c, &scope, "base fixed", "candidate fixed");
        assert_eq!(full::argv(&c, "base fixed", "candidate fixed"), expected);
        assert_eq!(String::from_utf8(out.stdout).unwrap().lines().collect::<Vec<_>>(), expected[1..]);
        assert!(yaml.contains("if: ${{ always() }}"));
        assert!(yaml.contains(&c.checkout_action) && yaml.contains(&c.upload_artifact_action));
        assert!(!root.join(".chrono-harness/state").exists());
    }
}

#[test]
fn malformed_scopes_and_ownership_collisions_fail_before_generation() {
    let c = config();
    let mut invalid = vec![
        Value::Null,
        json!("unit"),
        json!({}),
        json!({"kind":"all"}),
        json!({"kind":"unit"}),
        json!({"kind":"unit","unit":null}),
        json!({"kind":"unit","unit":7}),
        json!({"kind":"unit","unit":"ok","manifest":"extra"}),
        json!({"kind":"collect"}),
        json!({"kind":"collect","manifest":null}),
        json!({"kind":"collect","manifest":".chrono-harness/state/m.json","unknown":true}),
    ];
    for unit in [
        "",
        "bad/id",
        "two words",
        "--unit x",
        "λ",
        "${{ inputs.unit }}",
        "x\n",
    ] {
        invalid.push(json!({"kind":"unit","unit":unit}));
    }
    for path in [
        "/absolute.json",
        "../outside.json",
        ".chrono-harness/policy.json",
        ".chrono-harness/state/../policy.json",
        ".chrono-harness/state/${{ inputs.manifest }}",
        ".chrono-harness/state/m\n.json",
        &c.context_path,
        &c.preparation_path,
        ".chrono-harness/state/full/prepare.stdout.json",
        ".chrono-harness/state/full/prepare.stderr",
        ".chrono-harness/state/full",
        ".chrono-harness/state/full/context.json/child",
        ".chrono-harness/state/full/./context.json",
        ".chrono-harness/state/full//preparation.json",
        ".chrono-harness/state/full/prepare.stderr/",
    ] {
        invalid.push(json!({"kind":"collect","manifest":path}));
    }
    let d = crate::tools::temporary_host("host λ ");
    let root = d.path();
    for scope in invalid {
        let mut value = serde_json::to_value(&c).unwrap();
        value["scope"] = scope.clone();
        write(root, SOURCE, &value);
        assert!(generate(root, SOURCE, false).is_err(), "accepted {scope}");
        assert!(!root.join(&c.workflow_path).exists());
        assert!(!root.join(&c.artifact_directory).exists());
    }
    let valid = scoped_config(&scopes()[0]);
    write(root, SOURCE, &serde_json::to_value(&valid).unwrap());
    let mut unknown = serde_json::to_value(&valid).unwrap();
    unknown["selection"] = json!("all");
    write(root, SOURCE, &unknown);
    assert!(generate(root, SOURCE, false).is_err());
    assert!(!root.join(&c.workflow_path).exists());
    let raw = serde_json::to_string(&valid).unwrap();
    let duplicate = raw.replacen(
        "\"kind\":\"unit\"",
        "\"kind\":\"unit\",\"kind\":\"collect\"",
        1,
    );
    fs::write(root.join(SOURCE), duplicate).unwrap();
    assert!(
        generate(root, SOURCE, false)
            .unwrap_err()
            .contains("duplicate")
    );
    assert!(!root.join(&c.workflow_path).exists());
    let collector = scoped_config(&scopes()[1]);
    // A caller's explicit evidence location may coincide with a configured executable.
    let mut collided = collector.clone();
    collided.runner = ".chrono-harness/state/evidence/manifest.json".into();
    assert!(full::render(&collided, SOURCE).is_err());
    collided = collector;
    collided.generator = ".chrono-harness/state/evidence/manifest.json/child".into();
    assert!(full::render(&collided, SOURCE).is_err());
}

#[test]
fn independent_sources_regenerate_only_their_owned_projection_and_keep_customization() {
    let d = crate::tools::temporary_host("host λ ");
    let root = d.path();
    let unit_source = ".chrono-harness/providers/build-one.json";
    let collect_source = ".chrono-harness/providers/review.json";
    let mut unit = scoped_config(&scopes()[0]);
    unit.workflow_path = ".github/workflows/build-one.yml".into();
    unit.name = "Host build custom λ".into();
    unit.runs_on = "macos-14".into();
    unit.bootstrap = vec![
        "/bin/sh".into(),
        ".chrono-harness/host-bootstrap.sh".into(),
        "custom argument".into(),
    ];
    unit.artifact_directory = ".chrono-harness/state/build-one/".into();
    unit.context_path = format!("{}context.json", unit.artifact_directory);
    unit.preparation_path = format!("{}preparation.json", unit.artifact_directory);
    let mut collect = scoped_config(&scopes()[1]);
    collect.workflow_path = ".github/workflows/review.yml".into();
    collect.name = "Host review custom".into();
    collect.artifact_directory = ".chrono-harness/state/review/".into();
    collect.context_path = format!("{}caller.json", collect.artifact_directory);
    collect.preparation_path = format!("{}transport.json", collect.artifact_directory);
    for (path, c) in [(unit_source, &unit), (collect_source, &collect)] {
        write(root, path, &serde_json::to_value(c).unwrap());
        let out = cli(root, &["generate", "--host-root", ".", "--config", path]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            cli(root, &["verify", "--host-root", ".", "--config", path])
                .status
                .success()
        );
    }
    let collect_bytes = fs::read(root.join(&collect.workflow_path)).unwrap();
    let collect_source_bytes = fs::read(root.join(collect_source)).unwrap();
    let unit_bytes = fs::read(root.join(&unit.workflow_path)).unwrap();
    let unit_source_bytes = fs::read(root.join(unit_source)).unwrap();
    unit.timeout_minutes = 42;
    write(root, unit_source, &serde_json::to_value(&unit).unwrap());
    let custom_source = fs::read(root.join(unit_source)).unwrap();
    assert!(
        generate(root, unit_source, true)
            .unwrap_err()
            .contains("drift")
    );
    assert_eq!(
        fs::read(root.join(&unit.workflow_path)).unwrap(),
        unit_bytes
    );
    assert!(generate(root, unit_source, false).unwrap());
    assert_eq!(fs::read(root.join(unit_source)).unwrap(), custom_source);
    assert_ne!(custom_source, unit_source_bytes);
    assert_eq!(
        fs::read(root.join(&collect.workflow_path)).unwrap(),
        collect_bytes
    );
    assert_eq!(
        fs::read(root.join(collect_source)).unwrap(),
        collect_source_bytes
    );
    assert_eq!(
        fs::read_to_string(root.join(&unit.workflow_path)).unwrap(),
        full::render(&unit, unit_source).unwrap()
    );
    assert!(!generate(root, collect_source, true).unwrap());
    assert!(!root.join(".chrono-harness/ci/full.json").exists());
    assert!(!root.join(".chrono-harness/state").exists());
    let expected = fs::read(root.join(&unit.workflow_path)).unwrap();
    fs::write(
        root.join(&unit.workflow_path),
        [expected.as_slice(), b"# drift\n"].concat(),
    )
    .unwrap();
    assert!(
        generate(root, unit_source, true)
            .unwrap_err()
            .contains("drift")
    );
    assert!(generate(root, unit_source, false).unwrap());
    assert_eq!(fs::read(root.join(&unit.workflow_path)).unwrap(), expected);
    assert_eq!(
        fs::read(root.join(&collect.workflow_path)).unwrap(),
        collect_bytes
    );
    let mut collision = collect.clone();
    collision.workflow_path = unit.workflow_path.clone();
    write(
        root,
        collect_source,
        &serde_json::to_value(&collision).unwrap(),
    );
    let saved = fs::read(root.join(&unit.workflow_path)).unwrap();
    assert!(
        generate(root, collect_source, false)
            .unwrap_err()
            .contains("source")
    );
    assert_eq!(fs::read(root.join(&unit.workflow_path)).unwrap(), saved);
}

fn scoped_host(scope: &Value, selector: bool, mode: &str) -> (Host, FullConfig, &'static str) {
    let mut h = Host::new(mode);
    let c = scoped_config(scope);
    let path = if scope["kind"] == "unit" {
        ".chrono-harness/providers/unit.json"
    } else {
        ".chrono-harness/providers/collector.json"
    };
    write(&h.root, path, &serde_json::to_value(&c).unwrap());
    // Preserve the host's raw source identity as well as its parsed settings.
    let source = fs::read(h.root.join(path)).unwrap();
    fs::write(
        h.root.join(path),
        [b" \n".as_slice(), &source, b"\n\t "].concat(),
    )
    .unwrap();
    if selector {
        let chosen = ".chrono-harness/native full.json";
        write(&h.root, chosen, &read(&h.root, FACTS));
        let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
        write(
            &h.root,
            FACTS,
            &json!({"schema":"chrono-git-configs/v1","platforms":{platform:chosen}}),
        );
    }
    fs::write(h.root.join(".gitignore"), ".chrono-harness/state/\n").unwrap();
    generate(&h.root, path, false).unwrap();
    h.revise();
    (h, c, path)
}

fn scoped_prepare_cli(h: &Host, path: &str, bytes: &[u8], extra: &[&str]) -> std::process::Output {
    let payload = h.root.join(".chrono-harness/state/payload.json");
    write(
        &h.root,
        ".chrono-harness/state/payload.json",
        &json!({"inputs":{"context":String::from_utf8(bytes.to_vec()).unwrap()}}),
    );
    let output = h.root.join(".chrono-harness/state/github-output");
    fs::write(&output, "").unwrap();
    Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../ci/target/debug/chrono-ci"))
        .current_dir(h.root.parent().unwrap())
        .env("PATH", &h.shadow)
        .env("CHRONO_EVENT_AMBIENT", "ambient")
        .args([
            "prepare",
            "--host-root",
            h.root.file_name().unwrap().to_str().unwrap(),
            "--config",
            path,
            "--event",
            "workflow_dispatch",
            "--payload",
            payload.to_str().unwrap(),
            "--workflow-revision",
            &h.candidate,
            "--github-output",
            output.to_str().unwrap(),
        ])
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn committed_unit_and_collector_cli_prepare_preserve_caller_bytes_and_canonical_argv() {
    for scope in scopes() {
        for selector in [false, true] {
            let (h, c, path) = scoped_host(&scope, selector, "pass");
            let source = fs::read(h.root.join(path)).unwrap();
            let bytes = context(&h, &h.candidate);
            let out = scoped_prepare_cli(&h, path, &bytes, &[]);
            assert!(
                out.status.success(),
                "{scope}; selector={selector}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(out.stderr.is_empty());
            let report: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(fs::read(h.root.join(&c.context_path)).unwrap(), bytes);
            assert_eq!(report, read(&h.root, &c.preparation_path));
            assert_eq!(report["context"]["input_bytes"], json!(bytes));
            assert_eq!(report["context"]["sha256"], sha256(&bytes));
            assert_eq!(report["config_path"], path);
            assert_eq!(report["config_sha256"], sha256(&source));
            assert_eq!(fs::read(h.root.join(path)).unwrap(), source);
            assert_eq!(
                report["canonical_argv"],
                json!(local_argv(&c, &scope, &h.candidate, &h.candidate))
            );
            assert_eq!(report["status"], "prepared");
            assert_eq!(report["governance"], "not-evaluated");
            assert_eq!(report["parity"], "unestablished");
            assert_eq!(
                fs::read_to_string(h.root.join(".chrono-harness/state/github-output")).unwrap(),
                format!("base={}\ncandidate={}\n", h.candidate, h.candidate)
            );
            assert_eq!(report["git_facts"]["binding"]["path"], json!(h.program));
            if selector {
                assert_eq!(report["git_facts"]["selection"]["path"], FACTS);
                assert_eq!(
                    report["git_facts"]["config_path"],
                    ".chrono-harness/native full.json"
                );
            } else {
                assert!(report["git_facts"]["selection"].is_null());
                assert_eq!(report["git_facts"]["config_path"], FACTS);
            }
            assert!(!h.trace().contains(" fetch "));
            if let Some(manifest) = scope["manifest"].as_str() {
                assert!(
                    !h.root.join(manifest).exists(),
                    "provider must not fabricate collection evidence"
                );
                let retained = b"caller-owned manifest bytes; admission belongs to full core";
                fs::create_dir_all(h.root.join(manifest).parent().unwrap()).unwrap();
                fs::write(h.root.join(manifest), retained).unwrap();
                assert_eq!(
                    full::prepare(&h.root, path, &c, &bytes, &h.candidate).unwrap(),
                    report
                );
                assert_eq!(fs::read(h.root.join(manifest)).unwrap(), retained);
            }
            let saved = fs::read(h.root.join(&c.preparation_path)).unwrap();
            let override_scope = scoped_prepare_cli(&h, path, &bytes, &["--unit", "different"]);
            assert_eq!(override_scope.status.code(), Some(1));
            assert!(
                String::from_utf8_lossy(&override_scope.stderr)
                    .contains("unit selection requires unit provider")
            );
            assert_eq!(fs::read(h.root.join(&c.preparation_path)).unwrap(), saved);
        }
    }
}

#[test]
fn scoped_prepare_keeps_fixed_source_checkout_workflow_and_selector_controls() {
    for scope in scopes() {
        for case in [
            "source",
            "selected-config",
            "checkout",
            "workflow",
            "selector",
            "nested-selector",
        ] {
            let selector = matches!(case, "selected-config" | "selector" | "nested-selector");
            let (mut h, c, path) = scoped_host(&scope, selector, "pass");
            match case {
                "source" => {
                    let mut drift = read(&h.root, path);
                    drift["name"] = json!("source drift");
                    write(&h.root, path, &drift);
                }
                "selected-config" => {
                    let chosen = ".chrono-harness/native full.json";
                    let mut drift = read(&h.root, chosen);
                    drift["protocol"]["timeout_seconds"] = json!(11);
                    write(&h.root, chosen, &drift);
                }
                "checkout" => {
                    fs::write(h.root.join("later"), "later").unwrap();
                    commit(&h.root, false);
                }
                "workflow" => {
                    fs::write(h.root.join(&c.workflow_path), "different projection").unwrap();
                    h.revise();
                }
                "selector" => {
                    let mut drift = read(&h.root, FACTS);
                    drift["platforms"]["another-platform"] = json!(".chrono-harness/other.json");
                    write(&h.root, FACTS, &drift);
                }
                _ => {
                    write(
                        &h.root,
                        ".chrono-harness/native full.json",
                        &read(&h.root, FACTS),
                    );
                }
            }
            let selected: FullConfig = serde_json::from_value(read(&h.root, path)).unwrap();
            let error = full::prepare(
                &h.root,
                path,
                &selected,
                &context(&h, &h.candidate),
                &h.candidate,
            )
            .unwrap_err();
            let expected = match case {
                "source" => "source differs from fixed candidate",
                "selected-config" => "config differs from fixed candidate",
                "checkout" => "checkout is not supplied candidate",
                "workflow" => "workflow revision differs",
                "selector" => "selector differs from fixed candidate",
                _ => "never another selector",
            };
            assert!(error.contains(expected), "{case}: {error}");
            assert!(!h.root.join(&c.context_path).exists());
            assert!(!h.root.join(&c.preparation_path).exists());
            assert!(!h.trace().contains(" fetch "));
        }
    }
}

#[test]
fn scoped_git_failure_keeps_original_bytes_status_and_no_outputs() {
    for scope in scopes() {
        let (h, c, path) = scoped_host(&scope, false, "scope-probe-failure");
        let (_remote, base) = h.remote();
        let out = scoped_prepare_cli(&h, path, &context(&h, &base), &[]);
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
        let stderr = String::from_utf8(out.stderr).unwrap();
        let evidence = failure(stderr.strip_prefix("E_CI: ").unwrap().trim());
        let failed = evidence["git_facts"]["processes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["exit_code"] == 29)
            .unwrap();
        let failed = chrono_harness::full::expand_process(failed).unwrap();
        let bytes: Vec<u8> = serde_json::from_value(failed["stderr_bytes"].clone()).unwrap();
        assert_eq!(bytes, b"\xffscope-probe-failure");
        assert_eq!(failed["stderr_sha256"], sha256(&bytes));
        assert!(!h.trace().contains(" fetch "));
        assert!(!h.root.join(&c.context_path).exists());
        assert!(!h.root.join(&c.preparation_path).exists());
    }
}

#[test]
fn scoped_init_preserves_host_customization_and_can_adopt_scope_on_legacy_projection() {
    let d = crate::tools::temporary_host("host λ ");
    let root = d.path();
    let input = root.join("incoming.json");
    write(
        root,
        "incoming.json",
        &serde_json::to_value(config()).unwrap(),
    );
    assert!(init(root, &input).unwrap());
    let adopted = ".chrono-harness/ci/full.json";
    let mut custom = serde_json::to_value(scoped_config(&scopes()[0])).unwrap();
    custom["name"] = json!("adopted host customization");
    custom["bootstrap"] = json!(["/bin/sh", ".chrono-harness/custom.sh", "λ"]);
    write(root, adopted, &custom);
    let source_bytes = fs::read(root.join(adopted)).unwrap();
    assert!(init(root, &input).unwrap());
    assert_eq!(fs::read(root.join(adopted)).unwrap(), source_bytes);
    assert!(!generate(root, adopted, true).unwrap());
    assert_eq!(
        fs::read_to_string(root.join(config().workflow_path)).unwrap(),
        full::render(&serde_json::from_value(custom).unwrap(), adopted).unwrap()
    );
}

#[test]
fn scoped_generation_cannot_take_a_legacy_projection_from_another_source() {
    let d = crate::tools::temporary_host("host λ ");
    let root = d.path();
    let legacy_source = ".chrono-harness/providers/legacy.json";
    let scoped_source = ".chrono-harness/providers/scoped.json";
    let mut legacy = config();
    // A legitimate bootstrap invocation is not the projection's source binding.
    legacy.bootstrap = vec![
        legacy.generator.clone(),
        "prepare".into(),
        "--host-root".into(),
        ".".into(),
        "--config".into(),
        scoped_source.into(),
        "--event".into(),
        "workflow_dispatch".into(),
    ];
    write(root, legacy_source, &serde_json::to_value(&legacy).unwrap());
    generate(root, legacy_source, false).unwrap();
    let original = fs::read(root.join(&legacy.workflow_path)).unwrap();
    write(
        root,
        scoped_source,
        &serde_json::to_value(scoped_config(&scopes()[0])).unwrap(),
    );
    assert!(
        generate(root, scoped_source, false)
            .unwrap_err()
            .contains("source ownership collision")
    );
    assert_eq!(
        fs::read(root.join(&legacy.workflow_path)).unwrap(),
        original
    );
}

#[test]
fn collection_symlink_is_rejected_without_provisioning_or_output() {
    let d = crate::tools::temporary_host("host λ ");
    let root = d.path();
    let c = scoped_config(&scopes()[1]);
    write(root, SOURCE, &serde_json::to_value(&c).unwrap());
    let manifest = root.join(scopes()[1]["manifest"].as_str().unwrap());
    fs::create_dir_all(manifest.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(root.join(SOURCE), &manifest).unwrap();
    assert!(
        generate(root, SOURCE, false)
            .unwrap_err()
            .contains("symlink")
    );
    assert!(!root.join(&c.workflow_path).exists());
    // Preflight the explicitly selected path before Git or any context/report publication.
    assert!(
        full::prepare(root, SOURCE, &c, b"{}", &"1".repeat(40))
            .unwrap_err()
            .contains("symlink")
    );
    assert!(!root.join(&c.context_path).exists());
    assert!(!root.join(&c.preparation_path).exists());
}

#[test]
fn short_full_scope_survives_argv_and_rendering_early_returns() {
    for scope in scopes() {
        let mut c = scoped_config(&scope);
        c.schema = full::SHORT_SCHEMA.into();
        let mut expected = vec![c.runner.clone(), "check".into()];
        expected.extend(serde_json::from_value::<Scope>(scope).unwrap().short_argv());
        assert_eq!(full::argv(&c, "base", "candidate"), expected);
        let rendered = full::render(&c, SOURCE).unwrap();
        let command = expected
            .iter()
            .map(|a| format!("'{}'", a.replace('\'', "'\\''")))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(rendered.contains(&command));
        assert!(!rendered.contains("Preserve fixed full context"));
    }
}
