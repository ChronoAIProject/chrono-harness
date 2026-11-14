//! 原件交接的请求边界、精确成员与局部释放；IO 仍归保留所有者。
#[cfg(verus_keep_ghost)]
#[path = "retention_command.rs"]
pub(crate) mod commands;
#[cfg(verus_keep_ghost)]
#[path = "retention.rs"]
mod retention;
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

pub open spec fn recovery_text(text: Option<&str>) -> Option<Seq<char>> {
    match text { Some(value) => Some(value@), None => None }
}

fn same_recovery_text(left: Option<&str>, right: Option<&str>) -> (same: bool)
    ensures same == (recovery_text(left) == recovery_text(right)),
{
    match (left, right) {
        (Some(a), Some(b)) => identifiers::equal_text(a, b),
        (None, None) => true,
        _ => false,
    }
}

pub fn report_publication(schema: &str, exit_code: u8) -> (valid: bool)
    ensures valid == (schema@ == "chrono-report-publication/v1"@ && exit_code <= 2),
{
    identifiers::equal_text(schema, "chrono-report-publication/v1") && exit_code <= 2
}

/// 0 rejects the address/format; 1 and 2 are the two current scoped wire
/// representations; 3 is the current full report. No historical decoder runs.
pub fn recovered_report_kind(schema: Option<&str>, version: Option<u64>,
    retained_path: Option<&str>, full_path: Option<&str>, original: &str) -> (kind: u8)
    ensures kind == if recovery_text(schema) == Some("chrono-check-report/v1"@)
            && recovery_text(retained_path) == Some(original@) { 1u8 }
        else if recovery_text(schema) == Some("chrono-check-report/v2"@)
            && recovery_text(retained_path) == Some(original@) { 2u8 }
        else if schema.is_none() && version == Some(1u64)
            && recovery_text(full_path) == Some(original@) { 3u8 }
        else { 0u8 },
{
    match schema {
        Some(value) => {
            if identifiers::equal_text(value, "chrono-check-report/v1")
                && same_recovery_text(retained_path, Some(original)) { 1 }
            else if identifiers::equal_text(value, "chrono-check-report/v2")
                && same_recovery_text(retained_path, Some(original)) { 2 }
            else { 0 }
        },
        None => {
            if version == Some(1) && same_recovery_text(full_path, Some(original)) { 3 }
            else { 0 }
        },
    }
}

pub fn report_outcome(records: usize, request: Option<&str>, exit_code: Option<u64>,
    expected_request: &str, expected_exit: u8) -> (matches: bool)
    ensures matches == (records == 1 && recovery_text(request) == Some(expected_request@)
        && exit_code == Some(expected_exit as u64)),
{
    records == 1 && same_recovery_text(request, Some(expected_request))
        && exit_code == Some(expected_exit as u64)
}

pub struct ReportSlot {
    pub allowed: bool,
    pub already_published: bool,
}

/// A retry may reuse its exact bytes or replace only the originally observed
/// slot. A removed/replaced slot is not authority to overwrite newer results.
pub fn report_slot(current: Option<&str>, previous: Option<&str>, desired: &str) -> (plan: ReportSlot)
    ensures plan.already_published == (recovery_text(current) == Some(desired@)),
        plan.allowed == (recovery_text(current) == Some(desired@)
            || recovery_text(current) == recovery_text(previous)),
{
    let already_published = same_recovery_text(current, Some(desired));
    ReportSlot { already_published,
        allowed: already_published || same_recovery_text(current, previous) }
}

pub fn domain_recovery(declared: bool, attached: bool) -> (protect: bool)
    ensures protect == (declared && attached),
{ declared && attached }

pub open spec fn role_observation_protects(observation: Option<Option<bool>>) -> bool {
    match observation { None => false, Some(None) => true, Some(Some(member)) => member }
}

/// 已读角色的缺失声明保留原件；现存声明只有实际成员才构成保留根。
pub fn current_roles_protect(observations: &[Option<Option<bool>>]) -> (protect: bool)
    ensures protect == (exists|i: int| 0 <= i < observations.len()
        && role_observation_protects(observations[i])),
{
    let mut i = 0;
    while i < observations.len()
        invariant i <= observations.len(),
            forall|j: int| 0 <= j < i ==> !role_observation_protects(observations[j]),
        decreases observations.len() - i,
    {
        let protect = match observations[i] {
            None => false,
            Some(None) => true,
            Some(Some(member)) => member,
        };
        if protect { return true; }
        i += 1;
    }
    false
}

pub open spec fn finalized_observation(retired: usize, output: Option<(bool, &str)>) -> bool {
    retired == 0 && match output {
        None => false,
        Some((sealed, phase)) => sealed && (phase@ == "released"@ || phase@ == "disposing"@),
    }
}

pub fn original_finalized(retired: usize, output: Option<(bool, &str)>) -> (finalized: bool)
    ensures finalized == finalized_observation(retired, output),
{
    retired == 0 && match output {
        None => false,
        Some((sealed, phase)) => sealed && (identifiers::equal_text(phase, "released")
            || identifiers::equal_text(phase, "disposing")),
    }
}

pub fn defer_finalized_original(discover_under_lease: bool, finalized: bool) -> (defer: bool)
    ensures defer == (discover_under_lease && finalized),
{ discover_under_lease && finalized }

pub fn preserve_discovered_original(member: bool, retained: bool, retiring_frontier: bool)
    -> (preserve: bool)
    ensures preserve == (member || retained || retiring_frontier),
{ member || retained || retiring_frontier }

pub struct DeferredOriginal {
    pending: Option<String>,
}

impl DeferredOriginal {
    pub closed spec fn pending_id(&self) -> Option<String> { self.pending }

    pub fn new() -> (out: Self)
        ensures out.pending_id().is_none(),
    { Self { pending: None } }

    pub fn defer(&mut self, id: String)
        ensures self.pending_id() == Some(id),
    { self.pending = Some(id); }

    pub fn take(&mut self) -> (id: Option<String>)
        ensures id == old(self).pending_id(), self.pending_id().is_none(),
    {
        let mut id = None;
        core::mem::swap(&mut id, &mut self.pending);
        id
    }
}

pub enum FinalizedExpansion {
    RetainOnly,
    RootFrontier,
    RetireFrontier,
    Related,
}

/// 入向引用本身不是保留根。非初始根仍读取一层真实依赖，保留缺失依赖错误。
pub fn finalized_expansion(rooted: bool, initial: Option<&str>, id: &str,
    incoming: usize, roles: usize) -> (out: FinalizedExpansion)
    ensures match out {
        FinalizedExpansion::RetainOnly => rooted && recovery_text(initial) == Some(id@),
        FinalizedExpansion::RootFrontier => rooted && recovery_text(initial) != Some(id@),
        FinalizedExpansion::RetireFrontier => !rooted && recovery_text(initial) == Some(id@)
            && incoming == 0 && roles == 0,
        FinalizedExpansion::Related => !rooted && (recovery_text(initial) != Some(id@)
            || incoming != 0 || roles != 0),
    },
{
    let initial = same_recovery_text(initial, Some(id));
    if rooted {
        if initial { FinalizedExpansion::RetainOnly } else { FinalizedExpansion::RootFrontier }
    } else if initial && incoming == 0 && roles == 0 { FinalizedExpansion::RetireFrontier }
    else { FinalizedExpansion::Related }
}

/// 原件仍有登记时才排除其物理路径；已移除记录不进入内容维护。
pub fn maintenance_path_required(present: bool, initial: bool, retained: bool) -> (required: bool)
    ensures required == (present && (initial || !retained)),
{ present && (initial || !retained) }

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

pub open spec fn pending_retirement_values(records: Seq<(String, bool)>) -> Seq<String>
    decreases records.len(),
{
    if records.len() == 0 { Seq::empty() }
    else if records.last().1 { pending_retirement_values(records.drop_last()) }
    else { pending_retirement_values(records.drop_last()).push(records.last().0) }
}

/// 只根据记录终态选择待恢复项；字节已删除不等于记录已退役。
pub fn pending_retirements(records: &[(String, bool)]) -> (out: Vec<String>)
    ensures out@ == pending_retirement_values(records@),
{
    let mut out = Vec::new();
    let mut at = 0;
    while at < records.len()
        invariant at <= records.len(),
            out@ == pending_retirement_values(records@.take(at as int)),
        decreases records.len() - at,
    {
        if !records[at].1 { out.push(records[at].0.clone()); }
        proof {
            let next = records@.take(at as int + 1);
            assert(next.drop_last() =~= records@.take(at as int));
            assert(next.last() == records@[at as int]);
            reveal_with_fuel(pending_retirement_values, 1);
        }
        at += 1;
    }
    assert(records@.take(at as int) =~= records@);
    out
}

/// 发现阶段的排除和成员保全决策；实际租约及声明 IO 由调用方执行。
pub struct GateDiscovery {
    pub object_excluded: bool,
    pub role_excluded: bool,
    pub retain_member: bool,
}

pub fn gate_discovery(membership_only: bool, maintenance: bool, wait: bool, birth: bool)
    -> (plan: GateDiscovery)
    ensures
        plan.object_excluded == (maintenance && !wait && !birth),
        plan.role_excluded == (membership_only || (maintenance && !wait && !birth)),
        plan.retain_member == membership_only,
{
    let object_excluded = maintenance && !wait && !birth;
    GateDiscovery {
        object_excluded,
        role_excluded: membership_only || object_excluded,
        retain_member: membership_only,
    }
}

/// 所选原件的请求不进入全库事务、输出或消费者枚举。
pub fn global_maintenance(originals: Option<&[String]>) -> (global: bool)
    ensures global == originals.is_none(),
{
    match originals { None => true, Some(_) => false }
}

/// The explicit sweep retains bounded observation bytes, without overflowing
/// when an individual observation cannot fit its remaining report capacity.
pub fn observation_bytes_fit(used: u64, next: u64, maximum: u64) -> (fits: bool)
    ensures fits == (used as int + next as int <= maximum as int),
{
    used <= maximum && next <= maximum - used
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

/// A direct exit does not settle a launch whose owned work has not joined.
pub fn process_completion_known(exit_code: i32, signal: Option<i32>, owned_work_joined: bool) -> (known: bool)
    ensures known == (owned_work_joined && match signal {
        None => exit_code >= 0,
        Some(value) => exit_code == -1 && value > 0,
    }),
{
    owned_work_joined && process_exit_known(exit_code, signal)
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
