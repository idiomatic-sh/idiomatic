# Exact 92ac Ruff full inventory

[Run 41618035180](https://github.com/idiomatic-sh/idiomatic/actions/runs/41618035180), job `ruff-all-defaults` 125856306432, completed **403 PASS / 10 FAIL / 413** on source `92ac15fc1b02d712f6cd3c394678ab8cc64acf10`. The job remains red because ten pinned defaults fail. [Summary](summary.json), [raw registration and receipts](raw-receipts.tar.gz), and [source/tool identities](metadata.json) preserve the complete shared-runner result; the original GitHub artifact is `ruff-all-defaults-1` ID 13222079294.

- Relative to the previous [400/13 complete run](https://github.com/idiomatic-sh/idiomatic/actions/runs/41612985980), PYI020 (75 cases), UP031 (43; 41 raw and two documented process differences), and UP032 (30 raw) became PASS. No prior PASS regressed.
- Remaining FAIL IDs: B031, E902, EXE001, EXE002, F821, I001, PLW0602, RUF100, RUF200, SIM401.
- The `ruff-candidates` job passed on this head. Other CI checks and performance are separate gates. This receipt makes no engine speed or merge acceptance claim.
