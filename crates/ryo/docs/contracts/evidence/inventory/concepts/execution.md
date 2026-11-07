# Concepts: execution

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | file_dag | 3591 | 37 | service |
| application | file_dag/closure_partitions | 111 | 3 | types |
| application | file_dag/derived/ledger | 556 | 8 | types |
| application | file_dag/derived/owner | 1524 | 15 | types |
| application | file_dag/derived/owner/acquisition | 176 | 3 | service |
| application | file_dag/derived/owner/acquisition/admission | 178 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/admission/service/issuance | 229 | 2 | service |
| application | file_dag/derived/owner/acquisition/graph | 302 | 2 | service |
| application | file_dag/derived/owner/acquisition/graph/construction | 194 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/graph/identity | 136 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/population | 314 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/scoped | 111 | 2 | service |
| application | file_dag/derived/owner/acquisition/semantics | 484 | 5 | service, types |
| application | file_dag/derived/owner/cache | 69 | 2 | types |
| application | file_dag/derived/owner/cached_graph | 343 | 3 | service, types |
| application | file_dag/derived/owner/footprint | 18 | 2 | service |
| application | file_dag/derived/owner/graphs | 505 | 10 | service, types |
| application | file_dag/derived/owner/graphs/types/handles | 207 | 3 | service, types |
| application | file_dag/derived/owner/graphs/types/reserve | 8 | 1 | types |
| application | file_dag/derived/owner/graphs/types/retained | 69 | 2 | types |
| application | file_dag/derived/owner/graphs/types/roots/completed | 34 | 2 | service |
| application | file_dag/derived/owner/index | 69 | 3 | service, types |
| application | file_dag/derived/owner/witness | 37 | 2 | service |
| application | file_dag/derived/population | 8 | 2 | types |
| application | file_dag/derived/request | 393 | 2 | types |
| application | file_dag/derived/request/identity | 90 | 2 | types |
| application | file_dag/execution/lease/capability | 45 | 2 | service |
| application | file_dag/frontiers | 768 | 4 | service, types |
| application | file_dag/invocation | 728 | 7 | service |
| application | file_dag/invocation/prepared_use | 16 | 2 | service |
| application | file_dag/observations | 465 | 4 | types |
| application | file_dag/observations/restore | 260 | 1 | service |
| application | file_dag/program/admission/admitted | 269 | 2 | service |
| application | file_dag/program/admission/available | 112 | 2 | service |
| application | file_dag/program/admission/partitions | 192 | 2 | service |
| application | file_dag/program/facade_runtime | 2364 | 66 | service, types |
| application | file_dag/program/facade_runtime/file_execution/emission | 145 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/frontier | 194 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/rule | 205 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/walk | 305 | 2 | service |
| application | file_dag/program/facade_runtime/project_execution/deliver | 158 | 2 | service |
| application | file_dag/program/facade_runtime/project_execution/sources | 174 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/acquisition | 260 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/project | 147 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/publication | 156 | 2 | service |
| application | file_dag/program/file_session | 327 | 4 | interface, service, types |
| application | file_dag/program/kernel_session/evaluation | 333 | 2 | service |
| application | file_dag/program/kernel_session/strategies | 103 | 2 | service |
| application | file_dag/program/native_walk/lease | 71 | 2 | service |
| application | file_dag/program/native_walk/project | 356 | 2 | service |
| application | file_dag/program/observation_stage | 191 | 4 | interface, service, types |
| application | file_dag/program/observed_publication | 52 | 3 | service, types |
| application | file_dag/program/preparation | 436 | 3 | service, types |
| application | file_dag/program/preparation/service/census | 52 | 2 | service |
| application | file_dag/program/preparation/service/demands | 66 | 2 | service |
| application | file_dag/program/preparation/service/metadata | 136 | 2 | service |
| application | file_dag/program/preparation/service/replay | 125 | 2 | service |
| application | file_dag/program/preparation/service/schedule | 51 | 2 | service |
| application | file_dag/program/prepared_execution | 246 | 4 | interface, service, types |
| application | file_dag/program/project_phase | 198 | 4 | interface, service, types |
| application | file_dag/program/project_population | 104 | 3 | service, types |
| application | file_dag/program/request_execution | 437 | 4 | interface, service, types |
| application | file_dag/program/restored | 32 | 1 | types |
| application | file_dag/program/rule_outcome | 60 | 2 | types |
| application | file_dag/program/runtime | 392 | 6 | service, types |
| application | file_dag/program/runtime/service/admission | 155 | 2 | service |
| application | file_dag/program/runtime/service/cleanup | 184 | 2 | service |
| application | file_dag/program/runtime/service/counts | 82 | 2 | service |
| application | file_dag/program/runtime/service/evaluation | 252 | 2 | service |
| application | file_dag/program/runtime/service/invocation | 292 | 2 | service |
| application | file_dag/program/runtime/service/permits | 185 | 2 | service |
| application | file_dag/program/runtime/service/physical | 67 | 2 | service |
| application | file_dag/program/runtime/service/probes | 189 | 2 | service |
| application | file_dag/program/runtime/service/scheduling | 175 | 2 | service |
| application | file_dag/program/runtime/service/state | 285 | 2 | service |
| application | file_dag/program/runtime/service/strategies | 150 | 2 | service |
| application | file_dag/program/runtime/service/walks | 71 | 2 | service |
| application | file_dag/relations | 320 | 6 | types |
| application | file_dag/relations/admission | 45 | 3 | service, types |
| application | file_dag/relations/admission/service/issuance | 121 | 2 | service |
| application | file_dag/relations/generation | 25 | 2 | service |
| application | file_dag/relations/handle | 74 | 2 | service |
| application | file_dag/relations/inspection | 171 | 3 | service |
| application | file_dag/relations/number/service/issuance | 70 | 2 | service |
| application | file_dag/relations/packed | 16 | 2 | service |
| application | file_dag/relations/packed/service/issuance | 49 | 2 | service |
| application | file_dag/relations/producer | 139 | 2 | service |
| application | file_dag/relations/publication | 81 | 2 | types |
| application | file_dag/relations/source | 106 | 2 | service |
| application | file_dag/route_counts | 931 | 6 | types |
| application | file_dag/route_counts/profile | 14 | 1 | service |
| application | file_dag/route_counts/progress | 228 | 1 | service |
| application | file_dag/scope | 15 | 1 | types |
| application | file_dag/workspace | 475 | 3 | service |
| application | publication | 42 | 3 | service, types |
| application | rule_compilation | 905 | 7 | service, types |
| domain | comparing | 416 | 4 | service, types |
| domain | comparing/collections/comparisons | 606 | 15 | interface |
| domain | completion | 442 | 4 | service, types |
| domain | contributor_rows | 50 | 2 | service, types |
| domain | coverage | 425 | 3 | service, types |
| domain | derived_views | 210 | 4 | service, types |
| domain | executable_program | 1626 | 16 | service, types |
| domain | executable_program/file_demand | 383 | 3 | service, types |
| domain | executable_program/fusion | 282 | 3 | service, types |
| domain | features | 179 | 2 | service, types |
| domain | file_program | 1530 | 10 | service, types |
| domain | finish | 463 | 4 | service, types |
| domain | frontier_capture | 23 | 1 | types |
| domain | frontier_coverage | 111 | 2 | service |
| domain | frontiers | 64 | 2 | service, types |
| domain | functions | 35 | 2 | service, types |
| domain | hosted | 111 | 3 | service, types |
| domain | kind_check | 304 | 3 | service, types |
| domain | kind_sets | 348 | 5 | service, types |
| domain | lineage | 251 | 2 | service, types |
| domain | matching | 242 | 4 | service, types |
| domain | numeric_values | 375 | 3 | service, types |
| domain | operands | 69 | 1 | service |
| domain | operating | 262 | 3 | service, types |
| domain | operation_schema | 782 | 7 | service, types |
| domain | operator | 430 | 4 | service, types |
| domain | operator/contract | 2865 | 16 | service, types |
| domain | output_coverage | 367 | 3 | service, types |
| domain | placement | 225 | 2 | service, types |
| domain | prepared_request | 52 | 2 | service, types |
| domain | readiness | 237 | 3 | types |
| domain | replay | 103 | 2 | service, types |
| domain | row_emission | 14 | 1 | types |
| domain | rule_intent | 61 | 1 | types |
| domain | rules | 21 | 2 | service |
| domain | self_join_recogniser | 362 | 2 | service, types |
| domain | shared_walk | 1112 | 9 | service, types |
| domain | source_partitions | 346 | 4 | types |
| domain | spelling | 90 | 2 | service |
| domain | step_classes | 282 | 2 | service, types |
| domain | stored_graph | 407 | 3 | types |
| domain | table_cells | 307 | 3 | service, types |
| domain | table_dedup | 115 | 2 | service, types |
| domain | table_groups | 134 | 2 | service, types |
| domain | table_model | 572 | 7 | service, types |
| domain | table_model/collections/representations | 490 | 10 | interface, types |
| domain | table_reach | 292 | 3 | service, types |
| domain | table_scratch | 195 | 3 | service, types |
| domain | table_texts | 135 | 2 | service, types |
| domain | text_gate | 28 | 1 | service |
| domain | tree_walk | 47 | 1 | service |
| domain | trust | 610 | 3 | service, types |
| inbound | use_cases/execution | 172 | 3 | interface, use_case |
| inbound | use_cases/metadata | 310 | 4 | interface, types, use_case |
| inbound | use_cases/prepare | 143 | 4 | interface, types, use_case |
| outbound | derived | 66 | 3 | interface |
| outbound | derived/adapters | 7 | 1 | use_case |
| outbound | derived/adapters/use_case | 71 | 2 | adapter, types |
| outbound | digest | 44 | 5 | interface, types |
| outbound | digest/adapters/blake3 | 46 | 1 | adapter |
| outbound | facts | 263 | 7 | interface, types |
| outbound | facts/adapters/memory | 188 | 3 | adapter, types |
| outbound | facts/adapters/spill | 407 | 3 | adapter, types |
| outbound | matching | 44 | 4 | interface |
| outbound | matching/adapters/globset | 70 | 1 | adapter |
| outbound | operators | 30 | 3 | interface |
| outbound | operators/adapters | 6 | 1 | use_case |
| outbound | operators/adapters/use_case | 26 | 1 | adapter |
| outbound | publication | 33 | 4 | interface |
| outbound | publication/adapters/execution | 39 | 1 | adapter |
| outbound | query_language | 70 | 4 | interface |
| outbound | query_language/adapters | 7 | 1 | use_case |
| outbound | query_language/adapters/use_case | 262 | 2 | adapter, types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| domain | 243 |
| inbound | 21 |
