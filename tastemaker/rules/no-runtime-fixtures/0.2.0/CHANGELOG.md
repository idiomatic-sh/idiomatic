# no-runtime-fixtures

## 0.2.0

An item gated `#[cfg(any(test, feature = "fixtures"))]` is test-only like an exact `#[cfg(test)]` one, with its syntax descendants: the gate compiles it for the crate's own tests and, through the `fixtures` feature, for the tests of the crates that consume it, so what it names is test support, not production code. A crate split from the engine keeps its proof observations behind that gate because its consumers' tests read them. Fixture `fixtures-gated-item-reads` (the gated field and method are not findings; the same read in an ungated method is). Everything else is as in 0.1.0.

Edited in place: a group's `test.rs` is test code when the module file `<dir>.rs` beside its directory declares it gated, where it was keyed on `<dir>/mod.rs` (the one module-file style, owner 2026-11-11); the scenarios name module files `<dir>.rs`, and a `#[path]` in one resolves from that file's directory.

## 0.1.0


The first release: `fixtures` is
test-only with no exception, so production code never names a `fixtures` module. A path
segment `fixtures` (a `name-ref`) in a use or an expression is a finding in the files its
entry reaches; the entries leave out test code (`*_test.rs`, `tests/`, `benches/`) and
`fixtures.rs` itself. Declaring the module (`pub mod fixtures;`, a `name`) is not a finding.

What the syntax cannot see: a fixture reached through a re-export under another name; a
local binding named `fixtures` is read as the module.

Fixtures: passing `declared-and-tested`; failing `production-reads`, `other-crate`.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.

A `fixtures` qualified by `domain` (`crate::domain::fixtures::…`, a domain concept of that name) is not a finding, in a path of code, of an import or of a type: a product whose subject is test cases (a linter's fixtures of its rules) names its own domain, not test support. Fixtures `domain-concept-named-fixtures` and `domain-concept-in-a-type`.

A `fixtures` that is the whole of an expression's path (a binding: `fixtures.first()`) is no longer read as the module: a module is named by a path that goes on past it or by an import (fixture `binding-named-fixtures`).

## Test-only syntax scope

Exact `#[cfg(test)]` items and their syntax descendants are test-only. This includes nested inline modules and aliased fixture imports. Test-only `include!` sites no longer invalidate otherwise registered generic `test.rs` roles; production include/redirection controls remain conservative. Negated test cfgs remain production.
