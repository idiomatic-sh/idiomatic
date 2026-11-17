//! The `joining` and `distinctness` services' band joins, on query text read by the real syntax adapter: a `where` comparing the
//! sides in order makes a band join, `nearest` and `all-pairs` say which of its rows it answers, and each join's fan is proven.

use ryo_relations::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fanning::service::fanned;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use crate::domain::selecting::service::lower_select;
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::selects;
use ryo_values::domain::band_joins::types::{BandAnswer, BandJoin};
use ryo_values::domain::plan_parts::types::KeyFan;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_syntax::types::Expression;
use ryo_values::domain::query_tokens::types::QueryView;

/// The query text a row gives.
type Input = &'static str;
/// Each join of the plan, in order.
type Value = std::borrow::Cow<'static, [Joined]>;
/// What the lowering refuses a row with.
type Failure = LoweringError;

/// A join the plan holds: an equality join and its fan, or a band join, its answer, its edges and its fan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Joined {
    /// `Operation::Join`.
    Equal(KeyFan),
    /// `Operation::BandJoin`.
    Band(BandAnswer, usize, KeyFan),
}

/// The relations the rows read.
const CATALOG: &[Relation] = &[Relation::BindingEvent, Relation::Function];

const NEAREST: &str = "let a = select l.\"ordinal\" as \"o\", r.\"ordinal\" as \"bound\" from \"binding-event\" as l nearest join \"binding-event\" as r on l.\"name\" = r.\"name\" where r.\"ordinal\" < l.\"ordinal\"; emit a subject o key(o);";
const NEAREST_TWO: &str = "let a = select l.\"ordinal\" as \"o\" from \"binding-event\" as l nearest join \"binding-event\" as r on l.\"name\" = r.\"name\" where r.\"ordinal\" < l.\"ordinal\" and r.\"ordinal\" > l.\"ordinal\"; emit a subject o key(o);";
const FILE_BAND: &str = "let a = select l.\"ordinal\" as \"o\" from \"binding-event\" as l join \"binding-event\" as r on l.\"file\" = r.\"file\" where r.\"ordinal\" >= l.\"ordinal\" and r.\"ordinal\" <= l.\"ordinal\"; emit a subject o key(o);";
const ALL_PAIRS: &str = "let a = select l.\"ordinal\" as \"o\" from \"binding-event\" as l all-pairs join \"binding-event\" as r on l.\"file\" = r.\"file\" where r.\"ordinal\" >= l.\"ordinal\"; emit a subject o key(o);";
const FILE_ONLY: &str = "let a = select l.\"ordinal\" as \"o\" from \"binding-event\" as l join \"binding-event\" as r on l.\"file\" = r.\"file\"; emit a subject o key(o);";
const SHARED: &str = "let a = select l.\"ordinal\" as \"o\" from \"binding-event\" as l join \"binding-event\" as r on l.\"name\" = r.\"name\"; emit a subject o key(o);";
const UNIQUE: &str = "let a = select l.\"ordinal\" as \"o\" from \"binding-event\" as l join \"function\" as r on l.\"occurrence\" = r.\"id\"; emit a subject o key(o);";
const NOT_EQUAL: &str = "let a = select l.\"ordinal\" as \"o\" from \"binding-event\" as l join \"binding-event\" as r on l.\"name\" = r.\"name\" where r.\"ordinal\" != l.\"ordinal\"; emit a subject o key(o);";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_nearest_join_answers_the_closest_earlier_row_per_key"),
        word: Word::Succeeds,
        input: || NEAREST,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Band(
            BandAnswer::Nearest,
            1,
            KeyFan::Shared,
        )])),
    },
    Case {
        name: CaseName("a_nearest_join_needs_exactly_one_order_comparison"),
        word: Word::Refuses,
        input: || NEAREST_TWO,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::NearestNeedsOneOrder { count: 2, .. }
            ))
        }),
    },
    Case {
        name: CaseName("a_join_on_the_file_alone_with_a_band_is_a_file_only_band_join"),
        word: Word::Succeeds,
        input: || FILE_BAND,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Band(
            BandAnswer::Every,
            2,
            KeyFan::FileOnly,
        )])),
    },
    Case {
        name: CaseName("an_all_pairs_join_declares_its_pairs"),
        word: Word::Succeeds,
        input: || ALL_PAIRS,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Band(
            BandAnswer::Every,
            1,
            KeyFan::AllPairs,
        )])),
    },
    Case {
        name: CaseName("a_join_on_the_file_alone_is_file_only"),
        word: Word::Succeeds,
        input: || FILE_ONLY,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Equal(
            KeyFan::FileOnly,
        )])),
    },
    Case {
        name: CaseName("a_name_keyed_join_is_shared"),
        word: Word::Succeeds,
        input: || SHARED,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Equal(KeyFan::Shared)])),
    },
    Case {
        name: CaseName("a_join_on_a_relation_key_is_unique"),
        word: Word::Succeeds,
        input: || UNIQUE,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Equal(KeyFan::Unique)])),
    },
    Case {
        name: CaseName("a_not_equal_across_the_sides_stays_the_filter_after_the_join"),
        word: Word::Succeeds,
        input: || NOT_EQUAL,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Equal(KeyFan::Shared)])),
    },
];

fn subject(text: Input) -> Result<Value, Failure> {
    let program = winnow()
        .read(QueryView(text))
        .expect("the row's text is readable");
    let mut builder = Builder {
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
    for step in &program.steps {
        builder.span = step.span;
        if let Expression::Select(select) = &step.expression {
            lower_select(&mut builder, &step.name, select)?;
        }
    }
    Ok(fanned(builder.steps)
        .iter()
        .filter_map(|step| match &step.operation {
            Operation::Join { fan, .. } => Some(Joined::Equal(*fan)),
            Operation::BandJoin(BandJoin {
                answer, band, fan, ..
            }) => Some(Joined::Band(*answer, band.len(), *fan)),
            _ => None,
        })
        .collect())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
