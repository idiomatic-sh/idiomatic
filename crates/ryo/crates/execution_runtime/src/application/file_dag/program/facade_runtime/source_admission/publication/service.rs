//! Publication of a region result and replay of an admitted publication.
use super::super::service::SourceInputs;
use crate::application::file_dag::program::admission::service::AdmittedInput;
use crate::application::file_dag::program::admission::service::Inputs;
use crate::application::file_dag::program::admission::service::Representation;
use crate::application::file_dag::program::admission::types::Partition;
use crate::application::file_dag::program::admission::types::ProducerRoot;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use std::rc::Rc;

impl SourceInputs<'_, '_> {
    pub(crate) fn publish_source(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        mut representation: Representation,
    ) -> Result<AdmittedInput, OperatingError> {
        let actual_partition = self.execution.source_partition(root)?;
        actual_partition.require_supply(partition)?;
        let mut read =
            crate::application::file_dag::program::admission::service::contributed_proof(
                &actual_partition,
                inputs,
                &self.owner,
            )?;
        if let (ProducerRoot::Region(region), Representation::Computed(rows)) =
            (root, &mut representation)
            && read.observation()
                == crate::application::file_dag::program::admission::types::ObservationReadiness::Waiting
        {
            // A region starts with the admitted operands, then contains only
            // pure unary continuations. Absence operations publish before any
            // continuation can erase their completeness key.
            let code = self
                .program
                .program()
                .regions()
                .get(region.0)
                .ok_or_else(|| OperatingError::NotRun)?;
            let ordered =
                ryo_execution_domain::domain::executable_program::service::ordered_existence(
                    &code.route(),
                )
                .filter(|shape| {
                    inputs.len() == 1
                        && inputs[0].has_root(ProducerRoot::Region(shape.source))
                            == crate::application::file_dag::program::admission::types::Agreement::Same
                });
            let instruction = code
                .instructions()
                .first()
                .ok_or_else(|| OperatingError::NotRun)?;
            let contributions = inputs
                .iter()
                .enumerate()
                .map(|(ordinal, input)| {
                    let kinds = if let Some(shape) = ordered {
                        self.program
                            .program()
                            .regions()
                            .get(shape.source.0)
                            .and_then(|source| source.instructions().last())
                            .ok_or_else(|| OperatingError::NotRun)?
                            .contract()
                            .output_kinds
                            .as_ref()
                            .map_err(Clone::clone)?
                    } else {
                        instruction
                            .contract()
                            .input_kinds
                            .get(ordinal)
                            .ok_or_else(|| OperatingError::NotRun)?
                    };
                    // Where the operand's rows come from, as the compiler found it for the input the instruction reads.
                    let lineage = if let Some(shape) = ordered {
                        self.program
                            .program()
                            .regions()
                            .get(shape.source.0)
                            .map(|source| source.lineage().clone())
                            .unwrap_or_default()
                    } else {
                        code
                            .inputs()
                            .get(ordinal)
                            .map(|source| self.program.program().source_lineage(*source))
                            .unwrap_or_default()
                    };
                    Ok(ryo_execution_domain::domain::output_coverage::types::InputCoverage {
                        ordinal: ryo_execution_domain::domain::output_coverage::types::OperandOrdinal(ordinal),
                        kinds,
                        coverage: input.coverage().clone(),
                        lineage,
                    })
                })
                .collect::<Result<Vec<_>, OperatingError>>()?;
            ryo_execution_domain::domain::contributor_rows::service::filter_contributor_rows(
                &instruction.operation(),
                instruction.contract().absence_inputs,
                &contributions,
                rows,
            );
            let coverage = match &instruction.contract().output_kinds {
                Ok(kinds) => {
                    if let Some(shape) = ordered {
                        ryo_execution_domain::domain::output_coverage::service::ordered_output_coverage(
                            &shape.partition,
                            kinds,
                            &contributions[0],
                        )
                    } else {
                        ryo_execution_domain::domain::output_coverage::service::output_coverage(
                            &instruction.operation(),
                            kinds,
                            &contributions,
                        )?
                    }
                }
                // An intentionally undemanded row fault must stay deferred.
                // No precise lineage can be certified from a refused schema.
                Err(_) => {
                    ryo_execution_domain::domain::completion::service::partial_any(vec![None])
                }
            };
            read = crate::application::file_dag::program::admission::service::ReadProof::covered(
                actual_partition.clone(),
                coverage,
            );
        }
        let Representation::Computed(rows) = representation;
        let publication = self.owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(rows))?;
        self.state.bind(&self.owner, root, &publication)?;
        let resolved = self.owner.rows(&publication, self.read)?;
        let checked = self.owner.admit(
            &publication,
            ryo_execution_domain::domain::table_model::types::RepresentationInput::from(&resolved),
        )??;
        AdmittedInput::from_publication(
            Rc::clone(&self.owner),
            publication.clone(),
            &checked,
            root,
            read,
        )
    }
    pub(crate) fn replay_source(
        &self,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        output: &mut dyn ryo_execution_domain::domain::operator::types::Emit,
    ) -> Result<(), OperatingError> {
        let rows = input.rows(&self.owner, self.read)?;
        crate::application::file_dag::execution::output::service::replay(&rows, context, output)
    }
}
