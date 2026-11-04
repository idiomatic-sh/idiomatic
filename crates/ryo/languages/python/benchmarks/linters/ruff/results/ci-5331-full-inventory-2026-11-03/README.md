# Exact public 5331 Ruff full inventory

[Run 41700087440](https://github.com/idiomatic-sh/idiomatic/actions/runs/41700087440), job `ruff-all-defaults` 126141689004, completed **409 PASS / 4 FAIL / 413** on source `5331faa5d47c7c9a95ea741e2b763b64c9a48761`. The job remains red because four pinned defaults lack implementations. The [summary](summary.json), [raw registration and receipts](raw-receipts.tar.gz), and [source/tool identities](metadata.json) preserve the shared-runner result. The original GitHub artifact is `ruff-all-defaults-1` ID 13265435059.

Against the previous [408/5 complete run](../ci-5b267-full-inventory-2026-11-03/summary.json), the same 413 rule IDs were executed and only B031 changed from FAIL to PASS. B031 executed 36 cases: 20 raw exact and 16 independently certified reference discrepancies, with zero contract-only cases. No prior PASS regressed. Remaining FAIL IDs: E902, I001, RUF100, RUF200.

Other CI checks and performance remain separate gates. This inventory is not full Ruff parity or merge acceptance.
