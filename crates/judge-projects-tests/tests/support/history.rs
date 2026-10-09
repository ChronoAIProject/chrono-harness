//! Acquisition of one explicitly pinned historical test tree, before fixture use.
use chrono_harness::{CommandSpec, ProcessResult, facts, run_process_observed};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub schema: String,
    pub commit: String,
    pub tree: String,
    pub remote: String,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}

pub struct Snapshot {
    pub tree: facts::Tree,
    pub blobs: BTreeMap<String, Vec<u8>>,
    pub processes: Vec<ProcessResult>,
}

#[derive(Debug)]
pub struct Failure {
    pub message: String,
    pub processes: Vec<ProcessResult>,
}

pub fn acquire(
    root: &Path,
    binding: &Binding,
    mut spec: CommandSpec,
    digest: &str,
) -> Result<Snapshot, Failure> {
    let mut processes = vec![];
    let result = (|| -> Result<_, String> {
        if binding.schema != "chrono-test-history/v1"
            || binding.remote.is_empty()
            || binding.remote.starts_with('-')
            || binding.remote.contains(['\0', '\n', '\r'])
            || binding.timeout_seconds == 0
            || binding.output_limit_bytes == 0
            || binding.output_limit_bytes > 64 * 1024 * 1024
        {
            return Err("invalid historical test input binding".into());
        }
        facts::full_oid(&binding.commit)?;
        facts::full_oid(&binding.tree)?;
        spec.timeout_seconds = binding.timeout_seconds;
        spec.output_limit_bytes = binding.output_limit_bytes;
        spec.env.insert("GIT_NO_LAZY_FETCH".into(), "1".into());
        spec.env.insert("GIT_TERMINAL_PROMPT".into(), "0".into());
        for key in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
            spec.env.remove(key);
        }
        let mut git = |args: &[String], stdin: &[u8]| -> Result<(usize, Vec<u8>), String> {
            spec.args = ["--no-optional-locks", "--no-replace-objects"]
                .into_iter()
                .map(str::to_owned)
                .chain(args.iter().cloned())
                .collect();
            let process = run_process_observed(root, &spec, stdin, digest)?;
            let succeeded = process.exit_code == 0 && process.failure.is_none();
            let bytes = process.stdout_bytes.clone();
            let index = processes.len();
            processes.push(process);
            if !succeeded {
                return Err(format!("historical input Git failed: {args:?}"));
            }
            Ok((index, bytes))
        };
        let actual_tree = git(
            &["rev-parse".into(), format!("{}^{{tree}}", binding.commit)],
            &[],
        )?
        .1;
        if actual_tree != format!("{}\n", binding.tree).as_bytes() {
            return Err("historical test commit/tree identity mismatch".into());
        }
        let tree = facts::parse_tree(
            &git(
                &["ls-tree".into(), "-rz".into(), binding.commit.clone()],
                &[],
            )?
            .1,
        )?;
        let objects: Vec<_> = tree
            .values()
            .filter(|entry| entry.kind == "blob")
            .map(|entry| entry.oid.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        // The declared tree is the complete input, not an inferred test graph.
        // Bound one fetch's argv; never walk ancestry or discover another ref.
        if objects.iter().map(|oid| oid.len() + 1).sum::<usize>() > 64 * 1024 {
            return Err("historical test tree exceeds fixture acquisition argv bound".into());
        }
        let query = objects
            .iter()
            .map(|oid| format!("{oid}\n"))
            .collect::<String>();
        let probe = [
            "cat-file".into(),
            "--batch-check=%(objectname) %(objecttype)".into(),
        ];
        let missing = missing_blobs(&objects, &git(&probe, query.as_bytes())?.1)?;
        if !missing.is_empty() {
            let args = [
                "fetch",
                "--no-tags",
                "--no-write-fetch-head",
                "--no-auto-maintenance",
                "--no-filter",
                binding.remote.as_str(),
            ]
            .into_iter()
            .map(str::to_owned)
            .chain(missing)
            .collect::<Vec<_>>();
            git(&args, &[])?;
            if !missing_blobs(&objects, &git(&probe, query.as_bytes())?.1)?.is_empty() {
                return Err("historical test blobs remain missing after explicit fetch".into());
            }
        }
        let paths = tree
            .iter()
            .filter(|(_, entry)| entry.kind == "blob")
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        let acquired = facts::acquire_registry_blobs(
            &binding.commit,
            &paths,
            binding.output_limit_bytes,
            |args, stdin| {
                git(
                    &args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>(),
                    stdin,
                )
            },
            str::to_owned,
        )?;
        let blobs = acquired
            .into_iter()
            .map(|blob| (tree[&blob.path].oid.clone(), blob.bytes))
            .collect();
        Ok((tree, blobs))
    })();
    match result {
        Ok((tree, blobs)) => Ok(Snapshot {
            tree,
            blobs,
            processes,
        }),
        Err(message) => Err(Failure { message, processes }),
    }
}

fn missing_blobs(objects: &[String], bytes: &[u8]) -> Result<Vec<String>, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "non UTF-8 historical blob probe")?;
    let rows = text.split_terminator('\n').collect::<Vec<_>>();
    if rows.len() != objects.len() || (!objects.is_empty() && !text.ends_with('\n')) {
        return Err("historical blob probe count/terminator mismatch".into());
    }
    let mut missing = vec![];
    for (oid, row) in objects.iter().zip(rows) {
        if row == format!("{oid} missing") {
            missing.push(oid.clone());
        } else if row != format!("{oid} blob") {
            return Err("expected exact historical blob or missing probe".into());
        }
    }
    Ok(missing)
}

impl Snapshot {
    /// Fixture consumption uses already acquired bytes and cannot fetch.
    pub fn export(&self, dest: &Path) -> Result<(), String> {
        for (path, entry) in &self.tree {
            if entry.kind != "blob" {
                continue;
            }
            let target = chrono_harness::no_symlink_parents(dest, path)?;
            fs::create_dir_all(target.parent().ok_or("historical fixture parent")?)
                .map_err(|e| e.to_string())?;
            let bytes = self
                .blobs
                .get(&entry.oid)
                .ok_or("unacquired historical blob")?;
            if entry.mode == "120000" {
                #[cfg(unix)]
                std::os::unix::fs::symlink(facts::utf8(bytes.clone())?, &target)
                    .map_err(|e| e.to_string())?;
                #[cfg(not(unix))]
                return Err("historical symlink fixture needs Unix".into());
            } else {
                fs::write(&target, bytes).map_err(|e| e.to_string())?;
                let mut permissions = fs::metadata(&target)
                    .map_err(|e| e.to_string())?
                    .permissions();
                permissions.set_readonly(true);
                fs::set_permissions(&target, permissions).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

#[test]
fn blob_probe_requires_exact_identity_type_count_and_terminator() {
    let oid = "a".repeat(40);
    let objects = vec![oid.clone()];
    assert!(
        missing_blobs(&objects, format!("{oid} blob\n").as_bytes())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        missing_blobs(&objects, format!("{oid} missing\n").as_bytes()).unwrap(),
        objects
    );
    for response in [
        format!("{} blob\n", "b".repeat(40)),
        format!("{oid} commit\n"),
        format!("{oid} blob"),
        format!("{oid} blob\r\n"),
        format!("{oid} blob\nextra\n"),
        String::new(),
    ] {
        assert!(
            missing_blobs(&objects, response.as_bytes()).is_err(),
            "{response:?}"
        );
    }
}
