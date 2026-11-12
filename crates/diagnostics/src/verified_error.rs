//! Failure 的完整 owned 字段状态；真实错误所有者直接搬入、取回同一批字段。
#[cfg(not(verus_keep_ghost))]
use crate::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

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
