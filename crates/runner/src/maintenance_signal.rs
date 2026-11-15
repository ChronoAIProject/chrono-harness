//! Nonblocking hints to the existing maintenance owner. No task or disposal state.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;
verus! {
#[verifier::inline]
pub open spec fn maintenance_location(path: &str, directory: bool) -> bool {
    let bytes = path.spec_bytes();
    let prefix = ".chrono-harness/state/".spec_bytes();
    bytes.len() >= prefix.len() && bytes.take(prefix.len() as int) == prefix
        && (bytes.last() == 47) == directory
}

pub fn valid_maintenance_location(path: &str, directory: bool) -> (valid: bool)
    ensures valid == maintenance_location(path, directory),
{
    let bytes = path.as_bytes();
    let prefix = ".chrono-harness/state/".as_bytes();
    proof {
        reveal_strlit(".chrono-harness/state/");
        assert(vstd::string::is_ascii(".chrono-harness/state/"));
        vstd::string::is_ascii_spec_bytes(".chrono-harness/state/");
    }
    bytes.len() >= prefix.len()
        && identifiers::equal_bytes(&bytes[0..prefix.len()], prefix)
        && (bytes[bytes.len() - 1] == 47) == directory
}

#[verifier::inline]
pub open spec fn maintenance_policy_fields(schema: &str, directory: &str, socket: &str,
    idle: Option<u64>, timer: u64) -> bool
{
    schema@ == "chrono-unified-maintenance/v1"@
        && maintenance_location(directory, true) && maintenance_location(socket, false)
        && timer > 0 && idle != Some(0u64)
}

pub fn valid_maintenance_policy_fields(schema: &str, directory: &str, socket: &str,
    idle: Option<u64>, timer: u64) -> (valid: bool)
    ensures valid == maintenance_policy_fields(schema, directory, socket, idle, timer),
{
    identifiers::equal_text(schema, "chrono-unified-maintenance/v1")
        && valid_maintenance_location(directory, true) && valid_maintenance_location(socket, false)
        && timer > 0 && idle != Some(0)
}

pub open spec fn policy_validation_result<'a, S, A, B>(source: S,
    schema: &str, directory: &'a str, socket: &'a str, idle: Option<u64>, timer: u64,
    directory_check: A, socket_check: B, out: (S, Result<(), String>)) -> bool
    where A: FnOnce(S, &'a str) -> (S, Result<(), String>),
        B: FnOnce(S, &'a str) -> (S, Result<(), String>),
{
    exists|checked: S, result: Result<(), String>|
        #[trigger] directory_check.ensures((source, directory), (checked, result))
        && match result {
            Err(error) => out == (checked, Err::<(), String>(error)),
            Ok(()) => exists|checked_socket: S, socket_result: Result<(), String>|
                #[trigger] socket_check.ensures((checked, socket), (checked_socket, socket_result))
                && match socket_result {
                    Err(error) => out == (checked_socket, Err::<(), String>(error)),
                    Ok(()) => out.0 == checked_socket && if maintenance_policy_fields(
                        schema, directory, socket, idle, timer) { out.1 == Ok::<(), String>(()) }
                        else { match out.1 {
                            Err(error) => error@ == "invalid explicit maintenance policy"@,
                            _ => false,
                        } },
                },
        }
}

/// 先核目录、再核 socket，最后判断字段；首错保留原状态和完整错误。
pub fn validate_policy<'a, S, A, B>(source: S, schema: &str, directory: &'a str,
    socket: &'a str, idle: Option<u64>, timer: u64, directory_check: A, socket_check: B)
    -> (out: (S, Result<(), String>))
    where A: FnOnce(S, &'a str) -> (S, Result<(), String>),
        B: FnOnce(S, &'a str) -> (S, Result<(), String>),
    requires directory_check.requires((source, directory)),
        forall|state: S| socket_check.requires((state, socket)),
    ensures policy_validation_result(source, schema, directory, socket, idle, timer,
        directory_check, socket_check, out),
{
    let (state, result) = directory_check(source, directory);
    let ghost checked = state;
    let ghost directory_result = result;
    assert(directory_check.ensures((source, directory), (checked, directory_result)));
    match result {
        Err(error) => return (state, Err(error)),
        Ok(()) => (),
    }
    let (state, result) = socket_check(state, socket);
    let ghost checked_socket = state;
    let ghost socket_result = result;
    assert(socket_check.ensures((checked, socket), (checked_socket, socket_result)));
    match result {
        Err(error) => return (state, Err(error)),
        Ok(()) => (),
    }
    if valid_maintenance_policy_fields(schema, directory, socket, idle, timer) {
        (state, Ok(()))
    } else {
        (state, Err("invalid explicit maintenance policy".to_string()))
    }
}

pub fn valid_hint_length(bytes: usize) -> (valid: bool)
    ensures valid == (0 < bytes && bytes <= 64),
{ bytes > 0 && bytes <= 64 }

pub open spec fn hint_send_completion<S>(source: S, bytes: usize,
    sent: Result<usize, String>, out: (S, Result<(), String>)) -> bool
{
    out.0 == source && match sent {
        Err(error) => out.1 == Err::<(), String>(error),
        Ok(count) => if count == bytes { out.1 == Ok::<(), String>(()) }
            else { match out.1 {
                Err(error) => error@ == "incomplete maintenance datagram"@,
                _ => false,
            } },
    }
}

pub open spec fn hint_send_result<S, T, O, B, W>(source: S, bytes: usize,
    open: O, nonblocking: B, send: W, out: (S, Result<(), String>)) -> bool
    where O: FnOnce(S) -> (S, Result<T, String>),
        B: FnOnce(S, &T, bool) -> (S, Result<(), String>),
        W: FnOnce(S, T) -> (S, Result<usize, String>),
{
    if bytes == 0 || bytes > 64 {
        out.0 == source && match out.1 {
            Err(error) => error@ == "maintenance hint exceeds 64 bytes"@,
            _ => false,
        }
    } else {
        exists|opened: S, socket: Result<T, String>|
            #[trigger] open.ensures((source,), (opened, socket))
            && match socket {
                Err(error) => out == (opened, Err::<(), String>(error)),
                Ok(socket) => exists|configured: S, result: Result<(), String>|
                    #[trigger] nonblocking.ensures((opened, &socket, true),
                        (configured, result))
                    && match result {
                        Err(error) => out == (configured, Err::<(), String>(error)),
                        Ok(()) => exists|sent: Result<usize, String>|
                            #[trigger] send.ensures((configured, socket), (out.0, sent))
                            && hint_send_completion(out.0, bytes, sent, out),
                    },
            }
    }
}

/// 只发送合法长度的提示；先启用非阻塞，首错保留原文，核对实际发送长度。
pub fn send_hint<S, T, O, B, W>(source: S, bytes: usize,
    open: O, nonblocking: B, send: W) -> (out: (S, Result<(), String>))
    where O: FnOnce(S) -> (S, Result<T, String>),
        B: FnOnce(S, &T, bool) -> (S, Result<(), String>),
        W: FnOnce(S, T) -> (S, Result<usize, String>),
    requires open.requires((source,)),
        forall|state: S, socket: &T, enabled: bool| nonblocking.requires((state, socket, enabled)),
        forall|state: S, socket: T| send.requires((state, socket)),
    ensures hint_send_result(source, bytes, open, nonblocking, send, out),
{
    if !valid_hint_length(bytes) {
        return (source, Err("maintenance hint exceeds 64 bytes".to_string()));
    }
    let (state, result) = open(source);
    let ghost opened = state;
    let ghost observed_socket = result;
    assert(open.ensures((source,), (opened, observed_socket)));
    let socket = match result {
        Err(error) => return (state, Err(error)),
        Ok(socket) => socket,
    };
    let ghost original_socket = socket;
    let (state, result) = nonblocking(state, &socket, true);
    let ghost configured = state;
    let ghost configuration = result;
    assert(nonblocking.ensures((opened, &original_socket, true),
        (configured, configuration)));
    match result {
        Err(error) => return (state, Err(error)),
        Ok(()) => (),
    }
    let (state, result) = send(state, socket);
    let ghost sent = result;
    assert(send.ensures((configured, original_socket), (state, sent)));
    let out = match result {
        Err(error) => (state, Err(error)),
        Ok(count) => if count == bytes { (state, Ok(())) }
            else { (state, Err("incomplete maintenance datagram".to_string())) },
    };
    assert(hint_send_completion(state, bytes, sent, out));
    out
}
}

#[cfg(not(verus_keep_ghost))]
use crate::{decode, no_symlink_parents, relative_path, sha256};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
pub const POLICY_PATH: &str = ".chrono-harness/maintenance.json";

/// Routing context from an already admitted lifecycle caller. This is only an
/// activation destination, never an ownership capability or disposal policy.
#[cfg(not(verus_keep_ghost))]
pub const CONTEXT_ENV: &str = "CHRONO_MAINTENANCE_CONTEXT";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct RequestContext {
    source_root: PathBuf,
    owner_root: PathBuf,
}

#[cfg(not(verus_keep_ghost))]
impl RequestContext {
    pub fn new(source_root: &Path, owner_root: &Path) -> Result<Self, String> {
        let context = Self {
            source_root: source_root.into(),
            owner_root: owner_root.into(),
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> Result<(), String> {
        self.validate_shape()?;
        for root in [&self.source_root, &self.owner_root] {
            if fs::canonicalize(root).map_err(|error| error.to_string())? != *root {
                return Err("maintenance context requires physical absolute roots".into());
            }
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<(), String> {
        for root in [&self.source_root, &self.owner_root] {
            if !root.is_absolute()
                || root.components().any(|part| {
                    matches!(
                        part,
                        std::path::Component::CurDir | std::path::Component::ParentDir
                    )
                })
            {
                return Err("maintenance context requires physical absolute roots".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Policy {
    pub schema: String,
    pub state_directory: String,
    pub socket: String,
    pub idle_seconds: Option<u64>,
    pub timer_seconds: u64,
}
#[cfg(not(verus_keep_ghost))]
impl Policy {
    pub fn validate(&self) -> Result<(), String> {
        validate_policy(
            (),
            &self.schema,
            &self.state_directory,
            &self.socket,
            self.idle_seconds,
            self.timer_seconds,
            |(), path| ((), relative_path(path.trim_end_matches('/'))),
            |(), path| ((), relative_path(path)),
        )
        .1
    }
    pub fn socket_path(&self, root: &Path) -> Result<PathBuf, String> {
        self.validate()?;
        no_symlink_parents(root, &self.socket)
    }
}

#[cfg(not(verus_keep_ghost))]
pub fn adopted(root: &Path) -> Result<Option<(Policy, Vec<u8>)>, String> {
    let path = no_symlink_parents(root, POLICY_PATH)?;
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let policy: Policy = decode(&bytes)?;
    policy.validate()?;
    Ok(Some((policy, bytes)))
}

#[cfg(not(verus_keep_ghost))]
thread_local! { static IN_ROUND: Cell<usize> = const { Cell::new(0) }; }
/// Maintenance's own report publication/release is not new external work.
/// The guard is local to the executing thread and never suppresses callers.
#[cfg(not(verus_keep_ghost))]
pub struct RoundGuard;
#[cfg(not(verus_keep_ghost))]
impl RoundGuard {
    pub fn enter() -> Self {
        IN_ROUND.with(|depth| depth.set(depth.get() + 1));
        Self
    }
}
#[cfg(not(verus_keep_ghost))]
impl Drop for RoundGuard {
    fn drop(&mut self) {
        IN_ROUND.with(|depth| depth.set(depth.get() - 1));
    }
}

/// A datagram is an activation hint, not an acknowledgement from the executor.
/// Responsibility was durably recorded by the caller's original owner first.
#[cfg(not(verus_keep_ghost))]
pub fn request_adopted(root: &Path, reason: &str) -> Value {
    let context = std::env::var(CONTEXT_ENV).ok();
    request_with_context(root, reason, context.as_deref())
}

/// Explicit context is also available to admitted in-process callers. A route
/// for a different source cannot redirect this host's event.
#[cfg(not(verus_keep_ghost))]
pub fn request_with_context(root: &Path, reason: &str, context: Option<&str>) -> Value {
    let mut receipt = json!({"schema":"chrono-maintenance-request/v1", "reason":reason,
        "accepted":false,"round_completion":"not-established", "disposal":"not-attempted",
        "responsibility":"existing-owner-state"});
    if IN_ROUND.with(|depth| depth.get() != 0) {
        receipt["status"] = json!("maintenance-internal-no-new-event");
        return receipt;
    }
    let request = (|| {
        let context = context
            .map(|raw| decode::<RequestContext>(raw.as_bytes()))
            .transpose()?;
        if let Some(context) = &context {
            context.validate_shape()?;
        }
        let context = context
            .as_ref()
            .filter(|context| context.source_root == root);
        if let Some(context) = context {
            context.validate()?;
        }
        let root = context.map_or(root, |context| context.owner_root.as_path());
        receipt["activation_root"] = json!(root);
        let Some((policy, bytes)) = adopted(root)? else {
            return Ok(false);
        };
        receipt["policy_sha256"] = json!(sha256(&bytes));
        let socket = policy.socket_path(root)?;
        receipt["socket"] = json!(socket);
        send(&socket, reason.as_bytes())?;
        Ok::<_, String>(true)
    })();
    match request {
        Ok(true) => {
            receipt["status"] = json!("requested");
            receipt["accepted"] = json!(true);
        }
        Ok(false) => receipt["status"] = json!("not-adopted"),
        Err(error) => {
            receipt["status"] = json!("wake-failed");
            receipt["error"] = json!(error);
        }
    }
    receipt
}

#[cfg(unix)]
#[cfg(not(verus_keep_ghost))]
pub fn send(socket: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::os::unix::net::UnixDatagram;
    send_hint(
        (),
        bytes.len(),
        |()| {
            (
                (),
                UnixDatagram::unbound().map_err(|error| error.to_string()),
            )
        },
        |(), sender: &UnixDatagram, enabled| {
            let result = sender
                .set_nonblocking(enabled)
                .map_err(|error| error.to_string());
            ((), result)
        },
        |(), sender: UnixDatagram| {
            (
                (),
                sender
                    .send_to(bytes, socket)
                    .map_err(|error| error.to_string()),
            )
        },
    )
    .1
}
#[cfg(not(unix))]
#[cfg(not(verus_keep_ghost))]
pub fn send(_: &Path, _: &[u8]) -> Result<(), String> {
    Err("maintenance activation requires an adopted Unix socket provider".into())
}

/// Void APIs retain their original result and expose activation separately.
#[cfg(not(verus_keep_ghost))]
pub fn announce_adopted(root: &Path, reason: &str) {
    let receipt = request_adopted(root, reason);
    if !matches!(
        receipt["status"].as_str(),
        Some("not-adopted" | "maintenance-internal-no-new-event")
    ) {
        eprintln!("W_MAINTENANCE_ACTIVATION: {}", receipt);
    }
}
