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
use crate::domain::facts::types::{
    CellWord, FactAnswer, FileFacts, FileWords, FillExtent, FilledColumns, Wanted,
};
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
use ryo_relations::domain::relations::service::{
    column_name, column_named, columns, name, relation_named, relation_of,
};
use ryo_relations::domain::relations::types::Relation;
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

/// What a fill answered, in words, with the rows the columns hold.
#[must_use]
pub fn said(answer: &FactAnswer, columns: &FilledColumns) -> String {
    let rows = columns.0.first().map_or(0, |column| column.0.len());
    match answer {
        FactAnswer::Answered(FillExtent::Asked) => format!("the rows asked, {rows} rows"),
        FactAnswer::Answered(FillExtent::Whole) => {
            format!("every row though a key was asked, {rows} rows")
        }
        FactAnswer::Partial(_) => format!("a part of the rows, {rows} rows"),
        FactAnswer::Refused(cause) => format!("refused: {cause:?}"),
    }
}

/// Fills what `asked` names of the lent file and says what the fill answered: `<relation> whole`, or `<relation> by <column>`
/// with the key the whole fill's first row holds. Nothing when it names no relation or no column of it; why the fixture's fixed
/// names were refused when they were.
pub fn fill_named(file: &mut FileFacts<'_>, asked: &str) -> Option<String> {
    let (relation, by) = asked.split_once(' ')?;
    let relation = relation_named(NameRef(relation))?;
    let mut words = match fixture_words() {
        Ok(words) => words,
        Err(refused) => return Some(refused.to_string()),
    };
    let mut whole = FilledColumns::default();
    let answer = file
        .facts
        .fill(relation, Wanted::Whole, &mut words, &mut whole);
    if by == "whole" {
        return Some(said(&answer, &whole));
    }
    let column = column_named(relation, NameRef(by.strip_prefix("by ")?))?;
    let place = columns(relation).iter().position(|each| *each == column)?;
    let key = whole.0.get(place)?.0.first().copied()?;
    let mut by_key = FilledColumns::default();
    let answer = file
        .facts
        .fill(relation, Wanted::Key(column, key), &mut words, &mut by_key);
    Some(said(&answer, &by_key))
}

/// The word of the kind of the first link of the lent file with its text, when it is the fixed word the fixture writes by handle.
pub fn link_kind(file: &mut FileFacts<'_>, words: &mut FileWords) -> String {
    let mut links = FilledColumns::default();
    let _answer = file
        .facts
        .fill(Relation::TokenLink, Wanted::Whole, words, &mut links);
    let cell: Option<CellWord> = links.0.get(2).and_then(|kinds| kinds.0.first().copied());
    let handle = fixed(words, NEXT_TOKEN);
    cell.filter(|cell| *cell == handle).map_or_else(
        || "not the fixed word".to_owned(),
        |cell| format!("word {} `{}`", cell.0, text(words, cell).0),
    )
}

/// What a growth run held of the part asked for: the rows of each relation (`relations`), or of one key of each declared key
/// column (`keys`), at the two sizes, and its faults.
#[must_use]
pub fn held(report: &GrowthReport, part: &str) -> String {
    let rows: Vec<String> = if part == "keys" {
        report
            .keys
            .iter()
            .map(|each| {
                let relation = name(relation_of(each.column)).0;
                let column = column_name(each.column).0;
                format!(
                    "{relation} by {column} {} to {}",
                    each.small.0, each.large.0
                )
            })
            .collect()
    } else {
        report
            .relations
            .iter()
            .map(|each| {
                let relation = name(each.relation).0;
                format!(
                    "{relation} {:?} {} to {}",
                    each.size, each.small.0, each.large.0
                )
            })
            .collect()
    };
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
