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

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
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

fn native_seed_api(parent: &Path, args: &[std::ffi::OsString]) -> ExitCode {
    let data = read_json(&parent.join("seed-api.json"));
    let case = data["case"].as_str().unwrap_or("valid");
    let mut run = json!({"id":data.get("seed_run").unwrap_or(&json!(900)),
        "run_attempt":data.get("attempt").unwrap_or(&json!(1)),
        "status":"in_progress", "head_sha":data["candidate"],
        "event":data.get("event").unwrap_or(&json!("push")),
        "path":".github/workflows/collection.yml", "repository":{"full_name":"fixture/native"}});
    let argument = |flag: &str| {
        let position = args.iter().position(|arg| arg == flag).unwrap();
        args[position + 1].to_str().unwrap()
    };
    if args.first().is_some_and(|arg| arg == "run")
        && args.get(1).is_some_and(|arg| arg == "download")
    {
        let name = argument("--name");
        let unit = name
            .strip_prefix("chrono-unit-")
            .map(|suffix| suffix.rsplitn(3, '-').nth(2).unwrap());
        let upload = unit.map_or(&data["upload"], |unit| &data["units"][unit]["upload"]);
        let destination = Path::new(argument("--dir"));
        copy_tree(Path::new(upload.as_str().unwrap()), destination);
        if matches!(
            case,
            "wrong-base"
                | "wrong-attempt"
                | "wrong-workflow"
                | "wrong-digest"
                | "wrong-fork"
                | "stale"
                | "nested-loss"
                | "wrong-lineage"
                | "wrong-branch"
                | "wrong-repository"
                | "wrong-candidate"
        ) {
            let mut binding = read_json(&destination.join("binding.json"));
            let mut context = read_json(&destination.join("context.json"));
            match case {
                "wrong-base" => binding["base"] = json!("f".repeat(40)),
                "wrong-attempt" => binding["attempt"] = json!(2),
                "wrong-workflow" => binding["workflow_revision"] = json!("e".repeat(40)),
                "wrong-digest" => binding["context_sha256"] = json!("0".repeat(64)),
                "wrong-repository" => binding["repository"] = json!("other/repository"),
                "wrong-candidate" => binding["candidate"] = json!("e".repeat(40)),
                "wrong-lineage" => {
                    fs::write(destination.join("lineage.json"), b"original byte drift").unwrap()
                }
                "wrong-branch" => context["branch_ref"] = json!("feature/wrong"),
                "wrong-fork" => context["fork_point"] = json!("d".repeat(40)),
                "stale" => context["observed_at"] = json!("2020-01-01T00:00:00Z"),
                "nested-loss" => {
                    let first = binding["originals"]
                        .as_object()
                        .unwrap()
                        .values()
                        .next()
                        .unwrap();
                    fs::remove_file(destination.join(first["file"].as_str().unwrap())).unwrap();
                }
                _ => unreachable!(),
            }
            if matches!(case, "wrong-fork" | "stale" | "wrong-branch") {
                let raw = serde_json::to_vec(&context).unwrap();
                fs::write(destination.join("context.json"), &raw).unwrap();
                binding["context_sha256"] = json!(chrono_harness::sha256(&raw));
            }
            write_json(&destination.join("binding.json"), &binding);
        }
        return ExitCode::SUCCESS;
    }
    let endpoint = args[1].to_str().unwrap();
    let workflow = Path::new(data["workflow"].as_str().unwrap());
    let root = workflow.ancestors().nth(3).unwrap();
    if let Some((_, address)) = endpoint.split_once("/contents/") {
        let filename = address.split('?').next().unwrap();
        io::stdout()
            .write_all(&fs::read(root.join(filename)).unwrap())
            .unwrap();
    } else if endpoint.contains("/artifacts?") {
        let artifact = json!({"name":format!("chrono-context-{}-1", run["id"]), "expired":false});
        emit(
            &json!([{"artifacts":if case == "missing" {vec![]} else {vec![artifact.clone()]}},
            {"artifacts":if case == "ambiguous" {vec![artifact]} else {vec![]}}]),
        );
    } else if endpoint.contains("/jobs?") {
        let mut rows = vec![
            json!({"id":200,"run_id":run["id"],"run_attempt":run["run_attempt"],
            "name":"Detect registered DELTA", "status":"completed", "conclusion":"success", "head_sha":run["head_sha"]}),
        ];
        let provider = read_json(&root.join(".chrono-harness/ci/units.json"));
        if let Some(units) = data["units"].as_object() {
            for (index, (unit, observation)) in units.iter().enumerate() {
                let attempt = if data["carried"].as_bool().unwrap_or(false) {
                    run["run_attempt"].clone()
                } else {
                    observation.get("attempt").cloned().unwrap_or(json!(1))
                };
                rows.push(json!({"id":201+index,"run_id":run["id"],"run_attempt":attempt,
                    "name":provider["units"][unit]["name"],"status":"completed","conclusion":"success","head_sha":run["head_sha"]}));
            }
        }
        let split = rows.len().min(2);
        emit(&json!([{"jobs":rows[..split]}, {"jobs":rows[split..]}]));
    } else {
        if case == "run-race" && args[0] == "api" && endpoint.ends_with("/runs/900") {
            run["run_attempt"] = json!(2);
        }
        emit(&run);
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
        return native_seed_api(parent, &args);
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
