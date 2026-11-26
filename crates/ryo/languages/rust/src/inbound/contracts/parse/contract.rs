//! The `parse` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which
//! files are Rust and how they are read: through the walk, lent to the engine as numbered files, the one way the engine reads the
//! language's files.

use crate::domain::fields::service::{field_name, fields};
use crate::domain::language::service::reads;
use crate::domain::syntax::constants::NO_CONFIGURATION;
use crate::domain::syntax::service::{kind_name, kinds};
use ryo::languages::UnknownName;
use std::sync::Arc;

use ryo::languages::{self, Parse, ParseRequest, Recognition};

/// `parse`, implementing `ryo::Parse`.
pub(crate) struct ParseContract {
    /// The walk, lent to the engine as numbered files.
    numbered: languages::NumberedWalk<crate::domain::file_view::types::RustWalk<crate::inbound::contracts::walk::contract::WalkPorts>>,
}

impl ParseContract {
    /// `parse` over the walk's parser, which the edge injects.
    #[must_use]
    pub(crate) fn new(syntax: Arc<dyn crate::outbound::syntax::interface::SyntaxWalk>) -> Self {
        Self {
            numbered: languages::NumberedWalk::new(crate::domain::file_view::types::RustWalk {
                ports: crate::inbound::contracts::walk::contract::WalkPorts { syntax },
            }),
        }
    }
}

impl Parse for ParseContract {
    fn file_relations(&self) -> &dyn languages::FileRelations {
        self
    }

    fn recognizes_explicit(&self, _: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        &[
            languages::OptionalCapability::ExpressionShape,
            languages::OptionalCapability::NumericLiteral,
            languages::OptionalCapability::AttributePath,
        ]
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        languages::ParseConfigurationIdentity(NO_CONFIGURATION)
    }

    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition {
        reads(request.path)
    }

    fn skips(&self) -> &'static [languages::SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn kind_count(&self) -> languages::Count {
        languages::Count(kinds().count())
    }

    fn source_words(&self) -> Option<languages::SourceWords> {
        Some(languages::SourceWords { identifier: |word| ra_ap_syntax::utils::is_identifier(word, ra_ap_parser::Edition::CURRENT), columns: &[] })
    }

    fn stable_fields(&self) -> languages::FieldNames {
        languages::FieldNames(fields().map(field_name).collect())
    }

    fn kind_names(&self) -> languages::KindNames {
        languages::KindNames(kinds().map(kind_name).collect())
    }
}

impl languages::FileRelations for ParseContract {
    fn numbered(&self) -> &dyn languages::NumberedVisit {
        &self.numbered
    }

    fn declarations(&self) -> &'static [languages::FileRelation] {
        &[]
    }

    fn read(
        &self,
        schemas: &[&languages::FileRelation],
        _: ParseRequest<'_>,
        _: &languages::Elements,
        _: &languages::NodeTable,
        _: &mut languages::Symbols,
        _: Option<&dyn languages::FileProject>,
    ) -> Vec<Result<Vec<languages::FactColumn>, languages::ParseError>> {
        schemas
            .iter()
            .map(|_| {
                Err(languages::ParseError::Library {
                    reason: languages::ToolMessage("the language declares no file relation".into()),
                })
            })
            .collect()
    }
}
