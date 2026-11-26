//! The `walk` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files
//! are Markdown, has the syntax port walk a file into the engine's node table, and lends the engine the file over the language's
//! fillers while the parse lives.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::{Modelled, Syntax, MarkdownFile, MarkdownScratch, MarkdownWalk};
use crate::domain::language::service::reads;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{
    self, Count, FileFacts, FileVisit, NodeTableBuilder, RelationNeeds, Walk, WalkError,
    WalkRequest, fillers, node_table,
};
use std::sync::Arc;
use strum::EnumCount as _;

/// The ports the walk calls, which the edge injects.
pub(crate) struct WalkPorts {
    /// The parser.
    pub(crate) syntax: Arc<dyn SyntaxPort>,
}

impl Walk for MarkdownWalk<WalkPorts> {
    type Scratch = MarkdownScratch;

    const KIND_COUNT: Count = Count(Node::COUNT + Token::COUNT + Extra::COUNT);

    fn walk(
        &self,
        request: WalkRequest<'_>,
        _needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        scratch: &mut MarkdownScratch,
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        if !request.explicit && reads(request.path) != languages::Recognition::Reads {
            return Err(WalkError::Unsupported);
        }
        scratch.decodings.places.clear();
        scratch.decodings.text.clear();
        self.ports
            .syntax
            .walk(SourceText(request.source.0), request.path, &mut scratch.decodings, builder);
        let table = node_table::finished(builder);
        let mut file = MarkdownFile {
            syntax: Syntax {
                table,
                text: request.source.0,
                file: request.file,
                decodings: &scratch.decodings,
            },
            sections: &mut scratch.sections,
            text: &mut scratch.text,
            modelled: Modelled::Waiting,
        };
        let mut source = fillers::file_source::<MarkdownWalk<WalkPorts>>(&mut file);
        visit.visit(FileFacts {
            table,
            source: request.source,
            file: request.file,
            path: request.path,
            facts: &mut source,
        });
        Ok(())
    }

    fn recognizes(&self, path: languages::SourcePath<'_>, _explicit: bool) -> languages::Recognition {
        reads(path)
    }

    fn skips(&self) -> &'static [languages::SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        languages::ParseConfigurationIdentity(crate::domain::syntax::constants::NO_CONFIGURATION)
    }

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        &[]
    }

    fn includes_expansions(&self, relation: languages::Relation) -> bool {
        // The engine's reader of an element table covers every element, those a parser embeds too: it answers source-location.
        relation == languages::Relation::SourceLocation
    }

    fn unknown_reasons(&self) -> &'static [languages::UnknownName] {
        &[]
    }
}
