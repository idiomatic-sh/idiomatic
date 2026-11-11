//! Independent tree and scalar Join expectations through the compiled owner.
use super::fixtures::{
    a_few_rows_leave_in_the_order_of_the_edges,
    a_node_joined_on_the_child_of_the_tree_finds_its_parent,
    a_node_joined_on_the_parent_of_the_tree_finds_its_children,
    only_the_edges_a_filter_admits_are_joined,
    only_the_elements_a_filter_admits_are_joined_by_their_ids,
    the_edges_of_the_children_of_a_kind_are_found_by_the_kind,
    the_elements_covering_a_stretch_are_found_by_it_alone,
    the_root_has_no_parent_and_a_leaf_no_children,
};

#[test]
fn exact_tree_join_rows_filters_views_and_order() {
    assert_eq!(
        a_node_joined_on_the_child_of_the_tree_finds_its_parent().unwrap(),
        1
    );
    assert_eq!(
        a_node_joined_on_the_parent_of_the_tree_finds_its_children().unwrap(),
        1
    );
    assert_eq!(the_root_has_no_parent_and_a_leaf_no_children().unwrap(), 1);
    assert_eq!(
        only_the_elements_a_filter_admits_are_joined_by_their_ids().unwrap(),
        1
    );
    assert_eq!(only_the_edges_a_filter_admits_are_joined().unwrap(), 1);
    assert_eq!(
        the_edges_of_the_children_of_a_kind_are_found_by_the_kind().unwrap(),
        1
    );
    assert_eq!(a_few_rows_leave_in_the_order_of_the_edges().unwrap(), 1);
    assert_eq!(
        the_elements_covering_a_stretch_are_found_by_it_alone().unwrap(),
        1
    );
}
