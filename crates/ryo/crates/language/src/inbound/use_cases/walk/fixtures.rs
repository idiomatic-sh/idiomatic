//! The test support of the `walk` use case: the fillers concept's fixture language walked over a file of words (`*.walk`), one node
//! over one token per word, a diagnostic for each `!`, an implied node for each `~` (a word the writer left out). A word that ends
//! in `:` fills the field `label`. A word written `<inner>` is a quote: a node over its token, whose inner text is read again as
//! an embedded segment (a sentence over one word), after the file's own tree. A file whose bytes are `FAILING` makes its parser
//! fail. Beside the walk, what the feature's steps say of a lent file: what a fill named in words answered, the kind of its
//! links, and what a growth run held.

use super::errors::WalkError;
use super::interface::WalkUseCase;
use super::types::{FileVisit, RelationNeeds, WalkRequest};
use crate::domain::facts::service::{base_words, file_words, fixed, text};
use crate::domain::fact_tables::reading::service::{cells_of, every_row, indexed_by, rows_of};
use crate::domain::fact_tables::service::{answer_of, begun, request_of, table_of};
use crate::domain::fact_tables::types::{FactKey, FactSet};
use crate::domain::facts::types::{CellWord, FactAnswer, FileFacts, FileWords};
use crate::domain::fillers::fixtures::growth::GrowthReport;
use crate::domain::fillers::fixtures::{NEXT_TOKEN, Walking, WalkingFile, weakness};
use crate::domain::fillers::service::file_source;
use crate::domain::fillers::types::Fillers as _;
use crate::domain::fixed_names::errors::FixedNamesError;
use crate::domain::fixed_names::service::checked;
use crate::domain::node_table::service::{
    diagnose, embed, enter_implied_node, enter_node, finished, leave_node, token,
};
use crate::domain::node_table::types::{EmbeddedText, FieldCode, NodeTableBuilder};
use ryo_relations::domain::elements::types::EmbeddedMapping;
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::collections::relations::token_link::item::TokenLinkColumn;
use ryo_relations::domain::relations::service::{column_named, name, relation_named};
use ryo_relations::domain::relations::types::{Column, Relation};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset, Position};
use ryo_relations::domain::terms::types::NameText;

/// The bytes the fixture's parser fails on.
pub const FAILING: &[u8] = b"\xff";

/// The kind of the fixture's nodes: the sentence, and a word the writer left out.
const SENTENCE: KindBits = KindBits(1);

/// The kind of a word.
pub const WORD: KindBits = KindBits(2);

/// The kind of a quote: the node over a word written `<inner>`.
pub const QUOTE: KindBits = KindBits(3);

/// The field a word that ends in `:` fills: place 1 of the fixture's field names.
pub const LABEL: FieldCode = FieldCode(1);

/// A quote the walk met: its node, and where its inner text starts and ends in the file.
#[derive(Clone, Copy, Debug)]
struct Quoted {
    /// The quote's node.
    host: Position,
    /// The first byte of the inner text.
    from: usize,
    /// The byte after the inner text.
    to: usize,
}

impl WalkUseCase for Walking {
    type Scratch = ();
    const KIND_COUNT: Count = Count(4);

    fn walk(
        &self,
        request: WalkRequest<'_>,
        _needs: RelationNeeds<'_>,
        builder: &mut NodeTableBuilder<'_>,
        _scratch: &mut (),
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError> {
        if request.source.0 == FAILING {
            return Err(WalkError::Library);
        }
        if !request.path.0.ends_with(".walk") {
            return Err(WalkError::Unsupported);
        }
        let bytes = request.source.0;
        let offset = |at: usize| Offset(u32::try_from(at).unwrap_or(u32::MAX));
        enter_node(builder, SENTENCE, FieldCode(0), Offset(0));
        // Nodes are numbered as they open: the sentence is node 0, and each node opened after it takes the next id.
        let mut opened: u32 = 1;
        let mut quotes: Vec<Quoted> = Vec::new();
        let mut start = None;
        for (at, byte) in bytes.iter().chain(std::iter::once(&b' ')).enumerate() {
            match (start, byte.is_ascii_whitespace()) {
                (None, false) => start = Some(at),
                (Some(from), true) => {
                    let word = bytes.get(from..at).unwrap_or_default();
                    let quoted = word.len() > 2 && word.starts_with(b"<") && word.ends_with(b">");
                    let field = if word.ends_with(b":") {
                        LABEL
                    } else {
                        FieldCode(0)
                    };
                    if quoted {
                        enter_node(builder, QUOTE, FieldCode(0), offset(from));
                        quotes.push(Quoted {
                            host: Position(opened),
                            from: from.saturating_add(1),
                            to: at.saturating_sub(1),
                        });
                        opened = opened.saturating_add(1);
                    }
                    token(builder, WORD, field, offset(from), offset(at));
                    if quoted {
                        leave_node(builder, offset(at));
                    }
                    start = None;
                }
                _ => {}
            }
            if *byte == b'~' {
                enter_implied_node(builder, SENTENCE, FieldCode(0));
                leave_node(builder, offset(at));
                opened = opened.saturating_add(1);
            }
            if *byte == b'!' {
                diagnose(
                    builder,
                    offset(at),
                    offset(at.saturating_add(1)),
                    NameText("an exclamation"),
                );
            }
        }
        leave_node(builder, offset(bytes.len()));
        // The embedded segments follow the file's own tree: each quote's inner text, read again as a sentence of one word.
        for quote in quotes {
            let inner = bytes.get(quote.from..quote.to).unwrap_or_default();
            let text = EmbeddedText {
                text: NameText(std::str::from_utf8(inner).unwrap_or_default()),
                mapping: EmbeddedMapping::Exact(offset(quote.from)),
            };
            let end = offset(inner.len());
            let _root = embed(builder, quote.host, text);
            enter_node(builder, SENTENCE, FieldCode(0), Offset(0));
            token(builder, WORD, FieldCode(0), Offset(0), end);
            leave_node(builder, end);
        }
        let table = finished(builder);
        let mut file = WalkingFile {
            table,
            source: request.source.0,
            file: request.file,
            fills: Count(0),
            weakness: weakness(request.source.0),
        };
        let mut source = file_source::<Self>(&mut file);
        visit.visit(FileFacts {
            table,
            source: request.source,
            file: request.file,
            path: request.path,
            facts: &mut source,
        });
        Ok(())
    }
}

/// A worker's words over the fixture language's fixed names.
///
/// # Errors
///
/// When the fixture language's fixed names are refused: one is stated twice or is the empty text.
pub fn fixture_words() -> Result<FileWords, FixedNamesError> {
    let names = Walking::vocabulary().fixed;
    Ok(file_words(&base_words(checked(&names.0)?, &[])))
}

/// The lent file's facts of the one relation, asked in one request.
pub fn asked_one(file: &mut FileFacts<'_>, relation: Relation, words: &mut FileWords) -> FactSet {
    let request = request_of(&[relation]);
    let mut set = FactSet::default();
    begun(&mut set, &request);
    file.facts.facts(&request, words, &mut set);
    set
}

/// What a relation's answer was, in words, with the rows read.
#[must_use]
pub fn said(answer: &FactAnswer, rows: usize) -> String {
    match answer {
        FactAnswer::Answered => format!("answered, {rows} rows"),
        FactAnswer::Partial(_) => format!("a part of the rows, {rows} rows"),
        FactAnswer::Refused(cause) => format!("refused: {cause:?}"),
    }
}

/// Asks what `asked` names of the lent file and says what was answered: `<relation> whole`, or `<relation> by <column>` with
/// the key the first row holds, read by the table's index. Nothing when it names no relation or no column of it; why the
/// fixture's fixed names were refused when they were.
pub fn fill_named(file: &mut FileFacts<'_>, asked: &str) -> Option<String> {
    let (relation, by) = asked.split_once(' ')?;
    let relation = relation_named(NameRef(relation))?;
    let mut words = match fixture_words() {
        Ok(words) => words,
        Err(refused) => return Some(refused.to_string()),
    };
    let set = asked_one(file, relation, &mut words);
    let (answer, table) = (answer_of(&set, relation), table_of(&set, relation));
    if by == "whole" {
        return Some(said(
            &answer,
            usize::try_from(every_row(table).end.0).unwrap_or(usize::MAX),
        ));
    }
    let named = by.strip_prefix("by ")?;
    let column = column_named(relation, NameRef(named))?;
    if !indexed_by(table, column) {
        return Some(format!("not indexed by {named}"));
    }
    let key = cells_of(table, column).0.first().copied()?;
    Some(said(&answer, rows_of(table, column, FactKey::Cell(key)).into_iter().count()))
}

/// The word of the kind of the first link of the lent file with its text, when it is the fixed word the fixture writes by handle.
pub fn link_kind(file: &mut FileFacts<'_>, words: &mut FileWords) -> String {
    let set = asked_one(file, Relation::TokenLink, words);
    let kinds = cells_of(
        table_of(&set, Relation::TokenLink),
        Column::TokenLink(TokenLinkColumn::LinkKind),
    );
    let cell: Option<CellWord> = kinds.0.first().copied();
    let handle = fixed(words, NEXT_TOKEN);
    cell.filter(|cell| *cell == handle).map_or_else(
        || "not the fixed word".to_owned(),
        |cell| format!("word {} `{}`", cell.0, text(words, cell).0),
    )
}

/// What a growth run held: the rows of each relation at the two sizes, and its faults.
#[must_use]
pub fn held(report: &GrowthReport) -> String {
    let rows: Vec<String> = report
        .relations
        .iter()
        .map(|each| {
            let relation = name(each.relation).0;
            format!(
                "{relation} {:?} {} to {}",
                each.size, each.small.0, each.large.0
            )
        })
        .collect();
    let faults = if report.faults.is_empty() {
        "no fault".to_owned()
    } else {
        format!("{:?}", report.faults)
    };
    format!(
        "{} and {} elements: {}; {faults}",
        report.small.0,
        report.large.0,
        rows.join(", ")
    )
}
