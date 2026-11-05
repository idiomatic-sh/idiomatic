//! What the `parse` contract promises, against the fakes of the syntax and notes ports and against the language's own adapters.

use ryo::languages::{self, Parse as _};
use std::sync::Arc;

use ryo::languages::contract::{flow, parse};

use super::contract::ParseContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::notes::fixtures::fake::NotesFake;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use std::sync::Mutex;

/// What a row does: reads a file (its path, its bytes and the relations wanted), or runs the language contract.
#[derive(Clone, Copy)]
enum Given {
    Read(&'static str, &'static [u8], &'static [languages::Relation]),
    Contract,
}
/// How many elements the file's table has, and how many reference rows were written; for the language contract, how many samples it
/// flowed and how many kinds the language has.
type Value = (usize, usize);
type Input = Given;
type Failure = languages::ParseError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("parses_a_sample_file_into_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("app.py", b"sample", &[languages::Relation::Reference]),
        expect: Expect::Value((4, 1)),
    },
    Case {
        name: CaseName("passes_the_language_contract"),
        word: Word::Succeeds,
        input: || Given::Contract,
        expect: Expect::Value((3, 192)),
    },
    Case {
        name: CaseName("builds_only_the_wanted_relations"),
        word: Word::Succeeds,
        input: || Given::Read("app.py", b"sample", &[]),
        expect: Expect::Value((4, 0)),
    },
    Case {
        name: CaseName("a_file_in_another_language_is_unsupported"),
        word: Word::Refuses,
        input: || Given::Read("src/lib.rs", b"sample", &[languages::Relation::Reference]),
        expect: Expect::Error(languages::ParseError::Unsupported),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    let (path, bytes, wanted) = match given {
        Given::Read(path, bytes, wanted) => (path, bytes, wanted),
        Given::Contract => {
            let language = ParseContract::new(
                PythonParserSettings::default(),
                Arc::new(crate::outbound::syntax::factory::ruff_python_parser()),
                Arc::new(crate::outbound::notes::factory::postcard()),
            );
            parse(&language);
            flow(&language, SAMPLES);
            return Ok((SAMPLES.len(), language.kind_count().0));
        }
    };
    let language = ParseContract::new(
        PythonParserSettings::default(),
        Arc::new(SyntaxFake),
        Arc::new(NotesFake),
    );
    let mut out = languages::contract::Collect::default();
    let (mut table, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    languages::contract::read(
        &language,
        languages::ParseRequest {
            path: languages::SourcePath(path),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(bytes),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Unwanted,
        },
        &mut table,
        &mut symbols,
        wanted,
        &mut out,
    )?;
    Ok((
        languages::contract::element_count(&table).0,
        out.count(languages::Relation::Reference),
    ))
}

/// The files the flow of the contract reads: a module with definitions and imports, a class with decorators, and a file the parser
/// can only half read.
const SAMPLES: &[languages::SourceFile] = &[
    languages::SourceFile { path: languages::SourcePath("app.py"), bytes: languages::ByteSlice(b"import os\nfrom . import x as y\n\ndef run(a, *b, c=1, **d) -> int:  # run it\n    return os.path.join(a, 'x')\n") },
    languages::SourceFile { path: languages::SourcePath("model.py"), bytes: languages::ByteSlice(b"@dataclass(frozen=True)\nclass Point(Base, metaclass=Meta):\n    x: int = 0\n") },
    languages::SourceFile { path: languages::SourcePath("broken.py"), bytes: languages::ByteSlice(b"good = 1\ndef broken(\n") },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[derive(Clone)]
struct CapturingSyntax(Arc<Mutex<Vec<(PythonSourceKind, String)>>>);

impl SyntaxPort for CapturingSyntax {
    fn expand_annotations(
        &self,
        _source: SourceText<'_>,
        carriers: &[crate::outbound::syntax::types::DeferredAnnotation],
        _builder: &mut ryo::languages::TableBuilder<'_>,
    ) {
        assert!(
            carriers.is_empty(),
            "this syntax fixture supplies no deferred type carrier"
        );
    }

    fn read(
        &self,
        _text: SourceText<'_>,
        source_kind: PythonSourceKind,
        settings: &PythonParserSettings,
        _builder: &mut languages::TableBuilder<'_>,
    ) {
        self.0
            .lock()
            .expect("capture lock")
            .push((source_kind, settings.identity().to_owned()));
    }
}

#[test]
fn parse_contract_passes_path_kind_and_its_settings_to_syntax() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let settings = PythonParserSettings::new(PythonTargetVersion::Py310);
    let language = ParseContract::new(
        settings,
        Arc::new(CapturingSyntax(Arc::clone(&seen))),
        Arc::new(NotesFake),
    );

    for path in ["module.py", "module.pyi"] {
        let bytes = b"x = 1\n";
        let request = languages::ParseRequest {
            path: languages::SourcePath(path),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(bytes),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Unwanted,
        };
        let (mut elements, mut symbols) = (
            languages::Elements::default(),
            languages::Symbols::default(),
        );
        let mut builder =
            languages::table_builder(&mut elements, &mut symbols, languages::ByteSlice(bytes));
        language
            .read_tree(&request, &mut builder)
            .expect("read tree");
    }

    assert_eq!(
        *seen.lock().expect("capture lock"),
        [
            (PythonSourceKind::Python, "python-target=3.10".to_owned()),
            (PythonSourceKind::Stub, "python-target=3.10".to_owned()),
        ]
    );
    assert_eq!(language.configuration_identity().0, "python-target=3.10");
}

#[test]
fn linked_python_keeps_latest_as_its_default_target() {
    assert_eq!(
        crate::linked().configuration_identity().0,
        "python-target=latest"
    );
    assert_eq!(
        crate::linked_with_settings(PythonParserSettings::new(PythonTargetVersion::Py310))
            .configuration_identity()
            .0,
        "python-target=3.10"
    );
}

#[test]
fn explicit_source_selection_preserves_paths_and_default_recognition() {
    let provider = crate::linked().parse_engine();
    for path in ["test.txt", "file-with-dashes"] {
        let request = languages::ParseRequest {
            path: languages::SourcePath(path),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(b"# complete comment\nx = 1\n"),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Unwanted,
        };
        assert_eq!(
            provider.recognition(&request),
            languages::Recognition::Declines
        );
        let (mut elements, mut symbols) = (
            languages::Elements::default(),
            languages::Symbols::default(),
        );
        let mut out = languages::contract::Collect::default();
        assert_eq!(
            provider.read_file(request, &mut elements, &mut symbols, &[], &mut out),
            Err(languages::ParseError::Unsupported)
        );
        provider
            .read_file_explicit(
                request,
                &mut elements,
                &mut symbols,
                &[languages::Relation::Comment],
                &mut out,
            )
            .expect("explicitly selected Python source");
        assert_eq!(out.count(languages::Relation::Comment), 1);
        assert!(!languages::has_incomplete_lexical_stream(&elements));
        assert_eq!(request.path.0, path);
    }
    let request = languages::ParseRequest {
        path: languages::SourcePath("test.txt"),
        file: languages::FileId(0),
        bytes: languages::ByteSlice(b"x = \xff\n"),
        notes: languages::NoteRequest::Unwanted,
        fields: languages::FieldRequest::Unwanted,
    };
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    provider
        .read_file_explicit(
            request,
            &mut elements,
            &mut symbols,
            &[],
            &mut languages::contract::Collect::default(),
        )
        .expect("invalid bytes are diagnosed");
    assert!(languages::has_incomplete_lexical_stream(&elements));
}

#[test]
fn byte_literal_recovery_uses_original_source_acceptance_and_diagnostic_readiness() {
    let provider = ParseContract::new(
        PythonParserSettings::default(),
        Arc::new(crate::outbound::syntax::factory::ruff_python_parser()),
        Arc::new(crate::outbound::notes::factory::postcard()),
    );
    // Preserve invalid original UTF-8 and truncated escapes; an escaped high byte is valid UTF-8 source.
    for (bytes, incomplete) in [
        (&b"b'\xff'\n"[..], true),
        (&b"b'\\x"[..], true),
        (&b"b'\\xff'\n"[..], false),
    ] {
        let (mut elements, mut symbols) = (
            languages::Elements::default(),
            languages::Symbols::default(),
        );
        let mut out = languages::contract::Collect::default();
        languages::contract::read(
            &provider,
            languages::ParseRequest {
                path: languages::SourcePath("item.py"),
                file: languages::FileId(0),
                bytes: languages::ByteSlice(bytes),
                notes: languages::NoteRequest::Unwanted,
                fields: languages::FieldRequest::Wanted,
            },
            &mut elements,
            &mut symbols,
            &[languages::Relation::BytesLiteral],
            &mut out,
        )
        .expect("recovered original bytes remain readable with honest diagnostic metadata");
        assert_eq!(
            languages::has_incomplete_syntax(&elements),
            incomplete,
            "{bytes:?}"
        );
        assert_eq!(
            languages::has_incomplete_lexical_stream(&elements),
            incomplete,
            "{bytes:?}"
        );
        assert_eq!(
            languages::syntax_acceptance(&elements),
            if incomplete {
                languages::SyntaxAcceptance::Rejected
            } else {
                languages::SyntaxAcceptance::Accepted
            },
            "{bytes:?}"
        );
        if !incomplete {
            assert_eq!(out.count(languages::Relation::BytesLiteral), 1);
        }
        // Collect.partials describes reader payloads only. The common engine derives
        // semantic unknown-absence coverage from the Elements diagnostics above.
    }
}
