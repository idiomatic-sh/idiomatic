# factory-calls 0.1.0

The first release. Composition is the
edge's concern, one per CLI command, so only an edge names a `factory` module: a path
segment `factory` (a `name-ref`) in a use or an expression is a finding in every file its
entry reaches. The entries leave the edges out: each command's `composition.rs`, tests
(`*_test.rs`, `tests/`), benches and `fixtures.rs`.

What the syntax cannot see: a factory reached through a re-export under another name; a
local binding named `factory` is read as the module. The resolved-type factory lints take what needs types: a factory building no
client or port, and choosing nothing.

Fixtures: passing `edge`, `test`; failing `use-case`, `adapter-imports`.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.
