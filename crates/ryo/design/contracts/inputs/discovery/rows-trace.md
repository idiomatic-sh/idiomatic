<!-- a read-only reader's final report, 27 to 28 Nov 2026; task ad6d5beceaf72e737; copied from the session transcript -->

# Why Ryo builds too many rows: causes at trunk 2a207464c and what the record measured

Three things build the rows: every step that runs in a file makes its own table and keeps it until the file ends; cross-file rules carry a table out of every file and copy it again at the end; the kept store writes those carried tables to disk. What is needed is passed down by relation, and for cross-file inputs by column, but never by key, and the language sees relations only. The one counter of rows built never leaves the engine, and no rows-built total is on record for this trunk.

Path prefixes (code is `/workspaces/idiomatic-public-benchmark` at 2a207464c):
- `X:` = `crates/ryo/crates/executor/src/`, `L:` = `crates/ryo/crates/language/src/`, `PY:` = `crates/ryo/languages/python/src/`
- `REL:` = `crates/ryo/crates/relations/src/domain/`, `QL:` = `crates/ryo/crates/query_language/src/`, `SDK:` = `crates/ryo/crates/sdk/src/resources/linter/lint/`
- `S:` = `/workspaces/ryo-poc-evidence/next/engine/STATE.md`, `R1:` = `/workspaces/ryo-poc-evidence/next/RESULTS-STAGE-1.md`
- `W121:` = `/workspaces/ryo-poc-evidence/next/build/tm-int/WINDOW-RESULT-BUILD121.md`, `INBOX:` = `/workspaces/ryo-poc-evidence/next/build/codex/typescript/INBOX.md`

Every number below comes from an earlier build (date or line given). "grep" means I searched the trunk and found no other use.

## 1. Fact production
- **Who decides:** the executor, from the lowered plan. The plan's base steps give `Program.bases` (`X:check_session.rs:108-113`); `bind` turns the filled relations into a mask (`X:check_session.rs:658-664, 680`); per file the masks of its bound programs are OR-ed (`X:check_session.rs:1037`).
- **Is it passed to the language:** yes, as a list of relations: `reader.visit(request, &requested, ...)` (`X:check_session.rs:1065, 1621-1623`; contract `L:inbound/use_cases/parse/named.rs:161-165`). No columns, kinds or keys are passed.
- **Relation rows:** always filled whole with every column (`Wanted::Whole`, `X:reader_run/packet.rs:210, 251`; the executor never asks `Wanted::Key`, grep). They are filled at first read when the language says the file is complete (`X:check_session.rs:1209-1217`; `PY:inbound/contracts/walk/fillers.rs:261-272`); otherwise every requested relation is filled up front (`X:check_session.rs:1218-1223`).
- **Node table:** every node and token with all columns is written for every file, whatever the rules read (`L:inbound/use_cases/walk/numbered.rs:132`; `L:domain/node_table/types.rs:38-60`). Python reads the relation list only to choose syntax-error reporting (`PY:inbound/contracts/walk/contract.rs:154`).
- **Words and fields:** one text word per element is added when any bound program reads syntax-tree, semantic-syntax or token-link (`X:check_session.rs:630, 1094, 1142`; `L:.../numbered.rs:150-154`). `FieldRequest` is set (`X:check_session.rs:1083-1087`) but no walk reads it (grep).
- **Catalog hints:** the four tree relations are read in place, not filled (`REL:relations/types.rs:401-406`; `X:reader_run/packet.rs:205`). `size` and `keys` are read by no engine code (only `L:domain/fillers/fixtures/growth.rs:13, 158, 257`); form, extent, notes and scope are read (`X:check_session.rs:642, 658, 1468`; `X:check_session/project.rs:62`).
- **`relations_read` / `operators_used`** (`QL:domain/syntax/service.rs:286, 336`) feed only `query validate` (`QL:inbound/use_cases/typecheck/use_case.rs:163-167`), not lint.
- **Numbers:** Django has 2,874 files and 6.28 M elements (`S:1825`); its fact layer was 8,291,016 rows (`R1:31`). On an older engine, 51 relations gave 44.9 M rows, 31.6 M of them five relations restating the tree (`S:210`). At T: not measured.

## 2. Evaluation of one rule on one file
- **Sharing in the file phase:** intermediate tables are shared between rules. All bound programs' steps are interned into one plan per reader visit (`X:check_session/shared.rs:42-54`; `QL:domain/plan_lowering/shared.rs:19-115`), with one `Exec` (`X:check_session.rs:989-990`) and one memo per file (`X:check_session.rs:1556`; `X:finish.rs:1696-1719`). The exception is a partly read file, which gets a fresh frame per rule (`X:check_session.rs:1558`).
- **Sharing in the project phase:** none across rules. There is one `Exec` per rule (`X:check_session/replay/batch.rs:781-784`; `X:check_session/replay/closed_workers.rs:272-273`); reuse covers only tables of at most 128 cells, 512 entries (`X:finish/session/reuse.rs:33-43`).
- **Materialised:** each of the 34 operators returns an owned `Table` (`X:finish.rs:2041-2436`), held in the file memo until the file ends (`X:finish.rs:1718, 733-744`). Nothing releases a table after its last consumer (grep `consumers(`).
- **Select over a base relation:** with plain picks and tests it is a view read in place, with no copy (`X:finish.rs:1160-1162, 1641-1679`). It is copied once a consumer wants a table (`X:finish.rs:1887-1907`).
- **Joins and other selects:** kept rows are gathered column by column into a new table (`X:finish/select_kernel.rs:1079-1098, 1110-1126`; `X:finish.rs:2961-2987`).
- **Other copies:** `extend`, `span-between`, `span-bounds` and `replace` copy every input column (`X:finish.rs:2074, 2256, 2271, 2291`); `project` copies row by row (`X:finish.rs:2090-2108`); `base` copies the fill (`X:finish.rs:2482, 2486`). `union` concatenates unless one leaf passes through unchanged (`X:finish/union_kernel.rs:214-217`).
- **Lazy or skipped:**
  - a fact view stays unfilled until needed (`X:finish/lazy_view.rs:17-46`);
  - an identity select hands its input on (`X:finish/select_kernel.rs:685-688`);
  - a single-consumer step is read by key (`X:finish/keyed.rs:1-10`);
  - a closure is read by adjacency (`X:finish.rs:1203-1212`);
  - semi and counted joins make no pairs (`X:finish/band.rs:1-5`; `X:finish.rs:2003`);
  - known-empty steps and empty-input gates return a shared empty table (`X:finish.rs:1735, 1740-1751`; `X:finish/known.rs:70`; `X:finish/emptyprop.rs:1-12`).
- **Screening:** it removes (rule, file) pairs before any step runs (`X:check_session.rs:1175, 1286`) and can publish findings straight from the tree (`X:check_session.rs:1305`).
- **Recycling:** a per-thread column pool capped at 32 MB and split per worker (`X:finish.rs:155-178, 258-275`); answers and column lists (`X:finish/recycle.rs:76-133`); all file buffers at file end (`X:finish.rs:733-777`).

## 3. The project phase
- **Frontier:** for each (file, cross-file program) one block per cut input, meaning the outputs of file-local steps that wider steps read (`X:check_session/frontier.rs:61-182`; cut in `X:finish/partition.rs:36-45`). Blocks sit in `World.frontiers` (`X:check_session/frontier.rs:204-212`) until the last closed reader releases them (`X:check_session/replay/batch.rs:772, 865`).
- **Populations and notes:** project relations per project, limited to the project-scope relations the rules read (`batch.rs:150-155, 347, 727-751`), and notes per file (`X:check_session.rs:1120`).
- **Bounded by columns, not rows:** unread columns travel as a null constant (`frontier.rs:22-59, 329`). Every row of the cut input in every file is carried, so the set grows with files times cross-file programs.
- **Copied again:** `Population::full` copies all pieces into one table of 8 bytes a cell, expanding even the null columns (`X:finish/population.rs:139-141, 415-427`).
- **Record on scaling:**
  - dagster closed inputs 594.82 MiB, about 101 KB per added file version (`S:8522-8523`, 25 Nov);
  - quarter against full dagster: 426.2 against 1,040.0 MB, +129.6 KB a file; the growth was packed project columns +368.9 MB, operator workspace +125.3, closed tables +87.2, blocks +33.8 (`S:9245-9249`);
  - after carrying only what is read: +28.8 KB a file (`S:9409-9410`);
  - one edge on the real check fell from 12,021,121 to 140,064 rows, for a reader that touches 427 positions (`S:8524-8525`).
- **Peak at build 121:** django 439.3, pandas 399.2, fastapi 269.0, dagster 480.5 MiB; Tastemaker check 1,312.7 (`W121:9-12, 17`). Bead ryo-fmx3.1.16.3 gives Django 435 MB against Ruff 105. At T: not measured.

## 4. The kept store
- **Per file version:**
  - `files`: per binding, `Local{packed report rows, issues, frontier references, unavailable}` (`X:check_session/replay/stored.rs:53-59, 112-117, 1290-1367`);
  - `blocks`: each frontier block with its words (`stored.rs:1320-1349`; `X:check_session/stored_block.rs:41`);
  - `notes` (`stored.rs:105-110, 1277`).
- **Per request:** a record (packet key, proofs, unread) and a packet of final rows (`stored.rs:685-747`), plus groups, stamps, paths and binding tables (`stored.rs:338-353, 541-606`).
- **Not written:** no node table, no fills, no intermediate table.
- **Which pairs:** a pair is kept when its work is cross-file or its request has more than one pair (`stored.rs:1026`).
- **What the bytes are:** on fastapi, 53 MB = blocks 48.9 MB (frontier columns of cross-file programs), pairs 3.5, project 0.3, answers 0.22, rest 0.3 (bead ryo-fmx3.1.11.14; `INBOX:4524`). Only 7 of 413 Ruff programs are not file-local (`INBOX:4673`).
- **Sizes:** django 289.6, pandas 378.4, fastapi 53.5, dagster 518.2 MB against 0.69, 0.62, 0.26, 1.72 (bead ryo-fmx3.1.16.10; `S:10767`). The findings-only port wrote 0.84 to 3.7 MB (bead .11.14). Tastemaker: 291,426,101 against 2,482,937 bytes (bead ryo-fmx3.1.16.11).

## 5. Counters
- **`Stats{total_runs, total_rows}`** (`X:finish.rs:1118-1121`): rows are added at `finish.rs:1757, 1916, 2010`, `X:finish/keyed.rs:123` and `X:finish/view_input.rs:51`.
  - Runs and nodes made go out: `Clock` (`X:check_session.rs:1651-1652`), then event `ryo::node_counters` (`SDK:session/entry.rs:404-405`), read by `crates/ryo/crates/benchmark/src/application/profile_evidence.rs:325-331`.
  - `total_rows` is only printed to stderr under `VEC_STEP_TIME` (`X:check_session.rs:1653`); nothing reads it.
- **Operator ledger** (runs, rows in, rows out, CPU): `crates/ryo/crates/values/src/profile.rs:83-160`, with a parser at `profile_evidence.rs:373-395`. `configure` has no caller (grep), so it is never written. The per-node `rows_in`/`rows_out` events the benchmark expects (`profile_evidence.rs:332-339`) have no emitter.
- **No reader or no filler:** `FACT_IX_ROWS` (`X:finish.rs:576, 1025`); language fill counts under `RYO_X_FILLS` (`L:domain/fillers/service.rs:180-256`); `FeatureFed`, `RuleWork`, `StepFed` are types only (`crates/ryo/crates/core/src/inbound/use_cases/lint/types.rs:174-195`).
- **Rows per run on record:**

| run | django | pandas | fastapi | dagster | source |
|---|---|---|---|---|---|
| round 55, 23 Nov (M rows) | 75.1 | 131.9 | 8.1 | 166.3 | `S:4292-4295` |
| round 55 step runs (M) | 1.40 | 1.51 | 0.51 | 4.68 | `S:4295` |
| round 33 (M rows) | 91.2 | 138.9 | 13.1 | 181.1 | `S:3608` |
| rule rewrites, start to head (M rows) | 288.5 to 156.2 | 438.6 to 248.7 | 45.9 to 25.7 | 611.1 to 336.3 | `S:2822-2823` |

  Dagster's first harness run was 629.8 M rows (`S:2813`). At T: not measured.

## 6. The causes, ranked
| # | cause of rows that need not be built | where | evidence it is large | concept that owns the decision | what must know what |
|---|---|---|---|---|---|
| 1 | Store keeps every file's carried blocks, not findings | `stored.rs:1026, 1320-1349` | 48.9 of 53 MB on fastapi; 53 to 518 MB a repository (beads .11.14, .16.10) | store | the store must learn from the plan which programs are file-local (`batch.rs:202-209`) and keep findings for those |
| 2 | Cross-file inputs: all rows of every file carried, then copied whole | `frontier.rs:61-182, 204-212`; `population.rs:415-427` | 594.82 MiB, about 101 KB a file (`S:8522-8523`); +368.9 and +87.2 MB (`S:9248-9249`); at T not measured | plan (the cut) | the cut must know the wider step's keys and tests, so a file hands over only rows that can match |
| 3 | Each step makes an owned table kept to file end; copying operators | `finish.rs:1718, 2041, 2074, 2256, 2271, 2291, 2961` | 75 to 166 M rows a run (`S:4295`); 235 M of 566 M rows were copies of base rows on the old engine (`S:44-48`); at T not measured | operators | a step must be told by its consumers which columns, which keys, or only whether a row exists |
| 4 | Rules written as whole-file tables that re-derive language facts | rule packs; no keyed fill asked (`packet.rs:210`) | 245.8 of 524.7 M rows (`S:2111`); four rewrites cut 5 to 490 times (`R1:94-99`) | relations catalog and ryoql | the planner must see the catalog's `keys` and `size` to swap a derived table for a keyed fact |
| 5 | Project phase evaluated once per rule and per request | `batch.rs:781-784`; `reuse.rs:33-43` | capture: 34 M rows in, re-run per request; select 15.6 M in, 17.0 M out (`S:8896-8899`) | plan | the closed phase must know which steps are equal across rules and requests |
| 6 | Language writes the whole node table and whole relations with all columns | `numbered.rs:132, 150-154`; `packet.rs:210, 251`; `check_session.rs:1218-1223` | 6.28 M elements on Django (`S:1825`); source-location filled for every file when 387 of 7,379 pairs certify (`S:9251-9253`); at T not measured | language contract | the request must carry columns and kinds per relation; today it carries relations only |
| 7 | Findings held twice | `SDK:session/entry.rs:736-768, 846`; `stored.rs:57, 685-737` | 3.5 + 0.22 MB of 53 (bead .11.14); in memory not measured | lint and store | the store must know the packet already holds the rows |

Candidates rejected from the code and record:
- **Facts filled that no loaded rule reads**, at relation level: the mask and first-use fill prevent it (`S:8967-8975`).
- **Same sub-plan evaluated once per rule**, in the file phase: steps are interned.
- **Node table kept across files:** parser and file elements fell from 42.1 to 4.2 MB (`S:9247`).

One caution from the record: rows alone are not the CPU cost. Cutting rows 16 to 20% moved CPU 0 to 7% (`S:2942-2945`), and about 70% of step runs return nothing (`S:3166`).

## 7. Boundaries that need demand passed down
- **application/execute to outbound/language:** today a list of relations plus two flags. It needs columns, kinds and keys per relation, and which node-table parts. The plan already computes column use (`QL:domain/plan_lowering/readers/service.rs:166`) and the catalog holds form, keys and size.
- **domain/plan to domain/operators:** an operator's output contract is "a whole table". By-key and exists-only reads exist only as special cases inside execute (`keyed.rs`, `band.rs`, `lazy_view.rs`).
- **domain/plan to application/execute (project phase):** the cut passes column masks but no row or key demand.
- **application/execute to outbound/store and outbound/cache:** the store is handed the session's frontier. It needs "what a rerun must replay", which the plan's file-local flag answers.
- **application/execute to inbound/lint:** rows built stop at stderr. The typed work report exists in core, but nothing fills it.
- **outbound/corpus and outbound/scheduler to application/execute:** one source per file version (50 to 263 sources, bead ryo-fmx3.1.16.9) and one closed evaluation per request. Execute needs to be told which inputs are the same.

### Critical Files for Implementation
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/executor/src/finish.rs
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/executor/src/check_session.rs
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/executor/src/check_session/frontier.rs
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/executor/src/check_session/replay/stored.rs
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/language/src/inbound/use_cases/parse/named.rs
