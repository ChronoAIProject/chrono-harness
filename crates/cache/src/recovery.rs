//! Discard registered host outputs after unusable native restores, before producers run.
#![allow(unexpected_cfgs)]

#[cfg(verus_keep_ghost)]
use super as planner;
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_planner as planner;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// 文件系统仅提供原 metadata、错误种类及原错误文本。
pub open spec fn recovery_removal_result<S, M, D, F>(
    initial: S, not_found: u32, metadata: M, directory: D, file: F,
    out: (S, Result<&'static str, String>),
) -> bool
    where M: FnOnce(S) -> (S, Result<(bool, bool), (u32, String)>),
        D: FnOnce(S) -> (S, Result<(), String>),
        F: FnOnce(S) -> (S, Result<(), String>),
{
    exists|observed: S, observation: Result<(bool, bool), (u32, String)>|
        #[trigger] metadata.ensures((initial,), (observed, observation))
        && match observation {
            Err((kind, error)) => out.0 == observed && if kind == not_found {
                out.1 == Ok::<&'static str, String>("absent")
            } else {
                out.1 == Err::<&'static str, String>(error)
            },
            Ok((is_directory, is_file)) => if !is_directory && !is_file {
                out.0 == observed && match out.1 {
                    Err(error) => error@ ==
                        "cache artifact is neither a regular file nor a directory"@,
                    Ok(_) => false,
                }
            } else {
                exists|removed: Result<(), String>|
                    (if is_directory {
                        directory.ensures((observed,), (out.0, removed))
                    } else {
                        file.ensures((observed,), (out.0, removed))
                    }) && match removed {
                        Ok(()) => out.1 == Ok::<&'static str, String>("removed"),
                        Err(error) => out.1 == Err::<&'static str, String>(error),
                    }
            },
        }
}

/// 先观察一次；仅删除真实目录或普通文件，完整保留首个原错误。
pub fn remove_recovery_output<S, M, D, F>(
    state: S, not_found: u32, metadata: M, directory: D, file: F,
) -> (out: (S, Result<&'static str, String>))
    where M: FnOnce(S) -> (S, Result<(bool, bool), (u32, String)>),
        D: FnOnce(S) -> (S, Result<(), String>),
        F: FnOnce(S) -> (S, Result<(), String>),
    requires metadata.requires((state,)),
        forall|state: S| directory.requires((state,)),
        forall|state: S| file.requires((state,)),
    ensures recovery_removal_result(state, not_found, metadata, directory, file, out),
{
    let ghost initial = state;
    let (state, observation) = metadata(state);
    let ghost observed = state;
    let ghost original = observation;
    let out = match observation {
        Err((kind, error)) => if kind == not_found {
            (state, Ok("absent"))
        } else {
            (state, Err(error))
        },
        Ok((is_directory, is_file)) => if !is_directory && !is_file {
            (state, Err("cache artifact is neither a regular file nor a directory".to_string()))
        } else {
            let (state, removed) = if is_directory { directory(state) } else { file(state) };
            let ghost removal_result = removed;
            let out = match removed {
                Ok(()) => (state, Ok("removed")),
                Err(error) => (state, Err(error)),
            };
            assert(if is_directory {
                directory.ensures((observed,), (out.0, removal_result))
            } else {
                file.ensures((observed,), (out.0, removal_result))
            });
            out
        },
    };
    assert(metadata.ensures((initial,), (observed, original)));
    out
}

/// 外层错误是路径读取失败；内层保留 metadata 的原种类和原文本。
pub type RecoveryProbe = Result<Result<(), (u32, String)>, String>;
pub type RecoveryEntryError = (&'static str, String, &'static str);

pub open spec fn recovery_probe_matches(
    observation: RecoveryProbe, not_found: u32, prefix: &str,
    out: Result<bool, RecoveryEntryError>,
) -> bool {
    match observation {
        Err(error) => match out {
            Err(parts) => parts.0@ == ""@ && parts.1 == error && parts.2@ == ""@,
            _ => false,
        },
        Ok(Ok(())) => out == Ok::<bool, RecoveryEntryError>(true),
        Ok(Err((kind, error))) => if kind == not_found {
            out == Ok::<bool, RecoveryEntryError>(false)
        } else { match out {
            Err(parts) => parts.0@ == prefix@ && parts.1 == error && parts.2@ == ""@,
            _ => false,
        } },
    }
}

fn classify_recovery_probe(observation: RecoveryProbe, not_found: u32,
    prefix: &'static str) -> (out: Result<bool, RecoveryEntryError>)
    ensures recovery_probe_matches(observation, not_found, prefix, out),
{
    match observation {
        Err(error) => Err(("", error, "")),
        Ok(Ok(())) => Ok(true),
        Ok(Err((kind, error))) => if kind == not_found { Ok(false) }
            else { Err((prefix, error, "")) },
    }
}

pub open spec fn recovery_entry_result<S, T, O, I, R>(
    initial: S, not_found: u32, intent_path: &str,
    output: O, intent: I, replay: R, out: (S, Result<Option<T>, RecoveryEntryError>),
) -> bool
    where O: FnOnce(S) -> (S, RecoveryProbe),
        I: FnOnce(S) -> (S, RecoveryProbe),
        R: FnOnce(S) -> (S, Result<T, String>),
{
    exists|observed: S, observation: RecoveryProbe, checked: Result<bool, RecoveryEntryError>|
        #[trigger] output.ensures((initial,), (observed, observation))
        && #[trigger] recovery_probe_matches(observation, not_found,
            "E_CACHE_RECOVERY: result metadata: ", checked)
        && match checked {
            Err(error) => out.0 == observed && out.1 == Err::<Option<T>, RecoveryEntryError>(error),
            Ok(true) => exists|result: Result<T, String>|
                #[trigger] replay.ensures((observed,), (out.0, result))
                && match result {
                    Ok(value) => out.1 == Ok::<Option<T>, RecoveryEntryError>(Some(value)),
                    Err(error) => out.1 == Err::<Option<T>, RecoveryEntryError>(("", error, "")),
                },
            Ok(false) => exists|probed: S, probe: RecoveryProbe,
                checked: Result<bool, RecoveryEntryError>|
                #[trigger] intent.ensures((observed,), (probed, probe))
                && out.0 == probed && #[trigger] recovery_probe_matches(probe, not_found,
                    "E_CACHE_RECOVERY: intent metadata: ", checked)
                && match checked {
                    Err(error) => out.1 == Err::<Option<T>, RecoveryEntryError>(error),
                    Ok(false) => out.1 == Ok::<Option<T>, RecoveryEntryError>(None),
                    Ok(true) => match out.1 {
                        Err(parts) => parts.0@ == "E_CACHE_RECOVERY: unfinished original intent "@
                            && parts.1@ == intent_path@
                            && parts.2@ == "; no automatic repeat"@,
                        _ => false,
                    },
                },
        }
}

/// 已有结果只复用；只有结果和意图均缺失才允许新的恢复操作。
pub fn select_recovery_entry<S, T, O, I, R>(
    state: S, not_found: u32, intent_path: &str, output: O, intent: I, replay: R,
) -> (out: (S, Result<Option<T>, RecoveryEntryError>))
    where O: FnOnce(S) -> (S, RecoveryProbe),
        I: FnOnce(S) -> (S, RecoveryProbe),
        R: FnOnce(S) -> (S, Result<T, String>),
    requires output.requires((state,)),
        forall|state: S| intent.requires((state,)),
        forall|state: S| replay.requires((state,)),
    ensures recovery_entry_result(state, not_found, intent_path, output, intent, replay, out),
{
    let ghost initial = state;
    let (state, observation) = output(state);
    let ghost observed = state;
    let ghost original = observation;
    let checked = classify_recovery_probe(observation, not_found,
        "E_CACHE_RECOVERY: result metadata: ");
    let ghost output_checked = checked;
    let out = match checked {
        Err(error) => (state, Err(error)),
        Ok(true) => {
            let (state, result) = replay(state);
            let ghost replayed = result;
            let out = (state, match result {
                Ok(value) => Ok(Some(value)),
                Err(error) => Err(("", error, "")),
            });
            assert(replay.ensures((observed,), (out.0, replayed)));
            out
        },
        Ok(false) => {
            let (state, observation) = intent(state);
            let ghost probed = state;
            let ghost original_intent = observation;
            let checked = classify_recovery_probe(observation, not_found,
                "E_CACHE_RECOVERY: intent metadata: ");
            let ghost intent_checked = checked;
            let out = (state, match checked {
                Err(error) => Err(error),
                Ok(false) => Ok(None),
                Ok(true) => Err(("E_CACHE_RECOVERY: unfinished original intent ",
                    intent_path.to_string(), "; no automatic repeat")),
            });
            assert(intent.ensures((observed,), (probed, original_intent)));
            assert(recovery_probe_matches(original_intent, not_found,
                "E_CACHE_RECOVERY: intent metadata: ", intent_checked));
            out
        },
    };
    assert(output.ensures((initial,), (observed, original)));
    assert(recovery_probe_matches(original, not_found,
        "E_CACHE_RECOVERY: result metadata: ", output_checked));
    out
}

pub type RecoveryRemovalError = (&'static str, String, &'static str, String);

pub open spec fn recovery_removal_step<S, R, C, F>(
    state: S, index: usize, path: &String, remove: &R, success: &C, failure: &F,
    out: (S, Result<(), RecoveryRemovalError>),
) -> bool
    where R: Fn(S, usize) -> (S, Result<&'static str, String>),
        C: Fn(S, usize, &'static str) -> S, F: Fn(S, usize, &String) -> S,
{
    exists|observed: S, removed: Result<&'static str, String>|
        #[trigger] remove.ensures((state, index), (observed, removed))
        && match removed {
            Ok(status) => success.ensures((observed, index, status), out.0)
                && out.1 == Ok::<(), RecoveryRemovalError>(()),
            Err(error) => failure.ensures((observed, index, &error), out.0)
                && match out.1 {
                    Err(parts) => parts.0@ == "E_CACHE_RECOVERY: "@ && parts.1 == *path
                        && parts.2@ == ": "@ && parts.3 == error,
                    _ => false,
                },
        }
}

fn remove_recovery_row<S, R, C, F>(
    state: S, index: usize, path: &String, remove: &R, success: &C, failure: &F,
) -> (out: (S, Result<(), RecoveryRemovalError>))
    where R: Fn(S, usize) -> (S, Result<&'static str, String>),
        C: Fn(S, usize, &'static str) -> S, F: Fn(S, usize, &String) -> S,
    requires remove.requires((state, index)),
        forall|state: S, index: usize, status: &'static str| success.requires((state, index, status)),
        forall|state: S, index: usize, error: &String| failure.requires((state, index, error)),
    ensures recovery_removal_step(state, index, path, remove, success, failure, out),
{
    let ghost initial = state;
    let (state, removed) = remove(state, index);
    let ghost observed = state;
    let ghost original = removed;
    let out = match removed {
        Ok(status) => (success(state, index, status), Ok(())),
        Err(error) => {
            let state = failure(state, index, &error);
            (state, Err(("E_CACHE_RECOVERY: ", path.clone(), ": ", error)))
        },
    };
    assert(remove.ensures((initial, index), (observed, original)));
    out
}

pub open spec fn recovery_removal_prefix<S, R, C, F>(
    paths: Seq<(String, usize, String)>, states: Seq<S>, initial: S,
    remove: &R, success: &C, failure: &F,
) -> bool
    where R: Fn(S, usize) -> (S, Result<&'static str, String>),
        C: Fn(S, usize, &'static str) -> S, F: Fn(S, usize, &String) -> S,
{
    0 < states.len() <= paths.len() + 1 && states[0] == initial
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] recovery_removal_step(states[i], i as usize, &paths[i].2,
                remove, success, failure, (states[i + 1], Ok::<(), RecoveryRemovalError>(())))
}

pub open spec fn recovery_paths_result<S, R, C, F>(
    paths: Seq<(String, usize, String)>, initial: S,
    remove: &R, success: &C, failure: &F, out: (S, Result<(), RecoveryRemovalError>),
) -> bool
    where R: Fn(S, usize) -> (S, Result<&'static str, String>),
        C: Fn(S, usize, &'static str) -> S, F: Fn(S, usize, &String) -> S,
{
    exists|states: Seq<S>| #[trigger] recovery_removal_prefix(paths, states, initial,
        remove, success, failure)
        && match out.1 {
            Ok(()) => states.len() == paths.len() + 1 && out.0 == states.last(),
            Err(error) => states.len() <= paths.len()
                && recovery_removal_step(states.last(), (states.len() - 1) as usize,
                    &paths[states.len() - 1].2, remove, success, failure,
                    (out.0, Err::<(), RecoveryRemovalError>(error))),
        }
}

pub open spec fn recovery_removals_result<S, P, R, C, F>(
    initial: S, paths: Seq<(String, usize, String)>, publish: P,
    remove: &R, success: &C, failure: &F, out: (S, Result<(), RecoveryRemovalError>),
) -> bool
    where P: FnOnce(S) -> (S, Result<(), String>),
        R: Fn(S, usize) -> (S, Result<&'static str, String>),
        C: Fn(S, usize, &'static str) -> S, F: Fn(S, usize, &String) -> S,
{
    exists|published: S, result: Result<(), String>|
        #[trigger] publish.ensures((initial,), (published, result))
        && match result {
            Err(error) => out.0 == published && match out.1 {
                Err(parts) => parts.0@ == ""@ && parts.1 == error
                    && parts.2@ == ""@ && parts.3@ == ""@,
                _ => false,
            },
            Ok(()) => recovery_paths_result(paths, published, remove, success, failure, out),
        }
}

/// 先发布一次原意图，再按完整原顺序删除；首错记录后立即返回。
pub fn run_recovery_removals<S, P, R, C, F>(
    source: S, paths: &Vec<(String, usize, String)>, publish: P,
    remove: &R, success: &C, failure: &F,
) -> (out: (S, Result<(), RecoveryRemovalError>))
    where P: FnOnce(S) -> (S, Result<(), String>),
        R: Fn(S, usize) -> (S, Result<&'static str, String>),
        C: Fn(S, usize, &'static str) -> S, F: Fn(S, usize, &String) -> S,
    requires publish.requires((source,)),
        forall|state: S, index: usize| index < paths.len() ==> remove.requires((state, index)),
        forall|state: S, index: usize, status: &'static str| success.requires((state, index, status)),
        forall|state: S, index: usize, error: &String| failure.requires((state, index, error)),
    ensures recovery_removals_result(source, paths@, publish, remove, success, failure, out),
{
    let published_observation = publish(source);
    let (mut state, result) = published_observation;
    let ghost published = state;
    let ghost publication = result;
    assert(publish.ensures((source,), (published, publication)));
    match result { Err(error) => { return (state, Err(("", error, "", "".to_string()))); }, Ok(()) => {} }
    assert(publication is Ok);
    let ghost states = seq![published];
    let mut i = 0;
    while i < paths.len()
        invariant 0 <= i <= paths.len(), states.len() == i + 1, states.last() == state,
            recovery_removal_prefix(paths@, states, published, remove, success, failure),
            publish.ensures((source,), (published, publication)), publication is Ok,
            forall|state: S, index: usize| index < paths.len() ==> remove.requires((state, index)),
            forall|state: S, index: usize, status: &'static str| success.requires((state, index, status)),
            forall|state: S, index: usize, error: &String| failure.requires((state, index, error)),
        decreases paths.len() - i,
    {
        let (next, result) = remove_recovery_row(state, i, &paths[i].2, remove, success, failure);
        match result {
            Err(error) => {
                let out = (next, Err(error));
                assert(recovery_paths_result(paths@, published, remove, success, failure, out));
                assert(recovery_removals_result(source, paths@, publish, remove, success, failure, out));
                return out;
            },
            Ok(()) => {
                proof {
                    let previous = states;
                    states = states.push(next);
                    assert forall|j: int| 0 <= j < states.len() - 1 implies
                        #[trigger] recovery_removal_step(states[j], j as usize, &paths@[j].2,
                            remove, success, failure, (states[j + 1], Ok::<(), RecoveryRemovalError>(()))) by {
                        if j < previous.len() - 1 {
                            assert(recovery_removal_step(previous[j], j as usize, &paths@[j].2,
                                remove, success, failure, (previous[j + 1], Ok::<(), RecoveryRemovalError>(()))) );
                        }
                    }
                }
                state = next;
            },
        }
        i += 1;
    }
    let out = (state, Ok(()));
    assert(recovery_paths_result(paths@, published, remove, success, failure, out));
    out
}

pub enum RecoveryOption {
    HostRoot,
    Config,
    Plan,
    StepsEnvironment,
}

impl RecoveryOption {
    pub open spec fn slot(&self) -> int {
        match self {
            Self::HostRoot => 0,
            Self::Config => 1,
            Self::Plan => 2,
            Self::StepsEnvironment => 3,
        }
    }

    pub open spec fn missing(&self) -> Seq<char> {
        match self {
            Self::HostRoot => "E_CACHE_RECOVERY: missing --host-root"@,
            Self::Config => "E_CACHE_RECOVERY: missing --config"@,
            Self::Plan => "E_CACHE_RECOVERY: missing --plan"@,
            Self::StepsEnvironment => "E_CACHE_RECOVERY: missing --steps-env"@,
        }
    }
}

pub open spec fn recovery_arg_slot(text: Seq<char>) -> int {
    if text == "--host-root"@ { 0 }
    else if text == "--config"@ { 1 }
    else if text == "--plan"@ { 2 }
    else if text == "--steps-env"@ { 3 }
    else { 4 }
}

fn recovery_option(text: &str) -> (out: Option<RecoveryOption>)
    ensures match out {
        Some(key) => key.slot() == recovery_arg_slot(text@),
        None => recovery_arg_slot(text@) == 4,
    },
{
    if identifiers::equal_text(text, "--host-root") { Some(RecoveryOption::HostRoot) }
    else if identifiers::equal_text(text, "--config") { Some(RecoveryOption::Config) }
    else if identifiers::equal_text(text, "--plan") { Some(RecoveryOption::Plan) }
    else if identifiers::equal_text(text, "--steps-env") { Some(RecoveryOption::StepsEnvironment) }
    else { None }
}

pub open spec fn recovery_text(value: Option<&str>) -> Option<Seq<char>> {
    match value { None => None, Some(text) => Some(text@) }
}

/// 恢复决策的错误分类与完整缓存身份；适配器只按顺序拼接两段原文。
pub fn recovery_decision_error<'a>(error: &planner::RecoveryDecisionError, id: &'a str)
    -> (parts: [&'a str; 2])
    ensures parts[1]@ == id@,
        parts[0]@ == match error {
            planner::RecoveryDecisionError::MalformedOutputs =>
                "E_CACHE_RECOVERY: malformed restore outputs for "@,
            planner::RecoveryDecisionError::UnknownOutcome =>
                "E_CACHE_RECOVERY: missing, cancelled or unknown restore outcome for "@,
        },
{
    let prefix = match error {
        planner::RecoveryDecisionError::MalformedOutputs =>
            "E_CACHE_RECOVERY: malformed restore outputs for ",
        planner::RecoveryDecisionError::UnknownOutcome =>
            "E_CACHE_RECOVERY: missing, cancelled or unknown restore outcome for ",
    };
    [prefix, id]
}

pub struct RecoveryOptions<'a> {
    host_root: Option<&'a str>,
    config: Option<&'a str>,
    plan: Option<&'a str>,
    steps_environment: Option<&'a str>,
}

impl<'a> RecoveryOptions<'a> {
    pub closed spec fn value(&self, slot: int) -> Option<Seq<char>> {
        if slot == 0 { recovery_text(self.host_root) }
        else if slot == 1 { recovery_text(self.config) }
        else if slot == 2 { recovery_text(self.plan) }
        else if slot == 3 { recovery_text(self.steps_environment) }
        else { None }
    }

    fn empty() -> (out: Self)
        ensures forall|slot: int| 0 <= slot < 4 ==> out.value(slot).is_none(),
    {
        Self { host_root: None, config: None, plan: None, steps_environment: None }
    }

    pub fn get(&self, key: &RecoveryOption) -> (out: Option<&'a str>)
        ensures recovery_text(out) == self.value(key.slot()),
    {
        match key {
            RecoveryOption::HostRoot => self.host_root,
            RecoveryOption::Config => self.config,
            RecoveryOption::Plan => self.plan,
            RecoveryOption::StepsEnvironment => self.steps_environment,
        }
    }

    /// 真实 CLI 在原有 IO 边界逐项读取必需值。
    pub fn required(&self, key: &RecoveryOption) -> (out: Result<&'a str, &'static str>)
        ensures match out {
            Ok(text) => self.value(key.slot()) == Some(text@),
            Err(error) => self.value(key.slot()).is_none() && error@ == key.missing(),
        },
    {
        match self.get(key) {
            Some(text) => Ok(text),
            None => Err(match key {
                RecoveryOption::HostRoot => "E_CACHE_RECOVERY: missing --host-root",
                RecoveryOption::Config => "E_CACHE_RECOVERY: missing --config",
                RecoveryOption::Plan => "E_CACHE_RECOVERY: missing --plan",
                RecoveryOption::StepsEnvironment => "E_CACHE_RECOVERY: missing --steps-env",
            }),
        }
    }

    fn insert(self, key: &RecoveryOption, text: &'a str)
        -> (out: Result<Self, &'static str>)
        ensures match out {
            Ok(next) => self.value(key.slot()).is_none()
                && forall|slot: int| 0 <= slot < 4 ==> next.value(slot)
                    == if slot == key.slot() { Some(text@) } else { self.value(slot) },
            Err(error) => self.value(key.slot()).is_some()
                && error@ == "E_CACHE_RECOVERY: invalid or duplicate recovery option"@,
        },
    {
        if self.get(key).is_some() {
            return Err("E_CACHE_RECOVERY: invalid or duplicate recovery option");
        }
        let mut next = self;
        match key {
            RecoveryOption::HostRoot => next.host_root = Some(text),
            RecoveryOption::Config => next.config = Some(text),
            RecoveryOption::Plan => next.plan = Some(text),
            RecoveryOption::StepsEnvironment => next.steps_environment = Some(text),
        }
        Ok(next)
    }
}

pub open spec fn recovery_pair_slot(args: Seq<String>, pair: int) -> int {
    recovery_arg_slot(args[2 * pair]@)
}

pub open spec fn recovery_prefix_valid(args: Seq<String>, pairs: int) -> bool {
    0 <= pairs && 2 * pairs <= args.len()
        && (forall|i: int| 0 <= i < pairs ==> recovery_pair_slot(args, i) < 4)
        && (forall|i: int, j: int| 0 <= i < j < pairs
            ==> recovery_pair_slot(args, i) != recovery_pair_slot(args, j))
}

pub open spec fn recovery_options_bound(args: Seq<String>, pairs: int,
    options: &RecoveryOptions) -> bool
{
    forall|slot: int| 0 <= slot < 4 ==> (
        (options.value(slot).is_some() <==> exists|i: int| 0 <= i < pairs
            && recovery_pair_slot(args, i) == slot)
        && (forall|i: int| 0 <= i < pairs && recovery_pair_slot(args, i) == slot
            ==> options.value(slot) == Some(args[2 * i + 1]@))
    )
}

pub open spec fn recovery_args_valid(args: Seq<String>) -> bool {
    args.len() % 2 == 0 && recovery_prefix_valid(args, (args.len() / 2) as int)
}

/// IO 前解析完整键值对，拒绝未知／重复键，原样借用每个值。
pub fn parse_recovery_options<'a>(args: &'a [String])
    -> (out: Result<RecoveryOptions<'a>, &'static str>)
    ensures match out {
        Ok(options) => recovery_args_valid(args@)
            && recovery_options_bound(args@, (args.len() / 2) as int, &options),
        Err(error) => !recovery_args_valid(args@)
            && error@ == "E_CACHE_RECOVERY: invalid or duplicate recovery option"@,
    },
{
    if args.len() % 2 != 0 {
        return Err("E_CACHE_RECOVERY: invalid or duplicate recovery option");
    }
    let pairs = args.len() / 2;
    let mut options = RecoveryOptions::empty();
    let mut i = 0usize;
    while i < pairs
        invariant i <= pairs, pairs == args.len() / 2, 2 * pairs == args.len(),
            recovery_prefix_valid(args@, i as int),
            recovery_options_bound(args@, i as int, &options),
        decreases pairs - i,
    {
        let key = match recovery_option(args[2 * i].as_str()) {
            Some(key) => key,
            None => {
                proof {
                    assert(recovery_pair_slot(args@, i as int) == 4);
                    assert(!recovery_args_valid(args@)) by {
                        if recovery_args_valid(args@) {
                            assert(recovery_prefix_valid(args@, pairs as int));
                            assert(recovery_pair_slot(args@, i as int) < 4);
                        }
                    }
                }
                return Err("E_CACHE_RECOVERY: invalid or duplicate recovery option");
            },
        };
        let ghost before = options;
        let ghost slot = key.slot();
        match options.insert(&key, args[2 * i + 1].as_str()) {
            Err(error) => {
                proof {
                    let prior = choose|j: int| 0 <= j < i
                        && recovery_pair_slot(args@, j) == slot;
                    assert(recovery_pair_slot(args@, prior)
                        == recovery_pair_slot(args@, i as int));
                    assert(!recovery_args_valid(args@));
                }
                return Err(error);
            },
            Ok(next) => {
                proof {
                    assert forall|p: int, q: int| 0 <= p < q < i + 1
                        implies recovery_pair_slot(args@, p)
                            != recovery_pair_slot(args@, q) by {
                        if q == i {
                            if recovery_pair_slot(args@, p) == slot {
                                assert(before.value(slot).is_some());
                            }
                        }
                    }
                    assert forall|s: int| 0 <= s < 4 implies (
                        (next.value(s).is_some() <==> exists|p: int| 0 <= p < i + 1
                            && recovery_pair_slot(args@, p) == s)
                        && (forall|p: int| 0 <= p < i + 1
                            && recovery_pair_slot(args@, p) == s
                            ==> next.value(s) == Some(args[2 * p + 1]@))
                    ) by {
                        if s == slot {
                            assert(exists|p: int| 0 <= p < i + 1
                                && recovery_pair_slot(args@, p) == s) by {
                                assert(recovery_pair_slot(args@, i as int) == s);
                            }
                        }
                    }
                }
                options = next;
            },
        }
        i += 1;
    }
    Ok(options)
}

pub fn admit_recovery_paths(config: &str, plan: &str) -> (out: Result<(), &'static str>)
    ensures match out {
        Ok(()) => planner::restore_prefix_matches(config, Some(".chrono-harness/"))
            && planner::restore_prefix_matches(plan, Some(".chrono-harness/state/")),
        Err(error) => !(planner::restore_prefix_matches(config, Some(".chrono-harness/"))
            && planner::restore_prefix_matches(plan, Some(".chrono-harness/state/")))
            && error@ == "E_CACHE_RECOVERY: expected host config and retained state plan"@,
    },
{
    if !planner::restore_prefix(config, Some(".chrono-harness/"))
        || !planner::restore_prefix(plan, Some(".chrono-harness/state/")) {
        return Err("E_CACHE_RECOVERY: expected host config and retained state plan");
    }
    Ok(())
}

/// 仅完成原 pending 状态，其余原 JSON 值由适配器保留。
pub fn complete_recovery_cache(status: Option<&str>) -> (out: Option<&'static str>)
    ensures match out {
        Some(next) => recovery_text(status) == Some("pending"@) && next@ == "discarded"@,
        None => recovery_text(status) != Some("pending"@),
    },
{
    if planner::same_attempt("pending", status) { Some("discarded") } else { None }
}

pub struct RecoverySummary {
    changed: bool,
    warning_effect: bool,
}

/// 只借用原报告的字段和产物，不复制 JSON 或产物列表。
pub struct RecoveryCacheObservation<'a, A> {
    pub status: Option<&'a str>,
    pub outcome: Option<&'a str>,
    pub artifacts: &'a [A],
}

pub open spec fn recovery_cache_changed<A>(cache: RecoveryCacheObservation<A>) -> bool {
    recovery_text(cache.status) == Some("discarded"@)
        || recovery_text(cache.status) == Some("source-times-refreshed"@)
}

pub open spec fn recovery_cache_warns<A>(cache: RecoveryCacheObservation<A>,
    model: spec_fn(A) -> Option<Seq<char>>) -> bool
{
    recovery_text(cache.outcome) == Some("failure"@)
        || exists|index: int| 0 <= index < cache.artifacts.len()
            && #[trigger] model(cache.artifacts@[index]) == Some("removed"@)
}

proof fn recovery_any_prefix_step<A>(items: Seq<A>, index: int, effect: spec_fn(A) -> bool)
    requires 0 <= index < items.len(),
    ensures (exists|prior: int| 0 <= prior < index + 1 && #[trigger] effect(items[prior]))
        == ((exists|prior: int| 0 <= prior < index && #[trigger] effect(items[prior]))
            || effect(items[index])),
{
    if effect(items[index]) {
        assert(exists|prior: int| 0 <= prior < index + 1 && #[trigger] effect(items[prior]));
    }
    if exists|prior: int| 0 <= prior < index && #[trigger] effect(items[prior]) {
        let prior = choose|prior: int| 0 <= prior < index && #[trigger] effect(items[prior]);
        assert(effect(items[prior]));
        assert(exists|prior: int| 0 <= prior < index + 1 && #[trigger] effect(items[prior]));
    }
    if exists|prior: int| 0 <= prior < index + 1 && #[trigger] effect(items[prior]) {
        let prior = choose|prior: int| 0 <= prior < index + 1 && #[trigger] effect(items[prior]);
        if prior < index {
            assert(exists|prior: int| 0 <= prior < index && #[trigger] effect(items[prior]));
        } else {
            assert(prior == index);
        }
    }
}

fn summarize_recovery_artifacts<'a, A, R>(
    summary: RecoverySummary, artifacts: &'a [A], read: &R,
    #[cfg(verus_keep_ghost)] Ghost(model): Ghost<spec_fn(A) -> Option<Seq<char>>>,
) -> (out: RecoverySummary)
    where R: Fn(&'a A) -> Option<&'a str>,
    requires forall|artifact: &'a A| read.requires((artifact,)),
        forall|artifact: &'a A, status: Option<&'a str>|
            read.ensures((artifact,), status) ==> recovery_text(status) == model(*artifact),
    ensures out.changed() == summary.changed(),
        out.warning_effect() == (summary.warning_effect()
            || exists|index: int| 0 <= index < artifacts.len()
                && #[trigger] model(artifacts@[index]) == Some("removed"@)),
{
    let mut out = summary;
    let mut index = 0usize;
    while index < artifacts.len()
        invariant index <= artifacts.len(), out.changed() == summary.changed(),
            out.warning_effect() == (summary.warning_effect()
                || exists|prior: int| 0 <= prior < index
                    && #[trigger] model(artifacts@[prior]) == Some("removed"@)),
            forall|artifact: &'a A| read.requires((artifact,)),
            forall|artifact: &'a A, status: Option<&'a str>|
                read.ensures((artifact,), status) ==> recovery_text(status) == model(*artifact),
        decreases artifacts.len() - index,
    {
        let status = read(&artifacts[index]);
        out = out.observe_artifact(status);
        index += 1;
    }
    out
}

/// 完整遍历缓存及其原产物；摘要恰好反映全部条目的恢复效果。
pub fn summarize_recovery<'a, A, R>(
    caches: &[RecoveryCacheObservation<'a, A>], read: R,
    #[cfg(verus_keep_ghost)] Ghost(model): Ghost<spec_fn(A) -> Option<Seq<char>>>,
) -> (out: RecoverySummary)
    where R: Fn(&'a A) -> Option<&'a str>,
    requires forall|artifact: &'a A| read.requires((artifact,)),
        forall|artifact: &'a A, status: Option<&'a str>|
            read.ensures((artifact,), status) ==> recovery_text(status) == model(*artifact),
    ensures out.changed() == (exists|index: int| 0 <= index < caches.len()
            && #[trigger] recovery_cache_changed(caches@[index])),
        out.warning_effect() == (exists|index: int| 0 <= index < caches.len()
            && #[trigger] recovery_cache_warns(caches@[index], model)),
{
    let mut summary = RecoverySummary::new();
    let mut index = 0usize;
    while index < caches.len()
        invariant index <= caches.len(),
            summary.changed() == (exists|prior: int| 0 <= prior < index
                && #[trigger] recovery_cache_changed(caches@[prior])),
            summary.warning_effect() == (exists|prior: int| 0 <= prior < index
                && #[trigger] recovery_cache_warns(caches@[prior], model)),
            forall|artifact: &'a A| read.requires((artifact,)),
            forall|artifact: &'a A, status: Option<&'a str>|
                read.ensures((artifact,), status) ==> recovery_text(status) == model(*artifact),
        decreases caches.len() - index,
    {
        let cache = &caches[index];
        summary = summarize_recovery_artifacts(
            summary.observe_cache(cache.status, cache.outcome), cache.artifacts, &read,
            Ghost(model),
        );
        proof {
            recovery_any_prefix_step(caches@, index as int,
                |cache: RecoveryCacheObservation<A>| recovery_cache_changed(cache));
            recovery_any_prefix_step(caches@, index as int,
                |cache: RecoveryCacheObservation<A>| recovery_cache_warns(cache, model));
        }
        index += 1;
    }
    summary
}

impl RecoverySummary {
    pub closed spec fn changed(&self) -> bool { self.changed }
    pub closed spec fn warning_effect(&self) -> bool { self.warning_effect }

    pub fn new() -> (out: Self)
        ensures !out.changed(), !out.warning_effect(),
    { Self { changed: false, warning_effect: false } }

    pub fn observe_cache(self, status: Option<&str>, outcome: Option<&str>) -> (out: Self)
        ensures out.changed() == (self.changed()
            || recovery_text(status) == Some("discarded"@)
            || recovery_text(status) == Some("source-times-refreshed"@)),
            out.warning_effect() == (self.warning_effect()
                || recovery_text(outcome) == Some("failure"@)),
    {
        Self {
            changed: self.changed || planner::same_attempt("discarded", status)
                || planner::same_attempt("source-times-refreshed", status),
            warning_effect: self.warning_effect || planner::same_attempt("failure", outcome),
        }
    }

    pub fn observe_artifact(self, status: Option<&str>) -> (out: Self)
        ensures out.changed() == self.changed(),
            out.warning_effect() == (self.warning_effect()
                || recovery_text(status) == Some("removed"@)),
    {
        Self {
            changed: self.changed,
            warning_effect: self.warning_effect || planner::same_attempt("removed", status),
        }
    }

    pub fn status(&self, failed: bool) -> (out: &'static str)
        ensures out@ == if failed { "failed"@ }
            else if self.changed() { "recovered"@ } else { "not-required"@ },
    {
        if failed { "failed" } else if self.changed { "recovered" } else { "not-required" }
    }

    pub fn warning(&self, status: Option<&str>) -> (out: bool)
        ensures out == (recovery_text(status) == Some("recovered"@) && self.warning_effect()),
    { planner::same_attempt("recovered", status) && self.warning_effect }
}

pub open spec fn recovery_failure(result: Result<(), String>) -> Option<String> {
    match result { Ok(()) => None, Err(error) => Some(error) }
}

fn original_recovery_failure(result: Result<(), String>) -> (out: Option<String>)
    ensures out == recovery_failure(result),
{
    match result { Ok(()) => None, Err(error) => Some(error) }
}

pub open spec fn published_recovery_result<S, O, T, E, W, J, G>(
    state: S, recovery: Result<(), String>, original: O, warn: bool,
    warning: W, encode: J, recovery_error: &G, out: (S, Result<T, E>),
) -> bool
    where W: FnOnce(S, &O) -> S,
        J: FnOnce(S, O) -> (S, Result<T, E>), G: Fn(String, O) -> E,
{
    match recovery {
        Err(error) => out.0 == state && exists|rendered: E|
            #[trigger] recovery_error.ensures((error, original), rendered)
                && out.1 == Err::<T, E>(rendered),
        Ok(()) => exists|ready: S|
            (if warn { warning.ensures((state, &original), ready) }
                else { ready == state })
            && #[trigger] encode.ensures((ready, original), out),
    }
}

fn finish_published_recovery<S, O, T, E, W, J, G>(
    state: S, recovery: Result<(), String>, original: O, warn: bool,
    warning: W, encode: J, recovery_error: &G,
) -> (out: (S, Result<T, E>))
    where W: FnOnce(S, &O) -> S,
        J: FnOnce(S, O) -> (S, Result<T, E>), G: Fn(String, O) -> E,
    requires forall|state: S, original: &O| warning.requires((state, original)),
        forall|state: S, original: O| encode.requires((state, original)),
        forall|error: String, original: O| recovery_error.requires((error, original)),
    ensures published_recovery_result(state, recovery, original, warn,
        warning, encode, recovery_error, out),
{
    match recovery {
        Err(error) => {
            let rendered = recovery_error(error, original);
            assert(recovery_error.ensures((error, original), rendered));
            let out = (state, Err(rendered));
            assert(published_recovery_result(state, recovery, original, warn,
                warning, encode, recovery_error, out));
            out
        },
        Ok(()) => {
            let ready = if warn { warning(state, &original) } else { state };
            assert(if warn { warning.ensures((state, &original), ready) } else { ready == state });
            let out = encode(ready, original);
            assert(encode.ensures((ready, original), out));
            assert(published_recovery_result(state, recovery, original, warn,
                warning, encode, recovery_error, out));
            out
        },
    }
}

pub open spec fn recovery_finalization_result<S, O, T, E, R, P, W, J, F, G>(
    source: S, recovery: Result<(), String>, warn: bool,
    retain: R, pointer: P, warning: W, encode: J,
    publication_error: &F, recovery_error: &G, out: (S, Result<T, E>),
) -> bool
    where R: FnOnce(S) -> (S, Result<O, String>),
        P: FnOnce(S, &O) -> (S, Result<(), String>), W: FnOnce(S, &O) -> S,
        J: FnOnce(S, O) -> (S, Result<T, E>),
        F: Fn(Option<String>, String) -> E, G: Fn(String, O) -> E,
{
    exists|retained: S, observed: Result<O, String>|
        #[trigger] retain.ensures((source,), (retained, observed))
        && match observed {
            Err(error) => out.0 == retained && exists|rendered: E|
                #[trigger] publication_error.ensures((recovery_failure(recovery), error), rendered)
                    && out.1 == Err::<T, E>(rendered),
            Ok(original) => exists|published: S, publication: Result<(), String>|
                #[trigger] pointer.ensures((retained, &original), (published, publication))
                && match publication {
                    Err(error) => out.0 == published && exists|rendered: E|
                        #[trigger] publication_error.ensures((recovery_failure(recovery), error), rendered)
                            && out.1 == Err::<T, E>(rendered),
                    Ok(()) => published_recovery_result(published, recovery, original,
                        warn, warning, encode, recovery_error, out),
                },
        }
}

/// 保留原报告后发布指针；原恢复失败不报警或编码成功结果。
pub fn finalize_recovery<S, O, T, E, R, P, W, J, F, G>(
    source: S, recovery: Result<(), String>, warn: bool,
    retain: R, pointer: P, warning: W, encode: J,
    publication_error: &F, recovery_error: &G,
) -> (out: (S, Result<T, E>))
    where R: FnOnce(S) -> (S, Result<O, String>),
        P: FnOnce(S, &O) -> (S, Result<(), String>), W: FnOnce(S, &O) -> S,
        J: FnOnce(S, O) -> (S, Result<T, E>),
        F: Fn(Option<String>, String) -> E, G: Fn(String, O) -> E,
    requires retain.requires((source,)),
        forall|state: S, original: &O| pointer.requires((state, original)),
        forall|state: S, original: &O| warning.requires((state, original)),
        forall|state: S, original: O| encode.requires((state, original)),
        forall|recovery: Option<String>, error: String| publication_error.requires((recovery, error)),
        forall|error: String, original: O| recovery_error.requires((error, original)),
    ensures recovery_finalization_result(source, recovery, warn, retain, pointer,
        warning, encode, publication_error, recovery_error, out),
{
    let (state, retained_result) = retain(source);
    let ghost retained = state;
    let ghost observed = retained_result;
    assert(retain.ensures((source,), (retained, observed)));
    let original = match retained_result {
        Err(error) => {
            let failed = original_recovery_failure(recovery);
            return (state, Err(publication_error(failed, error)));
        },
        Ok(original) => original,
    };
    let (state, publication) = pointer(state, &original);
    let ghost published = state;
    let ghost published_result = publication;
    assert(pointer.ensures((retained, &original), (published, published_result)));
    match publication {
        Err(error) => {
            let failed = original_recovery_failure(recovery);
            (state, Err(publication_error(failed, error)))
        },
        Ok(()) => finish_published_recovery(state, recovery, original,
            warn, warning, encode, recovery_error),
    }
}

pub struct RecoveryReplay<'a> {
    pub schema: Option<&'a str>,
    pub plan_fields: Option<usize>,
    pub plan_path: Option<&'a str>,
    pub plan_sha256: Option<&'a str>,
    pub steps_sha256: Option<&'a str>,
    pub status: Option<&'a str>,
}

pub open spec fn recovery_replay_bound(report: &RecoveryReplay, path: &str,
    plan_hash: &str, steps_hash: &str) -> bool
{
    recovery_text(report.schema) == Some("chrono-cache-recovery/v1"@)
        && report.plan_fields == Some(2usize)
        && recovery_text(report.plan_path) == Some(path@)
        && recovery_text(report.plan_sha256) == Some(plan_hash@)
        && recovery_text(report.steps_sha256) == Some(steps_hash@)
}

pub open spec fn recovery_replay_completed(report: &RecoveryReplay) -> bool {
    recovery_text(report.status) == Some("recovered"@)
        || recovery_text(report.status) == Some("not-required"@)
}

/// 先核完整输入绑定，再核原终态；错误保留原件地址全文。
pub fn admit_recovery_replay<'a>(report: &RecoveryReplay, path: &str,
    plan_hash: &str, steps_hash: &str, original_path: &'a str)
    -> (out: Result<(), [&'a str; 2]>)
    ensures match out {
        Ok(()) => recovery_replay_bound(report, path, plan_hash, steps_hash)
            && recovery_replay_completed(report),
        Err(error) => if !recovery_replay_bound(report, path, plan_hash, steps_hash) {
            error[0]@ == "E_CACHE_RECOVERY: previous recovery belongs to different inputs"@
                && error[1]@ == ""@
        } else {
            !recovery_replay_completed(report)
                && error[0]@ == "E_CACHE_RECOVERY: previous attempt failed; original "@
                && error[1]@ == original_path@
        },
    },
{
    if !planner::same_attempt("chrono-cache-recovery/v1", report.schema)
        || report.plan_fields != Some(2usize)
        || !planner::same_attempt(path, report.plan_path)
        || !planner::same_attempt(plan_hash, report.plan_sha256)
        || !planner::same_attempt(steps_hash, report.steps_sha256) {
        return Err(["E_CACHE_RECOVERY: previous recovery belongs to different inputs", ""]);
    }
    if !planner::same_attempt("recovered", report.status)
        && !planner::same_attempt("not-required", report.status) {
        return Err(["E_CACHE_RECOVERY: previous attempt failed; original ", original_path]);
    }
    Ok(())
}

}

#[cfg(not(verus_keep_ghost))]
use chrono_harness::{decode, no_symlink_parents, prepared, sha256};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{collections::BTreeMap, fs, io::Write, path::Path};

#[cfg(not(verus_keep_ghost))]
fn restore_status(cache: &Value, restore: &Value) -> &'static str {
    let prefixes: Vec<_> = cache["restore_keys"]
        .as_array()
        .into_iter()
        .flatten()
        .map(Value::as_str)
        .collect();
    super::verified_planner::restore_status(
        restore["outcome"].as_str(),
        cache["key"].as_str(),
        restore["outputs"]["cache-matched-key"].as_str(),
        &prefixes,
    )
}

#[cfg(not(verus_keep_ghost))]
fn publish_once(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    let destination = chrono_harness::prepare_publication(root, path)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|e| format!("E_CACHE_RECOVERY: {path}: {e}"))
}

#[cfg(not(verus_keep_ghost))]
fn retained(root: &Path, plan_path: &str) -> Result<(prepared::Original, Value), String> {
    let directory = plan_path
        .rsplit_once('/')
        .ok_or("E_CACHE_RECOVERY: plan directory")?
        .0;
    let pointer = fs::read(no_symlink_parents(
        root,
        &format!("{directory}/recovery.json"),
    )?)
    .map_err(|e| format!("E_CACHE_RECOVERY: missing recovery result: {e}"))?;
    let original: prepared::Original = decode(&pointer)?;
    let raw = prepared::read_original(root, &original, None)?;
    Ok((original, decode(&raw)?))
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    let options = parse_recovery_options(args)?;
    let root = fs::canonicalize(options.required(&RecoveryOption::HostRoot)?)
        .map_err(|e| e.to_string())?;
    let _activity = chrono_harness::retained_artifacts::retention::read_scope(&root)?;
    let config_path = options.required(&RecoveryOption::Config)?;
    let plan_path = options.required(&RecoveryOption::Plan)?;
    admit_recovery_paths(config_path, plan_path)?;
    let plan_raw = fs::read(no_symlink_parents(&root, plan_path)?).map_err(|e| e.to_string())?;
    let plan: Value = decode(&plan_raw)?;
    let steps_raw = std::env::var(options.required(&RecoveryOption::StepsEnvironment)?)
        .map_err(|e| e.to_string())?;
    let directory = plan_path
        .rsplit_once('/')
        .ok_or("E_CACHE_RECOVERY: plan directory")?
        .0
        .to_owned()
        + "/";
    let output = format!("{directory}recovery.json");
    let intent_path = format!("{directory}recovery-intent.json");
    let plan_hash = sha256(&plan_raw);
    let binding = json!({"path":plan_path,"sha256":&plan_hash});
    let steps_hash = sha256(steps_raw.as_bytes());
    let output_file = no_symlink_parents(&root, &output)?;
    let previous = select_recovery_entry(
        (),
        std::io::ErrorKind::NotFound as u32,
        &intent_path,
        |()| {
            let observation = fs::symlink_metadata(&output_file)
                .map(|_| ())
                .map_err(|error| (error.kind() as u32, error.to_string()));
            ((), Ok(observation))
        },
        |()| {
            let observation = no_symlink_parents(&root, &intent_path).map(|path| {
                fs::symlink_metadata(path)
                    .map(|_| ())
                    .map_err(|error| (error.kind() as u32, error.to_string()))
            });
            ((), observation)
        },
        |()| {
            let result = (|| {
                let (original, report) = retained(&root, plan_path)?;
                admit_recovery_replay(
                    &RecoveryReplay {
                        schema: report["schema"].as_str(),
                        plan_fields: report["plan"].as_object().map(serde_json::Map::len),
                        plan_path: report["plan"]["path"].as_str(),
                        plan_sha256: report["plan"]["sha256"].as_str(),
                        steps_sha256: report["native_steps"]["sha256"].as_str(),
                        status: report["status"].as_str(),
                    },
                    plan_path,
                    &plan_hash,
                    &steps_hash,
                    &original.path,
                )
                .map_err(|parts| parts.concat())?;
                // This is the previous operation's result, never a second deletion of
                // outputs that a later producer may already have rebuilt.
                Ok(serde_json::to_string(
                    &json!({"report":original,"observation":report,"replayed":true}),
                )
                .map_err(|e| e.to_string())?
                    + "\n")
            })();
            ((), result)
        },
    )
    .1
    .map_err(|(prefix, original, suffix)| [prefix, original.as_str(), suffix].concat())?;
    if let Some(previous) = previous {
        return Ok(previous);
    }
    let steps_original =
        super::retain_original(&root, &directory, "restore-steps", steps_raw.as_bytes())?;
    let mut report = json!({"schema":"chrono-cache-recovery/v1","plan":binding,"native_steps":steps_original,
        "status":RecoverySummary::new().status(false),"caches":{},"error":null,
        "limits":["isolated provider restore phase, before producers", "registered native observations only; no archive integrity or build verdict", "missing matched keys do not distinguish miss from backend or extraction failure", "linked worktree recovery unsupported"]});
    let result = (|| -> Result<(), String> {
        let config: super::Config =
            decode(&fs::read(no_symlink_parents(&root, config_path)?).map_err(|e| e.to_string())?)?;
        if !config.recover_failed_restores {
            return Err("E_CACHE_RECOVERY: recovery is not registered".into());
        }
        let consumer = plan["consumer"]
            .as_str()
            .ok_or("E_CACHE_RECOVERY: consumer missing")?;
        let expected = super::prepare_with_inputs(&root, &config, consumer, Some(&plan["inputs"]))?;
        // Recovery may use an installed binary after a bootstrap-produced plan.
        // Revalidate registration, host and selected observations while retaining
        // the original planner identity. Seal checks the current executable.
        let mut expected_binding = expected;
        expected_binding["planner"] = plan["planner"].clone();
        if expected_binding != plan {
            return Err(
                "E_CACHE_RECOVERY: original plan differs from current registration or host".into(),
            );
        }
        super::attempt::validate(&root, &plan, false)?;
        let steps: Value = decode(steps_raw.as_bytes())?;
        if !steps.is_object() {
            return Err("E_CACHE_RECOVERY: native step observations missing".into());
        }
        let caches: BTreeMap<_, _> = plan["caches"]
            .as_object()
            .ok_or("E_CACHE_RECOVERY: caches missing")?
            .iter()
            .collect();
        let mut paths = vec![];
        let mut refreshes = vec![];
        for (index, (id, cache)) in caches.into_iter().enumerate() {
            let restore = &steps[format!("cache_{index}_restore")];
            let reason = restore_status(cache, restore);
            let outputs = &restore["outputs"];
            let outputs_is_null_or_object = outputs.is_null() || outputs.is_object();
            let matched_key_is_null_or_string = outputs
                .get("cache-matched-key")
                .is_none_or(|key| key.is_null() || key.is_string());
            let outcome = restore["outcome"].as_str();
            let base_action = super::verified_planner::recovery_action(
                outcome,
                reason,
                config.recover_unconfirmed_restores,
                outputs_is_null_or_object,
                matched_key_is_null_or_string,
                None,
                None,
                false,
            );
            let base_action =
                base_action.map_err(|error| recovery_decision_error(&error, id).concat())?;
            let content_method = cache["content_verification"].as_str();
            let mut verification = None;
            if base_action.requires_content_verification(content_method, reason) {
                let verified = super::content::verify(
                    &root,
                    &directory,
                    id,
                    cache,
                    restore["outputs"]["cache-matched-key"].as_str().unwrap(),
                )?;
                verification = Some(verified);
            }
            let content_status = verification
                .as_ref()
                .and_then(|value| value["status"].as_str());
            let action = super::verified_planner::recovery_action(
                outcome,
                reason,
                config.recover_unconfirmed_restores,
                outputs_is_null_or_object,
                matched_key_is_null_or_string,
                content_method,
                content_status,
                config.caches[id].refresh_sources,
            );
            let action = action.map_err(|error| recovery_decision_error(&error, id).concat())?;
            let discard = action.discards_outputs();
            report["caches"][id] = json!({"restore":restore,"reason":reason,"status":action.initial_status(),"artifacts":[]});
            if let Some(verification) = verification {
                report["caches"][id]["verification"] = verification;
            }
            if !discard {
                if action.refreshes_sources() {
                    let refresh = super::freshness::prepare(
                        &root,
                        &config.caches[id],
                        cache,
                        &plan["inputs"],
                    )?;
                    report["caches"][id]["source_freshness"] =
                        json!({"method":super::freshness::METHOD,"status":"pending"});
                    refreshes.push((id.clone(), refresh));
                }
                continue;
            }
            let artifacts: BTreeMap<_, _> = cache["artifacts"]
                .as_object()
                .ok_or("E_CACHE_RECOVERY: artifacts missing")?
                .iter()
                .collect();
            for (_, artifact) in artifacts {
                let path = artifact["path"]
                    .as_str()
                    .ok_or("E_CACHE_RECOVERY: artifact path")?;
                let target = super::path(&root, path)?;
                if !target.starts_with(&root) {
                    return Err(
                        "E_CACHE_RECOVERY: only registered host artifacts may be discarded".into(),
                    );
                }
                let position = report["caches"][id]["artifacts"].as_array().unwrap().len();
                report["caches"][id]["artifacts"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"path":path,"status":"pending"}));
                paths.push((id.clone(), position, path.to_owned()));
            }
        }
        // Validate the complete selected set before the first removal, and
        // publish the original plan/restore binding before touching any output.
        let (updated, removed) = run_recovery_removals(
            std::mem::take(&mut report),
            &paths,
            |report| {
                let published = serde_json::to_vec(&report)
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| publish_once(&root, &intent_path, &bytes));
                (report, published)
            },
            &|report, index| {
                let removed = super::path(&root, &paths[index].2).map(|target| {
                    remove_recovery_output(
                        target,
                        std::io::ErrorKind::NotFound as u32,
                        |target| {
                            let result = fs::symlink_metadata(&target)
                                .map(|metadata| (metadata.is_dir(), metadata.is_file()))
                                .map_err(|error| (error.kind() as u32, error.to_string()));
                            (target, result)
                        },
                        |target| {
                            let result =
                                fs::remove_dir_all(&target).map_err(|error| error.to_string());
                            (target, result)
                        },
                        |target| {
                            let result =
                                fs::remove_file(&target).map_err(|error| error.to_string());
                            (target, result)
                        },
                    )
                    .1
                });
                (report, removed.and_then(std::convert::identity))
            },
            &|mut report, index, status| {
                let (id, position, _) = &paths[index];
                report["caches"][id]["artifacts"][*position]["status"] = json!(status);
                report
            },
            &|mut report, index, error| {
                let (id, position, _) = &paths[index];
                let row = &mut report["caches"][id]["artifacts"][*position];
                row["status"] = json!("failed");
                row["error"] = json!(error);
                report
            },
        );
        report = updated;
        removed.map_err(|(prefix, path, separator, error)| {
            [prefix, path.as_str(), separator, error.as_str()].concat()
        })?;
        for cache in report["caches"].as_object_mut().unwrap().values_mut() {
            if let Some(status) = complete_recovery_cache(cache["status"].as_str()) {
                cache["status"] = json!(status);
            }
        }
        for (id, refresh) in refreshes {
            super::freshness::apply(
                &root,
                refresh,
                &mut report["caches"][&id]["source_freshness"],
            )?;
            report["caches"][&id]["status"] = json!("source-times-refreshed");
        }
        Ok(())
    })();
    let caches: Vec<_> = report["caches"]
        .as_object()
        .unwrap()
        .values()
        .map(|cache| RecoveryCacheObservation {
            status: cache["status"].as_str(),
            outcome: cache["restore"]["outcome"].as_str(),
            artifacts: cache["artifacts"].as_array().unwrap().as_slice(),
        })
        .collect();
    let summary = summarize_recovery(&caches, |artifact| artifact["status"].as_str());
    report["status"] = json!(summary.status(result.is_err()));
    if let Err(error) = &result {
        report["error"] = json!(error);
    }
    let raw = serde_json::to_vec(&report).map_err(|e| e.to_string())?;
    let warn = summary.warning(report["status"].as_str());
    finalize_recovery(
        report,
        result,
        warn,
        |report| {
            let original = super::retain_original(&root, &directory, "cache-recovery", &raw);
            (report, original)
        },
        |report, original| {
            let published = serde_json::to_vec(original)
                .map_err(|error| error.to_string())
                .and_then(|bytes| publish_once(&root, &output, &bytes));
            (report, published)
        },
        |report, original| {
            eprintln!(
                "W_CACHE_RESTORE_RECOVERED: unusable restore outputs cleared before registered build; original {}",
                original.path
            );
            report
        },
        |report, original| {
            let encoded = serde_json::to_string(
                &json!({"report":original,"observation":report,"replayed":false}),
            )
            .map(|text| text + "\n")
            .map_err(|error| error.to_string());
            (report, encoded)
        },
        &|recovery, publication| {
            format!("E_CACHE_RECOVERY: recovery {recovery:?}; publication {publication}")
        },
        &|error, original: prepared::Original| format!("{error}; original {}", original.path),
    )
    .1
}
