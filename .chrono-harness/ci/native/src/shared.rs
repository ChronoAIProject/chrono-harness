//! 本宿主共享启动产物；安装前校验全部原件，SDK 和工具不从产物中执行。
use crate::{
    bindings::{Environment, NativePlatform},
    construction::{BuildFailure, Report},
    environment,
    original::Producer,
    profile::{self, Profile},
    source::{self, SourceObservation},
    verified_bindings::{self as policy, Executable},
};
use chrono_harness::{decode, sha256};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

const INSTALL: &str = ".chrono-harness/state/startup-install.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileEntry {
    config: String,
    build: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    schema: String,
    directory: String,
    profiles: BTreeMap<String, ProfileEntry>,
    consumers: BTreeMap<String, Vec<String>>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    path: String,
    sha256: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct File {
    path: String,
    destination: String,
    sha256: String,
    size: u64,
    mode: u32,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactProfile {
    config: String,
    config_sha256: String,
    report: Reference,
    files: Vec<File>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    source: SourceObservation,
    platform: NativePlatform,
    config: String,
    config_sha256: String,
    profiles: BTreeMap<String, ArtifactProfile>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Installation {
    schema: String,
    binding: String,
    consumer: String,
    profiles: Vec<String>,
    source: SourceObservation,
    local_executables: BTreeMap<String, Vec<Executable>>,
}

struct SelectedProfile {
    config_sha256: String,
    producer: Producer,
    raw: Vec<u8>,
}
pub struct Shared {
    config: String,
    config_sha256: String,
    registration: Registration,
    profiles: BTreeMap<String, SelectedProfile>,
    transfer: PathBuf,
}
struct Validated {
    manifest: Manifest,
    originals: BTreeMap<String, Vec<u8>>,
    reports: BTreeMap<String, Report>,
    payloads: BTreeMap<String, Vec<u8>>,
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(path.parent().ok_or("missing startup parent")?)
        .map_err(|e| e.to_string())?;
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .and_then(|mut f| f.write_all(bytes))
        .map_err(|e| e.to_string())
}
fn retain(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        if fs::read(path).map_err(|e| e.to_string())? != bytes {
            return Err("startup original differs".into());
        }
        Ok(())
    } else {
        write_new(path, bytes)
    }
}
fn replace(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let parent = path.parent().ok_or("missing startup destination parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".startup-")
        .tempfile_in(parent)
        .map_err(|e| e.to_string())?;
    temporary.write_all(bytes).map_err(|e| e.to_string())?;
    temporary
        .as_file()
        .set_permissions(fs::Permissions::from_mode(mode))
        .map_err(|e| e.to_string())?;
    temporary.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
fn source(root: &Path, environment: &Environment) -> Result<SourceObservation, String> {
    let metadata = fs::symlink_metadata(root.join(".git")).map_err(|e| e.to_string())?;
    if !metadata.is_dir() {
        return Err("shared startup requires a primary checkout".into());
    }
    let observed = source::observe(root, environment);
    if observed.dirty != Some(false) || observed.error.is_some() {
        return Err(format!(
            "shared startup requires clean source: {observed:?}"
        ));
    }
    Ok(observed)
}
fn streams(report: &Report) -> impl Iterator<Item = &crate::execution::Stream> {
    report
        .operations
        .iter()
        .flat_map(|row| [&row.stdout, &row.stderr])
}
fn imported(original: &[u8], binding: &str) -> Result<Vec<u8>, String> {
    let mut value: Value = decode(original)?;
    value["provenance"] =
        json!({"kind":"shared-startup", "binding":binding, "report_sha256":sha256(original)});
    encode(&value)
}

impl Shared {
    /// 全部配置和目的地先预检；不选择工具链或建立缓存目录。
    pub fn read(root: &Path, config: &str, native: &NativePlatform) -> Result<Self, String> {
        let raw = profile::read(root, config, b".chrono-harness/ci/")?;
        let registration: Registration = decode(&raw)?;
        if !policy::startup_schema(&registration.schema, "chrono-host-startup/v1")
            || registration.profiles.is_empty()
            || registration.consumers.is_empty()
        {
            return Err("invalid host startup registration".into());
        }
        let directory = registration
            .directory
            .strip_suffix('/')
            .ok_or("startup directory requires trailing slash")?;
        let transfer = profile::path(root, directory, b".chrono-harness/")?;
        for reserved in [
            ".chrono-harness/state",
            ".chrono-harness/bin",
            ".chrono-harness/ci",
        ] {
            if !policy::disjoint_homes(directory.as_bytes(), reserved.as_bytes()) {
                return Err("startup transfer overlaps source, tools or evidence".into());
            }
        }
        let mut profiles = BTreeMap::new();
        let mut destinations = BTreeSet::new();
        let mut reports = BTreeSet::new();
        for (name, entry) in &registration.profiles {
            if !policy::startup_name(name.as_bytes()) {
                return Err("invalid startup profile name".into());
            }
            let bytes = profile::read(root, &entry.config, b".chrono-harness/ci/")?;
            let parsed = Profile::decode(root, &bytes)?;
            if parsed.install.is_empty() {
                return Err("startup profile must install declared tools".into());
            }
            if !reports.insert(parsed.report_destination(root)?) {
                return Err("startup profiles must have separate reports".into());
            }
            for installed in &parsed.install {
                if !destinations.insert(installed.destination.clone()) {
                    return Err("startup profiles must have separate tools".into());
                }
            }
            let producer = Producer::read(&entry.config, native, &|p| {
                profile::read(root, p, b".chrono-harness/")
            })?;
            profiles.insert(
                name.clone(),
                SelectedProfile {
                    config_sha256: sha256(&bytes),
                    producer,
                    raw: bytes,
                },
            );
        }
        let available = profiles.keys().cloned().collect();
        for selected in registration.consumers.values() {
            if selected.is_empty()
                || !policy::valid_names(selected)
                || !policy::names_subset(selected, &available)
            {
                return Err("startup consumer requires explicit unique profiles".into());
            }
        }
        Ok(Self {
            config: config.into(),
            config_sha256: sha256(&raw),
            registration,
            profiles,
            transfer,
        })
    }

    fn original(
        &self,
        name: &str,
        raw: &[u8],
        observed: &SourceObservation,
        storage: &Path,
    ) -> Result<Report, String> {
        self.profiles[name].producer.validate(raw, observed, &|p| {
            profile::read(storage, p, b".chrono-harness/state/")
        })
    }

    fn installations(&self, name: &str, report: &Report, files: &[File]) -> Result<(), String> {
        let expected = self.profiles[name]
            .producer
            .profile
            .install
            .iter()
            .map(|i| i.destination.clone())
            .collect();
        let actual = files.iter().map(|i| i.destination.clone()).collect();
        let reported = report.installed.iter().map(|i| i.path.clone()).collect();
        if !policy::text_list_equal(&actual, &expected)
            || !policy::text_list_equal(&reported, &expected)
        {
            return Err("startup install set differs".into());
        }
        for (item, installed) in files.iter().zip(&report.installed) {
            if !policy::startup_schema(&item.sha256, &installed.sha256) {
                return Err("startup original report payload differs".into());
            }
        }
        Ok(())
    }

    pub fn produce(
        &self,
        root: &Path,
        environment: &Environment,
        native: &NativePlatform,
        mut build: impl FnMut(&str) -> Result<(), BuildFailure>,
    ) -> Result<(), BuildFailure> {
        let observed = source(root, environment)?;
        if self.transfer.exists() {
            return Err("startup transfer already exists".into());
        }
        for entry in self.registration.profiles.values() {
            if entry.build {
                build(&entry.config)?;
            }
        }
        if !policy::observation_matches(&observed, &source(root, environment)?) {
            return Err("startup source changed during production".into());
        }
        let state = profile::path(root, ".chrono-harness/state", b".chrono-harness/")?;
        fs::create_dir_all(&state).map_err(|e| e.to_string())?;
        let temporary = tempfile::Builder::new()
            .prefix("startup-export-")
            .tempdir_in(state)
            .map_err(|e| e.to_string())?;
        let staging = temporary.path().join("artifact");
        fs::create_dir(&staging).map_err(|e| e.to_string())?;
        let mut profiles = BTreeMap::new();
        let mut index = 0;
        for (name, selected) in &self.profiles {
            let profile = &selected.producer.profile;
            let raw = fs::read(profile.report_destination(root)?).map_err(|e| e.to_string())?;
            let report = self.original(name, &raw, &observed, root)?;
            let report_path = format!("reports/{name}.json");
            write_new(&staging.join(&report_path), &raw)?;
            for descriptor in streams(&report) {
                let target = profile::path(&staging, &descriptor.path, b".chrono-harness/state/")?;
                write_new(
                    &target,
                    &profile::read(root, &descriptor.path, b".chrono-harness/state/")?,
                )?;
            }
            let mut files = Vec::new();
            for installed in &profile.install {
                let target = profile::path(root, &installed.destination, b".chrono-harness/bin/")?;
                let bytes = fs::read(&target).map_err(|e| e.to_string())?;
                let mode = fs::metadata(&target)
                    .map_err(|e| e.to_string())?
                    .permissions()
                    .mode()
                    & 0o777;
                if !policy::startup_mode(mode) {
                    return Err("startup tool is not executable".into());
                }
                let path = format!("tools/{index}");
                index += 1;
                write_new(&staging.join(&path), &bytes)?;
                files.push(File {
                    path,
                    destination: installed.destination.clone(),
                    sha256: sha256(&bytes),
                    size: bytes.len() as u64,
                    mode,
                });
            }
            self.installations(name, &report, &files)?;
            profiles.insert(
                name.clone(),
                ArtifactProfile {
                    config: self.registration.profiles[name].config.clone(),
                    config_sha256: selected.config_sha256.clone(),
                    report: Reference {
                        path: report_path,
                        sha256: sha256(&raw),
                    },
                    files,
                },
            );
        }
        let manifest = Manifest {
            schema: "chrono-host-startup-artifact/v1".into(),
            source: observed,
            platform: NativePlatform {
                system: native.system.clone(),
                machine: native.machine.clone(),
            },
            config: self.config.clone(),
            config_sha256: self.config_sha256.clone(),
            profiles,
        };
        let raw = encode(&manifest)?;
        write_new(&staging.join("manifest.json"), &raw)?;
        if !policy::observation_matches(&manifest.source, &source(root, environment)?) {
            return Err("startup source changed before publication".into());
        }
        let binding = sha256(&raw);
        let original = profile::path(
            root,
            &format!(".chrono-harness/state/startup-originals/manifest-{binding}.json"),
            b".chrono-harness/state/",
        )?;
        retain(&original, &raw)?;
        fs::create_dir_all(self.transfer.parent().ok_or("missing transfer parent")?)
            .map_err(|e| e.to_string())?;
        fs::rename(staging, &self.transfer).map_err(|e| e.to_string())?;
        let output = environment
            .get(OsStr::new("GITHUB_OUTPUT"))
            .ok_or("missing GITHUB_OUTPUT")?;
        fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(output)
            .and_then(|mut f| writeln!(f, "binding={binding}"))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn validate(
        &self,
        root: &Path,
        environment: &Environment,
        native: &NativePlatform,
        binding: &str,
    ) -> Result<Validated, String> {
        let observed = source(root, environment)?;
        if !chrono_harness::verified_identifiers::valid_digest_text(binding) {
            return Err("missing or invalid startup producer binding".into());
        }
        let raw = profile::read(&self.transfer, "manifest.json", b"")?;
        if !policy::startup_schema(&sha256(&raw), binding) {
            return Err("startup manifest differs from producer binding".into());
        }
        let manifest: Manifest = decode(&raw)?;
        let names = manifest.profiles.keys().cloned().collect();
        let expected_names = self.profiles.keys().cloned().collect();
        if !policy::startup_schema(&manifest.schema, "chrono-host-startup-artifact/v1")
            || !policy::startup_schema(&manifest.config, &self.config)
            || !policy::startup_schema(&manifest.config_sha256, &self.config_sha256)
            || !policy::observation_matches(&manifest.source, &observed)
            || !policy::platform_matches(Some(&manifest.platform), native)
            || !policy::text_list_equal(&names, &expected_names)
        {
            return Err("startup source, platform or registration differs".into());
        }
        let mut seen = BTreeSet::new();
        let mut payloads = BTreeMap::new();
        let mut originals = BTreeMap::new();
        let mut reports = BTreeMap::new();
        for (name, selected) in &self.profiles {
            let entry = &manifest.profiles[name];
            if !policy::startup_schema(&entry.config, &self.registration.profiles[name].config)
                || !policy::startup_schema(&entry.config_sha256, &selected.config_sha256)
                || !seen.insert(entry.report.path.clone())
            {
                return Err("startup profile differs".into());
            }
            let original = profile::read(&self.transfer, &entry.report.path, b"reports/")?;
            if !policy::startup_schema(&sha256(&original), &entry.report.sha256) {
                return Err("startup original report differs".into());
            }
            let report = self.original(name, &original, &observed, &self.transfer)?;
            self.installations(name, &report, &entry.files)?;
            for descriptor in streams(&report) {
                if !seen.insert(descriptor.path.clone()) {
                    return Err("duplicate startup original stream".into());
                }
            }
            for item in &entry.files {
                let data = profile::read(&self.transfer, &item.path, b"tools/")?;
                if !seen.insert(item.path.clone())
                    || !policy::stream_matches(
                        &sha256(&data),
                        &item.sha256,
                        data.len() as u64,
                        item.size,
                    )
                    || !policy::startup_mode(item.mode)
                {
                    return Err("startup tool bytes or metadata differ".into());
                }
                payloads.insert(item.destination.clone(), data);
            }
            reports.insert(name.clone(), report);
            originals.insert(name.clone(), original);
        }
        Ok(Validated {
            manifest,
            payloads,
            originals,
            reports,
        })
    }

    fn local_sdk(
        &self,
        root: &Path,
        environment: &Environment,
        native: &NativePlatform,
        name: &str,
        original: &Report,
    ) -> Result<Vec<Executable>, BuildFailure> {
        let selected = &self.profiles[name];
        let profile = &selected.producer.profile;
        let registry = profile::read(
            root,
            &profile.tool_bindings_registry,
            b".chrono-harness/ci/",
        )?;
        let selected = environment::select(
            root,
            &selected.raw,
            native,
            Some(&registry),
            environment,
            &profile.tools,
        )?;
        let versions: BTreeMap<_, _> = [
            ("cargo".into(), selected.versions.cargo),
            ("rustc".into(), selected.versions.rustc),
        ]
        .into();
        if versions != original.versions {
            return Err("startup consumer toolchain differs".into());
        }
        let bindings = selected
            .bindings
            .ok_or("startup consumer executable bindings are missing")?;
        let local = vec![bindings.cargo, bindings.rustc];
        let producer: Vec<Executable> = serde_json::from_value(
            original
                .executables
                .clone()
                .ok_or("startup producer executable bindings are missing")?,
        )
        .map_err(|e| e.to_string())?;
        if producer.len() != local.len() {
            return Err("startup consumer executable binding differs".into());
        }
        for (actual, expected) in local.iter().zip(&producer) {
            if !policy::selected_matches(actual, expected, true) {
                return Err("startup consumer executable binding differs".into());
            }
        }
        Ok(local)
    }

    pub fn install(
        &self,
        root: &Path,
        environment: &Environment,
        native: &NativePlatform,
        consumer: &str,
    ) -> Result<(), BuildFailure> {
        let selected = self
            .registration
            .consumers
            .get(consumer)
            .ok_or("unknown startup consumer")?;
        let binding = environment
            .get(OsStr::new("CHRONO_STARTUP_BINDING"))
            .and_then(|s| s.to_str())
            .ok_or("missing startup producer binding")?;
        let validated = self.validate(root, environment, native, binding)?;
        let result_path = profile::path(root, INSTALL, b".chrono-harness/state/")?;
        if result_path.exists() {
            return Err("startup installation already has a result".into());
        }
        let mut writes = Vec::new();
        for name in selected {
            for item in &validated.manifest.profiles[name].files {
                writes.push((
                    profile::path(root, &item.destination, b".chrono-harness/bin/")?,
                    validated.payloads[&item.destination].clone(),
                    item.mode,
                ));
            }
            writes.push((
                self.profiles[name]
                    .producer
                    .profile
                    .report_destination(root)?,
                imported(&validated.originals[name], binding)?,
                0o644,
            ));
            writes.push((
                profile::path(
                    root,
                    &format!(".chrono-harness/state/startup-originals/{name}.json"),
                    b".chrono-harness/state/",
                )?,
                validated.originals[name].clone(),
                0o644,
            ));
            for descriptor in streams(&validated.reports[name]) {
                let target = profile::path(root, &descriptor.path, b".chrono-harness/state/")?;
                let bytes =
                    profile::read(&self.transfer, &descriptor.path, b".chrono-harness/state/")?;
                if target.exists() && fs::read(&target).map_err(|e| e.to_string())? != bytes {
                    return Err("startup original process stream collision".into());
                }
                writes.push((target, bytes, 0o644));
            }
        }
        for (target, _, _) in &writes {
            if target.exists() && !target.is_file() {
                return Err("startup destination is not a file".into());
            }
        }
        let mut local_executables = BTreeMap::new();
        for name in selected {
            local_executables.insert(
                name.clone(),
                self.local_sdk(root, environment, native, name, &validated.reports[name])?,
            );
        }
        if !policy::observation_matches(&validated.manifest.source, &source(root, environment)?) {
            return Err("startup consumer source changed before installation".into());
        }
        for (target, bytes, mode) in writes {
            replace(&target, &bytes, mode)?;
        }
        replace(
            &result_path,
            &encode(&Installation {
                schema: "chrono-host-startup-install/v1".into(),
                binding: binding.into(),
                consumer: consumer.into(),
                profiles: selected.clone(),
                source: validated.manifest.source,
                local_executables,
            })?,
            0o644,
        )?;
        Ok(())
    }

    pub fn verify(
        &self,
        root: &Path,
        environment: &Environment,
        native: &NativePlatform,
        name: &str,
    ) -> Result<(), BuildFailure> {
        let result: Installation =
            decode(&profile::read(root, INSTALL, b".chrono-harness/state/")?)?;
        let expected = self
            .registration
            .consumers
            .get(&result.consumer)
            .ok_or("unknown installed startup consumer")?;
        if !policy::startup_schema(&result.schema, "chrono-host-startup-install/v1")
            || !policy::text_list_equal(expected, &result.profiles)
            || !policy::names_subset(&vec![name.into()], &result.profiles)
        {
            return Err("startup profile was not installed for declared consumer".into());
        }
        let validated = self.validate(root, environment, native, &result.binding)?;
        if !policy::observation_matches(&result.source, &validated.manifest.source) {
            return Err("startup installation source differs".into());
        }
        for item in &validated.manifest.profiles[name].files {
            let target = profile::path(root, &item.destination, b".chrono-harness/bin/")?;
            let bytes = fs::read(&target).map_err(|e| e.to_string())?;
            if !policy::stream_matches(&sha256(&bytes), &item.sha256, bytes.len() as u64, item.size)
                || fs::metadata(&target)
                    .map_err(|e| e.to_string())?
                    .permissions()
                    .mode()
                    & 0o777
                    != item.mode
            {
                return Err(format!("installed startup tool differs: {}", item.destination).into());
            }
        }
        if fs::read(
            self.profiles[name]
                .producer
                .profile
                .report_destination(root)?,
        )
        .map_err(|e| e.to_string())?
            != imported(&validated.originals[name], &result.binding)?
        {
            return Err("installed startup report differs".into());
        }
        for descriptor in streams(&validated.reports[name]) {
            let bytes = profile::read(root, &descriptor.path, b".chrono-harness/state/")?;
            if !policy::stream_matches(
                &sha256(&bytes),
                &descriptor.sha256,
                bytes.len() as u64,
                descriptor.size,
            ) {
                return Err("installed startup original process stream differs".into());
            }
        }
        let local = self.local_sdk(root, environment, native, name, &validated.reports[name])?;
        if serde_json::to_value(result.local_executables.get(name)).map_err(|e| e.to_string())?
            != serde_json::to_value(Some(local)).map_err(|e| e.to_string())?
        {
            return Err("installed local executable provenance differs".into());
        }
        Ok(())
    }
}
