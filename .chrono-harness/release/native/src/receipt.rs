//! 发布回执的消费边界：当前结果选原生产者，再核原始字节与实际操作。
use crate::{
    recipe::{Loaded, Scope, Unit},
    verified_recipe::{self as policy, DependencyResult},
};
use chrono_harness::{file_identity, no_symlink_parents};
use chrono_identifiers::valid_digest_text as digest;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_RECEIPT: {message}")
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value[field]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error(format!("missing {field}")))
}

fn decimal(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
}

fn attempt(value: &str) -> Result<u64, String> {
    if !decimal(value) {
        return Err(error("invalid producing attempt"));
    }
    value
        .parse::<u64>()
        .ok()
        .filter(|v| *v > 0)
        .ok_or_else(|| error("invalid producing attempt"))
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Local,
    Github,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Lineage {
    pub kind: Kind,
    pub run_id: String,
    pub attempt: String,
    pub job: String,
}

impl Lineage {
    pub fn validate(&self) -> Result<(), String> {
        attempt(&self.attempt)?;
        if self.run_id.is_empty() || self.job.is_empty() {
            return Err(error("missing original run/job binding"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub job: String,
    pub artifact_id: String,
    pub run_id: String,
    pub attempt: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Dependencies {
    pub units: BTreeMap<String, Selection>,
    pub frontend: Option<Selection>,
}

fn select_row(job: &str, row: &Value, lineage: &Lineage) -> Result<Selection, String> {
    let result = match row["result"].as_str() {
        Some("success") => DependencyResult::Success,
        Some("failure") => DependencyResult::Failure,
        Some("cancelled") => DependencyResult::Cancelled,
        Some("skipped") => DependencyResult::Skipped,
        _ => return Err(error(format!("invalid current dependency result: {job}"))),
    };
    let outputs = &row["outputs"];
    let selection = Selection {
        job: job.into(),
        artifact_id: text(outputs, "artifact_id")?.into(),
        run_id: text(outputs, "run_id")?.into(),
        attempt: text(outputs, "attempt")?.into(),
    };
    if !policy::selected_producer(
        &result,
        &selection.run_id,
        &lineage.run_id,
        attempt(&selection.attempt)?,
        attempt(&lineage.attempt)?,
        &selection.artifact_id,
    ) || (lineage.kind == Kind::Github && !decimal(&selection.artifact_id))
    {
        return Err(error(format!(
            "failed or incompatible current dependency: {job}"
        )));
    }
    Ok(selection)
}

/// 原生 needs 精确等于登记的业务依赖与发布工具；工具选择独立保留。
pub fn select(
    loaded: &Loaded,
    scope: Scope<'_>,
    lineage: &Lineage,
    raw: &[u8],
) -> Result<Dependencies, String> {
    lineage.validate()?;
    let (id, needs) = match scope {
        Scope::Unit(id) => {
            let unit = loaded.unit(id)?;
            (unit.id(), unit.needs())
        }
        Scope::Collect => (
            loaded.recipe.collector.id.as_str(),
            loaded.recipe.collector.needs.as_slice(),
        ),
    };
    let (bindings, frontend): (BTreeMap<String, String>, Option<&str>) =
        if lineage.kind == Kind::Local {
            if lineage.job != id {
                return Err(error("local producing job differs from selected unit"));
            }
            (
                needs.iter().map(|id| (id.clone(), id.clone())).collect(),
                None,
            )
        } else {
            let matching: Vec<_> = loaded
                .recipe
                .native_jobs
                .iter()
                .filter(|(_, b)| b.get(id) == Some(&lineage.job))
                .collect();
            if matching.len() != 1 {
                return Err(error("unregistered or ambiguous native producing job"));
            }
            let (platform, bindings) = matching[0];
            (
                bindings.clone(),
                Some(loaded.recipe.native_frontends[platform].job.as_str()),
            )
        };
    let input = chrono_harness::json(raw)?;
    let rows = input
        .as_object()
        .ok_or_else(|| error("current dependency input must be an object"))?;
    let mut jobs: BTreeSet<_> = needs
        .iter()
        .map(|id| {
            bindings
                .get(id)
                .map(String::as_str)
                .ok_or_else(|| error("missing native dependency binding"))
        })
        .collect::<Result<_, _>>()?;
    if let Some(job) = frontend {
        jobs.insert(job);
    }
    if rows.keys().map(String::as_str).collect::<BTreeSet<_>>() != jobs {
        return Err(error(
            "current dependency membership differs from registered needs",
        ));
    }
    let mut selected = BTreeMap::new();
    for id in needs {
        let job = &bindings[id];
        selected.insert(id.clone(), select_row(job, &rows[job], lineage)?);
    }
    Ok(Dependencies {
        units: selected,
        frontend: frontend
            .map(|job| select_row(job, &rows[job], lineage))
            .transpose()?,
    })
}

/// 只读普通、无链接的已登记原件；先核长度与摘要，再交给调用方读取。
pub fn original(directory: &Path, declaration: &Value) -> Result<PathBuf, String> {
    let path = no_symlink_parents(directory, text(declaration, "path")?)?;
    let metadata = fs::symlink_metadata(&path).map_err(error)?;
    let expected_size = declaration["size"]
        .as_u64()
        .ok_or_else(|| error("missing original size"))?;
    let expected_sha = text(declaration, "sha256")?;
    if !metadata.is_file() || !digest(expected_sha) || metadata.len() != expected_size {
        return Err(error(
            "original file type, length or digest declaration differs",
        ));
    }
    let (sha, size) = file_identity(&path)?;
    if sha != expected_sha || size != expected_size {
        return Err(error("original process/input bytes changed"));
    }
    Ok(path)
}

fn executable(value: &Value) -> Result<(), String> {
    let path = text(value, "path")?;
    let size = value["size"]
        .as_u64()
        .ok_or_else(|| error("invalid executable size"))?;
    let mode = value["mode"]
        .as_u64()
        .ok_or_else(|| error("invalid executable mode"))?;
    if !Path::new(path).is_absolute()
        || size == 0
        || mode > 0o7777
        || mode & 0o111 == 0
        || !digest(text(value, "sha256")?)
    {
        return Err(error("invalid original tool executable identity"));
    }
    Ok(())
}

fn process_list(receipt: &Value) -> Result<&[Value], String> {
    receipt["processes"]
        .as_array()
        .filter(|p| !p.is_empty())
        .map(Vec::as_slice)
        .ok_or_else(|| error("missing original processes"))
}

fn phase<'a>(receipt: &'a Value, name: &str, count: usize) -> Result<Vec<&'a Value>, String> {
    let rows: Vec<_> = process_list(receipt)?
        .iter()
        .filter(|p| p["phase"] == name)
        .collect();
    if rows.len() != count {
        return Err(error(format!(
            "missing or repeated original {name} invocation"
        )));
    }
    Ok(rows)
}

fn argv(process: &Value, expected: &[String]) -> Result<(), String> {
    if process["argv"] != json!(expected) {
        return Err(error("original registered invocation differs"));
    }
    Ok(())
}

fn observed_text(directory: &Path, process: &Value) -> Result<String, String> {
    String::from_utf8(fs::read(original(directory, &process["stdout"])?).map_err(error)?)
        .map_err(error)
}

fn tools<'a>(loaded: &Loaded, unit: &Unit, receipt: &'a Value) -> Result<&'a Value, String> {
    let mut required = BTreeSet::from(["git"]);
    if let Some(operation) = unit.operation() {
        let action = &loaded.actions[operation];
        if receipt["action_tool"] != action.tool {
            return Err(error("selected verification action tool differs"));
        }
        required.insert(&action.tool);
    }
    let requires_rust = policy::requires_rust(&unit.invocation());
    let identities = if requires_rust {
        required.extend(["rustup", "cargo", "rustc"]);
        &receipt["toolchain"]["tools"]
    } else {
        if receipt.get("toolchain").is_some()
            || process_list(receipt)?.iter().any(|p| {
                matches!(
                    p["phase"].as_str(),
                    Some("toolchain-install" | "compiler-version" | "compiler-executable")
                )
            })
        {
            return Err(error("undeclared Rust toolchain execution"));
        }
        &receipt["execution_tools"]
    };
    let observed = identities
        .as_object()
        .ok_or_else(|| error("missing original selected tool evidence"))?;
    if observed.keys().map(String::as_str).collect::<BTreeSet<_>>() != required {
        return Err(error("selected executable membership differs"));
    }
    for identity in observed.values() {
        executable(identity)?;
    }
    if requires_rust && receipt["execution_tools"] != *identities {
        return Err(error("execution/toolchain tool observations differ"));
    }
    Ok(identities)
}

fn verify_toolchain(
    loaded: &Loaded,
    unit: &Unit,
    directory: &Path,
    receipt: &Value,
    identities: &Value,
) -> Result<(), String> {
    if !policy::requires_rust(&unit.invocation()) {
        return Ok(());
    }
    let cfg = &loaded.recipe;
    let value = &receipt["toolchain"];
    if let Some(incremental) = cfg.rust_incremental {
        if receipt["environment"]["CARGO_INCREMENTAL"] != if incremental { "1" } else { "0" } {
            return Err(error(
                "selected incremental environment differs from recipe",
            ));
        }
    }
    if !text(&value["versions"], "rustc")?.starts_with(&format!("rustc {} ", cfg.rust_toolchain)) {
        return Err(error("selected compiler version differs"));
    }
    let mut install = vec![
        text(&identities["rustup"], "path")?.into(),
        "toolchain".into(),
        "install".into(),
        cfg.rust_toolchain.clone(),
        "--profile".into(),
        "minimal".into(),
    ];
    for component in &cfg.rust_components {
        install.extend(["--component".into(), component.clone()]);
    }
    argv(phase(receipt, "toolchain-install", 1)?[0], &install)?;
    let versions = phase(receipt, "compiler-version", 2)?;
    let selectors = phase(receipt, "compiler-executable", 2)?;
    let active = value["active_compilers"]
        .as_object()
        .ok_or_else(|| error("missing original active compilers"))?;
    if active.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != BTreeSet::from(["cargo", "rustc"])
    {
        return Err(error("active compiler membership differs"));
    }
    for (index, name) in ["cargo", "rustc"].iter().enumerate() {
        argv(
            versions[index],
            &[text(&identities[*name], "path")?.into(), "--version".into()],
        )?;
        argv(
            selectors[index],
            &[
                text(&identities["rustup"], "path")?.into(),
                "which".into(),
                "--toolchain".into(),
                cfg.rust_toolchain.clone(),
                (*name).into(),
            ],
        )?;
        executable(&active[*name])?;
        if observed_text(directory, versions[index])?.trim() != text(&value["versions"], name)?
            || observed_text(directory, selectors[index])?.trim() != text(&active[*name], "path")?
        {
            return Err(error(
                "original compiler observation differs from selected receipt",
            ));
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct Verified {
    pub directory: PathBuf,
    pub receipt: Value,
    pub sha256: String,
}

pub struct Imported {
    pub original: Verified,
    pub asset: String,
    pub lineage: Value,
}

/// 所选原回执和 tar 共用一条消费路径；不另造未经回执校验的导入入口。
pub fn import_build(
    loaded: &Loaded,
    root: &Path,
    context: &Value,
    parent: &Lineage,
    unit: &Unit,
    selection: &Selection,
) -> Result<Imported, String> {
    let Unit::Build { asset, .. } = unit else {
        return Err(error("selected executable producer is not a build unit"));
    };
    let verified = read(loaded, root, context, parent, unit, selection)?;
    let declared = &verified.receipt["assets"];
    let members = declared
        .as_object()
        .ok_or_else(|| error("missing build asset identities"))?;
    let source = &loaded.plan.assets[asset];
    if members.len() != 1
        || !members.contains_key(asset)
        || declared[asset]["path"] != *source
        || declared[asset].get("error") != Some(&Value::Null)
    {
        return Err(error("build asset membership/path differs"));
    }
    let identity = &declared[asset];
    let executable = crate::transport::ExecutableIdentity::from_receipt(identity)?;
    let transport = &verified.receipt["transport"];
    if transport["path"] != "executable.tar" || !digest(text(transport, "sha256")?) {
        return Err(error("invalid executable transport declaration"));
    }
    let archive = crate::transport::ArchiveIdentity {
        sha256: text(transport, "sha256")?.into(),
        size: transport["size"]
            .as_u64()
            .ok_or_else(|| error("missing executable archive size"))?,
    };
    let file = no_symlink_parents(&verified.directory, "executable.tar")?;
    crate::transport::import(&file, &archive, &executable, root, source)?;
    let lineage = json!({"receipt_sha256":verified.sha256,"transport":transport,"identity":identity,"selection":selection});
    Ok(Imported {
        original: verified,
        asset: asset.clone(),
        lineage,
    })
}

/// collector 核每项验证实际使用的资产向量，避免改换生产者后沿用旧成功。
pub fn verify_consumption(
    loaded: &Loaded,
    verified: &Verified,
    assets: &Value,
) -> Result<(), String> {
    let receipt = &verified.receipt;
    let id = text(&receipt["unit"], "id")?;
    let unit = loaded.unit(id)?;
    let Unit::Verify { operation, .. } = unit else {
        return Err(error("consumption receipt is not a verifier"));
    };
    let mut expected_assets = serde_json::Map::new();
    let mut expected_selected = serde_json::Map::new();
    for need in unit.needs() {
        let build = loaded.unit(need)?;
        let asset = build
            .asset()
            .ok_or_else(|| error("verifier dependency is not a build"))?;
        let lineage = assets
            .get(asset)
            .ok_or_else(|| error("missing selected build asset"))?;
        expected_assets.insert(asset.to_owned(), lineage.clone());
        expected_selected.insert(need.clone(), lineage["selection"].clone());
    }
    if receipt["assets"] != Value::Object(expected_assets.clone())
        || receipt["selected"] != Value::Object(expected_selected)
    {
        return Err(error(
            "selected binary or producer lineage changed after verification",
        ));
    }
    let observations = receipt["consumer_staging"]["observations"]
        .as_array()
        .ok_or_else(|| error("missing consumer staging observations"))?;
    let phases = ["staged", "before-verification", "after-verification"];
    if observations.len() != phases.len() {
        return Err(error("incomplete before/after staging observations"));
    }
    let mut expected_files = BTreeMap::new();
    for row in &loaded.staging {
        let Some(asset) = expected_assets.get(&row.asset) else {
            continue;
        };
        let identity = &asset["identity"];
        if identity["path"] != row.source {
            return Err(error("selected build source differs from staging"));
        }
        for path in [&row.source, &row.destination] {
            let mut expected = identity.clone();
            expected["path"] = json!(path);
            expected_files.insert(path.as_str(), expected);
        }
    }
    for (index, snapshot) in observations.iter().enumerate() {
        let op = if index == 0 {
            Value::Null
        } else {
            json!(operation)
        };
        if snapshot["phase"] != phases[index]
            || snapshot["matches"] != true
            || snapshot["operation"] != op
        {
            return Err(error("missing successful original staging observation"));
        }
        let files = snapshot["files"]
            .as_array()
            .ok_or_else(|| error("missing staging file vector"))?;
        let paths = files
            .iter()
            .map(|f| text(f, "path"))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if files.len() != expected_files.len() || paths != expected_files.keys().copied().collect()
        {
            return Err(error("verification staging vector membership differs"));
        }
        for file in files {
            let expected = &expected_files[text(file, "path")?];
            if file.get("error") != Some(&Value::Null)
                || ["sha256", "size", "mode"]
                    .iter()
                    .any(|key| file[*key] != expected[*key])
            {
                return Err(error("verification staging identity differs"));
            }
        }
    }
    Ok(())
}

impl Verified {
    /// Git 身份逐回执核验；它不属于编译器指纹。
    pub fn compiler_fingerprint(&self) -> Option<Value> {
        let toolchain = self.receipt.get("toolchain")?;
        let content = |v: &Value| json!({"sha256":v["sha256"],"size":v["size"],"mode":v["mode"]});
        let tools: BTreeMap<_, _> = ["rustup", "cargo", "rustc"]
            .into_iter()
            .map(|n| (n, content(&toolchain["tools"][n])))
            .collect();
        let active: BTreeMap<_, _> = ["cargo", "rustc"]
            .into_iter()
            .map(|n| (n, content(&toolchain["active_compilers"][n])))
            .collect();
        Some(
            json!({"versions":toolchain["versions"],"incremental":self.receipt["environment"]["CARGO_INCREMENTAL"],"tools":tools,"active_compilers":active}),
        )
    }
}

/// parent 是当前消费任务，selection 已由当前 needs 选择；不读漂移 latest。
pub fn read(
    loaded: &Loaded,
    root: &Path,
    context: &Value,
    parent: &Lineage,
    unit: &Unit,
    selection: &Selection,
) -> Result<Verified, String> {
    parent.validate()?;
    if !policy::selected_producer(
        &DependencyResult::Success,
        &selection.run_id,
        &parent.run_id,
        attempt(&selection.attempt)?,
        attempt(&parent.attempt)?,
        &selection.artifact_id,
    ) || (parent.kind == Kind::Github && !decimal(&selection.artifact_id))
    {
        return Err(error("incompatible selected producer"));
    }
    let directory = no_symlink_parents(root, unit.handoff())?;
    let receipt_path = no_symlink_parents(&directory, "receipt.json")?;
    if !fs::symlink_metadata(&receipt_path)
        .map_err(error)?
        .is_file()
    {
        return Err(error("receipt is not a regular file"));
    }
    let bytes = fs::read(&receipt_path).map_err(error)?;
    let sha256 = chrono_harness::sha256(&bytes);
    if parent.kind == Kind::Local && selection.artifact_id != sha256 {
        return Err(error("selected local original receipt digest mismatch"));
    }
    let receipt = chrono_harness::json(&bytes)?;
    if receipt["schema"] != "chrono-release-unit/v1"
        || receipt["status"] != "passed"
        || receipt.get("failure") != Some(&Value::Null)
        || receipt["unit"] != unit.value()
        || receipt["context"] != *context
    {
        return Err(error("incompatible or failed selected receipt"));
    }
    let lineage: Lineage = serde_json::from_value(receipt["lineage"].clone()).map_err(error)?;
    lineage.validate()?;
    if lineage.kind != parent.kind
        || !policy::producer_lineage(
            &lineage.job,
            &selection.job,
            &lineage.run_id,
            &selection.run_id,
            attempt(&lineage.attempt)?,
            attempt(&selection.attempt)?,
        )
    {
        return Err(error("selected artifact lineage mismatch"));
    }
    let started = receipt["started_ns"]
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or_else(|| error("missing original unit start"))?;
    let finished = receipt["finished_ns"]
        .as_u64()
        .ok_or_else(|| error("missing original unit finish"))?;
    if finished < started
        || u128::from(finished - started)
            > u128::from(loaded.recipe.limits.unit_seconds) * 1_000_000_000
    {
        return Err(error("original unit timing exceeds registered bound"));
    }
    for process in process_list(&receipt)? {
        if process["exit_code"].as_i64() != Some(0) {
            return Err(error("selected receipt contains original process failure"));
        }
        for stream in ["stdout", "stderr"] {
            original(&directory, &process[stream])?;
        }
    }
    let input = fs::read(original(&directory, &receipt["dependency_input"])?).map_err(error)?;
    let selected = select(loaded, Scope::Unit(unit.id()), &lineage, &input)?;
    if receipt["selected"] != json!(selected.units)
        || receipt["frontend_selection"] != json!(selected.frontend)
    {
        return Err(error(
            "selected dependencies differ from original current input",
        ));
    }
    let identities = tools(loaded, unit, &receipt)?;
    let git = text(&identities["git"], "path")?;
    for (name, args, expected) in [
        (
            "source-identity",
            vec!["rev-parse", "HEAD"],
            text(context, "source_commit")?,
        ),
        (
            "source-tree",
            vec!["rev-parse", "HEAD^{tree}"],
            text(context, "source_tree")?,
        ),
        (
            "source-final",
            vec!["rev-parse", "HEAD"],
            text(context, "source_commit")?,
        ),
        (
            "tree-final",
            vec!["rev-parse", "HEAD^{tree}"],
            text(context, "source_tree")?,
        ),
    ] {
        let process = phase(&receipt, name, 1)?[0];
        argv(
            process,
            &std::iter::once(git)
                .chain(args)
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        )?;
        if observed_text(&directory, process)?.trim() != expected {
            return Err(error("original source observation differs"));
        }
    }
    for name in ["source-clean", "source-clean-final"] {
        let process = phase(&receipt, name, 1)?[0];
        argv(
            process,
            &[
                git.into(),
                "status".into(),
                "--porcelain".into(),
                "--untracked-files=normal".into(),
            ],
        )?;
        if !fs::read(original(&directory, &process["stdout"])?)
            .map_err(error)?
            .is_empty()
        {
            return Err(error("selected source has no original clean observation"));
        }
    }
    verify_toolchain(loaded, unit, &directory, &receipt, identities)?;
    match unit {
        Unit::Build { manifest, .. } => argv(
            phase(&receipt, "build", 1)?[0],
            &[
                text(&identities["cargo"], "path")?.into(),
                "build".into(),
                "--release".into(),
                "--locked".into(),
                "--manifest-path".into(),
                manifest.clone(),
            ],
        )?,
        Unit::Verify { operation, .. } => {
            let process = phase(&receipt, "verification", 1)?[0];
            let action = &loaded.actions[operation];
            if process["operation"] != *operation {
                return Err(error("original verification operation differs"));
            }
            let expected = std::iter::once(text(&identities[&action.tool], "path")?.to_owned())
                .chain(action.argv.clone())
                .collect::<Vec<_>>();
            argv(process, &expected)?;
        }
    }
    Ok(Verified {
        directory,
        receipt,
        sha256,
    })
}
