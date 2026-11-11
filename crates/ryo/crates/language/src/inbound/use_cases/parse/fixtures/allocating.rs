//! A language double for the allocation row: its library parse allocates a scratch buffer in every file, and its one reader,
//! of `function`, allocates in every element or in none.

use std::marker::PhantomData;

use crate::domain::answers::service::{absent, reads};
use crate::domain::answers::types::{ReadItem, Reader, Reads, Support};
use crate::domain::builders::service::{enter_node, leave_node, token};
use crate::domain::elements::types::{Completion, TableBuilder, Tree};
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::parse::errors::ParseError;
use crate::inbound::use_cases::parse::interface::ParseUseCase;
use crate::inbound::use_cases::parse::types::{ParseRequest, Recognition};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::function::item::{
    Function, FunctionRow,
};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position, Rows};
use ryo_relations::domain::terms::types::UnknownName;

/// Answers every relation but `function` as not in the language.
macro_rules! all_but_function {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest:tt )* }
        engine { $( $engine:tt )* }
    ) => {
        $( all_but_function!(@one $module $variant); )*
    };
    (@one function Function) => {};
    (@one $module:ident $variant:ident) => {
        fn $module(
            &self,
        ) -> Support<Reader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, Self::FileModel>> {
            absent(AbsenceCause::NoSuchConstruct)
        }
    };
}

/// A visit that writes nothing and allocates nothing.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the signature is the reader\'s: it is given the model of the file by reference"
)]
fn quiet(
    _tree: &Tree<'_>,
    _model: &(),
    _symbols: &mut Symbols,
    _at: Position,
    _out: &mut dyn Rows<FunctionRow>,
) {
}

/// A visit that allocates a block in every element it is given.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the signature is the reader\'s: it is given the model of the file by reference"
)]
fn greedy(
    _tree: &Tree<'_>,
    _model: &(),
    _symbols: &mut Symbols,
    _at: Position,
    _out: &mut dyn Rows<FunctionRow>,
) {
    drop(std::hint::black_box(Box::new(0_u8)));
}

/// The item that reads `function` and allocates nothing.
struct QuietItem;

impl crate::domain::answers::types::declared::Syntax for QuietItem {}

impl crate::domain::answers::types::Visiting<Function, ()> for QuietItem {
    fn visit(
        &self,
        tree: &Tree<'_>,
        model: &(),
        symbols: &mut Symbols,
        position: Position,
        out: &mut dyn Rows<FunctionRow>,
    ) {
        quiet(tree, model, symbols, position, out);
    }
}

impl ReadItem for QuietItem {
    type Relation = Function;
    type Model = ();

    const READER: Reader<Function, ()> = Reader {
        reads: Reads::Every,
        visit: &QuietItem,
        expansions: crate::domain::answers::types::Expansions::Ignored,
        relation: PhantomData,
    };
}

/// The item that reads `function` and allocates in every element.
struct GreedyItem;

impl crate::domain::answers::types::declared::Syntax for GreedyItem {}

impl crate::domain::answers::types::Visiting<Function, ()> for GreedyItem {
    fn visit(
        &self,
        tree: &Tree<'_>,
        model: &(),
        symbols: &mut Symbols,
        position: Position,
        out: &mut dyn Rows<FunctionRow>,
    ) {
        greedy(tree, model, symbols, position, out);
    }
}

impl ReadItem for GreedyItem {
    type Relation = Function;
    type Model = ();

    const READER: Reader<Function, ()> = Reader {
        reads: Reads::Every,
        visit: &GreedyItem,
        expansions: crate::domain::answers::types::Expansions::Ignored,
        relation: PhantomData,
    };
}

/// A language whose library parse allocates a scratch buffer in every file, and whose reader allocates in every element or not.
#[derive(Clone, Copy, Debug)]
pub struct AllocatingLanguage {
    /// Whether its reader allocates for every element it reads, or for none.
    pub greedy_reader: bool,
}

impl ParseUseCase for AllocatingLanguage {
    type FileModel = ();

    fn begin_semantics(
        &self,
        _request: &ParseRequest<'_>,
        _tree: &crate::domain::elements::types::Elements,
        _slot: &mut crate::domain::semantics::types::SemanticSlot,
    ) {
    }

    fn completion_requirement(&self) -> crate::domain::elements::types::CompletionRequirement {
        crate::domain::elements::types::CompletionRequirement::Independent
    }

    fn extend_file_model(&self, _tree: &Tree<'_>, _from: Position, _model: &mut ()) {}

    fn recognizes_explicit(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
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
        Count(1)
    }

    fn kind_names(&self) -> crate::inbound::use_cases::parse::types::KindNames {
        crate::inbound::use_cases::parse::types::KindNames(Vec::new())
    }

    fn read_tree(
        &self,
        request: &ParseRequest<'_>,
        builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        // The scratch of the library: one block in every file, whatever the builder builds.
        drop(std::hint::black_box(Vec::<u8>::with_capacity(
            request.bytes.0.len() + 64,
        )));
        let end = Offset(u32::try_from(request.bytes.0.len()).unwrap_or(u32::MAX));
        enter_node(builder, KindBits(0), Offset(0));
        token(builder, KindBits(0), Offset(0), end);
        leave_node(builder, end);
        Ok(())
    }

    fn complete(&self, _request: &ParseRequest<'_>, _completion: &mut Completion<'_>) {}

    fn function(&self) -> Support<Reader<Function, ()>> {
        if self.greedy_reader {
            reads::<GreedyItem>()
        } else {
            reads::<QuietItem>()
        }
    }

    ryo_relations::each_relation!(all_but_function);
}
