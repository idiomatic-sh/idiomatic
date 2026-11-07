//! The `language` service's cases: which paths the language reads, and that the rows of two relations about one node name it alike.

use ryo::languages;

use super::service::reads;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::outbound::syntax::fixtures::script::table_of;
use crate::outbound::syntax::fixtures::trees::tree_of;
use std::sync::Arc;

/// What a row asks: whether the language reads a path, or whether the rows of two relations about one node agree on which node it is.
#[derive(Clone, Copy)]
enum Scenario {
    /// A path.
    Path(&'static str),
    /// A file, whose `function` rows are joined to its `syntax_tree` rows on the node.
    Join(&'static str),
}

/// What a row of the cases asks.
type Input = Scenario;

/// What the service answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// Whether the language reads the path.
    Recognition(languages::Recognition),
    /// Each `function` row's node, with the kind the `syntax_tree` row of that node gives, in the order of the rows.
    Functions(&'static [(u64, &'static str)]),
}
type Value = Answer;
type Failure = ();

/// Each `function` row of `source` with the kind of the node it names: the join of two relations on a node.
fn functions_with_their_kinds(source: &str) -> Vec<(u64, &'static str)> {
    let (mut elements, mut symbols, _) =
        table_of(&tree_of(source).expect("a tree is written for the source"));
    let mut out = languages::contract::Collect::default();
    languages::contract::walk(
        &ParseContract::new(
            Arc::new(SyntaxFake(
                tree_of(source).expect("a tree is written for the source"),
            )),
            Arc::new(crate::outbound::notes::factory::postcard()),
        ),
        &mut elements,
        &mut symbols,
        languages::ByteSlice(source.as_bytes()),
        languages::FileId(0),
        &[
            languages::Relation::Function,
            languages::Relation::SyntaxTree,
        ],
        &mut out,
    );
    let rows = out.rows();
    let kind_of = |id: u64| -> &'static str {
        rows.iter()
            .find(|(relation, words)| {
                *relation == languages::Relation::SyntaxTree && words[0] == id
            })
            .and_then(|(_, words)| {
                languages::text_of(
                    &symbols,
                    languages::ByteSlice(source.as_bytes()),
                    languages::Symbol(u32::try_from(words[1]).ok()?),
                )
            })
            .map_or("", |text| Box::leak(text.0.to_owned().into_boxed_str()))
    };
    rows.iter()
        .filter(|(relation, _)| *relation == languages::Relation::Function)
        .map(|(_, words)| (words[0], kind_of(words[0])))
        .collect()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rs_file_is_rust"),
        word: Word::Succeeds,
        input: || Scenario::Path("src/lib.rs"),
        expect: Expect::Value(Answer::Recognition(languages::Recognition::Reads)),
    },
    Case {
        name: CaseName("a_markdown_file_is_not"),
        word: Word::Succeeds,
        input: || Scenario::Path("README.md"),
        expect: Expect::Value(Answer::Recognition(languages::Recognition::Declines)),
    },
    Case {
        name: CaseName("a_function_row_and_a_syntax_tree_row_of_it_share_their_node"),
        word: Word::Succeeds,
        input: || Scenario::Join("fn one() {}\nstruct S;\nimpl S {\n    fn two(&self) {}\n}\n"),
        expect: Expect::Value(Answer::Functions(&[(1, "fn"), (34, "fn")])),
    },
];

fn subject(input: Input) -> Value {
    match input {
        Scenario::Path(path) => Answer::Recognition(reads(languages::SourcePath(path))),
        Scenario::Join(source) => Answer::Functions(Box::leak(
            functions_with_their_kinds(source).into_boxed_slice(),
        )),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
