//! One workflow per explicitly registered CI unit, plus a collection workflow.
use super::*;
use chrono_harness::{
    file_identity,
    units::{Scope, Unit},
};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: &str = "chrono-github-units/v1";
pub const FULL_SCHEMA: &str = "chrono-github-units/v2";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullContexts {
    pub collection: String,
    pub units: BTreeMap<String, String>,
}

fn full_contexts_present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<FullContexts>, D::Error> {
    FullContexts::deserialize(d).map(Some)
}
pub(super) const MARKER: &str = "# chrono-ci: owned github-units/v1\n";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "full_contexts_present"
    )]
    pub full_contexts: Option<FullContexts>,
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
    if !matches!(c.schema.as_str(), SCHEMA | FULL_SCHEMA)
        || c.units.is_empty()
        || c.collection.initial_inventory.is_some()
    {
        return Err("unit workflows require explicit units and the scoped initial contract".into());
    }
    match (&c.full_contexts, c.schema.as_str()) {
        (None, SCHEMA) => {}
        (Some(contexts), FULL_SCHEMA)
            if c.collection.schema == "chrono-github-ci/v4"
                && contexts.units.keys().collect::<Vec<_>>()
                    == c.units.keys().collect::<Vec<_>>() =>
        {
            let mut inputs = vec![(&contexts.collection, c.workflow(None)?)];
            for (id, path) in &contexts.units {
                inputs.push((path, c.workflow(Some(id))?));
            }
            for (path, workflow) in inputs {
                chrono_harness::units::artifact_path(path)?;
                if !Path::new(path).starts_with(&workflow.artifact_directory)
                    || chrono_harness::units::overlap(path, &workflow.context_path)
                {
                    return Err("full context must be a separate declared upload input".into());
                }
            }
        }
        _ => {
            return Err(
                "v1 units are scoped; v2 requires exact full context mappings and short workflows"
                    .into(),
            );
        }
    }
    if let Some(contexts) = &c.full_contexts {
        let outputs: Vec<_> = std::iter::once(&c.collection.context_path)
            .chain(c.units.values().map(|u| &u.context_path))
            .chain([
                &c.gather.manifest_path,
                &c.gather.report_path,
                &c.gather.download_directory,
            ])
            .collect();
        for input in std::iter::once(&contexts.collection).chain(contexts.units.values()) {
            if outputs
                .iter()
                .any(|output| chrono_harness::units::overlap(input, output))
            {
                return Err("full context overlaps provider publication".into());
            }
        }
        let mut roots = vec![&c.collection.artifact_directory];
        for unit in c.units.values() {
            if roots
                .iter()
                .any(|root| chrono_harness::units::overlap(&unit.artifact_directory, root))
            {
                return Err("full units require separate upload ownership".into());
            }
            roots.push(&unit.artifact_directory);
        }
    }
    super::validate_policy(&c.collection, c.schema == FULL_SCHEMA)?;
    let mut paths = BTreeSet::from([c.collection.workflow_path.clone()]);
    let mut names = BTreeSet::from([c.collection.name.clone()]);
    let mut contexts = BTreeSet::from([c.collection.context_path.clone()]);
    for (id, unit) in &c.units {
        chrono_harness::units::id(id)?;
        super::validate_policy(&c.workflow(Some(id))?, c.schema == FULL_SCHEMA)?;
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

pub(super) struct Profile {
    pub units: BTreeMap<String, Unit>,
    pub report_path: String,
    pub judge: Option<chrono_harness::CommandSpec>,
}
pub(super) fn profile(
    root: &Path,
    c: &Config,
) -> Result<(Profile, BTreeMap<String, Unit>), String> {
    let raw = chrono_harness::load_selected_config(root, &c.collection.check_config)?;
    let profile = if c.schema == FULL_SCHEMA {
        let block = chrono_harness::units::full_execution_units(&raw)?
            .ok_or("full provider requires registered full units")?;
        Profile {
            units: serde_json::from_value(block["units"].clone()).map_err(|e| e.to_string())?,
            report_path: block["report_path"]
                .as_str()
                .ok_or("full collection report path")?
                .into(),
            judge: None,
        }
    } else {
        let profile =
            chrono_harness::load_config(&no_symlink_parents(root, &c.collection.check_config)?)?;
        if profile.schema != chrono_harness::units::PROFILE {
            return Err("v1 unit provider requires chrono-ci-check/v3".into());
        }
        Profile {
            units: serde_json::from_value(profile.policy["units"].clone())
                .map_err(|e| e.to_string())?,
            report_path: profile.report_path,
            judge: Some(profile.judge),
        }
    };
    let units = profile.units.clone();
    if units.keys().collect::<Vec<_>>() != c.units.keys().collect::<Vec<_>>() {
        return Err("workflow units differ from check profile assignments".into());
    }
    for (id, unit) in &units {
        chrono_harness::units::artifact_path(&unit.report_path)?;
        if !Path::new(&unit.report_path).starts_with(&c.units[id].artifact_directory)
            || chrono_harness::units::overlap(&unit.report_path, &c.units[id].context_path)
        {
            return Err("unit report must be uploaded and separate from context".into());
        }
    }
    if !Path::new(&profile.report_path).starts_with(&c.collection.artifact_directory)
        || [
            &c.collection.context_path,
            &c.gather.manifest_path,
            &c.gather.report_path,
            &c.gather.download_directory,
        ]
        .iter()
        .any(|path| chrono_harness::units::overlap(path, &profile.report_path))
    {
        return Err("collection report collides with provider outputs".into());
    }
    if let Some(contexts) = &c.full_contexts {
        for input in std::iter::once(&contexts.collection).chain(contexts.units.values()) {
            if chrono_harness::units::overlap(input, &profile.report_path)
                || units
                    .values()
                    .any(|u| chrono_harness::units::overlap(input, &u.report_path))
            {
                return Err("full context overlaps check report publication".into());
            }
        }
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
                c.schema == FULL_SCHEMA,
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
    prepare_scope(
        root,
        path,
        c,
        unit,
        Some(c.scope(unit)),
        event,
        payload,
        revision,
    )
}
pub(crate) fn prepare_all(
    root: &Path,
    path: &str,
    c: &Config,
    event: &str,
    payload: &Value,
    revision: &str,
) -> Result<Value, String> {
    prepare_scope(root, path, c, None, None, event, payload, revision)
}
#[allow(clippy::too_many_arguments)]
fn prepare_scope(
    root: &Path,
    path: &str,
    c: &Config,
    unit: Option<&str>,
    scope: Option<Scope>,
    event: &str,
    payload: &Value,
    revision: &str,
) -> Result<Value, String> {
    generate(root, path, c, true)?;
    let (profile, _) = profile(root, c)?;
    let w = c.workflow(unit)?;
    let mut context =
        super::prepare_policy(root, &w, event, payload, revision, c.schema == FULL_SCHEMA)?;
    let argv = context["canonical_argv"]
        .as_array_mut()
        .ok_or("missing canonical command")?;
    if let Some(scope) = &scope {
        if w.schema == "chrono-github-ci/v4" {
            match scope {
                Scope::Unit { unit } => argv.extend([json!("--unit"), json!(unit)]),
                Scope::Collect { .. } => argv.push(json!("--collect")),
            }
        } else {
            argv.extend(scope.argv().into_iter().map(Value::String));
        }
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
    let pins = if let Some(judge) = &profile.judge {
        let judge = chrono_harness::resolve_program(root, &judge.program, None)?;
        json!({"runner_sha256":file_identity(&root.join(&w.runner))?.0,"judge_sha256":file_identity(&judge)?.0})
    } else {
        super::full::executable_pins(root, &w.check_config, &w.runner)?
    };
    context["executables"] = pins;
    if let Some(contexts) = &c.full_contexts {
        let path = match unit {
            Some(unit) => &contexts.units[unit],
            None => &contexts.collection,
        };
        let raw = fs::read(no_symlink_parents(root, path)?)
            .map_err(|e| format!("missing registered exact full context {path}: {e}"))?;
        let ctx = super::full::context_input(&raw)?;
        if ctx["base"] != context["base"]
            || ctx["candidate"] != context["candidate"]
            || context["initial"] == true
        {
            return Err("exact full context disagrees with automatic event endpoints".into());
        }
        context["full_context"] = json!({"path":path,"sha256":chrono_harness::sha256(&raw),"semantic_digest":chrono_harness::wire::digest(&ctx)?});
    }
    Ok(context)
}
