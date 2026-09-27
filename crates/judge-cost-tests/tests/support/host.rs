#![allow(dead_code)]
use super::support::*;
use chrono_harness::sha256;
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
pub struct Host {
    dir: tempfile::TempDir,
    pub base: String,
    pub candidate: String,
    pub values: Values,
}
impl Host {
    pub fn new(unknown: bool) -> Self {
        let dir = tempfile::Builder::new()
            .prefix("cost consumer with spaces ")
            .tempdir()
            .unwrap();
        let root = dir.path();
        git(root, &["init", "-q"]);
        fs::create_dir_all(root.join(".chrono-harness/bin")).unwrap();
        let mut values = values();
        values.get_mut(JUDGES).unwrap()["judges"]
            .as_array_mut()
            .unwrap()
            .push(json!({
            "id":"cost","executable":".chrono-harness/bin/chrono-judge-cost","version":"0.1.0",
            "sha256":null,"argv":["--protocol","chrono-judge/v1"],"selector":"every-delta",
            "after":["registration","filemap"],"modes":["evaluate"]}));
        for (project, binary) in [
            ("runner", "chrono-harness"),
            ("judge-registration", "chrono-judge-registration"),
            ("judge-filemap", "chrono-judge-filemap"),
            ("judge-cost", "chrono-judge-cost"),
        ] {
            let bytes = fs::read(source().join(format!("crates/{project}/target/debug/{binary}")))
                .unwrap_or_else(|e| panic!("build registered {project} binary first: {e}"));
            fs::write(root.join(format!(".chrono-harness/bin/{binary}")), &bytes).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    root.join(format!(".chrono-harness/bin/{binary}")),
                    fs::Permissions::from_mode(0o755),
                )
                .unwrap();
            }
            if project == "runner" {
                values.get_mut(CONFIG).unwrap()["runner"]["sha256"] = sha256(&bytes).into();
            } else {
                for j in values.get_mut(JUDGES).unwrap()["judges"]
                    .as_array_mut()
                    .unwrap()
                {
                    if j["id"] == project.strip_prefix("judge-").unwrap() {
                        j["sha256"] = sha256(&bytes).into();
                    }
                }
            }
        }
        let fm = values.get_mut(FM).unwrap();
        if !unknown {
            fm["cost_models"]["unknown"] = json!({"cpu_ms":1,"wall_ms":2,"peak_rss_bytes":3,"io_bytes":4,"basis":"fixture estimate"});
        }
        for f in fm["files"].as_array_mut().unwrap() {
            let path = f["path"].as_str().unwrap().to_string();
            if path == "src.bin" || path.starts_with("p/") {
                f["owner"] = "p".into();
            }
            if path.starts_with("t/") {
                f["owner"] = "t".into();
            }
            let p = root.join(&path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, "fixture").unwrap();
        }
        fs::write(
            root.join(".gitignore"),
            ".chrono-harness/bin/\n.chrono-harness/state/\n",
        )
        .unwrap();
        write_values(root, &values);
        let base = commit(root);
        fs::write(root.join("src.bin"), "changed").unwrap();
        let candidate = commit(root);
        Self {
            dir,
            base,
            candidate,
            values,
        }
    }
    pub fn root(&self) -> &Path {
        self.dir.path()
    }
    pub fn save(&mut self) {
        write_values(self.root(), &self.values);
        self.candidate = commit(self.root());
    }
    pub fn run(&self) -> (i32, Value) {
        self.run_with_inputs(None)
    }
    pub fn run_with_inputs(&self, retained: Option<Value>) -> (i32, Value) {
        fs::create_dir_all(self.root().join(".chrono-harness/state")).unwrap();
        let mut context = json!({
            "schema_version":1,"base":self.base,"candidate":self.candidate,"dev_tip":self.base,
            "branch_ref":"integration/fixture","fork_point":self.base,"branch_started_at":"2026-01-01T00:00:00Z",
            "observed_at":"2026-01-01T01:00:00Z","operation":"validate.delta","integration_evidence":null});
        if let Some(inputs) = retained {
            context["retained_inputs"] = ".chrono-harness/state/inputs.json".into();
            fs::write(
                self.root().join(".chrono-harness/state/inputs.json"),
                serde_json::to_vec(&inputs).unwrap(),
            )
            .unwrap();
        }
        fs::write(
            self.root().join(".chrono-harness/state/context.json"),
            serde_json::to_vec(&context).unwrap(),
        )
        .unwrap();
        let output = Command::new(self.root().join(".chrono-harness/bin/chrono-harness"))
            .current_dir("/")
            .env_remove("HOME")
            .args([
                "check",
                "--config",
                self.root().join(CONFIG).to_str().unwrap(),
                "--base",
                &self.base,
                "--candidate",
                &self.candidate,
                "--context",
                ".chrono-harness/state/context.json",
            ])
            .output()
            .unwrap();
        let report = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr)));
        (output.status.code().unwrap(), report)
    }
}
