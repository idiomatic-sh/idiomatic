# File-phase memory on a cold `tm check` of crates/ryo

Bead ryo-n6vk, lever l3 of [project-retention](../project-retention/diagnosis.md). This is a measurement-only diagnosis; no engine code is changed. [diagnosis.json](diagnosis.json) holds every number and frame.

- **Measured source:** dcea2ee7df.
- **Worker:** tm 0.1.2 with jemalloc's profiler, sha256 `d9e72218…`.
- **Workload:** the guard's recipe, `git archive` of the tree with `tm check` run cold in crates/ryo.
- **Timing:** no row is a timing row, because the load rose above 3 during every run.
- **Reading the profiles:** each dump is read by stack. A stack is symbolized to file:line, then assigned to the concept that owns its allocation.

## 1. What holds the heap when the file phase ends

At this tree the file phase ends at 2288 MB of heap (dump 471). Project-retention measured 2513 MB on 8a59189be5, which is a different tree.

| Concept | MB | Owner and frame |
| --- | --- | --- |
| Worker tables | 981 | Element table 617, facts columns 172, names 129, file bytes 63. These are the 9 workers' own `Tables` and `FactsWorker` columns, reused file after file. Each column is exactly 9 allocations, about 7 MB each (`language/domain/builders/service.rs:276-283`). |
| Answer rows | 346 | 344,403 rows with 4,557,530 owned cells, in the collect row sink (`row_sink/adapters/collect/adapter.rs:37`). |
| Store copy of the same rows | 252 | `Ran.rows` of every `FreshRecord` (`keeping/service.rs:39-40`), kept for `finish_run`. |
| Handed metadata | 161 | Per-bucket proofs and handles 107, arena text 40, notes 14 (`recording/service.rs:103/214/228`). The words themselves are already staged to the store's spool. |
| Spill index and pending bytes | 164 | One `SpillStretch` per (bucket, file) keep, each with a cloned 48-byte `FrontierProof` (`spill/adapter.rs:196`). |
| Other engine | 252 | Workers' name arenas 38, glob hits 41, the program. |
| Tastemaker | 122 | tm's plan, outside the lint call. |

When the workers are dropped the heap falls to 1422 MB (dump 474), and the worker tables go to 0. Everything else stays live through the project peak (dump 501, 4258 MB).

## 2. Needed by the project phase, or kept by accident

**Needed by the project phase**

- The spill index. Its size is an accident: the same proof is copied into roughly 1.5-2 million stretches.
- The name arenas.
- The program.

**Needed only after the run.** These are not needed while the project phase runs:

- The answer rows (346 MB), which go to tm.
- The store records (413 MB), which `finish_run` writes.

They are live through the project peak only because the engine keeps them in memory until the run ends.

**Kept by accident**

- **Worker tables.** Every worker keeps the capacity of the largest file it has read until the file phase ends. This sets the file-phase peak.
  - Removing the 27 files over 1 MB (mostly benchmark JSON) lowers the file-phase end from 2288 to 1516 MB. The worker tables fall from 981 to 217 MB, and the project phase is unchanged.
  - Placing a single Rust-only rule still parses those JSON files: the worker tables reach 1059 MB. The tree is built with the run-wide `wanted` set, not with the set the file's governing rules read (h4, supported but not proven).
- **The second copy of the rows.**
- **Handed metadata held after its words were staged.**
- **The per-stretch proof copy.**

## 3. Floor or fixable

Mostly fixable in place. Only the per-(rule, file) frontier metadata is the fact model's per-tree floor, and only in its count: spill index plus handed buckets, 252 MB. It is the counterpart of the Django study's ~378 MB of facts, and its bytes per entry can shrink too.

| Lever | Change | File phase | Project phase | Size |
| --- | --- | --- | --- | --- |
| l3a | Shrink the worker tables to a cap after a large file, and read files over a size bound one at a time. Alternative: skip building the tree when no governing rule reads the language; this needs a check that unread/partial reporting stays byte-identical. | -600 to -770 MB | 0 | 80-200 lines (h4 variant 150-300) |
| l3b | Stage `Ran.rows` to the store spool, as `stage_outcome` already does for the words. | -252 MB | -253 MB | 150-300 lines |
| l3c | Stage the handed proofs, text and notes the same way. | -161 MB | -161 MB | 100-200 lines |
| l3d | Keep the proof once per bucket in the spill index, so a stretch is about 24 B instead of 72 B. | -90 to -100 MB | -90 to -100 MB | 50-120 lines |
| l3e | Hold the answer rows as one text buffer plus ranges (as `FindingRows` does). | about -100 MB, uncertain | about -100 MB, uncertain | 80-150 lines |

## 4. Is the 1.33 GB guard reachable with l1 + l3?

**No.** The guard is about 1235 MB of heap, using this run's RSS/heap ratio of 1.077.

The largest single rule sets the bound under l1. With only program-implementation-boundary placed, the project phase holds 638 MB of capture rows plus 47 MB of other project memory, 685 MB in total. Shared-walk-owner alone also reaches 638 MB. Derived-view-acquisition alone reaches 9 MB.

| Fixes applied | Residual + largest rule (heap) | RSS |
| --- | --- | --- |
| l1 | 1422 + 685 = 2107 MB | about 2.27 GB |
| l1, l3b-l3d | 908 + 685 = 1593 MB | about 1.72 GB |
| l1, all l3 | 808 + 685 = 1493 MB | about 1.61 GB |

**The file phase can get under the guard.** After l3a-l3d it is about 0.9 GB of heap, or about 1.0 GB RSS. This is estimated from the ablation, not run.

**The project phase cannot.** Its floor under the current model is about 1.35 GB of heap, or about 1.45 GB RSS:

- the residual it cannot drop, about 650-700 MB: tm's plan, compact answer rows, the name arenas, a compact spill index, the decoded frontier and the program;
- plus the largest rule's materialized capture, 685 MB.

To get under 1.33 GB, the path-split captures must stream (project-retention l2), or the guard must change.

**Not measured**

- Which rules govern each large file.
- l3a when large files arrive together.
- Whether l1 can serve program-implementation-boundary and shared-walk-owner from one shared capture. If it cannot, two 638 MB captures are live at once.
