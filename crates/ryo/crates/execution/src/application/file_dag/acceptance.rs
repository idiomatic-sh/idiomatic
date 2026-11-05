//! Production-route and engine-owned behavioral acceptance.
pub(crate) mod column_kernel_service_test;
#[cfg(test)]
pub(crate) mod column_topology_test;
pub(crate) mod comparing_service_test;
pub(crate) mod file_program_service_test;
pub(crate) mod kind_sets_service_test;
pub(crate) mod kind_sets_types_vocabulary_value_test;
pub(crate) mod layout_service_test;
pub(crate) mod matching_types_prepared_value_test;
pub(crate) mod operating_service_test;
pub(crate) mod operation_schema_service_test;
pub(crate) mod operation_schema_types_bindings_value_test;
pub(crate) mod operation_schema_types_traversal_value_test;
pub(crate) mod operator_collections_operators_antijoin_item_test;
pub(crate) mod operator_collections_operators_axis_item_test;
pub(crate) mod operator_collections_operators_capture_item_test;
pub(crate) mod operator_collections_operators_closure_item_test;
pub(crate) mod operator_collections_operators_cycles_item_test;
pub(crate) mod operator_collections_operators_difference_item_test;
pub(crate) mod operator_collections_operators_extend_item_test;
pub(crate) mod operator_collections_operators_group_count_item_test;
pub(crate) mod operator_collections_operators_group_digest_item_test;
pub(crate) mod operator_collections_operators_intersection_item_test;
pub(crate) mod operator_collections_operators_join_item_test;
pub(crate) mod operator_collections_operators_ordered_match_item_test;
pub(crate) mod operator_collections_operators_project_item_test;
pub(crate) mod operator_collections_operators_rank_item_test;
pub(crate) mod operator_collections_operators_replace_item_test;
pub(crate) mod operator_collections_operators_rows_item_test;
pub(crate) mod operator_collections_operators_select_item_test;
pub(crate) mod operator_collections_operators_source_matches_item_test;
pub(crate) mod operator_collections_operators_span_between_item_test;
pub(crate) mod operator_collections_operators_span_bounds_item_test;
pub(crate) mod operator_collections_operators_union_item_test;
pub(crate) mod operator_collections_operators_words_item_test;
pub(crate) mod operator_contract_test;
pub(crate) mod operator_service_test;
pub(crate) mod stored_graph_service_test;
pub(crate) mod stored_selection_service_test;
pub(crate) mod stored_selection_types_program_value_test;
pub(crate) mod table_scratch_service_test;
pub(crate) mod tree_join_collections_strategies_span_index_item_test;
pub(crate) mod tree_join_service_test;

pub(crate) mod compiled_span_fixture;

pub(crate) mod observed_replay_test;
