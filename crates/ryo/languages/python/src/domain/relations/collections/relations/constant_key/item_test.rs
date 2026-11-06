//! Rows expose normalized classes while retaining each original occurrence.

use crate::cases::{Case, CaseName, Expect, Word, run};
use super::item::ConstantKeyItem;
use crate::domain::relations::collections::fixtures::contract::check;
use crate::fixtures::reading::reading;

fn repeated_numeric_literals_share_a_key_and_keep_both_rows() {
    check::<ConstantKeyItem>();
    let result = reading::<ConstantKeyItem>("1\nTrue\n1.0\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 3);
    assert!(result.rows.iter().all(|row| row[1] == result.rows[0][1]));
}

fn incomplete_syntax_retains_known_keys_and_marks_the_relation_partial() {
    let result = reading::<ConstantKeyItem>("1\nx +\n");
    assert_eq!(result.rows.len(), 1);
    assert!(!result.causes.is_empty());
}

fn decoded_bytes_and_concatenated_strings_share_full_keys() {
    let result = reading::<ConstantKeyItem>("b'a'\nb'\\x61'\n'a' 'b'\n'ab'\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 4);
    assert_eq!(result.rows[0][1], result.rows[1][1]);
    assert_eq!(result.rows[2][1], result.rows[3][1]);
    assert_ne!(result.rows[0][1], result.rows[2][1]);
}

fn interpolation_expressions_remain_primitive_but_string_fragments_do_not() {
    let expression = reading::<ConstantKeyItem>("f'{0}'\n");
    assert!(expression.causes.is_empty());
    assert_eq!(expression.rows.len(), 1);

    let escaped_fragment = reading::<ConstantKeyItem>("f'{{0}}'\n");
    assert!(escaped_fragment.causes.is_empty());
    assert!(escaped_fragment.rows.is_empty());
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("repeated_numeric_literals_share_a_key_and_keep_both_rows"),
        word: Word::Succeeds,
        input: || repeated_numeric_literals_share_a_key_and_keep_both_rows,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("incomplete_syntax_retains_known_keys_and_marks_the_relation_partial"),
        word: Word::Succeeds,
        input: || incomplete_syntax_retains_known_keys_and_marks_the_relation_partial,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("decoded_bytes_and_concatenated_strings_share_full_keys"),
        word: Word::Succeeds,
        input: || decoded_bytes_and_concatenated_strings_share_full_keys,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("interpolation_expressions_remain_primitive_but_string_fragments_do_not"),
        word: Word::Succeeds,
        input: || interpolation_expressions_remain_primitive_but_string_fragments_do_not,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
