// the typed cells, the rows and the text they are written as are one vocabulary a fixture is stated in
//! What a language's conformance expects, as typed rows: a relation's rows are cells, a name is its text, a span is a line and a
//! column on each end, and nothing is a line of a table file.

use std::fmt;

use crate::domain::semantics::types::{RenderContext, RenderOutcome};
use crate::inbound::use_cases::parse::types::SourceFile;
use crate::inbound::use_cases::parse::types::{KindSample, Why};
use ryo_relations::domain::relations::service::{column_name, columns, folder_of};
use ryo_relations::domain::relations::types::{Integer, KindBits, Offset, Position, Relation};

/// Whether a piece of a tree is a node or a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leafness {
    /// It holds other pieces.
    Node,
    /// It spells text.
    Token,
}

/// A piece of the library's own tree, in the language's kinds: what the language's walk of its library gives for a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Walked {
    /// The kind the language maps the library's kind to.
    pub kind: KindBits,
    /// Whether it is a node or a token.
    pub leaf: Leafness,
    /// Where it starts.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The pieces under it, in order.
    pub children: Vec<Walked>,
}

/// How a language walks its library's own tree of a file: the root, with every node and token under it, the file's own tree and not the
/// trees added for reading.
pub type LibraryWalk = fn(&str) -> Walked;

/// An independent reference rejected the source without returning a recovery tree.
/// This does not represent missing or malformed reference evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceRefusal;

/// What the tree of a file is compared with to hold its shape: an independent walk of the parser library's own tree, or a projection of
/// the language's standard (Python's `ast.dump`) mapped into the same vocabulary. Either is independent of the code that built the table;
/// no language compares its tree with itself.
#[derive(Clone, Copy, Debug)]
pub enum Shape {
    /// An independent walk of the library's tree.
    Walk(LibraryWalk),
    /// A projection of the standard's tree.
    Standard(LibraryWalk),
    /// An independently executed reference either supplies its tree or explicitly rejects the source.
    Reference(fn(&str) -> Result<Walked, ReferenceRefusal>),
}

/// The rows of one relation a language wrote for a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationRows {
    /// The relation.
    pub relation: Relation,
    /// Its rows, sorted.
    pub rows: Vec<Vec<Cell>>,
}

/// The rows a language wrote for a file, relation by relation, for each relation it reads from trees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleRows(pub Vec<RelationRows>);

impl OracleRows {
    /// The rows of `relation`; none when the language does not read it.
    #[must_use]
    pub fn of(&self, relation: Relation) -> &[Vec<Cell>] {
        self.0
            .iter()
            .find(|each| each.relation == relation)
            .map_or(&[], |each| each.rows.as_slice())
    }
}

/// How a language rebuilds facts from its rows and compares them with the library's own semantic model: it is given the file's text and
/// the rows, and answers what differs, one sentence for each; nothing when they agree.
pub type Oracle = fn(&str, &OracleRows) -> Vec<String>;

/// A place in a file: a line and a column, both counted from one, the column in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    /// The line.
    pub line: u32,
    /// The column.
    pub column: u32,
}

/// One cell of a row as a test writes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cell {
    /// The parser supplied no source location.
    Unlocated,
    /// A node, by its place in the file's element table.
    Node(u32),
    /// A name, by its text.
    Name(String),
    /// A value the language cannot resolve statically, by the name of the reason it declares for that.
    Unknown(String),
    /// A file, by its path.
    File(String),
    /// A stretch of a file, from one place to another.
    Span(Point, Point),
    /// A whole number.
    Number(Integer),
    /// A yes or no.
    Flag(bool),
}

impl fmt::Display for Cell {
    /// The cell as the Rust an `expected.rs` writes it in: its constructor's call.
    #[expect(
        clippy::use_debug,
        reason = "a text is written as a Rust string literal, and a `str`'s `Debug` is exactly that literal, escapes included; an expected file is Rust source, not a message"
    )]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unlocated => write!(f, "unlocated()"),
            Self::Node(place) => {
                let digits = place.to_string();
                let mut literal = String::new();
                for (at, digit) in digits.chars().enumerate() {
                    if at > 0 && (digits.len() - at).is_multiple_of(3) {
                        literal.push('_');
                    }
                    literal.push(digit);
                }
                write!(f, "node({literal})")
            }
            Self::Name(text) => write!(f, "name({text:?})"),
            Self::Unknown(reason) => write!(f, "unknown({reason:?})"),
            Self::File(path) => write!(f, "file({path:?})"),
            Self::Span(from, to) => write!(
                f,
                "span(({}, {}), ({}, {}))",
                from.line, from.column, to.line, to.column
            ),
            Self::Number(number) => write!(f, "number({number})"),
            Self::Flag(flag) => write!(f, "{flag}"),
        }
    }
}

/// A name, by its text, or a value the language cannot resolve and the reason it declares for that, as a fixture states it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text {
    /// A name, by its text.
    Written(&'static str),
    /// An unknown value, by the name of its reason.
    Unknown(&'static str),
}

impl Text {
    /// The cell the text is.
    #[must_use]
    pub fn cell(&self) -> Cell {
        match self {
            Self::Written(text) => Cell::Name((*text).to_owned()),
            Self::Unknown(reason) => Cell::Unknown((*reason).to_owned()),
        }
    }
}

/// A file, by its path, as a fixture states it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileName(pub &'static str);

/// A stretch of a file, from one place to another, as a fixture states it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stretch {
    /// Whether this expectation has a source location.
    pub located: bool,
    /// Where it starts.
    pub from: Point,
    /// Where it ends.
    pub to: Point,
}

/// A whole number, as a fixture states it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count(pub i64);

/// A node, by its place in the file's element table.
#[must_use]
pub const fn node(place: u32) -> Position {
    Position(place)
}

/// A name, by its text.
#[must_use]
pub const fn name(text: &'static str) -> Text {
    Text::Written(text)
}

/// A value the language cannot resolve statically, by the name of the reason it declares for that.
#[must_use]
pub const fn unknown(reason: &'static str) -> Text {
    Text::Unknown(reason)
}

/// A file, by its path.
#[must_use]
pub const fn file(path: &'static str) -> FileName {
    FileName(path)
}

/// A stretch of a file, from `(line, column)` to `(line, column)`.
#[must_use]
pub const fn span(from: (u32, u32), to: (u32, u32)) -> Stretch {
    Stretch {
        located: true,
        from: Point {
            line: from.0,
            column: from.1,
        },
        to: Point {
            line: to.0,
            column: to.1,
        },
    }
}

/// A whole number.
#[must_use]
pub const fn number(number: i64) -> Count {
    Count(number)
}

impl From<Position> for Cell {
    fn from(place: Position) -> Self {
        Self::Node(place.0)
    }
}

impl From<Text> for Cell {
    fn from(text: Text) -> Self {
        text.cell()
    }
}

impl From<FileName> for Cell {
    fn from(file: FileName) -> Self {
        Self::File(file.0.to_owned())
    }
}

impl From<Stretch> for Cell {
    fn from(stretch: Stretch) -> Self {
        if stretch.located {
            Self::Span(stretch.from, stretch.to)
        } else {
            Self::Unlocated
        }
    }
}

impl From<Count> for Cell {
    fn from(count: Count) -> Self {
        Self::Number(Integer(count.0))
    }
}

/// A row of a relation as a fixture states it, with a typed value for each column: `expected_rows::<relation>` has one for each relation
/// of the catalog.
pub trait ExpectedRow {
    /// The relation the row is of.
    const RELATION: Relation;

    /// The cells of the row, in the order of the relation's columns.
    fn cells(&self) -> Vec<Cell>;
}

/// The rows a relation read from trees must give for its fixture.
#[must_use]
pub fn expected_rows<Row>(rows: &[Row]) -> Expected
where
    Row: ExpectedRow,
{
    Expected {
        relation: folder_of(Row::RELATION).0,
        rows: rows.iter().map(ExpectedRow::cells).collect(),
    }
}

impl ManifestExpected {
    /// The expected rows of the project in `tests/fixtures/manifests/<case>/`, none yet.
    #[must_use]
    pub const fn of(case: &'static str) -> Self {
        Self {
            case,
            rows: Vec::new(),
        }
    }

    /// Adds the rows of one relation.
    #[must_use]
    pub fn rows<Row>(mut self, rows: &[Row]) -> Self
    where
        Row: ExpectedRow,
    {
        let folder = folder_of(Row::RELATION).0;
        self.rows
            .extend(rows.iter().map(|row| (folder, row.cells())));
        self
    }
}

/// One cell of a row as a case states it in a constant: the cells of `Cell`, with the text borrowed so that a constant can hold it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    /// No source location.
    Unlocated,
    /// A node, by its place in the file's element table.
    Node(u32),
    /// A name, by its text.
    Name(&'static str),
    /// A value the language cannot resolve statically, by the name of its reason.
    Unknown(&'static str),
    /// A file, by its path.
    File(&'static str),
    /// A stretch of a file, from one place to another.
    Span(Point, Point),
    /// A whole number.
    Number(i64),
    /// A yes or no.
    Flag(bool),
}

/// The rows a case wants, each in full: the cells of every column.
pub type Wanted = &'static [&'static [Want]];

/// A node, by its place in the file's element table.
#[must_use]
pub const fn want_node(place: u32) -> Want {
    Want::Node(place)
}

/// A value the language cannot resolve statically, by the name of its reason.
#[must_use]
pub const fn want_unknown(reason: &'static str) -> Want {
    Want::Unknown(reason)
}

/// A name, by its text.
#[must_use]
pub const fn want_name(text: &'static str) -> Want {
    Want::Name(text)
}

/// A file, by its path.
#[must_use]
pub const fn want_file(path: &'static str) -> Want {
    Want::File(path)
}

/// A stretch of a file, from `(line, column)` to `(line, column)`.
#[must_use]
pub const fn want_span(from: (u32, u32), to: (u32, u32)) -> Want {
    Want::Span(
        Point {
            line: from.0,
            column: from.1,
        },
        Point {
            line: to.0,
            column: to.1,
        },
    )
}

/// A whole number.
#[must_use]
pub const fn want_number(number: i64) -> Want {
    Want::Number(number)
}

/// A yes or no.
#[must_use]
pub const fn want_flag(flag: bool) -> Want {
    Want::Flag(flag)
}

/// The rows a read gave, as a case holds them: kept for the rest of the run, which is a test's.
#[must_use]
pub fn kept(rows: Vec<Vec<Cell>>) -> Wanted {
    let rows: Vec<&'static [Want]> = rows
        .into_iter()
        .map(|row| {
            let cells: Vec<Want> = row
                .into_iter()
                .map(|cell| match cell {
                    Cell::Unlocated => Want::Unlocated,
                    Cell::Node(place) => Want::Node(place),
                    Cell::Name(text) => Want::Name(Box::leak(text.into_boxed_str())),
                    Cell::Unknown(reason) => Want::Unknown(Box::leak(reason.into_boxed_str())),
                    Cell::File(path) => Want::File(Box::leak(path.into_boxed_str())),
                    Cell::Span(from, to) => Want::Span(from, to),
                    Cell::Number(number) => Want::Number(number.0),
                    Cell::Flag(flag) => Want::Flag(flag),
                })
                .collect();
            &*Box::leak(cells.into_boxed_slice())
        })
        .collect();
    Box::leak(rows.into_boxed_slice())
}

/// The rows a relation read from `tests/fixtures/relations/<relation>/input.<ext>` must write, each in full.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expected {
    /// The relation's folder, which is its name in code.
    pub relation: &'static str,
    /// The rows, in any order.
    pub rows: Vec<Vec<Cell>>,
}

/// The rows the manifest relations must write for the project in `tests/fixtures/manifests/<case>/`, each row with the folder of its
/// relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestExpected {
    /// The project's folder.
    pub case: &'static str,
    /// The rows, each with its relation, in any order.
    pub rows: Vec<(&'static str, Vec<Cell>)>,
}

/// One actual semantic-session request against a source fixture, addressed by a unique byte span.
#[derive(Clone, Debug)]
pub struct SemanticExpected {
    /// The source the provider parses and renders.
    pub source: SourceFile,
    /// The requested expression's start byte.
    pub start: Offset,
    /// The requested expression's end byte.
    pub end: Offset,
    /// The role in which the expression is rendered.
    pub context: RenderContext,
    /// The complete expected answer, including an explicit unsupported reason.
    pub outcome: RenderOutcome,
}

/// Everything a language's conformance is held to besides the files of its fixtures: how each kind of its trees is shown, and the
/// rows its fixtures must give.
#[derive(Clone, Debug)]
pub struct Fixtures {
    /// How each kind is shown, in the order of their numbers.
    pub kinds: Vec<KindSample>,
    /// The rows of each relation read from trees.
    pub relations: Vec<Expected>,
    /// The rows of the manifest relations, for each project.
    pub manifests: Vec<ManifestExpected>,
    /// Actual semantic-session requests and complete expected answers.
    pub semantic: Vec<SemanticExpected>,
    /// What the language holds the shape of its tree to: a walk of its library's own tree, or a projection of its standard.
    pub shape: Shape,
    /// How the language rebuilds facts from its rows and compares them with its library's semantic model.
    pub oracle: Oracle,
    /// The columns of the relations it reads that its fixtures cannot show filled, each with why.
    pub unfilled: Vec<Unfilled>,
}

/// A column of a relation the language reads that its fixtures cannot show filled, and why: the reader declares it does not apply,
/// the way a relation it does not read cites the standard. A column every fixture shows filled is never declared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unfilled {
    /// The relation, by its folder.
    pub relation: &'static str,
    /// The column, by its name.
    pub column: &'static str,
    /// What the fixtures cannot show.
    pub unshown: Unshown,
    /// Why: the standard, or the construct of the language, that never fills it.
    pub reason: Why,
}

/// What a fixture cannot show of a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unshown {
    /// Any value but an empty one: a name or a file that is not empty, a number that is not zero, a raised flag, a located span.
    Filled,
    /// One word of the column's closed set, or `true` or `false` of a flag.
    Word(&'static str),
}

/// The name of the function that writes a cell in the fixture's own syntax (`node(1)`, `span((1, 1), (1, 8))`); none for a flag.
fn constructor(cell: &Cell) -> Option<&'static str> {
    match cell {
        Cell::Unlocated => Some("unlocated"),
        Cell::Node(_) => Some("node"),
        Cell::Name(_) => Some("name"),
        Cell::File(_) => Some("file"),
        Cell::Span(..) => Some("span"),
        Cell::Number(_) => Some("number"),
        Cell::Unknown(_) => Some("unknown"),
        Cell::Flag(_) => None,
    }
}

/// The name of the typed row of `relation`.
fn row_type(relation: Relation) -> String {
    format!("{relation:?}ExpectedRow")
}

/// The fields of a row of `relation` written as the fixture writes them, one for each column.
fn fields(relation: Relation, row: &[Cell]) -> Vec<String> {
    columns(relation)
        .into_iter()
        .zip(row)
        .map(|(column, cell)| format!("{}: {cell}", column_name(column)))
        .collect()
}

/// The text of an `expected.rs` for `relation`, whose fixture input is `input`, holding `rows`: what a language that wants the rows it
/// writes to be the rows it expects keeps (`--bless`).
#[must_use]
pub fn expected_source(relation: Relation, input: &str, rows: &[Vec<Cell>]) -> String {
    let folder = folder_of(relation);
    let mut used: Vec<&str> = rows.iter().flatten().filter_map(constructor).collect();
    let row = row_type(relation);
    used.push(&row);
    used.push("expected_rows");
    used.sort_unstable();
    used.dedup();
    let mut text = format!(
        "// The rows `relations/{folder}/{input}` must give.\n\nuse ryo::languages::contract::{{Expected, {}}};\n\n/// The rows, in the order the language gives them.\nconst ROWS: &[{row}] = &[\n",
        used.join(", ")
    );
    for each in rows {
        text.push_str("    ");
        text.push_str(&row);
        text.push_str(" {\n");
        for field in fields(relation, each) {
            text.push_str("        ");
            text.push_str(&field);
            text.push_str(",\n");
        }
        text.push_str("    },\n");
    }
    text.push_str("];\n\n/// The rows the fixture's input must give, as the relations contract compares them.\npub(super) fn expected() -> Expected {\n    expected_rows(ROWS)\n}\n");
    text
}

/// A row of `relation` as a line of text, written as the fixture writes it, for the findings.
#[must_use]
pub fn shown(relation: Relation, row: &[Cell]) -> String {
    format!(
        "{} {{ {} }}",
        row_type(relation),
        fields(relation, row).join(", ")
    )
}

#[cfg(test)]
#[path = "expected_test.rs"]
mod tests;

/// An explicitly absent source location.
#[must_use]
pub const fn unlocated() -> Stretch {
    Stretch {
        located: false,
        from: Point { line: 0, column: 0 },
        to: Point { line: 0, column: 0 },
    }
}
