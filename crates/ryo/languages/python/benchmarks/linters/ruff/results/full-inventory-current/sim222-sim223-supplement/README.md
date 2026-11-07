# SIM222 and SIM223 source supplement

Both rules pass **17/17 independent cases** against Ruff 0.16.5 and the same sealed Python worker as the complete 413-rule run. All 34 case pairs are raw exact; no discrepancy certificate or expectation waiver was used. The four added cases per rule check Boolean-test context, a known prefix without duplicate findings, a parenthesized decisive literal, and a decisive literal before the final operand.

The [shared collector summary](summary.json) and [raw receipts](raw-receipts.tar.gz) bind query, case, profile, worker, and imported helper identities. [Proof metadata](proof.json) binds this separate two-rule result to the immutable 362/51 full-run summary. The current [413-row table](../../../RULES.md) applies only these two new PASS outcomes to that base; it does not claim one combined execution.
