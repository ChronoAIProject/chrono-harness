//! Exact admission of an explicitly declared bootstrap preparation prerequisite.
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_identifiers.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
use identifiers::equal_text;

verus! {

pub open spec fn preparation(
    command: Seq<&str>, program: Seq<char>, script: Seq<char>, config: Seq<char>,
    schema: Seq<char>, prerequisites: Seq<&str>, verifier: Seq<char>,
) -> bool {
    command.len() == 4
        && command[0]@ == program && command[1]@ == script
        && command[2]@ == seq!['.'] && command[3]@ == config
        && schema == "chrono-bootstrap/v1"@
        && prerequisites.len() == 1 && prerequisites[0]@ == verifier
}

pub fn matches_preparation(
    command: &Vec<&str>, program: &str, script: &str, config: &str,
    schema: &str, prerequisites: &Vec<&str>, verifier: &str,
) -> (matches: bool)
    ensures matches == preparation(command@, program@, script@, config@,
        schema@, prerequisites@, verifier@),
{
    command.len() == 4
        && equal_text(command[0], program) && equal_text(command[1], script)
        && equal_text(command[2], ".") && equal_text(command[3], config)
        && equal_text(schema, "chrono-bootstrap/v1")
        && prerequisites.len() == 1 && equal_text(prerequisites[0], verifier)
}

}
