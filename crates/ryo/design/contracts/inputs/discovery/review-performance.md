<!-- a read-only reader's final report, 27 to 28 Nov 2026; task a1035fcc03da998dc; copied from the session transcript -->

Sources: EV = `/workspaces/ryo-poc-evidence/next`; X = `crates/ryo/crates/executor/src` at `2a207464c`; INBOX = `EV/build/codex/typescript/INBOX.md`; PS = `EV/build/one-engine/PRICE-STATUS.md`; W121/W87 = `EV/build/tm-int/WINDOW-RESULT-BUILD121.md` / `WINDOW-RESULT.md`; KR = `EV/build/ports/KEEP-REJECT.md`; WP = `EV/build/one-engine/WORKERS-PRICE.md`; T1 = `EV/build/ports/stage2/trial-1/summary.txt`.

## Blocking
- **B1. C1, C2 and C7 never say who owns the layouts select reads in place.** Kernels read three layouts: computed `Table.cols` (`X/finish.rs:148`), lent provider columns translated per cell (`X/finish/fact_view.rs:21-43`), and the language's `NodeTable` (`X/finish.rs:920` `native_get`, `X/finish/cold_bucket.rs:16-19`, `walk.rs:45-83`). The plan's contract covers only the first. Select is 64.8% of operator CPU, and 212,469 of its 448,448 runs read no computed table (`EV/build/one-engine/STEP-COSTS.md`, FastAPI table and "Follow-up 26"). `domain/operators` may not import `outbound/language`, so either `NodeTable` and `FilledColumn` move into domain (then "store implements it" is false for the hot layouts) or select stays in `execute` (then "operators complete" is false).
- **B2. C2's contract contradicts what it restores.** The plan says `run(step, inputs, out)`, per step. Build 133 has `run(plan, node, buffers, &dyn PlanInputs, &mut dyn Resources)` (`operators/src/inbound/use_cases/run/interface.rs:32-47` at `f0219b3b3`), a pull over a whole plan; its 132 case-table files exist only at build 133. Today's kernels are `impl Exec` methods that pull children mid-kernel (`X/finish/select_kernel.rs:858`, `:556` `select_keyed`, `:472` `ksel_zero`; `X/finish.rs:2041-2075`). Pushing whole inputs loses lazy sources, keyed sub-selects (driving keys 5.58 M to 0.96 M, `EV/engine/STATE.md:4214`) and the early exit for the 263,799 selects that answer no row.
- **B3. The gate does not watch the bars that are met today.** Warm rerun is met on 9 of 9 benches with 0 to 10% headroom (Django 29 against 32 ms, dagster 48 against 50, fastapi 17 against 19; W121). Section 6 has no check for warm, one edit, linter store size, linter ten-worker wall or peak, or TypeScript and Rust start-up. "No slower than the build before" over about ten landings has no cumulative bar against 139.
- **B4. C3 puts a port around the store the bars require replacing.** `stored.rs` and `stored/*` are 3,038 lines and write 53 to 518 MB (W121). The store that fits `answers(key)`/`keep(key, answers)` is the findings-only one on `ba657e79b` (bead ryo-fmx3.1.11.14). Neither meets all three bars today: whole-session is warm 29 ms, edit 1,542 ms, 290 MB on Django; findings-only is warm 87 ms, edit 475 ms, 2.6 MB. `Key = [u8; 32]` has no stated owner. Warm validity today is a stat only (`X/check_session/replay/stored/request_answers.rs:295-301`); a content key means read and hash every file (67 of dagster's 120 ms, KR row 2, against a 50 ms bar).
- **B5. Section 6's bead table sends four bars to the wrong crate.**
  - Start-up: lowering 95-98 ms, RUF100 re-lowering 64, per-rule typing 103, partition 21, empty epoch 19, and 9 ms before `Linter::lint` (T1). That is `ryoql`, `plan` and `execute`, none of it `lint`, the SDK or the languages.
  - Peak: Python zero-file RSS is 150 against 68 MiB (W87 start-up row), and fastapi's whole peak gap is 89 MiB. That is load, not `store`.
  - Ten workers: every worker re-binds all programs (`X/check_session.rs:952-990`; prep 1.637 against 0.351 s; serial admission 0.41 of 1.31 s wall; division already 83% balanced; WP section 3). That is not `scheduler`.
  - One edit: the findings-only store's fastapi edit is 301 ms with a 296 ms start-up inside it. That is load, not `cache`.
- **B6. "Passes 1 and 2 should measure as zero change" is contradicted by the last landing.** Build 138 to 139 deleted only never-run code and moved all eight benches by -1.2% to -3.9% (`INBOX:5785-5798`). The plan has no rule for a rename that lands +2% with identical instruction counts. The only by-concept instrument, `EV/build/one-engine/scripts/price-root-split.py` (RULES, lines 38-52), matches `ryo_query_language`, `check_session::`, `::finish::`; the renames blind it, and it is not among the five scripts the plan updates.

## 1. Where time and memory go
Newest by-concept splits on record; none exists at build 139. fastapi = share of 18.16 B instructions, build 73 (PS "SAME PACK"). Check = share of 17.96 s engine CPU, ten workers, about build 138 (`INBOX:5353-5377`).

| Concept | fastapi | Tastemaker check | Other |
|---|---|---|---|
| Parse and walk (languages) | 11.0%; parse against fact production not measured | 22% (4.02 s; 28,087 parses) | |
| Filling fact tables | 13.0% (0.915 B of it source-location for certificates, KR row 6) | not isolated | Root 2.0% |
| Screening | 13.0% | 9% (skips 564,420 of 844,178 pairs) | switched off: +30% fastapi, +30% dagster, +14% check (`INBOX:5767-5777`) |
| Admission and start-up | 8.2% + set-up 0.6% | prepare 7%; SDK admission 1.59 of 9.62 s call wall | Python 296 ms split as in B5; TypeScript 27, Rust 21 ms: split not measured |
| Plan execution | kernels 10.1% + outside kernels 31.2% | 41% (7.38 s) | operator CPU (STEP-COSTS): select 64.8%, antijoin 13.0, union 7.9, base 4.0, axis 3.7, grouping 2.6, as-of/ordered 1.2, source-matches 0.6, capture 0.2, ordered-dfs 0.2, closure 0.1; worklist, text and regex inside select: not measured |
| Project phase | 3.6% | 17% (3.11 s) | |
| Decode and report | 1.8% | 3% (0.57 s) | 265 of 1,422 MiB heap on the check, build 16 (`EV/build/one-engine/PEAK-BY-ORDER.md`) |
| Teardown and allocation | teardown 5.1%; allocator about 6.6% (13.0 M mallocs) | not measured | Root teardown 7.3% |
| File and request identity | 0.2% | not measured | Root 4.8% (build 69); dagster warm 67 of 120 ms |
| Kept store | not on the cold path | `Cache::At`: +27% engine CPU, 291 MB | fastapi 53 MB = frontier blocks 48.9, pairs 3.5, project 0.3, answers 0.22 |
| Worker pool | ten workers: 3.62 CPU-s, 838 MiB against the prototype's 1.38, 387 | about 8x parallel | the scheduler's own cost: not measured |
| Peak memory | zero-file 150 against 68 MiB; whole 267.8 against 178.7 | 1,313 MiB | by-owner split at 139: not measured |

## 2. Cuts against the map (C1, C2, C3 are B1, B2, B4)
| Cut | Code today | Shape that costs nothing | Does the plan say it? |
|---|---|---|---|
| C4 `CorpusPort` | `std::fs::read` into `Arc<[u8]>` with device/inode and content dedupe (`X/check_session.rs:744`, `:782-789`); the check shares 38,645 versions (KR row 2) | bytes as `Arc<[u8]>`, one stat, identity (device, inode, mode, change time) in the answer | No. `CorpusEntry` has only path, size, modified, executable; the adapter's `read` adds two stamp calls and a linear scan of given files (`core/src/outbound/corpus/adapters/ignore/adapter.rs:269-311`) |
| C5 `SchedulerPort` | `X/check_session/jobs.rs` (23 lines), one job per shard | one port call per set of jobs | Yes in code (`d2502dca4`: two mutexes per shard, negligible). Ten-worker peak is unwatched: the pool budget is thread-local (`X/finish.rs:167`), set inside the job |
| C6 `Scan` per file | `visit` (`X/check_session.rs:864-1654`) does per-shard set-up, then loops files; the SDK reaches `batch.work`, `.schedule`, `.clocks` and 10 more members (`sdk/.../session/entry.rs`) | set-up in `Load`, per-worker scratch, lent facts, nothing crossing per pair (510,081 on fastapi) | No. "In front of `Batch`" reads as a facade; a per-file call onto today's `visit` repeats the 0.35 s set-up per file |
| C7 language contract | `walk(.., builder: &mut NodeTableBuilder)`, concrete, inside generic `NumberedWalk<W>` (`language/src/inbound/use_cases/walk/interface.rs:44`, `numbered.rs:46`); `enter_node` runs 2.41 M times on Root | stay concrete or generic per language; whole columns lent | No. A trait object here is a dynamic call per node |
| `lint` re-admission | SDK `admit` re-shapes each request (`entry.rs:477-720`), then `Batch::register_query` admits again | one admission, owned by load | No. Pass 2 moves the double admission across a crate boundary (bead ryo-fmx3.1.13.19) |

## 3. The gate
- **Unwatched:** warm, one edit, linter store size, linter ten-worker wall and peak, TypeScript and Rust start-up, the check's warm and one-edit. `gates.tsv` has ten-worker columns for `tm` only and one start-up figure. `landing-gate.sh` step 4 (`landing-pairs.sh`) prints `tm` pairs with no threshold.
- **Pair noise is not a bar.** The plan's reference pairs are 3 deep and called "indicative" by their author (`INBOX:5798`). Build 139's own row says `pairs_quiet` NO and carries pandas 10.47/10.93 (+4.4%). Loaded spread is 6 to 7% (`INBOX:5594`); the quiet paired range is 0.02 s on dagster.
- **Cheapest closure, both tools exist:**
  - `EV/build/tm-int/scripts/window-nine.sh TAG "old=DIR@REV,new=DIR@REV"` measures both image sets in one quiet window over every cell of the bar table. Three passes, `window-median.py`, fail any cell above +0.5%, old side always the kept landing-139 images. About 16 minutes per image set.
  - Instruction counts by concept (`price.py`, `price-root-split.py`) on fastapi, ripgrep and Root need no quiet window. Fail above +0.2%.

## 4. The owner's question
| Bar | Owning concept | Own crate in the plan? |
|---|---|---|
| Total CPU | evaluation around kernels, access paths, fills, screening | No: all inside `execute` |
| Ten-worker wall | load output shareable across workers | No |
| Peak | load, then answers and frames | No |
| Warm | file and request identity | No owner |
| One edit | load, then partial replay | No |
| Start-up | load | No |
| Store size | what is kept | Yes, if C3 is the swap |
| Tastemaker | evaluate, parse, project, schedule | No |

- **(a) Screening: pass 3, after C2.** 6,767 lines (`pair_checks`, `pair_summary`, `pair_skip`, `check_session/pairs`); 9 to 13% of CPU, saves 14 to 30%. It needs the one operator definition, since its `Builder::step` (`X/pair_summary.rs:1335-1622`) is the third. Boundary: one call per file returning the surviving pairs.
- **(b) Select access paths: pass 3, as the read side of C1, not as a crate apart from the layout.** About 5,500 lines. The per-cell loops (`Column::at` 28.5 M calls, `view_cols::cell` 13.1 M) must sit on the layout's side. Splitting select from the other operators: never; antijoin and band use the same views (`antijoin_kernel.rs:6`, `band.rs:7`).
- **(c) Functions and evaluator: pass 3 inside C2, as a second collection in `operators`; own crate never.** About 850 lines (`X/finish.rs:4063-4368`, `symbol_predicate`, `cold_word`), called per row (`eval`, `finish.rs:4106`). It serves the P0 I001 bead (0.455 against 0.22 s).
- **(d) Admission and load: pass 3, first after C1.** About 3,900 executor lines plus 900 in the SDK and 2,951 of lowering. It serves five bars. Output: one frozen `Send + Sync` program set (the executor has 426 `Rc` sites). Its bench needs only the pack: no files, no image build. Encoded pack: never for the start-up bar (owner: "we aren't shipping rule packs, w'er enot precompliing", `EV/HANDOFF.md:61-64`); later as a cache record for one edit.
- **(e) Project phase: pass 3 as C6's `Project` with an explicit hand-off type; own crate later.** 4,868 lines; 17% of the check's CPU and the store's 48.9 MB of blocks. A crate is possible only after the evaluator leaves `execute`.
- **(f) Answers and report: pass 3, decode moves into `lint`; never a crate.** About 750 lines, 2 to 3% of CPU. Hand over packed rows and a frozen dictionary.
- **(g) Symbols: decide inside C1; never a crate.** The per-worker dictionary (`X/finish.rs:301-370`) is why bound programs cannot be shared (WP section 4). Share as a concept: not measured.
- **(h) Fills: never a new crate.** The boundary exists (lent `FilledColumn`); C1 should adopt it as the one lent layout.
- **(i) Store split: never; replace it (B4).**
- **Not on the list:**
  - File and request identity: give it to `corpus`, with C4.
  - The schedule of which program meets which file (`X/check_session.rs:250-535`, `batch.rs:384-560`): a fourth use case in C6, and the check's scope lever.
  - A clock per contract call: the trial lost 12 of 62 minutes for lack of one and 23 waiting for the timing window; no obstacle was a crate boundary (`trial-1/obstacles.txt`).

## 5. Three changes
1. Run the two-hour table-contract spike the workshop agreed (`EV/engine/STATE.md:11520`, `11605`) on select over a native view, antijoin and one text function before C1; accept at +0.2% instructions or less.
2. Gate every landing on the full bar table against the kept build-139 images (section 3), and re-key `price-root-split.py` by crate in pass 1.
3. Re-scope pass 3: add `Load` as its own cut with a shareable output, make C3 the findings-only swap or drop it, and correct the section 6 bead table to the measured owners.

### Critical Files for Implementation
- `/workspaces/idiomatic-public-benchmark` at `2a207464c`: `crates/ryo/crates/executor/src/finish.rs`
- same revision: `crates/ryo/crates/executor/src/finish/select_kernel.rs`
- same revision: `crates/ryo/crates/executor/src/check_session.rs`
- same revision: `crates/ryo/crates/executor/src/check_session/replay/batch.rs`
- `/workspaces/ryo-poc-evidence/next/build/tm-int/scripts/window-nine.sh`
