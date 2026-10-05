use serde_json::{Value, json};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn read(path: &str) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.metadata().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn option<'a>(args: &'a [String], name: &str) -> &'a str {
    &args[args.iter().position(|value| value == name).unwrap() + 1]
}

fn contents(endpoint: &str, wrong_source: bool) {
    let path = endpoint.split_once("/contents/").unwrap().1;
    let path = path.split('?').next().unwrap();
    let bytes = if wrong_source {
        b"wrong workflow".to_vec()
    } else {
        fs::read(path).unwrap()
    };
    io::stdout().write_all(&bytes).unwrap();
}

pub fn units(args: &[String]) {
    let data = read(".chrono-harness/state/mock.json");
    let run = |unit: &str| {
        json!({"id":if unit == "alpha" {101} else {102},"run_attempt":1,
            "status":"completed","conclusion":"success","head_sha":data["candidate"],
            "event":"push","path":format!(".github/workflows/{unit}.yml")})
    };
    if args[0] == "run" {
        let unit = if args[2] == "101" { "alpha" } else { "beta" };
        copy_tree(
            &PathBuf::from(".chrono-harness/state/mock-artifacts").join(unit),
            Path::new(option(args, "--dir")),
        );
    } else if args[1].contains("/actions/workflows/") {
        let unit = if args[1].contains("/alpha.yml/") {
            "alpha"
        } else {
            "beta"
        };
        let mut rows = vec![run(unit)];
        if data["mode"] == "duplicate" {
            let mut duplicate = rows[0].clone();
            duplicate["id"] = json!(201);
            rows.push(duplicate);
        }
        println!("{}", json!([{"workflow_runs":rows}]));
    } else if args[1].contains("/contents/") {
        contents(&args[1], data["mode"] == "wrong-source");
    } else if args[1].contains("/actions/runs/") {
        let mut item = run(if args[1].ends_with("/101") {
            "alpha"
        } else {
            "beta"
        });
        if data["mode"] == "changed-attempt" {
            item["run_attempt"] = json!(2);
        }
        println!("{item}");
    } else {
        std::process::exit(7);
    }
}

pub fn parent(args: &[String]) {
    let data = read(".chrono-harness/state/parent-data.json");
    let detection = &data["detection"];
    let mode = data["mode"].as_str().unwrap();
    let provider = read(".chrono-harness/ci/units.json");
    let mut calls = OpenOptions::new()
        .create(true)
        .append(true)
        .open(".chrono-harness/state/parent-calls")
        .unwrap();
    serde_json::to_writer(&mut calls, args).unwrap();
    writeln!(calls).unwrap();
    if args[0] == "run" {
        let artifact = option(args, "--name").split_once("chrono-unit-").unwrap().1;
        let unit = artifact.rsplitn(3, '-').nth(2).unwrap();
        if mode == "missing-artifact" {
            std::process::exit(7);
        }
        copy_tree(
            &PathBuf::from(".chrono-harness/state/mock-artifacts").join(unit),
            Path::new(option(args, "--dir")),
        );
    } else if args[1].contains("/contents/") {
        contents(&args[1], mode == "wrong-source");
    } else if args[1].contains("/jobs?") {
        let mut rows = vec![json!({"id":200,"run_id":101,"run_attempt":2,
            "name":"Detect registered DELTA","status":"completed", "conclusion":"success",
            "head_sha":detection["candidate"]})];
        let mut units: Vec<_> = provider["units"].as_object().unwrap().keys().collect();
        units.sort();
        for (index, unit) in units.iter().enumerate() {
            let required = detection["required_units"]
                .as_array()
                .unwrap()
                .contains(&json!(unit));
            if mode == "skipped-jobs-omitted" && !required {
                continue;
            }
            let attempt = if mode == "job-attempt"
                || (matches!(mode, "retry" | "carried" | "latest-failure") && *unit == "alpha")
            {
                3
            } else {
                2
            };
            rows.push(json!({"id":201+index,"run_id":101,"run_attempt":attempt,
                "name":provider["units"][*unit]["name"],"head_sha":detection["candidate"],
                "status":"completed","conclusion":if required {"success"} else {"skipped"}}));
        }
        if mode == "carried" {
            rows[0]["run_attempt"] = json!(3);
            let mut previous = rows[rows.len() - 2].clone();
            previous["id"] = json!(301);
            previous["run_attempt"] = json!(2);
            rows.push(previous);
        }
        if mode == "latest-failure" {
            rows[1]["conclusion"] = json!("failure");
        }
        if mode == "aggregate-only" {
            for row in &mut rows {
                row["run_attempt"] = json!(3);
            }
        }
        if mode == "missing-job" {
            rows.remove(1);
        }
        println!("{}", json!([{"jobs":rows}]));
    } else if args[1].contains("/actions/runs/") {
        let attempt = if matches!(
            mode,
            "changed-attempt" | "retry" | "carried" | "latest-failure" | "aggregate-only"
        ) {
            3
        } else {
            2
        };
        println!(
            "{}",
            json!({"id":if mode == "wrong-run" {999} else {101},
            "run_attempt":attempt,"status":"in_progress","head_sha":detection["candidate"],
            "event":detection["event"],"path":provider["collection"]["workflow_path"],
            "repository":{"full_name":"owner/host"}})
        );
    } else {
        std::process::exit(8);
    }
}
