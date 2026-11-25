//! The `walk` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files are
//! go, has the syntax port walk a file into the engine's node table (a module's `go.mod` is read by the language itself), and lends the
//! engine the file over the language's fillers while the walk lives.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::{GoFile, GoScratch, GoWalk, Syntax};
use crate::domain::language::service::{file_of, reads};
use crate::domain::language::types::GoFile as Kind;
use crate::domain::modfile::service as modfile;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::numbered;
use crate::domain::syntax::types::Output;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{
    self, ByteSlice, Count, FileFacts, FileVisit, NodeTableBuilder, RelationNeeds, Walk, WalkError, WalkRequest, fillers,
    node_table,
};
use std::sync::Arc;
use strum::EnumCount as _;

/// The ports the walk calls, which the edge injects.
pub(crate) struct WalkPorts {
    /// The parser.
    pub(crate) syntax: Arc<dyn SyntaxPort>,
}

impl Walk for GoWalk<WalkPorts> {
    type Scratch = GoScratch;

    const KIND_COUNT: Count = Count(Node::COUNT + Token::COUNT + Extra::COUNT);

    fn walk(
        &self,
        request: WalkRequest<'_>,
        _needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        scratch: &mut GoScratch,
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        if reads(request.path) != languages::Recognition::Reads {
            return Err(WalkError::Unsupported);
        }
        match file_of(request.path) {
            Kind::Module => {
                let bytes = request.source;
                numbered(&mut scratch.roling, ByteSlice(bytes.0), builder, |out: &mut dyn Output| modfile::read(bytes, out));
            }
            Kind::Source | Kind::Other => {
                self.ports.syntax.walk(SourceText(request.source.0), &mut scratch.roling, builder);
            }
        }
        let table = node_table::finished(builder);
        let mut file = GoFile { syntax: Syntax { table, source: request.source, file: request.file } };
        let mut source = fillers::file_source::<GoWalk<WalkPorts>>(&mut file);
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
