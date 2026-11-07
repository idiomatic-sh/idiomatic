# Concepts: operators

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | aggregate | 69 | 2 | service, types |
| domain | append | 22 | 1 | service |
| domain | axis | 134 | 2 | service, types |
| domain | capture | 173 | 3 | service, types |
| domain | closure | 114 | 2 | service, types |
| domain | contract | 286 | 2 | types |
| domain | cycles | 210 | 2 | service, types |
| domain | equality_join | 430 | 3 | service, types |
| domain | evaluation | 427 | 2 | service, types |
| domain | fits_rendered | 188 | 2 | service, types |
| domain | ordered_dfs | 185 | 2 | service, types |
| domain | ordered_existence | 129 | 2 | service, types |
| domain | ordered_match | 111 | 2 | service, types |
| domain | project | 42 | 1 | service |
| domain | rank | 43 | 1 | service |
| domain | reduction | 357 | 2 | service, types |
| domain | render_batch | 157 | 2 | service, types |
| domain | replace | 85 | 2 | service, types |
| domain | select | 46 | 2 | service, types |
| domain | set_rows | 267 | 3 | service, types |
| domain | source_matches | 370 | 2 | service, types |
| domain | span_between | 44 | 1 | service |
| domain | span_bounds | 27 | 1 | service |
| domain | table_rows | 100 | 2 | service, types |
| domain | words | 100 | 2 | service, types |
| inbound | use_cases/operators | 717 | 4 | interface, types, use_case |
| outbound | matching | 44 | 3 | interface |
| outbound | matching/adapters | 6 | 1 | use_case |
| outbound | matching/adapters/use_case | 44 | 1 | adapter |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 9 |
