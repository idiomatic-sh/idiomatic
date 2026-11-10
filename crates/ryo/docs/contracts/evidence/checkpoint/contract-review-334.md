# Contract review, checkpoint 334 (re-check of B8): PASS

- **Source:** `7e0faf9d14578a091ee07d97dc0606dc1b9505dc`
- **Prior review:** `79c1329bdf` (FAIL on B8 only)
- **Base:** `40e4902152`
- **Frozen validation:** `333641a62a` (validation335)

The delta `79c1329bdf..7e0faf9d14` changes only the record and the harness. `4ac5f0594d` makes `run.py` write the `## Known red` section of review/README.md from `validation.json` and adds a test for it, with its red/green entry. `7e0faf9d14` rewrites Verification limits, fixes the punctuation in matrix item 1 and updates the records. No engine or production Rust source changed. This pass was read-only. All work ran in a scratch clone.

## Commands run by this review
| Command | Exit | Result |
| --- | --- | --- |
| `run.py --check` (clone at the source, stale consolidation ref `80f473c67d`) | 1 | 1011 DOC lines from the wrong base. Discarded as a mis-set probe; the clone stayed clean |
| `git update-ref refs/remotes/origin/claude/ryo-consolidation 40e4902152`, then `python3 -I .../run.py --check` | 0 | 6 RED lines, no STALE RECORD, no DOC/FIX/RESULT/TICKED line; `git status` clean afterwards |
| `diff` of the `^RED ` lines against the `- `RED` bullets of `## Known red` | 0 | identical, 6 = 6 |
| recount of `{temp}/ryo-contract-validation334/default_features.log` with run.py's `TEST_RESULT` and `RUNNER_COUNTS` | 0 | 2402 + 93 + 11 = 2506 |

## B8: closed
- The 6 RED bullets in `## Known red` match the 6 `RED` lines from `--check` byte for byte: sdk_api, default_features, libraries, core_allocation, cli_tests and cli_cost.
- `--check` rewrites the section and fails on any change. The clone was clean after an exit 0, so the per-test sub-bullets and the `tastemaker_workspace` line also equal `known_red(validation.json)`.
- Verification limits now cite validation335 at `333641a62a`. Their Django figures (89.8 s, load 1.89, `f50bb571`) and tm figures (0 in tracked files, `a33f361f7a`) match the recounts from the prior pass.

## The run.py change cannot turn a red green
- `red_line` builds exactly the string the old `print` built.
- `red_commands` is unchanged.
- `known_red` and `write_known_red` only read `v`.
- The only effect on the exit code is appending `review/README.md` to `changed`. Under `--check`, that makes the exit 1, never 0.

## 2506
The log has 85 `test result:` lines, which give 2402 passed plus failed. That equals the sum of the `running N tests` lines. On top of that come 41 cucumber `N scenarios (` lines totalling 93, and `activation: 11 actual cases passed`. The sum is 2402 + 93 + 11 = 2506, which is `checked.count`. The log's sha256 is `412f8ac7f54ef1119f49f4b7503cf8557182236c930132cc8c3ed46b3d9683e6`.

## Grades
| Criterion | Verdict |
| --- | --- |
| Enforced boundaries | PASS (carried from `79c1329bdf`) |
| Clear ownership and flow | PASS (carried) |
| Proof | PASS (B8 closed; B1 to B7 carried as closed) |
| Change isolation, 1, 2, 4, 5, 7, 10, 15, 19, 20, 21 | PASS (carried) |
| Goal exits | NOTE (carried) |

## Advisories (not blocking)
1. `run.py:833-836`: `--check` writes review/README.md before it fails on the change, as it already does for the matrix `.md` files. A checker that mutates the tree is the existing pattern, now one file wider. It never greens a red.
2. `run.py:834`: if review/README.md is missing, `if review.exists()` skips the check silently. A deleted README drops the check instead of failing it.
3. The record does not state the 2506 breakdown, and the committed evidence does not bind `default_features.log` by hash. The hash is recorded here.
4. The DOC result from `--check` depends on `origin/claude/ryo-consolidation`. A clone with a stale ref gives 1011 DOC lines and exit 1.
5. Not run in this read-only pass: `run_test.py`. The new test's red/green logs are recorded, and `--check`'s `red_green_problems` accepted them.
