#![allow(dead_code)]
#[path = "../../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::sha256;
use chrono_judge_registration::Registrations;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
pub use support::*;

pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub values: Values,
    pub contract: Value,
}
pub fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn rust_tool(name: &str) -> PathBuf {
    let o = Command::new("rustup")
        .args(["which", "--toolchain", "1.95.0", name])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    PathBuf::from(String::from_utf8(o.stdout).unwrap().trim())
}
impl Fixture {
    pub fn new() -> Self {
        Self::new_in(None)
    }
    pub fn new_in(parent: Option<&Path>) -> Self {
        let mut builder = tempfile::Builder::new();
        builder.prefix("cargo declared host ");
        let dir = match parent {
            Some(path) => builder.tempdir_in(path),
            None => builder.tempdir(),
        }
        .unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let cargo = rust_tool("cargo");
        let rustc = rust_tool("rustc");
        let o = Command::new(&rustc).arg("-vV").output().unwrap();
        assert!(o.status.success());
        let target = String::from_utf8(o.stdout)
            .unwrap()
            .lines()
            .find_map(|l| l.strip_prefix("host: "))
            .unwrap()
            .to_string();
        let mut v = values();
        v.get_mut(FM).unwrap()["schema_version"] = 2.into();
        v.get_mut(FM).unwrap()["files"] = json!([]);
        v.get_mut(FM).unwrap()["project_edges"] = json!([
            edge("project:p", "compile", "project:t"),
            edge("project:p", "test-execution", "test:t"),
            edge("project:t", "test-execution", "test:t")
        ]);
        let version = Command::new(&cargo).arg("--version").output().unwrap();
        assert!(version.status.success());
        v.get_mut(CONFIG).unwrap()["tools"] = json!([{"id":"cargo","program":cargo,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}]);
        v.get_mut(CONFIG).unwrap()["environment"] = json!({"inherit":["PATH"],"values":{"RUSTC":rustc,"CARGO_HOME":root.join(".chrono-harness/state/cargo-home"),"CARGO_TERM_COLOR":"never","CHRONO_TEST_EFFECT":root.join(".chrono-harness/state/test-ran")},"inputs":[]});
        write(
            &root,
            ".chrono-harness/cargo/config.toml",
            &format!(
                "[source.crates-io]\nreplace-with='fixtures'\n[source.fixtures]\ndirectory='{}'\n",
                root.join(".chrono-harness/state/vendor").display()
            ),
        );
        let mut external = Vec::new();
        for (id, name, body, lib, checksum) in [
            (
                "dep",
                "fixture-dep",
                "[dependencies]\nfixture-leaf='=1.0.0'\n[features]\nbonus=['fixture-leaf/enabled']\n",
                "pub fn value()->u32 { fixture_leaf::value() }\n",
                "a".repeat(64),
            ),
            (
                "leaf",
                "fixture-leaf",
                "[features]\nenabled=[]\n",
                "pub fn value()->u32 { if cfg!(feature=\"enabled\") {42} else {0} }\n",
                "b".repeat(64),
            ),
        ] {
            let prefix = format!(".chrono-harness/state/vendor/{name}");
            let manifest =
                format!("[package]\nname='{name}'\nversion='1.0.0'\nedition='2021'\n{body}");
            write(&root, &format!("{prefix}/Cargo.toml"), &manifest);
            write(&root, &format!("{prefix}/src/lib.rs"), lib);
            write(&root,&format!("{prefix}/.cargo-checksum.json"),&json!({"package":checksum,"files":{"Cargo.toml":sha256(manifest.as_bytes()),"src/lib.rs":sha256(lib.as_bytes())}}).to_string());
            let mut ids = Vec::new();
            for (suffix, path) in [
                ("manifest", "Cargo.toml"),
                ("lib", "src/lib.rs"),
                ("checksums", ".cargo-checksum.json"),
            ] {
                let input = format!("{id}.{suffix}");
                let location = format!("{prefix}/{path}");
                v.get_mut(CONFIG).unwrap()["environment"]["inputs"].as_array_mut().unwrap().push(json!({"id":input,"location":location,"sha256":sha256(&fs::read(root.join(&location)).unwrap())}));
                v.get_mut(FM).unwrap()["project_edges"]
                    .as_array_mut()
                    .unwrap()
                    .push(edge(
                        &format!("input:{input}"),
                        "runtime-input",
                        "project:p",
                    ));
                ids.push(input);
            }
            external.push(json!({"id":id,"name":name,"version":"1.0.0","source":"registry+https://github.com/rust-lang/crates.io-index","project":null,"manifest_input":format!("{id}.manifest"),"inputs":ids,"checksum":checksum,"features":if id=="dep" {vec!["bonus"]} else {vec!["enabled"]},"dependencies":if id=="dep" {json!([{"package":"leaf","name":"fixture_leaf","kinds":[{"kind":null,"target":null}]}])}else{json!([])}}));
        }
        let cp = ".chrono-harness/cargo/config.toml";
        v.get_mut(CONFIG).unwrap()["environment"]["inputs"].as_array_mut().unwrap().push(json!({"id":"cargo.config","location":cp,"sha256":sha256(&fs::read(root.join(cp)).unwrap())}));
        v.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .push(edge("input:cargo.config", "runtime-input", "project:p"));
        write(
            &root,
            "p/Cargo.toml",
            "[package]\nname='p'\nversion='0.1.0'\nedition='2021'\n[dependencies]\nvalue_dep={package='fixture-dep',version='=1.0.0',features=['bonus']}\n",
        );
        write(
            &root,
            "p/src/lib.rs",
            "pub fn value()->u32 { value_dep::value()+1 }\n",
        );
        write(
            &root,
            "t/Cargo.toml",
            "[package]\nname='t'\nversion='0.1.0'\nedition='2021'\n[dev-dependencies]\np={path='../p'}\n",
        );
        write(
            &root,
            "t/src/lib.rs",
            "#[test] fn actual_external_dependency() { assert_eq!(p::value(),43); std::fs::write(std::env::var(\"CHRONO_TEST_EFFECT\").unwrap(), b\"ran\").unwrap(); }\n",
        );
        for id in ["p", "t"] {
            let lock = Command::new(&cargo)
                .current_dir(&root)
                .env("CARGO_HOME", root.join(".chrono-harness/state/cargo-home"))
                .env("RUSTC", &rustc)
                .args([
                    "generate-lockfile",
                    "--offline",
                    "--config",
                    cp,
                    "--manifest-path",
                    &format!("{id}/Cargo.toml"),
                ])
                .output()
                .unwrap();
            assert!(
                lock.status.success(),
                "{}",
                String::from_utf8_lossy(&lock.stderr)
            );
            for path in [
                format!("{id}/Cargo.toml"),
                format!("{id}/Cargo.lock"),
                format!("{id}/src/lib.rs"),
            ] {
                let mut f = file(
                    &path,
                    json!([{"kind":"compile","to":format!("project:{id}")}]),
                );
                f["owner"] = id.into();
                v.get_mut(FM).unwrap()["files"]
                    .as_array_mut()
                    .unwrap()
                    .push(f);
            }
            v.get_mut(CONFIG).unwrap()["artifacts"].as_array_mut().unwrap().push(json!({"path":format!("{id}/target/"),"owner":id,"kind":"cargo-output","tracked":false}));
        }

        let metadata = json!({"tool":"cargo","argv":["metadata","--locked","--offline","--format-version","1","--filter-platform",target,"--manifest-path","t/Cargo.toml","--config",cp]});
        let operation = json!({"tool":"cargo","argv":["test","--locked","--offline","--manifest-path","t/Cargo.toml","--target",target,"--config",cp]});
        let binary = source().join("crates/judge-cargo/target/debug/chrono-judge-cargo");
        v.get_mut(CONFIG).unwrap()["tools"].as_array_mut().unwrap().push(json!({"id":"cargo-guard","program":binary,"resolution":"PATH-once","version_argv":["--version"],"expected_version":"chrono-judge-cargo 0.1.0"}));
        v.get_mut(PROJECTS).unwrap()["projects"][1]["actions"] = json!({"execute":{"operation":"test.t","tool":"cargo-guard","argv":["run","--host-root",root,"--config",CONFIG,"--policy",POLICY,"--operation","test.t"]}});
        v.get_mut(PROJECTS).unwrap()["projects"][0]["actions"] =
            json!({"inspect":{"operation":"inspect.p","tool":"cargo","argv":["--version"]}});
        v.get_mut(FM).unwrap()["execution_plans"] = json!({"test:t":{"operations":["test.t"],"timeout_seconds":120,"output_limit_bytes":1048576}});
        let mut packages = vec![
            json!({"id":"p","name":"p","version":"0.1.0","source":null,"project":"p","manifest_input":null,"inputs":[],"checksum":null,"features":[],"dependencies":[{"package":"dep","name":"value_dep","kinds":[{"kind":null,"target":null}]}]}),
            json!({"id":"t","name":"t","version":"0.1.0","source":null,"project":"t","manifest_input":null,"inputs":[],"checksum":null,"features":[],"dependencies":[{"package":"p","name":"p","kinds":[{"kind":"dev","target":null}]}]}),
        ];
        packages.extend(external);
        let projects: Vec<_> = ["p","t"].iter().map(|id| json!({"project":id,"manifest":format!("{id}/Cargo.toml"),"lockfile":format!("{id}/Cargo.lock"),"output":format!("{id}/target/"),"ancestor_manifests":[]})).collect();
        let contract = json!({"schema":"chrono-cargo-inputs/v2","root":"t","projects":projects,"metadata":metadata,"operations":{"test.t":operation},"target":target,"configuration_files":[{"path":cp,"input":"cargo.config"}],"packages":packages,"timeout_seconds":120,"output_limit_bytes":1048576});
        for path in [cp, POLICY] {
            let mut row = file(path, json!([{"kind":"build-input","to":"project:t"}]));
            row["owner"] = "t".into();
            row["surface"] = "judge-policy".into();
            v.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .push(row);
        }
        let mut f = Self {
            dir,
            values: v,
            contract,
        };
        for directory in root
            .ancestors()
            .map(|p| p.join(".cargo"))
            .chain([root.join(".chrono-harness/state/cargo-home")])
        {
            for name in ["config", "config.toml"] {
                f.configuration_absent(&directory.join(name));
            }
        }
        f.save();
        f
    }
    pub fn configuration_absent(&mut self, path: &Path) {
        let root = self.root();
        let rows = self.contract["configuration_files"].as_array_mut().unwrap();
        rows.retain(|v| root.join(v["path"].as_str().unwrap()) != path);
        rows.push(json!({"path":path,"input":null}));
    }
    pub fn configuration(&mut self, id: &str, path: &Path, contents: &str) {
        write(&self.root(), path.to_str().unwrap(), contents);
        let root = self.root();
        let rows = self.contract["configuration_files"].as_array_mut().unwrap();
        rows.retain(|v| root.join(v["path"].as_str().unwrap()) != path);
        rows.push(json!({"path":path,"input":id}));
        let inputs = self.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
            .as_array_mut()
            .unwrap();
        inputs.retain(|v| v["id"] != id);
        inputs.push(json!({"id":id,"location":path,"sha256":sha256(contents.as_bytes())}));
        let edges = self.values.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap();
        let from = format!("input:{id}");
        edges.retain(|v| v["from"] != from);
        edges.push(edge(&from, "runtime-input", "project:t"));
    }
    pub fn root(&self) -> PathBuf {
        fs::canonicalize(self.dir.path()).unwrap()
    }
    pub fn save(&self) {
        write_values(&self.root(), &self.values);
        write(
            &self.root(),
            POLICY,
            &serde_json::to_string_pretty(&self.contract).unwrap(),
        );
    }
    pub fn call(&self) -> (i32, Value, String) {
        Registrations::load(&self.values, CONFIG).unwrap();
        let binary = source().join("crates/judge-cargo/target/debug/chrono-judge-cargo");
        let out = Command::new(binary)
            .current_dir("/")
            .args([
                "run",
                "--host-root",
                self.root().to_str().unwrap(),
                "--config",
                CONFIG,
                "--policy",
                POLICY,
                "--operation",
                "test.t",
            ])
            .output()
            .unwrap();
        (
            out.status.code().unwrap(),
            serde_json::from_slice(&out.stdout).unwrap_or(Value::Null),
            String::from_utf8_lossy(&out.stderr).into(),
        )
    }
    pub fn committed_check(&mut self, change: &str, consumer: Consumer) -> (i32, Value) {
        let root = self.root();
        for (id, index) in [("tool.cargo", 0), ("tool.guard", 1)] {
            let path = PathBuf::from(
                self.values[CONFIG]["tools"][index]["program"]
                    .as_str()
                    .unwrap(),
            );
            let (digest, _) = chrono_harness::file_identity(&path).unwrap();
            self.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
                .as_array_mut()
                .unwrap()
                .push(json!({"id":id,"location":path,"sha256":digest}));
            self.values.get_mut(FM).unwrap()["project_edges"]
                .as_array_mut()
                .unwrap()
                .push(edge(&format!("input:{id}"), "runtime-input", "project:t"));
        }
        git(&root, &["init", "-q"]);
        git(&root, &["checkout", "-b", "integration/cargo"]);
        write(
            &root,
            ".gitignore",
            ".chrono-harness/bin/\n.chrono-harness/state/\np/target/\nt/target/\n",
        );
        write(&root, "doc.txt", "old documentation\n");
        for path in [
            CONFIG,
            FM,
            PROJECTS,
            JUDGES,
            WORKFLOW,
            ".gitignore",
            "doc.txt",
        ] {
            self.values.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .push(file(path, json!([])));
        }
        self.values.get_mut(CONFIG).unwrap()["canonical_check"]["argv"] = json!([
            ".chrono-harness/bin/chrono-harness",
            "check",
            "--config",
            CONFIG,
            "--base",
            "{base}",
            "--candidate",
            "{candidate}",
            "--context",
            ".chrono-harness/state/context.json"
        ]);
        self.values.get_mut(CONFIG).unwrap()["protocol"]["timeout_seconds"] = 120.into();
        self.values.get_mut(CONFIG).unwrap()["protocol"]["stdout_limit_bytes"] = 67108864.into();
        self.values.get_mut(JUDGES).unwrap()["judges"] = json!([]);
        self.values.get_mut(JUDGES).unwrap()["migration_validator"] = "workflow".into();
        fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
        for (id, deps) in [
            ("runner", vec![]),
            ("registration", vec![]),
            ("filemap", vec!["registration"]),
            ("routes", vec!["registration", "filemap"]),
            ("projects", vec!["registration", "filemap", "routes"]),
            ("cost", vec!["registration", "filemap"]),
            ("mixed", vec!["registration", "filemap", "cost"]),
            (
                "workflow",
                vec![
                    "registration",
                    "filemap",
                    "routes",
                    "projects",
                    "cost",
                    "mixed",
                ],
            ),
        ] {
            let (crate_name, binary) = if id == "runner" {
                ("runner".to_string(), "chrono-harness".to_string())
            } else {
                (format!("judge-{id}"), format!("chrono-judge-{id}"))
            };
            let path = format!(".chrono-harness/bin/{binary}");
            let original = source().join(format!("crates/{crate_name}/target/debug/{binary}"));
            fs::copy(original, root.join(&path)).unwrap();
            let digest = sha256(&fs::read(root.join(&path)).unwrap());
            if id == "runner" {
                self.values.get_mut(CONFIG).unwrap()["runner"]["sha256"] = digest.into();
            } else {
                self.values.get_mut(JUDGES).unwrap()["judges"].as_array_mut().unwrap().push(json!({"id":id,"executable":path,"version":"0.1.0","sha256":digest,"argv":["--protocol","chrono-judge/v1"],"selector":"every-delta","after":deps,"modes":["evaluate"]}));
            }
        }
        let w = self.values.get_mut(WORKFLOW).unwrap();
        w["integration"]["bind"] = json!([
            "base",
            "candidate_tree",
            "registry_digest",
            "executables",
            "tools",
            "environment",
            "effective_inputs",
            "required_tests",
            "results_digest"
        ]);
        w["integration"]["tests"] = json!(["t"]);
        let profile = ".chrono-harness/scoped.json";
        if consumer != Consumer::Full {
            fs::copy(
                source().join("crates/judge-ci/target/debug/chrono-judge-ci"),
                root.join(".chrono-harness/bin/chrono-judge-ci"),
            )
            .unwrap();
            self.values.get_mut(CONFIG).unwrap()["canonical_check"]["argv"] = json!([
                ".chrono-harness/bin/chrono-harness",
                "check",
                "--config",
                profile,
                "--base",
                "{base}",
                "--candidate",
                "{candidate}"
            ]);
            let artifacts: Vec<_> = self.values[CONFIG]["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a["path"].clone())
                .collect();
            let configuration = json!({"schema":"chrono-ci-check/v1","judge":{"program":".chrono-harness/bin/chrono-judge-ci","args":[],"timeout_seconds":180,"output_limit_bytes":8388608},"report_path":".chrono-harness/state/scoped.json","policy":{"filemap":FM,"projects":PROJECTS,"registration_config":CONFIG,"tools":{"cargo":self.values[CONFIG]["tools"][0]["program"],"cargo-guard":self.values[CONFIG]["tools"][1]["program"]},"artifacts":artifacts,"required_inputs":[profile],"operation_timeout_seconds":120,"operation_output_limit_bytes":1048576,"environment":{}}});
            write(&root, profile, &configuration.to_string());
            self.values.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .push(file(profile, json!([])));
        }
        self.save();
        write_values(&root, &self.values);
        let base = commit(&root);
        let files: serde_json::Map<_, _> = self.values[CONFIG]["environment"]["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let original = root.join(v["location"].as_str().unwrap());
                let (digest, length) = chrono_harness::file_identity(&original).unwrap();
                let blob = format!(".chrono-harness/state/inputs/blobs/{digest}");
                fs::create_dir_all(root.join(&blob).parent().unwrap()).unwrap();
                fs::copy(original, root.join(&blob)).unwrap();
                (
                    v["id"].as_str().unwrap().to_string(),
                    json!({"blob":blob,"sha256":digest,"length":length}),
                )
            })
            .collect();
        if change == "docs" {
            write(&root, "doc.txt", "changed documentation\n");
        } else if change == "contract" {
            self.contract["packages"][2]["features"] = json!([]);
            self.save();
        } else {
            write(
                &root,
                "p/src/lib.rs",
                "pub fn value()->u32 { 1+value_dep::value() }\n",
            );
        }
        let candidate = commit(&root);
        let environment = json!({"PATH":std::env::var("PATH").unwrap()});
        let retained = json!({"base":{"commit":base,"environment":environment,"files":files},"candidate":{"commit":candidate,"environment":environment,"files":files}});
        write(
            &root,
            ".chrono-harness/state/inputs.json",
            &retained.to_string(),
        );
        let context = json!({"schema_version":2,"base":base,"candidate":candidate,"dev_tip":base,"branch_ref":"integration/cargo","fork_point":base,"branch_started_at":"2026-01-01T00:00:00Z","observed_at":"2026-01-01T01:00:00Z","operation":"validate.delta","integration_evidence":null,"retained_inputs":".chrono-harness/state/inputs.json","run_kind":"integration"});
        write(
            &root,
            ".chrono-harness/state/context.json",
            &context.to_string(),
        );
        let config = root.join(if consumer == Consumer::Full {
            CONFIG
        } else {
            profile
        });
        let mut cmd = Command::new(root.join(".chrono-harness/bin/chrono-harness"));
        cmd.current_dir("/").args([
            "check",
            "--config",
            config.to_str().unwrap(),
            "--base",
            &base,
            "--candidate",
            &candidate,
        ]);
        if consumer == Consumer::Full {
            cmd.args([
                "--context",
                root.join(".chrono-harness/state/context.json")
                    .to_str()
                    .unwrap(),
            ]);
        }
        let out = cmd.output().unwrap();
        (
            out.status.code().unwrap(),
            serde_json::from_slice(&out.stdout)
                .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&out.stderr)})),
        )
    }
}
pub const POLICY: &str = ".chrono-harness/cargo/inputs.json";

#[derive(Clone, Copy, PartialEq)]
pub enum Consumer {
    Full,
    Scoped,
}
