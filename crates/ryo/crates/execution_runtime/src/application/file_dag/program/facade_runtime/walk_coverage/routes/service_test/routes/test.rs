//! Every operation variant routes as its meaning says; relabelled routes are refused.
use super::fixtures::{Covered, RouteRefusal, relabelled, verify_classifier, verify_program};
use crate::application::file_dag::program::facade_runtime::walk_coverage::fixtures::{
    meaning::Route,
    operations::{corpus::compiled_operations, heads::operations},
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::shared_walk::types::walk_route::value::WalkBarrier;
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;
use std::collections::HashSet;

/// What a row checks the routes of.
#[derive(Clone, Copy, Debug)]
enum Checked {
    /// The catalog of operations, one per kind.
    Catalog,
    /// The classifier's route of every catalog operation and selecting input.
    Classifier,
    /// The compiled corpus, with the head of a kind relabelled to a barrier or as compiled.
    Program(Option<(OperationKind, WalkBarrier)>),
}

/// What the check covered.
#[derive(Debug, PartialEq)]
enum Answer {
    /// The catalog's operations and the distinct kinds among them.
    Catalog { operations: usize, kinds: usize },
    /// The routing a verification covered.
    Routed(Covered),
}

type Input = Checked;
type Value = Answer;
type Failure = RouteRefusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_catalog_kind_has_a_head_in_the_corpus"),
        word: Word::Succeeds,
        input: || Checked::Catalog,
        expect: Expect::Value(Answer::Catalog {
            operations: OperationKind::ALL.len(),
            kinds: OperationKind::ALL.len(),
        }),
    },
    Case {
        name: CaseName("every variant's classifier route matches its meaning"),
        word: Word::Succeeds,
        input: || Checked::Classifier,
        expect: Expect::Value(Answer::Routed(Covered::Classifier {
            operations: OperationKind::ALL.len(),
            kinds: OperationKind::ALL.len(),
        })),
    },
    Case {
        name: CaseName("every instruction of the compiled corpus routes as its meaning says"),
        word: Word::Succeeds,
        input: || Checked::Program(None),
        expect: Expect::Value(Answer::Routed(Covered::Program {
            instructions: 33,
            kinds: OperationKind::ALL.len(),
            routes: Cow::Borrowed(&[
                Route::Shared,
                Route::Representation,
                Route::Dependent,
                Route::Row,
                Route::Derived,
                Route::Request,
                Route::Continuation,
            ]),
        })),
    },
    Case {
        name: CaseName("an extend head relabelled as a derived barrier is refused"),
        word: Word::Rejects,
        input: || Checked::Program(Some((OperationKind::Extend, WalkBarrier::DerivedOrOrdered))),
        expect: Expect::Error(RouteRefusal::RouteMismatch),
    },
    Case {
        name: CaseName("a rows source relabelled as row semantics is refused"),
        word: Word::Rejects,
        input: || Checked::Program(Some((OperationKind::Rows, WalkBarrier::RowSemantics))),
        expect: Expect::Error(RouteRefusal::RouteMismatch),
    },
    Case {
        name: CaseName("a join head relabelled as a continuation is refused"),
        word: Word::Rejects,
        input: || Checked::Program(Some((OperationKind::Join, WalkBarrier::Continuation))),
        expect: Expect::Error(RouteRefusal::RouteMismatch),
    },
];

/// Checks the routes of the row's subject.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        // `meaning` and `kind_of` are exhaustive without a wildcard, so a new variant must choose
        // a route to compile; the catalog and the corpus agree on the count.
        Checked::Catalog => {
            let catalog = operations();
            Ok(Answer::Catalog {
                operations: catalog.len(),
                kinds: catalog.iter().map(kind_of).collect::<HashSet<_>>().len(),
            })
        }
        Checked::Classifier => verify_classifier().map(Answer::Routed),
        Checked::Program(None) => verify_program(&compiled_operations()).map(Answer::Routed),
        Checked::Program(Some((kind, barrier))) => relabelled(kind, barrier).map(Answer::Routed),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn rejects() {
    run(CASES, Word::Rejects, subject);
}
