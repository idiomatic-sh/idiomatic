# cargo-config

## 0.3.0

Edited in place: no `[env]`. `JEMALLOC_SYS_WITH_MALLOC_CONF` was the profiling configuration of the `heap-diagnostic` build, which is gone. Every command line now links jemalloc (`cli` 0.12.0's `cli-main`), so the variable was compiled into each shipped binary and jemalloc printed `Invalid conf pair` for every key on each start. Remove the `[env]` table.

The `coverage` alias is `["llvm-cov", "--all-targets"]`: coverage is a report with no threshold, so
`--fail-under-file-lines 99` is gone. The `coverage` tool of `asymptote-config` 0.6.0 runs it in every product's test group.

### Moving up from 0.2.0

In `[alias]`, drop `"--fail-under-file-lines", "99"` from `coverage`.

## 0.2.0

Edited in place: no alias goes through `cargo bin` (cargo-run-bin is gone). `tools/install.py` installs each tool at the version the
workspace declares in `[workspace.metadata.bin]`, so `cargo deny`, `cargo hack` and `cargo nextest` are the subcommands themselves, and `mutate` is
`["mutants", "--test-workspace=true", "--annotations=none"]`. Remove the `deny`, `hack` and `nextest` aliases and change `mutate` to match.

`[env]` sets `JEMALLOC_SYS_WITH_MALLOC_CONF`, the profiling configuration tikv-jemalloc-sys compiles into a `heap-diagnostic` build of a command line. Only a build that links jemalloc reads it.

`cargo mutate` runs the `cargo-mutants` each workspace pins (`cargo-workspace` 0.6.0)
through cargo-run-bin, like `cargo deny`, `cargo hack` and `cargo nextest`: `mutate` is
`["bin", "cargo-mutants",...]` in place of the `cargo-mutants` on `PATH`.
There is no `mutants` alias: an alias that shadows an installed subcommand is a Cargo
warning that is becoming an error (rust-lang/cargo#10049).

`[profile.dev]` sets `incremental = false`: a build that starts from an
empty target directory is faster and half the size without incremental state.

Moving up from 0.1.0: in `[alias]`, replace `mutate`'s `"mutants"` with
`"bin", "cargo-mutants"`.

## 0.1.0

The first v2 release: the repository's `.cargo/config.toml`. It links with mold on Linux, keeps line tables instead of full debug info, drops dependency debug info, statically links the C runtime on Windows MSVC, and aliases `cargo deny`, `cargo hack` and `cargo nextest` to cargo-run-bin (`cargo bin`), which runs the version each workspace pins in its `[workspace.metadata.bin]` (the `cargo-workspace` template).
