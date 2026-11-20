//! The `spec_facts` service's cases: the interpolations of the f-strings of a source, each with the f-string that holds it and
//! the interpolation whose format spec it stands in.

use super::service::{owned_fields, parent_place};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::node::service::written;
use crate::fixtures::syntax_files::with_file;

/// A source.
type Input = &'static str;
/// Each interpolation as `<f-string>:<interpolation>^<place of its parent>`, joined by `|`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "an_interpolation_in_a_format_spec_belongs_to_the_f_string_and_has_a_parent",
        ),
        word: Word::Succeeds,
        input: || "f'{a:{b}}{c}'\n",
        expect: Expect::Value("f'{a:{b}}{c}':{a:{b}}^-|f'{a:{b}}{c}':{b}^0|f'{a:{b}}{c}':{c}^-"),
    },
    Case {
        name: CaseName("an_f_string_inside_an_interpolation_holds_its_own"),
        word: Word::Succeeds,
        input: || "f'{f\"{a}\"}'\n",
        expect: Expect::Value("f'{f\"{a}\"}':{f\"{a}\"}^-|f\"{a}\":{a}^-"),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let mut shown: Vec<String> = Vec::new();
    with_file(source, &mut |file| {
        let mut owned = Vec::new();
        owned_fields(file, &mut owned);
        for group in owned.chunk_by(|left, right| left.owner == right.owner) {
            for each in group {
                let parent = parent_place(file, group, *each)
                    .map_or_else(|| "-".to_owned(), |place| place.0.to_string());
                shown.push(format!(
                    "{}:{}^{parent}",
                    written(file, each.owner).0,
                    written(file, each.field).0
                ));
            }
        }
    });
    Ok(Box::leak(shown.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
