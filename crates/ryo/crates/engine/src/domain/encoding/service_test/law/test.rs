//! The encoding law: the pack that holds every step and every stage encodes, and each word, read back through its code's own
//! reader, gives the operands written. The expected text is the encoding's own listing, read line by line against the operand
//! table before it was recorded; there is no decoder to steps.

use super::super::fixtures::listing;
use super::fixtures::every_variant;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::ProgramCode;
use crate::domain::encoding::service::encoded;
use crate::domain::lowering::errors::LoweringError;
use std::borrow::Cow;

/// What a row reads of the law's code.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Every word, read back.
    Words,
    /// The tables of the code beside its words: what is numbered and counted.
    Tables,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = LoweringError;

/// Every word of the law's code, as its reader reads it.
const WORDS: &str = "\
0 End no operand @-\n\
1 Test TestWord { flag: WordFlag(2), comparison: Short(8), left: WordSlot(20), right: WordSlot(0), regex: RegexId(4) } @0\n\
2 Yield YieldWord { cells: Short(0), row: WordSlot(16) } @1\n\
3 End no operand @-\n\
4 Yield YieldWord { cells: Short(1), row: WordSlot(13) } @0\n\
5 End no operand @-\n\
6 Emit EmitWord { flag: WordFlag(0), columns: Short(1), rule: SelectedRuleOrdinal(0), input: WordSlot(0), operands: PoolAt(76) } @0\n\
7 End no operand @-\n\
8 EachKind EachKindWord { flag: WordFlag(1), kind: KindBits(0), out: Register(0), kinds: PoolAt(0) } @0\n\
9 EachMatchFilled EachMatchWord { flag: WordFlag(0), out: Register(1), value: WordSlot(0), index: IndexSite(0) } @1\n\
10 EachMatchMade EachMatchMadeWord { flag: WordFlag(0), values: Short(0), out: Register(16), value: WordSlot(1), index: IndexSite(1) } @2\n\
11 EachRange EachRangeWord { flag: WordFlag(19), values: Short(1), out: Register(2), operands: PoolAt(9), index: IndexSite(2) } @3\n\
12 EachRowMade EachRowMadeWord { flag: WordFlag(4), out: Register(4), table: MadeTableId(3) } @4\n\
13 ChildrenOfField ChildrenOfFieldWord { flag: WordFlag(0), field: FieldCode(6), parent: Register(0), out: Register(5) } @5\n\
14 Axis AxisWord { flag: WordFlag(0), direction: WordFlag(3), start: Register(5), out: Register(6) } @6\n\
15 Within WithinWord { flag: WordFlag(0), ancestor: Register(0), descendant: Register(6) } @7\n\
16 Nearest NearestWord { flag: WordFlag(1), start: Register(6), out: Register(7), operands: PoolAt(17) } @8\n\
17 Closure ClosureWord { seed: Register(7), out: Register(8), operands: PoolAt(21) } @9\n\
18 ReadNode ReadNodeWord { column: Short(1), row: Register(8), out: Register(9) } @10\n\
19 Test TestWord { flag: WordFlag(4), comparison: Short(6), left: WordSlot(9), right: WordSlot(29), regex: RegexId(0) } @11\n\
20 Call CallWord { meaning: Short(4), arguments: PoolAt(32), outputs: PoolAt(37), regex: RegexId(2) } @12\n\
21 Render RenderWord { context: Short(2), node: Register(8), out: Register(11) } @13\n\
22 Let LetWord { flag: WordFlag(2), from: WordSlot(0), out: Register(12) } @14\n\
23 Exists ExistsWord { flag: WordFlag(1), keys: Short(0), body: PoolAt(41), key: PoolAt(47), memo: MemoSite(4294967295) } @15\n\
24 Or OrWord { flag: WordFlag(2), bodies: Short(3), records: PoolAt(47), outputs: PoolAt(64), memo: MemoSite(2) } @16\n\
25 Latest LatestWord { flag: WordFlag(1), keys: Short(1), out: Register(15), bound: WordSlot(9), operands: PoolAt(69) } @17\n\
26 Emit EmitWord { flag: WordFlag(1), columns: Short(2), rule: SelectedRuleOrdinal(0), input: WordSlot(1), operands: PoolAt(72) } @18\n\
27 Fork ForkWord { slots: Short(1), entries: PoolAt(78) } @19\n\
28 End no operand @-\n\
29 StageScan ScanWord { flag: WordFlag(0), out: Register(30), constants: PoolAt(0), read: WordSlot(1) } @20\n\
30 StageProbe ProbeWord { flag: WordFlag(0), values: Short(1), out: Register(31), probed: WordSlot(30), index: IndexSite(0) } @21\n\
31 StageProbe ProbeWord { flag: WordFlag(13), values: Short(1), out: Register(32), probed: WordSlot(80), index: IndexSite(2) } @22\n\
32 StageProduct ProductWord { flag: WordFlag(2), out: Register(33), read: WordSlot(1) } @23\n\
33 StageUse UseWord { flag: WordFlag(0), outputs: Short(1), body: PoolAt(86), registers: PoolAt(93), memo: MemoSite(1) } @24\n\
34 StageProbe ProbeWord { flag: WordFlag(4), values: Short(1), out: Register(100), probed: WordSlot(95), index: IndexSite(3) } @25\n\
35 StageRead StageReadWord { flag: WordFlag(2), column: Short(1), row: Register(100), out: Register(35), read: WordSlot(1) } @-\n\
36 StageLatest LatestWord { flag: WordFlag(6), keys: Short(1), out: Register(36), bound: WordSlot(3), operands: PoolAt(97) } @26\n\
37 StageRead StageReadWord { flag: WordFlag(2), column: Short(1), row: Register(36), out: Register(37), read: WordSlot(2) } @27\n\
38 StageKeep TestWord { flag: WordFlag(0), comparison: Short(2), left: WordSlot(37), right: WordSlot(30), regex: RegexId(0) } @28\n\
39 StageCall CallWord { meaning: Short(8), arguments: PoolAt(100), outputs: PoolAt(103), regex: RegexId(0) } @29\n\
40 StageRender RenderWord { context: Short(0), node: Register(30), out: Register(39) } @30\n\
41 StageWalk WalkWord { flag: WordFlag(3), field: FieldCode(0), start: Register(30), out: Register(40) } @31\n\
42 StageWalk WalkWord { flag: WordFlag(0), field: FieldCode(6), start: Register(40), out: Register(41) } @32\n\
43 StageWrite WriteWord { flag: WordFlag(1), columns: Short(2), table: MadeTableId(0), operands: PoolAt(105) } @-\n\
44 StageScan ScanWord { flag: WordFlag(5), out: Register(50), constants: PoolAt(117), read: WordSlot(0) } @33\n\
45 StageWrite WriteWord { flag: WordFlag(0), columns: Short(2), table: MadeTableId(1), operands: PoolAt(119) } @-\n\
46 Children ChildrenWord { flag: WordFlag(0), parent: Register(60), out: Register(61) } @34\n\
47 Yield YieldWord { cells: Short(1), row: WordSlot(61) } @35\n\
48 End no operand @-\n\
49 StageScan ScanWord { flag: WordFlag(2), out: Register(50), constants: PoolAt(0), read: WordSlot(1) } @36\n\
50 StageWrite WriteWord { flag: WordFlag(0), columns: Short(2), table: MadeTableId(2), operands: PoolAt(142) } @-\n\
";

/// What the law's code numbers and counts.
const TABLES: &str = "\
counts StateCounts { registers: WordSlot(101), memos: WordSlot(3), indexes: WordSlot(7), tables: WordSlot(4), needs: WordSlot(7), relations: WordSlot(2), points: WordSlot(37), units: WordSlot(8) }\n\
rules [[BranchCode { need: NeedSite(0), entry: CodeEntry(8) }]]\n\
tables [TableCode { fill: Short(0), operands: WordSlot(109), need: NeedSite(1) }, TableCode { fill: Short(1), operands: WordSlot(123), need: NeedSite(3) }, TableCode { fill: Short(5), operands: WordSlot(146), need: NeedSite(5) }, TableCode { fill: Short(11), operands: WordSlot(134), need: NeedSite(4) }]\n\
indexes [IndexCode { source: Short(1), number: WordSlot(0), columns: WordSlot(3), ordered: WordSlot(4294967295) }, IndexCode { source: Short(2), number: WordSlot(0), columns: WordSlot(5), ordered: WordSlot(4294967295) }, IndexCode { source: Short(2), number: WordSlot(1), columns: WordSlot(7), ordered: WordSlot(1) }, IndexCode { source: Short(2), number: WordSlot(1), columns: WordSlot(39), ordered: WordSlot(4294967295) }, IndexCode { source: Short(2), number: WordSlot(2), columns: WordSlot(67), ordered: WordSlot(1) }, IndexCode { source: Short(2), number: WordSlot(3), columns: WordSlot(132), ordered: WordSlot(4294967295) }, IndexCode { source: Short(2), number: WordSlot(2), columns: WordSlot(140), ordered: WordSlot(4294967295) }]\n\
units Call 0+20, Table 0+0, Table 0+0, Call 20+13, Call 33+1, Call 34+0, Call 34+2, Call 36+1\n\
sizes [SizeSite { relation: WordSlot(0), column: Short(5), value: WordSlot(3) }]\n\
constants [9, 11, 12, 5, 40]\n\
relations [SyntaxError, SyntaxTree]\n\
needs [7, 14, 27, 18, 20, 22, 31, 0, 3, 2, 2, 0, 5, 3, 5, 2, 8, 10, 4, 3, 5, 0, 4, 1, 3, 6, 0, 6, 2, 24, 26, 1, 155]\n\
pool [2, 2, 3, 1, 5, 1, 0, 1, 0, 0, 1, 0, 0, 0, 4294967295, 3, 0, 0, 1, 20, 0, 1, 46, 1, 1, 60, 0, 1, 1, 2, 1, 2, 2, 0, 9, 1, 1, 1, 10, 1, 0, 2, 3, 1, 0, 1, 0, 0, 4, 1, 46, 1, 1, 60, 0, 1, 2, 3, 1, 0, 1, 1, 1, 14, 1, 14, 0, 1, 0, 4, 0, 1, 0, 14, 3, 0, 0, 15, 1, 6, 1, 3, 0, 30, 0, 31, 1, 46, 1, 1, 60, 0, 30, 1, 34, 0, 30, 4, 0, 30, 1, 0, 37, 1, 38, 0, 30, 0, 40, 2, 1, 1, 1, 1, 29, 0, 4294967295, 1, 3, 0, 50, 0, 50, 2, 1, 3, 1, 1, 44, 1, 0, 4294967295, 1, 0, 2, 1, 5, 1, 3, 4, 1, 0, 0, 50, 0, 50, 2, 1, 6, 0, 1, 49, 1, 0, 1, 1, 2]\n\
";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_step_and_stage_encodes_and_each_word_reads_back_as_written"),
        word: Word::Succeeds,
        input: || Scenario::Words,
        expect: Expect::Value(Cow::Borrowed(WORDS)),
    },
    Case {
        name: CaseName("every_site_unit_and_need_of_the_pack_is_numbered_once"),
        word: Word::Succeeds,
        input: || Scenario::Tables,
        expect: Expect::Value(Cow::Borrowed(TABLES)),
    },
];

/// The tables of a code, one line each.
fn tables(code: &ProgramCode) -> String {
    let units: Vec<String> = code
        .units
        .iter()
        .map(|unit| format!("{:?} {:?}+{:?}", unit.form, unit.first.0, unit.points.0))
        .collect();
    format!(
        "counts {:?}\nrules {:?}\ntables {:?}\nindexes {:?}\nunits {}\nsizes {:?}\nconstants {:?}\nrelations {:?}\nneeds {:?}\npool {:?}\n",
        code.counts,
        code.rules
            .iter()
            .map(|rule| &rule.branches)
            .collect::<Vec<_>>(),
        code.tables,
        code.indexes,
        units.join(", "),
        code.sizes,
        code.constants.iter().map(|word| word.0).collect::<Vec<_>>(),
        code.relations,
        code.needs.iter().map(|slot| slot.0).collect::<Vec<_>>(),
        code.pool.iter().map(|slot| slot.0).collect::<Vec<_>>(),
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let code = encoded(&every_variant())?;
    Ok(Cow::Owned(match scenario {
        Scenario::Words => listing(&code),
        Scenario::Tables => tables(&code),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
