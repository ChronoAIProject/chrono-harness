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
            "### CI resource observation\n\nStatus: **{}**. Known completed-step sum: **{} seconds**.\n\nThis snapshot is not billing time, workflow elapsed time, or pure test time. Running, missing and later steps are not measured.\n\n| Category | Known seconds | Unknown steps |\n| --- | ---: | ---: |\n",
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
    for (id, job) in rows {
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
        "metric":"sum-of-completed-step-seconds", "scope":"unique-job-ids-in-observed-run-history",
        "run_id":run,"through_attempt":attempt,"candidate":candidate,
        "categories":categories,"jobs":observations,"issues":issues,
        "unmeasured":["queue-time","billing-minutes","workflow-wall-time","cpu-time","peak-memory","disk-bytes","compile-test-split-within-check","steps-after-snapshot"]
    }).as_object().unwrap().clone());
    report
}
