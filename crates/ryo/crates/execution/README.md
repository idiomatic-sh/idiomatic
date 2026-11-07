# idiomatic-sh-ryo-execution

Owns Ryo's compiled request execution, source admission, shared scheduling, publication and release through inbound use cases.

## Domains

### column_kernel

Defines the column-oriented join kernels used by admitted execution.

### comparing

Evaluates typed selection comparisons against borrowed rows and bindings.

### coverage

Tracks which source relations and file partitions are complete or partial.

### crossing

Carries publication data and readiness across file and project phases.

### derived_views

Defines pure semantics for views prepared and acquired by the owner.

### describing

Builds operator and field descriptions for request reporting.

### executable_program

Defines immutable compiled regions, producers and dependency edges.

### features

Attributes observed row work to the features used by a request.

### file_program

Compiles per-file operation dependencies and admissible cut points.

### finish

Normalizes producer names and prepares portable publication values.

### frontiers

Describes the retained rows and project facts read by cross-file steps.

### functions

Computes comparison function text from typed operand values.

### hosted

Describes host-provided relations and their declared row shapes.

### kind_check

Checks query kind names against the linked language vocabulary.

### kind_sets

Prepares named kind comparisons as sets of admitted kinds.

### lowering

Derives tree-position shapes for eligible file-local operations.

### matching

Evaluates compiled text patterns over borrowed query values.

### native_positions

Evaluates admitted select, project, join, antijoin, union and axis positions.

### numeric_values

Orders canonical decimal and binary numeric values consistently.

### operands

Reads typed comparison operands from rows, parameters and literals.

### operating

Defines operator inputs, outputs and refusal semantics.

### operation_schema

Infers and checks the ordered columns produced by each operation.

### operator

Holds prepared operator metadata and borrowed semantic contracts.

### ordered_existence

Checks ordered existence against prepared join inputs.

### placement

Decides which configured rules govern a source file.

### positions

Represents borrowed tree coordinates and their row geometry.

### prepared_request

Holds immutable request declarations consumed by compilation.

### replay

Describes how produced rows are passed to their consumers.

### rules

Looks up admitted rule coordinates in the compiled request.

### selections

Evaluates tree selections and projections over borrowed rows.

### shared_walk

Groups compatible selections for one request-wide file walk.

### spelling

Checks equal name hashes against retained text to detect collisions.

### step_classes

Classifies compatible selection steps before shared traversal.

### stored_graph

Describes generation-bound graph population and adjacency.

### stored_selection

Describes selectors prepared once for stored relations.

### table_cells

Reads and validates typed cells in produced tables.

### table_dedup

Tracks distinct table rows while preserving encounter order.

### table_groups

Groups table rows for aggregate operations.

### table_model

Defines table representations and their physical row access.

### table_reach

Traverses reachable table nodes in source order.

### table_scratch

Holds per-invocation table workspace and work counters.

### table_texts

Resolves table text cells through their retained name sources.

### text_gate

Prepares fixed text comparisons for tree selection.

### tree_join

Matches tree edges to admitted left rows in demand order.

### tree_walk

Traverses parent and child relations for axis operations.

### trust

Tracks relation reads that rely on the absence of missing rows.

### walks

Applies filtered tree walks to admitted position inputs.


## Retired test routes and current owners

This source migration replaces tests of removed `FileDagExecution`, `PreparedPool`,
`PrimitiveExecutor`, `Room`, and `Feed` APIs. Its new tests are authored but unrun;
no compiler, timing, or TM closure follows from this table. Existing owner tests
remain registered. Physical counters of removed cut caches are not runtime APIs.

| Former subjects | Current controls and retained obligation |
| --- | --- |
| `request_dag_executes_shared_dependencies_once_and_releases_at_graph_last_use`; `global_dag_keeps_a_prefix_with_one_reader_per_rule`; `duplicate_cut_finishes_once_and_releases_only_after_last_rule` | Public `selected_join_shares_work_preserves_counts_and_exports_owned_frontiers` covers two rules, identical outputs/counts, no repeated readiness/walk/provider work, owned escaped frontiers and a fresh refused generation. Actual final-consumer release is covered by `shared_published_producer_keeps_index_until_its_final_real_consumer` and `upfront_skipped_reader_releases_the_real_shared_index_after_the_remaining_reader`. |
| `distinct_graph_joins_share_the_existing_file_owned_source_index`; `budget_refusal_is_explicit_recomputation_and_never_stale_generation_reuse` | Required derived lifetime tests retain actual index weak handles and zero-budget ownership through both final completion and refusing consumers. Public compact controls refuse an unavailable provider in a fresh file. Optional removed cut-cache budget/hit counters are retired, not reintroduced. |
| `incomplete_or_invalid_population_stays_entirely_on_the_residual_executor`; `partial_population_never_cuts_ancestor_execution` | Pure `invalid_schema_stays_deferred_in_same_graph`; actual `malformed_declared_population_is_refused_not_silently_shortened`, `ragged_selected_fallback_cannot_admit_mismatched_projection_kinds`, and partial named-observation controls. There is one interpreter: no residual-executor eligibility assertion survives. |
| `failed_pure_cut_is_never_published_or_replayed`; `opaque_first_failure_prevents_later_rule_graph_work` | Canonical public shared-walk semantic-failure/error-order controls and `nested_demand_preserves_both_frames_and_exact_semantic_failure` retain exact terminal errors/no publication/no repeated work. Independent declared roots now advance through the request walk; the former “later rule cannot run” policy is intentionally obsolete. |
| `registered_union_preserves_bag_contract_and_shares_its_repeated_dependency`; `provenance_admission_keeps_unrelated_internal_streams_lazy` | Public Union controls preserve the current distinct-set contract and demand both inputs in order; domain `ordered_dependencies_remain_authored_inputs` retains two exact typed upstream edges. The former bag/cache implementation comparison is superseded by the canonical Union contract. |
| `typed_graph_shares_only_complete_pure_cuts_and_releases_last_consumer` | Domain `caller_origins_do_not_split_pure_identity` and `different_bound_texts_keep_distinct_identity` retain origin independence and binding-sensitive physical identities. Completion/readiness and final-consumer release stay under their runtime owners. |
| `repeated_cuts_count_one_consumer_per_active_complete_rule` | Domain `duplicate_steps_share_physical_identity`; public deep/diamond/duplicate-input readiness controls, skipped-root controls, and actual producer-lifetime tests retain deduplication and last-consumer behavior. Removed activation-cache reader counters are not treated as the current ledger. |
| `cached_descendant_does_not_call_its_ancestor_evaluator`; `dependency_used_by_another_frontier_still_executes_and_cannot_evict_the_cut`; `cache_miss_runs_ancestors` | Public compact controls require stable provider/readiness/walk counters on repeated roots plus exact frontiers. Existing public first-run/cached-run and frontier/root-role controls retain real admission, dependency and publication behavior. |
| `empty_left_join_never_executes_its_right_dependency_and_cancels_its_lifetime` | Public `shared_walk_uses_the_same_join_dependency_contract_for_callback_delivery` requires the canonical RHS failure in both presentation modes and orders. Skipping the required RHS is intentionally removed; actual producer-lifetime tests still verify cancellation/release. |
| `ordinary_join_and_group_count_keep_single_reader_streams`; `streamed_count_left_and_held_count_right_join_under_one_owner` | Domain ordered dependency controls and public `grouped_dependency_and_original_source_join_in_both_input_positions` require exact grouped/source rows through the same real owner in both Join positions. Old transport mode names no longer choose evaluation. |
| `dag_root_and_recursive_child_use_injected_executor` | Actual public injected semantic capability controls exercise recursive producer and root dispatch, exact semantic refusal and no repeat, plus canonical publication callback-retry tests. No test executor is retained. |
| `native_batch_refusal_cannot_publish_classes_or_nodes` | Public per-class semantic failure isolation/error-order tests retain no failed publication and stable retry. A globally injected `PrimitiveExecutor.native_classes` batch-poison route no longer exists; private permit checks remain required. |
| `compact_chains_cross_rule_join_and_frontier_match_uncached_execution`; `compact_distinct_join_frontier_and_provider_refusal_match_fallback`; `selected_compact_inputs_preserve_fed_work_with_counting_enabled` | Public compact controls cover Stored and Computed inputs, 64→8 selection, distinct/bag projection, exact Join rows, mandatory per-step fed counts, no reexecution and escaped frontier lifetime. `distinct_projections_preserve_rows_and_predicate_cache` and selection retention controls retain actual shared predicate positions and bounded owner storage. Optional counting-off and separate ordinary pipeline executors are removed. |


The matching/schema migration additionally removes the four acceptance files
`matching_types_prepared_value_test`, `operation_schema_service_test`,
`operation_schema_types_bindings_value_test`, and
`operation_schema_types_traversal_value_test`. Their source-owned mapping is:

| Former proof | Current owner and exact retained requirement |
| --- | --- |
| Literal matcher lifetime, invalid/empty/short-circuit patterns, dynamic bindings/functions/rows/lists, glob family/polarity/Unicode, concurrent first use | `domain/matching/types/prepared/value_test.rs` retains finite slot identity, exact failed compilation, laziness, separate requests, Unicode fallback and concurrent once-only build. `inbound/.../prepared_matching_test.rs` uses actual prepared requests and fresh public FileRuns, observes zero invocation regex/glob builds for literals and one for dynamic patterns, preserves empty/error/output checks. An actual same-byte unused finite slot is deliberately prewarmed to falsify accidental dynamic reuse. No optional prepared-owner injection or raw evaluator remains. |
| Every empty/populated operation schema and every declared kind/feature | All 26 operations still run at both cardinalities through the public owner in `inbound/.../schema_test.rs`; pure catalog uniqueness, virtual reach schema and missing Hosted input metadata live in `domain/operation_schema/service_test.rs`. Original typed operation/input constructors live once in `domain/operation_schema/fixtures.rs`. Hosted runtime data now uses its actual supported Number/Text/File boundary; the original six-kind metadata projection remains an independent exact pure assertion. An undeclared Hosted request source fails source-partition admission with `NotRead`, before schema evaluation. `MissingInput { position: 1 }` remains the pure schema refusal for an absent input descriptor; these are different boundaries. |
| Twelve binding cases | Ten pure binding cases remain adjacent to `BindingShape` in `types/bindings/value_test.rs`; the two invalid-regex-on-real-row cases enter the actual inbound owner in `binding_schema_test.rs`, preserving schema success, empty success and nonempty invalid-regex refusal. |
| Eleven traversal cases | Nine pure shape/error cases remain adjacent to `TraversalShape`; both native Axis/view-projected Join cases run real compiled requests over the original constructed Elements in `traversal_schema_test.rs`, with populated/empty output-kind comparison. The public source selector chooses the same root node rather than forging native Rows. |
| Raw cross-phase `ReadsAStepNobodyHandedOver` | `inbound/.../project_input_test.rs` creates an actual CrossFile Project over an undeclared PerFile handover, prepares no invented packet, and requires typed `NotRead` failure on both demands, zero fed work and zero project provider calls. The old helper's unscoped `NotRun` is not a supported phase API contract. |

These replacement bodies are **unrun** until the combined compiler/test gate.
Fixture assembly only supplies typed hosted/catalog data and the real borrowed
semantic capability; preparation, admission, execution, publication and finish
remain exclusively in the production public owner.

The same batch migrates the remaining raw operator tests to
`inbound/use_cases/execution/use_case_test`: Select/Extend/Closure/Axis/Kinds,
span-key joins, complete26 conformance, output/error/delivery parity and real
injected semantic refusal. Source datasets remain independent of expected rows.
Generic selection/cache policy fixtures keep their private schema parameters;
public output oracles use genuine catalog schemas and hosted inputs. Schema
missing-input and foreign-operation checks remain pure preparation contracts.
The metadata catalog retains the example-relation assertion once across its
complete descriptor inventory. The old raw cross-file handover check is replaced by the public missing-restored-frontier control: exact `Failed(NotRead)`, no provider work or fed rows, and unchanged retry. The unscoped helper's old `NotRun` classification is intentionally not a public contract.

Lifetime assertions sample actual consumer release before/after events, recording
reader count, weak-backing liveness and required bytes. They do not infer
nonfinal retention from output delivery, which follows canonical preparation of
all declared roots. Optional budget zero, exact output/error, last-consumer
release before file finish, and upfront-skipped obligations remain explicit.
These authored postimages have not been compiled or executed.

Review corrections keep owner-facing selection/cache tests within the execution
inbound test module; application code does not import private inbound fixtures.
Collected conformance explicitly distinguishes `Failed` from `Unknown` (the
partial semantic batch expects `Unknown`), and ordinary exact-failure fixtures
reject unexpected `Unknown` outcomes. The matching cleanup observation now comes
from an actual file-owner weak handle dying at file finish; it is not a claimed
earlier invocation release. The invalid terminal Project remains unfused: its
actual Join reader releases first, then Project returns its exact column error.
The provider-refusal Union control now also declares callback delivery and
requires one terminal RHS refusal, no delivered prefix and no provider retry.
All correction controls remain unexecuted pending the combined gate.

The index lifetime control now follows the single `DerivedOwner` equality entry.
The former wrapper-eviction stage assumed a second backend equality cache;
`CacheBackend` no longer owns such a payload. The replacement keeps the same
live-consumer refusal, weak backing, shared-allocation charge and final-release
assertions at the actual owner. This retires the obsolete two-cache stage rather
than claiming its intermediate retention behavior still exists.
