//! 嵌套生产者的原进程交接。证明与来源准入仍由 verified_verifier 负责。
use crate::{
    ProcessResult, file_identity, no_symlink_parents, sha256, verified_verifier as verifier,
};
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub tool: String,
    pub executable: String,
    pub argv: Vec<String>,
    pub output_contract: Option<verifier::OutputContract>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Original {
    pub schema: String,
    pub operation: String,
    pub declaration: Declaration,
    /// 生产者在启动前确定的完整计划；不是从实际 argv 反推。
    pub resolved_argv: Vec<String>,
    pub executable: String,
    pub executable_sha256: String,
    pub component_directory: String,
    pub process: ProcessResult,
    pub compiler_dependency: Option<Result<Vec<u8>, String>>,
    pub compiler_artifact: Option<Result<verifier::CompiledArtifact, String>>,
}

pub struct CompilerIo<'a>(pub &'a Path);
impl verifier::sources::DependencyRead for CompilerIo<'_> {
    fn read(&self, path: &[u8], limit: u64) -> Result<Vec<u8>, String> {
        let path = std::str::from_utf8(path).map_err(|e| e.to_string())?;
        let file = fs::File::open(no_symlink_parents(self.0, path)?).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(limit)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Ok(bytes)
    }
}
impl verifier::ArtifactRead for CompilerIo<'_> {
    fn identity(&self, path: &[u8]) -> Result<(Vec<u8>, u64), String> {
        let path = std::str::from_utf8(path).map_err(|e| e.to_string())?;
        file_identity(&no_symlink_parents(self.0, path)?)
            .map(|(digest, length)| (digest.into_bytes(), length))
    }
}

impl Original {
    /// 原进程实际结束后取得编译观察；失败时保存原错误，不补造库或 dep-info。
    pub fn capture(
        root: &Path,
        operation: &str,
        declaration: Declaration,
        resolved_argv: Vec<String>,
        executable: String,
        executable_sha256: String,
        component_directory: String,
        process: ProcessResult,
    ) -> Self {
        let contract = declaration.output_contract.clone();
        Self {
            schema: "chrono-nested-proof-original/v1".into(),
            operation: operation.into(),
            compiler_dependency: verifier::sources::observe_dependency(
                &contract,
                &resolved_argv,
                process.exit_code,
                process.failure.is_some(),
                declaration.output_limit_bytes,
                &CompilerIo(root),
            ),
            compiler_artifact: verifier::observe_artifact(
                &contract,
                &resolved_argv,
                process.exit_code,
                process.failure.is_some(),
                &CompilerIo(root),
            ),
            declaration,
            resolved_argv,
            executable,
            executable_sha256,
            component_directory,
            process,
        }
    }

    /// 交接关系与实际进程观察先核对；返回成功不表示证明或输入资格通过。
    pub fn validate(
        &self,
        root: &Path,
        operation: &str,
        selected: &Declaration,
    ) -> Result<(), String> {
        crate::relative_path(&self.component_directory)?;
        let p = &self.process;
        let mut argv = vec![self.executable.clone()];
        argv.extend(self.resolved_argv.iter().cloned());
        if self.schema != "chrono-nested-proof-original/v1"
            || self.operation != operation
            || self.declaration != *selected
            || self.executable != p.executable.to_string_lossy()
            || self.executable_sha256 != p.sha256
            || !Path::new(&self.executable).is_absolute()
            || self.executable
                != root
                    .join(
                        selected
                            .executable
                            .replace("$COMPONENT", &self.component_directory),
                    )
                    .to_string_lossy()
            || p.argv != argv
            || p.cwd != root
            || !p.owned_work_joined
            || !p.stdout_complete
            || !p.stderr_complete
            || p.stdin_sha256 != sha256(&[])
            || p.environment_digest != crate::wire::digest(&p.environment)?
            || p.stdout_bytes.len() > selected.output_limit_bytes
            || p.stderr_bytes.len() > selected.output_limit_bytes
            || p.stdout_sha256 != sha256(&p.stdout_bytes)
            || p.stderr_sha256 != sha256(&p.stderr_bytes)
            || p.stdout != String::from_utf8_lossy(&p.stdout_bytes)
            || p.stderr != String::from_utf8_lossy(&p.stderr_bytes)
            || p.timeout_seconds != Some(selected.timeout_seconds)
            || p.output_limit_bytes != Some(selected.output_limit_bytes)
        {
            return Err(
                "E_VERUS_NESTED: original producer, declaration, process, join or stream differs"
                    .into(),
            );
        }
        verifier::nested::validate_resolution(&selected.argv, &self.resolved_argv)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub reason: String,
    pub operation: String,
    pub path: String,
    pub sha256: String,
    pub length: u64,
}

/// 只接受原包装器 stdout 中唯一的结构化引用；缺失、歧义、部分输出和字节漂移拒绝。
pub fn read_original(
    root: &Path,
    operation: &str,
    stdout: &[u8],
    limit: usize,
) -> Result<Original, String> {
    let mut reference = None;
    for value in serde_json::Deserializer::from_slice(stdout).into_iter::<serde_json::Value>() {
        let value = value.map_err(|e| format!("E_VERUS_NESTED: malformed producer stdout: {e}"))?;
        if value["reason"] == "chrono-nested-proof-original" {
            if reference.is_some() {
                return Err("E_VERUS_NESTED: ambiguous original reference".into());
            }
            reference =
                Some(serde_json::from_value::<Reference>(value).map_err(|e| e.to_string())?);
        }
    }
    let reference = reference.ok_or("E_VERUS_NESTED: missing original reference")?;
    if reference.operation != operation || reference.length > limit as u64 {
        return Err("E_VERUS_NESTED: original identity or bound differs".into());
    }
    crate::relative_path(&reference.path)?;
    let path = no_symlink_parents(root, &reference.path)?;
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .map_err(|e| e.to_string())?
        .take((limit as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 != reference.length || sha256(&bytes) != reference.sha256 {
        return Err("E_VERUS_NESTED: original bytes differ".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
