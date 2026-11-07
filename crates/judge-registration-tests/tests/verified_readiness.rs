//! full 准备状态的独立边界预期，直接调用生产判断。
#![allow(unexpected_cfgs)]
#[path = "../../judge-registration/src/verified_readiness.rs"]
mod readiness;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_exact_active_registry_is_ready() {
    proof {
        reveal_strlit("active"); reveal_strlit("proposed"); reveal_strlit("");
        reveal_strlit("Active"); reveal_strlit("active\0"); reveal_strlit("启用");
    }
    let ready = readiness::registry_active(Some("active"));
    assert(ready);
    let missing = readiness::registry_active(None);
    assert(!missing);
    let proposed = readiness::registry_active(Some("proposed"));
    assert(!proposed);
    let empty = readiness::registry_active(Some(""));
    assert(!empty);
    let case = readiness::registry_active(Some("Active"));
    assert(!case);
    let nul = readiness::registry_active(Some("active\0"));
    assert(!nul);
    let unicode = readiness::registry_active(Some("启用"));
    assert(!unicode);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn activation_requires_all_three_current_input_conditions() {
    proof {
        reveal_strlit("enabled"); reveal_strlit("not-implemented");
        reveal_strlit("declared-complete"); reveal_strlit("incomplete");
    }
    let empty: Vec<u8> = Vec::new();
    let ready = readiness::input_ready(Some("enabled"), Some("declared-complete"), Some(&empty));
    assert(ready);
    let disabled = readiness::input_ready(Some("not-implemented"), Some("declared-complete"), Some(&empty));
    assert(!disabled);
    let incomplete = readiness::input_ready(Some("enabled"), Some("incomplete"), Some(&empty));
    assert(!incomplete);
    let unresolved = vec![0u8];
    let pending = readiness::input_ready(Some("enabled"), Some("declared-complete"), Some(&unresolved));
    assert(!pending);
    let multiple = vec![0u8, 255u8];
    let pending_multiple = readiness::input_ready(Some("enabled"), Some("declared-complete"), Some(&multiple));
    assert(!pending_multiple);
    let absent_enforcement = readiness::input_ready(None, Some("declared-complete"), Some(&empty));
    assert(!absent_enforcement);
    let absent_closure = readiness::input_ready(Some("enabled"), None, Some(&empty));
    assert(!absent_closure);
    let absent_unresolved = readiness::input_ready::<u8>(Some("enabled"), Some("declared-complete"), None);
    assert(!absent_unresolved);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn empty_collection_does_not_make_mismatched_text_ready() {
    proof {
        reveal_strlit("enabled"); reveal_strlit("Enabled"); reveal_strlit("enabled\0");
        reveal_strlit("declared-complete"); reveal_strlit("declared-complete\0");
        reveal_strlit(""); reveal_strlit("完整");
    }
    let empty: Vec<&str> = Vec::new();
    let case = readiness::input_ready(Some("Enabled"), Some("declared-complete"), Some(&empty));
    assert(!case);
    let enforcement_nul = readiness::input_ready(Some("enabled\0"), Some("declared-complete"), Some(&empty));
    assert(!enforcement_nul);
    let closure_nul = readiness::input_ready(Some("enabled"), Some("declared-complete\0"), Some(&empty));
    assert(!closure_nul);
    let missing_text = readiness::input_ready(Some("enabled"), Some(""), Some(&empty));
    assert(!missing_text);
    let unicode = readiness::input_ready(Some("enabled"), Some("完整"), Some(&empty));
    assert(!unicode);
}

}
