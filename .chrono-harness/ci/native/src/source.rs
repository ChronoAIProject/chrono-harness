//! 当前候选的 Git 观察；不执行历史程序，观察失败保留错误及已取得的字段。
use crate::bindings::{self, Environment};
pub use crate::verified_bindings::{
    SourceIdentity, SourceObservation, SourceState, source_identity,
};
use chrono_harness::resolve_program;
use std::{ffi::OsStr, path::Path};

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
