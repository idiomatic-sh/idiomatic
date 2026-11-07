# case-vocabulary

## 0.2.0

The case-word check. A test, a function
with an attribute whose path ends in `test` or `rstest` (`#[test]`, `#[tokio::test]`,
`#[rstest]`; not `#[rstest::fixture]`), names its case with at most one of `succeeds`,
`refuses`, `fails` and `rejects`, each a whole snake_case word, a raw identifier read as
its plain spelling; `fails` appears only as `fails_when_<dependency>_fails`, the
dependency named (a prefix such as `<command>_` is allowed); and a `refuses` test does not
implement a port (the trait of a trait impl, named `*Port`, `*Item` or, in a hex crate, `*Interface`) in its body, since running
the contract over its own weaker implementation is a `rejects` case. The
`Proof`/`Rejection` harness check of 0.1.0 is unchanged. The category moves from
`correctness` to `testing`. `hex` 0.6.0 places it over a domain's `**/*_test.rs` and
`cli` 0.5.0 over `src/commands/**/*_test.rs`.

A test named exactly `succeeds`, `refuses`, `fails` or `rejects` is the runner of the one test shape (
`test-shape`), so a bare `fails` is allowed beside `fails_when_<dependency>_fails`. The `devtool` starter 0.27.0
places the rule over every `*_test.rs` of a product, as it places `error-variants-reached` and `no-ignored-tests`: the `hex` spec no longer names it, which also leaves it with a starter as its referrer.

A trait named `*Port` (an outbound port) or `*Item` (a collection's item) is a port as `*Interface` is, since the `part` spec names them so; `*UseCase` is not one, for a use case has no contract to run a weaker implementation through. The fixtures `refuses-weaker-port-implementation`, `refuses-weaker-item-implementation` and `port-words` hold the three names.

A test naming none of the four words is not a finding: which word a case deserves is a
judgment on what the test does, which a syntactic query cannot make.

Moving up from 0.1.0: rename each test the rule reports. Keep one case word, the one the
test proves: `succeeds`; `refuses` when invalid input yields a typed domain error; `fails`
when a dependency's error propagates, as `fails_when_<dependency>_fails`; `rejects` when
the contract catches a weaker implementation. A `refuses` test that declares its own
implementation of the port becomes `rejects_<...>`.

## 0.1.0

The first release: test code declares no
struct, enum, union, trait, trait alias or type alias named `Proof` or `Rejection`, the
hand-copied harness that ran weaker implementations through a contract. cargo-mutants,
the mutation gate, proves the tests bite; a weaker implementation no mutant expresses is a
plain `rejects_<...>` test. The entry's `include` names the test files (`hex` 0.4.0 places
it over `**/*_test.rs`). U1 adds the case-word check of test function
names in a later version.

Moving up: new; nothing to move from.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.
