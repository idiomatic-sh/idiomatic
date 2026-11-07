# Concepts: operators

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | aggregate | 21 | 1 | service |
| domain | append | 9 | 1 | service |
| domain | capture | 85 | 3 | service, types |
| domain | contract | 226 | 1 | types |
| domain | equality_join | 162 | 2 | service, types |
| domain | ordered_existence | 124 | 2 | service, types |
| domain | ordered_match | 76 | 2 | service, types |
| domain | project | 23 | 1 | service |
| domain | rank | 10 | 1 | service |
| domain | reduction | 340 | 2 | service, types |
| domain | replace | 48 | 2 | service, types |
| domain | select | 39 | 2 | service, types |
| domain | set_rows | 182 | 2 | service, types |
| domain | source_matches | 246 | 2 | service, types |
| domain | span_between | 44 | 1 | service |
| domain | span_bounds | 27 | 1 | service |
| domain | words | 43 | 2 | service, types |
| inbound | use_cases/operators | 418 | 3 | interface, use_case |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 8 |
