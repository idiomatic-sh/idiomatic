# Contract review, checkpoint 334 (final head): FAIL

- **Source:** `79c1329bdf3382a990054948b36b2f107e6e2e81`
- **Base:** `40e4902152` (333)
- **Frozen validation:** `333641a62a`

The delta after the frozen head is harness only: `sdk_api.py` (`b6aff49ae3`, `56169f083f`) and `run.py` (`c00074ffe6`, `9bfc1c34c8`). Beside it are the two test-line casts (`91c711ca46`), the clippy lock drain (`2eea0b8575`) and the record commits. No engine production source changed. Under the PM ruling, the frozen validation stands. This pass re-measured what the delta can move, in a scratch clone at the source. The integration tree was not modified.

## Commands run by this review
| Command | Exit | Result |
| --- | --- | --- |
| `run.py --check` (clone at the source, consolidation ref set) | 0 | no stale record; DOC, FIX and census clean |
| `run.py --census` (same clone) | 0 | equal to the committed `census.json`; ratcheted 2 / 51 / 24 / 25 |
| `tm --root crates/ryo check --format json` (tm 0.1.2 from the checkout, clean clone) | 0 | 0 findings, 28 suppressed; 4:09.68 wall, 4,284,024 KB |
| `cargo metadata --locked --offline`, then `internal_crates(…, idiomatic-sh-ryo-sdk)` | 0 | 12 crates |
| `rustfmt --check --edition 2024` on the two files of `91c711ca46` | 0 | formatted |

## Blockers B1 to B7
| Blocker | Status | Evidence |
| --- | --- | --- |
| B1 SDK internal set | closed | The resolve-graph walk gives 12 crates over the real metadata: core, derived, execution, execution_domain, execution_runtime, execution_store, language, matching, operators, query_language, relations and values. The forwarded-type test is red at `a33f361f7a` and green at `56169f083f`. `sdk_api-at-56169f083f.log` passes the leak and listing steps; semver reads 202 checks, 4 fail, which is the owned red. |
| B2 whole-tree tm | closed | 0 findings and 28 suppressed at the source, from my own run in a clone with no untracked files. |
| B3 status block | closed | 25 mutants (`seam-mutations.json`: 18 caught, 6 equivalent, 1 unobservable). cli_cost is 1 / 3 with 3 not run (`cli_cost.log`). The guard reads 4,348,400 and the Tastemaker rerun 264 / 4. |
| B4 counts | closed | language 49 and python 140, so 189. default_features 2393 / 9 from its result lines. |
| B5 cycles | closed | My Tarjan pass over `ownership_edges.log` finds 914 files, 26,707 occurrences and 0 file-level cycles. |
| B6 census.md | closed | The 334 column is present. Ruling below. |
| B7 clippy ratchet | closed | At `91c711ca46` (fresh run): new 0, resolved 2, accepted 291. At `2eea0b8575`: new 0, accepted 291. All 291 paths are in `rewrite-replaced.toml`, so 0 are outside it. |

**B6 ruling.** The `Relation::X` row is a census.md proxy. It is not in `validation.json` `census.ratcheted`, which lists edit footprint, cross-crate tests outside the replaced set, Operation matches outside it and fact touch points, so the row is informational.

My `git grep` at `40e4902152` and at `333641a62a` shows a rise of 3 files and 4 occurrences. Every one is a `///` doc-example line:
- `executable_program/presence/types.rs`
- `source_partitions/types.rs`
- `file_dag/derived/prepared.rs`

The count of files with a match outside comments is unchanged. A comment-only rise does not count against "census never rising", which binds the ratcheted production rows. These examples do compile as doctests, so a `Relation` rename touches them, as it touches any test.

## New blocker
1. **B8, `crates/ryo/docs/contracts/review/README.md:115-131` and the Verification limits section after it.** The failure: the known-red list is stale and incomplete.
   - It cites "the final checkpoint 333 run, validated source `563c0bc69b`".
   - Its figures are old: cli replay 96 / 3, cli_cost 4 / 3 and the guard at 4,424,456 KiB. The 334 record has 97 / 3, 1 / 3 with 3 not run, and 4,348,400.
   - It still lists the javascript and typescript `manifest_diagnostic` red, which `d0ddb8914a` fixed (68 passed).
   - It omits the `sdk_api` semver red and the `default_features` `allocation::python` red.

   The brief says integration is never red except for this list, so the list is a claim reviewers rely on. Required proof: rewrite both sections from `validation.json` at `333641a62a`, so the list matches the RED lines that `run.py --check` prints. Owner: the integrator.

## Grades
| Axis or item | Grade |
| --- | --- |
| Enforced boundaries | PASS |
| Clear ownership and flow | PASS |
| Proof | FAIL (B8) |
| Change isolation | PASS |
| Items 1, 2, 4, 5, 7, 10, 15, 19, 20, 21 | PASS |

Remaining manual judgment: item 19 (size of the change) and the run.py size advisory (1,102 lines).

## Goal exits
These record progress. They do not gate this checkpoint.

| Exit | Status |
| --- | --- |
| 1 | In progress |
| 2 | In progress |
| 3 | Met |
| 4 | Met at this head (my tm run) |
| 5 | Open, not dispatched |
| 6 | Met as recorded red |
| 7 | Met |

## Gates verified from logs
Each was read from the raw directory `{temp}/ryo-contract-validation334`:
- **Django:** `django-333641a62a/time.txt` reads exit 0, 1:29.75, 2,616,380 KB and load 1.89; the output sha256 is `f50bb571`.
- **Tests:** `language_lib`, `python_lib`, `default_features` (2402 tests run), `cli_cost`, and the Tastemaker rerun (264 / 4).
- **sdk_api:** the run at `56169f083f`, with the red/green logs of its test.
- **Clippy:** the three asymptote clippy JSONs.
- **no_print:** exit 0.
- **Seam table:** `seam-mutations.json`.

## Advisories
- The forwarded-type test is red at `a33f361f7a` only through a `TypeError` from the changed signature, not through a missed leak. The deleted test over the real ryo SDK manifest has no successor that asserts the live 12-crate set.
- `default_features` `checked.count` is 2506 in validation.json, but the log's `running N tests` lines sum to 2402 and its result lines to 2393 + 9. I made one probe and did not find where 2506 comes from. To settle it, read the counter in run.py.
- Matrix status item 1 has a stray space in "1 unobservable : ".
- `tm-steward.json` is committed as FAIL at `333641a62a`. That is the steward's lane, after both reviews.
