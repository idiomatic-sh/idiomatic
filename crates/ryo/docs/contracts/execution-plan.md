# One executable program, mandatory engine ownership

Proposed replacement; no implementation approval or speedup claim. Source: `7a3b1132`; `0e27ff09` correctness review remains pending, and Feed hardening `fb42` awaits tests. Incorporate only accepted corrections. Continue with the [existing ownership matrix](README.md).

**Skinny inbound flow; application owns each operation and lifetime:**

```text
prepare(request)       → immutable program, bindings, origins, identities
open_run(program)     → inventory, project inputs, snapshot, budget
plan_reuse(run)        → fresh/replay decisions, lazy frontier handles
visit_files(run)       → worker buffers; generation-local facts/leases
finish_project(run)    → same executor; demanded partitions only
finish_run(run)        → ordered report, persistence, release
```

Findings/frontiers and required coverage/dependency/persistence metadata escape a file with explicit lifetimes. Domain owns primitive meaning; application owns demand, admission, publication, scratch and retirement. Outbound owns external facts/effects. No stages framework or unrelated CLI-cleanup prerequisite.

**Freeze these internal contracts first** (signature sketches; constructors private to the engine):

```rust
enum Partition {
    RequestInvariant,
    File(FileGeneration),
    Project(RequiredPartitions),
}
struct DemandProgram {
    actions: Box<[DemandAction]>, // ordered demand/empty-gate/fault/barrier
}
struct AdmittedInput<'a> {
    proof: PopulationProof,      // owner + generation + logical origin
    root: ProducerRoot,
    partition: Partition,
    readiness: Readiness,        // Complete / Skipped / Unknown
    rows: InputDelivery<'a>,     // leased publication or declared stream
}
trait Inputs {
    fn admit(&mut self, input: InputId)
        -> Result<AdmittedInput<'_>, OperatingError>;
}
```

`Inputs` is application-private; only the shared runtime interprets `DemandProgram`. Kernels receive already-admitted borrowed values/views/scratch, never dependency resolvers. Every selectable mode/representation conforms to explicit, possibly conditioned demand/error order; do not silently unify existing variant semantics. Controls cover empty-left/failing-right and competing faults for Join/Axis/set variants. No owner/provenance defaults, implicit empty inputs or invalid-ordinal `AtHand`. Strategy decline precedes observable work; real errors never retry another engine.

Readiness is per `(producer root, partition)`, combined over exactly the consumer's demanded lineage. Persist typed applicability with frontier identity. `Skipped` stays distinct, never rewritten `Complete`; applicability may discharge an obligation. Complete request broadcasts do not expand file lineage; malformed broadcasts may legitimately fail consumers. Prove the syntax-skipped RUF100 witness cold/warm without poisoning unrelated partitions.

| Mandatory boundary | Closure requirement before claiming completion |
|---|---|
| ☐ Relations: 69, ten providers | Explicit absence/partials, file readiness, project/host/render/replay identity. Preserve syntax recovery and refused reads. |
| ☐ Operators: 26, every applicable physical route | Borrowed semantic kernels; engine-owned demand/error order, scratch and publication. Remove recursive domain `fragment::make` scheduling and native memo ownership. |
| ☐ Derived: 13 kinds | One acquisition/budget/lease authority across fanout, native, raw ordering, graph and partition transfer. Current source already migrated several routes; re-prove actual consumers. |
| ☐ Inputs/frontiers/store | Explicit provenance, required partitions, completeness and warm invalidation; lazy frontier decoding; last-consumer release on success/cancellation/error. Immediate Feed hardening is integration-owned. |
| ☐ Enforcement | Authoritative enums plus resolved route inventory; missing/new/unmapped implementations fail. Rust visibility/Tastemaker enforce structure; behavioral negatives prove bypass rejection. Update existing matrix only. |

**First structural bet and deletions:**

- [ ] Compile all 413 rules into one dense program. Native kernels become instructions; ordinary operators are single-instruction regions. Then fuse eligible chains into direct continuations and share pure scans/prefixes, preserving fault/effect/observation barriers, duplicate order and emitted prefixes.
- [ ] **Fusion eligibility:** retain owner-issued identity wherever derived acquisition, shared consumers, frontier/observation/capture/handover, partition or lifetime boundaries require it. Never erase that identity to fuse. Invocation-local derived work needs explicit scoped ownership; no invented shared-cache key. Unclassified routes block compilation. Representation choices follow this contract, without mandatory extra materialization.
- [ ] Delete superseded tables/publications/rooms/bookkeeping only where eligibility proves them unnecessary; replace original-plan execution metadata, recursive fragment scheduling and eager frontier preparation. Preserve logical attribution/counting and required lifetime obligations. No production selector, fallback pipeline or permanent `native_join_right_first` patch flag.
- [ ] Measure full-413 cold/warm/edit plus existing Rust regressions, identical inventory/settings and complete reports. Proposed gain screen: ≥20% lower cold CPU, lower peak RSS, no >5% regression beyond drift elsewhere. **Initial slowdown rejects a gain claim, not the structural design:** profile transition overhead, remove duplicated work, then remeasure. Counters/tiny files are diagnostics.

**Coherent freezes:** contracts/inventory → sole executor with single-instruction regions → legal fusion/deletion. Each gets appropriate full-scope parity/no-new-failure receipts against its frozen external baseline. Map all 36 inherited core failures to contracts; fix them without deleting or weakening order/reuse assertions to output-only checks. Final acceptance requires their resolution, complete all-413 reports and independent review.

**Evidence:** the current route gaps; the 714/763-second file phase and rejected sharing; one syntax-skipped file poisoning RUF100 (all three kept outside the repository). [Historical decisions, line 319](https://github.com/idiomatic-sh/idiomatic-internal/blob/main/.agents/design-history/ryo/decisions.md) and [spike mapping, row 54](https://github.com/idiomatic-sh/idiomatic-internal/blob/main/.agents/design-history/ryo/spike-mapping.md) show the shared-walk compiler was not delivered; per-plan streaming replaced it. The eight-rule spike proves neither full-413 semantics nor performance. The detailed source evidence, kept outside the repository, is supporting research, not another work ledger.
