// ignore: idiomatic.arch/file-length - the one test file the adapter template places for the winnow grammar: its parse cases and the check that the grammar table covers every parser fn, keyword and mark; a split would be a file no adapter template places
//! The winnow adapter of the syntax port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::SyntaxWinnowAdapter;
use super::constants::{GRAMMAR, Keyword, Mark, keyword_text, mark_text};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::syntax::errors::SyntaxError;
use crate::outbound::syntax::fixtures::contract::check;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::program::types::{
    ArgumentValue as Argument, ColumnName, Columns, Condition, Expression, Filter, Joining,
    Program, QueryView, SourceReference, WordText, WrittenFunction, WrittenLiteral, WrittenOperand,
    WrittenParameter, WrittenRelation, WrittenTest,
};

const LONG_FILES: &str = "param max-lines: integer;\nlet long = select * from \"filesystem.line-count\" where lines > $max-lines;\nemit long subject path key(path);";

const IFS_IN_FUNCTIONS: &str = "let functions = select * from \"syntax-tree\" where kind = \"fn\";\nlet descendants = axis(from=functions, relation=\"syntax-child\", direction=descendant, domain=\"syntax-tree\");\nlet nodes = select r.\"id\" as \"function\", l.\"id\" as \"id\" from \"syntax-tree\" as l join descendants as r on l.\"id\" = r.\"related\";\nemit nodes subject \"file\" key(\"span\");";

const ANTIJOIN: &str = "let trait-files = project(from=traits, columns=[\"path\"]);\nlet free = antijoin(left=functions, right=trait-files, left-on=[\"path\"], right-on=[\"path\"]);\nemit free subject \"path\" key(\"span\");";

const MISSPELLED: &str = "let a = selekt * from \"x\"; emit a subject path key(path);";
const TWO_EMITS: &str =
    "let a = select * from \"x\"; emit a subject path key(path); emit a subject path key(path);";
const FLOATS: &str = "let floats = select * from \"syntax-tree\" where float_text_gte(text, $limit);\nemit floats subject \"file\" key(\"span\");";

const FUNCTIONS: &str = "let named = select * from \"defines\" where starts_with(name, \"Todo\") and name = concat(pascal(pkg), \"Service\") and ends_with(snake(name), kebab( camel(pkg) ));\nemit named subject \"file\" key(\"span\");";

const OR: &str = "let a = select * from \"syntax-tree\" where kind = \"a\" or kind = \"b\" and line > 1;\nemit a subject \"file\" key(\"span\");";

const GROUPED: &str = "let a = select * from \"syntax-tree\" where (kind = \"a\" or kind = \"b\") and line > 1 and (line < 9 and line != 4);\nemit a subject \"file\" key(\"span\");";

const UNCLOSED: &str = "let a = select * from \"syntax-tree\" where (kind = \"a\" or kind = \"b\";\nemit a subject \"file\" key(\"span\");";

const LIST_AFTER_IN: &str = "let found = select * from \"function\" where name in [\"a\", \"b\"]; emit found subject \"file\" key(\"span\");";
/// Where the list of [`LIST_AFTER_IN`] begins.
const LIST_AT: usize = 51;

const DANGLING_OR: &str = "let a = select * from \"syntax-tree\" where kind = \"a\" or;\nemit a subject \"file\" key(\"span\");";

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// Reads `or` and `and` without parentheses: `and` binds tighter.
    OrPrecedence,
    /// Reads conditions in parentheses: an `or` in a group, and a group with no `or`.
    Grouped,
    /// Reads a group whose parenthesis is not closed.
    Unclosed,
    /// Reads an `or` with nothing after it.
    DanglingOr,
    /// Reads `starts_with`, `ends_with` and functions nested in each other.
    FunctionsAndAffixes,
    /// Reads a query with a parameter and a `where` filter.
    ParameterAndFilter,
    /// Reads a `float_text_gte` filter.
    FloatTextFilter,
    /// Reads a call, then a select with a join.
    CallAndJoin,
    /// Reads a call whose argument is a list of strings.
    ListsOfStrings,
    /// Reads text with a misspelled keyword.
    MisspelledKeyword,
    /// Reads text with two `emit` lines.
    TwoEmits,
    /// Reads a list written after `in`.
    ListAfterIn,
    /// Checks the grammar table: how many of its entries break the check.
    Grammar(Coverage),
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// How many `let` steps the text has.
type Value = usize;
/// What a row expects when the text is refused.
type Failure = SyntaxError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    if let Scenario::Grammar(coverage) = scenario {
        return Ok(uncovered(coverage));
    }
    let text = match scenario {
        Scenario::OrPrecedence => OR,
        Scenario::Grouped => GROUPED,
        Scenario::Unclosed => UNCLOSED,
        Scenario::DanglingOr => DANGLING_OR,
        Scenario::ParameterAndFilter => LONG_FILES,
        Scenario::FloatTextFilter => FLOATS,
        Scenario::FunctionsAndAffixes => FUNCTIONS,
        Scenario::CallAndJoin => IFS_IN_FUNCTIONS,
        Scenario::ListsOfStrings => ANTIJOIN,
        Scenario::MisspelledKeyword | Scenario::Grammar(_) => MISSPELLED,
        Scenario::TwoEmits => TWO_EMITS,
        Scenario::ListAfterIn => LIST_AFTER_IN,
    };
    let program = SyntaxWinnowAdapter.read(QueryView(text))?;
    match scenario {
        Scenario::ParameterAndFilter => parameter_and_filter_read(&program),
        Scenario::FloatTextFilter => float_text_filter_read(&program),
        Scenario::FunctionsAndAffixes => functions_and_affixes_read(&program),
        Scenario::CallAndJoin => call_and_join_read(&program),
        Scenario::ListsOfStrings => lists_of_strings_read(&program),
        Scenario::OrPrecedence => or_precedence_read(&program),
        Scenario::Grouped => grouped_read(&program),
        Scenario::MisspelledKeyword
        | Scenario::TwoEmits
        | Scenario::Unclosed
        | Scenario::DanglingOr
        | Scenario::ListAfterIn
        | Scenario::Grammar(_) => {}
    }
    Ok(program.steps.len())
}

/// The test at `place` of a select's `where`, which the row writes with no `or`.
fn test_at(select: &ryo_values::domain::program::types::Select, place: usize) -> &Filter {
    let Condition::Test(filter) = &select.filters[place] else {
        panic!("the condition is a test");
    };
    filter
}

/// How many tests each alternative of an `or` holds, the alternatives in order.
fn alternatives(condition: &Condition) -> Vec<usize> {
    let Condition::Any(alternatives) = condition else {
        panic!("the condition is an or");
    };
    alternatives.iter().map(Vec::len).collect()
}

/// `a or b and c` of `OR` is one `or` of `a` and of `b and c`.
fn or_precedence_read(program: &Program) {
    let Expression::Select(select) = &program.steps[0].expression else {
        panic!("the step is a select");
    };
    assert_eq!(select.filters.len(), 1);
    assert_eq!(alternatives(&select.filters[0]), vec![1, 2]);
}

/// `(a or b) and c and (d and e)` of `GROUPED` is an `or` and three tests: the group with no `or` is read as its tests.
fn grouped_read(program: &Program) {
    let Expression::Select(select) = &program.steps[0].expression else {
        panic!("the step is a select");
    };
    assert_eq!(select.filters.len(), 4);
    assert_eq!(alternatives(&select.filters[0]), vec![1, 1]);
    assert_eq!(test_at(select, 1).test, WrittenTest::GreaterThan);
    assert_eq!(test_at(select, 2).test, WrittenTest::LessThan);
    assert_eq!(test_at(select, 3).test, WrittenTest::NotEqual);
}

/// The parameter and the `where` filter of `LONG_FILES` are read as written.
fn parameter_and_filter_read(program: &Program) {
    assert_eq!(&*program.parameters[0].name.0, "max-lines");
    assert_eq!(&*program.parameters[0].kind.0, "integer");
    let Expression::Select(select) = &program.steps[0].expression else {
        panic!("the step is a select");
    };
    assert_eq!(
        select.from.reference,
        SourceReference::Relation(WrittenRelation("filesystem.line-count".into()))
    );
    assert_eq!(test_at(select, 0).test, WrittenTest::GreaterThan);
    assert_eq!(
        test_at(select, 0).right,
        WrittenOperand::Parameter(WrittenParameter("max-lines".into()))
    );
    assert_eq!(program.emit.key, vec![ColumnName("path".into())]);
}

/// The float-text filter of `FLOATS` is read with its parameter.
fn float_text_filter_read(program: &Program) {
    let Expression::Select(select) = &program.steps[0].expression else {
        panic!("the step is a select");
    };
    assert_eq!(
        test_at(select, 0).test,
        WrittenTest::FloatTextGreaterThanOrEqual
    );
    assert_eq!(
        test_at(select, 0).right,
        WrittenOperand::Parameter(WrittenParameter("limit".into()))
    );
}

/// The affix filters of `FUNCTIONS` are read with their nested functions.
fn functions_and_affixes_read(program: &Program) {
    let Expression::Select(select) = &program.steps[0].expression else {
        panic!("the step is a select");
    };
    assert_eq!(test_at(select, 0).test, WrittenTest::StartsWith);
    assert_eq!(test_at(select, 1).test, WrittenTest::Equal);
    assert_eq!(test_at(select, 2).test, WrittenTest::EndsWith);
    let WrittenOperand::Function(concat) = &test_at(select, 1).right else {
        panic!("the right side is a function");
    };
    assert_eq!(concat.function, WrittenFunction::Concat);
    assert_eq!(concat.arguments.len(), 2);
    assert!(
        matches!(&concat.arguments[0], WrittenOperand::Function(inner) if inner.function == WrittenFunction::Pascal)
    );
    assert_eq!(
        concat.arguments[1],
        WrittenOperand::Text(WrittenLiteral("Service".into()))
    );
    let WrittenOperand::Function(kebab) = &test_at(select, 2).right else {
        panic!("the right side is a function");
    };
    assert_eq!(kebab.function, WrittenFunction::Kebab);
    assert!(
        matches!(&kebab.arguments[0], WrittenOperand::Function(inner) if inner.function == WrittenFunction::Camel)
    );
}

/// The `axis` call and the join of `IFS_IN_FUNCTIONS` are read.
fn call_and_join_read(program: &Program) {
    let Expression::Call(call) = &program.steps[1].expression else {
        panic!("the step is a call");
    };
    assert_eq!(&*call.name.0, "axis");
    assert_eq!(
        call.arguments[2].value,
        Argument::Word(WordText("descendant".into()))
    );
    let Expression::Select(select) = &program.steps[2].expression else {
        panic!("the step is a select");
    };
    assert!(matches!(&select.columns, Columns::Listed(columns) if columns.len() == 2));
    assert!(matches!(select.join, Joining::Joined(_)));
}

/// The list of texts `ANTIJOIN` gives an argument is read.
fn lists_of_strings_read(program: &Program) {
    let Expression::Call(call) = &program.steps[0].expression else {
        panic!("the step is a call");
    };
    assert_eq!(
        call.arguments[1].value,
        Argument::List(vec![Argument::Text(WrittenLiteral("path".into()))])
    );
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_parameter_and_a_where_filter_are_read"),
        word: Word::Succeeds,
        input: || Scenario::ParameterAndFilter,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_float_text_filter_is_read"),
        word: Word::Succeeds,
        input: || Scenario::FloatTextFilter,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("affix_filters_and_nested_functions_are_read"),
        word: Word::Succeeds,
        input: || Scenario::FunctionsAndAffixes,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_call_and_a_join_are_read"),
        word: Word::Succeeds,
        input: || Scenario::CallAndJoin,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("lists_of_strings_are_read"),
        word: Word::Succeeds,
        input: || Scenario::ListsOfStrings,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("and_binds_tighter_than_or"),
        word: Word::Succeeds,
        input: || Scenario::OrPrecedence,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("parentheses_group_conditions"),
        word: Word::Succeeds,
        input: || Scenario::Grouped,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("every_production_example_is_read_whole_by_its_parser"),
        word: Word::Succeeds,
        input: || Scenario::Grammar(Coverage::ExamplesRead),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("every_keyword_and_mark_is_written_in_a_form"),
        word: Word::Succeeds,
        input: || Scenario::Grammar(Coverage::WordsWritten),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("every_parser_fn_has_one_production"),
        word: Word::Succeeds,
        input: || Scenario::Grammar(Coverage::ParsersListed),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_misspelled_keyword_is_unreadable"),
        word: Word::Refuses,
        input: || Scenario::MisspelledKeyword,
        expect: Expect::Refusal(|error| selects(matches!(error, SyntaxError::Unreadable { .. }))),
    },
    Case {
        name: CaseName("a_second_emit_line_is_unreadable"),
        word: Word::Refuses,
        input: || Scenario::TwoEmits,
        expect: Expect::Refusal(|error| selects(matches!(error, SyntaxError::Unreadable { .. }))),
    },
    Case {
        name: CaseName("an_unclosed_parenthesis_is_unreadable"),
        word: Word::Refuses,
        input: || Scenario::Unclosed,
        expect: Expect::Refusal(|error| selects(matches!(error, SyntaxError::Unreadable { .. }))),
    },
    Case {
        name: CaseName("an_or_with_nothing_after_it_is_unreadable"),
        word: Word::Refuses,
        input: || Scenario::DanglingOr,
        expect: Expect::Refusal(|error| selects(matches!(error, SyntaxError::Unreadable { .. }))),
    },
    Case {
        name: CaseName("a_list_after_in_is_refused_where_it_begins"),
        word: Word::Refuses,
        input: || Scenario::ListAfterIn,
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, SyntaxError::Unreadable { span, .. } if span.start.0 == LIST_AT),
            )
        }),
    },
];

#[test]
fn succeeds() {
    check(&SyntaxWinnowAdapter);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// Every keyword, in the order the grammar meets them.
const KEYWORDS: [Keyword; 24] = [
    Keyword::Param,
    Keyword::Let,
    Keyword::Emit,
    Keyword::Subject,
    Keyword::Key,
    Keyword::Select,
    Keyword::From,
    Keyword::As,
    Keyword::Join,
    Keyword::On,
    Keyword::Where,
    Keyword::And,
    Keyword::Or,
    Keyword::Not,
    Keyword::In,
    Keyword::GlobAny,
    Keyword::FloatTextGte,
    Keyword::StartsWith,
    Keyword::EndsWith,
    Keyword::Concat,
    Keyword::Pascal,
    Keyword::Camel,
    Keyword::Snake,
    Keyword::Kebab,
];

/// Every mark.
const MARKS: [Mark; 3] = [Mark::Different, Mark::AtMost, Mark::AtLeast];

/// What a row of the grammar cases checks of the grammar table.
#[derive(Clone, Copy)]
enum Coverage {
    /// Each production's example read whole by its own parser fn.
    ExamplesRead,
    /// Each keyword and mark written in some production's form.
    WordsWritten,
    /// One production for each parser fn of the adapter, and no other.
    ParsersListed,
}

/// Whether the parser fn `name` reads `text` to its end: 0 when it does, 1 when it does not or no parser fn has the name.
fn misread(name: &str, text: &str) -> usize {
    use super::adapter as parsers;
    use winnow::combinator::{eof, terminated};
    use winnow::prelude::*;
    use winnow::stream::LocatingSlice;
    fn whole<'i, O>(
        parser: impl Parser<
            LocatingSlice<&'i str>,
            O,
            winnow::error::ErrMode<winnow::error::ContextError>,
        >,
        text: &'i str,
    ) -> usize {
        terminated(parser, eof)
            .parse(LocatingSlice::new(text))
            .map_or(1, |_| 0)
    }
    match name {
        "program" => whole(parsers::program, text),
        "line" => whole(parsers::line, text),
        "parameter_line" => whole(parsers::parameter_line, text),
        "step_line" => whole(parsers::step_line, text),
        "emit_line" => whole(parsers::emit_line, text),
        "expression" => whole(parsers::expression, text),
        "select" => whole(parsers::select, text),
        "select_column" => whole(parsers::select_column, text),
        "source" => whole(parsers::source, text),
        "join" => whole(parsers::join, text),
        "join_key" => whole(parsers::join_key, text),
        "filters" => whole(parsers::filters, text),
        "condition" => whole(parsers::condition, text),
        "conjunction" => whole(parsers::conjunction, text),
        "factor" => whole(parsers::factor, text),
        "filter" => whole(parsers::filter, text),
        "float_text_filter" => whole(parsers::float_text_filter, text),
        "glob_filter" => whole(parsers::glob_filter, text),
        "affix_filter" => whole(parsers::affix_filter, text),
        "comparison_filter" => whole(parsers::comparison_filter, text),
        "test" => whole(parsers::test, text),
        "operand" => whole(parsers::operand, text),
        "function" => whole(parsers::function, text),
        "column_reference" => whole(parsers::column_reference, text),
        "call" => whole(parsers::call, text),
        "argument" => whole(parsers::argument, text),
        "value" => whole(parsers::value, text),
        "parameter" => whole(parsers::parameter, text),
        "number" => whole(parsers::number, text),
        "name_or_string" => whole(parsers::name_or_string, text),
        "string" => whole(parsers::string, text),
        "word" => whole(parsers::word, text),
        _ => 1,
    }
}

/// How many entries break the row's check: 0 when the grammar table is whole.
fn uncovered(coverage: Coverage) -> usize {
    match coverage {
        Coverage::ExamplesRead => GRAMMAR
            .iter()
            .map(|line| misread(line.name, line.example))
            .sum(),
        Coverage::WordsWritten => {
            let written: Vec<&str> = GRAMMAR
                .iter()
                .flat_map(|line| line.form.split_whitespace())
                .map(|token| token.trim_matches(|character| character == '"' || character == ','))
                .collect();
            let missing = KEYWORDS
                .iter()
                .map(|keyword| keyword_text(*keyword))
                .chain(MARKS.iter().map(|mark| mark_text(*mark)))
                .filter(|text| !written.contains(text))
                .count();
            // `keyword_text` and `mark_text` match every variant with no wildcard, so their arms count the variants.
            let constants = include_str!("constants.rs");
            let arms = |prefix: &str| {
                constants
                    .lines()
                    .filter(|line| line.trim_start().starts_with(prefix) && line.contains(" => "))
                    .count()
            };
            let unlisted =
                arms("Keyword::").abs_diff(KEYWORDS.len()) + arms("Mark::").abs_diff(MARKS.len());
            missing + unlisted
        }
        Coverage::ParsersListed => {
            let source = include_str!("adapter.rs");
            let parsers: std::collections::BTreeSet<&str> = source
                .lines()
                .filter(|line| line.contains("(input: &mut Input"))
                .filter_map(|line| line.split("fn ").nth(1))
                .filter_map(|rest| rest.split(['(', '<']).next())
                .collect();
            let named: std::collections::BTreeSet<&str> =
                GRAMMAR.iter().map(|line| line.name).collect();
            parsers.symmetric_difference(&named).count() + GRAMMAR.len() - named.len()
        }
    }
}
