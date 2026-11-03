// ignore: idiomatic.arch/file-length - the contract is one check run in order over the samples, and its steps (rows, flow, determinism) share the reads and the sink they run against.
//! What every implementation of the `parse` use case's trait must pass.

#![expect(
    clippy::expect_used,
    reason = "a contract run over the samples fails by panicking: the panic is the failure of the check that calls it"
)]

use super::super::errors::ParseError;
use super::super::types::ParseEngine;
use super::super::types::{ParseRequest, Recognition, SourceFile};
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest};

use super::super::service::{absence, build, read as read_file};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::elements::service::{end_of, positions};
use crate::domain::elements::types::Elements;
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::service::{all, columns, is_declaring, provenance_of};
use ryo_relations::domain::relations::types::Provenance;
use ryo_relations::domain::relations::types::{Bits, FileId, Missing, Relation, RelationDeclaring, Rows};
use crate::domain::symbols::types::Symbols;

/// A writer that keeps nothing: the contract looks at what the language answers, not at what it wrote.
#[derive(Default, Clone, Copy, Debug)]
pub struct Discard;

/// What a row of the contract is given: the path of a file.
type Input = &'static str;
/// What a row of the contract expects when it succeeds.
type Value = ();
/// What a row of the contract expects when it is refused.
type Failure = ParseError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_file_the_language_does_not_read_is_unsupported"),
    word: Word::Refuses,
    input: || "contract.not-a-language",
    expect: Expect::Error(ParseError::Unsupported),
}];

/// Runs the contract's rows against one implementation, in the crate that implements the trait.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(subject: &dyn ParseEngine) {
    run(CASES, Word::Refuses, |path| {
        let request = ParseRequest {
            path: SourcePath(path),
            file: FileId(0),
            bytes: ByteSlice(b"anything"),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        };
        assert!(
            subject.recognition(&request) == Recognition::Declines,
            "the language recognizes {path}, a file it refuses"
        );
        build(
            subject,
            request,
            &mut Elements::default(),
            &mut Symbols::default(),
        )
    });
}

/// A writer that keeps the rows it is given, each as the relation it is of and its numbers, one
/// row after another.
#[derive(Default, Debug)]
pub struct Collect {
    /// The relation of each row, in the order they were written.
    pub relations: Vec<Relation>,
    /// The numbers of every row, one row after another.
    pub words: Vec<u64>,
    /// The relations said to be partly read, each with its cause.
    pub partials: Vec<(Relation, String, Missing)>,
    /// The relations said to be partly read in one file of the run, each with the file and its cause.
    pub partials_at: Vec<(Relation, FileId, String)>,
    /// Reusable cause buffers after clear, so the collector adds no warmed allocations.
    pub cause_buffers: Vec<String>,
}

impl Collect {
    /// Forgets the rows, keeping the room.
    pub fn clear(&mut self) {
        self.relations.clear();
        self.words.clear();
        self.cause_buffers
            .extend(self.partials.drain(..).map(|(_, cause, _)| cause));
        self.cause_buffers
            .extend(self.partials_at.drain(..).map(|(_, _, cause)| cause));
    }

    /// Reuses a cause buffer retained from the previous file.
    fn cause(&mut self, text: &str) -> String {
        let mut buffer = self.cause_buffers.pop().unwrap_or_default();
        buffer.clear();
        buffer.push_str(text);
        buffer
    }

    /// How many rows were written of `relation`.
    #[must_use]
    pub fn count(&self, relation: Relation) -> usize {
        self.relations
            .iter()
            .filter(|each| **each == relation)
            .count()
    }

    /// The rows, each a relation and its numbers.
    #[must_use]
    pub fn rows(&self) -> Vec<(Relation, Vec<u64>)> {
        let mut rest = self.words.as_slice();
        self.relations
            .iter()
            .map(|relation| {
                let (row, after) = rest.split_at(columns(*relation).len());
                rest = after;
                (*relation, row.to_vec())
            })
            .collect()
    }

    /// Keeps one row: its relation, and its words after those of the rows kept before.
    fn keep(&mut self, relation: Relation, words: &[Bits]) {
        self.relations.push(relation);
        self.words.extend(words.iter().map(|word| word.0));
    }
}

/// What `Discard` and `Collect` write rows with, one `Rows` for each row type of the catalog, generated from its one list.
macro_rules! writers {
    (
        $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )*
    ) => {
        $( $(
            impl Rows<ryo_relations::domain::relations::collections::relations::$module::item::$row> for Discard {
                fn write(&mut self, _row: ryo_relations::domain::relations::collections::relations::$module::item::$row) {}

                fn partial(&mut self, _cause: ryo_relations::domain::relations::types::ReportText<'_>, _missing: Missing) {}

                fn partial_at(&mut self, _file: ryo_relations::domain::relations::types::FileId, _cause: ryo_relations::domain::relations::types::ReportText<'_>, _missing: Missing) {}
            }

            impl Rows<ryo_relations::domain::relations::collections::relations::$module::item::$row> for Collect {
                fn write(&mut self, row: ryo_relations::domain::relations::collections::relations::$module::item::$row) {
                    let words = <ryo_relations::domain::relations::collections::relations::$module::item::$variant as RelationsItem>::words(row);
                    self.keep(Relation::$variant, words.as_ref());
                }

                fn partial(&mut self, cause: ryo_relations::domain::relations::types::ReportText<'_>, missing: Missing) {
                    if !self.partials.iter().any(|(relation, each, how)| *relation == Relation::$variant && each == cause.0 && *how == missing) {
                        let cause = self.cause(cause.0);
                        self.partials.push((Relation::$variant, cause, missing));
                    }
                }

                fn partial_at(&mut self, file: ryo_relations::domain::relations::types::FileId, cause: ryo_relations::domain::relations::types::ReportText<'_>, _missing: Missing) {
                    if !self.partials_at.iter().any(|(relation, at, each)| *relation == Relation::$variant && *at == file && each == cause.0) {
                        let cause = self.cause(cause.0);
                        self.partials_at.push((Relation::$variant, file, cause));
                    }
                }
            }
        )* )*
    };
}

ryo_relations::each_relation!(writers);

/// What a sample row of the contract does with the samples.
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    /// Reads every sample and counts its rows.
    Declared,
    /// Reads a sample asking for the rows of one relation only.
    Wanted,
    /// Reads an empty file.
    Empty,
    /// Reads a file that is not well formed.
    Broken,
}

/// The sample rows of the contract: what reading a file must give, whatever the language.
const SAMPLE_ROWS: &[Case<Scenario, bool, ParseError>] = &[
    Case {
        name: CaseName("parses_a_sample_file_into_the_declared_relations"),
        word: Word::Succeeds,
        input: || Scenario::Declared,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("builds_only_the_wanted_columns"),
        word: Word::Succeeds,
        input: || Scenario::Wanted,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("empty_file_has_only_supported_root_rows_and_is_complete"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("relation_with_no_rows_for_this_file_is_not_missing"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("syntax_error_is_reported_not_fatal"),
        word: Word::Succeeds,
        input: || Scenario::Broken,
        expect: Expect::Value(true),
    },
];

/// Reads `bytes` as the file `sample` names, asking for `wanted`, and answers what was written.
fn read(
    subject: &dyn ParseEngine,
    sample: SourceFile,
    bytes: ByteSlice<'_>,
    wanted: &[Relation],
) -> Result<Collect, ParseError> {
    let mut out = Collect::default();
    let mut symbols = Symbols::default();
    let mut elements = Elements::default();
    read_file(
        subject,
        ParseRequest {
            path: sample.path,
            file: FileId(0),
            bytes,
            notes: NoteRequest::Unwanted,
            fields: if wanted.contains(&Relation::SemanticChild) {
                FieldRequest::Wanted
            } else {
                FieldRequest::Unwanted
            },
        },
        &mut elements,
        &mut symbols,
        wanted,
        &mut out,
    )?;
    assert!(
        embedded_roots_valid(&elements),
        "an embedded expression root must be a node attached directly to its source carrier"
    );
    if wanted.contains(&Relation::SemanticSyntax) {
        assert!(
            semantic_spans_valid(
                &out,
                bytes.0.len(),
                wanted.contains(&Relation::SourceLocation),
                |id| positions(&elements).any(|position| {
                    ryo_relations::domain::relations::service::node_id(FileId(0), position).0 == id
                        && crate::domain::elements::service::flags_at(&elements, position).0
                            & ryo_relations::domain::relations::constants::IMPLIED.0
                            != 0
                })
            ),
            "semantic occurrences must retain bounded physical spans and matching source coordinates"
        );
    }
    if wanted.contains(&Relation::SemanticSyntax)
        && wanted.contains(&Relation::SyntaxTree)
        && wanted.contains(&Relation::SemanticChild)
        && wanted.contains(&Relation::SyntaxChild)
        && crate::domain::expansions::service::embedded_expressions(&elements)
            .next()
            .is_none()
    {
        assert!(
            projection_preserves_native(&out),
            "without embedded expressions the semantic view must preserve every native row and edge exactly"
        );
        let mut fields: Vec<_> = out
            .rows()
            .into_iter()
            .filter_map(|(relation, row)| {
                (relation == Relation::SyntaxChild)
                    .then(|| row.get(3).copied())
                    .flatten()
            })
            .collect();
        fields.sort_unstable();
        fields.dedup();
        assert!(
            fields
                .into_iter()
                .all(|field| filtered_projection_valid(&out, field)),
            "native and semantic child field predicates must select the same edges"
        );
    }
    assert!(
        shape_representatives_valid(&out),
        "expression shape classes must name real representative occurrences in the same file"
    );
    for (relation, row) in out.rows() {
        if relation == Relation::Identifier {
            assert!(
                identifier_valid(&row, &symbols, bytes),
                "identifier must expose a known nonempty canonical name at a nonempty original byte span"
            );
        }
        if relation == Relation::QualifiedName {
            assert!(
                qualified_name_valid(&row, &symbols, bytes),
                "qualified-name must distinguish proven paths, unknown identities and builtin availability"
            );
        }
    }
    Ok(out)
}

/// Without embedded expressions, projection must not add, omit, or alter native syntax.
fn projection_preserves_native(out: &Collect) -> bool {
    [
        (Relation::SyntaxTree, Relation::SemanticSyntax),
        (Relation::SyntaxChild, Relation::SemanticChild),
    ]
    .into_iter()
    .all(|(native, semantic)| {
        let mut original: Vec<_> = out
            .rows()
            .into_iter()
            .filter_map(|(relation, row)| (relation == native).then_some(row))
            .collect();
        let mut projected: Vec<_> = out
            .rows()
            .into_iter()
            .filter_map(|(relation, row)| (relation == semantic).then_some(row))
            .collect();
        original.sort();
        projected.sort();
        original == projected
    })
}

/// Compare the consumer-visible result of a field equality predicate, not only row counts.
fn filtered_projection_valid(out: &Collect, field: u64) -> bool {
    let select = |wanted| {
        let mut selected: Vec<_> = out
            .rows()
            .into_iter()
            .filter_map(|(relation, row)| {
                (relation == wanted && row.get(3) == Some(&field)).then_some(row)
            })
            .collect();
        selected.sort();
        selected
    };
    select(Relation::SyntaxChild) == select(Relation::SemanticChild)
}

/// Logical token lengths cannot replace physical source offsets, even after decoding.
fn semantic_spans_valid(
    out: &Collect,
    length: usize,
    coordinates: bool,
    implied: impl Fn(u64) -> bool,
) -> bool {
    let rows = out.rows();
    rows.iter()
        .filter(|(relation, _)| *relation == Relation::SemanticSyntax)
        .all(|(_, row)| {
            let [id, _, _, _, span] = row.as_slice() else {
                return false;
            };
            let start = span >> 32_u32;
            let end = span & u64::from(u32::MAX);
            if start == u64::from(u32::MAX) && end == 0 {
                return implied(*id)
                    && !rows.iter().any(|(relation, coordinate)| {
                        *relation == Relation::SourceLocation && coordinate.first() == Some(id)
                    });
            }
            start <= end
                && end <= u64::try_from(length).unwrap_or(u64::MAX)
                && (!coordinates
                    || rows.iter().any(|(relation, coordinate)| {
                        *relation == Relation::SourceLocation
                            && coordinate.first() == Some(id)
                            && coordinate.get(2) == Some(span)
                    }))
        })
}

/// A normalized identifier remains a known spelling and an actual, nonempty source occurrence.
fn identifier_valid(row: &[u64], symbols: &Symbols, source: ByteSlice<'_>) -> bool {
    let [_, name, _, span] = row else {
        return false;
    };
    let Ok(name) = u32::try_from(*name) else {
        return false;
    };
    let start = span >> 32_u32;
    let end = span & u64::from(u32::MAX);
    crate::domain::symbols::service::text_of(
        symbols,
        source,
        ryo_relations::domain::terms::types::Symbol(name),
    )
    .is_some_and(|name| !name.0.is_empty())
        && start < end
        && end <= u64::try_from(source.0.len()).unwrap_or(u64::MAX)
}

/// Every provider must distinguish a proven lexical path from an unknown value and explicit builtin availability.
fn qualified_name_valid(row: &[u64], symbols: &Symbols, source: ByteSlice<'_>) -> bool {
    use crate::domain::symbols::service::text_of;
    use ryo_relations::domain::terms::{service::unknown_reason, types::Symbol};
    let [_, path, provenance, major, minor, _, _] = row else {
        return false;
    };
    let (Ok(path), Ok(provenance)) = (u32::try_from(*path), u32::try_from(*provenance)) else {
        return false;
    };
    let path = Symbol(path);
    let named = text_of(symbols, source, path).is_some_and(|name| !name.0.is_empty());
    match text_of(symbols, source, Symbol(provenance)).map(|text| text.0) {
        Some("builtin") => {
            named && *major > 0 && i64::try_from(*major).is_ok() && i64::try_from(*minor).is_ok()
        }
        Some("import") => named && *major == 0 && *minor == 0,
        Some("local" | "unresolved" | "ambiguous") => {
            unknown_reason(path).is_some() && *major == 0 && *minor == 0
        }
        _ => false,
    }
}

/// Runs the sample rows against one implementation.
fn rows(subject: &dyn ParseEngine, samples: &[SourceFile]) {
    let Some(first) = samples.first().copied() else {
        return;
    };
    run(SAMPLE_ROWS, Word::Succeeds, |scenario| match scenario {
        Scenario::Declared => {
            let mut held = true;
            for sample in samples {
                let out = read(subject, *sample, sample.bytes, all())?;
                held &= all().iter().any(|relation| out.count(*relation) > 0);
            }
            Ok(held)
        }
        Scenario::Wanted => {
            // Asking for one relation the language reads gives rows of that relation only; which relation it is is the
            // language's: the first it answers `Reads`.
            let Some(asked) = all().iter().copied().find(|relation| {
                provenance_of(*relation) != Provenance::Engine
                    && provenance_of(*relation) != Provenance::Manifests
                    && absence(subject, *relation).is_none()
            }) else {
                return Ok(true);
            };
            let out = read(subject, first, first.bytes, &[asked])?;
            Ok(all()
                .iter()
                .filter(|relation| **relation != asked)
                .all(|relation| out.count(*relation) == 0))
        }
        Scenario::Empty => {
            let out = read(subject, first, ByteSlice(b""), all())?;
            Ok(all()
                .iter()
                .filter(|relation| is_declaring(**relation) == RelationDeclaring::Declares)
                // An empty file still has its module scope and lexical root interval when read.
                .all(|relation| {
                    let count = out.count(*relation);
                    if subject.absence_of(*relation).is_some() {
                        return count == 0;
                    }
                    if matches!(*relation, Relation::Scope | Relation::IndentRegion) {
                        return count == 1;
                    }
                    // Compiler features describe module configuration even without source constructs.
                    if *relation == Relation::CompilerFeature {
                        return out.rows().iter().all(|(feature_relation, row)| {
                            *feature_relation != Relation::CompilerFeature
                                || (row.len() == columns(Relation::CompilerFeature).len()
                                    && row.first().is_some_and(|file| *file == 0)
                                    && row.get(2).is_some_and(|flag| *flag <= 1))
                        });
                    }
                    // Versioned implicit-symbol rows describe the environment catalog, not source text.
                    if *relation == Relation::ImplicitSymbol {
                        return out.rows().iter().all(|(catalog_relation, row)| {
                            *catalog_relation != Relation::ImplicitSymbol
                                || (row.len() == columns(Relation::ImplicitSymbol).len()
                                    && row.get(3).is_some_and(|major| *major > 0))
                        });
                    }
                    count == 0
                }))
        }
        Scenario::Broken => {
            // The broken input is the language's own: the input of its `syntax_error` fixture. A language that answers
            // `syntax_error` `NotInThisLanguage` has no such input, and skips this row.
            let Some(broken) = samples
                .iter()
                .find(|sample| sample.path.starts_with("relations/syntax_error/"))
            else {
                return Ok(true);
            };
            if absence(subject, Relation::SyntaxError).is_some() {
                return Ok(true);
            }
            let out = read(subject, *broken, broken.bytes, all())?;
            Ok(out.count(Relation::SyntaxError) > 0)
        }
    });
}

/// Whether `elements` is a pre-order tree: every element's subtree lies inside the table and
/// inside its parent's, and the first element is its own parent.
fn well_formed(elements: &Elements) -> bool {
    let count = end_of(elements);
    positions(elements).all(|position| {
        let end = crate::domain::elements::service::last_at(elements, position);
        let parent = crate::domain::elements::service::parent_at(elements, position);
        end < count
            && (parent == position
                || (parent < position
                    && end <= crate::domain::elements::service::last_at(elements, parent)))
    })
}

/// Reads every sample, each as the file numbered as it says, in the order given, on `workers` threads that each own a table and a
/// writer; the rows of every file, sorted.
fn rows_on(
    subject: &dyn ParseEngine,
    samples: &[(usize, SourceFile)],
    workers: usize,
) -> Vec<(Relation, Vec<u64>)> {
    let mut rows: Vec<(Relation, Vec<u64>)> = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..workers)
            .map(|worker| {
                scope.spawn(move || {
                    let (mut table, mut symbols, mut out) =
                        (Elements::default(), Symbols::default(), Collect::default());
                    for (place, sample) in samples
                        .iter()
                        .enumerate()
                        .filter(|(at, _)| at % workers == worker)
                        .map(|(_, (place, sample))| (*place, *sample))
                    {
                        let request = ParseRequest {
                            path: sample.path,
                            file: FileId(u32::try_from(place).unwrap_or(0)),
                            bytes: sample.bytes,
                            notes: NoteRequest::Unwanted,
                            fields: FieldRequest::Unwanted,
                        };
                        read_file(subject, request, &mut table, &mut symbols, all(), &mut out)
                            .expect("a sample is a file the language reads");
                    }
                    out.rows()
                })
            })
            .collect();
        threads
            .into_iter()
            .flat_map(|thread| thread.join().expect("a worker finishes"))
            .collect()
    });
    rows.sort();
    rows
}

/// Runs the flow rows against one implementation: every sample gives a well-formed pre-order
/// table, and the same rows whether one worker reads the samples or ten.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn flow(subject: &dyn ParseEngine, samples: &[SourceFile]) {
    for sample in samples {
        let (mut table, mut symbols) = (Elements::default(), Symbols::default());
        let request = ParseRequest {
            path: sample.path,
            file: FileId(0),
            bytes: sample.bytes,
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        };
        read_file(
            subject,
            request,
            &mut table,
            &mut symbols,
            all(),
            &mut Collect::default(),
        )
        .expect("a sample is a file the language reads");
        assert!(
            well_formed(&table),
            "the table of {} is not a well-formed pre-order tree",
            sample.path
        );
    }
    rows(subject, samples);
    // The module of a file is a scope that encloses itself: of the scopes of each file, exactly one is its own parent (parent == child), and every other has a
    // parent that is a scope of the file.
    if subject.absence_of(Relation::Scope).is_none() {
        for sample in samples {
            let out = read(subject, *sample, sample.bytes, &[Relation::Scope])
                .expect("a sample is a file the language reads");
            let scopes: Vec<(u64, u64)> = out
                .rows()
                .into_iter()
                .filter(|(relation, _)| *relation == Relation::Scope)
                .filter_map(|(_, row)| match row.as_slice() {
                    [scope, parent, ..] => Some((*scope, *parent)),
                    _ => None,
                })
                .collect();
            let self_parented = scopes
                .iter()
                .filter(|(parent, child)| parent == child)
                .count();
            assert_eq!(
                self_parented, 1,
                "{} has {self_parented} rows that are their own parent, and every file has exactly one: its module",
                sample.path
            );
            assert!(
                scopes
                    .iter()
                    .all(|(parent, _)| scopes.iter().any(|(_, child)| child == parent)),
                "a scope of {} has a parent that is no scope of the file",
                sample.path
            );
        }
    }
    // The rows of a file are the same whichever worker reads it and whichever files it read before: the samples numbered as they
    // are, read in order by one worker, in reverse by three, and rotated by ten.
    let numbered: Vec<(usize, SourceFile)> = samples.iter().copied().enumerate().collect();
    let mut reversed = numbered.clone();
    reversed.reverse();
    let mut rotated = numbered.clone();
    rotated.rotate_left(numbered.len() / 2);
    let alone = rows_on(subject, &numbered, 1);
    assert!(!alone.is_empty(), "the samples give no rows to compare");
    assert_eq!(
        alone,
        rows_on(subject, &numbered, 10),
        "one worker and ten give different rows"
    );
    assert_eq!(
        alone,
        rows_on(subject, &reversed, 3),
        "the files read in reverse by three workers give different rows"
    );
    assert_eq!(
        alone,
        rows_on(subject, &rotated, 10),
        "the files read in another order by ten workers give different rows"
    );
}

#[cfg(test)]
#[path = "contract_test.rs"]
mod tests;

/// Unknown or cross-file class sentinels cannot silently compare equal as structural identities.
fn shape_representatives_valid(out: &Collect) -> bool {
    use ryo_relations::domain::relations::{service::node_file, types::NodeId};
    let representatives: std::collections::BTreeSet<_> = out
        .rows()
        .into_iter()
        .filter_map(|(relation, row)| {
            if relation != Relation::ExpressionShape {
                return None;
            }
            let [id, shape, file, _] = row.as_slice() else {
                return None;
            };
            (*id == *shape).then_some((*file, *shape))
        })
        .collect();
    out.rows()
        .into_iter()
        .filter(|(relation, _)| *relation == Relation::ExpressionShape)
        .all(|(_, row)| {
            let [id, shape, file, _] = row.as_slice() else {
                return false;
            };
            let Ok(file) = u32::try_from(*file) else {
                return false;
            };
            node_file(NodeId(*id)) == FileId(file)
                && node_file(NodeId(*shape)) == FileId(file)
                && representatives.contains(&(u64::from(file), *shape))
        })
}

/// Token trivia cannot masquerade as the root of an independently parsed expression.
fn embedded_roots_valid(elements: &Elements) -> bool {
    use crate::domain::elements::service::{flags_at, parent_at};
    crate::domain::expansions::service::embedded_expressions(elements).all(|embedded| {
        embedded.root < embedded.end
            && flags_at(elements, embedded.root).0 & ryo_relations::domain::relations::constants::TOKEN.0
                == 0
            && parent_at(elements, embedded.root) == embedded.host
    })
}
