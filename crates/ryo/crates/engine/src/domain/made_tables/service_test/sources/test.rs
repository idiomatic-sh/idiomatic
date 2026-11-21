//! The cases of the seam's reads of tables and sites, each on a pack the contract's encoding wrote: a table made on its
//! first read and once, a table the loop says has no row, the rows a key holds over a made table and over the node table, a
//! row's cells, a stretch that stays good, the next file, and a site the code does not hold.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::small_codes::service::node_column_code;
use crate::domain::code::types::{IndexSite, RelationNumber, Short};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::made_tables::fixtures::packs::{FIRST, TOLD};
use crate::domain::made_tables::fixtures::packs::{constants, of, pipeline, rows, scan_made};
use crate::domain::made_tables::fixtures::reads::{asked, held, prober, told_of};
use crate::domain::made_tables::fixtures::{Telling, World, framed, shown, site_of, world_with};
use crate::domain::made_tables::reading::service::{index_at, rows_at};
use crate::domain::made_tables::sources::service::{
    cell_of, handle_of, made, matched, next_file, relation_rows, table_of,
};
use crate::domain::made_tables::types::{KeyFrom, ReadSource};
use crate::domain::steps::types::{MadeColumn, MadeTableId, ReadColumn, StepRows};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{NodeColumn, NodeExtent, Relation};
use std::borrow::Cow;

/// What a case reads through the seam.
#[derive(Clone, Copy, Debug)]
enum Read {
    /// A table made twice.
    MadeTwice,
    /// Two tables that scan a third, which the loop says has no row.
    NeedEmpty,
    /// The rows a key holds in a made table.
    OverMade,
    /// The rows a kind holds in the node table, and a node row's cells.
    OverNodes,
    /// A stretch read again after an index was built and a key asked.
    StretchStays,
    /// The next file.
    NextFile,
    /// A site the code does not hold.
    NoSite,
}

type Input = Read;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The relation's rows.
const ROWS: [[u64; 2]; 5] = [[1, 10], [2, 20], [1, 30], [3, 10], [1, 40]];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_table_is_made_on_its_first_read_and_once"),
        word: Word::Succeeds,
        input: || Read::MadeTwice,
        expect: Expect::Value(Cow::Borrowed("Made, Held: 1; 3 | asked: whole")),
    },
    Case {
        name: CaseName("a_table_the_loop_says_has_no_row_is_made_empty_and_asked_once"),
        word: Word::Succeeds,
        input: || Read::NeedEmpty,
        expect: Expect::Value(Cow::Borrowed(
            "[\"\", \"\", \"\"] | the loop was asked the need of [MadeTableId(0)]",
        )),
    },
    Case {
        name: CaseName("a_key_over_a_made_table_makes_it_and_reads_its_index"),
        word: Word::Succeeds,
        input: || Read::OverMade,
        expect: Expect::Value(Cow::Borrowed(
            "[1, 3] filled 0 indexed 4 | [1, 3] filled 0 indexed 0 | row 2 holds 3",
        )),
    },
    Case {
        name: CaseName(
            "a_key_over_the_node_table_gives_id_cells_and_a_node_rows_cells_are_read_by_them",
        ),
        word: Word::Succeeds,
        input: || Read::OverNodes,
        expect: Expect::Value(Cow::Borrowed(
            "[30064771073, 30064771074, 30064771075] filled 0 indexed 7 | kind, field, start: 12 0 0; 12 24 3; 12 0 6",
        )),
    },
    Case {
        name: CaseName("a_stretch_stays_good_whatever_is_read_after_it"),
        word: Word::Succeeds,
        input: || Read::StretchStays,
        expect: Expect::Value(Cow::Borrowed(
            "[0, 1, 2] filled 3 indexed 0 | after three more reads: [0, 1, 2] filled 3 indexed 0",
        )),
    },
    Case {
        name: CaseName("the_next_file_holds_no_table_no_index_and_no_row_of_the_last"),
        word: Word::Succeeds,
        input: || Read::NextFile,
        expect: Expect::Value(Cow::Borrowed(
            "table Some(Made), index of 5 rows, relation of 5 rows | next file: table None, index of 0 rows, relation of 0 rows",
        )),
    },
    Case {
        name: CaseName("a_site_the_code_does_not_hold_is_a_fault_of_the_code"),
        word: Word::Refuses,
        input: || Read::NoSite,
        expect: Expect::Error(FilledError::Code {
            named: MissingCode::Site,
        }),
    },
];

/// The world of a case: table 0 holds constants, and the table after it names the site the case reads.
fn world_of(read: Read) -> Result<World, FilledError> {
    let told = StepRows::Filled(TOLD);
    let tables = match read {
        Read::NeedEmpty => vec![
            constants(&[1, 4, 3]),
            rows(1, pipeline(Vec::from(scan_made(0, 0, 1)), [of(1)])),
            rows(1, pipeline(Vec::from(scan_made(0, 0, 1)), [of(1)])),
        ],
        Read::MadeTwice | Read::NextFile | Read::NoSite | Read::StretchStays => vec![
            constants(&[1, 4, 3]),
            prober(told, vec![ReadColumn::Filled(FIRST)]),
            prober(
                StepRows::Made(MadeTableId(0)),
                vec![ReadColumn::Made(MadeColumn(0))],
            ),
        ],
        Read::OverMade => vec![
            constants(&[1, 4, 3, 4]),
            prober(
                StepRows::Made(MadeTableId(0)),
                vec![ReadColumn::Made(MadeColumn(0))],
            ),
        ],
        Read::OverNodes => vec![
            constants(&[12]),
            prober(
                StepRows::Node(Relation::SyntaxTree),
                vec![ReadColumn::Node(NodeColumn::Kind)],
            ),
        ],
    };
    world_with(tables, Vec::new(), &ROWS, Telling::Keyed)
}

fn subject(read: Input) -> Result<Value, Failure> {
    let mut world = world_of(read)?;
    let relation_site = site_of(&world.programs.code, 1, &[0]);
    let said = match read {
        Read::MadeTwice => {
            let (mut frame, looped) = framed(&mut world);
            let first = made(&mut frame, looped, MadeTableId(1))?;
            let second = made(&mut frame, looped, MadeTableId(1))?;
            let rows = shown(frame.store, MadeTableId(1));
            drop(frame);
            format!("{first:?}, {second:?}: {rows} | asked: {}", told_of(&world))
        }
        Read::NeedEmpty => {
            world.looped.empty.push(MadeTableId(0));
            let (mut frame, looped) = framed(&mut world);
            let _one = made(&mut frame, looped, MadeTableId(1))?;
            let _two = made(&mut frame, looped, MadeTableId(2))?;
            let rows = [0, 1, 2].map(|each| shown(frame.store, MadeTableId(each)));
            drop(frame);
            format!(
                "{rows:?} | the loop was asked the need of {:?}",
                world.looped.needs
            )
        }
        Read::OverMade => {
            let site = site_of(&world.programs.code, 2, &[0]);
            let one = asked(&mut world, site, &[4], KeyFrom::Row)?;
            let again = asked(&mut world, site, &[4], KeyFrom::Constants)?;
            let (mut frame, _) = framed(&mut world);
            let cell = cell_of(
                &mut frame,
                ReadSource::Made(MadeTableId(0)),
                Short(0),
                CellWord(2),
            );
            format!("{one} | {again} | row 2 holds {}", cell.0)
        }
        Read::OverNodes => {
            let site = site_of(
                &world.programs.code,
                0,
                &[u32::from(node_column_code(NodeColumn::Kind).0)],
            );
            let key = [CellWord(12)];
            let rows = {
                let (mut frame, looped) = framed(&mut world);
                matched(&mut frame, looped, site, &key, KeyFrom::Row)?
            };
            let found = held(&world, rows);
            let handles: Vec<CellWord> = rows_at(&world.store, rows.rows)
                .iter()
                .map(|row| handle_of(*row, rows.tag))
                .collect();
            let (mut frame, _) = framed(&mut world);
            let own = ReadSource::Nodes(NodeExtent::Own);
            let cells: Vec<String> = handles
                .iter()
                .map(|handle| {
                    [NodeColumn::Kind, NodeColumn::Field, NodeColumn::Start]
                        .map(|column| {
                            cell_of(&mut frame, own, node_column_code(column), *handle)
                                .0
                                .to_string()
                        })
                        .join(" ")
                })
                .collect();
            format!("{found} | kind, field, start: {}", cells.join("; "))
        }
        Read::StretchStays => {
            let key = [CellWord(1)];
            let first = {
                let (mut frame, looped) = framed(&mut world);
                matched(&mut frame, looped, relation_site, &key, KeyFrom::Row)?
            };
            let before = held(&world, first);
            let _other = asked(&mut world, relation_site, &[3], KeyFrom::Row)?;
            let _index = asked(&mut world, relation_site, &[2], KeyFrom::Constants)?;
            let site = site_of(&world.programs.code, 2, &[0]);
            let _made = asked(&mut world, site, &[4], KeyFrom::Row);
            let after = held(&world, first);
            format!("{before} | after three more reads: {after}")
        }
        Read::NextFile => {
            let _one = asked(&mut world, relation_site, &[1], KeyFrom::Constants)?;
            {
                let (mut frame, looped) = framed(&mut world);
                let _now = made(&mut frame, looped, MadeTableId(1))?;
            }
            let before = format!(
                "table {:?}, index of {} rows, relation of {} rows",
                table_of(&world.store, MadeTableId(1)).map(|slot| slot.state),
                index_at(&world.store, relation_site).rows.len(),
                relation_rows(&world.store, RelationNumber(0)).0
            );
            next_file(&mut world.store);
            format!(
                "{before} | next file: table {:?}, index of {} rows, relation of {} rows",
                table_of(&world.store, MadeTableId(1)).map(|slot| slot.state),
                index_at(&world.store, relation_site).rows.len(),
                relation_rows(&world.store, RelationNumber(0)).0
            )
        }
        Read::NoSite => asked(&mut world, IndexSite(999), &[1], KeyFrom::Row)?,
    };
    Ok(Cow::Owned(said))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
