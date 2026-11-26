//! The `walk` contract's cases: over every Python source of the language's fixtures (514 files, the parser's suite among them), the walk writes the tree today's reading writes
//! (every element, under the same parent, at the same place; in the same field wherever the parser accepts the file) and its table keeps the laws of the engine's node
//! table; the lent file says the number the run gave it; the linked language carries this walk for the engine, with the names the
//! walk states for the run (each kind and each field at the code the table holds for it); a file that is not Python is not walked.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::labels::service::{field_bits, field_name, fields};

use crate::domain::syntax::service::{kind_name, kind_of, kinds};
use crate::fixtures::table_laws::broken_laws;
use crate::fixtures::walking::walked;
use crate::outbound::syntax::fixtures::library::fixture_sources;
use ryo::languages::{self, ByteSlice, FileFacts, FileId, FileVisit, KindBits, NodeTable, Offset, RelationNeeds, Walk, WalkError, WalkRequest, node_table};

/// What a case asks of the walk.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The laws of the node table the walked tables break, over every fixture source.
    Laws,
    /// Whether the walk's kind count is the count of the language's kinds.
    Kinds,
    /// What differs between the names the walk states for the run and the names of the codes it writes, and between them and
    /// the names the linked language's walk states.
    Names,
    /// What the walk answers for a file at this path.
    Path(&'static str),
    /// What differs between the number the run gave a file and the number the lent file says.
    Numbered(FileId),
    /// What differs between this walk and the walk the linked language carries for the engine.
    Linked,
}

type Input = Asked;
/// What was found, one line for each finding; empty when nothing differs and no law is broken.
type Value = String;
type Failure = WalkError;

/// The rows of the contract's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_walked_table_keeps_the_laws_of_the_node_table"),
        word: Word::Succeeds,
        input: || Asked::Laws,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_kind_count_is_the_count_of_the_language_s_kinds"),
        word: Word::Succeeds,
        input: || Asked::Kinds,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_walk_names_each_kind_and_each_field_at_its_code"),
        word: Word::Succeeds,
        input: || Asked::Names,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_python_file_is_walked"),
        word: Word::Succeeds,
        input: || Asked::Path("package/module.py"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_lent_file_says_the_number_the_run_gave_it"),
        word: Word::Succeeds,
        input: || Asked::Numbered(FileId(7)),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_linked_language_carries_this_walk_for_the_engine"),
        word: Word::Succeeds,
        input: || Asked::Linked,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_that_is_not_python_is_not_walked"),
        word: Word::Refuses,
        input: || Asked::Path("notes.txt"),
        expect: Expect::Error(WalkError::Unsupported),
    },
];

/// The laws the walked table of one source breaks.
fn law_differences<W: Walk>(walk: &W, folder: &str, bytes: &[u8]) -> Result<String, WalkError> {
    let (mut table, mut scratch) = (NodeTable::default(), Default::default());
    let mut found = String::new();
    let length = Offset(u32::try_from(bytes.len()).unwrap_or(u32::MAX));
    walked(
        walk,
        "input.py",
        bytes,
        &[],
        &mut table,
        &mut scratch,
        &mut |file| {
            found = broken_laws(file.table, W::KIND_COUNT, length)
                .iter()
                .take(3)
                .map(|law| format!("{folder}: {law}\n"))
                .collect();
        },
    )?;
    Ok(found)
}

/// The walk's kind count against the language's kinds.
fn kind_difference<W: Walk>(_walk: &W) -> String {
    if W::KIND_COUNT == languages::Count(kinds().count()) {
        String::new()
    } else {
        format!(
            "the walk counts {} kinds, the language has {}",
            W::KIND_COUNT.0,
            kinds().count()
        )
    }
}

/// What differs between the names the walk states for the run and the names of the codes it writes into the node table: a kind
/// is named at its code, a field at its code, and the linked language's walk states the same names. A walk that names no kind
/// or no field is a difference too.
fn name_differences<W: Walk>(_walk: &W) -> String {
    let names = W::vocabulary();
    let mut found = String::new();
    if names.kinds.0.len() != W::KIND_COUNT.0 || names.kinds.0.is_empty() {
        found.push_str(&format!(
            "{} kinds named, {} counted\n",
            names.kinds.0.len(),
            W::KIND_COUNT.0
        ));
    }
    for (code, stated) in names.kinds.0.iter().enumerate() {
        let written = kind_name(kind_of(KindBits(u16::try_from(code).unwrap_or(u16::MAX))));
        if *stated != written {
            found.push_str(&format!(
                "kind {code} is named {}, the table's is {}\n",
                stated.0, written.0
            ));
        }
    }
    if names.fields.0.len() != fields().count() || names.fields.0.is_empty() {
        found.push_str(&format!(
            "{} fields named, {} in the language\n",
            names.fields.0.len(),
            fields().count()
        ));
    }
    for field in fields() {
        let code = usize::from(field_bits(field).0);
        if names.fields.0.get(code) != Some(&field_name(field)) {
            found.push_str(&format!(
                "field {code} is not named {}\n",
                field_name(field).0
            ));
        }
    }
    if crate::linked().walk_engine().vocabulary() != names {
        found.push_str("the linked walk states other names\n");
    }
    found
}

/// The visit that keeps which file of the run the lent file says it is.
struct Numbering(Option<FileId>);

impl FileVisit for Numbering {
    fn visit(&mut self, file: FileFacts<'_>) {
        self.0 = Some(file.file);
    }
}

/// What differs between `number`, the number the run gave a file, and the number the lent file says.
fn numbered<W: Walk>(walk: &W, number: FileId) -> Result<String, WalkError> {
    let (mut table, mut numbering) = (NodeTable::default(), Numbering(None));
    walk.walk(
        WalkRequest {
            path: languages::SourcePath("module.py"),
            file: number,
            source: ByteSlice(b"x = 1\n"),
        },
        RelationNeeds(&[]),
        &mut node_table::builder(&mut table, W::KIND_COUNT),
        &mut Default::default(),
        &mut numbering,
    )?;
    Ok(match numbering.0 {
        Some(lent) if lent == number => String::new(),
        lent => format!("the run gave the file {number:?}, the lent file says {lent:?}"),
    })
}

/// What differs between `walk` and the walk the linked language carries for the engine: its kinds, and the table and the number
/// of one file walked through the engine's interface. A language linked without its walk has no kind and walks no file.
fn linked<W: Walk>(walk: &W) -> Result<String, WalkError> {
    const SOURCE: &[u8] = b"x = 1\n";
    let request = WalkRequest {
        path: languages::SourcePath("module.py"),
        file: FileId(3),
        source: ByteSlice(SOURCE),
    };
    let (mut own, mut numbering) = (NodeTable::default(), Numbering(None));
    walk.walk(
        request,
        RelationNeeds(&[]),
        &mut node_table::builder(&mut own, W::KIND_COUNT),
        &mut Default::default(),
        &mut numbering,
    )?;
    let engine = crate::linked().walk_engine();
    let (mut table, mut buffers, mut lent) =
        (NodeTable::default(), engine.buffers(), Numbering(None));
    engine.walk_file(
        request,
        RelationNeeds(&[]),
        &mut table,
        &mut *buffers,
        &mut lent,
    )?;
    let mut differences = String::new();
    if engine.kind_count() != W::KIND_COUNT {
        differences.push_str(&format!(
            "the linked walk has {:?} kinds, this walk {:?}\n",
            engine.kind_count(),
            W::KIND_COUNT
        ));
    }
    if lent.0 != Some(request.file) {
        differences.push_str(&format!("the linked walk lent the file as {:?}\n", lent.0));
    }
    let elements = (
        node_table::element_count(&table),
        node_table::element_count(&own),
    );
    if elements.0 != elements.1 || elements.0.0 == 0 {
        differences.push_str(&format!(
            "the linked walk wrote {:?} elements, this walk {:?}\n",
            elements.0, elements.1
        ));
    }
    Ok(differences)
}

fn subject(asked: Input) -> Result<Value, Failure> {
    match asked {
        Asked::Linked => linked(&crate::walk()),
        Asked::Numbered(number) => numbered(&crate::walk(), number),
        Asked::Laws => fixture_sources()
            .map_err(|_unread| WalkError::Library)?
            .iter()
            .map(|(path, bytes)| law_differences(&crate::walk(), path, bytes))
            .collect(),
        Asked::Kinds => Ok(kind_difference(&crate::walk())),
        Asked::Names => Ok(name_differences(&crate::walk())),
        Asked::Path(path) => {
            let (mut table, mut scratch) = (NodeTable::default(), Default::default());
            let mut visits = 0_u32;
            walked(
                &crate::walk(),
                path,
                b"x = 1\n",
                &[],
                &mut table,
                &mut scratch,
                &mut |_file| {
                    visits += 1;
                },
            )?;
            Ok(if visits == 1 {
                String::new()
            } else {
                format!("the file was lent {visits} times")
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(test)]
mod absent_test;
#[cfg(test)]
mod fixed_test;
#[cfg(test)]
mod growth_test;
#[cfg(test)]
mod keys_test;
