# Shared vocabulary: the types most sheets name, one owner each (bead ryo-fmx3.1.13.23.4.47)

DRAFT, 28 Nov, for the owner to agree first: agreeing these once unblocks most sheets. Taken from the 21 drafted
`api.txt` listings (top-level definitions only). "Named in" counts the listings that use the type.

## Owned once, already

| Type | Owner | Named in | What it is |
|---|---|---|---|
| `CellWord` | domain/table | 6 | one cell of a table, a 64-bit word whose meaning its column's kind gives |
| `CellKind` | domain/table | 6 | what a column holds: node, symbol, file, span, number, flag |
| `RowNumber`, `RowCount` | domain/table | 5, 4 | a row's position in a table; how many rows |
| `ColumnNumber` | domain/table | 5 | a column's position |
| `Shape` | domain/table | 5 | a table's column names and kinds |
| `Position` | domain/table | 5 | a node's or token's position in a file's tree |
| `Symbol` | domain/symbols | 8 | a run-wide text number (the symbols sheet proposes renaming it, because main's `Symbol` is the per-file number) |
| `Relation` | domain/relations | 8 | one table of the catalog |
| `FileId` | domain/relations | 6 | a file's number in a run |
| `RuleOrdinal` | domain/operators | 5 | a rule's position in a load |
| `Demand` | domain/operators | 4 | what a step needs of an input: these columns, these keys, or only "is there a row" |
| `RuleId` | domain/plan | 4 | a rule's name |
| `WorkerCount` | outbound/scheduler | 5 | how many workers |

## Named but owned by no sheet yet (a finding: each needs one home)

| Type | Named in | Proposed owner | Why |
|---|---|---|---|
| `RelationName` | 7 | domain/relations | a relation's written name is catalog knowledge |
| `HostedRelation` | 6 | domain/relations | a relation the host supplies (the ryoql and plan sheets also say relations) |
| `KindVocabularies` (and `KindVocabulary`) | 6 | domain/relations | the kind words each language declares; validate and plan both need them |
| `Key` | 6 | outbound/cache | the 32-byte key of a kept answer; only cache makes keys |
| `ColumnIndex` | 2 | none: use `ColumnNumber` | two names for one thing |
| `StepNumber` | 1 | domain/plan | a step's number in a plan |

## Owned twice (from `check-types.py`; the review settles each)

`Column` (relations, lint), `ExecutablePermission` (lint, corpus), `ObservationProducer` (plan, lint), `RuleEmit`
(plan, programs), `RuleRefusal` (execute, programs), `StepCount` (execute, plan).

## Names that clash (one word, two meanings)

| Word | Meaning 1 | Meaning 2 | Proposal on the sheets |
|---|---|---|---|
| `Symbol` | language's per-file name number (main, the SDK) | the run's text number (symbols) | symbols sheet: crate `dictionary`, type `TextId` |
| `ProjectUseCase` | a language's project facts | the cross-file phase | project sheet: crate `cross_file`, `CrossFileUseCase` |
| `PlanUseCase::plan` | operators' preparing a plan | the crate `plan` | operators sheet: call it `prepare` |
| `Tree` / `FileTree` | language sheet | store sheet | one name, chosen in the review |
