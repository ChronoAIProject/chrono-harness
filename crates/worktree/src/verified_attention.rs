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

}
