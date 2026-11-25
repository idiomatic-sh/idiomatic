//! The Python parse contract's scoped loan over the existing column walk.

use super::contract::ParseContract;
use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::syntax::{service::kind_of, types::Kind};
use crate::fixtures::columns::{ColumnFile, ColumnWalk};
use crate::inbound::contracts::walk::contract::WalkPorts;
use ryo::languages::{self as l, Fillers as _, node_table};
use ryo_relations::domain::relations::types::RowWriter;
use std::sync::OnceLock;

impl l::NumberedVisit for ParseContract {
    fn fields(&self) -> Option<&l::FieldNames> { Some(&walk_vocabulary().0) }

    fn projections(&self) -> &'static [l::Relation] {
        &[l::Relation::SemanticSyntax, l::Relation::SemanticChild]
    }

    fn open(&self) -> Box<dyn l::NumberedReader + '_> {
        let (_, words) = walk_vocabulary();
        Box::new(Reader {
            provider: self,
            walker: ColumnWalk::with_settings(words(), self.settings),
            tree: l::NodeTable::default(),
            words: Vec::new(),
            fact_words: words(),
        })
    }
}

struct Reader<'a> {
    provider: &'a ParseContract,
    walker: ColumnWalk,
    tree: l::NodeTable,
    words: Vec<l::Symbol>,
    fact_words: l::FileWords,
}

impl l::NumberedReader for Reader<'_> {
    fn visit(
        &mut self,
        request: l::ParseRequest<'_>,
        catalog: &[l::Relation],
        _named: &[&l::FileRelation],
        project: Option<&dyn l::FileProject>,
        symbols: &mut l::Symbols,
        consume: &mut dyn FnMut(l::NumberedFile<'_>, &mut l::Symbols),
    ) -> Result<(), l::ParseError> {
        let (fields, _) = walk_vocabulary();
        let Self { provider, walker, tree, words, fact_words } = self;
        l::facts::clear_words(fact_words);
        walker
            .visit_file(
                l::WalkRequest {
                    path: request.path,
                    file: request.file,
                    source: request.bytes,
                },
                l::RelationNeeds(catalog),
                tree,
                &mut |file| {
                    let tree = file.table;
                    let completeness = file.file.syntax.completeness;
                    words.clear();
                    if catalog.contains(&l::Relation::SyntaxTree) {
                        words.extend(node_table::kind_codes(tree).iter().enumerate().map(
                            |(at, &kind)| {
                                token_word(
                                    tree,
                                    request.bytes,
                                    symbols,
                                    l::Position(at as u32),
                                    l::KindBits(kind),
                                )
                            },
                        ));
                    }
                    let mut notes = l::Bytes::default();
                    if request.notes == l::NoteRequest::Wanted {
                        provider.notes.write(
                            &crate::domain::fact_notes::service::noted_walk(file, request.bytes),
                            &mut notes,
                        );
                    }
                    let mut facts = Facts {
                        provider,
                        request,
                        project,
                        file,
                    };
                    consume(
                        l::NumberedFile {
                            tree,
                            words,
                            fact_words,
                            fields,
                            syntax: completeness.syntax,
                            lexical: completeness.lexical,
                            notes: &notes,
                            facts: &mut facts,
                        },
                        symbols,
                    );
                },
            )
            .map_err(|_| l::ParseError::Unsupported)
    }
}

type FileWords = Box<dyn Fn() -> l::FileWords + Send + Sync>;

fn walk_vocabulary() -> &'static (l::FieldNames, FileWords) {
    static VOCABULARY: OnceLock<(l::FieldNames, FileWords)> = OnceLock::new();
    VOCABULARY.get_or_init(|| {
        let vocabulary = PythonWalk::<WalkPorts>::vocabulary();
        let base = l::facts::base_words(
            l::fixed_names::checked(&vocabulary.fixed.0).expect("walk vocabulary"),
            &[],
        );
        (
            vocabulary.fields,
            Box::new(move || l::facts::file_words(&base)),
        )
    })
}

struct Facts<'a, 'b, 'c> {
    provider: &'a ParseContract,
    request: l::ParseRequest<'a>,
    project: Option<&'a dyn l::FileProject>,
    file: &'a mut ColumnFile<'b, 'c>,
}

impl l::NumberedFacts for Facts<'_, '_, '_> {
    fn fill_columns(
        &mut self,
        relation: l::Relation,
        wanted: l::Wanted,
        words: &mut l::FileWords,
        columns: &mut l::FilledColumns,
    ) -> l::FactAnswer {
        self.file.fill(relation, wanted, words, columns)
    }

    fn project_structure(
        &mut self,
        relations: &[l::Relation],
        symbols: &mut l::Symbols,
        out: &mut dyn RowWriter,
    ) -> Result<(), l::ParseError> {
        for &relation in relations {
            if matches!(
                relation,
                l::Relation::SemanticSyntax | l::Relation::SemanticChild
            ) {
                self.semantic_rows(relation, symbols, out);
            }
        }
        Ok(())
    }

    fn named(
        &mut self,
        relations: &[&l::FileRelation],
        symbols: &mut l::Symbols,
    ) -> Vec<Result<l::FileRows, l::ParseError>> {
        self.provider
            .named_in(self.file, relations, self.request, symbols, self.project)
    }

    fn render(&mut self, request: l::RenderRequest) -> Result<l::RenderOutcome, l::ParseError> {
        Ok(self.file.render(request))
    }
}

fn token_word(
    tree: &l::NodeTable,
    bytes: l::ByteSlice<'_>,
    symbols: &mut l::Symbols,
    at: l::Position,
    kind: l::KindBits,
) -> l::Symbol {
    if !matches!(
        kind_of(kind),
        Kind::Token(_) | Kind::Extra(Extra::Whitespace)
    ) {
        return l::NO_SYMBOL;
    }
    let span = node_table::span_of(tree, at);
    let text = node_table::decoded_of(tree, at).or_else(|| {
        std::str::from_utf8(bytes.0.get(span.start.0 as usize..span.end.0 as usize)?)
            .ok()
            .map(l::NameText)
    });
    text.map_or(l::NO_SYMBOL, |text| l::intern_derived(symbols, bytes, text))
}

impl Facts<'_, '_, '_> {
    // The walk only embeds expressions it established were in type context.
    fn semantic_rows(
        &self,
        relation: l::Relation,
        symbols: &mut l::Symbols,
        out: &mut dyn RowWriter,
    ) {
        let tree = self.file.table;
        let request = self.request;
        let node = |at| l::node_id(request.file, at);
        let fields = &walk_vocabulary().0.0;
        for (at, &code) in node_table::kind_codes(tree).iter().enumerate() {
            let at = l::Position(at as u32);
            if relation == l::Relation::SemanticSyntax {
                let kind = l::KindBits(code);
                let name = crate::domain::syntax::service::kind_name(kind_of(kind));
                let kind_name = l::intern_derived(symbols, request.bytes, name);
                let span = node_table::span_of(tree, at);
                l::Rows::write(
                    out,
                    l::SemanticSyntaxRow {
                        id: node(at),
                        kind: kind_name,
                        text: token_word(tree, request.bytes, symbols, at, kind),
                        file: request.file,
                        span: l::Span {
                            start: span.start,
                            end: span.end,
                        },
                    },
                );
            } else {
                let parent = node_table::parent_of(tree, at);
                if parent == at {
                    continue;
                }
                let code = node_table::field_of(tree, at).0;
                let name = fields
                    .get(usize::from(code))
                    .copied()
                    .unwrap_or(l::NameText(""));
                l::Rows::write(
                    out,
                    l::SemanticChildRow {
                        parent: node(parent),
                        child: node(at),
                        ordinal: l::Integer(i64::from(node_table::ordinal_of(tree, at).0)),
                        field: l::intern_derived(symbols, request.bytes, name),
                        file: request.file,
                    },
                );
            }
        }
    }
}
