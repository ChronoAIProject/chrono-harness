//! Workflow 原错误消息到实际协议错误码与严重程度的唯一判断。
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_status.rs"]
pub mod statuses;
#[cfg(not(verus_keep_ghost))]
pub use chrono_harness::verified_identifiers as identifiers;
#[cfg(not(verus_keep_ghost))]
pub use chrono_harness::wire::Status;
#[cfg(verus_keep_ghost)]
pub use statuses::WireStatus as Status;
#[cfg(verus_keep_ghost)]
pub use statuses::identifiers;

verus! {

pub open spec fn error_code(message: Seq<u8>) -> Seq<u8> {
    let head = message.take(identifiers::ascii_separator_end(message, 58));
    if head.len() >= 2 && head[0] == 69 && head[1] == 95 {
        head
    } else {
        "E_WORKFLOW_INPUT".spec_bytes()
    }
}

pub open spec fn rejected(code: Seq<char>) -> bool {
    code == "E_BRANCH_STALE"@
        || code == "E_RETIREMENT"@
        || code == "E_REQUIRED_TEST_REMOVED"@
        || code == "E_INTEGRATION_REQUIRED"@
        || code == "E_INTEGRATION_MISMATCH"@
}

/// 只取首个 ASCII 冒号之前的完整字段；其余原消息由调用方原样保留。
/// 恰好五种领域拒绝产生 Fail，其余合法错误码及缺省码都产生 Error。
pub fn classify_error(message: &str) -> (out: (&str, Status))
    ensures out.0.spec_bytes() == error_code(message.spec_bytes()),
        out.1 == if rejected(out.0@) { Status::Fail } else { Status::Error },
{
    let (head, _) = identifiers::ascii_split_head(message, 58);
    let bytes = head.as_bytes();
    let code = if bytes.len() >= 2 && bytes[0] == 69 && bytes[1] == 95 {
        head
    } else {
        "E_WORKFLOW_INPUT"
    };
    let is_rejection = identifiers::equal_text(code, "E_BRANCH_STALE")
        || identifiers::equal_text(code, "E_RETIREMENT")
        || identifiers::equal_text(code, "E_REQUIRED_TEST_REMOVED")
        || identifiers::equal_text(code, "E_INTEGRATION_REQUIRED")
        || identifiers::equal_text(code, "E_INTEGRATION_MISMATCH");
    (code, if is_rejection { Status::Fail } else { Status::Error })
}

}
