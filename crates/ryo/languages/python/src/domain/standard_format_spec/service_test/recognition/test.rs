//! The observation describes the standard mini-language, not a custom value's `__format__`.
use super::super::super::{service::recognize, types::StandardSpecOutcome as Outcome};
use super::fixtures::target;
use crate::domain::standard_format_spec::types::{FormatSpecText, PresentationCharacter};

#[test]
fn static_clauses_and_unicode_fill_are_recognized() {
    for spec in [
        "",
        "s",
        "*^30s",
        "🦀>8.2f",
        "0=+10,.2f",
        "#x",
        ".3g",
        "٣d",
        ".٢f",
    ] {
        assert_eq!(
            recognize(FormatSpecText(spec), false.into(), target(10)),
            Outcome::Recognized,
            "{spec}"
        );
    }
}

#[test]
fn unsupported_type_is_distinct_from_other_structural_error() {
    for (spec, expected) in [
        ("z", Outcome::InvalidType(PresentationCharacter('z'))),
        ("*^30y", Outcome::InvalidType(PresentationCharacter('y'))),
        ("}", Outcome::InvalidType(PresentationCharacter('}'))),
        (".2fa", Outcome::OtherError),
        ("ds", Outcome::OtherError),
        (".q", Outcome::OtherError),
        ("٣q", Outcome::InvalidType(PresentationCharacter('q'))),
        ("²d", Outcome::InvalidType(PresentationCharacter('²'))),
    ] {
        assert_eq!(
            recognize(FormatSpecText(spec), false.into(), target(10)),
            expected,
            "{spec}"
        );
    }
}

#[test]
fn dynamic_outer_is_deferred_but_nested_specs_can_be_observed_separately() {
    assert_eq!(
        recognize(FormatSpecText(".{width}g"), true.into(), target(10)),
        Outcome::Dynamic
    );
    assert_eq!(
        recognize(FormatSpecText("y"), false.into(), target(10)),
        Outcome::InvalidType(PresentationCharacter('y'))
    );
}

#[test]
fn zero_coercion_is_version_bound() {
    assert_eq!(
        recognize(FormatSpecText("z.2f"), false.into(), target(10)),
        Outcome::InvalidType(PresentationCharacter('z'))
    );
    assert_eq!(
        recognize(FormatSpecText("z.2f"), false.into(), target(11)),
        Outcome::Recognized
    );
}
