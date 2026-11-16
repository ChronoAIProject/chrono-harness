//! Cache planning owns its original probe attempts; retention owns custody and disposal.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub open spec fn cache_record_publication_result<S, B, P, Q, F, E, W, L, R, D, O, Y>(
    initial: S, encode: E, write: W, link: L, remove: R, parent: D, open: O, sync: Y,
    out: (S, Result<(), String>)) -> bool
    where E: FnOnce(S) -> (S, Result<B, String>),
        W: FnOnce(S, &B) -> (S, Result<P, String>),
        L: FnOnce(S, &P) -> (S, Result<(), String>),
        R: FnOnce(S, &P) -> (S, Result<(), String>),
        D: FnOnce(S) -> (S, Option<Q>),
        O: FnOnce(S, Q) -> (S, Result<F, String>),
        Y: FnOnce(S, F) -> (S, Result<(), String>),
{
    exists|encoded: S, encoding: Result<B, String>|
        #[trigger] encode.ensures((initial,), (encoded, encoding))
        && match encoding {
            Err(error) => out == (encoded, Err::<(), String>(error)),
            Ok(bytes) => exists|written: S, writing: Result<P, String>|
                #[trigger] write.ensures((encoded, &bytes), (written, writing))
                && match writing {
                    Err(error) => out == (written, Err::<(), String>(error)),
                    Ok(temporary) => exists|linked: S, linking: Result<(), String>|
                        #[trigger] link.ensures((written, &temporary), (linked, linking))
                        && match linking {
                            Err(error) => out == (linked, Err::<(), String>(error)),
                            Ok(()) => exists|removed: S, removal: Result<(), String>|
                                #[trigger] remove.ensures((linked, &temporary), (removed, removal))
                                && match removal {
                                    Err(error) => out == (removed, Err::<(), String>(error)),
                                    Ok(()) => exists|parented: S, directory: Option<Q>|
                                        #[trigger] parent.ensures((removed,), (parented, directory))
                                        && match directory {
                                            None => out.0 == parented && match out.1 {
                                                Err(error) => error@ == "E_CACHE_ATTEMPT: record parent"@,
                                                _ => false,
                                            },
                                            Some(path) => exists|opened: S, opening: Result<F, String>|
                                                #[trigger] open.ensures((parented, path), (opened, opening))
                                                && match opening {
                                                    Err(error) => out == (opened, Err::<(), String>(error)),
                                                    Ok(file) => sync.ensures((opened, file), out),
                                                },
                                        },
                                },
                        },
                },
        }
}

/// 编码、落盘、链接、移除临时文件及父目录同步保留原顺序和首错。
pub fn record_cache_attempt<S, B, P, Q, F, E, W, L, R, D, O, Y>(state: S,
    encode: E, write: W, link: L, remove: R, parent: D, open: O, sync: Y,
) -> (out: (S, Result<(), String>))
    where E: FnOnce(S) -> (S, Result<B, String>),
        W: FnOnce(S, &B) -> (S, Result<P, String>),
        L: FnOnce(S, &P) -> (S, Result<(), String>),
        R: FnOnce(S, &P) -> (S, Result<(), String>),
        D: FnOnce(S) -> (S, Option<Q>),
        O: FnOnce(S, Q) -> (S, Result<F, String>),
        Y: FnOnce(S, F) -> (S, Result<(), String>),
    requires forall|state: S| encode.requires((state,)),
        forall|state: S, bytes: &B| write.requires((state, bytes)),
        forall|state: S, temporary: &P| link.requires((state, temporary)),
        forall|state: S, temporary: &P| remove.requires((state, temporary)),
        forall|state: S| parent.requires((state,)),
        forall|state: S, path: Q| open.requires((state, path)),
        forall|state: S, file: F| sync.requires((state, file)),
    ensures cache_record_publication_result(state, encode, write, link, remove, parent, open, sync, out),
{
    let ghost initial = state;
    let (state, encoding) = encode(state);
    let ghost encoded = state;
    let ghost encoding_result = encoding;
    assert(encode.ensures((initial,), (encoded, encoding_result)));
    let bytes = match encoding { Err(error) => return (state, Err(error)), Ok(bytes) => bytes, };
    let (state, writing) = write(state, &bytes);
    let ghost written = state;
    let ghost writing_result = writing;
    assert(write.ensures((encoded, &bytes), (written, writing_result)));
    let temporary = match writing { Err(error) => return (state, Err(error)), Ok(path) => path, };
    let (state, linking) = link(state, &temporary);
    let ghost linked = state;
    let ghost linking_result = linking;
    assert(link.ensures((written, &temporary), (linked, linking_result)));
    match linking { Err(error) => return (state, Err(error)), Ok(()) => (), }
    let (state, removal) = remove(state, &temporary);
    let ghost removed = state;
    let ghost removal_result = removal;
    assert(remove.ensures((linked, &temporary), (removed, removal_result)));
    match removal { Err(error) => return (state, Err(error)), Ok(()) => (), }
    let (state, directory) = parent(state);
    let ghost parented = state;
    let ghost directory_result = directory;
    assert(parent.ensures((removed,), (parented, directory_result)));
    let path = match directory {
        None => return (state, Err("E_CACHE_ATTEMPT: record parent".to_string())),
        Some(path) => path,
    };
    let (state, opening) = open(state, path);
    let ghost opened = state;
    let ghost opening_result = opening;
    assert(open.ensures((parented, path), (opened, opening_result)));
    let file = match opening { Err(error) => return (state, Err(error)), Ok(file) => file, };
    let out = sync(state, file);
    assert(sync.ensures((opened, file), out));
    out
}

pub enum CacheAttemptFailure {
    Original(String),
    Recording(String, Option<String>),
    Sealing(String, Option<String>),
}

pub enum CacheAttemptStage<S, U> {
    BeforeSeal(S),
    AfterSeal(U),
}

pub open spec fn attempt_original_error<T>(result: Result<T, String>) -> Option<String> {
    match result { Ok(_) => None, Err(error) => Some(error), }
}

pub open spec fn attempt_result_forwarded<T>(before: Result<T, String>,
    after: Result<T, CacheAttemptFailure>) -> bool
{
    match before {
        Ok(value) => after == Ok::<T, CacheAttemptFailure>(value),
        Err(error) => after == Err::<T, CacheAttemptFailure>(CacheAttemptFailure::Original(error)),
    }
}

fn forward_attempt_result<T>(result: Result<T, String>) -> (out: Result<T, CacheAttemptFailure>)
    ensures attempt_result_forwarded(result, out),
{
    match result { Ok(value) => Ok(value), Err(error) => Err(CacheAttemptFailure::Original(error)), }
}

pub open spec fn cache_attempt_finalization_result<S, U, T, H, R, D, C, L>(initial: S,
    terminal: bool, result: Result<T, String>, record: R, digest: D, seal: C, release: L,
    out: (CacheAttemptStage<S, U>, Result<T, CacheAttemptFailure>)) -> bool
    where R: FnOnce(S, &Result<T, String>) -> (S, Result<(), String>),
        D: FnOnce(S) -> (S, Result<H, String>),
        C: FnOnce(S, &H) -> (U, Result<(), String>),
        L: FnOnce(U) -> (U, Result<(), String>),
{
    exists|recorded: S, recording: Result<(), String>|
        #[trigger] record.ensures((initial, &result), (recorded, recording))
        && match recording {
            Err(error) => out == (CacheAttemptStage::<S, U>::BeforeSeal(recorded), Err::<T, CacheAttemptFailure>(
                CacheAttemptFailure::Recording(error, attempt_original_error(result)))),
            Ok(()) => if !terminal {
                out.0 == CacheAttemptStage::<S, U>::BeforeSeal(recorded) && attempt_result_forwarded(result, out.1)
            } else {
                exists|digested: S, hashing: Result<H, String>|
                    #[trigger] digest.ensures((recorded,), (digested, hashing))
                    && match hashing {
                        Err(error) => out == (CacheAttemptStage::<S, U>::BeforeSeal(digested), Err::<T, CacheAttemptFailure>(
                            CacheAttemptFailure::Original(error))),
                        Ok(hash) => exists|sealed: U, sealing: Result<(), String>|
                            #[trigger] seal.ensures((digested, &hash), (sealed, sealing))
                            && match sealing {
                                Err(error) => out == (CacheAttemptStage::<S, U>::AfterSeal(sealed), Err::<T, CacheAttemptFailure>(
                                    CacheAttemptFailure::Sealing(error, attempt_original_error(result)))),
                                Ok(()) => match result {
                                    Ok(value) => out == (CacheAttemptStage::<S, U>::AfterSeal(sealed), Ok::<T, CacheAttemptFailure>(value)),
                                    Err(original) => exists|released: U, releasing: Result<(), String>|
                                        #[trigger] release.ensures((sealed,), (released, releasing))
                                        && out == (CacheAttemptStage::<S, U>::AfterSeal(released), Err::<T, CacheAttemptFailure>(
                                            CacheAttemptFailure::Original(match releasing {
                                                Err(error) => error, Ok(()) => original,
                                            }))),
                                },
                            },
                    }
            },
        }
}

/// 先记录结果；未知任务保留恢复能力，终态封存后只释放失败计划的引用。
pub fn finalize_cache_attempt<S, U, T, H, R, D, C, L>(state: S, terminal: bool,
    result: Result<T, String>, record: R, digest: D, seal: C, release: L,
) -> (out: (CacheAttemptStage<S, U>, Result<T, CacheAttemptFailure>))
    where R: FnOnce(S, &Result<T, String>) -> (S, Result<(), String>),
        D: FnOnce(S) -> (S, Result<H, String>),
        C: FnOnce(S, &H) -> (U, Result<(), String>),
        L: FnOnce(U) -> (U, Result<(), String>),
    requires forall|state: S, result: &Result<T, String>| record.requires((state, result)),
        forall|state: S| digest.requires((state,)),
        forall|state: S, hash: &H| seal.requires((state, hash)),
        forall|state: U| release.requires((state,)),
    ensures cache_attempt_finalization_result(state, terminal, result, record, digest, seal, release, out),
{
    let ghost initial = state;
    let (state, recording) = record(state, &result);
    let ghost recorded = state;
    let ghost recording_result = recording;
    assert(record.ensures((initial, &result), (recorded, recording_result)));
    match recording {
        Err(error) => {
            let original = match result { Ok(_) => None, Err(original) => Some(original), };
            return (CacheAttemptStage::BeforeSeal(state), Err(CacheAttemptFailure::Recording(error, original)));
        },
        Ok(()) => (),
    }
    if !terminal { return (CacheAttemptStage::BeforeSeal(state), forward_attempt_result(result)); }
    let (state, hashing) = digest(state);
    let ghost digested = state;
    let ghost hashing_result = hashing;
    assert(digest.ensures((recorded,), (digested, hashing_result)));
    let hash = match hashing {
        Err(error) => return (CacheAttemptStage::BeforeSeal(state), Err(CacheAttemptFailure::Original(error))),
        Ok(hash) => hash,
    };
    let (state, sealing) = seal(state, &hash);
    let ghost sealed = state;
    let ghost sealing_result = sealing;
    assert(seal.ensures((digested, &hash), (sealed, sealing_result)));
    match sealing {
        Err(error) => {
            let original = match result { Ok(_) => None, Err(original) => Some(original), };
            return (CacheAttemptStage::AfterSeal(state), Err(CacheAttemptFailure::Sealing(error, original)));
        },
        Ok(()) => (),
    }
    match result {
        Ok(value) => (CacheAttemptStage::AfterSeal(state), Ok(value)),
        Err(original) => {
            let (state, releasing) = release(state);
            assert(release.ensures((sealed,), (state, releasing)));
            (CacheAttemptStage::AfterSeal(state), Err(CacheAttemptFailure::Original(match releasing {
                Err(error) => error, Ok(()) => original,
            })))
        },
    }
}

pub open spec fn cache_attempt_failure_parts(relative: &str, failure: &CacheAttemptFailure,
    parts: [&str; 7]) -> bool
{
    match failure {
        CacheAttemptFailure::Original(error) => parts@[0]@ == error@
            && parts@[1]@ == ""@ && parts@[2]@ == ""@ && parts@[3]@ == ""@
            && parts@[4]@ == ""@ && parts@[5]@ == ""@ && parts@[6]@ == ""@,
        CacheAttemptFailure::Recording(publication, original) => match original {
            Some(error) => parts@[0]@ == error@ && parts@[1]@ == "; result publication: "@
                && parts@[2]@ == publication@ && parts@[3]@ == "; attempt "@
                && parts@[4]@ == relative@ && parts@[5]@ == ""@ && parts@[6]@ == ""@,
            None => parts@[0]@ == "E_CACHE_ATTEMPT: result publication: "@
                && parts@[1]@ == publication@ && parts@[2]@ == "; original "@
                && parts@[3]@ == relative@ && parts@[4]@ == ""@
                && parts@[5]@ == ""@ && parts@[6]@ == ""@,
        },
        CacheAttemptFailure::Sealing(publication, original) =>
            parts@[0]@ == match original { Some(error) => error@, None => ""@ }
            && parts@[1]@ == match original { Some(_) => "; "@, None => ""@ }
            && parts@[2]@ == "E_CACHE_ATTEMPT: terminal publication: "@
            && parts@[3]@ == publication@ && parts@[4]@ == "; original "@
            && parts@[5]@ == relative@ && parts@[6]@ == "/result.json"@,
    }
}

/// 同一错误类型选择完整诊断片段；适配器只按原顺序连接。
pub fn cache_attempt_failure_fragments<'a>(relative: &'a str, failure: &'a CacheAttemptFailure)
    -> (out: [&'a str; 7])
    ensures cache_attempt_failure_parts(relative, failure, out),
{
    match failure {
        CacheAttemptFailure::Original(error) => [error.as_str(), "", "", "", "", "", ""],
        CacheAttemptFailure::Recording(publication, original) => match original {
            Some(error) => [error.as_str(), "; result publication: ", publication.as_str(),
                "; attempt ", relative, "", ""],
            None => ["E_CACHE_ATTEMPT: result publication: ", publication.as_str(),
                "; original ", relative, "", "", ""],
        },
        CacheAttemptFailure::Sealing(publication, original) => {
            let original = match original { Some(error) => Some(error.as_str()), None => None, };
            cache_publication_error_fragments(relative, publication.as_str(), original)
        },
    }
}

pub open spec fn cache_result_replay<'a, T>(plan: Option<&'a T>, plan_is_null: bool,
    error: Option<&str>, out: Result<&'a T, String>) -> bool
{
    if plan.is_some() && !plan_is_null && error.is_none() {
        out == Ok::<&T, String>(plan.unwrap())
    } else {
        match out {
            Err(message) => if (plan.is_none() || plan_is_null) && error.is_some() {
                message@ == error.unwrap()@
            } else {
                message@ == "E_CACHE_ATTEMPT: invalid original result; no redispatch"@
            },
            _ => false,
        }
    }
}

/// 只有非 Null 的 JSON 计划与缺失错误的组合可以重放；原错误不改写。
pub fn replay_cache_result<'a, T>(plan: Option<&'a T>, plan_is_null: bool,
    error: Option<&str>) -> (out: Result<&'a T, String>)
    ensures cache_result_replay(plan, plan_is_null, error, out),
{
    let selected = if plan_is_null { None } else { plan };
    match (selected, error) {
        (Some(value), None) => Ok(value),
        (None, Some(error)) => Err(error.to_string()),
        _ => Err("E_CACHE_ATTEMPT: invalid original result; no redispatch".to_string()),
    }
}

/// 原错误是否存在决定前缀；固定片段保留完整错误与原件地址的顺序。
pub fn cache_publication_error_fragments<'a>(relative: &'a str,
    publication: &'a str, original: Option<&'a str>) -> (out: [&'a str; 7])
    ensures
        out@[0]@ == match original { Some(error) => error@, None => ""@ },
        out@[1]@ == match original { Some(_) => "; "@, None => ""@ },
        out@[2]@ == "E_CACHE_ATTEMPT: terminal publication: "@,
        out@[3]@ == publication@,
        out@[4]@ == "; original "@,
        out@[5]@ == relative@,
        out@[6]@ == "/result.json"@,
{
    let (previous, separator) = match original {
        Some(error) => (error, "; "), None => ("", ""),
    };
    [previous, separator, "E_CACHE_ATTEMPT: terminal publication: ",
        publication, "; original ", relative, "/result.json"]
}

/// Request digests and original addresses use complete text, including absence.
pub fn same_attempt(expected: &str, original: Option<&str>) -> (bound: bool)
    ensures bound == match original { Some(value) => expected@ == value@, None => false },
{
    match original { Some(value) => identifiers::equal_text(expected, value), None => false }
}

pub fn accept_original(bound: bool, terminal: bool, sealed: bool) -> (accepted: bool)
    ensures accepted == (bound && terminal && sealed),
{ bound && terminal && sealed }

pub fn release_custody(bound: bool, terminal: bool, sealed: bool, consumer_finished: bool) -> (release: bool)
    ensures release == (bound && terminal && sealed && consumer_finished),
{ accept_original(bound, terminal, sealed) && consumer_finished }

/// A result can be replayed only for the exact request, after confirmed work.
pub fn replay_attempt(bound: bool, terminal: bool) -> (allowed: bool)
    ensures allowed == (bound && terminal),
{ bound && terminal }

/// A process receipt alone does not establish joined, matched terminal work.
/// Engine errors do not erase an actually joined tree or establish a missing join.
pub fn confirmed_probe(launch_matches: bool, exit_code: i32, exit_signal: Option<i32>, owned_work_joined: bool) -> (terminal: bool)
    ensures terminal == (launch_matches && (exit_code >= 0 || exit_signal.is_some()) && owned_work_joined),
{ launch_matches && (exit_code >= 0 || exit_signal.is_some()) && owned_work_joined }

pub open spec fn attempt_inventory_result<S, I, P, C>(
    initial: S, adoption: Result<Option<I>, String>, probe: P, plan: C,
    out: (S, Result<I, String>),
) -> bool
    where P: FnOnce(S, &I) -> (S, bool),
        C: FnOnce(S, &I) -> (S, bool),
{
    match adoption {
        Err(error) => out == (initial, Err::<I, String>(error)),
        Ok(None) => out.0 == initial && match out.1 {
            Err(error) => error@ ==
                "E_CACHE_ATTEMPT: host must adopt retention/v2 for cache probe originals"@,
            _ => false,
        },
        Ok(Some(inventory)) => exists|probed: S, present: bool|
            #[trigger] probe.ensures((initial, &inventory), (probed, present))
            && if !present {
                out.0 == probed && match out.1 {
                    Err(error) => error@ ==
                        "E_CACHE_ATTEMPT: adopted policy requires cache-probe-attempt and cache-plan terminal_limits"@,
                    _ => false,
                }
            } else {
                exists|planned: S, present: bool|
                    #[trigger] plan.ensures((probed, &inventory), (planned, present))
                    && out.0 == planned && if present {
                        out.1 == Ok::<I, String>(inventory)
                    } else {
                        match out.1 {
                            Err(error) => error@ ==
                                "E_CACHE_ATTEMPT: adopted policy requires cache-probe-attempt and cache-plan terminal_limits"@,
                            _ => false,
                        }
                    }
            },
    }
}

/// 库存采用与两类终态额度按原短路顺序准入；成功交回同一库存。
pub fn admit_attempt_inventory<S, I, P, C>(
    state: S, adoption: Result<Option<I>, String>, probe: P, plan: C,
) -> (out: (S, Result<I, String>))
    where P: FnOnce(S, &I) -> (S, bool),
        C: FnOnce(S, &I) -> (S, bool),
    requires forall|state: S, inventory: &I| probe.requires((state, inventory)),
        forall|state: S, inventory: &I| plan.requires((state, inventory)),
    ensures attempt_inventory_result(state, adoption, probe, plan, out),
{
    let inventory = match adoption {
        Err(error) => return (state, Err(error)),
        Ok(None) => return (state, Err(
            "E_CACHE_ATTEMPT: host must adopt retention/v2 for cache probe originals".to_string())),
        Ok(Some(inventory)) => inventory,
    };
    let ghost initial = state;
    let (state, present) = probe(state, &inventory);
    let ghost probed = state;
    let ghost probe_present = present;
    assert(probe.ensures((initial, &inventory), (probed, probe_present)));
    if !present {
        return (state, Err(
            "E_CACHE_ATTEMPT: adopted policy requires cache-probe-attempt and cache-plan terminal_limits".to_string()));
    }
    let (state, present) = plan(state, &inventory);
    assert(plan.ensures((probed, &inventory), (state, present)));
    if !present {
        return (state, Err(
            "E_CACHE_ATTEMPT: adopted policy requires cache-probe-attempt and cache-plan terminal_limits".to_string()));
    }
    (state, Ok(inventory))
}

pub open spec fn cache_plan_release_result<S, I, L, A, C, O, U, R>(
    initial: S, has_originals: bool, adopt: A, confirm: C, owner: O,
    consume: U, release: R, out: (S, Result<(), String>),
) -> bool
    where A: FnOnce(S) -> (S, Result<I, String>),
        C: FnOnce(S, &I) -> (S, Result<bool, String>),
        O: FnOnce(S, &I) -> (S, Result<Option<String>, String>),
        U: FnOnce(S, &I, &String) -> (S, Result<L, String>),
        R: FnOnce(S, &I, &String, L) -> (S, Result<(), String>),
{
    if !has_originals {
        out == (initial, Ok::<(), String>(()))
    } else {
        exists|adopted: S, adoption: Result<I, String>|
            #[trigger] adopt.ensures((initial,), (adopted, adoption))
            && match adoption {
                Err(error) => out == (adopted, Err::<(), String>(error)),
                Ok(inventory) => exists|confirmed: S, confirmation: Result<bool, String>|
                    #[trigger] confirm.ensures((adopted, &inventory), (confirmed, confirmation))
                    && match confirmation {
                        Err(error) => out == (confirmed, Err::<(), String>(error)),
                        Ok(false) => out.0 == confirmed && match out.1 {
                            Err(error) => error@ ==
                                "E_CACHE_ATTEMPT: consumer plan publication not bound"@,
                            _ => false,
                        },
                        Ok(true) => exists|observed: S, observation: Result<Option<String>, String>|
                            #[trigger] owner.ensures((confirmed, &inventory), (observed, observation))
                            && match observation {
                                Err(error) => out == (observed, Err::<(), String>(error)),
                                Ok(None) => out.0 == observed && match out.1 {
                                    Err(error) => error@ == "E_CACHE_ATTEMPT: plan owner missing"@,
                                    _ => false,
                                },
                                Ok(Some(id)) => exists|consumed: S, consumption: Result<L, String>|
                                    #[trigger] consume.ensures((observed, &inventory, &id), (consumed, consumption))
                                    && match consumption {
                                        Err(error) => out == (consumed, Err::<(), String>(error)),
                                        Ok(lease) => release.ensures((consumed, &inventory, &id, lease), out),
                                    },
                            },
                    },
            }
    }
}

/// 只在确认同一原件、取得当前所有者及其读取租约后释放引用；首错不丢失。
pub fn release_cache_plan<S, I, L, A, C, O, U, R>(
    state: S, has_originals: bool, adopt: A, confirm: C, owner: O, consume: U, release: R,
) -> (out: (S, Result<(), String>))
    where A: FnOnce(S) -> (S, Result<I, String>),
        C: FnOnce(S, &I) -> (S, Result<bool, String>),
        O: FnOnce(S, &I) -> (S, Result<Option<String>, String>),
        U: FnOnce(S, &I, &String) -> (S, Result<L, String>),
        R: FnOnce(S, &I, &String, L) -> (S, Result<(), String>),
    requires forall|state: S| adopt.requires((state,)),
        forall|state: S, inventory: &I| confirm.requires((state, inventory)),
        forall|state: S, inventory: &I| owner.requires((state, inventory)),
        forall|state: S, inventory: &I, id: &String| consume.requires((state, inventory, id)),
        forall|state: S, inventory: &I, id: &String, lease: L|
            release.requires((state, inventory, id, lease)),
    ensures cache_plan_release_result(state, has_originals, adopt, confirm, owner,
        consume, release, out),
{
    if !has_originals { return (state, Ok(())); }
    let ghost initial = state;
    let (state, adoption) = adopt(state);
    let ghost adopted = state;
    let ghost adoption_result = adoption;
    assert(adopt.ensures((initial,), (adopted, adoption_result)));
    let inventory = match adoption {
        Err(error) => return (state, Err(error)), Ok(inventory) => inventory,
    };
    let (state, confirmation) = confirm(state, &inventory);
    let ghost confirmed = state;
    let ghost confirmation_result = confirmation;
    assert(confirm.ensures((adopted, &inventory), (confirmed, confirmation_result)));
    match confirmation {
        Err(error) => return (state, Err(error)),
        Ok(false) => return (state, Err(
            "E_CACHE_ATTEMPT: consumer plan publication not bound".to_string())),
        Ok(true) => (),
    }
    let (state, observation) = owner(state, &inventory);
    let ghost observed = state;
    let ghost owner_result = observation;
    assert(owner.ensures((confirmed, &inventory), (observed, owner_result)));
    let id = match observation {
        Err(error) => return (state, Err(error)),
        Ok(None) => return (state, Err("E_CACHE_ATTEMPT: plan owner missing".to_string())),
        Ok(Some(id)) => id,
    };
    let (state, consumption) = consume(state, &inventory, &id);
    let ghost consumed = state;
    let ghost consumption_result = consumption;
    assert(consume.ensures((observed, &inventory, &id), (consumed, consumption_result)));
    let lease = match consumption {
        Err(error) => return (state, Err(error)), Ok(lease) => lease,
    };
    let out = release(state, &inventory, &id, lease);
    assert(release.ensures((consumed, &inventory, &id, lease), out));
    out
}

pub open spec fn cache_plan_publication_result<S, T, I, V, E, P, A, C, B>(
    initial: S, has_originals: bool, validate: V, encode: E, publish: P,
    adopt: A, confirm: C, bind: B, out: (S, Result<bool, String>),
) -> bool
    where V: FnOnce(S) -> (S, Result<(), String>),
        E: FnOnce(S) -> (S, Result<T, String>),
        P: FnOnce(S, &T) -> (S, Result<bool, String>),
        A: FnOnce(S) -> (S, Result<I, String>),
        C: FnOnce(S, &I, &T) -> (S, Result<bool, String>),
        B: FnOnce(S, I) -> (S, Result<(), String>),
{
    if !has_originals {
        out == (initial, Ok::<bool, String>(false))
    } else {
        exists|validated: S, validation: Result<(), String>|
            #[trigger] validate.ensures((initial,), (validated, validation))
            && match validation {
                Err(error) => out == (validated, Err::<bool, String>(error)),
                Ok(()) => exists|encoded: S, encoding: Result<T, String>|
                    #[trigger] encode.ensures((validated,), (encoded, encoding))
                    && match encoding {
                        Err(error) => out == (encoded, Err::<bool, String>(error)),
                        Ok(raw) => exists|published: S, publication: Result<bool, String>|
                            #[trigger] publish.ensures((encoded, &raw), (published, publication))
                            && match publication {
                                Err(error) => out == (published, Err::<bool, String>(error)),
                                Ok(false) => out.0 == published && match out.1 {
                                    Err(error) => error@ ==
                                        "E_CACHE_ATTEMPT: cache-plan scope must be adopted"@,
                                    _ => false,
                                },
                                Ok(true) => exists|adopted: S, adoption: Result<I, String>|
                                    #[trigger] adopt.ensures((published,), (adopted, adoption))
                                    && match adoption {
                                        Err(error) => out == (adopted, Err::<bool, String>(error)),
                                        Ok(inventory) => exists|confirmed: S, confirmation: Result<bool, String>|
                                            #[trigger] confirm.ensures((adopted, &inventory, &raw),
                                                (confirmed, confirmation))
                                            && match confirmation {
                                                Err(error) => out == (confirmed, Err::<bool, String>(error)),
                                                Ok(false) => out.0 == confirmed && match out.1 {
                                                    Err(error) => error@ ==
                                                        "E_CACHE_ATTEMPT: plan publication unfinished; original attempt retained; no redispatch"@,
                                                    _ => false,
                                                },
                                                Ok(true) => exists|bound: S, binding: Result<(), String>|
                                                    #[trigger] bind.ensures((confirmed, inventory), (bound, binding))
                                                    && out.0 == bound && match binding {
                                                        Err(error) => out.1 == Err::<bool, String>(error),
                                                        Ok(()) => out.1 == Ok::<bool, String>(true),
                                                    },
                                            },
                                    },
                            },
                    },
            }
    }
}

/// 原件资格、编码、发布、采用、确认和依赖绑定按原顺序执行；首错保留原状态及全文。
pub fn publish_cache_plan<S, T, I, V, E, P, A, C, B>(
    state: S, has_originals: bool, validate: V, encode: E, publish: P,
    adopt: A, confirm: C, bind: B,
) -> (out: (S, Result<bool, String>))
    where V: FnOnce(S) -> (S, Result<(), String>),
        E: FnOnce(S) -> (S, Result<T, String>),
        P: FnOnce(S, &T) -> (S, Result<bool, String>),
        A: FnOnce(S) -> (S, Result<I, String>),
        C: FnOnce(S, &I, &T) -> (S, Result<bool, String>),
        B: FnOnce(S, I) -> (S, Result<(), String>),
    requires forall|state: S| validate.requires((state,)),
        forall|state: S| encode.requires((state,)),
        forall|state: S, raw: &T| publish.requires((state, raw)),
        forall|state: S| adopt.requires((state,)),
        forall|state: S, inventory: &I, raw: &T| confirm.requires((state, inventory, raw)),
        forall|state: S, inventory: I| bind.requires((state, inventory)),
    ensures cache_plan_publication_result(state, has_originals,
        validate, encode, publish, adopt, confirm, bind, out),
{
    if !has_originals { return (state, Ok(false)); }
    let ghost initial = state;
    let (state, validation) = validate(state);
    let ghost validated = state;
    let ghost validation_result = validation;
    assert(validate.ensures((initial,), (validated, validation_result)));
    match validation { Err(error) => return (state, Err(error)), Ok(()) => (), }
    let (state, encoding) = encode(state);
    let ghost encoded = state;
    let ghost encoding_result = encoding;
    assert(encode.ensures((validated,), (encoded, encoding_result)));
    let raw = match encoding { Err(error) => return (state, Err(error)), Ok(raw) => raw, };
    let (state, publication) = publish(state, &raw);
    let ghost published = state;
    let ghost publication_result = publication;
    assert(publish.ensures((encoded, &raw), (published, publication_result)));
    match publication {
        Err(error) => return (state, Err(error)),
        Ok(false) => return (state, Err("E_CACHE_ATTEMPT: cache-plan scope must be adopted".to_string())),
        Ok(true) => (),
    }
    let (state, adoption) = adopt(state);
    let ghost adopted = state;
    let ghost adoption_result = adoption;
    assert(adopt.ensures((published,), (adopted, adoption_result)));
    let inventory = match adoption {
        Err(error) => return (state, Err(error)), Ok(inventory) => inventory,
    };
    let (state, confirmation) = confirm(state, &inventory, &raw);
    let ghost confirmed = state;
    let ghost confirmation_result = confirmation;
    assert(confirm.ensures((adopted, &inventory, &raw), (confirmed, confirmation_result)));
    match confirmation {
        Err(error) => return (state, Err(error)),
        Ok(false) => return (state, Err(
            "E_CACHE_ATTEMPT: plan publication unfinished; original attempt retained; no redispatch".to_string())),
        Ok(true) => (),
    }
    let (state, binding) = bind(state, inventory);
    assert(bind.ensures((confirmed, inventory), (state, binding)));
    (state, match binding { Err(error) => Err(error), Ok(()) => Ok(true), })
}
}

#[cfg(not(verus_keep_ghost))]
pub(crate) use runtime::{Attempt, prepare, publish_plan, release_plan, validate};

// JSON, filesystem and retention capabilities are the explicit IO boundary.
// The strict root compiles and proves the same judgments used below; it does
// not claim a proof of the transaction's IO or process engine.
#[cfg(not(verus_keep_ghost))]
mod runtime {
    use super as decision;
    use chrono_harness::retained_artifacts::retention::{Inventory, Publication, unique_path};
    use chrono_harness::{artifact_disposal::identity, decode, no_symlink_parents, sha256, wire};
    use serde_json::{Value, json};
    use std::{
        cell::Cell,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    const PRODUCER: &str = "cache-probe-attempt";
    const STORE: &str = ".chrono-harness/state/cache-probes";

    fn inventory(root: &Path) -> Result<Inventory, String> {
        decision::admit_attempt_inventory(
            (),
            Inventory::adopted(root),
            |(), state: &Inventory| ((), state.terminal_limits(PRODUCER).is_some()),
            |(), state: &Inventory| ((), state.terminal_limits("cache-plan").is_some()),
        )
        .1
    }
    fn timestamp() -> Result<u64, String> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs())
    }

    pub(crate) struct Attempt {
        pub directory: PathBuf,
        pub relative: String,
        pub publication: Publication,
        pub unknown: Cell<bool>,
    }
    impl Attempt {
        pub fn record(&self, path: &Path, value: &Value) -> Result<(), String> {
            decision::record_cache_attempt(
                (),
                |()| ((), serde_json::to_vec(value).map_err(|e| e.to_string())),
                |(), raw| {
                    let temporary = path.with_extension("json.writing");
                    // write_file already synchronizes the original bytes.
                    let written = self
                        .publication
                        .write_file(&temporary, raw)
                        .map(|()| temporary);
                    ((), written)
                },
                |(), temporary| {
                    (
                        (),
                        fs::hard_link(temporary, path).map_err(|e| e.to_string()),
                    )
                },
                |(), temporary| ((), fs::remove_file(temporary).map_err(|e| e.to_string())),
                |()| ((), path.parent()),
                |(), parent| ((), fs::File::open(parent).map_err(|e| e.to_string())),
                |(), file| {
                    let synced = file.sync_all();
                    drop(file);
                    ((), synced.map_err(|e| e.to_string()))
                },
            )
            .1
        }
    }

    /// An explicit plan-output names the exact attempt. Calls without an output
    /// allocate independent attempts, including callers of the public Rust API.
    pub(crate) fn prepare(
        root: &Path,
        c: &crate::Config,
        consumer: &str,
        output: Option<&str>,
    ) -> Result<Value, String> {
        crate::validate(c)?;
        crate::name(consumer)?;
        let has_probe = c
            .caches
            .values()
            .filter(|cache| cache.consumers.iter().any(|v| v == consumer))
            .flat_map(|cache| {
                cache
                    .compatibility_inputs
                    .iter()
                    .chain(&cache.source_inputs)
            })
            .any(|id| matches!(c.inputs[id], crate::Input::Command { .. }));
        if !has_probe {
            return crate::prepare_inputs(root, c, consumer, None, None);
        }
        let began = std::time::Instant::now();
        let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
        let store = no_symlink_parents(&root, STORE)?;
        fs::create_dir_all(&store).map_err(|e| e.to_string())?;
        let directory = match output {
            Some(output) => store.join(format!("plan-{}", sha256(output.as_bytes()))),
            None => unique_path(&store, "attempt-", "")?,
        };
        let relative = directory
            .strip_prefix(&root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("E_CACHE_ATTEMPT: UTF-8 path")?
            .to_owned();
        let span = tracing::debug_span!("cache_attempt", attempt = relative);
        let _entered = span.enter();
        let role = format!("cache-plan:{}", sha256(relative.as_bytes()));
        let request = json!({"host":root,"host_identity":identity(&root)?,"attempt":relative,
        "consumer":consumer,"registration":c,"plan_output":output});
        let state = inventory(&root)?;
        let rooted = state.rooted_original(PRODUCER, &role)?;
        let existing = match rooted {
            Some(record) => Some(record),
            None if state.enrolled(&relative)? => {
                let status = state.producer_status(PRODUCER, Some(&relative))?;
                status["records"]
                    .as_array()
                    .and_then(|rows| rows.first())
                    .map(|record| {
                        (
                            record["id"].as_str().unwrap_or_default().to_owned(),
                            relative.clone(),
                            record["phase"].as_str().unwrap_or_default().to_owned(),
                            record["outcome"].clone(),
                        )
                    })
            }
            None => None,
        };
        if let Some((id, original, mut phase, retained)) = existing {
            let mut declared = &retained;
            for _ in 0..8 {
                if declared.get("request_sha256").is_some() {
                    break;
                }
                declared = declared.get("original").unwrap_or(declared);
            }
            if !decision::same_attempt(&relative, Some(&original))
                || !decision::same_attempt(
                    &wire::digest(&request)?,
                    declared["request_sha256"].as_str(),
                )
            {
                return Err(format!(
                    "E_CACHE_ATTEMPT: original request differs; preserve {relative}"
                ));
            }
            let path = directory.join("result.json");
            let raw = fs::read(&path).map_err(|e| {
            format!(
                "E_CACHE_ATTEMPT: unknown original result ({e}); no redispatch; original {relative}"
            )
        })?;
            let result: Value = decode(&raw)?;
            if !decision::replay_attempt(
                decision::same_attempt(&wire::digest(&request)?, result["request_sha256"].as_str()),
                result["terminal"] == true,
            ) {
                return Err(format!(
                    "E_CACHE_ATTEMPT: original result unknown or unbound; no redispatch; original {relative}"
                ));
            }
            if phase == "sealing" {
                state.maintain_originals(std::slice::from_ref(&id))?;
                let status = state.producer_status(PRODUCER, Some(&relative))?;
                phase = status["records"][0]["phase"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                if phase == "sealing" {
                    return Err(format!(
                        "E_CACHE_ATTEMPT: original sealing unfinished; no redispatch; original {relative}"
                    ));
                }
            }
            if phase == "publishing" {
                inventory(&root)?.resume_unfinished_directory(PRODUCER, &relative)?
                .ok_or("E_CACHE_ATTEMPT: unfinished publication unavailable")?
                .complete_sealed(json!({"request_sha256":wire::digest(&request)?,"result":"result.json","completion":"confirmed"}))
                .map_err(|error| publication_error(&relative, &error, result["error"].as_str()))?;
            }
            if !inventory(&root)?.published_tree_member(
                PRODUCER,
                &relative,
                &format!("{relative}/result.json"),
                &sha256(&raw),
                raw.len() as u64,
            )? {
                return Err(format!(
                    "E_CACHE_ATTEMPT: original publication unverified; preserve {relative}"
                ));
            }
            let _use = state.consume(&id)?;
            // A terminal failed attempt has no downstream plan consumer. If
            // the first execution was interrupted after sealing its result but
            // before releasing the plan root, replay resumes that same
            // producer-owned release obligation after the exact result is
            // verified and consumed.
            if result["error"].is_string() {
                inventory(&root)?.reference_paths(&role, &[])?;
            }
            return replay(&result);
        }
        // Retired/unrooted addresses cannot be used to repeat the same attempt.
        if state.enrolled(&relative)? || directory.exists() {
            return Err(format!(
                "E_CACHE_ATTEMPT: original unavailable or released; no redispatch; original {relative}"
            ));
        }
        tracing::debug!(
            event_id = "cache.attempt.admitted",
            elapsed_seconds = began.elapsed().as_secs_f64(),
            "new probe attempt admission returned"
        );
        let intent = json!({"schema":"chrono-cache-attempt/v1","request":request,
        "request_sha256":wire::digest(&request)?,"updated":timestamp()?,
        "task_identity":null,"result":null,"completion":"unknown",
        "wait":"original process engine joins its owned work; missing result cannot be reacquired by redispatch",
        "consumer_role":role});
        let publication = state.plan_rooted(PRODUCER, &relative, json!({"request_sha256":wire::digest(&request)?,"intent":"intent.json","completion":"not-established"}), &role)?;
        publication.materialize()?;
        tracing::debug!(
            event_id = "cache.attempt.materialized",
            elapsed_seconds = began.elapsed().as_secs_f64(),
            "new probe attempt materialization returned"
        );
        let attempt = Attempt {
            directory,
            relative: relative.clone(),
            publication,
            unknown: Cell::new(false),
        };
        attempt.record(&attempt.directory.join("intent.json"), &intent)?;
        let result = crate::prepare_inputs(&root, c, consumer, None, Some(&attempt));
        tracing::debug!(
            event_id = "cache.attempt.inputs_returned",
            elapsed_seconds = began.elapsed().as_secs_f64(),
            succeeded = result.is_ok(),
            "cache input preparation returned"
        );
        let terminal = !attempt.unknown.get();
        let receipt = json!({"schema":"chrono-cache-attempt-result/v1","request_sha256":wire::digest(&request)?,
        "updated":timestamp()?,"terminal":terminal,"plan":result.as_ref().ok(),"error":result.as_ref().err()});
        decision::finalize_cache_attempt(
            attempt,
            terminal,
            result,
            |attempt, _result| {
                let recorded = attempt
                    .record(&attempt.directory.join("result.json"), &receipt)
                    .inspect(|_| {
                        tracing::debug!(
                            event_id = "cache.attempt.result_recorded",
                            elapsed_seconds = began.elapsed().as_secs_f64(),
                            terminal,
                            "original attempt result was recorded"
                        );
                    });
                (attempt, recorded)
            },
            |attempt| (attempt, wire::digest(&request)),
            |attempt, request_sha256| {
                (
                    (),
                    attempt
                        .publication
                        .complete_sealed(json!({"request_sha256":request_sha256,
                        "result":"result.json","completion":"confirmed"}))
                        .inspect(|_| {
                            tracing::debug!(
                                event_id = "cache.attempt.sealed",
                                elapsed_seconds = began.elapsed().as_secs_f64(),
                                "terminal attempt sealing returned"
                            );
                        }),
                )
            },
            |()| {
                (
                    (),
                    inventory(&root).and_then(|state| state.reference_paths(&role, &[])),
                )
            },
        )
        .1
        .map_err(|error| decision::cache_attempt_failure_fragments(&relative, &error).concat())
    }
    fn publication_error(relative: &str, publication: &str, original: Option<&str>) -> String {
        decision::cache_publication_error_fragments(relative, publication, original).concat()
    }
    fn replay(result: &Value) -> Result<Value, String> {
        decision::replay_cache_result(
            result.get("plan"),
            result["plan"].is_null(),
            result["error"].as_str(),
        )
        .cloned()
    }

    /// Derive custody only from current command originals and verify the unchanged
    /// original plan. Recovery and seal never execute the command again.
    pub(crate) fn validate(root: &Path, plan: &Value, release: bool) -> Result<(), String> {
        let mut attempts = std::collections::BTreeSet::new();
        for input in plan["inputs"]
            .as_object()
            .ok_or("E_CACHE_ATTEMPT: inputs")?
            .values()
        {
            if input["declaration"]["kind"] != "command" {
                continue;
            }
            let original = input["original"]["path"]
                .as_str()
                .ok_or("E_CACHE_ATTEMPT: missing process original")?;
            let path = Path::new(original);
            let directory = path
                .parent()
                .and_then(Path::parent)
                .ok_or("E_CACHE_ATTEMPT: original address")?;
            let relative = directory
                .to_str()
                .ok_or("E_CACHE_ATTEMPT: original UTF-8")?;
            if !relative.starts_with(&format!("{STORE}/")) {
                return Err("E_CACHE_ATTEMPT: original outside attempt".into());
            }
            let state = inventory(root)?;
            let status = state.producer_status(PRODUCER, Some(relative))?;
            let id = status["records"][0]["id"]
                .as_str()
                .ok_or("E_CACHE_ATTEMPT: original identity missing")?;
            let _use = state.consume(id)?;
            let raw = fs::read(no_symlink_parents(root, original)?).map_err(|e| e.to_string())?;
            if !decision::same_attempt(&sha256(&raw), input["original"]["sha256"].as_str())
                || !decision::same_attempt(id, input["original"]["attempt_id"].as_str())
            {
                return Err("E_CACHE_ATTEMPT: original digest or attempt identity differs".into());
            }
            if !state.published_tree_member(
                PRODUCER,
                relative,
                original,
                &sha256(&raw),
                raw.len() as u64,
            )? {
                return Err("E_CACHE_ATTEMPT: process original is not sealed and bound".into());
            }
            let process: chrono_harness::ProcessResult = decode(&raw)?;
            chrono_harness::validate_launch_evidence(
                &root.join(path.with_file_name("launch.json")),
                &process,
            )?;
            attempts.insert(relative.to_owned());
        }
        for relative in attempts {
            let state = inventory(root)?;
            let result_path = format!("{relative}/result.json");
            let raw =
                fs::read(no_symlink_parents(root, &result_path)?).map_err(|e| e.to_string())?;
            let result: Value = decode(&raw)?;
            let bound = result["plan"] == *plan;
            let terminal = result["terminal"] == true;
            let sealed = state.published_tree_member(
                PRODUCER,
                &relative,
                &result_path,
                &sha256(&raw),
                raw.len() as u64,
            )?;
            if !decision::accept_original(bound, terminal, sealed) {
                return Err(
                    "E_CACHE_ATTEMPT: original plan publication differs or is unfinished".into(),
                );
            }
            let status = state.producer_status(PRODUCER, Some(&relative))?;
            let id = status["records"][0]["id"]
                .as_str()
                .ok_or("E_CACHE_ATTEMPT: original owner identity missing")?;
            let _use = state.consume(id)?;
            if decision::release_custody(bound, terminal, sealed, release) {
                state
                    .reference_paths(&format!("cache-plan:{}", sha256(relative.as_bytes())), &[])?;
            }
        }
        Ok(())
    }

    /// Publish the existing external plan slot through the same retention owner.
    /// Its pending handoff root protects the attempt dependency until seal returns.
    pub(crate) fn publish_plan(root: &Path, path: &str, plan: &Value) -> Result<bool, String> {
        let originals: Vec<String> = plan["inputs"]
            .as_object()
            .ok_or("E_CACHE_ATTEMPT: inputs")?
            .values()
            .filter(|input| input["declaration"]["kind"] == "command")
            .map(|input| {
                input["original"]["path"]
                    .as_str()
                    .ok_or("E_CACHE_ATTEMPT: original missing")
                    .map(str::to_owned)
            })
            .collect::<Result<_, _>>()?;
        let began = std::time::Instant::now();
        decision::publish_cache_plan(
            (),
            !originals.is_empty(),
            |()| {
                let result = validate(root, plan, false).inspect(|_| {
                    tracing::debug!(
                        event_id = "cache.plan.originals_validated",
                        elapsed_seconds = began.elapsed().as_secs_f64(),
                        "plan original qualification returned"
                    );
                });
                ((), result)
            },
            |()| ((), serde_json::to_vec(plan).map_err(|e| e.to_string())),
            |(), raw: &Vec<u8>| {
                ((), chrono_harness::retained_artifacts::retention::publish_final_original(
                    root,
                    "cache-plan",
                    path,
                    raw,
                    json!({"kind":"cache-plan","completion":"confirmed","plan_sha256":sha256(raw)}),
                    true,
                ))
            },
            |()| {
                tracing::debug!(
                    event_id = "cache.plan.original_published",
                    elapsed_seconds = began.elapsed().as_secs_f64(),
                    "plan original publication returned"
                );
                ((), inventory(root))
            },
            |(), state: &Inventory, raw: &Vec<u8>| {
                ((), state.published_original(path, &sha256(raw), raw.len() as u64))
            },
            |(), state: Inventory| {
                let result = state.dependencies_paths(path, &originals).inspect(|_| {
                    tracing::debug!(
                        event_id = "cache.plan.dependencies_bound",
                        elapsed_seconds = began.elapsed().as_secs_f64(),
                        "published plan dependencies were bound"
                    );
                });
                ((), result)
            },
        ).1
    }

    pub(crate) fn release_plan(root: &Path, path: &str, raw: &[u8]) -> Result<(), String> {
        let plan: Value = decode(raw)?;
        let has_originals = plan["inputs"]
            .as_object()
            .ok_or("E_CACHE_ATTEMPT: inputs")?
            .values()
            .any(|input| input["declaration"]["kind"] == "command");
        decision::release_cache_plan(
            (),
            has_originals,
            |()| ((), inventory(root)),
            |(), state: &Inventory| {
                (
                    (),
                    state.published_original(path, &sha256(raw), raw.len() as u64),
                )
            },
            |(), state: &Inventory| {
                (
                    (),
                    state
                        .producer_status("cache-plan", Some(path))
                        .map(|status| status["records"][0]["id"].as_str().map(str::to_owned)),
                )
            },
            |(), state: &Inventory, id: &String| ((), state.consume(id.as_str())),
            |(), state: &Inventory, id: &String, _lease| {
                (
                    (),
                    state.reference(&format!("consumer:publication-recheck:{id}"), &[]),
                )
            },
        )
        .1
    }
}
