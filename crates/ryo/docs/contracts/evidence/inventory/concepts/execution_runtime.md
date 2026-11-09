# Concepts: execution_runtime

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | file_dag | 2701 | 28 | service |
| application | file_dag/execution/lease/capability | 42 | 2 | service |
| application | file_dag/frontiers | 862 | 4 | service, types |
| application | file_dag/invocation | 740 | 7 | service |
| application | file_dag/invocation/prepared_use | 19 | 2 | service |
| application | file_dag/observations | 474 | 4 | types |
| application | file_dag/observations/restore | 274 | 1 | service |
| application | file_dag/program/admission/admitted | 271 | 2 | service |
| application | file_dag/program/admission/available | 114 | 2 | service |
| application | file_dag/program/admission/partitions | 191 | 2 | service |
| application | file_dag/program/facade_runtime | 2533 | 69 | service, types |
| application | file_dag/program/facade_runtime/file_execution/emission | 141 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/frontier | 194 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/presence | 392 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/rule | 203 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/walk | 387 | 2 | service |
| application | file_dag/program/facade_runtime/project_execution/deliver | 158 | 2 | service |
| application | file_dag/program/facade_runtime/project_execution/sources | 172 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/acquisition | 277 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/project | 150 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/publication | 159 | 2 | service |
| application | file_dag/program/file_session | 381 | 4 | interface, service, types |
| application | file_dag/program/kernel_session/evaluation | 336 | 2 | service |
| application | file_dag/program/kernel_session/strategies | 102 | 2 | service |
| application | file_dag/program/native_walk/lease | 72 | 2 | service |
| application | file_dag/program/native_walk/project | 346 | 2 | service |
| application | file_dag/program/observation_stage | 216 | 4 | interface, service, types |
| application | file_dag/program/observed_publication | 57 | 3 | service, types |
| application | file_dag/program/preparation | 446 | 3 | service, types |
| application | file_dag/program/preparation/service/census | 48 | 2 | service |
| application | file_dag/program/preparation/service/demands | 66 | 2 | service |
| application | file_dag/program/preparation/service/metadata | 143 | 2 | service |
| application | file_dag/program/preparation/service/replay | 125 | 2 | service |
| application | file_dag/program/preparation/service/schedule | 53 | 2 | service |
| application | file_dag/program/prepared_execution | 224 | 4 | interface, service, types |
| application | file_dag/program/project_phase | 474 | 4 | interface, service, types |
| application | file_dag/program/project_population | 103 | 3 | service, types |
| application | file_dag/program/request_execution | 453 | 4 | interface, service, types |
| application | file_dag/program/restored | 33 | 1 | types |
| application | file_dag/program/rule_outcome | 60 | 2 | types |
| application | file_dag/program/runtime | 444 | 6 | service, types |
| application | file_dag/program/runtime/service/admission | 152 | 2 | service |
| application | file_dag/program/runtime/service/cleanup | 178 | 2 | service |
| application | file_dag/program/runtime/service/counts | 83 | 2 | service |
| application | file_dag/program/runtime/service/evaluation | 256 | 2 | service |
| application | file_dag/program/runtime/service/invocation | 323 | 2 | service |
| application | file_dag/program/runtime/service/permits | 184 | 2 | service |
| application | file_dag/program/runtime/service/physical | 67 | 2 | service |
| application | file_dag/program/runtime/service/probes | 216 | 2 | service |
| application | file_dag/program/runtime/service/scheduling | 193 | 2 | service |
| application | file_dag/program/runtime/service/state | 363 | 2 | service |
| application | file_dag/program/runtime/service/strategies | 191 | 2 | service |
| application | file_dag/program/runtime/service/walks | 70 | 2 | service |
| outbound | digest | 44 | 5 | interface, types |
| outbound | digest/adapters/blake3 | 46 | 1 | adapter |
| outbound | facts | 263 | 7 | interface, types |
| outbound | facts/adapters/memory | 201 | 3 | adapter, types |
| outbound | facts/adapters/spill | 421 | 3 | adapter, types |
| outbound | operators | 30 | 3 | interface |
| outbound | operators/adapters | 6 | 1 | use_case |
| outbound | operators/adapters/use_case | 28 | 1 | adapter |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
