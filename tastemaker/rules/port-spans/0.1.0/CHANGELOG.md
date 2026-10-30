# port-spans 0.1.0

The first version. It is written as a Tastemaker rule
rather than a dylint lint because a port is syntactic: a trait whose name ends in `Interface`
(G5). The `Interface` suffix is retired in favour of `<Dir>UseCase`, `<Dir>Port` and `<Dir>Item`, so the rule reads `UseCase` and `Port` as ports beside `Interface` (a hex crate's), and `Item` as none: an item never crosses a crate.

A finding is: a `fn` in the item list of an `impl <Name>UseCase`, `<Name>Port` or `<Name>Interface for <Type>` block without
`#[tracing::instrument(level = "info", skip_all)]`, optionally with `fields(..)` and with no
other argument (its top-level tokens are only the identifiers `level`, `skip_all`, `fields`
and the string `"info"`; the path is matched as `tracing::instrument`, so an imported
`instrument` does not count);
an `instrument` attribute on any other function; and a `span!`, `trace_span!`,
`debug_span!`, `info_span!`, `warn_span!` or `error_span!` call anywhere. It reads
`syntax-node`, `syntax-child` and `syntax-token`: the attribute is an `attr`
child of the `fn`, its `token-tree-meta` holds the path and the argument tokens. Not seen: a
span opened inside another macro's arguments or by a procedural macro.

Placed nowhere yet: the products' port methods carry no span today, and a later change
 adds the attribute with the placement. Its home is decided: an owned rule of the `hex` spec's working version, placed with
`include = ["**/*.rs"]` and the hex's tests, fixtures and `interface.rs` excluded; no Rust rule
package. Like every rule of `idiomatic.arch`, it reads the shared, unprefixed relations and
Rust's kebab-case syntax kinds, as renamed them.
