// ignore: idiomatic.arch/file-length - one row for each thing a walk gives a reader, over one language written for them
//! The `parse` service's cases: what a walk gives the readers a language answers.

use crate::domain::terms::types::UnknownName;
use crate::inbound::use_cases::parse::types::Recognition;
use std::marker::PhantomData;

use super::errors::ParseError;
use super::interface::ParseUseCase;
use super::service::{absence, build, walk};
use super::types::{FieldRequest, NoteRequest, ParseRequest};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::answers::service::{absent, reads};
use crate::domain::answers::types::{AbsenceCause, Expansions, ReadItem, Reader, Reads, Support};
use crate::domain::elements::service::{end_of, enter_node, leave_node, table_builder, token};
use crate::domain::elements::types::{Completion, Elements, TableBuilder, Tree};
use crate::domain::files::types::SourcePath;
use crate::domain::reasons::types::ToolMessage;
use crate::domain::relations::collections::relations::function::item::{Function, FunctionRow};
use crate::domain::relations::collections::relations::syntax_tree::item::{
    SyntaxTree, SyntaxTreeRow,
};
use crate::domain::relations::types::{
    Count, FileId, KindBits, NodeId, Offset, Position, Relation, Rows, Span,
};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use crate::domain::terms::types::Symbol;
use crate::inbound::use_cases::parse::fixtures::contract::Collect;

/// A language that reads `function` from the tokens of kind 2 and `syntax_tree` from every element,
/// and says the rest is not in it.
struct Small;

/// What a worker of `Small` keeps between files: the sum of the ends of the trees it has read.
#[derive(Default)]
struct Seen(u32);

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
        ) -> Support<Reader<crate::domain::relations::collections::relations::$module::item::$variant, Self::FileModel>> {
            absent(AbsenceCause::NoSuchConstruct)
        }
    };
}

/// The item that reads `function` from the tokens of kind 2.
struct FunctionItem;

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
struct TreeItem;

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

    fn capabilities(&self) -> &'static [crate::domain::relations::types::OptionalCapability] {
        &[]
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
            crate::domain::relations::collections::relations::source_location::item::SourceLocation,
            Seen,
        >,
    > {
        crate::domain::answers::service::source_locations()
    }

    crate::each_relation!(absent_but_two);
}

/// A language whose library fails: on bytes that are not text it says so, on any other it names the failure.
struct Breaking;

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
            ) -> Support<Reader<crate::domain::relations::collections::relations::$module::item::$variant, Self::FileModel>> {
                absent(AbsenceCause::NoSuchConstruct)
            }
        )*
    };
}

impl ParseUseCase for Breaking {
    type FileModel = ();

    fn capabilities(&self) -> &'static [crate::domain::relations::types::OptionalCapability] {
        &[]
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

    crate::each_relation!(absent_all);
}

/// Reads a file of these bytes through `language`.
fn built(language: &impl ParseUseCase, bytes: &'static [u8]) -> Result<usize, ParseError> {
    build(
        language,
        ParseRequest {
            path: SourcePath("a.small"),
            file: FileId(0),
            bytes: ByteSlice(bytes),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        },
        &mut Elements::default(),
        &mut Symbols::default(),
    )
    .map(|()| 0)
}

fn a_file_the_language_does_not_read() -> Result<usize, ParseError> {
    build(
        &Small,
        ParseRequest {
            path: SourcePath("a.other"),
            file: FileId(0),
            bytes: ByteSlice(b""),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        },
        &mut Elements::default(),
        &mut Symbols::default(),
    )
    .map(|()| 0)
}

fn a_file_the_library_cannot_read() -> Result<usize, ParseError> {
    built(&Breaking, b"x")
}

/// The rows a walk of a root with two tokens writes for the relations asked.
fn walked(wanted: &[Relation]) -> Collect {
    walked_times(wanted, 1)
}

/// The rows the last of `times` walks of the same table writes: the worker keeps the language's model between them.
fn walked_times(wanted: &[Relation], times: usize) -> Collect {
    let (mut elements, mut symbols): (Elements, Symbols) =
        (Elements::default(), Symbols::default());
    let source = ByteSlice(b"ab");
    let mut builder = table_builder(&mut elements, &mut symbols, source);
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    let mut out = Collect::default();
    for _ in 0..times {
        out = Collect::default();
        walk(
            &Small,
            &mut elements,
            &mut symbols,
            source,
            FileId(0),
            wanted,
            &mut out,
        );
    }
    out
}

fn the_model_a_worker_keeps_is_given_back_for_the_next_file() -> usize {
    walked_times(&[Relation::Function], 2)
        .rows()
        .iter()
        .map(|(_, words)| usize::try_from(words[0]).unwrap_or(usize::MAX))
        .sum()
}

fn a_reader_looks_only_at_the_kinds_it_names() -> usize {
    walked(&[Relation::Function]).count(Relation::Function)
}

fn a_reader_of_every_element_sees_the_root_and_both_tokens() -> usize {
    walked(&[Relation::SyntaxTree])
        .rows()
        .iter()
        .filter(|(_, words)| words[0] != 999)
        .count()
}

fn a_reader_is_given_what_it_worked_out_of_the_whole_file() -> usize {
    let rows = walked(&[Relation::Function]);
    rows.rows()
        .iter()
        .map(|(_, words)| usize::try_from(words[0]).unwrap_or(usize::MAX))
        .sum()
}

fn a_relation_nobody_asked_for_is_not_read() -> usize {
    walked(&[Relation::Function]).count(Relation::SyntaxTree)
}

fn a_relation_not_in_the_language_gives_no_rows() -> usize {
    walked(&[Relation::Comment, Relation::Mapping])
        .relations
        .len()
}

fn a_relation_the_language_reads_has_no_reason_and_one_it_does_not_has_one() -> usize {
    usize::from(
        absence(&Small, Relation::Function).is_none()
            && absence(&Small, Relation::Comment) == Some(AbsenceCause::NoSuchConstruct)
            && absence(&Small, Relation::PackageDependency).is_none(),
    )
}

/// A scenario: what a row asks of the walk.
type Input = fn() -> Result<usize, ParseError>;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ParseError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("optional reads preserve declared availability across retained requests"),
        word: Word::Succeeds,
        input: || || Ok(optional_readiness_is_fail_closed()),
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("shared_source_locations_index_and_visit_every_element"),
        word: Word::Succeeds,
        input: || {
            || {
                let found = walked(&[Relation::SourceLocation]);
                Ok(found
                    .rows()
                    .iter()
                    .filter(|(relation, row)| {
                        *relation == Relation::SourceLocation && row[3] == 1 && row[4] == 1
                    })
                    .count())
            }
        },
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_reader_looks_only_at_the_kinds_it_names"),
        word: Word::Succeeds,
        input: || || Ok(a_reader_looks_only_at_the_kinds_it_names()),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_reader_of_every_element_sees_the_root_and_both_tokens"),
        word: Word::Succeeds,
        input: || || Ok(a_reader_of_every_element_sees_the_root_and_both_tokens()),
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_reader_is_given_what_it_worked_out_of_the_whole_file"),
        word: Word::Succeeds,
        input: || || Ok(a_reader_is_given_what_it_worked_out_of_the_whole_file()),
        expect: Expect::Value(6),
    },
    Case {
        name: CaseName("the_model_a_worker_keeps_is_given_back_for_the_next_file"),
        word: Word::Succeeds,
        input: || || Ok(the_model_a_worker_keeps_is_given_back_for_the_next_file()),
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("a_relation_nobody_asked_for_is_not_read"),
        word: Word::Succeeds,
        input: || || Ok(a_relation_nobody_asked_for_is_not_read()),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_relation_not_in_the_language_gives_no_rows"),
        word: Word::Succeeds,
        input: || || Ok(a_relation_not_in_the_language_gives_no_rows()),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_relation_the_language_reads_has_no_reason_and_one_it_does_not_has_one"),
        word: Word::Succeeds,
        input: || || Ok(a_relation_the_language_reads_has_no_reason_and_one_it_does_not_has_one()),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_the_language_does_not_read_is_unsupported"),
        word: Word::Refuses,
        input: || a_file_the_language_does_not_read,
        expect: Expect::Error(ParseError::Unsupported),
    },
    Case {
        name: CaseName("a_library_that_fails_is_refused"),
        word: Word::Refuses,
        input: || a_file_the_library_cannot_read,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, ParseError::Library { reason }
                if &*reason.0 == "the library failed"))
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// An optional reader that emits one real row per visited occurrence.
struct IdentifierProbe<const FORBIDDEN: bool>;
impl<const FORBIDDEN: bool> crate::domain::answers::types::declared::Syntax
    for IdentifierProbe<FORBIDDEN>
{
}
impl<const FORBIDDEN: bool> ReadItem for IdentifierProbe<FORBIDDEN> {
    type Relation = crate::domain::relations::collections::relations::identifier::item::Identifier;
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
        crate::domain::relations::collections::relations::identifier::item::Identifier,
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
            crate::domain::relations::collections::relations::identifier::item::IdentifierRow,
        >,
    ) {
        assert!(!FORBIDDEN, "an unadvertised reader must never run");
        out.write(
            crate::domain::relations::collections::relations::identifier::item::IdentifierRow {
                id: crate::domain::relations::service::node_id(tree.file, position),
                name: crate::domain::symbols::service::intern_derived(
                    symbols,
                    tree.source,
                    crate::domain::terms::types::NameText("name"),
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
struct Promised(
    &'static [crate::domain::relations::types::OptionalCapability],
    bool,
);
macro_rules! forwarded_optional_method {
    (Identifier identifier) => {};
    ($variant:ident $method:ident) => {
        fn $method(
            &self,
        ) -> Support<
            Reader<crate::domain::relations::collections::relations::$method::item::$variant, Seen>,
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
    fn capabilities(&self) -> &'static [crate::domain::relations::types::OptionalCapability] {
        self.0
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
            crate::domain::relations::collections::relations::identifier::item::Identifier,
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
    crate::each_relation!(forwarded_optional_methods);
}
/// A retained worker alternates absent/advertised/absent promises without cached leakage.
fn optional_readiness_is_fail_closed() -> usize {
    use crate::domain::relations::types::OptionalCapability;
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    let source = ByteSlice(b"ab");
    let mut builder = table_builder(&mut elements, &mut symbols, source);
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    let mut passed = 0;
    for (provider, count, partials, absent_reason) in [
        (Promised(&[], true), 0, 1, Some(AbsenceCause::NotYetRead)),
        (
            Promised(&[OptionalCapability::Identifier], true),
            3,
            0,
            None,
        ),
        (Promised(&[], true), 0, 1, Some(AbsenceCause::NotYetRead)),
        (
            Promised(&[OptionalCapability::Identifier], false),
            0,
            1,
            Some(AbsenceCause::NotYetRead),
        ),
    ] {
        let before = absence(&provider, Relation::Identifier);
        let mut out = Collect::default();
        walk(
            &provider,
            &mut elements,
            &mut symbols,
            source,
            FileId(0),
            &[Relation::Identifier],
            &mut out,
        );
        if before == absent_reason
            && absence(&provider, Relation::Identifier) == before
            && out.rows().len() == count
            && out.partials_at.len() == partials
        {
            passed += 1;
        }
    }
    passed
}
