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
files, uniquely bound to exact registered Verus invocations included in execution plans. Empty
roots retain a concrete migration gap. Every project/script, including tests,
must declare `language: verus` and a verification action. Changing a label does
not establish a proof or source correspondence.

The host opts in through `policy.verus` in its existing scoped check profile.
The ordinary entry remains `.chrono-harness/bin/chrono-harness check` locally
and in CI. Coverage findings are added after independent selected operations;
they do not erase the original execution results. Unit checks and collection
both evaluate coverage. Removing an adopted requirement fails explicitly.

The current adapter checks declarations and canonical routes. For actual Verus
operations it decodes retained original stdout and requires a successful
whole-crate result with positive proof count, zero errors and the selected Verus
version/commit. It also compares the local compiler dependency output with
explicit regular FILEMAP entries and immutable candidate blobs, including all
files claiming that root. Execution identity, argv, tool binding and original
process success remain owned by routes/projects. Collection re-admits original
compiler results and acquired dependency bytes with the same candidate-source
comparison, after validating each original execution receipt. Complete clause
correspondence and the admission adapter's own Verus migration
remain required before claiming full verification. Its FILEMAP record
deliberately has empty roots.

`crates/judge-ci/src/verus_policy.rs` is the first executable Verus owner. Cargo
uses the pinned `verus_builtin_macros` erasure of that same source; admission calls
its executable logic. `Ledger` owns a private sequence of identity/decision
records. Its constructor establishes the empty state, and `record` proves that
appending preserves every preceding record. Consuming `finish` proves the exact
ordered sequence of rejected identities and the original count; duplicate and
empty identities remain distinct records. Coverage requires a nonempty inventory
and every decision to be accepted. `first_missing` safely returns the first
rejected identity, or none exactly when the result has no missing records. The
actual adapter uses this ledger instead of parallel identity/decision arrays or
unchecked indexing. Boundary/example
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
the typed decision and coverage ledger are called by the actual
adapter. Verified examples include zero proofs, partial verification, errors,
abnormal/missing exit, different identities and missing fields. These proofs do
not establish JSON decoding, original-report transport, toolchain package
identity, root/source correspondence, the graph algorithm or other program logic.

Requirements-field validation and fixed verifier-argument admission also use the
same proved owner. Text membership is exact and works for Unicode and empty
values; text comparison reuses byte equality and proves the UTF-8 correspondence.
The requirements predicate requires the current schema, a selected registered
tool, nonempty version/commit and at least one interface, with every interface
name and assumption nonempty. The argument predicate requires exactly the
registered checking flags and output switch; extra arguments, weakened checking
flags and function selection are rejected. The actual adapter borrows strings
into these calls. Constructing borrowed inventories from library values and
validating their registered root/output paths remain unproved adapter obligations.
The plan membership decision uses the proved root-route classifier below. These
predicates do not certify the adequacy of an external interface assumption.

The same owner specifies and implements unordered result-ID membership and the
expected inventory composed from `ci.inventory`, registered operations,
not-required tests and, when adopted, `ci.verus-coverage`. Character values own
identity; string reference identity and ordering do not affect membership. The
collector calls this proved predicate. Adopted coverage additionally requires a
passed result with no process exit code; missing, unpassed or exit-bearing
coverage fails. Shared response-protocol validation still
owns duplicate rejection. Verified examples cover adopted and unadopted coverage,
missing inventory/coverage, empty inventories, Unicode and duplicate membership.
The collector also retains digest-bound report references, receipt digests and
original-response pointers for validated unit execution rows in
`collected_executions`, avoiding another copy of process output. Its local
`executed` list stays empty because it
runs no business operations. These rows retain original execution addresses;
adopted Verus operations additionally require exactly one compiler-evidence row
per original operation. Missing, duplicate and extra evidence is rejected. The
collector rechecks the original whole-crate compiler result, candidate identity,
root, both artifact labels, source inventory and dependency checksums. It reads
the already acquired original dependency bytes and fixed Git candidate blobs,
without depending on local compiler output paths or executing the compiler.
Accepted compiler/source rows enter the collection's coverage ledger. Native and
complete proof evidence for every declared root remain outstanding.

The same executable Verus owner now proves unique root-route lookup and DELTA
root coverage. A changed program/specification file, its changed FILEMAP record,
or a changed root or root record it claims,
requires that unique route in the global explicit execution closure. Every
locally selected root additionally needs an admitted original compiler result and
candidate-source comparison. A unit may defer a globally selected root assigned
elsewhere; collection uses the global obligation and consumes the verified unit
originals. Multiple successful originals for a shared operation are allowed; a
rejected original cannot be hidden by another success. Missing and ambiguous
routes, missing global selection and absent or rejected required proof fail.
Unrelated proof failures retain their own original findings. Identity is text
value, including Unicode, rather than string reference identity.

The adapter also calls its proved root-route classifier. Given the explicit
test/operation plans and global/local selected test inventories, it returns a
route exactly when some plan contains the operation. Its two selection flags
equal the existence of a selected test whose plan contains that operation. Empty
plans, unselected registered operations, shared operations, operations assigned
to another unit, Unicode text values and duplicate memberships have independent
verified examples. Supplying an operation name as a selected test does not select
its differently named owner. The classifier replaces the adapter's three
independent plan/set searches; it adds no execution or dependency inference.
Decoding and constructing these input inventories, canonical-route filtering and
the rest of the adapter remain migration obligations. Additional total runtime
and allocation costs have not been measured.

Root declarations and unit verifier membership now use the same proved owner.
A declaration must supply a nonempty list with every entry a text value naming
both an explicitly registered file and exactly one canonical root route. Repeated
mentions of a valid root remain valid; ambiguous routes fail even when their
selection flags differ. A unit has a verifier exactly when one of its own action
operations matches a registered route operation. Missing or malformed action
operations cannot establish membership, and a root filename cannot substitute
for an operation. Independent verified examples cover absent/empty/malformed
lists, each missing inventory, ambiguity, Unicode value identity, duplicates and
unrelated actions. Declaration validity does not establish selected execution or
proof admission. The adapter calls these implementations after borrowing the
explicit JSON inventories; that conversion and the remaining adapter still have
no complete proof. No FILEMAP gap is cleared by these partial migrations.

**Product and coverage policy change together:** declaring a root no longer
satisfies affected-root coverage without its explicit selected proof consumer.
This gate reports the missing relationship and does not select extra tests or
infer dependencies. Dedicated behavior cases exercise the missing-edge failure,
explicit repair, split unit proofs, collection and ambiguous registration. A
pre-change canonical run observed the missing-edge case wrongly pass. Another
observed a role/root-only FILEMAP change wrongly pass without any source-byte
change. The inventory now supplies the same explicit file-record delta to graph
seeds and proof obligations; `changed_paths` retains its Git byte-delta meaning.
Dedicated unit and collection cases require proof for both role and root changes,
with passing controls using an explicit consumer and admitted originals. The same
registered commands remain current. Additional comparisons are bounded by the
registered roots and proof observations; total runtime/storage cost is unmeasured.
This predicate does not prove selection-input construction, JSON decoding, the
remaining adapter, complete SPEC correspondence or unchanged-root proof reuse.

This collection change retains the existing report budget and artifact reader.
Its additional work compares the acquired dependency bytes with the required
fixed candidate blobs for each original proof operation; it does not select
tests from compiler dependencies. The dedicated Rust external-interface fixture
tests rehashed semantic mutations and missing local outputs. Its synthetic
compiler results are test inputs, never repository proof evidence. Runtime,
memory and storage costs beyond the bounded input sizes remain unmeasured.

The registered verification operations additionally require `--compile` and
explicit `--emit=link=OUTPUT,dep-info=OUTPUT.d`, so successful verification is followed by Rust compilation
and compiler-produced dependency output. They also require
`-Zchecksum-hash-algorithm=sha256`; omitting the flag or selecting SHA-1 is rejected
by the same proved canonical-argument predicate. The pinned compiler's emitted
source checksums and unnormalized byte lengths have been compared with the
actual source bytes. This observation does not bind a later report or candidate.
Verification success alone is not a claim that these artifacts were produced or
consumed. The local adapter now reads both bounded regular output files after
the compiler result is admitted. Both exact output byte sequences now pass to the
existing addressed-original owner. Source observation v2 contains their original
addresses, digests and byte lengths instead of embedding dependency bytes.
The runner binds these explicit originals to unit reports as well as collection
reports. The shared typed unit reader, also used by native gather, acquires both
originals with the existing transport mapping and shared metadata/stream/artifact
budget. Its digest, nonempty length, expected length and remaining-budget decision
calls the same proved Verus predicate; hashing and IO remain external interfaces.
The collector now re-admits collected source observations against candidate
blobs; decoding, capture, transport and adapter implementations remain unproved.
Native and complete source-provenance acceptance remain incomplete.

The same Verus owner now defines and implements dependency-list byte decoding.
Its contract follows the pinned Rust 1.95.0
[`escape_dep_filename` and `write_out_deps`](https://github.com/rust-lang/rust/blob/1.95.0/compiler/rustc_interface/src/passes.rs):
ASCII spaces inside a filename are escaped with a backslash; a single unescaped
space separates filenames. The proved scanner preserves input order, duplicate
paths, UTF-8 bytes and relative components such as `../`. It accepts a nonempty
list of nonempty paths, rejects ASCII controls, literal backslashes and unknown
escapes, and proves that re-encoding accepted paths reproduces the exact input.
Its executable result also matches the specification's complete success/error
semantics. Verified examples cover those success, error and boundary cases.
This is a bounded filename grammar for the current compiler observations. The decoder alone
does not clear any file's migration obligation or establish SPEC correspondence.
Additional proof/build/storage costs remain unmeasured.

`crates/judge-ci/src/verus_sources.rs`, included by the same proof root, now
specifies and implements first-target dependency extraction and exact dependency
file content comparison. Extraction requires the registered `OUTPUT.d` target,
a nonempty decoded path list and a terminating LF. Content comparison requires
a nonempty expected source inventory, both exact ordered target rules, all phony
source rules and one matching checksum line per source in order. It allows blank
lines and `# env-dep:` comments before the checksum block, then only blank lines
after that block. Truncated lines and missing, altered or extra checksum content
fail. Each checksum comparison binds the supplied path, digest bytes and byte
length, including escaped filename spaces. Canonical decimal serialization of
the expected `u64` avoids parsing arbitrary integer text; proved examples include
zero, leading-zero rejection and the maximum `u64` boundary.

The same owner proves lexical POSIX relative-path normalization. Its byte scanner
maintains a component stack, removes `.` and repeated separators, and cancels `..`
only when a preceding component exists. Absolute paths, root escapes, an empty
normalized result, controls and backslashes fail. Spaces and UTF-8 bytes are
preserved. This does not resolve filesystem aliases or authorize symlinks.
Independent examples include nested cancellation, repeated/trailing separators,
Unicode, root escapes and empty results.

It also proves exact 64-byte lowercase hexadecimal SHA-256 text validation,
including valid all-zero text. The actual collection digest predicate now calls
this same executable source. Verified examples include short/long text, uppercase
and the hexadecimal range boundary. Hash computation and source-byte binding
remain separate obligations.

The same source proves observation identity matching: the reported candidate and
root must equal the fixed candidate and registered root, and the library and
dependency labels must equal the registered output and its `.d` suffix. This
predicate owns equality only; candidate eligibility and artifact acquisition
remain separate checks. Verified examples include changed identities, missing
suffixes and the empty equality boundary. Collection calls this executable
predicate before source re-admission.

The same owner now proves paired source-metadata admission. Every compiler path
must normalize to its corresponding candidate path and match its valid SHA-256
text and byte length. Candidate paths must be distinct; the nonempty inventory
must contain the exact root and every explicitly claimed source. Empty or unequal
inventories, duplicate normalized identities, changed digests/lengths and missing
roots/claims fail. Independent examples specify those success and rejection cases.

The actual adapter obtains only the needed candidate entries through explicit
FILEMAP/tree lookups and immutable Git blobs, then calls the proved metadata and
complete-content predicates. It does not use compiler observations to select
checks. Dependency bytes and source observations remain in the retained original
response through explicit addressed dependency and library originals.
JSON decoding, lookup/collection construction, hashing-interface correspondence,
artifact capture/transport and the remaining adapter decisions still require
proof. The complete-content examples exercise an independent
byte fixture, missing checksum content and a missing final newline. The source
adapter's existing empty-root migration obligation remains intact.

Product/policy change: both registered proof operations now request compilation
and dependency output, and the proved argument predicate requires those flags.
The proved argument builder and admission bind both emitted filenames to the
registered output. This avoids Rust's filename adaptation for implicit multiple
outputs. This preserves whole-crate checking and rejects extra or weakened arguments.
Additional compilation/storage cost has not yet been measured. Original process
exit and artifact evidence must be checked before accepting this stronger route.

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
