# Duplicates

## Public types and traits defined in more than one crate

A name written `pub struct`, `pub enum`, `pub trait` or `pub type` in two or more crates, with every concept that defines it.

| name | crates | kind | locations |
|---|---|---|---|
| ByteOffset | benchmark, sdk, values | struct | benchmark:outbound/suite; sdk:domain/query; values:domain/query_tokens |
| BoundObservationInput | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| Cell | sdk, values | enum | sdk:domain/cell; values:domain/row_texts |
| ColumnName | sdk, values | struct | sdk:domain/relation; values:domain/descriptions |
| Comparison | benchmark, benchmark_contracts | enum | benchmark:outbound/inventory; benchmark_contracts:domain/reports |
| ContextManifest | benchmark, benchmark_contracts | struct | benchmark:outbound/suite; benchmark_contracts:domain/context |
| Example | sdk, values | struct | sdk:domain/operator; values:domain/descriptions |
| FileObservation | benchmark, benchmark_contracts | enum | benchmark:outbound/inventory; benchmark_contracts:domain/workloads |
| Finding | benchmark, derived | enum/struct | benchmark:outbound/suite; derived:domain/contract |
| Function | relations, values | enum/struct | relations:domain/relations/collections/relations; values:domain/plan_parts |
| Holds | relations, sdk | struct | relations:domain/relations; sdk:domain/relation |
| HostColumn | sdk, values | struct | sdk:domain/host; values:domain/hosted |
| HostColumnName | sdk, values | struct | sdk:domain/host; values:domain/hosted |
| HostKind | sdk, values | enum | sdk:domain/host; values:domain/hosted |
| LanguageBuild | core, sdk | struct | core:outbound/language; sdk:domain/language |
| LanguageName | sdk, values | struct | sdk:domain/language; values:domain/descriptions |
| MatchingError | execution, matching | enum | execution:outbound/matching; matching:inbound/use_cases/matching |
| ObservationBindingName | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| ObservationColumn | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| ObservationColumnName | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| ObservationName | sdk, values | struct | sdk:domain/observation; values:domain/observations |
| OperatorName | sdk, values | struct | sdk:domain/operator; values:domain/descriptions |
| PreparedRequest | benchmark, execution | struct | benchmark:application/worker_request; execution:inbound/use_cases/execution |
| ProcessObservation | benchmark, benchmark_contracts | struct | benchmark:outbound/suite; benchmark_contracts:domain/process |
| ProjectScope | benchmark, execution | enum/struct | benchmark:outbound/inventory; execution:inbound/use_cases/execution |
| Purpose | sdk, values | struct | sdk:domain/operator; values:domain/descriptions |
| QueryOrigin | sdk, values | struct | sdk:domain/query; values:domain/query_tokens |
| QuerySpan | sdk, values | struct | sdk:domain/query; values:domain/query_tokens |
| QueryTraceSite | sdk, values | struct | sdk:domain/query; values:domain/query_tokens |
| ReadFailure | sdk, values | struct | sdk:domain/file; values:domain/stored_facts |
| ReferenceDiscrepancy | benchmark, benchmark_contracts | struct | benchmark:outbound/suite; benchmark_contracts:domain/workloads |
| ReferenceExpectation | benchmark, benchmark_contracts | enum | benchmark:outbound/suite; benchmark_contracts:domain/process |
| RelationName | sdk, values | struct | sdk:domain/relation; values:domain/descriptions |
| Side | benchmark, values | enum | benchmark:outbound/suite; values:domain/plan_parts |
| SourceLanguage | execution, sdk | enum | execution:inbound/use_cases/execution; sdk:domain/language |
| SyntaxError | query_language, relations | enum/struct | query_language:outbound/syntax; relations:domain/relations/collections/relations |
| Unread | sdk, values | struct | sdk:domain/file; values:domain/records |
| Unreadable | sdk, values | enum | sdk:domain/file; values:domain/records |
| ValidateError | core, sdk | enum | core:inbound/use_cases/validate; sdk:domain/query |

## Public functions defined in more than one concept of the same layer

A name written `pub fn` in two or more concepts of one layer, across all crates.

| name | layer | concepts | locations |
|---|---|---|---|
| use_case | outbound | 4 | core:outbound/derived; core:outbound/language; core:outbound/operators; core:outbound/query_language |
| local | outbound | 3 | benchmark:outbound/engine; benchmark:outbound/inventory; benchmark:outbound/suite |
| memory | outbound | 3 | core:outbound/corpus; core:outbound/facts; core:outbound/lint_store |
| identity | domain | 2 | values:domain/hosted; values:domain/observations |
| kind_of | domain | 2 | values:domain/hosted; values:domain/plans |
| positions | domain | 2 | language:domain/elements; values:domain/rows |
| json | outbound | 2 | benchmark:outbound/collection; benchmark:outbound/worker |

## Function names declared in more than one crate, any visibility

A free function name declared in two or more crates whatever its visibility, so a private copy of another crate's function shows. Methods are not listed.

| name | crates | locations |
|---|---|---|
| answers | core, execution, language, matching, operators, query_language | core:inbound/use_cases/describe_operator; core:inbound/use_cases/describe_relation; core:inbound/use_cases/lint; core:inbound/use_cases/list_operators; core:inbound/use_cases/list_relations; core:inbound/use_cases/open; core:inbound/use_cases/validate; execution:inbound/use_cases/execution; execution:inbound/use_cases/metadata; language:inbound/use_cases/parse; language:inbound/use_cases/project; matching:inbound/use_cases/matching; operators:inbound/use_cases/operators; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/describe; query_language:inbound/use_cases/typecheck |
| runs | core, execution, language, matching, operators, query_language | core:inbound/use_cases/describe_operator; core:inbound/use_cases/describe_relation; core:inbound/use_cases/lint; core:inbound/use_cases/list_operators; core:inbound/use_cases/list_relations; core:inbound/use_cases/open; core:inbound/use_cases/validate; execution:inbound/use_cases/execution; execution:inbound/use_cases/metadata; execution:inbound/use_cases/publication; language:inbound/use_cases/parse; language:inbound/use_cases/project; matching:inbound/use_cases/matching; operators:inbound/use_cases/operators; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/describe; query_language:inbound/use_cases/typecheck |
| given_input | core, language, matching, operators, query_language | core:inbound/use_cases/describe_operator; core:inbound/use_cases/describe_relation; core:inbound/use_cases/lint; core:inbound/use_cases/list_operators; core:inbound/use_cases/list_relations; core:inbound/use_cases/open; core:inbound/use_cases/validate; language:inbound/use_cases/parse; language:inbound/use_cases/project; matching:inbound/use_cases/matching; operators:inbound/use_cases/operators; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/describe; query_language:inbound/use_cases/typecheck |
| project | benchmark, execution, language, operators, query_language | benchmark:domain/candidate_rules; execution:domain/native_positions; language:inbound/use_cases/project; operators:domain/project; query_language:domain/fusing |
| finish | benchmark, core, execution, operators | benchmark:application/run; core:outbound/digest/adapters/blake3; execution:domain/shared_walk; operators:domain/reduction |
| matches | benchmark, execution, matching, operators | benchmark:application/process_discrepancy; execution:application/file_dag; execution:application/file_dag/derived/owner/acquisition/selection/predicate; execution:domain/matching; matching:domain/globbing; operators:domain/capture |
| observed | benchmark, core, execution, language | benchmark:outbound/suite/adapters/local; core:outbound/corpus/adapters/ignore; execution:application/rule_compilation; language:domain/project_context |
| prepare | benchmark, core, execution, language | benchmark:application/run; benchmark:application/worker_request; benchmark:inbound/use_cases/certify; benchmark:outbound/inventory/adapters/local; core:domain/reading; execution:application/file_dag/frontiers; execution:application/file_dag/program/preparation; execution:application/file_dag/program/preparation/service/orders; execution:domain/operator/contract; language:application/completion |
| read | benchmark, core, execution, language | benchmark:outbound/inventory/adapters/local; benchmark:outbound/suite/adapters/local; core:outbound/lint_store/adapters/postcard; execution:domain/numeric_values; language:inbound/use_cases/parse |
| admit | benchmark, execution, language | benchmark:application/run; execution:application/file_dag/frontiers; language:domain/semantics |
| begin | benchmark, core, execution | benchmark:outbound/inventory/adapters/local; core:domain/reading; execution:domain/operator/contract |
| cell | execution, operators, values | execution:domain/column_kernel; operators:domain/reduction; values:domain/rows |
| clear | core, execution, language | core:domain/ordering; execution:domain/placement; language:domain/diagnostics |
| count | core, execution, language | core:domain/repository; execution:domain/positions; execution:domain/table_model/collections/representations; language:domain/diagnostics |
| identity | core, execution, values | core:application/project_context; execution:application/file_dag; values:domain/hosted; values:domain/observations |
| number | execution, language, query_language | execution:application/file_dag; language:domain/elements; query_language:domain/idioms; query_language:outbound/syntax/adapters/winnow |
| positions | language, operators, values | language:domain/elements; operators:domain/equality_join; operators:domain/set_rows; values:domain/rows |
| push | language, operators, query_language | language:domain/builders; operators:domain/reduction; query_language:domain/planning |
| refused | benchmark, core, execution | benchmark:application/discrepancy_attachment; benchmark:application/discrepancy_validation; benchmark:domain/registered_inputs; core:outbound/facts/adapters/memory; core:outbound/facts/adapters/spill; core:outbound/query_language/adapters/use_case; execution:domain/finish |
| resolve | benchmark, core, execution | benchmark:domain/suite; benchmark:outbound/context/adapters/local; core:domain/relation_lookup; execution:domain/operands |
| row | benchmark, execution, operators | benchmark:outbound/suite/adapters/local; execution:domain/positions; operators:domain/reduction |
| select | execution, operators, query_language | execution:application/file_dag; execution:application/file_dag/derived/owner/selection; execution:domain/native_positions; operators:domain/select; query_language:outbound/syntax/adapters/winnow |
| text | benchmark, core, values | benchmark:application/run; benchmark:outbound/suite/adapters/local; core:outbound/lint_store/adapters/postcard; values:domain/hosted; values:domain/observations |
| validate | benchmark, core, execution | benchmark:application/discrepancy_validation; benchmark:application/inventory_receipts; benchmark:application/process_receipts; benchmark:application/run; benchmark:outbound/context/adapters/local; benchmark:outbound/suite/adapters/local; core:outbound/lint_store/adapters/postcard; execution:domain/derived_views; execution:domain/operator/collections/operators |
| value | benchmark, execution, query_language | benchmark:domain/suite; execution:domain/positions; query_language:outbound/syntax/adapters/winnow |
| absorb | core, execution | core:domain/workers; execution:domain/trust |
| append | core, operators | core:domain/ordering; core:outbound/lint_store/adapters/postcard; operators:domain/append |
| build | execution, language | execution:application/file_dag/derived/owner/acquisition/semantics; language:inbound/use_cases/parse |
| candidate | benchmark, query_language | benchmark:application/run; query_language:domain/fusing |
| canonical | benchmark, execution | benchmark:outbound/inventory/adapters/local; benchmark:outbound/suite/adapters/local; execution:domain/executable_program; execution:domain/file_program |
| changed | benchmark, core | benchmark:inbound/use_cases/certify; benchmark:inbound/use_cases/measure; core:outbound/corpus/adapters/ignore |
| check | benchmark, operators | benchmark:domain/suite; operators:domain/reduction |
| classify | benchmark, execution | benchmark:application/worker_request; execution:application/file_dag; execution:application/file_dag/program/facade_runtime; execution:domain/executable_program/fusion |
| column | execution, language | execution:domain/executable_program; language:domain/location |
| compare | benchmark, execution | benchmark:application/run; benchmark:domain/diagnostics; benchmark:inbound/use_cases/compare; execution:domain/numeric_values |
| comparisons_of | query_language, values | query_language:domain/selecting; values:domain/plans |
| compile | execution, matching | execution:domain/column_kernel; execution:domain/executable_program; execution:domain/executable_program/file_demand; execution:domain/matching; execution:domain/ordered_existence; matching:domain/globbing |
| compile_regex | execution, matching | execution:domain/matching; matching:domain/matching |
| complete | benchmark, language | benchmark:domain/discrepancy; benchmark:inbound/use_cases/audit; benchmark:inbound/use_cases/prepare_cases; benchmark:inbound/use_cases/publish; benchmark:inbound/use_cases/validate_case_receipts; benchmark:inbound/use_cases/verify_cases; benchmark:inbound/use_cases/verify_reference_cases; language:application/completion |
| configuration | benchmark, core | benchmark:application/run; core:domain/repository |
| count_groups | execution, operators | execution:domain/table_groups; operators:domain/reduction |
| decide | core, execution | core:domain/storing; execution:domain/placement |
| derived | benchmark, execution | benchmark:application/inventory_receipts; execution:application/file_dag |
| describe | execution, values | execution:application/file_dag; execution:domain/operator; values:domain/descriptions |
| digest_groups | execution, operators | execution:domain/table_groups; operators:domain/reduction |
| direction | execution, query_language | execution:domain/stored_graph; query_language:domain/rewriting |
| equivalent | benchmark, execution | benchmark:inbound/use_cases/compare; execution:domain/executable_program |
| executable | benchmark, core | benchmark:outbound/engine/adapters/local; core:outbound/corpus/adapters/ignore |
| expand | benchmark, query_language | benchmark:domain/suite/process; query_language:domain/expanding |
| expected | benchmark, execution | benchmark:domain/process_failure; execution:domain/shared_walk |
| file_root | core, execution | core:outbound/corpus/adapters/ignore; execution:domain/executable_program/file_demand |
| find_or_add | execution, language | execution:domain/table_dedup; language:domain/symbols |
| fingerprint | benchmark, execution | benchmark:outbound/context/adapters/local; execution:domain/file_program |
| flag | benchmark, operators | benchmark:outbound/suite/adapters/local; operators:domain/ordered_existence |
| frame | benchmark, core | benchmark:outbound/context/adapters/local; core:domain/repository |
| hash_of | language, operators | language:domain/symbols; operators:domain/reduction |
| incomplete | benchmark, execution | benchmark:inbound/use_cases/audit; execution:application/file_dag; execution:domain/source_partitions |
| inputs_of | query_language, values | query_language:domain/planning; values:domain/plans |
| invalid | core, query_language | core:outbound/lint_store/adapters/postcard; core:outbound/query_language/adapters/use_case; query_language:inbound/use_cases/compile; query_language:inbound/use_cases/typecheck |
| join | execution, query_language | execution:domain/native_positions; query_language:outbound/syntax/adapters/winnow |
| kind | benchmark, execution | benchmark:outbound/suite/adapters/local; execution:domain/table_model/collections/representations |
| kind_of | execution, values | execution:domain/lowering; values:domain/hosted; values:domain/plans |
| line | language, query_language | language:domain/location; query_language:outbound/syntax/adapters/winnow |
| link | execution, query_language | execution:application/file_dag; execution:domain/file_program; query_language:domain/expanding |
| listed | core, language | core:outbound/corpus/adapters/ignore; language:domain/diagnostics |
| lower | execution, query_language | execution:domain/lowering; query_language:domain/assembling |
| matching | benchmark, execution | benchmark:inbound/use_cases/measure; execution:domain/table_model |
| materialize | benchmark, execution | benchmark:outbound/context/adapters/local; benchmark:outbound/suite/adapters/local; execution:domain/table_model |
| named | core, values | core:outbound/lint_store/adapters/postcard; values:domain/plans |
| normalize | benchmark, language | benchmark:domain/invocation_identity; language:domain/targets |
| number_of | execution, relations | execution:domain/comparing/collections/comparisons; relations:domain/relations |
| offer | core, execution | core:domain/ordering; execution:domain/executable_program |
| offset | benchmark, core | benchmark:domain/discrepancy; core:outbound/query_language/adapters/use_case |
| operand | execution, query_language | execution:application/file_dag; execution:domain/operator/contract; query_language:outbound/syntax/adapters/winnow |
| owner_of | core, relations | core:domain/rules; relations:domain/terms |
| place | execution, language | execution:domain/placement; language:domain/builders |
| program | execution, query_language | execution:application/file_dag; query_language:outbound/syntax/adapters/winnow |
| projected | execution, query_language | execution:domain/operation_schema; query_language:domain/selecting |
| publish | benchmark, execution | benchmark:application/run; benchmark:outbound/inventory/adapters/local; execution:application/file_dag |
| pushed | language, query_language | language:domain/builders; query_language:domain/planning; query_language:domain/pushing |
| receipt | benchmark, language | benchmark:outbound/context/adapters/local; language:domain/project_context |
| record | benchmark, language | benchmark:outbound/suite/adapters/local; language:domain/diagnostics |
| refuses | benchmark, execution | benchmark:inbound/use_cases/collect; benchmark:inbound/use_cases/run_ryo; execution:application/file_dag/program/facade_runtime |
| replay | core, execution | core:domain/recording; execution:application/file_dag |
| request | benchmark, execution | benchmark:outbound/collection/adapters/json; benchmark:outbound/execution/adapters/timed; benchmark:outbound/worker/adapters/json; execution:domain/file_program; execution:inbound/use_cases/execution; execution:inbound/use_cases/metadata |
| rule | benchmark, execution | benchmark:outbound/suite/adapters/local; execution:application/file_dag; execution:application/file_dag/program/facade_runtime |
| run | benchmark, execution | benchmark:application/run; benchmark:inbound/use_cases/verify_inventory; execution:application/file_dag; execution:domain/operator/contract |
| same | execution, operators | execution:application/file_dag; execution:application/file_dag/derived/owner/selection/preparation; execution:domain/comparing/collections/comparisons; operators:domain/reduction |
| size_at | core, language | core:domain/workers; language:domain/elements |
| source | execution, query_language | execution:application/file_dag; execution:application/file_dag/derived/owner/acquisition/graph/identity; execution:application/file_dag/program/preparation/service/metadata; query_language:outbound/syntax/adapters/winnow |
| source_column | execution, query_language | execution:domain/tree_join; query_language:domain/fusing |
| source_id | query_language, values | query_language:domain/linking; values:domain/program |
| stage | benchmark, execution | benchmark:application/suite_staging; benchmark:outbound/suite/adapters/local; execution:domain/native_positions |
| subject | benchmark, execution | benchmark:inbound/use_cases/validate_case_receipts; execution:application/file_dag/program/facade_runtime |
| succeeds | benchmark, execution | benchmark:inbound/use_cases/collect; benchmark:inbound/use_cases/run_ryo; execution:application/file_dag/program/facade_runtime |
| summary | benchmark, relations | benchmark:application/run; relations:domain/relations/collections/relations |
| tally | execution, values | execution:domain/features; values:domain/hosted; values:domain/observations |
| unreadable | core, query_language | core:outbound/corpus/adapters/ignore; query_language:outbound/syntax/adapters/winnow |
| values | execution, operators | execution:domain/positions; operators:domain/set_rows |
| word_of | derived, query_language | derived:domain/contract; query_language:domain/resolving |
| words_of | query_language, relations | query_language:domain/resolving; relations:domain/terms |
