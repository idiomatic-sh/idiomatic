# Concepts: relations

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | answers | 60 | 3 | service, types |
| domain | capabilities | 66 | 1 | types |
| domain | catalog | 150 | 1 | service |
| domain | declaration_kind | 67 | 1 | types |
| domain | dependency_kind | 29 | 1 | types |
| domain | dependency_origin | 33 | 1 | types |
| domain | elements | 62 | 1 | types |
| domain | kinds | 103 | 1 | service |
| domain | labels | 61 | 1 | types |
| domain | mapping_kind | 31 | 1 | types |
| domain | parameter_kind | 35 | 1 | types |
| domain | relations | 1089 | 5 | service, types |
| domain | relations/collections/relations | 5663 | 208 | interface |
| domain | terms | 262 | 3 | service, types |
| domain | value_kind | 41 | 1 | types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
