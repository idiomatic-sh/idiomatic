<!-- a read-only reader's final report, 27 to 28 Nov 2026; task a7324778c17c21e84; copied from the session transcript -->

# Ryo performance history by concept (read-only research; nothing edited, built or run)

Legend: `S:n` = `/workspaces/ryo-poc-evidence/next/engine/STATE.md` line n; `T` = `build/redo-int/TABLE.md` round history; `KR` = `build/ports/KEEP-REJECT.md`; `M` = `build/codex/MECHANICS.md` "What has lost and what has won"; `TH` = `build/ports/stage2/TRUNK-HISTORY.tsv` + `gates.tsv` (all under `/workspaces/ryo-poc-evidence/next/`). `E/` = `crates/ryo/crates/executor/src`; the same files lived at `crates/ryo/crates/engine/benches/vec` and, from 11-22, at `crates/ryo/languages/python/benchmarks/vec` (`git log --follow` on `finish.rs`: 165 commits, first 57f570d50b "THROWAWAY vec finish"). `P/` = `crates/tastemaker/benchmarks/project`; `ops` = `crates/ryo/crates/operators`; `py` = `crates/ryo/languages/python`. Figures are total CPU-s on one worker unless said; dj/pd/fa/dg = django/pandas/fastapi/dagster; "tm" = the Tastemaker check. Dates are 2026.

## 1. The changes, most recent first
| # | Date | What changed | Concept | Files touched | Measured effect | State | Source |
|---|---|---|---|---|---|---|---|
| 1 | 11-27 | Old engine, operators, derived and execution_runtime crates deleted; one lowering moved into query_language; start-up trial | plan lowering; start-up | 556 and 623 files | tm 20.640→20.668 and 22.885→22.618; start-up 341.3→317.9 ms; fa 1.911→1.906 | landed | TH builds 138-139 |
| 2 | 11-26 | Fast bench composition moved into the product; first attempt published the store after every call; kept cache holds the whole session | SDK edge; kept store | sdk, core, `E/check_session/stored.rs` (suspect 964de529e) | 52.4 vs 28.3 CPU-s, store 291 MB vs 2.5 MB; kept cache dj 690 KB→290 MB, dg 1.7→518 MB | landed; cache bug open | memory `fastest-implementation-is-the-base.md`; S:10964; bead ryo-fmx3.1.11.14 |
| 3 | 11-26 | Python-only relations replaced by general forms (I001 last) | fact filling; plan shape | py provider + rule pack (builds 126, 133) | fa +12%, dg +16 to +27%; I001 alone 0.22→0.455 s | landed, regression open | bead ryo-fmx3.1.16.13 |
| 4 | 11-26 | Project environment built only for files whose language uses it | cross-file; driver | `E/check_session` (4d46c2f80) | before: sources 6,205→26,183, parses 8,660→25,762, +70 MiB, +25% CPU | landed build 112 | bead ryo-fmx3.1.10.24 |
| 5 | 11-25..26 | Nine languages moved to the numbered column visit; rows arm and bridge deleted | fact filling (hand-off) | `crates/ryo/crates/language` named.rs, every `languages/*`, `E/reader_packet.rs`, `E/check_session/source.rs` | Go check 15.784→14.401; hono 1.384→1.286; vite 2.245→1.997; ripgrep 0.559→0.516 (loaded) | landed builds 92-125 | KR row 14 |
| 6 | 11-25 | Three drivers joined into one session driver, base chosen by generality | per-file driver; admission | `E/check_session.rs`, `E/check_session/*` | dj 5.768→7.955, pd 7.008→9.706, fa 0.941→1.702, dg 10.998→15.146; dj peak 272.3→435.1 MiB; start-up 73.6→295.5 ms; tm 15.514→13.435 | landed; Python regression open | KR summary; S:9806 |
| 7 | 11-25 | Rules proven file-local leave the cross-file phase; closed inputs lent per file version, released after the last reader | cross-file; memory | `E/check_session/frontier.rs`, `project.rs`, `E/finish/population.rs`, `partition.rs` (17 files) | dg 26.127→22.124 and 1,245.4→477.0 MiB; fa 3.371→2.768 and 331.1→266.9 MiB | landed build 78 | S:9297 |
| 8 | 11-25 | Pack lowered once per request | plan lowering; admission | `E/lower.rs`, `check_session.rs`, `check_session/sdk.rs`, `batch.rs` | fa lowering 888→586 M instructions (old driver 286 M) | landed build 73 | S:9046; 28017029a |
| 9 | 11-25 | `keep` switch, parked parse cache and retention eviction removed | driver; memory budget | `E/check_session.rs`, `retention.rs`, `allocated.rs`, `batch.rs`, `P/live/calls.rs` (10 files) | ryo check 19.00→16.61, 1,747.6→1,478.5 MiB | landed | 887a8531e; KR row 13 |
| 10 | 11-25 | Per-request blake3 hashing and call cache | kept store; admission | `E/check_session/stored.rs`, `call_cache.rs`, `query.rs`, lint_image `sdk_request.rs` | +600 M instructions on Root; 205 of 46,831 calls reused | landed, unjustified | S:8862; KR list 4 #2 |
| 11 | 11-25 | Empty-answer verifier removed from screening | screening | `E/finish/session_certificate.rs` (-332 lines), `check_session/pairs.rs` | hono pairs 4,191 (old 4,196); fa still 27,818 run vs 17,511 | landed | S:8582; KR row 5 |
| 12 | 11-25 | Fill on first use; a two-step fill tried and reverted | fact filling; screening; driver | `E/reader_packet.rs`, `check_session.rs`, `pairs.rs`, py `columns.rs` + `numbered.rs`, language `named.rs` | gain not measured; 4,833 failed TOML/JSON calls before 129168bdb | landed | 4b6cf1c93; d0b8ea76b; KR row 6 |
| 13 | 11-24..25 | Many workers: plan, layout, constants, patterns, symbol tables frozen once and shared; only the memory budget was ported | workers; memory budget; interning | `E/workers.rs`, `finisher/worker.rs`, `finish/worker_schemas.rs`; ported `E/finish.rs`, `check_session/worker.rs` | ten cores: fa 3.619→1.380 and 838→387 MiB; dj 12.635→6.866 | parked (19 of 20 picks conflicted) | KR good part 1; S:9221 |
| 14 | 11-24 | Warm store: findings by file version, full hit before preparation | kept store and warm | `E/retained.rs`, then `check_session/stored.rs`, `shards.rs`; `P/stored.rs` | unchanged dj 62.0→32.1 ms, dg 104.5→49.6; one edit dj 177→2,225 ms | landed; edit path open | S:8606; KR row 10 |
| 15 | 11-24 | Several files through one operator start | driver; kernels | `E/finish/batch.rs`, `check_session/file_batch.rs` (+910/-83) | slower in all five pairs, median +0.894 | lost | S:7234 |
| 16 | 11-24 | Scalar predicate pushed through an inner join into the scan | plan lowering; select | engine `plan_lowering/pushdown.rs`, plan_codec, `E/finish/select_kernel.rs`, `view_join/probe.rs` | 4,659,443 fewer rows; 23.887→23.267 (paired -0.036) | kept | S:7262 |
| 17 | 11-24 | The check submits its calls in three rounds | SDK edge; admission | `P/live/calls.rs`, `round-host.rs` | Ryo part 29.013→25.541 (loaded) | landed | S:7056 |
| 18 | 11-24 | Typed admission and packed answer rows | answers; admission | `E/finish.rs`, `population.rs`, `frontier.rs`, `P/readers.rs` (26 files) | peak 1,845.1→1,510.0 MiB; answer rows 329 MB→58 MB; CPU flat | landed | S:6217 |
| 19 | 11-24 | Pair screen across the whole session | screening | `E/check_session.rs`, `check_session/pairs.rs`, `pair_checks.rs` | 22.34→17.04, later 36.97→30.04; evaluated pairs 557,238→199,111 | landed | S:5691; S:6480 |
| 20 | 11-24 | One shared plan per reader; step results reused per file version | plan shape; driver | engine `plan_lowering/shared.rs`, `E/check_session.rs`, `check_session/shared.rs`, `finish/session.rs` | 64.064→52.952; step runs 4.71→4.04 M | landed | S:6358 |
| 21 | 11-24 | Literal glob compiled once, answer reused per interned subject, glob visible to the screen | scalar functions; interning; screening | `E/pair_patterns.rs`, `pair_regexes.rs`, `check_session/pairs.rs`, `pair_summary.rs` | 57.226→36.237; 133.6 M glob calls before | landed | S:6180 |
| 22 | 11-24 | File-version columns and indexes set up once; symbol predicates bound once | table layout; scalar functions; interning | `E/finish.rs`, `reader.rs`, `finish/symbol_predicate.rs`, `cold_text.rs`, `pair_checks.rs` | set-ups 557,238→24,973 (-5.645); regex calls 325.3 M→8.7 M (-2.338) | landed | S:5762 |
| 23 | 11-24 | Shared project populations; closed capture by subject | cross-file; memory | `E/finish/population.rs`, `check_session/frontier.rs`, `finish/partition.rs`, `subject_scope.rs`, `capture.rs`, `P/readers.rs` | 132.153→120.127, peak 9,487.7→1,874.2 MiB; then project 29.306→3.565 | landed | S:5809; S:6337 |
| 24 | 11-23..24 | One provider call per file | fact filling | `py/src`, `E/finish/request.rs`; `fill_len.rs`, `lazy_view.rs` deleted | first try slower (dj 10.62→11.12); later dg 13.183→12.333, dj -8.93% | lost, then landed | S:3766; S:6010 |
| 25 | 11-23 | Executor bound from the shared typed plan; JSON plan reader removed; provider columns borrowed | plan shape; start-up | `E/finish/plan_load.rs`, `finish.rs`, `col/mod.rs`, `plan_select.rs` (29 files) | start-up dj 166.1→77.6 ms, hono 37.3→12.5; dg 10.03→9.65, fa 0.97→0.86 | landed | S:4691; T rounds 59-60 |
| 26 | 11-23 | Exact keyed probes in the first half; frozen emptiness bit; entry tests cached per file | screening | `E/finish/known.rs`, `emptyprop.rs`; probes not pushed | probes dg +4.0%, dj +1.3%; bit lost 1,193,380 skips; cache dj 6.46→6.46 | lost | S:5075; S:4154; S:4358 |
| 27 | 11-23 | Rules as programs anchored at a node | plan shape | branch `claude/redo-anchor` 1ebe7729c | covered rules dj 0.665→28.85 | lost | S:4529 |
| 28 | 11-23 | Pair decisions for TypeScript and Rust | screening | `E/reader_packet.rs`, `reader_run.rs`, `reader_pairs.rs`, `pair_summary.rs`, `pair_cost.rs` | pairs hono 19,716→4,196, vite 73,005→10,443; 1.176→1.021, 2.076→1.751 | landed | S:4440 |
| 29 | 11-23 | First-half tests ordered by measured cost | screening | `E/pair_summary.rs`, `pair_cost.rs` | first half dj 3.427→2.526; whole 9.93→8.91 | landed | S:4272 |
| 30 | 11-23 | Equijoin bound at plan load; joins read base views | operator kernels | `E/finish/view_join/*`, `select_kernel.rs` | fixed cost -8 to -18%; second half -2.2 to -5.0% | landed, small | S:4067; S:4204 |
| 31 | 11-23 | Binding rules and import sorting read provider facts | fact filling; plan shape (rule text) | py `fixtures/columns/resolution.rs`, `columns/imports.rs`, bench `fact_catalog.rs`, `provider_packaging.rs`, rule pack | nine rules dj 1.818→0.866 s; I001 dg 2.796→0.887 s | landed | S:4048; S:4180 |
| 32 | 11-23 | Row-id lists: filters compiled at plan load, lists borrow the provider's index | select access paths | `E/finish/cold_word.rs`, `cold_pred.rs`, `cold_test.rs`, `cold_text.rs` | cold-list dj 2.144→0.961→0.137 s; rows entered 369.5 M→0.25 M | landed | S:4018; S:4108 |
| 33 | 11-23 | First half: child ids placed by parent, model and fact columns deferred | language parsing; fact filling | language `node_table/service.rs`, py `ruff_python_parser/walker.rs` | first half dj 1.826→0.703, dg 3.878→1.560 | landed | S:3972; fd83be7ee |
| 34 | 11-23 | Typed projection kernel; fused tuple-at-a-time pass | operator kernels | branches `claude/redo-binding` c2d952219, `claude/redo-alloc` 14ccb1b23 | row 2.00x cheaper but second half -1.9 to +2.5%; fused dj 7.97→8.08 | lost | S:3930; S:3703 |
| 35 | 11-23 | Step summaries decide pairs; first-half spec from summaries | screening; plan lowering | `E/pair_summary.rs`, `pair_checks.rs`, `first.rs`, `lower.rs` | pairs asked dj 175,831→111,944; prep 1.0-1.4→0.045 s; dj 11.60→9.10 (rounds 44→49 quiet) | landed | S:3893; S:3785; T |
| 36 | 11-23 | Keyed path deleted: every step made whole | select access paths | six files deleted under `E/finish` (`memo.rs`, `drive.rs`, `select_bound.rs` ...) | dj 13.02→11.98, dg 28.45→25.59; rows +42 to +59% | landed | S:3574 |
| 37 | 11-22 | Allocator swap; pools for columns, tables, per-file buffers | allocation; table layout | `E/finish.rs`, cargo features | jemalloc -14 to -17%; allocation counts -67 to -73% | landed | S:3365 |
| 38 | 11-22 | Empty-run propagation | screening (per step) | `E/finish/known.rs`, `emptyprop.rs` | step runs -29%; dj 16.28→14.34 | landed | S:3350 |
| 39 | 11-22 | Native reads as columns; text cells to symbol ids through a word cache | fact filling; interning | `py/src`, `E/finish/request.rs` | dj 16.29→13.93; native_get 229.2 M→12.5 M | landed | S:3329 |
| 40 | 11-22 | Rust lowering replaces the Python chain | plan lowering; start-up | `E/lower.rs` | 8.0-8.8→0.093 | landed | S:3167 |
| 41 | 11-22 | Step fixed cost: known-empty before the bracket, per-node data flat at plan load; per-key memo rebuilt | driver; plan shape; select | `E/finish.rs`, `finish/known.rs`, `finish/frame.rs`, `finish/memo.rs` | dj 26.49→18.14; memo 10.06→1.84 s | landed | S:2990; S:3179 |
| 42 | 11-22 | Union and antijoin kernels; the step clock stops calling the system | operator kernels; measurement tooling | `E/finish.rs` | dj 32.68→24.82; the clock was 2.7 of 2.8 s system time | landed | S:2903 |
| 43 | 11-22 | Two select kernels | select | `E/finish/select_kernel.rs` | select dj 17.30→16.73 s | landed, little | S:2960 |
| 44 | 11-22 | Facts as columns; name resolution as a provider fact; one parse per file | fact filling; language parsing | py provider, bench `bench.rs` | fact layer 0.763→0.293; dj 32.46→28.08; one pass 0.649→0.647 | landed | S:1994; S:2880; S:3006 |
| 45 | 11-22 | 21 rules rewritten as per-candidate checks | plan shape (rule text) | rule pack only | 19.93→15.00; peak 1,118→946 MiB | landed (PR 64) | `RESULTS-STAGE-1.md` §6 |
| 46 | 11-21..22 | New engine in crates/engine: table form, needs, skip terms, rule preparation | plan shape; screening; start-up | `crates/ryo/crates/engine/src` | dj 36.0→19.9; call form 459.6 CPU-s and 8 GiB; prep 2.29→1.59 | fallback; deleted build 134 | `HANDOFF.md`; S:2519 |
| 47 | 11-19..20 | Old engine stages 1-9: lazy interpreter, guards, typed readers, kill order, in-place reads, release after last reader, sibling filters, provider name resolution, dense keys, host | screening; kernels; memory; fact filling | ops alone for six levers; py + relations for resolution; core + execution_runtime for host | 228.6→32.6; 1,639→940 MiB | not merged | S:23-38 |
| 48 | 11-07..17 | Consolidated and fused engine: compiled program cached; facts path one copy; presence gate restored; per-file release; ISC004 made linear | start-up; table storage; screening; memory | core, execution crates, ops, py pack | compile was 62% of a one-file run; kept copies 522 + 438 MB; fused 2,950 CPU-s / 5.6 GB vs main 625 / 1.9 | landed in that line | beads ryo-7hjh, ryo-2tpw, ryo-2964.32.x; memory `perf-priorities-and-targets.md` |

## 2. Clusters
Gains are quoted as the largest measured steps; the record itself says deltas from different bases cannot be added. "Commits" = trunk commits touching the file.
| Concept | Rows | Largest measured steps | Files that kept changing |
|---|---|---|---|
| Screening | 11, 19, 26, 28, 29, 35, 38, 46, 47 (9) | pairs asked -36% to -86%; tm 22.34→17.04 and 36.97→30.04; prize left: second half dj 4.512→0.746 if only productive pairs run (S:4721) | `E/check_session/pairs.rs` (54 commits), `pair_checks.rs` (37), `pair_summary.rs` (22), `finish/known.rs` (18) |
| Fact filling and hand-off | 3, 5, 12, 24, 31, 33, 39, 44 (8) | fact layer 0.763→0.293; first half 1.826→0.703; rule families -52 to -68% | `languages/python` (9,807 file touches since 11-17, the most of any crate), `crates/language` (1,288), `E/reader_packet.rs` (49) |
| Per-file driver | 6, 9, 15, 20, 41 (5) | step fixed cost dj 26.49→18.14; then +38% at the consolidation | `E/check_session.rs` (145 commits, the most-changed file), `finish.rs` (79), `finish/session.rs` (41) |
| Plan lowering and shape | 1, 8, 16, 20, 25, 27, 40, 45 (8) | 8.0-8.8→0.093 s; start-up 166→78 ms; rule rewrites 19.93→15.00 | `E/lower.rs` (14), `finish/plan_load.rs` (13), engine `plan_lowering` |
| Select and access paths | 16, 30, 32, 36, 41, 43 (6) | row-id lists 2.144→0.137 s; the rest small | `E/finish/select_kernel.rs` (25), `cold_*.rs`, `view_join/*` |
| Admission and start-up | 6, 8, 10, 17, 18, 25 (6) | start-up lost 73.6→295.5 ms; double admission is 16.6% of call wall (bead ryo-fmx3.1.13.19) | `E/check_session/batch.rs` (40), `sdk.rs` (21) |
| Other operator kernels | 30, 34, 42 (3) | union 5.19→0.70 s, antijoin 2.13→0.80 s; two of three lost or small | `E/finish/partition.rs` (22), `union_kernel.rs`, `antijoin_kernel.rs` |
| Cross-file project phase | 4, 7, 23 (3) | peak 9,487.7→1,874.2 MiB; project 29.3→3.6; dg 1,245→477 MiB | `E/check_session/frontier.rs` (27), `finish/population.rs` (18), `project.rs` (18) |
| Kept store and warm | 2, 10, 14 (3) | unchanged rerun dj 62→32 ms; one edit 12.6x slower; store 290 MB | `E/check_session/stored.rs` (29) |
| Scalar functions and interning | 21, 22, 39 (3) | -21.05 and -2.338 CPU-s on tm | `E/finish/symbol_predicate.rs` (10), `pair_patterns.rs`, `pair_regexes.rs` |
| Memory and allocation; workers; answers | 9, 13, 37; 13; 18 | allocator -14 to -17%; ten-core CPU 3.619→1.380 (parked); answers 329→58 MB | `E/finish.rs`, `check_session/worker.rs` (27), `finish/answer.rs` (25) |

- **Most effort:** screening, the per-file driver and fact filling. File touches in the prototype engine across all branches since 11-21: driver 860, screening 828, select 533, fact filling 518 (my grouping by file name; rebased commits count twice).
- **Most gain:** screening (pairs never asked), fact filling (rule text reads a fact the language already holds), and the fixed cost of a step run (row 41, with allocation row 37). For memory the gain came from the cross-file phase.
- **Effort without gain:** select bodies and the other kernels; five of rows 15, 30, 34, 36, 43 lost or were small.

## 3. Coupling
Changes that had to touch two or more concepts:
- **Fill on first use (row 12):** language contract `named.rs`, Python `numbered.rs` and `columns.rs`, `E/reader_packet.rs`, `E/check_session.rs`, `pairs.rs`. That is language, driver and screening in one commit.
- **Provider facts for rules (row 31):** Python provider, the bench's `fact_catalog.rs`, the relation catalog and the rule pack text. It was undone in part by row 3.
- **Join pushdown (row 16) and shared plan (row 20):** engine `plan_lowering`, plan_codec, `E/finish/select_kernel.rs`, `E/check_session.rs`. Lowering, kernel and driver.
- **Typed plan binding (row 25, 29 files):** `plan_load.rs`, `finish.rs`, `col/mod.rs`, `plan_select.rs`, `cold_test.rs`, the request edge.
- **Glob and symbol predicates (rows 21, 22):** scalar functions, interning and screening in the same files.
- **Cross-file (rows 7, 23):** `population.rs`, `frontier.rs`, `partition.rs`, `check_session.rs`, `P/readers.rs`, `P/batch.rs`. Project phase, driver, admission and the Tastemaker edge.
- **Visit clean-up (row 9) and request hashing (row 10):** driver, memory budget, store, admission and the SDK edge.
- **Workers (row 13):** the pool needed the plan, constants, patterns and symbol tables frozen; the port failed on exactly that.
- **Old engine:** the host lever touched python (43 file touches), core (14), relations (10), execution_runtime (7), operators (5).

Changed alone:
- Old-engine kernels: branches ss2-rows, ss2-base, ss2-shapes, ss3-fanout, ss3-dense touched the operators crate only.
- Row-id lists: `E/finish/cold_*.rs` (row 32).
- Empty propagation: `E/finish/known.rs` (row 38).
- Allocator and pools: `E/finish.rs` (row 37).
- Pair summaries: `E/pair_summary.rs` and `pair_checks.rs` (row 35).
- Rule rewrites: the pack only (row 45).

## 4. Rows: every time the record says too many were built
| When | What was seen | Cause found | What fixed it, or did not | Source |
|---|---|---|---|---|
| 11-08 | Project-phase frontier of about 56 M rows (1.95 GB words + 1.34 GB places); 4.44 GB file-phase output; kept copies 522 MB + 438 MB | the facts path held owned copies; the frontier's cause: not found (diagnosis file not read) | borrowed slices | beads ryo-57zl, ryo-2tpw |
| 11-15 | The fused engine held every node's whole table; a tree join was fed 5,720 rows (bound under 50) | the design promised only shared or blocking nodes hold rows; "no test or review checked it" | streaming for single-consumer nodes | beads ryo-2964.9, ryo-2964.13 |
| 11-17 | ISC004 probe 1.50 M→8.73e9 rows, 522 of 1,278 node-s; fused 2,950 CPU-s / 5.6 GB | a near-cartesian join in rule text; the per-file presence gate was lost | query restated; gate restored | bead ryo-2964.32.10; memory `perf-priorities-and-targets.md` |
| 11-17..19 | 1,362,890,477 row writes; 112.10 M rows with no consumer; 49.85 M of 463.95 M right-side rows ever matched; one closure made 36.47 M rows for 876 positions | rules compile to whole-file plans; B015 is 7 steps alone and 299 with suppression | partly (demand propagation); not solved | `/workspaces/ryo-poc-evidence/subsecond/experiment-inventory.md` |
| 11-19 | 94% of operator time and 529 M of 566 M rows fell in files where the rule found nothing; 235 M rows were copies of base rows | tables are built whole and the deciding join is the last one | lazy evaluation 228.6→115.6; in-place reads 566→441 M; kill order only -8 s; key-restricted joins lost | S:42-56; S:33 |
| 11-21 | Call form 459.6 CPU-s and 8 GiB; I001 filled 1,987.6 M rows for 16 findings; PLR0124 took 4 of 6.1 GiB | planner started from every named node (296,265; 42.6 M pairs) instead of 7,163 comparisons | table form as default; planner fix | `HANDOFF.md`; S:1448 |
| 11-22 | 524.7 M rows: 46.8% re-derive facts the provider holds, 84.9 M are "every node that is not X", 63.6 M text scans | rule text restates the language's model; the second half "never reads the survivors" | rule rewrites (SIM300 39.1→0.31 M); on-demand evaluation lost (-85 M rows, +78 M lookups) | S:2110; `RESULTS-STAGE-1.md` §3, §6 |
| 11-22..23 | Rows -16 to -20% with CPU flat; keyed path deleted, rows +42 to +59% with CPU down | "rows nobody reads are a SYMPTOM": a row is 7-50 ns, a step run about 1 µs | dj rows 142.6 M (round 1) → 55.8 M (round 60), by provider facts and smaller drivers | S:2942; S:3574; M; T |
| 11-23..24 | One request per file answered 143,651 relations, 76,740 read by nobody; source-location filled for every file (+915 M instructions) when 387 of 7,379 pairs certify | filling ahead of need | narrower request landed later; certificate fill rejected in KR | S:3766; KR row 6 |
| 11-24 | Project phase carried 72.7 M rows and gathered 73.2 M; peak 9,487.7 MiB; one edge carries 12 M rows to an axis that reads 427 | every reader gathered whole populations | keyed reads (14.55 M→9 for one rule); peak→1,874.2 MiB | S:5809; S:8243 |
| 11-24..25 | Peak at 1x/2x/4x workspace: 1,659 / 2,105 / 2,966 MiB; dg heap 253→2,217 MiB, 1,600 MiB of it contributions; nine source-location copies = 950.3 MB | kept past the last reader ("keep=true skips that path"); 8 rules sent cross-file | file-local proof and release: 1,245→477 MiB; dj still 435 vs 272 | S:7489; S:7946; S:9297 |
| 11-26 | Kept cache dj 690 KB→290 MB, dg 1.7→518 MB; sources 6,205→26,183 | the whole session is published; an environment per file version | second fixed; first open | beads ryo-fmx3.1.11.14, ryo-fmx3.1.10.24 |

Three causes recur: whole-file tables built for pairs that end empty; rule text rebuilding what the language already knows; data kept past its last reader.

## 5. Dead ends
| Design | Result | Why it lost | Source |
|---|---|---|---|
| DuckDB, Souffle, sorted relations as the engine | 594 CPU-s; 342 CPU-s for 36 rules; 59 CPU-s | set-at-a-time computes every row for every file | M |
| Keyed, demand-driven or memoised evaluation (three times) | 66.2→67.4; +536.7 M lookups; deleting it made CPU fall | lookups and copies cost what the rows save | S:33; M; row 36 |
| A program per candidate or per rule (blocks; anchored) | 1,380 ns a candidate; 17 to 43x worse | each rule walks its own population | M; row 27 |
| Cheaper unit of the same work (fused pass, typed projection, shared leaf scan) | no total change | leaves are 5-10%; rows are not the cost | M; row 34 |
| Batching across files (bake-off twice, then 11-24) | +0.894 CPU-s | no lever at any batch size | M; row 15 |
| Frozen or cached emptiness; exact first-half probes | lost 1.19 M live skips; +4.0% | most skips use what a producer learned at run time | row 26 |
| Sharing more across programs ("a parameter is a column", merged plan, closed-step memo) | ceiling 0.002 CPU-s; ratio 1.183; 0 hits of 36,826 | the plan already shares equal steps | S:6381; S:9277; KR list 4 #1 |
| Learned form flip; walk form on the word interpreter | 18-22% worse; 21.0 vs 20.3 | a loop step cost 130 ns against 7 ns a row | S:1498; S:1845 |
| Parked parse cache and retention clock; base driver chosen by generality | removal saved CPU and peak; Python +38% | a general base keeps its overheads | rows 9, 6 |

## 6. What the history implies for the proposed tree
- **Operator kernels:** yes, inside `domain/operators`; they were already changed alone (old-engine kernel branches, rows 32, 37, 42).
- **Select access paths:** only partly. The kernel is in operators, but every win bound work at plan load (`domain/plan`) or borrowed a language index (`outbound/language`).
- **Plan lowering and shape:** yes, in `domain/plan` + `ryoql`, provided the executor keeps no second plan form (`plan_load.rs`, plan_codec, per-program plans were the repeat offenders).
- **Fact filling:** `languages/*` + `outbound/language` for how a fact is written. When to fill (first use or up front) lived in the driver, and each large win also changed the relation catalog and rule text.
- **Per-file driver:** `application/execute`, but `check_session.rs` took 145 commits because screening, admission, cross-file, memory and answers all live in it.
- **Kept store and warm:** `outbound/store` + `cache`, if what is kept (findings only) is decided there; rows 2 and 10 show the executor deciding it.
- **Worker pool:** `outbound/scheduler` holds the division of files, but the measured gain was shared frozen plan state, which belongs to plan and execute.
- **No crate of their own, tuned often:**
  - Screening: 9 rows, top gain; spans plan, language and execute.
  - Admission and start-up: 6 rows; split over sdk, lint, execute, plan.
  - Cross-file project phase: 3 rows; all of the memory gain.
  - Text interning and symbols, scalar functions (regex, glob): the largest single Tastemaker cut.
  - Memory budget and allocation.
  - Answers and report.
  - Measurement tooling: the step clock alone was 2.7 s.

Not covered: the 196 MB Codex session log and its dashboard; `subsecond/` beyond the inventory; `build/codex/*/INBOX.md` (read only through their summaries in STATE.md); about 110 of STATE.md's 922 entries were read; `GOAL-PATH-APPROACH.md`, `REVIEW-2026-11-23.md` and the stage 2 `REPORT.md` were not read in full.

### Critical Files for Implementation
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/executor/src/check_session.rs
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/executor/src/finish.rs
- /workspaces/idiomatic-public-benchmark/crates/ryo/crates/executor/src/check_session/pairs.rs
- /workspaces/ryo-poc-evidence/next/build/ports/KEEP-REJECT.md
- /workspaces/ryo-poc-evidence/next/build/codex/MECHANICS.md
