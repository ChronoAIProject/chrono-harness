//! 固定请求的只读获取适配。仓内准入与失败状态由 Verus 所有者产生。
use chrono_harness::{
    Request,
    facts::Reader,
    verified_verifier::{
        CompilerReport, CompilerReportRead, ProofAdmission, ProofInvocation, ProofProcess,
        sources::{CandidateSourceRead, SourceRegistry},
    },
};
use chrono_judge_routes::{Operation, Receipt};
use std::collections::BTreeMap;

pub(super) struct CandidateIo<'a> {
    pub reader: &'a Reader,
    pub request: &'a Request,
    pub tree: &'a BTreeMap<String, (String, String)>,
}

impl CandidateSourceRead for CandidateIo<'_> {
    fn text(&self, path: &[u8]) -> Result<String, String> {
        std::str::from_utf8(path)
            .map(str::to_owned)
            .map_err(|e| e.to_string())
    }

    fn mode(&self, path: &str) -> Option<String> {
        self.tree.get(path).map(|entry| entry.0.clone())
    }

    fn blob(&self, path: &str) -> Result<Vec<u8>, String> {
        self.reader
            .blob(&self.request.host_root, &self.request.candidate, path)
    }

    fn digest(&self, bytes: &[u8]) -> Vec<u8> {
        chrono_harness::sha256(bytes).into_bytes()
    }
}

impl CompilerReportRead for CandidateIo<'_> {
    fn decode(&self, bytes: &[u8]) -> Option<CompilerReport> {
        serde_json::from_slice(bytes).ok()
    }
}

/// 只借用现役计划及同一原回执的字段；选择、比较和准入都由 Verus 所有者执行。
pub(super) fn invocations(operations: &[Operation]) -> Vec<ProofInvocation<'_>> {
    chrono_judge_routes::proof_invocations(operations)
}

pub(super) fn bind_output(
    state: &str,
    operation: &str,
    methods: &Vec<ProofInvocation<'_>>,
    original: Option<&Receipt>,
    registry: &Result<SourceRegistry, String>,
    io: &CandidateIo<'_>,
) -> Result<Option<ProofAdmission>, String> {
    let nested = original
        .and_then(|r| r.nested_proof.as_ref())
        .map(|r| r.as_ref())
        .transpose()?;
    if let Some(child) = nested {
        child.validate(&io.request.host_root, operation, &child.declaration)?;
        for method in methods.iter().filter(|m| m.operation == operation) {
            if method.argv != &child.declaration.argv
                || method.output_contract != &child.declaration.output_contract
            {
                return Err("E_VERUS_NESTED: child declaration differs from selected proof".into());
            }
        }
    }
    let resolved = methods
        .iter()
        .map(|method| ProofInvocation {
            operation: method.operation,
            argv: nested
                .filter(|_| method.operation == operation)
                .map(|child| &child.resolved_argv)
                .unwrap_or(method.argv),
            output_contract: method.output_contract,
            output_limit_bytes: method.output_limit_bytes,
        })
        .collect();
    chrono_harness::verified_verifier::bind_operation_output(
        state,
        operation,
        &resolved,
        nested
            .map(|child| ProofProcess {
                operation: &child.operation,
                argv: &child.process.argv,
                stdout: &child.process.stdout_bytes,
                exit: child.process.exit_code,
                failed: child.process.failure.is_some(),
                dependency: &child.compiler_dependency,
                artifact: &child.compiler_artifact,
            })
            .or_else(|| {
                original.map(|receipt| ProofProcess {
                    operation: &receipt.operation,
                    argv: &receipt.process.argv,
                    stdout: &receipt.process.stdout_bytes,
                    exit: receipt.process.exit_code,
                    failed: receipt.process.failure.is_some(),
                    dependency: &receipt.compiler_dependency,
                    artifact: &receipt.compiler_artifact,
                })
            }),
        registry,
        io,
        |message, path| format!("{message}{path}"),
    )
}
