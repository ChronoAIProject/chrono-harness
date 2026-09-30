use chrono_harness::facts;
use serde_json::json;
use std::{collections::BTreeMap, fs, path::Path, process::Command};

#[path = "checkout.rs"]
mod checkout;

const CONFIG: &str = ".chrono-harness/config with spaces.json";

fn git(root: &Path, args: &[&str]) -> String {
    let result = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().into()
}

fn commit(root: &Path) -> String {
    git(root, &["add", "--all"]);
    git(root, &["commit", "-qm", "fixture"]);
    git(root, &["rev-parse", "HEAD"])
}

fn fixture() -> (tempfile::TempDir, BTreeMap<String, Vec<u8>>, String) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".chrono-harness")).unwrap();
    let mut paths = BTreeMap::new();
    let mut originals = BTreeMap::new();
    for role in ["judges", "projects", "filemap", "workflow"] {
        let path = format!(".chrono-harness/{role} café.json");
        let bytes = format!("{{  \"role\": \"{role}\", \"n\": 1 }}\n\n").into_bytes();
        fs::write(dir.path().join(&path), &bytes).unwrap();
        originals.insert(path.clone(), bytes);
        paths.insert(role, path);
    }
    let config = format!(
        "{}\n\n",
        serde_json::to_string_pretty(&json!({"registries":paths,"future_field":"retained"}))
            .unwrap()
    )
    .into_bytes();
    fs::write(dir.path().join(CONFIG), &config).unwrap();
    originals.insert(CONFIG.into(), config);
    git(dir.path(), &["init", "-q"]);
    let oid = commit(dir.path());
    (dir, originals, oid)
}

#[test]
fn registry_snapshot_preserves_original_bytes_and_fixed_endpoint_values() {
    let (dir, originals, before) = fixture();
    let root = dir.path();
    let path = ".chrono-harness/projects café.json";
    let changed = b"{\"n\":2,\"role\":\"projects\"}\n";
    fs::write(root.join(path), changed).unwrap();
    let after = commit(root);
    fs::write(root.join(path), b"uncommitted and invalid JSON").unwrap();

    let old = facts::registry_snapshot(root, &before, CONFIG).unwrap();
    assert_eq!(old.bytes, originals);
    assert_eq!(old.values[path]["n"], 1);
    assert_eq!(old.values[CONFIG]["future_field"], "retained");
    for (path, original) in &old.bytes {
        assert_eq!(
            old.values[path],
            serde_json::from_slice::<serde_json::Value>(original).unwrap()
        );
    }
    assert_eq!(
        facts::registry_values(root, &before, CONFIG).unwrap(),
        old.values
    );
    let new = facts::registry_snapshot(root, &after, CONFIG).unwrap();
    assert_eq!(new.bytes[path], changed);
    assert_eq!(new.values[path]["n"], 2);
    assert_ne!(new.values, old.values);
    assert_eq!(old.bytes, originals, "a later read cannot alter old bytes");
}

#[test]
fn registry_snapshot_rejects_missing_objects_and_invalid_registered_json() {
    let (dir, _, valid) = fixture();
    let root = dir.path();
    let path = ".chrono-harness/workflow café.json";
    fs::write(root.join(path), b"{\"duplicate\":1,\"duplicate\":2}").unwrap();
    let malformed = commit(root);
    assert!(facts::registry_snapshot(root, &malformed, CONFIG).is_err());
    fs::remove_file(root.join(path)).unwrap();
    let missing = commit(root);
    assert!(facts::registry_snapshot(root, &missing, CONFIG).is_err());
    assert!(facts::registry_snapshot(root, &"f".repeat(40), CONFIG).is_err());
    assert_eq!(
        facts::registry_snapshot(root, &valid, CONFIG)
            .unwrap()
            .bytes
            .len(),
        5,
        "later damage cannot replace the fixed valid snapshot"
    );
}

#[test]
fn selector_snapshot_keeps_entry_and_independent_endpoint_targets() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join(".chrono-harness")).unwrap();
    git(root, &["init", "-q"]);
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let selector = ".chrono-harness/platforms.json";
    let base_target = ".chrono-harness/full base.json";
    let candidate_target = ".chrono-harness/full candidate.json";
    let base_judges = ".chrono-harness/base judges.json";
    let candidate_judges = ".chrono-harness/candidate judges.json";
    let policy = |judges: &str| {
        json!({"schema_version":3,"status":"proposed","enforcement":"not-implemented",
            "registries":{"judges":judges,"projects":".chrono-harness/projects.json","filemap":".chrono-harness/filemap.json","workflow":".chrono-harness/workflow.json"}})
    };
    fs::write(root.join(base_judges), b"{\"judges\":[]}\n").unwrap();
    fs::write(
        root.join(".chrono-harness/projects.json"),
        b"{\"projects\":[],\"scripts\":[]}\n",
    )
    .unwrap();
    fs::write(
        root.join(".chrono-harness/filemap.json"),
        b"{\"schema_version\":1,\"files\":[],\"test_costs\":[]}\n",
    )
    .unwrap();
    fs::write(root.join(".chrono-harness/workflow.json"), b"{\"schema_version\":1,\"historical_profiles\":[],\"migrations\":[],\"retirements\":[],\"stability\":[]}\n").unwrap();
    fs::write(
        root.join(base_target),
        serde_json::to_vec(&policy(base_judges)).unwrap(),
    )
    .unwrap();
    fs::write(
        root.join(selector),
        serde_json::to_vec(
            &json!({"schema":"chrono-git-configs/v1","platforms":{platform.clone():base_target}}),
        )
        .unwrap(),
    )
    .unwrap();
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "base",
        ],
    );
    let base = git(root, &["rev-parse", "HEAD"]);

    fs::remove_file(root.join(base_target)).unwrap();
    fs::remove_file(root.join(base_judges)).unwrap();
    fs::write(
        root.join(candidate_judges),
        b"{\"judges\":[{\"id\":\"candidate\"}]}\n",
    )
    .unwrap();
    fs::write(
        root.join(candidate_target),
        serde_json::to_vec(&policy(candidate_judges)).unwrap(),
    )
    .unwrap();
    fs::write(
        root.join(selector),
        serde_json::to_vec(
            &json!({"schema":"chrono-git-configs/v1","platforms":{platform.clone():candidate_target}}),
        )
        .unwrap(),
    )
    .unwrap();
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "candidate",
        ],
    );
    let candidate = git(root, &["rev-parse", "HEAD"]);

    let old = facts::registry_snapshot(root, &base, selector).unwrap();
    assert_eq!(old.entry_path, selector);
    assert_eq!(old.effective_path, base_target);
    assert!(old.values.contains_key(selector));
    assert!(old.values.contains_key(base_target));
    assert!(!old.values.contains_key(candidate_target));
    let new = facts::registry_snapshot(root, &candidate, selector).unwrap();
    assert_eq!(new.entry_path, selector);
    assert_eq!(new.effective_path, candidate_target);
    assert!(new.values.contains_key(selector));
    assert!(new.values.contains_key(candidate_target));
    assert!(!new.values.contains_key(base_target));
    assert_ne!(old.effective_path, new.effective_path);
    assert_eq!(
        old.bytes[base_target],
        serde_json::to_vec(&policy(base_judges)).unwrap()
    );
    assert_eq!(
        new.bytes[candidate_target],
        serde_json::to_vec(&policy(candidate_judges)).unwrap()
    );

    // A missing selected target is an endpoint error; it cannot fall back to
    // the old target or alias the selector value into a policy document.
    fs::write(
        root.join(selector),
        serde_json::to_vec(&json!({"schema":"chrono-git-configs/v1","platforms":{platform.clone():".chrono-harness/missing.json"}})).unwrap(),
    )
    .unwrap();
    git(root, &["add", selector]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "missing target",
        ],
    );
    let missing = git(root, &["rev-parse", "HEAD"]);
    assert!(facts::registry_snapshot(root, &missing, selector).is_err());
}
