# Exact bec4 Ruff full inventory

[Run 41635290943](https://github.com/idiomatic-sh/idiomatic/actions/runs/41635290943), job `ruff-all-defaults` 125914901161, completed **407 PASS / 6 FAIL / 413** on source `bec4bb1cb2eb455d651639de7b323a17f1c4f281`. The job remains red because six pinned defaults have no implementation. The [summary](summary.json), [raw registration and receipts](raw-receipts.tar.gz), and [source/tool identities](metadata.json) preserve the complete shared-runner result. The original GitHub artifact is `ruff-all-defaults-1` ID 13230938418, SHA-256 `9e0ff79385a4fef589dc7510d33e72ad883a344f122e0da6e89ccc06eea9cc12`.

Relative to the previous [405/8 complete run](../ci-ea787-full-inventory-2026-11-03/summary.json), only EXE001 and EXE002 moved from FAIL to PASS. Each executed 22 raw-exact cases with no documented discrepancy or contract-only case. No prior PASS regressed. Remaining FAIL IDs: B031, E902, I001, RUF100, RUF200, SIM401.

This inventory ran before the benchmark-only optional observation-subset correction at `89ee91cc2c68dda2f842137e496fff61924a8047`. That correction passed its focused benchmark library gate and scoped source review; the complete-profile EXE query, cases, and provider behavior are unchanged. Other CI checks and performance remain separate gates. No full Ruff parity or merge acceptance is claimed.
