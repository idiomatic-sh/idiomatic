# Exact ea787 Ruff full inventory

[Run 41622346356](https://github.com/idiomatic-sh/idiomatic/actions/runs/41622346356), job `ruff-all-defaults` 125870964031, completed **405 PASS / 8 FAIL / 413** on source `ea7874354c56d2a48e0eb9561826d61556abb46f`. The job remains red because eight pinned defaults fail. [Summary](summary.json), [raw registration and receipts](raw-receipts.tar.gz), and [source/tool identities](metadata.json) preserve the complete shared-runner result; the original GitHub artifact is `ruff-all-defaults-1` ID 13225257538.

- Relative to the previous [403/10 complete run](https://github.com/idiomatic-sh/idiomatic/actions/runs/41618035180), F821 (207 cases: 192 raw, 15 documented) and PLW0602 (68 cases: 39 raw, 29 documented) became PASS. No prior PASS regressed.
- Remaining FAIL IDs: B031, E902, EXE001, EXE002, I001, RUF100, RUF200, SIM401.
- The `ruff-candidates` job passed on this exact head after the [F821 evidence-root correction review](source-root-review.json). The historical `--prepared` manifest remains stale and fails closed; this complete full-inventory run does not validate that separate authoring route. Other CI checks and performance are separate gates. This receipt makes no engine speed or merge acceptance claim.
