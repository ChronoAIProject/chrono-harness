//! Failure 的完整 owned 字段状态；真实错误所有者直接搬入、取回同一批字段。
#[cfg(not(verus_keep_ghost))]
use crate::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// 实际错误节点；上下文保持调用方的完整类型，不在证明中替换为摘要。
#[cfg_attr(not(verus_keep_ghost), derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
#[cfg_attr(not(verus_keep_ghost), serde(bound(deserialize = "C: serde::Deserialize<'de>")))]
pub struct ErrorNode<C> {
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "type"))]
    pub error_type: String,
    pub code: String,
    pub message: String,
    pub context: C,
    pub source: Option<Box<ErrorNode<C>>>,
    #[cfg_attr(not(verus_keep_ghost), serde(default, skip_serializing_if = "Option::is_none"))]
    pub native_code: Option<i64>,
    #[cfg_attr(not(verus_keep_ghost), serde(default, skip_serializing_if = "Option::is_none"))]
    pub traceback: Option<String>,
    #[cfg_attr(not(verus_keep_ghost), serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub related: Vec<RelatedError<C>>,
}

#[cfg_attr(not(verus_keep_ghost), derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct RelatedError<C> {
    pub role: String,
    pub error: ErrorNode<C>,
}

impl<C> ErrorNode<C> {
    /// 无原因的新节点精确保留输入，不凭消息或上下文虚构错误链。
    pub fn new(error_type: String, code: String, message: String, context: C)
        -> (out: Self)
        ensures out.error_type == error_type, out.code == code, out.message == message,
            out.context == context, out.source.is_none(), out.native_code.is_none(),
            out.traceback.is_none(), out.related@ == Seq::<RelatedError<C>>::empty(),
    {
        Self {
            error_type, code, message, context,
            source: None, native_code: None, traceback: None, related: Vec::new(),
        }
    }
}

pub type State<N, E> = (N, Option<E>, Vec<(String, E)>);

/// 新错误保留原节点，不虚构原因或关联错误。
pub fn initialize<N, E>(node: N) -> (out: State<N, E>)
    ensures out.0 == node, out.1.is_none(), out.2@ == Seq::<(String, E)>::empty(),
{
    (node, None, Vec::new())
}

/// 原因字段采用完整真实错误，节点和已有关联错误均保持原值。
pub fn set_source<N, E>(state: State<N, E>, source: E) -> (out: State<N, E>)
    ensures out.0 == state.0, out.1 == Some(source), out.2@ == state.2@,
{
    let (node, _, related) = state;
    (node, Some(source), related)
}

/// 按原次序精确追加；重复和空角色均保留，节点及原原因不变。
pub fn append_related<N, E>(state: State<N, E>, role: String, error: E)
    -> (out: State<N, E>)
    ensures out.0 == state.0, out.1 == state.1,
        out.2@ == state.2@.push((role, error)),
{
    let (node, source, mut related) = state;
    related.push((role, error));
    (node, source, related)
}

}
