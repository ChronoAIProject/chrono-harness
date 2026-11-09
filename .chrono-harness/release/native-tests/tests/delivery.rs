#[path = "support/recipe.rs"]
mod fixture;
use chrono_host_release::publication::{Commands, Reply};
use chrono_host_release::{
    delivery::{self, Package},
    identity::{Channel, Identity, VerifiedCi},
    receipt::Selection,
    recipe::Loaded,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};
use std::{
    collections::BTreeSet,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

fn identity() -> Identity {
    Identity {
        channel: Channel::Preview,
        version: "v1.2.3-dev.10.1.gaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        source_commit: "a".repeat(40),
        source_tree: "b".repeat(40),
        verified_ci: Some(VerifiedCi {
            run_id: 10,
            attempt: 1,
        }),
    }
}
fn selected() -> Selection {
    Selection {
        job: "native_collect".into(),
        artifact_id: "30".into(),
        run_id: "20".into(),
        attempt: "1".into(),
    }
}

struct InstallCommands {
    calls: Vec<(String, Vec<String>)>,
    host: PathBuf,
    identity: Identity,
    fail: bool,
    bad: Option<&'static str>,
}
impl Commands for InstallCommands {
    fn body(&mut self, _: &Value) -> Result<String, String> {
        panic!("installation has no API mutation")
    }
    fn execute(&mut self, program: &str, args: &[String]) -> Result<Reply, String> {
        self.calls.push((program.into(), args.to_vec()));
        if self.fail {
            return Ok(Reply {
                exit: 17,
                stdout: b"original failed install".to_vec(),
            });
        }
        if program == "/usr/bin/curl" {
            let destination = &args[args.iter().position(|arg| arg == "--output").unwrap() + 1];
            fs::write(
                destination,
                if self.bad == Some("download") {
                    b"changed".as_slice()
                } else {
                    b"fixture public launcher".as_slice()
                },
            )
            .unwrap();
        }
        if program == "/bin/sh" {
            let mut rows = Vec::new();
            for name in ["chrono-distribution", "chrono-harness"] {
                let path = format!(".chrono-harness/bin/{name}");
                let file = self.host.join(&path);
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(
                    &file,
                    if self.bad == Some("bytes") {
                        b"changed".as_slice()
                    } else {
                        b"fixture executable".as_slice()
                    },
                )
                .unwrap();
                fs::set_permissions(
                    &file,
                    fs::Permissions::from_mode(if self.bad == Some("mode") {
                        0o644
                    } else {
                        0o755
                    }),
                )
                .unwrap();
                rows.push(json!({"name":name,"path":path,"sha256":chrono_harness::sha256(b"fixture executable"),"size":18}));
            }
            fixture::write(
                &self.host,
                ".chrono-harness/state/distribution.json",
                &json!({
                "schema":"chrono-install-result/v1","version":self.identity.version,
                "source_commit":if self.bad==Some("source") {"c".repeat(40)} else {self.identity.source_commit.clone()},
                "source_tree":self.identity.source_tree,"platform":"linux-x86_64","installed":rows}),
            );
        }
        Ok(Reply {
            exit: 0,
            stdout: b"original installer output".to_vec(),
        })
    }
}

fn install_fixture(channel: Channel) -> (tempfile::TempDir, Identity, InstallCommands) {
    let root = tempfile::tempdir().unwrap();
    let mut id = identity();
    id.channel = channel;
    let dir = root.path().join("publication/release");
    fs::create_dir_all(&dir).unwrap();
    let mut tools = BTreeMap::new();
    for name in ["chrono-distribution", "chrono-harness"] {
        let file = format!("{name}-linux-x86_64");
        fs::write(dir.join(&file), b"fixture executable").unwrap();
        tools.insert(
            name,
            json!({"file":file,"sha256":chrono_harness::sha256(b"fixture executable"),"size":18}),
        );
    }
    fixture::write(
        &dir,
        "release.json",
        &json!({"schema":"chrono-release/v1","version":id.version,
        "source_commit":id.source_commit,"source_tree":id.source_tree,"platforms":{"linux-x86_64":tools}}),
    );
    fs::write(dir.join("install.sh"), b"fixture public launcher").unwrap();
    let names = [
        "release.json",
        "install.sh",
        "chrono-distribution-linux-x86_64",
        "chrono-harness-linux-x86_64",
    ];
    let mut assets = BTreeMap::new();
    for name in names {
        let (sha, size) = chrono_harness::file_identity(&dir.join(name)).unwrap();
        assets.insert(name, json!({"sha256":sha,"size":size}));
    }
    fs::write(root.path().join("publication/empty"), b"").unwrap();
    let empty = json!({"path":"empty","sha256":chrono_harness::sha256(b""),"size":0});
    fixture::write(
        &root.path().join("publication"),
        "receipt.json",
        &json!({
        "schema":"chrono-release-publication/v1","status":"passed","identity":id,
        "processes":[{"exit_code":0,"stdout":empty,"stderr":empty}],
        "result":{"mode":"build","release":{"draft":false,"tag_name":id.version},"assets":assets}}),
    );
    let commands = InstallCommands {
        calls: vec![],
        host: root.path().join("new host"),
        identity: id.clone(),
        fail: false,
        bad: None,
    };
    (root, id, commands)
}

fn accept(root: &Path, id: &Identity, commands: &mut InstallCommands) -> Result<Value, String> {
    delivery::install_release(
        &root.join("publication"),
        &commands.host.clone(),
        id,
        "linux-x86_64",
        &BTreeSet::from(["chrono-distribution".into(), "chrono-harness".into()]),
        "/bin/sh",
        "/usr/bin/curl",
        "https://example.invalid/fixed/install.sh",
        commands,
    )
}

#[test]
fn integration_build_installs_all_members_from_its_own_package() {
    let (root, id, mut commands) = install_fixture(Channel::Build);
    let receipt = accept(root.path(), &id, &mut commands).unwrap();
    assert_eq!(receipt["source_commit"], id.source_commit);
    assert_eq!(commands.calls.len(), 2);
    let (program, args) = &commands.calls[0];
    assert_eq!(
        program,
        &root
            .path()
            .join("publication/release/chrono-distribution-linux-x86_64")
            .to_str()
            .unwrap()
    );
    assert_eq!(
        args,
        &vec![
            "adopt".to_owned(),
            "--host-root".into(),
            commands.host.to_str().unwrap().into(),
            "--manifest".into(),
            root.path()
                .join("publication/release/release.json")
                .to_str()
                .unwrap()
                .into(),
            "--source-dir".into(),
            root.path()
                .join("publication/release")
                .to_str()
                .unwrap()
                .into(),
            "--tool".into(),
            "chrono-distribution".into(),
            "--tool".into(),
            "chrono-harness".into()
        ]
    );
    assert_eq!(
        commands.calls[1],
        (
            "/bin/sh".into(),
            vec![
                commands
                    .host
                    .join(".chrono-harness/install.sh")
                    .to_str()
                    .unwrap()
                    .into(),
                "--host-root".into(),
                commands.host.to_str().unwrap().into()
            ]
        )
    );
}

#[test]
fn public_channels_consume_the_bound_public_installation_launcher() {
    for channel in [Channel::Preview, Channel::Stable] {
        let (root, id, mut commands) = install_fixture(channel);
        accept(root.path(), &id, &mut commands).unwrap();
        assert_eq!(
            commands.calls[0],
            (
                "/usr/bin/curl".into(),
                vec![
                    "--disable".into(),
                    "--proto".into(),
                    "=https".into(),
                    "--proto-redir".into(),
                    "=https".into(),
                    "--fail".into(),
                    "--show-error".into(),
                    "--location".into(),
                    "--output".into(),
                    commands.host.join("install.sh").to_str().unwrap().into(),
                    "https://example.invalid/fixed/install.sh".into()
                ]
            )
        );
        assert_eq!(
            commands.calls[1],
            (
                "/bin/sh".into(),
                vec![
                    commands.host.join("install.sh").to_str().unwrap().into(),
                    "--host-root".into(),
                    commands.host.to_str().unwrap().into()
                ]
            )
        );
    }
}

#[test]
fn failed_commands_and_false_installation_receipts_cannot_pass() {
    for failure in ["command", "download", "bytes", "mode", "source"] {
        let (root, id, mut commands) = install_fixture(Channel::Preview);
        if failure == "command" {
            commands.fail = true;
        } else {
            commands.bad = Some(failure);
        }
        assert!(
            accept(root.path(), &id, &mut commands).is_err(),
            "{failure}"
        );
        assert_eq!(
            commands.calls.len(),
            if ["command", "download"].contains(&failure) {
                1
            } else {
                2
            }
        );
    }
}

#[test]
fn changed_publication_or_assets_stop_before_installation() {
    for failure in ["identity", "status", "asset", "original", "existing-host"] {
        let (root, id, mut commands) = install_fixture(Channel::Preview);
        let dir = root.path().join("publication");
        match failure {
            "identity" | "status" => {
                let mut report: Value =
                    serde_json::from_slice(&fs::read(dir.join("receipt.json")).unwrap()).unwrap();
                if failure == "identity" {
                    report["identity"]["source_commit"] = json!("c".repeat(40));
                } else {
                    report["status"] = json!("failed");
                }
                fixture::write(&dir, "receipt.json", &report);
            }
            "asset" => fs::write(dir.join("release/install.sh"), b"changed").unwrap(),
            "original" => fs::write(dir.join("empty"), b"changed original").unwrap(),
            "existing-host" => fs::create_dir(&commands.host).unwrap(),
            _ => unreachable!(),
        }
        assert!(
            accept(root.path(), &id, &mut commands).is_err(),
            "{failure}"
        );
        assert!(commands.calls.is_empty(), "{failure}");
    }
}

#[test]
fn dependency_selection_keeps_the_original_attempt_and_rejects_current_failures() {
    let valid = json!({"native_collect":{"result":"success","outputs":{"run_id":"20","attempt":"1","artifact_id":"30"}}});
    assert_eq!(
        delivery::select("native_collect", &valid, "20", "2").unwrap(),
        selected()
    );
    for (path, value) in [
        ("result", json!("failure")),
        ("result", json!("cancelled")),
        ("result", json!("skipped")),
        ("run_id", json!("21")),
        ("attempt", json!("3")),
        ("artifact_id", json!("0")),
    ] {
        let mut invalid = valid.clone();
        if path == "result" {
            invalid["native_collect"][path] = value;
        } else {
            invalid["native_collect"]["outputs"][path] = value;
        }
        assert!(
            delivery::select("native_collect", &invalid, "20", "2").is_err(),
            "{path}"
        );
    }
}

fn package_fixture() -> (fixture::Fixture, Loaded, Package, Value) {
    let f = fixture::Fixture::new();
    let loaded = Loaded::load(f.root()).unwrap();
    let package = Package {
        platform: "linux-x86_64".into(),
        directory: "package".into(),
    };
    let root = f.root().join("package");
    fs::create_dir(&root).unwrap();
    let mut members = BTreeMap::new();
    let mut observed = BTreeMap::new();
    for asset in ["a", "b"] {
        let name = format!("{asset}-linux-x86_64");
        let bytes = format!("fixture executable {asset}").into_bytes();
        fs::write(root.join(&name), &bytes).unwrap();
        members.insert(
            asset,
            json!({"file":name,"sha256":chrono_harness::sha256(&bytes),"size":bytes.len()}),
        );
        observed.insert(asset,json!({"path":name,"sha256":chrono_harness::sha256(&bytes),"size":bytes.len(),"mode":493}));
    }
    fixture::write(
        &root,
        "release.json",
        &json!({"schema":"chrono-release/v1","version":identity().version,"source_commit":identity().source_commit,"source_tree":identity().source_tree,"platforms":{"linux-x86_64":members}}),
    );
    fs::write(root.join("empty"), b"").unwrap();
    let empty = json!({"path":"empty","size":0,"sha256":chrono_harness::sha256(b"")});
    let mut originals = BTreeMap::new();
    for id in ["build_a", "build_b", "test_none", "test_a"] {
        let dir = root.join("originals").join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("receipt.json"), b"original receipt").unwrap();
        originals.insert(id,json!({"receipt.json":{"path":"receipt.json","size":16,"sha256":chrono_harness::sha256(b"original receipt")}}));
    }
    let report = json!({"schema":"chrono-release-unit/v1","status":"passed","failure":null,"started_ns":1,"finished_ns":2,"unit":serde_json::to_value(&loaded.recipe.collector).unwrap(),"lineage":{"kind":"github","run_id":"20","attempt":"1","job":"native_collect"},"context":{"source_commit":identity().source_commit,"source_tree":identity().source_tree,"release_identity":identity()},"package":{"release_sha256":chrono_harness::file_identity(&root.join("release.json")).unwrap().0,"assets":observed},"originals":originals,"processes":[{"exit_code":0,"stdout":empty,"stderr":empty}]});
    fixture::write(&root, "receipt.json", &report);
    (f, loaded, package, report)
}

#[test]
fn consumes_exact_collector_source_lineage_originals_and_asset_bytes() {
    let (f, loaded, package, _) = package_fixture();
    delivery::verify_package(f.root(), &loaded, &package, &selected(), &identity()).unwrap();
    fs::write(f.root().join("package/a-linux-x86_64"), b"changed").unwrap();
    assert!(
        delivery::verify_package(f.root(), &loaded, &package, &selected(), &identity()).is_err()
    );
}

#[test]
fn passing_label_cannot_hide_bad_identity_originals_or_failed_processes() {
    for case in 0..7 {
        let (f, loaded, package, mut report) = package_fixture();
        match case {
            0 => report["context"]["release_identity"]["verified_ci"]["attempt"] = json!(2),
            1 => report["lineage"]["attempt"] = json!("2"),
            2 => report["processes"][0]["exit_code"] = json!(17),
            3 => {
                fs::write(
                    f.root().join("package/originals/build_a/receipt.json"),
                    b"changed",
                )
                .unwrap();
            }
            4 => report["package"]["release_sha256"] = json!("c".repeat(64)),
            5 => report["originals"]
                .as_object_mut()
                .unwrap()
                .remove("test_a")
                .map(|_| ())
                .unwrap(),
            6 => report["finished_ns"] = Value::Null,
            _ => unreachable!(),
        }
        fixture::write(&f.root().join("package"), "receipt.json", &report);
        assert!(
            delivery::verify_package(f.root(), &loaded, &package, &selected(), &identity())
                .is_err(),
            "{case}"
        );
    }
}
