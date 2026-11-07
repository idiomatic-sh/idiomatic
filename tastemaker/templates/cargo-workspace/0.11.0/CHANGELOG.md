# cargo-workspace

## 0.11.0

Edited in place: the workspace declares no `[workspace.metadata.bin]`. The tools every product runs are declared once,
in `tools/install.py`'s `ROOT_CRATES`, and a tool only one product runs stays in that product's table, which the template
neither requires nor refuses; the `no-tool-pins` and `no-cargo-*-pin` scenarios are gone.

`[profile.dev.package."*"]` builds every dependency at `opt-level = 1`, below the product's own `opt-level = 2`: a
cold build of the dependencies costs less, and the product's code, which its tests exercise, stays optimized.

The decisions every product made by hand are the template's: `wildcard_enum_match_arm`, `as_conversions`, `unwrap_used`
and `expect_used` at deny; `repository` in `[workspace.package]`; the `gate` and `dev` profiles (`opt-level = 2`,
line tables only), and, for a product with a distribution, the `release` (fat LTO, one codegen unit) and `dist` profiles.

Every rustc and rustdoc lint, and a chosen set of clippy's restriction lints, is on in `[workspace.lints]`:

- `[workspace.lints.rust]`: the groups `unused`, `nonstandard_style`, `let_underscore`, `rust_2018_idioms`, `keyword_idents`,
  `future_incompatible` and `refining_impl_trait`, and every stable allow-by-default lint of `rustc -W help` at the pinned
  toolchain (1.97.1), `missing_docs` among them. The comment above them names what is left out and why.
- `[workspace.lints.clippy]`: `missing_docs_in_private_items`, so every item is documented, private ones included; each
  restriction lint chosen by name (never the group), the ones left off named with their reason; and the `cargo` group,
  `multiple_crate_versions` allowed (no product can unify its dependencies' majors). `absolute_paths` is off, and so are
  `panic_in_result_fn` and `unwrap_in_result` (a case table's subject returns the error under test, so a fixture that
  fails must panic) and rustc's `variant_size_differences` (a size against an allocation is the performance pass's
  trade). `nursery` stays off.
- `[workspace.lints.rustdoc]`, new: every stable rustdoc lint, at `deny`. Only a documentation build reports them
  (`asymptote-config` 0.6.0's `cargo-doc`), and that tool is judged by its exit status alone: Cargo passes them to
  rustdoc as `-D rustdoc::<lint>`, with no `RUSTDOCFLAGS`, so a rustdoc warning fails the build.

Every other new lint is at `warn`, `missing_docs` and `missing_docs_in_private_items` included, so a product moves up
before its findings are fixed: clippy reports each to the ratchet, and each is raised to `deny` once its product has none.

`[workspace.metadata.bin]` pins `cargo-llvm-cov` `0.9.1` and `cargo-msrv` `0.19.6`, which `asymptote-config` 0.6.0's
`coverage` and `cargo-msrv` run.

New fixtures `no-missing-docs`, `no-private-docs`, `no-rustdoc-lints`, `rustdoc-lints-warn`, `no-cargo-llvm-cov-pin` and
`no-cargo-msrv-pin`.

### Moving up from 0.10.0

Copy the three lint blocks and the two pins from the template (`tm workspace scaffold idiomatic.arch/cargo-workspace
<product>/Cargo.toml` prints them as a diff). Each finding they raise is fixed, or ignored in the source with its reason
(`#[expect(<lint>, reason = "...")]`); none is accepted into the lock.

## 0.10.0

`hyperfine` is no longer required in `[workspace.metadata.bin]`: the repository root declares it once, so `tools/install.py` installs it for every product, and the `distribution` bench runs the bare command. The fixture `no-hyperfine-pin` is removed.

## 0.9.0

`divan` is no longer required in `[workspace.dependencies]`: the performance benches print the Bencher Metric Format with their own harness, and asymptote judges it.

`license` is the SPDX id `MIT`, not `mit`, so crates.io, npm and PyPI read it as a license.

`[workspace.package]`'s `version` is the entry's `version` param, no longer `0.1.0` for
every product: a product raises its version whenever its sources change, so one version is
one build and a machine installs a new build by its version. New
fixtures `own-version` (passes) and `unraised-version` (a violation).

`clap` is built with `color` and `wrap_help` beside `derive`, so help is styled on a
terminal and wrapped to its width. A dependency's feature
list is not part of the template's contract, so a product that turns them off is held by the
`cli` spec's generated `command-line-contract` test, which needs styled help.

`[workspace.dependencies]` pins `indicatif` `=0.18.6`, with which the `cli` spec's `output`
module draws a long command's progress line on a terminal (clig.dev, Robustness;
), and `snapbox` `0.6`, with which its `command-tree-contract` test holds
the command surface to `tests/command_tree/surface.txt`.

`members` lists each crate the product's `tastemaker.toml` places, by its path, rendered
from `instances.crate`. 0.8.0 wrote `["cli", "crates/*/*"]`, a glob
cargo refuses while any `crates/<crate>` is a lone crate or no grouped crate exists
("failed to load manifest"), and that misses every lone crate. `[workspace.dependencies]`
pins `trycmd` `=0.15.11`, with which a command line crate runs its product README's
Usage (`crate` 0.12.0). New failing fixture `no-trycmd-pin`. Membership itself is checked
by the `workspace-membership` rule. Everything else is 0.8.0's.

### Moving up from 0.8.0

`tm workspace update` takes `cargo-workspace` 0.9.0. List each crate in `members`
(`tm workspace scaffold idiomatic.arch/cargo-workspace <product>/Cargo.toml`
prints the list as a diff), and add `trycmd = "=0.15.11"`, `indicatif = "=0.18.6"` and
`snapbox = { version = "0.6", features = ["cmd"] }` to `[workspace.dependencies]`. Give
the `cargo-workspace` entry `params = { version = "<the product's version>" }`.

## 0.8.0

`[workspace.dependencies]` pins the crates `cli` 0.9.0's `src/output.rs` prints with: `anstream` `=1.0.0`, `serde` `=1.0.229` with `derive`, and `serde_json`
`=1.0.151`, as a table so a product may add features (Tastemaker and ryo turn on
`preserve_order`). A product generated from the devtool starter then compiles its CLI.
New failing fixture `no-output-dependencies`. Everything else is 0.7.0's.

### Moving up from 0.7.0

`tm workspace update` takes `cargo-workspace` 0.8.0. Add `anstream = "=1.0.0"`, and
write `serde` and `serde_json` as tables with those exact versions (`serde_json = { version
= "=1.0.151" }`), keeping any features the product turns on.

## 0.7.0

Drops `disallowed_methods`, `disallowed_macros` and `disallowed_types` from
`[workspace.lints.clippy]`, turns on clippy's groups in full, and pins `clippy-sarif`. Our own checks are Tastemaker rules, never clippy configuration, so `clippy.toml` configures no disallowed path
(`clippy-config` 0.3.0) and those lints have nothing to check.

Clippy keeps its shipped lints:
`correctness` (deny, its default), `suspicious`, `complexity`, `perf`, `style` and
`pedantic` (warn), each at `priority = -1`, with no allow-list of pedantic lints
(`missing_errors_doc`, `missing_panics_doc`, `module_name_repetitions` and
`must_use_candidate` are gone). A warning is a finding asymptote ratchets through
`clippy-sarif` 0.8.0 (`asymptote-config` 0.4.0), pinned in `[workspace.metadata.bin]`
with `default-features = true`: it ships no prebuilt binary for every host (none for
aarch64 Linux), and cargo-run-bin 1.7.5 builds a pin that names its features with
`cargo install` instead of `cargo binstall`, whose source fallback refuses the install path.
`unknown_lints` is denied: rustc reports an unknown lint name without a source span, and
`clippy-sarif` drops a diagnostic without one, so the run fails instead of losing it.
Everything else is 0.6.0's.

Moving up from 0.6.0: delete the three `disallowed_*` lines and the pedantic allow-list, set
the six groups and `unknown_lints = "deny"`, add the `clippy-sarif` pin, and baseline the new
findings in the asymptote lock.

## 0.6.0

`[workspace.metadata.bin]` pins every command-line tool a product runs, the same in every
product. Beside
`cargo-deny`, `cargo-nextest` and `cargo-machete` it pins `cargo-hack` 0.6.45 (the feature
combinations, run on main every three landings), `cargo-mutants` 27.1.0 (the mutation gate, run as
`cargo mutants` through the `cargo-config` 0.2.0 alias) and `hyperfine` 1.20.0 (a command
bench that runs the product's binary, `performance-main` 0.3.0), all `locked`. Each pin is
required.

Moving up from 0.5.0: add the three pins to `[workspace.metadata.bin]`.

## 0.5.0

`dead_code = "deny"` in `[workspace.lints.rust]`: a
field or item nothing reads, such as a placeholder setting added only to fill a settings
struct, fails the build instead of warning. Everything else is 0.4.0's (the `cpu-time` pin, no `trybuild`, `cargo-machete`).

Moving up from 0.4.0: add `dead_code = "deny"`.

## 0.4.0

No `nix`. The workspace pins `cpu-time`, which the benches read this
process's CPU time through (`crate-bench` 0.3.0), in its place, and no longer pins
`trybuild`, which only a workspace with a macro crate uses and pins itself.
`[workspace.metadata.bin]` also pins `cargo-machete` 0.9.2, `locked`, which finds a
dependency a package declares and never uses.

Moving up from 0.3.0: in `[workspace.dependencies]`, replace the `nix` pin with
`cpu-time = "=1.0.0"`, and drop `trybuild` unless a macro crate dev-depends on it; add
`cargo-machete = { version = "0.9.2", locked = true }` to `[workspace.metadata.bin]`.

## 0.3.0

Pins `nix` with its `resource` and `time` features, which the benches read CPU time and
peak RSS through.

Moving up from 0.2.0: add the `nix` pin to `[workspace.dependencies]`.

## 0.2.0

Adds `[workspace.metadata.bin]`, the command-line tools the workspace runs, pinned for cargo-run-bin: `cargo-deny` 0.20.2 and `cargo-nextest` 0.9.146, both `locked`. The `cargo-config` template's aliases run them as `cargo deny` and `cargo nextest`, installing the pinned version into the workspace's `.bin/` on first use. To move up from 0.1.0, add the table; a workspace may pin more tools beside these.

## 0.1.0

The first v2 release: a product's Cargo workspace: members, shared package fields and dependencies, and the lints every crate inherits.

The system allocator: no `mimalloc` pin in `[workspace.dependencies]`; a workspace may still carry it (the template accepts extra content).
