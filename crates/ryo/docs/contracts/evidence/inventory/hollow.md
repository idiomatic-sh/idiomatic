# Hollow concepts

## Zero production lines

None.

## Production lines but no item with a visibility

The directory holds code, but no struct, enum, trait, function or type alias is written with a visibility other than private: what it holds is re-exports, module declarations or private helpers.

| crate | layer | concept | production lines |
|---|---|---|---|
| core | application | cross_file/ordinary | 125 |
| core | application | cross_file/phase | 203 |
| core | application | file_analysis/evaluation | 304 |
| core | application | file_analysis/outcome | 125 |
| core | application | file_analysis/project | 237 |
| core | application | file_analysis/replaying | 298 |
| core | outbound | language/adapters | 8 |
| execution | outbound | query_language/adapters | 7 |
| execution_domain | application | matching | 31 |
| execution_runtime | application | file_dag/program/admission/admitted | 271 |
| execution_runtime | application | file_dag/program/admission/available | 114 |
| execution_runtime | application | file_dag/program/admission/partitions | 191 |
| execution_runtime | application | file_dag/program/facade_runtime/file_execution/emission | 141 |
| execution_runtime | application | file_dag/program/facade_runtime/file_execution/frontier | 194 |
| execution_runtime | application | file_dag/program/facade_runtime/file_execution/presence | 392 |
| execution_runtime | application | file_dag/program/facade_runtime/file_execution/rule | 203 |
| execution_runtime | application | file_dag/program/facade_runtime/file_execution/walk | 387 |
| execution_runtime | application | file_dag/program/facade_runtime/project_execution/deliver | 158 |
| execution_runtime | application | file_dag/program/facade_runtime/project_execution/sources | 172 |
| execution_runtime | application | file_dag/program/facade_runtime/source_admission/project | 150 |
| execution_runtime | application | file_dag/program/facade_runtime/source_admission/publication | 159 |
| execution_runtime | application | file_dag/program/kernel_session/evaluation | 336 |
| execution_runtime | application | file_dag/program/kernel_session/strategies | 102 |
| execution_runtime | application | file_dag/program/preparation/service/replay | 125 |
| execution_runtime | application | file_dag/program/preparation/service/schedule | 53 |
| execution_runtime | application | file_dag/program/runtime/service/cleanup | 178 |
| execution_runtime | application | file_dag/program/runtime/service/counts | 83 |
| execution_runtime | application | file_dag/program/runtime/service/evaluation | 256 |
| execution_runtime | application | file_dag/program/runtime/service/permits | 184 |
| execution_runtime | application | file_dag/program/runtime/service/physical | 67 |
| execution_runtime | application | file_dag/program/runtime/service/probes | 216 |
| execution_runtime | application | file_dag/program/runtime/service/scheduling | 193 |
| execution_runtime | application | file_dag/program/runtime/service/state | 363 |
| execution_runtime | application | file_dag/program/runtime/service/strategies | 191 |
| execution_runtime | application | file_dag/program/runtime/service/walks | 70 |
| execution_runtime | outbound | operators/adapters | 6 |
| execution_store | application | file_dag/derived/owner/acquisition | 176 |
| execution_store | application | file_dag/derived/owner/acquisition/admission/service/issuance | 234 |
| execution_store | application | file_dag/derived/owner/acquisition/scoped | 112 |
| execution_store | application | file_dag/relations/admission/service/issuance | 132 |
| execution_store | application | file_dag/relations/inspection | 183 |
| execution_store | application | file_dag/relations/number/service/issuance | 74 |
| execution_store | application | file_dag/relations/packed/service/issuance | 48 |
| execution_store | application | file_dag/relations/producer | 136 |
| execution_store | application | file_dag/relations/source | 111 |
| execution_store | outbound | derived/adapters | 7 |
| operators | outbound | matching/adapters | 6 |
| relations | domain | catalog | 150 |
| relations | domain | kinds | 103 |
