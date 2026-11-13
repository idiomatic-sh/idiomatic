//! The `use_case` adapter of the `query_language` port: the core's contract rows run through it, then the cases specific to it.

use std::sync::Arc;

use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;

use super::adapter::QueryLanguageUseCaseAdapter;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::fixtures::contract::CASES as CONTRACT;
use crate::outbound::query_language::interface::QueryLanguagePort as _;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::selects;

use std::borrow::Cow;
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
            selects!(error.to_string().contains("invalid regular expression"))
        }),
    },
    Case {
        name: CaseName("a_prior_filter_cannot_hide_an_invalid_literal_regex"),
        word: Word::Refuses,
        input: || {
            QueryText(r#"let rows = select * from "comment" where text = "never-present" and matches(text, "["); emit rows subject file key(span);"#.into())
        },
        expect: Expect::Refusal(|error| {
            selects!(error.to_string().contains("invalid regular expression"))
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
            selects!(matches!(error, QueryLanguageError::Refused { .. }))
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
/// What a compiled regex query reached: the patterns of its `matches` predicates, and whether the first, compiled, matches
/// a tab before `noqa` (`None` when there is no pattern or it does not compile).
#[derive(Debug, PartialEq, Eq)]
struct Evaluated {
    patterns: Cow<'static, [Cow<'static, str>]>,
    matches: Option<bool>,
}
const REGEX: &[Case<Input, Evaluated, Failure>] = &[
    Case {
        name: CaseName("raw_regex_whitespace_escape_survives_parser_and_evaluation"),
        word: Word::Succeeds,
        input: || {
            QueryText(r#"let rows = select * from "comment" where matches(text, "\s+noqa"); emit rows subject file key(span);"#.into())
        },
        expect: Expect::Value(Evaluated {
            patterns: Cow::Borrowed(&[Cow::Borrowed(r"\s+noqa")]),
            matches: Some(true),
        }),
    },
    Case {
        name: CaseName("doubled_query_backslash_matches_literal_backslash_not_whitespace"),
        word: Word::Succeeds,
        input: || {
            QueryText(r#"let rows = select * from "comment" where matches(text, "\\s+noqa"); emit rows subject file key(span);"#.into())
        },
        expect: Expect::Value(Evaluated {
            patterns: Cow::Borrowed(&[Cow::Borrowed(r"\\s+noqa")]),
            matches: Some(false),
        }),
    },
];

fn regex_evaluated(text: &Input) -> Result<Evaluated, Failure> {
    use ryo_matching::inbound::use_cases::matching::{
        interface::MatchingUseCase,
        types::{MatchOutcome, RegexSource, SearchText},
        use_case::MatchingUseCaseImpl,
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
    let matcher = MatchingUseCaseImpl::new();
    let matches = patterns.first().and_then(|pattern| {
        matcher
            .compile_regex(RegexSource(&pattern.0))
            .ok()
            .map(|compiled| {
                matcher.matches_regex(&compiled, SearchText("\tnoqa")) == MatchOutcome::Matches
            })
    });
    Ok(Evaluated {
        patterns: patterns
            .iter()
            .map(|pattern| Cow::Owned(pattern.0.to_string()))
            .collect(),
        matches,
    })
}

#[test]
fn succeeds() {
    run(CONTRACT, Word::Succeeds, |bundle| {
        compile_query(&bundle).map(|_| true)
    });
    run(CASES, Word::Succeeds, |text| subject(&text));
    run(REGEX, Word::Succeeds, |text| regex_evaluated(&text));
}

#[test]
fn refuses() {
    run(CONTRACT, Word::Refuses, |bundle| {
        compile_query(&bundle).map(|_| true)
    });
    run(CASES, Word::Refuses, |text| subject(&text));
}
