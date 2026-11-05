//! Boundary proofs exercise the same executable preparation predicate.
#![allow(unexpected_cfgs)]
#[path = "../../judge-registration/src/verified_consumption.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn preparation_requires_exact_command_and_prerequisite() {
    proof {
        reveal_strlit("python"); reveal_strlit("bootstrap"); reveal_strlit(".");
        reveal_strlit("profile"); reveal_strlit("projects"); reveal_strlit("verify");
        reveal_strlit("chrono-bootstrap/v1"); reveal_strlit("other"); reveal_strlit("extra");
        assert("verify"@.len() != "other"@.len());
        assert("profile"@.len() != "other"@.len());
        assert("chrono-bootstrap/v1"@.len() != "other"@.len());
        assert("."@.len() != "other"@.len());
        assert("python"@.len() != "other"@.len());
        assert("projects"@.len() != "other"@.len());
    }
    let command = vec!["python", "bootstrap", ".", "profile"];
    let prerequisites = vec!["verify"];
    let accepted = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(accepted);
    let missing: Vec<&str> = Vec::new();
    let absent = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &missing, "verify", "python", "python", "projects", "projects");
    assert(!absent);
    let duplicate = vec!["verify", "verify"];
    let duplicated = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &duplicate, "verify", "python", "python", "projects", "projects");
    assert(!duplicated);
    let wrong = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "other", "python", "python", "projects", "projects");
    assert(!wrong);
    let changed = policy::matches_preparation(&command, "python", "bootstrap", "other",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!changed);
    let schema = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "other", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!schema);
    let root = vec!["python", "bootstrap", "other", "profile"];
    let wrong_root = policy::matches_preparation(&root, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!wrong_root);
    let extra = vec!["python", "bootstrap", ".", "profile", "extra"];
    let extra_arg = policy::matches_preparation(&extra, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!extra_arg);
    let executor = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "other", "python", "projects", "projects");
    assert(!executor);
    let registry = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "other", "projects");
    assert(!registry);
}

}
