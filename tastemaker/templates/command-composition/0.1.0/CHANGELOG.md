# command-composition

## 0.1.0

The `cli` spec's owned `command-composition` with `compose(parts)` beside `build()`: the template for it. A command's `composition.rs` declares
`<Resource><Command>Parts`, one field per port its graph wires; `compose(parts)`, the one
wiring; and `build()`, which passes the production parts to `compose`. The command still
calls only `build`; a bench or a test calls `compose` with parts of its own.

Staged here, unplaced, because the `cli` spec's working version is the one the products
follow: changing its owned template now would make every command's composition a finding
before the products have `compose`. A later change moves this directory over
`specs/cli/0.10.0/templates/command-composition/`, renames the fixtures' rule to
`idiomatic.arch/cli/command-composition`, splits each command's wiring into its parts, and
deletes this directory.
