//! Behavioral scheduling fixtures use actual Git objects and the fixed native command.
use super::*;

const PROVIDER: &str = ".chrono-harness/ci/units.json";

fn native_job_detail(status: &str, conclusion: Value) -> Value {
    json!({"id":91,"run_id":101,"run_attempt":2,"head_sha":"a".repeat(40),
        "name":"chrono / alpha","status":status,"conclusion":conclusion})
}

#[test]
fn stale_running_job_list_uses_one_bound_terminal_detail() {
    for status in ["queued", "in_progress", "waiting", "pending", "requested"] {
        let listed = native_job_detail(status, Value::Null);
        let completed = native_job_detail("completed", json!("success"));
        let mut calls = Vec::new();
        let actual = chrono_ci::gating::completed_native_job(&listed, |id| {
            calls.push(id);
            Ok(completed.clone())
        })
        .unwrap();
        assert_eq!(actual, completed);
        assert_eq!(calls, vec![91]);
        assert_eq!(listed["status"], status);
    }
}

#[test]
fn terminal_job_lists_keep_their_current_success_or_failure() {
    for conclusion in ["success", "failure", "cancelled", "skipped"] {
        let listed = native_job_detail("completed", json!(conclusion));
        let result = chrono_ci::gating::completed_native_job(&listed, |_| {
            panic!("terminal list must not be retried")
        });
        assert_eq!(result.is_ok(), conclusion == "success");
    }
    for listed in [
        native_job_detail("unknown", Value::Null),
        native_job_detail("in_progress", json!("failure")),
        native_job_detail("in_progress", json!("success")),
        {
            let mut job = native_job_detail("in_progress", Value::Null);
            job.as_object_mut().unwrap().remove("conclusion");
            job
        },
        {
            let mut job = native_job_detail("in_progress", Value::Null);
            job["id"] = json!(0);
            job
        },
    ] {
        assert!(
            chrono_ci::gating::completed_native_job(&listed, |_| panic!(
                "malformed list must fail before detail"
            ))
            .is_err()
        );
    }
}

#[test]
fn nonterminal_or_failed_detail_never_becomes_a_pass() {
    for (status, conclusion) in [
        ("in_progress", Value::Null),
        ("completed", json!("failure")),
        ("completed", json!("cancelled")),
        ("completed", json!("skipped")),
    ] {
        let mut calls = 0;
        let result = chrono_ci::gating::completed_native_job(
            &native_job_detail("in_progress", Value::Null),
            |id| {
                assert_eq!(id, 91);
                calls += 1;
                Ok(native_job_detail(status, conclusion.clone()))
            },
        );
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }
}

#[test]
fn job_detail_must_keep_the_selected_identity_and_original_error() {
    for field in ["id", "run_id", "run_attempt", "head_sha", "name"] {
        let mut detail = native_job_detail("completed", json!("success"));
        detail[field] = if ["id", "run_id", "run_attempt"].contains(&field) {
            json!(999)
        } else {
            json!("changed")
        };
        let result = chrono_ci::gating::completed_native_job(
            &native_job_detail("in_progress", Value::Null),
            |_| Ok(detail.clone()),
        );
        assert!(result.is_err(), "{field}");
    }
    assert_eq!(
        chrono_ci::gating::completed_native_job(
            &native_job_detail("in_progress", Value::Null),
            |_| Err("original transport failure".into())
        )
        .unwrap_err(),
        "original transport failure"
    );
}

fn gating() -> Value {
    json!({"schema":"chrono-job-gating/v1","detector":{"runs_on":"ubuntu-24.04","timeout_minutes":10,"bootstrap":["host-tools-only"],"sparse_checkout":[".chrono-harness/", ".github/workflows/collection.yml"]}})
}

fn migrate_provider(root: &Path, next: &Value) {
    let previous = ".chrono-harness/state/previous-units.json";
    fs::create_dir_all(root.join(previous).parent().unwrap()).unwrap();
    fs::copy(root.join(PROVIDER), root.join(previous)).unwrap();
    json_file(root, PROVIDER, next);
    chrono_ci::migrate::migrate(root, previous, PROVIDER, PROVIDER).unwrap();
}

#[test]
fn parent_generation_uses_only_current_projection_ownership() {
    let h = ShortHost::new();
    let mut next: Value =
        serde_json::from_slice(&fs::read(h.root.join(PROVIDER)).unwrap()).unwrap();
    next["job_gating"] = gating();
    migrate_provider(&h.root, &next);
    let parent = fs::read(h.root.join(".github/workflows/collection.yml")).unwrap();
    for path in [".github/workflows/alpha.yml", ".github/workflows/beta.yml"] {
        fs::write(h.root.join(path), b"host-owned\n").unwrap();
    }

    assert!(!generate(&h.root, PROVIDER, true).unwrap());
    assert!(!generate(&h.root, PROVIDER, false).unwrap());
    assert_eq!(
        fs::read(h.root.join(".github/workflows/collection.yml")).unwrap(),
        parent
    );
    for path in [".github/workflows/alpha.yml", ".github/workflows/beta.yml"] {
        assert_eq!(fs::read(h.root.join(path)).unwrap(), b"host-owned\n");
    }
}

#[test]
fn parent_projection_preserves_unit_settings() {
    let h = ShortHost::new();
    let mut c: Value = serde_json::from_slice(&fs::read(h.root.join(PROVIDER)).unwrap()).unwrap();
    c["job_gating"] = gating();
    migrate_provider(&h.root, &c);
    assert!(!generate(&h.root, PROVIDER, true).unwrap());
    let text = fs::read_to_string(h.root.join(".github/workflows/collection.yml")).unwrap();
    assert!(text.contains("  detect:"));
    assert!(text.contains("  unit_alpha:"));
    assert!(text.contains("  unit_beta:"));
    assert!(text.contains("  aggregate:"));
    assert!(text.contains("if: ${{ always() }}"));
    assert!(text.contains("needs.detect.outputs.unit_alpha == 'true'"));
    assert!(text.contains("'check' '--collect'"));
    assert!(text.contains("'explicit-a.sh'"));
    assert!(text.contains("macos-14"));
    let workflow: Value = serde_yaml_ng::from_str(&text).unwrap();
    assert!(workflow["on"].get("workflow_dispatch").is_none());
    assert!(!text.contains("inputs.candidate"));
    assert_eq!(
        workflow["concurrency"]["group"],
        "chrono-ci:.github/workflows/collection.yml:${{ github.event_name }}:${{ github.event.pull_request.number || github.run_id }}"
    );
    assert_eq!(
        workflow["concurrency"]["cancel-in-progress"],
        "${{ github.event_name == 'pull_request' }}"
    );
    assert!(
        workflow["jobs"]
            .as_object()
            .unwrap()
            .values()
            .all(|job| job.get("concurrency").is_none())
    );
    assert!(!h.root.join(".github/workflows/alpha.yml").exists());
    assert!(!h.root.join(".github/workflows/beta.yml").exists());
}

pub(crate) fn install_parent_provider(root: &Path) {
    let program = root.join(".chrono-harness/bin/parent-gh");
    assert_eq!(
        fs::symlink_metadata(&program).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    let copied = Command::new("/bin/cp")
        .arg(env!("CARGO_BIN_EXE_chrono-ci-test-transport"))
        .arg(&program)
        .output()
        .unwrap();
    assert!(copied.status.success(), "parent provider copy: {copied:?}");
    let ready = Command::new(&program)
        .current_dir(root)
        .env_clear()
        .arg("--version")
        .output()
        .unwrap();
    assert!(
        ready.status.success(),
        "parent provider readiness: {ready:?}"
    );
    assert_eq!(ready.stdout, b"chrono-ci-test-provider/v1\n");
    assert!(
        ready.stderr.is_empty(),
        "parent provider readiness: {ready:?}"
    );
    assert!(!root.join(".chrono-harness/state/parent-calls").exists());
}

pub(crate) struct GatedHost {
    pub(crate) host: ShortHost,
    pub(crate) payload: Value,
}
impl std::ops::Deref for GatedHost {
    type Target = ShortHost;
    fn deref(&self) -> &Self::Target {
        &self.host
    }
}
impl std::ops::DerefMut for GatedHost {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.host
    }
}
impl GatedHost {
    pub(crate) fn new() -> Self {
        let mut h = ShortHost::new();
        install_parent_provider(&h.root);
        let mut cfg: Value =
            serde_json::from_slice(&fs::read(h.root.join(".chrono-harness/config.json")).unwrap())
                .unwrap();
        cfg["environment"]["inherit"]
            .as_array_mut()
            .unwrap()
            .push(json!("GH_TOKEN"));
        cfg["environment"]["credential_environment"] = json!(["GH_TOKEN"]);
        json_file(&h.root, ".chrono-harness/config.json", &cfg);
        let mut provider: Value =
            serde_json::from_slice(&fs::read(h.root.join(PROVIDER)).unwrap()).unwrap();
        provider["gather"]["program"] = json!(".chrono-harness/bin/parent-gh");
        migrate_provider(&h.root, &provider);
        h.revise();
        h.base = h.candidate.clone();
        git(&h.root, &["push", "-q", "origin", "dev"]);
        let payload = json!({"ref":"refs/heads/dev","before":h.base,"after":h.candidate,"created":false,"deleted":false});
        Self { host: h, payload }
    }
    pub(crate) fn change(&mut self, paths: &[&str]) {
        for path in paths {
            fs::write(
                self.host.root.join(path),
                format!("changed after {}", self.host.candidate),
            )
            .unwrap();
        }
        self.host.revise();
        self.payload["after"] = json!(self.host.candidate);
    }
    pub(crate) fn environment(&self, command: &mut Command) {
        json_file(
            &self.host.root,
            ".chrono-harness/state/event.json",
            &self.payload,
        );
        command
            .current_dir(&self.host.root)
            .env(prepared::SOURCE, "ci")
            .env(
                "GITHUB_EVENT_NAME",
                if self.payload.get("pull_request").is_some() {
                    "pull_request"
                } else {
                    "push"
                },
            )
            .env(
                "GITHUB_EVENT_PATH",
                self.host.root.join(".chrono-harness/state/event.json"),
            )
            .env(
                "CHRONO_WORKFLOW_REVISION",
                self.payload["pull_request"]["merge_commit_sha"]
                    .as_str()
                    .unwrap_or(&self.host.candidate),
            )
            .env("GITHUB_REPOSITORY", "owner/host")
            .env("GITHUB_RUN_ID", "101")
            .env("GITHUB_RUN_ATTEMPT", "2")
            .env("GH_TOKEN", "fixture-parent-secret");
    }
    fn detect(&self) -> std::process::Output {
        let output = self.host.root.join(".chrono-harness/state/detector-output");
        let mut c = Command::new(self.host.root.join(".chrono-harness/bin/chrono-ci"));
        c.args([
            "detect",
            "--host-root",
            ".",
            "--config",
            PROVIDER,
            "--github-output",
            output.to_str().unwrap(),
        ]);
        self.environment(&mut c);
        c.env("GITHUB_JOB", "detect");
        c.output().unwrap()
    }
    pub(crate) fn detection(&self) -> Value {
        let out = self.detect();
        assert!(
            out.status.success(),
            "{} {}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["detection"].clone()
    }
    pub(crate) fn command(&self, args: &[&str], d: &Value, needs: &Value) -> Command {
        let mut c = self.host.command(args);
        self.environment(&mut c);
        let job = args
            .windows(2)
            .find(|p| p[0] == "--unit")
            .map(|p| chrono_ci::gating::job_id(p[1]))
            .unwrap_or("aggregate".into());
        c.env("GITHUB_JOB", job);
        c.env("CHRONO_CI_DETECTION", serde_json::to_string(d).unwrap())
            .env("CHRONO_CI_NEEDS", serde_json::to_string(needs).unwrap());
        c
    }
    pub(crate) fn mock(&self, d: &Value, mode: &str) {
        json_file(
            &self.host.root,
            ".chrono-harness/state/parent-data.json",
            &json!({"detection":d, "mode":mode}),
        );
    }
    pub(crate) fn produce(&self, unit: &str) {
        let output = self
            .host
            .root
            .join(format!(".chrono-harness/state/production-output-{unit}"));
        let provider: Value =
            serde_json::from_slice(&fs::read(self.host.root.join(PROVIDER)).unwrap()).unwrap();
        let context: Value = serde_json::from_slice(
            &fs::read(
                self.host
                    .root
                    .join(provider["units"][unit]["context_path"].as_str().unwrap()),
            )
            .unwrap(),
        )
        .unwrap();
        let attempt = context["native_run"]["run_attempt"]
            .as_u64()
            .unwrap()
            .to_string();
        let mut command = Command::new(self.host.root.join(".chrono-harness/bin/chrono-ci"));
        command.args([
            "production",
            "--host-root",
            ".",
            "--config",
            PROVIDER,
            "--unit",
            unit,
            "--github-output",
            output.to_str().unwrap(),
        ]);
        self.environment(&mut command);
        command
            .env("GITHUB_JOB", chrono_ci::gating::job_id(unit))
            .env("GITHUB_RUN_ATTEMPT", attempt);
        let out = command.output().unwrap();
        assert!(
            out.status.success(),
            "{} {}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        fs::write(
            self.host
                .root
                .join(format!(".chrono-harness/state/production-{unit}")),
            out.stdout,
        )
        .unwrap();
    }
    pub(crate) fn units(&self, d: &Value) {
        for unit in d["required_units"].as_array().unwrap() {
            let unit = unit.as_str().unwrap();
            let command = self.command(&["check", "--unit", unit], d, &Value::Null);
            let argv: Vec<String> = std::iter::once(command.get_program())
                .chain(command.get_args())
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            let environment: Value = command
                .get_envs()
                .map(|(key, value)| {
                    (
                        key.to_string_lossy().into_owned(),
                        value
                            .map(|v| json!(v.to_string_lossy()))
                            .unwrap_or(Value::Null),
                    )
                })
                .collect::<serde_json::Map<String, Value>>()
                .into();
            let out = {
                let mut command = command;
                command.output().unwrap()
            };
            retain_command_result(
                &self.host.root,
                &argv,
                &json!({"unit":unit,"detection":d,"needs":Value::Null,"environment":environment}),
                &out,
            );
            assert!(
                out.status.success(),
                "{} {}",
                String::from_utf8_lossy(&out.stderr),
                String::from_utf8_lossy(&out.stdout)
            );
            self.produce(unit);
            copy_artifacts(
                &self.host.root.join(format!(".chrono-harness/state/{unit}")),
                &self
                    .host
                    .root
                    .join(format!(".chrono-harness/state/mock-artifacts/{unit}")),
            );
        }
    }
}
pub(crate) fn needs(h: &GatedHost, d: &Value) -> Value {
    let mut outputs = json!({"detection":serde_json::to_string(d).unwrap()});
    let mut value = json!({"detect":{"result":"success"}});
    let provider: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(PROVIDER)).unwrap()).unwrap();
    for unit in provider["units"].as_object().unwrap().keys() {
        let required = d["required_units"]
            .as_array()
            .unwrap()
            .contains(&json!(unit));
        outputs[chrono_ci::gating::job_id(unit)] = json!(if required { "true" } else { "false" });
        value[chrono_ci::gating::job_id(unit)] =
            json!({"result":if required {"success"} else {"skipped"}});
        if required {
            let output = fs::read_to_string(
                h.host
                    .root
                    .join(format!(".chrono-harness/state/production-{unit}")),
            );
            if let Ok(output) = output {
                value[chrono_ci::gating::job_id(unit)]["outputs"]["production"] =
                    json!(output.trim());
            }
        }
    }
    value["detect"]["outputs"] = outputs;
    value
}

#[test]
fn detector_selects_isolated_unrelated_and_empty_git_delta_without_business_execution() {
    for (paths, expected) in [
        (vec!["a.txt"], json!(["alpha"])),
        (vec!["a.txt", "b.txt"], json!(["alpha", "beta"])),
        (vec![], json!([])),
    ] {
        let mut h = GatedHost::new();
        if !paths.is_empty() {
            h.change(&paths);
        }
        let d = h.detection();
        assert_eq!(d["required_units"], expected);
        assert_eq!(d["changed_paths_count"], paths.len());
        assert!(!h.host.root.join(".chrono-harness/state/calls-a").exists());
        assert!(!h.host.root.join(".chrono-harness/state/calls-b").exists());
    }
}

#[test]
fn exact_parent_gather_and_final_judge_admit_real_selected_reports_and_empty_delta() {
    for paths in [vec!["a.txt"], vec!["a.txt", "b.txt"], vec![]] {
        let mut h = GatedHost::new();
        if !paths.is_empty() {
            h.change(&paths);
        }
        let d = h.detection();
        h.units(&d);
        h.mock(&d, "skipped-jobs-omitted");
        let n = needs(&h, &d);
        let counts: Vec<_> = ["a", "b"]
            .iter()
            .map(|id| {
                fs::read(
                    h.host
                        .root
                        .join(format!(".chrono-harness/state/calls-{id}")),
                )
                .ok()
            })
            .collect();
        for _ in 0..2 {
            let out = h.command(&["check", "--collect"], &d, &n).output().unwrap();
            if !out.status.success() {
                // Keep the actual nested producer receipts, including child exit,
                // stdout and stderr, before the fixture's temporary host is dropped.
                let evidence = tempfile::Builder::new()
                    .prefix("chrono-gating-failure-")
                    .tempdir()
                    .unwrap()
                    .keep();
                fs::write(evidence.join("stdout"), &out.stdout).unwrap();
                fs::write(evidence.join("stderr"), &out.stderr).unwrap();
                json_file(
                    &evidence,
                    "process.json",
                    &json!({"exit_code":out.status.code(),
                    "status":out.status.to_string(),"detection":d,"needs":n}),
                );
                copy_artifacts(
                    &h.host.root.join(".chrono-harness/state"),
                    &evidence.join("state"),
                );
                panic!(
                    "collection failed: status={} stdout={} stderr={}; original process evidence retained at {}",
                    out.status,
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr),
                    evidence.display()
                );
            }
            assert!(
                out.status.success(),
                "{} {}",
                String::from_utf8_lossy(&out.stderr),
                String::from_utf8_lossy(&out.stdout)
            );
            let report = published_report(&h.host.root, None, &out);
            assert_eq!(report["response"]["evidence"]["executed"], json!([]));
            assert_eq!(
                report["response"]["evidence"]["reports"]
                    .as_array()
                    .unwrap()
                    .len(),
                d["required_units"].as_array().unwrap().len()
            );
            let gather: Value = serde_json::from_slice(
                &fs::read(
                    h.host
                        .root
                        .join(".chrono-harness/state/collection/gather.json"),
                )
                .unwrap(),
            )
            .unwrap();
            let calls = gather["processes"].as_array().unwrap();
            let requests =
                |endpoint: &str| calls.iter().filter(|p| p["argv"][2] == endpoint).count();
            let selected = !d["required_units"].as_array().unwrap().is_empty();
            assert_eq!(
                requests("repos/owner/host/actions/runs/101"),
                if selected { 2 } else { 1 }
            );
            assert_eq!(
                requests("repos/owner/host/actions/runs/101/jobs?filter=all&per_page=100"),
                1
            );
            assert_eq!(
                requests("repos/owner/host/actions/runs/101/artifacts?per_page=100"),
                usize::from(selected)
            );
            assert_eq!(
                calls
                    .iter()
                    .filter(|p| p["argv"][2]
                        .as_str()
                        .is_some_and(|a| a.contains("/contents/")))
                    .count(),
                usize::from(selected)
            );
        }
        for (id, before) in ["a", "b"].iter().zip(counts) {
            assert_eq!(
                fs::read(
                    h.host
                        .root
                        .join(format!(".chrono-harness/state/calls-{id}"))
                )
                .ok(),
                before
            );
        }
        let calls =
            fs::read_to_string(h.host.root.join(".chrono-harness/state/parent-calls")).unwrap();
        assert!(!calls.contains("/actions/workflows/"));
        assert!(calls.contains("/jobs?filter=all"));
        assert!(!calls.contains("fixture-parent-secret"));
        if paths.len() == 1 {
            assert!(!calls.contains("chrono-unit-beta"));
        }
    }
}

#[test]
fn shared_parent_observation_checks_every_artifact_and_final_attempt_before_publication() {
    for mode in [
        "artifact-missing-last",
        "artifact-misbound-last",
        "artifact-ambiguous-last",
        "changed-attempt-after-downloads",
    ] {
        let mut h = GatedHost::new();
        h.change(&["a.txt", "b.txt"]);
        let d = h.detection();
        h.units(&d);
        h.mock(&d, mode);
        let out = h
            .command(&["check", "--collect"], &d, &needs(&h, &d))
            .output()
            .unwrap();
        assert!(!out.status.success(), "accepted {mode}");
        let report: Value = serde_json::from_slice(
            &fs::read(
                h.host
                    .root
                    .join(".chrono-harness/state/collection/gather.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(report["result"].is_null(), "{mode}");
        assert!(
            !h.host
                .root
                .join(".chrono-harness/state/collection/manifest.json")
                .exists()
        );
        let processes = report["processes"].as_array().unwrap();
        let downloads: Vec<_> = processes.iter().filter(|p| p["argv"][1] == "run").collect();
        assert_eq!(
            downloads.len(),
            if mode == "changed-attempt-after-downloads" {
                2
            } else {
                1
            },
            "{mode}: {report}"
        );
        assert!(
            report["error"].as_str().unwrap().contains(
                if mode == "changed-attempt-after-downloads" {
                    "parent run/repository/event/source/attempt mismatch"
                } else {
                    "binding differs"
                }
            ),
            "{mode}: {report}"
        );
        for download in downloads {
            let argv = download["argv"].as_array().unwrap();
            let index = argv.iter().position(|a| a == "--dir").unwrap();
            let directory = h.host.root.join(argv[index + 1].as_str().unwrap());
            let name = argv[argv.iter().position(|a| a == "--name").unwrap() + 1]
                .as_str()
                .unwrap();
            let unit = name
                .strip_prefix("chrono-unit-")
                .unwrap()
                .split('-')
                .next()
                .unwrap();
            assert_eq!(
                fs::read(directory.join("check.json")).unwrap(),
                fs::read(h.host.root.join(format!(
                    ".chrono-harness/state/mock-artifacts/{unit}/check.json"
                )))
                .unwrap(),
                "{mode}: original {unit} report changed"
            );
        }
    }
}

#[test]
fn parent_collection_transports_report_references_and_rejects_damaged_originals() {
    let mut h = GatedHost::new();
    h.host.modify(".chrono-harness/ci/check.json", |c| {
        c["policy"]["report_publication"] = json!("retained-reference/v1");
    });
    h.change(&["a.txt", "b.txt"]);
    let d = h.detection();
    h.units(&d);
    h.mock(&d, "skipped-jobs-omitted");
    let n = needs(&h, &d);
    let calls = ["a", "b"].map(|id| {
        fs::read(
            h.host
                .root
                .join(format!(".chrono-harness/state/calls-{id}")),
        )
        .unwrap()
    });
    // Only the transported copy remains; a local original must not hide missing upload bytes.
    let reference: Value = serde_json::from_slice(
        &fs::read(h.host.root.join(".chrono-harness/state/alpha/check.json")).unwrap(),
    )
    .unwrap();
    let original = reference["original"]["path"].as_str().unwrap();
    let downloaded = h
        .host
        .root
        .join(".chrono-harness/state/mock-artifacts/alpha")
        .join(
            original
                .strip_prefix(".chrono-harness/state/alpha/")
                .unwrap(),
        );
    let raw = fs::read(&downloaded).unwrap();
    fs::remove_file(h.host.root.join(original)).unwrap();
    for corrupt in [false, true] {
        if corrupt {
            fs::write(&downloaded, b"damaged uploaded original").unwrap();
        }
        let out = h.command(&["check", "--collect"], &d, &n).output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(if corrupt { 1 } else { 0 }),
            "{out:?}"
        );
        let report = referenced_report(&h.host.root, ".chrono-harness/state/check.json", &out);
        if corrupt {
            assert!(
                report["response"]["results"][0]["cause"]
                    .as_str()
                    .unwrap()
                    .contains("original report digest mismatch")
            );
        } else {
            assert_eq!(report["response"]["evidence"]["executed"], json!([]));
            assert_eq!(
                report["response"]["evidence"]["reports"]
                    .as_array()
                    .unwrap()
                    .len(),
                2
            );
        }
    }
    assert_ne!(raw, b"damaged uploaded original");
    for (id, original_calls) in ["a", "b"].into_iter().zip(calls) {
        assert_eq!(
            fs::read(
                h.host
                    .root
                    .join(format!(".chrono-harness/state/calls-{id}"))
            )
            .unwrap(),
            original_calls
        );
    }
}

#[test]
fn parent_collection_requires_transported_streams_without_local_fallback_or_business_reruns() {
    let mut h = GatedHost::new();
    h.host.modify(".chrono-harness/ci/check.json", |c| {
        c["policy"]["report_publication"] = json!("retained-reference/v2");
    });
    h.change(&["a.txt", "b.txt"]);
    let d = h.detection();
    h.units(&d);
    h.mock(&d, "skipped-jobs-omitted");
    let n = needs(&h, &d);
    let calls = ["a", "b"].map(|id| {
        fs::read(
            h.host
                .root
                .join(format!(".chrono-harness/state/calls-{id}")),
        )
        .unwrap()
    });
    let mut streams = vec![];
    for unit in ["alpha", "beta"] {
        let reference: Value = serde_json::from_slice(
            &fs::read(
                h.host
                    .root
                    .join(format!(".chrono-harness/state/{unit}/check.json")),
            )
            .unwrap(),
        )
        .unwrap();
        let original = reference["original"]["path"].as_str().unwrap();
        let report: Value =
            serde_json::from_slice(&fs::read(h.host.root.join(original)).unwrap()).unwrap();
        assert_eq!(report["schema"], "chrono-check-report/v2");
        let launcher_pid = report["judge"]["launcher_pid"]
            .as_u64()
            .expect("retained process launcher pid");
        assert!(launcher_pid > 0);
        let launch_path = report["judge"]["launch_original"]["path"]
            .as_str()
            .expect("retained launch original path");
        let launch: Value = serde_json::from_slice(
            &fs::read(h.host.root.join(launch_path)).expect("retained launch original"),
        )
        .unwrap();
        assert_eq!(launch["launcher_pid"].as_u64(), Some(launcher_pid));
        // Remove all local originals after upload, so only transported evidence can satisfy collection.
        fs::remove_file(h.host.root.join(original)).unwrap();
        fs::remove_file(h.host.root.join(launch_path)).unwrap();
        for stream in ["stdout", "stderr"] {
            let path = report["judge"][format!("{stream}_original")]["path"]
                .as_str()
                .unwrap();
            let downloaded = h
                .host
                .root
                .join(format!(".chrono-harness/state/mock-artifacts/{unit}"))
                .join(
                    path.strip_prefix(&format!(".chrono-harness/state/{unit}/"))
                        .unwrap(),
                );
            let raw = fs::read(&downloaded).unwrap();
            fs::remove_file(h.host.root.join(path)).unwrap();
            streams.push((stream, downloaded, raw));
        }
    }
    let collect = |expected_exit| {
        let out = h.command(&["check", "--collect"], &d, &n).output().unwrap();
        assert_eq!(out.status.code(), Some(expected_exit), "{out:?}");
        if expected_exit == 0 {
            stream_report(&h.host.root, ".chrono-harness/state/check.json", &out)
        } else {
            let gather: Value = serde_json::from_slice(
                &fs::read(
                    h.host
                        .root
                        .join(".chrono-harness/state/collection/gather.json"),
                )
                .unwrap(),
            )
            .unwrap();
            assert!(gather["result"].is_null());
            gather
        }
    };
    let report = collect(0);
    assert_eq!(report["response"]["evidence"]["executed"], json!([]));
    assert_eq!(
        report["response"]["evidence"]["reports"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for (stream, path, raw) in streams {
        for missing in [false, true] {
            if missing {
                fs::remove_file(&path).unwrap();
            } else {
                fs::write(&path, b"damaged transported stream").unwrap();
            }
            let report = collect(2);
            let cause = report["error"].as_str().unwrap();
            let expected = if missing {
                "No such file".into()
            } else {
                format!("judge {stream} original digest mismatch")
            };
            assert!(cause.contains(&expected), "{cause}");
            fs::write(&path, &raw).unwrap();
        }
    }
    collect(0);
    for (id, original) in ["a", "b"].into_iter().zip(calls) {
        assert_eq!(
            fs::read(
                h.host
                    .root
                    .join(format!(".chrono-harness/state/calls-{id}"))
            )
            .unwrap(),
            original
        );
    }
}

#[test]
fn aggregate_rejects_detection_failure_and_every_unsuccessful_selected_unit() {
    let mut h = GatedHost::new();
    h.change(&["a.txt"]);
    let d = h.detection();
    for status in ["failure", "cancelled", "skipped", "missing"] {
        let mut n = needs(&h, &d);
        if status == "missing" {
            n.as_object_mut().unwrap().remove("unit_alpha");
        } else {
            n["unit_alpha"]["result"] = json!(status);
        }
        let out = h.command(&["check", "--collect"], &d, &n).output().unwrap();
        assert!(!out.status.success(), "accepted {status}");
    }
    for status in ["failure", "cancelled", "skipped"] {
        let mut n = needs(&h, &d);
        n["detect"]["result"] = json!(status);
        let out = h
            .command(&["check", "--collect"], &Value::Null, &n)
            .output()
            .unwrap();
        assert!(!out.status.success(), "accepted detection {status}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("detection did not succeed"));
    }
}

#[test]
fn parent_collection_rejects_missing_artifact_source_run_attempt_and_original_context_mismatch() {
    for mode in [
        "missing-artifact",
        "wrong-source",
        "wrong-run",
        "changed-attempt",
        "job-attempt",
        "missing-job",
        "stale-context",
    ] {
        let mut h = GatedHost::new();
        h.change(&["a.txt"]);
        let d = h.detection();
        h.units(&d);
        h.mock(&d, mode);
        if mode == "stale-context" {
            let path = ".chrono-harness/state/mock-artifacts/alpha/context.json";
            let mut context: Value =
                serde_json::from_slice(&fs::read(h.host.root.join(path)).unwrap()).unwrap();
            context["detection"]["run_attempt"] = json!(1);
            json_file(&h.host.root, path, &context);
        }
        let out = h
            .command(&["check", "--collect"], &d, &needs(&h, &d))
            .output()
            .unwrap();
        assert!(!out.status.success(), "accepted {mode}");
        assert!(
            !h.host
                .root
                .join(".chrono-harness/state/collection/manifest.json")
                .exists()
        );
    }
}

#[test]
fn detector_uses_both_endpoint_edges_assignments_and_deleted_or_renamed_paths() {
    for kind in ["dependency", "assignment", "delete", "rename", "invalid"] {
        let mut h = GatedHost::new();
        let mut fm: Value = serde_json::from_slice(
            &fs::read(h.host.root.join(".chrono-harness/FILEMAP.json")).unwrap(),
        )
        .unwrap();
        match kind {
            "dependency" => {
                // A removed dependency still selects its old consumer.
                let row = fm["files"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["path"] == "a.txt")
                    .unwrap();
                row["edges"] = json!([]);
            }
            "assignment" => {
                let mut c: Value = serde_json::from_slice(
                    &fs::read(h.host.root.join(".chrono-harness/ci/check.json")).unwrap(),
                )
                .unwrap();
                c["policy"]["units"]["alpha"]["tests"] = json!(["test:b"]);
                c["policy"]["units"]["beta"]["tests"] = json!(["test:a"]);
                json_file(&h.host.root, ".chrono-harness/ci/check.json", &c);
            }
            "delete" => {
                fs::remove_file(h.host.root.join("a.txt")).unwrap();
                fm["files"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|r| r["path"] != "a.txt");
            }
            "rename" => {
                git(&h.host.root, &["mv", "a.txt", "renamed opaque.data"]);
                let row = fm["files"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["path"] == "a.txt")
                    .unwrap();
                row["path"] = json!("renamed opaque.data");
            }
            _ => {
                let row = fm["files"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["path"] == "a.txt")
                    .unwrap();
                row["edges"] = json!([{"kind":"test-execution","to":"test:missing"}]);
            }
        }
        json_file(&h.host.root, ".chrono-harness/FILEMAP.json", &fm);
        h.host.revise();
        h.payload["after"] = json!(h.host.candidate);
        let out = h.detect();
        if kind == "invalid" {
            assert!(!out.status.success());
            assert!(String::from_utf8_lossy(&out.stderr).contains("unresolved edge"));
        } else {
            assert!(
                out.status.success(),
                "{kind}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let d: Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(
                d["detection"]["required_units"],
                if kind == "assignment" {
                    json!(["alpha", "beta"])
                } else {
                    json!(["alpha"])
                }
            );
            if kind == "rename" {
                let paths = d["requirements"]["changed_paths"].as_array().unwrap();
                assert!(paths.contains(&json!("a.txt")));
                assert!(paths.contains(&json!("renamed opaque.data")));
            }
        }
    }
}

#[test]
fn large_complete_git_delta_retains_every_path_and_obligation() {
    let mut h = GatedHost::new();
    h.host.base = h.host.candidate.clone();
    h.payload["before"] = json!(h.host.base);
    let mut fm: Value = serde_json::from_slice(
        &fs::read(h.host.root.join(".chrono-harness/FILEMAP.json")).unwrap(),
    )
    .unwrap();
    fs::create_dir(h.host.root.join("opaque")).unwrap();
    for i in 0..3001 {
        let path = format!("opaque/{i:04}.data");
        fs::write(h.host.root.join(&path), "data").unwrap();
        fm["files"].as_array_mut().unwrap().push(json!({"path":path,"owner":"host","surface":"product","cost":"unmeasured","edges":[{"kind":"test-execution","to":"test:a"}]}));
    }
    json_file(&h.host.root, ".chrono-harness/FILEMAP.json", &fm);
    h.host.revise();
    h.payload["after"] = json!(h.host.candidate);
    let out = h.detect();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["detection"]["changed_paths_count"], 3002);
    assert_eq!(
        result["requirements"]["changed_paths"]
            .as_array()
            .unwrap()
            .len(),
        3002
    );
    assert_eq!(result["detection"]["required_units"], json!(["alpha"]));
}

#[test]
fn push_spans_multiple_commits_and_pr_uses_fixed_merge_base_despite_stale_event_base() {
    let mut h = GatedHost::new();
    h.change(&["a.txt"]);
    let previous = h.host.candidate.clone();
    h.change(&["b.txt"]);
    let d = h.detection();
    assert_eq!(d["required_units"], json!(["alpha", "beta"]));
    assert_eq!(d["base"], h.host.base);
    assert_ne!(d["base"], previous);
    let merge = tools::pr_workflow_merge(&h.host.root, &previous, &h.host.candidate);
    h.payload = json!({"pull_request":{"base":{"sha":h.host.base},"head":{"sha":h.host.candidate},"merge_commit_sha":merge}});
    git(&h.host.root, &["checkout", "--detach", &merge]);
    let d = h.detection();
    assert_eq!(d["source"], "pr-workflow-merge-tree");
    assert_eq!(d["base"], previous);
    assert_eq!(d["required_units"], json!(["beta"]));
    assert_eq!(d["candidate"], merge);
    assert_eq!(d["event_head"], h.host.candidate);
    h.units(&d);
    h.mock(&d, "skipped-jobs-omitted");
    let out = h
        .command(&["check", "--collect"], &d, &needs(&h, &d))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut wrong_run_head = d.clone();
    wrong_run_head["event_head"] = json!(merge);
    let rejected = h
        .command(&["check", "--collect"], &wrong_run_head, &needs(&h, &d))
        .output()
        .unwrap();
    assert!(!rejected.status.success());
}

#[test]
fn configured_integration_baseline_is_frozen_once_for_all_dependent_jobs() {
    let mut h = GatedHost::new();
    let mut c: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(PROVIDER)).unwrap()).unwrap();
    c["collection"]["push_baselines"] =
        json!([{"ref_prefix":"refs/heads/integration/","base_ref":"refs/heads/dev"}]);
    json_file(&h.host.root, PROVIDER, &c);
    h.host.revise();
    h.host.base = h.host.candidate.clone();
    h.payload["before"] = json!(h.host.base);
    git(&h.host.root, &["push", "-q", "origin", "dev"]);
    h.change(&["a.txt"]);
    h.payload["before"] = json!(h.host.candidate);
    h.change(&["b.txt"]);
    h.payload["ref"] = json!("refs/heads/integration/example");
    let d = h.detection();
    assert_eq!(d["source"], "push-configured-baseline-ref");
    assert_eq!(d["base"], h.host.base);
    assert_eq!(d["required_units"], json!(["alpha", "beta"]));
    // Move the remote dev tip after detection. Units must keep the observed base.
    git(
        &h.host.root,
        &["push", "-q", "origin", "HEAD:refs/heads/dev"],
    );
    h.units(&d);
    h.mock(&d, "success");
    let out = h
        .command(&["check", "--collect"], &d, &needs(&h, &d))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report = published_report(&h.host.root, None, &out);
    assert_eq!(report["request"]["base"], h.host.base);
}

#[test]
fn shallow_no_checkout_detection_fetches_missing_exact_endpoint_without_host_sources() {
    let mut h = GatedHost::new();
    h.change(&["a.txt"]);
    h.change(&["b.txt"]);
    h.change(&["a.txt"]);
    let root = h.host._dir.path().join("no materialized host");
    fs::create_dir(&root).unwrap();
    let remote = format!("file://{}", h.host.root.display());
    git(
        &root,
        &["clone", "-q", "--no-checkout", "--depth=2", &remote, "."],
    );
    let before = Command::new(fixture_git())
        .current_dir(&root)
        .args(["cat-file", "-e", &format!("{}^{{commit}}", h.host.base)])
        .status()
        .unwrap();
    assert!(!before.success());
    // Only explicitly declared registration and workflow blobs are materialized.
    for path in [
        ".chrono-harness/config.json",
        ".chrono-harness/FILEMAP.json",
        ".chrono-harness/projects.json",
        ".chrono-harness/judges.json",
        ".chrono-harness/workflow.json",
        ".chrono-harness/worktree.json",
        ".chrono-harness/ci/check.json",
        PROVIDER,
        ".github/workflows/collection.yml",
    ] {
        let bytes = Command::new(fixture_git())
            .current_dir(&root)
            .args(["show", &format!("HEAD:{path}")])
            .output()
            .unwrap();
        assert!(bytes.status.success());
        fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        fs::write(root.join(path), bytes.stdout).unwrap();
    }
    install(&root);
    let output = root.join(".chrono-harness/state/out");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    let mut c = Command::new(root.join(".chrono-harness/bin/chrono-ci"));
    c.args([
        "detect",
        "--host-root",
        ".",
        "--config",
        PROVIDER,
        "--github-output",
        output.to_str().unwrap(),
    ]);
    h.environment(&mut c);
    c.current_dir(&root).env("GITHUB_JOB", "detect");
    let out = c.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let d: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(d["detection"]["base"], h.host.base);
    assert_eq!(d["detection"]["required_units"], json!(["alpha", "beta"]));
    assert!(!root.join("a.txt").exists());
    assert!(!root.join("a.case").exists());
    assert!(!root.join("b.txt").exists());
    git(
        &root,
        &["cat-file", "-e", &format!("{}^{{commit}}", h.host.base)],
    );
}

#[test]
fn detector_rejects_missing_endpoint_deletion_parented_initial_and_missing_registry() {
    for mode in ["missing", "deleted", "parented-initial", "registry"] {
        let mut h = GatedHost::new();
        match mode {
            "missing" => h.payload["before"] = json!("f".repeat(40)),
            "deleted" => h.payload["deleted"] = json!(true),
            "parented-initial" => {
                h.payload["before"] = json!("0".repeat(40));
                h.payload["created"] = json!(true);
            }
            _ => {
                fs::remove_file(h.host.root.join(".chrono-harness/projects.json")).unwrap();
                h.host.revise();
                h.payload["after"] = json!(h.host.candidate);
            }
        }
        assert!(!h.detect().status.success(), "accepted {mode}");
    }
    let mut h = GatedHost::new();
    h.payload["ref"] = json!("refs/heads/integration/new");
    h.payload["before"] = json!("0".repeat(40));
    h.payload["created"] = json!(true);
    let d = h.detection();
    assert_eq!(d["base"], h.host.base);
    assert_eq!(d["required_units"], json!([]));
    assert_eq!(d["source"], "branch-creation-baseline-ref");
}

#[test]
fn full_v2_detection_uses_declaration_owners_before_any_context_or_sdk_snapshots() {
    let mut h = GatedHost::new();
    let mut cfg: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(".chrono-harness/config.json")).unwrap())
            .unwrap();
    cfg["execution_units"] = json!({"units":{"alpha":{"tests":["test:a"],"report_path":".chrono-harness/state/alpha/check.json"},"beta":{"tests":["test:b"],"report_path":".chrono-harness/state/beta/check.json"}},"shared_operations":{},"report_path":".chrono-harness/state/collection/check.json","collection_limits":{"manifest_bytes":1048576,"report_bytes":67108864}});
    json_file(&h.host.root, ".chrono-harness/config.json", &cfg);
    let mut wf: Value = serde_json::from_slice(
        &fs::read(h.host.root.join(".chrono-harness/workflow.json")).unwrap(),
    )
    .unwrap();
    wf["stability"] = json!([]);
    wf["integration"]["tests"] = json!(["a", "b"]);
    json_file(&h.host.root, ".chrono-harness/workflow.json", &wf);
    let mut c: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(PROVIDER)).unwrap()).unwrap();
    c["schema"] = json!("chrono-github-units/v2");
    c["collection"]["check_config"] = json!(".chrono-harness/config.json");
    c["collection"]["artifact_directory"] = json!(".chrono-harness/state/collection/");
    c["collection"]["context_path"] = json!(".chrono-harness/state/collection/event.json");
    c["full_contexts"] = json!({"collection":".chrono-harness/state/collection/full.json","units":{"alpha":".chrono-harness/state/alpha/full.json","beta":".chrono-harness/state/beta/full.json"}});
    json_file(&h.host.root, PROVIDER, &c);
    generate(&h.host.root, PROVIDER, false).unwrap();
    h.host.revise();
    h.host.base = h.host.candidate.clone();
    h.payload["before"] = json!(h.host.base);
    h.change(&["a.txt"]);
    let out = h.detect();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["detection"]["required_units"], json!(["alpha"]));
    assert!(
        !h.host
            .root
            .join(".chrono-harness/state/alpha/full.json")
            .exists()
    );
    assert!(!h.host.root.join(".chrono-harness/state/calls-a").exists());
    assert!(
        result["requirements"]["input_scope"]
            .as_str()
            .unwrap()
            .contains("final-effective-input-admission-required")
    );
    // Assignment/dependency removals use both immutable endpoint declarations too.
    let mut cfg: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(".chrono-harness/config.json")).unwrap())
            .unwrap();
    cfg["execution_units"]["units"]["alpha"]["tests"] = json!(["test:b"]);
    cfg["execution_units"]["units"]["beta"]["tests"] = json!(["test:a"]);
    json_file(&h.host.root, ".chrono-harness/config.json", &cfg);
    h.host.revise();
    h.payload["after"] = json!(h.host.candidate);
    let d = h.detection();
    assert_eq!(d["required_units"], json!(["alpha", "beta"]));
}

#[test]
fn scoped_and_full_detection_refuse_decoder_history_before_interpreter_acquisition() {
    for full in [false, true] {
        let mut h = GatedHost::new();
        let mut fm: Value = serde_json::from_slice(
            &fs::read(h.host.root.join(".chrono-harness/FILEMAP.json")).unwrap(),
        )
        .unwrap();
        let mut current = fm.clone();
        fm["schema_version"] = json!(1);
        json_file(&h.host.root, ".chrono-harness/FILEMAP.json", &fm);
        let check_path = ".chrono-harness/ci/check.json";
        let check: Value =
            serde_json::from_slice(&fs::read(h.host.root.join(check_path)).unwrap()).unwrap();
        let mut old_check = check.clone();
        old_check["policy"]["bindings"] = json!({"test:a":["test.a"],"test:b":["test.b"]});
        json_file(&h.host.root, check_path, &old_check);
        h.host.revise();
        h.host.base = h.host.candidate.clone();
        h.payload["before"] = json!(h.host.base);
        json_file(&h.host.root, check_path, &check);
        for id in ["decoder", "decoder-tests"] {
            current["files"].as_array_mut().unwrap().push(json!({"path":format!("{id}.case"),"owner":id,"surface":"product","cost":"unmeasured","edges":[]}));
        }
        json_file(&h.host.root, ".chrono-harness/FILEMAP.json", &current);
        let mut wf: Value = serde_json::from_slice(
            &fs::read(h.host.root.join(".chrono-harness/workflow.json")).unwrap(),
        )
        .unwrap();
        wf["schema_version"] = json!(3);
        wf["historical_profiles"] = json!([{"id":"fixture-decoder","from_versions":{"config":4,"filemap":1,"projects":1,"judges":1,"workflow":2},"to_versions":{"config":4,"filemap":2,"projects":1,"judges":1,"workflow":3},"script":"decoder","test":"decoder-tests","mappings":[],"legacy_records":[],"ambiguities":[]}]);
        json_file(&h.host.root, ".chrono-harness/workflow.json", &wf);
        let mut projects: Value = serde_json::from_slice(
            &fs::read(h.host.root.join(".chrono-harness/projects.json")).unwrap(),
        )
        .unwrap();
        for (id, pair) in [("decoder", "test_script"), ("decoder-tests", "tests_for")] {
            projects["owners"].as_array_mut().unwrap().push(json!(id));
            projects["scripts"].as_array_mut().unwrap().push(json!({"id":id,"path":format!("{id}.case"),pair:if id=="decoder" {"decoder-tests"}else{"decoder"},"actions":{"execute":{"operation":id,"tool":"unit-fixture","argv":["unit-operation",format!("{id}.case")]}}}));
            fs::write(h.host.root.join(format!("{id}.case")), "decoder-launch\n").unwrap();
        }
        json_file(&h.host.root, ".chrono-harness/projects.json", &projects);
        let mut cfg: Value = serde_json::from_slice(
            &fs::read(h.host.root.join(".chrono-harness/config.json")).unwrap(),
        )
        .unwrap();
        cfg["tools"][3]["version_argv"] = json!(["decoder-probe"]);
        if full {
            cfg["execution_units"] = json!({"units":{"alpha":{"tests":["test:a"],"report_path":".chrono-harness/state/alpha/check.json"},"beta":{"tests":["test:b"],"report_path":".chrono-harness/state/beta/check.json"}},"shared_operations":{},"report_path":".chrono-harness/state/collection/check.json","collection_limits":{"manifest_bytes":1048576,"report_bytes":67108864}});
            let mut c: Value =
                serde_json::from_slice(&fs::read(h.host.root.join(PROVIDER)).unwrap()).unwrap();
            c["schema"] = json!("chrono-github-units/v2");
            c["collection"]["check_config"] = json!(".chrono-harness/config.json");
            c["collection"]["artifact_directory"] = json!(".chrono-harness/state/collection/");
            c["collection"]["context_path"] = json!(".chrono-harness/state/collection/event.json");
            c["full_contexts"] = json!({"collection":".chrono-harness/state/collection/full.json","units":{"alpha":".chrono-harness/state/alpha/full.json","beta":".chrono-harness/state/beta/full.json"}});
            json_file(&h.host.root, PROVIDER, &c);
        }
        json_file(&h.host.root, ".chrono-harness/config.json", &cfg);
        generate(&h.host.root, PROVIDER, false).unwrap();
        h.host.revise();
        h.payload["after"] = json!(h.host.candidate);
        let out = h.detect();
        assert!(!out.status.success());
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("E_SCHEDULING_CONVERSION"),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !h.host
                .root
                .join(".chrono-harness/state/decoder-probed")
                .exists()
        );
        assert!(
            !h.host
                .root
                .join(".chrono-harness/state/decoder-launched")
                .exists()
        );
    }
}

#[test]
fn fresh_short_host_init_selects_parent_topology_and_retains_host_settings() {
    let h = ShortHost::new();
    let source = fs::read(h.root.join(PROVIDER)).unwrap();
    let mut incoming: Value = serde_json::from_slice(&source).unwrap();
    incoming["units"]["alpha"]["runs_on"] = json!("host-custom");
    incoming["units"]["alpha"]["timeout_minutes"] = json!(37);
    fs::remove_file(h.root.join(PROVIDER)).unwrap();
    for name in ["collection.yml"] {
        fs::remove_file(h.root.join(format!(".github/workflows/{name}"))).unwrap();
    }
    let c = serde_json::from_value(incoming).unwrap();
    assert!(chrono_ci::units::init(&h.root, c).unwrap());
    let adopted: Value = serde_json::from_slice(&fs::read(h.root.join(PROVIDER)).unwrap()).unwrap();
    assert_eq!(adopted["job_gating"]["schema"], "chrono-job-gating/v1");
    assert_eq!(adopted["units"]["alpha"]["runs_on"], "host-custom");
    assert_eq!(adopted["units"]["alpha"]["timeout_minutes"], 37);
    let projection = fs::read_to_string(h.root.join(".github/workflows/collection.yml")).unwrap();
    assert!(projection.contains("    needs: detect\n"));
    assert!(projection.contains("    if: ${{ always() }}\n"));
    assert!(!h.root.join(".github/workflows/alpha.yml").exists());
    assert!(!h.root.join(".github/workflows/beta.yml").exists());
    assert!(!generate(&h.root, PROVIDER, true).unwrap());
}

#[test]
fn init_regeneration_and_explicit_migration_keep_customization_and_refuse_host_edits() {
    for conflict in ["none", "target"] {
        let h = ShortHost::new();
        let mut c: Value =
            serde_json::from_slice(&fs::read(h.root.join(PROVIDER)).unwrap()).unwrap();
        c["job_gating"] = gating();
        migrate_provider(&h.root, &c);
        fs::write(h.root.join(".github/workflows/host.yml"), "host-owned\n").unwrap();
        if conflict == "target" {
            fs::write(
                h.root.join(".github/workflows/collection.yml"),
                "host edit\n",
            )
            .unwrap();
        }
        let incoming = h.root.join("incoming.json");
        fs::write(&incoming, serde_json::to_vec(&config_value()).unwrap()).unwrap();
        let source = fs::read(h.root.join(PROVIDER)).unwrap();
        let result = init(&h.root, &incoming);
        if conflict != "none" {
            assert!(result.is_err());
            assert!(!h.root.join(".github/workflows/alpha.yml").exists());
            assert_eq!(
                fs::read_to_string(h.root.join(".github/workflows/collection.yml")).unwrap(),
                "host edit\n"
            );
        } else {
            assert!(!result.unwrap());
            assert!(!init(&h.root, &incoming).unwrap());
            assert!(!generate(&h.root, PROVIDER, true).unwrap());
            let mut custom = c.clone();
            custom["units"]["alpha"]["runs_on"] = json!("host-custom");
            custom["units"]["alpha"]["bootstrap"] = json!(["my-tool", "literal space"]);
            custom["units"]["alpha"]["timeout_minutes"] = json!(37);
            json_file(&h.root, PROVIDER, &custom);
            generate(&h.root, PROVIDER, false).unwrap();
            let text = fs::read_to_string(h.root.join(".github/workflows/collection.yml")).unwrap();
            assert!(text.contains("host-custom"));
            assert!(text.contains("'my-tool' 'literal space'"));
            assert!(text.contains("timeout-minutes: 37"));
            json_file(&h.root, PROVIDER, &c);
            generate(&h.root, PROVIDER, false).unwrap();
        }
        assert_eq!(fs::read(h.root.join(PROVIDER)).unwrap(), source);
        assert_eq!(
            fs::read_to_string(h.root.join(".github/workflows/host.yml")).unwrap(),
            "host-owned\n"
        );
    }
    let h = ShortHost::new();
    let prior = ".chrono-harness/state/old-provider.json";
    json_file(
        &h.root,
        prior,
        &serde_json::from_slice::<Value>(&fs::read(h.root.join(PROVIDER)).unwrap()).unwrap(),
    );
    let mut c: Value = serde_json::from_slice(&fs::read(h.root.join(PROVIDER)).unwrap()).unwrap();
    c["job_gating"] = gating();
    c["units"]["alpha"]["runs_on"] = json!("new-custom");
    json_file(&h.root, PROVIDER, &c);
    let report = chrono_ci::dispatch(&[
        "migrate".into(),
        "--host-root".into(),
        h.root.to_str().unwrap().into(),
        "--config".into(),
        PROVIDER.into(),
        "--from".into(),
        prior.into(),
        "--previous-config".into(),
        PROVIDER.into(),
    ])
    .unwrap();
    let r: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(r["retired_workflows"], json!([]));
    assert!(
        fs::read_to_string(h.root.join(".github/workflows/collection.yml"))
            .unwrap()
            .contains("new-custom")
    );
    assert!(h.root.join(prior).exists());
    assert!(!generate(&h.root, PROVIDER, true).unwrap());
}

#[test]
fn copied_conditional_example_executes_fixed_unit_and_collection_commands_without_rust_source() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/ci-host-job-gating");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("installed tools host");
    fs::create_dir(&root).unwrap();
    let map: Value =
        serde_json::from_slice(&fs::read(source.join(".chrono-harness/FILEMAP.json")).unwrap())
            .unwrap();
    for row in map["files"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        fs::copy(source.join(path), root.join(path)).unwrap();
    }
    let distribution = dir.path().join("existing tools distribution");
    fs::create_dir(&distribution).unwrap();
    install(&distribution);
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../worktree/target/debug/chrono-worktree"),
        distribution.join(".chrono-harness/bin/chrono-worktree"),
    )
    .unwrap();
    let installed = Command::new("/bin/sh")
        .arg(root.join(".chrono-harness/ci/install.sh"))
        .arg(&root)
        .arg(distribution.join(".chrono-harness/bin"))
        .output()
        .unwrap();
    assert!(
        installed.status.success(),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    install_parent_provider(&root);
    let git_bin = fixture_git();
    let mut cfg: Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/config.json")).unwrap())
            .unwrap();
    for tool in cfg["tools"].as_array_mut().unwrap() {
        if tool["id"] == "git" {
            tool["program"] = json!(git_bin);
        }
    }
    json_file(&root, ".chrono-harness/config.json", &cfg);
    let mut provider: Value =
        serde_json::from_slice(&fs::read(root.join(PROVIDER)).unwrap()).unwrap();
    provider["gather"]["program"] = json!(".chrono-harness/bin/parent-gh");
    json_file(&root, PROVIDER, &provider);
    generate(&root, PROVIDER, false).unwrap();
    git(&root, &["init", "-q", "-b", "dev"]);
    git(&root, &["config", "user.name", "Fixture"]);
    git(&root, &["config", "user.email", "fixture@example.invalid"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "example base"]);
    let base = git(&root, &["rev-parse", "HEAD"]);
    let remote = dir.path().join("target.git");
    fs::create_dir(&remote).unwrap();
    git(&remote, &["init", "--bare", "-q"]);
    git(
        &root,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&root, &["push", "-q", "origin", "dev"]);
    let mut h = GatedHost {
        host: ShortHost {
            _dir: crate::tools::FixtureDirectory::Ephemeral(dir),
            root,
            remote,
            base: base.clone(),
            candidate: base.clone(),
        },
        payload: json!({"ref":"refs/heads/dev","before":base,"after":base,"created":false,"deleted":false}),
    };
    h.payload["before"] = json!("0".repeat(40));
    h.payload["created"] = json!(true);
    let initial = h.detection();
    assert_eq!(initial["initial"], true);
    assert_eq!(initial["required_units"], json!(["example", "harness"]));
    h.units(&initial);
    h.mock(&initial, "success");
    let initial_needs = needs(&h, &initial);
    let out = h
        .command(&["check", "--collect"], &initial, &initial_needs)
        .output()
        .unwrap();
    retain_command_result(
        &h.host.root,
        &["check".into(), "--collect".into()],
        &json!({"detection":initial,"needs":initial_needs}),
        &out,
    );
    assert!(
        out.status.success(),
        "status {}; stdout {}; stderr {}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    h.payload["before"] = json!(h.host.base);
    h.payload["created"] = json!(false);
    fs::write(h.host.root.join("message.txt"), "hello\n\n").unwrap();
    h.host.revise();
    h.payload["after"] = json!(h.host.candidate);
    let d = h.detection();
    assert_eq!(d["required_units"], json!(["example"]));
    let out = h
        .command(&["check", "--unit", "example"], &d, &Value::Null)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    h.produce("example");
    copy_artifacts(
        &h.host.root.join(".chrono-harness/state/example"),
        &h.host
            .root
            .join(".chrono-harness/state/mock-artifacts/example"),
    );
    h.mock(&d, "success");
    let n = needs(&h, &d);
    let out = h.command(&["check", "--collect"], &d, &n).output().unwrap();
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let original: Value = serde_json::from_slice(
        &fs::read(
            h.host
                .root
                .join(".chrono-harness/state/collection/check.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(original["response"]["evidence"]["executed"], json!([]));
    fs::write(h.host.root.join("message.txt"), "wrong\n").unwrap();
    h.host.revise();
    h.payload["after"] = json!(h.host.candidate);
    let d = h.detection();
    let out = h
        .command(&["check", "--unit", "example"], &d, &Value::Null)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!h.host.root.join("crates").exists());
}

#[test]
fn independent_job_retry_keeps_exact_original_prerequisite_attempts_in_one_parent_run() {
    let mut h = GatedHost::new();
    h.change(&["a.txt", "b.txt"]);
    let d = h.detection();
    h.units(&d);
    let beta = fs::read(
        h.host
            .root
            .join(".chrono-harness/state/mock-artifacts/beta/check.json"),
    )
    .unwrap();
    // GitHub reruns alpha and its dependent aggregate, retaining detector/beta attempt 2.
    let out = h
        .command(&["check", "--unit", "alpha"], &d, &Value::Null)
        .env("GITHUB_RUN_ATTEMPT", "3")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    h.produce("alpha");
    copy_artifacts(
        &h.host.root.join(".chrono-harness/state/alpha"),
        &h.host
            .root
            .join(".chrono-harness/state/mock-artifacts/alpha"),
    );
    h.mock(&d, "retry");
    let retry_needs = needs(&h, &d);
    let out = h
        .command(&["check", "--collect"], &d, &retry_needs)
        .env("GITHUB_RUN_ATTEMPT", "3")
        .output()
        .unwrap();
    retain_command_result(
        &h.host.root,
        &["check".into(), "--collect".into()],
        &json!({"detection":d,"needs":retry_needs,"attempt":3}),
        &out,
    );
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let gather: Value = serde_json::from_slice(
        &fs::read(
            h.host
                .root
                .join(".chrono-harness/state/collection/gather.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(gather["result"]["units"][0]["attempt"], 3);
    assert_eq!(gather["result"]["units"][1]["attempt"], 2);
    assert_eq!(
        fs::read(
            h.host
                .root
                .join(".chrono-harness/state/mock-artifacts/beta/check.json")
        )
        .unwrap(),
        beta
    );
    // A newly rerun parent cannot accept an old artifact labelled as alpha attempt 3.
    let context = ".chrono-harness/state/mock-artifacts/alpha/context.json";
    let mut c: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(context)).unwrap()).unwrap();
    c["native_run"]["run_attempt"] = json!(2);
    json_file(&h.host.root, context, &c);
    let out = h
        .command(&["check", "--collect"], &d, &needs(&h, &d))
        .env("GITHUB_RUN_ATTEMPT", "3")
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn parent_collection_recovers_download_without_reexecuting_units_or_relaxing_identity() {
    let mut h = GatedHost::new();
    let mut provider: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(PROVIDER)).unwrap()).unwrap();
    provider["gather"]["download_retry"] =
        json!({"max_attempts":3,"delay_seconds":1,"http_statuses":[503]});
    json_file(&h.host.root, PROVIDER, &provider);
    generate(&h.host.root, PROVIDER, false).unwrap();
    h.change(&["a.txt"]);
    let d = h.detection();
    h.units(&d);
    h.mock(&d, "download-unavailable-once");
    let out = h
        .command(&["check", "--collect"], &d, &needs(&h, &d))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let gather: Value = serde_json::from_slice(
        &fs::read(
            h.host
                .root
                .join(".chrono-harness/state/collection/gather.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let attempts: Vec<_> = gather["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| {
            p["download"]["artifact"]
                .as_str()
                .is_some_and(|name| name.starts_with("chrono-unit-alpha-"))
        })
        .collect();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0]["download"]["action"], "retry");
    assert_eq!(attempts[1]["download"]["action"], "completed");
    assert_eq!(
        attempts[0]["download"]["artifact"],
        attempts[1]["download"]["artifact"]
    );
    let collected = published_report(&h.host.root, None, &out);
    assert_eq!(collected["response"]["evidence"]["executed"], json!([]));
    fs::write(
        h.host
            .root
            .join(".chrono-harness/state/mock-artifacts/alpha/check.json"),
        b"{}",
    )
    .unwrap();
    assert!(
        !h.command(&["check", "--collect"], &d, &needs(&h, &d))
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn carried_api_attempts_and_aggregate_only_reruns_use_original_artifact_outputs() {
    let mut h = GatedHost::new();
    h.change(&["a.txt", "b.txt"]);
    let d = h.detection();
    h.units(&d);
    let originals: Vec<_> = ["alpha", "beta"]
        .into_iter()
        .map(|u| {
            fs::read(h.host.root.join(format!(
                ".chrono-harness/state/mock-artifacts/{u}/check.json"
            )))
            .unwrap()
        })
        .collect();
    for mode in ["carried", "aggregate-only"] {
        h.mock(&d, mode);
        let out = h
            .command(&["check", "--collect"], &d, &needs(&h, &d))
            .env("GITHUB_RUN_ATTEMPT", "3")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{mode}: {} {}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        let gather: Value = serde_json::from_slice(
            &fs::read(
                h.host
                    .root
                    .join(".chrono-harness/state/collection/gather.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(gather["result"]["units"][0]["attempt"], 2);
        assert_eq!(gather["result"]["units"][1]["attempt"], 2);
    }
    h.mock(&d, "latest-failure");
    let out = h
        .command(&["check", "--collect"], &d, &needs(&h, &d))
        .env("GITHUB_RUN_ATTEMPT", "3")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "revived success after latest API failure"
    );
    for (u, raw) in ["alpha", "beta"].into_iter().zip(originals) {
        assert_eq!(
            fs::read(h.host.root.join(format!(
                ".chrono-harness/state/mock-artifacts/{u}/check.json"
            )))
            .unwrap(),
            raw
        );
    }
}

#[test]
fn registered_dotted_and_underscore_unit_ids_have_distinct_valid_jobs_and_actual_short_checks() {
    let mut h = GatedHost::new();
    let mut p: Value =
        serde_json::from_slice(&fs::read(h.host.root.join(PROVIDER)).unwrap()).unwrap();
    let alpha = p["units"].as_object_mut().unwrap().remove("alpha").unwrap();
    let beta = p["units"].as_object_mut().unwrap().remove("beta").unwrap();
    p["units"]["compiler.arm64-1"] = alpha;
    p["units"]["compiler_2earm64-1"] = beta;
    json_file(&h.host.root, PROVIDER, &p);
    let path = ".chrono-harness/ci/check.json";
    let mut c: Value = serde_json::from_slice(&fs::read(h.host.root.join(path)).unwrap()).unwrap();
    let alpha = c["policy"]["units"]
        .as_object_mut()
        .unwrap()
        .remove("alpha")
        .unwrap();
    let beta = c["policy"]["units"]
        .as_object_mut()
        .unwrap()
        .remove("beta")
        .unwrap();
    c["policy"]["units"]["compiler.arm64-1"] = alpha;
    c["policy"]["units"]["compiler_2earm64-1"] = beta;
    json_file(&h.host.root, path, &c);
    generate(&h.host.root, PROVIDER, false).unwrap();
    h.host.revise();
    h.payload["after"] = json!(h.host.candidate);
    let d = h.detection();
    assert_eq!(
        d["required_units"],
        json!(["compiler.arm64-1", "compiler_2earm64-1"])
    );
    let outputs =
        fs::read_to_string(h.host.root.join(".chrono-harness/state/detector-output")).unwrap();
    assert!(outputs.contains("unit_compiler_2earm64-1=true"));
    assert!(outputs.contains("unit_compiler_5f2earm64-1=true"));
    let out = h
        .command(&["check", "--unit", "compiler.arm64-1"], &d, &Value::Null)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!h.host.root.join(".chrono-harness/state/calls-b").exists());
}

#[test]
fn current_product_host_registration_and_generated_parent_select_the_real_source_delta() {
    let source = fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    let original = git(&source, &["rev-parse", "HEAD"]);
    // Fixed baseline data is exported; no other branch or active worktree is read.
    let dir = tempfile::tempdir().unwrap();
    let archive = Command::new(fixture_git())
        .current_dir(&source)
        .args(["archive", &original])
        .output()
        .unwrap();
    assert!(archive.status.success());
    let mut unpack = Command::new("tar")
        .current_dir(dir.path())
        .arg("-x")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    unpack
        .stdin
        .take()
        .unwrap()
        .write_all(&archive.stdout)
        .unwrap();
    assert!(unpack.wait().unwrap().success());
    git(dir.path(), &["init", "-q", "-b", "dev"]);
    git(dir.path(), &["config", "user.name", "Fixture"]);
    git(
        dir.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    // Both fixed endpoints belong to this synthetic host. This fixture explicitly
    // requests digest/version rejection controls; the product host need not
    // maintain expected executable identities for every native machine.
    let program = fixture_git();
    let version = Command::new(&program).arg("--version").output().unwrap();
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).unwrap();
    let config_path = ".chrono-harness/config.json";
    let bind_config = |root: &Path| {
        let mut config: Value = serde_json::from_slice(
            &fs::read(root.join(".chrono-harness/git/macos-27.json")).unwrap(),
        )
        .unwrap();
        let tool_id = config["facts_git"]["tool"].as_str().unwrap().to_owned();
        let input_id = "fixture-git-executable";
        config["facts_git"]["input"] = json!(input_id);
        let tool = config["tools"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|tool| tool["id"] == tool_id)
            .unwrap();
        tool["program"] = json!(program);
        tool["expected_version"] = json!(version.trim());
        config["environment"]["inputs"]
            .as_array_mut()
            .unwrap()
            .push(
                json!({"id":input_id,"location":program,"presence":"present",
                "sha256":sha256(&fs::read(&program).unwrap())}),
            );
        let map_path = ".chrono-harness/FILEMAP.json";
        let mut map: Value =
            serde_json::from_slice(&fs::read(root.join(map_path)).unwrap()).unwrap();
        map["project_edges"].as_array_mut().unwrap().push(json!({
            "from":"input:fixture-git-executable", "kind":"runtime-input", "to":"project:ci"
        }));
        json_file(root, map_path, &map);
        json_file(root, config_path, &config);
        config
    };
    bind_config(dir.path());
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-qm", "fixed source baseline data"]);
    let base = git(dir.path(), &["rev-parse", "HEAD"]);
    let map: Value =
        serde_json::from_slice(&fs::read(source.join(".chrono-harness/FILEMAP.json")).unwrap())
            .unwrap();
    let allowed: std::collections::BTreeSet<_> = map["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["path"].as_str().unwrap().to_owned())
        .collect();
    let tracked = git(dir.path(), &["ls-files"]);
    for path in tracked.lines() {
        if !allowed.contains(path) {
            fs::remove_file(dir.path().join(path)).unwrap();
        }
    }
    for path in &allowed {
        let target = dir.path().join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        if fs::symlink_metadata(source.join(path))
            .unwrap()
            .file_type()
            .is_symlink()
        {
            if target.exists() {
                fs::remove_file(&target).unwrap();
            }
            std::os::unix::fs::symlink(fs::read_link(source.join(path)).unwrap(), &target).unwrap();
        } else {
            fs::copy(source.join(path), target).unwrap();
        }
    }
    let config = bind_config(dir.path());
    let tool_id = config["facts_git"]["tool"].as_str().unwrap().to_owned();
    let input_id = config["facts_git"]["input"].as_str().unwrap().to_owned();
    // A containing migration fixture deliberately supplies workflow drift.
    // This test owns a separate candidate projection; the outer verifier still
    // rejects its original drift and is never modified here.
    generate(dir.path(), PROVIDER, false).unwrap();
    git(dir.path(), &["config", "user.name", "Fixture"]);
    git(
        dir.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    // A copied migration candidate can already contain the group adoption;
    // regeneration can still change only its deliberately drifted workflow.
    // Bind both producer changes explicitly for the selection assertions below.
    for source_path in [
        "crates/ci/src/gating.rs",
        "crates/judge-workflow/src/lib.rs",
    ] {
        let path = dir.path().join(source_path);
        let mut bytes = fs::read(&path).unwrap();
        bytes.extend_from_slice(b"\n// Explicit fixture source DELTA.\n");
        fs::write(path, bytes).unwrap();
    }
    git(dir.path(), &["add", "."]);
    git(
        dir.path(),
        &["commit", "-qm", "current product host source delta"],
    );
    let candidate = git(dir.path(), &["rev-parse", "HEAD"]);
    install(dir.path());
    let payload = json!({"ref":"refs/heads/dev","before":base,"after":candidate,"created":false,"deleted":false});
    json_file(dir.path(), ".chrono-harness/state/event.json", &payload);
    let mut command = Command::new(dir.path().join(".chrono-harness/bin/chrono-ci"));
    command
        .current_dir(dir.path())
        .args([
            "detect",
            "--host-root",
            ".",
            "--config",
            PROVIDER,
            "--github-output",
            ".chrono-harness/state/output",
        ])
        .env("GITHUB_EVENT_NAME", "push")
        .env(
            "GITHUB_EVENT_PATH",
            dir.path().join(".chrono-harness/state/event.json"),
        )
        .env("GITHUB_REPOSITORY", "owner/host")
        .env("GITHUB_RUN_ID", "101")
        .env("GITHUB_RUN_ATTEMPT", "1")
        .env("GITHUB_JOB", "detect")
        .env("CHRONO_WORKFLOW_REVISION", &candidate);
    let argv: Vec<String> = std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    let out = command.output().unwrap();
    retain_command_result(
        dir.path(),
        &argv,
        &json!({"payload":payload,"candidate":candidate}),
        &out,
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: Value = serde_json::from_slice(&out.stdout).unwrap();
    // Keep the production binding checks strict after accepting the real
    // explicit fixture binding, including hosts with identical Git versions.
    for (case, expected) in [
        ("bytes", "facts_git declared digest mismatch"),
        ("version", "Git facts version mismatch"),
    ] {
        let mut wrong = config.clone();
        if case == "bytes" {
            wrong["environment"]["inputs"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|input| input["id"] == input_id)
                .unwrap()["sha256"] = json!("0".repeat(64));
        } else {
            wrong["tools"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|tool| tool["id"] == tool_id)
                .unwrap()["expected_version"] = json!("wrong fixture Git version");
        }
        json_file(dir.path(), config_path, &wrong);
        let error = chrono_harness::facts::Reader::for_config(dir.path(), config_path)
            .err()
            .expect("accepted a wrong fixture Git binding");
        assert!(error.contains(expected), "{case}: {error}");
    }
    json_file(dir.path(), config_path, &config);
    assert!(
        result["detection"]["required_units"]
            .as_array()
            .unwrap()
            .contains(&json!("ci"))
    );
    for (unit, test) in [
        ("judge-workflow", "test:judge-workflow-tests"),
        ("judge-workflow-units", "test:workflow-units"),
        ("judge-workflow-short", "test:workflow-short"),
    ] {
        assert!(
            result["detection"]["required_units"]
                .as_array()
                .unwrap()
                .contains(&json!(unit)),
            "{result:#}"
        );
        assert!(
            result["requirements"]["global_selected"]
                .as_array()
                .unwrap()
                .contains(&json!(test)),
            "{result:#}"
        );
    }
    let projects_path = ".chrono-harness/projects.json";
    let previous: Value = serde_json::from_str(&git(
        dir.path(),
        &["show", &format!("{base}:{projects_path}")],
    ))
    .unwrap();
    let current: Value =
        serde_json::from_slice(&fs::read(dir.path().join(projects_path)).unwrap()).unwrap();
    let project = |value: &Value| {
        value["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|project| project["id"] == "judge-workflow-tests")
            .unwrap()
            .clone()
    };
    let before = project(&previous);
    let after = project(&current);
    assert_eq!(before["actions"]["execute"], after["actions"]["execute"]);
    assert_eq!(
        after["test_groups"],
        json!({"judge-workflow-tests":"execute_core", "workflow-units":"execute_units", "workflow-short":"execute_short"})
    );
    // Retain the real selection as bounded validation data, not a fabricated check verdict.
    json_file(
        &source,
        ".chrono-harness/state/current-host-delta-selection.json",
        &json!({"source_baseline":original,"fixture_base":base,"fixture_candidate":candidate,"selection_scope":"actual-worktree-and-explicit-fixture-delta","detection":result["detection"],"global_selected":result["requirements"]["global_selected"],"native_actions_verified":false}),
    );
}

#[test]
fn artifact_identity_and_cumulative_budgets_cover_download_and_source_checks() {
    for mode in [
        "artifact-missing",
        "artifact-misbound",
        "artifact-deadline",
        "final-parent-deadline",
        "artifact-stdout",
        "artifact-stderr",
    ] {
        let mut h = GatedHost::new();
        let source_length = fs::metadata(h.host.root.join(".github/workflows/collection.yml"))
            .unwrap()
            .len();
        if matches!(
            mode,
            "artifact-deadline" | "final-parent-deadline" | "artifact-stdout" | "artifact-stderr"
        ) {
            h.host.modify(PROVIDER, |c| {
                if matches!(mode, "artifact-deadline" | "final-parent-deadline") {
                    c["gather"]["timeout_seconds"] = json!(2);
                }
                if mode == "artifact-stdout" {
                    c["gather"]["output_limit_bytes"] = json!(source_length + 1024);
                }
                if mode == "artifact-stderr" {
                    c["gather"]["output_limit_bytes"] = json!(24576);
                }
            });
        }
        h.change(&["a.txt"]);
        let d = h.detection();
        h.units(&d);
        h.mock(&d, mode);
        let out = h
            .command(&["check", "--collect"], &d, &needs(&h, &d))
            .output()
            .unwrap();
        assert!(!out.status.success(), "accepted {mode}");
        let report: Value = serde_json::from_slice(
            &fs::read(
                h.host
                    .root
                    .join(".chrono-harness/state/collection/gather.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(report["result"].is_null(), "{mode}: {report}");
        assert!(report["error"].is_string(), "{mode}: {report}");
        assert!(
            !h.host
                .root
                .join(".chrono-harness/state/collection/manifest.json")
                .exists()
        );
        let processes = report["processes"].as_array().unwrap();
        if mode.starts_with("artifact-m") {
            assert!(
                report["error"]
                    .as_str()
                    .unwrap()
                    .contains("binding differs")
            );
            assert!(!processes.iter().any(|p| p["argv"][1] == "run"));
        } else if matches!(mode, "artifact-deadline" | "final-parent-deadline") {
            // Metadata, download and source checks share one deadline. A slow
            // earlier phase must reject too; reaching the source is not promised.
            assert_eq!(
                report["error"], "artifact acquisition cumulative deadline exhausted",
                "{mode}: {report}"
            );
            assert!(
                processes.iter().any(|p| p["argv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|a| a.as_str().is_some_and(|s| s.contains("/artifacts?")))),
                "{mode}: acquisition was not reached"
            );
        } else {
            assert!(
                processes.iter().any(|p| p["argv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|a| a.as_str().is_some_and(|s| s.contains("/contents/")))),
                "{mode}: source check was not reached"
            );
            assert!(
                processes.iter().any(|p| !p["failure"].is_null())
                    || report["error"].as_str().unwrap().contains("cumulative"),
                "{mode}: {report}"
            );
        }
    }
}
