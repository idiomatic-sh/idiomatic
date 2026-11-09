# Concepts: core

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | cross_file | 622 | 5 | service, types |
| application | cross_file/completion | 335 | 4 | service, types |
| application | cross_file/ordinary | 125 | 2 | service |
| application | cross_file/phase | 203 | 2 | service |
| application | cross_file/rendering | 330 | 2 | service |
| application | engine | 93 | 1 | service |
| application | file_analysis | 490 | 4 | service, types |
| application | file_analysis/evaluation | 304 | 2 | service |
| application | file_analysis/gating | 205 | 2 | service |
| application | file_analysis/outcome | 125 | 2 | service |
| application | file_analysis/parsing | 395 | 2 | service |
| application | file_analysis/project | 237 | 2 | service |
| application | file_analysis/replaying | 298 | 2 | service |
| application | file_analysis/semantic | 108 | 2 | service |
| application | file_analysis/streaming | 337 | 2 | service |
| application | metadata | 56 | 1 | service |
| application | persisting | 133 | 2 | service, types |
| application | project_context | 160 | 2 | service |
| application | storing | 406 | 4 | service, types |
| domain | admission | 182 | 3 | service, types |
| domain | allocation_verdict | 114 | 3 | service, types |
| domain | answering | 117 | 2 | service, types |
| domain | budget | 79 | 2 | service |
| domain | choosing | 31 | 1 | service |
| domain | corpus_copies | 111 | 3 | service, types |
| domain | crossing | 256 | 2 | service, types |
| domain | finish | 361 | 4 | service, types |
| domain | handing | 310 | 1 | service |
| domain | keeping | 392 | 2 | service, types |
| domain | listing | 40 | 1 | service |
| domain | memory_diagnostics | 177 | 3 | service, types |
| domain | ordering | 271 | 3 | service, types |
| domain | project | 133 | 2 | service, types |
| domain | reading | 247 | 2 | service, types |
| domain | recording | 410 | 3 | service, types |
| domain | relation_lookup | 37 | 2 | service |
| domain | render | 577 | 3 | service, types |
| domain | reporting | 166 | 2 | service, types |
| domain | repository | 214 | 3 | service, types |
| domain | run_rules | 56 | 1 | service |
| domain | storing | 480 | 2 | service, types |
| domain | value_cells | 134 | 3 | service, types |
| domain | visit | 352 | 4 | service, types |
| domain | workers | 556 | 3 | service, types |
| inbound | use_cases/describe_operator | 123 | 5 | interface, types, use_case |
| inbound | use_cases/describe_relation | 182 | 5 | interface, types, use_case |
| inbound | use_cases/lint | 1644 | 7 | interface, service, types, use_case |
| inbound | use_cases/list_operators | 104 | 5 | interface, types, use_case |
| inbound | use_cases/list_relations | 122 | 5 | interface, types, use_case |
| inbound | use_cases/open | 170 | 5 | interface, types, use_case |
| inbound | use_cases/validate | 221 | 5 | interface, types, use_case |
| outbound | corpus | 133 | 7 | interface, types |
| outbound | corpus/adapters/ignore | 418 | 3 | adapter, types |
| outbound | corpus/adapters/memory | 174 | 2 | adapter |
| outbound | language | 303 | 5 | interface, types |
| outbound | language/adapters | 8 | 1 | use_case |
| outbound | language/adapters/use_case | 387 | 3 | adapter |
| outbound | lint_store | 215 | 8 | interface, types |
| outbound | lint_store/adapters/discard | 82 | 2 | adapter, types |
| outbound | lint_store/adapters/memory | 163 | 3 | adapter, types |
| outbound | lint_store/adapters/postcard | 3553 | 22 | adapter, types |
| outbound | memory | 67 | 5 | interface |
| outbound | memory/adapters/sysinfo | 58 | 1 | adapter |
| outbound | row_sink | 117 | 6 | interface, types |
| outbound | row_sink/adapters/collect | 73 | 2 | adapter |
| outbound | scheduler | 92 | 7 | interface, types |
| outbound | scheduler/adapters/rayon | 90 | 1 | adapter |
| outbound | scheduler/adapters/sequential | 25 | 1 | adapter |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 68 |
