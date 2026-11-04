//! Request-prepared declarations from actual immutable producers, never operation kinds.
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{PartitionRequirement, ProducerPartitions},
    file_program::types::{ResolvedObservationBinding, ResolvedObservationProducer},
    operating::errors::OperatingError,
    rules::types::RuleSpec,
    table_model::types::ParameterBinding,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{
    hosted::types::HostRelation,
    observations::types::{ObservationBindingId, ObservationProducer},
    plan_parts::types::{HostedNumber, Input, ParameterId},
    plans::types::{Plan, StepScope},
};

type Declaration = Result<PartitionRequirement, OperatingError>;
struct RulePartitions {
    parameters: Vec<Declaration>,
    observations: Vec<Declaration>,
}
/// No public field or alternate constructor can install guessed producer provenance.
/// These compact declarations belong to the same frozen request as the supplied sources.
pub(super) struct SourcePartitions {
    rules: Vec<Result<RulePartitions, OperatingError>>,
    hosted: usize,
}
impl SourcePartitions {
    pub(super) fn new(
        rules: &[RuleSpec],
        plans: &[Result<&Plan, OperatingError>],
        bindings: &[Vec<ParameterBinding>],
        hosted: &[HostRelation],
        observations: &[Vec<ResolvedObservationBinding>],
    ) -> Result<Self, OperatingError> {
        if plans.len() != rules.len()
            || bindings.len() != rules.len()
            || observations.len() != rules.len()
        {
            return Err(OperatingError::ShapeMismatch);
        }
        let mut declarations = Vec::with_capacity(rules.len());
        for (index, plan) in plans.iter().enumerate() {
            let plan = match plan {
                Ok(plan) => *plan,
                Err(error) => {
                    declarations.push(Err(error.clone()));
                    continue;
                }
            };
            if bindings[index].len() > plan.parameters.len()
                || observations[index].len() != plan.observation_inputs.len()
            {
                return Err(OperatingError::ShapeMismatch);
            }
            // ParameterBinding contains owned scalar/list/table values from this request.
            // Each real bound value is invariant; a missing value remains an Unbound refusal.
            let parameters = plan
                .parameters
                .iter()
                .enumerate()
                .map(|(ordinal, _)| {
                    bindings[index]
                        .get(ordinal)
                        .map(|_| PartitionRequirement::RequestInvariant)
                        .ok_or(OperatingError::Unbound)
                })
                .collect();
            let mut observed = Vec::with_capacity(observations[index].len());
            for (ordinal, binding) in observations[index].iter().enumerate() {
                if binding.input.0 as usize != ordinal {
                    return Err(incomplete(
                        "observation binding ordinal is not its declared slot",
                    ));
                }
                observed.push(observation_partition(
                    &rules[index],
                    plan,
                    binding,
                    rules,
                    plans,
                ));
            }
            declarations.push(Ok(RulePartitions {
                parameters,
                observations: observed,
            }));
        }
        // HostRelation is an owned request snapshot, not a lazy per-file producer. Its
        // rows may name files without changing the lifetime of the source population.
        Ok(Self {
            rules: declarations,
            hosted: hosted.len(),
        })
    }
    fn rule(&self, rule: RuleIndex) -> Result<&RulePartitions, OperatingError> {
        self.rules
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?
            .as_ref()
            .map_err(Clone::clone)
    }
}
impl ProducerPartitions for SourcePartitions {
    fn hosted(&self, rule: RuleIndex, relation: HostedNumber) -> Declaration {
        self.rule(rule)?;
        if (relation.0 as usize) < self.hosted {
            Ok(PartitionRequirement::RequestInvariant)
        } else {
            Err(OperatingError::NotRead)
        }
    }
    fn parameter(&self, rule: RuleIndex, parameter: ParameterId) -> Declaration {
        self.rule(rule)?
            .parameters
            .get(parameter.0 as usize)
            .cloned()
            .unwrap_or(Err(OperatingError::Unbound))
    }
    fn relation(&self, relation: Relation) -> PartitionRequirement {
        relation_partition(relation)
    }
    fn observation(&self, rule: RuleIndex, input: ObservationBindingId) -> Declaration {
        self.rule(rule)?
            .observations
            .get(input.0 as usize)
            .cloned()
            .unwrap_or_else(|| Err(incomplete("observation input has no producer declaration")))
    }
}
fn incomplete(reason: &str) -> OperatingError {
    OperatingError::ObservationIncomplete {
        reason: reason.into(),
    }
}
fn observation_partition(
    rule: &RuleSpec,
    plan: &Plan,
    binding: &ResolvedObservationBinding,
    rules: &[RuleSpec],
    plans: &[Result<&Plan, OperatingError>],
) -> Declaration {
    let ordinal = binding.input.0 as usize;
    let schema = plan
        .observation_inputs
        .get(ordinal)
        .ok_or_else(|| incomplete("observation input ordinal is undeclared"))?;
    let declaration = rule
        .observations
        .iter()
        .find(|declaration| declaration.name == schema.name)
        .ok_or_else(|| incomplete("observation has no authored consumer declaration"))?;
    if schema != &binding.schema
        || declaration.columns != schema.columns
        || declaration.producers.len() != binding.producers.len()
    {
        return Err(incomplete(
            "observation does not match its actual consumer declaration",
        ));
    }
    if binding.producers.is_empty() {
        return Err(incomplete("observation has no declared producer"));
    }
    let mut required = None;
    for (producer, authored) in binding.producers.iter().zip(&declaration.producers) {
        let ResolvedObservationProducer::Output { rule, output } = producer else {
            return Err(incomplete("observation producer has no executable query"));
        };
        let producer = plans
            .get(rule.0)
            .ok_or_else(|| incomplete("observation producer rule is missing"))?
            .as_ref()
            .map_err(Clone::clone)?;
        let root = producer
            .observations
            .get(output.0 as usize)
            .ok_or_else(|| incomplete("observation producer output ordinal is undeclared"))?;
        let producer_rule = rules
            .get(rule.0)
            .ok_or_else(|| incomplete("observation producer rule is missing"))?;
        if !matches!(authored, ObservationProducer::Output { rule, output }
            if rule == &producer_rule.id && output == &root.name)
            || root.columns != schema.columns
        {
            return Err(incomplete(
                "observation producer differs from its authored output",
            ));
        }
        let phase = match root.source {
            Input::Step(step) => {
                producer
                    .steps
                    .get(step.0 as usize)
                    .ok_or_else(|| incomplete("observation producer root step is missing"))?
                    .scope
            }
            Input::Relation(relation) => match relation_partition(relation) {
                PartitionRequirement::Project => StepScope::CrossFile,
                PartitionRequirement::CurrentFile => StepScope::PerFile,
                PartitionRequirement::RequestInvariant | PartitionRequirement::InputLineage => {
                    return Err(incomplete("relation has no concrete producer phase"));
                }
            },
            Input::BoundObservation(input) => {
                producer
                    .observation_inputs
                    .get(input.0 as usize)
                    .ok_or_else(|| incomplete("observation producer root binding is missing"))?
                    .scope
            }
        };
        if root.scope != phase || schema.scope != phase {
            return Err(incomplete(
                "observation phase disagrees with its actual producer root",
            ));
        }
        let partition = match phase {
            StepScope::PerFile => PartitionRequirement::CurrentFile,
            StepScope::CrossFile => PartitionRequirement::Project,
        };
        if required.is_some_and(|previous| previous != partition) {
            return Err(incomplete(
                "observation producers require different partitions",
            ));
        }
        required = Some(partition);
    }
    required.ok_or_else(|| incomplete("observation has no declared producer"))
}
/// Exhaustive over the owned catalog: adding a relation requires choosing its producer.
/// Tree and engine populations are read for one file; manifest populations are read
/// once for the project. A file-valued cell does not alter either declaration.
fn relation_partition(relation: Relation) -> PartitionRequirement {
    use PartitionRequirement::{CurrentFile, Project};
    match relation {
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
        | Relation::LiteralExpressionParse
        | Relation::StaticImport
        | Relation::StandardLibraryImport
        | Relation::StringLiteral
        | Relation::BytesLiteral
        | Relation::FormatSpecification
        | Relation::SyntaxChild
        | Relation::SyntaxError
        | Relation::SyntaxTree
        | Relation::SemanticSyntax
        | Relation::SemanticChild
        | Relation::EmbeddedExpression
        | Relation::SourceLocation
        | Relation::IndentRegion
        | Relation::Mapping
        | Relation::Entry
        | Relation::SequenceElement
        | Relation::Prose
        | Relation::NumericLiteral
        | Relation::TokenLink
        | Relation::FilesystemFile
        | Relation::FilesystemLineCount
        | Relation::FilesystemExecutable
        | Relation::FilesystemReadFailure => CurrentFile,
        Relation::PackageDependency
        | Relation::WorkspaceMember
        | Relation::SourceDependency
        | Relation::AnalyzedSource
        | Relation::FilePackage
        | Relation::ModuleIdentity
        | Relation::ProjectNamespaceMembership
        | Relation::ManifestDiagnostic => Project,
    }
}

#[cfg(test)]
#[path = "partitions_test.rs"]
mod tests;
