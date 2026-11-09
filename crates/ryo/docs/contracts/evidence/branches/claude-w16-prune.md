# Leak review: claude/w16-prune (re-review)

Source b5b47621fa2626284d51f006e1403483ad90387d, base e4f1d396eb (claude/checkpoint-333), bead ryo-y95r. This re-review replaces the FAIL at dfc5f15c35.
The review ran read-only. The worktree's HEAD (b5b47621fa) and its clean status were the same before and after. The plants ran in a `git archive` copy, which has since been deleted.

## What changed since dfc5f15c35
The repair commit changes only 6 files, all named in its message:
- `authoring/profile_proofs.py`: 4 lines added, 1 removed.
- `authoring/profile_staging.py`: 2 lines added.
- `authoring/profile_guard_tests.py`: 10 lines added.
- `profiles/prepared.json`: two hunks. Three authoring hashes are rehashed, and the 8 stale `expanded-defaults` `query_files` entries are removed.
- `results/correctness.json`: the `status` line is reworded, and `"historical": true` is added.
- `results/current-canonical-ready-284-case-preservation.json`: deleted. Nothing in the tree references it.

Nothing else changed.

## Check 1: proof where the contract lives: PASS
The edits stay in the ruff authoring modules and their manifest or results. They assert no engine, core or SDK contract.

## Check 2: every enforcement claim fires: PASS
The fix:
- `authoring/profile_proofs.py:137-139` refuses any combined cohort, active or archived, whose `query_files` or `case_files` value is not in its `input_files`. The refusal names the cohort and the keys.
- The self-test plant (`authoring/profile_guard_tests.py:127-136`) requires that exact refusal on an archived cohort.
- `verify_profiles.py --self-test` exits 0 on the archive copy.

Plants (each reverted):

| Plant | Count |
| --- | --- |
| The 8 removed entries (B039, DTZ007, G101, G201, LOG002, UP003, UP021, UP024) re-planted on `expanded-defaults` | 1 (refused, all 8 named) |
| A nonexistent `query_files` entry on `current-canonical-ready-310-r2` | 1 |
| A nonexistent `case_files` entry on `current-canonical-211-refresh` | 1 |
| `oracle_file` or `expectations_file` pointed at a missing file on `shape-110`, `expanded-defaults` and the active cohort, run through `stage_combined` | 6 of 6 refused |
| `case_preservation_file` pointed at an unsealed file on the active cohort | 1 (`profile_staging.py:173`) |
| `ready_assets_file` pointed at a missing file on the active cohort | 1 (`native_standalone.verify_ready_assets`) |
| `--profile current-canonical-211-refresh` through `stage_combined` | 1: the named refusal "is a case-preservation baseline only" (`profile_staging.py:76-77`), no longer a KeyError |

The rest of the class: I enumerated every path-bearing descriptor key. `profile_file`, `oracle_file`, `expectations_file`, `case_preservation_file` and `ready_assets_file` are each in `input_files` today. A dangling or unsealed value in any of them refuses when the cohort is used, either by a hash check, a missing file or an `input_files` membership check. `verify_files` does not check those keys statically. That is a refusal on use, not a failure that turns into a pass, so it gets no finding.

A plant of another cohort's sealed path inside `expanded-defaults.query_files` is accepted. That file is sealed by hash under the other cohort, so it is not an unsealed reference.

The advisories from the previous review are closed:
- `correctness.json:2,4` is marked historical and names the active cohort.
- The orphan preservation result is deleted.
- 211-refresh refuses by name.

## Check 3: boundaries hold across crates: PASS
There is no Rust, manifest, visibility, feature or import change.

## Check 4: one implementation and one representation per concept: PASS
The new check extends the existing `verify_files` loop (`profile_proofs.py:131-139`). The refusal is one guard at the top of `stage_combined`. Nothing is added in parallel.

## Check 5: layer direction: PASS
No crate is touched.

## Check 6: test seams stay out of production: PASS
The new plant is in the guard self-test, which runs only under `verify_profiles.py --self-test`.

## Enforcement candidates
- `unsealed-combined-input-reference-dfaacc29`, instance_status `fixed`: `verify_files` now holds the subset check, and the self-test plant covers it. It is carried forward for the TM steward's decision.

VERDICT: PASS
