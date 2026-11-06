//! Publication of a region result and replay of an admitted publication.
use super::super::SourceInputs;
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::admission::ProducerRoot;
use crate::application::file_dag::program::admission::Representation;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use std::rc::Rc;

impl SourceInputs<'_, '_> {
    pub(in crate::application::file_dag::program::facade_runtime::source_admission) fn publish_source(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        mut representation: Representation,
    ) -> Result<AdmittedInput, OperatingError> {
        let actual_partition = self.execution.source_partition(root)?;
        actual_partition.require_supply(partition)?;
        let mut readiness =
            crate::application::file_dag::program::admission::contributor_readiness(
                &actual_partition,
                inputs,
                &self.owner,
            )?;
        if let (ProducerRoot::Region(region), Representation::Computed(rows)) =
            (root, &mut representation)
            && matches!(
                readiness,
                crate::application::file_dag::program::admission::Readiness::Partial { .. }
            )
        {
            // A region starts with the admitted operands, then contains only
            // pure unary continuations. Absence operations publish before any
            // continuation can erase their completeness key.
            let code = self
                .program
                .program()
                .regions()
                .get(region.0)
                .ok_or(OperatingError::NotRun)?;
            let column = crate::domain::executable_program::service::column(&code.route).filter(|_| inputs.len() == 2);
            let ordered = crate::domain::executable_program::service::ordered_existence(&code.route).filter(|shape| {
                inputs.len() == 1
                    && inputs[0].has_root(ProducerRoot::Region(shape.source))
                        == crate::application::file_dag::program::admission::Agreement::Same
            });
            // Alternatives certify their original semantic operands, not the
            // unary Project/Select that owns the final publication boundary.
            let semantic = if let Some(shape) = column {
                self.program
                    .program()
                    .regions()
                    .get(shape.join().0)
                    .ok_or(OperatingError::NotRun)?
            } else {
                code
            };
            let instruction = semantic
                .instructions
                .first()
                .ok_or(OperatingError::NotRun)?;
            let contributions = inputs
                .iter()
                .enumerate()
                .map(|(ordinal, input)| {
                    let kinds = if let Some(shape) = ordered {
                        self.program
                            .program()
                            .regions()
                            .get(shape.source.0)
                            .and_then(|source| source.instructions.last())
                            .ok_or(OperatingError::NotRun)?
                            .contract
                            .output_kinds
                            .as_ref()
                            .map_err(Clone::clone)?
                    } else {
                        instruction
                            .contract
                            .input_kinds
                            .get(ordinal)
                            .ok_or(OperatingError::NotRun)?
                    };
                    Ok(crate::domain::completion::types::InputCoverage {
                        ordinal,
                        kinds,
                        coverage: input.coverage()?,
                    })
                })
                .collect::<Result<Vec<_>, OperatingError>>()?;
            crate::domain::completion::service::filter_contributor_rows(
                &instruction.operation,
                instruction.contract.absence_inputs,
                &contributions,
                rows,
            );
            let coverage = match &instruction.contract.output_kinds {
                Ok(kinds) => {
                    if let Some(shape) = ordered {
                        crate::domain::completion::service::ordered_output_coverage(
                            &shape.partition,
                            kinds,
                            &contributions[0],
                        )
                    } else {
                        crate::domain::completion::service::output_coverage(
                            &instruction.operation,
                            kinds,
                            &contributions,
                        )?
                    }
                }
                // An intentionally undemanded row fault must stay deferred.
                // No precise lineage can be certified from a refused schema.
                Err(_) => {
                    crate::domain::completion::types::CoverageState::Partial { files: vec![None] }
                }
            };
            readiness = match coverage {
                crate::domain::completion::types::CoverageState::Complete => {
                    crate::application::file_dag::program::admission::Readiness::Complete
                }
                crate::domain::completion::types::CoverageState::Partial { files } => {
                    crate::application::file_dag::program::admission::Readiness::Partial { files }
                }
            };
        }
        let publication = match representation {
            Representation::Computed(rows) => self.owner.computed(rows)?,
            Representation::Minted(publication) => publication,
        };
        self.state.bind(&self.owner, root, &publication)?;
        let resolved = self.owner.resolve(publication.as_ref(), self.read)?;
        let checked = self.owner.admit(
            publication.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::from(&resolved),
        )?;
        AdmittedInput::from_publication(
            Rc::clone(&self.owner),
            publication.clone(),
            &checked,
            root,
            actual_partition,
            readiness,
        )
    }
    pub(in crate::application::file_dag::program::facade_runtime::source_admission) fn replay_source(
        &self,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        output: &mut dyn crate::domain::operating::types::Emit,
    ) -> Result<(), OperatingError> {
        let rows = input.rows(&self.owner, self.read)?;
        crate::application::file_dag::execution::output::replay(&rows, context, output)
    }
}
