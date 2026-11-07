# Concepts: core

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | cross_file | 545 | 5 | service, types |
| application | cross_file/completion | 310 | 4 | service, types |
| application | cross_file/preparation | 202 | 4 | service, types |
| application | cross_file/rendering | 330 | 2 | service |
| application | file_analysis | 1780 | 5 | service, types |
| application | project_context | 143 | 2 | service |
| domain | admission | 182 | 3 | service, types |
| domain | allocation_verdict | 114 | 3 | service, types |
| domain | answering | 116 | 2 | service, types |
| domain | budget | 79 | 2 | service |
| domain | choosing | 31 | 1 | service |
| domain | corpus_copies | 111 | 3 | service, types |
| domain | crossing | 210 | 2 | service, types |
| domain | finish | 361 | 4 | service, types |
| domain | handing | 215 | 1 | service |
| domain | keeping | 392 | 2 | service, types |
| domain | listing | 40 | 1 | service |
| domain | ordering | 271 | 3 | service, types |
| domain | project | 133 | 2 | service, types |
| domain | reading | 245 | 2 | service, types |
| domain | recording | 467 | 2 | service, types |
| domain | relation_lookup | 37 | 2 | service |
| domain | render | 577 | 3 | service, types |
| domain | reporting | 64 | 2 | service, types |
| domain | repository | 138 | 3 | service, types |
| domain | rules | 57 | 1 | service |
| domain | storing | 543 | 2 | service, types |
| domain | value_cells | 134 | 3 | service, types |
| domain | visit | 351 | 4 | service, types |
| domain | workers | 468 | 3 | service, types |
| inbound | use_cases/describe_operator | 123 | 5 | interface, types, use_case |
| inbound | use_cases/describe_relation | 182 | 5 | interface, types, use_case |
| inbound | use_cases/lint | 2749 | 7 | interface, service, types, use_case |
| inbound | use_cases/list_operators | 104 | 5 | interface, types, use_case |
| inbound | use_cases/list_relations | 122 | 5 | interface, types, use_case |
| inbound | use_cases/open | 170 | 5 | interface, types, use_case |
| inbound | use_cases/validate | 214 | 5 | interface, types, use_case |
| outbound | corpus | 133 | 7 | interface, types |
| outbound | corpus/adapters/ignore | 416 | 3 | adapter, types |
| outbound | corpus/adapters/memory | 171 | 2 | adapter |
| outbound | digest/adapters/blake3 | 46 | 1 | adapter |
| outbound | facts/adapters/memory | 192 | 3 | adapter, types |
| outbound | facts/adapters/spill | 411 | 3 | adapter, types |
| outbound | language | 325 | 5 | interface, types |
| outbound | language/adapters | 7 | 1 | use_case |
| outbound | language/adapters/use_case | 360 | 2 | adapter |
| outbound | lint_store | 214 | 8 | interface, types |
| outbound | lint_store/adapters/discard | 49 | 2 | adapter, types |
| outbound | lint_store/adapters/memory | 130 | 3 | adapter, types |
| outbound | lint_store/adapters/postcard | 3151 | 8 | adapter, types |
| outbound | matching/adapters/globset | 70 | 1 | adapter |
| outbound | memory | 67 | 5 | interface |
| outbound | memory/adapters/sysinfo | 58 | 1 | adapter |
| outbound | publication | 35 | 4 | interface |
| outbound | publication/adapters/execution | 43 | 1 | adapter |
| outbound | query_language/adapters | 7 | 1 | use_case |
| outbound | query_language/adapters/use_case | 263 | 2 | adapter, types |
| outbound | row_sink | 120 | 6 | interface, types |
| outbound | row_sink/adapters/collect | 51 | 1 | adapter |
| outbound | scheduler | 122 | 7 | interface, types |
| outbound | scheduler/adapters/rayon | 74 | 1 | adapter |
| outbound | scheduler/adapters/sequential | 25 | 1 | adapter |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 68 |
| outbound | 132 |
