#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_judge_workflow::branch;
use serde_json::{Value, json};
use support::*;
fn context(dev: &str, fork: &str) -> Value {
    json!({"base":dev,"dev_tip":dev,"branch_ref":"feature/fixture","fork_point":fork,"branch_started_at":"2026-01-01T00:00:00Z","observed_at":"2026-01-02T00:00:00Z"})
}
fn fixture() -> (tempfile::TempDir, Vec<String>, Value) {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q"]);
    let mut c = vec![];
    for _ in 0..5 {
        c.push(commit(d.path()));
    }
    (d, c, values()[WORKFLOW].clone())
}
#[test]
fn equality_is_allowed_but_either_strict_excess_is_stale() {
    let (d, c, w) = fixture();
    let ctx = context(&c[3], &c[0]);
    let r = branch(d.path(), &c[4], &ctx, &w).unwrap();
    assert_eq!(r["behind_commits"], 3);
    assert_eq!(r["age_seconds"], 86400.0);
    let mut ctx = ctx.clone();
    ctx["observed_at"] = json!("2026-01-02T00:00:00.000000001Z");
    assert!(
        branch(d.path(), &c[4], &ctx, &w)
            .unwrap_err()
            .starts_with("E_BRANCH_STALE:")
    );
    ctx = context(&c[4], &c[0]);
    ctx["observed_at"] = json!("2026-01-01T00:00:01Z");
    assert!(
        branch(d.path(), &c[4], &ctx, &w)
            .unwrap_err()
            .starts_with("E_BRANCH_STALE:")
    );
}
#[test]
fn context_times_and_names_are_explicit_not_wall_clock_or_git_dates() {
    let (d, c, w) = fixture();
    let mut ctx = context(&c[0], &c[0]);
    ctx["branch_ref"] = json!("integration/topic");
    assert_eq!(
        branch(d.path(), &c[4], &ctx, &w).unwrap()["kind"],
        "integration"
    );
    for bad in [
        "dev",
        "unregistered/topic",
        "feature/",
        "feature/a..b",
        "feature/a b",
        "feature/a.lock",
    ] {
        ctx["branch_ref"] = bad.into();
        assert!(
            branch(d.path(), &c[4], &ctx, &w)
                .unwrap_err()
                .starts_with("E_BRANCH_CONTEXT:")
        );
    }
    ctx["branch_ref"] = "feature/topic".into();
    ctx["observed_at"] = "2025-12-31T23:59:59Z".into();
    assert!(
        branch(d.path(), &c[4], &ctx, &w)
            .unwrap_err()
            .starts_with("E_BRANCH_CONTEXT:")
    );
    ctx["observed_at"] = "2026-01-01T01:00:00+01:00".into();
    assert!(
        branch(d.path(), &c[4], &ctx, &w)
            .unwrap_err()
            .starts_with("E_BRANCH_CONTEXT:")
    );
}

#[test]
fn shallow_boundary_at_fork_is_sufficient_but_later_cutoff_is_not() {
    let (d, c, w) = fixture();
    let ctx = context(&c[3], &c[1]);
    std::fs::write(d.path().join(".git/shallow"), format!("{}\n", c[1])).unwrap();
    assert_eq!(
        branch(d.path(), &c[4], &ctx, &w).unwrap()["behind_commits"],
        2
    );
    std::fs::write(d.path().join(".git/shallow"), format!("{}\n", c[2])).unwrap();
    assert!(
        branch(d.path(), &c[4], &ctx, &w)
            .unwrap_err()
            .starts_with("E_HISTORY_MISSING:")
    );
}
#[test]
fn fork_must_be_common_and_history_must_be_available() {
    let (d, c, w) = fixture();
    let mut ctx = context(&c[0], &c[1]);
    assert!(
        branch(d.path(), &c[4], &ctx, &w)
            .unwrap_err()
            .starts_with("E_BRANCH_CONTEXT:")
    );
    ctx = context(&c[4], &c[4]);
    assert!(
        branch(d.path(), &c[0], &ctx, &w)
            .unwrap_err()
            .starts_with("E_BRANCH_CONTEXT:")
    );
    ctx["fork_point"] = "a".repeat(40).into();
    assert!(
        branch(d.path(), &c[4], &ctx, &w)
            .unwrap_err()
            .starts_with("E_HISTORY_MISSING:")
    );
}
#[test]
fn configured_prefixes_and_thresholds_replace_defaults() {
    let (d, c, mut w) = fixture();
    w["feature_prefix"] = "task/".into();
    w["integration_prefix"] = "verify/".into();
    w["staleness"]["max_behind_commits"] = 0.into();
    w["staleness"]["max_age_hours"] = 0.5.into();
    let mut ctx = context(&c[0], &c[0]);
    ctx["branch_ref"] = "verify/one".into();
    ctx["observed_at"] = "2026-01-01T00:30:00Z".into();
    assert_eq!(
        branch(d.path(), &c[4], &ctx, &w).unwrap()["kind"],
        "integration"
    );
    ctx["observed_at"] = "2026-01-01T00:30:01Z".into();
    assert!(
        branch(d.path(), &c[4], &ctx, &w)
            .unwrap_err()
            .starts_with("E_BRANCH_STALE:")
    );
}

#[test]
fn retained_context_freshness_uses_current_observation_without_changing_original() {
    let (_, c, w) = fixture();
    let mut ctx = context(&c[0], &c[0]);
    ctx["observed_at"] = json!("2026-01-01T00:00:01Z");
    let original = chrono_harness::wire::canonical(&ctx).unwrap();
    // The existing 24-hour policy permits equality and rejects one nanosecond over.
    for (nanos, pass) in [
        (1767312000000000000_i128, true),
        (1767312000000000001_i128, false),
        (1767225600000000000_i128, false),
    ] {
        let observation = json!({"unix_timestamp_nanos":nanos.to_string()});
        let result = chrono_judge_workflow::current_observation_age(&ctx, &w, &observation);
        assert_eq!(result.is_ok(), pass, "{result:?}");
        assert_eq!(chrono_harness::wire::canonical(&ctx).unwrap(), original);
    }
}
