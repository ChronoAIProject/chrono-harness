//! Each generation owns its original inputs, plan, effects and recovery reference.
use super::verified_generation::{self as verified, Object, same_object};
use chrono_harness::{
    artifact_disposal::identity,
    decode, no_symlink_parents, relative_path,
    retained_artifacts::retention::{Inventory, Publication, unique_path},
    sha256,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const STORE: &str = ".chrono-harness/state/ci-generation-operations";
const PRODUCER: &str = "ci-generation-operation";
const POLICY: &[u8] = include_bytes!("../../../assets/ci/standalone-retention.json");
const LIMIT: u64 = 16 * 1024 * 1024;

fn checkpoint(_point: &str) {
    #[cfg(feature = "generation-test")]
    if std::env::var("CHRONO_CI_GENERATION_TEST_CHECKPOINT")
        .ok()
        .as_deref()
        == Some(_point)
    {
        use std::io::Write;
        writeln!(std::io::stdout(), "generation-checkpoint:{_point}").unwrap();
        std::io::stdout().flush().unwrap();
        loop {
            std::thread::park();
        }
    }
}

struct Original {
    path: PathBuf,
    object: Option<Object>,
    bytes: Option<Vec<u8>>,
}
fn observe(path: &Path) -> Result<(Option<Object>, Option<Vec<u8>>), String> {
    let before = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => identity(path)?,
        Ok(_) => {
            return Err(format!(
                "not a regular generation input/output: {}",
                path.display()
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((None, None)),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let (_, observed) = verified::read_present_original(
        (),
        before,
        LIMIT,
        |state| {
            (
                state,
                (|| {
                    let file =
                        fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
                    let mut bytes = Vec::new();
                    file.take(LIMIT + 1)
                        .read_to_end(&mut bytes)
                        .map_err(|e| e.to_string())?;
                    Ok(bytes)
                })(),
            )
        },
        |state| (state, identity(path)),
        |state, bytes: &Vec<u8>| (state, sha256(bytes)),
        |state| {
            (
                state,
                format!(
                    "generation object changed while reading: {}",
                    path.display()
                ),
            )
        },
    );
    observed
}

pub(super) struct Inputs {
    root: PathBuf,
    originals: BTreeMap<String, Original>,
}
impl Inputs {
    pub fn read(&mut self, path: &str) -> Result<Vec<u8>, String> {
        self.optional(path)?
            .ok_or_else(|| format!("generation input unavailable: {path}"))
    }
    pub fn optional(&mut self, path: &str) -> Result<Option<Vec<u8>>, String> {
        let physical = no_symlink_parents(&self.root, path)?;
        self.capture(path, physical)
    }
    pub fn external(&mut self, path: &Path) -> Result<Vec<u8>, String> {
        let requested = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(path)
        };
        let physical =
            fs::canonicalize(&requested).map_err(|e| format!("{}: {e}", path.display()))?;
        self.capture(
            requested.to_str().ok_or("generation input path UTF8")?,
            physical,
        )?
        .ok_or_else(|| format!("generation input unavailable: {}", path.display()))
    }
    fn capture(&mut self, key: &str, path: PathBuf) -> Result<Option<Vec<u8>>, String> {
        let (object, bytes) = observe(&path)?;
        if let Some(previous) = self.originals.get(key) {
            if previous.path != path || !same_object(previous.object.as_ref(), object.as_ref()) {
                return Err(format!(
                    "generation input changed during preparation: {key}"
                ));
            }
        } else {
            self.originals.insert(
                key.into(),
                Original {
                    path,
                    object,
                    bytes: bytes.clone(),
                },
            );
        }
        Ok(bytes)
    }
    fn stable(&self) -> Result<(), String> {
        for (key, original) in &self.originals {
            let resolved = if Path::new(key).is_absolute() {
                fs::canonicalize(key).map_err(|e| e.to_string())?
            } else {
                no_symlink_parents(&self.root, key)?
            };
            if resolved != original.path {
                return Err(format!("generation input resolution changed: {key}"));
            }
            if !same_object(
                observe(&original.path)?.0.as_ref(),
                original.object.as_ref(),
            ) {
                return Err(format!(
                    "generation input changed before publication: {key}"
                ));
            }
        }
        Ok(())
    }
    fn retain(&self, publication: &Publication, directory: &Path) -> Result<Value, String> {
        let mut records = Vec::new();
        for (index, (requested, original)) in self.originals.iter().enumerate() {
            let name = format!("input-{index}.bytes");
            if let Some(bytes) = &original.bytes {
                publication.write_file(&directory.join(&name), bytes)?;
            }
            records.push(
                json!({"requested":requested,"resolved":original.path,"object":original.object,
                "original":original.bytes.as_ref().map(|_| name)}),
            );
        }
        Ok(json!(records))
    }
}

#[derive(Serialize)]
struct Target {
    path: String,
    before: Option<Object>,
    after_sha256: String,
    write: bool,
    original: String,
    previous_original: Option<String>,
    staging: String,
    #[serde(skip)]
    bytes: Vec<u8>,
    #[serde(skip)]
    previous_bytes: Option<Vec<u8>>,
}
#[derive(Default, Serialize)]
pub(super) struct Plan {
    targets: Vec<Target>,
}
impl Plan {
    pub fn workflow(
        &mut self,
        root: &Path,
        path: &str,
        expected: String,
        marker: &str,
        verify: bool,
        drift: &str,
    ) -> Result<(), String> {
        if path.starts_with(".github/workflows/") && expected.len() > 500_000 {
            return Err(format!(
                "GitHub workflow exceeds 500 KB: {path} ({} bytes); split the registered workflow",
                expected.len()
            ));
        }
        let physical = no_symlink_parents(root, path)?;
        let (before, bytes) = observe(&physical)?;
        if let Some(bytes) = &bytes {
            let text = std::str::from_utf8(bytes).map_err(|_| "owned output is not UTF-8")?;
            if !text.starts_with(marker) {
                return Err(format!("unowned workflow collision: {path}"));
            }
        }
        self.target(
            path,
            before,
            bytes.as_deref() == Some(expected.as_bytes()),
            expected.into_bytes(),
            bytes,
            verify,
            drift,
        )
    }
    pub fn inventory(
        &mut self,
        root: &Path,
        path: &str,
        expected: Vec<u8>,
        adopting: bool,
        verify: bool,
    ) -> Result<(), String> {
        let (before, bytes) = observe(&no_symlink_parents(root, path)?)?;
        if adopting && bytes.as_ref().is_some_and(|bytes| bytes != &expected) {
            return Err(format!("initial inventory adoption collision: {path}"));
        }
        self.target(
            path,
            before,
            bytes.as_ref() == Some(&expected),
            expected,
            bytes,
            verify,
            "generated initial inventory drift",
        )
    }
    pub fn adopt(&mut self, path: &str, bytes: Vec<u8>) -> Result<(), String> {
        self.target(path, None, false, bytes, None, false, "")
    }
    fn target(
        &mut self,
        path: &str,
        before: Option<Object>,
        same: bool,
        bytes: Vec<u8>,
        previous_bytes: Option<Vec<u8>>,
        verify: bool,
        drift: &str,
    ) -> Result<(), String> {
        relative_path(path)?;
        if self
            .targets
            .iter()
            .any(|target| super::overlap(&target.path, path))
        {
            return Err(format!("overlapping generation outputs: {path}"));
        }
        if verify && !same {
            return Err(format!("{drift}: {path}"));
        }
        self.targets.push(Target {
            path: path.into(),
            before,
            after_sha256: sha256(&bytes),
            write: !same,
            original: format!("output-{}.bytes", self.targets.len()),
            previous_original: previous_bytes
                .as_ref()
                .map(|_| format!("previous-{}.bytes", self.targets.len())),
            staging: format!("pending-{}.bytes", self.targets.len()),
            bytes,
            previous_bytes,
        });
        Ok(())
    }
    fn retain(&self, publication: &Publication, directory: &Path) -> Result<String, String> {
        for target in &self.targets {
            publication.write_file(&directory.join(&target.original), &target.bytes)?;
            if let (Some(name), Some(bytes)) = (&target.previous_original, &target.previous_bytes) {
                publication.write_file(&directory.join(name), bytes)?;
            }
        }
        let plan = json!({"schema":"chrono-ci-generation-plan/v1","targets":self.targets});
        record(publication, directory, "plan.json", &plan)?;
        digest(&plan)
    }
    fn install(
        &self,
        root: &Path,
        publication: &Publication,
        directory: &Path,
    ) -> Result<bool, String> {
        let (_, result) = verified::install_targets((), self.targets.len(), |state, index| {
            (
                state,
                (|| {
                    let target = &self.targets[index];
                    let path = no_symlink_parents(root, &target.path)?;
                    let io = verified::TargetIo {
                        observe: |state| (state, observe(&path).map(|(object, _)| object)),
                        stage: |state| {
                            // The original plan survives any interruption between a rename and its receipt.
                            verified::stage_target(
                                state,
                                directory.join(&target.staging),
                                |state, stage: &PathBuf| {
                                    (state, publication.write_file(stage, &target.bytes))
                                },
                                |state, stage: &PathBuf| (state, identity(stage)),
                                |state, _stage: &PathBuf, identity: String| {
                                    (
                                        state,
                                        record(
                                            publication,
                                            directory,
                                            &format!("staging-{index}.json"),
                                            &json!({"path":target.staging,"identity":identity,"target":target.path}),
                                        ),
                                    )
                                },
                                |state| {
                                    (
                                        state,
                                        path.parent()
                                            .ok_or_else(|| "generation output parent".to_owned()),
                                    )
                                },
                                |state, parent: &&Path| {
                                    (
                                        state,
                                        fs::create_dir_all(*parent).map_err(|e| e.to_string()),
                                    )
                                },
                                |state, parent: &&Path| (state, sync_ancestors(parent, root)),
                            )
                        },
                        create: |state, handles: &(PathBuf, &Path)| {
                            (
                                state,
                                chrono_harness::artifact_disposal::publish_entry(&handles.0, &path),
                            )
                        },
                        replace: |state, handles: &(PathBuf, &Path)| {
                            (
                                state,
                                fs::rename(&handles.0, &path).map_err(|e| e.to_string()),
                            )
                        },
                        sync: |state, handles: (PathBuf, &Path)| {
                            (state, sync(handles.1).and_then(|()| sync(directory)))
                        },
                        effect: |state, object: Option<Object>| {
                            (
                                state,
                                (|| {
                                    record(
                                        publication,
                                        directory,
                                        &format!("effect-{index}.json"),
                                        &json!({"path":target.path,"object":object,"updated":timestamp()?}),
                                    )?;
                                    checkpoint("effect");
                                    Ok(())
                                })(),
                            )
                        },
                        mismatch: |state, reason: verified::TargetMismatch| {
                            (state, format!("{}: {}", reason.label(), target.path))
                        },
                    };
                    verified::install_target(
                        (),
                        target.before.as_ref(),
                        target.write,
                        &target.after_sha256,
                        &io,
                    )
                    .1
                })(),
            )
        });
        result
    }
}

fn timestamp() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| e.to_string())
}
fn sync(path: &Path) -> Result<(), String> {
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| format!("sync {}: {e}", path.display()))
}
fn sync_ancestors(start: &Path, root: &Path) -> Result<(), String> {
    verified::sync_ancestor_directories(
        start.ancestors(),
        root,
        |mut ancestors: std::path::Ancestors<'_>| {
            let path = ancestors.next();
            (ancestors, path)
        },
        |path: &&Path, root: &&Path| *path == *root,
        |state, path: &Path| (state, sync(path)),
    )
    .1
}
fn record(
    publication: &Publication,
    directory: &Path,
    name: &str,
    value: &Value,
) -> Result<(), String> {
    let stage = unique_path(directory, &format!(".{name}-"), ".pending")?;
    publication.write_file(
        &stage,
        &serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )?;
    chrono_harness::artifact_disposal::publish_entry(&stage, &directory.join(name))?;
    sync(directory)
}
fn digest(value: &Value) -> Result<String, String> {
    Ok(sha256(
        &serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    ))
}
fn inventory(root: &Path) -> Result<Inventory, String> {
    let inventory = Inventory::with_standalone_policy(root, POLICY)?;
    if inventory.terminal_limits(PRODUCER).is_none() {
        return Err("ci-generation-operation requires explicit finite terminal_limits in the adopted retention policy".into());
    }
    Ok(inventory)
}

pub(super) fn run(
    root: &Path,
    operation: &str,
    config: &str,
    prepare: impl FnOnce(&Path, &mut Inputs) -> Result<Plan, String>,
) -> Result<bool, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let state = inventory(&root)?;
    let store = no_symlink_parents(&root, STORE)?;
    fs::create_dir_all(&store).map_err(|e| e.to_string())?;
    sync_ancestors(&store, &root)?;
    let directory = unique_path(&store, "operation-", "")?;
    let attempt = directory
        .strip_prefix(&root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("operation path UTF8")?
        .to_owned();
    let role = format!("ci-generation-recovery:{attempt}");
    let intent = json!({"schema":"chrono-ci-generation-intent/v1","producer":PRODUCER,
        "operation":operation,"config":config,"attempt":attempt,"host_identity":identity(&root)?,
        "started":timestamp()?,"recovery_role":role,"task_handle":null,
        "result_method":"generation-status --operation-path; never repeat unknown work",
        "original_process_exit":null});
    let publication = state.plan_rooted(PRODUCER, &attempt, intent.clone(), &role)?;
    eprintln!("CI generation operation: {attempt}");
    let evaluated = (|| {
        publication.materialize()?;
        sync(&store)?;
        record(&publication, &directory, "intent.json", &intent)?;
        checkpoint("intent");
        let mut inputs = Inputs {
            root: root.clone(),
            originals: BTreeMap::new(),
        };
        let prepared = prepare(&root, &mut inputs);
        let originals = inputs.retain(&publication, &directory)?;
        record(&publication, &directory, "inputs.json", &originals)?;
        let mut plan_digest = None;
        let (_, outcome) = verified::prepare_then_apply(
            (),
            |state| {
                (
                    state,
                    (|| {
                        let plan = prepared?;
                        plan_digest = Some(plan.retain(&publication, &directory)?);
                        if intent["host_identity"] != identity(&root)? {
                            return Err("generation host identity changed".into());
                        }
                        inputs.stable()?;
                        checkpoint("prepared");
                        Ok(plan)
                    })(),
                )
            },
            |state, plan: Plan| (state, plan.install(&root, &publication, &directory)),
        );
        let evaluated = outcome.result;
        let effects = outcome.effects;
        let result = json!({"schema":"chrono-ci-generation-result/v1","attempt":attempt,
            "intent_sha256":sha256(&serde_json::to_vec_pretty(&intent).map_err(|e| e.to_string())?),
            "updated":timestamp()?,"original_process_exit":null,"effects":effects.label(),
            "inputs_sha256":digest(&originals)?,"plan_sha256":plan_digest,
            "producer_evaluation":match &evaluated { Ok(changed)=>json!({"changed":changed}), Err(error)=>json!({"error":error}) }});
        // The evaluation has returned. Bind its exact result before attempting
        // to publish it, so recovery never needs to repeat projection effects.
        let io = verified::ReturnIo {
            observe: |state, evaluated: &Result<bool, String>| {
                (
                    state,
                    (|| {
                        inventory(&root)?.observe_producer_return(PRODUCER, &attempt,
                            json!({"method":"generation-evaluation","result_record":result,"process_exit":null}))
                            .map_err(|error| format!("{evaluated:?}; generation return observation failed: {error}; attempt {attempt}"))
                    })(),
                )
            },
            publish: |state, publication: &Publication, evaluated: &Result<bool, String>| {
                (
                    state,
                    record(publication, &directory, "result.json", &result).map_err(|error| {
                        format!("{evaluated:?}; generation result publication failed: {error}; attempt {attempt}")
                    }),
                )
            },
            seal: |state, publication: Publication, evaluated: &Result<bool, String>| {
                (
                    state,
                    publication.complete_sealed(json!({"intent":intent,"result":result}))
                        .map_err(|error| {
                            format!("{evaluated:?}; generation sealing failed: {error}; attempt {attempt}")
                        }),
                )
            },
            release: |state, evaluated: &Result<bool, String>| {
                (
                    state,
                    (|| {
                        inventory(&root)?.reference_paths(&role, &[]).map_err(|error| {
                            format!("{evaluated:?}; generation recovery release failed: {error}; attempt {attempt}")
                        })
                    })(),
                )
            },
            notify: |state, point: verified::ReturnCheckpoint| {
                checkpoint(point.label());
                state
            },
        };
        verified::finish_generation_return((), publication, evaluated, &effects, &io).1
    })();
    evaluated
}

pub fn status(root: &Path, path: &str) -> Result<Value, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let inventory = inventory(&root)?;
    let mut report = inventory.producer_status(PRODUCER, Some(path))?;
    report["completion"] = json!("unknown");
    let records = report["records"]
        .as_array()
        .ok_or("generation status records")?;
    if verified::sealed_record(
        records.len(),
        records.first().and_then(|record| record["phase"].as_str()),
        records
            .first()
            .and_then(|record| record["sealed"].as_bool()),
    ) {
        let _guard = inventory
            .read_guard(path)?
            .ok_or("generation original unavailable")?;
        let expected = records[0]["observed_return"]["result_record"].clone();
        let (intent, result) = originals(&root, path, &expected)?;
        for (name, bytes) in [("intent.json", intent), ("result.json", result)] {
            if !inventory.published_tree_member(
                PRODUCER,
                path,
                &format!("{path}/{name}"),
                &sha256(&bytes),
                bytes.len() as u64,
            )? {
                return Err(format!("generation {name} is not the sealed original"));
            }
        }
        report["completion"] = json!(
            if verified::reported_unknown(expected["effects"].as_str()) {
                "evaluation-known-effects-unknown"
            } else {
                "confirmed-evaluation"
            }
        );
        report["original_result"] = expected;
    }
    Ok(report)
}
pub fn maintain(root: &Path) -> Result<Value, String> {
    inventory(&fs::canonicalize(root).map_err(|e| e.to_string())?)?.maintain_producer(PRODUCER)
}

fn originals(root: &Path, path: &str, expected: &Value) -> Result<(Vec<u8>, Vec<u8>), String> {
    let directory = no_symlink_parents(root, path)?;
    let intent = observe(&directory.join("intent.json"))?
        .1
        .ok_or("generation intent unavailable")?;
    let result = observe(&directory.join("result.json"))?
        .1
        .ok_or("generation result unavailable")?;
    let intent_value: Value = decode(&intent)?;
    let result_value: Value = decode(&result)?;
    let fields = verified::OriginalBinding {
        intent_schema: intent_value["schema"].as_str(),
        intent_attempt: intent_value["attempt"].as_str(),
        intent_host: intent_value["host_identity"].as_str(),
        result_schema: result_value["schema"].as_str(),
        result_attempt: result_value["attempt"].as_str(),
        result_intent_sha256: result_value["intent_sha256"].as_str(),
        result_exit_is_null: result_value["original_process_exit"].is_null(),
    };
    verified::bind_generation_originals(
        (),
        &fields,
        path,
        &intent,
        |state| (state, identity(root)),
        |state| (state, result_value == *expected),
        |state, bytes| (state, sha256(bytes)),
    )
    .1?;
    let read_bound = |name: &str, expected: &str| -> Result<Vec<u8>, String> {
        let io = verified::BoundOriginalIo {
            validate: |state| (state, relative_path(name)),
            observe: |state| {
                (
                    state,
                    no_symlink_parents(root, &format!("{path}/{name}"))
                        .and_then(|path| observe(&path))
                        .map(|(_, bytes)| bytes),
                )
            },
            missing: |state| (state, format!("generation original unavailable: {name}")),
            hash: |state, bytes| (state, sha256(bytes)),
            changed: |state| (state, format!("generation original bytes changed: {name}")),
        };
        verified::read_bound_original((), expected, &io).1
    };
    let inputs: Value = decode(&read_bound(
        "inputs.json",
        expected["inputs_sha256"]
            .as_str()
            .ok_or("generation inputs binding missing")?,
    )?)?;
    for input in inputs
        .as_array()
        .ok_or("generation input originals malformed")?
    {
        if let Some(name) = input["original"].as_str() {
            let bytes = read_bound(
                name,
                input["object"]["sha256"]
                    .as_str()
                    .ok_or("generation input digest missing")?,
            )?;
            if Some(bytes.len() as u64) != input["object"]["bytes"].as_u64() {
                return Err("generation input original length changed".into());
            }
        }
    }
    if let Some(hash) = expected["plan_sha256"].as_str() {
        let plan: Value = decode(&read_bound("plan.json", hash)?)?;
        if plan["schema"] != "chrono-ci-generation-plan/v1" {
            return Err("generation plan schema mismatch".into());
        }
        for target in plan["targets"]
            .as_array()
            .ok_or("generation target originals malformed")?
        {
            read_bound(
                target["original"]
                    .as_str()
                    .ok_or("generation output original missing")?,
                target["after_sha256"]
                    .as_str()
                    .ok_or("generation output digest missing")?,
            )?;
            if let Some(name) = target["previous_original"].as_str() {
                read_bound(
                    name,
                    target["before"]["sha256"]
                        .as_str()
                        .ok_or("generation prior output digest missing")?,
                )?;
            }
        }
    }
    Ok((intent, result))
}

/// Finish only the original result publication. Projection work is never replayed.
pub fn resume(root: &Path, path: &str) -> Result<Value, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let state = inventory(&root)?;
    let report = state.producer_status(PRODUCER, Some(path))?;
    let records = report["records"]
        .as_array()
        .ok_or("generation status records")?;
    let [original] = records.as_slice() else {
        return Err("generation original unavailable or retired; no work was dispatched".into());
    };
    let role = format!("ci-generation-recovery:{path}");
    let expected = original["observed_return"]["result_record"].clone();
    if expected.is_null() {
        let mut unknown = report;
        unknown["completion"] = json!("unknown");
        return Ok(unknown);
    }
    let attached = state.rooted_original(PRODUCER, &role)?;
    if verified::sealed_record(
        records.len(),
        original["phase"].as_str(),
        original["sealed"].as_bool(),
    ) {
        let verified = status(&root, path)?;
        if attached.is_some() && !verified::reported_unknown(expected["effects"].as_str()) {
            state.reference_paths(&role, &[])?;
        }
        return Ok(verified);
    }
    if !verified::recovery_reference_matches(
        original["id"].as_str(),
        attached.as_ref().map(|(id, _, _, _)| id.as_str()),
        attached.as_ref().map(|(_, actual, _, _)| actual.as_str()),
        path,
    ) {
        return Err("generation recovery reference does not match the original".into());
    }
    let publication = inventory(&root)?
        .resume_unfinished_directory(PRODUCER, path)?
        .ok_or("generation recovery publication unavailable")?;
    let directory = no_symlink_parents(&root, path)?;
    // This record is reconstructed from the producer's already durable return,
    // never from current workflow contents or an enclosing process exit.
    let io = verified::ResumeIo {
        probe: |state| {
            (
                state,
                directory
                    .join("result.json")
                    .try_exists()
                    .map_err(|e| e.to_string()),
            )
        },
        write: |state, publication: &Publication, expected: &Value| {
            (
                state,
                record(publication, &directory, "result.json", expected),
            )
        },
        check: |state, expected: &Value| (state, originals(&root, path, expected).map(|_| ())),
        seal: |state, publication: Publication, expected: &Value| {
            (
                state,
                publication.complete_sealed(
                    json!({"result":expected,"recovered":"result-publication-only"}),
                ),
            )
        },
        release: |state| {
            (
                state,
                inventory(&root).and_then(|inventory| inventory.reference_paths(&role, &[])),
            )
        },
        status: |state| (state, status(&root, path)),
    };
    verified::resume_generation_result(
        (),
        publication,
        &expected,
        expected["effects"].as_str(),
        &io,
    )
    .1
}

pub(super) fn dispatch(args: &[String]) -> Result<String, String> {
    let mut opts = BTreeMap::new();
    for pair in args[1..].chunks(2) {
        if pair.len() != 2
            || !["--host-root", "--operation-path"].contains(&pair[0].as_str())
            || opts.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err(
                "generation state requires unique --host-root and optional --operation-path".into(),
            );
        }
    }
    let root = Path::new(*opts.get("--host-root").ok_or("--host-root required")?);
    let report = match args[0].as_str() {
        "generation-status" if opts.len() == 2 => status(
            root,
            opts.get("--operation-path")
                .ok_or("--operation-path required")?,
        )?,
        "generation-resume" if opts.len() == 2 => resume(
            root,
            opts.get("--operation-path")
                .ok_or("--operation-path required")?,
        )?,
        "generation-maintain" if opts.len() == 1 => maintain(root)?,
        _ => return Err("unexpected generation state options".into()),
    };
    Ok(serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n")
}
