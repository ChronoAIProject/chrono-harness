//! 官方 Cargo-Verus 原输出的顺序合同；JSON 和文件观察由调用方传入。
use super::identifiers::{equal_bytes, equal_text};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(untagged))]
pub enum Record { Proof(super::CompilerReport), Cargo(Cargo) }

#[cfg_attr(not(verus_keep_ghost), derive(serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(tag = "reason"))]
pub enum Cargo {
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "compiler-artifact"))]
    Artifact(Artifact),
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "compiler-message"))]
    Message { message: Message },
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "build-script-executed"))]
    Script { package_id: String, linked_libs: Vec<String>, linked_paths: Vec<String>,
        cfgs: Vec<String>, env: Vec<(String, String)>, out_dir: String },
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "build-finished"))]
    Finished { success: bool },
}

#[cfg_attr(not(verus_keep_ghost), derive(serde::Deserialize))]
pub struct Message { pub level: String }

#[cfg_attr(not(verus_keep_ghost), derive(Clone, serde::Deserialize, serde::Serialize))]
pub struct Target {
    pub name: String, pub src_path: String, pub kind: Vec<String>, pub crate_types: Vec<String>,
}

#[cfg_attr(not(verus_keep_ghost), derive(Clone, serde::Deserialize, serde::Serialize))]
pub struct Artifact {
    pub package_id: String,
    pub manifest_path: String,
    pub target: Target,
    pub filenames: Vec<String>,
    pub executable: Option<String>,
    pub fresh: bool,
}

#[cfg_attr(not(verus_keep_ghost), derive(Clone, serde::Deserialize, serde::Serialize))]
pub struct Identity { pub path: String, pub sha256: String, pub length: u64 }

pub fn bounded_original(regular: bool, bytes: u64) -> (valid: bool)
    ensures valid == (regular && bytes <= 8_388_608),
{ regular && bytes <= 8_388_608 }

pub fn content_matches(actual: &str, length: u64, expected: &str, expected_length: u64) -> (same: bool)
    ensures same == (actual@ == expected@ && length == expected_length),
{ equal_text(actual, expected) && length == expected_length }

pub open spec fn selected_target(
    artifact: &Artifact, manifest: Seq<char>, source: Seq<char>, name: Seq<char>, kind: Seq<char>,
) -> bool {
    artifact.manifest_path@ == manifest && artifact.target.src_path@ == source
        && artifact.target.name@ == name && artifact.target.kind.len() == 1
        && artifact.target.kind[0]@ == kind && artifact.target.crate_types.len() == 1
        && artifact.target.crate_types[0]@ == kind
}

pub fn artifact_selected(artifact: &Artifact, manifest: &str, source: &str, name: &str, kind: &str) -> (selected: bool)
    ensures selected == selected_target(artifact, manifest@, source@, name@, kind@),
{
    equal_text(artifact.manifest_path.as_str(), manifest)
        && equal_text(artifact.target.src_path.as_str(), source)
        && equal_text(artifact.target.name.as_str(), name)
        && artifact.target.kind.len() == 1 && equal_text(artifact.target.kind[0].as_str(), kind)
        && artifact.target.crate_types.len() == 1 && equal_text(artifact.target.crate_types[0].as_str(), kind)
}

pub fn diagnostic_rejected(level: &str) -> (rejected: bool)
    ensures rejected == (level@ != "warning"@ && level@ != "note"@ && level@ != "help"@),
{ !equal_text(level, "warning") && !equal_text(level, "note") && !equal_text(level, "help") }

pub fn registered_sources(schema: &str, source: &str, sources: &Vec<String>) -> (valid: bool)
    ensures valid == (schema@ == "chrono-proved-build/v1"@ && sources.len() > 0
        && exists|i: int| 0 <= i < sources.len() && sources[i]@ == source@),
{
    if !equal_text(schema, "chrono-proved-build/v1") || sources.len() == 0 { return false; }
    let mut i = 0;
    while i < sources.len()
        invariant 0 <= i <= sources.len(), sources.len() > 0, schema@ == "chrono-proved-build/v1"@,
            forall|j: int| 0 <= j < i ==> sources[j]@ != source@,
        decreases sources.len() - i,
    {
        if equal_text(sources[i].as_str(), source) { return true; }
        i += 1;
    }
    false
}

pub open spec fn published_binary(artifact: &Artifact, path: Seq<char>) -> bool {
    artifact.filenames.len() == 1 && artifact.filenames[0]@ == path
        && match artifact.executable { Some(value) => value@ == path, None => false }
}

pub fn binary_shape(artifact: &Artifact, path: &str) -> (valid: bool)
    ensures valid == published_binary(artifact, path@),
{
    artifact.filenames.len() == 1 && equal_text(artifact.filenames[0].as_str(), path)
        && match &artifact.executable { Some(value) => equal_text(value.as_str(), path), None => false }
}

pub enum Recovery { Continue, Rebuild, Reject }

pub fn recovery(fresh: bool, valid: bool, available: bool) -> (action: Recovery)
    ensures match action {
        Recovery::Continue => !fresh || valid,
        Recovery::Rebuild => fresh && !valid && available,
        Recovery::Reject => fresh && !valid && !available,
    },
{
    if !fresh || valid { Recovery::Continue }
    else if available { Recovery::Rebuild } else { Recovery::Reject }
}

pub open spec fn text_lists_match(first: Seq<String>, second: Seq<String>) -> bool {
    first.len() == second.len() && forall|i: int| 0 <= i < first.len() ==> first[i]@ == second[i]@
}

pub fn same_text_list(first: &Vec<String>, second: &Vec<String>) -> (same: bool)
    ensures same == text_lists_match(first@, second@),
{
    if first.len() != second.len() { return false; }
    let mut i = 0;
    while i < first.len()
        invariant 0 <= i <= first.len() == second.len(),
            forall|j: int| 0 <= j < i ==> first[j]@ == second[j]@,
        decreases first.len() - i,
    {
        if !equal_text(first[i].as_str(), second[i].as_str()) { return false; }
        i += 1;
    }
    true
}

pub open spec fn optional_texts_match(first: Option<String>, second: Option<String>) -> bool {
    match (first, second) { (None, None) => true, (Some(a), Some(b)) => a@ == b@, _ => false }
}

pub fn same_optional_text(first: &Option<String>, second: &Option<String>) -> (same: bool)
    ensures same == optional_texts_match(*first, *second),
{
    match (first, second) { (None, None) => true,
        (Some(a), Some(b)) => equal_text(a.as_str(), b.as_str()), _ => false }
}

pub open spec fn matching_artifact(first: &Artifact, second: &Artifact) -> bool {
    first.package_id@ == second.package_id@ && first.manifest_path@ == second.manifest_path@
        && first.target.name@ == second.target.name@ && first.target.src_path@ == second.target.src_path@
        && text_lists_match(first.target.kind@, second.target.kind@)
        && text_lists_match(first.target.crate_types@, second.target.crate_types@)
        && text_lists_match(first.filenames@, second.filenames@)
        && optional_texts_match(first.executable, second.executable) && first.fresh == second.fresh
}

pub fn same_artifact(first: &Artifact, second: &Artifact) -> (same: bool)
    ensures same == matching_artifact(first, second),
{
    equal_text(first.package_id.as_str(), second.package_id.as_str())
        && equal_text(first.manifest_path.as_str(), second.manifest_path.as_str())
        && equal_text(first.target.name.as_str(), second.target.name.as_str())
        && equal_text(first.target.src_path.as_str(), second.target.src_path.as_str())
        && same_text_list(&first.target.kind, &second.target.kind)
        && same_text_list(&first.target.crate_types, &second.target.crate_types)
        && same_text_list(&first.filenames, &second.filenames)
        && same_optional_text(&first.executable, &second.executable) && first.fresh == second.fresh
}

pub fn same_content(first: &Identity, second: &Identity) -> (same: bool)
    ensures same == (first.sha256@ == second.sha256@ && first.length == second.length
        && first.length > 0 && super::identifiers::digest_format(vstd::utf8::encode_utf8(first.sha256@))),
{
    equal_text(first.sha256.as_str(), second.sha256.as_str()) && first.length == second.length
        && first.length > 0 && super::identifiers::valid_digest(first.sha256.as_str().as_bytes())
}

pub open spec fn matching_identity(first: &Identity, second: &Identity) -> bool {
    first.path@ == second.path@ && first.sha256@ == second.sha256@ && first.length == second.length
        && first.path@.len() > 0 && first.length > 0
        && super::identifiers::digest_format(vstd::utf8::encode_utf8(first.sha256@))
}

pub fn same_identity(first: &Identity, second: &Identity) -> (same: bool)
    ensures same == matching_identity(first, second),
{
    equal_text(first.path.as_str(), second.path.as_str())
        && equal_text(first.sha256.as_str(), second.sha256.as_str()) && first.length == second.length
        && !first.path.as_str().is_empty() && first.length > 0
        && super::identifiers::valid_digest(first.sha256.as_str().as_bytes())
}

pub open spec fn matching_identities(first: Seq<Identity>, second: Seq<Identity>) -> bool {
    first.len() > 0 && first.len() == second.len()
        && forall|i: int| 0 <= i < first.len() ==> matching_identity(&first[i], &second[i])
}

pub fn same_identities(first: &Vec<Identity>, second: &Vec<Identity>) -> (same: bool)
    ensures same == matching_identities(first@, second@),
{
    if first.len() == 0 || first.len() != second.len() { return false; }
    let mut i = 0;
    while i < first.len()
        invariant 0 <= i <= first.len() == second.len(), first.len() > 0,
            forall|j: int| 0 <= j < i ==> matching_identity(&first[j], &second[j]),
        decreases first.len() - i,
    {
        if !same_identity(&first[i], &second[i]) { return false; }
        i += 1;
    }
    true
}

pub fn retained_header(schema: &str, inputs: &str, expected: &str, exit: i32, fresh: bool,
    dependency: &[u8], current: &[u8]) -> (accepted: bool)
    ensures accepted == (schema@ == "chrono-library-proof/v1"@ && inputs@ == expected@
        && exit == 0 && !fresh && dependency@ == current@),
{
    equal_text(schema, "chrono-library-proof/v1") && equal_text(inputs, expected)
        && exit == 0 && !fresh && equal_bytes(dependency, current)
}

pub open spec fn crate_name_byte(byte: u8) -> bool {
    (65 <= byte <= 90) || (97 <= byte <= 122) || (48 <= byte <= 57) || byte == 95
}

pub fn library_name(name: &[u8]) -> (valid: bool)
    ensures valid == (name@.len() > 0 && forall|i: int| 0 <= i < name@.len() ==> crate_name_byte(name@[i])),
{
    if name.len() == 0 { return false; }
    let mut i = 0;
    while i < name.len()
        invariant 0 <= i <= name.len(), name.len() > 0,
            forall|j: int| 0 <= j < i ==> crate_name_byte(name@[j]),
        decreases name.len() - i,
    {
        if !((65 <= name[i] && name[i] <= 90) || (97 <= name[i] && name[i] <= 122)
            || (48 <= name[i] && name[i] <= 57) || name[i] == 95) { return false; }
        i += 1;
    }
    true
}

pub open spec fn fingerprint_byte(byte: u8) -> bool { (48 <= byte <= 57) || (97 <= byte <= 102) }

pub fn fingerprint(bytes: &[u8]) -> (valid: bool)
    ensures valid == (bytes@.len() > 0 && forall|i: int| 0 <= i < bytes@.len() ==> fingerprint_byte(bytes@[i])),
{
    if bytes.len() == 0 { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant 0 <= i <= bytes.len(), bytes.len() > 0,
            forall|j: int| 0 <= j < i ==> fingerprint_byte(bytes@[j]),
        decreases bytes.len() - i,
    {
        if !((48 <= bytes[i] && bytes[i] <= 57) || (97 <= bytes[i] && bytes[i] <= 102)) { return false; }
        i += 1;
    }
    true
}

pub fn all_present(present: &Vec<bool>) -> (complete: bool)
    ensures complete == (present.len() > 0 && forall|i: int| 0 <= i < present.len() ==> present[i]),
{
    if present.len() == 0 { return false; }
    let mut i = 0;
    while i < present.len()
        invariant 0 <= i <= present.len(), present.len() > 0,
            forall|j: int| 0 <= j < i ==> present[j],
        decreases present.len() - i,
    {
        if !present[i] { return false; }
        i += 1;
    }
    true
}

pub open spec fn dependency_name(name: Seq<u8>, prefix: Seq<u8>) -> bool {
    let hash = name.subrange(prefix.len() as int, name.len() - 2);
    prefix.len() > 0 && name.len() >= 2 && prefix.len() < name.len() - 2
        && super::sources::prefix_at(name, 0, prefix)
        && name[name.len() - 2] == 46 && name[name.len() - 1] == 100
        && forall|i: int| 0 <= i < hash.len() ==> fingerprint_byte(hash[i])
}

pub fn retain_attempt(succeeded: bool, cache_rejected: bool) -> (retain: bool)
    ensures retain == (!succeeded || cache_rejected),
{ !succeeded || cache_rejected }

pub fn selected_dependency(name: &[u8], prefix: &[u8]) -> (selected: bool)
    ensures selected == dependency_name(name@, prefix@),
{
    if prefix.len() == 0 || name.len() < 2 || prefix.len() >= name.len() - 2
        || !super::sources::matches_at(name, 0, prefix)
        || name[name.len() - 2] != 46 || name[name.len() - 1] != 100 { return false; }
    let hash = super::sources::copy_range(name, prefix.len(), name.len() - 2);
    fingerprint(hash.as_slice())
}

pub fn output_inventory_within_bound(count: usize) -> (valid: bool)
    ensures valid == (count <= 4096),
{ count <= 4096 }

pub open spec fn library_prefix(directory: Seq<u8>, name: Seq<u8>) -> Seq<u8> {
    directory + seq![47u8, 108u8, 105u8, 98u8] + name + seq![45u8]
}

pub open spec fn library_hash(directory: Seq<u8>, name: Seq<u8>, filenames: Seq<String>) -> Seq<u8> {
    let prefix = library_prefix(directory, name);
    if filenames.len() != 2 { Seq::empty() }
    else {
        let first = vstd::utf8::encode_utf8(filenames[0]@);
        if first.len() < 5 || first.len() - 5 <= prefix.len() { Seq::empty() }
        else { first.subrange(prefix.len() as int, first.len() - 5) }
    }
}

pub open spec fn library_shape(directory: Seq<u8>, name: Seq<u8>, filenames: Seq<String>, executable: bool) -> bool {
    let prefix = library_prefix(directory, name);
    let hash = library_hash(directory, name, filenames);
    !executable && filenames.len() == 2 && name.len() > 0
        && (forall|i: int| 0 <= i < name.len() ==> crate_name_byte(name[i]))
        && hash.len() > 0 && (forall|i: int| 0 <= i < hash.len() ==> fingerprint_byte(hash[i]))
        && vstd::utf8::encode_utf8(filenames[0]@) == prefix + hash + seq![46u8, 114u8, 108u8, 105u8, 98u8]
        && vstd::utf8::encode_utf8(filenames[1]@) == prefix + hash + seq![46u8, 114u8, 109u8, 101u8, 116u8, 97u8]
}

pub struct LibraryPaths { pub dependency: Vec<u8>, pub outputs: Vec<Vec<u8>> }

/// 从同一 Cargo JSON 的 rlib/rmeta 对取得 Rust 1.95 的原 dep-info 地址，
/// 拒绝其它目录、别名、混配 fingerprint、未知字节和非库目标。
pub fn compiler_library_paths(directory: &[u8], name: &[u8], filenames: &Vec<String>, executable: bool) -> (out: Option<LibraryPaths>)
    ensures out.is_some() == library_shape(directory@, name@, filenames@, executable),
        out.is_some() ==> out.unwrap().dependency@ == directory@ + seq![47u8] + name@ + seq![45u8]
            + library_hash(directory@, name@, filenames@) + seq![46u8, 100u8],
        out.is_some() ==> out.unwrap().outputs.len() == 2
            && out.unwrap().outputs[0]@ == vstd::utf8::encode_utf8(filenames[0]@)
            && out.unwrap().outputs[1]@ == vstd::utf8::encode_utf8(filenames[1]@),
{
    if executable || filenames.len() != 2 || !library_name(name) { return None; }
    let mut prefix = Vec::new();
    super::append_bytes(&mut prefix, directory);
    super::append_bytes(&mut prefix, &[47u8, 108u8, 105u8, 98u8]);
    super::append_bytes(&mut prefix, name);
    prefix.push(45u8);
    assert(prefix@ == library_prefix(directory@, name@));
    let first = filenames[0].as_str().as_bytes();
    let second = filenames[1].as_str().as_bytes();
    if first.len() < 5 || first.len() - 5 <= prefix.len() { return None; }
    let hash = super::sources::copy_range(first, prefix.len(), first.len() - 5);
    assert(hash@ == library_hash(directory@, name@, filenames@));
    if !fingerprint(hash.as_slice()) { return None; }
    let mut expected_first = prefix.clone();
    super::append_bytes(&mut expected_first, hash.as_slice());
    super::append_bytes(&mut expected_first, &[46u8, 114u8, 108u8, 105u8, 98u8]);
    let mut expected_second = prefix;
    super::append_bytes(&mut expected_second, hash.as_slice());
    super::append_bytes(&mut expected_second, &[46u8, 114u8, 109u8, 101u8, 116u8, 97u8]);
    if !equal_bytes(first, expected_first.as_slice()) || !equal_bytes(second, expected_second.as_slice()) { return None; }
    let mut dependency = Vec::new();
    super::append_bytes(&mut dependency, directory);
    dependency.push(47u8);
    super::append_bytes(&mut dependency, name);
    dependency.push(45u8);
    super::append_bytes(&mut dependency, hash.as_slice());
    super::append_bytes(&mut dependency, &[46u8, 100u8]);
    let mut outputs = Vec::new();
    outputs.push(super::sources::copy_range(first, 0, first.len()));
    outputs.push(super::sources::copy_range(second, 0, second.len()));
    Some(LibraryPaths { dependency, outputs })
}

/// matches 由调用方核对点名库／原证明或最终二进制的实际字节后产生。
pub enum Event {
    Proof { accepted: bool },
    Library { fresh: bool, matches: bool },
    Binary { matches: bool },
    Finished { success: bool },
    OtherArtifact,
    Note,
    Error,
}

pub struct State { pub phase: u8 }

pub struct TargetIdentity { pub manifest: String, pub source: String, pub name: String }
pub struct BuildTargets { pub library: TargetIdentity, pub binary: TargetIdentity, pub published: String }

pub open spec fn target_matches(artifact: &Artifact, target: &TargetIdentity, kind: Seq<char>) -> bool {
    selected_target(artifact, target.manifest@, target.source@, target.name@, kind)
}

pub fn target_selected(artifact: &Artifact, target: &TargetIdentity, kind: &str) -> (selected: bool)
    ensures selected == target_matches(artifact, target, kind@),
{ artifact_selected(artifact, target.manifest.as_str(), target.source.as_str(), target.name.as_str(), kind) }

pub open spec fn record_event(record: &Record, targets: &BuildTargets, contract: &super::OutputContract) -> Event {
    match record {
        Record::Proof(report) => Event::Proof { accepted: super::report_accepted(report, contract) },
        Record::Cargo(Cargo::Artifact(artifact)) => {
            if target_matches(artifact, &targets.library, "lib"@) {
                Event::Library { fresh: artifact.fresh, matches: true }
            } else if target_matches(artifact, &targets.binary, "bin"@) {
                Event::Binary { matches: published_binary(artifact, targets.published@) }
            } else { Event::OtherArtifact }
        },
        Record::Cargo(Cargo::Message { message }) =>
            if message.level@ != "warning"@ && message.level@ != "note"@ && message.level@ != "help"@
                { Event::Error } else { Event::Note },
        Record::Cargo(Cargo::Script { .. }) => Event::Note,
        Record::Cargo(Cargo::Finished { success }) => Event::Finished { success: *success },
    }
}

pub fn inspect(record: &Record, targets: &BuildTargets, contract: &super::OutputContract) -> (event: Event)
    ensures event == record_event(record, targets, contract),
{
    match record {
        Record::Proof(report) => Event::Proof { accepted: super::accepts_report(report, contract) },
        Record::Cargo(Cargo::Artifact(artifact)) => {
            if target_selected(artifact, &targets.library, "lib") {
                Event::Library { fresh: artifact.fresh, matches: true }
            } else if target_selected(artifact, &targets.binary, "bin") {
                Event::Binary { matches: binary_shape(artifact, targets.published.as_str()) }
            } else { Event::OtherArtifact }
        },
        Record::Cargo(Cargo::Message { message }) =>
            if diagnostic_rejected(message.level.as_str()) { Event::Error } else { Event::Note },
        Record::Cargo(Cargo::Script { .. }) => Event::Note,
        Record::Cargo(Cargo::Finished { success }) => Event::Finished { success: *success },
    }
}

pub open spec fn trace(records: Seq<Record>, targets: &BuildTargets, contract: &super::OutputContract) -> Option<u8>
    decreases records.len(),
{
    if records.len() == 0 { Some(0u8) }
    else { match trace(records.drop_last(), targets, contract) {
        Some(phase) => next_phase(phase, record_event(&records.last(), targets, contract)),
        None => None,
    } }
}

/// 完整原输出在任何缓存恢复或文件发布前核对；错误不能被清理／重跑覆盖。
pub fn complete_records(records: &Vec<Record>, targets: &BuildTargets, contract: &super::OutputContract) -> (accepted: bool)
    ensures accepted == (trace(records@, targets, contract) == Some(4u8)),
{
    let mut state = State::new();
    let mut i = 0;
    while i < records.len()
        invariant 0 <= i <= records.len(),
            trace(records@.take(i as int), targets, contract) == Some(state.phase),
        decreases records.len() - i,
    {
        proof {
            assert(records@.take(i as int + 1).drop_last() =~= records@.take(i as int));
            assert(records@.take(i as int + 1).last() == records@[i as int]);
        }
        let event = inspect(&records[i], targets, contract);
        state = match state.step(event) {
            Ok(next) => next,
            Err(_) => {
                proof { failed_trace_stays_failed(records@, targets, contract, i as int + 1); }
                return false;
            },
        };
        i += 1;
    }
    proof { assert(records@.take(i as int) =~= records@); }
    state.accepted()
}

proof fn failed_trace_stays_failed(records: Seq<Record>, targets: &BuildTargets, contract: &super::OutputContract, prefix: int)
    requires 0 <= prefix <= records.len(), trace(records.take(prefix), targets, contract).is_none(),
    ensures trace(records, targets, contract).is_none(),
    decreases records.len() - prefix,
{
    if prefix < records.len() {
        assert(records.drop_last().take(prefix) =~= records.take(prefix));
        failed_trace_stays_failed(records.drop_last(), targets, contract, prefix);
    } else { assert(records.take(prefix) =~= records); }
}

pub open spec fn next_phase(phase: u8, event: Event) -> Option<u8> {
    if phase >= 4 { None }
    else {
        match event {
            Event::Proof { accepted } => if phase == 0 && accepted { Some(1u8) } else { None },
            Event::Library { fresh, matches } =>
                if matches && ((fresh && phase == 0) || (!fresh && phase == 1)) { Some(2u8) }
                else { None },
            Event::Binary { matches } => if phase == 2 && matches { Some(3u8) } else { None },
            Event::Finished { success } => if phase == 3 && success { Some(4u8) } else { None },
            Event::OtherArtifact => Some(phase),
            Event::Note => Some(phase),
            Event::Error => None,
        }
    }
}

impl State {
    pub fn new() -> (state: Self)
        ensures state.phase == 0,
    { Self { phase: 0 } }

    pub fn accepted(&self) -> (accepted: bool)
        ensures accepted == (self.phase == 4),
    { self.phase == 4 }

    /// 每条原记录只推进一次；未知、失败及重复终态不能被后续成功覆盖。
    pub fn step(&self, event: Event) -> (out: Result<Self, u8>)
        ensures out.is_ok() == next_phase(self.phase, event).is_some(),
            out.is_ok() ==> out.unwrap().phase == next_phase(self.phase, event).unwrap(),
            match out { Err(code) => code == self.phase, _ => true },
    {
        if self.phase >= 4 { return Err(self.phase); }
        let phase = match event {
            Event::Proof { accepted } => {
                if self.phase != 0 || !accepted { return Err(self.phase); }
                1u8
            },
            Event::Library { fresh, matches } => {
                if !matches || !((fresh && self.phase == 0) || (!fresh && self.phase == 1)) {
                    return Err(self.phase);
                }
                2u8
            },
            Event::Binary { matches } => {
                if self.phase != 2 || !matches { return Err(self.phase); }
                3u8
            },
            Event::Finished { success } => {
                if self.phase != 3 || !success { return Err(self.phase); }
                4u8
            },
            Event::OtherArtifact => {
                self.phase
            },
            Event::Note => self.phase,
            Event::Error => { return Err(self.phase); },
        };
        Ok(Self { phase })
    }
}

}
