# What ryo publishes

This directory is what the release uploads to the package registries other than crates.io; the crates.io package is the command line crate itself.

| Registry | Package | Built from | Proven by |
|---|---|---|---|
| crates.io | `idiomatic-sh-ryo` | `cli/` | `cli/tests/smoke.rs` |

`.github/workflows/release.yml` builds and publishes them.
