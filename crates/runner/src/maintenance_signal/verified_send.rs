//! Same-source maintenance datagram ordering and completion.
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
