//! The `parse` contract: it implements the contract another crate declares. It says which files are CSS, names its kinds, and hands the
//! engine the walk as the numbered visit the engine reads the language's files through.

use std::sync::Arc;

use ryo::languages::{self, Parse, ParseRequest, Recognition, UnknownName};

use crate::domain::file_view::types::CssWalk;
use crate::domain::language::constants::UNKNOWN_REASONS;
use crate::domain::language::service::reads;
use crate::domain::syntax::constants::NO_CONFIGURATION;
use crate::domain::syntax::service::{kind_name, kinds};
use crate::inbound::contracts::walk::contract::WalkPorts;
use crate::outbound::syntax::interface::SyntaxPort;

/// `parse`, implementing `ryo::Parse`.
pub(crate) struct ParseContract {
    /// The walk, lent to the engine as numbered files: the one way the engine reads the language's files.
    numbered: languages::NumberedWalk<CssWalk<WalkPorts>>,
}

impl ParseContract {
    /// `parse` over the ports it calls, which the edge injects.
    #[must_use]
    pub(crate) fn new(syntax: Arc<dyn SyntaxPort>) -> Self {
        Self { numbered: languages::NumberedWalk::new(CssWalk { ports: WalkPorts { syntax } }) }
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
        &[]
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        languages::ParseConfigurationIdentity(NO_CONFIGURATION)
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        UNKNOWN_REASONS
    }

    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition {
        reads(request.path)
    }

    fn skips(&self) -> &'static [languages::SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn kind_names(&self) -> languages::KindNames {
        languages::KindNames(kinds().map(kind_name).collect())
    }

    fn kind_count(&self) -> languages::Count {
        languages::Count(kinds().count())
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
