//! One executor per physical coordinator. Events never carry business authority.
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Pending events and due timers take precedence over an idle observation.
pub fn maintenance_idle_exit(event_pending: bool, timer_due: bool, idle_due: bool) -> (exit: bool)
    ensures exit == (!event_pending && !timer_due && idle_due),
{
    !event_pending && !timer_due && idle_due
}

/// Preserve both actual roots, including their order and a repeated root.
pub fn maintenance_exclusions<P>(coordinator: P, invoking: P) -> (excluded: Vec<P>)
    ensures excluded@ == seq![coordinator, invoking],
{
    let mut excluded = Vec::new();
    excluded.push(coordinator);
    excluded.push(invoking);
    excluded
}

pub open spec fn maintenance_retention_result<S, T, R, A, C, F, D>(
    source: S, namespace_complete: bool, unprotected: usize,
    attempt: A, completed: C, failed: F, deferred: D, out: (S, R)) -> bool
    where A: FnOnce(S) -> (S, Result<T, String>), C: FnOnce(S, T) -> (S, R),
        F: FnOnce(S, String) -> (S, R), D: FnOnce(S) -> (S, R),
{
    if namespace_complete && unprotected == 0 {
        exists|observed: S, result: Result<T, String>|
            #[trigger] attempt.ensures((source,), (observed, result))
            && match result {
                Ok(value) => completed.ensures((observed, value), out),
                Err(error) => failed.ensures((observed, error), out),
            }
    } else {
        deferred.ensures((source,), out)
    }
}

/// 只在完整命名空间且全部登记项原件已保护时清理；原结果决定唯一后续路径。
pub fn maintenance_retention_phase<S, T, R, A, C, F, D>(
    source: S, namespace_complete: bool, unprotected: usize,
    attempt: A, completed: C, failed: F, deferred: D) -> (out: (S, R))
    where A: FnOnce(S) -> (S, Result<T, String>), C: FnOnce(S, T) -> (S, R),
        F: FnOnce(S, String) -> (S, R), D: FnOnce(S) -> (S, R),
    requires attempt.requires((source,)), deferred.requires((source,)),
        forall|state: S, value: T| completed.requires((state, value)),
        forall|state: S, error: String| failed.requires((state, error)),
    ensures maintenance_retention_result(source, namespace_complete, unprotected,
        attempt, completed, failed, deferred, out),
{
    if namespace_complete && unprotected == 0 {
        let (state, result) = attempt(source);
        let ghost observed = state;
        let ghost observation = result;
        assert(attempt.ensures((source,), (observed, observation)));
        match result {
            Ok(value) => {
                let out = completed(state, value);
                assert(completed.ensures((observed, value), out));
                out
            },
            Err(error) => {
                let out = failed(state, error);
                assert(failed.ensures((observed, error), out));
                out
            },
        }
    } else {
        deferred(source)
    }
}

pub open spec fn activation_error<S>(source: S, message: Seq<char>,
    out: (S, Result<(), String>)) -> bool
{
    out.0 == source && match out.1 {
        Err(error) => error@ == message,
        Ok(()) => false,
    }
}

pub open spec fn activation_binding_result<S, T, B, D>(source: S, named: bool,
    address: T, binding: B, describe: D, out: (S, Result<(), String>)) -> bool
    where B: FnOnce(S, &T) -> (S, bool), D: FnOnce(S, T) -> (S, String),
{
    if named { out == (source, Ok::<(), String>(())) }
    else {
        exists|bound: S, matches: bool|
            #[trigger] binding.ensures((source, &address), (bound, matches))
            && if matches { out == (bound, Ok::<(), String>(())) }
            else { out.1.is_err()
                && describe.ensures((bound, address), (out.0, out.1->Err_0)) }
    }
}

fn check_activation_binding<S, T, B, D>(source: S, named: bool,
    address: T, binding: B, describe: D) -> (out: (S, Result<(), String>))
    where B: FnOnce(S, &T) -> (S, bool), D: FnOnce(S, T) -> (S, String),
    requires forall|state: S, address: &T| binding.requires((state, address)),
        forall|state: S, address: T| describe.requires((state, address)),
    ensures activation_binding_result(source, named, address, binding, describe, out),
{
    if named { return (source, Ok(())); }
    let ghost original_address = address;
    let (state, matches) = binding(source, &address);
    let ghost bound = state;
    assert(binding.ensures((source, &original_address), (bound, matches)));
    if matches { (state, Ok(())) }
    else {
        let (state, error) = describe(state, address);
        assert(describe.ensures((bound, original_address), (state, error)));
        (state, Err(error))
    }
}

pub open spec fn activation_path_result<S, T, M, U, B, D>(source: S, named: bool,
    address: T, metadata: M, user: U, binding: B, describe: D,
    out: (S, Result<(), String>)) -> bool
    where M: FnOnce(S) -> (S, Result<(bool, u32), String>),
        U: FnOnce(S) -> (S, u32), B: FnOnce(S, &T) -> (S, bool),
        D: FnOnce(S, T) -> (S, String),
{
    exists|observed: S, result: Result<(bool, u32), String>|
        #[trigger] metadata.ensures((source,), (observed, result))
        && match result {
            Err(error) => out == (observed, Err::<(), String>(error)),
            Ok(fields) => if !fields.0 {
                activation_error(observed, "adopted activation path is not an owned socket"@, out)
            } else {
                exists|identified: S, uid: u32|
                    #[trigger] user.ensures((observed,), (identified, uid))
                    && if fields.1 != uid {
                        activation_error(identified,
                            "adopted activation path is not an owned socket"@, out)
                    } else {
                        activation_binding_result(identified, named, address, binding, describe, out)
                    }
            },
        }
}

fn check_activation_path<S, T, M, U, B, D>(source: S, named: bool,
    address: T, metadata: M, user: U, binding: B, describe: D)
    -> (out: (S, Result<(), String>))
    where M: FnOnce(S) -> (S, Result<(bool, u32), String>),
        U: FnOnce(S) -> (S, u32), B: FnOnce(S, &T) -> (S, bool),
        D: FnOnce(S, T) -> (S, String),
    requires metadata.requires((source,)), forall|state: S| user.requires((state,)),
        forall|state: S, address: &T| binding.requires((state, address)),
        forall|state: S, address: T| describe.requires((state, address)),
    ensures activation_path_result(source, named, address, metadata, user, binding, describe, out),
{
    let (state, result) = metadata(source);
    let ghost observed = state;
    let ghost observation = result;
    assert(metadata.ensures((source,), (observed, observation)));
    let fields = match result {
        Err(error) => return (state, Err(error)),
        Ok(fields) => fields,
    };
    if !fields.0 {
        return (state, Err("adopted activation path is not an owned socket".to_string()));
    }
    let (state, uid) = user(state);
    let ghost identified = state;
    assert(user.ensures((observed,), (identified, uid)));
    if fields.1 != uid {
        return (state, Err("adopted activation path is not an owned socket".to_string()));
    }
    check_activation_binding(state, named, address, binding, describe)
}

pub open spec fn activation_sequence_result<S, T, Q, E, A, M, U, B, D>(source: S,
    named: bool, datagram: i32, query: Q, last_error: E, address: A,
    metadata: M, user: U, binding: B, describe: D, out: (S, Result<(), String>)) -> bool
    where Q: FnOnce(S) -> (S, (i32, i32)), E: FnOnce(S) -> (S, String),
        A: FnOnce(S) -> (S, Result<T, String>),
        M: FnOnce(S) -> (S, Result<(bool, u32), String>),
        U: FnOnce(S) -> (S, u32), B: FnOnce(S, &T) -> (S, bool),
        D: FnOnce(S, T) -> (S, String),
{
    exists|queried: S, raw: (i32, i32)|
        #[trigger] query.ensures((source,), (queried, raw))
        && if raw.0 != 0 {
            out.1.is_err() && last_error.ensures((queried,), (out.0, out.1->Err_0))
        } else if raw.1 != datagram {
            activation_error(queried, "OS activation requires a Unix datagram socket"@, out)
        } else {
            exists|observed: S, result: Result<T, String>|
                #[trigger] address.ensures((queried,), (observed, result))
                && match result {
                    Err(error) => out == (observed, Err::<(), String>(error)),
                    Ok(value) => activation_path_result(
                        observed, named, value, metadata, user, binding, describe, out),
                }
        }
}

/// 先核描述符类型，再观察地址和路径；首错保留原状态，命名激活也须核路径所有者。
pub fn validate_activation_sequence<S, T, Q, E, A, M, U, B, D>(source: S,
    named: bool, datagram: i32, query: Q, last_error: E, address: A,
    metadata: M, user: U, binding: B, describe: D) -> (out: (S, Result<(), String>))
    where Q: FnOnce(S) -> (S, (i32, i32)), E: FnOnce(S) -> (S, String),
        A: FnOnce(S) -> (S, Result<T, String>),
        M: FnOnce(S) -> (S, Result<(bool, u32), String>),
        U: FnOnce(S) -> (S, u32), B: FnOnce(S, &T) -> (S, bool),
        D: FnOnce(S, T) -> (S, String),
    requires query.requires((source,)),
        forall|state: S| last_error.requires((state,)),
        forall|state: S| address.requires((state,)),
        forall|state: S| metadata.requires((state,)),
        forall|state: S| user.requires((state,)),
        forall|state: S, address: &T| binding.requires((state, address)),
        forall|state: S, address: T| describe.requires((state, address)),
    ensures activation_sequence_result(source, named, datagram, query, last_error,
        address, metadata, user, binding, describe, out),
{
    let (state, raw) = query(source);
    let ghost queried = state;
    assert(query.ensures((source,), (queried, raw)));
    if raw.0 != 0 {
        let (state, error) = last_error(state);
        assert(last_error.ensures((queried,), (state, error)));
        return (state, Err(error));
    }
    if raw.1 != datagram {
        return (state, Err("OS activation requires a Unix datagram socket".to_string()));
    }
    let (state, result) = address(state);
    let ghost observed = state;
    let ghost observation = result;
    assert(address.ensures((queried,), (observed, observation)));
    let value = match result {
        Err(error) => return (state, Err(error)),
        Ok(value) => value,
    };
    check_activation_path(state, named, value, metadata, user, binding, describe)
}
}

#[cfg(not(verus_keep_ghost))]
use super::*;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::maintenance_signal::{self as signal, Policy as ServicePolicy, RoundGuard};
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
use std::os::{
    fd::{AsRawFd, FromRawFd},
    unix::net::UnixDatagram,
};
#[cfg(not(verus_keep_ghost))]
use std::time::{Duration, Instant};

#[cfg(not(verus_keep_ghost))]
pub(super) fn handoff(inputs: &PolicyInputs, reason: &str) -> Value {
    value!({"schema":"chrono-maintenance-handoff/v1","reason":reason,
        "coordinator_root":inputs.policy.coordinator_root,"responsibility":"existing-owner-state",
        "activation":"after-original-publication","accepted":false,
        "round_completion":"not-established","disposal":"not-attempted"})
}

#[cfg(not(verus_keep_ghost))]
fn service_policy(r: &mut Runner, inputs: &PolicyInputs) -> Result<Option<ServicePolicy>, String> {
    let root = &inputs.policy.coordinator_root;
    let Some((policy, bytes)) = signal::adopted(root)? else {
        return Ok(None);
    };
    start::registered_policy(
        &start::registrations(
            r,
            root,
            &inputs.anchor_head,
            &decode::<crate::Config>(&inputs.anchor_config_bytes)?.host_config,
        )?
        .0,
        signal::POLICY_PATH,
        &inputs.policy.state_directory,
    )?;
    if r.blob(root, &inputs.anchor_head, signal::POLICY_PATH)? != bytes
        || policy.state_directory != inputs.anchor_policy.state_directory
    {
        return Err("maintenance policy differs from committed coordinator ownership".into());
    }
    Ok(Some(policy))
}

/// Singleton is independent of enrollment metadata and is never acquired by business.
#[cfg(not(verus_keep_ghost))]
fn singleton(root: &Path, directory: &str) -> Result<Option<Lease>, String> {
    let directory = no_symlink_parents(root, directory.trim_end_matches('/'))?;
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    Lease::acquire(&directory.join("maintenance.lease"), None, true, true, None)
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn maintain(
    root: &Path,
    config_path: &str,
    trigger: &str,
    consumer: Option<&str>,
) -> Result<Value, String> {
    round(root, config_path, trigger, None, None, consumer)
}

#[cfg(not(verus_keep_ghost))]
fn round(
    root: &Path,
    config_path: &str,
    trigger: &str,
    held: Option<&Lease>,
    missed: Option<u64>,
    consumer: Option<&str>,
) -> Result<Value, String> {
    let _internal = RoundGuard::enter();
    let (config, bytes) = crate::configuration(root, config_path)?;
    // Keep executor ownership through the final parent publication and recovery handoff.
    let executor = std::cell::RefCell::new(None);
    start::with_report_for_consumer(
        root,
        config_path,
        config,
        &bytes,
        "maintain",
        "maintained",
        consumer,
        |r, report| {
            let inputs = PolicyInputs::load(r, root, config_path, &bytes)?
                .ok_or("automatic cleanup is not opted in")?;
            let policy = service_policy(r, &inputs)?;
            report["maintenance_policy"] = value!(policy);
            if let Some(held) = held {
                if root != inputs.policy.coordinator_root.as_path() {
                    return Err("maintenance service must run at the physical coordinator".into());
                }
                held.stable()?;
            } else {
                *executor.borrow_mut() = singleton(
                    &inputs.policy.coordinator_root,
                    &inputs.policy.state_directory,
                )?;
            }
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
                    .ok_or("maintenance executor busy without adopted activation")?
                    .0
                    .socket_path(&inputs.policy.coordinator_root)?;
                let sent = signal::send(&socket, b"explicit");
                report["maintenance_request"] = match sent {
                    Ok(()) => {
                        value!({"accepted":true,"status":"requested","round_completion":"not-established"})
                    }
                    Err(error) => {
                        value!({"accepted":false,"status":"wake-failed","error":error,"round_completion":"not-established"})
                    }
                };
                if report["maintenance_request"]["accepted"] != true {
                    return Err("maintenance executor busy; activation was not accepted; existing responsibilities retained".into());
                }
                return Ok(());
            }
            report["round_started"] = value!(true);
            run_round(r, token, report, &inputs, root)
        },
    )
}

#[cfg(not(verus_keep_ghost))]
fn linked_retention_status(
    r: &mut Runner,
    token: &str,
    inputs: &PolicyInputs,
    path: &Path,
) -> Result<&'static str, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok("absent"),
        Err(error) => return Err(error.to_string()),
        Ok(_) => {}
    }
    // Opening Inventory can create its namespace. Only inspect adoption here;
    // absent policies need no attachment probes, and foreign roots get no writes.
    let policy = no_symlink_parents(
        path,
        chrono_harness::retained_artifacts::retention::POLICY_PATH,
    )?;
    match fs::symlink_metadata(policy) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok("not-adopted"),
        Err(error) => return Err(error.to_string()),
        Ok(_) => {}
    }
    let owner = Manager::open_metadata(
        r,
        &inputs.policy.coordinator_root,
        &inputs.config_path,
        &inputs.anchor_config_bytes,
        token,
        path,
        false,
        false,
        false,
    )?
    .ok_or("cleanup policy disappeared")?;
    let Some(index) = owner.current_entry(path) else {
        return Ok("no-current-enrollment");
    };
    let head = r.oid(path, "HEAD")?;
    owner.check_attachment(r, &owner.ledger.entries[index], &head, None)?;
    Ok("ready")
}

#[cfg(not(verus_keep_ghost))]
fn maintain_logs(
    root: &Path,
    inventory: &mut inventory::Report,
    failures: &mut Vec<String>,
    results: &mut Vec<Value>,
) {
    let scope = format!("runtime logs: {}", root.display());
    let result = match chrono_harness::maintain_runtime_logs(root) {
        Ok(None) => return,
        Ok(Some(result)) => {
            for error in &result.errors {
                inventory.error(&scope, error);
                failures.push(format!("{scope}: {error}"));
            }
            value!({"source_root":root.to_str(),"log_retention":result})
        }
        Err(error) => {
            let error = error.to_string();
            inventory.error(&scope, &error);
            failures.push(format!("{scope}: {error}"));
            value!({"source_root":root.to_str(),"error":error})
        }
    };
    results.push(result);
}

#[cfg(not(verus_keep_ghost))]
fn maintain_retention_inventory(
    root: &Path,
    inventory: &mut inventory::Report,
) -> Result<Option<Value>, String> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Err("retention root is not a physical directory; preserve it".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            inventory.retention(root, value!({"kind":"source-result","status":"absent"}));
            return Ok(None);
        }
        Err(error) => return Err(error.to_string()),
    }
    let root = absolute(root, false)?;
    let Some(owner) = chrono_harness::retained_artifacts::retention::Inventory::adopted(&root)?
    else {
        inventory.retention(
            &root,
            value!({"kind":"source-result","status":"not-adopted"}),
        );
        return Ok(None);
    };
    let result = owner.maintain_explicit_observed(|row| {
        inventory.retention(&root, row.clone());
        Ok(())
    })?;
    // Individual observations are already complete above. Keep source accounting
    // and quota errors without duplicating the compact samples in the parent.
    let mut summary = result.clone();
    summary.as_object_mut().unwrap().remove("samples");
    inventory.retention(&root, value!({"kind":"source-result","original":summary}));
    Ok(Some(result))
}

#[cfg(not(verus_keep_ghost))]
fn run_round(
    r: &mut Runner,
    token: &str,
    report: &mut Value,
    inputs: &PolicyInputs,
    invoking_root: &Path,
) -> Result<(), String> {
    let root = &inputs.policy.coordinator_root;
    let exclude = maintenance_exclusions(root.clone(), invoking_root.to_path_buf());
    let directory = no_symlink_parents(root, inputs.policy.state_directory.trim_end_matches('/'))?;
    // Shared deployment custody protects executable/policy identity, never business metadata.
    let namespace = open_namespace(inputs, &directory, false, None)?;
    let observation = Observation {
        inputs,
        directory: &directory,
        scope: root,
    };
    report["drain"] = value!([]);
    report["cleanup_failures"] = value!([]);
    report["producer_drains"] = value!([]);
    report["maintenance_scope"] = value!({"kind":"declared-enrollments"});
    let mut inventory = inventory::Report::begin(root, report)?;
    if let Err(error) = observation.observe_inventory(r, &exclude, report, true, &BTreeSet::new()) {
        inventory.error("git-inventory", error);
    }
    inventory.worktrees(&observation, r, report, "before");
    let mut failures = Vec::new();
    let mut unprotected = Vec::new();
    let mut linked_retention_roots = BTreeSet::new();
    match retirement::resume_retired_addresses(inputs, &directory, &namespace, r, report) {
        Ok(errors) => failures.extend(errors),
        Err(error) => failures.push(error),
    }
    // Enumeration errors do not silently omit members or prevent independent producers.
    let mut namespace_complete = true;
    let mut paths = match observation.enrollment_paths_observed(|row| {
        namespace_complete = false;
        inventory.error("enrollment-namespace", row);
    }) {
        Ok(paths) => paths,
        Err(error) => {
            namespace_complete = false;
            inventory.error("enrollment-namespace", &error);
            failures.push(error);
            vec![]
        }
    };
    // The committed coordinator policy declares its artifacts even before a
    // cache enrollment exists. Keep the original explicit-maintain observation.
    if !paths.contains(root) {
        paths.push(root.clone());
        paths.sort();
    }
    for path in paths {
        // Observe adopted declarations, then DROP enrollment exclusion before footprint IO.
        let snapshot = (|| {
            let mut owner = Manager::open_metadata(
                r,
                root,
                &inputs.config_path,
                &inputs.anchor_config_bytes,
                token,
                &path,
                false,
                false,
                true,
            )?
            .ok_or("cleanup policy disappeared")?;
            // A verified enrollment is the association; an empty admission or
            // an inventory-only address cannot authorize a foreign store.
            if &path != root && !owner.ledger.entries.is_empty() {
                linked_retention_roots.insert(path.clone());
            }
            owner.restore_absent_policy(r)?;
            if crate::verified_lifecycle::enrolled_policy_required(owner.ledger.entries.len()) {
                owner.adopt_cleanup_policy(r, true)?;
            }
            Ok::<_, String>(inventory::ArtifactSnapshot::from_owner(&owner))
        })();
        match snapshot {
            Ok(snapshot) => inventory.artifacts(&snapshot),
            Err(error) => {
                inventory.error(&path.to_string_lossy(), &error);
            }
        }
        // Every actual effect reopens the original owner and validates current identity,
        // policy, lease, generation and source custody. Snapshots grant no permission.
        let result = (|| {
            let mut owner = Manager::open_metadata(
                r,
                root,
                &inputs.config_path,
                &inputs.anchor_config_bytes,
                token,
                &path,
                false,
                false,
                true,
            )?
            .ok_or("cleanup policy disappeared")?;
            owner.restore_absent_policy(r)?;
            observe_boundary("disposition", &path);
            let errors = owner.drain_entries(r, &exclude, report, true)?;
            owner.retire_empty_address(r, report)?;
            Ok::<_, String>(errors)
        })();
        match result {
            Ok(errors) => failures.extend(errors),
            Err(error) => {
                // Reopen CURRENT durable metadata under its own lease. This
                // validates identity and protects every declared original before
                // returning, without interpreting the checkout's cleanup policy.
                // A pre-drain ledger or footprint snapshot cannot establish that.
                let protection = Manager::open_metadata(
                    r,
                    root,
                    &inputs.config_path,
                    &inputs.anchor_config_bytes,
                    token,
                    &path,
                    false,
                    false,
                    false,
                )
                .and_then(|owner| owner.ok_or("cleanup policy disappeared".into()));
                let original_protection = match protection {
                    Ok(_) => value!({"status":"protected","source":"current-enrollment-ledger"}),
                    Err(protection_error) => {
                        unprotected.push(path.clone());
                        value!({"status":"unknown","error":protection_error})
                    }
                };
                let busy =
                    error.contains("metadata is busy") || error.contains("metadata remains busy");
                report["drain"]
                    .as_array_mut()
                    .unwrap()
                    .push(value!({"path":path,
                    "status":if busy {"preserved"} else {"failed"},"error":error,
                    "original_protection":original_protection}));
                if !busy {
                    failures.push(format!("{}: {error}", path.display()));
                }
            }
        }
    }
    for producer in inputs.anchor_policy.retained_producers.clone() {
        // No enrollment metadata is held during producer IO or its full observation.
        observe_boundary("producer", root);
        producer.observe_inventory(inputs, &mut inventory);
        let attempt = (|| {
            let (config, bytes) = crate::configuration(root, &inputs.config_path)?;
            let result = start::with_report(
                root,
                &inputs.config_path,
                config,
                &bytes,
                "producer-cleanup",
                "cleaned",
                |r, _, report| {
                    report["producer_objects"] = value!([]);
                    producer.drain(inputs, r, report)
                },
            )?;
            let path = result["report_path"]
                .as_str()
                .ok_or("producer drain report path")?;
            let bytes = crate::recovery::state_bytes(root, path)?;
            report["producer_drains"]
                .as_array_mut()
                .unwrap()
                .push(value!({"producer":producer.id,
                "status":result["status"],"error":result["error"],
                "receipt":{"path":path,"sha256":sha256(&bytes)},
                "input":crate::recovery::retained_input(root, path, &bytes, "receipt")}));
            if result["status"] != "cleaned" {
                return Err(format!("producer {}: {}", producer.id, result["error"]));
            }
            Ok::<_, String>(())
        })();
        if let Err(error) = attempt {
            inventory.error(&producer.id, &error);
            failures.push(error);
        }
    }
    let (_, retention) = maintenance_retention_phase(
        (&mut inventory, &mut failures),
        namespace_complete,
        unprotected.len(),
        |(inventory, failures)| {
            let result = maintain_retention_inventory(root, inventory);
            ((inventory, failures), result)
        },
        |state, result| (state, value!(result)),
        |(inventory, failures), error| {
            inventory.error("retention", &error);
            failures.push(error.clone());
            (
                (inventory, failures),
                value!({"status":"failed","error":error}),
            )
        },
        |(inventory, failures)| {
            inventory.error(
                "retention",
                "enrollment original protection unavailable; dependent retention deferred",
            );
            (
                (inventory, failures),
                value!({"status":"preserved",
                "reason":"enrollment original protection unavailable",
                "namespace_complete":namespace_complete,"unprotected_enrollments":unprotected}),
            )
        },
    );
    report["output_retention"] = retention;
    let mut linked_retention = Vec::new();
    let mut runtime_logs = Vec::new();
    maintain_logs(root, &mut inventory, &mut failures, &mut runtime_logs);
    for path in linked_retention_roots {
        if unprotected.contains(&path) {
            inventory.retention(
                &path,
                value!({"kind":"source-result","status":"preserved",
                "reason":"current enrollment original protection unavailable"}),
            );
            linked_retention.push(value!({"source_root":path,"status":"preserved",
                "reason":"current enrollment original protection unavailable"}));
            continue;
        }
        // The linked owner protects its own outputs. Unknown coordinator
        // originals defer the coordinator store, not independent linked stores.
        // No enrollment metadata lease spans this retention IO.
        let result = (|| {
            let status = linked_retention_status(r, token, inputs, &path)?;
            if status != "ready" {
                inventory.retention(&path, value!({"kind":"source-result","status":status}));
                return Ok(status);
            }
            maintain_logs(&path, &mut inventory, &mut failures, &mut runtime_logs);
            let result = maintain_retention_inventory(&path, &mut inventory)?;
            Ok::<_, String>(if result.is_some() {
                "reported"
            } else {
                "no-adopted-storage"
            })
        })();
        match result {
            Ok(status) => linked_retention.push(value!({"source_root":path,
                "status":status,
                "original":"see complete_inventory"})),
            Err(error) => {
                let scope = format!("retention:{}", path.display());
                inventory.error(&scope, &error);
                failures.push(format!("{scope}: {error}"));
                linked_retention.push(value!({"source_root":path,"status":"failed","error":error}));
            }
        }
    }
    report["linked_output_retention"] = value!(linked_retention);
    report["runtime_logs"] = value!(runtime_logs);
    inventory.worktrees(&observation, r, report, "after");
    if let Err(error) = inventory.finish(report) {
        failures.push(error);
    }
    report["cleanup_failures"] = value!(failures);
    report["owned_work_joined"] = value!(true); // All work above is synchronous; no children are detached.
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("maintenance incomplete: {}", failures.join("; ")))
    }
}

#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn activated_socket() -> Result<UnixDatagram, String> {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn launch_activate_socket(
                name: *const libc::c_char,
                fds: *mut *mut libc::c_int,
                count: *mut libc::size_t,
            ) -> libc::c_int;
        }
        let mut fds = std::ptr::null_mut();
        let mut count = 0;
        let result =
            unsafe { launch_activate_socket(c"Maintenance".as_ptr(), &mut fds, &mut count) };
        if result != 0 {
            return Err(format!("launch_activate_socket failed: {result}"));
        }
        if fds.is_null() {
            return Err("activation returned no socket descriptors".into());
        }
        let descriptors = unsafe { std::slice::from_raw_parts(fds, count) };
        if count != 1 {
            for fd in descriptors {
                unsafe {
                    libc::close(*fd);
                }
            }
            unsafe {
                libc::free(fds.cast());
            }
            return Err("maintenance activation requires exactly one socket".into());
        }
        let fd = descriptors[0];
        unsafe {
            libc::free(fds.cast());
        }
        let socket = unsafe { UnixDatagram::from_raw_fd(fd) };
        return Ok(socket);
    }
    #[cfg(not(target_os = "macos"))]
    {
        if std::env::var("LISTEN_PID")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            != Some(std::process::id())
            || std::env::var("LISTEN_FDS").ok().as_deref() != Some("1")
        {
            return Err("service requires one OS-owned activation socket".into());
        }
        Ok(unsafe { UnixDatagram::from_raw_fd(3) })
    }
}

/// Narrow, injectable service loop used by the Rust socket boundary expectations.
/// The supplied round must join its work; this function owns no detached tasks.
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
pub fn serve(
    socket: &UnixDatagram,
    policy: &ServicePolicy,
    mut execute: impl FnMut(&str, u64) -> Result<(), String>,
) -> Result<(), String> {
    policy.validate()?;
    socket.set_nonblocking(true).map_err(|e| e.to_string())?;
    let descriptor = socket.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    if flags == -1
        || unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) } == -1
    {
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
            let missed = if timer_due {
                now.duration_since(next_timer).as_secs() / policy.timer_seconds
            } else {
                0
            };
            let trigger = if timer_due {
                "timer"
            } else if messages != 0 {
                "event"
            } else {
                "os-activation"
            };
            // Advance from the original cadence before execution. Long rounds
            // cannot erase timer opportunities that occur while they are running.
            if timer_due {
                let periods = missed.checked_add(1).ok_or("timer opportunity overflow")?;
                let seconds = periods
                    .checked_mul(policy.timer_seconds)
                    .ok_or("timer overflow")?;
                next_timer = next_timer
                    .checked_add(Duration::from_secs(seconds))
                    .ok_or("timer overflow")?;
            }
            // Failed rounds retain originals but do not generate immediate retry loops.
            execute(trigger, missed)?;
            first = false;
            last_work = Instant::now();
            continue;
        }
        let idle = policy
            .idle_seconds
            .map(|seconds| {
                last_work
                    .checked_add(Duration::from_secs(seconds))
                    .ok_or("idle deadline overflow")
            })
            .transpose()?;
        if maintenance_idle_exit(
            messages != 0,
            timer_due,
            idle.is_some_and(|deadline| now >= deadline),
        ) {
            // OS still owns the socket. A racing/new datagram remains activatable.
            return Ok(());
        }
        let deadline = idle.map_or(next_timer, |idle| idle.min(next_timer));
        let timeout = deadline
            .saturating_duration_since(now)
            .as_millis()
            .min(i32::MAX as u128) as i32;
        let mut fd = libc::pollfd {
            fd: descriptor,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut fd, 1, timeout.max(1)) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted {
                return Err(error.to_string());
            }
        } else if fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            return Err("maintenance activation socket lost".into());
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn dispatch(args: &[String]) -> Result<Value, String> {
    let mut root = PathBuf::from(".");
    let mut config_path = ".chrono-harness/worktree.json".to_string();
    let mut seen = BTreeSet::new();
    let mut index = 1;
    while index < args.len() {
        let key = args[index].as_str();
        if index + 1 == args.len() || !seen.insert(key) {
            return Err("invalid service argument".into());
        }
        match key {
            "--host-root" => root = args[index + 1].clone().into(),
            "--config" => config_path = args[index + 1].clone(),
            _ => return Err("unknown service argument".into()),
        }
        index += 2;
    }
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        run_service(
            &root,
            &config_path,
            cfg!(target_os = "macos"),
            activated_socket,
        )
    }
    #[cfg(not(unix))]
    {
        Err("maintenance service requires Unix socket activation".into())
    }
}

#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
fn run_service(
    root: &Path,
    config_path: &str,
    named_activation: bool,
    activate: impl FnOnce() -> Result<UnixDatagram, String>,
) -> Result<Value, String> {
    let (policy, _) = signal::adopted(&root)?.ok_or("maintenance service policy is not adopted")?;
    let socket = activate()?;
    validate_activation(&socket, &policy.socket_path(&root)?, named_activation)?;
    let mut executor = None;
    serve(&socket, &policy, |trigger, missed| {
        if executor.is_none() {
            executor = singleton(&root, &policy.state_directory)?;
        }
        let Some(executor) = executor.as_ref() else {
            // Consume activation hints through the existing finite batch and
            // native wait. Durable owner state and the next event/timer retain
            // responsibility; sending to this socket would reactivate us.
            eprintln!("maintenance executor busy; no round started; existing owner state retained");
            return Ok(());
        };
        let report = round(
            &root,
            &config_path,
            trigger,
            Some(executor),
            Some(missed),
            None,
        )?;
        // Each round already has its own retained original. Do not accumulate reports.
        eprintln!(
            "{}",
            serde_json::to_string(&value!({"schema":"chrono-maintenance-round-reference/v1",
            "source_root":root,"report_path":report["report_path"],"status":report["status"],
            "trigger":trigger,"missed_timer_opportunities":missed}))
            .map_err(|e| e.to_string())?
        );
        Ok(())
    })?;
    Ok(value!({"status":"maintenance-idle-exit","owned_work_joined":true}))
}

/// launchd may forward a named socket from a private address. Its returned
/// descriptor is selected by the job's Maintenance entry, not getsockname text.
#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
pub fn validate_activation(
    socket: &UnixDatagram,
    expected: &Path,
    named_activation: bool,
) -> Result<(), String> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let (_, result) = validate_activation_sequence(
        (),
        named_activation,
        libc::SOCK_DGRAM,
        |()| {
            let mut kind: libc::c_int = 0;
            let mut length = std::mem::size_of_val(&kind) as libc::socklen_t;
            let status = unsafe {
                libc::getsockopt(
                    socket.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_TYPE,
                    (&mut kind as *mut libc::c_int).cast(),
                    &mut length,
                )
            };
            ((), (status, kind))
        },
        |()| ((), std::io::Error::last_os_error().to_string()),
        |()| ((), socket.local_addr().map_err(|error| error.to_string())),
        |()| {
            (
                (),
                fs::symlink_metadata(expected)
                    .map(|metadata| (metadata.file_type().is_socket(), metadata.uid()))
                    .map_err(|error| error.to_string()),
            )
        },
        |()| ((), unsafe { libc::geteuid() }),
        |(), address: &std::os::unix::net::SocketAddr| {
            ((), address.as_pathname() == Some(expected))
        },
        |(), address: std::os::unix::net::SocketAddr| {
            (
                (),
                format!(
                    "OS activation socket differs from adopted coordinator: {address:?}; expected {}",
                    expected.display()
                ),
            )
        },
    );
    result
}

/// Rust OS-provider mock exercises the production singleton and round path.
#[cfg(all(unix, feature = "producer-birth-test"))]
#[cfg(not(verus_keep_ghost))]
pub fn run_with_socket(
    root: &Path,
    config_path: &str,
    socket: UnixDatagram,
) -> Result<Value, String> {
    let root = fs::canonicalize(root).map_err(|error| error.to_string())?;
    run_service(&root, config_path, false, || Ok(socket))
}

#[cfg(feature = "producer-birth-test")]
#[cfg(not(verus_keep_ghost))]
thread_local! {
    static BOUNDARY: std::cell::RefCell<Option<Box<dyn FnMut(&str, &Path)>>> = const { std::cell::RefCell::new(None) };
}
#[cfg(feature = "producer-birth-test")]
#[cfg(not(verus_keep_ghost))]
pub fn with_observer<T>(observer: impl FnMut(&str, &Path) + 'static, run: impl FnOnce() -> T) -> T {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            BOUNDARY.with(|slot| {
                slot.borrow_mut().take();
            });
        }
    }
    BOUNDARY.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Box::new(observer));
    });
    let _reset = Reset;
    run()
}
#[cfg(not(verus_keep_ghost))]
pub(super) fn observe_boundary(kind: &str, path: &Path) {
    #[cfg(feature = "producer-birth-test")]
    BOUNDARY.with(|slot| {
        if let Some(observer) = slot.borrow_mut().as_mut() {
            observer(kind, path);
        }
    });
    #[cfg(not(feature = "producer-birth-test"))]
    let _ = (kind, path);
}
