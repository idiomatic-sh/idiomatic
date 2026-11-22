//! A step as the plan's node object: `{"op": "base", "rel": ..}`, `{"op": "select", "src": .., "items": .., "cond": ..}`
//! with the planner's `semi` and `count`, `{"op": <operator>, "args": {..}}` (an axis with the planner's `exists`), and
//! an operator outside the step set with its `args` as written.

use super::super::arguments::schemas::{AntijoinSchema, AsOfSchema, CaptureSchema, EdgeSchema, ExtendSchema, FitsRenderedSchema, GroupCountSchema, GroupOrderSchema, GroupOrdinalSchema, OrderedDfsSchema, OrderedMatchSchema, ProjectSchema, RenderSyntaxSchema, ReplaceSchema, RowsSchema, SourceMatchesSchema, SpanBetweenSchema, SpanBoundsSchema, UnionSchema, AxisSchema};
use super::super::expressions::schemas::{ConstantSchema, ExpressionSchema};
use super::super::fields::schemas::RelSchema;
use super::super::ordered::schemas::OrderedMap;
use crate::domain::finish_plans::arguments::groups::types::{GroupCountArguments, GroupOrderArguments, GroupOrdinalArguments, ProjectArguments};
use crate::domain::finish_plans::arguments::joins::types::{AntijoinArguments, AsOfArguments, OrderedMatchArguments, UnionArguments};
use crate::domain::finish_plans::arguments::literals::types::{ArgumentValue, ExtendArguments, RowsArguments, StepArgumentName, UnlistedArgument, UnlistedArguments};
use crate::domain::finish_plans::arguments::texts::types::{CaptureArguments, FitsRenderedArguments, RenderSyntaxArguments, ReplaceArguments, SourceMatchesArguments, SpanBetweenArguments, SpanBoundsArguments};
use crate::domain::finish_plans::arguments::types::StepNumber;
use crate::domain::finish_plans::arguments::walks::types::{AxisArguments, EdgeArguments, OrderedDfsArguments};
use crate::domain::finish_plans::expressions::types::SourceAlias;
use crate::domain::finish_plans::types::{
    AxisStep, CountMark, ExistsMark, FinishStep, ProbeMark, RowCondition, SelectItem, SelectSource,
    SelectStep, SelectedColumns, SourceOrdinal,
};
use ryo_values::domain::descriptions::types::{ColumnName, OperatorName, RelationName};
use ryo_values::domain::query_tokens::types::AliasName;
use serde::{Deserialize, Serialize};

/// A node: of an operator of the step set, or of one outside it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(in crate::outbound::plan_codec::adapters::json) enum NodeSchema {
    /// Of the step set, or a relation or a select.
    Listed(ListedNodeSchema),
    /// Outside the step set.
    Unlisted(UnlistedNodeSchema),
}

/// A node whose `op` is a step kind of the plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op")]
pub(in crate::outbound::plan_codec::adapters::json) enum ListedNodeSchema {
    /// A relation of the catalog.
    #[serde(rename = "base")]
    Base {
        /// Its name.
        rel: String,
    },
    /// A select.
    #[serde(rename = "select")]
    Select {
        /// Its sources: `[step, alias]`.
        src: Vec<(usize, Option<String>)>,
        /// Its items, `[name, expression]`, or `null` for `*`.
        items: Option<Vec<(String, ExpressionSchema)>>,
        /// Its condition, or `null`.
        cond: Option<ExpressionSchema>,
        /// The probed source of a semijoin.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        semi: Option<usize>,
        /// Whether it is answered by counts.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count: Option<bool>,
    },
    /// An axis.
    #[serde(rename = "axis")]
    Axis {
        /// Its arguments.
        #[serde(with = "AxisSchema")]
        args: AxisArguments,
        /// Whether only existence is asked.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exists: Option<bool>,
    },
    /// `union`.
    #[serde(rename = "union")]
    Union {
        /// Its arguments.
        #[serde(with = "UnionSchema")]
        args: UnionArguments,
    },
    /// `antijoin`.
    #[serde(rename = "antijoin")]
    Antijoin {
        /// Its arguments.
        #[serde(with = "AntijoinSchema")]
        args: AntijoinArguments,
    },
    /// `as-of`.
    #[serde(rename = "as-of")]
    AsOf {
        /// Its arguments.
        #[serde(with = "AsOfSchema")]
        args: AsOfArguments,
    },
    /// `ordered-match`.
    #[serde(rename = "ordered-match")]
    OrderedMatch {
        /// Its arguments.
        #[serde(with = "OrderedMatchSchema")]
        args: OrderedMatchArguments,
    },
    /// `closure`.
    #[serde(rename = "closure")]
    Closure {
        /// Its arguments.
        #[serde(with = "EdgeSchema")]
        args: EdgeArguments,
    },
    /// `follow`.
    #[serde(rename = "follow")]
    Follow {
        /// Its arguments.
        #[serde(with = "EdgeSchema")]
        args: EdgeArguments,
    },
    /// `ordered-dfs`.
    #[serde(rename = "ordered-dfs")]
    OrderedDfs {
        /// Its arguments.
        #[serde(with = "OrderedDfsSchema")]
        args: OrderedDfsArguments,
    },
    /// `group-count`.
    #[serde(rename = "group-count")]
    GroupCount {
        /// Its arguments.
        #[serde(with = "GroupCountSchema")]
        args: GroupCountArguments,
    },
    /// `group-min`.
    #[serde(rename = "group-min")]
    GroupMin {
        /// Its arguments.
        #[serde(with = "GroupOrderSchema")]
        args: GroupOrderArguments,
    },
    /// `group-max`.
    #[serde(rename = "group-max")]
    GroupMax {
        /// Its arguments.
        #[serde(with = "GroupOrderSchema")]
        args: GroupOrderArguments,
    },
    /// `group-ordinal`.
    #[serde(rename = "group-ordinal")]
    GroupOrdinal {
        /// Its arguments.
        #[serde(with = "GroupOrdinalSchema")]
        args: GroupOrdinalArguments,
    },
    /// `project`.
    #[serde(rename = "project")]
    Project {
        /// Its arguments.
        #[serde(with = "ProjectSchema")]
        args: ProjectArguments,
    },
    /// `extend`.
    #[serde(rename = "extend")]
    Extend {
        /// Its arguments.
        #[serde(with = "ExtendSchema")]
        args: ExtendArguments,
    },
    /// `span-bounds`.
    #[serde(rename = "span-bounds")]
    SpanBounds {
        /// Its arguments.
        #[serde(with = "SpanBoundsSchema")]
        args: SpanBoundsArguments,
    },
    /// `span-between`.
    #[serde(rename = "span-between")]
    SpanBetween {
        /// Its arguments.
        #[serde(with = "SpanBetweenSchema")]
        args: SpanBetweenArguments,
    },
    /// `replace`.
    #[serde(rename = "replace")]
    Replace {
        /// Its arguments.
        #[serde(with = "ReplaceSchema")]
        args: ReplaceArguments,
    },
    /// `source-matches`.
    #[serde(rename = "source-matches")]
    SourceMatches {
        /// Its arguments.
        #[serde(with = "SourceMatchesSchema")]
        args: SourceMatchesArguments,
    },
    /// `capture`.
    #[serde(rename = "capture")]
    Capture {
        /// Its arguments.
        #[serde(with = "CaptureSchema")]
        args: CaptureArguments,
    },
    /// `fits-rendered`.
    #[serde(rename = "fits-rendered")]
    FitsRendered {
        /// Its arguments.
        #[serde(with = "FitsRenderedSchema")]
        args: FitsRenderedArguments,
    },
    /// `render-syntax`.
    #[serde(rename = "render-syntax")]
    RenderSyntax {
        /// Its arguments.
        #[serde(with = "RenderSyntaxSchema")]
        args: RenderSyntaxArguments,
    },
    /// `rows`.
    #[serde(rename = "rows")]
    Rows {
        /// Its arguments.
        #[serde(with = "RowsSchema")]
        args: RowsArguments,
    },
}

/// A node whose operator is outside the step set: its arguments as written.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) struct UnlistedNodeSchema {
    /// The operator.
    op: String,
    /// Its arguments, in order.
    args: OrderedMap<ArgumentSchema>,
}

/// An argument of an unlisted operator: a step, or a constant.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(in crate::outbound::plan_codec::adapters::json) enum ArgumentSchema {
    /// `{"rel": n}`.
    Step(RelSchema),
    /// A constant.
    Constant(ConstantSchema),
}

/// An alias as written: none, or its name.
fn alias_text(alias: &SourceAlias) -> Option<String> {
    match alias {
        SourceAlias::Unwritten => None,
        SourceAlias::Written(name) => Some(name.0.to_string()),
    }
}

impl From<&FinishStep> for NodeSchema {
    fn from(step: &FinishStep) -> Self {
        Self::Listed(match step {
            FinishStep::Base(relation) => ListedNodeSchema::Base { rel: relation.0.to_string() },
            FinishStep::Select(select) => ListedNodeSchema::Select {
                src: select.sources.iter().map(|source| (source.step.0, alias_text(&source.alias))).collect(),
                items: match &select.columns {
                    SelectedColumns::All => None,
                    SelectedColumns::Items(items) => Some(
                        items.iter().map(|item| (item.name.0.to_string(), ExpressionSchema::from(&item.value))).collect(),
                    ),
                },
                cond: match &select.condition {
                    RowCondition::Every => None,
                    RowCondition::Tested(condition) => Some(condition.into()),
                },
                semi: match select.probe {
                    ProbeMark::Unmarked => None,
                    ProbeMark::Probed(place) => Some(place.0),
                },
                count: match select.count {
                    CountMark::Unmarked => None,
                    CountMark::Counted => Some(true),
                },
            },
            FinishStep::Axis(axis) => ListedNodeSchema::Axis {
                args: axis.arguments.clone(),
                exists: match axis.exists {
                    ExistsMark::Unmarked => None,
                    ExistsMark::Marked => Some(true),
                },
            },
            FinishStep::Union(args) => ListedNodeSchema::Union { args: args.clone() },
            FinishStep::Antijoin(args) => ListedNodeSchema::Antijoin { args: args.clone() },
            FinishStep::AsOf(args) => ListedNodeSchema::AsOf { args: args.clone() },
            FinishStep::OrderedMatch(args) => ListedNodeSchema::OrderedMatch { args: args.clone() },
            FinishStep::Closure(args) => ListedNodeSchema::Closure { args: args.clone() },
            FinishStep::Follow(args) => ListedNodeSchema::Follow { args: args.clone() },
            FinishStep::OrderedDfs(args) => ListedNodeSchema::OrderedDfs { args: args.clone() },
            FinishStep::GroupCount(args) => ListedNodeSchema::GroupCount { args: args.clone() },
            FinishStep::GroupMin(args) => ListedNodeSchema::GroupMin { args: args.clone() },
            FinishStep::GroupMax(args) => ListedNodeSchema::GroupMax { args: args.clone() },
            FinishStep::GroupOrdinal(args) => ListedNodeSchema::GroupOrdinal { args: args.clone() },
            FinishStep::Project(args) => ListedNodeSchema::Project { args: args.clone() },
            FinishStep::Extend(args) => ListedNodeSchema::Extend { args: args.clone() },
            FinishStep::SpanBounds(args) => ListedNodeSchema::SpanBounds { args: args.clone() },
            FinishStep::SpanBetween(args) => ListedNodeSchema::SpanBetween { args: args.clone() },
            FinishStep::Replace(args) => ListedNodeSchema::Replace { args: args.clone() },
            FinishStep::SourceMatches(args) => ListedNodeSchema::SourceMatches { args: args.clone() },
            FinishStep::Capture(args) => ListedNodeSchema::Capture { args: args.clone() },
            FinishStep::FitsRendered(args) => ListedNodeSchema::FitsRendered { args: args.clone() },
            FinishStep::RenderSyntax(args) => ListedNodeSchema::RenderSyntax { args: args.clone() },
            FinishStep::Rows(args) => ListedNodeSchema::Rows { args: args.clone() },
            FinishStep::Unlisted(unlisted) => {
                return Self::Unlisted(UnlistedNodeSchema {
                    op: unlisted.operator.0.to_string(),
                    args: OrderedMap(
                        unlisted
                            .arguments
                            .iter()
                            .map(|argument| {
                                let value = match &argument.value {
                                    ArgumentValue::Step(step) => ArgumentSchema::Step(RelSchema { rel: step.0 }),
                                    ArgumentValue::Constant(constant) => ArgumentSchema::Constant(constant.into()),
                                };
                                (argument.name.0.to_string(), value)
                            })
                            .collect(),
                    ),
                });
            }
        })
    }
}

impl From<NodeSchema> for FinishStep {
    fn from(node: NodeSchema) -> Self {
        let listed = match node {
            NodeSchema::Listed(listed) => listed,
            NodeSchema::Unlisted(unlisted) => {
                return Self::Unlisted(UnlistedArguments {
                    operator: OperatorName(unlisted.op.into()),
                    arguments: unlisted
                        .args
                        .0
                        .into_iter()
                        .map(|(name, value)| UnlistedArgument {
                            name: StepArgumentName(name.into()),
                            value: match value {
                                ArgumentSchema::Step(step) => ArgumentValue::Step(StepNumber(step.rel)),
                                ArgumentSchema::Constant(constant) => ArgumentValue::Constant(constant.into()),
                            },
                        })
                        .collect(),
                });
            }
        };
        match listed {
            ListedNodeSchema::Base { rel } => Self::Base(RelationName(rel.into())),
            ListedNodeSchema::Select { src, items, cond, semi, count } => Self::Select(SelectStep {
                sources: src
                    .into_iter()
                    .map(|(step, alias)| SelectSource {
                        step: StepNumber(step),
                        alias: alias.map_or(SourceAlias::Unwritten, |alias| SourceAlias::Written(AliasName(alias.into()))),
                    })
                    .collect(),
                columns: items.map_or(SelectedColumns::All, |items| {
                    SelectedColumns::Items(
                        items
                            .into_iter()
                            .map(|(name, value)| SelectItem { name: ColumnName(name.into()), value: value.into() })
                            .collect(),
                    )
                }),
                condition: cond.map_or(RowCondition::Every, |condition| RowCondition::Tested(condition.into())),
                probe: semi.map_or(ProbeMark::Unmarked, |place| ProbeMark::Probed(SourceOrdinal(place))),
                count: if count.is_some() { CountMark::Counted } else { CountMark::Unmarked },
            }),
            ListedNodeSchema::Axis { args, exists } => Self::Axis(AxisStep {
                arguments: args,
                exists: if exists.is_some() { ExistsMark::Marked } else { ExistsMark::Unmarked },
            }),
            ListedNodeSchema::Union { args } => Self::Union(args),
            ListedNodeSchema::Antijoin { args } => Self::Antijoin(args),
            ListedNodeSchema::AsOf { args } => Self::AsOf(args),
            ListedNodeSchema::OrderedMatch { args } => Self::OrderedMatch(args),
            ListedNodeSchema::Closure { args } => Self::Closure(args),
            ListedNodeSchema::Follow { args } => Self::Follow(args),
            ListedNodeSchema::OrderedDfs { args } => Self::OrderedDfs(args),
            ListedNodeSchema::GroupCount { args } => Self::GroupCount(args),
            ListedNodeSchema::GroupMin { args } => Self::GroupMin(args),
            ListedNodeSchema::GroupMax { args } => Self::GroupMax(args),
            ListedNodeSchema::GroupOrdinal { args } => Self::GroupOrdinal(args),
            ListedNodeSchema::Project { args } => Self::Project(args),
            ListedNodeSchema::Extend { args } => Self::Extend(args),
            ListedNodeSchema::SpanBounds { args } => Self::SpanBounds(args),
            ListedNodeSchema::SpanBetween { args } => Self::SpanBetween(args),
            ListedNodeSchema::Replace { args } => Self::Replace(args),
            ListedNodeSchema::SourceMatches { args } => Self::SourceMatches(args),
            ListedNodeSchema::Capture { args } => Self::Capture(args),
            ListedNodeSchema::FitsRendered { args } => Self::FitsRendered(args),
            ListedNodeSchema::RenderSyntax { args } => Self::RenderSyntax(args),
            ListedNodeSchema::Rows { args } => Self::Rows(args),
        }
    }
}
