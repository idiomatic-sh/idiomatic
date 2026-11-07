# unit-cases

## 0.1.0

The first release. A finding is a unit test with no `Word::Succeeds` row, or a part's main test with no `Word::Refuses` row where an `errors.rs` in the same directory declares a variant.

The `refuses` row is owed for a variant of the part's own, one with no `#[from]` field attribute: a variant that wraps another error (`Names(#[from] NamesError)`) is a dependency's failure, which a `fails` row states and `error-variants-reached` holds to a test, so a part whose errors only wrap owes no `refuses` row (a row for an input the part does not refuse would be a placeholder). Fixtures: `wrapper-only` (passing) and `mixed-without-refuses` (failing: an own variant beside a wrapping one, with a `fails` row only).

contracts (`fixtures/contract.rs`) and fakes are no longer `*_test.rs` files, so the contract exemption is gone.
