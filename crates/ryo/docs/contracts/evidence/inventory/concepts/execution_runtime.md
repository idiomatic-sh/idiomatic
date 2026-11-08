# Concepts: execution_runtime

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | file_dag | 2605 | 28 | service |
| application | file_dag/execution/lease/capability | 46 | 2 | service |
| application | file_dag/frontiers | 794 | 4 | service, types |
| application | file_dag/invocation | 699 | 7 | service |
| application | file_dag/invocation/prepared_use | 16 | 2 | service |
| application | file_dag/observations | 465 | 4 | types |
| application | file_dag/observations/restore | 260 | 1 | service |
| application | file_dag/program/admission/admitted | 266 | 2 | service |
| application | file_dag/program/admission/available | 112 | 2 | service |
| application | file_dag/program/admission/partitions | 192 | 2 | service |
| application | file_dag/program/facade_runtime | 2362 | 66 | service, types |
| application | file_dag/program/facade_runtime/file_execution/emission | 141 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/frontier | 194 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/rule | 201 | 2 | service |
| application | file_dag/program/facade_runtime/file_execution/walk | 305 | 2 | service |
| application | file_dag/program/facade_runtime/project_execution/deliver | 158 | 2 | service |
| application | file_dag/program/facade_runtime/project_execution/sources | 174 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/acquisition | 260 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/project | 147 | 2 | service |
| application | file_dag/program/facade_runtime/source_admission/publication | 156 | 2 | service |
| application | file_dag/program/file_session | 361 | 4 | interface, service, types |
| application | file_dag/program/kernel_session/evaluation | 318 | 2 | service |
| application | file_dag/program/kernel_session/strategies | 99 | 2 | service |
| application | file_dag/program/native_walk/lease | 71 | 2 | service |
| application | file_dag/program/native_walk/project | 358 | 2 | service |
| application | file_dag/program/observation_stage | 191 | 4 | interface, service, types |
| application | file_dag/program/observed_publication | 52 | 3 | service, types |
| application | file_dag/program/preparation | 436 | 3 | service, types |
| application | file_dag/program/preparation/service/census | 52 | 2 | service |
| application | file_dag/program/preparation/service/demands | 66 | 2 | service |
| application | file_dag/program/preparation/service/metadata | 136 | 2 | service |
| application | file_dag/program/preparation/service/replay | 125 | 2 | service |
| application | file_dag/program/preparation/service/schedule | 51 | 2 | service |
| application | file_dag/program/prepared_execution | 272 | 4 | interface, service, types |
| application | file_dag/program/project_phase | 199 | 4 | interface, service, types |
| application | file_dag/program/project_population | 104 | 3 | service, types |
| application | file_dag/program/request_execution | 455 | 4 | interface, service, types |
| application | file_dag/program/restored | 31 | 1 | types |
| application | file_dag/program/rule_outcome | 60 | 2 | types |
| application | file_dag/program/runtime | 401 | 6 | service, types |
| application | file_dag/program/runtime/service/admission | 155 | 2 | service |
| application | file_dag/program/runtime/service/cleanup | 181 | 2 | service |
| application | file_dag/program/runtime/service/counts | 82 | 2 | service |
| application | file_dag/program/runtime/service/evaluation | 257 | 2 | service |
| application | file_dag/program/runtime/service/invocation | 310 | 2 | service |
| application | file_dag/program/runtime/service/permits | 182 | 2 | service |
| application | file_dag/program/runtime/service/physical | 67 | 2 | service |
| application | file_dag/program/runtime/service/probes | 192 | 2 | service |
| application | file_dag/program/runtime/service/scheduling | 185 | 2 | service |
| application | file_dag/program/runtime/service/state | 284 | 2 | service |
| application | file_dag/program/runtime/service/strategies | 147 | 2 | service |
| application | file_dag/program/runtime/service/walks | 71 | 2 | service |
| outbound | digest | 44 | 5 | interface, types |
| outbound | digest/adapters/blake3 | 46 | 1 | adapter |
| outbound | facts | 266 | 7 | interface, types |
| outbound | facts/adapters/memory | 203 | 3 | adapter, types |
| outbound | facts/adapters/spill | 423 | 3 | adapter, types |
| outbound | operators | 30 | 3 | interface |
| outbound | operators/adapters | 6 | 1 | use_case |
| outbound | operators/adapters/use_case | 26 | 1 | adapter |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
