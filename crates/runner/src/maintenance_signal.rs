//! Nonblocking hints to the existing maintenance owner. No task or disposal state.
#[path = "maintenance_signal/verified_send.rs"]
pub mod verified_send;
use verified_send::send_hint;
pub use verified_send::{
    valid_hint_length, valid_maintenance_location, valid_maintenance_policy_fields, validate_policy,
};

use crate::{decode, no_symlink_parents, relative_path, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
};

pub const POLICY_PATH: &str = ".chrono-harness/maintenance.json";

/// Routing context from an already admitted lifecycle caller. This is only an
/// activation destination, never an ownership capability or disposal policy.
pub const CONTEXT_ENV: &str = "CHRONO_MAINTENANCE_CONTEXT";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestContext {
    source_root: PathBuf,
    owner_root: PathBuf,
}

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
pub struct Policy {
    pub schema: String,
    pub state_directory: String,
    pub socket: String,
    pub idle_seconds: Option<u64>,
    pub timer_seconds: u64,
}
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

thread_local! { static IN_ROUND: Cell<usize> = const { Cell::new(0) }; }
/// Maintenance's own report publication/release is not new external work.
/// The guard is local to the executing thread and never suppresses callers.
pub struct RoundGuard;
impl RoundGuard {
    pub fn enter() -> Self {
        IN_ROUND.with(|depth| depth.set(depth.get() + 1));
        Self
    }
}
impl Drop for RoundGuard {
    fn drop(&mut self) {
        IN_ROUND.with(|depth| depth.set(depth.get() - 1));
    }
}

/// A datagram is an activation hint, not an acknowledgement from the executor.
/// Responsibility was durably recorded by the caller's original owner first.
pub fn request_adopted(root: &Path, reason: &str) -> Value {
    let context = std::env::var(CONTEXT_ENV).ok();
    request_with_context(root, reason, context.as_deref())
}

/// Explicit context is also available to admitted in-process callers. A route
/// for a different source cannot redirect this host's event.
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
pub fn send(_: &Path, _: &[u8]) -> Result<(), String> {
    Err("maintenance activation requires an adopted Unix socket provider".into())
}

/// Void APIs retain their original result and expose activation separately.
pub fn announce_adopted(root: &Path, reason: &str) {
    let receipt = request_adopted(root, reason);
    if !matches!(
        receipt["status"].as_str(),
        Some("not-adopted" | "maintenance-internal-no-new-event")
    ) {
        eprintln!("W_MAINTENANCE_ACTIVATION: {}", receipt);
    }
}
