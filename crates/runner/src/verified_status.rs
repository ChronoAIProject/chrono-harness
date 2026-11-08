//! 实际 CI／协议结果状态及 finding 一致性。Serde 适配仍由外部库生成。
use core::cmp::Ordering;
use std::collections::BTreeMap;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

#[path = "verified_scope.rs"]
pub mod scopes;

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub enum Status {
    Passed,
    Failed,
    NotRequired,
    Blocked,
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "lowercase"))]
pub enum WireStatus {
    Pass,
    Warn,
    Fail,
    Error,
}

impl WireStatus {
    pub open spec fn rank_spec(&self) -> u8 {
        match self { Self::Pass => 0, Self::Warn => 1, Self::Fail => 2, Self::Error => 3 }
    }

    fn rank(&self) -> (rank: u8)
        ensures rank == self.rank_spec(),
    {
        match self { Self::Pass => 0, Self::Warn => 1, Self::Fail => 2, Self::Error => 3 }
    }

    pub open spec fn order_spec(&self, other: &Self) -> Ordering {
        if self.rank_spec() < other.rank_spec() { Ordering::Less }
        else if self.rank_spec() > other.rank_spec() { Ordering::Greater }
        else { Ordering::Equal }
    }

    pub open spec fn exit_spec(&self) -> i32 {
        match self { Self::Pass | Self::Warn => 0, Self::Fail => 1, Self::Error => 2 }
    }

    pub fn exit_code(&self) -> (exit: i32)
        ensures exit == self.exit_spec(),
    {
        match self { Self::Pass | Self::Warn => 0, Self::Fail => 1, Self::Error => 2 }
    }

    /// 汇总实际协议状态，精确保留严重程度较高的那个变体。
    pub fn maximum(self, other: Self) -> (out: Self)
        ensures out == (if self.rank_spec() >= other.rank_spec() { self } else { other }),
    {
        if self.rank() >= other.rank() { self } else { other }
    }
}

/// 普通响应和保留报告共用实际头部字段；不另加名称或协议内容限制。
pub fn wire_header_matches(
    protocol: &str, request_id: &str, judge_id: &str, status: &WireStatus, exit: i32,
    expected_protocol: &str, expected_request_id: &str, expected_judge_id: &str,
) -> (matches: bool)
    ensures matches <==> protocol@ == expected_protocol@
        && request_id@ == expected_request_id@ && judge_id@ == expected_judge_id@
        && exit == status.exit_spec(),
{
    identifiers::equal_text(protocol, expected_protocol)
        && identifiers::equal_text(request_id, expected_request_id)
        && identifiers::equal_text(judge_id, expected_judge_id)
        && exit == status.exit_code()
}

pub proof fn wire_status_order_laws(a: WireStatus, b: WireStatus, c: WireStatus)
    ensures
        (a.rank_spec() == b.rank_spec()) == (a == b),
        (a.order_spec(&b) == Ordering::Equal) == (a == b),
        (a.order_spec(&b) == Ordering::Less) == (b.order_spec(&a) == Ordering::Greater),
        a.order_spec(&b) == Ordering::Less && b.order_spec(&c) == Ordering::Less
            ==> a.order_spec(&c) == Ordering::Less,
{}

impl Clone for WireStatus {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        match self { Self::Pass => Self::Pass, Self::Warn => Self::Warn,
            Self::Fail => Self::Fail, Self::Error => Self::Error }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for WireStatus {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { *self == *other }
}

impl PartialEq for WireStatus {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (*self == *other),
    { self.rank() == other.rank() }
}

impl Eq for WireStatus {}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::OrdSpecImpl for WireStatus {
    open spec fn obeys_cmp_spec() -> bool { true }
    open spec fn cmp_spec(&self, other: &Self) -> Ordering { self.order_spec(other) }
}

impl Ord for WireStatus {
    fn cmp(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        let rank = self.rank();
        let other_rank = other.rank();
        if rank < other_rank { Ordering::Less }
        else if rank > other_rank { Ordering::Greater }
        else { Ordering::Equal }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialOrdSpecImpl for WireStatus {
    open spec fn obeys_partial_cmp_spec() -> bool { true }
    open spec fn partial_cmp_spec(&self, other: &Self) -> Option<Ordering> {
        Some(self.order_spec(other))
    }
}

impl PartialOrd for WireStatus {
    fn partial_cmp(&self, other: &Self) -> (order: Option<Ordering>)
        ensures order == Some(self.order_spec(other)),
    { Some(self.cmp(other)) }
}

/// 实际树键按完整 UTF-8 内容比较；不归一化路径。
#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
pub struct TreePath {
    pub text: String,
}

#[cfg(verus_keep_ghost)]
impl View for TreePath {
    type V = Seq<char>;
    open spec fn view(&self) -> Seq<char> { self.text@ }
}

impl TreePath {
    pub open spec fn bytes(&self) -> Seq<u8> { vstd::utf8::encode_utf8(self.text@) }
    pub open spec fn same(&self, other: &Self) -> bool { self.bytes() == other.bytes() }
    pub open spec fn order_spec(&self, other: &Self) -> Ordering {
        if self.same(other) { Ordering::Equal }
        else if identifiers::byte_lt(self.bytes(), other.bytes()) { Ordering::Less }
        else { Ordering::Greater }
    }
    pub fn as_str(&self) -> (out: &str)
        ensures out@ == self@, out.spec_bytes() == self.bytes(),
    { self.text.as_str() }
}

impl Clone for TreePath {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    { Self { text: self.text.clone() } }
}

impl TreePath {
    pub fn from_owned(text: String) -> (out: Self)
        ensures out.text == text,
    { Self { text } }

    pub fn from_text(text: &str) -> (out: Self)
        ensures out@ == text@, out.bytes() == text.spec_bytes(),
    { Self::from_owned(text.to_owned()) }
}

impl std::ops::Deref for TreePath {
    type Target = str;
    fn deref(&self) -> (out: &str)
        ensures out@ == self@, out.spec_bytes() == self.bytes(),
    { self.as_str() }
}

impl std::borrow::Borrow<str> for TreePath {
    fn borrow(&self) -> (out: &str)
        ensures ({ let text: &str = out; text@ == self@ && text.spec_bytes() == self.bytes() }),
    { self.as_str() }
}

impl std::borrow::Borrow<String> for TreePath {
    fn borrow(&self) -> (out: &String)
        ensures ({ let text: &String = out; *text == self.text }),
    { &self.text }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for TreePath {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self.same(other) }
}

impl PartialEq for TreePath {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == self.same(other),
    { identifiers::equal_bytes(self.as_str().as_bytes(), other.as_str().as_bytes()) }
}

impl Eq for TreePath {}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::OrdSpecImpl for TreePath {
    open spec fn obeys_cmp_spec() -> bool { true }
    open spec fn cmp_spec(&self, other: &Self) -> Ordering { self.order_spec(other) }
}

impl Ord for TreePath {
    fn cmp(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        let order = identifiers::compare_bytes(self.as_str().as_bytes(), other.as_str().as_bytes());
        proof {
            identifiers::byte_lt_asymmetric(self.bytes(), other.bytes());
            identifiers::byte_lt_asymmetric(other.bytes(), self.bytes());
        }
        if order == 0 { Ordering::Equal }
        else if order < 0 { Ordering::Less }
        else { Ordering::Greater }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialOrdSpecImpl for TreePath {
    open spec fn obeys_partial_cmp_spec() -> bool { true }
    open spec fn partial_cmp_spec(&self, other: &Self) -> Option<Ordering> {
        Some(self.order_spec(other))
    }
}

impl PartialOrd for TreePath {
    fn partial_cmp(&self, other: &Self) -> (order: Option<Ordering>)
        ensures order == Some(self.order_spec(other)),
    { Some(self.cmp(other)) }
}

pub proof fn tree_path_order_laws(a: &TreePath, b: &TreePath, c: &TreePath)
    ensures
        (a.order_spec(b) == Ordering::Equal) == a.same(b),
        (a.order_spec(b) == Ordering::Less) == (b.order_spec(a) == Ordering::Greater),
        a.order_spec(b) == Ordering::Less && b.order_spec(c) == Ordering::Less
            ==> a.order_spec(c) == Ordering::Less,
{
    identifiers::byte_trichotomy(a.bytes(), b.bytes());
    identifiers::byte_lt_asymmetric(a.bytes(), b.bytes());
    identifiers::byte_lt_asymmetric(b.bytes(), a.bytes());
    if identifiers::byte_lt(a.bytes(), b.bytes()) { identifiers::byte_lt_strict(a.bytes(), b.bytes()); }
    if identifiers::byte_lt(b.bytes(), a.bytes()) { identifiers::byte_lt_strict(b.bytes(), a.bytes()); }
    if a.order_spec(b) == Ordering::Less && b.order_spec(c) == Ordering::Less {
        identifiers::byte_lt_transitive(a.bytes(), b.bytes(), c.bytes());
        identifiers::byte_lt_strict(a.bytes(), c.bytes());
    }
}

pub proof fn tree_path_obeys_cmp()
    ensures vstd::laws_cmp::obeys_cmp::<TreePath>(),
{
    use vstd::std_specs::cmp::{PartialEqSpec, PartialOrdSpec, OrdSpec};
    reveal(vstd::laws_eq::obeys_eq_spec_properties);
    reveal(vstd::laws_cmp::obeys_partial_cmp_spec_properties);
    reveal(vstd::laws_cmp::obeys_cmp_partial_ord);
    reveal(vstd::laws_cmp::obeys_cmp_ord);
    assert forall|a: TreePath, b: TreePath|
        a.partial_cmp_spec(&b) == Some(Ordering::Less)
            <==> b.partial_cmp_spec(&a) == Some(Ordering::Greater) by {
        tree_path_order_laws(&a, &b, &b);
    }
    assert forall|a: TreePath, b: TreePath, c: TreePath|
        a.partial_cmp_spec(&b) == Some(Ordering::Less)
            && b.partial_cmp_spec(&c) == Some(Ordering::Less)
            implies a.partial_cmp_spec(&c) == Some(Ordering::Less) by {
        tree_path_order_laws(&a, &b, &c);
    }
    assert forall|a: TreePath, b: TreePath, c: TreePath|
        a.partial_cmp_spec(&b) == Some(Ordering::Greater)
            && b.partial_cmp_spec(&c) == Some(Ordering::Greater)
            implies a.partial_cmp_spec(&c) == Some(Ordering::Greater) by {
        tree_path_order_laws(&c, &b, &a);
    }
}

pub type Tree = BTreeMap<TreePath, TreeEntry>;

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
pub struct TreeEntry {
    pub mode: String,
    pub oid: String,
    pub kind: String,
}

pub open spec fn tree_entry_equal(left: &TreeEntry, right: &TreeEntry) -> bool {
    left.mode@ == right.mode@ && left.oid@ == right.oid@ && left.kind@ == right.kind@
}

impl Clone for TreeEntry {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        Self { mode: self.mode.clone(), oid: self.oid.clone(), kind: self.kind.clone() }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for TreeEntry {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { tree_entry_equal(self, other) }
}

impl PartialEq for TreeEntry {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == tree_entry_equal(self, other),
    {
        identifiers::equal_text(self.mode.as_str(), other.mode.as_str())
            && identifiers::equal_text(self.oid.as_str(), other.oid.as_str())
            && identifiers::equal_text(self.kind.as_str(), other.kind.as_str())
    }
}

impl Eq for TreeEntry {}

pub open spec fn tree_entries_equal(old: Option<&TreeEntry>, new: Option<&TreeEntry>) -> bool {
    match (old, new) {
        (None, None) => true,
        (Some(a), Some(b)) => tree_entry_equal(a, b),
        _ => false,
    }
}

fn unchanged_tree_entries(old: Option<&TreeEntry>, new: Option<&TreeEntry>) -> (equal: bool)
    ensures equal == tree_entries_equal(old, new),
{
    match (old, new) {
        (None, None) => true,
        (Some(a), Some(b)) => a.eq(b),
        _ => false,
    }
}

pub open spec fn entry_blob(entry: Option<&TreeEntry>) -> Option<Seq<char>> {
    match entry { None => None, Some(entry) => Some(entry.oid@) }
}

pub open spec fn entry_mode(entry: Option<&TreeEntry>) -> Option<Seq<char>> {
    match entry { None => None, Some(entry) => Some(entry.mode@) }
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Delta {
    pub kind: String,
    pub path: String,
    pub old_blob: Option<String>,
    pub new_blob: Option<String>,
    pub old_mode: Option<String>,
    pub new_mode: Option<String>,
}

fn clone_optional_text(value: &Option<String>) -> (out: Option<String>)
    ensures out == *value,
{
    match value { None => None, Some(text) => Some(text.clone()) }
}

#[verifier::inline]
pub open spec fn optional_text(value: &Option<String>) -> Option<Seq<char>> {
    match value { None => None, Some(text) => Some(text@) }
}

/// 直接读取两端实际条目；kind 变化也属于修改，即使 blob 和 mode 相同。
pub fn delta_for_entry(path: &str, old: Option<&TreeEntry>, new: Option<&TreeEntry>) -> (out: Option<Delta>)
    ensures
        out.is_none() == tree_entries_equal(old, new),
        match out {
            None => true,
            Some(change) => change.path@ == path@
                && change.kind@ == (if old.is_none() { "A"@ }
                    else if new.is_none() { "D"@ } else { "M"@ })
                && optional_text(&change.old_blob) == entry_blob(old)
                && optional_text(&change.new_blob) == entry_blob(new)
                && optional_text(&change.old_mode) == entry_mode(old)
                && optional_text(&change.new_mode) == entry_mode(new),
        },
{
    if unchanged_tree_entries(old, new) { return None; }
    let kind = if old.is_none() { "A" } else if new.is_none() { "D" } else { "M" };
    Some(Delta {
        kind: kind.to_owned(), path: path.to_owned(),
        old_blob: match old { None => None, Some(entry) => Some(entry.oid.clone()) },
        new_blob: match new { None => None, Some(entry) => Some(entry.oid.clone()) },
        old_mode: match old { None => None, Some(entry) => Some(entry.mode.clone()) },
        new_mode: match new { None => None, Some(entry) => Some(entry.mode.clone()) },
    })
}

fn optional_text_equal(left: &Option<String>, right: &Option<String>) -> (equal: bool)
    ensures equal == (optional_text(left) == optional_text(right)),
{
    match (left, right) {
        (Some(a), Some(b)) => identifiers::equal_text(a.as_str(), b.as_str()),
        (None, None) => true,
        _ => false,
    }
}

impl Clone for Delta {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        Self { kind: self.kind.clone(), path: self.path.clone(),
            old_blob: clone_optional_text(&self.old_blob),
            new_blob: clone_optional_text(&self.new_blob),
            old_mode: clone_optional_text(&self.old_mode),
            new_mode: clone_optional_text(&self.new_mode) }
    }
}

#[verifier::inline]
pub open spec fn delta_equal(left: &Delta, right: &Delta) -> bool {
    left.kind@ == right.kind@ && left.path@ == right.path@
        && optional_text(&left.old_blob) == optional_text(&right.old_blob)
        && optional_text(&left.new_blob) == optional_text(&right.new_blob)
        && optional_text(&left.old_mode) == optional_text(&right.old_mode)
        && optional_text(&left.new_mode) == optional_text(&right.new_mode)
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Delta {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { delta_equal(self, other) }
}

impl PartialEq for Delta {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == delta_equal(self, other),
    {
        identifiers::equal_text(self.kind.as_str(), other.kind.as_str())
            && identifiers::equal_text(self.path.as_str(), other.path.as_str())
            && optional_text_equal(&self.old_blob, &other.old_blob)
            && optional_text_equal(&self.new_blob, &other.new_blob)
            && optional_text_equal(&self.old_mode, &other.old_mode)
            && optional_text_equal(&self.new_mode, &other.new_mode)
    }
}

impl Eq for Delta {}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct Finding {
    pub code: String,
    pub level: String,
    pub message: String,
    pub delta_refs: Vec<String>,
    pub causes: Vec<String>,
}

/// 记录已经分类的问题；调用方仍负责选择严重程度和字段。
pub fn append_finding(
    status: &mut WireStatus, findings: &mut Vec<Finding>, severity: WireStatus, finding: Finding,
)
    ensures
        *status == (if old(status).rank_spec() >= severity.rank_spec() { *old(status) } else { severity }),
        findings@ == old(findings)@.push(finding),
{
    *status = status.clone().maximum(severity);
    findings.push(finding);
}

pub open spec fn finding_fields(finding: &Finding) -> bool {
    finding.code@.len() > 0 && finding.message@.len() > 0
        && (finding.level@ == "info"@ || finding.level@ == "warning"@
            || finding.level@ == "error"@)
        && finding.delta_refs@.len() > 0
}

/// 只判断原协议要求的字段形状；引用是否属于实际请求仍由该请求检查。
pub fn valid_finding_fields(finding: &Finding) -> (valid: bool)
    ensures valid == finding_fields(finding),
{
    !finding.code.as_str().is_empty() && !finding.message.as_str().is_empty()
        && (identifiers::equal_text(finding.level.as_str(), "info")
            || identifiers::equal_text(finding.level.as_str(), "warning")
            || identifiers::equal_text(finding.level.as_str(), "error"))
        && finding.delta_refs.len() > 0
}

#[verifier::inline]
pub open spec fn record_reference(reference: Seq<char>) -> bool {
    let bytes = vstd::utf8::encode_utf8(reference);
    bytes.len() > 0 && bytes[0] == 47
}

#[verifier::inline]
pub open spec fn reference_allowed(reference: Seq<char>, delta: Option<&[Delta]>) -> bool {
    record_reference(reference) || match delta {
        None => false,
        Some(entries) => exists|i: int| 0 <= i < entries.len()
            && #[trigger] entries[i].path@ == reference,
    }
}

/// None 对应初始库存；普通请求直接借用自己的完整 DELTA。
pub fn valid_finding_reference(reference: &str, delta: Option<&[Delta]>) -> (valid: bool)
    ensures valid == reference_allowed(reference@, delta),
{
    let bytes = reference.as_bytes();
    if bytes.len() > 0 && bytes[0] == 47 { return true; }
    match delta {
        None => false,
        Some(entries) => {
            let mut i = 0;
            while i < entries.len()
                invariant
                    0 <= i <= entries.len(),
                    delta == Some(entries),
                    !record_reference(reference@),
                    forall|j: int| 0 <= j < i ==> #[trigger] entries[j].path@ != reference@,
                decreases entries.len() - i,
            {
                if identifiers::equal_text(entries[i].path.as_str(), reference) {
                    assert(entries[i as int].path@ == reference@);
                    assert(exists|j: int| 0 <= j < entries.len()
                        && #[trigger] entries[j].path@ == reference@);
                    return true;
                }
                i += 1;
            }
            false
        }
    }
}

pub open spec fn finding_references(finding: &Finding, delta: Option<&[Delta]>) -> bool {
    forall|i: int| 0 <= i < finding.delta_refs.len()
        ==> reference_allowed(#[trigger] finding.delta_refs[i]@, delta)
}

pub fn valid_finding_references(finding: &Finding, delta: Option<&[Delta]>) -> (valid: bool)
    ensures valid == finding_references(finding, delta),
{
    let mut i = 0;
    while i < finding.delta_refs.len()
        invariant
            0 <= i <= finding.delta_refs.len(),
            forall|j: int| 0 <= j < i
                ==> reference_allowed(#[trigger] finding.delta_refs[j]@, delta),
        decreases finding.delta_refs.len() - i,
    {
        if !valid_finding_reference(finding.delta_refs[i].as_str(), delta) {
            return false;
        }
        i += 1;
    }
    true
}

pub open spec fn evidence_identity(path: &str, kind: &str, sha256: &str) -> bool {
    let prefix = ".chrono-harness/state/".spec_bytes();
    path.spec_bytes().len() >= prefix.len()
        && path.spec_bytes().take(prefix.len() as int) == prefix
        && kind@.len() > 0 && identifiers::digest_format(sha256.spec_bytes())
}

/// 保留原前缀合同，不在这里增加词法路径或文件存在条件。
pub fn valid_evidence_identity(path: &str, kind: &str, sha256: &str) -> (valid: bool)
    ensures valid == evidence_identity(path, kind, sha256),
{
    let bytes = path.as_bytes();
    let prefix = ".chrono-harness/state/".as_bytes();
    if bytes.len() < prefix.len() { return false; }
    let (head, _) = bytes.split_at(prefix.len());
    identifiers::equal_bytes(head, prefix) && !kind.is_empty()
        && identifiers::valid_digest_text(sha256)
}

impl Clone for Finding {
    fn clone(&self) -> (out: Self)
        ensures out.code == self.code, out.level == self.level, out.message == self.message,
            out.delta_refs@ == self.delta_refs@, out.causes@ == self.causes@,
    {
        Self { code: self.code.clone(), level: self.level.clone(), message: self.message.clone(),
            delta_refs: self.delta_refs.clone(), causes: self.causes.clone() }
    }
}

pub open spec fn finding_level_present(findings: Seq<Finding>, level: Seq<char>) -> bool {
    exists|i: int| 0 <= i < findings.len() && (#[trigger] findings[i]).level@ == level
}

proof fn finding_level_prefix_step(findings: Seq<Finding>, level: Seq<char>, at: int)
    requires 0 <= at < findings.len(),
    ensures finding_level_present(findings.take(at + 1), level)
        == (finding_level_present(findings.take(at), level) || findings[at].level@ == level),
{
    let before = findings.take(at);
    let after = findings.take(at + 1);
    if finding_level_present(after, level) {
        let i = choose|i: int| 0 <= i < after.len() && (#[trigger] after[i]).level@ == level;
        if i < at {
            assert(before[i].level@ == level);
            assert(finding_level_present(before, level));
        } else {
            assert(i == at && findings[at].level@ == level);
        }
    }
    if finding_level_present(before, level) {
        let i = choose|i: int| 0 <= i < before.len() && (#[trigger] before[i]).level@ == level;
        assert(after[i].level@ == level);
        assert(finding_level_present(after, level));
    }
    if findings[at].level@ == level {
        assert(after[at].level@ == level);
        assert(finding_level_present(after, level));
    }
}

pub open spec fn finding_status_agreement(status: &WireStatus, findings: Seq<Finding>) -> bool {
    let errors = finding_level_present(findings, "error"@);
    let warnings = finding_level_present(findings, "warning"@);
    (matches!(status, WireStatus::Fail | WireStatus::Error) == errors)
        && (*status == WireStatus::Pass ==> !warnings)
        && (*status == WireStatus::Warn ==> warnings)
}

/// 直接检查真实 finding；字段有效性和响应身份另由各自所有者检查。
pub fn findings_match_status(status: &WireStatus, findings: &[Finding]) -> (matches: bool)
    ensures matches == finding_status_agreement(status, findings@),
{
    let mut at = 0;
    let mut errors = false;
    let mut warnings = false;
    while at < findings.len()
        invariant at <= findings.len(),
            errors == finding_level_present(findings@.take(at as int), "error"@),
            warnings == finding_level_present(findings@.take(at as int), "warning"@),
        decreases findings.len() - at,
    {
        let level = findings[at].level.as_str();
        errors = errors || identifiers::equal_text(level, "error");
        warnings = warnings || identifiers::equal_text(level, "warning");
        proof {
            finding_level_prefix_step(findings@, "error"@, at as int);
            finding_level_prefix_step(findings@, "warning"@, at as int);
        }
        at += 1;
    }
    assert(findings@.take(at as int) =~= findings@);
    match status {
        WireStatus::Pass => !errors && !warnings,
        WireStatus::Warn => !errors && warnings,
        WireStatus::Fail | WireStatus::Error => errors,
    }
}

pub open spec fn response_findings_valid(findings: Seq<Finding>, delta: Option<&[Delta]>) -> bool {
    forall|i: int| 0 <= i < findings.len()
        ==> finding_fields(&(#[trigger] findings[i])) && finding_references(&findings[i], delta)
}

/// 同时核对每条真实 finding 的字段和引用，不接受预先计算的有效标志。
pub fn valid_response_findings(findings: &[Finding], delta: Option<&[Delta]>) -> (valid: bool)
    ensures valid == response_findings_valid(findings@, delta),
{
    let mut i = 0;
    while i < findings.len()
        invariant
            0 <= i <= findings.len(),
            forall|j: int| 0 <= j < i
                ==> finding_fields(&(#[trigger] findings@[j]))
                    && finding_references(&findings@[j], delta),
        decreases findings.len() - i,
    {
        if !valid_finding_fields(&findings[i]) || !valid_finding_references(&findings[i], delta) {
            return false;
        }
        i += 1;
    }
    true
}

pub open spec fn response_fields_error(
    protocol: &str, request_id: &str, judge_id: &str, status: &WireStatus, exit: i32,
    expected_protocol: &str, expected_request_id: &str, expected_judge_id: &str,
    findings: Seq<Finding>, delta: Option<&[Delta]>,
) -> Option<Seq<char>> {
    if protocol@ != expected_protocol@ || request_id@ != expected_request_id@
        || judge_id@ != expected_judge_id@ || exit != status.exit_spec() {
        Some("E_PROTOCOL: response identity/status/exit mismatch"@)
    } else if !response_findings_valid(findings, delta) {
        Some("E_PROTOCOL: invalid finding"@)
    } else if !finding_status_agreement(status, findings) {
        Some("E_PROTOCOL: finding/status disagreement"@)
    } else { None }
}

/// 普通与初始响应共用原错误优先级；成功后仍须由证据 IO 负责方完成验证。
pub fn validate_response_fields(
    protocol: &str, request_id: &str, judge_id: &str, status: &WireStatus, exit: i32,
    expected_protocol: &str, expected_request_id: &str, expected_judge_id: &str,
    findings: &[Finding], delta: Option<&[Delta]>,
) -> (out: Result<(), String>)
    ensures
        out.is_ok() == response_fields_error(protocol, request_id, judge_id, status, exit,
            expected_protocol, expected_request_id, expected_judge_id, findings@, delta).is_none(),
        match out {
            Ok(()) => true,
            Err(message) => response_fields_error(protocol, request_id, judge_id, status, exit,
                expected_protocol, expected_request_id, expected_judge_id, findings@, delta)
                    == Some(message@),
        },
{
    if !wire_header_matches(protocol, request_id, judge_id, status, exit,
        expected_protocol, expected_request_id, expected_judge_id) {
        return Err("E_PROTOCOL: response identity/status/exit mismatch".to_string());
    }
    if !valid_response_findings(findings, delta) {
        return Err("E_PROTOCOL: invalid finding".to_string());
    }
    if !findings_match_status(status, findings) {
        return Err("E_PROTOCOL: finding/status disagreement".to_string());
    }
    Ok(())
}

pub open spec fn record_response_error(
    protocol: &str, request_id: &str, judge_id: &str, status: &WireStatus, exit: i32,
    expected_protocol: &str, expected_request_id: &str, expected_judge_id: &str,
    findings: Seq<Finding>,
) -> Option<Seq<char>> {
    if protocol@ != expected_protocol@ || request_id@ != expected_request_id@
        || judge_id@ != expected_judge_id@ || exit != status.exit_spec() {
        Some("response identity/status/exit mismatch"@)
    } else if !finding_status_agreement(status, findings) {
        Some("response findings/status disagree"@)
    } else { None }
}

/// 留存报告的头部与状态审计；原件、JSON 与其它协议条件仍由其负责方处理。
pub fn validate_record_response(
    protocol: &str, request_id: &str, judge_id: &str, status: &WireStatus, exit: i32,
    expected_protocol: &str, expected_request_id: &str, expected_judge_id: &str,
    findings: &[Finding],
) -> (out: Result<(), String>)
    ensures
        out.is_ok() == record_response_error(protocol, request_id, judge_id, status, exit,
            expected_protocol, expected_request_id, expected_judge_id, findings@).is_none(),
        match out {
            Ok(()) => true,
            Err(message) => record_response_error(protocol, request_id, judge_id, status, exit,
                expected_protocol, expected_request_id, expected_judge_id, findings@)
                    == Some(message@),
        },
{
    if !wire_header_matches(protocol, request_id, judge_id, status, exit,
        expected_protocol, expected_request_id, expected_judge_id) {
        return Err("response identity/status/exit mismatch".to_string());
    }
    if !findings_match_status(status, findings) {
        return Err("response findings/status disagree".to_string());
    }
    Ok(())
}

impl Clone for Status {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        match self {
            Self::Passed => Self::Passed,
            Self::Failed => Self::Failed,
            Self::NotRequired => Self::NotRequired,
            Self::Blocked => Self::Blocked,
        }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Status {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { *self == *other }
}

impl PartialEq for Status {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (*self == *other),
    {
        match self {
            Self::Passed => matches!(other, Self::Passed),
            Self::Failed => matches!(other, Self::Failed),
            Self::NotRequired => matches!(other, Self::NotRequired),
            Self::Blocked => matches!(other, Self::Blocked),
        }
    }
}

impl Eq for Status {}

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct CheckResult {
    pub id: String,
    pub status: Status,
    pub cause: String,
    pub exit_code: Option<i32>,
}

impl Clone for CheckResult {
    fn clone(&self) -> (out: Self)
        ensures out.id@ == self.id@, out.status == self.status,
            out.cause@ == self.cause@, out.exit_code == self.exit_code,
    {
        Self {
            id: self.id.clone(),
            status: self.status.clone(),
            cause: self.cause.clone(),
            exit_code: self.exit_code,
        }
    }
}

}
