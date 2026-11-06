//! Real readers retain grammar ancestry and origins independently of call policy.
use super::FormatSpecificationItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::collections::relations::fixtures::contract::check;
use crate::fixtures::reading::reading;
use ryo::languages::contract::Want;
fn literals_without_method_calls_and_fstrings_have_independent_spec_facts() {
    check::<FormatSpecificationItem>();
    let result = reading::<FormatSpecificationItem>("'{:q}'\n'{:{width:y}}'\nf'{value:q}'\n");
    assert!(result.causes.is_empty(), "{:?}", result.causes);
    assert_eq!(result.rows.len(), 4);
    assert_eq!(
        result
            .rows
            .iter()
            .map(|row| (row[1], row[2], row[4], row[5], row[6], row[7]))
            .collect::<Vec<_>>(),
        [
            (
                Want::Number(0),
                Want::Number(-1),
                Want::Name("brace-string"),
                Want::Name("q"),
                Want::Name("invalid-type"),
                Want::Name("q")
            ),
            (
                Want::Number(0),
                Want::Number(-1),
                Want::Name("brace-string"),
                Want::Name("{width:y}"),
                Want::Name("dynamic"),
                Want::Name("")
            ),
            (
                Want::Number(1),
                Want::Number(0),
                Want::Name("brace-string"),
                Want::Name("y"),
                Want::Name("invalid-type"),
                Want::Name("y")
            ),
            (
                Want::Number(0),
                Want::Number(-1),
                Want::Name("fstring"),
                Want::Name("q"),
                Want::Name("invalid-type"),
                Want::Name("q")
            ),
        ]
    );
    assert_eq!(result.rows[0][0], result.rows[0][3]);
    assert_ne!(
        result.rows[3][0], result.rows[3][3],
        "an actual f-string format-spec node is its own origin"
    );
}
fn malformed_braces_and_absent_colons_do_not_fabricate_specs() {
    let result = reading::<FormatSpecificationItem>("'{}'\n'{broken'\n'{:}'\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0][5], Want::Name(""));
    assert_eq!(result.rows[0][6], Want::Name("recognized"));
}
fn unicode_width_and_precision_are_standard_syntax() {
    let result = reading::<FormatSpecificationItem>("'{:٣d} {:.٢f} {:²d}'\n");
    assert!(result.causes.is_empty());
    assert_eq!(
        result.rows.iter().map(|row| row[6]).collect::<Vec<_>>(),
        [
            Want::Name("recognized"),
            Want::Name("recognized"),
            Want::Name("invalid-type")
        ]
    );
}

fn dynamic_fstring_value_remains_unknown_and_nested_specs_keep_ancestry() {
    let result = reading::<FormatSpecificationItem>("f'{value:{width:y}}'\n");
    assert!(result.causes.is_empty(), "{:?}", result.causes);
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0][6], Want::Name("dynamic"));
    assert!(matches!(result.rows[0][5], Want::Unknown(_)));
    assert_eq!(result.rows[1][2], Want::Number(0));
    assert_eq!(result.rows[1][6], Want::Name("invalid-type"));
    assert_eq!(result.rows[1][7], Want::Name("y"));
}

/// Which of the file's cases a row runs.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The case `literals_without_method_calls_and_fstrings_have_independent_spec_facts`.
    LiteralsWithoutMethodCallsAndFstringsHaveIndependentSpecFacts,
    /// The case `malformed_braces_and_absent_colons_do_not_fabricate_specs`.
    MalformedBracesAndAbsentColonsDoNotFabricateSpecs,
    /// The case `unicode_width_and_precision_are_standard_syntax`.
    UnicodeWidthAndPrecisionAreStandardSyntax,
    /// The case `dynamic_fstring_value_remains_unknown_and_nested_specs_keep_ancestry`.
    DynamicFstringValueRemainsUnknownAndNestedSpecsKeepAncestry,
}

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Body, (), ()>] = &[
    Case {
        name: CaseName("literals_without_method_calls_and_fstrings_have_independent_spec_facts"),
        word: Word::Succeeds,
        input: || Body::LiteralsWithoutMethodCallsAndFstringsHaveIndependentSpecFacts,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("malformed_braces_and_absent_colons_do_not_fabricate_specs"),
        word: Word::Succeeds,
        input: || Body::MalformedBracesAndAbsentColonsDoNotFabricateSpecs,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unicode_width_and_precision_are_standard_syntax"),
        word: Word::Succeeds,
        input: || Body::UnicodeWidthAndPrecisionAreStandardSyntax,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("dynamic_fstring_value_remains_unknown_and_nested_specs_keep_ancestry"),
        word: Word::Succeeds,
        input: || Body::DynamicFstringValueRemainsUnknownAndNestedSpecsKeepAncestry,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn body(given: Body) -> Result<(), ()> {
    match given {
        Body::LiteralsWithoutMethodCallsAndFstringsHaveIndependentSpecFacts => {
            literals_without_method_calls_and_fstrings_have_independent_spec_facts()
        }
        Body::MalformedBracesAndAbsentColonsDoNotFabricateSpecs => {
            malformed_braces_and_absent_colons_do_not_fabricate_specs()
        }
        Body::UnicodeWidthAndPrecisionAreStandardSyntax => {
            unicode_width_and_precision_are_standard_syntax()
        }
        Body::DynamicFstringValueRemainsUnknownAndNestedSpecsKeepAncestry => {
            dynamic_fstring_value_remains_unknown_and_nested_specs_keep_ancestry()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, body);
}
