//! Nonblocking hints to the existing maintenance owner. No task or disposal state.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
verus! {
pub fn valid_hint_length(bytes: usize) -> (valid: bool)
    ensures valid == (0 < bytes && bytes <= 64),
{ bytes > 0 && bytes <= 64 }
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
    if !valid_hint_length(bytes.len()) {
        return Err("maintenance hint exceeds 64 bytes".into());
    }
    let sender = UnixDatagram::unbound().map_err(|e| e.to_string())?;
    sender.set_nonblocking(true).map_err(|e| e.to_string())?;
    let sent = sender.send_to(bytes, socket).map_err(|e| e.to_string())?;
    if sent != bytes.len() {
        return Err("incomplete maintenance datagram".into());
    }
    Ok(())
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
