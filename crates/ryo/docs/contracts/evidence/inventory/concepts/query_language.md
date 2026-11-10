# Concepts: query_language

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | assembling | 401 | 2 | service |
| domain | binding | 274 | 2 | service, types |
| domain | calling | 361 | 1 | service |
| domain | capture_pushing | 158 | 2 | service, types |
| domain | checking | 226 | 3 | service, types |
| domain | combining | 111 | 1 | service |
| domain | comparing | 441 | 2 | service |
| domain | cutting | 264 | 1 | service |
| domain | expanding | 354 | 2 | service |
| domain | fusing | 834 | 6 | service, types |
| domain | idioms | 322 | 1 | service |
| domain | inlining | 365 | 2 | service, types |
| domain | joining | 369 | 2 | service |
| domain | lineage | 306 | 2 | service, types |
| domain | linking | 72 | 2 | service |
| domain | lowering | 589 | 3 | types |
| domain | narrowing | 220 | 2 | service, types |
| domain | observation_inputs | 67 | 2 | service |
| domain | planning | 332 | 2 | service |
| domain | pruning | 74 | 1 | service |
| domain | pushing | 396 | 2 | service, types |
| domain | refusing | 115 | 1 | service |
| domain | regrouping | 395 | 2 | service, types |
| domain | remapping | 268 | 3 | service, types |
| domain | rendering | 185 | 1 | service |
| domain | reshaping | 296 | 1 | service |
| domain | resolving | 359 | 1 | service |
| domain | rewriting | 246 | 2 | service, types |
| domain | rooting | 243 | 1 | service |
| domain | selecting | 280 | 1 | service |
| domain | selection_pushing | 427 | 2 | service, types |
| domain | spanning | 165 | 1 | service |
| domain | substituting | 305 | 1 | service |
| domain | syntax | 666 | 3 | service, types |
| domain | traversing | 323 | 1 | service |
| domain | validation | 507 | 3 | service, types |
| domain | walking | 403 | 2 | service, types |
| inbound | use_cases/compile | 344 | 6 | interface, service, types, use_case |
| inbound | use_cases/describe | 106 | 5 | interface, types, use_case |
| inbound | use_cases/typecheck | 331 | 5 | interface, service, use_case |
| outbound | catalog | 52 | 6 | interface, types |
| outbound | catalog/adapters/language | 17 | 1 | adapter |
| outbound | syntax | 78 | 5 | interface |
| outbound | syntax/adapters/winnow | 1453 | 2 | adapter |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 31 |
