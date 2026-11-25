//! The `walk` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files are
//! go, has the syntax port walk a file into the engine's node table (a module's `go.mod` is read by the language itself), and lends the
//! engine the file over the language's fillers while the walk lives.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::{GoFile, GoWalk, Syntax};
use crate::domain::language::service::{file_of, reads};
use crate::domain::language::types::GoFile as Kind;
use crate::domain::modfile::service as modfile;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Numbered;
use crate::domain::walked::roles::role_of;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{
    self, Count, FileFacts, FileVisit, NodeTableBuilder, RelationNeeds, Walk, WalkError, WalkRequest, fillers,
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
    type Scratch = ();

    const KIND_COUNT: Count = Count(Node::COUNT + Token::COUNT + Extra::COUNT);

    fn recognizes(&self, path: languages::SourcePath<'_>, explicit: bool) -> languages::Recognition {
        // A file the caller names is no more the language's than one a walk finds: go reads its sources and modules only.
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

    /// The engine's reader of an element table writes a row for every element, those of embedded trees too; Go has no embedded tree.
    fn includes_expansions(&self, relation: languages::Relation) -> bool {
        relation == languages::Relation::SourceLocation
    }

    fn unknown_reasons(&self) -> &'static [languages::UnknownName] {
        crate::domain::language::constants::UNKNOWN_REASONS
    }

    fn walk(
        &self,
        request: WalkRequest<'_>,
        _needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        _scratch: &mut (),
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        if self.recognizes(request.path, false) != languages::Recognition::Reads {
            return Err(WalkError::Unsupported);
        }
        let module = file_of(request.path) == Kind::Module;
        if module {
            modfile::read(request.source, &mut Numbered { builder });
        } else {
            self.ports.syntax.walk(SourceText(request.source.0), builder);
        }
        // A role is read from the siblings, those after the element too: it is given once the table is written.
        let source = request.source;
        let file = request.file;
        node_table::classify_written_fields(builder, |table, at| {
            languages::FieldCode(role_of(&Syntax { table, source, file }, at).0)
        });
        let table = node_table::finished(builder);
        let mut file = GoFile { syntax: Syntax { table, source: request.source, file: request.file }, module };
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
