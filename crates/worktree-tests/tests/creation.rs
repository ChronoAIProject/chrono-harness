#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_harness::{json, sha256};
use serde_json::{Value, json as value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use support::*;
const POLICY: &str = ".chrono-harness/worktree.json";
const SOURCE_CONFIG: &str = ".chrono-harness/source platform.json";
const TARGET_CONFIG: &str = ".chrono-harness/fetched platform.json";
const TARGET_WORKFLOW: &str = ".chrono-harness/fetched workflow.json";
mod automatic;
mod check_inputs;
mod interrupted_cleanup;
mod maintenance;
mod rebind;

struct Host {
    _dir: tempfile::TempDir,
    root: PathBuf,
    remote: PathBuf,
    parent: PathBuf,
}
fn retain_fixture_state(root: &Path, destination: &Path) {
    if !root.is_dir() {
        return;
    }
    fs::create_dir_all(destination).unwrap();
    for row in fs::read_dir(root).unwrap() {
        let row = row.unwrap();
        let kind = row.file_type().unwrap();
        if kind.is_dir() {
            retain_fixture_state(&row.path(), &destination.join(row.file_name()));
        } else if kind.is_file() {
            fs::copy(row.path(), destination.join(row.file_name())).unwrap();
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if let Ok(directory) = std::env::var("CHRONO_WORKTREE_TEST_RECEIPTS") {
            let destination = Path::new(&directory).join(format!(
                "fixture-{}",
                sha256(self.root.as_os_str().as_encoded_bytes())
            ));
            retain_fixture_state(
                &self.root.join(".chrono-harness/state"),
                &destination.join("coordinator-state"),
            );
            for name in [
                "git-mutation.stdout",
                "git-mutation.stderr",
                "admission-holder",
                "native-hook",
            ] {
                let path = self.parent.join(name);
                if path.is_file() {
                    fs::copy(path, destination.join(name)).unwrap();
                }
            }
            // Only explicitly enrolled fixtures, never live worker artifacts.
            if let Ok(bytes) = fs::read(
                self.root
                    .join(".chrono-harness/state/automatic-cleanup/ledger.json"),
            ) {
                if let Ok(ledger) = json(&bytes) {
                    for entry in ledger["entries"].as_array().unwrap() {
                        let target = Path::new(entry["path"].as_str().unwrap());
                        retain_fixture_state(
                            &target.join(".chrono-harness/state"),
                            &destination.join(format!(
                                "enrollment-{}",
                                sha256(target.as_os_str().as_encoded_bytes())
                            )),
                        );
                    }
                }
            }
        }
    }
}
impl Host {
    fn new(layout: &str) -> Self {
        let dir = tempfile::Builder::new()
            .prefix("worktree host λ ")
            .tempdir()
            .unwrap();
        let parent = fs::canonicalize(dir.path()).unwrap();
        let root = parent.join("source with spaces");
        let remote = parent.join("declared upstream.git");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&remote).unwrap();
        git(&remote, &["init", "--bare", "--initial-branch=dev", "-q"]);
        git(&root, &["init", "--initial-branch=dev", "-q"]);
        let mut v = values();
        for x in v.values_mut() {
            x["status"] = value!("proposed");
        }
        v.get_mut(CONFIG).unwrap()["enforcement"] = value!("not-implemented");
        v.get_mut(CONFIG).unwrap()["input_closure"] = value!({"status":"incomplete","unresolved":["fixture does not certify complete inputs"]});
        v.get_mut(PROJECTS).unwrap()["owners"] = value!(["host"]);
        v.get_mut(PROJECTS).unwrap()["projects"] = value!([]);
        let paths = [
            CONFIG,
            FM,
            PROJECTS,
            JUDGES,
            WORKFLOW,
            POLICY,
            ".gitignore",
            layout,
        ];
        v.get_mut(FM).unwrap()["files"] = value!(
            paths
                .iter()
                .map(|p| file(p, value!([])))
                .collect::<Vec<_>>()
        );
        v.get_mut(FM).unwrap()["project_edges"] = value!([]);
        v.get_mut(FM).unwrap()["test_costs"] = value!([]);
        write_values(&root, &v);
        fs::write(
            root.join(".gitignore"),
            ".chrono-harness/state/\n.chrono-harness/bin/\n",
        )
        .unwrap();
        fs::create_dir_all(root.join(layout).parent().unwrap()).unwrap();
        fs::write(
            root.join(layout),
            "literal host input; no dependency inference\n",
        )
        .unwrap();
        let policy = value!({"schema":"chrono-worktree-config/v1","host_config":CONFIG,"remote":"warehouse","git":{"program":"git","expected_version":null,"sha256":null},"environment":{"inherit":["PATH"],"values":{"HOME":parent,"GIT_CONFIG_NOSYSTEM":"1","GIT_CONFIG_GLOBAL":"/dev/null","GIT_TERMINAL_PROMPT":"0"}},"timeout_seconds":30,"output_limit_bytes":1048576,"report_directory":".chrono-harness/state/worktrees/"});
        fs::write(
            root.join(POLICY),
            serde_json::to_vec_pretty(&policy).unwrap(),
        )
        .unwrap();
        commit(&root);
        git(
            &root,
            &["remote", "add", "warehouse", remote.to_str().unwrap()],
        );
        git(&root, &["push", "-q", "warehouse", "dev"]);
        Self {
            _dir: dir,
            root,
            remote,
            parent,
        }
    }
    fn policy(&self, f: impl FnOnce(&mut Value)) {
        let mut p = json(&fs::read(self.root.join(POLICY)).unwrap()).unwrap();
        f(&mut p);
        fs::write(
            self.root.join(POLICY),
            serde_json::to_vec_pretty(&p).unwrap(),
        )
        .unwrap();
        commit(&self.root);
        git(&self.root, &["push", "-q", "warehouse", "dev"]);
    }
    fn invoke(&self, kind: &str, name: &str, target: &Path) -> (i32, Value, String) {
        let out = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .current_dir("/")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("UNDECLARED_WORKTREE_ENV", "must-not-leak")
            .args([
                "start",
                "--host-root",
                self.root.to_str().unwrap(),
                "--config",
                POLICY,
                "--kind",
                kind,
                "--name",
                name,
                "--path",
                target.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        self.received(out)
    }
    fn received(&self, out: std::process::Output) -> (i32, Value, String) {
        if let Ok(directory) = std::env::var("CHRONO_WORKTREE_TEST_RECEIPTS") {
            static NUMBER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let id = NUMBER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let directory = Path::new(&directory).join(format!("cli-{}-{id}", std::process::id()));
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join("stdout.bin"), &out.stdout).unwrap();
            fs::write(directory.join("stderr.bin"), &out.stderr).unwrap();
            fs::write(directory.join("binding.json"), serde_json::to_vec_pretty(&value!({
                "root":self.root,"exit":out.status.code(),"status":out.status.to_string(),"joined":true,
                "head":git(&self.root, &["rev-parse", "HEAD"]),
                "policy_sha256":sha256(&fs::read(self.root.join(POLICY)).unwrap())
            })).unwrap()).unwrap();
        }
        let report = json(&out.stdout).unwrap_or(Value::Null);
        if !report.is_null() {
            assert_eq!(
                report,
                json(
                    &fs::read(
                        Path::new(report["source_root"].as_str().unwrap())
                            .join(report["report_path"].as_str().unwrap())
                    )
                    .unwrap()
                )
                .unwrap()
            );
            assert_eq!(report["governance"], "not-evaluated");
            assert_eq!(report["parity"], "unestablished");
            assert!(
                report["environment"]["effective"]
                    .get("UNDECLARED_WORKTREE_ENV")
                    .is_none()
            );
            for row in report["processes"].as_array().unwrap() {
                if row["process"].is_null() {
                    continue;
                }
                for stream in ["stdout", "stderr"] {
                    let bytes: Vec<u8> =
                        serde_json::from_value(row["process"][format!("{stream}_bytes")].clone())
                            .unwrap();
                    assert_eq!(row["process"][format!("{stream}_sha256")], sha256(&bytes));
                }
            }
        }
        (
            out.status.code().unwrap_or(-1),
            report,
            String::from_utf8_lossy(&out.stderr).into(),
        )
    }
    fn hook(&self, settings: Value) {
        use std::os::unix::fs::PermissionsExt;
        let p = self.root.join(".git/hooks/post-checkout");
        fs::copy(env!("CARGO_BIN_EXE_chrono-worktree-test-hook"), &p).unwrap();
        fs::write(
            p.with_extension("json"),
            serde_json::to_vec(&settings).unwrap(),
        )
        .unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Keep the worktree policy's stable host_config entry unchanged.
fn select_config(root: &Path, target: &str) {
    let entry = json(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    let old = entry["platforms"][platform()].as_str().unwrap_or(CONFIG);
    let mut config = json(&fs::read(root.join(old)).unwrap()).unwrap();
    if config["schema_version"] != 3 {
        let git = chrono_harness::resolve_program(root, "git", None).unwrap();
        let version = Command::new(&git).arg("--version").output().unwrap();
        assert!(version.status.success());
        config["schema_version"] = value!(3);
        config["facts_git"] = value!({"tool":"git","input":"git-bytes"});
        config["tools"] = value!([{"id":"git","program":git,"resolution":"PATH-once",
            "version_argv":["--version"],"expected_version":String::from_utf8(version.stdout).unwrap().trim()}]);
        config["environment"]["inputs"] = value!([{"id":"git-bytes","location":git,
            "presence":"present","sha256":sha256(&fs::read(&git).unwrap())}]);
    }
    fs::write(root.join(target), serde_json::to_vec(&config).unwrap()).unwrap();
    fs::write(
        root.join(CONFIG),
        serde_json::to_vec(&value!({
            "schema":"chrono-git-configs/v1","platforms":{platform():target}
        }))
        .unwrap(),
    )
    .unwrap();
    let mut fm = json(&fs::read(root.join(FM)).unwrap()).unwrap();
    let files = fm["files"].as_array_mut().unwrap();
    if old != CONFIG && old != target {
        fs::remove_file(root.join(old)).unwrap();
        files.retain(|f| f["path"] != old);
    }
    if !files.iter().any(|f| f["path"] == target) {
        files.push(file(target, value!([])));
    }
    fs::write(root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
}

fn selected_values(root: &Path, oid: &str, effective: &str, workflow: &str) -> Values {
    [CONFIG, effective, JUDGES, PROJECTS, FM, workflow]
        .into_iter()
        .map(|path| {
            (
                path.into(),
                json(git(root, &["show", &format!("{oid}:{path}")]).as_bytes()).unwrap(),
            )
        })
        .collect()
}

fn assert_fixed_registry_reads(
    report: &Value,
    repository: &Path,
    root: &Path,
    oid: &str,
    values: &Values,
) {
    let processes = report["processes"].as_array().unwrap();
    let config = values
        .values()
        .find(|v| v["registries"].is_object())
        .unwrap();
    let paths = ["judges", "projects", "filemap", "workflow"]
        .map(|key| config["registries"][key].as_str().unwrap());
    let metadata_input = paths
        .iter()
        .map(|path| format!("{oid}:{path}\n"))
        .collect::<String>();
    for (path, expected) in values {
        let argv = value!(["--no-replace-objects", "show", format!("{oid}:{path}")]);
        let mut rows: Vec<_> = processes
            .iter()
            .filter(|row| row["root"] == value!(root) && row["argv"] == argv)
            .collect();
        let batched = rows.is_empty();
        let mut framed = Vec::new();
        if batched {
            let index = paths.iter().position(|p| *p == path).unwrap();
            let metadata = processes
                .iter()
                .find(|row| {
                    row["root"] == value!(root)
                        && row["argv"]
                            == value!(["--no-replace-objects", "cat-file", "--batch-check"])
                        && row["process"]["stdin_sha256"] == sha256(metadata_input.as_bytes())
                })
                .expect("missing exact endpoint/path batch binding");
            let output = Command::new("git")
                .current_dir(repository)
                .args(["show", &format!("{oid}:{path}")])
                .output()
                .unwrap();
            assert!(output.status.success());
            assert_eq!(json(&output.stdout).unwrap(), *expected);
            let blob = git(repository, &["rev-parse", &format!("{oid}:{path}")]);
            let header = format!("{blob} blob {}", output.stdout.len());
            let headers = metadata["process"]["stdout"]
                .as_str()
                .unwrap()
                .lines()
                .collect::<Vec<_>>();
            assert_eq!(headers.len(), paths.len());
            assert_eq!(headers[index], header);
            let input = headers
                .iter()
                .map(|line| format!("{}\n", line.split(' ').next().unwrap()))
                .collect::<String>();
            rows = processes
                .iter()
                .filter(|row| {
                    row["root"] == value!(root)
                        && row["argv"] == value!(["--no-replace-objects", "cat-file", "--batch"])
                        && row["process"]["stdin_sha256"] == sha256(input.as_bytes())
                })
                .collect();
            assert!(!rows.is_empty(), "missing original batch contents");
            framed.extend_from_slice(format!("{header}\n").as_bytes());
            framed.extend_from_slice(&output.stdout);
            framed.push(b'\n');
            // Metadata is also a real bound process with original bytes.
            assert_bound_registry_process(report, metadata);
        }
        assert!(!rows.is_empty(), "missing fixed read {oid}:{path}");
        for row in rows {
            let bytes: Vec<u8> =
                serde_json::from_value(row["process"]["stdout_bytes"].clone()).unwrap();
            if batched {
                assert!(bytes.windows(framed.len()).any(|part| part == framed));
            } else {
                assert_eq!(json(&bytes).unwrap(), *expected);
            }
            assert_bound_registry_process(report, row);
        }
    }
}
fn assert_bound_registry_process(report: &Value, row: &Value) {
    assert_eq!(row["process"]["exit_code"], 0);
    assert!(row["process"]["failure"].is_null());
    let bytes: Vec<u8> = serde_json::from_value(row["process"]["stdout_bytes"].clone()).unwrap();
    assert_eq!(row["process"]["stdout_sha256"], sha256(&bytes));
    assert_eq!(row["process"]["stdout"], String::from_utf8(bytes).unwrap());
    assert_eq!(
        row["process"]["environment"],
        report["environment"]["effective"]
    );
    assert_eq!(row["process"]["executable"], report["tool"]["path"]);
    assert_eq!(row["process"]["sha256"], report["tool"]["sha256"]);
}

#[test]
fn selected_start_resolves_source_and_fetched_targets_with_complete_digest() {
    let h = Host::new("arbitrary/input.data");
    let policy = fs::read(h.root.join(POLICY)).unwrap();
    select_config(&h.root, SOURCE_CONFIG);
    let source_head = commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let other = h.parent.join("other writer");
    git(
        &h.root,
        &[
            "clone",
            "-q",
            h.remote.to_str().unwrap(),
            other.to_str().unwrap(),
        ],
    );
    select_config(&other, TARGET_CONFIG);
    let mut config = json(&fs::read(other.join(TARGET_CONFIG)).unwrap()).unwrap();
    config["registries"]["workflow"] = value!(TARGET_WORKFLOW);
    fs::write(
        other.join(TARGET_CONFIG),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let mut workflow = json(&fs::read(other.join(WORKFLOW)).unwrap()).unwrap();
    workflow["feature_prefix"] = value!("selected/");
    fs::write(
        other.join(TARGET_WORKFLOW),
        serde_json::to_vec(&workflow).unwrap(),
    )
    .unwrap();
    fs::remove_file(other.join(WORKFLOW)).unwrap();
    let mut fm = json(&fs::read(other.join(FM)).unwrap()).unwrap();
    let files = fm["files"].as_array_mut().unwrap();
    files.retain(|f| f["path"] != WORKFLOW);
    files.push(file(TARGET_WORKFLOW, value!([])));
    fs::write(other.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    fs::write(other.join("arbitrary/input.data"), "fetched target\n").unwrap();
    let latest = commit(&other);
    git(&other, &["push", "-q", "origin", "dev"]);
    let source_values = selected_values(&h.root, &source_head, SOURCE_CONFIG, WORKFLOW);
    let target_values = selected_values(&other, &latest, TARGET_CONFIG, TARGET_WORKFLOW);
    // Neither endpoint may use dirty source files to resolve the selector.
    for path in [CONFIG, SOURCE_CONFIG, WORKFLOW, "arbitrary/input.data"] {
        fs::write(h.root.join(path), "unsaved source work\n").unwrap();
    }
    let target = h.parent.join("selected λ");
    let (code, report, error) = h.invoke("feature", "task", &target);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["status"], "created");
    assert_eq!(report["source_commit"], source_head);
    assert_eq!(report["base"], latest);
    assert_eq!(report["branch_ref"], "selected/task");
    assert_eq!(
        report["source_registry_digest"],
        chrono_harness::wire::digest(&value!(source_values)).unwrap()
    );
    assert_eq!(
        report["registry_digest"],
        chrono_harness::wire::digest(&value!(target_values)).unwrap()
    );
    assert_fixed_registry_reads(&report, &h.root, &h.root, &source_head, &source_values);
    assert_fixed_registry_reads(&report, &h.root, &h.root, &latest, &target_values);
    let identity_reads: Vec<_> = report["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["root"] == value!(h.root)
                && row["argv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|arg| arg == "--show-toplevel")
        })
        .collect();
    assert_eq!(identity_reads.len(), 1);
    let source_identity = identity_reads[0]["process"]["stdout"].as_str().unwrap();
    assert!(
        source_identity.lines().any(|line| line == source_head),
        "source root and HEAD must come from the same live observation"
    );
    assert_eq!(
        report["processes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["argv"] == value!(["--no-replace-objects", "cat-file", "--batch"])
            })
            .count(),
        2,
        "each explicit endpoint must reuse the bounded registry batch reader"
    );
    assert_eq!(
        report["processes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["argv"]
                == value!([
                    "--no-replace-objects",
                    "show",
                    format!("{source_head}:{CONFIG}")
                ]))
            .count(),
        1,
        "source registrations must be reused before fetch"
    );
    assert_eq!(report["config_path"], POLICY);
    assert_eq!(report["config_sha256"], sha256(&policy));
    assert_eq!(fs::read(target.join(POLICY)).unwrap(), policy);
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), latest);
    assert!(!target.join(SOURCE_CONFIG).exists());
    for path in [CONFIG, SOURCE_CONFIG, WORKFLOW, "arbitrary/input.data"] {
        assert_eq!(
            fs::read(h.root.join(path)).unwrap(),
            b"unsaved source work\n"
        );
    }
}

#[test]
fn registered_batch_failures_preserve_original_bytes_before_checkout_effects() {
    for (exit, output, expected) in [
        (71, "original partial", "Git cat-file exited 71"),
        (0, "malformed frame", "batch blob header mismatch"),
    ] {
        let h = Host::new("independent/data");
        automatic::native_git(&h, "batch-failure", value!({"stdout":output,"exit":exit}));
        let target = h.parent.join("refused");
        let (code, report, error) = h.invoke("feature", "refused", &target);
        assert_ne!(code, 0, "{report} {error}");
        assert_eq!(report["status"], "failed");
        assert!(
            report["error"].as_str().unwrap().contains(expected),
            "{report}"
        );
        assert!(!target.exists());
        let processes = report["processes"].as_array().unwrap();
        assert!(!processes.iter().any(|row| row["argv"][1] == "fetch"));
        let original = &processes.last().unwrap()["process"];
        assert_eq!(original["exit_code"], exit);
        assert_eq!(original["stdout"], output);
        assert_eq!(original["stdout_bytes"], value!(output.as_bytes()));
        assert_eq!(original["stdout_sha256"], sha256(output.as_bytes()));
        assert_eq!(original["stderr"], "original batch diagnostic");
    }
}

#[test]
fn selected_endpoint_errors_preserve_fixed_evidence_without_creating_checkout() {
    for fault in [
        "source-missing",
        "source-platform",
        "fetched-missing",
        "fetched-platform",
        "fetched-policy",
        "fetched-workflow",
    ] {
        let h = Host::new("payload");
        select_config(&h.root, SOURCE_CONFIG);
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
        let fetched = fault.starts_with("fetched-");
        let other = h.parent.join("other writer");
        let endpoint = if fetched {
            git(
                &h.root,
                &[
                    "clone",
                    "-q",
                    h.remote.to_str().unwrap(),
                    other.to_str().unwrap(),
                ],
            );
            &other
        } else {
            &h.root
        };
        match fault {
            "source-missing" | "fetched-missing" => {
                fs::remove_file(endpoint.join(SOURCE_CONFIG)).unwrap();
            }
            "source-platform" | "fetched-platform" => {
                fs::write(endpoint.join(CONFIG), serde_json::to_vec(&value!({
                    "schema":"chrono-git-configs/v1","platforms":{"unsupported-system":SOURCE_CONFIG}
                })).unwrap()).unwrap();
            }
            "fetched-policy" => {
                let mut config = json(&fs::read(endpoint.join(SOURCE_CONFIG)).unwrap()).unwrap();
                config["schema_version"] = value!(1);
                fs::write(
                    endpoint.join(SOURCE_CONFIG),
                    serde_json::to_vec(&config).unwrap(),
                )
                .unwrap();
            }
            "fetched-workflow" => {
                let mut workflow = json(&fs::read(endpoint.join(WORKFLOW)).unwrap()).unwrap();
                workflow["target_branch"] = value!("wrong-target");
                fs::write(
                    endpoint.join(WORKFLOW),
                    serde_json::to_vec(&workflow).unwrap(),
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let fixed = commit(endpoint);
        if fetched {
            git(endpoint, &["push", "-q", "origin", "dev"]);
        }
        let source_head = git(&h.root, &["rev-parse", "HEAD"]);
        let target = h.parent.join("must not exist");
        let (code, report, error) = h.invoke("feature", "rejected", &target);
        assert_eq!(code, 2, "{fault}: {report} {error}");
        let expected = match fault {
            "source-missing" | "fetched-missing" => "Git show exited",
            "source-platform" | "fetched-platform" => "no registered platform",
            "fetched-policy" => "requires a direct full-v3 policy",
            _ => "fetched target changed target branch",
        };
        assert!(
            report["error"].as_str().unwrap().contains(expected),
            "{fault}: {report}"
        );
        assert_eq!(report["source_commit"], source_head);
        assert!(!target.exists());
        assert!(
            !git(
                &h.root,
                &["for-each-ref", "--format=%(refname)", "refs/heads/"]
            )
            .contains("feature/rejected")
        );
        let processes = report["processes"].as_array().unwrap();
        assert!(
            !processes
                .iter()
                .any(|row| row["argv"][1] == "worktree" && row["argv"][2] == "add")
        );
        assert_eq!(
            processes.iter().any(|row| row["argv"][1] == "fetch"),
            fetched
        );
        if fetched {
            assert_eq!(report["base"], fixed);
            assert_eq!(report["fetch_ref_removed"], true);
        }
        let selected_path = if fault.ends_with("platform") {
            CONFIG
        } else if fault.ends_with("workflow") {
            WORKFLOW
        } else {
            SOURCE_CONFIG
        };
        if fault.ends_with("workflow") {
            assert_fixed_registry_reads(
                &report,
                endpoint,
                &h.root,
                &fixed,
                &selected_values(endpoint, &fixed, SOURCE_CONFIG, WORKFLOW),
            );
        } else {
            let row = processes
                .iter()
                .find(|row| {
                    row["argv"]
                        == value!([
                            "--no-replace-objects",
                            "show",
                            format!("{fixed}:{selected_path}")
                        ])
                })
                .unwrap();
            assert_eq!(row["process"]["executable"], report["tool"]["path"]);
            assert_eq!(row["process"]["sha256"], report["tool"]["sha256"]);
            assert_eq!(
                row["process"]["environment"],
                report["environment"]["effective"]
            );
            if fault.ends_with("missing") {
                assert_ne!(row["process"]["exit_code"], 0);
                assert!(
                    !row["process"]["stderr_bytes"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                );
            } else {
                assert_eq!(row["process"]["exit_code"], 0);
                assert_eq!(
                    json(row["process"]["stdout"].as_str().unwrap().as_bytes()).unwrap(),
                    json(git(endpoint, &["show", &format!("{fixed}:{selected_path}")]).as_bytes())
                        .unwrap()
                );
            }
        }
        assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), source_head);
    }
}

#[test]
fn fetches_advanced_registered_target_preserves_dirty_source_and_explicit_layouts() {
    for layout in ["deep/engine/rates.go", "unrelated/client/entry.mts"] {
        let h = Host::new(layout);
        let old = git(&h.root, &["rev-parse", "HEAD"]);
        let other = h.parent.join("other writer");
        git(
            &h.root,
            &[
                "clone",
                "-q",
                h.remote.to_str().unwrap(),
                other.to_str().unwrap(),
            ],
        );
        fs::write(other.join(layout), "new target content\n").unwrap();
        let latest = commit(&other);
        git(&other, &["push", "-q", "origin", "dev"]);
        fs::write(h.root.join(layout), "unsaved original work\n").unwrap();
        let target = h.parent.join("literal $(touch ignored) λ");
        let (code, r, e) = h.invoke("feature", "task", &target);
        assert_eq!(code, 0, "{r} {e}");
        assert_eq!(r["base"], latest);
        assert_eq!(r["context"]["candidate"], latest);
        assert_eq!(r["status"], "created");
        assert_eq!(r["fetch_ref_removed"], true);
        assert_eq!(git(&target, &["rev-parse", "HEAD"]), latest);
        assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), old);
        assert_eq!(
            fs::read_to_string(h.root.join(layout)).unwrap(),
            "unsaved original work\n"
        );
        assert_eq!(
            fs::read_to_string(target.join(layout)).unwrap(),
            "new target content\n"
        );
        assert_eq!(
            git(
                &h.root,
                &[
                    "for-each-ref",
                    "--format=%(refname)",
                    "refs/chrono-harness/fetch/"
                ]
            ),
            ""
        );
        assert!(!target.join("Cargo.toml").exists() && !target.join(".lake").exists());
    }
}
#[test]
fn uses_registered_prefix_and_disables_inherited_tracking_without_changing_config() {
    let h = Host::new("payload");
    let mut w = json(&fs::read(h.root.join(WORKFLOW)).unwrap()).unwrap();
    w["integration_prefix"] = value!("trial/");
    fs::write(h.root.join(WORKFLOW), serde_json::to_vec(&w).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    git(&h.root, &["config", "branch.autoSetupMerge", "always"]);
    git(&h.root, &["config", "branch.autoSetupRebase", "always"]);
    let config = fs::read(h.root.join(".git/config")).unwrap();
    let target = h.parent.join("new trial");
    let (code, r, e) = h.invoke("integration", "sample", &target);
    assert_eq!(code, 0, "{r} {e}");
    assert_eq!(r["branch_ref"], "trial/sample");
    assert_eq!(
        git(&target, &["symbolic-ref", "HEAD"]),
        "refs/heads/trial/sample"
    );
    assert_eq!(fs::read(h.root.join(".git/config")).unwrap(), config);
    assert!(git(&h.root, &["worktree", "list", "--porcelain"]).contains("trial/sample"));
}
#[test]
fn preserves_existing_destination_and_branch_instead_of_resetting_them() {
    let h = Host::new("payload");
    let target = h.parent.join("keep");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("sentinel"), "keep").unwrap();
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert!(r["error"].as_str().unwrap().contains("already exists"));
    assert_eq!(fs::read_to_string(target.join("sentinel")).unwrap(), "keep");
    let first = h.parent.join("first");
    assert_eq!(h.invoke("feature", "task", &first).0, 0);
    let head = git(&first, &["rev-parse", "HEAD"]);
    fs::write(first.join("payload"), "dirty owned work").unwrap();
    git(&h.root, &["worktree", "lock", first.to_str().unwrap()]);
    let next = h.parent.join("next");
    let (code, r, _) = h.invoke("feature", "task", &next);
    assert_ne!(code, 0);
    assert!(
        r["error"]
            .as_str()
            .unwrap()
            .contains("branch already exists")
    );
    assert!(!next.exists());
    assert_eq!(git(&first, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        fs::read_to_string(first.join("payload")).unwrap(),
        "dirty owned work"
    );
}
#[test]
fn rejects_nested_destinations_and_unregistered_or_changed_policy() {
    let h = Host::new("payload");
    let (code, r, _) = h.invoke("feature", "task", &h.root.join("nested"));
    assert_ne!(code, 0);
    assert!(r["error"].as_str().unwrap().contains("overlaps"));
    let mut p = fs::read(h.root.join(POLICY)).unwrap();
    p.push(b' ');
    fs::write(h.root.join(POLICY), &p).unwrap();
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert!(r["error"].as_str().unwrap().contains("source commit"));
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    fm["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["path"] != POLICY);
    fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert!(r["error"].as_str().unwrap().contains("FILEMAP"));
    assert!(!target.exists());
}
#[test]
fn remote_policy_change_requires_adoption_before_creating_a_branch() {
    let h = Host::new("payload");
    let other = h.parent.join("writer");
    git(
        &h.root,
        &[
            "clone",
            "-q",
            h.remote.to_str().unwrap(),
            other.to_str().unwrap(),
        ],
    );
    let p = other.join(POLICY);
    let mut b = fs::read(&p).unwrap();
    b.push(b'\n');
    fs::write(p, b).unwrap();
    commit(&other);
    git(&other, &["push", "-q", "origin", "dev"]);
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert!(
        r["error"]
            .as_str()
            .unwrap()
            .contains("fetched target changed")
    );
    assert!(!target.exists());
}
#[test]
fn retains_actual_failed_fetch_and_version_observations() {
    let h = Host::new("payload");
    h.policy(|p| p["remote"] = value!("missing-remote"));
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    let rows = r["processes"].as_array().unwrap();
    let last = rows.last().unwrap();
    assert_eq!(last["argv"][1], "fetch");
    assert_ne!(last["process"]["exit_code"], 0);
    assert!(
        !last["process"]["stderr_bytes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!target.exists());
    h.policy(|p| {
        p["remote"] = value!("warehouse");
        p["git"]["expected_version"] = value!("not the actual Git version");
    });
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert_eq!(r["error"], "Git version mismatch");
    assert!(r["processes"].as_array().unwrap().is_empty());
    assert_eq!(r["tool"]["version"]["exit_code"], 0);
    assert!(
        r["tool"]["version"]["stdout"]
            .as_str()
            .unwrap()
            .starts_with("git version ")
    );
}
#[test]
fn failed_checkout_hook_keeps_work_and_owned_lock_with_original_exit() {
    let h = Host::new("payload");
    h.hook(value!({"writes":[{"path":"recovery-file","contents":"recoverable\n"}],"stderr":"hook-failure\n","exit":37}));
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("integration", "task", &target);
    assert_ne!(code, 0, "{r}");
    assert_eq!(
        fs::read_to_string(target.join("recovery-file")).unwrap(),
        "recoverable\n"
    );
    assert_eq!(
        git(&target, &["symbolic-ref", "HEAD"]),
        "refs/heads/integration/task"
    );
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"])
            .contains(r["lock_reason"].as_str().unwrap())
    );
    let process = &r["processes"].as_array().unwrap().last().unwrap()["process"];
    assert_eq!(process["exit_code"], 37);
    assert!(process["stderr"].as_str().unwrap().contains("hook-failure"));
}
#[test]
fn successful_hook_with_dirty_checkout_cannot_report_created() {
    let h = Host::new("payload");
    h.hook(value!({"writes":[{"path":"payload","contents":"changed\n"}]}));
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0, "{r}");
    assert_eq!(r["status"], "failed");
    assert!(r["error"].as_str().unwrap().contains("clean snapshot"));
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "changed\n"
    );
}

#[test]
fn raw_checkout_creation_rejects_hidden_mode_from_successful_hook() {
    let h = Host::new("payload");
    git(&h.root, &["config", "core.filemode", "false"]);
    h.hook(value!({"executable":["payload"]}));
    let target = h.parent.join("hidden hook change");
    let (code, report, error) = h.invoke("feature", "hidden-hook", &target);
    assert_ne!(code, 0, "{report} {error}");
    assert!(report["error"].as_str().unwrap().contains("clean snapshot"));
    assert!(target.join("payload").exists());
    assert!(git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
}
#[test]
fn strict_cli_and_configuration_fail_without_creating_a_worktree() {
    let h = Host::new("payload");
    let target = h.parent.join("target");
    let (code, r, _) = h.invoke("unknown", "task", &target);
    assert_ne!(code, 0);
    assert!(r.is_null());
    h.policy(|p| p["inferred_layout"] = value!("forbidden"));
    let (code, r, _) = h.invoke("feature", "task", &target);
    assert_ne!(code, 0);
    assert!(r.is_null());
    assert!(!target.exists());
    assert_eq!(chrono_worktree::run(&["--version".into()]).exit_code, 0);
    assert_ne!(chrono_worktree::run(&["start".into()]).exit_code, 0);
}

#[test]
fn configured_digest_mismatch_does_not_execute_the_wrong_git() {
    let h = Host::new("payload");
    let marker = h.parent.join("must-not-exist");
    automatic::native_git(&h, "mark-executed", value!({}));
    h.policy(|p| p["git"]["sha256"] = value!("0".repeat(64)));
    let (code, report, error) = h.invoke("feature", "task", &h.parent.join("target"));
    assert_ne!(code, 0);
    assert!(report.is_null());
    assert!(error.contains("before version probe"));
    assert!(!marker.exists());
}

#[test]
fn adopted_policy_runs_the_actual_creation_consumer() {
    let h = Host::new("host files/no-language-contract.data");
    let adopted_bytes = fs::read(source().join(POLICY)).unwrap();
    let adopted = json(&adopted_bytes).unwrap();
    if let Some(path) = adopted["automatic_cleanup"].as_str() {
        let cleanup_bytes = fs::read(source().join(path)).unwrap();
        fs::write(h.root.join(path), &cleanup_bytes).unwrap();
        let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
        fm["files"]
            .as_array_mut()
            .unwrap()
            .push(file(path, value!([])));
        fs::write(h.root.join(FM), serde_json::to_vec(&fm).unwrap()).unwrap();
        let host_config = json(&fs::read(source().join(CONFIG)).unwrap()).unwrap();
        let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
        config["artifacts"] = host_config["artifacts"].clone();
        for artifact in config["artifacts"].as_array_mut().unwrap() {
            artifact["owner"] = value!("host");
        }
        fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
    }
    h.policy(|p| {
        // This independent host binds its own declared Git, not the product host's bytes.
        let git =
            chrono_harness::resolve_program(&h.root, p["git"]["program"].as_str().unwrap(), None)
                .unwrap();
        let version = Command::new(&git).arg("--version").output().unwrap();
        assert!(version.status.success());
        let environment = p["environment"].clone();
        *p = adopted;
        p["remote"] = value!("warehouse");
        p["environment"] = environment;
        p["git"]["program"] = value!(git);
        p["git"]["expected_version"] = value!(String::from_utf8(version.stdout).unwrap().trim());
        p["git"]["sha256"] = value!(sha256(&fs::read(&git).unwrap()));
    });
    let policy = json(&fs::read(h.root.join(POLICY)).unwrap()).unwrap();
    assert_eq!(
        json(git(&h.remote, &["show", &format!("dev:{POLICY}")]).as_bytes()).unwrap(),
        policy
    );
    let target = h.parent.join("adopted");
    let (code, report, error) = h.invoke("integration", "adopted", &target);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["tool"]["path"], policy["git"]["program"]);
    assert_eq!(report["tool"]["sha256"], policy["git"]["sha256"]);
    assert_eq!(report["tool"]["version"]["exit_code"], 0);
    assert_eq!(
        report["tool"]["version"]["stdout"].as_str().unwrap().trim(),
        policy["git"]["expected_version"].as_str().unwrap()
    );
    assert_eq!(
        git(&target, &["rev-parse", "HEAD"]),
        git(&h.root, &["rev-parse", "HEAD"])
    );
    assert_eq!(fs::read(source().join(POLICY)).unwrap(), adopted_bytes);
}

#[test]
fn registered_artifacts_from_checkout_hooks_are_preserved() {
    let h = Host::new("payload");
    h.hook(value!({"writes":[{"path":".chrono-harness/state/hook-result","contents":"generated\n","create_parents":true}]}));
    let target = h.parent.join("target");
    let (code, report, error) = h.invoke("feature", "task", &target);
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(
        fs::read_to_string(target.join(".chrono-harness/state/hook-result")).unwrap(),
        "generated\n"
    );
}

impl Host {
    fn reconstruct(&self, plan: Value, name: &str) -> (i32, Value, String) {
        let path = ".chrono-harness/state/reconstruction.json";
        fs::create_dir_all(self.root.join(".chrono-harness/state")).unwrap();
        fs::write(self.root.join(path), serde_json::to_vec(&plan).unwrap()).unwrap();
        let output = Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .current_dir("/")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args([
                "reconstruct",
                "--host-root",
                self.root.to_str().unwrap(),
                "--config",
                POLICY,
                "--kind",
                "integration",
                "--name",
                name,
                "--path",
                self.parent.join(name).to_str().unwrap(),
                "--plan",
                path,
            ])
            .output()
            .unwrap();
        self.received(output)
    }
    fn upstream(&self, path: &str, bytes: &[u8]) -> String {
        let other = self.parent.join("other writer");
        git(
            &self.root,
            &[
                "clone",
                "-q",
                self.remote.to_str().unwrap(),
                other.to_str().unwrap(),
            ],
        );
        fs::write(other.join(path), bytes).unwrap();
        let head = commit(&other);
        git(&other, &["push", "-q", "origin", "dev"]);
        head
    }
}
fn reconstruction(base: &str, candidate: &str, changes: Value) -> Value {
    value!({"schema":"chrono-worktree-reconstruction/v1", "base":base,"candidate":candidate,"changes":changes})
}

#[test]
fn selected_reconstruction_carries_explicit_work_onto_fetched_target() {
    let h = Host::new("payload");
    select_config(&h.root, SOURCE_CONFIG);
    let base = commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    fs::write(h.root.join("payload"), "carried work\n").unwrap();
    let candidate = commit(&h.root);
    let latest = h.upstream("remote-only", b"fetched work\n");
    let (code, report, error) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"carry"}]),
        ),
        "selected-reconstruction",
    );
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["status"], "reconstructed");
    assert_eq!(report["base"], latest);
    let expected = selected_values(&h.root, &latest, SOURCE_CONFIG, WORKFLOW);
    assert_eq!(
        report["registry_digest"],
        chrono_harness::wire::digest(&value!(expected)).unwrap()
    );
    assert_fixed_registry_reads(
        &report,
        &h.root,
        &h.root,
        &candidate,
        &selected_values(&h.root, &candidate, SOURCE_CONFIG, WORKFLOW),
    );
    let target = h.parent.join("selected-reconstruction");
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), latest);
    assert_eq!(fs::read(target.join("payload")).unwrap(), b"carried work\n");
    assert_eq!(
        fs::read(target.join("remote-only")).unwrap(),
        b"fetched work\n"
    );
    assert_eq!(
        report["reconstruction"]["staged_paths"],
        value!(["payload"])
    );
    assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), candidate);
}

#[test]
fn reconstructs_explicit_choices_on_advanced_target_without_merging_old_branch() {
    let h = Host::new("source.ts");
    fs::write(h.root.join("obsolete.data"), "original").unwrap();
    fs::write(h.root.join("remote-only"), "original").unwrap();
    let base = commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    git(&h.root, &["checkout", "-b", "feature/old"]);
    fs::write(h.root.join("source.ts"), "carried work").unwrap();
    fs::write(h.root.join("obsolete.data"), "retired work").unwrap();
    let candidate = commit(&h.root);
    let latest = h.upstream("remote-only", b"advanced target");
    let plan = reconstruction(
        &base,
        &candidate,
        value!([
            {"path":"source.ts","action":"carry"},
            {"path":"obsolete.data","action":"retire","reason":"superseded requirement"}
        ]),
    );
    let (code, r, error) = h.reconstruct(plan, "fresh");
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["status"], "reconstructed");
    assert_eq!(r["base"], latest);
    assert!(r["context"]["candidate"].is_null());
    let target = h.parent.join("fresh");
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), latest);
    assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), candidate);
    assert_eq!(git(&h.root, &["branch", "--show-current"]), "feature/old");
    assert_eq!(fs::read(target.join("source.ts")).unwrap(), b"carried work");
    assert_eq!(fs::read(target.join("obsolete.data")).unwrap(), b"original");
    assert_eq!(
        fs::read(target.join("remote-only")).unwrap(),
        b"advanced target"
    );
    assert_eq!(
        r["reconstruction"]["index_tree"],
        git(&target, &["write-tree"])
    );
    assert_eq!(r["reconstruction"]["staged_paths"], value!(["source.ts"]));
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    let apply = r["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["argv"].as_array().unwrap().contains(&value!("apply")))
        .unwrap();
    assert_eq!(
        apply["process"]["stdin_sha256"],
        r["reconstruction"]["patch_sha256"]
    );
}

#[test]
fn reconstruction_preserves_binary_modes_symlinks_adds_deletes_and_literal_paths() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    let h = Host::new("entry.go");
    for path in [
        "literal[1].bin",
        "literal1.bin",
        "removed",
        "executable",
        "alias",
    ] {
        fs::write(h.root.join(path), b"original\0bytes").unwrap();
    }
    let base = commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    fs::write(h.root.join("literal[1].bin"), b"new\0binary\xff").unwrap();
    fs::write(h.root.join("literal1.bin"), "retired").unwrap();
    fs::remove_file(h.root.join("removed")).unwrap();
    fs::remove_file(h.root.join("alias")).unwrap();
    symlink("entry.go", h.root.join("alias")).unwrap();
    fs::set_permissions(h.root.join("executable"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(h.root.join("added λ"), "new file").unwrap();
    let candidate = commit(&h.root);
    let mut changes: Vec<_> = [
        "literal[1].bin",
        "removed",
        "alias",
        "executable",
        "added λ",
    ]
    .iter()
    .map(|p| value!({"path":p,"action":"carry"}))
    .collect();
    changes.push(value!({"path":"literal1.bin","action":"retire","reason":"not selected"}));
    let (code, r, error) =
        h.reconstruct(reconstruction(&base, &candidate, value!(changes)), "binary");
    assert_eq!(code, 0, "{r} {error}");
    let target = h.parent.join("binary");
    assert_eq!(
        fs::read(target.join("literal[1].bin")).unwrap(),
        b"new\0binary\xff"
    );
    assert_eq!(
        fs::read(target.join("literal1.bin")).unwrap(),
        b"original\0bytes"
    );
    assert_eq!(
        fs::read_link(target.join("alias")).unwrap(),
        Path::new("entry.go")
    );
    // Git records executability, not group/other permission bits masked by umask.
    assert!(git(&target, &["ls-files", "--stage", "--", "executable"]).starts_with("100755 "));
    assert_eq!(
        fs::metadata(target.join("executable"))
            .unwrap()
            .permissions()
            .mode()
            & 0o100,
        0o100
    );
    assert!(!target.join("removed").exists());
    assert_eq!(fs::read(target.join("added λ")).unwrap(), b"new file");
}

#[test]
fn reconstruction_conflict_preserves_both_worktrees_original_exit_and_owned_lock() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("payload"), "local change\n").unwrap();
    let candidate = commit(&h.root);
    let latest = h.upstream("payload", b"remote change\n");
    let (code, r, _) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"carry"}]),
        ),
        "conflict",
    );
    assert_ne!(code, 0, "{r}");
    assert_eq!(r["status"], "failed");
    let target = h.parent.join("conflict");
    assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), candidate);
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), latest);
    assert_eq!(fs::read(h.root.join("payload")).unwrap(), b"local change\n");
    assert!(!git(&target, &["ls-files", "--unmerged"]).is_empty());
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"])
            .contains(r["lock_reason"].as_str().unwrap())
    );
    let process = &r["processes"].as_array().unwrap().last().unwrap()["process"];
    assert_eq!(process["exit_code"], 1);
    assert!(r["context"]["candidate"].is_null());
    assert!(r["reconstruction"].get("index_tree").is_none());
}

#[test]
fn invalid_reconstruction_choices_or_dirty_source_never_create_a_destination() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("payload"), "committed").unwrap();
    let candidate = commit(&h.root);
    let changes = [
        value!([]),
        value!([{"path":"foreign","action":"carry"}]),
        value!([{"path":"payload","action":"retire"}]),
        value!([{"path":"payload","action":"carry"},{"path":"payload","action":"carry"}]),
    ];
    for (n, change) in changes.into_iter().enumerate() {
        let name = format!("invalid-{n}");
        let (code, _, _) = h.reconstruct(reconstruction(&base, &candidate, change), &name);
        assert_ne!(code, 0);
        assert!(!h.parent.join(name).exists());
    }
    let valid = value!([{"path":"payload","action":"carry"}]);
    assert_ne!(
        h.reconstruct(
            reconstruction(&base, &base, valid.clone()),
            "wrong-candidate"
        )
        .0,
        0
    );
    assert!(!h.parent.join("wrong-candidate").exists());
    fs::write(h.root.join("payload"), "unsaved work").unwrap();
    assert_ne!(
        h.reconstruct(reconstruction(&base, &candidate, valid), "dirty")
            .0,
        0
    );
    assert!(!h.parent.join("dirty").exists());
    assert_eq!(fs::read(h.root.join("payload")).unwrap(), b"unsaved work");
}

#[test]
fn retired_only_reconstruction_keeps_new_target_without_applying_a_patch() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("payload"), "old work").unwrap();
    let candidate = commit(&h.root);
    let (code, r, error) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"retire","reason":"already unnecessary"}]),
        ),
        "retired",
    );
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["reconstruction"]["staged_paths"], value!([]));
    assert_eq!(
        r["reconstruction"]["index_tree"],
        git(&h.root, &["rev-parse", &format!("{base}^{{tree}}")])
    );
    assert!(
        !r["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["argv"].as_array().unwrap().contains(&value!("apply")))
    );
    assert_eq!(fs::read(h.root.join("payload")).unwrap(), b"old work");
}

#[test]
fn source_mutation_during_reconstruction_fails_and_preserves_recovery_work() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    fs::write(h.root.join("payload"), "old work").unwrap();
    let candidate = commit(&h.root);
    h.hook(value!({"writes":[{"path":h.parent.join("source with spaces/payload"),"contents":"concurrent-work\n"}]}));
    let (code, r, _) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"carry"}]),
        ),
        "mutated",
    );
    assert_ne!(code, 0, "{r}");
    assert_eq!(r["status"], "failed");
    assert_eq!(
        fs::read(h.root.join("payload")).unwrap(),
        b"concurrent-work\n"
    );
    assert!(h.parent.join("mutated").is_dir());
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"])
            .contains(r["lock_reason"].as_str().unwrap())
    );
}

#[test]
fn reconstruction_patch_bound_fails_before_creating_work() {
    let h = Host::new("payload");
    h.policy(|p| p["output_limit_bytes"] = value!(16384));
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    let data: String = (0..10000)
        .map(|n| format!("line {n:08} explicit reconstruction payload\n"))
        .collect();
    fs::write(h.root.join("payload"), &data).unwrap();
    let candidate = commit(&h.root);
    let (code, r, _) = h.reconstruct(
        reconstruction(
            &base,
            &candidate,
            value!([{"path":"payload","action":"carry"}]),
        ),
        "bounded",
    );
    assert_ne!(code, 0, "{r}");
    assert!(!h.parent.join("bounded").exists());
    assert_eq!(fs::read_to_string(h.root.join("payload")).unwrap(), data);
    let last = r["processes"].as_array().unwrap().last().unwrap();
    assert!(
        last["argv"]
            .as_array()
            .unwrap()
            .contains(&value!("--binary"))
    );
    assert!(last["process"]["failure"].is_string());
}

#[test]
fn empty_reconstruction_and_existing_destination_keep_explicit_boundaries() {
    let h = Host::new("payload");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    let plan = reconstruction(&base, &base, value!([]));
    let (code, r, error) = h.reconstruct(plan.clone(), "empty");
    assert_eq!(code, 0, "{r} {error}");
    assert_eq!(r["reconstruction"]["staged_paths"], value!([]));
    fs::write(h.parent.join("empty/sentinel"), "preserved").unwrap();
    assert_ne!(h.reconstruct(plan, "empty").0, 0);
    assert_eq!(
        fs::read(h.parent.join("empty/sentinel")).unwrap(),
        b"preserved"
    );
    assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), base);
}

#[test]
fn large_registered_artifacts_do_not_exhaust_cleanliness_output_bound() {
    let h = Host::new("payload");
    h.policy(|p| p["output_limit_bytes"] = value!(16384));
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    let directory = h.root.join(".chrono-harness/state/large-build");
    fs::create_dir_all(&directory).unwrap();
    for i in 0..512 {
        fs::write(
            directory.join(format!("{i:04}-{}", "artifact".repeat(8))),
            "retained",
        )
        .unwrap();
    }
    let old = Command::new("git")
        .current_dir(&h.root)
        .args([
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--ignored",
        ])
        .output()
        .unwrap();
    assert!(old.status.success() && old.stdout.len() > 16384);
    let (code, r, error) =
        h.reconstruct(reconstruction(&base, &base, value!([])), "large-artifacts");
    assert_eq!(code, 0, "{} {error}", r["error"]);
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 512);
    assert!(
        r["processes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["process"]["stdout_bytes"].as_array().unwrap().len() <= 16384)
    );
}

fn register_artifact(h: &Host, artifact: &str, ignore: &str) {
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["artifacts"]
        .as_array_mut()
        .unwrap()
        .push(value!({"path":artifact,"owner":"host","kind":"build","tracked":false}));
    fs::write(h.root.join(CONFIG), serde_json::to_vec(&config).unwrap()).unwrap();
    fs::write(
        h.root.join(".gitignore"),
        format!(".chrono-harness/state/\n{ignore}\n"),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
}

#[test]
fn artifact_exclusions_keep_ignored_neighbors_literal_lookalikes_and_untracked_files_visible() {
    let h = Host::new("payload");
    register_artifact(&h, "cache[1]/allowed/", "cache*/");
    register_artifact(&h, "CacheCase/allowed/", "cache*/\nCacheCase/");
    h.policy(|p| {
        p["environment"]["values"]["GIT_LITERAL_PATHSPECS"] = value!("1");
        p["environment"]["values"]["GIT_ICASE_PATHSPECS"] = value!("1");
    });
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    let allowed = h.root.join("cache[1]/allowed");
    fs::create_dir_all(&allowed).unwrap();
    fs::write(allowed.join("kept"), "declared").unwrap();
    for (i, path) in [
        "cache[1]/outside",
        "cache1/allowed/foreign",
        "cachecase/allowed/foreign",
        "untracked/outside",
    ]
    .iter()
    .enumerate()
    {
        let file = h.root.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "unregistered").unwrap();
        let name = format!("neighbor-{i}");
        let (code, r, _) = h.reconstruct(reconstruction(&base, &base, value!([])), &name);
        assert_ne!(code, 0);
        assert!(r["error"].as_str().unwrap().contains("unregistered"));
        assert!(!h.parent.join(name).exists());
        assert_eq!(fs::read(&file).unwrap(), b"unregistered");
        fs::remove_file(file).unwrap();
    }
    let (code, r, error) =
        h.reconstruct(reconstruction(&base, &base, value!([])), "allowed-literal");
    assert_eq!(code, 0, "{} {error}", r["error"]);
    assert_eq!(fs::read(allowed.join("kept")).unwrap(), b"declared");
}

#[test]
fn changing_ignore_rules_cannot_hide_unregistered_work_during_reconstruction() {
    let h = Host::new("payload");
    h.remote_git("ignore-race");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    let work = h.root.join("unregistered-work");
    fs::write(&work, "must remain visible and preserved").unwrap();
    let (code, report, error) =
        h.reconstruct(reconstruction(&base, &base, value!([])), "changing-ignore");
    assert_ne!(code, 0, "{} {error}", report["error"]);
    assert!(report["error"].as_str().unwrap().contains("unregistered"));
    assert!(!h.parent.join("changing-ignore").exists());
    assert_eq!(
        fs::read(&work).unwrap(),
        b"must remain visible and preserved"
    );

    fs::remove_file(&work).unwrap();
    let (code, report, error) = h.reconstruct(
        reconstruction(&base, &base, value!([])),
        "changing-ignore-clean",
    );
    assert_eq!(code, 0, "{} {error}", report["error"]);
}

#[test]
fn artifact_directory_names_do_not_hide_tracked_changes_files_or_symlinks() {
    use std::os::unix::fs::symlink;
    let h = Host::new("tree/input");
    register_artifact(&h, "tree/", "");
    register_artifact(&h, "declared-directory/", "");
    let base = git(&h.root, &["rev-parse", "HEAD"]);
    let original = fs::read(h.root.join("tree/input")).unwrap();
    fs::write(h.root.join("tree/input"), "tracked dirty work").unwrap();
    let (code, r, _) = h.reconstruct(reconstruction(&base, &base, value!([])), "tracked-dirty");
    assert_ne!(code, 0);
    assert!(r["error"].as_str().unwrap().contains("clean snapshot"));
    assert!(!h.parent.join("tracked-dirty").exists());
    fs::write(h.root.join("tree/input"), original).unwrap();
    for is_link in [false, true] {
        let path = h.root.join("declared-directory");
        if is_link {
            symlink("tree", &path).unwrap();
        } else {
            fs::write(&path, "not a directory").unwrap();
        }
        let name = if is_link {
            "artifact-link"
        } else {
            "artifact-file"
        };
        let (code, r, _) = h.reconstruct(reconstruction(&base, &base, value!([])), name);
        assert_ne!(code, 0);
        assert!(r["error"].as_str().unwrap().contains("unregistered"));
        assert!(!h.parent.join(name).exists());
        assert!(fs::symlink_metadata(&path).is_ok());
        fs::remove_file(path).unwrap();
    }
}
