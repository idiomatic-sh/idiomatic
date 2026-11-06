//! The `resolving` service's cases, on query text read by the real syntax adapter: the names a query writes, and what is refused when they name nothing.

use ryo_relations::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, Sourced};
use crate::domain::resolving::service::{column_of, position, read_source};
use crate::outbound::syntax::factory::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::program::types::{
    Condition, Expression, QuerySpelling, QueryView, WrittenOperand,
};

/// The query text a row gives.
type Input = &'static str;
/// The name of the column the emitted subject is.
type Value = &'static str;
/// What the lowering refuses a row with.
type Failure = LoweringError;

/// Every column of a relation, nothing else asked.
const ALL: &str = "let a = select * from \"function\"; emit a subject file key(span);";
const NO_COLUMN: &str =
    "let a = select * from \"function\" where colour = \"red\"; emit a subject file key(span);";
const NO_SUBJECT: &str = "let a = select * from \"function\"; emit a subject path key(span);";
const UNKNOWN_RELATION: &str = "let a = select * from \"nothing\"; emit a subject file key(span);";
const UNKNOWN_STEP: &str = "let a = select * from earlier; emit a subject file key(span);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_subject_is_found_among_the_columns_of_a_relation"),
        word: Word::Succeeds,
        input: || ALL,
        expect: Expect::Value("file"),
    },
    Case {
        name: CaseName("a_column_the_relation_lacks_is_refused"),
        word: Word::Refuses,
        input: || NO_COLUMN,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::UnknownColumn { .. }))
        }),
    },
    Case {
        name: CaseName("an_emitted_column_the_step_lacks_is_refused"),
        word: Word::Refuses,
        input: || NO_SUBJECT,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::UnknownColumn { .. }))
        }),
    },
    Case {
        name: CaseName("a_relation_ryo_does_not_own_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_RELATION,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::UnknownRelation { .. }))
        }),
    },
    Case {
        name: CaseName("a_step_nobody_defined_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_STEP,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::UnknownStep { .. }))
        }),
    },
];

/// The relations the rows may read.
const CATALOG: &[Relation] = &[
    Relation::Function,
    Relation::SyntaxChild,
    Relation::SyntaxTree,
    Relation::WorkspaceMember,
];

/// Reads the source of each `select`, resolves the columns its `where` names and the subject the query emits, and names the column the subject is, but the answer must be one of the known names.
fn subject(text: Input) -> Result<Value, Failure> {
    let program = winnow()
        .read(QueryView(text))
        .expect("the row's text is readable");
    let builder = Builder {
        catalog: CATALOG,
        observation_inputs: &[],
        hosted_steps: Vec::new(),
        parameters: &program.parameters,
        kinds: Vec::new(),
        steps: Vec::new(),
        lowered: Vec::new(),
        span: program.emit.span,
        origin: None,
        comparison_origins: Vec::new(),
    };
    let mut found = String::new();
    for step in &program.steps {
        if let Expression::Select(select) = &step.expression {
            let Sourced { fields, .. } = read_source(&builder, &select.from)?;
            for condition in &select.filters {
                if let Condition::Test(filter) = condition
                    && let WrittenOperand::Column(reference) = &filter.left
                {
                    let ColumnIndex(_) =
                        position(&fields, QuerySpelling(&reference.column.0), builder.span)?;
                }
            }
            let index = position(
                &fields,
                QuerySpelling(&program.emit.subject.0),
                program.emit.span,
            )?;
            found = column_of(&fields, index).name.0.to_string();
        }
    }
    let known = ["file"];
    Ok(known
        .into_iter()
        .find(|name| *name == found)
        .unwrap_or("unexpected"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
