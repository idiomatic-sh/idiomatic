# clippy-config

## 0.3.0

Only the in-tests allowances. Our own checks are Tastemaker rules, never clippy
configuration. Each removed entry's rule:

| removed | now |
|---|---|
| `disallowed-methods`: the `std::fs` functions, `Path::{exists,is_file,is_dir}`, `cap_std`'s `open_ambient_dir`, `std::env::current_dir` | the `io-at-boundary` rule |
| `disallowed-methods`: `std::thread` and rayon pools | the `threads-at-adapter` rule |
| `disallowed-methods` and `disallowed-macros`: `serde_json` | the `no-json-in-process` rule |
| `disallowed-types`: the primitives, `String`, the std collections and `Option` | the policy of `tools/lints`' nominal-interface lints, which need resolved types and no longer read `clippy.toml` |

The devtool starter's shared rule block places the three rules. Added
`allow-indexing-slicing-in-tests`, which the retired `macro-clippy-config` carried: a macro
crate reads this file now, and its crate root denies `indexing_slicing`.

### Moving up from 0.2.0

Place `clippy.toml` with `version = "0.3.0"` and delete its `disallowed-*` lists, then every
`#[allow]`, `#[expect]`, `#[deny]` or `#[forbid]` of `clippy::disallowed_methods`,
`disallowed_types` or `disallowed_macros` in the code, and those lints' levels in
`[workspace.lints.clippy]` (`cargo-workspace` 0.7.0). Take the rules through the devtool
starter's shared rule block.

## 0.2.0

Comments only: environment reads are the `no-env-reads` Tastemaker rule's, and
`disallowed-types` is the input of the nominal-interface lints in `tools/lints`, the
repository's one dylint library. The `crates/tastemaker/lints` libraries are gone. The settings are unchanged.

## 0.1.0

The first v2 release: the repository root `clippy.toml`, with the test allowances, the
`disallowed-methods` and `disallowed-macros` for boundary I/O, threads and rayon pools and
in-process JSON, and the `disallowed-types` the nominal_types dylint reads. Environment
reads are the `no_env_reads` dylint lint's, which no `#[allow]` reaches.
