//! 原件交接的请求边界、精确成员与局部释放；IO 仍归保留所有者。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "../../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

verus! {

pub fn domain_recovery(declared: bool, attached: bool) -> (protect: bool)
    ensures protect == (declared && attached),
{ declared && attached }

pub fn terminal_eligible(released: bool, sealed: bool) -> (eligible: bool)
    ensures eligible == (released && sealed),
{ released && sealed }

pub fn resumed_seal(sealing: bool, sealed: bool, interrupted: bool) -> (released: bool)
    ensures released == (sealing && sealed && !interrupted),
{ sealing && sealed && !interrupted }

/// Checked without overflowing count or byte addition.
pub fn terminal_budget(count: u64, bytes: u64, next: u64, max_count: u64, max_bytes: u64) -> (keep: bool)
    ensures keep == (count < max_count && bytes as int + next as int <= max_bytes as int),
{
    count < max_count && bytes <= max_bytes && next <= max_bytes - bytes
}


pub open spec fn present(values: Seq<String>, value: Seq<char>) -> bool {
    exists|at: int| 0 <= at < values.len() && (#[trigger] values[at])@ == value
}

pub fn contains(values: &[String], value: &str) -> (found: bool)
    ensures found == present(values@, value@),
{
    let mut at = 0;
    while at < values.len()
        invariant at <= values.len(),
            forall|before: int| 0 <= before < at ==> values@[before]@ != value@,
        decreases values.len() - at,
    {
        if identifiers::equal_text(values[at].as_str(), value) { return true; }
        at += 1;
    }
    false
}

pub open spec fn unique(values: Seq<String>) -> bool {
    forall|a: int, b: int| 0 <= a < b < values.len()
        ==> (#[trigger] values[a])@ != (#[trigger] values[b])@
}

pub fn unique_selection(values: &[String], maximum: usize) -> (valid: bool)
    ensures valid == (0 < values.len() <= maximum && unique(values@)),
{
    if values.len() == 0 || values.len() > maximum { return false; }
    let mut at = 0;
    while at < values.len()
        invariant at <= values.len(), unique(values@.take(at as int)),
        decreases values.len() - at,
    {
        let mut before = 0;
        while before < at
            invariant before <= at, at < values.len(), unique(values@.take(at as int)),
                forall|i: int| 0 <= i < before ==> values@[i]@ != values@[at as int]@,
            decreases at - before,
        {
            if identifiers::equal_text(values[before].as_str(), values[at].as_str()) {
                return false;
            }
            before += 1;
        }
        assert(unique(values@.take(at as int + 1)));
        at += 1;
    }
    assert(values@.take(at as int) =~= values@);
    true
}

pub open spec fn bounded_roles(values: Seq<String>) -> bool {
    forall|at: int| 0 <= at < values.len()
        ==> 0 < vstd::utf8::encode_utf8((#[trigger] values[at])@).len() <= 256
}

/// trim 和字节长度由标准字符串接口提供，不替换原 Unicode 空白语义。
pub fn valid_request(
    roles: &[String], maximum: usize, trimmed_reason: &str, reason_bytes: usize,
    released: bool, length: u64, max_length: u64,
) -> (valid: bool)
    ensures valid == (released && 0 < trimmed_reason@.len() && reason_bytes <= 4096
        && length <= max_length && 0 < roles.len() <= maximum
        && unique(roles@) && bounded_roles(roles@)),
{
    if !released || trimmed_reason.is_empty() || reason_bytes > 4096
        || length > max_length || !unique_selection(roles, maximum) { return false; }
    let mut at = 0;
    while at < roles.len()
        invariant at <= roles.len(),
            forall|i: int| 0 <= i < at
                ==> 0 < vstd::utf8::encode_utf8((#[trigger] roles@[i])@).len() <= 256,
        decreases roles.len() - at,
    {
        let bytes = roles[at].as_str().as_bytes();
        if bytes.len() == 0 || bytes.len() > 256 { return false; }
        at += 1;
    }
    true
}

pub fn same_members(left: &[String], right: &[String]) -> (same: bool)
    ensures same == (forall|value: Seq<char>| present(left@, value) == present(right@, value)),
{
    let mut at = 0;
    while at < left.len()
        invariant at <= left.len(),
            forall|i: int| 0 <= i < at ==> present(right@, left@[i]@),
        decreases left.len() - at,
    {
        if !contains(right, left[at].as_str()) { return false; }
        at += 1;
    }
    let mut at = 0;
    while at < right.len()
        invariant at <= right.len(),
            forall|i: int| 0 <= i < left.len() ==> present(right@, left@[i]@),
            forall|i: int| 0 <= i < at ==> present(left@, right@[i]@),
        decreases right.len() - at,
    {
        if !contains(left, right[at].as_str()) { return false; }
        at += 1;
    }
    true
}

pub open spec fn removed(values: Seq<String>, id: Seq<char>) -> Seq<String>
    decreases values.len(),
{
    if values.len() == 0 { Seq::empty() }
    else if values.last()@ == id { removed(values.drop_last(), id) }
    else { removed(values.drop_last(), id).push(values.last()) }
}

pub fn without(values: &[String], id: &str) -> (out: Vec<String>)
    ensures out@ == removed(values@, id@),
{
    let mut out = Vec::new();
    let mut at = 0;
    while at < values.len()
        invariant at <= values.len(), out@ == removed(values@.take(at as int), id@),
        decreases values.len() - at,
    {
        if !identifiers::equal_text(values[at].as_str(), id) { out.push(values[at].clone()); }
        proof {
            let next = values@.take(at as int + 1);
            assert(next.drop_last() =~= values@.take(at as int));
            assert(next.last() == values@[at as int]);
            reveal_with_fuel(removed, 1);
        }
        at += 1;
    }
    assert(values@.take(at as int) =~= values@);
    out
}

/// 只有对象观察的成员更新采用只读同伴；普通发布、维护及创建沿用原完整发现。
pub fn observation_peer_frontier(observation: bool, maintenance: bool, birth: bool) -> (use_peers: bool)
    ensures use_peers == (observation && !maintenance && !birth),
{ observation && !maintenance && !birth }

pub fn gate_role_exclusion(nonblocking_maintenance: bool, observation_frontier: bool) -> (excluded: bool)
    ensures excluded == (nonblocking_maintenance || observation_frontier),
{ nonblocking_maintenance || observation_frontier }

pub fn gate_discovery_candidate(
    peers: bool, active: &[String], id: &str, observed: bool, birth: bool,
) -> (discover: bool)
    ensures discover == (!observed && !birth && (!peers || present(active@, id@))),
{
    !observed && !birth && (!peers || contains(active, id))
}

pub open spec fn gate_active_values(active: Seq<String>, references: Seq<String>) -> Seq<String>
    decreases references.len(),
{
    if references.len() == 0 { active }
    else {
        let before = gate_active_values(active, references.drop_last());
        if present(before, references.last()@) { before }
        else { before.push(references.last()) }
    }
}

/// 仅加入实际出向引用，保留原顺序；重复引用不增加成员。
pub fn extend_gate_active(active: &mut Vec<String>, references: &[String])
    ensures active@ == gate_active_values(old(active)@, references@),
{
    let ghost initial = active@;
    let mut at = 0;
    while at < references.len()
        invariant at <= references.len(),
            active@ == gate_active_values(initial, references@.take(at as int)),
        decreases references.len() - at,
    {
        let ghost before = active@;
        if !contains(active, references[at].as_str()) {
            active.push(references[at].clone());
        }
        proof {
            let next = references@.take(at as int + 1);
            assert(next.drop_last() =~= references@.take(at as int));
            assert(next.last() == references@[at as int]);
            reveal_with_fuel(gate_active_values, 1);
        }
        at += 1;
    }
    assert(references@.take(at as int) =~= references@);
}

pub open spec fn gate_peer_values(active: Seq<String>, selected: Seq<String>) -> Seq<String>
    decreases selected.len(),
{
    if selected.len() == 0 { Seq::empty() }
    else if present(active, selected.last()@) { gate_peer_values(active, selected.drop_last()) }
    else { gate_peer_values(active, selected.drop_last()).push(selected.last()) }
}

pub fn gate_role_peers(peers: bool, active: &[String], selected: &[String]) -> (out: Vec<String>)
    ensures out@ == if peers { gate_peer_values(active@, selected@) } else { Seq::<String>::empty() },
{
    let mut out = Vec::new();
    if !peers { return out; }
    let mut at = 0;
    while at < selected.len()
        invariant at <= selected.len(), out@ == gate_peer_values(active@, selected@.take(at as int)),
        decreases selected.len() - at,
    {
        if !contains(active, selected[at].as_str()) { out.push(selected[at].clone()); }
        proof {
            let next = selected@.take(at as int + 1);
            assert(next.drop_last() =~= selected@.take(at as int));
            assert(next.last() == selected@[at as int]);
            reveal_with_fuel(gate_peer_values, 1);
        }
        at += 1;
    }
    assert(selected@.take(at as int) =~= selected@);
    out
}

pub fn must_preserve_gate_peer(observation_frontier: bool, retained: bool) -> (preserve: bool)
    ensures preserve == (observation_frontier && retained),
{ observation_frontier && retained }

pub open spec fn gate_output_position(ids: Seq<&str>, id: Seq<char>, at: int) -> bool {
    0 <= at < ids.len() && ids[at]@ == id
        && forall|other: int| 0 <= other < ids.len() && other != at ==> (#[trigger] ids[other])@ != id
}

pub fn gate_output_index(ids: &[&str], id: &str) -> (out: Result<Option<usize>, &'static str>)
    ensures match out {
        Ok(Some(at)) => gate_output_position(ids@, id@, at as int),
        Ok(None) => forall|at: int| 0 <= at < ids.len() ==> (#[trigger] ids[at])@ != id@,
        Err(error) => error@ == "ambiguous observation peer output; originals preserved"@
            && exists|a: int, b: int| 0 <= a < b < ids.len() && ids[a]@ == id@ && ids[b]@ == id@,
    },
{
    let mut at = 0;
    let mut found: Option<usize> = None;
    while at < ids.len()
        invariant at <= ids.len(),
            match found {
                Some(index) => gate_output_position(ids@.take(at as int), id@, index as int),
                None => forall|before: int| 0 <= before < at ==> (#[trigger] ids[before])@ != id@,
            },
        decreases ids.len() - at,
    {
        if identifiers::equal_text(ids[at], id) {
            if let Some(index) = found {
                assert(0 <= index < at && ids@[index as int]@ == id@);
                assert(exists|a: int, b: int| 0 <= a < b < ids.len() && ids[a]@ == id@ && ids[b]@ == id@);
                return Err("ambiguous observation peer output; originals preserved");
            }
            found = Some(at);
        }
        at += 1;
    }
    assert(ids@.take(at as int) =~= ids@);
    Ok(found)
}

pub fn admit_gate_peer_output(before: &[u8], after: &[u8]) -> (out: Result<(), &'static str>)
    ensures out.is_ok() == (before@ == after@),
        match out {
            Ok(_) => true,
            Err(error) => error@ == "read-only observation peer output changed; originals preserved"@,
        },
{
    if identifiers::equal_bytes(before, after) { Ok(()) }
    else { Err("read-only observation peer output changed; originals preserved") }
}

/// 所选原件的请求不进入全库事务、输出或消费者枚举。
pub fn global_maintenance(originals: Option<&[String]>) -> (global: bool)
    ensures global == originals.is_none(),
{
    match originals { None => true, Some(_) => false }
}


pub fn observation_role(trimmed_role: &str, role_bytes: usize, attempt: u64) -> (ok: bool)
    ensures ok == (0 < trimmed_role@.len() && role_bytes <= 256 && attempt > 0),
{ !trimmed_role.is_empty() && role_bytes <= 256 && attempt > 0 }

pub fn observation_token(trimmed_role: &str, role_bytes: usize, serial: u64, attempt: u64) -> (ok: bool)
    ensures ok == (0 < trimmed_role@.len() && role_bytes <= 256 && serial > 0 && attempt > 0),
{ observation_role(trimmed_role, role_bytes, attempt) && serial > 0 }

pub fn observation_producer(tree: bool, publishing: bool, physical: bool) -> (ok: bool)
    ensures ok == (tree && publishing && physical),
{ tree && publishing && physical }

pub fn observation_owner(ordered: bool, empty: bool, unattached: bool) -> (ok: bool)
    ensures ok == (ordered || (empty && unattached)),
{ ordered || (empty && unattached) }

pub fn next_observation(issued: u64) -> (next: Option<u64>)
    ensures next == if issued < u64::MAX { Some((issued + 1) as u64) } else { None },
{
    if issued == u64::MAX { None } else { Some(issued + 1) }
}

pub fn observation_order(epoch_bytes: usize, issued: u64, accepted: u64,
    has_current: bool, current_member: bool) -> (ok: bool)
    ensures ok == (epoch_bytes == 64 && issued > 0 && accepted <= issued && (!has_current || current_member)),
{ epoch_bytes == 64 && issued > 0 && accepted <= issued && (!has_current || current_member) }

pub fn observation_result(tree: bool, released: bool, sealed: bool,
    address: bool, physical: bool) -> (ok: bool)
    ensures ok == (tree && released && sealed && address && physical),
{ tree && released && sealed && address && physical }

pub struct ObservationBinding<'a> {
    pub role: &'a str, pub epoch: &'a str, pub id: &'a str,
    pub path: &'a str, pub physical: &'a str, pub serial: u64, pub attempt: u64,
}

pub fn matching_observation(request: &ObservationBinding, observed: &ObservationBinding) -> (ok: bool)
    ensures ok == (request.role@ == observed.role@ && request.epoch@ == observed.epoch@
        && request.id@ == observed.id@ && request.path@ == observed.path@
        && request.physical@ == observed.physical@ && 0 < observed.serial <= request.serial
        && 0 < observed.attempt <= request.attempt),
{
    identifiers::equal_text(request.role, observed.role)
        && identifiers::equal_text(request.epoch, observed.epoch)
        && identifiers::equal_text(request.id, observed.id)
        && identifiers::equal_text(request.path, observed.path)
        && identifiers::equal_text(request.physical, observed.physical)
        && observed.serial > 0 && observed.serial <= request.serial
        && observed.attempt > 0 && observed.attempt <= request.attempt
}

pub fn observation_epoch(member: bool, same_epoch: bool, serial: u64, issued: u64) -> (ok: bool)
    ensures ok == (!member || (same_epoch && serial <= issued)),
{ !member || (same_epoch && serial <= issued) }

pub fn observation_edit(ordered: bool, owns_update: bool, same_members: bool) -> (ok: bool)
    ensures ok == (!ordered || owns_update || same_members),
{ !ordered || owns_update || same_members }

pub fn unknown_pr_operation(unreturned: bool, unresolved_create: bool) -> (unknown: bool)
    ensures unknown == (unreturned || unresolved_create),
{ unreturned || unresolved_create }

/// Native wait failure has neither an exit code nor a terminal signal.
/// Returning a receipt alone does not establish either terminal result.
pub fn process_exit_known(exit_code: i32, signal: Option<i32>) -> (known: bool)
    ensures known == match signal {
        None => exit_code >= 0,
        Some(value) => exit_code == -1 && value > 0,
    },
{
    match signal {
        None => exit_code >= 0,
        Some(value) => exit_code == -1 && value > 0,
    }
}

pub fn settled_pr_command(unreturned: bool, unresolved_create: bool, originals_complete: bool) -> (ok: bool)
    ensures ok == (!unreturned && !unresolved_create && originals_complete),
{ !unreturned && !unresolved_create && originals_complete }

pub fn continued_pr_handoff(complete: bool, no_error: bool, observed: bool, matching_head: bool) -> (ok: bool)
    ensures ok == (complete && no_error && observed && matching_head),
{ complete && no_error && observed && matching_head }

pub fn open_pr(state: &str) -> (active: bool)
    ensures active == (state@ == "OPEN"@),
{ proof { reveal_strlit("OPEN"); } identifiers::equal_text(state, "OPEN") }

pub fn observed_pr(state: &str, active: bool) -> (ok: bool)
    ensures ok == ((state@ == "OPEN"@ && active)
        || ((state@ == "CLOSED"@ || state@ == "MERGED"@) && !active)),
{
    proof {
        reveal_strlit("OPEN"); reveal_strlit("CLOSED"); reveal_strlit("MERGED");
        assert("OPEN"@[0] != "CLOSED"@[0] && "OPEN"@[0] != "MERGED"@[0]);
    }
    if open_pr(state) { active }
    else { !active && (identifiers::equal_text(state, "CLOSED") || identifiers::equal_text(state, "MERGED")) }
}

pub fn creating_pr(command: &str, action: &str) -> (create: bool)
    ensures create == (command@ == "pr"@ && action@ == "create"@),
{ proof { reveal_strlit("pr"); reveal_strlit("create"); }
    identifiers::equal_text(command, "pr") && identifiers::equal_text(action, "create") }

pub fn continued_observation(request: &ObservationBinding, role: &str, path: &str) -> (ok: bool)
    ensures ok == (request.role@ == role@ && request.path@ == path@
        && request.serial > 0 && request.attempt > 0),
{ identifiers::equal_text(request.role, role) && identifiers::equal_text(request.path, path)
    && request.serial > 0 && request.attempt > 0 }

pub fn object_state(schema: &str, unresolved_create: bool) -> (ok: bool)
    ensures ok == (schema@ == "chrono-pr-object-state/v1"@ && !unresolved_create),
{ identifiers::equal_text(schema, "chrono-pr-object-state/v1") && !unresolved_create }

pub open spec fn current_value(current: Option<String>) -> Option<Seq<char>> {
    match current { Some(id) => Some(id@), None => None }
}
pub open spec fn pending_removed(values: Seq<String>, id: String, current: Option<String>) -> Seq<String> {
    if current_value(current) == Some(id@) { values } else { removed(values, id@) }
}
pub open spec fn old_current_removed(values: Seq<String>, current: Option<String>) -> Seq<String> {
    match current { Some(id) => removed(values, id@), None => values }
}
pub open spec fn observation_members(values: Seq<String>, id: String, current: Option<String>, accepted: u64, observed: Option<(u64,bool)>) -> Seq<String> {
    let pending = pending_removed(values, id, current);
    match observed {
        Some((serial,active)) if serial > accepted => {
            let next = old_current_removed(pending,current);
            if active {
                if present(next,id@) { next } else { next.push(id) }
            } else { removed(next,id@) }
        },
        _ => pending,
    }
}
pub open spec fn observation_accepted(accepted: u64, observed: Option<(u64,bool)>) -> u64 {
    match observed { Some((serial,_)) if serial > accepted => serial, _ => accepted }
}
pub open spec fn observation_current(id: String, current: Option<String>, accepted: u64, observed: Option<(u64,bool)>) -> Option<Seq<char>> {
    match observed {
        Some((serial,active)) if serial > accepted => if active { Some(id@) } else { None },
        _ => current_value(current),
    }
}
pub struct ObservationSettlement {
    pub members: Vec<String>, pub accepted: u64, pub current: Option<String>,
}
/// The one production member transition. It preserves every unrelated pending
/// output; only the returned command and the superseded current observation move.
pub fn settle_observation_members(values: &[String], id: &String, current: &Option<String>, accepted: u64, observed: Option<(u64,bool)>) -> (out: ObservationSettlement)
    ensures out.members@ == observation_members(values@,*id,*current,accepted,observed),
        out.accepted == observation_accepted(accepted,observed), out.accepted >= accepted,
        current_value(out.current) == observation_current(*id,*current,accepted,observed),
{
    let own_current = match current {
        Some(value) => identifiers::equal_text(value.as_str(),id.as_str()), None => false,
    };
    let mut next = Vec::new();
    if own_current {
        let mut at = 0;
        while at < values.len()
            invariant at <= values.len(), next@ == values@.take(at as int),
            decreases values.len() - at,
        {
            next.push(values[at].clone());
            assert(values@.take(at as int + 1) =~= values@.take(at as int).push(values@[at as int]));
            at += 1;
        }
        assert(values@.take(at as int) =~= values@);
    } else { next = without(values,id.as_str()); }
    match observed {
        Some((serial,active)) if serial > accepted => {
            match current { Some(value) => { next = without(&next,value.as_str()); }, None => {} }
            if active {
                if !contains(&next,id.as_str()) { next.push(id.clone()); }
                ObservationSettlement { members: next, accepted: serial, current: Some(id.clone()) }
            } else {
                next = without(&next,id.as_str());
                ObservationSettlement { members: next, accepted: serial, current: None }
            }
        },
        _ => {
            let kept = match current { Some(value) => Some(value.clone()), None => None };
            ObservationSettlement { members: next, accepted, current: kept }
        },
    }
}

}
