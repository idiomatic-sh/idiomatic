# Leak review: claude/w13-visibility (re-review)

Source `d0e39d059cfbd3cf00a26799c8d4d3cf07ca777a`, base `b1e81f724206237fad67a42bfe1557fdb838b9e2`, previous review of `616dbb9433` gave FAIL: 4. Scope: `idiomatic-sh-ryo-python` (semantic_syntax, model, parse contract) and `ryo_language` domain/elements. An all-targets check of `idiomatic-sh-ryo-python` on a `git archive` copy finished with 0 errors. The engine crate's own test target is outside the allowed command, so it was not built. The copy and its target directory are deleted.

## Equivalence with the old logic
- **visibility:** gives the same answer, for the reason the previous review gave. The engine keeps embedded expressions disjoint and ascending by root, and a holder is appended before the expressions nested in it. `hidden` now holds roots (`model/types.rs:136`), and `hidden_at` / `visibility` (`semantic_syntax/service.rs:29-78`) use `embedded_source_at` plus a `binary_search` of those roots. A hidden expression nested in a visible one is still decided by `type_position`.
- **operand_place** (`service.rs:228-245`):
  - `cast`: the first operand is `next()`.
  - `NewType`: the second operand is `nth(1)`.
  - `TypeVar`: the argument is an Expression and is not the first Expression, which is the old `positional > 0`.
- **ancestors** (`service.rs:185-198`): stops after yielding the host's statement. Slice, Subscript, CallArguments and AnnAssign never hold a statement, and embedded trees hold no statements. So nothing above the statement can change the answer.
- **annotated_operand** (`service.rs:267`): now stops at the subscription. The old walk reached the root. Both end with `owner != first`, so both answer Value.

## Check 1 Proof where the contract lives: PASS
The `embedded_source_at` bound is proven in the engine crate (`crates/ryo/crates/language/src/domain/elements/service_test.rs:137-171`). The python tests prove only provider logic.

## Check 2 Every enforcement claim fires: PASS
No rule, probe or lint is cited. Each new test can fail:
- **Engine:** interval reads through `holding` must stay within 13,312 for 1024 x 1024, with answers checked against a hand formula. A scan per row reads at least 524,288.
- **Operands:** the production `operand_place` may read at most 2 operands, for both the 2nd and the 10,000th operand of each form (`comparisons_test.rs:111-119`). The replaced `position()` scans read up to 10,000.
- **Blocks:** the production `ancestors` walk from a `cast` operand 64 `if`-blocks deep must take at most 4 steps and end at `Assign` (`comparisons_test.rs:124-146`). The replaced walk took 64 or more.
- `nested_test.rs` adds a two-deep `cast` case with fixed offsets.

Advisory: the engine count is taken on a direct `holding` call (`service_test.rs:161`), beside `embedded_source_at` (`:165`). It guards the public function only while that function delegates to `holding` (`elements/service.rs:347`).

## Check 3 Boundaries hold across crates: PASS
`holding` is `pub(crate)` (`elements/service.rs:354`) and is not re-exported. `embedded_source_at` keeps its signature, so the SDK surface is unchanged.

## Check 4 One implementation and one representation per concept: PASS
The provider's copy is deleted. One implementation remains, `ryo_language` `holding`, and `embedded_source_at` delegates to it. `git grep` over crates and languages finds no other bisection over embedded roots; markdown `model::holding` and python `siblings::holding` are unrelated concepts. The test counters are closures passed into production functions, not copies.

## Check 5 Layer direction: PASS
The changes stay in the domain layers of each crate. No inbound or application item crosses a crate.

## Check 6 Test seams stay out of production: PASS
The `bounds` parameter of `holding` and the `category` parameter of `operand_place` are crate-private. Production passes the real readers, and nothing is bypassed.

## Residual: long left-nested string chains
Accepted, and no memo is needed now. A chain of K string operands in one expression (`"a" + "b" + ...`) still costs O(K^2) ancestor steps through `completion_hosts`.

I measured the benchmark Django tree (the benchmark corpora cache's `projects/django-36b5f39d93...`, all of it including `tests/`) with Python `ast`:

| Measure | Value |
| --- | --- |
| Files | 2,874 |
| String constants | 150,007 |
| Steps from each string to its statement, total | 435,837 |
| Steps per string, mean | 2.9 |
| Steps per string, maximum | 20 (`tests/aggregation/tests.py:1523`) |
| Strings at 15-19 steps | 16 |

The ryo tree adds Extra nodes, which is a small constant factor. Real chains stay short, so the cost is linear in practice. The quadratic shape needs generated code, so it stays an open item.

## Enforcement candidates (carried forward)
- `provider-reimplements-engine-embedded-lookup-9aea485e`: instance fixed.
- `sibling-index-scan-per-element-38619f80`: instance fixed.
- `nested-write-embedded-in-callback-0badb528`: open. The engine still does not refuse `write_embedded` inside a `write_embedded` callback.

VERDICT: PASS
