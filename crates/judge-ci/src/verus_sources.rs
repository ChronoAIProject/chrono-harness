//! Compiler dependency content is compared against caller-supplied source metadata.
use super::{append_bytes, equal_bytes, parse_dependency_paths};
#[cfg(verus_keep_ghost)]
use super::{
    dependency_byte, dependency_encoding, dependency_escape, dependency_scan, dependency_values,
    dependency_word,
};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

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

/// Lexical POSIX normalization only. This does not resolve filesystem aliases
/// or authorize symlinks; source-byte and path admission remain the caller's duty.
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

pub open spec fn sha256_text(bytes: Seq<u8>) -> bool {
    bytes.len() == 64 && forall|i: int| 0 <= i < bytes.len() ==> super::oid_byte(#[trigger] bytes[i])
}

/// SHA-256 text may be all zeroes; unlike an object ID it has no nonzero rule.
pub fn valid_sha256_text(bytes: &[u8]) -> (valid: bool)
    ensures valid <==> sha256_text(bytes@),
{
    if bytes.len() != 64 { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant 0 <= i <= bytes.len(), bytes.len() == 64,
            forall|j: int| 0 <= j < i ==> super::oid_byte(#[trigger] bytes[j]),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if !(48 <= byte && byte <= 57 || 97 <= byte && byte <= 102) { return false; }
        i += 1;
    }
    true
}

pub open spec fn artifact_admitted(
    actual_digest: Seq<u8>, expected_digest: Seq<u8>,
    actual_length: u64, expected_length: u64, limit: u64,
) -> bool {
    sha256_text(expected_digest) && actual_digest == expected_digest
        && 0 < actual_length <= limit && actual_length == expected_length
}

/// Hash computation belongs to the explicitly trusted library interface. The
/// repository's admission of its digest, length and remaining budget is proved.
pub fn admit_artifact(
    actual_digest: &[u8], expected_digest: &[u8],
    actual_length: u64, expected_length: u64, limit: u64,
) -> (accepted: bool)
    ensures accepted <==> artifact_admitted(
        actual_digest@, expected_digest@, actual_length, expected_length, limit),
{
    valid_sha256_text(expected_digest)
        && equal_bytes(actual_digest, expected_digest)
        && actual_length > 0 && actual_length <= limit
        && actual_length == expected_length
}

pub struct ObservationIdentity<'a> {
    pub candidate: &'a [u8],
    pub root: &'a [u8],
    pub dependency: &'a [u8],
    pub library: &'a [u8],
}

pub open spec fn observation_identity_matches(
    observed: &ObservationIdentity, candidate: Seq<u8>, root: Seq<u8>, output: Seq<u8>,
) -> bool {
    observed.candidate@ == candidate && observed.root@ == root
        && observed.library@ == output && observed.dependency@ == output + seq![46u8, 100u8]
}

/// Artifact labels and source identities must bind the same registered invocation.
pub fn matches_observation_identity(
    observed: &ObservationIdentity, candidate: &[u8], root: &[u8], output: &[u8],
) -> (matched: bool)
    ensures matched <==> observation_identity_matches(observed, candidate@, root@, output@),
{
    let mut dependency = Vec::new();
    append_bytes(&mut dependency, output);
    append_bytes(&mut dependency, &[46u8, 100u8]);
    equal_bytes(observed.candidate, candidate) && equal_bytes(observed.root, root)
        && equal_bytes(observed.library, output) && equal_bytes(observed.dependency, dependency.as_slice())
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

pub open spec fn source_pair_matches(source: &SourceBinding, candidate: &SourceBinding) -> bool {
    relative_components(source.path@).is_some()
        && candidate.path@ == joined_components(relative_components(source.path@).unwrap())
        && sha256_text(candidate.sha256@)
        && source.sha256@ == candidate.sha256@
        && source.length == candidate.length
}

pub fn matches_source_pair(source: &SourceBinding, candidate: &SourceBinding) -> (matched: bool)
    ensures matched <==> source_pair_matches(source, candidate),
{
    let Some(path) = normalize_relative_path(source.path.as_slice()) else { return false; };
    equal_bytes(path.as_slice(), candidate.path.as_slice())
        && valid_sha256_text(candidate.sha256.as_slice())
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

pub open spec fn source_metadata_admitted(
    sources: Seq<SourceBinding>, candidate: Seq<SourceBinding>, root: Seq<u8>, claims: Seq<Vec<u8>>,
) -> bool {
    sources.len() > 0 && sources.len() == candidate.len()
        && (forall|i: int| 0 <= i < sources.len() ==>
            source_pair_matches(#[trigger] &sources[i], &candidate[i]))
        && (forall|i: int| 0 <= i < candidate.len() ==>
            !listed_source_path(candidate, (#[trigger] candidate[i]).path@, i))
        && listed_source_path(candidate, root, candidate.len() as int)
        && (forall|i: int| 0 <= i < claims.len() ==>
            listed_source_path(candidate, (#[trigger] claims[i])@, candidate.len() as int))
}

/// Each compiler path must match one distinct candidate entry, including its
/// digest and byte length. The exact root and all explicit claims must be present.
/// The caller owns obtaining these candidate entries from the fixed registry/Git.
pub fn admit_source_metadata(
    sources: &Vec<SourceBinding>, candidate: &Vec<SourceBinding>, root: &[u8], claims: &Vec<Vec<u8>>,
) -> (accepted: bool)
    ensures accepted <==> source_metadata_admitted(sources@, candidate@, root@, claims@),
{
    if sources.len() == 0 || sources.len() != candidate.len() { return false; }
    let mut i = 0;
    while i < sources.len()
        invariant 0 <= i <= sources.len(), 0 < sources.len() == candidate.len(),
            forall|j: int| 0 <= j < i ==>
                source_pair_matches(#[trigger] &sources[j], &candidate[j]),
            forall|j: int| 0 <= j < i ==>
                !listed_source_path(candidate@, (#[trigger] candidate[j]).path@, j),
        decreases sources.len() - i,
    {
        if !matches_source_pair(&sources[i], &candidate[i]) { return false; }
        if contains_source_path(candidate, candidate[i].path.as_slice(), i) { return false; }
        i += 1;
    }
    if !contains_source_path(candidate, root, candidate.len()) { return false; }
    let mut j = 0;
    while j < claims.len()
        invariant 0 <= j <= claims.len(), 0 < sources.len() == candidate.len(),
            forall|i: int| 0 <= i < sources.len() ==>
                source_pair_matches(#[trigger] &sources[i], &candidate[i]),
            forall|i: int| 0 <= i < candidate.len() ==>
                !listed_source_path(candidate@, (#[trigger] candidate[i]).path@, i),
            listed_source_path(candidate@, root@, candidate.len() as int),
            forall|i: int| 0 <= i < j ==>
                listed_source_path(candidate@, (#[trigger] claims[i])@, candidate.len() as int),
        decreases claims.len() - j,
    {
        if !contains_source_path(candidate, claims[j].as_slice(), candidate.len()) { return false; }
        j += 1;
    }
    true
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
