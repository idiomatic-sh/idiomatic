# Hollow concepts

## Zero production lines

None.

## Production lines but no item with a visibility

The directory holds code, but no struct, enum, trait, function or type alias is written with a visibility other than private: what it holds is re-exports, module declarations or private helpers.

| crate | layer | concept | production lines |
|---|---|---|---|
| core | outbound | language/adapters | 7 |
| core | outbound | query_language/adapters | 7 |
| execution | application | file_dag/derived/owner/acquisition | 178 |
| execution | application | file_dag/derived/owner/acquisition/admission/service/issuance | 255 |
| execution | application | file_dag/derived/owner/acquisition/scoped | 111 |
| execution | application | file_dag/program/admission/admitted | 224 |
| execution | application | file_dag/program/admission/available | 112 |
| execution | application | file_dag/program/admission/partitions | 187 |
| execution | application | file_dag/program/facade_runtime/file_execution/emission | 144 |
| execution | application | file_dag/program/facade_runtime/file_execution/frontier | 194 |
| execution | application | file_dag/program/facade_runtime/file_execution/rule | 204 |
| execution | application | file_dag/program/facade_runtime/file_execution/walk | 305 |
| execution | application | file_dag/program/facade_runtime/project_execution/deliver | 156 |
| execution | application | file_dag/program/facade_runtime/project_execution/sources | 174 |
| execution | application | file_dag/program/facade_runtime/source_admission/project | 144 |
| execution | application | file_dag/program/facade_runtime/source_admission/publication | 167 |
| execution | application | file_dag/program/kernel_session/evaluation | 397 |
| execution | application | file_dag/program/kernel_session/positions | 241 |
| execution | application | file_dag/program/kernel_session/strategies | 313 |
| execution | application | file_dag/program/native_join/lease/service/dispatch | 76 |
| execution | application | file_dag/program/preparation/service/replay | 125 |
| execution | application | file_dag/program/preparation/service/schedule | 51 |
| execution | application | file_dag/program/runtime/service/cleanup | 184 |
| execution | application | file_dag/program/runtime/service/counts | 103 |
| execution | application | file_dag/program/runtime/service/evaluation | 305 |
| execution | application | file_dag/program/runtime/service/permits | 269 |
| execution | application | file_dag/program/runtime/service/physical | 78 |
| execution | application | file_dag/program/runtime/service/probes | 190 |
| execution | application | file_dag/program/runtime/service/scheduling | 175 |
| execution | application | file_dag/program/runtime/service/state | 286 |
| execution | application | file_dag/program/runtime/service/strategies | 257 |
| execution | application | file_dag/program/runtime/service/walks | 156 |
| execution | application | file_dag/relations/admission/service/issuance | 109 |
| execution | application | file_dag/relations/number/service/issuance | 70 |
| execution | application | file_dag/relations/packed/service/issuance | 49 |
| relations | domain | kinds | 103 |
