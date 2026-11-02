# CI inventory receipt at 24274af7

The [GitHub Actions run](https://github.com/idiomatic-sh/idiomatic/actions/runs/41533126791) executed the shared benchmark inventory runner at source commit `24274af7b5587d7c4b2563dc846f9550897880c2`. Its [complete summary](summary.json) records **383 PASS / 30 FAIL / 413 pinned defaults**: 19 missing implementations and 11 fixture failures. Every rule reached a terminal status; none timed out. The strict inventory job exited nonzero while rules remain failing.

The [raw receipt archive](raw-receipts.tar.gz) preserves the registration, every per-rule result and available case record, child stdout and stderr, build logs, and executable seals. It contains the same receipt categories as the [481af run](../ci-481af-full-inventory-2026-11-02/README.md). Compared by all 413 rule IDs and failure reasons, only FURB188 changed from FAIL to PASS; no previous PASS regressed. This count is one complete CI inventory execution on the exact source commit. It does not assert full parity or a Rust speed result.

GitHub artifact ID `13183296690` has ZIP digest `sha256:fdeab54466fb4799da998b8a76761b854e7c3fe6c1d8e891e7255b2e7f290143`. The extracted worker SHA-256 matches the summary identity: `2b6d974f06fae12d1c8a7ac2c135af02a6bc7d740fc13649b76202f64149d8c3`. The shared inventory runner SHA-256 is `3ed2e0800a443994641cbc27b301707797d6abb00ae2e7c31da5742663fa4522`.

Repository artifact identities: `summary.json` SHA-256 `a4d50b0bc5cc0a5babaf8714d20ae9dfdb2df43f2ce9df6d24435ebc32f5236e`; `raw-receipts.tar.gz` SHA-256 `ccbc774b1088c09d90ec2f3cda730eae7e9f0c1e42555e183b824f41fac347ba`.
