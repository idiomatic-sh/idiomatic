# parallel-members 0.1.0

The first release. The principle: colocation is by
domain, not primitive. The same set of names under two parents (the same ten query
operators under the planner, two evaluators and a conformance check; the same five
comparisons under two evaluators and the planner) is one set of concepts split by role, so
every change to one of them is made in each copy.

A member is a child directory that holds a file and has its M1 module file
`<parent>/<name>.rs` beside it; its name is not in `roles`. Two parents that share three or
more member names are split: each shared member is one finding at its module file, naming
the other parent's copy. A pair whose two copies both lie under a directory named in
`independent` mirrors by design and is not a finding.

## Placing it

The `idiomatic-rust` spec, over the whole product, with the trees a spec places as mirrors
left out by the entry:

```toml
{ package = "idiomatic.arch", version = "0.1.0", rule = "parallel-members", include = ["**"],
 exclude = ["**/src/commands/**", "**/src/resources/**", "**/benches/**", "**/tests/**"],
 params = { roles = ["adapters", "methods", "commands", "macros", "tests", "benches", "fixtures", "src",
 "domain", "inbound", "outbound", "application", "use_cases", "contracts", "collections"],
 independent = ["languages", "relations"] } }
```

| exclusion | mechanism | why |
|---|---|---|
| `adapters`, `methods`, `commands`, `macros` | `roles` | N1's namespace directories: every hex, adapter, CLI and macro crate repeats them |
| `tests`, `benches`, `fixtures`, `src` | `roles` | a crate's structural directories, repeated by every crate |
| `domain`, `inbound`, `outbound`, `application`, `use_cases`, `contracts`, `collections` | `roles` | the nu-hex layer directories, repeated by every hex |
| `**/src/resources/**`, `**/tests/**` | entry `exclude` | the CLI's resource tree mirrors the service hexes, and test trees mirror what they test |
| `**/src/commands/**` | entry `exclude` | the CLI is resource then verb, so verbs repeat under each resource and resources mirror the service hexes the CLI layer calls (the cli spec) |
| `**/benches/**` | entry `exclude` | bench trees mirror the commands, outside `src` because Cargo requires it (the cli spec places them) |
| `languages`, `relations` | `independent` | independent language providers implement the same ports and fill the shared relation shapes, so `relations/<relation>/` and `languages/<p>/src/<relation>/` mirror by design |

## Limits

The rule reads paths, not code: what is not an M1 module tree (a directory with no
`<name>.rs` beside it) is not read. Paths of any depth are read: parents through a
`{*parent}` capture, and the directories named in `independent` through a capture that
answers a row for each directory of a path.

## Fixtures

`split-concept`, `three-parents`, `four-shared`, `root-parent`, `commands-included`,
`provider-and-core` fail;
`two-shared`, `roles`, `no-module-file`, `leaf-modules`, `commands-excluded`,
`independent-providers` and `independent-deep` pass.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.
