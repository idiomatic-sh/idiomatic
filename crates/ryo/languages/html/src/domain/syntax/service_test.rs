//! The `syntax` service's cases.

use super::service::{field_bits, field_name, fields, kind_bits, kind_name, kind_of, kinds};
use super::types::Kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// Whether every kind's number names it again and every kind has a name, and how many kinds there are.
    Every,
    /// The name of an extra.
    Extra(Extra),
    /// How many extras there are.
    Extras,
    /// The names of the roles, in the order of their numbers, each number its place.
    Fields,
    /// Whether the number a resolved target is kept under is the one after the DOM scalar properties and none of theirs.
    TargetProperty,
}

/// What a row asks.
type Input = Question;
/// The answer, as text.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_kind_is_found_by_its_number_and_has_a_name"),
        word: Word::Succeeds,
        input: || Question::Every,
        expect: Expect::Value("true 21"),
    },
    Case {
        name: CaseName("text_the_parser_could_not_read_is_an_error"),
        word: Word::Succeeds,
        input: || Question::Extra(Extra::Error),
        expect: Expect::Value("error"),
    },
    Case {
        name: CaseName("there_is_one_extra"),
        word: Word::Succeeds,
        input: || Question::Extras,
        expect: Expect::Value("1"),
    },
    Case {
        name: CaseName("the_roles_are_the_properties_and_the_attribute_numbered_in_order"),
        word: Word::Succeeds,
        input: || Question::Fields,
        expect: Expect::Value(
            "true none|attribute|name|namespace|prefix|value|public-id|system-id|document-mode",
        ),
    },
    Case {
        name: CaseName("the_resolved_target_is_kept_under_the_number_after_the_dom_properties"),
        word: Word::Succeeds,
        input: || Question::TargetProperty,
        expect: Expect::Value("true"),
    },
];

fn subject(question: Input) -> Value {
    let answer = match question {
        Question::Every => {
            let round = kinds()
                .all(|kind| kind_of(kind_bits(kind)) == kind && !kind_name(kind).0.is_empty());
            format!("{round} {}", kinds().count())
        }
        Question::Extra(extra) => kind_name(Kind::Extra(extra)).0.to_owned(),
        Question::Fields => {
            let numbered = fields()
                .enumerate()
                .all(|(place, field)| usize::from(field_bits(field).0) == place);
            let names = fields()
                .map(|field| field_name(field).0)
                .collect::<Vec<_>>()
                .join("|");
            format!("{numbered} {names}")
        }
        Question::TargetProperty => {
            use super::constants::TARGET_PROPERTY;
            use super::service::property_id;
            use super::types::Property;
            let dom = [
                Property::Name,
                Property::Namespace,
                Property::Prefix,
                Property::Value,
                Property::PublicId,
                Property::SystemId,
                Property::DocumentMode,
            ]
            .map(|property| property_id(property).0);
            let after = dom.iter().max().map(|last| last + 1);
            (Some(TARGET_PROPERTY.0) == after && !dom.contains(&TARGET_PROPERTY.0)).to_string()
        }
        Question::Extras => kinds()
            .filter(|kind| matches!(kind, Kind::Extra(_)))
            .count()
            .to_string(),
    };
    Box::leak(answer.into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
