//! The service of the `reading` part: the bytes `writing` wrote read back as a rule set's programs, section by section.
//! Anything that is not exactly what `writing` writes is no pack: bytes that end early, a count the bytes left cannot
//! hold, an operation or a kind no code names, a relation the catalog does not name, bytes after the last section.

use crate::domain::code::types::{
    BranchCode, CodeEntry, CodeWord, FillCode, IndexCode, MeasuredForm, NeedSite, Op, ProgramCode,
    RuleCode, ShapeChoice, Short, SizeSite, StateCounts, TableCode, UnitCode, UnitKey, WordFlag,
    WordSlot,
};
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::packs::constants::{CALL_FORM, OPEN_SHAPE, PATTERN_KEY, RULE_KEY, TABLE_KEY};
use crate::domain::programs::types::{ProgramsEngine, RuleSteps};
use crate::domain::scratch::types::NodeNeed;
use crate::domain::steps::types::{ColumnMask, ConstantText, MadeColumn, StepRegex};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service::relation_named;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::query_plans::types::{NodeDigest, NodeDigests};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// Every operation, by the byte it is written as: its own number.
const OPS: [Op; 46] = [
    Op::End,
    Op::EachKind,
    Op::EachMatchNode,
    Op::EachMatchFilled,
    Op::EachMatchFilledMany,
    Op::EachMatchMade,
    Op::EachRange,
    Op::EachRowNode,
    Op::EachRowFilled,
    Op::EachRowMade,
    Op::Children,
    Op::ChildrenOfField,
    Op::Axis,
    Op::Within,
    Op::Nearest,
    Op::NearestDerived,
    Op::Closure,
    Op::ReadNode,
    Op::ReadFilled,
    Op::ReadMade,
    Op::Test,
    Op::TestEqualConstant,
    Op::TestNotEqualConstant,
    Op::TestEqualRegister,
    Op::TestNotEqualRegister,
    Op::TestInConstants,
    Op::Call,
    Op::Render,
    Op::Let,
    Op::Exists,
    Op::Or,
    Op::Fork,
    Op::Yield,
    Op::Latest,
    Op::Emit,
    Op::StageScan,
    Op::StageProbe,
    Op::StageLatest,
    Op::StageRead,
    Op::StageKeep,
    Op::StageCall,
    Op::StageProduct,
    Op::StageUse,
    Op::StageRender,
    Op::StageWalk,
    Op::StageWrite,
];

/// The bytes of a pack being read, and how far the read is.
pub(crate) struct Reading<'b> {
    /// The bytes.
    pub(crate) bytes: &'b [u8],
    /// The next byte to read.
    pub(crate) at: usize,
}

impl<'b> Reading<'b> {
    /// The next `width` bytes.
    fn taken(&mut self, width: usize) -> Option<&'b [u8]> {
        let end = self.at.checked_add(width)?;
        let held = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(held)
    }

    /// A byte.
    fn byte(&mut self) -> Option<u8> {
        self.taken(1)?.first().copied()
    }

    /// A number of thirty-two bits.
    fn number(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.taken(4)?.try_into().ok()?))
    }

    /// A number that must fit sixteen bits.
    fn short(&mut self) -> Option<u16> {
        u16::try_from(self.number()?).ok()
    }

    /// How many of something follow, each at least `width` bytes: never more than the bytes left hold.
    fn count(&mut self, width: usize) -> Option<usize> {
        let count = usize::try_from(self.number()?).ok()?;
        let left = self.bytes.len().checked_sub(self.at)?;
        (count <= left / width.max(1)).then_some(count)
    }

    /// A text.
    fn text(&mut self) -> Option<&'b str> {
        let length = self.count(1)?;
        std::str::from_utf8(self.taken(length)?).ok()
    }

    /// A node's digest.
    fn digest(&mut self) -> Option<NodeDigest> {
        Some(NodeDigest(self.taken(16)?.try_into().ok()?))
    }

    /// Slots.
    fn slots(&mut self) -> Option<Vec<WordSlot>> {
        let count = self.count(4)?;
        (0..count).map(|_| self.number().map(WordSlot)).collect()
    }

    /// Relations, by their catalog names.
    fn relations(&mut self) -> Option<Vec<Relation>> {
        let count = self.count(4)?;
        (0..count)
            .map(|_| relation_named(NameRef(self.text()?)))
            .collect()
    }

    /// The kinds of a row's columns.
    fn kinds(&mut self) -> Option<ColumnKinds> {
        let count = self.count(1)?;
        let kinds: Option<Vec<CellKind>> = (0..count)
            .map(|_| match self.byte()? {
                0 => Some(CellKind::Node),
                1 => Some(CellKind::Symbol),
                2 => Some(CellKind::File),
                3 => Some(CellKind::Span),
                4 => Some(CellKind::Number),
                5 => Some(CellKind::Flag),
                _ => None,
            })
            .collect();
        kinds.map(ColumnKinds)
    }

    /// The columns of a mask.
    fn mask(&mut self) -> Option<ColumnMask> {
        let count = self.count(4)?;
        let columns: Option<Vec<MadeColumn>> =
            (0..count).map(|_| self.short().map(MadeColumn)).collect();
        Some(ColumnMask(columns?.into()))
    }

    /// One word.
    fn word(&mut self) -> Option<CodeWord> {
        let op = self.byte()?;
        let op = OPS.get(usize::from(op)).copied().filter(|held| *held as u8 == op)?;
        let flag = WordFlag(self.byte()?);
        let short = Short(u16::from_le_bytes(self.taken(2)?.try_into().ok()?));
        Some(CodeWord {
            op,
            flag,
            short,
            a: WordSlot(self.number()?),
            b: WordSlot(self.number()?),
            x: WordSlot(self.number()?),
        })
    }

    /// One unit.
    fn unit(&mut self) -> Option<UnitCode> {
        let kind = self.byte()?;
        let node = self.digest()?;
        let key = if kind == PATTERN_KEY {
            let bound = self.mask()?;
            UnitKey::Pattern(node, bound, self.mask()?)
        } else if kind == RULE_KEY {
            UnitKey::Rule(node)
        } else if kind == TABLE_KEY {
            UnitKey::Table(node, FillCode(self.short()?))
        } else {
            return None;
        };
        let form = match self.byte()? {
            form if form == CALL_FORM => MeasuredForm::Call,
            form if form == CALL_FORM + 1 => MeasuredForm::Table,
            _ => return None,
        };
        let shape = match self.byte()? {
            shape if shape == OPEN_SHAPE => ShapeChoice::Open,
            shape if shape == OPEN_SHAPE + 1 => ShapeChoice::Decided,
            _ => return None,
        };
        Some(UnitCode {
            key,
            form,
            shape,
            first: WordSlot(self.number()?),
            points: WordSlot(self.number()?),
        })
    }

    /// One rule's entries.
    fn rule(&mut self) -> Option<RuleCode> {
        let rule = SelectedRuleOrdinal(self.number()?);
        let unit = WordSlot(self.number()?);
        let count = self.count(8)?;
        let branches: Option<Vec<BranchCode>> = (0..count)
            .map(|_| {
                Some(BranchCode {
                    need: NeedSite(self.number()?),
                    entry: CodeEntry(self.number()?),
                })
            })
            .collect();
        let emit = self.kinds()?;
        let count = self.count(4)?;
        let frontier: Option<Vec<ColumnKinds>> = (0..count).map(|_| self.kinds()).collect();
        Some(RuleCode {
            rule,
            unit,
            branches: branches?.into(),
            columns: RuleColumns {
                emit,
                frontier: frontier?,
            },
        })
    }

    /// The code.
    fn code(&mut self) -> Option<ProgramCode> {
        let mut held = [WordSlot(0); 8];
        for slot in &mut held {
            *slot = WordSlot(self.number()?);
        }
        let [registers, memos, indexes, tables, needs, relations, points, units] = held;
        let counts = StateCounts {
            registers,
            memos,
            indexes,
            tables,
            needs,
            relations,
            points,
            units,
        };
        let count = self.count(16)?;
        let words: Option<Vec<CodeWord>> = (0..count).map(|_| self.word()).collect();
        let pool = self.slots()?;
        let count = self.count(8)?;
        let constants: Option<Vec<CellWord>> = (0..count)
            .map(|_| Some(CellWord(u64::from_le_bytes(self.taken(8)?.try_into().ok()?))))
            .collect();
        let needs = self.slots()?;
        let points = self.slots()?;
        let count = self.count(16)?;
        let tables: Option<Vec<TableCode>> = (0..count)
            .map(|_| {
                Some(TableCode {
                    fill: Short(self.short()?),
                    operands: WordSlot(self.number()?),
                    need: NeedSite(self.number()?),
                    unit: WordSlot(self.number()?),
                })
            })
            .collect();
        let count = self.count(16)?;
        let rules: Option<Vec<RuleCode>> = (0..count).map(|_| self.rule()).collect();
        let relations = self.relations()?;
        let count = self.count(16)?;
        let indexes: Option<Vec<IndexCode>> = (0..count)
            .map(|_| {
                Some(IndexCode {
                    source: Short(self.short()?),
                    number: WordSlot(self.number()?),
                    columns: WordSlot(self.number()?),
                    ordered: WordSlot(self.number()?),
                })
            })
            .collect();
        let count = self.count(27)?;
        let units: Option<Vec<UnitCode>> = (0..count).map(|_| self.unit()).collect();
        let count = self.count(12)?;
        let sizes: Option<Vec<SizeSite>> = (0..count)
            .map(|_| {
                Some(SizeSite {
                    relation: WordSlot(self.number()?),
                    column: Short(self.short()?),
                    value: WordSlot(self.number()?),
                })
            })
            .collect();
        Some(ProgramCode {
            words: words?,
            pool,
            constants: constants?,
            needs,
            tables: tables?,
            rules: rules?,
            relations,
            indexes: indexes?,
            points,
            units: units?,
            sizes: sizes?,
            counts,
        })
    }
}

/// The programs the bytes after a pack's header hold, their words left to the load (which has the vocabulary): none when
/// the bytes are not exactly what `writing` writes.
pub(crate) fn read(bytes: &[u8]) -> Option<ProgramsEngine> {
    let mut reading = Reading { bytes, at: 0 };
    let code = reading.code()?;
    let count = reading.count(4)?;
    let texts: Option<Vec<ConstantText>> = (0..count)
        .map(|_| reading.text().map(|text| ConstantText(text.into())))
        .collect();
    let count = reading.count(4)?;
    let regexes: Option<Vec<StepRegex>> = (0..count)
        .map(|_| reading.text().map(|text| StepRegex(text.into())))
        .collect();
    let reads = reading.relations()?;
    let count = reading.count(12)?;
    let steps: Option<Vec<RuleSteps>> = (0..count)
        .map(|_| {
            let rule = SelectedRuleOrdinal(reading.number()?);
            let emitted = StepId(reading.number()?);
            let nodes = reading.count(16)?;
            let nodes: Option<Vec<NodeDigest>> = (0..nodes).map(|_| reading.digest()).collect();
            Some(RuleSteps {
                rule,
                emitted,
                nodes: NodeDigests(nodes?),
            })
        })
        .collect();
    let count = reading.count(1)?;
    let node_needs: Option<Vec<Option<NodeNeed>>> = (0..count)
        .map(|_| match reading.byte()? {
            0 => Some(Some(NodeNeed::NotAsked)),
            1 => Some(Some(NodeNeed::Asked)),
            2 => Some(None),
            _ => None,
        })
        .collect();
    (reading.at == bytes.len()).then_some(())?;
    Some(ProgramsEngine {
        code,
        texts: texts?,
        regexes: regexes?,
        reads,
        steps: steps?,
        node_needs: node_needs?,
        ..ProgramsEngine::default()
    })
}
