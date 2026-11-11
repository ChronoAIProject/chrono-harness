//! Recovery decisions shared by the runtime and the proof.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub enum IntentLayer { Done, Original, Intent, Invalid }

pub fn intent_layer(schema: bool, original: bool, intent: bool) -> (layer: IntentLayer)
    ensures
        (layer is Done) == schema,
        (layer is Original) == (!schema && original),
        (layer is Intent) == (!schema && !original && intent),
        (layer is Invalid) == (!schema && !original && !intent),
{
    if schema { IntentLayer::Done }
    else if original { IntentLayer::Original }
    else if intent { IntentLayer::Intent }
    else { IntentLayer::Invalid }
}

pub fn record_bound(identity: bool, attempt: bool, digest: bool) -> (bound: bool)
    ensures bound == (identity && attempt && digest),
{
    identity && attempt && digest
}

pub enum Repair {
    Leave,
    Restore,
    Preserve,
}

pub fn repair(before: bool, installed: bool, backup_ready: bool, committed: bool) -> (action: Repair)
    ensures
        (action is Leave) == (if committed { installed } else { before }),
        (action is Restore) == (!committed && !before && installed && backup_ready),
        (action is Preserve) == (!(if committed { installed } else { before })
                                 && !(!committed && !before && installed && backup_ready)),
{
    if if committed { installed } else { before } {
        Repair::Leave
    } else if !committed && installed && backup_ready {
        Repair::Restore
    } else {
        Repair::Preserve
    }
}

pub fn can_release(sealed: bool, result_bound: bool, resolved: bool) -> (release: bool)
    ensures release == (sealed && result_bound && resolved),
{
    sealed && result_bound && resolved
}

}
