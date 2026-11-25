//! The fillers' cases: over a corpus of Rust sources, every relation the language answers has the rows its reader writes today, the
//! filler writing the same rows in the same words (a name is the text it spells), in any order. The first differing row is shown.

use ryo::languages::{
    self as l, ByteSlice, CatalogRelation, CellKind, CellWord, FileId, FileVisit, FileFacts, FilledColumns, NodeTable, RelationNeeds,
    Relation, SourcePath, Walk, WalkRequest, contract, facts, node_table as t,
};
use std::collections::BTreeMap;

/// The relations the language fills, each with the kinds of its columns.
fn answered() -> Vec<(Relation, &'static [CellKind])> {
    macro_rules! relations {
        ($( $variant:ident ),* $(,)?) => { vec![ $( (Relation::$variant, <l::$variant as CatalogRelation>::kinds()) ),* ] };
    }
    relations!(
        Annotates, Assigns, AttributePath, CallArgument, Comment, Decorates, Defines, ExpressionShape, Function, Inherits, Mention,
        NumericLiteral, Parameter, Reference, Resolves, Scope, SourceLocation, StaticCall, StaticImport, StringLiteral, SyntaxError,
        Visibility,
    )
}

/// A row, shown by its cells: a name by its text, everything else by its words.
type Shown = Vec<String>;

/// The rows of every relation today's reading writes for the bytes.
fn read_rows(bytes: &[u8]) -> BTreeMap<String, Vec<Shown>> {
    let engine = crate::linked().parse_engine();
    let (mut elements, mut symbols) = (l::Elements::default(), l::Symbols::default());
    let mut collect = contract::Collect::default();
    let wanted: Vec<Relation> = answered().iter().map(|(relation, _)| *relation).collect();
    contract::read(
        &*engine,
        l::ParseRequest {
            path: SourcePath("src/a.rs"),
            file: FileId(0),
            bytes: ByteSlice(bytes),
            notes: l::NoteRequest::Unwanted,
            fields: l::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
        &wanted,
        &mut collect,
    )
    .expect("a Rust source is read");
    let kinds: BTreeMap<String, &[CellKind]> = answered().into_iter().map(|(relation, kinds)| (format!("{relation:?}"), kinds)).collect();
    let mut by: BTreeMap<String, Vec<Shown>> = kinds.keys().map(|name| (name.clone(), Vec::new())).collect();
    for (relation, words) in collect.rows() {
        let name = format!("{relation:?}");
        let columns = kinds[&name];
        let row = columns
            .iter()
            .zip(&words)
            .map(|(kind, word)| match kind {
                CellKind::Symbol => match l::text_of(&symbols, ByteSlice(bytes), l::Symbol(u32::try_from(*word).unwrap_or(u32::MAX))) {
                    Some(text) => format!("text:{}", text.0),
                    None => format!("raw:{word}"),
                },
                _ => format!("{word}"),
            })
            .collect();
        by.get_mut(&name).expect("an answered relation").push(row);
    }
    by.values_mut().for_each(|rows| rows.sort_unstable());
    by
}

/// Keeps the rows the fillers write for the lent file.
struct Fill(BTreeMap<String, Vec<Shown>>);

impl FileVisit for Fill {
    fn visit(&mut self, mut file: FileFacts<'_>) {
        let mut words = facts::file_words(&facts::base_words(Default::default(), &[]));
        for (relation, kinds) in answered() {
            let mut columns = FilledColumns::default();
            let _answer = file.facts.fill(relation, l::Wanted::Whole, &mut words, &mut columns);
            let count = columns.0.first().map_or(0, |column| column.0.len());
            let mut rows: Vec<Shown> = (0..count)
                .map(|row| {
                    kinds
                        .iter()
                        .zip(&columns.0)
                        .map(|(kind, column)| {
                            let word = column.0.get(row).copied().unwrap_or(CellWord(0)).0;
                            match kind {
                                CellKind::Symbol => {
                                    if l::term_text(l::Symbol(u32::try_from(word).unwrap_or(u32::MAX))).is_some()
                                        || word >= u64::from(l::unknown_symbol(l::UnknownReason(0)).0)
                                        || word == u64::from(l::NO_SYMBOL.0)
                                    {
                                        format!("raw:{word}")
                                    } else {
                                        format!("text:{}", facts::text(&words, CellWord(word)).0)
                                    }
                                }
                                _ => format!("{word}"),
                            }
                        })
                        .collect()
                })
                .collect();
            rows.sort_unstable();
            self.0.insert(format!("{relation:?}"), rows);
        }
    }
}

/// The rows of every relation the filler writes for the bytes.
fn fill_rows<W: Walk>(walk: &W, bytes: &[u8]) -> BTreeMap<String, Vec<Shown>> {
    let mut table = NodeTable::default();
    let mut builder = t::builder(&mut table, W::KIND_COUNT);
    let mut fill = Fill(BTreeMap::new());
    let mut scratch = W::Scratch::default();
    walk.walk(
        WalkRequest { path: SourcePath("src/a.rs"), file: FileId(0), source: ByteSlice(bytes) },
        RelationNeeds(&[]),
        &mut builder,
        &mut scratch,
        &mut fill,
    )
    .expect("a Rust source is walked");
    fill.0
}

/// What differs between the reader's rows and the filler's for one source: one line for each relation that differs.
fn differences(bytes: &[u8]) -> String {
    let (old, new) = (read_rows(bytes), fill_rows(&crate::walk(), bytes));
    let mut found = String::new();
    for (relation, rows) in &old {
        let filled = &new[relation];
        if rows != filled {
            let at = rows.iter().zip(filled).position(|(a, b)| a != b).unwrap_or(rows.len().min(filled.len()));
            found.push_str(&format!(
                "{relation}: {} rows by the reader, {} by the filler; first difference at {at}: reader {:?} filler {:?}\n",
                rows.len(), filled.len(), rows.get(at), filled.get(at)
            ));
        }
    }
    found
}

/// Real sources of the language's own crate, and corners of the grammar.
const SOURCES: &[&str] = &[
    include_str!("../../../domain/syntax/service.rs"),
    include_str!("../../../domain/references/service.rs"),
    include_str!("../../../domain/numeric_literals/service.rs"),
    include_str!("../../../domain/declarations/service.rs"),
    include_str!("../../../domain/imports/service.rs"),
    include_str!("../../../domain/attribute_paths/service.rs"),
    include_str!("../../../domain/visibilities/service.rs"),
    include_str!("../../../domain/shapes/service.rs"),
    include_str!("../../../outbound/syntax/adapters/ra_ap_syntax/adapter.rs"),
    "pub(crate) struct S { pub a: u8, b: i64 }\nimpl S { pub fn f(&self, x: u32) -> u32 { x + 1 } }\ntrait T: Clone { fn g(); }\n",
    "use std::{io, fmt::Write as _};\n#[derive(Debug)] #[cfg(test)] fn k<'a>(v: &'a str) { let s = \"a\\nb\"; let n = 0x1f_u8 + 1.5e3 as u8; println!(\"{}\", vec![1u8, 2]); }\n",
    "fn broken( {",
];

#[test]
fn every_relation_has_the_rows_its_reader_writes() {
    for source in SOURCES {
        assert_eq!(differences(source.as_bytes()), "", "source {:?}", source.chars().take(60).collect::<String>());
    }
}

/// Every relation the fillers answer for: filled or absent.
const FILLERS: [Relation; 75] = [
    Relation::Comment,
    Relation::Defines,
    Relation::Prose,
    Relation::Reference,
    Relation::Scope,
    Relation::SourceLocation,
    Relation::StringLiteral,
    Relation::SyntaxError,
    Relation::BindingEvent,
    Relation::BindingResolution,
    Relation::BindingUse,
    Relation::BraceFormatField,
    Relation::BraceFormatParse,
    Relation::BytesLiteral,
    Relation::CallArgument,
    Relation::CompilerFeature,
    Relation::Decorates,
    Relation::EmbeddedExpression,
    Relation::FormatField,
    Relation::FormatParse,
    Relation::FormatSpecification,
    Relation::FunctionTerminal,
    Relation::Identifier,
    Relation::ImplicitSymbol,
    Relation::IndentRegion,
    Relation::InterpolatedStringLiteral,
    Relation::LanguageTarget,
    Relation::LiteralExpressionParse,
    Relation::LiteralProperties,
    Relation::NumericLiteral,
    Relation::Parameter,
    Relation::QualifiedName,
    Relation::RawFormatField,
    Relation::StandardLibraryImport,
    Relation::StaticImport,
    Relation::BindingRead,
    Relation::BindingShadow,
    Relation::ConstantKey,
    Relation::ExpressionShape,
    Relation::ExpressionType,
    Relation::Function,
    Relation::Inherits,
    Relation::LookupScope,
    Relation::RawFormatSegment,
    Relation::Resolves,
    Relation::TokenLink,
    Relation::AnalyzedSource,
    Relation::Annotates,
    Relation::Assigns,
    Relation::AttributePath,
    Relation::Directive,
    Relation::Entry,
    Relation::FilePackage,
    Relation::FilesystemExecutable,
    Relation::FilesystemFile,
    Relation::FilesystemLineCount,
    Relation::FilesystemReadFailure,
    Relation::FlowBlock,
    Relation::FlowEdge,
    Relation::FlowMember,
    Relation::FlowStep,
    Relation::ManifestDiagnostic,
    Relation::Mapping,
    Relation::Mention,
    Relation::ModuleIdentity,
    Relation::PackageDependency,
    Relation::PatternChild,
    Relation::PatternParse,
    Relation::PatternSyntax,
    Relation::ProjectNamespaceMembership,
    Relation::SequenceElement,
    Relation::SourceDependency,
    Relation::StaticCall,
    Relation::Visibility,
    Relation::WorkspaceMember,
];

/// The walk's cause for a relation, asked of a file's fillers: nothing when it fills the relation.
fn walked_cause(relation: Relation) -> Option<l::AbsenceCause> {
    walked_cause_with(&crate::walk(), relation)
}

fn walked_cause_with<W: Walk>(walk: &W, relation: Relation) -> Option<l::AbsenceCause> {
    struct Ask(Relation, Option<l::AbsenceCause>);
    impl FileVisit for Ask {
        fn visit(&mut self, mut lent: FileFacts<'_>) {
            let mut words = facts::file_words(&facts::base_words(Default::default(), &[]));
            self.1 = match lent.facts.fill(self.0, l::Wanted::Whole, &mut words, &mut FilledColumns::default()) {
                l::FactAnswer::Refused(cause) => Some(cause),
                _ => None,
            };
        }
    }
    let mut table = NodeTable::default();
    let mut builder = t::builder(&mut table, W::KIND_COUNT);
    let mut ask = Ask(relation, None);
    let mut scratch = W::Scratch::default();
    walk_with(walk, &mut builder, &mut scratch, &mut ask);
    ask.1
}

fn walk_with<W: Walk>(walk: &W, builder: &mut l::NodeTableBuilder<'_>, scratch: &mut W::Scratch, visit: &mut dyn FileVisit) {
    walk.walk(
        WalkRequest { path: SourcePath("a.rs"), file: FileId(0), source: ByteSlice(b"fn f() {}") },
        RelationNeeds(&[]),
        builder,
        scratch,
        visit,
    )
    .expect("the walk reads the source");
}

#[test]
fn each_relation_the_walk_does_not_fill_has_the_cause_todays_contracts_give() {
    use std::fmt::Write as _;
    let language = crate::linked();
    let (parse, project) = (language.parse_engine(), language.project());
    let mut report = String::new();
    for relation in FILLERS {
        let old = parse.absence_of(relation).or_else(|| project.absence_of(relation)).or(matches!(
            relation,
            Relation::FilesystemExecutable | Relation::FilesystemFile | Relation::FilesystemLineCount | Relation::FilesystemReadFailure
        )
        .then_some(l::AbsenceCause::NotYetRead));
        let new = walked_cause(relation);
        if old != new {
            let _ = writeln!(report, "{relation:?}: today {old:?}, walk {new:?}");
        }
    }
    assert!(report.is_empty(), "{report}");
}
