# Concepts: language

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | completion | 172 | 1 | service |
| domain | answers | 371 | 2 | service, types |
| domain | builders | 389 | 2 | service, types |
| domain | completion | 156 | 1 | service |
| domain | decoded | 43 | 1 | service |
| domain | diagnostics | 248 | 3 | service, types |
| domain | elements | 758 | 3 | service, types |
| domain | expansions | 174 | 1 | service |
| domain | files | 65 | 2 | service, types |
| domain | kind_index | 61 | 1 | service |
| domain | location | 59 | 2 | service, types |
| domain | names | 168 | 2 | service, types |
| domain | notes | 60 | 2 | service, types |
| domain | project_context | 109 | 2 | service, types |
| domain | reasons | 19 | 1 | types |
| domain | relation_masks | 46 | 3 | service, types |
| domain | semantics | 114 | 3 | service, types |
| domain | symbols | 516 | 2 | service, types |
| domain | targets | 154 | 3 | service, types |
| domain | trees | 119 | 2 | service, types |
| domain | words | 140 | 1 | service |
| inbound | use_cases/parse | 1054 | 7 | interface, service, types, use_case |
| inbound | use_cases/project | 549 | 6 | interface, service, types, use_case |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 30 |
