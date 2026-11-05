//! Whether this phase owns a coordinator-reserved publication arena.
#[derive(Clone, Copy, Debug)]
pub enum PublicationSlot {
    /// No named roots are demanded from this phase.
    Unused,
    /// An ordinary, unique index reserved by the existing request owner.
    Reserved(ryo_relations::domain::relations::types::WorkerNumber),
}

/// Each named member carries its own admission source, never just decoded words.
#[derive(Clone)]
pub enum BoundMember {
    Current(crate::application::file_dag::program::facade::ObservedPublication),
    Restored(crate::application::file_dag::program::facade::ValidatedFrontier),
}

/// Application-only named binding. The producer schema remains pure metadata;
/// the member list retains the matching owner evidence in the same payload.
#[derive(Clone)]
pub struct BoundInput {
    pub input: ryo_values::domain::observations::types::ObservationBindingId,
    pub members: Result<Vec<BoundMember>, crate::domain::operating::errors::OperatingError>,
}

/// Admission preserves independent input successes and explicit refusal causes.
pub struct PreparedFrontiers {
    pub(in crate::application::file_dag) rows: Vec<FrontierInput>,
    pub(in crate::application::file_dag) program:
        std::sync::Arc<crate::domain::executable_program::types::ExecutableProgram>,
    pub(in crate::application::file_dag) required:
        crate::application::file_dag::program::admission::RequiredPartitions,
    pub(in crate::application::file_dag) refused:
        Vec<crate::domain::crossing::types::RefusedFrontier>,
}

/// Admitted phase inputs. Compact sources have schemas/name mappings but no
/// decoded cell allocation; the same graph owner materializes them on demand.
pub(crate) enum FrontierInput {
    Decoded {
        frontier: crate::domain::table_model::types::Frontier,
        coverage: crate::application::file_dag::completion::CoverageState,
    },
    Compact {
        input: ryo_values::domain::plan_parts::types::Input,
        table: crate::domain::table_words::types::CompactTable,
        spelling: Vec<ryo_relations::domain::relations::types::Name>,
        coverage: crate::application::file_dag::completion::CoverageState,
    },
}

impl PreparedFrontiers {
    pub(in crate::application::file_dag) fn validate_scope(
        &self,
        program: &std::sync::Arc<crate::domain::executable_program::types::ExecutableProgram>,
        required: &crate::application::file_dag::program::admission::RequiredPartitions,
    ) -> Result<(), crate::domain::operating::errors::OperatingError> {
        if !std::sync::Arc::ptr_eq(&self.program, program) || !self.required.same_request(required)
        {
            return Err(crate::domain::operating::errors::OperatingError::NotRead);
        }
        Ok(())
    }
}
