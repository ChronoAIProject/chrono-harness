//! 专属测试的语言配对；直接消费登记中的完整文本。
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

verus! {

pub open spec fn language_pair(implementation: Seq<char>, testing: Seq<char>) -> bool {
    implementation == testing || implementation == "shell"@ && testing == "python"@
}

pub fn dedicated_test_language(implementation: &str, testing: &str) -> (accepted: bool)
    ensures accepted == language_pair(implementation@, testing@),
{
    identifiers::equal_text(implementation, testing)
        || identifiers::equal_text(implementation, "shell")
            && identifiers::equal_text(testing, "python")
}

}
