//! The `fillers` service's cases: the fact source answers a relation its language fills with one column per column and its rows,
//! a relation the language has not read as refused for that reason, a node-table relation as refused (no language fills it), and
//! a filler that read the file in part as partial.

use super::service::file_source;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::facts::service::{base_words, file_words};
use crate::domain::facts::types::{FactAnswer, FactSource as _, FilledColumns};
use crate::domain::fillers::fixtures::{Walking, WalkingFile};
use crate::domain::node_table::service::{builder, diagnose, enter_node, finished, leave_node};
use crate::domain::node_table::types::{FieldCode, NodeTable};
use ryo_relations::domain::relations::types::{Count, FileId, KindBits, Offset, Relation};
use ryo_relations::domain::terms::types::NameText;

/// What a row fills: a relation, over a file whose text is given.
#[derive(Clone, Copy, Debug)]
struct Scenario {
    /// The relation asked.
    relation: Relation,
    /// The file's text.
    text: &'static str,
}

type Input = Scenario;
/// The answer's kind (0 answered, 1 partial, 2 refused), the columns written and the rows of the first.
type Value = [usize; 3];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filled_relation_answers_its_rows_column_by_column"),
        word: Word::Succeeds,
        input: || Scenario {
            relation: Relation::SyntaxError,
            text: "one!",
        },
        expect: Expect::Value([0, 7, 1]),
    },
    Case {
        name: CaseName("a_relation_the_language_has_not_read_is_refused"),
        word: Word::Succeeds,
        input: || Scenario {
            relation: Relation::Function,
            text: "one",
        },
        expect: Expect::Value([2, 0, 0]),
    },
    Case {
        name: CaseName("a_node_table_relation_is_refused_no_language_fills_it"),
        word: Word::Succeeds,
        input: || Scenario {
            relation: Relation::SyntaxTree,
            text: "one",
        },
        expect: Expect::Value([2, 0, 0]),
    },
    Case {
        name: CaseName("a_filler_that_read_the_file_in_part_answers_partial"),
        word: Word::Succeeds,
        input: || Scenario {
            relation: Relation::SyntaxError,
            text: "one?",
        },
        expect: Expect::Value([1, 7, 0]),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut table = NodeTable::default();
    let mut writer = builder(&mut table, Count(3));
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    if scenario.text.contains('!') {
        diagnose(
            &mut writer,
            Offset(3),
            Offset(4),
            NameText("an exclamation"),
        );
    }
    leave_node(&mut writer, Offset(4));
    let mut file = WalkingFile {
        table: finished(&mut writer),
        source: scenario.text.as_bytes(),
        file: FileId(1),
        fills: Count(0),
    };
    let (mut words, mut columns) = (file_words(&base_words(&[])), FilledColumns::default());
    let answer =
        file_source::<Walking>(&mut file).fill(scenario.relation, &mut words, &mut columns);
    let kind = match answer {
        FactAnswer::Answered => 0,
        FactAnswer::Partial(_) => 1,
        FactAnswer::Refused(_) => 2,
    };
    Ok([
        kind,
        columns.0.len(),
        columns.0.first().map_or(0, |column| column.0.len()),
    ])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
