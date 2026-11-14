# clippy benchmark

The Rust provider against clippy 0.1.100, the `clippy-driver` of the Rust 1.100.0 toolchain that `rust-toolchain.toml` pins: rustup
installs it with the toolchain's `clippy` component, and no other install is made. The executable verified and hashed is the
toolchain's own `clippy-driver` (`rustup which clippy-driver`), never the rustup proxy of the same name on the path.

`benchmark.toml` pins the 495 lints a plain run of clippy 0.1.100 enables, `clippy::all`: its warn and deny lints. rustc's own warnings,
which the same run prints, are the compiler's and are not compared. Of the 495, 70 have a query, each in `rules/<lint>/` with its cases;
the other 425 are `deferred`, each under the limitation that blocks it. 410 of them read an expression's type or the standard item a
path names, which is semantic analysis. `fragments/lint-attributes.ryoql` holds the `allow`, `expect`, `warn`, `deny` and
`forbid` attributes every query honors, as rustc reads them: the nearest attribute naming the lint, its group or `clippy::all` decides,
and an `allow(warnings)` nearer than a `warn(warnings)` silences every warning under it.

These clippy findings have no query yet:

- a literal inside a `macro_rules!` body, which clippy reports where a call expands it;
- a string a macro other than the standard formatting macros passes to `format_args!` (`anyhow!`, `log::info!`), which
  `octal_escapes` does not read as an expression;
- an outer `#[allow]` on a `mod` item, which clippy applies to the module's own file;
- an `if` or a `let` written inside a `macro_rules!` body (`manual_take`, `toplevel_ref_arg`) and a binding a local macro's
  argument names (`disallowed_names`), which clippy reports where a call expands them;
- an expression inside the arguments of a standard macro (`println!`, `format!`, `write!`, `assert!(a == b || a < b)`, `vec!`), which
  the Rust provider reads as a token tree, not as an expression, so a lint clippy reports there has nothing to match;
- a pattern inside `matches!`, which is not read as a pattern;
- two expressions clippy holds equal that differ beyond layout, comments inside them and a number's spelling: a type suffix inside a
  larger expression (`x + 1` and `x + 1u32`), and locals bound alike under other names (`let y = x * 2; y` and `let z = x * 2; z`);
- a comparison's constant that is a named constant or a `char`, which clippy evaluates (`x < C && x > 10`, `c > 'z' && c < 'a'`);
- a doc comment inside a `macro_rules!` body, which clippy reports at the definition when a call expands it (`tabs_in_doc_comments` and
  the other doc-comment lints read no token tree);
- an attribute a `cfg_attr` expands, which `duplicated_attributes` compares only when the condition holds, and an attribute list
  nested deeper than two levels;
- an attribute macro other than `derive` and `test`, which consumes its attribute before `empty_line_after_outer_attr` and
  `empty_line_after_doc_comments` see it;
- a struct literal with a `..` spread whose assigned field is reached through `DerefMut` (`temporary_assignment`).

The reference runs, in each root, the toolchain's `cargo-clippy` beside the verified driver:

```text
cargo-clippy clippy --workspace --lib --bins --tests --examples --message-format=json --offline -- -A clippy::all
  -A clippy::pedantic -A clippy::nursery -A clippy::restriction -A clippy::cargo -W clippy::<lint>…
```

Every target but the benches: ripgrep's bench builds on nightly only (globset's `#![feature(test)]`), and clippy on the pinned
stable toolchain refuses the whole run for it.

with `CARGO` and `RUSTC` naming the toolchain's own, `CARGO_TARGET_DIR` and `CLIPPY_CONF_DIR` the command's scratch `targets/` (a
project's `clippy.toml` is never read) and one job. The four groups clippy leaves off are allowed too, so a lint a project's manifest
turns on stays off: the command line comes after the manifest's `[lints]`. A lint a crate's own `#![warn(clippy::…)]` turns on, which no
flag outranks, is outside the comparison. A case is a crate: its root is given the manifest of the edition-2024 package `case`.

Discovery is one untimed run with `--locked` in place of `--offline`, the one run that fetches. It keeps the `.rs` sources under the
root that the dependency files it wrote name (`targets/debug/build/<package>/<unit>/out/*.d`), so the files `cfg` turns off, a
directory outside the workspace and a test harness's own inputs are not requested. Its run also checks every dependency once.

Each measured run marks the requested sources modified before the timer starts, so clippy checks every workspace crate again with its
dependencies warm instead of replaying cached messages. Besides linting, the timed run compiles the workspace's build scripts and
proc-macro crates (clap's `clap_derive`), because a check of a crate needs them run: that is what a clippy user waits for too.

From `crates/ryo`:

```sh
cargo bench -p idiomatic-sh-ryo-rust-clippy-benchmark --bench clippy -- verify-reference-cases "$(rustup which clippy-driver)"
cargo bench -p idiomatic-sh-ryo-rust-clippy-benchmark --bench clippy -- verify-cases "$(rustup which clippy-driver)"
cargo bench -p idiomatic-sh-ryo-rust-clippy-benchmark --bench clippy -- audit
```

`verify-reference-cases` checks the cases against clippy alone; `verify-cases` runs both contestants over the authored rules and refuses
while any default is unaccounted. `bench-parity rust/clippy` and `bench-measure rust/clippy` clone ripgrep and clap at the revisions
`../../benchmark.toml` pins; the collector finds `clippy-driver` in the directory of the toolchain's `cargo` first.
