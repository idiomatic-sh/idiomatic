//! The `fact_tables` contract's cases, at its seam (one request of a file, its answer written through the writer, the one
//! lookup out): a keyed list reads back as the written rows filtered by the key, in written order; a per-node column (one
//! row at most a key) through the same lookup; a second key column of a relation finds its rows through the same call; a
//! word key and a run-wide entity key are found as a node key is; presence comes with the answer and agrees with the rows; a
//! relation filled whole answers every row in written order; a partial answer says so and keeps its rows; a refused or
//! unasked relation says so and has no row; the next file's request forgets the last file's answer; a key of another space,
//! or one no row holds, has no row; and every relation of the catalog is written and read by the same calls, so a new
//! family of facts needs no code here.

use super::fixtures::{EXPRESSION, expression_types, length, node, one, read, row_of, with_cells};
use super::reading::service::{every_row, presence, rows_of};
use super::service::table_of;
use super::types::{FactKey, KeyPresence};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::facts::service::{base_words, clear_words, entity, file_words, intern};
use crate::domain::facts::types::CellWord;
use crate::domain::fixed_names::types::CheckedNames;
use ryo_relations::domain::catalog::service::{fact_form, key_columns};
use ryo_relations::domain::relations::collections::relations::binding_read::item::BindingReadColumn;
use ryo_relations::domain::relations::collections::relations::expression_type::item::ExpressionTypeColumn;
use ryo_relations::domain::relations::collections::relations::module_identity::item::ModuleIdentityColumn;
use ryo_relations::domain::relations::collections::relations::string_literal::item::StringLiteralColumn;
use ryo_relations::domain::relations::collections::relations::token_link::item::TokenLinkColumn;
use ryo_relations::domain::relations::service::{ALL, kind};
use ryo_relations::domain::relations::types::{
    CellKind, Column, FactForm, FilledKey, Position, Relation,
};
use ryo_relations::domain::terms::types::NameText;

/// What a row asks of the tables.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// `expression-type`, filled by its node, several rows a node: read key by key against a filter of the written rows.
    ListedEqualsFilter,
    /// `string-literal`, one row a node.
    PerNodeColumn,
    /// `token-link` read by its target, a key the rows are not in the order of.
    SecondKey,
    /// `binding-read` read by its name, a word of the file.
    WordKey,
    /// `module-identity` read by its path, an entity of the run interned by two workers.
    EntityKey,
    /// Presence against the rows, key by key.
    PresenceAgrees,
    /// A node-keyed table read by a word key and by a node no row holds.
    KeyNoRowHolds,
    /// Every relation of the catalog, written and read by each of its node keys.
    EveryRelation,
}

type Input = Scenario;
/// Up to four numbers a row reads, the rest zero.
type Value = [u64; 4];
/// No row of this part refuses.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_keyed_list_reads_back_as_the_written_rows_filtered_by_the_key_in_written_order",
        ),
        word: Word::Succeeds,
        input: || Scenario::ListedEqualsFilter,
        expect: Expect::Value([12, 6, 0, 6]),
    },
    Case {
        name: CaseName("a_per_node_column_reads_back_through_the_same_lookup"),
        word: Word::Succeeds,
        input: || Scenario::PerNodeColumn,
        expect: Expect::Value([1, 0, 2, 3]),
    },
    Case {
        name: CaseName("a_second_key_column_finds_its_rows_in_written_order_through_the_same_call"),
        word: Word::Succeeds,
        input: || Scenario::SecondKey,
        expect: Expect::Value([2, 1, 1, 0]),
    },
    Case {
        name: CaseName("a_word_key_finds_its_rows_as_a_node_key_does"),
        word: Word::Succeeds,
        input: || Scenario::WordKey,
        expect: Expect::Value([2, 1, 0, 1]),
    },
    Case {
        name: CaseName("an_entity_key_interned_by_another_worker_finds_its_rows"),
        word: Word::Succeeds,
        input: || Scenario::EntityKey,
        expect: Expect::Value([1, 2, 1, 1]),
    },
    Case {
        name: CaseName("presence_comes_with_the_answer_and_agrees_with_the_rows"),
        word: Word::Succeeds,
        input: || Scenario::PresenceAgrees,
        expect: Expect::Value([12, 3, 0, 0]),
    },
    Case {
        name: CaseName("a_key_of_another_space_or_one_no_row_holds_has_no_row"),
        word: Word::Succeeds,
        input: || Scenario::KeyNoRowHolds,
        expect: Expect::Value([0, 1, 0, 1]),
    },
    Case {
        name: CaseName("every_relation_of_the_catalog_is_written_and_read_by_the_same_calls"),
        word: Word::Succeeds,
        input: || Scenario::EveryRelation,
        expect: Expect::Value([1, 0, 1, 1]),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::ListedEqualsFilter => {
            let rows = expression_types();
            let set = one(Relation::ExpressionType, rows.clone());
            let table = table_of(&set, Relation::ExpressionType);
            let type_ = Column::ExpressionType(ExpressionTypeColumn::Type);
            let (mut seen, mut differing) = (0, 0);
            for place in 0..12 {
                let found = read(
                    table,
                    type_,
                    rows_of(table, EXPRESSION, FactKey::Node(Position(place))),
                );
                let filtered: Vec<CellWord> = rows
                    .iter()
                    .filter(|cells| cells.first() == Some(&node(place)))
                    .filter_map(|cells| cells.get(2).copied())
                    .collect();
                seen += found.len();
                differing += u64::from(found != filtered);
            }
            let all = every_row(table);
            [
                12,
                u64::try_from(seen).map_err(|_| ())?,
                differing,
                u64::from(all.end.0 - all.first.0),
            ]
        }
        Scenario::PerNodeColumn => {
            let rows = [(7, 1), (3, 2), (11, 3)]
                .into_iter()
                .map(|(place, serial)| row_of(Relation::StringLiteral, place, serial));
            let set = one(Relation::StringLiteral, rows.collect());
            let table = table_of(&set, Relation::StringLiteral);
            let occurrence = Column::StringLiteral(StringLiteralColumn::Occurrence);
            let value = Column::StringLiteral(StringLiteralColumn::Value);
            let all = every_row(table);
            [
                length(rows_of(table, occurrence, FactKey::Node(Position(3)))),
                length(rows_of(table, occurrence, FactKey::Node(Position(4)))),
                read(
                    table,
                    value,
                    rows_of(table, occurrence, FactKey::Node(Position(3))),
                )
                .first()
                .map_or(0, |cell| cell.0),
                u64::from(all.end.0 - all.first.0),
            ]
        }
        Scenario::SecondKey => {
            // Source, target, link kind, file, span: sources 4, 2, 6 and targets 8, 9, 8.
            let rows =
                [(4, 8, 1), (2, 9, 2), (6, 8, 3)]
                    .into_iter()
                    .map(|(source, target, serial)| {
                        let mut cells = row_of(Relation::TokenLink, source, serial);
                        if let Some(cell) = cells.get_mut(1) {
                            *cell = node(target);
                        }
                        cells
                    });
            let set = one(Relation::TokenLink, rows.collect());
            let table = table_of(&set, Relation::TokenLink);
            let target = Column::TokenLink(TokenLinkColumn::Target);
            let source = Column::TokenLink(TokenLinkColumn::Source);
            let sources = read(
                table,
                source,
                rows_of(table, target, FactKey::Node(Position(8))),
            );
            [
                length(rows_of(table, target, FactKey::Node(Position(8)))),
                length(rows_of(table, target, FactKey::Node(Position(9)))),
                u64::from(sources == [node(4), node(6)]),
                length(rows_of(table, target, FactKey::Node(Position(4)))),
            ]
        }
        Scenario::WordKey => {
            let mut words = file_words(&base_words(CheckedNames::default(), &[]));
            let (first, second) = (
                intern(&mut words, NameText("x")),
                intern(&mut words, NameText("y")),
            );
            let rows = (4..7)
                .map(|place| row_of(Relation::BindingRead, place, 9))
                .collect();
            let set = one(
                Relation::BindingRead,
                with_cells(rows, 1, &[first, second, first]),
            );
            let table = table_of(&set, Relation::BindingRead);
            let name = Column::BindingRead(BindingReadColumn::Name);
            let unknown = intern(&mut words, NameText("z"));
            [
                length(rows_of(table, name, FactKey::Word(first))),
                length(rows_of(table, name, FactKey::Word(second))),
                length(rows_of(table, name, FactKey::Word(unknown))),
                u64::from(presence(table, name, FactKey::Word(second)) == KeyPresence::Held),
            ]
        }
        Scenario::EntityKey => {
            let base = base_words(CheckedNames::default(), &[]);
            let (mut one_worker, mut other) = (file_words(&base), file_words(&base));
            let module = entity(&mut one_worker, NameText("pkg.module"));
            let sibling = entity(&mut other, NameText("pkg.sibling"));
            clear_words(&mut one_worker);
            let again = entity(&mut other, NameText("pkg.module"));
            // File, path, module name, package name, package root name.
            let rows = vec![
                row_of(Relation::ModuleIdentity, 0, 1),
                row_of(Relation::ModuleIdentity, 0, 2),
            ];
            let paths = [
                CellWord(u64::from(module.0)),
                CellWord(u64::from(sibling.0)),
            ];
            let set = one(Relation::ModuleIdentity, with_cells(rows, 1, &paths));
            let table = table_of(&set, Relation::ModuleIdentity);
            let path = Column::ModuleIdentity(ModuleIdentityColumn::Path);
            let file = Column::ModuleIdentity(ModuleIdentityColumn::File);
            [
                length(rows_of(table, path, FactKey::Entity(again))),
                read(table, file, rows_of(table, path, FactKey::Entity(sibling)))
                    .first()
                    .map_or(0, |cell| cell.0),
                u64::from(again == module),
                u64::from(
                    presence(table, path, FactKey::Node(Position(module.0))) == KeyPresence::Absent,
                ),
            ]
        }
        Scenario::PresenceAgrees => {
            let set = one(Relation::ExpressionType, expression_types());
            let table = table_of(&set, Relation::ExpressionType);
            let (mut held, mut differing) = (0, 0);
            for place in 0..12 {
                let key = FactKey::Node(Position(place));
                let has = presence(table, EXPRESSION, key) == KeyPresence::Held;
                held += u64::from(has);
                differing += u64::from(has != (length(rows_of(table, EXPRESSION, key)) > 0));
            }
            [12, held, differing, 0]
        }
        Scenario::KeyNoRowHolds => {
            let set = one(Relation::ExpressionType, expression_types());
            let table = table_of(&set, Relation::ExpressionType);
            [
                length(rows_of(table, EXPRESSION, FactKey::Word(CellWord(5)))),
                u64::from(
                    presence(table, EXPRESSION, FactKey::Word(CellWord(5))) == KeyPresence::Absent,
                ),
                length(rows_of(
                    table,
                    EXPRESSION,
                    FactKey::Node(Position(1_000_000)),
                )),
                u64::from(
                    presence(table, EXPRESSION, FactKey::Node(Position(3))) == KeyPresence::Absent,
                ),
            ]
        }
        Scenario::EveryRelation => {
            let (mut relations, mut differing, mut keys) = (0_u64, 0_u64, 0_u64);
            for relation in ALL
                .iter()
                .copied()
                .filter(|relation| matches!(fact_form(*relation), FactForm::Filled(_)))
            {
                relations += 1;
                let rows = [(4, 1), (2, 2), (4, 3)]
                    .into_iter()
                    .map(|(place, serial)| row_of(relation, place, serial));
                let set = one(relation, rows.collect());
                let table = table_of(&set, relation);
                differing += u64::from(every_row(table).end.0 != 3);
                let filled_by = match fact_form(relation) {
                    FactForm::Filled(FilledKey::Unique(column) | FilledKey::Many(column)) => {
                        vec![column]
                    }
                    FactForm::Filled(FilledKey::Whole) | FactForm::NodeTable => Vec::new(),
                };
                let node_keys = filled_by
                    .into_iter()
                    .chain(key_columns(relation).iter().copied())
                    .filter(|column| kind(*column) == CellKind::Node);
                for column in node_keys {
                    keys += 1;
                    differing +=
                        u64::from(length(rows_of(table, column, FactKey::Node(Position(4)))) != 2);
                    differing +=
                        u64::from(length(rows_of(table, column, FactKey::Node(Position(2)))) != 1);
                }
            }
            // 75 filled relations at 4e26fdf05: a relation added later only raises the count.
            [
                u64::from(relations >= 75),
                differing,
                u64::from(keys > 40),
                u64::from(ALL.is_sorted()),
            ]
        }
    })
}

#[cfg(test)]
mod answers_test;

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
