//! The `walk` contract: it implements the contract another crate declares. It says which files are typescript, has the syntax port walk
//! a file into the engine's numbered node table, and lends the engine the file over the language's fillers while the walk lives.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::{TypeScriptFile, TypeScriptWalk};
use crate::domain::language::service::{reading_of, reads};
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::{PatternNode, Scripting, Stretch, Written};
use crate::domain::tokens::types::Token;
use crate::outbound::embedded::interface::EmbeddedPort;
use crate::outbound::embedded::types::{BlockLanguage, Markup};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{
    ByteSlice, Count, FileFacts, FileVisit, NodeTableBuilder, Recognition, Relation, RelationNeeds, Walk, WalkError, WalkRequest,
    fillers, node_table,
};
use crate::outbound::syntax::types::Wants;
use std::sync::Arc;
use strum::EnumCount as _;

/// The ports the walk calls, which the edge injects.
pub(crate) struct WalkPorts {
    /// The parser that reads TypeScript text into the tree.
    pub(crate) syntax: Arc<dyn SyntaxPort>,
    /// The adapters that find TypeScript blocks in files of other formats, such as `.vue`.
    pub(crate) embedded: Vec<Arc<dyn EmbeddedPort>>,
}

impl TypeScriptWalk<WalkPorts> {
    /// The blocks of typescript a file of another format carries, when an embedded adapter claims the file.
    fn stretches<'a>(&self, request: &WalkRequest<'a>) -> Option<Vec<Stretch<'a>>> {
        let adapter = self
            .ports
            .embedded
            .iter()
            .find(|adapter| adapter.claims(request.path) == Recognition::Reads)?;
        let blocks = adapter.blocks(Markup(std::str::from_utf8(request.source.0).unwrap_or_default()));
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

impl Walk for TypeScriptWalk<WalkPorts> {
    type Scratch = ();

    const KIND_COUNT: Count = Count(Node::COUNT + Token::COUNT + Extra::COUNT + PatternNode::COUNT);

    fn recognizes(&self, path: ryo::languages::SourcePath<'_>, explicit: bool) -> Recognition {
        // The language does not claim a file it was only explicitly asked for. A file of another format is read when an embedded
        // adapter claims it; whether it carries a block of script is found when the file is walked.
        if explicit {
            return Recognition::Declines;
        }
        if reads(path) == Recognition::Reads
            || self.ports.embedded.iter().any(|adapter| adapter.claims(path) == Recognition::Reads)
        {
            Recognition::Reads
        } else {
            Recognition::Declines
        }
    }

    fn skips(&self) -> &'static [ryo::languages::SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn configuration_identity(&self) -> ryo::languages::ParseConfigurationIdentity<'_> {
        ryo::languages::ParseConfigurationIdentity(crate::domain::syntax::constants::NO_CONFIGURATION)
    }

    fn capabilities(&self) -> &'static [ryo::languages::OptionalCapability] {
        &[]
    }

    fn unknown_reasons(&self) -> &'static [ryo::languages::UnknownName] {
        crate::domain::language::constants::UNKNOWN_REASONS
    }

    fn includes_expansions(&self, relation: Relation) -> bool {
        // The patterns a file carries are embedded segments of its table, and the relations over every element cover them.
        matches!(relation, Relation::SourceLocation | Relation::PatternSyntax | Relation::PatternChild)
    }

    fn source_words(&self) -> Option<ryo::languages::SourceWords> {
        Some(ryo::languages::SourceWords { identifier: oxc_syntax::identifier::is_identifier_name, columns: &[] })
    }

    fn walk(
        &self,
        request: WalkRequest<'_>,
        needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        _scratch: &mut (),
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        let stretches = self.stretches(&request);
        let recognition = crate::domain::language::service::recognized(reads(request.path), stretches.as_deref());
        if recognition != Recognition::Reads {
            return Err(WalkError::Unsupported);
        }
        let bytes = ByteSlice(request.source.0);
        let flow = [Relation::FlowBlock, Relation::FlowEdge, Relation::FlowMember, Relation::FlowStep]
            .iter()
            .any(|relation| needs.0.contains(relation));
        let wants = Wants { flow, patterns: [Relation::PatternSyntax, Relation::PatternChild, Relation::PatternParse].iter().any(|relation| needs.0.contains(relation)) };
        let walked = match reading_of(request.path, bytes, stretches.as_deref()) {
            Scripting::Own(dialect) => self.ports.syntax.walk(SourceText(bytes.0), dialect, builder, wants),
            Scripting::Script(script, dialect) => self.ports.syntax.walk(SourceText(&script.0), dialect, builder, wants),
        };
        let table = node_table::finished(builder);
        let mut file = TypeScriptFile { table, source: request.source, file: request.file, model: ReadIndexes::default(), flow: walked.flow, patterns: walked.patterns };
        let mut source = fillers::file_source::<TypeScriptWalk<WalkPorts>>(&mut file);
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

