//! The `literal_facts` service's cases: what the literals of a source are and hold.

use super::service::{bytes_hex, fragments, number, properties, specifier, string_value};
use super::types::{StringValue, TextBuffer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::node::service::typed_nodes;
use crate::domain::node::types::Numbered;
use crate::fixtures::syntax_files::with_file;
use ryo::languages::Flag;

/// A source.
type Input = &'static str;
/// What each literal of the source answers, in order, joined by `|`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_number_is_its_kind_its_value_and_whether_its_digits_are_exact"),
        word: Word::Succeeds,
        input: || "0\n1_000\n0.1\n1e400\n2j\n",
        expect: Expect::Value(
            "integer 0 exact=true|integer 1000 exact=true|float 0.1 exact=true|float inf exact=false|complex zero=false",
        ),
    },
    Case {
        name: CaseName("a_string_holds_its_joined_parts_and_an_interpolating_one_no_value"),
        word: Word::Succeeds,
        input: || "'a' \"b\"\nf'{{c}}'\nf'{d}e'\nb'\\x00\\xff'\n__import__('module')\n",
        expect: Expect::Value(
            "string zero=false|holds \"ab\" loads=false|holds \"{c}\" loads=false|piece \"{c}\"|interpolates|piece \"e\"|bytes zero=false|hex 00ff|string zero=false|holds \"module\" loads=true",
        ),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let mut shown: Vec<String> = Vec::new();
    with_file(source, &mut |file| {
        let (mut buffer, mut pieces) = (TextBuffer::default(), Vec::new());
        for Numbered { id, node } in typed_nodes(file) {
            if let Some(found) = number(file, id, &mut buffer) {
                let exact = found.exact == Flag::Raised;
                shown.push(format!("{} {} exact={exact}", found.kind.0, buffer.0));
            } else if let Some(found) = properties(node) {
                let zero = found.zero == Flag::Raised;
                shown.push(format!("{} zero={zero}", found.kind.0));
            } else {
                // A node that is no number and no primitive literal may still be an f-string.
            }
            match string_value(node, &mut buffer) {
                Some(StringValue::Known(text)) => {
                    let loads = specifier(file, id) == Flag::Raised;
                    shown.push(format!("holds {:?} loads={loads}", text.0));
                }
                Some(StringValue::Interpolated) => shown.push("interpolates".to_owned()),
                None => {}
            }
            if bytes_hex(node, &mut buffer).is_some() {
                shown.push(format!("hex {}", buffer.0));
            }
            pieces.clear();
            fragments(node, &mut pieces);
            shown.extend(
                pieces
                    .iter()
                    .map(|piece| format!("piece {:?}", piece.value.0)),
            );
        }
    });
    Ok(Box::leak(shown.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
