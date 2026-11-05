# Contract migration agent

Move the implementation and authority for every contract in the existing
[ownership matrix](../../crates/ryo/docs/contracts/README.md) into its declared crate.
Read the entire matrix, including relation, operator, derived, and route inventories.
Treat missing mappings or enforcement as failures, not exemptions.

## Implementation order

1. Move the actual implementation and authority into the owning crate first.
   Accept a broken compile while the boundary is being established.
2. Migrate every caller promptly to the required public contract. Remove old
   entry points, compatibility re-exports, optional owners, and second pipelines,
   including obsolete test executors and adapters.
   Never wait for a clean core build before making the crate move.
3. Keep scheduling, cache identity, invalidation, data acquisition, publication,
   and lifetime under the execution engine. Semantic crates must not depend on
   the engine or mint its authority.
4. Use Rust visibility and crate dependencies to make forbidden execution
   paths inaccessible. Use narrow engine-issued capabilities for supported calls.
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
- The normal CI contract gate must cover the whole matrix inventory and every
  declared route. Missing, stale, or unexecuted enforcement is red. A failed
  build is missing proof, never a successful forbidden-call negative control.
- Update only the existing matrix with the exact source checkpoint and proof
  status, including unexecuted controls. Keep unchecked guarantees unchecked
  until their controls pass.
- Send exact source and receipts to the independent
  [contract review agent](contract-review.md). Never self-approve a boundary.
