//! 字符串 CLI 的完整参数顺序与首个解码失败；OS 解码由标准库提供。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub struct InvalidArgument {
    pub argv_index: usize,
    pub argument_index: Option<usize>,
}

pub open spec fn argument_position(index: usize) -> Option<usize> {
    if index == 0 { None } else { Some((index - 1) as usize) }
}

/// 成功保持完整原文、顺序、空串和重复；失败精确返回首个 None。
/// argv[0] 没有用户参数序号，其余参数的序号从零开始。
pub fn decoded_arguments<'a>(arguments: &[Option<&'a str>])
    -> (out: Result<Vec<&'a str>, InvalidArgument>)
    ensures match out {
        Ok(texts) => texts.len() == arguments.len()
            && forall|j: int| 0 <= j < arguments.len() ==> (
                arguments[j].is_some()
                && texts[j]@ == arguments[j].unwrap()@
            ),
        Err(error) => error.argv_index < arguments.len()
            && arguments[error.argv_index as int].is_none()
            && error.argument_index == argument_position(error.argv_index)
            && forall|j: int| 0 <= j < error.argv_index ==> arguments[j].is_some(),
    },
{
    let mut texts: Vec<&'a str> = Vec::new();
    let mut i = 0;
    while i < arguments.len()
        invariant
            i <= arguments.len(), texts.len() == i,
            forall|j: int| 0 <= j < i ==> (
                arguments[j].is_some()
                && texts[j]@ == arguments[j].unwrap()@
            ),
        decreases arguments.len() - i,
    {
        match arguments[i] {
            Some(text) => texts.push(text),
            None => return Err(InvalidArgument {
                argv_index: i,
                argument_index: if i == 0 { None } else { Some(i - 1) },
            }),
        }
        i += 1;
    }
    Ok(texts)
}

}
