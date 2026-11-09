//! owner 原始构建／发布资格的候选读取器；发布和租约仍由现有生命周期所有者执行。
use crate::{
    bindings::{Environment, NativePlatform},
    construction::Report,
    profile::{self, Profile},
    source,
    verified_bindings::{self as policy, owner_publication as publication},
};
use chrono_harness::{decode, file_identity, json, retained_artifacts::retention, sha256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

pub const STAGES: &str = ".chrono-harness/bin/owner-stages/";
pub const CURRENT: &str = ".chrono-harness/state/bootstrap-owner-current.json";

pub fn coordinator_root(
    root: &Path,
    profile: &Profile,
    environment: &Environment,
) -> Result<PathBuf, String> {
    let mut command = crate::bindings::command(
        root,
        Path::new(&profile.participation.git),
        &[
            "--no-optional-locks",
            "worktree",
            "list",
            "--porcelain",
            "-z",
        ],
        environment,
    )?;
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "coordinator Git inventory failed: {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let first = output
        .stdout
        .split(|byte| *byte == 0)
        .next()
        .ok_or("coordinator Git inventory is missing")?;
    let path = first
        .strip_prefix(b"worktree ")
        .ok_or("coordinator Git inventory is missing")?;
    use std::os::unix::ffi::OsStrExt;
    let path = PathBuf::from(OsStr::from_bytes(path));
    if !path.is_absolute() {
        return Err("coordinator Git root must be absolute".into());
    }
    Ok(path)
}

pub fn check_shared_install(
    root: &Path,
    environment: &Environment,
    profile: &Profile,
    source: &Path,
    target: &Path,
) -> Result<(), String> {
    if environment
        .get(OsStr::new("CHRONO_WORKTREE_BOOTSTRAP"))
        .map(|value| Path::new(value))
        != Some(root)
    {
        return Ok(());
    }
    let anchor = coordinator_root(root, profile, environment)?;
    let local = profile::path(
        root,
        &profile.participation.program,
        b".chrono-harness/bin/",
    )?;
    if target != local {
        return Err("lifecycle install destination differs from participation".into());
    }
    let current = if target.exists() {
        Some(file_binding(target)?)
    } else {
        None
    };
    if anchor != root {
        let selected = profile::path(
            &anchor,
            &profile.participation.program,
            b".chrono-harness/bin/",
        )?;
        let coordinator = file_binding(&selected)?;
        if current
            .as_ref()
            .is_none_or(|binding| binding.identity != coordinator.identity)
        {
            return Ok(());
        }
    }
    let Some(current) = current else {
        return Err("changed lifecycle executable requires exclusive owner deployment".into());
    };
    if file_binding(source)?.sha256 != current.sha256 {
        return Err("changed lifecycle executable requires exclusive owner deployment".into());
    }
    Ok(())
}

pub struct Reuse {
    pub operations: BTreeMap<String, crate::execution::Operation>,
    pub source: Option<PathBuf>,
}

/// 原操作只沿用当前 adoption 指向的原始构建，保留 PID、退出和两流，不伪造第二次执行。
pub fn reuse_construction(
    root: &Path,
    environment: &Environment,
    native: &NativePlatform,
    profile: &Profile,
    result: &Report,
    observed: &source::SourceObservation,
) -> Result<Reuse, String> {
    let empty = || Reuse {
        operations: BTreeMap::new(),
        source: None,
    };
    if profile.reuse_owner_operations.is_empty()
        || coordinator_root(root, profile, environment)? != root
    {
        return Ok(empty());
    }
    let current = profile::path(root, CURRENT, b".chrono-harness/state/")?;
    if !current.exists() {
        return Ok(empty());
    }
    let current = json(&fs::read(current).map_err(|error| error.to_string())?)?;
    if current.get("source_commit").and_then(Value::as_str) != observed.commit.as_deref() {
        return Err("owner construction must belong to the current candidate".into());
    }
    qualified_publication(
        root,
        environment,
        native,
        &current,
        &profile.participation.program,
    )?;
    let qualified = stage_report(
        root,
        environment,
        native,
        current["stage_result"]
            .as_str()
            .ok_or("owner stage result is missing")?,
        Some(
            current["stage_sha256"]
                .as_str()
                .ok_or("owner stage digest is missing")?,
        ),
        None,
    )?;
    let previous = serde_json::to_value(&qualified.profile).map_err(|error| error.to_string())?;
    let candidate = serde_json::to_value(profile).map_err(|error| error.to_string())?;
    for field in [
        "projects",
        "rust_toolchain",
        "rust_incremental",
        "require_tool_bindings",
        "tool_bindings_registry",
        "host_environment",
        "environment",
    ] {
        if previous.get(field) != candidate.get(field) {
            return Err(format!("owner construction selection differs: {field}"));
        }
    }
    let original = serde_json::to_value(&qualified.report).map_err(|error| error.to_string())?;
    let actual = serde_json::to_value(result).map_err(|error| error.to_string())?;
    for field in [
        "environment",
        "executables",
        "selection",
        "versions",
        "host_environment",
    ] {
        if original.get(field) != actual.get(field) {
            return Err(format!("owner construction observation differs: {field}"));
        }
    }
    let mut rows: BTreeMap<_, _> = qualified
        .report
        .operations
        .into_iter()
        .map(|row| (row.operation.clone(), row))
        .collect();
    let mut reused = BTreeMap::new();
    for name in &profile.reuse_owner_operations {
        let mut row = rows
            .remove(name)
            .ok_or("owner construction is missing a declared operation")?;
        if qualified.profile.tools.get(&row.tool) != profile.tools.get(&row.tool) {
            return Err(format!(
                "owner construction selection differs: tool {}",
                row.tool
            ));
        }
        row.reused_from = Some(crate::execution::StreamOrigin {
            path: current["stage_result"].as_str().unwrap().into(),
            sha256: current["stage_sha256"].as_str().unwrap().into(),
        });
        reused.insert(name.clone(), row);
    }
    let source = profile::path(root, &qualified.stage.staged.path, STAGES.as_bytes())?;
    Ok(Reuse {
        operations: reused,
        source: Some(source),
    })
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct ProducerSource {
    pub commit: String,
    pub tree: String,
}

#[derive(Debug, Serialize)]
pub struct Qualification {
    pub schema: &'static str,
    pub path: String,
    pub sha256: String,
    pub source: ProducerSource,
    pub selected: publication::Binding,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Publication {
    schema: String,
    stage: publication::Stage,
    installed: publication::Binding,
    adoption: String,
    task_completion: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Adoption {
    schema: String,
    source_commit: String,
    stage_result: String,
    stage_sha256: String,
    installed: publication::Binding,
    original_owner: publication::Binding,
    original_protection: Value,
    publication: Publication,
    terminal_handoff: String,
    task_completion: String,
    receipt: Reference,
}

pub struct QualifiedStage {
    pub report: Report,
    pub profile: Profile,
    pub stage: publication::Stage,
    pub raw: Vec<u8>,
}

pub fn file_binding(path: &Path) -> Result<publication::Binding, String> {
    let identity = chrono_harness::ownership::identity(path)?;
    let before = fs::metadata(path).map_err(|error| error.to_string())?;
    let (digest, size) = file_identity(path)?;
    let after = fs::metadata(path).map_err(|error| error.to_string())?;
    if !publication::stable_observation(
        &identity,
        &chrono_harness::ownership::identity(path)?,
        before.len(),
        after.len(),
        before.mtime(),
        after.mtime(),
        before.mtime_nsec(),
        after.mtime_nsec(),
    ) || size != after.len()
    {
        return Err("owner executable changed during observation".into());
    }
    Ok(publication::Binding {
        identity,
        sha256: digest,
        size,
    })
}

pub fn require_capabilities(root: &Path, environment: &Environment) -> Result<(), String> {
    if environment
        .get(OsStr::new("CHRONO_WORKTREE_BOOTSTRAP"))
        .map(|value| Path::new(value))
        != Some(root)
        || !chrono_harness::process_fds::inherited_available()?
    {
        return Err(
            "owner operation requires original ownership descriptors and host binding".into(),
        );
    }
    Ok(())
}

/// 只构造独立 stage，不改选中 owner；独占发布仍由原 Rust 生命周期入口执行。
pub fn stage(
    root: &Path,
    environment: &Environment,
    profile: &Profile,
    previous: &publication::Binding,
    report_directory: &str,
) -> Result<publication::Stage, String> {
    require_capabilities(root, environment)?;
    let declaration = profile
        .owner_stage
        .as_ref()
        .ok_or("owner stage declaration is missing")?;
    let selected = profile::path(root, &declaration.selected, b".chrono-harness/bin/")?;
    if !publication::binding_equal(&file_binding(&selected)?, previous) {
        return Err("selected owner changed during construction".into());
    }
    let source = profile::path(root, &declaration.source, b"")?;
    let before = file_binding(&source)?;
    let parent = profile::path(root, STAGES.trim_end_matches('/'), b".chrono-harness/bin/")?;
    fs::create_dir_all(&parent).map_err(|error| error.to_string())?;
    let directory = retention::unique_path(&parent, "stage-", "")?;
    let publication = retention::create_adopted(
        root,
        "bootstrap-owner-stage",
        &directory,
        true,
        serde_json::json!({"stage":"construction","publication":"not-established"}),
    )?;
    let relative = directory
        .strip_prefix(root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("non UTF-8 owner stage")?;
    if publication.is_some() {
        let inventory =
            retention::Inventory::adopted(root)?.ok_or("owner stage policy disappeared")?;
        inventory.retain_diagnostic_path(relative)?;
        // The live construction report owns this stage before any writer can
        // release it. Current/adopted report references later own its lifetime.
        inventory.dependencies_paths(report_directory, &[relative.into()])?;
    }
    let target = directory.join("chrono-worktree");
    retention::copy_adopted(publication.as_ref(), &source, &target)?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
        .map_err(|error| error.to_string())?;
    let staged = file_binding(&target)?;
    if !publication::binding_equal(&file_binding(&source)?, &before)
        || staged.sha256 != before.sha256
        || staged.size != before.size
    {
        return Err("built owner changed during staging".into());
    }
    Ok(publication::Stage {
        selected: declaration.selected.clone(),
        previous: publication::Binding {
            identity: previous.identity.clone(),
            sha256: previous.sha256.clone(),
            size: previous.size,
        },
        staged: publication::Staged {
            path: target
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_str()
                .ok_or("non UTF-8 owner stage")?
                .into(),
            identity: staged.identity,
            sha256: staged.sha256,
            size: staged.size,
        },
    })
}

pub fn stage_report(
    root: &Path,
    environment: &Environment,
    native: &NativePlatform,
    reference: &str,
    expected_sha: Option<&str>,
    producer_commit: Option<&str>,
) -> Result<QualifiedStage, String> {
    let raw = profile::read(root, reference, b".chrono-harness/state/bootstrap-")?;
    if let Some(expected) = expected_sha {
        if !chrono_harness::verified_identifiers::equal_text(&sha256(&raw), expected) {
            return Err("owner stage original report differs".into());
        }
    }
    let decoded: Report = decode(&raw)?;
    if Path::new(&decoded.host_root) != root {
        return Err("owner stage producer belongs to another host".into());
    }
    let fixed;
    let current;
    let input: Box<dyn Fn(&str) -> Result<Vec<u8>, String> + '_>;
    let observed;
    if let Some(expected) = producer_commit {
        let identity = decoded
            .source
            .as_ref()
            .ok_or("owner stage source is missing")?;
        let commit = identity
            .commit
            .as_deref()
            .ok_or("owner stage commit is missing")?;
        if commit != expected {
            return Err("selected owner stage producer differs".into());
        }
        fixed = source::Committed::new(
            root,
            environment,
            commit,
            identity
                .tree
                .as_deref()
                .ok_or("owner stage tree is missing")?,
        )?;
        observed = fixed.observation();
        input = Box::new(|path| fixed.read(path));
    } else {
        current = source::observe(root, environment);
        observed = current;
        input = Box::new(|path| profile::read(root, path, b".chrono-harness/"));
    }
    let producer = crate::original::Producer::read(&decoded.config, native, &input)?;
    let report = producer.validate(&raw, &observed, &|path| {
        profile::read(root, path, b".chrono-harness/state/")
    })?;
    let stage: publication::Stage = serde_json::from_value(
        report
            .owner_stage
            .clone()
            .ok_or("owner stage record is missing")?,
    )
    .map_err(|error| error.to_string())?;
    let declared = producer.profile.owner_stage.as_ref();
    if !policy::owner_stage_input(
        report.installed.is_empty(),
        declared.is_some(),
        &stage.selected,
        declared.map(|item| item.selected.as_str()).unwrap_or(""),
    ) || !publication::retained_paths(stage.staged.path.as_bytes(), stage.selected.as_bytes())
    {
        return Err("owner stage declaration differs".into());
    }
    let staged = profile::path(root, &stage.staged.path, STAGES.as_bytes())?;
    if !publication::staged_binding_equal(&file_binding(&staged)?, &stage.staged) {
        return Err("retained owner stage changed".into());
    }
    Ok(QualifiedStage {
        report,
        profile: producer.profile,
        stage,
        raw,
    })
}

fn qualification(
    reference: &str,
    raw: &[u8],
    report: &Report,
    selected: publication::Binding,
) -> Result<Qualification, String> {
    let identity = report
        .source
        .as_ref()
        .ok_or("owner producer source is missing")?;
    Ok(Qualification {
        schema: "chrono-bootstrap-selected-original/v1",
        path: reference.into(),
        sha256: sha256(raw),
        source: ProducerSource {
            commit: identity
                .commit
                .as_ref()
                .ok_or("owner producer commit is missing")?
                .clone(),
            tree: identity
                .tree
                .as_ref()
                .ok_or("owner producer tree is missing")?
                .clone(),
        },
        selected,
    })
}

pub fn qualified_publication(
    root: &Path,
    environment: &Environment,
    native: &NativePlatform,
    current: &Value,
    selected: &str,
) -> Result<Qualification, String> {
    let adoption: Adoption =
        serde_json::from_value(current.clone()).map_err(|error| error.to_string())?;
    let raw = profile::read(
        root,
        &adoption.receipt.path,
        b".chrono-harness/state/automatic-cleanup-v2/",
    )?;
    let mut expected = current.clone();
    expected
        .as_object_mut()
        .ok_or("invalid owner adoption record")?
        .remove("receipt");
    if sha256(&raw) != adoption.receipt.sha256
        || json(&raw)? != expected
        || !policy::owner_adoption_header(&adoption.schema)
    {
        return Err("selected owner adoption original differs".into());
    }
    let qualified = stage_report(
        root,
        environment,
        native,
        &adoption.stage_result,
        Some(&adoption.stage_sha256),
        Some(&adoption.source_commit),
    )?;
    let record = &qualified.stage;
    let selected_path = profile::path(root, selected, b".chrono-harness/bin/")?;
    let installed = file_binding(&selected_path)?;
    let parent = Path::new(&record.staged.path)
        .parent()
        .ok_or("missing owner stage directory")?;
    let retained = |name: &str| -> Result<String, String> {
        parent
            .join(name)
            .to_str()
            .map(str::to_owned)
            .ok_or("non UTF-8 owner stage path".into())
    };
    let previous = file_binding(&profile::path(
        root,
        &retained("previous-owner")?,
        STAGES.as_bytes(),
    )?)?;
    let intent: publication::Intent = decode(&profile::read(
        root,
        &retained("publication-intent.json")?,
        STAGES.as_bytes(),
    )?)?;
    let result: Publication = decode(&profile::read(
        root,
        &retained("publication-result.json")?,
        STAGES.as_bytes(),
    )?)?;
    if record.selected != selected
        || !publication::binding_equal(&installed, &adoption.installed)
        || !publication::binding_equal(&adoption.original_owner, &record.previous)
        || !publication::binding_equal(&previous, &record.previous)
        || !publication::valid_intent(&intent, record)
        || !publication::binding_equal(&intent.replacement, &installed)
        || !publication::stage_equal(&adoption.publication.stage, record)
        || !policy::owner_publication_header(
            &adoption.publication.schema,
            &adoption.publication.adoption,
            &adoption.publication.task_completion,
        )
        || !publication::valid_publication(
            &installed,
            &adoption.publication.installed,
            &previous,
            record,
        )
        || serde_json::to_value(&result).map_err(|error| error.to_string())?
            != serde_json::to_value(&adoption.publication).map_err(|error| error.to_string())?
    {
        return Err("selected owner publication originals differ".into());
    }
    if !publication::binding_equal(&file_binding(&selected_path)?, &installed) {
        return Err("selected owner changed during publication qualification".into());
    }
    qualification(
        &adoption.stage_result,
        &qualified.raw,
        &qualified.report,
        installed,
    )
}

pub fn qualified_selected_original(
    root: &Path,
    environment: &Environment,
    native: &NativePlatform,
    reference: &str,
    selected: &str,
) -> Result<Qualification, String> {
    let selected_path = profile::path(root, selected, b".chrono-harness/bin/")?;
    let before = file_binding(&selected_path)?;
    let current = profile::path(root, CURRENT, b".chrono-harness/state/")?;
    if current.exists() {
        let current = json(&fs::read(current).map_err(|error| error.to_string())?)?;
        if current.get("installed")
            == Some(&serde_json::to_value(&before).map_err(|error| error.to_string())?)
        {
            return qualified_publication(root, environment, native, &current, selected);
        }
    }
    let raw = profile::read(root, reference, b".chrono-harness/state/")?;
    let report = crate::original::validate_fixed_execution(root, environment, native, &raw)?;
    let identity = report
        .source
        .as_ref()
        .ok_or("selected owner source is missing")?;
    let fixed = source::Committed::new(
        root,
        environment,
        identity
            .commit
            .as_deref()
            .ok_or("selected owner commit is missing")?,
        identity
            .tree
            .as_deref()
            .ok_or("selected owner tree is missing")?,
    )?;
    let config = Profile::parse(&fixed.read(&report.config)?)?;
    if config
        .report_path
        .as_deref()
        .unwrap_or(".chrono-harness/state/bootstrap.json")
        != reference
        || !policy::unique_method(
            config
                .install
                .iter()
                .filter(|item| item.destination == selected)
                .count(),
        )
    {
        return Err("selected owner original producer destination differs".into());
    }
    let expected_paths = config
        .install
        .iter()
        .map(|item| item.destination.clone())
        .collect();
    let installed_paths = report
        .installed
        .iter()
        .map(|item| item.path.clone())
        .collect();
    if !policy::text_list_equal(&installed_paths, &expected_paths) {
        return Err("selected owner original installed set differs".into());
    }
    for installed in &report.installed {
        let actual = file_binding(&profile::path(
            root,
            &installed.path,
            b".chrono-harness/bin/",
        )?)?;
        if actual.sha256 != installed.sha256 {
            return Err("selected owner original installed bytes differ".into());
        }
    }
    if profile::read(root, reference, b".chrono-harness/state/")? != raw
        || !publication::binding_equal(&file_binding(&selected_path)?, &before)
    {
        return Err("selected owner or its original changed during qualification".into());
    }
    qualification(reference, &raw, &report, before)
}
