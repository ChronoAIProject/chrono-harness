//! 固定请求的只读获取适配。仓内准入与失败状态由 verified_sources 产生。
use chrono_harness::{Request, facts::Reader, verified_verifier::sources::CandidateSourceRead};
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
