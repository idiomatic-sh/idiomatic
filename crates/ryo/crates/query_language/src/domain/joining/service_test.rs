//! The `joining` service's cases, on query text read by the real syntax adapter: the steps a join makes, and the sides its
//! `where` reads.

use ryo_language::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::joining::service::{select_join, shown_of};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, LastStep};
use crate::domain::resolving::service::read_source;
use crate::outbound::syntax::factory::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::program::types::{Expression, Joining, QueryView};

/// The query text a row gives.
type Input = &'static str;
/// The columns the `select` shows, and the columns its join gives.
type Value = (usize, usize);
/// What the lowering refuses a row with.
type Failure = LoweringError;

const JOINED: &str = "let a = select l.\"id\" as \"x\", r.\"name\" as \"y\" from \"function\" as l join \"function\" as r on l.\"id\" = r.\"id\"; emit a subject x key(x);";
const JOINED_WHERE: &str = "let a = select l.\"id\" as \"x\" from \"function\" as l join \"function\" as r on l.\"id\" = r.\"id\" where r.\"name\" = \"main\"; emit a subject x key(x);";
const JOINED_KEPT_WHERE: &str = "let a = select l.\"id\" as \"x\", r.\"name\" as \"y\" from \"function\" as l join \"function\" as r on l.\"id\" = r.\"id\" where r.\"name\" = \"main\"; emit a subject x key(x);";
const JOINED_EITHER: &str = "let a = select l.\"id\" as \"x\" from \"function\" as l join \"function\" as r on l.\"id\" = r.\"id\" where r.\"name\" = \"main\" or l.\"name\" = \"run\"; emit a subject x key(x);";
const UNKNOWN_RIGHT: &str = "let a = select l.\"id\" as \"x\" from \"function\" as l join \"function\" as r on l.\"id\" = r.\"id\" where r.\"nothing\" = \"main\"; emit a subject x key(x);";
const UNKNOWN_LEFT: &str = "let a = select l.\"id\" as \"x\" from \"function\" as l join \"function\" as r on l.\"id\" = r.\"id\" where l.\"nothing\" = \"main\"; emit a subject x key(x);";
const UNKNOWN_SIDE: &str = "let a = select l.\"id\" as \"x\" from \"function\" as l join \"function\" as r on l.\"id\" = r.\"id\" where q.\"name\" = \"main\"; emit a subject x key(x);";

/// A join whose `on` names a side by an alias neither side has.
const UNKNOWN_ON_SIDE: &str = "let a = select l.\"id\" as \"x\" from \"function\" as l join \"function\" as r on q.\"id\" = r.\"id\"; emit a subject x key(x);";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_join_gives_the_columns_the_select_shows"),
        word: Word::Succeeds,
        input: || JOINED,
        expect: Expect::Value((2, 2)),
    },
    Case {
        name: CaseName(
            "a_where_on_a_column_the_select_does_not_show_is_given_after_the_shown_ones",
        ),
        word: Word::Succeeds,
        input: || JOINED_WHERE,
        expect: Expect::Value((1, 2)),
    },
    Case {
        name: CaseName("a_where_on_a_shown_column_of_the_joined_side_gives_no_other"),
        word: Word::Succeeds,
        input: || JOINED_KEPT_WHERE,
        expect: Expect::Value((2, 2)),
    },
    Case {
        name: CaseName(
            "an_or_over_both_sides_of_a_join_gives_the_column_of_each_after_the_shown_ones",
        ),
        word: Word::Succeeds,
        input: || JOINED_EITHER,
        expect: Expect::Value((1, 3)),
    },
    Case {
        name: CaseName("a_column_the_joined_side_lacks_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_RIGHT,
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, LoweringError::UnknownSideColumn { alias, name, .. } if alias == "r" && name == "nothing"),
            )
        }),
    },
    Case {
        name: CaseName("a_column_the_left_side_lacks_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_LEFT,
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, LoweringError::UnknownSideColumn { alias, name, .. } if alias == "l" && name == "nothing"),
            )
        }),
    },
    Case {
        name: CaseName("an_alias_neither_side_has_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_SIDE,
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, LoweringError::UnknownSide { alias, sides, .. } if alias == "q" && sides == "`l` and `r`"),
            )
        }),
    },
    Case {
        name: CaseName("an_on_alias_neither_side_has_is_refused"),
        word: Word::Refuses,
        input: || UNKNOWN_ON_SIDE,
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, LoweringError::UnknownSide { alias, sides, .. } if alias == "q" && sides == "`l` and `r`"),
            )
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

/// Joins the source of each joined `select` with the part's own join and counts the columns the `select` shows and the columns the join gives.
fn subject(text: Input) -> Result<Value, Failure> {
    let program = winnow()
        .read(QueryView(text))
        .expect("the row's text is readable");
    let mut builder = Builder {
        catalog: CATALOG,
        hosted_steps: Vec::new(),
        parameters: &program.parameters,
        kinds: Vec::new(),
        steps: Vec::new(),
        lowered: Vec::new(),
        span: program.emit.span,
    };
    let mut counted = (0, 0);
    for step in &program.steps {
        if let Expression::Select(select) = &step.expression
            && let Joining::Joined(join) = &select.join
        {
            builder.span = step.span;
            let source = read_source(&builder, &select.from)?;
            let made = select_join(
                &mut builder,
                &step.name,
                select,
                join,
                source,
                LastStep::Later,
            )?;
            counted = (shown_of(&select.columns).0, made.sourced.fields.len());
        }
    }
    Ok(counted)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
