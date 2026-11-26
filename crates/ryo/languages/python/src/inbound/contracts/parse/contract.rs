//! The `parse` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files
//! are Python and what the language promises; its files are read by the walk, lent as numbered files.

use super::constants::CAPABILITIES;
use crate::domain::language::constants::UNKNOWN_REASONS;
use crate::domain::language::service::reads;
use crate::domain::syntax::service::{kind_name, kinds};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::settings::PythonParserSettings;
use ryo::languages::{self, Parse, UnknownName};
use std::sync::Arc;

/// `parse`, implementing `ryo::languages::Parse`.
pub(crate) struct ParseContract {
    pub(super) settings: PythonParserSettings,
    /// The walk, lent to the engine as numbered files.
    pub(super) numbered: languages::NumberedWalk<crate::domain::file_view::types::PythonWalk<crate::inbound::contracts::walk::contract::WalkPorts>>,
}

impl ParseContract {
    /// `parse` over the ports it calls, which the edge injects.
    #[must_use]
    pub(crate) fn new(
        settings: PythonParserSettings,
        syntax: Arc<dyn SyntaxPort>,
    ) -> Self {
        Self {
            settings,
            numbered: languages::NumberedWalk::new(crate::domain::file_view::types::PythonWalk {
                ports: crate::inbound::contracts::walk::contract::WalkPorts { settings, syntax },
            }),
        }
    }
}

impl Parse for ParseContract {
    fn file_relations(&self) -> Option<&dyn languages::FileRelations> {
        Some(self)
    }

    fn source_words(&self) -> Option<ryo::languages::SourceWords> {
        // Semantic identifiers are normalized and embedded expressions can be decoded.
        // A missing physical token is insufficient to disprove these fact columns.
        Some(ryo::languages::SourceWords { identifier: crate::inbound::contracts::walk::named::is_identifier, columns: &[] })
    }

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        CAPABILITIES
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        self.settings.identity()
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        UNKNOWN_REASONS
    }

    fn recognizes(&self, request: &languages::ParseRequest<'_>) -> languages::Recognition {
        reads(request.path)
    }

    fn recognizes_explicit(&self, _: &languages::ParseRequest<'_>) -> languages::Recognition {
        languages::Recognition::Reads
    }

    fn skips(&self) -> &'static [languages::SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn kind_count(&self) -> languages::Count {
        languages::Count(kinds().count())
    }

    fn kind_names(&self) -> languages::KindNames {
        languages::KindNames(kinds().map(kind_name).collect())
    }
}

impl languages::FileRelations for ParseContract {
    fn numbered(&self) -> &dyn languages::NumberedVisit { &self.numbered }
    fn uses_project(&self) -> bool { true }
    fn declarations(&self) -> &'static [languages::FileRelation] {
        crate::inbound::contracts::walk::named::NamedFile::named_schemas()
    }

    fn read(&self, schemas: &[&languages::FileRelation], _: languages::ParseRequest<'_>,
        _: &languages::Elements, _: &languages::NodeTable, _: &mut languages::Symbols,
        _: Option<&dyn languages::FileProject>) -> Vec<Result<Vec<languages::FactColumn>, languages::ParseError>> {
        schemas.iter().map(|_| Err(languages::ParseError::Library {
            reason: languages::ToolMessage("these facts require the provider's numbered visit".into()),
        })).collect()
    }
}
