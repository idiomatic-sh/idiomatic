# Concepts: relations

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | answers | 58 | 3 | service, types |
| domain | kinds | 103 | 1 | service |
| domain | relations | 1414 | 5 | service, types |
| domain | relations/collections/relations | 5657 | 208 | interface |
| domain | terms | 511 | 3 | service, types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
