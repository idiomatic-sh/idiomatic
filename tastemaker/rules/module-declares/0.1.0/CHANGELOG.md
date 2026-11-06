# module-declares

## 0.1.0

The first release. It is the check a part's module file cannot template: a template sees only its own instance, so the module file beside a part directory cannot declare the part's optional files. A finding is a `.rs` file its module file does not declare (reported at the file), a declaration with no file (reported at the declaration), a declaration without the gating of its role, a `mod.rs`, or a directory of `.rs` files with no module file beside it (reported at the directory's first file). `lib.rs` is the module file of the files of its own directory; `schemas_test.rs` is declared by `schemas.rs` and a directory named `src` or `bin` needs no module file.

Test files: `X_test.rs` is declared either by the directory's module file (`#[cfg(test)] mod X_test;`) or by its sibling logic file (`#[cfg(test)] #[path = "X_test.rs"] mod tests;`), which generalizes the `schemas.rs` exception to the command line's infrastructure files. A bodyless `mod tests;` with that exact attribute is neither an extra declaration nor a missing one; any other `#[path]` module, a test declared twice and a test declared by neither way are findings.

Placement: a root `rules` entry of the product's tastemaker.toml (the starter writes it), not a namespace spec. A rule placed by a spec instance does not read the directories of the part instances nested in it, and a part's module file lies outside the part, so only a rule that covers the whole tree sees a module file and the files it must declare.

A namespace file holds declarations and no code: an item in `inbound.rs`, `use_cases.rs`, `outbound.rs`, `domain.rs`, `collections.rs`, `adapters.rs` or the module file of a part beside them is a finding.

a port's and a collection's contract and fake live in the part's own `fixtures/` directory, so the contract special case is gone: the only gating roles left are `test` on a `*_test` module and `test` plus the `fixtures` feature on `fixtures` and `cases`.

## Test-only syntax scope

Exact `#[cfg(test)]` is sufficient for private fixtures/cases modules, including directory modules. Cross-crate shared fixture support may continue to use the existing test plus fixtures feature cfg. Unguarded and negated-test fixture declarations remain findings; no production feature is required for test-only authority.
