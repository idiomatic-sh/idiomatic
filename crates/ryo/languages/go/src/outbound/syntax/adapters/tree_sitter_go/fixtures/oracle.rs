//! What `go/ast` says of a file, compared with the rows: the comments, the strings, the imports, the functions, the parameters and the
//! calls the Go program pinned in `tests/fixtures/standard/dump.go` reads from the file with `go/parser`, as its output pinned in
//! `output.txt` states them, against the rows of the relations that read them. A reader that loses, adds or misreads one is a difference.

use ryo::languages::Relation;
use ryo::languages::contract::{Cell, OracleRows, Point};

/// What `go/ast` said of every file pinned: a block for each, led by the line `file <hash> <length>`.
#[expect(
    clippy::large_include_file,
    reason = "the pinned output of the standard's own parser is the oracle's whole input, compiled in so the comparison reads no file and cannot fail to"
)]
const DUMP: &str = include_str!("../../../../../../tests/fixtures/standard/output.txt");

/// What the dump states of a relation: the word its lines start with, and where the cells it states are in a row, in the order it
/// states them.
struct Fact {
    /// The word the dump's lines of the relation start with.
    word: &'static str,
    /// The relation whose rows the lines are compared with.
    relation: Relation,
    /// Where in a row each cell the dump states is, in the order it states them.
    places: &'static [usize],
}

/// The facts the dump states, one for each relation it states.
const FACTS: [Fact; 7] = [
    Fact {
        word: "comment",
        relation: Relation::Comment,
        places: &[3, 1],
    },
    Fact {
        word: "constraint",
        relation: Relation::Directive,
        places: &[1, 2, 3],
    },
    Fact {
        word: "string",
        relation: Relation::StringLiteral,
        places: &[4, 2],
    },
    Fact {
        word: "import",
        relation: Relation::StaticImport,
        places: &[5, 2, 3],
    },
    Fact {
        word: "function",
        relation: Relation::Function,
        places: &[3, 1],
    },
    Fact {
        word: "parameter",
        relation: Relation::Parameter,
        places: &[7, 1, 4, 3],
    },
    Fact {
        word: "call",
        relation: Relation::StaticCall,
        places: &[4, 2],
    },
];

/// The line that leads the block of a file.
const FILE: &str = "file ";

/// What ends the head of a file `go/parser` refuses.
const REFUSED: &str = " refused";

/// The reason the rows give for the name an `import . "p"` binds.
const DOT_IMPORT: &str = "dot-import";

/// Where the kind of a parameter is in its row.
const KIND_PLACE: usize = 1;

/// The word of the kind of the last parameter, in the rows and in the dump.
const VARIADIC_POSITIONAL: &str = "variadic-positional";
/// The same, in the dump.
const VARIADIC: &str = "variadic";

/// What the oracle says of a file the dump holds no block for.
const UNPINNED: &str =
    "go/ast has no pinned reading of this file: add it to tests/fixtures/standard/output.txt";

/// The 64-bit FNV-1a hash of `text`, as the dump program writes it.
fn hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// What leads the hex of a text in the dump, so that the empty text is a field.
const TEXT_MARK: char = 'x';

/// The text as the dump writes it: a mark and the hex of its bytes.
fn hex(text: &str) -> String {
    let mut hexed = String::from(TEXT_MARK);
    hexed.extend(text.bytes().map(|byte| format!("{byte:02x}")));
    hexed
}

/// A point as the dump writes it.
fn point(point: Point) -> String {
    format!("{}:{}", point.line, point.column)
}

/// A cell as the dump writes it: a name as the hex of its text, an unknown value as `?`, but for the name `import . "p"` binds.
fn shown(cell: &Cell) -> String {
    match cell {
        Cell::Unlocated => "<unlocated>".to_owned(),
        Cell::Name(text) => hex(text),
        Cell::Number(number) => number.to_string(),
        Cell::Span(from, to) => format!("{} {}", point(*from), point(*to)),
        Cell::Unknown(reason) if reason == DOT_IMPORT => hex("."),
        Cell::Unknown(_) => "?".to_owned(),
        Cell::Node(_) | Cell::File(_) | Cell::Flag(_) => String::new(),
    }
}

/// The word the dump uses for the kind of a parameter: the rows' word, but for the one the dump spells shorter.
fn kind_word(cell: &Cell) -> String {
    match cell {
        Cell::Name(text) if text == VARIADIC_POSITIONAL => VARIADIC.to_owned(),
        Cell::Name(text) => text.clone(),
        other @ (Cell::Unlocated
        | Cell::Node(_)
        | Cell::Unknown(_)
        | Cell::File(_)
        | Cell::Span(..)
        | Cell::Number(_)
        | Cell::Flag(_)) => shown(other),
    }
}

/// The facts of `relation` the rows state, as the dump writes them: its word and the cells.
fn from_rows(rows: &OracleRows, fact: &Fact) -> Vec<String> {
    let (relation, word) = (fact.relation, fact.word);
    rows.of(relation)
        .iter()
        .map(|row| {
            let cells: Vec<String> = fact
                .places
                .iter()
                .filter_map(|place| row.get(*place).map(|cell| (*place, cell)))
                .map(|(place, cell)| {
                    if relation == Relation::Parameter && place == KIND_PLACE {
                        kind_word(cell)
                    } else {
                        shown(cell)
                    }
                })
                .collect();
            format!("{word} {}", cells.join(" "))
        })
        .collect()
}

/// What the dump holds of a file.
enum Pinned {
    /// The facts `go/ast` read from it, one line each.
    Facts(Vec<&'static str>),
    /// `go/parser` refused it, and states no fact.
    Refused,
    /// The dump holds nothing of it.
    Absent,
}

/// The block of the dump for `text`.
fn from_dump(text: &str) -> Pinned {
    let head = format!("file {:016x} {}", hash(text), text.len());
    let mut lines = DUMP.lines();
    let Some(found) = lines.find(|line| line.starts_with(&head)) else {
        return Pinned::Absent;
    };
    if found.ends_with(REFUSED) {
        return Pinned::Refused;
    }
    Pinned::Facts(lines.take_while(|line| !line.starts_with(FILE)).collect())
}

/// What the dump states of `text` that `rows` do not say, and the reverse: the differences, none when the rows agree.
#[must_use]
pub(crate) fn oracle(text: &str, rows: &OracleRows) -> Vec<String> {
    let block = match from_dump(text) {
        Pinned::Facts(block) => block,
        Pinned::Refused => return Vec::new(),
        Pinned::Absent => return vec![UNPINNED.to_owned()],
    };
    let mut differences = Vec::new();
    for fact in FACTS {
        let word = fact.word;
        let stated: Vec<String> = block
            .iter()
            .filter(|line| line.split(' ').next() == Some(word))
            .map(|line| (*line).to_owned())
            .collect();
        let read = from_rows(rows, &fact);
        let mut missing: Vec<&String> = stated.iter().filter(|line| !read.contains(line)).collect();
        let mut added: Vec<&String> = read.iter().filter(|line| !stated.contains(line)).collect();
        missing.sort();
        added.sort();
        differences.extend(
            missing
                .iter()
                .map(|line| format!("go/ast reads `{line}`, which no row says")),
        );
        differences.extend(
            added
                .iter()
                .map(|line| format!("a row says `{line}`, which go/ast does not read")),
        );
        if stated.len() != read.len() && missing.is_empty() && added.is_empty() {
            differences.push(format!(
                "go/ast reads {} `{word}` facts and the rows say {}",
                stated.len(),
                read.len()
            ));
        }
    }
    differences
}
