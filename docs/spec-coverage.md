# Current SPEC coverage

This is a maintained coverage map, not a run log or a second policy. Full SPEC remains the objective. `implemented` means the named contract has a real producer and behavior test; it does not mean this host is activated or delivered. `partial` retains the uncovered obligation. All full host registries remain proposed, with incomplete input closure and missing executable bindings. Registration/filemap success on a bounded host is not seven-judge success. Local checks do not certify native CI or landing.

Test sources: **R** = `crates/runner-tests/tests/protocol_v1.rs`; **G** = `crates/judge-registration-tests/tests/registration_contract.rs`; **L** = `crates/runner-tests/tests/cli_contract.rs`; **C** = `crates/judge-ci-tests/tests/behavior.rs`; **F** = `crates/judge-filemap-tests/tests/impact.rs`; **P** = `crates/judge-filemap-tests/tests/consumer.rs`; instruction and CI generator tests live in their dedicated test projects. Tests in G invoke the built runner and separately built registration executable against real synthetic Git commits, including real host registry material. Production build prerequisites are explicit in the CI binding. P runs actual runner → registration → filemap subprocesses on committed bounded hosts with absent historical binaries and exact report source pointers. F checks witnesses against independently specified endpoint edges. No peer/independent review evidence is claimed.

| SPEC section | State | Producer and direct evidence | Remaining obligation |
| --- | --- | --- | --- |
| 1 authority/boundary | partial | runner `full.rs` declared external DAG; R `dag_*` | remaining judge policies and lifecycle |
| 2 independent projects/adoption | partial | independent manifests, locks, bootstrap registration build | generic full bootstrap provenance/activation |
| 3 five registries | partial | registration `schema.rs`, G `strict_five_schemas_*`, `real_host_pseudo_script_*` | complete Cargo/SDK closure; bounded pairing/routes/migration/snapshots implemented |
| 4 fixed snapshots/context/CI | partial | runner `facts.rs`, registration `lib.rs`; G `context_and_config_identity_mismatch`, `ignored_untracked_staged_and_wrong_checkout_fail` | full native CI/parity evidence, full Cargo/SDK closure beyond retained bounded inputs |
| 5 DELTA graph | partial | filemap typed union/records/closure and CI adapter; F all tests, P `actual_runner_registration_filemap_*` | complete host input closure and native activation; bounded execution and retained inputs are implemented |
| 6 evolution/removal | partial | affected reference checks; G `changed_references_*` | workflow certification beyond the finite implemented decoder and required replacement execution |
| 7 protocol | implemented within ordinary two-commit v1 | runner `wire.rs`, `full.rs`, shared process engine; R all tests | no full root/initial success mode; platform evidence limited to local Unix |
| 8 seven judges/mixed | partial | registration and filemap; G/F/P external pass/negative findings | workflow; routes/projects/cost/mixed bounded contracts implemented |
| 9 report/cost | partial | runner `full.rs`; G `full_report_*`: all required fields, executable list with observed/configured distinction, findings and named-output sources, null/unresolved reasons, pass/warn/fail/error exits and stored/stdout agreement | complete toolchain/input coverage and optional measurements; declared cost reporting is implemented; bounded routes tools and projects test evidence are supplied; unknown or conflicting results do not establish complete governance |
| 10 lifecycle | pending | caller owns current delivery; no runtime producer | fresh dev/integration, stale rebuild, provenance and landing orchestration |
| 11 script/plugin | partial | direct argv external transport; R real Python child fixtures | plugin/full external closure; real script pairing and routes have bounded full-chain evidence |
| 12 bootstrap/initial | partial | registered bootstrap builds and installs registration/filemap/routes/projects | full source/toolchain provenance, activation migration and explicit initial transport |
| 13 acceptance | partial | row map below | all pending row obligations remain |
| 14 boundaries | implemented documentation | README/SPEC/this map | update as later increments land |
| 15 generic reference experience | partial | existing fixed-source references; no reference repo modifications | apply remaining generic workflow lessons |
| 16 instructions | implemented existing scope | instructions and dedicated tests, current docs/instructions.md | existing documented platform/crash/concurrency limitations remain |

Rows below follow §13 in order; scoped evidence is explicitly limited to that profile.

| §13 row/scenario | State and direct evidence | Remaining full contract |
| --- | --- | --- |
| 1 registered source M selects tests | implemented bounded: F `typed_content_mode_binary_*`, P impact, projects consumer real execution | full native activation |
| 2 unregistered A / ignored dirt | implemented: G `changed_unregistered_fails_and_explicit_registration_repairs`, `ignored_untracked_staged_and_wrong_checkout_fail` | downstream test execution separate |
| 3 source plus membership | implemented bounded: registration repair, FILEMAP selection, mixed ordinary-member/edge controls | full native activation |
| 4 deleted file uses base edges/costs | partial: F `base_only_test_edge_*`, P removed-test facts, scoped C | workflow retirement certification; old declared costs are retained |
| 5 rename D+A | partial: G D+A facts; F `typed_content_mode_binary_add_modify_delete_and_rename` | workflow retirement certification; old declared costs are retained |
| 6 changed/deleted edge seeds | implemented bounded: F `file_record_field_and_edge_*`, `file_edge_only_*`, shared scoped execution | full native activation |
| 7 legal retirement | partial: preserved mappings, old obligations and required successful replacement | workflow certification |
| 8 required test removed | partial: F/P old-only facts; projects consumer rejects missing replacement, executes mapped old methods | workflow retirement certification |
| 9 nonunique/missing test pair | implemented bounded: projects and real consumer negatives | full native activation |
| 10 manifest undeclared dependency | implemented bounded: semantic TOML path dependencies and missing-edge rejection | registry dependency/Cargo/SDK closure |
| 11 unknown input closure | partial: G `proposed_and_incomplete_never_receive_governance_success` | complete Cargo/SDK closure; retained bounded external/tool inputs implemented, no discovery guarantee |
| 12 README no all-tests | implemented bounded: F selection, C and projects consumer real nonexecution with unrelated historical defects | full native activation |
| 13 independent script tests | implemented bounded: real full-chain script pair | full native activation |
| 14 duplicate operation | implemented: routes preflight and full consumer | none in tested boundary |
| 15 bypass/missing operation evidence | implemented bounded: actual invocation and receipt comparison | does not monitor arbitrary shell actions |
| 16 mixed warning with integration | partial: mixed real-chain warning, zero exit, both groups and bound costs | workflow integration certification |
| 17 engine stability | pending | workflow |
| 18 policy requires integration | pending | workflow |
| 19 changed integration binding | pending | workflow |
| 20 stale branch | pending | workflow/lifecycle |
| 21 threshold equality | pending | workflow with injected context time |
| 22 same local/CI command | partial: existing scoped CI generator | full native events and exact inputs |
| 23 same complete deterministic inputs | pending | deterministic evidence/parity comparison |
| 24 parity unestablished | implemented field: full runner report; G external host | unresolved closure remains nonzero |
| 25 staged/unstaged/untracked | implemented: G dirt cases | no hidden dirty mode |
| 26 missing history | partial: G `missing_oid_and_symbolic_endpoint_are_errors`; registration fork object check | complete workflow ancestry |
| 27 empty/invalid stdout | implemented: R `malformed_stdout_crash_and_bounds`, R/L embedded-invalid-UTF-8 subprocess/CLI cases and valid U+FFFD controls | — |
| 28 crash/timeout/digest | implemented: R bounds and prelaunch digest tests | resource guard is infrastructure, not functional verdict |
| 29 unknown costs | implemented: cost consumer warns with zero exit, preserves null and known coordinates; no-Delta retained-input case | full host activation |
| 30 candidate migration validator | partial: candidate-only finite decoder, retained old bytes/definitions, real ci.verify drift/restoration consumer | workflow migration/retirement certification |
| 31 no DELTA | partial: G no-delta/dirt behavior | full input/workflow checks |
| 32 initial root | partial: full CLI rejects absent base, no green | explicit initial report mode |
| 33 check CLI | implemented bounded transport/registration plus existing slice | whole configured host remains nonzero until obligations implemented |
| 34 unknown/malformed CLI | implemented: L `unknown_and_malformed_commands_are_errors` | — |
| 35 spec status | implemented: L `status_help_and_version_are_information_only` | — |
| 36 help/version | implemented: L same test | — |

The registered execution increment adds routes/projects, FILEMAP v2, retained endpoint inputs and the finite chrono-ci-check/v1 / FILEMAP v1 decoder. See [the execution contract and exact boundary](execution.md). Dedicated routes tests check ordered plans, actual argv/environment/tool bindings, receipt tampering, PATH shadow and byte replacement. Dedicated projects tests run the actual runner/registration/filemap/routes/projects chain on committed project/script hosts, cover exclusive pairs, TOML edges, retained input failures, real exits/effects, blocked dependents, docs nonexecution and mapped replacements. Its migration consumer uses real old repository registrations and actual ci.verify, including workflow drift and restoration. Existing test identities remain; the historical pseudo-script rejection reads the fixed old tree. Maintained regressions also cover retained inherited-environment changes through the full chain (including absent/empty, overridden and disconnected controls), registered intermediate workspace rejection before operations, both protocols' embedded invalid UTF-8, arbitrary operation bytes, and migration version failure diagnostics. The host interpreter binding is explicit macOS data; the failed original native run and the verified repaired native results are recorded in [CI documentation](ci.md).

Workflow certification and all other remaining obligations stay active. Project fixtures validate declared method execution and TOML consistency, not Cargo/SDK completeness. Single-worker checks do not certify independent review, native CI or landing.

This increment adds mixed classification and adopted registrations together. **Mixed policy/product change warning:** the project pair, semantic comparisons, bootstrap binding and execution plan expand the validation surface. Costs remain unmeasured. Dedicated tests validate the mixed report and shared cost consumer; host activation still requires the outstanding governance and input obligations.

The cost judge and its dedicated tests implement the [declared cost contract](costs.md): distinct endpoint values and source pointers, old-only costs, known/unknown coordinates, identity deduplication, affected-project members, declaration-only changes and docs locality. Real subprocess consumers verify pass/warn exits, missing impact failure, source fidelity and retained-environment impact with empty Git DELTA. Measurements are explicitly absent; no additive resource total or full-governance claim is made.

The mixed judge implements [explicit rule/product classification](mixed.md). Dedicated behavior tests cover normal member/edge additions, real action changes, endpoint surfaces, array identity and ordering, historical patterns, named JSON and malformed inputs. Real subprocess tests check warnings, pass/error exits, exact cost and source fidelity, rule-only outputs, missing predecessors, stale binding rejection and strict UTF-8. Workflow remains responsible for stability and integration certification; this increment supplies its rule-change input.
