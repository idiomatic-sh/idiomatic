//! The standard's own tree of a file, in the language's kinds: what the shape rail of the conformance holds our tree to. The Go program
//! `tests/fixtures/standard/dump.go` writes the tree `go/parser`, `go/ast` and `go/scanner` make of each file (the nodes of `go/ast`,
//! the tokens of `go/scanner`, and the white space between the tokens, nested by the bytes they span), `output.txt` pins what it wrote,
//! and a file is found in it by its length and its hash. This reads those lines and nothing of the parser library or of our own tree.

use ryo::languages::contract::{Leafness, Walked};
use ryo::languages::{KindBits, Offset};

use crate::domain::syntax::service::{kind_bits, kind_name, kind_of, kinds};

/// What the Go program wrote, for every file of the fixtures.
#[expect(
    clippy::large_include_file,
    reason = "the pinned output of the standard's own parser is the walk's whole input, compiled in so the comparison reads no file and cannot fail to"
)]
const OUTPUT: &str = include_str!("../../../../../../tests/fixtures/standard/output.txt");

/// The start of the line that heads the block of a file.
const FILE: &str = "file ";

/// The folder of the suite of files `go/parser` refuses, held to a syntax error alone.
const REFUSED: &str = "refused";

/// The word a line of the tree starts with.
const TREE: &str = "tree ";

/// The word a line of a role starts with.
const ROLE: &str = "role ";

/// The 64-bit FNV-1a hash of `text`, as the Go program writes it.
fn hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The lines the Go program wrote for the file `text`: its block, after its head; none when the fixtures hold no such file.
fn block(text: &str) -> Vec<&'static str> {
    let head = format!("{FILE}{:016x} {}", hash(text), text.len());
    let refused = format!("{head} refused");
    OUTPUT
        .lines()
        .skip_while(|line| *line != head && *line != refused)
        .skip(1)
        .take_while(|line| !line.starts_with(FILE))
        .collect()
}

/// The number of the kind the standard calls `name`; none for a name the language has no kind for.
fn number(name: &str) -> Option<KindBits> {
    kinds()
        .find(|kind| kind_name(*kind).0 == name)
        .map(kind_bits)
}

/// A number of a line of the Go program's output.
fn field(fields: &mut std::str::Split<'_, char>) -> u32 {
    fields
        .next()
        .and_then(|each| each.parse().ok())
        .unwrap_or_default()
}

/// Whether `go/parser` accepts a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Acceptance {
    /// It reads the file without an error.
    Accepted,
    /// It reports an error in the file.
    Refused,
    /// The fixtures hold no such file.
    Unknown,
}

/// Whether `go/parser` accepted the file `text`, as the Go program recorded it.
#[must_use]
pub(crate) fn acceptance(text: &str) -> Acceptance {
    let head = format!("{FILE}{:016x} {}", hash(text), text.len());
    let refused = format!("{head} refused");
    OUTPUT
        .lines()
        .find(|line| *line == head || *line == refused)
        .map_or(Acceptance::Unknown, |line| {
            if line == refused {
                Acceptance::Refused
            } else {
                Acceptance::Accepted
            }
        })
}

/// The roles `go/ast` gives the elements of the file `text`, each `start end N|T kind field`, the field of `go/ast` that holds it;
/// none when `go/parser` refuses the file or the fixtures hold no such file.
#[must_use]
pub(crate) fn roles(text: &str) -> Option<Vec<&'static str>> {
    (acceptance(text) == Acceptance::Accepted).then(|| {
        block(text)
            .into_iter()
            .filter_map(|line| line.strip_prefix(ROLE))
            .collect()
    })
}

/// The tree the standard makes of the file `text`.
///
/// # Panics
///
/// When the fixtures hold no such file, or the Go program wrote a kind the language does not have.
#[must_use]
pub(crate) fn walked(text: &str) -> Walked {
    let lines = block(text);
    assert!(
        lines.iter().any(|line| line.starts_with(TREE)),
        "tests/fixtures/standard/output.txt has no tree for this file: run `go run standard/dump.go <files> > standard/output.txt` from tests/fixtures"
    );
    tree(&lines)
}

/// The tree the lines of a block of the Go program's output make.
///
/// # Panics
///
/// When the Go program wrote a kind the language does not have.
#[must_use]
pub(crate) fn parsed(block: &str) -> Walked {
    tree(&block.lines().collect::<Vec<_>>())
}

/// The tree the `tree` lines among `lines` make.
#[expect(
    clippy::panic,
    reason = "a kind the standard names that the language lacks is a failure of the fixtures: the conformance cannot judge the file, and says which"
)]
fn tree(lines: &[&str]) -> Walked {
    let mut open: Vec<Walked> = Vec::new();
    for line in lines.iter().filter_map(|each| each.strip_prefix(TREE)) {
        let mut fields = line.split(' ');
        let depth = usize::try_from(field(&mut fields)).unwrap_or_default();
        let leaf = if fields.next() == Some("T") {
            Leafness::Token
        } else {
            Leafness::Node
        };
        let name = fields.next().unwrap_or_default();
        let Some(kind) = number(name) else {
            panic!("the standard names a kind the language does not have: {name}")
        };
        let (start, end) = (field(&mut fields), field(&mut fields));
        while open.len() > depth {
            if let Some(done) = open.pop()
                && let Some(parent) = open.last_mut()
            {
                parent.children.push(done);
            }
        }
        open.push(Walked {
            kind,
            leaf,
            start: Offset(start),
            end: Offset(end),
            children: Vec::new(),
        });
    }
    while open.len() > 1 {
        if let Some(done) = open.pop()
            && let Some(parent) = open.last_mut()
        {
            parent.children.push(done);
        }
    }
    open.pop().unwrap_or(Walked {
        kind: KindBits(0),
        leaf: Leafness::Node,
        start: Offset(0),
        end: Offset(0),
        children: Vec::new(),
    })
}

/// The name of the kind a number stands for.
fn named(bits: KindBits) -> &'static str {
    kind_name(kind_of(bits)).0
}

/// What differs between our tree and the standard's, the first place found; nothing when they agree.
fn difference(ours: &Walked, theirs: &Walked, path: &str) -> Option<String> {
    if (ours.kind, ours.leaf, ours.start, ours.end)
        != (theirs.kind, theirs.leaf, theirs.start, theirs.end)
    {
        return Some(format!(
            "{path}: we have {} at {}..{}, the standard has {} at {}..{}",
            named(ours.kind),
            ours.start.0,
            ours.end.0,
            named(theirs.kind),
            theirs.start.0,
            theirs.end.0
        ));
    }
    if ours.children.len() != theirs.children.len() {
        return Some(format!(
            "{path}: we nest {} elements under {}, the standard nests {}",
            ours.children.len(),
            named(ours.kind),
            theirs.children.len()
        ));
    }
    ours.children
        .iter()
        .zip(&theirs.children)
        .enumerate()
        .find_map(|(place, (left, right))| difference(left, right, &format!("{path}/{place}")))
}

/// What differs between our tree and the standard's, the first place found; nothing when they agree.
#[must_use]
pub(crate) fn differs(ours: &Walked, theirs: &Walked) -> Option<String> {
    difference(ours, theirs, "root")
}

/// The paths of the files below `fixtures` that `go/parser` refuses and whose partial tree the language does not reproduce.
#[expect(
    clippy::expect_used,
    reason = "a folder of the fixtures that cannot be read is a broken checkout: the test stops, never runs no cases"
)]
#[must_use]
pub(crate) fn refused_files(fixtures: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found: Vec<std::path::PathBuf> = std::fs::read_dir(fixtures.join("suite/refused"))
        .expect("a folder of the fixtures is read")
        .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|each| each == "go"))
        .collect();
    found.sort();
    found
}

/// The paths of the files below `fixtures` whose tree is pinned: every relation's input, every file of the corpus and of the suite.
#[expect(
    clippy::expect_used,
    reason = "a folder of the fixtures that cannot be read is a broken checkout: the test stops, never runs no cases"
)]
pub(crate) fn pinned(fixtures: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut folders = vec![
        fixtures.join("relations"),
        fixtures.join("corpus"),
        fixtures.join("suite"),
    ];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder)
            .expect("a folder of the fixtures is read")
            .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        {
            let path = entry.path();
            if path.is_dir() {
                if !path.ends_with(REFUSED) {
                    folders.push(path);
                }
                continue;
            }
            if path.extension().is_some_and(|each| each == "go") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}
