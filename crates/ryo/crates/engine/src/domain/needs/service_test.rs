//! The `needs` service's cases: each kind of record, a formula of all and of any, a site that asks another, and a decision
//! asked twice and walked once.

use super::service::decided;
use super::types::{NeedFacts, NeedVerdict};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::records::service::{push_list, push_need};
use crate::domain::code::records::types::NeedRecord;
use crate::domain::code::types::{NeedSite, PoolAt, ProgramCode, RelationNumber, WordSlot};
use crate::domain::steps::types::MadeTableId;
use ryo_relations::domain::relations::types::KindBits;

/// The file a case decides for: kind 4 is present, relation 1 has a row, table 2 has a row; nothing else has.
#[derive(Debug, Default)]
struct File {
    /// The decisions kept, by site.
    kept: Vec<Option<NeedVerdict>>,
    /// How many times a leaf was asked.
    asked: u32,
}

/// A yes or no as a verdict.
fn verdict(holds: bool) -> NeedVerdict {
    if holds {
        NeedVerdict::Holds
    } else {
        NeedVerdict::Empty
    }
}

impl NeedFacts for File {
    fn kept(&self, site: NeedSite) -> Option<NeedVerdict> {
        self.kept.get(site.0 as usize).copied().flatten()
    }

    fn keep(&mut self, site: NeedSite, kept: NeedVerdict) {
        if self.kept.len() <= site.0 as usize {
            self.kept.resize(site.0 as usize + 1, None);
        }
        self.kept[site.0 as usize] = Some(kept);
    }

    fn kind_present(&mut self, kind: KindBits) -> NeedVerdict {
        self.asked += 1;
        verdict(kind.0 == 4)
    }

    fn relation_row(
        &mut self,
        relation: RelationNumber,
        column: WordSlot,
        constant: WordSlot,
    ) -> NeedVerdict {
        self.asked += 1;
        verdict(relation.0 == 1 && (column == NO_SLOT || constant.0 == 8))
    }

    fn table_row(&mut self, table: MadeTableId) -> NeedVerdict {
        self.asked += 1;
        verdict(table.0 == 2)
    }
}

/// The formula a case decides.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// Nothing is known.
    Unknown,
    /// A node of one of two kinds.
    Kinds(u32, u32),
    /// A row of a relation, with a constant in column 0 or with none (`u32::MAX`).
    Row(u32, u32),
    /// A table's row.
    Table(u32),
    /// Every one of: kind 4 present, and a row of the relation.
    All(u32),
    /// Some one of: a row of the relation, or kind 4 present.
    Any(u32),
    /// Another site's decision, which is a row of the relation; asked twice.
    SiteTwice(u32),
}

type Input = Asked;
/// 1 when the need holds, 0 when the unit is empty; and how many times a leaf was asked.
type Value = (u8, u32);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nothing_known_holds"),
        word: Word::Succeeds,
        input: || Asked::Unknown,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_kind_of_the_list_is_present"),
        word: Word::Succeeds,
        input: || Asked::Kinds(9, 4),
        expect: Expect::Value((1, 2)),
    },
    Case {
        name: CaseName("no_kind_of_the_list_is_present"),
        word: Word::Succeeds,
        input: || Asked::Kinds(9, 5),
        expect: Expect::Value((0, 2)),
    },
    Case {
        name: CaseName("a_relation_with_a_row"),
        word: Word::Succeeds,
        input: || Asked::Row(1, u32::MAX),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_relation_with_no_row_of_the_value"),
        word: Word::Succeeds,
        input: || Asked::Row(1, 0),
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("a_table_with_a_row"),
        word: Word::Succeeds,
        input: || Asked::Table(2),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_table_with_no_row"),
        word: Word::Succeeds,
        input: || Asked::Table(3),
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("all_holds_when_every_part_holds"),
        word: Word::Succeeds,
        input: || Asked::All(1),
        expect: Expect::Value((1, 2)),
    },
    Case {
        name: CaseName("all_is_empty_when_one_part_is"),
        word: Word::Succeeds,
        input: || Asked::All(7),
        expect: Expect::Value((0, 2)),
    },
    Case {
        name: CaseName("any_holds_at_its_first_part_that_holds"),
        word: Word::Succeeds,
        input: || Asked::Any(1),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("any_holds_by_the_part_that_costs_nothing_and_asks_no_relation"),
        word: Word::Succeeds,
        input: || Asked::Any(7),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_site_asked_twice_is_walked_once"),
        word: Word::Succeeds,
        input: || Asked::SiteTwice(1),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("an_empty_site_asked_twice_is_walked_once"),
        word: Word::Succeeds,
        input: || Asked::SiteTwice(7),
        expect: Expect::Value((0, 1)),
    },
];

/// A row record of a relation with no value.
fn row(relation: u32) -> NeedRecord {
    NeedRecord::Row {
        relation: RelationNumber(relation),
        column: NO_SLOT,
        constant: NO_SLOT,
    }
}

/// A record of parts, written with the places of its parts after it.
fn compound(needs: &mut Vec<WordSlot>, all: bool, relation: u32, list: PoolAt) -> WordSlot {
    let kind = push_need(needs, NeedRecord::Kinds(list));
    let rows = push_need(needs, row(relation));
    let (parts, offsets) = (WordSlot(2), WordSlot(0));
    if all {
        let at = push_need(needs, NeedRecord::All { parts, offsets });
        needs.extend([kind, rows]);
        at
    } else {
        let at = push_need(needs, NeedRecord::Any { parts, offsets });
        needs.extend([rows, kind]);
        at
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    // Sites 0 and 1 come first; site 0 is the formula asked, site 1 a row of the relation a `SiteTwice` names.
    let mut code = ProgramCode {
        needs: vec![WordSlot(0), WordSlot(0)],
        pool: vec![WordSlot(0)],
        ..ProgramCode::default()
    };
    let present = push_list(&mut code.pool, &[WordSlot(4)]);
    let place = match asked {
        Asked::Unknown => push_need(&mut code.needs, NeedRecord::Unknown),
        Asked::Kinds(one, other) => {
            let list = push_list(&mut code.pool, &[WordSlot(one), WordSlot(other)]);
            push_need(&mut code.needs, NeedRecord::Kinds(list))
        }
        Asked::Row(relation, value) => push_need(
            &mut code.needs,
            if value == u32::MAX {
                row(relation)
            } else {
                NeedRecord::Row {
                    relation: RelationNumber(relation),
                    column: WordSlot(0),
                    constant: WordSlot(value),
                }
            },
        ),
        Asked::Table(table) => push_need(&mut code.needs, NeedRecord::Table(MadeTableId(table))),
        Asked::All(relation) => compound(&mut code.needs, true, relation, present),
        Asked::Any(relation) => compound(&mut code.needs, false, relation, present),
        Asked::SiteTwice(relation) => {
            let other = push_need(&mut code.needs, row(relation));
            code.needs[1] = other;
            push_need(&mut code.needs, NeedRecord::Site(NeedSite(1)))
        }
    };
    code.needs[0] = place;
    let mut file = File::default();
    let first = decided(&mut file, &code, NeedSite(0));
    let again = decided(&mut file, &code, NeedSite(0));
    if first == again {
        Ok((u8::from(first == NeedVerdict::Holds), file.asked))
    } else {
        Err(())
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
