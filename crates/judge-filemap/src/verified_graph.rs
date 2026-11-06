//! Typed edge ordering, exact name lookup and indexed FIFO graph traversal.
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
use core::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

#[derive(Copy)]
pub enum EdgeKind {
    Compile,
    BuildInput,
    RuntimeInput,
    TestExecution,
    JudgeTrigger,
}

impl Clone for EdgeKind {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    { *self }
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

pub struct Edge {
    pub from: String,
    pub kind: EdgeKind,
    pub to: String,
}

#[derive(Copy)]
pub enum Origin {
    Base,
    Candidate,
    Both,
}

impl Clone for Origin {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    { *self }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Origin {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { *self == *other }
}

impl PartialEq for Origin {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (*self == *other),
    {
        match (self, other) {
            (Self::Base, Self::Base) | (Self::Candidate, Self::Candidate)
                | (Self::Both, Self::Both) => true,
            _ => false,
        }
    }
}

impl Eq for Origin {}

pub struct UnionEdge {
    pub edge: Edge,
    pub origin: Origin,
}

impl Clone for UnionEdge {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    { Self { edge: self.edge.clone(), origin: self.origin } }
}

impl Edge {
    pub open spec fn key(&self) -> (Seq<u8>, u8, Seq<u8>) {
        (self.from_bytes(), self.kind.rank_spec(), self.to_bytes())
    }
    pub open spec fn from_bytes(&self) -> Seq<u8> {
        vstd::utf8::encode_utf8(self.from@)
    }

    pub open spec fn to_bytes(&self) -> Seq<u8> {
        vstd::utf8::encode_utf8(self.to@)
    }

    pub open spec fn same(&self, other: &Self) -> bool {
        self.from_bytes() == other.from_bytes() && self.kind == other.kind
            && self.to_bytes() == other.to_bytes()
    }

    pub open spec fn less(&self, other: &Self) -> bool {
        byte_lt(self.from_bytes(), other.from_bytes())
            || self.from_bytes() == other.from_bytes() && (
                self.kind.rank_spec() < other.kind.rank_spec()
                || self.kind == other.kind && byte_lt(self.to_bytes(), other.to_bytes()))
    }

    pub open spec fn order_spec(&self, other: &Self) -> Ordering {
        if self.same(other) { Ordering::Equal }
        else if self.less(other) { Ordering::Less }
        else { Ordering::Greater }
    }

    pub fn compare(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        proof { edge_order_laws(self, other, other); }
        let from = compare_bytes(self.from.as_str().as_bytes(), other.from.as_str().as_bytes());
        if from < 0 { return Ordering::Less; }
        if from > 0 { return Ordering::Greater; }
        let kind = self.kind.compare(&other.kind);
        match kind {
            Ordering::Less => { return Ordering::Less; },
            Ordering::Greater => { return Ordering::Greater; },
            Ordering::Equal => {},
        }
        let to = compare_bytes(self.to.as_str().as_bytes(), other.to.as_str().as_bytes());
        if to < 0 { Ordering::Less }
        else if to > 0 { Ordering::Greater }
        else { Ordering::Equal }
    }
}

impl Clone for Edge {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        Self { from: self.from.clone(), kind: self.kind, to: self.to.clone() }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Edge {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self.same(other) }
}

impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == self.same(other),
    {
        match self.compare(other) {
            Ordering::Equal => true,
            _ => false,
        }
    }
}

impl Eq for Edge {}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::OrdSpecImpl for Edge {
    open spec fn obeys_cmp_spec() -> bool { true }
    open spec fn cmp_spec(&self, other: &Self) -> Ordering { self.order_spec(other) }
}

impl Ord for Edge {
    fn cmp(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        self.compare(other)
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialOrdSpecImpl for Edge {
    open spec fn obeys_partial_cmp_spec() -> bool { true }
    open spec fn partial_cmp_spec(&self, other: &Self) -> Option<Ordering> {
        Some(self.order_spec(other))
    }
}

impl PartialOrd for Edge {
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

proof fn byte_lt_asymmetric(a: Seq<u8>, b: Seq<u8>)
    ensures byte_lt(a, b) ==> !byte_lt(b, a),
{
    if byte_lt(a, b) && byte_lt(b, a) {
        byte_lt_transitive(a, b, a);
        byte_lt_strict(a, a);
    }
}

proof fn byte_trichotomy(a: Seq<u8>, b: Seq<u8>)
    ensures a == b || byte_lt(a, b) || byte_lt(b, a),
    decreases a.len() + b.len(),
{
    if a.len() == 0 {
        if b.len() == 0 { assert(a =~= b); }
        else { assert(byte_prefix(a, b, 0)); assert(byte_lt(a, b)); }
    } else if b.len() == 0 {
        assert(byte_prefix(b, a, 0)); assert(byte_lt(b, a));
    } else if a[0] < b[0] {
        assert(byte_prefix(a, b, 0)); assert(byte_lt(a, b));
    } else if b[0] < a[0] {
        assert(byte_prefix(b, a, 0)); assert(byte_lt(b, a));
    } else {
        let x = a.subrange(1, a.len() as int);
        let y = b.subrange(1, b.len() as int);
        byte_trichotomy(x, y);
        if x == y {
            assert(a.len() == b.len());
            assert(a =~= b) by {
                assert forall|j: int| 0 <= j < a.len() implies a[j] == b[j] by {
                    if j > 0 {
                        assert(a[j] == x[j - 1]);
                        assert(b[j] == y[j - 1]);
                    }
                }
            }
        } else if byte_lt(x, y) {
            let i = choose|i: int| 0 <= i <= x.len() && i < y.len() && byte_prefix(x, y, i)
                && (i == x.len() || x[i] < y[i]);
            assert(byte_prefix(a, b, i + 1)) by {
                assert forall|j: int| 0 <= j < i + 1 implies a[j] == b[j] by {
                    if j > 0 { assert(x[j - 1] == y[j - 1]); }
                }
            }
            assert(byte_lt(a, b));
        } else {
            let i = choose|i: int| 0 <= i <= y.len() && i < x.len() && byte_prefix(y, x, i)
                && (i == y.len() || y[i] < x[i]);
            assert(byte_prefix(b, a, i + 1)) by {
                assert forall|j: int| 0 <= j < i + 1 implies b[j] == a[j] by {
                    if j > 0 { assert(y[j - 1] == x[j - 1]); }
                }
            }
            assert(byte_lt(b, a));
        }
    }
}

pub proof fn edge_order_laws(a: &Edge, b: &Edge, c: &Edge)
    ensures
        (a.order_spec(b) == Ordering::Equal) == a.same(b),
        (a.order_spec(b) == Ordering::Less) == (b.order_spec(a) == Ordering::Greater),
        a.order_spec(b) == Ordering::Less && b.order_spec(c) == Ordering::Less
            ==> a.order_spec(c) == Ordering::Less,
{
    byte_trichotomy(a.from_bytes(), b.from_bytes());
    byte_trichotomy(a.to_bytes(), b.to_bytes());
    byte_lt_asymmetric(a.from_bytes(), b.from_bytes());
    byte_lt_asymmetric(a.to_bytes(), b.to_bytes());
    if byte_lt(a.from_bytes(), b.from_bytes()) { byte_lt_strict(a.from_bytes(), b.from_bytes()); }
    if byte_lt(b.from_bytes(), a.from_bytes()) { byte_lt_strict(b.from_bytes(), a.from_bytes()); }
    if byte_lt(a.to_bytes(), b.to_bytes()) { byte_lt_strict(a.to_bytes(), b.to_bytes()); }
    if byte_lt(b.to_bytes(), a.to_bytes()) { byte_lt_strict(b.to_bytes(), a.to_bytes()); }
    edge_kind_order_laws(a.kind, b.kind, c.kind);
    if a.less(b) && b.less(c) {
        if byte_lt(a.from_bytes(), b.from_bytes()) && byte_lt(b.from_bytes(), c.from_bytes()) {
            byte_lt_transitive(a.from_bytes(), b.from_bytes(), c.from_bytes());
        }
        if a.from_bytes() == b.from_bytes() && b.from_bytes() == c.from_bytes()
            && a.kind == b.kind && b.kind == c.kind
            && byte_lt(a.to_bytes(), b.to_bytes()) && byte_lt(b.to_bytes(), c.to_bytes()) {
            byte_lt_transitive(a.to_bytes(), b.to_bytes(), c.to_bytes());
        }
        assert(a.less(c));
        if byte_lt(a.from_bytes(), c.from_bytes()) { byte_lt_strict(a.from_bytes(), c.from_bytes()); }
        if byte_lt(a.to_bytes(), c.to_bytes()) { byte_lt_strict(a.to_bytes(), c.to_bytes()); }
    }
}

pub proof fn edge_obeys_cmp()
    ensures vstd::laws_cmp::obeys_cmp::<Edge>(),
{
    use vstd::std_specs::cmp::{PartialEqSpec, PartialOrdSpec, OrdSpec};
    reveal(vstd::laws_eq::obeys_eq_spec_properties);
    reveal(vstd::laws_cmp::obeys_partial_cmp_spec_properties);
    reveal(vstd::laws_cmp::obeys_cmp_partial_ord);
    reveal(vstd::laws_cmp::obeys_cmp_ord);
    assert forall|a: Edge, b: Edge|
        a.partial_cmp_spec(&b) == Some(Ordering::Less)
            <==> b.partial_cmp_spec(&a) == Some(Ordering::Greater) by {
        edge_order_laws(&a, &b, &b);
    }
    assert forall|a: Edge, b: Edge, c: Edge|
        a.partial_cmp_spec(&b) == Some(Ordering::Less)
            && b.partial_cmp_spec(&c) == Some(Ordering::Less)
            implies a.partial_cmp_spec(&c) == Some(Ordering::Less) by {
        edge_order_laws(&a, &b, &c);
    }
    assert forall|a: Edge, b: Edge, c: Edge|
        a.partial_cmp_spec(&b) == Some(Ordering::Greater)
            && b.partial_cmp_spec(&c) == Some(Ordering::Greater)
            implies a.partial_cmp_spec(&c) == Some(Ordering::Greater) by {
        edge_order_laws(&a, &b, &c);
        edge_order_laws(&b, &c, &a);
        edge_order_laws(&c, &b, &a);
    }
}

/// Preserve the exact BTreeSet row while exposing its proved full edge order.
pub fn edge_inventory<'a>(edges: &BTreeSet<&'a Edge>) -> (out: Vec<&'a Edge>)
    ensures
        out@.to_set() == edges@,
        out@.no_duplicates(),
        out.len() == edges@.len(),
        forall|i: int, j: int| 0 <= i < j < out.len()
            ==> (#[trigger] out@[i]).order_spec(#[trigger] out@[j]) == Ordering::Less,
{
    proof {
        edge_obeys_cmp();
        broadcast use vstd::laws_cmp::lemma_ref_obeys_cmp_spec;
        assert(vstd::laws_cmp::obeys_cmp::<&Edge>());
    }
    let mut iter = edges.iter();
    let ghost contents = iter@.1;
    proof {
        use vstd::std_specs::cmp::OrdSpec;
        broadcast use vstd::std_specs::btree::axiom_increasing_seq_meaning;
        assert forall|i: int, j: int| 0 <= i < j < contents.len()
            implies (#[trigger] contents[i]).order_spec(#[trigger] contents[j]) == Ordering::Less by {
            assert(<&Edge as OrdSpec>::cmp_spec(&contents[i], &contents[j]) == Ordering::Less);
        }
        contents.unique_seq_to_set();
    }
    let mut out = Vec::with_capacity(edges.len());
    loop
        invariant
            iter@.1 == contents,
            0 <= iter@.0 <= contents.len(),
            out@ == contents.take(iter@.0),
            contents.to_set() == edges@,
            contents.no_duplicates(),
            contents.len() == edges@.len(),
            forall|i: int, j: int| 0 <= i < j < contents.len()
                ==> (#[trigger] contents[i]).order_spec(#[trigger] contents[j]) == Ordering::Less,
        decreases contents.len() - iter@.0,
    {
        match iter.next() {
            None => {
                assert(out@ == contents);
                return out;
            },
            Some(edge) => { out.push(*edge); },
        }
    }
}

#[cfg(verus_keep_ghost)]
pub type EdgeKey = (Seq<u8>, u8, Seq<u8>);

pub open spec fn reference_edge_keys(edges: Seq<&Edge>) -> Seq<EdgeKey> {
    edges.map_values(|edge: &Edge| edge.key())
}

pub open spec fn union_edge_keys(edges: Seq<UnionEdge>) -> Seq<EdgeKey> {
    edges.map_values(|edge: UnionEdge| edge.edge.key())
}

#[verifier::opaque]
pub open spec fn ordered_edges(edges: Seq<&Edge>) -> bool {
    forall|i: int, j: int| 0 <= i < j < edges.len()
        ==> (#[trigger] edges[i]).order_spec(#[trigger] edges[j]) == Ordering::Less
}

#[verifier::opaque]
pub open spec fn ordered_union(edges: Seq<UnionEdge>) -> bool {
    forall|i: int, j: int| 0 <= i < j < edges.len()
        ==> (#[trigger] edges[i]).edge.order_spec(&(#[trigger] edges[j]).edge) == Ordering::Less
}

pub open spec fn exact_origin(origin: Origin, in_base: bool, in_candidate: bool) -> bool {
    match origin {
        Origin::Base => in_base && !in_candidate,
        Origin::Candidate => !in_base && in_candidate,
        Origin::Both => in_base && in_candidate,
    }
}

fn owned_edge_inventory<'a>(edges: &'a BTreeSet<Edge>) -> (out: Vec<&'a Edge>)
    ensures out.len() == edges@.len(), ordered_edges(out@),
        reference_edge_keys(out@).to_set() == edges@.map(|edge: Edge| edge.key()),
{
    proof { edge_obeys_cmp(); reveal(ordered_edges); }
    let mut iter = edges.iter();
    let ghost contents = iter@.1;
    proof {
        use vstd::std_specs::cmp::OrdSpec;
        broadcast use vstd::std_specs::btree::axiom_increasing_seq_meaning;
        assert forall|i: int, j: int| 0 <= i < j < contents.len()
            implies #[trigger] contents[i].order_spec(&contents[j]) == Ordering::Less by {
            assert(contents[i].cmp_spec(&contents[j]) == Ordering::Less);
        }
        contents.unique_seq_to_set();
    }
    let mut out: Vec<&Edge> = Vec::with_capacity(edges.len());
    loop
        invariant iter@.1 == contents, 0 <= iter@.0 <= contents.len(),
            out.len() == iter@.0,
            forall|i: int| 0 <= i < out.len() ==> *(#[trigger] out@[i]) == contents[i],
            contents.to_set() == edges@, contents.len() == edges@.len(),
            forall|i: int, j: int| 0 <= i < j < contents.len()
                ==> #[trigger] contents[i].order_spec(&contents[j]) == Ordering::Less,
        decreases contents.len() - iter@.0,
    {
        match iter.next() {
            None => {
                proof {
                    reveal(ordered_edges);
                    contents.lemma_to_set_map_commutes(|edge: Edge| edge.key());
                    assert(reference_edge_keys(out@) =~= contents.map_values(|edge: Edge| edge.key()));
                    assert forall|i: int, j: int| 0 <= i < j < out.len()
                        implies (#[trigger] out@[i]).order_spec(#[trigger] out@[j]) == Ordering::Less by {
                        assert(*out@[i] == contents[i]);
                        assert(*out@[j] == contents[j]);
                        assert(contents[i].order_spec(&contents[j]) == Ordering::Less);
                    }
                }
                return out;
            },
            Some(edge) => { out.push(edge); },
        }
    }
}

proof fn less_keys_differ(a: &Edge, b: &Edge)
    requires a.order_spec(b) == Ordering::Less,
    ensures a.key() != b.key(),
{
    edge_kind_order_laws(a.kind, b.kind, b.kind);
    if a.key() == b.key() {
        if byte_lt(a.from_bytes(), b.from_bytes()) {
            byte_lt_strict(a.from_bytes(), b.from_bytes());
        }
        if byte_lt(a.to_bytes(), b.to_bytes()) {
            byte_lt_strict(a.to_bytes(), b.to_bytes());
        }
    }
}

proof fn selected_before_tail(edges: Seq<&Edge>, at: int, edge: &Edge, take: bool)
    requires ordered_edges(edges), 0 <= at <= edges.len(),
        take ==> at < edges.len() && edge.same(edges[at]),
        !take && at < edges.len() ==> edge.order_spec(edges[at]) == Ordering::Less,
    ensures forall|j: int| at + (if take { 1int } else { 0int }) <= j < edges.len()
        ==> edge.order_spec(#[trigger] edges[j]) == Ordering::Less,
{
    reveal(ordered_edges);
    assert forall|j: int| at + (if take { 1int } else { 0int }) <= j < edges.len()
        implies edge.order_spec(#[trigger] edges[j]) == Ordering::Less by {
        if j > at {
            assert(edges[at].order_spec(edges[j]) == Ordering::Less);
            if !take { edge_order_laws(edge, edges[at], edges[j]); }
        }
    }
}

proof fn excluded_key(edges: Seq<&Edge>, at: int, out: Seq<UnionEdge>, edge: &Edge)
    requires 0 <= at <= edges.len(),
        reference_edge_keys(edges.take(at)).to_set().subset_of(union_edge_keys(out).to_set()),
        forall|i: int| 0 <= i < out.len()
            ==> (#[trigger] out[i]).edge.order_spec(edge) == Ordering::Less,
        forall|j: int| at <= j < edges.len()
            ==> edge.order_spec(#[trigger] edges[j]) == Ordering::Less,
    ensures !reference_edge_keys(edges).to_set().contains(edge.key()),
{
    assert forall|j: int| 0 <= j < edges.len()
        implies (#[trigger] edges[j]).key() != edge.key() by {
        if j >= at { less_keys_differ(edge, edges[j]); }
        else {
            assert(reference_edge_keys(edges.take(at))[j] == edges[j].key());
            assert(reference_edge_keys(edges.take(at)).to_set().contains(edges[j].key()));
            assert(union_edge_keys(out).to_set().contains(edges[j].key()));
            let i = choose|i: int| 0 <= i < out.len() && out[i].edge.key() == edges[j].key();
            less_keys_differ(&out[i].edge, edge);
        }
    }
    if reference_edge_keys(edges).to_set().contains(edge.key()) {
        let j = choose|j: int| 0 <= j < edges.len() && reference_edge_keys(edges)[j] == edge.key();
        assert(edges[j].key() == edge.key());
    }
}

proof fn ordered_union_push(out: Seq<UnionEdge>, next: UnionEdge)
    requires ordered_union(out),
        forall|i: int| 0 <= i < out.len()
            ==> (#[trigger] out[i]).edge.order_spec(&next.edge) == Ordering::Less,
    ensures ordered_union(out.push(next)),
{
    reveal(ordered_union);
}

#[verifier::opaque]
pub closed spec fn union_origins(out: Seq<UnionEdge>, base: Seq<&Edge>, candidate: Seq<&Edge>) -> bool {
    forall|i: int| 0 <= i < out.len() ==> exact_origin((#[trigger] out[i]).origin,
        reference_edge_keys(base).to_set().contains(out[i].edge.key()),
        reference_edge_keys(candidate).to_set().contains(out[i].edge.key()))
}

#[verifier::opaque]
closed spec fn union_prefix(out: Seq<UnionEdge>, base: Seq<&Edge>, candidate: Seq<&Edge>, bi: int, ci: int) -> bool {
    union_edge_keys(out).to_set() == reference_edge_keys(base.take(bi)).to_set()
        + reference_edge_keys(candidate.take(ci)).to_set()
}

#[verifier::opaque]
closed spec fn before_rest(out: Seq<UnionEdge>, edges: Seq<&Edge>, at: int) -> bool {
    forall|i: int, j: int| 0 <= i < out.len() && at <= j < edges.len()
        ==> (#[trigger] out[i]).edge.order_spec(#[trigger] edges[j]) == Ordering::Less
}

#[verifier::opaque]
closed spec fn merge_state(out: Seq<UnionEdge>, base: Seq<&Edge>, candidate: Seq<&Edge>, bi: int, ci: int) -> bool {
    ordered_union(out) && union_origins(out, base, candidate)
        && union_prefix(out, base, candidate, bi, ci)
        && before_rest(out, base, bi) && before_rest(out, candidate, ci)
}

spec fn head_choice(base: Seq<&Edge>, candidate: Seq<&Edge>, bi: int, ci: int,
    edge: &Edge, origin: Origin, tb: bool, tc: bool) -> bool {
    (tb || tc) && exact_origin(origin, tb, tc)
        && (tb ==> bi < base.len() && edge.same(base[bi]))
        && (tc ==> ci < candidate.len() && edge.same(candidate[ci]))
        && (!tb && bi < base.len() ==> edge.order_spec(base[bi]) == Ordering::Less)
        && (!tc && ci < candidate.len() ==> edge.order_spec(candidate[ci]) == Ordering::Less)
}

fn union_head<'a>(base: &[&'a Edge], candidate: &[&'a Edge], bi: usize, ci: usize)
    -> (choice: (&'a Edge, Origin, bool, bool))
    requires bi <= base.len(), ci <= candidate.len(), bi < base.len() || ci < candidate.len(),
    ensures head_choice(base@, candidate@, bi as int, ci as int, choice.0, choice.1, choice.2, choice.3),
{
    if bi == base.len() { (candidate[ci], Origin::Candidate, false, true) }
    else if ci == candidate.len() { (base[bi], Origin::Base, true, false) }
    else {
        match base[bi].compare(candidate[ci]) {
            Ordering::Less => (base[bi], Origin::Base, true, false),
            Ordering::Greater => {
                proof { edge_order_laws(base@[bi as int], candidate@[ci as int], candidate@[ci as int]); }
                (candidate[ci], Origin::Candidate, false, true)
            },
            Ordering::Equal => (base[bi], Origin::Both, true, true),
        }
    }
}

proof fn merge_order_step(out: Seq<UnionEdge>, base: Seq<&Edge>, candidate: Seq<&Edge>,
    bi: int, ci: int, next: UnionEdge, tb: bool, tc: bool)
    requires 0 <= bi <= base.len(), 0 <= ci <= candidate.len(),
        ordered_edges(base), ordered_edges(candidate), ordered_union(out),
        before_rest(out, base, bi), before_rest(out, candidate, ci),
        head_choice(base, candidate, bi, ci, &next.edge, next.origin, tb, tc),
    ensures ordered_union(out.push(next)),
        before_rest(out.push(next), base, bi + (if tb { 1int } else { 0int })),
        before_rest(out.push(next), candidate, ci + (if tc { 1int } else { 0int })),
{
    reveal(before_rest);
    selected_before_tail(base, bi, &next.edge, tb);
    selected_before_tail(candidate, ci, &next.edge, tc);
    assert forall|i: int| 0 <= i < out.len()
        implies (#[trigger] out[i]).edge.order_spec(&next.edge) == Ordering::Less by {
        if tb { assert(out[i].edge.order_spec(base[bi]) == Ordering::Less); }
        else { assert(out[i].edge.order_spec(candidate[ci]) == Ordering::Less); }
    }
    ordered_union_push(out, next);
}

proof fn merge_members_step(out: Seq<UnionEdge>, base: Seq<&Edge>, candidate: Seq<&Edge>,
    bi: int, ci: int, next: UnionEdge, tb: bool, tc: bool)
    requires 0 <= bi <= base.len(), 0 <= ci <= candidate.len(),
        union_prefix(out, base, candidate, bi, ci),
        head_choice(base, candidate, bi, ci, &next.edge, next.origin, tb, tc),
    ensures union_prefix(out.push(next), base, candidate,
        bi + (if tb { 1int } else { 0int }), ci + (if tc { 1int } else { 0int })),
{
    reveal(union_prefix);
    if tb {
        assert(reference_edge_keys(base.take(bi + 1)) =~= reference_edge_keys(base.take(bi)).push(next.edge.key()));
        reference_edge_keys(base.take(bi)).lemma_push_to_set_commute(next.edge.key());
    }
    if tc {
        assert(reference_edge_keys(candidate.take(ci + 1)) =~= reference_edge_keys(candidate.take(ci)).push(next.edge.key()));
        reference_edge_keys(candidate.take(ci)).lemma_push_to_set_commute(next.edge.key());
    }
    out.lemma_push_map_commute(|edge: UnionEdge| edge.edge.key(), next);
    assert(union_edge_keys(out.push(next)) =~= union_edge_keys(out).push(next.edge.key()));
    union_edge_keys(out).lemma_push_to_set_commute(next.edge.key());
    assert(union_edge_keys(out.push(next)).to_set()
        =~= union_edge_keys(out).to_set().insert(next.edge.key()));
    assert((reference_edge_keys(base.take(bi + (if tb { 1int } else { 0int }))).to_set()
        + reference_edge_keys(candidate.take(ci + (if tc { 1int } else { 0int }))).to_set())
        =~= union_edge_keys(out).to_set().insert(next.edge.key()));
}

proof fn merge_origins_step(out: Seq<UnionEdge>, base: Seq<&Edge>, candidate: Seq<&Edge>,
    bi: int, ci: int, next: UnionEdge, tb: bool, tc: bool)
    requires 0 <= bi <= base.len(), 0 <= ci <= candidate.len(),
        ordered_edges(base), ordered_edges(candidate),
        union_origins(out, base, candidate), union_prefix(out, base, candidate, bi, ci),
        before_rest(out, base, bi), before_rest(out, candidate, ci),
        head_choice(base, candidate, bi, ci, &next.edge, next.origin, tb, tc),
    ensures union_origins(out.push(next), base, candidate),
{
    reveal(union_origins); reveal(union_prefix); reveal(before_rest);
    assert forall|i: int| 0 <= i < out.len()
        implies (#[trigger] out[i]).edge.order_spec(&next.edge) == Ordering::Less by {
        if tb { assert(out[i].edge.order_spec(base[bi]) == Ordering::Less); }
        else { assert(out[i].edge.order_spec(candidate[ci]) == Ordering::Less); }
    }
    if !tb {
        selected_before_tail(base, bi, &next.edge, false);
        excluded_key(base, bi, out, &next.edge);
    } else {
        assert(reference_edge_keys(base)[bi] == next.edge.key());
        assert(reference_edge_keys(base).to_set().contains(next.edge.key()));
    }
    if !tc {
        selected_before_tail(candidate, ci, &next.edge, false);
        excluded_key(candidate, ci, out, &next.edge);
    } else {
        assert(reference_edge_keys(candidate)[ci] == next.edge.key());
        assert(reference_edge_keys(candidate).to_set().contains(next.edge.key()));
    }
}

proof fn merge_step(out: Seq<UnionEdge>, base: Seq<&Edge>, candidate: Seq<&Edge>,
    bi: int, ci: int, next: UnionEdge, tb: bool, tc: bool)
    requires 0 <= bi <= base.len(), 0 <= ci <= candidate.len(),
        ordered_edges(base), ordered_edges(candidate), merge_state(out, base, candidate, bi, ci),
        head_choice(base, candidate, bi, ci, &next.edge, next.origin, tb, tc),
    ensures merge_state(out.push(next), base, candidate,
        bi + (if tb { 1int } else { 0int }), ci + (if tc { 1int } else { 0int })),
{
    reveal(merge_state);
    merge_order_step(out, base, candidate, bi, ci, next, tb, tc);
    merge_members_step(out, base, candidate, bi, ci, next, tb, tc);
    merge_origins_step(out, base, candidate, bi, ci, next, tb, tc);
}

/// Merge existing ordered inventories and retain exact endpoint provenance.
fn merge_union(base: &[&Edge], candidate: &[&Edge]) -> (out: Vec<UnionEdge>)
    requires ordered_edges(base@), ordered_edges(candidate@),
    ensures ordered_union(out@), union_origins(out@, base@, candidate@),
        union_edge_keys(out@).to_set()
            == reference_edge_keys(base@).to_set() + reference_edge_keys(candidate@).to_set(),
{
    let mut out: Vec<UnionEdge> = Vec::new();
    assert(merge_state(out@, base@, candidate@, 0, 0)) by {
        reveal(merge_state); reveal(ordered_union); reveal(union_origins);
        reveal(union_prefix); reveal(before_rest);
        assert(union_edge_keys(out@) =~= Seq::<EdgeKey>::empty());
        assert(reference_edge_keys(base@.take(0)) =~= Seq::<EdgeKey>::empty());
        assert(reference_edge_keys(candidate@.take(0)) =~= Seq::<EdgeKey>::empty());
        assert(Seq::<EdgeKey>::empty().to_set() =~= Set::<EdgeKey>::empty());
        assert((Set::<EdgeKey>::empty() + Set::<EdgeKey>::empty()) =~= Set::<EdgeKey>::empty());
    }
    let mut bi: usize = 0;
    let mut ci: usize = 0;
    while bi < base.len() || ci < candidate.len()
        invariant bi <= base.len(), ci <= candidate.len(),
            ordered_edges(base@), ordered_edges(candidate@),
            merge_state(out@, base@, candidate@, bi as int, ci as int),
        decreases (base.len() - bi) + (candidate.len() - ci),
    {
        let (edge, origin, tb, tc) = union_head(base, candidate, bi, ci);
        let next = UnionEdge { edge: edge.clone(), origin };
        proof { merge_step(out@, base@, candidate@, bi as int, ci as int, next, tb, tc); }
        if tb { bi += 1; }
        if tc { ci += 1; }
        out.push(next);
    }
    proof {
        reveal(merge_state); reveal(union_prefix);
        assert(base@.take(bi as int) =~= base@);
        assert(candidate@.take(ci as int) =~= candidate@);
    }
    out
}

proof fn ordered_union_count(out: Seq<UnionEdge>)
    requires ordered_union(out),
    ensures union_edge_keys(out).no_duplicates(),
        out.len() == union_edge_keys(out).to_set().len(),
{
    reveal(ordered_union);
    let keys = union_edge_keys(out);
    assert forall|i: int, j: int| 0 <= i < keys.len() && 0 <= j < keys.len() && i != j
        implies (#[trigger] keys[i]) != (#[trigger] keys[j]) by {
        if i < j { less_keys_differ(&out[i].edge, &out[j].edge); }
        else { less_keys_differ(&out[j].edge, &out[i].edge); }
    }
    keys.unique_seq_to_set();
}

/// Exact typed union: one strictly ordered edge per full key, with both endpoint origins.
pub fn union(base: &BTreeSet<Edge>, candidate: &BTreeSet<Edge>) -> (out: Vec<UnionEdge>)
    ensures ordered_union(out@),
        union_edge_keys(out@).no_duplicates(),
        out.len() == (base@.map(|edge: Edge| edge.key()) + candidate@.map(|edge: Edge| edge.key())).len(),
        union_edge_keys(out@).to_set() == base@.map(|edge: Edge| edge.key())
            + candidate@.map(|edge: Edge| edge.key()),
        forall|i: int| 0 <= i < out.len() ==> exact_origin((#[trigger] out@[i]).origin,
            base@.map(|edge: Edge| edge.key()).contains(out@[i].edge.key()),
            candidate@.map(|edge: Edge| edge.key()).contains(out@[i].edge.key())),
{
    let before = owned_edge_inventory(base);
    let after = owned_edge_inventory(candidate);
    let out = merge_union(&before, &after);
    proof { reveal(union_origins); ordered_union_count(out@); }
    out
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

/// Complete typed keys belonging to one source, independent of input order/origin.
pub open spec fn source_edge_keys(edges: Seq<UnionEdge>, source: Seq<u8>) -> Set<EdgeKey> {
    union_edge_keys(edges).to_set().filter(|key: EdgeKey| key.0 == source)
}

pub open spec fn source_names_present(names: Seq<&str>, edges: Seq<UnionEdge>) -> bool {
    forall|i: int| 0 <= i < edges.len()
        ==> name_members(names).contains((#[trigger] edges[i]).edge.from_bytes())
}

#[verifier::opaque]
pub open spec fn source_row_matches(row: Seq<&Edge>, edges: Seq<UnionEdge>, source: Seq<u8>) -> bool {
    ordered_edges(row)
        && reference_edge_keys(row).to_set() == source_edge_keys(edges, source)
        && reference_edge_keys(row).no_duplicates()
        && row.len() == source_edge_keys(edges, source).len()
}

spec fn bucket_keys(buckets: Map<usize, BTreeSet<&Edge>>, at: usize) -> Set<EdgeKey> {
    if buckets.contains_key(at) { buckets[at]@.map(|edge: &Edge| edge.key()) }
    else { Set::empty() }
}

proof fn source_keys_push(edges: Seq<UnionEdge>, next: UnionEdge, source: Seq<u8>)
    ensures source_edge_keys(edges.push(next), source)
        == if next.edge.from_bytes() == source {
            source_edge_keys(edges, source).insert(next.edge.key())
        } else { source_edge_keys(edges, source) },
{
    assert(union_edge_keys(edges.push(next)) =~= union_edge_keys(edges).push(next.edge.key()));
    union_edge_keys(edges).lemma_push_to_set_commute(next.edge.key());
    assert(union_edge_keys(edges).push(next.edge.key()).to_set()
        =~= union_edge_keys(edges).to_set().insert(next.edge.key()));
    assert(source_edge_keys(edges.push(next), source) =~= if next.edge.from_bytes() == source {
        source_edge_keys(edges, source).insert(next.edge.key())
    } else { source_edge_keys(edges, source) });
}

proof fn ordered_reference_count(row: Seq<&Edge>)
    requires ordered_edges(row),
    ensures reference_edge_keys(row).no_duplicates(),
        reference_edge_keys(row).to_set().len() == row.len(),
{
    reveal(ordered_edges);
    assert forall|i: int, j: int| 0 <= i < j < row.len()
        implies reference_edge_keys(row)[i] != reference_edge_keys(row)[j] by {
        less_keys_differ(row[i], row[j]);
    }
    reference_edge_keys(row).unique_seq_to_set();
}

/// Group arbitrary supplied edges into exact sorted rows in the supplied node order.
pub fn source_rows<'a>(names: &[&str], edges: &'a [UnionEdge]) -> (out: Option<Vec<Vec<&'a Edge>>>)
    requires ordered_names(names@),
    ensures out.is_some() == source_names_present(names@, edges@),
        match out {
            Some(rows) => rows.len() == names.len()
                && forall|i: int| 0 <= i < rows.len()
                    ==> source_row_matches(#[trigger] rows[i]@, edges@, names@[i].spec_bytes()),
            None => true,
        },
{
    proof {
        edge_obeys_cmp();
        broadcast use vstd::laws_cmp::group_laws_cmp;
        assert(vstd::laws_cmp::obeys_cmp::<usize>());
        broadcast use vstd::laws_cmp::lemma_ref_obeys_cmp_spec;
        assert(vstd::laws_cmp::obeys_cmp::<&Edge>());
    }
    broadcast use vstd::std_specs::btree::axiom_maps_deref_key_to_value;
    broadcast use vstd::std_specs::btree::axiom_contains_deref_key;
    broadcast use vstd::std_specs::btree::axiom_deref_key_removed;
    let mut buckets: BTreeMap<usize, BTreeSet<&Edge>> = BTreeMap::new();
    let mut end: usize = 0;
    while end < edges.len()
        invariant end <= edges.len(), ordered_names(names@),
            vstd::laws_cmp::obeys_cmp::<usize>(), vstd::laws_cmp::obeys_cmp::<&Edge>(),
            source_names_present(names@, edges@.take(end as int)),
            forall|i: usize| i < names.len() ==> bucket_keys(buckets@, i)
                == source_edge_keys(edges@.take(end as int), #[trigger] names@[i as int].spec_bytes()),
        decreases edges.len() - end,
    {
        let edge = &edges[end].edge;
        let at = match name_index(names, edge.from.as_str()) {
            Some(at) => at,
            None => {
                proof {
                    assert(!name_members(names@).contains(edge.from_bytes()));
                    assert(!source_names_present(names@, edges@));
                }
                return None;
            },
        };
        let ghost old_buckets = buckets@;
        let mut row = match buckets.remove(&at) {
            Some(row) => row,
            None => BTreeSet::new(),
        };
        assert(row@.map(|edge: &Edge| edge.key()) == bucket_keys(old_buckets, at));
        let ghost old_row = row@;
        row.insert(edge);
        proof {
            old_row.lemma_set_map_insert_commute(edge, |edge: &Edge| edge.key());
            assert(row@ == old_row.insert(edge));
            assert(row@.map(|edge: &Edge| edge.key())
                == bucket_keys(old_buckets, at).insert(edge.key()));
        }
        buckets.insert(at, row);
        proof {
            assert(edges@.take(end as int + 1) =~= edges@.take(end as int).push(edges@[end as int]));
            assert(name_members(names@).contains(edge.from_bytes()));
            assert forall|i: usize| i < names.len() implies bucket_keys(buckets@, i)
                == source_edge_keys(edges@.take(end as int + 1), #[trigger] names@[i as int].spec_bytes()) by {
                source_keys_push(edges@.take(end as int), edges@[end as int], names@[i as int].spec_bytes());
                if i != at {
                    if i < at {
                        byte_lt_strict(names@[i as int].spec_bytes(), names@[at as int].spec_bytes());
                    } else {
                        byte_lt_strict(names@[at as int].spec_bytes(), names@[i as int].spec_bytes());
                    }
                    assert(names@[i as int].spec_bytes() != edge.from_bytes());
                }
            }
        }
        end += 1;
    }
    assert(edges@.take(end as int) =~= edges@);
    let mut rows: Vec<Vec<&Edge>> = Vec::with_capacity(names.len());
    let mut at: usize = 0;
    while at < names.len()
        invariant at <= names.len(), rows.len() == at,
            vstd::laws_cmp::obeys_cmp::<usize>(),
            source_names_present(names@, edges@),
            forall|i: usize| i < names.len() ==> bucket_keys(buckets@, i)
                == source_edge_keys(edges@, #[trigger] names@[i as int].spec_bytes()),
            forall|i: int| 0 <= i < rows.len()
                ==> source_row_matches(#[trigger] rows@[i]@, edges@, names@[i].spec_bytes()),
        decreases names.len() - at,
    {
        let row = match buckets.get(&at) {
            Some(bucket) => {
                let row = edge_inventory(bucket);
                proof {
                    reveal(ordered_edges);
                    assert(ordered_edges(row@));
                    row@.lemma_to_set_map_commutes(|edge: &Edge| edge.key());
                    assert(reference_edge_keys(row@).to_set() == bucket_keys(buckets@, at));
                }
                row
            },
            None => {
                let row: Vec<&Edge> = Vec::new();
                proof {
                    reveal(ordered_edges);
                    assert(reference_edge_keys(row@).to_set() =~= Set::<EdgeKey>::empty());
                    assert(reference_edge_keys(row@).to_set() == bucket_keys(buckets@, at));
                }
                row
            },
        };
        proof {
            ordered_reference_count(row@);
            reveal(source_row_matches);
            assert(source_row_matches(row@, edges@, names@[at as int].spec_bytes()));
        }
        rows.push(row);
        at += 1;
    }
    Some(rows)
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

pub open spec fn edge_names_present(
    names: Seq<&str>, rows: Seq<Vec<&Edge>>, roots: Seq<&str>,
) -> bool {
    rows.len() == names.len()
        && (forall|row: int, slot: int| 0 <= row < rows.len() && 0 <= slot < rows[row].len()
            ==> name_members(names).contains(#[trigger] rows[row]@[slot].to_bytes()))
        && row_names_present(names, roots)
}

pub open spec fn edge_indices_match(
    names: Seq<&str>, edges: Seq<&Edge>, indices: Seq<usize>,
) -> bool {
    indices.len() == edges.len()
        && forall|slot: int| 0 <= slot < edges.len() ==> {
            let index = #[trigger] indices[slot];
            index < names.len() && names[index as int].spec_bytes() == edges[slot].to_bytes()
        }
}

fn target_name_rows<'a>(rows: &[Vec<&'a Edge>]) -> (out: Vec<Vec<&'a str>>)
    ensures out.len() == rows.len(),
        forall|row: int| 0 <= row < rows.len() ==> {
            let names = #[trigger] out@[row];
            names.len() == rows@[row].len()
                && forall|slot: int| 0 <= slot < names.len()
                    ==> (#[trigger] names@[slot]).spec_bytes() == rows@[row]@[slot].to_bytes()
        },
{
    let mut out: Vec<Vec<&str>> = Vec::with_capacity(rows.len());
    let mut row: usize = 0;
    while row < rows.len()
        invariant row <= rows.len(), out.len() == row,
            forall|r: int| 0 <= r < row ==> {
                let names = #[trigger] out@[r];
                names.len() == rows@[r].len()
                    && forall|slot: int| 0 <= slot < names.len()
                        ==> (#[trigger] names@[slot]).spec_bytes() == rows@[r]@[slot].to_bytes()
            },
        decreases rows.len() - row,
    {
        let edges = &rows[row];
        let mut names: Vec<&str> = Vec::with_capacity(edges.len());
        let mut slot: usize = 0;
        while slot < edges.len()
            invariant slot <= edges.len(), names.len() == slot,
                edges@ == rows@[row as int]@,
                forall|s: int| 0 <= s < slot
                    ==> (#[trigger] names@[s]).spec_bytes() == edges@[s].to_bytes(),
            decreases edges.len() - slot,
        {
            names.push(edges[slot].to.as_str());
            slot += 1;
        }
        out.push(names);
        row += 1;
    }
    out
}

/// Map the actual typed edge rows without omitting or reordering any target slot.
pub fn index_edges(names: &[&str], rows: &[Vec<&Edge>], roots: &[&str]) -> (out: Option<NameIndices>)
    requires ordered_names(names@),
    ensures out.is_some() == edge_names_present(names@, rows@, roots@),
        match out {
            Some(mapped) => mapped.rows.len() == rows.len() && valid_graph(mapped.rows@)
                && (forall|row: int| 0 <= row < rows.len()
                    ==> edge_indices_match(names@, rows@[row]@, #[trigger] mapped.rows@[row]@))
                && name_indices_match(names@, roots@, mapped.roots@),
            None => true,
        },
{
    let targets = target_name_rows(rows);
    proof {
        if graph_names_present(names@, targets@, roots@) {
            assert forall|r: int, s: int| 0 <= r < rows.len() && 0 <= s < rows@[r].len()
                implies name_members(names@).contains(#[trigger] rows@[r]@[s].to_bytes()) by {
                assert(row_names_present(names@, targets@[r]@));
                assert(targets@[r]@[s].spec_bytes() == rows@[r]@[s].to_bytes());
            }
        }
        if edge_names_present(names@, rows@, roots@) {
            assert forall|r: int| 0 <= r < targets.len()
                implies row_names_present(names@, #[trigger] targets@[r]@) by {
                assert forall|s: int| 0 <= s < targets@[r].len()
                    implies name_members(names@).contains(#[trigger] targets@[r]@[s].spec_bytes()) by {
                    assert(targets@[r]@[s].spec_bytes() == rows@[r]@[s].to_bytes());
                }
            }
        }
        assert(graph_names_present(names@, targets@, roots@)
            == edge_names_present(names@, rows@, roots@));
    }
    let out = index_names(names, &targets, roots);
    proof {
        if let Some(mapped) = &out {
            assert forall|r: int| 0 <= r < rows.len()
                implies edge_indices_match(names@, rows@[r]@, #[trigger] mapped.rows@[r]@) by {
                assert(name_indices_match(names@, targets@[r]@, mapped.rows@[r]@));
                assert forall|s: int| 0 <= s < rows@[r].len() implies {
                    let index = #[trigger] mapped.rows@[r]@[s];
                    index < names.len() && names@[index as int].spec_bytes() == rows@[r]@[s].to_bytes()
                } by {
                    assert(targets@[r]@[s].spec_bytes() == rows@[r]@[s].to_bytes());
                }
            }
        }
    }
    out
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

/// Every complete endpoint byte value, independent of kind, origin or duplication.
pub open spec fn endpoint_names(edges: Seq<UnionEdge>) -> Set<Seq<u8>> {
    Set::new(|key: Seq<u8>| exists|i: int| 0 <= i < edges.len()
        && ((#[trigger] edges[i]).edge.from_bytes() == key || edges[i].edge.to_bytes() == key))
}

proof fn endpoint_names_push(edges: Seq<UnionEdge>, next: UnionEdge)
    ensures endpoint_names(edges.push(next))
        == endpoint_names(edges).insert(next.edge.from_bytes()).insert(next.edge.to_bytes()),
{
    assert forall|key: Seq<u8>| endpoint_names(edges.push(next)).contains(key)
        == endpoint_names(edges).insert(next.edge.from_bytes()).insert(next.edge.to_bytes()).contains(key) by {
        if endpoint_names(edges.push(next)).contains(key) {
            let i = choose|i: int| 0 <= i < edges.push(next).len()
                && (edges.push(next)[i].edge.from_bytes() == key || edges.push(next)[i].edge.to_bytes() == key);
            if i < edges.len() { assert(endpoint_names(edges).contains(key)); }
        }
        if endpoint_names(edges).contains(key) {
            let i = choose|i: int| 0 <= i < edges.len()
                && (edges[i].edge.from_bytes() == key || edges[i].edge.to_bytes() == key);
            assert(edges.push(next)[i] == edges[i]);
        }
        if key == next.edge.from_bytes() || key == next.edge.to_bytes() {
            assert(edges.push(next)[edges.len() as int] == next);
        }
    }
}

/// Extract both actual typed endpoints and supplied roots, then sort and deduplicate.
pub fn graph_names<'a>(edges: &'a [UnionEdge], roots: &[&'a str]) -> (out: Vec<&'a str>)
    ensures ordered_names(out@),
        name_members(out@) == endpoint_names(edges@) + name_members(roots@),
        (endpoint_names(edges@) + name_members(roots@)).finite(),
        out.len() == (endpoint_names(edges@) + name_members(roots@)).len(),
        source_names_present(out@, edges@),
{
    let mut supplied: Vec<&str> = Vec::new();
    let mut at: usize = 0;
    assert(endpoint_names(edges@.take(0)) =~= Set::<Seq<u8>>::empty());
    assert(name_members(supplied@) =~= Set::<Seq<u8>>::empty());
    while at < edges.len()
        invariant at <= edges.len(),
            name_members(supplied@) == endpoint_names(edges@.take(at as int)),
        decreases edges.len() - at,
    {
        let edge = &edges[at].edge;
        let from = edge.from.as_str();
        let to = edge.to.as_str();
        proof {
            endpoint_names_push(edges@.take(at as int), edges@[at as int]);
            assert(edges@.take(at as int + 1) =~= edges@.take(at as int).push(edges@[at as int]));
            name_members_push(supplied@, from);
        }
        supplied.push(from);
        proof { name_members_push(supplied@, to); }
        supplied.push(to);
        at += 1;
    }
    assert(edges@.take(at as int) =~= edges@);
    let mut at: usize = 0;
    assert(name_members(roots@.take(0)) =~= Set::<Seq<u8>>::empty());
    while at < roots.len()
        invariant at <= roots.len(),
            name_members(supplied@) == endpoint_names(edges@) + name_members(roots@.take(at as int)),
        decreases roots.len() - at,
    {
        proof {
            name_members_step(roots@, at as int);
            name_members_push(supplied@, roots@[at as int]);
            assert((endpoint_names(edges@) + name_members(roots@.take(at as int))).insert(roots@[at as int].spec_bytes())
                =~= endpoint_names(edges@) + name_members(roots@.take(at as int + 1)));
        }
        supplied.push(roots[at]);
        at += 1;
    }
    assert(roots@.take(at as int) =~= roots@);
    let out = name_inventory(&supplied);
    assert forall|i: int| 0 <= i < edges.len() implies
        name_members(out@).contains((#[trigger] edges@[i]).edge.from_bytes()) by {
        assert(endpoint_names(edges@).contains(edges@[i].edge.from_bytes()));
    }
    out
}

pub struct Topology<'a> {
    pub names: Vec<&'a str>,
    pub edge_rows: Vec<Vec<&'a Edge>>,
    pub indices: NameIndices,
}

/// Compose exact name admission, typed source grouping and target/root indexing.
/// Every supplied endpoint/root is admitted, so neither internal lookup can fail.
pub fn topology<'a>(edges: &'a [UnionEdge], roots: &[&'a str]) -> (out: Topology<'a>)
    ensures ordered_names(out.names@),
        name_members(out.names@) == endpoint_names(edges@) + name_members(roots@),
        (endpoint_names(edges@) + name_members(roots@)).finite(),
        out.names.len() == (endpoint_names(edges@) + name_members(roots@)).len(),
        out.edge_rows.len() == out.names.len(),
        forall|row: int| 0 <= row < out.edge_rows.len()
            ==> source_row_matches(#[trigger] out.edge_rows@[row]@, edges@, out.names@[row].spec_bytes()),
        out.indices.rows.len() == out.names.len(),
        valid_graph(out.indices.rows@),
        forall|row: int| 0 <= row < out.edge_rows.len()
            ==> edge_indices_match(out.names@, out.edge_rows@[row]@, #[trigger] out.indices.rows@[row]@),
        name_indices_match(out.names@, roots@, out.indices.roots@),
{
    let names = graph_names(edges, roots);
    let edge_rows = source_rows(&names, edges).unwrap();
    proof {
        assert forall|row: int, slot: int| 0 <= row < edge_rows.len() && 0 <= slot < edge_rows@[row].len()
            implies name_members(names@).contains(#[trigger] edge_rows@[row]@[slot].to_bytes()) by {
            reveal(source_row_matches);
            let key = edge_rows@[row]@[slot].key();
            assert(reference_edge_keys(edge_rows@[row]@)[slot] == key);
            assert(reference_edge_keys(edge_rows@[row]@).to_set().contains(key));
            assert(union_edge_keys(edges@).to_set().contains(key));
            let at = choose|at: int| 0 <= at < edges.len() && union_edge_keys(edges@)[at] == key;
            assert(edges@[at].edge.key() == key);
            assert(endpoint_names(edges@).contains(edge_rows@[row]@[slot].to_bytes()));
        }
        assert forall|slot: int| 0 <= slot < roots.len()
            implies name_members(names@).contains(#[trigger] roots@[slot].spec_bytes()) by {
            assert(name_members(roots@).contains(roots@[slot].spec_bytes()));
        }
        assert(edge_names_present(names@, edge_rows@, roots@));
    }
    let indices = index_edges(&names, &edge_rows, roots).unwrap();
    Topology { names, edge_rows, indices }
}

pub struct Seed {
    pub id: String,
    pub node: String,
    pub reference: String,
    pub reason: String,
}

impl Seed {
    pub open spec fn key(&self) -> (Seq<u8>, Seq<u8>, Seq<u8>, Seq<u8>) {
        (vstd::utf8::encode_utf8(self.id@), vstd::utf8::encode_utf8(self.node@),
            vstd::utf8::encode_utf8(self.reference@), vstd::utf8::encode_utf8(self.reason@))
    }

    pub open spec fn same(&self, other: &Self) -> bool { self.key() == other.key() }

    pub open spec fn less(&self, other: &Self) -> bool {
        let a = self.key();
        let b = other.key();
        byte_lt(a.0, b.0) || a.0 == b.0 && (
            byte_lt(a.1, b.1) || a.1 == b.1 && (
                byte_lt(a.2, b.2) || a.2 == b.2 && byte_lt(a.3, b.3)))
    }

    pub open spec fn order_spec(&self, other: &Self) -> Ordering {
        if self.same(other) { Ordering::Equal }
        else if self.less(other) { Ordering::Less }
        else { Ordering::Greater }
    }

    pub fn compare(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    {
        proof { seed_order_laws(self, other, other); }
        let id = compare_bytes(self.id.as_str().as_bytes(), other.id.as_str().as_bytes());
        if id < 0 { return Ordering::Less; }
        if id > 0 { return Ordering::Greater; }
        let node = compare_bytes(self.node.as_str().as_bytes(), other.node.as_str().as_bytes());
        if node < 0 { return Ordering::Less; }
        if node > 0 { return Ordering::Greater; }
        let reference = compare_bytes(self.reference.as_str().as_bytes(), other.reference.as_str().as_bytes());
        if reference < 0 { return Ordering::Less; }
        if reference > 0 { return Ordering::Greater; }
        let reason = compare_bytes(self.reason.as_str().as_bytes(), other.reason.as_str().as_bytes());
        if reason < 0 { Ordering::Less }
        else if reason > 0 { Ordering::Greater }
        else { Ordering::Equal }
    }
}

proof fn byte_order_facts(a: Seq<u8>, b: Seq<u8>, c: Seq<u8>)
    ensures a == b || byte_lt(a, b) || byte_lt(b, a),
        byte_lt(a, b) ==> a != b && !byte_lt(b, a),
        byte_lt(b, a) ==> b != a,
        byte_lt(a, b) && byte_lt(b, c) ==> byte_lt(a, c),
        byte_lt(a, c) ==> a != c,
{
    byte_trichotomy(a, b);
    byte_lt_asymmetric(a, b);
    if byte_lt(a, b) { byte_lt_strict(a, b); }
    if byte_lt(b, a) { byte_lt_strict(b, a); }
    if byte_lt(a, b) && byte_lt(b, c) { byte_lt_transitive(a, b, c); }
    if byte_lt(a, c) { byte_lt_strict(a, c); }
}

pub proof fn seed_order_laws(a: &Seed, b: &Seed, c: &Seed)
    ensures (a.order_spec(b) == Ordering::Equal) == a.same(b),
        (a.order_spec(b) == Ordering::Less) == (b.order_spec(a) == Ordering::Greater),
        a.order_spec(b) == Ordering::Less && b.order_spec(c) == Ordering::Less
            ==> a.order_spec(c) == Ordering::Less,
{
    byte_order_facts(a.key().0, b.key().0, c.key().0);
    byte_order_facts(a.key().1, b.key().1, c.key().1);
    byte_order_facts(a.key().2, b.key().2, c.key().2);
    byte_order_facts(a.key().3, b.key().3, c.key().3);
}

impl Clone for Seed {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    {
        Self { id: self.id.clone(), node: self.node.clone(),
            reference: self.reference.clone(), reason: self.reason.clone() }
    }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for Seed {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { self.same(other) }
}

impl PartialEq for Seed {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == self.same(other),
    {
        match self.compare(other) { Ordering::Equal => true, _ => false }
    }
}

impl Eq for Seed {}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::OrdSpecImpl for Seed {
    open spec fn obeys_cmp_spec() -> bool { true }
    open spec fn cmp_spec(&self, other: &Self) -> Ordering { self.order_spec(other) }
}

impl Ord for Seed {
    fn cmp(&self, other: &Self) -> (order: Ordering)
        ensures order == self.order_spec(other),
    { self.compare(other) }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialOrdSpecImpl for Seed {
    open spec fn obeys_partial_cmp_spec() -> bool { true }
    open spec fn partial_cmp_spec(&self, other: &Self) -> Option<Ordering> {
        Some(self.order_spec(other))
    }
}

impl PartialOrd for Seed {
    fn partial_cmp(&self, other: &Self) -> (order: Option<Ordering>)
        ensures order == Some(self.order_spec(other)),
    { Some(self.compare(other)) }
}

pub proof fn seed_obeys_cmp()
    ensures vstd::laws_cmp::obeys_cmp::<Seed>(),
{
    use vstd::std_specs::cmp::{PartialEqSpec, PartialOrdSpec, OrdSpec};
    reveal(vstd::laws_eq::obeys_eq_spec_properties);
    reveal(vstd::laws_cmp::obeys_partial_cmp_spec_properties);
    reveal(vstd::laws_cmp::obeys_cmp_partial_ord);
    reveal(vstd::laws_cmp::obeys_cmp_ord);
    assert forall|a: Seed, b: Seed|
        a.partial_cmp_spec(&b) == Some(Ordering::Less)
            <==> b.partial_cmp_spec(&a) == Some(Ordering::Greater) by {
        seed_order_laws(&a, &b, &b);
    }
    assert forall|a: Seed, b: Seed, c: Seed|
        a.partial_cmp_spec(&b) == Some(Ordering::Less)
            && b.partial_cmp_spec(&c) == Some(Ordering::Less)
            implies a.partial_cmp_spec(&c) == Some(Ordering::Less) by {
        seed_order_laws(&a, &b, &c);
    }
    assert forall|a: Seed, b: Seed, c: Seed|
        a.partial_cmp_spec(&b) == Some(Ordering::Greater)
            && b.partial_cmp_spec(&c) == Some(Ordering::Greater)
            implies a.partial_cmp_spec(&c) == Some(Ordering::Greater) by {
        seed_order_laws(&a, &b, &c);
        seed_order_laws(&b, &c, &a);
        seed_order_laws(&c, &b, &a);
    }
}

/// Preserve the node field and position of every actual Seed, including duplicates.
pub open spec fn seed_node_names(seeds: Seq<Seed>) -> Set<Seq<u8>> {
    Set::new(|key: Seq<u8>| exists|at: int| 0 <= at < seeds.len()
        && (#[trigger] seeds[at]).key().1 == key)
}

pub fn seed_roots<'a>(seeds: &'a [Seed]) -> (out: Vec<&'a str>)
    ensures out.len() == seeds.len(),
        forall|at: int| 0 <= at < out.len()
            ==> (#[trigger] out@[at]).spec_bytes() == seeds@[at].key().1,
        name_members(out@) == seed_node_names(seeds@),
{
    let mut out: Vec<&str> = Vec::with_capacity(seeds.len());
    let mut at: usize = 0;
    while at < seeds.len()
        invariant at <= seeds.len(), out.len() == at,
            forall|i: int| 0 <= i < at
                ==> (#[trigger] out@[i]).spec_bytes() == seeds@[i].key().1,
        decreases seeds.len() - at,
    {
        out.push(seeds[at].node.as_str());
        at += 1;
    }
    assert forall|key: Seq<u8>| name_members(out@).contains(key)
        == seed_node_names(seeds@).contains(key) by {
        if name_members(out@).contains(key) {
            let i = choose|i: int| 0 <= i < out.len() && out@[i].spec_bytes() == key;
            assert(seeds@[i].key().1 == key);
        }
        if seed_node_names(seeds@).contains(key) {
            let i = choose|i: int| 0 <= i < seeds.len() && seeds@[i].key().1 == key;
            assert(out@[i].spec_bytes() == key);
        }
    }
    out
}

/// Construct the actual graph directly from Seeds and preserve every root position.
pub fn seed_topology<'a>(edges: &'a [UnionEdge], seeds: &'a [Seed]) -> (out: Topology<'a>)
    ensures ordered_names(out.names@),
        name_members(out.names@) == endpoint_names(edges@) + seed_node_names(seeds@),
        (endpoint_names(edges@) + seed_node_names(seeds@)).finite(),
        out.names.len() == (endpoint_names(edges@) + seed_node_names(seeds@)).len(),
        out.edge_rows.len() == out.names.len(),
        forall|row: int| 0 <= row < out.edge_rows.len()
            ==> source_row_matches(#[trigger] out.edge_rows@[row]@, edges@, out.names@[row].spec_bytes()),
        out.indices.rows.len() == out.names.len(),
        valid_graph(out.indices.rows@),
        forall|row: int| 0 <= row < out.edge_rows.len()
            ==> edge_indices_match(out.names@, out.edge_rows@[row]@, #[trigger] out.indices.rows@[row]@),
        out.indices.roots.len() == seeds.len(),
        forall|at: int| 0 <= at < seeds.len() ==> {
            let index = #[trigger] out.indices.roots@[at];
            index < out.names.len() && out.names@[index as int].spec_bytes() == seeds@[at].key().1
        },
{
    let roots = seed_roots(seeds);
    let out = topology(edges, &roots);
    assert forall|at: int| 0 <= at < seeds.len() implies {
        let index = #[trigger] out.indices.roots@[at];
        index < out.names.len() && out.names@[index as int].spec_bytes() == seeds@[at].key().1
    } by {
        assert(roots@[at].spec_bytes() == seeds@[at].key().1);
    }
    out
}

}
