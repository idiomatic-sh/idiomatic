//! The support of the group's tests.
use super::super::super::{service::recognize, types::StandardSpecOutcome as Outcome};
use crate::domain::standard_format_spec::types::StandardFormatTarget;
use crate::domain::standard_format_spec::types::{FormatSpecText, PresentationCharacter};
use crate::domain::standard_library::types::{PythonMajor, PythonMinor, TargetVersion};

/// Python 3 at the given minor release, as the recognizer is asked about it.
pub(super) fn target(minor: i64) -> StandardFormatTarget {
    TargetVersion {
        major: PythonMajor(3),
        minor: PythonMinor(minor),
    }
    .into()
}

pub(super) fn static_clauses_and_unicode_fill_are_recognized() -> String {
    let mut observed: Vec<String> = Vec::new();
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
        observed.push(format!(
            "{:?}",
            recognize(FormatSpecText(spec), false.into(), target(10))
        ));
    }
    observed.join("|")
}

pub(super) fn unsupported_type_is_distinct_from_other_structural_error() -> String {
    let mut observed: Vec<String> = Vec::new();
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
        observed.push(format!(
            "{:?}",
            recognize(FormatSpecText(spec), false.into(), target(10))
        ));
    }
    observed.join("|")
}

pub(super) fn dynamic_outer_is_deferred_but_nested_specs_can_be_observed_separately() -> String {
    let mut observed: Vec<String> = Vec::new();
    observed.push(format!(
        "{:?}",
        recognize(FormatSpecText(".{width}g"), true.into(), target(10))
    ));
    observed.push(format!(
        "{:?}",
        recognize(FormatSpecText("y"), false.into(), target(10))
    ));
    observed.join("|")
}

pub(super) fn zero_coercion_is_version_bound() -> String {
    let mut observed: Vec<String> = Vec::new();
    observed.push(format!(
        "{:?}",
        recognize(FormatSpecText("z.2f"), false.into(), target(10))
    ));
    observed.push(format!(
        "{:?}",
        recognize(FormatSpecText("z.2f"), false.into(), target(11))
    ));
    observed.join("|")
}
