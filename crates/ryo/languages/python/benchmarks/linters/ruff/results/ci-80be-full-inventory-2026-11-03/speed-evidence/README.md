# Exact 80be speed gate

[Run 41550736630](https://github.com/idiomatic-sh/idiomatic/actions/runs/41550736630) speed job 125637397973 failed. The archived [GitHub artifact](artifact.zip) has the verified SHA-256 in [summary.json](summary.json).

- Rust warm Ruff peak: **101,654,528 bytes**, above the unchanged **100,000,000-byte** guard. Earlier heads also failed this guard; this observation alone does not establish a new regression.
- Go baseline fourfold file read: **3.216174011 s** versus a **0.276318014 s** linear bound. The baseline failure prevents a valid cross-language candidate comparison.

No bound was raised, skipped, or retried to claim success. The 80be full rule inventory is recorded separately in the parent directory.
