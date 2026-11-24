//! Direct handoff from the lowered slot graph. The first walk and executor share the same
//! immutable steps.
use super::plan_args::Param;
use super::{Args, PNode, Plan};
use ryo_values::domain::finish_plans::{
    arguments::{
        groups::types::CountedRows, joins::types::AsOfBound, literals::types::ArgumentValue,
        texts::types::OptionalColumn, walks::types::EdgeKeys,
    },
    expressions::types::*,
    types::*,
};
use ryo_values::domain::plan_parts::types::Side;
use std::collections::BTreeMap;
pub(crate) fn from_typed(plan: std::sync::Arc<FinishPlan>) -> Plan {
    let nodes = plan
        .steps
        .iter()
        .enumerate()
        .map(|(id, step)| {
            let (op, mut rels) = slots(step);
            // The original slot reader requests named operator inputs in field-name order.
            rels.sort_unstable_by(|a, b| a.0.cmp(&b.0));
            let inputs = if let FinishStep::Select(s) = step {
                s.sources.iter().map(|s| s.step.0).collect()
            } else {
                rels.iter().map(|x| x.1).collect()
            };
            PNode {
                op: op.to_owned(),
                inputs,
                rels,
                source: plan.clone(),
                slot: id,
                args: Default::default(),
            }
        })
        .collect();
    let mut emits: Vec<_> = plan
        .rules
        .iter()
        .map(|r| {
            let RuleEmit::Written(e) = &r.emit else {
                panic!("no emit node for {}", r.rule.0)
            };
            (
                r.rule.0.to_string(),
                e.step.0,
                e.subject.0.to_string(),
                e.key.iter().map(|k| k.0.to_string()).collect(),
            )
        })
        .collect();
    emits.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    Plan {
        nodes,
        emits,
    }
}
fn slots(step: &FinishStep) -> (&str, Vec<(String, usize)>) {
    use FinishStep::*;
    match step {
        Base(_) => ("base", vec![]),
        Select(_) => ("select", vec![]),
        Union(a) => (
            "union",
            vec![("left".into(), a.left.0), ("right".into(), a.right.0)],
        ),
        Antijoin(a) => (
            "antijoin",
            vec![("left".into(), a.left.0), ("right".into(), a.right.0)],
        ),
        AsOf(a) => (
            "as-of",
            vec![("left".into(), a.left.0), ("right".into(), a.right.0)],
        ),
        OrderedMatch(a) => (
            "ordered-match",
            vec![("left".into(), a.left.0), ("right".into(), a.right.0)],
        ),
        Axis(a) => (
            "axis",
            vec![
                ("from".into(), a.arguments.from.0),
                ("relation".into(), a.arguments.relation.0),
                ("domain".into(), a.arguments.domain.0),
            ],
        ),
        Closure(a) => ("closure", vec![("edges".into(), a.edges.0)]),
        Follow(a) => ("follow", vec![("edges".into(), a.edges.0)]),
        OrderedDfs(a) => (
            "ordered-dfs",
            vec![("from".into(), a.from.0), ("edges".into(), a.edges.0)],
        ),
        GroupCount(a) => ("group-count", vec![("from".into(), a.from.0)]),
        GroupMin(a) => ("group-min", vec![("from".into(), a.from.0)]),
        GroupMax(a) => ("group-max", vec![("from".into(), a.from.0)]),
        GroupOrdinal(a) => ("group-ordinal", vec![("from".into(), a.from.0)]),
        Project(a) => ("project", vec![("from".into(), a.from.0)]),
        Extend(a) => ("extend", vec![("from".into(), a.from.0)]),
        SpanBounds(a) => ("span-bounds", vec![("from".into(), a.from.0)]),
        SpanBetween(a) => ("span-between", vec![("from".into(), a.from.0)]),
        Replace(a) => ("replace", vec![("from".into(), a.from.0)]),
        SourceMatches(a) => ("source-matches", vec![("from".into(), a.from.0)]),
        Capture(a) => ("capture", vec![("from".into(), a.from.0)]),
        FitsRendered(a) => ("fits-rendered", vec![("from".into(), a.from.0)]),
        RenderSyntax(a) => ("render-syntax", vec![("from".into(), a.from.0)]),
        Rows(_) => ("rows", vec![]),
        Prefixes(a) => ("prefixes", vec![("from".into(), a.from.0)]),
        SubtreeClass(a) => (
            "subtree-class",
            vec![
                ("from".into(), a.from.0),
                ("relation".into(), a.relation.0),
                ("labels".into(), a.labels.0),
                ("leaves".into(), a.leaves.0),
            ],
        ),
        Unlisted(a) => (
            &a.operator.0,
            a.arguments
                .iter()
                .filter_map(|arg| match &arg.value {
                    ArgumentValue::Step(i) => Some((arg.name.0.to_string(), i.0)),
                    _ => None,
                })
                .collect(),
        ),
    }
}
impl PNode {
    pub(super) fn summary_text(&self) -> String {
        super::json_output::step(self.step())
    }
    pub(crate) fn step(&self) -> &FinishStep {
        &self.source.steps[self.slot]
    }
    pub(crate) fn base(&self) -> Option<&str> {
        match self.step() {
            FinishStep::Base(r) => Some(&r.0),
            _ => None,
        }
    }
    pub(crate) fn semi(&self) -> Option<usize> {
        match self.step() {
            FinishStep::Select(s) => match s.probe {
                ProbeMark::Probed(n) => Some(n.0),
                _ => None,
            },
            _ => None,
        }
    }
    pub(super) fn distinct_count(&self) -> bool {
        match self.step() {
            FinishStep::GroupCount(g) => matches!(g.distinct, CountedRows::Distinct(_)),
            _ => false,
        }
    }
    pub(crate) fn counted(&self) -> bool {
        match self.step() {
            FinishStep::Select(s) => s.count == CountMark::Counted,
            _ => false,
        }
    }
    pub(crate) fn tested(&self) -> bool {
        match self.step() {
            FinishStep::Select(s) => s.condition != RowCondition::Every,
            _ => false,
        }
    }
    pub(crate) fn exists(&self) -> bool {
        match self.step() {
            FinishStep::Axis(s) => s.exists == ExistsMark::Marked,
            _ => false,
        }
    }
    pub(crate) fn args(&self) -> &Args {
        self.args.get_or_init(|| parameters(self.step()))
    }
}
fn parameters(step: &FinishStep) -> Args {
    use FinishStep::*;
    let mut m = BTreeMap::new();
    let mut set = |key: &str, value: Param| {
        m.insert(key.to_owned(), value);
    };
    match step {
        Base(_) => {}
        Select(_) => {}
        Union(_) => {}
        Antijoin(a) => {
            set(
                "left-on",
                Param::strings(a.left_on.iter().map(|v| v.0.as_ref())),
            );
            set(
                "right-on",
                Param::strings(a.right_on.iter().map(|v| v.0.as_ref())),
            );
        }
        AsOf(a) => {
            set(
                "left-on",
                Param::strings(a.left_on.iter().map(|v| v.0.as_ref())),
            );
            set(
                "right-on",
                Param::strings(a.right_on.iter().map(|v| v.0.as_ref())),
            );
            set("left-order", Param::text(&a.left_order.0));
            set("right-order", Param::text(&a.right_order.0));
            set(
                "bound",
                Param::text(match a.bound {
                    AsOfBound::AtOrBefore => "at-or-before",
                    AsOfBound::Before => "before",
                    AsOfBound::AtOrAfter => "at-or-after",
                    AsOfBound::After => "after",
                }),
            );
            set(
                "output",
                Param::Named(
                    a.output
                        .iter()
                        .map(|v| {
                            (
                                v.name.0.to_string(),
                                Param::strings(
                                    [
                                        match v.side {
                                            Side::Left => "left",
                                            Side::Right => "right",
                                        },
                                        &v.column.0,
                                    ]
                                    .into_iter(),
                                ),
                            )
                        })
                        .collect(),
                ),
            );
        }
        OrderedMatch(a) => {
            set(
                "left-on",
                Param::strings(a.left_on.iter().map(|v| v.0.as_ref())),
            );
            set(
                "right-on",
                Param::strings(a.right_on.iter().map(|v| v.0.as_ref())),
            );
            set("left-order", Param::text(&a.left_order.0));
            set("right-order", Param::text(&a.right_order.0));
            set(
                "output",
                Param::Named(
                    a.output
                        .iter()
                        .map(|v| {
                            (
                                v.name.0.to_string(),
                                Param::strings(
                                    [
                                        match v.side {
                                            Side::Left => "left",
                                            Side::Right => "right",
                                        },
                                        &v.column.0,
                                    ]
                                    .into_iter(),
                                ),
                            )
                        })
                        .collect(),
                ),
            );
        }
        Axis(a) => {
            set(
                "direction",
                Param::text(ryo_values::domain::finish_plans::service::direction_name(
                    a.arguments.direction,
                )),
            );
        }
        Closure(a) => {
            set("source", Param::text(&a.source.0));
            set("destination", Param::text(&a.destination.0));
            if let EdgeKeys::Written(cols) = &a.keys {
                set("keys", Param::strings(cols.iter().map(|c| c.0.as_ref())));
            }
        }
        Follow(a) => {
            set("source", Param::text(&a.source.0));
            set("destination", Param::text(&a.destination.0));
            if let EdgeKeys::Written(cols) = &a.keys {
                set("keys", Param::strings(cols.iter().map(|c| c.0.as_ref())));
            }
        }
        OrderedDfs(a) => {
            set("root", Param::text(&a.root.0));
            set("source", Param::text(&a.source.0));
            set("destination", Param::text(&a.destination.0));
            set("order", Param::text(&a.order.0));
            set("edge", Param::text(&a.edge.0));
            set("keys", Param::strings(a.keys.iter().map(|v| v.0.as_ref())));
            set("output", Param::text(&a.output.0));
        }
        GroupCount(a) => {
            set("keys", Param::strings(a.keys.iter().map(|v| v.0.as_ref())));
            set("count", Param::text(&a.count.0));
            if let CountedRows::Distinct(col) = &a.distinct {
                set("distinct", Param::text(&col.0));
            }
        }
        GroupMin(a) => {
            set("keys", Param::strings(a.keys.iter().map(|v| v.0.as_ref())));
            set("order", Param::text(&a.order.0));
        }
        GroupMax(a) => {
            set("keys", Param::strings(a.keys.iter().map(|v| v.0.as_ref())));
            set("order", Param::text(&a.order.0));
        }
        GroupOrdinal(a) => {
            set("keys", Param::strings(a.keys.iter().map(|v| v.0.as_ref())));
            set("order", Param::text(&a.order.0));
            set("output", Param::text(&a.output.0));
        }
        Project(a) => {
            set(
                "columns",
                Param::strings(a.columns.iter().map(|v| v.0.as_ref())),
            );
        }
        Extend(a) => {
            set("value-order", Param::strings(a.values.iter().map(|v|v.name.0.as_ref())));
            set(
                "values",
                Param::Named(
                    a.values
                        .iter()
                        .map(|v| (v.name.0.to_string(), Param::from_constant(&v.value)))
                        .collect(),
                ),
            );
        }
        SpanBounds(a) => {
            set("span", Param::text(&a.span.0));
            set("start-output", Param::text(&a.start_output.0));
            set("end-output", Param::text(&a.end_output.0));
        }
        SpanBetween(a) => {
            set("start", Param::text(&a.start.0));
            set("end", Param::text(&a.end.0));
            set("output", Param::text(&a.output.0));
        }
        Replace(a) => {
            set("text", Param::text(&a.text.0));
            set("pattern", Param::text(&a.pattern.0));
            set("replacement", Param::text(&a.replacement.0));
            set("output", Param::text(&a.output.0));
        }
        SourceMatches(a) => {
            set("span", Param::text(&a.span.0));
            if let OptionalColumn::Written(col) = &a.base_line {
                set("base-line", Param::text(&col.0));
            }
            set("pattern", Param::text(&a.pattern.0));
            set("output", Param::text(&a.output.0));
            if let OptionalColumn::Written(col) = &a.line_output {
                set("line-output", Param::text(&col.0));
            }
            if let OptionalColumn::Written(col) = &a.capture {
                set("capture", Param::text(&col.0));
            }
            if let OptionalColumn::Written(col) = &a.capture_output {
                set("capture-output", Param::text(&col.0));
            }
            if let OptionalColumn::Written(col) = &a.capture_text_output {
                set("capture-text-output", Param::text(&col.0));
            }
            if let OptionalColumn::Written(col) = &a.capture_line_output {
                set("capture-line-output", Param::text(&col.0));
            }
            if !a.capture_texts.is_empty() {
                set(
                    "capture-texts",
                    Param::Named(
                        a.capture_texts
                            .iter()
                            .map(|v| (v.group.0.to_string(), Param::text(&v.output.0)))
                            .collect(),
                    ),
                );
            }
        }
        Capture(a) => {
            if let Some(sep)=&a.separator {set("separator",Param::text(&sep.0));}
            set("path", Param::text(&a.path.0));
            set("pattern", Param::text(&a.pattern.0));
            set(
                "captures",
                Param::strings(a.captures.iter().map(|v| v.0.as_ref())),
            );
        }
        FitsRendered(a) => {
            set(
                "texts",
                Param::strings(a.texts.iter().map(|v| v.0.as_ref())),
            );
            set(
                "literals",
                Param::strings(a.literals.iter().map(|v| v.0.as_ref())),
            );
            set("span", Param::text(&a.span.0));
            set("max-line", Param::Number(PlanNumber::Whole(a.max_line)));
            set("tab-width", Param::Number(PlanNumber::Whole(a.tab_width)));
        }
        RenderSyntax(a) => {
            set("node", Param::text(&a.node.0));
            set("context", Param::text(&a.context.0));
            set("output", Param::text(&a.output.0));
        }
        Rows(a) => {
            set(
                "from",
                Param::list(
                    a.rows
                        .iter()
                        .map(|r| Param::strings(r.cells.iter().map(|v| v.0.as_ref())))
                        .collect(),
                ),
            );
            set(
                "columns",
                Param::strings(a.columns.iter().map(|v| v.0.as_ref())),
            );
        }
        Prefixes(a) => {
            set("text", Param::text(&a.text.0));
            set("separator", Param::text(&a.separator.0));
            set("output", Param::text(&a.output.0));
        }
        SubtreeClass(a) => {
            set(
                "label",
                Param::strings(a.label.iter().map(|v| v.0.as_ref())),
            );
            set(
                "leaf-label",
                Param::strings(a.leaf_label.iter().map(|v| v.0.as_ref())),
            );
            set("step", Param::strings(a.step.iter().map(|v| v.0.as_ref())));
            set("output", Param::text(&a.output.0));
            set("id", Param::text(&a.id.0));
            set("parent", Param::text(&a.parent.0));
            set("child", Param::text(&a.child.0));
        }
        Unlisted(a) => {
            for a in &a.arguments {
                if let ArgumentValue::Constant(v) = &a.value {
                    set(&a.name.0, Param::from_constant(v));
                }
            }
        }
    }
    Param::Named(m)
}
