# error-variants-reached

## 0.2.0

A placed `test.rs` counts like a `*_test.rs`: a match, a row or a token group in a `test.rs` under the same `src/` reaches the variant. The crates' use-case and application tests are written in files so named, and they drive the variants end to end. Fixtures: `placed-test-rs-reaches-the-variant` (passing) and `a-helper-file-is-no-test` (failing: a file that is neither is no test). Nothing else changes.

## 0.1.0

Every error variant is proven (, a Tastemaker rule per
U10). Each variant of an `enum` whose name ends in `Error`, declared in `errors.rs`, is owed a
test unless it is `#[error(transparent)]`, an `error` attribute whose group holds only the
word `transparent` (it only forwards an error whose own hex proves
it). A variant is reached when a `*_test.rs` of its crate ("The crate is the scope", below)
names it qualified by its enum's name:

| where | counts |
|---|---|
| a pattern: `match` arm, `if let`, `let.. else`, `while let` | `FooError::Bar`, `FooError::Bar(..)`, `FooError::Bar {.. }`, `Err(FooError::Bar)`, with any path before `FooError` |
| a macro's arguments (`matches!`, `assert!(matches!(..))`, `assert_eq!`) | the enum's name, then later in the same token group the variant's |
| an expression outside a macro (`let error = FooError::Bar;`) | no: constructing a variant proves nothing |

A variant a dependency's failure produces is proven by a `fails_when_<dependency>_fails`
test with a failing fake from `fixtures.rs` (the `case-vocabulary` names).

### What the syntax cannot see

- **Pattern or expression inside a macro.** A macro's arguments are tokens until
 expansion, so `assert_eq!(result, Err(FooError::Bar))` counts as reaching `Bar` exactly as
 `matches!(result, Err(FooError::Bar))` does, and so would a macro that only constructs
 the variant. Telling them apart needs the expanded macro, which the syntax relations do
 not hold; the rule accepts both, and review holds a test to driving the input.
- **Adjacency in a token group.** "Later in the same group" is an ordinal comparison: the
 query language has no arithmetic to require the `::` right between, so
 `(FooError::Baz, Bar)` would count for `Bar`.
- **Names not written as `Enum::Variant`.** A variant matched unqualified (after
 `use FooError::*` or `use FooError::Bar`), through `Self::`, or through an alias of the
 enum (`use..::FooError as E`) is not seen: resolving a name to its declaration needs
 name resolution the syntax does not give. Write the enum's name.

### Fixtures

Passing: `matched` (match, if let and `assert!(matches!(..))`), `compared` (`assert_eq!`
against a constructed variant), `method-test` (a `let.. else` in a split adapter's
`method_test.rs`), `not-an-error` (an enum in `errors.rs` not named `*Error`). Failing:
`unreached`, `constructed-only`, `other-crate` (a test of another crate matches it) and
`unqualified` (matched only after a glob import) and `named-transparent` (a message
naming a field called `transparent` is no transparent variant).

The shared rule block places it over every `errors.rs` and `*_test.rs` of a product
(devtool 0.25.0): an error enum outside a hex is owed tests as much as one inside, and the
hexes of both the 0.7.0 shape and the hexagonal model are covered by one entry.

For each finding:

- Write the test that produces the variant and match it, qualified by its enum:
 drive the input that makes the domain refuse (`refuses_<...>`, `rejects_<...>`), or,
 for a variant only a dependency's failure produces, make that dependency fail with a
 failing fake from `fixtures.rs` (`fails_when_<dependency>_fails`). A test that only
 constructs the variant and reads its message proves nothing.
- Qualify a variant matched after a glob import (`Err(Cut(..))` becomes
 `Err(FooError::Cut(..))`).
- Remove a variant nothing can produce: it is dead code.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.

## The crate is the scope

A variant is reached by a test anywhere under the `src/` of the crate that declares it, not only under the
directory holding its `errors.rs`. A part keeps the errors it produces, and a port's variant is produced by
an adapter of another crate and handled by the use case that wraps it, whose `fails` row (a failing fake) states it
in another directory of the core: that row reaches the variant. A test of another crate does not count, and a
variant no test of its crate names is dead or untested as before. Fixtures: `consumer-test` (passing: a test of
another hex of the same crate matches it) replaces `other-hex`; `other-crate` (failing). A hex directory has no
scope of its own any more.

## The rows of a part's test

A variant is also reached when it is the error of a row, the argument of an `Expect::Error(..)` call (`Expect::Error(FooError::Bar)`): the refuses, fails and rejects rows of the part's test, in the one test shape, reach the variants they name. A variant named only as the value of a `succeeds` row (`Expect::Value(..)`) is not reached. The pattern and token-group forms stay for the crates still on the hex spec.
