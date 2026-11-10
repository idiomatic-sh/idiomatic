# Contract review: landed checkpoint 333

- Source: `40e49021520d4392d29df28ee5bd2fef739e5d37` (tree `b2e4a8c1f1`), base `dcea2ee7df` (landed 332); validated source `563c0bc69b`.
- Criteria: the claude/checkpoint-334 copy of `.agents/agents/contract-review.md` (four criteria).
- **Verdict: BLOCKING.** Enforced boundaries PASS, Clear ownership and flow PASS, Proof PASS, Change isolation FAIL ((a) and (b); (c) PASS).
- HEAD of the reviewed tree before and after: `40e4902152`, status unchanged (two untracked w16-prune branch evidence files, not this review's). Nothing was built and no test was run; tm and `run.py --check` ran on a git archive copy.

## Receipts

| Receipt | Command | Exit | Raw result |
|---|---|---|---|
| tm whole tree | `tm --root crates/ryo check --format json` (tm 0.1.2 release, sha256 `33bba487`) on an archive of `40e4902152`, fresh store | 0 | 0 findings, 19 suppressed (15 file-length, 3 no-tracker-text-shipped, 1 no-unwrap-or-on-lookup); 176.6 s, 4,425,044 KiB |
| tm scoped | `tm_source.json` of the final run | 0 | 0 findings, 5 suppressed |
| run.py | `python3 -I crates/ryo/docs/contracts/evidence/checkpoint/run.py --check` on the archive | 0 | 17 "newly scoped ... (0 findings)" lines |
| no_print | `no_print.json` / `no_print.log` | 0 | 0 lines starting `error` |
| cycles | Tarjan SCC over `ownership_edges.log` | n/a | 0 cycles, 4,971 occurrences, 766 files |

## Recounts (raw logs of the final run, all `<name>.json` at `563c0bc69b`)

libraries 1,142 / 1 (process_capture, owned); bdd 9 and core_bdd 18 scenarios; ruff 12 / 0; boundary 99 / 0; activation 11; harness hooks 135 OK; checkpoint_check 17 OK; python 140 / 0; language 43 / 0; replay 96 / 3; cli_cost 4 / 3; tm_import_cli 1 / 0; tm validate 241 / 224 hold; parity 412 identical of 413, 0 unaccepted. Every figure matches the record. From `563c0bc69b` to the source only docs, evidence and the `ci/contract_review` validator change (20 files).

## Change isolation

**(a) Seam tests: FAIL (C1).** Test files outside `execution_domain` naming it: 154 → 157; outside `execution_runtime` naming it: 124 → 126 (same recount). Added at 333:

| File | Names behind another seam |
|---|---|
| `execution/.../use_case_test/conformance_emptied/test.rs` (new) | domain `operator::contract::presence`, `::service::semantics`; runtime `acceptance::operator_contract_test::fixtures` |
| `execution/.../use_case_test/conformance/fixtures.rs` (new, split) | runtime `file_session`, `request_execution`, `rule_outcome`; domain `rule_intent`, `prepared_request` |
| `execution_runtime/.../acceptance/presence_service_test.rs` (new, 1,121 lines) | domain `executable_program::presence::service::of_graph`, `file_program::service::compile_prepared_observations`; store `fixtures::program_derived_port`, `outbound::derived::factory::semantics` |

Skip rules also rewrote execution-crate assertions on the evaluator's internals: `tests/shared_walk/order/join_dependency/test.rs` walk cohorts `counts(&file, 2, 8)` → `(1, 4)`, and the read order in `use_case_test/provider/test.rs` and `conformance_driver/test.rs`.

**(b) Footprint: FAIL (C2).** Per merge:

| Change | Files | Outside the concept's hex | Justified |
|---|---|---|---|
| skip rules `a73aa2bee8` | 70 | store `derived/owner/fixtures/mod.rs`, runtime `walk_coverage/fixtures/{catalog,operations/corpus}`, runtime `lifecycle/inputs/fixtures.rs`, execution `tests/union_rows/mod.rs`: each only passes the new `PatternMatcher` argument of domain `compile()` | No: forced by deep-path calls (C2) |
| skip rules | | execution tests asserting read order and cohort counts | No (C1) |
| skip rules | | `cli/tests/fixtures/unfed.toml` (3 lines) | Yes: the feed check's list |
| noqa rewrite `e8dade10ae` | 466 | none: languages/python rules and fragments, docs | Yes |
| provider alias `cbdc70adad` | 20 | none: languages/rust | Yes |
| isolation lens `226de88f02` | 1 | none | Yes |
| integration | | crate versions 0.1.2 → 0.1.3 in every Cargo.toml (leak 332 check 3); cli replay `base.rs`/`counting.rs` one `replay_languages()` (leak 332 check 2b); `ci/contract_review` validator and its case | Yes: release versioning and owned repairs in their homes |

**(c) No regression: PASS.** Census proxy rows (census.md lines 51 to 55), recounted from archives of `dcea2ee7df` and `40e4902152` with the census's production-file rule: binding_read 18 / 13, static_import 32 / 12, project_context 78 / 18 (32 benchmark), engine `Relation::X` 26 / 156 / 9, equal at both and equal under case and spelling variants. census.md's own 332 and 333 columns agree.

## Blockers

1. **C1**, files above: tests outside the owning crate name domain, runtime and store internals; the class from 332 grew by 3 files. Required: move each assertion into its owning crate or through execution's use cases or the port; the count returns to 154 or fewer.
2. **C2**, `execution_store/src/application/file_dag/derived/owner/fixtures/mod.rs:77` and four more: a domain signature change edited five files in three other crates. Required: one owned entry point for building compiled programs outside the domain, proved by a planted signature change that edits 0 files elsewhere; route the deep-path import class to the tm-specialist.

## Notes

- **N1** census.md: no census command exists (not in `run.py` at the source or in the 334 tree); the stated method does not reproduce the absolute figures (binding_read 18 / 25 / 27 files by variant against 23; `Relation::X` 26 / 156 against 25 / 155). Commit the command.
- **N2** census rows outside (c) rose 332 → 333: exhaustive `Operation` matches 24 → 27 (presence `narrowing.rs`, `operation_schema/projection.rs`, `operator/contract/service.rs`), GroupCount 43 → 45, store concrete types 52 → 53, evaluator rewrite total 179 → 181. The owner decision covers the 23 → 24 rise only.
- **P3** validation.json `scope` still reads checkpoint 330; `tm_whole_tree` binds `8568d6f1a2`; the only raw whole-tree result at `563c0bc69b` is the pre-scrub run (exit 1, 10 findings). This review's run gives 0 / 19 at the source.
- **P4** README.md:73 reads 8 check passes, the metrics 9; README.md:84 reads 6 red-then-green entries, validation.json has 7.
- **P5** the checkpoint table's Review link points at a 2026-11-07 verdict on `293ca1a9a2`.

Owner of C1, C2, N1 and P3 to P5: the checkpoint 334 integrator (C1 and C2 through the port landing's seam contract suite, as 332's C1 and C2 were assigned); N2: the PM.

## Manual judgment

Skip rules' production spread (domain presence fold, ten runtime services) has no strategy port; this review fails only its cross-crate ripples and leaves the spread to the port landing and the fused rewrite.
