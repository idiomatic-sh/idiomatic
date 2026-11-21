//! The service of the `writing` part: the programs of a rule set written after a pack's header, a section each, every
//! number the low byte first: the state counts; the words (sixteen bytes each); the pool, the constants, the need
//! formulas and the plan points; the tables; the rules with their branches and column kinds; the relations, by catalog
//! name; the indexes; the units; the size sites; the constant texts and the regular expressions' sources; the relations
//! read; each rule's plan-step digests; and for every need site whether its formula asks the node rows. The words of the
//! run are not written: a load rebuilds them from the vocabulary it is handed and the texts.

use crate::domain::code::types::{
    CodeWord, MeasuredForm, ProgramCode, ShapeChoice, UnitKey, WordSlot,
};
use crate::domain::packs::constants::{CALL_FORM, OPEN_SHAPE, PATTERN_KEY, RULE_KEY, TABLE_KEY};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::NodeNeed;
use crate::domain::steps::types::ColumnMask;
use ryo_relations::domain::relations::service::name;
use ryo_relations::domain::relations::types::{CellKind, Relation};

/// Writes a number of thirty-two bits.
pub(crate) fn number(bytes: &mut Vec<u8>, number: u32) {
    bytes.extend_from_slice(&number.to_le_bytes());
}

/// Writes how many of something follow.
fn count(bytes: &mut Vec<u8>, count: usize) {
    number(bytes, u32::try_from(count).unwrap_or(u32::MAX));
}

/// Writes a text: its length in bytes, then its bytes.
fn text(bytes: &mut Vec<u8>, text: &str) {
    count(bytes, text.len());
    bytes.extend_from_slice(text.as_bytes());
}

/// Writes slots: how many, then each.
fn slots(bytes: &mut Vec<u8>, slots: &[WordSlot]) {
    count(bytes, slots.len());
    for slot in slots {
        number(bytes, slot.0);
    }
}

/// Writes relations by their catalog names.
fn relations(bytes: &mut Vec<u8>, relations: &[Relation]) {
    count(bytes, relations.len());
    for relation in relations {
        text(bytes, name(*relation).0);
    }
}

/// The byte a column's kind is written as.
const fn kind_byte(kind: CellKind) -> u8 {
    match kind {
        CellKind::Node => 0,
        CellKind::Symbol => 1,
        CellKind::File => 2,
        CellKind::Span => 3,
        CellKind::Number => 4,
        CellKind::Flag => 5,
    }
}

/// Writes the kinds of a row's columns.
fn kinds(bytes: &mut Vec<u8>, kinds: &[CellKind]) {
    count(bytes, kinds.len());
    bytes.extend(kinds.iter().map(|kind| kind_byte(*kind)));
}

/// Writes the columns of a mask.
fn mask(bytes: &mut Vec<u8>, mask: &ColumnMask) {
    count(bytes, mask.0.len());
    for column in &mask.0 {
        number(bytes, u32::from(column.0));
    }
}

/// Writes one word: its sixteen bytes.
fn word(bytes: &mut Vec<u8>, word: CodeWord) {
    bytes.push(word.op as u8);
    bytes.push(word.flag.0);
    bytes.extend_from_slice(&word.short.0.to_le_bytes());
    for slot in [word.a, word.b, word.x] {
        number(bytes, slot.0);
    }
}

/// Writes the code.
fn code(bytes: &mut Vec<u8>, code: &ProgramCode) {
    let counts = code.counts;
    for held in [
        counts.registers,
        counts.memos,
        counts.indexes,
        counts.tables,
        counts.needs,
        counts.relations,
        counts.points,
        counts.units,
    ] {
        number(bytes, held.0);
    }
    count(bytes, code.words.len());
    for each in &code.words {
        word(bytes, *each);
    }
    slots(bytes, &code.pool);
    count(bytes, code.constants.len());
    for constant in &code.constants {
        bytes.extend_from_slice(&constant.0.to_le_bytes());
    }
    slots(bytes, &code.needs);
    slots(bytes, &code.points);
    count(bytes, code.tables.len());
    for table in &code.tables {
        for held in [
            u32::from(table.fill.0),
            table.operands.0,
            table.need.0,
            table.unit.0,
        ] {
            number(bytes, held);
        }
    }
    count(bytes, code.rules.len());
    for rule in &code.rules {
        number(bytes, rule.rule.0);
        number(bytes, rule.unit.0);
        count(bytes, rule.branches.len());
        for branch in &rule.branches {
            number(bytes, branch.need.0);
            number(bytes, branch.entry.0);
        }
        kinds(bytes, &rule.columns.emit.0);
        count(bytes, rule.columns.frontier.len());
        for frontier in &rule.columns.frontier {
            kinds(bytes, &frontier.0);
        }
    }
    relations(bytes, &code.relations);
    count(bytes, code.indexes.len());
    for index in &code.indexes {
        for held in [
            u32::from(index.source.0),
            index.number.0,
            index.columns.0,
            index.ordered.0,
        ] {
            number(bytes, held);
        }
    }
    count(bytes, code.units.len());
    for unit in &code.units {
        match &unit.key {
            UnitKey::Pattern(node, bound, collected) => {
                bytes.push(PATTERN_KEY);
                bytes.extend_from_slice(&node.0);
                mask(bytes, bound);
                mask(bytes, collected);
            }
            UnitKey::Rule(node) => {
                bytes.push(RULE_KEY);
                bytes.extend_from_slice(&node.0);
            }
            UnitKey::Table(node, fill) => {
                bytes.push(TABLE_KEY);
                bytes.extend_from_slice(&node.0);
                number(bytes, u32::from(fill.0));
            }
        }
        bytes.push(match unit.form {
            MeasuredForm::Call => CALL_FORM,
            MeasuredForm::Table => CALL_FORM + 1,
        });
        bytes.push(match unit.shape {
            ShapeChoice::Decided => OPEN_SHAPE + 1,
            ShapeChoice::Open => OPEN_SHAPE,
        });
        number(bytes, unit.first.0);
        number(bytes, unit.points.0);
    }
    count(bytes, code.sizes.len());
    for site in &code.sizes {
        for held in [site.relation.0, u32::from(site.column.0), site.value.0] {
            number(bytes, held);
        }
    }
}

/// The programs of a rule set as the bytes after a pack's header.
pub(crate) fn written(bytes: &mut Vec<u8>, programs: &ProgramsEngine) {
    code(bytes, &programs.code);
    count(bytes, programs.texts.len());
    for each in &programs.texts {
        text(bytes, &each.0);
    }
    count(bytes, programs.regexes.len());
    for regex in &programs.regexes {
        text(bytes, &regex.0);
    }
    relations(bytes, &programs.reads);
    count(bytes, programs.steps.len());
    for steps in &programs.steps {
        number(bytes, steps.rule.0);
        number(bytes, steps.emitted.0);
        count(bytes, steps.nodes.0.len());
        for node in &steps.nodes.0 {
            bytes.extend_from_slice(&node.0);
        }
    }
    count(bytes, programs.node_needs.len());
    bytes.extend(programs.node_needs.iter().map(|need| match need {
        Some(NodeNeed::Asked) => 1,
        Some(NodeNeed::NotAsked) => 0,
        None => 2,
    }));
}
