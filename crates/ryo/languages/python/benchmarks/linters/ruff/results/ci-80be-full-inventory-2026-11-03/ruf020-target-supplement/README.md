# RUF020 source-bound fixture proof

Both Python3.10 and Python3.12 controls preserve all42 source cases, including the exact full pinned upstream fixture. Each target has33 raw-parity matches and9 exact documented reference differences. Native candidate coverage is complete; no skipped/unknown accounting or general error filter is allowed.

Eight exact cases distinguish PEP695 type parameters from same-spelled bottom-type/Union/module imports. The ninth explicitly adopts lexical static-typing scope in a postponed annotation; its default runtime module lookup and explicit-localns lookup produce different outcomes, both preserved. It is a documented scope-policy difference, not a blanket claim of a Ruff bug. The rule README gives exact reproducers, observed/intended ranges, primary rationale and evidence links for personal examination. No upstream issue was filed.

Actual CPython3.13 controls establish supported-feature namespaces. Separately labeled compatible3.10 controls import NoReturn as Never because typing.Never was added later. Original unsupported3.10 header diagnostics remain; supported3.12 expectations omit those target errors. No original source body was rewritten and no query/provider/engine code changed.

The existing generic validator binds current query, cases, profiles, helpers and actual native attestations. These summaries derive offline from verify-cases receipts, not a collector or combined413 replay. Original failed canonical arrays and full-run observations remain archived, and every earlier published summary is unchanged.
