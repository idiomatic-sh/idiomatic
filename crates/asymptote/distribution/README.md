# What asymptote publishes

This directory is what the release uploads to the package registries other than crates.io; the crates.io package is the command line crate itself.

| Registry | Package | Built from | Proven by |
|---|---|---|---|
| crates.io | `idiomatic-sh-asymptote` | `cli/` | `cli/tests/smoke.rs` |
| npm | `@idiomatic-sh/asymptote`, and one `@idiomatic-sh/asymptote-<target>` per platform | `distribution/npm/`, carrying the PyPI wheel's binary | `distribution/npm/test.rs` |
| PyPI | `idiomatic-sh-asymptote` | `distribution/pypi/` (maturin) | `distribution/pypi/test.rs` |

`.github/workflows/release.yml` builds and publishes them.
