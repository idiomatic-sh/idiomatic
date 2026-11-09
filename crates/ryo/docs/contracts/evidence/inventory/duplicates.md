# Duplicates

## Public types and traits defined in more than one crate

A name written `pub struct`, `pub enum`, `pub trait` or `pub type` in two or more crates, with every concept that defines it.

| name | crates | kind | locations |
|---|---|---|---|
| BoundObservationInput | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| ByteOffset | sdk, values | struct | sdk:domain/query; values:domain/query_tokens |
| Cell | sdk, values | enum | sdk:domain/cell; values:domain/row_texts |
| ColumnName | sdk, values | struct | sdk:domain/relation; values:domain/descriptions |
| Emit | execution_domain, values | struct/trait | execution_domain:domain/operator; values:domain/plans |
| Example | sdk, values | struct | sdk:domain/operator; values:domain/descriptions |
| FlagState | sdk, values | enum | sdk:domain/parameter; values:domain/plan_parts |
| Function | relations, values | enum/struct | relations:domain/relations/collections/relations; values:domain/plan_parts |
| GivenProjectInventory | language, sdk | struct | language:domain/project_context; sdk:domain/project |
| Holds | relations, sdk | struct | relations:domain/labels; sdk:domain/relation |
| HostColumn | sdk, values | struct | sdk:domain/host; values:domain/hosted |
| HostColumnName | sdk, values | struct | sdk:domain/host; values:domain/hosted |
| HostKind | sdk, values | enum | sdk:domain/host; values:domain/hosted |
| LanguageBuild | core, sdk | struct | core:outbound/language; sdk:domain/language |
| LanguageName | sdk, values | struct | sdk:domain/language; values:domain/descriptions |
| MatchingPort | execution, operators | trait | execution:outbound/matching; operators:outbound/matching |
| ObservationBindingName | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| ObservationColumn | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| ObservationColumnName | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| ObservationName | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| OperatorName | sdk, values | struct | sdk:domain/operator; values:domain/descriptions |
| ParameterCell | core, sdk | struct | core:inbound/use_cases/lint; sdk:domain/parameter |
| ParameterName | sdk, values | struct | sdk:domain/parameter; values:domain/plan_parts |
| ParameterTable | core, sdk | struct | core:inbound/use_cases/lint; sdk:domain/parameter |
| ParameterValue | core, sdk | enum | core:inbound/use_cases/lint; sdk:domain/parameter |
| ProjectDependency | core, sdk | struct | core:inbound/use_cases/lint; sdk:domain/project |
| ProjectDependencyState | core, sdk | enum | core:inbound/use_cases/lint; sdk:domain/project |
| ProjectInventoryCompleteness | language, sdk | enum | language:domain/project_context; sdk:domain/project |
| ProjectSearchScope | language, sdk | enum | language:domain/project_context; sdk:domain/project |
| Purpose | sdk, values | struct | sdk:domain/operator; values:domain/descriptions |
| QueryOrigin | sdk, values | struct | sdk:domain/query; values:domain/query_tokens |
| QuerySpan | sdk, values | struct | sdk:domain/query; values:domain/query_tokens |
| QueryTraceSite | sdk, values | struct | sdk:domain/query; values:domain/query_tokens |
| ReadFailure | sdk, values | struct | sdk:domain/file; values:domain/stored_facts |
| RelationName | sdk, values | struct | sdk:domain/relation; values:domain/descriptions |
| Rows | execution_domain, relations | enum/trait | execution_domain:domain/table_model; relations:domain/relations |
| Sealed | execution_domain, relations | trait | execution_domain:domain/matching; relations:domain/relations/collections/relations |
| SourceLanguage | execution_runtime, sdk | enum | execution_runtime:application/file_dag/program/file_session; sdk:domain/language |
| SyntaxError | query_language, relations | enum/struct | query_language:outbound/syntax; relations:domain/relations/collections/relations |
| Unread | sdk, values | struct | sdk:domain/file; values:domain/records |
| Unreadable | sdk, values | enum | sdk:domain/file; values:domain/records |
| ValidateError | core, sdk | enum | core:inbound/use_cases/validate; sdk:domain/query |

## Public functions defined in more than one concept of the same layer

A name written `pub fn` in two or more concepts of one layer, across all crates.

| name | layer | concepts | locations |
|---|---|---|---|
| local | outbound | 5 | benchmark:outbound/context; benchmark:outbound/engine; benchmark:outbound/inventory; benchmark:outbound/receipts; benchmark:outbound/suite |
| use_case | outbound | 5 | core:outbound/language; execution:outbound/query_language; execution_runtime:outbound/operators; execution_store:outbound/derived; operators:outbound/matching |
| admit | application | 3 | benchmark:application/certified_measurement; benchmark:application/input_metadata; benchmark:application/registered_certification |
| prepare | application | 3 | benchmark:application/worker_request; execution_runtime:application/file_dag/program/prepared_execution; execution_store:application/file_dag/closure_partitions |
| describe | domain | 3 | derived:domain/footprint; execution_domain:domain/operator; values:domain/descriptions |
| kind_of | domain | 3 | execution_domain:domain/derived_views; values:domain/hosted; values:domain/plans |
| memory | outbound | 3 | core:outbound/corpus; core:outbound/lint_store; execution_runtime:outbound/facts |
| capacities | domain | 2 | derived:domain/closure_adjacency; derived:domain/equality |
| column_in | domain | 2 | values:domain/plan_parts; values:domain/rows |
| compile | domain | 2 | execution_domain:domain/executable_program; execution_domain:domain/executable_program/file_demand |
| digest | domain | 2 | benchmark:domain/input_metadata; benchmark:domain/project_inputs |
| identity | domain | 2 | values:domain/hosted; values:domain/observations |
| kind | domain | 2 | derived:domain/preparation; execution_domain:domain/table_model |
| positions | domain | 2 | language:domain/elements; values:domain/rows |
| prepare | domain | 2 | derived:domain/preparation; execution_domain:domain/file_program |
| registered | domain | 2 | execution_domain:domain/operator/contract; execution_domain:domain/table_model |
| unknown_message | domain | 2 | execution_domain:domain/coverage; execution_domain:domain/kind_check |
| validate | domain | 2 | execution_domain:domain/derived_views; execution_domain:domain/operator/contract |
| json | outbound | 2 | benchmark:outbound/collection; benchmark:outbound/worker |

## Function names declared in more than one crate, any visibility

A free function name declared in two or more crates whatever its visibility, so a private copy of another crate's function shows. Methods are not listed.

| name | crates | locations |
|---|---|---|
| answers | core, derived, execution, language, matching, operators, query_language | core:inbound/use_cases/describe_operator; core:inbound/use_cases/describe_relation; core:inbound/use_cases/lint; core:inbound/use_cases/list_operators; core:inbound/use_cases/list_relations; core:inbound/use_cases/open; core:inbound/use_cases/validate; derived:inbound/use_cases/build; derived:inbound/use_cases/prepare_derived; derived:inbound/use_cases/view; execution:inbound/use_cases/compile_rules; execution:inbound/use_cases/execution; execution:inbound/use_cases/metadata; execution:inbound/use_cases/prepare; execution:inbound/use_cases/publication; language:inbound/use_cases/parse; language:inbound/use_cases/project; matching:inbound/use_cases/matching; operators:inbound/use_cases/operators; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/describe; query_language:inbound/use_cases/typecheck |
| prepare | benchmark, core, derived, execution_domain, execution_runtime, execution_store, language | benchmark:application/case_execution; benchmark:application/worker_request; benchmark:inbound/use_cases/certify; benchmark:inbound/use_cases/prepare_cases; benchmark:outbound/inventory/adapters/local; core:domain/reading; derived:domain/preparation; execution_domain:domain/file_program; execution_domain:domain/operator/contract; execution_runtime:application/file_dag; execution_runtime:application/file_dag/frontiers; execution_runtime:application/file_dag/invocation; execution_runtime:application/file_dag/program/preparation; execution_runtime:application/file_dag/program/prepared_execution; execution_store:application/file_dag/closure_partitions; language:application/completion |
| runs | core, derived, execution, language, matching, operators, query_language | core:inbound/use_cases/describe_operator; core:inbound/use_cases/describe_relation; core:inbound/use_cases/lint; core:inbound/use_cases/list_operators; core:inbound/use_cases/list_relations; core:inbound/use_cases/open; core:inbound/use_cases/validate; derived:inbound/use_cases/build; derived:inbound/use_cases/prepare_derived; derived:inbound/use_cases/view; execution:inbound/use_cases/compile_rules; execution:inbound/use_cases/execution; execution:inbound/use_cases/metadata; execution:inbound/use_cases/prepare; execution:inbound/use_cases/publication; language:inbound/use_cases/parse; language:inbound/use_cases/project; matching:inbound/use_cases/matching; operators:inbound/use_cases/operators; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/describe; query_language:inbound/use_cases/typecheck |
| given_input | core, derived, language, matching, operators, query_language | core:inbound/use_cases/describe_operator; core:inbound/use_cases/describe_relation; core:inbound/use_cases/lint; core:inbound/use_cases/list_operators; core:inbound/use_cases/list_relations; core:inbound/use_cases/open; core:inbound/use_cases/validate; derived:inbound/use_cases/build; derived:inbound/use_cases/prepare_derived; derived:inbound/use_cases/view; language:inbound/use_cases/parse; language:inbound/use_cases/project; matching:inbound/use_cases/matching; operators:inbound/use_cases/operators; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/describe; query_language:inbound/use_cases/typecheck |
| matches | benchmark, execution_domain, execution_store, matching, operators | benchmark:application/process_discrepancy; execution_domain:domain/matching; execution_store:application/file_dag/relations; matching:domain/globbing; operators:domain/capture |
| use_case | core, execution, execution_runtime, execution_store, operators | core:outbound/language; execution:outbound/query_language; execution_runtime:outbound/operators; execution_store:outbound/derived; operators:outbound/matching |
| begin | benchmark, core, execution_domain, execution_runtime | benchmark:outbound/inventory/adapters/local; core:domain/reading; execution_domain:domain/operator/contract; execution_runtime:application/file_dag/program/request_execution |
| describe | derived, execution_domain, execution_store, values | derived:domain/footprint; execution_domain:domain/operator; execution_store:application/file_dag/relations; values:domain/descriptions |
| observed | benchmark, core, execution, language | benchmark:domain/process_receipts; benchmark:outbound/suite/adapters/local; core:outbound/corpus/adapters/ignore; execution:application/rule_compilation; language:domain/project_context |
| project | benchmark, language, operators, query_language | benchmark:domain/candidate_rules; language:inbound/use_cases/project; operators:domain/project; query_language:domain/fusing |
| read | benchmark, core, execution_domain, language | benchmark:outbound/receipts/adapters/local; benchmark:outbound/suite/adapters/local; core:outbound/lint_store/adapters/postcard; execution_domain:domain/executable_program/presence; execution_domain:domain/numeric_values; language:inbound/use_cases/parse |
| refused | benchmark, execution, execution_domain, execution_runtime | benchmark:application/discrepancy_attachment; benchmark:application/discrepancy_validation; benchmark:application/inventory_verification; benchmark:domain/registered_inputs; execution:outbound/query_language/adapters/use_case; execution_domain:domain/finish; execution_runtime:outbound/facts/adapters/memory; execution_runtime:outbound/facts/adapters/spill |
| admit | benchmark, execution_runtime, language | benchmark:application/certified_measurement; benchmark:application/input_metadata; benchmark:application/registered_certification; benchmark:inbound/use_cases/admit_measurement; execution_runtime:application/file_dag/frontiers; language:domain/semantics |
| build | derived, execution_store, language | derived:domain/closure_adjacency; derived:domain/cycle_adjacency; derived:domain/dfs; derived:domain/dispatch; derived:domain/equality; derived:domain/membership; derived:domain/ordered; execution_store:application/file_dag/derived/owner/acquisition/semantics; language:inbound/use_cases/parse |
| cell | derived, operators, values | derived:domain/cycle_adjacency; derived:domain/ordered; operators:domain/reduction; values:domain/rows |
| check | benchmark, derived, operators | benchmark:application/worker_identity; benchmark:domain/rule_context; benchmark:inbound/use_cases/check_worker_request; derived:domain/dfs; derived:domain/preparation; operators:domain/reduction |
| classify | benchmark, execution_domain, execution_runtime | benchmark:application/worker_request; execution_domain:domain/executable_program/fusion; execution_runtime:application/file_dag; execution_runtime:application/file_dag/program/facade_runtime |
| clear | core, execution_domain, language | core:domain/ordering; execution_domain:domain/placement; language:domain/diagnostics |
| count | benchmark, core, language | benchmark:outbound/evidence/adapters/bmf; core:domain/repository; language:domain/diagnostics |
| evaluate | execution_domain, execution_runtime, operators | execution_domain:domain/operator/contract; execution_domain:domain/shared_walk; execution_runtime:application/file_dag; operators:domain/aggregate; operators:domain/append; operators:domain/axis; operators:domain/capture; operators:domain/closure; operators:domain/cycles; operators:domain/equality_join; operators:domain/evaluation; operators:domain/fits_rendered; operators:domain/ordered_dfs; operators:domain/ordered_match; operators:domain/project; operators:domain/rank; operators:domain/render_batch; operators:domain/replace; operators:domain/select; operators:domain/set_rows; operators:domain/source_matches; operators:domain/table_rows; operators:domain/words |
| finish | execution_domain, execution_runtime, operators | execution_domain:domain/shared_walk; execution_runtime:outbound/digest/adapters/blake3; operators:domain/reduction |
| identity | core, execution_store, values | core:application/project_context; execution_store:application/file_dag/route_counts; values:domain/hosted; values:domain/observations |
| incomplete | benchmark, execution_domain, execution_runtime | benchmark:inbound/use_cases/audit; execution_domain:domain/source_partitions; execution_runtime:application/file_dag/observations |
| invalid | core, execution, query_language | core:outbound/lint_store/adapters/postcard; execution:outbound/query_language/adapters/use_case; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/typecheck |
| joined | execution_domain, operators, values | execution_domain:domain/operation_schema; operators:domain/equality_join; values:domain/lineage |
| link | execution_domain, execution_store, query_language | execution_domain:domain/file_program; execution_store:application/file_dag/derived/owner; query_language:domain/expanding |
| materialize | benchmark, derived, execution_domain | benchmark:outbound/context/adapters/local; benchmark:outbound/suite/adapters/local; derived:domain/dfs; execution_domain:domain/table_model |
| named | core, execution, values | core:outbound/lint_store/adapters/postcard; execution:application/rule_compilation; values:domain/plans |
| number | execution_store, language, query_language | execution_store:application/file_dag/route_counts; language:domain/elements; query_language:domain/idioms; query_language:outbound/syntax/adapters/winnow |
| position | benchmark, execution, query_language | benchmark:domain/diagnostics; benchmark:inbound/use_cases/compare_corpus; execution:application/rule_compilation; query_language:domain/resolving |
| positions | language, operators, values | language:domain/elements; operators:domain/equality_join; operators:domain/set_rows; values:domain/rows |
| push | language, operators, query_language | language:domain/builders; operators:domain/reduction; query_language:domain/planning |
| request | benchmark, execution, execution_domain | benchmark:inbound/use_cases/measure_corpus; benchmark:outbound/collection/adapters/json; benchmark:outbound/execution/adapters/timed; execution:inbound/use_cases/execution; execution:inbound/use_cases/metadata; execution:inbound/use_cases/prepare; execution:inbound/use_cases/publication; execution_domain:domain/file_program |
| resolve | benchmark, core, execution_domain | benchmark:domain/rule_context; benchmark:outbound/context/adapters/local; core:domain/relation_lookup; execution_domain:domain/executable_program; execution_domain:domain/operands |
| rule | benchmark, execution, execution_runtime | benchmark:outbound/suite/adapters/local; execution:inbound/use_cases/compile_rules; execution_runtime:application/file_dag/program/facade_runtime |
| same | execution_domain, execution_store, operators | execution_domain:domain/comparing/collections/comparisons; execution_store:application/file_dag/relations; operators:domain/reduction |
| select | benchmark, operators, query_language | benchmark:application/case_selection; benchmark:inbound/use_cases/compare_corpus; benchmark:inbound/use_cases/select_cases; operators:domain/select; query_language:outbound/syntax/adapters/winnow |
| source | execution_runtime, execution_store, query_language | execution_runtime:application/file_dag/program/preparation/service/metadata; execution_store:application/file_dag/derived/owner/acquisition/graph/identity; query_language:outbound/syntax/adapters/winnow |
| source_column | derived, execution_domain, query_language | derived:domain/native; execution_domain:domain/table_cells; query_language:domain/fusing |
| table | derived, execution_domain, execution_store | derived:domain/footprint; execution_domain:domain/executable_program/file_demand; execution_store:application/file_dag/derived/ledger |
| text | benchmark, core, values | benchmark:outbound/receipts/adapters/local; core:outbound/lint_store/adapters/postcard; values:domain/hosted; values:domain/observations |
| validate | benchmark, core, execution_domain | benchmark:application/discrepancy_validation; benchmark:application/receipt_validation; benchmark:domain/inventory_receipts; benchmark:domain/process_receipts; benchmark:outbound/context/adapters/local; benchmark:outbound/suite/adapters/local; core:outbound/lint_store/adapters/postcard; execution_domain:domain/derived_views; execution_domain:domain/operator/contract |
| absorb | core, execution_domain | core:domain/workers; execution_domain:domain/trust |
| agree | benchmark, core | benchmark:inbound/use_cases/measure_corpus; core:outbound/corpus/adapters/ignore |
| all | execution_domain, relations | execution_domain:domain/executable_program/presence; relations:domain/relations |
| append | core, operators | core:domain/ordering; core:outbound/lint_store/adapters/postcard; operators:domain/append |
| bind | core, values | core:application/cross_file; values:domain/rule_requests |
| candidate | benchmark, query_language | benchmark:application/case_execution; query_language:domain/fusing |
| canonical | benchmark, execution_domain | benchmark:outbound/inventory/adapters/local; benchmark:outbound/receipts/adapters/local; execution_domain:domain/executable_program |
| capture | benchmark, execution_store | benchmark:outbound/inventory/adapters/local; execution_store:application/file_dag/route_counts |
| changed | benchmark, core | benchmark:inbound/use_cases/certify; benchmark:inbound/use_cases/measure; core:outbound/corpus/adapters/ignore |
| check_columns | derived, execution_domain | derived:domain/adjacency; derived:domain/cycle_adjacency; derived:domain/ordered; derived:domain/partition; execution_domain:domain/table_model |
| compare | benchmark, execution_domain | benchmark:domain/diagnostics; benchmark:domain/inventory_comparison; benchmark:inbound/use_cases/compare; benchmark:inbound/use_cases/compare_corpus; execution_domain:domain/numeric_values |
| comparison | benchmark, execution | benchmark:outbound/evidence/adapters/bmf; execution:application/rule_compilation |
| comparisons_of | query_language, values | query_language:domain/selecting; values:domain/plans |
| compile | execution_domain, matching | execution_domain:domain/executable_program; execution_domain:domain/executable_program/file_demand; execution_domain:domain/matching; execution_domain:domain/self_join_recogniser; matching:domain/globbing |
| compile_regex | execution_domain, matching | execution_domain:domain/matching; matching:domain/matching |
| complete | benchmark, language | benchmark:domain/discrepancy; benchmark:inbound/use_cases/audit; benchmark:inbound/use_cases/prepare_cases; benchmark:inbound/use_cases/publish; benchmark:inbound/use_cases/validate_case_receipts; benchmark:inbound/use_cases/verify_cases; benchmark:inbound/use_cases/verify_reference_cases; language:application/completion |
| count_groups | execution_domain, operators | execution_domain:domain/table_groups; operators:domain/reduction |
| decide | core, execution_domain | core:domain/storing; execution_domain:domain/placement |
| deliver | core, operators | core:application/cross_file; operators:domain/render_batch |
| derived | benchmark, execution_store | benchmark:domain/inventory_receipts; execution_store:application/file_dag/route_counts |
| differences | benchmark, core | benchmark:inbound/use_cases/compare_corpus; core:domain/allocation_verdict |
| digest_groups | execution_domain, operators | execution_domain:domain/table_groups; operators:domain/reduction |
| direction | execution_domain, query_language | execution_domain:domain/stored_graph; query_language:domain/rewriting |
| document | benchmark, execution_store | benchmark:domain/project_inputs; execution_store:application/file_dag/route_counts |
| equivalent | benchmark, execution_domain | benchmark:inbound/use_cases/compare; execution_domain:domain/executable_program |
| executable | benchmark, core | benchmark:outbound/engine/adapters/local; core:outbound/corpus/adapters/ignore |
| expand | benchmark, query_language | benchmark:domain/process_output; query_language:domain/expanding |
| expected | benchmark, execution_runtime | benchmark:domain/process_failure; execution_runtime:application/file_dag/program/facade_runtime |
| file_root | core, execution_domain | core:outbound/corpus/adapters/ignore; execution_domain:domain/executable_program/file_demand |
| find_or_add | execution_domain, language | execution_domain:domain/table_dedup; language:domain/symbols |
| finding | benchmark, derived | benchmark:application/discrepancy_attachment; derived:domain/native |
| flush | execution_runtime, execution_store | execution_runtime:outbound/facts/adapters/spill; execution_store:application/file_dag/route_counts/progress |
| frame | benchmark, core | benchmark:outbound/context/adapters/local; core:domain/repository |
| hash_of | language, operators | language:domain/symbols; operators:domain/reduction |
| hosted | operators, values | operators:domain/evaluation; values:domain/lineage |
| inputs_of | query_language, values | query_language:domain/planning; values:domain/plans |
| keyed | operators, query_language | operators:domain/equality_join; query_language:domain/planning |
| kind | derived, execution_domain | derived:domain/preparation; execution_domain:domain/table_model |
| kind_of | execution_domain, values | execution_domain:domain/derived_views; values:domain/hosted; values:domain/plans |
| line | language, query_language | language:domain/location; query_language:outbound/syntax/adapters/winnow |
| listed | core, language | core:outbound/corpus/adapters/ignore; language:domain/diagnostics |
| matching | benchmark, execution_domain | benchmark:inbound/use_cases/measure; execution_domain:domain/table_model |
| measured | benchmark, execution_store | benchmark:inbound/use_cases/measure_corpus; execution_store:application/file_dag/derived/ledger |
| memory | core, execution_runtime | core:outbound/corpus; core:outbound/lint_store; execution_runtime:outbound/facts |
| moved | core, execution_domain | core:domain/keeping; execution_domain:domain/completion |
| narrowed | benchmark, query_language | benchmark:outbound/suite/adapters/local; query_language:domain/narrowing |
| normalize | benchmark, language | benchmark:domain/invocation_identity; language:domain/targets |
| number_of | execution_domain, relations | execution_domain:domain/comparing/collections/comparisons; relations:domain/relations |
| offset | benchmark, execution | benchmark:domain/discrepancy; execution:outbound/query_language/adapters/use_case |
| operand | execution_domain, query_language | execution_domain:domain/operator/contract; query_language:outbound/syntax/adapters/winnow |
| owner_of | core, relations | core:domain/run_rules; relations:domain/terms |
| place | execution_domain, language | execution_domain:domain/placement; language:domain/builders |
| projected | execution_domain, query_language | execution_domain:domain/operation_schema; query_language:domain/selecting |
| pushed | language, query_language | language:domain/builders; query_language:domain/planning; query_language:domain/pushing |
| quote | benchmark, execution_store | benchmark:domain/canonical_text; execution_store:application/file_dag/route_counts |
| reads | benchmark, language | benchmark:inbound/use_cases/select_cases; language:domain/answers |
| receipt | benchmark, language | benchmark:outbound/context/adapters/local; language:domain/project_context |
| record | benchmark, language | benchmark:outbound/receipts/adapters/local; language:domain/diagnostics |
| refuses | benchmark, execution_runtime | benchmark:inbound/use_cases/account_defaults; benchmark:inbound/use_cases/admit_measurement; benchmark:inbound/use_cases/check_worker_request; benchmark:inbound/use_cases/collect; benchmark:inbound/use_cases/measure_corpus; benchmark:inbound/use_cases/run_ryo; benchmark:inbound/use_cases/select_cases; execution_runtime:application/file_dag/program/facade_runtime |
| remapped | execution_domain, values | execution_domain:domain/completion; values:domain/plan_parts |
| replace | execution_domain, execution_store | execution_domain:domain/describing; execution_store:application/file_dag/derived/owner/graphs |
| replay | core, execution_runtime | core:domain/handing; execution_runtime:application/file_dag |
| row | benchmark, operators | benchmark:application/inventory_verification; benchmark:outbound/inventory/adapters/local; operators:domain/reduction |
| rows | benchmark, operators | benchmark:outbound/receipts/adapters/local; operators:domain/table_rows |
| rules | execution, execution_runtime | execution:inbound/use_cases/compile_rules; execution_runtime:application/file_dag/program/preparation/service/metadata |
| semantics | execution_domain, execution_store | execution_domain:domain/operator/contract; execution_store:outbound/derived |
| size_at | core, language | core:domain/workers; language:domain/elements |
| source_id | query_language, values | query_language:domain/linking; values:domain/program |
| stopped | core, execution_runtime | core:application/cross_file; core:application/file_analysis/evaluation; execution_runtime:application/file_dag/program/request_execution |
| stored_cell | derived, execution_domain | derived:domain/equality; execution_domain:domain/table_model |
| succeeds | benchmark, execution_runtime | benchmark:inbound/use_cases/collect; benchmark:inbound/use_cases/run_ryo; execution_runtime:application/file_dag/program/facade_runtime |
| summary | benchmark, relations | benchmark:application/inventory_verification; benchmark:outbound/inventory/adapters/local; relations:domain/relations/collections/relations |
| tally | execution_domain, values | execution_domain:domain/features; values:domain/hosted; values:domain/observations |
| test | execution_domain, query_language | execution_domain:domain/executable_program/presence; query_language:outbound/syntax/adapters/winnow |
| unreadable | core, query_language | core:outbound/corpus/adapters/ignore; query_language:outbound/syntax/adapters/winnow |
| value | benchmark, query_language | benchmark:outbound/inventory/adapters/local; query_language:outbound/syntax/adapters/winnow |
| vector | derived, execution_store | derived:domain/footprint; execution_store:application/file_dag/derived/ledger |
| view | derived, execution_store | derived:domain/dispatch; execution_store:application/file_dag; execution_store:application/file_dag/derived/owner |
| word_of | derived, query_language | derived:domain/key; query_language:domain/resolving |
| words_of | query_language, relations | query_language:domain/resolving; relations:domain/terms |
