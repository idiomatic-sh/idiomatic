# Concepts: execution_store

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | file_dag | 868 | 9 | service |
| application | file_dag/closure_partitions | 117 | 3 | types |
| application | file_dag/derived/ledger | 558 | 8 | types |
| application | file_dag/derived/owner | 1579 | 15 | types |
| application | file_dag/derived/owner/acquisition | 176 | 3 | service |
| application | file_dag/derived/owner/acquisition/admission | 171 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/admission/service/issuance | 234 | 2 | service |
| application | file_dag/derived/owner/acquisition/graph | 314 | 2 | service |
| application | file_dag/derived/owner/acquisition/graph/construction | 203 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/graph/identity | 136 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/population | 323 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/scoped | 112 | 2 | service |
| application | file_dag/derived/owner/acquisition/semantics | 477 | 5 | service, types |
| application | file_dag/derived/owner/cache | 68 | 2 | types |
| application | file_dag/derived/owner/cached_graph | 354 | 3 | service, types |
| application | file_dag/derived/owner/footprint | 16 | 2 | service |
| application | file_dag/derived/owner/graphs | 500 | 10 | service, types |
| application | file_dag/derived/owner/graphs/types/handles | 213 | 3 | service, types |
| application | file_dag/derived/owner/graphs/types/reserve | 8 | 1 | types |
| application | file_dag/derived/owner/graphs/types/retained | 69 | 2 | types |
| application | file_dag/derived/owner/graphs/types/roots/completed | 34 | 2 | service |
| application | file_dag/derived/owner/index | 69 | 3 | service, types |
| application | file_dag/derived/owner/witness | 42 | 2 | service |
| application | file_dag/derived/population | 6 | 2 | types |
| application | file_dag/derived/request | 393 | 2 | types |
| application | file_dag/derived/request/identity | 90 | 2 | types |
| application | file_dag/relations | 325 | 6 | types |
| application | file_dag/relations/admission | 54 | 3 | service, types |
| application | file_dag/relations/admission/service/issuance | 132 | 2 | service |
| application | file_dag/relations/generation | 40 | 2 | service |
| application | file_dag/relations/handle | 75 | 2 | service |
| application | file_dag/relations/inspection | 183 | 3 | service |
| application | file_dag/relations/number/service/issuance | 74 | 2 | service |
| application | file_dag/relations/packed | 16 | 2 | service |
| application | file_dag/relations/packed/service/issuance | 48 | 2 | service |
| application | file_dag/relations/producer | 136 | 2 | service |
| application | file_dag/relations/publication | 82 | 2 | types |
| application | file_dag/relations/source | 111 | 2 | service |
| application | file_dag/route_counts | 950 | 6 | types |
| application | file_dag/route_counts/profile | 14 | 1 | service |
| application | file_dag/route_counts/progress | 230 | 1 | service |
| application | file_dag/service | 15 | 1 | types |
| application | file_dag/workspace | 578 | 4 | service, types |
| outbound | derived | 68 | 3 | interface |
| outbound | derived/adapters | 7 | 1 | use_case |
| outbound | derived/adapters/use_case | 73 | 2 | adapter, types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
