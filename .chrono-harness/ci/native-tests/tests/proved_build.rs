//! 正式生产者的命令边界验收；外部编译器全部由 Rust mock 提供。
use chrono_host_bootstrap::{bindings::Environment, proved_build};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    environment: Environment,
    config: Value,
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn escape(path: &Path) -> String {
    path.to_str().unwrap().replace(' ', "\\ ")
}

// 外部 Rust 1.95 接口样本，独立写出多目标、phony 和 checksum 全文。
fn dependency(path: &Path, outputs: &[PathBuf], inputs: &[PathBuf]) {
    let list = inputs
        .iter()
        .map(|p| escape(p))
        .collect::<Vec<_>>()
        .join(" ");
    let mut raw = String::new();
    for target in std::iter::once(path.to_path_buf()).chain(outputs.iter().cloned()) {
        raw += &format!("{}: {list}\n\n", escape(&target));
    }
    for source in inputs {
        raw += &format!("{}:\n", escape(source));
    }
    raw += "\n";
    for source in inputs {
        let (digest, length) = chrono_harness::file_identity(source).unwrap();
        raw += &format!(
            "# checksum:sha256={digest} file_len:{length} {}\n",
            escape(source)
        );
    }
    write(path, raw.as_bytes());
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::Builder::new()
            .prefix("proof fixture ")
            .tempdir()
            .unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let config = json!({"schema":"chrono-proved-build/v1","owner":"app","operation":"build.app",
            "projects":".chrono-harness/projects.json","recovery_operation":"recover.fixture-proof",
            "manifest":"app/Cargo.toml","target":"app/target","cargo_verus":".chrono-harness/bin/verus/cargo-verus",
            "verus_directory":".chrono-harness/bin/verus","rust_toolchain":"1.95.0",
            "output_contract":{"version":"v","commit":"c"},
            "library":{"manifest":"library/Cargo.toml","source":"library/src/lib.rs","target":"identity","sources":["library/src/lib.rs"]},
            "binary":{"source":"app/src/main.rs","target":"app","published":"app/target/debug/app","dependency_prefix":"app-"}});
        write(
            &root.join(".chrono-harness/ci/build.json"),
            &serde_json::to_vec(&config).unwrap(),
        );
        write(&root.join(".chrono-harness/projects.json"), &serde_json::to_vec(&json!({"projects":[{"id":"identity","actions":{
            "recover":{"operation":"recover.fixture-proof","tool":"cargo","argv":["clean","--package","identity","--manifest-path","app/Cargo.toml","--target-dir","app/target"]}}}],"scripts":[]})).unwrap());
        for name in [
            "cargo-verus",
            "verus",
            "rust_verify",
            "z3",
            "libvstd.rlib",
            "vstd.vir",
        ] {
            let destination = root.join(".chrono-harness/bin/verus").join(name);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(env!("CARGO_BIN_EXE_chrono-test-proved-cargo"), destination).unwrap();
        }
        write(&root.join("app/Cargo.toml"), b"[package]\nname=\"app\"\n");
        write(
            &root.join("library/Cargo.toml"),
            b"[package]\nname=\"identity\"\n",
        );
        write(&root.join("library/src/lib.rs"), b"pub fn identity() {}\n");
        write(&root.join("app/src/main.rs"), b"fn main() {}\n");
        let deps = root.join("app/target/debug/deps");
        let library = vec![
            deps.join("libidentity-01.rlib"),
            deps.join("libidentity-01.rmeta"),
        ];
        for path in &library {
            write(path, b"proved library");
        }
        dependency(
            &deps.join("identity-01.d"),
            &library,
            &[root.join("library/src/lib.rs")],
        );
        let binary = deps.join("app-01");
        write(&binary, b"linked executable");
        write(&root.join("app/target/debug/app"), b"linked executable");
        let mut inputs = vec![root.join("app/src/main.rs")];
        inputs.extend(library);
        dependency(&deps.join("app-01.d"), &[binary], &inputs);
        let mut environment: Environment = std::env::vars_os().collect();
        environment.insert(
            "CHRONO_PROOF_FIXTURE".into(),
            root.join("mock.json").into_os_string(),
        );
        environment.insert(
            "CARGO_HOME".into(),
            root.join("cargo-home").into_os_string(),
        );
        environment.insert(
            "RUSTUP_HOME".into(),
            root.join("rustup-home").into_os_string(),
        );
        let cargo = root.join("mock-tools/cargo");
        fs::create_dir_all(cargo.parent().unwrap()).unwrap();
        fs::copy(env!("CARGO_BIN_EXE_chrono-test-proved-cargo"), &cargo).unwrap();
        environment.insert(
            "PATH".into(),
            cargo.parent().unwrap().as_os_str().to_os_string(),
        );
        Self {
            _temp: temp,
            root,
            environment,
            config,
        }
    }
    fn argv(&self) -> Vec<String> {
        let verus = self.root.join(".chrono-harness/bin/verus");
        let flags = json!([
            "-Zbinary-dep-depinfo",
            "-Zchecksum-hash-algorithm=sha256",
            "--extern",
            format!("vstd={}/libvstd.rlib", verus.display()),
            "-L",
            format!("dependency={}", verus.display())
        ]);
        vec![
            "build".into(),
            "--jobs".into(),
            "1".into(),
            "-v".into(),
            "--fwd-verus-args-to".into(),
            "all".into(),
            "--locked".into(),
            "--manifest-path".into(),
            "app/Cargo.toml".into(),
            "--target-dir".into(),
            "app/target".into(),
            "--config".into(),
            format!("build.rustflags={flags}"),
            "--message-format=json".into(),
            "--".into(),
            "--import".into(),
            format!("vstd={}/vstd.vir", verus.display()),
            "--no-cheating".into(),
            "--no-external-by-default".into(),
            "--num-threads".into(),
            "2".into(),
            "--output-json".into(),
            "-Zchecksum-hash-algorithm=sha256".into(),
        ]
    }
    fn records(&self, fresh: bool) -> Vec<Value> {
        let mut records = Vec::new();
        if !fresh {
            records.push(json!({"verus":{"version":"v","commit":"c"},"verification-results":{"encountered-error":false,"encountered-vir-error":false,
                "success":true,"verified":1,"errors":0,"is-verifying-entire-crate":true}}));
        }
        records.push(json!({"reason":"compiler-artifact","package_id":"identity@0.1.0","manifest_path":self.root.join("library/Cargo.toml"),
            "target":{"name":"identity","src_path":self.root.join("library/src/lib.rs"),"kind":["lib"],"crate_types":["lib"]},
            "filenames":[self.root.join("app/target/debug/deps/libidentity-01.rlib"),self.root.join("app/target/debug/deps/libidentity-01.rmeta")],"executable":null,"fresh":fresh}));
        records.push(json!({"reason":"compiler-artifact","package_id":"app@0.1.0","manifest_path":self.root.join("app/Cargo.toml"),
            "target":{"name":"app","src_path":self.root.join("app/src/main.rs"),"kind":["bin"],"crate_types":["bin"]},
            "filenames":[self.root.join("app/target/debug/app")],"executable":self.root.join("app/target/debug/app"),"fresh":fresh}));
        records.push(json!({"reason":"build-finished","success":true}));
        records
    }
    fn plan(&self, records: &[Value], exit: u8, writes: Value) {
        let mut stdout = Vec::new();
        for record in records {
            serde_json::to_writer(&mut stdout, record).unwrap();
            stdout.push(b'\n');
        }
        write(&self.root.join("mock.json"), &serde_json::to_vec(&json!([{"argv":self.argv(),"stdout":stdout,"stderr":[],"writes":writes,"exit":exit}])).unwrap());
        let calls = self.root.join("mock.calls");
        if calls.exists() {
            fs::remove_file(calls).unwrap();
        }
    }
    fn recovery_plan(&self, exit: u8, remains_fresh: bool) {
        let encode = |records: Vec<Value>| {
            let mut bytes = Vec::new();
            for r in records {
                serde_json::to_writer(&mut bytes, &r).unwrap();
                bytes.push(b'\n');
            }
            bytes
        };
        let build = |records: Vec<Value>| json!({"argv":self.argv(),"stdout":encode(records),"stderr":[],"writes":[],"exit":0});
        let clean = json!({"argv":["clean","--package","identity","--manifest-path","app/Cargo.toml","--target-dir","app/target"],"stdout":[],"stderr":b"cache recovery original".to_vec(),"writes":[],"exit":exit});
        let mut invocations = vec![build(self.records(true)), clean];
        if exit == 0 {
            invocations.push(build(self.records(remains_fresh)));
        }
        write(
            &self.root.join("mock.json"),
            &serde_json::to_vec(&invocations).unwrap(),
        );
    }
    fn run(&self) -> Result<(), String> {
        proved_build::build(
            &self.root,
            ".chrono-harness/ci/build.json",
            &self.environment,
            &mut Vec::new(),
            &mut Vec::new(),
        )
        .map_err(|e| e.message)
    }
    fn proof(&self) -> PathBuf {
        self.root.join("app/target/.chrono-verus-proof.json")
    }
}

#[test]
fn cold_and_warm_builds_bind_the_same_proof_and_library_without_a_new_report() {
    let f = Fixture::new();
    f.plan(&f.records(false), 0, json!([]));
    f.run().unwrap();
    let original = fs::read(f.proof()).unwrap();
    f.plan(&f.records(true), 0, json!([]));
    f.run().unwrap();
    assert_eq!(
        fs::read(f.proof()).unwrap(),
        original,
        "warm use preserves the original producer"
    );
    assert_eq!(
        fs::read_dir(f.root.join(".chrono-harness/state/proved-builds"))
            .unwrap()
            .count(),
        0,
        "successful calls consume their temporary originals and leave no accumulating directories"
    );
    let proof: Value = serde_json::from_slice(&original).unwrap();
    assert!(proof["stdout"].is_string());
    assert!(proof["dependency"].is_string());
    let _ = &f.config;
}

#[test]
fn unrelated_operation_registration_does_not_invalidate_the_library_proof() {
    let f = Fixture::new();
    f.plan(&f.records(false), 0, json!([]));
    f.run().unwrap();
    let proof = fs::read(f.proof()).unwrap();
    let path = f.root.join(".chrono-harness/projects.json");
    let mut registry: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    registry["scripts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"unrelated","actions":{
        "build":{"operation":"build.unrelated","tool":"cargo","argv":["build"]}}}));
    write(&path, &serde_json::to_vec(&registry).unwrap());
    f.plan(&f.records(true), 0, json!([]));
    f.run().unwrap();
    assert_eq!(fs::read(f.proof()).unwrap(), proof);
    assert_eq!(fs::read_to_string(f.root.join("mock.calls")).unwrap(), "1");
}

#[test]
fn missing_truncated_unknown_duplicate_or_failed_output_never_publishes_a_proof() {
    for variant in 0..6 {
        let f = Fixture::new();
        let mut records = f.records(false);
        let exit = if variant == 0 { 7 } else { 0 };
        match variant {
            1 => {
                records.remove(0);
            }
            2 => {
                records.pop();
            }
            3 => {
                records.push(json!({"reason":"unknown-record"}));
            }
            4 => {
                records.push(json!({"reason":"build-finished","success":true}));
            }
            5 => {
                records[0]["verification-results"]["success"] = json!(false);
            }
            _ => {}
        }
        f.plan(&records, exit, json!([]));
        assert!(f.run().is_err(), "variant {variant}");
        assert!(
            !f.proof().exists(),
            "failed originals cannot become reusable proof"
        );
        assert!(f.root.join(".chrono-harness/state/proved-builds").exists());
    }
}

#[test]
fn source_library_and_final_link_drift_each_reject_independently() {
    for path in [
        "library/src/lib.rs",
        "app/target/debug/deps/libidentity-01.rlib",
        "app/target/debug/deps/app-01",
        "app/target/debug/app",
    ] {
        let f = Fixture::new();
        f.plan(
            &f.records(false),
            0,
            json!([[f.root.join(path), b"drift".to_vec()]]),
        );
        assert!(f.run().is_err(), "drift: {path}");
        assert!(!f.proof().exists());
    }
}

#[test]
fn absent_or_corrupt_retained_proof_rebuilds_through_the_registered_owner() {
    for retained in [None, Some(b"{invalid}".as_slice())] {
        let f = Fixture::new();
        if let Some(bytes) = retained {
            write(&f.proof(), bytes);
        }
        f.recovery_plan(0, false);
        f.run().unwrap();
        assert_eq!(fs::read_to_string(f.root.join("mock.calls")).unwrap(), "3");
        let proof: Value = serde_json::from_slice(&fs::read(f.proof()).unwrap()).unwrap();
        assert_eq!(
            proof["artifact"]["fresh"], false,
            "only the new original can replace a rejected cache"
        );
    }
}

#[test]
fn failed_or_ineffective_recovery_preserves_the_failure_and_never_loops() {
    for (exit, remains_fresh) in [(9, false), (0, true)] {
        let f = Fixture::new();
        f.recovery_plan(exit, remains_fresh);
        assert!(f.run().is_err());
        assert_eq!(
            fs::read_to_string(f.root.join("mock.calls")).unwrap(),
            if exit == 9 { "2" } else { "3" }
        );
        assert!(!f.proof().exists());
    }
}

#[test]
fn retained_recovery_originals_publish_a_consumer_that_can_be_released() {
    let f = Fixture::new();
    write(&f.root.join(".chrono-harness/retention.json"), &serde_json::to_vec(&json!({
        "schema":"chrono-output-retention/v2", "producers":{"check-original":[".chrono-harness/state/proved-builds/"]},
        "keep_seconds":0, "max_related_outputs":512, "max_summaries":128, "max_nodes_per_round":256,
        "max_bytes_per_round":67108864, "max_millis_per_round":25
    })).unwrap());
    f.recovery_plan(0, false);
    let mut errors = Vec::new();
    proved_build::build(
        &f.root,
        ".chrono-harness/ci/build.json",
        &f.environment,
        &mut Vec::new(),
        &mut errors,
    )
    .unwrap();
    let start = errors.iter().position(|byte| *byte == b'{').unwrap();
    let notice: Value = serde_json::from_slice(&errors[start..]).unwrap();
    let declaration = &notice["report_consumer"];
    let raw = serde_json::to_vec(declaration).unwrap();
    let declaration_path = ".chrono-harness/state/repair-consumer.json";
    write(&f.root.join(declaration_path), &raw);
    let reference = json!({"path":declaration_path,"sha256":chrono_harness::sha256(&raw)});
    assert_eq!(declaration["schema"], "chrono-retention-consumer/v1");
    assert_eq!(declaration["originals"][0]["path"], notice["path"]);
    assert!(f.root.join(notice["path"].as_str().unwrap()).is_dir());
    let receipt = serde_json::to_vec(&json!({"consumer":declaration["consumer"],
        "declaration_sha256":reference["sha256"], "state":"released"}))
    .unwrap();
    write(
        &f.root.join(".chrono-harness/state/repair-release.json"),
        &receipt,
    );
    let request = json!({"operation":"release-consumer", "declaration":reference,
        "receipt":{"path":".chrono-harness/state/repair-release.json","sha256":chrono_harness::sha256(&receipt)}, "pointer":""});
    let request_path = ".chrono-harness/state/repair-request.json";
    write(
        &f.root.join(request_path),
        &serde_json::to_vec(&request).unwrap(),
    );
    let root = f.root.to_str().unwrap();
    let released = chrono_harness::retained_artifacts::retention_command::command(&[
        "--host-root",
        root,
        "--request",
        request_path,
    ]);
    assert_eq!(released.exit_code, 0, "{}", released.stderr);
    let release: Value = serde_json::from_str(&released.stdout).unwrap();
    assert_eq!(release["consumer"], declaration["consumer"]);
    assert_eq!(release["original_count"], 1);
    assert_eq!(release["effects"], "consumer-root-release-only");
    assert_eq!(release["completion"], "not-established");
    write(
        &f.root.join(request_path),
        &serde_json::to_vec(
            &json!({"operation":"maintain", "originals":[declaration["originals"][0]["id"]]}),
        )
        .unwrap(),
    );
    let maintained = chrono_harness::retained_artifacts::retention_command::command(&[
        "--host-root",
        root,
        "--request",
        request_path,
    ]);
    assert_eq!(maintained.exit_code, 0, "{}", maintained.stderr);
    let maintenance: Value = serde_json::from_str(&maintained.stdout).unwrap();
    assert_eq!(maintenance["accounting_errors"], json!([]));
    // 25 ms 也可能在访问对象前用完；释放不承诺同轮开始或完成删除。
    if f.root.join(notice["path"].as_str().unwrap()).exists() {
        if maintenance["nodes"] == 0 && maintenance["protected"] == json!([]) {
            assert_eq!(maintenance["remaining_outputs"], 1);
            assert_eq!(maintenance["disposed"], json!([]));
        } else {
            assert_eq!(
                maintenance["protected"],
                json!([{"id":declaration["originals"][0]["id"],
                    "reason":"bounded partial disposal; resumes on ordinary entry"}]),
                "release: {}; maintain: {}",
                released.stdout,
                maintained.stdout
            );
        }
    } else {
        assert_eq!(maintenance["remaining_outputs"], 0);
    }
}

#[test]
fn missing_or_corrupt_fresh_dependency_metadata_recovers_once() {
    for corrupt in [false, true] {
        let f = Fixture::new();
        let metadata = f.root.join("app/target/debug/deps/identity-01.d");
        let original = fs::read(&metadata).unwrap();
        if corrupt {
            write(&metadata, b"invalid dependency original");
        } else {
            fs::remove_file(&metadata).unwrap();
        }
        f.recovery_plan(0, false);
        let plan = f.root.join("mock.json");
        let mut invocations: Value = serde_json::from_slice(&fs::read(&plan).unwrap()).unwrap();
        invocations[2]["writes"] = json!([[metadata, original]]);
        write(&plan, &serde_json::to_vec(&invocations).unwrap());
        let mut errors = Vec::new();
        proved_build::build(
            &f.root,
            ".chrono-harness/ci/build.json",
            &f.environment,
            &mut Vec::new(),
            &mut errors,
        )
        .unwrap();
        if corrupt {
            let start = errors.iter().position(|byte| *byte == b'{').unwrap();
            let notice: Value = serde_json::from_slice(&errors[start..]).unwrap();
            assert_eq!(
                fs::read(
                    f.root
                        .join(notice["path"].as_str().unwrap())
                        .join("rejected-dependency.d")
                )
                .unwrap(),
                b"invalid dependency original",
                "the damaged original survives registered cleanup and replacement"
            );
        }
        assert_eq!(fs::read_to_string(f.root.join("mock.calls")).unwrap(), "3");
    }
}

#[test]
fn an_original_that_cannot_be_retained_stops_before_cache_cleanup() {
    for path in [
        "app/target/.chrono-verus-proof.json",
        "app/target/debug/deps/identity-01.d",
    ] {
        let f = Fixture::new();
        let original = f.root.join(path);
        write(&original, &vec![b'x'; 8_388_609]);
        f.recovery_plan(0, false);
        assert!(f.run().is_err());
        assert_eq!(fs::read_to_string(f.root.join("mock.calls")).unwrap(), "1");
        assert_eq!(fs::metadata(original).unwrap().len(), 8_388_609);
    }
}

#[test]
fn invalid_original_records_cannot_trigger_cache_recovery() {
    for variant in 0..3 {
        let f = Fixture::new();
        let mut records = f.records(true);
        match variant {
            0 => {
                records.push(json!({"reason":"build-finished", "success":true}));
            }
            1 => {
                records.insert(
                    0,
                    json!({"reason":"compiler-message", "message":{"level":"error"}}),
                );
            }
            _ => {
                records.pop();
            }
        }
        f.recovery_plan(0, false);
        let plan = f.root.join("mock.json");
        let mut invocations: Value = serde_json::from_slice(&fs::read(&plan).unwrap()).unwrap();
        let mut output = Vec::new();
        for record in records {
            serde_json::to_writer(&mut output, &record).unwrap();
            output.push(b'\n');
        }
        invocations[0]["stdout"] = json!(output);
        write(&plan, &serde_json::to_vec(&invocations).unwrap());
        assert!(f.run().is_err());
        assert_eq!(fs::read_to_string(f.root.join("mock.calls")).unwrap(), "1");
    }
}
