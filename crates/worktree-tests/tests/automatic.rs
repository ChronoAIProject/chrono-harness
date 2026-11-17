#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::policy as absence_policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as absence_identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(not(verus_keep_ghost))]
use chrono_worktree::verified_lifecycle as absence_policy;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod absence_identifiers;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn imported_policy_source_preserves_explicit_field_and_only_uses_absent_field_source() {
    let policy = "policy";
    let source = "source";
    require_selected_source(absence_policy::imported_policy_source_commit(true, Some(policy), Some(source)), policy);
    require_selected_source(absence_policy::imported_policy_source_commit(true, Some(policy), None), policy);
    require_absence_expectation(absence_policy::imported_policy_source_commit(true, None, Some(source)).is_none());
    require_selected_source(absence_policy::imported_policy_source_commit(false, None, Some(source)), source);
    require_absence_expectation(absence_policy::imported_policy_source_commit(false, None, None).is_none());
}

fn require_selected_source(actual: Option<&str>, expected: &str)
    requires match actual { Some(actual) => actual@ == expected@, None => false },
{
    match actual {
        Some(actual) => require_absence_expectation(absence_identifiers::equal_text(actual, expected)),
        None => require_absence_expectation(false),
    }
}

fn require_capacity_fixture_exit(code: i32)
    requires code == 0,
    ensures code == 0,
{
    if code != 0 { panic!(); }
}

fn require_absence_expectation(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

fn handoff_trace_step(trace: Vec<u8>, stage: u8, result: Result<(), String>)
    -> (out: (Vec<u8>, Result<(), String>))
    ensures out.0@ == trace@.push(stage), out.1 == result,
{
    let mut trace = trace;
    trace.push(stage);
    (trace, result)
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ordinary_admission_selects_only_complete_registered_operation_names() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("use"); reveal_strlit("check"); reveal_strlit("bootstrap");
        reveal_strlit("status"); reveal_strlit("maintain"); reveal_strlit("migrate");
        reveal_strlit("deploy-owner"); reveal_strlit("import");
        reveal_strlit(""); reveal_strlit("use\0");
        assert("use"@.len() == 3);
        assert("check"@.len() == 5);
        assert("bootstrap"@.len() == 9);
        assert("status"@.len() == 6);
        assert("maintain"@.len() == 8);
        assert("migrate"@.len() == 7);
        assert("deploy-owner"@.len() == 12);
        assert("import"@.len() == 6);
        assert(""@.len() == 0);
        assert("use\0"@.len() == 4);
    }
    require_absence_expectation(absence_policy::ordinary_admission("use"));
    require_absence_expectation(absence_policy::ordinary_admission("check"));
    require_absence_expectation(absence_policy::ordinary_admission("bootstrap"));
    require_absence_expectation(!absence_policy::ordinary_admission("status"));
    require_absence_expectation(!absence_policy::ordinary_admission("maintain"));
    require_absence_expectation(!absence_policy::ordinary_admission("migrate"));
    require_absence_expectation(!absence_policy::ordinary_admission("deploy-owner"));
    require_absence_expectation(!absence_policy::ordinary_admission("import"));
    require_absence_expectation(!absence_policy::ordinary_admission(""));
    require_absence_expectation(!absence_policy::ordinary_admission("use\0"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn admission_handoff_orders_recovery_and_preserves_full_errors() {
    broadcast use vstd::array::group_array_axioms;
    let handoff_ok = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 1, Ok(())) };
    let handoff_error = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1),
            match out.1 { Err(error) => error@ == "原交接错误\0完整"@, Ok(()) => false },
        { handoff_trace_step(trace, 1, Err("原交接错误\0完整".to_string())) };
    let reload_ok = |trace: Vec<u8>, _error: String| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(2), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 2, Ok(())) };
    let reload_error = |trace: Vec<u8>, error: String| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(2), out.1 == Err::<(), String>(error),
        { handoff_trace_step(trace, 2, Err(error)) };
    let protect_ok = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(3), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 3, Ok(())) };
    let protect_error = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(3),
            match out.1 { Err(error) => error@ == "必要保护失败\0完整"@, Ok(()) => false },
        { handoff_trace_step(trace, 3, Err("必要保护失败\0完整".to_string())) };
    let (trace, result) = absence_policy::admission_handoff(Vec::<u8>::new(),
        handoff_ok, reload_error, protect_ok);
    assert(trace@ =~= seq![1u8, 3]);
    require_absence_expectation(absence_identifiers::equal_bytes(&trace, &[1u8, 3]));
    require_absence_expectation(result.is_ok());

    let (trace, result) = absence_policy::admission_handoff(Vec::<u8>::new(),
        handoff_error, reload_ok, protect_ok);
    assert(trace@ =~= seq![1u8, 2, 3]);
    require_absence_expectation(absence_identifiers::equal_bytes(&trace, &[1u8, 2, 3]));
    require_absence_expectation(result.is_ok());

    let (trace, result) = absence_policy::admission_handoff(Vec::<u8>::new(),
        handoff_error, reload_error, protect_ok);
    assert(trace@ =~= seq![1u8, 2]);
    require_absence_expectation(absence_identifiers::equal_bytes(&trace, &[1u8, 2]));
    match result {
        Err(error) => require_absence_expectation(absence_identifiers::equal_text(&error, "原交接错误\0完整")),
        Ok(()) => require_absence_expectation(false),
    }

    let (trace, result) = absence_policy::admission_handoff(Vec::<u8>::new(),
        handoff_ok, reload_ok, protect_error);
    assert(trace@ =~= seq![1u8, 3]);
    require_absence_expectation(absence_identifiers::equal_bytes(&trace, &[1u8, 3]));
    match result {
        Err(error) => require_absence_expectation(absence_identifiers::equal_text(&error, "必要保护失败\0完整")),
        Ok(()) => require_absence_expectation(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn enrollment_publication_skips_only_the_shrinking_preparation_and_stops_on_first_error() {
    broadcast use vstd::array::group_array_axioms;
    let before_ok = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 1, Ok(())) };
    let before_error = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1),
            match out.1 { Err(error) => error@ == "新增引用保护失败\0原件"@, Ok(()) => false },
        { handoff_trace_step(trace, 1, Err("新增引用保护失败\0原件".to_string())) };
    let forbidden = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(1),
            match out.1 { Err(error) => error@ == "不得执行"@, Ok(()) => false },
        { handoff_trace_step(trace, 1, Err("不得执行".to_string())) };
    let write_ok = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(2), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 2, Ok(())) };
    let write_error = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(2),
            match out.1 { Err(error) => error@ == "账本写入失败\0原件"@, Ok(()) => false },
        { handoff_trace_step(trace, 2, Err("账本写入失败\0原件".to_string())) };
    let after_ok = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(3), out.1 == Ok::<(), String>(()),
        { handoff_trace_step(trace, 3, Ok(())) };
    let after_error = |trace: Vec<u8>| -> (out: (Vec<u8>, Result<(), String>))
        ensures out.0@ == trace@.push(3),
            match out.1 { Err(error) => error@ == "引用收窄失败\0原件"@, Ok(()) => false },
        { handoff_trace_step(trace, 3, Err("引用收窄失败\0原件".to_string())) };
    let (trace, result) = absence_policy::publish_enrollment_record(Vec::<u8>::new(), true,
        before_ok, write_ok, after_ok);
    assert(trace@ =~= seq![1u8, 2, 3]);
    require_absence_expectation(absence_identifiers::equal_bytes(&trace, &[1u8, 2, 3]));
    require_absence_expectation(result.is_ok());
    let (trace, result) = absence_policy::publish_enrollment_record(Vec::<u8>::new(), false,
        forbidden, write_ok, after_ok);
    assert(trace@ =~= seq![2u8, 3]);
    require_absence_expectation(absence_identifiers::equal_bytes(&trace, &[2u8, 3]));
    require_absence_expectation(result.is_ok());
    let (trace, result) = absence_policy::publish_enrollment_record(Vec::<u8>::new(), true,
        before_error, write_ok, after_ok);
    assert(trace@ =~= seq![1u8]);
    let expected = [1u8];
    assert(expected@ =~= seq![1u8]);
    require_absence_expectation(absence_identifiers::equal_bytes(&trace, &expected));
    match result {
        Err(error) => require_absence_expectation(absence_identifiers::equal_text(&error, "新增引用保护失败\0原件")),
        Ok(()) => require_absence_expectation(false),
    }
    let (trace, result) = absence_policy::publish_enrollment_record(Vec::<u8>::new(), false,
        before_ok, write_error, after_ok);
    assert(trace@ =~= seq![2u8]);
    let expected = [2u8];
    assert(expected@ =~= seq![2u8]);
    require_absence_expectation(absence_identifiers::equal_bytes(&trace, &expected));
    match result {
        Err(error) => require_absence_expectation(absence_identifiers::equal_text(&error, "账本写入失败\0原件")),
        Ok(()) => require_absence_expectation(false),
    }
    let (trace, result) = absence_policy::publish_enrollment_record(Vec::<u8>::new(), false,
        before_ok, write_ok, after_error);
    assert(trace@ =~= seq![2u8, 3]);
    require_absence_expectation(absence_identifiers::equal_bytes(&trace, &[2u8, 3]));
    match result {
        Err(error) => require_absence_expectation(absence_identifiers::equal_text(&error, "引用收窄失败\0原件")),
        Ok(()) => require_absence_expectation(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_cache_probe_requires_maintenance_pending_cache_and_the_original_exclusion() {
    require_absence_expectation(absence_policy::absence_probe_required(true, true, true));
    require_absence_expectation(!absence_policy::absence_probe_required(false, true, true));
    require_absence_expectation(!absence_policy::absence_probe_required(true, false, true));
    require_absence_expectation(!absence_policy::absence_probe_required(true, true, false));
    require_absence_expectation(!absence_policy::absence_probe_required(false, false, true));
    require_absence_expectation(!absence_policy::absence_probe_required(false, true, false));
    require_absence_expectation(!absence_policy::absence_probe_required(true, false, false));
    require_absence_expectation(!absence_policy::absence_probe_required(false, false, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_cache_observation_preserves_unknown_task_and_unclaimed_removal() {
    let observation = absence_policy::missing_cache_observation();
    require_absence_expectation(absence_identifiers::equal_text(observation.status, "preserved"));
    require_absence_expectation(absence_identifiers::equal_text(observation.checkout_presence, "absent"));
    require_absence_expectation(absence_identifiers::equal_text(observation.metadata_presence, "absent"));
    require_absence_expectation(absence_identifiers::equal_text(observation.task_completion, "unknown"));
    require_absence_expectation(absence_identifiers::equal_text(observation.worktree_removal, "not-claimed"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn absent_enrollment_requires_exact_registered_kind_and_no_other_attachment() {
    broadcast use vstd::string::group_string_axioms;
    proof {
        reveal_strlit("birth"); reveal_strlit("import"); reveal_strlit("");
        reveal_strlit("birth\0"); reveal_strlit("HEAD"); reveal_strlit("HEAD\0");
        reveal_strlit("refs/heads/task");
        assert(""@.len() != "birth"@.len());
        assert(""@.len() != "import"@.len());
        assert("birth\0"@.len() != "birth"@.len());
        assert("birth\0"@[0] != "import"@[0]);
        assert("refs/heads/task"@.len() != "HEAD"@.len());
        assert("HEAD\0"@.len() != "HEAD"@.len());
    }
    require_absence_expectation(absence_policy::known_absent_enrollment(false, "birth"));
    require_absence_expectation(absence_policy::known_absent_enrollment(false, "import"));
    require_absence_expectation(!absence_policy::known_absent_enrollment(true, "birth"));
    require_absence_expectation(!absence_policy::known_absent_enrollment(true, "import"));
    require_absence_expectation(!absence_policy::known_absent_enrollment(false, ""));
    require_absence_expectation(!absence_policy::known_absent_enrollment(false, "birth\0"));
    require_absence_expectation(absence_policy::absent_attachment_conflict(true, "HEAD", false));
    require_absence_expectation(absence_policy::absent_attachment_conflict(true, "HEAD", true));
    require_absence_expectation(!absence_policy::absent_attachment_conflict(false, "HEAD", true));
    require_absence_expectation(!absence_policy::absent_attachment_conflict(false, "HEAD", false));
    require_absence_expectation(absence_policy::absent_attachment_conflict(false, "refs/heads/task", true));
    require_absence_expectation(!absence_policy::absent_attachment_conflict(false, "refs/heads/task", false));
    require_absence_expectation(absence_policy::absent_attachment_conflict(false, "HEAD\0", true));
}

}

#[path = "../../worktree-tests/tests/support/fixture_coordinator.rs"]
#[cfg(not(verus_keep_ghost))]
mod chrono_git_boundary;
#[path = "../../runner-tests/tests/support/retention_state.rs"]
#[cfg(not(verus_keep_ghost))]
mod retention_state;
#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use std::collections::BTreeMap;
#[cfg(not(verus_keep_ghost))]
const AUTO_POLICY: &str = ".chrono-harness/cleanup.json";
#[path = "complete_inventory.rs"]
#[cfg(not(verus_keep_ghost))]
mod complete_inventory;

// A metadata fixture with a fully mocked Git boundary. The absent checkout's
// adopted policy cannot be read, while its ledger still consumes an original.
#[cfg(not(verus_keep_ghost))]
struct EnrollmentOriginalsFixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    target: PathBuf,
    record: PathBuf,
    original: String,
    sibling: String,
}
#[cfg(not(verus_keep_ghost))]
impl EnrollmentOriginalsFixture {
    fn new() -> Self {
        Self::build(None)
    }

    fn imported() -> Self {
        Self::build(Some(""))
    }

    fn make_present(&self) {
        let fixture = json(&fs::read(self.root.join("git-fixture.json")).unwrap()).unwrap();
        fs::create_dir_all(self.target.join(".chrono-harness")).unwrap();
        for path in [POLICY, AUTO_POLICY] {
            fs::write(
                self.target.join(path),
                fixture["retained"]["b".repeat(64)][path].as_str().unwrap(),
            )
            .unwrap();
        }
        fs::write(self.target.join("unsaved-source.rs"), "unique source\n").unwrap();
    }

    fn build(import_fault: Option<&str>) -> Self {
        Self::build_with_lifetime(import_fault, false)
    }

    fn build_with_lifetime(import_fault: Option<&str>, finite: bool) -> Self {
        use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        let fixture = root.join("git-fixture.json");
        let mut registered = values();
        for value in registered.values_mut() {
            value["status"] = value!("proposed");
        }
        registered.get_mut(CONFIG).unwrap()["enforcement"] = value!("not-implemented");
        registered.get_mut(CONFIG).unwrap()["input_closure"] = value!({
            "status":"incomplete","unresolved":["mock lifecycle fixture does not certify complete host inputs"]
        });
        registered.get_mut(CONFIG).unwrap()["schema_version"] = value!(4);
        registered.get_mut(CONFIG).unwrap()["canonical_check"]["argv"] =
            value!([".chrono-harness/bin/chrono-harness", "check"]);
        registered.get_mut(CONFIG).unwrap()["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .push(value!("CHRONO_CHECK_SOURCE"));
        registered.get_mut(CONFIG).unwrap()["canonical_check"]["profile"] =
            value!(".chrono-harness/ci/check.json");
        registered.get_mut(CONFIG).unwrap()["canonical_check"]["inputs"] = value!({
            "local":{"operation":"prepare.fixture","tool":"sh","argv":["unused"]}
        });
        registered.get_mut(CONFIG).unwrap()["facts_git"] = value!({"tool":"git"});
        registered.get_mut(CONFIG).unwrap()["tools"]
            .as_array_mut()
            .unwrap()
            .push(value!({
                "id":"git","program":env!("CARGO_BIN_EXE_chrono-worktree-test-git"),
                "resolution":"PATH-once","version_argv":["--version"],"expected_version":null
            }));
        registered.get_mut(CONFIG).unwrap()["environment"]["values"] = value!({
            "HOME":root,"CHRONO_TEST_GIT_FIXTURE":fixture
        });
        registered.get_mut(PROJECTS).unwrap()["owners"] = value!(["host"]);
        registered.get_mut(PROJECTS).unwrap()["projects"] = value!([]);
        registered.get_mut(FM).unwrap()["project_edges"] = value!([]);
        registered.get_mut(FM).unwrap()["test_costs"] = value!([]);
        for path in [POLICY, AUTO_POLICY, POLICY_PATH] {
            registered.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .push(file(path, value!([])));
        }
        write_values(&root, &registered);
        let write = |path: &str, value: Value| {
            fs::write(root.join(path), serde_json::to_vec(&value).unwrap()).unwrap();
        };
        write(
            "git-fixture.json",
            value!({"mode":"enrollment-originals","root":root,"paths":registered.keys().collect::<Vec<_>>()}),
        );
        write(
            POLICY,
            value!({
                "schema":"chrono-worktree-config/v1","host_config":CONFIG,"remote":"unused",
                "git":{"program":env!("CARGO_BIN_EXE_chrono-worktree-test-git"),"expected_version":null,"sha256":null},
                "environment":{"inherit":[],"values":{"HOME":root,"CHRONO_TEST_GIT_FIXTURE":fixture}},
                "timeout_seconds":30,"output_limit_bytes":1048576,
                "report_directory":".chrono-harness/state/worktrees/","automatic_cleanup":AUTO_POLICY
            }),
        );
        write(
            AUTO_POLICY,
            value!({
                "schema":"chrono-worktree-automatic-cleanup/v1","coordinator_root":root,
                "state_directory":".chrono-harness/state/automatic-cleanup/",
                "retained_ref":"refs/heads/dev","retention":"ancestor","remove_branch":false,
                "allow_evidence_disposal":true,"artifacts":[]
            }),
        );
        let mut retention = value!({
            "schema":"chrono-output-retention/v2",
            "producers":{"lifecycle-original":[".chrono-harness/state/"]},
            "keep_seconds":0,"max_related_outputs":512,"max_summaries":128,
            "max_nodes_per_round":256,"max_bytes_per_round":67108864,
            "max_millis_per_round":25,"max_output_bytes":1073741824,
            "max_manifest_nodes":16384,"max_metadata_bytes_per_round":67108864
        });
        if finite {
            retention["max_lifetime_seconds"] = value!(86400);
            retention["metadata_keep_seconds"] = value!(0);
        }
        write(POLICY_PATH, retention);
        let target = root.join("absent-checkout");
        let record = root
            .join(".chrono-harness/state/automatic-cleanup/enrollments")
            .join(sha256(target.to_str().unwrap().as_bytes()));
        fs::create_dir_all(&record).unwrap();
        let original = ".chrono-harness/state/birth-original.json".to_owned();
        let sibling = ".chrono-harness/state/released-sibling.json".to_owned();
        fs::write(
            record.join("owner.json"),
            serde_json::to_vec(&value!({
                "schema":"chrono-worktree-enrollment-owner/v1","coordinator_root":root,
                "common":root.join(".git"),"path":target
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            record.join("ledger.json"),
            serde_json::to_vec(&value!({
                "schema":"chrono-worktree-enrollment-state/v1","coordinator_root":root,
                "common":root.join(".git"),"entries":[{
                    "path":target,"branch":"integration/absent","status":"active",
                    "attachment":{"gitfile_sha256":"unknown","gitfile_id":"unknown",
                        "metadata":root.join(".git/worktrees/absent"),"metadata_id":"unknown"},
                    "policy_sha256":"old-policy",
                "enrollment":{"kind":"birth","observed_head":"b".repeat(64),
                        "policy_inputs":{},"receipt":{"source_root":root,"report_path":original}},
                    "uses":[],"terminal":null,"attempts":[]
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        let mut original_bytes = b"original failure bytes\n".to_vec();
        if let Some(fault) = import_fault {
            let source = "c".repeat(64);
            let mut config = json(&fs::read(root.join(POLICY)).unwrap()).unwrap();
            config["git"] = value!({"not-executable":"retained caller data"});
            if fault == "policy-address" {
                config["automatic_cleanup"] = value!(".chrono-harness/other.json");
            }
            let config_bytes = serde_json::to_string(&config).unwrap();
            let mut policy = json(&fs::read(root.join(AUTO_POLICY)).unwrap()).unwrap();
            policy["allow_evidence_disposal"] = value!(false);
            if fault == "namespace" {
                policy["state_directory"] = value!(".chrono-harness/state/other-cleanup/");
            }
            let policy_bytes = serde_json::to_string(&policy).unwrap();
            let mut retained = BTreeMap::<String, String>::new();
            for path in registered.keys() {
                retained.insert(path.clone(), fs::read_to_string(root.join(path)).unwrap());
            }
            retained.insert(POLICY.into(), config_bytes.clone());
            retained.insert(AUTO_POLICY.into(), policy_bytes.clone());
            if matches!(fault, "config-bytes" | "source-only-config-bytes") {
                retained.insert(POLICY.into(), format!("{config_bytes}\n"));
            }
            if matches!(fault, "policy-bytes" | "source-only-policy-bytes") {
                retained.insert(AUTO_POLICY.into(), format!("{policy_bytes}\n"));
            }
            if fault == "registration" {
                let mut filemap = json(retained[FM].as_bytes()).unwrap();
                filemap["files"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|row| row["path"] != AUTO_POLICY);
                retained.insert(FM.into(), serde_json::to_string(&filemap).unwrap());
            }
            let mut git_fixture = json(&fs::read(&fixture).unwrap()).unwrap();
            git_fixture["retained"] = value!({
                (source.clone()):retained,
                ("b".repeat(64)):{(POLICY):"target original configuration is raw data",
                    (AUTO_POLICY):"target original policy is different raw data"}
            });
            write("git-fixture.json", git_fixture);
            let mut ledger = json(&fs::read(record.join("ledger.json")).unwrap()).unwrap();
            let entry = &mut ledger["entries"][0];
            entry["policy_sha256"] = value!(sha256(policy_bytes.as_bytes()));
            entry["enrollment"]["kind"] = value!("import");
            entry["enrollment"]["policy_inputs"] = value!({
                (POLICY):sha256(b"target original configuration is raw data"),
                (AUTO_POLICY):sha256(b"target original policy is different raw data")
            });
            fs::write(
                record.join("ledger.json"),
                serde_json::to_vec(&ledger).unwrap(),
            )
            .unwrap();
            let mut report = value!({
                "schema":"chrono-worktree-report/v1","operation":"import","status":"imported",
                "source_root":root,"source_commit":source,"policy_source_root":root,"policy_source_commit":source,
                "report_path":original,"config_path":POLICY,
                "config_sha256":sha256(config_bytes.as_bytes())
            });
            match fault {
                "explicit-policy-source" => report["source_commit"] = value!("b".repeat(64)),
                "source-only" | "source-only-config-bytes" | "source-only-policy-bytes" => {
                    report
                        .as_object_mut()
                        .unwrap()
                        .remove("policy_source_commit");
                }
                "null-policy-source" => report["policy_source_commit"] = value!(null),
                "invalid-policy-source" => report["policy_source_commit"] = value!(17),
                "short-policy-source" => report["policy_source_commit"] = value!("not-an-oid"),
                "source-only-null" | "source-only-invalid" | "source-only-short" => {
                    report
                        .as_object_mut()
                        .unwrap()
                        .remove("policy_source_commit");
                    report["source_commit"] = match fault {
                        "source-only-null" => value!(null),
                        "source-only-invalid" => value!(17),
                        _ => value!("not-an-oid"),
                    };
                }
                "missing-source" => {
                    report.as_object_mut().unwrap().remove("source_commit");
                    report
                        .as_object_mut()
                        .unwrap()
                        .remove("policy_source_commit");
                }
                "schema" => report["schema"] = value!("chrono-worktree-report/v2"),
                "operation" => report["operation"] = value!("migrate"),
                "status" => report["status"] = value!("failed"),
                "report-path" => report["report_path"] = value!(sibling),
                "source-root" => report["source_root"] = value!(target),
                "config-path" => report["config_path"] = value!(AUTO_POLICY),
                "config-digest" => report["config_sha256"] = value!("0".repeat(64)),
                "retained-input" => {
                    report["original_result"] = value!({"input":{
                        "schema":"chrono-worktree-retained-input/v1","source_root":root,
                        "path":".chrono-harness/state/missing-input.bin","sha256":"0".repeat(64),
                        "byte_length":0,"format":"bytes"
                    }})
                }
                _ => (),
            }
            original_bytes = serde_json::to_vec(&report).unwrap();
        }
        for path in [&original, &sibling] {
            fs::write(
                root.join(path),
                if path == &original {
                    original_bytes.as_slice()
                } else {
                    b"original failure bytes\n"
                },
            )
            .unwrap();
            let publication = Inventory::adopted(&root)
                .unwrap()
                .unwrap()
                .begin("lifecycle-original", path, value!({"status":"unknown"}))
                .unwrap();
            if path != &original || import_fault != Some("unsealed") {
                publication
                    .complete(value!({"status":"failed","error":"original failure"}))
                    .unwrap();
            }
        }
        Self {
            _directory: directory,
            root,
            target,
            record,
            original,
            sibling,
        }
    }
    fn invoke(&self, operation: &str, select: bool) -> (u8, Value) {
        let mut args = vec![
            operation.into(),
            "--host-root".into(),
            self.root.to_str().unwrap().into(),
        ];
        if select {
            args.extend(["--path".into(), self.target.to_str().unwrap().into()]);
        }
        let output = chrono_worktree::run(&args);
        let report = json(output.stdout.as_bytes())
            .unwrap_or_else(|error| panic!("{error}: {} {}", output.stdout, output.stderr));
        (output.exit_code, report)
    }

    fn adopt_retained_inputs(&self, config: Value) {
        let policy = json(&fs::read(self.root.join(AUTO_POLICY)).unwrap()).unwrap();
        self.adopt_retained_policy(config, policy);
    }

    fn adopt_retained_policy(&self, config: Value, policy: Value) {
        let fixture_path = self.root.join("git-fixture.json");
        let mut fixture = json(&fs::read(&fixture_path).unwrap()).unwrap();
        let head = "b".repeat(64);
        let mut retained = BTreeMap::<String, String>::new();
        for path in fixture["paths"].as_array().unwrap() {
            let path = path.as_str().unwrap();
            retained.insert(
                path.into(),
                fs::read_to_string(self.root.join(path)).unwrap(),
            );
        }
        let config_bytes = serde_json::to_string(&config).unwrap();
        let policy_bytes = serde_json::to_string(&policy).unwrap();
        retained.insert(POLICY.into(), config_bytes.clone());
        retained.insert(AUTO_POLICY.into(), policy_bytes.clone());
        fixture["retained"] = value!({(head):retained});
        fs::write(&fixture_path, serde_json::to_vec(&fixture).unwrap()).unwrap();
        let path = self.record.join("ledger.json");
        let mut record = json(&fs::read(&path).unwrap()).unwrap();
        let entry = &mut record["entries"][0];
        entry["policy_sha256"] = value!(sha256(policy_bytes.as_bytes()));
        entry["enrollment"]["policy_inputs"] = value!({
            (POLICY):Some(sha256(config_bytes.as_bytes())),
            (AUTO_POLICY):Some(sha256(policy_bytes.as_bytes()))
        });
        fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn published_result_handoffs_recover_a_full_enrollment_without_raising_capacity() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
    let h = EnrollmentOriginalsFixture::imported();
    let policy_path = h.root.join(POLICY_PATH);
    let previous_policy = fs::read(&policy_path).unwrap();
    let mut policy = json(&previous_policy).unwrap();
    policy["max_related_outputs"] = value!(4);
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    inventory.migrate_policy(&previous_policy, None).unwrap();
    let role = format!(
        "worktree-enrollment:{}",
        sha256(h.target.to_str().unwrap().as_bytes())
    );
    let mut receipts = Vec::new();
    let mut parents = Vec::new();
    let mut parent_bytes = Vec::new();
    for n in 0..4 {
        let path = format!(".chrono-harness/state/pending-use-{n}.json");
        let parent = format!(".chrono-harness/state/final-use-{n}.json");
        let raw = serde_json::to_vec(&value!({
            "schema":"chrono-worktree-managed-use-result/v1","report_path":parent
        }))
        .unwrap();
        let receipt = value!({"path":path,"sha256":sha256(&raw)});
        fs::write(h.root.join(&path), raw).unwrap();
        Inventory::adopted(&h.root)
            .unwrap()
            .unwrap()
            .begin("lifecycle-original", &path, value!(null))
            .unwrap()
            .complete(value!({"status":"used"}))
            .unwrap();
        let raw = serde_json::to_vec(&value!({
            "schema":"chrono-worktree-report/v1","report_path":parent,
            "status":"used","managed_use_receipt":receipt
        }))
        .unwrap();
        fs::write(h.root.join(&parent), &raw).unwrap();
        Inventory::adopted(&h.root)
            .unwrap()
            .unwrap()
            .begin("lifecycle-original", &parent, value!(null))
            .unwrap()
            .complete(value!({"status":"used"}))
            .unwrap();
        receipts.push(receipt);
        parents.push(parent);
        parent_bytes.push(raw);
    }
    inventory
        .reference_paths("actual-final-report-reader", &parents)
        .unwrap();
    let mut old_paths = vec![h.original.clone()];
    old_paths.extend(
        receipts[..3]
            .iter()
            .map(|r| r["path"].as_str().unwrap().to_owned()),
    );
    inventory.reference_paths(&role, &old_paths).unwrap();
    // A joined result was committed after the role reached its limit. The
    // next invocation must accept sealed parents before protecting all paths.
    let ledger_path = h.record.join("ledger.json");
    let mut ledger = json(&fs::read(&ledger_path).unwrap()).unwrap();
    ledger["entries"][0]["enrollment"]["completed_uses"] = value!(receipts);
    fs::write(&ledger_path, serde_json::to_vec(&ledger).unwrap()).unwrap();
    // A partial parent remains pending even while the other handoffs settle.
    fs::write(h.root.join(&parents[3]), b"{\"status\":").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let before = fs::read(&ledger_path).unwrap();
        let old_role = fs::read(retention_state::role_path(&h.root, &role)).unwrap();
        let permissions = fs::metadata(&h.record).unwrap().permissions();
        fs::set_permissions(&h.record, fs::Permissions::from_mode(0o500)).unwrap();
        let (_code, failed) = h.invoke("maintain", false);
        fs::set_permissions(&h.record, permissions).unwrap();
        assert!(failed.to_string().contains("Permission denied"), "{failed}");
        assert_eq!(
            fs::read(&ledger_path).unwrap(),
            before,
            "failed ledger publication must leave the original recovery inputs"
        );
        assert_eq!(
            fs::read(retention_state::role_path(&h.root, &role)).unwrap(),
            old_role,
            "a failed handoff must not release the old role"
        );
    }
    let (_code, maintained) = h.invoke("maintain", false);
    let current = json(&fs::read(&ledger_path).unwrap()).unwrap();
    assert_eq!(
        current["entries"][0]["enrollment"]["completed_uses"],
        value!([receipts[3]]),
        "sealed report handoffs must not deadlock behind capacity: {maintained}"
    );
    let snapshot = retention_state::snapshot(&h.root);
    let pending = snapshot["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|output| output["path"] == receipts[3]["path"])
        .unwrap();
    assert_eq!(snapshot["roots"][&role], value!([pending["id"]]));
    assert!(
        !maintained
            .to_string()
            .contains("related retention graph capacity")
    );
    for receipt in &receipts {
        assert!(
            h.root.join(receipt["path"].as_str().unwrap()).exists(),
            "actual report readers must keep their inputs"
        );
    }
    let (_code, _) = h.invoke("maintain", false);
    assert_eq!(
        json(&fs::read(&ledger_path).unwrap()).unwrap()["entries"][0]["enrollment"]["completed_uses"],
        value!([receipts[3]]),
        "partial originals must survive retries"
    );
    fs::write(h.root.join(&parents[3]), &parent_bytes[3]).unwrap();
    let (_code, maintained) = h.invoke("maintain", false);
    assert!(json(&fs::read(&ledger_path).unwrap()).unwrap()["entries"][0]["enrollment"]["completed_uses"].is_null(),
        "restored sealed parent must finish its own handoff: {maintained}");
    assert_eq!(
        json(&fs::read(&policy_path).unwrap()).unwrap()["max_related_outputs"],
        4
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn expired_import_original_hands_policy_to_current_state_without_renewal() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::{Inventory, read_guard};
    for present in [false, true] {
        let h = EnrollmentOriginalsFixture::build_with_lifetime(Some(""), true);
        if present {
            h.make_present();
        }
        let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
        let role = format!(
            "worktree-enrollment:{}",
            sha256(h.target.to_str().unwrap().as_bytes())
        );
        inventory
            .reference_paths(&role, &[h.original.clone()])
            .unwrap();
        let reader = read_guard(&h.root, &h.original).unwrap();
        let snapshot = retention_state::snapshot(&h.root);
        let mut output = snapshot["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|output| output["path"] == h.original)
            .unwrap()
            .clone();
        output["lifetime"] = value!({"created_at":1,"deadline":2});
        output["expires"] = value!(2);
        retention_state::replace_output(&h.root, &output);
        let before = json(&fs::read(h.record.join("ledger.json")).unwrap()).unwrap();
        let original = fs::read(h.root.join(&h.original)).unwrap();
        let (code, report) = h.invoke("maintain", false);
        assert_eq!(code, 0, "{report}");
        assert_import_policy_handoff(&h, &before);
        assert_eq!(
            fs::read(h.root.join(&h.original)).unwrap(),
            original,
            "an independent active reader still owns the original"
        );
        let snapshot = retention_state::snapshot(&h.root);
        let observed = snapshot["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|observed| observed["id"] == output["id"])
            .unwrap();
        assert_eq!(observed["lifetime"], output["lifetime"]);
        assert_eq!(observed["expires"], output["expires"]);
        assert!(read_guard(&h.root, &h.original).is_err());
        drop(reader);
        maintain_original_until_absent(&inventory, &h.root.join(&h.original));
        assert!(!h.root.join(&h.original).exists());
        let adopted = fs::read(h.record.join("ledger.json")).unwrap();
        let (code, report) = h.invoke("maintain", false);
        assert_eq!(code, 0, "{report}");
        assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), adopted);
        assert_eq!(h.target.exists(), present);
    }
}

#[cfg(not(verus_keep_ghost))]
fn assert_import_policy_handoff(h: &EnrollmentOriginalsFixture, before: &Value) {
    let after = json(&fs::read(h.record.join("ledger.json")).unwrap()).unwrap();
    let previous = &before["entries"][0];
    let current = &after["entries"][0];
    for field in [
        "path",
        "branch",
        "attachment",
        "policy_sha256",
        "status",
        "enrollment",
        "uses",
        "terminal",
        "attempts",
    ] {
        assert_eq!(current[field], previous[field], "handoff changed {field}");
    }
    let report = json(&fs::read(h.root.join(&h.original)).unwrap()).unwrap();
    assert_eq!(
        current["adopted_policy"],
        value!({
            "policy_sha256":previous["policy_sha256"],
            "head":previous["enrollment"]["observed_head"],
            "inputs":previous["enrollment"]["policy_inputs"],
            "policy_source":{"head":"c".repeat(64),"inputs":{
                (POLICY):report["config_sha256"],(AUTO_POLICY):previous["policy_sha256"]
            }}
        })
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn invalid_import_policy_handoff_preserves_original_role_and_ledger() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::Inventory;
    for fault in [
        "digest",
        "input-keys",
        "source-policy",
        "migration-conflict",
    ] {
        let h = EnrollmentOriginalsFixture::imported();
        let role = format!(
            "worktree-enrollment:{}",
            sha256(h.target.to_str().unwrap().as_bytes())
        );
        Inventory::adopted(&h.root)
            .unwrap()
            .unwrap()
            .reference_paths(&role, &[h.original.clone()])
            .unwrap();
        let ledger_path = h.record.join("ledger.json");
        let mut ledger = json(&fs::read(&ledger_path).unwrap()).unwrap();
        let entry = &mut ledger["entries"][0];
        let original = fs::read(h.root.join(&h.original)).unwrap();
        let report = json(&original).unwrap();
        entry["adopted_policy"] = value!({
            "policy_sha256":entry["policy_sha256"],
            "head":entry["enrollment"]["observed_head"],
            "inputs":entry["enrollment"]["policy_inputs"],
            "policy_source":{"head":"c".repeat(64),"inputs":{
                (POLICY):report["config_sha256"],(AUTO_POLICY):entry["policy_sha256"]
            }}
        });
        let expected_error = match fault {
            "digest" => {
                entry["adopted_policy"]["policy_sha256"] = value!("invalid");
                "invalid current adopted policy binding"
            }
            "input-keys" => {
                entry["adopted_policy"]["inputs"]["unexpected"] = value!(null);
                "invalid current adopted policy binding"
            }
            "source-policy" => {
                entry["adopted_policy"]["policy_source"]["inputs"][AUTO_POLICY] =
                    value!("0".repeat(64));
                "current adopted policy digest differs from its committed inputs"
            }
            "migration-conflict" => {
                entry["policy_migrations"] =
                    value!([{"path":h.original,"sha256":sha256(&original)}]);
                "current policy binding cannot retain a migration chain"
            }
            _ => unreachable!(),
        };
        let ledger = serde_json::to_vec(&ledger).unwrap();
        fs::write(&ledger_path, &ledger).unwrap();
        let role_path = retention_state::role_path(&h.root, &role);
        let previous_role = fs::read(&role_path).unwrap();
        let (code, report) = h.invoke("maintain", false);
        assert_ne!(code, 0, "{fault}: {report}");
        assert!(
            report.to_string().contains(expected_error),
            "{fault}: {report}"
        );
        assert_eq!(
            fs::read(&role_path).ok(),
            Some(previous_role),
            "invalid handoff must not release the original role: {fault}"
        );
        assert_eq!(fs::read(&ledger_path).unwrap(), ledger, "{fault}");
        assert_eq!(
            fs::read(h.root.join(&h.original)).unwrap(),
            original,
            "{fault}"
        );
    }
}

#[cfg(not(verus_keep_ghost))]
fn terminal_original_fixture(fault: &str) -> EnrollmentOriginalsFixture {
    use chrono_harness::retained_artifacts::retention::Inventory;
    let h = EnrollmentOriginalsFixture::build_with_lifetime(None, true);
    let config = json(&fs::read(h.root.join(POLICY)).unwrap()).unwrap();
    h.adopt_retained_inputs(config);
    let path = ".chrono-harness/state/terminal-original.json";
    let ledger_path = h.record.join("ledger.json");
    let mut ledger = json(&fs::read(&ledger_path).unwrap()).unwrap();
    let e = &mut ledger["entries"][0];
    e["adopted_birth"] = value!("established");
    e["status"] = value!("retained");
    e["creator_agent"] = value!(null);
    e["custodian_agent"] = value!(null);
    let mut original = value!({
        "schema":"chrono-worktree-terminal/v1","path":h.target,
        "attachment":e["attachment"],"branch":e["branch"],
        "head":"b".repeat(64),"retained_commit":"c".repeat(64),
        "policy_sha256":e["policy_sha256"],"retained_ref":"refs/heads/dev",
        "dispose_evidence":false,"artifacts_only":true,
        "handoff":"caller-joined-owned-jobs-and-established-completion",
        "report_path":".chrono-harness/state/finish-original.json"
    });
    match fault {
        "schema" => original["schema"] = value!("chrono-worktree-age-retirement/v1"),
        "path" => original["path"] = value!(h.root),
        "attachment" => original["attachment"]["metadata_id"] = value!("other"),
        "head" => original["head"] = value!("d".repeat(64)),
        "retained-commit" => original["retained_commit"] = value!("d".repeat(64)),
        "dispose-evidence" => original["dispose_evidence"] = value!(true),
        "artifacts-only" => original["artifacts_only"] = value!(false),
        "handoff" => original["handoff"] = value!("not-established"),
        _ => (),
    }
    let raw = serde_json::to_vec(&original).unwrap();
    fs::write(h.root.join(path), &raw).unwrap();
    let receipt = value!({"path":path,"sha256":sha256(&raw)});
    e["terminal"] = value!({
        "head":"b".repeat(64),"retained_commit":"c".repeat(64),
        "dispose_evidence":false,"artifacts_only":true,"receipt":receipt
    });
    // Legacy pending progress could carry a source path. It supplies an
    // observation only; it must not root a transferred original indefinitely.
    e["cleanup"] = value!({"stage":"pending","detail":"terminal handoff recorded",
        "report_path":path});
    fs::write(&ledger_path, serde_json::to_vec(&ledger).unwrap()).unwrap();
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    let publication = inventory
        .begin("lifecycle-original", path, value!(null))
        .unwrap();
    if fault != "unsealed" {
        publication.complete(value!({"status":"finished"})).unwrap();
    }
    let role = format!(
        "worktree-enrollment:{}",
        sha256(h.target.to_str().unwrap().as_bytes())
    );
    Inventory::adopted(&h.root)
        .unwrap()
        .unwrap()
        .reference_paths(&role, &[path.into()])
        .unwrap();
    if fault == "changed" {
        fs::write(h.root.join(path), b"changed original\n").unwrap();
    } else if fault == "missing" {
        fs::remove_file(h.root.join(path)).unwrap();
    }
    h
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn expired_terminal_original_transfers_facts_without_renewal_and_retires_after_reader_release() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::{Inventory, read_guard};
    let h = terminal_original_fixture("");
    let path = ".chrono-harness/state/terminal-original.json";
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    let reader = read_guard(&h.root, path).unwrap();
    let before = json(&fs::read(h.record.join("ledger.json")).unwrap()).unwrap();
    let raw = fs::read(h.root.join(path)).unwrap();
    let mut output = retention_state::snapshot(&h.root)["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|output| output["path"] == path)
        .unwrap()
        .clone();
    output["lifetime"] = value!({"created_at":1,"deadline":2});
    output["expires"] = value!(2);
    retention_state::replace_output(&h.root, &output);
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    let after = json(&fs::read(h.record.join("ledger.json")).unwrap()).unwrap();
    let binding = after["entries"][0]["terminal"]["established"]
        .as_str()
        .unwrap();
    assert_eq!(binding.len(), 64);
    let mut unchanged = after.clone();
    unchanged["entries"][0]["terminal"]
        .as_object_mut()
        .unwrap()
        .remove("established");
    assert_eq!(
        unchanged, before,
        "transfer must preserve the original work and cleanup states"
    );
    assert_eq!(fs::read(h.root.join(path)).unwrap(), raw);
    let snapshot = retention_state::snapshot(&h.root);
    let observed = snapshot["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|observed| observed["id"] == output["id"])
        .unwrap();
    assert_eq!(observed["lifetime"], output["lifetime"]);
    assert_eq!(observed["expires"], output["expires"]);
    let role = format!(
        "worktree-enrollment:{}",
        sha256(h.target.to_str().unwrap().as_bytes())
    );
    assert!(
        snapshot["roots"][&role]
            .as_array()
            .is_none_or(|ids| !ids.contains(&output["id"]))
    );
    assert!(read_guard(&h.root, path).is_err());
    drop(reader);
    maintain_original_until_absent(&inventory, &h.root.join(path));
    for _ in 0..8 {
        inventory.maintain_explicit().unwrap();
    }
    assert!(!retention_state::object_path(&h.root, output["id"].as_str().unwrap()).exists());
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(
        code, 0,
        "transferred facts must work without their original: {report}"
    );
    assert_eq!(
        json(&fs::read(h.record.join("ledger.json")).unwrap()).unwrap(),
        after
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn invalid_terminal_original_handoff_preserves_ledger_and_existing_consumer() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for fault in [
        "schema",
        "path",
        "attachment",
        "head",
        "retained-commit",
        "dispose-evidence",
        "artifacts-only",
        "handoff",
        "unsealed",
        "changed",
        "missing",
    ] {
        let h = terminal_original_fixture(fault);
        let path = ".chrono-harness/state/terminal-original.json";
        let ledger = fs::read(h.record.join("ledger.json")).unwrap();
        let raw = fs::read(h.root.join(path)).ok();
        let role = format!(
            "worktree-enrollment:{}",
            sha256(h.target.to_str().unwrap().as_bytes())
        );
        let role_path = retention_state::role_path(&h.root, &role);
        let role_bytes = fs::read(&role_path).unwrap();
        let (code, report) = h.invoke("maintain", false);
        assert_ne!(code, 0, "{fault}: {report}");
        assert_eq!(
            fs::read(h.record.join("ledger.json")).unwrap(),
            ledger,
            "{fault}"
        );
        assert_eq!(fs::read(&role_path).unwrap(), role_bytes, "{fault}");
        assert_eq!(fs::read(h.root.join(path)).ok(), raw, "{fault}");
    }
}

#[cfg(not(verus_keep_ghost))]
fn cleanup_result_handoff_fixture(fault: &str) -> EnrollmentOriginalsFixture {
    use chrono_harness::retained_artifacts::retention::Inventory;
    let h = terminal_original_fixture("");
    let path = ".chrono-harness/state/worktrees/cleanup-published.json";
    fs::create_dir_all(h.root.join(".chrono-harness/state/worktrees")).unwrap();
    let ledger_path = h.record.join("ledger.json");
    let mut ledger = json(&fs::read(&ledger_path).unwrap()).unwrap();
    let e = &mut ledger["entries"][0];
    e["attempts"] = value!([{"path":path,"token":"cleanup-published","receipt":null}]);
    let mut report = value!({
        "schema":"chrono-worktree-report/v1","operation":"cleanup","status":"failed",
        "source_root":h.root,"config_path":POLICY,"report_path":path,
        "lock_reason":"cleanup-published","automatic_cleanup":true,
        "terminal_receipt":e["terminal"]["receipt"],"error":"original cleanup failure"
    });
    match fault {
        "schema" => report["schema"] = value!("other/v1"),
        "operation" => report["operation"] = value!("finish"),
        "source" => report["source_root"] = value!(h.target),
        "config" => report["config_path"] = value!(AUTO_POLICY),
        "path" => report["report_path"] = value!(".chrono-harness/state/other.json"),
        "token" => report["lock_reason"] = value!("cleanup-other"),
        "automatic" => report["automatic_cleanup"] = value!(false),
        "terminal" => report["terminal_receipt"]["sha256"] = value!("0".repeat(64)),
        "status" => report["status"] = value!("unfinished"),
        "handoff" => report["terminal_handoff"] = value!(null),
        "current" => {
            let t = &e["terminal"];
            let mut handoff = value!({"schema":"chrono-worktree-terminal-handoff/v1", "path":e["path"],
                "attachment":e["attachment"],"head":t["head"],"retained_commit":t["retained_commit"],
                "dispose_evidence":t["dispose_evidence"],"artifacts_only":t["artifacts_only"],"source":t["receipt"]});
            handoff["established"] = value!(chrono_harness::wire::digest(&handoff).unwrap());
            report["terminal_handoff"] = handoff;
            report.as_object_mut().unwrap().remove("terminal_receipt");
        }
        "cleaned" => {
            report["status"] = value!("cleaned");
            report["worktree_removal"] = value!("removed");
        }
        "input" => {
            report["original_result"] = value!({"input":{
                "schema":"chrono-worktree-retained-input/v1","source_root":h.root,
                "path":".chrono-harness/state/absent-input.bin","sha256":"0".repeat(64),
                "byte_length":0,"format":"bytes"
            }})
        }
        _ => (),
    }
    fs::write(&ledger_path, serde_json::to_vec(&ledger).unwrap()).unwrap();
    let raw = serde_json::to_vec(&report).unwrap();
    fs::write(h.root.join(path), &raw).unwrap();
    let publication = Inventory::adopted(&h.root)
        .unwrap()
        .unwrap()
        .begin("lifecycle-original", path, value!(null))
        .unwrap();
    if fault != "unsealed" {
        publication
            .complete(value!({"status":report["status"]}))
            .unwrap();
    }
    let role = format!(
        "worktree-enrollment:{}",
        sha256(h.target.to_str().unwrap().as_bytes())
    );
    Inventory::adopted(&h.root)
        .unwrap()
        .unwrap()
        .reference_paths(
            &role,
            &[
                ".chrono-harness/state/terminal-original.json".into(),
                path.into(),
            ],
        )
        .unwrap();
    match fault {
        "changed" => fs::write(h.root.join(path), b"changed original\n").unwrap(),
        "partial" => fs::write(h.root.join(path), b"{").unwrap(),
        "missing" => fs::remove_file(h.root.join(path)).unwrap(),
        "symlink" => {
            fs::remove_file(h.root.join(path)).unwrap();
            fs::write(h.root.join("outside-original.json"), &raw).unwrap();
            std::os::unix::fs::symlink(h.root.join("outside-original.json"), h.root.join(path))
                .unwrap();
        }
        _ => (),
    }
    h
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn sealed_cleanup_result_recovers_expired_pending_receipt_without_promoting_state() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::read_guard;
    for outcome in ["", "cleaned", "current"] {
        let h = cleanup_result_handoff_fixture(outcome);
        let path = ".chrono-harness/state/worktrees/cleanup-published.json";
        let raw = fs::read(h.root.join(path)).unwrap();
        let ledger_path = h.record.join("ledger.json");
        let before = json(&fs::read(&ledger_path).unwrap()).unwrap();
        let mut output = retention_state::snapshot(&h.root)["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|output| output["path"] == path)
            .unwrap()
            .clone();
        output["lifetime"] = value!({"created_at":1,"deadline":2});
        output["expires"] = value!(2);
        retention_state::replace_output(&h.root, &output);
        assert!(
            read_guard(&h.root, path).is_err(),
            "new readers cannot renew expired output"
        );
        let (code, report) = h.invoke("maintain", false);
        assert_eq!(code, 0, "{outcome}: {report}");
        let after = json(&fs::read(&ledger_path).unwrap()).unwrap();
        assert_eq!(
            after["entries"][0]["attempts"][0]["receipt"],
            value!({"path":path,"sha256":sha256(&raw)})
        );
        let mut unchanged = after.clone();
        unchanged["entries"][0]["attempts"][0]["receipt"] = value!(null);
        unchanged["entries"][0]["terminal"]
            .as_object_mut()
            .unwrap()
            .remove("established");
        assert_eq!(
            unchanged, before,
            "receipt recovery must not manufacture a cleanup outcome"
        );
        assert_eq!(fs::read(h.root.join(path)).unwrap(), raw);
        let snapshot = retention_state::snapshot(&h.root);
        let observed = snapshot["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["id"] == output["id"])
            .unwrap();
        assert_eq!(observed["lifetime"], output["lifetime"]);
        assert_eq!(observed["expires"], output["expires"]);
        let (code, report) = h.invoke("maintain", false);
        assert_eq!(code, 0, "repeated handoff: {report}");
        assert_eq!(json(&fs::read(&ledger_path).unwrap()).unwrap(), after);
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn invalid_cleanup_result_preserves_pending_receipt_and_original_consumer() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for fault in [
        "schema",
        "operation",
        "source",
        "config",
        "path",
        "token",
        "automatic",
        "terminal",
        "status",
        "handoff",
        "input",
        "unsealed",
        "changed",
        "partial",
        "missing",
        "symlink",
    ] {
        let h = cleanup_result_handoff_fixture(fault);
        let ledger_path = h.record.join("ledger.json");
        // Settle the independent terminal transfer before measuring this handoff.
        // Its valid facts are unrelated to the invalid cleanup original.
        let mut before = json(&fs::read(&ledger_path).unwrap()).unwrap();
        let e = &before["entries"][0];
        let t = &e["terminal"];
        let facts = value!({"schema":"chrono-worktree-terminal-handoff/v1", "path":e["path"],
            "attachment":e["attachment"],"head":t["head"],"retained_commit":t["retained_commit"],
            "dispose_evidence":t["dispose_evidence"],"artifacts_only":t["artifacts_only"],"source":t["receipt"]});
        before["entries"][0]["terminal"]["established"] =
            value!(chrono_harness::wire::digest(&facts).unwrap());
        let before = serde_json::to_vec(&before).unwrap();
        fs::write(&ledger_path, &before).unwrap();
        let role = format!(
            "worktree-enrollment:{}",
            sha256(h.target.to_str().unwrap().as_bytes())
        );
        chrono_harness::retained_artifacts::retention::Inventory::adopted(&h.root)
            .unwrap()
            .unwrap()
            .prune_reference_paths(
                &role,
                &[".chrono-harness/state/worktrees/cleanup-published.json".into()],
            )
            .unwrap();
        let role_path = retention_state::role_path(&h.root, &role);
        let role_bytes = fs::read(&role_path).unwrap();
        let original_path = h
            .root
            .join(".chrono-harness/state/worktrees/cleanup-published.json");
        let original = fs::read(&original_path).ok();
        let symlink = fs::symlink_metadata(&original_path)
            .ok()
            .map(|m| m.file_type().is_symlink());
        let (code, report) = h.invoke("maintain", false);
        assert_ne!(code, 0, "{fault}: {report}");
        assert_eq!(fs::read(&ledger_path).unwrap(), before, "{fault}");
        assert_eq!(fs::read(&role_path).unwrap(), role_bytes, "{fault}");
        assert_eq!(fs::read(&original_path).ok(), original, "{fault}");
        assert_eq!(
            fs::symlink_metadata(&original_path)
                .ok()
                .map(|m| m.file_type().is_symlink()),
            symlink,
            "{fault}"
        );
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn failed_terminal_handoff_write_preserves_original_roots_then_resumes() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let h = terminal_original_fixture("");
        let before = fs::read(h.record.join("ledger.json")).unwrap();
        let role = format!(
            "worktree-enrollment:{}",
            sha256(h.target.to_str().unwrap().as_bytes())
        );
        let role_path = retention_state::role_path(&h.root, &role);
        let role_bytes = fs::read(&role_path).unwrap();
        let permissions = fs::metadata(&h.record).unwrap().permissions();
        fs::set_permissions(&h.record, fs::Permissions::from_mode(0o500)).unwrap();
        let (code, report) = h.invoke("maintain", false);
        fs::set_permissions(&h.record, permissions).unwrap();
        assert_ne!(code, 0, "{report}");
        assert!(report.to_string().contains("Permission denied"), "{report}");
        assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), before);
        assert_eq!(fs::read(&role_path).unwrap(), role_bytes);
        let (code, report) = h.invoke("maintain", false);
        assert_eq!(code, 0, "{report}");
        assert!(json(&fs::read(h.record.join("ledger.json")).unwrap()).unwrap()["entries"][0]["terminal"]["established"].is_string());
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn conflicting_terminal_handoff_preserves_original_role_and_fact_binding() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for field in [
        "established",
        "head",
        "retained_commit",
        "dispose_evidence",
        "artifacts_only",
        "receipt",
        "attachment",
    ] {
        let h = terminal_original_fixture("");
        let path = ".chrono-harness/state/terminal-original.json";
        let role = format!(
            "worktree-enrollment:{}",
            sha256(h.target.to_str().unwrap().as_bytes())
        );
        let role_path = retention_state::role_path(&h.root, &role);
        let _reader =
            chrono_harness::retained_artifacts::retention::read_guard(&h.root, path).unwrap();
        let (code, report) = h.invoke("maintain", false);
        assert_eq!(code, 0, "{field}: {report}");
        // Recreate the stale role membership through the producer API to
        // exercise recovery after a write succeeded but pruning was interrupted.
        chrono_harness::retained_artifacts::retention::Inventory::adopted(&h.root)
            .unwrap()
            .unwrap()
            .reference_paths(&role, &[path.into()])
            .unwrap();
        let ledger_path = h.record.join("ledger.json");
        let mut ledger = json(&fs::read(&ledger_path).unwrap()).unwrap();
        let entry = &mut ledger["entries"][0];
        match field {
            "attachment" => entry["attachment"]["metadata_id"] = value!("other"),
            "receipt" => entry["terminal"]["receipt"]["sha256"] = value!("d".repeat(64)),
            "dispose_evidence" | "artifacts_only" => {
                let original = entry["terminal"][field].as_bool().unwrap();
                entry["terminal"][field] = value!(!original);
            }
            _ => entry["terminal"][field] = value!("d".repeat(64)),
        }
        let ledger = serde_json::to_vec(&ledger).unwrap();
        fs::write(&ledger_path, &ledger).unwrap();
        let role_bytes = fs::read(&role_path).unwrap();
        let raw = fs::read(h.root.join(path)).unwrap();
        let (code, report) = h.invoke("maintain", false);
        assert_ne!(code, 0, "{field}: {report}");
        assert!(
            report
                .to_string()
                .contains("terminal handoff binding changed"),
            "{field}: {report}"
        );
        assert_eq!(fs::read(ledger_path).unwrap(), ledger, "{field}");
        assert_eq!(fs::read(role_path).unwrap(), role_bytes, "{field}");
        assert_eq!(fs::read(h.root.join(path)).unwrap(), raw, "{field}");
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn new_terminal_facts_survive_original_retirement_through_cleanup_and_history_retirement() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
    let h = Host::new("payload");
    h.kernel_cleanup();
    adopt_lifecycle_retention(&h);
    let retention_path = h.root.join(POLICY_PATH);
    let mut retention = json(&fs::read(&retention_path).unwrap()).unwrap();
    retention["producers"]["lifecycle-original"] = value!([
        ".chrono-harness/state/worktrees/",
        ".chrono-harness/state/automatic-cleanup/"
    ]);
    retention["max_related_outputs"] = value!(512);
    fs::write(retention_path, serde_json::to_vec(&retention).unwrap()).unwrap();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("handoff-without-original");
    let (code, created, error) = h.invoke("integration", "handoff-without-original", &target);
    assert_eq!(code, 0, "{created} {error}");
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    let terminal = h.ledger()["entries"][0]["terminal"].clone();
    assert!(terminal["established"].is_string());
    assert!(h.ledger()["entries"][0]["cleanup"]["report_path"].is_null());
    let source = terminal["receipt"]["path"].as_str().unwrap();
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    maintain_original_until_absent(&inventory, &h.root.join(source));
    assert!(target.join("payload").is_file());
    let (code, observed, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{observed} {error}");
    let row = observed["worktree_status"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target))
        .unwrap();
    assert!(row["terminal_receipt"].is_null());
    assert_eq!(
        row["terminal_handoff"]["established"],
        terminal["established"]
    );
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!target.exists());
    let cleanup = drain_report(&maintained["drain"][0]);
    assert_eq!(
        cleanup["terminal_handoff"]["established"],
        terminal["established"]
    );
    assert!(cleanup["terminal_receipt"].is_null());
    assert!(cleanup["terminal_input"].is_null());
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(h.ledger()["entries"].as_array().unwrap().is_empty());
    assert!(!h.record_directory(&target).exists());
    assert!(!h.root.join(source).exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn absent_import_restores_adopted_caller_policy_without_reinterpreting_target_inputs() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::imported();
    let ledger = json(&fs::read(h.record.join("ledger.json")).unwrap()).unwrap();
    let original = fs::read(h.root.join(&h.original)).unwrap();
    let _reader =
        chrono_harness::retained_artifacts::retention::read_guard(&h.root, &h.original).unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["cleanup_failures"], value!([]), "{report}");
    assert_eq!(attention_row(&report, &h.target)["work_state"], "unknown");
    assert_import_policy_handoff(&h, &ledger);
    assert_eq!(fs::read(h.root.join(&h.original)).unwrap(), original);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn present_import_restores_adopted_caller_policy_without_reinterpreting_target_inputs() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::imported();
    h.make_present();
    let ledger = json(&fs::read(h.record.join("ledger.json")).unwrap()).unwrap();
    let original = fs::read(h.root.join(&h.original)).unwrap();
    let _reader =
        chrono_harness::retained_artifacts::retention::read_guard(&h.root, &h.original).unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["cleanup_failures"], value!([]), "{report}");
    assert_import_policy_handoff(&h, &ledger);
    assert_eq!(fs::read(h.root.join(&h.original)).unwrap(), original);
    for (path, expected) in [
        (POLICY, "target original configuration is raw data"),
        (AUTO_POLICY, "target original policy is different raw data"),
        ("unsaved-source.rs", "unique source\n"),
    ] {
        assert_eq!(fs::read(h.target.join(path)).unwrap(), expected.as_bytes());
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn import_uses_original_fixed_caller_source_only_when_policy_field_is_absent() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for (present, fault) in [
        (false, "source-only"),
        (true, "source-only"),
        (false, "explicit-policy-source"),
        (true, "explicit-policy-source"),
    ] {
        let h = EnrollmentOriginalsFixture::build(Some(fault));
        if present {
            h.make_present();
        }
        let ledger = json(&fs::read(h.record.join("ledger.json")).unwrap()).unwrap();
        let original = fs::read(h.root.join(&h.original)).unwrap();
        let _reader =
            chrono_harness::retained_artifacts::retention::read_guard(&h.root, &h.original)
                .unwrap();
        let (code, report) = h.invoke("maintain", false);
        assert_eq!(code, 0, "{report}");
        assert_eq!(report["cleanup_failures"], value!([]), "{report}");
        assert_import_policy_handoff(&h, &ledger);
        assert_eq!(fs::read(h.root.join(&h.original)).unwrap(), original);
        assert_eq!(h.target.exists(), present);
        if present {
            assert_eq!(
                fs::read(h.target.join("unsaved-source.rs")).unwrap(),
                b"unique source\n"
            );
        } else {
            assert_eq!(attention_row(&report, &h.target)["work_state"], "unknown");
        }
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn absent_import_after_explicit_migration_uses_the_last_target_binding() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::Inventory;
    let h = EnrollmentOriginalsFixture::build(Some("missing-source"));
    let fixture_path = h.root.join("git-fixture.json");
    let mut fixture = json(&fs::read(&fixture_path).unwrap()).unwrap();
    let mut snapshot = fixture["retained"]["c".repeat(64)].clone();
    let config_bytes = format!("{}\n", snapshot[POLICY].as_str().unwrap());
    snapshot[POLICY] = value!(config_bytes);
    let policy_bytes = snapshot[AUTO_POLICY].as_str().unwrap().to_owned();
    let head = "d".repeat(64);
    fixture["retained"][&head] = snapshot;
    fs::write(fixture_path, serde_json::to_vec(&fixture).unwrap()).unwrap();
    let ledger_path = h.record.join("ledger.json");
    let mut ledger = json(&fs::read(&ledger_path).unwrap()).unwrap();
    let entry = &mut ledger["entries"][0];
    let migration = value!({
        "schema":"chrono-worktree-policy-migration/v1",
        "binding":{
            "path":entry["path"],"branch":entry["branch"],"attachment":entry["attachment"],
            "policy_sha256":entry["policy_sha256"],"observed_head":entry["enrollment"]["observed_head"],
            "policy_inputs":entry["enrollment"]["policy_inputs"],
            "original_receipt":entry["enrollment"]["receipt"],"ownership":null
        },
        "previous":{"policy_sha256":entry["policy_sha256"],
            "head":entry["enrollment"]["observed_head"],"inputs":entry["enrollment"]["policy_inputs"]},
        "next":{"policy_sha256":sha256(policy_bytes.as_bytes()),"head":head,"inputs":{
            (POLICY):sha256(config_bytes.as_bytes()),(AUTO_POLICY):sha256(policy_bytes.as_bytes())
        }},
        "terminal_handoff":"not-claimed","disposal":"not-performed"
    });
    let path = ".chrono-harness/state/import-policy-migration.json";
    let bytes = serde_json::to_vec(&migration).unwrap();
    fs::write(h.root.join(path), &bytes).unwrap();
    Inventory::adopted(&h.root)
        .unwrap()
        .unwrap()
        .begin("lifecycle-original", path, value!({"status":"unknown"}))
        .unwrap()
        .complete(value!({"status":"published"}))
        .unwrap();
    entry["policy_migrations"] = value!([{"path":path,"sha256":sha256(&bytes)}]);
    let ledger_bytes = serde_json::to_vec(&ledger).unwrap();
    fs::write(&ledger_path, &ledger_bytes).unwrap();
    let original = fs::read(h.root.join(&h.original)).unwrap();
    let _reader =
        chrono_harness::retained_artifacts::retention::read_guard(&h.root, &h.original).unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["cleanup_failures"], value!([]), "{report}");
    assert_eq!(fs::read(&ledger_path).unwrap(), ledger_bytes);
    assert_eq!(fs::read(h.root.join(&h.original)).unwrap(), original);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn absent_import_preserves_unqualified_originals_and_rejects_source_drift() {
    assert_import_recovery_rejects_faults(false);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn present_import_preserves_unqualified_originals_and_rejects_source_drift() {
    assert_import_recovery_rejects_faults(true);
}

#[cfg(not(verus_keep_ghost))]
fn assert_import_recovery_rejects_faults(present: bool) {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for fault in [
        "missing-source",
        "null-policy-source",
        "invalid-policy-source",
        "short-policy-source",
        "source-only-null",
        "source-only-invalid",
        "source-only-short",
        "source-only-config-bytes",
        "source-only-policy-bytes",
        "schema",
        "operation",
        "status",
        "report-path",
        "source-root",
        "config-path",
        "config-digest",
        "retained-input",
        "unsealed",
        "config-bytes",
        "policy-bytes",
        "policy-address",
        "namespace",
        "registration",
        "replaced",
        "changed",
        "missing",
        "symlink",
    ] {
        let h = EnrollmentOriginalsFixture::build(Some(fault));
        if present {
            h.make_present();
        }
        let original_path = h.root.join(&h.original);
        match fault {
            "replaced" => {
                let bytes = fs::read(&original_path).unwrap();
                fs::rename(&original_path, h.root.join("original-before-replacement")).unwrap();
                fs::write(&original_path, bytes).unwrap();
            }
            "changed" => {
                let mut bytes = fs::read(&original_path).unwrap();
                bytes.push(b'\n');
                fs::write(&original_path, bytes).unwrap();
            }
            "missing" => fs::remove_file(&original_path).unwrap(),
            "symlink" => {
                let kept = h.root.join("original-before-symlink");
                fs::rename(&original_path, &kept).unwrap();
                std::os::unix::fs::symlink(kept, &original_path).unwrap();
            }
            _ => (),
        }
        let ledger = fs::read(h.record.join("ledger.json")).unwrap();
        let original = fs::read(&original_path).ok();
        let (code, report) = h.invoke("maintain", false);
        assert_eq!(code, 2, "{fault}: {report}");
        assert!(
            !report["cleanup_failures"].as_array().unwrap().is_empty(),
            "{fault}: {report}"
        );
        assert_eq!(
            fs::read(h.record.join("ledger.json")).unwrap(),
            ledger,
            "{fault}"
        );
        assert_eq!(fs::read(&original_path).ok(), original, "{fault}");
        assert_eq!(h.target.exists(), present, "{fault}");
        if present {
            assert_eq!(
                fs::read(h.target.join("unsaved-source.rs")).unwrap(),
                b"unique source\n",
                "{fault}"
            );
        }
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn absent_adoption_reads_only_bound_policy_addresses_and_preserves_originals() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::new();
    let mut config = json(&fs::read(h.root.join(POLICY)).unwrap()).unwrap();
    config["git"] = value!({"execution-only":"not an executable choice"});
    config["unconsumed"] = value!({"arbitrary":[null, true, "retained data"]});
    h.adopt_retained_inputs(config);
    let ledger = fs::read(h.record.join("ledger.json")).unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["cleanup_failures"], value!([]), "{report}");
    assert_eq!(attention_row(&report, &h.target)["work_state"], "unknown");
    assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), ledger);
    assert_eq!(
        fs::read(h.root.join(&h.original)).unwrap(),
        b"original failure bytes\n"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn absent_adoption_does_not_execute_coordinator_producer_declarations() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::new();
    let config = json(&fs::read(h.root.join(POLICY)).unwrap()).unwrap();
    let mut policy = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
    policy["retained_producers"] = value!({"unconsumed":[null, true, "fixed data"]});
    h.adopt_retained_policy(config, policy);
    let ledger = fs::read(h.record.join("ledger.json")).unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["cleanup_failures"], value!([]), "{report}");
    assert_eq!(attention_row(&report, &h.target)["work_state"], "unknown");
    assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), ledger);
    assert_eq!(
        fs::read(h.root.join(&h.original)).unwrap(),
        b"original failure bytes\n"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn absent_adoption_keeps_consumed_policy_and_live_producer_validation() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for live in [false, true] {
        let h = EnrollmentOriginalsFixture::new();
        let config = json(&fs::read(h.root.join(POLICY)).unwrap()).unwrap();
        let mut policy = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
        if live {
            policy["retained_producers"] = value!({"invalid":"executed owner declaration"});
            fs::write(
                h.root.join(AUTO_POLICY),
                serde_json::to_vec(&policy).unwrap(),
            )
            .unwrap();
        } else {
            policy["artifacts"] = value!({"invalid":"consumed cleanup declaration"});
        }
        h.adopt_retained_policy(config, policy);
        let ledger = fs::read(h.record.join("ledger.json")).unwrap();
        let (code, report) = h.invoke("maintain", false);
        assert_eq!(code, 2, "live={live}: {report}");
        assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), ledger);
        assert_eq!(
            fs::read(h.root.join(&h.original)).unwrap(),
            b"original failure bytes\n"
        );
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn absent_adoption_rejects_invalid_addresses_and_original_drift() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for fault in ["host-type", "policy-type", "relocated-policy", "bytes"] {
        let h = EnrollmentOriginalsFixture::new();
        let mut config = json(&fs::read(h.root.join(POLICY)).unwrap()).unwrap();
        match fault {
            "host-type" => config["host_config"] = value!(null),
            "policy-type" => config["automatic_cleanup"] = value!(7),
            "relocated-policy" => {
                config["automatic_cleanup"] = value!(".chrono-harness/other.json")
            }
            "bytes" => (),
            _ => unreachable!(),
        }
        h.adopt_retained_inputs(config);
        if fault == "bytes" {
            let path = h.root.join("git-fixture.json");
            let mut fixture = json(&fs::read(&path).unwrap()).unwrap();
            let bytes = fixture["retained"]["b".repeat(64)][POLICY]
                .as_str()
                .unwrap()
                .to_owned();
            fixture["retained"]["b".repeat(64)][POLICY] = value!(format!("{bytes}\n"));
            fs::write(path, serde_json::to_vec(&fixture).unwrap()).unwrap();
        }
        let ledger = fs::read(h.record.join("ledger.json")).unwrap();
        let (code, report) = h.invoke("maintain", false);
        assert_eq!(code, 2, "{fault}: {report}");
        assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), ledger);
        assert_eq!(
            fs::read(h.root.join(&h.original)).unwrap(),
            b"original failure bytes\n",
            "{fault}: {report}"
        );
        let role = format!(
            "worktree-enrollment:{}",
            sha256(h.target.to_str().unwrap().as_bytes())
        );
        assert_eq!(
            retention_state::snapshot(&h.root)["roots"][&role]
                .as_array()
                .map(Vec::len),
            Some(1),
            "{fault}: {report}"
        );
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn status_keeps_enrollment_originals_after_failed_absent_policy_read() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::new();
    // The mutating lifecycle owner establishes the original consumers before
    // trying to interpret the absent checkout's policy. Observation only reads.
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("mock retained policy read failed"),
        "{report}"
    );
    let role = format!(
        "worktree-enrollment:{}",
        sha256(h.target.to_str().unwrap().as_bytes())
    );
    let state = retention_state::snapshot(&h.root);
    assert_eq!(
        state["roots"][&role].as_array().map(Vec::len),
        Some(1),
        "a policy failure must not prevent the verified ledger from protecting its original"
    );
    let ledger = fs::read(h.record.join("ledger.json")).unwrap();
    let owner = fs::read(h.record.join("owner.json")).unwrap();
    let (code, observed) = h.invoke("status", true);
    assert_eq!(code, 0, "{observed}");
    assert_eq!(attention_row(&observed, &h.target)["work_state"], "unknown");
    assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), ledger);
    assert_eq!(fs::read(h.record.join("owner.json")).unwrap(), owner);
    assert_eq!(
        retention_state::snapshot(&h.root)["roots"][&role],
        state["roots"][&role],
        "observation must preserve the lifecycle owner's original consumers"
    );
    assert_eq!(
        fs::read(h.root.join(&h.original)).unwrap(),
        b"original failure bytes\n"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn enrollment_originals_are_protected_before_global_maintenance() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::new();
    let ledger = fs::read(h.record.join("ledger.json")).unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("mock retained policy read failed"),
        "{report}"
    );
    assert_ne!(
        report["output_retention"]["status"], "preserved",
        "a policy failure after verified original protection must not defer independent retention: {report}"
    );
    assert!(
        report["drain"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["path"] == value!(h.target)
                && row["original_protection"]["status"] == "protected"),
        "the failed cleanup must separately report current original protection: {report}"
    );
    // A 25 ms round may publish only the disposal intent. Continue the same
    // bounded owner maintenance; the failed policy's input must stay rooted.
    let inventory = chrono_harness::retained_artifacts::retention::Inventory::adopted(&h.root)
        .unwrap()
        .unwrap();
    maintain_original_until_absent(&inventory, &h.root.join(&h.sibling));
    assert_eq!(
        fs::read(h.root.join(&h.original)).unwrap(),
        b"original failure bytes\n"
    );
    assert!(
        !h.root.join(&h.sibling).exists(),
        "independent released originals can still be reclaimed: {report}"
    );
    assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), ledger);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn empty_admission_preserves_foreign_checkout_without_reading_its_policy() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::new();
    let ledger_path = h.record.join("ledger.json");
    let mut ledger = json(&fs::read(&ledger_path).unwrap()).unwrap();
    ledger["entries"] = value!([]);
    let ledger_bytes = serde_json::to_vec(&ledger).unwrap();
    fs::write(&ledger_path, &ledger_bytes).unwrap();
    fs::create_dir_all(h.target.join(".chrono-harness")).unwrap();
    let foreign_policy = h.target.join(POLICY);
    let foreign_bytes = b"this checkout has not adopted this coordinator's policy\n";
    fs::write(&foreign_policy, foreign_bytes).unwrap();
    let foreign_retention = h
        .target
        .join(chrono_harness::retained_artifacts::retention::POLICY_PATH);
    let foreign_retention_bytes = b"this retention store has not adopted this coordinator\n";
    fs::write(&foreign_retention, foreign_retention_bytes).unwrap();
    let source = h.target.join("unsaved-source");
    fs::write(&source, b"independent work\n").unwrap();

    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 0, "{report}");
    assert!(report["cleanup_failures"].as_array().unwrap().is_empty());
    assert_ne!(report["output_retention"]["status"], "preserved");
    assert_eq!(fs::read(&ledger_path).unwrap(), ledger_bytes);
    assert_eq!(fs::read(&foreign_policy).unwrap(), foreign_bytes);
    assert_eq!(
        fs::read(&foreign_retention).unwrap(),
        foreign_retention_bytes
    );
    assert!(
        report["linked_output_retention"]
            .as_array()
            .is_none_or(|rows| rows.is_empty())
    );
    assert_eq!(fs::read(&source).unwrap(), b"independent work\n");
    let inventory = chrono_harness::retained_artifacts::retention::Inventory::adopted(&h.root)
        .unwrap()
        .unwrap();
    maintain_original_until_absent(&inventory, &h.root.join(&h.sibling));
    assert!(!h.root.join(&h.sibling).exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn enrollment_originals_unknown_metadata_defers_only_global_original_maintenance() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::new();
    fs::create_dir(h.record.join("gate.lock")).unwrap();
    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 2, "{report}");
    assert_eq!(report["complete_inventory"]["status"], "incomplete");
    assert!(
        h.root.join(&h.original).is_file(),
        "busy metadata has unverified consumers: {report}"
    );
    assert!(h.root.join(&h.sibling).is_file());
    assert_eq!(report["output_retention"]["status"], "preserved");
    assert!(
        report["drain"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["path"] == value!(h.target)
                && row["status"] == "preserved"
                && row["original_protection"]["status"] == "unknown")
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn enrollment_originals_changed_identity_still_defers_global_maintenance() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = EnrollmentOriginalsFixture::new();
    let owner_path = h.record.join("owner.json");
    let mut owner = json(&fs::read(&owner_path).unwrap()).unwrap();
    owner["path"] = value!(h.root.join("another-checkout"));
    let owner_bytes = serde_json::to_vec(&owner).unwrap();
    fs::write(&owner_path, &owner_bytes).unwrap();
    let ledger = fs::read(h.record.join("ledger.json")).unwrap();

    let (code, report) = h.invoke("maintain", false);
    assert_eq!(code, 2, "{report}");
    assert_eq!(report["output_retention"]["status"], "preserved");
    assert_eq!(report["output_retention"]["namespace_complete"], false);
    assert!(
        report["complete_inventory"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error
                .as_str()
                .unwrap()
                .contains("enrollment declaration address differs from its namespace"))
    );
    assert_eq!(fs::read(&owner_path).unwrap(), owner_bytes);
    assert_eq!(fs::read(h.record.join("ledger.json")).unwrap(), ledger);
    assert_eq!(
        fs::read(h.root.join(&h.original)).unwrap(),
        b"original failure bytes\n"
    );
    assert!(h.root.join(&h.sibling).is_file());
}

#[cfg(not(verus_keep_ghost))]
impl Host {
    pub(super) fn automatic(&self, disposition: &str) {
        let mut config = json(&fs::read(self.root.join(CONFIG)).unwrap()).unwrap();
        for artifact in [
            value!({"path":"output λ/","owner":"host","kind":"arbitrary-output","tracked":false}),
            value!({"path":"nested/cache/","owner":"host","kind":"another-layout","tracked":false}),
        ] {
            if !config["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["path"] == artifact["path"])
            {
                config["artifacts"].as_array_mut().unwrap().push(artifact);
            }
        }
        fs::write(
            self.root.join(CONFIG),
            serde_json::to_vec_pretty(&config).unwrap(),
        )
        .unwrap();
        let mut fm = json(&fs::read(self.root.join(FM)).unwrap()).unwrap();
        fm["files"]
            .as_array_mut()
            .unwrap()
            .push(file(AUTO_POLICY, value!([])));
        fs::write(self.root.join(FM), serde_json::to_vec_pretty(&fm).unwrap()).unwrap();
        fs::write(
            self.root.join(AUTO_POLICY),
            serde_json::to_vec_pretty(&value!({
                "schema":"chrono-worktree-automatic-cleanup/v1",
                "coordinator_root":self.root,
                "state_directory":".chrono-harness/state/automatic-cleanup/",
                "retained_ref":"refs/heads/dev", "retention":"ancestor", "remove_branch":false,
                "allow_evidence_disposal":true,
                "artifacts":[
                    {"path":"output λ/","disposition":"dispose"},
                    {"path":"nested/cache/","disposition":"dispose"},
                    {"path":".chrono-harness/state/","disposition":disposition},
                    {"path":".chrono-harness/bin/","disposition":"retain"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            self.root.join(".gitignore"),
            ".chrono-harness/state/\n.chrono-harness/bin/\noutput λ/\nnested/cache/\n",
        )
        .unwrap();
        self.policy(|p| p["automatic_cleanup"] = value!(AUTO_POLICY));
    }
    pub(super) fn auto_command(&self, command: &str, extra: &[&str]) -> Command {
        self.auto_command_at(&self.root, command, extra)
    }
    pub(super) fn auto_command_at(
        &self,
        invoking: &Path,
        command: &str,
        extra: &[&str],
    ) -> Command {
        let mut c = Command::new(Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-cli")));
        c.current_dir(invoking)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args([
                command,
                "--host-root",
                invoking.to_str().unwrap(),
                "--config",
                POLICY,
            ])
            .args(extra);
        self._dir.bind(&mut c);
        c
    }
    pub(super) fn auto(&self, command: &str, extra: &[&str]) -> (i32, Value, String) {
        let mut command = self.auto_command(command, extra);
        self.received(self._dir.capture_bound(&mut command))
    }
    pub(super) fn ledger(&self) -> Value {
        snapshot(&self.root).unwrap()
    }
    pub(super) fn ledger_path(&self, target: &Path) -> PathBuf {
        self.record_directory(target).join("ledger.json")
    }
    pub(super) fn record_directory(&self, target: &Path) -> PathBuf {
        self.root
            .join(".chrono-harness/state/automatic-cleanup/enrollments")
            .join(sha256(target.to_str().unwrap().as_bytes()))
    }
    pub(super) fn set_ledger(&self, ledger: &Value) {
        let directory = self
            .root
            .join(".chrono-harness/state/automatic-cleanup/enrollments");
        for item in fs::read_dir(directory).unwrap() {
            let path = item.unwrap().path().join("ledger.json");
            let mut current = json(&fs::read(&path).unwrap()).unwrap();
            let marker =
                json(&fs::read(path.parent().unwrap().join("owner.json")).unwrap()).unwrap();
            current["entries"] = value!(
                ledger["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|entry| entry["path"] == marker["path"])
                    .cloned()
                    .collect::<Vec<_>>()
            );
            fs::write(path, serde_json::to_vec_pretty(&current).unwrap()).unwrap();
        }
    }
}
#[cfg(not(verus_keep_ghost))]
pub(super) fn snapshot(root: &Path) -> Result<Value, String> {
    let mut ledger = value!({"entries":[]});
    let namespace = root.join(".chrono-harness/state/automatic-cleanup/enrollments");
    for item in fs::read_dir(namespace).map_err(|e| e.to_string())? {
        let bytes = fs::read(item.map_err(|e| e.to_string())?.path().join("ledger.json"))
            .map_err(|e| e.to_string())?;
        let current = json(&bytes)?;
        ledger["entries"]
            .as_array_mut()
            .unwrap()
            .extend(current["entries"].as_array().unwrap().iter().cloned());
    }
    ledger["entries"]
        .as_array_mut()
        .unwrap()
        .sort_by_key(|entry| {
            entry["enrollment"]["receipt"]["report_path"]
                .as_str()
                .unwrap_or("")
                .split('-')
                .rev()
                .nth(1)
                .unwrap_or("")
                .to_owned()
        });
    Ok(ledger)
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn output(target: &Path) {
    fs::create_dir_all(target.join("output λ")).unwrap();
    fs::write(target.join("output λ/data"), vec![42u8; 8192]).unwrap();
}

#[cfg(not(verus_keep_ghost))]
fn capacity_fixture_with_diagnostics(
    host: &Host,
    directory: &Path,
    body: impl FnOnce(),
) -> std::thread::Result<()> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
    if result.is_err() {
        host.retain_fixture_diagnostics(directory);
    }
    result
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn caught_capacity_failure_exports_original_inventory_and_command_streams() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let host = Host::new("payload");
    let destination = tempfile::tempdir().unwrap();
    let originals = [
        (
            "output-retention-v1/objects/output-diagnostic/record.json",
            b"original addressed declaration\0\xff".as_slice(),
        ),
        (
            "capacity-excluded.stdout",
            b"partial stdout\0\xfe".as_slice(),
        ),
        (
            "capacity-excluded.stderr",
            b"original capacity refusal\0\xfd".as_slice(),
        ),
        ("capacity-excluded.status", b"exit status: 2".as_slice()),
    ];
    for (name, bytes) in &originals {
        let path = host.root.join(".chrono-harness/state").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let source_bytes = fs::read(host.root.join("payload")).unwrap();
    let index = fs::read(host.root.join(".git/index")).unwrap();
    let result = capacity_fixture_with_diagnostics(&host, destination.path(), || {
        panic!("controlled original capacity fixture failure");
    });
    let failure = result.unwrap_err();
    assert_eq!(
        failure.downcast_ref::<&str>(),
        Some(&"controlled original capacity fixture failure")
    );
    let retained = destination.path().join(format!(
        "fixture-{}",
        sha256(host.root.as_os_str().as_encoded_bytes())
    ));
    for (name, bytes) in originals {
        assert_eq!(
            fs::read(retained.join("coordinator-state").join(name)).unwrap(),
            bytes
        );
        assert_eq!(
            fs::read(host.root.join(".chrono-harness/state").join(name)).unwrap(),
            bytes
        );
    }
    assert_eq!(fs::read(host.root.join("payload")).unwrap(), source_bytes);
    assert_eq!(fs::read(host.root.join(".git/index")).unwrap(), index);
}

// Exercise the actual lifecycle CLI with the adopted finite owner, not an
// alternate maintenance implementation. Failed cases retain their original host.
#[cfg(not(verus_keep_ghost))]
fn ordinary_report_capacity(
    operation: &str,
    all_rooted: bool,
    invoking_root: impl FnOnce(&Host) -> PathBuf,
) {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
    let h = Host::new("payload");
    let directory = Host::diagnostic_directory(true).unwrap();
    let result = capacity_fixture_with_diagnostics(&h, &directory, || {
        h.kernel_cleanup();
        native_consumer(&h, "ordinary");
        let invoking = invoking_root(&h);
        let bootstrap = ".chrono-harness/ci/capacity.json";
        fs::create_dir_all(h.root.join(".chrono-harness/ci")).unwrap();
        fs::write(h.root.join(bootstrap), serde_json::to_vec(&value!({
            "schema":"chrono-bootstrap/v1", "participation":{"coordinator_root":"git-main-worktree"},
            "entrypoint":{"tool":"consumer","script":"noop"}
        })).unwrap()).unwrap();
        // The registered Rust interface fixture returns immediately for noop;
        // the real bootstrap participation dispatch still resolves and joins it.
        fs::write(h.root.join("noop"), include_bytes!("support/consumer.rs")).unwrap();
        fs::write(
            h.root.join(POLICY_PATH),
            serde_json::to_vec(&value!({
                "schema":"chrono-output-retention/v2",
                "producers":{"check-original":[".chrono-harness/state/"],
                    "lifecycle-original":[".chrono-harness/state/worktrees/"]},
                "keep_seconds":0,"max_related_outputs":512,"max_summaries":128,
                "max_nodes_per_round":256,"max_bytes_per_round":67108864,
                "max_millis_per_round":25,"max_output_bytes":1073741824,
                "max_manifest_nodes":16384,
                "max_metadata_bytes_per_round":67108864
            }))
            .unwrap(),
        )
        .unwrap();
        let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
        for path in [POLICY_PATH, bootstrap, "noop"] {
            fm["files"]
                .as_array_mut()
                .unwrap()
                .push(file(path, value!([])));
        }
        fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
        let source_bytes = fs::read(h.root.join("payload")).unwrap();
        let index = fs::read(h.root.join(".git/index")).unwrap();
        let inventory = || Inventory::adopted(&h.root).unwrap().unwrap();
        let state = || retention_state::snapshot(&h.root);
        fs::create_dir_all(h.root.join(".chrono-harness/state/worktrees")).unwrap();
        let mut ids = Vec::new();
        let mut live = None;
        for n in 0..512 {
            let path = format!(".chrono-harness/state/worktrees/history-{n}.json");
            fs::write(h.root.join(&path), format!("original {n}\n")).unwrap();
            let publication = inventory()
                .begin(
                    "lifecycle-original",
                    &path,
                    value!({"status":"not-established"}),
                )
                .unwrap();
            ids.push(publication.id().to_owned());
            if n == 0 {
                live = Some(publication); // Actual unfinished producer capability.
            } else {
                publication
                    .complete(if n == 1 {
                        value!({"status":"failed","error":"original failure"})
                    } else if n == 2 {
                        value!({"status":"not-established","publication":"interrupted"})
                    } else {
                        value!({"status":"cleaned"})
                    })
                    .unwrap();
            }
        }
        inventory()
            .reference("consumer:lifecycle-recheck:failure", &[ids[1].clone()])
            .unwrap();
        inventory()
            .reference(
                "consumer:publication-recheck:interrupted",
                &[ids[2].clone()],
            )
            .unwrap();
        inventory()
            .reference("native-delivery:pending", &[ids[3].clone()])
            .unwrap();
        inventory()
            .reference("current-report:exact", &[ids[4].clone()])
            .unwrap();
        inventory()
            .dependencies(&ids[4], &[ids[5].clone()])
            .unwrap();
        inventory()
            .dependencies(&ids[6], &[ids[7].clone()])
            .unwrap();
        let reader = inventory().consume(&ids[6]).unwrap();
        if all_rooted {
            inventory()
                .reference("consumer:required-originals", &ids)
                .unwrap();
        }
        let before = state();
        assert_eq!(before["outputs"].as_array().unwrap().len(), 512);
        let invoke = |phase: &str| {
            let extra = if operation == "bootstrap" {
                vec!["--bootstrap-config", bootstrap]
            } else {
                vec![]
            };
            let mut command = h.auto_command_at(&invoking, operation, &extra);
            let out = h._dir.capture_bound(&mut command);
            let dir = h.root.join(".chrono-harness/state");
            fs::write(dir.join(format!("capacity-{phase}.stdout")), &out.stdout).unwrap();
            fs::write(dir.join(format!("capacity-{phase}.stderr")), &out.stderr).unwrap();
            fs::write(
                dir.join(format!("capacity-{phase}.status")),
                out.status.to_string(),
            )
            .unwrap();
            h.received(out)
        };
        // This collector declares every original as a required input. Its
        // exact root prevents reclamation; an unrelated activity has no effect.
        inventory()
            .reference("collector-required-inputs", &ids)
            .unwrap();
        let (code, report, error) = invoke("active");
        assert_eq!(code, 0, "{report} {error}");
        let active = state();
        for original in before["outputs"].as_array().unwrap() {
            assert!(active["outputs"].as_array().unwrap().contains(original));
        }
        inventory()
            .reference("collector-required-inputs", &[])
            .unwrap();
        let (code, report, error) = invoke("excluded");
        assert_eq!(code, 0, "{report} {error}");
        assert_eq!(
            report["status"],
            if operation == "maintain" {
                "maintained"
            } else {
                "used"
            }
        );
        if operation == "bootstrap" {
            assert_eq!(report["managed_process"]["exit_code"], 0);
            assert_eq!(report["managed_process"]["owned_work_joined"], true);
            assert_eq!(
                report["managed_process"]["argv"],
                value!([
                    env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"),
                    "noop",
                    h.root,
                    bootstrap
                ])
            );
        }
        // Only the explicit maintenance case consumes cross-object cleanup.
        // Bootstrap admission must leave all unrelated originals intact even
        // after the collector releases them; it has no global capacity gate.
        if operation == "maintain" {
            for _ in 0..8 {
                let round = inventory().maintain().unwrap();
                assert!(round["nodes"].as_u64().unwrap() <= 256);
                if all_rooted
                    || state()["summaries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|s| ids.iter().skip(8).any(|id| s["id"] == *id))
                {
                    break;
                }
            }
            if !all_rooted {
                assert!(
                    state()["summaries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|s| ids.iter().skip(8).any(|id| s["id"] == *id))
                );
            }
        }
        let after = state();
        let retained = if all_rooted || operation == "bootstrap" {
            512
        } else {
            8
        };
        for n in 0..retained {
            assert_eq!(
                fs::read(
                    h.root
                        .join(format!(".chrono-harness/state/worktrees/history-{n}.json"))
                )
                .unwrap(),
                format!("original {n}\n").as_bytes()
            );
            let original = before["outputs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["id"] == ids[n])
                .unwrap();
            assert_eq!(
                after["outputs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|o| o["id"] == ids[n])
                    .unwrap(),
                original
            );
        }
        assert_eq!(fs::read(h.root.join("payload")).unwrap(), source_bytes);
        assert_eq!(fs::read(h.root.join(".git/index")).unwrap(), index);
        drop(reader);
        drop(live);
    });
    if let Err(failure) = result {
        eprintln!(
            "original capacity fixture retained at {}",
            h.parent.display()
        );
        std::mem::forget(h);
        std::panic::resume_unwind(failure);
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn ordinary_report_capacity_maintenance_reclaims_only_eligible_originals() {
    ordinary_report_capacity("maintain", false, |h| h.root.clone());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn ordinary_report_capacity_bootstrap_admits_without_unrelated_maintenance() {
    ordinary_report_capacity("bootstrap", false, |h| h.root.clone());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn ordinary_report_capacity_maintenance_preserves_required_originals() {
    ordinary_report_capacity("maintain", true, |h| h.root.clone());
}

#[cfg(not(verus_keep_ghost))]
fn linked_capacity_invoker(h: &Host) -> PathBuf {
    let target = h.parent.join("capacity-invoker");
    let (code, _, _) = h.invoke("feature", "capacity-invoker", &target);
    require_capacity_fixture_exit(code);
    target
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn linked_report_capacity_maintenance_reclaims_only_eligible_originals() {
    ordinary_report_capacity("maintain", false, linked_capacity_invoker);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn linked_report_capacity_maintenance_preserves_required_originals() {
    ordinary_report_capacity("maintain", true, linked_capacity_invoker);
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn inventory_row<'a>(report: &'a Value, target: &Path) -> &'a Value {
    report["worktree_inventory"]
        .as_array()
        .expect("shared drain must publish its Git inventory")
        .iter()
        .find(|row| row["path"] == value!(target))
        .expect("Git-registered worktree must be reported")
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_inventory_command_failure_is_fatal_for_maintenance_and_admission() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    native_git(&h, "inventory-failure", value!({}));
    let target = h.parent.join("inventory-failure");
    assert_eq!(h.invoke("feature", "inventory-failure", &target).0, 0);
    output(&target);
    fs::write(h.parent.join("fail-inventory"), "fail").unwrap();
    for command in ["maintain", "use"] {
        let extra = if command == "use" {
            vec![
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ]
        } else {
            vec![]
        };
        let ledger = h.ledger();
        let (code, report, error) = h.auto(command, &extra);
        assert_eq!(code, 2, "{report} {error}");
        assert!(
            report["error"]
                .as_str()
                .unwrap()
                .contains("original inventory failure")
        );
        if command == "maintain" {
            assert_eq!(report["complete_inventory"]["status"], "incomplete");
            assert!(report["complete_inventory"]["path"].is_string());
            assert!(
                report["cleanup_failures"]
                    .to_string()
                    .contains("original inventory failure")
            );
            assert_eq!(report["worktree_inventory"], value!([]));
        } else {
            assert_eq!(report["cleanup_failures"], value!([]));
            // Target admission failed before the optional status projection.
            // The original command error and its exact process result remain.
            assert!(report["worktree_status"].is_null());
            let failed = report["processes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["process"]["exit_code"] == 71)
                .unwrap();
            assert_eq!(
                failed["argv"],
                value!([
                    "--no-replace-objects",
                    "worktree",
                    "list",
                    "--porcelain",
                    "-z"
                ])
            );
            let process = chrono_harness::full::expand_process(&failed["process"]).unwrap();
            assert_eq!(process["stderr"], "original inventory failure\n");
            assert_eq!(process["owned_work_joined"], true);
        }
        assert!(report["managed_process"].is_null());
        let mut after = h.ledger();
        // Failure progress is durable state, not a mutation of work or custody.
        if command == "maintain" {
            assert_eq!(after["entries"][0]["cleanup"]["stage"], "failed");
            assert!(
                after["entries"][0]["cleanup"]["detail"]
                    .as_str()
                    .unwrap()
                    .contains("original inventory failure")
            );
            if let Some(cleanup) = ledger["entries"][0].get("cleanup") {
                after["entries"][0]["cleanup"] = cleanup.clone();
            } else {
                after["entries"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("cleanup");
            }
        }
        assert_eq!(after, ledger);
        assert!(target.join("output λ/data").exists());
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_inventory_preserves_ledgerless_dirty_locked_and_prunable_worktrees() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for fault in ["dirty", "locked", "prunable"] {
        let h = Host::new("payload");
        h.kernel_cleanup();
        let target = h.parent.join("unowned");
        git(
            &h.root,
            &[
                "worktree",
                "add",
                "-b",
                "unowned",
                target.to_str().unwrap(),
                "HEAD",
            ],
        );
        output(&target);
        fs::write(target.join("payload"), "unretained commit").unwrap();
        commit(&target);
        let head = git(&target, &["rev-parse", "HEAD"]);
        let gitfile = fs::read(target.join(".git")).unwrap();
        match fault {
            "dirty" => fs::write(target.join("payload"), "real dirty work").unwrap(),
            "locked" => {
                git(
                    &h.root,
                    &[
                        "worktree",
                        "lock",
                        "--reason",
                        "live external validation",
                        target.to_str().unwrap(),
                    ],
                );
            }
            _ => {
                fs::rename(&target, h.parent.join("preserved-missing-checkout")).unwrap();
            }
        }
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{fault}: {report} {error}");
        let row = inventory_row(&report, &target);
        assert_eq!(row["head"], head);
        assert_eq!(row["branch"], "refs/heads/unowned");
        assert_eq!(row["enrollment_match"], false);
        assert!(
            row["preserved_reason"]
                .as_str()
                .unwrap()
                .contains("no current enrollment")
        );
        assert_eq!(row["locked"], fault == "locked");
        assert_eq!(row["prunable"], fault == "prunable");
        assert_eq!(
            row["status"],
            if fault == "prunable" {
                "blocked"
            } else {
                "preserved"
            }
        );
        assert_eq!(h.ledger()["entries"], value!([]));
        assert_eq!(git(&h.root, &["rev-parse", "refs/heads/unowned"]), head);
        let retained = if fault == "prunable" {
            h.parent.join("preserved-missing-checkout")
        } else {
            target.clone()
        };
        assert_eq!(fs::read(retained.join(".git")).unwrap(), gitfile);
        assert!(retained.join("output λ/data").exists());
        if fault == "dirty" {
            assert_eq!(
                fs::read_to_string(retained.join("payload")).unwrap(),
                "real dirty work"
            );
        }
        let main = inventory_row(&report, &h.root);
        assert_eq!(main["status"], "preserved");
        assert_eq!(main["preserved_reason"], "coordinator worktree");
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_inventory_requires_current_attachment_but_accepts_advanced_head() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for fault in ["advanced", "gitfile", "metadata", "unreadable"] {
        let h = Host::new("payload");
        h.kernel_cleanup();
        let target = h.parent.join("enrolled");
        assert_eq!(h.invoke("feature", "enrolled", &target).0, 0);
        output(&target);
        let metadata = PathBuf::from(git(&target, &["rev-parse", "--absolute-git-dir"]));
        match fault {
            "advanced" => {
                fs::write(target.join("payload"), "new committed source").unwrap();
                commit(&target);
            }
            "gitfile" => {
                fs::rename(target.join(".git"), h.parent.join("original-gitfile")).unwrap();
                fs::write(
                    target.join(".git"),
                    fs::read(h.parent.join("original-gitfile")).unwrap(),
                )
                .unwrap();
            }
            "metadata" => {
                let original = metadata.with_extension("original");
                fs::rename(&metadata, &original).unwrap();
                retain_fixture_state(&original, &metadata).unwrap();
            }
            _ => {
                fs::write(target.join(".git"), "unreadable attachment").unwrap();
            }
        }
        let ledger = h.ledger();
        let (code, report, error) = h.auto("maintain", &[]);
        let row = inventory_row(&report, &target);
        if fault == "advanced" {
            assert_eq!(code, 0, "{report} {error}");
            assert_eq!(row["status"], "enrolled");
            assert_eq!(row["enrollment_match"], true);
            assert_eq!(row["head"], git(&target, &["rev-parse", "HEAD"]));
            assert!(!target.join("output λ").exists());
            assert_eq!(h.ledger()["entries"][0]["status"], "active");
            assert!(h.ledger()["entries"][0]["terminal"].is_null());
        } else {
            assert_ne!(code, 0, "{fault}: {report} {error}");
            assert_eq!(row["status"], "blocked");
            assert_eq!(row["enrollment_match"], false);
            assert!(
                row["preserved_reason"]
                    .as_str()
                    .unwrap()
                    .contains("attachment")
            );
            assert!(target.join("output λ/data").exists());
            assert_eq!(
                h.ledger()["entries"][0]["attachment"],
                ledger["entries"][0]["attachment"]
            );
        }
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn independent_birth_has_no_dependency_on_another_worktrees_maintenance_original() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    let first = h.parent.join("drained-first");
    let next = h.parent.join("consumer-next");
    assert_eq!(h.invoke("feature", "drained-first", &first).0, 0);
    output(&first);
    let (code, maintenance, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintenance} {error}");
    let input = &maintenance["drain"][0]["input"];
    let original_path = h.root.join(input["path"].as_str().unwrap());
    let original = fs::read(&original_path).unwrap();
    assert_eq!(sha256(&original), input["sha256"]);
    let (code, birth, error) = h.invoke("feature", "consumer-next", &next);
    assert_eq!(code, 0, "{birth} {error}");
    assert!(birth["drain"].as_array().unwrap().is_empty());
    let args = [
        "--path",
        next.to_str().unwrap(),
        "--operation",
        "use.consumer",
    ];
    for missing in [false, true] {
        if missing {
            fs::remove_file(&original_path).unwrap();
        } else {
            fs::write(&original_path, b"changed unrelated original").unwrap();
        }
        let (code, used, error) = h.auto("use", &args);
        assert_eq!(code, 0, "{used} {error}");
        assert_eq!(used["managed_process"]["stdout"], "ordinary-work");
        assert!(next.join("payload").exists());
        fs::write(&original_path, &original).unwrap();
    }
    assert_eq!(fs::read(&original_path).unwrap(), original);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_birth_enrollment_and_active_retained_checkout_survives_start() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("some language/input.odd");
    h.automatic("evidence-retain");
    let first = h.parent.join("active");
    let (code, r, e) = h.invoke("feature", "active", &first);
    assert_eq!(code, 0, "{r} {e}");
    output(&first);
    let ledger = h.ledger();
    let entry = ledger["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["path"] == value!(first))
        .unwrap();
    assert_eq!(entry["status"], "active");
    assert_eq!(entry["enrollment"]["kind"], "birth");
    assert_eq!(h.invoke("feature", "next", &h.parent.join("next")).0, 0);
    assert!(first.join("output λ/data").exists());
}

#[cfg(not(verus_keep_ghost))]
fn attention_row<'a>(report: &'a Value, path: &Path) -> &'a Value {
    report["worktree_status"]["worktrees"]
        .as_array()
        .expect("persistent lifecycle status must be observable")
        .iter()
        .find(|row| row["path"] == value!(path))
        .expect("status must include the addressed worktree")
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_attention_persists_across_commands_and_stops_after_verified_removal() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    native_consumer(&h, "finished");
    let target = h.parent.join("attention active");
    let (code, birth, stderr) = h.invoke("integration", "attention", &target);
    assert_eq!(code, 0, "{birth} {stderr}");
    assert_eq!(attention_row(&birth, &target)["work_state"], "active");
    assert!(stderr.contains("WORKTREE_ATTENTION"), "{stderr}");

    let (code, status, stderr) = h.auto("status", &[]);
    assert_eq!(code, 0, "{status} {stderr}");
    assert_eq!(attention_row(&status, &target)["attention_required"], true);
    assert_eq!(attention_row(&status, &h.root)["attention_required"], false);
    let (code, used, stderr) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {stderr}");
    assert!(stderr.contains("WORKTREE_ATTENTION"), "{stderr}");
    let (code, finished, stderr) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{finished} {stderr}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    assert!(!target.exists());
    let (code, status, stderr) = h.auto("status", &[]);
    assert_eq!(code, 0, "{status} {stderr}");
    let row = attention_row(&status, &target);
    assert_eq!(row["cleanup_state"], "disposed");
    assert_eq!(row["work_state"], "terminal");
    assert_eq!(row["attention_required"], false);
    assert!(!stderr.contains("WORKTREE_ATTENTION"), "{stderr}");
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_attention_linked_query_publishes_its_original_at_the_coordinator() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    let target = h.parent.join("linked status");
    assert_eq!(h.invoke("feature", "linked-status", &target).0, 0);
    let mut command = Command::new(Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-cli")));
    command
        .current_dir(&target)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args([
            "status",
            "--host-root",
            target.to_str().unwrap(),
            "--config",
            POLICY,
        ]);
    // received checks exact stdout against the original at source_root/report_path.
    let output = h.native_output(&mut command);
    let (code, report, stderr) = h.received(output);
    assert_eq!(code, 0, "{report} {stderr}");
    assert_eq!(report["source_root"], value!(h.root));
    assert_eq!(attention_row(&report, &target)["attention_required"], true);
    assert!(stderr.contains("WORKTREE_ATTENTION"), "{stderr}");
    assert!(
        !target
            .join(report["report_path"].as_str().unwrap())
            .exists()
    );

    let policy_path = target.join(AUTO_POLICY);
    let original_policy = fs::read(&policy_path).unwrap();
    fs::write(&policy_path, b"uncommitted policy input").unwrap();
    let output = h.native_output(&mut command);
    let (code, failed, stderr) = h.received(output);
    assert_eq!(code, 2, "{failed} {stderr}");
    assert_eq!(failed["source_root"], value!(target));
    assert_eq!(failed["status"], "failed");
    assert!(
        failed["error"]
            .as_str()
            .unwrap()
            .contains("policy differs from source commit")
    );
    assert_eq!(fs::read(&policy_path).unwrap(), b"uncommitted policy input");
    fs::write(policy_path, original_policy).unwrap();
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_attention_preserves_unregistered_dirty_and_corrupt_worktrees() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    let healthy = h.parent.join("healthy");
    let corrupt = h.parent.join("corrupt");
    assert_eq!(h.invoke("feature", "healthy", &healthy).0, 0);
    assert_eq!(h.invoke("feature", "corrupt", &corrupt).0, 0);
    let unknown = h.parent.join("unknown");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "--detach",
            unknown.to_str().unwrap(),
            "HEAD",
        ],
    );
    fs::write(unknown.join("unique-work"), "keep this source").unwrap();
    fs::write(h.ledger_path(&corrupt), b"broken original ledger").unwrap();
    let (code, status, stderr) = h.auto("status", &[]);
    assert_eq!(code, 0, "{status} {stderr}");
    assert_eq!(attention_row(&status, &healthy)["work_state"], "active");
    assert_eq!(attention_row(&status, &corrupt)["work_state"], "unknown");
    assert_eq!(
        attention_row(&status, &unknown)["work_state"],
        "unregistered"
    );
    assert_eq!(
        fs::read(unknown.join("unique-work")).unwrap(),
        b"keep this source"
    );
    assert_eq!(
        fs::read(h.ledger_path(&corrupt)).unwrap(),
        b"broken original ledger"
    );
    assert!(
        !h.ledger_path(&unknown).exists(),
        "observation must not import work"
    );
    let marker = h.record_directory(&healthy).join("owner.json");
    let original_owner = fs::read(&marker).unwrap();
    let mut foreign = json(&original_owner).unwrap();
    foreign["common"] = value!(h.parent.join("foreign-owner"));
    let foreign = serde_json::to_vec(&foreign).unwrap();
    fs::write(&marker, &foreign).unwrap();
    let (code, observed, error) = h.auto("status", &[]);
    assert_eq!(code, 0, "{observed} {error}");
    assert_eq!(attention_row(&observed, &healthy)["work_state"], "unknown");
    assert!(attention_row(&observed, &healthy)["error"].is_string());
    assert_eq!(fs::read(&marker).unwrap(), foreign);
    fs::write(marker, original_owner).unwrap();
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_attention_keeps_retained_terminal_work_visible_and_scopes_ordinary_commands() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    let target = h.parent.join("retained");
    let other = h.parent.join("other");
    assert_eq!(h.invoke("feature", "retained", &target).0, 0);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(
        target.join(".chrono-harness/state/evidence"),
        "required evidence",
    )
    .unwrap();
    let (code, report, stderr) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {stderr}");
    // 原业务交接与实际清理分别核原结果。
    let (report_cleanup_code, report_cleanup, report_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        report_cleanup_code, 0,
        "{report_cleanup} {report_cleanup_error}"
    );
    assert!(target.exists());
    let row = attention_row(&report_cleanup, &target);
    assert_eq!(row["work_state"], "terminal");
    assert_eq!(row["cleanup_state"], "retained");
    assert_eq!(row["attention_required"], true);
    assert!(stderr.contains("WORKTREE_ATTENTION"), "{stderr}");
    let (code, report, stderr) = h.invoke("feature", "other", &other);
    assert_eq!(code, 0, "{report} {stderr}");
    assert_eq!(
        report["worktree_status"]["worktrees"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(attention_row(&report, &other)["work_state"], "active");
    let (code, report, stderr) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {stderr}");
    assert_eq!(
        report["worktree_status"]["worktrees"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(attention_row(&report, &target)["attention_required"], true);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_finish_absent_evidence_does_not_retain_the_whole_checkout() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("no-evidence");
    assert_eq!(h.invoke("integration", "no-evidence", &target).0, 0);
    output(&target);
    assert!(!target.join(".chrono-harness/state").exists());
    let (code, report, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (report_cleanup_code, report_cleanup, report_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        report_cleanup_code, 0,
        "{report_cleanup} {report_cleanup_error}"
    );
    assert!(
        !target.exists(),
        "absent evidence cannot keep an otherwise disposable checkout"
    );
    assert_eq!(
        drain_report(&report_cleanup["drain"][0])["worktree_removal"],
        "verified-absent"
    );
    assert_eq!(h.ledger()["entries"][0]["status"], "disposed");
    assert_eq!(h.ledger()["entries"][0]["cleanup"]["stage"], "disposed");
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains(target.to_str().unwrap()));
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_maintenance_resumes_retained_checkout_after_evidence_release() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("released-evidence");
    assert_eq!(h.invoke("integration", "released-evidence", &target).0, 0);
    let evidence = target.join(".chrono-harness/state");
    fs::create_dir_all(&evidence).unwrap();
    fs::write(
        evidence.join("consumer-original"),
        "required until consumed",
    )
    .unwrap();
    output(&target);
    let (code, report, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (report_cleanup_code, report_cleanup, report_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        report_cleanup_code, 0,
        "{report_cleanup} {report_cleanup_error}"
    );
    assert!(target.exists());
    assert_eq!(h.ledger()["entries"][0]["status"], "retained");
    let progress = h.ledger()["entries"][0]["cleanup"].clone();
    assert_eq!(progress["stage"], "retained");
    assert!(progress["detail"].as_str().unwrap().contains("evidence"));
    assert_eq!(
        progress["report_path"],
        report_cleanup["drain"][0]["receipt"]["path"]
    );
    let original = entry_receipts(&h, &h.ledger()["entries"][0]);
    // The fixture's sole evidence consumer finishes and releases its exact files.
    fs::remove_file(evidence.join("consumer-original")).unwrap();
    fs::remove_dir(&evidence).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        !target.exists(),
        "retained checkout must remain discoverable for later cleanup"
    );
    assert_eq!(
        drain_report(&report["drain"][0])["worktree_removal"],
        "verified-absent"
    );
    assert_eq!(h.ledger()["entries"][0]["status"], "disposed");
    for (path, bytes) in original {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn retained_checkout_keeps_its_cleanup_original_until_the_responsibility_is_released() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::Inventory;
    let h = Host::new("payload");
    h.kernel_cleanup();
    adopt_lifecycle_retention(&h);
    // Retention itself owns metadata under state/. The fixture consumer owns
    // this separate evidence path and can release all of it without deleting
    // another producer's metadata or weakening the original-receipt assertion.
    // Target-local retention metadata has no retained caller after cleanup; the
    // ordinary disposal owner must verify its release before removing the tree.
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    for artifact in policy["artifacts"].as_array_mut().unwrap() {
        if artifact["path"] == ".chrono-harness/state/" {
            artifact["disposition"] = value!("dispose");
        }
    }
    policy["artifacts"].as_array_mut().unwrap().push(value!({
        "path":"consumer-evidence/", "disposition":"evidence-retain"
    }));
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["artifacts"].as_array_mut().unwrap().push(value!({
        "path":"consumer-evidence/", "owner":"host", "kind":"evidence", "tracked":false
    }));
    fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("retained-original");
    let (code, created, error) = h.invoke("integration", "retained-original", &target);
    assert_eq!(code, 0, "{created} {error}");
    let evidence = target.join("consumer-evidence");
    fs::create_dir_all(&evidence).unwrap();
    fs::write(
        evidence.join("needed"),
        "consumer has not released this original",
    )
    .unwrap();
    output(&target);
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    let entry = h.ledger()["entries"][0].clone();
    assert_eq!(entry["status"], "retained");
    assert_eq!(entry["cleanup"]["stage"], "retained");
    let receipt = &entry["attempts"][0]["receipt"];
    let path = h.root.join(receipt["path"].as_str().unwrap());
    let original = fs::read(&path).unwrap();
    assert_eq!(receipt["sha256"], sha256(&original));
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    let maintenance = inventory.maintain().unwrap();
    assert!(
        path.is_file(),
        "an unresolved checkout still consumes its original cleanup result: {maintenance}"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(target.join("payload").is_file());
    assert_eq!(h.ledger()["entries"][0], entry);
    // The sole fixture consumer now releases the evidence; the next entry must
    // be able to read the preserved result and finish the physical cleanup.
    fs::remove_file(evidence.join("needed")).unwrap();
    fs::remove_dir(&evidence).unwrap();
    let (code, resumed, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{resumed} {error}");
    assert!(!target.exists(), "{resumed} {error}");
    assert_eq!(h.ledger()["entries"][0]["status"], "disposed");
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains(target.to_str().unwrap()));
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn creator_agent_and_pending_reminders_follow_the_original_creator() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    h.policy(|policy| {
        policy["agent_identity_env"] = value!("CHRONO_TEST_AGENT_ID");
        policy["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .push(value!("CHRONO_TEST_AGENT_ID"));
    });
    let invoke = |operation: &str, extra: &[&str], agent: Option<&str>| {
        let mut command = h.auto_command(operation, extra);
        if let Some(agent) = agent {
            command.env("CHRONO_TEST_AGENT_ID", agent);
        }
        let output = h._dir.capture_bound(&mut command);
        h.received(output)
    };
    let a = h.parent.join("agent-a-work");
    let b = h.parent.join("agent-b-work");
    let unknown = h.parent.join("unknown-agent-work");
    for (name, path, agent) in [
        ("agent-a", &a, Some("agent-A")),
        ("agent-b", &b, Some("agent-B")),
        ("unknown-agent", &unknown, None),
    ] {
        let (code, created, error) = invoke(
            "start",
            &[
                "--kind",
                "integration",
                "--name",
                name,
                "--path",
                path.to_str().unwrap(),
            ],
            agent,
        );
        assert_eq!(code, 0, "{created} {error}");
        if let Some(agent) = agent {
            assert_eq!(created["agent"]["identity"]["id"], agent);
            let intent = json(
                &fs::read(
                    h.root
                        .join(created["recovery_intent"]["path"].as_str().unwrap()),
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(intent["creator_agent"]["id"], agent);
        } else {
            assert_eq!(created["agent"]["status"], "unknown");
        }
    }
    let a_entry = h.ledger()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == value!(a))
        .unwrap()
        .clone();
    assert_eq!(a_entry["creator_agent"]["id"], "agent-A");
    let imported = h.parent.join("imported-agent-work");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "-qb",
            "external-agent-work",
            imported.to_str().unwrap(),
            "dev",
        ],
    );
    let (code, imported_report, error) = invoke(
        "import",
        &["--path", imported.to_str().unwrap()],
        Some("agent-B"),
    );
    assert_eq!(code, 0, "{imported_report} {error}");
    let imported_entry = h.ledger()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == value!(imported))
        .unwrap()
        .clone();
    assert!(imported_entry["creator_agent"].is_null());
    assert_eq!(imported_entry["custodian_agent"]["id"], "agent-B");
    let (code, maintained, error) = invoke("maintain", &[], Some("agent-A"));
    assert_eq!(code, 0, "{maintained} {error}");
    let reminders: Vec<_> = maintained["worktree_status"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["agent_attention_required"] == true)
        .collect();
    assert_eq!(reminders.len(), 1, "{maintained}");
    assert_eq!(reminders[0]["path"], value!(a));
    assert_eq!(reminders[0]["work_state"], "active");
    assert!(error.contains(a.to_str().unwrap()), "{error}");
    assert!(!error.contains(b.to_str().unwrap()), "{error}");
    assert!(
        maintained["worktree_status"]["unknown_creator_records"]
            .as_u64()
            .unwrap()
            >= 1
    );
    let unregistered = h.parent.join("agent-unregistered");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "--detach",
            unregistered.to_str().unwrap(),
            "dev",
        ],
    );
    // A related operation on B must also remind A about its own outstanding A.
    // Git addresses without any enrollment keep the observation incomplete.
    let (code, scoped, error) = invoke("status", &["--path", b.to_str().unwrap()], Some("agent-A"));
    assert_eq!(code, 0, "{scoped} {error}");
    assert_eq!(attention_row(&scoped, &a)["agent_attention_required"], true);
    assert_eq!(
        attention_row(&scoped, &b)["agent_attention_required"],
        false
    );
    assert!(error.contains(a.to_str().unwrap()), "{error}");
    assert!(!error.contains(b.to_str().unwrap()), "{error}");
    assert_eq!(
        scoped["worktree_status"]["agent_observation_complete"],
        false
    );
    assert_eq!(
        scoped["worktree_status"]["unknown_creator_records"]
            .as_u64()
            .unwrap(),
        maintained["worktree_status"]["unknown_creator_records"]
            .as_u64()
            .unwrap()
            + 1
    );
    let (code, unknown_scope, error) = invoke("status", &["--path", b.to_str().unwrap()], None);
    assert_eq!(code, 0, "{unknown_scope} {error}");
    assert_eq!(
        unknown_scope["worktree_status"]["agent"]["status"],
        "unknown"
    );
    assert!(error.contains("W_WORKTREE_AGENT_SCOPE"), "{error}");
    let (code, finished, error) =
        invoke("finish", &["--path", a.to_str().unwrap()], Some("agent-B"));
    assert_eq!(code, 0, "{finished} {error}");
    assert!(a.exists(), "finish hands off without physical cleanup");
    let a_entry = h.ledger()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == value!(a))
        .unwrap()
        .clone();
    assert_eq!(a_entry["creator_agent"]["id"], "agent-A");
    let (code, maintained, error) = invoke("maintain", &[], Some("agent-A"));
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!a.exists());
    assert!(
        maintained["worktree_status"]["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["agent_attention_required"] != true)
    );
    assert!(b.join("payload").is_file());
    assert!(unknown.join("payload").is_file());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn terminal_history_rotation_preserves_current_work_and_releases_only_disposed_records() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":2,"max_bytes":1048576});
    fs::write(&policy_path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let active = h.parent.join("still-active");
    let target = h.parent.join("rotating-history");
    let (code, created, error) = h.invoke("integration", "still-active", &active);
    assert_eq!(code, 0, "{created} {error}");
    let active_entry = h.ledger()["entries"][0].clone();
    let (code, created, error) = h.invoke("integration", "rotating-history", &target);
    assert_eq!(code, 0, "{created} {error}");
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    assert!(!target.exists());
    let disposed = h.ledger()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == value!(target))
        .unwrap()
        .clone();
    assert_eq!(disposed["status"], "disposed");
    assert!(disposed["disposed_at"].as_u64().is_some());
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let remaining = h.ledger();
    assert_eq!(remaining["entries"].as_array().unwrap().len(), 1);
    assert_eq!(remaining["entries"][0]["path"], active_entry["path"]);
    assert_eq!(
        remaining["entries"][0]["attachment"],
        active_entry["attachment"]
    );
    assert_eq!(remaining["entries"][0]["status"], "active");
    assert!(remaining["entries"][0]["terminal"].is_null());
    assert!(active.join("payload").is_file());
    assert!(!target.exists());
    assert!(
        !h.root
            .join(disposed["ownership"]["path"].as_str().unwrap())
            .exists(),
        "retired attachment capabilities must not outlive their final responsibility"
    );
    assert!(
        !h.record_directory(&target).exists(),
        "an empty retired address must release its metadata and directory"
    );
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains(target.to_str().unwrap()));
    let (code, again, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{again} {error}");
    assert_eq!(h.ledger()["entries"].as_array().unwrap().len(), 1);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn retired_address_preserves_directory_consumers_and_unknown_members() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::ownership::DirectoryLease;
    let h = Host::new("payload");
    h.kernel_cleanup();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("retirement-boundaries");
    let (code, created, error) = h.invoke("integration", "retirement-boundaries", &target);
    assert_eq!(code, 0, "{created} {error}");
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    let directory = h.record_directory(&target);
    let before = fs::read(directory.join("ledger.json")).unwrap();
    let reader = DirectoryLease::acquire(&directory, false, None)
        .unwrap()
        .unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 2, "{maintained} {error}");
    assert_eq!(maintained["complete_inventory"]["status"], "incomplete");
    assert_eq!(fs::read(directory.join("ledger.json")).unwrap(), before);
    drop(reader);
    let foreign = directory.join("unknown-work");
    fs::write(&foreign, b"retain this independent work").unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 2, "{maintained} {error}");
    assert_eq!(fs::read(&foreign).unwrap(), b"retain this independent work");
    assert!(directory.join("owner.json").is_file());
    fs::remove_file(foreign).unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!directory.exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn interrupted_address_retirement_preserves_changed_members_then_resumes_original_identity() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("interrupted-retirement");
    let (code, created, error) = h.invoke("integration", "interrupted-retirement", &target);
    assert_eq!(code, 0, "{created} {error}");
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    let directory = h.record_directory(&target);
    fs::write(directory.join("fixture-consumer"), b"owned by this fixture").unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 2, "{maintained} {error}");
    assert!(h.ledger()["entries"].as_array().unwrap().is_empty());
    fs::remove_file(directory.join("fixture-consumer")).unwrap();
    let members: Vec<_> = ["ledger.json", "owner.json", "metadata.lease"]
        .iter()
        .map(|name| {
            let path = directory.join(name);
            let bytes = fs::read(&path).unwrap();
            value!({"name":name,"identity":chrono_harness::ownership::identity(&path).unwrap(),
            "sha256":sha256(&bytes),"length":bytes.len()})
        })
        .collect();
    let declaration = value!({"schema":"chrono-worktree-address-retirement/v1",
        "coordinator_root":h.root,"common":h.root.join(".git"),"path":target,
        "directory_identity":chrono_harness::artifact_disposal::identity(&directory).unwrap(),
        "admission_id":members[2]["identity"],"members":members});
    fs::write(
        directory.join("retirement.json"),
        serde_json::to_vec(&declaration).unwrap(),
    )
    .unwrap();
    // A real unlink happened before interruption. Recovery must accept this
    // partial effect without accepting another member's changed bytes.
    fs::remove_file(directory.join("ledger.json")).unwrap();
    let owner = directory.join("owner.json");
    let original = fs::read(&owner).unwrap();
    fs::write(&owner, [original.as_slice(), b" "].concat()).unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 2, "{maintained} {error}");
    assert!(directory.join("retirement.json").is_file());
    assert!(directory.join("metadata.lease").is_file());
    assert_eq!(
        fs::read(&owner).unwrap(),
        [original.as_slice(), b" "].concat()
    );
    fs::write(&owner, original).unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!directory.exists());
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn retired_capability_failure_keeps_the_creators_original_responsibility_visible() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    h.policy(|policy| {
        policy["agent_identity_env"] = value!("CHRONO_TEST_AGENT_ID");
        policy["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .push(value!("CHRONO_TEST_AGENT_ID"));
    });
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let invoke = |operation: &str, extra: &[&str]| {
        let mut command = h.auto_command(operation, extra);
        command.env("CHRONO_TEST_AGENT_ID", "retirement-agent-A");
        h.received(h._dir.capture_bound(&mut command))
    };
    let target = h.parent.join("retired-capability");
    let (code, created, error) = invoke(
        "start",
        &[
            "--kind",
            "integration",
            "--name",
            "retired-capability",
            "--path",
            target.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{created} {error}");
    let lease = h.root.join(
        h.ledger()["entries"][0]["ownership"]["path"]
            .as_str()
            .unwrap(),
    );
    let (code, finished, error) = invoke("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    // Finish only hands off; reach actual checkout disposal before injecting
    // the retired-capability fault, without retiring the capability itself.
    let (code, disposed, error) = invoke("maintain", &[]);
    assert_eq!(code, 0, "{disposed} {error}");
    assert!(!target.exists());
    let identity = chrono_harness::ownership::identity(&lease).unwrap();
    fs::write(&lease, b"unknown capability bytes").unwrap();
    let (code, maintained, error) = invoke("maintain", &[]);
    assert_eq!(code, 2, "{maintained} {error}");
    let (code, status, error) = invoke("status", &["--path", h.root.to_str().unwrap()]);
    assert_eq!(code, 0, "{status} {error}");
    let pending = status["worktree_status"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["record_kind"] == "retired-lease")
        .unwrap();
    assert_eq!(pending["creator_agent"]["id"], "retirement-agent-A");
    assert_eq!(pending["branch"], "integration/retired-capability");
    assert_eq!(pending["agent_attention_required"], true);
    assert_eq!(pending["next_action"], "maintain-retired-lease");
    assert!(pending["error"].as_str().unwrap().contains("unknown bytes"));
    assert!(error.contains(target.to_str().unwrap()), "{error}");
    assert_eq!(fs::read(&lease).unwrap(), b"unknown capability bytes");
    assert_eq!(
        chrono_harness::ownership::identity(&lease).unwrap(),
        identity
    );
    fs::write(&lease, b"").unwrap();
    let (code, maintained, error) = invoke("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(!lease.exists());
    assert!(!h.record_directory(&target).exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn terminal_history_count_and_byte_limits_release_verified_generations_before_the_ttl() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let policy_path = h.root.join(AUTO_POLICY);
    let set_history = |bytes| {
        let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
        policy["terminal_history"] =
            value!({"keep_seconds":86400,"max_entries":1,"max_bytes":bytes});
        fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
    };
    set_history(1048576);
    let target = h.parent.join("reused-history-address");
    for name in ["history-first", "history-second"] {
        let (code, created, error) = h.invoke("integration", name, &target);
        assert_eq!(code, 0, "{created} {error}");
        let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
        assert_eq!(code, 0, "{finished} {error}");
        // 原业务交接与实际清理分别核原结果。
        let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) =
            h.auto("maintain", &[]);
        assert_eq!(
            finished_cleanup_code, 0,
            "{finished_cleanup} {finished_cleanup_error}"
        );
        assert!(!target.exists());
    }
    assert_eq!(h.ledger()["entries"].as_array().unwrap().len(), 2);
    let newest = h.ledger()["entries"][1].clone();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert_eq!(h.ledger()["entries"], value!([newest]));
    set_history(1);
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(h.ledger()["entries"].as_array().unwrap().is_empty());
    assert!(!target.exists());
    assert!(git(&h.root, &["show-ref"]).contains("refs/heads/integration/history-first"));
    assert!(git(&h.root, &["show-ref"]).contains("refs/heads/integration/history-second"));
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_originals_follow_enrollment_and_report_consumers_after_history_rotation() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::Inventory;
    let h = Host::new("payload");
    h.kernel_cleanup();
    adopt_lifecycle_retention(&h);
    let retention_path = h
        .root
        .join(chrono_harness::retained_artifacts::retention::POLICY_PATH);
    let mut retention = json(&fs::read(&retention_path).unwrap()).unwrap();
    retention["producers"]["lifecycle-original"] = value!([
        ".chrono-harness/state/worktrees/",
        ".chrono-harness/state/automatic-cleanup/"
    ]);
    retention["max_related_outputs"] = value!(512);
    fs::write(&retention_path, serde_json::to_vec(&retention).unwrap()).unwrap();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    for artifact in policy["artifacts"].as_array_mut().unwrap() {
        if artifact["path"] == ".chrono-harness/state/" {
            artifact["disposition"] = value!("dispose");
        }
    }
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("finite-originals");
    let (code, created, error) = h.invoke("integration", "finite-originals", &target);
    assert_eq!(code, 0, "{created} {error}");
    let birth = h.ledger()["entries"][0]["enrollment"]["sealed_receipt"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    assert!(
        Inventory::adopted(&h.root)
            .unwrap()
            .unwrap()
            .enrolled(&birth)
            .unwrap(),
        "the automatic producer must enroll its own original, not leave an unbounded file"
    );
    inventory
        .reference_paths("fixture-birth-consumer", &[birth.clone()])
        .unwrap();
    inventory.maintain().unwrap();
    assert!(
        h.root.join(&birth).is_file(),
        "the fixture's actual birth reader still consumes the original"
    );
    inventory
        .reference_paths("fixture-birth-consumer", &[])
        .unwrap();
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    // The real consumer must be registered before a zero-retention round can
    // release the unconsumed finish report and its terminal dependency.
    let finished_path = finished["report_path"].as_str().unwrap();
    inventory
        .reference_paths("fixture-finish-consumer", &[finished_path.into()])
        .unwrap();
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    assert!(!target.exists());
    let terminal = h.ledger()["entries"][0]["terminal"]["receipt"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(h.ledger()["entries"].as_array().unwrap().is_empty());
    inventory.maintain().unwrap();
    assert!(
        !h.root.join(&birth).exists(),
        "unconsumed birth must leave with history"
    );
    assert!(
        h.root.join(&terminal).is_file(),
        "finish report still consumes terminal input"
    );
    inventory
        .reference_paths("fixture-finish-consumer", &[])
        .unwrap();
    maintain_original_until_absent(&inventory, &h.root.join(&terminal));
    assert!(
        !h.root.join(&terminal).exists(),
        "released terminal original must be reclaimed"
    );
    assert!(git(&h.root, &["show-ref"]).contains("refs/heads/integration/finite-originals"));
}

#[cfg(not(verus_keep_ghost))]
fn maintain_original_until_absent(
    inventory: &chrono_harness::retained_artifacts::retention::Inventory,
    path: &Path,
) {
    let mut rounds = Vec::new();
    for _ in 0..8 {
        // 此处明确等待跨对象原件回收；普通入口只处理其本轮局部范围。
        let round = inventory.maintain_explicit().unwrap();
        // 只核本用例的原件；无关对象的保留失败仍留在每轮原始诊断中。
        rounds.push(round);
        if !path.exists() {
            return;
        }
    }
    panic!(
        "bounded original disposal did not finish: {} {rounds:?}",
        path.display()
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn managed_result_handoff_preserves_partial_reports_then_releases_the_interim_reference() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    adopt_lifecycle_retention(&h);
    let policy_path = h.root.join(POLICY_PATH);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["producers"]["lifecycle-original"] = value!([
        ".chrono-harness/state/worktrees/",
        ".chrono-harness/state/automatic-cleanup/"
    ]);
    policy["max_related_outputs"] = value!(512);
    fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    let cleanup_path = h.root.join(AUTO_POLICY);
    let mut cleanup = json(&fs::read(&cleanup_path).unwrap()).unwrap();
    cleanup["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(cleanup_path, serde_json::to_vec(&cleanup).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("pending-result-handoff");
    let (code, created, error) = h.invoke("integration", "pending-result-handoff", &target);
    assert_eq!(code, 0, "{created} {error}");
    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {error}");
    let receipt = used["managed_use_receipt"].clone();
    let originals = h
        .root
        .join(used["managed_process_original"]["path"].as_str().unwrap());
    assert_eq!(used["managed_process_original"]["joined"], true);
    let result = json(&fs::read(originals.join("result.json")).unwrap()).unwrap();
    assert_eq!(result["owned_work_joined"], true);
    assert_eq!(result["exit_code"], 0);
    assert_eq!(result["cwd"], value!(target));
    assert!(result.get("stdout_bytes").is_none());
    assert_eq!(
        fs::read(originals.join("stdout")).unwrap(),
        b"ordinary-work"
    );
    assert_eq!(
        result["stdout_original"]["sha256"],
        sha256(b"ordinary-work")
    );
    assert_eq!(
        retention_state::snapshot(&h.root)["roots"].get(format!(
            "recovery:{}",
            used["managed_process_original"]["path"].as_str().unwrap()
        )),
        None,
        "joined report handoff must release its process publication recovery root"
    );
    let raw_path = h.root.join(receipt["path"].as_str().unwrap());
    let raw = fs::read(&raw_path).unwrap();
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([receipt])
    );
    let report_path = h.root.join(used["report_path"].as_str().unwrap());
    let report_bytes = fs::read(&report_path).unwrap();
    let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
    inventory.maintain().unwrap();
    assert!(originals.join("result.json").exists());
    assert_eq!(
        fs::read(&report_path).unwrap(),
        report_bytes,
        "the pending handoff still consumes its final report before the next worktree command"
    );
    inventory
        .reference_paths(
            "fixture-use-report",
            &[used["report_path"].as_str().unwrap().into()],
        )
        .unwrap();
    // Fault injection at the final report boundary: a partial JSON original
    // cannot discharge the pending handoff or establish a command outcome.
    fs::write(&report_path, b"{\"status\":").unwrap();
    let (code, observed, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{observed} {error}");
    assert_eq!(fs::read(&raw_path).unwrap(), raw);
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([receipt])
    );
    let mut changed = json(&report_bytes).unwrap();
    changed["untrusted_edit"] = value!(true);
    fs::write(&report_path, serde_json::to_vec(&changed).unwrap()).unwrap();
    let (code, observed, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{observed} {error}");
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([receipt]),
        "matching JSON fields do not replace the sealed producer bytes"
    );
    let (code, finished, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{finished} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    assert!(!target.exists());
    let (_code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(
        h.ledger()["entries"].as_array().unwrap().len(),
        1,
        "an unresolved report handoff must survive disposed-history rotation: {maintained} {error}"
    );
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([receipt])
    );
    let row = maintained["worktree_status"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target))
        .unwrap();
    assert_eq!(row["work_state"], "terminal");
    assert_eq!(row["report_handoff_pending"], true);
    assert_eq!(row["attention_required"], true);
    assert_eq!(row["next_action"], "recover-result-handoff");
    fs::write(&report_path, &report_bytes).unwrap();
    let (code, observed, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{observed} {error}");
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([receipt]),
        "observation must leave result recovery to its lifecycle owner"
    );
    assert_eq!(
        attention_row(&observed, &target)["report_handoff_pending"],
        true
    );
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(
        h.ledger()["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| { entry["enrollment"]["completed_uses"].is_null() })
    );
    inventory.maintain().unwrap();
    assert_eq!(
        fs::read(&raw_path).unwrap(),
        raw,
        "report's actual consumer still needs the result"
    );
    inventory
        .reference_paths("fixture-use-report", &[])
        .unwrap();
    maintain_original_until_absent(&inventory, &raw_path);
    maintain_original_until_absent(&inventory, &originals);
    assert!(
        !raw_path.exists(),
        "successful report handoff must not create permanent roots"
    );
    assert!(!target.exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn ordinary_use_hands_off_sealed_results_and_keeps_unqualified_results_pending() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::retained_artifacts::retention::{Inventory, POLICY_PATH};
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    adopt_lifecycle_retention(&h);
    let policy_path = h.root.join(POLICY_PATH);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["producers"]["lifecycle-original"] = value!([
        ".chrono-harness/state/worktrees/",
        ".chrono-harness/state/automatic-cleanup/"
    ]);
    fs::write(policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("ordinary-result-handoff");
    let (code, created, error) = h.invoke("integration", "ordinary-result-handoff", &target);
    assert_eq!(code, 0, "{created} {error}");
    let mut results = Vec::new();
    for _ in 0..2 {
        let (code, used, error) = h.auto(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        );
        assert_eq!(code, 0, "{used} {error}");
        let inventory = Inventory::adopted(&h.root).unwrap().unwrap();
        inventory
            .reference_paths(
                &format!("fixture-use-report:{}", results.len()),
                &[used["report_path"].as_str().unwrap().into()],
            )
            .unwrap();
        results.push(used);
    }
    let entry = h.ledger()["entries"][0].clone();
    assert_eq!(
        entry["enrollment"]["completed_uses"],
        value!([results[1]["managed_use_receipt"]]),
        "ordinary admission must hand off its predecessor without a maintenance round"
    );
    assert_eq!(entry["status"], "active");
    assert!(entry["terminal"].is_null());
    for used in &results {
        let receipt = &used["managed_use_receipt"];
        let bytes = fs::read(h.root.join(receipt["path"].as_str().unwrap())).unwrap();
        assert_eq!(value!(sha256(&bytes)), receipt["sha256"]);
        let process = h
            .root
            .join(used["managed_process_original"]["path"].as_str().unwrap());
        assert_eq!(fs::read(process.join("stdout")).unwrap(), b"ordinary-work");
    }
    let second_report = h.root.join(results[1]["report_path"].as_str().unwrap());
    let second_bytes = fs::read(&second_report).unwrap();
    fs::write(&second_report, b"{\"status\":").unwrap();
    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([
            results[1]["managed_use_receipt"],
            used["managed_use_receipt"]
        ]),
        "an incomplete predecessor report must stay pending while independent work proceeds"
    );
    assert_eq!(fs::read(&second_report).unwrap(), b"{\"status\":");
    fs::write(&second_report, second_bytes).unwrap();
    let receipt_path = h
        .root
        .join(results[1]["managed_use_receipt"]["path"].as_str().unwrap());
    let receipt_bytes = fs::read(&receipt_path).unwrap();
    fs::write(&receipt_path, b"changed original receipt").unwrap();
    let third_receipt = used["managed_use_receipt"].clone();
    let (code, used, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert_eq!(
        used["result_handoff_error"],
        "original lifecycle receipt changed"
    );
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["completed_uses"],
        value!([
            results[1]["managed_use_receipt"],
            third_receipt,
            used["managed_use_receipt"]
        ]),
        "a failed handoff must not publish a partial in-memory ledger or block independent use"
    );
    assert_eq!(
        fs::read(&receipt_path).unwrap(),
        b"changed original receipt"
    );
    fs::write(receipt_path, receipt_bytes).unwrap();
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn terminal_history_rotation_preserves_live_consumers_and_changed_originals() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let policy_path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&policy_path).unwrap()).unwrap();
    policy["terminal_history"] = value!({"keep_seconds":0,"max_entries":1,"max_bytes":1048576});
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("protected-history");
    assert_eq!(h.invoke("integration", "protected-history", &target).0, 0);
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    let entry = h.ledger()["entries"][0].clone();
    let lease = chrono_harness::ownership::Lease::acquire(
        &h.root.join(entry["ownership"]["path"].as_str().unwrap()),
        Some(entry["ownership"]["id"].as_str().unwrap()),
        false,
        false,
        None,
    )
    .unwrap()
    .unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{maintained} {error}");
    assert_eq!(h.ledger()["entries"], value!([entry.clone()]));
    drop(lease);
    let original_path = h
        .root
        .join(entry["attempts"][0]["receipt"]["path"].as_str().unwrap());
    let original = fs::read(&original_path).unwrap();
    fs::write(&original_path, b"changed original cleanup report").unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{maintained} {error}");
    assert_eq!(h.ledger()["entries"], value!([entry]));
    fs::write(&original_path, original).unwrap();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    assert!(h.ledger()["entries"].as_array().unwrap().is_empty());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_finish_reclaims_real_checkout_and_reports_measured_outputs() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    let target = h.parent.join("finished");
    assert_eq!(h.invoke("integration", "finished", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(
        target.join(".chrono-harness/state/evidence"),
        "explicitly settled",
    )
    .unwrap();
    let (code, r, e) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{r} {e}");
    // 原业务交接与实际清理分别核原结果。
    let (r_cleanup_code, r_cleanup, r_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(r_cleanup_code, 0, "{r_cleanup} {r_cleanup_error}");
    assert!(!target.exists());
    let effect = &drain_report(&r_cleanup["drain"][0]);
    assert_eq!(effect["worktree_removal"], "verified-absent");
    assert_eq!(effect["artifact_disposals"][0]["bytes_before"], 8192);
    assert_eq!(effect["artifact_disposals"][0]["bytes_after"], 0);
    assert!(git(&h.root, &["show-ref"]).contains("refs/heads/integration/finished"));
    assert_eq!(h.auto("maintain", &[]).0, 0);
}

#[cfg(not(verus_keep_ghost))]
fn entry_receipts(h: &Host, entry: &Value) -> Vec<(PathBuf, Vec<u8>)> {
    let evidence = std::env::var("CHRONO_WORKTREE_TEST_RECEIPTS")
        .ok()
        .map(|p| PathBuf::from(p).join("original-lifecycle-state"));
    if let Some(directory) = &evidence {
        fs::create_dir_all(directory).unwrap();
        let ledger = serde_json::to_vec_pretty(&h.ledger()).unwrap();
        fs::write(
            directory.join(format!("ledger-{}.json", sha256(&ledger))),
            ledger,
        )
        .unwrap();
    }
    let mut receipts = vec![entry["terminal"]["receipt"].clone()];
    if !entry["enrollment"]["sealed_receipt"].is_null() {
        receipts.push(entry["enrollment"]["sealed_receipt"].clone());
    }
    receipts.extend(
        entry["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["receipt"].clone()),
    );
    receipts
        .iter()
        .map(|r| {
            let path = h.root.join(r["path"].as_str().unwrap());
            let bytes = fs::read(&path).unwrap();
            assert_eq!(r["sha256"], sha256(&bytes));
            if let Some(directory) = &evidence {
                fs::write(
                    directory.join(format!("receipt-{}.json", sha256(&bytes))),
                    &bytes,
                )
                .unwrap();
            }
            (path, bytes)
        })
        .collect()
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_disposed_path_reuse_targets_new_birth_and_import_generations() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("input.custom");
    h.automatic("evidence-retain");
    native_consumer(&h, "output-branch");
    let target = h.parent.join("reused");
    let args = ["--path", target.to_str().unwrap(), "--dispose-evidence"];
    let (code, r, e) = h.invoke("feature", "first-task", &target);
    assert_eq!(code, 0, "{r} {e}");
    output(&target);
    let (code, r, e) = h.auto("finish", &args);
    assert_eq!(code, 0, "{r} {e}");
    // 原业务交接与实际清理分别核原结果。
    let (r_cleanup_code, r_cleanup, r_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(r_cleanup_code, 0, "{r_cleanup} {r_cleanup_error}");
    assert!(!target.exists());
    assert_eq!(
        drain_report(&r_cleanup["drain"][0])["artifact_disposals"][0]["bytes_before"],
        8192
    );
    assert_eq!(
        drain_report(&r_cleanup["drain"][0])["artifact_disposals"][0]["bytes_after"],
        0
    );
    let first = h.ledger()["entries"][0].clone();
    assert_eq!(first["status"], "disposed");
    assert!(!first["attempts"].as_array().unwrap().is_empty());
    let mut originals = entry_receipts(&h, &first);

    let (code, r, e) = h.invoke("feature", "second-task", &target);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["drain"], value!([]));
    let ledger = h.ledger();
    assert_eq!(ledger["entries"].as_array().unwrap().len(), 2);
    assert_eq!(ledger["entries"][0], first);
    assert_eq!(ledger["entries"][1]["branch"], "feature/second-task");
    assert_ne!(
        ledger["entries"][1]["enrollment"]["receipt"],
        first["enrollment"]["receipt"]
    );
    for branch in ["second-task", "third-task"] {
        let (code, r, e) = h.auto(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        );
        assert_eq!(code, 0, "{r} {e}");
        assert_eq!(
            r["managed_process"]["stdout"],
            format!("feature/{branch}\n")
        );
        assert_eq!(
            fs::read(target.join("output λ/use")).unwrap(),
            b"managed-output"
        );
        originals.push((
            h.root
                .join(r["managed_use_receipt"]["path"].as_str().unwrap()),
            fs::read(
                h.root
                    .join(r["managed_use_receipt"]["path"].as_str().unwrap()),
            )
            .unwrap(),
        ));
        let (code, r, e) = h.auto("finish", &args);
        assert_eq!(code, 0, "{r} {e}");
        // 原业务交接与实际清理分别核原结果。
        let (r_cleanup_code, r_cleanup, r_cleanup_error) = h.auto("maintain", &[]);
        assert_eq!(r_cleanup_code, 0, "{r_cleanup} {r_cleanup_error}");
        assert!(!target.exists());
        assert_eq!(r_cleanup["drain"].as_array().unwrap().len(), 1);
        assert_eq!(
            drain_report(&r_cleanup["drain"][0])["branch_ref"],
            format!("feature/{branch}")
        );
        assert_eq!(
            drain_report(&r_cleanup["drain"][0])["worktree_removal"],
            "verified-absent"
        );
        assert_eq!(h.ledger()["entries"][0], first);
        let entries = h.ledger()["entries"].as_array().unwrap().clone();
        let retired = entries.last().unwrap().clone();
        assert_eq!(retired["status"], "disposed");
        assert_eq!(retired["uses"], value!([]));
        originals.extend(entry_receipts(&h, &retired));
        let (code, r, e) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{r} {e}");
        assert_eq!(r["drain"], value!([]));
        assert_eq!(h.ledger()["entries"], value!(entries));
        for (path, bytes) in &originals {
            assert_eq!(&fs::read(path).unwrap(), bytes);
        }
        if branch == "second-task" {
            git(
                &h.root,
                &[
                    "worktree",
                    "add",
                    "-b",
                    "feature/third-task",
                    target.to_str().unwrap(),
                    "dev",
                ],
            );
            let before_observation = h.ledger();
            let (code, observed, error) = h.auto("maintain", &[]);
            assert_eq!(code, 0, "{observed} {error}");
            assert_eq!(inventory_row(&observed, &target)["enrollment_match"], false);
            assert_eq!(inventory_row(&observed, &target)["status"], "preserved");
            assert_eq!(h.ledger(), before_observation);
            let (code, r, e) = h.auto("import", &["--path", target.to_str().unwrap()]);
            assert_eq!(code, 0, "{r} {e}");
            assert_eq!(h.ledger()["entries"][2]["enrollment"]["kind"], "import");
            assert_eq!(
                &h.ledger()["entries"].as_array().unwrap()[..2],
                entries.as_slice()
            );
        }
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn absent_cache_maintenance_preserves_unknown_work_without_retrying_checkout_io() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("missing-cache-owner");
    let (code, created, error) = h.invoke("feature", "missing-cache-owner", &target);
    assert_eq!(code, 0, "{created} {error}");
    git(
        &h.root,
        &["worktree", "remove", "--force", target.to_str().unwrap()],
    );
    let ledger_path = h.ledger_path(&target);
    let original = fs::read(&ledger_path).unwrap();
    let entry = json(&original).unwrap()["entries"][0].clone();
    assert_eq!(entry["ownership"]["cache_pending"], true);
    let refs = git(&h.root, &["show-ref"]);
    for _ in 0..2 {
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        assert!(report["cleanup_failures"].as_array().unwrap().is_empty());
        let observation = report["drain"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == value!(target))
            .unwrap();
        assert_eq!(observation["status"], "preserved");
        assert_eq!(observation["checkout_presence"], "absent");
        assert_eq!(observation["task_completion"], "unknown");
        assert_eq!(observation["worktree_removal"], "not-claimed");
        assert!(observation["receipt"].is_null());
        assert_eq!(fs::read(&ledger_path).unwrap(), original);
        assert!(!target.exists());
        assert_eq!(git(&h.root, &["show-ref"]), refs);
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn absent_cache_maintenance_rejects_partial_and_relocated_attachments() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for conflict in ["metadata", "relocated"] {
        let h = Host::new("payload");
        h.kernel_cleanup();
        let target = h.parent.join("missing-cache-owner");
        assert_eq!(h.invoke("feature", "missing-cache-owner", &target).0, 0);
        let entry = h.ledger()["entries"][0].clone();
        git(
            &h.root,
            &["worktree", "remove", "--force", target.to_str().unwrap()],
        );
        let metadata = PathBuf::from(entry["attachment"]["metadata"].as_str().unwrap());
        let relocated = h.parent.join("relocated-owner");
        if conflict == "metadata" {
            fs::create_dir_all(&metadata).unwrap();
            fs::write(metadata.join("unknown-work"), b"original metadata conflict").unwrap();
        } else {
            git(
                &h.root,
                &[
                    "worktree",
                    "add",
                    relocated.to_str().unwrap(),
                    "feature/missing-cache-owner",
                ],
            );
            fs::write(relocated.join("payload"), b"new unsaved work").unwrap();
        }
        let original = fs::read(h.ledger_path(&target)).unwrap();
        let refs = git(&h.root, &["show-ref"]);
        let (code, report, error) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{conflict}: {report} {error}");
        assert!(
            report["cleanup_failures"]
                .as_array()
                .unwrap()
                .iter()
                .any(|failure| {
                    failure
                        .as_str()
                        .unwrap()
                        .contains(if conflict == "metadata" {
                            "partial or unknown attachment"
                        } else {
                            "another Git attachment"
                        })
                }),
            "{conflict}: {report}"
        );
        assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), original);
        assert!(!target.exists());
        assert_eq!(git(&h.root, &["show-ref"]), refs);
        if conflict == "metadata" {
            assert_eq!(
                fs::read(metadata.join("unknown-work")).unwrap(),
                b"original metadata conflict"
            );
        } else {
            assert_eq!(
                fs::read(relocated.join("payload")).unwrap(),
                b"new unsaved work"
            );
        }
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_absent_active_enrollment_conflict_preflights_without_creation() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let target = h.parent.join("unresolved");
    assert_eq!(h.invoke("feature", "original-task", &target).0, 0);
    let original = h.ledger();
    // External removal does not establish a terminal handoff in this owner.
    git(
        &h.root,
        &["worktree", "remove", "--force", target.to_str().unwrap()],
    );
    assert!(!target.exists());
    let refs = git(&h.root, &["show-ref"]);
    let inventory = git(&h.root, &["worktree", "list", "--porcelain"]);
    let (code, r, e) = h.invoke("feature", "conflicting-task", &target);
    assert_ne!(code, 0, "{r} {e}");
    assert!(
        r["error"].as_str().unwrap().contains("already enrolled"),
        "{r} {e}"
    );
    assert!(
        !target.exists(),
        "registration conflict must precede checkout creation: {r}"
    );
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    assert_eq!(
        git(&h.root, &["worktree", "list", "--porcelain"]),
        inventory
    );
    assert_eq!(h.ledger(), original);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_cache_only_preserves_evidence_dirty_and_unretained_work() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for reason in ["evidence", "dirty", "unretained"] {
        let h = Host::new("payload");
        h.automatic("evidence-retain");
        let target = h.parent.join("preserved");
        assert_eq!(h.invoke("feature", "preserved", &target).0, 0);
        output(&target);
        match reason {
            "evidence" => {
                fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
                fs::write(target.join(".chrono-harness/state/receipt"), "needed").unwrap();
            }
            "dirty" => fs::write(target.join("payload"), "unsaved").unwrap(),
            _ => {
                fs::write(target.join("payload"), "unretained commit").unwrap();
                commit(&target);
            }
        }
        let args = if reason == "evidence" {
            vec!["--path", target.to_str().unwrap()]
        } else {
            vec!["--path", target.to_str().unwrap(), "--dispose-evidence"]
        };
        let (code, r, e) = h.auto("finish", &args);
        assert_eq!(code, 0, "{reason}: {r} {e}");
        // 原业务交接与实际清理分别核原结果。
        let (r_cleanup_code, r_cleanup, r_cleanup_error) = h.auto("maintain", &[]);
        assert_eq!(r_cleanup_code, 0, "{r_cleanup} {r_cleanup_error}");
        assert!(target.exists());
        assert!(!target.join("output λ").exists());
        assert!(drain_report(&r_cleanup["drain"][0])["preserved_reason"].is_string());
        if reason == "evidence" {
            assert!(target.join(".chrono-harness/state/receipt").exists());
        }
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn explicit_current_custody_accepts_an_existing_client_branch_without_claiming_birth() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("client-checkout");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "-b",
            "client/current-work",
            target.to_str().unwrap(),
            "dev",
        ],
    );
    let original_head = git(&target, &["rev-parse", "HEAD"]);
    let original_source = fs::read(target.join("payload")).unwrap();

    let (code, explicit, error) = h.maintain(
        "cleanup",
        value!({"schema":"chrono-worktree-maintenance/v1","operation":"cleanup",
            "path":target,"branch":"client/current-work","head":original_head,
            "retained_ref":"refs/heads/dev","retained_commit":git(&h.root,&["rev-parse","dev"]),
            "retention":"ancestor","discard_artifacts":[],"remove_branch":false,
            "allow_absent_worktree":false}),
    );
    assert_ne!(code, 0, "{explicit} {error}");
    assert!(
        explicit["error"]
            .as_str()
            .unwrap()
            .contains("registered work prefix")
    );
    assert!(target.exists());

    let (code, observation, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{observation} {error}");
    let missing = observation["worktree_inventory"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == target.to_str().unwrap())
        .unwrap();
    assert_eq!(missing["enrollment_match"], false);
    assert!(
        missing["preserved_reason"]
            .as_str()
            .unwrap()
            .contains("explicit import")
    );
    assert!(target.exists());

    // A refused command leaves an address record; an empty ledger is still unowned.
    assert_ne!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    let empty = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert!(empty["entries"].as_array().unwrap().is_empty());
    let (code, observation, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{observation} {error}");
    let empty = observation["worktree_inventory"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == target.to_str().unwrap())
        .unwrap();
    assert_eq!(empty["enrollment_match"], false);
    assert!(target.exists());

    let (code, imported, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{imported} {error}");
    let entry = h.ledger()["entries"][0].clone();
    assert_eq!(entry["branch"], "client/current-work");
    assert_eq!(entry["status"], "active");
    assert_eq!(entry["enrollment"]["kind"], "import");
    assert_eq!(
        entry["enrollment"]["receipt"]["historical_birth"],
        "not-claimed"
    );
    assert!(entry["terminal"].is_null());
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), original_head);
    assert_eq!(fs::read(target.join("payload")).unwrap(), original_source);

    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    assert!(!target.exists());
    assert_eq!(h.ledger()["entries"][0]["status"], "disposed");
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains(target.to_str().unwrap()));
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_import_is_explicit_and_not_a_historical_birth() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["artifacts"].as_array_mut().unwrap().extend([
        value!({"path":"output λ/","owner":"host","kind":"arbitrary-output","tracked":false}),
        value!({"path":"nested/cache/","owner":"host","kind":"another-layout","tracked":false}),
    ]);
    fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
    h.policy(|_| ());
    let target = h.parent.join("legacy");
    assert_eq!(h.invoke("feature", "legacy", &target).0, 0);
    h.automatic("evidence-retain");
    // Import the unchanged legacy checkout, without inventing a birth or rewriting its source.
    assert!(!target.join(AUTO_POLICY).exists());
    assert_ne!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    let (code, r, e) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(h.ledger()["entries"][0]["enrollment"]["kind"], "import");
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!target.exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn observational_import_binds_original_inputs_without_adopting_format_or_disposal_scopes() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    native_consumer(&h, "ordinary");
    for (detached, retired) in [(false, false), (false, true), (true, false), (true, true)] {
        let target = h
            .parent
            .join(format!("observational import {detached} {retired}"));
        let branch = format!("client/observational-import-{detached}-{retired}");
        let mut args = vec!["worktree", "add"];
        if detached {
            args.push("--detach");
        } else {
            args.extend(["-b", &branch]);
        }
        args.extend([target.to_str().unwrap(), "dev"]);
        git(&h.root, &args);
        let config_path = target.join(POLICY);
        let mut config = json(&fs::read(&config_path).unwrap()).unwrap();
        if retired {
            config["schema"] = value!("chrono-worktree-config/v3");
            config["git"]["identities"] = value!([]);
        }
        let config_bytes = serde_json::to_vec_pretty(&config).unwrap();
        fs::write(&config_path, &config_bytes).unwrap();
        let mut registry = json(&fs::read(target.join(CONFIG)).unwrap()).unwrap();
        registry["artifacts"]
            .as_array_mut()
            .unwrap()
            .retain(|artifact| artifact["path"] != "nested/cache/");
        fs::write(
            target.join(CONFIG),
            serde_json::to_vec_pretty(&registry).unwrap(),
        )
        .unwrap();
        let head = commit(&target);
        fs::write(target.join("payload"), "unfinished source\n").unwrap();
        output(&target);
        let metadata = PathBuf::from(git(&target, &["rev-parse", "--absolute-git-dir"]));
        let originals = [
            target.join(".git"),
            metadata.join("HEAD"),
            metadata.join("index"),
            target.join("payload"),
            target.join(AUTO_POLICY),
            target.join(CONFIG),
        ]
        .map(|path| {
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        });

        let (code, report, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
        assert_eq!(code, 0, "{report} {error}");
        fs::write(&config_path, [&config_bytes[..], b"\n"].concat()).unwrap();
        let (code, refused, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
        assert_ne!(code, 0, "{refused} {error}");
        assert!(
            refused["error"]
                .as_str()
                .unwrap()
                .contains("differs from commit")
        );
        fs::write(&config_path, &config_bytes).unwrap();
        let (code, imported, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
        assert_eq!(code, 0, "{imported} {error}");
        assert_eq!(imported["policy_source_root"], value!(h.root));
        let ledger_bytes = fs::read(h.ledger_path(&target)).unwrap();
        let ledger = json(&ledger_bytes).unwrap();
        let entry = &ledger["entries"][0];
        assert_eq!(entry["enrollment"]["observed_head"], head);
        assert_eq!(
            entry["enrollment"]["policy_inputs"][POLICY],
            sha256(&config_bytes)
        );
        assert_eq!(entry["adopted_policy"]["head"], head);
        assert_eq!(
            entry["adopted_policy"]["inputs"],
            entry["enrollment"]["policy_inputs"]
        );
        assert_eq!(
            entry["adopted_policy"]["policy_source"]["head"],
            imported["policy_source_commit"]
        );
        assert_eq!(
            entry["adopted_policy"]["policy_source"]["inputs"][POLICY],
            sha256(&fs::read(h.root.join(POLICY)).unwrap())
        );
        assert_eq!(
            entry["adopted_policy"]["policy_source"]["inputs"][AUTO_POLICY],
            entry["policy_sha256"]
        );
        assert!(entry["terminal"].is_null());
        let (code, observed, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
        assert_eq!(code, 0, "{observed} {error}");
        let row = observed["worktree_status"]["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == value!(target))
            .unwrap();
        assert_eq!(row["work_state"], "active");
        assert_eq!(row["attention_required"], true);
        for operation in ["use", "migrate", "finish"] {
            let mut args = vec!["--path", target.to_str().unwrap()];
            if operation == "use" {
                args.extend(["--operation", "use.consumer"]);
            }
            let (code, refused, error) = h.auto(operation, &args);
            assert_ne!(code, 0, "{operation}: {refused} {error}");
            assert!(
                refused["error"].as_str().unwrap().contains(if retired {
                    "identities"
                } else {
                    "disposal directory is not an artifact"
                }),
                "{operation}: {refused} {error}"
            );
        }
        assert_eq!(fs::read(h.ledger_path(&target)).unwrap(), ledger_bytes);
        assert_eq!(fs::read(&config_path).unwrap(), config_bytes);
        assert!(target.join("output λ/data").is_file());
        for (path, bytes) in originals {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn detached_import_preserves_unresolved_index_and_reports_unfinished_work() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    git(&h.root, &["checkout", "-b", "client/other-side"]);
    fs::write(h.root.join("payload"), "other side\n").unwrap();
    let other = commit(&h.root);
    git(&h.root, &["checkout", "dev"]);
    fs::write(h.root.join("payload"), "current side\n").unwrap();
    let head = commit(&h.root);
    let target = h.parent.join("detached unresolved checkout");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "--detach",
            target.to_str().unwrap(),
            &head,
        ],
    );
    git(&target, &["read-tree", "-m", &base, &head, &other]);
    assert!(!git(&target, &["ls-files", "--unmerged"]).is_empty());
    let metadata = PathBuf::from(git(&target, &["rev-parse", "--absolute-git-dir"]));
    let original = [
        target.join(".git"),
        metadata.join("HEAD"),
        metadata.join("index"),
        target.join("payload"),
    ]
    .map(|path| {
        let bytes = fs::read(&path).unwrap();
        (path, bytes)
    });

    let (code, report, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    let (code, migrated, error) = h.auto("migrate", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{migrated} {error}");
    let (code, status, error) = h.auto("status", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{status} {error}");
    let row = status["worktree_status"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target))
        .unwrap();
    assert_eq!(row["branch"], "HEAD");
    assert_eq!(row["work_state"], "active");
    assert_eq!(row["attention_required"], true);
    assert_eq!(row["next_action"], "complete-work-and-finish");
    let ledger = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert!(ledger["entries"][0]["terminal"].is_null());
    for (path, bytes) in original {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert!(!git(&target, &["ls-files", "--unmerged"]).is_empty());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn detached_finish_removes_only_the_enrolled_checkout_and_keeps_all_branches() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let path = h.root.join(AUTO_POLICY);
    let mut policy = json(&fs::read(&path).unwrap()).unwrap();
    policy["remove_branch"] = value!(true);
    fs::write(path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    commit(&h.root);
    let target = h.parent.join("detached finished checkout");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "--detach",
            target.to_str().unwrap(),
            "dev",
        ],
    );
    let branches = git(&h.root, &["for-each-ref", "refs/heads/"]);
    let (code, report, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    output(&target);
    let (code, finished, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{finished} {error}");
    // 原业务交接与实际清理分别核原结果。
    let (finished_cleanup_code, finished_cleanup, finished_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(
        finished_cleanup_code, 0,
        "{finished_cleanup} {finished_cleanup_error}"
    );
    assert!(!target.exists(), "{finished}");
    assert_eq!(git(&h.root, &["for-each-ref", "refs/heads/"]), branches);
    let row = finished_cleanup["drain"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == value!(target))
        .unwrap();
    let cleanup = drain_report(row);
    assert_eq!(cleanup["branch_removal"], "not-applicable-detached");
    assert_eq!(cleanup["branch_removed"], false);
    assert_eq!(cleanup["worktree_removal"], "verified-absent");
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn detached_enrollment_rejects_later_branch_attachment_without_changing_work() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("changed detached checkout");
    git(
        &h.root,
        &[
            "worktree",
            "add",
            "--detach",
            target.to_str().unwrap(),
            "dev",
        ],
    );
    let (code, report, error) = h.auto("import", &["--path", target.to_str().unwrap()]);
    assert_eq!(code, 0, "{report} {error}");
    git(&target, &["checkout", "-b", "client/reattached"]);
    let metadata = PathBuf::from(git(&target, &["rev-parse", "--absolute-git-dir"]));
    let index = fs::read(metadata.join("index")).unwrap();
    let branches = git(&h.root, &["for-each-ref", "refs/heads/"]);
    let (code, refused, error) = h.auto("finish", &["--path", target.to_str().unwrap()]);
    assert_ne!(code, 0, "{refused} {error}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("identity or ownership lock mismatch")
    );
    assert!(target.is_dir());
    assert_eq!(fs::read(metadata.join("index")).unwrap(), index);
    assert_eq!(git(&h.root, &["for-each-ref", "refs/heads/"]), branches);
    let ledger = json(&fs::read(h.ledger_path(&target)).unwrap()).unwrap();
    assert!(ledger["entries"][0]["terminal"].is_null());
}

#[cfg(not(verus_keep_ghost))]
fn blocked_remove(h: &Host, after_effect: bool) {
    native_git(h, "automatic-remove", value!({"after_effect":after_effect}));
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_partial_failure_preserves_independent_start_and_explicit_retry() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for after_effect in [false, true] {
        let h = Host::new("payload");
        blocked_remove(&h, after_effect);
        h.automatic("evidence-retain");
        let target = h.parent.join("pending");
        assert_eq!(h.invoke("feature", "pending", &target).0, 0);
        output(&target);
        fs::write(h.parent.join("fail-remove"), "one failure").unwrap();
        let (code, failed, e) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_eq!(code, 0, "{failed} {e}");
        assert!(
            target.exists(),
            "finish only records the original terminal responsibility"
        );
        let (code, failed, e) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{failed} {e}");
        let original = &drain_report(&failed["drain"][0]);
        assert_eq!(original["status"], "failed");
        assert_eq!(original["worktree_removal"], "attempted-unverified");
        assert_eq!(original["worktree_removed"], false);
        assert_eq!(
            original["artifact_disposals"][0]["status"],
            "verified-absent"
        );
        assert_eq!(target.exists(), !after_effect);
        assert!(original["processes"].as_array().unwrap().iter().any(|p| {
            p["process"]["exit_code"] == 71
                && p["process"]["stderr"]
                    .as_str()
                    .unwrap_or("")
                    .contains("original automatic removal failure")
        }));
        let receipt = h.root.join(original["report_path"].as_str().unwrap());
        let bytes = fs::read(&receipt).unwrap();
        let next = h.parent.join("next");
        if !after_effect {
            let independent = h.parent.join("independent");
            let (code, admitted, error) = h.invoke("feature", "independent", &independent);
            assert_eq!(code, 0, "{} {error}", admitted["error"]);
            assert!(independent.exists());
            assert!(admitted["cleanup_failures"].as_array().unwrap().is_empty());
            assert!(admitted["drain"].as_array().unwrap().is_empty());
            assert_eq!(fs::read(&receipt).unwrap(), bytes);
        }
        fs::remove_file(h.parent.join("fail-remove")).unwrap();
        let (code, r, e) = h.invoke("feature", "next", &next);
        assert_eq!(code, 0, "{r} {e}");
        assert_eq!(target.exists(), !after_effect);
        assert!(next.exists());
        assert_eq!(fs::read(receipt).unwrap(), bytes);
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(!target.exists());
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_retries_transfer_lock_recovery_without_rooting_old_attempts() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for kernel in [false, true] {
        let h = Host::new("payload");
        blocked_remove(&h, false);
        if kernel {
            h.kernel_cleanup();
        } else {
            h.automatic("evidence-retain");
        }
        let target = h.parent.join("bounded-retry");
        assert_eq!(h.invoke("feature", "bounded-retry", &target).0, 0);
        output(&target);
        fs::write(h.parent.join("fail-remove"), "retain original failure").unwrap();
        let (code, failed, error) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_eq!(code, 0, "{failed} {error}");
        assert!(
            target.exists(),
            "finish only records the original terminal responsibility"
        );
        let (code, failed, error) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{failed} {error}");
        let report = &drain_report(&failed["drain"][0]);
        let original_path = h.root.join(report["report_path"].as_str().unwrap());
        let original = fs::read(&original_path).unwrap();
        let bound = original.len() * 3;
        let mut originals = vec![(original_path, original)];
        for retry in 0..3 {
            let (code, failed, error) = h.auto("maintain", &[]);
            assert_ne!(code, 0, "{failed} {error}");
            let report = &drain_report(&failed["drain"][0]);
            assert_eq!(report["status"], "failed");
            assert!(report["processes"].as_array().unwrap().iter().any(|p| {
                p["process"]["exit_code"] == 71
                    && p["process"]["stderr"]
                        .as_str()
                        .unwrap_or("")
                        .contains("original automatic removal failure")
            }));
            let path = h.root.join(report["report_path"].as_str().unwrap());
            let bytes = fs::read(&path).unwrap();
            // The controlled operation is unchanged across retries. Its report
            // may grow with current observations, but cannot multiply history.
            assert!(
                bytes.len() <= bound,
                "retry {retry}, kernel={kernel}: {} bytes exceeds current-operation bound {bound}",
                bytes.len()
            );
            assert!(report["prior_report"].is_null());
            let recovered = &report["cleanup_lock_recovery"];
            assert_eq!(
                recovered["source"]["path"],
                originals
                    .last()
                    .unwrap()
                    .0
                    .strip_prefix(&h.root)
                    .unwrap()
                    .to_str()
                    .unwrap()
            );
            assert_eq!(
                recovered["source"]["sha256"],
                sha256(&originals.last().unwrap().1)
            );
            assert_eq!(recovered["path"], value!(target));
            assert_eq!(
                recovered["handoff"],
                "prior-lock-released-under-original-exclusion"
            );
            let attempts = h.ledger()["entries"][0]["attempts"].clone();
            assert_eq!(attempts.as_array().unwrap().len(), 1);
            assert_eq!(attempts[0]["path"], report["report_path"]);
            for (path, bytes) in &originals {
                assert_eq!(fs::read(path).unwrap(), *bytes);
            }
            originals.push((path, bytes));
        }
        fs::remove_file(h.parent.join("fail-remove")).unwrap();
        for fault in ["latest", "absent", "symlink"] {
            let (path, bytes) = originals.last().unwrap();
            match fault {
                "absent" => fs::remove_file(path).unwrap(),
                "symlink" => {
                    fs::remove_file(path).unwrap();
                    let outside = h.parent.join("same-original-bytes");
                    fs::write(&outside, bytes).unwrap();
                    std::os::unix::fs::symlink(&outside, path).unwrap();
                }
                _ => {
                    let mut drifted = bytes.clone();
                    drifted.push(b' ');
                    fs::write(path, drifted).unwrap();
                }
            }
            let (code, refused, error) = h.auto("maintain", &[]);
            assert_ne!(code, 0, "{fault}: {refused} {error}");
            if fault == "latest" {
                assert!(
                    refused["error"]
                        .as_str()
                        .unwrap()
                        .contains("original lifecycle receipt changed")
                );
            }
            assert!(
                target.exists(),
                "{fault}: invalid evidence cannot authorize deletion"
            );
            if fault == "symlink" {
                fs::remove_file(path).unwrap();
            }
            fs::write(path, bytes).unwrap();
        }
        let (ancestor, ancestor_bytes) = &originals[0];
        fs::remove_file(ancestor).unwrap();
        assert_eq!(
            h.auto("maintain", &[]).0,
            0,
            "a transferred ancestor is no longer a recovery input"
        );
        assert!(!ancestor.exists());
        fs::write(ancestor, ancestor_bytes).unwrap();

        assert!(!target.exists());
        for (path, bytes) in originals {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_symlink_containment_and_internal_external_target_safety() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use std::os::unix::fs::symlink;
    for fault in ["internal", "ancestor", "selected-root"] {
        let h = Host::new("payload");
        h.automatic("evidence-retain");
        let target = h.parent.join("links");
        assert_eq!(h.invoke("feature", "links", &target).0, 0);
        output(&target);
        let outside = h.parent.join("external-data");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep"), "outside").unwrap();
        match fault {
            "internal" => symlink(&outside, target.join("output λ/link")).unwrap(),
            "ancestor" => symlink(&outside, target.join("nested")).unwrap(),
            _ => {
                fs::remove_dir_all(target.join("output λ")).unwrap();
                symlink(&outside, target.join("output λ")).unwrap();
            }
        }
        let (code, r, e) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_eq!(code, 0, "{r} {e}");
        let (code, r, e) = h.auto("maintain", &[]);
        assert_eq!(fs::read(outside.join("keep")).unwrap(), b"outside");
        if fault == "internal" {
            assert_eq!(code, 0, "{r} {e}");
            assert!(!target.exists());
        } else {
            assert_ne!(code, 0, "{r} {e}");
            assert!(target.exists());
            if fault == "ancestor" {
                assert!(
                    target.join("output λ/data").exists(),
                    "whole selection must preflight before deletion"
                );
            }
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn native_git(h: &Host, mode: &str, mut parameters: Value) -> PathBuf {
    let input = h.parent.join("git-fixture.json");
    parameters["mode"] = value!(mode);
    parameters["git"] = value!(fixture_git());
    fs::write(&input, serde_json::to_vec(&parameters).unwrap()).unwrap();
    h.policy(|policy| {
        policy["git"]["program"] = value!(env!("CARGO_BIN_EXE_chrono-worktree-test-git"));
        policy["environment"]["values"]["CHRONO_TEST_GIT_FIXTURE"] = value!(input);
    });
    input
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn native_consumer(h: &Host, mode: &str) {
    native_consumer_actions(h, mode, "noop");
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn native_consumer_actions(h: &Host, mode: &str, test_mode: &str) {
    super::check_inputs::ready_version_program(
        &h.root,
        Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-consumer")),
        b"fixture\n",
    );
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["tools"].as_array_mut().unwrap().push(value!({
        "id":"consumer", "program":env!("CARGO_BIN_EXE_chrono-worktree-test-consumer"),
        "resolution":"PATH-once", "version_argv":["--version"], "expected_version":"fixture"
    }));
    if matches!(mode, "bounded-cargo" | "cargo-run") {
        // Capture the actual Cargo selection before auto_command clears its environment.
        for name in ["HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN"] {
            if let Ok(value) = std::env::var(name) {
                config["environment"]["values"][name] = value!(value);
            }
        }
    }
    fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
    consuming_scripts(
        h,
        value!([
            {"id":"consumer","path":"consumer.rs","test_script":"consumer-tests","actions":{"execute":{"operation":"use.consumer","tool":"consumer","argv":[mode]}}},
            {"id":"consumer-tests","path":"consumer-tests.rs","tests_for":"consumer","actions":{"execute":{"operation":"test.consumer","tool":"consumer","argv":[test_mode]}}}
        ]),
        &[
            ("consumer.rs", include_bytes!("support/consumer.rs")),
            ("consumer-tests.rs", include_bytes!("support/consumer.rs")),
            (
                "native_cargo/Cargo.toml",
                include_bytes!("support/native_cargo/Cargo.toml"),
            ),
            (
                "native_cargo/src/lib.rs",
                include_bytes!("support/native_cargo/src/lib.rs"),
            ),
        ],
    );
}

#[cfg(not(verus_keep_ghost))]
fn consuming_scripts(h: &Host, scripts: Value, sources: &[(&str, &[u8])]) {
    let mut projects = json(&fs::read(h.root.join(PROJECTS)).unwrap()).unwrap();
    projects["owners"]
        .as_array_mut()
        .unwrap()
        .extend([value!("consumer"), value!("consumer-tests")]);
    projects["scripts"] = scripts;
    fs::write(
        h.root.join(PROJECTS),
        serde_json::to_vec(&projects).unwrap(),
    )
    .unwrap();
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    fm["files"]
        .as_array_mut()
        .unwrap()
        .extend(sources.iter().map(|(path, _)| file(path, value!([]))));
    fm["project_edges"] = value!([edge(
        "script:consumer",
        "test-execution",
        "test:consumer-tests"
    )]);
    fm["test_costs"] = value!([{"test":"consumer-tests","cost":"unknown"}]);
    fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    for (path, bytes) in sources {
        fs::create_dir_all(h.root.join(path).parent().unwrap()).unwrap();
        fs::write(h.root.join(path), bytes).unwrap();
    }
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_managed_real_child_blocks_finish_and_exit_releases_use() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    native_consumer(&h, "managed-child");
    let target = h.parent.join("in-use");
    assert_eq!(h.invoke("feature", "in-use", &target).0, 0);
    output(&target);
    let mut child = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        ),
        &h.root,
    );
    child.release_on_drop(target.join(".chrono-harness/state/release"));
    // Input preparation precedes the consumer. Use its registered process
    // bounds and retain the wrapper's complete result, as other managed tests do.
    child.await_managed_file(&target.join(".chrono-harness/state/ready"));
    let ready = target.join(".chrono-harness/state/ready").exists();
    // Join the wrapper before asserting, including its failure path. A missing
    // readiness marker must expose the real wrapper result and retain its receipt.
    let finish = ready.then(|| {
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        )
    });
    let output_retained = target.join("output λ/data").exists();
    let uses = h.ledger()["entries"][0]["uses"].clone();
    let release = fs::write(target.join(".chrono-harness/state/release"), "join now");
    let (code, r, e) = h.received(child.wait_with_output().unwrap());
    assert!(
        ready,
        "managed child did not become ready: exit {code}; {r} {e}"
    );
    release.unwrap();
    let (finish_code, finish_report, finish_error) = finish.unwrap();
    assert_ne!(finish_code, 0, "{finish_report} {finish_error}");
    assert!(output_retained);
    assert_eq!(uses.as_array().unwrap().len(), 1);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["managed_process"]["stdout"], "joined-child");
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    let (code, r, e) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{r} {e}");
    // 原业务交接与实际清理分别核原结果。
    let (r_cleanup_code, r_cleanup, r_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(r_cleanup_code, 0, "{r_cleanup} {r_cleanup_error}");
    assert!(!target.exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_terminal_rejects_changed_head_rewound_ref_policy_attachment_and_git_locks() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for fault in [
        "head",
        "rewound-ref",
        "diverged-ref",
        "symbolic-ref",
        "policy",
        "target-policy",
        "target-config",
        "attachment",
        "git-lock",
        "worktree-lock",
    ] {
        let h = Host::new("payload");
        blocked_remove(&h, false);
        h.automatic("evidence-retain");
        let target = h.parent.join("drift");
        assert_eq!(h.invoke("feature", "drift", &target).0, 0);
        output(&target);
        fs::write(h.parent.join("fail-remove"), "fail").unwrap();
        assert_eq!(
            h.auto(
                "finish",
                &["--path", target.to_str().unwrap(), "--dispose-evidence"]
            )
            .0,
            0
        );
        assert_ne!(h.auto("maintain", &[]).0, 0);
        fs::remove_file(h.parent.join("fail-remove")).unwrap();
        output(&target);
        match fault {
            "head" => {
                fs::write(target.join("payload"), "changed head").unwrap();
                commit(&target);
            }
            "rewound-ref" => {
                git(&h.root, &["update-ref", "refs/heads/dev", "HEAD^"]);
            }
            "diverged-ref" => {
                git(&h.root, &["update-ref", "refs/heads/dev", "HEAD^"]);
                git(
                    &h.root,
                    &[
                        "-c",
                        "user.name=fixture",
                        "-c",
                        "user.email=fixture@example.invalid",
                        "commit",
                        "--allow-empty",
                        "-qm",
                        "diverged reference",
                    ],
                );
            }
            "symbolic-ref" => {
                git(
                    &h.root,
                    &["update-ref", "refs/heads/retention-alias", "HEAD"],
                );
                git(
                    &h.root,
                    &[
                        "symbolic-ref",
                        "refs/heads/dev",
                        "refs/heads/retention-alias",
                    ],
                );
            }
            "policy" => {
                let mut p = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
                p["remove_branch"] = value!(true);
                fs::write(h.root.join(AUTO_POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            }
            "target-policy" => {
                let mut p = json(&fs::read(target.join(AUTO_POLICY)).unwrap()).unwrap();
                p["remove_branch"] = value!(true);
                fs::write(target.join(AUTO_POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            }
            "target-config" => {
                let mut p = json(&fs::read(target.join(POLICY)).unwrap()).unwrap();
                p["output_limit_bytes"] = value!(8192);
                fs::write(target.join(POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            }
            "attachment" => {
                fs::rename(target.join(".git"), h.parent.join("original-gitfile")).unwrap();
                fs::write(
                    target.join(".git"),
                    fs::read(h.parent.join("original-gitfile")).unwrap(),
                )
                .unwrap();
            }
            "git-lock" => {
                let metadata = git(&target, &["rev-parse", "--absolute-git-dir"]);
                fs::write(Path::new(&metadata).join("index.lock"), "owned").unwrap();
            }
            _ => {
                git(
                    &h.root,
                    &[
                        "worktree",
                        "lock",
                        "--reason",
                        "another-owner",
                        target.to_str().unwrap(),
                    ],
                );
            }
        }
        let (code, r, e) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{fault}: {r} {e}");
        assert!(target.exists());
        assert!(target.join("output λ/data").exists());
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_terminal_resumes_after_ref_advance_using_the_original_saved_commit() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for retention in ["ancestor", "same-tree"] {
        let h = Host::new("payload");
        h.automatic("evidence-retain");
        let mut policy = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
        policy["retention"] = value!(retention);
        fs::write(
            h.root.join(AUTO_POLICY),
            serde_json::to_vec(&policy).unwrap(),
        )
        .unwrap();
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
        let target = h.parent.join("saved-before-advance");
        assert_eq!(h.invoke("feature", "saved-before-advance", &target).0, 0);
        output(&target);
        let (code, report, error) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        assert_eq!(code, 0, "{report} {error}");
        let terminal = h.ledger()["entries"][0]["terminal"].clone();
        let original_path = h.root.join(terminal["receipt"]["path"].as_str().unwrap());
        let original_bytes = fs::read(&original_path).unwrap();
        let saved = terminal["retained_commit"].as_str().unwrap();
        fs::write(h.root.join("payload"), "later independent change\n").unwrap();
        let advanced = commit(&h.root);
        assert_ne!(advanced, saved);
        assert_ne!(
            git(&h.root, &["rev-parse", &format!("{advanced}^{{tree}}")]),
            git(&h.root, &["rev-parse", &format!("{saved}^{{tree}}")]),
        );
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{retention}: {report} {error}");
        assert!(!target.exists(), "the saved checkout must be reclaimed");
        assert_eq!(git(&h.root, &["rev-parse", "dev"]), advanced);
        assert_eq!(fs::read(&original_path).unwrap(), original_bytes);
        assert_eq!(h.ledger()["entries"][0]["terminal"], terminal);
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_reconstruct_enrolls_only_success_and_unresolved_evidence_can_later_settle() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let head = git(&h.root, &["rev-parse", "HEAD"]);
    let (code, r, e) = h.reconstruct(reconstruction(&head, &head, value!([])), "reborn");
    assert_eq!(code, 0, "{r} {e}");
    let target = h.parent.join("reborn");
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(
        target.join(".chrono-harness/state/needed-original"),
        "evidence",
    )
    .unwrap();
    output(&target);
    assert_eq!(
        h.ledger()["entries"][0]["enrollment"]["receipt"]["operation"],
        "reconstruct"
    );
    assert_eq!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(target.exists());
    assert_eq!(h.ledger()["entries"][0]["status"], "retained");
    let (code, r, e) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(
        drain_report(&r["drain"][0])["worktree_removal"],
        "preserved"
    );
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!target.exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_same_tree_retention_remains_exact_and_branch_removal_is_opt_in() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let mut policy = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
    policy["retention"] = value!("same-tree");
    policy["remove_branch"] = value!(true);
    fs::write(
        h.root.join(AUTO_POLICY),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("squash");
    assert_eq!(h.invoke("integration", "squash", &target).0, 0);
    fs::write(target.join("payload"), "landed work").unwrap();
    let candidate = commit(&target);
    fs::write(h.root.join("payload"), "landed work").unwrap();
    git(&h.root, &["add", "payload"]);
    git(
        &h.root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-qm",
            "verified squash landing",
        ],
    );
    let landed = git(&h.root, &["rev-parse", "HEAD"]);
    assert_ne!(candidate, landed);
    output(&target);
    let (code, r, e) = h.auto(
        "finish",
        &[
            "--path",
            target.to_str().unwrap(),
            "--dispose-evidence",
            "--retained-commit",
            &landed,
        ],
    );
    assert_eq!(code, 0, "{r} {e}");
    // 原业务交接与实际清理分别核原结果。
    let (r_cleanup_code, r_cleanup, r_cleanup_error) = h.auto("maintain", &[]);
    assert_eq!(r_cleanup_code, 0, "{r_cleanup} {r_cleanup_error}");
    assert!(!target.exists());
    assert!(
        !git(&h.root, &["for-each-ref", "--format=%(refname)"])
            .contains("refs/heads/integration/squash")
    );
    assert_eq!(git(&h.root, &["rev-parse", "dev"]), landed);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_filesystem_partial_failure_retries_under_original_owned_lock() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    let target = h.parent.join("filesystem-failure");
    assert_eq!(h.invoke("feature", "filesystem-failure", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(target.join("nested/cache/keep-until-retry"), "cache").unwrap();
    let special = target.join("nested/cache/special");
    use std::os::unix::ffi::OsStrExt;
    let special_name = std::ffi::CString::new(special.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifo(special_name.as_ptr(), 0o600) },
        0,
        "{}",
        std::io::Error::last_os_error()
    );
    let (code, r, e) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{r} {e}");
    assert!(
        target.exists(),
        "finish only records the original terminal responsibility"
    );
    let (code, r, e) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{r} {e}");
    let original = &drain_report(&r["drain"][0]);
    assert_eq!(
        original["artifact_disposals"][0]["status"],
        "verified-absent"
    );
    assert_eq!(original["worktree_removal"], "not-attempted");
    assert!(target.exists());
    assert!(!target.join("output λ").exists());
    assert!(special.exists());
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"])
            .contains(original["lock_reason"].as_str().unwrap())
    );
    let receipt = h.root.join(original["report_path"].as_str().unwrap());
    let bytes = fs::read(&receipt).unwrap();
    fs::remove_file(special).unwrap();
    let (code, r, e) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    assert_eq!(fs::read(receipt).unwrap(), bytes);
    assert_eq!(
        drain_report(&r["drain"][0])["artifact_disposals"][0]["status"],
        "already-absent"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_failed_birth_and_missing_state_are_protected() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    h.hook(value!({"stderr":"original-birth-failure","exit":17}));
    let failed = h.parent.join("failed");
    assert_ne!(h.invoke("feature", "failed", &failed).0, 0);
    assert!(failed.exists());
    assert_eq!(h.ledger()["entries"], value!([]));
    assert_ne!(h.auto("finish", &["--path", failed.to_str().unwrap()]).0, 0);
    assert!(failed.exists());
    fs::remove_file(h.root.join(".git/hooks/post-checkout")).unwrap();
    let active = h.parent.join("active");
    assert_eq!(h.invoke("feature", "active", &active).0, 0);
    output(&active);
    fs::remove_file(h.ledger_path(&active)).unwrap();
    let next = h.parent.join("next");
    assert_eq!(h.invoke("feature", "next", &next).0, 0);
    assert!(next.exists());
    assert!(active.join("output λ/data").exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_explicit_main_anchor_short_commands_and_finish_from_source_then_start() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("layout/source.ext");
    h.automatic("evidence-retain");
    let mut policy = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
    policy["coordinator_root"] = value!("git-main-worktree");
    fs::write(
        h.root.join(AUTO_POLICY),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("self-finish");
    assert_eq!(h.invoke("feature", "self-finish", &target).0, 0);
    output(&target);
    let out = Command::new(Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-cli")))
        .current_dir(&target)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args(["finish", "--dispose-evidence"])
        .output()
        .unwrap();
    let (code, r, e) = h.received(out);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["drain"], value!([]));
    assert_eq!(r["maintenance_request"]["disposal"], "not-attempted");
    assert!(r["terminal_receipt"]["path"].is_string());
    assert!(target.exists());
    let next = h.parent.join("fresh");
    let (code, r, e) = h.invoke("feature", "fresh", &next);
    assert_eq!(code, 0, "{r} {e}");
    assert!(
        target.exists(),
        "another birth has no cleanup dependency on this target"
    );
    let (code, r, e) = h.received(
        Command::new(Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-cli")))
            .current_dir(&h.root)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .arg("maintain")
            .output()
            .unwrap(),
    );
    assert_eq!(code, 0, "{r} {e}");
    assert!(!target.exists());
    assert!(next.exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_staged_source_inside_artifact_and_unauthorized_evidence_remain() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    for fault in ["staged-source", "evidence-authorization", "gate"] {
        let h = Host::new("payload");
        h.automatic("evidence-retain");
        if fault == "evidence-authorization" {
            let mut p = json(&fs::read(h.root.join(AUTO_POLICY)).unwrap()).unwrap();
            p["allow_evidence_disposal"] = value!(false);
            fs::write(h.root.join(AUTO_POLICY), serde_json::to_vec(&p).unwrap()).unwrap();
            commit(&h.root);
            git(&h.root, &["push", "-q", "warehouse", "dev"]);
        }
        let target = h.parent.join("protected");
        assert_eq!(h.invoke("feature", "protected", &target).0, 0);
        output(&target);
        if fault == "staged-source" {
            git(&target, &["add", "-f", "output λ/data"]);
        }
        if fault == "gate" {
            fs::create_dir(h.record_directory(&target).join("gate.lock")).unwrap();
        }
        let (code, r, e) = h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"],
        );
        if fault == "staged-source" {
            assert_eq!(code, 0, "{fault}: {r} {e}");
            assert_eq!(r["maintenance_request"]["disposal"], "not-attempted");
            let (code, cleanup, error) = h.auto("maintain", &[]);
            assert_ne!(code, 0, "{fault}: {cleanup} {error}");
        } else {
            assert_ne!(code, 0, "{fault}: {r} {e}");
        }
        assert!(target.join("output λ/data").exists());
    }
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_interrupted_managed_wrapper_keeps_unknown_use_protected() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    native_consumer(&h, "unknown-use");
    let target = h.parent.join("unknown-use");
    assert_eq!(h.invoke("feature", "unknown-use", &target).0, 0);
    output(&target);
    let mut wrapper = super::interrupted_cleanup::CapturedChild::spawn(
        &mut h.auto_command(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        ),
        &h.root,
    );
    // Input preparation runs before the managed consumer. Its registered
    // process bounds govern readiness, just as for the other managed entries.
    wrapper.await_managed_file(&target.join(".chrono-harness/state/ready"));
    let child_pid = fs::read_to_string(target.join(".chrono-harness/state/ready")).unwrap();
    wrapper.kill().unwrap();
    let out = wrapper.wait_with_output().unwrap();
    assert_ne!(h.received(out).0, 0);
    // Join exactly the test-owned process group; no process scanning or age heuristic.
    assert_eq!(
        unsafe { libc::kill(-child_pid.trim().parse::<i32>().unwrap(), libc::SIGKILL) },
        0,
        "{}",
        std::io::Error::last_os_error()
    );
    assert_eq!(
        h.ledger()["entries"][0]["uses"].as_array().unwrap().len(),
        1
    );
    assert_ne!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/data").exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_concurrent_managed_completions_release_both_uses() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    native_consumer(&h, "waiting-completion");
    let target = h.parent.join("parallel-use");
    assert_eq!(h.invoke("feature", "parallel-use", &target).0, 0);
    let mut children = vec![];
    for _ in 0..2 {
        children.push(
            h.auto_command(
                "use",
                &[
                    "--path",
                    target.to_str().unwrap(),
                    "--operation",
                    "use.consumer",
                ],
            )
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
        );
    }
    let began = Instant::now();
    while h.ledger()["entries"][0]["uses"].as_array().unwrap().len() != 2
        && began.elapsed() < Duration::from_secs(30)
    {
        thread::sleep(Duration::from_millis(20));
    }
    let admitted = h.ledger()["entries"][0]["uses"].as_array().unwrap().len();
    fs::create_dir_all(target.join(".chrono-harness/state")).unwrap();
    fs::write(target.join(".chrono-harness/state/release"), "joined").unwrap();
    for child in children {
        let (code, r, e) = h.received(child.wait_with_output().unwrap());
        assert_eq!(code, 0, "{r} {e}");
    }
    assert_eq!(
        admitted, 2,
        "both live consumers must enroll before release"
    );
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!target.exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn automatic_use_from_checkout_publishes_at_surviving_anchor() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.automatic("evidence-retain");
    native_consumer(&h, "finished");
    let target = h.parent.join("local-use");
    assert_eq!(h.invoke("feature", "local-use", &target).0, 0);
    let out = Command::new(Path::new(env!("CARGO_BIN_EXE_chrono-worktree-test-cli")))
        .current_dir(&target)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .args(["use", "--operation", "use.consumer"])
        .output()
        .unwrap();
    let (code, r, e) = h.received(out);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["source_root"], value!(h.root));
    assert_eq!(r["invoking_root"], value!(target));
    let receipt = h.root.join(r["report_path"].as_str().unwrap());
    let bytes = fs::read(&receipt).unwrap();
    assert_eq!(
        h.auto(
            "finish",
            &["--path", target.to_str().unwrap(), "--dispose-evidence"]
        )
        .0,
        0
    );
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!target.exists());
    assert_eq!(fs::read(receipt).unwrap(), bytes);
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn status_observes_busy_metadata_without_mutating_enrollments() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    use chrono_harness::ownership::{DirectoryLease, Lease};
    let h = Host::new("payload");
    h.kernel_cleanup();
    let target = h.parent.join("busy-status");
    let (code, created, error) = h.invoke("feature", "busy-status", &target);
    assert_eq!(code, 0, "{created} {error}");
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    h.policy(|policy| policy["timeout_seconds"] = value!(1));
    let originals: Vec<_> = [&h.root, &target]
        .into_iter()
        .flat_map(|path| {
            ["ledger.json", "owner.json"].map(|name| {
                let path = h.record_directory(path).join(name);
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
        })
        .collect();
    for path in [&h.root, &target] {
        let record = h.record_directory(path);
        let owner = json(&fs::read(record.join("owner.json")).unwrap()).unwrap();
        let directory = DirectoryLease::acquire(&record, true, None)
            .unwrap()
            .unwrap();
        let metadata = Lease::acquire(
            &record.join("metadata.lease"),
            owner["admission_id"].as_str(),
            false,
            true,
            None,
        )
        .unwrap()
        .unwrap();
        for extra in [vec![], vec!["--path", target.to_str().unwrap()]] {
            let (code, observed, error) = h.auto("status", &extra);
            assert_eq!(
                code,
                0,
                "status during {} exclusion: {observed} {error}",
                path.display()
            );
            let row = attention_row(&observed, &target);
            assert_eq!(row["work_state"], "active");
            assert_eq!(row["attention_required"], true);
            assert_eq!(row["next_action"], "complete-work-and-finish");
            for (original, bytes) in &originals {
                assert_eq!(fs::read(original).unwrap(), *bytes);
            }
        }
        directory.stable().unwrap();
        metadata.stable().unwrap();
    }
    assert_eq!(
        fs::read(target.join("payload")).unwrap(),
        b"literal host input; no dependency inference\n"
    );
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn status_does_not_initialize_namespace_or_unknown_address() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let namespace = h.root.join(".chrono-harness/state/automatic-cleanup");
    let unknown = h.parent.join("unknown-status-address");
    let (code, observed, error) = h.auto("status", &["--path", unknown.to_str().unwrap()]);
    assert_eq!(code, 0, "{observed} {error}");
    assert_eq!(
        attention_row(&observed, &unknown)["work_state"],
        "unregistered"
    );
    assert!(
        !namespace.exists(),
        "a status query must not initialize cleanup state"
    );
    let target = h.parent.join("registered-status-address");
    assert_eq!(
        h.invoke("feature", "registered-status-address", &target).0,
        0
    );
    let (code, observed, error) = h.auto("status", &["--path", unknown.to_str().unwrap()]);
    assert_eq!(code, 0, "{observed} {error}");
    assert!(!h.record_directory(&unknown).exists());
    assert!(!h.record_directory(&h.root).exists());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn status_reports_damaged_invoking_metadata_and_rejects_namespace_identity_changes() {
    if !chrono_git_boundary::isolated_git() {
        return;
    }
    let h = Host::new("payload");
    h.kernel_cleanup();
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{maintained} {error}");
    let ledger_path = h.ledger_path(&h.root);
    let original = fs::read(&ledger_path).unwrap();
    fs::write(&ledger_path, b"damaged invoking ledger").unwrap();
    let (code, observed, error) = h.auto("status", &[]);
    assert_eq!(code, 0, "{observed} {error}");
    let row = attention_row(&observed, &h.root);
    assert_eq!(row["work_state"], "unknown");
    assert_eq!(row["attention_required"], true);
    assert!(row["error"].is_string());
    assert_eq!(fs::read(&ledger_path).unwrap(), b"damaged invoking ledger");
    fs::write(&ledger_path, original).unwrap();
    let namespace = h
        .root
        .join(".chrono-harness/state/automatic-cleanup/namespace.json");
    let original = fs::read(&namespace).unwrap();
    let mut changed = json(&original).unwrap();
    changed["common"] = value!(h.parent.join("foreign-repository"));
    let changed = serde_json::to_vec(&changed).unwrap();
    fs::write(&namespace, &changed).unwrap();
    let (code, failed, error) = h.auto("status", &[]);
    assert_eq!(code, 2, "{failed} {error}");
    assert_eq!(failed["status"], "failed");
    assert!(failed["error"].as_str().unwrap().contains("namespace"));
    assert_eq!(fs::read(&namespace).unwrap(), changed);
    fs::write(namespace, original).unwrap();
}
