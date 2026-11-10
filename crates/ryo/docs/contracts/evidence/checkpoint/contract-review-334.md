# Contract review, checkpoint 334 (delta re-review): FAIL

Source `a33f361f7a7f1421358fe006a7b88fa5f9456ff4`; frozen validation at `333641a62a`; base `40e4902152`. The delta is `c00074ffe6` (one line per validation.json entry), `b6aff49ae3` (sdk_api derives its internal crates) and the record commits `a01a31de23` and `a33f361f7a`. It changes no engine source. Under the PM ruling, the frozen run plus the sdk_api rerun stands. This pass was read-only.

## Blockers
| Blocker | Status | Evidence |
| --- | --- | --- |
| B1 SDK leak gate | closed | `internal_crates()` reads every path dependency from the SDK manifest. Its test is red at `c00074ffe6` and green at `b6aff49ae3`. The rerun at `b6aff49ae3` passes the leak step and the listing; its exit 1 is the owned semver red (202 checks, 4 fail). |
| B2 whole-tree tm | closed | The integrator's run at `a33f361f7a` has 1 finding, in the untracked `contract-review-334-early.md`, which is not in the source, so the committed tree is at 0 findings with 28 suppressed. validation.json is 316 lines. |
| B3 status block | **open** | README lines 16-21 are unchanged: they still say "F3 part 2 is not merged yet ... 29 against 24", "22 planted mutants", "met at 293766d1a3" and "cli_cost 4 / 3" / 4,424,456. |
| B4 counts | **open** | The record still says language 42 / total 182. `language_lib.log` shows 49 passed, so the total is 189. |
| B5 cycles | **open** | `ownership_graph` still has source `563c0bc69b`. My recount at `333641a62a` gives 0 cycles, but it is not recorded. |
| B6 census.md | **open** | Line 49 still ends at the 333 column. |
| B7 clippy ratchet | **open** | No clippy group run since `9e03aa81fe`, and no log for one in the validation directory. |

The integrator owns B3 to B7. Each is a record or measurement step; none needs an engine change.

## Items
- Enforced boundaries PASS.
- Clear ownership and flow PASS.
- Proof FAIL (B3, B4, B5, B7).
- Change isolation FAIL (B6).
- Items 1, 2, 4, 5, 7, 10, 15, 19, 20 and 21 PASS.

## Goal exits
| Exit | Status |
| --- | --- |
| 1 | In progress |
| 2 | Met |
| 3 | Met |
| 4 | Met on the committed tree, but the matrix still dates it to `293766d1a3` |
| 5 | Open: no dispatch |
| 6 | Met as recorded red |
| 7 | Met |

## Advisories
- Record `tm_whole_tree` from the 334 run; it still holds the 333 numbers.
- `contract-review-334-early.md` names a machine path; strip it before it is committed.
- Leak review: plant a type from a crate the SDK reaches only transitively (execution_domain, execution_store, derived) against the derived internal set.
