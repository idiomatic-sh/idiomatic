//! The analysis port, implemented with ryo's SDK: `ryo::query::validate` and `ryo::linter::lint_with` over one engine, and the catalog
//! `ryo::relation::{list, describe}`, `ryo::operator::{list, describe}` and `ryo::query::describe`.

use ryo::linter::HostKind;
use ryo::linter::{HostDeclaration, Languages};
use ryo::query::{RyoQlQuery, ValidateRequest};

use super::constants::{
    BLANK_QUERY, FILE_KIND, OPERATORS_LISTED, PLACED, PLACED_SUMMARY, RELATIONS_LISTED, TEXT_KIND,
    UNKNOWN_OPERATION,
};
use super::schemas::{self, error, operator_page, placed_columns, relation_page, report};
use crate::domain::catalog::types::{
    CatalogColumn, CatalogEntries, CatalogEntry, CatalogName, CatalogText, Grammar, OperatorAnswer,
    Production, RelationAnswer, RelationPage,
};
use crate::domain::definitions::types::QueryText;
use crate::outbound::analysis::errors::AnalysisError;
use crate::outbound::analysis::interface::AnalysisPort;
use crate::outbound::analysis::types::{
    AnalysisReport, AnalysisRequest, AnalysisSettings, InvalidReason, Memory, QueryVerdict,
};

/// The ryo analysis adapter, over the languages the edge linked and the settings it set.
#[derive(Debug)]
pub(crate) struct AnalysisRyoAdapter {
    /// The languages the edge linked, which every call runs over.
    languages: Languages,
    /// The pool ryo reads with and its memory.
    settings: AnalysisSettings,
    /// Where ryo keeps what a lint call leaves for the next, or nowhere.
    cache: ryo::linter::Cache,
    /// ryo's engine over the languages, built by the first lint call and shared by every other: the plans it compiled
    /// and the globs it made serve every project linted after; or why it could not be built.
    // ignore: idiomatic.arch/idiomatic-rust/bounded-engine-state - the one engine, built once: its plan cache is bounded in ryo (`CompiledPlans`), and this holds no state of a call
    linter: std::sync::OnceLock<Result<ryo::linter::Linter, AnalysisError>>,
}

impl AnalysisRyoAdapter {
    /// The adapter over `languages`, every call run under `settings` and keeping its work in `cache`.
    #[must_use]
    pub(crate) fn new(
        languages: Languages,
        settings: AnalysisSettings,
        cache: ryo::linter::Cache,
    ) -> Self {
        Self {
            languages,
            settings,
            cache,
            linter: std::sync::OnceLock::new(),
        }
    }

    /// The adapter over `languages` and `settings`, keeping its work where `store` says.
    #[must_use]
    pub(crate) fn kept(
        languages: Languages,
        settings: AnalysisSettings,
        store: &crate::domain::settings::types::Store,
    ) -> Self {
        Self::new(languages, settings, schemas::cache(store))
    }
}

impl AnalysisPort for AnalysisRyoAdapter {
    fn validate(&self, query: &QueryText) -> Result<QueryVerdict, AnalysisError> {
        let Ok(text) = RyoQlQuery::try_from(&*query.0) else {
            return Ok(QueryVerdict::Invalid(InvalidReason(BLANK_QUERY.into())));
        };
        let (name, columns) = placed_columns();
        match ryo::query::validate(
            &self.languages,
            ValidateRequest {
                query: text,
                hosted: vec![HostDeclaration { name, columns }],
            },
        ) {
            Ok(_) => Ok(QueryVerdict::Valid),
            Err(ryo::query::ValidateError::Catalog { reason }) => {
                Err(AnalysisError::Engine { reason })
            }
            Err(refusal @ ryo::query::ValidateError::UnknownRelation { .. }) => {
                Ok(QueryVerdict::Invalid(InvalidReason(
                    format!("{refusal}; {RELATIONS_LISTED}").into(),
                )))
            }
            Err(refusal) => {
                let said = refusal.to_string();
                Ok(QueryVerdict::Invalid(InvalidReason(
                    if said.starts_with(UNKNOWN_OPERATION) {
                        format!("{said}; {OPERATORS_LISTED}")
                    } else {
                        said
                    }
                    .into(),
                )))
            }
        }
    }

    fn lint(&self, request: AnalysisRequest) -> Result<AnalysisReport, AnalysisError> {
        let sdk = schemas::request(request)?;
        let linter = self
            .linter
            .get_or_init(|| {
                // ryo's engine reads files on the invocation's one pool, the scheduler's own.
                let pool = self.settings.pool.0.as_ref().map_err(engine)?;
                Ok(ryo::linter::Linter::new(
                    self.languages.clone(),
                    ryo::linter::WorkerPool::from(std::sync::Arc::clone(pool)),
                    match self.settings.memory {
                        Memory::Machine => ryo::linter::MemoryBudget::Machine,
                        Memory::Bytes(bytes) => std::num::NonZero::new(bytes.0).map_or(
                            ryo::linter::MemoryBudget::Machine,
                            ryo::linter::MemoryBudget::Bytes,
                        ),
                    },
                    self.cache.clone(),
                ))
            })
            .as_ref()
            .map_err(Clone::clone)?;
        ryo::linter::lint_with(linter, sdk)
            .map(report)
            .map_err(error)
    }

    fn relations(&self) -> Result<CatalogEntries, AnalysisError> {
        let listed = ryo::relation::list().map_err(engine)?;
        Ok(CatalogEntries(
            listed
                .relations
                .into_iter()
                .map(|relation| CatalogEntry {
                    name: CatalogName(relation.name.0),
                    summary: CatalogText(relation.summary.0),
                })
                .chain(std::iter::once(placed_page().entry))
                .collect(),
        ))
    }

    fn relation(&self, name: &CatalogName) -> Result<RelationAnswer, AnalysisError> {
        if &*name.0 == PLACED {
            return Ok(RelationAnswer::Found(placed_page()));
        }
        let described = ryo::relation::describe(
            self.languages.clone(),
            ryo::relation::DescribeRequest {
                name: ryo::query::RelationName(name.0.clone()),
            },
        );
        if let Err(ryo::relation::DescribeError::UnknownRelation { .. }) = described {
            return Ok(RelationAnswer::Unknown);
        }
        described
            .map(|report| RelationAnswer::Found(relation_page(report.relation)))
            .map_err(engine)
    }

    fn operators(&self) -> Result<CatalogEntries, AnalysisError> {
        let listed = ryo::operator::list().map_err(engine)?;
        Ok(CatalogEntries(
            listed
                .operators
                .into_iter()
                .map(|operator| CatalogEntry {
                    name: CatalogName(operator.name.0),
                    summary: CatalogText(operator.summary.0),
                })
                .collect(),
        ))
    }

    fn operator(&self, name: &CatalogName) -> Result<OperatorAnswer, AnalysisError> {
        let described = ryo::operator::describe(ryo::operator::DescribeRequest {
            name: ryo::query::OperatorName(name.0.clone()),
        });
        if let Err(ryo::operator::DescribeError::UnknownOperator { .. }) = described {
            return Ok(OperatorAnswer::Unknown);
        }
        described
            .map(|report| OperatorAnswer::Found(operator_page(report.operator)))
            .map_err(engine)
    }

    fn grammar(&self) -> Result<Grammar, AnalysisError> {
        let described = ryo::query::describe().map_err(engine)?;
        Ok(Grammar(
            described
                .productions
                .into_iter()
                .map(|production| Production {
                    name: CatalogName(production.name.0),
                    form: CatalogText(production.form.0),
                    example: CatalogText(production.example.0),
                })
                .collect(),
        ))
    }
}

/// The port's failure for a catalog call ryo could not answer.
fn engine(reason: impl std::fmt::Display) -> AnalysisError {
    AnalysisError::Engine {
        reason: reason.to_string(),
    }
}

/// The catalog's page for `tastemaker.placed`: its columns, and no language, since Tastemaker supplies it.
fn placed_page() -> RelationPage {
    let entry = CatalogEntry {
        name: CatalogName(PLACED.into()),
        summary: CatalogText(PLACED_SUMMARY.into()),
    };
    let columns = placed_columns().1.into_iter();
    let columns = columns.map(|column| CatalogColumn {
        kind: CatalogName(
            if column.kind == HostKind::File {
                FILE_KIND
            } else {
                TEXT_KIND
            }
            .into(),
        ),
        name: CatalogName(column.name.0),
        words: Vec::new(),
    });
    RelationPage {
        entry,
        columns: columns.collect(),
        languages: Vec::new(),
    }
}
