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
            let mut rels = Vec::new();
            visit_slots(step, |name, source| rels.push((name, source)));
            // The original slot reader requests named operator inputs in field-name order.
            rels.sort_unstable_by(|a, b| a.0.cmp(b.0));
            let inputs = if let FinishStep::Select(s) = step {
                s.sources.iter().map(|s| s.step.0).collect()
            } else {
                rels.iter().map(|x| x.1).collect()
            };
            PNode {
                inputs,
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
        frames: Default::default(),
        consumers: Default::default(),
        nodes,
        emits,
    }
}
/// Read dependency names from the immutable graph shared with lowering.
fn visit_slots<'a>(step: &'a FinishStep, mut visit: impl FnMut(&'a str, usize)) -> &'a str {
    use FinishStep::*;
    match step {
        Base(_) => "base",
        Select(_) => "select",
        Union(a) => {
            visit("left", a.left.0);
            visit("right", a.right.0);
            "union"
        }
        Antijoin(a) => {
            visit("left", a.left.0);
            visit("right", a.right.0);
            "antijoin"
        }
        AsOf(a) => {
            visit("left", a.left.0);
            visit("right", a.right.0);
            "as-of"
        }
        OrderedMatch(a) => {
            visit("left", a.left.0);
            visit("right", a.right.0);
            "ordered-match"
        }
        Axis(a) => {
            visit("from", a.arguments.from.0);
            visit("relation", a.arguments.relation.0);
            visit("domain", a.arguments.domain.0);
            "axis"
        }
        Closure(a) => {
            visit("edges", a.edges.0);
            "closure"
        }
        Follow(a) => {
            visit("edges", a.edges.0);
            "follow"
        }
        OrderedDfs(a) => {
            visit("from", a.from.0);
            visit("edges", a.edges.0);
            "ordered-dfs"
        }
        GroupCount(a) => {
            visit("from", a.from.0);
            "group-count"
        }
        GroupMin(a) => {
            visit("from", a.from.0);
            "group-min"
        }
        GroupMax(a) => {
            visit("from", a.from.0);
            "group-max"
        }
        GroupOrdinal(a) => {
            visit("from", a.from.0);
            "group-ordinal"
        }
        Project(a) => {
            visit("from", a.from.0);
            "project"
        }
        Extend(a) => {
            visit("from", a.from.0);
            "extend"
        }
        SpanBounds(a) => {
            visit("from", a.from.0);
            "span-bounds"
        }
        SpanBetween(a) => {
            visit("from", a.from.0);
            "span-between"
        }
        Replace(a) => {
            visit("from", a.from.0);
            "replace"
        }
        SourceMatches(a) => {
            visit("from", a.from.0);
            "source-matches"
        }
        Capture(a) => {
            visit("from", a.from.0);
            "capture"
        }
        FitsRendered(a) => {
            visit("from", a.from.0);
            "fits-rendered"
        }
        RenderSyntax(a) => {
            visit("from", a.from.0);
            "render-syntax"
        }
        Rows(_) => "rows",
        Prefixes(a) => {
            visit("from", a.from.0);
            "prefixes"
        }
        SubtreeClass(a) => {
            visit("from", a.from.0);
            visit("relation", a.relation.0);
            visit("labels", a.labels.0);
            visit("leaves", a.leaves.0);
            "subtree-class"
        }
        Unlisted(a) => {
            for arg in &a.arguments {
                if let ArgumentValue::Step(i) = &arg.value {
                    visit(&arg.name.0, i.0);
                }
            }
            &a.operator.0
        }
    }
}
impl PNode {
    pub(crate) fn op(&self) -> &str {
        visit_slots(self.step(), |_, _| {})
    }

    pub(crate) fn relation_input(&self, name: &str) -> Option<usize> {
        let mut input = None;
        visit_slots(self.step(), |key, source| {
            if key == name && input.is_none() { input = Some(source); }
        });
        input
    }
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
            set("capture-columns", Param::CaptureColumns(super::capture::columns(
                &a.pattern.0, a.captures.iter().map(|column| column.0.as_ref()),
            )));
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
