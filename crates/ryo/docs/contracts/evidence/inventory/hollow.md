# Hollow concepts

## Zero production lines

None.

## Production lines but no item with a visibility

The directory holds code, but no struct, enum, trait, function or type alias is written with a visibility other than private: what it holds is re-exports, module declarations or private helpers.

| crate | layer | concept | production lines |
|---|---|---|---|
| core | application | cross_file/ordinary | 122 |
| core | application | cross_file/phase | 190 |
| core | application | file_analysis/evaluation | 301 |
| core | application | file_analysis/gating | 175 |
| core | application | file_analysis/outcome | 118 |
| core | application | file_analysis/project | 238 |
| core | application | file_analysis/replaying | 285 |
| core | outbound | language/adapters | 8 |
| execution | application | file_dag/derived/owner/acquisition | 176 |
| execution | application | file_dag/derived/owner/acquisition/admission/service/issuance | 229 |
| execution | application | file_dag/derived/owner/acquisition/scoped | 111 |
| execution | application | file_dag/program/admission/admitted | 269 |
| execution | application | file_dag/program/admission/available | 112 |
| execution | application | file_dag/program/admission/partitions | 192 |
| execution | application | file_dag/program/facade_runtime/file_execution/emission | 145 |
| execution | application | file_dag/program/facade_runtime/file_execution/frontier | 194 |
| execution | application | file_dag/program/facade_runtime/file_execution/rule | 205 |
| execution | application | file_dag/program/facade_runtime/file_execution/walk | 305 |
| execution | application | file_dag/program/facade_runtime/project_execution/deliver | 158 |
| execution | application | file_dag/program/facade_runtime/project_execution/sources | 174 |
| execution | application | file_dag/program/facade_runtime/source_admission/project | 147 |
| execution | application | file_dag/program/facade_runtime/source_admission/publication | 156 |
| execution | application | file_dag/program/kernel_session/evaluation | 333 |
| execution | application | file_dag/program/kernel_session/strategies | 103 |
| execution | application | file_dag/program/preparation/service/replay | 125 |
| execution | application | file_dag/program/preparation/service/schedule | 51 |
| execution | application | file_dag/program/runtime/service/cleanup | 184 |
| execution | application | file_dag/program/runtime/service/counts | 82 |
| execution | application | file_dag/program/runtime/service/evaluation | 252 |
| execution | application | file_dag/program/runtime/service/permits | 185 |
| execution | application | file_dag/program/runtime/service/physical | 67 |
| execution | application | file_dag/program/runtime/service/probes | 189 |
| execution | application | file_dag/program/runtime/service/scheduling | 175 |
| execution | application | file_dag/program/runtime/service/state | 285 |
| execution | application | file_dag/program/runtime/service/strategies | 150 |
| execution | application | file_dag/program/runtime/service/walks | 71 |
| execution | application | file_dag/relations/admission/service/issuance | 121 |
| execution | application | file_dag/relations/inspection | 171 |
| execution | application | file_dag/relations/number/service/issuance | 70 |
| execution | application | file_dag/relations/packed/service/issuance | 49 |
| execution | application | file_dag/relations/producer | 139 |
| execution | application | file_dag/relations/source | 106 |
| execution | outbound | derived/adapters | 7 |
| execution | outbound | operators/adapters | 6 |
| execution | outbound | query_language/adapters | 7 |
| operators | outbound | matching/adapters | 6 |
| relations | domain | catalog | 150 |
| relations | domain | kinds | 103 |
