# Concepts: benchmark_contracts

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | artifacts | 129 | 6 | types |
| domain | context | 163 | 6 | types |
| domain | measurements | 148 | 1 | types |
| domain | process | 89 | 1 | types |
| domain | reports | 189 | 4 | service, types |
| domain | requests | 176 | 4 | types |
| domain | workloads | 692 | 27 | types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
