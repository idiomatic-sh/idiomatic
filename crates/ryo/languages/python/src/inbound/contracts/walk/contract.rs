//! The `walk` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files
//! are Python, has the syntax port walk a file into the engine's node table, and lends the engine the file over the language's
//! fillers while the parse lives.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::{PythonFile, PythonScratch, PythonWalk};
use crate::domain::language::service::reads;
use crate::domain::lexical_facts::types::QualifiedPath;
use crate::domain::model::lexical::types::TypedTree;
use crate::domain::model::view::types::Built;
use crate::domain::node::types::{SyntaxFile, Walked, WalkedTree};
use crate::domain::nodes::types::Node;
use crate::domain::standard_library::types::TargetVersion;
use crate::domain::string_annotations::service::{form_imports, type_strings};
use crate::domain::string_annotations::types::{FormImports, FormSource};
use crate::domain::tokens::types::Token;
use crate::inbound::contracts::parse::service::source_kind;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{ReportedSyntax, SourceText};
use crate::settings::PythonParserSettings;
use ryo::languages::{
    self, Count, FileFacts, FileVisit, NodeTableBuilder, Position, RelationNeeds, Walk, WalkError,
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

/// The buffers a worker's walk keeps from file to file: the file view's, and what it decides the type expressions through.
#[derive(Debug, Default)]
pub(crate) struct PythonWalkScratch {
    /// The model's and the fillers' buffers.
    pub(crate) file: PythonScratch,
    /// The imports of the file in hand that can name a typing form.
    pub(crate) forms: FormImports,
    /// The qualified name in hand while the walk asks what a name is.
    pub(crate) path: QualifiedPath,
}

impl Walk for PythonWalk<WalkPorts> {
    type Scratch = PythonWalkScratch;

    const KIND_COUNT: Count = Count(Node::COUNT + Token::COUNT + Extra::COUNT);

    fn walk(
        &self,
        request: WalkRequest<'_>,
        needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        scratch: &mut PythonWalkScratch,
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
        scratch.file.model.target = target;
        // The words of the last file are forgotten: so are the fixed words it interned.
        scratch.file.buffers.fixed.texts.clear();
        scratch.file.buffers.fixed.words.clear();
        let mut reading = Reading {
            request,
            target,
            scratch,
            built: Built::default(),
            visit,
        };
        self.ports.syntax.walk(
            SourceText(request.source.0),
            source_kind(request.path),
            reported,
            builder,
            &mut |walked| match walked {
                Walked::Strings(tree) => type_hosts(&mut reading, tree),
                Walked::Whole(written, tree) => {
                    lent(&mut reading, written, tree);
                    Vec::new()
                }
            },
        );
        Ok(())
    }
}

/// The reader of one file's walk: it says which strings are type expressions while the walk embeds them, then lends the engine
/// the walked file over the language's fillers.
struct Reading<'r, 'v> {
    /// The file asked for.
    request: WalkRequest<'r>,
    /// The language version the file is read under.
    target: TargetVersion,
    /// The worker's buffers.
    scratch: &'r mut PythonWalkScratch,
    /// The parts of the model built for the tree as the walk last showed it.
    built: Built,
    /// The engine's visit.
    visit: &'v mut dyn FileVisit,
}

/// The places of the strings the walk met since it last asked that the language reads as type expressions.
fn type_hosts(reading: &mut Reading<'_, '_>, tree: &WalkedTree<'_>) -> Vec<Position> {
    // The tree has grown since the model was last built for it, if it was: it is built again when a name is asked.
    reading.built = Built::default();
    let mut hosts = Vec::new();
    if tree.strings.is_empty() {
        return hosts;
    }
    let typed = TypedTree {
        segments: &tree.segments,
        parents: &tree.parents,
        ends: &tree.ends,
        nodes: &tree.nodes,
        annotations: tree.annotations,
    };
    form_imports(typed, &tree.imports, &mut reading.scratch.forms);
    type_strings(
        &mut FormSource {
            tree: typed,
            imports: &reading.scratch.forms,
            model: &mut reading.scratch.file.model,
            built: &mut reading.built,
            path: &mut reading.scratch.path,
        },
        &tree.strings,
        &mut hosts,
    );
    hosts
}

/// Lends the engine the walked file, over the language's fillers, while the parses live.
fn lent(reading: &mut Reading<'_, '_>, builder: &mut NodeTableBuilder<'_>, tree: WalkedTree<'_>) {
    let table = node_table::finished(builder);
    let own = tree
        .segments
        .first()
        .map_or(tree.nodes.len(), |segment| usize::from(segment.first));
    let mut file = PythonFile {
        syntax: SyntaxFile {
            table,
            nodes: tree.nodes.get(..own).unwrap_or_default(),
            tree: TypedTree {
                segments: &tree.segments,
                parents: &tree.parents,
                ends: &tree.ends,
                nodes: &tree.nodes,
                annotations: tree.annotations,
            },
            text: tree.text,
            file: reading.request.file,
            completeness: tree.completeness,
        },
        target: reading.target,
        model: &mut reading.scratch.file.model,
        built: reading.built,
        buffers: &mut reading.scratch.file.buffers,
    };
    let mut source = fillers::file_source::<PythonWalk<WalkPorts>>(&mut file);
    reading.visit.visit(FileFacts {
        table,
        source: reading.request.source,
        file: reading.request.file,
        path: reading.request.path,
        facts: &mut source,
    });
}
