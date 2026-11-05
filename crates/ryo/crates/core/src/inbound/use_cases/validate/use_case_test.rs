//! What the `validate` use case promises, over the query language's own use cases and the fake of the language port.

use std::sync::Arc;

use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_relations::domain::relations::types::{CellKind, Offset, Span};
use ryo_values::domain::descriptions::types::{StepCount, ValidateReport};
use ryo_values::domain::hosted::types::{ColumnLabel, HostedColumn, HostedName, HostedRelation};
use ryo_values::domain::plan_parts::types::{QueryText, RefusalMessage, RefusedKind, RefusedQuery};
use ryo_values::domain::plans::types::{Plan, QueryOperator};

use super::errors::ValidateError;
use super::interface::ValidateUseCase as _;
use super::types::ValidateRequest;
use super::use_case::ValidateUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::language::interface::LanguagePort;
use ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguageError;
use ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguagePort;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;

const VALID: &str =
    "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");";
const MISSPELLED: &str =
    "let functions = selekt * from \"function\"; emit functions subject \"file\" key(\"span\");";
/// A comparison of the kind of a node with a word that is no kind of any language.
const MISTYPED_KIND: &str = "let nodes = select * from \"syntax-tree\" where kind = \"no-such-kind\"; emit nodes subject \"file\" key(\"span\");";
/// A query of the relation the host declares, `host.placed`.
const HOSTED: &str =
    "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"spec\");";

/// The query text a case validates: one written here, or the example `operator describe` prints for an operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Text {
    /// A text written in this file.
    Written(&'static str),
    /// The example of the operator's description.
    ExampleOf(QueryOperator),
}

/// Whether a language is linked: the fake, which has kinds to check against, or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Linked {
    /// The fake language is linked.
    Fake,
    /// No language is linked.
    Nothing,
}

/// Whether the host declares `host.placed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Declared {
    /// It declares it.
    Placed,
    /// It declares nothing.
    Nothing,
}

/// Which query language validates and compiles the query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Compiler {
    /// The query language over its own use cases.
    Own,
    /// A query language that finds every text correct and compiles none.
    CompilesNothing,
}

/// The query text, whether the host declares `host.placed`, whether a language is linked, and which query language reads it.
type Input = (Text, Declared, Linked, Compiler);
/// How many of ryo's relations the query reads.
type Value = usize;
type Failure = ValidateError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_valid_query_is_reported"),
        word: Word::Succeeds,
        input: || plain(Text::Written(VALID)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("with_no_language_linked_the_kinds_are_not_asked"),
        word: Word::Succeeds,
        input: || {
            (
                Text::Written(MISTYPED_KIND),
                Declared::Nothing,
                Linked::Nothing,
                Compiler::Own,
            )
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_relation_the_host_declared_is_read_as_ryos_own_are"),
        word: Word::Succeeds,
        input: || {
            (
                Text::Written(HOSTED),
                Declared::Placed,
                Linked::Fake,
                Compiler::Own,
            )
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the_example_of_select_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Select)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_project_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Project)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_extend_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Extend)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_join_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Join)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_union_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Union)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_intersection_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Intersection)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_difference_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Difference)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_antijoin_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Antijoin)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_group_count_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::GroupCount)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_group_digest_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::GroupDigest)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_capture_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Capture)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_words_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Words)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_rank_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Rank)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_axis_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Axis)),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("the_example_of_rows_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Rows)),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the_example_of_closure_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Closure)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_example_of_cycles_is_a_valid_query"),
        word: Word::Succeeds,
        input: || plain(Text::ExampleOf(QueryOperator::Cycles)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_syntax_error_is_refused_with_its_place"),
        word: Word::Refuses,
        input: || plain(Text::Written(MISSPELLED)),
        expect: Expect::Refusal(|error| {
            selects(matches!(
                error,
                ValidateError::Refused { error: refused } if refused.kind == RefusedKind::Syntax
            ))
        }),
    },
    Case {
        name: CaseName("a_relation_the_host_did_not_declare_is_refused"),
        word: Word::Refuses,
        input: || plain(Text::Written(HOSTED)),
        expect: Expect::Refusal(|error| {
            selects(matches!(
                error,
                ValidateError::Refused { error: refused }
                    if refused.kind == RefusedKind::UnknownRelation
            ))
        }),
    },
    Case {
        name: CaseName("a_kind_no_linked_language_has_is_refused"),
        word: Word::Refuses,
        input: || plain(Text::Written(MISTYPED_KIND)),
        expect: Expect::Refusal(|error| {
            selects(matches!(error, ValidateError::UnknownKind { .. }))
        }),
    },
    Case {
        name: CaseName("a_correct_query_the_compiler_refuses_is_uncompilable"),
        word: Word::Fails,
        input: || {
            (
                Text::Written(VALID),
                Declared::Nothing,
                Linked::Fake,
                Compiler::CompilesNothing,
            )
        },
        expect: Expect::Refusal(|error| {
            selects(matches!(error, ValidateError::Uncompilable { .. }))
        }),
    },
];

/// The text with nothing declared, the fake linked and the query language's own compiler.
fn plain(text: Text) -> Input {
    (text, Declared::Nothing, Linked::Fake, Compiler::Own)
}

/// The relation `host.placed`, as the host declares it: a file and a text.
fn placed() -> HostedRelation {
    HostedRelation {
        name: HostedName("host.placed".into()),
        columns: vec![
            HostedColumn {
                name: ColumnLabel("path".into()),
                kind: CellKind::File,
            },
            HostedColumn {
                name: ColumnLabel("spec".into()),
                kind: CellKind::Symbol,
            },
        ],
    }
}

/// The query language over its own use cases, as an edge composes it.
fn query_language() -> Arc<dyn QueryLanguagePort> {
    Arc::new(crate::outbound::query_language::factory::use_case(
        Arc::new(CompileUseCaseImpl::new(
            Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
            Arc::new(ryo_query_language::outbound::catalog::factory::language()),
        )),
        Arc::new(TypecheckUseCaseImpl::new(
            Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
            Arc::new(ryo_query_language::outbound::catalog::factory::language()),
        )),
    ))
}

fn subject((text, declared, linked, compiler): Input) -> Result<Value, Failure> {
    let language: Arc<dyn LanguagePort> = match linked {
        Linked::Fake => Arc::new(LanguageFake::default()),
        Linked::Nothing => Arc::new(crate::outbound::language::factory::use_case(Vec::new())),
    };
    let hosted = match declared {
        Declared::Placed => vec![placed()],
        Declared::Nothing => Vec::new(),
    };
    let compiling: Arc<dyn QueryLanguagePort> = match compiler {
        Compiler::Own => query_language(),
        Compiler::CompilesNothing => Arc::new(CompilesNothing),
    };
    let text: Box<str> = match text {
        Text::Written(written) => written.into(),
        Text::ExampleOf(operator) => MetadataUseCaseImpl.describe(operator).example.0,
    };
    ValidateUseCaseImpl::new(compiling, language)
        .validate(ValidateRequest {
            observations: Vec::new(),
            text: ryo_values::domain::program::types::QueryBundle::from(QueryText(text)),
            hosted,
        })
        .map(|report| report.relations.len())
}

/// A query language that finds every text correct and compiles none: a correct query the compiler still refuses.
#[derive(Debug)]
struct CompilesNothing;

impl QueryLanguagePort for CompilesNothing {
    fn validate(
        &self,
        _text: &ryo_values::domain::program::types::QueryBundle,
        _hosted: &[HostedRelation],
        _observations: &[ryo_values::domain::observations::types::BoundObservationSchema],
    ) -> Result<ValidateReport, QueryLanguageError> {
        Ok(ValidateReport {
            relations: Vec::new(),
            operators: Vec::new(),
            steps: StepCount(0),
        })
    }

    fn compile(
        &self,
        _text: &ryo_values::domain::program::types::QueryBundle,
        _hosted: &[HostedRelation],
        _observations: &[ryo_values::domain::observations::types::BoundObservationSchema],
    ) -> Result<Plan, QueryLanguageError> {
        Err(QueryLanguageError::Refused {
            error: RefusedQuery {
                kind: RefusedKind::Invalid,
                message: RefusalMessage("no plan".into()),
                span: Span {
                    start: Offset(0),
                    end: Offset(0),
                },
                origin: None,
            },
        })
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
