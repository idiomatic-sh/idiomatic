# Duplicates

## Public types and traits defined in more than one crate

A name written `pub struct`, `pub enum`, `pub trait` or `pub type` in two or more crates, with every concept that defines it.

| name | crates | kind | locations |
|---|---|---|---|
| BoundObservationInput | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| ByteOffset | sdk, values | struct | sdk:domain/query; values:domain/query_tokens |
| Cell | sdk, values | enum | sdk:domain/cell; values:domain/row_texts |
| ColumnName | sdk, values | struct | sdk:domain/relation; values:domain/descriptions |
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
| SourceLanguage | execution, sdk | enum | execution:application/file_dag/program/file_session; sdk:domain/language |
| SyntaxError | query_language, relations | enum/struct | query_language:outbound/syntax; relations:domain/relations/collections/relations |
| Unread | sdk, values | struct | sdk:domain/file; values:domain/records |
| Unreadable | sdk, values | enum | sdk:domain/file; values:domain/records |
| ValidateError | core, sdk | enum | core:inbound/use_cases/validate; sdk:domain/query |

## Public functions defined in more than one concept of the same layer

A name written `pub fn` in two or more concepts of one layer, across all crates.

| name | layer | concepts | locations |
|---|---|---|---|
| local | outbound | 5 | benchmark:outbound/context; benchmark:outbound/engine; benchmark:outbound/inventory; benchmark:outbound/receipts; benchmark:outbound/suite |
| use_case | outbound | 5 | core:outbound/language; execution:outbound/derived; execution:outbound/operators; execution:outbound/query_language; operators:outbound/matching |
| admit | application | 3 | benchmark:application/certified_measurement; benchmark:application/input_metadata; benchmark:application/registered_certification |
| memory | outbound | 3 | core:outbound/corpus; core:outbound/lint_store; execution:outbound/facts |
| capacities | domain | 2 | derived:domain/closure_adjacency; derived:domain/equality |
| column_in | domain | 2 | values:domain/plan_parts; values:domain/rows |
| describe | domain | 2 | derived:domain/footprint; values:domain/descriptions |
| digest | domain | 2 | benchmark:domain/input_metadata; benchmark:domain/project_inputs |
| identity | domain | 2 | values:domain/hosted; values:domain/observations |
| kind_of | domain | 2 | values:domain/hosted; values:domain/plans |
| positions | domain | 2 | language:domain/elements; values:domain/rows |
| json | outbound | 2 | benchmark:outbound/collection; benchmark:outbound/worker |

## Function names declared in more than one crate, any visibility

A free function name declared in two or more crates whatever its visibility, so a private copy of another crate's function shows. Methods are not listed.

| name | crates | locations |
|---|---|---|
| answers | core, derived, execution, language, matching, operators, query_language | core:inbound/use_cases/describe_operator; core:inbound/use_cases/describe_relation; core:inbound/use_cases/lint; core:inbound/use_cases/list_operators; core:inbound/use_cases/list_relations; core:inbound/use_cases/open; core:inbound/use_cases/validate; derived:inbound/use_cases/build; derived:inbound/use_cases/prepare_derived; derived:inbound/use_cases/view; execution:inbound/use_cases/execution; execution:inbound/use_cases/metadata; execution:inbound/use_cases/prepare; language:inbound/use_cases/parse; language:inbound/use_cases/project; matching:inbound/use_cases/matching; operators:inbound/use_cases/operators; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/describe; query_language:inbound/use_cases/typecheck |
| runs | core, derived, execution, language, matching, operators, query_language | core:inbound/use_cases/describe_operator; core:inbound/use_cases/describe_relation; core:inbound/use_cases/lint; core:inbound/use_cases/list_operators; core:inbound/use_cases/list_relations; core:inbound/use_cases/open; core:inbound/use_cases/validate; derived:inbound/use_cases/build; derived:inbound/use_cases/prepare_derived; derived:inbound/use_cases/view; execution:inbound/use_cases/execution; execution:inbound/use_cases/metadata; execution:inbound/use_cases/prepare; language:inbound/use_cases/parse; language:inbound/use_cases/project; matching:inbound/use_cases/matching; operators:inbound/use_cases/operators; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/describe; query_language:inbound/use_cases/typecheck |
| given_input | core, derived, language, matching, operators, query_language | core:inbound/use_cases/describe_operator; core:inbound/use_cases/describe_relation; core:inbound/use_cases/lint; core:inbound/use_cases/list_operators; core:inbound/use_cases/list_relations; core:inbound/use_cases/open; core:inbound/use_cases/validate; derived:inbound/use_cases/build; derived:inbound/use_cases/prepare_derived; derived:inbound/use_cases/view; language:inbound/use_cases/parse; language:inbound/use_cases/project; matching:inbound/use_cases/matching; operators:inbound/use_cases/operators; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/describe; query_language:inbound/use_cases/typecheck |
| prepare | benchmark, core, derived, execution, language | benchmark:application/case_execution; benchmark:application/worker_request; benchmark:inbound/use_cases/certify; benchmark:inbound/use_cases/prepare_cases; benchmark:outbound/inventory/adapters/local; core:domain/reading; derived:domain/preparation; execution:application/file_dag; execution:application/file_dag/closure_partitions; execution:application/file_dag/frontiers; execution:application/file_dag/invocation; execution:application/file_dag/program/preparation; execution:application/file_dag/program/prepared_execution; execution:domain/operator/contract; language:application/completion |
| matches | benchmark, execution, matching, operators | benchmark:application/process_discrepancy; execution:application/file_dag/relations; execution:domain/matching; matching:domain/globbing; operators:domain/capture |
| observed | benchmark, core, execution, language | benchmark:domain/process_receipts; benchmark:outbound/suite/adapters/local; core:outbound/corpus/adapters/ignore; execution:application/rule_compilation; language:domain/project_context |
| project | benchmark, language, operators, query_language | benchmark:domain/candidate_rules; language:inbound/use_cases/project; operators:domain/project; query_language:domain/fusing |
| read | benchmark, core, execution, language | benchmark:outbound/receipts/adapters/local; benchmark:outbound/suite/adapters/local; core:outbound/lint_store/adapters/postcard; execution:domain/numeric_values; language:inbound/use_cases/parse |
| admit | benchmark, execution, language | benchmark:application/certified_measurement; benchmark:application/input_metadata; benchmark:application/registered_certification; benchmark:inbound/use_cases/admit_measurement; execution:application/file_dag/frontiers; language:domain/semantics |
| begin | benchmark, core, execution | benchmark:outbound/inventory/adapters/local; core:domain/reading; execution:application/file_dag/program/request_execution; execution:domain/operator/contract |
| build | derived, execution, language | derived:domain/closure_adjacency; derived:domain/cycle_adjacency; derived:domain/dfs; derived:domain/dispatch; derived:domain/equality; derived:domain/membership; derived:domain/ordered; execution:application/file_dag/derived/owner/acquisition/semantics; language:inbound/use_cases/parse |
| cell | derived, operators, values | derived:domain/cycle_adjacency; derived:domain/ordered; operators:domain/reduction; values:domain/rows |
| check | benchmark, derived, operators | benchmark:application/worker_identity; benchmark:domain/rule_context; benchmark:inbound/use_cases/check_worker_request; derived:domain/dfs; derived:domain/preparation; operators:domain/reduction |
| clear | core, execution, language | core:domain/ordering; execution:domain/placement; language:domain/diagnostics |
| describe | derived, execution, values | derived:domain/footprint; execution:application/file_dag/relations; execution:domain/operator; values:domain/descriptions |
| identity | core, execution, values | core:application/project_context; execution:application/file_dag/route_counts; values:domain/hosted; values:domain/observations |
| invalid | core, execution, query_language | core:outbound/lint_store/adapters/postcard; execution:outbound/query_language/adapters/use_case; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/typecheck |
| materialize | benchmark, derived, execution | benchmark:outbound/context/adapters/local; benchmark:outbound/suite/adapters/local; derived:domain/dfs; execution:domain/table_model |
| number | execution, language, query_language | execution:application/file_dag/route_counts; language:domain/elements; query_language:domain/idioms; query_language:outbound/syntax/adapters/winnow |
| positions | language, operators, values | language:domain/elements; operators:domain/equality_join; operators:domain/set_rows; values:domain/rows |
| push | language, operators, query_language | language:domain/builders; operators:domain/reduction; query_language:domain/planning |
| resolve | benchmark, core, execution | benchmark:domain/rule_context; benchmark:outbound/context/adapters/local; core:domain/relation_lookup; execution:domain/executable_program; execution:domain/operands |
| select | benchmark, operators, query_language | benchmark:application/case_selection; benchmark:inbound/use_cases/select_cases; operators:domain/select; query_language:outbound/syntax/adapters/winnow |
| source_column | derived, execution, query_language | derived:domain/native; execution:domain/table_cells; query_language:domain/fusing |
| text | benchmark, core, values | benchmark:outbound/receipts/adapters/local; core:outbound/lint_store/adapters/postcard; values:domain/hosted; values:domain/observations |
| use_case | core, execution, operators | core:outbound/language; execution:outbound/derived; execution:outbound/operators; execution:outbound/query_language; operators:outbound/matching |
| validate | benchmark, core, execution | benchmark:application/discrepancy_validation; benchmark:application/receipt_validation; benchmark:domain/inventory_receipts; benchmark:domain/process_receipts; benchmark:outbound/context/adapters/local; benchmark:outbound/suite/adapters/local; core:outbound/lint_store/adapters/postcard; execution:domain/derived_views; execution:domain/operator/contract |
| absorb | core, execution | core:domain/workers; execution:domain/trust |
| agree | benchmark, core | benchmark:inbound/use_cases/measure_corpus; core:outbound/corpus/adapters/ignore |
| append | core, operators | core:domain/ordering; core:outbound/lint_store/adapters/postcard; operators:domain/append |
| candidate | benchmark, query_language | benchmark:application/case_execution; query_language:domain/fusing |
| canonical | benchmark, execution | benchmark:outbound/inventory/adapters/local; benchmark:outbound/receipts/adapters/local; execution:domain/executable_program |
| capture | benchmark, execution | benchmark:outbound/inventory/adapters/local; execution:application/file_dag/route_counts |
| changed | benchmark, core | benchmark:inbound/use_cases/certify; benchmark:inbound/use_cases/measure; core:outbound/corpus/adapters/ignore |
| check_columns | derived, execution | derived:domain/adjacency; derived:domain/cycle_adjacency; derived:domain/ordered; derived:domain/partition; execution:domain/table_model |
| classify | benchmark, execution | benchmark:application/worker_request; execution:application/file_dag; execution:application/file_dag/program/facade_runtime; execution:domain/executable_program/fusion |
| compare | benchmark, execution | benchmark:domain/diagnostics; benchmark:domain/inventory_comparison; benchmark:inbound/use_cases/compare; execution:domain/numeric_values |
| comparison | benchmark, execution | benchmark:outbound/evidence/adapters/bmf; execution:application/rule_compilation |
| comparisons_of | query_language, values | query_language:domain/selecting; values:domain/plans |
| compile | execution, matching | execution:domain/executable_program; execution:domain/executable_program/file_demand; execution:domain/matching; execution:domain/self_join_recogniser; matching:domain/globbing |
| compile_regex | execution, matching | execution:domain/matching; matching:domain/matching |
| complete | benchmark, language | benchmark:domain/discrepancy; benchmark:inbound/use_cases/audit; benchmark:inbound/use_cases/prepare_cases; benchmark:inbound/use_cases/publish; benchmark:inbound/use_cases/validate_case_receipts; benchmark:inbound/use_cases/verify_cases; benchmark:inbound/use_cases/verify_reference_cases; language:application/completion |
| count | core, language | core:domain/repository; language:domain/diagnostics |
| count_groups | execution, operators | execution:domain/table_groups; operators:domain/reduction |
| decide | core, execution | core:domain/storing; execution:domain/placement |
| deliver | core, operators | core:application/cross_file; operators:domain/render_batch |
| derived | benchmark, execution | benchmark:domain/inventory_receipts; execution:application/file_dag/route_counts |
| digest_groups | execution, operators | execution:domain/table_groups; operators:domain/reduction |
| direction | execution, query_language | execution:domain/stored_graph; query_language:domain/rewriting |
| document | benchmark, execution | benchmark:domain/project_inputs; execution:application/file_dag/route_counts |
| equivalent | benchmark, execution | benchmark:inbound/use_cases/compare; execution:domain/executable_program |
| evaluate | execution, operators | execution:application/file_dag; execution:domain/operator/contract; execution:domain/shared_walk; operators:domain/aggregate; operators:domain/append; operators:domain/axis; operators:domain/capture; operators:domain/closure; operators:domain/cycles; operators:domain/equality_join; operators:domain/evaluation; operators:domain/fits_rendered; operators:domain/ordered_dfs; operators:domain/ordered_match; operators:domain/project; operators:domain/rank; operators:domain/render_batch; operators:domain/replace; operators:domain/select; operators:domain/set_rows; operators:domain/source_matches; operators:domain/table_rows; operators:domain/words |
| executable | benchmark, core | benchmark:outbound/engine/adapters/local; core:outbound/corpus/adapters/ignore |
| expand | benchmark, query_language | benchmark:domain/process_output; query_language:domain/expanding |
| expected | benchmark, execution | benchmark:domain/process_failure; execution:application/file_dag/program/facade_runtime |
| file_root | core, execution | core:outbound/corpus/adapters/ignore; execution:domain/executable_program/file_demand |
| find_or_add | execution, language | execution:domain/table_dedup; language:domain/symbols |
| finding | benchmark, derived | benchmark:application/discrepancy_attachment; derived:domain/native |
| finish | execution, operators | execution:domain/shared_walk; execution:outbound/digest/adapters/blake3; operators:domain/reduction |
| frame | benchmark, core | benchmark:outbound/context/adapters/local; core:domain/repository |
| hash_of | language, operators | language:domain/symbols; operators:domain/reduction |
| incomplete | benchmark, execution | benchmark:inbound/use_cases/audit; execution:application/file_dag/observations; execution:domain/source_partitions |
| inputs_of | query_language, values | query_language:domain/planning; values:domain/plans |
| joined | execution, operators | execution:domain/lineage; execution:domain/operation_schema; operators:domain/equality_join |
| keyed | operators, query_language | operators:domain/equality_join; query_language:domain/planning |
| kind | derived, execution | derived:domain/preparation; execution:domain/table_model |
| kind_of | execution, values | execution:domain/derived_views; values:domain/hosted; values:domain/plans |
| line | language, query_language | language:domain/location; query_language:outbound/syntax/adapters/winnow |
| link | execution, query_language | execution:application/file_dag/derived/owner; execution:domain/file_program; query_language:domain/expanding |
| listed | core, language | core:outbound/corpus/adapters/ignore; language:domain/diagnostics |
| matching | benchmark, execution | benchmark:inbound/use_cases/measure; execution:domain/table_model |
| measured | benchmark, execution | benchmark:inbound/use_cases/measure_corpus; execution:application/file_dag/derived/ledger |
| memory | core, execution | core:outbound/corpus; core:outbound/lint_store; execution:outbound/facts |
| moved | core, execution | core:domain/keeping; execution:domain/completion |
| named | core, values | core:outbound/lint_store/adapters/postcard; values:domain/plans |
| normalize | benchmark, language | benchmark:domain/invocation_identity; language:domain/targets |
| number_of | execution, relations | execution:domain/comparing/collections/comparisons; relations:domain/relations |
| offset | benchmark, execution | benchmark:domain/discrepancy; execution:outbound/query_language/adapters/use_case |
| operand | execution, query_language | execution:domain/operator/contract; query_language:outbound/syntax/adapters/winnow |
| owner_of | core, relations | core:domain/run_rules; relations:domain/terms |
| place | execution, language | execution:domain/placement; language:domain/builders |
| position | execution, query_language | execution:application/rule_compilation; query_language:domain/resolving |
| projected | execution, query_language | execution:domain/operation_schema; query_language:domain/selecting |
| pushed | language, query_language | language:domain/builders; query_language:domain/planning; query_language:domain/pushing |
| quote | benchmark, execution | benchmark:domain/canonical_text; execution:application/file_dag/route_counts |
| reads | benchmark, language | benchmark:inbound/use_cases/select_cases; language:domain/answers |
| receipt | benchmark, language | benchmark:outbound/context/adapters/local; language:domain/project_context |
| record | benchmark, language | benchmark:outbound/receipts/adapters/local; language:domain/diagnostics |
| refused | benchmark, execution | benchmark:application/discrepancy_attachment; benchmark:application/discrepancy_validation; benchmark:application/inventory_verification; benchmark:domain/registered_inputs; execution:domain/finish; execution:outbound/facts/adapters/memory; execution:outbound/facts/adapters/spill; execution:outbound/query_language/adapters/use_case |
| refuses | benchmark, execution | benchmark:inbound/use_cases/account_defaults; benchmark:inbound/use_cases/admit_measurement; benchmark:inbound/use_cases/check_worker_request; benchmark:inbound/use_cases/collect; benchmark:inbound/use_cases/measure_corpus; benchmark:inbound/use_cases/run_ryo; benchmark:inbound/use_cases/select_cases; execution:application/file_dag/program/facade_runtime |
| remapped | execution, values | execution:domain/completion; values:domain/plan_parts |
| replay | core, execution | core:domain/handing; execution:application/file_dag |
| request | benchmark, execution | benchmark:inbound/use_cases/measure_corpus; benchmark:outbound/collection/adapters/json; benchmark:outbound/execution/adapters/timed; execution:domain/file_program; execution:inbound/use_cases/execution; execution:inbound/use_cases/metadata; execution:inbound/use_cases/prepare |
| row | benchmark, operators | benchmark:application/inventory_verification; benchmark:outbound/inventory/adapters/local; operators:domain/reduction |
| rows | benchmark, operators | benchmark:outbound/receipts/adapters/local; operators:domain/table_rows |
| rule | benchmark, execution | benchmark:outbound/suite/adapters/local; execution:application/file_dag/program/facade_runtime; execution:application/file_dag/route_counts |
| same | execution, operators | execution:application/file_dag/relations; execution:domain/comparing/collections/comparisons; operators:domain/reduction |
| size_at | core, language | core:domain/workers; language:domain/elements |
| source | execution, query_language | execution:application/file_dag/derived/owner/acquisition/graph/identity; execution:application/file_dag/program/preparation/service/metadata; query_language:outbound/syntax/adapters/winnow |
| source_id | query_language, values | query_language:domain/linking; values:domain/program |
| stopped | core, execution | core:application/file_analysis/evaluation; execution:application/file_dag/program/request_execution |
| stored_cell | derived, execution | derived:domain/equality; execution:domain/table_model |
| succeeds | benchmark, execution | benchmark:inbound/use_cases/collect; benchmark:inbound/use_cases/run_ryo; execution:application/file_dag/program/facade_runtime |
| summary | benchmark, relations | benchmark:application/inventory_verification; benchmark:outbound/inventory/adapters/local; relations:domain/relations/collections/relations |
| table | derived, execution | derived:domain/footprint; execution:application/file_dag/derived/ledger; execution:domain/executable_program/file_demand |
| tally | execution, values | execution:domain/features; values:domain/hosted; values:domain/observations |
| unreadable | core, query_language | core:outbound/corpus/adapters/ignore; query_language:outbound/syntax/adapters/winnow |
| value | benchmark, query_language | benchmark:outbound/inventory/adapters/local; query_language:outbound/syntax/adapters/winnow |
| vector | derived, execution | derived:domain/footprint; execution:application/file_dag/derived/ledger |
| view | derived, execution | derived:domain/dispatch; execution:application/file_dag; execution:application/file_dag/derived/owner |
| word_of | derived, query_language | derived:domain/key; query_language:domain/resolving |
| words_of | query_language, relations | query_language:domain/resolving; relations:domain/terms |
