//! The `joining` and `distinctness` services' band joins, on query text read by the real syntax adapter: a `where` comparing the
//! sides in order makes a band join, `nearest` and `all-pairs` say which of its rows it answers, and each join's fan is proven.

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::band_joins::types::BandJoin;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::assembling::service::lower as lower_program;
use crate::domain::fanning::service::fanned;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::Builder;
use crate::domain::selecting::service::lower_select;
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::selects;
use ryo_values::domain::band_joins::types::BandAnswer;
use ryo_values::domain::plan_parts::types::KeyFan;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_syntax::types::Expression;
use ryo_values::domain::query_tokens::types::QueryView;

/// The query text a row gives: the selects of a program lowered alone, or a whole program lowered as the compiler does.
#[derive(Clone, Copy, Debug)]
enum Written {
    /// Each select of the text lowered alone, then the fans proven.
    Selects(&'static str),
    /// The whole program, through every pass.
    Program(&'static str),
}
type Input = Written;
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

/// The last point of each file, the points distinct, by a strict self join; then each event read joined to its file's last point.
const LAST_BY_SELF_JOIN: &str = "let events = select ordinal as point, file from \"binding-event\"; let points = project(from=events, columns=[\"point\", \"file\"]); let earlier = select l.point as point, l.file as file from points as l join points as r on l.file = r.file where l.point < r.point; let last = antijoin(left=points, right=earlier, left-on=[\"point\", \"file\"], right-on=[\"point\", \"file\"]); let at_last = select l.ordinal as o, r.point as point from \"binding-event\" as l join last as r on l.file = r.file; emit at_last subject o key(o);";
/// The same last point, by an `ordered-match` of the points with themselves.
const LAST_BY_ORDERED_MATCH: &str = "let events = select ordinal as point, file from \"binding-event\"; let points = project(from=events, columns=[\"point\", \"file\"]); let pairs = ordered-match(left=points, right=points, left-on=[file], right-on=[file], left-order=\"point\", right-order=\"point\", output={point:[left,point], file:[left,file]}); let last = antijoin(left=points, right=pairs, left-on=[\"point\", \"file\"], right-on=[\"point\", \"file\"]); let at_last = select l.ordinal as o, r.point as point from \"binding-event\" as l join last as r on l.file = r.file; emit at_last subject o key(o);";
/// The same over the points as selected: a select of columns is a projection that drops repeated rows, so they are distinct.
const LAST_OF_SELECTED: &str = "let points = select ordinal as point, file from \"binding-event\"; let earlier = select l.point as point, l.file as file from points as l join points as r on l.file = r.file where l.point < r.point; let last = antijoin(left=points, right=earlier, left-on=[\"point\", \"file\"], right-on=[\"point\", \"file\"]); let at_last = select l.ordinal as o, r.point as point from \"binding-event\" as l join last as r on l.file = r.file; emit at_last subject o key(o);";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_nearest_join_answers_the_closest_earlier_row_per_key"),
        word: Word::Succeeds,
        input: || Written::Selects(NEAREST),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Band(
            BandAnswer::Nearest,
            1,
            KeyFan::Shared,
        )])),
    },
    Case {
        name: CaseName("a_nearest_join_needs_exactly_one_order_comparison"),
        word: Word::Refuses,
        input: || Written::Selects(NEAREST_TWO),
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
        input: || Written::Selects(FILE_BAND),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Band(
            BandAnswer::Every,
            2,
            KeyFan::FileOnly,
        )])),
    },
    Case {
        name: CaseName("an_all_pairs_join_declares_its_pairs"),
        word: Word::Succeeds,
        input: || Written::Selects(ALL_PAIRS),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Band(
            BandAnswer::Every,
            1,
            KeyFan::AllPairs,
        )])),
    },
    Case {
        name: CaseName("a_join_on_the_file_alone_is_file_only"),
        word: Word::Succeeds,
        input: || Written::Selects(FILE_ONLY),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Equal(
            KeyFan::FileOnly,
        )])),
    },
    Case {
        name: CaseName("a_name_keyed_join_is_shared"),
        word: Word::Succeeds,
        input: || Written::Selects(SHARED),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Equal(KeyFan::Shared)])),
    },
    Case {
        name: CaseName("a_join_on_a_relation_key_is_unique"),
        word: Word::Succeeds,
        input: || Written::Selects(UNIQUE),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Equal(KeyFan::Unique)])),
    },
    Case {
        name: CaseName("a_not_equal_across_the_sides_stays_the_filter_after_the_join"),
        word: Word::Succeeds,
        input: || Written::Selects(NOT_EQUAL),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Equal(KeyFan::Shared)])),
    },
    Case {
        name: CaseName("a_join_on_the_file_to_its_last_point_by_a_strict_self_join_is_unique"),
        word: Word::Succeeds,
        input: || Written::Program(LAST_BY_SELF_JOIN),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Joined::Band(BandAnswer::Every, 1, KeyFan::FileOnly),
            Joined::Equal(KeyFan::Unique),
        ])),
    },
    Case {
        name: CaseName("a_join_on_the_file_to_its_last_point_by_an_ordered_match_is_unique"),
        word: Word::Succeeds,
        input: || Written::Program(LAST_BY_ORDERED_MATCH),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[Joined::Equal(KeyFan::Unique)])),
    },
    Case {
        name: CaseName(
            "a_join_on_the_file_to_the_last_of_selected_points_is_unique_as_a_select_of_columns_drops_repeats",
        ),
        word: Word::Succeeds,
        input: || Written::Program(LAST_OF_SELECTED),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            Joined::Band(BandAnswer::Every, 1, KeyFan::FileOnly),
            Joined::Equal(KeyFan::Unique),
        ])),
    },
];

fn subject(written: Input) -> Result<Value, Failure> {
    let steps = match written {
        Written::Selects(text) => lowered_selects(text)?,
        Written::Program(text) => {
            let program = winnow()
                .read(QueryView(text))
                .expect("the row's text is readable");
            lower_program(&program, CATALOG, &[])?.steps
        }
    };
    Ok(steps
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

/// The steps of each select of `text` lowered alone, each join's fan proven.
fn lowered_selects(text: &str) -> Result<Vec<ryo_values::domain::plans::types::PlanStep>, Failure> {
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
    Ok(fanned(builder.steps))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
