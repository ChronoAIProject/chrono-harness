//! 发布工具自己的生产与消费证据；复用 bootstrap 原报告及现有单执行物运输。
use crate::{
    execution::Environment,
    receipt::{self, Kind, Lineage, Selection},
    recipe::{Loaded, NativeFrontend},
    transport::{self, ExecutableIdentity},
    verified_recipe as policy,
};
use chrono_host_bootstrap::{
    bindings::{NativePlatform, native_platform},
    construction::Report,
    original::Producer,
    profile,
    source::{self, SourceObservation},
};
use serde_json::{Value, json};
use std::{ffi::OsStr, fs, io::Write, path::Path};

const INSTALLED: &str = ".chrono-harness/bin/chrono-host-release";

fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_FRONTEND: {message}")
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value[field].as_str().ok_or_else(|| error(format!("missing {field}")))
}

fn variable<'a>(environment: &'a Environment, name: &str) -> Result<&'a str, String> {
    environment.get(OsStr::new(name)).and_then(|s| s.to_str())
        .filter(|s| !s.is_empty()).ok_or_else(|| error(format!("missing {name}")))
}

fn producer(root: &Path, declared: &NativeFrontend, native: &NativePlatform) -> Result<Producer, String> {
    Producer::read(&declared.bootstrap_profile, native, &|path| profile::read(root, path, b""))
        .map_err(error)
}

fn qualify(
    producer: &Producer, raw: &[u8], source: &SourceObservation,
    stream: &impl Fn(&str) -> Result<Vec<u8>, String>, identity: &ExecutableIdentity,
) -> Result<Report, String> {
    let report = producer.validate(raw, source, stream).map_err(error)?;
    let planned = producer.profile.install.first();
    let installed = report.installed.first();
    if !policy::frontend_installation(
        producer.profile.install.len() as u64, report.installed.len() as u64,
        planned.map(|p| p.destination.as_str()).unwrap_or(""),
        installed.map(|p| p.path.as_str()).unwrap_or(""),
        installed.map(|p| p.sha256.as_str()).unwrap_or(""), &identity.sha256,
    ) {
        return Err(error("original bootstrap installation differs from release frontend"));
    }
    Ok(report)
}

fn write(directory: &Path, relative: &str, raw: &[u8]) -> Result<Value, String> {
    let path = chrono_harness::no_symlink_parents(directory, relative)?;
    fs::create_dir_all(path.parent().ok_or("missing original parent")?).map_err(error)?;
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(error)?;
    file.write_all(raw).and_then(|()| file.flush()).map_err(error)?;
    Ok(json!({"path":relative,"size":raw.len(),"sha256":chrono_harness::sha256(raw)}))
}

fn actual(path: &Path) -> Result<ExecutableIdentity, String> {
    let parent = path.parent().ok_or("missing frontend parent")?;
    let name = path.file_name().and_then(|s| s.to_str()).ok_or("invalid frontend filename")?;
    transport::observe(parent, name)
}

fn same(actual: &ExecutableIdentity, expected: &ExecutableIdentity) -> bool {
    policy::same_executable(&actual.sha256, actual.size, actual.mode, &expected.sha256, expected.size, expected.mode)
}

/// 只导出已经成功构建并安装、且实际正在执行的工具；最后才发布清单。
pub fn export(root: &Path, output: &Path, environment: &Environment) -> Result<(), String> {
    let root = fs::canonicalize(root).map_err(error)?;
    let loaded = Loaded::load(&root)?;
    let lineage = Lineage {kind:Kind::Github, run_id:variable(environment,"CHRONO_RELEASE_RUN")?.into(),
        attempt:variable(environment,"CHRONO_RELEASE_ATTEMPT")?.into(), job:variable(environment,"CHRONO_RELEASE_JOB")?.into()};
    lineage.validate()?;
    let declared = loaded.recipe.native_frontends.values().find(|f| f.job == lineage.job)
        .ok_or_else(|| error("unregistered frontend producing job"))?;
    let native = native_platform()?;
    let source = source::observe(&root, environment);
    let producer = producer(&root, declared, &native)?;
    let report_path = producer.profile.report_path.as_deref().ok_or("missing bootstrap report path")?;
    let raw = profile::read(&root, report_path, b".chrono-harness/state/")?;
    let identity = transport::observe(&root, INSTALLED)?;
    if !same(&actual(&std::env::current_exe().map_err(error)?)?, &identity) {
        return Err(error("executing frontend differs from bootstrap installation"));
    }
    let report = qualify(&producer, &raw, &source, &|path| profile::read(&root, path, b".chrono-harness/state/"), &identity)?;
    // create_dir 拒绝任何已有输出，包括链接；失败的部分输出保留供诊断。
    fs::create_dir(output).map_err(error)?;
    let packed = transport::pack(&root, INSTALLED, &output.join("executable.tar"))?;
    if !same(&packed.executable, &identity) {
        return Err(error("installed frontend changed during export"));
    }
    let bootstrap = write(output, &declared.bootstrap_report, &raw)?;
    for operation in &report.operations {
        for stream in [&operation.stdout, &operation.stderr] {
            let path = receipt::original(&root, &serde_json::to_value(stream).map_err(error)?)?;
            write(output, &stream.path, &fs::read(path).map_err(error)?)?;
        }
    }
    if !chrono_host_bootstrap::verified_bindings::observation_matches(&source, &source::observe(&root, environment)) {
        return Err(error("frontend source changed during export"));
    }
    let manifest = json!({"schema":"chrono-release-frontend/v1", "source":source,
        "system":native.system,"machine":native.machine,"lineage":lineage,
        "executable":identity.receipt(&declared.executable),"archive":packed.archive.receipt(),"bootstrap":bootstrap});
    write(output, "frontend.json", &serde_json::to_vec_pretty(&manifest).map_err(error)?)?;
    Ok(())
}

/// needs 已选出原生产者；本边界另核原构建、工具字节和实际执行物。
/// collector 不执行依赖工具，传 None；真正消费者传当前执行物路径。
pub fn validate(
    root: &Path, directory: &Path, declared: &NativeFrontend, native: &NativePlatform,
    source: &SourceObservation, selected: &Selection, executing: Option<&Path>,
) -> Result<Value, String> {
    let manifest = crate::recipe::read_json(directory, "frontend.json")?;
    let lineage: Lineage = serde_json::from_value(manifest["lineage"].clone()).map_err(error)?;
    lineage.validate()?;
    if !policy::frontend_header(text(&manifest,"schema")?, matches!(lineage.kind,Kind::Github),
        text(&manifest["source"],"commit")?,text(&manifest["source"],"tree")?, text(&manifest,"system")?, text(&manifest,"machine")?,
        source.commit.as_deref().unwrap_or(""), source.tree.as_deref().unwrap_or(""), &native.system, &native.machine)
        || !chrono_identifiers::equal_text(&selected.job, &declared.job)
        || !policy::producer_lineage(&lineage.job,&declared.job,&lineage.run_id,&selected.run_id,
            lineage.attempt.parse().map_err(error)?,selected.attempt.parse().map_err(error)?) {
        return Err(error("frontend source, platform or original production differs"));
    }
    if text(&manifest["executable"],"path")? != declared.executable
        || text(&manifest["bootstrap"],"path")? != declared.bootstrap_report
        || text(&manifest["archive"],"path")? != "executable.tar" {
        return Err(error("frontend declared artifact paths differ"));
    }
    let identity = ExecutableIdentity::from_receipt(&manifest["executable"])?;
    let archive = receipt::original(directory, &manifest["archive"])?;
    transport::validate(&archive, &transport::ArchiveIdentity {
        sha256:text(&manifest["archive"],"sha256")?.into(),
        size:manifest["archive"]["size"].as_u64().ok_or("missing archive size")?,
    }, &identity)?;
    let raw = fs::read(receipt::original(directory, &manifest["bootstrap"])?).map_err(error)?;
    let producer = producer(root,declared,native)?;
    qualify(&producer, &raw, source, &|path| {
        let file = chrono_harness::no_symlink_parents(directory,path)?;
        fs::read(file).map_err(error)
    }, &identity)?;
    if let Some(path) = executing {
        let expected = chrono_harness::no_symlink_parents(directory, &declared.executable)?;
        if fs::canonicalize(path).map_err(error)? != expected || !same(&actual(path)?,&identity) {
            return Err(error("actual frontend differs from selected bootstrap artifact"));
        }
    }
    Ok(manifest)
}
