# Plan: build-133-designed-crates.md (mapping of the designed crates at O=f0219b3b3 onto the agreed tree)

Status: research done in read-only mode (git cat-file over O, scratch files only). The one output file has NOT been written, because plan mode began before the write.

## Output file (the only file to write after approval)
/workspaces/ryo-poc-evidence/next/build/ports/stage2/mapping/build-133-designed-crates.md

## Sections, in the brief's order
1. Table
   - 1.1 Contracts: one row per `pub trait` (80 traits found; engine 11, operators 15, execution_runtime 15, execution 9, core 16, derived 6, execution_domain 4; counts match the owner's). Columns: name, path:line at O, methods, types named in signatures, implementors, files naming it by crate, destination, what must change.
   - 1.2 Proof assets: test counts per crate (engine 149, operators 173, execution_runtime 46, execution 225, core 143, execution_domain 74, derived 33), 132 case-table files in operators (`CaseName(` rows; AWB says 141, not reconciled), 7 compile_fail in operators, 33 .feature scenario files (7 core, 3 derived, 3 engine, 6 execution, 2 operators), execution/tests boundary_contract (171 files, 17,718 lines), 4 refusal_order fixtures, core/tests/fixtures/operators (30 dirs of input.ryoql + expected.rs, language-free: they run over the fake language's `function` relation), allocation tests (operators/tests, core/tests, execution_runtime/tests/allocation, execution/tests), benches. Language-free check: no language word in operators case tables (the 27 "clippy" hits are `clippy::` lint attributes).
   - 1.3 Module groups: one row per domain directory with non-test lines (counts taken: derived 2,917; execution_domain 13,592; operators 25,763; execution_runtime 14,431; execution 3,472; engine 48,614; core 22,234), destination, trunk counterpart or "not checked".
2. Totals per destination.
3. NO HOME (e.g. PlanCodecPort, memory port and budget, observer, found_rows two-engine comparison, spike instrumentation, EngineMode).
4. No caller found (trait with no implementor outside tests; method: git grep -w at O; report only).
5. Contracts: pointer to 1.1 plus summary of which traits name row/column shapes.
6. Import-direction conflicts.
7. Capabilities with no trunk counterpart.

## Findings to carry (for the final message)
- At O `PlanInputs::facts` already returned `FactRows::Lent(ColumnsView)` (relations' column type), while held rows were the row-major `values::rows::Table { kinds, cells: Vec<Value>, count }` and `FileBuffers.nodes: Vec<Option<Table>>`. Row-shaped signatures: `PlanInputs::{frontier,observation,hosted} -> HeldRows` (Table), `Resources::intern -> Value`, derived `RowCells`/`BulkRows`, `RowSinkPort::row(LentCell)`, `FoundRows::found(FoundCells)`, execution_domain `RepresentationsItem` (RowRead/CellRead/Value). Column-shaped: `FactsWorker`/`FactSource` (ColumnsView, ColumnsMut). Types from deleted crates: none of these traits names a crate absent from O; at T the operators/derived/execution_runtime crates they name are gone.
- engine `plan_lowering` (2,944 lines at O) now lives at T as query_language/domain/plan_lowering (3,083): moved, not lost. kind_check moved to query_language (193 lines at T against 334 at O).
- Trunk counterpart of operators kernels, engine and execution_runtime run path is the executor (finish*, pair_*, reader_run, check_session); the documents give only a whole-engine speed figure (old product 35.6 to 36.4 s against 19.9 s on Django cold), so per-module speed is "not checked".
- Possible losses with no trunk counterpart: derived crate views as a contract, FactsPort spill, execution phase ports (D4), lint store (trunk has executor-private store), 162 boundary_contract files, operators 141 case tables, call-count and allocation rules.

## Open limits to state in the file
- Groups over 300 lines are split only where a directory boundary shows the concern (physical_plan: kernels 6,314, rewrites 1,404, service 4,253, types 3,571); file-level split of every large file is not done.
- Destination column is a proposal; confidence marked medium/low where only module doc lines were read.
