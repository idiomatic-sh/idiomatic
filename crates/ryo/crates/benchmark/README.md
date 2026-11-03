# idiomatic-sh-ryo-benchmark

Shared development harness for comparing Ryo with pinned linter defaults.

## Domains

### diagnostics

Canonical finding multisets retain duplicate occurrences, exact paths and byte locations.

### measurements

Process observations separate measured wall time from decoded reports.

For the shipped Rust recording benchmark, check `cargo metadata --locked --no-deps --format-version 1`: package `idiomatic-sh-ryo` has target `lint`, kind `bench`, source `cli/benches/lint.rs` (`harness = false`). Build it with `cargo bench --locked --no-run -p idiomatic-sh-ryo --bench lint`, seal the reported bench executable, then run that executable with `--bench --rounds 1`. The ordinary `ryo` CLI binary is a different target and does not accept those arguments.

### suite

Pinned profiles, authored cases and shared project inputs define the benchmark contract.


The existing `ryo-benchmark-collector` also accepts `verify-inventory <registration.json> <output> <timeout-seconds>`. A linter adapter derives that temporary registration from its pinned `benchmark.toml` and canonical `rules/<ID>/{query.ryoql,cases.toml}` convention, supplying native verifier arguments and exact case identities. The shared domain initializes every default as FAIL; the application invokes the existing verification path for each implementation, retains native observations, and continues after missing or failed rules. The injected artifact boundary atomically publishes `summary.json` and raw per-rule evidence. PASS plus FAIL always equals the supplied inventory length; no provider-specific count or admission list exists in the runner. Existing collector stdin measurement behavior is unchanged when no command is supplied.
