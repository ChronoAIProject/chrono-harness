//! Verify explicit Cargo/libtest groups; never discover dependencies or select tests.
use chrono_harness::{CommandSpec, decode, file_identity, no_symlink_parents, sha256};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: String,
    projects: String,
    project: String,
    unfiltered_action: String,
    groups: BTreeMap<String, String>,
    binaries: BTreeMap<String, Vec<String>>,
    tools: BTreeMap<String, String>,
    report_directory: String,
    timeout_seconds: f64,
    output_limit_bytes: usize,
}

fn string(value: &Value) -> Result<&str, String> {
    value
        .as_str()
        .ok_or_else(|| "inventory string field missing".into())
}

fn parse_listing(
    stdout: &str,
    stderr: &str,
    binaries: &[String],
) -> Result<BTreeSet<String>, String> {
    let observed = stderr
        .lines()
        .filter_map(|line| line.split_once("Running "))
        .map(|(_, line)| {
            let path = line
                .rsplit_once('(')
                .and_then(|(_, path)| path.strip_suffix(')'))
                .ok_or("malformed listing executable")?;
            let name = Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("malformed listing executable")?;
            Ok(match name.rsplit_once('-') {
                Some((name, hash))
                    if !hash.is_empty()
                        && hash
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
                {
                    name
                }
                _ => name,
            }
            .to_owned())
        })
        .collect::<Result<Vec<_>, String>>()?;
    if observed != binaries {
        return Err(format!("listing binaries differ: {observed:?}"));
    }
    let mut suites = Vec::new();
    let mut names = Vec::new();
    for line in stdout.lines() {
        if let Some(name) = line.strip_suffix(": test") {
            names.push(name.to_owned());
        } else if let Some((tests, benches)) = line.split_once(", ").and_then(|(tests, benches)| {
            let number = |text: &str, singular: &str, plural: &str| {
                text.strip_suffix(singular)
                    .or_else(|| text.strip_suffix(plural))?
                    .parse::<usize>()
                    .ok()
            };
            Some((
                number(tests, " test", " tests")?,
                number(benches, " benchmark", " benchmarks")?,
            ))
        }) {
            if benches != 0
                || tests != names.len()
                || names.iter().collect::<BTreeSet<_>>().len() != names.len()
            {
                return Err("malformed/duplicate libtest listing".into());
            }
            suites.push(std::mem::take(&mut names));
        } else if !line.trim().is_empty() {
            return Err(format!("unexpected libtest listing: {line}"));
        }
    }
    if !names.is_empty() || suites.len() != binaries.len() {
        return Err("incomplete libtest listing".into());
    }
    Ok(binaries
        .iter()
        .zip(suites)
        .flat_map(|(binary, suite)| {
            suite
                .into_iter()
                .map(move |name| format!("{binary}::{name}"))
        })
        .collect())
}

struct Inventory<'a> {
    root: &'a Path,
    config: Config,
    project: &'a Value,
    receipts: Vec<Value>,
}
impl Inventory<'_> {
    fn write(&self, name: &str, bytes: &[u8]) -> Result<String, String> {
        let path = format!("{}/{name}", self.config.report_directory);
        let destination = no_symlink_parents(self.root, &path)?;
        fs::create_dir_all(
            destination
                .parent()
                .ok_or("inventory output parent missing")?,
        )
        .map_err(|e| e.to_string())?;
        fs::write(destination, bytes).map_err(|e| e.to_string())?;
        Ok(path)
    }
    fn listing(&mut self, action: &str, ignored: bool) -> Result<BTreeSet<String>, String> {
        let row = &self.project["actions"][action];
        let tool = string(&row["tool"])?;
        let program = self
            .config
            .tools
            .get(tool)
            .ok_or_else(|| format!("inventory tool missing: {tool}"))?;
        let mut argv = vec![program.clone()];
        for arg in row["argv"]
            .as_array()
            .ok_or("inventory action argv missing")?
        {
            argv.push(string(arg)?.to_owned());
        }
        if !argv.iter().any(|arg| arg == "--") {
            argv.push("--".into());
        }
        if ignored {
            argv.push("--ignored".into());
        }
        argv.push("--list".into());
        // Action IDs also name retained original files; validate before spawning.
        chrono_harness::relative_path(action)?;
        let label = format!("{action}{}", if ignored { "-ignored" } else { "" });
        let environment: BTreeMap<_, _> = std::env::vars().collect();
        let executable = chrono_harness::resolve_program(
            self.root,
            program,
            environment.get("PATH").map(String::as_str),
        )?;
        let spec = CommandSpec {
            program: executable
                .to_str()
                .ok_or("inventory executable UTF8")?
                .into(),
            args: argv[1..].to_vec(),
            env: environment,
            timeout_seconds: self.config.timeout_seconds.ceil() as u64,
            output_limit_bytes: self.config.output_limit_bytes,
        };
        let start = Instant::now();
        let process = chrono_harness::run_process_observed_for(
            self.root,
            &spec,
            &[],
            &file_identity(&executable)?.0,
            Duration::from_secs_f64(self.config.timeout_seconds),
        )?;
        let stdout = self.write(&format!("{label}.stdout"), &process.stdout_bytes)?;
        let stderr = self.write(&format!("{label}.stderr"), &process.stderr_bytes)?;
        self.receipts.push(
            json!({"action":action,"operation":string(&row["operation"])?,
            "argv":argv,"exit_code":process.exit_code,"error":process.failure,
            "duration_seconds":start.elapsed().as_secs_f64(),"stdout":stdout,"stderr":stderr,
            "stdout_sha256":process.stdout_sha256,"stderr_sha256":process.stderr_sha256}),
        );
        self.write(
            "receipts.json",
            &serde_json::to_vec_pretty(&self.receipts).map_err(|e| e.to_string())?,
        )?;
        if let Some(failure) = process.failure {
            return Err(format!("failed listing: {label}: {failure}"));
        }
        if process.exit_code != 0 {
            return Err(format!(
                "failed listing: {label} exit {}",
                process.exit_code
            ));
        }
        parse_listing(
            std::str::from_utf8(&process.stdout_bytes).map_err(|e| e.to_string())?,
            std::str::from_utf8(&process.stderr_bytes).map_err(|e| e.to_string())?,
            self.config
                .binaries
                .get(action)
                .ok_or("inventory action binaries missing")?,
        )
    }
}

/// Consume only the declared project, actions, groups, tool bindings and bounds.
pub fn execute(root: &Path, path: &str) -> Result<String, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let config_bytes = fs::read(no_symlink_parents(&root, path)?).map_err(|e| e.to_string())?;
    let config: Config = decode(&config_bytes)?;
    if config.schema != "chrono-workflow-inventory/v1" {
        return Err("unsupported inventory schema".into());
    }
    if Duration::try_from_secs_f64(config.timeout_seconds).is_err()
        || config.timeout_seconds <= 0.0
        || config.output_limit_bytes == 0
    {
        return Err("invalid inventory process bounds".into());
    }
    no_symlink_parents(&root, &config.report_directory)?;
    let projects_bytes =
        fs::read(no_symlink_parents(&root, &config.projects)?).map_err(|e| e.to_string())?;
    let projects: Value = decode(&projects_bytes)?;
    let rows: Vec<_> = projects["projects"]
        .as_array()
        .ok_or("inventory projects missing")?
        .iter()
        .filter(|row| row["id"].as_str() == Some(&config.project))
        .collect();
    if rows.len() != 1 || rows[0]["test_groups"] != json!(config.groups) {
        return Err("inventory group binding drift".into());
    }
    let all = config.unfiltered_action.clone();
    let groups = config.groups.clone();
    let mut inventory = Inventory {
        root: &root,
        config,
        project: rows[0],
        receipts: Vec::new(),
    };
    let complete = inventory.listing(&all, false)?;
    let ignored = inventory.listing(&all, true)?;
    let observed = groups
        .iter()
        .map(|(id, action)| Ok((id.clone(), inventory.listing(action, false)?)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    if !ignored.is_empty() {
        return Err("ignored tests cannot establish complete coverage".into());
    }
    if complete.is_empty() || observed.is_empty() || observed.values().any(BTreeSet::is_empty) {
        return Err("empty inventory/group".into());
    }
    let mut union = BTreeSet::new();
    for (id, group) in &observed {
        if group.iter().any(|test| union.contains(test)) {
            return Err(format!("overlapping inventory group: {id}"));
        }
        union.extend(group.iter().cloned());
    }
    if union != complete {
        return Err(format!(
            "inventory omission/addition: {:?}",
            union.symmetric_difference(&complete)
        ));
    }
    let report = json!({"schema":inventory.config.schema,"project":inventory.config.project,
        "config_sha256":sha256(&config_bytes),"projects_sha256":sha256(&projects_bytes),
        "complete":complete,"groups":observed,"receipts":inventory.receipts});
    inventory.write(
        "inventory.json",
        &serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )?;
    Ok(format!(
        "{}\n",
        json!({"complete":complete.len(),"groups":observed.iter()
        .map(|(id,tests)|(id,tests.len())).collect::<BTreeMap<_,_>>()})
    ))
}
