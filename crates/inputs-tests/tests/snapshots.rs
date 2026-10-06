#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::{json, sha256};
use serde_json::{Value, json as value};
use std::{fs, path::Path, process::Command};
use support::*;
fn invoke(root: &Path, args: &[&str]) -> (i32, Value, String) {
    let mut cmd = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"));
    cmd.current_dir("/")
        .env_clear()
        .env("DECLARED_EMPTY", "")
        .env("SHOULD_NOT_LEAK", "undeclared");
    cmd.args(args).args(["--host-root", root.to_str().unwrap()]);
    let out = cmd.output().unwrap();
    (
        out.status.code().unwrap(),
        json(&out.stdout).unwrap_or(Value::Null),
        String::from_utf8_lossy(&out.stderr).into(),
    )
}
fn fixture() -> (tempfile::TempDir, tempfile::NamedTempFile, Values, String) {
    let d = tempfile::Builder::new()
        .prefix("retained input host ")
        .tempdir()
        .unwrap();
    git(d.path(), &["init", "-q"]);
    fs::write(d.path().join(".gitignore"), ".chrono-harness/state/\n").unwrap();
    let f = tempfile::NamedTempFile::new().unwrap();
    fs::write(f.path(), b"original\0binary\xff").unwrap();
    let mut v = values();
    v.get_mut(CONFIG).unwrap()["environment"] = value!({"inherit":["DECLARED_EMPTY","DECLARED_ABSENT"],"values":{"FIXED":"fixed"},"inputs":[{"id":"data","location":f.path(),"sha256":sha256(b"original\0binary\xff")},{"id":"same-bytes","location":f.path(),"sha256":sha256(b"original\0binary\xff")}]});
    write_values(d.path(), &v);
    let commit = commit(d.path());
    (d, f, v, commit)
}
#[test]
fn schema4_snapshots_exclude_acquisition_credentials_and_legacy_readers_stay_strict() {
    let (d, _f, mut v, _) = fixture();
    let path_dir = tempfile::tempdir().unwrap();
    let ambient_git = chrono_harness::resolve_program(d.path(), "git", None).unwrap();
    let git_link = path_dir.path().join("git");
    std::os::unix::fs::symlink(&ambient_git, &git_link).unwrap();
    let path = format!(
        "{}:{}",
        path_dir.path().display(),
        std::env::var("PATH").unwrap()
    );
    let git =
        fs::canonicalize(chrono_harness::resolve_program(d.path(), "git", Some(&path)).unwrap())
            .unwrap();
    assert!(fs::metadata(&git).unwrap().is_file());
    let version = Command::new(&git).arg("--version").output().unwrap();
    let c = v.get_mut(CONFIG).unwrap();
    c["schema_version"] = value!(4);
    c["canonical_check"] = value!({"operation":"validate.delta","argv":[".chrono-harness/bin/chrono-harness","check"],"profile":CONFIG,"inputs":{"local":{"operation":"fixture.input","tool":"git","argv":["--version"]}}});
    c["facts_git"] = value!({"tool":"git","input":"git-bytes"});
    c["tools"].as_array_mut().unwrap().push(value!({"id":"git","program":git,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}));
    for i in c["environment"]["inputs"].as_array_mut().unwrap() {
        i["presence"] = value!("present");
        i["location"] = value!(fs::canonicalize(i["location"].as_str().unwrap()).unwrap());
    }
    c["environment"]["inputs"].as_array_mut().unwrap().push(value!({"id":"git-bytes","location":git,"presence":"present","sha256":sha256(&fs::read(&git).unwrap())}));
    c["environment"]["inherit"]
        .as_array_mut()
        .unwrap()
        .extend([value!("CHRONO_CHECK_SOURCE"), value!("GH_TOKEN")]);
    c["environment"]["credential_environment"] = value!(["GH_TOKEN"]);
    c["input_closure"] = value!({"status":"incomplete","unresolved":["fixture"]});
    write_values(d.path(), &v);
    let oid = commit(d.path());
    let out = Command::new(source().join("crates/inputs/target/debug/chrono-inputs"))
        .env("GH_TOKEN", "fixture-secret-not-a-business-input")
        .env_remove("CHRONO_CHECK_SOURCE")
        .args([
            "capture",
            "--host-root",
            d.path().to_str().unwrap(),
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/credential-snapshot.json",
        ])
        .env("PATH", &path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!String::from_utf8_lossy(&out.stdout).contains("fixture-secret-not-a-business-input"));
    let snapshot = json(&out.stdout).unwrap();
    assert_eq!(snapshot["environment"]["GH_TOKEN"], Value::Null);
    chrono_judge_registration::inputs::environment(&v[CONFIG], &snapshot["environment"]).unwrap();
    let mut wrong = snapshot["environment"].clone();
    wrong["GH_TOKEN"] = value!("retained-by-mistake");
    assert!(
        chrono_judge_registration::inputs::environment(&v[CONFIG], &wrong)
            .unwrap_err()
            .contains("acquisition credentials")
    );
    let mut old = values();
    old.get_mut(CONFIG).unwrap()["environment"]["credential_environment"] = value!([]);
    assert!(
        chrono_judge_registration::Registrations::load(&old, CONFIG)
            .err()
            .unwrap()
            .contains("unknown field credential_environment")
    );
}
#[test]
fn captures_only_declared_bytes_and_environment_and_deduplicates_content() {
    let (d, f, _, oid) = fixture();
    let args = [
        "capture",
        "--config",
        CONFIG,
        "--commit",
        &oid,
        "--output",
        ".chrono-harness/state/one.json",
    ];
    let (code, s, err) = invoke(d.path(), &args);
    assert_eq!(code, 0, "{err}");
    assert_eq!(s["schema"], "chrono-input-snapshot/v1");
    assert_eq!(s["commit"], oid);
    assert_eq!(
        s["environment"],
        value!({"DECLARED_EMPTY":"","DECLARED_ABSENT":null})
    );
    assert_eq!(s["files"]["data"], s["files"]["same-bytes"]);
    assert!(s["files"]["data"].get("bytes").is_none());
    let blob = s["files"]["data"]["blob"].as_str().unwrap();
    assert_eq!(
        fs::read(d.path().join(blob)).unwrap(),
        fs::read(f.path()).unwrap()
    );
    let (code, again, err) = invoke(d.path(), &args);
    assert_eq!(code, 0, "{err}");
    assert_eq!(s, again);
    assert_eq!(
        fs::read_dir(d.path().join(".chrono-harness/state/inputs/blobs"))
            .unwrap()
            .count(),
        1
    );
}
#[test]
fn captures_observed_mismatch_without_rewriting_registration_and_preserves_old_blob() {
    let (d, f, _, oid) = fixture();
    let cfg = fs::read(d.path().join(CONFIG)).unwrap();
    let (code, old, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    fs::write(f.path(), b"changed").unwrap();
    let (code, new, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/two.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert_ne!(
        old["files"]["data"]["sha256"],
        new["files"]["data"]["sha256"]
    );
    assert_eq!(
        fs::read(
            d.path()
                .join(old["files"]["data"]["blob"].as_str().unwrap())
        )
        .unwrap(),
        b"original\0binary\xff"
    );
    assert_eq!(fs::read(d.path().join(CONFIG)).unwrap(), cfg);
    let (code, _, _) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_ne!(code, 0);
}
#[test]
fn pairs_explicit_snapshots_without_recapturing_the_old_environment_or_file() {
    let (d, f, _, oid) = fixture();
    let (code, a, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    fs::write(f.path(), b"new bytes").unwrap();
    let newer = commit(d.path());
    let (code, b, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &newer,
            "--output",
            ".chrono-harness/state/two.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    let (code, p, err) = invoke(
        d.path(),
        &[
            "pair",
            "--base-snapshot",
            ".chrono-harness/state/one.json",
            "--candidate-snapshot",
            ".chrono-harness/state/two.json",
            "--output",
            ".chrono-harness/state/pair.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(p, value!({"base":a,"candidate":b}));
}
#[test]
fn wrong_checkout_dirty_config_missing_input_and_unsafe_output_never_publish_snapshot() {
    let (d, f, _, oid) = fixture();
    for (commit, path) in [
        ("HEAD", ".chrono-harness/state/one.json"),
        (&oid, "outside.json"),
        (&oid, ".chrono-harness/state/../bad.json"),
    ] {
        let (code, _, _) = invoke(
            d.path(),
            &[
                "capture", "--config", CONFIG, "--commit", commit, "--output", path,
            ],
        );
        assert_ne!(code, 0);
    }
    let original = fs::read(d.path().join(CONFIG)).unwrap();
    fs::write(d.path().join(CONFIG), b"{}").unwrap();
    assert_ne!(
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/one.json"
            ]
        )
        .0,
        0
    );
    fs::write(d.path().join(CONFIG), original).unwrap();
    commit(d.path());
    assert_ne!(
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/wrong-checkout.json"
            ]
        )
        .0,
        0
    );
    git(d.path(), &["checkout", "--detach", &oid]);
    fs::remove_file(f.path()).unwrap();
    assert_ne!(
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/one.json"
            ]
        )
        .0,
        0
    );
    assert!(!d.path().join(".chrono-harness/state/one.json").exists());
}

#[test]
fn transport_to_a_new_host_uses_retained_bytes_after_original_input_disappears() {
    let (d, f, _, oid) = fixture();
    let (code, snapshot, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    fs::remove_file(f.path()).unwrap();
    let dest = tempfile::Builder::new()
        .prefix("new snapshot host ")
        .tempdir()
        .unwrap();
    let args = [
        "pair",
        "--base-root",
        d.path().to_str().unwrap(),
        "--candidate-root",
        d.path().to_str().unwrap(),
        "--base-snapshot",
        ".chrono-harness/state/one.json",
        "--candidate-snapshot",
        ".chrono-harness/state/one.json",
        "--output",
        ".chrono-harness/state/pair.json",
    ];
    let (code, pair, err) = invoke(dest.path(), &args);
    assert_eq!(code, 0, "{err}");
    assert_eq!(pair["base"], snapshot);
    assert_eq!(pair["candidate"], snapshot);
    let blob = snapshot["files"]["data"]["blob"].as_str().unwrap();
    assert_eq!(
        fs::read(dest.path().join(blob)).unwrap(),
        b"original\0binary\xff"
    );
    fs::write(d.path().join(blob), b"corrupt retained content").unwrap();
    let (code, _, err) = invoke(dest.path(), &args);
    assert_ne!(code, 0);
    assert!(err.contains("E_INPUT_BLOB"), "{err}");
}

#[test]
fn corrupt_existing_content_address_and_symlink_output_are_rejected() {
    let (d, _file, _, oid) = fixture();
    let args = [
        "capture",
        "--config",
        CONFIG,
        "--commit",
        &oid,
        "--output",
        ".chrono-harness/state/one.json",
    ];
    let (code, s, err) = invoke(d.path(), &args);
    assert_eq!(code, 0, "{err}");
    fs::write(
        d.path().join(s["files"]["data"]["blob"].as_str().unwrap()),
        b"wrong",
    )
    .unwrap();
    let (code, _, err) = invoke(d.path(), &args);
    assert_ne!(code, 0);
    assert!(err.contains("E_INPUT_BLOB"), "{err}");
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), d.path().join(".chrono-harness/state/alias"))
            .unwrap();
        let (code, _, err) = invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/alias/output.json",
            ],
        );
        assert_ne!(code, 0);
        assert!(err.contains("symlink"), "{err}");
        assert!(!outside.path().join("output.json").exists());
    }
}

#[test]
fn pairing_does_not_silently_normalize_conflicting_or_incomplete_snapshot_fields() {
    let (d, _file, _, oid) = fixture();
    let (code, snapshot, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/one.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    let mut cases = Vec::new();
    let mut bad = snapshot.clone();
    bad["files"]["data"]["bytes"] = value!([1, 2]);
    cases.push(bad);
    bad = snapshot.clone();
    bad.as_object_mut().unwrap().remove("config_digest");
    cases.push(bad);
    bad = snapshot.clone();
    bad["environment"] = value!([]);
    cases.push(bad);
    bad = snapshot.clone();
    bad["environment"]["DECLARED_EMPTY"] = value!(false);
    cases.push(bad);
    bad = snapshot.clone();
    bad["files"]["data"]["length"] = value!(-1);
    cases.push(bad);
    bad = snapshot.clone();
    bad["files"]["data"]["sha256"] = value!("bad digest");
    cases.push(bad);
    bad = snapshot.clone();
    bad["files"]["data"]["blob"] = value!(".chrono-harness/state/../outside");
    cases.push(bad);
    bad = snapshot.clone();
    bad["config_path"] = value!("../config.json");
    cases.push(bad);
    bad = snapshot.clone();
    bad["unknown"] = value!(true);
    cases.push(bad);
    for data in cases {
        fs::write(
            d.path().join(".chrono-harness/state/bad.json"),
            serde_json::to_vec(&data).unwrap(),
        )
        .unwrap();
        let (code, _, err) = invoke(
            d.path(),
            &[
                "pair",
                "--base-snapshot",
                ".chrono-harness/state/bad.json",
                "--candidate-snapshot",
                ".chrono-harness/state/one.json",
                "--output",
                ".chrono-harness/state/must-not-publish.json",
            ],
        );
        assert_ne!(code, 0, "{err}");
        assert!(
            !d.path()
                .join(".chrono-harness/state/must-not-publish.json")
                .exists()
        );
    }
}

#[test]
fn explicit_absence_is_retained_transported_and_distinct_from_empty_bytes() {
    let (d, f, mut v, _) = fixture();
    v.get_mut(CONFIG).unwrap()["schema_version"] = value!(2);
    for row in v.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        row["location"] = value!(fs::canonicalize(f.path()).unwrap());
        row["presence"] = value!("absent");
        row.as_object_mut().unwrap().remove("sha256");
    }
    write_values(d.path(), &v);
    let oid = commit(d.path());
    fs::remove_file(f.path()).unwrap();
    let capture = |out| {
        invoke(
            d.path(),
            &[
                "capture", "--config", CONFIG, "--commit", &oid, "--output", out,
            ],
        )
    };
    let (code, absent, err) = capture(".chrono-harness/state/absent.json");
    assert_eq!(code, 0, "{err}");
    assert_eq!(absent["schema"], "chrono-input-snapshot/v2");
    assert_eq!(absent["files"]["data"], value!({"absent":true}));
    fs::write(f.path(), []).unwrap();
    let (code, empty, err) = capture(".chrono-harness/state/empty.json");
    assert_eq!(
        code, 0,
        "capture observes even a declaration mismatch: {err}"
    );
    assert_eq!(empty["files"]["data"]["length"], 0);
    assert_eq!(empty["files"]["data"]["sha256"], sha256(b""));
    let target = tempfile::tempdir().unwrap();
    let (code, pair, err) = invoke(
        target.path(),
        &[
            "pair",
            "--base-root",
            d.path().to_str().unwrap(),
            "--candidate-root",
            d.path().to_str().unwrap(),
            "--base-snapshot",
            ".chrono-harness/state/absent.json",
            "--candidate-snapshot",
            ".chrono-harness/state/empty.json",
            "--output",
            ".chrono-harness/state/pair.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(pair["base"], absent);
    assert_eq!(pair["candidate"], empty);
    assert_eq!(
        fs::read_dir(target.path().join(".chrono-harness/state/inputs/blobs"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn v2_absence_rejects_symlink_directory_and_non_directory_ancestors() {
    let (d, f, mut v, _) = fixture();
    let external = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(external.path()).unwrap();
    let bad = root.join("bad");
    v.get_mut(CONFIG).unwrap()["schema_version"] = value!(2);
    v.get_mut(CONFIG).unwrap()["environment"]["inputs"] =
        value!([{"id":"data","location":bad,"presence":"absent"}]);
    write_values(d.path(), &v);
    let oid = commit(d.path());
    let capture = || {
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/bad.json",
            ],
        )
    };
    fs::create_dir(&bad).unwrap();
    assert_ne!(capture().0, 0, "directory is not absent");
    fs::remove_dir(&bad).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("missing"), &bad).unwrap();
        assert_ne!(capture().0, 0, "dangling link is not absent");
        fs::remove_file(&bad).unwrap();
        std::os::unix::fs::symlink(f.path(), &bad).unwrap();
        assert_ne!(capture().0, 0, "live link is not a v2 input");
        fs::remove_file(&bad).unwrap();
    }
    fs::write(&bad, []).unwrap();
    v.get_mut(CONFIG).unwrap()["environment"]["inputs"][0]["location"] = value!(bad.join("child"));
    write_values(d.path(), &v);
    let oid = commit(d.path());
    assert_ne!(
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/non-directory.json"
            ]
        )
        .0,
        0
    );
    fs::remove_file(&bad).unwrap();
    let (code, observed, err) = invoke(
        d.path(),
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/missing-parent.json",
        ],
    );
    assert_eq!(code, 0, "missing directory gives explicit absence: {err}");
    assert_eq!(observed["files"]["data"], value!({"absent":true}));
}

#[test]
fn absence_versioning_rejects_ambiguous_declarations_and_legacy_null() {
    use chrono_judge_registration::{Registrations, inputs::snapshot_shape};
    let (d, f, mut v, _) = fixture();
    for row in v.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        row["sha256"] = Value::Null;
    }
    assert!(
        Registrations::load(&v, CONFIG).is_ok(),
        "v1 null remains an unbound draft"
    );
    write_values(d.path(), &v);
    let oid = commit(d.path());
    fs::remove_file(f.path()).unwrap();
    assert_ne!(
        invoke(
            d.path(),
            &[
                "capture",
                "--config",
                CONFIG,
                "--commit",
                &oid,
                "--output",
                ".chrono-harness/state/legacy.json"
            ]
        )
        .0,
        0
    );
    for row in [
        value!({"id":"data","location":"external","presence":"absent","sha256":null}),
        value!({"id":"data","location":"external","presence":"present"}),
        value!({"id":"data","location":"external","sha256":null}),
        value!({"id":"data","location":"external","presence":"unknown"}),
    ] {
        v.get_mut(CONFIG).unwrap()["schema_version"] = value!(2);
        v.get_mut(CONFIG).unwrap()["environment"]["inputs"] = value!([row]);
        assert!(Registrations::load(&v, CONFIG).is_err());
    }
    v.get_mut(CONFIG).unwrap()["schema_version"] = value!(1);
    v.get_mut(CONFIG).unwrap()["environment"]["inputs"] =
        value!([{"id":"data","location":"external","presence":"absent"}]);
    assert!(Registrations::load(&v, CONFIG).is_err());
    let mut empty = values();
    empty.get_mut(CONFIG).unwrap()["schema_version"] = value!(2);
    let r = Registrations::load(&empty, CONFIG).unwrap();
    assert!(
        chrono_judge_registration::inputs::effective_environments(&Value::Null, &r, &r).is_err()
    );
    let mut s = value!({"schema":"chrono-input-snapshot/v1","commit":oid,"config_path":CONFIG,"config_digest":"a".repeat(64),"environment":{},"files":{"data":{"absent":true}}});
    assert!(snapshot_shape(&s).is_err());
    s["schema"] = value!("chrono-input-snapshot/v2");
    assert!(snapshot_shape(&s).is_ok());
    s["files"]["data"]["absent"] = value!(false);
    assert!(snapshot_shape(&s).is_err());
}

#[test]
fn v3_capture_uses_bound_git_and_keeps_v2_snapshot_semantics() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, _data, mut values, _) = fixture();
    let root = fs::canonicalize(dir.path()).unwrap();
    let state = root.join(".chrono-harness/state");
    fs::create_dir_all(&state).unwrap();
    let program = state.join("snapshot Git λ");
    let trace = state.join("git-trace");
    let real =
        fs::canonicalize(chrono_harness::resolve_program(&root, "git", None).unwrap()).unwrap();
    let version = Command::new(&real).arg("--version").output().unwrap();
    assert!(version.status.success());
    fs::copy(env!("CARGO_BIN_EXE_chrono-inputs-test-git"), &program).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let config = values.get_mut(CONFIG).unwrap();
    config["schema_version"] = value!(3);
    config["facts_git"] = value!({"tool":"chosen-git","input":"git-input"});
    config["tools"].as_array_mut().unwrap().push(value!({"id":"chosen-git","program":program,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim_end()}));
    config["environment"]["values"]["SNAPSHOT_GIT"] = value!("declared");
    config["environment"]["values"]["SNAPSHOT_REAL_GIT"] = value!(real);
    config["environment"]["values"]["SNAPSHOT_TRACE"] = value!(trace);
    for input in config["environment"]["inputs"].as_array_mut().unwrap() {
        input["presence"] = value!("present");
        input["location"] = value!(fs::canonicalize(input["location"].as_str().unwrap()).unwrap());
    }
    config["environment"]["inputs"].as_array_mut().unwrap().push(value!({"id":"git-input","location":program,"presence":"present","sha256":sha256(&fs::read(&program).unwrap())}));
    config["environment"]["inputs"].as_array_mut().unwrap().push(value!({"id":"real-git-input","location":real,"presence":"present","sha256":sha256(&fs::read(&real).unwrap())}));
    config["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .push(value!({"id":"absent","location":state.join("absent-input"),"presence":"absent"}));
    // A v3 declared-complete config must carry the explicit external-input
    // inventory. Capture consumes the registration snapshot contract directly,
    // so keep every tool, retained file and environment value visible here.
    config["input_closure"] = value!({
        "status": "declared-complete",
        "unresolved": [],
        "bindings": [{
            "id": "snapshot-capture-inputs",
            "consumer": "judge:registration",
            "kind": "snapshot-capture",
            "inputs": [
                "tool:sh",
                "tool:chosen-git",
                "input:data",
                "input:same-bytes",
                "input:git-input",
                "input:real-git-input",
                "input:absent",
                "environment:DECLARED_EMPTY",
                "environment:DECLARED_ABSENT",
                "environment:FIXED",
                "environment:SNAPSHOT_GIT",
                "environment:SNAPSHOT_REAL_GIT",
                "environment:SNAPSHOT_TRACE"
            ]
        }]
    });
    let edges = values.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap();
    for input in [
        "tool:sh",
        "tool:chosen-git",
        "input:data",
        "input:same-bytes",
        "input:git-input",
        "input:real-git-input",
        "input:absent",
        "environment:DECLARED_EMPTY",
        "environment:DECLARED_ABSENT",
        "environment:FIXED",
        "environment:SNAPSHOT_GIT",
        "environment:SNAPSHOT_REAL_GIT",
        "environment:SNAPSHOT_TRACE",
    ] {
        edges.push(value!({
            "from": input,
            "kind": "runtime-input",
            "to": "judge:registration"
        }));
    }
    write_values(&root, &values);
    let oid = commit(&root);
    let (exit, snapshot, error) = invoke(
        &root,
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/v3.json",
        ],
    );
    assert_eq!(exit, 0, "{error}");
    assert_eq!(snapshot["schema"], "chrono-input-snapshot/v2");
    assert_eq!(snapshot["files"]["absent"], value!({"absent":true}));
    assert_eq!(
        snapshot["config_digest"],
        chrono_harness::wire::digest(&values[CONFIG]).unwrap()
    );
    assert_eq!(
        snapshot["files"]["git-input"]["sha256"],
        sha256(&fs::read(&program).unwrap())
    );
    assert!(fs::read_to_string(trace).unwrap().contains("rev-parse"));

    // The same bound v3 policy can be reached through the native selector
    // entry. Capture publishes a versioned selection binding while retaining
    // the caller's entry path for later registration validation.
    let selector = ".chrono-harness/platforms.json";
    fs::write(
        root.join(selector),
        serde_json::to_vec(&value!({
            "schema": "chrono-git-configs/v1",
            "platforms": {format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH): CONFIG}
        }))
        .unwrap(),
    )
    .unwrap();
    let selected_base = commit(&root);
    let (exit, selected, error) = invoke(
        &root,
        &[
            "capture",
            "--config",
            selector,
            "--commit",
            &selected_base,
            "--output",
            ".chrono-harness/state/selected-base.json",
        ],
    );
    assert_eq!(exit, 0, "{error}");
    assert_eq!(selected["schema"], "chrono-input-snapshot/v3");
    assert_eq!(selected["config_path"], selector);
    assert_eq!(selected["effective_config_path"], CONFIG);
    assert_eq!(selected["selection"]["config_path"], CONFIG);
    fs::write(root.join("src.bin"), b"selected candidate").unwrap();
    let selected_candidate = commit(&root);
    let (exit, _, error) = invoke(
        &root,
        &[
            "capture",
            "--config",
            selector,
            "--commit",
            &selected_candidate,
            "--output",
            ".chrono-harness/state/selected-candidate.json",
        ],
    );
    assert_eq!(exit, 0, "{error}");
    let (exit, paired, error) = invoke(
        &root,
        &[
            "pair",
            "--base-snapshot",
            ".chrono-harness/state/selected-base.json",
            "--candidate-snapshot",
            ".chrono-harness/state/selected-candidate.json",
            "--output",
            ".chrono-harness/state/selected-paired.json",
        ],
    );
    assert_eq!(exit, 0, "{error}");
    assert_eq!(paired["base"]["schema"], "chrono-input-snapshot/v3");
    assert_eq!(paired["candidate"]["effective_config_path"], CONFIG);
    let manifest = ".chrono-harness/state/selected-compose.json";
    let original = fs::read(root.join(".chrono-harness/state/selected-paired.json")).unwrap();
    fs::write(root.join(manifest), serde_json::to_vec(&value!({"schema":"chrono-input-composition/v1",
        "sources":[{"pair":{"path":".chrono-harness/state/selected-paired.json","sha256":sha256(&original)}}]})).unwrap()).unwrap();
    assert_eq!(
        chrono_inputs::compose(
            &root,
            selector,
            &selected_base,
            &selected_candidate,
            manifest,
            ".chrono-harness/state/selected-composed.json",
            ".chrono-harness/state/selected-receipt.json"
        )
        .unwrap(),
        paired
    );
    for field in ["effective_config_path", "selection"] {
        let mut wrong = paired.clone();
        wrong["candidate"][field] = if field == "selection" {
            let mut v = wrong["candidate"][field].clone();
            v["platform"] = value!("wrong-platform");
            v
        } else {
            value!("wrong-config.json")
        };
        if field == "effective_config_path" {
            wrong["candidate"]["selection"]["config_path"] = value!("wrong-config.json");
        }
        let path = format!(".chrono-harness/state/wrong-{field}.json");
        let raw = serde_json::to_vec(&wrong).unwrap();
        fs::write(root.join(&path), &raw).unwrap();
        fs::write(
            root.join(manifest),
            serde_json::to_vec(&value!({"schema":"chrono-input-composition/v1",
            "sources":[{"pair":{"path":path,"sha256":sha256(&raw)}}]}))
            .unwrap(),
        )
        .unwrap();
        let error = chrono_inputs::compose(
            &root,
            selector,
            &selected_base,
            &selected_candidate,
            manifest,
            &format!(".chrono-harness/state/reject-{field}.json"),
            &format!(".chrono-harness/state/reject-{field}.receipt.json"),
        )
        .unwrap_err();
        assert!(error.contains("E_COMPOSE_HEADER"), "{error}");
    }
    fs::copy(env!("CARGO_BIN_EXE_chrono-inputs-test-zero"), &program).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let (exit, _, error) = invoke(
        &root,
        &[
            "capture",
            "--config",
            CONFIG,
            "--commit",
            &oid,
            "--output",
            ".chrono-harness/state/rejected.json",
        ],
    );
    assert_ne!(exit, 0);
    assert!(error.contains("E_GIT_FACTS"));
    assert!(!state.join("rejected.json").exists());
    assert!(state.join("v3.json").exists());
}

#[test]
fn compose_genuine_captures_retains_originals_and_rejects_conflicts_and_missing_coverage() {
    let (d, input, mut v, base) = fixture();
    let captured = chrono_inputs::capture(
        d.path(),
        CONFIG,
        &base,
        ".chrono-harness/state/base-capture.json",
    )
    .unwrap();
    // Historical input bytes remain genuine after the live input and declaration change.
    fs::write(input.path(), b"new actual endpoint bytes").unwrap();
    for i in v.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        i["sha256"] = value!(sha256(b"new actual endpoint bytes"));
    }
    write_values(d.path(), &v);
    let candidate = commit(d.path());
    let current = chrono_inputs::capture(
        d.path(),
        CONFIG,
        &candidate,
        ".chrono-harness/state/candidate-capture.json",
    )
    .unwrap();
    let pair = chrono_inputs::pair(
        d.path(),
        d.path(),
        ".chrono-harness/state/base-capture.json",
        d.path(),
        ".chrono-harness/state/candidate-capture.json",
        ".chrono-harness/state/constituent.json",
    )
    .unwrap();
    assert_eq!(pair, value!({"base":captured,"candidate":current}));
    let original = fs::read(d.path().join(".chrono-harness/state/constituent.json")).unwrap();
    let source = value!({"pair":{"path":".chrono-harness/state/constituent.json","sha256":sha256(&original)}});
    let manifest = ".chrono-harness/state/composition.json";
    fs::write(
        d.path().join(manifest),
        serde_json::to_vec(&value!({"schema":"chrono-input-composition/v1","sources":[source]}))
            .unwrap(),
    )
    .unwrap();
    fs::remove_file(input.path()).unwrap();
    let (code, result, err) = invoke(
        d.path(),
        &[
            "compose",
            "--config",
            CONFIG,
            "--base",
            &base,
            "--candidate",
            &candidate,
            "--manifest",
            manifest,
            "--output",
            ".chrono-harness/state/composed.json",
            "--receipt",
            ".chrono-harness/state/composed-receipt.json",
        ],
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(result, pair);
    assert_eq!(
        fs::read(d.path().join(".chrono-harness/state/constituent.json")).unwrap(),
        original
    );
    let receipt =
        json(&fs::read(d.path().join(".chrono-harness/state/composed-receipt.json")).unwrap())
            .unwrap();
    assert_eq!(receipt["completeness_proven"], false);
    let retained = receipt["originals"][0]["retained"]["path"]
        .as_str()
        .unwrap();
    assert_eq!(fs::read(d.path().join(retained)).unwrap(), original);
    for (case, expected) in [
        ("coverage", "E_COMPOSE_COVERAGE"),
        ("environment", "E_COMPOSE_ENVIRONMENT"),
        ("config", "E_COMPOSE_HEADER"),
        ("absence", "E_INPUT_SNAPSHOT"),
        ("endpoint", "E_COMPOSE_HEADER"),
        ("blob", "E_COMPOSE_BLOB"),
    ] {
        let mut bad = pair.clone();
        match case {
            "coverage" => {
                bad["base"]["files"].as_object_mut().unwrap().remove("data");
            }
            "environment" => {
                bad["base"]["environment"]["DECLARED_EMPTY"] = value!("different actual capture")
            }
            "config" => bad["base"]["config_digest"] = value!("0".repeat(64)),
            "absence" => bad["base"]["files"]["data"] = value!({"absent":true}),
            "endpoint" => bad["base"]["commit"] = value!(candidate),
            _ => bad["base"]["files"]["data"]["length"] = value!(1),
        }
        let path = format!(".chrono-harness/state/{case}.json");
        let raw = serde_json::to_vec(&bad).unwrap();
        fs::write(d.path().join(&path), &raw).unwrap();
        let source = value!({"pair":{"path":path,"sha256":sha256(&raw)}});
        let sources = if case == "environment" {
            value!([{"pair":{"path":".chrono-harness/state/constituent.json","sha256":sha256(&original)}},source])
        } else {
            value!([source])
        };
        fs::write(
            d.path().join(manifest),
            serde_json::to_vec(&value!({"schema":"chrono-input-composition/v1","sources":sources}))
                .unwrap(),
        )
        .unwrap();
        let error = chrono_inputs::compose(
            d.path(),
            CONFIG,
            &base,
            &candidate,
            manifest,
            &format!(".chrono-harness/state/out-{case}.json"),
            &format!(".chrono-harness/state/receipt-{case}.json"),
        )
        .unwrap_err();
        assert!(error.contains(expected), "{case}: {error}");
        assert!(
            !d.path()
                .join(format!(".chrono-harness/state/out-{case}.json"))
                .exists()
        );
    }
}

#[test]
fn governance_seed_uses_separately_captured_same_path_original_and_current_observation() {
    let (d, input, mut v, _) = fixture();
    v.get_mut(FM).unwrap()["schema_version"] = value!(2);
    v.get_mut(FM).unwrap()["execution_plans"] = value!({});
    v.get_mut(CONFIG).unwrap()["schema_version"] = value!(2);
    for i in v.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        i["presence"] = value!("present");
        i["location"] = value!(fs::canonicalize(input.path()).unwrap());
    }
    let absent = fs::canonicalize(input.path().parent().unwrap())
        .unwrap()
        .join("missing-governance-original");
    assert!(!absent.exists());
    v.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .push(value!({"id":"absent-governance","location":absent,"presence":"absent"}));
    v.get_mut(FM).unwrap()["project_edges"]
        .as_array_mut()
        .unwrap()
        .push(value!({"from":"input:data","kind":"runtime-input","to":"judge:registration"}));
    v.get_mut(FM).unwrap()["project_edges"].as_array_mut().unwrap().push(
        value!({"from":"input:absent-governance","kind":"runtime-input","to":"judge:registration"}),
    );
    write_values(d.path(), &v);
    let base = commit(d.path());
    let base_path = ".chrono-harness/state/base-original.json";
    let old = chrono_inputs::capture(d.path(), CONFIG, &base, base_path).unwrap();
    let old_raw = fs::read(d.path().join(base_path)).unwrap();
    fs::write(input.path(), b"current bytes at the SAME fixed location").unwrap();
    for i in v.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        if i["presence"] == "present" {
            i["sha256"] = value!(sha256(b"current bytes at the SAME fixed location"));
        }
    }
    write_values(d.path(), &v);
    let candidate = commit(d.path());
    let manifest = ".chrono-harness/state/governance-sources.json";
    let source = value!({"pair":{"path":base_path,"sha256":sha256(&old_raw)}});
    fs::write(
        d.path().join(manifest),
        serde_json::to_vec(&value!({
            "schema":"chrono-input-composition/v1","sources":[source]
        }))
        .unwrap(),
    )
    .unwrap();
    let args = [
        "capture-governance",
        "--config",
        CONFIG,
        "--base",
        &base,
        "--candidate",
        &candidate,
        "--manifest",
        manifest,
        "--output",
        ".chrono-harness/state/seed/inputs.json",
    ];
    let (code, seed, err) = invoke(d.path(), &args);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        seed["base"]["files"]["data"]["sha256"],
        old["files"]["data"]["sha256"]
    );
    assert_eq!(
        seed["candidate"]["files"]["data"]["sha256"],
        sha256(b"current bytes at the SAME fixed location")
    );
    assert_eq!(
        seed["base"]["files"]["absent-governance"],
        value!({"absent":true})
    );
    assert_eq!(
        seed["candidate"]["files"]["absent-governance"],
        value!({"absent":true})
    );
    assert_eq!(fs::read(d.path().join(base_path)).unwrap(), old_raw);
    let receipt = json(
        &fs::read(
            d.path()
                .join(".chrono-harness/state/seed/inputs.json.receipt.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        fs::read(
            d.path().join(
                receipt["originals"][0]["retained"]["path"]
                    .as_str()
                    .unwrap()
            )
        )
        .unwrap(),
        old_raw
    );
    chrono_inputs::capture(
        d.path(),
        CONFIG,
        &candidate,
        ".chrono-harness/state/current-original.json",
    )
    .unwrap();
    chrono_inputs::pair(
        d.path(),
        d.path(),
        base_path,
        d.path(),
        ".chrono-harness/state/current-original.json",
        ".chrono-harness/state/original-pair.json",
    )
    .unwrap();
    let original_pair =
        fs::read(d.path().join(".chrono-harness/state/original-pair.json")).unwrap();
    fs::write(
        d.path().join(manifest),
        serde_json::to_vec(&value!({
            "schema":"chrono-input-composition/v1","sources":[{"pair":{
                "path":".chrono-harness/state/original-pair.json","sha256":sha256(&original_pair)}}]
        }))
        .unwrap(),
    )
    .unwrap();
    let mut from_pair = args;
    from_pair[from_pair.len() - 1] = ".chrono-harness/state/pair-seed/inputs.json";
    let (code, paired, err) = invoke(d.path(), &from_pair);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        paired["base"]["files"]["data"]["sha256"],
        old["files"]["data"]["sha256"]
    );
    assert_eq!(
        fs::read(d.path().join(".chrono-harness/state/original-pair.json")).unwrap(),
        original_pair
    );
    // Every negative uses a fresh output; a retained successful seed cannot hide failure.
    for (case, expected) in [
        ("missing", "missing original evidence"),
        ("digest", "digest"),
        ("endpoint", "E_COMPOSE_HEADER"),
        ("config", "E_COMPOSE_HEADER"),
        ("absent", "E_COMPOSE_INPUT"),
        ("absence-lie", "E_COMPOSE_INPUT"),
        ("coverage", "E_GOVERNANCE_INPUT"),
        ("blob", "E_COMPOSE_BLOB"),
        ("blob-missing", "E_INPUT_READ"),
        ("blob-corrupt", "E_COMPOSE_BLOB"),
        ("candidate-drift", "E_GOVERNANCE_INPUT"),
    ] {
        let mut bad = old.clone();
        match case {
            "endpoint" => bad["commit"] = value!("f".repeat(40)),
            "config" => bad["config_digest"] = value!("0".repeat(64)),
            "absent" => bad["files"]["data"] = value!({"absent":true}),
            "absence-lie" => bad["files"]["absent-governance"] = old["files"]["data"].clone(),
            "coverage" => {
                bad["files"].as_object_mut().unwrap().remove("data");
            }
            "blob" => bad["files"]["data"]["length"] = value!(1),
            "candidate-drift" => fs::write(input.path(), b"unbound current bytes").unwrap(),
            _ => {}
        }
        let path = format!(".chrono-harness/state/original-{case}.json");
        let bytes = serde_json::to_vec(&bad).unwrap();
        fs::write(d.path().join(&path), &bytes).unwrap();
        if case == "missing" {
            fs::remove_file(d.path().join(&path)).unwrap();
        }
        fs::write(
            d.path().join(manifest),
            serde_json::to_vec(&value!({
                "schema":"chrono-input-composition/v1","sources":[{"pair":{"path":path,
                "sha256":if case == "digest" { "0".repeat(64) } else { sha256(&bytes) }}}]
            }))
            .unwrap(),
        )
        .unwrap();
        let output = format!(".chrono-harness/state/rejected-{case}/inputs.json");
        let blob_path = d
            .path()
            .join(old["files"]["data"]["blob"].as_str().unwrap());
        let blob_raw = fs::read(&blob_path).unwrap();
        if case == "blob-missing" {
            fs::remove_file(&blob_path).unwrap();
        }
        if case == "blob-corrupt" {
            fs::write(&blob_path, b"corrupt retained original").unwrap();
        }
        let mut negative = args;
        negative[negative.len() - 1] = &output;
        let (code, _, err) = invoke(d.path(), &negative);
        fs::write(&blob_path, blob_raw).unwrap();
        assert_eq!(code, 2, "{case}: {err}");
        assert!(err.contains(expected), "{case}: {err}");
        assert!(!d.path().join(output).exists());
    }
}
