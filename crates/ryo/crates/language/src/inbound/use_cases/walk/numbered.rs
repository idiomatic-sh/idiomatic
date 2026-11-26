//! The one adapter from a language's walk to the numbered visit the one-path driver consumes: every language that implements
//! `WalkUseCase` is lent to the engine through it, and states only the items of `Fillers` the walk contract requires.

use super::interface::WalkUseCase;
use super::types::{FileVisit, RelationNeeds, WalkRequest};
use crate::domain::facts::service::{base_words, clear_words, file_words};
use crate::domain::facts::types::{
    BaseWords, FactAnswer, FactSource, FileFacts, FileWords, FilledColumns, NamedRequest, Wanted,
};
use crate::domain::fillers::types::{FieldNames, WalkKinds, WalkVocabulary};
use crate::domain::fixed_names::service::checked;
use crate::domain::node_table::service::{
    builder, children_of, decoded_of, field_of, is_token, kind_codes, own_end, ordinal_of, parent_of, span_of,
};
use crate::domain::node_table::types::NodeTable;
use crate::domain::semantics::types::{RenderOutcome, RenderRequest};
use crate::domain::symbols::service::intern_derived;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use crate::inbound::use_cases::parse::errors::ParseError;
use crate::inbound::use_cases::parse::named::{
    FileProject, FileRelation, FileRows, NumberedFacts, NumberedFile, NumberedReader, NumberedVisit,
};
use crate::inbound::use_cases::parse::types::{NoteRequest, ParseRequest};
use ryo_relations::domain::catalog::service::effective_extent;
use ryo_relations::domain::relations::types::NodeExtent;
use ryo_relations::domain::relations::collections::relations::semantic_child::item::SemanticChildRow;
use ryo_relations::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntaxRow;
use ryo_relations::domain::relations::constants::NO_SYMBOL;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{
    Bytes, FileId, Integer, KindBits, Position, Relation, RowWriter, Rows, Span,
};
use ryo_relations::domain::terms::types::{NameText, Symbol};

/// A language's walk, lent to the engine as numbered files: the kinds, fields and fixed names come from its vocabulary, the
/// token words from what its kinds spell, the completeness, the notes and the fills of a file from its `Fillers`.
///
/// # Examples
///
/// ```
/// use ryo_language::inbound::use_cases::walk::numbered::NumberedWalk;
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
///
/// let _walk = NumberedWalk::new(WalkUseCaseImpl::new());
/// ```
pub struct NumberedWalk<W: WalkUseCase> {
    walk: W,
    fields: FieldNames,
    kinds: WalkKinds,
    base: BaseWords,
}

impl<W: WalkUseCase> NumberedWalk<W> {
    /// The language's walk as the numbered visit.
    ///
    /// # Panics
    ///
    /// When the language's fixed names are not a valid set.
    #[must_use]
    pub fn new(walk: W) -> Self {
        let WalkVocabulary { kinds, fields, fixed } = W::vocabulary();
        let base = base_words(checked(&fixed.0).expect("walk vocabulary"), &[]);
        Self { walk, fields, kinds, base }
    }
}

impl<W: WalkUseCase> NumberedVisit for NumberedWalk<W> {
    fn fields(&self) -> Option<&FieldNames> {
        Some(&self.fields)
    }

    fn projections(&self) -> &'static [Relation] {
        &[Relation::SemanticSyntax, Relation::SemanticChild]
    }

    fn open(&self) -> Box<dyn NumberedReader + '_> {
        Box::new(Reader {
            walk: self,
            scratch: W::Scratch::default(),
            tree: NodeTable::default(),
            words: Vec::new(),
            fact_words: file_words(&self.base),
            named_words: file_words(&self.base),
            explicit: false,
        })
    }
}

struct Reader<'a, W: WalkUseCase> {
    walk: &'a NumberedWalk<W>,
    scratch: W::Scratch,
    tree: NodeTable,
    words: Vec<Symbol>,
    fact_words: FileWords,
    named_words: FileWords,
    /// Whether the request names the language, so a file is read whatever its path.
    explicit: bool,
}

/// The engine's loan of one walked file, made while the language's parse lives.
struct Lend<'a, W: WalkUseCase> {
    walk: &'a NumberedWalk<W>,
    request: ParseRequest<'a>,
    catalog: &'a [Relation],
    words: &'a mut Vec<Symbol>,
    fact_words: &'a mut FileWords,
    named_words: &'a mut FileWords,
    project: Option<&'a dyn FileProject>,
    symbols: &'a mut Symbols,
    consume: &'a mut dyn FnMut(NumberedFile<'_>, &mut Symbols),
}

impl<W: WalkUseCase> NumberedReader for Reader<'_, W> {
    fn explicit(&mut self, yes: bool) {
        self.explicit = yes;
    }

    fn visit(
        &mut self,
        request: ParseRequest<'_>,
        catalog: &[Relation],
        _named: &[&FileRelation],
        project: Option<&dyn FileProject>,
        symbols: &mut Symbols,
        consume: &mut dyn FnMut(NumberedFile<'_>, &mut Symbols),
    ) -> Result<(), ParseError> {
        let Self { walk, scratch, tree, words, fact_words, named_words, explicit } = self;
        clear_words(fact_words);
        clear_words(named_words);
        let mut lend =
            Lend { walk, request, catalog, words, fact_words, named_words, project, symbols, consume };
        let writer = &mut builder(tree, W::KIND_COUNT);
        walk.walk
            .walk(
                WalkRequest { path: request.path, file: request.file, source: request.bytes, explicit: *explicit },
                RelationNeeds(catalog),
                writer,
                scratch,
                &mut lend,
            )
            .map_err(|_| ParseError::Unsupported)
    }
}

impl<W: WalkUseCase> FileVisit for Lend<'_, W> {
    fn visit(&mut self, file: FileFacts<'_>) {
        let FileFacts { table, source, file: number, facts, .. } = file;
        let completeness = facts.completeness();
        self.words.clear();
        if self.catalog.contains(&Relation::SyntaxTree) {
            self.words.extend(kind_codes(table).iter().enumerate().map(|(at, &kind)| {
                token_word::<W>(table, source, self.symbols, Position(at as u32), KindBits(kind))
            }));
        }
        let mut notes = Bytes::default();
        if self.request.notes == NoteRequest::Wanted {
            facts.notes(table, source, &mut notes);
        }
        let mut numbered = Numbered {
            walk: self.walk,
            table,
            number,
            source,
            request: self.request,
            project: self.project,
            words: self.named_words,
            facts,
        };
        (self.consume)(
            NumberedFile {
                tree: table,
                words: self.words,
                fact_words: self.fact_words,
                fields: &self.walk.fields,
                syntax: completeness.syntax,
                lexical: completeness.lexical,
                notes: &notes,
                facts: &mut numbered,
            },
            self.symbols,
        );
    }
}

struct Numbered<'a, W: WalkUseCase> {
    walk: &'a NumberedWalk<W>,
    table: &'a NodeTable,
    number: FileId,
    source: ByteSlice<'a>,
    request: ParseRequest<'a>,
    project: Option<&'a dyn FileProject>,
    words: &'a mut FileWords,
    facts: &'a mut dyn FactSource,
}

impl<W: WalkUseCase> NumberedFacts for Numbered<'_, W> {
    fn fill_columns(
        &mut self,
        relation: Relation,
        wanted: Wanted,
        words: &mut FileWords,
        columns: &mut FilledColumns,
    ) -> FactAnswer {
        self.facts.fill(relation, wanted, words, columns)
    }

    fn project_structure(
        &mut self,
        relations: &[Relation],
        symbols: &mut Symbols,
        out: &mut dyn RowWriter,
    ) -> Result<(), ParseError> {
        for &relation in relations {
            if matches!(relation, Relation::SemanticSyntax | Relation::SemanticChild) {
                self.rows(relation, symbols, out);
            }
        }
        Ok(())
    }

    fn named(
        &mut self,
        relations: &[&FileRelation],
        symbols: &mut Symbols,
    ) -> Vec<Result<FileRows, ParseError>> {
        self.facts.named(NamedRequest {
            table: self.table,
            relations,
            request: self.request,
            project: self.project,
            words: self.words,
            symbols,
        })
    }

    fn fills_are_complete(&mut self) -> bool {
        self.facts.fills_are_complete()
    }

    fn render(&mut self, request: RenderRequest) -> Result<RenderOutcome, ParseError> {
        Ok(self.facts.render(request))
    }
}

impl<W: WalkUseCase> Numbered<'_, W> {
    /// The standard projection of the lent table: its kinds and texts, its parents and fields.
    fn rows(&self, relation: Relation, symbols: &mut Symbols, out: &mut dyn RowWriter) {
        let (table, file) = (self.table, self.number);
        // The catalog says whether the relation may include what the file embeds and the language says whether it does: the one rule
        // the engine's reads take too (`effective_extent`).
        let covered = if effective_extent(relation, self.walk.walk.includes_expansions(relation)) == Some(NodeExtent::Own) {
            usize::try_from(own_end(table).0).unwrap_or(usize::MAX)
        } else {
            usize::MAX
        };
        for (at, &code) in kind_codes(table).iter().enumerate().take(covered) {
            let at = Position(at as u32);
            if relation == Relation::SemanticSyntax {
                let kind = KindBits(code);
                let name = self.walk.kinds.0.get(usize::from(code)).copied().unwrap_or(NameText(""));
                let span = span_of(table, at);
                let kind_name = intern_derived(symbols, self.source, name);
                Rows::write(
                    out,
                    SemanticSyntaxRow {
                        id: node_id(file, at),
                        kind: kind_name,
                        text: token_word::<W>(table, self.source, symbols, at, kind),
                        file,
                        span: Span { start: span.start, end: span.end },
                    },
                );
            } else {
                let parent = parent_of(table, at);
                if parent == at {
                    continue;
                }
                let field = field_of(table, at).0;
                let name = self.walk.fields.0.get(usize::from(field)).copied().unwrap_or(NameText(""));
                Rows::write(
                    out,
                    SemanticChildRow {
                        parent: node_id(file, parent),
                        child: node_id(file, at),
                        ordinal: Integer(i64::from(ordinal_of(table, at).0)),
                        field: intern_derived(symbols, self.source, name),
                        file,
                    },
                );
            }
        }
    }
}

/// The word of a position: the written text of a leaf whose kind the language says it spells, else none.
///
/// The rule for text, for every provider: an element has text only if the provider's `spells(kind)` says yes AND the element is a
/// leaf, that is a token (the table marks it so) or an element with no children. An element with children never has text,
/// whatever its kind. A provider may write a leaf either as a token or as a childless element (Rust does the latter, to keep its
/// preorder numbering).
fn token_word<W: WalkUseCase>(
    table: &NodeTable,
    source: ByteSlice<'_>,
    symbols: &mut Symbols,
    at: Position,
    kind: KindBits,
) -> Symbol {
    // Only a leaf has text, whatever its kind says: an element the table marks as a token, or a node with no children (a language
    // may write its tokens as childless nodes). A node with children, of a spelled kind, has none.
    if !W::spells(kind) || !(is_token(table, at) || children_of(table, at).is_empty()) {
        return NO_SYMBOL;
    }
    let span = span_of(table, at);
    let text = decoded_of(table, at).or_else(|| {
        std::str::from_utf8(source.0.get(span.start.0 as usize..span.end.0 as usize)?).ok().map(NameText)
    });
    text.map_or(NO_SYMBOL, |text| intern_derived(symbols, source, text))
}

/// Names the struct alone: a language's walk has no `Debug` of its own.
impl<W: WalkUseCase> std::fmt::Debug for NumberedWalk<W> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct(std::any::type_name::<Self>()).finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fillers::fixtures::Walking;
    use crate::domain::node_table::service::{enter_node, finished, leave_node, token};
    use crate::domain::node_table::types::FieldCode;
    use ryo_relations::domain::relations::types::{Count, Offset};

    /// Nodes and a token of the same, spelled kind: a node with children has no text; a childless node and a token have theirs.
    #[test]
    fn only_a_leaf_has_text_whatever_the_kind() {
        let mut table = NodeTable::default();
        let mut writer = builder(&mut table, Count(4));
        // Kind 2 is the fixture's spelled kind.
        // Position 0 the root; 1 a node of the spelled kind with a child; 2 a childless node of that kind; 3 the token inside 1.
        enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
        enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(0));
        token(&mut writer, KindBits(2), FieldCode(0), Offset(0), Offset(3));
        leave_node(&mut writer, Offset(3));
        enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(0));
        leave_node(&mut writer, Offset(3));
        leave_node(&mut writer, Offset(3));
        let table = finished(&mut writer);
        let (source, mut symbols) = (ByteSlice(b"one"), Symbols::default());
        let mut word = |at| token_word::<Walking>(table, source, &mut symbols, Position(at), KindBits(2));
        assert_eq!(word(1), NO_SYMBOL, "a node with children has no text, whatever its kind");
        assert_ne!(word(2), NO_SYMBOL, "a childless node of a spelled kind is a leaf and has its written text");
        assert_ne!(word(3), NO_SYMBOL, "a token of the same kind has its written text");
    }
}
