//! The words and decoded columns a comparison reads, on query text read by the real syntax adapter.

use crate::domain::comparing::service::comparison_of;
use crate::domain::lowering::types::Addressable;
use crate::domain::lowering::types::Builder;
use crate::domain::lowering::types::Sourced;
use crate::domain::resolving::service::read_source;
use crate::domain::syntax::constants::parameter_kind_named;
use crate::outbound::syntax::fixtures::winnow::winnow;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::program::types::Condition;
use ryo_values::domain::program::types::Expression;
use ryo_values::domain::program::types::QueryView;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::errors::LoweringError;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::selects;

const UNKNOWN_KIND: &str =
    "let a = select * from \"defines\" where kind = \"headline\"; emit a subject file key(span);";

const UNKNOWN_ORIGIN: &str = "let a = select * from \"package-dependency\" where origin = \"nope\"; emit a subject source key(source);";

const KNOWN_KIND: &str =
    "let a = select * from \"defines\" where kind = \"section\"; emit a subject file key(span);";

const FLAG_WORD: &str = "let a = select * from \"string-literal\" where import = \"false\"; emit a subject file key(span);";

const FLAG_OTHER_TEXT: &str = "let a = select * from \"string-literal\" where import = \"no\"; emit a subject file key(span);";

const DECODED_BOOLEAN: &str = "let a = select * from \"entry\" where key = \"strict\" and boolean = \"true\"; emit a subject file key(span);";

const DECODED_INTEGER: &str = "let a = select * from \"sequence-element\" where kind = \"integer\" and integer > 3; emit a subject file key(span);";

const EXACT_REQUIREMENT: &str = "let a = select * from \"package-dependency\" where exact = \"false\" and requirement != \"\"; emit a subject source key(package);";

const DECODED_FLOAT: &str = "let a = select * from \"entry\" where kind = \"float\" and float_text_gte(float, \"0.5\"); emit a subject file key(span);";

const DECODED_DATETIME: &str = "let a = select * from \"sequence-element\" where datetime >= \"2000-01-01\"; emit a subject file key(span);";

const INTEGER_RADIX: &str = "let a = select * from \"entry\" where kind = \"integer\" and radix = 16; emit a subject file key(span);";

const GIT_REVISION: &str = "let a = select * from \"package-dependency\" where revision != \"\"; emit a subject source key(package);";

const INTERPRETER_CONSTRAINT: &str = "let a = select * from \"package-dependency\" where interpreter != \"\"; emit a subject source key(package);";

const CATALOG_SPELLING: &str = "let a = select * from \"package-dependency\" where catalog = \"catalog:\"; emit a subject source key(package);";

const REPLACED: &str = "let a = select * from \"package-dependency\" where replacement != \"\"; emit a subject source key(package);";

const DOC_COMMENT: &str =
    "let a = select * from \"comment\" where doc = \"true\"; emit a subject file key(span);";

const NUMBER_AND_TEXT_ORDERED: &str =
    "let a = select * from \"function\" where name < 5; emit a subject file key(span);";

/// The query text a row gives.
type Input = &'static str;

/// How many comparisons the `where` of the row made.
type Value = usize;

/// What the lowering refuses a row with.
type Failure = LoweringError;

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

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_word_that_is_a_kind_of_the_column_is_accepted"),
        word: Word::Succeeds,
        input: || KNOWN_KIND,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_word_that_is_no_kind_of_the_column_is_refused_with_the_kinds_there_are"),
        word: Word::Refuses,
        input: || UNKNOWN_KIND,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::UnknownWord { word, owner, .. } if &**word == "headline" && &**owner == "defines"
            ))
        }),
    },
    Case {
        name: CaseName("a_word_that_is_no_origin_is_refused_naming_the_column_it_is_compared_with"),
        word: Word::Refuses,
        input: || UNKNOWN_ORIGIN,
        expect: Expect::Refusal(|error| {
            selects!(
                error.to_string()
                    == "`nope` is not an origin of package-dependency; the origins are registry, path, git, url, workspace",
            )
        }),
    },
    Case {
        name: CaseName("a_decoded_boolean_is_compared_with_the_word_it_is_shown_as"),
        word: Word::Succeeds,
        input: || DECODED_BOOLEAN,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_decoded_integer_is_ordered_against_a_number"),
        word: Word::Succeeds,
        input: || DECODED_INTEGER,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_requirement_is_tested_for_one_version_by_its_exact_flag"),
        word: Word::Succeeds,
        input: || EXACT_REQUIREMENT,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_decoded_float_is_ordered_as_a_number_by_float_text"),
        word: Word::Succeeds,
        input: || DECODED_FLOAT,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_decoded_date_time_is_ordered_as_text"),
        word: Word::Succeeds,
        input: || DECODED_DATETIME,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_integer_is_tested_for_its_base_by_its_radix"),
        word: Word::Succeeds,
        input: || INTEGER_RADIX,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_git_dependency_is_tested_for_the_revision_it_names"),
        word: Word::Succeeds,
        input: || GIT_REVISION,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_dependency_is_tested_for_an_interpreter_constraint"),
        word: Word::Succeeds,
        input: || INTERPRETER_CONSTRAINT,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_dependency_is_tested_for_the_catalog_spelling_it_writes"),
        word: Word::Succeeds,
        input: || CATALOG_SPELLING,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_dependency_is_tested_for_what_replaces_it"),
        word: Word::Succeeds,
        input: || REPLACED,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_comment_is_tested_for_documentation_by_its_doc_flag"),
        word: Word::Succeeds,
        input: || DOC_COMMENT,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_flag_is_compared_with_the_word_it_is_shown_as"),
        word: Word::Succeeds,
        input: || FLAG_WORD,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_flag_compared_with_other_text_is_refused"),
        word: Word::Refuses,
        input: || FLAG_OTHER_TEXT,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("ordering_a_name_against_a_number_is_refused"),
        word: Word::Refuses,
        input: || NUMBER_AND_TEXT_ORDERED,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
