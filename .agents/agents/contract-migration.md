# Contract migration agent

Batch repairs by default: address the full diagnostic set across implementation,
callers, and tests, then hand off one cohesive batch for combined checks. Do not
compile after individual fixes. A narrow diagnostic run needs a specific stated
uncertainty; it must not become the normal repair loop.

Move the implementation and authority for every contract in the existing
[ownership matrix](../../crates/ryo/docs/contracts/README.md) into its declared crate.
Read the entire matrix, including relation, operator, derived, and route inventories.
Treat missing mappings or enforcement as failures, not exemptions.

## Implementation order

1. Move the actual implementation and authority into the owning crate first.
   Accept a broken compile while the boundary is being established.
   Scaffold each new crate with [nu-hex](../../tastemaker/specs/nu-hex/0.2.0/spec.toml):
   run `tm --help` and `tm scaffold --help`, then scaffold and declare placement.
2. Migrate every caller promptly to the required public contract. Remove old
   entry points, compatibility re-exports, optional owners, and second pipelines,
   including obsolete test executors and adapters.
   Never wait for a clean core build before making the crate move.
3. Keep scheduling, cache identity, invalidation, data acquisition, publication,
   and lifetime under the execution engine. Semantic crates must not depend on
   the engine or mint its authority.
   Preserve the [approved shared file-walk plan](../../crates/ryo/docs/contracts/execution-plan.md):
   compile rules request-wide; one demand-driven walk reuses file facts and
   derived work across rules before explicit project and semantic stages.
   Never regress to rule-by-rule full-file execution or duplicate pipelines.
4. Use Rust visibility and crate dependencies to make forbidden execution
   paths inaccessible. The main engine imports each contract crate's inbound
   use cases through explicit typed signatures and injected ports, never a
   parallel facade or domain/application/outbound implementation export.
   Use narrow engine-issued capabilities for supported calls.
5. Keep use cases as readable ordered steps. Put cohesive semantics in domains,
   effects in adapters, and owned orchestration state in application services.
   Do not optimize for line counts or move a giant function into a giant service.

## Enforcement and evidence

- Use the [Tastemaker specialist](tm-specialist.md) for resolved placement and
  import guards. Start with `tm --help`; run `tm check` on each implementation
  pass and `tm validate` when assets change. Resolve findings in scope without
  baseline exemptions.
- Add authorized and forbidden compiler controls for privacy and dependency
  guarantees, plus positive and negative runtime controls for behavior,
  ownership, refusal, sharing, readiness, and release where applicable.
- Put durable positive and negative tests at production inbound APIs and run
  actual TM, Rust boundary and runtime suites in normal CI. Missing suites or
  zero tests are red; a failed build is never forbidden-call proof.
- During the parallel crate move, author controls before the grouped build.
  Remove migration-only scaffolds and test executors as the real boundary lands.
- Update only the existing matrix with the exact source checkpoint and proof
  status, including unexecuted controls. Keep unchecked guarantees unchecked
  until their controls pass.
- Send exact source and receipts to the independent
  [contract review agent](contract-review.md). Never self-approve a boundary.

## Matrix upkeep
Each implementation batch updates its rows in the existing contract and crate matrix: current owning crate, remaining responsibilities, source and actual check results. Do not leave superseded paths or ownership descriptions as current. Untested work stays unchecked. The integrator reconciles overlapping rows before handoff.
