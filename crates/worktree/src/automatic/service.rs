//! One executor per physical coordinator. Events never carry business authority.
use super::*;
use chrono_harness::maintenance_signal::{self as signal, Policy as ServicePolicy, RoundGuard};
use std::time::{Duration, Instant};
#[cfg(unix)]
use std::os::{fd::{AsRawFd, FromRawFd}, unix::net::UnixDatagram};

pub(super) fn handoff(inputs: &PolicyInputs, reason: &str) -> Value {
    value!({"schema":"chrono-maintenance-handoff/v1","reason":reason,
        "coordinator_root":inputs.policy.coordinator_root,"responsibility":"existing-owner-state",
        "activation":"after-original-publication","accepted":false,
        "round_completion":"not-established","disposal":"not-attempted"})
}

fn service_policy(r: &mut Runner, inputs: &PolicyInputs) -> Result<Option<ServicePolicy>, String> {
    let root = &inputs.policy.coordinator_root;
    let Some((policy, bytes)) = signal::adopted(root)? else { return Ok(None); };
    start::registered_policy(&start::registrations(r, root, &inputs.anchor_head,
        &decode::<crate::Config>(&inputs.anchor_config_bytes)?.host_config)?.0, signal::POLICY_PATH,
        &inputs.policy.state_directory)?;
    if r.blob(root, &inputs.anchor_head, signal::POLICY_PATH)? != bytes
        || policy.state_directory != inputs.anchor_policy.state_directory
    { return Err("maintenance policy differs from committed coordinator ownership".into()); }
    Ok(Some(policy))
}

/// Singleton is independent of enrollment metadata and is never acquired by business.
fn singleton(root: &Path, directory: &str) -> Result<Option<Lease>, String> {
    let directory = no_symlink_parents(root, directory.trim_end_matches('/'))?;
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    Lease::acquire(&directory.join("maintenance.lease"), None, true, true, None)
}

pub(super) fn maintain(root: &Path, config_path: &str, trigger: &str, consumer: Option<&str>) -> Result<Value, String> {
    round(root, config_path, trigger, None, None, consumer)
}

fn round(root: &Path, config_path: &str, trigger: &str, held: Option<&Lease>, missed: Option<u64>, consumer: Option<&str>) -> Result<Value, String> {
    let _internal = RoundGuard::enter();
    let (config, bytes) = crate::configuration(root, config_path)?;
    // Keep executor ownership through the final parent publication and recovery handoff.
    let executor = std::cell::RefCell::new(None);
    start::with_report_for_consumer(root, config_path, config, &bytes, "maintain", "maintained", consumer,
        |r, report| {
            let inputs = PolicyInputs::load(r, root, config_path, &bytes)?
                .ok_or("automatic cleanup is not opted in")?;
            let policy = service_policy(r, &inputs)?;
            report["maintenance_policy"] = value!(policy);
            if let Some(held) = held {
                if root != inputs.policy.coordinator_root.as_path() { return Err("maintenance service must run at the physical coordinator".into()); }
                held.stable()?;
            }
            else { *executor.borrow_mut() = singleton(&inputs.policy.coordinator_root, &inputs.policy.state_directory)?; }
            Ok((inputs.policy.coordinator_root.clone(), inputs))
        },
        |r, token, report, inputs| {
            inputs.report_source(report);
            report["invoking_root"] = value!(root);
            report["maintenance_trigger"] = value!(trigger);
            report["missed_timer_opportunities"] = value!(missed);
            if held.is_none() && executor.borrow().is_none() {
                // Do not claim a round or wait for the current executor.
                // Suppression applies only to actual rounds, not this external hint.
                report["round_started"] = value!(false);
                let socket = signal::adopted(&inputs.policy.coordinator_root)?
                    .ok_or("maintenance executor busy without adopted activation")?.0
                    .socket_path(&inputs.policy.coordinator_root)?;
                let sent = signal::send(&socket, b"explicit");
                report["maintenance_request"] = match sent {
                    Ok(()) => value!({"accepted":true,"status":"requested","round_completion":"not-established"}),
                    Err(error) => value!({"accepted":false,"status":"wake-failed","error":error,"round_completion":"not-established"}),
                };
                if report["maintenance_request"]["accepted"] != true {
                    return Err("maintenance executor busy; activation was not accepted; existing responsibilities retained".into());
                }
                return Ok(());
            }
            report["round_started"] = value!(true);
            run_round(r, token, report, &inputs)
        })
}

fn run_round(r: &mut Runner, token: &str, report: &mut Value, inputs: &PolicyInputs) -> Result<(), String> {
    let root = &inputs.policy.coordinator_root;
    let directory = no_symlink_parents(root, inputs.policy.state_directory.trim_end_matches('/'))?;
    // Shared deployment custody protects executable/policy identity, never business metadata.
    let namespace = open_namespace(inputs, &directory, false, None)?;
    let observation = Observation { inputs, directory: &directory, scope: root };
    report["drain"] = value!([]);
    report["cleanup_failures"] = value!([]);
    report["producer_drains"] = value!([]);
    report["maintenance_scope"] = value!({"kind":"declared-enrollments"});
    let mut inventory = inventory::Report::begin(root, report)?;
    if let Err(error) = observation.observe_inventory(r, &[root.clone()], report, true, &BTreeSet::new()) {
        inventory.error("git-inventory", error);
    }
    inventory.worktrees(&observation, r, report, "before");
    let mut failures = Vec::new();
    let mut unprotected = Vec::new();
    match retirement::resume_retired_addresses(inputs, &directory, &namespace, r, report) {
        Ok(errors) => failures.extend(errors), Err(error) => failures.push(error),
    }
    // Enumeration errors do not silently omit members or prevent independent producers.
    let mut namespace_complete = true;
    let mut paths = match observation.enrollment_paths_observed(|row| {
        namespace_complete = false;
        inventory.error("enrollment-namespace", row);
    }) {
        Ok(paths) => paths,
        Err(error) => { namespace_complete = false; inventory.error("enrollment-namespace", &error); failures.push(error); vec![] }
    };
    // The committed coordinator policy declares its artifacts even before a
    // cache enrollment exists. Keep the original explicit-maintain observation.
    if !paths.contains(root) { paths.push(root.clone()); paths.sort(); }
    for path in paths {
        // Observe adopted declarations, then DROP enrollment exclusion before footprint IO.
        let snapshot = (|| {
            let mut owner = Manager::open_metadata(r, root, &inputs.config_path,
                &inputs.anchor_config_bytes, token, &path, false, false)?
                .ok_or("cleanup policy disappeared")?;
            owner.restore_absent_policy(r)?;
            if crate::verified_lifecycle::enrolled_policy_required(owner.ledger.entries.len()) {
                owner.adopt_cleanup_policy(r, true)?;
            }
            Ok::<_, String>(inventory::ArtifactSnapshot::from_owner(&owner))
        })();
        match snapshot {
            Ok(snapshot) => inventory.artifacts(&snapshot),
            Err(error) => { inventory.error(&path.to_string_lossy(), &error); }
        }
        // Every actual effect reopens the original owner and validates current identity,
        // policy, lease, generation and source custody. Snapshots grant no permission.
        let result = (|| {
            let mut owner = Manager::open_metadata(r, root, &inputs.config_path,
                &inputs.anchor_config_bytes, token, &path, false, false)?
                .ok_or("cleanup policy disappeared")?;
            owner.restore_absent_policy(r)?;
            observe_boundary("disposition", &path);
            let errors = owner.drain_entries(r, &[root.clone()], report, true)?;
            owner.retire_empty_address(r, report)?;
            Ok::<_, String>(errors)
        })();
        match result {
            Ok(errors) => failures.extend(errors),
            Err(error) => {
                unprotected.push(path.clone());
                let busy = error.contains("metadata is busy") || error.contains("metadata remains busy");
                report["drain"].as_array_mut().unwrap().push(value!({"path":path,
                    "status":if busy {"preserved"} else {"failed"},"error":error}));
                if !busy { failures.push(format!("{}: {error}", path.display())); }
            }
        }
    }
    for producer in inputs.anchor_policy.retained_producers.clone() {
        // No enrollment metadata is held during producer IO or its full observation.
        observe_boundary("producer", root);
        producer.observe_inventory(inputs, &mut inventory);
        let attempt = (|| {
            let (config, bytes) = crate::configuration(root, &inputs.config_path)?;
            let result = start::with_report(root, &inputs.config_path, config, &bytes,
                "producer-cleanup", "cleaned", |r, _, report| {
                    report["producer_objects"] = value!([]);
                    producer.drain(inputs, r, report)
                })?;
            let path = result["report_path"].as_str().ok_or("producer drain report path")?;
            let bytes = crate::recovery::state_bytes(root, path)?;
            report["producer_drains"].as_array_mut().unwrap().push(value!({"producer":producer.id,
                "status":result["status"],"error":result["error"],
                "receipt":{"path":path,"sha256":sha256(&bytes)},
                "input":crate::recovery::retained_input(root, path, &bytes, "receipt")}));
            if result["status"] != "cleaned" { return Err(format!("producer {}: {}", producer.id, result["error"])); }
            Ok::<_, String>(())
        })();
        if let Err(error) = attempt { inventory.error(&producer.id, &error); failures.push(error); }
    }
    report["output_retention"] = if namespace_complete && unprotected.is_empty() {
        match chrono_harness::retained_artifacts::retention::Inventory::adopted(root)
            .and_then(|owner| owner.map(|owner| owner.maintain_explicit_observed(|row| {
                inventory.retention(row.clone()); Ok(())
            })).transpose()) {
            Ok(result) => value!(result),
            Err(error) => { inventory.error("retention", &error); failures.push(error.clone());
                value!({"status":"failed","error":error}) }
        }
    } else {
        inventory.error("retention", "enrollment original protection unavailable; dependent retention deferred");
        value!({"status":"preserved","reason":"enrollment original protection unavailable",
            "namespace_complete":namespace_complete,"unprotected_enrollments":unprotected})
    };
    inventory.worktrees(&observation, r, report, "after");
    if let Err(error) = inventory.finish(report) { failures.push(error); }
    report["cleanup_failures"] = value!(failures);
    report["owned_work_joined"] = value!(true); // All work above is synchronous; no children are detached.
    if failures.is_empty() { Ok(()) } else { Err(format!("maintenance incomplete: {}", failures.join("; "))) }
}

#[cfg(unix)]
fn activated_socket() -> Result<UnixDatagram, String> {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" { fn launch_activate_socket(name: *const libc::c_char,
            fds: *mut *mut libc::c_int, count: *mut libc::size_t) -> libc::c_int; }
        let mut fds = std::ptr::null_mut();
        let mut count = 0;
        let result = unsafe { launch_activate_socket(c"Maintenance".as_ptr(), &mut fds, &mut count) };
        if result != 0 { return Err(format!("launch_activate_socket failed: {result}")); }
        if fds.is_null() { return Err("activation returned no socket descriptors".into()); }
        let descriptors = unsafe { std::slice::from_raw_parts(fds, count) };
        if count != 1 {
            for fd in descriptors { unsafe { libc::close(*fd); } }
            unsafe { libc::free(fds.cast()); }
            return Err("maintenance activation requires exactly one socket".into());
        }
        let fd = descriptors[0];
        unsafe { libc::free(fds.cast()); }
        let socket = unsafe { UnixDatagram::from_raw_fd(fd) };
        return Ok(socket);
    }
    #[cfg(not(target_os = "macos"))]
    {
        if std::env::var("LISTEN_PID").ok().and_then(|v| v.parse::<u32>().ok()) != Some(std::process::id())
            || std::env::var("LISTEN_FDS").ok().as_deref() != Some("1") {
            return Err("service requires one OS-owned activation socket".into());
        }
        Ok(unsafe { UnixDatagram::from_raw_fd(3) })
    }
}

/// Narrow, injectable service loop used by the Rust socket boundary expectations.
/// The supplied round must join its work; this function owns no detached tasks.
#[cfg(unix)]
pub fn serve(socket: &UnixDatagram, policy: &ServicePolicy,
    mut execute: impl FnMut(&str, u64) -> Result<(), String>) -> Result<(), String> {
    policy.validate()?;
    socket.set_nonblocking(true).map_err(|e| e.to_string())?;
    let descriptor = socket.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    if flags == -1 || unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) } == -1 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let period = Duration::from_secs(policy.timer_seconds);
    let mut next_timer = Instant::now().checked_add(period).ok_or("timer overflow")?;
    let mut last_work = Instant::now();
    let mut first = true;
    loop {
        let mut messages = 0u64;
        let mut bytes = [0u8; 64];
        // Coalesce a finite batch so concurrent publishers cannot indefinitely
        // postpone the actual round. Remaining hints stay in the OS queue; the
        // round still reads ALL authoritative members, independent of hints.
        for _ in 0..64 {
            match socket.recv(&mut bytes) {
                Ok(_) => messages += 1,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.to_string()),
            }
        }
        let now = Instant::now();
        let timer_due = now >= next_timer;
        if first || messages != 0 || timer_due {
            let missed = if timer_due { now.duration_since(next_timer).as_secs() / policy.timer_seconds } else { 0 };
            let trigger = if timer_due { "timer" } else if messages != 0 { "event" } else { "os-activation" };
            // Advance from the original cadence before execution. Long rounds
            // cannot erase timer opportunities that occur while they are running.
            if timer_due {
                let periods = missed.checked_add(1).ok_or("timer opportunity overflow")?;
                let seconds = periods.checked_mul(policy.timer_seconds).ok_or("timer overflow")?;
                next_timer = next_timer.checked_add(Duration::from_secs(seconds)).ok_or("timer overflow")?;
            }
            // Failed rounds retain originals but do not generate immediate retry loops.
            execute(trigger, missed)?;
            first = false;
            last_work = Instant::now();
            continue;
        }
        let idle = policy.idle_seconds.map(|seconds| last_work.checked_add(Duration::from_secs(seconds))
            .ok_or("idle deadline overflow")).transpose()?;
        if crate::verified_lifecycle::maintenance_idle_exit(messages != 0, timer_due,
            idle.is_some_and(|deadline| now >= deadline)) {
            // OS still owns the socket. A racing/new datagram remains activatable.
            return Ok(());
        }
        let deadline = idle.map_or(next_timer, |idle| idle.min(next_timer));
        let timeout = deadline.saturating_duration_since(now).as_millis().min(i32::MAX as u128) as i32;
        let mut fd = libc::pollfd { fd: descriptor, events: libc::POLLIN, revents: 0 };
        let ready = unsafe { libc::poll(&mut fd, 1, timeout.max(1)) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted { return Err(error.to_string()); }
        } else if fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            return Err("maintenance activation socket lost".into());
        }
    }
}

pub(super) fn dispatch(args: &[String]) -> Result<Value, String> {
    let mut root = PathBuf::from(".");
    let mut config_path = ".chrono-harness/worktree.json".to_string();
    let mut seen = BTreeSet::new();
    let mut index = 1;
    while index < args.len() {
        let key = args[index].as_str();
        if index + 1 == args.len() || !seen.insert(key) { return Err("invalid service argument".into()); }
        match key { "--host-root" => root = args[index+1].clone().into(),
            "--config" => config_path = args[index+1].clone(), _ => return Err("unknown service argument".into()) }
        index += 2;
    }
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    { run_service(&root, &config_path, activated_socket) }
    #[cfg(not(unix))]
    { Err("maintenance service requires Unix socket activation".into()) }
}

#[cfg(unix)]
fn run_service(root: &Path, config_path: &str,
    activate: impl FnOnce() -> Result<UnixDatagram, String>) -> Result<Value, String> {
    let (policy, _) = signal::adopted(&root)?.ok_or("maintenance service policy is not adopted")?;
    let Some(executor) = singleton(&root, &policy.state_directory)? else {
        let request = signal::request_adopted(&root, "service-competition");
        return Ok(value!({"status":if request["accepted"] == true {"maintenance-requested"} else {"failed"},
            "maintenance_request":request,"round_completion":"not-established"}));
    };
    let socket = activate()?;
    if socket.local_addr().map_err(|e| e.to_string())?.as_pathname() != Some(policy.socket_path(&root)?.as_path()) {
        return Err("OS activation socket differs from adopted coordinator".into());
    }
    serve(&socket, &policy, |trigger, missed| {
        let report = round(&root, &config_path, trigger, Some(&executor), Some(missed), None)?;
        // Each round already has its own retained original. Do not accumulate reports.
        eprintln!("{}", serde_json::to_string(&value!({"schema":"chrono-maintenance-round-reference/v1",
            "source_root":root,"report_path":report["report_path"],"status":report["status"],
            "trigger":trigger,"missed_timer_opportunities":missed})).map_err(|e| e.to_string())?);
        Ok(())
    })?;
    Ok(value!({"status":"maintenance-idle-exit","owned_work_joined":true}))
}

/// Rust OS-provider mock exercises the production singleton and round path.
#[cfg(all(unix, feature = "producer-birth-test"))]
pub fn run_with_socket(root: &Path, config_path: &str, socket: UnixDatagram) -> Result<Value, String> {
    let root = fs::canonicalize(root).map_err(|error| error.to_string())?;
    run_service(&root, config_path, || Ok(socket))
}

#[cfg(feature = "producer-birth-test")]
thread_local! {
    static BOUNDARY: std::cell::RefCell<Option<Box<dyn FnMut(&str, &Path)>>> = const { std::cell::RefCell::new(None) };
}
#[cfg(feature = "producer-birth-test")]
pub fn with_observer<T>(observer: impl FnMut(&str, &Path) + 'static, run: impl FnOnce() -> T) -> T {
    struct Reset;
    impl Drop for Reset { fn drop(&mut self) { BOUNDARY.with(|slot| { slot.borrow_mut().take(); }); } }
    BOUNDARY.with(|slot| { assert!(slot.borrow().is_none()); *slot.borrow_mut() = Some(Box::new(observer)); });
    let _reset = Reset;
    run()
}
pub(super) fn observe_boundary(kind: &str, path: &Path) {
    #[cfg(feature = "producer-birth-test")]
    BOUNDARY.with(|slot| { if let Some(observer) = slot.borrow_mut().as_mut() { observer(kind, path); } });
    #[cfg(not(feature = "producer-birth-test"))]
    let _ = (kind, path);
}
