//! The allocation row of the engine, proved on language doubles: the allocations of a language's own library parse (`read_tree`) are in
//! the count of reading and in the count of the baseline alike, so they cancel, and only the table and the readers are judged. A
//! `read_tree` that allocates a scratch buffer in every file passes; a reader that allocates in every file fails. It runs alone in its
//! process, with `dhat` as the global allocator, so nothing else counts.

use ryo_relations::domain::terms::types::UnknownName;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};

use ryo_language::domain::answers::service::{absent, reads};
use ryo_language::domain::answers::types::{ReadItem, Reader, Reads, Support};
use ryo_language::domain::builders::service::{enter_node, leave_node, token};
use ryo_language::domain::elements::types::{Completion, TableBuilder, Tree};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::parse::errors::ParseError;
use ryo_language::inbound::use_cases::parse::fixtures::samples::allocations;
use ryo_language::inbound::use_cases::parse::interface::ParseUseCase;
use ryo_language::inbound::use_cases::parse::service::baseline;
use ryo_language::inbound::use_cases::parse::types::Recognition;
use ryo_language::inbound::use_cases::parse::types::{ParseRequest, SourceFile};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::function::item::{
    Function, FunctionRow,
};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position, Rows};

/// Counts every allocation the test makes, so a run's allocations are read back.
#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

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

impl ryo_language::domain::answers::types::declared::Syntax for QuietItem {}

impl ryo_language::domain::answers::types::Visiting<Function, ()> for QuietItem {
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
        expansions: ryo_language::domain::answers::types::Expansions::Ignored,
        relation: PhantomData,
    };
}

/// The item that reads `function` and allocates in every element.
struct GreedyItem;

impl ryo_language::domain::answers::types::declared::Syntax for GreedyItem {}

impl ryo_language::domain::answers::types::Visiting<Function, ()> for GreedyItem {
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
        expansions: ryo_language::domain::answers::types::Expansions::Ignored,
        relation: PhantomData,
    };
}

/// A language whose library parse allocates a scratch buffer in every file, and whose reader allocates in every element or not.
struct Language {
    /// Whether its reader allocates for every element it reads, or for none.
    greedy_reader: bool,
}

impl ParseUseCase for Language {
    type FileModel = ();

    fn begin_semantics(
        &self,
        _request: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
        _tree: &ryo_language::domain::elements::types::Elements,
        _slot: &mut ryo_language::domain::semantics::types::SemanticSlot,
    ) {
    }

    fn completion_requirement(
        &self,
    ) -> ryo_language::domain::elements::types::CompletionRequirement {
        ryo_language::domain::elements::types::CompletionRequirement::Independent
    }

    fn extend_file_model(
        &self,
        _tree: &ryo_language::domain::elements::types::Tree<'_>,
        _from: ryo_relations::domain::relations::types::Position,
        _model: &mut (),
    ) {
    }

    fn recognizes_explicit(
        &self,
        _request: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
    ) -> ryo_language::inbound::use_cases::parse::types::Recognition {
        ryo_language::inbound::use_cases::parse::types::Recognition::Declines
    }

    fn capabilities(
        &self,
    ) -> &'static [ryo_relations::domain::capabilities::types::OptionalCapability] {
        &[]
    }

    fn configuration_identity(
        &self,
    ) -> ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_> {
        ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity("")
    }

    fn file_model(&self, _tree: &Tree<'_>, _model: &mut ()) {}

    fn recognizes(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Reads
    }

    fn skips(&self) -> &'static [ryo_language::inbound::use_cases::parse::types::SkippedDirectory] {
        &[]
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn kind_count(&self) -> Count {
        Count(1)
    }

    fn kind_names(&self) -> ryo_language::inbound::use_cases::parse::types::KindNames {
        ryo_language::inbound::use_cases::parse::types::KindNames(Vec::new())
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

/// Runs the allocation row over the double and says whether it held.
fn held(language: &Language) -> bool {
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
                    notes: ryo_language::inbound::use_cases::parse::types::NoteRequest::Unwanted,
                    fields: ryo_language::inbound::use_cases::parse::types::FieldRequest::Unwanted,
                },
            )
        });
    }))
    .is_ok();
    std::panic::set_hook(printed);
    held
}

fn main() {
    assert!(
        held(&Language {
            greedy_reader: false
        }),
        "a library parse that allocates in every file is the library's, and the row holds"
    );
    assert!(
        !held(&Language {
            greedy_reader: true
        }),
        "a reader that allocates in every element is not the library's, and the row does not hold"
    );
}
