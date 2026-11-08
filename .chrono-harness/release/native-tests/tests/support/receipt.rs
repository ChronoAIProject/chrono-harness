//! 自含的发布方外部回执样本；预期由字面过程和文件生成，不调用被测判法。
#[path = "recipe.rs"]
pub mod recipe;
use chrono_host_release::{
    receipt::{Kind, Lineage, Selection},
    recipe::Loaded,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Fixture {
    pub host: recipe::Fixture,
    pub report: Value,
    pub context: Value,
    pub parent: Lineage,
    pub id: &'static str,
}

pub fn identity(path: &str, byte: char) -> Value {
    json!({"path":path,"sha256":byte.to_string().repeat(64),"size":100,"mode":493})
}

pub fn original(directory: &Path, path: &str, bytes: &[u8]) -> Value {
    let file = directory.join(path);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, bytes).unwrap();
    json!({"path":path,"size":bytes.len(),"sha256":chrono_harness::sha256(bytes)})
}

pub fn process(
    directory: &Path,
    rows: &mut Vec<Value>,
    phase: &str,
    args: &[&str],
    stdout: &str,
    operation: Option<&str>,
) {
    let index = rows.len();
    let out = original(
        directory,
        &format!("processes/{index}.stdout"),
        stdout.as_bytes(),
    );
    let err = original(directory, &format!("processes/{index}.stderr"), b"");
    rows.push(json!({"phase":phase,"operation":operation,"argv":args,"cwd":"/independent/producer","started_ns":10+index,"finished_ns":11+index,"exit_code":0,"stdout":out,"stderr":err}));
}

impl Fixture {
    pub fn new(rust: bool) -> Self {
        let host = recipe::Fixture::new();
        let id = if rust { "build_a" } else { "test_none" };
        let loaded = Loaded::load(host.root()).unwrap();
        let unit = loaded.unit(id).unwrap();
        let directory = host.root().join(unit.handoff());
        let context = json!({"source_commit":"1".repeat(40),"source_tree":"2".repeat(40),"system":"Fixture","machine":"fixture","recipe":{},"rust_toolchain":"1.95.0","rust_components":["rustfmt"]});
        let mut rows = Vec::new();
        process(
            &directory,
            &mut rows,
            "source-identity",
            &["/tools/git", "rev-parse", "HEAD"],
            &format!("{}\n", context["source_commit"].as_str().unwrap()),
            None,
        );
        process(
            &directory,
            &mut rows,
            "source-tree",
            &["/tools/git", "rev-parse", "HEAD^{tree}"],
            &format!("{}\n", context["source_tree"].as_str().unwrap()),
            None,
        );
        process(
            &directory,
            &mut rows,
            "source-clean",
            &[
                "/tools/git",
                "status",
                "--porcelain",
                "--untracked-files=normal",
            ],
            "",
            None,
        );
        let mut tools = json!({"git":identity("/tools/git",'a')});
        let toolchain = if rust {
            for (name, byte) in [("rustup", 'b'), ("cargo", 'c'), ("rustc", 'd')] {
                tools[name] = identity(&format!("/tools/{name}"), byte);
            }
            process(
                &directory,
                &mut rows,
                "toolchain-install",
                &[
                    "/tools/rustup",
                    "toolchain",
                    "install",
                    "1.95.0",
                    "--profile",
                    "minimal",
                    "--component",
                    "rustfmt",
                ],
                "",
                None,
            );
            for name in ["cargo", "rustc"] {
                process(
                    &directory,
                    &mut rows,
                    "compiler-version",
                    &[&format!("/tools/{name}"), "--version"],
                    &format!("{name} 1.95.0 (fixture)\n"),
                    None,
                );
            }
            for name in ["cargo", "rustc"] {
                process(
                    &directory,
                    &mut rows,
                    "compiler-executable",
                    &["/tools/rustup", "which", "--toolchain", "1.95.0", name],
                    &format!("/active/{name}\n"),
                    None,
                );
            }
            process(
                &directory,
                &mut rows,
                "build",
                &[
                    "/tools/cargo",
                    "build",
                    "--release",
                    "--locked",
                    "--manifest-path",
                    "a/Cargo.toml",
                ],
                "",
                None,
            );
            Some(
                json!({"tools":tools,"versions":{"cargo":"cargo 1.95.0 (fixture)","rustc":"rustc 1.95.0 (fixture)"},"active_compilers":{"cargo":identity("/active/cargo",'e'),"rustc":identity("/active/rustc",'f')}}),
            )
        } else {
            tools["probe"] = identity("/tools/probe", 'b');
            process(
                &directory,
                &mut rows,
                "verification",
                &[
                    "/tools/probe",
                    "verify none",
                    "space value",
                    "$literal",
                    "λ",
                ],
                "literal external command ran\n",
                Some("test.none"),
            );
            None
        };
        process(
            &directory,
            &mut rows,
            "source-clean-final",
            &[
                "/tools/git",
                "status",
                "--porcelain",
                "--untracked-files=normal",
            ],
            "",
            None,
        );
        process(
            &directory,
            &mut rows,
            "source-final",
            &["/tools/git", "rev-parse", "HEAD"],
            &format!("{}\n", context["source_commit"].as_str().unwrap()),
            None,
        );
        process(
            &directory,
            &mut rows,
            "tree-final",
            &["/tools/git", "rev-parse", "HEAD^{tree}"],
            &format!("{}\n", context["source_tree"].as_str().unwrap()),
            None,
        );
        let dependency = original(&directory, "dependency-context.json", b"{}");
        let mut report = json!({"schema":"chrono-release-unit/v1","unit":unit.value(),"status":"passed","failure":null,"context":context,"lineage":{"kind":"local","run_id":"same-run","attempt":"1","job":id},"started_ns":1,"finished_ns":1_000_000,"processes":rows,"dependency_input":dependency,"selected":{},"execution_tools":tools,"environment":{"CARGO_INCREMENTAL":"1"}});
        if let Some(value) = toolchain {
            report["toolchain"] = value;
        } else {
            report["action_tool"] = json!("probe");
        }
        Self {
            host,
            report,
            context,
            parent: Lineage {
                kind: Kind::Local,
                run_id: "same-run".into(),
                attempt: "3".into(),
                job: "collect".into(),
            },
            id,
        }
    }

    pub fn loaded(&self) -> Loaded {
        Loaded::load(self.host.root()).unwrap()
    }
    pub fn directory(&self) -> PathBuf {
        self.host
            .root()
            .join(self.loaded().unit(self.id).unwrap().handoff())
    }
    pub fn save(&self) -> Selection {
        let bytes = serde_json::to_vec_pretty(&self.report).unwrap();
        fs::write(self.directory().join("receipt.json"), &bytes).unwrap();
        Selection {
            job: self.id.into(),
            artifact_id: chrono_harness::sha256(&bytes),
            run_id: "same-run".into(),
            attempt: "1".into(),
        }
    }
    pub fn read(&self) -> Result<chrono_host_release::receipt::Verified, String> {
        let selection = self.save();
        let loaded = self.loaded();
        chrono_host_release::receipt::read(
            &loaded,
            self.host.root(),
            &self.context,
            &self.parent,
            loaded.unit(self.id).unwrap(),
            &selection,
        )
    }
}
