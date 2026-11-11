//! A walk publishes nothing for an empty next file, declines invalid generic rows, and does no semantic work for unready classes.
use std::borrow::Cow;

use crate::domain::operating::errors::OperatingError;
use crate::domain::shared_walk::service_test::{
    add_pattern, bound, evaluate_plans, evaluate_rows, generic_plans, invalid_second_step,
    numbered, plans, relation_plan, tree_of, with_context, with_small_tree,
};
use crate::domain::shared_walk::types::native_batch::value::NativeBatch;
use crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk;
use crate::domain::shared_walk::types::{WalkRefusal, WalkWork};
use crate::domain::step_classes::types::ClassPlace;
use crate::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn, FileId, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::RowCount;

/// What a row walks.
#[derive(Clone, Copy, Debug)]
enum Walk {
    /// The four fixture rules, rules 0 and 2 also matching a parameter bound to the invalid regex `[`, every class ready.
    ReachedError,
    /// The four fixture rules over the small tree of file 0, then over the empty tree of file 1, every class ready.
    NextFile,
    /// Three `source_location` rules testing column 8 against a parameter bound to 1, 2 and the text `1`, and a `semantic_syntax`
    /// rule, over three stored rows whose column 8 is 1, 2, 1; then over an empty `semantic_syntax` table.
    GenericRows,
    /// A `source_location` rule over a computed table of no cell said to be of the relation and to have this many rows.
    Declined(Relation, usize),
    /// A rule whose second step matches the invalid regex `[`, the classes of its steps at these places ready.
    Unready(&'static [usize]),
}

/// How a class's selection ended.
#[derive(Clone, Debug, PartialEq)]
enum Completion {
    Complete,
    InvalidRegex,
    Other,
}

/// What a class answered: how it ended and the positions it selected.
type Class = (Completion, Cow<'static, [u32]>);

/// What a walk observed, as the old assertions checked it.
#[derive(Debug, PartialEq)]
enum Observed {
    /// The members of class 0, whether it failed and its visits, and the positions of class 1.
    Reached {
        members: usize,
        failed: bool,
        visits: u64,
        other: Option<Cow<'static, [u32]>>,
    },
    /// The positions class 0 kept of file 0 after file 1 was walked, and whether every class of file 1 is empty and complete.
    NextFile {
        first: Option<Cow<'static, [u32]>>,
        next_empty_and_complete: bool,
    },
    /// The classes, the passes and source rows of the walk, the positions of classes 0, 1 and 3 and the visits of 3; then the
    /// positions of classes 0 and 3 over the other schema.
    Generic {
        classes: usize,
        walk: (u64, u64),
        positions: [Option<Cow<'static, [u32]>>; 3],
        fourth_visits: u64,
        other_schema: [Option<Cow<'static, [u32]>>; 2],
    },
    /// Whether the declined walk's class answered, its refusal, and its passes.
    Declined(bool, Option<WalkRefusal>, u64),
    /// Whether the classes of the rule's two steps are distinct and of one rule, each class's answer and visits, the passes
    /// and source rows, and the refusal.
    Unready {
        distinct: bool,
        same_rule: bool,
        classes: [Option<Class>; 2],
        visits: [u64; 2],
        walk: (u64, u64),
        refusal: Option<WalkRefusal>,
    },
}

type Input = Walk;
type Value = Observed;
type Failure = std::convert::Infallible;

/// The positions a class selected; none when it did not answer.
fn positions(batch: &NativeBatch, place: ClassPlace) -> Option<Cow<'static, [u32]>> {
    batch
        .class(place)
        .map(|result| Cow::Owned(result.positions.0.clone()))
}

/// A walk that answered no class and made no pass, with this refusal.
const fn unwalked(refusal: Option<WalkRefusal>) -> Observed {
    Observed::Declined(false, refusal, 0)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reached semantic error retained"),
        word: Word::Succeeds,
        input: || Walk::ReachedError,
        expect: Expect::Value(Observed::Reached {
            members: 2,
            failed: true,
            visits: 1,
            other: Some(Cow::Borrowed(&[2])),
        }),
    },
    Case {
        name: CaseName("next file drops previous native publication"),
        word: Word::Succeeds,
        input: || Walk::NextFile,
        expect: Expect::Value(Observed::NextFile {
            first: Some(Cow::Borrowed(&[1, 3])),
            next_empty_and_complete: true,
        }),
    },
    Case {
        name: CaseName("generic rows share one walk under exact typed bindings and relation"),
        word: Word::Succeeds,
        input: || Walk::GenericRows,
        expect: Expect::Value(Observed::Generic {
            classes: 4,
            walk: (1, 3),
            positions: [
                Some(Cow::Borrowed(&[0, 2])),
                Some(Cow::Borrowed(&[1])),
                None,
            ],
            fourth_visits: 0,
            other_schema: [None, Some(Cow::Borrowed(&[]))],
        }),
    },
    Case {
        name: CaseName(
            "an invalid generic representation or relation schema declines without semantic work",
        ),
        word: Word::Succeeds,
        input: || Walk::Declined(Relation::SourceLocation, 1),
        expect: Expect::Value(unwalked(Some(WalkRefusal::Representation))),
    },
    Case {
        name: CaseName("an oversized generic representation declines without semantic work"),
        word: Word::Succeeds,
        input: || Walk::Declined(Relation::SourceLocation, usize::MAX),
        expect: Expect::Value(unwalked(Some(WalkRefusal::Representation))),
    },
    Case {
        name: CaseName("a foreign ready class is not a representation refusal"),
        word: Word::Succeeds,
        input: || Walk::Declined(Relation::SemanticSyntax, 0),
        expect: Expect::Value(unwalked(None)),
    },
    Case {
        name: CaseName("an unready class of a demanded rule performs no semantic work"),
        word: Word::Succeeds,
        input: || Walk::Unready(&[0]),
        expect: Expect::Value(Observed::Unready {
            distinct: true,
            same_rule: true,
            classes: [Some((Completion::Complete, Cow::Borrowed(&[1, 3]))), None],
            visits: [2, 0],
            walk: (1, 4),
            refusal: None,
        }),
    },
    Case {
        name: CaseName("no demand performs no semantic work and invents no refusal"),
        word: Word::Succeeds,
        input: || Walk::Unready(&[]),
        expect: Expect::Value(Observed::Unready {
            distinct: true,
            same_rule: true,
            classes: [None, None],
            visits: [0, 0],
            walk: (0, 0),
            refusal: None,
        }),
    },
    Case {
        name: CaseName("the skipped predicate errors when its own continuation becomes ready"),
        word: Word::Succeeds,
        input: || Walk::Unready(&[1]),
        expect: Expect::Value(Observed::Unready {
            distinct: true,
            same_rule: true,
            classes: [None, Some((Completion::InvalidRegex, Cow::Borrowed(&[])))],
            visits: [0, 1],
            walk: (1, 4),
            refusal: None,
        }),
    },
];

fn subject(walk: Input) -> Result<Value, Failure> {
    Ok(match walk {
        Walk::ReachedError => a_reached_semantic_error_is_kept_for_its_class(),
        Walk::NextFile => an_empty_next_file_has_no_stale_native_publication(),
        Walk::GenericRows => generic_rows_share_one_walk_under_exact_typed_bindings_and_relation(),
        Walk::Declined(relation, rows) => declined(relation, rows),
        Walk::Unready(ready) => unready(ready),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn an_empty_next_file_has_no_stale_native_publication() -> Observed {
    let plans = plans();
    let bindings = vec![Vec::new(); plans.len()];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let (bound, ready) = (
        bound(&bindings),
        [ClassPlace(0), ClassPlace(1), ClassPlace(2)],
    );
    let mut work = WalkWork::default();
    with_small_tree(|elements, context| {
        let first = evaluate_rows(
            &prepared,
            Relation::SyntaxTree,
            &tree_of(elements, FileId(0)),
            &bound,
            context,
            &ready,
            &mut work,
        );
        let empty = ryo_language::domain::elements::types::Elements::default();
        let next = evaluate_rows(
            &prepared,
            Relation::SyntaxTree,
            &tree_of(&empty, FileId(1)),
            &bound,
            context,
            &ready,
            &mut work,
        );
        Observed::NextFile {
            first: positions(&first, ClassPlace(0)),
            next_empty_and_complete: (0..prepared.classes().len()).all(|at| {
                next.class(ClassPlace(at)).is_some_and(|result| {
                    result.positions.0.is_empty() && result.completion == Ok(())
                })
            }),
        }
    })
}
fn generic_rows_share_one_walk_under_exact_typed_bindings_and_relation() -> Observed {
    let (plans, bindings) = generic_plans();
    let prepared = PreparedWalk::compile(&plans, &bindings);
    // The fourth class belongs to another source and receives no visits.
    let columns = (0..9)
        .map(|at| FactColumn(if at == 8 { vec![1, 2, 1] } else { vec![0; 3] }))
        .collect::<Vec<_>>();
    let rows = Rows::Stored(ColumnsView {
        kinds: ryo_relations::domain::relations::service::kinds(Relation::SourceLocation),
        columns: &columns,
    });
    let bound = bound(&bindings);
    let batch = with_context(&mut Symbols::default(), |context| {
        evaluate_rows(
            &prepared,
            Relation::SourceLocation,
            &rows,
            &bound,
            context,
            &[ClassPlace(0), ClassPlace(1), ClassPlace(2)],
            &mut WalkWork::default(),
        )
    });
    // Reuse the exact program for a second schema; no SourceLocation class runs.
    let other = ryo_values::domain::rows::types::Table {
        kinds: ryo_relations::domain::relations::service::kinds(Relation::SemanticSyntax).to_vec(),
        cells: Vec::new(),
        count: RowCount(0),
    };
    let empty = with_context(&mut Symbols::default(), |context| {
        evaluate_rows(
            &prepared,
            Relation::SemanticSyntax,
            &Rows::Computed(&other),
            &bound,
            context,
            &[ClassPlace(3)],
            &mut WalkWork::default(),
        )
    });
    Observed::Generic {
        classes: prepared.classes().len(),
        walk: (batch.counts().passes.0, batch.counts().source_rows.0),
        positions: [
            positions(&batch, ClassPlace(0)),
            positions(&batch, ClassPlace(1)),
            positions(&batch, ClassPlace(3)),
        ],
        fourth_visits: batch.visits(ClassPlace(3)).0,
        other_schema: [
            positions(&empty, ClassPlace(0)),
            positions(&empty, ClassPlace(3)),
        ],
    }
}
/// An invalid generic representation or relation schema declines without semantic work.
fn declined(relation: Relation, rows: usize) -> Observed {
    let plans = [relation_plan(
        Relation::SourceLocation,
        vec![numbered(8, 1)],
    )];
    let prepared = PreparedWalk::compile(&plans, &[vec![]]);
    let table = ryo_values::domain::rows::types::Table {
        kinds: ryo_relations::domain::relations::service::kinds(Relation::SourceLocation).to_vec(),
        cells: vec![],
        count: RowCount(rows),
    };
    let batch = with_context(&mut Symbols::default(), |context| {
        evaluate_rows(
            &prepared,
            relation,
            &Rows::Computed(&table),
            &[Vec::new().into_boxed_slice()],
            context,
            &[ClassPlace(0)],
            &mut WalkWork::default(),
        )
    });
    Observed::Declined(
        batch.class(ClassPlace(0)).is_some(),
        batch.refusal(),
        batch.counts().passes.0,
    )
}
/// An unready class of a demanded rule performs no semantic work.
fn unready(ready: &[usize]) -> Observed {
    let bindings = vec![Vec::new()];
    let prepared = PreparedWalk::compile(&[invalid_second_step()], &bindings);
    let classes: Vec<ClassPlace> = prepared
        .class_of(crate::domain::coverage::types::RuleIndex(0))
        .unwrap()
        .iter()
        .flatten()
        .copied()
        .collect();
    let places: Vec<ClassPlace> = ready.iter().map(|at| classes[*at]).collect();
    let batch = evaluate_plans(&prepared, &bindings, &places);
    let class = |place: ClassPlace| {
        batch.class(place).map(|result| {
            let completion = match result.completion.as_ref().err() {
                None => Completion::Complete,
                Some(OperatingError::InvalidRegex { .. }) => Completion::InvalidRegex,
                Some(_) => Completion::Other,
            };
            (completion, Cow::Owned(result.positions.0.clone()))
        })
    };
    let (first, second) = (classes[0], classes[1]);
    Observed::Unready {
        distinct: first != second,
        same_rule: prepared.classes()[first.0].first.rule
            == prepared.classes()[second.0].first.rule,
        classes: [class(first), class(second)],
        visits: [batch.visits(first).0, batch.visits(second).0],
        walk: (batch.counts().passes.0, batch.counts().source_rows.0),
        refusal: batch.refusal(),
    }
}

fn a_reached_semantic_error_is_kept_for_its_class() -> Observed {
    let mut plans = plans();
    add_pattern(&mut plans, 0);
    let mut bindings = vec![Vec::new(); plans.len()];
    bindings[0] = vec![ParameterBinding::Text(LiteralText("[".into()))];
    bindings[2] = vec![ParameterBinding::Text(LiteralText("[".into()))];
    let prepared = PreparedWalk::compile(&plans, &bindings);
    let batch = evaluate_plans(
        &prepared,
        &bindings,
        &[ClassPlace(0), ClassPlace(1), ClassPlace(2)],
    );
    Observed::Reached {
        members: prepared.classes()[0].members.len(),
        failed: batch
            .class(ClassPlace(0))
            .and_then(|result| result.completion.as_ref().err())
            .is_some(),
        visits: batch.visits(ClassPlace(0)).0,
        other: positions(&batch, ClassPlace(1)),
    }
}
