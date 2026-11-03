# idiomatic-sh-ryo-benchmark

Shared development harness for comparing Ryo with pinned linter defaults.

## Domains

### diagnostics

Canonical finding multisets retain duplicate occurrences, exact paths and byte locations.

### measurements

Process observations separate measured wall time from decoded reports.

For Rust recordings, check `cargo metadata --locked --no-deps --format-version 1` for the `idiomatic-sh-ryo` target `lint` (`bench`, `cli/benches/lint.rs`, `harness = false`). Build with `cargo bench --locked --no-run -p idiomatic-sh-ryo --bench lint`; run that sealed executable with `--bench --rounds 1`.

### suite

Pinned profiles, authored cases and shared project inputs define the benchmark contract.


The existing `ryo-benchmark-collector` also accepts `verify-inventory <registration.json> <output> <timeout-seconds>`. A linter adapter derives that temporary registration from its pinned `benchmark.toml` and canonical `rules/<ID>/{query.ryoql,cases.toml}` convention, supplying native verifier arguments and exact case identities. The shared domain initializes every default as FAIL; the application invokes the existing verification path for each implementation, retains native observations, and continues after missing or failed rules. The injected artifact boundary atomically publishes `summary.json` and raw per-rule evidence. PASS plus FAIL always equals the supplied inventory length; no provider-specific count or admission list exists in the runner. Existing collector stdin measurement behavior is unchanged when no command is supplied.

## Explicit reference process failures

An authored `reference-discrepancy.process` contract is tagged `kind = "process-failure"` and declares a positive numeric exit, whole stdout/stderr templates, and two distinct archived observations for each exact executable pin. Template parts are literal UTF8 bytes or declared input paths encoded as UTF8 bytes or JSON string content. Existing independent semantic evidence and source/settings/profile/output identities remain mandatory. Signals, spawn failures, deadline observers, unexpected decoded success, and any extra output remain failures. The candidate must still emit its exact intended diagnostics with complete execution accounting.

Verification preserves raw bytes and invocation metadata beside the explicit receipt file in fresh receipt-scoped attempt directories. Unreferenced attempts and resume history remain immutable. Offline validation uses the same shared comparison:

```sh
ryo-benchmark-collector validate-case-receipts registration.json shard.cases.jsonl
```

The command returns verified identities and explicit errors; process differences count within documented discrepancies and never as raw full-report equality. Authoring adapters delegate to this command using a pinned validator executable. They do not match process failures themselves.

For authoring tools that delegate process-receipt checks, set both `RYO_BENCHMARK_RECEIPT_VALIDATOR` to the absolute path of this collector and `RYO_BENCHMARK_RECEIPT_VALIDATOR_SHA256` to the independently recorded SHA256 of its bytes. Missing or incorrect configuration fails receipt validation and is retained in the shard error; it never falls back to local matching. A shard with no receipt is unfinished and is rerun on resume.
