//! 编译器依赖路径、规范化及完整源码 checksum 内容的同源规格和实现。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

use super::{append_bytes, equal_bytes};

verus! {

/// 仅获取指定路径的原字节；准入、路径选择和长度判断由下方实现负责。
pub trait DependencyRead {
    fn read(&self, path: &[u8], limit: u64) -> Result<Vec<u8>, String>;
}

pub open spec fn dependency_observed(
    contract: &Option<super::OutputContract>, observed: &Option<Result<Vec<u8>, String>>,
    limit: usize,
) -> bool {
    match contract {
        None => observed.is_none(),
        Some(_) => match observed {
            Some(Ok(bytes)) => 0 < limit <= 67_108_864 && bytes.len() <= limit,
            _ => false,
        },
    }
}

/// 原进程已结束后才获取编译器输出；失败观察仍进入原回执，不替换原进程。
pub fn observe_dependency<I: DependencyRead>(
    contract: &Option<super::OutputContract>, args: &Vec<String>,
    exit: i32, process_failed: bool, limit: usize, io: &I,
) -> (observed: Option<Result<Vec<u8>, String>>)
    ensures observed.is_none() == contract.is_none(),
        match observed {
            Some(Ok(bytes)) => super::verifier_arguments(args@)
                && exit == 0 && !process_failed && 0 < limit <= 67_108_864
                && bytes.len() <= limit,
            _ => true,
        },
{
    if contract.is_none() { return None; }
    if exit != 0 || process_failed || !super::canonical_verifier_arguments(args) {
        return Some(Err("E_VERUS_SOURCES: original compiler process was not successful".to_string()));
    }
    if limit == 0 || limit > 67_108_864 {
        return Some(Err("E_VERUS_SOURCES: invalid dependency observation bound".to_string()));
    }
    let mut path = Vec::new();
    append_bytes(&mut path, args[10].as_str().as_bytes());
    append_bytes(&mut path, &[46u8, 100u8]);
    let read = io.read(path.as_slice(), limit as u64 + 1);
    match read {
        Err(error) => Some(Err(error)),
        Ok(bytes) => {
            if bytes.len() > limit {
                Some(Err("E_VERUS_SOURCES: compiler dependency exceeds its bound".to_string()))
            } else { Some(Ok(bytes)) }
        },
    }
}

pub fn admit_dependency_observation(
    contract: &Option<super::OutputContract>, observed: &Option<Result<Vec<u8>, String>>,
    limit: usize,
) -> (out: Result<(), String>)
    ensures out.is_ok() == dependency_observed(contract, observed, limit),
{
    match contract {
        None => {
            if observed.is_some() {
                Err("E_VERUS_SOURCES: ordinary operation has compiler source evidence".to_string())
            } else { Ok(()) }
        },
        Some(_) => {
            if limit == 0 || limit > 67_108_864 {
                return Err("E_VERUS_SOURCES: invalid dependency observation bound".to_string());
            }
            match observed {
                Some(Ok(bytes)) => if bytes.len() <= limit { Ok(()) }
                    else { Err("E_VERUS_SOURCES: compiler dependency exceeds its bound".to_string()) },
                _ => Err("E_VERUS_SOURCES: missing or failed original dependency observation".to_string()),
            }
        },
    }
}

/// Rust 1.95 的 dep-info 只转义 ASCII 空格。当前有界语法拒绝控制字节和
/// 字面反斜杠，不额外解释 Make 语法。
pub open spec fn dependency_byte(byte: u8) -> bool {
    byte >= 32 && byte != 92 && byte != 127
}

pub open spec fn dependency_word(word: Seq<u8>) -> bool {
    word.len() > 0 && forall|i: int| 0 <= i < word.len() ==> dependency_byte(#[trigger] word[i])
}

pub open spec fn dependency_escape(word: Seq<u8>) -> Seq<u8>
    decreases word.len(),
{
    if word.len() == 0 { Seq::empty() }
    else {
        dependency_escape(word.drop_last())
            + if word.last() == 32 { seq![92u8, 32u8] } else { seq![word.last()] }
    }
}

pub open spec fn dependency_values(paths: Seq<Vec<u8>>) -> Seq<Seq<u8>> {
    paths.map(|i: int, path: Vec<u8>| path@)
}

pub open spec fn dependency_encoding(paths: Seq<Seq<u8>>) -> Seq<u8>
    decreases paths.len(),
{
    if paths.len() == 0 { Seq::empty() }
    else {
        dependency_encoding(paths.drop_last())
            + if paths.len() == 1 { Seq::empty() } else { seq![32u8] }
            + dependency_escape(paths.last())
    }
}

pub open spec fn dependency_prefix(paths: Seq<Seq<u8>>, word: Seq<u8>) -> Seq<u8> {
    if paths.len() == 0 { dependency_escape(word) }
    else { dependency_encoding(paths) + seq![32u8] + dependency_escape(word) }
}

proof fn dependency_prefix_push(paths: Seq<Seq<u8>>, word: Seq<u8>, byte: u8)
    ensures dependency_prefix(paths, word.push(byte)) == dependency_prefix(paths, word)
        + if byte == 32 { seq![92u8, 32u8] } else { seq![byte] },
{
    assert(word.push(byte).drop_last() =~= word);
}

proof fn dependency_encoding_push(paths: Seq<Seq<u8>>, word: Seq<u8>)
    ensures dependency_encoding(paths.push(word)) == dependency_prefix(paths, word),
{
    assert(paths.push(word).drop_last() =~= paths);
    assert(paths.push(word).last() == word);
    if paths.len() == 0 {
        assert(paths =~= Seq::<Seq<u8>>::empty());
        assert(dependency_encoding(paths) == Seq::<u8>::empty());
    }
}

/// 明确规定首尾／重复分隔符及未知转义的失败结果；成功路径保留原顺序。
pub open spec fn dependency_scan(
    input: Seq<u8>, paths: Seq<Seq<u8>>, word: Seq<u8>,
) -> Option<Seq<Seq<u8>>>
    decreases input.len(),
{
    if input.len() == 0 {
        if word.len() == 0 { None } else { Some(paths.push(word)) }
    } else if input[0] == 32 {
        if word.len() == 0 { None }
        else { dependency_scan(input.drop_first(), paths.push(word), Seq::empty()) }
    } else if input[0] == 92 {
        if input.len() < 2 || input[1] != 32 { None }
        else { dependency_scan(input.skip(2), paths, word.push(32u8)) }
    } else if !dependency_byte(input[0]) {
        None
    } else {
        dependency_scan(input.drop_first(), paths, word.push(input[0]))
    }
}

pub fn parse_dependency_paths(input: &[u8]) -> (parsed: Option<Vec<Vec<u8>>>)
    ensures
        match parsed {
            None => dependency_scan(input@, Seq::empty(), Seq::empty()).is_none(),
            Some(paths) => {
                let values = dependency_values(paths@);
                dependency_scan(input@, Seq::empty(), Seq::empty()) == Some(values)
                    && values.len() > 0
                    && (forall|i: int| 0 <= i < values.len() ==> dependency_word(#[trigger] values[i]))
                    && dependency_encoding(values) == input@
            },
        },
{
    let mut paths: Vec<Vec<u8>> = Vec::new();
    let mut word: Vec<u8> = Vec::new();
    let mut i = 0;
    assert(dependency_values(paths@) =~= Seq::<Seq<u8>>::empty());
    assert(input@.skip(0) =~= input@);
    while i < input.len()
        invariant
            0 <= i <= input.len(),
            dependency_scan(input@, Seq::empty(), Seq::empty())
                == dependency_scan(input@.skip(i as int), dependency_values(paths@), word@),
            dependency_prefix(dependency_values(paths@), word@) == input@.take(i as int),
            forall|j: int| 0 <= j < paths.len() ==> dependency_word((#[trigger] paths[j])@),
            forall|j: int| 0 <= j < word.len() ==> dependency_byte(#[trigger] word[j]),
        decreases input.len() - i,
    {
        let ghost suffix = input@.skip(i as int);
        let ghost previous_paths = dependency_values(paths@);
        let ghost previous_word = word@;
        let ghost previous_prefix = dependency_prefix(previous_paths, previous_word);
        let byte = input[i];
        if byte == 32 {
            if word.len() == 0 { return None; }
            let ghost previous = dependency_values(paths@);
            let ghost last = word@;
            paths.push(word);
            word = Vec::new();
            assert(dependency_values(paths@) =~= previous.push(last));
            proof { dependency_encoding_push(previous, last); }
            assert(dependency_prefix(dependency_values(paths@), word@)
                == previous_prefix + seq![32u8]);
            assert(input@.take(i + 1) =~= input@.take(i as int).push(byte));
            assert(input@.skip(i + 1) =~= suffix.drop_first());
            i += 1;
        } else if byte == 92 {
            if input.len() - i < 2 || input[i + 1] != 32 { return None; }
            word.push(32u8);
            proof { dependency_prefix_push(previous_paths, previous_word, 32u8); }
            assert(input@.take(i + 2) =~= input@.take(i as int) + seq![92u8, 32u8]);
            assert(input@.skip(i + 2) =~= suffix.skip(2));
            i += 2;
        } else {
            if byte < 32 || byte == 127 { return None; }
            word.push(byte);
            proof { dependency_prefix_push(previous_paths, previous_word, byte); }
            assert(input@.take(i + 1) =~= input@.take(i as int).push(byte));
            assert(input@.skip(i + 1) =~= suffix.drop_first());
            i += 1;
        }
    }
    if word.len() == 0 { return None; }
    let ghost previous = dependency_values(paths@);
    let ghost last = word@;
    assert(input@.skip(i as int) =~= Seq::<u8>::empty());
    assert(input@.take(i as int) =~= input@);
    paths.push(word);
    assert(dependency_values(paths@) =~= previous.push(last));
    proof { dependency_encoding_push(previous, last); }
    assert(dependency_encoding(dependency_values(paths@)) == input@);
    assert(dependency_scan(input@, Seq::empty(), Seq::empty())
        == Some(dependency_values(paths@)));
    assert forall|j: int| 0 <= j < paths.len()
        implies dependency_word((#[trigger] paths[j])@) by {
        if j < previous.len() { assert(paths@[j]@ == previous[j]); }
        else { assert(paths@[j]@ == last); }
    }
    Some(paths)
}

pub open spec fn component_step(parts: Seq<Seq<u8>>, word: Seq<u8>) -> Option<Seq<Seq<u8>>> {
    if word.len() == 0 || word == seq![46u8] { Some(parts) }
    else if word == seq![46u8, 46u8] {
        if parts.len() == 0 { None } else { Some(parts.drop_last()) }
    } else { Some(parts.push(word)) }
}

pub open spec fn path_scan(
    input: Seq<u8>, parts: Seq<Seq<u8>>, word: Seq<u8>,
) -> Option<Seq<Seq<u8>>>
    decreases input.len(),
{
    if input.len() == 0 { component_step(parts, word) }
    else if !dependency_byte(input[0]) { None }
    else if input[0] == 47 {
        match component_step(parts, word) {
            None => None,
            Some(next) => path_scan(input.drop_first(), next, Seq::empty()),
        }
    } else { path_scan(input.drop_first(), parts, word.push(input[0])) }
}

pub open spec fn relative_components(input: Seq<u8>) -> Option<Seq<Seq<u8>>> {
    if input.len() == 0 || input[0] == 47 { None }
    else {
        match path_scan(input, Seq::empty(), Seq::empty()) {
            Some(parts) => if parts.len() == 0 { None } else { Some(parts) },
            None => None,
        }
    }
}

pub open spec fn joined_components(parts: Seq<Seq<u8>>) -> Seq<u8>
    decreases parts.len(),
{
    if parts.len() == 0 { Seq::empty() }
    else {
        joined_components(parts.drop_last())
            + if parts.len() == 1 { Seq::empty() } else { seq![47u8] }
            + parts.last()
    }
}

pub open spec fn normal_component(word: Seq<u8>) -> bool {
    dependency_word(word) && word != seq![46u8] && word != seq![46u8, 46u8]
        && forall|i: int| 0 <= i < word.len() ==> (#[trigger] word[i]) != 47
}

pub fn finish_component(parts: &mut Vec<Vec<u8>>, word: Vec<u8>) -> (accepted: bool)
    requires
        forall|i: int| 0 <= i < old(parts).len() ==> normal_component((#[trigger] old(parts)[i])@),
        forall|i: int| 0 <= i < word.len() ==> dependency_byte(#[trigger] word[i]) && word[i] != 47,
    ensures
        accepted <==> component_step(dependency_values(old(parts)@), word@).is_some(),
        accepted ==> component_step(dependency_values(old(parts)@), word@)
            == Some(dependency_values(parts@)),
        !accepted ==> parts@ == old(parts)@,
        forall|i: int| 0 <= i < parts.len() ==> normal_component((#[trigger] parts[i])@),
{
    if word.len() == 0 {
        assert(word@ =~= Seq::<u8>::empty());
        return true;
    }
    if word.len() == 1 && word[0] == 46 {
        assert(word@ =~= seq![46u8]);
        return true;
    }
    if word.len() == 2 && word[0] == 46 && word[1] == 46 {
        assert(word@ =~= seq![46u8, 46u8]);
        if parts.len() == 0 { return false; }
        let ghost before = dependency_values(parts@);
        let _ = parts.pop();
        assert(dependency_values(parts@) =~= before.drop_last());
    } else {
        assert(word@ != seq![46u8] && word@ != seq![46u8, 46u8]);
        let ghost before = dependency_values(parts@);
        let ghost last = word@;
        parts.push(word);
        assert(dependency_values(parts@) =~= before.push(last));
    }
    true
}

/// 只做 POSIX 词法规范化。文件系统别名、符号链接及固定源码字节绑定
/// 仍由实际获取接口和消费链负责。
pub fn normalize_relative_path(input: &[u8]) -> (normalized: Option<Vec<u8>>)
    ensures
        match normalized {
            None => relative_components(input@).is_none(),
            Some(path) => relative_components(input@).is_some()
                && path@ == joined_components(relative_components(input@).unwrap())
                && path.len() > 0,
        },
{
    if input.len() == 0 || input[0] == 47 { return None; }
    let mut parts: Vec<Vec<u8>> = Vec::new();
    let mut word: Vec<u8> = Vec::new();
    let mut i = 0;
    assert(dependency_values(parts@) =~= Seq::<Seq<u8>>::empty());
    assert(input@.skip(0) =~= input@);
    while i < input.len()
        invariant
            0 <= i <= input.len(), input.len() > 0, input@[0] != 47,
            path_scan(input@, Seq::empty(), Seq::empty())
                == path_scan(input@.skip(i as int), dependency_values(parts@), word@),
            forall|j: int| 0 <= j < parts.len() ==> normal_component((#[trigger] parts[j])@),
            forall|j: int| 0 <= j < word.len() ==> dependency_byte(#[trigger] word[j]) && word[j] != 47,
        decreases input.len() - i,
    {
        let byte = input[i];
        if byte < 32 || byte == 92 || byte == 127 { return None; }
        if byte == 47 {
            if !finish_component(&mut parts, word) { return None; }
            word = Vec::new();
        } else { word.push(byte); }
        assert(input@.skip(i as int).drop_first() =~= input@.skip(i + 1));
        i += 1;
    }
    assert(input@.skip(i as int) =~= Seq::<u8>::empty());
    if !finish_component(&mut parts, word) || parts.len() == 0 { return None; }
    assert(relative_components(input@) == Some(dependency_values(parts@)));
    let mut path = Vec::new();
    let mut j = 0;
    assert(dependency_values(parts@.take(0)) =~= Seq::<Seq<u8>>::empty());
    while j < parts.len()
        invariant 0 <= j <= parts.len(), parts.len() > 0,
            forall|k: int| 0 <= k < parts.len() ==> normal_component((#[trigger] parts[k])@),
            path@ == joined_components(dependency_values(parts@.take(j as int))),
        decreases parts.len() - j,
    {
        if j > 0 { path.push(47u8); }
        append_bytes(&mut path, parts[j].as_slice());
        proof {
            let prefix = dependency_values(parts@.take(j + 1));
            assert(prefix.drop_last() =~= dependency_values(parts@.take(j as int)));
            assert(prefix.last() == parts@[j as int]@);
        }
        j += 1;
    }
    assert(parts@.take(j as int) =~= parts@);
    assert(normal_component(parts@[parts.len() - 1]@));
    assert(dependency_values(parts@).last() == parts@[parts.len() - 1]@);
    Some(path)
}

pub open spec fn prefix_at(input: Seq<u8>, start: int, prefix: Seq<u8>) -> bool {
    0 <= start <= input.len() && prefix.len() <= input.len() - start
        && input.subrange(start, start + prefix.len()) == prefix
}

pub fn matches_at(input: &[u8], start: usize, prefix: &[u8]) -> (matched: bool)
    ensures matched <==> prefix_at(input@, start as int, prefix@),
{
    if start > input.len() || prefix.len() > input.len() - start { return false; }
    let mut i = 0;
    while i < prefix.len()
        invariant
            start <= input.len(), prefix.len() <= input.len() - start,
            0 <= i <= prefix.len(),
            forall|j: int| 0 <= j < i ==> input@[start + j] == prefix@[j],
        decreases prefix.len() - i,
    {
        if input[start + i] != prefix[i] { return false; }
        i += 1;
    }
    assert(input@.subrange(start as int, start + prefix.len()) =~= prefix@);
    true
}

pub fn copy_range(input: &[u8], start: usize, end: usize) -> (bytes: Vec<u8>)
    requires start <= end <= input.len(),
    ensures bytes@ == input@.subrange(start as int, end as int),
{
    let mut bytes = Vec::new();
    let mut i = start;
    while i < end
        invariant start <= i <= end <= input.len(),
            bytes@ == input@.subrange(start as int, i as int),
        decreases end - i,
    {
        bytes.push(input[i]);
        assert(input@.subrange(start as int, i + 1)
            =~= input@.subrange(start as int, i as int).push(input[i as int]));
        i += 1;
    }
    bytes
}

pub open spec fn next_byte(input: Seq<u8>, start: int, byte: u8) -> int
    recommends 0 <= start <= input.len(),
    decreases input.len() - start,
{
    if start >= input.len() || input[start] == byte { start }
    else { next_byte(input, start + 1, byte) }
}

pub fn find_byte(input: &[u8], start: usize, byte: u8) -> (end: usize)
    requires start <= input.len(),
    ensures start <= end <= input.len(),
        end == next_byte(input@, start as int, byte),
        end < input.len() ==> input@[end as int] == byte,
        forall|j: int| start <= j < end ==> input@[j] != byte,
{
    let mut i = start;
    while i < input.len()
        invariant start <= i <= input.len(),
            next_byte(input@, start as int, byte) == next_byte(input@, i as int, byte),
            forall|j: int| start <= j < i ==> input@[j] != byte,
        decreases input.len() - i,
    {
        if input[i] == byte { return i; }
        i += 1;
    }
    i
}

pub open spec fn decimal_text(value: nat) -> Seq<u8>
    decreases value,
{
    if value < 10 { seq![(value + 48) as u8] }
    else { decimal_text(value / 10).push((value % 10 + 48) as u8) }
}

/// 递归按预期 u64 的十进制位数终止；不解析任意输入数字，也不截断或回绕。
pub fn decimal_bytes(value: u64) -> (bytes: Vec<u8>)
    ensures bytes@ == decimal_text(value as nat),
    decreases value,
{
    if value < 10 {
        vec![value as u8 + 48]
    } else {
        let mut bytes = decimal_bytes(value / 10);
        bytes.push((value % 10) as u8 + 48);
        bytes
    }
}

pub fn encode_path(path: &[u8]) -> (encoded: Vec<u8>)
    ensures encoded@ == dependency_escape(path@),
{
    let mut encoded = Vec::new();
    let mut i = 0;
    while i < path.len()
        invariant 0 <= i <= path.len(), encoded@ == dependency_escape(path@.take(i as int)),
        decreases path.len() - i,
    {
        if path[i] == 32 { encoded.push(92u8); }
        encoded.push(path[i]);
        assert(path@.take(i + 1) =~= path@.take(i as int).push(path[i as int]));
        assert(path@.take(i + 1).drop_last() =~= path@.take(i as int));
        i += 1;
    }
    assert(path@.take(i as int) =~= path@);
    encoded
}

pub open spec fn target_prefix(output: Seq<u8>) -> Seq<u8> {
    output + seq![46u8, 100u8, 58u8, 32u8]
}

pub open spec fn compiler_paths(input: Seq<u8>, output: Seq<u8>) -> Option<Seq<Seq<u8>>> {
    let prefix = target_prefix(output);
    if !prefix_at(input, 0, prefix) { None }
    else {
        let end = next_byte(input, prefix.len() as int, 10u8);
        if end < prefix.len() || end >= input.len() { None }
        else { dependency_scan(input.subrange(prefix.len() as int, end), Seq::empty(), Seq::empty()) }
    }
}

/// 读取首个编译目标的依赖列表；完整规则与 checksum 另由
/// matches_dependency_content 精确核对。
pub fn compiler_dependency_paths(input: &[u8], output: &[u8]) -> (parsed: Option<Vec<Vec<u8>>>)
    ensures
        match parsed {
            None => compiler_paths(input@, output@).is_none(),
            Some(paths) => compiler_paths(input@, output@) == Some(dependency_values(paths@))
                && paths.len() > 0
                && (forall|i: int| 0 <= i < paths.len() ==> dependency_word((#[trigger] paths[i])@)),
        },
{
    let mut prefix = Vec::new();
    append_bytes(&mut prefix, output);
    append_bytes(&mut prefix, &[46u8, 100u8, 58u8, 32u8]);
    assert(prefix@ =~= target_prefix(output@));
    if !matches_at(input, 0, prefix.as_slice()) { return None; }
    let end = find_byte(input, prefix.len(), 10u8);
    if end == input.len() { return None; }
    let list = copy_range(input, prefix.len(), end);
    assert(compiler_paths(input@, output@) == dependency_scan(list@, Seq::empty(), Seq::empty()));
    let parsed = parse_dependency_paths(list.as_slice());
    proof {
        if let Some(paths) = &parsed {
            assert forall|i: int| 0 <= i < paths.len() implies dependency_word((#[trigger] paths[i])@) by {
                assert(paths@[i]@ == dependency_values(paths@)[i]);
            }
        }
    }
    parsed
}

pub struct SourceBinding {
    pub path: Vec<u8>,
    pub sha256: Vec<u8>,
    pub length: u64,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
pub struct SourceRegistry {
    pub files: Vec<SourceFile>,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
pub struct SourceFile {
    pub path: String,
}

/// 适配器只提供解码、固定候选字节、原模式和摘要观察，不能在此接口内决定准入。
pub trait CandidateSourceRead {
    fn text(&self, path: &[u8]) -> Result<String, String>;
    fn mode(&self, path: &str) -> Option<String>;
    fn blob(&self, path: &str) -> Result<Vec<u8>, String>;
    fn digest(&self, bytes: &[u8]) -> Vec<u8>;
}

pub struct SourceAdmission {
    pub compiled: Vec<SourceBinding>,
    pub candidate: Vec<SourceBinding>,
    pub modes: Vec<String>,
}

pub open spec fn registered_source(files: Seq<SourceFile>, path: Seq<u8>) -> bool {
    exists|i: int| 0 <= i < files.len()
        && vstd::utf8::encode_utf8((#[trigger] files[i]).path@) == path
}

pub fn source_is_registered(files: &Vec<SourceFile>, path: &[u8]) -> (found: bool)
    ensures found == registered_source(files@, path@),
{
    let mut i = 0;
    while i < files.len()
        invariant 0 <= i <= files.len(),
            forall|j: int| 0 <= j < i ==>
                vstd::utf8::encode_utf8((#[trigger] files[j]).path@) != path@,
        decreases files.len() - i,
    {
        if equal_bytes(files[i].path.as_str().as_bytes(), path) { return true; }
        i += 1;
    }
    false
}

pub open spec fn regular_mode(mode: Seq<char>) -> bool {
    mode == "100644"@ || mode == "100755"@
}

pub open spec fn admission_matches(
    admission: &SourceAdmission, args: Seq<String>, dependency: Seq<u8>, files: Seq<SourceFile>,
) -> bool {
    super::verifier_arguments(args)
        && source_metadata_admitted(admission.compiled@, admission.candidate@,
            vstd::utf8::encode_utf8(args[8]@))
        && dependency_content_matches(dependency, vstd::utf8::encode_utf8(args[10]@), admission.compiled@)
        && admission.candidate.len() == admission.modes.len()
        && (forall|i: int| 0 <= i < admission.candidate.len() ==>
            registered_source(files, (#[trigger] admission.candidate[i]).path@)
                && regular_mode(admission.modes[i]@))
}

/// 真实本地判官和离线汇总共用此入口。所有路径、注册成员、模式、完整内容和
/// 失败分支都在此证明根内；固定候选的字节获取仍经现役 Git 观察接口。
pub fn bind_compiler_sources<I: CandidateSourceRead>(
    args: &Vec<String>,
    observed: &Option<Result<Vec<u8>, String>>, registry: &Result<SourceRegistry, String>, io: &I,
) -> (out: Result<Option<SourceAdmission>, String>)
    ensures match out {
        Ok(None) => observed.is_none(),
        Ok(Some(admission)) => match (registry, observed) {
                (Ok(registry), Some(Ok(bytes))) => admission_matches(&admission, args@, bytes@, registry.files@),
                _ => false,
            },
        Err(_) => true,
    },
{
    // 路由准入已证明合同与观察的有无一致；此层不再复制合同或按名称推断操作。
    if observed.is_none() { return Ok(None); }
    if !super::canonical_verifier_arguments(args) {
        return Err("E_VERUS_SOURCES: verifier arguments differ from the registered invocation".to_string());
    }
    let bytes = match observed {
        Some(Ok(bytes)) => bytes,
        _ => { return Err("E_VERUS_SOURCES: missing or failed original dependency observation".to_string()); },
    };
    let registry = match registry {
        Ok(value) => value,
        Err(error) => { return Err(error.clone()); },
    };
    let output = args[10].as_str();
    let paths = match compiler_dependency_paths(bytes.as_slice(), output.as_bytes()) {
        Some(paths) => paths,
        None => { return Err("E_VERUS_SOURCES: invalid compiler dependency target".to_string()); },
    };
    let mut compiled: Vec<SourceBinding> = Vec::new();
    let mut candidate: Vec<SourceBinding> = Vec::new();
    let mut modes: Vec<String> = Vec::new();
    let mut i = 0;
    while i < paths.len()
        invariant 0 <= i <= paths.len(),
            compiled.len() == i, candidate.len() == i, modes.len() == i,
            forall|j: int| 0 <= j < i ==>
                registered_source(registry.files@, (#[trigger] candidate[j]).path@)
                    && regular_mode(modes[j]@),
        decreases paths.len() - i,
    {
        let path = &paths[i];
        let normalized = match normalize_relative_path(path.as_slice()) {
            Some(path) => path,
            None => { return Err("E_VERUS_SOURCES: compiler source escapes the relative path domain".to_string()); },
        };
        let name = match io.text(normalized.as_slice()) {
            Ok(name) => name,
            Err(error) => { return Err(error); },
        };
        if !equal_bytes(name.as_str().as_bytes(), normalized.as_slice())
            || !source_is_registered(&registry.files, normalized.as_slice()) {
            return Err("E_VERUS_SOURCES: source is not an explicit candidate file".to_string());
        }
        let mode = match io.mode(name.as_str()) {
            Some(mode) => mode,
            None => { return Err("E_VERUS_SOURCES: source is absent from the fixed candidate".to_string()); },
        };
        if !super::identifiers::equal_text(mode.as_str(), "100644")
            && !super::identifiers::equal_text(mode.as_str(), "100755") {
            return Err("E_VERUS_SOURCES: candidate source is not a regular file".to_string());
        }
        let content = match io.blob(name.as_str()) {
            Ok(content) => content,
            Err(error) => { return Err(error); },
        };
        let digest = io.digest(content.as_slice());
        if !super::identifiers::valid_digest(digest.as_slice()) {
            return Err("E_VERUS_SOURCES: invalid observed source digest".to_string());
        }
        let length = content.len() as u64;
        let raw_path = copy_range(path.as_slice(), 0, path.len());
        let raw_digest = copy_range(digest.as_slice(), 0, digest.len());
        compiled.push(SourceBinding { path: raw_path, sha256: raw_digest, length });
        candidate.push(SourceBinding { path: normalized, sha256: digest, length });
        modes.push(mode);
        i += 1;
    }
    if !admit_source_metadata(&compiled, &candidate, args[8].as_str().as_bytes())
        || !matches_dependency_content(bytes.as_slice(), output.as_bytes(), &compiled) {
        return Err("E_VERUS_SOURCES: fixed source metadata or compiler checksums differ".to_string());
    }
    Ok(Some(SourceAdmission { compiled, candidate, modes }))
}

pub fn process_arguments(argv: &Vec<String>) -> (args: Result<Vec<String>, String>)
    ensures args.is_ok() == (argv.len() > 0),
        args.is_ok() ==> args.unwrap()@ == argv@.skip(1),
{
    if argv.len() == 0 {
        return Err("E_VERUS_SOURCES: original process argv is empty".to_string());
    }
    let mut args = Vec::new();
    let mut i = 1;
    while i < argv.len()
        invariant 1 <= i <= argv.len(), args@ == argv@.subrange(1, i as int),
        decreases argv.len() - i,
    {
        args.push(argv[i].clone());
        assert(argv@.subrange(1, i + 1) =~= argv@.subrange(1, i as int).push(argv[i as int]));
        i += 1;
    }
    Ok(args)
}

pub fn admission_result(admission: Result<Option<SourceAdmission>, String>) -> (out: Result<(), String>)
    ensures out.is_ok() == admission.is_ok(),
        match (out, admission) {
            (Err(first), Err(second)) => first@ == second@,
            _ => true,
        },
{
    match admission { Ok(_) => Ok(()), Err(error) => Err(error) }
}

/// 只对同一原回执的成功操作绑定源码；原失败、阻断及其原件交回既有结果生产者。
pub fn bind_process_sources<I: CandidateSourceRead>(
    state: &str, receipt: Option<(&Vec<String>, &Option<Result<Vec<u8>, String>>)>,
    registry: &Result<SourceRegistry, String>, io: &I,
) -> (out: Result<(), String>)
    ensures state@ != "passed"@ ==> out.is_ok(),
        state@ == "passed"@ && receipt.is_none() ==> out.is_err(),
        state@ == "passed"@ && out.is_ok() ==> match receipt {
            Some((argv, observed)) => argv.len() > 0 && match observed {
                None => true,
                Some(Ok(bytes)) => registry.is_ok() && exists|admission: SourceAdmission|
                    admission_matches(&admission, argv@.skip(1), bytes@, registry.unwrap().files@),
                Some(Err(_)) => false,
            },
            None => false,
        },
{
    if !super::identifiers::equal_text(state, "passed") { return Ok(()); }
    let (argv, observed) = match receipt {
        Some(value) => value,
        None => { return Err("E_VERUS_SOURCES: successful operation has no original receipt".to_string()); },
    };
    let args = match process_arguments(argv) {
        Ok(args) => args,
        Err(error) => { return Err(error); },
    };
    admission_result(bind_compiler_sources(&args, observed, registry, io))
}

pub open spec fn source_pair_matches(source: &SourceBinding, candidate: &SourceBinding) -> bool {
    relative_components(source.path@).is_some()
        && candidate.path@ == joined_components(relative_components(source.path@).unwrap())
        && super::identifiers::digest_format(candidate.sha256@)
        && source.sha256@ == candidate.sha256@
        && source.length == candidate.length
}

pub fn matches_source_pair(source: &SourceBinding, candidate: &SourceBinding) -> (matched: bool)
    ensures matched <==> source_pair_matches(source, candidate),
{
    let Some(path) = normalize_relative_path(source.path.as_slice()) else { return false; };
    equal_bytes(path.as_slice(), candidate.path.as_slice())
        && super::identifiers::valid_digest(candidate.sha256.as_slice())
        && equal_bytes(source.sha256.as_slice(), candidate.sha256.as_slice())
        && source.length == candidate.length
}

pub open spec fn listed_source_path(sources: Seq<SourceBinding>, path: Seq<u8>, limit: int) -> bool {
    exists|i: int| 0 <= i < limit && (#[trigger] sources[i]).path@ == path
}

pub fn contains_source_path(sources: &Vec<SourceBinding>, path: &[u8], limit: usize) -> (found: bool)
    requires limit <= sources.len(),
    ensures found <==> listed_source_path(sources@, path@, limit as int),
{
    let mut i = 0;
    while i < limit
        invariant 0 <= i <= limit <= sources.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] sources[j]).path@ != path@,
        decreases limit - i,
    {
        if equal_bytes(sources[i].path.as_slice(), path) { return true; }
        i += 1;
    }
    false
}

pub open spec fn source_aliases_consistent(sources: Seq<SourceBinding>, index: int) -> bool {
    forall|j: int| 0 <= j < index && (#[trigger] sources[j]).path@ == sources[index].path@
        ==> sources[j].sha256@ == sources[index].sha256@
            && sources[j].length == sources[index].length
}

pub fn consistent_source_aliases(sources: &Vec<SourceBinding>, index: usize) -> (consistent: bool)
    requires index < sources.len(),
    ensures consistent <==> source_aliases_consistent(sources@, index as int),
{
    let mut j = 0;
    while j < index
        invariant 0 <= j <= index < sources.len(),
            forall|k: int| 0 <= k < j && (#[trigger] sources[k]).path@ == sources[index as int].path@
                ==> sources[k].sha256@ == sources[index as int].sha256@
                    && sources[k].length == sources[index as int].length,
        decreases index - j,
    {
        if equal_bytes(sources[j].path.as_slice(), sources[index].path.as_slice())
            && (!equal_bytes(sources[j].sha256.as_slice(), sources[index].sha256.as_slice())
                || sources[j].length != sources[index].length) { return false; }
        j += 1;
    }
    true
}

pub open spec fn source_metadata_admitted(
    sources: Seq<SourceBinding>, candidate: Seq<SourceBinding>, root: Seq<u8>,
) -> bool {
    sources.len() > 0 && sources.len() == candidate.len()
        && (forall|i: int| 0 <= i < sources.len() ==>
            source_pair_matches(#[trigger] &sources[i], &candidate[i]))
        && (forall|i: int| 0 <= i < candidate.len() ==>
            #[trigger] source_aliases_consistent(candidate, i))
        && listed_source_path(candidate, root, candidate.len() as int)
}

/// 每个原编译器路径逐项对应候选条目，同一规范路径的别名须有相同摘要和长度；
/// 完整根路径必须出现。调用方须从固定登记／Git 取得预期条目。
pub fn admit_source_metadata(
    sources: &Vec<SourceBinding>, candidate: &Vec<SourceBinding>, root: &[u8],
) -> (accepted: bool)
    ensures accepted <==> source_metadata_admitted(sources@, candidate@, root@),
{
    if sources.len() == 0 || sources.len() != candidate.len() { return false; }
    let mut i = 0;
    while i < sources.len()
        invariant 0 <= i <= sources.len(), 0 < sources.len() == candidate.len(),
            forall|j: int| 0 <= j < i ==>
                source_pair_matches(#[trigger] &sources[j], &candidate[j]),
            forall|j: int| 0 <= j < i ==>
                #[trigger] source_aliases_consistent(candidate@, j),
        decreases sources.len() - i,
    {
        if !matches_source_pair(&sources[i], &candidate[i]) { return false; }
        if !consistent_source_aliases(candidate, i) { return false; }
        i += 1;
    }
    contains_source_path(candidate, root, candidate.len())
}

pub open spec fn checksum_prefix(source: &SourceBinding) -> Seq<u8> {
    vstd::utf8::encode_utf8("# checksum:sha256="@) + source.sha256@ + vstd::utf8::encode_utf8(" file_len:"@)
}

pub open spec fn checksum_line(source: &SourceBinding) -> Seq<u8> {
    checksum_prefix(source) + decimal_text(source.length as nat)
        + seq![32u8] + dependency_escape(source.path@)
}

pub fn matches_checksum(line: &[u8], source: &SourceBinding) -> (matched: bool)
    ensures matched <==> line@ == checksum_line(source),
{
    let mut prefix = Vec::new();
    append_bytes(&mut prefix, "# checksum:sha256=".as_bytes());
    append_bytes(&mut prefix, source.sha256.as_slice());
    append_bytes(&mut prefix, " file_len:".as_bytes());
    let length = decimal_bytes(source.length);
    append_bytes(&mut prefix, length.as_slice());
    prefix.push(32u8);
    assert(prefix@ =~= checksum_prefix(source) + decimal_text(source.length as nat) + seq![32u8]);
    let path = encode_path(source.path.as_slice());
    append_bytes(&mut prefix, path.as_slice());
    assert(prefix@ =~= checksum_line(source));
    equal_bytes(line, prefix.as_slice())
}

pub open spec fn source_paths(sources: Seq<SourceBinding>) -> Seq<Seq<u8>> {
    sources.map(|i: int, source: SourceBinding| source.path@)
}

pub open spec fn phony_rules(sources: Seq<SourceBinding>) -> Seq<u8>
    decreases sources.len(),
{
    if sources.len() == 0 { Seq::empty() }
    else { phony_rules(sources.drop_last()) + dependency_escape(sources.last().path@) + seq![58u8, 10u8] }
}

pub open spec fn dependency_header(output: Seq<u8>, sources: Seq<SourceBinding>) -> Seq<u8> {
    target_prefix(output) + dependency_encoding(source_paths(sources)) + seq![10u8, 10u8]
        + output + seq![58u8, 32u8] + dependency_encoding(source_paths(sources)) + seq![10u8, 10u8]
        + phony_rules(sources)
}

pub fn dependency_header_bytes(output: &[u8], sources: &Vec<SourceBinding>) -> (header: Vec<u8>)
    ensures header@ == dependency_header(output@, sources@),
{
    let mut list = Vec::new();
    let mut i = 0;
    assert(source_paths(sources@.take(0)) =~= Seq::<Seq<u8>>::empty());
    while i < sources.len()
        invariant 0 <= i <= sources.len(),
            list@ == dependency_encoding(source_paths(sources@.take(i as int))),
        decreases sources.len() - i,
    {
        let ghost previous = source_paths(sources@.take(i as int));
        let path = encode_path(sources[i].path.as_slice());
        if i > 0 { list.push(32u8); }
        append_bytes(&mut list, path.as_slice());
        assert(source_paths(sources@.take(i + 1)) =~= previous.push(sources@[i as int].path@));
        assert(previous.push(sources@[i as int].path@).drop_last() =~= previous);
        i += 1;
    }
    assert(sources@.take(i as int) =~= sources@);
    let mut header = Vec::new();
    append_bytes(&mut header, output);
    append_bytes(&mut header, &[46u8, 100u8, 58u8, 32u8]);
    append_bytes(&mut header, list.as_slice());
    append_bytes(&mut header, &[10u8, 10u8]);
    append_bytes(&mut header, output);
    append_bytes(&mut header, &[58u8, 32u8]);
    append_bytes(&mut header, list.as_slice());
    append_bytes(&mut header, &[10u8, 10u8]);
    let ghost targets = header@;
    assert(phony_rules(sources@.take(0)) == Seq::<u8>::empty());
    i = 0;
    while i < sources.len()
        invariant 0 <= i <= sources.len(), header@ == targets + phony_rules(sources@.take(i as int)),
        decreases sources.len() - i,
    {
        let path = encode_path(sources[i].path.as_slice());
        append_bytes(&mut header, path.as_slice());
        append_bytes(&mut header, &[58u8, 10u8]);
        assert(sources@.take(i + 1).drop_last() =~= sources@.take(i as int));
        i += 1;
    }
    assert(sources@.take(i as int) =~= sources@);
    header
}

pub open spec fn checksum_trailer(
    input: Seq<u8>, sources: Seq<SourceBinding>, index: int, before_checksums: bool,
) -> bool
    recommends 0 <= index <= sources.len(),
    decreases input.len(),
{
    if input.len() == 0 { index == sources.len() }
    else {
        let end = next_byte(input, 0, 10u8);
        if end < 0 || end >= input.len() { false }
        else {
            let line = input.take(end);
            let remaining = input.skip(end + 1);
            if index < sources.len() {
                if before_checksums && (line.len() == 0
                    || prefix_at(line, 0, vstd::utf8::encode_utf8("# env-dep:"@))) {
                    checksum_trailer(remaining, sources, index, true)
                } else {
                    line == checksum_line(&sources[index])
                        && checksum_trailer(remaining, sources, index + 1, false)
                }
            } else {
                line.len() == 0 && checksum_trailer(remaining, sources, index, false)
            }
        }
    }
}

pub open spec fn dependency_content_matches(
    input: Seq<u8>, output: Seq<u8>, sources: Seq<SourceBinding>,
) -> bool {
    let header = dependency_header(output, sources);
    sources.len() > 0 && prefix_at(input, 0, header)
        && checksum_trailer(input.skip(header.len() as int), sources, 0, true)
}

/// 按预期源码元数据精确核对完整有序内容；调用方仍须把每个预期路径、摘要和
/// 长度绑定到固定源码字节，单独通过此函数不建立该来源关系。
pub fn matches_dependency_content(input: &[u8], output: &[u8], sources: &Vec<SourceBinding>) -> (matched: bool)
    ensures matched <==> dependency_content_matches(input@, output@, sources@),
{
    if sources.len() == 0 { return false; }
    let header = dependency_header_bytes(output, sources);
    if !matches_at(input, 0, header.as_slice()) { return false; }
    let mut position = header.len();
    let mut index = 0;
    let mut before_checksums = true;
    assert(input@.skip(position as int) == input@.skip(header.len() as int));
    while position < input.len()
        invariant
            header.len() <= position <= input.len(), 0 <= index <= sources.len(),
            sources.len() > 0, prefix_at(input@, 0, header@),
            header@ == dependency_header(output@, sources@),
            checksum_trailer(input@.skip(header.len() as int), sources@, 0, true)
                == checksum_trailer(input@.skip(position as int), sources@, index as int, before_checksums),
        decreases input.len() - position,
    {
        let end = find_byte(input, position, 10u8);
        proof { next_byte_shift(input@, position as int, 10u8); }
        if end == input.len() {
            assert(!checksum_trailer(input@.skip(position as int), sources@, index as int, before_checksums));
            return false;
        }
        let line = copy_range(input, position, end);
        assert(line@ =~= input@.skip(position as int).take(end - position));
        assert(input@.skip(end + 1) =~= input@.skip(position as int).skip(end - position + 1));
        if index < sources.len() {
            if before_checksums && (line.len() == 0
                || matches_at(line.as_slice(), 0, "# env-dep:".as_bytes())) {
            } else {
                if !matches_checksum(line.as_slice(), &sources[index]) {
                    assert(!checksum_trailer(input@.skip(position as int), sources@, index as int, before_checksums));
                    return false;
                }
                index += 1;
                before_checksums = false;
            }
        } else {
            if line.len() != 0 { return false; }
            before_checksums = false;
        }
        position = end + 1;
    }
    assert(input@.skip(position as int) =~= Seq::<u8>::empty());
    index == sources.len()
}

pub proof fn next_byte_shift(input: Seq<u8>, start: int, byte: u8)
    requires 0 <= start <= input.len(),
    ensures next_byte(input.skip(start), 0, byte) == next_byte(input, start, byte) - start,
    decreases input.len() - start,
{
    next_byte_suffix(input, start, 0, byte);
}

proof fn next_byte_suffix(input: Seq<u8>, start: int, offset: int, byte: u8)
    requires 0 <= start <= input.len(), 0 <= offset <= input.len() - start,
    ensures next_byte(input.skip(start), offset, byte) == next_byte(input, start + offset, byte) - start,
    decreases input.len() - start - offset,
{
    if start + offset < input.len() && input[start + offset] != byte {
        next_byte_suffix(input, start, offset + 1, byte);
    }
}

pub proof fn next_byte_at(input: Seq<u8>, start: int, end: int, byte: u8)
    requires 0 <= start <= end <= input.len(),
        end == input.len() || input[end] == byte,
        forall|i: int| start <= i < end ==> input[i] != byte,
    ensures next_byte(input, start, byte) == end,
    decreases end - start,
{
    if start < end { next_byte_at(input, start + 1, end, byte); }
}


}
