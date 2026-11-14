//! runner 的 Cargo 生产记录；观察与正式证明资格分别报告。
use super::{bounded, identity, persist, records, stream_matches, Config as BuildConfig};
use crate::{bindings, construction::BuildFailure, environment::Prepared, execution, profile};
use chrono_harness::{retained_artifacts::retention, verified_verifier::cargo};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, ffi::OsStr, fs, io::{Read, Write}, path::{Path, PathBuf}};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub operation: String,
    pub bootstrap_profile: String,
    pub receipt: String,
    pub component_producer_root: String,
    pub component_producer: cargo::Identity,
    pub component_dependency: cargo::Identity,
    pub consumers: BTreeMap<String, Consumer>,
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
}

fn key(environment: &bindings::Environment, inputs: &[cargo::Identity]) -> Result<String, String> {
    // 与实际命令相同，能力载体不属于业务环境；只保存摘要，不复制环境原值。
    let values: Vec<_> = environment.iter()
        .filter(|(k, _)| *k != OsStr::new("CHRONO_PROCESS_FDS"))
        .map(|(k, v)| (k.as_encoded_bytes(), v.as_encoded_bytes())).collect();
    chrono_harness::wire::digest(&(values, inputs))
}

fn observe(files: &mut BTreeMap<String, cargo::Identity>, path: &Path) -> Result<(), String> {
    let value = identity(path)?;
    if let Some(before) = files.insert(value.path.clone(), value.clone()) {
        if !cargo::content_matches(&value.sha256, value.length, &before.sha256, before.length) {
            return Err(format!("E_VERUS_DEPENDENCIES: input changed: {}", path.display()));
        }
    }
    Ok(())
}

fn unchanged(files: &[cargo::Identity]) -> Result<(), String> {
    for before in files {
        let after = identity(Path::new(&before.path))?;
        if !cargo::content_matches(&after.sha256, after.length, &before.sha256, before.length) {
            return Err(format!("E_VERUS_DEPENDENCIES: bytes changed: {}", before.path));
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
    fs::File::open(path).map_err(|e| e.to_string())?.take(67_108_865)
        .read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > 67_108_864 { return Err("dependency stream grew beyond its bound".into()); }
    stream_matches(root, stream, &bytes)?;
    Ok(bytes)
}

/// 原 Cargo JSON 选产物，不枚举 target 或猜 hash；完整原值仍保存在原 stdout。
/// metadata 决定实际直接依赖名字，root dep-info 决定这一编译实例实际装载的库。
pub fn collect(
    target: &Path, metadata: &Value, output: &[u8], manifest: &Path,
) -> Result<(Vec<cargo::Identity>, BTreeMap<String, String>, Vec<String>, Vec<String>), String> {
    let decoded = records(output)?;
    let root_id = metadata["resolve"]["root"].as_str().ok_or("E_VERUS_DEPENDENCIES: missing Cargo resolve root")?;
    let package = metadata["packages"].as_array().ok_or("missing resolved packages")?
        .iter().find(|p| p["id"] == root_id).ok_or("missing root package")?;
    if package["manifest_path"].as_str() != manifest.to_str() {
        return Err("E_VERUS_DEPENDENCIES: resolved root manifest differs".into());
    }
    let targets: Vec<_> = package["targets"].as_array().ok_or("missing root targets")?
        .iter().filter(|t| t["kind"] == json!(["lib"]) && t["crate_types"] == json!(["lib"])).collect();
    if targets.len() != 1 { return Err("E_VERUS_DEPENDENCIES: root library target is ambiguous".into()); }
    let source = targets[0]["src_path"].as_str().ok_or("missing root source")?;
    let name = targets[0]["name"].as_str().ok_or("missing root name")?;
    let nodes = metadata["resolve"]["nodes"].as_array().ok_or("missing Cargo resolve nodes")?;
    let node = nodes.iter().find(|n| n["id"] == root_id).ok_or("missing root node")?;
    let mut files = BTreeMap::new();
    let mut artifacts = Vec::new();
    let mut root_library = None;
    let mut phase = 0;
    let mut unresolved = Vec::new();
    let mut search = Vec::new();
    for record in &decoded {
        let is_root = match record {
            cargo::Record::Cargo(cargo::Cargo::Artifact(a)) => a.package_id == root_id
                && cargo::artifact_selected(a, &manifest.to_string_lossy(), source, name, "lib"),
            _ => false,
        };
        let event = match record {
            cargo::Record::Cargo(cargo::Cargo::Artifact(_)) => if is_root { 2 } else { 1 },
            cargo::Record::Cargo(cargo::Cargo::Finished { success: true }) => 3,
            cargo::Record::Cargo(cargo::Cargo::Script { .. }) => 0,
            cargo::Record::Cargo(cargo::Cargo::Message { message })
                if !cargo::diagnostic_rejected(&message.level) => 0,
            _ => 4,
        };
        phase = cargo::dependency_step(phase, event)
            .ok_or("E_VERUS_DEPENDENCIES: rejected Cargo stream order/result")?;
        match record {
            cargo::Record::Cargo(cargo::Cargo::Finished { success: true }) => {},
            cargo::Record::Cargo(cargo::Cargo::Message { message })
                if !cargo::diagnostic_rejected(&message.level) => {},
            cargo::Record::Cargo(cargo::Cargo::Artifact(artifact)) => {
                observe(&mut files, Path::new(&artifact.manifest_path))?;
                observe(&mut files, Path::new(&artifact.target.src_path))?;
                for filename in &artifact.filenames {
                    if !Path::new(filename).starts_with(target) {
                        return Err(format!("E_VERUS_DEPENDENCIES: compiler output outside runner target: {filename}"));
                    }
                    observe(&mut files, Path::new(filename))?;
                }
                if artifact.fresh {
                    unresolved.push(format!("fresh unit requires its original compiler producer: {}:{}", artifact.package_id, artifact.target.name));
                }
                if is_root {
                    if root_library.replace(artifact).is_some() { return Err("ambiguous root library".into()); }
                }
                artifacts.push(artifact);
            },
            cargo::Record::Cargo(cargo::Cargo::Script { package_id, linked_libs, linked_paths, out_dir, .. }) => {
                for lib in linked_libs {
                    if let Some(name) = lib.strip_prefix("static=") {
                        let mut selected = Vec::new();
                        for directory in linked_paths {
                            if let Some(directory) = directory.strip_prefix("native=") {
                                let path = Path::new(directory).join(format!("lib{name}.a"));
                                if path.try_exists().map_err(|e| e.to_string())? { selected.push(path); }
                            }
                        }
                        if selected.len() != 1 {
                            return Err(format!("E_VERUS_DEPENDENCIES: native library {lib} has {} declared producers", selected.len()));
                        }
                        if !selected[0].starts_with(target) {
                            return Err("E_VERUS_DEPENDENCIES: native output outside runner target".into());
                        }
                        observe(&mut files, &selected[0])?;
                    } else {
                        unresolved.push(format!("native/system linker search has no exact loaded input: {package_id}:{lib}"));
                    }
                }
                // Cargo 声明的脚本接口原值保留；目录存在不证明脚本/原生编译输入闭合。
                unresolved.push(format!("build-script producer, generated inputs and native tool invocation not bound: {package_id}; out_dir={out_dir}; linked_libs={linked_libs:?}; linked_paths={linked_paths:?}"));
            },
            _ => return Err("E_VERUS_DEPENDENCIES: failed, proof or unknown Cargo record".into()),
        }
    }
    if phase != 2 { return Err("E_VERUS_DEPENDENCIES: missing successful build-finished".into()); }
    let library = root_library.ok_or("missing runner library compiler-artifact")?;
    let mut root_inputs = Vec::new();
    for artifact in &artifacts {
        if artifact.target.kind != ["lib"] {
            unresolved.push(format!("macro/build-script dep-info and loaded tool domain not bound: {}:{}", artifact.package_id, artifact.target.name));
            continue;
        }
        let first = artifact.filenames.first().ok_or("empty library filenames")?;
        let directory = Path::new(first).parent().ok_or("missing library directory")?;
        let paths = cargo::compiler_library_paths(directory.as_os_str().as_encoded_bytes(),
            artifact.target.name.as_bytes(), &artifact.filenames, artifact.executable.is_some())
            .ok_or("E_VERUS_DEPENDENCIES: unsupported actual compiler library shape")?;
        let dependency = PathBuf::from(std::str::from_utf8(&paths.dependency).map_err(|e| e.to_string())?);
        let outputs = paths.outputs.iter().map(|p| std::str::from_utf8(p).map(PathBuf::from).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        // 同一现役 checksum 判法，不把 fresh 或文件存在当作编译来源。
        super::bind_dependency(Path::new(&artifact.manifest_path), &dependency, &outputs,
            &[PathBuf::from(&artifact.target.src_path)])?;
        observe(&mut files, &dependency)?;
        let raw = bounded(&dependency)?;
        let encoded = chrono_harness::verified_verifier::sources::encode_path(dependency.with_extension("").as_os_str().as_encoded_bytes());
        let inputs = chrono_harness::verified_verifier::sources::compiler_dependency_paths(&raw, &encoded)
            .ok_or("invalid compiler dependency header")?;
        let mut loaded = Vec::new();
        for input in inputs {
            let path = PathBuf::from(std::str::from_utf8(&input).map_err(|e| e.to_string())?);
            let path = if path.is_absolute() { path } else { Path::new(&artifact.manifest_path).parent().ok_or("missing manifest directory")?.join(path) };
            observe(&mut files, &path)?;
            loaded.push(path);
        }
        if std::ptr::eq(*artifact, library) { root_inputs = loaded; }
        let text = directory.to_str().ok_or("non UTF-8 search path")?.to_owned();
        if !search.contains(&text) { search.push(text); }
    }
    let mut externs = BTreeMap::new();
    for dependency in node["deps"].as_array().ok_or("missing resolved dependencies")? {
        let name = dependency["name"].as_str().ok_or("missing extern name")?;
        let package = dependency["pkg"].as_str().ok_or("missing extern package")?;
        let mut matches = Vec::new();
        for artifact in &artifacts {
            if artifact.package_id != package { continue; }
            for path in &artifact.filenames {
                if root_inputs.contains(&PathBuf::from(path)) && path.ends_with(".rlib") {
                    matches.push(path.clone());
                }
            }
        }
        // Cargo 自己构建的 proc macro 和工具组件的宏是两个编译实例，不能混用。
        if name == "verus_builtin_macros" { continue; }
        if matches.len() != 1 { return Err(format!("E_VERUS_DEPENDENCIES: extern {name} has {} actual loaded producers", matches.len())); }
        if externs.insert(name.to_owned(), matches.remove(0)).is_some() { return Err("duplicate Cargo extern".into()); }
    }
    // 当前 bindings 所有者仅认证 Cargo/rustc 入口；不能据此授予 backend/linker/SDK 资格。
    unresolved.push("selected Cargo/rustc entries do not bind compiler backend, target libraries, linker/config/SDK or their original compilation inputs".into());
    Ok((files.into_values().collect(), externs, search, unresolved))
}

/// 只替换当前操作声明的无路径 extern；现有 emit/import/严格参数原样保留。
pub fn arguments(argv: &[String], externs: &BTreeMap<String, String>, search: &[String]) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < argv.len() {
        if argv[index] == "--extern" {
            let value = argv.get(index + 1).ok_or("missing extern argument")?;
            result.push(argv[index].clone());
            result.push(if value.contains('=') { value.clone() } else {
                format!("{value}={}", externs.get(value).ok_or_else(|| format!("E_VERUS_DEPENDENCIES: missing current producer for {value}"))?)
            });
            index += 2;
        } else { result.push(argv[index].clone()); index += 1; }
    }
    for directory in search { result.extend(["-L".into(), format!("dependency={directory}")]); }
    Ok(result)
}

/// 门前没有执行路径；门后仍复用原执行器，不签发 ProofAdmission。
pub fn execute_consumer(
    root: &Path, directory: &Path, operation: &str, consumer: &Consumer,
    externs: &BTreeMap<String, String>, search: &[String], environment: &bindings::Environment,
    stdout: &mut (impl Write + Send), stderr: &mut (impl Write + Send),
) -> Result<(), BuildFailure> {
    let method = execution::Method {
        owner: "runner".into(), operation: operation.into(), tool: consumer.tool.clone(),
        argv: arguments(&consumer.argv, externs, search)?, output_contract: None,
    };
    let executable = profile::path(root, &consumer.executable, b".chrono-harness/state/")?;
    let result = execution::run_operation(root, directory, 0, &method, &executable,
        environment, stdout, stderr)?;
    persist(&directory.join("consumer-operation.json"), &result)?;
    if result.exit_code != 0 {
        return Err(BuildFailure { message: "strict dependency consumer process failed".into(), exit_code: Some(result.exit_code) });
    }
    Ok(())
}

pub fn validate_component(root: &Path, config: &Config, externs: &BTreeMap<String, String>) -> Result<(), String> {
    let producer = &config.component_producer;
    let storage = Path::new(&config.component_producer_root);
    let relative = Path::new(&producer.path).strip_prefix(storage).map_err(|e| e.to_string())?
        .to_str().ok_or("non UTF-8 component original path")?;
    let _guard = retention::read_guard(storage, relative)?;
    unchanged(std::slice::from_ref(producer))?;
    let report: Value = chrono_harness::json(&bounded(Path::new(&producer.path))?)?;
    let process = &report["managed_process"];
    if process["exit_code"] != 0 || !process["failure"].is_null() || process["owned_work_joined"] != true {
        return Err("E_VERUS_DEPENDENCIES: component original process is not successful and joined".into());
    }
    let cwd = process["cwd"].as_str().ok_or("missing component cwd")?;
    let argv = process["argv"].as_array().ok_or("missing component producer argv")?;
    let dependency_path = profile::path(root, &config.component_dependency.path, b".chrono-harness/state/")?;
    let actual = identity(&dependency_path)?;
    if !cargo::content_matches(&actual.sha256, actual.length, &config.component_dependency.sha256, config.component_dependency.length) {
        return Err("E_VERUS_DEPENDENCIES: component dep-info differs from retained input".into());
    }
    let raw = bounded(&dependency_path)?;
    let encoded = chrono_harness::verified_verifier::sources::encode_path(config.component_dependency.path.as_bytes());
    let inputs = chrono_harness::verified_verifier::sources::compiler_dependency_paths(&raw, &encoded)
        .ok_or("component compiler dependency header rejected")?;
    for pair in argv.windows(2) {
        if pair[0] != "--extern" { continue; }
        let value = pair[1].as_str().ok_or("invalid component extern")?;
        let (name, path) = value.split_once('=').ok_or("component extern has no producer path")?;
        let expected = Path::new(cwd).join(path);
        let current = Path::new(externs.get(name).ok_or("component dependency absent from runner graph")?);
        let before = identity(&expected).map_err(|error| format!("E_VERUS_DEPENDENCIES: component original extern {name} unavailable: {}; {error}", expected.display()))?;
        let after = identity(current)?;
        if !cargo::content_matches(&before.sha256, before.length, &after.sha256, after.length) {
            return Err(format!("E_VERUS_DEPENDENCIES: component and runner use different compiled instances of {name}"));
        }
        if !inputs.iter().any(|p| Path::new(cwd).join(std::str::from_utf8(p).unwrap_or("")) == expected) {
            return Err(format!("E_VERUS_DEPENDENCIES: component dep-info has no binary input binding for {name}; argv and equal versions do not supply that origin"));
        }
    }
    Ok(())
}

pub(super) fn run(
    root: &Path, declaration: &str, consumer: &str, inherited: &bindings::Environment,
    stdout: &mut (impl Write + Send), stderr: &mut (impl Write + Send), directory: &Path,
) -> Result<(), BuildFailure> {
    let raw = profile::read(root, declaration, b".chrono-harness/ci/")?;
    let build: BuildConfig = serde_json::from_value(chrono_harness::json(&raw)?).map_err(|e| e.to_string())?;
    let config = build.dependencies.as_ref().ok_or("missing runner dependency declaration")?;
    let bootstrap_raw = profile::read(root, &config.bootstrap_profile, b".chrono-harness/ci/")?;
    let bootstrap = profile::Profile::decode(root, &bootstrap_raw)?;
    if bootstrap.rust_toolchain != build.rust_toolchain { return Err("dependency toolchains differ".into()); }
    let registry_raw = profile::read(root, &bootstrap.tool_bindings_registry, b".chrono-harness/ci/")?;
    let native = bindings::native_platform()?;
    let registry: Value = chrono_harness::json(&registry_raw)?;
    let platform = format!("{}/{}", native.system, native.machine);
    let target = registry["platforms"][&platform]["target"].as_str().ok_or("missing native target")?;
    let prepared = Prepared::parse(root, &bootstrap_raw, inherited)?;
    let mut environment = prepared.environment;
    for name in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"] {
        if environment.get(OsStr::new(name)).is_some_and(|v| !v.is_empty()) {
            return Err(format!("E_VERUS_DEPENDENCIES: {name} overrides registered checksum flags").into());
        }
    }
    environment.insert("RUSTUP_TOOLCHAIN".into(), build.rust_toolchain.clone().into());
    environment.insert("RUSTC_BOOTSTRAP".into(), "1".into());
    environment.insert("CARGO_TERM_COLOR".into(), "never".into());
    if let Some(incremental) = bootstrap.rust_incremental {
        environment.insert("CARGO_INCREMENTAL".into(), if incremental { "1" } else { "0" }.into());
    }
    let search_path = environment.get(OsStr::new("PATH")).and_then(|p| p.to_str());
    let rustup = chrono_harness::resolve_program(root, "rustup", search_path)?;
    let declarations = bindings::Declarations::parse(&bootstrap_raw, &native, Some(&registry_raw))?
        .ok_or("bound Cargo and rustc required")?;
    let tools = declarations.select(root, &rustup, &build.rust_toolchain, &BTreeMap::new(), &environment)?;
    environment.insert("RUSTC".into(), tools.rustc.path.clone().into());
    let mut inputs = Vec::new();
    for path in [declaration, &config.bootstrap_profile, &bootstrap.tool_bindings_registry, &build.manifest,
        &build.manifest.replace("Cargo.toml", "Cargo.lock"), &build.projects] {
        inputs.push(identity(&profile::path(root, path, b"")?)?);
    }
    inputs.extend([identity(Path::new(&tools.cargo.path))?, identity(Path::new(&tools.rustc.path))?]);
    inputs.push(identity(&std::env::current_exe().map_err(|e| e.to_string())?)?);
    let input_key = key(&environment, &inputs)?;
    let receipt = profile::path(root, &config.receipt, b"")?;
    let manifest = profile::path(root, &build.manifest, b"")?;
    if consumer == "prepare" {
        for path in prepared.host_directories { fs::create_dir_all(path).map_err(|e| e.to_string())?; }
        let method = |argv: Vec<String>| execution::Method { owner: build.owner.clone(), operation: config.operation.clone(), tool: "cargo".into(), argv, output_contract: None };
        let metadata = execution::run_operation(root, directory, 0, &method(vec!["metadata".into(), "--locked".into(), "--format-version=1".into(), "--filter-platform".into(), target.into(), "--manifest-path".into(), build.manifest.clone()]), Path::new(&tools.cargo.path), &environment, &mut *stdout, &mut *stderr)?;
        persist(&directory.join("metadata-operation.json"), &metadata)?;
        if metadata.exit_code != 0 { return Err(BuildFailure { message: "Cargo dependency resolution failed".into(), exit_code: Some(metadata.exit_code) }); }
        let graph_raw = original_stream(root, &metadata.stdout)?;
        let graph: Value = chrono_harness::json(&graph_raw)?;
        let operation = execution::run_operation(root, directory, 1, &method(vec!["build".into(), "--lib".into(), "--locked".into(), "--profile=dev".into(), "--target".into(), target.into(), "--manifest-path".into(), build.manifest.clone(), "--target-dir".into(), build.target.clone(), "--message-format=json".into(), "--jobs".into(), "1".into(), "--config".into(), "build.rustflags=[\"-Zbinary-dep-depinfo\",\"-Zchecksum-hash-algorithm=sha256\"]".into()]), Path::new(&tools.cargo.path), &environment, &mut *stdout, &mut *stderr)?;
        persist(&directory.join("build-operation.json"), &operation)?;
        if operation.exit_code != 0 { return Err(BuildFailure { message: "Cargo runner dependency production failed".into(), exit_code: Some(operation.exit_code) }); }
        let output = original_stream(root, &operation.stdout)?;
        let (mut files, externs, search, unresolved) = collect(&profile::path(root, &build.target, b"")?, &graph, &output, &manifest)?;
        files.extend(inputs);
        unchanged(&files)?;
        let observation = Observation { schema: "chrono-runner-dependencies/v1".into(), inputs: input_key, metadata, build: operation, files, externs, search, unresolved };
        persist(&directory.join("dependencies.json"), &observation)?;
        persist(&receipt, &identity(&directory.join("dependencies.json"))?)?;
        writeln!(stdout, "{}", json!({"reason":"chrono-runner-dependencies", "receipt":config.receipt, "status":"observed-incomplete", "unresolved":observation.unresolved})).map_err(|e| e.to_string())?;
        // Cargo exit 0 是原观察；完整域尚未建立，准备操作不得报告合格成功。
        return Err("E_VERUS_DEPENDENCIES: dependency observations retained; loaded compiler/native domain remains unqualified".into());
    }
    let operation = consumer;
    let consumer = config.consumers.get(operation).ok_or("unregistered dependency consumer")?;
    let reference: cargo::Identity = serde_json::from_slice(&bounded(&receipt)?).map_err(|e| e.to_string())?;
    unchanged(std::slice::from_ref(&reference))?;
    let evidence = Path::new(&reference.path);
    let relative = evidence.parent().and_then(|p| p.strip_prefix(root).ok()).and_then(|p| p.to_str()).ok_or("dependency original outside host")?;
    let _guard = retention::read_guard(root, relative)?;
    let observation: Observation = serde_json::from_slice(&bounded(evidence)?).map_err(|e| e.to_string())?;
    if observation.schema != "chrono-runner-dependencies/v1" || observation.inputs != input_key {
        return Err("E_VERUS_DEPENDENCIES: producer configuration, tool, environment or inputs differ".into());
    }
    unchanged(&observation.files)?;
    if observation.metadata.exit_code != 0 || observation.build.exit_code != 0 {
        return Err("E_VERUS_DEPENDENCIES: failed original producer".into());
    }
    let graph_raw = original_stream(root, &observation.metadata.stdout)?;
    let output = original_stream(root, &observation.build.stdout)?;
    let graph = chrono_harness::json(&graph_raw)?;
    let (_, externs, search, unresolved) = collect(&profile::path(root, &build.target, b"")?, &graph, &output, &manifest)?;
    if observation.externs != externs || observation.search != search || observation.unresolved != unresolved {
        return Err("E_VERUS_DEPENDENCIES: receipt differs from actual Cargo originals".into());
    }
    validate_component(root, config, &observation.externs)?;
    // 不接受可手填的 readiness 标志。当前 collect 会如实保留工具/原生域缺口。
    if !unresolved.is_empty() {
        return Err(format!("E_VERUS_DEPENDENCIES: strict consumer {operation} is blocked by {unresolved:?}").into());
    }
    execute_consumer(root, directory, operation, consumer, &externs, &search, &environment, stdout, stderr)
}
