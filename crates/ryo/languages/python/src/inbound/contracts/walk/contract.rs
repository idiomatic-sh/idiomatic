//! The `walk` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files
//! are Python, has the syntax port walk a file into the engine's node table, and lends the engine the file over the language's
//! fillers while the parse lives.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::{PythonFile, PythonScratch, PythonWalk};
use crate::domain::language::service::reads;
use crate::domain::model::view::types::Built;
use crate::domain::node::types::SyntaxFile;
use crate::domain::nodes::types::Node;
use crate::domain::owner_facts::types::Readiness;
use crate::domain::tokens::types::Token;
use crate::inbound::contracts::parse::service::source_kind;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{ReportedSyntax, SourceText};
use crate::settings::PythonParserSettings;
use ryo::languages::{
    self, Count, FileFacts, FileVisit, NodeTableBuilder, RelationNeeds, Walk, WalkError,
    WalkRequest, fillers, node_table,
};
use std::sync::Arc;
use strum::EnumCount as _;

/// The ports the walk calls, which the edge injects.
pub(crate) struct WalkPorts {
    /// The settings the files are read under.
    pub(crate) settings: PythonParserSettings,
    /// The parser.
    pub(crate) syntax: Arc<dyn SyntaxPort>,
}

impl Walk for PythonWalk<WalkPorts> {
    type Scratch = PythonScratch;

    const KIND_COUNT: Count = Count(Node::COUNT + Token::COUNT + Extra::COUNT);

    fn walk(
        &self,
        request: WalkRequest<'_>,
        needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        scratch: &mut PythonScratch,
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        if reads(request.path) != languages::Recognition::Reads {
            return Err(WalkError::Unsupported);
        }
        let reported = if needs.0.contains(&languages::Relation::SyntaxError) {
            ReportedSyntax::Every
        } else {
            ReportedSyntax::Refusals
        };
        let target = self.ports.settings.target_version().components();
        scratch.owners.ready = Readiness::Pending;
        self.ports.syntax.walk(
            SourceText(request.source.0),
            source_kind(request.path),
            reported,
            builder,
            &mut |written, tree| {
                let table = node_table::finished(written);
                let mut file = PythonFile {
                    syntax: SyntaxFile {
                        table,
                        nodes: &tree.nodes,
                        text: tree.text,
                        file: request.file,
                        completeness: tree.completeness,
                        annotations: tree.annotations,
                    },
                    bytes: request.source,
                    target,
                    model: &mut scratch.model,
                    built: Built::default(),
                    owners: &mut scratch.owners,
                };
                let mut source = fillers::file_source::<Self>(&mut file);
                visit.visit(FileFacts {
                    table,
                    source: request.source,
                    facts: &mut source,
                });
            },
        );
        Ok(())
    }
}
