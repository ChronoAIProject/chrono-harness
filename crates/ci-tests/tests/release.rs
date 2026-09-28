use chrono_ci::{generate, init, release};
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
    Command::new(host.join(".chrono-harness/bin/chrono-ci"))
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
    let d = tempfile::Builder::new()
        .prefix("release host ")
        .tempdir()
        .unwrap();
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
    let d = tempfile::tempdir().unwrap();
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
    c["push_branches"] = json!([]);
    let mut second = c["jobs"][0].clone();
    second["id"] = json!("another");
    second["runs_on"] = json!("macos-14");
    second["artifact_name"] = json!("another-artifact");
    c["jobs"].as_array_mut().unwrap().push(second);
    write(root, ".chrono-harness/ci/release.json", &c);
    assert!(init(root, &source).unwrap());
    let changed = fs::read_to_string(&output).unwrap();
    assert_ne!(changed.as_bytes(), before);
    assert!(changed.contains("Host customization") && changed.contains("  another:\n"));
    assert!(!changed.contains("  push:\n"));
    assert!(!generate(root, ".chrono-harness/ci/release.json", true).unwrap());
    let drift = changed + "# altered\n";
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

#[test]
fn invalid_release_declarations_fail_before_adoption() {
    let mut bad = Vec::new();
    for (key, value) in [
        ("schema", json!("chrono-github-release/v2")),
        ("extra", json!(true)),
        ("jobs", json!([])),
        ("name", json!("${{ secrets.X }}")),
        ("checkout_action", json!("actions/checkout@main")),
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
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "source.json", &c);
        assert!(
            init(d.path(), &d.path().join("source.json")).is_err(),
            "{c}"
        );
        assert!(!d.path().join(".github").exists());
        assert!(!d.path().join(".chrono-harness").exists());
    }
    let d = tempfile::tempdir().unwrap();
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
        let d = tempfile::tempdir().unwrap();
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
    let d = tempfile::tempdir().unwrap();
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
