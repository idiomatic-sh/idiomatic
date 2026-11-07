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
    Current(crate::application::file_dag::program::observed_publication::types::ObservedPublication),
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
        std::sync::Arc<crate::domain::executable_program::types::executable_program::value::ExecutableProgram>,
    pub(in crate::application::file_dag) required:
        crate::application::file_dag::program::admission::RequiredPartitions,
    pub(in crate::application::file_dag) refused:
        std::sync::Arc<[crate::domain::frontiers::types::RefusedFrontier]>,
}

/// What makes two prepare calls one population: the same rule, first bucket, demand and request
/// scope, read from the same facts and project providers.
struct FrontierKey {
    rule: crate::domain::coverage::types::RuleIndex,
    first: ryo_values::domain::stored_facts::types::Bucket,
    ready: Option<Vec<InputReadiness>>,
    required: crate::application::file_dag::program::admission::RequiredPartitions,
    providers: Providers,
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
pub(crate) struct FrontierCache(std::sync::Mutex<Vec<FrontierEntry>>);

impl FrontierCache {
    /// The one cache of a request: `begin` issues it into the request and nothing else makes one.
    pub(crate) fn new() -> Self {
        Self(std::sync::Mutex::new(Vec::new()))
    }
    pub(super) fn shared<E>(
        &self,
        rule: crate::domain::coverage::types::RuleIndex,
        first: ryo_values::domain::stored_facts::types::Bucket,
        ready: Option<&[InputReadiness]>,
        required: &crate::application::file_dag::program::admission::RequiredPartitions,
        providers: Providers,
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
                ready: ready.map(<[InputReadiness]>::to_vec),
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

/// The proven words of one stored frontier, decoded by `admit_all` once per population.
#[derive(Clone)]
pub(crate) struct PackedFrontier {
    pub input: ryo_values::domain::plan_parts::types::Input,
    pub words: std::sync::Arc<ryo_values::domain::stored_facts::types::KeptWords>,
    pub mode: ryo_values::domain::table_words::types::DecodeMode,
    pub coverage: crate::domain::completion::types::CoverageState,
}

/// How many packed frontiers one publication scope decoded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DecodeCount(pub usize);

/// What a packed frontier decodes to: compact rows for a step, decoded rows for any other input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum DecodeShape {
    Compact,
    Decoded,
}

/// One decode of a publication scope: the words it read, how and under which symbol generation it
/// read them, and what it admitted. The words are the stored bytes, held so that a later root's
/// content-equal words find this decode; the admitted rows are already retained by the population.
pub(in crate::application::file_dag) struct DecodedFrontier {
    pub words: std::sync::Arc<ryo_values::domain::stored_facts::types::KeptWords>,
    pub shape: DecodeShape,
    pub mode: ryo_values::domain::table_words::types::DecodeMode,
    pub permit: ryo_language::domain::symbols::types::TransportPermit,
    pub admitted: Result<FrontierInput, crate::domain::frontiers::types::RefusedFrontier>,
}

/// The frontier decodes of one publication scope. Every rule of a cohort reads its own stored
/// bucket, but the buckets of one population hold the same words: the first root that admits them
/// decodes them, and every later root of the scope receives the same rows, relabelled with its own
/// input and coverage. Names are places in the symbol generation that decoded them, so a decode is
/// shared only within that generation.
pub(in crate::application::file_dag) struct FrontierDecodes {
    pub(super) decoded: std::cell::RefCell<Vec<DecodedFrontier>>,
    pub(super) count: std::cell::Cell<DecodeCount>,
}

impl FrontierDecodes {
    /// The decodes of one publication scope: `publication_owner` issues it into the scope.
    pub(in crate::application::file_dag) fn new() -> Self {
        Self {
            decoded: std::cell::RefCell::new(Vec::new()),
            count: std::cell::Cell::new(DecodeCount::default()),
        }
    }
    #[cfg(test)]
    /// How many packed frontiers this scope decoded.
    pub(in crate::application::file_dag) fn count(&self) -> DecodeCount {
        self.count.get()
    }
    /// Admit one packed frontier: words this scope already decoded in the same shape, mode and
    /// symbol generation are not decoded again; the earlier rows are relabelled with this root's
    /// input and coverage, and an earlier refusal with this root's input. Only a miss calls `decode`.
    pub(super) fn admit(
        &self,
        packed: PackedFrontier,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        decode: impl FnOnce(
            PackedFrontier,
            &mut ryo_language::domain::symbols::types::Symbols,
        ) -> Result<FrontierInput, crate::domain::frontiers::types::RefusedFrontier>,
    ) -> Result<FrontierInput, crate::domain::frontiers::types::RefusedFrontier> {
        use ryo_language::domain::symbols::{
            service::{same_transport_generation, transport_permit},
            types::Generation,
        };
        let permit = transport_permit(symbols);
        let shape = shape_of(packed.input);
        let mut decoded = self.decoded.borrow_mut();
        if let Some(found) = decoded.iter().find(|found| {
            found.shape == shape
                && found.mode == packed.mode
                && same_transport_generation(&found.permit, &permit) == Generation::Same
                && found.words == packed.words
        }) {
            return relabel(&found.admitted, packed.input, packed.coverage);
        }
        let words = std::sync::Arc::clone(&packed.words);
        let mode = packed.mode;
        let admitted = decode(packed, symbols);
        self.count.set(DecodeCount(self.count.get().0 + 1));
        decoded.push(DecodedFrontier {
            words,
            shape,
            mode,
            permit,
            admitted: admitted.clone(),
        });
        admitted
    }
}

/// What the words of a packed frontier of `input` decode to.
fn shape_of(input: ryo_values::domain::plan_parts::types::Input) -> DecodeShape {
    if matches!(input, ryo_values::domain::plan_parts::types::Input::Step(_)) {
        DecodeShape::Compact
    } else {
        DecodeShape::Decoded
    }
}

/// An earlier decode as this root admits it: the shared rows under this root's input and coverage.
fn relabel(
    admitted: &Result<FrontierInput, crate::domain::frontiers::types::RefusedFrontier>,
    input: ryo_values::domain::plan_parts::types::Input,
    coverage: crate::domain::completion::types::CoverageState,
) -> Result<FrontierInput, crate::domain::frontiers::types::RefusedFrontier> {
    match admitted.clone() {
        Ok(FrontierInput::Decoded { frontier, .. }) => Ok(FrontierInput::Decoded {
            frontier: crate::domain::table_model::types::Frontier {
                input,
                table: frontier.table,
            },
            coverage,
        }),
        Ok(FrontierInput::Compact {
            table, spelling, ..
        }) => Ok(FrontierInput::Compact {
            input,
            table,
            spelling,
            coverage,
        }),
        Ok(FrontierInput::Packed(packed)) => Ok(FrontierInput::Packed(PackedFrontier {
            input,
            coverage,
            ..packed
        })),
        Err(refused) => Err(crate::domain::frontiers::types::RefusedFrontier {
            input,
            error: refused.error,
        }),
    }
}

impl PreparedFrontiers {
    pub(in crate::application::file_dag) fn validate_scope(
        &self,
        program: &std::sync::Arc<crate::domain::executable_program::types::executable_program::value::ExecutableProgram>,
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

/// Whether the producer feeding one retained frontier is available to this phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputReadiness {
    /// The producer published; the frontier is read.
    Ready,
    /// The producer is unavailable; the frontier is refused.
    Unavailable,
}

/// The address of one provider, the identity of what a population was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct ProviderAddress(pub usize);

/// The two providers a population is read from: the facts provider and the project provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct Providers {
    pub facts: ProviderAddress,
    pub project: ProviderAddress,
}

/// Whether the files a source proof accounts for are exactly the files the request requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum FileAccounting {
    /// Every required file is accounted for, and no other.
    Exact,
    /// A required file is missing, or an unrequired one is named.
    Inexact,
}
