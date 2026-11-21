//! The fill side's test world: a pack written by the contract's own encoding, a language that counts what it is asked, a
//! small file of nodes, and a loop's stand-in that answers the callback. The cases of `made_tables` and the reference
//! bench build their tables with it.

use self::packs::{FILE, FIRST, SECOND, TEXT, TOLD};
use crate::domain::code::service::list;
use crate::domain::code::types::{IndexSite, ProgramCode};
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{MadeTable, PackDefinition};
use crate::domain::made_tables::types::{
    FillFrame, FillStore, PatternAsk, PatternFound, PatternRows, RuleSetView, TableNeed,
};
use crate::domain::steps::types::{MadeTableId, StepRegex};
use ryo_language::domain::facts::service::{base_words, file_words};
use ryo_language::domain::facts::types::{
    CellWord, FactAnswer, FactSource, FileWords, FillExtent, FilledColumn, FilledColumns, Wanted,
};
#[cfg(test)]
use ryo_language::domain::facts::types::{FillerSentence, PartialRows};
use ryo_language::domain::fixed_names::service::checked;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_operators::domain::patterns::types::PatternMatcher;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::Relation;
#[cfg(test)]
use ryo_relations::domain::relations::types::{Count, Missing};
use ryo_relations::domain::terms::types::NameText;

pub mod benches;
pub mod packs;
#[cfg(test)]
pub mod reads;

/// How the world's language answers a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Telling {
    /// A key's rows alone.
    Keyed,
    /// Every row, saying so.
    #[cfg(test)]
    WholeToAKey,
    /// A refusal, to every ask.
    #[cfg(test)]
    Refusing,
    /// Rows in part, to every ask.
    #[cfg(test)]
    Partial,
}

/// A language of one relation that counts what it is asked.
#[derive(Debug)]
pub(crate) struct Told {
    /// The relation's rows: its first two cells.
    pub(crate) rows: Vec<[u64; 2]>,
    /// How it answers.
    pub(crate) telling: Telling,
    /// What it was asked, in order.
    pub(crate) asked: Vec<Wanted>,
}

impl FactSource for Told {
    fn fill(
        &mut self,
        relation: Relation,
        wanted: Wanted,
        _: &mut FileWords,
        out: &mut FilledColumns,
    ) -> FactAnswer {
        self.asked.push(wanted);
        if relation != TOLD {
            return FactAnswer::Refused(AbsenceCause::NotYetRead);
        }
        #[cfg(test)]
        match self.telling {
            Telling::Refusing => return FactAnswer::Refused(AbsenceCause::NotYetRead),
            Telling::Partial => {
                return FactAnswer::Partial(PartialRows {
                    cause: FillerSentence("an unread stretch".into()),
                    missing: Missing::Any,
                });
            }
            Telling::Keyed | Telling::WholeToAKey => {}
        }
        if out.0.len() < 5 {
            out.0.resize_with(5, FilledColumn::default);
        }
        let (wanted, extent) = match (wanted, self.telling) {
            #[cfg(test)]
            (Wanted::Key(..), Telling::WholeToAKey) => (Wanted::Whole, FillExtent::Whole),
            (Wanted::Key(..) | Wanted::Whole, _) => (wanted, FillExtent::Asked),
        };
        for [first, second] in &self.rows {
            let held = match wanted {
                Wanted::Whole => true,
                Wanted::Key(column, cell) if column == FIRST => cell.0 == *first,
                Wanted::Key(column, cell) if column == SECOND => cell.0 == *second,
                Wanted::Key(..) => false,
            };
            if held {
                for (column, cell) in
                    out.0
                        .iter_mut()
                        .zip([*first, *second, 0, u64::from(FILE.0), 0])
                {
                    column.0.push(CellWord(cell));
                }
            }
        }
        FactAnswer::Answered(extent)
    }

    fn render(&mut self, _: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }
}

/// The loop's stand-in: it says which tables have no row, answers a pattern with rows made from the key, and counts.
#[derive(Debug, Default)]
pub(crate) struct Looped {
    /// The tables it says have no row.
    pub(crate) empty: Vec<MadeTableId>,
    /// Each table it was asked the need of, in order.
    pub(crate) needs: Vec<MadeTableId>,
    /// Each key a pattern was asked for, in order.
    pub(crate) keys: Vec<Vec<u64>>,
    /// How many cells a pattern's row has.
    pub(crate) width: usize,
}

impl PatternRows for Looped {
    fn table_need(
        &mut self,
        _: &mut FillFrame<'_>,
        table: MadeTableId,
    ) -> Result<TableNeed, FilledError> {
        self.needs.push(table);
        Ok(if self.empty.contains(&table) {
            TableNeed::Empty
        } else {
            TableNeed::Rows
        })
    }

    /// A key whose first cell is `k` has `k % 3` rows; row `i` holds `k * 100 + i` in every cell.
    fn pattern_rows(
        &mut self,
        _: &mut FillFrame<'_>,
        asked: PatternAsk<'_>,
        rows: &mut Vec<CellWord>,
    ) -> Result<PatternFound, FilledError> {
        self.keys
            .push(asked.key.iter().map(|cell| cell.0).collect());
        let first = asked.key.first().map_or(0, |cell| cell.0);
        for row in 0..first % 3 {
            rows.extend(std::iter::repeat_n(
                CellWord(first.saturating_mul(100).saturating_add(row)),
                self.width,
            ));
        }
        Ok(if first % 3 == 0 {
            PatternFound::Nothing
        } else {
            PatternFound::Rows
        })
    }
}

/// A rule set's parts as the fill side reads them, owned by a world.
#[derive(Debug)]
pub(crate) struct RuleSet {
    /// The code.
    pub(crate) code: ProgramCode,
    /// The word of each kind's name, by the kind's code.
    pub(crate) kinds: Vec<CellWord>,
    /// The word of each field's name, by the field's code.
    pub(crate) fields: Vec<CellWord>,
    /// The sources of the regular expressions, by number.
    pub(crate) regexes: Vec<StepRegex>,
}

/// Everything a seam call is made with, owned: a case builds one, then borrows a frame from it.
#[derive(Debug)]
pub(crate) struct World {
    /// What the fill side reads of the rule set: the code the contract's encoding wrote for the case's pack, and the
    /// vocabulary's words.
    pub(crate) programs: RuleSet,
    /// The file's nodes.
    pub(crate) nodes: NodeTable,
    /// The file's words.
    pub(crate) words: FileWords,
    /// The fill side's state.
    pub(crate) store: FillStore,
    /// The language.
    pub(crate) told: Told,
    /// The loop's stand-in.
    pub(crate) looped: Looped,
}

#[cfg(test)]
/// The world of a pack of the tables, whose language holds `rows` and answers a key as `telling` says.
///
/// # Errors
///
/// The encoding refuses the pack.
pub(crate) fn world_with(
    tables: Vec<MadeTable>,
    definitions: Vec<PackDefinition>,
    rows: &[[u64; 2]],
    telling: Telling,
) -> Result<World, FilledError> {
    world_of(tables, definitions, rows.to_vec(), packs::nodes(), telling)
}

/// The world of a pack of the tables over a file of the nodes given, whose language holds `rows`.
///
/// # Errors
///
/// The encoding refuses the pack.
pub(crate) fn world_of(
    tables: Vec<MadeTable>,
    definitions: Vec<PackDefinition>,
    rows: Vec<[u64; 2]>,
    nodes: NodeTable,
    telling: Telling,
) -> Result<World, FilledError> {
    let code = packs::coded(tables, definitions)?;
    let mut store = FillStore::default();
    crate::domain::made_tables::sources::service::next_file(&mut store);
    Ok(World {
        programs: RuleSet {
            code,
            kinds: [0_u64, 11, 12, 13].map(CellWord).to_vec(),
            fields: [0_u64, 0, 0, 0, 24].map(CellWord).to_vec(),
            regexes: packs::REGEXES
                .iter()
                .map(|source| StepRegex((*source).into()))
                .collect(),
        },
        nodes,
        words: file_words(&base_words(
            checked(&[]).map_err(|_| FilledError::Code {
                named: crate::domain::filled::types::MissingCode::Word,
            })?,
            &packs::TEXTS.map(NameText),
        )),
        store,
        told: Told {
            rows,
            telling,
            asked: Vec::new(),
        },
        looped: Looped {
            width: 1,
            ..Looped::default()
        },
    })
}

/// The frame of a world and its loop's stand-in.
pub(crate) fn framed(world: &mut World) -> (FillFrame<'_>, &mut Looped) {
    let World {
        programs,
        nodes: file,
        words,
        store,
        told,
        looped,
    } = world;
    (
        FillFrame {
            programs: RuleSetView {
                code: &programs.code,
                kinds: &programs.kinds,
                fields: &programs.fields,
                regexes: &programs.regexes,
            },
            nodes: file,
            file: FILE,
            text: ByteSlice(TEXT),
            facts: told,
            words,
            store,
            matcher: PatternMatcher(&MatchingUseCaseImpl),
        },
        looped,
    )
}

#[cfg(test)]
/// A table's rows as text: each row's cells, rows separated by `;`.
pub(crate) fn shown(store: &FillStore, table: MadeTableId) -> String {
    let Some(slot) = crate::domain::made_tables::sources::service::table_of(store, table) else {
        return "unmade".to_owned();
    };
    let rows: Vec<String> = (0..slot.rows.count.0)
        .map(|row| {
            crate::domain::made_tables::service::row(&slot.rows, Count(row))
                .iter()
                .map(|cell| cell.0.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    rows.join("; ")
}

#[cfg(test)]
/// How a column of a table is shown by `shown_as`.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Shown {
    /// The cell's number.
    Number,
    /// The text of the word the cell is, in quotes.
    Text,
    /// A span, as `start..end`.
    Span,
    /// A node, as `n` and its place in the file's table.
    Node,
}

#[cfg(test)]
/// A table's rows as text, each column shown as `columns` says: rows separated by `;`.
pub(crate) fn shown_as(world: &World, table: MadeTableId, columns: &[Shown]) -> String {
    let Some(slot) = crate::domain::made_tables::sources::service::table_of(&world.store, table)
    else {
        return "unmade".to_owned();
    };
    let rows: Vec<String> = (0..slot.rows.count.0)
        .map(|row| {
            crate::domain::made_tables::service::row(&slot.rows, Count(row))
                .iter()
                .zip(columns)
                .map(|(cell, shown)| match shown {
                    Shown::Number => cell.0.to_string(),
                    Shown::Text => format!(
                        "{:?}",
                        ryo_language::domain::facts::service::text(&world.words, *cell).0
                    ),
                    Shown::Span => format!("{}..{}", cell.0 >> 32, cell.0 & 0xffff_ffff),
                    Shown::Node => format!("n{}", cell.0 & 0xffff_ffff),
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    rows.join("; ")
}

/// The site of the index of a source (0 the node table, 1 a relation, 2 a made table) by the columns, as the code
/// numbers it; a site past every site when the code holds none.
pub(crate) fn site_of(code: &ProgramCode, source: u16, columns: &[u32]) -> IndexSite {
    let found = code.indexes.iter().position(|index| {
        index.source.0 == source
            && list(&code.pool, index.columns)
                .iter()
                .map(|slot| slot.0)
                .eq(columns.iter().copied())
    });
    IndexSite(
        found
            .and_then(|at| u32::try_from(at).ok())
            .unwrap_or(u32::MAX),
    )
}
