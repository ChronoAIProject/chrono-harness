#![allow(unexpected_cfgs)]
//! 实际身份判断及其精确 Verus 合同。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

/// 一行在 LF 之前结束，或在完整输入末尾结束；CR 和 NUL 都保留。
pub open spec fn first_header_line(raw: Seq<u8>, end: int) -> bool {
    0 <= end <= raw.len() && (end == raw.len() || raw[end] == 10)
        && forall|i: int| 0 <= i < end ==> raw[i] != 10
}

pub open spec fn header_line_end(raw: Seq<u8>) -> int {
    choose|end: int| first_header_line(raw, end)
}

pub proof fn header_line_end_is(raw: Seq<u8>, end: int)
    requires first_header_line(raw, end),
    ensures header_line_end(raw) == end,
{
    assert(exists|stop: int| first_header_line(raw, stop));
    let selected = header_line_end(raw);
    if selected < end { assert(raw[selected] == 10 && raw[selected] != 10); }
    if end < selected { assert(raw[end] == 10 && raw[end] != 10); }
}

pub open spec fn parent_header(line: Seq<u8>) -> bool {
    line.len() >= 7 && line[0] == 112 && line[1] == 97 && line[2] == 114
        && line[3] == 101 && line[4] == 110 && line[5] == 116 && line[6] == 32
}

pub open spec fn commit_parent_lines(raw: Seq<u8>) -> Seq<Seq<u8>>
    decreases raw.len(),
{
    let end = header_line_end(raw);
    if !(0 < end <= raw.len()) { seq![] }
    else {
        let line = raw.take(end);
        let head = if parent_header(line) { seq![line.skip(7)] } else { seq![] };
        head + if end < raw.len() { commit_parent_lines(raw.skip(end + 1)) } else { seq![] }
    }
}

pub open spec fn parent_record_views(records: Seq<&[u8]>) -> Seq<Seq<u8>> {
    records.map(|i: int, bytes: &[u8]| bytes@)
}

fn is_parent_header(line: &[u8]) -> (selected: bool)
    ensures selected == parent_header(line@),
{
    line.len() >= 7 && line[0] == 112 && line[1] == 97 && line[2] == 114
        && line[3] == 101 && line[4] == 110 && line[5] == 116 && line[6] == 32
}

/// 完整借用第一段 commit 头中的原 parent 后缀；验证与错误分类仍归实际使用方。
pub fn commit_parent_records(raw: &[u8]) -> (out: Vec<&[u8]>)
    ensures parent_record_views(out@) == commit_parent_lines(raw@),
        out.len() == commit_parent_lines(raw@).len(),
        forall|i: int| 0 <= i < out.len()
            ==> (#[trigger] out@[i])@ == commit_parent_lines(raw@)[i],
{
    let mut remaining = raw;
    let mut out: Vec<&[u8]> = Vec::new();
    loop
        invariant parent_record_views(out@) + commit_parent_lines(remaining@)
            == commit_parent_lines(raw@),
        decreases remaining.len(),
    {
        let mut end = 0usize;
        while end < remaining.len() && remaining[end] != 10
            invariant end <= remaining.len(),
                forall|i: int| 0 <= i < end ==> remaining@[i] != 10,
            decreases remaining.len() - end,
        { end += 1; }
        proof {
            header_line_end_is(remaining@, end as int);
            reveal_with_fuel(commit_parent_lines, 1);
        }
        if end == 0 { return out; }
        let (line, rest) = remaining.split_at(end);
        if is_parent_header(line) {
            let (_, oid) = line.split_at(7);
            let ghost previous = parent_record_views(out@);
            out.push(oid);
            proof { assert(parent_record_views(out@) =~= previous + seq![oid@]); }
        }
        if rest.len() == 0 { return out; }
        let (_, tail) = rest.split_at(1);
        proof { assert(tail@ =~= remaining@.skip(end as int + 1)); }
        remaining = tail;
    }
}

pub open spec fn byte_prefix(a: Seq<u8>, b: Seq<u8>, n: int) -> bool {
    0 <= n <= a.len() && n <= b.len()
        && forall|i: int| 0 <= i < n ==> a[i] == b[i]
}

/// 按完整字节序列定义严格词典序；前缀较短者在前。
pub open spec fn byte_lt(a: Seq<u8>, b: Seq<u8>) -> bool {
    exists|i: int| 0 <= i <= a.len() && i < b.len() && byte_prefix(a, b, i)
        && (i == a.len() || a[i] < b[i])
}

pub proof fn byte_lt_strict(a: Seq<u8>, b: Seq<u8>)
    requires byte_lt(a, b),
    ensures a != b,
{
    let i = choose|i: int| 0 <= i <= a.len() && i < b.len() && byte_prefix(a, b, i)
        && (i == a.len() || a[i] < b[i]);
}

pub proof fn byte_lt_transitive(a: Seq<u8>, b: Seq<u8>, c: Seq<u8>)
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

pub proof fn byte_lt_asymmetric(a: Seq<u8>, b: Seq<u8>)
    ensures byte_lt(a, b) ==> !byte_lt(b, a),
{
    if byte_lt(a, b) && byte_lt(b, a) {
        byte_lt_transitive(a, b, a);
        byte_lt_strict(a, a);
    }
}

pub proof fn byte_trichotomy(a: Seq<u8>, b: Seq<u8>)
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
        assert(x.len() == a.len() - 1);
        assert(y.len() == b.len() - 1);
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

/// -1、0、1 分别表示完整字节序列的小于、相同、大于。
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

/// 只检查请求头；judge 名称沿用非空合同，不套用 CI 单元的 ASCII ID 限制。
pub fn valid_request_header(
    protocol: &str, mode: &str, judge_id: &str,
    expected_protocol: &str, expected_mode: &str,
) -> (valid: bool)
    ensures valid <==> protocol@ == expected_protocol@ && mode@ == expected_mode@
        && judge_id@.len() > 0,
{
    equal_text(protocol, expected_protocol) && equal_text(mode, expected_mode)
        && !judge_id.is_empty()
}

pub fn equal_text(actual: &str, expected: &str) -> (equal: bool)
    ensures equal <==> actual@ == expected@,
{
    let equal = equal_bytes(actual.as_bytes(), expected.as_bytes());
    proof {
        vstd::utf8::encode_utf8_decode_utf8(actual@);
        vstd::utf8::encode_utf8_decode_utf8(expected@);
    }
    equal
}

pub fn equal_bytes(actual: &[u8], expected: &[u8]) -> (equal: bool)
    ensures equal <==> actual@ == expected@,
{
    if actual.len() != expected.len() {
        return false;
    }
    let mut i = 0;
    while i < actual.len()
        invariant
            actual.len() == expected.len(),
            0 <= i <= actual.len(),
            forall|j: int| 0 <= j < i ==> actual[j] == expected[j],
        decreases actual.len() - i,
    {
        if actual[i] != expected[i] {
            return false;
        }
        i += 1;
    }
    assert(actual@ =~= expected@);
    true
}



#[verifier::inline]
pub open spec fn oid_byte(byte: u8) -> bool {
    48 <= byte <= 57 || 97 <= byte <= 102
}

#[verifier::inline]
pub open spec fn digest_format(bytes: Seq<u8>) -> bool {
    bytes.len() == 64
        && forall|i: int| 0 <= i < bytes.len() ==> oid_byte(#[trigger] bytes[i])
}

pub fn valid_digest(bytes: &[u8]) -> (valid: bool)
    ensures valid == digest_format(bytes@),
{
    if bytes.len() != 64 {
        return false;
    }
    let mut i = 0;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            bytes.len() == 64,
            forall|j: int| 0 <= j < i ==> oid_byte(#[trigger] bytes[j]),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if !((48 <= byte && byte <= 57) || (97 <= byte && byte <= 102)) {
            return false;
        }
        i += 1;
    }
    true
}

pub fn valid_digest_text(text: &str) -> (valid: bool)
    ensures valid == digest_format(text.spec_bytes()),
{
    valid_digest(text.as_bytes())
}

#[verifier::inline]
pub open spec fn oid(bytes: Seq<u8>) -> bool {
    (bytes.len() == 40 || bytes.len() == 64)
        && (forall|i: int| 0 <= i < bytes.len() ==> oid_byte(#[trigger] bytes[i]))
        && (exists|i: int| 0 <= i < bytes.len() && bytes[i] != 48)
}

pub fn valid_oid(bytes: &[u8]) -> (valid: bool)
    ensures valid == oid(bytes@),
{
    if bytes.len() != 40 && bytes.len() != 64 {
        return false;
    }
    let mut i = 0;
    let mut nonzero = false;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            bytes.len() == 40 || bytes.len() == 64,
            forall|j: int| 0 <= j < i ==> oid_byte(#[trigger] bytes[j]),
            nonzero == (exists|j: int| 0 <= j < i && bytes[j] != 48),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if !((48 <= byte && byte <= 57) || (97 <= byte && byte <= 102)) {
            return false;
        }
        nonzero = nonzero || byte != 48;
        i += 1;
    }
    nonzero
}

pub fn valid_merge_parents(
    count: usize, base: &[u8], second: &[u8], head: &[u8],
) -> (valid: bool)
    ensures valid <==> count == 2 && oid(base@) && oid(head@) && second@ == head@,
{
    count == 2 && valid_oid(base) && valid_oid(head) && equal_bytes(second, head)
}


#[verifier::inline]
pub open spec fn identifier_byte(byte: u8) -> bool {
    48 <= byte <= 57 || 65 <= byte <= 90 || 97 <= byte <= 122
        || byte == 46 || byte == 95 || byte == 45
}

#[verifier::inline]
pub open spec fn identifier(bytes: Seq<u8>) -> bool {
    bytes.len() > 0
        && forall|i: int| 0 <= i < bytes.len() ==> identifier_byte(#[trigger] bytes[i])
}

pub fn valid_id(bytes: &[u8]) -> (valid: bool)
    ensures valid == identifier(bytes@),
{
    if bytes.len() == 0 {
        return false;
    }
    let mut i = 0;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            bytes.len() > 0,
            forall|j: int| 0 <= j < i ==> identifier_byte(#[trigger] bytes[j]),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if !((48 <= byte && byte <= 57) || (65 <= byte && byte <= 90)
            || (97 <= byte && byte <= 122) || byte == 46 || byte == 95 || byte == 45) {
            return false;
        }
        i += 1;
    }
    true
}


}
