//! The `walk` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files
//! are Python, has the syntax port walk a file into the engine's node table, and lends the engine the file over the language's
//! fillers while the parse lives.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::{Analysed, PythonFile, PythonScratch, PythonWalk};
use crate::domain::language::service::reads;
use crate::domain::lexical_facts::types::QualifiedPath;
use crate::domain::model::lexical::types::TypedTree;
use crate::domain::model::view::types::Built;
use crate::domain::node::types::{SyntaxFile, Walked, WalkedTree};
use crate::domain::nodes::types::Node;
use crate::domain::string_annotations::service::{form_imports, type_strings};
use crate::domain::string_annotations::types::{FormImports, FormSource};
use crate::domain::tokens::types::Token;
use crate::inbound::contracts::parse::service::source_kind;
use crate::inbound::contracts::walk::fillers::fixed_names;
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
#[derive(Debug)]
pub(crate) struct PythonWalkScratch {
    /// The model's and the fillers' buffers.
    pub(crate) file: PythonScratch,
    /// The imports of the file in hand that can name a typing form.
    pub(crate) forms: FormImports,
    /// The qualified name in hand while the walk asks what a name is.
    pub(crate) path: QualifiedPath,
    /// The buffers the walk interns names and writes the lexical part into, handed to the walk and back from file to file.
    pub(crate) names: crate::domain::model::names::types::WalkNames,
}

impl Default for PythonWalkScratch {
    /// A worker's buffers, born with the fixed names the language states for the run: its fillers write each by its handle,
    /// the same word in every file, so no word is forgotten when a file begins.
    fn default() -> Self {
        let mut file = PythonScratch::default();
        file.model.words.get_mut().fixed.declared = fixed_names().0;
        Self {
            file,
            forms: FormImports::default(),
            path: QualifiedPath::default(),
            names: crate::domain::model::names::types::WalkNames::default(),
        }
    }
}

impl Walk for PythonWalk<WalkPorts> {
    type Scratch = PythonWalkScratch;

    const KIND_COUNT: Count = Count(Node::COUNT + Token::COUNT + Extra::COUNT);

    fn recognizes(&self, path: languages::SourcePath<'_>, explicit: bool) -> languages::Recognition {
        if explicit { languages::Recognition::Reads } else { reads(path) }
    }

    fn skips(&self) -> &'static [languages::SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        self.ports.settings.identity()
    }

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        crate::inbound::contracts::parse::constants::CAPABILITIES
    }

    fn unknown_reasons(&self) -> &'static [languages::UnknownName] {
        crate::domain::language::constants::UNKNOWN_REASONS
    }

    fn source_words(&self) -> Option<languages::SourceWords> {
        // Semantic identifiers are normalized and embedded expressions can be decoded.
        // A missing physical token is insufficient to disprove these fact columns.
        Some(languages::SourceWords { identifier: super::named::is_identifier, columns: &[] })
    }

    fn includes_expansions(&self, relation: languages::Relation) -> bool {
        use languages::Relation as R;
        matches!(
            relation,
            R::BraceFormatField
                | R::BraceFormatParse | R::BytesLiteral | R::CallArgument | R::ConstantKey | R::EmbeddedExpression
                | R::ExpressionShape | R::ExpressionType | R::FormatField | R::FormatParse | R::FormatSpecification
                | R::FunctionTerminal | R::LiteralExpressionParse | R::LiteralProperties | R::NumericLiteral
                | R::QualifiedName | R::RawFormatField | R::RawFormatSegment | R::SemanticChild | R::SemanticSyntax
                | R::SourceLocation | R::StringLiteral
        )
    }

    fn named_declarations(&self) -> &'static [languages::FileRelation] {
        super::named::NamedFile::named_schemas()
    }

    fn uses_project(&self) -> bool {
        true
    }

    fn walk(
        &self,
        request: WalkRequest<'_>,
        needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        scratch: &mut PythonWalkScratch,
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        if reads(request.path) != languages::Recognition::Reads || true {
            return Err(WalkError::Unsupported); // PROBE: the numbered walk reads no Python file
        }
        visit_numbered(
            self,
            request,
            needs,
            builder,
            scratch,
            &mut |table, tree, view, built| {
                let mut file = file_view(table, tree, request.file, view, built);
                let mut source = fillers::file_source::<PythonWalk<WalkPorts>>(&mut file);
                visit.visit(FileFacts {
                    table,
                    source: request.source,
                    file: request.file,
                    path: request.path,
                    facts: &mut source,
                });
            },
        );
        Ok(())
    }
}

/// The native syntax walk and the annotation expansion share this numbering.
#[expect(
    clippy::too_many_arguments,
    reason = "the visit borrows the caller's table and the provider's worker scratch"
)]
pub(crate) fn visit_numbered(
    provider: &PythonWalk<WalkPorts>,
    request: WalkRequest<'_>,
    needs: RelationNeeds<'_>,
    builder: &mut NodeTableBuilder<'_>,
    scratch: &mut PythonWalkScratch,
    visit: &mut dyn FnMut(&ryo::languages::NodeTable, &WalkedTree<'_>, &mut PythonScratch, Built),
) {
    let reported = if needs.0.contains(&languages::Relation::SyntaxError) {
        ReportedSyntax::Every
    } else {
        ReportedSyntax::Refusals
    };
    scratch.file.model.target = provider.ports.settings.target_version().components();
    let mut names = std::mem::take(&mut scratch.names);
    crate::domain::model::names::service::cleared(&mut names);
    let mut built = Built::default();
    let mut consume = |walked: Walked<'_, '_, '_>| match walked {
        Walked::Strings(tree) => type_hosts(scratch, &mut built, tree),
        Walked::Whole(written, tree) => {
            visit(
                node_table::finished(written),
                &tree,
                &mut scratch.file,
                built,
            );
            scratch.names = tree.names;
            Vec::new()
        }
    };
    let source = SourceText(request.source.0);
    let kind = source_kind(request.path);
    provider
        .ports
        .syntax
        .walk(source, kind, reported, builder, names, &mut consume);
}

/// The places of the strings the walk met since it last asked that the language reads as type expressions.
fn type_hosts(
    scratch: &mut PythonWalkScratch,
    built: &mut Built,
    tree: &WalkedTree<'_>,
) -> Vec<Position> {
    // The tree has grown since the model was last built for it, if it was: it is built again when a name is asked.
    *built = Built::default();
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
        declarations: tree.declarations,
        named: &tree.names,
    };
    form_imports(typed, &tree.imports, &mut scratch.forms);
    type_strings(
        &mut FormSource {
            tree: typed,
            imports: &scratch.forms,
            model: &mut scratch.file.model,
            built,
            path: &mut scratch.path,
        },
        &tree.strings,
        &mut hosts,
    );
    hosts
}

/// Builds the borrowed file view used by both catalog and column suppliers.
pub(crate) fn file_view<'a>(
    table: &'a languages::NodeTable,
    tree: &'a WalkedTree<'_>,
    file: languages::FileId,
    scratch: &'a mut PythonScratch,
    built: Built,
) -> PythonFile<'a> {
    let own = tree
        .segments
        .first()
        .map_or(tree.nodes.len(), |segment| usize::from(segment.first));
    let target = scratch.model.target;
    PythonFile {
        syntax: SyntaxFile {
            table,
            nodes: tree.nodes.get(..own).unwrap_or_default(),
            tree: TypedTree {
                segments: &tree.segments,
                parents: &tree.parents,
                ends: &tree.ends,
                nodes: &tree.nodes,
                annotations: tree.annotations,
                declarations: tree.declarations,
                named: &tree.names,
            },
            text: tree.text,
            file,
            completeness: tree.completeness,
        },
        target,
        import_presence: tree.import_presence,
        model: &mut scratch.model,
        built,
        analysed: Analysed::default(),
        buffers: &mut scratch.buffers,
    }
}
