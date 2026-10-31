# asymptote-config

## 0.6.0

The groups follow ruff's hook, CI and release, with Asymptote as the driver: `lint` is the git hook and compiles nothing (`cargo-fmt`, `tastemaker`, `cargo-machete`, `cargo-deny`); `check` is
the landing gate (`cargo-clippy`, `sdk-api`, `test-core`, `test-cli`, `test-bdd`, `test-doc`, `cargo-doc`); `release` runs
before a publish (`cargo-clippy-default`, `test-default`, `cargo-hack`, `cargo-dylint`, `coverage`, `cargo-msrv`,
`test-distribution`, `release-smoke`, `fresh-resolution`), split into one group per independent build so each runs as a job of
its own: `release-tests` (`cargo-clippy-default`, `test-default`, `test-distribution`, `release-smoke`, the one stable build they
share), `release-coverage` (`coverage`), `release-msrv` (`cargo-msrv`), `release-dylint` (`cargo-dylint`), `release-hack`
(`cargo-hack`) and `release-resolution` (`fresh-resolution`); `asymptote check --groups release-tests,release-coverage,release-msrv,release-dylint,release-hack,release-resolution`
runs all of them locally. `cargo-clippy` is the `clippy` group, `cargo-clippy-default` the `release-clippy` group and `sdk-api`
the `api` group, each run only where its tool ships a prebuilt (clippy-sarif: Linux x64 and macOS arm64; cargo-public-api: macOS). `perf`
is unchanged. `fresh-resolution` packages each published crate
without the workspace's `Cargo.lock` and builds it from its package, so its dependencies resolve as a user's do. The `test` group is gone. Clippy runs without `--jobs 2`.

`test-all-features` is split by area: `test-core` and `test-cli` run `cargo nextest run --workspace --locked
--all-features`, and a product names each one's packages after `-E` (`"-E", "package(=<package>)"`), with the inputs of
that area alone, so an edit reruns only the tests it can change. A product with more areas adds a tool for each, and one
whose inputs capture a directory (`languages/{lang}`) is one tool per directory (the scenarios `no-test-core` and
`no-test-cli`).

`tastemaker-validate` is gone: `tm check` first proves every definition whose proof is missing or stale, as `tm validate` does, and a definition whose own scenarios fail stops the check with an error (exit 2) that names it. `tm validate` stays the author's command.

`cargo-clippy-default` runs clippy with the default features beside `cargo-clippy`'s `--all-features` run, in the lint
group: code a feature turns off is linted as a user builds it (the scenario `no-cargo-clippy-default`).

Each tool names the inputs every product gives it: the product, and the toolchain, Cargo, clippy, rustfmt, lint and
check-script files of the repository its result depends on.

The agreed quality tools run in every product's test group:

- `test-default` and `test-all-features` run `cargo nextest run` instead of `cargo test`. nextest cannot list the cucumber
  runners or the doctests, so two tools run them: `test-bdd` (`cargo test --test bdd`) and `test-doc` (`cargo test --doc`).
- `cargo-doc` builds the documentation (`cargo doc --workspace --no-deps --all-features`): the rustdoc lints of
  `[workspace.lints.rustdoc]` are reported only by a documentation build.
- `coverage` runs `cargo llvm-cov`, a report with no threshold: it fails only when a test does.
- `cargo-msrv` runs `cargo msrv verify --path cli -- cargo check --workspace ...`: the declared `rust-version`, read from the
  command line crate every product has, checks the whole workspace.

`cargo-dylint` runs in the test group, not the lint group the git hook runs. `cargo-check` is gone: clippy runs with `--all-features` and reports every
compiler warning, and `cargo-hack` and `test-default` build the default features.

New fixtures `no-test-bdd`, `no-test-doc`, `no-cargo-doc`, `no-coverage` and `no-cargo-msrv`: a product missing any of them
is a finding.

### Moving up from 0.5.0

- Set each tool's `group` as the template writes it, drop `--jobs 2` from the clippy runs, and replace
  `test-all-features` with `test-core` and `test-cli`, each naming its packages after `-E`.
- Remove the `cargo-check` tool, set `group = "test"` on `cargo-dylint`, and add `cargo-clippy-default` as the template writes it.
- Change the `run` of `test-default` and `test-all-features` to `["cargo", "nextest", "run", ...]` as the template writes it.
- Add `test-bdd`, `test-doc`, `cargo-doc`, `coverage` and `cargo-msrv` from the template.
- Pin `cargo-llvm-cov` and `cargo-msrv` in `[workspace.metadata.bin]` (`cargo-workspace` 0.11.0), and add `llvm-tools` to
  the toolchain's components.

## 0.5.0

Edited in place a third time: asymptote assumes nothing about how a tool is installed. Every `run` and `fix` names a bare command on the PATH, with
no version and no launcher: `["tm", "validate"]`, `["clippy-sarif"]`, `["cargo-deny", "check", ...]`, never `cargo bin ...` or `uvx --from x==1.2.3 ...`. A
tool's version is declared once, where `tools/install.py` reads it (`[workspace.metadata.bin]` for a Rust tool). `test-distribution` runs
`targets_have_their_findings` and `every_registry_is_declared` as well as `target_test::`, so every test of the `distribution` target runs in exactly one group.
A scenario shows the refusal of a version or a launcher in a `run` line.

Edited in place a second time: every check a product passes is an asymptote tool, in a group, a label asymptote gives no meaning. Each tool names its `group`;
`asymptote check --groups lint` is the commit gate (`.githooks/pre-commit`) and `asymptote check --groups lint,test` is CI (a plain `asymptote check` runs every tool). The file adds
`tastemaker-validate`, `cargo-check`, `cargo-hack` and `sdk-api` (a product that names no SDK passes it) to the lint group,
`test-default`, `test-all-features` and `test-distribution` to the test group (strictly pass or fail: no lock holds a failing test), and `performance`, which reads the Bencher Metric Format
(`output = "bmf"`) and is judged against the perf records in the lock, to the perf group. `cargo-hack` is no longer a manual stage.

### Moving up from the earlier 0.5.0

- Add `group = "lint"` to each existing tool and the nine tools above, from the template.
- Pin `taplo-cli` in `[workspace.metadata.bin]` (the root's `taplo` tool runs it).

Edited in place for asymptote, which ryo and Tastemaker are gated by: a SARIF tool says so once, with
`output = "sarif"`, and a run is complete only when its output parses; an exit of 2 or more fails it. `parser` and `ok_exit_codes` are gone, and with them the
defaults keyed by tool id, so a tool that exits 1 without writing its log fails instead of resolving every record. A tool may name the files it reads beyond the product with `inputs`, so `check` reuses a run whose
inputs did not move; the template does not require it.

### Moving to it

- Replace each `parser = "sarif"` with `output = "sarif"`, and drop `ok_exit_codes`.

### Before the edit

`cargo-machete` checks the workspace's packages, not every `Cargo.toml` under the product. Given no path, cargo-machete 0.9.2 walks the directory for manifests and,
with `--with-metadata`, runs `cargo metadata` in each one it finds; a fixture that is its
own Cargo workspace (ryo's `comparisons` and `operations` spec fixtures) then gains a
`Cargo.lock`, which changes a released version's digest. cargo-machete has no exclude
option, so the run hands it each member's manifest, `.packages[].manifest_path` of
`cargo metadata --no-deps --locked` read with `jq` and passed NUL-separated, so a path
with a space or a quote stays one argument; a manifest path is walked as itself alone. A fixture
is not a package the product builds, so it is not the gate's to check. The template
contract does not compare a TOML array's elements yet, so a file that keeps
0.4.0's `run` is not a finding until it does. Everything else is 0.4.0's.

### Moving up from 0.4.0

- Replace the `cargo-machete` tool's `run` with the template's pipeline.
- Place the file with `version = "0.5.0"`.

## 0.4.0

`cargo-clippy` reports each warning as a finding asymptote ratchets, instead of failing on
the first. The run pipes
`cargo clippy --message-format=json` through `clippy-sarif` (sarif-rs, the pinned
`cargo bin clippy-sarif`) and is read with `parser = "sarif"`; `-D warnings` is gone. A
compile error, or a lint at `deny` such as clippy's `correctness`, still fails the run, which
asymptote never reads as clean. Every rustc and clippy warning the workspace lints turn on
with a source span is a result (`clippy-sarif` drops one without a span; `cargo-workspace`
0.7.0 denies `unknown_lints`, the one the workspace lints can raise), so the lock holds the
product's accepted clippy debt and the gate fails on a new warning and on a fixed one the
lock has not dropped. Everything else is 0.3.0's.

The fixture `tastemaker-without-sarif` is gone. With `cargo-clippy` also read as SARIF, the
template contract matches a `[[tools]]` key across entries, so the `parser = "sarif"` the
Tastemaker tool loses is still found on the clippy tool: a Tastemaker tool read without SARIF
is not a finding until each template entry is matched to one file entry.

### Moving up from 0.3.0

- Replace the `cargo-clippy` tool's `run` with the pipeline above and add
 `parser = "sarif"`.
- Pin `clippy-sarif` in `[workspace.metadata.bin]` (`cargo-workspace` 0.7.0).
- Baseline the findings: `asymptote ratchet up --reason "..."` in a commit of its own.
- Place the file with `version = "0.4.0"`.

## 0.3.0

Every linter a product runs is an asymptote item, the same in every product. Beside `cargo-fmt`,
`tastemaker`, `cargo-clippy` and `cargo-dylint`, the file runs two exit-status gates, each
through the version the workspace pins (`cargo-workspace` 0.6.0):

| tool | fails when |
|---|---|
| `cargo-machete` | a package declares a dependency it never names |
| `cargo-deny` | a dependency breaks the product's `deny.toml` (bans, licenses, sources) |

Each tool is required: a file without one of the six is a finding. `cargo-hack`, pinned by
`cargo-workspace` 0.6.0, is not an asymptote item: it is one of the slowest checks, so it is
the manual stage run on main every three landings.

### Moving up from 0.2.0

- Add the two `[[tools]]` entries from the template to the product's `asymptote.toml`.
- Remove what `cargo bin cargo-machete --with-metadata` reports; a dependency only a
 macro's expansion names goes in the package's `[package.metadata.cargo-machete]`
 `ignored`, with a comment saying why.
- The product's Cargo manifest follows `cargo-workspace` 0.6.0 and its `deny.toml`
 follows `deny-config`.
- Place the file with `version = "0.3.0"`.

## 0.2.0

`cargo dylint` runs the repository's one dylint library, `tools/lints`,
instead of `crates/tastemaker/lints/nominal_types` and `crates/tastemaker/lints/architecture`.
The architecture library is gone: its lints are Tastemaker rules the `tastemaker` tool
already runs. The nominal-interface lints, which need resolved types,
moved unchanged to `tools/lints`.

### Moving up from 0.1.0

- In the product's `asymptote.toml`, the `cargo-dylint` tool's `run` passes
 `"--path", "../../tools/lints"` and no other `--path`.
- In the product's `Cargo.toml`, `[[workspace.metadata.dylint.libraries]]` names
 `path = "../../tools/lints"`.
- Place the file with `version = "0.2.0"`.

## 0.1.0

The first v2 release: a product's asymptote configuration: the tools whose findings its lock ratchets.
