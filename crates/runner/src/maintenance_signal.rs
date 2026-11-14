//! Nonblocking hints to the existing maintenance owner. No task or disposal state.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
verus! {
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
        relative_path(self.state_directory.trim_end_matches('/'))?;
        relative_path(&self.socket)?;
        if self.schema != "chrono-unified-maintenance/v1"
            || !self.state_directory.starts_with(".chrono-harness/state/")
            || !self.state_directory.ends_with('/')
            || !self.socket.starts_with(".chrono-harness/state/")
            || self.socket.ends_with('/')
            || self.timer_seconds == 0
            || self.idle_seconds == Some(0)
        {
            return Err("invalid explicit maintenance policy".into());
        }
        Ok(())
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
    let mut receipt = json!({"schema":"chrono-maintenance-request/v1", "reason":reason,
        "accepted":false,"round_completion":"not-established", "disposal":"not-attempted",
        "responsibility":"existing-owner-state"});
    if IN_ROUND.with(|depth| depth.get() != 0) {
        receipt["status"] = json!("maintenance-internal-no-new-event");
        return receipt;
    }
    let request = (|| {
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
