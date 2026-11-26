//! The `parse` contract: the language's implementation of `ryo::Parse`. It says which files are the language's, builds the file's
//! tree in the worker's element table through the syntax port, notes the modules the file imports when the run asks for its notes,
//! and answers, for every syntax relation of ryo's catalog, how the language reads it or that it is not in the language yet.

use std::sync::Arc;

use ryo::languages::{
    self, Count, KindNames, OptionalCapability, Parse, ParseConfigurationIdentity, ParseRequest, Recognition, SkippedDirectory,
    ByteSlice, Elements, ParseError, UnknownName,
};

use crate::domain::language::service::{reads, recognized};
use crate::domain::syntax::constants::NO_CONFIGURATION;
use crate::domain::syntax::service::{kind_name, kinds};
use crate::domain::syntax::types::{Stretch, Written};
use crate::outbound::embedded::interface::EmbeddedPort;
use crate::outbound::embedded::types::{BlockLanguage, Markup};
use crate::outbound::syntax::interface::SyntaxPort;

/// `parse`, implementing `ryo::Parse`.
pub(crate) struct ParseContract {
    /// The adapters that find TypeScript blocks in files of other formats, such as `.vue`.
    embedded: Vec<Arc<dyn EmbeddedPort>>,
    /// The walk, lent to the engine as numbered files: the one way the engine reads the language's files.
    numbered: languages::NumberedWalk<crate::domain::file_view::types::TypeScriptWalk<crate::inbound::contracts::walk::contract::WalkPorts>>,
}

impl ParseContract {
    /// `parse` over the ports it calls, which the edge injects.
    #[must_use]
    pub(crate) fn new(syntax: Arc<dyn SyntaxPort>, embedded: Vec<Arc<dyn EmbeddedPort>>) -> Self {
        Self {
            numbered: languages::NumberedWalk::new(crate::domain::file_view::types::TypeScriptWalk {
                ports: crate::inbound::contracts::walk::contract::WalkPorts { syntax, embedded: embedded.clone() },
            }),
            embedded,
        }
    }

    /// The blocks of typescript a file of another format carries, when an embedded adapter claims the file.
    fn stretches<'a>(&self, request: &ParseRequest<'a>) -> Option<Vec<Stretch<'a>>> {
        let adapter = self
            .embedded
            .iter()
            .find(|adapter| adapter.claims(request.path) == Recognition::Reads)?;
        let blocks = adapter.blocks(Markup(
            std::str::from_utf8(request.bytes.0).unwrap_or_default(),
        ));
        Some(
            blocks
                .0
                .iter()
                .map(|block| Stretch {
                    start: Count(block.start.0),
                    text: ByteSlice(block.text.0.as_bytes()),
                    written: match block.dialect {
                        BlockLanguage::Script => Written::Script,
                        BlockLanguage::TypeScript => Written::TypeScript,
                        BlockLanguage::Tsx => Written::Tsx,
                    },
                })
                .collect(),
        )
    }
}

impl Parse for ParseContract {
    fn file_relations(&self) -> &dyn languages::FileRelations {
        self
    }

    fn recognizes_explicit(&self, _: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }

    fn capabilities(&self) -> &'static [OptionalCapability] {
        &[]
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseConfigurationIdentity(NO_CONFIGURATION)
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        crate::domain::language::constants::UNKNOWN_REASONS
    }

    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition {
        // A file of another format is read when it carries a block of script, typed or plain.
        recognized(reads(request.path), self.stretches(request).as_deref())
    }

    fn skips(&self) -> &'static [SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn kind_count(&self) -> Count {
        Count(kinds().count())
    }

    fn source_words(&self) -> Option<languages::SourceWords> {
        Some(languages::SourceWords { identifier: oxc_syntax::identifier::is_identifier_name, columns: &[] })
    }

    fn stable_fields(&self) -> languages::FieldNames {
        use crate::domain::fields::service::{field_name, fields};
        languages::FieldNames(fields().map(field_name).collect())
    }

    fn kind_names(&self) -> KindNames {
        KindNames(kinds().map(kind_name).collect())
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
        _: &Elements,
        _: &languages::NodeTable,
        _: &mut languages::Symbols,
        _: Option<&dyn languages::FileProject>,
    ) -> Vec<Result<Vec<languages::FactColumn>, ParseError>> {
        schemas
            .iter()
            .map(|_| {
                Err(ParseError::Library {
                    reason: languages::ToolMessage("the language declares no file relation".into()),
                })
            })
            .collect()
    }
}
