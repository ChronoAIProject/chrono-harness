#![allow(dead_code)]
use super::support::*;
use chrono_harness::sha256;
use serde_json::{Value, json};
use std::{fs, process::Command};
pub struct Host {
    dir: tempfile::TempDir,
    pub base: String,
    pub candidate: String,
    pub values: Values,
    pub external: tempfile::NamedTempFile,
    pub tool: std::path::PathBuf,
}
impl Host {
    pub fn new(script: bool) -> Self {
        let dir = tempfile::Builder::new()
            .prefix("execution host space ")
            .tempdir()
            .unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        git(&root, &["init", "-q"]);
        fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
        let mut v = values();
        for (id, crate_name, binary) in [
            ("runner", "runner", "chrono-harness"),
            (
                "registration",
                "judge-registration",
                "chrono-judge-registration",
            ),
            ("filemap", "judge-filemap", "chrono-judge-filemap"),
            ("routes", "judge-routes", "chrono-judge-routes"),
            ("projects", "judge-projects", "chrono-judge-projects"),
        ] {
            let source = source().join(format!("crates/{crate_name}/target/debug/{binary}"));
            assert!(source.is_file(), "build {}", source.display());
            let path = format!(".chrono-harness/bin/{binary}");
            fs::copy(source, root.join(&path)).unwrap();
            if id == "runner" {
                v.get_mut(CONFIG).unwrap()["runner"]["sha256"] =
                    json!(sha256(&fs::read(root.join(&path)).unwrap()));
            } else {
                let after = match id {
                    "registration" => vec![],
                    "filemap" => vec!["registration"],
                    "routes" => vec!["filemap"],
                    _ => vec!["routes"],
                };
                let row = json!({"id":id,"executable":path,"version":"0.1.0","sha256":sha256(&fs::read(root.join(&path)).unwrap()),"argv":["--protocol","chrono-judge/v1"],"selector":"every-delta","after":after,"modes":["evaluate"]});
                let judges = v.get_mut(JUDGES).unwrap()["judges"].as_array_mut().unwrap();
                judges.retain(|j| j["id"] != id);
                judges.push(row);
            }
        }
        let tool = chrono_harness::resolve_program(&root, "python3", None).unwrap();
        let version = Command::new(&tool).arg("--version").output().unwrap();
        let external = tempfile::NamedTempFile::new().unwrap();
        fs::write(external.path(), b"bounded input\n").unwrap();
        v.get_mut(CONFIG).unwrap()["canonical_check"]["argv"] = json!([
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
        v.get_mut(CONFIG).unwrap()["tools"] = json!([{"id":"python","program":tool,"resolution":"PATH-once","version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim_end()}]);
        v.get_mut(CONFIG).unwrap()["protocol"]["stdout_limit_bytes"] = json!(67108864);
        v.get_mut(CONFIG).unwrap()["environment"] = json!({"inherit":["DECLARED_EMPTY","DECLARED_ABSENT"],"values":{"EMPTY":""},"inputs":[{"id":"interpreter","location":tool,"sha256":sha256(&fs::read(&tool).unwrap())},{"id":"data","location":external.path(),"sha256":sha256(b"bounded input\n")}]});
        v.get_mut(FM).unwrap()["schema_version"] = json!(2);
        v.get_mut(FM).unwrap()["execution_plans"] = json!({"test:t":{"operations":["prepare.p","execute.t"],"timeout_seconds":15,"output_limit_bytes":4096}});
        let pre = json!({"operation":"prepare.p","tool":"python","argv":["-c","open('.chrono-harness/state/order','a').write('p')"]});
        let run = json!({"operation":"execute.t","tool":"python","argv":["-B","t/check.py","$HOME `literal` <tag>","","line\nnext"]});
        if script {
            v.get_mut(PROJECTS).unwrap()["projects"] = json!([]);
            v.get_mut(PROJECTS).unwrap()["scripts"] = json!([{"id":"p","path":"p/product.py","test_script":"t","actions":{"execute":pre}},{"id":"t","path":"t/check.py","tests_for":"p","actions":{"execute":run}}]);
            v.get_mut(FM).unwrap()["project_edges"] = json!([
                edge("script:p", "test-execution", "test:t"),
                edge("script:t", "test-execution", "test:t"),
                edge("input:data", "runtime-input", "script:p"),
                edge("input:interpreter", "runtime-input", "script:p")
            ]);
        } else {
            v.get_mut(PROJECTS).unwrap()["projects"][0]["actions"] = json!({"build":pre});
            v.get_mut(PROJECTS).unwrap()["projects"][1]["actions"] = json!({"execute":run});
            v.get_mut(FM).unwrap()["project_edges"] = json!([
                edge("project:p", "compile", "project:t"),
                edge("project:p", "test-execution", "test:t"),
                edge("project:t", "test-execution", "test:t"),
                edge("input:data", "runtime-input", "project:p"),
                edge("input:interpreter", "runtime-input", "project:p")
            ]);
            for id in ["p", "t"] {
                v.get_mut(CONFIG).unwrap()["artifacts"].as_array_mut().unwrap().push(json!({"path":format!("{id}/target/"),"owner":id,"kind":"cargo-output","tracked":false}));
            }
        }
        let prefix = if script { "script" } else { "project" };
        for f in v.get_mut(FM).unwrap()["files"].as_array_mut().unwrap() {
            let path = f["path"].as_str().unwrap().to_string();
            if path == "src.bin" {
                f["edges"] = json!([]);
            }
            if path.starts_with("p/") || path.starts_with("t/") {
                let id = &path[..1];
                f["owner"] = json!(id);
                f["edges"] = json!([{"kind":"runtime-input","to":format!("{prefix}:{id}")}]);
            }
        }
        for (path, owner) in [("p/product.py", "p"), ("t/check.py", "t")] {
            let mut f = file(
                path,
                json!([{"kind":"runtime-input","to":format!("{prefix}:{owner}")}]),
            );
            f["owner"] = json!(owner);
            v.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .push(f);
        }
        for f in v[FM]["files"].as_array().unwrap() {
            let path = root.join(f["path"].as_str().unwrap());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "fixture").unwrap();
        }
        fs::write(
            root.join(".gitignore"),
            ".chrono-harness/bin/\n.chrono-harness/state/\np/target/\nt/target/\n",
        )
        .unwrap();
        fs::write(
            root.join("p/Cargo.toml"),
            "[package]\nname='p'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.join("t/Cargo.toml"),
            "[package]\nname='t'\nversion='0.1.0'\n[dev-dependencies]\np={path='../p'}\n",
        )
        .unwrap();
        fs::write(root.join("p/product.py"), "def double(n): return n*2\n").unwrap();
        fs::write(root.join("t/check.py"),"import runpy,sys,os,json\nf=runpy.run_path('p/product.py')['double']\nassert f(21)==42\nassert open('.chrono-harness/state/order').read().endswith('p')\nprint(json.dumps([sys.argv[1:],os.getcwd(),os.environ.get('EMPTY'),os.environ.get('ABSENT')]))\nopen('.chrono-harness/state/order','a').write('t')\n").unwrap();
        write_values(&root, &v);
        let base = commit(&root);
        fs::write(root.join("p/product.py"), "def double(n): return n+n\n").unwrap();
        let candidate = commit(&root);
        Self {
            dir,
            base,
            candidate,
            values: v,
            external,
            tool,
        }
    }
    pub fn root(&self) -> std::path::PathBuf {
        fs::canonicalize(self.dir.path()).unwrap()
    }
    pub fn save(&mut self) {
        write_values(&self.root(), &self.values);
        self.candidate = commit(&self.root());
    }
    pub fn run(&self, edit: impl FnOnce(&mut Value)) -> (i32, Value) {
        self.run_with_context(edit, |_| {})
    }
    pub fn prepare_with_context(
        &self,
        edit: impl FnOnce(&mut Value),
        context_edit: impl FnOnce(&mut Value),
    ) {
        let root = self.root();
        fs::create_dir_all(root.join(".chrono-harness/state")).unwrap();
        let interpreter = fs::read(&self.tool).unwrap();
        let digest = sha256(&interpreter);
        let blob = format!(".chrono-harness/state/interpreter-{digest}");
        fs::write(root.join(&blob), &interpreter).unwrap();
        let files = json!({"interpreter":{"blob":blob,"sha256":digest,"length":interpreter.len()},"data":{"bytes":b"bounded input\n".to_vec()}});
        let mut inputs = json!({"base":{"commit":self.base,"environment":{"DECLARED_EMPTY":"","DECLARED_ABSENT":null},"files":files},"candidate":{"commit":self.candidate,"environment":{"DECLARED_EMPTY":"","DECLARED_ABSENT":null},"files":files}});
        edit(&mut inputs);
        fs::write(
            root.join(".chrono-harness/state/inputs.json"),
            serde_json::to_vec(&inputs).unwrap(),
        )
        .unwrap();
        let mut ctx = json!({"schema_version":1,"base":self.base,"candidate":self.candidate,"dev_tip":self.base,"branch_ref":"integration/fixture","fork_point":self.base,"branch_started_at":"2026-01-01T00:00:00Z","observed_at":"2026-01-01T01:00:00Z","operation":"validate.delta","integration_evidence":null,"retained_inputs":".chrono-harness/state/inputs.json"});
        context_edit(&mut ctx);
        fs::write(
            root.join(".chrono-harness/state/context.json"),
            serde_json::to_vec(&ctx).unwrap(),
        )
        .unwrap();
    }
    pub fn run_with_context(
        &self,
        edit: impl FnOnce(&mut Value),
        context_edit: impl FnOnce(&mut Value),
    ) -> (i32, Value) {
        self.prepare_with_context(edit, context_edit);
        let root = self.root();
        let out = Command::new(root.join(".chrono-harness/bin/chrono-harness"))
            .current_dir("/")
            .env("DECLARED_EMPTY", "")
            .env_remove("DECLARED_ABSENT")
            .args([
                "check",
                "--config",
                root.join(CONFIG).to_str().unwrap(),
                "--base",
                &self.base,
                "--candidate",
                &self.candidate,
                "--context",
                root.join(".chrono-harness/state/context.json")
                    .to_str()
                    .unwrap(),
            ])
            .output()
            .unwrap();
        self.retain_command_result("check", &out);
        let v = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| json!({"stderr":String::from_utf8_lossy(&out.stderr)}));
        (out.status.code().unwrap(), v)
    }
    pub fn retain_command_result(&self, label: &str, out: &std::process::Output) {
        let Ok(directory) = std::env::var("CHRONO_FULL_TEST_RECEIPTS") else {
            return;
        };
        static NUMBER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let number = NUMBER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let destination = std::path::Path::new(&directory)
            .join(format!("{}-{number}-{label}", std::process::id()));
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("stdout.bin"), &out.stdout).unwrap();
        fs::write(destination.join("stderr.bin"), &out.stderr).unwrap();
        fs::write(
            destination.join("binding.json"),
            serde_json::to_vec_pretty(&json!({
                "root": self.root(), "base": self.base, "candidate": self.candidate,
                "exit": out.status.code(), "status": out.status.to_string(), "joined": true
            }))
            .unwrap(),
        )
        .unwrap();
    }
}
