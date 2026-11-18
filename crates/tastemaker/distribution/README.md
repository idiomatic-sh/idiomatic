# What tastemaker publishes

This directory is what the release uploads to the package registries other than crates.io; the crates.io package is the command line crate itself.

| Registry | Package | Built from | Proven by |
|---|---|---|---|
| crates.io | `idiomatic-sh-tastemaker` | `cli/` | `cli/tests/smoke.rs` |
| npm | `@idiomatic-sh/tastemaker`, and one `@idiomatic-sh/tastemaker-<target>` per platform | `distribution/npm/`, carrying the PyPI wheel's binary | `distribution/npm/test.rs` |
| PyPI | `idiomatic-sh-tastemaker` | `distribution/pypi/` (maturin) | `distribution/pypi/test.rs` |

`.github/workflows/release.yml` builds and publishes them.
