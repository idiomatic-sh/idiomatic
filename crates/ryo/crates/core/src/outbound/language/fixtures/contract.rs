//! What every `language` adapter must pass.

use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::parse::fixtures::contract::Discard;
use ryo_relations::domain::relations::types::{FileId, Relation};

use super::super::errors::LanguageError;
use super::super::interface::LanguagePort;
use super::super::types::{
    FileReading, FileToRead, ManifestReading, NoteWriting, ProjectNotes, ProjectToRead,
    SourceSelection,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::language::types::RoleWriting;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::stored_facts::types::LanguageNumber;

/// What a row of the contract is given: which scenario to run against the adapter.
type Input = fn(&dyn LanguagePort) -> Result<Value, LanguageError>;
/// Whether the scenario's expectation held.
type Value = bool;
/// What a row of the contract expects when it is refused.
type Failure = LanguageError;

/// Whether the relations listed include one every language may read, `function`.
fn the_catalog_lists_every_relation(language: &dyn LanguagePort) -> Result<Value, LanguageError> {
    Ok(language.relations()?.0.contains(&Relation::Function))
}

/// Whether a file of an extension no linked language claims is answered as unsupported.
fn a_file_no_language_reads_is_unsupported(language: &dyn LanguagePort) -> Value {
    let path = FilePath("contract.not-a-language".into());
    let bytes = FileBytes(b"anything".to_vec());
    let file = FileToRead {
        source_language: SourceSelection::Recognized,
        path: &path,
        file: FileId(0),
        bytes: &bytes,
        wanted: &[],
        notes: NoteWriting::Unwanted,
        roles: RoleWriting::Unwanted,
    };
    language.parse(
        file,
        &mut Elements::default(),
        &mut Symbols::default(),
        &mut Discard,
    ) == FileReading::Unsupported
}

/// Whether every linked language says, of every relation, whether it reads it.
fn every_linked_language_answers_for_a_relation(
    language: &dyn LanguagePort,
) -> Result<Value, LanguageError> {
    let linked = language.linked().0.len();
    Ok(language
        .relations()?
        .0
        .into_iter()
        .all(|relation| language.supports(relation).0.len() == linked))
}

/// Whether every linked language declares the vocabulary of the kinds of its trees.
fn each_linked_language_declares_the_kinds_of_its_trees(language: &dyn LanguagePort) -> Value {
    let linked = language.linked().0.len();
    let vocabularies = language.kind_vocabularies().0;
    vocabularies.len() == linked
}

/// Whether the reasons the linked languages declare for an unknown value are numbered apart: a number is one name.
fn the_reasons_for_an_unknown_value_are_numbered_apart(language: &dyn LanguagePort) -> Value {
    let reasons = language.unknown_reasons().0;
    reasons.iter().all(|each| {
        reasons
            .iter()
            .filter(|other| other.reason == each.reason)
            .all(|other| other.text == each.text)
    })
}

/// Whether a path that is no manifest of any language is not read as one.
fn a_file_that_is_no_manifest_is_not_read_as_one(language: &dyn LanguagePort) -> Value {
    let path = FilePath("contract.not-a-language".into());
    language.reads_manifest(&path) == ManifestReading::NotRead
}

/// Whether reading a project of no manifests refuses none.
fn reading_no_manifest_refuses_none(language: &dyn LanguagePort) -> Result<Value, LanguageError> {
    let root = RootPath("contract".into());
    let notes = ProjectNotes::default();
    let input = ProjectToRead {
        search_scope: &Default::default(),
        given_inventory: &Default::default(),
        observed_entries: &Default::default(),
        repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
        root: &root,
        manifests: &[],
        files: &[],
        notes: &notes,
        wanted: &[],
    };
    language
        .project(&input, &mut Symbols::default(), &mut Discard)
        .map(|answer| answer.refused.is_empty())
}

/// Whether asking for a semantic session of a language past the ones linked, over a tree nothing was read into, begins none.
fn a_language_not_linked_begins_no_semantic_session(language: &dyn LanguagePort) -> Value {
    let path = FilePath("contract.not-a-language".into());
    let bytes = FileBytes(b"anything".to_vec());
    let file = FileToRead {
        source_language: SourceSelection::Recognized,
        path: &path,
        file: FileId(0),
        bytes: &bytes,
        wanted: &[],
        notes: NoteWriting::Unwanted,
        roles: RoleWriting::Unwanted,
    };
    let not_linked = LanguageNumber(language.linked().0.len());
    matches!(
        language.begin_semantics(not_linked, file, &mut Elements::default()),
        crate::outbound::language::types::BegunSemantics::Unsupported
    )
}

/// Whether the entries a project of no manifests and no files asks for are each named once, in order.
fn the_entries_a_project_asks_for_are_named_once_in_order(language: &dyn LanguagePort) -> Value {
    let root = RootPath("contract".into());
    let notes = ProjectNotes::default();
    let input = ProjectToRead {
        search_scope: &Default::default(),
        given_inventory: &Default::default(),
        observed_entries: &Default::default(),
        repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
        root: &root,
        manifests: &[],
        files: &[],
        notes: &notes,
        wanted: &[],
    };
    language
        .project_entries(&input)
        .into_paths()
        .windows(2)
        .all(|pair| pair[0] < pair[1])
}

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_catalog_lists_every_relation"),
        word: Word::Succeeds,
        input: || the_catalog_lists_every_relation,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_no_language_reads_is_unsupported"),
        word: Word::Succeeds,
        input: || {
            |language: &dyn LanguagePort| Ok(a_file_no_language_reads_is_unsupported(language))
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("every_linked_language_answers_for_a_relation"),
        word: Word::Succeeds,
        input: || every_linked_language_answers_for_a_relation,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("each_linked_language_declares_the_kinds_of_its_trees"),
        word: Word::Succeeds,
        input: || {
            |language: &dyn LanguagePort| {
                Ok(each_linked_language_declares_the_kinds_of_its_trees(
                    language,
                ))
            }
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_reasons_for_an_unknown_value_are_numbered_apart"),
        word: Word::Succeeds,
        input: || {
            |language: &dyn LanguagePort| {
                Ok(the_reasons_for_an_unknown_value_are_numbered_apart(
                    language,
                ))
            }
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_that_is_no_manifest_is_not_read_as_one"),
        word: Word::Succeeds,
        input: || {
            |language: &dyn LanguagePort| {
                Ok(a_file_that_is_no_manifest_is_not_read_as_one(language))
            }
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_language_not_linked_begins_no_semantic_session"),
        word: Word::Succeeds,
        input: || {
            |language: &dyn LanguagePort| {
                Ok(a_language_not_linked_begins_no_semantic_session(language))
            }
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_entries_a_project_asks_for_are_named_once_in_order"),
        word: Word::Succeeds,
        input: || {
            |language: &dyn LanguagePort| {
                Ok(the_entries_a_project_asks_for_are_named_once_in_order(
                    language,
                ))
            }
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("reading_no_manifest_refuses_none"),
        word: Word::Succeeds,
        input: || reading_no_manifest_refuses_none,
        expect: Expect::Value(true),
    },
];

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &dyn LanguagePort) {
    run(CASES, Word::Succeeds, |scenario| scenario(adapter));
}
