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
                "second":{"operation":"verify.custom","tool":"probe","argv":["verify", "--declared"]},
                "third":{"operation":"finish.custom","tool":"probe","argv":[]}
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
    assert_eq!(report["schema"], "chrono-native-build/v2");
    assert_eq!(report["source_commit"], "a".repeat(40));
    assert_eq!(report["verification"].as_array().unwrap().len(), 3);
    for (i, operation) in ["prepare.custom", "verify.custom", "finish.custom"]
        .iter()
        .enumerate()
    {
        let actual = &report["verification"][i];
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
    assert!(!f.output.exists());
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
