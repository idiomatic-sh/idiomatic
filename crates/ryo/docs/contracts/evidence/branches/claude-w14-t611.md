# Leak review: claude/w14-t611 (re-review)

Source `bec665436a9ef167601be9a9749bb0fcc64b3908`, base `b8776a549e5852ba55970a2de079f89d09c61669`. The previous review, of `c2ba5669a6`, was FAIL: 2.

## Check 2 Every enforcement claim fires: PASS

**The fix.**
- `stored_of` (`domain/handing/service.rs:241`) stores a part of the record's own file as `OneFile(PartialAt(0))`.
- `partials_of` (`:155`) maps it back to the reusing run's number; `hand_rest_again` passes this run's `number`.
- `interface_bytes` digests `stored_of(.., number)`, so a renumbered file keeps its interface digest, as `moved_words` already does for words.

**The new case can fail.** It lints `[a, b]`, then `[a, a2, b]` over the same store, so `b` is reused at a new number. It compares the edit run's rows, unknowns and kept covered rows with a cold run of the three files.
- On the old code, the edit store keeps `b`'s part at number 1 while the cold store keeps it at 2, so the covered comparison fails.
- In the run itself, `b`'s part would name `a2` and `b`'s Unknown would disappear, so the rows/unknowns comparison fails too.
- A guard assertion requires the cold store to hold a part for `b`, so the case cannot pass vacuously.

**Unchanged.** The `read_alone` fix is as in the first review (correct). The test is unrun because the hook allows only `cargo check`.

## Ruling: a part naming another file, kept repository-wide
The builder's claim holds, so this loses nothing today. A file's parts come from three sources:

| Source | Scope it records |
| --- | --- |
| Engine rows (`parsing/service.rs:354`, `reading/service.rs:124`) | `OneFile` of the file's own number |
| A provider's per-file `partial` (`execution_runtime/src/outbound/facts/types.rs:99-105`) | Repository |
| Recoveries (`coverage/service.rs:239-244`) | Repository |

`partial_at <file>` is the only way to name another file. It appears only in project-scope items (the `manifests/` and `links/` collections). Those parts go to the project phase (`project/service.rs:101-104`), which is recomputed every run and never kept per file.

If a per-file item ever named another file, the widening would make more rules Unknown. That is a coverage change, but the byte checks would see it, because it changes the report's unknowns. It is still wider than the truth, so I offer a mechanical guard: `per-file-partial-names-another-file-2a55efb0`.

## What the integrator must run (updated)
1. `cargo test -p idiomatic-sh-ryo-core --lib -- inbound::use_cases::lint::use_case_test::lifecycle`
2. `cargo test -p idiomatic-sh-ryo-core --lib -- domain::storing domain::handing domain::recording application::file_analysis`
3. **The bead's real reproduction.** Run tm in edit mode over a workspace whose listing puts an excluded file first, and edit a non-first file. The edit run's findings must equal a cold run's.

## Checks 1, 3, 4, 5, 6: PASS
- **1:** the proofs are in core.
- **3:** no visibility change.
- **4:** one position-independent form for a file's own facts (words and parts at 0), and one numbering for the run.
- **5:** direction unchanged.
- **6:** the fixtures are test-only.

## Enforcement candidates
| Candidate | Instance |
| --- | --- |
| `stored-file-number-reused-across-runs-13950b5d` | fixed |
| `per-file-partial-names-another-file-2a55efb0` | open (none today) |

VERDICT: PASS
