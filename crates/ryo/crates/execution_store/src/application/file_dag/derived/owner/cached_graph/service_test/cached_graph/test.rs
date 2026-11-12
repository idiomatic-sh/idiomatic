//! A graph demand reaches the cache only through the owner of a prepared file; before
//! preparation and after retirement it is refused, never traversed by another route.
use super::fixtures::{ENDPOINTS, SOURCE, admitted_graph_of, graph_of};
use crate::application::file_dag::derived::owner::{
    fixtures::graph::GraphFixture, index::service::LIMIT,
};
use crate::application::file_dag::relations::service::RelationOwner;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};

/// The owner a graph demand reaches, and what it is asked.
#[derive(Clone, Copy, Debug)]
enum Demand {
    /// A file owner never prepared is asked for its graph cache.
    Unprepared,
    /// A prepared file owner is asked whether its cache is ready.
    Prepared,
    /// A prepared file owner, retired, is asked whether its cache is ready.
    Retired,
    /// A prepared file owner is asked for the graph over the edges 1 -> 2 -> 3.
    Graph,
    /// A project owner is asked for an axis's graph before it serves the project.
    ProjectBeforeServing,
    /// A project owner serving the project is asked for an axis's graph.
    Project,
    /// A project owner serving the project, its population retired, is asked again.
    ProjectRetired,
    /// A file owner is asked to serve the project.
    FileServingProject,
    /// A prepared file owner is asked, through the admitted views port, for an axis's graph over two functions admitted at
    /// its three places.
    AdmittedAxis,
}

/// What the demand answered.
#[derive(Debug, PartialEq)]
enum Answer {
    /// The cache is ready.
    Ready,
    /// A graph with this many vertices, after this many builds.
    Graph { vertices: usize, builds: u64 },
    /// The owner prepared a graph for the axis.
    Prepared,
    /// The owner prepared no graph for the axis: the axis walks its tables.
    Declined,
}

type Input = Demand;
type Value = Answer;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_unprepared_owner_refuses_graph_demand_with_a_typed_error"),
        word: Word::Refuses,
        input: || Demand::Unprepared,
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        name: CaseName("a_prepared_owner_has_its_cache_ready"),
        word: Word::Succeeds,
        input: || Demand::Prepared,
        expect: Expect::Value(Answer::Ready),
    },
    Case {
        name: CaseName("a_retired_owner_refuses_graph_demand_instead_of_traversing"),
        word: Word::Refuses,
        input: || Demand::Retired,
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        name: CaseName("a_prepared_owner_answers_graph_demand_through_its_cache"),
        word: Word::Succeeds,
        input: || Demand::Graph,
        expect: Expect::Value(Answer::Graph {
            vertices: 3,
            builds: 1,
        }),
    },
    Case {
        name: CaseName("a_prepared_owner_answers_an_axis_through_the_admitted_views_port"),
        word: Word::Succeeds,
        input: || Demand::AdmittedAxis,
        expect: Expect::Value(Answer::Prepared),
    },
    Case {
        name: CaseName("a_project_owner_refuses_the_graph_before_it_serves_the_project"),
        word: Word::Refuses,
        input: || Demand::ProjectBeforeServing,
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        name: CaseName("a_project_owner_declines_the_graph_so_its_axis_walks_its_tables"),
        word: Word::Succeeds,
        input: || Demand::Project,
        expect: Expect::Value(Answer::Declined),
    },
    Case {
        name: CaseName("a_project_owner_still_declines_the_graph_after_its_population_retires"),
        word: Word::Succeeds,
        input: || Demand::ProjectRetired,
        expect: Expect::Value(Answer::Declined),
    },
    Case {
        name: CaseName("a_file_owner_cannot_serve_the_project"),
        word: Word::Refuses,
        input: || Demand::FileServingProject,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];

fn project_owner() -> std::rc::Rc<RelationOwner> {
    std::rc::Rc::new(RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        crate::application::file_dag::fixtures::program_derived_port(),
    ))
}

fn axis_graph(owner: &std::rc::Rc<RelationOwner>) -> Result<Answer, OperatingError> {
    graph_of(owner).map(|prepared| {
        if prepared {
            Answer::Prepared
        } else {
            Answer::Declined
        }
    })
}

/// Builds the owner in the demanded state and asks it.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Demand::Unprepared => project_owner()
            .derived()
            .graph_cache()
            .map(|_| Answer::Ready),
        Demand::Prepared => GraphFixture::new(&[Relation::Function], &[], LIMIT)
            .cache_ready()
            .map(|()| Answer::Ready),
        Demand::Retired => {
            let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
            fixture.retire();
            fixture.cache_ready().map(|()| Answer::Ready)
        }
        Demand::Graph => {
            let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
            let rows = Rows::Stored(ColumnsView {
                kinds: &[CellKind::Number; 2],
                columns: &columns,
            });
            let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
            let graph = fixture
                .acquire(SOURCE, &rows, ENDPOINTS, &[])?
                .expect("the owner route answers");
            Ok(Answer::Graph {
                vertices: graph.graph().vertices().len(),
                builds: fixture.snapshot().counts.builds,
            })
        }
        Demand::AdmittedAxis => {
            // Two functions, ids 1 and 2, in the columns the axis contract reads: id, name, file and span.
            let columns = [
                FactColumn(vec![1, 2]),
                FactColumn(vec![0, 0]),
                FactColumn(vec![0, 0]),
                FactColumn(vec![0, 0]),
            ];
            let rows = Rows::Stored(ColumnsView {
                kinds: ryo_relations::domain::relations::service::kinds(Relation::Function),
                columns: &columns,
            });
            let fixture = GraphFixture::new(&[Relation::Function], &[], LIMIT);
            fixture.cache_ready()?;
            admitted_graph_of(&fixture.owner, &rows).map(|prepared| {
                if prepared {
                    Answer::Prepared
                } else {
                    Answer::Declined
                }
            })
        }
        Demand::ProjectBeforeServing => axis_graph(&project_owner()),
        Demand::Project => {
            let owner = project_owner();
            owner.derived().serve_project()?;
            axis_graph(&owner)
        }
        Demand::ProjectRetired => {
            let owner = project_owner();
            owner.derived().serve_project()?;
            // A retirement of its population leaves it the project's.
            owner.release_population();
            axis_graph(&owner)
        }
        Demand::FileServingProject => GraphFixture::new(&[Relation::Function], &[], LIMIT)
            .owner
            .derived()
            .serve_project()
            .map(|()| Answer::Ready),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
