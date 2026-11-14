# clippy benchmark

The Rust provider against clippy 0.1.100, the `clippy-driver` of the Rust 1.100.0 toolchain that `rust-toolchain.toml` pins: rustup
installs it with the toolchain's `clippy` component, and no other install is made. The executable verified and hashed is the
toolchain's own `clippy-driver` (`rustup which clippy-driver`), never the rustup proxy of the same name on the path.

`benchmark.toml` pins the 495 lints a plain run of clippy 0.1.100 enables, `clippy::all`: its warn and deny lints. rustc's own warnings,
which the same run prints, are the compiler's and are not compared. Of the 495, 14 have a query, each in `rules/<lint>/` with its cases;
the other 481 are `deferred`, each under the limitation that blocks it. 407 of them read an expression's type or the standard item a
path names, which is semantic analysis; `unauthored` holds the lints this round's batches author from the relations the Rust provider
reads. `fragments/lint-attributes.ryoql` holds the `allow`, `expect`, `warn`, `deny` and
`forbid` attributes every query honors, as rustc reads them: the nearest attribute naming the lint, its group or `clippy::all` decides,
and an `allow(warnings)` nearer than a `warn(warnings)` silences every warning under it.

These clippy findings have no query yet:

- a literal inside a `macro_rules!` body, which clippy reports where a call expands it;
- a string a macro other than the standard formatting macros passes to `format_args!` (`anyhow!`, `log::info!`), which
  `octal_escapes` does not read as an expression;
- an outer `#[allow]` on a `mod` item, which clippy applies to the module's own file;
- two expressions compared inside a macro's arguments (`assert!(a == b || a < b)`): `expression_shape` holds the file's own tree, not
  the arguments read again from a call;
- a pattern inside `matches!`, which is not read as a pattern;
- two expressions clippy holds equal that differ beyond layout, comments inside them and a number's spelling: a type suffix inside a
  larger expression (`x + 1` and `x + 1u32`), and locals bound alike under other names (`let y = x * 2; y` and `let z = x * 2; z`);
- a comparison's constant that is a named constant or a `char`, which clippy evaluates (`x < C && x > 10`, `c > 'z' && c < 'a'`).

The reference runs, in each root, the toolchain's `cargo-clippy` beside the verified driver:

```text
cargo-clippy clippy --workspace --all-targets --message-format=json --offline -- -A clippy::all -A clippy::pedantic
  -A clippy::nursery -A clippy::restriction -A clippy::cargo -W clippy::<lint>…
```

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
