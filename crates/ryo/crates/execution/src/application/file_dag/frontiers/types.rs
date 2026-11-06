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
pub(crate) enum BoundMember {
    Current(crate::application::file_dag::program::protocol::types::ObservedPublication),
    Restored(crate::application::file_dag::program::restored::types::ValidatedFrontier),
}

/// Application-only named binding. The producer schema remains pure metadata;
/// the member list retains the matching owner evidence in the same payload.
#[derive(Clone)]
pub struct BoundInput {
    pub(crate) input: ryo_values::domain::observations::types::ObservationBindingId,
    pub(crate) members: Result<Vec<BoundMember>, crate::domain::operating::errors::OperatingError>,
}

/// Admission preserves independent input successes and explicit refusal causes.
#[derive(Clone)]
pub(in crate::application::file_dag) struct FrontierSource {
    pub input: ryo_values::domain::plan_parts::types::Input,
    pub root: Result<
        crate::domain::executable_program::types::InputSource,
        crate::domain::operating::errors::OperatingError,
    >,
}
/// One acquisition of the frontier sources. Roots of one population submit clones of it,
/// so the provider is read once per cohort, never once per root.
#[derive(Clone)]
pub struct PreparedFrontiers {
    pub(in crate::application::file_dag) producer: crate::domain::coverage::types::RuleIndex,
    pub(in crate::application::file_dag) sources: Box<[FrontierSource]>,
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
#[derive(Clone)]
pub(crate) enum FrontierInput {
    Decoded {
        frontier: crate::domain::table_model::types::Frontier,
        coverage: crate::domain::completion::types::CoverageState,
    },
    Compact {
        input: ryo_values::domain::plan_parts::types::Input,
        table: ryo_values::domain::table_words::types::CompactTable,
        spelling: Vec<ryo_relations::domain::relations::types::Name>,
        coverage: crate::domain::completion::types::CoverageState,
    },
}

impl PreparedFrontiers {
    pub(in crate::application::file_dag) fn validate_scope(
        &self,
        program: &std::sync::Arc<crate::domain::executable_program::types::ExecutableProgram>,
        required: &crate::application::file_dag::program::admission::RequiredPartitions,
    ) -> Result<(), crate::domain::operating::errors::OperatingError> {
        if !std::sync::Arc::ptr_eq(&self.program, program) || self.required.same_request(required)
            == crate::application::file_dag::program::admission::Agreement::Different
        {
            return Err(crate::domain::operating::errors::OperatingError::NotRead);
        }
        Ok(())
    }
}

pub(in crate::application::file_dag) fn validate_consumer_sources(
    prepared: &crate::application::file_dag::program::preparation::types::PreparedProgram,
    producer: crate::domain::coverage::types::RuleIndex,
    sources: &[FrontierSource],
    consumer: crate::domain::coverage::types::RuleIndex,
) -> Result<(), crate::domain::operating::errors::OperatingError> {
    if consumer == producer {
        return Ok(());
    }
    // A consumer other than the declared producer is admitted only by sources that name it.
    // An undeclared rule in range finds no source here and is refused like an unread input.
    prepared.frontiers(consumer)?;
    if sources.is_empty() {
        return Err(crate::domain::operating::errors::OperatingError::NotRead);
    }
    for binding in sources {
        let at = prepared
            .frontiers(consumer)?
            .iter()
            .position(|input| *input == binding.input)
            .ok_or(crate::domain::operating::errors::OperatingError::NotRead)?;
        let expected = prepared
            .program()
            .rules()
            .get(consumer.0)
            .and_then(|rule| rule.frontier.get(at))
            .ok_or(crate::domain::operating::errors::OperatingError::NotRead)?;
        if !matches!((&binding.root, expected), (Ok(actual), Ok(expected)) if actual == expected) {
            return Err(crate::domain::operating::errors::OperatingError::NotRead);
        }
    }
    Ok(())
}
