//! The kind-set scenarios: trees named by kind, selections over them, and what each observed.
use super::super::super::fixtures;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;

use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::node_position;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, KindBits, Offset, Relation, Value as Cell,
};
use ryo_relations::domain::terms::types::NameText;
use std::borrow::Cow;

use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_domain::domain::kind_sets::service::{names_a_kind, satisfies};
use ryo_execution_domain::domain::operating::types::KindNaming;
use ryo_execution_domain::domain::table_model::types::Verdict;
use ryo_execution_domain::domain::table_texts::types::TextView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;

/// What a row asks of the kind sets.
#[derive(Clone, Copy, Debug)]
pub(super) enum Kinds {
    /// Twelve selections of a named tree by kind name and text.
    Named,
    /// A file with no tree, then a named one, through one prepared request.
    NoNames,
    /// One prepared request over trees of two languages that number their kinds otherwise.
    Languages,
    /// Which tests name a kind, and how a name satisfies tests.
    Naming,
    /// A selection by a test on the kind name, keeping the node, the name and the node again.
    Mask(Test, &'static str),
}
/// The positions of the nodes each labelled selection found, in order.
pub(super) type Found = Cow<'static, [(&'static str, Cow<'static, [u32]>)]>;
/// What a row observed.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Observed {
    Found(Found),
    /// The rows of the file with no names, then of the named file.
    Counts([usize; 2]),
    /// What a test on the name, on the text and by order names; then four verdicts of a name against a test.
    Naming {
        naming: [KindNaming; 3],
        verdicts: [Verdict; 4],
    },
    /// The schema, the rows, the node positions and the kept name of a masked selection.
    Masked {
        kinds: Cow<'static, [CellKind]>,
        rows: usize,
        nodes: Cow<'static, [u32]>,
        names: Cow<'static, [Cow<'static, str>]>,
    },
}

const FROM: PlanInput = PlanInput::Relation(Relation::SyntaxTree);

/// The comparison of the column at `place` with the text `text` by `test`.
fn compared(test: Test, place: u32, literal: &str) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(column(place)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(literal.into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

/// The elements of a tree filtered by the name of their kind and by their text are found among the elements of that kind.
pub(super) fn finds_the_elements_of_a_named_kind() -> Result<Observed, OperatingError> {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let source = b"abcd";
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(source));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    token(&mut builder, KindBits(3), Offset(1), Offset(2));
    token(&mut builder, KindBits(2), Offset(2), Offset(3));
    leave_node(&mut builder, Offset(4));
    for (bits, name) in [(1, "file"), (2, "word"), (3, "other")] {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(source), FileId(0)),
            KindBits(bits),
            NameText(name),
        );
    }
    index_kinds(&mut elements);
    let kind_is = |name: &str| compared(Test::Equal, 1, name);
    let kind_is_not = |name: &str| compared(Test::NotEqual, 1, name);
    let kind_glob = |test: Test, pattern: &str| compared(test, 1, pattern);
    let text_is = |text: &str| compared(Test::Equal, 2, text);
    let mut found = |comparisons: Vec<PlanComparison>| -> Result<Vec<Cell>, OperatingError> {
        let plan = inputs::plan(vec![Operation::Select {
            from: FROM,
            comparisons,
            kept: vec![column(0)],
        }]);
        Ok(fixtures::requests::published_file(
            plan,
            vec![],
            vec![],
            source,
            &elements,
            &mut symbols,
        )?
        .cells)
    };
    let words = found(vec![kind_is("word")])?;
    let a_word = found(vec![kind_is("word"), text_is("c")])?;
    let none = found(vec![kind_is("nothing")])?;
    let others = found(vec![kind_is_not("word")])?;
    let neither = found(vec![kind_is_not("word"), kind_is_not("file")])?;
    let a_word_that_is_not_the_file = found(vec![kind_is("word"), kind_is_not("file")])?;
    let nodes = found(vec![text_is("")])?;
    let tokens = found(vec![compared(Test::NotEqual, 2, "")])?;
    let one_of = found(vec![kind_is("word"), compared(Test::GlobAny, 2, "{a,c}")])?;
    let in_a_list = found(vec![kind_glob(Test::GlobAny, "{file,other}")])?;
    let not_in_a_list = found(vec![kind_glob(Test::NotGlobAny, "{file,other}")])?;
    let a_prefix = found(vec![
        kind_glob(Test::GlobAny, "wo*"),
        kind_glob(Test::NotGlobAny, "x*"),
    ])?;
    Ok(Observed::Found(
        vec![
            ("nodes", nodes),
            ("tokens", tokens),
            ("one_of", one_of),
            ("in_a_list", in_a_list),
            ("not_in_a_list", not_in_a_list),
            ("a_prefix", a_prefix),
            ("words", words),
            ("a_word", a_word),
            ("none", none),
            ("others", others),
            ("neither", neither),
            ("a_word_that_is_not_the_file", a_word_that_is_not_the_file),
        ]
        .into_iter()
        .map(|(label, cells)| (label, positions(&cells)))
        .collect(),
    ))
}

/// A file nothing read has no tree and no names: asking the kinds of it what a name means must not teach the
/// scratch that no kind has the name, for the files that come after.
pub(super) fn a_file_with_no_names_teaches_nothing_about_kinds() -> Result<Observed, OperatingError>
{
    let operation = Operation::Select {
        from: FROM,
        comparisons: vec![compared(Test::Equal, 1, "word")],
        kept: vec![column(0)],
    };
    let (engine, prepared) =
        fixtures::requests::prepared(inputs::plan(vec![operation]), vec![], vec![])?;
    let never = |_| Err(OperatingError::NotRead);
    let empty = fixtures::requests::run_prepared(
        &engine,
        &prepared,
        b"",
        &Elements::default(),
        &mut Symbols::default(),
        &never,
    )?;
    let (elements, mut symbols) = tree_of(&[(1, "file"), (2, "word")], 2);
    let later = fixtures::requests::run_prepared(
        &engine,
        &prepared,
        b"abcd",
        &elements,
        &mut symbols,
        &never,
    )?;
    Ok(Observed::Counts([empty.count.0, later.count.0]))
}

/// A tree of one root and one token, whose kinds are numbered and named as `names` say.
fn tree_of(names: &[(u16, &str)], token_kind: u16) -> (Elements, Symbols) {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let source = b"abcd";
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(source));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(token_kind), Offset(0), Offset(1));
    leave_node(&mut builder, Offset(4));
    for (bits, name) in names {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(source), FileId(0)),
            KindBits(*bits),
            NameText(name),
        );
    }
    index_kinds(&mut elements);
    (elements, symbols)
}

/// Two languages number their kinds otherwise: a name the first file of one language taught the scratch is not the sets of
/// the files of another.
pub(super) fn another_language_does_not_inherit_the_sets_of_the_first()
-> Result<Observed, OperatingError> {
    let operation = Operation::Select {
        from: FROM,
        comparisons: vec![PlanComparison {
            test: Test::Equal,
            left: Operand::Column(column(1)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        }],
        kept: vec![column(0)],
    };
    let (engine, prepared) =
        fixtures::requests::prepared(inputs::plan(vec![operation]), vec![], vec![])?;
    let found = |names: &[(u16, &str)], token_kind: u16| -> Result<Vec<Cell>, OperatingError> {
        let (elements, mut symbols) = tree_of(names, token_kind);
        let never = |_| Err(OperatingError::NotRead);
        Ok(fixtures::requests::run_prepared(
            &engine,
            &prepared,
            b"abcd",
            &elements,
            &mut symbols,
            &never,
        )?
        .cells)
    };
    // In the first language the token is a `word` numbered 2; in the second, `word` is numbered 3 and number 2 is `other`.
    let first = found(&[(1, "file"), (2, "word")], 2)?;
    let second = found(&[(1, "file"), (2, "other"), (3, "word")], 3)?;
    let wrong = found(&[(1, "file"), (2, "other"), (3, "word")], 2)?;
    let again = found(&[(1, "file"), (2, "word")], 2)?;
    Ok(Observed::Found(
        [
            ("first", first),
            ("second", second),
            ("wrong", wrong),
            ("again", again),
        ]
        .into_iter()
        .map(|(label, cells)| (label, positions(&cells)))
        .collect(),
    ))
}

/// Only an equality-like test of the kind column against a text names a kind, and a name satisfies a test by its text.
pub(super) fn names_a_kind_by_a_test_on_its_name() -> Observed {
    let verdict = |test, text, pattern| {
        satisfies(
            test,
            TextView(text),
            TextView(pattern),
            ryo_execution_runtime::application::file_dag::fixtures::pattern_matcher(),
        )
    };
    Observed::Naming {
        naming: [
            names_a_kind(&compared(Test::Equal, 1, "word")),
            names_a_kind(&compared(Test::Equal, 2, "word")),
            names_a_kind(&compared(Test::LessThan, 1, "word")),
        ],
        verdicts: [
            verdict(Test::Equal, "word", "word"),
            verdict(Test::NotEqual, "word", "word"),
            verdict(Test::StartsWith, "word", "wo"),
            verdict(Test::GlobAny, "word", "{file,word}"),
        ],
    }
}

/// A selection by `test` of `text` on the kind name of a root `file` over one `word` token.
pub(super) fn masked(test: Test, text: &str) -> Result<Observed, OperatingError> {
    let (elements, mut symbols) = tree_of(&[(1, "file"), (2, "word")], 2);
    let operation = Operation::Select {
        from: FROM,
        comparisons: vec![compared(test, 1, text)],
        kept: vec![column(0), column(1), column(0)],
    };
    let out = fixtures::requests::published_file(
        inputs::plan(vec![operation]),
        vec![],
        vec![],
        b"abcd",
        &elements,
        &mut symbols,
    )?;
    Ok(Observed::Masked {
        nodes: positions(&out.cells),
        names: out
            .cells
            .iter()
            .filter(|cell| matches!(cell, Cell::Symbol(_)))
            .map(|cell| Cow::Owned(inputs::text(*cell, &symbols).to_string()))
            .collect(),
        rows: out.count.0,
        kinds: out.kinds.into(),
    })
}

/// The positions of the node cells, in order.
fn positions(cells: &[Cell]) -> Cow<'static, [u32]> {
    cells
        .iter()
        .filter_map(|cell| {
            if let Cell::Node(node) = cell {
                Some(node_position(*node).0)
            } else {
                None
            }
        })
        .collect()
}
