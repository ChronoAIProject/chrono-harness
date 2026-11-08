use serde_json::{Value, json};
use std::{
    env,
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    os::unix::{ffi::OsStrExt, process::CommandExt},
    path::Path,
    process::{Command, ExitCode},
};

fn append(path: &Path, bytes: &[u8]) {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn emit(value: &Value) {
    serde_json::to_writer(io::stdout().lock(), value).unwrap();
}

fn prepared_producer(args: &[std::ffi::OsString]) -> ExitCode {
    if args.first().is_some_and(|arg| arg == "--version") {
        println!("chrono-worktree 0.1.0");
        return ExitCode::SUCCESS;
    }
    let mut raw = Vec::new();
    io::stdin().read_to_end(&mut raw).unwrap();
    let request: Value = serde_json::from_slice(&raw).unwrap();
    let mut prepared = read_json(Path::new(".chrono-harness/state/fixture-prepared.json"));
    prepared["request_sha256"] = json!(chrono_harness::sha256(&raw));
    let selection = &request["selection"];
    prepared["scope"] = match selection["kind"].as_str().unwrap() {
        "all" => Value::Null,
        "unit" => json!({"kind":"unit", "unit":selection["unit"]}),
        "collect" => {
            json!({"kind":"collect", "manifest":".chrono-harness/state/collection/manifest.json"})
        }
        other => panic!("unknown prepared fixture selection: {other}"),
    };
    emit(&prepared);
    ExitCode::SUCCESS
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn full_units_api(data: &Value, args: &[std::ffi::OsString]) -> ExitCode {
    let run = |unit: &str| {
        json!({"id":if unit == "one" {1} else {2},"run_attempt":1,
            "status":"completed","conclusion":"success","head_sha":data["candidate"],
            "event":"push","path":format!(".github/workflows/{unit}.yml")})
    };
    if args[0] == "api" {
        let endpoint = args[1].to_str().unwrap();
        if let Some((_, address)) = endpoint.split_once("/actions/workflows/") {
            let unit = address.split(".yml").next().unwrap();
            emit(&json!([{"workflow_runs":[run(unit)]}]));
        } else if let Some((_, address)) = endpoint.split_once("/contents/") {
            let path = address.split('?').next().unwrap();
            io::stdout()
                .write_all(
                    &fs::read(Path::new(data["workflow_root"].as_str().unwrap()).join(path))
                        .unwrap(),
                )
                .unwrap();
        } else {
            emit(&run(if endpoint.ends_with("/1") {
                "one"
            } else {
                "two"
            }));
        }
    } else {
        let unit = if args[2] == "1" { "one" } else { "two" };
        let position = args.iter().position(|arg| arg == "--dir").unwrap();
        copy_tree(
            &Path::new(data["uploads"].as_str().unwrap()).join(unit),
            Path::new(&args[position + 1]),
        );
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let invocation = env::args_os().next().unwrap();
    let executable = Path::new(&invocation);
    let parent = executable.parent().unwrap();
    let args: Vec<_> = env::args_os().skip(1).collect();
    let name = executable.file_name().unwrap();
    if name == "fixture-producer" {
        return prepared_producer(&args);
    }
    if name == "mock-gh" {
        let data = read_json(&parent.join("full-units-api.json"));
        assert_eq!(data["profile"], "full-units");
        return full_units_api(&data, &args);
    }
    if name == "git" {
        eprintln!("ambient-git-must-not-run");
        return ExitCode::from(83);
    }
    let (variable, trace) = match name.to_str().unwrap() {
        "selected Git λ" => ("CHRONO_WORKFLOW_REAL_GIT", Some("git-trace")),
        "facts-git" => ("CHRONO_WORKFLOW_REAL_GIT", None),
        "cargo-fixture" => ("CHRONO_WORKFLOW_REAL_CARGO", Some("business-launches")),
        "python-migration" => ("CHRONO_WORKFLOW_REAL_PYTHON", Some("business-launches")),
        other => panic!("unknown registered test tool: {other}"),
    };
    if variable == "CHRONO_WORKFLOW_REAL_GIT"
        && env::var_os("FACTS_SENTINEL").as_deref() != Some(OsStr::new("declared"))
    {
        return ExitCode::from(81);
    }
    if let Some(trace) = trace {
        let mut line = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            if index != 0 {
                line.push(b' ');
            }
            line.extend_from_slice(arg.as_bytes());
        }
        line.push(b'\n');
        append(&parent.join(trace), &line);
    }
    if name == "cargo-fixture" {
        if args.first().is_some_and(|arg| arg == "fixture-prepare") {
            append(Path::new(".chrono-harness/state/order"), b"s");
            return ExitCode::SUCCESS;
        }
        if Path::new(".chrono-harness/state/fail").is_file()
            && args.get(2).is_some_and(|arg| arg == "t2/Cargo.toml")
        {
            return ExitCode::from(7);
        }
    }
    let real = env::var_os(variable).expect("explicit native fixture tool binding");
    panic!(
        "test tool exec failed: {}",
        Command::new(real).args(args).exec()
    );
}
