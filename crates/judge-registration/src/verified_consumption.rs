//! 精确判定已明确登记的 bootstrap 准备前置。
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

#[verifier::inline]
pub open spec fn preparation(
    command: Seq<&str>, program: Seq<char>, script: Seq<char>, config: Seq<char>,
    schema: Seq<char>, prerequisites: Seq<&str>, verifier: Seq<char>,
    executor: Seq<char>, verifier_program: Seq<char>, projects: Seq<char>, expected_projects: Seq<char>,
) -> bool {
    command.len() == 4
        && command[0]@ == program && command[1]@ == script
        && command[2]@ == "."@ && command[3]@ == config
        && schema == "chrono-bootstrap/v1"@
        && prerequisites.len() == 1 && prerequisites[0]@ == verifier
        && executor == verifier_program && projects == expected_projects
}

pub fn matches_preparation(
    command: &Vec<&str>, program: &str, script: &str, config: &str,
    schema: &str, prerequisites: &Vec<&str>, verifier: &str,
    executor: &str, verifier_program: &str, projects: &str, expected_projects: &str,
) -> (matches: bool)
    ensures matches == preparation(command@, program@, script@, config@,
        schema@, prerequisites@, verifier@, executor@, verifier_program@, projects@, expected_projects@),
{
    command.len() == 4
        && equal_text(command[0], program) && equal_text(command[1], script)
        && equal_text(command[2], ".") && equal_text(command[3], config)
        && equal_text(schema, "chrono-bootstrap/v1")
        && prerequisites.len() == 1 && equal_text(prerequisites[0], verifier)
        && equal_text(executor, verifier_program) && equal_text(projects, expected_projects)
}

}
