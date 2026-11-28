# Brief: map what exists onto the agreed Ryo tree (read-only)

BEAD: ryo-fmx3.1.13.23.3
WORKTREE: none. This is a READ-ONLY job. You write exactly one file (named in your prompt) and nothing else.

## Why

The owner and the main session have agreed a new crate layout for Ryo. Before anything moves, the owner wants to see
what exists today and decide what maps to where. Your table is what he will decide from. Propose; do not decide.
Accuracy matters more than coverage: a wrong row is worse than a row that says "not checked".

## How to read (hard rules)

- Repo: `/workspaces/idiomatic-public-benchmark`. Never check it out, never edit it, never `cd` into it.
- Read only through git at a fixed revision:
  - `git -C /workspaces/idiomatic-public-benchmark ls-tree -r --name-only REV -- PATH`
  - `git -C /workspaces/idiomatic-public-benchmark show "REV:PATH"` (in zsh always quote as `"${REV}:path"`)
  - `git -C /workspaces/idiomatic-public-benchmark grep -n PATTERN REV -- PATH`
- Revisions: the trunk `T=2a207464c` (build 139). The designed crates before they were deleted: `O=f0219b3b3` (build 133).
- Never run Cargo (a shell guard blocks it). Never `rm`. No `sleep`. Do not commit or push anything.
- Trunk line counts per file are already in `/workspaces/ryo-poc-evidence/next/build/ports/stage2/mapping/lines-per-file.txt`
  (`path:lines`). "Lines" in your table means Rust lines WITHOUT tests (`_test.rs`, `_test/`, `tests/`, `cases`, fixtures).

## The agreed tree, and the single thing each crate owns

```
crates/ryo/crates
├── domain
│   ├── relations    the catalog of tables (which tables exist, their columns and kinds)
│   ├── operators    the operators: each one's name, arguments and meaning (its kernel; its own use of regex or glob)
│   ├── ryoql        query text -> checked query (syntax, parsing, typed checks, the kind check)
│   ├── plan         checked query -> plan (the lowering, and the plan's data type)
│   └── execute      run a plan on one file's facts, and once on the project -> rows (includes the screening that
│                    skips rules that cannot match a file, and the cross-file project phase)
├── outbound
│   ├── language     the contract a language implements
│   ├── store        the tables during a run (where working tables live; layout hidden behind its contract)
│   ├── cache        what is kept between runs (key -> kept answer)
│   ├── corpus       the set of files a request covers (walking or listing, ignore rules, path globs, reading bytes)
│   └── scheduler    running jobs on workers
├── languages        python, typescript, rust, go, html, css, markdown, json, toml (each: one language's facts)
├── inbound
│   ├── lint         requests in, reports out (owns the request, rule, report and finding types; `query run` goes through it)
│   ├── validate     is this query valid, or why not
│   └── describe     list and describe operators, relations, the query language
└── edges
    ├── sdk          thin: `Linter::new(languages, pool, cache)`, `Linter::lint(requests) -> reports`, and the calls above
    └── (cli)        the command line: each command returns what an inbound crate gives
tools/ (top of the repo, not Ryo): benchmark, benchmark_contracts, profile_tools
```

Rules the owner set:
- Every crate owns ONE thing behind one contract. If a module needs "and" to describe it, split the row.
- "We do not colocate by mechanism": two things using the same library or technique (globs, caches, value types) is not
  a reason to share a home. Place each piece with the concept it serves.
- No contract may name rows versus columns (the layout belongs to `store`, injected).
- Import directions: domain -> domain only; outbound and languages -> domain; inbound -> domain and outbound, never
  another inbound crate; edges -> inbound.
- Contracts are not dead code: never recommend deleting a trait or a crate boundary because nothing calls it. Report it.

## What to write

One markdown file. Sections, in this order:

1. **Table**, one row per module directory. Split a row further (by file, or by line range and symbol group) when a
   file is over 300 lines or a directory mixes concerns. Columns:
   `path | lines | what it does | destination | confidence | evidence`
   - `what it does`: one plain sentence FROM READING THE CODE, not from the name.
   - `destination`: one crate of the tree (for example `domain/operators`), or `tools/...`, or `NO HOME`, or `SPLIT`
     (then give the sub-rows).
   - `confidence`: high, medium or low.
   - `evidence`: a `path:line` or a symbol that shows it.
2. **Totals per destination** (lines).
3. **NO HOME**: everything that fits no crate of the tree, or fits two, each with the reason. Do not force a fit.
4. **No caller found**: items nothing calls, with how you checked. Report only; no recommendation.
5. **Contracts**: every `pub trait` in scope: name, `path:line`, number of methods, who implements it, who calls it,
   and whether its signatures name a row-shaped or column-shaped type.
6. **Import-direction conflicts**: places where the proposed destination would import against the directions above
   (for example a domain piece that calls a store type directly).

Write "not checked" wherever you did not check. Plain words, short sentences.

## Your final message

At most 12 lines: lines per destination, the number of NO HOME rows, and the three findings the owner most needs to see.
A context stop is not a result: if you are running out of room, finish the table for what you read and say exactly
which paths are not covered.
