//! Test-owned custody coordinator. The real producer API still reads committed
//! policy and owns allocations; its external Git interface is the Rust mock.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use super::identifiers;
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
#[path = "../../../judge-filemap-tests/tests/support/mod.rs"]
mod policy;
use std::{fs,path::{Path,PathBuf},sync::OnceLock};
use serde_json::{Value,json};

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
        cleanup["artifacts"]=json!([{"path":".chrono-harness/state/","disposition":"evidence-retain"}]);
        let config=json!({"schema":"chrono-worktree-config/v2","host_config":policy::CONFIG,"remote":"unused",
            "git":{"program":env!("CARGO_BIN_EXE_chrono-test-git-model"),"expected_version":null,"sha256":null},
            "environment":{"inherit":["PATH"],"values":{}},"timeout_seconds":30,"output_limit_bytes":1048576,
            "report_directory":".chrono-harness/state/worktrees/","automatic_cleanup":".chrono-harness/cleanup.json"});
        fs::write(root.join(".chrono-harness/worktree.json"),serde_json::to_vec(&config).unwrap()).unwrap();
        fs::write(root.join(".chrono-harness/cleanup.json"),serde_json::to_vec(&cleanup).unwrap()).unwrap();
        fs::copy(source.join(".chrono-harness/retention.json"),root.join(".chrono-harness/retention.json")).unwrap();
        policy::commit(&root);
        root
    })
}

}
#[cfg(not(verus_keep_ghost))]
pub use runtime::{root};
