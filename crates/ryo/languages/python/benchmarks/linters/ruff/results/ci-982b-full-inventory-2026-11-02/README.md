# CI inventory receipt at 982b0e29

The [GitHub Actions run](https://github.com/idiomatic-sh/idiomatic/actions/runs/41527208203) executed the shared benchmark inventory runner at source commit `982b0e29b4a568a6f32867a8c5a779423c23732e`. Its [complete summary](summary.json) records **380 PASS / 33 FAIL / 413 pinned defaults**: 22 missing implementations and 11 fixture failures. Every rule reached a terminal status; none timed out. The strict inventory job exited nonzero while rules remain failing.

The [raw receipt archive](raw-receipts.tar.gz) retains registration, every per-rule result, available case records, child stdout and stderr, build logs, and executable seals. Fixture failures: F811, F821, F841, N999, PLW0602, PYI019, PYI020, PYI034, RUF020, RUF041, RUF051. The source commit supplies the registered query and helper bytes. The GitHub artifact ID is `13181211845`, digest `sha256:5adf4576495b3a8e7cc00869fd2592632dadee6a9e8824b6a4e40163f9bd9f83`.

Compared with the archived [f1c run](../ci-f1c-full-inventory-2026-11-02/README.md), only RUF009 changed from FAIL to PASS; no previous PASS regressed. This is one execution of `982b0e29` and predates the separately validated SIM115 and F621 source/proof supplements. Its 380 count must not be relabeled as a combined replay at the later 382-rule checkpoint.

Artifact identities: `summary.json` SHA-256 `85049bef9504bbe53b338d9861315a7b109984f200304dd74af0b3c33167bb1d`; `raw-receipts.tar.gz` SHA-256 `64d3dba4ddbe18bacb28f4fb039b907f70d414998baf47a2a3d265b8fdf79cb4`.
