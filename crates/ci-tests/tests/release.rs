use chrono_ci::{generate, init, release};
#[path = "support/tools.rs"]
mod tools;
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn config() -> Value {
    json!({
        "schema":"chrono-github-release/v1",
        "workflow_path":".github/workflows/package.yml",
        "name":"Portable packages",
        "push_branches":["integration/packages"],
        "checkout_action":"actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
        "upload_artifact_action":"actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
        "jobs":[{
            "id":"native", "runs_on":"ubuntu-24.04", "timeout_minutes":20,
            "command":["/bin/sh",".chrono-harness/pack.sh","out/"],
            "artifact_name":"native-output", "artifact_directory":"out/"
        }]
    })
}
fn write(root: &Path, path: &str, value: &Value) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn cli(root: &Path, args: &[&str]) -> std::process::Output {
    let host = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    Command::new(host.join("crates/ci/target/debug/chrono-ci"))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap()
}
fn script(rendered: &str) -> String {
    rendered
        .split("        run: |\n")
        .nth(1)
        .unwrap()
        .lines()
        .take_while(|line| line.starts_with("          "))
        .map(|line| &line[10..])
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn generated_command_preserves_literal_argv_and_original_failure() {
    let d = crate::tools::temporary_host("release host ");
    let root = d.path();
    let program = root.join("literal command.py");
    fs::write(&program, "import json,sys\nfrom pathlib import Path\nPath('args.json').write_text(json.dumps(sys.argv[1:]))\nprint('original failure',file=sys.stderr)\nsys.exit(17)\n").unwrap();
    let args = [
        "",
        "space value",
        "'\"",
        "$HOME",
        "`touch bad1`",
        "$(touch bad2)",
    ];
    let mut c = config();
    c["jobs"][0]["command"] = json!(["/usr/bin/python3", program.to_str().unwrap()]);
    c["jobs"][0]["command"]
        .as_array_mut()
        .unwrap()
        .extend(args.iter().map(|v| json!(v)));
    write(root, "source.json", &c);
    let out = cli(
        root,
        &[
            "init",
            "--host-root",
            root.to_str().unwrap(),
            "--config",
            "source.json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let workflow = fs::read_to_string(root.join(".github/workflows/package.yml")).unwrap();
    let run = Command::new("/bin/bash")
        .args(["-e", "-c", &script(&workflow)])
        .current_dir(root)
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(17));
    assert_eq!(run.stderr, b"original failure\n");
    let observed: Vec<String> =
        serde_json::from_slice(&fs::read(root.join("args.json")).unwrap()).unwrap();
    assert_eq!(observed, args);
    assert!(!root.join("bad1").exists() && !root.join("bad2").exists());
    assert!(workflow.contains("if: ${{ always() }}\n        uses: actions/upload-artifact@"));
    assert!(workflow.contains("permissions:\n  contents: read\n"));
}

#[test]
fn release_adoption_preserves_customization_and_verify_does_not_repair() {
    let d = crate::tools::temporary_host("host λ ");
    let root = d.path();
    let source = root.join("source.json");
    write(root, "source.json", &config());
    assert!(init(root, &source).unwrap());
    let output = root.join(".github/workflows/package.yml");
    let before = fs::read(&output).unwrap();
    let mtime = fs::metadata(&output).unwrap().modified().unwrap();
    assert!(!init(root, &source).unwrap());
    assert_eq!(fs::metadata(&output).unwrap().modified().unwrap(), mtime);
    let mut c = config();
    c["name"] = json!("Host customization");
    c["include_hidden_files"] = json!(true);
    c["push_branches"] = json!([]);
    let mut second = c["jobs"][0].clone();
    second["id"] = json!("another");
    second["runs_on"] = json!("macos-14");
    second["artifact_name"] = json!("another-artifact");
    c["jobs"].as_array_mut().unwrap().push(second);
    write(root, ".chrono-harness/ci/release.json", &c);
    let adopted_before = fs::read(root.join(".chrono-harness/ci/release.json")).unwrap();
    assert!(init(root, &source).unwrap());
    assert_eq!(
        fs::read(root.join(".chrono-harness/ci/release.json")).unwrap(),
        adopted_before
    );
    let changed = fs::read_to_string(&output).unwrap();
    assert_ne!(changed.as_bytes(), before);
    assert!(changed.contains("Host customization") && changed.contains("  another:\n"));
    assert!(!changed.contains("  push:\n"));
    assert!(!generate(root, ".chrono-harness/ci/release.json", true).unwrap());
    assert!(changed.contains("          include-hidden-files: true\n"));
    for drift in [
        format!("{changed}# altered\n"),
        changed.replace("          include-hidden-files: true\n", ""),
    ] {
        fs::write(&output, &drift).unwrap();
        let out = cli(
            root,
            &[
                "verify",
                "--host-root",
                root.to_str().unwrap(),
                "--config",
                ".chrono-harness/ci/release.json",
            ],
        );
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("drift"));
        assert_eq!(fs::read_to_string(&output).unwrap(), drift);
        assert!(generate(root, ".chrono-harness/ci/release.json", false).unwrap());
        assert!(!generate(root, ".chrono-harness/ci/release.json", true).unwrap());
    }
}

#[test]
fn invalid_release_declarations_fail_before_adoption() {
    let mut bad = Vec::new();
    for (key, value) in [
        ("schema", json!("chrono-github-release/v2")),
        ("extra", json!(true)),
        ("jobs", json!([])),
        ("name", json!("${{ secrets.X }}")),
        ("checkout_action", json!("actions/checkout@main")),
        ("include_hidden_files", json!("true")),
        ("push_branches", json!(["same", "same"])),
    ] {
        let mut c = config();
        c[key] = value;
        bad.push(c);
    }
    for (key, value) in [
        ("id", json!("bad: job")),
        ("command", json!([])),
        ("command", json!(["", "arg"])),
        ("command", json!(["tool", "${{ github.token }}"])),
        ("command", json!(["tool", "line\nbreak"])),
        ("artifact_directory", json!("../escape/")),
        ("artifact_directory", json!("out/*/")),
        ("artifact_directory", json!(".github/")),
        ("artifact_name", json!("bad/path")),
        ("timeout_minutes", json!(0)),
    ] {
        let mut c = config();
        c["jobs"][0][key] = value;
        bad.push(c);
    }
    for duplicate_id in [true, false] {
        let mut c = config();
        let mut second = c["jobs"][0].clone();
        if duplicate_id {
            second["artifact_name"] = json!("other")
        } else {
            second["id"] = json!("other")
        }
        c["jobs"].as_array_mut().unwrap().push(second);
        bad.push(c);
    }
    for c in bad {
        let d = crate::tools::temporary_host("host λ ");
        write(d.path(), "source.json", &c);
        assert!(
            init(d.path(), &d.path().join("source.json")).is_err(),
            "{c}"
        );
        assert!(!d.path().join(".github").exists());
        assert!(!d.path().join(".chrono-harness").exists());
    }
    let d = crate::tools::temporary_host("host λ ");
    let duplicate = serde_json::to_string(&config()).unwrap().replacen(
        "\"schema\":",
        "\"schema\":\"chrono-github-release/v1\",\"schema\":",
        1,
    );
    fs::write(d.path().join("source.json"), duplicate).unwrap();
    assert!(init(d.path(), &d.path().join("source.json")).is_err());
    let valid: release::Config = serde_json::from_value(config()).unwrap();
    assert!(release::render(&valid).is_ok());
}

#[test]
fn release_ownership_refuses_old_check_marker_unowned_and_symlink_outputs() {
    for text in ["host-owned\n", "# chrono-ci: owned github-actions/v1\n"] {
        let d = crate::tools::temporary_host("host λ ");
        let root = d.path();
        write(root, "source.json", &config());
        let path = root.join(".github/workflows/package.yml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        assert!(
            init(root, &root.join("source.json"))
                .unwrap_err()
                .contains("collision")
        );
        assert_eq!(fs::read_to_string(path).unwrap(), text);
        assert!(!root.join(".chrono-harness").exists());
    }
    let d = crate::tools::temporary_host("host λ ");
    let root = d.path();
    write(root, "source.json", &config());
    init(root, &root.join("source.json")).unwrap();
    let path = root.join(".github/workflows/package.yml");
    fs::remove_file(&path).unwrap();
    fs::write(root.join("keep"), "unchanged").unwrap();
    std::os::unix::fs::symlink("../../keep", &path).unwrap();
    assert!(generate(root, ".chrono-harness/ci/release.json", false).is_err());
    assert_eq!(fs::read_to_string(root.join("keep")).unwrap(), "unchanged");
    assert!(
        generate(root, "source.json", false)
            .unwrap_err()
            .contains(".chrono-harness")
    );
}

fn units_config() -> Value {
    let mut c = config();
    c["schema"] = json!("chrono-github-release/v2");
    c["download_artifact_action"] =
        json!("actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093");
    let mut consumer = c["jobs"][0].clone();
    consumer["id"] = json!("consumer");
    consumer["artifact_name"] = json!("consumer-output");
    consumer["artifact_directory"] = json!("consumer-out/");
    consumer["needs"] = json!(["native"]);
    consumer["downloads"] = json!([{"job":"native","directory":"inputs/native/"}]);
    consumer["always"] = json!(true);
    consumer["dependency_metadata"] = json!("inputs/dependencies.json");
    c["jobs"].as_array_mut().unwrap().push(consumer);
    c
}

#[test]
fn release_units_render_literal_edges_exact_artifact_ids_and_current_dependency_metadata() {
    let c: release::Config = serde_json::from_value(units_config()).unwrap();
    let yaml = release::render(&c).unwrap();
    assert!(yaml.contains("needs: [\"native\"]"));
    assert!(yaml.contains("artifact-ids: ${{ needs.native.outputs.artifact_id }}"));
    assert!(yaml.contains("path: \"inputs/native/\""));
    assert!(yaml.contains("artifact_id: ${{ steps.release_upload.outputs.artifact-id }}"));
    assert!(yaml.contains("${{ github.run_id }}-${{ github.run_attempt }}"));
    assert!(yaml.contains("CHRONO_RELEASE_DEPENDENCIES: ${{ toJson(needs) }}"));
    assert!(yaml.contains("if: ${{ always() && github.event.deleted != true }}"));
    assert!(!yaml.contains("cargo") && !yaml.contains("rustup"));
}

#[test]
fn shared_downloads_preserve_each_jobs_expanded_dependencies_and_destinations() {
    let mut value = units_config();
    for (id, directory) in [("same", "inputs/native/"), ("different", "other/native/")] {
        let mut job = value["jobs"][1].clone();
        job["id"] = json!(id);
        job["artifact_name"] = json!(format!("{id}-output"));
        job["downloads"][0]["directory"] = json!(directory);
        value["jobs"].as_array_mut().unwrap().push(job);
    }
    let c: release::Config = serde_json::from_value(value).unwrap();
    let yaml = release::render(&c).unwrap();
    assert_eq!(yaml.matches("uses: actions/download-artifact@").count(), 2);
    let expanded: Value = serde_yaml_ng::from_str(&yaml).unwrap();
    for job in c.jobs.iter().skip(1) {
        let actual = &expanded["jobs"][&job.id];
        assert_eq!(actual["needs"], json!(job.needs));
        assert_eq!(actual["timeout-minutes"], job.timeout_minutes);
        let downloads: Vec<_> = actual["steps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|step| {
                step["uses"]
                    .as_str()
                    .is_some_and(|v| v.starts_with("actions/download-artifact@"))
            })
            .collect();
        assert_eq!(downloads.len(), 1);
        assert_eq!(
            downloads[0]["with"]["artifact-ids"],
            "${{ needs.native.outputs.artifact_id }}"
        );
        assert_eq!(downloads[0]["with"]["path"], job.downloads[0].directory);
        assert_eq!(downloads[0]["with"]["merge-multiple"], true);
        assert_eq!(
            downloads[0]["if"],
            "${{ always() && needs.native.outputs.artifact_id != '' }}"
        );
    }
}

#[test]
fn oversized_workflows_reject_adoption_and_generation_without_writes() {
    let d = crate::tools::temporary_host("host λ ");
    let root = d.path();
    let source = root.join("source.json");
    let mut c = config();
    c["jobs"][0]["command"] = json!(["echo", "x".repeat(501_000)]);
    write(root, "source.json", &c);
    assert!(init(root, &source).unwrap_err().contains("500 KB"));
    let adopted = root.join(".chrono-harness/ci/release.json");
    let workflow = root.join(".github/workflows/package.yml");
    assert!(!adopted.exists());
    assert!(!workflow.exists());
    write(root, "source.json", &config());
    init(root, &source).unwrap();
    let previous = fs::read(&workflow).unwrap();
    write(root, ".chrono-harness/ci/release.json", &c);
    let before = fs::read(&adopted).unwrap();
    for verify in [false, true] {
        assert!(
            generate(root, ".chrono-harness/ci/release.json", verify)
                .unwrap_err()
                .contains("500 KB")
        );
        assert_eq!(fs::read(&workflow).unwrap(), previous);
        assert_eq!(fs::read(&adopted).unwrap(), before);
    }
}

#[test]
fn release_units_reject_undeclared_edges_cycles_and_overlapping_downloads() {
    for case in 0..7 {
        let mut c = units_config();
        match case {
            0 => c["jobs"][1]["needs"] = json!(["missing"]),
            1 => c["jobs"][0]["needs"] = json!(["consumer"]),
            2 => c["jobs"][1]["downloads"][0]["job"] = json!("missing"),
            3 => c["jobs"][1]["downloads"][0]["directory"] = json!("../escape/"),
            4 => {
                c["jobs"][1]["downloads"] = json!([
                {"job":"native","directory":"inputs/"},
                {"job":"native","directory":"inputs/nested/"}])
            }
            5 => c["jobs"][1]["dependency_metadata"] = json!("inputs/native/metadata.json"),
            _ => c["schema"] = json!("chrono-github-release/v1"),
        }
        let c: release::Config = serde_json::from_value(c).unwrap();
        assert!(release::render(&c).is_err(), "case {case}");
    }
}

#[test]
fn release_units_cli_runs_non_rust_literal_command_and_writes_original_dependency_data() {
    let d = crate::tools::temporary_host("units host λ ");
    let root = d.path();
    let program = root.join("consumer command.py");
    fs::write(&program,"import json,sys\nfrom pathlib import Path\nPath('args.json').write_text(json.dumps(sys.argv[1:]))\nsys.stderr.buffer.write(b'original failure\\xff')\nsys.exit(17)\n").unwrap();
    let mut c = units_config();
    c["jobs"][1]["command"] = json!([
        "/usr/bin/python3",
        program.to_str().unwrap(),
        "$HOME",
        "$(touch wrong)",
        "space value"
    ]);
    c["jobs"][1]["dependency_metadata"] = json!("metadata with spaces/current.json");
    write(root, "source.json", &c);
    assert!(init(root, &root.join("source.json")).unwrap());
    assert!(!generate(root, ".chrono-harness/ci/release.json", true).unwrap());
    let yaml = fs::read_to_string(root.join(".github/workflows/package.yml")).unwrap();
    let command = yaml
        .split("      - name: Run registered release command\n")
        .nth(2)
        .unwrap();
    let command = script(command);
    let original = r#"{"native":{"result":"failure","outputs":{"artifact_id":"123","attempt":"1","run_id":"45"}}}"#;
    let result = Command::new("/bin/bash")
        .args(["-e", "-c", &command])
        .env("CHRONO_RELEASE_DEPENDENCIES", original)
        .current_dir(root)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(17));
    assert_eq!(result.stderr, b"original failure\xff");
    assert_eq!(
        fs::read(root.join("metadata with spaces/current.json")).unwrap(),
        format!("{original}\n").as_bytes()
    );
    let argv: Value = serde_json::from_slice(&fs::read(root.join("args.json")).unwrap()).unwrap();
    assert_eq!(argv, json!(["$HOME", "$(touch wrong)", "space value"]));
    assert!(!root.join("wrong").exists());
}

#[test]
fn release_v1_rejects_even_empty_opt_in_fields() {
    for (key, value) in [
        ("needs", json!([])),
        ("downloads", json!([])),
        ("always", json!(false)),
        ("dependency_metadata", Value::Null),
    ] {
        let d = crate::tools::temporary_host("host λ ");
        let mut c = config();
        c["jobs"][0][key] = value;
        write(d.path(), "source.json", &c);
        assert!(init(d.path(), &d.path().join("source.json")).is_err());
    }
}

#[test]
fn release_readoption_rejects_v2_fields_in_existing_v1_without_writes() {
    for (key, value) in [
        ("needs", json!([])),
        ("downloads", json!([])),
        ("always", json!(false)),
        ("dependency_metadata", Value::Null),
        ("download_artifact_action", Value::Null),
    ] {
        let d = crate::tools::temporary_host("host λ ");
        let root = d.path();
        let source = root.join("source.json");
        write(root, "source.json", &config());
        init(root, &source).unwrap();
        let adopted = root.join(".chrono-harness/ci/release.json");
        let mut existing = config();
        if key == "download_artifact_action" {
            existing[key] = value;
        } else {
            existing["jobs"][0][key] = value;
        }
        write(root, ".chrono-harness/ci/release.json", &existing);
        let original = fs::read(&adopted).unwrap();
        let workflow = root.join(".github/workflows/package.yml");
        let rendered = fs::read(&workflow).unwrap();
        for result in [
            init(root, &source),
            generate(root, ".chrono-harness/ci/release.json", false),
            generate(root, ".chrono-harness/ci/release.json", true),
        ] {
            assert!(result.unwrap_err().contains("require v2"), "{key}");
            assert_eq!(fs::read(&adopted).unwrap(), original, "{key}");
            assert_eq!(fs::read(&workflow).unwrap(), rendered, "{key}");
        }
    }
}

#[test]
fn release_single_id_download_extracts_receipt_at_the_registered_root() {
    let c: release::Config = serde_json::from_value(units_config()).unwrap();
    let yaml = release::render(&c).unwrap();
    let d = crate::tools::temporary_host("host λ ");
    let workflow = d.path().join("workflow.yml");
    fs::write(&workflow, yaml).unwrap();
    // This models the exact pinned action's extraction contract, including its
    // default-false negative control. A single artifact ID is NOT a name download.
    // https://github.com/actions/download-artifact/blob/d3f86a106a0bac45b974a628896c90dbdf5c8093/src/download-artifact.ts
    let out = Command::new("/usr/bin/python3")
        .args(["-c", r#"
import io, pathlib, sys, zipfile
root = pathlib.Path(sys.argv[1]); workflow = (root/'workflow.yml').read_text()
step = workflow.split('      - name: Download selected original artifact from native\n')[1].split('      - name:')[0]
assert 'uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093' in step
inputs = dict(line.strip().split(': ', 1) for line in step.splitlines() if line.startswith('          '))
assert inputs['artifact-ids'] == '${{ needs.native.outputs.artifact_id }}'
assert 'name' not in inputs and 'pattern' not in inputs
registered = root / inputs['path'].strip('"')
artifact_name = 'native-output-123-2'
archive = io.BytesIO()
with zipfile.ZipFile(archive, 'w') as z: z.writestr('receipt.json', b'original receipt bytes')
def extract(inputs, resolved_path):
    # Pinned action: isSingleArtifactDownload = !!inputs.name; merge default false.
    target = resolved_path if inputs.get('name') or inputs.get('merge-multiple', 'false') == 'true' else resolved_path / artifact_name
    with zipfile.ZipFile(archive) as z: z.extractall(target)
    return target
negative = root/'default-control'
assert extract({'artifact-ids': '567'}, negative) == negative/artifact_name
assert not (negative/'receipt.json').exists()
assert (negative/artifact_name/'receipt.json').read_bytes() == b'original receipt bytes'
extract(inputs, registered)
assert (registered/'receipt.json').read_bytes() == b'original receipt bytes'
assert not (registered/artifact_name).exists()
"#, d.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
