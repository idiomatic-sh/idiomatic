//! Whether this phase owns a coordinator-reserved publication arena.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PublicationSlot {
    /// No named roots are demanded from this phase.
    Unused,
    /// An ordinary, unique index reserved by the existing request owner.
    Reserved(ryo_relations::domain::relations::types::WorkerNumber),
}

/// Each named member carries its own admission source, never just decoded words.
#[derive(Clone)]
pub(crate) enum BoundMember {
    Current(
        crate::application::file_dag::program::observed_publication::service::ObservedPublication,
    ),
    Restored(crate::application::file_dag::program::restored::types::ValidatedFrontier),
}

/// Application-only named binding. The producer schema remains pure metadata;
/// the member list retains the matching owner evidence in the same payload.
#[derive(Clone)]
pub(crate) struct BoundInput {
    pub(crate) input: ryo_values::domain::plan_parts::types::ObservationBindingId,
    pub(crate) members:
        Result<Vec<BoundMember>, ryo_execution_domain::domain::operating::errors::OperatingError>,
}

/// Admission preserves independent input successes and explicit refusal causes.
#[derive(Clone)]
pub(crate) struct FrontierSource {
    pub input: ryo_values::domain::plan_parts::types::Input,
    pub root: Result<
        ryo_execution_domain::domain::interpreted_program::types::InputSource,
        ryo_execution_domain::domain::operating::errors::OperatingError,
    >,
}
/// One acquisition of the frontier sources. The request owner hands the same acquisition to every
/// root of one population (see `FrontierCache`); a clone shares the rows and never copies them.
#[derive(Clone)]
pub(crate) struct PreparedFrontiers {
    pub(in crate::application::file_dag::frontiers) producer: ryo_execution_domain::domain::coverage::types::RuleIndex,
    pub(in crate::application::file_dag::frontiers) sources: std::sync::Arc<[FrontierSource]>,
    pub(in crate::application::file_dag::frontiers) rows: std::sync::Arc<[FrontierInput]>,
    pub(in crate::application::file_dag::frontiers) program:
        std::sync::Arc<ryo_execution_domain::domain::interpreted_program::types::interpreted_program::value::InterpretedProgram>,
    pub(in crate::application::file_dag::frontiers) required:
        crate::application::file_dag::program::admission::types::RequiredPartitions,
    pub(in crate::application::file_dag::frontiers) refused:
        std::sync::Arc<[ryo_execution_domain::domain::frontiers::types::RefusedFrontier]>,
}

/// Admitted phase inputs. Compact sources have schemas/name mappings but no
/// decoded cell allocation; the same graph owner materializes them on demand.
#[derive(Clone)]
pub(crate) enum FrontierInput {
    Decoded {
        frontier: ryo_execution_domain::domain::table_model::types::Frontier,
        coverage: ryo_execution_domain::domain::completion::types::CoverageState,
    },
    Compact {
        input: ryo_values::domain::plan_parts::types::Input,
        table: std::sync::Arc<ryo_values::domain::table_words::types::CompactTable>,
        /// Shared by every root the decode is relabelled for: one copy per decode, not per root.
        spelling: std::sync::Arc<[ryo_relations::domain::relations::types::Name]>,
        coverage: ryo_execution_domain::domain::completion::types::CoverageState,
    },
    /// Read from the store and proven, not yet decoded: the first demand admits it.
    Packed(PackedFrontier),
}

/// The proven words of one stored frontier, decoded by `admit_all` once per population.
#[derive(Clone)]
pub(crate) struct PackedFrontier {
    pub input: ryo_values::domain::plan_parts::types::Input,
    pub words: std::sync::Arc<ryo_values::domain::stored_facts::types::KeptWords>,
    pub mode: ryo_values::domain::table_words::types::DecodeMode,
    pub coverage: ryo_execution_domain::domain::completion::types::CoverageState,
}

/// How many packed frontiers one publication scope decoded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DecodeCount(pub usize);

/// What a packed frontier decodes to: compact rows for a step, decoded rows for any other input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DecodeShape {
    Compact,
    Decoded,
}

/// One decode of a publication scope: the digest of the words it read, how and under which symbol
/// generation it read them, and what it admitted. A later root's content-equal words find this decode
/// by their digest, so the packed words are released once decoded; the admitted rows are what the
/// population retains.
pub(crate) struct DecodedFrontier {
    pub content: ryo_values::domain::stored_facts::types::Digest,
    pub shape: DecodeShape,
    pub mode: ryo_values::domain::table_words::types::DecodeMode,
    pub permit: ryo_language::domain::symbols::types::TransportPermit,
    pub admitted:
        Result<FrontierInput, ryo_execution_domain::domain::frontiers::types::RefusedFrontier>,
}

pub(crate) fn validate_consumer_sources(
    prepared: &crate::application::file_dag::program::preparation::types::PreparedProgram,
    producer: ryo_execution_domain::domain::coverage::types::RuleIndex,
    sources: &[FrontierSource],
    consumer: ryo_execution_domain::domain::coverage::types::RuleIndex,
) -> Result<(), ryo_execution_domain::domain::operating::errors::OperatingError> {
    if consumer == producer {
        return Ok(());
    }
    // A consumer other than the declared producer is admitted only by sources that name it.
    // An undeclared rule in range finds no source here and is refused like an unread input.
    prepared.frontiers(consumer)?;
    if sources.is_empty() {
        return Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead);
    }
    for binding in sources {
        let at = prepared
            .frontiers(consumer)?
            .iter()
            .position(|input| *input == binding.input)
            .ok_or(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)?;
        let expected = prepared
            .program()
            .rules()
            .get(consumer.0)
            .and_then(|rule| rule.frontier.get(at))
            .ok_or(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)?;
        if !matches!((&binding.root, expected), (Ok(actual), Ok(expected)) if actual == expected) {
            return Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead);
        }
    }
    Ok(())
}

/// Whether the producer feeding one retained frontier is available to this phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputReadiness {
    /// The producer published; the frontier is read.
    Ready,
    /// The producer is unavailable; the frontier is refused.
    Unavailable,
}

/// The readiness of each retained frontier of one rule, in frontier order, as the observation stage's publications prove it: only
/// the stage mints one, so no caller chooses which producers count as published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadinessProof(pub(crate) Vec<InputReadiness>);

/// The address of one provider, the identity of what a population was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProviderAddress(pub usize);

/// The two providers a population is read from: the facts provider and the project provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Providers {
    pub facts: ProviderAddress,
    pub project: ProviderAddress,
}

/// Whether the files a source proof accounts for are exactly the files the request requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileAccounting {
    /// Every required file is accounted for, and no other.
    Exact,
    /// A required file is missing, or an unrequired one is named.
    Inexact,
}
