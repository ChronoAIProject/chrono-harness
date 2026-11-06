//! Selected coordinator bytes change only under original affected enrollment EX leases.
use super::*;

fn executable_binding(path: &Path) -> Result<Value, String> {
    let id = crate::ownership::identity(path)?;
    let before = fs::metadata(path).map_err(|e| e.to_string())?;
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let after = fs::metadata(path).map_err(|e| e.to_string())?;
    if crate::ownership::identity(path)? != id
        || before.len() != after.len()
        || before.modified().map_err(|e| e.to_string())?
            != after.modified().map_err(|e| e.to_string())?
    {
        return Err("deployment executable identity changed".into());
    }
    Ok(value!({"identity":id,"sha256":sha256(&bytes),"size":bytes.len()}))
}

// Fixed producer data is checked here before any selected bytes change.
fn stage_registration(
    r: &mut Runner,
    root: &Path,
    head: &str,
    path: &str,
) -> Result<Vec<u8>, String> {
    let bytes = fs::read(no_symlink_parents(root, path)?).map_err(|e| e.to_string())?;
    if r.blob(root, head, path)? != bytes {
        return Err(format!(
            "owner stage registration differs from source: {path}"
        ));
    }
    Ok(bytes)
}

fn validate_stage(
    r: &mut Runner,
    root: &Path,
    head: &str,
    profile: &str,
    policy: &Value,
    stage: &Value,
) -> Result<(), String> {
    let tree = r.oid(root, &format!("{head}^{{tree}}"))?;
    let observed = value!({"commit":head,"tree":tree,"dirty":false,"error":null});
    if stage["source"]
        != value!({"state":"clean","commit":head,"tree":tree,
        "before":observed,"after":observed})
        || stage["installed"] != value!([])
    {
        return Err("owner stage requires its original clean source and no installations".into());
    }
    let projects_path = policy["projects"].as_str().ok_or("owner stage projects")?;
    let mut paths = vec![
        profile,
        projects_path,
        ".chrono-harness/ci/bootstrap.py",
        ".chrono-harness/ci/bootstrap_owner.py",
        ".chrono-harness/ci/bootstrap_shared.py",
    ];
    if let Some(path) = policy.get("tool_bindings_registry") {
        paths.push(path.as_str().ok_or("owner stage toolchain registry path")?);
    }
    if let Some(path) = policy.get("owner_upgrade") {
        paths.push(path.as_str().ok_or("owner stage upgrade path")?);
    }
    let mut registrations = serde_json::Map::new();
    let mut registration_bytes = BTreeMap::new();
    for path in paths {
        let bytes = stage_registration(r, root, head, path)?;
        registrations.insert(path.into(), value!(sha256(&bytes)));
        registration_bytes.insert(path, bytes);
    }
    if stage["registrations"] != Value::Object(registrations) {
        return Err("owner stage execution registrations differ".into());
    }
    if let Some(homes) = policy.get("host_environment") {
        if stage["host_environment"] != *homes {
            return Err("owner stage host environment differs".into());
        }
        for (key, path) in homes.as_object().ok_or("owner stage host environment")? {
            let path = path.as_str().ok_or("owner stage host environment path")?;
            relative_path(path)?;
            if !path.starts_with(".chrono-harness/") {
                return Err("owner stage host environment must be a host path".into());
            }
            if stage["environment"][key] != value!(root.join(path)) {
                return Err(format!(
                    "owner stage producer host environment differs: {key}"
                ));
            }
        }
    }
    let registry_bindings;
    let bindings = if let Some(path) = policy.get("tool_bindings_registry") {
        if policy.get("tool_bindings").is_some() || policy.get("native_platform").is_some() {
            return Err("conflicting owner stage binding declarations".into());
        }
        let path = path.as_str().ok_or("owner stage toolchain registry")?;
        if !path.starts_with(".chrono-harness/ci/") {
            return Err("owner stage toolchain registry must be a CI path".into());
        }
        let registry = json(
            registration_bytes
                .get(path)
                .ok_or("owner stage registry input")?,
        )?;
        if registry["schema"] != "chrono-bootstrap-toolchains/v1"
            || registry["rust_toolchain"] != policy["rust_toolchain"]
        {
            return Err("owner stage toolchain registry differs".into());
        }
        let system = match std::env::consts::OS {
            "macos" => "Darwin",
            "linux" => "Linux",
            _ => return Err("unsupported owner stage native platform".into()),
        };
        let machine = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "aarch64") => "arm64",
            (_, arch) => arch,
        };
        let tools = registry["platforms"][format!("{system}/{machine}")]["tools"]
            .as_object()
            .ok_or("missing owner stage native toolchain registration")?;
        registry_bindings = Value::Object(
            tools
                .iter()
                .map(|(name, item)| {
                    (
                        name.clone(),
                        value!({
            "sha256":item["sha256"],"version":item["version"],"allow_relocation":true}),
                    )
                })
                .collect(),
        );
        Some(&registry_bindings)
    } else {
        if policy["require_tool_bindings"] == true {
            let system = match std::env::consts::OS {
                "macos" => "Darwin",
                "linux" => "Linux",
                _ => "unsupported",
            };
            let machine = match (std::env::consts::OS, std::env::consts::ARCH) {
                ("macos", "aarch64") => "arm64",
                (_, arch) => arch,
            };
            if policy.get("tool_bindings").is_none()
                || policy["native_platform"] != value!({"system":system,"machine":machine})
            {
                return Err("missing owner stage native binding declarations".into());
            }
        }
        policy.get("tool_bindings")
    };
    let mut selected = BTreeMap::new();
    if let Some(bindings) = bindings {
        let object = bindings
            .as_object()
            .ok_or("owner stage executable bindings")?;
        let rows = stage["executables"]
            .as_array()
            .ok_or("owner stage executable observations")?;
        if object.len() != 2
            || !object.contains_key("cargo")
            || !object.contains_key("rustc")
            || rows.len() != 2
        {
            return Err("owner stage requires exactly Cargo and rustc bindings".into());
        }
        for (name, row) in ["cargo", "rustc"].into_iter().zip(rows) {
            let binding = &bindings[name];
            let fields = binding
                .as_object()
                .ok_or("owner stage executable binding")?;
            let digest = binding["sha256"]
                .as_str()
                .ok_or("owner stage executable digest")?;
            if fields.keys().any(|key| {
                !["path", "sha256", "version", "allow_relocation"].contains(&key.as_str())
            }) || digest.len() != 64
                || !digest
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                || binding["version"].as_str().is_none()
                || binding.get("path").is_some_and(|v| !v.is_string())
                || (binding.get("path").is_none() && policy.get("tool_bindings_registry").is_none())
                || binding
                    .get("allow_relocation")
                    .is_some_and(|v| !v.is_boolean())
            {
                return Err("invalid owner stage executable binding".into());
            }
            let path = row["path"].as_str().ok_or("owner stage executable path")?;
            if !Path::new(path).is_absolute()
                || row["sha256"] != binding["sha256"]
                || row["version"] != binding["version"]
            {
                return Err(format!("owner stage executable binding differs: {name}"));
            }
            if binding["allow_relocation"] != true {
                let declared = binding["path"]
                    .as_str()
                    .ok_or("owner stage declared executable path")?;
                if root.join(declared) != Path::new(path) {
                    return Err("owner stage executable relocation is not permitted".into());
                }
            }
            selected.insert(name, path);
        }
        if stage["selection"]
            != value!({"cargo":selected["cargo"],"rustc":selected["rustc"],
            "toolchain":policy["rust_toolchain"],"compiler":selected["rustc"]})
            || stage["versions"]
                != value!({"cargo":bindings["cargo"]["version"],"rustc":bindings["rustc"]["version"]})
        {
            return Err("owner stage executable selection differs".into());
        }
        let mut environment = value!({"RUSTUP_TOOLCHAIN":policy["rust_toolchain"],"RUSTC":selected["rustc"],"RUSTC_WRAPPER":"","RUSTC_WORKSPACE_WRAPPER":""});
        if let Some(incremental) = policy.get("rust_incremental") {
            environment["CARGO_INCREMENTAL"] = value!(if incremental
                .as_bool()
                .ok_or("owner stage incremental declaration")?
            {
                "1"
            } else {
                "0"
            });
        }
        for (key, value) in environment.as_object().unwrap() {
            if stage["environment"][key] != *value {
                return Err(format!("owner stage compiler environment differs: {key}"));
            }
        }
    }
    let projects = json(
        registration_bytes
            .get(projects_path)
            .ok_or("owner stage projects input")?,
    )?;
    let mut actions = BTreeMap::new();
    for owner in projects["projects"]
        .as_array()
        .ok_or("owner stage projects")?
        .iter()
        .chain(
            projects["scripts"]
                .as_array()
                .ok_or("owner stage scripts")?,
        )
    {
        for action in owner["actions"]
            .as_object()
            .ok_or("owner stage actions")?
            .values()
        {
            let name = action["operation"]
                .as_str()
                .ok_or("owner stage operation")?;
            if actions.insert(name, action).is_some() {
                return Err(format!("duplicate owner stage operation: {name}"));
            }
        }
    }
    let mut sequence = Vec::new();
    if let Some(prerequisites) = policy.get("prerequisites") {
        sequence.extend(
            prerequisites
                .as_array()
                .ok_or("owner stage prerequisites")?
                .iter(),
        );
    }
    sequence.extend(
        policy["operations"]
            .as_array()
            .ok_or("owner stage operations")?
            .iter(),
    );
    let mut unique = std::collections::BTreeSet::new();
    for name in &sequence {
        let name = name.as_str().ok_or("owner stage operation name")?;
        if name.is_empty() || !unique.insert(name) {
            return Err("duplicate or empty owner stage operation".into());
        }
    }
    let actual = stage["operations"]
        .as_array()
        .ok_or("owner stage original operations")?;
    if actual
        .iter()
        .map(|row| &row["operation"])
        .collect::<Vec<_>>()
        != sequence
    {
        return Err("owner stage producer operations differ".into());
    }
    let mut streams = std::collections::BTreeSet::new();
    for row in actual {
        let name = row["operation"]
            .as_str()
            .ok_or("owner stage operation name")?;
        let action = actions
            .get(name)
            .ok_or("owner stage operation is not registered")?;
        let tool = action["tool"]
            .as_str()
            .ok_or("owner stage operation tool")?;
        let program = match selected.get(tool) {
            Some(path) => *path,
            None => policy["tools"][tool]
                .as_str()
                .ok_or("owner stage operation executable")?,
        };
        let program = if !Path::new(program).is_absolute() && program.contains('/') {
            root.join(program)
                .to_str()
                .ok_or("owner stage executable UTF-8")?
                .to_owned()
        } else {
            program.to_owned()
        };
        let mut argv = vec![value!(program)];
        argv.extend(
            action["argv"]
                .as_array()
                .ok_or("owner stage operation argv")?
                .iter()
                .cloned(),
        );
        if row["tool"] != action["tool"]
            || row["argv"] != value!(argv)
            || row["cwd"] != value!(root)
            || row["exit_code"].as_i64() != Some(0)
            || row["pid"].as_u64().unwrap_or(0) == 0
        {
            return Err(format!(
                "owner stage producer operation execution differs: {name}"
            ));
        }
        if selected.contains_key(tool)
            && row["executable_sha256"] != bindings.unwrap()[tool]["sha256"]
        {
            return Err("owner stage operation executable differs".into());
        }
        for channel in ["stdout", "stderr"] {
            let reference = &row[channel];
            let path = reference["path"]
                .as_str()
                .ok_or("owner stage original stream path")?;
            let bytes = crate::recovery::state_bytes(root, path)?;
            if !streams.insert(path)
                || reference["sha256"] != sha256(&bytes)
                || reference["size"].as_u64() != Some(bytes.len() as u64)
            {
                return Err(format!(
                    "owner stage original process stream differs: {path}"
                ));
            }
        }
    }
    Ok(())
}

fn publication_original(path: &Path, value: &Value) -> Result<(), String> {
    let mut raw = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    raw.push(b'\n');
    if path.exists() {
        crate::ownership::identity(path)?;
        if fs::read(path).map_err(|e| e.to_string())? != raw {
            return Err("owner publication original collision".into());
        }
    } else {
        write_bytes(path, &raw, true)?;
    }
    Ok(())
}

// Only deploy_owner calls this, while its original admission and every affected
// enrollment remain exclusive. Intent and the retained inode survive interruption
// after rename; publication alone claims neither adoption nor task completion.
fn publish_owner(root: &Path, record: &Value) -> Result<Value, String> {
    let staged_path = record["staged"]["path"]
        .as_str()
        .ok_or("staged owner path")?;
    let selected_path = record["selected"].as_str().ok_or("selected owner path")?;
    if !staged_path.starts_with(".chrono-harness/bin/owner-stages/")
        || !selected_path.starts_with(".chrono-harness/bin/")
    {
        return Err("owner publication requires retained executable paths".into());
    }
    let staged = no_symlink_parents(root, staged_path)?;
    let target = no_symlink_parents(root, selected_path)?;
    let mut binding = executable_binding(&staged)?;
    binding["path"] = value!(staged_path);
    if binding != record["staged"] {
        return Err("retained owner stage changed before publication".into());
    }
    let parent = staged_path.rsplit_once('/').ok_or("stage directory")?.0;
    let previous = no_symlink_parents(root, &format!("{parent}/previous-owner"))?;
    let intent = no_symlink_parents(root, &format!("{parent}/publication-intent.json"))?;
    let next = no_symlink_parents(root, &format!("{parent}/publication-next-owner"))?;
    let result_path = no_symlink_parents(root, &format!("{parent}/publication-result.json"))?;
    if !previous.exists() {
        if executable_binding(&target)? != record["previous"] {
            return Err("selected owner changed before original preservation".into());
        }
        fs::hard_link(&target, &previous).map_err(|e| e.to_string())?;
    }
    if executable_binding(&previous)? != record["previous"] {
        return Err("retained original owner changed".into());
    }
    let planned = if intent.exists() {
        let planned = json(&fs::read(&intent).map_err(|e| e.to_string())?)?;
        if planned.as_object().map(|o| o.len()) != Some(3)
            || planned["schema"] != "chrono-bootstrap-owner-publication-intent/v1"
            || planned["stage"] != *record
        {
            return Err("owner publication intent binding differs".into());
        }
        planned
    } else {
        // Never adopt a replacement whose preparation has no original intent.
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&next)
            .map_err(|e| format!("prepare owner publication: {e}"))?;
        output
            .write_all(&fs::read(&staged).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            output
                .set_permissions(fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
        output.sync_all().map_err(|e| e.to_string())?;
        let planned = value!({"schema":"chrono-bootstrap-owner-publication-intent/v1",
            "stage":record,"replacement":executable_binding(&next)?});
        publication_original(&intent, &planned)?;
        planned
    };
    if planned["replacement"]["sha256"] != record["staged"]["sha256"]
        || planned["replacement"]["size"] != record["staged"]["size"]
    {
        return Err("prepared publication owner differs from retained stage".into());
    }
    let current = executable_binding(&target)?;
    if current == record["previous"] {
        if executable_binding(&next)? != planned["replacement"] {
            return Err("prepared publication owner changed".into());
        }
        fs::rename(&next, &target).map_err(|e| e.to_string())?;
    } else if current != planned["replacement"] {
        return Err("selected owner drifted after publication intent".into());
    }
    let installed = executable_binding(&target)?;
    if installed != planned["replacement"]
        || installed["sha256"] != record["staged"]["sha256"]
        || executable_binding(&previous)? != record["previous"]
    {
        return Err("installed owner verification failed".into());
    }
    let result = value!({"schema":"chrono-bootstrap-owner-publication/v1","stage":record,
        "installed":installed,"adoption":"not-claimed","task_completion":"not-claimed"});
    publication_original(&result_path, &result)?;
    Ok(result)
}

impl Manager {
    pub(super) fn deploy_owner(
        &mut self,
        r: &mut Runner,
        root: &Path,
        profile: &str,
        stage_path: &str,
        stage_sha: &str,
        report: &mut Value,
    ) -> Result<(), String> {
        if root != self.inputs.policy.coordinator_root || self._gate.lease.is_none() {
            return Err("owner deployment requires the original kernel Git-main owner".into());
        }
        let i = self
            .current_entry(root)
            .ok_or("owner deployment requires enrollment")?;
        let e = self.ledger.entries[i].clone();
        if !e.cache_only || e.status != "active" || e.terminal.is_some() {
            return Err("owner deployment preserves unknown or terminal enrollment".into());
        }
        // Exclusive executable deployment excludes new ordinary entrants. Each already admitted target
        // has its own enrollment SH, including linked checkout descendants.
        let inventory = r.inventory(root)?;
        let mut entries = Vec::new();
        let mut originals = Vec::new();
        for path in self.enrollment_paths()? {
            let file = record_directory(&self.directory, &path)?.join("ledger.json");
            let raw = fs::read(&file).map_err(|e| e.to_string())?;
            let ledger: Ledger = decode(&raw)?;
            if ledger.coordinator_root != self.inputs.policy.coordinator_root
                || ledger.common != self.inputs.common
                || ledger.entries.iter().any(|entry| entry.path != path)
            {
                return Err("owner deployment enrollment namespace changed".into());
            }
            entries.extend(
                ledger
                    .entries
                    .into_iter()
                    .filter(|entry| entry.status != "disposed"),
            );
            originals.push((file, raw));
        }
        for row in &inventory {
            let path = Path::new(&row["worktree"]);
            if !entries.iter().any(|entry| entry.path == path) {
                return Err(format!(
                    "owner deployment affected checkout {} has no current enrollment; original custodian exclusion required; preserve selected bytes",
                    path.display()
                ));
            }
            if row.contains_key("locked") || row.contains_key("prunable") {
                return Err(format!(
                    "owner deployment affected checkout {} has unknown Git ownership; preserve selected bytes",
                    path.display()
                ));
            }
        }
        let mut exclusions = Vec::new();
        for entry in &entries {
            if inventory
                .iter()
                .filter(|row| Path::new(&row["worktree"]) == entry.path)
                .count()
                != 1
            {
                return Err(format!(
                    "owner deployment affected enrollment {} is unavailable in original Git inventory; preserve selected bytes",
                    entry.path.display()
                ));
            }
            let lease = self.lease(entry, true)?.ok_or_else(|| format!(
                "owner deployment affected enrollment {} has live inherited consumers; preserve selected bytes", entry.path.display()))?;
            if !entry.uses.is_empty() {
                return Err(format!(
                    "owner deployment affected enrollment {} has unjoined use inputs; preserve selected bytes",
                    entry.path.display()
                ));
            }
            exclusions.push((entry, r.oid(&entry.path, "HEAD")?, lease));
        }
        let ledger_path = self.record_directory.join("ledger.json");
        let ledger_bytes = fs::read(&ledger_path).map_err(|e| e.to_string())?;
        if value!(decode::<Ledger>(&ledger_bytes)?) != value!(&self.ledger) {
            return Err(
                "owner deployment original enrollment changed; preserve selected bytes".into(),
            );
        }
        let stable = |r: &mut Runner| -> Result<(), String> {
            self.stable(r)?;
            for (file, raw) in &originals {
                if fs::read(file).map_err(|e| e.to_string())? != *raw {
                    return Err("owner deployment enrollment changed; preserve originals".into());
                }
            }
            if fs::read(&ledger_path).map_err(|e| e.to_string())? != ledger_bytes
                || r.inventory(root)? != inventory
            {
                return Err(
                    "owner deployment affected enrollment inventory changed; preserve originals"
                        .into(),
                );
            }
            for (entry, head, lease) in &exclusions {
                lease.stable()?;
                self.birth_ready(entry)?;
                if entry.path == root {
                    self.check_entry(r, entry, head, None)?;
                } else {
                    // Publication excludes these consumers without using their
                    // caches or adopting their independently owned policies.
                    self.check_attachment(r, entry, head, None)?;
                }
            }
            Ok(())
        };
        stable(r)?;
        let head = r.oid(root, "HEAD")?;
        let policy = bootstrap_policy(r, root, &self.inputs.registrations, profile)?;
        let declaration = &policy["owner_stage"];
        if !stage_path.starts_with(".chrono-harness/state/bootstrap-") {
            return Err("owner deployment requires an original retained bootstrap report".into());
        }
        let raw = crate::recovery::state_bytes(root, stage_path)?;
        if sha256(&raw) != stage_sha {
            return Err("owner deployment stage report changed".into());
        }
        let stage = json(&raw)?;
        if stage["schema"] != "chrono-bootstrap-result/v1"
            || stage["status"] != "passed"
            || stage["config"] != profile
            || stage["source"]["state"] != "clean"
            || stage["source"]["commit"] != head
            || stage["host_root"] != value!(root)
        {
            return Err("owner deployment stage source/configuration differs".into());
        }
        validate_stage(r, root, &head, profile, &policy, &stage)?;
        let record = &stage["owner_stage"];
        if record["selected"] != declaration["selected"] {
            return Err("owner deployment destination differs from declaration".into());
        }
        let staged_path = record["staged"]["path"]
            .as_str()
            .ok_or("staged executable path")?;
        if !staged_path.starts_with(".chrono-harness/bin/owner-stages/") {
            return Err("owner deployment stage must survive cache disposal".into());
        }
        let staged = no_symlink_parents(root, staged_path)?;
        let mut bound = executable_binding(&staged)?;
        bound["path"] = value!(staged_path);
        if bound != record["staged"]
            || fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
                != staged
        {
            return Err("owner deployment must execute its exact retained candidate bytes".into());
        }
        let selected = no_symlink_parents(
            root,
            declaration["selected"]
                .as_str()
                .ok_or("selected owner path")?,
        )?;
        let owner_identity = value!({"attachment":e.attachment,
            "admission":self._gate.lease.as_ref().unwrap().id(),
            "enrollment":e.ownership.as_ref().ok_or("kernel ownership")?.id,
            "coordinator_root":self.inputs.policy.coordinator_root,"common":self.ledger.common,
            "enrollments":exclusions.iter().map(|(entry, head, lease)| value!({
                "path":entry.path,"attachment":entry.attachment,"head":head,
                "enrollment":{"path":entry.ownership.as_ref().unwrap().path,"id":lease.id()}
            })).collect::<Vec<_>>()});
        report["owner_deployment_identity"] = owner_identity.clone();
        // Stage qualification and publication use the same original Rust owner.
        // No child process or synthetic managed-process receipt is produced.
        stable(r)?;
        let publication = publish_owner(root, record)?;
        stable(r)?;
        if publication["schema"] != "chrono-bootstrap-owner-publication/v1"
            || publication["stage"] != *record
            || publication["installed"] != executable_binding(&selected)?
            || publication["installed"]["sha256"] != record["staged"]["sha256"]
            || executable_binding(
                &staged
                    .parent()
                    .ok_or("stage directory")?
                    .join("previous-owner"),
            )? != record["previous"]
        {
            return Err(
                "owner publication original/result identity mismatch; adoption not claimed".into(),
            );
        }
        let mut adoption = value!({"schema":"chrono-worktree-owner-adoption/v1",
            "source_commit":head,"stage_result":stage_path,"stage_sha256":stage_sha,
            "installed":publication["installed"],"original_owner":record["previous"],
            "original_protection":owner_identity,"publication":publication,
            "terminal_handoff":"not-claimed",
            "task_completion":"not-claimed"});
        let receipt = self.immutable_reconciled(
            &format!(
                "owner-adoption-{}.json",
                chrono_harness::wire::digest(&adoption)?
            ),
            &adoption,
        )?;
        stable(r)?;
        if executable_binding(&selected)? != adoption["installed"] {
            return Err("installed owner changed before adoption".into());
        }
        adoption["receipt"] = value!(receipt);
        let index = no_symlink_parents(root, ".chrono-harness/state/bootstrap-owner-current.json")?;
        write_json(&index, &adoption, false)?;
        stable(r)?;
        report["owner_deployment"] = adoption;
        Ok(())
    }
}
