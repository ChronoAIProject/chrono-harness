//! Bootstrap 工具绑定判断；文件观察和解码归调用方。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../../../crates/identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
use identifiers::equal_text;

// 复用生命周期所有者的权威类型和发布判断；不复制为宿主私有版本。
#[path = "../../../../crates/worktree/src/verified_owner_publication.rs"]
pub mod owner_publication;

verus! {

pub fn archive_matches(actual: &[u8], expected: &[u8]) -> (valid: bool)
    ensures valid <==> (expected@.len() == 64
        && (forall|i: int| 0 <= i < expected@.len() ==> hash_byte(expected@[i]))
        && actual@ == expected@),
{
    valid_sha256(expected) && identifiers::equal_bytes(actual, expected)
}

pub fn owner_publication_header(schema: &str, adoption: &str, completion: &str) -> (valid: bool)
    ensures valid <==> (schema@ == "chrono-bootstrap-owner-publication/v1"@
        && adoption@ == "not-claimed"@ && completion@ == "not-claimed"@),
{
    equal_text(schema, "chrono-bootstrap-owner-publication/v1") && equal_text(adoption, "not-claimed")
        && equal_text(completion, "not-claimed")
}

pub fn owner_adoption_header(schema: &str) -> (valid: bool)
    ensures valid <==> schema@ == "chrono-worktree-owner-adoption/v1"@,
{ equal_text(schema, "chrono-worktree-owner-adoption/v1") }

pub fn owner_stage_input(installed_empty: bool, declared: bool, selected: &str, expected: &str) -> (valid: bool)
    ensures valid <==> (installed_empty && declared && selected@ == expected@),
{ installed_empty && declared && equal_text(selected, expected) }

pub enum DeclarationSource { Registry, Direct, Unbound, Reject }

pub fn startup_schema(actual: &str, expected: &str) -> (valid: bool)
    ensures valid <==> actual@ == expected@,
{ equal_text(actual, expected) }

pub fn startup_mode(mode: u32) -> (valid: bool)
    ensures valid <==> (0 < mode <= 511),
{ 0 < mode && mode <= 511 }

pub open spec fn startup_name_byte(b: u8) -> bool {
    (65 <= b <= 90) || (97 <= b <= 122) || (48 <= b <= 57) || b == 95 || b == 45
}

pub fn startup_name(bytes: &[u8]) -> (valid: bool)
    ensures valid <==> (bytes@.len() > 0
        && forall|i: int| 0 <= i < bytes@.len() ==> startup_name_byte(bytes@[i])),
{
    if bytes.len() == 0 { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant 0 <= i <= bytes.len(), bytes.len() > 0,
            forall|j: int| 0 <= j < i ==> startup_name_byte(bytes@[j]),
        decreases bytes.len() - i,
    {
        let b = bytes[i];
        if !((65 <= b && b <= 90) || (97 <= b && b <= 122) || (48 <= b && b <= 57) || b == 95 || b == 45) { return false; }
        i += 1;
    }
    true
}

pub fn immutable_producer(commit: &[u8], tree: &[u8]) -> (valid: bool)
    ensures valid <==> (identifiers::oid(commit@) && identifiers::oid(tree@)),
{ identifiers::valid_oid(commit) && identifiers::valid_oid(tree) }

pub fn original_header(schema: &str, passed: bool, config: &str, expected: &str,
    absolute_host: bool, imported: bool, errored: bool) -> (valid: bool)
    ensures valid <==> (schema@ == "chrono-bootstrap-result/v1"@ && passed
        && config@ == expected@ && absolute_host && !imported && !errored),
{
    equal_text(schema, "chrono-bootstrap-result/v1") && passed && equal_text(config, expected)
        && absolute_host && !imported && !errored
}

pub fn operation_record_matches(tool: &str, expected_tool: &str,
    argv: &Vec<String>, expected_argv: &Vec<String>, cwd: &str, expected_cwd: &str,
    pid: u32, exit_code: i32) -> (valid: bool)
    ensures valid <==> (tool@ == expected_tool@ && same_name_list(argv@, expected_argv@)
        && cwd@ == expected_cwd@ && pid > 0 && exit_code == 0),
{
    equal_text(tool, expected_tool) && text_list_equal(argv, expected_argv)
        && equal_text(cwd, expected_cwd) && pid > 0 && exit_code == 0
}

pub fn stream_matches(digest: &str, expected_digest: &str, size: u64, expected_size: u64) -> (valid: bool)
    ensures valid <==> (digest@ == expected_digest@ && size == expected_size),
{ equal_text(digest, expected_digest) && size == expected_size }

pub fn participation_mode(coordinator: &str, absolute_git: bool) -> (valid: bool)
    ensures valid <==> (coordinator@ == "git-main-worktree"@ && absolute_git),
{
    equal_text(coordinator, "git-main-worktree") && absolute_git
}

pub fn bootstrap_uses_local_owner(main: bool, local_present: bool) -> (local: bool)
    ensures local <==> (main || local_present),
{
    main || local_present
}

pub fn needs_configured_tool(prerequisite: bool, tool: &str) -> (required: bool)
    ensures required <==> (prerequisite || (tool@ != "cargo"@ && tool@ != "rustc"@)),
{
    prerequisite || (!equal_text(tool, "cargo") && !equal_text(tool, "rustc"))
}

pub fn unique_method(count: usize) -> (valid: bool)
    ensures valid <==> count == 1,
{ count == 1 }

pub fn profile_options(schema: &str, bound: bool, upgrade: bool, stage: bool, reuse_empty: bool) -> (valid: bool)
    ensures valid <==> (schema@ == "chrono-bootstrap/v1"@ && bound
        && !(upgrade && stage) && (reuse_empty || upgrade)),
{
    equal_text(schema, "chrono-bootstrap/v1") && bound && !(upgrade && stage) && (reuse_empty || upgrade)
}

pub open spec fn unique_names(names: Seq<String>) -> bool {
    (forall|i: int| 0 <= i < names.len() ==> names[i]@ != ""@)
        && (forall|i: int, j: int| 0 <= j < i < names.len() ==> names[i]@ != names[j]@)
}

pub fn valid_names(names: &Vec<String>) -> (valid: bool)
    ensures valid <==> unique_names(names@),
{
    let mut i = 0;
    while i < names.len()
        invariant
            0 <= i <= names.len(),
            forall|k: int| 0 <= k < i ==> names@[k]@ != ""@,
            forall|k: int, j: int| 0 <= j < k < i ==> names@[k]@ != names@[j]@,
        decreases names.len() - i,
    {
        if equal_text(names[i].as_str(), "") { return false; }
        let mut j = 0;
        while j < i
            invariant
                0 <= j <= i < names.len(),
                names@[i as int]@ != ""@,
                forall|k: int| 0 <= k < i ==> names@[k]@ != ""@,
                forall|k: int, earlier: int| 0 <= earlier < k < i ==> names@[k]@ != names@[earlier]@,
                forall|k: int| 0 <= k < j ==> names@[i as int]@ != names@[k]@,
            decreases i - j,
        {
            if equal_text(names[i].as_str(), names[j].as_str()) { return false; }
            j += 1;
        }
        i += 1;
    }
    true
}

pub open spec fn name_member(values: Seq<String>, target: Seq<char>) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i])@ == target
}

fn contains_name(values: &Vec<String>, target: &str) -> (found: bool)
    ensures found <==> name_member(values@, target@),
{
    let mut i = 0;
    while i < values.len()
        invariant 0 <= i <= values.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] values@[j])@ != target@,
        decreases values.len() - i,
    {
        if equal_text(values[i].as_str(), target) { return true; }
        i += 1;
    }
    false
}

pub fn names_subset(selected: &Vec<String>, available: &Vec<String>) -> (valid: bool)
    ensures valid <==> (forall|i: int| 0 <= i < selected@.len()
        ==> name_member(available@, (#[trigger] selected@[i])@)),
{
    let mut i = 0;
    while i < selected.len()
        invariant
            0 <= i <= selected.len(),
            forall|k: int| 0 <= k < i ==> name_member(available@, (#[trigger] selected@[k])@),
        decreases selected.len() - i,
    {
        if !contains_name(available, selected[i].as_str()) { return false; }
        i += 1;
    }
    true
}

pub open spec fn same_name_list(a: Seq<String>, b: Seq<String>) -> bool {
    a.len() == b.len() && forall|i: int| 0 <= i < a.len() ==> a[i]@ == b[i]@
}

pub fn text_list_equal(a: &Vec<String>, b: &Vec<String>) -> (equal: bool)
    ensures equal <==> same_name_list(a@, b@),
{
    if a.len() != b.len() { return false; }
    let mut i = 0;
    while i < a.len()
        invariant a.len() == b.len(), 0 <= i <= a.len(),
            forall|j: int| 0 <= j < i ==> a@[j]@ == b@[j]@,
        decreases a.len() - i,
    {
        if !equal_text(a[i].as_str(), b[i].as_str()) { return false; }
        i += 1;
    }
    true
}

pub fn all_successful(codes: &Vec<i32>) -> (success: bool)
    ensures success <==> (forall|i: int| 0 <= i < codes@.len() ==> codes@[i] == 0),
{
    let mut i = 0;
    while i < codes.len()
        invariant 0 <= i <= codes.len(), forall|j: int| 0 <= j < i ==> codes@[j] == 0,
        decreases codes.len() - i,
    {
        if codes[i] != 0 { return false; }
        i += 1;
    }
    true
}

pub fn construction_complete(expected: &Vec<String>, actual: &Vec<String>, codes: &Vec<i32>,
    expected_installations: &Vec<String>, installations: &Vec<String>,
    selected: bool, expected_stage: bool, observed_stage: bool) -> (complete: bool)
    ensures complete <==> (selected && same_name_list(expected@, actual@)
        && actual@.len() == codes@.len()
        && (forall|i: int| 0 <= i < codes@.len() ==> codes@[i] == 0)
        && same_name_list(expected_installations@, installations@)
        && expected_stage == observed_stage),
{
    selected && text_list_equal(expected, actual) && actual.len() == codes.len()
        && all_successful(codes) && text_list_equal(expected_installations, installations)
        && expected_stage == observed_stage
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "lowercase"))]
pub enum BuildStatus { Passed, Failed }

pub fn construction_status(failed: bool, complete: bool) -> (out: BuildStatus)
    ensures (out is Passed) <==> (!failed && complete),
        (out is Failed) <==> (failed || !complete),
{
    if !failed && complete { BuildStatus::Passed } else { BuildStatus::Failed }
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct SourceObservation {
    pub commit: Option<String>,
    pub tree: Option<String>,
    pub dirty: Option<bool>,
    pub error: Option<String>,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "lowercase"))]
pub enum SourceState { Unavailable, Changed, Dirty, Clean }

pub open spec fn optional_text_same(a: &Option<String>, b: &Option<String>) -> bool {
    (a is None && b is None)
        || (a is Some && b is Some && a.unwrap()@ == b.unwrap()@)
}

pub fn optional_text_equal(a: &Option<String>, b: &Option<String>) -> (equal: bool)
    ensures equal <==> optional_text_same(a, b),
{
    match (a, b) {
        (Some(left), Some(right)) => equal_text(left.as_str(), right.as_str()),
        (None, None) => true,
        _ => false,
    }
}

pub open spec fn source_observations_same(a: &SourceObservation, b: &SourceObservation) -> bool {
    optional_text_same(&a.commit, &b.commit) && optional_text_same(&a.tree, &b.tree)
        && a.dirty == b.dirty && optional_text_same(&a.error, &b.error)
}

pub fn observation_matches(a: &SourceObservation, b: &SourceObservation) -> (same: bool)
    ensures same <==> source_observations_same(a, b),
{
    optional_text_equal(&a.commit, &b.commit) && optional_text_equal(&a.tree, &b.tree)
        && a.dirty == b.dirty && optional_text_equal(&a.error, &b.error)
}

pub open spec fn source_state_spec(before: &SourceObservation, after: &SourceObservation) -> SourceState {
    if before.error is Some || after.error is Some { SourceState::Unavailable }
    else if !source_observations_same(before, after) { SourceState::Changed }
    else if before.dirty == Some(true) { SourceState::Dirty }
    else { SourceState::Clean }
}

pub fn source_state(before: &SourceObservation, after: &SourceObservation) -> (out: SourceState)
    ensures out == source_state_spec(before, after),
{
    if before.error.is_some() || after.error.is_some() { SourceState::Unavailable }
    else if !optional_text_equal(&before.commit, &after.commit)
        || !optional_text_equal(&before.tree, &after.tree)
        || before.dirty != after.dirty { SourceState::Changed }
    else if before.dirty == Some(true) { SourceState::Dirty }
    else { SourceState::Clean }
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct SourceIdentity {
    pub state: SourceState,
    pub commit: Option<String>,
    pub tree: Option<String>,
    pub before: SourceObservation,
    pub after: SourceObservation,
}

pub fn original_source(identity: &SourceIdentity, expected: &SourceObservation) -> (valid: bool)
    ensures valid <==> (identity.state is Clean && expected.commit is Some && expected.tree is Some
        && expected.dirty == Some(false) && expected.error is None
        && optional_text_same(&identity.commit, &expected.commit)
        && optional_text_same(&identity.tree, &expected.tree)
        && source_observations_same(&identity.before, expected)
        && source_observations_same(&identity.after, expected)),
{
    matches!(identity.state, SourceState::Clean) && expected.commit.is_some() && expected.tree.is_some()
        && expected.dirty == Some(false) && expected.error.is_none()
        && optional_text_equal(&identity.commit, &expected.commit)
        && optional_text_equal(&identity.tree, &expected.tree)
        && observation_matches(&identity.before, expected) && observation_matches(&identity.after, expected)
}

fn copy_optional_text(value: &Option<String>) -> (out: Option<String>)
    ensures optional_text_same(value, &out),
{
    match value { Some(text) => Some(text.clone()), None => None }
}

pub fn source_identity(before: SourceObservation, after: SourceObservation) -> (out: SourceIdentity)
    ensures
        out.state == source_state_spec(&before, &after),
        out.before == before, out.after == after,
        out.state is Clean ==> optional_text_same(&out.commit, &before.commit)
            && optional_text_same(&out.tree, &before.tree),
        !(out.state is Clean) ==> out.commit is None && out.tree is None,
{
    let state = source_state(&before, &after);
    let (commit, tree) = if matches!(state, SourceState::Clean) {
        (copy_optional_text(&before.commit), copy_optional_text(&before.tree))
    } else { (None, None) };
    SourceIdentity { state, commit, tree, before, after }
}

pub fn process_exit(code: Option<i32>, signal: Option<i32>) -> (out: Option<i32>)
    ensures
        code is Some ==> out == code,
        code is None && signal is Some && signal.unwrap() > 0
            ==> out is Some && out.unwrap() == -signal.unwrap(),
        code is None && (signal is None || signal.unwrap() <= 0) ==> out is None,
{
    match code {
        Some(value) => Some(value),
        None => match signal {
            Some(value) => if value > 0 { Some(-value) } else { None },
            None => None,
        },
    }
}

pub enum ProbeAction { Ready, InstallMinimal, AddFormat }

pub fn probe_action(ready: bool, cargo: bool) -> (out: ProbeAction)
    ensures
        (out is Ready) <==> ready,
        (out is InstallMinimal) <==> (!ready && cargo),
        (out is AddFormat) <==> (!ready && !cargo),
{
    if ready { ProbeAction::Ready }
    else if cargo { ProbeAction::InstallMinimal }
    else { ProbeAction::AddFormat }
}

pub fn environment_key(bytes: &[u8]) -> (valid: bool)
    ensures valid <==> (bytes@.len() > 0
        && !(bytes@.len() >= 7 && bytes@[0] == 67 && bytes@[1] == 72
            && bytes@[2] == 82 && bytes@[3] == 79 && bytes@[4] == 78
            && bytes@[5] == 79 && bytes@[6] == 95)
        && forall|i: int| 0 <= i < bytes@.len() ==> bytes@[i] != 0 && bytes@[i] != 61),
{
    if bytes.len() == 0 || byte_prefix(bytes, &[67u8, 72, 82, 79, 78, 79, 95]) { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            forall|j: int| 0 <= j < i ==> bytes@[j] != 0 && bytes@[j] != 61,
        decreases bytes.len() - i,
    {
        if bytes[i] == 0 || bytes[i] == 61 { return false; }
        i += 1;
    }
    true
}

pub fn environment_value(bytes: &[u8]) -> (valid: bool)
    ensures valid <==> (forall|i: int| 0 <= i < bytes@.len() ==> bytes@[i] != 0),
{
    let mut i = 0;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            forall|j: int| 0 <= j < i ==> bytes@[j] != 0,
        decreases bytes.len() - i,
    {
        if bytes[i] == 0 { return false; }
        i += 1;
    }
    true
}

pub fn host_variable(key: &str) -> (valid: bool)
    ensures valid <==> (key@ == "HOME"@ || key@ == "CARGO_HOME"@ || key@ == "TMPDIR"@),
{
    equal_text(key, "HOME") || equal_text(key, "CARGO_HOME") || equal_text(key, "TMPDIR")
}

pub open spec fn bad_path_byte(bytes: Seq<u8>, i: int) -> bool {
    bytes[i] == 0 || bytes[i] == 10 || bytes[i] == 13 || bytes[i] == 92
    || (bytes[i] == 47 && (i == 0 || i + 1 == bytes.len() || bytes[i - 1] == 47))
    || (bytes[i] == 46 && (i == 0 || bytes[i - 1] == 47)
        && (i + 1 == bytes.len() || bytes[i + 1] == 47
            || (bytes[i + 1] == 46 && (i + 2 == bytes.len() || bytes[i + 2] == 47))))
}

pub fn host_path(bytes: &[u8], prefix: &[u8]) -> (valid: bool)
    ensures valid <==> (bytes@.len() > 0 && prefix@.len() <= bytes@.len()
        && (forall|i: int| 0 <= i < prefix@.len() ==> bytes@[i] == prefix@[i])
        && forall|i: int| 0 <= i < bytes@.len() ==> !bad_path_byte(bytes@, i)),
{
    if bytes.len() == 0 || !byte_prefix(bytes, prefix) { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant
            bytes.len() > 0,
            prefix.len() <= bytes.len(),
            forall|j: int| 0 <= j < prefix@.len() ==> bytes@[j] == prefix@[j],
            0 <= i <= bytes.len(),
            forall|j: int| 0 <= j < i ==> !bad_path_byte(bytes@, j),
        decreases bytes.len() - i,
    {
        let b = bytes[i];
        if b == 0 || b == 10 || b == 13 || b == 92 {
            assert(bad_path_byte(bytes@, i as int));
            return false;
        }
        if b == 47 && (i == 0 || i + 1 == bytes.len() || bytes[i - 1] == 47) {
            assert(bad_path_byte(bytes@, i as int));
            return false;
        }
        if b == 46 && (i == 0 || bytes[i - 1] == 47) {
            if i + 1 == bytes.len() || bytes[i + 1] == 47 {
                assert(bad_path_byte(bytes@, i as int));
                return false;
            }
            if bytes[i + 1] == 46 && (i + 2 == bytes.len() || bytes[i + 2] == 47) {
                assert(bad_path_byte(bytes@, i as int));
                return false;
            }
        }
        i += 1;
    }
    true
}

pub fn path_contains(parent: &[u8], child: &[u8]) -> (contains: bool)
    ensures contains <==> (parent@.len() <= child@.len()
        && (forall|i: int| 0 <= i < parent@.len() ==> parent@[i] == child@[i])
        && (parent@.len() == child@.len() || child@[parent@.len() as int] == 47)),
{
    byte_prefix(child, parent)
        && (parent.len() == child.len() || child[parent.len()] == 47)
}

pub fn disjoint_homes(a: &[u8], b: &[u8]) -> (valid: bool)
    ensures valid <==> (
        !(a@.len() <= b@.len()
            && (forall|i: int| 0 <= i < a@.len() ==> a@[i] == b@[i])
            && (a@.len() == b@.len() || b@[a@.len() as int] == 47))
        && !(b@.len() <= a@.len()
            && (forall|i: int| 0 <= i < b@.len() ==> b@[i] == a@[i])
            && (b@.len() == a@.len() || a@[b@.len() as int] == 47))),
{
    !path_contains(a, b) && !path_contains(b, a)
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize, serde::Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct NativePlatform {
    pub system: String,
    pub machine: String,
}

pub fn platform_matches(declared: Option<&NativePlatform>, native: &NativePlatform) -> (valid: bool)
    ensures valid <==> (declared is Some
        && declared.unwrap().system@ == native.system@
        && declared.unwrap().machine@ == native.machine@),
{
    match declared {
        None => false,
        Some(value) => equal_text(value.system.as_str(), native.system.as_str())
            && equal_text(value.machine.as_str(), native.machine.as_str()),
    }
}

pub fn registry_matches(schema: &str, version: &str, expected_version: &str) -> (valid: bool)
    ensures valid <==> (schema@ == "chrono-bootstrap-toolchains/v1"@ && version@ == expected_version@),
{
    equal_text(schema, "chrono-bootstrap-toolchains/v1") && equal_text(version, expected_version)
}

pub fn byte_prefix(bytes: &[u8], prefix: &[u8]) -> (matched: bool)
    ensures matched <==> (prefix@.len() <= bytes@.len()
        && forall|i: int| 0 <= i < prefix@.len() ==> bytes@[i] == prefix@[i]),
{
    if prefix.len() > bytes.len() { return false; }
    let mut i = 0;
    while i < prefix.len()
        invariant
            prefix.len() <= bytes.len(),
            0 <= i <= prefix.len(),
            forall|j: int| 0 <= j < i ==> bytes@[j] == prefix@[j],
        decreases prefix.len() - i,
    {
        if bytes[i] != prefix[i] { return false; }
        i += 1;
    }
    true
}

// prefix 是调用方组装的精确工具名、空格和工具链版本。
pub fn version_matches(bytes: &[u8], prefix: &[u8], allow_exact: bool) -> (valid: bool)
    ensures valid <==> (prefix@.len() <= bytes@.len()
        && (forall|i: int| 0 <= i < prefix@.len() ==> bytes@[i] == prefix@[i])
        && ((allow_exact && bytes@.len() == prefix@.len())
            || (bytes@.len() > prefix@.len() && bytes@[prefix@.len() as int] == 32))),
{
    byte_prefix(bytes, prefix) && ((allow_exact && bytes.len() == prefix.len())
        || (bytes.len() > prefix.len() && bytes[prefix.len()] == 32))
}

pub fn declaration_source(
    registry: bool, direct: bool, native: bool, required: bool, native_matches: bool,
) -> (out: DeclarationSource)
    ensures
        (out is Registry) <==> (registry && !direct && !native),
        (out is Direct) <==> (!registry && direct
            && (!required || (native && native_matches))),
        (out is Unbound) <==> (!registry && !direct && !required),
        (out is Reject) <==> ((registry && (direct || native))
            || (!registry && required && (!direct || !native || !native_matches))),
{
    if registry {
        if direct || native { DeclarationSource::Reject }
        else { DeclarationSource::Registry }
    } else if required && (!direct || !native || !native_matches) {
        DeclarationSource::Reject
    } else if direct { DeclarationSource::Direct }
    else { DeclarationSource::Unbound }
}

pub open spec fn hash_byte(b: u8) -> bool {
    (48 <= b <= 57) || (97 <= b <= 102)
}

pub fn valid_sha256(bytes: &[u8]) -> (valid: bool)
    ensures valid <==> (bytes@.len() == 64
        && forall|i: int| 0 <= i < bytes@.len() ==> hash_byte(bytes@[i])),
{
    if bytes.len() != 64 { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant
            bytes.len() == 64,
            0 <= i <= bytes.len(),
            forall|j: int| 0 <= j < i ==> hash_byte(bytes@[j]),
        decreases bytes.len() - i,
    {
        let b = bytes[i];
        if !((48 <= b && b <= 57) || (97 <= b && b <= 102)) { return false; }
        i += 1;
    }
    true
}

pub fn binding_valid(bytes: &[u8], registry: bool, has_path: bool) -> (valid: bool)
    ensures valid <==> (bytes@.len() == 64
        && (forall|i: int| 0 <= i < bytes@.len() ==> hash_byte(bytes@[i]))
        && (registry || has_path)),
{
    valid_sha256(bytes) && (registry || has_path)
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Deserialize, serde::Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Executable {
    pub path: String,
    pub sha256: String,
    pub version: String,
}

pub open spec fn same_payload(a: &Executable, b: &Executable) -> bool {
    a.sha256@ == b.sha256@ && a.version@ == b.version@
}

pub fn payload_equal(actual: &Executable, expected: &Executable) -> (equal: bool)
    ensures equal <==> same_payload(actual, expected),
{
    equal_text(actual.sha256.as_str(), expected.sha256.as_str())
        && equal_text(actual.version.as_str(), expected.version.as_str())
}

// expected.path 由调用方按宿主根解析；可重定位只放开路径，不放开字节和版本。
pub fn selected_matches(
    selected: &Executable, expected: &Executable, relocatable: bool,
) -> (valid: bool)
    ensures valid <==> (same_payload(selected, expected)
        && (relocatable || selected.path@ == expected.path@)),
{
    payload_equal(selected, expected)
        && (relocatable || equal_text(selected.path.as_str(), expected.path.as_str()))
}

pub enum ConfiguredAction { Observe, Retired, Missing, NotDeclared }

pub fn configured_action(
    configured: Option<&str>, declared_path: Option<&str>, relocatable: bool,
    registry: bool,
) -> (out: ConfiguredAction)
    ensures
        (out is Retired) <==> (configured is Some && declared_path is Some
            && relocatable && configured.unwrap()@ == declared_path.unwrap()@),
        (out is Observe) <==> (configured is Some
            && !(declared_path is Some && relocatable
                && configured.unwrap()@ == declared_path.unwrap()@)),
        (out is Missing) <==> (configured is None && !registry),
        (out is NotDeclared) <==> (configured is None && registry),
{
    match configured {
        None => {
            if registry { ConfiguredAction::NotDeclared }
            else { ConfiguredAction::Missing }
        },
        Some(actual) => {
            match declared_path {
                Some(expected) => {
                    if relocatable && equal_text(actual, expected) {
                        return ConfiguredAction::Retired;
                    }
                },
                None => {},
            }
            ConfiguredAction::Observe
        },
    }
}

// 调用方先按现行合同解析 selected／非空 inherited 的物理路径；空值原样保留。
pub fn compiler_selection(
    selected: &str, inherited: Option<&str>, wrapper: Option<&str>,
    workspace_wrapper: Option<&str>,
) -> (valid: bool)
    ensures valid <==> (
        (inherited is None || inherited.unwrap()@ == ""@
            || inherited.unwrap()@ == selected@)
        && (wrapper is None || wrapper.unwrap()@ == ""@)
        && (workspace_wrapper is None || workspace_wrapper.unwrap()@ == ""@)),
{
    match inherited {
        Some(path) => {
            if !equal_text(path, "") && !equal_text(path, selected) { return false; }
        },
        None => {},
    }
    match wrapper {
        Some(value) => { if !equal_text(value, "") { return false; } },
        None => {},
    }
    match workspace_wrapper {
        Some(value) => { if !equal_text(value, "") { return false; } },
        None => {},
    }
    true
}

}
