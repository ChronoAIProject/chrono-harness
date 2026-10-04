use super::automatic::consuming_operation;
use super::*;
use std::{
    process::Stdio,
    thread,
    time::{Duration, Instant},
};
const AUTO_POLICY: &str = ".chrono-harness/cleanup.json";
impl Host {
    pub(super) fn kernel_cleanup(&self) {
        self.automatic("evidence-retain");
        let path = self.root.join(AUTO_POLICY);
        let mut policy = json(&fs::read(&path).unwrap()).unwrap();
        policy["schema"] = value!("chrono-worktree-automatic-cleanup/v2");
        fs::write(path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
        commit(&self.root);
        git(&self.root, &["push", "-q", "warehouse", "dev"]);
    }
}
fn output(target: &Path) {
    fs::create_dir_all(target.join("output λ")).unwrap();
    fs::write(target.join("output λ/cache"), "reproducible").unwrap();
}
#[test]
fn live_checkout_identity_failures_preserve_pending_cache_and_original_git_error() {
    use std::os::unix::fs::PermissionsExt;
    let h = Host::new("payload");
    h.kernel_cleanup();
    let real = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let script = h.parent.join("identity-git");
    fs::write(
        &script,
        format!(
            r#"#!/bin/sh
if [ -f "$HOME/identity-fault" ] && [ "$PWD" = "$HOME/observed" ] && [ "$2" = rev-parse ] && [ "$3" = --show-toplevel ] && [ "$4" = --git-common-dir ]; then
    case "$(cat "$HOME/identity-fault")" in
        truncated) printf '%s\n' "$PWD"; exit 0 ;;
        malformed-head) printf '%s\n' "$PWD" "$HOME/source with spaces/.git" "$('{}' rev-parse --absolute-git-dir)" not-an-oid refs/heads/feature/observed; exit 0 ;;
        foreign-common) printf '%s\n' "$PWD" "$HOME/foreign-common" "$('{}' rev-parse --absolute-git-dir)" "$('{}' rev-parse HEAD)" refs/heads/feature/observed; exit 0 ;;
        foreign-branch) '{}' "$@" | /usr/bin/sed 's@refs/heads/feature/observed@refs/heads/feature/other@'; exit 0 ;;
        foreign-metadata) '{}' "$@" | /usr/bin/sed "s@.*/worktrees/observed@$HOME/foreign-common@"; exit 0 ;;
        failure) printf 'original identity failure\n' >&2; exit 71 ;;
    esac
fi
exec '{}' "$@"
"#,
            real.display(),
            real.display(),
            real.display(),
            real.display(),
            real.display(),
            real.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    h.policy(|p| p["git"]["program"] = value!(script));
    let target = h.parent.join("observed");
    assert_eq!(h.invoke("feature", "observed", &target).0, 0);
    output(&target);
    fs::create_dir(h.parent.join("foreign-common")).unwrap();
    let source = fs::read(target.join("payload")).unwrap();
    for fault in [
        "truncated",
        "malformed-head",
        "foreign-common",
        "foreign-branch",
        "foreign-metadata",
        "failure",
    ] {
        fs::write(h.parent.join("identity-fault"), fault).unwrap();
        let (code, report, error) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{fault}: {report} {error}");
        assert!(target.join("output λ/cache").exists(), "{fault}");
        assert_eq!(fs::read(target.join("payload")).unwrap(), source);
        assert_eq!(h.ledger()["entries"][0]["ownership"]["cache_pending"], true);
        if fault == "failure" {
            let processes = report["drain"][0]["report"]["processes"]
                .as_array()
                .unwrap();
            let original = &processes.last().unwrap()["process"];
            assert_eq!(original["exit_code"], 71);
            assert_eq!(original["stderr"], "original identity failure\n");
        }
    }
    fs::remove_file(h.parent.join("identity-fault")).unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!target.join("output λ").exists());
    assert_eq!(fs::read(target.join("payload")).unwrap(), source);
}
fn participating_check(h: &Host) {
    super::check_inputs::bind(h);
    let mut cfg = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    cfg["runner"]["path"] = value!(".chrono-harness/bin/inner-check");
    cfg["canonical_check"]["argv"] = value!([".chrono-harness/bin/inner-check", "check"]);
    cfg["canonical_check"]["participation"] =
        value!({"operation":"worktree.check","tool":"chrono-worktree","argv":["check"]});
    cfg["tools"].as_array_mut().unwrap().push(value!({"id":"sh","program":"/bin/sh","resolution":"PATH-once","version_argv":["-c","printf fixture"],"expected_version":"fixture"}));
    fs::write(
        h.root.join(CONFIG),
        serde_json::to_vec_pretty(&cfg).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
}
fn install_inner(root: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    super::check_inputs::install(root);
    let path = root.join(".chrono-harness/bin/inner-check");
    fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
#[test]
fn public_participating_check_preserves_zero_and_nonzero_stderr_results() {
    for exit in [0, 7] {
        let h = Host::new("payload");
        participating_check(&h);
        h.kernel_cleanup();
        let target = h.parent.join("console");
        assert_eq!(h.invoke("feature", "console", &target).0, 0);
        install_inner(
            &target,
            &format!(
                "printf 'original stdout\\n'; printf 'original diagnostic\\n' >&2; exit {exit}\n"
            ),
        );
        let out = Command::new(target.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&target)
            .arg("check")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(exit), "{out:?}");
        assert_eq!(out.stdout, b"original stdout\n");
        assert_eq!(out.stderr, b"original diagnostic\n");
        assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
        let (_, owner, _) = h.received(
            Command::new(target.join(".chrono-harness/bin/chrono-worktree"))
                .current_dir(&target)
                .args(["check", "--config", POLICY])
                .output()
                .unwrap(),
        );
        assert_eq!(owner["managed_process"]["exit_code"], exit);
        assert_eq!(
            owner["managed_process"]["stdout_bytes"],
            value!(b"original stdout\n".to_vec())
        );
        assert_eq!(
            owner["managed_process"]["stderr_bytes"],
            value!(b"original diagnostic\n".to_vec())
        );
        // Target policy failure is a lifecycle refusal, distinct from diagnostics.
        fs::write(target.join(AUTO_POLICY), "ordinary policy edit").unwrap();
        let refused = Command::new(target.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&target)
            .arg("check")
            .output()
            .unwrap();
        assert_eq!(refused.status.code(), Some(2), "{refused:?}");
        assert!(String::from_utf8_lossy(&refused.stderr).contains("participation refused"));
        assert!(refused.stdout.is_empty());
    }
}
#[test]
fn unrelated_policy_drift_retains_failure_and_allows_independent_admission() {
    let h = Host::new("payload");
    participating_check(&h);
    h.kernel_cleanup();
    consuming_operation(&h, "printf ordinary-work\n");
    let a = h.parent.join("drifted-a");
    let b = h.parent.join("valid-b");
    assert_eq!(h.invoke("feature", "drifted-a", &a).0, 0);
    output(&a);
    fs::write(a.join(AUTO_POLICY), "ordinary uncommitted policy edit").unwrap();
    let (code, start, error) = h.invoke("feature", "valid-b", &b);
    assert_eq!(code, 0, "{start} {error}");
    assert!(!start["cleanup_failures"].as_array().unwrap().is_empty());
    let receipt = start["drain"][0]["receipt"].clone();
    let receipt_path = h.root.join(receipt["path"].as_str().unwrap());
    let original = fs::read(&receipt_path).unwrap();
    assert_eq!(sha256(&original), receipt["sha256"]);
    install_inner(&b, "printf 'check warning\\n' >&2; printf checked\n");
    let (code, used, error) = h.auto(
        "use",
        &["--path", b.to_str().unwrap(), "--operation", "use.consumer"],
    );
    assert_eq!(code, 0, "{used} {error}");
    assert_eq!(used["managed_process"]["stdout"], "ordinary-work");
    let checked = Command::new(b.join(".chrono-harness/bin/chrono-harness"))
        .current_dir(&b)
        .arg("check")
        .output()
        .unwrap();
    assert_eq!(checked.status.code(), Some(0), "{checked:?}");
    assert_eq!(checked.stdout, b"checked");
    assert_eq!(checked.stderr, b"check warning\n");
    let head = git(&h.root, &["rev-parse", "HEAD"]);
    let (code, rebuilt, error) = h.reconstruct(
        reconstruction(&head, &head, value!([])),
        "independent-reconstruct",
    );
    assert_eq!(code, 0, "{} {error}", rebuilt["error"]);
    assert!(!rebuilt["cleanup_failures"].as_array().unwrap().is_empty());
    let (code, maintained, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{maintained} {error}");
    assert!(a.join("output λ/cache").exists());
    assert_eq!(fs::read(receipt_path).unwrap(), original);
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    let blocked = h.auto(
        "use",
        &["--path", a.to_str().unwrap(), "--operation", "use.consumer"],
    );
    assert_ne!(blocked.0, 0, "{blocked:?}");
    assert!(blocked.1["managed_process"].is_null());
}
fn await_file(path: &Path) {
    let began = Instant::now();
    while !path.exists() && began.elapsed() < Duration::from_secs(10) {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(path.exists(), "missing handshake: {}", path.display());
}
#[test]
fn successful_use_without_finish_reclaims_caches_and_preserves_active_work() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    consuming_operation(
        &h,
        "mkdir -p 'output λ' .chrono-harness/state\nprintf rebuilt > 'output λ/cache'\nprintf evidence > .chrono-harness/state/evidence\n",
    );
    let target = h.parent.join("resumable");
    assert_eq!(h.invoke("feature", "resumable", &target).0, 0);
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    fs::write(target.join("payload"), "unsaved source").unwrap();
    let refs = git(&h.root, &["show-ref"]);
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(
        !target.join("output λ").exists(),
        "no-token session loss must reclaim quiescent caches"
    );
    assert_eq!(
        fs::read_to_string(target.join("payload")).unwrap(),
        "unsaved source"
    );
    assert_eq!(
        fs::read_to_string(target.join(".chrono-harness/state/evidence")).unwrap(),
        "evidence"
    );
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    let ledger = h.ledger();
    assert_eq!(ledger["entries"][0]["status"], "active");
    assert!(ledger["entries"][0]["terminal"].is_null());
    assert_eq!(ledger["entries"][0]["uses"], value!([]));
    assert_eq!(
        h.auto("maintain", &[]).1["drain"],
        value!([]),
        "unchanged generation must not be disposed again"
    );
}

struct OwnedGroup(String);
impl Drop for OwnedGroup {
    fn drop(&mut self) {
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{}", self.0)])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}
pub(super) struct CapturedChild {
    child: std::process::Child,
    stdout: PathBuf,
    stderr: PathBuf,
    joined: bool,
}
impl CapturedChild {
    pub(super) fn spawn(command: &mut Command, root: &Path) -> Self {
        static NUMBER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NUMBER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let state = root.join(".chrono-harness/state/child-captures");
        fs::create_dir_all(&state).unwrap();
        let stdout = state.join(format!("{id}.stdout"));
        let stderr = state.join(format!("{id}.stderr"));
        let child = command
            .stdout(fs::File::create(&stdout).unwrap())
            .stderr(fs::File::create(&stderr).unwrap())
            .spawn()
            .unwrap();
        fs::write(
            state.join(format!("{id}.binding.json")),
            serde_json::to_vec_pretty(&value!({"pid":child.id(),"command":format!("{command:?}")}))
                .unwrap(),
        )
        .unwrap();
        Self {
            child,
            stdout,
            stderr,
            joined: false,
        }
    }
    pub(super) fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }
    fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        let status = self.child.wait()?;
        self.joined = true;
        fs::write(
            self.stdout.with_extension("joined.json"),
            serde_json::to_vec(
                &value!({"pid":self.child.id(),"status":format!("{status:?}"),"joined":true}),
            )
            .unwrap(),
        )?;
        Ok(status)
    }
    pub(super) fn wait_with_output(mut self) -> std::io::Result<std::process::Output> {
        let status = self.wait()?;
        Ok(std::process::Output {
            status,
            stdout: fs::read(&self.stdout)?,
            stderr: fs::read(&self.stderr)?,
        })
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn record_wait_state(&self, watched: &Path) {
        // Observe this fixture's declared child before teardown. These reads are
        // failure diagnostics, never a cleanup or completion decision.
        let mut pending = vec![self.child.id()];
        let mut seen = std::collections::BTreeSet::new();
        let mut processes = Vec::new();
        while let Some(pid) = pending.pop() {
            if seen.len() >= 128 || !seen.insert(pid) {
                continue;
            }
            #[cfg(target_os = "linux")]
            {
                let root = PathBuf::from(format!("/proc/{pid}"));
                let read = |name: &str| match fs::read(root.join(name)) {
                    Ok(mut bytes) => {
                        bytes.truncate(4096);
                        value!({"bytes": String::from_utf8_lossy(&bytes).replace('\0', " ")})
                    }
                    Err(error) => value!({"error": error.to_string()}),
                };
                let before = read("stat");
                let mut children = Vec::new();
                if let Ok(tasks) = fs::read_dir(root.join("task")) {
                    for task in tasks.flatten() {
                        if let Ok(ids) = fs::read_to_string(task.path().join("children")) {
                            children.extend(
                                ids.split_whitespace()
                                    .filter_map(|id| id.parse::<u32>().ok()),
                            );
                        }
                    }
                }
                pending.extend(children.iter().copied());
                processes.push(value!({"pid":pid,"stat_before":before,"command":read("cmdline"),"wait_channel":read("wchan"),"children":children,"stat_after":read("stat")}));
            }
            #[cfg(target_os = "macos")]
            {
                let observe = |program: &str, args: &[&str]| match Command::new(program)
                    .args(args)
                    .output()
                {
                    Ok(out) => {
                        value!({"exit":out.status.code(),"stdout":String::from_utf8_lossy(&out.stdout),"stderr":String::from_utf8_lossy(&out.stderr)})
                    }
                    Err(error) => value!({"error":error.to_string()}),
                };
                let pid_text = pid.to_string();
                let state = observe(
                    "/bin/ps",
                    &[
                        "-p",
                        &pid_text,
                        "-o",
                        "pid=,ppid=,state=,etime=,time=,command=",
                    ],
                );
                let children = observe("/usr/bin/pgrep", &["-P", &pid_text]);
                pending.extend(
                    children["stdout"]
                        .as_str()
                        .unwrap_or("")
                        .split_whitespace()
                        .filter_map(|pid| pid.parse::<u32>().ok()),
                );
                processes.push(value!({"pid":pid,"state":state,"children":children}));
            }
        }
        let parent = watched.parent().unwrap();
        let markers: Vec<_> = [
            "cargo-wrapper",
            "native-pid",
            "native-holder",
            "check-holder",
        ]
        .into_iter()
        .map(|name| {
            let result = fs::read_to_string(parent.join(name));
            value!({"name":name,"result":result.map_err(|e| e.to_string())})
        })
        .collect();
        eprintln!(
            "CAPTURED_CHILD_WAIT_STATE {}",
            value!({"watched":watched,"processes":processes,"markers":markers})
        );
    }
    pub(super) fn await_file(&mut self, path: &Path) {
        let began = Instant::now();
        while !path.exists() && began.elapsed() < Duration::from_secs(10) {
            if self.try_wait().unwrap().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if !path.exists() {
            let before_kill = self.try_wait().unwrap();
            if before_kill.is_none() {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                self.record_wait_state(path);
                let _ = self.kill();
            }
            let status = self.wait().unwrap();
            panic!(
                "missing handshake {}; before test kill {before_kill:?}; joined {status:?}; stdout {}; stderr {}; captures {} {}",
                path.display(),
                String::from_utf8_lossy(&fs::read(&self.stdout).unwrap()),
                String::from_utf8_lossy(&fs::read(&self.stderr).unwrap()),
                self.stdout.display(),
                self.stderr.display()
            );
        }
    }
}
impl Drop for CapturedChild {
    fn drop(&mut self) {
        if !self.joined {
            let _ = self.kill();
            let status = self.wait();
            eprintln!(
                "joined fixture child after test exit: {status:?}; stdout {}; stderr {}",
                String::from_utf8_lossy(&fs::read(&self.stdout).unwrap()),
                String::from_utf8_lossy(&fs::read(&self.stderr).unwrap())
            );
        }
    }
}
fn wrapper(h: &Host, target: &Path) -> CapturedChild {
    CapturedChild::spawn(
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
    )
}
fn reclaim(h: &Host, target: &Path) -> Value {
    let began = Instant::now();
    loop {
        let (code, report, error) = h.auto("maintain", &[]);
        assert_eq!(code, 0, "{report} {error}");
        if !target.join("output λ").exists() {
            return report;
        }
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "final holder did not release: {report}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn killed_wrapper_and_parent_preserve_live_grandchild_until_final_close() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    consuming_operation(
        &h,
        r#"mkdir -p .chrono-harness/state
sh -c 'printf "%s" "$$" > .chrono-harness/state/grandchild; while [ ! -f .chrono-harness/state/release-grandchild ]; do sleep 0.02; done; printf done > .chrono-harness/state/grandchild-done' </dev/null >/dev/null 2>&1 &
printf '%s' "$$" > .chrono-harness/state/parent
while [ ! -f .chrono-harness/state/release-parent ]; do sleep 0.02; done
printf done > .chrono-harness/state/parent-done
"#,
    );
    let target = h.parent.join("descendants");
    assert_eq!(h.invoke("feature", "descendants", &target).0, 0);
    output(&target);
    let mut child = wrapper(&h, &target);
    child.await_file(&target.join(".chrono-harness/state/parent"));
    await_file(&target.join(".chrono-harness/state/grandchild"));
    let _group =
        OwnedGroup(fs::read_to_string(target.join(".chrono-harness/state/parent")).unwrap());
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(target.join("output λ/cache").exists());
    assert!(
        report["drain"][0]["preserved_reason"]
            .as_str()
            .unwrap()
            .contains("live registered")
    );
    fs::write(target.join(".chrono-harness/state/release-parent"), "exit").unwrap();
    await_file(&target.join(".chrono-harness/state/parent-done"));
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(
        target.join("output λ/cache").exists(),
        "grandchild must retain the inherited description"
    );
    assert_ne!(h.auto("finish", &["--path", target.to_str().unwrap()]).0, 0);
    fs::write(
        target.join(".chrono-harness/state/release-grandchild"),
        "exit",
    )
    .unwrap();
    await_file(&target.join(".chrono-harness/state/grandchild-done"));
    reclaim(&h, &target);
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    assert_eq!(h.ledger()["entries"][0]["status"], "active");
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    let receipt = &h.ledger()["entries"][0]["enrollment"]["interrupted_uses"][0];
    let interrupted =
        json(&fs::read(h.root.join(receipt["path"].as_str().unwrap())).unwrap()).unwrap();
    assert_eq!(interrupted["original_result"]["presence"], "absent");
    assert_eq!(interrupted["task_completion"], "not-claimed");
}

#[test]
fn one_dead_wrapper_and_one_live_use_keep_shared_protection() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    consuming_operation(
        &h,
        "mkdir -p .chrono-harness/state\nprintf '%s' \"$$\" > .chrono-harness/state/ready-$$\nwhile [ ! -f .chrono-harness/state/release ]; do sleep 0.02; done\nprintf completed\n",
    );
    let target = h.parent.join("concurrent");
    assert_eq!(h.invoke("feature", "concurrent", &target).0, 0);
    output(&target);
    let mut first = wrapper(&h, &target);
    let second = CapturedChild::spawn(
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
    let began = Instant::now();
    while h.ledger()["entries"][0]["uses"].as_array().unwrap().len() != 2
        && began.elapsed() < Duration::from_secs(10)
    {
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        h.ledger()["entries"][0]["uses"].as_array().unwrap().len(),
        2
    );
    let state = target.join(".chrono-harness/state");
    while fs::read_dir(&state)
        .map(|rows| {
            rows.filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().starts_with("ready-"))
                .count()
        })
        .unwrap_or(0)
        != 2
        && began.elapsed() < Duration::from_secs(10)
    {
        thread::sleep(Duration::from_millis(10));
    }
    let _groups: Vec<_> = fs::read_dir(&state)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("ready-"))
        .map(|e| OwnedGroup(fs::read_to_string(e.path()).unwrap()))
        .collect();
    assert_eq!(_groups.len(), 2);
    first.kill().unwrap();
    first.wait().unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/cache").exists());
    fs::write(state.join("release"), "joined").unwrap();
    let (code, report, error) = h.received(second.wait_with_output().unwrap());
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(report["managed_process"]["stdout"], "completed");
    reclaim(&h, &target);
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
}

#[test]
fn interrupted_partial_cache_disposal_retries_without_owned_git_lock_or_finish() {
    use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};
    let h = Host::new("payload");
    h.kernel_cleanup();
    let real = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let script = h.parent.join("cache-git");
    fs::write(&script,format!("#!/bin/sh\nif [ -f \"$HOME/interrupt-cache\" ] && [ ! -d \"$HOME/partial/output λ\" ]; then\nprintf interrupted > \"$HOME/cache-observed\"\nkill -KILL \"$PPID\"\nfi\nexec '{}' \"$@\"\n",real.display())).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    h.policy(|p| p["git"]["program"] = value!(script));
    let target = h.parent.join("partial");
    assert_eq!(h.invoke("feature", "partial", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(target.join("nested/cache/data"), "second output").unwrap();
    fs::write(
        h.parent.join("interrupt-cache"),
        "interrupt after first effect",
    )
    .unwrap();
    let out = h.auto_command("maintain", &[]).output().unwrap();
    assert_eq!(out.status.signal(), Some(9), "{out:?}");
    assert!(!target.join("output λ").exists());
    assert!(target.join("nested/cache/data").exists());
    assert!(!git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"));
    let entry = h.ledger()["entries"][0].clone();
    let attempt = &entry["cache_attempts"][0];
    assert!(attempt["receipt"].is_null());
    let intent_path = h.root.join(attempt["intent"]["path"].as_str().unwrap());
    let intent_bytes = fs::read(&intent_path).unwrap();
    let result_path = h.root.join(attempt["path"].as_str().unwrap());
    let original = fs::read(&result_path).ok();
    fs::remove_file(h.parent.join("interrupt-cache")).unwrap();
    let (code, report, error) = h.auto("maintain", &[]);
    assert_eq!(code, 0, "{report} {error}");
    assert!(!target.join("nested/cache").exists());
    assert_eq!(fs::read(intent_path).unwrap(), intent_bytes);
    assert_eq!(
        fs::read(result_path).ok(),
        original,
        "original missing/partial result must be unchanged"
    );
    assert_eq!(
        report["drain"][0]["report"]["artifact_disposals"][0]["status"],
        "already-absent"
    );
    assert_eq!(h.ledger()["entries"][0]["status"], "active");
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    assert_eq!(
        h.ledger()["entries"][0]["cache_attempts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn cache_retries_reference_failed_and_partial_originals_without_growth() {
    use std::os::unix::fs::PermissionsExt;
    let h = Host::new("payload");
    h.kernel_cleanup();
    let real = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let script = h.parent.join("cache-git");
    fs::write(&script, format!("#!/bin/sh\nif [ -f \"$HOME/fail-cache\" ] && [ ! -d \"$HOME/cache-retry/output λ\" ]; then\nprintf 'original cache failure\\n' >&2\nexit 71\nfi\nexec '{}' \"$@\"\n", real.display())).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    h.policy(|p| p["git"]["program"] = value!(script));
    let target = h.parent.join("cache-retry");
    assert_eq!(h.invoke("feature", "cache-retry", &target).0, 0);
    fs::write(h.parent.join("fail-cache"), "fail after disposal").unwrap();
    let mut originals = Vec::new();
    for retry in 0..4 {
        output(&target);
        let (code, report, error) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{report} {error}");
        let report = &report["drain"][0]["report"];
        assert_eq!(report["status"], "failed");
        assert!(report["processes"].as_array().unwrap().iter().any(|p| {
            p["process"]["exit_code"] == 71 && p["process"]["stderr"] == "original cache failure\n"
        }));
        let path = h.root.join(report["report_path"].as_str().unwrap());
        let bytes = fs::read(&path).unwrap();
        if retry > 0 {
            let input = &report["prior_cache_attempt"]["result"]["input"];
            assert_eq!(input["schema"], "chrono-worktree-retained-input/v1");
            let retained = fs::read(h.root.join(input["path"].as_str().unwrap())).unwrap();
            let (_, previous): &(PathBuf, Vec<u8>) = originals.last().unwrap();
            assert_eq!(&retained, previous);
            assert_eq!(input["sha256"], sha256(&retained));
            assert_eq!(input["byte_length"], retained.len());
            assert!(
                bytes.len() <= originals[0].1.len() * 3,
                "recursive cache report growth"
            );
        }
        originals.push((path, bytes));
    }
    // Model a torn, unpublished result of the explicitly registered last attempt.
    let mut ledger = h.ledger();
    let latest = ledger["entries"][0]["cache_attempts"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap();
    latest["receipt"] = Value::Null;
    let path = originals.last().unwrap().0.clone();
    let partial = b"{\"status\":\"failed\",\"processes\":";
    fs::write(&path, partial).unwrap();
    fs::write(
        h.root
            .join(".chrono-harness/state/automatic-cleanup/ledger.json"),
        serde_json::to_vec_pretty(&ledger).unwrap(),
    )
    .unwrap();
    output(&target);
    let (code, failed, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{failed} {error}");
    let original = &failed["drain"][0]["report"]["prior_cache_attempt"]["result"];
    assert_eq!(original["original_outcome"], "unknown");
    let input = &original["input"];
    assert_eq!(input["format"], "bytes");
    let retained_path = h.root.join(input["path"].as_str().unwrap());
    assert_ne!(retained_path, path);
    assert_eq!(fs::read(&retained_path).unwrap(), partial);
    assert_eq!(input["sha256"], sha256(partial));
    assert_eq!(input["byte_length"], partial.len());
    fs::write(&path, "source subsequently changed").unwrap();
    fs::remove_file(h.parent.join("fail-cache")).unwrap();
    output(&target);
    fs::write(&retained_path, "damaged snapshot").unwrap();
    let (code, refused, error) = h.auto("maintain", &[]);
    assert_ne!(code, 0, "{refused} {error}");
    assert!(target.join("output λ/cache").exists());
    fs::write(&retained_path, partial).unwrap();
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(!target.join("output λ").exists());
    assert!(target.exists());
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    assert_eq!(fs::read(&path).unwrap(), b"source subsequently changed");
    for (path, bytes) in &originals[..originals.len() - 1] {
        assert_eq!(fs::read(path).unwrap(), *bytes);
    }
}

#[test]
fn interrupted_cache_generation_is_superseded_by_admitted_rebuilding_after_source_commit() {
    use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};
    let h = Host::new("payload");
    h.kernel_cleanup();
    consuming_operation(
        &h,
        "mkdir -p 'output λ'; printf rebuilt > 'output λ/cache'\n",
    );
    let real = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let script = h.parent.join("supersession-git");
    fs::write(&script, format!("#!/bin/sh\nif [ -f \"$HOME/interrupt-cache\" ] && [ ! -d \"$HOME/superseded/output λ\" ]; then\nkill -KILL \"$PPID\"\nfi\nexec '{}' \"$@\"\n", real.display())).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    h.policy(|p| p["git"]["program"] = value!(script));
    let target = h.parent.join("superseded");
    assert_eq!(h.invoke("feature", "superseded", &target).0, 0);
    output(&target);
    fs::create_dir_all(target.join("nested/cache")).unwrap();
    fs::write(target.join("nested/cache/data"), "partial cleanup").unwrap();
    fs::write(h.parent.join("interrupt-cache"), "interrupt").unwrap();
    let interrupted = CapturedChild::spawn(&mut h.auto_command("maintain", &[]), &h.root)
        .wait_with_output()
        .unwrap();
    assert_eq!(interrupted.status.signal(), Some(9), "{interrupted:?}");
    h.received(interrupted);
    assert!(!target.join("output λ").exists());
    assert!(target.join("nested/cache/data").exists());
    let old = h.ledger()["entries"][0]["cache_attempts"][0].clone();
    let intent_path = h.root.join(old["intent"]["path"].as_str().unwrap());
    let result_path = h.root.join(old["path"].as_str().unwrap());
    let intent = fs::read(&intent_path).unwrap();
    let result = fs::read(&result_path).ok();
    fs::remove_file(h.parent.join("interrupt-cache")).unwrap();
    fs::write(target.join("payload"), "ordinary source commit").unwrap();
    let head = commit(&target);
    let refs = git(&h.root, &["show-ref"]);
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    assert!(target.join("output λ/cache").exists());
    reclaim(&h, &target);
    let entry = h.ledger()["entries"][0].clone();
    assert_eq!(entry["ownership"]["generation"], 2);
    assert_eq!(entry["cache_attempts"][0], old);
    assert_eq!(fs::read(intent_path).unwrap(), intent);
    assert_eq!(fs::read(result_path).ok(), result);
    assert_eq!(git(&target, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&h.root, &["show-ref"]), refs);
    assert_eq!(entry["status"], "active");
    assert!(entry["terminal"].is_null());
}

#[test]
fn persisted_unsealed_birth_recovers_through_normal_use_after_actual_interruption() {
    use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};
    let h = Host::new("payload");
    h.kernel_cleanup();
    consuming_operation(
        &h,
        "mkdir -p 'output λ'; printf rebuilt > 'output λ/cache'\n",
    );
    let real = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let program = h.parent.join("birth-git");
    let ledger = h
        .root
        .join(".chrono-harness/state/automatic-cleanup/ledger.json");
    fs::write(&program, format!("#!/bin/sh\nif [ -f \"$HOME/interrupt-birth\" ] && [ -f '{}' ] && /usr/bin/grep -q '\"kind\": \"birth\"' '{}'; then\nkill -KILL \"$PPID\"\nfi\nexec '{}' \"$@\"\n", ledger.display(), ledger.display(), real.display())).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    h.policy(|p| p["git"]["program"] = value!(program));
    fs::write(h.parent.join("interrupt-birth"), "after durable enrollment").unwrap();
    let target = h.parent.join("unsealed");
    let interrupted = CapturedChild::spawn(
        Command::new(source().join("crates/worktree/target/debug/chrono-worktree")).args([
            "start",
            "--host-root",
            h.root.to_str().unwrap(),
            "--config",
            POLICY,
            "--kind",
            "feature",
            "--name",
            "unsealed",
            "--path",
            target.to_str().unwrap(),
        ]),
        &h.root,
    )
    .wait_with_output()
    .unwrap();
    assert_eq!(interrupted.status.signal(), Some(9), "{interrupted:?}");
    h.received(interrupted);
    let original = h.ledger()["entries"][0].clone();
    assert!(original["enrollment"]["sealed_receipt"].is_null());
    fs::remove_file(h.parent.join("interrupt-birth")).unwrap();
    // Refusal still validates the original attachment/policy before recovery.
    fs::write(target.join(AUTO_POLICY), "unsaved policy").unwrap();
    let (code, _, _) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_ne!(code, 0);
    assert!(h.ledger()["entries"][0]["enrollment"]["recovered_receipt"].is_null());
    fs::copy(h.root.join(AUTO_POLICY), target.join(AUTO_POLICY)).unwrap();
    let recovery_path = h.root.join(format!(
        ".chrono-harness/state/automatic-cleanup/interrupted-birth-{}.json",
        sha256(
            original["enrollment"]["receipt"]["report_path"]
                .as_str()
                .unwrap()
                .as_bytes()
        )
    ));
    fs::write(&program, format!("#!/bin/sh\nif [ -f \"$HOME/interrupt-recovery\" ] && [ -f '{}' ]; then\nkill -KILL \"$PPID\"\nfi\nexec '{}' \"$@\"\n", recovery_path.display(), real.display())).unwrap();
    fs::write(
        h.parent.join("interrupt-recovery"),
        "after durable reconciliation",
    )
    .unwrap();
    let again = CapturedChild::spawn(
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
    )
    .wait_with_output()
    .unwrap();
    assert_eq!(again.status.signal(), Some(9), "{again:?}");
    h.received(again);
    assert!(h.ledger()["entries"][0]["enrollment"]["recovered_receipt"].is_null());
    let interrupted_recovery = fs::read(&recovery_path).unwrap();
    fs::remove_file(h.parent.join("interrupt-recovery")).unwrap();
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    let recovered = h.ledger()["entries"][0].clone();
    assert!(recovered["enrollment"]["sealed_receipt"].is_null());
    assert_eq!(
        recovered["enrollment"]["receipt"],
        original["enrollment"]["receipt"]
    );
    assert!(recovered["enrollment"]["recovered_receipt"].is_object());
    let recovery_path = h.root.join(
        recovered["enrollment"]["recovered_receipt"]["path"]
            .as_str()
            .unwrap(),
    );
    let bytes = fs::read(&recovery_path).unwrap();
    assert_eq!(bytes, interrupted_recovery);
    let recovery = json(&bytes).unwrap();
    assert_eq!(recovery["original_outcome"], "not-established");
    assert_eq!(recovery["terminal_handoff"], "not-claimed");
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    assert_eq!(fs::read(recovery_path).unwrap(), bytes);
    reclaim(&h, &target);
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
}

#[test]
fn kernel_caches_preserve_staged_source_unknown_lease_drift_and_foreign_locks() {
    use std::os::unix::fs::symlink;
    for fault in [
        "staged",
        "missing-lease",
        "replaced-lease",
        "foreign-lock",
        "policy-drift",
        "symlink",
    ] {
        let h = Host::new("payload");
        h.kernel_cleanup();
        let target = h.parent.join("protected");
        assert_eq!(h.invoke("feature", "protected", &target).0, 0);
        output(&target);
        let lease = h.root.join(
            h.ledger()["entries"][0]["ownership"]["path"]
                .as_str()
                .unwrap(),
        );
        match fault {
            "staged" => {
                fs::write(target.join("output λ/source"), "staged source").unwrap();
                git(&target, &["add", "-f", "output λ/source"]);
            }
            "missing-lease" => {
                fs::remove_file(&lease).unwrap();
            }
            "replaced-lease" => {
                fs::rename(&lease, lease.with_extension("original")).unwrap();
                fs::write(&lease, "unknown").unwrap();
            }
            "foreign-lock" => {
                git(
                    &h.root,
                    &[
                        "worktree",
                        "lock",
                        "--reason",
                        "foreign owner",
                        target.to_str().unwrap(),
                    ],
                );
            }
            "policy-drift" => {
                fs::write(target.join(AUTO_POLICY), "uncommitted policy").unwrap();
            }
            "symlink" => {
                fs::remove_dir_all(target.join("output λ")).unwrap();
                symlink(&h.root, target.join("output λ")).unwrap();
            }
            _ => unreachable!(),
        }
        let refs = git(&h.root, &["show-ref"]);
        let (code, report, error) = h.auto("maintain", &[]);
        assert_ne!(code, 0, "{fault}: {report} {error}");
        assert!(target.exists());
        assert!(target.join("output λ").exists());
        assert_eq!(git(&h.root, &["show-ref"]), refs);
        assert!(h.ledger()["entries"][0]["terminal"].is_null());
    }
}

#[test]
fn kernel_generation_rearms_and_normal_finish_retains_original_nonzero_exit() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    consuming_operation(
        &h,
        "mkdir -p 'output λ'\nprintf rebuilt > 'output λ/cache'\nprintf original-error >&2\nexit 23\n",
    );
    let target = h.parent.join("rearm");
    assert_eq!(h.invoke("feature", "rearm", &target).0, 0);
    for generation in [1, 2] {
        let (code, report, error) = h.auto(
            "use",
            &[
                "--path",
                target.to_str().unwrap(),
                "--operation",
                "use.consumer",
            ],
        );
        assert_ne!(code, 0, "{report} {error}");
        assert_eq!(report["managed_process"]["exit_code"], 23);
        assert_eq!(report["managed_process"]["stderr"], "original-error");
        reclaim(&h, &target);
        assert_eq!(
            h.ledger()["entries"][0]["ownership"]["generation"],
            generation
        );
    }
    let (code, report, error) = h.auto(
        "finish",
        &["--path", target.to_str().unwrap(), "--dispose-evidence"],
    );
    assert_eq!(code, 0, "{report} {error}");
    assert!(!target.exists());
}

#[test]
fn admission_owner_death_keeps_git_mutation_and_native_hook_protected() {
    use std::os::unix::fs::PermissionsExt;
    let h = Host::new("payload");
    h.kernel_cleanup();
    let real = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let program = h.parent.join("admission-git");
    fs::write(&program,format!("#!/bin/sh\nif [ -f \"$HOME/interrupt-admission\" ] && [ \"$2 $3\" = 'worktree add' ]; then\nprintf '%s' \"$$\" > \"$HOME/admission-holder\"\nexec >\"$HOME/git-mutation.stdout\" 2>\"$HOME/git-mutation.stderr\"\nkill -KILL \"$PPID\"\nwhile [ ! -f \"$HOME/release-mutation\" ]; do sleep 0.02; done\nfi\nexec '{}' \"$@\"\n",real.display())).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    h.policy(|p| p["git"]["program"] = value!(program));
    h.hook("printf native > \"$HOME/native-hook\"\nwhile [ ! -f \"$HOME/release-hook\" ]; do sleep 0.02; done");
    fs::write(h.parent.join("interrupt-admission"), "kill owner").unwrap();
    let target = h.parent.join("interrupted-birth");
    let mut start = CapturedChild::spawn(
        Command::new(source().join("crates/worktree/target/debug/chrono-worktree"))
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .args([
                "start",
                "--host-root",
                h.root.to_str().unwrap(),
                "--config",
                POLICY,
                "--kind",
                "feature",
                "--name",
                "interrupted-birth",
                "--path",
                target.to_str().unwrap(),
            ]),
        &h.root,
    );
    start.await_file(&h.parent.join("admission-holder"));
    let _group = OwnedGroup(fs::read_to_string(h.parent.join("admission-holder")).unwrap());
    assert!(!start.wait().unwrap().success());
    let mut pending = CapturedChild::spawn(&mut h.auto_command("maintain", &[]), &h.root);
    thread::sleep(Duration::from_millis(100));
    assert!(
        pending.try_wait().unwrap().is_none(),
        "mutation child still owns admission"
    );
    fs::write(h.parent.join("release-mutation"), "continue Git").unwrap();
    await_file(&h.parent.join("native-hook"));
    assert!(
        pending.try_wait().unwrap().is_none(),
        "actual Git/hook must inherit admission"
    );
    fs::write(h.parent.join("release-hook"), "join native hook").unwrap();
    let (code, report, error) = h.received(pending.wait_with_output().unwrap());
    assert_eq!(code, 0, "{report} {error}");
    assert!(target.join("payload").exists());
    assert!(
        h.ledger()["entries"].as_array().unwrap().is_empty(),
        "crash must not fabricate completed birth"
    );
    assert!(
        git(&h.root, &["worktree", "list", "--porcelain"]).contains("locked"),
        "ordinary interrupted birth recovery remains explicit"
    );
}

#[test]
fn returned_use_without_token_still_protects_detached_python_child() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let helper = source().join(".chrono-harness");
    let body = format!(
        r#"mkdir -p .chrono-harness/state
/usr/bin/python3 -B - <<'PY'
import os, subprocess, sys
sys.path.insert(0, {helper:?})
from process_fds import inherited_fds
child = subprocess.Popen(['/bin/sh','-c','printf "%s" "$$" > .chrono-harness/state/detached; while [ ! -f .chrono-harness/state/release ]; do sleep 0.02; done; printf done > .chrono-harness/state/detached-done'], pass_fds=inherited_fds(), stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
PY
"#,
        helper = helper.to_str().unwrap()
    );
    consuming_operation(&h, &body);
    let target = h.parent.join("returned");
    assert_eq!(h.invoke("feature", "returned", &target).0, 0);
    output(&target);
    let (code, report, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "use.consumer",
        ],
    );
    assert_eq!(code, 0, "{report} {error}");
    await_file(&target.join(".chrono-harness/state/detached"));
    let _group =
        OwnedGroup(fs::read_to_string(target.join(".chrono-harness/state/detached")).unwrap());
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/cache").exists());
    fs::write(target.join(".chrono-harness/state/release"), "join").unwrap();
    await_file(&target.join(".chrono-harness/state/detached-done"));
    reclaim(&h, &target);
}

#[test]
fn adopted_short_check_enters_owner_and_nested_native_runner_keeps_lease() {
    let h = Host::new("payload");
    super::check_inputs::bind(&h);
    h.kernel_cleanup();
    let mut cfg = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    let trace = std::env::var_os("CHRONO_CHECK_HANDOFF_TRACE").map(PathBuf::from);
    if let Some(path) = &trace {
        assert!(path.is_absolute());
        cfg["environment"]["values"]["GIT_TRACE2_EVENT"] = value!(path);
        h.policy(|p| p["environment"]["values"]["GIT_TRACE2_EVENT"] = value!(path));
    }
    cfg["canonical_check"]["participation"] =
        value!({"operation":"worktree.check","tool":"chrono-worktree","argv":["check"]});
    fs::write(
        h.root.join(CONFIG),
        serde_json::to_vec_pretty(&cfg).unwrap(),
    )
    .unwrap();
    let mut body = fs::read_to_string(h.root.join("context-judge.sh")).unwrap();
    body=body.replace("exec /usr/bin/python3", "mkdir -p .chrono-harness/state\nprintf '%s' \"$$\" > .chrono-harness/state/check-holder\nwhile [ ! -f .chrono-harness/state/check-release ]; do sleep 0.02; done\nexec /usr/bin/python3");
    fs::write(h.root.join("context-judge.sh"), &body).unwrap();
    let mut judges = json(&fs::read(h.root.join(JUDGES)).unwrap()).unwrap();
    judges["judges"][0]["sha256"] = value!(sha256(body.as_bytes()));
    fs::write(
        h.root.join(JUDGES),
        serde_json::to_vec_pretty(&judges).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("normal-check");
    let other = h.parent.join("interrupted-other");
    assert_eq!(h.invoke("integration", "normal-check", &target).0, 0);
    assert_eq!(h.invoke("feature", "interrupted-other", &other).0, 0);
    super::check_inputs::install(&target);
    output(&target);
    output(&other);
    fs::write(target.join("payload"), "candidate").unwrap();
    commit(&target);
    assert!(
        git(&target, &["ls-files", "--", ".chrono-harness/bin/"]).is_empty(),
        "installed executables must remain untracked host artifacts"
    );
    if let Some(path) = &trace {
        fs::write(
            path.with_extension("handoff.json"),
            serde_json::to_vec(&value!({
                "target":target,
                "start_unix_ns":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos().to_string(),
            })).unwrap(),
        ).unwrap();
    }
    let mut check = CapturedChild::spawn(
        Command::new(target.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&target)
            .env("CHRONO_CHECK_SOURCE", "local")
            .arg("check"),
        &h.root,
    );
    let handshake = target.join(".chrono-harness/state/check-holder");
    let began = Instant::now();
    while !handshake.exists() && began.elapsed() < Duration::from_secs(10) {
        if check.try_wait().unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    if !handshake.exists() {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        check.record_wait_state(&handshake);
        let _ = check.kill();
        let original = check.wait_with_output().unwrap();
        panic!(
            "missing check handoff; original exit {:?}; stdout {}; stderr {}",
            original.status,
            String::from_utf8_lossy(&original.stdout),
            String::from_utf8_lossy(&original.stderr)
        );
    }
    let handoff_seconds = began.elapsed().as_secs_f64();
    assert!(
        !other.join("output λ").exists(),
        "ordinary check must drain other interrupted enrollment"
    );
    let _group =
        OwnedGroup(fs::read_to_string(target.join(".chrono-harness/state/check-holder")).unwrap());
    assert_eq!(h.auto("maintain", &[]).0, 0);
    assert!(target.join("output λ/cache").exists());
    fs::write(
        target.join(".chrono-harness/state/check-release"),
        "join native check",
    )
    .unwrap();
    let out = check.wait_with_output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("check: pass (exit 0)"),
        "original check console must be forwarded: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let policy = json(&fs::read(h.root.join(POLICY)).unwrap()).unwrap();
    let directory = h.root.join(policy["report_directory"].as_str().unwrap());
    for row in fs::read_dir(directory).unwrap() {
        let path = row.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("check-")
        {
            let report = json(&fs::read(&path).unwrap()).unwrap();
            let mut calls = std::collections::BTreeMap::<String, usize>::new();
            for process in report["processes"].as_array().unwrap() {
                *calls.entry(process["argv"].to_string()).or_default() += 1;
            }
            eprintln!(
                "CHECK_ADMISSION_MEASUREMENT {}",
                value!({"handoff_seconds":handoff_seconds,"report_sha256":sha256(&fs::read(&path).unwrap()),"git_calls":calls,"drain":report["drain"].as_array().map(Vec::len)})
            );
        }
    }
    reclaim(&h, &target);
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    for args in [
        vec!["check", "--unit", "missing"],
        vec!["check", "--collect"],
    ] {
        let out = Command::new(target.join(".chrono-harness/bin/chrono-harness"))
            .current_dir(&target)
            .env("CHRONO_CHECK_SOURCE", "local")
            .args(args)
            .output()
            .unwrap();
        assert_ne!(out.status.code(), Some(0));
        let entry = &h.ledger()["entries"][0];
        assert_eq!(entry["uses"], value!([]));
        assert_eq!(
            entry["ownership"]["cache_pending"], true,
            "selection failure still participates and retains original error"
        );
        assert_eq!(h.auto("maintain", &[]).0, 0);
    }
}

#[test]
fn registered_python_sources_load_without_creating_unregistered_outputs() {
    let sources = [
        ".chrono-harness/process_fds.py",
        ".chrono-harness/ci/bootstrap.py",
        ".chrono-harness/ci/workflow-inventory.py",
        ".chrono-harness/ci/workflow-inventory-tests.py",
        ".chrono-harness/migrations/scoped-v1.py",
        ".chrono-harness/migrations/scoped-v1-tests.py",
    ];
    for script in [sources[1], sources[2], sources[3], sources[5]] {
        let root = tempfile::tempdir().unwrap();
        for path in sources {
            let destination = root.path().join(path);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(source().join(path), destination).unwrap();
        }
        let loaded = Command::new("/usr/bin/python3")
            .args([
                "-c",
                "import runpy,sys; sys.dont_write_bytecode=False; sys.pycache_prefix=None; runpy.run_path(sys.argv[1],run_name='consumer')",
                script,
            ])
            .current_dir(root.path())
            .env_remove("PYTHONDONTWRITEBYTECODE")
            .env_remove("PYTHONPYCACHEPREFIX")
            .env_remove("CHRONO_PROCESS_FDS")
            .output()
            .unwrap();
        assert!(loaded.status.success(), "{script}: {loaded:?}");
        for parent in [
            ".chrono-harness",
            ".chrono-harness/ci",
            ".chrono-harness/migrations",
        ] {
            assert!(
                !root.path().join(parent).join("__pycache__").exists(),
                "{script} wrote unregistered bytecode in {parent}"
            );
        }
        for path in sources {
            assert_eq!(
                fs::read(root.path().join(path)).unwrap(),
                fs::read(source().join(path)).unwrap()
            );
        }
    }
}

#[test]
fn registered_python_consumers_forward_leases_after_both_wrappers_die() {
    use std::os::unix::fs::PermissionsExt;
    for route in ["bootstrap", "workflow-inventory", "scoped-v1-tests"] {
        let h = Host::new("payload");
        h.kernel_cleanup();
        let script = if route == "scoped-v1-tests" {
            source().join(".chrono-harness/migrations/scoped-v1-tests.py")
        } else {
            source().join(format!(".chrono-harness/ci/{route}.py"))
        };
        let tools = h.parent.join("python-tools");
        fs::create_dir(&tools).unwrap();
        for name in ["rustup", "cargo", "rustc"] {
            let body = if name == "rustup" {
                "#!/bin/sh\nexit 0\n".into()
            } else {
                format!("#!/bin/sh\nprintf '{} 1.95.0 (fixture)\\n'\n", name)
            };
            let path = tools.join(name);
            fs::write(&path, body).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let holder = "mkdir -p .chrono-harness/state\nprintf '%s' \"$$\" > .chrono-harness/state/python-holder\nwhile [ ! -f .chrono-harness/state/python-release ]; do sleep 0.02; done\nprintf joined > .chrono-harness/state/python-done\n";
        if route == "scoped-v1-tests" {
            let path = tools.join("git");
            fs::write(&path, format!("#!/bin/sh\n{holder}")).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let setup = match route {
            "bootstrap" => {
                r#"import json
(root/'.chrono-harness/ci').mkdir(exist_ok=True)
(root/'.chrono-harness/ci/probe.json').write_text(json.dumps(dict(schema='chrono-bootstrap/v1',projects='.chrono-harness/projects.json',rust_toolchain='1.95.0',tools=dict(sh='/bin/sh'),operations=['test.consumer'],install=[])))
sys.argv=[str(script),str(root),'.chrono-harness/ci/probe.json']
module.main()
"#
            }
            "workflow-inventory" => {
                r#"import json
p='.chrono-harness/state/list-projects.json'; c='.chrono-harness/state/list-config.json'
actions={key:dict(operation=key,tool='sh',argv=['st.sh']) for key in ['all','left','right']}
groups=dict(left='left',right='right')
(root/p).write_text(json.dumps(dict(projects=[dict(id='probe',test_groups=groups,actions=actions)])))
(root/c).write_text(json.dumps(dict(schema='chrono-workflow-inventory/v1',projects=p,project='probe',groups=groups,unfiltered_action='all',tools=dict(sh='/bin/sh'),binaries={key:['cases'] for key in actions},report_directory='.chrono-harness/state/listing',timeout_seconds=30,output_limit_bytes=65536)))
module.validate(root,Path(c))
"#
            }
            _ => "module.blob('opaque-declared-input')\n",
        };
        let body = format!(
            "mkdir -p .chrono-harness/state\nprintf '%s' \"$$\" > .chrono-harness/state/python-wrapper\nexport PATH='{}':\"$PATH\"\nexec /usr/bin/python3 -B - <<'PY'\nimport importlib.util,sys\nfrom pathlib import Path\nroot=Path.cwd();script=Path({:?})\nspec=importlib.util.spec_from_file_location('consumer',script);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)\n{}\nPY\n",
            tools.display(),
            script.to_str().unwrap(),
            setup
        );
        consuming_operation(&h, &body);
        fs::write(h.root.join("st.sh"), holder).unwrap();
        commit(&h.root);
        git(&h.root, &["push", "-q", "warehouse", "dev"]);
        let target = h.parent.join(format!("python-{route}"));
        assert_eq!(h.invoke("feature", route, &target).0, 0);
        output(&target);
        let mut owner = wrapper(&h, &target);
        owner.await_file(&target.join(".chrono-harness/state/python-holder"));
        let python =
            fs::read_to_string(target.join(".chrono-harness/state/python-wrapper")).unwrap();
        let _group = OwnedGroup(python.clone());
        owner.kill().unwrap();
        owner.wait().unwrap();
        assert!(
            Command::new("/bin/kill")
                .args(["-KILL", &python])
                .status()
                .unwrap()
                .success()
        );
        assert_eq!(h.auto("maintain", &[]).0, 0);
        assert!(
            target.join("output λ/cache").exists(),
            "{route}: subprocess child must retain lease after Python and worktree wrappers die"
        );
        fs::write(
            target.join(".chrono-harness/state/python-release"),
            "join declared child",
        )
        .unwrap();
        await_file(&target.join(".chrono-harness/state/python-done"));
        reclaim(&h, &target);
        assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    }
}

#[test]
fn native_cargo_child_survives_its_cargo_and_managed_wrappers() {
    let h = Host::new("payload");
    h.kernel_cleanup();
    let preparation = r#"mkdir -p .chrono-harness/state/cargo-case/src
cat > .chrono-harness/state/cargo-case/Cargo.toml <<'TOML'
[package]
name="lease-native-fixture"
version="0.1.0"
edition="2024"
TOML
cat > .chrono-harness/state/cargo-case/src/lib.rs <<'RUST'
#[test]
fn native_holder() {
    std::env::set_current_dir(std::env::var("CHRONO_NATIVE_FIXTURE_ROOT").unwrap()).unwrap();
    std::fs::write(".chrono-harness/state/native-pid", std::process::id().to_string()).unwrap();
    std::fs::write(".chrono-harness/state/native-holder", std::env::var("CHRONO_PROCESS_FDS").expect("native capability carrier")).unwrap();
    while !std::path::Path::new(".chrono-harness/state/native-release").exists() { std::thread::sleep(std::time::Duration::from_millis(10)); }
    std::fs::write(".chrono-harness/state/native-done", "joined").unwrap();
}
RUST
exec cargo test --offline --manifest-path .chrono-harness/state/cargo-case/Cargo.toml --no-run
"#;
    let body = r#"printf '%s' "$$" > .chrono-harness/state/cargo-wrapper
export CHRONO_NATIVE_FIXTURE_ROOT="$PWD"
exec cargo test --offline --manifest-path .chrono-harness/state/cargo-case/Cargo.toml -- --nocapture
"#;
    consuming_operation(&h, body);
    // Compile under the separately registered fixture action, with the same
    // managed environment and process bounds as its later Cargo invocation.
    // The native-holder deadline measures launch/lease handoff after preparation.
    fs::write(h.root.join("st.sh"), preparation).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    let target = h.parent.join("native-cargo");
    assert_eq!(h.invoke("feature", "native-cargo", &target).0, 0);
    let (code, prepared, error) = h.auto(
        "use",
        &[
            "--path",
            target.to_str().unwrap(),
            "--operation",
            "test.consumer",
        ],
    );
    assert_eq!(code, 0, "{prepared} {error}");
    assert_eq!(prepared["status"], "used");
    assert_eq!(prepared["managed_process"]["exit_code"], 0);
    assert_eq!(prepared["managed_process"]["failure"], Value::Null);
    assert!(!target.join(".chrono-harness/state/native-holder").exists());
    output(&target);
    let mut owner = wrapper(&h, &target);
    owner.await_file(&target.join(".chrono-harness/state/native-holder"));
    let cargo = fs::read_to_string(target.join(".chrono-harness/state/cargo-wrapper")).unwrap();
    let native = fs::read_to_string(target.join(".chrono-harness/state/native-pid")).unwrap();
    eprintln!("CARGO_HANDOFF launcher_pid={cargo} native_pid={native}");
    assert_ne!(
        cargo, native,
        "the killed launcher must be distinct from its live child"
    );
    let _group = OwnedGroup(cargo.clone());
    owner.kill().unwrap();
    owner.wait().unwrap();
    assert!(
        Command::new("/bin/kill")
            .args(["-KILL", &cargo])
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(h.auto("maintain", &[]).0, 0);
    let preserved_while_native_live = target.join("output λ/cache").exists();
    fs::write(target.join(".chrono-harness/state/native-release"), "join").unwrap();
    await_file(&target.join(".chrono-harness/state/native-done"));
    assert!(
        preserved_while_native_live,
        "actual native Cargo child completed after cleanup but lost enrollment lease"
    );
    reclaim(&h, &target);
}

#[test]
fn standalone_adopted_bootstrap_uses_coordinator_before_any_build_effect() {
    let h = Host::new("payload");
    participating_check(&h);
    h.kernel_cleanup();
    consuming_operation(&h, "exit 0\n");
    let ci = h.root.join(".chrono-harness/ci");
    fs::create_dir_all(&ci).unwrap();
    fs::copy(
        source().join(".chrono-harness/ci/bootstrap.py"),
        ci.join("bootstrap.py"),
    )
    .unwrap();
    fs::copy(
        source().join(".chrono-harness/process_fds.py"),
        h.root.join(".chrono-harness/process_fds.py"),
    )
    .unwrap();
    let cfg_path = ".chrono-harness/ci/probe.json";
    let git_program = chrono_harness::resolve_program(&h.root, "git", None).unwrap();
    let bootstrap = value!({"schema":"chrono-bootstrap/v1","projects":PROJECTS,"rust_toolchain":"1.95.0","tools":{"sh":"/bin/sh"},"operations":["test.consumer"],"install":[],"entrypoint":{"tool":"python3","script":".chrono-harness/ci/bootstrap.py"},"participation":{"coordinator_root":"git-main-worktree","program":".chrono-harness/bin/chrono-worktree","git":git_program,"config":POLICY}});
    fs::write(
        h.root.join(cfg_path),
        serde_json::to_vec_pretty(&bootstrap).unwrap(),
    )
    .unwrap();
    fs::write(h.root.join("st.sh"),"mkdir -p 'output λ' .chrono-harness/state\nprintf rebuilt > 'output λ/cache'\nprintf ready > .chrono-harness/state/bootstrap-holder\nwhile [ ! -f .chrono-harness/state/bootstrap-release ]; do sleep 0.02; done\n").unwrap();
    let mut config = json(&fs::read(h.root.join(CONFIG)).unwrap()).unwrap();
    config["tools"].as_array_mut().unwrap().push(value!({"id":"python3","program":"/usr/bin/python3","resolution":"PATH-once","version_argv":["--version"],"expected_version":"Python 3.9.6"}));
    fs::write(
        h.root.join(CONFIG),
        serde_json::to_vec_pretty(&config).unwrap(),
    )
    .unwrap();
    let mut fm = json(&fs::read(h.root.join(FM)).unwrap()).unwrap();
    for path in [
        cfg_path,
        ".chrono-harness/ci/bootstrap.py",
        ".chrono-harness/process_fds.py",
    ] {
        fm["files"]
            .as_array_mut()
            .unwrap()
            .push(file(path, value!([])));
    }
    fs::write(h.root.join(FM), serde_json::to_vec_pretty(&fm).unwrap()).unwrap();
    commit(&h.root);
    git(&h.root, &["push", "-q", "warehouse", "dev"]);
    fs::create_dir_all(h.root.join(".chrono-harness/bin")).unwrap();
    fs::copy(
        source().join("crates/worktree/target/debug/chrono-worktree"),
        h.root.join(".chrono-harness/bin/chrono-worktree"),
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(h.root.join(AUTO_POLICY)).unwrap().trim(),
        git(&h.root, &["show", &format!("HEAD:{AUTO_POLICY}")])
    );
    assert_eq!(
        sha256(&fs::read(h.root.join(".chrono-harness/bin/chrono-worktree")).unwrap()),
        sha256(&fs::read(source().join("crates/worktree/target/debug/chrono-worktree")).unwrap())
    );
    let drifted = h.parent.join("bootstrap-drifted-a");
    assert_eq!(h.invoke("feature", "bootstrap-drifted-a", &drifted).0, 0);
    output(&drifted);
    fs::write(drifted.join(AUTO_POLICY), "ordinary policy edit").unwrap();
    let target = h.parent.join("standalone-bootstrap");
    assert_eq!(h.invoke("feature", "standalone-bootstrap", &target).0, 0);
    assert!(
        !target.join(".chrono-harness/bin/chrono-worktree").exists(),
        "fresh checkout resolves explicit coordinator binary"
    );
    let tools = h.parent.join("bootstrap-tools");
    fs::create_dir(&tools).unwrap();
    use std::os::unix::fs::PermissionsExt;
    for name in ["rustup", "cargo", "rustc"] {
        let text = if name == "rustup" {
            "#!/bin/sh\nexit 0\n".into()
        } else {
            format!("#!/bin/sh\nprintf '{} 1.95.0 (fixture)\\n'\n", name)
        };
        let path = tools.join(name);
        fs::write(&path, text).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::join_paths(
        std::iter::once(tools).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let mut child = CapturedChild::spawn(
        Command::new("/usr/bin/python3")
            .current_dir(&target)
            .env("PATH", &path)
            .args([".chrono-harness/ci/bootstrap.py", ".", cfg_path]),
        &h.root,
    );
    let marker = target.join(".chrono-harness/state/bootstrap-holder");
    let began = Instant::now();
    while !marker.exists() && began.elapsed() < Duration::from_secs(10) {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    if !marker.exists() {
        let _ = child.kill();
        let original = child.wait_with_output().unwrap();
        panic!("bootstrap handoff failed: {original:?}");
    }
    assert_ne!(
        h.auto("maintain", &[]).0,
        0,
        "explicit maintain retains A's refusal"
    );
    assert!(target.join("output λ/cache").exists());
    fs::write(
        target.join(".chrono-harness/state/bootstrap-release"),
        "join",
    )
    .unwrap();
    let original = child.wait_with_output().unwrap();
    assert_eq!(
        original.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&original.stdout),
        String::from_utf8_lossy(&original.stderr)
    );
    // Restore only the fixture's ordinary edit; retained failed receipts stay immutable.
    fs::copy(h.root.join(AUTO_POLICY), drifted.join(AUTO_POLICY)).unwrap();
    reclaim(&h, &target);
    assert_eq!(h.ledger()["entries"][0]["uses"], value!([]));
    assert!(h.ledger()["entries"][0]["terminal"].is_null());
    install_inner(&target, "printf canonical-linked-check\n");
    let checked = Command::new(target.join(".chrono-harness/bin/chrono-harness"))
        .current_dir(&target)
        .arg("check")
        .output()
        .unwrap();
    assert_eq!(checked.status.code(), Some(0), "{checked:?}");
    assert_eq!(checked.stdout, b"canonical-linked-check");
    // A real nonzero build followed by owner reopening refusal retains both outcomes.
    let original_policy = fs::read(h.root.join(AUTO_POLICY)).unwrap();
    fs::write(target.join("st.sh"), format!(
        "printf 'original build stdout\\n'; printf 'original build stderr\\n' >&2; printf drift > '{}'; exit 7\n",
        h.root.join(AUTO_POLICY).display()
    )).unwrap();
    commit(&target);
    let refused = Command::new("/usr/bin/python3")
        .current_dir(&target)
        .env("PATH", &path)
        .args([".chrono-harness/ci/bootstrap.py", ".", cfg_path])
        .output()
        .unwrap();
    assert_ne!(
        refused.status.code(),
        Some(7),
        "lifecycle refusal was masked: {refused:?}"
    );
    assert_eq!(refused.stdout, b"original build stdout\n");
    let error = String::from_utf8_lossy(&refused.stderr);
    assert!(error.starts_with("original build stderr\n"), "{error}");
    assert!(error.contains("bootstrap lifecycle failed"), "{error}");
    let reference = error
        .split("original report ")
        .nth(1)
        .unwrap()
        .lines()
        .next()
        .unwrap();
    let original = json(&fs::read(h.root.join(reference)).unwrap()).unwrap();
    // The existing inner bootstrap wraps the build's exit 7 as its own exit 1;
    // preserve that wrapper result and its original build exception/streams.
    assert_eq!(original["managed_process"]["exit_code"], 1);
    let build_error = original["managed_process"]["stderr"].as_str().unwrap();
    assert!(build_error.starts_with("original build stderr\n"));
    assert!(build_error.contains("non-zero exit status 7"));
    assert_eq!(
        original["managed_process"]["stdout"],
        "original build stdout\n"
    );
    assert_eq!(original["status"], "failed");
    assert_ne!(original["managed_command_failed"], true);
    fs::write(h.root.join(AUTO_POLICY), original_policy).unwrap();
    // An upgraded binary alone cannot bridge committed v1/v2 policy disagreement.
    let candidate_policy = fs::read(h.root.join(AUTO_POLICY)).unwrap();
    let mut old = json(&candidate_policy).unwrap();
    old["schema"] = value!("chrono-worktree-automatic-cleanup/v1");
    fs::write(
        h.root.join(AUTO_POLICY),
        serde_json::to_vec_pretty(&old).unwrap(),
    )
    .unwrap();
    commit(&h.root);
    output(&target);
    let refused_check = Command::new(target.join(".chrono-harness/bin/chrono-harness"))
        .current_dir(&target)
        .arg("check")
        .output()
        .unwrap();
    assert_eq!(refused_check.status.code(), Some(2), "{refused_check:?}");
    assert!(
        String::from_utf8_lossy(&refused_check.stderr)
            .contains("coordinator policy/configuration identity mismatch")
    );
    fs::remove_file(target.join(".chrono-harness/state/bootstrap-holder")).unwrap();
    let refused_bootstrap = Command::new("/usr/bin/python3")
        .current_dir(&target)
        .args([".chrono-harness/ci/bootstrap.py", ".", cfg_path])
        .output()
        .unwrap();
    assert_ne!(
        refused_bootstrap.status.code(),
        Some(0),
        "{refused_bootstrap:?}"
    );
    assert!(
        String::from_utf8_lossy(&refused_bootstrap.stderr)
            .contains("coordinator policy/configuration identity mismatch")
    );
    assert!(
        !target
            .join(".chrono-harness/state/bootstrap-holder")
            .exists(),
        "mismatch must refuse before build"
    );
    assert!(
        target.join("output λ/cache").exists(),
        "mismatch must preserve caches"
    );
    fs::write(h.root.join(AUTO_POLICY), candidate_policy).unwrap();
    commit(&h.root);
}
