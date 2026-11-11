# command

## 0.2.1

Places `no-runtime-fixtures@0.2.0`, where 0.2.0 named its 0.1.0: a product that places this spec beside the specs that moved to that version locks one version of the rule. What each entry reads is as in 0.2.0; `no-runtime-fixtures@0.2.0` also reads an item gated `cfg(any(test, feature = "fixtures"))` as test-only. 0.2.0 stays as published: the v1 products lock it.

Edited in place: places `unit-cases@0.2.0`, which reads a namespace `*_test.rs` with the tables of its groups.

Edited in place: the `unit-cases` entry reads `*_test/**/*.rs` beside its own files, the flat groups and the group directories of a namespace test, which the rule reads only when its entry includes them.

## 0.2.0

Edited in place: `command-refuses` owes no row for a command whose arguments struct has no field (`pub struct DownArguments;`): it reads no input, so it has nothing to refuse.

Edited in place: the edge is the `cli` spec's `edge` template, `src/edge.rs`, so a command's `imports` allow `crate::edge` in place of `crate::Edge`, and the `command` template's `run` takes it, `run(arguments, edge: &crate::edge::Edge) -> Outcome`, printing in `edge.format`: the format is the command line's, read once by the command tree, and the arguments hold the command's own flags only.

Edited in place before that: a command's `imports` allow `crate::Edge`, which the crate root declares, in place of `crate::resources::Edge`: the command tree's modules and the commands they reach no longer name each other (`module-cycles`).

Places the `composition` template 0.2.0, whose `compose` is `pub(crate)` (rustc's `unreachable_pub`); nothing else changes. The scenarios' compositions follow. A product moves by placing `cli` 0.12.0.

## 0.1.0

The first version, split from the retired `part` spec by kind. It lists its own files with their counts, shares the templates and rules the other kinds use, and is placed by the crate kind that holds it, so a product's tastemaker.toml names only the crate and its crate kind.

- **No `types.rs`**: the command's arguments are in `command.rs`, the shape of what it is given in `schemas.rs`, and the values passed and returned are the core's types; the `<Resource><Part>Parts` its `compose` takes is declared in `composition.rs`. A `types.rs` in the part is a finding of the closed placement (`command-with-types`). The template and the four `types.rs` rules are no longer placed; the shared `composition-parts` template is deleted, and the `types` template writes no `Parts`.

- `connections.rs` is present exactly when the verb opens a client (the `opens-client` condition, answered by `uses-connections` from the composition); both directions are checked.

- The `command` template calls `compose` with the connections only when the verb has a `connections.rs`.

- **`test-runners-match-rows`** is placed beside `test-shape`: a test file has a `run(CASES, Word::<word>, subject)` runner exactly for each case word its `CASES` table has rows of. The test templates (`unit-test`, `item-test`, `schemas-test`, `composition-test`, `command-test`, `entry-test`, `use-case-test`, `adapter-test`) no longer render all four runners: an empty table has no runner. The fixtures `*-without-rejects-runner` became `*-with-rejects-row-without-runner`.

- **`command` imports the edge**: a command's `imports` allow `crate::resources::Edge`, the one type of the command tree's root every command takes; any other item of `crate::resources` is still refused.
