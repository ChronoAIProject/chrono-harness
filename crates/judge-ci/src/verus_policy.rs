//! The executable admission predicate and its Verus specification share this source.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[path = "verus_sources.rs"]
pub mod sources;

verus! {

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

pub open spec fn text_member(values: Seq<&str>, target: Seq<char>) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i])@ == target
}

pub fn contains_text(values: &Vec<&str>, target: &str) -> (found: bool)
    ensures found <==> text_member(values@, target@),
{
    let mut i = 0;
    while i < values.len()
        invariant
            0 <= i <= values.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] values[j])@ != target@,
        decreases values.len() - i,
    {
        if equal_text(values[i], target) {
            return true;
        }
        i += 1;
    }
    false
}

pub open spec fn same_text_membership(actual: Seq<&str>, expected: Seq<&str>) -> bool {
    (forall|i: int| 0 <= i < actual.len() ==>
        text_member(expected, (#[trigger] actual[i])@))
        && (forall|i: int| 0 <= i < expected.len() ==>
            text_member(actual, (#[trigger] expected[i])@))
}

/// Membership is unordered. The shared response protocol owns duplicate rejection.
pub fn matches_text_membership(actual: &Vec<&str>, expected: &Vec<&str>) -> (matches: bool)
    ensures matches <==> same_text_membership(actual@, expected@),
{
    let mut i = 0;
    while i < actual.len()
        invariant 0 <= i <= actual.len(),
            forall|j: int| 0 <= j < i ==>
                text_member(expected@, (#[trigger] actual[j])@),
        decreases actual.len() - i,
    {
        if !contains_text(expected, actual[i]) {
            return false;
        }
        i += 1;
    }
    let mut i = 0;
    while i < expected.len()
        invariant 0 <= i <= expected.len(),
            forall|j: int| 0 <= j < actual.len() ==>
                text_member(expected@, (#[trigger] actual[j])@),
            forall|j: int| 0 <= j < i ==>
                text_member(actual@, (#[trigger] expected[j])@),
        decreases expected.len() - i,
    {
        if !contains_text(actual, expected[i]) {
            return false;
        }
        i += 1;
    }
    true
}

pub open spec fn text_values(values: Seq<&str>) -> Seq<Seq<char>> {
    values.map(|i: int, text: &str| text@)
}

pub open spec fn character_member(values: Seq<Seq<char>>, target: Seq<char>) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i]) == target
}

proof fn text_member_representation(values: Seq<&str>, target: Seq<char>)
    ensures text_member(values, target) <==> character_member(text_values(values), target),
{
    if text_member(values, target) {
        let i = choose|i: int| 0 <= i < values.len() && (#[trigger] values[i])@ == target;
        assert(text_values(values)[i] == target);
    }
    if character_member(text_values(values), target) {
        let i = choose|i: int| 0 <= i < text_values(values).len()
            && text_values(values)[i] == target;
        assert(values[i]@ == target);
    }
}

pub open spec fn result_membership(actual: Seq<&str>, expected: Seq<Seq<char>>) -> bool {
    (forall|i: int| 0 <= i < actual.len() ==>
        character_member(expected, (#[trigger] actual[i])@))
        && (forall|i: int| 0 <= i < expected.len() ==>
            text_member(actual, #[trigger] expected[i]))
}

proof fn result_membership_representation(actual: Seq<&str>, expected: Seq<&str>)
    ensures same_text_membership(actual, expected)
        <==> result_membership(actual, text_values(expected)),
{
    if same_text_membership(actual, expected) {
        assert forall|i: int| 0 <= i < actual.len() implies
            character_member(text_values(expected), (#[trigger] actual[i])@) by {
                text_member_representation(expected, actual[i]@);
            }
        assert forall|i: int| 0 <= i < text_values(expected).len() implies
            text_member(actual, #[trigger] text_values(expected)[i]) by {
                assert(text_values(expected)[i] == expected[i]@);
                assert(text_member(actual, expected[i]@));
            }
    }
    if result_membership(actual, text_values(expected)) {
        assert forall|i: int| 0 <= i < actual.len() implies
            text_member(expected, (#[trigger] actual[i])@) by {
                text_member_representation(expected, actual[i]@);
            }
        assert forall|i: int| 0 <= i < expected.len() implies
            text_member(actual, (#[trigger] expected[i])@) by {
                assert(text_values(expected)[i] == expected[i]@);
                assert(text_member(actual, text_values(expected)[i]));
            }
    }
}

pub open spec fn expected_result_ids(
    operations: Seq<&str>, not_required: Seq<&str>, require_verus: bool,
) -> Seq<Seq<char>> {
    seq!["ci.inventory"@] + text_values(operations) + text_values(not_required)
        + if require_verus { seq!["ci.verus-coverage"@] } else { Seq::empty() }
}

pub fn matches_result_ids(
    actual: &Vec<&str>, operations: &Vec<&str>, not_required: &Vec<&str>, require_verus: bool,
    coverage: Option<(bool, Option<i64>)>,
) -> (matches: bool)
    ensures matches <==> (result_membership(
        actual@, expected_result_ids(operations@, not_required@, require_verus))
        && (!require_verus || coverage == Some((true, None::<i64>)))),
{
    let mut expected = Vec::new();
    expected.push("ci.inventory");
    let mut i = 0;
    while i < operations.len()
        invariant 0 <= i <= operations.len(),
            text_values(expected@) == seq!["ci.inventory"@] + text_values(operations@).take(i as int),
        decreases operations.len() - i,
    {
        let ghost before = text_values(expected@);
        expected.push(operations[i]);
        proof {
            assert(text_values(expected@) =~= before.push(operations@[i as int]@));
            assert(text_values(operations@).take(i + 1)
                =~= text_values(operations@).take(i as int).push(operations@[i as int]@));
        }
        i += 1;
    }
    let mut i = 0;
    while i < not_required.len()
        invariant 0 <= i <= not_required.len(),
            text_values(expected@) == seq!["ci.inventory"@] + text_values(operations@)
                + text_values(not_required@).take(i as int),
        decreases not_required.len() - i,
    {
        let ghost before = text_values(expected@);
        expected.push(not_required[i]);
        proof {
            assert(text_values(expected@) =~= before.push(not_required@[i as int]@));
            assert(text_values(not_required@).take(i + 1)
                =~= text_values(not_required@).take(i as int).push(not_required@[i as int]@));
        }
        i += 1;
    }
    if require_verus {
        let ghost before = text_values(expected@);
        expected.push("ci.verus-coverage");
        assert(text_values(expected@) =~= before.push("ci.verus-coverage"@));
    }
    let matches = matches_text_membership(actual, &expected);
    proof {
        assert(text_values(expected@) =~= expected_result_ids(operations@, not_required@, require_verus));
        result_membership_representation(actual@, expected@);
    }
    matches && (!require_verus || match coverage {
        Some((passed, exit)) => passed && exit.is_none(),
        None => false,
    })
}

/// Rust 1.95 dep-info escapes ASCII spaces only. This bounded grammar rejects
/// controls and literal backslashes rather than interpreting them as Make syntax.
pub open spec fn dependency_byte(byte: u8) -> bool {
    byte >= 32 && byte != 92 && byte != 127
}

pub open spec fn dependency_word(word: Seq<u8>) -> bool {
    word.len() > 0 && forall|i: int| 0 <= i < word.len() ==> dependency_byte(#[trigger] word[i])
}

pub open spec fn dependency_escape(word: Seq<u8>) -> Seq<u8>
    decreases word.len(),
{
    if word.len() == 0 { Seq::empty() }
    else {
        dependency_escape(word.drop_last())
            + if word.last() == 32 { seq![92u8, 32u8] } else { seq![word.last()] }
    }
}

pub open spec fn dependency_values(paths: Seq<Vec<u8>>) -> Seq<Seq<u8>> {
    paths.map(|i: int, path: Vec<u8>| path@)
}

pub open spec fn dependency_encoding(paths: Seq<Seq<u8>>) -> Seq<u8>
    decreases paths.len(),
{
    if paths.len() == 0 { Seq::empty() }
    else {
        dependency_encoding(paths.drop_last())
            + if paths.len() == 1 { Seq::empty() } else { seq![32u8] }
            + dependency_escape(paths.last())
    }
}

pub open spec fn dependency_prefix(paths: Seq<Seq<u8>>, word: Seq<u8>) -> Seq<u8> {
    if paths.len() == 0 { dependency_escape(word) }
    else { dependency_encoding(paths) + seq![32u8] + dependency_escape(word) }
}

proof fn dependency_prefix_push(paths: Seq<Seq<u8>>, word: Seq<u8>, byte: u8)
    ensures dependency_prefix(paths, word.push(byte)) == dependency_prefix(paths, word)
        + if byte == 32 { seq![92u8, 32u8] } else { seq![byte] },
{
    assert(word.push(byte).drop_last() =~= word);
}

proof fn dependency_encoding_push(paths: Seq<Seq<u8>>, word: Seq<u8>)
    ensures dependency_encoding(paths.push(word)) == dependency_prefix(paths, word),
{
    assert(paths.push(word).drop_last() =~= paths);
    assert(paths.push(word).last() == word);
    if paths.len() == 0 {
        assert(paths =~= Seq::<Seq<u8>>::empty());
        assert(dependency_encoding(paths) == Seq::<u8>::empty());
    }
}

/// Exact error and delimiter semantics, including leading/trailing/repeated
/// separators and unknown escapes. Successful paths retain their input order.
pub open spec fn dependency_scan(
    input: Seq<u8>, paths: Seq<Seq<u8>>, word: Seq<u8>,
) -> Option<Seq<Seq<u8>>>
    decreases input.len(),
{
    if input.len() == 0 {
        if word.len() == 0 { None } else { Some(paths.push(word)) }
    } else if input[0] == 32 {
        if word.len() == 0 { None }
        else { dependency_scan(input.drop_first(), paths.push(word), Seq::empty()) }
    } else if input[0] == 92 {
        if input.len() < 2 || input[1] != 32 { None }
        else { dependency_scan(input.skip(2), paths, word.push(32u8)) }
    } else if !dependency_byte(input[0]) {
        None
    } else {
        dependency_scan(input.drop_first(), paths, word.push(input[0]))
    }
}

pub fn parse_dependency_paths(input: &[u8]) -> (parsed: Option<Vec<Vec<u8>>>)
    ensures
        match parsed {
            None => dependency_scan(input@, Seq::empty(), Seq::empty()).is_none(),
            Some(paths) => {
                let values = dependency_values(paths@);
                dependency_scan(input@, Seq::empty(), Seq::empty()) == Some(values)
                    && values.len() > 0
                    && (forall|i: int| 0 <= i < values.len() ==> dependency_word(#[trigger] values[i]))
                    && dependency_encoding(values) == input@
            },
        },
{
    let mut paths: Vec<Vec<u8>> = Vec::new();
    let mut word: Vec<u8> = Vec::new();
    let mut i = 0;
    assert(dependency_values(paths@) =~= Seq::<Seq<u8>>::empty());
    assert(input@.skip(0) =~= input@);
    while i < input.len()
        invariant
            0 <= i <= input.len(),
            dependency_scan(input@, Seq::empty(), Seq::empty())
                == dependency_scan(input@.skip(i as int), dependency_values(paths@), word@),
            dependency_prefix(dependency_values(paths@), word@) == input@.take(i as int),
            forall|j: int| 0 <= j < paths.len() ==> dependency_word((#[trigger] paths[j])@),
            forall|j: int| 0 <= j < word.len() ==> dependency_byte(#[trigger] word[j]),
        decreases input.len() - i,
    {
        let ghost suffix = input@.skip(i as int);
        let ghost previous_paths = dependency_values(paths@);
        let ghost previous_word = word@;
        let ghost previous_prefix = dependency_prefix(previous_paths, previous_word);
        let byte = input[i];
        if byte == 32 {
            if word.len() == 0 { return None; }
            let ghost previous = dependency_values(paths@);
            let ghost last = word@;
            paths.push(word);
            word = Vec::new();
            assert(dependency_values(paths@) =~= previous.push(last));
            proof { dependency_encoding_push(previous, last); }
            assert(dependency_prefix(dependency_values(paths@), word@)
                == previous_prefix + seq![32u8]);
            assert(input@.take(i + 1) =~= input@.take(i as int).push(byte));
            assert(input@.skip(i + 1) =~= suffix.drop_first());
            i += 1;
        } else if byte == 92 {
            if input.len() - i < 2 || input[i + 1] != 32 { return None; }
            word.push(32u8);
            proof { dependency_prefix_push(previous_paths, previous_word, 32u8); }
            assert(input@.take(i + 2) =~= input@.take(i as int) + seq![92u8, 32u8]);
            assert(input@.skip(i + 2) =~= suffix.skip(2));
            i += 2;
        } else {
            if byte < 32 || byte == 127 { return None; }
            word.push(byte);
            proof { dependency_prefix_push(previous_paths, previous_word, byte); }
            assert(input@.take(i + 1) =~= input@.take(i as int).push(byte));
            assert(input@.skip(i + 1) =~= suffix.drop_first());
            i += 1;
        }
    }
    if word.len() == 0 { return None; }
    let ghost previous = dependency_values(paths@);
    let ghost last = word@;
    assert(input@.skip(i as int) =~= Seq::<u8>::empty());
    assert(input@.take(i as int) =~= input@);
    paths.push(word);
    assert(dependency_values(paths@) =~= previous.push(last));
    proof { dependency_encoding_push(previous, last); }
    assert(dependency_encoding(dependency_values(paths@)) == input@);
    assert(dependency_scan(input@, Seq::empty(), Seq::empty())
        == Some(dependency_values(paths@)));
    assert forall|j: int| 0 <= j < paths.len()
        implies dependency_word((#[trigger] paths[j])@) by {
        if j < previous.len() { assert(paths@[j]@ == previous[j]); }
        else { assert(paths@[j]@ == last); }
    }
    Some(paths)
}

/// Decoded fields remain borrowed from the actual registered requirements.
/// The adapter still owns and must prove their extraction from library values.
pub struct RequirementFields<'a> {
    pub schema: &'a str,
    pub tool: &'a str,
    pub version: &'a str,
    pub commit: &'a str,
    pub interfaces: Vec<(&'a str, &'a str)>,
}

pub open spec fn requirement_fields_valid(fields: &RequirementFields, tools: Seq<&str>) -> bool {
    fields.schema@ == "chrono-verus/v1"@
        && text_member(tools, fields.tool@)
        && fields.version@.len() > 0 && fields.commit@.len() > 0
        && fields.interfaces.len() > 0
        && forall|i: int| 0 <= i < fields.interfaces.len() ==> (
            (#[trigger] fields.interfaces[i]).0@.len() > 0
            && fields.interfaces[i].1@.len() > 0
        )
}

pub fn valid_requirement_fields(fields: &RequirementFields, tools: &Vec<&str>) -> (valid: bool)
    ensures valid <==> requirement_fields_valid(fields, tools@),
{
    if !equal_text(fields.schema, "chrono-verus/v1")
        || !contains_text(tools, fields.tool)
        || fields.version.is_empty() || fields.commit.is_empty()
        || fields.interfaces.len() == 0
    {
        return false;
    }
    let mut i = 0;
    while i < fields.interfaces.len()
        invariant
            0 <= i <= fields.interfaces.len(),
            fields.schema@ == "chrono-verus/v1"@,
            text_member(tools@, fields.tool@),
            fields.version@.len() > 0 && fields.commit@.len() > 0,
            fields.interfaces.len() > 0,
            forall|j: int| 0 <= j < i ==> (
                (#[trigger] fields.interfaces[j]).0@.len() > 0
                && fields.interfaces[j].1@.len() > 0
            ),
        decreases fields.interfaces.len() - i,
    {
        if fields.interfaces[i].0.is_empty() || fields.interfaces[i].1.is_empty() {
            return false;
        }
        proof {
            assert forall|j: int| 0 <= j < i + 1 implies (
                (#[trigger] fields.interfaces[j]).0@.len() > 0
                && fields.interfaces[j].1@.len() > 0
            ) by {
                if j < i {
                    assert(fields.interfaces[j].0@.len() > 0);
                    assert(fields.interfaces[j].1@.len() > 0);
                }
            }
        }
        i += 1;
    }
    true
}

/// Fixed checking flags are independent of root/source and output-path binding.
/// Those bindings remain separate obligations at their actual owners.
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
        invariant
            0 <= i <= suffix.len(),
            bytes@ == before + suffix@.take(i as int),
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

pub open spec fn verifier_arguments(args: Seq<String>) -> bool {
    args.len() == 12
        && args[0]@ == "--crate-type=lib"@
        && args[1]@ == "--no-cheating"@
        && args[2]@ == "--no-external-by-default"@
        && args[3]@ == "--num-threads"@
        && args[4]@ == "2"@
        && args[5]@ == "--output-json"@
        && args[6]@ == "--compile"@
        && args[7]@ == "--emit=link="@ + args[10]@ + ",dep-info="@ + args[10]@ + ".d"@
        && args[9]@ == "-o"@
        && args[11]@ == "-Zchecksum-hash-algorithm=sha256"@
}

pub fn canonical_verifier_arguments(args: &Vec<String>) -> (valid: bool)
    ensures valid <==> verifier_arguments(args@),
{
    if args.len() != 12 {
        return false;
    }
    let outputs = compiler_outputs(args[10].as_str());
    let outputs_match = equal_bytes(args[7].as_str().as_bytes(), outputs.as_slice());
    proof {
        vstd::utf8::encode_utf8_decode_utf8(args[7]@);
        vstd::utf8::encode_utf8_decode_utf8(
            "--emit=link="@ + args[10]@ + ",dep-info="@ + args[10]@ + ".d"@);
    }
    equal_text(args[0].as_str(), "--crate-type=lib")
        && equal_text(args[1].as_str(), "--no-cheating")
        && equal_text(args[2].as_str(), "--no-external-by-default")
        && equal_text(args[3].as_str(), "--num-threads")
        && equal_text(args[4].as_str(), "2")
        && equal_text(args[5].as_str(), "--output-json")
        && equal_text(args[6].as_str(), "--compile")
        && outputs_match
        && equal_text(args[9].as_str(), "-o")
        && equal_text(args[11].as_str(), "-Zchecksum-hash-algorithm=sha256")
}

/// One record binds the finding identity to the decision which produced it.
pub struct Obligation {
    pub identity: String,
    pub accepted: bool,
}

/// The identity sequence preserves every rejected record, including duplicates.
pub open spec fn rejected_identities(entries: Seq<Obligation>) -> Seq<String>
    decreases entries.len(),
{
    if entries.len() == 0 {
        Seq::empty()
    } else {
        let earlier = rejected_identities(entries.drop_last());
        if entries.last().accepted {
            earlier
        } else {
            earlier.push(entries.last().identity)
        }
    }
}

pub struct Coverage {
    pub obligations: usize,
    pub complete: bool,
    pub missing: Vec<String>,
}

impl Coverage {
    pub fn first_missing(&self) -> (first: Option<&str>)
        ensures
            first.is_none() <==> self.missing.len() == 0,
            first.is_some() ==> first.unwrap()@ == self.missing[0]@,
    {
        if self.missing.len() == 0 {
            None
        } else {
            Some(self.missing[0].as_str())
        }
    }
}

/// The private vector is the sole owner of coverage registration state.
/// Finishing consumes the ledger, so no later record can change that result.
pub struct Ledger {
    entries: Vec<Obligation>,
}

impl Ledger {
    pub closed spec fn view(&self) -> Seq<Obligation> {
        self.entries@
    }

    pub fn new() -> (ledger: Ledger)
        ensures ledger.view() == Seq::<Obligation>::empty(),
    {
        Ledger { entries: Vec::new() }
    }

    pub fn record(&mut self, identity: String, accepted: bool)
        ensures self.view() == old(self).view().push(Obligation { identity, accepted }),
    {
        self.entries.push(Obligation { identity, accepted });
    }

    pub fn finish(self) -> (coverage: Coverage)
        ensures
            coverage.obligations == self.view().len(),
            coverage.missing@ == rejected_identities(self.view()),
            coverage.complete <==> (
                self.view().len() > 0
                && forall|i: int| 0 <= i < self.view().len() ==> #[trigger] self.view()[i].accepted
            ),
            coverage.missing.len() == 0 <==> (
                forall|i: int| 0 <= i < self.view().len() ==> #[trigger] self.view()[i].accepted
            ),
    {
        let mut missing = Vec::new();
        let mut i = 0;
        while i < self.entries.len()
            invariant
                0 <= i <= self.entries.len(),
                missing@ == rejected_identities(self.entries@.take(i as int)),
                missing.len() == 0 <==> (
                    forall|j: int| 0 <= j < i ==> #[trigger] self.entries[j].accepted
                ),
            decreases self.entries.len() - i,
        {
            if !self.entries[i].accepted {
                missing.push(self.entries[i].identity.clone());
            }
            proof {
                let prefix = self.entries@.take(i + 1);
                assert(prefix.drop_last() =~= self.entries@.take(i as int));
                assert(prefix.last() == self.entries@[i as int]);
            }
            i += 1;
        }
        assert(self.entries@.take(i as int) =~= self.entries@);
        let complete = self.entries.len() > 0 && missing.len() == 0;
        Coverage { obligations: self.entries.len(), complete, missing }
    }
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

/// External JSON decoding supplies optional typed fields. Missing/wrongly
/// typed fields stay absent; the repository owns and proves their admission.
pub struct CompilerResult<'a> {
    pub exit_code: Option<i64>,
    pub success: Option<bool>,
    pub encountered_error: Option<bool>,
    pub encountered_vir_error: Option<bool>,
    pub errors: Option<u64>,
    pub verified: Option<u64>,
    pub entire_crate: Option<bool>,
    pub version: Option<&'a [u8]>,
    pub commit: Option<&'a [u8]>,
}

pub open spec fn compiler_accepted(
    result: &CompilerResult,
    version: Seq<u8>,
    commit: Seq<u8>,
) -> bool {
    version.len() > 0 && commit.len() > 0
        && result.exit_code == Some(0i64)
        && result.success == Some(true)
        && result.encountered_error == Some(false)
        && result.encountered_vir_error == Some(false)
        && result.errors == Some(0u64)
        && result.verified.is_some() && result.verified.unwrap() > 0
        && result.entire_crate == Some(true)
        && result.version.is_some() && result.version.unwrap()@ == version
        && result.commit.is_some() && result.commit.unwrap()@ == commit
}

pub fn accept_compiler_result(
    result: &CompilerResult,
    version: &[u8],
    commit: &[u8],
) -> (accepted: bool)
    ensures accepted <==> compiler_accepted(result, version@, commit@),
{
    if version.len() == 0 || commit.len() == 0
        || result.exit_code != Some(0i64)
        || result.success != Some(true)
        || result.encountered_error != Some(false)
        || result.encountered_vir_error != Some(false)
        || result.errors != Some(0u64)
        || result.entire_crate != Some(true)
    {
        return false;
    }
    match result.verified {
        Some(count) => { if count == 0 { return false; } },
        None => { return false; },
    }
    match (result.version, result.commit) {
        (Some(actual_version), Some(actual_commit)) => {
            equal_bytes(actual_version, version) && equal_bytes(actual_commit, commit)
        },
        _ => false,
    }
}

#[verifier::inline]
pub open spec fn oid_byte(byte: u8) -> bool {
    48 <= byte <= 57 || 97 <= byte <= 102
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
