# Verus ownership and migration

Every normative repository SPEC and all repository decision and state logic must
use Verus. This includes parsers, judges, selection, error handling, lifecycle,
generators, bootstrap, tests and workflow production. Existing Rust/Python is an
outstanding migration obligation. Markdown explains the contract and current
gaps; the final authoritative specifications belong beside the executable Verus
implementation. The current SPEC has not yet been fully translated.

The user permits explicitly listed external interfaces as trusted assumptions.
The host list is `.chrono-harness/verus.json`. OS and external executable/library
interfaces do not exempt repository wrappers, decisions, parsing policy or state
maintenance. Concrete interface contracts and their implementations must still
be connected. Repository functions cannot acquire an exemption by being marked
external. Pure roots use Verus's `--no-cheating` and
`--no-external-by-default`, without function filters or disabled checks.

FILEMAP owns each file's `verification: {kind, roots}`. The three explicit roles
are `program`, `specification`, and `data`; the executor never infers them from
names or source scans. Program/specification roots must be nonempty, registered
files and exact registered Verus invocations included in execution plans. Empty
roots retain a concrete migration gap. Every project/script, including tests,
must declare `language: verus` and a verification action. Changing a label does
not establish a proof or source correspondence.

The host opts in through `policy.verus` in its existing scoped check profile.
The ordinary entry remains `.chrono-harness/bin/chrono-harness check` locally
and in CI. Coverage findings are added after independent selected operations;
they do not erase the original execution results. Unit checks and collection
both evaluate coverage. Removing an adopted requirement fails explicitly.

The current adapter checks declarations and canonical routes. For actual Verus
operations it also decodes retained original stdout and requires a successful
whole-crate result with positive proof count, zero errors and the selected Verus
version/commit. Execution identity, argv, tool binding and original process
success remain owned by routes/projects. Collection still consumes original unit
reports. This increment does not prove that every declared root includes every
claimed source or expresses all natural-language requirements. Compiler source
coverage, complete clause correspondence, independent-unit proof transport and
the admission adapter's own Verus migration remain required before claiming full
verification. Its FILEMAP record deliberately has empty roots.

`crates/judge-ci/src/verus_policy.rs` is the first executable Verus owner. Cargo
uses the pinned `verus_builtin_macros` erasure of that same source; admission calls
its executable predicate. Verus proves that `first_unverified` returns exactly
the earliest false obligation, or none precisely when all obligations hold.
`complete` additionally rejects an empty inventory. `missing_obligations` returns
every false obligation exactly once, in increasing inventory order; its loop
proves soundness, completeness and termination. The actual adapter uses these
indices when reporting missing identities. Boundary/example
functions in the dedicated test project are verified by Verus as well; Cargo's
ordinary tests do not replace this verification. The same source also proves the full nonzero lowercase hexadecimal OID predicate
(40 or 64 bytes) and the nonempty ASCII identifier predicate. Actual `full_oid`
and `id` callers use these predicates through byte slices, without allocating a
copy. The dedicated examples cover zero/empty/uppercase rejection, both OID
lengths, identifier ASCII range endpoints and invalid separator/non-ASCII bytes.

The same owner now contains the executable compiler-result admission decision.
It accepts exactly a successful, error-free whole-crate result with a positive
verified count, normal exit and exact nonempty version/commit bytes. Every field
is optional at the external decoding boundary: absent or wrongly typed values
cannot become success. The byte comparison proves equality including length;
the typed decision and missing-obligation collector are called by the actual
adapter. Verified examples include zero proofs, partial verification, errors,
abnormal/missing exit, different identities and missing fields. These proofs do
not establish JSON decoding, original-report transport, toolchain package
identity, root/source correspondence, the graph algorithm or other program logic.

The selected external verifier is official Verus
`0.2026.04.24.f8e1704`, commit
`f8e17045c4ac17a02e566b8997070f2eecc255be`, using the existing Rust 1.95.0.
The macro dependency is fixed at `0.0.0-2026-04-20-1748` in the affected independent
lockfiles. The host tool is `.chrono-harness/bin/verus/verus`.
The official macOS arm64 archive is
`https://github.com/verus-lang/verus/releases/download/release%2F0.2026.04.24.f8e1704/verus-0.2026.04.24.f8e1704-arm64-macos.zip`,
SHA-256 `023e0a6e67bb9927f43a8757ebdec318b50cbfe1fec4a1c320c53d5fd71008e7`.
The corresponding Linux x86 archive digest is
`36b3268ed3a70d948d0d7cb59a5397c54c52e519fc4a644311dadec02d240ce9`.
Automatic package acquisition, complete verifier/toolchain input binding,
shared-startup transport and native acceptance remain migration work; a missing
tool fails and must not fall back to Cargo or an unrelated verifier.

Product and host policy change together: the product accepts explicit file
verification roles and the scoped coverage policy, and this host adopts the
stronger Verus contract. Previously its Rust/Python declarations could pass
ordinary scoped checks; they now produce coverage failures. The added dependency,
two registered verification operations and explicit file-role inventory are the
validation cost. Full build/CI/storage/lifecycle costs are unknown. This warning
adds no approval gate and waives no required check.

Proof/debug originals have the current migration, regression and review consumers.
They belong to the candidate's registered `.chrono-harness/state/` directory,
retained until those consumers and acceptance/debug obligations end, then retired
through the existing lifecycle owner. Downloads outside the checkout are task
temporary inputs; they are not a permanent evidence archive. The failed original
worktree creation is preserved at the original coordinator. A successful local
proof does not establish native CI, landing, release or completion of this goal.
