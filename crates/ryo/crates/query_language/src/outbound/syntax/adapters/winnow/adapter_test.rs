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
    Program, QueryBundle, QueryView, SourceReference, WordText, WrittenFunction, WrittenLiteral,
    WrittenOperand, WrittenParameter, WrittenRelation, WrittenTest,
};
use winnow::Parser;

const FRAGMENT_DOCUMENT: &str = "import nested from \"shared/predicate\";\ninput candidates: relation<id: node, file: file, span: span, line: number>;\nparam inline-pattern: string;\nparam range-pattern: string;\noutput relation<id: node, file: file, span: span>;\nlet violations = select id, file, span from candidates;\nreturn violations;";

const LONG_FILES: &str = "param max-lines: integer;\nlet long = select * from \"filesystem.line-count\" where lines > $max-lines;\nemit long subject path key(path);";

const IFS_IN_FUNCTIONS: &str = "let functions = select * from \"syntax-tree\" where kind = \"fn\";\nlet descendants = axis(from=functions, relation=\"syntax-child\", direction=descendant, domain=\"syntax-tree\");\nlet nodes = select r.\"id\" as \"function\", l.\"id\" as \"id\" from \"syntax-tree\" as l join descendants as r on l.\"id\" = r.\"related\";\nemit nodes subject \"file\" key(\"span\");";

const ANTIJOIN: &str = "let trait-files = project(from=traits, columns=[\"path\"]);\nlet free = antijoin(left=functions, right=trait-files, left-on=[\"path\"], right-on=[\"path\"]);\nemit free subject \"path\" key(\"span\");";

const MISSPELLED: &str = "let a = selekt * from \"x\"; emit a subject path key(path);";
const TWO_EMITS: &str =
    "let a = select * from \"x\"; emit a subject path key(path); emit a subject path key(path);";
const FLOATS: &str = "let floats = select * from \"syntax-tree\" where float_text_gte(text, $limit);\nemit floats subject \"file\" key(\"span\");";

const FUNCTIONS: &str = "let named = select * from \"defines\" where starts_with(name, \"Todo\") and name = concat(pascal(pkg), \"Service\") and ends_with(snake(name), kebab( camel(pkg) )) and matches(name, \"(?i)^todo\");\nemit named subject \"file\" key(\"span\");";

const OR: &str = "let a = select * from \"syntax-tree\" where kind = \"a\" or kind = \"b\" and line > 1;\nemit a subject \"file\" key(\"span\");";

const GROUPED: &str = "let a = select * from \"syntax-tree\" where (kind = \"a\" or kind = \"b\") and line > 1 and (line < 9 and line != 4);\nemit a subject \"file\" key(\"span\");";

const UNCLOSED: &str = "let a = select * from \"syntax-tree\" where (kind = \"a\" or kind = \"b\";\nemit a subject \"file\" key(\"span\");";

const LIST_AFTER_IN: &str = "let found = select * from \"function\" where name in [\"a\", \"b\"]; emit found subject \"file\" key(\"span\");";
/// Where the list of [`LIST_AFTER_IN`] begins.
const LIST_AT: usize = 51;

const DANGLING_OR: &str = "let a = select * from \"syntax-tree\" where kind = \"a\" or;\nemit a subject \"file\" key(\"span\");";

fn parse_and_link(bundle: &QueryBundle) -> Result<Program, String> {
    let parsed = SyntaxWinnowAdapter
        .read_bundle(bundle)
        .map_err(|error| error.to_string())?;
    crate::domain::linking::service::link(bundle, parsed.root, parsed.fragments)
        .map_err(|error| error.message)
}

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

#[test]
fn fragment_calls_are_hygienic_and_only_relation_slots_are_linked() {
    use crate::outbound::syntax::interface::SyntaxPort;
    use ryo_values::domain::program::types::ArgumentValue;

    let root_id = ryo_values::domain::program::service::source_id("root").unwrap();
    let module_id = ryo_values::domain::program::service::source_id("library/summarize").unwrap();
    let root = "import summarize from \"library/summarize\";\nlet id = select id from \"syntax-tree\";\nlet first = summarize(id=id, label=\"first\");\nlet second = summarize(id=id, label=\"second\");\nemit second subject id key(id);";
    let module = "input id: relation<id: node>;\nparam label: string;\noutput relation<id: node>;\nlet grouped = group-count(from=id, keys=[\"id\"], count=\"n\");\nlet linked = closure(edges=grouped, source=id, destination=grouped);\nlet matched = capture(from=linked, path=\"path\", pattern=$label, captures=[\"group\"]);\nreturn matched;";
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(root_id, root),
        [ryo_values::domain::program::service::document_source(
            module_id, module,
        )],
    )
    .unwrap();
    let parsed = SyntaxWinnowAdapter.read_bundle(&bundle).unwrap();
    let program =
        crate::domain::linking::service::link(&bundle, parsed.root, parsed.fragments).unwrap();

    assert_eq!(program.steps.len(), 7);
    let grouped = [&program.steps[1], &program.steps[4]];
    let linked = [&program.steps[2], &program.steps[5]];
    let matched = [&program.steps[3], &program.steps[6]];
    let mut group_ids = Vec::new();
    for step in grouped {
        let Expression::Call(call) = &step.expression else {
            panic!("fragment group-count step is a call");
        };
        let from = call
            .arguments
            .iter()
            .find(|argument| argument.name.0.as_ref() == "from")
            .unwrap();
        assert!(
            matches!(&from.value, ArgumentValue::Step(name) if name.1 == Some(program.steps[0].name.1.unwrap()))
        );
        let keys = call
            .arguments
            .iter()
            .find(|argument| argument.name.0.as_ref() == "keys")
            .unwrap();
        assert_eq!(
            keys.value,
            ArgumentValue::List(vec![ArgumentValue::Text(WrittenLiteral("id".into()))])
        );
        group_ids.push(step.name.1.unwrap());
    }
    assert_ne!(group_ids[0], group_ids[1]);

    for (place, step) in linked.iter().enumerate() {
        let Expression::Call(call) = &step.expression else {
            panic!("fragment closure step is a call");
        };
        let argument = |name: &str| {
            call.arguments
                .iter()
                .find(|argument| argument.name.0.as_ref() == name)
                .unwrap()
        };
        assert_eq!(
            argument("edges").value,
            ArgumentValue::Step(ryo_values::domain::program::types::StepName(
                "grouped".into(),
                Some(group_ids[place])
            ))
        );
        assert_eq!(
            argument("source").value,
            ArgumentValue::Word(WordText("id".into()))
        );
        assert_eq!(
            argument("destination").value,
            ArgumentValue::Word(WordText("grouped".into()))
        );
    }

    let patterns = matched
        .iter()
        .map(|step| {
            let Expression::Call(call) = &step.expression else {
                panic!("fragment capture step is a call");
            };
            call.arguments
                .iter()
                .find(|argument| argument.name.0.as_ref() == "pattern")
                .unwrap()
                .value
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        patterns,
        [
            ArgumentValue::Text(WrittenLiteral("first".into())),
            ArgumentValue::Text(WrittenLiteral("second".into())),
        ]
    );
    let sources = matched
        .iter()
        .map(|step| {
            let Expression::Call(call) = &step.expression else {
                unreachable!()
            };
            call.arguments
                .iter()
                .find(|argument| argument.name.0.as_ref() == "from")
                .unwrap()
                .value
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        sources,
        [
            ArgumentValue::Step(ryo_values::domain::program::types::StepName(
                "linked".into(),
                Some(linked[0].name.1.unwrap())
            )),
            ArgumentValue::Step(ryo_values::domain::program::types::StepName(
                "linked".into(),
                Some(linked[1].name.1.unwrap())
            )),
        ]
    );
}

#[test]
fn nested_fragments_forward_relation_and_scalar_inputs() {
    use crate::outbound::syntax::interface::SyntaxPort;
    use ryo_values::domain::program::types::{ArgumentValue, ParsedQueryBundle};

    let root_id = ryo_values::domain::program::service::source_id("root").unwrap();
    let outer_id = ryo_values::domain::program::service::source_id("outer").unwrap();
    let inner_id = ryo_values::domain::program::service::source_id("inner").unwrap();
    let root = "import outer from \"outer\";\nlet source = select id from \"syntax-tree\";\nlet first = outer(source=source, label=\"a\");\nlet second = outer(source=source, label=\"b\");\nemit second subject id key(id);";
    let outer = "import inner from \"inner\";\ninput source: relation<id: node>;\nparam label: string;\noutput relation<id: node>;\nlet found = inner(rows=source, label=$label);\nreturn found;";
    let inner = "input rows: relation<id: node>;\nparam label: string;\noutput relation<id: node>;\nlet found = capture(from=rows, path=\"id\", pattern=$label, captures=[\"hit\"]);\nreturn found;";
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(root_id, root),
        [
            ryo_values::domain::program::service::document_source(outer_id, outer),
            ryo_values::domain::program::service::document_source(inner_id, inner),
        ],
    )
    .unwrap();
    let ParsedQueryBundle { root, fragments } = SyntaxWinnowAdapter.read_bundle(&bundle).unwrap();
    let program = crate::domain::linking::service::link(&bundle, root, fragments).unwrap();
    assert_eq!(program.steps.len(), 3);
    for (step, pattern) in program.steps[1..].iter().zip(["a", "b"]) {
        let Expression::Call(call) = &step.expression else {
            panic!("the nested fragment expands to capture");
        };
        let source = call
            .arguments
            .iter()
            .find(|argument| argument.name.0.as_ref() == "from")
            .unwrap();
        assert!(
            matches!(&source.value, ArgumentValue::Step(name) if name.1 == Some(program.steps[0].name.1.unwrap()))
        );
        let value = call
            .arguments
            .iter()
            .find(|argument| argument.name.0.as_ref() == "pattern")
            .unwrap();
        assert_eq!(
            value.value,
            ArgumentValue::Text(WrittenLiteral(pattern.into()))
        );
    }
}

#[test]
fn imported_module_cycle_is_refused() {
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("root").unwrap(),
            "import query from \"query\";\nlet result = query();\nemit result subject id key(id);",
        ),
        [ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("query").unwrap(),
            "import recursive from \"query\";\noutput relation<id: node>;\nlet rows = select id from \"syntax-tree\";\nreturn rows;",
        )],
    )
    .unwrap();
    assert!(
        parse_and_link(&bundle)
            .unwrap_err()
            .contains("import cycle")
    );
}

#[test]
fn imported_module_must_be_present_and_arguments_must_be_unique() {
    let missing = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("root").unwrap(),
            "import query from \"missing\";\nlet result = query();\nemit result subject id key(id);",
        ),
        [],
    )
    .unwrap();
    assert!(
        parse_and_link(&missing)
            .unwrap_err()
            .contains("was not supplied")
    );

    let repeated = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("root").unwrap(),
            "import query from \"query\";\nlet result = query(source=\"syntax-tree\", source=\"syntax-tree\");\nemit result subject id key(id);",
        ),
        [ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("query").unwrap(),
            "input source: relation<id: node>;\noutput relation<id: node>;\nlet rows = select id from source;\nreturn rows;",
        )],
    )
    .unwrap();
    assert!(
        parse_and_link(&repeated)
            .unwrap_err()
            .contains("given more than once")
    );
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
    assert_eq!(test_at(select, 3).test, WrittenTest::Matches);
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
const KEYWORDS: [Keyword; 30] = [
    Keyword::Param,
    Keyword::Import,
    Keyword::Input,
    Keyword::Output,
    Keyword::Return,
    Keyword::Relation,
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
    Keyword::Matches,
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
        "fragment_program" => whole(parsers::fragment_program, text),
        "fragment_line" => whole(parsers::fragment_line, text),
        "fragment_input_line" => whole(parsers::fragment_input_line, text),
        "fragment_output_line" => whole(parsers::fragment_output_line, text),
        "relation_signature" => whole(parsers::relation_signature, text),
        "fragment_return_line" => whole(parsers::fragment_return_line, text),
        "line" => whole(parsers::line, text),
        "import_line" => whole(parsers::import_line, text),
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

#[test]
fn fragment_document_reads_typed_signature_nested_import_and_return() {
    use winnow::stream::LocatingSlice;

    let parsed = super::adapter::fragment_program
        .parse(LocatingSlice::new(FRAGMENT_DOCUMENT))
        .expect("fragment has a signature and result");
    assert_eq!(parsed.imports.len(), 1);
    assert_eq!(parsed.imports[0].alias.0.as_ref(), "nested");
    assert_eq!(parsed.imports[0].module.0.as_ref(), "shared/predicate");
    assert_eq!(parsed.inputs.len(), 1);
    assert_eq!(parsed.inputs[0].relation.columns.len(), 4);
    assert_eq!(parsed.parameters.len(), 2);
    assert_eq!(parsed.output.columns.len(), 3);
    assert_eq!(parsed.result.step.0.as_ref(), "violations");

    let root = SyntaxWinnowAdapter
        .read(QueryView(
            "import suppress from \"python/ruff/suppress\"; let rows = select id from candidates; emit rows subject file key(id);",
        ))
        .expect("root imports parse alongside query statements");
    assert_eq!(root.imports.len(), 1);
    assert_eq!(root.imports[0].alias.0.as_ref(), "suppress");
    assert_eq!(root.imports[0].module.0.as_ref(), "python/ruff/suppress");
}

#[test]
fn source_names_keep_written_spelling_separate_from_definition_identity() {
    let parsed = SyntaxWinnowAdapter
        .read(QueryView(
            "let rows = select * from \"function\"; let copy = select * from rows; emit copy subject file key(span);",
        ))
        .expect("the query parses");
    let first = parsed.steps[0].name.1.expect("definition has identity");
    let second = parsed.steps[1].name.1.expect("definition has identity");
    assert_ne!(first, second);
    assert_eq!(parsed.steps[0].name.0.as_ref(), "rows");
    let Expression::Select(select) = &parsed.steps[1].expression else {
        panic!("the second step is a select");
    };
    let SourceReference::Step(reference) = &select.from.reference else {
        panic!("the second step reads the first step");
    };
    assert_eq!(reference.0.as_ref(), "rows");
    assert_eq!(reference.1, Some(first));
    assert_eq!(parsed.emit.step.1, Some(second));
}
