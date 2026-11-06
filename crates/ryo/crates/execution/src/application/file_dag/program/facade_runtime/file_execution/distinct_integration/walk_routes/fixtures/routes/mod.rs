//! Authored plans for per-row and request-owned routes, and for combining operations whose
//! inputs may fault; each is a plan of `Authored` like the rest.
use super::super::super::*;
use super::plans::{Authored, collected, comparison, plan, rules};
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::plan_parts::types::{
    CapturePattern, HostedNumber, JoinColumn, LiteralText, Operand, ParameterId, ParameterName,
    QueryLiteral, Side, TableKey, Test,
};
use ryo_values::domain::query_bindings::types::ParameterType;
use ryo_values::domain::query_bindings::types::PlanParameter;
use ryo_values::domain::query_bindings::types::{BindingCell, BindingTable, ParameterBinding};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::records::types::Cell as HostedCell;

/// A request-owned or per-row operation read beside a shared head.
#[derive(Clone, Copy, Debug)]
pub(in super::super) enum RowRoute {
    Capture,
    Rank,
    Replace,
    SpanBetween,
    SourceMatches,
    RenderSyntax,
    FitsRendered,
    SpanBounds,
    Rows,
    Hosted,
}
impl RowRoute {
    pub(in super::super) const ALL: [Self; 10] = [
        Self::Capture,
        Self::Rank,
        Self::Replace,
        Self::SpanBetween,
        Self::SourceMatches,
        Self::RenderSyntax,
        Self::FitsRendered,
        Self::SpanBounds,
        Self::Rows,
        Self::Hosted,
    ];
    pub(in super::super) fn kind(self) -> OperationKind {
        match self {
            Self::Capture => OperationKind::Capture,
            Self::Rank => OperationKind::Rank,
            Self::Replace => OperationKind::Replace,
            Self::SpanBetween => OperationKind::SpanBetween,
            Self::SourceMatches => OperationKind::SourceMatches,
            Self::RenderSyntax => OperationKind::RenderSyntax,
            Self::FitsRendered => OperationKind::FitsRendered,
            Self::SpanBounds => OperationKind::SpanBounds,
            Self::Rows => OperationKind::Rows,
            Self::Hosted => OperationKind::Hosted,
        }
    }
}
const TEXT: ColumnIndex = ColumnIndex(1);
const SPAN: ColumnIndex = ColumnIndex(3);
fn functions() -> PlanInput {
    PlanInput::Relation(Relation::Function)
}
impl Authored {
    /// A shared head over the function relation (node, name text, file, span columns), and
    /// beside it one per-row or request-owned operation over the same rows.
    pub(in super::super) fn row_route(route: RowRoute) -> Self {
        let from = functions();
        let mut parameters = vec![];
        let mut bindings = vec![];
        let mut hosts = vec![];
        let operation = match route {
            RowRoute::Capture => Operation::Capture {
                from,
                path: TEXT,
                pattern: CapturePattern("{parent}/{child}".into()),
            },
            RowRoute::Rank => {
                parameters.push(PlanParameter {
                    name: ParameterName("sets".into()),
                    kind: ParameterType::TextSets,
                });
                bindings.push(ParameterBinding::TextSets(vec![vec![LiteralText(
                    "a".into(),
                )]]));
                Operation::Rank {
                    from,
                    column: TEXT,
                    sets: ParameterId(0),
                }
            }
            RowRoute::Replace => Operation::Replace {
                from,
                text: TEXT,
                pattern: LiteralText("a".into()),
                replacement: LiteralText("b".into()),
            },
            RowRoute::SpanBetween => Operation::SpanBetween {
                from,
                start: SPAN,
                end: SPAN,
            },
            RowRoute::SourceMatches => Operation::SourceMatches {
                from,
                span: SPAN,
                base_line: None,
                pattern: LiteralText("a".into()),
                capture: None,
                capture_text: ryo_values::domain::query_operators::types::TextOutput::Omitted,
                capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
            },
            RowRoute::RenderSyntax => Operation::RenderSyntax {
                from,
                node: ColumnIndex(0),
                context: ryo_language::domain::semantics::types::RenderContext::PositionalArgument,
            },
            RowRoute::FitsRendered => Operation::FitsRendered {
                from,
                texts: vec![TEXT],
                literals: vec![LiteralText("".into()), LiteralText("".into())],
                span: SPAN,
                max_line: ryo_values::domain::query_operators::types::MaxLine(88),
                tab_width: ryo_values::domain::query_operators::types::TabWidth(4),
            },
            RowRoute::SpanBounds => Operation::SpanBounds { from, span: SPAN },
            RowRoute::Rows => {
                parameters.push(PlanParameter {
                    name: ParameterName("rows".into()),
                    kind: ParameterType::Tables,
                });
                bindings.push(ParameterBinding::Tables(vec![BindingTable(vec![
                    BindingCell {
                        key: LiteralText("key".into()),
                        value: LiteralText("value".into()),
                    },
                ])]));
                Operation::Rows {
                    parameter: ParameterId(0),
                    keys: vec![TableKey("key".into())],
                }
            }
            RowRoute::Hosted => {
                hosts.push(HostRelation {
                    name: HostName("route-hosted".into()),
                    columns: vec![HostColumn {
                        name: HostColumnName("n".into()),
                        kind: HostKind::Number,
                    }],
                    rows: [5, 6]
                        .into_iter()
                        .map(|number| HostRow(vec![HostedCell::Number(Integer(number))]))
                        .collect(),
                });
                Operation::Hosted {
                    relation: HostedNumber(0),
                }
            }
        };
        let head = plan(vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from,
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        }]);
        let mut routed = plan(vec![PlanStep {
            scope: StepScope::PerFile,
            operation,
        }]);
        routed.parameters = parameters;
        let plans = vec![head, routed];
        Self {
            rules: rules(&plans),
            intents: vec![collected(); plans.len()],
            plans,
            bindings: vec![vec![], bindings],
            hosts,
            observations: ObservationIdentity::Independent,
        }
    }
    /// Named producers whose captured head is read by a pure projection.
    pub(in super::super) fn named_continuation() -> Self {
        let mut authored = Self::named();
        for plan in &mut authored.plans[..2] {
            plan.steps.push(PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Step(StepId(0)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            });
            plan.emit.step = StepId(1);
        }
        authored
    }
    /// Two ordered inputs of one combining operation, each a head, a pure projection and an
    /// Extend that can fault; `held` gives every head a second reader so nothing fuses.
    pub(in super::super) fn combined(
        combine: Combine,
        left: Feed,
        right: Feed,
        fusion: Fusion,
    ) -> Self {
        let mut steps = vec![];
        let mut feed = |feed: Feed, offset: i64| {
            let head = steps.len() as u32;
            let threshold = match feed {
                Feed::Empty => 100 + offset,
                Feed::Healthy | Feed::Failing => offset,
            };
            let literal = match feed {
                Feed::Failing => 99,
                Feed::Empty | Feed::Healthy => 7 + offset,
            };
            steps.push(PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::LanguageTarget),
                    comparisons: vec![comparison(Test::GreaterThan, Integer(threshold))],
                    kept: vec![ColumnIndex(1)],
                },
            });
            steps.push(PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Step(StepId(head)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            });
            steps.push(PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Extend {
                    from: PlanInput::Step(StepId(head + 1)),
                    values: vec![Operand::Literal(QueryLiteral::Number(Integer(literal)))],
                },
            });
            head
        };
        let left_head = feed(left, 0);
        let right_head = feed(right, 1);
        let left_input = PlanInput::Step(StepId(left_head + 2));
        let right_input = PlanInput::Step(StepId(right_head + 2));
        let keys = vec![ColumnIndex(0)];
        let operation = match combine {
            Combine::Join => Operation::Join {
                left: left_input,
                right: right_input,
                left_on: keys.clone(),
                right_on: keys,
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            },
            Combine::Union => Operation::Union {
                left: left_input,
                right: right_input,
            },
            Combine::Intersection => Operation::Intersection {
                left: left_input,
                right: right_input,
            },
            Combine::Difference => Operation::Difference {
                left: left_input,
                right: right_input,
            },
            Combine::Antijoin => Operation::Antijoin {
                left: left_input,
                right: right_input,
                left_on: keys.clone(),
                right_on: keys,
            },
        };
        let emit = steps.len() as u32;
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation,
        });
        if fusion == Fusion::Held {
            for head in [left_head, right_head] {
                steps.push(PlanStep {
                    scope: StepScope::PerFile,
                    operation: Operation::Select {
                        from: PlanInput::Step(StepId(head)),
                        comparisons: vec![],
                        kept: vec![],
                    },
                });
            }
        }
        let mut combined = plan(steps);
        combined.emit.step = StepId(emit);
        let plans = vec![combined];
        Self {
            rules: rules(&plans),
            intents: vec![collected()],
            plans,
            bindings: vec![],
            hosts: vec![],
            observations: ObservationIdentity::Independent,
        }
    }
}

/// The combining operations that admit two ordered inputs of the same schema.
#[derive(Clone, Copy, Debug)]
pub(in super::super) enum Combine {
    Join,
    Union,
    Intersection,
    Difference,
    Antijoin,
}
impl Combine {
    pub(in super::super) const ALL: [Self; 5] = [
        Self::Join,
        Self::Union,
        Self::Intersection,
        Self::Difference,
        Self::Antijoin,
    ];
}
/// What one input of a combining operation delivers.
#[derive(Clone, Copy, Debug)]
pub(in super::super) enum Feed {
    /// No row survives its head; its Extend is never reached.
    Empty,
    /// Rows whose Extend succeeds.
    Healthy,
    /// Rows whose Extend faults.
    Failing,
}
impl Feed {
    pub(in super::super) const ALL: [Self; 3] = [Self::Empty, Self::Healthy, Self::Failing];
}
/// Whether the pure projections of the program may fuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Fusion {
    Fused,
    Held,
}
