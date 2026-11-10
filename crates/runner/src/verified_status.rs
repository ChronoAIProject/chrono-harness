//! 实际 CI／协议结果状态及 finding 一致性。Serde 适配仍由外部库生成。
use core::cmp::Ordering;
use std::collections::BTreeMap;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::cmp::OrdSpec;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

#[cfg(verus_keep_ghost)]
#[path = "verified_paths.rs"]
pub mod paths;
#[cfg(not(verus_keep_ghost))]
use crate::verified_paths as paths;

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

pub open spec fn tree_rows_view(rows: Seq<(&TreePath, &TreeEntry)>) -> Seq<(TreePath, TreeEntry)> {
    rows.map(|i: int, row: (&TreePath, &TreeEntry)| (*row.0, *row.1))
}

pub open spec fn tree_rows_ordered(rows: Seq<(&TreePath, &TreeEntry)>) -> bool {
    forall|i: int, j: int| 0 <= i < j < rows.len()
        ==> identifiers::byte_lt((#[trigger] rows[i]).0.bytes(), (#[trigger] rows[j]).0.bytes())
}

/// 借用真实树的全部原键和值，保留字节严格顺序。
fn tree_rows(tree: &Tree) -> (rows: Vec<(&TreePath, &TreeEntry)>)
    ensures
        tree_rows_view(rows@).to_set() == tree@.kv_pairs(),
        tree_rows_ordered(rows@),
{
    proof { tree_path_obeys_cmp(); }
    let mut iterator = tree.iter();
    let ghost all = iterator@.1;
    let mut rows: Vec<(&TreePath, &TreeEntry)> = Vec::new();
    loop
        invariant
            iterator@.1 == all,
            iterator@.0 == rows.len(),
            0 <= iterator@.0 <= all.len(),
            tree_rows_view(rows@) == all.take(iterator@.0),
            all.to_set() == tree@.kv_pairs(),
            vstd::std_specs::btree::increasing_seq(all.map(|i: int, row: (TreePath, TreeEntry)| row.0)),
        decreases all.len() - rows.len(),
    {
        match iterator.next() {
            None => {
                proof {
                    tree_path_obeys_cmp();
                    let keys = all.map(|i: int, row: (TreePath, TreeEntry)| row.0);
                    vstd::std_specs::btree::axiom_increasing_seq_meaning(keys);
                    assert(tree_rows_view(rows@) == all);
                    assert forall|i: int, j: int| 0 <= i < j < rows.len()
                        implies identifiers::byte_lt((#[trigger] rows@[i]).0.bytes(),
                            (#[trigger] rows@[j]).0.bytes()) by {
                        assert(keys[i] == all[i].0);
                        assert(keys[j] == all[j].0);
                        assert(keys[i].cmp_spec(&keys[j]) is Less);
                        assert(all[i].0.order_spec(&all[j].0) is Less);
                        identifiers::byte_lt_strict(all[i].0.bytes(), all[j].0.bytes());
                    }
                }
                return rows;
            }
            Some((key, value)) => {
                rows.push((key, value));
                proof { assert(tree_rows_view(rows@) =~= all.take(iterator@.0)); }
            }
        }
    }
}

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

#[cfg(verus_keep_ghost)]
pub type DeltaFields = (Seq<char>, Seq<char>, Option<Seq<char>>, Option<Seq<char>>,
    Option<Seq<char>>, Option<Seq<char>>);

pub open spec fn delta_record_view(change: Delta) -> DeltaFields {
    (change.path@, change.kind@, optional_text(&change.old_blob), optional_text(&change.new_blob),
        optional_text(&change.old_mode), optional_text(&change.new_mode))
}

pub open spec fn delta_records_view(changes: Seq<Delta>) -> Seq<DeltaFields> {
    changes.map(|i: int, change: Delta| delta_record_view(change))
}

pub open spec fn entry_delta_fields(path: Seq<char>, old_entry: Option<&TreeEntry>,
    new_entry: Option<&TreeEntry>) -> Seq<DeltaFields>
{
    if tree_entries_equal(old_entry, new_entry) { seq![] }
    else { seq![(path,
        if old_entry.is_none() { "A"@ } else if new_entry.is_none() { "D"@ } else { "M"@ },
        entry_blob(old_entry), entry_blob(new_entry), entry_mode(old_entry), entry_mode(new_entry))] }
}

#[cfg(verus_keep_ghost)]
pub type TreeEntryFields = (Seq<char>, Seq<char>, Seq<char>);

pub open spec fn tree_entry_fields(entry: &TreeEntry) -> TreeEntryFields {
    (entry.mode@, entry.oid@, entry.kind@)
}

/// 标准迭代器的完整、严格有序库存连接实际 map 键与全文身份。
/// 只创建迭代器，不遍历或复制库存。
fn tree_keys_have_unique_text(tree: &Tree)
    ensures forall|a: TreePath, b: TreePath| #![trigger tree@.contains_key(a), tree@.contains_key(b)]
        tree@.contains_key(a) && tree@.contains_key(b) && a@ == b@ ==> a == b,
{
    proof { tree_path_obeys_cmp(); }
    let iterator = tree.iter();
    proof {
        let rows = iterator@.1;
        let keys = rows.map(|i: int, row: (TreePath, TreeEntry)| row.0);
        vstd::std_specs::btree::axiom_increasing_seq_meaning(keys);
        assert forall|a: TreePath, b: TreePath| #![trigger tree@.contains_key(a), tree@.contains_key(b)]
            tree@.contains_key(a) && tree@.contains_key(b) && a@ == b@
                implies a == b by {
            assert(rows.to_set().contains((a, tree@[a])));
            assert(rows.to_set().contains((b, tree@[b])));
            let i = choose|i: int| 0 <= i < rows.len() && rows[i] == (a, tree@[a]);
            let j = choose|j: int| 0 <= j < rows.len() && rows[j] == (b, tree@[b]);
            if i < j {
                assert(keys[i].cmp_spec(&keys[j]) is Less);
                assert(a.order_spec(&b) is Less);
            } else if j < i {
                assert(keys[j].cmp_spec(&keys[i]) is Less);
                assert(b.order_spec(&a) is Less);
            }
        }
    }
}

/// Git tree 的实际插入状态：完整构造条目，原路径已存在时保留原重复诊断。
/// 沿用原 insert 的更新顺序；失败时该路径也已替换，调用方丢弃整棵候选树。
pub fn insert_git_tree_entry(
    tree: &mut Tree, path: TreePath, mode: &str, oid: &str, kind: &str,
) -> (out: Result<(), String>)
    ensures
        tree@.contains_key(path),
        tree@ == old(tree)@.insert(path, tree@[path]),
        tree_entry_fields(&tree@[path]) == (mode@, oid@, kind@),
        out.is_ok() == !old(tree)@.contains_key(path),
        out.is_ok() == !(exists|key: TreePath| (#[trigger] old(tree)@.contains_key(key)) && key@ == path@),
        match out {
            Ok(()) => true,
            Err(message) => message@ == "duplicate Git tree path"@,
        },
{
    proof { tree_path_obeys_cmp(); }
    let ghost before = tree@;
    let entry = TreeEntry {
        mode: mode.to_owned(), oid: oid.to_owned(), kind: kind.to_owned(),
    };
    let previous = tree.insert(path, entry);
    tree_keys_have_unique_text(tree);
    proof {
        if exists|key: TreePath| (#[trigger] before.contains_key(key)) && key@ == path@ {
            let key = choose|key: TreePath| (#[trigger] before.contains_key(key)) && key@ == path@;
            assert(tree@.contains_key(key));
            assert(tree@.contains_key(path));
            assert(key == path);
            assert(before.contains_key(path));
        }
        if before.contains_key(path) {
            assert(exists|key: TreePath| (#[trigger] before.contains_key(key)) && key@ == path@);
        }
        assert(before.contains_key(path) ==
            (exists|key: TreePath| (#[trigger] before.contains_key(key)) && key@ == path@));
    }
    match previous {
        Some(_) => Err("duplicate Git tree path".to_owned()),
        None => Ok(()),
    }
}

#[cfg(verus_keep_ghost)]
pub enum GitInventoryErrorView {
    Message(Seq<char>),
    InvalidPath(Seq<u8>),
}

pub enum GitInventoryError<'a> {
    Message(String),
    InvalidPath(&'a str),
}

impl<'a> GitInventoryError<'a> {
    pub open spec fn view(&self) -> GitInventoryErrorView {
        match self {
            Self::Message(message) => GitInventoryErrorView::Message(message@),
            Self::InvalidPath(path) => GitInventoryErrorView::InvalidPath(path.spec_bytes()),
        }
    }

    /// 唯一外部边界是标准 Debug 格式化；仓内分支及原路径转交在此证明。
    pub fn render<F: Fn(&str) -> String>(self, format_path: F) -> (out: String)
        requires forall|path: &str| format_path.requires((path,)),
        ensures match self {
            Self::Message(message) => out == message,
            Self::InvalidPath(path) => format_path.ensures((path,), out),
        },
    {
        match self {
            Self::Message(message) => message,
            Self::InvalidPath(path) => format_path(path),
        }
    }
}

pub struct GitTreeRow<'a> {
    pub path: &'a str,
    pub mode: &'a str,
    pub oid: &'a str,
    pub kind: &'a str,
}

#[cfg(verus_keep_ghost)]
pub type GitTreeRowBytes = (Seq<u8>, Seq<u8>, Seq<u8>, Seq<u8>);

impl<'a> GitTreeRow<'a> {
    pub open spec fn bytes(&self) -> GitTreeRowBytes {
        (self.path.spec_bytes(), self.mode.spec_bytes(), self.oid.spec_bytes(), self.kind.spec_bytes())
    }
}

/// 输入字节独立定义原准入次序：首 TAB、三字段、路径、完整 OID。
#[verifier::opaque]
pub open spec fn git_tree_row_spec(raw: Seq<u8>) -> Result<GitTreeRowBytes, GitInventoryErrorView> {
    let end = identifiers::ascii_separator_end(raw, 9);
    if end == raw.len() { Err(GitInventoryErrorView::Message("invalid Git tree row"@)) }
    else {
        let fields = identifiers::ascii_split_records(raw.take(end), 32, true);
        let path = raw.skip(end + 1);
        if fields.len() != 3 { Err(GitInventoryErrorView::Message("invalid Git tree metadata"@)) }
        else if !paths::relative_path(path) { Err(GitInventoryErrorView::InvalidPath(path)) }
        else if !identifiers::oid(fields[2]) {
            Err(GitInventoryErrorView::Message("expected full nonzero lowercase commit OID"@))
        } else { Ok((path, fields[0], fields[2], fields[1])) }
    }
}

pub fn parse_git_tree_row(raw: &str) -> (out: Result<GitTreeRow<'_>, GitInventoryError<'_>>)
    ensures match out {
        Ok(row) => git_tree_row_spec(raw.spec_bytes()) == Ok::<GitTreeRowBytes, GitInventoryErrorView>(row.bytes()),
        Err(error) => git_tree_row_spec(raw.spec_bytes()) == Err::<GitTreeRowBytes, GitInventoryErrorView>(error.view()),
    },
{
    proof { reveal(git_tree_row_spec); }
    let (meta, path) = match identifiers::git_inventory_row(raw, "invalid Git tree row") {
        Ok(parts) => parts,
        Err(message) => return Err(GitInventoryError::Message(message)),
    };
    let fields = identifiers::git_inventory_fields(meta);
    match identifiers::git_tree_metadata(&fields) {
        Ok(()) => {},
        Err(message) => return Err(GitInventoryError::Message(message)),
    }
    if !paths::valid_relative_path(path.as_bytes()) { return Err(GitInventoryError::InvalidPath(path)); }
    match identifiers::full_oid(fields[2]) {
        Ok(()) => {},
        Err(message) => return Err(GitInventoryError::Message(message)),
    }
    Ok(GitTreeRow { path, mode: fields[0], oid: fields[2], kind: fields[1] })
}

pub struct GitIndexRow<'a> {
    pub path: &'a str,
    pub mode: &'a str,
    pub oid: &'a str,
    pub stage: &'a str,
}

#[cfg(verus_keep_ghost)]
pub type GitIndexRowBytes = (Seq<u8>, Seq<u8>, Seq<u8>, Seq<u8>);

impl<'a> GitIndexRow<'a> {
    pub open spec fn bytes(&self) -> GitIndexRowBytes {
        (self.path.spec_bytes(), self.mode.spec_bytes(), self.oid.spec_bytes(), self.stage.spec_bytes())
    }
}

/// 实际 index 行的完整输入合同：首 TAB、三字段／单字节 stage、路径、完整 OID。
#[verifier::opaque]
pub open spec fn git_index_row_spec(raw: Seq<u8>) -> Result<GitIndexRowBytes, GitInventoryErrorView> {
    let end = identifiers::ascii_separator_end(raw, 9);
    if end == raw.len() { Err(GitInventoryErrorView::Message("invalid Git index row"@)) }
    else {
        let fields = identifiers::ascii_split_records(raw.take(end), 32, true);
        let path = raw.skip(end + 1);
        if fields.len() != 3 || fields[2].len() != 1 || fields[2][0] < 48 || fields[2][0] > 51 {
            Err(GitInventoryErrorView::Message("invalid Git index metadata"@))
        } else if !paths::relative_path(path) { Err(GitInventoryErrorView::InvalidPath(path)) }
        else if !identifiers::oid(fields[1]) {
            Err(GitInventoryErrorView::Message("expected full nonzero lowercase commit OID"@))
        } else { Ok((path, fields[0], fields[1], fields[2])) }
    }
}

pub fn parse_git_index_row(raw: &str) -> (out: Result<GitIndexRow<'_>, GitInventoryError<'_>>)
    ensures match out {
        Ok(row) => git_index_row_spec(raw.spec_bytes()) == Ok::<GitIndexRowBytes, GitInventoryErrorView>(row.bytes()),
        Err(error) => git_index_row_spec(raw.spec_bytes()) == Err::<GitIndexRowBytes, GitInventoryErrorView>(error.view()),
    },
{
    proof { reveal(git_index_row_spec); }
    let (meta, path) = match identifiers::git_inventory_row(raw, "invalid Git index row") {
        Ok(parts) => parts,
        Err(message) => return Err(GitInventoryError::Message(message)),
    };
    let fields = identifiers::git_inventory_fields(meta);
    match identifiers::git_index_metadata(&fields) {
        Ok(()) => {},
        Err(message) => return Err(GitInventoryError::Message(message)),
    }
    if !paths::valid_relative_path(path.as_bytes()) { return Err(GitInventoryError::InvalidPath(path)); }
    match identifiers::full_oid(fields[1]) {
        Ok(()) => {},
        Err(message) => return Err(GitInventoryError::Message(message)),
    }
    Ok(GitIndexRow { path, mode: fields[0], oid: fields[1], stage: fields[2] })
}

/// 与原 checkout 一致：只比较 stage、mode 和 OID，不增加 kind／mode 准入。
pub fn git_index_entry_changed(row: &GitIndexRow<'_>, entry: Option<&TreeEntry>) -> (changed: bool)
    ensures changed == !(row.stage@ == "0"@ && match entry {
        Some(value) => value.mode@ == row.mode@ && value.oid@ == row.oid@,
        None => false,
    }),
{
    !identifiers::equal_text(row.stage, "0") || match entry {
        Some(value) => !identifiers::equal_text(value.mode.as_str(), row.mode)
            || !identifiers::equal_text(value.oid.as_str(), row.oid),
        None => true,
    }
}

/// checkout 只支持原有三种 blob 模式，完整文本决定支持与否。
pub fn checkout_entry_supported(kind: &str, mode: &str) -> (supported: bool)
    ensures supported == (kind@ == "blob"@
        && (mode@ == "100644"@ || mode@ == "100755"@ || mode@ == "120000"@)),
{
    identifiers::equal_text(kind, "blob") && (identifiers::equal_text(mode, "100644")
        || identifiers::equal_text(mode, "100755") || identifiers::equal_text(mode, "120000"))
}

pub fn checkout_symbolic_link(mode: &str) -> (symbolic: bool)
    ensures symbolic == (mode@ == "120000"@),
{ identifiers::equal_text(mode, "120000") }

/// 与原消费一致：只核普通文件与 owner execute 位，不增加其他权限准入。
pub fn checkout_regular_file_matches(is_file: bool, permissions: u32, mode: &str) -> (matches: bool)
    ensures matches == (is_file && ((permissions & 0o100 != 0) == (mode@ == "100755"@))),
{
    is_file && ((permissions & 0o100 != 0) == identifiers::equal_text(mode, "100755"))
}

/// 打开后必须仍是普通文件，长度和完整权限字必须与打开前相同。
pub fn checkout_opened_file_matches(
    is_file: bool, opened_size: u64, opened_permissions: u32,
    observed_size: u64, observed_permissions: u32,
) -> (matches: bool)
    ensures matches == (is_file && opened_size == observed_size
        && opened_permissions == observed_permissions),
{
    is_file && opened_size == observed_size && opened_permissions == observed_permissions
}

pub open spec fn checkout_hash_advance_spec(total: u64, size: u64, chunk: u64)
    -> Result<u64, Seq<char>>
{
    let next = total as int + chunk as int;
    if next > u64::MAX { Err("checkout file size overflow"@) }
    else if next > size { Err("checkout file grew during observation"@) }
    else { Ok(next as u64) }
}

/// 先判计数溢出，再判相对本次原观察长度增长；保留原错误优先级。
pub fn checkout_hash_advance(total: u64, size: u64, chunk: u64) -> (out: Result<u64, String>)
    ensures match out {
        Ok(next) => checkout_hash_advance_spec(total, size, chunk) == Ok::<u64, Seq<char>>(next),
        Err(message) => checkout_hash_advance_spec(total, size, chunk) == Err::<u64, Seq<char>>(message@),
    },
{
    match total.checked_add(chunk) {
        None => Err("checkout file size overflow".to_owned()),
        Some(next) => {
            if next > size { Err("checkout file grew during observation".to_owned()) }
            else { Ok(next) }
        },
    }
}

pub fn checkout_hash_complete(total: u64, size: u64) -> (out: Result<(), String>)
    ensures
        out.is_ok() == (total == size),
        match out { Ok(()) => true, Err(message) => message@ == "checkout file shrank during observation"@ },
{
    if total == size { Ok(()) }
    else { Err("checkout file shrank during observation".to_owned()) }
}

pub open spec fn git_tree_row_at(rows: Seq<&str>, i: int) -> Result<GitTreeRowBytes, GitInventoryErrorView> {
    git_tree_row_spec(rows[i].spec_bytes())
}

pub open spec fn git_tree_prefix_valid(rows: Seq<&str>, count: int) -> bool {
    0 <= count <= rows.len()
        && (forall|i: int| 0 <= i < count ==> (#[trigger] git_tree_row_at(rows, i)).is_ok())
        && (forall|i: int, j: int| 0 <= i < j < count ==>
            ((#[trigger] git_tree_row_at(rows, i))->Ok_0).0
                != ((#[trigger] git_tree_row_at(rows, j))->Ok_0).0)
}

pub open spec fn git_tree_entry_matches(key: TreePath, entry: TreeEntry, row: GitTreeRowBytes) -> bool {
    key.bytes() == row.0 && vstd::utf8::encode_utf8(entry.mode@) == row.1
        && vstd::utf8::encode_utf8(entry.oid@) == row.2
        && vstd::utf8::encode_utf8(entry.kind@) == row.3
}

/// 双向完整对应排除遗漏与额外条目；数量是实际 map 的键数。
pub open spec fn git_tree_prefix_map(tree: Map<TreePath, TreeEntry>, rows: Seq<&str>, count: int) -> bool {
    tree.len() == count
        && (forall|key: TreePath| tree.contains_key(key) ==> exists|i: int| 0 <= i < count
            && git_tree_entry_matches(key, tree[key], (git_tree_row_at(rows, i)->Ok_0)))
        && (forall|i: int| #![trigger git_tree_row_at(rows, i)]
            0 <= i < count ==> exists|key: TreePath| tree.contains_key(key)
            && git_tree_entry_matches(key, tree[key], (git_tree_row_at(rows, i)->Ok_0)))
}

pub open spec fn git_tree_error_at(rows: Seq<&str>, i: int, error: GitInventoryErrorView) -> bool {
    match git_tree_row_at(rows, i) {
        Err(expected) => error == expected,
        Ok(row) => error == GitInventoryErrorView::Message("duplicate Git tree path"@)
            && exists|j: int| 0 <= j < i && (git_tree_row_at(rows, j)->Ok_0).0 == row.0,
    }
}

pub proof fn git_tree_error_blocks_prefix(rows: Seq<&str>, i: int,
    error: GitInventoryErrorView, count: int)
    requires 0 <= i < count <= rows.len(), git_tree_error_at(rows, i, error),
    ensures !git_tree_prefix_valid(rows, count),
{
    if git_tree_prefix_valid(rows, count) {
        match git_tree_row_at(rows, i) {
            Err(_) => {},
            Ok(row) => {
                let j = choose|j: int| 0 <= j < i && (git_tree_row_at(rows, j)->Ok_0).0 == row.0;
                assert((git_tree_row_at(rows, j)->Ok_0).0 != (git_tree_row_at(rows, i)->Ok_0).0);
            },
        }
    }
}

pub proof fn git_tree_first_error_unique(rows: Seq<&str>, i: int, error: GitInventoryErrorView,
    j: int, other: GitInventoryErrorView)
    requires 0 <= i < rows.len(), git_tree_prefix_valid(rows, i), git_tree_error_at(rows, i, error),
        0 <= j < rows.len(), git_tree_prefix_valid(rows, j), git_tree_error_at(rows, j, other),
    ensures i == j, error == other,
{
    if i < j { git_tree_error_blocks_prefix(rows, i, error, j); }
    if j < i { git_tree_error_blocks_prefix(rows, j, other, i); }
}

/// 已发布前缀的路径见证与下一次插入的全文判重使用同一身份。
proof fn git_tree_next_path_is_new(tree: Map<TreePath, TreeEntry>, rows: Seq<&str>,
    count: int, path: TreePath)
    requires
        git_tree_prefix_map(tree, rows, count),
        !(exists|key: TreePath| (#[trigger] tree.contains_key(key)) && key@ == path@),
    ensures forall|j: int| 0 <= j < count ==>
        (#[trigger] git_tree_row_at(rows, j)->Ok_0).0 != path.bytes(),
{
    assert forall|j: int| 0 <= j < count implies
        (#[trigger] git_tree_row_at(rows, j)->Ok_0).0 != path.bytes() by {
        let key = choose|key: TreePath| tree.contains_key(key) && git_tree_entry_matches(
            key, tree[key], (git_tree_row_at(rows, j)->Ok_0));
        if key.bytes() == path.bytes() {
            tree_path_same_text(&key, &path);
            assert(exists|key: TreePath| (#[trigger] tree.contains_key(key)) && key@ == path@);
        }
    }
}

proof fn git_tree_extend_valid_prefix(rows: Seq<&str>, count: int, row: GitTreeRowBytes)
    requires
        0 <= count < rows.len(), git_tree_prefix_valid(rows, count),
        git_tree_row_at(rows, count) == Ok::<GitTreeRowBytes, GitInventoryErrorView>(row),
        forall|j: int| 0 <= j < count ==> (#[trigger] git_tree_row_at(rows, j)->Ok_0).0 != row.0,
    ensures git_tree_prefix_valid(rows, count + 1),
{
    assert forall|j: int| 0 <= j < count + 1 implies (#[trigger] git_tree_row_at(rows, j)).is_ok() by {
        if j < count {} else { assert(j == count); }
    }
    assert forall|i: int, j: int| 0 <= i < j < count + 1 implies
        ((#[trigger] git_tree_row_at(rows, i))->Ok_0).0
            != ((#[trigger] git_tree_row_at(rows, j))->Ok_0).0 by {
        if j < count {} else { assert(j == count); }
    }
}

/// 精确 map 更新的双向完整性单独证明，不展开行字节解析。
proof fn git_tree_extend_prefix_map(tree: Map<TreePath, TreeEntry>, rows: Seq<&str>,
    count: int, inserted: TreePath, entry: TreeEntry, row: GitTreeRowBytes)
    requires
        0 <= count < rows.len(), git_tree_prefix_map(tree, rows, count), tree.dom().finite(),
        git_tree_row_at(rows, count) == Ok::<GitTreeRowBytes, GitInventoryErrorView>(row),
        git_tree_entry_matches(inserted, entry, row), !tree.contains_key(inserted),
    ensures git_tree_prefix_map(tree.insert(inserted, entry), rows, count + 1),
{
    let next = tree.insert(inserted, entry);
    assert(next.len() == count + 1);
    assert forall|key: TreePath| (#[trigger] next.contains_key(key)) implies exists|j: int|
        0 <= j < count + 1 && git_tree_entry_matches(
            key, next[key], (git_tree_row_at(rows, j)->Ok_0)) by {
        if key == inserted {
            assert(git_tree_entry_matches(key, next[key], (git_tree_row_at(rows, count)->Ok_0)));
        } else {
            assert(tree.contains_key(key));
            let j = choose|j: int| 0 <= j < count && git_tree_entry_matches(
                key, tree[key], (git_tree_row_at(rows, j)->Ok_0));
            assert(git_tree_entry_matches(key, next[key], (git_tree_row_at(rows, j)->Ok_0)));
        }
    }
    assert forall|j: int| #![trigger git_tree_row_at(rows, j)]
        0 <= j < count + 1 implies exists|key: TreePath|
        next.contains_key(key) && git_tree_entry_matches(
            key, next[key], (git_tree_row_at(rows, j)->Ok_0)) by {
        if j == count {
            assert(next.contains_key(inserted));
            assert(git_tree_entry_matches(inserted, next[inserted], (git_tree_row_at(rows, j)->Ok_0)));
        } else {
            let key = choose|key: TreePath| tree.contains_key(key) && git_tree_entry_matches(
                key, tree[key], (git_tree_row_at(rows, j)->Ok_0));
            assert(key != inserted);
            assert(next.contains_key(key));
            assert(git_tree_entry_matches(key, next[key], (git_tree_row_at(rows, j)->Ok_0)));
        }
    }
}

proof fn git_tree_duplicate_error(tree: Map<TreePath, TreeEntry>, rows: Seq<&str>,
    count: int, path: TreePath, row: GitTreeRowBytes)
    requires
        0 <= count < rows.len(), git_tree_prefix_map(tree, rows, count),
        git_tree_row_at(rows, count) == Ok::<GitTreeRowBytes, GitInventoryErrorView>(row),
        path.bytes() == row.0,
        exists|key: TreePath| (#[trigger] tree.contains_key(key)) && key@ == path@,
    ensures git_tree_error_at(rows, count, GitInventoryErrorView::Message("duplicate Git tree path"@)),
{
    let key = choose|key: TreePath| (#[trigger] tree.contains_key(key)) && key@ == path@;
    let j = choose|j: int| 0 <= j < count && git_tree_entry_matches(
        key, tree[key], (git_tree_row_at(rows, j)->Ok_0));
    assert((git_tree_row_at(rows, j)->Ok_0).0 == row.0);
    assert(exists|j: int| 0 <= j < count && (git_tree_row_at(rows, j)->Ok_0).0 == row.0);
}

/// 完整逐行解析：返回全部原字段，或首个失败；失败后的局部树不发布。
pub fn parse_git_tree_rows<'a>(rows: &[&'a str]) -> (out: Result<Tree, GitInventoryError<'a>>)
    ensures match out {
        Ok(tree) => git_tree_prefix_valid(rows@, rows.len() as int)
            && git_tree_prefix_map(tree@, rows@, rows.len() as int),
        Err(error) => exists|i: int| 0 <= i < rows.len() && git_tree_prefix_valid(rows@, i)
            && git_tree_error_at(rows@, i, error.view()),
    },
{
    let mut tree = Tree::new();
    let mut i = 0;
    while i < rows.len()
        invariant
            0 <= i <= rows.len(),
            git_tree_prefix_valid(rows@, i as int),
            git_tree_prefix_map(tree@, rows@, i as int),
        decreases rows.len() - i,
    {
        let row = match parse_git_tree_row(rows[i]) {
            Ok(row) => row,
            Err(error) => {
                proof { assert(git_tree_error_at(rows@, i as int, error.view())); }
                return Err(error);
            },
        };
        let key = TreePath::from_text(row.path);
        let ghost inserted = key;
        let ghost before = tree@;
        proof { vstd::std_specs::btree::axiom_btree_map_view_finite_dom(tree); }
        let result = insert_git_tree_entry(&mut tree, key, row.mode, row.oid, row.kind);
        match result {
            Err(message) => {
                proof {
                    git_tree_duplicate_error(before, rows@, i as int, inserted, row.bytes());
                }
                return Err(GitInventoryError::Message(message));
            },
            Ok(()) => {},
        }
        proof {
            git_tree_next_path_is_new(before, rows@, i as int, inserted);
            git_tree_extend_valid_prefix(rows@, i as int, row.bytes());
            assert(git_tree_entry_matches(inserted, tree@[inserted], row.bytes()));
            git_tree_extend_prefix_map(before, rows@, i as int, inserted, tree@[inserted], row.bytes());
        }
        i += 1;
    }
    Ok(tree)
}

pub open spec fn optional_entry_fields(entry: Option<&TreeEntry>) -> Option<TreeEntryFields> {
    match entry { None => None, Some(entry) => Some(tree_entry_fields(entry)) }
}

pub open spec fn fields_blob(entry: Option<TreeEntryFields>) -> Option<Seq<char>> {
    match entry { None => None, Some(entry) => Some(entry.1) }
}

pub open spec fn fields_mode(entry: Option<TreeEntryFields>) -> Option<Seq<char>> {
    match entry { None => None, Some(entry) => Some(entry.0) }
}

pub open spec fn entry_delta_values(path: Seq<char>, old_entry: Option<TreeEntryFields>,
    new_entry: Option<TreeEntryFields>) -> Seq<DeltaFields>
{
    if old_entry == new_entry { seq![] }
    else { seq![(path,
        if old_entry.is_none() { "A"@ } else if new_entry.is_none() { "D"@ } else { "M"@ },
        fields_blob(old_entry), fields_blob(new_entry), fields_mode(old_entry), fields_mode(new_entry))] }
}

pub open spec fn rows_entry(rows: Seq<(&TreePath, &TreeEntry)>, start: int,
    path: Seq<char>) -> Option<TreeEntryFields>
    decreases rows.len() - start,
{
    if !(0 <= start < rows.len()) { None }
    else if rows[start].0@ == path { Some(tree_entry_fields(rows[start].1)) }
    else { rows_entry(rows, start + 1, path) }
}

pub open spec fn rows_has_path(rows: Seq<(&TreePath, &TreeEntry)>, start: int,
    path: Seq<char>) -> bool
{
    exists|k: int| start <= k < rows.len() && (#[trigger] rows[k]).0@ == path
}

proof fn rows_entry_witness(rows: Seq<(&TreePath, &TreeEntry)>, start: int, path: Seq<char>)
    requires 0 <= start <= rows.len(),
    ensures rows_entry(rows, start, path).is_some() == rows_has_path(rows, start, path),
        match rows_entry(rows, start, path) {
            None => true,
            Some(fields) => exists|k: int| start <= k < rows.len()
                && (#[trigger] rows[k]).0@ == path && tree_entry_fields(rows[k].1) == fields,
        },
    decreases rows.len() - start,
{
    if start < rows.len() {
        if rows[start].0@ == path {
            assert(rows_has_path(rows, start, path));
        } else {
            rows_entry_witness(rows, start + 1, path);
            assert(rows_has_path(rows, start, path) == rows_has_path(rows, start + 1, path));
        }
    }
}

proof fn tree_path_same_text(a: &TreePath, b: &TreePath)
    requires a.same(b),
    ensures a@ == b@,
{
    vstd::utf8::encode_utf8_decode_utf8(a@);
    vstd::utf8::encode_utf8_decode_utf8(b@);
}

proof fn rows_entry_absent(rows: Seq<(&TreePath, &TreeEntry)>, start: int, path: Seq<char>)
    requires 0 <= start <= rows.len(),
        forall|k: int| start <= k < rows.len() ==> (#[trigger] rows[k]).0@ != path,
    ensures rows_entry(rows, start, path).is_none(),
    decreases rows.len() - start,
{
    if start < rows.len() { rows_entry_absent(rows, start + 1, path); }
}

proof fn rows_entry_before_head(rows: Seq<(&TreePath, &TreeEntry)>, start: int, key: &TreePath)
    requires tree_rows_ordered(rows), 0 <= start <= rows.len(),
        start == rows.len() || identifiers::byte_lt(key.bytes(), rows[start].0.bytes()),
    ensures rows_entry(rows, start, key@).is_none(),
        forall|k: int| start <= k < rows.len()
            ==> identifiers::byte_lt(key.bytes(), (#[trigger] rows[k]).0.bytes()),
{
    assert forall|k: int| start <= k < rows.len()
        implies identifiers::byte_lt(key.bytes(), (#[trigger] rows[k]).0.bytes())
            && rows[k].0@ != key@ by {
        if start < k {
            identifiers::byte_lt_transitive(key.bytes(), rows[start].0.bytes(), rows[k].0.bytes());
        }
        identifiers::byte_lt_strict(key.bytes(), rows[k].0.bytes());
    }
    rows_entry_absent(rows, start, key@);
}

proof fn entry_delta_projection(selected_path: Seq<char>, old_entry: Option<&TreeEntry>,
    new_entry: Option<&TreeEntry>, path: Seq<char>)
    ensures delta_at_path(entry_delta_fields(selected_path, old_entry, new_entry), path)
        == if selected_path == path {
            entry_delta_values(path, optional_entry_fields(old_entry), optional_entry_fields(new_entry))
        } else { seq![] },
{
    reveal_with_fuel(Seq::filter, 2);
}

pub open spec fn tree_merge_fields(a: Seq<(&TreePath, &TreeEntry)>,
    b: Seq<(&TreePath, &TreeEntry)>, i: int, j: int) -> Seq<DeltaFields>
    decreases a.len() - i + b.len() - j,
{
    if !(0 <= i <= a.len() && 0 <= j <= b.len()) { seq![] }
    else if i == a.len() && j == b.len() { seq![] }
    else if i == a.len() {
        entry_delta_fields(b[j].0@, None, Some(b[j].1)) + tree_merge_fields(a, b, i, j + 1)
    } else if j == b.len() || a[i].0.order_spec(b[j].0) is Less {
        entry_delta_fields(a[i].0@, Some(a[i].1), None) + tree_merge_fields(a, b, i + 1, j)
    } else if a[i].0.order_spec(b[j].0) is Greater {
        entry_delta_fields(b[j].0@, None, Some(b[j].1)) + tree_merge_fields(a, b, i, j + 1)
    } else {
        entry_delta_fields(a[i].0@, Some(a[i].1), Some(b[j].1))
            + tree_merge_fields(a, b, i + 1, j + 1)
    }
}

/// 对任意完整路径，输出恰为该路径实际两端字段的零或一条变化。
proof fn tree_merge_path_projection(a: Seq<(&TreePath, &TreeEntry)>,
    b: Seq<(&TreePath, &TreeEntry)>, i: int, j: int, path: Seq<char>)
    requires tree_rows_ordered(a), tree_rows_ordered(b),
        0 <= i <= a.len(), 0 <= j <= b.len(),
    ensures delta_at_path(tree_merge_fields(a, b, i, j), path)
        == entry_delta_values(path, rows_entry(a, i, path), rows_entry(b, j, path)),
    decreases a.len() - i + b.len() - j,
{
    reveal_with_fuel(tree_merge_fields, 1);
    if i == a.len() && j == b.len() {
        reveal_with_fuel(Seq::filter, 1);
        return;
    }
    if i < a.len() && j < b.len() {
        identifiers::byte_trichotomy(a[i].0.bytes(), b[j].0.bytes());
        if a[i].0.order_spec(b[j].0) is Equal {
            tree_path_same_text(a[i].0, b[j].0);
        }
    }
    let (key, old_entry, new_entry, ni, nj):
        (&TreePath, Option<&TreeEntry>, Option<&TreeEntry>, int, int) =
        if i == a.len() { (b[j].0, None, Some(b[j].1), i, j + 1) }
        else if j == b.len() || a[i].0.order_spec(b[j].0) is Less {
            (a[i].0, Some(a[i].1), None, i + 1, j)
        } else if a[i].0.order_spec(b[j].0) is Greater {
            (b[j].0, None, Some(b[j].1), i, j + 1)
        } else { (a[i].0, Some(a[i].1), Some(b[j].1), i + 1, j + 1) };
    assert(ni == a.len() || identifiers::byte_lt(key.bytes(), a[ni].0.bytes()));
    assert(nj == b.len() || identifiers::byte_lt(key.bytes(), b[nj].0.bytes()));
    tree_merge_path_projection(a, b, ni, nj, path);
    let head = entry_delta_fields(key@, old_entry, new_entry);
    let rest = tree_merge_fields(a, b, ni, nj);
    assert(tree_merge_fields(a, b, i, j) == head + rest);
    entry_delta_projection(key@, old_entry, new_entry, path);
    Seq::<DeltaFields>::filter_distributes_over_add(head, rest,
        delta_path_predicate(path));
    if path == key@ {
        rows_entry_before_head(a, ni, key);
        rows_entry_before_head(b, nj, key);
        assert(rows_entry(a, i, path) == optional_entry_fields(old_entry));
        assert(rows_entry(b, j, path) == optional_entry_fields(new_entry));
    } else {
        assert(rows_entry(a, i, path) == rows_entry(a, ni, path));
        assert(rows_entry(b, j, path) == rows_entry(b, nj, path));
    }
}

proof fn tree_merge_sources(a: Seq<(&TreePath, &TreeEntry)>,
    b: Seq<(&TreePath, &TreeEntry)>, i: int, j: int)
    requires tree_rows_ordered(a), tree_rows_ordered(b),
        0 <= i <= a.len(), 0 <= j <= b.len(),
    ensures forall|k: int| 0 <= k < tree_merge_fields(a, b, i, j).len()
        ==> rows_has_path(a, i, (#[trigger] tree_merge_fields(a, b, i, j)[k]).0)
            || rows_has_path(b, j, tree_merge_fields(a, b, i, j)[k].0),
{
    let changes = tree_merge_fields(a, b, i, j);
    assert forall|k: int| 0 <= k < changes.len()
        implies rows_has_path(a, i, (#[trigger] changes[k]).0)
            || rows_has_path(b, j, changes[k].0) by {
        let path = changes[k].0;
        changes.lemma_filter_contains(delta_path_predicate(path), k);
        tree_merge_path_projection(a, b, i, j, path);
        rows_entry_witness(a, i, path);
        rows_entry_witness(b, j, path);
        assert(entry_delta_values(path, rows_entry(a, i, path), rows_entry(b, j, path)).len() > 0);
    }
}

pub open spec fn delta_fields_ordered(changes: Seq<DeltaFields>) -> bool {
    forall|i: int, j: int| 0 <= i < j < changes.len()
        ==> identifiers::byte_lt(vstd::utf8::encode_utf8((#[trigger] changes[i]).0),
            vstd::utf8::encode_utf8((#[trigger] changes[j]).0))
}

proof fn tree_merge_ordered(a: Seq<(&TreePath, &TreeEntry)>,
    b: Seq<(&TreePath, &TreeEntry)>, i: int, j: int)
    requires tree_rows_ordered(a), tree_rows_ordered(b),
        0 <= i <= a.len(), 0 <= j <= b.len(),
    ensures delta_fields_ordered(tree_merge_fields(a, b, i, j)),
    decreases a.len() - i + b.len() - j,
{
    reveal_with_fuel(tree_merge_fields, 1);
    if i == a.len() && j == b.len() { return; }
    if i < a.len() && j < b.len() {
        identifiers::byte_trichotomy(a[i].0.bytes(), b[j].0.bytes());
    }
    let (key, old_entry, new_entry, ni, nj):
        (&TreePath, Option<&TreeEntry>, Option<&TreeEntry>, int, int) =
        if i == a.len() { (b[j].0, None, Some(b[j].1), i, j + 1) }
        else if j == b.len() || a[i].0.order_spec(b[j].0) is Less {
            (a[i].0, Some(a[i].1), None, i + 1, j)
        } else if a[i].0.order_spec(b[j].0) is Greater {
            (b[j].0, None, Some(b[j].1), i, j + 1)
        } else { (a[i].0, Some(a[i].1), Some(b[j].1), i + 1, j + 1) };
    assert(ni == a.len() || identifiers::byte_lt(key.bytes(), a[ni].0.bytes()));
    assert(nj == b.len() || identifiers::byte_lt(key.bytes(), b[nj].0.bytes()));
    tree_merge_ordered(a, b, ni, nj);
    tree_merge_sources(a, b, ni, nj);
    rows_entry_before_head(a, ni, key);
    rows_entry_before_head(b, nj, key);
    let head = entry_delta_fields(key@, old_entry, new_entry);
    let rest = tree_merge_fields(a, b, ni, nj);
    let changes = tree_merge_fields(a, b, i, j);
    assert(changes == head + rest);
    assert(head.len() <= 1);
    assert(head.len() > 0 ==> head[0].0 == key@);
    assert forall|k: int| 0 <= k < rest.len()
        implies identifiers::byte_lt(key.bytes(), vstd::utf8::encode_utf8((#[trigger] rest[k]).0)) by {
        if rows_has_path(a, ni, rest[k].0) {
            let index = choose|index: int| ni <= index < a.len() && a[index].0@ == rest[k].0;
            assert(a[index].0.bytes() == vstd::utf8::encode_utf8(rest[k].0));
        } else {
            let index = choose|index: int| nj <= index < b.len() && b[index].0@ == rest[k].0;
            assert(b[index].0.bytes() == vstd::utf8::encode_utf8(rest[k].0));
        }
    }
    assert forall|x: int, y: int| 0 <= x < y < changes.len()
        implies identifiers::byte_lt(vstd::utf8::encode_utf8((#[trigger] changes[x]).0),
            vstd::utf8::encode_utf8((#[trigger] changes[y]).0)) by {
        if x < head.len() {
            assert(x == 0 && head.len() == 1);
            assert(changes[y] == rest[y - head.len()]);
        } else {
            assert(changes[x] == rest[x - head.len()]);
            assert(changes[y] == rest[y - head.len()]);
        }
    }
}

fn push_tree_change(out: &mut Vec<Delta>, path: &str,
    old_entry: Option<&TreeEntry>, new_entry: Option<&TreeEntry>)
    ensures delta_records_view(out@) == delta_records_view(old(out)@)
        + entry_delta_fields(path@, old_entry, new_entry),
{
    match delta_for_entry(path, old_entry, new_entry) {
        None => {},
        Some(change) => { out.push(change); },
    }
    proof { assert(delta_records_view(out@) =~= delta_records_view(old(out)@)
        + entry_delta_fields(path@, old_entry, new_entry)); }
}

fn delta_for_rows(a: &Vec<(&TreePath, &TreeEntry)>, b: &Vec<(&TreePath, &TreeEntry)>)
    -> (out: Vec<Delta>)
    ensures delta_records_view(out@) == tree_merge_fields(a@, b@, 0, 0),
{
    let mut out: Vec<Delta> = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;
    while i < a.len() || j < b.len()
        invariant
            i <= a.len(), j <= b.len(),
            delta_records_view(out@) + tree_merge_fields(a@, b@, i as int, j as int)
                == tree_merge_fields(a@, b@, 0, 0),
        decreases a.len() - i + b.len() - j,
    {
        proof { reveal_with_fuel(tree_merge_fields, 1); }
        if i == a.len() {
            push_tree_change(&mut out, b[j].0.as_str(), None, Some(b[j].1));
            j += 1;
        } else if j == b.len() {
            push_tree_change(&mut out, a[i].0.as_str(), Some(a[i].1), None);
            i += 1;
        } else {
            let order = a[i].0.cmp(b[j].0);
            match order {
                Ordering::Less => {
                    push_tree_change(&mut out, a[i].0.as_str(), Some(a[i].1), None);
                    i += 1;
                },
                Ordering::Greater => {
                    push_tree_change(&mut out, b[j].0.as_str(), None, Some(b[j].1));
                    j += 1;
                },
                Ordering::Equal => {
                    push_tree_change(&mut out, a[i].0.as_str(), Some(a[i].1), Some(b[j].1));
                    i += 1;
                    j += 1;
                },
            }
        }
    }
    out
}

pub open spec fn map_has_path(tree: Map<TreePath, TreeEntry>, path: Seq<char>) -> bool {
    exists|key: TreePath| tree.contains_key(key) && key@ == path
}

pub open spec fn map_path_key(tree: Map<TreePath, TreeEntry>, path: Seq<char>) -> TreePath {
    choose|key: TreePath| tree.contains_key(key) && key@ == path
}

pub open spec fn map_entry_fields(tree: Map<TreePath, TreeEntry>, path: Seq<char>)
    -> Option<TreeEntryFields>
{
    if map_has_path(tree, path) {
        let key = map_path_key(tree, path);
        Some(tree_entry_fields(&tree[key]))
    } else { None }
}

pub open spec fn delta_path_predicate(path: Seq<char>) -> spec_fn(DeltaFields) -> bool {
    |change: DeltaFields| change.0 == path
}

pub open spec fn delta_at_path(changes: Seq<DeltaFields>, path: Seq<char>) -> Seq<DeltaFields> {
    changes.filter(delta_path_predicate(path))
}

proof fn map_rows_entry(tree: Map<TreePath, TreeEntry>, rows: Seq<(&TreePath, &TreeEntry)>,
    path: Seq<char>)
    requires tree_rows_view(rows).to_set() == tree.kv_pairs(), tree_rows_ordered(rows),
    ensures map_entry_fields(tree, path) == rows_entry(rows, 0, path),
{
    rows_entry_witness(rows, 0, path);
    if map_has_path(tree, path) {
        let key = map_path_key(tree, path);
        assert(tree.contains_key(key) && key@ == path);
        assert(tree.kv_pairs().contains((key, tree[key])));
        assert(tree_rows_view(rows).to_set().contains((key, tree[key])));
        assert(tree_rows_view(rows).contains((key, tree[key])));
        let index = tree_rows_view(rows).lemma_contains_to_index((key, tree[key]));
        assert(rows[index].0@ == path);
        assert(rows_has_path(rows, 0, path));
        match rows_entry(rows, 0, path) {
            None => { assert(false); },
            Some(fields) => {
                let found = choose|found: int| 0 <= found < rows.len()
                    && rows[found].0@ == path && tree_entry_fields(rows[found].1) == fields;
                if found < index {
                    identifiers::byte_lt_strict(rows[found].0.bytes(), rows[index].0.bytes());
                    assert(false);
                } else if index < found {
                    identifiers::byte_lt_strict(rows[index].0.bytes(), rows[found].0.bytes());
                    assert(false);
                }
                assert(found == index);
                assert(fields == tree_entry_fields(&tree[key]));
            },
        }
    } else {
        assert forall|k: int| 0 <= k < rows.len()
            implies (#[trigger] rows[k]).0@ != path by {
            assert(tree_rows_view(rows).contains(tree_rows_view(rows)[k]));
            assert(tree.kv_pairs().contains(tree_rows_view(rows)[k]));
            assert(tree.contains_key(*rows[k].0));
            if rows[k].0@ == path { assert(map_has_path(tree, path)); }
        }
        rows_entry_absent(rows, 0, path);
    }
}

pub fn delta_for_trees(a: &Tree, b: &Tree) -> (out: Vec<Delta>)
    ensures
        delta_fields_ordered(delta_records_view(out@)),
        forall|path: Seq<char>| #[trigger] delta_at_path(delta_records_view(out@), path)
            == entry_delta_values(path, map_entry_fields(a@, path), map_entry_fields(b@, path)),
{
    let left = tree_rows(a);
    let right = tree_rows(b);
    let out = delta_for_rows(&left, &right);
    proof {
        assert(delta_records_view(out@) == tree_merge_fields(left@, right@, 0, 0));
        tree_merge_ordered(left@, right@, 0, 0);
        assert forall|path: Seq<char>|
            #[trigger] delta_at_path(delta_records_view(out@), path)
                == entry_delta_values(path, map_entry_fields(a@, path), map_entry_fields(b@, path)) by {
            tree_merge_path_projection(left@, right@, 0, 0, path);
            map_rows_entry(a@, left@, path);
            map_rows_entry(b@, right@, path);
            assert(rows_entry(left@, 0, path) == map_entry_fields(a@, path));
            assert(rows_entry(right@, 0, path) == map_entry_fields(b@, path));
            assert(delta_at_path(tree_merge_fields(left@, right@, 0, 0), path)
                == entry_delta_values(path, rows_entry(left@, 0, path), rows_entry(right@, 0, path)));
            assert(delta_at_path(delta_records_view(out@), path)
                == entry_delta_values(path, map_entry_fields(a@, path), map_entry_fields(b@, path)));
        }
    }
    out
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
