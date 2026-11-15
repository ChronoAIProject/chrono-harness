//! Same-source maintenance policy admission, datagram ordering and completion.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
#[path = "../../../identifiers/src/lib.rs"]
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
