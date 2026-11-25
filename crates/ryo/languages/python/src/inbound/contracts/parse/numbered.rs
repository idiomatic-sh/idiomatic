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

    fn visit(
        &self,
        request: l::ParseRequest<'_>,
        catalog: &[l::Relation],
        _named: &[&l::FileRelation],
        project: Option<&dyn l::FileProject>,
        symbols: &mut l::Symbols,
        consume: &mut dyn FnMut(l::NumberedFile<'_>, &mut l::Symbols),
    ) -> Result<(), l::ParseError> {
        let (fields, words) = walk_vocabulary();
        let words = words();
        let mut walker = ColumnWalk::with_settings(words, self.settings);
        walker
            .visit_file(
                l::WalkRequest {
                    path: request.path,
                    file: request.file,
                    source: request.bytes,
                },
                l::RelationNeeds(catalog),
                &mut l::NodeTable::default(),
                &mut |file| {
                    let tree = file.table;
                    let completeness = file.file.syntax.completeness;
                    let mut words = Vec::new();
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
                        self.notes.write(
                            &crate::domain::fact_notes::service::noted_walk(file, request.bytes),
                            &mut notes,
                        );
                    }
                    let mut facts = Facts {
                        provider: self,
                        request,
                        project,
                        file,
                    };
                    consume(
                        l::NumberedFile {
                            tree,
                            words: &words,
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
    fn fill(
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
                continue;
            }
            write_relation(
                relation,
                self.file.file,
                self.file.words,
                self.request.bytes,
                symbols,
                out,
            );
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

// The typed catalog filler writes its existing rows. Translate its file-word handles
// once at the RowWriter boundary, without projecting another syntax table.
macro_rules! translated {
    ($symbol:expr, $words:ident, $bytes:expr, $symbols:ident, Symbol) => {
        if $symbol.0 != l::NO_SYMBOL.0 {
            $symbol = l::intern_derived(
                $symbols,
                $bytes,
                l::facts::text($words, l::CellWord(u64::from($symbol.0))),
            );
        }
    };
    ($symbol:expr, $words:ident, $bytes:expr, $symbols:ident, $kind:ident $(($convert:path))?) => {};
}

struct CatalogRows<'a, 'o> {
    out: &'a mut (dyn RowWriter + 'o),
    symbols: &'a mut l::Symbols,
    bytes: l::ByteSlice<'a>,
}

macro_rules! catalog_writer {
    ($module:ident, $relation:ident, $file:ident, $words:ident, $sink:ident
     item $item:ident, row { $(#[$doc:meta])* $row:ident }, column $column:tt, expected $expected:tt,
     class $class:ident, scope $scope:ident, notes $notes:ident, form NodeTable $nodes:tt, columns $columns:tt) => {};
    ($module:ident, $relation:ident, $file:ident, $words:ident, $sink:ident
     item $item:ident, row { $(#[$doc:meta])* $row:ident }, column $column:tt, expected $expected:tt,
     class $class:ident, scope $scope:ident, notes $notes:ident, form Filled $key:tt, size $size:ident, keys $keys:tt,
     columns { $( $(#[$column_doc:meta])* $field:ident $variant:ident : $ty:ty => $kind:ident $(($convert:path))? ; )* }) => {
        if $relation == l::Relation::$item {
            // The filler needs a mutable word pool and may add a word just before
            // writing each row. Queue typed rows until that borrow has ended.
            #[derive(Default)]
            struct Buffer {
                rows: Vec<l::$row>,
                partial: Option<(String, l::Missing)>,
            }
            impl l::Rows<l::$row> for Buffer {
                fn write(&mut self, row: l::$row) { self.rows.push(row); }
                fn partial(&mut self, cause: l::ReportText<'_>, missing: l::Missing) {
                    self.partial.get_or_insert_with(|| (cause.0.to_owned(), missing));
                }
                fn partial_at(&mut self, _: l::FileId, cause: l::ReportText<'_>, missing: l::Missing) {
                    self.partial(cause, missing);
                }
            }
            let mut buffer = Buffer::default();
            let fill = <PythonWalk<WalkPorts> as l::Fillers>::$module($file, $words, l::Wanted::Whole, &mut buffer);
            if let l::Fill::Absent(cause) = fill {
                if cause == l::AbsenceCause::NotYetRead {
                    l::Rows::<l::$row>::partial(
                        $sink.out, l::ReportText("catalog relation has no numbered filler"), l::Missing::Any
                    );
                }
                return;
            }
            let CatalogRows { out, bytes, symbols } = &mut $sink;
            #[allow(unused_mut, reason = "only rows with symbol columns are rewritten")]
            for mut row in buffer.rows {
                $( translated!(row.$field, $words, *bytes, symbols, $kind $(($convert))?); )*
                l::Rows::<l::$row>::write(*out, row);
            }
            if let Some((cause, missing)) = buffer.partial {
                l::Rows::<l::$row>::partial(*out, l::ReportText(&cause), missing);
            }
            return;
        }
    };
}

macro_rules! catalog_dispatch {
    ($( $group:ident { $( $item:ident $module:ident $row:ident $column:ident $( $qualifier:ident )* ; )* } )*) => {
        fn write_relation(relation: l::Relation, file: &mut crate::domain::file_view::types::PythonFile<'_>,
            words: &mut l::FileWords, bytes: l::ByteSlice<'_>, symbols: &mut l::Symbols, out: &mut dyn RowWriter) {
            let mut sink = CatalogRows { out, symbols, bytes };
            $( $( ryo_relations::$module!(catalog_writer, $module, relation, file, words, sink); )* )*
        }
    };
}

ryo_relations::each_relation!(catalog_dispatch);
