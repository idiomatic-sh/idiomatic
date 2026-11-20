//! pass: a `fills!` item with a key clause for each of two columns. The item names the columns it has a body for; run for a key
//! of one of them it writes that key's rows alone and says it wrote what was asked; run whole, or for a key of a column it has
//! no body for, it writes every row, and says so of the key.

use ryo::languages::{
    self, CellWord, Column, Count, FieldCode, FileId, Fill, FillExtent, FixedNames, KeyedFillItem,
    KindBits, Missing, NodeTable, Offset, Position, ReportText, Rows, Span, Symbol,
    TokenLinkColumn, TokenLinkRow, Wanted, facts, fillers, node_table,
};
use ryo_language::domain::fillers::fixtures::{Walking, WalkingFile, Weakness};

/// The link from the token at `source` to the token after it.
fn link(file: &WalkingFile<'_>, source: u32) -> TokenLinkRow {
    TokenLinkRow {
        source: languages::node_id(file.file, Position(source)),
        target: languages::node_id(file.file, Position(source + 1)),
        link_kind: Symbol(0),
        file: file.file,
        span: Span {
            start: Offset(0),
            end: Offset(0),
        },
    }
}

/// The ids of the file's tokens but the last: each has a link.
fn sources(file: &WalkingFile<'_>) -> std::ops::Range<u32> {
    let first = u32::try_from(node_table::node_count(file.table).0).unwrap_or(u32::MAX);
    first..node_table::own_end(file.table).0.saturating_sub(1)
}

struct Links;

ryo::fills! {
    Links => languages::TokenLink, walk(Walking) |file, _words, rows| {
        for source in sources(file) {
            rows.write(link(file, source));
        }
    }
    key(Column::TokenLink(TokenLinkColumn::Source)) |file, _words, key, rows| {
        let at = languages::node_position(languages::NodeId(key.0)).0;
        if sources(file).contains(&at) {
            rows.write(link(file, at));
        }
    }
    key(Column::TokenLink(TokenLinkColumn::Target)) |file, _words, key, rows| {
        let at = languages::node_position(languages::NodeId(key.0)).0;
        if at > 0 && sources(file).contains(&(at - 1)) {
            rows.write(link(file, at - 1));
        }
    }
}

/// The rows a run of the item wrote.
#[derive(Default)]
struct Kept(Vec<TokenLinkRow>);

impl Rows<TokenLinkRow> for Kept {
    fn write(&mut self, row: TokenLinkRow) {
        self.0.push(row);
    }
    fn partial(&mut self, _: ReportText<'_>, _: Missing) {}
    fn partial_at(&mut self, _: FileId, _: ReportText<'_>, _: Missing) {}
}

fn main() {
    let mut table = NodeTable::default();
    let mut writer = node_table::builder(&mut table, Count(4));
    node_table::enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    for at in 0..4 {
        node_table::token(
            &mut writer,
            KindBits(2),
            FieldCode(0),
            Offset(at * 2),
            Offset(at * 2 + 1),
        );
    }
    node_table::leave_node(&mut writer, Offset(7));
    let table = node_table::finished(&mut writer);
    let mut file = WalkingFile {
        table,
        source: b"a b c d",
        file: FileId(1),
        fills: Count(0),
        weakness: Weakness::Sound,
    };
    let base = facts::base_words(&FixedNames::default(), &[]).expect("no fixed name to refuse");
    let mut words = facts::file_words(&base);
    let key = |column, id| {
        Wanted::Key(
            Column::TokenLink(column),
            CellWord(languages::node_id(FileId(1), Position(id)).0),
        )
    };
    let mut run = |wanted| {
        let mut kept = Kept::default();
        let fill = fillers::filled_keyed::<Links>(&mut file, &mut words, wanted, &mut kept);
        (
            fill,
            kept.0.len(),
            kept.0
                .first()
                .map(|row| languages::node_position(row.source).0),
        )
    };
    assert_eq!(
        <Links as KeyedFillItem>::KEYS,
        [
            Column::TokenLink(TokenLinkColumn::Source),
            Column::TokenLink(TokenLinkColumn::Target)
        ]
    );
    assert_eq!(
        run(Wanted::Whole),
        (Fill::Written(FillExtent::Asked), 3, Some(1))
    );
    assert_eq!(
        run(key(TokenLinkColumn::Source, 2)),
        (Fill::Written(FillExtent::Asked), 1, Some(2))
    );
    assert_eq!(
        run(key(TokenLinkColumn::Target, 2)),
        (Fill::Written(FillExtent::Asked), 1, Some(1))
    );
    assert_eq!(
        run(key(TokenLinkColumn::Source, 4)),
        (Fill::Written(FillExtent::Asked), 0, None)
    );
    assert_eq!(
        run(key(TokenLinkColumn::LinkKind, 2)),
        (Fill::Written(FillExtent::Whole), 3, Some(1))
    );
}
