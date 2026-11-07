//! The `scaffolding` service's cases, on values alone.

use std::collections::BTreeMap;

use super::fixtures::{captures, declared, entry, template};
use super::service::{
    declaring, fewest, first_render, first_writing, given, literal_values, named_by_directory,
    naming, optionality, placed_name, unfilled, waited, waiting,
};
use super::types::{
    Declaring, FirstWriting, MissingParam, Naming, Optionality, PlacedName, Unfilled, Waiting,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::Captures;
use crate::domain::contexts::types::ContextKey;
use crate::domain::entries::types::{
    Amount, Bound, ConditionName, Count, ParamName, ParamSource, ParamValue, ParamValues, Setting,
    Value as EntryValue,
};
use crate::domain::paths::types::RepoPath;
use crate::domain::scalars::types::TextValue;
use crate::domain::selectors::types::PathPattern;

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The name `items/{lib}.*` places with `lib = x` and `ext = rs`.
    Placed,
    /// The capture `items/{lib}.txt` still holds when nothing gave `lib`.
    Unfilled,
    /// Whether a name with a `?` left is a pattern.
    Pattern,
    /// The literal `pkg::{lang}` an entry passes, with `lang = go`.
    Literal,
    /// The param a spec takes from the name of the directory `langs/go`.
    Directory,
    /// The fewest files a conditional entry asks for, one when it holds and none when not.
    Fewest,
    /// The values set on the command line, as captures.
    Given,
    /// A first render's values: the scaffold flag, the value given and the param named by the directory.
    Rendered,
    /// A first render of a template declaring a param nothing gives.
    Missing,
    /// Whether a conditional entry that may place none and an entry of one file are optional.
    Optional,
    /// Whether a template that reads the scaffold flag, one that does not, and none say what a first writing starts with.
    FirstWriting,
    /// Which placed names wait for the capture an optional `items/{lib}.txt` introduces.
    Waiting,
    /// The module file that declares `src/inbound`, and which entries place it.
    Module,
}

/// What a row asks.
type Input = Asked;
/// What the service answered, as text.
type Value = String;
/// The param nothing gave.
type Failure = MissingParam;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_placed_name_takes_the_captures_and_the_extension"),
        word: Word::Succeeds,
        input: || Asked::Placed,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_capture_nothing_gave_is_named"),
        word: Word::Succeeds,
        input: || Asked::Unfilled,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_with_a_wildcard_left_is_a_pattern"),
        word: Word::Succeeds,
        input: || Asked::Pattern,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_literal_param_takes_the_captures"),
        word: Word::Succeeds,
        input: || Asked::Literal,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_param_from_the_directory_is_its_name"),
        word: Word::Succeeds,
        input: || Asked::Directory,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_conditional_entry_needs_its_fewer_answer"),
        word: Word::Succeeds,
        input: || Asked::Fewest,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_values_set_are_captures"),
        word: Word::Succeeds,
        input: || Asked::Given,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_first_render_has_the_flag_the_values_and_the_directory_param"),
        word: Word::Succeeds,
        input: || Asked::Rendered,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_entry_that_may_place_none_is_optional_and_one_of_one_file_is_required"),
        word: Word::Succeeds,
        input: || Asked::Optional,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "only_a_template_that_reads_the_scaffold_flag_says_what_a_first_writing_starts_with",
        ),
        word: Word::Succeeds,
        input: || Asked::FirstWriting,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_holding_a_capture_an_optional_file_introduces_waits_for_it"),
        word: Word::Succeeds,
        input: || Asked::Waiting,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "the_module_file_is_beside_the_directory_and_placed_by_a_required_entry_of_its_name",
        ),
        word: Word::Succeeds,
        input: || Asked::Module,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_param_nothing_gives_is_refused_by_name"),
        word: Word::Refuses,
        input: || Asked::Missing,
        expect: Expect::Refusal(|actual| {
            if actual == &(MissingParam("kind".into())) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// An empty text when `said` is what was wanted, else what was said.
fn held(said: String, wanted: &str) -> Value {
    if said == wanted { String::new() } else { said }
}

/// What the `Optional` row asks, as text.
fn optional_said() -> Value {
    held(
        format!(
            "{:?} {:?}",
            optionality(&Count::When {
                condition: ConditionName("marked".into()),
                then: Amount(1),
                otherwise: Amount(0),
            }),
            optionality(&Count::Exactly(Amount(1)))
        ),
        &format!("{:?} {:?}", Optionality::Optional, Optionality::Required),
    )
}

/// What the `FirstWriting` row asks, as text.
fn first_writing_said() -> Value {
    held(
        format!(
            "{:?} {:?} {:?}",
            first_writing(Some(&template("{% if scaffold %}x{% endif %}"))),
            first_writing(Some(&template("x"))),
            first_writing(None)
        ),
        &format!(
            "{:?} {:?} {:?}",
            FirstWriting::Said,
            FirstWriting::Unsaid,
            FirstWriting::Unsaid
        ),
    )
}

/// What the `Waiting` row asks, as text.
fn waiting_said() -> Value {
    let waits = waited(&[
        entry(
            "items/{lib}.txt",
            Count::Range {
                min: Amount(0),
                max: Bound::Unbounded,
            },
        ),
        entry("root.txt", Count::Exactly(Amount(1))),
    ]);
    held(
        format!(
            "{:?} {:?}",
            waiting(&PlacedName("items/{lib}/notes.txt".into()), &waits),
            waiting(&PlacedName("root.txt".into()), &waits)
        ),
        &format!("{:?} {:?}", Waiting::Waits, Waiting::Ready),
    )
}

/// What the `Module` row asks, as text.
fn module_said() -> Value {
    let at = RepoPath("src/inbound".into());
    let given = captures(&[("domain", "inbound")]);
    held(
        format!(
            "{:?} {:?} {:?}",
            declaring(
                &RepoPath::default(),
                &entry("src/{domain}.py", Count::Exactly(Amount(1))),
                &given,
                &at
            ),
            declaring(
                &RepoPath::default(),
                &entry("src/{domain}.txt", Count::Exactly(Amount(1))),
                &given,
                &at
            ),
            declaring(
                &RepoPath::default(),
                &entry(
                    "src/{domain}.txt",
                    Count::Range {
                        min: Amount(0),
                        max: Bound::Unbounded
                    }
                ),
                &given,
                &at
            )
        ),
        &format!(
            "{:?} {:?} {:?}",
            Declaring::Declares,
            Declaring::Declares,
            Declaring::Other
        ),
    )
}

/// What a literal param that names a capture is written as, against what should be.
fn literal_said() -> Value {
    let passed = ParamValues(BTreeMap::from([(
        ParamName("trait".into()),
        ParamValue::Literal(EntryValue::Text(TextValue("pkg::{lang}".into()))),
    )]));
    held(
        format!(
            "{:?}",
            literal_values(
                &passed,
                &captures(&[("lang", "go")]),
                &crate::outbound::manifest::factory::toml()
            )
        ),
        &format!("{:?}", captures(&[("trait", "pkg::go")])),
    )
}

/// What the first render of a file at `directory` holds, against what should be: the given captures, the param the
/// directory names, and the scaffold flag.
fn rendered_said(directory: &RepoPath) -> Result<Value, Failure> {
    let context = first_render(
        &captures(&[("lib", "x")]),
        &declared("lang", ParamSource::DirectoryName),
        &ParamValues::default(),
        directory,
        &crate::outbound::manifest::factory::toml(),
    )?;
    let keys = context
        .values
        .iter()
        .map(|(key, value)| format!("{}={}", key.0, value.0))
        .collect::<Vec<_>>()
        .join(",");
    let captured = context.captured.contains_key(&ContextKey("lib".into()));
    Ok(held(
        format!("{keys} {captured}"),
        "lang=go,lib=x,scaffold=yes true",
    ))
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let directory = RepoPath("langs/go".into());
    Ok(match asked {
        Asked::Placed => held(
            placed_name(
                &PathPattern("items/{lib}.*".into()),
                &captures(&[("lib", "x"), ("ext", "txt")]),
            )
            .0,
            "items/x.txt",
        ),
        Asked::Unfilled => {
            let left = unfilled(&PlacedName("items/{lib}.txt".into()));
            held(
                format!("{left:?}"),
                &format!("{:?}", Some(Unfilled("lib".into()))),
            )
        }
        Asked::Pattern => held(
            format!("{:?}", naming(&PlacedName("a?.txt".into()))),
            &format!("{:?}", Naming::Pattern),
        ),
        Asked::Literal => literal_said(),
        Asked::Directory => held(
            format!(
                "{:?}",
                named_by_directory(
                    &declared("lang", ParamSource::DirectoryName),
                    &directory,
                    &crate::domain::findings::types::WorkspaceName("ws".into()),
                )
            ),
            &format!("{:?}", captures(&[("lang", "go")])),
        ),
        Asked::Fewest => held(
            format!(
                "{:?}",
                fewest(&Count::When {
                    condition: ConditionName("marked".into()),
                    then: Amount(1),
                    otherwise: Amount(0),
                })
            ),
            &format!("{:?}", Amount(0)),
        ),
        Asked::Given => held(
            format!(
                "{:?}",
                given(&BTreeMap::from([(
                    Setting("lib".into()),
                    Setting("x".into())
                )]))
            ),
            &format!("{:?}", captures(&[("lib", "x")])),
        ),
        Asked::Rendered => rendered_said(&directory)?,
        Asked::Optional => optional_said(),
        Asked::FirstWriting => first_writing_said(),
        Asked::Waiting => waiting_said(),
        Asked::Module => module_said(),
        Asked::Missing => {
            first_render(
                &Captures::default(),
                &declared("kind", ParamSource::Given),
                &ParamValues::default(),
                &directory,
                &crate::outbound::manifest::factory::toml(),
            )
            .map(drop)?;
            String::new()
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
