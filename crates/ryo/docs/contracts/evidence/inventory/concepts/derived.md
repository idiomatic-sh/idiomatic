# Concepts: derived

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | adjacency | 273 | 6 | service, types |
| domain | allocation | 13 | 1 | types |
| domain | closure_adjacency | 101 | 2 | service, types |
| domain | contract | 225 | 5 | types |
| domain | cycle_adjacency | 102 | 2 | service, types |
| domain | dfs | 260 | 2 | service, types |
| domain | dispatch | 222 | 1 | service |
| domain | equality | 138 | 2 | service, types |
| domain | fault | 33 | 1 | types |
| domain | footprint | 245 | 2 | service, types |
| domain | key | 76 | 2 | service, types |
| domain | membership | 90 | 2 | service, types |
| domain | native | 67 | 2 | service, types |
| domain | ordered | 129 | 2 | service, types |
| domain | partition | 117 | 2 | service, types |
| domain | positions | 5 | 1 | types |
| domain | preparation | 102 | 1 | service |
| domain | request | 108 | 1 | types |
| domain | row_cells | 86 | 1 | types |
| domain | sharing | 32 | 2 | service, types |
| domain | vertices | 52 | 2 | types |
| inbound | use_cases/build | 124 | 3 | interface, use_case |
| inbound | use_cases/prepare_derived | 102 | 3 | interface, use_case |
| inbound | use_cases/view | 159 | 3 | interface, use_case |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 21 |
