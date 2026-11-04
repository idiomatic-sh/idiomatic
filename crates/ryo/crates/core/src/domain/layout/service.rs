//! The `layout` service: free functions named by verb that lay a plan out for the worker that runs it.

use ryo_relations::domain::relations::service::kinds as relation_kinds;
use ryo_relations::domain::relations::types::{CellKind, Relation};

use crate::domain::lowering::service::lower;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operation_schema::service::schema_of;
use crate::domain::operation_schema::types::InputSchema;
#[cfg(test)]
use crate::domain::operator::types::{KeptSteps, PlanIdentity, PlanLayout, ReadCount, StepPlace};
use crate::domain::table_cells::types::TreeForm;
use crate::domain::table_model::types::{Frontier, ParameterBinding, Rows};
#[cfg(test)]
use crate::domain::trust::service::needs_of;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Plan, StepScope};

/// How many steps read the step `id`, the plan's emitting and handing over counting as reads.
#[must_use]
#[cfg(test)]
pub(crate) fn reads_of(plan: &Plan, id: StepPlace) -> ReadCount {
    let id = id.0;
    let steps = plan
        .steps
        .iter()
        .filter(|step| step.scope == StepScope::PerFile)
        .map(|step| {
            named(&inputs_of(&step.operation))
                .iter()
                .filter(|input| {
                    **input == Input::Step(StepId(u32::try_from(id).unwrap_or(u32::MAX)))
                })
                .count()
        })
        .sum::<usize>();
    let handed = plan
        .frontier
        .iter()
        .filter(|input| **input == Input::Step(StepId(u32::try_from(id).unwrap_or(u32::MAX))))
        .count();
    let observed = plan
        .observations
        .iter()
        .filter(|output| {
            output.source == Input::Step(StepId(u32::try_from(id).unwrap_or(u32::MAX)))
        })
        .count();
    ReadCount(steps + handed + observed + usize::from(usize::try_from(plan.emit.step.0) == Ok(id)))
}

/// The layout of a plan: which steps keep their rows, and what columns each answers.
///
/// # Errors
///
/// `NotRun` when a step reads a step that has no columns, `NotRead` when `read` has no rows for a relation, and whatever an
/// operator refuses.
#[cfg(test)]
pub(crate) fn layout_of<'f>(
    plan: &Plan,
    index: PlanIdentity,
    read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    context: &mut OperatorContext<'_>,
) -> Result<PlanLayout, OperatingError> {
    let emitted = usize::try_from(plan.emit.step.0).unwrap_or(usize::MAX);
    // A step keeps its rows when it is the one emitted, is handed to the cross-file steps, or is read by more than one step.
    let (kept, consumed): (Vec<bool>, Vec<bool>) = plan
        .steps
        .iter()
        .enumerate()
        .map(|(id, step)| {
            let handed = plan
                .frontier
                .iter()
                .filter(|input| {
                    **input == Input::Step(StepId(u32::try_from(id).unwrap_or(u32::MAX)))
                })
                .count();
            let exported = usize::from(id == emitted)
                + plan
                    .observations
                    .iter()
                    .filter(|output| {
                        output.source == Input::Step(StepId(u32::try_from(id).unwrap_or(u32::MAX)))
                    })
                    .count();
            let reads = reads_of(plan, StepPlace(id)).0;
            (
                step.scope != StepScope::CrossFile && (exported != 0 || handed != 0 || reads > 1),
                reads > handed + exported,
            )
        })
        .unzip();
    // Layout reads provider availability and column metadata only. No operator is run.
    let _span =
        tracing::trace_span!(target: "ryo::diagnostic::layout", "schema", plan = index.0).entered();
    let metadata = |relation| {
        // Probe every actual view before exposing only its schema to derivation.
        Ok(match read(relation)? {
            Rows::Stored(view) => InputSchema {
                kinds: view.kinds,
                tree: None,
            },
            Rows::Computed(table) => InputSchema {
                kinds: &table.kinds,
                tree: None,
            },
            Rows::Tree(tree) => InputSchema {
                kinds: ryo_relations::domain::relations::service::kinds(match tree.form {
                    TreeForm::Elements => Relation::SyntaxTree,
                    TreeForm::Edges => Relation::SyntaxChild,
                }),
                tree: Some(tree.form),
            },
            Rows::Selected(_) => return Err(OperatingError::WrongOperation),
        })
    };
    let kinds = schema_steps(plan, &metadata, context.bindings)?;
    if tracing::enabled!(target: "ryo::diagnostic::layout", tracing::Level::TRACE) {
        let file_steps = plan
            .steps
            .iter()
            .filter(|step| step.scope != StepScope::CrossFile);
        let relation_probes = file_steps
            .clone()
            .map(|step| {
                named(&inputs_of(&step.operation))
                    .iter()
                    .filter(|input| matches!(input, Input::Relation(_)))
                    .count()
            })
            .sum::<usize>();
        tracing::trace!(target: "ryo::diagnostic::layout", plan = index.0,
            schema_steps = file_steps.count(), relation_probes,
            output_columns = kinds.iter().map(Vec::len).sum::<usize>(),
            payload_rows = 0, metadata_only = true, "plan layout");
    }
    Ok(PlanLayout {
        plan: index,
        kept: KeptSteps(kept),
        consumed,
        kinds,
        lowering: lower(plan),
        needs: needs_of(plan),
    })
}

/// Fill missing frontier schemas in place, never copying resident cell payloads. A table the workers gave no row of has no kinds, for
/// the words of a table say them with its rows (a rule that governs no file, a relation no file wrote): the columns the input has are
/// then those the plan says, so that the steps after it find the columns they ask for.
///
/// # Errors
///
/// Whatever an operator refuses when the columns of the steps are worked out.
pub(crate) fn shaped(
    plan: &Plan,
    frontier: &mut [Frontier],
    context: &mut OperatorContext<'_>,
) -> Result<(), OperatingError> {
    if frontier.iter().all(|each| !each.table.kinds.is_empty()) {
        return Ok(());
    }
    // No file supplied this frontier. Use the declared relation representation,
    // as the old empty-tree fallback did, without making any table or elements.
    let metadata = |relation| {
        Ok(InputSchema {
            kinds: relation_kinds(relation),
            tree: match relation {
                Relation::SyntaxTree => Some(TreeForm::Elements),
                Relation::SyntaxChild => Some(TreeForm::Edges),
                Relation::Function
                | Relation::Annotates
                | Relation::Assigns
                | Relation::Comment
                | Relation::Directive
                | Relation::Decorates
                | Relation::Defines
                | Relation::Visibility
                | Relation::Inherits
                | Relation::Parameter
                | Relation::Reference
                | Relation::Mention
                | Relation::Scope
                | Relation::StaticCall
                | Relation::CallArgument
                | Relation::Resolves
                | Relation::BindingEvent
                | Relation::BindingRead
                | Relation::BindingUse
                | Relation::BindingShadow
                | Relation::ImplicitSymbol
                | Relation::QualifiedName
                | Relation::Identifier
                | Relation::ExpressionShape
                | Relation::LiteralProperties
                | Relation::NumericLiteral
                | Relation::FunctionTerminal
                | Relation::ExpressionType
                | Relation::InterpolatedStringLiteral
                | Relation::LanguageTarget
                | Relation::CompilerFeature
                | Relation::ConstantKey
                | Relation::FormatParse
                | Relation::FormatField
                | Relation::BraceFormatParse
                | Relation::BraceFormatField
                | Relation::RawFormatSegment
                | Relation::RawFormatField
                | Relation::TokenLink
                | Relation::LiteralExpressionParse
                | Relation::StaticImport
                | Relation::StandardLibraryImport
                | Relation::StringLiteral
                | Relation::BytesLiteral
                | Relation::FormatSpecification
                | Relation::SyntaxError
                | Relation::SemanticSyntax
                | Relation::SemanticChild
                | Relation::EmbeddedExpression
                | Relation::SourceLocation
                | Relation::IndentRegion
                | Relation::Mapping
                | Relation::Entry
                | Relation::SequenceElement
                | Relation::Prose
                | Relation::PackageDependency
                | Relation::WorkspaceMember
                | Relation::SourceDependency
                | Relation::AnalyzedSource
                | Relation::FilePackage
                | Relation::ManifestDiagnostic
                | Relation::ModuleIdentity
                | Relation::FilesystemFile
                | Relation::FilesystemLineCount
                | Relation::FilesystemExecutable
                | Relation::FilesystemReadFailure => None,
            },
        })
    };
    let kinds = schema_steps(plan, &metadata, context.bindings)?;
    // Complete all fallible metadata lookup before changing any supplied frontier.
    let filled = frontier
        .iter()
        .map(|each| {
            if !each.table.kinds.is_empty() {
                return Ok(None);
            }
            let kinds = match each.input {
                Input::Relation(relation) => relation_kinds(relation).to_vec(),
                Input::BoundObservation(id) => bound_kinds(plan, id)?,
                Input::Step(id) => kinds
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                    .cloned()
                    .ok_or(OperatingError::NotRun)?,
            };
            Ok(Some(kinds))
        })
        .collect::<Result<Vec<_>, OperatingError>>()?;
    for (each, kinds) in frontier.iter_mut().zip(filled) {
        if let Some(kinds) = kinds {
            each.table.kinds = kinds;
        }
    }
    Ok(())
}

/// Propagate schemas topologically, with only the caller's metadata/probe boundary.
fn schema_steps<'a>(
    plan: &Plan,
    read: &dyn Fn(Relation) -> Result<InputSchema<'a>, OperatingError>,
    bindings: &[ParameterBinding],
) -> Result<Vec<Vec<CellKind>>, OperatingError> {
    let mut kinds: Vec<Vec<CellKind>> = vec![Vec::new(); plan.steps.len()];
    let bound = plan
        .observation_inputs
        .iter()
        .map(|input| {
            input
                .columns
                .iter()
                .map(|column| column.kind)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (id, step) in plan.steps.iter().enumerate() {
        if step.scope == StepScope::CrossFile {
            continue;
        }
        let inputs = inputs_of(&step.operation);
        let schemas = named(&inputs)
            .iter()
            .map(|input| {
                match input {
                    Input::Step(source) => Ok(InputSchema {
                        kinds: kinds
                            .get(usize::try_from(source.0).unwrap_or(usize::MAX))
                            .ok_or(OperatingError::NotRun)?,
                        tree: None,
                    }),
                    Input::BoundObservation(id) => Ok(InputSchema {
                        kinds: bound
                            .get(usize::try_from(id.0).map_err(|_| OperatingError::NotRun)?)
                            .ok_or(OperatingError::NotRun)?,
                        tree: None,
                    }),
                    Input::Relation(relation) => {
                        // Do not substitute catalog metadata for a missing/refusing view.
                        read(*relation)
                    }
                }
            })
            .collect::<Result<Vec<_>, OperatingError>>()?;
        let answered = schema_of(&step.operation, &schemas, bindings)?;
        kinds[id] = answered;
    }
    Ok(kinds)
}

/// A bound input's schema is a compiled contract, not evidence of any resident rows or readiness.
fn bound_kinds(
    plan: &Plan,
    id: ryo_values::domain::observations::types::ObservationBindingId,
) -> Result<Vec<CellKind>, OperatingError> {
    let input = plan
        .observation_inputs
        .get(usize::try_from(id.0).map_err(|_| OperatingError::NotRun)?)
        .ok_or(OperatingError::NotRun)?;
    Ok(input.columns.iter().map(|column| column.kind).collect())
}
