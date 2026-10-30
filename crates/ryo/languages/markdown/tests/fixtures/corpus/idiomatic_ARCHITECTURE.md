# Repository architecture

Tastemaker is the definition of this repository's architecture. The structure, the files each
directory holds and the checks they must pass are the Tastemaker specs, templates and rules of the
package `tastemaker/` (`idiomatic.arch`) and of each product's own `tastemaker/`, placed by each
`tastemaker.toml`. Tastemaker's own design is in `crates/tastemaker/docs/README.md`.

This file keeps only what those cannot express: the principles behind the specs, and the
index of the products' design documents.

## A. Principles

### A.1 Levels

The levels (repository, workspace, crate, domain), the product layout and what each level is
built from are the specs of `tastemaker/`: `idiomatic-project`, `crate`, `nu-hex`, `cli`, `sdk`.

### A.2 A domain is a hexagon

Every domain is a hex, wired only at entry points (a command, an SDK entry): the `nu-hex`,
`use-case`, `port` and `adapter` specs, with their rules.

### A.3 Public interfaces

A consumer of a product names its SDK alone (the `sdk` spec). Named, not restated: the rules
no-reexports, no-public-options, no-domain-aliases, private-checked-fields and
no-json-in-process; rustc's `private_interfaces` and `unnameable_types`; and the
nominal-interface lint in `tools/lints` for interface types.

### A.4 Testing and evidence

Test files are placed by the `crate`, `use-case`, `cli`, `sdk` and `language` specs. A missing
tool, a zero-test target or an unexecuted case is never success.

## B. Products

| product | design |
|---|---|
| Ryo | `crates/ryo/docs/README.md`, with the language contract (`crates/ryo/docs/language-contract.md`); its specs are `crates/ryo/tastemaker/` (`idiomatic.ryo`) |
| Tastemaker | `crates/tastemaker/docs/README.md` |
| Asymptote | `crates/asymptote/docs/README.md` |
