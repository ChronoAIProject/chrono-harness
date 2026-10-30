//! Real external interpreter adapter; all fixture assertions are Rust.
use std::{fs, io::{self, Read, Write}, path::Path, process::{Command, ExitCode}};
fn excluded(root: &Path) {
    let dir = root.join(".chrono-harness/state/automatic-cleanup");
    let ledger: serde_json::Value = serde_json::from_slice(&fs::read(dir.join("ledger.json")).unwrap()).unwrap();
    let e = ledger["entries"].as_array().unwrap().iter().find(|e| e["path"] == root.to_str().unwrap()).unwrap();
    for path in [dir.join("admission.lease"), root.join(e["ownership"]["path"].as_str().unwrap())] {
        assert!(chrono_harness::ownership::Lease::acquire(&path, None, false, true, None).unwrap().is_none(),
                "original EX must remain held at {}", path.display());
    }
}
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--version"] { println!("owner probe"); return ExitCode::SUCCESS; }
    if args == ["--hold"] {
        println!("ready"); io::stdout().flush().unwrap();
        let mut byte = [0]; io::stdin().read_exact(&mut byte).unwrap();
        return ExitCode::SUCCESS;
    }
    let publication = args.iter().any(|a| a == "--owner-publication");
    let root = Path::new(&args[1]);
    if publication { excluded(root); }
    let mut child = Command::new("/usr/bin/python3"); child.args(&args);
    chrono_harness::process_fds::forward_command(&mut child, &[]).unwrap();
    let status = child.status().unwrap();
    if publication {
        excluded(root);
        if root.join(".chrono-harness/state/owner-probe-drift").exists() {
            let lease = root.join(".chrono-harness/state/automatic-cleanup/admission.lease");
            fs::rename(&lease, lease.with_extension("original-lease")).unwrap();
            fs::write(lease, "different physical input").unwrap();
        }
    }
    ExitCode::from(u8::try_from(status.code().unwrap_or(1)).unwrap_or(1))
}
