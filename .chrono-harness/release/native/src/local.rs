//! 本地发布驱动：固定 Git 源、独立检出、有限并发和原生阻塞完成通道。
use crate::{execution::Environment, recipe::{Loaded, Scope}};
use chrono_harness::{CommandSpec, file_identity, no_symlink_parents};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs::{self, OpenOptions}, io::Write, os::unix::fs::PermissionsExt, path::{Path, PathBuf}, sync::mpsc, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

fn error(value: impl std::fmt::Display) -> String { format!("E_RELEASE_LOCAL: {value}") }
fn text(path: &Path) -> Result<String,String> {path.to_str().map(str::to_owned).ok_or_else(||error("non UTF-8 local release path"))}
fn timestamp() -> Result<u128,String> {SystemTime::now().duration_since(UNIX_EPOCH).map(|v|v.as_nanos()).map_err(error)}

fn write(path: &Path, bytes: &[u8]) -> Result<(),String> {
    if let Some(parent)=path.parent() {fs::create_dir_all(parent).map_err(error)?;}
    let mut file=OpenOptions::new().write(true).create_new(true).open(path).map_err(error)?;
    file.write_all(bytes).and_then(|()|file.flush()).map_err(error)
}

/// 只运输本次实际产生的普通文件；复制原字节、权限并核对原始摘要。
fn copy_tree(source: &Path, target: &Path) -> Result<(),String> {
    let metadata=fs::symlink_metadata(source).map_err(error)?;
    if !metadata.is_dir() {return Err(error("release handoff source is not a regular directory"));}
    fs::create_dir(target).map_err(error)?;
    let mut entries=fs::read_dir(source).map_err(error)?.collect::<Result<Vec<_>,_>>().map_err(error)?;
    entries.sort_by_key(|entry|entry.file_name());
    for entry in entries {
        let source=entry.path();let target=target.join(entry.file_name());let metadata=fs::symlink_metadata(&source).map_err(error)?;
        if metadata.is_dir() {copy_tree(&source,&target)?;} else if metadata.is_file() {
            let expected=file_identity(&source)?;
            let input=fs::File::open(&source).map_err(error)?;
            let mut output=OpenOptions::new().create_new(true).write(true).open(&target).map_err(error)?;
            let actual=chrono_harness::copy_hashed(input,&mut output)?;output.flush().map_err(error)?;
            fs::set_permissions(&target,metadata.permissions()).map_err(error)?;
            if actual!=expected || file_identity(&target)?!=expected || file_identity(&source)?!=expected || fs::metadata(&source).map_err(error)?.permissions().mode()!=metadata.permissions().mode() {return Err(error("local release handoff changed during transport"));}
        } else {return Err(error("release handoff contains a link or non-regular member"));}
    }
    Ok(())
}

struct Driver<'a> {
    root: PathBuf,
    output: PathBuf,
    git: PathBuf,
    executable: PathBuf,
    environment: Environment,
    loaded: &'a Loaded,
    source: String,
    run_id: String,
    started: Instant,
}

impl Driver<'_> {
    fn process(&self, root: &Path, logs: &Path, records: &mut Vec<Value>, argv: Vec<String>, environment: &Environment) -> Result<Vec<u8>,String> {
        let remaining=Duration::from_secs(self.loaded.recipe.limits.platform_seconds).checked_sub(self.started.elapsed()).ok_or_else(||error("registered local release platform span exceeded"))?;
        let timeout=remaining.min(Duration::from_secs(self.loaded.recipe.limits.unit_seconds));
        if timeout.is_zero() {return Err(error("registered local release span exhausted"));}
        let env=environment.iter().filter(|(key,_)|key.as_os_str()!=std::ffi::OsStr::new(chrono_harness::process_fds::ENV)).map(|(key,value)|Ok((key.to_str().ok_or_else(||error("non UTF-8 environment name"))?.to_owned(),value.to_str().ok_or_else(||error("non UTF-8 environment value"))?.to_owned()))).collect::<Result<BTreeMap<_,_>,String>>()?;
        let (program,args)=argv.split_first().ok_or_else(||error("empty local release command"))?;
        let spec=CommandSpec {program:program.clone(),args:args.to_vec(),env,timeout_seconds:timeout.as_secs().max(1),output_limit_bytes:chrono_harness::verified_limits::MAX_OUTPUT_BYTES};
        let started=timestamp()?;
        let observed=chrono_harness::run_process_observed_for(root,&spec,&[],&file_identity(Path::new(program))?.0,timeout)?;
        let index=records.len();let mut record=json!({"argv":observed.argv,"cwd":observed.cwd,"pid":observed.child_pid,"launcher_pid":observed.launcher_pid,"executable":observed.executable,"sha256":observed.sha256,"environment_digest":observed.environment_digest,"started_ns":started,"finished_ns":timestamp()?,"exit_code":observed.exit_code,"failure":observed.failure,"timeout_ns":timeout.as_nanos(),"output_limit_bytes":spec.output_limit_bytes});
        let mut failures=Vec::new();
        for (stream,bytes) in [("stdout",&observed.stdout_bytes),("stderr",&observed.stderr_bytes)] {
            let path=logs.join(format!("{index}.{stream}"));
            let relative=path.strip_prefix(&self.output).map_err(error)?;
            let mut declaration=json!({"size":bytes.len(),"sha256":chrono_harness::sha256(bytes)});
            match write(&path,bytes) {Ok(())=>declaration["path"]=json!(text(relative)?),Err(failure)=>{declaration["status"]=json!("unavailable");declaration["bytes"]=json!(bytes);failures.push(failure);}}
            record[stream]=declaration;
        }
        record["retention_failures"]=json!(failures);records.push(record);
        if observed.exit_code!=0 || observed.failure.is_some() {return Err(error(format!("original local release process failed: exit {}, {:?}",observed.exit_code,observed.failure)));}
        if !failures.is_empty() {return Err(error(failures.join("; ")));}
        Ok(observed.stdout_bytes)
    }

    fn invoke(&self,id: &str,dependencies: Value,collect: bool) -> Value {
        let clone=self.output.join("roots").join(id);let logs=self.output.join("driver-processes").join(id);let handoff=self.output.join("units").join(id);
        let mut records=Vec::new();let mut outputs=json!({});
        let outcome=(|| -> Result<(),String> {
            fs::create_dir_all(clone.parent().ok_or_else(||error("clone parent"))?).map_err(error)?;
            let git=text(&self.git)?;
            self.process(&self.root,&logs,&mut records,vec![git.clone(),"clone".into(),"--no-hardlinks".into(),"--no-checkout".into(),text(&self.root)?,text(&clone)?],&self.environment)?;
            self.process(&clone,&logs,&mut records,vec![git,"checkout".into(),"--detach".into(),self.source.clone()],&self.environment)?;
            for (need,dependency) in dependencies.as_object().ok_or_else(||error("dependency outcomes are not an object"))? {
                if dependency["result"]!="success" {continue;}
                let destination=no_symlink_parents(&clone,self.loaded.unit(need)?.handoff())?;
                fs::create_dir_all(destination.parent().ok_or_else(||error("handoff parent"))?).map_err(error)?;
                copy_tree(&self.output.join("units").join(need),&destination)?;
            }
            let mut environment=self.environment.clone();
            for (name,value) in [("CHRONO_RELEASE_LOCAL","1"),("CHRONO_RELEASE_RUN",&self.run_id),("CHRONO_RELEASE_ATTEMPT","1"),("CHRONO_RELEASE_JOB",id)] {environment.insert(name.into(),value.into());}
            environment.insert("CHRONO_RELEASE_DEPENDENCIES".into(),dependencies.to_string().into());
            if collect {write(&no_symlink_parents(&clone,&self.loaded.recipe.collector.dependency_metadata)?,&serde_json::to_vec(&dependencies).map_err(error)?)?;}
            let mut argv=vec![text(&self.executable)?,text(&clone)?,text(&handoff)?];
            if collect {argv.push("--collect".into());} else {argv.extend(["--unit".into(),id.into()]);}
            self.process(&clone,&logs,&mut records,argv,&environment)?;
            let raw=fs::read(handoff.join("receipt.json")).map_err(error)?;let receipt=chrono_harness::json(&raw)?;
            if receipt["status"]!="passed" || receipt["context"]["source_commit"]!=self.source || receipt["lineage"]!=json!({"kind":"local","run_id":self.run_id,"attempt":"1","job":id}) {return Err(error("local child has no matching passing original receipt"));}
            outputs=json!({"artifact_id":chrono_harness::sha256(&raw),"run_id":self.run_id,"attempt":"1"});
            Ok(())
        })();
        let mut result=json!({"result":if outcome.is_ok() {"success"} else {"failure"},"outputs":outputs,"processes":records});
        if let Err(failure)=outcome {result["error"]=json!(failure);}
        // 失败检出保留原件；成功后只回收本次 clone，原回执／两流继续归 output。
        if result["result"]=="success" {
            if let Err(failure)=fs::remove_dir_all(&clone) {result["cleanup_error"]=json!(failure.to_string());}
        } else if clone.exists() {result["retained_root"]=json!(clone);}
        result
    }

    fn units(&self,outcomes: &mut BTreeMap<String,Value>) -> Result<(),String> {
        thread::scope(|scope| {
            let (sender,receiver)=mpsc::channel();let mut running=BTreeMap::new();
            let mut pending=self.loaded.recipe.units.iter().collect::<Vec<_>>();
            while !pending.is_empty() || !running.is_empty() {
                let mut index=0;
                while index<pending.len() && running.len()<self.loaded.recipe.local_workers {
                    let unit=pending[index];
                    if !unit.needs().iter().all(|need|outcomes.contains_key(need)) {index+=1;continue;}
                    let id=unit.id().to_owned();let dependencies=unit.needs().iter().map(|need|(need.clone(),json!({"result":outcomes[need]["result"],"outputs":outcomes[need]["outputs"]}))).collect::<BTreeMap<_,_>>();
                    let sender=sender.clone();let expected=id.clone();
                    let handle=scope.spawn(move || {
                        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||self.invoke(&id,json!(dependencies),false)));
                        let result=result.unwrap_or_else(|_|json!({"result":"failure","outputs":{},"error":"local release worker panicked; original process outcome is unknown"}));
                        let _=sender.send((id,result));
                    });
                    running.insert(expected,handle);pending.remove(index);
                }
                if running.is_empty() {return Err(error("local release has no ready registered unit"));}
                let (id,result)=receiver.recv().map_err(error)?;
                let handle=running.remove(&id).ok_or_else(||error("completion names a different local release unit"))?;
                handle.join().map_err(|_|error("local release completion worker panicked"))?;
                outcomes.insert(id,result);
            }
            Ok(())
        })
    }
}

pub fn run(root: &Path,output: &Path,environment: Environment) -> Result<i32,String> {
    let root=fs::canonicalize(root).map_err(error)?;let loaded=Loaded::load(&root)?;
    match fs::symlink_metadata(output) {
        Ok(_)=>return Err(error("release output already exists")),
        Err(failure) if failure.kind()==std::io::ErrorKind::NotFound=>{},
        Err(failure)=>return Err(error(failure)),
    }
    let output=if output.is_absolute() {output.to_owned()} else {std::env::current_dir().map_err(error)?.join(output)};
    let git=loaded.tools(&root,Scope::Collect)?["git"].clone();let executable=std::env::current_exe().map_err(error)?;
    // 验证前不创建输出；引导命令也使用同一所有者等待原进程和后代。
    let env=environment.iter().filter(|(key,_)|key.as_os_str()!=std::ffi::OsStr::new(chrono_harness::process_fds::ENV)).map(|(key,value)|Ok((key.to_str().ok_or_else(||error("non UTF-8 environment name"))?.to_owned(),value.to_str().ok_or_else(||error("non UTF-8 environment value"))?.to_owned()))).collect::<Result<BTreeMap<_,_>,String>>()?;
    let mut source_observations=Vec::new();
    let mut observe=|args: &[&str]| -> Result<Vec<u8>,String> {
        let command=CommandSpec {program:text(&git)?,args:args.iter().map(|s|(*s).to_owned()).collect(),env:env.clone(),timeout_seconds:loaded.recipe.limits.unit_seconds,output_limit_bytes:chrono_harness::verified_limits::MAX_OUTPUT_BYTES};
        let process=chrono_harness::run_process_observed(&root,&command,&[],&file_identity(&git)?.0)?;
        source_observations.push(json!({"argv":process.argv,"cwd":process.cwd,"pid":process.child_pid,"sha256":process.sha256,"environment_digest":process.environment_digest,"exit_code":process.exit_code,"failure":process.failure,"stdout":{"bytes":process.stdout_bytes,"sha256":process.stdout_sha256},"stderr":{"bytes":process.stderr_bytes,"sha256":process.stderr_sha256}}));
        if process.exit_code!=0 || process.failure.is_some() {return Err(error(format!("source observation failed: {} {:?}, original stdout {:?}, original stderr {:?}",process.exit_code,process.failure,process.stdout_bytes,process.stderr_bytes)));}Ok(process.stdout_bytes)
    };
    let source=String::from_utf8(observe(&["rev-parse","HEAD"])?).map_err(error)?.trim().to_owned();
    if !chrono_identifiers::valid_oid(source.as_bytes()) || !observe(&["status","--porcelain","--untracked-files=normal"])?.is_empty() {return Err(error("local release requires a fixed clean candidate"));}
    fs::create_dir_all(&output).map_err(error)?;let started_ns=timestamp()?;
    let driver=Driver {root,output:output.clone(),git,executable,environment,loaded:&loaded,source,run_id:format!("{}-{started_ns}",std::process::id()),started:Instant::now()};
    let mut outcomes=BTreeMap::new();let outcome=(|| -> Result<(),String> {
        driver.units(&mut outcomes)?;
        let dependencies=outcomes.iter().map(|(id,result)|(id.clone(),json!({"result":result["result"],"outputs":result["outputs"]}))).collect::<BTreeMap<_,_>>();
        let id=&loaded.recipe.collector.id;let result=driver.invoke(id,json!(dependencies),true);let passed=result["result"]=="success";outcomes.insert(id.clone(),result);
        if !passed {return Err(error("local release collector failed; inspect original unit and driver reports"));}
        let accepted=output.join("units").join(id);
        for entry in fs::read_dir(&accepted).map_err(error)? {
            let entry=entry.map_err(error)?;let target=output.join(entry.file_name());
            match fs::symlink_metadata(&target) {
                Ok(_)=>return Err(error("local release publication would replace an existing member")),
                Err(failure) if failure.kind()==std::io::ErrorKind::NotFound=>{},
                Err(failure)=>return Err(error(failure)),
            }
            fs::rename(entry.path(),target).map_err(error)?;
        }
        fs::remove_dir(&accepted).map_err(error)?;
        Ok(())
    })();
    let metadata=json!({"schema":"chrono-release-local/v1","source_commit":driver.source,"run_id":driver.run_id,"started_ns":started_ns,"finished_ns":timestamp()?,"status":if outcome.is_ok() {"passed"} else {"failed"},"units":outcomes,"source_observations":source_observations,"driver_tools":{"git":{"path":driver.git,"sha256":file_identity(&driver.git)?.0},"producer":{"path":driver.executable,"sha256":file_identity(&driver.executable)?.0}},"output_limit_bytes":chrono_harness::verified_limits::MAX_OUTPUT_BYTES,"error":outcome.as_ref().err()});
    write(&output.join("local.json"),&serde_json::to_vec_pretty(&metadata).map_err(error)?)?;
    Ok(if outcome.is_ok() {0} else {2})
}
