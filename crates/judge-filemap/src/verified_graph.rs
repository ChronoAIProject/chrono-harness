//! Typed edge ordering, exact name lookup and indexed FIFO graph traversal.
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
use core::cmp::Ordering;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

#[derive(Clone, Copy)]
pub enum EdgeKind {
    Compile,
    BuildInput,
    RuntimeInput,
    TestExecution,
    JudgeTrigger,
}

impl EdgeKind {
    pub open spec fn rank_spec(&self) -> u8 {
        match self {
            Self::Compile => 0,
            Self::BuildInput => 1,
            Self::RuntimeInput => 2,
            Self::TestExecution => 3,
            Self::JudgeTrigger => 4,
        }
    }

    pub fn rank(&self) -> (rank: u8)
        ensures rank == self.rank_spec(),
    {
        match self {
            Self::Compile => 0,
            Self::BuildInput => 1,
            Self::RuntimeInput => 2,
            Self::TestExecution => 3,
            Self::JudgeTrigger => 4,
        }
    }

    pub open spec fn order_spec(&self, other: &Self) -> Ordering {
        if self.rank_spec() < other.rank_spec() { Ordering::Less }
        else if self.rank_spec() > other.rank_spec() { Ordering::Greater }
        else { Ordering::Equal }
    }

    pub fn compare(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        let rank = self.rank();
        let other_rank = other.rank();
        if rank < other_rank { Ordering::Less }
        else if rank > other_rank { Ordering::Greater }
        else { Ordering::Equal }
    }

    pub open spec fn wire_spec(&self) -> Seq<u8> {
        match self {
            Self::Compile => "compile".spec_bytes(),
            Self::BuildInput => "build-input".spec_bytes(),
            Self::RuntimeInput => "runtime-input".spec_bytes(),
            Self::TestExecution => "test-execution".spec_bytes(),
            Self::JudgeTrigger => "judge-trigger".spec_bytes(),
        }
    }

    pub fn wire_name(&self) -> (name: &'static str)
        ensures name.spec_bytes() == self.wire_spec(),
    {
        match self {
            Self::Compile => "compile",
            Self::BuildInput => "build-input",
            Self::RuntimeInput => "runtime-input",
            Self::TestExecution => "test-execution",
            Self::JudgeTrigger => "judge-trigger",
        }
    }

    pub fn from_wire(name: &str) -> (kind: Option<Self>)
        ensures
            kind.is_some() <==> exists|k: Self| k.wire_spec() == name.spec_bytes(),
            kind matches Some(k) ==> k.wire_spec() == name.spec_bytes(),
    {
        let kind = if compare_bytes(name.as_bytes(), "compile".as_bytes()) == 0 {
            Some(Self::Compile)
        } else if compare_bytes(name.as_bytes(), "build-input".as_bytes()) == 0 {
            Some(Self::BuildInput)
        } else if compare_bytes(name.as_bytes(), "runtime-input".as_bytes()) == 0 {
            Some(Self::RuntimeInput)
        } else if compare_bytes(name.as_bytes(), "test-execution".as_bytes()) == 0 {
            Some(Self::TestExecution)
        } else if compare_bytes(name.as_bytes(), "judge-trigger".as_bytes()) == 0 {
            Some(Self::JudgeTrigger)
        } else {
            proof {
                assert forall|k: Self| k.wire_spec() != name.spec_bytes() by {
                    match k {
                        Self::Compile => {},
                        Self::BuildInput => {},
                        Self::RuntimeInput => {},
                        Self::TestExecution => {},
                        Self::JudgeTrigger => {},
                    }
                }
            }
            None
        };
        proof {
            if let Some(k) = kind {
                assert(k.wire_spec() == name.spec_bytes());
                assert(exists|accepted: Self| accepted.wire_spec() == name.spec_bytes());
            }
        }
        kind
    }
}

pub proof fn edge_kind_order_laws(a: EdgeKind, b: EdgeKind, c: EdgeKind)
    ensures
        a.rank_spec() <= 4,
        (a.rank_spec() == b.rank_spec()) == (a == b),
        (a.order_spec(&b) == Ordering::Equal) == (a == b),
        (a.order_spec(&b) == Ordering::Less) == (b.order_spec(&a) == Ordering::Greater),
        a.order_spec(&b) == Ordering::Less && b.order_spec(&c) == Ordering::Less
            ==> a.order_spec(&c) == Ordering::Less,
{}

pub proof fn edge_kind_wire_unique(a: EdgeKind, b: EdgeKind)
    ensures (a.wire_spec() == b.wire_spec()) == (a == b),
{
    reveal_strlit("compile");
    reveal_strlit("build-input");
    reveal_strlit("runtime-input");
    reveal_strlit("test-execution");
    reveal_strlit("judge-trigger");
    vstd::string::is_ascii_spec_bytes("compile");
    vstd::string::is_ascii_spec_bytes("build-input");
    vstd::string::is_ascii_spec_bytes("runtime-input");
    vstd::string::is_ascii_spec_bytes("test-execution");
    vstd::string::is_ascii_spec_bytes("judge-trigger");
    assert("compile".spec_bytes().len() == 7);
    assert("build-input".spec_bytes().len() == 11);
    assert("runtime-input".spec_bytes().len() == 13);
    assert("test-execution".spec_bytes().len() == 14);
    assert("judge-trigger".spec_bytes().len() == 13);
    assert("runtime-input".spec_bytes()[0] == 114);
    assert("judge-trigger".spec_bytes()[0] == 106);
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for EdgeKind {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { *self == *other }
}

impl PartialEq for EdgeKind {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (*self == *other),
    {
        self.rank() == other.rank()
    }
}

impl Eq for EdgeKind {}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::OrdSpecImpl for EdgeKind {
    open spec fn obeys_cmp_spec() -> bool { true }
    open spec fn cmp_spec(&self, other: &Self) -> Ordering {
        self.order_spec(other)
    }
}

impl Ord for EdgeKind {
    fn cmp(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        self.compare(other)
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialOrdSpecImpl for EdgeKind {
    open spec fn obeys_partial_cmp_spec() -> bool { true }
    open spec fn partial_cmp_spec(&self, other: &Self) -> Option<Ordering> {
        Some(self.order_spec(other))
    }
}

impl PartialOrd for EdgeKind {
    fn partial_cmp(&self, other: &Self) -> (order: Option<Ordering>)
        ensures order == Some(self.order_spec(other)),
    {
        Some(self.compare(other))
    }
}

pub open spec fn byte_prefix(a: Seq<u8>, b: Seq<u8>, n: int) -> bool {
    0 <= n <= a.len() && n <= b.len()
        && forall|i: int| 0 <= i < n ==> a[i] == b[i]
}

/// Rust string identity and order use the complete UTF-8 byte sequence.
pub open spec fn byte_lt(a: Seq<u8>, b: Seq<u8>) -> bool {
    exists|i: int| 0 <= i <= a.len() && i < b.len() && byte_prefix(a, b, i)
        && (i == a.len() || a[i] < b[i])
}

proof fn byte_lt_strict(a: Seq<u8>, b: Seq<u8>)
    requires byte_lt(a, b),
    ensures a != b,
{
    let i = choose|i: int| 0 <= i <= a.len() && i < b.len() && byte_prefix(a, b, i)
        && (i == a.len() || a[i] < b[i]);
}

proof fn byte_lt_transitive(a: Seq<u8>, b: Seq<u8>, c: Seq<u8>)
    requires byte_lt(a, b), byte_lt(b, c),
    ensures byte_lt(a, c),
{
    let i = choose|i: int| 0 <= i <= a.len() && i < b.len() && byte_prefix(a, b, i)
        && (i == a.len() || a[i] < b[i]);
    let j = choose|j: int| 0 <= j <= b.len() && j < c.len() && byte_prefix(b, c, j)
        && (j == b.len() || b[j] < c[j]);
    let k = if i < j { i } else { j };
    assert(byte_prefix(a, c, k)) by {
        assert forall|t: int| 0 <= t < k implies a[t] == c[t] by {}
    }
    assert(k == a.len() || a[k] < c[k]);
    assert(0 <= k <= a.len() && k < c.len());
}

/// -1, 0 and 1 denote less, byte-identical and greater respectively.
pub fn compare_bytes(a: &[u8], b: &[u8]) -> (order: i8)
    ensures
        -1 <= order <= 1,
        order == -1 ==> byte_lt(a@, b@),
        order == 0 <==> a@ == b@,
        order == 1 ==> byte_lt(b@, a@),
{
    let mut i: usize = 0;
    while i < a.len() && i < b.len()
        invariant i <= a.len(), i <= b.len(), byte_prefix(a@, b@, i as int),
        decreases a.len() - i,
    {
        if a[i] < b[i] {
            assert(byte_lt(a@, b@));
            proof { byte_lt_strict(a@, b@); }
            return -1;
        }
        if a[i] > b[i] {
            assert(byte_prefix(b@, a@, i as int));
            assert(byte_lt(b@, a@));
            proof { byte_lt_strict(b@, a@); }
            return 1;
        }
        i += 1;
    }
    if a.len() < b.len() {
        assert(byte_lt(a@, b@));
        proof { byte_lt_strict(a@, b@); }
        -1
    } else if a.len() > b.len() {
        assert(byte_prefix(b@, a@, i as int));
        assert(byte_lt(b@, a@));
        proof { byte_lt_strict(b@, a@); }
        1
    } else {
        assert(a@ =~= b@);
        0
    }
}

pub open spec fn ordered_names(names: Seq<&str>) -> bool {
    forall|i: int, j: int| 0 <= i < j < names.len()
        ==> byte_lt(#[trigger] names[i].spec_bytes(), #[trigger] names[j].spec_bytes())
}

/// Identity is the complete byte value, independently of string references.
pub open spec fn name_members(names: Seq<&str>) -> Set<Seq<u8>> {
    Set::new(|key: Seq<u8>| exists|i: int| 0 <= i < names.len()
        && (#[trigger] names[i].spec_bytes()) == key)
}

proof fn name_members_push(names: Seq<&str>, next: &str)
    ensures name_members(names.push(next)) == name_members(names).insert(next.spec_bytes()),
{
    assert forall|key: Seq<u8>| name_members(names.push(next)).contains(key)
        == name_members(names).insert(next.spec_bytes()).contains(key) by {
        if name_members(names.push(next)).contains(key) {
            let i = choose|i: int| 0 <= i < names.push(next).len()
                && names.push(next)[i].spec_bytes() == key;
            if i < names.len() { assert(name_members(names).contains(key)); }
        }
        if name_members(names).contains(key) {
            let i = choose|i: int| 0 <= i < names.len() && names[i].spec_bytes() == key;
            assert(names.push(next)[i].spec_bytes() == key);
        }
        if key == next.spec_bytes() {
            assert(names.push(next)[names.len() as int].spec_bytes() == key);
        }
    }
}

proof fn name_members_concat(left: Seq<&str>, right: Seq<&str>)
    ensures name_members(left + right) == name_members(left) + name_members(right),
{
    assert forall|key: Seq<u8>| name_members(left + right).contains(key)
        == (name_members(left) + name_members(right)).contains(key) by {
        if name_members(left + right).contains(key) {
            let i = choose|i: int| 0 <= i < (left + right).len()
                && (left + right)[i].spec_bytes() == key;
            if i < left.len() { assert(name_members(left).contains(key)); }
            else {
                assert(right[i - left.len()].spec_bytes() == key);
                assert(name_members(right).contains(key));
            }
        }
        if name_members(left).contains(key) {
            let i = choose|i: int| 0 <= i < left.len() && left[i].spec_bytes() == key;
            assert((left + right)[i].spec_bytes() == key);
        }
        if name_members(right).contains(key) {
            let i = choose|i: int| 0 <= i < right.len() && right[i].spec_bytes() == key;
            assert((left + right)[left.len() + i].spec_bytes() == key);
        }
    }
}

proof fn name_members_step(names: Seq<&str>, end: int)
    requires 0 <= end < names.len(),
    ensures name_members(names.take(end + 1))
        == name_members(names.take(end)).insert(names[end].spec_bytes()),
{
    assert(names.take(end + 1) =~= names.take(end).push(names[end]));
    name_members_push(names.take(end), names[end]);
}

proof fn ordered_name_count(names: Seq<&str>)
    requires ordered_names(names),
    ensures name_members(names).finite(), name_members(names).len() == names.len(),
    decreases names.len(),
{
    if names.len() == 0 {
        assert(name_members(names) =~= Set::<Seq<u8>>::empty());
    } else {
        let prefix = names.drop_last();
        let last = names.last();
        assert(ordered_names(prefix));
        ordered_name_count(prefix);
        assert(!name_members(prefix).contains(last.spec_bytes())) by {
            if name_members(prefix).contains(last.spec_bytes()) {
                let i = choose|i: int| 0 <= i < prefix.len()
                    && prefix[i].spec_bytes() == last.spec_bytes();
                byte_lt_strict(names[i].spec_bytes(), names[names.len() - 1].spec_bytes());
            }
        }
        name_members_push(prefix, last);
        assert(names =~= prefix.push(last));
    }
}

/// Merge two exact inventories, retaining one representative of equal byte values.
fn merge_names<'a>(left: &[&'a str], right: &[&'a str]) -> (out: Vec<&'a str>)
    requires ordered_names(left@), ordered_names(right@),
        left.len() + right.len() <= usize::MAX,
    ensures ordered_names(out@),
        name_members(out@) == name_members(left@) + name_members(right@),
        out.len() <= left.len() + right.len(),
{
    let mut out: Vec<&str> = Vec::new();
    let mut i: usize = 0;
    let mut j: usize = 0;
    while i < left.len() || j < right.len()
        invariant i <= left.len(), j <= right.len(),
            left.len() + right.len() <= usize::MAX,
            ordered_names(left@), ordered_names(right@), ordered_names(out@),
            out.len() <= i + j,
            name_members(out@) == name_members(left@.take(i as int)) + name_members(right@.take(j as int)),
            forall|a: int, b: int| 0 <= a < out.len() && i <= b < left.len()
                ==> byte_lt(#[trigger] out@[a].spec_bytes(), #[trigger] left@[b].spec_bytes()),
            forall|a: int, b: int| 0 <= a < out.len() && j <= b < right.len()
                ==> byte_lt(#[trigger] out@[a].spec_bytes(), #[trigger] right@[b].spec_bytes()),
        decreases (left.len() - i) + (right.len() - j),
    {
        let ghost old_i = i as int;
        let ghost old_j = j as int;
        let ghost old_out = out@;
        let order = if i == left.len() { 1i8 }
            else if j == right.len() { -1i8 }
            else { compare_bytes(left[i].as_bytes(), right[j].as_bytes()) };
        let next: &str;
        if order <= 0 {
            next = left[i];
            i += 1;
            if order == 0 { j += 1; }
            proof {
                assert forall|k: int| j <= k < right.len()
                    implies byte_lt(next.spec_bytes(), #[trigger] right@[k].spec_bytes()) by {
                    if order == 0 {
                        assert(next.spec_bytes() == right@[old_j].spec_bytes());
                    } else if old_j < k {
                        byte_lt_transitive(next.spec_bytes(), right@[old_j].spec_bytes(), right@[k].spec_bytes());
                    }
                }
                name_members_step(left@, old_i);
                if order == 0 { name_members_step(right@, old_j); }
            }
        } else {
            next = right[j];
            j += 1;
            proof {
                assert forall|k: int| i <= k < left.len()
                    implies byte_lt(next.spec_bytes(), #[trigger] left@[k].spec_bytes()) by {
                    if old_i < k {
                        byte_lt_transitive(next.spec_bytes(), left@[old_i].spec_bytes(), left@[k].spec_bytes());
                    }
                }
                name_members_step(right@, old_j);
            }
        }
        proof {
            assert forall|k: int| 0 <= k < old_out.len()
                implies byte_lt(#[trigger] old_out[k].spec_bytes(), next.spec_bytes()) by {}
            assert forall|k: int| i <= k < left.len()
                implies byte_lt(next.spec_bytes(), #[trigger] left@[k].spec_bytes()) by {}
            assert forall|k: int| j <= k < right.len()
                implies byte_lt(next.spec_bytes(), #[trigger] right@[k].spec_bytes()) by {}
            name_members_push(old_out, next);
            assert(name_members(left@.take(i as int)) + name_members(right@.take(j as int))
                =~= name_members(old_out).insert(next.spec_bytes()));
        }
        out.push(next);
    }
    proof {
        assert(left@.take(i as int) =~= left@);
        assert(right@.take(j as int) =~= right@);
    }
    out
}

/// Produce exactly the supplied names using divide-and-conquer merge and deduplication.
pub fn name_inventory<'a>(names: &[&'a str]) -> (out: Vec<&'a str>)
    ensures ordered_names(out@), name_members(out@) == name_members(names@),
        out.len() <= names.len(), name_members(names@).finite(),
        out.len() == name_members(names@).len(),
    decreases names.len(),
{
    let out = if names.len() <= 1 {
        let mut out = Vec::new();
        if names.len() == 1 { out.push(names[0]); }
        assert(out@ =~= names@);
        out
    } else {
        let mid = names.len() / 2;
        let (a, b) = names.split_at(mid);
        let left = name_inventory(a);
        let right = name_inventory(b);
        let out = merge_names(&left, &right);
        proof {
            assert(names@ =~= a@ + b@);
            name_members_concat(a@, b@);
        }
        out
    };
    proof { ordered_name_count(out@); }
    out
}

/// Check the supplied inventory, including uniqueness, without trusting its producer.
pub fn valid_names(names: &[&str]) -> (valid: bool)
    ensures valid == ordered_names(names@),
{
    let mut end: usize = 1;
    while end < names.len()
        invariant 1 <= end, end <= names.len() || names.len() == 0,
            forall|i: int, j: int| 0 <= i < j < end && j < names.len()
                ==> byte_lt(#[trigger] names@[i].spec_bytes(), #[trigger] names@[j].spec_bytes()),
        decreases names.len() - end,
    {
        let order = compare_bytes(names[end - 1].as_bytes(), names[end].as_bytes());
        if order != -1 {
            proof {
                if order == 0 {
                    if ordered_names(names@) {
                        byte_lt_strict(names@[end as int - 1].spec_bytes(), names@[end as int].spec_bytes());
                    }
                } else {
                    if ordered_names(names@) {
                        byte_lt_transitive(names@[end as int - 1].spec_bytes(), names@[end as int].spec_bytes(),
                            names@[end as int - 1].spec_bytes());
                        byte_lt_strict(names@[end as int - 1].spec_bytes(), names@[end as int - 1].spec_bytes());
                    }
                }
            }
            return false;
        }
        proof {
            assert forall|i: int, j: int| 0 <= i < j < end + 1 && j < names.len()
                implies byte_lt(#[trigger] names@[i].spec_bytes(), #[trigger] names@[j].spec_bytes()) by {
                if j == end && i < end - 1 {
                    byte_lt_transitive(names@[i].spec_bytes(), names@[end as int - 1].spec_bytes(),
                        names@[j].spec_bytes());
                }
            }
        }
        end += 1;
    }
    true
}

/// Exact binary lookup; missing names are never assigned a fallback index.
pub fn name_index(names: &[&str], key: &str) -> (found: Option<usize>)
    requires ordered_names(names@),
    ensures match found {
        Some(i) => i < names.len() && names@[i as int].spec_bytes() == key.spec_bytes(),
        None => forall|i: int| 0 <= i < names.len()
            ==> (#[trigger] names@[i].spec_bytes()) != key.spec_bytes(),
    },
{
    let mut low: usize = 0;
    let mut high = names.len();
    while low < high
        invariant 0 <= low <= high <= names.len(), ordered_names(names@),
            forall|i: int| 0 <= i < low ==> byte_lt(#[trigger] names@[i].spec_bytes(), key.spec_bytes()),
            forall|i: int| high <= i < names.len() ==> byte_lt(key.spec_bytes(), #[trigger] names@[i].spec_bytes()),
        decreases high - low,
    {
        let mid = low + (high - low) / 2;
        let order = compare_bytes(names[mid].as_bytes(), key.as_bytes());
        if order == 0 {
            return Some(mid);
        } else if order == -1 {
            proof {
                assert forall|i: int| 0 <= i < mid + 1
                    implies byte_lt(#[trigger] names@[i].spec_bytes(), key.spec_bytes()) by {
                    if low <= i < mid {
                        byte_lt_transitive(names@[i].spec_bytes(), names@[mid as int].spec_bytes(), key.spec_bytes());
                    }
                }
            }
            low = mid + 1;
        } else {
            proof {
                assert forall|i: int| mid <= i < names.len()
                    implies byte_lt(key.spec_bytes(), #[trigger] names@[i].spec_bytes()) by {
                    if mid < i < high {
                        byte_lt_transitive(key.spec_bytes(), names@[mid as int].spec_bytes(), names@[i].spec_bytes());
                    }
                }
            }
            high = mid;
        }
    }
    proof {
        assert forall|i: int| 0 <= i < names.len()
            implies (#[trigger] names@[i].spec_bytes()) != key.spec_bytes() by {
            if i < low { byte_lt_strict(names@[i].spec_bytes(), key.spec_bytes()); }
            else { byte_lt_strict(key.spec_bytes(), names@[i].spec_bytes()); }
        }
    }
    None
}

pub open spec fn row_names_present(names: Seq<&str>, keys: Seq<&str>) -> bool {
    forall|slot: int| 0 <= slot < keys.len()
        ==> name_members(names).contains(#[trigger] keys[slot].spec_bytes())
}

pub open spec fn name_indices_match(names: Seq<&str>, keys: Seq<&str>, indices: Seq<usize>) -> bool {
    indices.len() == keys.len()
        && forall|slot: int| 0 <= slot < keys.len() ==> {
            let index = #[trigger] indices[slot];
            index < names.len() && names[index as int].spec_bytes() == keys[slot].spec_bytes()
        }
}

pub open spec fn graph_names_present(
    names: Seq<&str>, rows: Seq<Vec<&str>>, roots: Seq<&str>,
) -> bool {
    rows.len() == names.len()
        && (forall|row: int| 0 <= row < rows.len()
            ==> row_names_present(names, #[trigger] rows[row]@))
        && row_names_present(names, roots)
}

pub struct NameIndices {
    pub rows: Vec<Vec<usize>>,
    pub roots: Vec<usize>,
}

fn index_name_row(names: &[&str], keys: &[&str]) -> (out: Option<Vec<usize>>)
    requires ordered_names(names@),
    ensures out.is_some() == row_names_present(names@, keys@),
        match out {
            Some(indices) => name_indices_match(names@, keys@, indices@),
            None => true,
        },
{
    let mut indices: Vec<usize> = Vec::new();
    let mut slot: usize = 0;
    while slot < keys.len()
        invariant slot <= keys.len(), ordered_names(names@), indices.len() == slot,
            name_indices_match(names@, keys@.take(slot as int), indices@),
            row_names_present(names@, keys@.take(slot as int)),
        decreases keys.len() - slot,
    {
        let index = name_index(names, keys[slot]);
        match index {
            Some(index) => {
                assert(name_members(names@).contains(keys@[slot as int].spec_bytes()));
                indices.push(index);
            },
            None => {
                assert(!name_members(names@).contains(keys@[slot as int].spec_bytes()));
                assert(!row_names_present(names@, keys@));
                return None;
            },
        }
        slot += 1;
    }
    assert(keys@.take(slot as int) =~= keys@);
    Some(indices)
}

/// Preserve every supplied adjacency slot and seed position, or reject a missing name.
pub fn index_names(names: &[&str], rows: &[Vec<&str>], roots: &[&str]) -> (out: Option<NameIndices>)
    requires ordered_names(names@),
    ensures
        out.is_some() == graph_names_present(names@, rows@, roots@),
        match out {
            Some(mapped) => mapped.rows.len() == rows.len()
                && valid_graph(mapped.rows@)
                && (forall|row: int| 0 <= row < rows.len()
                    ==> name_indices_match(names@, rows@[row]@, #[trigger] mapped.rows@[row]@))
                && name_indices_match(names@, roots@, mapped.roots@),
            None => true,
        },
{
    if rows.len() != names.len() { return None; }
    let mut mapped: Vec<Vec<usize>> = Vec::new();
    let mut row: usize = 0;
    while row < rows.len()
        invariant row <= rows.len(), rows.len() == names.len(), ordered_names(names@),
            mapped.len() == row,
            forall|r: int| 0 <= r < row
                ==> name_indices_match(names@, rows@[r]@, #[trigger] mapped@[r]@),
            forall|r: int| 0 <= r < row ==> row_names_present(names@, #[trigger] rows@[r]@),
        decreases rows.len() - row,
    {
        let indices = index_name_row(names, &rows[row]);
        match indices {
            Some(indices) => { mapped.push(indices); },
            None => {
                assert(!graph_names_present(names@, rows@, roots@));
                return None;
            },
        }
        row += 1;
    }
    let root_indices = index_name_row(names, roots);
    match root_indices {
        Some(root_indices) => {
            assert(valid_graph(mapped@)) by {
                assert forall|r: int, slot: int| 0 <= r < mapped.len() && 0 <= slot < mapped@[r].len()
                    implies (#[trigger] mapped@[r]@[slot]) < mapped.len() by {}
            }
            Some(NameIndices { rows: mapped, roots: root_indices })
        },
        None => None,
    }
}

pub struct Search {
    /// The graph size denotes an unreachable node; all other values are hop counts.
    pub distances: Vec<usize>,
    /// Source node and adjacency slot, preserving parallel edge identity.
    pub parents: Vec<Option<(usize, usize)>>,
    pub order: Vec<usize>,
}

/// First discovery owns a node permanently. Slots are examined in their supplied
/// order; previously seen targets leave all three state sequences unchanged.
pub open spec fn discovery(
    g: Seq<Vec<usize>>, node: usize, slot: int,
    d: Seq<usize>, p: Seq<Option<(usize, usize)>>, q: Seq<usize>,
) -> (Seq<usize>, Seq<Option<(usize, usize)>>, Seq<usize>) {
    if node < g.len() && 0 <= slot < g[node as int].len() && node < d.len() {
        let target = g[node as int]@[slot];
        if target < d.len() && target < p.len() && d[target as int] == g.len()
            && d[node as int] < usize::MAX
        {
            (d.update(target as int, (d[node as int] + 1) as usize),
                p.update(target as int, Some((node, slot as usize))), q.push(target))
        } else { (d, p, q) }
    } else { (d, p, q) }
}

#[verifier::opaque]
pub open spec fn ordered_slots(
    g: Seq<Vec<usize>>, node: usize, slot: int,
    d: Seq<usize>, p: Seq<Option<(usize, usize)>>, q: Seq<usize>,
) -> (Seq<usize>, Seq<Option<(usize, usize)>>, Seq<usize>)
    decreases if node < g.len() { g[node as int].len() - slot } else { 0 },
{
    if node < g.len() && 0 <= slot < g[node as int].len() {
        let next = discovery(g, node, slot, d, p, q);
        ordered_slots(g, node, slot + 1, next.0, next.1, next.2)
    } else { (d, p, q) }
}

#[verifier::opaque]
pub open spec fn fifo_nodes(
    g: Seq<Vec<usize>>, front: int,
    d: Seq<usize>, p: Seq<Option<(usize, usize)>>, q: Seq<usize>,
) -> (Seq<usize>, Seq<Option<(usize, usize)>>, Seq<usize>)
    decreases g.len() - front,
{
    if 0 <= front < g.len() && front < q.len() {
        let next = ordered_slots(g, q[front], 0, d, p, q);
        fifo_nodes(g, front + 1, next.0, next.1, next.2)
    } else { (d, p, q) }
}

/// Only the seed is discovered initially; every other distance is the sentinel.
pub open spec fn initial_state(n: nat, root: usize)
    -> (Seq<usize>, Seq<Option<(usize, usize)>>, Seq<usize>)
{
    (Seq::new(n, |i: int| if i == root { 0usize } else { n as usize }),
        Seq::new(n, |i: int| None), seq![root])
}

/// A deterministic reference contract: FIFO nodes, ordered adjacency slots and
/// immutable first predecessors. The bound is the number of distinct nodes;
/// the executable traversal separately proves it never leaves queued work out.
pub open spec fn fifo(g: Seq<Vec<usize>>, root: usize)
    -> (Seq<usize>, Seq<Option<(usize, usize)>>, Seq<usize>)
{
    if root < g.len() && g.len() <= usize::MAX {
        let start = initial_state(g.len(), root);
        fifo_nodes(g, 0, start.0, start.1, start.2)
    } else { (Seq::empty(), Seq::empty(), Seq::empty()) }
}

proof fn ordered_slot_step(
    g: Seq<Vec<usize>>, node: usize, slot: int,
    d: Seq<usize>, p: Seq<Option<(usize, usize)>>, q: Seq<usize>,
)
    requires node < g.len(), 0 <= slot < g[node as int].len(),
    ensures ordered_slots(g, node, slot, d, p, q) == {
        let next = discovery(g, node, slot, d, p, q);
        ordered_slots(g, node, slot + 1, next.0, next.1, next.2)
    },
{
    reveal(ordered_slots);
}

pub open spec fn valid_graph(g: Seq<Vec<usize>>) -> bool {
    forall|u: int, k: int| 0 <= u < g.len() && 0 <= k < g[u].len()
        ==> (#[trigger] g[u]@[k]) < g.len()
}

pub open spec fn walk(g: Seq<Vec<usize>>, w: Seq<usize>) -> bool {
    w.len() > 0
        && (forall|i: int| 0 <= i < w.len() ==> (#[trigger] w[i]) < g.len())
        && (forall|i: int| 0 <= i < w.len() - 1
            ==> (#[trigger] g[w[i] as int])@.contains(w[i + 1]))
}

pub open spec fn tree(
    g: Seq<Vec<usize>>, root: usize, d: Seq<usize>, p: Seq<Option<(usize, usize)>>,
) -> bool {
    root < g.len() && d.len() == g.len() && p.len() == g.len()
        && d[root as int] == 0 && p[root as int].is_none()
        && (forall|v: int| 0 <= v < g.len() ==> (#[trigger] d[v]) <= g.len())
        && forall|v: int| #![trigger p[v]] 0 <= v < g.len() ==> (
            if d[v] == g.len() { p[v].is_none() }
                else if v != root {
                    p[v].is_some() && {
                        let (u, k) = p[v].unwrap();
                        u < g.len() && k < g[u as int].len() && g[u as int]@[k as int] == v
                            && d[u as int] < d[v] && d[v] == d[u as int] + 1
                    }
                } else { true }
        )
}

pub open spec fn queue_state(
    g: Seq<Vec<usize>>, root: usize, d: Seq<usize>, p: Seq<Option<(usize, usize)>>,
    q: Seq<usize>,
) -> bool {
    tree(g, root, d, p) && q.len() > 0 && q[0] == root && q.no_duplicates()
        && (forall|i: int| 0 <= i < q.len() ==> (
            (#[trigger] q[i]) < g.len() && d[q[i] as int] < g.len() && d[q[i] as int] <= i
        ))
        && (forall|v: int| 0 <= v < g.len()
            ==> ((#[trigger] d[v]) < g.len() <==> q.contains(v as usize)))
        && (forall|i: int, j: int| 0 <= i <= j < q.len()
            ==> d[#[trigger] q[i] as int] <= d[#[trigger] q[j] as int])
}

pub open spec fn processed(
    g: Seq<Vec<usize>>, d: Seq<usize>, q: Seq<usize>, front: int, slot: int,
) -> bool {
    forall|i: int, k: int| 0 <= i < q.len() && 0 <= k < g[q[i] as int].len()
        && (i < front || (i == front && k < slot)) ==> {
            let v = (#[trigger] g[q[i] as int]@[k]);
            d[v as int] < g.len() && d[v as int] <= d[q[i] as int] + 1
        }
}

pub proof fn bounded_queue(q: Seq<usize>, n: usize)
    requires q.no_duplicates(), forall|i: int| 0 <= i < q.len() ==> (#[trigger] q[i]) < n,
    ensures q.len() <= n,
{
    use vstd::set_lib::FiniteRange;
    q.unique_seq_to_set();
    vstd::set_lib::range_set_properties::<usize>(0, n);
    assert(q.to_set().subset_of(usize::range_set(0, n))) by {
        assert forall|v: usize| q.to_set().contains(v) implies usize::range_set(0, n).contains(v) by {
            let i = choose|i: int| 0 <= i < q.len() && q[i] == v;
        }
    }
    vstd::set_lib::lemma_len_subset(q.to_set(), usize::range_set(0, n));
}

/// A finite predecessor chain supplies an actual path of the recorded length.
pub proof fn tree_path(
    g: Seq<Vec<usize>>, root: usize, d: Seq<usize>, p: Seq<Option<(usize, usize)>>, v: usize,
) -> (w: Seq<usize>)
    requires tree(g, root, d, p), v < g.len(), d[v as int] < g.len(),
    ensures walk(g, w), w[0] == root, w.last() == v, w.len() == d[v as int] + 1,
    decreases d[v as int],
{
    if v == root {
        seq![root]
    } else {
        let (u, k) = p[v as int].unwrap();
        let prefix = tree_path(g, root, d, p, u);
        let w = prefix.push(v);
        assert forall|i: int| 0 <= i < w.len() - 1
            implies (#[trigger] g[w[i] as int])@.contains(w[i + 1]) by {
            if i == prefix.len() - 1 {
                assert(g[u as int]@[k as int] == v);
            }
        }
        w
    }
}

pub open spec fn closed(g: Seq<Vec<usize>>, d: Seq<usize>) -> bool {
    forall|u: int, k: int| 0 <= u < g.len() && d[u] < g.len() && 0 <= k < g[u].len()
        ==> {
            let v = (#[trigger] g[u]@[k]);
            d[v as int] < g.len() && d[v as int] <= d[u] + 1
        }
}

/// Edge closure gives reachability completeness and a lower bound for every walk.
pub proof fn walk_bound(g: Seq<Vec<usize>>, d: Seq<usize>, root: usize, w: Seq<usize>)
    requires valid_graph(g), d.len() == g.len(), root < g.len(), d[root as int] == 0,
        closed(g, d), walk(g, w), w[0] == root,
    ensures d[w.last() as int] < g.len(), d[w.last() as int] <= w.len() - 1,
    decreases w.len(),
{
    if w.len() > 1 {
        let prefix = w.drop_last();
        assert(walk(g, prefix));
        walk_bound(g, d, root, prefix);
        let u = prefix.last();
        let v = w.last();
        let k = choose|k: int| 0 <= k < g[u as int].len() && g[u as int]@[k] == v;
        assert(d[v as int] <= d[u as int] + 1);
    }
}

#[verifier::opaque]
pub open spec fn complete(g: Seq<Vec<usize>>, root: usize, s: Search) -> bool {
    queue_state(g, root, s.distances@, s.parents@, s.order@)
        && closed(g, s.distances@)
        && (forall|v: usize| v < g.len() ==> (
            s.distances@[v as int] < g.len() <==>
                exists|w: Seq<usize>| walk(g, w) && w[0] == root && w.last() == v
        ))
        && (forall|w: Seq<usize>| walk(g, w) && w[0] == root
            ==> s.distances@[w.last() as int] <= w.len() - 1)
}

/// Validate once before traversing several roots of the same immutable graph.
pub fn valid_indices(g: &[Vec<usize>]) -> (valid: bool)
    ensures valid == valid_graph(g@),
{
    let n = g.len();
    let mut u = 0;
    while u < n
        invariant n == g.len(), u <= n,
            forall|i: int, k: int| 0 <= i < u && 0 <= k < g[i].len()
                ==> (#[trigger] g[i]@[k]) < n,
        decreases n - u,
    {
        let mut k = 0;
        while k < g[u].len()
            invariant n == g.len(), u < n, k <= g[u as int].len(),
                forall|i: int, j: int| 0 <= i < u && 0 <= j < g[i].len()
                    ==> (#[trigger] g[i]@[j]) < n,
                forall|j: int| 0 <= j < k ==> (#[trigger] g[u as int]@[j]) < n,
            decreases g[u as int].len() - k,
        {
            if g[u][k] >= n { return false; }
            k += 1;
        }
        u += 1;
    }
    true
}

/// One adjacency transition, separated from loop bookkeeping for local proofs.
fn expand_edge(
    g: &[Vec<usize>], _root: usize, d: &mut Vec<usize>,
    p: &mut Vec<Option<(usize, usize)>>, q: &mut Vec<usize>, front: usize, slot: usize,
)
    requires
        valid_graph(g@), queue_state(g@, _root, old(d)@, old(p)@, old(q)@),
        front < old(q).len(), slot < g[old(q)@[front as int] as int].len(),
        forall|i: int| 0 <= i < old(q).len()
            ==> old(d)@[#[trigger] old(q)@[i] as int] <= old(d)@[old(q)@[front as int] as int] + 1,
        processed(g@, old(d)@, old(q)@, front as int, slot as int),
    ensures
        queue_state(g@, _root, d@, p@, q@), q.len() <= g.len(), front < q.len(),
        (d@, p@, q@) == discovery(g@, old(q)@[front as int], slot as int,
            old(d)@, old(p)@, old(q)@),
        q@[front as int] == old(q)@[front as int],
        forall|i: int| 0 <= i < q.len()
            ==> d@[#[trigger] q@[i] as int] <= d@[q@[front as int] as int] + 1,
        processed(g@, d@, q@, front as int, slot as int + 1),
{
    let n = g.len();
    let node = q[front];
    let v = g[node][slot];
    if d[v] == n {
        let ghost old_d = d@;
        let ghost old_p = p@;
        let ghost old_q = q@;
        proof {
            assert(!q@.contains(v));
            bounded_queue(q@.push(v), n);
        }
        let depth = d[node] + 1;
        d[v] = depth;
        p[v] = Some((node, slot));
        q.push(v);
        proof {
            assert(q@.no_duplicates());
            assert(tree(g@, _root, d@, p@));
            assert forall|i: int| 0 <= i < q.len() implies (
                (#[trigger] q[i]) < n && d[q[i] as int] < n && d[q[i] as int] <= i
            ) by {
                if i < old_q.len() { assert(old_q[i] != v); }
            }
            assert forall|x: int| 0 <= x < n implies
                ((#[trigger] d[x]) < n <==> q@.contains(x as usize)) by {
                vstd::seq_lib::lemma_seq_contains_after_push(old_q, v, x as usize);
                if x != v { assert(d[x] == old_d[x]); }
            }
            assert forall|i: int, j: int| 0 <= i <= j < q.len()
                implies d[#[trigger] q[i] as int] <= d[#[trigger] q[j] as int] by {
                if j < old_q.len() { assert(old_q[i] != v && old_q[j] != v); }
                else if i < old_q.len() { assert(old_q[i] != v); }
            }
            assert(processed(g@, d@, q@, front as int, slot as int));
        }
    }
    proof {
        bounded_queue(q@, n);
        assert forall|i: int, k: int| 0 <= i < q.len() && 0 <= k < g[q[i] as int].len()
            && (i < front || (i == front && k < slot + 1)) implies {
                let target = (#[trigger] g[q[i] as int]@[k]);
                d[target as int] < n && d[target as int] <= d[q[i] as int] + 1
            } by {
            if i == front && k == slot { assert(g[q[i] as int]@[k] == v); }
        }
    }
}

/// FIFO traversal of explicitly validated immutable adjacency. The caller must
/// establish valid_indices once; an out-of-range root is rejected here.
pub fn breadth_first(g: &[Vec<usize>], root: usize) -> (result: Option<Search>)
    requires valid_graph(g@),
    ensures
        result.is_some() == (root < g.len()),
        result.is_some() ==> complete(g@, root, result.unwrap()),
        result.is_some() ==> {
            let search = result.unwrap();
            (search.distances@, search.parents@, search.order@) == fifo(g@, root)
        },
{
    reveal(complete);
    let n = g.len();
    if root >= n { return None; }
    let mut d: Vec<usize> = Vec::new();
    let mut p: Vec<Option<(usize, usize)>> = Vec::new();
    while d.len() < n
        invariant n == g.len(), root < n, valid_graph(g@),
            d.len() <= n, p.len() == d.len(),
            forall|i: int| 0 <= i < d.len() ==> (#[trigger] d[i]) == n && p[i].is_none(),
        decreases n - d.len(),
    {
        d.push(n);
        p.push(None);
    }
    proof { assert(p[root as int].is_none()) by { assert(d[root as int] == n); } }
    d[root] = 0;
    let mut q = Vec::new();
    q.push(root);
    proof {
        assert(q@ == seq![root]);
        assert(d.len() == g.len() && p.len() == g.len());
        assert(d[root as int] == 0 && p[root as int].is_none());
        assert forall|x: int| 0 <= x < n implies
            d[x] == (if x == root { 0usize } else { n }) && p[x].is_none() by {}
        assert forall|x: int| 0 <= x < n implies
            ((#[trigger] d[x]) < n <==> q@.contains(x as usize)) by {
            if x == root { assert(q@[0] == x); }
        }
        assert(tree(g@, root, d@, p@));
        assert(q@.no_duplicates());
        assert forall|i: int| 0 <= i < q.len() implies (
            (#[trigger] q[i]) < n && d[q[i] as int] < n && d[q[i] as int] <= i
        ) by {}
        assert forall|i: int, j: int| 0 <= i <= j < q.len()
            implies d[#[trigger] q[i] as int] <= d[#[trigger] q[j] as int] by {}
        assert(queue_state(g@, root, d@, p@, q@));
        assert(g@.len() == n && g@.len() <= usize::MAX);
        let start = initial_state(g@.len(), root);
        assert(d@ =~= start.0);
        assert(p@ =~= start.1);
        assert(q@ == start.2);
        reveal(fifo);
        assert(fifo_nodes(g@, 0, d@, p@, q@) == fifo(g@, root));
    }
    let mut front = 0;
    while front < q.len()
        invariant
            root < n, n == g.len(), valid_graph(g@),
            queue_state(g@, root, d@, p@, q@), front <= q.len(), q.len() <= n,
            front < q.len() ==> (forall|i: int| 0 <= i < q.len()
                ==> d[#[trigger] q[i] as int] <= d[q[front as int] as int] + 1),
            processed(g@, d@, q@, front as int, 0),
            fifo_nodes(g@, front as int, d@, p@, q@) == fifo(g@, root),
        decreases n - front,
    {
        let node = q[front];
        let ghost row_result = ordered_slots(g@, node, 0, d@, p@, q@);
        proof {
            reveal(fifo_nodes);
            assert(fifo_nodes(g@, front as int + 1, row_result.0, row_result.1, row_result.2)
                == fifo(g@, root));
        }
        let mut slot = 0;
        while slot < g[node].len()
            invariant
                root < n, n == g.len(), valid_graph(g@),
                queue_state(g@, root, d@, p@, q@), front < q.len(), q.len() <= n,
                node == q[front as int], slot <= g[node as int].len(),
                forall|i: int| 0 <= i < q.len()
                    ==> d[#[trigger] q[i] as int] <= d[node as int] + 1,
                processed(g@, d@, q@, front as int, slot as int),
                ordered_slots(g@, node, slot as int, d@, p@, q@) == row_result,
                fifo_nodes(g@, front as int + 1, row_result.0, row_result.1, row_result.2)
                    == fifo(g@, root),
            decreases g[node as int].len() - slot,
        {
            proof { ordered_slot_step(g@, node, slot as int, d@, p@, q@); }
            expand_edge(g, root, &mut d, &mut p, &mut q, front, slot);
            slot += 1;
        }
        proof {
            reveal(ordered_slots);
            assert((d@, p@, q@) == row_result);
        }
        front += 1;
    }
    proof {
        reveal(fifo_nodes);
        assert((d@, p@, q@) == fifo(g@, root));
        assert(closed(g@, d@)) by {
            assert forall|u: int, k: int| 0 <= u < n && d[u] < n && 0 <= k < g[u].len()
                implies {
                    let v = (#[trigger] g[u]@[k]);
                    d[v as int] < n && d[v as int] <= d[u] + 1
                } by {
                let i = choose|i: int| 0 <= i < q.len() && q[i] == u;
            }
        }
        assert forall|v: usize| v < n implies (
            d[v as int] < n <==> exists|w: Seq<usize>| walk(g@, w) && w[0] == root && w.last() == v
        ) by {
            if d[v as int] < n {
                let w = tree_path(g@, root, d@, p@, v);
            } else if exists|w: Seq<usize>| walk(g@, w) && w[0] == root && w.last() == v {
                let w = choose|w: Seq<usize>| walk(g@, w) && w[0] == root && w.last() == v;
                walk_bound(g@, d@, root, w);
            }
        }
        assert forall|w: Seq<usize>| walk(g@, w) && w[0] == root
            implies d[w.last() as int] <= w.len() - 1 by {
            walk_bound(g@, d@, root, w);
        }
    }
    Some(Search { distances: d, parents: p, order: q })
}

}
