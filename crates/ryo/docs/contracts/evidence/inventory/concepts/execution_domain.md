# Concepts: execution_domain

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | matching | 31 | 1 | service |
| domain | comparing | 362 | 4 | service, types |
| domain | comparing/collections/comparisons | 604 | 15 | interface |
| domain | completion | 456 | 4 | service, types |
| domain | contributor_rows | 45 | 1 | service |
| domain | coverage | 452 | 3 | service, types |
| domain | derived_views | 286 | 4 | service, types |
| domain | executable_program | 1798 | 16 | service, types |
| domain | executable_program/file_demand | 415 | 3 | service, types |
| domain | executable_program/fusion | 280 | 3 | service, types |
| domain | executable_program/presence | 706 | 4 | service, types |
| domain | features | 179 | 2 | service, types |
| domain | file_program | 1668 | 10 | service, types |
| domain | finish | 489 | 4 | service, types |
| domain | frontier_capture | 35 | 1 | types |
| domain | frontier_coverage | 106 | 2 | service |
| domain | frontiers | 66 | 2 | service, types |
| domain | functions | 35 | 2 | service, types |
| domain | hosted | 111 | 3 | service, types |
| domain | kind_check | 310 | 3 | service, types |
| domain | kind_sets | 354 | 5 | service, types |
| domain | matching | 293 | 4 | service, types |
| domain | numeric_values | 379 | 3 | service, types |
| domain | operands | 69 | 1 | service |
| domain | operating | 324 | 3 | service, types |
| domain | operation_schema | 952 | 7 | service, types |
| domain | operator | 495 | 4 | service, types |
| domain | operator/contract | 2974 | 17 | service, types |
| domain | output_coverage | 437 | 3 | service, types |
| domain | placement | 225 | 2 | service, types |
| domain | prepared_request | 54 | 2 | service, types |
| domain | readiness | 269 | 3 | types |
| domain | replay | 99 | 2 | service, types |
| domain | row_emission | 18 | 1 | types |
| domain | rule_intent | 152 | 2 | types |
| domain | rules | 21 | 2 | service |
| domain | self_join_recogniser | 411 | 2 | service, types |
| domain | shared_walk | 1178 | 9 | service, types |
| domain | source_partitions | 350 | 4 | types |
| domain | spelling | 90 | 2 | service |
| domain | step_classes | 282 | 2 | service, types |
| domain | stored_graph | 416 | 3 | types |
| domain | table_cells | 307 | 3 | service, types |
| domain | table_dedup | 115 | 2 | service, types |
| domain | table_groups | 134 | 2 | service, types |
| domain | table_model | 615 | 7 | service, types |
| domain | table_model/collections/representations | 477 | 10 | interface, types |
| domain | table_reach | 380 | 3 | service, types |
| domain | table_scratch | 194 | 3 | service, types |
| domain | table_texts | 152 | 3 | service, types |
| domain | text_gate | 28 | 1 | service |
| domain | tree_walk | 47 | 1 | service |
| domain | trust | 573 | 3 | service, types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| domain | 243 |
