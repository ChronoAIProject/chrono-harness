# Native generated-unit adoption evidence

This document records live GitHub Actions evidence for the generated
`chrono-github-units/v1` projection. It closes the bounded adoption question
for the public Go, TypeScript and mixed example hosts: each host ran its
generated collection workflow and every declared unit on both a
`pull_request` event and a `push` event. All runs in the Recorded runs table completed with
`success` on 2026-09-29 UTC and used the listed candidate commit.

The beta.14 evidence in the Recorded runs table is about generated unit workflows
and their collection. It does not establish complete effective-input closure, `chrono-github-ci/v3` host
adoption, universal local/CI parity, or full seven-judge host activation.

## Recorded runs

| Host | Candidate | Declared units | Pull request | Push |
| --- | --- | --- | --- | --- |
| [Go](https://github.com/ChronoAIProject/chrono-harness-examples-go) | `ebeb022df573cb907ff1659c29844722a9f569f5` | `harness`, `rates` | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624618475), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624618481), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624618430) | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624589716), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624589723), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36624589731) |
| [TypeScript](https://github.com/ChronoAIProject/chrono-harness-examples-ts) | `8b55a765dfdd8d3ff8b4ed0f82df69e1ba36e6bb` | `harness`, `labels` | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624896292), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624896319), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624896450) | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624854511), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624854173), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-ts/actions/runs/36624854228) |
| [Mixed](https://github.com/ChronoAIProject/chrono-harness-examples-mix) | `5137df028de4b1bf21d21825fb67483fcd7b78cf` | `harness`, `labels`, `normalize`, `rates` | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624900081), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624899959), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624900164), [normalize](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624900085), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624899957) | [collection](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853951), [harness](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853836), [labels](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853948), [normalize](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853834), [rates](https://github.com/ChronoAIProject/chrono-harness-examples-mix/actions/runs/36624853905) |

The final mixed `rates` link above is the push run at
`36624853905`.

## Command identity

The unit logs show `chrono-ci prepare` selecting the registered unit. The
prepared workflow then records and invokes the same canonical check used by a
local run:

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE \
  --candidate FULL_CANDIDATE \
  --unit UNIT
```

Each collection workflow gathers the pinned unit attempts and passes the same
entry point with the explicit collection selector:

```sh
.chrono-harness/bin/chrono-harness check \
  --config .chrono-harness/ci/check.json \
  --base FULL_BASE \
  --candidate FULL_CANDIDATE \
  --collect .chrono-harness/state/collection/manifest.json
```

The run IDs and candidate hashes are retained so later updates can replace
this evidence with newer observations without treating an old successful run
as a current workflow guarantee.

## Native Git policy adoption

The original successful unit-workflow runs above retain their beta.14 scope.
Go [PR #18](https://github.com/ChronoAIProject/chrono-harness-examples-go/pull/18)
adopts public beta.18 and explicitly opts both scoped checks and provider-v3
event preparation into the same platform selector. It registers each native
Git executable, expected version, environment and guarded global/include files.
The six selected tools, SDK profiles and three independent workflow bytes are
preserved. This is scoped governance; full native activation, effective-input
closure and deterministic local/CI parity remain unfinished.

The accepted candidate is `ada0b39e2adfe4adf6cd51440317f3944c9abb98`, based on
`9b008540944976258d2ec61857cfb34b17474d8a`. The merged dev commit
`6cf641fbd1e7828d922262884b2d735de3235ed0` has the same source tree. Local macOS
canonical rates/harness checks execute four operations, including 10 bootstrap
tests and the registered Go build/test. Their original reports measure
50,296,479 and 50,284,966 bytes; canonical collection passes. Global-config
drift and appearance of the declared-absent include each fail before business
operations, and restoration passes an empty DELTA with zero operations.

| Native event | Harness unit | Rates unit | Original-report collection |
| --- | --- | --- | --- |
| integration push | [36668775068](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668775068) | [36668775175](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668775175) | [36668775063](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668775063) |
| pull request | [36668778188](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668778188) | [36668777932](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668777932) | [36668778051](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36668778051) |
| dev push | [36669165203](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36669165203) | [36669165232](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36669165232) | [36669165186](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36669165186) |

All runs in this table succeeded. Push and PR original report hashes, fixed
endpoints/configuration, producer stream hashes and executable identities were
verified. Their provider and check reports select the registered Linux policy,
Git digest and observed version 2.55.0. Rates reports measure 50,197,605 and
50,196,779 bytes respectively. The host explicitly bounds judge stdout at 16 MiB,
collection manifests at 1 MiB and each original report at 64 MiB. Different
platform policies do not establish equivalent effective inputs or verdict parity.

The original beta.17 candidate `e508cb05e5079155fb6a46e9dcf608534d75ba4a`
passed both native units but failed collection
([push](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36663954779),
[PR](https://github.com/ChronoAIProject/chrono-harness-examples-go/actions/runs/36664001392))
on 149,387,940- and 149,386,115-byte rates reports. Beta.18 compact scoped-v3
serialization fixes this overflow while retaining original process bytes and
report verification. The candidate/configuration change requires fresh reports;
old failures and original report bytes are not rewritten. This adoption does not
increase the 64 MiB report-read bound. Other example dev branches still pin beta.14.
