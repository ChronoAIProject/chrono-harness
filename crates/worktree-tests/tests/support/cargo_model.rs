//! Closed command mock for the registered Cargo consumers. No compiler is run.
//! Outputs remain source-bound data, and native holders are this Rust fixture's
//! real children so the process/custody assertions exercise the original leases.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn known_cargo_command(command: &str) -> (allowed: bool)
    ensures allowed == (command@ == "--native-holder"@ || command@ == "--version"@ || command@ == "build"@ || command@ == "test"@ || command@ == "generate-lockfile"@),
{ identifiers::equal_text(command, "--native-holder") || identifiers::equal_text(command, "--version") || identifiers::equal_text(command, "build") || identifiers::equal_text(command, "test") || identifiers::equal_text(command, "generate-lockfile") }
}

#[cfg(not(any(verus_keep_ghost, test)))]
#[path = "."]
mod runtime {
use std::{env, fs, io::{self, Write}, path::{Path, PathBuf}, process::{Command, ExitCode}};
#[path = "native_cargo/src/lib.rs"]
mod native;

fn emit(manifest: &Path, dependency: &[u8]) -> Result<(), String> {
    let project=manifest.parent().ok_or("manifest has no parent")?;
    let source=fs::read(project.join("src/lib.rs")).map_err(|e|format!("{}: {e}",manifest.display()))?;
    let lock=fs::read(project.join("Cargo.lock")).unwrap_or_default();
    let mut input=fs::read(manifest).map_err(|e|e.to_string())?;input.extend_from_slice(&source);input.extend_from_slice(&lock);input.extend_from_slice(dependency);
    let target=project.join("target/debug/deps");fs::create_dir_all(&target).map_err(|e|e.to_string())?;
    fs::write(target.join("fixture-output"),chrono_harness::sha256(&input)).map_err(|e|e.to_string())?;
    Ok(())
}
fn run(args:&[String])->Result<u8,String>{
    if !args.first().is_some_and(|command|super::known_cargo_command(command)){return Err(format!("unknown Cargo fixture operation: {args:?}"));}
    if args==["--native-holder"]{native::hold();return Ok(0);}
    if args==["--version"]{println!("cargo fixture/v1");return Ok(0);}
    let operation=args.first().ok_or("missing Cargo operation")?;
    if !matches!(operation.as_str(),"build"|"test"|"generate-lockfile"){return Err(format!("unknown Cargo fixture operation: {args:?}"));}
    let at=args.iter().position(|v|v=="--manifest-path").ok_or("missing fixture manifest argument")?;
    let manifest=PathBuf::from(args.get(at+1).ok_or("missing fixture manifest")?);
    for (i,arg) in args.iter().enumerate().skip(1){
        if i==at+1||matches!(arg.as_str(),"--manifest-path"|"--lib"|"--locked"|"--offline"|"--verbose"|"--"|"--list"|"--nocapture"|"alpha"|"beta"){continue;}
        return Err(format!("unknown Cargo fixture argument: {arg}"));
    }
    if !manifest.is_file(){return Err(format!("{}: missing fixture manifest",manifest.display()));}
    let project=manifest.parent().unwrap();let source=fs::read(project.join("src/lib.rs")).map_err(|e|e.to_string())?;
    let is_native=source==include_bytes!("native_cargo/src/lib.rs");
    let is_producer=source==include_bytes!("cargo_pair/producer/src/lib.rs");
    let tests=include_bytes!("cargo_pair/tests/src/lib.rs");
    let failure=b"\n#[test] fn actual_failure() { assert_eq!(bounded_cache_producer::answer(), 0, \"original business exception\"); }\n";
    let failing_tests=[tests.as_slice(),failure.as_slice()].concat();
    let is_tests=source==tests||source==failing_tests;
    let workflow_prod=workflow_producer(&source);
    let workflow_test=["p","p2"].iter().any(|p| {
        let original=format!("#[test] fn real_implementation() {{ assert_eq!({p}::double(21),42); }}\n");
        source==original.as_bytes()||source==format!("{original}\n// One explicitly assigned test changes.\n").as_bytes()
    });
    let grouped=source==GROUPED_TEST.as_bytes();
    if !(is_native||is_producer||is_tests||workflow_prod||workflow_test||grouped){return Err(format!("unregistered Cargo fixture source: {}",manifest.display()));}
    let manifest_bytes=fs::read(&manifest).map_err(|e|e.to_string())?;
    let locked=fs::read(project.join("Cargo.lock")).ok();
    let exact=|expected:&[u8],lock:&[u8]|manifest_bytes==expected&&locked.as_deref()==Some(lock);
    let workflow_pair=if project.file_name().is_some_and(|v|v=="p2"||v=="t2"){("p2","t2")}else{("p","t")};
    let (prod,test)=workflow_pair;
    let workflow_manifest=if workflow_prod{format!("[package]\nname='{prod}'\nversion='0.1.0'\nedition='2024'\n")}else{format!("[package]\nname='{test}'\nversion='0.1.0'\nedition='2024'\n[dev-dependencies]\n{prod}={{path='../{prod}'}}\n")};
    let workflow_lock=if workflow_prod{format!("version = 4\n[[package]]\nname = \"{prod}\"\nversion = \"0.1.0\"\n")}else{format!("version = 4\n[[package]]\nname = \"{prod}\"\nversion = \"0.1.0\"\n[[package]]\nname = \"{test}\"\nversion = \"0.1.0\"\ndependencies = [\"{prod}\"]\n")};
    let grouped_lock=b"version = 4\n[[package]]\nname = \"p\"\nversion = \"0.1.0\"\n[[package]]\nname = \"t\"\nversion = \"0.1.0\"\ndependencies = [\"p\"]\n";
    let manifest_known=if is_native {manifest_bytes==include_bytes!("native_cargo/Cargo.toml")&&(locked.is_none()||locked.as_deref()==Some(b"version = 4\n[[package]]\nname = \"lease-native-fixture\"\nversion = \"0.1.0\"\n".as_slice()))}
        else if is_producer {exact(include_bytes!("cargo_pair/producer/Cargo.toml"),include_bytes!("cargo_pair/producer/Cargo.lock"))}
        else if is_tests {exact(include_bytes!("cargo_pair/tests/Cargo.toml"),include_bytes!("cargo_pair/tests/Cargo.lock"))}
        else if grouped {manifest_bytes==b"[package]\nname='t'\nversion='0.1.0'\n[dev-dependencies]\np={path='../p'}\n"&&(operation=="generate-lockfile"||locked.as_deref()==Some(grouped_lock.as_slice()))}
        else {exact(workflow_manifest.as_bytes(),workflow_lock.as_bytes())};
    if !manifest_known{return Err(format!("unregistered Cargo fixture manifest or lock: {}",manifest.display()));}
    if args.iter().any(|arg|matches!(arg.as_str(),"alpha"|"beta"))&&!grouped {return Err("unregistered fixture filter".into());}
    if args.iter().any(|arg|matches!(arg.as_str(),"--list"|"--nocapture"))&&!is_native {return Err("unregistered fixture libtest argument".into());}
    let dependency=if is_tests {Some("producer")} else if workflow_test {Some(prod)} else if grouped {Some("p")} else {None};
    let mut dependency_bytes=Vec::new();
    if let Some(name)=dependency {
        let dependency=project.join(format!("../{name}"));
        let manifest=fs::read(dependency.join("Cargo.toml")).map_err(|e|e.to_string())?;
        let implementation=fs::read(dependency.join("src/lib.rs")).map_err(|e|e.to_string())?;
        let known=if is_tests {manifest==include_bytes!("cargo_pair/producer/Cargo.toml")&&implementation==include_bytes!("cargo_pair/producer/src/lib.rs")}
            else if grouped {manifest==b"[package]\nname='p'\nversion='0.1.0'\n"&&(implementation==b"pub fn double(n: u32) -> u32 { n + n }\n"||implementation==b"pub fn double(n:u32)->u32 { 2*n }\n")}
            else {manifest==format!("[package]\nname='{prod}'\nversion='0.1.0'\nedition='2024'\n").as_bytes()&&workflow_producer(&implementation)};
        if !known {return Err("unregistered fixture producer inputs".into());}
        dependency_bytes.extend_from_slice(&manifest);dependency_bytes.extend_from_slice(&implementation);
    }
    if operation=="generate-lockfile" {
        if !grouped {return Err("unregistered fixture lock generation".into());}
        fs::write(project.join("Cargo.lock"),"version = 4\n[[package]]\nname = \"p\"\nversion = \"0.1.0\"\n[[package]]\nname = \"t\"\nversion = \"0.1.0\"\ndependencies = [\"p\"]\n").map_err(|e|e.to_string())?;
        return Ok(0);
    }
    if is_native && !project.join("Cargo.lock").exists(){fs::write(project.join("Cargo.lock"),"version = 4\n[[package]]\nname = \"lease-native-fixture\"\nversion = \"0.1.0\"\n").map_err(|e|e.to_string())?;}
    emit(&manifest,&dependency_bytes)?;
    if is_producer {fs::copy(project.join("target/debug/deps/fixture-output"),project.join("target/debug/libbounded_cache_producer.rlib")).map_err(|e|e.to_string())?;}
    if grouped {
        if operation!="test" {return Err("unsupported grouped fixture operation".into());}
        let filters:Vec<_>=args.iter().filter(|v|matches!(v.as_str(),"alpha"|"beta")).collect();
        if filters.len()>1{return Err("multiple fixture filters".into());}
        let state=project.join("../.chrono-harness/state");fs::create_dir_all(&state).map_err(|e|e.to_string())?;
        for name in ["alpha","beta"] {if filters.is_empty()||filters[0].as_str()==name {fs::write(state.join(name),name).map_err(|e|e.to_string())?;}}
    }
    if is_tests{
        if operation=="test"&&source!=tests{println!("running 2 tests\noriginal business exception");eprintln!("original business exception");return Ok(101);}
    }
    if is_native&&operation=="test"{
        if args.iter().any(|a|a=="--list"){println!("native_holder: test\n\n1 test, 0 benchmarks");return Ok(0);}
        let mut child=Command::new(env::current_exe().map_err(|e|e.to_string())?);
        child.arg("--native-holder");chrono_harness::process_fds::forward_command(&mut child,&[])?;
        let status=child.status().map_err(|e|e.to_string())?;
        return Ok(status.code().and_then(|c|u8::try_from(c).ok()).unwrap_or(1));
    }
    println!("cargo fixture {operation}: {}",manifest.display());
    Ok(0)
}
pub fn dispatch(args: &[String])->ExitCode{
    match run(args){Ok(code)=>ExitCode::from(code),Err(error)=>{let _=writeln!(io::stderr(),"{error}");ExitCode::from(93)}}
}

fn main()->ExitCode { dispatch(&env::args().skip(1).collect::<Vec<_>>()) }

fn workflow_producer(source:&[u8])->bool {
    ["pub fn double(n:i32)->i32 { n*2 }\n","pub fn double(n:i32)->i32 { n+n }\n","pub fn double(n:i32)->i32 {n*2}\n","pub fn double(n:i32)->i32 {n+n}\n","pub fn double(n:i32)->i32 {2*n}\n"].iter().any(|s|source==s.as_bytes())
}
const GROUPED_TEST:&str="#[cfg(test)] mod tests { fn run(name: &str) { assert_eq!(p::double(21),42); let root=std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(); std::fs::write(root.join(\".chrono-harness/state\").join(name), name).unwrap(); } #[test] fn alpha() { run(\"alpha\"); } #[test] fn beta() { run(\"beta\"); } }\n";

}
#[cfg(not(any(verus_keep_ghost, test)))]
pub use runtime::{dispatch};

#[cfg(not(any(verus_keep_ghost, test)))]
fn main() -> std::process::ExitCode { runtime::dispatch(&std::env::args().skip(1).collect::<Vec<_>>()) }
