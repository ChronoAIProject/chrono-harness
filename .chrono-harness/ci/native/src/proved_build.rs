//! 正式构建消费官方 Cargo-Verus 的原输出、编译器 checksum 和原库字节。
use crate::{bindings::Environment, construction::BuildFailure, execution, profile};
use chrono_harness::{file_identity, retained_artifacts::retention, verified_verifier as verifier};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use verifier::{
    OutputContract,
    cargo::{self, Artifact, BuildTargets, Cargo, Identity, Record, Recovery, TargetIdentity},
    sources::{self, SourceBinding},
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    pub manifest: String,
    pub source: String,
    pub target: String,
    pub sources: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binary {
    pub source: String,
    pub target: String,
    pub published: String,
    pub dependency_prefix: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub owner: String,
    pub operation: String,
    pub projects: String,
    pub recovery_operation: String,
    pub manifest: String,
    pub target: String,
    pub cargo_verus: String,
    pub verus_directory: String,
    pub rust_toolchain: String,
    pub output_contract: OutputContract,
    pub library: Library,
    pub binary: Binary,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    schema: String,
    inputs: String,
    operation: execution::Operation,
    stdout: String,
    stderr: Vec<u8>,
    artifact: Artifact,
    libraries: Vec<Identity>,
    dependency: String,
}

#[derive(Serialize)]
struct Admission {
    schema: &'static str,
    proof_cache: Identity,
    library: Vec<Identity>,
    binary: Identity,
    proof_reused: bool,
}

fn bounded(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !cargo::bounded_original(metadata.file_type().is_file(), metadata.len()) {
        return Err(format!(
            "E_VERUS_BUILD: original exceeds its registered bound: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(8_388_609)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if !cargo::bounded_original(true, bytes.len() as u64) {
        return Err(format!(
            "E_VERUS_BUILD: original exceeds its registered bound: {}",
            path.display()
        ));
    }
    Ok(bytes)
}

fn identity(path: &Path) -> Result<Identity, String> {
    let (sha256, length) = file_identity(path)?;
    Ok(Identity {
        path: path.to_str().ok_or("non UTF-8 compiler artifact")?.into(),
        sha256,
        length,
    })
}

fn preserve_if_present(source: &Path, destination: &Path) -> Result<(), String> {
    match fs::symlink_metadata(source) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
        Ok(_) => fs::write(destination, bounded(source)?).map_err(|e| e.to_string()),
    }
}

fn persist(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path.parent().ok_or("missing proof parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer(temporary.as_file_mut(), value).map_err(|e| e.to_string())?;
    temporary.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn records(raw: &[u8]) -> Result<Vec<Record>, String> {
    serde_json::Deserializer::from_slice(raw)
        .into_iter::<Record>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("E_VERUS_BUILD: complete Cargo stdout rejected: {e}"))
}

fn selected(artifact: &Artifact, manifest: &Path, source: &Path, name: &str, kind: &str) -> bool {
    cargo::artifact_selected(
        artifact,
        &manifest.to_string_lossy(),
        &source.to_string_lossy(),
        name,
        kind,
    )
}

fn library_paths(
    root: &Path,
    config: &Config,
    artifact: &Artifact,
) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let directory = profile::path(root, &config.target, b"")?.join("debug/deps");
    let paths = cargo::compiler_library_paths(
        directory.as_os_str().as_encoded_bytes(),
        config.library.target.as_bytes(),
        &artifact.filenames,
        artifact.executable.is_some(),
    )
    .ok_or("E_VERUS_BUILD: compiler library filenames do not match the registered target")?;
    let path = |bytes: &[u8]| -> Result<PathBuf, String> {
        Ok(PathBuf::from(
            std::str::from_utf8(bytes).map_err(|e| e.to_string())?,
        ))
    };
    Ok((
        path(&paths.dependency)?,
        paths
            .outputs
            .iter()
            .map(|bytes| path(bytes))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn bind_dependency(
    manifest: &Path,
    dependency: &Path,
    outputs: &[PathBuf],
    required: &[PathBuf],
) -> Result<Vec<u8>, String> {
    let raw = bounded(dependency)?;
    let stem = dependency.with_extension("");
    let encoded = sources::encode_path(stem.as_os_str().as_encoded_bytes());
    let paths = sources::compiler_dependency_paths(&raw, &encoded)
        .ok_or("E_VERUS_BUILD: compiler dependency header rejected")?;
    let mut bindings = Vec::new();
    let mut seen = vec![false; required.len()];
    for path in paths {
        let text = std::str::from_utf8(&path).map_err(|e| e.to_string())?;
        let absolute = if Path::new(text).is_absolute() {
            PathBuf::from(text)
        } else {
            manifest
                .parent()
                .ok_or("missing compiler working directory")?
                .join(text)
        };
        let observed = identity(&absolute)?;
        for (index, expected) in required.iter().enumerate() {
            if &absolute == expected {
                seen[index] = true;
            }
        }
        bindings.push(SourceBinding {
            path,
            sha256: observed.sha256.into_bytes(),
            length: observed.length,
        });
    }
    let mut targets = vec![dependency.as_os_str().as_encoded_bytes().to_vec()];
    targets.extend(
        outputs
            .iter()
            .map(|p| p.as_os_str().as_encoded_bytes().to_vec()),
    );
    if !cargo::all_present(&seen) || !sources::matches_cargo_dependency(&raw, &targets, &bindings) {
        return Err("E_VERUS_BUILD: compiler source or binary checksums differ".into());
    }
    Ok(raw)
}

fn stream_matches(root: &Path, stream: &execution::Stream, bytes: &[u8]) -> Result<(), String> {
    if !cargo::content_matches(
        &chrono_harness::sha256(bytes),
        bytes.len() as u64,
        &stream.sha256,
        stream.size,
    ) {
        return Err("E_VERUS_BUILD: original stream identity differs".into());
    }
    profile::path(root, &stream.path, b".chrono-harness/state/")?;
    Ok(())
}

/// 官方构建进程仍由既有 runner 执行；这里不实现编译器 wrapper。
pub fn build(
    root: &Path,
    declaration: &str,
    inherited: &Environment,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
) -> Result<(), BuildFailure> {
    build_attempt(root, declaration, inherited, stdout, stderr, true)
}

fn build_attempt(
    root: &Path,
    declaration: &str,
    inherited: &Environment,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
    can_recover: bool,
) -> Result<(), BuildFailure> {
    let parent = profile::path(
        root,
        ".chrono-harness/state/proved-builds",
        b".chrono-harness/state/",
    )?;
    fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    let temporary = tempfile::Builder::new()
        .prefix("build-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let result = run_attempt(
        root,
        declaration,
        inherited,
        stdout,
        stderr,
        can_recover,
        temporary.path(),
    );
    let rejected = temporary
        .path()
        .join("cache-rejected.json")
        .try_exists()
        .unwrap_or(true);
    if cargo::retain_attempt(result.is_ok(), rejected) {
        let directory = temporary.keep();
        let retained = (|| -> Result<(), String> {
            let relative = directory
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("non UTF-8 original directory")?;
            let consumer = format!("proved-build-repair:{relative}");
            let outcome = serde_json::json!({"owner":"registered build caller", "consumer":consumer,
                "status":if result.is_ok() {"recovered"} else {"failed"},
                "retention":"release after diagnosis/acceptance; rebuildable originals reclaimed within seven days after consumers end"});
            let declaration = if let Some(publication) =
                retention::begin_adopted(root, "check-original", &directory, outcome.clone())?
            {
                let original = publication.protect_consumer(&consumer)?;
                let declaration = retention::Consumer {
                    schema: "chrono-retention-consumer/v1".into(),
                    consumer: consumer.clone(),
                    purpose:
                        "registered build caller diagnosis and acceptance; release when consumed"
                            .into(),
                    originals: vec![original],
                };
                publication.complete(outcome)?;
                Some(declaration)
            } else {
                None
            };
            writeln!(stderr, "{}", serde_json::json!({"reason":"chrono-proved-build-original", "path":relative,"consumer":consumer,"report_consumer":declaration})).map_err(|e| e.to_string())?;
            Ok(())
        })();
        match (result, retained) {
            (Err(mut failure), Err(note)) => {
                failure.message += &format!(
                    "; original retention failed: {note}; preserve {}",
                    directory.display()
                );
                Err(failure)
            }
            (Err(mut failure), Ok(())) => {
                failure.message += &format!("; original: {}", directory.display());
                Err(failure)
            }
            (Ok(()), Err(note)) => Err(note.into()),
            (Ok(()), Ok(())) => Ok(()),
        }
    } else {
        temporary.close().map_err(|e| e.to_string())?;
        result
    }
}

fn run_attempt(
    root: &Path,
    declaration: &str,
    inherited: &Environment,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
    can_recover: bool,
    directory: &Path,
) -> Result<(), BuildFailure> {
    let raw = profile::read(root, declaration, b".chrono-harness/ci/")?;
    let config: Config =
        serde_json::from_value(chrono_harness::json(&raw)?).map_err(|e| e.to_string())?;
    if !cargo::registered_sources(
        &config.schema,
        &config.library.source,
        &config.library.sources,
    ) {
        return Err("E_VERUS_BUILD: invalid build registration".into());
    }
    verifier::validate_contract(&Some(config.output_contract.clone()))?;
    let manifest = profile::path(root, &config.manifest, b"")?;
    let target = profile::path(root, &config.target, b"")?;
    let verus = profile::path(root, &config.verus_directory, b".chrono-harness/bin/")?;
    let executable = profile::path(root, &config.cargo_verus, b".chrono-harness/bin/")?;
    let projects = profile::read(root, &config.projects, b".chrono-harness/")?;
    let mut methods = execution::methods(&chrono_harness::json(&projects)?)?;
    let mut recovery = methods
        .remove(&config.recovery_operation)
        .ok_or("E_VERUS_BUILD: missing registered cache recovery")?;
    if !crate::verified_bindings::unique_method(recovery.len()) {
        return Err("E_VERUS_BUILD: ambiguous registered cache recovery".into());
    }
    let recovery = recovery.pop().ok_or("missing registered cache recovery")?;
    let rustflags = vec![
        "-Zbinary-dep-depinfo".to_owned(),
        "-Zchecksum-hash-algorithm=sha256".to_owned(),
        "--extern".to_owned(),
        format!("vstd={}", verus.join("libvstd.rlib").display()),
        "-L".to_owned(),
        format!("dependency={}", verus.display()),
    ];
    let method = execution::Method {
        owner: config.owner.clone(),
        operation: config.operation.clone(),
        tool: "cargo-verus".into(),
        argv: vec![
            "build".into(),
            "-v".into(),
            "--fwd-verus-args-to".into(),
            "all".into(),
            "--locked".into(),
            "--manifest-path".into(),
            config.manifest.clone(),
            "--target-dir".into(),
            config.target.clone(),
            "--config".into(),
            format!(
                "build.rustflags={}",
                serde_json::to_string(&rustflags).map_err(|e| e.to_string())?
            ),
            "--message-format=json".into(),
            // Verus 报告没有目标标识，须防止并行编译插入其他库的产物记录。
            // Cargo 专用参数须在 Verus 相关 Cargo 参数之后。
            "--jobs".into(),
            "1".into(),
            "--".into(),
            "--import".into(),
            format!("vstd={}", verus.join("vstd.vir").display()),
            "--no-cheating".into(),
            "--no-external-by-default".into(),
            "--num-threads".into(),
            "2".into(),
            "--output-json".into(),
            "-Zchecksum-hash-algorithm=sha256".into(),
        ],
        output_contract: None,
    };
    let mut environment = inherited.clone();
    environment.insert(
        "RUSTUP_TOOLCHAIN".into(),
        config.rust_toolchain.clone().into(),
    );
    environment.insert("RUSTC_BOOTSTRAP".into(), "1".into());
    let mut tools = BTreeMap::new();
    for name in [
        "cargo-verus",
        "verus",
        "rust_verify",
        "z3",
        "libvstd.rlib",
        "vstd.vir",
    ] {
        let file = verus.join(name);
        tools.insert(file.clone(), file_identity(&file)?);
    }
    tools.insert(executable.clone(), file_identity(&executable)?);
    let required = config
        .library
        .sources
        .iter()
        .map(|p| profile::path(root, p, b""))
        .collect::<Result<Vec<_>, _>>()?;
    let mut inputs = BTreeMap::new();
    for source in required.iter().chain([
        &manifest,
        &profile::path(root, &config.library.manifest, b"")?,
        &profile::path(root, declaration, b".chrono-harness/ci/")?,
    ]) {
        inputs.insert(
            source.to_string_lossy().into_owned(),
            file_identity(source)?,
        );
    }
    let key = chrono_harness::wire::digest(&(
        chrono_harness::sha256(&raw),
        &tools,
        &inputs,
        &method.argv,
        &recovery,
    ))?;
    let operation = execution::run_operation(
        root,
        &directory,
        0,
        &method,
        &executable,
        &environment,
        &mut *stdout,
        &mut *stderr,
    )?;
    persist(&directory.join("operation.json"), &operation)?;
    if operation.exit_code != 0 {
        return Err(BuildFailure {
            message: "E_VERUS_BUILD: official compiler process failed".into(),
            exit_code: Some(operation.exit_code),
        });
    }
    let output = bounded(&root.join(&operation.stdout.path))?;
    let errors = bounded(&root.join(&operation.stderr.path))?;
    stream_matches(root, &operation.stdout, &output)?;
    stream_matches(root, &operation.stderr, &errors)?;
    let cache = target.join(".chrono-verus-proof.json");
    let prior = bounded(&cache)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Proof>(&bytes).ok());
    let library_manifest = profile::path(root, &config.library.manifest, b"")?;
    let library_source = profile::path(root, &config.library.source, b"")?;
    let binary_source = profile::path(root, &config.binary.source, b"")?;
    let targets = BuildTargets {
        library: TargetIdentity {
            manifest: library_manifest
                .to_str()
                .ok_or("non UTF-8 library manifest")?
                .into(),
            source: library_source
                .to_str()
                .ok_or("non UTF-8 library source")?
                .into(),
            name: config.library.target.clone(),
        },
        binary: TargetIdentity {
            manifest: manifest.to_str().ok_or("non UTF-8 binary manifest")?.into(),
            source: binary_source
                .to_str()
                .ok_or("non UTF-8 binary source")?
                .into(),
            name: config.binary.target.clone(),
        },
        published: profile::path(root, &config.binary.published, b"")?
            .to_str()
            .ok_or("non UTF-8 binary")?
            .into(),
    };
    let decoded = records(&output)?;
    if !cargo::complete_records(&decoded, &targets, &config.output_contract) {
        return Err("E_VERUS_BUILD: missing or invalid complete original build result".into());
    }
    let mut library = None;
    let mut binary = None;
    let mut reused = false;
    for record in decoded {
        match record {
            Record::Cargo(Cargo::Artifact(artifact)) => {
                if selected(
                    &artifact,
                    &library_manifest,
                    &library_source,
                    &config.library.target,
                    "lib",
                ) {
                    let original: Result<(Vec<Identity>, Vec<u8>), String> = (|| {
                        let (dependency, outputs) = library_paths(root, &config, &artifact)?;
                        let metadata =
                            bind_dependency(&library_manifest, &dependency, &outputs, &required)?;
                        let identities = outputs
                            .iter()
                            .map(|p| identity(p))
                            .collect::<Result<Vec<_>, _>>()?;
                        if !cargo::same_identities(&identities, &identities) {
                            return Err("E_VERUS_BUILD: invalid library identities".into());
                        }
                        if artifact.fresh {
                            let proof = prior.as_ref().ok_or(
                                "E_VERUS_BUILD: fresh library has no retained original proof",
                            )?;
                            validate_prior(
                                root,
                                &config,
                                proof,
                                &key,
                                &metadata,
                                &identities,
                                &method,
                                &executable,
                                &targets,
                            )?;
                        }
                        Ok((identities, metadata))
                    })(
                    );
                    let (identities, metadata) = match original {
                        Ok(observation) => observation,
                        Err(error) => {
                            match cargo::recovery(artifact.fresh, false, can_recover) {
                                Recovery::Continue | Recovery::Reject => return Err(error.into()),
                                Recovery::Rebuild => {}
                            }
                            persist(
                                &directory.join("cache-rejected.json"),
                                &serde_json::json!({"error":error,"operation":operation,"recovery_operation":recovery.operation}),
                            )?;
                            preserve_if_present(&cache, &directory.join("rejected-proof.json"))?;
                            if let Ok((dependency, outputs)) =
                                library_paths(root, &config, &artifact)
                            {
                                preserve_if_present(
                                    &dependency,
                                    &directory.join("rejected-dependency.d"),
                                )?;
                                for output in outputs {
                                    let name =
                                        output.file_name().ok_or("missing library filename")?;
                                    preserve_if_present(&output, &directory.join(name))?;
                                }
                            }
                            let search = environment
                                .get(std::ffi::OsStr::new("PATH"))
                                .and_then(|s| s.to_str());
                            let cargo =
                                chrono_harness::resolve_program(root, &recovery.tool, search)?;
                            let recovered = execution::run_operation(
                                root,
                                &directory,
                                1,
                                &recovery,
                                &cargo,
                                &environment,
                                &mut *stdout,
                                &mut *stderr,
                            )?;
                            persist(&directory.join("recovery.json"), &recovered)?;
                            if recovered.exit_code != 0 {
                                return Err(BuildFailure {
                                    message: "E_VERUS_BUILD: registered cache recovery failed"
                                        .into(),
                                    exit_code: Some(recovered.exit_code),
                                });
                            }
                            return build_attempt(
                                root,
                                declaration,
                                inherited,
                                stdout,
                                stderr,
                                false,
                            );
                        }
                    };
                    reused = artifact.fresh;
                    library = Some((artifact, identities, metadata));
                } else if selected(
                    &artifact,
                    &manifest,
                    &binary_source,
                    &config.binary.target,
                    "bin",
                ) {
                    let published = profile::path(root, &config.binary.published, b"")?;
                    let expected = published.to_str().ok_or("non UTF-8 binary path")?;
                    if !cargo::binary_shape(&artifact, expected) {
                        return Err(
                            "E_VERUS_BUILD: published binary differs from the registered target"
                                .into(),
                        );
                    }
                    let observed = identity(&published)?;
                    if !cargo::same_identity(&observed, &observed) {
                        return Err("E_VERUS_BUILD: invalid binary identity".into());
                    }
                    binary = Some(observed);
                }
            }
            _ => {}
        }
    }
    let (artifact, libraries, dependency) =
        library.ok_or("E_VERUS_BUILD: missing proved library")?;
    let binary = binary.ok_or("E_VERUS_BUILD: missing final binary")?;
    bind_final_binary(root, &config, &manifest, &binary, &libraries)?;
    for (source, before) in &inputs {
        let after = file_identity(Path::new(source))?;
        if !cargo::content_matches(&after.0, after.1, &before.0, before.1) {
            return Err("E_VERUS_BUILD: source changed during compilation".into());
        }
    }
    for (tool, before) in &tools {
        let after = file_identity(tool)?;
        if !cargo::content_matches(&after.0, after.1, &before.0, before.1) {
            return Err("E_VERUS_BUILD: compiler tool changed during compilation".into());
        }
    }
    if !reused {
        let proof = Proof {
            schema: "chrono-library-proof/v1".into(),
            inputs: key,
            operation: operation.clone(),
            stdout: String::from_utf8(output).map_err(|e| e.to_string())?,
            stderr: errors,
            artifact,
            libraries: libraries.clone(),
            dependency: String::from_utf8(dependency).map_err(|e| e.to_string())?,
        };
        persist(&cache, &proof)?;
    }
    let admission = Admission {
        schema: "chrono-proved-build-admission/v1",
        proof_cache: identity(&cache)?,
        library: libraries,
        binary,
        proof_reused: reused,
    };
    writeln!(
        stdout,
        "{}",
        serde_json::to_string(&admission).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn validate_prior(
    root: &Path,
    config: &Config,
    proof: &Proof,
    key: &str,
    dependency: &[u8],
    libraries: &Vec<Identity>,
    method: &execution::Method,
    executable: &Path,
    targets: &BuildTargets,
) -> Result<(), String> {
    if !cargo::retained_header(
        &proof.schema,
        &proof.inputs,
        key,
        proof.operation.exit_code,
        proof.artifact.fresh,
        proof.dependency.as_bytes(),
        dependency,
    ) || !cargo::same_identities(&proof.libraries, libraries)
    {
        return Err("E_VERUS_BUILD: retained proof inputs or library bytes differ".into());
    }
    let mut argv = vec![
        executable
            .to_str()
            .ok_or("non UTF-8 compiler executable")?
            .to_owned(),
    ];
    argv.extend(method.argv.iter().cloned());
    if !crate::verified_bindings::operation_record_matches(
        &proof.operation.tool,
        &method.tool,
        &proof.operation.argv,
        &argv,
        &proof.operation.cwd,
        root.to_str().ok_or("non UTF-8 host root")?,
        proof.operation.pid,
        proof.operation.exit_code,
    ) || !chrono_harness::verified_identifiers::equal_text(
        &proof.operation.executable_sha256,
        &file_identity(executable)?.0,
    ) || !chrono_harness::verified_identifiers::equal_text(
        &proof.operation.operation,
        &config.operation,
    ) || proof.operation.reused_from.is_some()
    {
        return Err("E_VERUS_BUILD: retained compiler process identity differs".into());
    }
    stream_matches(root, &proof.operation.stdout, proof.stdout.as_bytes())?;
    stream_matches(root, &proof.operation.stderr, &proof.stderr)?;
    let decoded = records(proof.stdout.as_bytes())?;
    if !cargo::complete_records(&decoded, targets, &config.output_contract) {
        return Err("E_VERUS_BUILD: retained proof has no complete original result".into());
    }
    for record in &decoded {
        if let Record::Cargo(Cargo::Artifact(artifact)) = record {
            if cargo::target_selected(artifact, &targets.library, "lib")
                && !cargo::same_artifact(artifact, &proof.artifact)
            {
                return Err(
                    "E_VERUS_BUILD: retained library does not match original Cargo output".into(),
                );
            }
        }
    }
    Ok(())
}

fn bind_final_binary(
    root: &Path,
    config: &Config,
    manifest: &Path,
    binary: &Identity,
    libraries: &[Identity],
) -> Result<(), String> {
    let directory = profile::path(root, &config.target, b"")?.join("debug/deps");
    let mut found = false;
    let mut required = vec![profile::path(root, &config.binary.source, b"")?];
    required.extend(libraries.iter().map(|file| PathBuf::from(&file.path)));
    let mut count = 0;
    for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
        count += 1;
        if !cargo::output_inventory_within_bound(count) {
            return Err(
                "E_VERUS_BUILD: compiler output inventory exceeds its registered bound".into(),
            );
        }
        let path = entry.map_err(|e| e.to_string())?.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !cargo::selected_dependency(name.as_bytes(), config.binary.dependency_prefix.as_bytes())
        {
            continue;
        }
        let output = path.with_extension("");
        if !output.is_file() {
            continue;
        }
        let observed = identity(&output)?;
        if !cargo::same_content(&observed, binary) {
            continue;
        }
        bind_dependency(manifest, &path, &[output], &required)?;
        found = true;
    }
    if !found {
        return Err(
            "E_VERUS_BUILD: final binary has no matching compiler checksum original".into(),
        );
    }
    Ok(())
}
