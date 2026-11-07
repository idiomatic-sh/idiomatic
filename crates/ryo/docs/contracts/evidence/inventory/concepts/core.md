# Concepts: core

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | cross_file | 665 | 5 | service, types |
| application | cross_file/completion | 321 | 4 | service, types |
| application | cross_file/ordinary | 122 | 2 | service |
| application | cross_file/phase | 190 | 2 | service |
| application | cross_file/preparation | 203 | 4 | service, types |
| application | cross_file/rendering | 330 | 2 | service |
| application | engine | 151 | 2 | service, types |
| application | file_analysis | 476 | 4 | service, types |
| application | file_analysis/evaluation | 301 | 2 | service |
| application | file_analysis/gating | 175 | 2 | service |
| application | file_analysis/outcome | 118 | 2 | service |
| application | file_analysis/parsing | 380 | 2 | service |
| application | file_analysis/project | 238 | 2 | service |
| application | file_analysis/replaying | 285 | 2 | service |
| application | file_analysis/semantic | 108 | 2 | service |
| application | file_analysis/streaming | 334 | 2 | service |
| application | metadata | 11 | 1 | service |
| application | persisting | 133 | 2 | service, types |
| application | project_context | 160 | 2 | service |
| application | storing | 404 | 4 | service, types |
| domain | admission | 182 | 3 | service, types |
| domain | allocation_verdict | 114 | 3 | service, types |
| domain | answering | 118 | 2 | service, types |
| domain | budget | 79 | 2 | service |
| domain | choosing | 31 | 1 | service |
| domain | corpus_copies | 111 | 3 | service, types |
| domain | crossing | 211 | 2 | service, types |
| domain | finish | 361 | 4 | service, types |
| domain | handing | 304 | 1 | service |
| domain | keeping | 392 | 2 | service, types |
| domain | listing | 40 | 1 | service |
| domain | memory_diagnostics | 176 | 3 | service, types |
| domain | ordering | 271 | 3 | service, types |
| domain | project | 133 | 2 | service, types |
| domain | reading | 246 | 2 | service, types |
| domain | recording | 407 | 3 | service, types |
| domain | relation_lookup | 37 | 2 | service |
| domain | render | 577 | 3 | service, types |
| domain | reporting | 165 | 2 | service, types |
| domain | repository | 139 | 3 | service, types |
| domain | run_rules | 56 | 1 | service |
| domain | storing | 479 | 2 | service, types |
| domain | value_cells | 134 | 3 | service, types |
| domain | visit | 351 | 4 | service, types |
| domain | workers | 519 | 3 | service, types |
| inbound | use_cases/describe_operator | 123 | 5 | interface, types, use_case |
| inbound | use_cases/describe_relation | 182 | 5 | interface, types, use_case |
| inbound | use_cases/lint | 1641 | 7 | interface, service, types, use_case |
| inbound | use_cases/list_operators | 104 | 5 | interface, types, use_case |
| inbound | use_cases/list_relations | 122 | 5 | interface, types, use_case |
| inbound | use_cases/open | 170 | 5 | interface, types, use_case |
| inbound | use_cases/validate | 221 | 5 | interface, types, use_case |
| outbound | corpus | 133 | 7 | interface, types |
| outbound | corpus/adapters/ignore | 416 | 3 | adapter, types |
| outbound | corpus/adapters/memory | 171 | 2 | adapter |
| outbound | language | 303 | 5 | interface, types |
| outbound | language/adapters | 8 | 1 | use_case |
| outbound | language/adapters/use_case | 385 | 3 | adapter |
| outbound | lint_store | 214 | 8 | interface, types |
| outbound | lint_store/adapters/discard | 79 | 2 | adapter, types |
| outbound | lint_store/adapters/memory | 160 | 3 | adapter, types |
| outbound | lint_store/adapters/postcard | 3423 | 20 | adapter, types |
| outbound | memory | 67 | 5 | interface |
| outbound | memory/adapters/sysinfo | 58 | 1 | adapter |
| outbound | row_sink | 117 | 6 | interface, types |
| outbound | row_sink/adapters/collect | 68 | 2 | adapter |
| outbound | scheduler | 94 | 7 | interface, types |
| outbound | scheduler/adapters/rayon | 90 | 1 | adapter |
| outbound | scheduler/adapters/sequential | 25 | 1 | adapter |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 68 |
