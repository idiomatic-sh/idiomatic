# Concepts: execution

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | file_dag | 8891 | 59 | types |
| application | file_dag/derived/ledger | 574 | 8 | types |
| application | file_dag/derived/owner | 1680 | 16 | types |
| application | file_dag/derived/owner/acquisition | 178 | 3 | service |
| application | file_dag/derived/owner/acquisition/admission | 194 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/admission/service/issuance | 255 | 2 | service |
| application | file_dag/derived/owner/acquisition/graph | 332 | 2 | service |
| application | file_dag/derived/owner/acquisition/graph/construction | 208 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/graph/identity | 159 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/population | 375 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/scoped | 111 | 2 | service |
| application | file_dag/derived/owner/acquisition/selection | 380 | 3 | service, types |
| application | file_dag/derived/owner/acquisition/selection/predicate | 149 | 2 | service |
| application | file_dag/derived/owner/acquisition/semantics | 613 | 5 | service, types |
| application | file_dag/derived/owner/cache | 130 | 2 | types |
| application | file_dag/derived/owner/cached_graph | 337 | 3 | service, types |
| application | file_dag/derived/owner/footprint | 18 | 2 | service |
| application | file_dag/derived/owner/graphs | 499 | 10 | service, types |
| application | file_dag/derived/owner/graphs/types/handles | 204 | 3 | service, types |
| application | file_dag/derived/owner/graphs/types/reserve | 8 | 1 | types |
| application | file_dag/derived/owner/graphs/types/retained | 60 | 2 | types |
| application | file_dag/derived/owner/graphs/types/roots/completed | 31 | 2 | service |
| application | file_dag/derived/owner/index | 187 | 3 | service, types |
| application | file_dag/derived/owner/selection | 349 | 9 | types |
| application | file_dag/derived/owner/selection/candidates | 243 | 2 | service |
| application | file_dag/derived/owner/selection/fanout | 151 | 2 | service |
| application | file_dag/derived/owner/selection/literals | 33 | 2 | service |
| application | file_dag/derived/owner/selection/preparation | 258 | 2 | service |
| application | file_dag/derived/owner/witness | 37 | 2 | service |
| application | file_dag/derived/population | 8 | 2 | types |
| application | file_dag/derived/prepared | 12 | 1 | types |
| application | file_dag/derived/request | 6 | 1 | types |
| application | file_dag/derived/request/identity | 111 | 2 | types |
| application | file_dag/execution/lease/capability | 45 | 2 | service |
| application | file_dag/executor/response | 16 | 2 | types |
| application | file_dag/frontiers | 591 | 2 | service, types |
| application | file_dag/invocation/native | 44 | 2 | service |
| application | file_dag/invocation/prepared_use | 16 | 2 | service |
| application | file_dag/program/admission/admitted | 224 | 2 | service |
| application | file_dag/program/admission/available | 112 | 2 | service |
| application | file_dag/program/admission/partitions | 187 | 2 | service |
| application | file_dag/program/facade_runtime | 2455 | 69 | service, types |
| application | file_dag/program/facade_runtime/file_execution/emission | 144 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/frontier | 194 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/rule | 204 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/walk | 305 | 2 | service |
| application | file_dag/program/facade_runtime/project_execution/deliver | 156 | 2 | service |
| application | file_dag/program/facade_runtime/project_execution/sources | 174 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/acquisition | 266 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/project | 144 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/publication | 167 | 2 | service |
| application | file_dag/program/kernel_session/evaluation | 397 | 2 | service |
| application | file_dag/program/kernel_session/positions | 241 | 2 | service |
| application | file_dag/program/kernel_session/strategies | 313 | 2 | service |
| application | file_dag/program/native_join/lease | 51 | 2 | service |
| application | file_dag/program/native_join/lease/service/dispatch | 76 | 2 | service |
| application | file_dag/program/native_walk/lease | 70 | 2 | service |
| application | file_dag/program/preparation | 468 | 3 | service, types |
| application | file_dag/program/preparation/service/census | 52 | 2 | service |
| application | file_dag/program/preparation/service/demands | 66 | 2 | service |
| application | file_dag/program/preparation/service/metadata | 131 | 2 | service |
| application | file_dag/program/preparation/service/orders | 49 | 2 | service |
| application | file_dag/program/preparation/service/replay | 125 | 2 | service |
| application | file_dag/program/preparation/service/schedule | 51 | 2 | service |
| application | file_dag/program/project_population | 104 | 3 | service, types |
| application | file_dag/program/protocol | 196 | 3 | service, types |
| application | file_dag/program/restored | 32 | 1 | types |
| application | file_dag/program/runtime | 462 | 6 | service, types |
| application | file_dag/program/runtime/service/admission | 155 | 2 | service |
| application | file_dag/program/runtime/service/cleanup | 184 | 2 | service |
| application | file_dag/program/runtime/service/counts | 103 | 2 | service |
| application | file_dag/program/runtime/service/evaluation | 305 | 2 | service |
| application | file_dag/program/runtime/service/invocation | 297 | 2 | service |
| application | file_dag/program/runtime/service/permits | 269 | 2 | service |
| application | file_dag/program/runtime/service/physical | 78 | 2 | service |
| application | file_dag/program/runtime/service/probes | 190 | 2 | service |
| application | file_dag/program/runtime/service/scheduling | 175 | 2 | service |
| application | file_dag/program/runtime/service/state | 286 | 2 | service |
| application | file_dag/program/runtime/service/strategies | 257 | 2 | service |
| application | file_dag/program/runtime/service/walks | 156 | 2 | service |
| application | file_dag/relations/admission | 25 | 2 | service |
| application | file_dag/relations/admission/service/issuance | 109 | 2 | service |
| application | file_dag/relations/generation | 21 | 2 | service |
| application | file_dag/relations/handle | 63 | 2 | service |
| application | file_dag/relations/number | 27 | 2 | service |
| application | file_dag/relations/number/service/issuance | 70 | 2 | service |
| application | file_dag/relations/packed | 16 | 2 | service |
| application | file_dag/relations/packed/service/issuance | 49 | 2 | service |
| application | file_dag/relations/publication | 68 | 2 | types |
| application | file_dag/scope | 18 | 1 | types |
| application | rule_compilation | 742 | 7 | service, types |
| domain | column_kernel | 449 | 4 | service, types |
| domain | comparing | 442 | 3 | service |
| domain | comparing/collections/comparisons | 546 | 15 | interface |
| domain | completion | 489 | 5 | service, types |
| domain | coverage | 394 | 3 | service, types |
| domain | crossing | 8 | 1 | types |
| domain | derived_views | 240 | 2 | service, types |
| domain | executable_program | 1720 | 9 | service, types |
| domain | executable_program/file_demand | 331 | 3 | service, types |
| domain | executable_program/fusion | 297 | 3 | service, types |
| domain | features | 179 | 2 | service, types |
| domain | file_program | 1419 | 5 | service, types |
| domain | finish | 463 | 4 | service, types |
| domain | frontiers | 56 | 1 | service |
| domain | functions | 35 | 2 | service, types |
| domain | hosted | 111 | 3 | service, types |
| domain | kind_check | 270 | 3 | service, types |
| domain | kind_sets | 363 | 5 | service, types |
| domain | lowering | 485 | 3 | service, types |
| domain | matching | 211 | 4 | service, types |
| domain | native_positions | 1276 | 8 | service, types |
| domain | numeric_values | 273 | 2 | service, types |
| domain | operands | 69 | 1 | service |
| domain | operating | 332 | 3 | service, types |
| domain | operation_schema | 779 | 7 | service, types |
| domain | operator | 535 | 6 | service, types |
| domain | operator/collections/operators | 3211 | 63 | interface |
| domain | operator/contract | 1836 | 16 | service, types |
| domain | ordered_existence | 354 | 3 | service, types |
| domain | placement | 225 | 2 | service, types |
| domain | positions | 212 | 3 | service, types |
| domain | prepared_request | 52 | 2 | service, types |
| domain | readiness | 230 | 3 | types |
| domain | replay | 103 | 2 | service, types |
| domain | rules | 21 | 2 | service |
| domain | selections | 211 | 2 | service |
| domain | shared_walk | 1179 | 16 | service, types |
| domain | source_partitions | 346 | 4 | types |
| domain | spelling | 90 | 2 | service |
| domain | step_classes | 265 | 2 | service, types |
| domain | stored_graph | 296 | 3 | types |
| domain | stored_selection | 294 | 3 | types |
| domain | table_cells | 215 | 3 | service, types |
| domain | table_dedup | 114 | 2 | service, types |
| domain | table_groups | 130 | 2 | service, types |
| domain | table_model | 472 | 4 | service, types |
| domain | table_model/collections/representations | 1032 | 19 | interface, service, types |
| domain | table_reach | 272 | 3 | service, types |
| domain | table_scratch | 330 | 2 | service, types |
| domain | table_texts | 133 | 2 | service, types |
| domain | text_gate | 332 | 1 | service |
| domain | tree_join | 493 | 5 | service, types |
| domain | tree_join/collections/strategies | 193 | 6 | interface, types |
| domain | tree_walk | 47 | 1 | service |
| domain | trust | 572 | 3 | service, types |
| domain | walks | 237 | 3 | service, types |
| inbound | use_cases/execution | 1201 | 4 | interface, types, use_case |
| inbound | use_cases/metadata | 283 | 4 | interface, types, use_case |
| inbound | use_cases/publication | 139 | 5 | interface, types, use_case |
| outbound | derived | 13 | 1 | interface |
| outbound | digest | 25 | 2 | interface, types |
| outbound | facts | 209 | 3 | interface, types |
| outbound | matching | 36 | 2 | interface |
| outbound | operators | 16 | 1 | interface |
| outbound | query_language | 49 | 2 | interface |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| domain | 35 |
| inbound | 26 |
