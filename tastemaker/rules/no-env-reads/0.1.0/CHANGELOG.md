# no-env-reads 0.1.0

The first release. It replaces the `no_env_reads` dylint lint of the deleted
`crates/tastemaker/lints/architecture` library ( the custom lints become
Tastemaker rules defined in TOML). The devtool starter's shared rule block places it with
`include = ["**"]`: every file, adapters, the composition root, `main.rs` and tests
included.

A finding is:

- a reference or import whose written path is `std::env::{var,var_os,vars,vars_os}`,
 `env::{var,var_os,vars,vars_os}` (a read through an imported `std::env`), a glob import of
 `std::env::*`, or `option_env` (`std::`/`core::` qualified or not). A renamed import
 (`use std::env::var_os as lookup;`) is a finding at the import, where the lint reported
 each call of `lookup`;
- an `option_env` token anywhere, which also covers `option_env!` inside another macro's
 arguments;
- inside a macro's arguments, where paths are only tokens: `var_os` or `vars_os`, or `var` or
 `vars` in a token tree that also names `env`.

`env!` (a compile-time Cargo constant) and `std::env::args` are not reads.

## Differences from the lint

- The lint resolved each path; the rule matches paths as written. Not seen: a read through
 a renamed `std` (`use std as s; s::env::var`). Seen although the lint would not: a `var`
 in a macro's arguments beside an unrelated `env` token, and any item written
 `env::var` even when `env` is not `std::env`.
- The lint was `forbid`, so an `#[allow(no_env_reads)]` was itself an error (E0453). No
 attribute reaches a Tastemaker rule, so the read under such an allow is still a finding.
 The one way to silence a single finding is a Tastemaker in-source ignore with its reason.

## Moving up from the no_env_reads lint

- Take the devtool starter 0.6.0's shared rule block, or add an entry
 `{ package = "idiomatic.arch", version = "0.1.0", rule = "no-env-reads", include = ["**"] }`
 to the product's `tastemaker.toml`.
- Drop `--path../tastemaker/lints/architecture` from `cargo dylint`.

## Fixtures

Each `//~ no_env_reads` marker of the lint's UI fixtures (`no_env_reads`,
`no_env_reads_main`, `no_env_reads_exempt`) is one failing fixture: `var` (under a
module-wide `#![allow(clippy::disallowed_methods)]`), `vars-as-value`, `renamed-import`,
`vars-call`, `vars-os`, `option-env`, `option-env-in-macro`, and `allow-attempt` (the E0453
cases). `var-in-macro` and `module-import` add a read inside `println!` and a read through
`use std::env;`. `args-and-env` (`std::env::args` and `env!`) passes, and so does
`method-named-var` (methods `.var()` and `.var_os()` on a value: a method call is no written
path, so it never counts).

## Named exceptions

The rule names its deliberate exceptions in itself: `[exceptions.pager]` admits `PAGER`, clig.dev's variable for paging
output, which a command line's `output` module reads to page a long answer on a terminal. A read whose variable is a string literal an exception admits, written
at the call (`std::env::var_os("PAGER")`, `env::var("PAGER")`), is not a finding: the
query joins the literal through the call's argument list to the called path and drops that
path's read. A name passed through a binding is not seen, so its read stays a finding
(fixture `pager-by-a-binding`); `pager` passes.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.

The reads inside a macro's tokens come from the relation `mention`, the identifiers written in a
token tree, instead of the syntax tree's tokens: the language says which identifiers a macro
mentions, and the rule no longer walks tokens itself.
