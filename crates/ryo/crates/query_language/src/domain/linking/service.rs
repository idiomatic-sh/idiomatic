//! Links parsed query modules as syntax trees, assigning step identities without rewriting names.

use std::collections::{BTreeMap, BTreeSet};

use ryo_values::domain::program::types::{
    Argument, ArgumentValue, Call, Condition, Expression, Filter, FragmentContract,
    FragmentContractInput, FragmentProgram, ImportDeclaration, LetStep, QueryBundle, QueryOrigin,
    QuerySourceId, QuerySpan, QueryTraceSite, Select, SourceReference, StepIdentity, StepName,
    WrittenApplied, WrittenOperand, WrittenProgram,
};

use super::errors::LinkingError;
use super::types::{
    Bindings, BoundArguments, FragmentBindingKey, ImportAlias, ImportIndex, ParameterCheck,
    ScalarMatch,
};
use crate::domain::syntax::constants::{
    BOOLEAN_WORDS, fragment_scalar_kind_named, relation_cell_kind_named,
};
use crate::domain::syntax::service::shape_named;
use ryo_values::domain::plans::types::ParameterType;

/// Expands every imported-fragment call into fresh, identity-linked AST definitions.
#[cfg(test)]
pub(crate) fn link(
    bundle: &QueryBundle,
    root: WrittenProgram,
    fragments: &BTreeMap<QuerySourceId, FragmentProgram>,
) -> Result<WrittenProgram, LinkingError> {
    link_with_observations(bundle, root, fragments, &[])
}

/// Bind declared external input names before expanding calls; no local definition is synthesized.
pub(crate) fn link_with_observations(
    bundle: &QueryBundle,
    mut root: WrittenProgram,
    fragments: &BTreeMap<QuerySourceId, FragmentProgram>,
    observations: &[ryo_values::domain::observations::types::BoundObservationInput],
) -> Result<WrittenProgram, LinkingError> {
    let root_id = bundle.root.id.clone();
    assign_root_identities(&mut root);
    validate_step_names(&root_id, &root.steps)?;
    let mut imports = BTreeMap::new();
    let previous_root_imports =
        imports.insert(root_id.clone(), imports_for(&root_id, &root.imports)?);
    debug_assert!(
        previous_root_imports.is_none(),
        "a root source ID is inserted exactly once"
    );
    for (id, fragment) in fragments {
        validate_step_names(id, &fragment.steps)?;
        validate_fragment_signature(id, fragment)?;
        let previous_fragment_imports =
            imports.insert(id.clone(), imports_for(id, &fragment.imports)?);
        debug_assert!(
            previous_fragment_imports.is_none(),
            "each fragment source ID is inserted exactly once"
        );
    }
    validate_import_targets(bundle, &imports)?;
    reject_cycles(&root_id, &imports, fragments)?;

    let mut next = StepIdentity(
        root.steps
            .iter()
            .filter_map(|step| {
                ryo_values::domain::program::service::identity_of(step.name.1)
                    .map(|identity| identity.0)
            })
            .max()
            .unwrap_or(0)
            .saturating_add(1),
    );
    let mut linked = Vec::new();
    let mut root_visible = Vec::new();
    let mut contracts = Vec::new();
    let root_imports = imports.get(&root_id).cloned().unwrap_or_default();
    bind_declared_sources(&mut root, observations, &root_imports, fragments, &root_id)?;
    for original in root.steps.clone() {
        if let Expression::Call(call) = &original.expression
            && let Some(import) = root_imports.0.get(&*call.name.0)
        {
            let target = source_id(&root_id, import)?;
            let mut trace = vec![trace_site(&root_id, &target, import.span)];
            trace.push(trace_site(&root_id, &target, call.span));
            let expanded = expand(
                fragments,
                &imports,
                &target,
                call,
                &original.name,
                &root_visible,
                &Bindings::default(),
                &Bindings::default(),
                &root.parameters,
                &mut next,
                &trace,
                &mut root.origins,
                &mut root.step_origins,
                &mut contracts,
            )?;
            linked.extend(expanded);
        } else {
            let mut remapped_step = original.clone();
            if let Expression::Call(call) = &mut remapped_step.expression {
                remap_call(
                    call,
                    &Bindings::default(),
                    &Bindings::default(),
                    &root_visible,
                    &root_id,
                    &[],
                    ParameterCheck::Lenient,
                )?;
            }
            linked.push(remapped_step);
        }
        root_visible.push(original);
    }
    root.linked = ryo_values::domain::program::types::Linking::Linked;
    root.steps = linked;
    root.fragment_contracts = contracts;
    attribute_root(&mut root, &root_id);
    Ok(root)
}

/// Completes source-local identities for syntax ports that return named, unlinked programs.
fn assign_root_identities(program: &mut WrittenProgram) {
    let mut next = program
        .steps
        .iter()
        .filter_map(|step| {
            ryo_values::domain::program::service::identity_of(step.name.1)
                .map(|identity| identity.0)
        })
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    for step in &mut program.steps {
        if ryo_values::domain::program::service::identity_of(step.name.1).is_none() {
            step.name.1 =
                ryo_values::domain::program::service::identified(Some(StepIdentity(next)));
            next = next.saturating_add(1);
        }
    }
    for place in 0..program.steps.len() {
        let (preceding, current) = program.steps.split_at_mut(place);
        if let Some(step) = current.first_mut()
            && let Expression::Select(select) = &mut step.expression
        {
            let sources = std::iter::once(&mut select.from).chain(match &mut select.join {
                ryo_values::domain::program::types::Joining::Alone => None,
                ryo_values::domain::program::types::Joining::Joined(join) => Some(&mut join.source),
            });
            for source in sources {
                if let SourceReference::Step(reference) = &mut source.reference
                    && ryo_values::domain::program::service::identity_of(reference.1).is_none()
                {
                    reference.1 = ryo_values::domain::program::service::identified(
                        preceding
                            .iter()
                            .rev()
                            .find(|candidate| candidate.name.0 == reference.0)
                            .and_then(|candidate| {
                                ryo_values::domain::program::service::identity_of(candidate.name.1)
                            }),
                    );
                }
            }
        }
    }
    for output in &mut program.observations {
        if ryo_values::domain::program::service::identity_of(output.step.1).is_none() {
            output.step.1 = ryo_values::domain::program::service::identified(
                program
                    .steps
                    .iter()
                    .rev()
                    .find(|step| step.name.0 == output.step.0)
                    .and_then(|step| {
                        ryo_values::domain::program::service::identity_of(step.name.1)
                    }),
            );
        }
    }
    if ryo_values::domain::program::service::identity_of(program.emit.step.1).is_none() {
        program.emit.step.1 = ryo_values::domain::program::service::identified(
            program
                .steps
                .iter()
                .rev()
                .find(|step| step.name.0 == program.emit.step.0)
                .and_then(|step| ryo_values::domain::program::service::identity_of(step.name.1)),
        );
    }
}

/// Rejects repeated step names within one source module.
fn validate_step_names(source: &QuerySourceId, steps: &[LetStep]) -> Result<(), LinkingError> {
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

/// Checks a fragment's input, parameter, and output signature for valid unique names.
fn validate_fragment_signature(
    source: &QuerySourceId,
    fragment: &FragmentProgram,
) -> Result<(), LinkingError> {
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
        if fragment_scalar_kind_named(&parameter.kind.0).is_none() {
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

/// Checks that relation signature columns use supported kinds and unique names.
fn validate_relation_signature(
    source: &QuerySourceId,
    signature: &ryo_values::domain::program::types::RelationSignature,
) -> Result<(), LinkingError> {
    let mut names = BTreeSet::new();
    for column in &signature.columns {
        if relation_cell_kind_named(&column.kind.0).is_none() {
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

/// Indexes module imports by local alias and rejects duplicate aliases.
fn imports_for(
    source: &QuerySourceId,
    imports: &[ImportDeclaration],
) -> Result<ImportIndex, LinkingError> {
    let mut indexed = ImportIndex::default();
    for import in imports {
        if indexed
            .0
            .insert(ImportAlias(import.alias.0.clone()), import.clone())
            .is_some()
        {
            return Err(located(
                source,
                import.span,
                Vec::new(),
                super::constants::DUPLICATE_IMPORT_ALIAS,
            ));
        }
    }
    Ok(indexed)
}

/// Rejects imports whose target module is absent from the immutable bundle.
fn validate_import_targets(
    bundle: &QueryBundle,
    imports: &BTreeMap<QuerySourceId, ImportIndex>,
) -> Result<(), LinkingError> {
    for (source, aliases) in imports {
        for import in aliases.0.values() {
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

/// Rejects cyclic imports before expansion begins.
fn reject_cycles(
    root: &QuerySourceId,
    imports: &BTreeMap<QuerySourceId, ImportIndex>,
    fragments: &BTreeMap<QuerySourceId, FragmentProgram>,
) -> Result<(), LinkingError> {
    fn visit(
        source: &QuerySourceId,
        imports: &BTreeMap<QuerySourceId, ImportIndex>,
        colors: &mut BTreeSet<QuerySourceId>,
        path: &mut Vec<QueryTraceSite>,
    ) -> Result<(), LinkingError> {
        let _new_color = colors.insert(source.clone());
        for import in imports
            .get(source)
            .into_iter()
            .flat_map(|index| index.0.values())
        {
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
            let _edge = path.pop();
        }
        let _removed_color = colors.remove(source);
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

/// Expands one call while preserving source-local visibility, bindings, and provenance.
#[expect(
    clippy::too_many_arguments,
    reason = "expansion carries independent lexical, identity, and provenance state"
)]
#[expect(
    clippy::too_many_lines,
    reason = "the expansion transaction must preserve one source-local frame"
)]
fn expand(
    fragments: &BTreeMap<QuerySourceId, FragmentProgram>,
    imports: &BTreeMap<QuerySourceId, ImportIndex>,
    source: &QuerySourceId,
    call: &Call,
    result_name: &StepName,
    preceding: &[LetStep],
    inherited_inputs: &Bindings,
    inherited_parameters: &Bindings,
    root_parameters: &[ryo_values::domain::program::types::ParameterDeclaration],
    next: &mut StepIdentity,
    trace: &[QueryTraceSite],
    origins: &mut Vec<QueryOrigin>,
    step_origins: &mut BTreeMap<StepIdentity, QueryOrigin>,
    contracts: &mut Vec<FragmentContract>,
) -> Result<Vec<LetStep>, LinkingError> {
    let fragment = fragments.get(source).ok_or_else(|| {
        located(
            source,
            call.span,
            trace.to_vec(),
            format!("query module `{}` was not parsed", source.0),
        )
    })?;
    let BoundArguments { inputs, parameters } = bind_arguments(
        source,
        fragment,
        call,
        preceding,
        inherited_inputs,
        inherited_parameters,
        root_parameters,
        trace,
    )?;
    let result_identity = ryo_values::domain::program::service::identity_of(fragment.result.step.1)
        .ok_or_else(|| {
            located(
                source,
                fragment.result.span,
                trace.to_vec(),
                super::constants::UNDECLARED_RETURNED_STEP,
            )
        })?;
    let mut identities = BTreeMap::new();
    for step in &fragment.steps {
        let identity =
            ryo_values::domain::program::service::identity_of(step.name.1).ok_or_else(|| {
                located(
                    source,
                    step.span,
                    trace.to_vec(),
                    super::constants::MISSING_STEP_IDENTITY,
                )
            })?;
        let replacement = if identity == result_identity {
            ryo_values::domain::program::service::identity_of(result_name.1).ok_or_else(|| {
                located(
                    source,
                    call.span,
                    trace.to_vec(),
                    super::constants::MISSING_CALL_IDENTITY,
                )
            })?
        } else {
            let fresh = *next;
            *next = StepIdentity(next.0.saturating_add(1));
            fresh
        };
        let _previous_identity = identities.insert(identity, replacement);
    }

    let mut expanded = Vec::new();
    // Keep the fragment's lexical frame separate from flattened expansion output: a nested
    // fragment exposes only its call result, never its private local definitions.
    let mut visible = Vec::new();
    for original in &fragment.steps {
        let mut step = original.clone();
        remap_step(
            &mut step,
            &identities,
            &inputs,
            &parameters,
            source,
            &visible,
            trace,
        )?;
        if ryo_values::domain::program::service::identity_of(step.name.1)
            == ryo_values::domain::program::service::identity_of(result_name.1)
        {
            step.name.0 = result_name.0.clone();
        }
        let mut step_trace = trace.to_vec();
        let step_origin = QueryOrigin {
            source_id: source.clone(),
            span: step.span,
            trace: step_trace.clone(),
        };
        origins.push(step_origin.clone());
        if let Some(identity) = ryo_values::domain::program::service::identity_of(step.name.1) {
            let _previous_step_origin = step_origins.insert(identity, step_origin);
        }
        if let Expression::Call(inner) = &step.expression
            && let Some(import) = imports
                .get(source)
                .and_then(|found| found.0.get(&*inner.name.0))
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
                &step_trace,
                origins,
                step_origins,
                contracts,
            )?);
        } else {
            expanded.push(step.clone());
        }
        let mut visible_step = original.clone();
        visible_step.name.1 = ryo_values::domain::program::service::identified(
            ryo_values::domain::program::service::identity_of(step.name.1),
        );
        visible.push(visible_step);
    }
    let first = expanded
        .first()
        .and_then(|step| ryo_values::domain::program::service::identity_of(step.name.1))
        .ok_or_else(|| {
            located(
                source,
                fragment.output_span,
                trace.to_vec(),
                super::constants::EMPTY_FRAGMENT,
            )
        })?;
    let result =
        ryo_values::domain::program::service::identity_of(result_name.1).ok_or_else(|| {
            located(
                source,
                fragment.result.span,
                trace.to_vec(),
                super::constants::MISSING_RESULT_IDENTITY,
            )
        })?;
    let input_contracts = fragment
        .inputs
        .iter()
        .map(|input| {
            let value = inputs.0.get(&*input.name.0).cloned().ok_or_else(|| {
                located(
                    source,
                    input.span,
                    trace.to_vec(),
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
        .collect::<Result<Vec<_>, LinkingError>>()?;
    contracts.push(FragmentContract {
        first,
        result,
        inputs: input_contracts,
        output: fragment.output.clone(),
        output_span: fragment.output_span,
        origin: QueryOrigin {
            source_id: source.clone(),
            span: fragment.output_span,
            trace: trace.to_vec(),
        },
    });
    Ok(expanded)
}

/// Binds a call's named inputs and scalar parameters to a fragment signature.
#[expect(
    clippy::too_many_arguments,
    reason = "binding compares one call against its lexical inputs and declarations"
)]
fn bind_arguments(
    source: &QuerySourceId,
    fragment: &FragmentProgram,
    call: &Call,
    preceding: &[LetStep],
    inherited_inputs: &Bindings,
    inherited_parameters: &Bindings,
    root_parameters: &[ryo_values::domain::program::types::ParameterDeclaration],
    trace: &[QueryTraceSite],
) -> Result<BoundArguments, LinkingError> {
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
    let mut inputs = Bindings::default();
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
        let _previous_input = inputs
            .0
            .insert(FragmentBindingKey(input.name.0.clone()), value);
    }
    let mut parameters = Bindings::default();
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
        if scalar_matches(&parameter.kind, &value, root_parameters) == ScalarMatch::Differs {
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
        let _previous_parameter = parameters
            .0
            .insert(FragmentBindingKey(parameter.name.0.clone()), value);
    }
    Ok(BoundArguments { inputs, parameters })
}

/// Resolves a relation argument against inherited bindings and visible preceding steps.
fn bind_relation_argument(
    value: &ArgumentValue,
    inherited: &Bindings,
    preceding: &[LetStep],
) -> ArgumentValue {
    match value {
        ArgumentValue::Word(word) => inherited.0.get(&*word.0).cloned().unwrap_or_else(|| {
            preceding
                .iter()
                .rev()
                .find(|step| step.name.0 == word.0)
                .map_or_else(
                    || value.clone(),
                    |step| ArgumentValue::Step(step.name.clone()),
                )
        }),
        ArgumentValue::Text(_)
        | ArgumentValue::Number(_)
        | ArgumentValue::Step(_)
        | ArgumentValue::Parameter(_)
        | ArgumentValue::List(_)
        | ArgumentValue::Map(_) => substitute_value(value, inherited),
    }
}

/// Checks a fragment scalar argument against its declared parameter type.
fn scalar_matches(
    kind: &ryo_values::domain::program::types::TypeName,
    value: &ArgumentValue,
    root_parameters: &[ryo_values::domain::program::types::ParameterDeclaration],
) -> ScalarMatch {
    let Some(expected) = fragment_scalar_kind_named(&kind.0) else {
        return ScalarMatch::Differs;
    };
    let actual = match value {
        ArgumentValue::Text(_) => Some(ParameterType::Text),
        ArgumentValue::Number(_) => Some(ParameterType::Number),
        ArgumentValue::Word(word) if BOOLEAN_WORDS.contains(&word.0.as_ref()) => {
            Some(ParameterType::Flag)
        }
        ArgumentValue::Parameter(parameter) => root_parameters
            .iter()
            .find(|declared| declared.name.0.as_ref() == parameter.0.as_ref())
            .and_then(|declared| fragment_scalar_kind_named(&declared.kind.0)),
        ArgumentValue::Word(_)
        | ArgumentValue::Step(_)
        | ArgumentValue::List(_)
        | ArgumentValue::Map(_) => None,
    };
    if actual == Some(expected) {
        ScalarMatch::Matches
    } else {
        ScalarMatch::Differs
    }
}

/// Rewrites one fragment step's local identities, inputs, parameters, and nested calls.
fn remap_step(
    step: &mut LetStep,
    identities: &BTreeMap<StepIdentity, StepIdentity>,
    inputs: &Bindings,
    parameters: &Bindings,
    source: &QuerySourceId,
    preceding: &[LetStep],
    trace: &[QueryTraceSite],
) -> Result<(), LinkingError> {
    if let Some(identity) = ryo_values::domain::program::service::identity_of(step.name.1) {
        step.name.1 =
            ryo_values::domain::program::service::identified(identities.get(&identity).copied());
    }
    match &mut step.expression {
        Expression::Select(select) => {
            remap_select(select, identities, inputs, parameters, source, trace)?;
        }
        Expression::Call(call) => {
            remap_call(
                call,
                inputs,
                parameters,
                preceding,
                source,
                trace,
                ParameterCheck::Strict,
            )?;
        }
    }
    Ok(())
}

/// Rewrites step and relation sources in a select to their caller bindings.
fn remap_select(
    select: &mut Select,
    identities: &BTreeMap<StepIdentity, StepIdentity>,
    inputs: &Bindings,
    parameters: &Bindings,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkingError> {
    for query_source in std::iter::once(&mut select.from).chain(match &mut select.join {
        ryo_values::domain::program::types::Joining::Alone => None,
        ryo_values::domain::program::types::Joining::Joined(join) => Some(&mut join.source),
    }) {
        let SourceReference::Step(reference) = &query_source.reference else {
            continue;
        };
        let replacement = if let Some(identity) =
            ryo_values::domain::program::service::identity_of(reference.1)
        {
            identities.get(&identity).copied().map(|identity| {
                SourceReference::Step(StepName(
                    reference.0.clone().into(),
                    ryo_values::domain::program::types::StepIdentified::Assigned(identity),
                ))
            })
        } else {
            inputs
                .get(reference.0.as_ref())
                .and_then(|input| match input {
                    ArgumentValue::Text(name) => Some(SourceReference::Relation(
                        ryo_values::domain::program::types::WrittenRelation(name.0.clone()),
                    )),
                    ArgumentValue::Step(step) => Some(SourceReference::Step(step.clone())),
                    ArgumentValue::Word(word) => Some(SourceReference::Step(StepName(
                        word.0.clone().into(),
                        ryo_values::domain::program::types::StepIdentified::Unassigned,
                    ))),
                    ArgumentValue::Number(_)
                    | ArgumentValue::Parameter(_)
                    | ArgumentValue::List(_)
                    | ArgumentValue::Map(_) => None,
                })
        };
        if let Some(replacement) = replacement {
            query_source.reference = replacement;
        }
    }
    for condition in &mut select.filters {
        remap_condition(condition, parameters, source, trace)?;
    }
    Ok(())
}

/// Rewrites parameters through a nested boolean condition.
fn remap_condition(
    condition: &mut Condition,
    parameters: &Bindings,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkingError> {
    match condition {
        Condition::Test(filter) => remap_filter(filter, parameters, source, trace),
        Condition::Any(alternatives) => {
            for alternative in alternatives {
                for nested_condition in alternative {
                    remap_condition(nested_condition, parameters, source, trace)?;
                }
            }
            Ok(())
        }
    }
}

/// Rewrites both operands in one filter.
fn remap_filter(
    filter: &mut Filter,
    parameters: &Bindings,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkingError> {
    remap_operand(&mut filter.left, parameters, filter.span, source, trace)?;
    remap_operand(&mut filter.right, parameters, filter.span, source, trace)
}

/// Rewrites a parameter operand using the current fragment bindings.
fn remap_operand(
    operand: &mut WrittenOperand,
    parameters: &Bindings,
    span: QuerySpan,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkingError> {
    match operand {
        WrittenOperand::Parameter(parameter) => {
            let value = parameters.0.get(&*parameter.0).ok_or_else(|| {
                located(
                    source,
                    span,
                    trace.to_vec(),
                    format!("fragment parameter `${}` was not bound", parameter.0),
                )
            })?;
            *operand = match value {
                ArgumentValue::Text(value) => WrittenOperand::Text(value.clone()),
                ArgumentValue::Number(value) => WrittenOperand::Number(*value),
                ArgumentValue::Parameter(value) => WrittenOperand::Parameter(value.clone()),
                ArgumentValue::Word(_)
                | ArgumentValue::Step(_)
                | ArgumentValue::List(_)
                | ArgumentValue::Map(_) => {
                    return Err(located(
                        source,
                        span,
                        trace.to_vec(),
                        super::constants::INVALID_FILTER_PARAMETER,
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

/// Substitutes fragment arguments and resolves relation inputs at a call site.
fn remap_call(
    call: &mut Call,
    inputs: &Bindings,
    parameters: &Bindings,
    preceding: &[LetStep],
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
    strictness: ParameterCheck,
) -> Result<(), LinkingError> {
    if strictness == ParameterCheck::Strict
        && let Some(parameter) = call
            .arguments
            .iter()
            .find_map(|argument| unbound_parameter(&argument.value, parameters))
    {
        return Err(located(
            source,
            call.span,
            trace.to_vec(),
            format!("fragment parameter `${}` is not declared", parameter.0),
        ));
    }
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
            argument.value = inputs.0.get(&*word.0).cloned().unwrap_or_else(|| {
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

/// The first parameter reference in a call value that has no local fragment binding.
/// Finds the first parameter reference absent from the current fragment's declarations.
fn unbound_parameter<'a>(
    value: &'a ArgumentValue,
    bindings: &Bindings,
) -> Option<&'a ryo_values::domain::program::types::WrittenParameter> {
    match value {
        ArgumentValue::Parameter(parameter) => {
            (!bindings.0.contains_key(&*parameter.0)).then_some(parameter)
        }
        ArgumentValue::List(values) => values
            .iter()
            .find_map(|nested_value| unbound_parameter(nested_value, bindings)),
        ArgumentValue::Map(arguments) => arguments
            .iter()
            .find_map(|argument| unbound_parameter(&argument.value, bindings)),
        ArgumentValue::Text(_)
        | ArgumentValue::Number(_)
        | ArgumentValue::Word(_)
        | ArgumentValue::Step(_) => None,
    }
}

/// Recursively substitutes scalar parameters in an argument value.
fn substitute_value(value: &ArgumentValue, bindings: &Bindings) -> ArgumentValue {
    match value {
        ArgumentValue::Parameter(parameter) => bindings
            .0
            .get(&*parameter.0)
            .cloned()
            .unwrap_or_else(|| value.clone()),
        ArgumentValue::List(values) => ArgumentValue::List(
            values
                .iter()
                .map(|nested_value| substitute_value(nested_value, bindings))
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
        ArgumentValue::Text(_)
        | ArgumentValue::Number(_)
        | ArgumentValue::Word(_)
        | ArgumentValue::Step(_) => value.clone(),
    }
}

/// Replaces an argument with its recursively substituted value.
fn substitute_argument_value(value: &mut ArgumentValue, bindings: &Bindings) {
    *value = substitute_value(value, bindings);
}

/// Resolves the target module identifier written by an import.
fn source_id(
    source: &QuerySourceId,
    import: &ImportDeclaration,
) -> Result<QuerySourceId, LinkingError> {
    ryo_values::domain::program::service::source_id(
        ryo_values::domain::program::types::QuerySpelling(&import.module.0),
    )
    .map_err(|error| {
        located(
            source,
            import.span,
            Vec::new(),
            format!("invalid module id: {error}"),
        )
    })
}

/// Records one source-to-target import location in a diagnostic trace.
fn trace_site(source: &QuerySourceId, target: &QuerySourceId, span: QuerySpan) -> QueryTraceSite {
    QueryTraceSite {
        source_id: source.clone(),
        target_id: target.clone(),
        span,
    }
}

/// Builds a link error with its source and import trace.
fn located(
    source_id: &QuerySourceId,
    span: QuerySpan,
    trace: Vec<QueryTraceSite>,
    message: impl std::fmt::Display,
) -> LinkingError {
    LinkingError::Located {
        origin: QueryOrigin {
            source_id: source_id.clone(),
            span,
            trace,
        },
        message: message.to_string(),
    }
}

/// Assigns root source locations to parser nodes not already attributed to fragments.
fn attribute_root(program: &mut WrittenProgram, root: &QuerySourceId) {
    for import in &program.imports {
        program.origins.push(QueryOrigin {
            source_id: root.clone(),
            span: import.span,
            trace: Vec::new(),
        });
    }
    for step in &program.steps {
        if let Some(identity) = ryo_values::domain::program::service::identity_of(step.name.1) {
            let _root_step_origin =
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
    for output in &mut program.observations {
        output.origin = Some(QueryOrigin {
            source_id: root.clone(),
            span: output.span,
            trace: Vec::new(),
        });
        program.origins.push(
            output
                .origin
                .clone()
                .expect(super::constants::ASSIGNED_ROOT_ORIGIN),
        );
    }
    program.emit_origin = ryo_values::domain::program::types::EmitOrigin::Located(QueryOrigin {
        source_id: root.clone(),
        span: program.emit.span,
        trace: Vec::new(),
    });
}

/// Root local definitions cannot shadow external inputs; hygienic private fragment definitions remain local.
fn bind_declared_sources(
    root: &mut WrittenProgram,
    observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    imports: &ImportIndex,
    fragments: &BTreeMap<QuerySourceId, FragmentProgram>,
    source: &QuerySourceId,
) -> Result<(), LinkingError> {
    let names: BTreeSet<_> = observations
        .iter()
        .map(|input| input.name.0.as_ref())
        .collect();
    if names.is_empty() {
        return Ok(());
    }
    for step in &root.steps {
        if names.contains(step.name.0.as_ref()) {
            return Err(located(
                source,
                step.span,
                Vec::new(),
                format!(
                    "step `{}` conflicts with a declared observation input",
                    step.name.0
                ),
            ));
        }
    }
    for step in &mut root.steps {
        match &mut step.expression {
            Expression::Select(select) => {
                let joined = match &mut select.join {
                    ryo_values::domain::program::types::Joining::Alone => None,
                    ryo_values::domain::program::types::Joining::Joined(join) => {
                        Some(&mut join.source)
                    }
                };
                for input in std::iter::once(&mut select.from).chain(joined) {
                    if let SourceReference::Step(name) = &input.reference
                        && ryo_values::domain::program::service::identity_of(name.1).is_none()
                        && names.contains(name.0.as_ref())
                    {
                        input.reference = SourceReference::Relation(
                            ryo_values::domain::program::types::WrittenRelation(name.0.clone()),
                        );
                    }
                }
            }
            Expression::Call(call) => {
                let shape = shape_named(&call.name);
                let fragment = imports
                    .0
                    .get(&*call.name.0)
                    .map(|import| source_id(source, import))
                    .transpose()?
                    .and_then(|id| fragments.get(&id));
                for argument in &mut call.arguments {
                    let relation_input = shape.is_some_and(|shape| {
                        crate::domain::syntax::service::is_input(shape.inputs, &argument.name)
                            == crate::domain::syntax::types::ArgumentRole::Input
                    }) || fragment.is_some_and(|fragment| {
                        fragment
                            .inputs
                            .iter()
                            .any(|input| input.name.0 == argument.name.0)
                    });
                    if !relation_input {
                        continue;
                    }
                    let external = match &argument.value {
                        ArgumentValue::Word(name) if names.contains(name.0.as_ref()) => {
                            Some(name.0.clone())
                        }
                        ArgumentValue::Step(name)
                            if ryo_values::domain::program::service::identity_of(name.1)
                                .is_none()
                                && names.contains(name.0.as_ref()) =>
                        {
                            Some(name.0.clone())
                        }
                        _ => None,
                    };
                    if let Some(name) = external {
                        argument.value = ArgumentValue::Text(
                            ryo_values::domain::program::types::WrittenLiteral(name),
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
