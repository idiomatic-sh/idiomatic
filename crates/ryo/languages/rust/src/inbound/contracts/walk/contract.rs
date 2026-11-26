//! The `walk` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files
//! are Rust, has the syntax port walk a file into the engine's node table, and lends the engine the file over the language's
//! fillers while the parse lives.

use super::tape::{Tee, replay};
use crate::domain::fields::service::{field_bits, field_of};
use crate::domain::file_view::types::{RustFile, RustScratch, RustWalk};
use crate::domain::language::service::reads;
use crate::domain::nodes::types::Node;
use crate::domain::extras::types::Extra;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxWalk;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{
    self, ByteSlice, Count, FileFacts, FileVisit, NodeTableBuilder, Position, RelationNeeds, Tree, Walk, WalkError,
    WalkRequest, fillers, node_table,
};
use std::sync::Arc;
use strum::EnumCount as _;

/// The ports the walk calls, which the edge injects.
pub(crate) struct WalkPorts {
    /// The parser.
    pub(crate) syntax: Arc<dyn SyntaxWalk>,
}

impl Walk for RustWalk<WalkPorts> {
    type Scratch = RustScratch;

    const KIND_COUNT: Count = Count(Node::COUNT + Token::COUNT + Extra::COUNT);

    /// Today's contract declines a path the caller names explicitly: only the extension says a file is Rust.
    fn recognizes(&self, path: languages::SourcePath<'_>, explicit: bool) -> languages::Recognition {
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

    fn unknown_reasons(&self) -> &'static [languages::UnknownName] {
        &[]
    }

    fn source_words(&self) -> Option<languages::SourceWords> {
        Some(languages::SourceWords {
            identifier: |word| ra_ap_syntax::utils::is_identifier(word, ra_ap_parser::Edition::CURRENT),
            columns: &[],
        })
    }

    fn includes_expansions(&self, relation: languages::Relation) -> bool {
        use languages::Relation as R;
        matches!(
            relation,
            R::NumericLiteral | R::Reference | R::SemanticChild | R::SemanticSyntax | R::SourceLocation | R::StringLiteral
        )
    }

    fn walk(
        &self,
        request: WalkRequest<'_>,
        _needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        scratch: &mut RustScratch,
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        if !request.explicit && reads(request.path) != languages::Recognition::Reads {
            return Err(WalkError::Unsupported);
        }
        let bytes = request.source.0;
        scratch.tape.steps.clear();
        scratch.tape.texts.clear();
        scratch.elements = languages::Elements::default();
        scratch.symbols = languages::Symbols::default();
        {
            let mut elements = languages::table_builder(&mut scratch.elements, &mut scratch.symbols, ByteSlice(bytes));
            self.ports.syntax.walk(SourceText(bytes), &mut Tee { tape: &mut scratch.tape, elements: &mut elements });
        }
        let tree = Tree { elements: &scratch.elements, source: ByteSlice(bytes), file: request.file };
        replay(&scratch.tape, builder, |place| {
            let at = Position(u32::try_from(place).unwrap_or(u32::MAX));
            field_bits(field_of(&tree, at)).0
        });
        let table = node_table::finished(builder);
        crate::domain::model::service::clear(&mut scratch.model);
        let mut file = RustFile {
            table,
            elements: &mut scratch.elements,
            source: ByteSlice(bytes),
            file: request.file,
            symbols: &mut scratch.symbols,
            model: &mut scratch.model,
        };
        let mut source = fillers::file_source::<RustWalk<WalkPorts>>(&mut file);
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
