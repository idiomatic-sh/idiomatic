# no-ignored-tests 0.1.0

The first release. It replaces the `no_ignored_tests` dylint lint
 of the deleted `crates/tastemaker/lints/architecture` library (the custom lints become Tastemaker rules defined in TOML), with the same reach:
every `#[ignore]`, with a reason or without, and one a `#[cfg_attr]` applies, on any item.
The `devtool` starter 0.6.0's shared rule block places it with `include = ["**"]`.

## Moving up from the no_ignored_tests lint

- Add `{ package = "idiomatic.arch", version = "0.1.0", rule = "no-ignored-tests", include = ["**"] }`
 to the product's root `tastemaker.toml` (or take the `devtool` starter 0.6.0's block).
- Drop `--path../tastemaker/lints/architecture` from `cargo dylint`.

## Fixtures

Each UI case of the lint is one fixture: `ignored`, `ignored-with-a-reason` and
`ignored-when-tested` fail; `allow-attempt` ports the lint's E0453 cases, an
`#[allow(no_ignored_tests)]` over an ignored test, which still fails because no attribute
reaches a Tastemaker rule. `runs` passes: a running test, a function named `ignore`, and
`ignore` inside a doc comment's code fence.

The rule reads syntax, not the compiler's attributes: an `#[ignore]` a procedural macro
generates is not seen, and one in code a `#[cfg]` configures out is (the lint saw the
reverse). A `macro_rules!` body is read as its tokens: `#[ignore` there is reported, at the
`ignore` (fixture `ignored-in-a-macro-body`), since every test the macro writes is ignored.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.
