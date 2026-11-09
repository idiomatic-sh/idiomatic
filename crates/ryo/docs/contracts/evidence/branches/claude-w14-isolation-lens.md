# Leak review: claude/w14-isolation-lens

Source `3d1e6d91a4895cd0867d81c806983d81a08e8bb0`. Base `781653e221c3c7d4b937918d46496c5564ebe647`: the commit's parent and its merge-base with checkpoint 333. The request named no base. The change is docs only: `.agents/agents/contract-review.md` gains a "Change isolation" criterion. The reviewed tree had HEAD 3d1e6d91a4 and a clean status before and after the review.

## 1 Proof where the contract lives: PASS
The diff touches only contract-review.md:10,17,21. No contract is asserted from benchmarks/ or languages/.

## 2 Every enforcement claim fires: PASS
No rule, probe or lint is cited, so nothing was planted. No harness is in the closure.

The criterion can be enforced with evidence that exists, and it fails closed:
- (a) seam tests and (b) footprint are read from the diff.
- (c) is checked against census.md at 333 (`crates/ryo/docs/contracts/evidence/checkpoint/census.md`). Its method (lines 5-21 and 176-181) can be repeated read-only by grep, contract-review.md:10 makes the reviewer recount, and contract-review.md:21 makes missing evidence a FAIL.
- `ci/contract_review/service.py:37-39` needs only a non-empty checks list, so a fourth criterion breaks no validator.

## 3 Boundaries hold across crates: PASS
No code. Criterion (a) partly overlaps leak checks 3 and 5 (leak-review.md:14,16), but the overlap is tolerable:
- (a) grades how coupled the test is.
- The leak checks grade the production visibility that lets that test compile.

(b) and (c) have no leak counterpart. Line 10 now gives "whether each cited enforcement fires" to the leak review explicitly.

## 4 One implementation and one representation per concept: PASS
No code. Each lens has one owner: isolation belongs only to contract-review.md, and "census" appears in no other agent doc.

## 5 Layer direction: PASS
No code.

## 6 Test seams stay out of production: PASS
No code. The diff contains no machine paths and no tracker text. The bead id appears only in the commit subject.

## Advisories (not blocking)
- (c) says "the census" without a link. Link census.md and state the recount rule: the reviewer recounts at the reviewed sha, and the previous checkpoint's column is the baseline. Nothing names who adds the next column after 333.
- Census section 6 (semantic analysis) is a traced must-change table, not a number. Name the section 2 proxy rows (census.md:51-55) as the figures (c) compares.

VERDICT: PASS
