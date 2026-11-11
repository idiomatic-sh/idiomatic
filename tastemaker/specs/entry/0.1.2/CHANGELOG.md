# entry 0.1.2

Places `no-runtime-fixtures@0.2.0` where 0.1.1 named its earlier version, so a product that places it locks one version of the rule: two versions of one rule share its id, and the earlier entry ran the later query. Nothing else changes: an entry's schemas are its one `schemas.rs`, and scenario `schemas-split-by-kind` holds a `schemas/` directory of per-kind files closed. 0.1.1 is removed with this version, not kept as published: no lock names it, nothing has been released.

Edited in place: places `unit-cases@0.2.0`, which reads a namespace `*_test.rs` with the tables of its groups.

Edited in place: the `unit-cases` entry reads `*_test/**/*.rs` beside its own files, the flat groups and the group directories of a namespace test, which the rule reads only when its entry includes them.

# entry

## 0.1.0

The first version, split from the retired `part` spec by kind. It lists its own files with their counts, shares the templates and rules the other kinds use, and is placed by the crate kind that holds it, so a product's tastemaker.toml names only the crate and its crate kind.

- **No `types.rs`**: the entry's arguments are in `entry.rs`, the shape of what it is given in `schemas.rs`, and the values passed and returned are the core's types; the `<Resource><Part>Parts` its `compose` takes is declared in `composition.rs`. A `types.rs` in the part is a finding of the closed placement (`entry-with-types`). The template and the four `types.rs` rules are no longer placed; the shared `composition-parts` template is deleted, and the `types` template writes no `Parts`.

**.** New spec rule `entry-names`: an entry function takes its request and returns `Result<<Verb>Report, <Verb>Error>`; the template writes the signature from the verb directory, which pins the stems. Fixtures: `answer-not-a-report`, `error-not-an-error`, `no-request`, `no-answer`.

**.** The entry parts sit at `src/resources/<resource>/<entry>`.

- The entry's request is `<Verb>Request`, its answer `<Verb>Report` and its error `<Verb>Error`; `entry-names` requires the `Request` suffix.

- `connections.rs` is present exactly when the verb opens a client (the `opens-client` condition, answered by `uses-connections` from the composition); both directions are checked.

- The `entry` template calls `compose` with the connections only when the verb has a `connections.rs`.

**.** The entry places `types.rs` and `errors.rs` (`count = { min = 0 }`) for the types and the error the SDK owns for it: its request, its report and its refusal. The shared `types` and `errors` templates take `kind = "entry"`.

**`entry-names` allows no request and a unit report.** An entry takes no parameter when the verb has no input (`pub fn list() -> Result<ListReport, ListError>`) and may return `Result<(), <Verb>Error>` when no caller reads an answer; a parameter it does take must still be a `*Request`. Fixtures `no-request` (passes) and `unit-report`.

- **`test-runners-match-rows`** is placed beside `test-shape`: a test file has a `run(CASES, Word::<word>, subject)` runner exactly for each case word its `CASES` table has rows of. The test templates (`unit-test`, `item-test`, `schemas-test`, `composition-test`, `command-test`, `entry-test`, `use-case-test`, `adapter-test`) no longer render all four runners: an empty table has no runner. The fixtures `*-without-rejects-runner` became `*-with-rejects-row-without-runner`.

## 0.1.1

- **Fixtures directory**: a new optional template `entry-fixture` places `fixtures/{*fixture}`, one test-data file per thing it fixes, beside `fixtures.rs`; the long `fixtures.rs` of an entry splits into it. The template asks nothing of the file but its doc comment.
