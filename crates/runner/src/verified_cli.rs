//! 字符串 CLI 的完整参数顺序与首个解码失败；OS 解码由标准库提供。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

verus! {

pub struct InvalidArgument {
    pub argv_index: usize,
    pub argument_index: Option<usize>,
}

pub const MAX_JUDGE_REQUEST_BYTES: usize = 64 * 1024 * 1024;
pub const JUDGE_READ_LIMIT: u64 = 64 * 1024 * 1024 + 1;

pub enum JudgeMode { Version, Protocol, Usage }

pub open spec fn judge_mode_spec(args: Seq<&str>) -> JudgeMode {
    if args.len() == 1 && args[0]@ == "--version"@ {
        JudgeMode::Version
    } else if args.len() == 2
        && args[0]@ == "--protocol"@
        && args[1]@ == "chrono-judge/v1"@ {
        JudgeMode::Protocol
    } else { JudgeMode::Usage }
}

pub fn judge_mode(args: &[&str]) -> (mode: JudgeMode)
    ensures mode == judge_mode_spec(args@),
{
    if args.len() == 1 && identifiers::equal_text(args[0], "--version") {
        JudgeMode::Version
    } else if args.len() == 2
        && identifiers::equal_text(args[0], "--protocol")
        && identifiers::equal_text(args[1], "chrono-judge/v1") {
        JudgeMode::Protocol
    } else { JudgeMode::Usage }
}

pub fn judge_request_admissible(read_ok: bool, length: usize) -> (allowed: bool)
    ensures allowed == (read_ok && length <= 67108864),
{
    read_ok && length <= MAX_JUDGE_REQUEST_BYTES
}

pub enum JudgeOutcome<E> {
    Version,
    Usage,
    ReadBound,
    Codec(E),
    OutputFailed,
    Delivered(u8),
}

pub open spec fn judge_publish_result<S, E, WE, O>(
    state: S, bytes: Vec<u8>, exit: u8, publish: O, out: (S, JudgeOutcome<E>),
) -> bool where O: FnOnce(S, Vec<u8>) -> (S, Result<(), WE>),
{
    exists|after: S, result: Result<(), WE>|
        #[trigger] publish.ensures((state, bytes), (after, result)) && out.0 == after
        && out.1 == match result { Err(_) => JudgeOutcome::<E>::OutputFailed,
            Ok(_) => JudgeOutcome::<E>::Delivered(exit) }
}

pub open spec fn judge_encode_result<S, P, E, WE, C, O>(
    state: S, response: P, exit: u8, encode: C, publish: O, out: (S, JudgeOutcome<E>),
) -> bool where C: FnOnce(S, P) -> (S, Result<Vec<u8>, E>),
    O: FnOnce(S, Vec<u8>) -> (S, Result<(), WE>),
{
    exists|after: S, encoded: Result<Vec<u8>, E>|
        #[trigger] encode.ensures((state, response), (after, encoded))
        && match encoded { Err(error) => out == (after, JudgeOutcome::Codec(error)),
            Ok(bytes) => judge_publish_result(after, bytes, exit, publish, out) }
}

pub open spec fn judge_evaluation_result<S, Q, P, E, WE, J, C, O>(
    state: S, request: Q, judge: J, encode: C, publish: O, out: (S, JudgeOutcome<E>),
) -> bool where J: FnOnce(S, Q) -> (S, P, u8),
    C: FnOnce(S, P) -> (S, Result<Vec<u8>, E>),
    O: FnOnce(S, Vec<u8>) -> (S, Result<(), WE>),
{
    exists|after: S, response: P, exit: u8|
        #[trigger] judge.ensures((state, request), (after, response, exit))
        && judge_encode_result(after, response, exit, encode, publish, out)
}

pub open spec fn judge_decode_result<S, Q, P, E, WE, D, J, C, O>(
    state: S, bytes: Vec<u8>, decode: D, judge: J, encode: C, publish: O, out: (S, JudgeOutcome<E>),
) -> bool where D: FnOnce(S, Vec<u8>) -> (S, Result<Q, E>),
    J: FnOnce(S, Q) -> (S, P, u8), C: FnOnce(S, P) -> (S, Result<Vec<u8>, E>),
    O: FnOnce(S, Vec<u8>) -> (S, Result<(), WE>),
{
    exists|after: S, decoded: Result<Q, E>|
        #[trigger] decode.ensures((state, bytes), (after, decoded))
        && match decoded { Err(error) => out == (after, JudgeOutcome::Codec(error)),
            Ok(request) => judge_evaluation_result(after, request, judge, encode, publish, out) }
}

/// 状态逐阶段转交；规范保留真实接口观察、完整载荷和首错停止后的状态。
pub open spec fn judge_protocol_result<S, Q, P, E, RE, WE, R, D, J, C, O>(
    initial: S, read: R, decode: D, judge: J, encode: C, publish: O,
    out: (S, JudgeOutcome<E>),
) -> bool
    where R: FnOnce(S) -> (S, Result<Vec<u8>, RE>),
        D: FnOnce(S, Vec<u8>) -> (S, Result<Q, E>),
        J: FnOnce(S, Q) -> (S, P, u8),
        C: FnOnce(S, P) -> (S, Result<Vec<u8>, E>),
        O: FnOnce(S, Vec<u8>) -> (S, Result<(), WE>),
{
    exists|s1: S, input: Result<Vec<u8>, RE>|
        #[trigger] read.ensures((initial,), (s1, input))
        && match input {
            Err(_) => out == (s1, JudgeOutcome::<E>::ReadBound),
            Ok(bytes) => if bytes.len() > 67108864 {
                out == (s1, JudgeOutcome::<E>::ReadBound)
            } else {
                judge_decode_result(s1, bytes, decode, judge, encode, publish, out)
            },
        }
}

pub fn run_judge_entry<S, Q, P, E, RE, WE, R, D, J, C, O>(
    state: S, args: &[&str], read: R, decode: D, judge: J, encode: C, publish: O,
) -> (out: (S, JudgeOutcome<E>))
    where R: FnOnce(S) -> (S, Result<Vec<u8>, RE>),
        D: FnOnce(S, Vec<u8>) -> (S, Result<Q, E>),
        J: FnOnce(S, Q) -> (S, P, u8),
        C: FnOnce(S, P) -> (S, Result<Vec<u8>, E>),
        O: FnOnce(S, Vec<u8>) -> (S, Result<(), WE>),
    requires
        read.requires((state,)),
        forall|s: S, bytes: Vec<u8>| decode.requires((s, bytes)),
        forall|s: S, request: Q| judge.requires((s, request)),
        forall|s: S, response: P| encode.requires((s, response)),
        forall|s: S, bytes: Vec<u8>| publish.requires((s, bytes)),
    ensures match judge_mode_spec(args@) {
        JudgeMode::Version => out == (state, JudgeOutcome::<E>::Version),
        JudgeMode::Usage => out == (state, JudgeOutcome::<E>::Usage),
        JudgeMode::Protocol => judge_protocol_result(state, read, decode, judge, encode, publish, out),
    },
{
    match judge_mode(args) {
        JudgeMode::Version => return (state, JudgeOutcome::Version),
        JudgeMode::Usage => return (state, JudgeOutcome::Usage),
        JudgeMode::Protocol => {},
    }
    let (state, input) = read(state);
    let bytes = match input {
        Err(_) => return (state, JudgeOutcome::ReadBound),
        Ok(bytes) => bytes,
    };
    if !judge_request_admissible(true, bytes.len()) {
        return (state, JudgeOutcome::ReadBound);
    }
    let (state, decoded) = decode(state, bytes);
    let request = match decoded {
        Err(error) => return (state, JudgeOutcome::Codec(error)),
        Ok(request) => request,
    };
    let (state, response, exit) = judge(state, request);
    let (state, encoded) = encode(state, response);
    let bytes = match encoded {
        Err(error) => return (state, JudgeOutcome::Codec(error)),
        Ok(bytes) => bytes,
    };
    let (state, result) = publish(state, bytes);
    match result {
        Err(_) => (state, JudgeOutcome::OutputFailed),
        Ok(_) => (state, JudgeOutcome::Delivered(exit)),
    }
}

pub fn finish_judge_entry<S, E, V, U, B, F>(
    state: S, outcome: JudgeOutcome<E>, version: V, usage: U, bound: B, codec: F,
) -> (out: (S, u8))
    where V: FnOnce(S) -> S, U: FnOnce(S) -> S, B: FnOnce(S) -> S,
        F: FnOnce(S, E) -> S,
    requires version.requires((state,)), usage.requires((state,)), bound.requires((state,)),
        forall|error: E| codec.requires((state, error)),
    ensures match outcome {
        JudgeOutcome::Version => version.ensures((state,), out.0) && out.1 == 0,
        JudgeOutcome::Usage => usage.ensures((state,), out.0) && out.1 == 2,
        JudgeOutcome::ReadBound => bound.ensures((state,), out.0) && out.1 == 2,
        JudgeOutcome::Codec(error) => codec.ensures((state, error), out.0) && out.1 == 2,
        JudgeOutcome::OutputFailed => out == (state, 2u8),
        JudgeOutcome::Delivered(exit) => out == (state, exit),
    },
{
    match outcome {
        JudgeOutcome::Version => (version(state), 0),
        JudgeOutcome::Usage => (usage(state), 2),
        JudgeOutcome::ReadBound => (bound(state), 2),
        JudgeOutcome::Codec(error) => (codec(state, error), 2),
        JudgeOutcome::OutputFailed => (state, 2),
        JudgeOutcome::Delivered(exit) => (state, exit),
    }
}

pub open spec fn argument_position(index: usize) -> Option<usize> {
    if index == 0 { None } else { Some((index - 1) as usize) }
}

/// 成功保持完整原文、顺序、空串和重复；失败精确返回首个 None。
/// argv[0] 没有用户参数序号，其余参数的序号从零开始。
pub fn decoded_arguments<'a>(arguments: &[Option<&'a str>])
    -> (out: Result<Vec<&'a str>, InvalidArgument>)
    ensures match out {
        Ok(texts) => texts.len() == arguments.len()
            && forall|j: int| 0 <= j < arguments.len() ==> (
                arguments[j].is_some()
                && texts[j]@ == arguments[j].unwrap()@
            ),
        Err(error) => error.argv_index < arguments.len()
            && arguments[error.argv_index as int].is_none()
            && error.argument_index == argument_position(error.argv_index)
            && forall|j: int| 0 <= j < error.argv_index ==> arguments[j].is_some(),
    },
{
    let mut texts: Vec<&'a str> = Vec::new();
    let mut i = 0;
    while i < arguments.len()
        invariant
            i <= arguments.len(), texts.len() == i,
            forall|j: int| 0 <= j < i ==> (
                arguments[j].is_some()
                && texts[j]@ == arguments[j].unwrap()@
            ),
        decreases arguments.len() - i,
    {
        match arguments[i] {
            Some(text) => texts.push(text),
            None => return Err(InvalidArgument {
                argv_index: i,
                argument_index: if i == 0 { None } else { Some(i - 1) },
            }),
        }
        i += 1;
    }
    Ok(texts)
}

}
