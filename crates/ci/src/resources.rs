//! Observations from the existing jobs response, independent of admission verdicts.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Exact step names; unknown names stay visible instead of being inferred.
    pub step_categories: BTreeMap<String, String>,
    /// Optional presentation sink supplied by the explicitly inherited CI environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_environment: Option<String>,
    /// Host-owned comparisons of completed categories within the same observed job.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub comparisons: BTreeMap<String, Comparison>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    pub category: String,
    pub reference_category: String,
    pub factor: f64,
    pub minimum_seconds: u64,
}

impl Config {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.step_categories.is_empty()
            || self.step_categories.iter().any(|(name, category)| {
                name.trim().is_empty()
                    || category.trim().is_empty()
                    || name.chars().any(char::is_control)
                    || category.chars().any(char::is_control)
                    || category == "unclassified"
            })
        {
            return Err("resource step categories require nonempty literal names and categories; unclassified is reserved".into());
        }
        if self.summary_environment.as_ref().is_some_and(|name| {
            name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        }) {
            return Err("invalid resource summary environment name".into());
        }
        let categories: BTreeSet<_> = self.step_categories.values().collect();
        for (id, rule) in &self.comparisons {
            if id.is_empty()
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                || !categories.contains(&rule.category)
                || !categories.contains(&rule.reference_category)
                || rule.category == rule.reference_category
                || !rule.factor.is_finite()
                || rule.factor <= 0.0
            {
                return Err(format!(
                    "invalid resource comparison {id}: declare distinct known categories and a positive finite factor"
                ));
            }
        }
        Ok(())
    }
}

fn text(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '|' => "&#124;".into(),
            '`' => "&#96;".into(),
            c if c.is_control() => " ".into(),
            c => c.to_string(),
        })
        .collect()
}

/// Rendering failure is an observation, never a replacement admission verdict.
pub(crate) fn publish_summary(config: &Config, report: &Value, original: &str) -> Option<Value> {
    use std::io::Write;
    let key = config.summary_environment.as_ref()?;
    let result: Result<(), String> = (|| {
        let path = std::env::var_os(key).ok_or("summary environment not supplied")?;
        if path.is_empty() {
            return Err("summary path empty".into());
        }
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        let number = |v: &Value| {
            v.as_f64()
                .map(|n| format!("{n:.1}"))
                .unwrap_or("unknown".into())
        };
        let mut body = format!(
            "### CI resource observation\n\nStatus: **{}**. Known completed-step sum: **{} seconds**.\n\nThis snapshot is not billing time, workflow elapsed time, or pure test time. The current attempt's API view may include carried results; these seconds are not the cost of rerunning this attempt. Prior-attempt rows are excluded from totals and comparisons and retained in the source response. Running, missing and later steps are not measured.\n\n| Category | Known seconds | Unknown steps |\n| --- | ---: | ---: |\n",
            report["status"].as_str().unwrap_or("unavailable"),
            number(&report["known_step_seconds"])
        );
        if let Some(categories) = report["categories"].as_object() {
            for (category, totals) in categories {
                body.push_str(&format!(
                    "| {} | {} | {} |\n",
                    text(category),
                    number(&totals["known_step_seconds"]),
                    totals["unknown_steps"]
                ));
            }
        }
        if let Some(jobs) = report["jobs"].as_array() {
            body.push_str("\n| Job | Attempt | Known seconds | Unknown steps |\n| --- | ---: | ---: | ---: |\n");
            for job in jobs {
                body.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    text(job["name"].as_str().unwrap_or("unknown")),
                    job["run_attempt"],
                    number(&job["totals"]["known_step_seconds"]),
                    job["totals"]["unknown_steps"]
                ));
            }
        }
        if let Some(rows) = report["comparisons"].as_array() {
            let warnings: Vec<_> = rows
                .iter()
                .filter(|row| row["status"] == "exceeded")
                .collect();
            let unavailable = rows
                .iter()
                .filter(|row| row["status"] == "unavailable")
                .count();
            body.push_str(&format!("\nRegistered cost comparisons: {} warnings; {unavailable} unavailable. These flag measured overhead for review; they do not establish unnecessary work or a failed check.\n", warnings.len()));
            if !warnings.is_empty() {
                body.push_str("\n| Rule | Job / ID / attempt | Category seconds | Reference seconds | Factor |\n| --- | --- | ---: | ---: | ---: |\n");
                for row in warnings {
                    body.push_str(&format!(
                        "| {} | {} / {} / {} | {} | {} | {} |\n",
                        text(row["rule_id"].as_str().unwrap_or("unknown")),
                        text(row["job_name"].as_str().unwrap_or("unknown")),
                        row["job_id"],
                        row["run_attempt"],
                        number(&row["category_seconds"]),
                        number(&row["reference_seconds"]),
                        row["rule"]["factor"]
                    ));
                }
            }
        }
        body.push_str(&format!("\nUnclassified steps: {}. Observation issues: {}.\n\nOriginal report in the collected artifact: `{}`. It identifies each job, step, missing value and source response.\n\n",
            number(&report["unclassified_steps"]), report["issues"].as_array().map(|v|v.len().to_string()).unwrap_or("unknown".into()),text(original)));
        if let Some(reason) = report["reason"].as_str() {
            body.push_str(&format!("Observation unavailable: {}.\n\n", text(reason)));
        }
        file.write_all(body.as_bytes()).map_err(|e| e.to_string())
    })();
    Some(match result {
        Ok(()) => json!({"status":"written","environment":key}),
        Err(error) => json!({"status":"unavailable","environment":key,"reason":error}),
    })
}

fn compare(config: &Config, jobs: &[Value]) -> Vec<Value> {
    let mut evaluations = vec![];
    for job in jobs {
        for (id, rule) in &config.comparisons {
            let mut row = json!({"rule_id":id,"rule":rule,"job_id":job["job_id"],
                "job_name":job["name"],"run_attempt":job["run_attempt"],
                "status":"unavailable","reason":null,"code":null,
                "category_seconds":null,"reference_seconds":null});
            let measure = |category: &str| -> Result<f64, &'static str> {
                let steps: Vec<_> = job["steps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|step| step["category"] == category)
                    .collect();
                if steps.is_empty() {
                    return Err("category-not-observed");
                }
                if steps.iter().any(|step| step["status"] == "unknown") {
                    return Err("category-measurement-incomplete");
                }
                let measured: Vec<_> = steps
                    .iter()
                    .filter_map(|step| step["seconds"].as_f64())
                    .collect();
                if measured.is_empty() {
                    return Err("category-skipped");
                }
                Ok(measured.iter().sum())
            };
            let result = if job["conclusion"] == "skipped" {
                row["status"] = json!("not-applicable");
                Err("job-skipped")
            } else if job["status"] != "completed" {
                Err("job-not-completed")
            } else {
                measure(&rule.category).and_then(|category| {
                    row["category_seconds"] = json!(category);
                    measure(&rule.reference_category).map(|reference| (category, reference))
                })
            };
            match result {
                Ok((category, reference)) => {
                    row["reference_seconds"] = json!(reference);
                    let threshold = reference * rule.factor;
                    if !threshold.is_finite() {
                        row["reason"] = json!("comparison-threshold-overflow");
                    } else if category >= rule.minimum_seconds as f64 && category > threshold {
                        row["status"] = json!("exceeded");
                        row["code"] = json!("W_CI_RESOURCE_COMPARISON");
                    } else {
                        row["status"] = json!("within-threshold");
                    }
                }
                Err(reason) => row["reason"] = json!(reason),
            }
            evaluations.push(row);
        }
    }
    evaluations
}

#[derive(Default, Serialize)]
struct Totals {
    known_step_seconds: Option<f64>,
    measured_steps: usize,
    unknown_steps: usize,
    skipped_steps: usize,
    unclassified_steps: usize,
}

impl Totals {
    fn add(&mut self, step: &Value) {
        if let Some(seconds) = step["seconds"].as_f64() {
            *self.known_step_seconds.get_or_insert(0.0) += seconds;
            self.measured_steps += 1;
        } else if step["status"] == "skipped" {
            self.skipped_steps += 1;
        } else {
            self.unknown_steps += 1;
        }
        if step["category"] == "unclassified" {
            self.unclassified_steps += 1;
        }
    }
}

fn measured_seconds(step: &Value) -> Result<f64, &'static str> {
    if step["status"] != "completed" {
        return Err("step-not-completed");
    }
    if step["conclusion"].as_str().is_none_or(str::is_empty) {
        return Err("step-conclusion-missing");
    }
    let parse = |key| {
        let raw = step[key].as_str().ok_or("timestamp-missing")?;
        let t = OffsetDateTime::parse(raw, &Rfc3339).map_err(|_| "timestamp-invalid")?;
        if t.year() < 1970 {
            return Err("timestamp-unavailable");
        }
        Ok(t)
    };
    let duration = parse("completed_at")? - parse("started_at")?;
    if duration.is_negative() {
        return Err("timestamps-reversed");
    }
    Ok(duration.as_seconds_f64())
}

pub(crate) fn unavailable(reason: &str) -> Value {
    json!({"schema":"chrono-ci-resources/v1","status":"unavailable",
        "metric":"sum-of-completed-step-seconds", "known_step_seconds":null,
        "source":null,"reason":reason})
}

pub(crate) fn observe(
    config: &Config,
    pages: &Value,
    run: u64,
    attempt: u64,
    candidate: &str,
) -> Value {
    let mut issues = vec![];
    let mut rows = BTreeMap::new();
    let mut conflicting = BTreeSet::new();
    let Some(pages) = pages.as_array() else {
        return unavailable("jobs-response-not-pages");
    };
    for (page_index, page) in pages.iter().enumerate() {
        let Some(jobs) = page["jobs"].as_array() else {
            issues.push(json!({"page":page_index,"reason":"jobs-list-missing"}));
            continue;
        };
        for (row_index, job) in jobs.iter().enumerate() {
            let id = job["id"].as_u64().filter(|id| *id != 0);
            if id.is_none()
                || job["run_id"] != run
                || job["head_sha"] != candidate
                || job["run_attempt"]
                    .as_u64()
                    .is_none_or(|a| a == 0 || a > attempt)
            {
                issues.push(
                    json!({"page":page_index,"row":row_index,"reason":"job-identity-invalid"}),
                );
                continue;
            }
            let id = id.unwrap();
            if rows.insert(id, job).is_some_and(|prior| prior != job) {
                conflicting.insert(id);
            }
        }
    }
    for id in conflicting {
        rows.remove(&id);
        issues.push(json!({"job_id":id,"reason":"conflicting-job-observations"}));
    }
    let mut totals = Totals::default();
    let mut categories: BTreeMap<String, Totals> = BTreeMap::new();
    let mut observations = vec![];
    let mut excluded_prior_attempts = vec![];
    for (id, job) in rows {
        // GitHub can copy completed jobs into a later attempt with new IDs.
        // Keep one API attempt view; IDs do not identify distinct executions.
        if job["run_attempt"] != attempt {
            excluded_prior_attempts.push(json!({"job_id":id,"run_attempt":job["run_attempt"]}));
            continue;
        }
        let mut job_totals = Totals::default();
        let mut steps = vec![];
        if let Some(raw_steps) = job["steps"].as_array() {
            for (index, step) in raw_steps.iter().enumerate() {
                let category = step["name"]
                    .as_str()
                    .and_then(|name| config.step_categories.get(name))
                    .map(String::as_str)
                    .unwrap_or("unclassified");
                let skipped = step["status"] == "completed" && step["conclusion"] == "skipped";
                let measured = measured_seconds(step);
                let observed = json!({"index":index,"number":step["number"],"name":step["name"],
                    "category":category,"started_at":step["started_at"],"completed_at":step["completed_at"],
                    "status":if skipped {"skipped"} else if measured.is_ok() {"measured"} else {"unknown"},
                    "seconds":if skipped {None} else {measured.as_ref().ok().copied()},
                    "reason":if skipped {None} else {measured.err()}});
                totals.add(&observed);
                job_totals.add(&observed);
                categories
                    .entry(category.into())
                    .or_default()
                    .add(&observed);
                steps.push(observed);
            }
            if raw_steps.is_empty() && job["conclusion"] != "skipped" {
                issues.push(json!({"job_id":id,"reason":"steps-empty"}));
            }
        } else if job["conclusion"] != "skipped" {
            issues.push(json!({"job_id":id,"reason":"steps-missing"}));
        }
        if job["status"] != "completed" {
            issues.push(json!({"job_id":id,"reason":"job-not-completed"}));
        }
        observations.push(json!({"job_id":id,"run_attempt":job["run_attempt"],"name":job["name"],
            "status":job["status"],"conclusion":job["conclusion"],"totals":job_totals,"steps":steps}));
    }
    if observations.is_empty() {
        issues.push(json!({"reason":"no-observed-jobs"}));
    }
    let mut report = serde_json::to_value(&totals).unwrap();
    report.as_object_mut().unwrap().extend(json!({"schema":"chrono-ci-resources/v1",
        "status":if issues.is_empty() && totals.unknown_steps == 0 && totals.unclassified_steps == 0 {"observed"} else {"partial"},
        "metric":"sum-of-completed-step-seconds", "scope":"current-attempt-api-job-view",
        "run_id":run,"through_attempt":attempt,"candidate":candidate,
        "categories":categories,"jobs":observations,"issues":issues,
        "excluded_prior_attempts":excluded_prior_attempts,
        "unmeasured":["queue-time","billing-minutes","workflow-wall-time","cpu-time","peak-memory","disk-bytes","compile-test-split-within-check","steps-after-snapshot","distinct-execution-history","current-attempt-rerun-cost"]
    }).as_object().unwrap().clone());
    if !config.comparisons.is_empty() {
        report["comparisons"] = json!(compare(config, report["jobs"].as_array().unwrap()));
    }
    report
}
