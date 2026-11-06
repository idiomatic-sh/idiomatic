//! The rails' cases: a file whose tokens do not spell it, a tree that is not the library's, and one that is.

use super::super::expected::{Leafness, ReferenceRefusal, Shape, Walked};
use super::super::findings::Findings;
use super::{
    Category, Input as Railed, inputs_in, lossless, reference_differences, shape_differences,
    syntax_errors_finding,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::elements::service::{diagnose, enter_node, leave_node, table_builder, token};
use crate::domain::elements::types::Elements;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{KindBits, Offset, Position, ReportText};

/// What a row of the cases asks.
#[derive(Clone, Copy)]
enum Scenario {
    /// Tokens that spell the file.
    Spelled,
    /// Tokens with a byte of the file between two of them.
    Gap,
    /// Tokens that stop before the end of the file.
    Short,
    /// Our tree against a library tree that is the same.
    SameShape,
    /// Our tree against one whose token ends one byte later.
    OtherSpan,
    /// Our tree against one with a token more under the root.
    OtherNesting,
    /// An independent accepted reference has the same tree.
    ReferenceAccepted,
    /// An independent accepted reference has a different tree.
    ReferenceDifferent,
    /// Both reference and runtime reject invalid source.
    ReferenceRefused,
    /// A reference refusal is not matched by a runtime diagnostic.
    ReferenceRefusedSilently,
    /// A valid-suite source is refused by the reference.
    ValidReferenceRefused,
    /// A reference refuses a relation fixture and the runtime reports it.
    FixtureReferenceRefused,
    /// An accepted reference cannot match a runtime without a root.
    ReferenceMissingRoot,
    /// A file of this kind that the language reads with this many syntax errors.
    Counted(Category, usize),
    /// A folder named `corpus` holding one file: how many inputs the rails read from it are corpus inputs.
    CorpusFolder,
}

/// The table of the file `ab c`: a root with the tokens `ab` and `c` (or less of it when `short`, or with a gap).
fn table(scenario: Scenario) -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"ab c"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(2));
    match scenario {
        Scenario::Gap => token(&mut builder, KindBits(2), Offset(3), Offset(4)),
        Scenario::Short => {}
        Scenario::Spelled
        | Scenario::Counted(..)
        | Scenario::CorpusFolder
        | Scenario::SameShape
        | Scenario::OtherSpan
        | Scenario::OtherNesting
        | Scenario::ReferenceAccepted
        | Scenario::ReferenceDifferent
        | Scenario::ReferenceRefused
        | Scenario::ReferenceRefusedSilently
        | Scenario::ValidReferenceRefused
        | Scenario::FixtureReferenceRefused
        | Scenario::ReferenceMissingRoot => {
            token(&mut builder, KindBits(3), Offset(2), Offset(3));
            token(&mut builder, KindBits(2), Offset(3), Offset(4));
        }
    }
    leave_node(&mut builder, Offset(4));
    let diagnostics = match scenario {
        Scenario::Counted(_, broken) => broken,
        Scenario::ReferenceRefused
        | Scenario::ValidReferenceRefused
        | Scenario::FixtureReferenceRefused => 1,
        Scenario::Spelled
        | Scenario::Gap
        | Scenario::Short
        | Scenario::SameShape
        | Scenario::OtherSpan
        | Scenario::OtherNesting
        | Scenario::ReferenceAccepted
        | Scenario::ReferenceDifferent
        | Scenario::ReferenceRefusedSilently
        | Scenario::ReferenceMissingRoot
        | Scenario::CorpusFolder => 0,
    };
    for _ in 0..diagnostics {
        diagnose(
            &mut builder,
            ReportText("rejected input"),
            Offset(0),
            Offset(1),
        );
    }
    elements
}

/// A token of the library's tree.
fn piece(kind: u16, start: u32, end: u32) -> Walked {
    Walked {
        kind: KindBits(kind),
        leaf: Leafness::Token,
        start: Offset(start),
        end: Offset(end),
        children: Vec::new(),
    }
}

/// The library's tree of `ab c`.
fn library(children: Vec<Walked>) -> Walked {
    Walked {
        kind: KindBits(1),
        leaf: Leafness::Node,
        start: Offset(0),
        end: Offset(4),
        children,
    }
}

/// What the rail said: nothing, or how many sentences.
type Value = usize;
type Input = Scenario;
type Failure = ();

/// The independent reference outcome of a reference scenario.
fn reference(scenario: Scenario) -> (Shape, Category) {
    match scenario {
        Scenario::ReferenceAccepted | Scenario::ReferenceMissingRoot => (
            Shape::Reference(|_| {
                Ok(library(vec![
                    piece(2, 0, 2),
                    piece(3, 2, 3),
                    piece(2, 3, 4),
                ]))
            }),
            Category::Valid,
        ),
        Scenario::ReferenceDifferent => (
            Shape::Reference(|_| Ok(library(vec![piece(2, 0, 4)]))),
            Category::Valid,
        ),
        Scenario::ValidReferenceRefused => {
            (Shape::Reference(|_| Err(ReferenceRefusal)), Category::Valid)
        }
        Scenario::FixtureReferenceRefused => (
            Shape::Reference(|_| Err(ReferenceRefusal)),
            Category::Fixture,
        ),
        Scenario::Spelled
        | Scenario::Counted(..)
        | Scenario::CorpusFolder
        | Scenario::Gap
        | Scenario::Short
        | Scenario::SameShape
        | Scenario::OtherSpan
        | Scenario::OtherNesting
        | Scenario::ReferenceRefused
        | Scenario::ReferenceRefusedSilently => (
            Shape::Reference(|_| Err(ReferenceRefusal)),
            Category::Invalid,
        ),
    }
}

/// How many of the inputs the rails read from a fixtures folder holding one file in `corpus/` are corpus inputs.
fn corpus_inputs() -> usize {
    let fixtures = std::env::temp_dir().join(format!("rails-corpus-{}", std::process::id()));
    std::fs::create_dir_all(fixtures.join("corpus")).expect("a folder");
    std::fs::write(fixtures.join("corpus").join("a.json"), "{}").expect("a file");
    let mut findings = Findings(Vec::new());
    let found = inputs_in(&fixtures, &mut findings)
        .iter()
        .filter(|input| input.category == Category::Corpus)
        .count();
    std::fs::remove_dir_all(&fixtures).expect("the folder removed");
    found
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let elements = table(scenario);
    Ok(match scenario {
        Scenario::Counted(category, _) => usize::from(
            syntax_errors_finding(
                &Railed {
                    shown: "a.json".to_owned(),
                    path: std::path::PathBuf::new(),
                    category,
                },
                &elements,
            )
            .is_some(),
        ),
        Scenario::CorpusFolder => corpus_inputs(),
        Scenario::Spelled | Scenario::Gap | Scenario::Short => {
            usize::from(lossless(&elements, b"ab c").is_some())
        }
        Scenario::SameShape | Scenario::OtherSpan | Scenario::OtherNesting => {
            let children = match scenario {
                Scenario::OtherSpan => {
                    vec![piece(2, 0, 2), piece(3, 2, 4), piece(2, 3, 4)]
                }
                Scenario::OtherNesting => vec![
                    piece(2, 0, 2),
                    piece(3, 2, 3),
                    piece(2, 3, 4),
                    piece(2, 4, 4),
                ],
                Scenario::Spelled
                | Scenario::Counted(..)
                | Scenario::CorpusFolder
                | Scenario::Gap
                | Scenario::Short
                | Scenario::SameShape
                | Scenario::ReferenceAccepted
                | Scenario::ReferenceDifferent
                | Scenario::ReferenceRefused
                | Scenario::ReferenceRefusedSilently
                | Scenario::ValidReferenceRefused
                | Scenario::FixtureReferenceRefused
                | Scenario::ReferenceMissingRoot => {
                    vec![piece(2, 0, 2), piece(3, 2, 3), piece(2, 3, 4)]
                }
            };
            let mut found = Vec::new();
            shape_differences(
                &elements,
                Position(0),
                &library(children),
                "root",
                &mut found,
            );
            found.len()
        }
        Scenario::ReferenceAccepted
        | Scenario::ReferenceDifferent
        | Scenario::ReferenceRefused
        | Scenario::ReferenceRefusedSilently
        | Scenario::ValidReferenceRefused
        | Scenario::FixtureReferenceRefused
        | Scenario::ReferenceMissingRoot => {
            let (shape, category) = reference(scenario);
            let empty = Elements::default();
            let checked = if matches!(scenario, Scenario::ReferenceMissingRoot) {
                &empty
            } else {
                &elements
            };
            let mut found = Vec::new();
            reference_differences(checked, shape, "ab c", category, &mut found);
            found.len()
        }
    })
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("accepted_reference_tree_is_compared"),
        word: Word::Succeeds,
        input: || Scenario::ReferenceAccepted,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("accepted_reference_mismatch_is_found"),
        word: Word::Succeeds,
        input: || Scenario::ReferenceDifferent,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("explicit_refusal_with_runtime_diagnostic_is_accepted"),
        word: Word::Succeeds,
        input: || Scenario::ReferenceRefused,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("explicit_refusal_without_runtime_diagnostic_fails"),
        word: Word::Succeeds,
        input: || Scenario::ReferenceRefusedSilently,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("valid_suite_cannot_be_refused_by_reference"),
        word: Word::Succeeds,
        input: || Scenario::ValidReferenceRefused,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("invalid_relation_fixture_can_be_refused"),
        word: Word::Succeeds,
        input: || Scenario::FixtureReferenceRefused,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_corpus_file_with_a_syntax_error_is_found"),
        word: Word::Succeeds,
        input: || Scenario::Counted(Category::Corpus, 1),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_corpus_file_with_no_syntax_error_passes"),
        word: Word::Succeeds,
        input: || Scenario::Counted(Category::Corpus, 0),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_file_of_the_corpus_folder_is_a_corpus_input"),
        word: Word::Succeeds,
        input: || Scenario::CorpusFolder,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_valid_suite_file_with_a_syntax_error_is_found"),
        word: Word::Succeeds,
        input: || Scenario::Counted(Category::Valid, 1),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_invalid_suite_file_with_no_syntax_error_is_found"),
        word: Word::Succeeds,
        input: || Scenario::Counted(Category::Invalid, 0),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_relation_fixture_may_carry_syntax_errors"),
        word: Word::Succeeds,
        input: || Scenario::Counted(Category::Fixture, 2),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("accepted_reference_requires_runtime_root"),
        word: Word::Succeeds,
        input: || Scenario::ReferenceMissingRoot,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("tokens_that_spell_the_file_are_lossless"),
        word: Word::Succeeds,
        input: || Scenario::Spelled,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_byte_between_two_tokens_is_lost"),
        word: Word::Succeeds,
        input: || Scenario::Gap,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("tokens_that_stop_before_the_end_lose_the_rest"),
        word: Word::Succeeds,
        input: || Scenario::Short,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_tree_that_is_the_libraries_has_its_shape"),
        word: Word::Succeeds,
        input: || Scenario::SameShape,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_span_that_differs_from_the_libraries_is_found"),
        word: Word::Succeeds,
        input: || Scenario::OtherSpan,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_piece_the_library_nests_and_we_do_not_is_found"),
        word: Word::Succeeds,
        input: || Scenario::OtherNesting,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// The same independent files used for typed rows must also pass syntax/shape/oracle rails.
#[test]
fn relation_rails_include_each_independent_input() -> std::io::Result<()> {
    let root = std::env::temp_dir().join(format!("ryo-multiinput-rails-{}", std::process::id()));
    let folder = root.join("relations").join("sample");
    std::fs::create_dir_all(&folder)?;
    std::fs::write(folder.join("input.py"), "first")?;
    std::fs::write(folder.join("input.other.py"), "second")?;
    let mut findings = Findings(Vec::new());
    let actual: Vec<_> = inputs_in(&root, &mut findings)
        .into_iter()
        .filter(|input| input.category == Category::Fixture)
        .map(|input| input.shown)
        .collect();
    assert_eq!(
        actual,
        ["relations/sample/other.py", "relations/sample/input.py"]
    );
    std::fs::remove_dir_all(root)?;
    Ok(())
}
