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
    /// 完整节点只搬运实际字段，保留嵌套原因和有序关联值。
    pub fn assemble(
        error_type: String, code: String, message: String, context: C,
        source: Option<Box<ErrorNode<C>>>, native_code: Option<i64>,
        traceback: Option<String>, related: Vec<RelatedError<C>>,
    ) -> (out: Self)
        ensures out.error_type == error_type, out.code == code, out.message == message,
            out.context == context, out.source == source, out.native_code == native_code,
            out.traceback == traceback, out.related@ == related@,
    {
        Self { error_type, code, message, context, source, native_code, traceback, related }
    }

    /// 无原因的新节点精确保留输入，不凭消息或上下文虚构错误链。
    pub fn new(error_type: String, code: String, message: String, context: C)
        -> (out: Self)
        ensures out.error_type == error_type, out.code == code, out.message == message,
            out.context == context, out.source.is_none(), out.native_code.is_none(),
            out.traceback.is_none(), out.related@ == Seq::<RelatedError<C>>::empty(),
    {
        Self::assemble(error_type, code, message, context, None, None, None, Vec::new())
    }

    /// 替换完整原因和关联表，其余字段保持原值。
    pub fn with_causes(self, source: Option<Box<ErrorNode<C>>>, related: Vec<RelatedError<C>>)
        -> (out: Self)
        ensures out.error_type == self.error_type, out.code == self.code,
            out.message == self.message, out.context == self.context,
            out.native_code == self.native_code, out.traceback == self.traceback,
            out.source == source, out.related@ == related@,
    {
        Self::assemble(self.error_type, self.code, self.message, self.context,
            source, self.native_code, self.traceback, related)
    }

    /// 拆出原原因，不丢失节点的其它字段或关联错误。
    pub fn detach_source(self) -> (out: (Self, Option<Box<ErrorNode<C>>>))
        ensures out.0.error_type == self.error_type, out.0.code == self.code,
            out.0.message == self.message, out.0.context == self.context,
            out.0.native_code == self.native_code, out.0.traceback == self.traceback,
            out.0.related@ == self.related@, out.0.source.is_none(), out.1 == self.source,
    {
        let Self { error_type, code, message, context, source, native_code, traceback, related } = self;
        (Self::assemble(error_type, code, message, context, None, native_code, traceback, related),
            source)
    }

    /// 只替换原因，完整保留原关联表和其它字段。
    pub fn with_source(self, source: Option<Box<ErrorNode<C>>>) -> (out: Self)
        ensures out.error_type == self.error_type, out.code == self.code,
            out.message == self.message, out.context == self.context,
            out.native_code == self.native_code, out.traceback == self.traceback,
            out.related@ == self.related@, out.source == source,
    {
        Self::assemble(self.error_type, self.code, self.message, self.context,
            source, self.native_code, self.traceback, self.related)
    }
}

impl<C> RelatedError<C> {
    pub fn new(role: String, error: ErrorNode<C>) -> (out: Self)
        ensures out.role == role, out.error == error,
    {
        Self { role, error }
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
