//! POSIX 相对路径的词法准入。调用方提供 UTF-8 路径字节；
//! 文件系统身份、规范拼写和 glob 政策另由各自所有者处理。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

/// 只接受带末尾斜杠、分量非空且不含单独 . 或 .. 的声明，不归一化输入。
pub open spec fn artifact_component_ok(path: Seq<u8>, i: int) -> bool {
    (i == 0 || path[i - 1] == 47) ==> path[i] != 47
        && !(path[i] == 46 && i + 1 < path.len() && path[i + 1] == 47)
        && !(path[i] == 46 && i + 2 < path.len()
            && path[i + 1] == 46 && path[i + 2] == 47)
}

pub open spec fn artifact_directory(path: Seq<u8>) -> bool {
    path.len() > 0 && path.last() == 47
        && forall|i: int| 0 <= i < path.len() ==> artifact_component_ok(path, i)
}

pub fn valid_artifact_directory(path: &[u8]) -> (valid: bool)
    ensures valid == artifact_directory(path@),
{
    if path.len() == 0 || path[path.len() - 1] != 47 { return false; }
    let mut i = 0;
    while i < path.len()
        invariant path.len() > 0, path@[path.len() - 1] == 47, i <= path.len(),
            forall|j: int| 0 <= j < i ==> artifact_component_ok(path@, j),
        decreases path.len() - i,
    {
        if i == 0 || path[i - 1] == 47 {
            if path[i] == 47
                || (path[i] == 46 && path.len() - i >= 2 && path[i + 1] == 47)
                || (path[i] == 46 && path.len() - i >= 3
                    && path[i + 1] == 46 && path[i + 2] == 47)
            {
                assert(!artifact_component_ok(path@, i as int));
                return false;
            }
        }
        i += 1;
    }
    true
}

pub open spec fn literal_directory(path: &str) -> Option<&str> {
    if artifact_directory(path.spec_bytes()) { Some(path) }
    else { None }
}

/// 完整过滤保持输入顺序、重复和原文借用；字面前缀由原标准格式接口输出。
pub fn literal_artifact_directories<'a>(paths: &[&'a str]) -> (out: Vec<&'a str>)
    ensures out@ == paths@.filter_map(|path: &str| literal_directory(path)),
{
    let mut out = Vec::new();
    let mut i = 0;
    while i < paths.len()
        invariant i <= paths.len(),
            out@ == paths@.take(i as int).filter_map(|path: &str| literal_directory(path)),
        decreases paths.len() - i,
    {
        proof {
            paths@.lemma_filter_map_take_succ(|path: &str| literal_directory(path), i as int);
        }
        let path = paths[i];
        if valid_artifact_directory(path.as_bytes()) {
            out.push(path);
        }
        i += 1;
    }
    assert(paths@.take(paths.len() as int) =~= paths@);
    out
}

pub enum ArtifactDeclarationError {
    Missing,
    Invalid { row: usize },
}

impl ArtifactDeclarationError {
    pub fn message(self) -> (out: &'static str)
        ensures out@ == match self {
            Self::Missing => "missing artifact declarations"@,
            Self::Invalid { .. } => "invalid artifact path"@,
        },
    {
        match self {
            Self::Missing => "missing artifact declarations",
            Self::Invalid { .. } => "invalid artifact path",
        }
    }
}

pub open spec fn artifact_path_observed<'a, T, F>(observe: F, row: &'a T) -> bool
    where F: Fn(&'a T) -> Option<&'a str>,
{
    exists|path: &'a str| #[trigger] observe.ensures((row,), Some(path))
}

/// 按原顺序观察声明，遇到首个非字符串字段就停止。成功保留完整借用序列。
pub fn collect_artifact_declarations<'a, T, F>(rows: Option<&'a [T]>, observe: F)
    -> (out: Result<Vec<&'a str>, ArtifactDeclarationError>)
    where F: Fn(&'a T) -> Option<&'a str>,
    requires forall|row: &'a T| observe.requires((row,)),
    ensures match (rows, out) {
        (None, Err(ArtifactDeclarationError::Missing)) => true,
        (Some(rows), Ok(paths)) => paths.len() == rows.len()
            && forall|i: int| 0 <= i < rows.len() ==>
                #[trigger] observe.ensures((&rows@[i],), Some(paths@[i])),
        (Some(rows), Err(ArtifactDeclarationError::Invalid { row })) => row < rows.len()
            && observe.ensures((&rows@[row as int],), None)
            && forall|j: int| 0 <= j < row ==> #[trigger] artifact_path_observed(observe, &rows@[j]),
        _ => false,
    },
{
    let declarations = match rows {
        Some(declarations) => declarations,
        None => return Err(ArtifactDeclarationError::Missing),
    };
    let mut paths = Vec::new();
    let mut i = 0;
    while i < declarations.len()
        invariant
            rows == Some(declarations), i <= declarations.len(), paths.len() == i,
            forall|row: &'a T| observe.requires((row,)),
            forall|j: int| 0 <= j < i ==>
                #[trigger] observe.ensures((&declarations@[j],), Some(paths@[j])),
        decreases declarations.len() - i,
    {
        let observed = observe(&declarations[i]);
        match observed {
            Some(path) => paths.push(path),
            None => {
                proof {
                    assert(observe.ensures((&declarations@[i as int],), None));
                    assert forall|j: int| 0 <= j < i implies
                        #[trigger] artifact_path_observed(observe, &declarations@[j]) by {
                        assert(observe.ensures((&declarations@[j],), Some(paths@[j])));
                    }
                }
                return Err(ArtifactDeclarationError::Invalid { row: i });
            },
        }
        i += 1;
    }
    Ok(paths)
}

#[verifier::inline]
pub open spec fn path_byte(byte: u8) -> bool {
    byte != 0 && byte != 10 && byte != 13 && byte != 92
}

#[verifier::inline]
pub open spec fn parent_component_at(path: Seq<u8>, i: int) -> bool {
    0 <= i < path.len()
        && (i == 0 || path[i - 1] == 47)
        && path[i] == 46
        && i + 1 < path.len() && path[i + 1] == 46
        && (i + 2 == path.len() || path[i + 2] == 47)
}

/// 精确匹配 runner 的 POSIX 词法范围：拒绝根、开头的当前目录、父目录分量、
/// NUL、LF、CR 和反斜杠；重复／末尾分隔符及非开头的当前目录分量不影响准入。
#[verifier::inline]
pub open spec fn relative_path(path: Seq<u8>) -> bool {
    path.len() > 0 && path[0] != 47
        && !(path[0] == 46 && (path.len() == 1 || path[1] == 47))
        && (forall|i: int| 0 <= i < path.len() ==> (
            path_byte(#[trigger] path[i]) && !parent_component_at(path, i)
        ))
}

pub fn valid_relative_path(path: &[u8]) -> (valid: bool)
    ensures valid == relative_path(path@),
{
    if path.len() == 0 || path[0] == 47
        || (path[0] == 46 && (path.len() == 1 || path[1] == 47))
    {
        return false;
    }
    let mut i = 0;
    while i < path.len()
        invariant
            0 <= i <= path.len(),
            path.len() > 0 && path[0] != 47,
            !(path[0] == 46 && (path.len() == 1 || path[1] == 47)),
            forall|j: int| 0 <= j < i ==> (
                path_byte(#[trigger] path[j]) && !parent_component_at(path@, j)
            ),
        decreases path.len() - i,
    {
        let byte = path[i];
        if byte == 0 || byte == 10 || byte == 13 || byte == 92 {
            return false;
        }
        if byte == 46 && (i == 0 || path[i - 1] == 47)
            && path.len() - i >= 2 && path[i + 1] == 46
            && (path.len() - i == 2 || path[i + 2] == 47)
        {
            return false;
        }
        i += 1;
    }
    true
}

#[verifier::inline]
pub open spec fn host_state_prefix() -> Seq<u8> {
    seq![46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47]
}

/// state 根之后须有非空字节后缀；不增加规范拼写或文件系统身份约束。
#[verifier::inline]
pub open spec fn host_state_path(path: Seq<u8>) -> bool {
    relative_path(path) && path.len() > 22
        && (forall|i: int| 0 <= i < 22 ==> #[trigger] path[i] == host_state_prefix()[i])
}

pub fn valid_host_state_path(path: &[u8]) -> (valid: bool)
    ensures valid == host_state_path(path@),
{
    if path.len() <= 22 {
        return false;
    }
    let prefix = [46u8, 99, 104, 114, 111, 110, 111, 45, 104, 97, 114, 110, 101, 115, 115, 47, 115, 116, 97, 116, 101, 47];
    let mut i = 0;
    while i < 22
        invariant
            0 <= i <= 22,
            path.len() > 22,
            prefix@ == host_state_prefix(),
            forall|j: int| 0 <= j < i ==> #[trigger] path[j] == host_state_prefix()[j],
        decreases 22 - i,
    {
        if path[i] != prefix[i] {
            return false;
        }
        i += 1;
    }
    valid_relative_path(path)
}

pub enum ParentPathFailure<P, E> {
    Symlink(P),
    Native(E),
}

impl<P, E> ParentPathFailure<P, E> {
    /// 渲染只调用实际失败对应的接口，完整移交该路径或原错误。
    pub fn render<T, L, R>(self, symlink: L, native: R) -> (out: T)
        where L: Fn(P) -> T, R: Fn(E) -> T,
        requires forall|path: P| symlink.requires((path,)),
            forall|error: E| native.requires((error,)),
        ensures match self {
            Self::Symlink(path) => symlink.ensures((path,), out),
            Self::Native(error) => native.ensures((error,), out),
        },
    {
        match self {
            Self::Symlink(path) => symlink(path),
            Self::Native(error) => native(error),
        }
    }
}

/// 标准有限组件迭代器的接口合同；rank 只在证明中使用。
pub open spec fn component_steps_finite<S, C, N>(next: N, rank: spec_fn(S) -> nat) -> bool
    where N: Fn(S) -> (S, Option<C>),
{
    forall|source: S, observed: (S, Option<C>)|
        #[trigger] next.ensures((source,), observed) && observed.1.is_some()
            ==> rank(observed.0) < rank(source)
}

pub open spec fn parent_path_continues<E, F>(observed: Result<bool, E>, is_missing: F) -> bool
    where F: Fn(&E) -> bool,
{
    match observed {
        Ok(symlink) => !symlink,
        Err(error) => is_missing.ensures((&error,), true),
    }
}

#[verifier::opaque]
pub open spec fn parent_path_step<S, P, C, E, N, A, M, F>(
    before: (S, P), after: (S, P), next: N, append: A, metadata: M, is_missing: F,
) -> bool
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>), F: Fn(&E) -> bool,
{
    exists|component: C, built: P, observation: Result<bool, E>|
        next.ensures((before.0,), (after.0, Some(component)))
        && append.ensures((before.1, component), built)
        && metadata.ensures((built,), (after.1, observation))
        && parent_path_continues(observation, is_missing)
}

pub open spec fn parent_path_prefix<S, P, C, E, N, A, M, F>(
    states: Seq<(S, P)>, initial: (S, P), next: N, append: A, metadata: M, is_missing: F,
) -> bool
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>), F: Fn(&E) -> bool,
{
    states.len() > 0 && states.first() == initial
        && forall|i: int| 0 <= i < states.len() - 1 ==>
            #[trigger] parent_path_step::<S, P, C, E, N, A, M, F>(
                states[i], states[i + 1], next, append, metadata, is_missing)
}

pub open spec fn parent_path_terminal<S, P, C, E, N, A, M, F>(
    last: (S, P), following: (S, Option<C>), built: Option<P>,
    observed: Option<(P, Result<bool, E>)>, next: N, append: A, metadata: M, is_missing: F,
    out: Result<P, ParentPathFailure<P, E>>,
) -> bool
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>), F: Fn(&E) -> bool,
{
    next.ensures((last.0,), following) && match following.1 {
        None => built.is_none() && observed.is_none()
            && out == Ok::<P, ParentPathFailure<P, E>>(last.1),
        Some(component) => match (built, observed) {
            (Some(built), Some(observed)) => append.ensures((last.1, component), built)
                && metadata.ensures((built,), observed) && match observed.1 {
                    Ok(true) => out == Err::<P, ParentPathFailure<P, E>>(
                        ParentPathFailure::Symlink(observed.0)),
                    Err(error) => is_missing.ensures((&error,), false)
                        && out == Err::<P, ParentPathFailure<P, E>>(ParentPathFailure::Native(error)),
                    _ => false,
                },
            _ => false,
        },
    }
}

pub open spec fn parent_path_result<S, P, C, E, N, A, M, F>(
    initial: S, root: P, next: N, append: A, metadata: M, is_missing: F,
    out: Result<P, ParentPathFailure<P, E>>,
) -> bool
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>), F: Fn(&E) -> bool,
{
    exists|states: Seq<(S, P)>, following: (S, Option<C>), built: Option<P>,
        observed: Option<(P, Result<bool, E>)>|
        parent_path_prefix::<S, P, C, E, N, A, M, F>(states, (initial, root), next, append, metadata, is_missing)
        && parent_path_terminal(states.last(), following, built, observed,
            next, append, metadata, is_missing, out)
}

/// 每个真实组件先追加、再观察；缺失继续，首个 symlink 或其他错误立即停止。
pub fn no_symlink_path<S, P, C, E, N, A, M, F>(
    initial: S, root: P, next: N, append: A, metadata: M, is_missing: F,
) -> (out: Result<P, ParentPathFailure<P, E>>)
    where N: Fn(S) -> (S, Option<C>), A: Fn(P, C) -> P,
          M: Fn(P) -> (P, Result<bool, E>), F: Fn(&E) -> bool,
    requires
        forall|source: S| #[trigger] next.requires((source,)),
        exists|rank: spec_fn(S) -> nat| component_steps_finite::<S, C, N>(next, rank),
        forall|path: P, component: C| #[trigger] append.requires((path, component)),
        forall|path: P| #[trigger] metadata.requires((path,)),
        forall|error: &E| #[trigger] is_missing.requires((error,)),
    ensures parent_path_result(initial, root, next, append, metadata, is_missing, out),
{
    let ghost rank = choose|rank: spec_fn(S) -> nat| component_steps_finite::<S, C, N>(next, rank);
    let mut source = initial;
    let mut path = root;
    let ghost states = seq![(initial, root)];
    loop
        invariant
            parent_path_prefix::<S, P, C, E, N, A, M, F>(states, (initial, root), next, append, metadata, is_missing),
            states.last() == (source, path),
            component_steps_finite::<S, C, N>(next, rank),
            forall|source: S| #[trigger] next.requires((source,)),
            forall|path: P, component: C| #[trigger] append.requires((path, component)),
            forall|path: P| #[trigger] metadata.requires((path,)),
            forall|error: &E| #[trigger] is_missing.requires((error,)),
        decreases rank(source),
    {
        let following = next(source);
        let ghost following_observation = following;
        let (after, part) = following;
        source = after;
        let component = match part {
            None => {
                assert(parent_path_terminal(states.last(), following_observation, None, None,
                    next, append, metadata, is_missing, Ok::<P, ParentPathFailure<P, E>>(path)));
                assert(parent_path_result(initial, root, next, append, metadata, is_missing,
                    Ok::<P, ParentPathFailure<P, E>>(path)));
                return Ok(path);
            },
            Some(component) => component,
        };
        let ghost original_component = component;
        path = append(path, component);
        let ghost built = path;
        let observation = metadata(path);
        let ghost original = observation;
        let (after_path, result) = observation;
        path = after_path;
        match result {
            Ok(true) => {
                let error = ParentPathFailure::Symlink(path);
                assert(parent_path_terminal(states.last(), following_observation, Some(built), Some(original),
                    next, append, metadata, is_missing, Err::<P, ParentPathFailure<P, E>>(error)));
                assert(parent_path_result(initial, root, next, append, metadata, is_missing,
                    Err::<P, ParentPathFailure<P, E>>(error)));
                return Err(error);
            },
            Ok(false) => {},
            Err(error) => {
                if !is_missing(&error) {
                    let failure = ParentPathFailure::Native(error);
                    assert(parent_path_terminal(states.last(), following_observation, Some(built), Some(original),
                        next, append, metadata, is_missing, Err::<P, ParentPathFailure<P, E>>(failure)));
                    assert(parent_path_result(initial, root, next, append, metadata, is_missing,
                        Err::<P, ParentPathFailure<P, E>>(failure)));
                    return Err(failure);
                }
            },
        }
        let ghost after_state = (source, path);
        assert(next.ensures((states.last().0,), (source, Some(original_component))));
        assert(append.ensures((states.last().1, original_component), built));
        assert(metadata.ensures((built,), original));
        assert(original.0 == path);
        assert(parent_path_continues(original.1, is_missing));
        assert(parent_path_step::<S, P, C, E, N, A, M, F>(states.last(), after_state,
            next, append, metadata, is_missing)) by {
            reveal(parent_path_step);
        }
        proof {
            let previous = states;
            states = states.push(after_state);
            assert forall|i: int| 0 <= i < states.len() - 1 implies
                #[trigger] parent_path_step::<S, P, C, E, N, A, M, F>(states[i], states[i + 1],
                    next, append, metadata, is_missing) by {
                if i < previous.len() - 1 {
                    assert(parent_path_step::<S, P, C, E, N, A, M, F>(previous[i], previous[i + 1],
                        next, append, metadata, is_missing));
                }
            }
        }
    }
}

}
