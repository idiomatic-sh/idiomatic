# Plan: map the PoC `executor` crate (T=2a207464c) onto the agreed Ryo tree

Status: reading is done (read-only, through a `git archive` copy in the scratchpad). The one output file,
`/workspaces/ryo-poc-evidence/next/build/ports/stage2/mapping/executor.md`, is NOT written yet: plan mode blocked it.
On approval: write that file in the six sections the BRIEF asks for, then send the 12-line final message.

## Verified facts (from the code, not from the notes)

- Crate: 111 non-test .rs files under src/, 7 workspace deps. `lib.rs` has 14 `pub mod`s.
- Operators: `Exec::step` (finish.rs:2041-2437) dispatches 34 operator names:
  base, select, union, intersection, difference, group-digest, antijoin, extend, project, axis, group-count, group-min,
  group-max, cycles, rank, closure, follow, ordered-match, as-of, span-between, span-bounds, words, source-matches,
  replace, capture, group-ordinal, render-syntax, subtree-class, prefixes, order, fits-rendered, rows, worklist, ordered-dfs.
  Output columns are computed per operator a second time in `Exec::schema` (finish.rs:1348-1586), and a third time as
  an abstract screening step in `pair_summary.rs` `Builder::step` (1335-1622). So operator knowledge lives in three places.
- Table: `finish.rs:148` `pub struct Table { names, tys, cols: Vec<Vec<u64>>, n }`, all fields public. `Ty` (101) has 6
  variants. `.cols` appears 81 times in finish.rs and in 25+ other files. Narrow helpers exist (`push_row`, `row`, `copy`,
  `col(name)`, `push_const_col`), but kernels mostly index `cols` directly.
- Where each operator is implemented, and how it touches layout:
  - Inline in `step`, direct `cols` use: union (2049), project, group-count, group-min/max, follow, span-between,
    span-bounds, replace, group-ordinal, rows, fits-rendered.
  - Inline, narrow only (`push_row` / `row` / `push_const_col`): extend, closure, render-syntax.
  - Own files, direct `cols` use: union_kernel.rs (3), antijoin_kernel.rs (9), set_ops.rs (intersection, difference,
    group-digest; 3), words.rs (1), capture.rs (3), prefixes.rs (3), tree_class.rs (subtree-class; 5), ordering.rs (order; 5),
    worklist.rs (9), ordered_dfs.rs (9), populations.rs (cycles, rank; 4), select_kernel.rs (16) with band.rs, keyed.rs,
    view_*.rs, cold_*.rs as access paths, walk.rs + finish.rs 3007-3384 (axis, 4).
  - as-of / ordered-match: `Exec::ordered` (finish.rs:3205-3316), direct `cols`. source-matches: 3316-3384.
  - base: finish.rs:2468 plus fact_view.rs, host.rs. antijoin: 2491 + antijoin_kernel.rs.
  - Function library (starts_with, ends_with, concat, matches, glob_any, lower, snake/pascal/camel/kebab, natural_lt,
    integer_successor, float_text_gte, numeric_gte): finish.rs:4180-4280; expression evaluator 4063-4180; E/Op 1118-1160.
- Traits (6 `pub trait`s):
  - `Source` check_session/source.rs:10, 19 methods; impls `Linked` (linked.rs:36), `Fake` (cfg(test)); callers: sdk entry.rs,
    batch.rs, check_session.rs. Names language-contract types (`l::NumberedVisit`, `ProjectRequest`, `Symbols`): not row/column.
  - `Provider` finish.rs:791, 4 methods; impls `Snapshot` (hosted.rs:16), `View` (reader_run/packet.rs:471); caller `Exec`.
    Returns `Table` from `fill`: column-shaped.
  - `FileFacts` finish/fact_view.rs:172, 1 method; impl `OperatorColumns` (packet.rs:89); caller check_session.rs:1551.
    Returns `Fact<'_>`/`Column`: column-shaped.
  - `TokenOrder` finish.rs:443, 2 methods; NO implementer anywhere in the repo and nothing assigns `Run.tokens`
    (checked with `git grep` over crates/ and tools/). Report only.
  - `Slots<T>` worker.rs:5, 3 methods; impls for `[T]`, `Vec<(usize,T)>`, `ReportSlots`, `PairAliases`; callers `visit`,
    `versions`, `worker`.
  - `ReportSelection` check_session.rs:231, 1 method (+FromIterator); impls `Vec<usize>`, `PairSlots`; callers `admit_ids`, `Batch::admit`.
- `Batch` (replay/batch.rs:102): 30 pub fields (not 36). `Store` (stored.rs:367): about 55 pub fields, 35 `pub fn` methods in
  `impl Store` (441-1493). `Store::admit/probe/reuse_closed/local/closed/prepare` take `&mut Batch` / `&World`, so the
  "store" is coupled to the driver in both directions (Batch holds `Active`, Store reads `Batch`).
- Disk and process I/O outside any outbound crate: check_session.rs:744/782/789 (reads source bytes, `versions`),
  worker.rs:117 (stat for shard weight), project.rs:327/414 (disk authority), stored.rs 447-463 (reads own exe, cache dir),
  shards.rs (mmap, 1 unsafe), replay.rs (env vars TM_CORPUS_*, TSV manifest, stdout), host_columns.rs:15, request_input.rs:35/199,
  finish/json_input.rs:47. Debug `println!`/`eprintln!` in lower.rs:35, emptyprop.rs, pair_summary.rs, pair_summary/cost.rs, exact.rs.
- Import-direction facts: `languages.rs` re-exports `ryo_language` (outbound contract) and `ryo_relations` and one
  `ryo_core` item (`FindingCoordinates`) as one vocabulary; `pair_checks`, `reader_run/packet`, `host_columns`,
  `check_session/*` all use it. So a `domain/execute` crate would import `outbound/language` types (not allowed).
  `lower.rs` returns the executor-owned `finish::Plan` (via `from_typed`), so `domain/plan` would import `domain/execute`.
  `ryo_query_language`, `ryo_matching`, `ryo_execution_domain`, `ryo_values::profile` (re-exported as `finish::operators`) are separate crates today.
- Callers outside the crate: sdk (`linter/lint/session/*`), 3 language benchmarks (ruff, clippy, oxlint), tastemaker
  `benchmarks/project/live/capture.rs` (the only caller of `replay::batch_mode*` and `one_source`).
- No caller found (to report, no recommendation): `TokenOrder` (no impl, never set); `FACT_IX_ROWS` (finish.rs:576, written at 1025,
  never read); Cargo feature `pair-diagnostics` ("nothing enables it today"); `Stats` fields only via VEC_STEP_TIME print. All checked with `git grep` at T.

## Proposed row groups for the table (file ranges verified; destinations are proposals)

domain/operators (medium): union_kernel, antijoin_kernel, set_ops, words, capture, prefixes, tree_class, ordering, worklist,
ordered_dfs, populations, inline_key (hashing helper for group keys, low); `glob_pattern.rs` (41, glob -> regex, used by select and
by pair_summary: two consumers, medium); finish.rs 4063-4280 function library; 4280-4368 natural_lt / fits.
domain/execute (medium to high):
- finish.rs 1118-2040 and 2041-2437 (Exec, step driver), 2438-3384 (base, antijoin, select, axis, ordered, source_matches),
  3384-3790 (view and bucket helpers), 3991-4062 (expr utils); plan_load/plan_args/plan_select (plan adapter),
  select_kernel, band, keyed, filter_order, known, emptyprop, empty_sel, empty_why, frame, lazy_view, leaf_scan, walk,
  view_*, cold_* (access paths over provider columns), fact_view, subject_scope, partition (file-local vs project split),
  finish/session.rs + session/* (project phase), answer.rs + answer/report.rs (row decoding; also a lint report type, split medium).
- first.rs, reader_run.rs (+pairs.rs), pair_skip.rs, pair_checks.rs (+counts, witness), pair_summary.rs (+9 files): screening.
  pair_summary.rs 240-1622 `Builder` is the largest single concern (per-operator abstract interpretation).
- check_session.rs: 1-140 Program; 146-230 Clock; 250-535 Schedule (admission, source ids) ; 549-706 bind; 864-1654 `visit`
  (read bytes, screen, evaluate, contribute; one 790-line function). Mixed: reading bytes -> outbound/corpus (726-840 `versions`),
  clocks/metrics -> nobody; the rest execute.
- check_session/{completeness, file_programs, named, shared, report_slots, refusal, kinds, query, pairs, pairs/atoms,
  frontier 1-185, replay/readers, replay/closed_workers}.
outbound/store (medium): finish.rs 100-295 (Ty, V, Table, pool, PoolBudget), 3787-3990 (Index); recycle.rs; population.rs (710, Block, Words,
  Population; layout-bound columns); frontier.rs 186-555 (World, carry, transfer); fill_len? no (execute).
outbound/cache (medium): replay/stored.rs 16-440 and 441-610, 1495-1512 (keys, wire types, open, publish); stored/shards.rs;
  stored/request_answers.rs (Packet, Record, Proof: validity proofs); stored_block.rs.
  replay/stored.rs 611-1494 (admit, probe, reuse_closed, local, closed, proofs) is replay decision logic bound to `Batch`: SPLIT or NO HOME.
outbound/scheduler (medium): check_session/jobs.rs (23, high); worker.rs 60-238 sharding by weight + `jobs::run` (the visit call and
  clock merge inside it are execute).
outbound/corpus (low to medium): check_session.rs 726-840 (`versions`, reads bytes, hard-link dedupe); project.rs `Authority::observe`,
  `Environment` (disk entry observation; 277-350, 583-651); source.rs `routed` / `takes_environment` (path -> language routing).
outbound/language (medium): source.rs (Source trait, 19 methods), linked.rs (Linked); `languages.rs` (78) is a re-export hub: NO HOME.
inbound/lint (low): request_input.rs (Hosted, HostCell, Project, Parameters: request types plus benchmark JSON reading), hosted.rs,
  replay/batch.rs `Input`/`Work`/`Admission`/`Batch` request admission (Batch itself is a 30-field mix: SPLIT, low).
domain/plan (medium): lower.rs (parse pack, `lowered`, `mark_through`, `finish_plan`, `query_batch`, `SharedPlanData`).
tools/benchmark (medium): replay.rs (`batch_mode`, `one_source`, TSV, env vars, receipts; only caller is tastemaker benchmark),
  benches/lint_image/*, bench_support.rs (6, `thread_cpu`; also used by sdk session.rs: NO HOME or tools, low),
  json_input.rs, json_output.rs (benchmark JSON), finish.rs 20-40 step_clock and VEC_* debug switches.
NO HOME candidates: languages.rs; refusal.rs (error for execute, validate and lint); `host_columns.rs` + `finish/host.rs` (filesystem
  facts supplied by the host: not a language, not corpus); `Clock`; debug printing in emptyprop/empty_*/pair_summary cost.

## Section 5 (contracts): table of the 6 traits above, each with name, path:line, method count, implementers, callers, row/column-shape.

## Section 6 (direction conflicts) to list
1. execute -> outbound/language types (languages.rs, pair_checks, packet, project, source).
2. plan -> execute (lower.rs builds `finish::Plan`).
3. operators -> store layout: kernels index `Table.cols` directly (violates "layout belongs to store, injected").
4. cache -> execute: `Store` takes `&mut Batch`, `&World`, `Block`, `Population`; `Batch` holds store keys.
5. execute -> corpus: `versions`, `Authority::observe`, and shard stat read the disk from inside the visit.
6. execute -> scheduler: worker.rs calls `visit` from inside the sharding closure.
7. operators and execute are interleaved in finish.rs (one file holds the Table type, the step driver, the expression evaluator).

## Still to do before writing the file
- Compute the per-destination line totals with a small script over lines-per-file.txt (finish.rs and the split files by range).
- Spot-check low-confidence rows: pair_summary/exact.rs, pair_summary/cost.rs, stored/request_answers.rs, replay/closed_workers.rs.
- Report "not checked" for: bodies of cold_*.rs, view_*.rs, band.rs, keyed.rs, pair_summary/exact.rs; `benches/` internals.
