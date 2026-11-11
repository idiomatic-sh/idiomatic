//! One request's frontier cache: the first call for a population reads it, every later live call shares it.
use super::super::types::{
    FrontierInput, FrontierSource, InputReadiness, PreparedFrontiers, Providers,
};

/// What makes two prepare calls one population: the same rule, first bucket, demand and request
/// scope, read from the same facts and project providers.
struct FrontierKey {
    rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    first: ryo_values::domain::stored_facts::types::Bucket,
    ready: Option<Vec<InputReadiness>>,
    required: crate::application::file_dag::program::admission::types::RequiredPartitions,
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
    pub(in super::super) fn shared<E>(
        &self,
        rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
        first: ryo_values::domain::stored_facts::types::Bucket,
        ready: Option<&[InputReadiness]>,
        required: &crate::application::file_dag::program::admission::types::RequiredPartitions,
        providers: Providers,
        acquire: impl FnOnce() -> Result<PreparedFrontiers, E>,
    ) -> Result<PreparedFrontiers, E> {
        let mut held = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.retain(|entry| entry.rows.strong_count() > 0);
        if let Some((rows, entry)) = held.iter().find_map(|entry| {
            let key = &entry.key;
            (key.rule == rule
                && key.first == first
                && key.ready.as_deref() == ready
                && key.providers == providers
                && key.required.same_request(required)
                    == crate::application::file_dag::program::admission::types::Agreement::Same)
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

impl PreparedFrontiers {
    /// The rule whose frontiers these are.
    pub(crate) fn producer(&self) -> ryo_execution_domain::domain::coverage::types::RuleIndex {
        self.producer
    }
    /// The sources the frontier rows were read from, shared with every root of the population.
    pub(crate) fn sources(&self) -> &std::sync::Arc<[FrontierSource]> {
        &self.sources
    }
    /// The frontier rows, shared with every root of the population.
    pub(crate) fn rows(&self) -> &std::sync::Arc<[FrontierInput]> {
        &self.rows
    }
    /// The frontiers the preparation refused, each with its error.
    pub(crate) fn refused(
        &self,
    ) -> &[ryo_execution_domain::domain::frontiers::types::RefusedFrontier] {
        &self.refused
    }
    pub(crate) fn validate_scope(
        &self,
        program: &std::sync::Arc<ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram>,
        required: &crate::application::file_dag::program::admission::types::RequiredPartitions,
    ) -> Result<(), ryo_execution_domain::domain::operating::errors::OperatingError> {
        if !std::sync::Arc::ptr_eq(&self.program, program)
            || self.required.same_request(required)
                == crate::application::file_dag::program::admission::types::Agreement::Different
        {
            return Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead);
        }
        Ok(())
    }
}
