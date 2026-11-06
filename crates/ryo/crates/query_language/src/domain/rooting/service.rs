//! The `rooting` service: free functions named by verb over the identities and origins the root of a bundle is given.

use crate::domain::checking::types::ImportIndex;
use crate::domain::linking::errors::LinkingError;
use crate::domain::linking::service::located;
use crate::domain::linking::service::source_id;
use crate::domain::syntax::service::shape_named;
use ryo_values::domain::program::types::FragmentProgram;
use ryo_values::domain::program::types::WrittenProgram;
use ryo_values::domain::query_syntax::types::ArgumentValue;
use ryo_values::domain::query_syntax::types::Expression;
use ryo_values::domain::query_syntax::types::SourceReference;
use ryo_values::domain::query_tokens::types::QueryOrigin;
use ryo_values::domain::query_tokens::types::QuerySourceId;
use ryo_values::domain::query_tokens::types::StepIdentity;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Completes source-local identities for syntax ports that return named, unlinked programs.
pub(crate) fn assign_root_identities(program: &mut WrittenProgram) {
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
                ryo_values::domain::query_syntax::types::Joining::Alone => None,
                ryo_values::domain::query_syntax::types::Joining::Joined(join) => {
                    Some(&mut join.source)
                }
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

/// Assigns root source locations to parser nodes not already attributed to fragments.
pub(crate) fn attribute_root(program: &mut WrittenProgram, root: &QuerySourceId) {
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
        let origin = QueryOrigin {
            source_id: root.clone(),
            span: output.span,
            trace: Vec::new(),
        };
        output.origin =
            ryo_values::domain::query_tokens::types::Attribution::Located(origin.clone());
        program.origins.push(origin);
    }
    program.emit_origin =
        ryo_values::domain::query_tokens::types::Attribution::Located(QueryOrigin {
            source_id: root.clone(),
            span: program.emit.span,
            trace: Vec::new(),
        });
}

/// Root local definitions cannot shadow external inputs; hygienic private fragment definitions remain local.
pub(crate) fn bind_declared_sources(
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
                    ryo_values::domain::query_syntax::types::Joining::Alone => None,
                    ryo_values::domain::query_syntax::types::Joining::Joined(join) => {
                        Some(&mut join.source)
                    }
                };
                for input in std::iter::once(&mut select.from).chain(joined) {
                    if let SourceReference::Step(name) = &input.reference
                        && ryo_values::domain::program::service::identity_of(name.1).is_none()
                        && names.contains(name.0.as_ref())
                    {
                        input.reference = SourceReference::Relation(
                            ryo_values::domain::query_tokens::types::WrittenRelation(
                                name.0.clone(),
                            ),
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
                            ryo_values::domain::query_tokens::types::WrittenLiteral(name),
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
