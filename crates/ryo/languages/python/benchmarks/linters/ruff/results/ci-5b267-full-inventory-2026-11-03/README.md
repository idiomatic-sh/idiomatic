# Exact public 5b267 Ruff full inventory

[Run 41658909320](https://github.com/idiomatic-sh/idiomatic/actions/runs/41658909320), job `ruff-all-defaults` 125996608309, completed **408 PASS / 5 FAIL / 413** on source `5b26737babfd9ef9440992179fb7b856b38e0f4a`. The job remains red because five pinned defaults lack implementations. The [summary](summary.json), [raw registration and receipts](raw-receipts.tar.gz), and [source/tool identities](metadata.json) preserve the complete shared-runner result. The original GitHub artifact is `ruff-all-defaults-1` ID 13243055264.

Against the previous [407/6 complete run](../ci-bec4-full-inventory-2026-11-03/summary.json), the same 413 rule IDs were executed and only SIM401 changed from FAIL to PASS. SIM401 executed 20 raw-exact cases with no documented discrepancy or contract-only case. No prior PASS regressed. Remaining FAIL IDs: B031, E902, I001, RUF100, RUF200.

Other CI checks and performance remain separate gates. This inventory is not full Ruff parity or merge acceptance.
