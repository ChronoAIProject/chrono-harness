//! Compiler dependency content is compared against caller-supplied source metadata.
use super::{append_bytes, equal_bytes, parse_dependency_paths};
#[cfg(verus_keep_ghost)]
use super::{
    dependency_encoding, dependency_escape, dependency_scan, dependency_values, dependency_word,
};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

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

/// Recursion is bounded by the expected u64's decimal digits. Arbitrary input
/// number text is never parsed, so it cannot overflow an integer parser.
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

/// Read only the first compiler target's dependency list. This does not admit
/// the remaining rules or checksums; matches_dependency_content does that.
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
    if let Some(paths) = &parsed {
        assert forall|i: int| 0 <= i < paths.len() implies dependency_word((#[trigger] paths[i])@) by {
            assert(paths@[i]@ == dependency_values(paths@)[i]);
        }
    }
    parsed
}

pub struct SourceBinding {
    pub path: Vec<u8>,
    pub sha256: Vec<u8>,
    pub length: u64,
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

/// Exact ordered content admission against expected source metadata. The caller
/// still must bind every expected path, digest and length to fixed source bytes.
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
