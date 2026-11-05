The directive implementation is shared by three wrappers:

- `suppress` preserves the existing mapping graph and supplies empty additional lines.
- `suppress-unmapped` passes original locations directly; it never constructs a mapping graph.
- `suppress-with-additional-lines` maps primary and additional lines through the same full graph.

The core receives ORIGINAL candidates, mapped primary locations and additional
inline-noqa locations. Range checks still read original candidate IDs. Primary
inline checks use mapped lines. Additional lines hide IDs after primary inline
suppression and before standalone-file suppression, exactly where F401/F822
previously checked parent lines. Extra lines never become diagnostic locations.
The frozen 72ea helper remains `baseline-72ea.ryoql`; this is a distinct identity.

F401/F822 and eight reviewed unmapped callers complete the original inventory:
146 full inline implementations and eight partial variants now use shared modules.
UP005's unused prelude remains separate. The caller preludes, regex literals and
emit clauses are byte-bound by `source-receipt.json`. Seven unmapped directive
bodies matched the original standard body exactly after rule substitution. UP009
used its existing preamble token relation, whose predicate is exactly the shared
code-token predicate (its extra file column was not consumed by the range projection).
`extraction.py` verifies 165 unchanged directive bindings, 17 unchanged mapping
bindings, original/mapped roles and the unmapped graph's absence.

The immutable run `/tmp/suppress-common-consumers-20261101-v2` checks all 310 active
current callers / 7,171 unchanged cases / 14,342 real baseline/candidate records.
Every process exited zero; full reports were exactly equal, with zero Unknown.
Six pre-existing expectation differences remain in F841, F842, PLW0602 and PYI020;
complete expected/actual arrays remain in `proof.json`. No Ruff was rerun. This is
migration equivalence, not full 413-rule parity or a wall-time claim. All effective
phase/target/context/flags are compared with captured canonical profiles and the
pinned registry. EXE005's 26 cases and PYI033's 18 cases explicitly use `always`,
supplementing the original 142-rule author run; see `phase-supplement.json`.

The all-consumer source staging was `/tmp/suppress-common-consumers-source-20261101`.
Its baseline uses frozen 72ea and the previously accepted SIM220/SIM221 adapters;
only ten caller queries change. All other caller identities are unchanged. All
registered module bytes, canonical profile settings, cases and actual requests are
bound before execution and rechecked by the shared v2 sealer. An initial staging
attempt refused the previously unreviewed `deferred` suite-accounting table before
launching any rule-case workers; it remains explicitly rejected at
`/tmp/suppress-common-consumers-20261101/REJECTED.txt`.

`boundaries.py` checks 23 boundary cases / 46 side records, including four explicitly
designated partial-source refusal pairs. Successful cases require the strict full
report validator. Nonempty controls distinguish unmapped continuation behavior
(one diagnostic) from mapped behavior (none); additional-line tests cover empty,
multiple lines with any hidden line, duplicate IDs, wrong codes and two files.
Evidence: `/tmp/suppress-common-core-boundaries-20261101`. The executed positive
compiled plan retains the left-seeded code/range anchor chains sharing one deduped
candidate-file group (step38); output projections stay 3/8 columns. `plan-proof.json`
claims executed chains only; the full source producer inventory is checked separately.
