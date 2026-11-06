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
    pub(crate) input: ryo_values::domain::plan_parts::types::ObservationBindingId,
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
/// One acquisition of the frontier sources. The request owner hands the same acquisition to every
/// root of one population (see `FrontierCache`); a clone shares the rows and never copies them.
#[derive(Clone)]
pub struct PreparedFrontiers {
    pub(in crate::application::file_dag) producer: crate::domain::coverage::types::RuleIndex,
    pub(in crate::application::file_dag) sources: std::sync::Arc<[FrontierSource]>,
    pub(in crate::application::file_dag) rows: std::sync::Arc<[FrontierInput]>,
    pub(in crate::application::file_dag) program:
        std::sync::Arc<crate::domain::executable_program::types::ExecutableProgram>,
    pub(in crate::application::file_dag) required:
        crate::application::file_dag::program::admission::RequiredPartitions,
    pub(in crate::application::file_dag) refused:
        std::sync::Arc<[crate::domain::crossing::types::RefusedFrontier]>,
}

/// What makes two prepare calls one population: the same rule, first bucket, demand and request
/// scope, read from the same facts and project providers.
struct FrontierKey {
    rule: crate::domain::coverage::types::RuleIndex,
    first: ryo_values::domain::stored_facts::types::Bucket,
    ready: Option<Vec<bool>>,
    required: crate::application::file_dag::program::admission::RequiredPartitions,
    providers: (usize, usize),
}

/// One acquisition, kept only while some root still holds its rows: the rows are held weakly, so a
/// population whose roots have all finished is read again by the next caller and a provider freed
/// and reallocated at the same address can never be mistaken for the one that was read.
struct FrontierEntry {
    key: FrontierKey,
    rows: std::sync::Weak<[FrontierInput]>,
    shared: PreparedFrontiers,
}

/// Engine-owned dedup of prepared frontiers for one request: the first call for a population reads
/// the provider; every later call for it, while a root of that population is live, receives the same
/// shared acquisition. Failures are not kept.
#[derive(Default)]
pub(crate) struct FrontierCache(std::sync::Mutex<Vec<FrontierEntry>>);

impl FrontierCache {
    /// The only route to a provider read: the cache keys the population, then prepares it once.
    pub(crate) fn prepared(
        &self,
        program: &crate::application::file_dag::program::preparation::types::PreparedProgram,
        rule: crate::domain::coverage::types::RuleIndex,
        first: ryo_values::domain::stored_facts::types::Bucket,
        facts: &dyn crate::outbound::facts::interface::FactsFinish,
        project: &dyn crate::outbound::facts::interface::FactsWorker,
        project_symbols: &ryo_language::domain::symbols::types::Symbols,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        required: &crate::application::file_dag::program::admission::RequiredPartitions,
        policy: super::service::PreparationPolicy<'_>,
    ) -> Result<PreparedFrontiers, super::service::PreparationError> {
        let ready = match policy {
            super::service::PreparationPolicy::Diagnostic => None,
            super::service::PreparationPolicy::Observations { ready } => Some(ready),
        };
        let providers = (
            std::ptr::from_ref(facts).cast::<()>().addr(),
            std::ptr::from_ref(project).cast::<()>().addr(),
        );
        self.shared(rule, first, ready, required, providers, || {
            super::service::prepare(
                program,
                rule,
                first,
                facts,
                project,
                project_symbols,
                symbols,
                required,
                policy,
            )
        })
    }

    fn shared<E>(
        &self,
        rule: crate::domain::coverage::types::RuleIndex,
        first: ryo_values::domain::stored_facts::types::Bucket,
        ready: Option<&[bool]>,
        required: &crate::application::file_dag::program::admission::RequiredPartitions,
        providers: (usize, usize),
        acquire: impl FnOnce() -> Result<PreparedFrontiers, E>,
    ) -> Result<PreparedFrontiers, E> {
        let mut held = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        held.retain(|entry| entry.rows.strong_count() > 0);
        if let Some((rows, entry)) = held.iter().find_map(|entry| {
            let key = &entry.key;
            (key.rule == rule
                && key.first == first
                && key.ready.as_deref() == ready
                && key.providers == providers
                && key.required.same_request(required)
                    == crate::application::file_dag::program::admission::Agreement::Same)
                .then(|| entry.rows.upgrade().map(|rows| (rows, entry)))
                .flatten()
        }) {
            return Ok(PreparedFrontiers {
                rows,
                ..entry.shared.clone()
            });
        }
        let prepared = acquire()?;
        held.push(FrontierEntry {
            key: FrontierKey {
                rule,
                first,
                ready: ready.map(<[bool]>::to_vec),
                required: required.clone(),
                providers,
            },
            rows: std::sync::Arc::downgrade(&prepared.rows),
            shared: PreparedFrontiers {
                rows: std::sync::Arc::from([]),
                ..prepared.clone()
            },
        });
        Ok(prepared)
    }
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
        table: std::sync::Arc<ryo_values::domain::table_words::types::CompactTable>,
        spelling: Vec<ryo_relations::domain::relations::types::Name>,
        coverage: crate::domain::completion::types::CoverageState,
    },
    /// Read from the store and proven, not yet decoded: the first demand admits it.
    Packed(PackedFrontier),
}

/// The proven words of one stored frontier, decoded by `admit_all`.
#[derive(Clone)]
pub(crate) struct PackedFrontier {
    pub input: ryo_values::domain::plan_parts::types::Input,
    pub words: std::sync::Arc<ryo_values::domain::stored_facts::types::KeptWords>,
    pub mode: ryo_values::domain::table_words::types::DecodeMode,
    pub coverage: crate::domain::completion::types::CoverageState,
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
