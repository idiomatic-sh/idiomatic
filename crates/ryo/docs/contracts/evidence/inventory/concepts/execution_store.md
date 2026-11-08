# Concepts: execution_store

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | file_dag | 873 | 9 | service |
| application | file_dag/closure_partitions | 112 | 3 | types |
| application | file_dag/derived/ledger | 558 | 8 | types |
| application | file_dag/derived/owner | 1540 | 15 | types |
| application | file_dag/derived/owner/acquisition | 177 | 3 | service |
| application | file_dag/derived/owner/acquisition/admission | 181 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/admission/service/issuance | 230 | 2 | service |
| application | file_dag/derived/owner/acquisition/graph | 302 | 2 | service |
| application | file_dag/derived/owner/acquisition/graph/construction | 194 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/graph/identity | 136 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/population | 315 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/scoped | 111 | 2 | service |
| application | file_dag/derived/owner/acquisition/semantics | 479 | 5 | service, types |
| application | file_dag/derived/owner/cache | 69 | 2 | types |
| application | file_dag/derived/owner/cached_graph | 349 | 3 | service, types |
| application | file_dag/derived/owner/footprint | 18 | 2 | service |
| application | file_dag/derived/owner/graphs | 504 | 10 | service, types |
| application | file_dag/derived/owner/graphs/types/handles | 211 | 3 | service, types |
| application | file_dag/derived/owner/graphs/types/reserve | 8 | 1 | types |
| application | file_dag/derived/owner/graphs/types/retained | 69 | 2 | types |
| application | file_dag/derived/owner/graphs/types/roots/completed | 34 | 2 | service |
| application | file_dag/derived/owner/index | 69 | 3 | service, types |
| application | file_dag/derived/owner/witness | 42 | 2 | service |
| application | file_dag/derived/population | 8 | 2 | types |
| application | file_dag/derived/request | 396 | 2 | types |
| application | file_dag/derived/request/identity | 90 | 2 | types |
| application | file_dag/relations | 326 | 6 | types |
| application | file_dag/relations/admission | 53 | 3 | service, types |
| application | file_dag/relations/admission/service/issuance | 126 | 2 | service |
| application | file_dag/relations/generation | 40 | 2 | service |
| application | file_dag/relations/handle | 78 | 2 | service |
| application | file_dag/relations/inspection | 173 | 3 | service |
| application | file_dag/relations/number/service/issuance | 69 | 2 | service |
| application | file_dag/relations/packed | 16 | 2 | service |
| application | file_dag/relations/packed/service/issuance | 48 | 2 | service |
| application | file_dag/relations/producer | 141 | 2 | service |
| application | file_dag/relations/publication | 83 | 2 | types |
| application | file_dag/relations/source | 107 | 2 | service |
| application | file_dag/route_counts | 947 | 6 | types |
| application | file_dag/route_counts/profile | 14 | 1 | service |
| application | file_dag/route_counts/progress | 232 | 1 | service |
| application | file_dag/service | 15 | 1 | types |
| application | file_dag/workspace | 563 | 4 | service, types |
| outbound | derived | 66 | 3 | interface |
| outbound | derived/adapters | 7 | 1 | use_case |
| outbound | derived/adapters/use_case | 71 | 2 | adapter, types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
