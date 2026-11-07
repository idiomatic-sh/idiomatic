# Concepts: derived

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | contract | 873 | 2 | types |
| inbound | use_cases/derived | 130 | 2 | interface, use_case |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 3 |
