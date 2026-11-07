# Integrated full inventory checkpoint

Frozen source `a402537b63a0a96ffd4ff973be610642edf8c26a`, sealed worker `f0e2a143d9eb685ae6bc3a311159c861e0081711bbd16b67069c4e3f4bb6367b`: **406 PASS / 7 FAIL / 413**, no omitted or unrun rules. [Summary](summary.json) and [registration](registration.json) preserve the full result; failure process streams and the exact worker receipt are archived here.

Failures: F541 authoring family admission; PLW0133 and UP035 stack overflow; RUF028 and UP034 dependency NotRun; RUF100 blocked by its RUF028 producer; UP031 immutable archived invocation identity. Later F541 admission repair passes21 cases with the same worker. I001 passed this151-case precursor, but review found N3; source correction and153 cases remain unverified. Subsequent balanced-Union/stream-capture changes compile; targeted runtime and full replay are pending.

This is an incomplete draft checkpoint, not parity, ownership-category or performance acceptance. Historical archives are unchanged.
