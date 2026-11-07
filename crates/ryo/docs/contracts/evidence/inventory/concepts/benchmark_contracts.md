# Concepts: benchmark_contracts

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | artifacts | 138 | 7 | types |
| domain | context | 233 | 6 | types |
| domain | documents | 855 | 13 | types |
| domain | measurements | 173 | 1 | types |
| domain | process | 72 | 1 | types |
| domain | receipts | 84 | 2 | types |
| domain | reports | 236 | 4 | service, types |
| domain | requests | 364 | 4 | types |
| domain | workloads | 818 | 27 | types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
