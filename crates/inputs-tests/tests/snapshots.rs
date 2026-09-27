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
