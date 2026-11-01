//! Basic expression inference is complete, source ordered, and cached only on request.
use super::super::fixtures::contract::check;
use super::item::ExpressionTypeItem;
use crate::fixtures::reading::reading;

use ryo::languages::contract::Want;

#[test]
fn classifies_literals_unions_unknowns_and_type_errors() {
    check::<ExpressionTypeItem>();
    let report = reading::<ExpressionTypeItem>(
        "1\nTrue\n1 if flag else 2.0\nname\n1 + 'x'\n1 if flag else 'x'\n[1]\n(x for x in items)\n",
    );
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    let types = report
        .rows
        .iter()
        .filter_map(|row| match row[2] {
            Want::Name(name) => Some(name),
            _ => None,
        })
        .collect::<Vec<_>>();
    let resolutions = report
        .rows
        .iter()
        .filter_map(|row| match row[1] {
            Want::Name(name) => Some(name),
            _ => None,
        })
        .collect::<Vec<_>>();
    for expected in [
        "integer",
        "boolean",
        "float",
        "string",
        "list",
        "generator",
        "",
    ] {
        assert!(
            types.contains(&expected),
            "missing {expected:?} in {types:?}"
        );
    }
    assert!(resolutions.contains(&"union"));
    assert!(resolutions.contains(&"unknown"));
    assert!(resolutions.contains(&"type-error"), "{:?}", report.rows);
    for row in report.rows {
        let [_, resolution, inferred_type, _, _] = row else {
            panic!("expression-type rows have five columns");
        };
        match (*resolution, *inferred_type) {
            (Want::Name("unknown" | "type-error"), Want::Name("")) => {}
            (Want::Name("unknown" | "type-error"), other) => {
                panic!("unexpected unresolved type {other:?}")
            }
            (Want::Name("atom" | "union"), Want::Name("")) => {
                panic!("resolved expression has no type")
            }
            _ => {}
        }
    }
}

#[test]
fn validation_diagnostics_do_not_make_basic_types_partial() {
    check::<ExpressionTypeItem>();
    let report = reading::<ExpressionTypeItem>("nonlocal x\n1 if flag else 'x'\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert!(
        report
            .rows
            .iter()
            .any(|row| row[1] == Want::Name("union"))
    );
}

#[test]
fn percent_formatting_keeps_the_left_operand_type_when_the_right_is_unknown() {
    check::<ExpressionTypeItem>();
    let report = reading::<ExpressionTypeItem>("'%s' % unknown\nb'%s' % unknown\n");
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    for (line, column, end_column, expected) in [(1, 1, 15, "string"), (2, 1, 16, "bytes")] {
        assert!(
            report.rows.iter().any(|row| {
                let [_, resolution, inferred_type, _, span] = row else {
                    panic!("expression-type rows have five columns");
                };
                *resolution == Want::Name("atom")
                    && *inferred_type == Want::Name(expected)
                    && matches!(
                        span,
                        Want::Span(start, end)
                            if start.line == line
                                && start.column == column
                                && end.line == line
                                && end.column == end_column
                    )
            }),
            "missing % formatting result type {expected} on line {line}: {:?}",
            report.rows
        );
    }
}

#[test]
fn parser_recovery_keeps_type_rows_partial() {
    check::<ExpressionTypeItem>();
    let report = reading::<ExpressionTypeItem>("value = 1 +\n");
    assert!(!report.causes.is_empty());
}
