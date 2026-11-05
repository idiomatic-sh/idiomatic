# Exact 37e speed gate

[Run 41556074310](https://github.com/idiomatic-sh/idiomatic/actions/runs/41556074310) speed job 125655494262 failed. The archived [GitHub artifact](artifact.zip) has the verified SHA-256 in [summary.json](summary.json).

- Rust warm Ruff peak: **100,974,592 bytes**, above the unchanged **100,000,000-byte** guard. Earlier heads also failed this guard; this observation alone does not establish a new regression.
- Go baseline fourfold file read: **3.233017623 s** versus a **0.279896586 s** linear bound. The baseline failure prevents a valid cross-language candidate comparison.

No bound was raised, skipped, or retried to claim success. The 37e full rule inventory is recorded separately in the parent directory.
