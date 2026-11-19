#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
#[path = "support/recipe.rs"]
mod fixture;
#[cfg(not(verus_keep_ghost))]
use chrono_host_release::{recipe::Loaded, staging::Staging};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    os::unix::{
        fs::{MetadataExt, PermissionsExt, symlink},
        process::ExitStatusExt,
    },
    path::Path,
    process::{Command, Output},
};

#[path = "support/failure_case.rs"]
mod failure_case;

#[cfg(not(verus_keep_ghost))]
#[test]
fn release_case_retains_a_failed_batch_until_its_creation_deadline() {
    use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
    use std::time::{SystemTime, UNIX_EPOCH};

    let host = tempfile::tempdir().unwrap();
    let root = host.path().canonicalize().unwrap();
    let policy = root.join(POLICY_PATH);
    fs::create_dir_all(policy.parent().unwrap()).unwrap();
    fs::write(
        &policy,
        serde_json::to_vec(&json!({
            "schema":"chrono-output-retention/v2",
            "producers":{"check-original":[".chrono-harness/state/release-test-failures/"]},
            "max_lifetime_seconds":3600, "keep_seconds":0, "diagnostic_keep_seconds":86400,
            "metadata_keep_seconds":0, "max_related_outputs":32, "max_summaries":16,
            "max_nodes_per_round":256, "max_bytes_per_round":1048576, "max_millis_per_round":100
        }))
        .unwrap(),
    )
    .unwrap();
    let owner = Inventory::adopted(&root).unwrap().unwrap();
    let compiled = root.join("compiled-input");
    fs::write(&compiled, b"shared executable input").unwrap();
    let interrupted = ".chrono-harness/state/release-test-failures/interrupted";
    fs::create_dir_all(root.join(interrupted).parent().unwrap()).unwrap();
    let publication = Inventory::adopted(&root)
        .unwrap()
        .unwrap()
        .create_batch("check-original", interrupted, json!({"fixture":"started"}))
        .unwrap();
    fs::hard_link(&compiled, root.join(interrupted).join("probe")).unwrap();
    drop(publication);
    let mut failed_path = None;
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let case = failure_case::FailureCase::allocate(&root, "failed-").unwrap();
        let path = case.path().to_owned();
        fs::write(path.join("stderr"), b"original failure").unwrap();
        fs::hard_link(&compiled, path.join("git")).unwrap();
        failed_path = Some(path);
        panic!("release verification failed");
    }));
    assert!(failure.is_err());
    let failed_path = failed_path.unwrap();
    let observed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    owner.maintain_at(observed + 1).unwrap();
    assert_eq!(
        fs::read(failed_path.join("stderr")).unwrap(),
        b"original failure"
    );
    let case = failure_case::FailureCase::allocate(&root, "passed-").unwrap();
    let success = case.path().to_owned();
    fs::write(success.join("stdout"), b"passed").unwrap();
    fs::hard_link(&compiled, success.join("git")).unwrap();
    assert_eq!(
        fs::read(root.join(interrupted).join("probe")).unwrap(),
        b"shared executable input"
    );
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    for _ in 0..8 {
        owner.maintain_at(now + 3601).unwrap();
        if !failed_path.exists() {
            break;
        }
    }
    assert!(
        success.is_dir(),
        "running verification holds the batch lease"
    );
    drop(case);
    for _ in 0..8 {
        owner.maintain_at(now + 3601).unwrap();
        if !success.exists() {
            break;
        }
    }
    assert!(!success.exists());
    assert!(!failed_path.exists());
    assert!(!root.join(interrupted).exists());
    assert_eq!(fs::read(compiled).unwrap(), b"shared executable input");
}

#[cfg(not(verus_keep_ghost))]
fn cache_process(
    root: &Path,
    index: &mut usize,
    program: &Path,
    args: &[&str],
    steps: Option<&Value>,
) -> Output {
    let mut command = Command::new(program);
    command.args(args).current_dir(root);
    if let Some(steps) = steps {
        command.env("CHRONO_RELEASE_REGRESSION_STEPS", steps.to_string());
    }
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    let output = command.output().unwrap();
    let directory = root.join("original-processes");
    fs::create_dir_all(&directory).unwrap();
    let streams: Vec<_> = [("stdout", &output.stdout), ("stderr", &output.stderr)]
        .into_iter()
        .map(|(name, bytes)| {
            let path = format!("{index}.{name}");
            fs::write(directory.join(&path), bytes).unwrap();
            json!({"path":path,"size":bytes.len(),"sha256":chrono_harness::sha256(bytes)})
        })
        .collect();
    fixture::write(
        &directory,
        &format!("{index}.json"),
        &json!({
            "program":program,"argv":args,"cwd":root,"exit_code":output.status.code(),
            "signal":output.status.signal(),"stdout":streams[0],"stderr":streams[1]
        }),
    );
    *index += 1;
    assert!(
        output.status.success(),
        "{} {args:?}: {:?}: {}",
        program.display(),
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn actual_cache_plan_recover_staging_and_consumer_preserve_the_outer_transport_inode() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let case = failure_case::FailureCase::new("rust-release-cache-");
    let root = case.path();
    let read = |relative: &str| -> Value {
        serde_json::from_slice(&fs::read(repository.join(relative)).unwrap()).unwrap()
    };
    let config_path = ".chrono-harness/release/build.json";
    let config = read(config_path);
    let plan_path = config["consumer_staging"]["release_plan"].as_str().unwrap();
    let host_path = config["consumer_staging"]["host_config"].as_str().unwrap();
    let projects_path = config["projects"].as_str().unwrap();
    let plan = read(plan_path);
    for (relative, value) in [
        (config_path, &config),
        (plan_path, &plan),
        (host_path, &read(host_path)),
        (projects_path, &read(projects_path)),
    ] {
        fixture::write(root, relative, value);
    }
    let bootstrap = read(".chrono-harness/ci/bootstrap-cache.json");
    let asset = plan["assets"]["chrono-cache"].as_str().unwrap();
    let source = repository.join(bootstrap["install"][0]["source"].as_str().unwrap());
    assert!(
        source.is_file(),
        "build.cache must produce the registered real cache CLI before this test: {}",
        source.display()
    );
    fs::create_dir_all(root.join(asset).parent().unwrap()).unwrap();
    fs::copy(source, root.join(asset)).unwrap();
    let transport = root.join(bootstrap["install"][0]["destination"].as_str().unwrap());
    fs::create_dir_all(transport.parent().unwrap()).unwrap();
    fs::copy(root.join(asset), &transport).unwrap();
    let identity = |path: &Path| {
        let metadata = fs::metadata(path).unwrap();
        (
            metadata.dev(),
            metadata.ino(),
            chrono_harness::file_identity(path).unwrap().0,
        )
    };
    let original = identity(&transport);
    fixture::write(
        root,
        ".chrono-harness/cache.json",
        &json!({
            "schema":"chrono-cache/v2","namespace":"release-regression","artifact_registry":host_path,
            "inputs":{"profile":{"kind":"literal","value":"fixture"},"source":{"kind":"literal","value":"fixed-source"}},
            "artifacts":{"target":{"owner":"cache-tests","path":"crates/cache-tests/target/"}},
            "caches":{"tests":{"owner":"cache-tests","producer":"test.cache-tests","consumers":["release.verify"],
                "artifacts":["target"],"compatibility_inputs":["profile"],"source_inputs":["source"],"restore":"compatible"}},
            "consumer_operations":{"release.verify":[{"registry":config_path,"pointer":"/verification_operations"}]},
            "require_primary_checkout":true,"recover_failed_restores":true
        }),
    );
    let mut index = 0;
    cache_process(
        root,
        &mut index,
        Path::new(env!("CARGO_BIN_EXE_chrono-test-release-git")),
        &["init", "-q", "."],
        None,
    );
    let root_text = root.to_str().unwrap();
    let args = [
        "plan",
        "--host-root",
        root_text,
        "--config",
        ".chrono-harness/cache.json",
        "--consumer",
        "release.verify",
    ];
    let planned = cache_process(root, &mut index, &transport, &args, None);
    let prepared: Value = serde_json::from_slice(&planned.stdout).unwrap();
    fixture::write(root, ".chrono-harness/state/cache/plan.json", &prepared);
    let steps = json!({"cache_0_restore":{"outcome":"success","outputs":{}},"cache_0_save":{"outcome":"skipped","outputs":{}},"work":{"outcome":"success"}});
    let recovered = cache_process(
        root,
        &mut index,
        &transport,
        &[
            "recover",
            "--host-root",
            root_text,
            "--config",
            ".chrono-harness/cache.json",
            "--plan",
            ".chrono-harness/state/cache/plan.json",
            "--steps-env",
            "CHRONO_RELEASE_REGRESSION_STEPS",
        ],
        Some(&steps),
    );
    let recovered: Value = serde_json::from_slice(&recovered.stdout).unwrap();
    let reference = &recovered["report"];
    let raw = fs::read(root.join(reference["path"].as_str().unwrap())).unwrap();
    assert_eq!(reference["sha256"], chrono_harness::sha256(&raw));
    let original_recovery: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(original_recovery, recovered["observation"]);
    // 未声明命中且无失败恢复输出，本次应发布“不需恢复”的真实原件。
    assert_eq!(original_recovery["status"], "not-required");
    assert_eq!(
        original_recovery["caches"]["tests"]["reason"],
        "miss-or-unavailable"
    );
    let loaded = Loaded::load(root).unwrap();
    let observed = chrono_host_release::transport::observe(root, asset).unwrap();
    let mut staging = Staging::selected(
        &loaded,
        &json!({"chrono-cache":{"identity":observed.receipt(asset)}}),
    )
    .unwrap();
    assert_eq!(
        staging.destinations(),
        ["crates/cache/target/debug/chrono-cache"]
    );
    staging.copy(root).unwrap();
    assert!(staging.observe(root, "staged", None));
    let consumer = root.join("crates/cache/target/debug/chrono-cache");
    let consumed = cache_process(root, &mut index, &consumer, &args, None);
    let consumed: Value = serde_json::from_slice(&consumed.stdout).unwrap();
    assert_eq!(consumed["planner"]["sha256"], observed.sha256);
    let replanned = cache_process(root, &mut index, &transport, &args, None);
    let replanned: Value = serde_json::from_slice(&replanned.stdout).unwrap();
    assert_eq!(replanned["planner"], prepared["planner"]);
    assert_eq!(
        identity(&transport),
        original,
        "release staging replaced the outer cache transport"
    );
    assert!(staging.observe(root, "after-verification", None));
    fixture::write(root, "staging.json", &staging.receipt());
}

#[cfg(not(verus_keep_ghost))]
fn source(f: &fixture::Fixture) -> Value {
    let path = f.root().join("out/a");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"the selected program").unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o751)).unwrap();
    json!({"a":{"identity":{"path":"out/a","sha256":chrono_harness::sha256(b"the selected program"),"size":20,"mode":489,"error":null}}})
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn only_selected_consumers_receive_exact_bytes_mode_and_three_real_observations() {
    let f = fixture::Fixture::new();
    let assets = source(&f);
    let loaded = Loaded::load(f.root()).unwrap();
    let mut staging = Staging::selected(&loaded, &assets).unwrap();
    assert_eq!(staging.destinations(), ["consumer/a"]);
    staging.copy(f.root()).unwrap();
    let file = f.root().join("consumer/a");
    assert_eq!(fs::read(&file).unwrap(), b"the selected program");
    assert_eq!(
        fs::metadata(file).unwrap().permissions().mode() & 0o7777,
        0o751
    );
    assert!(!f.root().join("out/b").exists());
    assert!(!f.root().join("consumer/b").exists());
    for (phase, operation) in [
        ("staged", None),
        ("before-verification", Some("test.a")),
        ("after-verification", Some("test.a")),
    ] {
        assert!(staging.observe(f.root(), phase, operation));
    }
    let receipt = staging.receipt();
    let observations = receipt["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 3);
    for observation in observations {
        assert_eq!(observation["matches"], true);
        assert_eq!(observation["files"].as_array().unwrap().len(), 2);
    }
    assert_eq!(receipt["bindings"][0]["expected"], assets["a"]["identity"]);
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn source_or_destination_byte_and_mode_drift_records_the_actual_failure() {
    for (path, mode) in [
        ("out/a", false),
        ("out/a", true),
        ("consumer/a", false),
        ("consumer/a", true),
    ] {
        let f = fixture::Fixture::new();
        let assets = source(&f);
        let loaded = Loaded::load(f.root()).unwrap();
        let mut staging = Staging::selected(&loaded, &assets).unwrap();
        staging.copy(f.root()).unwrap();
        assert!(staging.observe(f.root(), "staged", None));
        if mode {
            fs::set_permissions(f.root().join(path), fs::Permissions::from_mode(0o755)).unwrap();
        } else {
            fs::write(f.root().join(path), b"changed program").unwrap();
        }
        assert!(
            !staging.observe(f.root(), "after-verification", Some("test.a")),
            "{path} mode={mode}"
        );
        let receipt = staging.receipt();
        let file = receipt["observations"][1]["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["path"] == path)
            .unwrap();
        assert_eq!(file["error"], Value::Null);
        if mode {
            assert_eq!(file["mode"], 0o755);
        } else {
            assert_eq!(file["sha256"], chrono_harness::sha256(b"changed program"));
        }
        assert_eq!(receipt["bindings"][0]["expected"], assets["a"]["identity"]);
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn source_drift_or_symlink_never_replaces_the_existing_consumer() {
    for mutation in 0..3 {
        let f = fixture::Fixture::new();
        let assets = source(&f);
        let loaded = Loaded::load(f.root()).unwrap();
        let mut staging = Staging::selected(&loaded, &assets).unwrap();
        fs::create_dir(f.root().join("consumer")).unwrap();
        let destination = f.root().join("consumer/a");
        fs::write(&destination, b"existing program").unwrap();
        match mutation {
            0 => fs::write(f.root().join("out/a"), b"changed source").unwrap(),
            1 => {
                let source = f.root().join("out/a");
                fs::remove_file(&source).unwrap();
                symlink(&destination, source).unwrap();
            }
            2 => {
                fs::remove_file(&destination).unwrap();
                symlink(f.root().join("out/a"), &destination).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(staging.copy(f.root()).is_err());
        assert!(!staging.observe(f.root(), "staged", None));
        if mutation < 2 {
            assert_eq!(fs::read(destination).unwrap(), b"existing program");
        } else {
            assert!(
                fs::symlink_metadata(destination)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        }
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn empty_selection_has_empty_successful_observations_and_invalid_asset_vectors_fail() {
    let f = fixture::Fixture::new();
    let loaded = Loaded::load(f.root()).unwrap();
    let mut staging = Staging::selected(&loaded, &json!({})).unwrap();
    assert!(staging.destinations().is_empty());
    staging.copy(f.root()).unwrap();
    assert!(staging.observe(f.root(), "staged", None));
    assert_eq!(staging.receipt()["observations"][0]["files"], json!([]));
    for value in [
        json!({"unknown":{}}),
        json!({"a":{"identity":{}}}),
        json!([]),
    ] {
        assert!(Staging::selected(&loaded, &value).is_err());
    }
}
