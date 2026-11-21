//! The cost class and the code of each fill, one row for each.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::records::types::EdgeColumns;
use crate::domain::code::types::FillCode;
use crate::domain::lowered::service::{fill_class, fill_code};
use crate::domain::lowered::types::{
    ConstantRows, FillInput, GraphFill, GroupFill, MadeFill, OrderedFill,
};
use crate::domain::steps::types::{ColumnList, CostClass, MadeColumn};
use ryo_values::domain::plan_orders::types::GroupEnd;
use ryo_values::domain::plan_parts::types::RepeatedRows;

/// The fill a row classes.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// `a definitions rows are the files`.
    Rows,
    /// `a group count is one groups rows`.
    Count,
    /// `a group digest is one groups rows`.
    Digest,
    /// `a groups ends are one groups rows`.
    Ends,
    /// `a groups places are one groups rows`.
    Places,
    /// `a sorted table is one groups rows`.
    Sorted,
    /// `an ordered match is one groups rows`.
    Match,
    /// `the fixed points are one groups rows`.
    FixedPoints,
    /// `an ordered walk is the files`.
    OrderedDfs,
    /// `a worklist is the files`.
    Worklist,
    /// `the cycles are the files`.
    Cycles,
    /// `written rows are made once for the run`.
    Constant,
}

/// What a fill is classed and coded as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Classed {
    /// Its cost class.
    class: CostClass,
    /// Its code.
    code: FillCode,
}

type Input = Scenario;
type Value = Classed;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_definitions_rows_are_the_files"),
        word: Word::Succeeds,
        input: || Scenario::Rows,
        expect: Expect::Value(Classed {
            class: CostClass::File,
            code: FillCode(0),
        }),
    },
    Case {
        name: CaseName("a_group_count_is_one_groups_rows"),
        word: Word::Succeeds,
        input: || Scenario::Count,
        expect: Expect::Value(Classed {
            class: CostClass::Group,
            code: FillCode(1),
        }),
    },
    Case {
        name: CaseName("a_group_digest_is_one_groups_rows"),
        word: Word::Succeeds,
        input: || Scenario::Digest,
        expect: Expect::Value(Classed {
            class: CostClass::Group,
            code: FillCode(2),
        }),
    },
    Case {
        name: CaseName("a_groups_ends_are_one_groups_rows"),
        word: Word::Succeeds,
        input: || Scenario::Ends,
        expect: Expect::Value(Classed {
            class: CostClass::Group,
            code: FillCode(3),
        }),
    },
    Case {
        name: CaseName("a_groups_places_are_one_groups_rows"),
        word: Word::Succeeds,
        input: || Scenario::Places,
        expect: Expect::Value(Classed {
            class: CostClass::Group,
            code: FillCode(4),
        }),
    },
    Case {
        name: CaseName("a_sorted_table_is_one_groups_rows"),
        word: Word::Succeeds,
        input: || Scenario::Sorted,
        expect: Expect::Value(Classed {
            class: CostClass::Group,
            code: FillCode(5),
        }),
    },
    Case {
        name: CaseName("an_ordered_match_is_one_groups_rows"),
        word: Word::Succeeds,
        input: || Scenario::Match,
        expect: Expect::Value(Classed {
            class: CostClass::Group,
            code: FillCode(6),
        }),
    },
    Case {
        name: CaseName("the_fixed_points_are_one_groups_rows"),
        word: Word::Succeeds,
        input: || Scenario::FixedPoints,
        expect: Expect::Value(Classed {
            class: CostClass::Group,
            code: FillCode(7),
        }),
    },
    Case {
        name: CaseName("an_ordered_walk_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::OrderedDfs,
        expect: Expect::Value(Classed {
            class: CostClass::File,
            code: FillCode(8),
        }),
    },
    Case {
        name: CaseName("a_worklist_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::Worklist,
        expect: Expect::Value(Classed {
            class: CostClass::File,
            code: FillCode(9),
        }),
    },
    Case {
        name: CaseName("the_cycles_are_the_files"),
        word: Word::Succeeds,
        input: || Scenario::Cycles,
        expect: Expect::Value(Classed {
            class: CostClass::File,
            code: FillCode(10),
        }),
    },
    Case {
        name: CaseName("written_rows_are_made_once_for_the_run"),
        word: Word::Succeeds,
        input: || Scenario::Constant,
        expect: Expect::Value(Classed {
            class: CostClass::Constant,
            code: FillCode(11),
        }),
    },
];

/// An input of no pipeline.
fn input() -> FillInput {
    FillInput {
        pipelines: Box::new([]),
        keys: ColumnList::default(),
        order: None,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let graph = |fill| MadeFill::Graph(fill, Box::new([input()]));
    let fill = match scenario {
        Scenario::Rows => MadeFill::Rows(Box::new([]), RepeatedRows::Dropped),
        Scenario::Count => MadeFill::Group(GroupFill::Count { distinct: None }, input()),
        Scenario::Digest => MadeFill::Group(
            GroupFill::Digest {
                over: MadeColumn(0),
            },
            input(),
        ),
        Scenario::Ends => MadeFill::Group(GroupFill::Ends(GroupEnd::Least), input()),
        Scenario::Places => MadeFill::Group(GroupFill::Places, input()),
        Scenario::Sorted => MadeFill::Ordered(OrderedFill::Sorted(input())),
        Scenario::Match => MadeFill::Ordered(OrderedFill::Match(input(), input())),
        Scenario::FixedPoints => graph(GraphFill::FixedPoints(EdgeColumns {
            source: MadeColumn(0),
            destination: MadeColumn(1),
        })),
        Scenario::OrderedDfs => graph(GraphFill::OrderedDfs),
        Scenario::Worklist => graph(GraphFill::Worklist),
        Scenario::Cycles => graph(GraphFill::Cycles),
        Scenario::Constant => MadeFill::Constant(ConstantRows::default()),
    };
    Ok(Classed {
        class: fill_class(&fill),
        code: fill_code(&fill),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
