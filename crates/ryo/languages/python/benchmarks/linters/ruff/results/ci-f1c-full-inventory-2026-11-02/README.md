# CI inventory receipt at f1c0d0e5

The [GitHub Actions run](https://github.com/idiomatic-sh/idiomatic/actions/runs/41519435286) executed the shared benchmark inventory runner at source commit `f1c0d0e53ac54367583b0460a56acf96ea415611`. Its [complete summary](summary.json) records **379 PASS / 34 FAIL / 413 pinned defaults**: 23 missing implementations and 11 fixture failures. Every rule reached a terminal status; none timed out. The inventory job exited nonzero, as required while rules remain failing.

The [raw receipt archive](raw-receipts.tar.gz) retains registration, every per-rule result, available case records, child stdout and stderr, build logs, and executable seals. The 11 fixture failures are F811, F821, F841, N999, PLW0602, PYI019, PYI020, PYI034, RUF020, RUF041, and RUF051. The source commit supplies the exact registered query and helper bytes.

This is one execution of `f1c0d0e5`. It predates the RUF009 source and its separate 26-case proof, so its 379 PASS count must not be relabeled as a 380-rule replay. Compared with the archived [6bff run](../ci-6bff-full-inventory-2026-11-02/README.md), 18 additional rules passed and no prior PASS regressed; the earlier 22 timeouts are absent. Rules that previously timed out can now expose fixture failures.

Artifact identities: `summary.json` SHA-256 `b3dbfbe93ce15ca10b6874aef1f191c3108e7a471804a16a3028bab48aac2288`; `raw-receipts.tar.gz` SHA-256 `efa9ab1e9f64e4818ba048cc459925ad7d6fc116e6ade09622ebc23a59a31140`.
