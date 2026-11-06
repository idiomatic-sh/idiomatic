//! The `expanding` service: free functions named by verb over the expansion of every imported-fragment call into fresh, identity-linked steps.

use crate::domain::binding::service::bind_arguments;
use crate::domain::binding::types::Bindings;
use crate::domain::binding::types::BoundArguments;
use crate::domain::checking::service::imports_for;
use crate::domain::checking::service::reject_cycles;
use crate::domain::checking::service::validate_fragment_signature;
use crate::domain::checking::service::validate_import_targets;
use crate::domain::checking::service::validate_step_names;
use crate::domain::checking::types::ImportIndex;
use crate::domain::linking::errors::LinkingError;
use crate::domain::linking::service::located;
use crate::domain::linking::service::source_id;
use crate::domain::linking::service::trace_site;
use crate::domain::remapping::service::remap_call;
use crate::domain::remapping::service::remap_step;
use crate::domain::remapping::types::ParameterCheck;
use crate::domain::rooting::service::assign_root_identities;
use crate::domain::rooting::service::attribute_root;
use crate::domain::rooting::service::bind_declared_sources;
use ryo_values::domain::program::types::Call;
use ryo_values::domain::program::types::Expression;
use ryo_values::domain::program::types::FragmentContract;
use ryo_values::domain::program::types::FragmentContractInput;
use ryo_values::domain::program::types::FragmentProgram;
use ryo_values::domain::program::types::LetStep;
use ryo_values::domain::program::types::QueryBundle;
use ryo_values::domain::program::types::QueryOrigin;
use ryo_values::domain::program::types::QuerySourceId;
use ryo_values::domain::program::types::QueryTraceSite;
use ryo_values::domain::program::types::StepIdentity;
use ryo_values::domain::program::types::StepName;
use ryo_values::domain::program::types::WrittenProgram;
use std::collections::BTreeMap;

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

/// Expands one call while preserving source-local visibility, bindings, and provenance.
#[expect(
    clippy::too_many_arguments,
    reason = "expansion carries independent lexical, identity, and provenance state"
)]
#[expect(
    clippy::too_many_lines,
    reason = "the expansion transaction must preserve one source-local frame"
)]
pub(crate) fn expand(
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
