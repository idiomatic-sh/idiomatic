# file-length

## 0.3.0

The use-case landing no longer sends shared orchestration to "steps": composition is not DRY, so a use case keeps its
orchestration and two use cases that compose the same parts repeat their wiring.

An oversized file is a signal to map its concepts, not a default destination. The adapter landing no longer splits by port method: it says to map
the adapter's concepts first, then file each by its kind (its own `types.rs`, its `dtos.rs`,
its `collections/`, the hex's `domain/<concept>/`, a hex of its own). `method.rs` and
`method_test.rs` leave the procedure landing, which now names `use_case.rs` beside
`command.rs`; an adapter's `types.rs` takes the adapter landing, and a concept's
`domain/<concept>/domain.rs` a landing of its own. Fixture `every-kind` carries an
adapter's `types.rs` and a concept's `domain.rs` in place of a split method.

The count is the whole file, for every kind of file, and no language details leak
into the Tastemaker assets. The rule reads only `filesystem.line-count`, the engine's own
line count of every file, and no language's relations.

- Every line counts: blank lines, comment lines, lines inside string literals and block
 comments, and a last line with no line break. 0.2.0 counted only a Rust file's
 non-blank lines outside literals and block comments.
- Every file the entry includes is counted, whatever its kind: `Cargo.toml`, Markdown,
 TOML, TypeScript and Python files are warned as Rust files are. A kind with no landing
 pattern is told so, as before.
- The landing patterns and the message's shape are unchanged; the message says `lines`
 rather than `non-blank lines`.
- A finding says how to act on it. The message states only the file and its count; the
 help, printed with every finding, says the limit is a target, names the pattern this
 kind of file splits along, and gives the exact top-of-file ignore in the file's own
 comment syntax (`//` or `#`), whose reason says why the file is long and why splitting
 would not help. A kind with no line comment (Markdown, Gherkin, JSON) is told it cannot
 hold an ignore. The query adds an `ignore` column beside `landing`.
- The landings name no design ids, and the test files' landing changed: tests in
 `tests/bdd.rs` and `tests/end_to_end.rs` move next to the logic they test, in the crate
 that owns it, a CLI command's tests into that command's `command_test.rs`. A file kind
 with no named split is told to examine it for one, not to leave it whole.
- Fixtures: `ignored-at-top` keeps a long TOML file and a long Rust file whole with their
 top-of-file ignores; `markdown-holds-no-ignore` shows a Markdown file cannot hold one.
- Fixtures: `at-limit` holds exactly 400 lines, blank lines and comments among them;
 `over-limit` adds one with no line break; the new `blank-lines` is a Markdown file of
 one line and three blank ones; `every-kind` gains `README.md`, and its `Cargo.toml` is
 now counted.

### Moving up from 0.2.0

Change the entry's `version` to `"0.3.0"`; the params are unchanged. The same
`max-lines` now reports more files: every file of any kind the entry includes, counted
with its blank lines. Leave out files that are generated rather than written, such as
lockfiles and recordings, with the entry's `exclude`, and keep a legitimately long file
whole with the top-of-file ignore its finding prints, and a reason.

## 0.2.0

The help names a landing pattern for five more kinds of
file.

- `fixtures.rs` splits by the interface method each fixture feeds into
 `fixtures/<method>/fixture.rs`.
- `tests/bdd.rs` and `tests/end_to_end.rs` split by feature or workflow into
 `tests/<feature>/`, with the same files in each.
- `lib.rs`, `main.rs` and `composition.rs` are never split: they only declare and wire, so
 a long one holds logic that belongs in a hex.
- A macro's `src/macros/<macro>/{parse,check,expand}.rs` and their tests split by construct
 into `<stage>/<construct>/`, such as `expand/field/`.
- `benches/performance.rs` splits by workload into `benches/performance/<workload>/bench.rs`.

The kinds' globs are disjoint, so a path takes at most one landing. The `every-kind`
fixture gains a file of each new kind and `src/store.rs`, which still has no pattern.

### Moving up from 0.1.0

Change the entry's `version` to `"0.2.0"`; the params are unchanged.

## 0.1.0

The first release: a Rust file with more than `max-lines`
non-blank lines is reported at level `warn`, which never fails the check, with the pattern
its split lands into.

- A line is counted when a line break outside a string literal or block comment ends it:
 blank lines, including those before the first token, are not counted; `//` and `///`
 comment lines are. Ryo's facts carry byte spans and no line numbers or token adjacency,
 so a comment-only line cannot be told from a code line in the query.
- `max-lines` is an integer param with no default (v2 params have none); the shared rule
 block sets it to 400.
- The help names the landing pattern by file kind: `adapter.rs`, `adapter_test.rs` and
 `dtos.rs` split by method into `methods/<method>/` (H1); `method.rs`, `command.rs` and
 their tests move domain logic into the hex's types or a new hex; `types.rs`,
 `interface.rs`, `errors.rs` and their tests mean the domain is split into more hexes.
 Every other file is told that no landing pattern is defined for it yet.
