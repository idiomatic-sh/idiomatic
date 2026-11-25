//! The `walk` contract: it implements the contract another crate declares. It says which files are html, has the syntax port walk a
//! file into the engine's numbered node table, and lends the engine the file over the language's fillers while the walk lives.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::{HtmlFile, HtmlWalk};
use crate::domain::language::service::reads;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{
    self, Count, FileFacts, FileVisit, NodeTableBuilder, RelationNeeds, Walk, WalkError, WalkRequest, fillers, node_table,
};
use std::sync::Arc;
use strum::EnumCount as _;

/// The ports the walk calls, which the edge injects.
pub(crate) struct WalkPorts {
    /// The parser that reads html text into the tree.
    pub(crate) syntax: Arc<dyn SyntaxPort>,
}

impl Walk for HtmlWalk<WalkPorts> {
    type Scratch = ();

    const KIND_COUNT: Count = Count(Node::COUNT + Token::COUNT + Extra::COUNT);

    fn recognizes(&self, path: languages::SourcePath<'_>, explicit: bool) -> languages::Recognition {
        // today's reading declines a path named explicitly and reads one by its extension
        if explicit { languages::Recognition::Declines } else { reads(path) }
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
        // the engine's own location reader covers the elements of embedded sources too
        relation == languages::Relation::SourceLocation
    }

    fn unknown_reasons(&self) -> &'static [languages::UnknownName] {
        &[]
    }

    fn walk(
        &self,
        request: WalkRequest<'_>,
        _needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        _scratch: &mut (),
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        if reads(request.path) != languages::Recognition::Reads {
            return Err(WalkError::Unsupported);
        }
        self.ports.syntax.walk(SourceText(request.source.0), request.path, builder);
        let table = node_table::finished(builder);
        let mut file = HtmlFile { table, source: request.source, file: request.file };
        let mut source = fillers::file_source::<HtmlWalk<WalkPorts>>(&mut file);
        visit.visit(FileFacts {
            table,
            source: request.source,
            file: request.file,
            path: request.path,
            facts: &mut source,
        });
        Ok(())
    }
}
