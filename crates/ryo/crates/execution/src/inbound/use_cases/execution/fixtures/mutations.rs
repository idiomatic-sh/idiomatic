//! Every mutation of an operation a fault-coverage proof runs (`conformance_faults`): the match names every field of every operation, so a new field or operation does not
//! compile until it is listed here.
use super::mutation_fields::{
    BAD, DIRECTIONS, Mutation, Mutations, comparisons, outputs, patterns,
};
use ryo_language::domain::semantics::types::RenderContext;
use ryo_values::domain::plan_parts::types::{
    CapturePattern, HostedNumber, LiteralText, Operand, ParameterId, ReachLookup, RepeatedRows,
    TableKey,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::{LineOutput, MaxLine, TabWidth, TextOutput};

/// Field accessors: each the field of one operation kind, found in a copy of it.
macro_rules! field {
    ($variant:ident, $name:ident) => {
        |operation: &mut Operation| {
            let Operation::$variant { $name, .. } = operation else {
                return None;
            };
            Some($name)
        }
    };
}

/// Every mutation of `operation` read with `bindings`.
#[allow(
    clippy::too_many_lines,
    reason = "one arm per operation kind, each naming every field it has, so a new field or kind does not compile until listed"
)]
pub(crate) fn mutations(operation: &Operation, bindings: &[ParameterBinding]) -> Vec<Mutation> {
    let mut all = Mutations {
        base: operation,
        bindings,
        out: Vec::new(),
    };
    let columns = [BAD];
    match operation {
        Operation::Select {
            from: _,
            comparisons: _,
            kept: _,
        } => {
            all.elements("comparisons", field!(Select, comparisons), &comparisons());
            all.elements("kept", field!(Select, kept), &columns);
        }
        Operation::Project {
            from: _,
            columns: _,
            distinct: _,
        } => {
            all.elements("columns", field!(Project, columns), &columns);
            all.set(
                "distinct",
                field!(Project, distinct),
                &[RepeatedRows::Kept, RepeatedRows::Dropped],
            );
        }
        Operation::Extend { from: _, values: _ } => {
            all.elements(
                "values",
                field!(Extend, values),
                &[Operand::Column(BAD), Operand::Parameter(ParameterId(99))],
            );
        }
        Operation::Join {
            left: _,
            right: _,
            left_on: _,
            right_on: _,
            output: _,
            filter: _,
            view: _,
            reach: _,
        } => {
            all.elements("left_on", field!(Join, left_on), &columns);
            all.elements("right_on", field!(Join, right_on), &columns);
            all.elements("output", field!(Join, output), &outputs());
            all.elements("filter", field!(Join, filter), &comparisons());
            all.elements("view", field!(Join, view), &columns);
            all.set(
                "reach",
                field!(Join, reach),
                &[Some(ReachLookup {
                    source: BAD,
                    destination: BAD,
                    keys: vec![BAD],
                    columns: vec![BAD],
                })],
            );
        }
        Operation::Union { left: _, right: _ }
        | Operation::Intersection { left: _, right: _ }
        | Operation::Difference { left: _, right: _ } => {}
        Operation::Antijoin {
            left: _,
            right: _,
            left_on: _,
            right_on: _,
        } => {
            all.elements("left_on", field!(Antijoin, left_on), &columns);
            all.elements("right_on", field!(Antijoin, right_on), &columns);
        }
        Operation::GroupCount {
            from: _,
            keys: _,
            distinct: _,
        } => {
            all.elements("keys", field!(GroupCount, keys), &columns);
            all.set("distinct", field!(GroupCount, distinct), &[Some(BAD), None]);
        }
        Operation::GroupDigest {
            from: _,
            keys: _,
            over: _,
        } => {
            all.elements("keys", field!(GroupDigest, keys), &columns);
            all.set("over", field!(GroupDigest, over), &columns);
        }
        Operation::Capture {
            from: _,
            path: _,
            pattern: _,
        } => {
            all.set("path", field!(Capture, path), &columns);
            all.set(
                "pattern",
                field!(Capture, pattern),
                &[
                    CapturePattern("{".into()),
                    CapturePattern("".into()),
                    CapturePattern("{a}{a}".into()),
                ],
            );
        }
        Operation::Rank {
            from: _,
            column: _,
            sets: _,
        } => {
            all.set("column", field!(Rank, column), &columns);
            all.set("sets", field!(Rank, sets), &[ParameterId(99)]);
        }
        Operation::Replace {
            from: _,
            text: _,
            pattern: _,
            replacement: _,
        } => {
            all.set("text", field!(Replace, text), &columns);
            all.set("pattern", field!(Replace, pattern), &patterns());
            all.set("replacement", field!(Replace, replacement), &patterns());
        }
        Operation::OrderedMatch {
            left: _,
            right: _,
            left_on: _,
            right_on: _,
            left_order: _,
            right_order: _,
            output: _,
        } => {
            all.elements("left_on", field!(OrderedMatch, left_on), &columns);
            all.elements("right_on", field!(OrderedMatch, right_on), &columns);
            all.set("left_order", field!(OrderedMatch, left_order), &columns);
            all.set("right_order", field!(OrderedMatch, right_order), &columns);
            all.elements("output", field!(OrderedMatch, output), &outputs());
        }
        Operation::Words { from: _, text: _ } => all.set("text", field!(Words, text), &columns),
        Operation::Axis {
            from: _,
            edges: _,
            domain: _,
            direction: _,
            id: _,
            parent: _,
            child: _,
            domain_id: _,
            domain_filter: _,
            domain_view: _,
            output: _,
        } => {
            all.set("direction", field!(Axis, direction), &DIRECTIONS);
            all.set("id", field!(Axis, id), &columns);
            all.set("parent", field!(Axis, parent), &columns);
            all.set("child", field!(Axis, child), &columns);
            all.set("domain_id", field!(Axis, domain_id), &columns);
            all.elements("domain_filter", field!(Axis, domain_filter), &comparisons());
            all.elements("domain_view", field!(Axis, domain_view), &columns);
            all.elements("output", field!(Axis, output), &outputs());
        }
        Operation::Closure {
            edges: _,
            keys: _,
            source: _,
            destination: _,
        } => {
            all.elements("keys", field!(Closure, keys), &columns);
            all.set("source", field!(Closure, source), &columns);
            all.set("destination", field!(Closure, destination), &columns);
        }
        Operation::OrderedDfs {
            from: _,
            edges: _,
            root: _,
            source: _,
            destination: _,
            order: _,
            edge: _,
            root_keys: _,
            edge_keys: _,
        } => {
            all.set("root", field!(OrderedDfs, root), &columns);
            all.set("source", field!(OrderedDfs, source), &columns);
            all.set("destination", field!(OrderedDfs, destination), &columns);
            all.set("order", field!(OrderedDfs, order), &columns);
            all.set("edge", field!(OrderedDfs, edge), &columns);
            all.elements("root_keys", field!(OrderedDfs, root_keys), &columns);
            all.elements("edge_keys", field!(OrderedDfs, edge_keys), &columns);
        }
        Operation::Worklist {
            from: _,
            edges: _,
            lattice: _,
            root: _,
            state: _,
            source: _,
            destination: _,
            order: _,
            before: _,
            after: _,
            lower: _,
            upper: _,
            root_keys: _,
            edge_keys: _,
        } => {
            all.set("root", field!(Worklist, root), &columns);
            all.set("state", field!(Worklist, state), &columns);
            all.set("source", field!(Worklist, source), &columns);
            all.set("destination", field!(Worklist, destination), &columns);
            all.set("order", field!(Worklist, order), &columns);
            all.set("before", field!(Worklist, before), &columns);
            all.set("after", field!(Worklist, after), &columns);
            all.set("lower", field!(Worklist, lower), &columns);
            all.set("upper", field!(Worklist, upper), &columns);
            all.elements("root_keys", field!(Worklist, root_keys), &columns);
            all.elements("edge_keys", field!(Worklist, edge_keys), &columns);
        }
        Operation::Cycles {
            edges: _,
            source: _,
            destination: _,
        } => {
            all.set("source", field!(Cycles, source), &columns);
            all.set("destination", field!(Cycles, destination), &columns);
        }
        Operation::Rows {
            parameter: _,
            keys: _,
        } => {
            all.set("parameter", field!(Rows, parameter), &[ParameterId(99)]);
            all.elements("keys", field!(Rows, keys), &[TableKey("missing".into())]);
        }
        Operation::Hosted { relation: _ } => {
            all.set("relation", field!(Hosted, relation), &[HostedNumber(99)]);
        }
        Operation::SpanBetween {
            from: _,
            start: _,
            end: _,
        } => {
            all.set("start", field!(SpanBetween, start), &columns);
            all.set("end", field!(SpanBetween, end), &columns);
        }
        Operation::SourceMatches {
            from: _,
            span: _,
            base_line: _,
            pattern: _,
            capture: _,
            capture_text: _,
            capture_line: _,
        } => {
            all.set("span", field!(SourceMatches, span), &columns);
            all.set(
                "base_line",
                field!(SourceMatches, base_line),
                &[Some(BAD), None],
            );
            all.set("pattern", field!(SourceMatches, pattern), &patterns());
            all.set(
                "capture",
                field!(SourceMatches, capture),
                &[Some(LiteralText("missing".into())), None],
            );
            all.set(
                "capture_text",
                field!(SourceMatches, capture_text),
                &[TextOutput::Appended, TextOutput::Omitted],
            );
            all.set(
                "capture_line",
                field!(SourceMatches, capture_line),
                &[LineOutput::Appended, LineOutput::Omitted],
            );
        }
        Operation::RenderSyntax {
            from: _,
            node: _,
            context: _,
        } => {
            all.set("node", field!(RenderSyntax, node), &columns);
            all.set(
                "context",
                field!(RenderSyntax, context),
                &[
                    RenderContext::AssignmentTarget,
                    RenderContext::AttributeReceiver,
                    RenderContext::PositionalArgument,
                ],
            );
        }
        Operation::FitsRendered {
            from: _,
            texts: _,
            literals: _,
            span: _,
            max_line: _,
            tab_width: _,
        } => {
            all.elements("texts", field!(FitsRendered, texts), &columns);
            all.elements(
                "literals",
                field!(FitsRendered, literals),
                &[LiteralText("".into())],
            );
            all.set("span", field!(FitsRendered, span), &columns);
            all.set(
                "max_line",
                field!(FitsRendered, max_line),
                &[MaxLine(0), MaxLine(-1)],
            );
            all.set("tab_width", field!(FitsRendered, tab_width), &[TabWidth(0)]);
        }
        Operation::SpanBounds { from: _, span: _ } => {
            all.set("span", field!(SpanBounds, span), &columns)
        }
    }
    all.bindings();
    all.out
}
