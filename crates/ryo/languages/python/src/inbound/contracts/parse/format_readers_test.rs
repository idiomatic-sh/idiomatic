//! Regression cases for format readers bindings.
use super::{FormatFieldItem, FormatParseItem};
use crate::domain::relations::collections::relations::fixtures::contract::check;
use crate::fixtures::reading::reading;
use ryo::languages::contract::Want;

#[test]
fn parse_rows_distinguish_zero_fields_malformed_and_dynamic_inputs() {
    check::<FormatParseItem>();
    let report = reading::<FormatParseItem>(concat!(
        "a = '100%%'\n",
        "b = '%*.*s'\n",
        "c = '%(name)s %(name)r'\n",
        "d = 'é%n'\n",
        "e = f'%s {value}'\n",
        "f = 'left' ' %s'\n",
        "g = f'100%%'\n",
    ));
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(report.rows.len(), 7);
    let outcomes = report
        .rows
        .iter()
        .map(|row| match row[3] {
            Want::Name(value) => value,
            Want::Unlocated
            | Want::Node(_)
            | Want::Unknown(_)
            | Want::File(_)
            | Want::Span(..)
            | Want::Number(_)
            | Want::Flag(_) => panic!("format outcome is a name"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes,
        [
            "parsed",
            "parsed",
            "parsed",
            "malformed",
            "unknown-input",
            "parsed",
            "parsed",
        ]
    );
    assert!(report.rows[1].len() > 10);
    assert!(report.rows[2].len() > 8);
    assert!(report.rows[3].len() > 6);
    assert_eq!(report.rows[0][7], Want::Number(0));
    assert_eq!(report.rows[1][7], Want::Number(3));
    assert_eq!(report.rows[1][9], Want::Flag(true));
    assert_eq!(report.rows[1][10], Want::Flag(true));
    assert_eq!(report.rows[2][8], Want::Flag(true));
    assert_eq!(report.rows[2][7], Want::Number(0));
    assert_eq!(
        report.rows[3][4],
        Want::Name("unsupported-format-character")
    );
    assert_eq!(
        report.rows[3][6],
        Want::Number(2),
        "Unicode index counts scalars"
    );
}

#[test]
fn conversion_rows_keep_repeated_mapping_keys_and_order() {
    check::<FormatFieldItem>();
    let report = reading::<FormatFieldItem>("value = '%(key)s %*.*r %% %s'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(report.rows.len(), 3);
    assert!(report.rows[0].len() > 3);
    assert!(report.rows[1].len() > 6);
    assert_eq!(report.rows[0][1], Want::Number(0));
    assert_eq!(report.rows[0][2], Want::Name("mapping"));
    assert_eq!(report.rows[0][3], Want::Name("key"));
    assert_eq!(report.rows[1][1], Want::Number(1));
    assert_eq!(report.rows[1][2], Want::Name("positional"));
    assert_eq!(report.rows[1][5], Want::Name("star"));
    assert_eq!(report.rows[1][6], Want::Name("star"));
    assert_eq!(report.rows[2][1], Want::Number(2));
}

#[test]
fn unknown_and_malformed_inputs_have_no_conversion_rows() {
    let dynamic = reading::<FormatFieldItem>("value = f'%s {value}'\n");
    assert!(dynamic.causes.is_empty(), "{:?}", dynamic.causes);
    assert!(dynamic.rows.is_empty());
    let malformed = reading::<FormatFieldItem>("value = '%n'\n");
    assert!(malformed.causes.is_empty(), "{:?}", malformed.causes);
    assert!(malformed.rows.is_empty());
}

#[test]
fn template_strings_are_not_percent_string_inputs() {
    let report = reading::<FormatParseItem>("value = t'%s'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert!(report.rows.is_empty());
}

#[test]
fn error_characters_preserve_two_and_four_byte_unicode_scalars() {
    let report = reading::<FormatParseItem>("a = '%é'\nb = '%🧪'\nc = '%s'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(report.rows.len(), 3);
    assert!(report.rows[0].len() > 6);
    assert!(report.rows[1].len() > 6);
    assert_eq!(report.rows[0][5], Want::Name("é"));
    assert_eq!(report.rows[1][5], Want::Name("🧪"));
    assert_eq!(report.rows[2][5], Want::Name(""));
    assert_eq!(report.rows[0][6], Want::Number(1));
    assert_eq!(report.rows[1][6], Want::Number(1));
    let fields = reading::<FormatFieldItem>("a = '%s'\n");
    assert_eq!(fields.rows[0][4], Want::Name("s"));
}
