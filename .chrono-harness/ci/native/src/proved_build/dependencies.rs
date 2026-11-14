//! runner 的 Cargo 生产记录；观察与正式证明资格分别报告。
use super::{Config as BuildConfig, bounded, identity, persist, records, stream_matches};
use crate::{bindings, construction::BuildFailure, environment::Prepared, execution, profile};
use chrono_harness::{retained_artifacts::retention, verified_verifier::cargo};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub operation: String,
    pub bootstrap_profile: String,
    pub receipt: String,
    pub component: ComponentConfig,
    pub consumers: BTreeMap<String, Consumer>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentConfig {
    pub checksum: String,
    pub patch: String,
    pub source: String,
    pub receipt: String,
    pub archive_root: String,
    pub argv: Vec<String>,
    pub files: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub inputs: String,
    pub directory: String,
    pub operation: execution::Operation,
    pub files: Vec<cargo::Identity>,
    pub externs: BTreeMap<String, String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub tool: String,
    pub executable: String,
    pub argv: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    schema: String,
    inputs: String,
    metadata: execution::Operation,
    build: execution::Operation,
    files: Vec<cargo::Identity>,
    externs: BTreeMap<String, String>,
    search: Vec<String>,
    unresolved: Vec<String>,
    component: Component,
}

fn key(environment: &bindings::Environment, inputs: &[cargo::Identity]) -> Result<String, String> {
    // 与实际命令相同，能力载体不属于业务环境；只保存摘要，不复制环境原值。
    let values: Vec<_> = environment
        .iter()
        .filter(|(k, _)| *k != OsStr::new("CHRONO_PROCESS_FDS"))
        .map(|(k, v)| (k.as_encoded_bytes(), v.as_encoded_bytes()))
        .collect();
    chrono_harness::wire::digest(&(values, inputs))
}

fn observe(files: &mut BTreeMap<String, cargo::Identity>, path: &Path) -> Result<(), String> {
    let value = identity(path)?;
    if let Some(before) = files.insert(value.path.clone(), value.clone()) {
        if !cargo::content_matches(&value.sha256, value.length, &before.sha256, before.length) {
            return Err(format!(
                "E_VERUS_DEPENDENCIES: input changed: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn unchanged(files: &[cargo::Identity]) -> Result<(), String> {
    for before in files {
        let after = identity(Path::new(&before.path))?;
        if !cargo::content_matches(&after.sha256, after.length, &before.sha256, before.length) {
            return Err(format!(
                "E_VERUS_DEPENDENCIES: bytes changed: {}",
                before.path
            ));
        }
    }
    Ok(())
}

fn original_stream(root: &Path, stream: &execution::Stream) -> Result<Vec<u8>, String> {
    let path = profile::path(root, &stream.path, b".chrono-harness/state/")?;
    let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 67_108_864 {
        return Err("E_VERUS_DEPENDENCIES: original stream exceeds registered 64 MiB".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(67_108_865)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 67_108_864 {
        return Err("dependency stream grew beyond its bound".into());
    }
    stream_matches(root, stream, &bytes)?;
    Ok(bytes)
}

/// 原 Cargo JSON 选产物，不枚举 target 或猜 hash；完整原值仍保存在原 stdout。
/// metadata 决定实际直接依赖名字，root dep-info 决定这一编译实例实际装载的库。
pub fn collect(
    target: &Path,
    metadata: &Value,
    output: &[u8],
    manifest: &Path,
) -> Result<
    (
        Vec<cargo::Identity>,
        BTreeMap<String, String>,
        Vec<String>,
        Vec<String>,
    ),
    String,
> {
    let decoded = records(output)?;
    let root_id = metadata["resolve"]["root"]
        .as_str()
        .ok_or("E_VERUS_DEPENDENCIES: missing Cargo resolve root")?;
    let package = metadata["packages"]
        .as_array()
        .ok_or("missing resolved packages")?
        .iter()
        .find(|p| p["id"] == root_id)
        .ok_or("missing root package")?;
    if package["manifest_path"].as_str() != manifest.to_str() {
        return Err("E_VERUS_DEPENDENCIES: resolved root manifest differs".into());
    }
    let targets: Vec<_> = package["targets"]
        .as_array()
        .ok_or("missing root targets")?
        .iter()
        .filter(|t| t["kind"] == json!(["lib"]) && t["crate_types"] == json!(["lib"]))
        .collect();
    if targets.len() != 1 {
        return Err("E_VERUS_DEPENDENCIES: root library target is ambiguous".into());
    }
    let source = targets[0]["src_path"]
        .as_str()
        .ok_or("missing root source")?;
    let name = targets[0]["name"].as_str().ok_or("missing root name")?;
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("missing Cargo resolve nodes")?;
    let node = nodes
        .iter()
        .find(|n| n["id"] == root_id)
        .ok_or("missing root node")?;
    let mut files = BTreeMap::new();
    let mut artifacts = Vec::new();
    let mut root_library = None;
    let mut phase = 0;
    let mut unresolved = Vec::new();
    let mut search = Vec::new();
    for record in &decoded {
        let is_root = match record {
            cargo::Record::Cargo(cargo::Cargo::Artifact(a)) => {
                a.package_id == root_id
                    && cargo::artifact_selected(a, &manifest.to_string_lossy(), source, name, "lib")
            }
            _ => false,
        };
        let event = match record {
            cargo::Record::Cargo(cargo::Cargo::Artifact(_)) => {
                if is_root {
                    2
                } else {
                    1
                }
            }
            cargo::Record::Cargo(cargo::Cargo::Finished { success: true }) => 3,
            cargo::Record::Cargo(cargo::Cargo::Script { .. }) => 0,
            cargo::Record::Cargo(cargo::Cargo::Message { message })
                if !cargo::diagnostic_rejected(&message.level) =>
            {
                0
            }
            _ => 4,
        };
        phase = cargo::dependency_step(phase, event)
            .ok_or("E_VERUS_DEPENDENCIES: rejected Cargo stream order/result")?;
        match record {
            cargo::Record::Cargo(cargo::Cargo::Finished { success: true }) => {}
            cargo::Record::Cargo(cargo::Cargo::Message { message })
                if !cargo::diagnostic_rejected(&message.level) => {}
            cargo::Record::Cargo(cargo::Cargo::Artifact(artifact)) => {
                observe(&mut files, Path::new(&artifact.manifest_path))?;
                observe(&mut files, Path::new(&artifact.target.src_path))?;
                for filename in &artifact.filenames {
                    if !Path::new(filename).starts_with(target) {
                        return Err(format!(
                            "E_VERUS_DEPENDENCIES: compiler output outside runner target: {filename}"
                        ));
                    }
                    observe(&mut files, Path::new(filename))?;
                }
                if artifact.fresh {
                    unresolved.push(format!(
                        "fresh unit requires its original compiler producer: {}:{}",
                        artifact.package_id, artifact.target.name
                    ));
                }
                if is_root {
                    if root_library.replace(artifact).is_some() {
                        return Err("ambiguous root library".into());
                    }
                }
                artifacts.push(artifact);
            }
            cargo::Record::Cargo(cargo::Cargo::Script {
                package_id,
                linked_libs,
                linked_paths,
                out_dir,
                ..
            }) => {
                for lib in linked_libs {
                    if let Some(name) = lib.strip_prefix("static=") {
                        let mut selected = Vec::new();
                        for directory in linked_paths {
                            if let Some(directory) = directory.strip_prefix("native=") {
                                let path = Path::new(directory).join(format!("lib{name}.a"));
                                if path.try_exists().map_err(|e| e.to_string())? {
                                    selected.push(path);
                                }
                            }
                        }
                        if selected.len() != 1 {
                            return Err(format!(
                                "E_VERUS_DEPENDENCIES: native library {lib} has {} declared producers",
                                selected.len()
                            ));
                        }
                        if !selected[0].starts_with(target) {
                            return Err(
                                "E_VERUS_DEPENDENCIES: native output outside runner target".into(),
                            );
                        }
                        observe(&mut files, &selected[0])?;
                    } else {
                        unresolved.push(format!("native/system linker search has no exact loaded input: {package_id}:{lib}"));
                    }
                }
                // Cargo 声明的脚本接口原值保留；目录存在不证明脚本/原生编译输入闭合。
                unresolved.push(format!("build-script producer, generated inputs and native tool invocation not bound: {package_id}; out_dir={out_dir}; linked_libs={linked_libs:?}; linked_paths={linked_paths:?}"));
            }
            _ => return Err("E_VERUS_DEPENDENCIES: failed, proof or unknown Cargo record".into()),
        }
    }
    if phase != 2 {
        return Err("E_VERUS_DEPENDENCIES: missing successful build-finished".into());
    }
    let library = root_library.ok_or("missing runner library compiler-artifact")?;
    let mut root_inputs = Vec::new();
    for artifact in &artifacts {
        let mut outputs: Vec<PathBuf> = artifact.filenames.iter().map(PathBuf::from).collect();
        let first = outputs.first().ok_or("empty compiler filenames")?.clone();
        // Cargo publishes the root rlib without a hash, while rustc's dep-info names
        // the hashed sibling of the reported rmeta. Check the alias bytes explicitly.
        if let Some(meta) = outputs
            .iter()
            .find(|p| p.extension() == Some(OsStr::new("rmeta")))
            .cloned()
        {
            let hashed = meta.with_extension("rlib");
            let published = outputs
                .iter()
                .find(|p| p.extension() == Some(OsStr::new("rlib")))
                .ok_or("missing rlib")?;
            if *published != hashed {
                let actual = identity(&hashed)?;
                let alias = identity(published)?;
                if !cargo::content_matches(
                    &actual.sha256,
                    actual.length,
                    &alias.sha256,
                    alias.length,
                ) {
                    return Err(
                        "E_VERUS_DEPENDENCIES: published root differs from compiler library".into(),
                    );
                }
                observe(&mut files, &hashed)?;
                outputs = vec![hashed, meta];
            }
        }
        let directory = outputs[0]
            .parent()
            .ok_or("missing compiler output directory")?;
        let text = directory
            .to_str()
            .ok_or("non UTF-8 search path")?
            .to_owned();
        if !search.contains(&text) {
            search.push(text);
        }
        let dependency = if artifact.target.kind == ["lib"] {
            let names = outputs
                .iter()
                .map(|p| p.to_str().map(str::to_owned).ok_or("non UTF-8 output"))
                .collect::<Result<Vec<_>, _>>()?;
            let paths = cargo::compiler_library_paths(
                directory.as_os_str().as_encoded_bytes(),
                artifact.target.name.as_bytes(),
                &names,
                artifact.executable.is_some(),
            )
            .ok_or("E_VERUS_DEPENDENCIES: unsupported actual compiler library shape")?;
            PathBuf::from(std::str::from_utf8(&paths.dependency).map_err(|e| e.to_string())?)
        } else if artifact.target.kind == ["proc-macro"] {
            let stem = first
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix("lib"))
                .ok_or("invalid proc macro output")?;
            directory.join(format!("{stem}.d"))
        } else {
            unresolved.push(format!(
                "build-script source/generated/native compilation inputs not qualified: {}:{}",
                artifact.package_id, artifact.target.name
            ));
            continue;
        };
        observe(&mut files, &dependency)?;
        let raw = bounded(&dependency)?;
        if raw
            .windows(b"# checksum:".len())
            .any(|w| w == b"# checksum:")
        {
            super::bind_dependency(
                Path::new(&artifact.manifest_path),
                &dependency,
                &outputs,
                &[PathBuf::from(&artifact.target.src_path)],
            )
            .map_err(|e| {
                format!(
                    "{e}; unit={}; dep-info={}",
                    artifact.target.name,
                    dependency.display()
                )
            })?;
        } else {
            // Explicit --target does not apply target rustflags to host macro units.
            // Preserve their real dep-info and current bytes, without granting origin qualification.
            unresolved.push(format!(
                "host unit has no compiler checksum producer: {}:{}",
                artifact.package_id, artifact.target.name
            ));
        }
        let encoded = chrono_harness::verified_verifier::sources::encode_path(
            dependency.with_extension("").as_os_str().as_encoded_bytes(),
        );
        let inputs =
            chrono_harness::verified_verifier::sources::compiler_dependency_paths(&raw, &encoded)
                .ok_or("invalid compiler dependency header")?;
        let mut loaded = Vec::new();
        for input in inputs {
            let path = PathBuf::from(std::str::from_utf8(&input).map_err(|e| e.to_string())?);
            let path = if path.is_absolute() {
                path
            } else {
                Path::new(&artifact.manifest_path)
                    .parent()
                    .ok_or("missing manifest directory")?
                    .join(path)
            };
            observe(&mut files, &path)?;
            loaded.push(path);
        }
        if std::ptr::eq(*artifact, library) {
            if !raw
                .windows(b"# checksum:".len())
                .any(|w| w == b"# checksum:")
            {
                return Err(
                    "E_VERUS_DEPENDENCIES: runner target has no compiler checksum input binding"
                        .into(),
                );
            }
            root_inputs = loaded;
        }
    }
    let mut externs = BTreeMap::new();
    for dependency in node["deps"]
        .as_array()
        .ok_or("missing resolved dependencies")?
    {
        let name = dependency["name"].as_str().ok_or("missing extern name")?;
        let package = dependency["pkg"].as_str().ok_or("missing extern package")?;
        let mut matches = Vec::new();
        for artifact in &artifacts {
            if artifact.package_id != package {
                continue;
            }
            for path in &artifact.filenames {
                // rustc may consume Cargo's rmeta output while the raw linker
                // needs the paired rlib from the very same compiler-artifact.
                if path.ends_with(".rlib")
                    && (root_inputs.contains(&PathBuf::from(path))
                        || (artifact.filenames.contains(
                            &PathBuf::from(path)
                                .with_extension("rmeta")
                                .to_string_lossy()
                                .into_owned(),
                        ) && root_inputs
                            .contains(&PathBuf::from(path).with_extension("rmeta"))))
                {
                    matches.push(path.clone());
                }
            }
        }
        // Cargo 自己构建的 proc macro 和工具组件的宏是两个编译实例，不能混用。
        if name == "verus_builtin_macros" {
            continue;
        }
        if matches.len() != 1 {
            return Err(format!(
                "E_VERUS_DEPENDENCIES: extern {name} has {} actual loaded producers",
                matches.len()
            ));
        }
        if externs.insert(name.to_owned(), matches.remove(0)).is_some() {
            return Err("duplicate Cargo extern".into());
        }
    }
    // 当前 bindings 所有者仅认证 Cargo/rustc 入口；不能据此授予 backend/linker/SDK 资格。
    unresolved.push("selected Cargo/rustc entries do not bind compiler backend, target libraries, linker/config/SDK or their original compilation inputs".into());
    Ok((files.into_values().collect(), externs, search, unresolved))
}

/// 只替换当前操作声明的无路径 extern；现有 emit/import/严格参数原样保留。
pub fn arguments(
    argv: &[String],
    externs: &BTreeMap<String, String>,
    search: &[String],
) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < argv.len() {
        if argv[index] == "--extern" {
            let value = argv.get(index + 1).ok_or("missing extern argument")?;
            result.push(argv[index].clone());
            result.push(if value.contains('=') {
                value.clone()
            } else {
                format!(
                    "{value}={}",
                    externs.get(value).ok_or_else(|| format!(
                        "E_VERUS_DEPENDENCIES: missing current producer for {value}"
                    ))?
                )
            });
            index += 2;
        } else {
            result.push(argv[index].clone());
            index += 1;
        }
    }
    // 严格调用的固定五项后缀保持最后；动态搜索属于扩展参数。
    let insert = result
        .iter()
        .position(|s| s.starts_with("--emit=link="))
        .unwrap_or(result.len());
    let searches = search
        .iter()
        .flat_map(|directory| ["-L".into(), format!("dependency={directory}")]);
    result.splice(insert..insert, searches);
    Ok(result)
}

/// 门前没有执行路径；门后仍复用原执行器，不签发 ProofAdmission。
pub fn execute_consumer(
    root: &Path,
    directory: &Path,
    operation: &str,
    consumer: &Consumer,
    externs: &BTreeMap<String, String>,
    search: &[String],
    environment: &bindings::Environment,
    proof: Option<&chrono_harness::nested_proof::Declaration>,
    component_directory: &str,
    publication: Option<&retention::Publication>,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
) -> Result<(), BuildFailure> {
    let method = execution::Method {
        owner: "runner".into(),
        operation: operation.into(),
        tool: consumer.tool.clone(),
        argv: arguments(&consumer.argv, externs, search)?,
        output_contract: None,
        nested_proof: None,
    };
    let executable = profile::path(root, &consumer.executable, b".chrono-harness/state/")?;
    if let Some(proof) = proof {
        chrono_harness::verified_verifier::nested::validate_resolution(
            &proof.component_arguments(component_directory),
            &method.argv,
        )?;
        let environment = environment
            .iter()
            .map(|(name, value)| {
                Ok((
                    name.to_str()
                        .ok_or("non UTF-8 environment name")?
                        .to_owned(),
                    value
                        .to_str()
                        .ok_or("non UTF-8 environment value")?
                        .to_owned(),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        let executable_sha256 = chrono_harness::file_identity(&executable)?.0;
        let process = chrono_harness::run_process_observed_retained(
            root,
            &chrono_harness::CommandSpec {
                program: executable
                    .to_str()
                    .ok_or("non UTF-8 nested executable")?
                    .into(),
                args: method.argv.clone(),
                env: environment,
                timeout_seconds: proof.timeout_seconds,
                output_limit_bytes: proof.output_limit_bytes,
            },
            &[],
            &executable_sha256,
            &chrono_harness::ProcessEvidence {
                root,
                stdout: &directory.join("nested.stdout"),
                stderr: &directory.join("nested.stderr"),
                launch: &directory.join("nested.launch.json"),
                publication,
            },
        )?;
        stdout
            .write_all(&process.stdout_bytes)
            .map_err(|e| e.to_string())?;
        stderr
            .write_all(&process.stderr_bytes)
            .map_err(|e| e.to_string())?;
        let original = chrono_harness::nested_proof::Original::capture(
            root,
            operation,
            proof.clone(),
            method.argv,
            executable
                .to_str()
                .ok_or("non UTF-8 nested executable")?
                .into(),
            executable_sha256,
            component_directory.into(),
            process,
        );
        let path = directory.join("nested-proof-original.json");
        persist(&path, &original)?;
        let reference = identity(&path)?;
        writeln!(
            stdout,
            "\n{}",
            serde_json::to_string(&chrono_harness::nested_proof::Reference {
                reason: "chrono-nested-proof-original".into(),
                operation: operation.into(),
                path: path
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_str()
                    .ok_or("non UTF-8 nested original")?
                    .into(),
                sha256: reference.sha256,
                length: reference.length,
            })
            .map_err(|e| e.to_string())?
        )
        .map_err(|e| e.to_string())?;
        if original.process.exit_code != 0
            || original.process.failure.is_some()
            || !original.process.owned_work_joined
        {
            return Err(BuildFailure {
                message: "strict nested proof process failed".into(),
                exit_code: Some(original.process.exit_code),
            });
        }
        return Ok(());
    }
    let result = execution::run_operation(
        root,
        directory,
        0,
        &method,
        &executable,
        environment,
        stdout,
        stderr,
    )?;
    persist(&directory.join("consumer-operation.json"), &result)?;
    if result.exit_code != 0 {
        return Err(BuildFailure {
            message: "strict dependency consumer process failed".into(),
            exit_code: Some(result.exit_code),
        });
    }
    Ok(())
}

/// Raw component observations are not ProofAdmission. Recheck the original
/// process, streams, exact extern instances, output and compiler checksums.
pub fn validate_component(
    root: &Path,
    component: &Component,
    externs: &BTreeMap<String, String>,
    source: &Path,
    contract: &chrono_harness::verified_verifier::OutputContract,
) -> Result<(), String> {
    unchanged(&component.files)?;
    if component.externs != *externs || component.operation.exit_code != 0 {
        return Err(
            "E_VERUS_DEPENDENCIES: component compiled instances differ or producer failed".into(),
        );
    }
    let expected_extern = format!(
        "serde_json={}",
        externs
            .get("serde_json")
            .ok_or("missing current serde_json")?
    );
    let actual_externs = component
        .operation
        .argv
        .windows(2)
        .filter(|pair| pair[0] == "--extern")
        .map(|pair| pair[1].as_str())
        .collect::<Vec<_>>();
    if actual_externs != [expected_extern.as_str()] {
        return Err("E_VERUS_DEPENDENCIES: component actual extern invocation differs".into());
    }
    let output = original_stream(root, &component.operation.stdout)?;
    original_stream(root, &component.operation.stderr)?;
    let report: chrono_harness::verified_verifier::CompilerReport =
        serde_json::from_slice(&output).map_err(|e| e.to_string())?;
    if !chrono_harness::verified_verifier::accepts_report(&report, contract) {
        return Err("E_VERUS_DEPENDENCIES: component original report rejected".into());
    }
    let directory = profile::path(root, &component.directory, b".chrono-harness/state/")?;
    let mut required = vec![source.to_path_buf()];
    required.push(PathBuf::from(
        externs
            .get("serde_json")
            .ok_or("missing current serde_json")?,
    ));
    super::bind_dependency(
        &root.join("Cargo.toml"),
        &directory.join("libvstd.rlib.d"),
        &[directory.join("libvstd.rlib")],
        &required,
    )?;
    Ok(())
}

fn component(
    root: &Path,
    build: &BuildConfig,
    config: &ComponentConfig,
    externs: &BTreeMap<String, String>,
    search: &[String],
    environment: &bindings::Environment,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
) -> Result<Component, BuildFailure> {
    let checksum = profile::read(root, &config.checksum, b".chrono-harness/ci/")?;
    let text = std::str::from_utf8(&checksum).map_err(|e| e.to_string())?;
    let (expected, archive) = text
        .trim_end_matches('\n')
        .split_once("  ")
        .ok_or("invalid archive checksum record")?;
    let archive = profile::path(root, archive, b".chrono-harness/cache/")?;
    let archive_id = identity(&archive)?;
    if archive_id.sha256 != expected {
        return Err("E_VERUS_DEPENDENCIES: cached Verus archive checksum differs".into());
    }
    let patch = profile::path(root, &config.patch, b".chrono-harness/experiments/")?;
    let source = profile::path(root, &config.source, b"")?;
    let inputs = chrono_harness::wire::digest(&(
        config,
        &archive_id,
        identity(&patch)?,
        identity(&source)?,
        externs,
    ))?;
    let receipt = profile::path(root, &config.receipt, b"")?;
    if receipt.try_exists().map_err(|e| e.to_string())? {
        let reference: cargo::Identity =
            serde_json::from_slice(&bounded(&receipt)?).map_err(|e| e.to_string())?;
        unchanged(std::slice::from_ref(&reference))?;
        let prior: Component = serde_json::from_slice(&bounded(Path::new(&reference.path))?)
            .map_err(|e| e.to_string())?;
        if prior.inputs == inputs {
            let _guard = retention::read_guard(root, &prior.directory)?;
            validate_component(root, &prior, externs, &source, &build.output_contract)?;
            return Ok(prior);
        }
    }
    // Separate immutable slot: the prior AC/AE bytes and their consumers remain intact.
    let directory = tempfile::Builder::new()
        .prefix("runner-verus-")
        .tempdir_in(root.join(".chrono-harness/state"))
        .map_err(|e| e.to_string())?
        .keep();
    let relative = directory
        .strip_prefix(root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("non UTF-8 component")?
        .to_owned();
    let publication = retention::begin_adopted(
        root,
        "check-original",
        &directory,
        json!({"owner":"runner dependency producer","status":"in-progress"}),
    )?;
    let consumer = format!("runner-dependencies:{relative}");
    let retained = publication
        .as_ref()
        .map(|p| p.protect_consumer(&consumer))
        .transpose()?;
    writeln!(stderr, "{}", json!({"reason":"chrono-runner-component-original", "path":relative,"consumer":consumer,"original":retained})).map_err(|e| e.to_string())?;
    let result = (|| {
        let run = |index,
                   tool: &str,
                   argv: Vec<String>,
                   stdout: &mut _,
                   stderr: &mut _|
         -> Result<execution::Operation, BuildFailure> {
            let executable = if tool == "verus" {
                directory.join("verus")
            } else {
                chrono_harness::resolve_program(
                    root,
                    tool,
                    environment.get(OsStr::new("PATH")).and_then(|s| s.to_str()),
                )?
            };
            let method = execution::Method {
                owner: build.owner.clone(),
                operation: "prepare.ci-generation-dependency.runner".into(),
                tool: tool.into(),
                argv,
                output_contract: None,
                nested_proof: None,
            };
            let operation = execution::run_operation(
                root,
                &directory,
                index,
                &method,
                &executable,
                environment,
                std::io::sink(),
                stderr,
            )?;
            persist(
                &directory.join(format!("operation-{index}.json")),
                &operation,
            )?;
            if operation.exit_code != 0 {
                return Err(BuildFailure {
                    message: format!("component {tool} failed"),
                    exit_code: Some(operation.exit_code),
                });
            }
            Ok(operation)
        };
        run(
            0,
            "tar",
            vec![
                "-x".into(),
                "-f".into(),
                archive.to_str().ok_or("non UTF-8 archive")?.into(),
                "-s".into(),
                format!("|^{}|{relative}|", config.archive_root),
                "--no-same-owner".into(),
            ],
            stdout,
            stderr,
        )?;
        run(
            1,
            "patch",
            vec![
                "-f".into(),
                "-F".into(),
                "0".into(),
                "-p1".into(),
                "-d".into(),
                relative.clone(),
                "-i".into(),
                patch.to_str().ok_or("non UTF-8 patch")?.into(),
            ],
            stdout,
            stderr,
        )?;
        let component_path = directory.to_str().ok_or("non UTF-8 component path")?;
        let argv = config
            .argv
            .iter()
            .map(|s| s.replace("$COMPONENT", component_path))
            .collect::<Vec<_>>();
        let operation = run(
            2,
            "verus",
            arguments(&argv, externs, search)?,
            stdout,
            stderr,
        )?;
        let mut files = BTreeMap::new();
        for name in &config.files {
            observe(&mut files, &directory.join(name))?;
        }
        observe(&mut files, &source)?;
        observe(&mut files, &patch)?;
        observe(&mut files, &archive)?;
        let component = Component {
            inputs,
            directory: relative.clone(),
            operation,
            files: files.into_values().collect(),
            externs: externs.clone(),
        };
        validate_component(root, &component, externs, &source, &build.output_contract)?;
        persist(&directory.join("component.json"), &component)?;
        persist(&receipt, &identity(&directory.join("component.json"))?)?;
        Ok(component)
    })();
    if let Some(publication) = publication {
        publication.complete(
            json!({"owner":"runner dependency producer", "consumer":consumer,
            "status":if result.is_ok() {"observed"} else {"failed"},
            "retention":"caller releases after last consumer/diagnosis; review within seven days"}),
        )?;
    }
    result
}

pub(super) fn run(
    root: &Path,
    declaration: &str,
    consumer: &str,
    inherited: &bindings::Environment,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
    directory: &Path,
    publication: Option<&retention::Publication>,
) -> Result<(), BuildFailure> {
    let raw = profile::read(root, declaration, b".chrono-harness/ci/")?;
    let build: BuildConfig =
        serde_json::from_value(chrono_harness::json(&raw)?).map_err(|e| e.to_string())?;
    let config = build
        .dependencies
        .as_ref()
        .ok_or("missing runner dependency declaration")?;
    let bootstrap_raw = profile::read(root, &config.bootstrap_profile, b".chrono-harness/ci/")?;
    let bootstrap = profile::Profile::decode(root, &bootstrap_raw)?;
    if bootstrap.rust_toolchain != build.rust_toolchain {
        return Err("dependency toolchains differ".into());
    }
    let registry_raw = profile::read(
        root,
        &bootstrap.tool_bindings_registry,
        b".chrono-harness/ci/",
    )?;
    let native = bindings::native_platform()?;
    let registry: Value = chrono_harness::json(&registry_raw)?;
    let platform = format!("{}/{}", native.system, native.machine);
    let target = registry["platforms"][&platform]["target"]
        .as_str()
        .ok_or("missing native target")?;
    let prepared = Prepared::parse(root, &bootstrap_raw, inherited)?;
    let mut environment = prepared.environment;
    for name in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"] {
        if environment
            .get(OsStr::new(name))
            .is_some_and(|v| !v.is_empty())
        {
            return Err(format!(
                "E_VERUS_DEPENDENCIES: {name} overrides registered checksum flags"
            )
            .into());
        }
    }
    environment.insert(
        "RUSTUP_TOOLCHAIN".into(),
        build.rust_toolchain.clone().into(),
    );
    environment.insert("RUSTC_BOOTSTRAP".into(), "1".into());
    environment.insert("CARGO_TERM_COLOR".into(), "never".into());
    if let Some(incremental) = bootstrap.rust_incremental {
        environment.insert(
            "CARGO_INCREMENTAL".into(),
            if incremental { "1" } else { "0" }.into(),
        );
    }
    let search_path = environment.get(OsStr::new("PATH")).and_then(|p| p.to_str());
    let rustup = chrono_harness::resolve_program(root, "rustup", search_path)?;
    let declarations = bindings::Declarations::parse(&bootstrap_raw, &native, Some(&registry_raw))?
        .ok_or("bound Cargo and rustc required")?;
    let tools = declarations.select(
        root,
        &rustup,
        &build.rust_toolchain,
        &BTreeMap::new(),
        &environment,
    )?;
    environment.insert("RUSTC".into(), tools.rustc.path.clone().into());
    let mut inputs = Vec::new();
    for path in [
        declaration,
        &config.bootstrap_profile,
        &bootstrap.tool_bindings_registry,
        &build.manifest,
        &build.manifest.replace("Cargo.toml", "Cargo.lock"),
        &build.projects,
    ] {
        inputs.push(identity(&profile::path(root, path, b"")?)?);
    }
    inputs.extend([
        identity(Path::new(&tools.cargo.path))?,
        identity(Path::new(&tools.rustc.path))?,
    ]);
    inputs.push(identity(
        &std::env::current_exe().map_err(|e| e.to_string())?,
    )?);
    let input_key = key(&environment, &inputs)?;
    let receipt = profile::path(root, &config.receipt, b"")?;
    let manifest = profile::path(root, &build.manifest, b"")?;
    if consumer == "prepare" {
        for path in prepared.host_directories {
            fs::create_dir_all(path).map_err(|e| e.to_string())?;
        }
        let method = |argv: Vec<String>| execution::Method {
            owner: build.owner.clone(),
            operation: config.operation.clone(),
            tool: "cargo".into(),
            argv,
            output_contract: None,
            nested_proof: None,
        };
        let metadata = execution::run_operation(
            root,
            directory,
            0,
            &method(vec![
                "metadata".into(),
                "--locked".into(),
                "--format-version=1".into(),
                "--filter-platform".into(),
                target.into(),
                "--manifest-path".into(),
                build.manifest.clone(),
            ]),
            Path::new(&tools.cargo.path),
            &environment,
            &mut std::io::sink(),
            &mut *stderr,
        )?;
        persist(&directory.join("metadata-operation.json"), &metadata)?;
        if metadata.exit_code != 0 {
            return Err(BuildFailure {
                message: "Cargo dependency resolution failed".into(),
                exit_code: Some(metadata.exit_code),
            });
        }
        let graph_raw = original_stream(root, &metadata.stdout)?;
        let graph: Value = chrono_harness::json(&graph_raw)?;
        let operation = execution::run_operation(
            root,
            directory,
            1,
            &method(vec![
                "build".into(),
                "--lib".into(),
                "--locked".into(),
                "--profile=dev".into(),
                "--target".into(),
                target.into(),
                "--manifest-path".into(),
                build.manifest.clone(),
                "--target-dir".into(),
                build.target.clone(),
                "--message-format=json".into(),
                "--jobs".into(),
                "1".into(),
                "--config".into(),
                "build.rustflags=[\"-Zbinary-dep-depinfo\",\"-Zchecksum-hash-algorithm=sha256\"]"
                    .into(),
            ]),
            Path::new(&tools.cargo.path),
            &environment,
            &mut std::io::sink(),
            &mut *stderr,
        )?;
        persist(&directory.join("build-operation.json"), &operation)?;
        if operation.exit_code != 0 {
            return Err(BuildFailure {
                message: "Cargo runner dependency production failed".into(),
                exit_code: Some(operation.exit_code),
            });
        }
        let output = original_stream(root, &operation.stdout)?;
        let (mut files, externs, search, unresolved) = collect(
            &profile::path(root, &build.target, b"")?,
            &graph,
            &output,
            &manifest,
        )?;
        files.extend(inputs);
        unchanged(&files)?;
        let component = component(
            root,
            &build,
            &config.component,
            &externs,
            &search,
            &environment,
            stdout,
            stderr,
        )?;
        let observation = Observation {
            component,
            schema: "chrono-runner-dependencies/v1".into(),
            inputs: input_key,
            metadata,
            build: operation,
            files,
            externs,
            search,
            unresolved,
        };
        persist(&directory.join("dependencies.json"), &observation)?;
        persist(&receipt, &identity(&directory.join("dependencies.json"))?)?;
        writeln!(stdout, "{}", json!({"reason":"chrono-runner-dependencies", "receipt":config.receipt, "status":"observed", "formal_qualification":"unavailable", "unresolved":observation.unresolved})).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let operation = consumer;
    let methods = execution::methods(&chrono_harness::json(&profile::read(
        root,
        &build.projects,
        b".chrono-harness/",
    )?)?)?;
    let registered = methods
        .get(operation)
        .ok_or("unregistered dependency consumer")?;
    let [registered] = registered.as_slice() else {
        return Err("ambiguous dependency consumer".into());
    };
    let proof = registered.nested_proof.as_ref();
    let declared;
    let consumer = if let Some(proof) = proof {
        declared = Consumer {
            tool: proof.tool.clone(),
            executable: proof.executable.clone(),
            argv: proof.argv.clone(),
        };
        &declared
    } else {
        config
            .consumers
            .get(operation)
            .ok_or("unregistered dependency consumer")?
    };
    let reference: cargo::Identity =
        serde_json::from_slice(&bounded(&receipt)?).map_err(|e| e.to_string())?;
    unchanged(std::slice::from_ref(&reference))?;
    let evidence = Path::new(&reference.path);
    let relative = evidence
        .parent()
        .and_then(|p| p.strip_prefix(root).ok())
        .and_then(|p| p.to_str())
        .ok_or("dependency original outside host")?;
    let _guard = retention::read_guard(root, relative)?;
    let observation: Observation =
        serde_json::from_slice(&bounded(evidence)?).map_err(|e| e.to_string())?;
    if observation.schema != "chrono-runner-dependencies/v1" || observation.inputs != input_key {
        return Err(
            "E_VERUS_DEPENDENCIES: producer configuration, tool, environment or inputs differ"
                .into(),
        );
    }
    unchanged(&observation.files)?;
    if observation.metadata.exit_code != 0 || observation.build.exit_code != 0 {
        return Err("E_VERUS_DEPENDENCIES: failed original producer".into());
    }
    let graph_raw = original_stream(root, &observation.metadata.stdout)?;
    let output = original_stream(root, &observation.build.stdout)?;
    let graph = chrono_harness::json(&graph_raw)?;
    let (_, externs, search, unresolved) = collect(
        &profile::path(root, &build.target, b"")?,
        &graph,
        &output,
        &manifest,
    )?;
    if observation.externs != externs
        || observation.search != search
        || observation.unresolved != unresolved
    {
        return Err("E_VERUS_DEPENDENCIES: receipt differs from actual Cargo originals".into());
    }
    let _component_guard = retention::read_guard(root, &observation.component.directory)?;
    validate_component(
        root,
        &observation.component,
        &observation.externs,
        &profile::path(root, &config.component.source, b"")?,
        &build.output_contract,
    )?;
    // These are registered raw investigations. Known typed qualification gaps are
    // retained in the observation; they cannot issue a formal ProofAdmission.
    let consumer = Consumer {
        tool: consumer.tool.clone(),
        executable: consumer
            .executable
            .replace("$COMPONENT", &observation.component.directory),
        argv: consumer
            .argv
            .iter()
            .map(|s| s.replace("$COMPONENT", &observation.component.directory))
            .collect(),
    };
    execute_consumer(
        root,
        directory,
        operation,
        &consumer,
        &externs,
        &search,
        &environment,
        proof,
        &observation.component.directory,
        publication,
        stdout,
        stderr,
    )
}
