//! What the `analysis` port asks of the outside, in the core's own vocabulary: whether a query is one ryo runs, one lint
//! call over a repository, and what ryo's catalog says of the relations, operators and grammar a query is written with.

use super::errors::AnalysisError;
use super::types::{AnalysisReport, AnalysisRequest, QueryVerdict};
use tastemaker_values::domain::catalog::types::{
    CatalogEntries, CatalogName, Grammar, OperatorAnswer, RelationAnswer,
};
use tastemaker_values::domain::definitions::types::QueryText;

/// What every `analysis` adapter implements.
pub trait AnalysisPort: Send + Sync {
    /// Whether `query` is correct RyoQL over ryo's relations and `tastemaker.placed`.
    ///
    /// # Errors
    ///
    /// `Engine` when the engine cannot say.
    fn validate(&self, query: &QueryText) -> Result<QueryVerdict, AnalysisError>;

    /// A lint call: every rule of the request over the files it reads.
    ///
    /// # Errors
    ///
    /// `Rule` when a rule cannot run, `Engine` when the engine fails.
    fn lint(&self, request: AnalysisRequest) -> Result<AnalysisReport, AnalysisError>;

    /// How many lint calls a caller may hold pending to send together through `lint_batch`: one means the calls go one at a time.
    fn batch_capacity(&self) -> usize {
        1
    }

    /// An ordered batch of lint calls: one answer for each request, in the order of the requests. The default answers them one by one;
    /// an adapter whose engine spreads a batch over its workers overrides it.
    fn lint_batch(&self, requests: Vec<AnalysisRequest>) -> Vec<Result<AnalysisReport, AnalysisError>> {
        requests.into_iter().map(|request| self.lint(request)).collect()
    }

    /// Every relation a query may read, a line each.
    ///
    /// # Errors
    ///
    /// `Engine` when the catalog cannot be read.
    fn relations(&self) -> Result<CatalogEntries, AnalysisError>;

    /// The relation of that name, with its columns and what each linked language answers for it, or `Unknown`.
    ///
    /// # Errors
    ///
    /// `Engine` when the catalog cannot be read.
    fn relation(&self, name: &CatalogName) -> Result<RelationAnswer, AnalysisError>;

    /// Every operator a query may call, a line each.
    ///
    /// # Errors
    ///
    /// `Engine` when the catalog cannot be read.
    fn operators(&self) -> Result<CatalogEntries, AnalysisError>;

    /// The operator of that name, with its fields and an example, or `Unknown`.
    ///
    /// # Errors
    ///
    /// `Engine` when the catalog cannot be read.
    fn operator(&self, name: &CatalogName) -> Result<OperatorAnswer, AnalysisError>;

    /// The grammar a query is written in, a production each.
    ///
    /// # Errors
    ///
    /// `Engine` when the grammar cannot be read.
    fn grammar(&self) -> Result<Grammar, AnalysisError>;
}
