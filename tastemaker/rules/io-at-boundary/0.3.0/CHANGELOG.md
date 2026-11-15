# io-at-boundary 0.3.0

A benchmark linter's `host.rs` is no longer a boundary. The shared benchmark harness observes the host (`outbound/host`, an adapter, which this rule already places), so no linter bench has a `host.rs`.

- The query no longer reads the `benchmark-linter` spec's `host-probe` placement (`host-roles`) and no longer exempts `host.rs`; the prose no longer names it.
- Fixture `benchmark-linter-host-probe-boundary` is removed with it: it placed a `benchmark-linter` of `idiomatic.ryo`, a spec this rule no longer reads.
- `idiomatic-rust` 0.1.15, `nu-hex` 0.3.6 and `language` 0.4.10 place this version; `idiomatic-rust` 0.1.0, `nu-hex` 0.2.0 and the products that lock them keep 0.1.0. A product that places these locks one version of the rule, so all three moved together.

# io-at-boundary 0.2.0

A benchmark linter's `host.rs` is a boundary. The `benchmark-linter` spec (idiomatic.ryo) places it as the linter's host probe and says "environment and filesystem observation belong in host.rs", the observation at the linter process edge that supplies facts to the applicability decision; 0.1.0 counted only the placed adapter roles and a benchmark executable's workflow as boundaries, so the probe's five `std::fs` reads were findings against the spec's own placement.

- The query reads `host.rs` of a file the `benchmark-linter` spec placed as a boundary, beside the placed adapter and workflow roles. An unplaced `host.rs`, and every other source of the linter, is still outside.
- Fixture `benchmark-linter-host-probe-boundary`: the placed `host.rs` passes; the linter's `settings.rs` and an unplaced `host.rs` are findings.
- `idiomatic-rust` 0.1.10 places this version; `nu-hex` 0.2.0 and `language` 0.4.2 keep 0.1.0.
