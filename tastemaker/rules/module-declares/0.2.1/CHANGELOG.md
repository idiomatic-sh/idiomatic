# module-declares 0.2.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. A declaration is in a module file when its file is among the module paths, grouped by path, and an item is in a namespace file when its file is among the namespace paths, grouped by path (one row per file each). A test file's name is captured from its own path. A missing, ambiguous or undeclared file takes its span from its source file's root node, of which a file has one: those three joins are written `all-pairs join`, every pair being the answer. The findings and the scenarios are as in 0.2.0. It needs a Tastemaker whose RyoQL reads `all-pairs join`.

# module-declares 0.2.0

The description, `why`, two problem texts and the help name one module-file form, `X.rs` beside the directory `X/` (owner, 2026-11-11; clippy `mod_module_files`, denied by cargo-workspace 0.13.0, refuses `X/mod.rs`). 0.1.0 told the author both forms were supported. What the query reads and reports is as in 0.1.0. 0.1.0 stays as published: tastemaker and asymptote lock it.

Edited in place: the `why` no longer speaks of "either supported module file", and the passing scenarios name module files `X.rs` (scenario `mod-rs`, which showed a `mod.rs` passing, is removed); the scenarios that report a directory module or two forms keep their `mod.rs` inputs, as the query still reads that form.
