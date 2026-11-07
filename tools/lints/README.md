# lints

The repository's one dylint library. It holds only the lints a Tastemaker rule cannot
express because they need the compiler's resolved types; every other custom check is a
Tastemaker rule in `tastemaker/rules/`. Each product's `asymptote.toml` runs it
(`cargo dylint --path ../../tools/lints`), placed by the `asymptote-config` scaffold.

| lint | enforces |
|---|---|
| `nominal_boundary_type` | a `*Interface` trait in an `interface` module, and each implementation of it, never resolves to one of the representation types its own policy names (`src/boundary/types.rs`), however the type is written (aliases included); no configuration file changes the policy |
| `unproven_boundary_type` | an implementation's boundary type the compiler cannot prove nominal (a generic receiver whose bound admits a primitive) |

```sh
cargo test
```
