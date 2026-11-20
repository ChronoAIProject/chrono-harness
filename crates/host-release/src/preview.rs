//! 自动预发布按固定落地检查及两端登记选择构建和验证。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_identifiers as identifiers;

verus! {
pub fn matching_predecessor(preview: bool, source: &str, base: &str) -> (matches: bool)
    ensures matches == (preview && source@ == base@),
{
    preview && identifiers::equal_text(source, base)
}
}

#[cfg(not(verus_keep_ghost))]
pub use native::{Recovery, select_landing, write_plan};

#[cfg(not(verus_keep_ghost))]
mod native {
    use crate::{
        execution::Environment,
        identity::{Channel, Identity},
        receipt::{self, Kind, Lineage, Selection},
        recipe::{self, Loaded, Unit},
    };
    use chrono_host_bootstrap::source::Committed;
    use chrono_judge_registration::execution::test_bindings;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
        path::Path,
    };

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct Origin {
        pub selection: Selection,
        pub receipt_sha256: String,
        pub release: Identity,
        pub collector: Selection,
    }

    pub struct Recovery {
        pub(crate) origins: BTreeMap<String, Origin>,
        directory: std::path::PathBuf,
        suffix: String,
        pub(crate) started_ns: u64,
    }

    impl Recovery {
        pub(crate) fn from_needs(
            root: &Path,
            loaded: &Loaded,
            lineage: &Lineage,
            raw: &[u8],
            context: &Value,
        ) -> Result<Option<Self>, String> {
            let needs = chrono_harness::json(raw)?;
            let rows = needs.as_object().ok_or("invalid release needs")?;
            if lineage.kind != Kind::Github || !rows.values().any(|row| row["result"] == "skipped")
            {
                return Ok(None);
            }
            let (platform, _) = loaded
                .recipe
                .native_jobs
                .iter()
                .find(|(_, jobs)| jobs.values().any(|job| job == &lineage.job))
                .ok_or("unregistered recovery consumer")?;
            let frontend = &loaded.recipe.native_frontends[platform].job;
            let selection =
                crate::delivery::select(frontend, &needs, &lineage.run_id, &lineage.attempt)?;
            let outputs = &needs[&selection.job]["outputs"];
            let digest = outputs["preview_sha256"]
                .as_str()
                .ok_or("missing frontend selection digest")?;
            let config: crate::delivery::Config = serde_json::from_value(recipe::read_json(
                root,
                ".chrono-harness/release/delivery.json",
            )?)
            .map_err(|e| e.to_string())?;
            let package = config
                .packages
                .get(platform)
                .ok_or("unregistered recovery platform")?;
            let directory = chrono_harness::no_symlink_parents(root, &package.directory)?;
            let bytes = fs::read(chrono_harness::no_symlink_parents(
                &directory,
                "preview.json",
            )?)
            .map_err(|e| e.to_string())?;
            if chrono_harness::sha256(&bytes) != digest {
                return Err("preview decision differs from selected frontend".into());
            }
            let decision = chrono_harness::json(&bytes)?;
            if decision["schema"] != "chrono-release-preview-plan/v1"
                || decision["platform"] != *platform
                || decision["current"] != context["release_identity"]
                || decision["current"]["channel"] != "preview"
            {
                return Err("preview decision does not apply to this execution".into());
            }
            let mut origins: BTreeMap<String, Origin> =
                serde_json::from_value(decision["validated_originals"].clone())
                    .map_err(|e| e.to_string())?;
            origins.retain(|job, _| rows.contains_key(job));
            let started_ns = decision["recovery_started_ns"]
                .as_u64()
                .filter(|v| *v > 0)
                .ok_or("missing original recovery start")?;
            Ok(Some(Self {
                origins,
                directory,
                suffix: package.platform.clone(),
                started_ns,
            }))
        }

        pub(crate) fn verified(
            &self,
            unit: &Unit,
            origin: &Origin,
        ) -> Result<receipt::Verified, String> {
            let directory = chrono_harness::no_symlink_parents(
                &self.directory,
                &format!("originals/{}", unit.id()),
            )?;
            let bytes = fs::read(chrono_harness::no_symlink_parents(
                &directory,
                "receipt.json",
            )?)
            .map_err(|e| e.to_string())?;
            let sha256 = chrono_harness::sha256(&bytes);
            let receipt = chrono_harness::json(&bytes)?;
            if sha256 != origin.receipt_sha256
                || receipt["unit"] != unit.value()
                || receipt["context"]["release_identity"] != serde_json::json!(origin.release)
                || receipt["lineage"]
                    != serde_json::json!({"kind":"github","job":origin.selection.job,"attempt":origin.selection.attempt,"run_id":origin.selection.run_id})
            {
                return Err("restored original differs from the validated preview decision".into());
            }
            Ok(receipt::Verified {
                directory,
                receipt,
                sha256,
            })
        }

        pub(crate) fn import(
            &self,
            root: &Path,
            loaded: &Loaded,
            unit: &Unit,
            origin: &Origin,
        ) -> Result<receipt::Imported, String> {
            let original = self.verified(unit, origin)?;
            let asset = unit.asset().ok_or("expected a restored executable")?;
            let identity = &original.receipt["assets"][asset];
            let destination = &loaded.plan.assets[asset];
            if identity["path"] != *destination {
                return Err("restored member path differs from current release plan".into());
            }
            let expected = crate::transport::ExecutableIdentity::from_receipt(identity)?;
            let path = chrono_harness::no_symlink_parents(
                &self.directory,
                &format!("{asset}-{}", self.suffix),
            )?;
            crate::staging::copy_member(root, &path, destination, &expected)?;
            let lineage = serde_json::json!({"selection":origin.selection,"receipt_sha256":original.sha256,
                "transport":original.receipt["transport"],"identity":identity});
            Ok(receipt::Imported {
                original,
                asset: asset.into(),
                lineage,
            })
        }
    }

    #[derive(Deserialize)]
    struct Original {
        path: String,
        sha256: String,
    }

    impl Original {
        fn read(&self, collection: &Path) -> Result<Value, String> {
            let relative = self
                .path
                .strip_prefix(".chrono-harness/state/")
                .ok_or("landing original outside collection state")?;
            let file = chrono_harness::no_symlink_parents(collection, relative)?;
            let bytes = fs::read(file).map_err(|e| e.to_string())?;
            if chrono_harness::sha256(&bytes) != self.sha256 {
                return Err(format!("landing original digest differs: {}", self.path));
            }
            chrono_harness::json(&bytes)
        }
    }

    #[derive(Deserialize)]
    struct Closure {
        schema: String,
        reached: BTreeSet<String>,
    }

    #[derive(Deserialize)]
    struct Explanation {
        closure: Closure,
    }

    #[derive(Deserialize)]
    struct Evidence {
        base: String,
        candidate: String,
        global_selected: BTreeSet<String>,
        selection_explanation: Explanation,
    }

    impl Evidence {
        fn unaffected(&self) -> bool {
            self.global_selected.is_empty()
                && self
                    .selection_explanation
                    .closure
                    .reached
                    .iter()
                    .all(|node| node.starts_with("file:"))
        }
    }

    #[derive(Deserialize)]
    struct Answer {
        protocol: String,
        status: String,
        request_id: String,
        evidence: Evidence,
    }

    fn landing(
        collection: &Path,
        repository: &str,
        run: u64,
        attempt: u64,
        candidate: &str,
    ) -> Result<Option<Evidence>, String> {
        let context = recipe::read_json(collection, "collection/context.json")?;
        let native = &context["native_run"];
        let detection = &context["detection"];
        let initial = context["initial"]
            .as_bool()
            .ok_or("missing landing inventory mode")?;
        if run == 0
            || attempt == 0
            || context["schema"] != "chrono-ci-inputs/v1"
            || context["event"] != "push"
            || context["scope"]["kind"] != "collect"
            || native["job"] != "aggregate"
            || context["candidate"] != candidate
            || context["event_head"] != candidate
            || native["repository"] != repository
            || native["run_id"] != run
            || native["run_attempt"] != attempt
            || detection["schema"] != "chrono-ci-detection/v2"
            || detection["event"] != "push"
            || detection["repository"] != repository
            || detection["run_id"] != run
            || detection["run_attempt"]
                .as_u64()
                .is_none_or(|n| n == 0 || n > attempt)
            || detection["event_head"] != candidate
            || detection["candidate"] != candidate
            || detection["initial"] != initial
            || detection["base"] != context["base"]
        {
            return Err("landing collection differs from the fixed push".into());
        }
        if initial {
            if context["mode"] != "initial-inventory" || context.get("base") != Some(&Value::Null) {
                return Err("invalid initial landing inventory".into());
            }
            return Ok(None);
        }
        if context["mode"] != "delta" {
            return Err("missing delta collection scope".into());
        }
        let reference = recipe::read_json(collection, "check.json")?;
        if reference["schema"] != "chrono-check-reference/v1" {
            return Err("invalid landing check reference".into());
        }
        let decode = |value: &Value| {
            serde_json::from_value::<Original>(value.clone()).map_err(|e| e.to_string())
        };
        let report = decode(&reference["original"])?.read(collection)?;
        let judge = &report["judge"];
        let request = &report["request"];
        let stdout = decode(&judge["stdout_original"])?;
        if request["base"] != context["base"]
            || request["candidate"] != candidate
            || request["initial"] != false
            || request["scope"] != context["scope"]
            || judge["exit_code"] != 0
            || judge["stdout_complete"] != true
            || judge["stdout_sha256"] != stdout.sha256
        {
            return Err("landing judge input or result differs".into());
        }
        let answer: Answer =
            serde_json::from_value(stdout.read(collection)?).map_err(|e| e.to_string())?;
        if answer.evidence.selection_explanation.closure.schema != "chrono-ci-reachability/v1"
            || answer.protocol != "chrono-ci-judge/v2"
            || answer.status != "passed"
            || request["request_id"] != answer.request_id
            || context["base"] != answer.evidence.base
            || !chrono_identifiers::valid_oid(answer.evidence.base.as_bytes())
            || answer.evidence.candidate != candidate
            || chrono_harness::wire::digest(&answer.evidence.global_selected)?
                != detection["selection_sha256"]
        {
            return Err("landing judge decision differs".into());
        }
        Ok(Some(answer.evidence))
    }

    struct Snapshot {
        loaded: Loaded,
        build_owners: BTreeMap<String, String>,
        test_targets: BTreeMap<String, BTreeSet<String>>,
        jobs: BTreeMap<String, Value>,
        frontend_inputs: BTreeMap<String, Value>,
        frontend_owners: BTreeMap<String, BTreeSet<String>>,
        workflow: Value,
    }

    impl Snapshot {
        fn read(
            root: &Path,
            workflow_config: &str,
            source: &Identity,
            environment: &Environment,
        ) -> Result<Self, String> {
            let fixed = Committed::new(
                root,
                environment,
                &source.source_commit,
                &source.source_tree,
            )?;
            let read = |path: &str| chrono_harness::json(&fixed.read(path)?);
            let loaded = Loaded::load_from(root, read)?;
            let projects = read(&loaded.recipe.projects)?;
            let mut build_owners = BTreeMap::new();
            for project in projects["projects"]
                .as_array()
                .ok_or("missing release projects")?
            {
                if let Some(manifest) = project["manifest"].as_str() {
                    let id = project["id"].as_str().ok_or("missing release project id")?;
                    if build_owners
                        .insert(manifest.to_owned(), format!("project:{id}"))
                        .is_some()
                    {
                        return Err(format!("ambiguous manifest owner: {manifest}"));
                    }
                }
            }
            let mut test_targets = BTreeMap::<String, BTreeSet<String>>::new();
            for (target, bindings) in test_bindings(&projects)? {
                for binding in bindings {
                    test_targets
                        .entry(binding.owner)
                        .or_default()
                        .insert(target.clone());
                }
            }
            let config = read(workflow_config)?;
            let workflow_path = config["workflow_path"]
                .as_str()
                .ok_or("missing registered workflow path")?;
            let mut workflow: Value = serde_yaml_ng::from_slice(&fixed.read(workflow_path)?)
                .map_err(|e| format!("invalid release workflow: {e}"))?;
            let jobs: BTreeMap<String, Value> = serde_json::from_value(workflow["jobs"].take())
                .map_err(|e| format!("invalid release workflow jobs: {e}"))?;
            let mut frontend_inputs = BTreeMap::new();
            let mut frontend_owners = BTreeMap::new();
            for (platform, frontend) in &loaded.recipe.native_frontends {
                let profile = read(&frontend.bootstrap_profile)?;
                let bindings = profile["tool_bindings_registry"]
                    .as_str()
                    .ok_or("missing frontend toolchain registry")?;
                let registrations = read(
                    profile["projects"]
                        .as_str()
                        .ok_or("missing frontend project registry")?,
                )?;
                let methods = chrono_judge_registration::execution::methods(&registrations)?;
                let mut owners = BTreeSet::new();
                let mut operations = Vec::new();
                for name in profile["operations"]
                    .as_array()
                    .ok_or("missing frontend operations")?
                {
                    let [method] = methods
                        .get(name.as_str().ok_or("invalid frontend operation")?)
                        .map(Vec::as_slice)
                        .unwrap_or_default()
                    else {
                        return Err("missing or ambiguous frontend operation".into());
                    };
                    owners.insert(method.owner.clone());
                    operations.push(method);
                }
                frontend_inputs.insert(platform.clone(), serde_json::json!({"profile":profile,"bindings":read(bindings)?,"operations":operations}));
                frontend_owners.insert(platform.clone(), owners);
            }
            Ok(Self {
                loaded,
                frontend_inputs,
                frontend_owners,
                build_owners,
                test_targets,
                jobs,
                workflow,
            })
        }

        fn source_affected(
            &self,
            platform: &str,
            unit: &Unit,
            evidence: &Evidence,
        ) -> Result<bool, String> {
            let reached = &evidence.selection_explanation.closure.reached;
            if !reached.is_disjoint(
                self.frontend_owners
                    .get(platform)
                    .ok_or("missing frontend owners")?,
            ) {
                return Ok(true);
            }
            match unit {
                Unit::Build { manifest, .. } => {
                    let owner = self
                        .build_owners
                        .get(manifest)
                        .ok_or_else(|| format!("unregistered release manifest: {manifest}"))?;
                    Ok(reached.contains(owner))
                }
                Unit::Verify { operation, .. } => {
                    let method = self
                        .loaded
                        .actions
                        .get(operation)
                        .ok_or("missing verification action")?;
                    let targets = self
                        .test_targets
                        .get(&method.owner)
                        .ok_or("missing verification test binding")?;
                    Ok(reached.contains(&method.owner)
                        || !targets.is_disjoint(&evidence.global_selected))
                }
            }
        }

        fn contract(&self, platform: &str, unit: &Unit) -> Result<Value, String> {
            let recipe = &self.loaded.recipe;
            let job = recipe
                .native_jobs
                .get(platform)
                .and_then(|m| m.get(unit.id()))
                .ok_or("missing native unit binding")?;
            let frontend = recipe
                .native_frontends
                .get(platform)
                .ok_or("missing release frontend")?;
            let mut members = BTreeSet::new();
            for id in unit.needs() {
                members.insert(
                    self.loaded
                        .unit(id)?
                        .asset()
                        .ok_or("unit dependency does not produce an asset")?,
                );
            }
            if let Some(asset) = unit.asset() {
                members.insert(asset);
            }
            let staging: Vec<_> = self
                .loaded
                .staging
                .iter()
                .filter(|row| members.contains(row.asset.as_str()))
                .collect();
            Ok(serde_json::json!({
                "unit": unit, "action": unit.operation().and_then(|op| self.loaded.actions.get(op)),
                "declared_consumption": recipe.verification_consumers.get(unit.id()),
                "staging": staging, "tools": recipe.tools, "rust_toolchain": recipe.rust_toolchain,
                "rust_components": recipe.rust_components, "rust_incremental": recipe.rust_incremental,
                "job": self.jobs.get(job).ok_or("unregistered native unit job")?,
                "frontend_inputs": self.frontend_inputs[platform], "frontend": frontend, "frontend_job": self.jobs.get(&frontend.job).ok_or("unregistered frontend job")?,
                "limits": recipe.limits, "workflow": self.workflow,
            }))
        }
    }

    #[derive(Debug, Serialize)]
    struct Plan {
        required: BTreeSet<String>,
        needs_original: BTreeSet<String>,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct InterveningLanding {
        collection: String,
        run_id: u64,
        attempt: u64,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        collection: String,
        platform: String,
        current: Identity,
        previous: Option<Identity>,
        previous_package: Option<PreviousPackage>,
        #[serde(default)]
        intervening: Vec<InterveningLanding>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PreviousPackage {
        selection: Selection,
        run: String,
        artifact: String,
        release: String,
    }

    /// 发布工作流提交和产品源码可以不同。只读固定运行及其实际生成的工作流。
    fn public_members(
        root: &Path,
        config: &crate::delivery::Config,
        previous: &Identity,
        original: &PreviousPackage,
        fixed: &Committed<'_>,
    ) -> Result<BTreeMap<String, Value>, String> {
        let selection = &original.selection;
        let run_id = selection.run_id.parse::<u64>().map_err(|e| e.to_string())?;
        let attempt = selection
            .attempt
            .parse::<u64>()
            .map_err(|e| e.to_string())?;
        let artifact_id = selection
            .artifact_id
            .parse::<u64>()
            .map_err(|e| e.to_string())?;
        if [run_id, attempt, artifact_id].contains(&0) {
            return Err("missing original package identity".into());
        }
        let promotion = config
            .promotion
            .as_ref()
            .ok_or("missing original workflow")?;
        let workflow: Value = chrono_harness::json(&fixed.read(&promotion.workflow_config)?)?;
        let path = workflow["workflow_path"]
            .as_str()
            .ok_or("missing previous workflow path")?;
        let run = recipe::read_json(root, &original.run)?;
        crate::delivery::promotion::verify_run(
            &run,
            &config.publication.repository,
            promotion.workflow_id,
            path,
            run_id,
            run["run_attempt"]
                .as_u64()
                .filter(|n| *n >= attempt)
                .ok_or("previous package attempt exceeds published run")?,
        )?;
        let head = run["head_sha"].as_str().ok_or("missing workflow head")?;
        if run["head_branch"] != config.publication.branch
            || (head != previous.source_commit
                && chrono_harness::facts::blob(root, head, path)? != fixed.read(path)?)
        {
            return Err("previous workflow differs from the released source".into());
        }
        let name = crate::delivery::promotion::artifact_name(
            &workflow,
            &selection.job,
            &selection.run_id,
            &selection.attempt,
        )?;
        let artifact = recipe::read_json(root, &original.artifact)?;
        if !crate::verified_recipe::delivery::promotion_artifact(
            artifact["id"].as_u64(),
            Some(artifact_id),
            artifact["expired"].as_bool(),
            artifact["size_in_bytes"].as_u64(),
            &[(artifact["workflow_run"]["id"].as_u64(), run_id)],
            &[
                (artifact["workflow_run"]["head_sha"].as_str(), head),
                (artifact["name"].as_str(), &name),
            ],
        ) {
            return Err("previous platform artifact differs from the fixed selection".into());
        }
        let release = recipe::read_json(root, &original.release)?;
        if release["draft"] != false
            || release["prerelease"] != true
            || release["tag_name"] != previous.version
            || release["target_commitish"] != previous.source_commit
        {
            return Err("previous public Release identity differs".into());
        }
        let mut members = BTreeMap::new();
        for member in release["assets"]
            .as_array()
            .ok_or("missing published assets")?
        {
            let name = member["name"].as_str().ok_or("missing public asset name")?;
            if members.insert(name.to_owned(), member.clone()).is_some() {
                return Err("duplicate public asset".into());
            }
        }
        Ok(members)
    }

    fn retained_origins(
        root: &Path,
        config: &crate::delivery::Config,
        platform: &str,
        previous: &Identity,
        original: &PreviousPackage,
        jobs: &Plan,
        environment: &Environment,
        downloaded: Option<&crate::delivery::Package>,
    ) -> Result<BTreeMap<String, Value>, String> {
        let workflow = &config
            .promotion
            .as_ref()
            .ok_or("missing registered release workflow")?
            .workflow_config;
        let snapshot = Snapshot::read(root, workflow, previous, environment)?;
        let loaded = &snapshot.loaded;
        let package = downloaded
            .or_else(|| config.packages.get(platform))
            .ok_or("missing registered platform package")?;
        let bindings = loaded
            .recipe
            .native_jobs
            .get(platform)
            .ok_or("missing previous native jobs")?;
        let package_selection = &original.selection;
        if bindings.get(&loaded.recipe.collector.id) != Some(&package_selection.job) {
            return Err("previous collector differs from the registered platform".into());
        }
        let report =
            crate::delivery::verify_package(root, loaded, package, package_selection, previous)?;
        let context = &report["context"];
        let (system, machine) = match (context["system"].as_str(), context["machine"].as_str()) {
            (Some("Linux"), Some(arch)) if package.platform == format!("linux-{arch}") => {
                ("Linux", arch)
            }
            (Some("Darwin"), Some("aarch64")) if package.platform == "macos-aarch64" => {
                ("Darwin", "arm64")
            }
            (Some("Darwin"), Some(arch)) if package.platform == format!("macos-{arch}") => {
                ("Darwin", arch)
            }
            _ => {
                return Err(
                    "previous collector execution platform differs from its package".into(),
                );
            }
        };
        let native = chrono_host_bootstrap::bindings::NativePlatform {
            system: system.into(),
            machine: machine.into(),
        };
        let fixed = Committed::new(
            root,
            environment,
            &previous.source_commit,
            &previous.source_tree,
        )?;
        let frontend = &loaded.recipe.native_frontends[platform];
        let members = public_members(root, config, previous, original, &fixed)?;
        let packaged = report["package"]["assets"]
            .as_object()
            .ok_or("missing previous package assets")?;
        for asset in packaged.values() {
            let name = asset["path"].as_str().ok_or("missing released file name")?;
            let public = members
                .get(name)
                .ok_or_else(|| format!("missing public asset: {name}"))?;
            let hash = asset["sha256"]
                .as_str()
                .ok_or("missing packaged asset digest")?;
            if public["state"] != "uploaded"
                || public["size"] != asset["size"]
                || public["digest"] != format!("sha256:{hash}")
            {
                return Err(format!(
                    "public asset differs from previous collector: {name}"
                ));
            }
        }
        let producer = chrono_host_bootstrap::original::Producer::read(
            &frontend.bootstrap_profile,
            &native,
            &|path| fixed.read(path),
        )?;
        let parent = Lineage {
            kind: Kind::Github,
            job: package_selection.job.clone(),
            run_id: package_selection.run_id.clone(),
            attempt: package_selection.attempt.clone(),
        };
        let retained: BTreeMap<String, Origin> = match report.get("retained_origins") {
            Some(value) => serde_json::from_value(value.clone()).map_err(|e| e.to_string())?,
            None => BTreeMap::new(),
        };
        let retained_selections = retained
            .iter()
            .map(|(job, origin)| (job.clone(), origin.selection.clone()))
            .collect();
        let package_root = chrono_harness::no_symlink_parents(root, &package.directory)?;
        let input = receipt::original(&package_root, &report["dependency_input"])?;
        let selected = receipt::select_retained(
            loaded,
            recipe::Scope::Collect,
            &parent,
            &fs::read(input).map_err(|e| e.to_string())?,
            &retained_selections,
        )?;
        if report["selected"] != serde_json::json!(selected.units) {
            return Err("platform collector selection differs from its original needs".into());
        }
        let mut previous_sources = BTreeMap::new();
        let mut origins = BTreeMap::new();
        for unit in &loaded.recipe.units {
            let job = &bindings[unit.id()];
            if !jobs.needs_original.contains(job) {
                continue;
            }
            let selected: Selection = serde_json::from_value(report["selected"][unit.id()].clone())
                .map_err(|e| format!("invalid previous selection for {job}: {e}"))?;
            if selected.job != *job {
                return Err(format!("previous unit selected another job: {job}"));
            }
            let directory = chrono_harness::no_symlink_parents(
                &package_root,
                &format!("originals/{}", unit.id()),
            )?;
            let inherited = retained.get(job);
            let original_context;
            let inherited_parent;
            let (source_recipe, source_producer, source_context, collector) = if let Some(origin) =
                inherited
            {
                if origin.selection != selected
                    || origin.release.channel != Channel::Preview
                    || origin.collector.job != parent.job
                    || origin.collector.run_id != selected.run_id
                {
                    return Err("previous collector retained a different original".into());
                }
                original_context =
                    recipe::read_json(&directory, "receipt.json")?["context"].clone();
                if original_context["release_identity"] != serde_json::json!(origin.release)
                    || original_context["source_commit"] != origin.release.source_commit
                    || original_context["source_tree"] != origin.release.source_tree
                    || original_context["system"] != context["system"]
                    || original_context["machine"] != context["machine"]
                {
                    return Err("retained producer source or platform differs".into());
                }
                let key = (
                    origin.release.source_commit.clone(),
                    origin.release.source_tree.clone(),
                );
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    previous_sources.entry(key.clone())
                {
                    let fixed = Committed::new(root, environment, &key.0, &key.1)?;
                    let recipe =
                        Loaded::load_from(root, |path| chrono_harness::json(&fixed.read(path)?))?;
                    let producer = chrono_host_bootstrap::original::Producer::read(
                        &recipe
                            .recipe
                            .native_frontends
                            .get(platform)
                            .ok_or("missing original platform frontend")?
                            .bootstrap_profile,
                        &native,
                        &|path| fixed.read(path),
                    )?;
                    entry.insert((recipe, producer));
                }
                let (recipe, producer) = &previous_sources[&key];
                inherited_parent = Lineage {
                    kind: Kind::Github,
                    job: origin.collector.job.clone(),
                    run_id: origin.collector.run_id.clone(),
                    attempt: origin.collector.attempt.clone(),
                };
                (recipe, producer, &original_context, &inherited_parent)
            } else {
                (loaded, &producer, context, &parent)
            };
            let verified = receipt::read_at(
                source_recipe,
                directory,
                source_context,
                collector,
                unit,
                &selected,
                |directory, declared, source, selection| {
                    if declared.job != frontend.job {
                        return Err("previous unit consumed another platform frontend".into());
                    }
                    crate::frontend::validate_producer_metadata(
                        directory,
                        declared,
                        &native,
                        source,
                        selection,
                        source_producer,
                    )
                },
            )?;
            if inherited.is_some_and(|origin| origin.receipt_sha256 != verified.sha256) {
                return Err("retained receipt differs from its original collector".into());
            }
            let declaration = &report["originals"][unit.id()]["receipt.json"];
            if declaration["path"] != "receipt.json" || declaration["sha256"] != verified.sha256 {
                return Err(format!(
                    "original receipt disagrees with previous collector: {job}"
                ));
            }
            if let Some(asset) = unit.asset() {
                let expected = &verified.receipt["assets"][asset];
                let packaged = &report["package"]["assets"][asset];
                let used = &report["assets"][asset];
                if used["receipt_sha256"] != verified.sha256
                    || used["selection"] != serde_json::json!(selected)
                    || used["identity"] != *expected
                    || ["sha256", "size", "mode"]
                        .iter()
                        .any(|k| expected[k] != packaged[k])
                {
                    return Err(format!(
                        "previous platform member differs from its producer: {asset}"
                    ));
                }
            } else {
                receipt::verify_consumption(loaded, &verified, &report["assets"])?;
            }
            origins.insert(
                job.clone(),
                serde_json::json!({
                    "selection": selected, "receipt_sha256": verified.sha256, "release": inherited.map_or(previous, |origin| &origin.release),
                    "collector": inherited.map_or(package_selection, |origin| &origin.collector),
                }),
            );
        }
        if origins.len() != jobs.needs_original.len() {
            return Err("missing original job from previous release".into());
        }
        Ok(origins)
    }

    /// 从已发布的固定标签读取生产运行，并由原晋升读取器核对发布者及两个平台包。
    fn prepare_previous(
        root: &Path,
        output: &Path,
        config: &crate::delivery::Config,
        previous: &Identity,
        environment: &Environment,
        commands: &mut impl crate::publication::Commands,
    ) -> Result<Option<crate::delivery::promotion::Prepared>, String> {
        if previous.channel != Channel::Preview
            || previous.verified_ci.is_none()
            || !crate::verified_recipe::delivery::name(previous.version.as_bytes())
        {
            return Err("expected a fixed preview release".into());
        }
        let registry = config
            .promotion
            .as_ref()
            .ok_or("missing registered preview workflow")?;
        let repository = &config.publication.repository;
        let Some(release) =
            crate::publication::existing_preview(commands, &config.publication, previous)?
        else {
            return Ok(None);
        };
        let mut links = release["body"]
            .as_str()
            .unwrap_or_default()
            .lines()
            .filter_map(|line| line.strip_prefix("Release run: "));
        let Some(link) = links.next() else {
            return Ok(None);
        };
        if links.next().is_some() {
            return Err("ambiguous preview producer link".into());
        }
        let (run, attempt) = link
            .strip_prefix(&format!("https://github.com/{repository}/actions/runs/"))
            .and_then(|link| link.split_once("/attempts/"))
            .ok_or("invalid preview producer link")?;
        let fixed = Committed::new(
            root,
            environment,
            &previous.source_commit,
            &previous.source_tree,
        )?;
        let read = |path: &str| chrono_harness::json(&fixed.read(path)?);
        let loaded = Loaded::load_from(root, read)?;
        let workflow = read(&registry.workflow_config)?;
        crate::delivery::promotion::read_preview_origin(
            root, output, config, &loaded, previous, &workflow, run, attempt, commands,
        )
    }

    fn store_record(
        root: &Path,
        output: &Path,
        name: &str,
        value: &Value,
    ) -> Result<String, String> {
        let file = output.join(name);
        let target = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&file)
            .map_err(|e| e.to_string())?;
        serde_json::to_writer(target, value).map_err(|e| e.to_string())?;
        file.strip_prefix(root)
            .ok()
            .and_then(Path::to_str)
            .map(str::to_owned)
            .ok_or("selection output outside checkout".into())
    }

    fn prior_tag(
        root: &Path,
        base: &str,
        environment: &Environment,
        commands: &mut impl crate::publication::Commands,
    ) -> Result<Option<Identity>, String> {
        let args = ["--no-replace-objects", "tag", "--points-at", base];
        let reply = commands.execute("git", &args.map(str::to_owned))?;
        if reply.exit != 0 {
            return Err("could not enumerate fixed predecessor tags".into());
        }
        let text = std::str::from_utf8(&reply.stdout).map_err(|e| e.to_string())?;
        let suffix = format!(".g{base}");
        let mut versions = text
            .lines()
            .filter(|tag| tag.contains("-dev.") && tag.ends_with(&suffix));
        let Some(version) = versions.next() else {
            return Ok(None);
        };
        if versions.next().is_some() {
            return Ok(None);
        }
        let (core, ids) = version
            .strip_suffix(&suffix)
            .and_then(|v| v.split_once("-dev."))
            .ok_or("invalid preview tag")?;
        let (run, attempt) = ids
            .split_once('.')
            .ok_or("missing landing identity in preview tag")?;
        let tree_reply = commands.execute(
            "git",
            &[
                "--no-replace-objects".into(),
                "rev-parse".into(),
                "--verify".into(),
                format!("{base}^{{tree}}"),
            ],
        )?;
        if tree_reply.exit != 0 {
            return Err("previous source tree unavailable".into());
        }
        let tree = std::str::from_utf8(&tree_reply.stdout)
            .map_err(|e| e.to_string())?
            .trim();
        let fixed = Committed::new(root, environment, base, tree)?;
        let loaded = Loaded::load_from(root, |p| chrono_harness::json(&fixed.read(p)?))?;
        let mut original = environment.clone();
        original.insert("CHRONO_RELEASE_SOURCE".into(), base.into());
        original.insert("CHRONO_RELEASE_CI_RUN_ID".into(), run.into());
        original.insert("CHRONO_RELEASE_CI_ATTEMPT".into(), attempt.into());
        let id = crate::identity::resolve(&loaded.plan.version, base, tree, &original)?;
        if id.channel != Channel::Preview || id.version != version || core.is_empty() {
            return Err("previous tag differs from its original release recipe".into());
        }
        Ok(Some(id))
    }

    /// 逐个沿落地原件给出的 base 寻址，只跨越没有发布使用方的提交。
    fn previous_release(
        root: &Path,
        output: &Path,
        config: &crate::publication::Config,
        evidence: Option<&Evidence>,
        environment: &Environment,
        commands: &mut impl crate::publication::Commands,
    ) -> Result<(Option<Identity>, Vec<InterveningLanding>), String> {
        let mut links = Vec::new();
        let mut visited = BTreeSet::new();
        let mut candidate = evidence.map(|impact| impact.base.clone());
        while let Some(head) = candidate {
            if !visited.insert(head.clone()) {
                return Err("repeated intermediate landing".into());
            }
            if let Some(previous) = prior_tag(root, &head, environment, commands)? {
                return Ok((Some(previous), links));
            }
            let endpoint = format!(
                "repos/{}/actions/workflows/{}/runs?event=push&head_sha={head}&per_page=2",
                config.repository, config.workflow_id
            );
            let list = crate::publication::read_api(commands, endpoint)?;
            let runs = list["workflow_runs"]
                .as_array()
                .ok_or("missing landing runs")?;
            if list["total_count"] != 1 || runs.len() != 1 {
                break;
            }
            let run = &runs[0];
            if run["status"] != "completed" || run["conclusion"] != "success" {
                break;
            }
            let run_id = run["id"].as_u64().ok_or("missing landing run id")?;
            let attempt = run["run_attempt"]
                .as_u64()
                .ok_or("missing landing attempt")?;
            crate::publication::verify_landing_run(config, run, &head, run_id, attempt)?;
            let directory = output.join(format!("landing-{run_id}-{attempt}"));
            let name = format!("chrono-collection-{run_id}-{attempt}");
            if crate::delivery::promotion::download(
                &config.repository,
                run,
                &name,
                &directory,
                None,
                commands,
            )?
            .is_none()
            {
                break;
            }
            let Some(bridge) = landing(&directory, &config.repository, run_id, attempt, &head)?
            else {
                break;
            };
            if !bridge.unaffected() {
                break;
            }
            links.push(InterveningLanding {
                collection: directory
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_str()
                    .ok_or("invalid landing path")?
                    .into(),
                run_id,
                attempt,
            });
            candidate = Some(bridge.base);
        }
        Ok((None, Vec::new()))
    }

    pub fn select_landing(
        root: &Path,
        output: &Path,
        config: &crate::delivery::Config,
        collection: &str,
        current: &Identity,
        environment: &Environment,
        commands: &mut impl crate::publication::Commands,
    ) -> Result<(), String> {
        let ci = current
            .verified_ci
            .as_ref()
            .ok_or("missing verified landing")?;
        let evidence = landing(
            &chrono_harness::no_symlink_parents(root, collection)?,
            &config.publication.repository,
            ci.run_id,
            ci.attempt,
            &current.source_commit,
        )?;
        let started_ns: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
            .try_into()
            .map_err(|_| "invalid recovery start")?;
        let (prior, intervening) = previous_release(
            root,
            output,
            &config.publication,
            evidence.as_ref(),
            environment,
            commands,
        )?;
        let mut request = Request {
            collection: collection.into(),
            platform: String::new(),
            current: current.clone(),
            previous: prior,
            previous_package: None,
            intervening,
        };
        let mut decisions = BTreeMap::new();
        for platform in config.packages.keys() {
            request.platform = platform.clone();
            decisions.insert(platform, plan(root, config, &request, environment)?);
        }
        let origin = match &request.previous {
            Some(old) if decisions.values().any(|p| !p.needs_original.is_empty()) => {
                prepare_previous(root, output, config, old, environment, commands)?
            }
            _ => None,
        };
        let mut skipped = BTreeSet::new();
        for (platform, decision) in decisions {
            let mut origins = BTreeMap::new();
            if let Some(prepared) = &origin {
                let old = &prepared.identity;
                let observation = &prepared.observation;
                let metadata = PreviousPackage {
                    selection: prepared.selections[platform].clone(),
                    run: store_record(
                        root,
                        output,
                        &format!("{platform}-run.json"),
                        &observation["run"],
                    )?,
                    release: store_record(
                        root,
                        output,
                        &format!("{platform}-public.json"),
                        &observation["verified_publication"]["release"],
                    )?,
                    artifact: store_record(
                        root,
                        output,
                        &format!("{platform}-artifact.json"),
                        &observation["package_artifacts"][platform],
                    )?,
                };
                origins = retained_origins(
                    root,
                    config,
                    platform,
                    old,
                    &metadata,
                    &decision,
                    environment,
                    Some(&prepared.packages[platform]),
                )?;
            }
            skipped.extend(origins.keys().cloned());
            let report = serde_json::json!({"schema":"chrono-release-preview-plan/v1","current":current,"previous":request.previous,
                "platform":platform,"jobs":decision,"validated_originals":origins});
            let mut report = report;
            report["recovery_started_ns"] = serde_json::json!(started_ns);
            if !origins.is_empty() {
                let prepared = origin.as_ref().ok_or("missing recovered package")?;
                let directory = chrono_harness::no_symlink_parents(
                    root,
                    &prepared.packages[platform].directory,
                )?;
                store_record(root, &directory, "preview.json", &report)?;
            }
            store_record(root, output, &format!("{platform}.json"), &report)?;
        }
        store_record(root, output, "skipped.json", &serde_json::json!(skipped))?;
        Ok(())
    }

    pub fn write_plan(
        root: &Path,
        output: &Path,
        config_path: &str,
        request_path: &str,
        environment: &Environment,
    ) -> Result<(), String> {
        use std::io::Write;
        let config = serde_json::from_value(recipe::read_json(root, config_path)?)
            .map_err(|error| error.to_string())?;
        let request: Request = serde_json::from_value(recipe::read_json(root, request_path)?)
            .map_err(|error| error.to_string())?;
        let result = plan(root, &config, &request, environment)?;
        let originals = match (&request.previous_package, &request.previous) {
            (Some(package), Some(previous)) if !result.needs_original.is_empty() => {
                retained_origins(
                    root,
                    &config,
                    &request.platform,
                    previous,
                    package,
                    &result,
                    environment,
                    None,
                )?
            }
            (Some(_), None) => return Err("previous package requires a previous release".into()),
            _ => BTreeMap::new(),
        };
        let report = serde_json::json!({
            "schema": "chrono-release-preview-plan/v1", "current": request.current,
            "previous": request.previous, "platform": request.platform, "jobs": result,
            "intervening": request.intervening,
            "validated_originals": originals,
        });
        let parent = output.parent().ok_or("missing preview output directory")?;
        let mut staged =
            tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
        serde_json::to_writer(staged.as_file_mut(), &report).map_err(|error| error.to_string())?;
        staged.flush().map_err(|error| error.to_string())?;
        staged
            .persist_noclobber(output)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn plan(
        root: &Path,
        config: &crate::delivery::Config,
        request: &Request,
        environment: &Environment,
    ) -> Result<Plan, String> {
        let current = &request.current;
        let platform = &request.platform;
        if current.channel != Channel::Preview {
            return Err("component planning requires a preview identity".into());
        }
        let ci = current
            .verified_ci
            .as_ref()
            .ok_or("missing verified landing CI")?;
        let evidence = landing(
            &chrono_harness::no_symlink_parents(root, &request.collection)?,
            &config.publication.repository,
            ci.run_id,
            ci.attempt,
            &current.source_commit,
        )?;
        let mut predecessor = evidence.as_ref().map(|impact| impact.base.clone());
        let mut visited = BTreeSet::from([current.source_commit.clone()]);
        for link in &request.intervening {
            let candidate = predecessor
                .as_deref()
                .ok_or("missing intermediate landing head")?;
            if !visited.insert(candidate.to_owned()) {
                return Err("repeated intermediate landing".into());
            }
            let bridge = landing(
                &chrono_harness::no_symlink_parents(root, &link.collection)?,
                &config.publication.repository,
                link.run_id,
                link.attempt,
                candidate,
            )?
            .ok_or("intermediate landing has no delta")?;
            if !bridge.unaffected() {
                return Err("intermediate landing has affected consumers".into());
            }
            predecessor = Some(bridge.base);
        }
        let workflow_config = &config
            .promotion
            .as_ref()
            .ok_or("missing registered release workflow")?
            .workflow_config;
        let previous = request
            .previous
            .as_ref()
            .zip(evidence.as_ref())
            .filter(|(old, _)| {
                super::matching_predecessor(
                    old.channel == Channel::Preview,
                    &old.source_commit,
                    predecessor.as_deref().unwrap_or(""),
                )
            })
            .map(|(old, impact)| {
                Snapshot::read(root, workflow_config, old, environment)
                    .map(|snapshot| (snapshot, impact))
            })
            .transpose()?;
        let current = Snapshot::read(root, workflow_config, current, environment)?;
        let registered = current
            .loaded
            .recipe
            .native_jobs
            .get(platform)
            .ok_or("unregistered preview platform")?;
        let mut required = BTreeSet::new();
        for unit in &current.loaded.recipe.units {
            let id = unit.id();
            let required_here = match &previous {
                None => true,
                Some((previous, impact)) => match previous
                    .loaded
                    .recipe
                    .units
                    .iter()
                    .find(|old| old.id() == id)
                {
                    None => true,
                    Some(old) => {
                        !previous.loaded.recipe.native_jobs.contains_key(platform)
                            || current.source_affected(platform, unit, impact)?
                            || previous.source_affected(platform, old, impact)?
                            || previous.contract(platform, old)?
                                != current.contract(platform, unit)?
                    }
                },
            };
            if required_here {
                required.insert(id.to_owned());
            }
        }
        for unit in &current.loaded.recipe.units {
            if unit
                .needs()
                .iter()
                .any(|producer| required.contains(producer))
            {
                required.insert(unit.id().to_owned());
            }
        }
        let mut plan = Plan {
            required: BTreeSet::new(),
            needs_original: BTreeSet::new(),
        };
        for unit in &current.loaded.recipe.units {
            let job = registered
                .get(unit.id())
                .ok_or("missing registered preview job")?
                .clone();
            if required.contains(unit.id()) {
                plan.required.insert(job);
            } else {
                plan.needs_original.insert(job);
            }
        }
        Ok(plan)
    }
}
