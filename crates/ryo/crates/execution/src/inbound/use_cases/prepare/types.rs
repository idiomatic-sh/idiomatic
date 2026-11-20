//! What `prepare` takes: the derived views it prepares over, and a request's plans, rules, digests, hosted relations
//! and observation identity.
use crate::application::rule_compilation::service::CompiledRules;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;

/// Where the plans of a request come from.
#[derive(Debug)]
pub enum PlanSource {
    /// Plans and their bindings as the caller authored them.
    Authored {
        /// The plans, one for each rule.
        plans: Vec<ryo_values::domain::plans::types::Plan>,
        /// The parameter bindings of each plan.
        bindings: Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>,
    },
    /// Rules the `compile` use case compiled.
    Compiled(CompiledRules),
}

/// What the edge composed for the build branch's `RYO_ENGINE` harness: the new engine, the digest its packs are keyed with, the
/// kinds of every linked language, and the language the request names when it names one.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::prepare::types::{HarnessLanguage, HarnessSetting};
/// use ryo_values::domain::descriptions::types::KindVocabularies;
/// use std::sync::Arc;
///
/// let setting = HarnessSetting {
///     engine: Arc::new(ryo_execution_runtime::outbound::engine::factory::use_case()),
///     digest: Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
///     vocabularies: KindVocabularies(Vec::new()),
///     language: HarnessLanguage::OnlyLinked,
/// };
/// assert_eq!(setting.language, HarnessLanguage::OnlyLinked);
/// ```
pub struct HarnessSetting {
    /// The new engine.
    pub engine: std::sync::Arc<dyn ryo_execution_runtime::outbound::engine::interface::EnginePort>,
    /// What a rule set's pack key is digested with.
    pub digest: std::sync::Arc<dyn ryo_execution_runtime::outbound::digest::interface::DigestPort>,
    /// The kinds of every linked language, in the order they were linked.
    pub vocabularies: ryo_values::domain::descriptions::types::KindVocabularies,
    /// Which of them the request's rules are lowered against.
    pub language: HarnessLanguage,
}

/// Which linked language's kinds the harness lowers a request's rules against.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::prepare::types::HarnessLanguage;
/// use ryo_values::domain::descriptions::types::LanguageName;
///
/// assert_ne!(HarnessLanguage::OnlyLinked, HarnessLanguage::Named(LanguageName::default()));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HarnessLanguage {
    /// The one language linked.
    OnlyLinked,
    /// The language the request says reads every file.
    Named(ryo_values::domain::descriptions::types::LanguageName),
}

impl std::fmt::Debug for HarnessSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// What a caller asks to have prepared.
#[derive(Debug)]
pub struct PrepareRequest {
    /// The plans of the request.
    pub plans: PlanSource,
    /// The rules the plans belong to.
    pub rules: Vec<ryo_values::domain::rule_requests::types::RuleSpec>,
    /// The digest of each rule.
    pub digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
    /// The relations the host provides.
    pub hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    /// The identity of the observations the request names.
    pub observations: ObservationIdentity,
}
