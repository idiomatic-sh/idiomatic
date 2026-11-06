//! The `use_case` adapter of the `query_language` port: the core's contract rows run through it, then the cases specific to it.

use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use std::sync::Arc;

use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;

use super::adapter::QueryLanguageUseCaseAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::query_language::fixtures::contract::CASES as CONTRACT;
use ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguageError;
use ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguagePort as _;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::query_tokens::types::QueryBundle;

type Input = QueryText;
type Value = bool;
type Failure = QueryLanguageError;

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("invalid_literal_regex_is_refused_without_reading_any_input"),
        word: Word::Refuses,
        input: || {
            QueryText(r#"let rows = select * from "comment" where matches(text, "["); emit rows subject file key(span);"#.into())
        },
        expect: Expect::Refusal(|error| {
            selects(error.to_string().contains("invalid regular expression"))
        }),
    },
    Case {
        name: CaseName("a_prior_filter_cannot_hide_an_invalid_literal_regex"),
        word: Word::Refuses,
        input: || {
            QueryText(r#"let rows = select * from "comment" where text = "never-present" and matches(text, "["); emit rows subject file key(span);"#.into())
        },
        expect: Expect::Refusal(|error| {
            selects(error.to_string().contains("invalid regular expression"))
        }),
    },
    Case {
        name: CaseName("a_query_whose_step_has_another_name_compiles"),
        word: Word::Succeeds,
        input: || {
            QueryText(
                "let found = select * from \"function\"; emit found subject \"file\" key(\"span\");"
                    .into(),
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_empty_text_is_refused"),
        word: Word::Refuses,
        input: || QueryText("".into()),
        expect: Expect::Refusal(|error| {
            selects(matches!(error, QueryLanguageError::Refused { .. }))
        }),
    },
];

/// Calls the adapter on one input, for the contract's rows and for its own.
fn subject(text: &Input) -> Result<Value, Failure> {
    compile_query(&QueryBundle::from(text.clone())).map(|_| true)
}

/// Uses the real parser and lowering pipeline, including raw string-literal handling.
fn compile_query(bundle: &QueryBundle) -> Result<ryo_values::domain::plans::types::Plan, Failure> {
    let compile = Arc::new(CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let typecheck = Arc::new(TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    QueryLanguageUseCaseAdapter::new(compile, typecheck).compile(bundle, &[], &[])
}

/// Raw query backslashes must reach the regex evaluator unchanged.
const REGEX: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("raw_regex_whitespace_escape_survives_parser_and_evaluation"),
        word: Word::Succeeds,
        input: || {
            QueryText(r#"let rows = select * from "comment" where matches(text, "\s+noqa"); emit rows subject file key(span);"#.into())
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("doubled_query_backslash_matches_literal_backslash_not_whitespace"),
        word: Word::Succeeds,
        input: || {
            QueryText(r#"let rows = select * from "comment" where matches(text, "\\s+noqa"); emit rows subject file key(span);"#.into())
        },
        expect: Expect::Value(false),
    },
];

fn regex_evaluated(text: &Input) -> Result<Value, Failure> {
    use ryo_matching::inbound::use_cases::matching::{
        interface::MatchingUseCase, use_case::MatchingUseCaseImpl,
    };
    use ryo_values::domain::plan_parts::types::{Operand, QueryLiteral, Test};
    use ryo_values::domain::plans::types::Operation;
    let plan = compile_query(&QueryBundle::from(text.clone()))?;
    let patterns: Vec<_> = plan
        .steps
        .iter()
        .filter_map(|step| {
            let Operation::Select { comparisons, .. } = &step.operation else {
                return None;
            };
            comparisons.iter().find_map(|comparison| {
                if comparison.test != Test::Matches {
                    return None;
                }
                let Operand::Literal(QueryLiteral::Text(pattern)) = &comparison.right else {
                    return None;
                };
                Some(pattern)
            })
        })
        .collect();
    assert_eq!(
        patterns.len(),
        1,
        "the parsed predicate must reach the executable plan"
    );
    let matcher = MatchingUseCaseImpl::new();
    let compiled = matcher
        .compile_regex(&patterns[0].0)
        .expect("the query contains a valid regular expression");
    Ok(matcher.matches_regex(&compiled, "\tnoqa"))
}

/// The rows of the examples the operators print: no input, and the count of examples that do not compile as the value.
const EXAMPLES: &[Case<(), usize, Failure>] = &[Case {
    name: CaseName("every_operator_example_compiles_as_it_is_printed"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(0),
}];

/// How many of the examples the operators print do not compile.
fn uncompiled() -> usize {
    MetadataUseCaseImpl
        .operators()
        .iter()
        .filter(|operator| {
            let described = MetadataUseCaseImpl.describe(**operator);
            matches!(
                subject(&QueryText(described.example.0.clone())),
                Err(QueryLanguageError::Refused { .. })
            )
        })
        .count()
}

#[test]
fn succeeds() {
    run(CONTRACT, Word::Succeeds, |bundle| {
        compile_query(&bundle).map(|_| true)
    });
    run(CASES, Word::Succeeds, |text| subject(&text));
    run(EXAMPLES, Word::Succeeds, |()| Ok(uncompiled()));
    run(REGEX, Word::Succeeds, |text| regex_evaluated(&text));
}

#[test]
fn refuses() {
    run(CONTRACT, Word::Refuses, |bundle| {
        compile_query(&bundle).map(|_| true)
    });
    run(CASES, Word::Refuses, |text| subject(&text));
}
