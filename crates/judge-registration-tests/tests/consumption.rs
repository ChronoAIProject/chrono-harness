use chrono_judge_registration::consumption::{self, Endpoint};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

const UNITS: &str = ".chrono-harness/ci/units.json";
const STARTUP: &str = ".chrono-harness/ci/bootstrap-shared.json";
const CORE: &str = ".chrono-harness/ci/bootstrap-core.json";
const CACHE: &str = ".chrono-harness/ci/bootstrap-cache.json";
const CHECK: &str = ".chrono-harness/ci/check.json";
const FM: &str = ".chrono-harness/FILEMAP.json";
const PROJECTS: &str = ".chrono-harness/projects.json";
const POLICY: &str = ".chrono-harness/git/pinned-macos.json";

fn host() -> Endpoint {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let read = |path: &str| -> Value {
        serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap()
    };
    Endpoint {
        adoption: read(CHECK)["policy"].get("causal_consumption").cloned(),
        filemap: read(FM),
        projects: read(PROJECTS),
        config: read(POLICY),
        config_paths: BTreeSet::from([POLICY.into()]),
        documents: [UNITS, STARTUP, CORE, CACHE, CHECK]
            .into_iter()
            .map(|p| (p.into(), read(p)))
            .collect(),
        unavailable: BTreeMap::new(),
    }
}
fn paths(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|s| s.to_string()).collect()
}
fn action<'a>(s: &'a mut Endpoint, op: &str) -> &'a mut Value {
    s.projects
        .as_object_mut()
        .unwrap()
        .iter_mut()
        .filter(|(k, _)| k.as_str() == "projects" || k.as_str() == "scripts")
        .flat_map(|(_, v)| v.as_array_mut().unwrap())
        .flat_map(|r| r["actions"].as_object_mut().unwrap().values_mut())
        .find(|a| a["operation"] == op)
        .unwrap()
}

#[test]
fn real_registered_startup_chain_requires_original_consumer_evidence() {
    let s = host();
    let result = consumption::validate_delta(None, &s, &BTreeSet::new(), &BTreeSet::new()).unwrap();
    assert_eq!(result.checked, paths(&["runner-startup"]));
    assert_eq!(result.required_tests, paths(&["test:host-bootstrap-tests"]));
    let mut legacy = host();
    legacy.adoption = None;
    assert!(
        consumption::validate_delta(
            Some(&legacy),
            &legacy,
            &paths(&["unrelated.md"]),
            &BTreeSet::new()
        )
        .unwrap()
        .checked
        .is_empty()
    );
}

#[test]
fn affected_chain_rejects_missing_dangling_and_misbound_truths() {
    for defect in [
        "need",
        "use",
        "caller",
        "producer",
        "unit",
        "profiles",
        "output",
        "missing-output",
        "source",
        "duplicate-output",
        "check",
        "verify",
        "evidence-method",
        "evidence-plan",
        "evidence-output",
        "missing-doc",
        "unregistered-doc",
    ] {
        let old = host();
        let mut s = host();
        match defect {
            "need" => {
                s.documents.get_mut(UNITS).unwrap()["job_gating"]["startup"]["consumers"]["unit_runner"].as_object_mut().unwrap().remove("need");
            }
            "use" => {
                s.documents.get_mut(UNITS).unwrap()["job_gating"]["startup"]["consumers"]["unit_runner"]
                    ["output_use"] = json!("")
            }
            "caller" => {
                action(&mut s, "host.bootstrap-shared-install-unit_runner")["argv"][5] =
                    json!("unit_ci")
            }
            "producer" => {
                action(&mut s, "host.bootstrap-shared-produce")["operation"] =
                    json!("retired.produce")
            }
            "unit" => {
                s.documents.get_mut(UNITS).unwrap()["units"]
                    .as_object_mut()
                    .unwrap()
                    .remove("runner");
            }
            "profiles" => {
                s.documents.get_mut(STARTUP).unwrap()["consumers"]["unit_runner"] = json!([])
            }
            "output" => {
                s.documents.get_mut(CORE).unwrap()["install"][0]["destination"] =
                    json!("unrelated/tool")
            }
            "missing-output" => {
                s.documents.get_mut(CORE).unwrap()["install"]
                    .as_array_mut()
                    .unwrap()
                    .remove(1);
            }
            "source" => {
                s.documents.get_mut(CORE).unwrap()["install"][0]["source"] = json!("unrelated/tool")
            }
            "duplicate-output" => {
                let row = s.documents[CORE]["install"][0].clone();
                s.documents.get_mut(CORE).unwrap()["install"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
            "check" => s.config["canonical_check"]["argv"] = json!(["some-other-runner", "check"]),
            "verify" => {
                s.documents.get_mut(UNITS).unwrap()["units"]["runner"]["bootstrap"][5] =
                    json!("absent-profile")
            }
            "evidence-method" => {
                action(&mut s, "host.bootstrap-real-consumer")["operation"] =
                    json!("retired.evidence")
            }
            "evidence-plan" => {
                s.filemap["execution_plans"]["test:host-bootstrap-tests"]["operations"] =
                    json!(["host.bootstrap-tests"])
            }
            "evidence-output" => {
                s.filemap["execution_scheduling"]["claims"]["host.bootstrap-real-consumer"]["outputs"] =
                    json!([])
            }
            "missing-doc" => {
                s.documents.remove(CORE);
            }
            "unregistered-doc" => s.filemap["files"]
                .as_array_mut()
                .unwrap()
                .retain(|f| f["path"] != CORE),
            _ => unreachable!(),
        }
        let error = consumption::validate_delta(Some(&old), &s, &paths(&[CORE]), &BTreeSet::new())
            .unwrap_err();
        assert!(error.starts_with("E_CONSUMPTION:"), "{defect}: {error}");
    }
}

#[test]
fn both_endpoints_select_removed_inputs_and_old_owners_without_global_rejudgment() {
    let mut old = host();
    let mut new = host();
    // A historical malformed need remains scoped until its chain is affected.
    new.documents.get_mut(UNITS).unwrap()["job_gating"]["startup"]["consumers"]["unit_runner"]["need"] =
        json!("");
    let unaffected = consumption::validate_delta(
        Some(&old),
        &new,
        &paths(&["unrelated.md"]),
        &BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(unaffected.historical, paths(&["runner-startup"]));
    assert!(
        consumption::validate_delta(
            Some(&old),
            &new,
            &BTreeSet::new(),
            &paths(&["project:runner"])
        )
        .is_err()
    );
    // Removed old profile input must select even after the new declaration no longer references it.
    old.documents.get_mut(STARTUP).unwrap()["profiles"]["core"]["config"] =
        json!("old-profile.json");
    assert!(
        consumption::validate_delta(
            Some(&old),
            &new,
            &paths(&["old-profile.json"]),
            &BTreeSet::new()
        )
        .is_err()
    );
    // Old operation owners select a defect even after the candidate moves that action.
    action(&mut new, "build.runner")["operation"] = json!("retired.runner");
    assert!(
        consumption::validate_delta(
            Some(&old),
            &new,
            &BTreeSet::new(),
            &paths(&["project:runner"])
        )
        .is_err()
    );
}

#[test]
fn retired_contract_and_unknown_versions_fail_while_legacy_consumers_keep_meaning() {
    let old = host();
    let mut new = host();
    new.adoption = None;
    assert!(
        consumption::validate_delta(Some(&old), &new, &BTreeSet::new(), &BTreeSet::new())
            .unwrap_err()
            .contains("E_CONSUMPTION_RETIRED")
    );
    for value in [
        json!(null),
        json!({"schema":"chrono-internal-causal-consumption/v2","shared_startup":[]}),
        json!({"schema":"chrono-internal-causal-consumption/v1","shared_startup":[]}),
    ] {
        new.adoption = Some(value);
        assert!(consumption::contract(new.adoption.as_ref()).is_err());
    }
    let mut new = host();
    new.documents.get_mut(UNITS).unwrap()["job_gating"]["startup"]["consumers"]["unit_ci"]["need"] =
        json!("");
    consumption::validate_delta(Some(&old), &new, &paths(&[UNITS]), &BTreeSet::new()).unwrap();
}

#[test]
fn scoped_adapter_reads_fixed_native_git_endpoints_and_retains_original_failure() {
    use std::process::Command;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "fixture@example.invalid"]);
    git(&["config", "user.name", "Consumption fixture"]);
    for path in [
        FM,
        PROJECTS,
        POLICY,
        UNITS,
        STARTUP,
        CORE,
        CACHE,
        CHECK,
        ".chrono-harness/judges.json",
        ".chrono-harness/workflow.json",
    ] {
        let target = root.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source.join(path), target).unwrap();
    }
    git(&["add", "."]);
    git(&["commit", "-qm", "valid startup endpoint"]);
    let base = git(&["rev-parse", "HEAD"]);
    let reader = chrono_harness::facts::Reader::legacy();
    let check = |candidate: &str, changed: &BTreeSet<String>| {
        consumption::check_scoped(
            &reader,
            root,
            Some(&base),
            candidate,
            FM,
            PROJECTS,
            Some(POLICY),
            CHECK,
            changed,
            &BTreeSet::new(),
        )
    };
    assert!(
        check(&base, &paths(&[CORE]))
            .unwrap()
            .required_tests
            .contains("test:host-bootstrap-tests")
    );
    let mut core: Value = serde_json::from_slice(&fs::read(root.join(CORE)).unwrap()).unwrap();
    core["install"][0]["source"] = json!("wrong/producer");
    fs::write(root.join(CORE), serde_json::to_vec(&core).unwrap()).unwrap();
    // Worktree bytes do not replace the fixed candidate's declared inputs.
    check(&base, &paths(&[CORE])).unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "misbound output endpoint"]);
    let candidate = git(&["rev-parse", "HEAD"]);
    let error = check(&candidate, &paths(&[CORE])).unwrap_err();
    assert!(
        error.contains("wrong/producer") && error.contains("declared producer claim"),
        "{error}"
    );
    assert!(
        check(&candidate, &paths(&["unrelated.md"]))
            .unwrap()
            .checked
            .is_empty()
    );
}
