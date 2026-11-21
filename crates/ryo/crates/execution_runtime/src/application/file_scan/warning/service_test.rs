//! The `warning` part's cases: one written form for each kind of work the engine names, each with the engine's counts; a
//! relation past its declared size; and the statement named with its line, without it when the query holds no such
//! document, and as the plan step when the query language says no statement.

use super::service::worded;
use super::types::{NoticedStatement, SlowRule, WrittenStatement};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_engine::domain::findings::types::{
    HeaviestWork, InstructionCount, WorkNotice, WorkVerdict,
};
use ryo_language::domain::files::types::SourcePath;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::query_tokens::types::{
    ByteOffset, QueryBundle, QueryDocumentSource, QuerySourceId, QuerySpan, StepSpelling,
};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::runs::types::RuleId;
use std::borrow::Cow;
use std::sync::Arc;

/// The text of the rule's own document: its statement `pairs` is on its third line.
const DOCUMENT: &str =
    "param limit: integer\nlet words = token\nlet pairs = join words with words\nemit pairs\n";

/// How the statement of a row's warning is said.
#[derive(Clone, Copy, Debug)]
enum Said {
    /// By the query language, in the rule's own document.
    Written,
    /// By the query language, in a document the rule's query does not hold.
    Elsewhere,
    /// Not at all: the plan step is named.
    PlanStep,
}

/// What a row words: what the engine's notice says did the most work, its verdict, and how its statement is said.
type Input = (HeaviestWork, WorkVerdict, Said);
/// The warning's lines.
type Value = Cow<'static, str>;
type Failure = ();

const fn rows(count: u64) -> InstructionCount {
    InstructionCount(count)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_join_with_no_key_is_said_with_its_two_sides_and_their_pairs"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::Product {
                left: rows(1000),
                right: rows(1000),
            };
            (heaviest, WorkVerdict::Over, Said::Written)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` is slow in `b.walk`\n  statement `pairs` (rule.ryo:3) joins `words` (1000 rows) with `words` (1000 rows) with no key between them\n  that is 1000000 pairs in this one file; the run continues and its findings are complete\n  to fix: join on a shared column or add a range, so each row meets only its matches",
        )),
    },
    Case {
        name: CaseName("a_rules_own_program_is_said_with_its_candidates_and_the_rows_it_visits"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::Program {
                candidates: rows(40),
                visited: rows(90_000),
            };
            (heaviest, WorkVerdict::Over, Said::Written)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` is slow in `b.walk`\n  statement `pairs` (rule.ryo:3) visits 90000 rows from 40 candidates in the rule's own program\n  that is 90000 rows visited in this one file; the run continues and its findings are complete\n  to fix: start from the rarest thing the rule needs, or reach the rest by a key, so each candidate visits few rows",
        )),
    },
    Case {
        name: CaseName("a_definition_asked_by_key_is_said_with_its_keys_and_the_rows_it_visits"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::Calls {
                keys: rows(300),
                visited: rows(70_000),
            };
            (heaviest, WorkVerdict::Over, Said::Written)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` is slow in `b.walk`\n  statement `pairs` (rule.ryo:3) is asked for 300 keys and visits 70000 rows to answer them\n  that is 70000 rows visited in this one file; the run continues and its findings are complete\n  to fix: ask it by a column that tells its rows apart, so each key reaches few rows",
        )),
    },
    Case {
        name: CaseName("a_table_of_rows_read_by_key_is_said_with_the_rows_it_reads_and_makes"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::Rows {
                read: rows(2000),
                made: rows(50_000),
            };
            (heaviest, WorkVerdict::Over, Said::Written)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` is slow in `b.walk`\n  statement `pairs` (rule.ryo:3) reads 2000 rows by its keys and makes 50000 rows\n  that is 52000 rows read and made in this one file; the run continues and its findings are complete\n  to fix: join on a column that tells the rows apart, so each key matches few rows",
        )),
    },
    Case {
        name: CaseName("a_group_is_said_with_the_rows_it_groups_and_makes"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::Group {
                read: rows(60_000),
                made: rows(12),
            };
            (heaviest, WorkVerdict::Over, Said::Written)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` is slow in `b.walk`\n  statement `pairs` (rule.ryo:3) groups 60000 rows into 12 rows\n  that is 60012 rows read and made in this one file; the run continues and its findings are complete\n  to fix: keep fewer rows before the group, or group by fewer columns",
        )),
    },
    Case {
        name: CaseName("a_sort_is_said_with_the_rows_it_sorts"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::Sorted { rows: rows(64_000) };
            (heaviest, WorkVerdict::Over, Said::Written)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` is slow in `b.walk`\n  statement `pairs` (rule.ryo:3) sorts 64000 rows\n  that is 64000 rows sorted in this one file; the run continues and its findings are complete\n  to fix: keep fewer rows before the sort",
        )),
    },
    Case {
        name: CaseName("an_ordered_match_is_said_with_its_two_sides_and_the_pairs_it_makes"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::OrderedPairs {
                left: rows(800),
                right: rows(900),
                made: rows(360_000),
            };
            (heaviest, WorkVerdict::Over, Said::Written)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` is slow in `b.walk`\n  statement `pairs` (rule.ryo:3) matches `words` (800 rows) with `words` (900 rows) in order and makes 360000 pairs\n  that is 360000 pairs in this one file; the run continues and its findings are complete\n  to fix: use `as-of` for the nearest row, or bound the order on both sides, so each row meets only its neighbours",
        )),
    },
    Case {
        name: CaseName("a_graph_is_said_with_the_rows_it_walks_and_makes"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::Graph {
                read: rows(5000),
                made: rows(45_000),
            };
            (heaviest, WorkVerdict::Over, Said::Written)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` is slow in `b.walk`\n  statement `pairs` (rule.ryo:3) walks a graph of 5000 rows and makes 45000 rows\n  that is 50000 rows read and made in this one file; the run continues and its findings are complete\n  to fix: walk from fewer starts or over fewer edges, so the walk reaches only what the rule reads",
        )),
    },
    Case {
        name: CaseName("a_relation_past_its_declared_size_is_said_by_its_name"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::Sorted { rows: rows(64_000) };
            let verdict = WorkVerdict::Oversized(Relation::SyntaxError);
            (heaviest, verdict, Said::Written)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` reads a relation past its declared size in `b.walk`\n  relation `syntax-error` holds more rows there than the size it declares allows\n  the rule's work is 1000000 rows against an allowance of 128000 in this one file; the run continues and its findings are complete\n  to fix: this is the language's to fix: the relation is filled with fewer rows, or declares the size it has",
        )),
    },
    Case {
        name: CaseName("a_statement_of_a_document_the_query_does_not_hold_is_said_without_a_line"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::Sorted { rows: rows(64_000) };
            (heaviest, WorkVerdict::Over, Said::Elsewhere)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` is slow in `b.walk`\n  statement `pairs` (pack/other) sorts 64000 rows\n  that is 64000 rows sorted in this one file; the run continues and its findings are complete\n  to fix: keep fewer rows before the sort",
        )),
    },
    Case {
        name: CaseName("with_no_statement_said_the_plan_step_and_the_sides_of_a_join_are_named"),
        word: Word::Succeeds,
        input: || {
            let heaviest = HeaviestWork::Product {
                left: rows(1000),
                right: rows(1000),
            };
            (heaviest, WorkVerdict::Over, Said::PlanStep)
        },
        expect: Expect::Value(Cow::Borrowed(
            "warning: rule `rule` is slow in `b.walk`\n  step 2 of the rule's plan joins its left input (1000 rows) with its right input (1000 rows) with no key between them\n  that is 1000000 pairs in this one file; the run continues and its findings are complete\n  to fix: join on a shared column or add a range, so each row meets only its matches",
        )),
    },
];

fn subject((heaviest, verdict, said): Input) -> Result<Value, Failure> {
    let notice = WorkNotice {
        rule: SelectedRuleOrdinal(0),
        verdict,
        work: rows(1_000_000),
        allowance: rows(128_000),
        step: StepId(2),
        heaviest,
    };
    let query = QueryBundle {
        root: QueryDocumentSource {
            id: QuerySourceId("rule.ryo".into()),
            text: DOCUMENT.into(),
        },
        modules: Arc::default(),
    };
    let start = DOCUMENT.find("let pairs").unwrap_or_default();
    let written = WrittenStatement {
        document: QuerySourceId(match said {
            Said::Written | Said::PlanStep => "rule.ryo".into(),
            Said::Elsewhere => "pack/other".into(),
        }),
        statement: StepSpelling("pairs".into()),
        span: QuerySpan {
            start: ByteOffset(start),
            end: ByteOffset(start.saturating_add(32)),
        },
        inputs: vec![StepSpelling("words".into()), StepSpelling("words".into())],
    };
    let rule = RuleId("rule".into());
    let slow = SlowRule {
        rule: &rule,
        file: SourcePath("b.walk"),
        statement: match said {
            Said::Written | Said::Elsewhere => NoticedStatement::Written(&written, &query),
            Said::PlanStep => NoticedStatement::PlanStep,
        },
    };
    Ok(Cow::Owned(worded(&notice, &slow).0.into()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
