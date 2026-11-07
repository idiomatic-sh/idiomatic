//! The `parse` service's cases: what a walk gives the readers a language answers.

use crate::inbound::use_cases::parse::types::Recognition;
use ryo_relations::domain::terms::types::UnknownName;
use std::marker::PhantomData;
use std::sync::Mutex;

use super::errors::ParseError;
use super::fixtures::breaking::Breaking;
use super::fixtures::ordered::Ordered;
use super::fixtures::promised::Promised;
use super::fixtures::small::{Seen, Small};
use super::interface::ParseUseCase;
use super::service::{absence, build, walk};
use super::types::{FieldRequest, NoteRequest, ParseRequest};
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
use crate::inbound::use_cases::parse::fixtures::collect::Collect;
use crate::selects;
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
        name: CaseName(
            "default_requirement_preserves_completion_before_each_model_and_real_reader_results",
        ),
        word: Word::Succeeds,
        input: || {
            || {
                Ok(default_requirement_preserves_completion_before_each_model_and_real_reader_results())
            }
        },
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("unsupported_compiler_feature_is_unknown_and_emits_no_false_row"),
        word: Word::Succeeds,
        input: || || Ok(unsupported_compiler_feature_is_unknown_and_emits_no_false_row()),
        expect: Expect::Value(1),
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
            selects!(error, ParseError::Library { reason }
                if &*reason.0 == "the library failed")
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

/// A retained worker alternates absent/advertised/absent promises without cached leakage.
fn optional_readiness_is_fail_closed() -> usize {
    use ryo_relations::domain::relations::types::OptionalCapability;
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

/// A language without the optional compiler-feature promise cannot imply a disabled flag.
fn unsupported_compiler_feature_is_unknown_and_emits_no_false_row() -> usize {
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    let source = ByteSlice(b"ab");
    let mut builder = table_builder(&mut elements, &mut symbols, source);
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    assert_eq!(
        absence(&Small, Relation::CompilerFeature),
        Some(AbsenceCause::NotYetRead)
    );
    let mut out = Collect::default();
    walk(
        &Small,
        &mut elements,
        &mut symbols,
        source,
        FileId(0),
        &[Relation::CompilerFeature],
        &mut out,
    );
    assert!(out.rows().is_empty());
    assert_eq!(out.partials_at.len(), 1);
    out.partials_at.len()
}

/// Non-refining providers retain their original completed-tree/per-walk initialization order: the events it saw.
fn default_requirement_preserves_completion_before_each_model_and_real_reader_results() -> usize {
    let provider = Ordered::default();
    assert_eq!(
        provider.completion_requirement(),
        crate::domain::elements::types::CompletionRequirement::Independent
    );
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    build(
        &provider,
        ParseRequest {
            path: SourcePath("order.small"),
            file: FileId(0),
            bytes: ByteSlice(b"x"),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        },
        &mut elements,
        &mut symbols,
    )
    .unwrap();
    assert_eq!(*provider.events.lock().unwrap(), ["complete"]);
    for expected in [2, 4] {
        let mut out = Collect::default();
        walk(
            &provider,
            &mut elements,
            &mut symbols,
            ByteSlice(b"x"),
            FileId(0),
            &[Relation::Function],
            &mut out,
        );
        assert_eq!(out.count(Relation::Function), 1);
        assert_eq!(out.rows()[0].1[0], expected);
    }
    assert_eq!(
        *provider.events.lock().unwrap(),
        ["complete", "model", "model"]
    );
    provider.events.lock().unwrap().len()
}
