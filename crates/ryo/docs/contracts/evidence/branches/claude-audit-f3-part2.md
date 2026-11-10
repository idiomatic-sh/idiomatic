# Leak review: claude/audit-f3-part2 (re-check)

Source `64f77797efe09b3fb9fd74b3f0c5c878de79f55f`, base `29e4717b64`, reviewer leak-review, 2026-11-10.

This is a targeted re-check of the check 2 FAIL at `29e4717b64`. That review is superseded by this one.

**Delta.** One commit adds 51 lines to two test files, both `cfg(test)`:
- `execution_domain/src/domain/matching/types/prepared/value_test.rs` (+21): the `Scenario::Replaced` arm and the row `replace_literals_are_declared_before_any_worker`.
- `execution_domain/src/domain/self_join_recogniser/service_test.rs` (+30): the `difference` and `intersection` helpers and two `Scenario::Traced` rows.

No production, manifest or doc file changes.

**Method.**
- I ran a `git archive` of the head in the scratch directory, with its own target dir `the review's own target directory`, every run behind `flock the shared build lock`.
- Each plant was reverted with `git checkout`, and `git status` was clean afterwards.
- The reviewed worktree the builder worktree was never written: HEAD stayed `64f77797ef` and its status stayed clean.

## 1 Proof where the contract lives: PASS
The new rows are colocated in `execution_domain`. Everything else carries forward from the `29e4717b64` review.

## 2 Every enforcement claim fires: PASS
**Without plants.** `cargo test -p idiomatic-sh-ryo-execution-domain` gives 144/144 green, plus 2/2 for the second test target.

**With each plant.** I re-planted the three single-variant drops that survived before. Each now goes red on its new row only (143/144):

| plant | result |
|---|---|
| A1: drop `Replace` from `Replace \| SourceMatches` (`matching/types/prepared/value.rs:61`) | `value_test::succeeds` FAILED, `replace_literals_are_declared_before_any_worker: the subject panicked` (`value_test.rs:239`) |
| A3: drop `Difference` (`self_join_recogniser/service.rs:121`) | `service_test::succeeds` FAILED, `a_number_traced_through_a_difference_keeps_its_relation_origin: expected (), answered Err(UnprovenNumberOrigin)` |
| A3: drop `Intersection` (`:122`) | `service_test::succeeds` FAILED, `a_number_traced_through_an_intersection_keeps_its_relation_origin: expected (), answered Err(UnprovenNumberOrigin)` |

**Carried forward.** The census ratchet and the M-A2, M-A45b, M-A3 Join and M-A1 SourceMatches catches hold from the earlier review, because no production code changed.

**Harness.** The new rows use `ryo_values::cases::run` (`values/src/cases.rs:95-145`) with `Expect::Value(())`. Panics and kind mismatches are recorded as failed rows, so a row cannot pass without running.

## 3 Boundaries hold across crates: PASS
The delta adds no `pub` item, `cfg` or `allow`. Carried forward.

## 4 One implementation and one representation per concept: PASS
The delta has no production code. The new test helpers `difference` and `intersection` mirror the existing `antijoin` and `union` helpers in the same file. Carried forward.

## 5 Layer direction: PASS
Carried forward. The delta adds no edge.

## 6 Test seams stay out of production: PASS
Both changed files are `*_test.rs` modules. Carried forward.

## Owned review criteria
| item | verdict | evidence |
|---|---|---|
| 3 | PASS | check 3 |
| 4 | PASS | check 2: each grouped-arm variant now has a catching seam test |
| 6 | PASS | check 4 |
| 13, 16, 17, 18, 20 | PASS | carried forward. The delta is test-only and deterministic (fixed plans) |

## Enforcement candidates
None, as before.

VERDICT: PASS
