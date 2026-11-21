//! The cases of the planning over definitions: where a branch that joins definitions starts, and when a decision of a
//! definition runs. A use is costed from the definition's own plan for the columns it binds, so a join is entered from the
//! side that reads the other by its key. Every expected order is what a run answered.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowered::types::Emptiness;
use crate::domain::lowering::costing::service::branches_rows;
use crate::domain::lowering::definitions::service::defined;
use crate::domain::lowering::definitions::types::{Drafts, Flattened};
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowsPerFile, TextNumber,
};
use crate::domain::lowering::nodes::types::{AcceptedPlace, NodeHome, StepShape};
use crate::domain::lowering::planning::service::planned;
use crate::domain::lowering::planning::types::UnitRole;
use crate::domain::steps::types::{DefinitionId, Polarity, TestOperation};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::query_plans::types::NodeDigest;
use std::collections::HashSet;

const fn variable(number: u32) -> FlatTerm {
    FlatTerm::Variable(FlatVariable(number))
}

fn rows(relation: Relation, cells: Vec<FlatTerm>) -> FlatAtom {
    FlatAtom::Rows {
        rows: AtomRows::Relation(relation),
        cells,
        per_file: RowsPerFile::Many,
    }
}

/// A row of the semantic tree: its element, its kind, and three cells nothing else names (from `fresh` on).
fn element(id: u32, kind: u32, fresh: u32) -> FlatAtom {
    let cells = vec![
        variable(id),
        variable(kind),
        variable(fresh),
        FlatTerm::File,
        variable(fresh + 1),
    ];
    rows(Relation::SemanticSyntax, cells)
}

/// A child of a parent in a field: its ordinal is a cell nothing else names.
fn child(parent: u32, id: u32, field: u32, fresh: u32) -> FlatAtom {
    let cells = vec![
        variable(parent),
        variable(id),
        variable(fresh),
        variable(field),
        FlatTerm::File,
    ];
    rows(Relation::SemanticChild, cells)
}

/// The name an element is spelled with.
fn spelled(id: u32, name: u32, fresh: u32) -> FlatAtom {
    let cells = vec![
        variable(id),
        variable(name),
        FlatTerm::File,
        variable(fresh),
    ];
    rows(Relation::Identifier, cells)
}

fn held(left: u32, text: u32) -> FlatAtom {
    FlatAtom::Test {
        test: TestOperation::Equal,
        left: variable(left),
        right: FlatTerm::Text(TextNumber(text)),
    }
}

/// A use of a definition: one cell a column.
fn used(definition: u32, cells: &[u32]) -> FlatAtom {
    FlatAtom::Rows {
        rows: AtomRows::Definition(DefinitionId(definition)),
        cells: cells.iter().copied().map(variable).collect(),
        per_file: RowsPerFile::Many,
    }
}

/// A definition of `branches` over `columns` element columns, sized as the lowering sizes one.
fn define(drafts: &mut Drafts, columns: usize, branches: Vec<FlatBranch>) -> DefinitionId {
    let number = u8::try_from(drafts.definitions.len()).unwrap_or(u8::MAX);
    let rows = branches_rows(drafts, &branches);
    let flattened = Flattened {
        branches,
        emptiness: Emptiness::Unknown,
        keyless: Vec::new(),
    };
    let home = NodeHome {
        rule: AcceptedPlace(0),
        step: StepId(u32::from(number)),
    };
    let shape = StepShape(vec![CellKind::Node; columns]);
    defined(
        drafts,
        NodeDigest([number; 16]),
        home,
        shape,
        flattened,
        rows,
    )
}

/// The elements of a kind that are a child in a field of their parent, each with its name: (element, name).
fn named(drafts: &mut Drafts) -> DefinitionId {
    let atoms = vec![
        child(10, 11, 12, 13),
        held(12, 1),
        spelled(11, 14, 15),
        element(11, 16, 17),
        held(16, 2),
    ];
    let row = vec![variable(11), variable(14)];
    define(drafts, 2, vec![FlatBranch { atoms, row }])
}

/// The operands of the comparisons, of either of two fields: (comparison, operand).
fn operands(drafts: &mut Drafts) -> DefinitionId {
    let arm = |field: u32, base: u32| FlatBranch {
        atoms: vec![
            element(base, base + 1, base + 2),
            held(base + 1, 3),
            child(base, base + 4, base + 5, base + 6),
            held(base + 5, field),
        ],
        row: vec![variable(base), variable(base + 4)],
    };
    define(drafts, 2, vec![arm(4, 20), arm(5, 30)])
}

/// The pairs of operands of one comparison: (comparison, left, right).
fn pairs(drafts: &mut Drafts, operands: DefinitionId) -> DefinitionId {
    let atoms = vec![used(operands.0, &[40, 41]), used(operands.0, &[40, 42])];
    let row = vec![variable(40), variable(41), variable(42)];
    define(drafts, 3, vec![FlatBranch { atoms, row }])
}

/// What a row plans.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// The pairs of operands that are spelled with one name: pairs(c, l, r), named(l, n), named(r, n), as a table's pipeline.
    SameName,
    /// The pairs with a decision of the named elements by a name that is given: pairs(c, l, r), then "no element is
    /// named n" beside "l is named", as a pattern asked by its comparison and a name.
    DearDecision,
}

type Input = Scene;
/// The atoms' places in the order they run.
type Value = &'static [usize];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_use_by_a_column_that_is_no_key_waits_for_the_use_read_by_its_key"),
        word: Word::Succeeds,
        input: || Scene::SameName,
        expect: Expect::Value(&[1, 0, 2]),
    },
    Case {
        name: CaseName("a_dear_decision_waits_for_a_use_by_key_that_a_cheap_one_goes_before"),
        word: Word::Succeeds,
        input: || Scene::DearDecision,
        expect: Expect::Value(&[0, 2, 3, 1]),
    },
];

fn subject(scene: Input) -> Result<Vec<usize>, Failure> {
    let mut drafts = Drafts::default();
    let named = named(&mut drafts);
    let operands = operands(&mut drafts);
    let pairs = pairs(&mut drafts, operands);
    let (atoms, role, given): (Vec<FlatAtom>, UnitRole, &[u32]) = match scene {
        Scene::SameName => (
            vec![
                used(pairs.0, &[50, 51, 52]),
                used(named.0, &[51, 53]),
                used(named.0, &[52, 53]),
            ],
            UnitRole::Pipeline,
            &[],
        ),
        Scene::DearDecision => (
            vec![
                used(pairs.0, &[50, 51, 52]),
                // Dear: the named elements asked by a name alone, which is no key of them.
                FlatAtom::Decided {
                    polarity: Polarity::Absent,
                    rows: AtomRows::Definition(named),
                    cells: vec![None, Some(variable(54))],
                },
                // Cheap: the operands asked by their comparison and the operand, which is their key.
                FlatAtom::Decided {
                    polarity: Polarity::Present,
                    rows: AtomRows::Definition(operands),
                    cells: vec![Some(variable(50)), Some(variable(51))],
                },
                used(named.0, &[51, 55]),
            ],
            UnitRole::Pattern,
            &[50, 54],
        ),
    };
    let branch = FlatBranch {
        atoms,
        row: vec![variable(51)],
    };
    let given: HashSet<FlatVariable> = given.iter().copied().map(FlatVariable).collect();
    planned(&drafts, &branch, &given, role)
        .map(|plan| plan.order.into_iter().map(|at| at.0).collect())
        .ok_or(())
}

fn ordered(input: Input) -> Result<Value, Failure> {
    subject(input).map(|order| &*Vec::leak(order))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, ordered);
}
