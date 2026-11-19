//! Execution originals belong to projects; retention only protects their lifetime.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Admission for a declared compiler destination inside the shared run directory.
pub fn owned_compiler_output(exact_name: bool, single_component: bool, canonical: bool) -> (owned: bool)
    ensures owned == (exact_name && single_component && canonical),
{
    exact_name && single_component && canonical
}
}

#[cfg(not(verus_keep_ghost))]
use crate::verified_state::{self, OperationObservation};
#[cfg(not(verus_keep_ghost))]
use crate::{OperationResult, Results};
#[cfg(not(verus_keep_ghost))]
use chrono_harness::retained_artifacts::retention::{self, Inventory, Publication};
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{ProcessEvidence, no_symlink_parents, sha256};
#[cfg(not(verus_keep_ghost))]
use chrono_judge_routes::Execution;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{fs, path::PathBuf, time::SystemTime};

#[cfg(not(verus_keep_ghost))]
struct Original {
    root: PathBuf,
    path: String,
    publication: Option<Publication>,
}

#[cfg(not(verus_keep_ghost))]
fn now() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .map_err(|e| e.to_string())
}

#[cfg(not(verus_keep_ghost))]
impl Original {
    fn recovery(&self) -> String {
        format!("projects-recovery:{}", sha256(self.path.as_bytes()))
    }

    fn begin(plan: &Execution, path: String, intent: Value) -> Result<Self, String> {
        chrono_harness::units::artifact_path(&path)?;
        let absolute = no_symlink_parents(&plan.root, &path)?;
        fs::create_dir_all(absolute.parent().ok_or("operation state parent missing")?)
            .map_err(|e| e.to_string())?;
        no_symlink_parents(&plan.root, &path)?;
        let mut original = Self {
            root: plan.root.clone(),
            path,
            publication: None,
        };
        if let Some(inventory) = Inventory::adopted(&plan.root)? {
            if !inventory.accepts("check-original", &original.path) {
                return Err(
                    "operation originals require the registered check-original retention scope"
                        .into(),
                );
            }
            let publication = inventory.plan(
                "check-original",
                &original.path,
                true,
                json!({"kind":"project-execution", "plan":plan.identity, "state":"unknown"}),
            )?;
            publication.materialize_for_reference(&original.recovery())?;
            original.publication = Some(publication);
        } else {
            fs::create_dir(&absolute).map_err(|e| {
                format!(
                    "operation state {}: {e}; preserve originals, do not redispatch",
                    original.path
                )
            })?;
        }
        original.write("intent.json", &intent)?;
        Ok(original)
    }

    fn write(&self, name: &str, value: &Value) -> Result<Value, String> {
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        let path = format!("{}/{name}", self.path);
        retention::write_adopted(self.publication.as_ref(), &self.root.join(&path), &bytes)?;
        fs::File::open(self.root.join(&self.path))
            .and_then(|directory| directory.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(json!({"path":path,"sha256":sha256(&bytes),"length":bytes.len()}))
    }

    fn finish(mut self, terminal: bool, outcome: Value, inputs: &[String]) -> Result<(), String> {
        let inventory = Inventory::adopted(&self.root)?;
        if let Some(inventory) = &inventory {
            inventory.dependencies_paths(&self.path, inputs)?;
        }
        if let Some(publication) = self.publication.take() {
            publication.complete_sealed(outcome)?;
        }
        if let Some(inventory) = inventory {
            inventory.retain_diagnostic_path(&self.path)?;
            if terminal {
                inventory.reference_paths(&self.recovery(), &[])?;
            }
        }
        Ok(())
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) struct Run {
    original: Original,
    pub(crate) plan_original: Value,
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn address(plan: &Execution) -> Result<&str, String> {
    plan.binding["operation_state"]
        .as_str()
        .ok_or("E_OPERATION_STATE: missing registered operation state address".into())
}

#[cfg(not(verus_keep_ghost))]
fn observation(row: &OperationResult) -> OperationObservation<'_> {
    OperationObservation {
        status: &row.status,
        process_terminal: row.receipt.as_ref().map(|receipt| {
            chrono_harness::verified_release::process_completion_known(
                receipt.process.exit_code,
                receipt.process.exit_signal,
                receipt.process.owned_work_joined,
            )
        }),
    }
}

#[cfg(not(verus_keep_ghost))]
fn encoded(row: &OperationResult, retained_process: Option<Value>) -> Result<Value, String> {
    let mut value = serde_json::to_value(row).map_err(|e| e.to_string())?;
    if row.receipt.is_some() {
        value["receipt"]["process"] = match retained_process {
            Some(process) => process,
            None => chrono_harness::full::compact_process(&value["receipt"]["process"])?,
        };
    }
    value["schema"] = json!("chrono-project-operation-result/v1");
    value["updated_at_unix_millis"] = json!(now()?);
    value["state"] = json!(verified_state::operation_state(&observation(row)));
    Ok(value)
}

#[cfg(not(verus_keep_ghost))]
impl Run {
    pub(crate) fn begin(plan: &Execution) -> Result<Self, String> {
        let path = address(plan)?.to_owned();
        let original = Original::begin(
            plan,
            path,
            json!({
                "schema":"chrono-project-execution/v1", "producer":"chrono-judge-projects",
                "plan":plan.identity, "state":"intent", "updated_at_unix_millis":now()?,
                "handle":null, "result":null,
                "recovery":"read this execution and its independent operation originals; absent results remain unknown; never redispatch the same identity"
            }),
        )?;
        let plan_original = original.write(
            "plan.json",
            &serde_json::to_value(plan).map_err(|e| e.to_string())?,
        )?;
        Ok(Self {
            original,
            plan_original,
        })
    }

    pub(crate) fn finish(self, results: &mut Results) -> Result<(), String> {
        let mut originals = Vec::new();
        let mut inputs = Vec::new();
        for (index, row) in results
            .executed
            .iter_mut()
            .chain(&mut results.blocked)
            .enumerate()
        {
            let reference = match &row.original {
                Some(original) => original.clone(),
                None => self
                    .original
                    .write(&format!("undelivered-{index}.json"), &encoded(row, None)?)?,
            };
            if row.original.is_some() {
                inputs.push(
                    reference["path"]
                        .as_str()
                        .ok_or("operation original path")?
                        .to_owned(),
                );
            }
            row.original = Some(reference.clone());
            originals.push(reference);
        }
        let observations = results
            .executed
            .iter()
            .chain(&results.blocked)
            .map(observation)
            .collect();
        let terminal = verified_state::all_terminal(&observations);
        let reference = self.original.write(
            "result.json",
            &json!({
                "schema":"chrono-project-execution-result/v1", "producer":"chrono-judge-projects",
                "plan":results.plan, "state":"returned", "terminal":terminal,
                "updated_at_unix_millis":now()?, "plan_original":self.plan_original,
                "operations":originals, "tests":results.tests,
            }),
        )?;
        self.original.finish(
            terminal,
            json!({"kind":"project-execution","state":"returned","terminal":terminal}),
            &inputs,
        )?;
        results.operation_state = Some(reference);
        Ok(())
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) struct Operation {
    original: Original,
    streams: [PathBuf; 3],
    compiler_output: Option<String>,
}

#[cfg(not(verus_keep_ghost))]
impl Operation {
    pub(crate) fn begin(
        plan: &Execution,
        index: usize,
        plan_original: &Value,
    ) -> Result<Self, String> {
        let run = address(plan)?;
        let path = format!("{run}.operation-{index}");
        let streams =
            ["stdout", "stderr", "launch.json"].map(|suffix| plan.root.join(&path).join(suffix));
        let op = &plan.operations[index];
        let mut compiler_output = None;
        if op.method.output_contract.is_some() {
            if let Some(output) = op
                .method
                .argv
                .get(10)
                .filter(|output| std::path::Path::new(output).starts_with(run))
            {
                let expected = format!("{run}/{}.rlib", op.method.operation);
                chrono_harness::units::artifact_path(&expected)?;
                if !owned_compiler_output(
                    output == &expected,
                    !op.method.operation.contains('/'),
                    chrono_harness::verified_verifier::canonical_verifier_arguments(
                        &op.method.argv,
                    ),
                ) {
                    return Err(
                        "compiler output in the run directory must belong to this operation".into(),
                    );
                }
                compiler_output = Some(expected);
            }
        }
        let original = Original::begin(
            plan,
            path,
            json!({
                "schema":"chrono-project-operation/v1", "producer":"chrono-judge-projects",
                "plan":plan.identity, "plan_original":plan_original, "operation":op,
                "state":"intent", "updated_at_unix_millis":now()?, "handle":null, "result":null,
                "process_originals":streams.iter().map(|p| p.strip_prefix(&plan.root).unwrap()).collect::<Vec<_>>(),
                "recovery":"the native runner owns the only wait; inspect launch and partial streams; missing result remains unknown"
            }),
        )?;
        Ok(Self {
            original,
            streams,
            compiler_output,
        })
    }

    pub(crate) fn evidence(&self) -> ProcessEvidence<'_> {
        ProcessEvidence {
            root: &self.original.root,
            stdout: &self.streams[0],
            stderr: &self.streams[1],
            launch: &self.streams[2],
            publication: self.original.publication.as_ref(),
        }
    }

    /// The run owns this directory until all its operations have joined. Its durable receipt contains
    /// everything later consumers read. Failed or interrupted runs keep their evidence.
    fn retire_compiler_files(&self, row: &OperationResult) -> Result<(), String> {
        let Some(receipt) = &row.receipt else {
            return Ok(());
        };
        let (Some(Ok(library)), Some(Ok(dependency))) =
            (&receipt.compiler_artifact, &receipt.compiler_dependency)
        else {
            return Ok(());
        };
        let Some(output) = self.compiler_output.as_deref() else {
            return Ok(());
        };
        if !verified_state::can_release_compiler_files(
            &row.status,
            receipt.process.owned_work_joined,
            library.path == output.as_bytes(),
        ) {
            return Ok(());
        }
        let root = &self.original.root;
        let library_digest = std::str::from_utf8(&library.sha256).map_err(|e| e.to_string())?;
        let dependency_path = format!("{output}.d");
        let dependency_digest = sha256(dependency);
        let mut entries = Vec::new();
        for (relative, digest, length) in [
            (output, library_digest, library.length),
            (
                dependency_path.as_str(),
                dependency_digest.as_str(),
                dependency.len() as u64,
            ),
        ] {
            let path = no_symlink_parents(root, relative)?;
            let identity = chrono_harness::artifact_disposal::identity(&path)?;
            let actual = chrono_harness::file_identity(&path)?;
            verified_state::validate_original(&actual.0, actual.1, Some((digest, length)))?;
            entries.push((relative, identity));
        }
        if self.original.publication.is_none() {
            fs::File::open(root.join(&self.original.path).join("result.json"))
                .and_then(|file| file.sync_all())
                .map_err(|e| e.to_string())?;
        }
        let parent = std::path::Path::new(output)
            .parent()
            .and_then(|p| p.to_str())
            .ok_or("compiler output parent")?;
        chrono_harness::artifact_disposal::source_safe(root, parent)?;
        for (relative, identity) in entries {
            chrono_harness::artifact_disposal::dispose_entry(root, relative, &identity, || {
                if let Some(publication) = &self.original.publication {
                    publication.check_length(0)?;
                }
                Ok(())
            })?;
        }
        fs::File::open(no_symlink_parents(root, parent)?)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| e.to_string())
    }

    pub(crate) fn finish(self, row: &OperationResult) -> Result<Value, String> {
        let retained_process = if let Some(receipt) = &row.receipt {
            let paths = self
                .streams
                .iter()
                .map(|path| {
                    path.strip_prefix(&self.original.root)
                        .map_err(|e| e.to_string())?
                        .to_str()
                        .ok_or("operation original UTF-8".into())
                })
                .collect::<Result<Vec<_>, String>>()?;
            let process = chrono_harness::scoped_report::retain_process_evidence(
                &self.original.root,
                &receipt.process,
                paths[0],
                paths[1],
                paths[2],
            )?;
            for (index, (stream, expected_digest, expected_length)) in [
                (
                    "stdout",
                    &receipt.process.stdout_sha256,
                    receipt.process.stdout_bytes.len() as u64,
                ),
                (
                    "stderr",
                    &receipt.process.stderr_sha256,
                    receipt.process.stderr_bytes.len() as u64,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let digest = process[format!("{stream}_original")]["sha256"]
                    .as_str()
                    .ok_or("operation stream original digest")?;
                let length = fs::symlink_metadata(&self.streams[index])
                    .map_err(|e| e.to_string())?
                    .len();
                verified_state::validate_original(
                    digest,
                    length,
                    Some((expected_digest, expected_length)),
                )?;
            }
            Some(process)
        } else {
            None
        };
        let reference = self
            .original
            .write("result.json", &encoded(row, retained_process)?)?;
        self.retire_compiler_files(row)?;
        // The operation directory owns both its result and the exact streams it
        // references. Unknown results keep that directory's recovery protection.
        let known = verified_state::terminal(&observation(row));
        self.original.finish(
            known,
            json!({"kind":"project-operation", "operation":row.operation, "terminal":known}),
            &[],
        )?;
        Ok(reference)
    }
}
