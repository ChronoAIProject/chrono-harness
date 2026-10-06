# Explicit cache keys

`chrono-cache` is an independent Rust product with its own `cache-tests` project,
CI unit and central release asset. It does not compile with the CI generator.

`chrono-cache plan --host-root H --config .chrono-harness/cache.json
--consumer ID` prepares keys for one explicitly registered consumer. This is an
internal provider entry; the daily check remains `chrono-harness check`.

The current implementation prepares a plan only. It does not restore or save a
cache, invoke a build, verify a restored executable or report a cache hit. The
generated check and release workflows have not yet adopted persistent caching.
SPEC §4.2's transport, lifecycle protection and native
cold/warm/failure/concurrent acceptance remain required work.

This host explicitly sets `CARGO_INCREMENTAL=1` for its canonical check actions.
Both bootstrap registrations and the v5 release recipe set
`rust_incremental: true`; their existing build invocations receive the matching
environment value, including release builds and Rust verification units. The
release recipe's explicit `rust_toolchain` selection keeps that setting scoped
to Rust consumers. Non-Rust units retain their own environment.

Bootstrap and release receipts record the value passed to children. Release
import and collection compare it with the registered choice and include it in
the producer toolchain fingerprint. Omitted settings preserve the previous
environment behavior; malformed settings fail before launching build tools.
These observations do not establish cache transport, actual compiler reuse,
complete build inputs, or native acceptance. Rust profile, flags, compiler and
SDK identities still belong in the persistent cache compatibility contract.

The `chrono-cache/v1` registration belongs to the host's `.chrono-harness/`
directory. It explicitly declares:

- A namespace and an `artifact_registry` path. Host cache artifacts reference
  exact paths in that registry's `artifacts` array; each must have the same owner,
  `tracked: false`, and must not be execution evidence. An explicitly external
  dependency cache may instead declare an absolute path. Other artifacts remain
  relative to the host. No directories or languages are discovered.
- Named inputs: `file` has a literal path and expected `presence` (`present` or
  `absent`); `environment` has a variable name and expected presence; `literal`
  carries a JSON declaration. Present files are hashed from their actual bytes;
  environment observations retain a digest and byte length without the value.
  Literal declarations are not observations of actual toolchains or platforms.
- Named artifacts with `owner`, `path`, `kind` (`dependencies`, `compilation`, or
  `executable-candidate`) and explicit `external` boolean.
- Named caches with `owner`, registered `producer` operation ID, `consumers`,
  artifact IDs, disjoint `compatibility_inputs` and `source_inputs`, and `restore`
  (`exact` or `compatible`). Each list is nonempty and has unique members.

A consumer selects only caches naming it. Missing consumers fail instead of
selecting all caches. Only the selected input closure is read; an unavailable
input belonging solely to another consumer does not affect this plan. Registration
shape and input/artifact references are checked without probing unrelated inputs.
Producer and consumer IDs are declarations here; the execution provider must
resolve them against its registered operations before restoring or building.

The compatibility digest binds the namespace, cache identity, owner, producer,
consumer set, restore policy, artifact declarations and their selected registry records, plus observations of
the declared compatibility inputs. The source digest binds the declared source
input observations. Keys are `chrono-v1-<compatibility digest>-<source digest>`.
Reordering input IDs does not change the digest. Changes to unrelated cache
declarations or unrelated files do not invalidate the selected cache.

`compatible` supplies exactly one restore prefix ending after the compatibility
digest. Source changes can therefore find older compilation data in the same
compatibility domain. Compiler, target, profile, flags, SDK, build-script,
configuration and environment identities belong in that domain whenever
applicable; the host must register them explicitly. The planner does not infer
these dependencies or prove that this list is complete. Lock files and manifests
must likewise be assigned according to their actual compatibility role.

`executable-candidate` artifacts permit exact restoration only. Every plan entry
still requires its registered build producer; this interface never authorizes
direct execution of a restored historical judge. Current executable verification
and the selected judgments/tests remain obligations of the consuming pipeline.

Literal paths reject traversal, globs and symlink components. Selected outputs
cannot overlap one another, contain Git metadata or harness execution state, or
contain one of their declared file inputs. This is a finite path/registration
check, not a claim that arbitrary contents of a cache have been proved safe or
that all rebuild inputs are known. Restoring and saving must additionally hold
the adopted lifecycle protection until joined producers and saving are complete.

The returned `chrono-cache-plan/v1` includes selected entries and input
observations. Its status is `prepared-unrestored`, execution is `not-started`, and
`input_completeness_proven` is false. These are plan facts, not passing judgments,
cache hit states, successful builds or parity evidence. Cache backend results,
failure chains and original reports must be supplied by their actual producers.
