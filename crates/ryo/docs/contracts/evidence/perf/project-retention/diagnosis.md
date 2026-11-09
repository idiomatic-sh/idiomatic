# Project-phase retention on a cold `tm check` of crates/ryo

Bead ryo-n6vk (from ryo-2r77). This is a measurement-only diagnosis; no engine code is changed. [diagnosis.json](diagnosis.json) holds every number. The measured source is 8a59189be5, the head of claude/checkpoint-332. No row is a timing row: the load was over 3 throughout.

## What holds the memory

A cold `tm check` of a `git archive` of the tree, run in crates/ryo, peaks at 4.36-4.52 GB RSS. The jemalloc profile (`lg_prof_interval:29`) has its largest dump at 4.29 GB:

- **2.98 GB is the project phase** (`ryo_core` `cross_file/phase/service.rs:181`).
  - 2.17 GB of that is rows the **capture** operator emits (`operators/src/domain/capture/service.rs:121`). They are pushed into published region tables inside `prepare_roots` (`deliver/service.rs:67`), during the first delivery of the ordinary project batch.
  - 0.38 GB is the decoded frontier.
  - Nothing is held under root invocation (`deliver/service.rs:89`).
- **1.31 GB is left over from the file phase**, 0.73 GB of it under `visit_file`.

The heap is 2.51 GB when the file phase ends. It falls to 1.56 GB, then climbs in one ramp to 4.29 GB inside a single `prepare_roots` call, and falls back to 1.41 GB.

## 1. Which commit (established, and the range that remains)

No single commit is proven.

**The engine has not regressed.** On the same tree (7b104be366), the published ryo 0.1.1 engine peaks at 3.57 GB and the source engine at 3.12 GB.

**The workload grew.** These runs hold the engine fixed and vary only the tree:

| Tree | Date | Peak RSS |
| --- | --- | --- |
| 1cd5de7fea (the guard's 1.21 GB tree) | 2026-10-30 | 1.20 GB |
| first-parent 889/890 | 2026-11-05 | 2.15-2.34 GB |
| first-parent 960 and 1080 | 2026-11-05 | 3.01-3.12 GB |
| 0330d84902 (first-parent 1630) | 2026-11-09 | 4.36 GB |

Up to first-parent 1080 the peak is still the file phase. The project phase there reaches only 0.37-0.67 GB, with 51-101 MB of capture rows.

**Remaining range:** first-parent 1081-1629, which is 7b104be366..0330d84902. Every tree measured inside it ends with exit 2 under the current engine, because the tree's own definitions fail their proofs. Bisecting it would need a per-tree tm build. Two named candidates in the range add the large capture shape:

- 7540749ec5: `shared-walk-owner` adds `semantic-targets`.
- 3c6f3c3f04: `program-implementation-boundary` adds `local-dependencies`.

Together with `derived-view-acquisition`, these account for 734 MB of the 2.17 GB of capture rows. The other 1.44 GB is unattributed. Removing the nu-hex placements or the `idiomatic.ryo/language` placements each lowers the peak by 0.7-1.1 GB.

**Why the guard did not catch it.** From d22aceaaf9 to 10c561ef6a, tm linked the published ryo 0.1.1 crates. That engine refuses the current tree (exit 2 at 0.44 GB), and the guard accepts exit codes 0 through 2. 10c561ef6a ("tastemaker resolves the local ryo crates") made tm check the tree with the local engine, and the 4.4 GB became visible. 96b5545cb8 on claude/w14-tm-import-reading makes the guard count only runs that checked the tree. Unproven: I did not run each guard tree in that window.

## 2. Which rows, which rules, why they are kept

**Which rows.** The rows are path-split captures over project relations. A slot that crosses `/`, such as `{*root}/{*tail}`, emits one row for every way a path can split. For example, for `program-implementation-boundary`:

| Step | Rows |
| --- | --- |
| `source-dependency` | 118,446 |
| `paths` | 727,134 |
| `local-dependencies` | 3,480,084 |

**Which rules.** The same shape appears in `shared-walk-owner` (`semantic-targets`), `derived-view-acquisition`, `layer-matrix` (nu-hex) and others.

**Why they are not streamed** (confirmed in the code). The capture ends its region and its whole output is published as a table whenever:

- it has more than one reader (`Barrier::SharedConsumer`), or
- its reader keeps columns, as in `select a, b from ...` (`Ineligible::Reshapes`).

The rules for both are in `execution_domain/.../fusion/service.rs`.

**Why they are not released** (supported, not proven). Since e5cbe5f397, `evaluate_roots` prepares every root of a cohort in the first delivery. Each rule's published inputs then wait for that rule's own invocation, so several rules' inputs are alive at the same time. The evidence:

- The heap grows in one monotonic 2.7 GB ramp inside a single `prepare_roots` call.
- Removing three rules that are not the largest contributor lowers the capture rows by 734 MB. If the spike were only the largest single rule, removing them would change nothing.

No per-rule counter of live publications was taken.

## 3. The fix

Both levers change scheduling only. Neither adds a second data structure for any concept, and findings stay byte-identical.

- **l1 (rank 1): prepare per rule inside a cohort.** Every root is still declared to the shared walk. A rule's non-walk inputs are evaluated in that rule's own delivery and released before the next rule's inputs are built.
  - Where: `facade_runtime/project_execution.rs` (`evaluate_roots`) and `scheduling/service.rs` (`prepare_roots`).
  - Size: about 150-300 lines plus project-phase tests.
- **l2 (rank 2): fan-out and reshaping continuations.** A row-local pure reader continues on the capture's rows even when there are several readers or the reader keeps columns, so the capture's output is never published whole.
  - Where: fusion classification, the region compiler and the kernel continuation.
  - Size: about 600-1000 lines.
- **l3 (rank 3): the file phase.** The file phase holds 2.51 GB of heap before the project phase starts (`visit_file` 1.94 GB, `parse_file` 1.08 GB). It needs its own diagnosis.

## 4. Expected peak

After l1 or l2: about 2.6-2.8 GB RSS. This is an estimate from the profile, not a measurement. The file phase's 2.51 GB of heap, times the head run's RSS/heap ratio of 1.054, becomes the maximum. That is still over the 1.33 GB guard, so getting under the guard also needs l3.
