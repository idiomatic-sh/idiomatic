//! Links parsed query modules as syntax trees, assigning step identities without rewriting names.

use std::collections::{BTreeMap, BTreeSet};

use ryo_values::domain::program::types::{
    Argument, ArgumentValue, Call, Condition, Expression, Filter, FragmentContract,
    FragmentContractInput, FragmentProgram, ImportDeclaration, LetStep, Program, QueryBundle,
    QueryOrigin, QuerySourceId, QuerySpan, QueryTraceSite, Select, SourceReference, StepIdentity,
    StepName, WrittenApplied, WrittenOperand,
};

use super::errors::LinkError;
use super::types::{Bindings, Imported};
use crate::domain::syntax::service::shape_named;

/// Expands every imported-fragment call into fresh, identity-linked AST definitions.
pub(crate) fn link(
    bundle: &QueryBundle,
    mut root: Program,
    fragments: BTreeMap<QuerySourceId, FragmentProgram>,
) -> Result<Program, LinkError> {
    let root_id = bundle.root.id.clone();
    validate_step_names(&root_id, &root.steps)?;
    let mut imports = BTreeMap::new();
    imports.insert(root_id.clone(), imports_for(&root_id, &root.imports)?);
    for (id, fragment) in &fragments {
        validate_step_names(id, &fragment.steps)?;
        validate_fragment_signature(id, fragment)?;
        imports.insert(id.clone(), imports_for(id, &fragment.imports)?);
    }
    validate_import_targets(bundle, &imports)?;
    reject_cycles(&root_id, &imports, &fragments)?;

    let mut next = root
        .steps
        .iter()
        .filter_map(|step| step.name.1.map(|identity| identity.0))
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let mut linked = Vec::new();
    let mut contracts = Vec::new();
    let root_imports = imports.get(&root_id).cloned().unwrap_or_default();
    for step in root.steps.clone() {
        if let Expression::Call(call) = &step.expression
            && let Some(import) = root_imports.get(call.name.0.as_ref())
        {
            let target = source_id(&root_id, import)?;
            let mut trace = vec![trace_site(&root_id, &target, import.span)];
            trace.push(trace_site(&root_id, &target, call.span));
            let expanded = expand(
                &fragments,
                &imports,
                &target,
                call,
                &step.name,
                &linked,
                &Bindings::new(),
                &Bindings::new(),
                &root.parameters,
                &mut next,
                &mut trace,
                &mut root.origins,
                &mut root.step_origins,
                &mut contracts,
            )?;
            linked.extend(expanded);
        } else {
            let mut step = step;
            if let Expression::Call(call) = &mut step.expression {
                remap_call(
                    call,
                    &BTreeMap::new(),
                    &Bindings::new(),
                    &Bindings::new(),
                    &linked,
                    &root_id,
                    &[],
                )?;
            }
            linked.push(step);
        }
    }
    root.steps = linked;
    root.linked = true;
    root.fragment_contracts = contracts;
    attribute_root(&mut root, &root_id);
    Ok(root)
}

fn validate_step_names(source: &QuerySourceId, steps: &[LetStep]) -> Result<(), LinkError> {
    let mut names = BTreeSet::new();
    for step in steps {
        if !names.insert(step.name.0.as_ref()) {
            return Err(located(
                source,
                step.span,
                Vec::new(),
                format!(
                    "step `{}` is declared more than once in this source",
                    step.name.0
                ),
            ));
        }
    }
    Ok(())
}

fn validate_fragment_signature(
    source: &QuerySourceId,
    fragment: &FragmentProgram,
) -> Result<(), LinkError> {
    let mut names = BTreeSet::new();
    for input in &fragment.inputs {
        if !names.insert(input.name.0.as_ref()) {
            return Err(located(
                source,
                input.span,
                Vec::new(),
                format!(
                    "fragment input `{}` is declared more than once",
                    input.name.0
                ),
            ));
        }
        validate_relation_signature(source, &input.relation)?;
    }
    for parameter in &fragment.parameters {
        if !names.insert(parameter.name.0.as_ref()) {
            return Err(located(
                source,
                parameter.span,
                Vec::new(),
                format!(
                    "fragment parameter `{}` is declared more than once",
                    parameter.name.0
                ),
            ));
        }
        if !matches!(
            parameter.kind.0.as_ref(),
            "string" | "text" | "integer" | "number" | "boolean"
        ) {
            return Err(located(
                source,
                parameter.span,
                Vec::new(),
                format!("unsupported fragment parameter type `{}`", parameter.kind.0),
            ));
        }
    }
    validate_relation_signature(source, &fragment.output)
}

fn validate_relation_signature(
    source: &QuerySourceId,
    signature: &ryo_values::domain::program::types::RelationSignature,
) -> Result<(), LinkError> {
    let mut names = BTreeSet::new();
    for column in &signature.columns {
        if !matches!(
            column.kind.0.as_ref(),
            "node"
                | "symbol"
                | "text"
                | "file"
                | "span"
                | "number"
                | "integer"
                | "flag"
                | "boolean"
        ) {
            return Err(located(
                source,
                column.span,
                Vec::new(),
                format!("unknown relation cell type `{}`", column.kind.0),
            ));
        }
        if !names.insert(column.name.0.as_ref()) {
            return Err(located(
                source,
                column.span,
                Vec::new(),
                format!(
                    "relation column `{}` is declared more than once",
                    column.name.0
                ),
            ));
        }
    }
    Ok(())
}

fn imports_for(
    source: &QuerySourceId,
    imports: &[ImportDeclaration],
) -> Result<Imported, LinkError> {
    let mut indexed = BTreeMap::new();
    for import in imports {
        if indexed
            .insert(import.alias.0.clone(), import.clone())
            .is_some()
        {
            return Err(located(
                source,
                import.span,
                Vec::new(),
                "an import alias is declared more than once",
            ));
        }
    }
    Ok(indexed)
}

fn validate_import_targets(
    bundle: &QueryBundle,
    imports: &BTreeMap<QuerySourceId, Imported>,
) -> Result<(), LinkError> {
    for (source, aliases) in imports {
        for import in aliases.values() {
            let target = source_id(source, import)?;
            if !bundle.modules.contains_key(&target) {
                return Err(located(
                    source,
                    import.span,
                    vec![trace_site(source, &target, import.span)],
                    format!("query module `{}` was not supplied in the bundle", target.0),
                ));
            }
        }
    }
    Ok(())
}

fn reject_cycles(
    root: &QuerySourceId,
    imports: &BTreeMap<QuerySourceId, Imported>,
    fragments: &BTreeMap<QuerySourceId, FragmentProgram>,
) -> Result<(), LinkError> {
    fn visit(
        source: &QuerySourceId,
        imports: &BTreeMap<QuerySourceId, Imported>,
        colors: &mut BTreeSet<QuerySourceId>,
        path: &mut Vec<QueryTraceSite>,
    ) -> Result<(), LinkError> {
        colors.insert(source.clone());
        for import in imports.get(source).into_iter().flat_map(BTreeMap::values) {
            let target = source_id(source, import)?;
            let edge = trace_site(source, &target, import.span);
            if colors.contains(&target) {
                let mut trace = path.clone();
                trace.push(edge);
                return Err(located(
                    source,
                    import.span,
                    trace,
                    format!("query module import cycle reaches `{}`", target.0),
                ));
            }
            path.push(edge);
            visit(&target, imports, colors, path)?;
            path.pop();
        }
        colors.remove(source);
        Ok(())
    }

    let mut colors = BTreeSet::new();
    let mut path = Vec::new();
    visit(root, imports, &mut colors, &mut path)?;
    for source in fragments.keys() {
        visit(source, imports, &mut colors, &mut path)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn expand(
    fragments: &BTreeMap<QuerySourceId, FragmentProgram>,
    imports: &BTreeMap<QuerySourceId, Imported>,
    source: &QuerySourceId,
    call: &Call,
    result_name: &StepName,
    preceding: &[LetStep],
    inherited_inputs: &Bindings,
    inherited_parameters: &Bindings,
    root_parameters: &[ryo_values::domain::program::types::ParameterDeclaration],
    next: &mut u64,
    trace: &mut Vec<QueryTraceSite>,
    origins: &mut Vec<QueryOrigin>,
    step_origins: &mut BTreeMap<StepIdentity, QueryOrigin>,
    contracts: &mut Vec<FragmentContract>,
) -> Result<Vec<LetStep>, LinkError> {
    let fragment = fragments.get(source).ok_or_else(|| {
        located(
            source,
            call.span,
            trace.clone(),
            format!("query module `{}` was not parsed", source.0),
        )
    })?;
    let (inputs, parameters) = bind_arguments(
        source,
        fragment,
        call,
        preceding,
        inherited_inputs,
        inherited_parameters,
        root_parameters,
        trace,
    )?;
    let result_identity = fragment.result.step.1.ok_or_else(|| {
        located(
            source,
            fragment.result.span,
            trace.clone(),
            "the returned step is not declared in this fragment",
        )
    })?;
    let mut identities = BTreeMap::new();
    for step in &fragment.steps {
        let identity = step.name.1.ok_or_else(|| {
            located(
                source,
                step.span,
                trace.clone(),
                "a fragment step has no definition identity",
            )
        })?;
        let replacement = if identity == result_identity {
            result_name.1.ok_or_else(|| {
                located(
                    source,
                    call.span,
                    trace.clone(),
                    "the call result has no definition identity",
                )
            })?
        } else {
            let fresh = StepIdentity(*next);
            *next = next.saturating_add(1);
            fresh
        };
        identities.insert(identity, replacement);
    }

    let mut expanded = Vec::new();
    for step in &fragment.steps {
        let mut visible = preceding.to_vec();
        visible.extend(expanded.iter().cloned());
        let mut step = step.clone();
        remap_step(
            &mut step,
            &identities,
            &inputs,
            &parameters,
            source,
            &visible,
            trace,
        )?;
        if step.name.1 == result_name.1 {
            step.name.0 = result_name.0.clone();
        }
        let mut step_trace = trace.clone();
        let step_origin = QueryOrigin {
            source_id: source.clone(),
            span: step.span,
            trace: step_trace.clone(),
        };
        origins.push(step_origin.clone());
        if let Some(identity) = step.name.1 {
            step_origins.insert(identity, step_origin);
        }
        if let Expression::Call(inner) = &step.expression
            && let Some(import) = imports
                .get(source)
                .and_then(|found| found.get(inner.name.0.as_ref()))
        {
            let target = source_id(source, import)?;
            step_trace.push(trace_site(source, &target, import.span));
            step_trace.push(trace_site(source, &target, inner.span));
            expanded.extend(expand(
                fragments,
                imports,
                &target,
                inner,
                &step.name,
                &visible,
                &inputs,
                &parameters,
                root_parameters,
                next,
                &mut step_trace,
                origins,
                step_origins,
                contracts,
            )?);
        } else {
            expanded.push(step);
        }
    }
    let first = expanded
        .first()
        .and_then(|step| step.name.1)
        .ok_or_else(|| {
            located(
                source,
                fragment.output_span,
                trace.clone(),
                "fragment expands to no step definitions",
            )
        })?;
    let result = result_name.1.ok_or_else(|| {
        located(
            source,
            fragment.result.span,
            trace.clone(),
            "fragment result has no linked identity",
        )
    })?;
    let input_contracts = fragment
        .inputs
        .iter()
        .map(|input| {
            let value = inputs.get(&input.name.0).cloned().ok_or_else(|| {
                located(
                    source,
                    input.span,
                    trace.clone(),
                    format!("relation input `{}` was not bound", input.name.0),
                )
            })?;
            Ok(FragmentContractInput {
                name: input.name.clone(),
                value,
                relation: input.relation.clone(),
                span: input.span,
            })
        })
        .collect::<Result<Vec<_>, LinkError>>()?;
    contracts.push(FragmentContract {
        first,
        result,
        inputs: input_contracts,
        output: fragment.output.clone(),
        output_span: fragment.output_span,
        origin: QueryOrigin {
            source_id: source.clone(),
            span: fragment.output_span,
            trace: trace.clone(),
        },
    });
    Ok(expanded)
}

fn bind_arguments(
    source: &QuerySourceId,
    fragment: &FragmentProgram,
    call: &Call,
    preceding: &[LetStep],
    inherited_inputs: &Bindings,
    inherited_parameters: &Bindings,
    root_parameters: &[ryo_values::domain::program::types::ParameterDeclaration],
    trace: &[QueryTraceSite],
) -> Result<(Bindings, Bindings), LinkError> {
    let mut given = BTreeMap::new();
    for argument in &call.arguments {
        if given
            .insert(argument.name.0.clone(), &argument.value)
            .is_some()
        {
            return Err(located(
                source,
                call.span,
                trace.to_vec(),
                format!("argument `{}` is given more than once", argument.name.0),
            ));
        }
    }
    let expected = fragment
        .inputs
        .iter()
        .map(|input| input.name.0.as_ref())
        .chain(
            fragment
                .parameters
                .iter()
                .map(|parameter| parameter.name.0.as_ref()),
        )
        .collect::<BTreeSet<_>>();
    if let Some(unexpected) = given.keys().find(|name| !expected.contains(name.as_ref())) {
        return Err(located(
            source,
            call.span,
            trace.to_vec(),
            format!("fragment has no input or parameter named `{unexpected}`"),
        ));
    }
    let mut inputs = Bindings::new();
    for input in &fragment.inputs {
        let value = given.get(input.name.0.as_ref()).ok_or_else(|| {
            located(
                source,
                call.span,
                trace.to_vec(),
                format!("missing relation input `{}`", input.name.0),
            )
        })?;
        let value = bind_relation_argument(value, inherited_inputs, preceding);
        if !matches!(
            value,
            ArgumentValue::Text(_) | ArgumentValue::Step(_) | ArgumentValue::Word(_)
        ) {
            return Err(located(
                source,
                call.span,
                trace.to_vec(),
                format!(
                    "relation input `{}` must name a relation or step",
                    input.name.0
                ),
            ));
        }
        inputs.insert(input.name.0.clone(), value);
    }
    let mut parameters = Bindings::new();
    for parameter in &fragment.parameters {
        let value = given.get(parameter.name.0.as_ref()).ok_or_else(|| {
            located(
                source,
                call.span,
                trace.to_vec(),
                format!("missing scalar parameter `{}`", parameter.name.0),
            )
        })?;
        let value = substitute_value(value, inherited_parameters);
        if !scalar_matches(&parameter.kind.0, &value, root_parameters) {
            return Err(located(
                source,
                call.span,
                trace.to_vec(),
                format!(
                    "parameter `{}` has a value of the wrong type",
                    parameter.name.0
                ),
            ));
        }
        parameters.insert(parameter.name.0.clone(), value);
    }
    Ok((inputs, parameters))
}

fn bind_relation_argument(
    value: &ArgumentValue,
    inherited: &Bindings,
    preceding: &[LetStep],
) -> ArgumentValue {
    match value {
        ArgumentValue::Word(word) => inherited.get(&word.0).cloned().unwrap_or_else(|| {
            preceding
                .iter()
                .rev()
                .find(|step| step.name.0 == word.0)
                .map_or_else(
                    || value.clone(),
                    |step| ArgumentValue::Step(step.name.clone()),
                )
        }),
        _ => substitute_value(value, inherited),
    }
}

fn scalar_matches(
    kind: &str,
    value: &ArgumentValue,
    root_parameters: &[ryo_values::domain::program::types::ParameterDeclaration],
) -> bool {
    let actual = match value {
        ArgumentValue::Text(_) => "string",
        ArgumentValue::Number(_) => "integer",
        ArgumentValue::Word(word) if matches!(word.0.as_ref(), "true" | "false") => "boolean",
        ArgumentValue::Parameter(parameter) => root_parameters
            .iter()
            .find(|declared| declared.name.0.as_ref() == parameter.0.as_ref())
            .map_or("", |declared| declared.kind.0.as_ref()),
        _ => "",
    };
    matches!(
        (kind, actual),
        ("string" | "text", "string" | "text")
            | ("integer" | "number", "integer" | "number")
            | ("boolean", "boolean")
    )
}

fn remap_step(
    step: &mut LetStep,
    identities: &BTreeMap<StepIdentity, StepIdentity>,
    inputs: &Bindings,
    parameters: &Bindings,
    source: &QuerySourceId,
    preceding: &[LetStep],
    trace: &[QueryTraceSite],
) -> Result<(), LinkError> {
    if let Some(identity) = step.name.1 {
        step.name.1 = identities.get(&identity).copied();
    }
    match &mut step.expression {
        Expression::Select(select) => {
            remap_select(select, identities, inputs, parameters, source, trace)?;
        }
        Expression::Call(call) => remap_call(
            call, identities, inputs, parameters, preceding, source, trace,
        )?,
    }
    Ok(())
}

fn remap_select(
    select: &mut Select,
    identities: &BTreeMap<StepIdentity, StepIdentity>,
    inputs: &Bindings,
    parameters: &Bindings,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkError> {
    for source in std::iter::once(&mut select.from).chain(match &mut select.join {
        ryo_values::domain::program::types::Joining::Alone => None,
        ryo_values::domain::program::types::Joining::Joined(join) => Some(&mut join.source),
    }) {
        let SourceReference::Step(reference) = &source.reference else {
            continue;
        };
        let replacement = if let Some(identity) = reference.1 {
            identities.get(&identity).copied().map(|identity| {
                SourceReference::Step(StepName(reference.0.clone(), Some(identity)))
            })
        } else {
            inputs
                .get(reference.0.as_ref())
                .and_then(|input| match input {
                    ArgumentValue::Text(name) => Some(SourceReference::Relation(
                        ryo_values::domain::program::types::WrittenRelation(name.0.clone()),
                    )),
                    ArgumentValue::Step(step) => Some(SourceReference::Step(step.clone())),
                    ArgumentValue::Word(word) => {
                        Some(SourceReference::Step(StepName(word.0.clone(), None)))
                    }
                    _ => None,
                })
        };
        if let Some(replacement) = replacement {
            source.reference = replacement;
        }
    }
    for condition in &mut select.filters {
        remap_condition(condition, parameters, source, trace)?;
    }
    Ok(())
}

fn remap_condition(
    condition: &mut Condition,
    parameters: &Bindings,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkError> {
    match condition {
        Condition::Test(filter) => remap_filter(filter, parameters, source, trace),
        Condition::Any(alternatives) => {
            for alternative in alternatives {
                for condition in alternative {
                    remap_condition(condition, parameters, source, trace)?;
                }
            }
            Ok(())
        }
    }
}

fn remap_filter(
    filter: &mut Filter,
    parameters: &Bindings,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkError> {
    remap_operand(&mut filter.left, parameters, filter.span, source, trace)?;
    remap_operand(&mut filter.right, parameters, filter.span, source, trace)
}

fn remap_operand(
    operand: &mut WrittenOperand,
    parameters: &Bindings,
    span: QuerySpan,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkError> {
    match operand {
        WrittenOperand::Parameter(parameter) => {
            let value = parameters.get(&parameter.0).ok_or_else(|| {
                located(
                    source,
                    span,
                    trace.to_vec(),
                    format!("fragment parameter `${}` was not bound", parameter.0),
                )
            })?;
            *operand = match value {
                ArgumentValue::Text(value) => WrittenOperand::Text(value.clone()),
                ArgumentValue::Number(value) => WrittenOperand::Number(value.clone()),
                ArgumentValue::Parameter(value) => WrittenOperand::Parameter(value.clone()),
                _ => {
                    return Err(located(
                        source,
                        span,
                        trace.to_vec(),
                        "this filter position requires a text or number parameter",
                    ));
                }
            };
        }
        WrittenOperand::Function(WrittenApplied { arguments, .. }) => {
            for argument in arguments {
                remap_operand(argument, parameters, span, source, trace)?;
            }
        }
        WrittenOperand::Column(_) | WrittenOperand::Text(_) | WrittenOperand::Number(_) => {}
    }
    Ok(())
}

fn remap_call(
    call: &mut Call,
    _identities: &BTreeMap<StepIdentity, StepIdentity>,
    inputs: &Bindings,
    parameters: &Bindings,
    preceding: &[LetStep],
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkError> {
    for argument in &mut call.arguments {
        substitute_argument_value(&mut argument.value, parameters);
    }
    let input_slots = shape_named(&call.name).map_or(&[][..], |shape| shape.inputs);
    for argument in &mut call.arguments {
        let is_input = crate::domain::syntax::service::is_input(input_slots, &argument.name)
            == crate::domain::syntax::types::ArgumentRole::Input;
        if !is_input {
            continue;
        }
        if let ArgumentValue::Word(word) = &argument.value {
            argument.value = inputs.get(&word.0).cloned().unwrap_or_else(|| {
                preceding
                    .iter()
                    .rev()
                    .find(|step| step.name.0 == word.0)
                    .map_or_else(
                        || argument.value.clone(),
                        |step| ArgumentValue::Step(step.name.clone()),
                    )
            });
        }
        if !matches!(
            argument.value,
            ArgumentValue::Text(_) | ArgumentValue::Step(_) | ArgumentValue::Word(_)
        ) {
            return Err(located(
                source,
                call.span,
                trace.to_vec(),
                format!(
                    "relation input `{}` must name a relation or step",
                    argument.name.0
                ),
            ));
        }
    }
    Ok(())
}

fn substitute_value(value: &ArgumentValue, bindings: &Bindings) -> ArgumentValue {
    match value {
        ArgumentValue::Parameter(parameter) => bindings
            .get(&parameter.0)
            .cloned()
            .unwrap_or_else(|| value.clone()),
        ArgumentValue::List(values) => ArgumentValue::List(
            values
                .iter()
                .map(|value| substitute_value(value, bindings))
                .collect(),
        ),
        ArgumentValue::Map(arguments) => ArgumentValue::Map(
            arguments
                .iter()
                .map(|argument| Argument {
                    name: argument.name.clone(),
                    value: substitute_value(&argument.value, bindings),
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn substitute_argument_value(value: &mut ArgumentValue, bindings: &Bindings) {
    *value = substitute_value(value, bindings);
}

fn source_id(
    source: &QuerySourceId,
    import: &ImportDeclaration,
) -> Result<QuerySourceId, LinkError> {
    ryo_values::domain::program::service::source_id(&import.module.0).map_err(|error| {
        located(
            source,
            import.span,
            Vec::new(),
            format!("invalid module id: {error}"),
        )
    })
}

fn trace_site(source: &QuerySourceId, target: &QuerySourceId, span: QuerySpan) -> QueryTraceSite {
    QueryTraceSite {
        source_id: source.clone(),
        target_id: target.clone(),
        span,
    }
}

fn located(
    source_id: &QuerySourceId,
    span: QuerySpan,
    trace: Vec<QueryTraceSite>,
    message: impl Into<String>,
) -> LinkError {
    LinkError {
        origin: QueryOrigin {
            source_id: source_id.clone(),
            span,
            trace,
        },
        message: message.into(),
    }
}

fn attribute_root(program: &mut Program, root: &QuerySourceId) {
    for import in &program.imports {
        program.origins.push(QueryOrigin {
            source_id: root.clone(),
            span: import.span,
            trace: Vec::new(),
        });
    }
    for step in &program.steps {
        if let Some(identity) = step.name.1 {
            program
                .step_origins
                .entry(identity)
                .or_insert_with(|| QueryOrigin {
                    source_id: root.clone(),
                    span: step.span,
                    trace: Vec::new(),
                });
        }
        program.origins.push(QueryOrigin {
            source_id: root.clone(),
            span: step.span,
            trace: Vec::new(),
        });
        if let Expression::Call(call) = &step.expression {
            program.origins.push(QueryOrigin {
                source_id: root.clone(),
                span: call.span,
                trace: Vec::new(),
            });
        }
    }
    program.origins.push(QueryOrigin {
        source_id: root.clone(),
        span: program.emit.span,
        trace: Vec::new(),
    });
    program.emit_origin = Some(QueryOrigin {
        source_id: root.clone(),
        span: program.emit.span,
        trace: Vec::new(),
    });
}
