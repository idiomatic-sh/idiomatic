# Leak review: claude/cp334-b (checkpoint 334 gate repairs, builder B)

Source `2308cf70792e72f268b70a680759166d1e59b547`, base `100297522f`. Scope: `.13` (RelationStore sealed, `fixture` an inherent constructor), `.18` (S13, plants 12b/15b/15c with a `bases` table), and RelationStore's `# Examples`. The reviewed worktree the builder worktree stayed at the source sha with a clean status from start to end. Plants ran in a detached review worktree at the source sha, with its own target dir the review's own target directory, under `flock the shared build lock`. Both were removed afterwards.

## Measurements

| Run | Result |
| --- | --- |
| `boundary_contract` unplanted | 154/154 pass |
| plant `13-latch-last.patch` | 153/154: `shared_walk::order::error_order::test::the_first_engine_error_of_an_evaluation_is_the_one_reported` FAIL |
| plant `15b-release-keeps-generation.patch` | 153/154: `project_phase::consumers::test::a_publication_held_past_its_stage_is_refused_by_the_next_stage` FAIL |
| store doctests (`cargo test -p idiomatic-sh-ryo-execution-store --doc -- relations`) | 7/7 pass, including the `compile_fail,E0277` probe (`interface.rs:54`), the trait `# Examples` and `RelationOwner::fixture`'s example |
| plant: supertrait removed from `RelationStore` (`interface.rs:110`), `expect` changed to `allow` and `dead_code` allowed so the lib builds | the E0277 probe FAILED (the impl compiled), so the probe's body has no other error and the seal is the only thing refusing it |
| `run.py --census` at source vs base (both git trees) | identical on every key |
| `tm check` (tm 0.1.2) on the 5 changed `.rs` files | 0 errors, 0 suppressed |

All three patches (12b, 15b, 15c) and `13-latch-last` also apply cleanly at the source sha, not only at their `bases` commit `10821828c5`.

## Departures judged

- **15b kept behind `if false`.** This is a faithful mutant. `if false { invalidate() }` behaves exactly like deleting the call: the ended generation is never invalidated. Only the dead-code lint tells the two apart. It is the form `15-retire.patch` already uses (`if false && ...`).
- **15b caught by the consumers test, not S15.** Acceptable. `project_phase/consumers/test.rs:234` drives only `ryo_execution` inbound vocabulary (`ExecutionUseCaseImpl`, `ObservationBatches`, `RefusedFiles`) and the test crate's own fixtures. It names no `ryo_execution_runtime` item (0 occurrences), and the census is unchanged. S15 stays green under 15b for a real reason: `release_population` installs a fresh `GenerationLease`, so `resident.same(..)` refuses `NotRead` without invalidation. S15 sees the generation swap. Only a publication held across stages sees the missing invalidation. The builder recorded this honestly. The integrator's table row for 15b must name the consumers test, not S15 (advisory).
- **S13 adds its own Select over the tree's step.** Needed and sound. A Select of `SyntaxTree` is answered by the shared walk and never reaches `OperatorsPort`. The second step (`Input::Step(StepId(0))`) is what makes the operators evaluate in row mode, where `population` refuses `NotRun`. The plan types come from `ryo_values`, which is seam vocabulary.
- **S13 rewritten to keep the census unchanged.** Confirmed: the census is identical at base and source. The fake `OperatorsPort` follows the same pattern as `shared_walk/fixtures.rs:52-68`.

## Check 1: Proof where the contract lives: PASS
The seal is proved in the store crate (`interface.rs:51-92`). Semantics 12, 13 and 15 are proved in `execution/tests` through `boundary_contract`. Nothing rests on `benchmarks/`, `languages/` or a product corpus.

## Check 2: Every enforcement claim fires: PASS
Three plants each reported 1 failure (table above). No skip, filter or caught error turns a failure into a pass. S13 can fail, as the 13-latch-last plant showed. Advisory: `let _finished = file.finish();` (`error_order/test.rs:287`) discards finish's result after the asserted refusal. The sibling test unwraps it. This hides nothing the test claims.

## Check 3: Boundaries hold across crates: PASS
`interface.rs:16` declares `pub(crate) mod sealed` and nothing re-exports it. The only `Sealed` impl is `relations.rs:191` and the only `RelationStore` impl is `relations.rs:192`. No feature touches the seal. `interface.rs` has no `cfg`, so the trait's item list is the same with and without `fixtures` (criterion 13).

## Check 4: One implementation and one representation per concept: PASS
The trait's `fixture` and its impl are deleted. The inherent `RelationOwner::fixture` (`relations.rs:167-175`) is the only constructor. The fake operator delegates to `OperatorsUseCaseImpl` and copies no operator logic.

## Check 5: Layer direction: PASS
The seal sits beside the port trait. The other changes are tests, docs and evidence.

## Check 6: Test seams stay out of production: PASS
`fixture` is gated by `cfg(any(test, feature = "fixtures"))` (`relations.rs:167`). `execution` and `execution_runtime` take `execution_store` without `fixtures` in `[dependencies]` (`Cargo.toml:20` and `:17`). They turn it on only in dev-dependencies (`:32`).

## Advisories (no FAIL)
- 15b's catcher is the consumers test. The integrator's `run.py table_of` row should cite it, not S15.
- At base, the committed `census.json` says `vocabulary: 72`, while `run.py --census` computes 71 at both base and source. The artifact was already stale and this branch did not cause it. The integrator's regeneration fixes it.

Enforcement candidates: none.

VERDICT: PASS
