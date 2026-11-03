//! The observation describes the standard mini-language, not a custom value's `__format__`.
use super::{StandardSpecOutcome as Outcome, recognize};

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
            recognize(spec, false, (3, 10)),
            Outcome::Recognized,
            "{spec}"
        );
    }
}

#[test]
fn unsupported_type_is_distinct_from_other_structural_error() {
    for (spec, expected) in [
        ("z", Outcome::InvalidType('z')),
        ("*^30y", Outcome::InvalidType('y')),
        ("}", Outcome::InvalidType('}')),
        (".2fa", Outcome::OtherError),
        ("ds", Outcome::OtherError),
        (".q", Outcome::OtherError),
        ("٣q", Outcome::InvalidType('q')),
        ("²d", Outcome::InvalidType('²')),
    ] {
        assert_eq!(recognize(spec, false, (3, 10)), expected, "{spec}");
    }
}

#[test]
fn dynamic_outer_is_deferred_but_nested_specs_can_be_observed_separately() {
    assert_eq!(recognize(".{width}g", true, (3, 10)), Outcome::Dynamic);
    assert_eq!(recognize("y", false, (3, 10)), Outcome::InvalidType('y'));
}

#[test]
fn zero_coercion_is_version_bound() {
    assert_eq!(recognize("z.2f", false, (3, 10)), Outcome::InvalidType('z'));
    assert_eq!(recognize("z.2f", false, (3, 11)), Outcome::Recognized);
}
