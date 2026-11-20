//! Spike (ryo-fmx3, bead ryo-r6ux): the plans of a request as JSON for the engine bake-off: every rule's plan after the front
//! end and the merged logical program the engine runs, written from the plan types themselves (an exhaustive match per type,
//! never their `Debug` text). Throwaway export code; no engine behaviour reads it.

use crate::domain::executable_program::types::logical_build::value::LogicalBuild;
use ryo_operators::domain::physical_plan::types::PlanInput;
use ryo_relations::domain::relations::service::name;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Direction, Function, Input, JoinColumn, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::query_bindings::types::{ParameterBinding, ParameterType};
use std::fmt::Write as _;

/// JSON text of `text`, quoted and escaped.
fn text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ignored = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn list<T>(items: &[T], each: impl Fn(&T) -> String) -> String {
    let parts: Vec<String> = items.iter().map(each).collect();
    format!("[{}]", parts.join(","))
}

fn columns(items: &[ColumnIndex]) -> String {
    list(items, |c| c.0.to_string())
}

fn relation(read: Relation) -> String {
    text(name(read).0)
}

/// The kind of a cell, as the README names it.
#[must_use]
pub fn kind(cell: CellKind) -> &'static str {
    match cell {
        CellKind::Node => "node",
        CellKind::Symbol => "symbol",
        CellKind::File => "file",
        CellKind::Span => "span",
        CellKind::Number => "number",
        CellKind::Flag => "flag",
    }
}

fn scope(at: StepScope) -> &'static str {
    match at {
        StepScope::PerFile => "per-file",
        StepScope::CrossFile => "cross-file",
    }
}

fn input(read: &Input) -> String {
    match read {
        Input::Relation(r) => format!("{{\"relation\":{}}}", relation(*r)),
        Input::Step(step) => format!("{{\"step\":{}}}", step.0),
        Input::BoundObservation(id) => format!("{{\"observation\":{}}}", id.0),
    }
}

fn plan_input(read: &PlanInput) -> String {
    match read {
        PlanInput::Node(node) => format!("{{\"node\":{}}}", node.0),
        PlanInput::Relation(r) => format!("{{\"relation\":{}}}", relation(*r)),
        PlanInput::Frontier(place) => format!(
            "{{\"frontier\":{{\"rule\":{},\"input\":{}}}}}",
            place.rule.0, place.input.0
        ),
        PlanInput::Observation(place) => format!(
            "{{\"observation\":{{\"rule\":{},\"input\":{}}}}}",
            place.rule.0, place.input.0
        ),
    }
}

fn test(compared: Test) -> &'static str {
    match compared {
        Test::Equal => "=",
        Test::NotEqual => "!=",
        Test::LessThan => "<",
        Test::LessThanOrEqual => "<=",
        Test::GreaterThan => ">",
        Test::GreaterThanOrEqual => ">=",
        Test::FloatTextGreaterThanOrEqual => "float-text>=",
        Test::In => "in",
        Test::NotIn => "not-in",
        Test::GlobAny => "glob-any",
        Test::NotGlobAny => "not-glob-any",
        Test::StartsWith => "starts-with",
        Test::EndsWith => "ends-with",
        Test::Matches => "matches",
        Test::NumericGreaterThanOrEqual => "numeric>=",
        Test::IntegerSuccessor => "integer-successor",
        Test::NaturalLessThan => "natural<",
    }
}

fn function(applied: Function) -> &'static str {
    match applied {
        Function::Concat => "concat",
        Function::Pascal => "pascal",
        Function::Camel => "camel",
        Function::Snake => "snake",
        Function::Kebab => "kebab",
        Function::Lower => "lower",
    }
}

fn operand(value: &Operand) -> String {
    match value {
        Operand::Column(c) => format!("{{\"column\":{}}}", c.0),
        Operand::Parameter(p) => format!("{{\"parameter\":{}}}", p.0),
        Operand::Literal(QueryLiteral::Number(n)) => format!("{{\"number\":{}}}", n.0),
        Operand::Literal(QueryLiteral::Text(t)) => format!("{{\"text\":{}}}", text(&t.0)),
        Operand::Function(applied) => format!(
            "{{\"function\":\"{}\",\"arguments\":{}}}",
            function(applied.function),
            list(&applied.arguments, operand)
        ),
    }
}

fn comparisons(items: &[PlanComparison]) -> String {
    list(items, |c| {
        let origin = match c.origin {
            ComparisonOrigin::Recorded(id) => id.0.to_string(),
            ComparisonOrigin::Unrecorded => "null".to_owned(),
        };
        format!(
            "{{\"test\":\"{}\",\"left\":{},\"right\":{},\"origin\":{origin}}}",
            test(c.test),
            operand(&c.left),
            operand(&c.right)
        )
    })
}

fn join_columns(items: &[JoinColumn]) -> String {
    list(items, |c| {
        let side = match c.side {
            Side::Left => "left",
            Side::Right => "right",
        };
        format!("{{\"side\":\"{side}\",\"column\":{}}}", c.column.0)
    })
}

fn direction(walk: Direction) -> &'static str {
    match walk {
        Direction::Parent => "parent",
        Direction::Child => "child",
        Direction::Ancestor => "ancestor",
        Direction::Descendant => "descendant",
        Direction::NearestAncestor => "nearest-ancestor",
        Direction::Deep => "deep",
        Direction::PreviousSibling => "previous-sibling",
        Direction::NextSibling => "next-sibling",
    }
}

/// The operation's name and every field, as JSON object members (no braces): `"op":..,"field":..`.
#[must_use]
pub fn operation(op: &Operation) -> String {
    match op {
        Operation::Select { from, comparisons: c, kept } => format!(
            "\"op\":\"select\",\"from\":{},\"comparisons\":{},\"kept\":{}",
            input(from),
            comparisons(c),
            columns(kept)
        ),
        Operation::Project { from, columns: c, distinct } => format!(
            "\"op\":\"project\",\"from\":{},\"columns\":{},\"distinct\":{}",
            input(from),
            columns(c),
            matches!(distinct, RepeatedRows::Dropped)
        ),
        Operation::Extend { from, values } => format!(
            "\"op\":\"extend\",\"from\":{},\"values\":{}",
            input(from),
            list(values, operand)
        ),
        Operation::Join { left, right, left_on, right_on, output, filter, view, reach } => format!(
            "\"op\":\"join\",\"left\":{},\"right\":{},\"left_on\":{},\"right_on\":{},\"output\":{},\"filter\":{},\"view\":{},\"reach\":{}",
            input(left),
            input(right),
            columns(left_on),
            columns(right_on),
            join_columns(output),
            comparisons(filter),
            columns(view),
            reach.as_ref().map_or_else(
                || "null".to_owned(),
                |r| format!(
                    "{{\"source\":{},\"destination\":{},\"keys\":{},\"columns\":{}}}",
                    r.source.0,
                    r.destination.0,
                    columns(&r.keys),
                    columns(&r.columns)
                )
            )
        ),
        Operation::Union { left, right } => {
            format!("\"op\":\"union\",\"left\":{},\"right\":{}", input(left), input(right))
        }
        Operation::Intersection { left, right } => format!(
            "\"op\":\"intersection\",\"left\":{},\"right\":{}",
            input(left),
            input(right)
        ),
        Operation::Difference { left, right } => format!(
            "\"op\":\"difference\",\"left\":{},\"right\":{}",
            input(left),
            input(right)
        ),
        Operation::Antijoin { left, right, left_on, right_on } => format!(
            "\"op\":\"antijoin\",\"left\":{},\"right\":{},\"left_on\":{},\"right_on\":{}",
            input(left),
            input(right),
            columns(left_on),
            columns(right_on)
        ),
        Operation::GroupCount { from, keys, distinct } => format!(
            "\"op\":\"group_count\",\"from\":{},\"keys\":{},\"distinct\":{}",
            input(from),
            columns(keys),
            distinct.map_or_else(|| "null".to_owned(), |c| c.0.to_string())
        ),
        Operation::GroupDigest { from, keys, over } => format!(
            "\"op\":\"group_digest\",\"from\":{},\"keys\":{},\"over\":{}",
            input(from),
            columns(keys),
            over.0
        ),
        Operation::Capture { from, path, pattern } => format!(
            "\"op\":\"capture\",\"from\":{},\"path\":{},\"pattern\":{}",
            input(from),
            path.0,
            text(&pattern.0)
        ),
        Operation::Rank { from, column, sets } => format!(
            "\"op\":\"rank\",\"from\":{},\"column\":{},\"sets\":{}",
            input(from),
            column.0,
            sets.0
        ),
        Operation::Replace { from, text: t, pattern, replacement } => format!(
            "\"op\":\"replace\",\"from\":{},\"text\":{},\"pattern\":{},\"replacement\":{}",
            input(from),
            t.0,
            text(&pattern.0),
            text(&replacement.0)
        ),
        Operation::OrderedMatch { left, right, left_on, right_on, left_order, right_order, output } => format!(
            "\"op\":\"ordered_match\",\"left\":{},\"right\":{},\"left_on\":{},\"right_on\":{},\"left_order\":{},\"right_order\":{},\"output\":{}",
            input(left),
            input(right),
            columns(left_on),
            columns(right_on),
            left_order.0,
            right_order.0,
            join_columns(output)
        ),
        Operation::Words { from, text: t } => {
            format!("\"op\":\"words\",\"from\":{},\"text\":{}", input(from), t.0)
        }
        Operation::Axis { from, edges, domain, direction: d, id, parent, child, domain_id, domain_filter, domain_view, output } => format!(
            "\"op\":\"axis\",\"from\":{},\"edges\":{},\"domain\":{},\"direction\":\"{}\",\"id\":{},\"parent\":{},\"child\":{},\"domain_id\":{},\"domain_filter\":{},\"domain_view\":{},\"output\":{}",
            input(from),
            input(edges),
            input(domain),
            direction(*d),
            id.0,
            parent.0,
            child.0,
            domain_id.0,
            comparisons(domain_filter),
            columns(domain_view),
            join_columns(output)
        ),
        Operation::Closure { edges, keys, source, destination } => format!(
            "\"op\":\"closure\",\"edges\":{},\"keys\":{},\"source\":{},\"destination\":{}",
            input(edges),
            columns(keys),
            source.0,
            destination.0
        ),
        Operation::OrderedDfs { from, edges, root, source, destination, order, edge, root_keys, edge_keys } => format!(
            "\"op\":\"ordered_dfs\",\"from\":{},\"edges\":{},\"root\":{},\"source\":{},\"destination\":{},\"order\":{},\"edge\":{},\"root_keys\":{},\"edge_keys\":{}",
            input(from),
            input(edges),
            root.0,
            source.0,
            destination.0,
            order.0,
            edge.0,
            columns(root_keys),
            columns(edge_keys)
        ),
        Operation::Worklist { from, edges, lattice, root, state, source, destination, order, before, after, lower, upper, root_keys, edge_keys } => format!(
            "\"op\":\"worklist\",\"from\":{},\"edges\":{},\"lattice\":{},\"root\":{},\"state\":{},\"source\":{},\"destination\":{},\"order\":{},\"before\":{},\"after\":{},\"lower\":{},\"upper\":{},\"root_keys\":{},\"edge_keys\":{}",
            input(from),
            input(edges),
            input(lattice),
            root.0,
            state.0,
            source.0,
            destination.0,
            order.0,
            before.0,
            after.0,
            lower.0,
            upper.0,
            columns(root_keys),
            columns(edge_keys)
        ),
        Operation::Cycles { edges, source, destination } => format!(
            "\"op\":\"cycles\",\"edges\":{},\"source\":{},\"destination\":{}",
            input(edges),
            source.0,
            destination.0
        ),
        Operation::Rows { parameter, keys } => format!(
            "\"op\":\"rows\",\"parameter\":{},\"keys\":{}",
            parameter.0,
            list(keys, |k| text(&k.0))
        ),
        Operation::Hosted { relation: r } => format!("\"op\":\"hosted\",\"relation\":{}", r.0),
        Operation::SpanBetween { from, start, end } => format!(
            "\"op\":\"span_between\",\"from\":{},\"start\":{},\"end\":{}",
            input(from),
            start.0,
            end.0
        ),
        Operation::SourceMatches { from, span, base_line, pattern, capture, capture_text, capture_line } => format!(
            "\"op\":\"source_matches\",\"from\":{},\"span\":{},\"base_line\":{},\"pattern\":{},\"capture\":{},\"capture_text\":{},\"capture_line\":{}",
            input(from),
            span.0,
            base_line.map_or_else(|| "null".to_owned(), |c| c.0.to_string()),
            text(&pattern.0),
            capture.as_ref().map_or_else(|| "null".to_owned(), |c| text(&c.0)),
            matches!(capture_text, ryo_values::domain::query_operators::types::TextOutput::Appended),
            matches!(capture_line, ryo_values::domain::query_operators::types::LineOutput::Appended)
        ),
        Operation::RenderSyntax { from, node, context } => format!(
            "\"op\":\"render_syntax\",\"from\":{},\"node\":{},\"context\":\"{}\"",
            input(from),
            node.0,
            match context {
                ryo_language::domain::semantics::types::RenderContext::AssignmentTarget => "assignment-target",
                ryo_language::domain::semantics::types::RenderContext::AttributeReceiver => "attribute-receiver",
                ryo_language::domain::semantics::types::RenderContext::PositionalArgument => "positional-argument",
            }
        ),
        Operation::FitsRendered { from, texts, literals, span, max_line, tab_width } => format!(
            "\"op\":\"fits_rendered\",\"from\":{},\"texts\":{},\"literals\":{},\"span\":{},\"max_line\":{},\"tab_width\":{}",
            input(from),
            columns(texts),
            list(literals, |l| text(&l.0)),
            span.0,
            max_line.0,
            tab_width.0
        ),
        Operation::SpanBounds { from, span } => {
            format!("\"op\":\"span_bounds\",\"from\":{},\"span\":{}", input(from), span.0)
        }
    }
}

fn binding(bound: &ParameterBinding) -> String {
    match bound {
        ParameterBinding::Number(n) => format!("{{\"number\":{}}}", n.0),
        ParameterBinding::Flag(f) => format!(
            "{{\"flag\":{}}}",
            matches!(f, ryo_values::domain::plan_parts::types::FlagState::Raised)
        ),
        ParameterBinding::Text(t) => format!("{{\"text\":{}}}", text(&t.0)),
        ParameterBinding::Numbers(ns) => format!("{{\"numbers\":{}}}", list(ns, |n| n.0.to_string())),
        ParameterBinding::Texts(ts) => format!("{{\"texts\":{}}}", list(ts, |t| text(&t.0))),
        ParameterBinding::TextSets(sets) => format!(
            "{{\"text_sets\":{}}}",
            list(sets, |set| list(set, |t| text(&t.0)))
        ),
        ParameterBinding::Tables(tables) => format!(
            "{{\"tables\":{}}}",
            list(tables, |table| {
                let cells: Vec<String> = table
                    .0
                    .iter()
                    .map(|cell| format!("{}:{}", text(&cell.key.0), text(&cell.value.0)))
                    .collect();
                format!("{{{}}}", cells.join(","))
            })
        ),
    }
}

fn parameter_type(kind: ParameterType) -> &'static str {
    match kind {
        ParameterType::Flag => "flag",
        ParameterType::Number => "number",
        ParameterType::Text => "text",
        ParameterType::Numbers => "numbers",
        ParameterType::Texts => "texts",
        ParameterType::TextSets => "text_sets",
        ParameterType::Tables => "tables",
    }
}

/// One rule's plan after the front end, as JSON.
#[must_use]
pub fn plan(rule_plan: &Plan, bindings: &[ParameterBinding]) -> String {
    let mut out = String::from("{\"parameters\":[");
    for (at, parameter) in rule_plan.parameters.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let _ignored = write!(
            out,
            "{{\"id\":{at},\"name\":{},\"kind\":\"{}\",\"bound\":{}}}",
            text(&parameter.name.0),
            parameter_type(parameter.kind),
            bindings.get(at).map_or_else(|| "null".to_owned(), binding)
        );
    }
    out.push_str("],\"steps\":[");
    for (at, step) in rule_plan.steps.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let _ignored = write!(
            out,
            "{{\"step\":{at},\"scope\":\"{}\",{}}}",
            scope(step.scope),
            operation(&step.operation)
        );
    }
    let _ignored = write!(
        out,
        "],\"emit\":{{\"step\":{},\"subject\":{},\"key\":{},\"headings\":{}}},\"frontier\":{},\"observations\":{},\"observation_inputs\":{}}}",
        rule_plan.emit.step.0,
        rule_plan.emit.subject.0,
        columns(&rule_plan.emit.key),
        list(&rule_plan.emit.headings, |h| text(&h.0)),
        list(&rule_plan.frontier, input),
        list(&rule_plan.observations, |o| format!(
            "{{\"name\":{},\"source\":{},\"scope\":\"{}\",\"columns\":{}}}",
            text(&o.name.0),
            input(&o.source),
            scope(o.scope),
            list(&o.columns, |c| format!("{{\"name\":{},\"kind\":\"{}\"}}", text(&c.name.0), kind(c.kind)))
        )),
        list(&rule_plan.observation_inputs, |o| format!(
            "{{\"scope\":\"{}\",\"columns\":{}}}",
            scope(o.scope),
            list(&o.columns, |c| format!("{{\"name\":{},\"kind\":\"{}\"}}", text(&c.name.0), kind(c.kind)))
        ))
    );
    out
}

/// The whole export: every rule's plan with the node of each of its steps, then the merged program with each node's owners.
#[must_use]
pub fn program(build: &LogicalBuild<'_>, plans: &[std::sync::Arc<Plan>], bindings: &[Vec<ParameterBinding>], ids: &[&str]) -> String {
    let nodes = build.program.nodes();
    let mut owners: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (rule, rule_nodes) in build.rules.iter().enumerate() {
        for node in rule_nodes.steps.iter().flatten() {
            if let Some(list) = owners.get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
                && !list.contains(&rule)
            {
                list.push(rule);
            }
        }
    }
    let mut out = String::from("{\"rules\":[\n");
    for (rule, plan_of) in plans.iter().enumerate() {
        if rule > 0 {
            out.push_str(",\n");
        }
        let rule_nodes = build.rules.get(rule);
        let steps = rule_nodes.map_or_else(
            || "[]".to_owned(),
            |r| list(&r.steps, |s| s.as_ref().map_or_else(|_| "null".to_owned(), |n| n.0.to_string())),
        );
        let emit = rule_nodes
            .and_then(|r| r.emit.as_ref().ok())
            .map_or_else(|| "null".to_owned(), |n| n.0.to_string());
        let _ignored = write!(
            out,
            "{{\"ordinal\":{rule},\"id\":{},\"plan\":{},\"step_nodes\":{steps},\"emit_node\":{emit}}}",
            text(ids.get(rule).copied().unwrap_or("")),
            plan(plan_of, bindings.get(rule).map_or(&[], Vec::as_slice))
        );
    }
    out.push_str("\n],\n\"program\":{\"nodes\":[\n");
    for (at, node) in nodes.iter().enumerate() {
        if at > 0 {
            out.push_str(",\n");
        }
        let layout = build.layouts.get(at).and_then(|l| l.as_ref().ok()).map_or_else(
            || "null".to_owned(),
            |kinds| list(kinds, |k| format!("\"{}\"", kind(*k))),
        );
        let _ignored = write!(
            out,
            "{{\"id\":{at},\"scope\":\"{}\",\"inputs\":{},{},\"bindings\":{},\"columns\":{layout},\"owners\":{}}}",
            scope(node.scope),
            list(node.inputs.0, plan_input),
            operation(node.operation),
            list(node.bindings.0, binding),
            list(owners.get(at).map_or(&[][..], Vec::as_slice), ToString::to_string)
        );
    }
    out.push_str("\n],\n\"emits\":[");
    let mut first = true;
    for (rule, rule_nodes) in build.rules.iter().enumerate() {
        if let Ok(node) = &rule_nodes.emit {
            if !first {
                out.push(',');
            }
            first = false;
            let _ignored = write!(out, "{{\"rule\":{rule},\"node\":{}}}", node.0);
        }
    }
    out.push_str("]}}\n");
    out
}
