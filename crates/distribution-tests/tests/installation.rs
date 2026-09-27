use chrono_distribution::{Asset, Release, Source, adopt, install, platform, run};
use chrono_harness::file_identity;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::{TempDir, tempdir};

fn save(path: &Path, v: &Value) {
    fs::write(path, serde_json::to_vec_pretty(v).unwrap()).unwrap();
}
struct Fixture {
    host: TempDir,
    release: TempDir,
    manifest: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let host = tempdir().unwrap();
        let release = tempdir().unwrap();
        let mut tools = BTreeMap::new();
        for (name, body) in [
            ("chrono-distribution", b"#!/bin/sh\nexit 0\n".as_slice()),
            ("alpha", b"#!/bin/sh\nprintf 'alpha-v1\\n'\n"),
            ("beta", b"#!/bin/sh\nprintf 'beta-v1\\n'\n"),
        ] {
            let file = format!("{name}-{}", platform());
            let path = release.path().join(&file);
            fs::write(&path, body).unwrap();
            let (sha256, size) = file_identity(&path).unwrap();
            tools.insert(name.into(), Asset { file, sha256, size });
        }
        let r = Release {
            schema: "chrono-release/v1".into(),
            version: "v0.1.0-beta.1".into(),
            source_commit: "1".repeat(40),
            source_tree: "2".repeat(40),
            platforms: BTreeMap::from([(platform(), tools)]),
        };
        let manifest = release.path().join("release.json");
        save(&manifest, &serde_json::to_value(r).unwrap());
        Self {
            host,
            release,
            manifest,
        }
    }
    fn adopt(&self) {
        adopt(
            self.host.path(),
            &self.manifest,
            Source::Directory {
                path: self.release.path().into(),
            },
            &["alpha".into(), "beta".into()],
            false,
        )
        .unwrap();
    }
    fn edit_lock(&self, edit: impl FnOnce(&mut Value)) {
        let p = self.host.path().join(".chrono-harness/distribution.json");
        let mut v: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
        edit(&mut v);
        save(&p, &v);
    }
    fn binary(&self, n: &str) -> PathBuf {
        self.host.path().join(format!(".chrono-harness/bin/{n}"))
    }
    fn changed_release(&self) {
        let mut r: Value = serde_json::from_slice(&fs::read(&self.manifest).unwrap()).unwrap();
        r["version"] = json!("v0.1.0-beta.2");
        for n in ["alpha", "beta"] {
            let path = self.release.path().join(format!("{n}-{}", platform()));
            fs::write(&path, format!("#!/bin/sh\nprintf '{n}-v2\\n'\n")).unwrap();
            let (hash, size) = file_identity(&path).unwrap();
            r["platforms"][platform()][n]["sha256"] = json!(hash);
            r["platforms"][platform()][n]["size"] = json!(size);
        }
        save(&self.manifest, &r);
        adopt(
            self.host.path(),
            &self.manifest,
            Source::Directory {
                path: self.release.path().into(),
            },
            &["alpha".into(), "beta".into()],
            true,
        )
        .unwrap();
    }
}

#[test]
fn install_update_repair_and_idempotence_preserve_unselected_tools() {
    let f = Fixture::new();
    f.adopt();
    fs::create_dir_all(f.binary("node").parent().unwrap()).unwrap();
    fs::write(f.binary("node"), "host SDK").unwrap();
    let report = install(f.host.path()).unwrap();
    assert_eq!(report["installed"].as_array().unwrap().len(), 2);
    assert_eq!(
        Command::new(f.binary("alpha")).output().unwrap().stdout,
        b"alpha-v1\n"
    );
    let first = fs::metadata(f.binary("alpha")).unwrap().modified().unwrap();
    install(f.host.path()).unwrap();
    assert_eq!(
        first,
        fs::metadata(f.binary("alpha")).unwrap().modified().unwrap()
    );
    fs::write(f.binary("alpha"), "corrupt").unwrap();
    install(f.host.path()).unwrap();
    f.changed_release();
    install(f.host.path()).unwrap();
    assert_eq!(
        Command::new(f.binary("beta")).output().unwrap().stdout,
        b"beta-v2\n"
    );
    assert_eq!(fs::read(f.binary("node")).unwrap(), b"host SDK");
}

#[test]
fn corrupt_late_download_preserves_all_old_binaries_and_receipt() {
    let f = Fixture::new();
    f.adopt();
    install(f.host.path()).unwrap();
    let old = fs::read(f.binary("alpha")).unwrap();
    let receipt = f
        .host
        .path()
        .join(".chrono-harness/state/distribution.json");
    let old_receipt = fs::read(&receipt).unwrap();
    f.changed_release();
    fs::write(f.release.path().join(format!("beta-{}", platform())), "bad").unwrap();
    assert!(
        install(f.host.path())
            .unwrap_err()
            .contains("integrity mismatch")
    );
    assert_eq!(fs::read(f.binary("alpha")).unwrap(), old);
    assert_eq!(fs::read(receipt).unwrap(), old_receipt);
}

#[test]
fn manifest_tampering_and_wrong_version_never_install() {
    let f = Fixture::new();
    f.adopt();
    let mut bytes = fs::read(&f.manifest).unwrap();
    bytes.push(b' ');
    fs::write(&f.manifest, bytes).unwrap();
    assert!(
        install(f.host.path())
            .unwrap_err()
            .contains("integrity mismatch")
    );
    assert!(!f.binary("alpha").exists());
    let (hash, size) = file_identity(&f.manifest).unwrap();
    f.edit_lock(|v| {
        v["manifest"]["sha256"] = json!(hash);
        v["manifest"]["size"] = json!(size);
        v["version"] = json!("wrong");
    });
    assert!(
        install(f.host.path())
            .unwrap_err()
            .contains("version mismatch")
    );
}

#[test]
fn missing_platform_member_installer_or_duplicate_destination_rejected() {
    for case in 0..4 {
        let f = Fixture::new();
        f.adopt();
        f.edit_lock(|v| match case {
            0 => {
                let a = v["installers"][platform()].take();
                v["installers"] = json!({"other-platform":a});
            }
            1 => {
                v["install"]["missing"] = json!(".chrono-harness/bin/missing");
            }
            2 => {
                v["installers"][platform()]["sha256"] = json!("0".repeat(64));
            }
            _ => {
                v["install"]["beta"] = json!(".chrono-harness/bin/alpha");
            }
        });
        assert!(install(f.host.path()).is_err());
        assert!(!f.binary("alpha").exists());
    }
}

#[test]
fn escaping_and_symlink_destinations_are_rejected() {
    let f = Fixture::new();
    f.adopt();
    f.edit_lock(|v| v["install"]["alpha"] = json!(".chrono-harness/bin/../outside"));
    assert!(install(f.host.path()).is_err());
    f.edit_lock(|v| v["install"]["alpha"] = json!(".chrono-harness/bin/alpha"));
    let outside = tempdir().unwrap();
    symlink(outside.path(), f.host.path().join(".chrono-harness/bin")).unwrap();
    assert!(install(f.host.path()).unwrap_err().contains("symlink"));
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn ordinary_publish_failure_rolls_back_already_replaced_tools() {
    let f = Fixture::new();
    f.adopt();
    install(f.host.path()).unwrap();
    let old = fs::read(f.binary("alpha")).unwrap();
    f.changed_release();
    fs::remove_file(
        f.host
            .path()
            .join(".chrono-harness/state/distribution.json"),
    )
    .unwrap();
    fs::remove_dir(f.host.path().join(".chrono-harness/state")).unwrap();
    fs::write(
        f.host.path().join(".chrono-harness/state"),
        "blocked parent",
    )
    .unwrap();
    let error = install(f.host.path()).unwrap_err();
    assert!(error.contains("rollback errors: []"), "{error}");
    assert_eq!(fs::read(f.binary("alpha")).unwrap(), old);
}

#[test]
fn adoption_is_explicit_and_preserves_customizations() {
    let f = Fixture::new();
    f.adopt();
    f.adopt();
    let path = f.host.path().join(".chrono-harness/install.py");
    fs::write(&path, "custom").unwrap();
    assert!(
        adopt(
            f.host.path(),
            &f.manifest,
            Source::Directory {
                path: f.release.path().into()
            },
            &["alpha".into()],
            false
        )
        .unwrap_err()
        .contains("existing adoption differs")
    );
    assert_eq!(fs::read(path).unwrap(), b"custom");
}

#[test]
fn bootstrap_rejects_corrupt_installer_and_propagates_real_exit() {
    let f = Fixture::new();
    let path = f
        .release
        .path()
        .join(format!("chrono-distribution-{}", platform()));
    fs::write(&path, "#!/bin/sh\nexit 23\n").unwrap();
    let (hash, size) = file_identity(&path).unwrap();
    let mut r: Value = serde_json::from_slice(&fs::read(&f.manifest).unwrap()).unwrap();
    r["platforms"][platform()]["chrono-distribution"]["sha256"] = json!(hash);
    r["platforms"][platform()]["chrono-distribution"]["size"] = json!(size);
    save(&f.manifest, &r);
    f.adopt();
    let launch = || {
        Command::new("python3")
            .arg(f.host.path().join(".chrono-harness/install.py"))
            .arg(f.host.path())
            .current_dir("/")
            .output()
            .unwrap()
    };
    assert_eq!(launch().status.code(), Some(23));
    fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    let out = launch();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("integrity mismatch"));
}

#[test]
fn pack_and_assemble_bind_real_source_and_declared_members() {
    let repo = tempdir().unwrap();
    let out = tempdir().unwrap();
    let git = |args: &[&str]| {
        let s = Command::new("git")
            .args(args)
            .current_dir(repo.path())
            .output()
            .unwrap();
        assert!(s.status.success(), "{}", String::from_utf8_lossy(&s.stderr));
    };
    git(&["init", "-q"]);
    fs::write(repo.path().join(".gitignore"), "/built/\n").unwrap();
    fs::write(repo.path().join("source"), "real input").unwrap();
    fs::create_dir(repo.path().join("built")).unwrap();
    fs::write(repo.path().join("built/installer"), "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(
        repo.path().join("built/installer"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    save(
        &repo.path().join("plan.json"),
        &json!({"schema":"chrono-release-plan/v1","version":"v1-beta.1","assets":{"chrono-distribution":"built/installer"}}),
    );
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "-qm",
        "fixture",
    ]);
    let packed = out.path().join("packed");
    let args = vec![
        "pack".into(),
        "--root".into(),
        repo.path().display().to_string(),
        "--plan".into(),
        repo.path().join("plan.json").display().to_string(),
        "--output".into(),
        packed.display().to_string(),
    ];
    run(&args).unwrap();
    let combined = out.path().join("combined");
    run(&[
        "assemble".into(),
        "--input".into(),
        packed.display().to_string(),
        "--output".into(),
        combined.display().to_string(),
    ])
    .unwrap();
    assert_eq!(
        fs::read(packed.join("release.json")).unwrap(),
        fs::read(combined.join("release.json")).unwrap()
    );
    fs::write(repo.path().join("source"), "dirty").unwrap();
    let mut dirty = args;
    *dirty.last_mut().unwrap() = out.path().join("dirty").display().to_string();
    assert!(run(&dirty).unwrap_err().contains("dirty"));
}

#[test]
fn rejects_duplicate_json_members_unknown_cli_and_lock_contention() {
    assert!(
        run(&[
            "install".into(),
            "--host-root".into(),
            ".".into(),
            "--unknown".into(),
            "x".into()
        ])
        .is_err()
    );
    let f = Fixture::new();
    f.adopt();
    fs::write(
        f.host.path().join(".chrono-harness/.distribution-lock"),
        "busy",
    )
    .unwrap();
    assert!(
        install(f.host.path())
            .unwrap_err()
            .contains("installation lock")
    );
    fs::remove_file(f.host.path().join(".chrono-harness/.distribution-lock")).unwrap();
    let p = f.host.path().join(".chrono-harness/distribution.json");
    let bytes = fs::read_to_string(&p).unwrap();
    fs::write(&p, bytes.replacen('{', "{\"version\":\"v9\",", 1)).unwrap();
    assert!(install(f.host.path()).is_err());
}

#[test]
fn real_generated_bootstrap_installs_from_unrelated_cwd_and_spaced_host() {
    let fixture = Fixture::new();
    let installer = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../distribution/target/debug/chrono-distribution");
    assert!(
        installer.is_file(),
        "build registered distribution prerequisite first"
    );
    let target = fixture
        .release
        .path()
        .join(format!("chrono-distribution-{}", platform()));
    fs::copy(&installer, &target).unwrap();
    let (hash, size) = file_identity(&target).unwrap();
    let mut r: Value = serde_json::from_slice(&fs::read(&fixture.manifest).unwrap()).unwrap();
    r["platforms"][platform()]["chrono-distribution"]["sha256"] = json!(hash);
    r["platforms"][platform()]["chrono-distribution"]["size"] = json!(size);
    save(&fixture.manifest, &r);
    let host = fixture.host.path().join("host with spaces");
    fs::create_dir(&host).unwrap();
    adopt(
        &host,
        &fixture.manifest,
        Source::Directory {
            path: fixture.release.path().into(),
        },
        &["alpha".into(), "beta".into()],
        false,
    )
    .unwrap();
    let result = Command::new("python3")
        .arg(host.join(".chrono-harness/install.py"))
        .arg(&host)
        .current_dir("/")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        Command::new(host.join(".chrono-harness/bin/alpha"))
            .output()
            .unwrap()
            .stdout,
        b"alpha-v1\n"
    );
    let report: Value = serde_json::from_slice(
        &fs::read(host.join(".chrono-harness/state/distribution.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["platform"], platform());
    assert_eq!(report["installed"].as_array().unwrap().len(), 2);
}

#[test]
fn assemble_rejects_different_sources_and_corrupt_members_without_output() {
    for corrupt in [false, true] {
        let f = Fixture::new();
        let second = Fixture::new();
        let output = tempdir().unwrap();
        if corrupt {
            fs::write(
                f.release.path().join(format!("alpha-{}", platform())),
                "changed",
            )
            .unwrap();
        } else {
            let mut r: Value =
                serde_json::from_slice(&fs::read(&second.manifest).unwrap()).unwrap();
            r["source_commit"] = json!("3".repeat(40));
            save(&second.manifest, &r);
        }
        let target = output.path().join("release");
        let error = run(&[
            "assemble".into(),
            "--input".into(),
            f.release.path().display().to_string(),
            "--input".into(),
            second.release.path().display().to_string(),
            "--output".into(),
            target.display().to_string(),
        ])
        .unwrap_err();
        assert!(
            error.contains(if corrupt {
                "integrity mismatch"
            } else {
                "source/version mismatch"
            }),
            "{error}"
        );
        assert!(!target.exists());
    }
}
