//! The language whose library fails.

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

/// A language whose library fails: on bytes that are not text it says so, on any other it names the failure.
pub(crate) struct Breaking;

/// Answers every relation as not in the language.
macro_rules! absent_all {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest:tt )* }
        engine { $( $engine:tt )* }
    ) => {
        $(
            fn $module(
                &self,
            ) -> Support<Reader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, Self::FileModel>> {
                absent(AbsenceCause::NoSuchConstruct)
            }
        )*
    };
}

impl ParseUseCase for Breaking {
    type FileModel = ();

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
        _model: &mut (),
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

    fn file_model(&self, _tree: &Tree<'_>, _model: &mut ()) {}

    fn recognizes(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Reads
    }

    fn skips(&self) -> &'static [crate::inbound::use_cases::parse::types::SkippedDirectory] {
        &[]
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn kind_count(&self) -> Count {
        Count(0)
    }

    fn kind_names(&self) -> crate::inbound::use_cases::parse::types::KindNames {
        crate::inbound::use_cases::parse::types::KindNames(Vec::new())
    }

    fn read_tree(
        &self,
        _request: &ParseRequest<'_>,
        _builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        Err(ParseError::Library {
            reason: ToolMessage("the library failed".into()),
        })
    }

    fn complete(&self, _request: &ParseRequest<'_>, _completion: &mut Completion<'_>) {}

    ryo_relations::each_relation!(absent_all);
}
