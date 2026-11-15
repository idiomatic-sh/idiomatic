//! The language that adds a tree for reading under its root, as a provider adds a macro call's arguments, and answers its semantic
//! projection with the readers the engine shares: the expanded pair or the native pair, as the case asks.

use crate::domain::answers::service::{
    expanded_semantic_child, expanded_semantic_syntax, native_semantic_child,
    native_semantic_syntax,
};
use crate::domain::answers::types::{Expansions, Reader, Support};
use crate::domain::builders::service::{enter_node, leave_node, token};
use crate::domain::completion::service::name_kind;
use crate::domain::elements::types::{Completion, TableBuilder, Tree};
use crate::domain::expansions::service::{adopt, mark_expansion, release};
use crate::inbound::use_cases::parse::errors::ParseError;
use crate::inbound::use_cases::parse::fixtures::small::{Seen, Small};
use crate::inbound::use_cases::parse::interface::ParseUseCase;
use crate::inbound::use_cases::parse::types::{ParseRequest, Recognition};
use ryo_relations::domain::elements::types::ExpansionId;
use ryo_relations::domain::relations::collections::relations::semantic_child::item::SemanticChild;
use ryo_relations::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntax;
use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
use ryo_relations::domain::terms::types::{NameText, UnknownName};

/// A language whose file `ab` is a root over two letters, with one element added under the root for reading; its semantic slots
/// answer with the expanded readers when it holds `Included`, and with the native readers when it holds `Ignored`.
pub(crate) struct Expanding(pub(crate) Expansions);

/// Answers every relation but the semantic pair as `Small` does.
macro_rules! forward_but_semantic {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest:tt )* }
        engine { $( $engine:tt )* }
    ) => {
        $(
            forward_but_semantic!(@one $module $variant);
        )*
    };
    (@one semantic_syntax SemanticSyntax) => {};
    (@one semantic_child SemanticChild) => {};
    (@one $module:ident $variant:ident) => {
        fn $module(
            &self,
        ) -> Support<Reader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, Seen>> {
            Small.$module()
        }
    };
}

impl ParseUseCase for Expanding {
    type FileModel = Seen;

    fn begin_semantics(
        &self,
        _request: &ParseRequest<'_>,
        _tree: &crate::domain::elements::types::Elements,
        _slot: &mut crate::domain::semantics::types::SemanticSlot,
    ) {
    }

    fn semantics(&self) -> Option<&dyn crate::domain::answers::types::Semantics<Seen>> {
        None
    }

    fn completion_requirement(&self) -> crate::domain::elements::types::CompletionRequirement {
        Small.completion_requirement()
    }

    fn extend_file_model(&self, _tree: &Tree<'_>, _from: Position, _model: &mut Seen) {}

    fn recognizes_explicit(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }

    fn capabilities(&self) -> &'static [ryo_relations::domain::catalog::types::OptionalCapability] {
        Small.capabilities()
    }

    fn configuration_identity(
        &self,
    ) -> crate::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_> {
        Small.configuration_identity()
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
        Count(4)
    }

    fn kind_names(&self) -> crate::inbound::use_cases::parse::types::KindNames {
        Small.kind_names()
    }

    /// The root (kind 1) over the letters `a` and `b` (kind 2), then, under the root and after both, the added element (kind 3) over
    /// `ab`, marked as the expansion of the root.
    fn read_tree(
        &self,
        _request: &ParseRequest<'_>,
        builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        enter_node(builder, KindBits(1), Offset(0));
        token(builder, KindBits(2), Offset(0), Offset(1));
        token(builder, KindBits(2), Offset(1), Offset(2));
        leave_node(builder, Offset(2));
        adopt(builder, Position(0));
        token(builder, KindBits(3), Offset(0), Offset(2));
        mark_expansion(builder, Position(3), Position(0), ExpansionId(0));
        release(builder);
        Ok(())
    }

    fn complete(&self, _request: &ParseRequest<'_>, completion: &mut Completion<'_>) {
        for (kind, name) in [(1, "root"), (2, "letter"), (3, "added")] {
            name_kind(completion, KindBits(kind), NameText(name));
        }
    }

    fn semantic_syntax(&self) -> Support<Reader<SemanticSyntax, Seen>> {
        match self.0 {
            Expansions::Included => expanded_semantic_syntax(),
            Expansions::Ignored => native_semantic_syntax(),
        }
    }

    fn semantic_child(&self) -> Support<Reader<SemanticChild, Seen>> {
        match self.0 {
            Expansions::Included => expanded_semantic_child(),
            Expansions::Ignored => native_semantic_child(),
        }
    }

    ryo_relations::each_relation!(@declared ryo_relations::baseline_relations, forward_but_semantic);
}

#[cfg(test)]
#[path = "expanding_test.rs"]
mod tests;
