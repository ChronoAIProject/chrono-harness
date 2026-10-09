//! Human console projection of already completed reports; never a verdict producer.

/// Keep identifying text on one line, including when a diagnostic embeds evidence.
pub(crate) fn brief(text: &str) -> String {
    let mut chars = text.chars();
    let mut out: String = chars
        .by_ref()
        .take(240)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if chars.next().is_some() {
        out.push_str("… [text omitted; see original]");
    }
    out
}

// Deserialize only presentation fields from the existing serialized report.
// Serde skips the receipts/graphs, so wrapping accepted evidence in a report
// cannot introduce a second verdict-bearing recursion limit.
#[derive(serde::Deserialize)]
struct Report {
    status: Option<String>,
    response: Option<Response>,
    retained_report: Option<String>,
    report_path: Option<String>,
    transport_failure: Option<String>,
    findings: Option<Vec<crate::wire::Finding>>,
    #[serde(default)]
    judges: Vec<Judge>,
}
#[derive(serde::Deserialize)]
struct Response {
    status: Option<String>,
    #[serde(default)]
    results: Vec<ResultRow>,
}
#[derive(serde::Deserialize)]
struct ResultRow {
    id: String,
    status: String,
    cause: String,
}
#[derive(serde::Deserialize)]
struct Judge {
    id: String,
    state: String,
    transport_failure: Option<String>,
    blocked_by: Option<Vec<String>>,
    response: Option<Response>,
}

pub(crate) fn error_cause(error: &str) -> String {
    // A retained-process publication failure is already the lossless error
    // envelope.  Keep its embedded process receipt intact so callers can
    // recover the original stdout/stderr bytes, exit code, and identity even
    // when the human-facing check path fails before a report is published.
    if error.starts_with("E_PROCESS_EVIDENCE: ") {
        return error.to_owned();
    }
    if let Some(text) = error.strip_prefix("E_GIT_FACTS: ") {
        if let Ok(observation) = crate::json(text.as_bytes()) {
            if let Some(message) = observation["message"].as_str() {
                return format!(
                    "E_GIT_FACTS: {} [observation omitted; see original]",
                    brief(message)
                );
            }
        }
    }
    brief(error)
}

#[derive(serde::Deserialize)]
struct Locator {
    retained_report: Option<serde_json::Value>,
    report_path: Option<serde_json::Value>,
}

/// Preserve the completed check's exit code when only its human projection is
/// malformed. The published report remains the sole locator authority.
pub(crate) fn projection_fallback(code: u8, text: &str, error: &str) -> String {
    let mut out = format!(
        "check projection unavailable (exit {code}): {}\n",
        brief(error)
    );
    match serde_json::from_str::<Locator>(text)
        .ok()
        .and_then(|locator| {
            locator
                .retained_report
                .as_ref()
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    locator
                        .report_path
                        .as_ref()
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
        }) {
        Some(path) => out.push_str(&format!("Original report: {path}\n")),
        None => out.push_str("Original report locator unavailable in serialized result.\n"),
    }
    out
}

pub(crate) fn producer_cause(stderr: &str, stdout: &str) -> String {
    // The local producer's established error envelope puts its identifying
    // error beside process receipts. Read only that field, never the receipts.
    if let Some(text) = stderr.strip_prefix("E_WORKTREE_INPUTS: ") {
        if let Ok(report) = crate::json(text.as_bytes()) {
            if report["schema"] == "chrono-worktree-report/v1" {
                if let Some(error) = report["error"].as_str() {
                    return format!(
                        "E_WORKTREE_INPUTS: {} [producer report omitted; see original]",
                        error_cause(error)
                    );
                }
            }
        }
    }
    brief(if stderr.is_empty() { stdout } else { stderr })
}

pub(crate) fn completed(code: u8, text: &str) -> Result<String, String> {
    let report: Report = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let path = report
        .retained_report
        .as_deref()
        .or(report.report_path.as_deref())
        .ok_or("completed short check has no original report path")?;
    let status = report
        .status
        .as_deref()
        .or_else(|| report.response.as_ref().and_then(|r| r.status.as_deref()))
        .unwrap_or("unavailable");
    let mut out = format!("check: {} (exit {code})\n", brief(status));
    let mut count = 0;
    let mut omitted = 0;
    let mut row = |parts: &[&str]| {
        if count == 12 {
            omitted += 1;
        } else {
            count += 1;
            out.push_str(
                &parts
                    .iter()
                    .map(|s| brief(s))
                    .collect::<Vec<_>>()
                    .join(" | "),
            );
            out.push('\n');
        }
    };
    if let Some(failure) = &report.transport_failure {
        row(&["transport", failure]);
    }
    for judge in &report.judges {
        if let Some(failure) = &judge.transport_failure {
            row(&["transport", &judge.id, failure]);
        }
        if judge.state == "blocked" {
            let mut cause = judge
                .blocked_by
                .as_ref()
                .map(|ids| {
                    ids.iter()
                        .take(3)
                        .map(|id| brief(id))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            if let Some(ids) = judge.blocked_by.as_ref().filter(|ids| ids.len() > 3) {
                cause.push_str(&format!(" [+{} blockers in original]", ids.len() - 3));
            }
            row(&["blocked", &judge.id, &cause]);
        } else if let Some(status @ ("warn" | "fail" | "error")) =
            judge.response.as_ref().and_then(|r| r.status.as_deref())
        {
            row(&["judge", &judge.id, status]);
        }
    }
    if let Some(findings) = &report.findings {
        for finding in findings {
            let location = finding
                .delta_refs
                .first()
                .map(String::as_str)
                .unwrap_or("unavailable");
            let mut more = String::new();
            if finding.delta_refs.len() > 1 {
                more.push_str(&format!(
                    " [+{} refs in original]",
                    finding.delta_refs.len() - 1
                ));
            }
            if !finding.causes.is_empty() {
                more.push_str(&format!(" [{} causes in original]", finding.causes.len()));
            }
            row(&[
                &finding.level,
                &finding.code,
                &finding.message,
                &format!("{}{more}", brief(location)),
            ]);
        }
    }
    if let Some(response) = &report.response {
        for result in &response.results {
            if matches!(result.status.as_str(), "failed" | "blocked") {
                row(&[&result.status, &result.id, &result.cause]);
            }
        }
    }
    if omitted > 0 {
        out.push_str(&format!(
            "{omitted} diagnostic rows omitted; see original report.\n"
        ));
    }
    // This is the exact published immutable path, not a synthesized report.
    out.push_str(&format!("Original report: {path}\n"));
    Ok(out)
}
