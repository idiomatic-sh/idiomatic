//! What the presence cases observe of a file run, and the routed program they run.
use super::*;

/// What a rule's root answered on the file: its rows and their kinds, or the outcome's kind with
/// its error as the error prints.
#[derive(Clone, Debug, PartialEq)]
pub(in super::super) enum Outcome {
    Rows(usize, Cow<'static, [CellKind]>),
    Partial(usize),
    Skipped,
    Failed(Cow<'static, str>),
    Unknown(Cow<'static, str>),
    Refused(Cow<'static, str>),
    NoQuery,
    Deferred,
}

/// What one file run answered and left behind: each rule's outcome and the rows each of its steps
/// was fed, each probed region's instantiation and a second release's refusal, the roots presence
/// found absent, the cleanup refusal, the dormant obligations and the finish refusal.
#[derive(Clone, Debug, PartialEq)]
pub(in super::super) struct Observed {
    pub(in super::super) outcomes: Cow<'static, [Outcome]>,
    pub(in super::super) fed: Cow<'static, [Cow<'static, [u64]>]>,
    pub(in super::super) probes: Cow<'static, [RegionProbe]>,
    pub(in super::super) presence_absent: u64,
    pub(in super::super) again: Cow<'static, [Option<OperatingError>]>,
    pub(in super::super) cleanup: Option<OperatingError>,
    pub(in super::super) dormant: usize,
    pub(in super::super) finished: Option<OperatingError>,
}

/// The recogniser's self-join anti-join shape over `source_location`: its select runs on the ordered-existence route.
pub(in super::super) fn routed() -> Plan {
    let schema = kinds(Relation::SourceLocation);
    let number = ColumnIndex(
        u32::try_from(
            schema
                .iter()
                .position(|kind| *kind == CellKind::Number)
                .expect("a numeric column"),
        )
        .unwrap_or(0),
    );
    plan(vec![
        select(Input::Relation(Relation::SourceLocation), vec![]),
        step(Operation::Join {
            left: at(0),
            right: at(0),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: number,
                },
                JoinColumn {
                    side: Side::Right,
                    column: number,
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
        step(Operation::Select {
            from: at(1),
            comparisons: vec![PlanComparison {
                test: Test::LessThan,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Column(ColumnIndex(1)),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: vec![],
        }),
        step(Operation::Project {
            from: at(2),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        }),
        step(Operation::Antijoin {
            left: at(0),
            right: at(3),
            left_on: vec![number],
            right_on: vec![ColumnIndex(0)],
        }),
    ])
}
