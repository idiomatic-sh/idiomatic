//! What the schemas of this entry accept and refuse, one row of `CASES` for each input.

use ryo_core::inbound::use_cases::validate::errors::ValidateError as CoreError;
use ryo_core::inbound::use_cases::validate::types::ValidateRequest as CoreRequest;
use ryo_relations::domain::relations::types::{CellKind, Offset, Span as CoreSpan};
use ryo_values::domain::hosted::types::{ColumnLabel, HostedColumn, HostedName, HostedRelation};
use ryo_values::domain::{
    plan_parts::types::{QueryText, RefusalMessage, RefusedKind, RefusedQuery},
    program::types::{
        ByteOffset as CoreByteOffset, QueryOrigin as CoreQueryOrigin, QuerySourceId,
        QuerySpan as CoreQuerySpan,
    },
};

use super::{error, request};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::host::types::{
    HostColumn, HostColumnName, HostDeclaration, HostKind, HostRelationName,
};
use crate::domain::query::errors::QueryOrigin;
use crate::domain::query::errors::ValidateError;
use crate::domain::query::types::{ByteOffset, QuerySpan, RyoQlQuery};
use crate::resources::query::validate::types::ValidateRequest;

/// The core's error offered to the conversion.
type Input = CoreError;
/// The byte the SDK's error says the text goes wrong at.
type Value = usize;
/// Why the schemas end in error.
type Failure = ValidateError;

/// The query text the rows validate.
const TEXT: &str =
    "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"spec\");";

/// A refusal of the text of `kind` from byte `start` to byte `end`.
fn refused(kind: RefusedKind, start: u32, end: u32) -> RefusedQuery {
    RefusedQuery {
        kind,
        message: RefusalMessage(String::new()),
        span: CoreSpan {
            start: Offset(start),
            end: Offset(end),
        },
        origin: None,
    }
}

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_syntax_error_keeps_its_place"),
        word: Word::Succeeds,
        input: || CoreError::Refused {
            error: refused(RefusedKind::Syntax, 3, 5),
        },
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("an_unknown_relation_keeps_its_place"),
        word: Word::Succeeds,
        input: || CoreError::Refused {
            error: refused(RefusedKind::UnknownRelation, 7, 14),
        },
        expect: Expect::Value(7),
    },
    Case {
        name: CaseName("a_type_error_keeps_its_place"),
        word: Word::Succeeds,
        input: || CoreError::Refused {
            error: refused(RefusedKind::Type, 2, 4),
        },
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("an_invalid_query_keeps_its_place"),
        word: Word::Succeeds,
        input: || CoreError::Refused {
            error: refused(RefusedKind::Invalid, 5, 6),
        },
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("a_word_for_no_kind_makes_the_whole_text_invalid"),
        word: Word::Succeeds,
        input: || CoreError::UnknownKind {
            reason: String::new(),
            origin: None,
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_correct_query_that_cannot_be_compiled_makes_the_whole_text_invalid"),
        word: Word::Succeeds,
        input: || CoreError::Uncompilable {
            error: refused(RefusedKind::Invalid, 5, 6),
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("fails_when_the_catalog_fails"),
        word: Word::Fails,
        input: || CoreError::Refused {
            error: refused(RefusedKind::Catalog, 0, 0),
        },
        expect: Expect::Error(ValidateError::Catalog {
            reason: String::new(),
        }),
    },
];

/// Converts one error of the core and answers where the SDK's says the text goes wrong.
fn subject(input: Input) -> Result<Value, Failure> {
    match error(TEXT, input) {
        ValidateError::Syntax { span, .. }
        | ValidateError::UnknownRelation { span, .. }
        | ValidateError::Type { span, .. }
        | ValidateError::Invalid { span, .. } => Ok(span.start.0),
        ValidateError::Located { error, .. } => match *error {
            ValidateError::Syntax { span, .. }
            | ValidateError::UnknownRelation { span, .. }
            | ValidateError::Type { span, .. }
            | ValidateError::Invalid { span, .. } => Ok(span.start.0),
            error @ (ValidateError::Catalog { .. } | ValidateError::Located { .. }) => Err(error),
        },
        error @ ValidateError::Catalog { .. } => Err(error),
    }
}

#[test]
fn an_uncompilable_fragment_error_keeps_its_module_location() {
    let origin = QueryOrigin {
        source_id: "nested/module".into(),
        span: QuerySpan {
            start: ByteOffset(7),
            end: ByteOffset(12),
        },
        trace: Vec::new(),
    };
    let core_origin = CoreQueryOrigin {
        source_id: QuerySourceId("nested/module".into()),
        span: CoreQuerySpan {
            start: CoreByteOffset(7),
            end: CoreByteOffset(12),
        },
        trace: Vec::new(),
    };
    let mut core_error = refused(RefusedKind::Invalid, 7, 12);
    core_error.origin = Some(core_origin);
    let converted = error(TEXT, CoreError::Uncompilable { error: core_error });
    assert!(matches!(
        converted,
        ValidateError::Located { origin: found, error }
            if found == origin && matches!(*error, ValidateError::Invalid { span, .. } if span.start.0 == 7 && span.end.0 == 12)
    ));
}

#[test]
fn an_unknown_kind_fragment_error_keeps_its_module_location() {
    let origin = QueryOrigin {
        source_id: "nested/module".into(),
        span: QuerySpan {
            start: ByteOffset(7),
            end: ByteOffset(12),
        },
        trace: Vec::new(),
    };
    let core_origin = CoreQueryOrigin {
        source_id: QuerySourceId("nested/module".into()),
        span: CoreQuerySpan {
            start: CoreByteOffset(7),
            end: CoreByteOffset(12),
        },
        trace: Vec::new(),
    };
    let converted = error(
        TEXT,
        CoreError::UnknownKind {
            reason: "unknown kind".to_owned(),
            origin: Some(core_origin),
        },
    );
    assert!(matches!(
        converted,
        ValidateError::Located { origin: found, error }
            if found == origin && matches!(*error, ValidateError::Invalid { span, .. } if span.start.0 == 7 && span.end.0 == 12)
    ));
}

/// What a row of the requests gives: the relations the caller declares, and the ones the core is expected to be told of.
type Declared = (Vec<HostDeclaration>, Vec<HostedRelation>);

/// The rows of the requests: the core's request is the one the command line builds for the same text when the caller declares
/// nothing (the command line declares none), and carries each relation the caller declares with its columns. The two edges hand the
/// one use case the same request, so they answer alike.
const REQUESTS: &[Case<Declared, bool, Failure>] = &[
    Case {
        name: CaseName("with_no_declaration_the_request_is_the_command_lines"),
        word: Word::Succeeds,
        input: || (Vec::new(), Vec::new()),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_declared_relation_is_handed_on_with_its_columns"),
        word: Word::Succeeds,
        input: || {
            (
                vec![HostDeclaration {
                    name: HostRelationName("host.placed".into()),
                    columns: vec![
                        HostColumn {
                            name: HostColumnName("path".into()),
                            kind: HostKind::File,
                        },
                        HostColumn {
                            name: HostColumnName("spec".into()),
                            kind: HostKind::Text,
                        },
                    ],
                }],
                vec![HostedRelation {
                    name: HostedName("host.placed".into()),
                    columns: vec![
                        HostedColumn {
                            name: ColumnLabel("path".into()),
                            kind: CellKind::File,
                        },
                        HostedColumn {
                            name: ColumnLabel("spec".into()),
                            kind: CellKind::Symbol,
                        },
                    ],
                }],
            )
        },
        expect: Expect::Value(true),
    },
];

/// Builds the core's request for `TEXT` and the declarations, and answers whether it is the one expected.
fn requested((declared, expected): Declared) -> bool {
    let query = RyoQlQuery::try_from(TEXT).expect("the text is not blank");
    request(ValidateRequest {
        observations: Vec::new(),
        query,
        hosted: declared,
    }) == CoreRequest {
        observations: Vec::new(),
        text: ryo_values::domain::program::types::QueryBundle::from(QueryText(TEXT.into())),
        hosted: expected,
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(REQUESTS, Word::Succeeds, |row| {
        Ok::<_, Failure>(requested(row))
    });
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

#[test]
fn query_bundle_retains_module_identity_and_exact_source() {
    use crate::domain::query::types::{QueryModuleCatalog, QueryModuleSource, RyoQlQuery};
    use std::sync::Arc;

    let module = QueryModuleSource::try_new("python/ruff/suppress", "fragment body\nλ🦀\n")
        .expect("module identity is nonblank");
    let modules = QueryModuleCatalog::try_new([module.clone()]).expect("unique module ids");
    let first = RyoQlQuery::with_modules("rules/F401", "root query\n", &modules)
        .expect("bundle identities are unique");
    let same_catalog = modules.clone();
    let same = RyoQlQuery::with_modules("rules/F401", "root query\n", &same_catalog)
        .expect("bundle identities are unique");
    let other_root = RyoQlQuery::with_modules("rules/F822", "another root\n", &modules)
        .expect("one immutable catalog supports different roots");
    let changed_modules = QueryModuleCatalog::try_new([QueryModuleSource::try_new(
        "python/ruff/suppress",
        "fragment body",
    )
    .expect("module identity is nonblank")])
    .expect("unique module ids");
    let changed_source = RyoQlQuery::with_modules("rules/F401", "root query\n", &changed_modules)
        .expect("bundle identities are unique");
    let independently_allocated = QueryModuleCatalog::try_new([QueryModuleSource::try_new(
        "python/ruff/suppress",
        "fragment body\nλ🦀\n",
    )
    .unwrap()])
    .unwrap();
    let same_contents =
        RyoQlQuery::with_modules("rules/F401", "root query\n", &independently_allocated).unwrap();
    assert_eq!(first, same_contents);
    assert_eq!(first.cmp(&same_contents), std::cmp::Ordering::Equal);
    let hashed = |query: &RyoQlQuery| {
        use std::hash::{Hash, Hasher};
        let mut digest = std::collections::hash_map::DefaultHasher::new();
        query.hash(&mut digest);
        digest.finish()
    };
    assert_eq!(hashed(&first), hashed(&same_contents));
    let empty = QueryModuleCatalog::try_new([]).unwrap();
    let empty_root = RyoQlQuery::with_modules("root/empty", "root query\n", &empty)
        .unwrap()
        .into_bundle();
    assert!(empty_root.modules.is_empty());
    assert_eq!(empty_root.root.id.0.as_ref(), "root/empty");
    assert_eq!(empty_root.root.text.as_ref(), "root query\n");
    assert_eq!(first, same);
    assert_ne!(first, changed_source);
    assert_eq!(first.root_text(), "root query\n");
    let original = first.clone().into_bundle();
    let cloned = first.into_bundle();
    let other_root = other_root.into_bundle();
    drop(modules);
    drop(same_catalog);
    assert!(Arc::ptr_eq(&original.modules, &cloned.modules));
    assert!(Arc::ptr_eq(&original.root.text, &cloned.root.text));
    assert!(Arc::ptr_eq(&original.modules, &other_root.modules));
    let id = ryo_values::domain::program::service::source_id(module.id()).unwrap();
    assert!(Arc::ptr_eq(
        &original.modules[&id].text,
        &cloned.modules[&id].text
    ));
    assert!(std::ptr::eq(
        module.text(),
        original.modules[&id].text.as_ref()
    ));
    let mut edited = original.clone();
    Arc::make_mut(&mut edited.modules)
        .get_mut(&id)
        .unwrap()
        .text = Arc::from("edited module");
    assert_eq!(original.modules[&id].text.as_ref(), "fragment body\nλ🦀\n");
    assert_eq!(
        other_root.modules[&id].text.as_ref(),
        "fragment body\nλ🦀\n"
    );
    assert_ne!(original, edited);
    assert!(!Arc::ptr_eq(&original.modules, &edited.modules));
    let _blank_module_error =
        QueryModuleSource::try_new(" \t", "body").expect_err("blank module ids must be rejected");
    let _duplicate_module_error = QueryModuleCatalog::try_new([module.clone(), module])
        .expect_err("duplicate module ids are rejected before root construction");
    let collision = QueryModuleCatalog::try_new([
        QueryModuleSource::try_new("rules/F401", "body").expect("module id is nonblank")
    ])
    .expect("unique module id");
    let _duplicate_id_error = RyoQlQuery::with_modules("rules/F401", "root query", &collision)
        .expect_err("a module cannot reuse the root source id");
}
