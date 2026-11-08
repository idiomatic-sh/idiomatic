# Concepts: values

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | compact_table | 274 | 1 | service |
| domain | descriptions | 277 | 3 | service, types |
| domain | files | 176 | 3 | service, types |
| domain | hosted | 185 | 3 | service, types |
| domain | observations | 480 | 5 | service, types |
| domain | plan_parts | 378 | 2 | service, types |
| domain | plans | 543 | 2 | service, types |
| domain | program | 383 | 3 | service, types |
| domain | query_bindings | 61 | 1 | types |
| domain | query_operators | 155 | 1 | types |
| domain | query_syntax | 266 | 1 | types |
| domain | query_tokens | 245 | 2 | types |
| domain | records | 488 | 2 | service, types |
| domain | row_texts | 123 | 1 | types |
| domain | rows | 61 | 3 | service, types |
| domain | rule_requests | 161 | 3 | service, types |
| domain | run_metadata | 157 | 2 | service, types |
| domain | runs | 129 | 3 | service, types |
| domain | stored_facts | 135 | 2 | service, types |
| domain | stored_findings | 103 | 1 | types |
| domain | stores | 664 | 4 | service, types |
| domain | table_words | 492 | 4 | service, types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
