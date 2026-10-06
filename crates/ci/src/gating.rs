//! Single-parent GitHub scheduling. The adjudicator owns DELTA obligations;
//! this provider owns their projection and native job/run transport predicate.
use super::*;
use chrono_harness::{file_identity, sha256, wire};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: &str = "chrono-job-gating/v1";
pub(crate) const MARKER: &str = "# chrono-ci: owned github-job-gating/v1\n";
pub const DETECTION_ENV: &str = "CHRONO_CI_DETECTION";
pub const NEEDS_ENV: &str = "CHRONO_CI_NEEDS";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub detector: Detector,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Detector {
    pub runs_on: String,
    pub timeout_minutes: u32,
    pub bootstrap: Vec<String>,
    /// Literal source/registration paths chosen by the host, not unit inference.
    pub sparse_checkout: Vec<String>,
}

pub(crate) fn validate(c: &units::Config, g: &Config) -> Result<(), String> {
    if g.schema != SCHEMA || c.collection.schema != "chrono-github-ci/v4" {
        return Err(
            "job gating requires chrono-job-gating/v1 and the fixed short v4 command".into(),
        );
    }
    if c.collection.name == "Detect registered DELTA"
        || c.units
            .values()
            .any(|u| u.name == "Detect registered DELTA")
    {
        return Err("detector and unit/aggregate job names must be distinct".into());
    }
    let mut detector = c.collection.clone();
    detector.runs_on = g.detector.runs_on.clone();
    detector.timeout_minutes = g.detector.timeout_minutes;
    detector.bootstrap = g.detector.bootstrap.clone();
    super::validate_policy(&detector, c.schema == units::FULL_SCHEMA)?;
    if g.detector.sparse_checkout.is_empty() {
        return Err("detector requires explicit source/registration checkout paths".into());
    }
    let mut paths = BTreeSet::new();
    for path in &g.detector.sparse_checkout {
        relative_path(path.trim_end_matches('/'))?;
        if path.contains(['\n', '\r', '*', '?', '[', ']', '!']) || !paths.insert(path) {
            return Err("detector sparse checkout must contain unique literal paths".into());
        }
    }
    Ok(())
}
pub fn job_id(unit: &str) -> String {
    let encoded: String = unit
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b == b'-' {
                (b as char).to_string()
            } else {
                format!("_{b:02x}")
            }
        })
        .collect();
    format!("unit_{encoded}")
}

fn job(c: &units::Config, path: &str, unit: Option<&str>) -> Result<String, String> {
    let w = c.workflow(unit)?;
    let scope = c.scope(unit);
    let artifact = unit
        .map(|u| format!("chrono-unit-{u}"))
        .unwrap_or("chrono-collection".into());
    let rendered = super::render_extended(
        &w,
        path,
        Some(&scope),
        "",
        "",
        &artifact,
        unit.is_none(),
        c.schema == units::FULL_SCHEMA,
    )?;
    let body = rendered
        .split_once("  check:\n")
        .ok_or("unit job section missing")?
        .1;
    let old_if = "    if: ${{ github.event.deleted != true }}\n";
    let condition = if let Some(unit) = unit {
        format!(
            "    needs: detect\n    if: ${{{{ needs.detect.outputs.{} == 'true' }}}}\n",
            job_id(unit)
        )
    } else {
        let needs: Vec<_> = std::iter::once("detect".into())
            .chain(c.units.keys().map(|u| job_id(u)))
            .collect();
        format!(
            "    needs: {}\n    if: ${{{{ always() }}}}\n",
            serde_json::to_string(&needs).unwrap()
        )
    };
    let mut body = body.replacen(old_if, &condition, 1);
    let extra = if unit.is_none() {
        "          CHRONO_CI_NEEDS: ${{ toJSON(needs) }}\n"
    } else {
        ""
    };
    body = body.replacen("          CHRONO_CHECK_SOURCE: ci\n", &format!("          CHRONO_CHECK_SOURCE: ci\n          CHRONO_CI_DETECTION: ${{{{ needs.detect.outputs.detection }}}}\n{extra}"), 1);
    if let Some(a) = &c.native_adoption {
        let command = format!(
            "{} {} acquire --config {}{}",
            shell(&a.interpreter),
            shell(&a.adapter_path),
            shell(path),
            unit.map(|u| format!(" --unit {}", shell(u)))
                .unwrap_or_default()
        );
        let hook = format!(
            "      - name: Acquire detector original context\n        shell: bash\n        env:\n          CHRONO_WORKFLOW_REVISION: ${{{{ github.workflow_sha }}}}\n          CHRONO_CI_DETECTION: ${{{{ needs.detect.outputs.detection }}}}\n          GH_TOKEN: ${{{{ github.token }}}}\n{extra}        run: |\n          {command}\n"
        );
        body = body.replacen(
            "      - name: Canonical harness check",
            &format!("{hook}      - name: Canonical harness check"),
            1,
        );
    }
    if unit.is_some() {
        let command = format!(
            "{} production --host-root . --config {} --unit {} --github-output \"$GITHUB_OUTPUT\"",
            shell(&c.collection.generator),
            shell(path),
            shell(unit.unwrap())
        );
        let step = format!(
            "      - name: Bind original unit artifact production\n        id: production\n        shell: bash\n        run: |\n          {command}\n"
        );
        body = body.replacen(
            "      - name: Preserve actual check evidence",
            &format!("{step}      - name: Preserve actual check evidence"),
            1,
        );
        body = format!(
            "    outputs:\n      production: ${{{{ steps.production.outputs.production }}}}\n{body}"
        );
    }
    // Scoped checks obtain missing endpoints on demand. Full admission retains
    // its existing history acquisition for the branch/fork judge contract.
    let checkout = if c.schema == units::FULL_SCHEMA {
        "          fetch-depth: 0\n          filter: blob:none\n"
    } else {
        "          fetch-depth: 2\n          filter: blob:none\n"
    };
    body = body.replacen("          fetch-depth: 0\n", checkout, 1);
    body = super::cache::project(
        c.persistent_cache.as_ref(),
        &unit.map(job_id).unwrap_or("aggregate".into()),
        body,
        "Bootstrap registered tools",
        "Canonical harness check",
        None,
        Some("Preserve actual check evidence"),
    )?;
    Ok(format!(
        "  {}:\n{body}",
        unit.map(job_id).unwrap_or("aggregate".into())
    ))
}
pub(crate) fn render(c: &units::Config, path: &str) -> Result<BTreeMap<String, String>, String> {
    let g = c.job_gating.as_ref().ok_or("job gating missing")?;
    let collection = super::render_extended(
        &c.collection,
        path,
        Some(&c.scope(None)),
        "",
        "",
        "chrono-collection",
        true,
        c.schema == units::FULL_SCHEMA,
    )?;
    let prefix = collection
        .split_once("jobs:\n")
        .ok_or("jobs missing")?
        .0
        .replacen(super::MARKER, MARKER, 1);
    let bootstrap = g
        .detector
        .bootstrap
        .iter()
        .map(|s| shell(s))
        .collect::<Vec<_>>()
        .join(" ");
    let sparse = g
        .detector
        .sparse_checkout
        .iter()
        .map(|s| format!("            {s}\n"))
        .collect::<String>();
    let mut outputs = "      detection: ${{ steps.detect.outputs.detection }}\n".to_string();
    for id in c.units.keys() {
        let job = job_id(id);
        outputs.push_str(&format!(
            "      {job}: ${{{{ steps.detect.outputs.{job} }}}}\n"
        ));
    }
    let mut text = format!(
        r#"{prefix}jobs:
  detect:
    name: Detect registered DELTA
    runs-on: {runner}
    timeout-minutes: {timeout}
    outputs:
{outputs}    steps:
      - name: Acquire declared detector source
        uses: {checkout}
        with:
          ref: ${{{{ github.event.pull_request.head.sha || github.event.after || github.sha }}}}
          fetch-depth: {detector_depth}
          filter: blob:none
          sparse-checkout-cone-mode: false
          sparse-checkout: |
{sparse}          persist-credentials: true
      - name: Bootstrap registered detector tools
        shell: bash
        run: |
          {bootstrap}
      - name: Detect required units from fixed Git endpoints
        id: detect
        shell: bash
        env:
          CHRONO_WORKFLOW_REVISION: ${{{{ github.workflow_sha }}}}
        run: |
          {generator} detect --host-root . --config {config} --github-output "$GITHUB_OUTPUT"
"#,
        detector_depth = if c.native_adoption.is_some() { 0 } else { 2 },
        runner = scalar(&g.detector.runs_on),
        timeout = g.detector.timeout_minutes,
        checkout = c.collection.checkout_action,
        generator = shell(&c.collection.generator),
        config = shell(path)
    );
    text = super::cache::project(
        c.persistent_cache.as_ref(),
        "detect",
        text,
        "Bootstrap registered detector tools",
        "Detect required units from fixed Git endpoints",
        Some("detect"),
        None,
    )?;
    if let Some(a) = &c.native_adoption {
        let command = format!(
            "{} {} publish --config {}",
            shell(&a.interpreter),
            shell(&a.adapter_path),
            shell(path)
        );
        text.push_str(&format!("      - name: Publish shared native context\n        shell: bash\n        env:\n          CHRONO_WORKFLOW_REVISION: ${{{{ github.workflow_sha }}}}\n          CHRONO_CI_DETECTION: ${{{{ steps.detect.outputs.detection }}}}\n          GH_TOKEN: ${{{{ github.token }}}}\n        run: |\n          {command}\n      - name: Upload detector original context\n        uses: {}\n        with:\n          name: {}-${{{{ github.run_id }}}}-${{{{ github.run_attempt }}}}\n          path: {}\n          if-no-files-found: error\n{}", c.collection.upload_artifact_action, a.seed_artifact, scalar(&a.seed_directory), super::artifact_hidden_files(c.collection.include_hidden_files)));
    }
    for id in c.units.keys() {
        text.push_str(&job(c, path, Some(id))?);
    }
    text.push_str(&job(c, path, None)?);
    let mut outputs = BTreeMap::from([(c.collection.workflow_path.clone(), text)]);
    if let Some(a) = &c.native_adoption {
        outputs.insert(
            a.adapter_path.clone(),
            include_str!("../../../assets/ci/native.py").into(),
        );
    }
    Ok(outputs)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Detection {
    pub schema: String,
    pub repository: String,
    pub run_id: u64,
    pub run_attempt: u64,
    pub event: String,
    pub event_sha256: String,
    pub workflow_source_revision: String,
    pub workflow_sha256: String,
    pub provider_sha256: String,
    pub check_config_sha256: String,
    pub initial: bool,
    pub base: Option<String>,
    pub candidate: String,
    pub source: String,
    pub changed_paths_count: usize,
    pub required_units: Vec<String>,
    pub selection_sha256: String,
}
fn env(key: &str) -> Result<String, String> {
    std::env::var(key).map_err(|_| format!("missing native job binding {key}"))
}
fn number(key: &str) -> Result<u64, String> {
    env(key)?
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| format!("invalid native job binding {key}"))
}
pub(crate) fn requirements(
    root: &Path,
    c: &units::Config,
    base: Option<String>,
    candidate: String,
    initial: bool,
) -> Result<Value, String> {
    if c.schema == units::FULL_SCHEMA {
        if initial {
            return Err("full scheduling requires DELTA endpoints; full initial inventory is a separate contract".into());
        }
        chrono_judge_ci::full_scheduling_requirements(
            root,
            &c.collection.check_config,
            base.as_deref().ok_or("full scheduling base missing")?,
            &candidate,
        )
    } else {
        chrono_judge_ci::scheduling_requirements(
            root,
            &c.collection.check_config,
            base,
            candidate,
            initial,
        )
    }
}

pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    let mut opts = BTreeMap::new();
    if args.len() != 7 {
        return Err("detect requires fixed host-root, config and github-output".into());
    }
    for pair in args[1..].chunks_exact(2) {
        if !["--host-root", "--config", "--github-output"].contains(&pair[0].as_str())
            || opts.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown/duplicate detector option".into());
        }
    }
    let root = Path::new(
        *opts
            .get("--host-root")
            .ok_or("detector host-root missing")?,
    );
    let path = *opts.get("--config").ok_or("detector config missing")?;
    let output = *opts
        .get("--github-output")
        .ok_or("detector github-output missing")?;
    let Projection::Units(c) = load_projection(&no_symlink_parents(root, path)?)? else {
        return Err("detect requires registered unit provider".into());
    };
    c.job_gating
        .as_ref()
        .ok_or("detect requires explicit job_gating registration")?;
    units::generate(root, path, &c, true)?;
    if env("GITHUB_JOB")? != "detect" {
        return Err("detector job identity mismatch".into());
    }
    let event = env("GITHUB_EVENT_NAME")?;
    let payload =
        chrono_harness::json(&fs::read(env("GITHUB_EVENT_PATH")?).map_err(|e| e.to_string())?)?;
    let revision = env("CHRONO_WORKFLOW_REVISION")?;
    let context = super::prepare_policy(
        root,
        &c.collection,
        &event,
        &payload,
        &revision,
        c.schema == units::FULL_SCHEMA,
    )?;
    let candidate = context["candidate"]
        .as_str()
        .ok_or("detector candidate missing")?;
    let facts = event_git::EventGit::new(root, &c.collection, candidate)?;
    for input in [
        path,
        c.collection.check_config.as_str(),
        c.collection.workflow_path.as_str(),
    ] {
        if facts
            .git(&["show", &format!("{candidate}:{input}")])?
            .as_bytes()
            != fs::read(root.join(input)).map_err(|e| e.to_string())?
        {
            return Err(format!(
                "detector input differs from fixed candidate: {input}"
            ));
        }
    }
    facts.require_commit(&revision)?;
    let source = facts.git(&[
        "show",
        &format!("{revision}:{}", c.collection.workflow_path),
    ])?;
    if source.as_bytes()
        != fs::read(root.join(&c.collection.workflow_path)).map_err(|e| e.to_string())?
    {
        return Err("detector executed a different workflow source".into());
    }
    // Count complete immutable trees before obligation selection.
    // A rename contributes its deleted and added endpoint paths; nothing is truncated.
    let changed_paths = facts.changed_paths(context["base"].as_str(), candidate)?;
    let count = changed_paths.len();
    let selected = requirements(
        root,
        &c,
        context["base"].as_str().map(str::to_owned),
        candidate.into(),
        context["initial"] == true,
    )?;
    if selected["changed_paths"] != json!(changed_paths) {
        return Err("selection owner changed paths disagree with complete Git DELTA".into());
    }
    let required: Vec<String> =
        serde_json::from_value(selected["required_units"].clone()).map_err(|e| e.to_string())?;
    if required.iter().any(|u| !c.units.contains_key(u)) {
        return Err("detector selection not registered by provider".into());
    }
    let detection = Detection {
        schema: "chrono-ci-detection/v1".into(),
        repository: env("GITHUB_REPOSITORY")?,
        run_id: number("GITHUB_RUN_ID")?,
        run_attempt: number("GITHUB_RUN_ATTEMPT")?,
        event,
        event_sha256: wire::digest(&payload)?,
        workflow_source_revision: revision,
        workflow_sha256: sha256(source.as_bytes()),
        provider_sha256: file_identity(&root.join(path))?.0,
        check_config_sha256: file_identity(&root.join(&c.collection.check_config))?.0,
        initial: context["initial"] == true,
        base: context["base"].as_str().map(str::to_owned),
        candidate: candidate.into(),
        source: context["source"]
            .as_str()
            .ok_or("detector source missing")?
            .into(),
        changed_paths_count: count,
        required_units: required,
        selection_sha256: wire::digest(&selected["global_selected"])?,
    };
    let raw = serde_json::to_string(&detection).map_err(|e| e.to_string())?;
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    writeln!(f, "detection={raw}").map_err(|e| e.to_string())?;
    for unit in c.units.keys() {
        writeln!(
            f,
            "{}={}",
            job_id(unit),
            detection.required_units.contains(unit)
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(serde_json::to_string(
        &json!({"detection":detection,"requirements":selected,"event_inputs":context}),
    )
    .map_err(|e| e.to_string())?
        + "\n")
}

pub(crate) fn native_detection(
    root: &Path,
    path: &str,
    c: &units::Config,
    event: &str,
    payload: &Value,
    revision: &str,
) -> Result<Value, String> {
    let d: Detection = decode(env(DETECTION_ENV)?.as_bytes())?;
    let candidate = match event {
        "push" => payload["after"].as_str(),
        "pull_request" => payload["pull_request"]["head"]["sha"].as_str(),
        _ => None,
    }
    .ok_or("gated native event has no candidate")?;
    if d.schema != "chrono-ci-detection/v1"
        || d.repository != env("GITHUB_REPOSITORY")?
        || d.run_id != number("GITHUB_RUN_ID")?
        || d.run_attempt == 0
        || d.run_attempt > number("GITHUB_RUN_ATTEMPT")?
        || d.event != event
        || d.event_sha256 != wire::digest(payload)?
        || d.workflow_source_revision != revision
        || d.candidate != candidate
        || d.provider_sha256 != file_identity(&root.join(path))?.0
        || d.check_config_sha256 != file_identity(&root.join(&c.collection.check_config))?.0
        || d.workflow_sha256 != file_identity(&root.join(&c.collection.workflow_path))?.0
        || d.required_units.iter().any(|u| !c.units.contains_key(u))
        || d.required_units.windows(2).any(|w| w[0] >= w[1])
    {
        return Err("detector repository/run/attempt/event/source/profile binding mismatch".into());
    }
    if !full_oid(&d.candidate)
        || !full_oid(&d.workflow_source_revision)
        || d.base.as_deref().is_some_and(|b| !full_oid(b))
        || d.initial != d.base.is_none()
    {
        return Err("invalid detector endpoints".into());
    }
    Ok(serde_json::to_value(d).map_err(|e| e.to_string())?)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NativeRun {
    pub repository: String,
    pub run_id: u64,
    pub run_attempt: u64,
    pub job: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Production {
    pub schema: String,
    pub native_run: NativeRun,
    pub artifact: String,
    pub context_sha256: String,
    pub report_sha256: String,
}
pub(crate) fn production(args: &[String]) -> Result<String, String> {
    if args.len() != 9 {
        return Err("production requires host-root, config, unit and github-output".into());
    }
    let mut opts = BTreeMap::new();
    for pair in args[1..].chunks_exact(2) {
        if !["--host-root", "--config", "--unit", "--github-output"].contains(&pair[0].as_str())
            || opts.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown/duplicate production option".into());
        }
    }
    let root = Path::new(*opts.get("--host-root").ok_or("production root missing")?);
    let path = *opts.get("--config").ok_or("production config missing")?;
    let unit = *opts.get("--unit").ok_or("production unit missing")?;
    let Projection::Units(c) = load_projection(&no_symlink_parents(root, path)?)? else {
        return Err("production requires units".into());
    };
    if c.job_gating.is_none() {
        return Err("production requires job gating".into());
    }
    let (_, units) = units::profile(root, &c)?;
    let w = c.workflow(Some(unit))?;
    let context_raw =
        fs::read(no_symlink_parents(root, &w.context_path)?).map_err(|e| e.to_string())?;
    let context: Value = decode(&context_raw)?;
    let native_run = native_run(Some(unit))?;
    if context["native_run"] != json!(native_run) {
        return Err("original production identity differs from executing unit".into());
    }
    let report = units.get(unit).ok_or("production unit unregistered")?;
    let production = Production {
        schema: "chrono-ci-production/v1".into(),
        artifact: format!(
            "chrono-unit-{unit}-{}-{}",
            native_run.run_id, native_run.run_attempt
        ),
        native_run,
        context_sha256: sha256(&context_raw),
        report_sha256: file_identity(&no_symlink_parents(root, &report.report_path)?)?.0,
    };
    let raw = serde_json::to_string(&production).map_err(|e| e.to_string())?;
    use std::io::Write;
    let mut output = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(
            *opts
                .get("--github-output")
                .ok_or("production output missing")?,
        )
        .map_err(|e| e.to_string())?;
    writeln!(output, "production={raw}").map_err(|e| e.to_string())?;
    Ok(raw + "\n")
}
pub(crate) fn native_run(unit: Option<&str>) -> Result<NativeRun, String> {
    let job = unit.map(job_id).unwrap_or("aggregate".into());
    if env("GITHUB_JOB")? != job {
        return Err("native unit/aggregate job identity mismatch".into());
    }
    Ok(NativeRun {
        repository: env("GITHUB_REPOSITORY")?,
        run_id: number("GITHUB_RUN_ID")?,
        run_attempt: number("GITHUB_RUN_ATTEMPT")?,
        job,
    })
}

/// Native status is a necessary transport gate. The final check still judges originals.
pub fn validate_statuses(c: &units::Config, d: &Detection, needs: &Value) -> Result<(), String> {
    if needs["detect"]["result"] != "success" {
        return Err("change detection did not succeed".into());
    }
    let observed: Detection = decode(
        needs["detect"]["outputs"]["detection"]
            .as_str()
            .ok_or("detector outputs missing")?
            .as_bytes(),
    )?;
    if observed != *d {
        return Err("aggregate detector outputs disagree with fixed detection".into());
    }
    for unit in c.units.keys() {
        let required = d.required_units.contains(unit);
        let output = if required { "true" } else { "false" };
        let job = job_id(unit);
        if needs["detect"]["outputs"][&job] != output {
            return Err(format!("detector output missing/contradictory for {unit}"));
        }
        let result = needs[&job]["result"]
            .as_str()
            .ok_or_else(|| format!("unit {unit} job result missing"))?;
        if (required && result != "success")
            || (!required && !matches!(result, "skipped" | "success"))
        {
            return Err(format!(
                "unit {unit}: required={required}, native_result={result}"
            ));
        }
    }
    Ok(())
}
