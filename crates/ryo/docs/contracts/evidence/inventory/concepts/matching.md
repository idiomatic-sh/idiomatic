# Concepts: matching

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | globbing | 510 | 3 | service, types |
| domain | matching | 158 | 2 | service, types |
| inbound | use_cases/matching | 334 | 6 | interface, types, use_case |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 12 |
