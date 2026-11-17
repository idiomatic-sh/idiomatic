# distribution-wiring 0.1.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. The gated manifests and the configured manifests are joined to the set of manifests with a `distribution` feature, and to the set of manifests that name the distribution, each a projection on `file` (one row per file). The findings and the scenarios are as in 0.1.0.

# distribution-wiring 0.1.0

The first release. Code on an earlier version has nothing to move from.

The distribution test table must set `test = false`, so it runs only when named (`--test distribution`), never in `cargo test` or `cargo test --all-features`.
