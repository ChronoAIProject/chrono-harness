<!-- chrono-instructions:begin -->
## Shared instructions

Before working in this host, read BOTH canonical files in full, in this order
(paths are relative to this host's root, not the current working directory):

1. `.chrono-harness/instructions/methodology.md` — shared methodology.
2. `.chrono-harness/instructions/host-context.md` — deliberate host context; an empty file is valid.

Both AGENTS.md and CLAUDE.md route to these same sources. This managed block
is a generated reading route, not an independent methodology source. Text
outside the block belongs to the host. Evolve the canonical files deliberately,
then run `chrono-instructions generate --host-root <host-root>` to validate the
inputs and refresh both routes. Generation does not certify that an agent read
or obeyed these instructions, and does not activate harness judges.
<!-- chrono-instructions:end -->
