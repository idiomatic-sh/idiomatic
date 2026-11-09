# Leak review: claude/w13-hooks-append-only

Source `b81150e2c66680089b4cf3210f73950df71df85b`, base `e839bb19131f315e1685a9a1b2a572c87b0d33ab` (the live hooks). The only change is the append-only union, in `candidate_problem` and the new `dropped_candidates` of `.claude/hooks/guard_shell.py`, with its tests.

## 1 Proof where the contract lives: PASS
The proofs sit beside the hook. In a clone, test_hooks ran 136 tests OK in 7 of 8 runs. The first run, under load average 14, had 1 error that did not recur in 7 reruns and was not captured. ci/contract_review passed 76.

## 2 Every enforcement claim fires: PASS
`dropped_candidates` (`guard_shell.py:380`) unions the candidate ids over every version from the previous head to the pushed sha. `candidate_problem` (`:397`, `:408`) walks every branch path touched in that range. Results on the PreviousReviewsPush fixture:

| Probe | e839bb1913 | this head |
| --- | --- | --- |
| 1. Added with X, re-reviewed without X | 0 | 2, append-only |
| 2. Added with X, re-reviewed with only a decided Y | 0 | 2, append-only |
| 3. Existing file deleted | 0 | 2, append-only |
| 4. Existing file renamed without X | 0 | 2, append-only |
| 5. Added with X, then deleted inside the checkpoint | 0 | 2, append-only |
| 6. Existing file drops X across the boundary | 2 | 2, append-only |
| Control: X carried as fixed plus Y, both decided | 0 | 0 |
| Control: added with X, decided | 0 | 0 |
| Control: empty file added, then deleted | 0 | 0 |

The four new test rows fail against the e839bb1913 `guard_shell.py` and pass here.

The only other check-2 failures are the hidden-push forms. They are accepted as the frozen backlog:
- `/usr/bin/git push`, `/usr/bin/env git push` and `/bin/bash -c '…'`;
- `dash -c` and `su -c`;
- `setsid`, `chrt`, `taskset`, `doas`, `parallel`, `find -exec` and `builtin command` in front of a push;
- `script -qc '…' /dev/null` and attached `env -S'…'`;
- the inline alias `git -c alias.p=push p`.

## 3 Boundaries hold across crates: PASS
There is no Rust visibility, manifest or import boundary in the diff.

## 4 One implementation and one representation per concept: PASS
`dropped_candidates` is the one drop check; the status-M-only comparison of e839bb1913 is deleted. Candidate ids still come from the one validator, through `service_ids`.

## 5 Layer direction: PASS
There is no crate source in the diff.

## 6 Test seams stay out of production: PASS
There is no test seam.

## Notes
- Deleting a branch file is refused whenever an earlier version carried candidates, even if the steward decided them at an earlier checkpoint. Committed branch evidence is never deleted, so this fails closed by design.
- One test error appeared in 8 runs under heavy load and did not recur. It is worth a rerun at landing.

VERDICT: PASS
