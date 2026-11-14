//! 显式 Verus 输出合同、规范参数及原执行结果准入。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;
use identifiers::{equal_bytes, equal_text};

#[path = "verified_cargo.rs"]
pub mod cargo;
#[path = "verified_sources.rs"]
pub mod sources;

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct OutputContract {
    pub version: String,
    pub commit: String,
}

impl Clone for OutputContract {
    fn clone(&self) -> (out: Self)
        ensures out.version == self.version, out.commit == self.commit,
    { Self { version: self.version.clone(), commit: self.commit.clone() } }
}

pub open spec fn same_contract(first: &OutputContract, second: &OutputContract) -> bool {
    first.version@ == second.version@ && first.commit@ == second.commit@
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for OutputContract {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { same_contract(self, other) }
}

impl PartialEq for OutputContract {
    fn eq(&self, other: &Self) -> (same: bool)
        ensures same == same_contract(self, other),
    {
        equal_text(self.version.as_str(), other.version.as_str())
            && equal_text(self.commit.as_str(), other.commit.as_str())
    }
}

impl Eq for OutputContract {}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct RegisteredAction {
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    #[cfg_attr(not(verus_keep_ghost), serde(default))]
    pub output_contract: Option<OutputContract>,
}

/// 字段名和必需字段由外部 Serde 解码；这些真实类型也由证明根拥有。
#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
pub struct CompilerReport {
    pub verus: CompilerVersion,
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "verification-results"))]
    pub verification: Verification,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
pub struct CompilerVersion {
    pub version: String,
    pub commit: String,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub struct Verification {
    pub encountered_error: bool,
    pub encountered_vir_error: bool,
    pub success: bool,
    pub verified: u64,
    pub errors: u64,
    pub is_verifying_entire_crate: bool,
}

/// 原编译进程结束后取得的库文件身份；不在回执中复制库内容。
#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct CompiledArtifact {
    pub path: Vec<u8>,
    pub sha256: Vec<u8>,
    pub length: u64,
}

impl Clone for CompiledArtifact {
    fn clone(&self) -> (out: Self)
        ensures out.path@ == self.path@, out.sha256@ == self.sha256@,
            out.length == self.length,
    {
        Self { path: self.path.clone(), sha256: self.sha256.clone(), length: self.length }
    }
}

/// 外部文件／SHA-256 接口返回点名普通文件的实际摘要与长度或原错误。
pub trait ArtifactRead {
    fn identity(&self, path: &[u8]) -> Result<(Vec<u8>, u64), String>;
}

pub open spec fn artifact_matches(artifact: &CompiledArtifact, args: Seq<String>) -> bool {
    verifier_arguments(args)
        && artifact.path@ == vstd::utf8::encode_utf8(args[invocation_output(args)]@)
        && identifiers::digest_format(artifact.sha256@) && artifact.length > 0
}

/// 失败进程与非规范参数不读取库；原 IO 错误仍进入同一回执。
pub fn observe_artifact<I: ArtifactRead>(
    contract: &Option<OutputContract>, args: &Vec<String>,
    exit: i32, process_failed: bool, io: &I,
) -> (observed: Option<Result<CompiledArtifact, String>>)
    ensures observed.is_none() == contract.is_none(),
        match observed {
            Some(Ok(artifact)) => exit == 0 && !process_failed && artifact_matches(&artifact, args@),
            _ => true,
        },
{
    if contract.is_none() { return None; }
    if exit != 0 || process_failed {
        return Some(Err("E_VERUS_ARTIFACT: original compiler process was not successful".to_string()));
    }
    let parsed = match parse_verifier_arguments(args) {
        Some(parsed) => parsed,
        None => { return Some(Err("E_VERUS_ARTIFACT: original compiler process was not successful".to_string())); },
    };
    let (sha256, length) = match io.identity(args[parsed.output].as_str().as_bytes()) {
        Ok(value) => value,
        Err(error) => { return Some(Err(error)); },
    };
    if length == 0 || !identifiers::valid_digest(sha256.as_slice()) {
        return Some(Err("E_VERUS_ARTIFACT: empty library or invalid observed digest".to_string()));
    }
    let mut path = Vec::new();
    append_bytes(&mut path, args[parsed.output].as_str().as_bytes());
    Some(Ok(CompiledArtifact { path, sha256, length }))
}

pub open spec fn artifact_observed(
    contract: &Option<OutputContract>, observed: &Option<Result<CompiledArtifact, String>>,
    args: Seq<String>,
) -> bool {
    match contract {
        None => observed.is_none(),
        Some(_) => match observed {
            Some(Ok(artifact)) => artifact_matches(artifact, args),
            _ => false,
        },
    }
}

pub fn admit_artifact_observation(
    contract: &Option<OutputContract>, observed: &Option<Result<CompiledArtifact, String>>,
    args: &Vec<String>,
) -> (out: Result<(), String>)
    ensures out.is_ok() == artifact_observed(contract, observed, args@),
{
    if contract.is_none() {
        return if observed.is_none() { Ok(()) }
            else { Err("E_VERUS_ARTIFACT: ordinary operation has compiler library evidence".to_string()) };
    }
    let parsed = match parse_verifier_arguments(args) {
        Some(parsed) => parsed,
        None => { return Err("E_VERUS_ARTIFACT: verifier arguments differ from the registered invocation".to_string()); },
    };
    let artifact = match observed {
        Some(Ok(artifact)) => artifact,
        _ => { return Err("E_VERUS_ARTIFACT: missing or failed original library observation".to_string()); },
    };
    if !equal_bytes(artifact.path.as_slice(), args[parsed.output].as_str().as_bytes())
        || !identifiers::valid_digest(artifact.sha256.as_slice()) || artifact.length == 0 {
        return Err("E_VERUS_ARTIFACT: library path, digest or length rejected".to_string());
    }
    Ok(())
}

pub open spec fn valid_contract(contract: &Option<OutputContract>) -> bool {
    match contract {
        None => true,
        Some(value) => value.version@.len() > 0 && value.commit@.len() > 0,
    }
}

pub fn validate_contract(contract: &Option<OutputContract>) -> (out: Result<(), String>)
    ensures out.is_ok() == valid_contract(contract),
        out.is_err() ==> out.unwrap_err()@ == "E_VERUS_CONTRACT: version and commit must be nonempty"@,
{
    match contract {
        None => Ok(()),
        Some(value) => {
            if value.version.as_str().is_empty() || value.commit.as_str().is_empty() {
                Err("E_VERUS_CONTRACT: version and commit must be nonempty".to_string())
            } else { Ok(()) }
        },
    }
}

/// 普通操作不解析其业务输出；只向解码接口传空输入。
pub fn proof_stdout<'a>(contract: &Option<OutputContract>, raw: &'a [u8]) -> (out: &'a [u8])
    ensures out@ == if contract.is_some() { raw@ } else { Seq::<u8>::empty() },
{
    match contract { None => &[], Some(_) => raw }
}

pub proof fn encode_concat(first: Seq<char>, second: Seq<char>)
    ensures vstd::utf8::encode_utf8(first + second)
        == vstd::utf8::encode_utf8(first) + vstd::utf8::encode_utf8(second),
    decreases first.len(),
{
    if first.len() > 0 {
        assert((first + second).drop_first() =~= first.drop_first() + second);
        encode_concat(first.drop_first(), second);
    }
}

fn append_bytes(bytes: &mut Vec<u8>, suffix: &[u8])
    ensures bytes@ == old(bytes)@ + suffix@,
{
    let ghost before = bytes@;
    let mut i = 0;
    while i < suffix.len()
        invariant 0 <= i <= suffix.len(), bytes@ == before + suffix@.take(i as int),
        decreases suffix.len() - i,
    {
        bytes.push(suffix[i]);
        assert(suffix@.take(i + 1) =~= suffix@.take(i as int).push(suffix[i as int]));
        i += 1;
    }
    assert(suffix@.take(i as int) =~= suffix@);
}

pub fn compiler_outputs(output: &str) -> (argument: Vec<u8>)
    ensures argument@ == vstd::utf8::encode_utf8(
        "--emit=link="@ + output@ + ",dep-info="@ + output@ + ".d"@),
{
    let mut argument = Vec::new();
    append_bytes(&mut argument, "--emit=link=".as_bytes());
    append_bytes(&mut argument, output.as_bytes());
    append_bytes(&mut argument, ",dep-info=".as_bytes());
    append_bytes(&mut argument, output.as_bytes());
    append_bytes(&mut argument, ".d".as_bytes());
    proof {
        encode_concat("--emit=link="@, output@);
        encode_concat("--emit=link="@ + output@, ",dep-info="@);
        encode_concat("--emit=link="@ + output@ + ",dep-info="@, output@);
        encode_concat("--emit=link="@ + output@ + ",dep-info="@ + output@, ".d"@);
    }
    argument
}

/// 当前登记语法的选项种类；不解释任意 rustc／Verus flag。
pub enum InvocationKind { Edition, CrateName, Unstable, Environment, Search, Extern, Import, Export, Unknown }

pub open spec fn invocation_kind(token: Seq<char>) -> InvocationKind {
    if token == "--edition=2024"@ { InvocationKind::Edition }
    else if token == "--crate-name"@ { InvocationKind::CrateName }
    else if token == "-Zunstable-options"@ { InvocationKind::Unstable }
    else if token == "--env-set"@ { InvocationKind::Environment }
    else if token == "-L"@ { InvocationKind::Search }
    else if token == "--extern"@ { InvocationKind::Extern }
    else if token == "--import"@ { InvocationKind::Import }
    else if token == "--export"@ { InvocationKind::Export }
    else { InvocationKind::Unknown }
}

pub fn classify_invocation_token(token: &str) -> (kind: InvocationKind)
    ensures kind == invocation_kind(token@),
{
    if equal_text(token, "--edition=2024") { InvocationKind::Edition }
    else if equal_text(token, "--crate-name") { InvocationKind::CrateName }
    else if equal_text(token, "-Zunstable-options") { InvocationKind::Unstable }
    else if equal_text(token, "--env-set") { InvocationKind::Environment }
    else if equal_text(token, "-L") { InvocationKind::Search }
    else if equal_text(token, "--extern") { InvocationKind::Extern }
    else if equal_text(token, "--import") { InvocationKind::Import }
    else if equal_text(token, "--export") { InvocationKind::Export }
    else { InvocationKind::Unknown }
}

pub open spec fn option_width(kind: InvocationKind) -> nat {
    if matches!(kind, InvocationKind::Edition | InvocationKind::Unstable) { 1 } else { 2 }
}

pub fn invocation_option_width(kind: &InvocationKind) -> (width: usize)
    ensures width == option_width(*kind),
{
    if matches!(kind, InvocationKind::Edition | InvocationKind::Unstable) { 1 } else { 2 }
}

pub open spec fn identifier_byte(byte: u8, first: bool) -> bool {
    byte == 95 || 65 <= byte <= 90 || 97 <= byte <= 122 || (!first && 48 <= byte <= 57)
}

pub open spec fn invocation_identifier(value: Seq<u8>) -> bool {
    value.len() > 0 && forall|i: int| 0 <= i < value.len() ==> identifier_byte(#[trigger] value[i], i == 0)
}

pub fn valid_invocation_identifier(value: &[u8]) -> (valid: bool)
    ensures valid == invocation_identifier(value@),
{
    if value.len() == 0 { return false; }
    let mut i = 0;
    while i < value.len()
        invariant 0 <= i <= value.len(), value.len() > 0,
            forall|j: int| 0 <= j < i ==> identifier_byte(#[trigger] value[j], j == 0),
        decreases value.len() - i,
    {
        let byte = value[i];
        if !(byte == 95 || 65 <= byte && byte <= 90 || 97 <= byte && byte <= 122
            || i != 0 && 48 <= byte && byte <= 57) { return false; }
        i += 1;
    }
    true
}

pub open spec fn invocation_value(value: Seq<u8>) -> bool {
    value.len() > 0 && value[0] != 45
        && forall|i: int| 0 <= i < value.len() ==> 32 <= (#[trigger] value[i]) && value[i] != 127
}

pub fn valid_invocation_value(value: &[u8]) -> (valid: bool)
    ensures valid == invocation_value(value@),
{
    if value.len() == 0 || value[0] == 45 { return false; }
    let mut i = 0;
    while i < value.len()
        invariant 0 <= i <= value.len(), value.len() > 0, value[0] != 45,
            forall|j: int| 0 <= j < i ==> 32 <= (#[trigger] value[j]) && value[j] != 127,
        decreases value.len() - i,
    {
        if value[i] < 32 || value[i] == 127 { return false; }
        i += 1;
    }
    true
}

pub open spec fn named_invocation_value(value: Seq<u8>) -> bool {
    let split = sources::next_byte(value, 0, 61);
    0 < split < value.len() && invocation_identifier(value.take(split))
        && invocation_value(value.skip(split + 1))
}

pub fn valid_named_invocation_value(value: &[u8]) -> (valid: bool)
    ensures valid == named_invocation_value(value@),
{
    let split = sources::find_byte(value, 0, 61);
    if split == 0 || split == value.len() { return false; }
    let name = sources::copy_range(value, 0, split);
    let path = sources::copy_range(value, split + 1, value.len());
    valid_invocation_identifier(name.as_slice()) && valid_invocation_value(path.as_slice())
}

pub open spec fn same_input_name(first: Seq<u8>, second: Seq<u8>) -> bool {
    first.take(sources::next_byte(first, 0, 61)) == second.take(sources::next_byte(second, 0, 61))
}

pub fn same_invocation_input_name(first: &[u8], second: &[u8]) -> (same: bool)
    ensures same == same_input_name(first@, second@),
{
    let first_end = sources::find_byte(first, 0, 61);
    let second_end = sources::find_byte(second, 0, 61);
    let first_name = sources::copy_range(first, 0, first_end);
    let second_name = sources::copy_range(second, 0, second_end);
    equal_bytes(first_name.as_slice(), second_name.as_slice())
}

pub open spec fn extension_value_valid(args: Seq<String>, i: int, end: int) -> bool {
    7 <= i < end <= args.len() && {
        let kind = invocation_kind(args[i]@);
        !matches!(kind, InvocationKind::Unknown) && i + option_width(kind) <= end
            && match kind {
                InvocationKind::Edition | InvocationKind::Unstable => true,
                InvocationKind::CrateName => invocation_identifier(vstd::utf8::encode_utf8(args[i + 1]@)),
                InvocationKind::Environment | InvocationKind::Extern | InvocationKind::Import =>
                    named_invocation_value(vstd::utf8::encode_utf8(args[i + 1]@)),
                InvocationKind::Search => {
                    let value = vstd::utf8::encode_utf8(args[i + 1]@);
                    sources::prefix_at(value, 0, b"dependency="@)
                        && invocation_value(value.skip(11))
                },
                InvocationKind::Export => invocation_value(vstd::utf8::encode_utf8(args[i + 1]@)),
                InvocationKind::Unknown => false,
            }
    }
}

pub fn valid_extension_value(args: &Vec<String>, i: usize, end: usize) -> (valid: bool)
    requires 7 <= i < end <= args.len(),
    ensures valid == extension_value_valid(args@, i as int, end as int),
{
    let kind = classify_invocation_token(args[i].as_str());
    let width = invocation_option_width(&kind);
    if width > end - i { return false; }
    match kind {
        InvocationKind::Edition | InvocationKind::Unstable => true,
        InvocationKind::CrateName => valid_invocation_identifier(args[i + 1].as_str().as_bytes()),
        InvocationKind::Environment | InvocationKind::Extern | InvocationKind::Import =>
            valid_named_invocation_value(args[i + 1].as_str().as_bytes()),
        InvocationKind::Search => {
            let value = args[i + 1].as_str().as_bytes();
            if !sources::matches_at(value, 0, b"dependency=") { return false; }
            let path = sources::copy_range(value, 11, value.len());
            valid_invocation_value(path.as_slice())
        },
        InvocationKind::Export => valid_invocation_value(args[i + 1].as_str().as_bytes()),
        InvocationKind::Unknown => false,
    }
}

pub open spec fn conflicting_extension(args: Seq<String>, first: int, second: int) -> bool {
    let kind = invocation_kind(args[first]@);
    kind == invocation_kind(args[second]@) && match kind {
        InvocationKind::Edition | InvocationKind::CrateName | InvocationKind::Unstable | InvocationKind::Export => true,
        InvocationKind::Environment | InvocationKind::Extern | InvocationKind::Import =>
            same_input_name(vstd::utf8::encode_utf8(args[first + 1]@), vstd::utf8::encode_utf8(args[second + 1]@)),
        InvocationKind::Search => args[first + 1]@ == args[second + 1]@,
        InvocationKind::Unknown => false,
    }
}

pub fn extensions_conflict(args: &Vec<String>, first: usize, second: usize) -> (conflict: bool)
    requires first + 1 < args.len(), second + 1 < args.len(),
    ensures conflict == conflicting_extension(args@, first as int, second as int),
{
    let kind = classify_invocation_token(args[first].as_str());
    let other = classify_invocation_token(args[second].as_str());
    if !same_invocation_kind(&kind, &other) { return false; }
    match kind {
        InvocationKind::Edition | InvocationKind::CrateName | InvocationKind::Unstable | InvocationKind::Export => true,
        InvocationKind::Environment | InvocationKind::Extern | InvocationKind::Import =>
            same_invocation_input_name(args[first + 1].as_str().as_bytes(), args[second + 1].as_str().as_bytes()),
        InvocationKind::Search => equal_text(args[first + 1].as_str(), args[second + 1].as_str()),
        InvocationKind::Unknown => false,
    }
}

fn same_invocation_kind(first: &InvocationKind, second: &InvocationKind) -> (same: bool)
    ensures same == (*first == *second),
{
    match (first, second) {
        (InvocationKind::Edition, InvocationKind::Edition)
        | (InvocationKind::CrateName, InvocationKind::CrateName)
        | (InvocationKind::Unstable, InvocationKind::Unstable)
        | (InvocationKind::Environment, InvocationKind::Environment)
        | (InvocationKind::Search, InvocationKind::Search)
        | (InvocationKind::Extern, InvocationKind::Extern)
        | (InvocationKind::Import, InvocationKind::Import)
        | (InvocationKind::Export, InvocationKind::Export)
        | (InvocationKind::Unknown, InvocationKind::Unknown) => true,
        _ => false,
    }
}

pub open spec fn fresh_extension(args: Seq<String>, i: int) -> bool {
    forall|j: int| 7 <= j < i ==> !conflicting_extension(args, j, i)
}

pub fn extension_is_fresh(args: &Vec<String>, i: usize) -> (fresh: bool)
    requires 7 <= i, i + 1 < args.len(),
    ensures fresh == fresh_extension(args@, i as int),
{
    let mut j = 7;
    while j < i
        invariant 7 <= j <= i, i + 1 < args.len(),
            forall|k: int| 7 <= k < j ==> !conflicting_extension(args@, k, i as int),
        decreases i - j,
    {
        if extensions_conflict(args, j, i) { return false; }
        j += 1;
    }
    true
}

pub open spec fn import_has_extern(args: Seq<String>, i: int, end: int) -> bool {
    !matches!(invocation_kind(args[i]@), InvocationKind::Import)
        || exists|j: int| 7 <= j < end && args[j]@ == "--extern"@
            && j + 1 < end && same_input_name(vstd::utf8::encode_utf8(args[j + 1]@), vstd::utf8::encode_utf8(args[i + 1]@))
}

pub fn paired_import(args: &Vec<String>, i: usize, end: usize) -> (paired: bool)
    requires 7 <= i < end < args.len(), i + 1 < args.len(),
    ensures paired == import_has_extern(args@, i as int, end as int),
{
    if !matches!(classify_invocation_token(args[i].as_str()), InvocationKind::Import) { return true; }
    let mut j = 7;
    while j < end
        invariant 7 <= j <= end < args.len(), i + 1 < args.len(),
            matches!(invocation_kind(args[i]@), InvocationKind::Import),
            forall|k: int| 7 <= k < j && args[k]@ == "--extern"@ && k + 1 < end ==>
                !same_input_name(vstd::utf8::encode_utf8(args[k + 1]@), vstd::utf8::encode_utf8(args[i + 1]@)),
        decreases end - j,
    {
        if equal_text(args[j].as_str(), "--extern") && j + 1 < end
            && same_invocation_input_name(args[j + 1].as_str().as_bytes(), args[i + 1].as_str().as_bytes()) {
            return true;
        }
        j += 1;
    }
    false
}

pub open spec fn invocation_extensions(args: Seq<String>, i: int, end: int) -> bool
    decreases if end >= i { end - i } else { 0int },
{
    if i == end { true }
    else if i < 7 || i > end || end >= args.len() { false }
    else {
        extension_value_valid(args, i, end) && fresh_extension(args, i)
            && import_has_extern(args, i, end)
            && invocation_extensions(args, i + option_width(invocation_kind(args[i]@)), end)
    }
}

/// 投影只由这个语法所有者定义；使用方不得自行定位 argv 槽位。
pub open spec fn invocation_source(args: Seq<String>) -> int { args.len() - 4 }
pub open spec fn invocation_output(args: Seq<String>) -> int { args.len() - 2 }
pub open spec fn invocation_emit(args: Seq<String>) -> int { args.len() - 5 }

pub open spec fn verifier_arguments(args: Seq<String>) -> bool {
    args.len() >= 12
        && args[0]@ == "--crate-type=lib"@
        && args[1]@ == "--no-cheating"@
        && args[2]@ == "--no-external-by-default"@
        && args[3]@ == "--num-threads"@ && args[4]@ == "2"@
        && args[5]@ == "--output-json"@ && args[6]@ == "--compile"@
        && invocation_extensions(args, 7, invocation_emit(args))
        && args[invocation_emit(args)]@ == "--emit=link="@ + args[invocation_output(args)]@
            + ",dep-info="@ + args[invocation_output(args)]@ + ".d"@
        && invocation_value(vstd::utf8::encode_utf8(args[invocation_source(args)]@))
        && args[args.len() - 3]@ == "-o"@
        && invocation_value(vstd::utf8::encode_utf8(args[invocation_output(args)]@))
        && args[args.len() - 1]@ == "-Zchecksum-hash-algorithm=sha256"@
}

/// kind 给出 namespace，value 指向完整原值：edition 为原单项，其余具值
/// 选项为紧随 flag 的值。extern/import/env 的名字由首个 '=' 唯一分隔，
/// search 只接受 dependency= 路径；export 是直接路径，不拆成 name=path。
pub struct InvocationOption {
    pub kind: InvocationKind,
    pub token: usize,
    pub value: usize,
}

pub open spec fn interpreted_option_sequence(args: Seq<String>, i: int, end: int) -> Seq<InvocationOption>
    decreases if end >= i { end - i } else { 0int },
{
    if i < 7 || i >= end || end >= args.len() || !extension_value_valid(args, i, end) { Seq::empty() }
    else {
        let kind = invocation_kind(args[i]@);
        let width = option_width(kind);
        seq![InvocationOption { kind, token: i as usize,
            value: (if width == 1 { i } else { i + 1 }) as usize }]
            + interpreted_option_sequence(args, i + width, end)
    }
}

pub open spec fn interpreted_options(args: Seq<String>, options: Seq<InvocationOption>, end: int) -> bool {
    options == interpreted_option_sequence(args, 7, end)
}

/// 这里的索引引用完整原 argv，扩展项没有被剥去或另存为登记真源。
pub struct VerifierInvocation {
    pub source: usize,
    pub output: usize,
    pub emit: usize,
    pub options: Vec<InvocationOption>,
}

pub open spec fn invocation_matches(parsed: &VerifierInvocation, args: Seq<String>) -> bool {
    verifier_arguments(args) && parsed.source == invocation_source(args)
        && parsed.output == invocation_output(args) && parsed.emit == invocation_emit(args)
        && interpreted_options(args, parsed.options@, parsed.emit as int)
}

pub fn parse_verifier_arguments(args: &Vec<String>) -> (parsed: Option<VerifierInvocation>)
    ensures parsed.is_some() == verifier_arguments(args@),
        parsed.is_some() ==> invocation_matches(&parsed.unwrap(), args@),
{
    if args.len() < 12 { return None; }
    let emit = args.len() - 5;
    let source = emit + 1;
    let output = emit + 3;
    let outputs = compiler_outputs(args[output].as_str());
    let outputs_match = equal_bytes(args[emit].as_str().as_bytes(), outputs.as_slice());
    proof {
        vstd::utf8::encode_utf8_decode_utf8(args[emit]@);
        vstd::utf8::encode_utf8_decode_utf8(
            "--emit=link="@ + args[output]@ + ",dep-info="@ + args[output]@ + ".d"@);
    }
    if !equal_text(args[0].as_str(), "--crate-type=lib")
        || !equal_text(args[1].as_str(), "--no-cheating")
        || !equal_text(args[2].as_str(), "--no-external-by-default")
        || !equal_text(args[3].as_str(), "--num-threads") || !equal_text(args[4].as_str(), "2")
        || !equal_text(args[5].as_str(), "--output-json") || !equal_text(args[6].as_str(), "--compile")
        || !outputs_match || !valid_invocation_value(args[source].as_str().as_bytes())
        || !equal_text(args[emit + 2].as_str(), "-o") || !valid_invocation_value(args[output].as_str().as_bytes())
        || !equal_text(args[emit + 4].as_str(), "-Zchecksum-hash-algorithm=sha256") { return None; }
    let mut options: Vec<InvocationOption> = Vec::new();
    let mut i = 7;
    while i < emit
        invariant 7 <= i <= emit, emit == args.len() - 5, args.len() >= 12,
            options@ + interpreted_option_sequence(args@, i as int, emit as int)
                == interpreted_option_sequence(args@, 7, emit as int),
            invocation_extensions(args@, 7, emit as int) == invocation_extensions(args@, i as int, emit as int),
        decreases emit - i,
    {
        reveal_with_fuel(invocation_extensions, 2);
        reveal_with_fuel(interpreted_option_sequence, 2);
        if !valid_extension_value(args, i, emit) || !extension_is_fresh(args, i)
            || !paired_import(args, i, emit) { return None; }
        let kind = classify_invocation_token(args[i].as_str());
        let width = invocation_option_width(&kind);
        let value = if width == 1 { i } else { i + 1 };
        let ghost before = options@;
        options.push(InvocationOption { kind, token: i, value });
        proof {
            assert(options@.drop_last() =~= before);
            assert(options@.last().token == i);
            assert(options@ + interpreted_option_sequence(args@, (i + width) as int, emit as int)
                == interpreted_option_sequence(args@, 7, emit as int));
        }
        i += width;
    }
    Some(VerifierInvocation { source, output, emit, options })
}

pub fn canonical_verifier_arguments(args: &Vec<String>) -> (valid: bool)
    ensures valid == verifier_arguments(args@),
{
    parse_verifier_arguments(args).is_some()
}

/// 现役 Receipt／Cargo-Verus 还没有可供这些输入使用的完整 producer、VIR 和
/// 隐含工具上下文原件。语法识别不能填补这个缺口。没有 carrier 时明确拒绝，
/// 不建立空资格记录，也不把普通缓存或失败 producer 的库当证明。
/// 只保留原 source/checksum 合同的输入域；这不证明隐含工具上下文已完整。
pub open spec fn source_only_invocation(args: Seq<String>) -> bool {
    verifier_arguments(args) && interpreted_option_sequence(args, 7, invocation_emit(args)).len() == 0
}

pub fn require_registered_input_evidence(args: &Vec<String>) -> (out: Result<(), String>)
    ensures out.is_ok() == source_only_invocation(args@),
{
    let parsed = match parse_verifier_arguments(args) {
        Some(parsed) => parsed,
        None => { return Err("E_VERUS_RESULT: process or verifier arguments rejected".to_string()); },
    };
    if parsed.options.len() != 0 {
        return Err("E_VERUS_QUALIFICATION: compiler context and dependency/VIR producer evidence unavailable".to_string());
    }
    Ok(())
}

pub open spec fn report_accepted(report: &CompilerReport, contract: &OutputContract) -> bool {
    report.verus.version@ == contract.version@
        && report.verus.commit@ == contract.commit@
        && report.verification.success && !report.verification.encountered_error
        && !report.verification.encountered_vir_error
        && report.verification.errors == 0 && report.verification.verified > 0
        && report.verification.is_verifying_entire_crate
}

/// 完整证明结果的准入；原进程和调用参数仍由入口核对。
pub fn accepts_report(report: &CompilerReport, contract: &OutputContract) -> (accepted: bool)
    ensures accepted == report_accepted(report, contract),
{
    equal_text(report.verus.version.as_str(), contract.version.as_str())
        && equal_text(report.verus.commit.as_str(), contract.commit.as_str())
        && report.verification.success && !report.verification.encountered_error
        && !report.verification.encountered_vir_error && report.verification.errors == 0
        && report.verification.verified > 0 && report.verification.is_verifying_entire_crate
}

pub open spec fn output_accepted(
    contract: &Option<OutputContract>, decoded: &Option<CompilerReport>,
    args: Seq<String>, exit: i32, process_failed: bool,
) -> bool {
    match contract {
        None => true,
        Some(value) => valid_contract(contract) && verifier_arguments(args)
            && source_only_invocation(args) && exit == 0 && !process_failed
            && match decoded { None => false, Some(report) => report_accepted(report, value) },
    }
}

pub open spec fn output_error(
    contract: &Option<OutputContract>, decoded: &Option<CompilerReport>,
    args: Seq<String>, exit: i32, process_failed: bool,
) -> Option<Seq<char>> {
    match contract {
        None => None,
        Some(value) => {
            if !valid_contract(contract) {
                Some("E_VERUS_CONTRACT: version and commit must be nonempty"@)
            } else if exit != 0 || process_failed || !verifier_arguments(args) {
                Some("E_VERUS_RESULT: process or verifier arguments rejected"@)
            } else if !source_only_invocation(args) {
                Some("E_VERUS_QUALIFICATION: compiler context and dependency/VIR producer evidence unavailable"@)
            } else {
                match decoded {
                    None => Some("E_VERUS_RESULT: missing or malformed typed report"@),
                    Some(report) => if report_accepted(report, value) { None }
                        else { Some("E_VERUS_RESULT: incomplete, failed or mismatched verification"@) },
                }
            }
        },
    }
}

/// 直接消费同一原进程的退出、失败、参数和一次解码得到的完整报告。
pub fn admit_output(
    contract: &Option<OutputContract>, decoded: &Option<CompilerReport>,
    args: &Vec<String>, exit: i32, process_failed: bool,
) -> (out: Result<(), String>)
    ensures out.is_ok() == output_accepted(contract, decoded, args@, exit, process_failed),
        match out {
            Ok(()) => output_error(contract, decoded, args@, exit, process_failed).is_none(),
            Err(message) => output_error(contract, decoded, args@, exit, process_failed) == Some(message@),
        },
{
    let value = match contract { None => { return Ok(()); }, Some(value) => value };
    match validate_contract(contract) {
        Ok(()) => {},
        Err(error) => { return Err(error); },
    }
    if exit != 0 || process_failed || !canonical_verifier_arguments(args) {
        return Err("E_VERUS_RESULT: process or verifier arguments rejected".to_string());
    }
    match require_registered_input_evidence(args) {
        Ok(()) => {},
        Err(error) => { return Err(error); },
    }
    let report = match decoded {
        None => { return Err("E_VERUS_RESULT: missing or malformed typed report".to_string()); },
        Some(report) => report,
    };
    if !accepts_report(report, value) {
        return Err("E_VERUS_RESULT: incomplete, failed or mismatched verification".to_string());
    }
    Ok(())
}

/// 本地与离线回执共同消费原报告及原依赖观察。
pub fn admit_observed_output(
    contract: &Option<OutputContract>, decoded: &Option<CompilerReport>,
    args: &Vec<String>, exit: i32, process_failed: bool,
    dependency: &Option<Result<Vec<u8>, String>>, limit: usize,
    artifact: &Option<Result<CompiledArtifact, String>>,
) -> (out: Result<(), String>)
    ensures out.is_ok() == (output_accepted(contract, decoded, args@, exit, process_failed)
        && sources::dependency_observed(contract, dependency, limit)
        && artifact_observed(contract, artifact, args@)),
{
    match admit_output(contract, decoded, args, exit, process_failed) {
        Err(error) => { return Err(error); },
        Ok(()) => {},
    }
    match sources::admit_dependency_observation(contract, dependency, limit) {
        Err(error) => Err(error),
        Ok(()) => admit_artifact_observation(contract, artifact, args),
    }
}

/// 现役计划的借用投影；没有独立的准入标志或第二份方法权威。
pub struct ProofInvocation<'a> {
    pub operation: &'a str,
    pub argv: &'a Vec<String>,
    pub output_contract: &'a Option<OutputContract>,
    pub output_limit_bytes: usize,
}

/// 同一原回执的输入投影，库和源码观察不能来自另一次进程。
pub struct ProofProcess<'a> {
    pub operation: &'a str,
    pub argv: &'a Vec<String>,
    pub stdout: &'a [u8],
    pub exit: i32,
    pub failed: bool,
    pub dependency: &'a Option<Result<Vec<u8>, String>>,
    pub artifact: &'a Option<Result<CompiledArtifact, String>>,
}

/// 只负责原字节的标准 JSON 类型化解码；报告准入仍由本证明根负责。
pub trait CompilerReportRead {
    spec fn decoded(&self, bytes: Seq<u8>) -> Option<CompilerReport>;
    fn decode(&self, bytes: &[u8]) -> (out: Option<CompilerReport>)
        ensures out == self.decoded(bytes@);
}

pub struct ProofAdmission {
    pub report: CompilerReport,
    pub sources: sources::SourceAdmission,
    pub invocation: VerifierInvocation,
    pub artifact: CompiledArtifact,
}

pub open spec fn unique_proof_invocation(
    methods: Seq<ProofInvocation>, operation: Seq<char>, index: int,
) -> bool {
    0 <= index < methods.len() && methods[index].operation@ == operation
        && forall|i: int| 0 <= i < methods.len() && (#[trigger] methods[i]).operation@ == operation
            ==> i == index
}

pub fn find_proof_invocation(methods: &Vec<ProofInvocation>, operation: &str)
    -> (out: Result<usize, String>)
    ensures out.is_ok() == (exists|i: int| unique_proof_invocation(methods@, operation@, i)),
        out.is_ok() ==> unique_proof_invocation(methods@, operation@, out.unwrap() as int),
{
    let mut found: Option<usize> = None;
    let mut i = 0;
    while i < methods.len()
        invariant 0 <= i <= methods.len(),
            match found {
                None => forall|j: int| 0 <= j < i ==> (#[trigger] methods[j]).operation@ != operation@,
                Some(index) => 0 <= index < i && methods[index as int].operation@ == operation@
                    && forall|j: int| 0 <= j < i && (#[trigger] methods[j]).operation@ == operation@
                        ==> j == index,
            },
        decreases methods.len() - i,
    {
        if equal_text(methods[i].operation, operation) {
            if let Some(index) = found {
                assert(index != i);
                assert forall|j: int| unique_proof_invocation(methods@, operation@, j)
                    implies false by {
                    assert(j == index as int);
                    assert(j == i as int);
                }
                return Err("E_VERUS_RESULT: ambiguous registered operation".to_string());
            }
            found = Some(i);
        }
        i += 1;
    }
    match found {
        Some(index) => {
            assert(unique_proof_invocation(methods@, operation@, index as int));
            assert(exists|j: int| unique_proof_invocation(methods@, operation@, j));
            Ok(index)
        },
        None => {
            assert forall|j: int| unique_proof_invocation(methods@, operation@, j)
                implies false by {
                assert(0 <= j < methods.len());
                assert(methods[j].operation@ != operation@);
            }
            Err("E_VERUS_RESULT: operation has no registered invocation".to_string())
        },
    }
}

pub open spec fn same_arguments(first: Seq<String>, second: Seq<String>) -> bool {
    first.len() == second.len()
        && forall|i: int| 0 <= i < first.len() ==> (#[trigger] first[i])@ == second[i]@
}

pub fn equal_arguments(first: &Vec<String>, second: &Vec<String>) -> (same: bool)
    ensures same == same_arguments(first@, second@),
{
    if first.len() != second.len() { return false; }
    let mut i = 0;
    while i < first.len()
        invariant 0 <= i <= first.len(), first.len() == second.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] first[j])@ == second[j]@,
        decreases first.len() - i,
    {
        if !equal_text(first[i].as_str(), second[i].as_str()) { return false; }
        i += 1;
    }
    true
}

/// 完整参数逐字相同才可跨原回执／登记投影使用同一个解释；不能只比根或输出。
pub proof fn invocation_extensions_congruent(
    first: Seq<String>, second: Seq<String>, i: int, end: int,
)
    requires same_arguments(first, second), 7 <= i <= end < first.len(),
    ensures invocation_extensions(first, i, end) == invocation_extensions(second, i, end),
        interpreted_option_sequence(first, i, end) == interpreted_option_sequence(second, i, end),
    decreases end - i,
{
    reveal_with_fuel(invocation_extensions, 2);
    reveal_with_fuel(interpreted_option_sequence, 2);
    if i < end {
        assert(first[i]@ == second[i]@);
        assert(first[i + 1]@ == second[i + 1]@);
        assert(extension_value_valid(first, i, end) == extension_value_valid(second, i, end));
        assert forall|j: int| 7 <= j < i implies
            conflicting_extension(first, j, i) == conflicting_extension(second, j, i) by {
            assert(first[j]@ == second[j]@);
            assert(first[j + 1]@ == second[j + 1]@);
        }
        assert(fresh_extension(first, i) == fresh_extension(second, i));
        assert forall|j: int| 7 <= j < end && j + 1 < end implies
            (first[j]@ == "--extern"@ && same_input_name(vstd::utf8::encode_utf8(first[j + 1]@),
                vstd::utf8::encode_utf8(first[i + 1]@)))
            == (second[j]@ == "--extern"@ && same_input_name(vstd::utf8::encode_utf8(second[j + 1]@),
                vstd::utf8::encode_utf8(second[i + 1]@))) by {
            assert(first[j]@ == second[j]@);
            assert(first[j + 1]@ == second[j + 1]@);
        }
        assert(import_has_extern(first, i, end) == import_has_extern(second, i, end));
        if extension_value_valid(first, i, end) {
            let next = i + option_width(invocation_kind(first[i]@));
            invocation_extensions_congruent(first, second, next, end);
        }
    }
}

pub proof fn same_invocation_semantics(first: Seq<String>, second: Seq<String>)
    requires same_arguments(first, second),
    ensures verifier_arguments(first) == verifier_arguments(second),
        source_only_invocation(first) == source_only_invocation(second),
        first.len() >= 12 ==> interpreted_option_sequence(first, 7, invocation_emit(first))
            == interpreted_option_sequence(second, 7, invocation_emit(second)),
{
    if first.len() >= 12 {
        invocation_extensions_congruent(first, second, 7, invocation_emit(first));
    }
}

pub open spec fn proof_bound(
    proof: &ProofAdmission, method: &ProofInvocation, process: &ProofProcess,
    registry: &sources::SourceRegistry,
) -> bool {
    method.output_contract.is_some() && process.argv.len() > 0
        && invocation_matches(&proof.invocation, method.argv@)
        && process.operation@ == method.operation@
        && same_arguments(process.argv@.skip(1), method.argv@)
        && process.exit == 0 && !process.failed
        && output_accepted(method.output_contract, &Some(proof.report), process.argv@.skip(1),
            process.exit, process.failed)
        && sources::dependency_observed(method.output_contract, process.dependency, method.output_limit_bytes)
        && artifact_observed(method.output_contract, process.artifact, process.argv@.skip(1))
        && match process.artifact {
            Some(Ok(artifact)) => proof.artifact.path@ == artifact.path@
                && proof.artifact.sha256@ == artifact.sha256@ && proof.artifact.length == artifact.length,
            _ => false,
        }
        && sources::binding_error(registry.files.entries@, method.operation@, method.argv[invocation_source(method.argv@)]@,
            proof.sources.candidate@, 0).is_none()
        && match process.dependency {
            Some(Ok(bytes)) => sources::admission_matches(&proof.sources,
                process.argv@.skip(1), bytes@, registry.files.entries@),
            _ => false,
        }
}

/// 无证明合同的普通操作成功，仍须绑定唯一方法、完整原参数及无失败原进程。
pub open spec fn ordinary_process_bound(method: &ProofInvocation, process: &ProofProcess) -> bool {
    method.output_contract.is_none() && process.argv.len() > 0
        && process.operation@ == method.operation@
        && same_arguments(process.argv@.skip(1), method.argv@)
        && process.exit == 0 && !process.failed
        && process.dependency.is_none() && process.artifact.is_none()
}

pub open spec fn ordinary_output_accepted(
    operation: Seq<char>, methods: Seq<ProofInvocation>, original: &Option<ProofProcess>,
) -> bool {
    match original {
        Some(process) => exists|i: int| unique_proof_invocation(methods, operation, i)
            && ordinary_process_bound(&methods[i], process),
        None => false,
    }
}

/// 成功报告、原 argv、库、完整依赖和固定候选源码在此共同准入。
/// 普通成功与非 passed 操作只返回 None，不能用作全仓覆盖的证明条目。
pub fn bind_operation_output<I: sources::CandidateSourceRead + CompilerReportRead,
    F: Fn(&str, &str) -> String>(
    state: &str, operation: &str, methods: &Vec<ProofInvocation>,
    original: Option<ProofProcess>, registry: &Result<sources::SourceRegistry, String>, io: &I,
    format_error: F,
) -> (out: Result<Option<ProofAdmission>, String>)
    requires forall|message: &str, path: &str| format_error.requires((message, path)),
    ensures state@ != "passed"@ ==> out.is_ok() && out.unwrap().is_none(),
        state@ == "passed"@ ==> ((match out { Ok(None) => true, _ => false })
            == ordinary_output_accepted(operation@, methods@, &original)),
        match out {
            Ok(Some(proof)) => state@ == "passed"@ && original.is_some() && registry.is_ok()
                && exists|i: int| unique_proof_invocation(methods@, operation@, i)
                    && proof_bound(&proof, &methods[i], &original.unwrap(), &registry.unwrap())
                    && io.decoded(original.unwrap().stdout@) == Some(proof.report),
            Ok(None) => state@ != "passed"@ || exists|i: int|
                unique_proof_invocation(methods@, operation@, i) && methods[i].output_contract.is_none(),
            Err(_) => true,
        },
        state@ == "passed"@ && out.is_ok() ==> original.is_some()
            && original.unwrap().exit == 0 && !original.unwrap().failed,
{
    if !equal_text(state, "passed") { return Ok(None); }
    let process = match original {
        Some(process) => process,
        None => { return Err("E_VERUS_RESULT: successful operation has no original receipt".to_string()); },
    };
    let index = match find_proof_invocation(methods, operation) {
        Ok(index) => index,
        Err(error) => { return Err(error); },
    };
    let method = &methods[index];
    proof {
        if ordinary_output_accepted(operation@, methods@, &original) {
            let ordinary_index = choose|i: int| unique_proof_invocation(methods@, operation@, i)
                && ordinary_process_bound(&methods[i], &process);
            assert(ordinary_index == index as int);
            assert(ordinary_process_bound(method, &process));
        }
    }
    let args = match sources::process_arguments(process.argv) {
        Ok(args) => args,
        Err(error) => { return Err(error); },
    };
    if !equal_text(process.operation, operation) || !equal_arguments(&args, method.argv)
        || process.exit != 0 || process.failed {
        return Err("E_VERUS_RESULT: original operation, arguments or process rejected".to_string());
    }
    proof {
        same_invocation_semantics(args@, method.argv@);
        same_invocation_semantics(args@, process.argv@.skip(1));
    }
    let decoded = io.decode(proof_stdout(method.output_contract, process.stdout));
    match admit_observed_output(method.output_contract, &decoded, &args,
        process.exit, process.failed, process.dependency, method.output_limit_bytes, process.artifact) {
        Err(error) => { return Err(error); },
        Ok(()) => {},
    }
    if method.output_contract.is_none() {
        assert(ordinary_process_bound(method, &process));
        assert(ordinary_output_accepted(operation@, methods@, &original));
        return Ok(None);
    }
    assert(!ordinary_output_accepted(operation@, methods@, &original));
    let source_admission = match sources::bind_compiler_sources(&args, process.dependency, registry, io) {
        Ok(Some(admission)) => admission,
        Err(error) => { return Err(error); },
        Ok(None) => { return Err("E_VERUS_RESULT: proof has no bound compiler sources".to_string()); },
    };
    let source_registry = match registry {
        Ok(registry) => registry,
        Err(error) => { return Err(error.clone()); },
    };
    let parsed = match parse_verifier_arguments(&args) {
        Some(parsed) => parsed,
        None => { return Err("E_VERUS_RESULT: process or verifier arguments rejected".to_string()); },
    };
    match sources::bind_registered_roots(&source_registry.files.entries, operation,
        args[parsed.source].as_str(), &source_admission.candidate) {
        Ok(()) => {},
        Err(error) => { return Err(error.render(format_error)); },
    }
    let report = match decoded {
        Some(report) => report,
        None => { return Err("E_VERUS_RESULT: proof has no original typed report".to_string()); },
    };
    let artifact = match process.artifact {
        Some(Ok(artifact)) => artifact.clone(),
        _ => { return Err("E_VERUS_ARTIFACT: missing or failed original library observation".to_string()); },
    };
    let proof = ProofAdmission { report, sources: source_admission, invocation: parsed, artifact };
    assert(unique_proof_invocation(methods@, operation@, index as int));
    assert(proof_bound(&proof, method, &process, &registry.unwrap()));
    Ok(Some(proof))
}

}
