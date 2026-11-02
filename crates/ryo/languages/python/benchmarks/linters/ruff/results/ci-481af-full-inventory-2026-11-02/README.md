# CI inventory receipt at 481af7a8

The [GitHub Actions run](https://github.com/idiomatic-sh/idiomatic/actions/runs/41529492409) executed the shared benchmark inventory runner at source commit `481af7a8b86099ed4cfae8f853ca166c2617ddf0`. Its [complete summary](summary.json) records **382 PASS / 31 FAIL / 413 pinned defaults**: 20 missing implementations and 11 fixture failures. Every rule reached a terminal status; none timed out. The strict inventory job exited nonzero while rules remain failing.

The [raw receipt archive](raw-receipts.tar.gz) retains registration, every per-rule result, available case records, child stdout and stderr, build logs, and executable seals. Fixture failures: F811, F821, F841, N999, PLW0602, PYI019, PYI020, PYI034, RUF020, RUF041, RUF051. The source commit supplies the registered query and helper bytes. GitHub artifact ID `13182357184` has digest `sha256:b0f8e402abc2c2e706f8c319bf7737b8f67dd9e2312c9da88c262095b618d5a0`.

Compared with the archived [982b run](../ci-982b-full-inventory-2026-11-02/README.md), F621 and SIM115 changed from FAIL to PASS; no previous PASS regressed. Unlike the earlier separately linked proofs, this 382 count is one complete CI inventory execution on the exact source commit. It does not assert full rule parity or Rust speed acceptance.

Artifact identities: `summary.json` SHA-256 `6b2f48cba893bc2cfc70a66ad5a1e046b2d7e0302b69886eb7631f3bb1ff588a`; `raw-receipts.tar.gz` SHA-256 `99086675462a59cb576bd6f79c27e149fc214931f02396f4d3d9d416e229201b`.
