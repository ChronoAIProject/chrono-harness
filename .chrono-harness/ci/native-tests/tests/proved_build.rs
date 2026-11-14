//! 正式生产者的命令边界验收；外部编译器全部由 Rust mock 提供。
#[path = "../../../../crates/runner-tests/tests/support/isolated_capability.rs"]
mod isolated_capability;
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
            "--jobs".into(),
            "1".into(),
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
fn unrelated_cached_outputs_do_not_consume_the_binary_dependency_bound() {
    let f = Fixture::new();
    let deps = f.root.join("app/target/debug/deps");
    for index in 0..4097 {
        write(&deps.join(format!("unrelated-{index}.d")), b"unrelated");
    }
    f.plan(&f.records(false), 0, json!([]));
    f.run().unwrap();
    let original = fs::read(f.proof()).unwrap();
    f.plan(&f.records(true), 0, json!([]));
    f.run().unwrap();
    assert_eq!(fs::read(f.proof()).unwrap(), original);
    assert_eq!(fs::read_to_string(f.root.join("mock.calls")).unwrap(), "1");
    assert_eq!(
        fs::read(deps.join("unrelated-4096.d")).unwrap(),
        b"unrelated"
    );
}

#[test]
fn selected_binary_dependency_candidates_keep_the_original_bound() {
    let f = Fixture::new();
    let deps = f.root.join("app/target/debug/deps");
    fs::remove_file(deps.join("app-01.d")).unwrap();
    for index in 0..4097 {
        write(&deps.join(format!("app-{index:016x}.d")), b"candidate");
    }
    f.plan(&f.records(false), 0, json!([]));
    let error = f.run().unwrap_err();
    assert!(error.contains("compiler output inventory exceeds its registered bound"));
    assert!(!f.proof().exists());
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
    if !isolated_capability::isolated_capability_test(
        "retained_recovery_originals_publish_a_consumer_that_can_be_released",
    ) {
        return;
    }
    let mut f = Fixture::new();
    f._temp.disable_cleanup(true);
    eprintln!(
        "recovery consumer fixture (retained on failure): {}",
        f.root.display()
    );
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
    assert_eq!(release["disposal"], "not-attempted");
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
    let id = declaration["originals"][0]["id"].as_str().unwrap();
    let busy = json!({"id":id,"reason":format!("retention output metadata is busy: {id}")});
    if maintenance["accounting_errors"] == json!([busy.clone()]) {
        // 非阻塞维护可拒绝这个明确对象；拒绝时必须保留原件和未知计数。
        assert_eq!(maintenance["protected"], json!([busy]));
        assert_eq!(maintenance["scope"]["complete"], false);
        assert_eq!(maintenance["remaining_outputs"], Value::Null);
        assert_eq!(maintenance["disposed"], json!([]));
        assert!(f.root.join(notice["path"].as_str().unwrap()).is_dir());
        f._temp.disable_cleanup(false);
        return;
    }
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
    f._temp.disable_cleanup(false);
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

// Cargo JSON 外部接口样本独立成文件。它是测试数据，不是成功生产回执。
fn dependency_fixture() -> (Fixture, Value, Vec<Value>) {
    let fixture = Fixture::new();
    let encoded = include_str!("fixtures/cargo-dependencies.json")
        .replace("$ROOT", fixture.root.to_str().unwrap());
    let sample: Value = serde_json::from_str(&encoded).unwrap();
    let deps = fixture.root.join("app/target/debug/deps");
    let outputs = vec![deps.join("libapp-02.rlib"), deps.join("libapp-02.rmeta")];
    for output in &outputs {
        write(output, b"ordinary Cargo library, not proof");
    }
    dependency(
        &deps.join("app-02.d"),
        &outputs,
        &[
            fixture.root.join("app/src/main.rs"),
            deps.join("libidentity-01.rlib"),
        ],
    );
    (
        fixture,
        sample["metadata"].clone(),
        sample["records"].as_array().unwrap().clone(),
    )
}

fn dependency_output(records: &[Value]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for record in records {
        serde_json::to_writer(&mut bytes, record).unwrap();
        bytes.push(b'\n');
    }
    bytes
}

fn collect_dependencies(
    f: &Fixture,
    graph: &Value,
    records: &[Value],
) -> Result<
    (
        Vec<chrono_harness::verified_verifier::cargo::Identity>,
        std::collections::BTreeMap<String, String>,
        Vec<String>,
        Vec<String>,
    ),
    String,
> {
    proved_build::dependencies::collect(
        &f.root.join("app/target"),
        graph,
        &dependency_output(records),
        &f.root.join("app/Cargo.toml"),
    )
}

#[test]
fn dependencies_select_the_actual_root_loaded_instance_and_keep_sources() {
    let (f, graph, records) = dependency_fixture();
    let (files, externs, search, unresolved) = collect_dependencies(&f, &graph, &records).unwrap();
    assert_eq!(externs.len(), 1);
    assert_eq!(
        externs["identity"],
        f.root
            .join("app/target/debug/deps/libidentity-01.rlib")
            .to_str()
            .unwrap()
    );
    assert_eq!(
        search,
        vec![
            f.root
                .join("app/target/debug/deps")
                .to_str()
                .unwrap()
                .to_owned()
        ]
    );
    for path in [
        "library/Cargo.toml",
        "library/src/lib.rs",
        "app/src/main.rs",
        "app/target/debug/deps/identity-01.d",
        "app/target/debug/deps/app-02.d",
        "app/target/debug/deps/libidentity-01.rlib",
        "app/target/debug/deps/libapp-02.rlib",
    ] {
        assert!(
            files
                .iter()
                .any(|i| Path::new(&i.path) == f.root.join(path)),
            "missing {path}"
        );
    }
    assert!(
        unresolved.iter().any(|v| v.contains("compiler backend")),
        "ordinary compile cannot qualify the SDK/compiler domain"
    );
}

#[test]
fn dependency_stream_rejects_missing_root_terminal_duplicate_and_trailing_records() {
    let (f, graph, records) = dependency_fixture();
    let mut cases = Vec::new();
    cases.push(records[..2].to_vec());
    cases.push(vec![records[0].clone(), records[2].clone()]);
    let mut duplicate = records.clone();
    duplicate.insert(2, records[1].clone());
    cases.push(duplicate);
    let mut trailing = records.clone();
    trailing.push(records[0].clone());
    cases.push(trailing);
    let mut failed = records.clone();
    failed[2]["success"] = json!(false);
    cases.push(failed);
    for records in cases {
        assert!(collect_dependencies(&f, &graph, &records).is_err());
    }
    let mut raw = dependency_output(&records);
    raw.extend_from_slice(b"{broken");
    assert!(
        proved_build::dependencies::collect(
            &f.root.join("app/target"),
            &graph,
            &raw,
            &f.root.join("app/Cargo.toml")
        )
        .is_err()
    );
}

#[test]
fn missing_changed_wrong_domain_and_ambiguous_producers_are_rejected() {
    let (f, graph, records) = dependency_fixture();
    let mut no_producer = records.clone();
    no_producer.remove(0);
    assert!(
        collect_dependencies(&f, &graph, &no_producer)
            .err()
            .unwrap()
            .contains("actual loaded producers")
    );
    let mut duplicate = records.clone();
    duplicate.insert(1, records[0].clone());
    assert!(
        collect_dependencies(&f, &graph, &duplicate)
            .err()
            .unwrap()
            .contains("2 actual loaded producers")
    );
    assert!(
        proved_build::dependencies::collect(
            &f.root.join("other-target"),
            &graph,
            &dependency_output(&records),
            &f.root.join("app/Cargo.toml")
        )
        .err()
        .unwrap()
        .contains("outside runner target")
    );
    write(
        &f.root.join("library/src/lib.rs"),
        b"changed after compiler checksum",
    );
    assert!(collect_dependencies(&f, &graph, &records).is_err());
    fs::remove_file(f.root.join("app/target/debug/deps/libidentity-01.rlib")).unwrap();
    assert!(collect_dependencies(&f, &graph, &records).is_err());
}

#[test]
fn fresh_and_native_outputs_are_observations_with_unresolved_origins() {
    let (f, graph, mut records) = dependency_fixture();
    records[0]["fresh"] = json!(true);
    let archive = f.root.join("app/target/debug/build/native/out/libnative.a");
    write(&archive, b"native compiler output");
    records.insert(2, json!({"reason":"build-script-executed", "package_id":"identity@0.1.0",
        "linked_libs":["static=native"], "linked_paths":[format!("native={}",archive.parent().unwrap().display())],
        "cfgs":[], "env":[], "out_dir":archive.parent().unwrap()}));
    let (files, _, _, unresolved) = collect_dependencies(&f, &graph, &records).unwrap();
    assert!(files.iter().any(|i| Path::new(&i.path) == archive));
    assert!(unresolved.iter().any(|v| v.contains("fresh unit")));
    assert!(
        unresolved
            .iter()
            .any(|v| v.contains("native tool invocation"))
    );
    fs::remove_file(&archive).unwrap();
    assert!(
        collect_dependencies(&f, &graph, &records)
            .err()
            .unwrap()
            .contains("0 declared producers")
    );
}

#[test]
fn consumer_resolves_names_without_changing_strict_or_import_export_arguments() {
    use std::collections::BTreeMap;
    let argv: Vec<String> = [
        "--no-cheating",
        "--no-external-by-default",
        "--compile",
        "--num-threads",
        "2",
        "--extern",
        "serde_json",
        "--import",
        "runner=current.vir",
        "--export",
        "out.vir",
    ]
    .iter()
    .map(|s| (*s).into())
    .collect();
    let selected = BTreeMap::from([(
        "serde_json".into(),
        "/runner/target/libserde_json-new.rlib".into(),
    )]);
    let result =
        proved_build::dependencies::arguments(&argv, &selected, &["/runner/target/deps".into()])
            .unwrap();
    assert_eq!(&result[..6], &argv[..6]);
    assert_eq!(
        result[6],
        "serde_json=/runner/target/libserde_json-new.rlib"
    );
    assert_eq!(&result[7..11], &argv[7..11]);
    assert_eq!(&result[11..], ["-L", "dependency=/runner/target/deps"]);
    assert!(proved_build::dependencies::arguments(&argv, &BTreeMap::new(), &[]).is_err());
    assert!(proved_build::dependencies::arguments(&["--extern".into()], &selected, &[]).is_err());
}

#[test]
fn component_requires_the_original_compiled_instance_and_its_binary_binding() {
    use chrono_harness::verified_verifier::{OutputContract, cargo::Identity};
    use chrono_host_bootstrap::execution::{Operation, Stream};
    use std::collections::BTreeMap;
    let (f, _, _) = dependency_fixture();
    let directory = f.root.join(".chrono-harness/state/component");
    let current = f.root.join("app/target/debug/deps/libidentity-01.rlib");
    let source = f.root.join("library/src/lib.rs");
    let library = directory.join("libvstd.rlib");
    let dep = directory.join("libvstd.rlib.d");
    write(&library, b"new component compiled with current identity");
    let json_current = f.root.join("app/target/debug/deps/libserde_json-01.rlib");
    write(&json_current, b"current serde_json");
    let externs = BTreeMap::from([("serde_json".into(), json_current.to_str().unwrap().into())]);
    let stream = |name: &str, bytes: &[u8]| {
        let path = directory.join(name);
        write(&path, bytes);
        Stream {
            path: path.strip_prefix(&f.root).unwrap().to_str().unwrap().into(),
            sha256: chrono_harness::sha256(bytes),
            size: bytes.len() as u64,
        }
    };
    let report = json!({"verus":{"version":"v","commit":"c"},"verification-results":{"encountered-error":false,"encountered-vir-error":false,"success":true,"verified":1,"errors":0,"is-verifying-entire-crate":true}});
    let operation = Operation {
        operation: "component".into(),
        tool: "verus".into(),
        argv: vec![
            "verus".into(),
            "--extern".into(),
            format!("serde_json={}", json_current.display()),
        ],
        cwd: f.root.to_str().unwrap().into(),
        executable_sha256: "unused".into(),
        pid: 1,
        exit_code: 0,
        stdout: stream("stdout", &serde_json::to_vec(&report).unwrap()),
        stderr: stream("stderr", b""),
        duration_seconds: 0.0,
        reused_from: None,
    };
    let mut component = proved_build::dependencies::Component {
        inputs: "current".into(),
        directory: directory
            .strip_prefix(&f.root)
            .unwrap()
            .to_str()
            .unwrap()
            .into(),
        operation,
        files: vec![],
        externs: externs.clone(),
    };
    let contract = OutputContract {
        version: "v".into(),
        commit: "c".into(),
    };
    dependency(
        &dep,
        std::slice::from_ref(&library),
        &[source.clone(), json_current.clone()],
    );
    proved_build::dependencies::validate_component(
        &f.root, &component, &externs, &source, &contract,
    )
    .unwrap();
    dependency(
        &dep,
        std::slice::from_ref(&library),
        std::slice::from_ref(&source),
    );
    assert!(
        proved_build::dependencies::validate_component(
            &f.root, &component, &externs, &source, &contract
        )
        .is_err(),
        "equal versions and current bytes cannot replace the binary input binding"
    );
    let different = BTreeMap::from([("serde_json".into(), current.to_str().unwrap().into())]);
    assert!(
        proved_build::dependencies::validate_component(
            &f.root, &component, &different, &source, &contract
        )
        .err()
        .unwrap()
        .contains("compiled instances differ")
    );
    let (sha256, length) = chrono_harness::file_identity(&json_current).unwrap();
    component.files.push(Identity {
        path: json_current.to_str().unwrap().into(),
        sha256,
        length,
    });
    write(&json_current, b"changed compiler input");
    assert!(
        proved_build::dependencies::validate_component(
            &f.root, &component, &externs, &source, &contract
        )
        .err()
        .unwrap()
        .contains("bytes changed")
    );
    component.files.clear();
    component.operation.exit_code = 1;
    assert!(
        proved_build::dependencies::validate_component(
            &f.root, &component, &externs, &source, &contract
        )
        .is_err()
    );
}

#[test]
fn root_metadata_input_selects_only_the_rlib_from_its_same_cargo_artifact() {
    let (f, graph, records) = dependency_fixture();
    let deps = f.root.join("app/target/debug/deps");
    let outputs = [deps.join("libapp-02.rlib"), deps.join("libapp-02.rmeta")];
    dependency(
        &deps.join("app-02.d"),
        &outputs,
        &[
            f.root.join("app/src/main.rs"),
            deps.join("libidentity-01.rmeta"),
        ],
    );
    let (_, externs, _, _) = collect_dependencies(&f, &graph, &records).unwrap();
    assert_eq!(
        externs["identity"],
        deps.join("libidentity-01.rlib").to_str().unwrap()
    );
    let mut wrong_pair = records.clone();
    wrong_pair[0]["filenames"][1] = json!(deps.join("libidentity-02.rmeta"));
    write(&deps.join("libidentity-02.rmeta"), b"another compilation");
    assert!(collect_dependencies(&f, &graph, &wrong_pair).is_err());
}

#[test]
fn cargo_published_root_alias_keeps_the_actual_compiler_paths() {
    let (f, graph, mut records) = dependency_fixture();
    let hashed = f.root.join("app/target/debug/deps/libapp-02.rlib");
    let published = f.root.join("app/target/debug/libapp.rlib");
    write(&published, &fs::read(&hashed).unwrap());
    records[1]["filenames"][0] = json!(published);
    let (files, externs, _, _) = collect_dependencies(&f, &graph, &records).unwrap();
    assert!(files.iter().any(|i| Path::new(&i.path) == hashed));
    assert!(externs.contains_key("identity"));
    write(&published, b"different alias bytes");
    assert!(
        collect_dependencies(&f, &graph, &records)
            .err()
            .unwrap()
            .contains("published root differs")
    );
}

#[test]
fn strict_consumer_command_boundary_preserves_exact_arguments_streams_and_exit() {
    use std::collections::BTreeMap;
    let mut f = Fixture::new();
    f.environment
        .insert("RUSTUP_TOOLCHAIN".into(), "1.95.0".into());
    f.environment.insert("RUSTC_BOOTSTRAP".into(), "1".into());
    let executable = ".chrono-harness/state/mock-component/verus";
    let destination = f.root.join(executable);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_chrono-test-proved-cargo"), &destination).unwrap();
    let consumer = proved_build::dependencies::Consumer {
        tool: "verus-json-contract".into(),
        executable: executable.into(),
        argv: vec![
            "--no-cheating".into(),
            "--no-external-by-default".into(),
            "--compile".into(),
            "--extern".into(),
            "identity".into(),
        ],
    };
    let externs = BTreeMap::from([(
        "identity".into(),
        "/runner/target/libidentity-current.rlib".into(),
    )]);
    let expected = vec![
        "--no-cheating",
        "--no-external-by-default",
        "--compile",
        "--extern",
        "identity=/runner/target/libidentity-current.rlib",
        "-L",
        "dependency=/runner/target/deps",
    ];
    for exit in [0u8, 1u8] {
        let directory = f
            .root
            .join(format!(".chrono-harness/state/consumer-{exit}"));
        fs::create_dir_all(&directory).unwrap();
        write(&f.root.join("mock.json"), &serde_json::to_vec(&json!([{"argv":expected,
            "stdout":b"original compiler stdout".to_vec(), "stderr":b"original compiler stderr".to_vec(), "writes":[], "exit":exit}])).unwrap());
        let calls = f.root.join("mock.calls");
        if calls.exists() {
            fs::remove_file(calls).unwrap();
        }
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let result = proved_build::dependencies::execute_consumer(
            &f.root,
            &directory,
            "strict.fixture",
            &consumer,
            &externs,
            &["/runner/target/deps".into()],
            &f.environment,
            None,
            "",
            None,
            &mut stdout,
            &mut stderr,
        );
        assert_eq!(stdout, b"original compiler stdout");
        assert_eq!(stderr, b"original compiler stderr");
        if exit == 0 {
            assert!(result.is_ok());
        } else {
            assert_eq!(result.err().unwrap().exit_code, Some(1));
        }
        let original: Value =
            serde_json::from_slice(&fs::read(directory.join("consumer-operation.json")).unwrap())
                .unwrap();
        assert_eq!(original["exit_code"], exit);
        assert_eq!(original["operation"], "strict.fixture");
    }
}

#[test]
fn selected_nested_proof_consumes_the_original_child_even_if_a_parent_would_succeed() {
    use chrono_harness::{
        nested_proof, retained_artifacts::retention, verified_verifier as verifier,
    };
    use std::collections::BTreeMap;
    if !isolated_capability::isolated_capability_test(
        "selected_nested_proof_consumes_the_original_child_even_if_a_parent_would_succeed",
    ) {
        return;
    }
    let mut f = Fixture::new();
    f.environment
        .insert("RUSTUP_TOOLCHAIN".into(), "1.95.0".into());
    f.environment.insert("RUSTC_BOOTSTRAP".into(), "1".into());
    write(&f.root.join(".chrono-harness/retention.json"), &serde_json::to_vec(&json!({
        "schema":"chrono-output-retention/v2", "producers":{"check-original":[".chrono-harness/state/"]},
        "keep_seconds":0, "max_related_outputs":512, "max_summaries":128, "max_nodes_per_round":256,
        "max_bytes_per_round":67108864, "max_millis_per_round":25
    })).unwrap());
    let executable = ".chrono-harness/state/mock-component/verus";
    let destination = f.root.join(executable);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_chrono-test-proved-cargo"), &destination).unwrap();
    let argv: Vec<String> = [
        "--crate-type=lib",
        "--no-cheating",
        "--no-external-by-default",
        "--num-threads",
        "2",
        "--output-json",
        "--compile",
        "--emit=link=out,dep-info=out.d",
        "root",
        "-o",
        "out",
        "-Zchecksum-hash-algorithm=sha256",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let declaration = nested_proof::Declaration {
        tool: "verus-json-contract".into(),
        executable: "$COMPONENT/verus".into(),
        argv: argv.clone(),
        output_contract: Some(verifier::OutputContract {
            version: "v".into(),
            commit: "c".into(),
        }),
        timeout_seconds: 10,
        output_limit_bytes: 1048576,
    };
    let consumer = proved_build::dependencies::Consumer {
        tool: declaration.tool.clone(),
        executable: executable.into(),
        argv,
    };
    for exit in [0u8, 1u8] {
        let directory = f.root.join(format!(".chrono-harness/state/nested-{exit}"));
        fs::create_dir_all(&directory).unwrap();
        write(&f.root.join("mock.json"), &serde_json::to_vec(&json!([{"argv":consumer.argv,
            "stdout":include_bytes!("fixtures/verus-success.json").as_slice(), "stderr":b"original child stderr".as_slice(),
            "writes":[], "exit":exit}])).unwrap());
        let calls = f.root.join("mock.calls");
        if calls.exists() {
            fs::remove_file(calls).unwrap();
        }
        let publication = retention::begin_adopted(
            &f.root,
            "check-original",
            &directory,
            json!({"status":"in-progress"}),
        )
        .unwrap()
        .unwrap();
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let result = proved_build::dependencies::execute_consumer(
            &f.root,
            &directory,
            "strict.fixture",
            &consumer,
            &BTreeMap::new(),
            &[],
            &f.environment,
            Some(&declaration),
            ".chrono-harness/state/mock-component",
            Some(&publication),
            &mut stdout,
            &mut stderr,
        );
        assert_eq!(result.is_ok(), exit == 0);
        assert_eq!(stderr, b"original child stderr");
        let original =
            nested_proof::read_original(&f.root, "strict.fixture", &stdout, 1048576).unwrap();
        original
            .validate(&f.root, "strict.fixture", &declaration)
            .unwrap();
        assert_eq!(original.process.exit_code, exit as i32);
        assert!(original.process.owned_work_joined);
        assert_eq!(
            original.process.stdout_bytes,
            include_bytes!("fixtures/verus-success.json")
        );
        assert!(nested_proof::read_original(&f.root, "wrong.operation", &stdout, 1048576).is_err());
        assert!(nested_proof::read_original(&f.root, "strict.fixture", b"{}", 1048576).is_err());
        let mut duplicate = stdout.clone();
        duplicate.extend_from_slice(&stdout);
        assert!(
            nested_proof::read_original(&f.root, "strict.fixture", &duplicate, 1048576).is_err()
        );
        for mutation in [
            "operation",
            "executable",
            "argv",
            "join",
            "output",
            "source",
            "declaration",
        ] {
            let mut changed = original.clone();
            match mutation {
                "operation" => changed.operation.push('x'),
                "executable" => changed.executable.push('x'),
                "argv" => changed.process.argv[2].push('x'),
                "join" => changed.process.owned_work_joined = false,
                "output" => changed.process.stdout_bytes.push(0),
                "source" => {
                    changed.resolved_argv[8] = "other".into();
                    changed.process.argv[9] = "other".into();
                }
                "declaration" => changed.declaration.tool.push('x'),
                _ => unreachable!(),
            }
            assert!(
                changed
                    .validate(&f.root, "strict.fixture", &declaration)
                    .is_err(),
                "{mutation}"
            );
        }
        if exit != 0 {
            let decoded =
                serde_json::from_slice::<verifier::CompilerReport>(&original.process.stdout_bytes)
                    .ok();
            assert!(
                verifier::admit_observed_output(
                    &declaration.output_contract,
                    &decoded,
                    &original.resolved_argv,
                    original.process.exit_code,
                    false,
                    &original.compiler_dependency,
                    1048576,
                    &original.compiler_artifact
                )
                .is_err()
            );
        }
        let original_path = directory.join("nested-proof-original.json");
        let original_bytes = fs::read(&original_path).unwrap();
        let wire: Value = serde_json::from_slice(&original_bytes).unwrap();
        assert_eq!(wire["process"]["encoding"], "chrono-retained-process/v2");
        assert!(wire["process"].get("stderr_bytes").is_none());
        write(&original_path, b"{}");
        assert!(nested_proof::read_original(&f.root, "strict.fixture", &stdout, 1048576).is_err());
        write(&original_path, &original_bytes);
        publication.complete(json!({"status":"observed"})).unwrap();
        let tree = directory.strip_prefix(&f.root).unwrap().to_str().unwrap();
        let member = original_path
            .strip_prefix(&f.root)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            retention::Inventory::adopted(&f.root)
                .unwrap()
                .unwrap()
                .published_tree_member(
                    "check-original",
                    tree,
                    member,
                    &chrono_harness::sha256(&original_bytes),
                    original_bytes.len() as u64,
                )
                .unwrap()
        );
    }
}

#[test]
fn selected_nested_proof_cannot_substitute_an_explicit_compiler_input() {
    let strict = [
        "--crate-type=lib",
        "--no-cheating",
        "--no-external-by-default",
        "--num-threads",
        "2",
        "--output-json",
        "--compile",
    ];
    for option in ["--extern", "--import"] {
        let mut declared: Vec<String> = strict.into_iter().map(str::to_owned).collect();
        declared.extend(["--extern".into(), "producer=expected".into()]);
        if option == "--import" {
            declared.extend(["--import".into(), "producer=expected".into()]);
        }
        let changed = declared.len() - 1;
        declared.extend(
            [
                "--emit=link=out,dep-info=out.d",
                "root",
                "-o",
                "out",
                "-Zchecksum-hash-algorithm=sha256",
            ]
            .map(str::to_owned),
        );
        assert!(
            chrono_harness::verified_verifier::nested::validate_resolution(&declared, &declared)
                .is_ok()
        );
        let mut resolved = declared.clone();
        resolved[changed] = "producer=different".into();
        assert!(
            chrono_harness::verified_verifier::nested::validate_resolution(&declared, &resolved)
                .is_err(),
            "{option}"
        );
    }
}
