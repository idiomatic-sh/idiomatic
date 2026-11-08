# Concepts: execution_domain

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| domain | comparing | 416 | 4 | service, types |
| domain | comparing/collections/comparisons | 606 | 15 | interface |
| domain | completion | 465 | 4 | service, types |
| domain | contributor_rows | 45 | 1 | service |
| domain | coverage | 437 | 3 | service, types |
| domain | derived_views | 282 | 4 | service, types |
| domain | executable_program | 1712 | 16 | service, types |
| domain | executable_program/file_demand | 413 | 3 | service, types |
| domain | executable_program/fusion | 278 | 3 | service, types |
| domain | features | 179 | 2 | service, types |
| domain | file_program | 1690 | 10 | service, types |
| domain | finish | 483 | 4 | service, types |
| domain | frontier_capture | 35 | 1 | types |
| domain | frontier_coverage | 112 | 2 | service |
| domain | frontiers | 66 | 2 | service, types |
| domain | functions | 35 | 2 | service, types |
| domain | hosted | 111 | 3 | service, types |
| domain | kind_check | 304 | 3 | service, types |
| domain | kind_sets | 348 | 5 | service, types |
| domain | lineage | 306 | 2 | service, types |
| domain | matching | 242 | 4 | service, types |
| domain | numeric_values | 375 | 3 | service, types |
| domain | operands | 69 | 1 | service |
| domain | operating | 266 | 3 | service, types |
| domain | operation_schema | 782 | 7 | service, types |
| domain | operator | 432 | 4 | service, types |
| domain | operator/contract | 3057 | 17 | service, types |
| domain | output_coverage | 439 | 3 | service, types |
| domain | placement | 225 | 2 | service, types |
| domain | prepared_request | 54 | 2 | service, types |
| domain | readiness | 260 | 3 | types |
| domain | replay | 106 | 2 | service, types |
| domain | row_emission | 18 | 1 | types |
| domain | rule_intent | 159 | 2 | types |
| domain | rules | 21 | 2 | service |
| domain | self_join_recogniser | 403 | 2 | service, types |
| domain | shared_walk | 1174 | 9 | service, types |
| domain | source_partitions | 350 | 4 | types |
| domain | spelling | 90 | 2 | service |
| domain | step_classes | 282 | 2 | service, types |
| domain | stored_graph | 409 | 3 | types |
| domain | table_cells | 307 | 3 | service, types |
| domain | table_dedup | 115 | 2 | service, types |
| domain | table_groups | 136 | 2 | service, types |
| domain | table_model | 635 | 7 | service, types |
| domain | table_model/collections/representations | 466 | 10 | interface, types |
| domain | table_reach | 294 | 3 | service, types |
| domain | table_scratch | 195 | 3 | service, types |
| domain | table_texts | 149 | 3 | service, types |
| domain | text_gate | 28 | 1 | service |
| domain | tree_walk | 47 | 1 | service |
| domain | trust | 610 | 3 | service, types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| domain | 243 |
