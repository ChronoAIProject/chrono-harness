//! Test-owned custody coordinator. The real producer API still reads committed
//! policy and owns allocations; its external Git interface is the Rust mock.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn known_custody_producer(command: &str) -> (allowed: bool)
    ensures allowed == (command@ == "ci-test-host"@ || command@ == "projects-test-host"@ || command@ == "workflow-test-tools"@ || command@ == "worktree-test-host"@),
{ identifiers::equal_text(command, "ci-test-host") || identifiers::equal_text(command, "projects-test-host") || identifiers::equal_text(command, "workflow-test-tools") || identifiers::equal_text(command, "worktree-test-host") }
}

#[cfg(not(verus_keep_ghost))]
#[path = "."]
mod runtime {
    #[path = "../../../runner-tests/tests/support/isolated_capability.rs"]
    mod isolation;
    #[path = "../../../judge-filemap-tests/tests/support/mod.rs"]
    mod policy;
    use serde_json::{Value, json};
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::OnceLock,
    };

    /// Join the original libtest child, then ask the existing producer owner about
    /// each declared store. A passed test alone is never a release observation.
    pub fn isolated_git() -> bool {
        isolation::isolated_git_with_retirement(retired)
    }

    pub fn retired(root: &Path) -> Result<bool, String> {
        for producer in [
            "ci-test-host",
            "projects-test-host",
            "workflow-test-tools",
            "worktree-test-host",
        ] {
            let (directory, _) = chrono_worktree::TemporaryHost::registered_store(
                root,
                ".chrono-harness/worktree.json",
                producer,
            )?;
            if !chrono_worktree::TemporaryHost::retired_fixture_store(&directory, producer)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn root() -> &'static Path {
        static ROOT: OnceLock<PathBuf> = OnceLock::new();
        ROOT.get_or_init(|| {
        let root=PathBuf::from(std::env::var_os("CHRONO_TEST_CUSTODY_ROOT").expect("isolated fixture custody root"));
        fs::create_dir_all(&root).unwrap();let root=fs::canonicalize(root).unwrap();
        policy::git(&root,&["init","-q","-b","dev"]);
        let mut values=policy::values();
        for v in values.values_mut(){v["status"]=json!("proposed");}
        values.get_mut(policy::CONFIG).unwrap()["enforcement"]=json!("not-implemented");
        values.get_mut(policy::CONFIG).unwrap()["input_closure"]=json!({"status":"incomplete","unresolved":["fixture exercises custody, not full host governance"]});
        // The joined parent also reads source-safety facts while observing
        // retirement. Bind the same mock in policy, independent of child PATH.
        let git=fs::canonicalize(env!("CARGO_BIN_EXE_chrono-test-git-model")).unwrap();
        let config=values.get_mut(policy::CONFIG).unwrap();
        config["schema_version"]=json!(3);
        config["facts_git"]=json!({"tool":"git","input":"git-bytes"});
        config["tools"]=json!([{"id":"git","program":git,"resolution":"PATH-once","version_argv":["--version"],"expected_version":"git version fixture"}]);
        config["environment"]["inputs"]=json!([{"id":"git-bytes","location":git,"presence":"present","sha256":chrono_harness::sha256(&fs::read(&git).unwrap())}]);
        values.get_mut(policy::PROJECTS).unwrap()["owners"]=json!(["host"]);
        values.get_mut(policy::PROJECTS).unwrap()["projects"]=json!([]);
        values.get_mut(policy::FM).unwrap()["project_edges"]=json!([]);
        values.get_mut(policy::FM).unwrap()["test_costs"]=json!([]);
        values.get_mut(policy::FM).unwrap()["files"]=json!([policy::CONFIG,policy::FM,policy::PROJECTS,policy::JUDGES,policy::WORKFLOW,".gitignore",".chrono-harness/worktree.json",".chrono-harness/cleanup.json",".chrono-harness/retention.json"].iter().map(|p|policy::file(p,json!([]))).collect::<Vec<_>>());
        policy::write_values(&root,&values);
        fs::write(root.join(".gitignore"),".chrono-harness/state/\n.chrono-harness/bin/\n").unwrap();
        let source=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut cleanup:Value=serde_json::from_slice(&fs::read(source.join(".chrono-harness/cleanup.json")).unwrap()).unwrap();
        cleanup["coordinator_root"]=json!("git-main-worktree");
        // Only these current producers are consumers of this fixture.
        cleanup["retained_producers"].as_array_mut().unwrap().retain(|p|p["id"].as_str().is_some_and(super::known_custody_producer));
        // All success consumers end within this one joined case. Failed,
        // diagnostic and interrupted allocations keep the current finite policy.
        // Metadata of an actually disposed success has no later test consumer.
        for producer in cleanup["retained_producers"].as_array_mut().unwrap() {
            producer["rolling_directory"]["metadata_keep_seconds"] = json!(0);
        }
        cleanup["artifacts"]=json!([{"path":".chrono-harness/state/","disposition":"evidence-retain"}]);
        let config=json!({"schema":"chrono-worktree-config/v2","host_config":policy::CONFIG,"remote":"unused",
            "git":{"program":env!("CARGO_BIN_EXE_chrono-test-git-model"),"expected_version":null,"sha256":null},
            "environment":{"inherit":["PATH"],"values":{}},"timeout_seconds":30,"output_limit_bytes":1048576,
            "report_directory":".chrono-harness/state/worktrees/","automatic_cleanup":".chrono-harness/cleanup.json",
            "check_inputs":{"origin_path":".chrono-harness/state/origin.json",
                "context_path":".chrono-harness/state/local/context.json",
                "collection_manifest":".chrono-harness/state/collection/manifest.json",
                "roles":{"feature":"integration","integration":"integration"}}});
        fs::write(root.join(".chrono-harness/worktree.json"),serde_json::to_vec(&config).unwrap()).unwrap();
        fs::write(root.join(".chrono-harness/cleanup.json"),serde_json::to_vec(&cleanup).unwrap()).unwrap();
        fs::copy(source.join(".chrono-harness/retention.json"),root.join(".chrono-harness/retention.json")).unwrap();
        policy::commit(&root);
        root
    })
    }
}
#[cfg(not(verus_keep_ghost))]
pub use runtime::{isolated_git, retired, root};
