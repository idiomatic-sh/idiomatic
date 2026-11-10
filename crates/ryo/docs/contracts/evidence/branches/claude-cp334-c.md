# Leak review: claude/cp334-c (940a264aaa) against 752cc02c32

Scope: beads .16 (index conversions) and .17 (one counting observer per run, ExecutablePermission example). Closure: values, relations, derived, operators, query_language, core, sdk, cli tests. Reviewed tree unchanged (HEAD 940a264aaa, clean status before and after).

## 1 Proof where the contract lives: PASS
The .17 proof is core's own case `runs/test.rs:97-102` over core fixtures (`lifecycle.rs:240-264`, `scenario.rs:85,160`). The .16 conversions are in each newtype's own `types.rs`. Nothing is proved from `benchmarks/` or `languages/`.

## 2 Every enforcement claim fires: PASS
- Plant: `lint` builds its observer once per use case (a thread_local `OnceCell` around `make` at `use_case.rs:181`), in a detached review worktree. The nextest run of core's lib filter `runs::test` gave `expected (2, 2), answered Ok((2, 4))`. After the revert, 3/3 passed. This matches the builder's red.
- Harness: `cases.rs:83-138` records mismatches, panics and wrong-kind rows as failures, and refuses a word with no rows. No path turns a failure into a pass.
- Advisory (residual): the clippy plant for `unfulfilled_lint_expectations` (removing the `as` in a `From`) was not run, because the shell guard allows only check and nextest outside an integration tree. By reading: `as_conversions = "deny"` (`crates/ryo/Cargo.toml:140`) and each `#[expect]` covers exactly one `as`, so all six are fulfilled. Probe to settle it: the integrator's `asymptote check --groups clippy` with one `as` removed.
- Advisory: a whole-package nextest run of core (without `--lib`) fails at test listing on the red `allocation` harness=false binary. That is .19's subject, not this branch's.

## 3 Boundaries hold across crates: PASS
- The `From<X> for usize` impls sit on types that were already pub. No manifest changed.
- `counting` names the same `ObserverPort` it named before.
- sdk `api.txt` is unchanged, and the doctest uses only the re-exported `ryo::linter` paths.

## 4 One implementation and one representation per concept: PASS
- There is one `From` per index type (StepId, ColumnIndex, ObservationBindingId, VertexId, PartitionId, Position). The diff adds 0 sentinels and removes 36.
- StepId and PartitionId are needed by `selecting/test.rs:608` and `compact_graph/value.rs:33`.
- Advisory: 239 newtype sentinels remain in the tree, some in the touched crates (derived 1, operators 5, query_language 13, values 4). Scope 2.4 puts them outside this gate. They are handed to the TM steward as the candidate `usize-max-index-sentinel-0c59892e`.

## 5 Layer direction: PASS
Core receives an `fn() -> Arc<dyn ObserverPort>` from the edge and calls no runtime factory.

## 6 Test seams stay out of production: PASS
`Scenario::CountedTwice` and `counted_twice` are under `lint/fixtures`, gated by `cfg(any(test, feature = "fixtures"))` (`lint.rs:10-11`). Advisory, older than this diff: `LintUseCaseImpl::counting` is pub, ungated and has no production caller.

## Criteria for .16
- **Lossless.** Every `From` is u32 to usize. Every release target is 64-bit (x86_64/aarch64 linux-gnu/musl, darwin, windows-msvc), and the conversion is lossless on any target with a usize of 32 bits or more. The expect reasons are therefore true.
- **No caller relied on the sentinel.** On 64-bit, `usize::try_from(u32)` never failed, so `usize::MAX` never stood in for "absent". Every `.get(..)` and indexing site behaves the same.
- **adjacency.** `pair.0 + 1` moved from u32 to usize arithmetic (`adjacency/service.rs:108`). It cannot overflow where the u32 sum could (debug panic at u32::MAX). That is no regression.
- **Kernel expects.** The function-level expects on `build_borrowed` and `forest` follow scope 2.4. Their reasons are true.

## Criteria for .17
- `lint` calls `make()` once at the top of each run (`use_case.rs:181`). `begin`, the unobserved split and the visit all use that one observer.
- Advisory (doc): the comment on `Scenario::CountedTwice` (`scenario.rs:85`) says "the same request twice". The fixture actually lints rules "one" and "two" (`lifecycle.rs:240-243`), so the comment should name the two rules.
- The ExecutablePermission doctest matches the scope.

VERDICT: PASS
