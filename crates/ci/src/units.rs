//! One workflow per explicitly registered CI unit, plus a collection workflow.
use super::*;
use chrono_harness::{
    file_identity,
    units::{Scope, Unit},
};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: &str = "chrono-github-units/v1";
pub(super) const MARKER: &str = "# chrono-ci: owned github-units/v1\n";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub collection: super::Config,
    pub units: BTreeMap<String, Workflow>,
    pub gather: Gather,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    pub workflow_path: String,
    pub name: String,
    pub runs_on: String,
    pub timeout_minutes: u32,
    pub bootstrap: Vec<String>,
    pub context_path: String,
    pub artifact_directory: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gather {
    pub program: String,
    pub inherit_environment: Vec<String>,
    pub credential_environment: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
    pub wait_seconds: u64,
    pub poll_seconds: u64,
    pub manifest_path: String,
    pub download_directory: String,
    pub report_path: String,
}

impl Config {
    pub fn workflow(&self, unit: Option<&str>) -> Result<super::Config, String> {
        let mut c = self.collection.clone();
        if let Some(unit) = unit {
            let u = self.units.get(unit).ok_or("unregistered workflow unit")?;
            c.workflow_path = u.workflow_path.clone();
            c.name = u.name.clone();
            c.runs_on = u.runs_on.clone();
            c.timeout_minutes = u.timeout_minutes;
            c.bootstrap = u.bootstrap.clone();
            c.context_path = u.context_path.clone();
            c.artifact_directory = u.artifact_directory.clone();
        }
        Ok(c)
    }

    pub fn scope(&self, unit: Option<&str>) -> Scope {
        match unit {
            Some(unit) => Scope::Unit { unit: unit.into() },
            None => Scope::Collect {
                manifest: self.gather.manifest_path.clone(),
            },
        }
    }
}

pub fn validate(c: &Config) -> Result<(), String> {
    if c.schema != SCHEMA || c.units.is_empty() || c.collection.initial_inventory.is_some() {
        return Err("unit workflows require explicit units and the scoped initial contract".into());
    }
    super::validate(&c.collection)?;
    let mut paths = BTreeSet::from([c.collection.workflow_path.clone()]);
    let mut names = BTreeSet::from([c.collection.name.clone()]);
    let mut contexts = BTreeSet::from([c.collection.context_path.clone()]);
    for (id, unit) in &c.units {
        chrono_harness::units::id(id)?;
        super::validate(&c.workflow(Some(id))?)?;
        if !paths.insert(unit.workflow_path.clone())
            || !names.insert(unit.name.clone())
            || !contexts.insert(unit.context_path.clone())
        {
            return Err("unit workflow path/name/context collision".into());
        }
    }
    chrono_harness::validate_command(&chrono_harness::CommandSpec {
        program: c.gather.program.clone(),
        args: vec![],
        env: BTreeMap::new(),
        timeout_seconds: c.gather.timeout_seconds,
        output_limit_bytes: c.gather.output_limit_bytes,
    })?;
    let inherited: BTreeSet<_> = c.gather.inherit_environment.iter().collect();
    let credentials: BTreeSet<_> = c.gather.credential_environment.iter().collect();
    if inherited.len() != c.gather.inherit_environment.len()
        || credentials.len() != c.gather.credential_environment.len()
        || c.gather
            .inherit_environment
            .iter()
            .chain(c.gather.environment.keys())
            .any(|s| s.is_empty() || s.contains(['=', '\0']))
        || c.gather.environment.values().any(|s| s.contains('\0'))
        || credentials.iter().any(|s| !inherited.contains(s))
        || c.gather.environment.keys().any(|s| credentials.contains(s))
        || ["GH_TOKEN", "GITHUB_TOKEN"].iter().any(|key| {
            inherited.iter().any(|s| s.as_str() == *key)
                && !credentials.iter().any(|s| s.as_str() == *key)
        })
    {
        return Err("invalid transport environment or credential omission declaration".into());
    }
    if c.gather.wait_seconds == 0
        || c.gather.poll_seconds == 0
        || c.gather.poll_seconds > c.gather.wait_seconds
        || c.gather.poll_seconds > 60
    {
        return Err("invalid bounded unit collection wait".into());
    }
    for path in [&c.gather.manifest_path, &c.gather.report_path] {
        chrono_harness::units::artifact_path(path)?;
        if !path.starts_with(&c.collection.artifact_directory) || !contexts.insert(path.clone()) {
            return Err(
                "collection output collision or outside uploaded artifact directory".into(),
            );
        }
    }
    if !c.gather.download_directory.ends_with('/') {
        return Err("download_directory must end /".into());
    }
    chrono_harness::units::artifact_path(c.gather.download_directory.trim_end_matches('/'))?;
    if !c
        .gather
        .download_directory
        .starts_with(&c.collection.artifact_directory)
        || contexts
            .iter()
            .any(|p| p.starts_with(&c.gather.download_directory))
    {
        return Err("download directory overlaps context/collection outputs".into());
    }
    Ok(())
}

pub(super) fn profile(
    root: &Path,
    c: &Config,
) -> Result<(chrono_harness::CheckConfig, BTreeMap<String, Unit>), String> {
    let profile =
        chrono_harness::load_config(&no_symlink_parents(root, &c.collection.check_config)?)?;
    if profile.schema != chrono_harness::units::PROFILE {
        return Err("unit provider requires chrono-ci-check/v3".into());
    }
    let units: BTreeMap<String, Unit> =
        serde_json::from_value(profile.policy["units"].clone()).map_err(|e| e.to_string())?;
    if units.keys().collect::<Vec<_>>() != c.units.keys().collect::<Vec<_>>() {
        return Err("workflow units differ from check profile assignments".into());
    }
    for (id, unit) in &units {
        chrono_harness::units::artifact_path(&unit.report_path)?;
        if !unit
            .report_path
            .starts_with(&c.units[id].artifact_directory)
            || unit.report_path == c.units[id].context_path
        {
            return Err("unit report must be uploaded and separate from context".into());
        }
    }
    if !profile
        .report_path
        .starts_with(&c.collection.artifact_directory)
        || [
            &c.collection.context_path,
            &c.gather.manifest_path,
            &c.gather.report_path,
        ]
        .contains(&&profile.report_path)
        || profile
            .report_path
            .starts_with(&c.gather.download_directory)
    {
        return Err("collection report collides with provider outputs".into());
    }
    Ok((profile, units))
}

pub fn render(c: &Config, config_path: &str) -> Result<BTreeMap<String, String>, String> {
    validate(c)?;
    relative_path(config_path)?;
    if !config_path.starts_with(".chrono-harness/ci/") {
        return Err("unit source must belong to host CI configuration".into());
    }
    let mut outputs = BTreeMap::new();
    for unit in c
        .units
        .keys()
        .map(|s| Some(s.as_str()))
        .chain(std::iter::once(None))
    {
        let w = c.workflow(unit)?;
        let scope = c.scope(unit);
        let prepare = unit
            .map(|u| format!(" --unit {}", shell(u)))
            .unwrap_or_default();
        let pre_check = if unit.is_none() {
            format!(
                "      - name: Gather independent unit reports\n        shell: bash\n        env:\n          GH_TOKEN: ${{{{ github.token }}}}\n        run: |\n          {} gather --host-root . --config {} --repository \"$GITHUB_REPOSITORY\"\n",
                shell(&w.generator),
                shell(config_path)
            )
        } else {
            String::new()
        };
        let artifact = unit
            .map(|u| format!("chrono-unit-{u}"))
            .unwrap_or("chrono-collection".into());
        outputs.insert(
            w.workflow_path.clone(),
            super::render_extended(
                &w,
                config_path,
                Some(&scope),
                &prepare,
                &pre_check,
                &artifact,
                unit.is_none(),
            )?
            .replacen(super::MARKER, MARKER, 1),
        );
    }
    Ok(outputs)
}

pub fn generate(root: &Path, path: &str, c: &Config, verify: bool) -> Result<bool, String> {
    profile(root, c)?;
    let outputs = render(c, path)?;
    let mut writes = vec![];
    for (path, bytes) in &outputs {
        if !output_preflight(root, path, bytes, MARKER)? {
            if verify {
                return Err(format!("generated unit workflow drift: {path}"));
            }
            writes.push((path, bytes));
        }
    }
    // Preflight every output before modifying any of them; I/O failure is reported, never atomicity claimed.
    for (path, bytes) in &writes {
        write_file(root, path, bytes.as_bytes())?;
    }
    Ok(!writes.is_empty())
}

pub fn init(root: &Path, incoming: Config) -> Result<bool, String> {
    let path = ".chrono-harness/ci/units.json";
    let existing = no_symlink_parents(root, path)?.exists();
    let c: Config = if existing {
        decode(&fs::read(root.join(path)).map_err(|e| e.to_string())?)?
    } else {
        incoming
    };
    profile(root, &c)?;
    let outputs = render(&c, path)?;
    for (path, bytes) in &outputs {
        output_preflight(root, path, bytes, MARKER)?;
    }
    if !existing {
        write_file(
            root,
            path,
            &serde_json::to_vec_pretty(&c).map_err(|e| e.to_string())?,
        )?;
    }
    Ok(generate(root, path, &c, false)? || !existing)
}

pub fn prepare(
    root: &Path,
    path: &str,
    c: &Config,
    unit: Option<&str>,
    event: &str,
    payload: &Value,
    revision: &str,
) -> Result<Value, String> {
    generate(root, path, c, true)?;
    let (profile, _) = profile(root, c)?;
    let w = c.workflow(unit)?;
    let scope = c.scope(unit);
    let mut context = super::prepare(root, &w, event, payload, revision)?;
    let argv = context["canonical_argv"]
        .as_array_mut()
        .ok_or("missing canonical command")?;
    if w.schema == "chrono-github-ci/v4" {
        match &scope {
            Scope::Unit { unit } => argv.extend([json!("--unit"), json!(unit)]),
            Scope::Collect { .. } => argv.push(json!("--collect")),
        }
    } else {
        argv.extend(scope.argv().into_iter().map(Value::String));
    }
    context["scope"] = serde_json::to_value(scope).map_err(|e| e.to_string())?;
    context["provider_sha256"] = json!(file_identity(&root.join(path))?.0);
    context["check_config_sha256"] = json!(file_identity(&root.join(&w.check_config))?.0);
    let candidate = context["candidate"].as_str().ok_or("candidate missing")?;
    let facts = event_git::EventGit::new(root, &w, candidate)?;
    for input in [path, w.check_config.as_str(), w.workflow_path.as_str()] {
        if facts
            .git(&["show", &format!("{candidate}:{input}")])?
            .as_bytes()
            != fs::read(root.join(input)).map_err(|e| e.to_string())?
        {
            return Err(format!(
                "unit provider input differs from fixed candidate: {input}"
            ));
        }
    }
    let judge = chrono_harness::resolve_program(root, &profile.judge.program, None)?;
    context["executables"] = json!({"runner_sha256":file_identity(&root.join(&w.runner))?.0,"judge_sha256":file_identity(&judge)?.0});
    Ok(context)
}
