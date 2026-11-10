//! The language whose advertised optional promises a row states.

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
use crate::inbound::use_cases::parse::fixtures::small::{Seen, Small};
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

/// An optional reader that emits one real row per visited occurrence.
pub(crate) struct IdentifierProbe<const FORBIDDEN: bool>;
impl<const FORBIDDEN: bool> crate::domain::answers::types::declared::Syntax
    for IdentifierProbe<FORBIDDEN>
{
}
impl<const FORBIDDEN: bool> ReadItem for IdentifierProbe<FORBIDDEN> {
    type Relation =
        ryo_relations::domain::relations::collections::relations::identifier::item::Identifier;
    type Model = Seen;
    const READER: Reader<Self::Relation, Seen> = Reader {
        reads: Reads::Every,
        visit: &Self,
        expansions: Expansions::Ignored,
        relation: PhantomData,
    };
}
impl<const FORBIDDEN: bool>
    crate::domain::answers::types::Visiting<
        ryo_relations::domain::relations::collections::relations::identifier::item::Identifier,
        Seen,
    > for IdentifierProbe<FORBIDDEN>
{
    fn visit(
        &self,
        tree: &Tree<'_>,
        _model: &Seen,
        symbols: &mut Symbols,
        position: Position,
        out: &mut dyn Rows<
            ryo_relations::domain::relations::collections::relations::identifier::item::IdentifierRow,
        >,
    ) {
        assert!(!FORBIDDEN, "an unadvertised reader must never run");
        out.write(
            ryo_relations::domain::relations::collections::relations::identifier::item::IdentifierRow {
                id: ryo_relations::domain::relations::service::node_id(tree.file, position),
                name: crate::domain::symbols::service::intern_derived(
                    symbols,
                    tree.source,
                    ryo_relations::domain::terms::types::NameText("name"),
                ),
                file: tree.file,
                span: Span {
                    start: Offset(0),
                    end: Offset(1),
                },
            },
        );
    }
}
/// Explicit advertised promises, independent of the reader implementations.
pub(crate) struct Promised(
    pub(crate) &'static [ryo_relations::domain::capabilities::types::OptionalCapability],
    pub(crate) bool,
);
macro_rules! forwarded_optional_method {
    (Identifier identifier) => {};
    ($variant:ident $method:ident) => {
        fn $method(
            &self,
        ) -> Support<
            Reader<
                ryo_relations::domain::relations::collections::relations::$method::item::$variant,
                Seen,
            >,
        > {
            Small.$method()
        }
    };
}
macro_rules! forwarded_optional_methods {
    (syntax { $( $variant:ident $method:ident $row:ident $column:ident ; )* }
     manifest { $( $manifest:tt )* } engine { $( $engine:tt )* }) => {
        $(forwarded_optional_method!($variant $method);)*
    };
}
impl ParseUseCase for Promised {
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
        self.0
    }

    fn configuration_identity(
        &self,
    ) -> crate::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_> {
        crate::inbound::use_cases::parse::types::ParseConfigurationIdentity("")
    }
    fn file_model(&self, tree: &Tree<'_>, model: &mut Seen) {
        Small.file_model(tree, model);
    }
    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition {
        Small.recognizes(request)
    }
    fn skips(&self) -> &'static [crate::inbound::use_cases::parse::types::SkippedDirectory] {
        Small.skips()
    }
    fn unknown_reasons(&self) -> &'static [UnknownName] {
        Small.unknown_reasons()
    }
    fn kind_count(&self) -> Count {
        Small.kind_count()
    }
    fn kind_names(&self) -> crate::inbound::use_cases::parse::types::KindNames {
        Small.kind_names()
    }
    fn read_tree(
        &self,
        request: &ParseRequest<'_>,
        builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        Small.read_tree(request, builder)
    }
    fn complete(&self, request: &ParseRequest<'_>, completion: &mut Completion<'_>) {
        Small.complete(request, completion);
    }
    fn identifier(
        &self,
    ) -> Support<
        Reader<
            ryo_relations::domain::relations::collections::relations::identifier::item::Identifier,
            Seen,
        >,
    > {
        if !self.1 {
            absent(AbsenceCause::NotYetRead)
        } else if self.0.is_empty() {
            reads::<IdentifierProbe<true>>()
        } else {
            reads::<IdentifierProbe<false>>()
        }
    }
    ryo_relations::each_relation!(forwarded_optional_methods);
}
