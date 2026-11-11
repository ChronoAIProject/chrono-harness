//! Closed Shell command-boundary mock for CI render consumers. It accepts only
//! one canonically quoted argv, optionally preceded by the release metadata
//! pair. This fixture never interprets general Shell or resolves a host tool.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
verus! {
pub fn known_target(target:&str) -> (known:bool)
    ensures known == (target@ == "chrono-harness"@ || target@ == "$CHRONO_BASE"@ || target@ == "/usr/bin/printf"@),
{ identifiers::equal_text(target,"chrono-harness") || identifiers::equal_text(target,"$CHRONO_BASE") || identifiers::equal_text(target,"/usr/bin/printf") }
}
#[cfg(not(any(verus_keep_ghost,test)))]
mod runtime {
use std::{env,fs,io::{self,Write},os::unix::process::CommandExt,path::Path,process::{self,Command}};
fn quoted(line:&str)->Result<Vec<String>,String> {
    let args=shlex::split(line).ok_or("unterminated generated command")?;
    let canonical=args.iter().map(|arg|format!("'{}'",arg.replace('\'',"'\"'\"'"))).collect::<Vec<_>>().join(" ");
    if line!=canonical || args.is_empty() {return Err("unsupported generated command quotation".into());}
    Ok(args)
}
fn run(args:&[String])->Result<(),String> {
    let script=match args { [flag,script] if flag=="-c"=>script, [errexit,flag,script] if errexit=="-e"&&flag=="-c"=>script,_=>return Err(format!("unknown Shell fixture argv: {args:?}")) };
    let lines:Vec<_>=script.lines().map(str::trim).collect();
    let line=match lines.as_slice() {
        [command]=>*command,
        [mkdir,metadata,command]=> {
            let directory=quoted(mkdir.strip_prefix("mkdir -p ").ok_or("unknown release metadata setup")?)?;
            let destination=quoted(metadata.strip_prefix("printf '%s\\n' \"$CHRONO_RELEASE_DEPENDENCIES\" > ").ok_or("unknown release metadata output")?)?;
            if directory.len()!=1||destination.len()!=1 {return Err("unknown release metadata paths".into());}
            let parent=Path::new(&destination[0]).parent().unwrap_or(Path::new("."));
            if parent!=Path::new(&directory[0]) {return Err("release metadata parent mismatch".into());}
            fs::create_dir_all(&directory[0]).map_err(|e|e.to_string())?;
            fs::write(&destination[0],format!("{}\n",env::var("CHRONO_RELEASE_DEPENDENCIES").map_err(|e|e.to_string())?)).map_err(|e|e.to_string())?;
            *command
        }
        _=>return Err("unknown generated Shell command body".into()),
    };
    let argv=quoted(line)?;let program=&argv[0];
    let target=if program=="/usr/bin/printf" {program.as_str()} else {Path::new(program).file_name().and_then(|s|s.to_str()).ok_or("unknown generated target")?};
    if !super::known_target(target) {return Err(format!("unknown generated target: {program}"));}
    if target=="/usr/bin/printf" {
        if argv.len()!=3||argv[1]!="%s" {return Err("unknown printf fixture argv".into());}
        io::stdout().write_all(argv[2].as_bytes()).map_err(|e|e.to_string())?;return Ok(());
    }
    if target=="chrono-harness" && program!=".chrono-harness/bin/chrono-harness" {return Err("unbound harness fixture target".into());}
    // The only targets are the Rust implementation under test and the private
    // Rust recording fixture. No interpreter or system dependency is launched.
    let mut command=Command::new(program);command.args(&argv[1..]);
    chrono_harness::process_fds::forward_command(&mut command,&[])?;
    Err(format!("generated fixture target exec failed: {}",command.exec()))
}
pub fn main() {
    if let Err(error)=run(&env::args().skip(1).collect::<Vec<_>>()) {eprintln!("{error}");process::exit(93);}
}
}
#[cfg(not(any(verus_keep_ghost,test)))]
fn main() {runtime::main();}
