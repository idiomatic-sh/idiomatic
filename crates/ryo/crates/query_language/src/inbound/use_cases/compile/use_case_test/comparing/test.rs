//! The `comparing` service's cases, on query text read by the real syntax adapter: a comparison is accepted or refused by the types of its sides.

use ryo_relations::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use crate::domain::comparing::service::comparison_of;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Addressable, Builder, Sourced};
use crate::domain::resolving::service::read_source;
use crate::domain::syntax::constants::parameter_kind_named;
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::program::types::{Condition, Expression, QueryView};

/// The query text a row gives.
type Input = &'static str;
/// How many comparisons the `where` of the row made.
type Value = usize;
/// What the lowering refuses a row with.
type Failure = LoweringError;

const FILTERED: &str =
    "let a = select * from \"function\" where name = \"main\"; emit a subject file key(span);";
const UNLIKE: &str =
    "let a = select * from \"function\" where name = 5; emit a subject file key(span);";
const TEXT_ORDERED: &str =
    "let a = select * from \"function\" where name < \"main\"; emit a subject file key(span);";

const NAMED_FOR_ITS_FILE: &str = "let a = select * from \"function\" where name = concat(pascal(file), \"Service\") and starts_with(snake(name), kebab(camel(file))); emit a subject file key(span);";
const AFFIXES: &str = "let a = select * from \"function\" where starts_with(file, \"src/\") and ends_with(name, file); emit a subject file key(span);";
const FUNCTION_OF_A_NUMBER: &str =
    "let a = select * from \"function\" where name = pascal(5); emit a subject file key(span);";
const AFFIX_OF_A_NUMBER: &str =
    "let a = select * from \"function\" where starts_with(name, 5); emit a subject file key(span);";
const CONCAT_OF_ONE: &str =
    "let a = select * from \"function\" where name = concat(file); emit a subject file key(span);";
const PASCAL_OF_TWO: &str = "let a = select * from \"function\" where name = pascal(file, name); emit a subject file key(span);";
const FUNCTION_COMPARED_WITH_A_NUMBER: &str = "let a = select * from \"function\" where concat(name, file) = 5; emit a subject file key(span);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("natural_order_accepts_texts"),
        word: Word::Succeeds,
        input: || "param limit: string; let a = select * from \"function\" where natural_lt(name, $limit); emit a subject file key(span);",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("natural_order_refuses_integer_operands"),
        word: Word::Refuses,
        input: || "let a = select * from \"entry\" where natural_lt(integer, 10); emit a subject file key(span);",
        expect: Expect::Refusal(|error| selects!(matches!(error, LoweringError::NotText { .. }))),
    },
    Case {
        name: CaseName("successor_accepts_existing_integer_and_numeric_text"),
        word: Word::Succeeds,
        input: || "let a = select * from \"numeric-literal\" where integer_successor(value, 43) and integer_successor(41, value); emit a subject file key(id);",
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("successor_refuses_flags"),
        word: Word::Refuses,
        input: || "let a = select * from \"literal-properties\" where integer_successor(boolean, 1); emit a subject file key(id);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("numeric_text_to_integer"),
        word: Word::Succeeds,
        input: || "let a = select * from \"numeric-literal\" where numeric_gte(value, 86400); emit a subject file key(span);",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("numeric_integer_to_text"),
        word: Word::Succeeds,
        input: || "let a = select * from \"numeric-literal\" where numeric_gte(86400, value); emit a subject file key(span);",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("numeric_integer_to_integer"),
        word: Word::Succeeds,
        input: || "let a = select * from \"entry\" where numeric_gte(integer, 3); emit a subject file key(span);",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("numeric_text_to_text"),
        word: Word::Succeeds,
        input: || "let a = select * from \"numeric-literal\" where numeric_gte(value, \"9007199254740993\"); emit a subject file key(span);",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("numeric_flags_refused"),
        word: Word::Refuses,
        input: || "let a = select * from \"literal-properties\" where numeric_gte(boolean, 1); emit a subject file key(span);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("numeric_lists_refused"),
        word: Word::Refuses,
        input: || "param limit: list<string>; let a = select * from \"numeric-literal\" where numeric_gte(value, $limit); emit a subject file key(span);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("invalid_literal_regex_is_refused_before_rows_are_available"),
        word: Word::Refuses,
        input: || r#"let a = select * from "comment" where matches(text, "["); emit a subject file key(span);"#,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, LoweringError::InvalidRegex { reason, .. } if !reason.is_empty()),
            )
        }),
    },
    Case {
        name: CaseName("a_filter_is_one_select_step"),
        word: Word::Succeeds,
        input: || FILTERED,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_name_is_ordered_against_a_text"),
        word: Word::Succeeds,
        input: || TEXT_ORDERED,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_column_is_compared_with_functions_of_another_column"),
        word: Word::Succeeds,
        input: || NAMED_FOR_ITS_FILE,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("starts_with_and_ends_with_take_a_column_or_a_literal_on_either_side"),
        word: Word::Succeeds,
        input: || AFFIXES,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_function_of_a_number_is_refused_naming_the_function_and_argument"),
        word: Word::Refuses,
        input: || FUNCTION_OF_A_NUMBER,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::NotText { function, argument: 1, .. } if function == "pascal"
            ))
        }),
    },
    Case {
        name: CaseName("starts_with_a_number_is_refused_naming_the_test_and_argument"),
        word: Word::Refuses,
        input: || AFFIX_OF_A_NUMBER,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::NotText { function, argument: 2, .. } if function == "starts_with"
            ))
        }),
    },
    Case {
        name: CaseName("concat_of_one_argument_is_refused"),
        word: Word::Refuses,
        input: || CONCAT_OF_ONE,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::FunctionArity { function, .. } if function == "concat"
            ))
        }),
    },
    Case {
        name: CaseName("a_case_of_two_arguments_is_refused"),
        word: Word::Refuses,
        input: || PASCAL_OF_TWO,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::FunctionArity { function, .. } if function == "pascal"
            ))
        }),
    },
    Case {
        name: CaseName("a_function_compared_with_a_number_is_refused"),
        word: Word::Refuses,
        input: || FUNCTION_COMPARED_WITH_A_NUMBER,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("comparing_a_name_with_a_number_is_refused"),
        word: Word::Refuses,
        input: || UNLIKE,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
];

/// The relations the rows may read.
const CATALOG: &[Relation] = &[
    Relation::NumericLiteral,
    Relation::LiteralProperties,
    Relation::Defines,
    Relation::Function,
    Relation::SyntaxChild,
    Relation::StringLiteral,
    Relation::SyntaxTree,
    Relation::WorkspaceMember,
    Relation::Entry,
    Relation::SequenceElement,
    Relation::PackageDependency,
    Relation::Comment,
];

/// Compares each test of each `where` with the part's own comparison and counts the comparisons made.
fn subject(text: Input) -> Result<Value, Failure> {
    let program = winnow()
        .read(QueryView(text))
        .expect("the row's text is readable");
    let mut builder = Builder {
        catalog: CATALOG,
        observation_inputs: &[],
        hosted_steps: Vec::new(),
        parameters: &program.parameters,
        kinds: program
            .parameters
            .iter()
            .map(|parameter| {
                parameter_kind_named(&parameter.kind.0).expect("the row's parameter type is known")
            })
            .collect(),
        steps: Vec::new(),
        lowered: Vec::new(),
        span: program.emit.span,
        origin: None,
        comparison_origins: Vec::new(),
    };
    let mut compared = 0;
    for step in &program.steps {
        if let Expression::Select(select) = &step.expression {
            let Sourced { fields, .. } = read_source(&builder, &select.from)?;
            for condition in &select.filters {
                if let Condition::Test(filter) = condition {
                    let columns = Addressable {
                        fields: &fields,
                        sides: &[],
                    };
                    let PlanComparison { .. } = comparison_of(&mut builder, filter, columns)?;
                    compared += 1;
                }
            }
        }
    }
    Ok(compared)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
