use crate::verified_branch::{self, SourceKind};
use chrono_harness::facts;
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("E_BRANCH_CONTEXT: missing {k}"))
}
fn moment(v: &Value, k: &str) -> Result<OffsetDateTime, String> {
    let d = OffsetDateTime::parse(text(v, k)?, &Rfc3339)
        .map_err(|e| format!("E_BRANCH_CONTEXT: {k}: {e}"))?;
    if d.offset() != UtcOffset::UTC {
        return Err("E_BRANCH_CONTEXT: time must be UTC".into());
    }
    Ok(d)
}
/// Registered observation freshness, shared with the local round producer.
/// The caller supplies the observation; this predicate never reads a clock.
pub fn observation_age(ctx: &Value, w: &Value) -> Result<time::Duration, String> {
    age_at(ctx, w, moment(ctx, "observed_at")?)
}
/// Admission can observe now without changing the original common context.
pub fn current_observation_age(
    ctx: &Value,
    w: &Value,
    observation: &Value,
) -> Result<time::Duration, String> {
    let nanos = observation["unix_timestamp_nanos"]
        .as_str()
        .ok_or("E_BRANCH_CONTEXT: current observation missing")?
        .parse()
        .map_err(|_| "E_BRANCH_CONTEXT: invalid current observation")?;
    let current = OffsetDateTime::from_unix_timestamp_nanos(nanos).map_err(|e| e.to_string())?;
    if current < moment(ctx, "observed_at")? {
        return Err("E_BRANCH_CONTEXT: current observation precedes original production".into());
    }
    age_at(ctx, w, current)
}
fn age_at(ctx: &Value, w: &Value, observed: OffsetDateTime) -> Result<time::Duration, String> {
    let age = observed - moment(ctx, "branch_started_at")?;
    if age.is_negative() {
        return Err("E_BRANCH_CONTEXT: negative age".into());
    }
    let hours = w["staleness"]["max_age_hours"]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or("E_BRANCH_CONTEXT: invalid age threshold")?;
    if age.whole_nanoseconds() > (hours * 3_600_000_000_000.0) as i128 {
        return Err(format!(
            "E_BRANCH_STALE: age_ns={}",
            age.whole_nanoseconds()
        ));
    }
    Ok(age)
}
fn commits(reader: &facts::Reader, root: &Path, oid: &str) -> Result<BTreeSet<String>, String> {
    Ok(facts::utf8(reader.git(root, &["rev-list", oid])?)?
        .lines()
        .map(str::to_string)
        .collect())
}
/// Fixed Git/context inputs only. No branch discovery, wall clock or commit-date inference.
pub fn branch(root: &Path, candidate: &str, ctx: &Value, w: &Value) -> Result<Value, String> {
    with_reader(&facts::Reader::legacy(), root, candidate, ctx, w)
}
pub fn with_reader(
    reader: &facts::Reader,
    root: &Path,
    candidate: &str,
    ctx: &Value,
    w: &Value,
) -> Result<Value, String> {
    let dev = text(ctx, "dev_tip")?;
    let fork = text(ctx, "fork_point")?;
    for oid in [candidate, dev, fork] {
        reader
            .verify_oid(root, oid)
            .map_err(|e| format!("E_HISTORY_MISSING: {e}"))?;
    }
    if ctx["base"] != dev {
        return Err("E_BRANCH_CONTEXT: base/dev mismatch".into());
    }
    let source = text(ctx, "branch_ref")?;
    reader
        .git(root, &["check-ref-format", &format!("refs/heads/{source}")])
        .map_err(|e| format!("E_BRANCH_CONTEXT: invalid Git branch name: {e}"))?;
    let feature_prefix = text(w, "feature_prefix")?;
    let integration_prefix = text(w, "integration_prefix")?;
    let feature_matches = source
        .strip_prefix(feature_prefix)
        .is_some_and(|tail| !tail.is_empty());
    let integration_matches = source
        .strip_prefix(integration_prefix)
        .is_some_and(|tail| !tail.is_empty());
    let kind = verified_branch::select_source_kind(
        feature_matches,
        integration_matches,
        source == text(w, "target_branch")?,
    )
    .ok_or("E_BRANCH_CONTEXT: missing or ambiguous registered source branch")?;
    let kind_name = match kind {
        SourceKind::Feature => "feature",
        SourceKind::Integration => "integration",
    };
    let age = observation_age(ctx, w)?;
    let dev_ancestors = commits(reader, root, dev)?;
    let branch_ancestors = commits(reader, root, candidate)?;
    // Shallow boundaries are acceptable only at/before the proven fork. Any other
    // truncated relevant history prevents a complete count or ancestry decision.
    let fork_ancestors = commits(reader, root, fork)?;
    let shallow_path = facts::utf8(reader.git(root, &["rev-parse", "--git-path", "shallow"])?)?;
    let path = root.join(shallow_path.trim());
    if path.exists() {
        let shallow =
            std::fs::read_to_string(path).map_err(|e| format!("E_HISTORY_MISSING: {e}"))?;
        for s in shallow.lines() {
            if (dev_ancestors.contains(s) || branch_ancestors.contains(s))
                && !fork_ancestors.contains(s)
            {
                return Err("E_HISTORY_MISSING: truncated ancestry beyond fork".into());
            }
        }
    }
    if !dev_ancestors.contains(fork) || !branch_ancestors.contains(fork) {
        return Err("E_BRANCH_CONTEXT: fork is not a common ancestor".into());
    }
    let behind = dev_ancestors.difference(&fork_ancestors).count();
    let count = w["staleness"]["max_behind_commits"]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or("E_BRANCH_CONTEXT: invalid count threshold")?;
    // The JSON/floating-point threshold is an external input adapter. Once
    // validated and normalized, the admission decision is owned by Verus.
    let max_behind = count as usize;
    if !verified_branch::commit_count_within_limit(behind, max_behind) {
        return Err(format!(
            "E_BRANCH_STALE: behind={behind}; age_ns={}",
            age.whole_nanoseconds()
        ));
    }
    Ok(
        json!({"kind":kind_name,"branch_ref":source,"fork_point":fork,"behind_commits":behind,"age_seconds":age.as_seconds_f64(),"branch_started_at":ctx["branch_started_at"],"observed_at":ctx["observed_at"]}),
    )
}
