use serde_json::Value;
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::ExitCode,
    thread,
    time::Duration,
};

fn main() -> ExitCode {
    if std::env::args_os().skip(1).eq(["--version"]) {
        println!("fixture");
        return ExitCode::SUCCESS;
    }
    let input = std::env::current_exe().unwrap().with_extension("json");
    let config: Value = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
    if let Some(writes) = config["writes"].as_array() {
        for write in writes {
            let path = Path::new(write["path"].as_str().unwrap());
            if write["create_parents"] == true {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
            }
            fs::write(path, write["contents"].as_str().unwrap()).unwrap();
        }
    }
    if let Some(paths) = config["executable"].as_array() {
        for path in paths {
            let path = Path::new(path.as_str().unwrap());
            let mode = fs::metadata(path).unwrap().permissions().mode();
            fs::set_permissions(path, fs::Permissions::from_mode(mode | 0o111)).unwrap();
        }
    }
    if let Some(release) = config["release"].as_str() {
        while !Path::new(release).is_file() {
            thread::sleep(Duration::from_millis(20));
        }
    }
    if let Some(error) = config["stderr"].as_str() {
        io::stderr().write_all(error.as_bytes()).unwrap();
    }
    ExitCode::from(u8::try_from(config["exit"].as_u64().unwrap_or(0)).unwrap())
}
