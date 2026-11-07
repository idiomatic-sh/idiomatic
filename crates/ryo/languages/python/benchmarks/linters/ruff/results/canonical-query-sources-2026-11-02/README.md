# Canonical active query paths

Active Ruff rule source now uses `rules/<ID>/query.ryoql`. The [normalization manifest](normalization.json) records the exact old and active path and SHA-256 for every affected rule: 232 candidate-only files moved to that path, 43 identical duplicate candidate files removed, and seven differing candidate files preserved in the [byte-for-byte archive](divergent-candidates.tar.gz). Existing active queries won every conflict. The frozen profile snapshots and prior execution receipts were not rewritten.

This is a source-path change, not a correctness promotion. The 413-rule result remains **151 PASS / 262 FAIL** until fresh executed evidence changes it. `F621` and `PLE0116` still have cases but no query source and remain FAIL.
