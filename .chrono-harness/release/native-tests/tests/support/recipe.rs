//! 自含的 v5 外部登记样本；不借生产解析器生成预期或选择依赖。
use serde_json::{Value, json};
use std::{fs, path::Path};

pub struct Fixture {
    pub temp: tempfile::TempDir,
    pub config: Value,
    pub plan: Value,
    pub projects: Value,
    pub host: Value,
}

pub fn write(root: &Path, name: &str, value: &Value) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

impl Fixture {
    pub fn new() -> Self {
        let units = json!([
            {"id":"build_a","kind":"build","manifest":"a/Cargo.toml","asset":"a","needs":[],"handoff":"handoffs/a"},
            {"id":"build_b","kind":"build","manifest":"b/Cargo.toml","asset":"b","needs":[],"handoff":"handoffs/b"},
            {"id":"test_none","kind":"verify","operation":"test.none","needs":[],"handoff":"handoffs/none","rust_toolchain":false},
            {"id":"test_a","kind":"verify","operation":"test.a","needs":["build_a"],"handoff":"handoffs/test_a","rust_toolchain":false}
        ]);
        let names = ["build_a", "build_b", "test_none", "test_a", "collect"];
        let mut costs = serde_json::Map::new();
        let mut jobs = serde_json::Map::new();
        for name in names {
            costs.insert(name.into(), json!("unmeasured"));
            jobs.insert(name.into(), json!(format!("native_{name}")));
        }
        let config = json!({
            "schema":"chrono-release-build/v5","projects":"projects.json",
            "tools":{"git":"/usr/bin/git","probe":"/bin/sh","rustup":"unavailable-release-rustup","cargo":"unavailable-release-cargo","rustc":"unavailable-release-rustc"},
            "rust_toolchain":"1.95.0","rust_components":["rustfmt"],"rust_incremental":true,
            "manifests":["a/Cargo.toml","b/Cargo.toml"],"verification_operations":["test.none","test.a"],
            "consumer_staging":{"release_plan":"plan.json","host_config":"host.json","bindings":[{"asset":"a","destinations":["consumer/a"]},{"asset":"b","destinations":["consumer/b"]}]},
            "units":units,"collector":{"id":"collect","needs":["build_a","build_b","test_none","test_a"],"package_asset":"a","dependency_metadata":"dependencies.json"},
            "unit_costs":costs,"native_jobs":{"fixture":jobs},"local_workers":2,"limits":{"unit_seconds":30,"platform_seconds":120},
            "native_frontends":{"fixture":{"job":"native_frontend","directory":"frontend","executable":"chrono-host-release","bootstrap_profile":".chrono-harness/ci/bootstrap-release.json","bootstrap_report":"bootstrap.json","cost":"unmeasured"}},
            "verification_consumers":{"test_none":{"need":"Exercise the independent source check.","release_assets":[]},"test_a":{"need":"Invoke only executable a.","release_assets":["a"]}},
            "failure_evidence":{"test.a":["failures/test-a/"]}
        });
        let result = Self {
            temp: tempfile::Builder::new()
                .prefix("release recipe λ ")
                .tempdir()
                .unwrap(),
            config,
            plan: json!({"schema":"chrono-release-plan/v1","version":"fixture-v1","assets":{"a":"out/a","b":"out/b"}}),
            projects: json!({"projects":[],"scripts":[{"id":"checks","actions":{"none":{"operation":"test.none","tool":"probe","argv":["verify none","space value","$literal","λ"]},"a":{"operation":"test.a","tool":"probe","argv":["verify a"]}}}]}),
            host: json!({"artifacts":[{"path":"consumer/","tracked":false},{"path":"failures/","tracked":false}]}),
        };
        result.save();
        result
    }

    pub fn root(&self) -> &Path {
        self.temp.path()
    }
    pub fn save(&self) {
        write(
            self.root(),
            ".chrono-harness/release/build.json",
            &self.config,
        );
        write(self.root(), "plan.json", &self.plan);
        write(self.root(), "projects.json", &self.projects);
        write(self.root(), "host.json", &self.host);
    }
}
