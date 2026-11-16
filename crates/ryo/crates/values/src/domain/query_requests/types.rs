//! The types of the `query_requests` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The RyoQL text to compile or check, and the relations the host declares beside ryo's own.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryRequest {
    /// The query: text that entered the engine.
    pub text: crate::domain::query_tokens::types::QueryBundle,
    /// The relations the host declares, which a query names as it names ryo's: their rows come with the run.
    pub hosted: Vec<crate::domain::hosted::types::HostedRelation>,
    /// Resolved named producer contracts from the request graph's declaration pass.
    pub observations: Vec<crate::domain::observations::types::BoundObservationInput>,
    /// The text's modules, parsed from the same catalog by an earlier compile of the session: empty parses them with the root.
    pub modules: crate::domain::program::types::ParsedModules,
}
