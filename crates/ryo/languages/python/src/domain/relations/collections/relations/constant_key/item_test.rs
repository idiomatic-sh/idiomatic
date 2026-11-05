//! Rows expose normalized classes while retaining each original occurrence.

use super::item::ConstantKeyItem;
use crate::domain::relations::collections::fixtures::contract::check;
use crate::fixtures::reading::reading;

#[test]
fn repeated_numeric_literals_share_a_key_and_keep_both_rows() {
    check::<ConstantKeyItem>();
    let result = reading::<ConstantKeyItem>("1\nTrue\n1.0\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 3);
    assert!(result.rows.iter().all(|row| row[1] == result.rows[0][1]));
}

#[test]
fn incomplete_syntax_retains_known_keys_and_marks_the_relation_partial() {
    let result = reading::<ConstantKeyItem>("1\nx +\n");
    assert_eq!(result.rows.len(), 1);
    assert!(!result.causes.is_empty());
}

#[test]
fn decoded_bytes_and_concatenated_strings_share_full_keys() {
    let result = reading::<ConstantKeyItem>("b'a'\nb'\\x61'\n'a' 'b'\n'ab'\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 4);
    assert_eq!(result.rows[0][1], result.rows[1][1]);
    assert_eq!(result.rows[2][1], result.rows[3][1]);
    assert_ne!(result.rows[0][1], result.rows[2][1]);
}

#[test]
fn interpolation_expressions_remain_primitive_but_string_fragments_do_not() {
    let expression = reading::<ConstantKeyItem>("f'{0}'\n");
    assert!(expression.causes.is_empty());
    assert_eq!(expression.rows.len(), 1);

    let escaped_fragment = reading::<ConstantKeyItem>("f'{{0}}'\n");
    assert!(escaped_fragment.causes.is_empty());
    assert!(escaped_fragment.rows.is_empty());
}
