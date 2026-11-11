//! The `keyed` service's cases, on values alone: the declarations a data document's rows make, with its sequences'
//! elements in their places, and a decoration's arguments.

use super::service::{compare_data, data_declarations};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contracts::service::facts;
use crate::domain::contracts::types::{
    DataRow, PartText, RowKey, RowNode, RowPlace, Syntax, SyntaxKind, WrittenValue,
};
use crate::domain::declarations::service::arguments;
use crate::domain::definitions::types::Exactness;

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The keys of a document, from its data.
    Keyed,
    /// How many mismatches a sequence emptied has with its template's one element.
    Emptied,
    /// How many mismatches a sequence with a value written between its template's values has, and one in another order.
    Inserted,
    /// How many mismatches a table lacking a key another table carries has, and a sequence whose tables moved.
    PerElement,
    /// The arguments a decoration passes, one at a time.
    Arguments,
    /// How many mismatches an exact document has with its template when one of its tables holds the instance's values.
    InstanceValues,
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_key_is_named_by_its_path_an_element_by_its_place_and_a_scalar_by_its_value",
        ),
        word: Word::Succeeds,
        input: || Question::Keyed,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_element_of_the_template_is_not_met_by_an_empty_sequence"),
        word: Word::Succeeds,
        input: || Question::Emptied,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_sequence_holds_its_templates_values_in_order_with_others_between"),
        word: Word::Succeeds,
        input: || Question::Inserted,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("each_table_of_a_sequence_is_judged_whole_wherever_it_is_placed"),
        word: Word::Succeeds,
        input: || Question::PerElement,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_decorations_arguments_split_outside_brackets_and_quotes"),
        word: Word::Succeeds,
        input: || Question::Arguments,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "an_instance_values_tables_entries_hold_the_templates_values_and_may_hold_more",
        ),
        word: Word::Succeeds,
        input: || Question::InstanceValues,
        expect: Expect::Value(String::new()),
    },
];

/// A row of `syntax`.
fn row(
    syntax: Syntax,
    (id, owner): (&str, &str),
    (key, kind, text): (&str, &str, &str),
) -> DataRow {
    DataRow {
        syntax,
        id: RowNode(id.into()),
        owner: RowNode(owner.into()),
        key: RowKey(key.into()),
        kind: SyntaxKind(kind.into()),
        text: WrittenValue(text.into()),
        span: RowPlace::Unplaced,
    }
}

/// The rows of `[package]\nname = "a"\nrequires = ["x"]\n[[bench]]\nharness = false`, as a data language gives them.
fn document() -> Vec<DataRow> {
    vec![
        row(Syntax::Entries, ("#4", "#0"), ("package", "mapping", "")),
        row(Syntax::Mappings, ("#1", "#4"), ("0", "standard", "")),
        row(Syntax::Entries, ("#9", "#1"), ("name", "string", "\"a\"")),
        row(
            Syntax::Entries,
            ("#10", "#1"),
            ("requires", "sequence", "[\"x\"]"),
        ),
        row(Syntax::Elements, ("#11", "#10"), ("0", "string", "\"x\"")),
        row(
            Syntax::Entries,
            ("#7", "#0"),
            ("bench", "sequence-of-mappings", ""),
        ),
        row(Syntax::Elements, ("#6", "#7"), ("0", "mapping", "")),
        row(Syntax::Mappings, ("#6", "#6"), ("0", "sequence-member", "")),
        row(
            Syntax::Entries,
            ("#8", "#6"),
            ("harness", "boolean", "false"),
        ),
    ]
}

/// The rows of a sequence of tables, each by its id and whether it carries a parser.
fn tables(each: &[(&str, bool)]) -> Vec<DataRow> {
    let mut rows = vec![row(
        Syntax::Entries,
        ("#1", "#0"),
        ("tools", "sequence-of-mappings", ""),
    )];
    for (place, (id, parsed)) in each.iter().enumerate() {
        let element = format!("#e{place}");
        rows.push(row(
            Syntax::Elements,
            (&element, "#1"),
            (&place.to_string(), "mapping", ""),
        ));
        rows.push(row(
            Syntax::Entries,
            (&format!("{element}id"), &element),
            ("id", "string", &format!("\"{id}\"")),
        ));
        if *parsed {
            rows.push(row(
                Syntax::Entries,
                (&format!("{element}parser"), &element),
                ("parser", "string", "\"sarif\""),
            ));
        }
    }
    rows
}

/// How many mismatches `found` has with the template's `expected`, which it must contain.
fn counted(expected: &[DataRow], found: &[DataRow]) -> String {
    compare_data(&facts(expected), &facts(found), Exactness::Contains, None)
        .len()
        .to_string()
}

/// What a document of `variables.libraries`, `variables-x.a` and `rules.[0].exclude` writes, and whether its
/// `variables` also holds `more = []`.
struct Written<'a> {
    libraries: &'a [&'a str],
    x: &'a [&'a str],
    exclude: &'a [&'a str],
    more: bool,
}

/// The rows of `written`'s document, as a data language gives them.
fn manifest(written: &Written<'_>) -> Vec<DataRow> {
    let values = |rows: &mut Vec<DataRow>, owner: &str, each: &[&str]| {
        for (place, value) in each.iter().enumerate() {
            rows.push(row(
                Syntax::Elements,
                (&format!("{owner}{place}"), owner),
                (&place.to_string(), "string", &format!("\"{value}\"")),
            ));
        }
    };
    let mut rows = vec![
        row(Syntax::Entries, ("#v", "#0"), ("variables", "mapping", "")),
        row(Syntax::Mappings, ("#vm", "#v"), ("0", "standard", "")),
        row(Syntax::Entries, ("#vl", "#vm"), ("libraries", "sequence", "")),
    ];
    values(&mut rows, "#vl", written.libraries);
    if written.more {
        rows.push(row(Syntax::Entries, ("#vo", "#vm"), ("more", "sequence", "")));
    }
    rows.extend([
        row(Syntax::Entries, ("#x", "#0"), ("variables-x", "mapping", "")),
        row(Syntax::Mappings, ("#xm", "#x"), ("0", "standard", "")),
        row(Syntax::Entries, ("#xa", "#xm"), ("a", "sequence", "")),
    ]);
    values(&mut rows, "#xa", written.x);
    rows.extend([
        row(
            Syntax::Entries,
            ("#r", "#0"),
            ("rules", "sequence-of-mappings", ""),
        ),
        row(Syntax::Elements, ("#r0", "#r"), ("0", "mapping", "")),
        row(Syntax::Mappings, ("#r0", "#r0"), ("0", "sequence-member", "")),
        row(Syntax::Entries, ("#re", "#r0"), ("exclude", "sequence", "")),
    ]);
    values(&mut rows, "#re", written.exclude);
    rows
}

fn subject(question: Input) -> Value {
    let (said, expected) = match question {
        Question::Keyed => (
            data_declarations(&facts(&document()))
                .iter()
                .map(|declaration| {
                    std::iter::once(&*declaration.name.0)
                        .chain(declaration.attributes.iter().map(|value| &*value.0))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("; "),
            "bench; bench.[0]; bench.[0].harness = false; package; package.name = \"a\"; package.requires; package.requires.[0] = \"x\"".to_owned(),
        ),
        Question::Emptied => {
            let emptied = document()
                .into_iter()
                .filter(|row| row.syntax != Syntax::Elements || &*row.id.0 != "#11")
                .collect::<Vec<_>>();
            (counted(&document(), &emptied), "1".to_owned())
        }
        Question::Inserted => {
            let values = |each: &[&str]| {
                let mut rows = vec![row(Syntax::Entries, ("#1", "#0"), ("run", "sequence", ""))];
                for (place, value) in each.iter().enumerate() {
                    rows.push(row(
                        Syntax::Elements,
                        (&format!("#v{place}"), "#1"),
                        (&place.to_string(), "string", &format!("\"{value}\"")),
                    ));
                }
                rows
            };
            (
                format!(
                    "{} {}",
                    counted(&values(&["a", "c"]), &values(&["a", "b", "c"])),
                    counted(&values(&["a", "c"]), &values(&["c", "a"])),
                ),
                "0 1".to_owned(),
            )
        }
        Question::PerElement => (
            format!(
                "{} {}",
                counted(
                    &tables(&[("a", true), ("b", true)]),
                    &tables(&[("a", false), ("b", true)])
                ),
                counted(
                    &tables(&[("a", true), ("b", false)]),
                    &tables(&[("b", false), ("c", true), ("a", true)])
                ),
            ),
            "1 0".to_owned(),
        ),
        Question::InstanceValues => {
            let template = Written {
                libraries: &["a"],
                x: &["p"],
                exclude: &["x"],
                more: false,
            };
            let exact = |found: &Written<'_>, instance_values: Option<&str>| {
                compare_data(
                    &facts(&manifest(&template)),
                    &facts(&manifest(found)),
                    Exactness::Exact,
                    instance_values,
                )
                .len()
                .to_string()
            };
            let added = Written {
                libraries: &["a", "b"],
                ..template
            };
            (
                [
                    exact(&added, Some("variables")),
                    exact(
                        &Written {
                            exclude: &["x", "y"],
                            ..template
                        },
                        Some("variables"),
                    ),
                    exact(
                        &Written {
                            libraries: &["b"],
                            ..template
                        },
                        Some("variables"),
                    ),
                    exact(
                        &Written {
                            more: true,
                            ..template
                        },
                        Some("variables"),
                    ),
                    exact(&added, None),
                    exact(
                        &Written {
                            x: &["p", "q"],
                            ..template
                        },
                        Some("variables"),
                    ),
                ]
                .join(" "),
                "0 1 1 1 1 1".to_owned(),
            )
        }
        Question::Arguments => (
            arguments(PartText("Clone,  Debug, rename = \"a, b\", f(x, y)"))
                .iter()
                .map(|argument| &*argument.0)
                .collect::<Vec<_>>()
                .join(" "),
            "Clone Debug rename=\"a, b\" f(x,y)".to_owned(),
        ),
    };
    if said == expected {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
