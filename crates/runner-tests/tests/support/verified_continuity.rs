//! 固定独立预期：合法搬迁、逻辑变化、错误对象、复制和分类退化。
use super::{expect_verifier_true, identifiers, policy};
use policy::sources::{self, continuity as c};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub struct Bytes { pub left: Vec<u8>, pub right: Vec<u8> }
impl c::Read for Bytes {
    open spec fn contents(&self, base: bool, _path: Seq<char>) -> Result<Seq<u8>, Seq<char>> {
        if (base && _path == "a/x"@) || (!base && _path == "b/x"@) {
            Ok(if base { self.left@ } else { self.right@ })
        } else { Err("unexpected source input"@) }
    }
    fn blob(&self, base: bool, _path: &str) -> (out: Result<Vec<u8>, String>)
        ensures match out { Ok(value) => self.contents(base, _path@) == Ok::<Seq<u8>, Seq<char>>(value@),
            Err(error) => self.contents(base, _path@) == Err::<Seq<u8>, Seq<char>>(error@) },
    {
        reveal(<Bytes as c::Read>::contents);
        if (base && identifiers::equal_text(_path, "a/x")) || (!base && identifiers::equal_text(_path, "b/x")) {
            assert(self.contents(base, _path@) == Ok::<Seq<u8>, Seq<char>>(if base { self.left@ } else { self.right@ }));
            if base {
                let bytes = sources::copy_range(self.left.as_slice(), 0, self.left.len());
                assert(bytes@ =~= self.left@);
                Ok(bytes)
            } else {
                let bytes = sources::copy_range(self.right.as_slice(), 0, self.right.len());
                assert(bytes@ =~= self.right@);
                Ok(bytes)
            }
        } else {
            assert(self.contents(base, _path@) == Err::<Seq<u8>, Seq<char>>("unexpected source input"@));
            Err("unexpected source input".to_string())
        }
    }
}

fn file(path: &str) -> (out: c::FileObservation)
    ensures out.path@ == path@, out.metadata@ == seq![1u8], out.mode@ == "100644"@,
        out.kind@ == "blob"@, out.registered, !out.classified,
{
    c::FileObservation { path: path.to_string(), metadata: vec![1u8], mode: "100644".to_string(),
        kind: "blob".to_string(), registered: true, classified: false }
}

fn side(root: &str, path: &str) -> (out: c::Endpoint)
    ensures out.projects.len() == 1, out.projects[0].id@ == "p"@, out.projects[0].root@ == root@,
        out.files.len() == 1, out.files[0].path@ == path@, out.files[0].metadata@ == seq![1u8],
        out.files[0].mode@ == "100644"@, out.files[0].kind@ == "blob"@, out.files[0].registered,
        !out.files[0].classified,
        out.outputs.len() == 0, out.reference_syntax.len() == 0,
{
    c::Endpoint { projects: vec![c::ProjectRoot { id: "p".to_string(), root: root.to_string() }],
        files: vec![file(path)], outputs: Vec::new(), reference_syntax: Vec::new() }
}

fn unknown(path: &str) -> (out: sources::SourceRegistry)
    ensures out.files.require_new_file_classification == Some(true), out.files.entries.len() == 1,
        out.files.entries[0].path@ == path@, out.files.entries[0].verification.is_none(),
{
    sources::source_registry(vec![sources::SourceFile { path: path.to_string(), verification: None }], Some(true))
}

proof fn names()
    ensures vstd::utf8::encode_utf8("a"@) == seq![97u8],
        vstd::utf8::encode_utf8("b"@) == seq![98u8],
        vstd::utf8::encode_utf8("a/x"@) == seq![97u8,47u8,120u8],
        vstd::utf8::encode_utf8("b/x"@) == seq![98u8,47u8,120u8],
        "a/x"@ != "b/x"@, "a"@ != "b"@,
{
    reveal_strlit("a"); reveal_strlit("b"); reveal_strlit("a/x"); reveal_strlit("b/x");
    assert("a"@ =~= seq!['a']); assert("b"@ =~= seq!['b']);
    assert("a/x"@ =~= seq!['a','/','x']); assert("b/x"@ =~= seq!['b','/','x']);
    vstd::utf8::is_ascii_chars_encode_utf8("a"@); vstd::utf8::is_ascii_chars_encode_utf8("b"@);
    vstd::utf8::is_ascii_chars_encode_utf8("a/x"@); vstd::utf8::is_ascii_chars_encode_utf8("b/x"@);
}

proof fn single_move(base: &c::Endpoint, current: &c::Endpoint)
    requires base.projects.len() == 1, current.projects.len() == 1,
        base.projects[0].id@ == "p"@, current.projects[0].id@ == "p"@,
        base.projects[0].root@ == "a"@, current.projects[0].root@ == "b"@,
        base.files.len() == 1, current.files.len() == 1,
        base.files[0].path@ == "a/x"@, current.files[0].path@ == "b/x"@,
        base.files[0].registered, current.files[0].registered,
        base.files[0].mode@ == "100644"@, current.files[0].mode@ == "100644"@,
        base.files[0].kind@ == "blob"@, current.files[0].kind@ == "blob"@,
        base.files[0].metadata@ == current.files[0].metadata@,
    ensures c::relocation(current.projects@, base.projects@, seq![98u8,47u8,120u8]) == Some(seq![97u8,47u8,120u8]),
        c::file_index(base.files@, seq![97u8,47u8,120u8]) == Some(0int),
        c::file_index(current.files@, seq![98u8,47u8,120u8]) == Some(0int),
        c::comparable(&base.files[0], &current.files[0], base, current),
        !c::manifest_path(seq![98u8,47u8,120u8]),
{
    names();
    assert(c::under(seq![98u8,47u8,120u8], seq![98u8])) by (compute);
    assert(c::project_match(current.projects@, base.projects@, 0, 0, seq![98u8,47u8,120u8]));
    assert(c::moved_target(current.projects@, base.projects@, seq![98u8,47u8,120u8], 0, 0)
        =~= seq![97u8,47u8,120u8]);
    assert forall|i: int, j: int| c::project_match(current.projects@, base.projects@, i, j,
        seq![98u8,47u8,120u8]) implies i == 0 && j == 0 by { }
    assert(exists|pair: (int,int)| c::project_match(current.projects@, base.projects@,
        pair.0, pair.1, seq![98u8,47u8,120u8])) by {
        assert(c::project_match(current.projects@, base.projects@, (0int,0int).0, (0int,0int).1,
            seq![98u8,47u8,120u8]));
    }
    let pair = choose|pair: (int,int)| c::project_match(current.projects@, base.projects@,
        pair.0, pair.1, seq![98u8,47u8,120u8]);
    assert(pair == (0int,0int));
    assert(c::unique_file(base.files@, seq![97u8,47u8,120u8], 0));
    assert(c::unique_file(current.files@, seq![98u8,47u8,120u8], 0));
    assert(c::absent(base.files@, seq![98u8,47u8,120u8]));
    assert(c::absent(current.files@, seq![97u8,47u8,120u8]));
    assert(!c::manifest_path(seq![98u8,47u8,120u8])) by (compute);
}

proof fn plain(base: &c::Endpoint, current: &c::Endpoint)
    requires current.reference_syntax.len() == 0,
    ensures c::source_equal(seq![120u8], seq![120u8], 0, 0, seq![97u8,47u8,120u8],
        seq![98u8,47u8,120u8], base, current, false),
        !c::source_equal(seq![120u8], seq![121u8], 0, 0, seq![97u8,47u8,120u8],
        seq![98u8,47u8,120u8], base, current, false),
{
    assert forall|k: u16| !c::anchor(seq![120u8], 0, k, false, current, seq![98u8,47u8,120u8], false) by { reveal(c::prefix); reveal(c::builtin_prefix); }
    reveal_with_fuel(c::source_equal, 3);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_identity_preserves_unknown_in_both_gates() {
    let base = side("a", "a/x"); let current = side("b", "b/x");
    let io = Bytes { left: vec![120u8], right: vec![120u8] };
    proof {
        names(); single_move(&base, &current); plain(&base, &current); reveal(<Bytes as c::Read>::contents);
        assert(!c::manifest_path(seq![98u8,47u8,120u8])) by (compute);
        reveal(c::assessment);
        assert(c::assessment(&base, &current, seq![98u8,47u8,120u8], &io).is_ok());
        assert(<Bytes as c::Read>::contents(&io, true, base.files[0].path@) == Ok::<Seq<u8>, Seq<char>>(io.left@));
        assert(<Bytes as c::Read>::contents(&io, false, current.files[0].path@) == Ok::<Seq<u8>, Seq<char>>(io.right@));
        assert(io.left@ =~= seq![120u8]); assert(io.right@ =~= seq![120u8]);
        assert(c::source_equal(io.left@, io.right@, 0, 0, seq![97u8,47u8,120u8], seq![98u8,47u8,120u8], &base, &current, false));
        assert(match c::assessment(&base, &current, seq![98u8,47u8,120u8], &io) {
            Ok(value) => value.1.is_some(), Err(_) => false,
        });
    }
    let evidence = c::assess(&base, &current, &vec!["b/x".to_string()], &io);
    expect_verifier_true(evidence.is_ok());
    let evidence = evidence.unwrap();
    proof {
        assert(evidence.view().len() == 1);
        assert(evidence.view()[0].1.is_some());
        assert(evidence.view()[0].1.unwrap().1 == "b/x"@);
        assert(evidence.preserves("b/x"@));
    }
    let kept = evidence.preserved("b/x"); expect_verifier_true(kept);
    let old = unknown("a/x"); let new = unknown("b/x");
    proof { reveal_with_fuel(sources::classifications_error, 3); }
    let inventory = sources::validate_file_classifications(&new, Ok(Some(&old)), &evidence);
    expect_verifier_true(inventory.is_ok());
    let nodes = vec!["file:b/x".to_string()];
    proof { reveal_with_fuel(sources::selected_plan_error, 3); reveal(sources::file_plan_error); }
    let plan = sources::validate_selected_roots(&new.files, &nodes, &Vec::new(), &Vec::new(), &evidence);
    expect_verifier_true(plan.is_ok());
    expect_verifier_true(new.files.entries[0].verification.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn changed_logic_gets_no_source_continuity() {
    let base = side("a", "a/x"); let current = side("b", "b/x");
    let io = Bytes { left: vec![120u8], right: vec![121u8] };
    proof {
        names(); single_move(&base, &current); plain(&base, &current); reveal(<Bytes as c::Read>::contents);
        assert(!c::manifest_path(seq![98u8,47u8,120u8])) by (compute);
        reveal(c::assessment);
        assert(c::assessment(&base, &current, seq![98u8,47u8,120u8], &io).is_ok());
    }
    let evidence = c::assess(&base, &current, &vec!["b/x".to_string()], &io);
    expect_verifier_true(evidence.is_ok()); let evidence = evidence.unwrap();
    let kept = evidence.preserved("b/x"); expect_verifier_true(!kept);
    let old = unknown("a/x"); let new = unknown("b/x");
    proof { reveal_with_fuel(sources::classifications_error, 3); }
    let inventory = sources::validate_file_classifications(&new, Ok(Some(&old)), &evidence);
    expect_verifier_true(inventory.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn a_copy_is_not_a_move_and_missing_sources_stay_unproven() {
    let base = side("a", "a/x"); let mut current = side("b", "b/x");
    proof { single_move(&base, &current); }
    current.files.push(file("a/x"));
    let io = Bytes { left: vec![120u8], right: vec![120u8] };
    proof {
        names(); reveal(c::assessment);
        assert(c::unique_file(current.files@, seq![98u8,47u8,120u8], 0));
        assert(c::file_index(current.files@, seq![98u8,47u8,120u8]) == Some(0int));
        assert(!c::absent(current.files@, seq![97u8,47u8,120u8])) by {
            assert(vstd::utf8::encode_utf8(current.files[1].path@) == seq![97u8,47u8,120u8]);
        }
        assert(c::assessment(&base, &current, seq![98u8,47u8,120u8], &io)
            == Ok::<c::AssessmentView, Seq<char>>((Some(("a/x"@, "b/x"@)), None)));
    }
    let evidence = c::assess(&base, &current, &vec!["b/x".to_string()], &io);
    expect_verifier_true(evidence.is_ok()); let evidence = evidence.unwrap();
    let copied = evidence.preserved("b/x"); expect_verifier_true(!copied);
    let missing = c::relocate("b/x".as_bytes(), &current.projects, &Vec::new());
    let same = match missing { None => false, Some(value) => identifiers::equal_bytes(value.as_slice(), "b/x".as_bytes()) };
    expect_verifier_true(same);
    let no_base = c::assess(&c::Endpoint { projects: Vec::new(), files: Vec::new(), outputs: Vec::new(), reference_syntax: Vec::new() },
        &current, &vec!["b/x".to_string()], &io);
    expect_verifier_true(no_base.is_ok()); let no_base = no_base.unwrap();
    let kept = no_base.preserved("b/x"); expect_verifier_true(!kept);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ambiguous_project_identity_is_rejected() {
    let base = side("a", "a/x"); let mut current = side("b", "b/x");
    current.projects.push(c::ProjectRoot { id: "p".to_string(), root: "b".to_string() });
    proof {
        names();
        assert(c::under(seq![97u8,47u8,120u8], seq![97u8])) by (compute);
        assert(c::project_match(base.projects@, current.projects@, 0, 0, seq![97u8,47u8,120u8]));
        assert(c::project_match(base.projects@, current.projects@, 0, 1, seq![97u8,47u8,120u8]));
        assert(c::relocation(base.projects@, current.projects@, seq![97u8,47u8,120u8]).is_none());
    }
    let out = c::relocate("a/x".as_bytes(), &base.projects, &current.projects);
    expect_verifier_true(out.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn target_rebinding_checks_the_actual_object_identity() {
    let mut base = side("a", "a/x"); let mut current = side("b", "b/x");
    base.files.push(file("a/y")); current.files.push(file("b/y")); current.files.push(file("b/z"));
    proof {
        names();
        reveal_strlit("a/y"); reveal_strlit("b/y"); reveal_strlit("b/z");
        assert("a/y"@ =~= seq!['a','/','y']); assert("b/y"@ =~= seq!['b','/','y']); assert("b/z"@ =~= seq!['b','/','z']);
        vstd::utf8::is_ascii_chars_encode_utf8("a/y"@); vstd::utf8::is_ascii_chars_encode_utf8("b/y"@);
        vstd::utf8::is_ascii_chars_encode_utf8("b/z"@);
        assert(c::directory(seq![97u8,47u8,120u8]) == seq![97u8]) by (compute);
        assert(c::directory(seq![98u8,47u8,120u8]) == seq![98u8]) by (compute);
        assert(c::resolved(seq![97u8], seq![121u8]) == Some(seq![97u8,47u8,121u8])) by (compute);
        assert(c::resolved(seq![98u8], seq![121u8]) == Some(seq![98u8,47u8,121u8])) by (compute);
        assert(c::resolved(seq![98u8], seq![122u8]) == Some(seq![98u8,47u8,122u8])) by (compute);
        assert(c::under(seq![97u8,47u8,121u8], seq![97u8])) by (compute);
        assert("a/x"@ != "a/y"@); assert("b/x"@ != "b/y"@); assert("b/y"@ != "b/z"@);
        assert(seq![97u8,47u8,120u8] != seq![97u8,47u8,121u8]) by (compute);
        assert(seq![98u8,47u8,120u8] != seq![98u8,47u8,121u8]) by (compute);
        assert(seq![98u8,47u8,121u8] != seq![98u8,47u8,122u8]) by (compute);
        assert(vstd::utf8::encode_utf8("a/y"@) =~= seq![97u8,47u8,121u8]);
        assert(vstd::utf8::encode_utf8("b/y"@) =~= seq![98u8,47u8,121u8]);
        assert(vstd::utf8::encode_utf8("b/z"@) =~= seq![98u8,47u8,122u8]);
        assert forall|j: int| 0 <= j < base.files.len() && j != 1 implies
            vstd::utf8::encode_utf8(base.files[j].path@) != seq![97u8,47u8,121u8] by {
            assert(j == 0);
        }
        assert forall|j: int| 0 <= j < current.files.len() && j != 1 implies
            vstd::utf8::encode_utf8(current.files[j].path@) != seq![98u8,47u8,121u8] by {
            if j != 0 { assert(j == 2); }
        }
        assert(c::unique_file(base.files@, seq![97u8,47u8,121u8], 1));
        assert(c::unique_file(current.files@, seq![98u8,47u8,121u8], 1));
        assert(c::unique_file(current.files@, seq![98u8,47u8,122u8], 2));
        assert(c::target_exists(&base, seq![97u8,47u8,121u8]));
        assert(c::target_exists(&current, seq![98u8,47u8,121u8]));
        assert(c::project_match(base.projects@, current.projects@, 0, 0, seq![97u8,47u8,121u8]));
        assert forall|i: int, j: int| c::project_match(base.projects@, current.projects@, i, j,
            seq![97u8,47u8,121u8]) implies i == 0 && j == 0 by { }
        assert(c::moved_target(base.projects@, current.projects@, seq![97u8,47u8,121u8], 0, 0)
            =~= seq![98u8,47u8,121u8]);
        reveal(c::reference_pair); reveal(c::reference_base);
        assert(c::reference_pair(seq![121u8], seq![121u8], seq![97u8,47u8,120u8], seq![98u8,47u8,120u8],
            &base, &current, 1));
        assert(!c::reference_pair(seq![121u8], seq![122u8], seq![97u8,47u8,120u8], seq![98u8,47u8,120u8],
            &base, &current, 1));
    }
    let left: &[u8] = &[121u8]; let right: &[u8] = &[122u8];
    let old_path = "a/x".as_bytes(); let new_path = "b/x".as_bytes();
    assert(left@ =~= seq![121u8]); assert(right@ =~= seq![122u8]);
    assert(old_path@ == seq![97u8,47u8,120u8]); assert(new_path@ == seq![98u8,47u8,120u8]);
    let yes = c::same_reference(left, left, old_path, new_path, &base, &current, 1);
    expect_verifier_true(yes);
    let wrong = c::same_reference(left, right, old_path, new_path, &base, &current, 1);
    expect_verifier_true(!wrong);
}


struct FailedRead {}
impl c::Read for FailedRead {
    open spec fn contents(&self, _base: bool, _path: Seq<char>) -> Result<Seq<u8>, Seq<char>> {
        Err("read failed"@)
    }
    fn blob(&self, _base: bool, _path: &str) -> (out: Result<Vec<u8>, String>)
        ensures match out { Err(error) => error@ == "read failed"@, Ok(_) => false },
    { Err("read failed".to_string()) }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn read_failure_preserves_the_original_error() {
    let base = side("a", "a/x"); let current = side("b", "b/x");
    proof { names(); single_move(&base, &current); reveal(c::assessment); }
    let out = c::assess(&base, &current, &vec!["b/x".to_string()], &FailedRead {});
    expect_verifier_true(out.is_err());
    let error = match out { Err(error) => error, Ok(_) => String::new() };
    let exact = identifiers::equal_text(error.as_str(), "read failed");
    expect_verifier_true(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn classified_sources_retain_origin_without_reading_or_granting_continuity() {
    let base = side("a", "a/x");
    let mut classified = file("b/x"); classified.classified = true;
    let current = c::Endpoint {
        projects: vec![c::ProjectRoot { id: "p".to_string(), root: "b".to_string() }],
        files: vec![classified], outputs: Vec::new(), reference_syntax: Vec::new(),
    };
    proof { names(); single_move(&base, &current); }
    proof { reveal(c::assessment); }
    // 任何源码读取都会返回原错误；已分类路径应只取得来源。
    let evidence = c::assess(&base, &current, &vec!["b/x".to_string()], &FailedRead {});
    expect_verifier_true(evidence.is_ok()); let evidence = evidence.unwrap();
    let origin = evidence.origin("b/x");
    expect_verifier_true(origin.is_some());
    let correct = identifiers::equal_text(origin.unwrap(), "a/x");
    expect_verifier_true(correct);
    let preserved = evidence.preserved("b/x");
    expect_verifier_true(!preserved);
    let old = sources::source_registry(vec![sources::SourceFile { path: "a/x".to_string(),
        verification: Some(sources::SourceVerification { role: sources::SourceRole::Program,
            roots: vec![sources::SourceProofRoot { path: "root".to_string(), operation: "proof".to_string() }] }) }], Some(true));
    let new = sources::source_registry(vec![sources::SourceFile { path: "b/x".to_string(),
        verification: Some(sources::SourceVerification { role: sources::SourceRole::Data, roots: Vec::new() }) }], Some(true));
    proof { reveal_with_fuel(sources::classifications_error, 3); reveal_with_fuel(c::first_origin, 3); }
    let result = sources::validate_file_classifications(&new, Ok(Some(&old)), &evidence);
    expect_verifier_true(result.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn moved_proof_obligations_cannot_become_data_when_source_changed() {
    let base = side("a", "a/x"); let current = side("b", "b/x");
    let io = Bytes { left: vec![120u8], right: vec![121u8] };
    proof {
        names(); single_move(&base, &current); plain(&base, &current); reveal(<Bytes as c::Read>::contents);
        assert(!c::manifest_path(seq![98u8,47u8,120u8])) by (compute);
        reveal(c::assessment);
    }
    let evidence = c::assess(&base, &current, &vec!["b/x".to_string()], &io);
    expect_verifier_true(evidence.is_ok()); let evidence = evidence.unwrap();
    let old = sources::source_registry(vec![sources::SourceFile { path: "a/x".to_string(),
        verification: Some(sources::SourceVerification { role: sources::SourceRole::Program,
            roots: vec![sources::SourceProofRoot { path: "root".to_string(), operation: "proof".to_string() }] }) }], Some(true));
    let new = sources::source_registry(vec![sources::SourceFile { path: "b/x".to_string(),
        verification: Some(sources::SourceVerification { role: sources::SourceRole::Data, roots: Vec::new() }) }], Some(true));
    proof { reveal_with_fuel(sources::classifications_error, 3); reveal_with_fuel(c::first_origin, 3); }
    let result = sources::validate_file_classifications(&new, Ok(Some(&old)), &evidence);
    expect_verifier_true(result.is_err());
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn declared_frames_are_bound_to_the_file_and_leave_all_other_bytes_exact() {
    let base = side("a", "a/x"); let mut current = side("b", "b/x");
    let left = [112u8,34,46,34];
    let right = [112u8,10,34,46,34];
    let extra = [112u8,10,34,46,34,33];
    current.reference_syntax.push(c::ReferenceSyntax {
        file: "b/x".to_string(), before: "p\"".to_string(), after: "p\n\"".to_string(),
        trailing: String::new(), suffix_before: String::new(), suffix_after: String::new(), base: 2,
    });
    proof {
        names();
        reveal_strlit("p\""); reveal_strlit("p\n\""); reveal_strlit("");
        vstd::utf8::is_ascii_chars_encode_utf8("p\""@);
        vstd::utf8::is_ascii_chars_encode_utf8("p\n\""@);
        vstd::utf8::is_ascii_chars_encode_utf8(""@);
        assert(vstd::utf8::encode_utf8("p\""@) =~= seq![112u8,34]);
        assert(vstd::utf8::encode_utf8("p\n\""@) =~= seq![112u8,10,34]);
        assert(vstd::utf8::encode_utf8(""@) =~= Seq::<u8>::empty());
        assert(left@ =~= seq![112u8,34,46,34]);
        assert(right@ =~= seq![112u8,10,34,46,34]);
        assert(extra@ =~= seq![112u8,10,34,46,34,33]);
        reveal(c::prefix); reveal(c::suffix); reveal(c::trailing); reveal(c::reference_pair);
        assert(c::prefix(6, &current, false) == seq![112u8,34]);
        assert(c::prefix(6, &current, true) == seq![112u8,10,34]);
        assert(left@.subrange(0,2) =~= seq![112u8,34]);
        assert(right@.subrange(0,3) =~= seq![112u8,10,34]);
        assert forall|k: u16| 1 <= k <= 5 implies c::builtin_prefix(k).len() > 5 by {
            reveal(c::builtin_prefix);
        }
        assert forall|k: u16| c::anchor(left@, 0, k, false, &current, seq![98u8,47u8,120u8], false)
            implies k == 6 by { }
        assert(c::anchor(left@, 0, 6, false, &current, seq![98u8,47u8,120u8], false));
        let chosen = choose|k: u16| c::anchor(left@, 0, k, false, &current, seq![98u8,47u8,120u8], false)
            && forall|j: u16| 1 <= j < k ==> !c::anchor(left@, 0, j, false, &current, seq![98u8,47u8,120u8], false);
        assert(chosen == 6);
        assert(sources::next_byte(seq![112u8,34,46,34], 2, 34) == 3) by (compute);
        assert(sources::next_byte(seq![112u8,10,34,46,34], 3, 34) == 4) by (compute);
        assert(sources::next_byte(seq![112u8,10,34,46,34,33], 3, 34) == 4) by (compute);
        assert(left@.subrange(2,3) =~= seq![46u8]);
        assert(right@.subrange(3,4) =~= seq![46u8]);
        assert(extra@.subrange(3,4) =~= seq![46u8]);
        assert(c::trailing(6,&current) =~= Seq::<u8>::empty());
        assert(c::suffix(6,&current,false) =~= Seq::<u8>::empty());
        assert(c::suffix(6,&current,true) =~= Seq::<u8>::empty());
        assert(c::reference_end(left@,0,6,&current,false) == 3);
        assert(c::reference_end(right@,0,6,&current,true) == 4);
        assert(left@.subrange(3,3) =~= Seq::<u8>::empty());
        assert(left@.subrange(4,4) =~= Seq::<u8>::empty());
        assert(right@.subrange(4,4) =~= Seq::<u8>::empty());
        assert(right@.subrange(5,5) =~= Seq::<u8>::empty());
        assert(c::closed_reference(left@,0,6,&current,false));
        assert(c::closed_reference(right@,0,6,&current,true));
        assert(c::resolved(Seq::<u8>::empty(), seq![46u8]) == Some(Seq::<u8>::empty())) by (compute);
        assert forall|i: int,j: int| !c::project_match(base.projects@, current.projects@, i,j, Seq::<u8>::empty()) by { }
        assert(c::reference_pair(seq![46u8], seq![46u8], seq![97u8,47u8,120u8],
            seq![98u8,47u8,120u8], &base, &current, 6));
        reveal_with_fuel(c::source_equal, 3);
    }
    let accepted = c::same_source(&left, &right, "a/x".as_bytes(), "b/x".as_bytes(), &base, &current, false);
    let changed = c::same_source(&left, &extra, "a/x".as_bytes(), "b/x".as_bytes(), &base, &current, false);
    expect_verifier_true(accepted); expect_verifier_true(!changed);
    let mut form = current.reference_syntax.pop().unwrap();
    form.file = "a/x".to_string();
    current.reference_syntax.push(form);
    proof {
        assert forall|k: u16| !c::anchor(left@, 0, k, false, &current, seq![98u8,47u8,120u8], false) by { }
        assert forall|k: u16| !c::anchor(left@, 1, k, false, &current, seq![98u8,47u8,120u8], false) by { }
        reveal_with_fuel(c::source_equal, 4);
    }
    let wrong_file = c::same_source(&left, &right, "a/x".as_bytes(), "b/x".as_bytes(), &base, &current, false);
    expect_verifier_true(!wrong_file);
}

} // verus!
