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
        "processes":[{"exit_code":0,"stdout":empty,"stderr":empty,
            "observation":{"failure":null,"exit_signal":null,"engine_exit_code":0}}],
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
        "example/product",
        "https://example.invalid/fixed/install.sh",
        commands,
    )
}

fn missing_lookup(root: &Path, id: &Identity, release: bool) {
    let directory = root.join("publication");
    let file = if release { "release" } else { "tag" };
    let stdout: &[u8] = if release {
        include_bytes!("data/github-release-not-found.json")
    } else {
        include_bytes!("data/tag-not-found.json")
    };
    let stderr = include_bytes!("data/github-not-found.stderr");
    let original = |stream: &str, bytes: &[u8]| {
        let path = format!("{file}.{stream}");
        fs::write(directory.join(&path), bytes).unwrap();
        json!({"path":path,"size":bytes.len(),"sha256":chrono_harness::sha256(bytes)})
    };
    let endpoint = format!(
        "repos/example/product/{}/{}",
        if release {
            "releases/tags"
        } else {
            "git/ref/tags"
        },
        id.version
    );
    let mut report: Value =
        serde_json::from_slice(&fs::read(directory.join("receipt.json")).unwrap()).unwrap();
    report["processes"].as_array_mut().unwrap().push(json!({
        "tool":"gh","argv":["/usr/bin/gh","api","--method","GET",endpoint],
        "exit_code":1,"stdout":original("stdout",stdout),"stderr":original("stderr",stderr),
        "observation":{"failure":null,"exit_signal":null,"engine_exit_code":1,"executable":"/usr/bin/gh"}
    }));
    fixture::write(&directory, "receipt.json", &report);
}

#[test]
fn first_publication_missing_release_and_tag_can_be_installed() {
    for channel in [Channel::Preview, Channel::Stable] {
        let (root, id, mut commands) = install_fixture(channel);
        missing_lookup(root.path(), &id, true);
        missing_lookup(root.path(), &id, false);
        let installed = accept(root.path(), &id, &mut commands).unwrap();
        assert_eq!(installed["source_commit"], id.source_commit);
        assert_eq!(commands.calls.len(), 2);
        assert_eq!(commands.calls[0].0, "/usr/bin/curl");
    }
}

#[test]
fn only_bound_missing_queries_with_valid_originals_allow_nonzero_exit() {
    for failure in [
        "upload",
        "method",
        "repository",
        "version",
        "endpoint",
        "arguments",
        "tool",
        "executable",
        "relative-executable",
        "403",
        "500",
        "numeric-status",
        "message",
        "malformed",
        "exit",
        "unknown-exit",
        "engine-exit",
        "signal",
        "engine-failure",
        "incomplete",
        "stdout",
        "stderr",
    ] {
        let (root, id, mut commands) = install_fixture(Channel::Preview);
        missing_lookup(root.path(), &id, true);
        let directory = root.path().join("publication");
        let mut report: Value =
            serde_json::from_slice(&fs::read(directory.join("receipt.json")).unwrap()).unwrap();
        let process = report["processes"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap();
        let mut response: Value =
            serde_json::from_slice(include_bytes!("data/github-release-not-found.json")).unwrap();
        match failure {
            "upload" => {
                process["argv"] = json!([
                    "/usr/bin/gh",
                    "release",
                    "upload",
                    id.version,
                    "asset",
                    "--repo",
                    "example/product"
                ])
            }
            "method" => process["argv"][3] = json!("POST"),
            "repository" => {
                process["argv"][4] =
                    json!(format!("repos/other/product/releases/tags/{}", id.version))
            }
            "version" => process["argv"][4] = json!("repos/example/product/releases/tags/other"),
            "endpoint" => process["argv"][4] = json!("repos/example/product/releases/123/assets"),
            "arguments" => process["argv"]
                .as_array_mut()
                .unwrap()
                .push(json!("--paginate")),
            "tool" => process["tool"] = json!("git"),
            "executable" => process["observation"]["executable"] = json!("/other/gh"),
            "relative-executable" => {
                process["argv"][0] = json!("gh");
                process["observation"]["executable"] = json!("gh");
            }
            "403" | "500" => response["status"] = json!(failure),
            "numeric-status" => response["status"] = json!(404),
            "message" => response["message"] = json!("Forbidden"),
            "malformed" | "stdout" | "stderr" => {}
            "exit" => {
                process["exit_code"] = json!(2);
                process["observation"]["engine_exit_code"] = json!(2);
            }
            "unknown-exit" => process["exit_code"] = Value::Null,
            "engine-exit" => process["observation"]["engine_exit_code"] = json!(101),
            "signal" => process["observation"]["exit_signal"] = json!(15),
            "engine-failure" => process["observation"]["failure"] = json!("original timeout"),
            "incomplete" => process["stdout"]["status"] = json!("incomplete"),
            _ => unreachable!(),
        }
        let bytes = if failure == "malformed" {
            b"not JSON".to_vec()
        } else {
            serde_json::to_vec(&response).unwrap()
        };
        fs::write(directory.join("release.stdout"), &bytes).unwrap();
        process["stdout"]["size"] = json!(bytes.len());
        process["stdout"]["sha256"] = json!(chrono_harness::sha256(&bytes));
        if ["stdout", "stderr"].contains(&failure) {
            fs::write(
                directory.join(format!("release.{failure}")),
                b"changed original",
            )
            .unwrap();
        }
        fixture::write(&directory, "receipt.json", &report);
        assert!(
            accept(root.path(), &id, &mut commands).is_err(),
            "{failure}"
        );
        assert!(commands.calls.is_empty(), "{failure}");
        assert!(!commands.host.exists(), "{failure}");
    }
    let (root, id, mut commands) = install_fixture(Channel::Build);
    missing_lookup(root.path(), &id, true);
    assert!(accept(root.path(), &id, &mut commands).is_err());
    assert!(commands.calls.is_empty());
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
    let report = json!({"schema":"chrono-release-unit/v1","status":"passed","failure":null,"started_ns":1,"finished_ns":2,"platform_started_ns":1,"platform_span_ns":1,"unit":serde_json::to_value(&loaded.recipe.collector).unwrap(),"lineage":{"kind":"github","run_id":"20","attempt":"1","job":"native_collect"},"context":{"source_commit":identity().source_commit,"source_tree":identity().source_tree,"release_identity":identity()},"package":{"release_sha256":chrono_harness::file_identity(&root.join("release.json")).unwrap().0,"assets":observed},"originals":originals,"processes":[{"exit_code":0,"stdout":empty,"stderr":empty}]});
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
    for case in 0..10 {
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
            7 => report["platform_started_ns"] = Value::Null,
            8 => report["platform_span_ns"] = json!(2),
            9 => {
                report["finished_ns"] = json!(121_000_000_001u64);
                report["platform_span_ns"] = json!(121_000_000_000u64);
            }
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

mod promotion_tests {
    use super::fixture;
    use chrono_host_release::{
        delivery,
        delivery::promotion,
        execution::Environment,
        identity::{Channel, Identity, VerifiedCi},
        publication::{Commands, Reply},
        recipe::Loaded,
    };
    use serde_json::{Value, json};
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
    };

    fn stable() -> Identity {
        Identity {
            channel: Channel::Stable,
            version: "v1.2.3".into(),
            source_commit: "a".repeat(40),
            source_tree: "b".repeat(40),
            verified_ci: Some(VerifiedCi {
                run_id: 10,
                attempt: 1,
            }),
        }
    }
    fn preview() -> Identity {
        Identity {
            channel: Channel::Preview,
            version: format!("v1.2.3-dev.10.1.g{}", "a".repeat(40)),
            ..stable()
        }
    }
    fn run() -> Value {
        json!({"id":20,"run_attempt":2,"workflow_id":100,"path":".github/workflows/release.yml","repository":{"full_name":"example/product"},"status":"completed","conclusion":"success","event":"workflow_run","head_sha":"c".repeat(40)})
    }
    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for row in fs::read_dir(from).unwrap() {
            let row = row.unwrap();
            let target = to.join(row.file_name());
            if row.file_type().unwrap().is_dir() {
                copy(&row.path(), &target)
            } else {
                fs::copy(row.path(), target).unwrap();
            }
        }
    }
    struct Mock {
        originals: PathBuf,
        run: Value,
        release: Value,
        assets: Vec<Value>,
        bad: &'static str,
        calls: Vec<(String, Vec<String>)>,
    }
    impl Commands for Mock {
        fn body(&mut self, _: &Value) -> Result<String, String> {
            panic!("promotion must not mutate GitHub")
        }
        fn execute(&mut self, program: &str, args: &[String]) -> Result<Reply, String> {
            self.calls.push((program.into(), args.to_vec()));
            let reply = |v: Value| {
                Ok(Reply {
                    exit: 0,
                    stdout: serde_json::to_vec(&v).unwrap(),
                })
            };
            if program == "git" {
                assert_eq!(
                    &args[..4],
                    [
                        "--no-replace-objects",
                        "rev-parse",
                        "--verify",
                        "--end-of-options"
                    ]
                );
                assert!(args[4].ends_with(":.github/workflows/release.yml"));
                return Ok(Reply {
                    exit: 0,
                    stdout: format!(
                        "{}\n",
                        if self.bad == "workflow" && args[4].starts_with('a') {
                            "e".repeat(40)
                        } else {
                            "d".repeat(40)
                        }
                    )
                    .into_bytes(),
                });
            }
            assert_eq!(program, "gh");
            if args[0] == "run" {
                assert_eq!(
                    &args[..5],
                    ["run", "download", "20", "--repo", "example/product"]
                );
                assert_eq!(args[5], "--name");
                assert_eq!(args[7], "--dir");
                assert_eq!(args.len(), 9);
                if self.bad == "download" {
                    return Ok(Reply {
                        exit: 17,
                        stdout: b"original download failed".to_vec(),
                    });
                }
                let source = if args[6] == "publication-20-2" {
                    "publication"
                } else {
                    assert_eq!(args[6], "package-20-1");
                    "package"
                };
                copy(&self.originals.join(source), Path::new(&args[8]));
                return Ok(Reply {
                    exit: 0,
                    stdout: vec![],
                });
            }
            assert_eq!(&args[..3], ["api", "--method", "GET"]);
            let path = &args[3];
            if path.ends_with("/actions/runs/20/attempts/2") {
                return reply(self.run.clone());
            }
            if path.contains("/artifacts?") {
                let name = if path.contains("name=publication-") {
                    "publication-20-2"
                } else {
                    assert!(path.contains("name=package-20-1"));
                    "package-20-1"
                };
                return reply(
                    json!({"total_count":1,"artifacts":[{"id":if name.starts_with("publication") {32} else if self.bad=="artifact" {31} else {30},"name":name,"expired":self.bad=="expired","size_in_bytes":123,"workflow_run":{"id":20,"head_sha":"c".repeat(40)}}]}),
                );
            }
            if path.ends_with("/actions/runs/10/attempts/1") {
                return reply(
                    json!({"id":10,"run_attempt":1,"workflow_id":99,"path":".github/workflows/ci.yml","event":"push","head_branch":"dev","head_sha":if self.bad=="ci-source" {"e".repeat(40)} else {"a".repeat(40)},"status":"completed","conclusion":"success","repository":{"full_name":"example/product"}}),
                );
            }
            if path.contains("/releases/tags/") {
                return reply(self.release.clone());
            }
            if path.ends_with("/releases/7/assets?per_page=100") {
                return reply(json!(self.assets));
            }
            if path.contains("/git/ref/tags/") {
                return reply(
                    json!({"object":{"type":"commit","sha":if self.bad=="tag" {"e".repeat(40)} else {"a".repeat(40)}}}),
                );
            }
            panic!("unexpected {program} {args:?}");
        }
    }

    fn setup() -> (
        fixture::Fixture,
        Loaded,
        delivery::Config,
        Environment,
        Mock,
    ) {
        let mut f = fixture::Fixture::new();
        f.plan["version"] = json!("v1.2.3");
        f.save();
        let loaded = Loaded::load(f.root()).unwrap();
        let config:delivery::Config=serde_json::from_value(json!({
        "schema":"chrono-release-delivery/v1","publication":{"repository":"example/product","workflow_id":99,"workflow_path":".github/workflows/ci.yml","branch":"dev"},
        "frontend":"fixture","packages":{"fixture":{"platform":"linux-x86_64","directory":"unused"}},"tools":{"git":"git","gh":"gh"},"curl":"/usr/bin/curl",
        "installation":{"publisher":"publish","directory":"publication/","shell":"/bin/sh"},"promotion":{"workflow_id":100,"workflow_config":"workflow.json"}
    })).unwrap();
        fixture::write(
            f.root(),
            "workflow.json",
            &json!({"workflow_path":".github/workflows/release.yml","delivery":{"publisher":"publish"},"jobs":[{"id":"publish","artifact_name":"publication"},{"id":"native_collect","artifact_name":"package"}]}),
        );
        let originals = f.root().join("originals");
        let package = originals.join("package");
        let publication = originals.join("publication");
        fs::create_dir_all(publication.join("release")).unwrap();
        fs::create_dir_all(&package).unwrap();
        let mut tools = BTreeMap::new();
        let mut observed = BTreeMap::new();
        for name in ["a", "b"] {
            let file = format!("{name}-linux-x86_64");
            let bytes = format!("original tool {name}").into_bytes();
            for dest in [&package, &publication.join("release")] {
                fs::write(dest.join(&file), &bytes).unwrap();
            }
            let sha = chrono_harness::sha256(&bytes);
            tools.insert(name, json!({"file":file,"sha256":sha,"size":bytes.len()}));
            observed.insert(
                name,
                json!({"path":file,"sha256":sha,"size":bytes.len(),"mode":493}),
            );
        }
        let manifest = json!({"schema":"chrono-release/v1","version":preview().version,"source_commit":stable().source_commit,"source_tree":stable().source_tree,"platforms":{"linux-x86_64":tools}});
        for dest in [&package, &publication.join("release")] {
            fixture::write(dest, "release.json", &manifest);
        }
        let mut retained = BTreeMap::new();
        for id in ["build_a", "build_b", "test_none", "test_a"] {
            let dir = package.join("originals").join(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("receipt.json"), b"original receipt").unwrap();
            retained.insert(id,json!({"receipt.json":{"path":"receipt.json","sha256":chrono_harness::sha256(b"original receipt"),"size":16}}));
        }
        let empty = json!({"path":"empty","sha256":chrono_harness::sha256(b""),"size":0});
        for dest in [&package, &publication] {
            fs::write(dest.join("empty"), b"").unwrap();
        }
        fixture::write(
            &package,
            "receipt.json",
            &json!({"schema":"chrono-release-unit/v1","status":"passed","failure":null,"started_ns":1,"finished_ns":2,"platform_started_ns":1,"platform_span_ns":1,
        "unit":loaded.recipe.collector,"lineage":{"kind":"github","run_id":"20","attempt":"1","job":"native_collect"},
        "context":{"source_commit":preview().source_commit,"source_tree":preview().source_tree,"release_identity":preview()},"package":{"release_sha256":chrono_harness::file_identity(&package.join("release.json")).unwrap().0,"assets":observed},"originals":retained,"processes":[{"exit_code":0,"stdout":empty,"stderr":empty}]}),
        );
        fs::write(publication.join("release/install.sh"), b"original launcher").unwrap();
        let mut assets = BTreeMap::new();
        let mut remote = Vec::new();
        for name in [
            "release.json",
            "install.sh",
            "a-linux-x86_64",
            "b-linux-x86_64",
        ] {
            let (sha, size) =
                chrono_harness::file_identity(&publication.join("release").join(name)).unwrap();
            assets.insert(
                name,
                json!({"path":format!("old-output/{name}"),"sha256":sha,"size":size}),
            );
            remote.push(json!({"name":name,"digest":format!("sha256:{sha}"),"size":size,"state":"uploaded"}));
        }
        let release = json!({"id":7,"tag_name":preview().version,"target_commitish":stable().source_commit,"draft":false,"prerelease":true});
        fixture::write(
            &publication,
            "receipt.json",
            &json!({"schema":"chrono-release-publication/v1","status":"passed","identity":preview(),
        "lineage":{"kind":"github","run_id":"20","attempt":"2","job":"publish"},"dependencies":{"native_collect":{"result":"success","outputs":{"artifact_id":"30","run_id":"20","attempt":"1"}}},
        "processes":[{"exit_code":0,"stdout":empty,"stderr":empty,"observation":{"failure":null,"exit_signal":null,"engine_exit_code":0}}],"result":{"release":release,"assets":assets}}),
        );
        let environment = Environment::from([
            ("CHRONO_RELEASE_CHANNEL".into(), "stable".into()),
            (
                "CHRONO_RELEASE_SOURCE".into(),
                stable().source_commit.into(),
            ),
            ("CHRONO_RELEASE_CI_RUN_ID".into(), "10".into()),
            ("CHRONO_RELEASE_CI_ATTEMPT".into(), "1".into()),
            ("CHRONO_RELEASE_VERSION".into(), "v1.2.3".into()),
            ("CHRONO_RELEASE_RUN".into(), "40".into()),
            ("CHRONO_RELEASE_PREVIEW_RUN".into(), "20".into()),
            ("CHRONO_RELEASE_PREVIEW_ATTEMPT".into(), "2".into()),
        ]);
        let mock = Mock {
            originals,
            run: run(),
            release,
            assets: remote,
            bad: "none",
            calls: vec![],
        };
        (f, loaded, config, environment, mock)
    }

    #[test]
    fn stable_promotion_preserves_preview_identity_and_actual_producing_attempt() {
        let (f, loaded, config, environment, mut mock) = setup();
        let output = f.root().join("out");
        fs::create_dir(&output).unwrap();
        let original = fs::read(mock.originals.join("package/receipt.json")).unwrap();
        let prepared = promotion::prepare(
            f.root(),
            &output,
            &config,
            &loaded,
            &stable(),
            &environment,
            &mut mock,
        )
        .unwrap();
        assert_eq!(prepared.identity, preview());
        assert_eq!(prepared.selections["fixture"].attempt, "1");
        let package = &prepared.packages["fixture"];
        delivery::verify_package(
            f.root(),
            &loaded,
            package,
            &prepared.selections["fixture"],
            &preview(),
        )
        .unwrap();
        assert_eq!(
            fs::read(f.root().join(&package.directory).join("receipt.json")).unwrap(),
            original
        );
        assert_eq!(prepared.observation["run"]["run_attempt"], 2);
        assert_eq!(
            prepared.observation["workflow"]["source_commit"],
            stable().source_commit
        );
        assert_eq!(
            prepared.observation["workflow"]["workflow_commit"],
            "c".repeat(40)
        );
        assert_eq!(
            prepared.observation["package_artifacts"]["fixture"]["id"],
            30
        );
    }

    #[test]
    fn promotion_rejects_failed_or_unbound_origins_without_mutation_or_rebuild() {
        for bad in [
            "run",
            "attempt",
            "workflow",
            "expired",
            "artifact",
            "download",
            "identity",
            "process",
            "bytes",
            "dependency",
            "public-assets",
            "draft",
            "tag",
            "ci-source",
            "missing-input",
        ] {
            let (f, loaded, config, mut environment, mut mock) = setup();
            mock.bad = bad;
            let receipt_path = mock.originals.join("publication/receipt.json");
            let mut receipt: Value =
                serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
            match bad {
                "run" => mock.run["conclusion"] = json!("failure"),
                "attempt" => mock.run["run_attempt"] = json!(1),
                "identity" => receipt["identity"]["source_tree"] = json!("e".repeat(40)),
                "process" => receipt["processes"][0]["observation"]["engine_exit_code"] = json!(17),
                "bytes" => fs::write(
                    mock.originals.join("publication/release/a-linux-x86_64"),
                    b"corrupt",
                )
                .unwrap(),
                "dependency" => {
                    receipt["dependencies"]["native_collect"]["outputs"]["attempt"] = json!("3")
                }
                "public-assets" => {
                    mock.assets[0]["digest"] = json!(format!("sha256:{}", "e".repeat(64)))
                }
                "draft" => mock.release["draft"] = json!(true),
                "missing-input" => {
                    environment.remove(std::ffi::OsStr::new("CHRONO_RELEASE_PREVIEW_RUN"));
                }
                _ => {}
            }
            fixture::write(
                mock.originals.join("publication").as_path(),
                "receipt.json",
                &receipt,
            );
            let output = f.root().join("out");
            fs::create_dir(&output).unwrap();
            let error = promotion::prepare(
                f.root(),
                &output,
                &config,
                &loaded,
                &stable(),
                &environment,
                &mut mock,
            )
            .err()
            .unwrap_or_else(|| panic!("accepted {bad}"));
            assert!(!error.is_empty(), "{bad}");
            assert!(
                mock.calls
                    .iter()
                    .all(|(p, a)| p == "git" || (p == "gh" && (a[0] == "run" || a[2] == "GET"))),
                "{bad}: {:?}",
                mock.calls
            );
        }
    }
}
