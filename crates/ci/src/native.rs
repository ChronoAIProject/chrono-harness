//! Native CI context publication, acquisition and forwarding belong to the CI product.
use super::*;
use chrono_harness::sha256;
use std::{collections::BTreeMap, io::Read};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub use super::check_inputs::{NativeChild as Process, native_process as process};

const BOUND: usize = 64 * 1024 * 1024;

fn bytes(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err(format!("native original is not a file: {}", path.display()));
    }
    let mut raw = Vec::new();
    file.take(BOUND as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() > BOUND {
        return Err("native original exceeds byte bound".into());
    }
    Ok(raw)
}
fn read(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    bytes(&no_symlink_parents(root, path)?)
}
fn encode(value: &Value) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|e| e.to_string())
}
fn write(root: &Path, path: &str, raw: &[u8], replace: bool) -> Result<(), String> {
    let target = no_symlink_parents(root, path)?;
    if target.exists() && !replace {
        if fs::read(&target).map_err(|e| e.to_string())? != raw {
            return Err(format!("native original publication collision: {path}"));
        }
        return Ok(());
    }
    super::write_file(root, path, raw)
}
fn retain(root: &Path, directory: &str, prefix: &str, raw: &[u8]) -> Result<Value, String> {
    let path = format!("{directory}{prefix}-{}.json", sha256(raw));
    write(root, &path, raw, false)?;
    Ok(json!({"path":path,"sha256":sha256(raw)}))
}
fn text(value: &Value) -> Result<&str, String> {
    value
        .as_str()
        .ok_or_else(|| "native original string field missing".into())
}
fn array(value: &Value) -> Result<&Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| "native original array field missing".into())
}
fn timestamp(value: &str) -> Result<i128, String> {
    if !(value.ends_with('Z') || value.ends_with("+00:00")) {
        return Err("native context time must be UTC".into());
    }
    if let Some((_, fraction)) = value.split_once('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if !(1..=9).contains(&digits) {
            return Err("native timestamp fraction bound".into());
        }
    }
    if value.as_bytes().get(17..19) == Some(b"60") {
        return Err("native context leap second unsupported".into());
    }
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|v| v.unix_timestamp_nanos())
        .map_err(|e| e.to_string())
}
/// Ordering is a transport check; the workflow judge owns the allowed age.
pub fn validate_observation(context: &Value, now: &str) -> Result<(), String> {
    let started = timestamp(text(&context["branch_started_at"])?)?;
    let observed = timestamp(text(&context["observed_at"])?)?;
    if observed < started || timestamp(now)? < observed {
        return Err("invalid original context observation order".into());
    }
    Ok(())
}
fn now() -> Result<String, String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|e| e.to_string())
}

struct Native<'a> {
    root: &'a Path,
    path: &'a str,
    provider: units::Config,
    environment: BTreeMap<String, String>,
}
impl Native<'_> {
    fn adoption(&self) -> &units::NativeAdoption {
        self.provider
            .native_adoption
            .as_ref()
            .expect("validated native adoption")
    }
    fn selection(&self, unit: Option<&str>) -> Result<(&str, &str, String), String> {
        let contexts = self
            .provider
            .full_contexts
            .as_ref()
            .ok_or("native full contexts absent")?;
        let (artifact, context) = if let Some(unit) = unit {
            let workflow = self.provider.units.get(unit).ok_or("unknown native unit")?;
            (
                &workflow.artifact_directory,
                contexts
                    .units
                    .get(unit)
                    .ok_or("native unit context absent")?,
            )
        } else {
            (
                &self.provider.collection.artifact_directory,
                &contexts.collection,
            )
        };
        Ok((artifact, context, format!("{artifact}preparation/")))
    }
    fn child(
        &self,
        argv: Vec<String>,
        stdin: Vec<u8>,
        environment: BTreeMap<String, String>,
        directory: &str,
        prefix: &str,
        timeout_seconds: f64,
        output_limit_bytes: usize,
    ) -> Result<(Vec<u8>, Vec<Value>), String> {
        process(
            self.root,
            &self.provider,
            &Process {
                argv,
                stdin,
                directory: directory.into(),
                prefix: prefix.into(),
                timeout_seconds,
                output_limit_bytes,
            },
            environment,
        )
    }
    fn observations(&self, unit: Option<&str>) -> Result<(Value, Value), String> {
        let (artifact, _, directory) = self.selection(unit)?;
        let mut keys = vec![
            "GITHUB_EVENT_NAME",
            "GITHUB_EVENT_PATH",
            "CHRONO_WORKFLOW_REVISION",
            "GITHUB_REPOSITORY",
            "GITHUB_RUN_ID",
            "GITHUB_RUN_ATTEMPT",
        ];
        if self.provider.job_gating.is_some() {
            keys.extend(["CHRONO_CI_DETECTION", "GITHUB_JOB"]);
            if self.environment.contains_key("CHRONO_CI_NEEDS") {
                keys.push("CHRONO_CI_NEEDS");
            }
        }
        let actual = keys
            .into_iter()
            .map(|key| {
                self.environment
                    .get(key)
                    .cloned()
                    .map(|v| (key.to_owned(), v))
                    .ok_or_else(|| format!("native acquisition environment missing: {key}"))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let payload = retain(
            self.root,
            &directory,
            "event-payload",
            &bytes(Path::new(&actual["GITHUB_EVENT_PATH"]))?,
        )?;
        let report =
            json!({"schema":"chrono-native-acquisition/v1","observed":actual,"payload":payload});
        let original = retain(self.root, &directory, "acquisition", &encode(&report)?)?;
        write(
            self.root,
            &format!("{artifact}acquisition.json"),
            &encode(&original)?,
            true,
        )?;
        Ok((report, original))
    }
    fn endpoint_args(
        &self,
        observed: &Value,
        unit: Option<&str>,
        command: &str,
    ) -> Result<Vec<String>, String> {
        let mut argv = vec![
            self.provider.collection.generator.clone(),
            command.into(),
            "--host-root".into(),
            self.root.to_str().ok_or("native host root UTF8")?.into(),
            "--config".into(),
            self.path.into(),
            "--event".into(),
            text(&observed["GITHUB_EVENT_NAME"])?.into(),
            "--payload".into(),
            text(&observed["GITHUB_EVENT_PATH"])?.into(),
            "--workflow-revision".into(),
            text(&observed["CHRONO_WORKFLOW_REVISION"])?.into(),
        ];
        if let Some(unit) = unit {
            argv.extend(["--unit".into(), unit.into()]);
        }
        Ok(argv)
    }
    fn publish(&self) -> Result<(), String> {
        let (report, acquisition) = self.observations(None)?;
        let actual = &report["observed"];
        let (_, context_path, directory) = self.selection(None)?;
        let (raw, mut processes) = self.child(
            self.endpoint_args(actual, None, "prepare-endpoints")?,
            vec![],
            self.environment.clone(),
            &directory,
            "event-endpoints",
            self.provider.gather.timeout_seconds as f64,
            self.provider.gather.output_limit_bytes,
        )?;
        let endpoints: Value = decode(&raw)?;
        let birth = &endpoints["lineage"]["birth"];
        let a = self.adoption();
        let lineage = read(self.root, &a.lineage.path)?;
        if sha256(&lineage) != a.lineage.sha256 {
            return Err("original lineage byte drift".into());
        }
        let payload: Value = decode(&read(self.root, text(&report["payload"]["path"])?)?)?;
        let role = match text(&actual["GITHUB_EVENT_NAME"])? {
            "push" => {
                let reference = text(&payload["ref"])?;
                let roles = a
                    .push_roles
                    .iter()
                    .filter(|(prefix, _)| reference.starts_with(prefix.as_str()))
                    .map(|(_, role)| role.as_str())
                    .collect::<Vec<_>>();
                if roles.len() != 1 {
                    return Err("missing/ambiguous host push role".into());
                }
                roles[0]
            }
            "pull_request" => a.pull_request_role.as_str(),
            _ => return Err("shared native seed requires push/pull_request".into()),
        };
        let mut evidence = a.integration_evidence.clone();
        let mut integration = None;
        if role == "delivery" {
            if let Some(path) = &a.integration_evidence_path {
                if self.root.join(path).exists() {
                    let raw = read(self.root, path)?;
                    let digest = sha256(&raw);
                    if a.integration_evidence
                        .as_ref()
                        .is_some_and(|expected| *expected != digest)
                    {
                        return Err("declared integration handoff digest differs".into());
                    }
                    evidence = Some(digest);
                    integration = Some(raw);
                } else {
                    evidence = None;
                }
            }
        }
        let observed = now()?;
        let context = json!({"schema_version":2,"base":endpoints["base"],"candidate":endpoints["candidate"],"dev_tip":endpoints["base"],"branch_ref":birth["branch_ref"],"fork_point":birth["base"],"branch_started_at":birth["branch_started_at"],"observed_at":observed,"operation":"validate.delta","run_kind":role,"integration_evidence":evidence,"retained_inputs":a.retained_inputs});
        validate_observation(&context, &observed)?;
        let seed = encode(&context)?;
        write(self.root, context_path, &seed, true)?;
        for (name, bytes) in [
            ("context.json", seed.clone()),
            ("lineage.json", lineage.clone()),
            ("endpoints.json", raw.clone()),
            (
                "payload.json",
                read(self.root, text(&report["payload"]["path"])?)?,
            ),
        ] {
            write(
                self.root,
                &format!("{}{name}", a.seed_directory),
                &bytes,
                false,
            )?;
        }
        if let Some(raw) = integration {
            write(
                self.root,
                &format!("{}integration.json", a.seed_directory),
                &raw,
                false,
            )?;
        }
        let governance_path = format!("{}inputs.json", a.seed_directory);
        let manifest = retain(
            self.root,
            &directory,
            "governance-sources",
            &encode(
                &json!({"schema":"chrono-input-composition/v1","sources":a.composition_sources.iter().map(|original|json!({"pair":original})).collect::<Vec<_>>()}),
            )?,
        )?;
        let mut environment = self.environment.clone();
        environment.insert("CHRONO_CHECK_SOURCE".into(), "ci".into());
        let (_, refs) = self.child(
            vec![
                a.inputs_program.clone(),
                "capture-governance".into(),
                "--host-root".into(),
                self.root.to_str().ok_or("native host root UTF8")?.into(),
                "--config".into(),
                self.provider.collection.check_config.clone(),
                "--base".into(),
                text(&endpoints["base"])?.into(),
                "--candidate".into(),
                text(&endpoints["candidate"])?.into(),
                "--manifest".into(),
                text(&manifest["path"])?.into(),
                "--output".into(),
                governance_path.clone(),
            ],
            vec![],
            environment,
            &directory,
            "governance-capture",
            900.,
            BOUND,
        )?;
        processes.extend(refs);
        let governance = read(self.root, &governance_path)?;
        let receipt = retain(
            self.root,
            &directory,
            "governance-receipt",
            &read(self.root, &format!("{governance_path}.receipt.json"))?,
        )?;
        let provenance: Value = decode(&read(self.root, text(&receipt["path"])?)?)?;
        let mut originals = vec![acquisition.clone(), report["payload"].clone()];
        originals.extend(processes.clone());
        originals.extend([manifest, receipt]);
        originals.extend(
            array(&provenance["originals"])?
                .iter()
                .map(|item| item["retained"].clone()),
        );
        let mut original_map = BTreeMap::new();
        for original in originals {
            let raw = read(self.root, text(&original["path"])?)?;
            if sha256(&raw) != text(&original["sha256"])? {
                return Err("native seed original identity drift".into());
            }
            let file = format!("originals/{}", text(&original["sha256"])?);
            write(
                self.root,
                &format!("{}{file}", a.seed_directory),
                &raw,
                false,
            )?;
            original_map.insert(
                text(&original["path"])?.to_owned(),
                json!({"file":file,"sha256":original["sha256"]}),
            );
        }
        let integer = |key: &str| {
            text(&actual[key])?
                .parse::<u64>()
                .map_err(|e| format!("native {key}: {e}"))
        };
        let binding = json!({"schema":"chrono-native-seed/v1","event":actual["GITHUB_EVENT_NAME"],"repository":actual["GITHUB_REPOSITORY"],"base":endpoints["base"],"candidate":endpoints["candidate"],"workflow":self.provider.collection.workflow_path,"workflow_revision":actual["CHRONO_WORKFLOW_REVISION"],"run":integer("GITHUB_RUN_ID")?,"attempt":integer("GITHUB_RUN_ATTEMPT")?,"context_sha256":sha256(&seed),"lineage_sha256":sha256(&lineage),"integration_sha256":evidence,"payload_sha256":report["payload"]["sha256"],"endpoints_sha256":sha256(&raw),"acquisition":acquisition,"processes":processes,"governance_sha256":sha256(&governance),"originals":original_map});
        write(
            self.root,
            &format!("{}binding.json", a.seed_directory),
            &encode(&binding)?,
            false,
        )
    }
    fn acquire(&self, unit: Option<&str>) -> Result<(), String> {
        let (report, _) = self.observations(unit)?;
        let (_, context, directory) = self.selection(unit)?;
        let mut argv = self.endpoint_args(&report["observed"], unit, "acquire-seed")?;
        argv.extend([
            "--repository".into(),
            text(&report["observed"]["GITHUB_REPOSITORY"])?.into(),
        ]);
        self.child(
            argv,
            vec![],
            self.environment.clone(),
            &directory,
            "seed-acquisition",
            900.,
            BOUND,
        )?;
        validate_observation(&decode(&read(self.root, context)?)?, &now()?)
    }
    fn artifact_json(
        &self,
        artifacts: &Value,
        address: &str,
        transport: &Value,
    ) -> Result<Value, String> {
        let a = &artifacts[address];
        let raw = if let Some(hex) = a["hex"].as_str() {
            if hex.len() % 2 != 0 {
                return Err("native original hex length".into());
            }
            hex.as_bytes()
                .chunks_exact(2)
                .map(|pair| {
                    std::str::from_utf8(pair)
                        .map_err(|e| e.to_string())
                        .and_then(|s| u8::from_str_radix(s, 16).map_err(|e| e.to_string()))
                })
                .collect::<Result<Vec<u8>, String>>()?
        } else {
            if a["schema"] != "chrono-retained-blob/v1" {
                return Err("external original schema".into());
            }
            let mut path = text(&a["storage"])?.to_owned();
            if !transport.is_null() {
                let suffix = path
                    .strip_prefix(text(&transport["source_directory"])?)
                    .ok_or("external original outside transport root")?;
                path = format!("{}{suffix}", text(&transport["directory"])?);
            }
            read(self.root, &path)?
        };
        if sha256(&raw) != text(&a["sha256"])? || a["length"].as_u64() != Some(raw.len() as u64) {
            return Err("external original identity".into());
        }
        decode(&raw)
    }
    fn composition(
        &self,
        prepared: &Value,
        directory: &str,
        environment: BTreeMap<String, String>,
    ) -> Result<Vec<Value>, String> {
        let a = self.adoption();
        let manifest: Value = decode(&read(self.root, &self.provider.gather.manifest_path)?)?;
        let seed_directory = format!("{}seed/", self.provider.collection.artifact_directory);
        let binding: Value = decode(&read(self.root, &format!("{seed_directory}binding.json"))?)?;
        let mut sources = vec![
            json!({"pair":{"path":format!("{}inputs.json",a.seed_directory),"sha256":binding["governance_sha256"]},"transport":{"source_directory":a.seed_directory,"directory":seed_directory}}),
        ];
        for item in array(&manifest["reports"])? {
            let raw = read(self.root, text(&item["path"])?)?;
            if sha256(&raw) != text(&item["sha256"])? {
                return Err("gather report identity drift".into());
            }
            let report: Value = decode(&raw)?;
            let context = self.artifact_json(
                &report["artifacts"],
                text(&report["request"]["context"]["path"])?,
                &item["artifacts"],
            )?;
            let pair = text(&context["retained_inputs"])?;
            sources.push(json!({"pair":{"path":pair,"sha256":report["artifacts"][pair]["sha256"]},"transport":item["artifacts"],"artifacts":report["artifacts"]}));
        }
        let manifest = retain(
            self.root,
            directory,
            "composition-manifest",
            &encode(&json!({"schema":"chrono-input-composition/v1","sources":sources}))?,
        )?;
        let output = format!(
            ".chrono-harness/state/inputs/composed-{}.json",
            text(&manifest["sha256"])?
        );
        let receipt = format!("{output}.receipt.json");
        let (_, refs) = self.child(
            vec![
                a.inputs_program.clone(),
                "compose".into(),
                "--host-root".into(),
                self.root.to_str().ok_or("native host root UTF8")?.into(),
                "--config".into(),
                text(&prepared["profile"])?.into(),
                "--base".into(),
                text(&prepared["base"])?.into(),
                "--candidate".into(),
                text(&prepared["candidate"])?.into(),
                "--manifest".into(),
                text(&manifest["path"])?.into(),
                "--output".into(),
                output.clone(),
                "--receipt".into(),
                receipt.clone(),
            ],
            vec![],
            environment,
            directory,
            "composition",
            900.,
            BOUND,
        )?;
        write(
            self.root,
            &a.retained_inputs,
            &read(self.root, &output)?,
            true,
        )?;
        let provenance: Value = decode(&read(self.root, &receipt)?)?;
        let mut originals = vec![manifest];
        originals.extend(refs);
        for path in [&output, &receipt, &a.retained_inputs] {
            originals.push(retain(
                self.root,
                directory,
                "composition-original",
                &read(self.root, path)?,
            )?);
        }
        for original in array(&provenance["originals"])? {
            originals.push(retain(
                self.root,
                directory,
                "constituent",
                &read(self.root, text(&original["retained"]["path"])?)?,
            )?);
        }
        Ok(originals)
    }
    fn forward(&self, input: &[u8]) -> Result<String, String> {
        let request: Value = decode(input)?;
        let requested_root =
            fs::canonicalize(text(&request["host_root"])?).map_err(|e| e.to_string())?;
        if requested_root != self.root || request["source"] != "ci" {
            return Err("forwarding source/root mismatch".into());
        }
        let unit = request["selection"]["unit"].as_str();
        let (artifact, context, directory) = self.selection(unit)?;
        let acquisition: Value = decode(&read(self.root, &format!("{artifact}acquisition.json"))?)?;
        let raw = read(self.root, text(&acquisition["path"])?)?;
        if sha256(&raw) != text(&acquisition["sha256"])? {
            return Err("acquisition original drift".into());
        }
        let acquired: Value = decode(&raw)?;
        let actual = &acquired["observed"];
        let payload = &acquired["payload"];
        if sha256(&read(self.root, text(&payload["path"])?)?) != text(&payload["sha256"])? {
            return Err("event original drift".into());
        }
        let mut environment = self.environment.clone();
        for key in [
            "GITHUB_EVENT_NAME",
            "CHRONO_WORKFLOW_REVISION",
            "GITHUB_REPOSITORY",
        ] {
            environment.insert(key.into(), text(&actual[key])?.into());
        }
        environment.insert(
            "GITHUB_EVENT_PATH".into(),
            self.root
                .join(text(&payload["path"])?)
                .to_str()
                .ok_or("native event path UTF8")?
                .into(),
        );
        for key in [
            "GITHUB_RUN_ID",
            "GITHUB_RUN_ATTEMPT",
            "GITHUB_JOB",
            "CHRONO_CI_DETECTION",
            "CHRONO_CI_NEEDS",
        ] {
            if let Some(value) = actual.get(key) {
                environment.insert(key.into(), text(value)?.into());
            }
        }
        validate_observation(&decode(&read(self.root, context)?)?, &now()?)?;
        let argv = vec![
            self.provider.collection.generator.clone(),
            "check-inputs".into(),
            "--config".into(),
            self.path.into(),
            "--event-env".into(),
            "GITHUB_EVENT_NAME".into(),
            "--payload-env".into(),
            "GITHUB_EVENT_PATH".into(),
            "--revision-env".into(),
            "CHRONO_WORKFLOW_REVISION".into(),
            "--repository-env".into(),
            "GITHUB_REPOSITORY".into(),
        ];
        let (stdout, refs) = self.child(
            argv,
            input.to_vec(),
            environment.clone(),
            &directory,
            "forward",
            900.,
            BOUND,
        )?;
        let mut prepared: Value = decode(&stdout)?;
        let mut report: Value = decode(&read(
            self.root,
            text(&prepared["evidence"]["report_path"])?,
        )?)?;
        let mut originals = array(&prepared["originals"])?.clone();
        originals.extend(refs.clone());
        originals.extend([acquisition.clone(), payload.clone()]);
        let seed_directory = format!("{artifact}seed/");
        for name in [
            "context.json",
            "binding.json",
            "lineage.json",
            "payload.json",
            "endpoints.json",
        ] {
            originals.push(retain(
                self.root,
                &directory,
                "seed-original",
                &read(self.root, &format!("{seed_directory}{name}"))?,
            )?);
        }
        let binding: Value = decode(&read(self.root, &format!("{seed_directory}binding.json"))?)?;
        if !binding["integration_sha256"].is_null()
            && self.adoption().integration_evidence_path.is_some()
        {
            let raw = read(self.root, &format!("{seed_directory}integration.json"))?;
            if sha256(&raw) != text(&binding["integration_sha256"])? {
                return Err("original integration handoff drift".into());
            }
            originals.push(retain(
                self.root,
                &directory,
                "seed-integration-original",
                &raw,
            )?);
        }
        for original in binding["originals"]
            .as_object()
            .ok_or("native seed original map missing")?
            .values()
        {
            let raw = read(
                self.root,
                &format!("{seed_directory}{}", text(&original["file"])?),
            )?;
            if sha256(&raw) != text(&original["sha256"])? {
                return Err("seed original publication closure drift".into());
            }
            originals.push(retain(
                self.root,
                &directory,
                "seed-producer-original",
                &raw,
            )?);
        }
        if request["selection"]["kind"] == "collect" {
            originals.extend(self.composition(&prepared, &directory, environment)?);
        }
        report["forwarding"] = json!({"acquisition":acquisition,"process":refs.last().ok_or("native process original absent")?,"child_report":prepared["evidence"]});
        let mut by_path = BTreeMap::new();
        for original in originals {
            let path = text(&original["path"])?.to_owned();
            if by_path.get(&path).is_some_and(|prior| prior != &original) {
                return Err("original address conflict".into());
            }
            by_path.insert(path, original);
        }
        let mut originals = by_path.into_values().collect::<Vec<_>>();
        report["originals"] = json!(originals);
        let outer = retain(self.root, &directory, "adapter-result", &encode(&report)?)?;
        prepared["evidence"]["report_path"] = outer["path"].clone();
        prepared["evidence"]["report_sha256"] = outer["sha256"].clone();
        originals.push(outer);
        prepared["originals"] = json!(originals);
        Ok(serde_json::to_string(&prepared).map_err(|e| e.to_string())? + "\n")
    }
}

pub(crate) fn dispatch(args: &[String]) -> Result<String, String> {
    let operation = args
        .get(1)
        .ok_or("native-adoption requires publish/acquire/forward")?;
    if !matches!(operation.as_str(), "publish" | "acquire" | "forward") {
        return Err("unknown native adoption operation".into());
    }
    let mut options = BTreeMap::new();
    let mut i = 2;
    while i < args.len() {
        let key = args[i].as_str();
        let value = args
            .get(i + 1)
            .ok_or("native adoption missing option value")?;
        if !["--host-root", "--config", "--unit"].contains(&key)
            || options.insert(key, value.as_str()).is_some()
        {
            return Err(format!("unknown/duplicate native option: {key}"));
        }
        i += 2;
    }
    if operation != "acquire" && options.contains_key("--unit") {
        return Err("native unit option belongs to acquire".into());
    }
    let root = fs::canonicalize(
        options
            .get("--host-root")
            .ok_or("native host-root required")?,
    )
    .map_err(|e| e.to_string())?;
    let path = *options.get("--config").ok_or("native config required")?;
    let provider: units::Config = decode(&read(&root, path)?)?;
    units::validate(&provider)?;
    if provider.native_adoption.is_none() {
        return Err("native adoption absent".into());
    }
    let native = Native {
        root: &root,
        path,
        provider,
        environment: std::env::vars().collect(),
    };
    match operation.as_str() {
        "publish" => {
            native.publish()?;
            Ok(String::new())
        }
        "acquire" => {
            native.acquire(options.get("--unit").copied())?;
            Ok(String::new())
        }
        "forward" => {
            let mut input = Vec::new();
            std::io::stdin()
                .take(BOUND as u64 + 1)
                .read_to_end(&mut input)
                .map_err(|e| e.to_string())?;
            if input.len() > BOUND {
                return Err("native input request bound".into());
            }
            native.forward(&input)
        }
        _ => unreachable!(),
    }
}
