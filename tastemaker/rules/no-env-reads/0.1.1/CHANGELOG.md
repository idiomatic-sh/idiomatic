# no-env-reads 0.1.1

A read inside a macro's arguments is one finding, at the path, whatever the macro. The Rust
language now reads every macro call's arguments as expressions (ryo-b1gq.27.8), so a path
written there, `show!(std::env::vars_os)` or `println!("{:?}", std::env::var("X"))`, is a
`reference` as well as `mention` tokens. 0.1.0 reported both, the path and its last token,
as two findings for one read, and how many it reported depended on which macros the
language reads.

A token read (`option_env`, or `var_os`/`vars_os`, or `var`/`vars` beside `env` in a token
tree) is now dropped where it is a name of an env path read in the same file: its span lies
inside the path's span, and no `<` of the syntax tree lies between the path's start and the
token, so the token is not in the path's generic arguments. A turbofish
(`std::env::var::<&str>("X")` inside `println!`'s arguments), whose path runs past its last
name, is one finding; a separate read written inside those generic arguments (an
`option_env!` in a `macro_rules!` body in a const block) stays a finding of its own. The
match reads only the spans of `reference` and `static-import` and the syntax tree's tokens,
so it holds whether or not a build of the language also lays macro arguments out as
semantic syntax. A token the language reads as no path still has no `reference`, so its
read stays a finding: a `macro_rules!` body, an opaque built-in macro, arguments that do
not parse.

Classification: a fix that tightens. Fewer duplicate findings, and no read is lost: a token
is dropped only where its path is reported. The pager exception is unchanged.

Scenarios: `var-in-macro` expects one finding where it expected two; `vars-os-in-macro`
keeps its one finding and no longer says the language leaves the arguments unread. Added:
`turbofish-var-in-macro` (one finding where the path runs past its last name),
`vars-os-in-macro-rules-body` (the token branch still reports what no path covers),
`token-read-and-path-read-in-two-files` (a token read in one file with the span of a covered
token in another file is a finding: the match is per file) and
`token-reads-before-and-after-a-path-read` (a token read outside a path read's span, before
or after it in the same file, is a finding), `separate-read-inside-generic-arguments` and
`separate-read-inside-generic-arguments-in-macro` (a read inside another read's generic
arguments is two findings, natively and inside `println!`'s arguments) and
`read-inside-generic-arguments-of-a-read-in-macro` (a path read written inside another's
generic arguments is one finding each: whether a token is a path's name is decided per
path).

0.1.0 is removed with this version, not kept as published: no lock names it, nothing has
been released, and an unreleased version keeps no copy for compatibility (as with
idiomatic-rust 0.1.13's removal in 0.1.14).

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
