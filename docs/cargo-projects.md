# Optional Cargo project judge

`judge-cargo` / `judge-cargo-tests` is an independent production/test pair. Generic
registration and projects have no dependency on it or on TOML. A host adopts the
adapter by registering its binary, arguments, policy input and FILEMAP edges. A
Go/TS/script host has no Cargo policy obligation merely because Cargo-like files
or directory names exist.

The policy must be below the host `.chrono-harness/` and registered in FILEMAP:

```json
{
  "schema": "chrono-cargo-projects/v1",
  "projects": [
    {
      "project": "library",
      "manifest": "units/library/Cargo.toml",
      "lockfile": "units/library/Cargo.lock",
      "output": "outputs/library/",
      "ancestor_manifests": []
    }
  ]
}
```

Each row explicitly maps a registered project to owned manifest/lock inputs and
an already registered generated-output directory. Generic project fields are not
consulted for Cargo meaning. The output path is not inferred from a source root.
`ancestor_manifests` explicitly lists the registered Cargo ancestors that the
adapter must inspect. It verifies this inventory against registered ancestor
paths; a missing declaration is an error, never an automatically added input.

The bounded check rejects workspace aggregation/association, workspace dependency
inheritance, missing manifest/lock ownership, missing output ownership and path
dependencies without a unique declared Cargo project and explicit FILEMAP compile
edge. Normal, dev, build and target-conditioned TOML dependencies are checked.
Registry/git dependencies still fail as unresolved retained input closure. This
adapter does not yet validate actual Cargo metadata, registry checksums, compiler
backend, build scripts, linker or SDK input completeness.

Full judge binding uses:

```text
chrono-judge-cargo --protocol chrono-judge/v1 --policy .chrono-harness/cargo/projects.json
```

Register it after `registration` and `filemap`, and before operations by making
`projects` depend on it. The full adapter validates request identity, registration
views, retained inputs and committed policy bytes, then checks only Cargo projects
reached by the supplied explicit FILEMAP impact. Other languages are unaffected;
a documentation DELTA does not rejudge historical Cargo defects. Policy changes
need explicit edges to their consumers, as do manifest/lock/ancestor changes.
Errors retain their actual policy path; passing output lists checked projects and
explicitly does not certify complete inputs.

A host that maintains full registration data can instead register a single
ordinary prerequisite with explicit selection:

```sh
chrono-judge-cargo check --host-root HOST --config .chrono-harness/config.json \
  --policy .chrono-harness/cargo/projects.json --project library --project library-tests
```

That command never selects projects itself; its caller's FILEMAP plan decides
when to run it. Full protocol and standalone mode use the same check function.
The surrounding canonical runner still owns clean snapshots, tool binding,
operation receipts and failed-prerequisite handling. No Cargo subprocess is
launched by this bounded adapter.

The product builds and distributes this optional binary. Its own full host
registration remains proposed: adopting a path-only Cargo contract cannot close
this product's registry dependencies. The retained Cargo metadata/input adapter
and complete compiler/SDK closure remain required work. Existing core checks are
not claimed to cover that missing language-specific scope.
