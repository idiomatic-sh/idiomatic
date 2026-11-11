//! Named producer populations use the actual prepared union and common file scheduler.
use super::fixtures::exercise;
#[test]
fn singleton_and_many_producers_preserve_distinct_union_order_through_one_prepared_root() {
    let cases: &[(&[&[i64]], &[i64], usize)] = &[
        (&[&[]], &[], 0),
        (&[&[1, 1, 2]], &[1, 2], 6),
        (&[&[1, 1, 2], &[]], &[1, 2], 3),
        (&[&[], &[1, 1, 2]], &[1, 2], 3),
        (&[&[1, 2], &[2, 3], &[1, 3]], &[1, 2, 3], 5),
    ];
    for &(given, expected, registered) in cases {
        exercise(given, Some(expected), registered);
    }
}
#[test]
fn zero_producers_preserve_the_declared_project_phase_in_file_scope() {
    exercise(&[], None, 0);
}
