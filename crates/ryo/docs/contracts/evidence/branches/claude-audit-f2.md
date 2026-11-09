# Leak re-review: claude/audit-f2 (bead ryo-isbn)

Source `0d6914eef0ee651b933e0fb79f77fd329844f085`. This final re-check covers `fe96c87071..0d6914eef0`, one commit adding one assertion; it closes the only open item (check 2). The review before it was of `fe96c87071` and failed check 2. Before that, the review was of `80fc0634a4` and failed checks 2 and 4. Since `80fc0634a4` there is exactly one commit, and it touches four files:
- `values/src/domain/rule_requests/errors.rs`
- `values/src/domain/rule_requests/service_test.rs`
- `values/src/domain/compact_table/service_test.rs`
- `core/src/inbound/use_cases/lint/errors.rs`

Nothing else changed. The reviewed worktree stayed at HEAD `fe96c87071` and stayed clean. No build was run: the archive copy could not be built because the shell guard requires the shared `ryo-333` target, so the copy was deleted unbuilt. The runner and the assertions were read instead.

## Check 1: Proof where the contract lives: PASS
The fix diff asserts no contract from `benchmarks/` or `languages/`. The earlier PASS stands.

## Check 2: Every enforcement claim fires: PASS
Re-check of `fe96c87071..0d6914eef0`. The delta is one commit, one line: `assert!(compact::compact_kinds(&meta).is_empty());` at `crates/ryo/crates/values/src/domain/compact_table/service_test.rs:216`.
- `meta` is the table returned by `admit_compact` on `PackedWords::default()` (`:206-213`), and `compact_kinds` returns `&input.kinds` (`compact_table/service.rs:153-155`). The case therefore fails if admission invents kinds for an empty input, which closes the weakened assertion from the previous review.
- The case runs under `run(CASES, Word::Succeeds, …)`, with panics recorded as failed rows (`crates/ryo/crates/values/src/cases.rs:107-113`, `:135-140`), so it can fail.
- No other assertion changed. The earlier plants (`private_interfaces` count 1, `internal-package-allowlist` count 1) carry over, because the delta touches no visibility and no manifest.

## Check 3: Boundaries hold across crates: PASS
The fix adds no `pub` item, `cfg`, `#[allow]` or `pub use`.

## Check 4: One implementation and one representation per concept: PASS
Both variants are gone from `values/src/domain/rule_requests/errors.rs`. The `From<RuleRequestsError> for LintError` arm (`core/src/inbound/use_cases/lint/errors.rs:155-165`) maps only `BlankRuleId`, `DuplicateRuleId`, `MissingParameter` and `Failed`. No Rust file names either variant. The remaining hits of those names belong to other types:
- `OperatingError` (`execution_domain/src/domain/operating/errors.rs:17`, `:29`), the one live representation.
- A local test enum, `Refusal` (`execution_runtime/.../acceptance/observations_test.rs:39`).

Enforcement candidate `unconstructed-public-variant-58a9552d` is carried forward as `instance_status: fixed`.

## Check 5: Layer direction: PASS
No item moves between layers.

## Check 6: Test seams stay out of production: PASS
No seam is added. The restored fn is in a `_test.rs` module.

## Advisory
`crates/ryo/docs/contracts/evidence/inventory/items/values.md:22` still lists `observation_incomplete` and `semantic_incomplete`. It is a regenerated census snapshot, so it is not counted.

## Builder fix 2: 0d6914eef0ee651b933e0fb79f77fd329844f085
The empty-bucket case adds `assert!(compact::compact_kinds(&meta).is_empty());` after its cell-count assertion (`values/src/domain/compact_table/service_test.rs:216`). The values nextest run (build lock, target `ryo-audit-f2`): 28 passed, 0 failed. rustfmt is clean.

VERDICT: PASS
