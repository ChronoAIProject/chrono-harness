use serde_json::{Value, json};
#[path = "../fixtures/transition_decoder.rs"]
mod transition_decoder;
#[path = "../fixtures/transition_decoder_tests.rs"]
mod transition_decoder_tests;
use std::{
    env,
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    os::unix::{ffi::OsStrExt, process::CommandExt},
    path::Path,
    process::{Command, ExitCode, Stdio},
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

fn transition_command(name: &OsStr, args: &[std::ffi::OsString]) -> ExitCode {
    if args.len() == 1 && args[0] == "--version" {
        println!("chrono-workflow-transition 0.1.0");
        return ExitCode::SUCCESS;
    }
    if let Some(trace) = env::var_os("CHRONO_WORKFLOW_TRANSITION_TRACE") {
        let mut line = name.as_bytes().to_vec();
        line.push(b'\n');
        append(Path::new(&trace), &line);
    }
    let execute = || -> Result<(), String> {
        if name == "decoder-tests" {
            if !args.is_empty() {
                return Err("unexpected compatibility arguments".into());
            }
            transition_decoder_tests::check(&read_json(Path::new(".chrono-harness/case.json")));
        } else {
            match args {
                [] => (),
                [flag, path] if flag == "--write" => {
                    fs::write(path, "decoder output").map_err(|error| error.to_string())?;
                }
                _ => return Err("unexpected decoder arguments".into()),
            }
            let input: Value =
                serde_json::from_reader(io::stdin().lock()).map_err(|error| error.to_string())?;
            emit(&transition_decoder::convert(&input)?);
        }
        Ok(())
    };
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("E_MIGRATION: {error}");
            ExitCode::from(1)
        }
    }
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

fn native_inputs(args: &[std::ffi::OsString]) -> ExitCode {
    if args.len() == 1 && args[0] == "--version" {
        println!("chrono-workflow-native-inputs 1");
        return ExitCode::SUCCESS;
    }
    let mut raw = Vec::new();
    io::stdin().read_to_end(&mut raw).unwrap();
    let request: Value = serde_json::from_slice(&raw).unwrap();
    let native_raw = fs::read(".chrono-harness/state/native-transport.json").unwrap();
    let native: std::collections::BTreeMap<String, String> =
        serde_json::from_slice(&native_raw).unwrap();
    let mut child = Command::new(".chrono-harness/bin/chrono-ci")
        .args(args)
        .envs(native)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&raw).unwrap();
    let output = child.wait_with_output().unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
    if output.status.success() {
        let digest = chrono_harness::sha256(&native_raw);
        let path = format!(
            "{}native-transport-{digest}.json",
            request["native_artifacts"]["directory"].as_str().unwrap()
        );
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => file.write_all(&native_raw).unwrap(),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                assert_eq!(fs::read(&path).unwrap(), native_raw);
            }
            Err(error) => panic!("native acquisition original: {error}"),
        }
        let mut prepared: Value = serde_json::from_slice(&output.stdout).unwrap();
        prepared["originals"]
            .as_array_mut()
            .unwrap()
            .push(json!({"path":path,"sha256":digest}));
        emit(&prepared);
    } else {
        io::stdout().write_all(&output.stdout).unwrap();
    }
    ExitCode::from(u8::try_from(output.status.code().unwrap()).unwrap())
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
    if args[0] == "api" {
        let endpoint = args[1].to_str().unwrap();
        if let Some((_, address)) = endpoint.split_once("/contents/") {
            let path = address.split('?').next().unwrap();
            io::stdout()
                .write_all(
                    &fs::read(Path::new(data["workflow_root"].as_str().unwrap()).join(path))
                        .unwrap(),
                )
                .unwrap();
        } else if endpoint.contains("/jobs?") {
            let mut rows = vec![
                json!({"id":200,"run_id":101,"run_attempt":1,"name":"Detect registered DELTA","status":"completed","conclusion":"success","head_sha":data["candidate"]}),
            ];
            for (index, id) in ["one", "two"].iter().enumerate() {
                rows.push(json!({"id":201+index,"run_id":101,"run_attempt":1,"name":format!("Full {id}"),"status":"completed","conclusion":"success","head_sha":data["candidate"]}));
            }
            emit(&json!([{"jobs":rows}]));
        } else if endpoint.contains("/artifacts?") {
            let rows: Vec<_> = ["one","two"].iter().enumerate().map(|(i,id)| json!({"id":300+i,"name":data["productions"][*id]["artifact"],"expired":false,"workflow_run":{"id":101,"head_sha":data["candidate"]},"archive_download_url":format!("https://api.github.com/repos/fixture/full/actions/artifacts/{}/zip",300+i)})).collect();
            emit(&json!([{"artifacts":rows}]));
        } else {
            emit(
                &json!({"id":101,"run_attempt":1,"head_sha":data["candidate"],"event":"push","path":".github/workflows/collection.yml","repository":{"full_name":"fixture/full"}}),
            );
        }
    } else {
        let at = args.iter().position(|arg| arg == "--name").unwrap();
        let name = args[at + 1].to_str().unwrap();
        let unit = ["one", "two"]
            .into_iter()
            .find(|id| data["productions"][*id]["artifact"] == name)
            .unwrap();
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
    if name == "decoder" || name == "decoder-tests" {
        return transition_command(name, &args);
    }
    if name == "fixture-producer" {
        return prepared_producer(&args);
    }
    if name == "native-inputs" {
        return native_inputs(&args);
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
