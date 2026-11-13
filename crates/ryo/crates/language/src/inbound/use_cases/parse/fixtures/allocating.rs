//! The language double of the kernel's allocation row: a language whose own library parse (`read_tree`) allocates a scratch
//! buffer in every file, and whose reader allocates in every element or in none. The allocations of the library parse are in
//! the count of reading and in the count of the baseline alike, so they cancel, and only the table and the readers are judged.
//! The caller's test binary installs `dhat::Alloc` as its global allocator and runs alone in its process.

use ryo_relations::domain::terms::types::UnknownName;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::domain::answers::service::{absent, reads};
use crate::domain::answers::types::{ReadItem, Reader, Reads, Support};
use crate::domain::builders::service::{enter_node, leave_node, token};
use crate::domain::elements::types::{Completion, TableBuilder, Tree};
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::parse::errors::ParseError;
use crate::inbound::use_cases::parse::fixtures::samples::allocations;
use crate::inbound::use_cases::parse::interface::ParseUseCase;
use crate::inbound::use_cases::parse::service::baseline;
use crate::inbound::use_cases::parse::types::Recognition;
use crate::inbound::use_cases::parse::types::{ParseRequest, SourceFile};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::function::item::{
    Function, FunctionRow,
};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position, Rows};

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

    fn semantics(&self) -> Option<&dyn crate::domain::answers::types::Semantics> {
        None
    }

    fn completion_requirement(&self) -> crate::domain::elements::types::CompletionRequirement {
        crate::domain::elements::types::CompletionRequirement::Independent
    }

    fn extend_file_model(&self, _tree: &Tree<'_>, _from: Position, _model: &mut ()) {}

    fn recognizes_explicit(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }

    fn capabilities(&self) -> &'static [ryo_relations::domain::catalog::types::OptionalCapability] {
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

    ryo_relations::each_relation!(@declared ryo_relations::baseline_relations, all_but_function);
}

/// Runs the allocation row over the double and says whether it held.
pub fn held(language: &AllocatingLanguage) -> bool {
    let sample = SourceFile {
        path: SourcePath("sample.double"),
        bytes: ByteSlice(b"fn one() {}\nfn two() {}\n"),
    };
    // The refusal is the row's answer here, not a failure: its message is not printed.
    let printed = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let held = catch_unwind(AssertUnwindSafe(|| {
        allocations(language, sample, &|bytes| {
            baseline(
                language,
                &ParseRequest {
                    path: sample.path,
                    file: ryo_relations::domain::relations::types::FileId(0),
                    bytes,
                    notes: crate::inbound::use_cases::parse::types::NoteRequest::Unwanted,
                    fields: crate::inbound::use_cases::parse::types::FieldRequest::Unwanted,
                },
            )
        });
    }))
    .is_ok();
    std::panic::set_hook(printed);
    held
}
