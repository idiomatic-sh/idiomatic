# Concepts: execution

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | publication | 42 | 3 | service, types |
| application | rule_compilation | 930 | 7 | service, types |
| inbound | use_cases/compile_rules | 187 | 4 | interface, types, use_case |
| inbound | use_cases/execution | 159 | 3 | interface, use_case |
| inbound | use_cases/metadata | 307 | 4 | interface, types, use_case |
| inbound | use_cases/prepare | 151 | 4 | interface, types, use_case |
| inbound | use_cases/publication | 93 | 3 | interface, use_case |
| outbound | matching | 43 | 4 | interface |
| outbound | matching/adapters/globset | 66 | 1 | adapter |
| outbound | query_language | 89 | 4 | interface |
| outbound | query_language/adapters | 7 | 1 | use_case |
| outbound | query_language/adapters/use_case | 287 | 2 | adapter, types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 34 |
