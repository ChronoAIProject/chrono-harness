//! 故意失败的严格使用实验；不能成为成功的 ProofAdmission。
use serde_json::Value;
use vstd::prelude::*;

verus! {

// 同一新组件不能让仓内函数取得假设许可。
pub fn repository_assumption(value: &Value) -> Option<&str> {
    proof { assume(false); }
    value.as_str()
}

} // verus!
