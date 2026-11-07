//! The language that reads `function` and `syntax_tree` and says the rest is not in it, with the model its worker keeps.

use crate::inbound::use_cases::parse::types::Recognition;
use ryo_relations::domain::terms::types::UnknownName;
use std::marker::PhantomData;
use std::sync::Mutex;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::answers::service::{absent, reads};
use crate::domain::answers::types::{Expansions, ReadItem, Reader, Reads, Support};
use crate::domain::builders::service::{enter_node, leave_node, table_builder, token};
use crate::domain::elements::service::end_of;
use crate::domain::elements::types::{Completion, Elements, TableBuilder, Tree};
use crate::domain::files::types::SourcePath;
use crate::domain::reasons::types::ToolMessage;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::parse::errors::ParseError;
use crate::inbound::use_cases::parse::fixtures::collect::Collect;
use crate::inbound::use_cases::parse::interface::ParseUseCase;
use crate::inbound::use_cases::parse::service::{absence, build, walk};
use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest, ParseRequest};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::function::item::{
    Function, FunctionRow,
};
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::{
    SyntaxTree, SyntaxTreeRow,
};
use ryo_relations::domain::relations::types::{
    Count, FileId, KindBits, NodeId, Offset, Position, Relation, Rows, Span,
};
use ryo_relations::domain::terms::types::Symbol;

/// A language that reads `function` from the tokens of kind 2 and `syntax_tree` from every element,
/// and says the rest is not in it.
pub(crate) struct Small;

/// What a worker of `Small` keeps between files: the sum of the ends of the trees it has read.
#[derive(Default)]
pub(crate) struct Seen(pub(crate) u32);

fn write_function(
    tree: &Tree<'_>,
    count: &Seen,
    _symbols: &mut Symbols,
    _at: Position,
    out: &mut dyn Rows<FunctionRow>,
) {
    out.write(FunctionRow {
        id: NodeId(u64::from(count.0)),
        name: Symbol(0),
        file: tree.file,
        span: Span {
            start: Offset(0),
            end: Offset(0),
        },
    });
}

fn write_tree(
    tree: &Tree<'_>,
    _model: &Seen,
    _symbols: &mut Symbols,
    at: Position,
    out: &mut dyn Rows<SyntaxTreeRow>,
) {
    out.write(SyntaxTreeRow {
        id: NodeId(u64::from(at.0)),
        kind: Symbol(0),
        text: Symbol(0),
        file: tree.file,
        span: Span {
            start: Offset(0),
            end: Offset(0),
        },
    });
}

/// Answers the relations `Small` does not read as not in the language.
macro_rules! absent_but_two {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest:tt )* }
        engine { $( $engine:tt )* }
    ) => {
        $(
            absent_but_two!(@one $module $variant);
        )*
    };
    (@one function Function) => {};
    (@one syntax_tree SyntaxTree) => {};
    (@one source_location SourceLocation) => {};
    (@one $module:ident $variant:ident) => {
        fn $module(
            &self,
        ) -> Support<Reader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, Self::FileModel>> {
            absent(AbsenceCause::NoSuchConstruct)
        }
    };
}

/// The item that reads `function` from the tokens of kind 2.
pub(crate) struct FunctionItem;

impl crate::domain::answers::types::declared::Syntax for FunctionItem {}

impl crate::domain::answers::types::Visiting<Function, Seen> for FunctionItem {
    fn visit(
        &self,
        tree: &Tree<'_>,
        model: &Seen,
        symbols: &mut Symbols,
        position: Position,
        out: &mut dyn Rows<FunctionRow>,
    ) {
        write_function(tree, model, symbols, position, out);
    }
}

impl ReadItem for FunctionItem {
    type Relation = Function;
    type Model = Seen;

    const READER: Reader<Function, Seen> = Reader {
        reads: Reads::Kinds(&[KindBits(2)]),
        visit: &FunctionItem,
        expansions: Expansions::Ignored,
        relation: PhantomData,
    };
}

/// The item that reads `syntax_tree` from every element, and finishes after the last.
pub(crate) struct TreeItem;

impl crate::domain::answers::types::declared::Syntax for TreeItem {}

impl crate::domain::answers::types::Visiting<SyntaxTree, Seen> for TreeItem {
    fn visit(
        &self,
        tree: &Tree<'_>,
        model: &Seen,
        symbols: &mut Symbols,
        position: Position,
        out: &mut dyn Rows<SyntaxTreeRow>,
    ) {
        write_tree(tree, model, symbols, position, out);
    }
}

impl ReadItem for TreeItem {
    type Relation = SyntaxTree;
    type Model = Seen;

    const READER: Reader<SyntaxTree, Seen> = Reader {
        reads: Reads::Every,
        visit: &TreeItem,
        expansions: Expansions::Ignored,
        relation: PhantomData,
    };
}

impl ParseUseCase for Small {
    type FileModel = Seen;

    fn begin_semantics(
        &self,
        _request: &crate::inbound::use_cases::parse::types::ParseRequest<'_>,
        _tree: &crate::domain::elements::types::Elements,
        _slot: &mut crate::domain::semantics::types::SemanticSlot,
    ) {
    }

    fn completion_requirement(&self) -> crate::domain::elements::types::CompletionRequirement {
        crate::domain::elements::types::CompletionRequirement::Independent
    }

    fn extend_file_model(
        &self,
        _tree: &crate::domain::elements::types::Tree<'_>,
        _from: ryo_relations::domain::relations::types::Position,
        _model: &mut Seen,
    ) {
    }

    fn recognizes_explicit(
        &self,
        _request: &crate::inbound::use_cases::parse::types::ParseRequest<'_>,
    ) -> crate::inbound::use_cases::parse::types::Recognition {
        crate::inbound::use_cases::parse::types::Recognition::Declines
    }

    fn capabilities(
        &self,
    ) -> &'static [ryo_relations::domain::capabilities::types::OptionalCapability] {
        &[]
    }

    fn configuration_identity(
        &self,
    ) -> crate::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_> {
        crate::inbound::use_cases::parse::types::ParseConfigurationIdentity("")
    }

    fn file_model(&self, tree: &Tree<'_>, model: &mut Seen) {
        // The model is the worker's, kept between files: what the last file left is still in it.
        model.0 += end_of(tree.elements).0;
    }

    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition {
        if request.path.ends_with(".small") {
            Recognition::Reads
        } else {
            Recognition::Declines
        }
    }

    fn skips(&self) -> &'static [crate::inbound::use_cases::parse::types::SkippedDirectory] {
        &[]
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn kind_count(&self) -> Count {
        Count(3)
    }

    fn kind_names(&self) -> crate::inbound::use_cases::parse::types::KindNames {
        crate::inbound::use_cases::parse::types::KindNames(Vec::new())
    }

    fn read_tree(
        &self,
        _request: &ParseRequest<'_>,
        _builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        Ok(())
    }

    fn complete(&self, _request: &ParseRequest<'_>, _completion: &mut Completion<'_>) {}

    fn function(&self) -> Support<Reader<Function, Seen>> {
        reads::<FunctionItem>()
    }

    fn syntax_tree(&self) -> Support<Reader<SyntaxTree, Seen>> {
        reads::<TreeItem>()
    }

    fn source_location(
        &self,
    ) -> Support<
        Reader<
            ryo_relations::domain::relations::collections::relations::source_location::item::SourceLocation,
            Seen,
        >,
    >{
        crate::domain::answers::service::source_locations()
    }

    ryo_relations::each_relation!(absent_but_two);
}
