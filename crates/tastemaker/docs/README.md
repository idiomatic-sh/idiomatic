# Tastemaker

Tastemaker's design: what it is for, how it works with ryo, how definitions are stored and checked, and its command line.

## 0. Goals

One row for each goal, and the measure that says the goal is met.

| # | goal | met when |
| --- | --- | --- |
| 1 | Fewer files | A rule, a spec or a template is 1 file, its scenarios in it |
| 2 | As fast as ryo | On one tree, ryo's `lint` of the same rules beside Tastemaker's `check` and `validate`, each its own measure of the perf group (`cost/cold-check`, `cost/ryo-cold-lint`, `cost/warm-check`, `cost/ryo-warm-lint`, `cost/validate-unchanged`, CPU ms, the median of its rounds), judged by Asymptote against the lock; no test asserts a time. Tastemaker's own preparation (read definitions, resolve selectors and variables, place, build the request) reported on its own by `--timings` |
| 3 | A bounded store | One store at `<project root>/.tastemaker_cache/`, the latest run only, bounded by construction (section 3.3) |
| 4 | Two ryo calls, and the catalog for `explain` | The `library-surface` entry of `tastemaker.toml` lists `ryo::query::validate` and `ryo::linter::lint`, the catalog calls `ryo::relation::{list, describe}`, `ryo::operator::{list, describe}` and `ryo::query::describe`, and the types they take and give; any other `ryo` path is a finding (met: as built) |
| 5 | Typed rows, not JSON | `PlacedFile` rows cross the SDK contract as a host relation a rule can read (`tastemaker.placed`); no JSON in process |
| 6 | Scenarios next to the asset | `[[scenarios]]` in the asset's own file, a Gherkin Scenario each; no `fixtures/` and no `behavior.feature` |
| 7 | Nested specs, by selectors | A selector reaches a part at any depth, `{ part = "use-case", recursive = true }`, and the spec rules over nested instances fire |
| 8 | Say a value once | `variables = {...}` and `{ variable = "x" }` replace every repeated list |
| 9 | A smaller command line | 9 flat verbs, each `tm <verb>` |
| 10 | One rule set across the next products | Both place the same specs; no per-product copy of a rule |
| 11 | Validation by digest | `check` first proves every definition as `validate` does, a lookup for each whose proof is current, and a definition whose scenarios fail stops it with an error (exit 2); both re-check only the definitions whose digest changed |
| 12 | SARIF drop-in compatibility | On the same tree, an asymptote lock built from v1's SARIF reads Tastemaker's SARIF with zero differences |

A registry is not part of this design. Package resolution stays behind one port, so a registry can be added.

## 1. What Tastemaker is

|  |  |
| --- | --- |
| What | It reads `tastemaker.toml` and the definitions, places files, and asks ryo for the findings. It owns the preparation and none of the analysis |
| Boundary | `tastemaker.toml` and the definitions are Tastemaker's to organise and send through the ryo SDK; that preparation is Tastemaker's own work |
| Reads no source file | Comments, imports and declarations come back as rows from the one call |
| Ryo imports | Two calls (section 2), widened to the five catalog calls `explain` and `list` read |
| Rule set | Same specs and rules as ryo: one consistent rule set for the next products. The first Tastemaker, ryo and asymptote are out of scope |
| Ships with ryo | One Cargo workspace; Tastemaker depends on the ryo SDK by path dependency; each language is a Cargo feature of the CLI, linked only at the edge |
| Not carried over | The commands section 4 drops; v1's 17 GB analysis store |

## 2. Part 1: driving off the ryo SDK

| decision | design |
| --- | --- |
| Two calls | `ryo::query::validate(&languages, ValidateRequest { query, hosted }) -> ValidateReport` and `ryo::linter::lint_with(&linter, LintRequest { hosted, … }) -> LintReport`, over one `Linter::new(languages, pool, memory, cache)` built once at the edge, as built |
| Two lint calls per check | `workspace check` builds one `LintRequest` for every rule it placed and calls `lint_with` once over an engine that keeps its work in the store; when the plan holds counts or pairs, a second call judges them over the rows they host, on an engine that keeps nothing, so it never rewrites what the first kept; no `recognize` loop, no per-rule calls |
| Request carries | `LintRequest { hosted, … }`: `hosted` declares host relations and their rows (the placed rows are one), and the files a call reads are given as text or listed; the worker pool, the memory budget and the cache belong to the engine and are set once when it is built, never per request; a scenario's files are given to the call as text, written to no directory; no `ryo_core` type in any signature. Built in ryo: `Cache`, `Rule`, `LintReport`, `hosted`, the given sources. Step 0's real gaps (the rules agent is building them): `line` on `Finding`, and rule queries in ryo's vocabulary directly (`syntax-token` and `path` are v1's names; ryo has `syntax-tree` and `file`).  There is no translation layer: the rules are rewritten, nothing translates them |
| Store location is the host's | The host passes the location; ryo's own CLI passes its own; composed at the edge, no environment variable. Tastemaker passes `Cache::At(<project root>/.tastemaker_cache/ryo)`, inside its own store (3.3) |
| Placed rows are typed | `tastemaker.placed` is a hosted relation with the columns `path`, `entry`, `package`, `template`, `version`, `spec`: one row for each (path, entry). It crosses as typed rows in `LintRequest.hosted`; not JSON |
| Enforced | Tastemaker's `imports` allow-list admits only `ryo::query::validate`, `ryo::linter::lint` and their request and report types, and at the edge `ryo::languages::{link, Language}`; languages are linked only at the CLI edge |
| Spans cross as text | The SDK answers every finding column as text, so a span (`start..end`) reaches Tastemaker as the text the engine wrote. Tastemaker reads a column back as a span only when its text is exactly `start..end` as the engine writes it (`1..2` is a span, `001..002` stays text); a rule that selects a string that is written exactly like a span has it read as one. This is a known limit of text columns, kept until the SDK carries a typed cell |
| Rule queries | Tastemaker resolves every selector at placement into the paths it hands to ryo; RyoQL is unchanged. It validates every rule query with `ryo::query::validate` (section 3.3) |

Part 1 ends where the SDK ends: `lint` returns findings (`Finding { rule, path, key }`, and `line` once step 0 adds it), `unread` files and `unknowns`
(`#unknown`, a rule that could not decide). Tastemaker renders them; it re-derives nothing.

## 3. Part 2: the definitions

### 3.1 `tastemaker.toml`

The shape of `tastemaker.toml` (`workspace`, entries `rules`/`specs`/`templates`, `closed`, `conditions`).
Additions are the keys of 3.5 to 3.8.

| decision | design |
| --- | --- |
| One identity per entry | An entry names its definition in one string: `spec = "idiomatic.arch/crate"` in a `tastemaker.toml`, `rule = "idiomatic.arch/file-length@0.3.0"` in a spec, and a bare `name` for a definition the spec owns |
| Versions in the lock | `tastemaker.lock` pins the version of each definition a `tastemaker.toml` names; `tm update` moves it, and the `tastemaker.toml` is never rewritten |
| One path base | Every path is relative to the directory of the file that holds it, and a workspace is that whole directory: `workspace.include` is gone. A directory below it holding its own root `tastemaker.toml` is a workspace of its own |
| Most specific placement | A file several template entries match is placed by the one whose pattern writes out the most segments, so no capture excludes what another entry places |
| Derived count | A literal path places one file or directory, a glob or capture at least one; `count` is written only where it differs, and writing what the path says is refused |
| Kebab-case keys | Every key, param, variable and condition name is kebab-case; a template's content reads a param `item-trait` as `item_trait` |
| Derived params | A param `from = "workspace-name"` takes the workspace's name, as `from = "directory-name"` takes the instance directory's |

| decision | design |
| --- | --- |
| Rust policy rules in a spec | `idiomatic-rust`: a rules-only open spec in `idiomatic.arch`; each product places it once, `{ spec = "idiomatic.arch/idiomatic-rust", include = ["."] }` |
| Repo-wide rules | `idiomatic-project`, placed by the product |
| Root does not know Rust | The root `tastemaker.toml` places nothing of Rust |
| New findings are the point | New rules land with their wave of findings fixed |
| Rule not expressible | A rule that cannot be a query is a ryo feature request |
| The files a workspace sees | Only the repository's own ignore files apply (`.gitignore`, `.ignore`, `.git/info/exclude`); a user's global git excludes file is never read, by Tastemaker's walk or ryo's, so a check gives the same result on every machine. A file only a personal list hides is hidden by adding it to the repository's `.gitignore` |

### 3.2 Validation by digest (one `check`, which proves first)

| decision | design |
| --- | --- |
| Only changed versions re-validated | `validate` proves the version in use of each definition of a package: its latest, and any version the workspace's lock pins. A proof is a row of the `definitions` table: the version and its digest, of the files of its directory and of every version it uses (its `definition_uses` rows), with the size and latest modification time of those files. A row is kept only for a version whose scenarios all held. While the size and time have not moved no file is read again; `check` first proves every version in use as `validate` does, a lookup for each whose row is current, and a version whose scenarios fail stops the check with an error (exit 2) |
| Compare hashes | A hit is a lookup; an edit re-proves only the versions whose digests moved |
| A changed file is a changed key | No early cutoff by parsed form |
| Frozen package working version | Every unreleased version is proved |
| Scenarios through the same call | Scenarios (3.4) run through the same `check`: `validate` writes each scenario's files to a project of its own in memory, places the asset there and checks it like any project; ryo's lint call reads the same files as text handed to it (`Corpus::Given`), so no scenario directory is written to a disk |

### 3.3 One store

| decision | design |
| --- | --- |
| One store, at the project root | One store, `<project root>/.tastemaker_cache/`, gitignored: the project root is the directory of the root `tastemaker.toml`. ryo's cache lives inside it, at `.tastemaker_cache/ryo` (the host passes `Cache::At` that directory). Each worktree has its own root, so each has its own store and no two worktrees share or overwrite one |
| Flat tables | The store is two files, one a port: `.tastemaker_cache/validate.store` for `validate` and `.tastemaker_cache/check.store` for `check`. The edge hands both store adapters one value, the store's directory, the build that runs and whether the run writes; each adapter reads its file whole when it is made. Each file holds the build that wrote it and flat, compact tables, a row a thing, every text kept once in `strings` and named by number: `definitions` and `definition_uses` for `validate`; `files` (each file the answer rests on, its digest, size and modification time), `findings` and `rules` (each rule and check that ran) for `check`. A run that succeeds writes its file whole and renames it into place; a run that fails, or runs under `-n/--dry-run`, writes nothing. A check whose files all have the size, time or digest kept answers the kept findings without placing or linting |
| Bounded | Latest run wins: each table is replaced whole. The build that wrote the store is kept once for it, in `build`; a run of another build discards every table and proves and checks again once. Evidence: 17 of the 18 GB is v1 ryo's analysis store, never evicted |
| One build per store | Two builds alternating in one tree each discard the other's tables; same-build runs write the same rows |
| Worktrees | Settled by the location: the store is under each worktree's own project root |

### 3.4 Scenarios live in the definition

| decision | design |
| --- | --- |
| One file, scenarios in it | `[[scenarios]]` in the `rule.toml`, `spec.toml` or `template.toml` itself: `id` and `scenario` (the Gherkin Scenario line), `given` (the files, and where the asset is placed in them) and `then` (the findings expected, none for a scenario that passes). `fixtures/` directories and `behavior.feature` do not exist |
| Fewer files | One file per asset; a scenario is a table, not a directory |
| Shared fixtures once | What every scenario of an asset starts from is written once, in the asset's own `[given]`; a scenario writes only what differs, its keys winning and its files and params added (`tm explain scenarios`) |
| Every scenario writes out its files | `given.files."<path>" = "<text>"`, every file written out in the asset's toml; no `base`, no change verb. A `tastemaker.toml` among the files places the asset itself: a root that declares `workspace` and its dependencies, each `path` from the directory of the asset's file, and names each definition `package/name` (`tm explain scenarios`) |
| Both sides covered | Every asset has a scenario that passes (no `then.findings`) and one that fails (`then.findings` lists them), and every `id` is unique in the asset and says what the scenario covers |
| A rule per bug | Every new rule ships the scenario that reproduces its bug |

### 3.5 Selectors are a collection

| decision | design |
| --- | --- |
| One concept | Wherever a spec or entry names where something is, it names a selector |
| A collection | `domain/selectors/collections/selectors/<kind>/item.rs`, one item per kind, each implementing the collection's one item trait (parse its table, validate, resolve to paths at placement). A new kind is a new item directory. Recursion is literal: `{ part = "use-case", recursive = true }` |
| Closed set to start | glob string, regex, part, recursive part, placed-by; `exclude` subtracts at the collection level. `language` is cut: a rule is language-specific through the kinds it compares and the paths its entry places it on (rule.toml.md, Relations) |
| Why first | The language spec's 10/10 depends on it: the four language package rules return to the language spec as `{part}` selectors, and nu-hex `core-fakes` and `fixture-location` fire |
| Evidence | Rules reaching into nested parts today: use-case rename fires; `core-fakes` and `fixture-location` are blind; `plugin-schemas` did not fire |

### 3.6 Variables

| decision | design |
| --- | --- |
| Named values shared by rules | `variables = { name = [...] }` in a `tastemaker.toml` or spec; referenced as `{ variable = "x" }`. Captures stay `{x}`, params stay `{ param = "x" }` |
| Check | An undefined or unused variable is a validation error; each language's parser-library list is declared once and read by `imports-allow` and `client-at-adapter` |

### 3.7 Item trait param and the contract part kind

| decision | design |
| --- | --- |
| Item trait as a param | The `collection` spec takes `item-trait = "ryo::languages::ReadItem"`; no `interface.rs` is demanded; items must implement it. Resolves the ~240 findings of the item, collection, types and use-case specs against the engine-owned language types |
| Contract part kind | The language spec places `inbound/contracts/parse/` and `inbound/contracts/project/`: one file implementing a named ryo trait plus its test, checked as "implements `ryo::languages::Parse` / `Project`", instead of use cases (about 45 findings gone). Mirrors the item-trait decision |

### 3.8 Placed rows and spec rules over nested instances

| item | design |
| --- | --- |
| `tastemaker.placed` | The hosted relation of 2: `path`, `entry`, `package`, `template`, `version`, `spec` (empty for a `tastemaker.toml`'s own entry), one row for each (path, entry). A rule can join a file to the entry that placed it |
| Spec rules into nested parts | Through selectors (3.5); until then the three blind rules stay listed |

## 4. The command line

**Decided.** The typed command is flat (`add`, `create`, `scaffold`, `check`, `validate`, `update`, `explain`, `list`), `workspace` implicit, and in the code each command stays under its resource, `cli/src/resources/workspace/<verb>/`. `language list` is `-V`, `docs show|list|export` are `explain` and `list`; `remove`, `tree`, `coverage`, `outdated`, `docs search`, `--error-format` and `--capture*` are dropped. This is the existing `cli` spec (`src/resources/{resource}/{command}`) with `workspace` as the resource, so no shared scaffold changes. `-n` is global. `explain` resolves its argument as a path on disk, else `<kind>/<name>`, else a topic.

## 5. Definition formats

The four files are `rule.toml`, `template.toml`, `spec.toml` and `tastemaker.toml`, each one file, and none carries a version. What was decided about them:

| # | decision |
| --- | --- |
| 1 | `[[scenarios]]` are inline in the asset's own file: a rule, a spec and a template |
| 2 | Findings carry the line |
| 3 | Scenarios are self-contained |
| 4 | `category` is dropped |
| 5 | A template's content is inline in `template.toml` |
| 7 | `validate` runs the three generic template checks |
| 8 | Assets of the same package are referenced by name |
| 9 | **Decided.** Scenarios never look at the repository's own code. Every scenario of a rule, a spec or a template writes out all the files it needs, in the asset's toml, and depends on nothing but itself: no `base`, no render-then-edit, no change verb. `validate` writes each scenario's files to a project of its own, and checks it like any project |
| 10 | Selectors are a collection, and recursion is literal: `{ part = "use-case", recursive = true }` |
| 11 | Variables stay: moot now that the stubs are gone, and the feature is kept |
| 12 | The eight unused keys are removed |
| 13 | No definition carries a `version`: only a lock has one, `version = 1` |
| 14 | No check for the process a definition is written in; plain descriptions instead |
| 15 | A `version` key in a definition is an error: `` `version` is no key of a Tastemaker file: only a lock has a version; remove the line `` |
| 16 | A template's `match` is `"contains"` (the default, not written): the placed file holds the content's declarations and may hold more; `"declarations"`: it declares what the content declares and no more, its bodies free, and a `tastemaker.toml` placed by a `declarations` template declares what the template writes and nothing more, except that each of its `variables` may hold values beyond the template's; `"text"`: its text is the content's, byte for byte. `exact`, `verbatim` and `closed` are errors. The scaffolded bench files of `distribution` are `text` |
| 17 | A root file's `tastemaker = "X.Y.Z"` names the Tastemaker the workspace needs, and it is matched exactly, never as a range: `tm init` writes the version that ran, and any other version than the one installed is refused with `this repo needs tastemaker X.Y.Z; A.B.C is installed` |
