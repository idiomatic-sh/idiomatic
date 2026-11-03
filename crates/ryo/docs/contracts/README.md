# Extension ownership contracts

Ryo lets languages supply facts and operators supply row semantics while the application engine owns scheduling, reuse, invalidation, data, and lifetimes.

- `☑` means an enforced engine-owned route today; `☐` means ownership is split or unproven; `—` means it does not apply. A checked cell is not a speed claim.
- This table describes accepted production source. Parked native and projection experiments receive no credit.

| Contract | Schedule | Share | Invalidate | Data | Lifetime | Current boundary |
| --- | :---: | :---: | :---: | :---: | :---: | --- |
| Language parse and project | ☑ | — | ☑ | ☐ | ☐ | Provider parser state and scratch remain local. |
| Typed relation catalog | ☑ | — | — | ☑ | ☑ | Provider supplies values; engine stores typed columns. |
| Complete and partial facts | ☑ | ☑ | ☑ | ☑ | ☑ | Provider certifies recovery and partials. |
| Provider walk vs execution demand | ☑ | — | — | ☑ | — | Typed demand distinction remains open. |
| Source bytes and spans | ☑ | — | ☑ | ☑ | ☑ | Provider supplies token coordinates. |
| Source and derived symbols | — | — | ☑ | ☑ | ☑ | Provider supplies source-name meaning. |
| Query grammar and typed plan | ☑ | — | ☑ | ☑ | ☑ | Engine compiler; map compile errors. |
| Primitive schema and evaluator | ☐ | ☐ | ☑ | ☑ | ☐ | Optimized routes still bypass one evaluator owner. |
| Operator inputs and absence | ☑ | — | — | ☑ | — | Provider completeness must be truthful. |
| Ordinary plan and frontier | ☐ | ☐ | ☐ | ☑ | ☐ | Stepper and DAG still divide terminal/frontier execution and release. |
| Request-wide file graph | ☐ | ☐ | ☐ | ☑ | ☐ | Stepper retains separate work and release path. |
| Stored selections and indexes | ☐ | ☐ | ☐ | ☑ | ☐ | Stepper and FileIndexes bypass one acquisition owner. |
| Derived view acquisition | ☐ | ☐ | ☐ | ☑ | ☐ | Column kernel bypasses DerivedOwner pins. |
| Graph adjacency and traversal | ☐ | ☐ | ☐ | ☑ | ☐ | Direct graph cache and memo routes bypass one acquisition owner. |
| Persistent lint snapshot | ☑ | ☑ | ☑ | ☐ | ☐ | Adapter owns framing and file handles. |
| Outbound reports and jobs | ☑ | — | ☑ | ☑ | ☐ | Scheduler adapter owns executor resources. |

**Priority:** Fix correctness and ownership bypasses first; otherwise rank avoidable reached work × workload coverage × evidence confidence, then weigh effort and semantic risk.

- [ ] Name the semantic interface and enumerate every applicable implementation for an open route.
- [ ] Give the engine sole admission, scheduling, cache, invalidation, and lifetime authority; add a test that fails on each bypass.
- [ ] Preserve diagnostics, row bags and order, partial facts, lazy errors, and release; measure cold, warm, and edit separately.
- [ ] Repeat source fix → falsifying test → independent review until blocking gaps close.

**Detail:** [Decisions and measured evidence](contract-decisions-and-evidence.md) · [Comparative rubric (draft, under review)](comparative-rubric-draft.md) · [Ryo design](../README.md).
