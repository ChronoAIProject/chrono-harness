//! 本地生命周期观察的提醒分类；不产生删除或终态授权。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
use identifiers::equal_text;

verus! {

pub enum SnapshotProtocol {
    Unleased,
    Kernel,
    Invalid,
}

pub open spec fn expected_snapshot_protocol(schema: Seq<char>, admission: bool) -> SnapshotProtocol {
    if schema == "chrono-worktree-enrollment-state/v1"@ && !admission { SnapshotProtocol::Unleased }
    else if schema == "chrono-worktree-enrollment-state/v2"@ && admission { SnapshotProtocol::Kernel }
    else { SnapshotProtocol::Invalid }
}

pub fn snapshot_protocol(schema: &str, admission: bool) -> (protocol: SnapshotProtocol)
    ensures protocol == expected_snapshot_protocol(schema@, admission),
{
    if equal_text(schema, "chrono-worktree-enrollment-state/v1") && !admission { SnapshotProtocol::Unleased }
    else if equal_text(schema, "chrono-worktree-enrollment-state/v2") && admission { SnapshotProtocol::Kernel }
    else { SnapshotProtocol::Invalid }
}

pub fn snapshot_owned(protocol: &SnapshotProtocol, owner: bool, coordinator: bool,
    common: bool, addresses: bool, admission: bool) -> (valid: bool)
    ensures valid <==> !(protocol is Invalid) && owner && coordinator && common && addresses
        && (!(protocol is Kernel) || admission),
{
    match protocol {
        SnapshotProtocol::Invalid => false,
        SnapshotProtocol::Unleased => owner && coordinator && common && addresses,
        SnapshotProtocol::Kernel => owner && coordinator && common && addresses && admission,
    }
}

pub fn snapshot_stable(directory: bool, owner: bool, ledger: bool) -> (stable: bool)
    ensures stable <==> directory && owner && ledger,
{
    directory && owner && ledger
}

pub enum Attention {
    Coordinator,
    Active,
    TerminalPending,
    Retained,
    Disposed,
    Unregistered,
    Unknown,
}

pub open spec fn expected_attention(state: Seq<char>, coordinator: bool,
    readable: bool, enrolled: bool, present: bool, git_registered: bool,
    attachment_matches: bool, terminal: bool) -> Attention
{
    if !readable { Attention::Unknown }
    else if coordinator { Attention::Coordinator }
    else if !enrolled { Attention::Unregistered }
    else if state == "disposed"@ && !present && !git_registered && terminal {
        Attention::Disposed
    }
    else if !present || !attachment_matches { Attention::Unknown }
    else if state == "active"@ && !terminal { Attention::Active }
    else if state == "retained"@ && terminal { Attention::Retained }
    else if state == "terminal"@ && terminal { Attention::TerminalPending }
    else { Attention::Unknown }
}

pub fn classify(state: &str, coordinator: bool, readable: bool,
    enrolled: bool, present: bool, git_registered: bool,
    attachment_matches: bool, terminal: bool) -> (view: Attention)
    ensures view == expected_attention(state@, coordinator, readable,
        enrolled, present, git_registered, attachment_matches, terminal),
{
    if !readable { Attention::Unknown }
    else if coordinator { Attention::Coordinator }
    else if !enrolled { Attention::Unregistered }
    else if equal_text(state, "disposed") && !present && !git_registered && terminal {
        Attention::Disposed
    }
    else if !present || !attachment_matches { Attention::Unknown }
    else if equal_text(state, "active") && !terminal { Attention::Active }
    else if equal_text(state, "retained") && terminal { Attention::Retained }
    else if equal_text(state, "terminal") && terminal { Attention::TerminalPending }
    else { Attention::Unknown }
}

pub fn needs_attention(view: &Attention) -> (needed: bool)
    ensures needed <==> !(view is Coordinator || view is Disposed),
{
    match view {
        Attention::Coordinator | Attention::Disposed => false,
        _ => true,
    }
}

pub fn pending_for_agent(creator: &str, current: &str, attention: bool) -> (pending: bool)
    ensures pending <==> (creator@ == current@ && creator@ != ""@ && attention),
{
    equal_text(creator, current) && !equal_text(creator, "") && attention
}

}
