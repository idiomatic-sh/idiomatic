# Extension ownership contracts

The engine should own scheduling, sharing, invalidation, and lifetime so a language provider or RyoQL primitive can plug in facts or typed row semantics. This table records **current ownership**, not the desired performance or type-safety level. `☑` means the generic engine owns the responsibility today; `☐` means a language, primitive, compiler extension, or outbound adapter still owns part of that responsibility; `—` means it does not apply. Producing correct facts or row semantics alone does not make an implementor the owner of stored data or scheduling. A checked cell is not a performance claim.

| Contract | Scheduling | Sharing / cache | Invalidation | Data ownership | Resource lifetime | Residual implementor ownership or central gap |
| --- | :---: | :---: | :---: | :---: | :---: | --- |
| Language parse and project | ☑ | — | ☑ | ☐ | ☐ | Provider owns parser state and its call-local scratch; engine requests and releases resident facts. Provider fact semantics are separate. |
| Typed relation catalog | ☑ | — | — | ☑ | ☑ | Engine owns typed resident columns; provider populates their values. Semantic row correctness remains a provider contract. |
| Complete and partial facts | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns reuse refusal and resident generations; provider must truthfully report recovery and partials. |
| Provider walk vs execution demand | ☑ | — | — | ☑ | — | Engine owns both demands. Raw slices still permit confusing them; a typed wrapper is pending, without changing ownership. |
| Source bytes and spans | ☑ | — | ☑ | ☑ | ☑ | Engine holds the file buffer; provider supplies token coordinates and lexical meaning, not a retained cache. |
| Source and derived symbols | — | — | ☑ | ☑ | ☑ | Engine owns one file's symbol table and query mutation capability; provider supplies source name semantics. |
| Query grammar and typed plan | ☑ | — | ☑ | ☑ | ☑ | Compiler is an engine component; new syntax still requires compile/typecheck error mapping. |
| Primitive schema and evaluator | ☑ | ☐ | ☐ | ☑ | ☐ | Some primitives still allocate and manage operator-local indexes/scratch directly. Registration covers six of 24 kinds; global policy is not uniformly declared. |
| Operator inputs and absence | ☐ | — | — | ☐ | — | Implementors still add per-kind dependency, trust, feature, and provenance branches in several engine services. |
| Ordinary plan and frontier | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns ordering, rule-local tables, frontier handover, and existing opt-in sharing; complete DAG scheduling is still missing. |
| Request-wide file graph | ☑ | ☑ | ☑ | ☑ | ☑ | The opt-in graph state is engine-owned and file-local. Its current overlay still eagerly runs some ancestors and is slower on the measured corpus. |
| Stored selections and indexes | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns predicate/index state, bounds, fallback, and release; provider only supplies complete resident columns. |
| Graph adjacency and traversal | ☑ | ☑ | ☑ | ☑ | ☑ | Engine owns typed per-file graph views and memo. Alternate storage remains unmeasured. |
| Persistent lint snapshot | ☑ | ☑ | ☑ | ☐ | ☐ | Engine chooses digests/reuse; outbound adapter owns framing, snapshot file handles, and atomic byte replacement behind the port. |
| Outbound reports and jobs | ☑ | — | ☑ | ☑ | ☐ | Engine owns reports and visit order; scheduler adapter still owns executor resources behind its port. |

The open ownership cells identify code that a new implementor must still coordinate. A provider supplies facts and completeness evidence; a primitive supplies typed row semantics. The engine should validate inputs and outputs, schedule their dependencies, share safe work, invalidate it, and release each file generation. Incomplete performance or type-safety evidence is named in the last column and does not by itself turn an engine-owned cell into an implementor-owned cell.

## Executable evidence and open gates

- [Primitive registry and conformance](../../crates/core/src/domain/operator/contract_test.rs) exercises six registered kinds, including ordered bags, errors, and counted scaling. The other 18 kinds still need the same behavioral contract; a catalog match alone is insufficient.
- [Source readiness](../../crates/core/src/domain/table_model/service_test.rs) distinguishes stored/native complete rows from partial or recovered input. [QuerySymbols tests](../../crates/language/src/domain/symbols/service_test.rs) protect source versus derived symbol provenance.
- [Request graph tests](../../crates/core/src/domain/file_program/service_test.rs) cover identity, successful cut reuse, failure, last-consumer release, and demanded ancestor reads. The [scoped graph receipt](../../languages/python/benchmarks/linters/ruff/results/file-program-graph-2026-11-02/summary.json) proves one 12-rule Join reuse but a 1.63× full-Django cold slowdown on the first overlay. The demand-mask follow-up skips 44 ancestor steps in the one-file trace but its complete cold run remains slower at 435.77 s versus the retained 248.62 s graph-off run (1.75×); neither single observation establishes stable component attribution.
- The [pipeline Tastemaker spec](../../../../tastemaker/specs/pipeline/0.1.0/spec.toml) currently checks structure and test registration. A string containing “bounded” is not behavioral proof of bounded state. Product lifecycle tests with repeated unchanged and changed runs, measured retained state, and a deliberately unbounded negative control are required before a bounded-lifetime checkbox can be credited to that spec.

The [Ryo design](../README.md) is the architectural source. Update this table only when a passing executable contract changes ownership. Keep source-bound fixture, lifecycle, and performance receipts separate; do not mark an opt-in strategy production-ready from one-file parity alone.
