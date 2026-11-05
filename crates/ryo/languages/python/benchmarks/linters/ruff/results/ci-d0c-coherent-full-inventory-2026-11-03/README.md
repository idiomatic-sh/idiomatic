# Coherent owner and rule candidate: full Ruff inventory

- [CI run 41598314675](https://github.com/idiomatic-sh/idiomatic/actions/runs/41598314675) used clean compute source [d0c70917](https://github.com/idiomatic-sh/idiomatic/commit/d0c709171c0a77b19018b99c74ed10596fda3a23): corrected owner checkpoint plus reviewed UP031/UP034/N999 rule sources. This is an isolated compute branch, not the accepted inventory pointer or PR head.
- [Compact summary](summary.json): 400 PASS / 13 FAIL across all 413 rules, with zero per-rule verdict changes from the prior UP034 source run. UP034 is 19/19 raw exact; N999 is 55/55 raw exact. Full CI summary SHA-256 `e64034a0f665afc4d9b0d879e0609daa565429028d09129a4372331971af6bef`; the CI artifact retains the full report, per-rule raw receipts and worker/tool seals.
- Remaining FAIL IDs: B031, E902, EXE001, EXE002, F821, I001, PLW0602, PYI020, RUF100, RUF200, SIM401, UP031, UP032. The Python 3.14 UP034 counterprofile is a separate gate.
