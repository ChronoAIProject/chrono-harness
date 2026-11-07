//! 当前候选的 Git 观察；不执行历史程序，观察失败保留错误及已取得的字段。
use crate::bindings::{self, Environment};
pub use crate::verified_bindings::{
    SourceIdentity, SourceObservation, SourceState, source_identity,
};
use chrono_harness::resolve_program;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// 候选 Git 只读取明确固定的提交数据；不执行 producer 的程序，也不读漂移 HEAD。
pub struct Committed<'a> {
    root: &'a Path,
    environment: &'a Environment,
    git: PathBuf,
    commit: String,
    tree: String,
}

impl<'a> Committed<'a> {
    pub fn new(
        root: &'a Path,
        environment: &'a Environment,
        commit: &str,
        tree: &str,
    ) -> Result<Self, String> {
        if !crate::verified_bindings::immutable_producer(commit.as_bytes(), tree.as_bytes()) {
            return Err("immutable producer commit and tree are required".into());
        }
        let search = environment
            .get(OsStr::new("PATH"))
            .map(|value| value.to_str().ok_or("non UTF-8 PATH"))
            .transpose()?;
        let git = resolve_program(root, "git", search)?;
        let reader = Self {
            root,
            environment,
            git,
            commit: commit.into(),
            tree: tree.into(),
        };
        let observed = reader.query(&["rev-parse", "--verify", &format!("{commit}^{{tree}}")])?;
        if observed != format!("{tree}\n").as_bytes() {
            return Err("immutable producer tree differs".into());
        }
        Ok(reader)
    }

    fn query(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let mut command = bindings::command(
            self.root,
            &self.git,
            &["--no-optional-locks", "--no-replace-objects"],
            self.environment,
        )?;
        command
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE");
        let output = command.output().map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "immutable producer Git failed: {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(output.stdout)
    }

    pub fn observation(&self) -> SourceObservation {
        SourceObservation {
            commit: Some(self.commit.clone()),
            tree: Some(self.tree.clone()),
            dirty: Some(false),
            error: None,
        }
    }

    pub fn read(&self, declared: &str) -> Result<Vec<u8>, String> {
        if !crate::verified_bindings::host_path(declared.as_bytes(), b"") {
            return Err(format!("invalid immutable producer path: {declared}"));
        }
        // 核固定 Git 对象的文件类型；当前工作树的删除、改写或符号链接不参与原件读取。
        let listing = self.query(&["ls-tree", "-z", &self.commit, "--", declared])?;
        let record = listing
            .strip_suffix(&[0])
            .ok_or("missing immutable producer file")?;
        let record = std::str::from_utf8(record).map_err(|error| error.to_string())?;
        let (metadata, name) = record
            .split_once('\t')
            .ok_or("invalid immutable producer file record")?;
        let fields: Vec<_> = metadata.split_ascii_whitespace().collect();
        if fields.len() != 3
            || !matches!(fields[0], "100644" | "100755")
            || fields[1] != "blob"
            || name != declared
            || !chrono_harness::verified_identifiers::valid_oid(fields[2].as_bytes())
        {
            return Err(format!(
                "immutable producer input is not a regular file: {declared}"
            ));
        }
        self.query(&["cat-file", "blob", fields[2]])
    }
}

pub fn observe(root: &Path, environment: &Environment) -> SourceObservation {
    let mut observation = SourceObservation {
        commit: None,
        tree: None,
        dirty: None,
        error: None,
    };
    let result = (|| -> Result<(), String> {
        let path = environment
            .get(OsStr::new("PATH"))
            .map(|value| value.to_str().ok_or("non UTF-8 PATH"))
            .transpose()?;
        let git = resolve_program(root, "git", path)?;
        let read = |args: &[&str]| -> Result<String, String> {
            let mut command = bindings::command(
                root,
                &git,
                &["--no-optional-locks", "--no-replace-objects"],
                environment,
            )?;
            command
                .args(args)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE");
            bindings::output_text(
                command.output().map_err(|error| error.to_string())?,
                "source Git",
            )
        };
        let identity = read(&["rev-parse", "--show-toplevel", "--verify", "HEAD"])?;
        let (source_root, commit) = identity
            .rsplit_once('\n')
            .ok_or("missing source root/commit")?;
        if std::fs::canonicalize(source_root).map_err(|error| error.to_string())? != root {
            return Err("host root is not the Git source root".into());
        }
        observation.commit = Some(commit.into());
        observation.tree = Some(read(&[
            "rev-parse",
            "--verify",
            &format!("{commit}^{{tree}}"),
        ])?);
        observation.dirty = Some(
            !read(&[
                "status",
                "--porcelain=v1",
                "--untracked-files=normal",
                "--ignore-submodules=none",
            ])?
            .is_empty(),
        );
        if read(&["rev-parse", "--verify", "HEAD"])? != commit {
            return Err("source HEAD changed during observation".into());
        }
        Ok(())
    })();
    observation.error = result.err();
    observation
}
