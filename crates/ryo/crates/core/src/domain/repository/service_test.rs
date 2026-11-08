//! The `repository` service's cases, on values alone.

use super::constants::COMPILED_PLAN_LABEL;
use super::service::{
    by_path, catalog_schema, configuration, paths_of, plan_identity, schema_bytes,
    source_selection_identity,
};
use super::types::{DigestColumn, DigestRelation, IdentityHasher};
use crate::domain::keeping::types::DigestInput;
use ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::labels::types::NameRef;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, CorpusInput, FileSize, Nanoseconds};
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::rule_requests::types::RuleParameters;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::RuleId;

/// What a row of the cases does with the files of a run.
#[derive(Clone, Copy)]
enum Scenario {
    /// Puts three files out of order in the order of their paths and reads their sizes.
    SortedByPath,
    /// Reads the paths of three files.
    Paths,
    /// Digests the same configuration twice, then with another rule, then with one more rule.
    Configured,
    /// The identity of an explicit source selection is exact and framed.
    SourceSelection,
    /// The names, kinds and column order of a catalog are part of its identity.
    SchemaIdentity,
    /// The current catalog differs from its older identities.
    LegacyCatalog,
    /// Adjacent names do not alias and the real catalog is stable.
    SchemaFrames,
    /// A plan's identity is the label then its derived `Hash` bytes, numbers at a fixed width: the same plan the same bytes, another
    /// plan of one more step other bytes.
    PlanIdentity,
}

/// What a row of the cases does with the files of a run.
type Input = Scenario;
/// Three numbers: a file stands for its size or its number, a path for its first byte; -1 says there was nothing.
type Value = [i64; 3];
type Failure = ();

/// A relation of the given columns, as the digest of the catalog sees it.
fn relation<'a>(name: &'a str, columns: &[(&'a str, CellKind)]) -> DigestRelation<'a> {
    DigestRelation {
        name: NameRef(name),
        columns: columns
            .iter()
            .map(|(name, kind)| DigestColumn {
                name: NameRef(*name),
                kind: *kind,
            })
            .collect(),
    }
}

fn entry(path: &str, size: u64) -> CorpusEntry {
    CorpusEntry {
        executable: Default::default(),
        path: FilePath(path.into()),
        size: FileSize(size),
        modified: Nanoseconds(0),
    }
}

fn files() -> Vec<CorpusInput> {
    vec![entry("c", 3), entry("a", 1), entry("b", 2)]
        .into_iter()
        .map(CorpusInput::Available)
        .collect()
}

fn rule(id: &str, query: &str) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(query.into())),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
        spelled: RuleParameters("".into()),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

fn files_in_order() -> Vec<CorpusInput> {
    let mut files = files();
    by_path(&mut files);
    files
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::SortedByPath => {
            let mut sorted = [0; 3];
            for (slot, file) in sorted.iter_mut().zip(files_in_order()) {
                let CorpusInput::Available(file) = file else {
                    panic!("fixture requires observed metadata");
                };
                *slot = i64::try_from(file.size.0).unwrap_or(i64::MAX);
            }
            sorted
        }
        Scenario::Paths => {
            let mut read = [0; 3];
            for (slot, path) in read.iter_mut().zip(paths_of(&files())) {
                *slot = i64::from(path.0.bytes().next().unwrap_or(0));
            }
            read
        }
        Scenario::SourceSelection => {
            use ryo_values::domain::descriptions::types::LanguageName;
            let encode = source_selection_identity;
            let python = LanguageName("python".into());
            let other = LanguageName("python-other".into());
            assert_eq!(encode(Some(&python)), encode(Some(&python)));
            assert_ne!(encode(None), encode(Some(&python)));
            assert_ne!(encode(Some(&python)), encode(Some(&other)));
            assert_ne!(encode(None), encode(Some(&LanguageName("".into()))));
            [1, 1, 1]
        }
        Scenario::SchemaIdentity => {
            let columns = [("file", CellKind::File), ("path", CellKind::Symbol)];
            let original = [relation("module", &columns)];
            let same = [relation("module", &columns)];
            assert_eq!(schema_bytes(&original), schema_bytes(&same));
            let changed = [
                [relation("renamed", &columns)],
                [relation(
                    "module",
                    &[("file", CellKind::File), ("renamed", CellKind::Symbol)],
                )],
                [relation(
                    "module",
                    &[("file", CellKind::File), ("path", CellKind::Number)],
                )],
                [relation(
                    "module",
                    &[("path", CellKind::Symbol), ("file", CellKind::File)],
                )],
                [relation(
                    "module",
                    &[
                        ("file", CellKind::File),
                        ("path", CellKind::Symbol),
                        ("extra", CellKind::Symbol),
                    ],
                )],
            ];
            for each in &changed {
                assert_ne!(schema_bytes(&original), schema_bytes(each));
            }
            [1, 1, 1]
        }
        Scenario::LegacyCatalog => {
            use ryo_relations::domain::relations::{
                service::{column_name, columns, kind},
                types::Relation,
            };
            let legacy = [relation(
                "module-identity",
                &[("file", CellKind::File), ("path", CellKind::Symbol)],
            )];
            let current = [DigestRelation {
                name: NameRef("module-identity"),
                columns: columns(Relation::ModuleIdentity)
                    .into_iter()
                    .map(|column| DigestColumn {
                        name: column_name(column),
                        kind: kind(column),
                    })
                    .collect(),
            }];
            assert_ne!(schema_bytes(&legacy), schema_bytes(&current));
            let previous = [relation(
                "module-identity",
                &[
                    ("file", CellKind::File),
                    ("path", CellKind::Symbol),
                    ("module-name", CellKind::Symbol),
                    ("package-name", CellKind::Symbol),
                ],
            )];
            assert_ne!(schema_bytes(&previous), schema_bytes(&current));
            [1, 1, 1]
        }
        Scenario::SchemaFrames => {
            assert_ne!(
                schema_bytes(&[relation("ab", &[("c", CellKind::Number)])]),
                schema_bytes(&[relation("a", &[("bc", CellKind::Number)])])
            );
            assert_eq!(catalog_schema(), catalog_schema());
            assert!(!catalog_schema().0.is_empty());
            [1, 1, 1]
        }
        Scenario::PlanIdentity => {
            use std::hash::Hasher as _;
            let mut length = IdentityHasher(DigestInput::default());
            length.write_usize(7);
            assert_eq!(length.0.0, 7_u64.to_le_bytes());
            let once = plan_identity(&QueryLanguageFake::selecting().plan);
            let again = plan_identity(&QueryLanguageFake::selecting().plan);
            let rewritten = plan_identity(&QueryLanguageFake::selecting_twice().plan);
            [
                i64::from(once == again),
                i64::from(once.0.starts_with(COMPILED_PLAN_LABEL.as_bytes())),
                i64::from(once == rewritten),
            ]
        }
        Scenario::Configured => {
            let digest = |rules: &[RuleSpec]| configuration(rules, &[]);
            let first = digest(&[rule("a", "select x")]);
            let again = digest(&[rule("a", "select x")]);
            let other_rule = digest(&[rule("a", "select y")]);
            let one_more = digest(&[rule("a", "select x"), rule("b", "select x")]);
            [
                i64::from(first == again),
                i64::from(first == other_rule),
                i64::from(first == one_more),
            ]
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("files_are_put_in_the_order_of_their_paths"),
        word: Word::Succeeds,
        input: || Scenario::SortedByPath,
        expect: Expect::Value([1, 2, 3]),
    },
    Case {
        name: CaseName("the_paths_are_read_in_the_order_of_the_files"),
        word: Word::Succeeds,
        input: || Scenario::Paths,
        expect: Expect::Value([99, 97, 98]),
    },
    Case {
        name: CaseName(
            "the_same_configuration_has_the_same_bytes_and_another_rule_or_one_more_other_bytes",
        ),
        word: Word::Succeeds,
        input: || Scenario::Configured,
        expect: Expect::Value([1, 0, 0]),
    },
    Case {
        name: CaseName("explicit_source_selection_identity_is_exact_and_framed"),
        word: Word::Succeeds,
        input: || Scenario::SourceSelection,
        expect: Expect::Value([1, 1, 1]),
    },
    Case {
        name: CaseName("names_kinds_and_column_order_are_part_of_snapshot_identity"),
        word: Word::Succeeds,
        input: || Scenario::SchemaIdentity,
        expect: Expect::Value([1, 1, 1]),
    },
    Case {
        name: CaseName("current_module_context_schema_rejects_its_legacy_identities"),
        word: Word::Succeeds,
        input: || Scenario::LegacyCatalog,
        expect: Expect::Value([1, 1, 1]),
    },
    Case {
        name: CaseName("frames_distinguish_adjacent_names_and_real_catalog_is_stable"),
        word: Word::Succeeds,
        input: || Scenario::SchemaFrames,
        expect: Expect::Value([1, 1, 1]),
    },
    Case {
        name: CaseName("a_plan_identity_is_its_label_then_its_hash_bytes_and_another_plan_differs"),
        word: Word::Succeeds,
        input: || Scenario::PlanIdentity,
        expect: Expect::Value([1, 1, 0]),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
