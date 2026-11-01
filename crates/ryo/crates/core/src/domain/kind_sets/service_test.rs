//! The `kind_sets` service's cases: the elements of a named kind are found among that kind, and a file nothing read teaches nothing.

use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::relations::service::node_id;
use ryo_language::domain::relations::types::{
    FileId, KindBits, Offset, Position, Relation, Value as Cell,
};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::terms::types::NameText;

use super::service::{names_a_kind, satisfies};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::KindNaming;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::interface::OperatorsItem as _;
use crate::domain::operator::collections::operators::select::item::SelectItem;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::{Rows, Table, Verdict};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::{SourceView, TextView};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;

/// A scenario: what a row observes of the kind sets, as a number.
type Input = fn() -> Result<Value, OperatingError>;
/// The number a scenario observes: one when every observation held.
type Value = usize;
type Failure = OperatingError;

const FROM: PlanInput = PlanInput::Relation(Relation::Function);

/// The comparison of the column at `place` with the text `text` by `test`.
fn compared(test: Test, place: u32, literal: &str) -> Comparison {
    Comparison {
        test,
        left: Operand::Column(column(place)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(literal.into()))),
        origin: None,
    }
}

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

/// The elements of a tree filtered by the name of their kind and by their text are found among the elements of that kind.
fn finds_the_elements_of_a_named_kind() -> Result<Value, OperatingError> {
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
    let tree = TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    };
    let kind_is = |name: &str| compared(Test::Equal, 1, name);
    let kind_is_not = |name: &str| compared(Test::NotEqual, 1, name);
    let kind_glob = |test: Test, pattern: &str| compared(test, 1, pattern);
    let text_is = |text: &str| compared(Test::Equal, 2, text);
    let mut scratch = OperatorScratch::default();
    let mut found = |comparisons: Vec<Comparison>| -> Result<Vec<Cell>, OperatingError> {
        let mut out = Table::default();
        let mut context = OperatorContext {
            stored_indexes: None,
            bindings: &[],
            symbols: &mut symbols,
            source: SourceView(source),
            paths: &[],
            arenas: &[],
            memo: &mut Vec::new(),
            scratch: &mut scratch,
        };
        SelectItem.compute(
            &Operation::Select {
                from: FROM,
                comparisons,
                kept: vec![column(0)],
            },
            &[Rows::Tree(tree)],
            &mut context,
            &mut out,
        )?;
        Ok(out.cells)
    };
    let id = |at: u32| Cell::Node(node_id(FileId(0), Position(at)));
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
    Ok(usize::from(
        nodes == [id(0)]
            && tokens == [id(1), id(2), id(3)]
            && one_of == words
            && in_a_list == [id(0), id(2)]
            && not_in_a_list == words
            && a_prefix == words
            && words == [id(1), id(3)]
            && a_word == [id(3)]
            && none.is_empty()
            && others == [id(0), id(2)]
            && neither == [id(2)]
            && a_word_that_is_not_the_file == words,
    ))
}

/// A file nothing read has no tree and no names: asking the kinds of it what a name means must not teach the
/// scratch that no kind has the name, for the files that come after.
fn a_file_with_no_names_teaches_nothing_about_kinds() -> Result<Value, OperatingError> {
    let kind_is = Comparison {
        test: Test::Equal,
        left: Operand::Column(column(1)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
        origin: None,
    };
    let operation = Operation::Select {
        from: FROM,
        comparisons: vec![kind_is],
        kept: vec![column(0)],
    };
    let mut scratch = OperatorScratch::default();
    let empty = Elements::default();
    let mut symbols = Symbols::default();
    let mut out = Table::default();
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut scratch,
    };
    SelectItem.compute(
        &operation,
        &[Rows::Tree(TreeRows {
            elements: &empty,
            file: FileId(0),
            form: TreeForm::Elements,
        })],
        &mut context,
        &mut out,
    )?;
    let before = out.count.0;
    let (mut built_elements, mut built_symbols) = (Elements::default(), Symbols::default());
    let source = b"abcd";
    let mut builder = table_builder(&mut built_elements, &mut built_symbols, ByteSlice(source));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    leave_node(&mut builder, Offset(4));
    for (bits, name) in [(1, "file"), (2, "word")] {
        name_kind(
            &mut completion(
                &mut built_elements,
                &mut built_symbols,
                ByteSlice(source),
                FileId(0),
            ),
            KindBits(bits),
            NameText(name),
        );
    }
    index_kinds(&mut built_elements);
    let mut built_context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut built_symbols,
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut scratch,
    };
    SelectItem.compute(
        &operation,
        &[Rows::Tree(TreeRows {
            elements: &built_elements,
            file: FileId(0),
            form: TreeForm::Elements,
        })],
        &mut built_context,
        &mut out,
    )?;
    Ok(before * 10 + out.count.0)
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
fn another_language_does_not_inherit_the_sets_of_the_first() -> Result<Value, OperatingError> {
    let operation = Operation::Select {
        from: FROM,
        comparisons: vec![Comparison {
            test: Test::Equal,
            left: Operand::Column(column(1)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
            origin: None,
        }],
        kept: vec![column(0)],
    };
    let mut scratch = OperatorScratch::default();
    let mut found = |names: &[(u16, &str)], token_kind: u16| -> Result<Vec<Cell>, OperatingError> {
        let (elements, mut symbols) = tree_of(names, token_kind);
        let mut out = Table::default();
        let mut context = OperatorContext {
            stored_indexes: None,
            bindings: &[],
            symbols: &mut symbols,
            source: SourceView(b"abcd"),
            paths: &[],
            arenas: &[],
            memo: &mut Vec::new(),
            scratch: &mut scratch,
        };
        SelectItem.compute(
            &operation,
            &[Rows::Tree(TreeRows {
                elements: &elements,
                file: FileId(0),
                form: TreeForm::Elements,
            })],
            &mut context,
            &mut out,
        )?;
        Ok(out.cells)
    };
    let id = |at: u32| Cell::Node(node_id(FileId(0), Position(at)));
    // In the first language the token is a `word` numbered 2; in the second, `word` is numbered 3 and number 2 is `other`.
    let first = found(&[(1, "file"), (2, "word")], 2)?;
    let second = found(&[(1, "file"), (2, "other"), (3, "word")], 3)?;
    let wrong = found(&[(1, "file"), (2, "other"), (3, "word")], 2)?;
    let again = found(&[(1, "file"), (2, "word")], 2)?;
    Ok(usize::from(
        first == [id(1)] && second == [id(1)] && wrong.is_empty() && again == [id(1)],
    ))
}

/// Only an equality-like test of the kind column against a text names a kind, and a name satisfies a test by its text.
fn names_a_kind_by_a_test_on_its_name() -> Value {
    let by_name = compared(Test::Equal, 1, "word");
    let by_text = compared(Test::Equal, 2, "word");
    let by_order = compared(Test::LessThan, 1, "word");
    let naming = names_a_kind(&by_name) == KindNaming::Names
        && names_a_kind(&by_text) == KindNaming::Silent
        && names_a_kind(&by_order) == KindNaming::Silent;
    let satisfying = satisfies(Test::Equal, TextView("word"), TextView("word")) == Verdict::Holds
        && satisfies(Test::NotEqual, TextView("word"), TextView("word")) == Verdict::Fails
        && satisfies(Test::StartsWith, TextView("word"), TextView("wo")) == Verdict::Holds
        && satisfies(Test::GlobAny, TextView("word"), TextView("{file,word}")) == Verdict::Holds;
    usize::from(naming && satisfying)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_test_on_the_name_of_a_kind_names_a_kind"),
        word: Word::Succeeds,
        input: || || Ok(names_a_kind_by_a_test_on_its_name()),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_elements_of_a_named_kind_are_found_among_that_kind"),
        word: Word::Succeeds,
        input: || finds_the_elements_of_a_named_kind,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("another_language_does_not_inherit_the_sets_of_the_first"),
        word: Word::Succeeds,
        input: || another_language_does_not_inherit_the_sets_of_the_first,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_with_no_names_teaches_nothing_about_kinds"),
        word: Word::Succeeds,
        input: || a_file_with_no_names_teaches_nothing_about_kinds,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
