//! The `walk` contract's cases of the fills by key, over every Python source of the language's fixtures: for each relation the
//! catalog declares a key of, the rows of every key of a source, taken together, are the rows of its whole fill; each fill of a
//! key writes rows that hold the key, and says it wrote what was asked wherever the whole fill answers in full. Every element
//! of a file is asked as a node key; every word the whole fill wrote and a word no row holds as a word key. A key of a column
//! with no body writes every row and says so.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::walking::{declared_keys, walk_words, walked};
use crate::outbound::syntax::fixtures::library::fixture_sources;
use ryo::languages::{
    self, CellKind, CellWord, Column, FactAnswer, FileFacts, FileWords, FillExtent, FilledColumns,
    NodeTable, Position, Relation, WalkError, Wanted, facts, node_id, node_table,
};

/// What a case asks of the fills by key.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// What differs between the fills of every key of each declared key column of the relation and its whole fill.
    Keys(Relation),
    /// What a key of a column the relation declares no key for is answered with, against the whole fill.
    Undeclared,
    /// The relations the catalog declares a key of, each with its key columns.
    Declared,
}

type Input = Asked;
/// What was asked and written over the fixture sources (the files, the rows of the whole fills, and for each key column the
/// keys that wrote a row and the rows they wrote), then what was found, one line for each finding; `no difference` for none.
type Value = &'static str;
type Failure = WalkError;

/// The rows of the contract's cases of the fills by key.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_reads_of_every_name_are_the_reads_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::BindingRead),
        expect: Expect::Value(
            "592 files, 20878 rows whole, 4229 keys of BindingRead(Name) wrote 20878: no difference",
        ),
    },
    Case {
        name: CaseName("the_shadows_of_every_binding_are_the_shadows_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::BindingShadow),
        expect: Expect::Value(
            "592 files, 2728 rows whole, 1371 keys of BindingShadow(Binding) wrote 2728: no difference",
        ),
    },
    Case {
        name: CaseName("the_keys_of_every_constant_are_the_constant_keys_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::ConstantKey),
        expect: Expect::Value(
            "592 files, 7897 rows whole, 7897 keys of ConstantKey(Id) wrote 7897: no difference",
        ),
    },
    Case {
        name: CaseName("the_shapes_of_every_expression_are_the_shapes_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::ExpressionShape),
        expect: Expect::Value(
            "592 files, 47448 rows whole, 47448 keys of ExpressionShape(Id) wrote 47448: no difference",
        ),
    },
    Case {
        name: CaseName("the_types_of_every_expression_are_the_types_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::ExpressionType),
        expect: Expect::Value(
            "592 files, 50834 rows whole, 50832 keys of ExpressionType(Id) wrote 50834: no difference",
        ),
    },
    Case {
        name: CaseName("the_rows_of_every_definition_are_the_functions_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::Function),
        expect: Expect::Value(
            "592 files, 1120 rows whole, 1120 keys of Function(Id) wrote 1120: no difference",
        ),
    },
    Case {
        name: CaseName("the_rows_of_every_base_are_the_bases_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::Inherits),
        expect: Expect::Value(
            "592 files, 119 rows whole, 119 keys of Inherits(Occurrence) wrote 119: no difference",
        ),
    },
    Case {
        name: CaseName("the_paths_of_every_read_are_the_lookup_paths_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::LookupScope),
        expect: Expect::Value(
            "592 files, 39800 rows whole, 20834 keys of LookupScope(Reference) wrote 39800: no difference",
        ),
    },
    Case {
        name: CaseName("the_segments_of_every_string_are_the_raw_segments_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::RawFormatSegment),
        expect: Expect::Value(
            "592 files, 3070 rows whole, 2977 keys of RawFormatSegment(Id) wrote 3070: no difference",
        ),
    },
    Case {
        name: CaseName("the_rows_of_every_expression_are_the_references_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::Reference),
        expect: Expect::Value(
            "592 files, 20744 rows whole, 20744 keys of Reference(Occurrence) wrote 20744: no difference",
        ),
    },
    Case {
        name: CaseName("the_rows_of_every_name_expression_are_the_resolutions_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::Resolves),
        expect: Expect::Value(
            "592 files, 15509 rows whole, 15509 keys of Resolves(Occurrence) wrote 15509: no difference",
        ),
    },
    Case {
        name: CaseName("the_rows_of_every_scope_are_the_scopes_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::Scope),
        expect: Expect::Value(
            "592 files, 2349 rows whole, 2349 keys of Scope(Child) wrote 2349: no difference",
        ),
    },
    Case {
        name: CaseName("the_links_from_and_to_every_token_are_the_links_of_the_file"),
        word: Word::Succeeds,
        input: || Asked::Keys(Relation::TokenLink),
        expect: Expect::Value(
            "592 files, 207744 rows whole, 139998 keys of TokenLink(Source) wrote 207744, 139969 keys of TokenLink(Target) wrote 207744: no difference",
        ),
    },
    Case {
        name: CaseName("every_relation_the_catalog_declares_a_key_of_has_its_case_here"),
        word: Word::Succeeds,
        input: || Asked::Declared,
        expect: Expect::Value(
            "[Function(Id)] [Inherits(Occurrence)] [Reference(Occurrence)] [Scope(Child)] [Resolves(Occurrence)] [BindingRead(Name)] [LookupScope(Reference)] [BindingShadow(Binding)] [ExpressionShape(Id)] [ExpressionType(Id)] [ConstantKey(Id)] [RawFormatSegment(Id)] [TokenLink(Source), TokenLink(Target)]",
        ),
    },
    Case {
        name: CaseName("a_key_of_a_column_with_no_body_writes_every_row_and_says_so"),
        word: Word::Succeeds,
        input: || Asked::Undeclared,
        expect: Expect::Value("592 files, 207744 rows whole: no difference"),
    },
];

/// The rows of filled columns from the row `from` on, each as its cells.
fn rows_from(columns: &FilledColumns, from: usize) -> Vec<Vec<CellWord>> {
    let count = columns.0.first().map_or(0, |column| column.0.len());
    (from..count)
        .map(|row| {
            columns
                .0
                .iter()
                .filter_map(|column| column.0.get(row).copied())
                .collect()
        })
        .collect()
}

/// What the fills of a relation wrote over the files: the rows of the whole fills, and for each declared key column, in the
/// catalog's order, the keys that wrote a row and the rows written by key.
#[derive(Clone, Debug, Default)]
struct Written {
    /// The files lent.
    files: usize,
    /// The rows the whole fills wrote.
    rows: usize,
    /// For each key column: the keys that wrote a row, and the rows the keys wrote.
    by_key: Vec<(Column, usize, usize)>,
    /// What differs, one line for each finding.
    found: String,
}

/// Adds to `written` the whole fill of `relation` for a lent file against its fills by every key of each declared key column.
fn key_differences(
    shown: &str,
    file: &mut FileFacts<'_>,
    words: &mut FileWords,
    relation: Relation,
    written: &mut Written,
) {
    let found = &mut written.found;
    let mut whole = FilledColumns::default();
    let answer = file.facts.fill(relation, Wanted::Whole, words, &mut whole);
    let mut all = rows_from(&whole, 0);
    all.sort_unstable();
    written.files += 1;
    written.rows += all.len();
    let elements = u32::try_from(node_table::element_count(file.table).0).unwrap_or(u32::MAX);
    let keys = declared_keys(relation);
    if keys.is_empty() {
        found.push_str(&format!("{relation:?}: the catalog declares no key\n"));
    }
    for (number, (column, place, kind)) in keys.into_iter().enumerate() {
        if written.by_key.len() <= number {
            written.by_key.push((column, 0, 0));
        }
        let mut asked: Vec<CellWord> = if kind == CellKind::Node {
            (0..elements)
                .map(|id| CellWord(node_id(file.file, Position(id)).0))
                .collect()
        } else {
            let absent = facts::intern(words, languages::NameText("a word no row holds"));
            all.iter()
                .filter_map(|row| row.get(place).copied())
                .chain([absent])
                .collect()
        };
        asked.sort_unstable();
        asked.dedup();
        let mut by_key = FilledColumns::default();
        for key in asked {
            let from = by_key.0.first().map_or(0, |cells| cells.0.len());
            let told = file
                .facts
                .fill(relation, Wanted::Key(column, key), words, &mut by_key);
            let asked_for = FactAnswer::Answered(FillExtent::Asked);
            let in_part = matches!(
                (&told, &answer),
                (FactAnswer::Partial(_), FactAnswer::Partial(_))
            );
            if told != asked_for && !in_part {
                found.push_str(&format!(
                    "{shown}: {column:?} {key:?} answered {told:?}, whole {answer:?}\n"
                ));
            }
            let of_key = rows_from(&by_key, from);
            if let Some(total) = written.by_key.get_mut(number) {
                total.1 += usize::from(!of_key.is_empty());
                total.2 += of_key.len();
            }
            let strays = of_key
                .iter()
                .filter(|row| row.get(place) != Some(&key))
                .count();
            if strays != 0 {
                found.push_str(&format!(
                    "{shown}: {column:?} {key:?} wrote {strays} rows of another key\n"
                ));
            }
        }
        let mut together = rows_from(&by_key, 0);
        together.sort_unstable();
        if together != all {
            let lost = all.iter().find(|row| together.binary_search(row).is_err());
            let extra = together.iter().find(|row| all.binary_search(row).is_err());
            found.push_str(&format!(
                "{shown}: {column:?}: {} rows by key, {} whole; only whole {lost:?}; only by key {extra:?}\n",
                together.len(),
                all.len()
            ));
        }
    }
}

/// What a key of `token-link`'s kind of link, a column with no body, is answered with against the whole fill: the rows the
/// whole fill wrote when the key's fill wrote every one of them and said so; a finding otherwise.
fn undeclared(
    shown: &str,
    file: &mut FileFacts<'_>,
    words: &mut FileWords,
) -> Result<usize, String> {
    let (mut whole, mut by_key) = (FilledColumns::default(), FilledColumns::default());
    let _answer = file
        .facts
        .fill(Relation::TokenLink, Wanted::Whole, words, &mut whole);
    let column = Column::TokenLink(languages::TokenLinkColumn::LinkKind);
    let key = whole
        .0
        .get(2)
        .and_then(|cells| cells.0.first().copied())
        .unwrap_or_default();
    let told = file.facts.fill(
        Relation::TokenLink,
        Wanted::Key(column, key),
        words,
        &mut by_key,
    );
    let (every, rows) = (
        rows_from(&by_key, 0) == rows_from(&whole, 0),
        rows_from(&whole, 0).len(),
    );
    let said = matches!(
        told,
        FactAnswer::Answered(FillExtent::Whole) | FactAnswer::Partial(_)
    );
    if every && said {
        Ok(rows)
    } else {
        Err(format!(
            "{shown}: answered {told:?}; every row written: {every}\n"
        ))
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    if matches!(asked, Asked::Declared) {
        let keyed: Vec<String> = languages::all()
            .iter()
            .filter_map(|relation| {
                let keys: Vec<Column> = declared_keys(*relation).iter().map(|key| key.0).collect();
                (!keys.is_empty()).then(|| format!("{keys:?}"))
            })
            .collect();
        return Ok(Box::leak(keyed.join(" ").into_boxed_str()));
    }
    let walk = crate::walk();
    let (mut table, mut scratch) = (NodeTable::default(), Default::default());
    let mut written = Written::default();
    for (path, bytes) in fixture_sources().map_err(|_unread| WalkError::Library)? {
        let mut words = walk_words(&walk).expect("each fixed name of the walk is stated once");
        walked(
            &walk,
            "input.py",
            &bytes,
            languages::all(),
            &mut table,
            &mut scratch,
            &mut |mut file| match asked {
                Asked::Keys(relation) => {
                    key_differences(&path, &mut file, &mut words, relation, &mut written)
                }
                Asked::Undeclared | Asked::Declared => {
                    written.files += 1;
                    match undeclared(&path, &mut file, &mut words) {
                        Ok(rows) => written.rows += rows,
                        Err(finding) => written.found.push_str(&finding),
                    }
                }
            },
        )?;
    }
    let by_key: String = written
        .by_key
        .iter()
        .map(|(column, keys, rows)| format!(", {keys} keys of {column:?} wrote {rows}"))
        .collect();
    // The first findings say what is wrong: every later file would repeat them.
    let found: String = written
        .found
        .lines()
        .take(12)
        .map(|line| format!("\n{line}"))
        .collect();
    let found = if found.is_empty() {
        "no difference".to_owned()
    } else {
        found
    };
    let told = format!(
        "{} files, {} rows whole{by_key}: {found}",
        written.files, written.rows
    );
    Ok(Box::leak(told.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
