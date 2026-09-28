use chrono_harness::sha256;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

const BUILD: &str = ".chrono-harness/release/build.json";
const PROJECTS: &str = "registered/actions.json";
const STUB: &str = r#"#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
name = Path(sys.argv[0]).name
with open(os.environ['CHRONO_RECIPE_CALLS'], 'a') as log:
    log.write(json.dumps({'tool':name,'argv':sys.argv[1:],'cwd':str(Path.cwd())})+'\n')
failure = os.environ.get('CHRONO_RECIPE_FAIL')
if ((failure == 'source' and name == 'git') or
    (failure == 'install' and name == 'rustup') or
    (failure == 'build' and name == 'cargo' and sys.argv[1:2] == ['build']) or
    (failure == 'pack' and name == 'chrono-distribution')):
    sys.stdout.buffer.write(b'failed phase\xff\n')
    sys.stderr.buffer.write(b'original diagnostic\xfe\n')
    if failure == 'pack':
        Path(sys.argv[sys.argv.index('--output')+1]).mkdir()
    sys.exit(41)
if failure == 'version' and name == 'rustc':
    print('rustc different-version')
    sys.exit(0)
if name == 'rustc': print('rustc 1.95.0 fixture')
elif name == 'cargo' and sys.argv[1:] == ['--version']: print('cargo fixture')
elif name == 'git': print('a'*40)
elif name == 'probe':
    sys.stdout.buffer.write(b'actual stdout\xff\n')
    sys.stderr.buffer.write(b'actual stderr\xfe\n')
    if len(sys.argv) > 1 and os.environ.get('CHRONO_RECIPE_FAIL') == sys.argv[1]: sys.exit(73)
elif name == 'chrono-distribution':
    Path(sys.argv[sys.argv.index('--output')+1]).mkdir()
"#;

fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

struct Recipe {
    _temp: TempDir,
    root: PathBuf,
    output: PathBuf,
    calls: PathBuf,
}

impl Recipe {
    fn new() -> Self {
        let temp = tempfile::Builder::new()
            .prefix("release consumer λ ")
            .tempdir()
            .unwrap();
        let root = fs::canonicalize(temp.path())
            .unwrap()
            .join("source with spaces");
        fs::create_dir_all(root.join("tools")).unwrap();
        for tool in [
            "tools/rustup",
            "tools/cargo",
            "tools/rustc",
            "tools/git",
            "tools/probe",
            "crates/distribution/target/release/chrono-distribution",
        ] {
            let path = root.join(tool);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, STUB).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::create_dir_all(root.join("arbitrary layout")).unwrap();
        fs::write(root.join("arbitrary layout/product.toml"), "fixture").unwrap();
        write(
            &root.join(BUILD),
            &json!({
                "schema":"chrono-release-build/v2", "rust_toolchain":"1.95.0",
                "manifests":["arbitrary layout/product.toml"], "projects":PROJECTS,
                "tools":{"rustup":"tools/rustup","cargo":"tools/cargo","rustc":"tools/rustc","git":"tools/git","probe":"tools/probe"},
                "verification_operations":["prepare.custom", "verify.custom", "finish.custom"]
            }),
        );
        write(
            &root.join(PROJECTS),
            &json!({"projects":[{"id":"custom","actions":{
                "first":{"operation":"prepare.custom","tool":"probe","argv":["prepare", "a space", "$literal\n`text`"]},
                "second":{"operation":"verify.custom","tool":"probe","argv":["verify", "--declared"]}
            }}],"scripts":[{"id":"standalone","actions":{
                "run":{"operation":"finish.custom","tool":"probe","argv":[]}
            }}]}),
        );
        Self {
            output: temp.path().join("native output"),
            calls: temp.path().join("calls.jsonl"),
            _temp: temp,
            root,
        }
    }

    fn edit(&self, path: &str, f: impl FnOnce(&mut Value)) {
        let path = self.root.join(path);
        let mut value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        f(&mut value);
        write(&path, &value);
    }

    fn run(&self, fail: &str) -> Output {
        let script =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.chrono-harness/release/build.py");
        Command::new("python3")
            .current_dir("/")
            .env("CHRONO_RECIPE_CALLS", &self.calls)
            .env("CHRONO_RECIPE_FAIL", fail)
            .env(
                "PATH",
                std::env::join_paths(
                    [self.root.join("tools")]
                        .into_iter()
                        .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
                )
                .unwrap(),
            )
            .args([script.as_path(), self.root.as_path(), self.output.as_path()])
            .output()
            .unwrap()
    }

    fn calls(&self) -> Vec<Value> {
        fs::read_to_string(&self.calls)
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

#[test]
fn release_recipe_runs_registered_operations_in_order_with_literal_arguments() {
    let f = Recipe::new();
    let out = f.run("");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let calls = f.calls();
    assert!(calls.iter().all(|c| c["cwd"] == f.root.to_str().unwrap()));
    let probes: Vec<_> = calls.iter().filter(|c| c["tool"] == "probe").collect();
    assert_eq!(probes.len(), 3);
    assert_eq!(
        probes[0]["argv"],
        json!(["prepare", "a space", "$literal\n`text`"])
    );
    assert_eq!(probes[1]["argv"], json!(["verify", "--declared"]));
    assert_eq!(probes[2]["argv"], json!([]));
    let pack = calls
        .iter()
        .position(|c| c["tool"] == "chrono-distribution")
        .unwrap();
    assert_eq!(calls[pack - 1]["tool"], "probe");
    let report: Value =
        serde_json::from_slice(&fs::read(f.output.join("build.json")).unwrap()).unwrap();
    assert_eq!(report["schema"], "chrono-native-build/v3");
    assert_eq!(report["status"], "passed");
    assert!(report["failure"].is_null());
    assert_eq!(report["source_commit"], "a".repeat(40));
    let verification: Vec<_> = report["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["phase"] == "verification")
        .collect();
    assert_eq!(verification.len(), 3);
    assert_eq!(report["processes"].as_array().unwrap().len(), calls.len());
    assert_eq!(
        report["processes"].as_array().unwrap().last().unwrap()["phase"],
        "package"
    );
    for (i, operation) in ["prepare.custom", "verify.custom", "finish.custom"]
        .iter()
        .enumerate()
    {
        let actual = verification[i];
        assert_eq!(actual["operation"], *operation);
        assert_eq!(actual["exit_code"], 0);
        assert_eq!(actual["cwd"], f.root.to_str().unwrap());
        assert_eq!(
            actual["argv"][0],
            f.root.join("tools/probe").to_str().unwrap()
        );
        assert_eq!(
            actual["stdout_bytes"],
            json!(b"actual stdout\xff\n".to_vec())
        );
        assert_eq!(
            actual["stderr_bytes"],
            json!(b"actual stderr\xfe\n".to_vec())
        );
        assert_eq!(actual["stdout_sha256"], sha256(b"actual stdout\xff\n"));
        assert_eq!(actual["stderr_sha256"], sha256(b"actual stderr\xfe\n"));
    }
}

#[test]
fn release_recipe_rejects_invalid_operations_before_building() {
    for case in 0..6 {
        let f = Recipe::new();
        match case {
            0 => f.edit(BUILD, |v| v["verification_operations"] = json!(["missing"])),
            1 => f.edit(BUILD, |v| {
                v["verification_operations"] = json!(["verify.custom", "verify.custom"])
            }),
            2 => f.edit(BUILD, |v| {
                v["tools"].as_object_mut().unwrap().remove("probe");
            }),
            3 => f.edit(PROJECTS, |v| {
                v["projects"][0]["actions"]["second"]["operation"] = json!("prepare.custom")
            }),
            4 => f.edit(PROJECTS, |v| {
                v["projects"][0]["actions"]["second"]["argv"] = json!(["verify", 7])
            }),
            _ => f.edit(BUILD, |v| v["tools"]["probe"] = json!("tools/missing")),
        }
        let out = f.run("");
        assert!(!out.status.success(), "case {case}");
        assert!(
            f.calls().is_empty(),
            "case {case} started work before preflight completed"
        );
        assert!(!f.output.exists());
    }
}

#[test]
fn release_recipe_preserves_failed_operation_exit_and_never_packages() {
    let f = Recipe::new();
    let out = f.run("prepare");
    assert_eq!(out.status.code(), Some(73));
    let calls = f.calls();
    assert_eq!(calls.last().unwrap()["argv"][0], "prepare");
    assert!(!calls.iter().any(|c| c["tool"] == "chrono-distribution"));
    let report = evidence(&f);
    assert_eq!(report["status"], "failed");
    assert!(!f.output.join("release.json").exists());
    assert!(
        out.stdout
            .windows(b"actual stdout\xff\n".len())
            .any(|w| w == b"actual stdout\xff\n")
    );
    assert!(
        out.stderr
            .windows(b"actual stderr\xfe\n".len())
            .any(|w| w == b"actual stderr\xfe\n")
    );
}

fn evidence(f: &Recipe) -> Value {
    let report: Value = serde_json::from_slice(
        &fs::read(f.output.join("build.json")).expect("retained native evidence"),
    )
    .unwrap();
    assert_eq!(report["schema"], "chrono-native-build/v3");
    assert_eq!(report["source_commit"], "a".repeat(40));
    for p in report["processes"].as_array().unwrap() {
        assert_eq!(p["cwd"], f.root.to_str().unwrap());
        for stream in ["stdout", "stderr"] {
            let bytes: Vec<u8> =
                serde_json::from_value(p[format!("{stream}_bytes")].clone()).unwrap();
            assert_eq!(p[format!("{stream}_sha256")], json!(sha256(&bytes)));
        }
    }
    report
}

#[test]
fn release_recipe_retains_failed_operation_and_prior_processes() {
    let f = Recipe::new();
    let out = f.run("verify");
    assert_eq!(out.status.code(), Some(73));
    let report = evidence(&f);
    assert_eq!(report["status"], "failed");
    assert_eq!(report["failure"]["phase"], "verification");
    assert_eq!(report["failure"]["exit_code"], 73);
    let all = report["processes"].as_array().unwrap();
    assert_eq!(all.len(), f.calls().len(), "every actual process retained");
    let last = all.last().unwrap();
    assert_eq!(last["operation"], "verify.custom");
    assert_eq!(last["exit_code"], 73);
    assert_eq!(last["stdout_bytes"], json!(b"actual stdout\xff\n".to_vec()));
    assert_eq!(last["stderr_bytes"], json!(b"actual stderr\xfe\n".to_vec()));
    assert!(
        all.iter()
            .any(|p| p["operation"] == "prepare.custom" && p["exit_code"] == 0)
    );
    assert!(
        !all.iter()
            .any(|p| p["operation"] == "finish.custom" || p["phase"] == "package")
    );
    assert!(!f.output.join("release.json").exists());
}

#[test]
fn release_recipe_retains_install_build_and_package_failures() {
    for (fault, phase) in [
        ("install", "toolchain-install"),
        ("build", "build"),
        ("pack", "package"),
    ] {
        let f = Recipe::new();
        let out = f.run(fault);
        assert_eq!(out.status.code(), Some(41));
        let report = evidence(&f);
        assert_eq!(report["status"], "failed");
        assert_eq!(report["failure"]["phase"], phase);
        let all = report["processes"].as_array().unwrap();
        assert_eq!(all.len(), f.calls().len());
        let last = all.last().unwrap();
        assert_eq!(last["phase"], phase);
        assert_eq!(last["exit_code"], 41);
        assert_eq!(last["stdout_bytes"], json!(b"failed phase\xff\n".to_vec()));
        assert_eq!(
            last["stderr_bytes"],
            json!(b"original diagnostic\xfe\n".to_vec())
        );
        assert!(!f.output.join("release.json").exists());
    }
}

#[test]
fn release_recipe_semantic_failure_keeps_successful_process_exit() {
    let f = Recipe::new();
    let out = f.run("version");
    assert_eq!(out.status.code(), Some(2));
    let report = evidence(&f);
    assert_eq!(report["status"], "failed");
    assert_eq!(report["failure"]["phase"], "compiler-version");
    assert!(report["failure"]["exit_code"].is_null());
    let all = report["processes"].as_array().unwrap();
    assert_eq!(all.last().unwrap()["exit_code"], 0);
    assert_eq!(
        all.last().unwrap()["stdout_bytes"],
        json!(b"rustc different-version\n".to_vec())
    );
    assert!(
        !all.iter()
            .any(|p| p["phase"] == "build" || p["phase"] == "package")
    );
}

#[test]
fn release_recipe_missing_program_and_existing_output_preserve_real_state() {
    let f = Recipe::new();
    fs::remove_file(
        f.root
            .join("crates/distribution/target/release/chrono-distribution"),
    )
    .unwrap();
    let out = f.run("");
    assert_eq!(out.status.code(), Some(2));
    let report = evidence(&f);
    assert_eq!(report["status"], "failed");
    assert_eq!(report["failure"]["phase"], "package");
    assert!(report["failure"]["exit_code"].is_null());
    assert!(
        !report["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["phase"] == "package")
    );
    let f = Recipe::new();
    fs::create_dir(&f.output).unwrap();
    fs::write(f.output.join("build.json"), "preexisting evidence").unwrap();
    let out = f.run("");
    assert_eq!(out.status.code(), Some(2));
    assert!(f.calls().is_empty());
    assert_eq!(
        fs::read_to_string(f.output.join("build.json")).unwrap(),
        "preexisting evidence"
    );
    let f = Recipe::new();
    std::os::unix::fs::symlink(f.root.join("absent destination"), &f.output).unwrap();
    let out = f.run("");
    assert_eq!(out.status.code(), Some(2));
    assert!(f.calls().is_empty());
    assert!(f.output.is_symlink());
    assert!(!f.root.join("absent destination").exists());
}

#[test]
fn release_recipe_retains_source_failure_without_inventing_identity() {
    let f = Recipe::new();
    let out = f.run("source");
    assert_eq!(out.status.code(), Some(41));
    let report: Value =
        serde_json::from_slice(&fs::read(f.output.join("build.json")).unwrap()).unwrap();
    assert_eq!(report["schema"], "chrono-native-build/v3");
    assert_eq!(report["status"], "failed");
    assert!(report["source_commit"].is_null());
    assert_eq!(f.calls().len(), 1);
    let processes = report["processes"].as_array().unwrap();
    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0]["phase"], "source-identity");
    assert_eq!(processes[0]["exit_code"], 41);
    assert_eq!(
        processes[0]["stdout_bytes"],
        json!(b"failed phase\xff\n".to_vec())
    );
    assert_eq!(
        processes[0]["stdout_sha256"],
        json!(sha256(b"failed phase\xff\n"))
    );
}
