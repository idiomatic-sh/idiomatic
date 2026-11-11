//! Schema, publication and refusal controls through production request preparation.
use super::fixtures::{
    contracts, contracts_with, deep_walk_plan, extended_plan, native_plan, ordered_plan, owned,
    plan, prepared, selects_plan,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::executable_program::types::InputSource;
use ryo_execution_domain::domain::file_program::service::compile_prepared_observations;
use ryo_execution_domain::domain::file_program::types::NodeInput;
use ryo_execution_domain::domain::observation_schedule::types::ObservationRequest;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{CellKind, Integer, Relation, Value as CellValue};
use ryo_values::domain::plan_parts::types::{Input as PlanInput, StepId};
use ryo_values::domain::rows::types::{RowCount, Table};
use std::borrow::Cow;

/// The authored request a row prepares.
#[derive(Clone, Copy, Debug)]
enum Request {
    /// A select and a distinct projection, run per file.
    NativeSteps,
    /// The same, its projection handed to cross-file steps as the frontier.
    CrossFileFrontier,
    /// The native steps through the production compiler, counting each region's consumers.
    StepPublication,
    /// A select, an extension and a closure prepared from borrowed schemas.
    Extended,
    /// A join of two relations, prepared while recording the relations whose schema was asked.
    Ordered,
    /// A deep syntax walk as the frontier.
    DeepWalk,
    /// Three selects prepared from schemas borrowed from two tables' kinds.
    Borrowed,
    /// A relation frontier then a frontier naming a missing step.
    InvalidFrontier,
}

/// A list of schemas.
type Schemas = Cow<'static, [Cow<'static, [CellKind]>]>;

/// What preparing the request answered.
#[derive(Debug, PartialEq)]
enum Laid {
    /// Each prepared step's output kinds.
    StepKinds(Schemas),
    /// The schema of the step handed on, and whether the rule's frontier is a region.
    Frontier {
        schema: Option<Cow<'static, [CellKind]>>,
        region: bool,
    },
    /// Whether the two steps publish from distinct regions, and each region's consumers.
    Publications {
        distinct: bool,
        consumers: (usize, usize),
    },
    /// The preparations made and each compiled node's output kinds.
    Metadata {
        preparations: usize,
        schemas: Schemas,
    },
    /// The relations whose schema was asked, in order, and the first node's inputs.
    Ordered {
        probes: Cow<'static, [Relation]>,
        inputs: Cow<'static, [NodeInput]>,
    },
    /// The frontier node's output kinds.
    DeepWalk(Option<Cow<'static, [CellKind]>>),
    /// Whether both tables kept their cells and allocations and the prepared kinds their
    /// allocation, and the last node's output kinds in each of two compilations.
    Borrowed {
        cells_kept: bool,
        allocations_kept: bool,
        prepared_kept: bool,
        outputs: Schemas,
    },
    /// The first frontier's relation source, and the second frontier's refusal.
    InvalidFrontier {
        first: Option<Relation>,
        second: Option<OperatingError>,
    },
}

type Input = Request;
type Value = Laid;
type Failure = OperatingError;

const SYNTAX_TREE: &[CellKind] = &[
    CellKind::Node,
    CellKind::Symbol,
    CellKind::Symbol,
    CellKind::File,
    CellKind::Span,
];

const SYNTAX_CHILD: &[CellKind] = &[
    CellKind::Node,
    CellKind::Node,
    CellKind::Number,
    CellKind::Symbol,
    CellKind::File,
];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_native_step_answers_its_output_kinds"),
        word: Word::Succeeds,
        input: || Request::NativeSteps,
        expect: Expect::Value(Laid::StepKinds(Cow::Borrowed(&[
            Cow::Borrowed(SYNTAX_TREE),
            Cow::Borrowed(&[CellKind::Node]),
        ]))),
    },
    Case {
        name: CaseName("the_step_handed_to_cross_file_steps_is_its_frontier"),
        word: Word::Succeeds,
        input: || Request::CrossFileFrontier,
        expect: Expect::Value(Laid::Frontier {
            schema: Some(Cow::Borrowed(&[CellKind::Symbol, CellKind::Number])),
            region: true,
        }),
    },
    Case {
        name: CaseName("a_step_dependency_and_its_terminal_publication_are_counted_once_each"),
        word: Word::Succeeds,
        input: || Request::StepPublication,
        expect: Expect::Value(Laid::Publications {
            distinct: true,
            consumers: (1, 1),
        }),
    },
    Case {
        name: CaseName(
            "metadata_preparation_propagates_select_extend_and_graph_schemas_without_rows",
        ),
        word: Word::Succeeds,
        input: || Request::Extended,
        expect: Expect::Value(Laid::Metadata {
            preparations: 3,
            schemas: Cow::Borrowed(&[
                Cow::Borrowed(&[CellKind::Symbol, CellKind::File]),
                Cow::Borrowed(&[CellKind::Symbol, CellKind::File, CellKind::Symbol]),
                Cow::Borrowed(&[CellKind::Symbol, CellKind::Symbol]),
            ]),
        }),
    },
    Case {
        name: CaseName(
            "preparation_keeps_ordered_relation_dependencies_without_probing_availability",
        ),
        word: Word::Succeeds,
        input: || Request::Ordered,
        expect: Expect::Value(Laid::Ordered {
            probes: Cow::Borrowed(&[Relation::Function, Relation::Comment]),
            inputs: Cow::Borrowed(&[
                NodeInput::Relation(Relation::Function),
                NodeInput::Relation(Relation::Comment),
            ]),
        }),
    },
    Case {
        name: CaseName("empty_native_frontier_has_the_deep_walk_schema_before_any_file_is_read"),
        word: Word::Succeeds,
        input: || Request::DeepWalk,
        expect: Expect::Value(Laid::DeepWalk(Some(Cow::Borrowed(&[
            CellKind::Node,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::File,
            CellKind::Span,
            CellKind::Node,
        ])))),
    },
    Case {
        name: CaseName(
            "preparing_from_borrowed_schemas_preserves_cell_allocations_and_prepared_metadata",
        ),
        word: Word::Succeeds,
        input: || Request::Borrowed,
        expect: Expect::Value(Laid::Borrowed {
            cells_kept: true,
            allocations_kept: true,
            prepared_kept: true,
            outputs: Cow::Borrowed(&[Cow::Borrowed(SYNTAX_CHILD), Cow::Borrowed(SYNTAX_CHILD)]),
        }),
    },
    Case {
        name: CaseName("an_invalid_frontier_keeps_the_prior_source_and_its_own_typed_refusal"),
        word: Word::Succeeds,
        input: || Request::InvalidFrontier,
        expect: Expect::Value(Laid::InvalidFrontier {
            first: Some(Relation::Function),
            second: Some(OperatingError::NotRun),
        }),
    },
];

/// Preparing three selects from schemas borrowed from a computed and a native table's kinds: the
/// tables' cells, the prepared kinds and the last node's output kinds in two compilations.
fn borrowed() -> Result<Laid, OperatingError> {
    let mut cells = Vec::with_capacity(97);
    cells.push(CellValue::Number(Integer(7)));
    let computed = Table {
        kinds: vec![CellKind::Number],
        cells,
        count: RowCount(1),
    };
    let mut native_cells = Vec::with_capacity(79);
    native_cells.push(CellValue::Number(Integer(17)));
    // Payload bytes are deliberately unrelated to the borrowed schema: preparation cannot inspect them.
    let native = Table {
        kinds: kinds(Relation::SyntaxTree).to_vec(),
        cells: native_cells,
        count: RowCount(1),
    };
    let allocations = |computed: &Table, native: &Table| {
        (
            computed.cells.as_ptr(),
            computed.cells.capacity(),
            native.cells.as_ptr(),
            native.cells.capacity(),
        )
    };
    let before = allocations(&computed, &native);
    let plans = [selects_plan()];
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the test overrides two relations of the catalog and reads every other as the catalog declares it"
    )]
    let metadata = |relation| match relation {
        Relation::Function => computed.kinds.as_slice(),
        Relation::SyntaxTree => native.kinds.as_slice(),
        _ => kinds(relation),
    };
    let operators = contracts_with(&plans, &metadata)?;
    let prepared_kinds = operators.rules[0][1]
        .output_kinds
        .as_ref()
        .map(|kinds| kinds.as_ptr())
        .ok();
    let outputs = [
        compile_prepared_observations(&plans, &operators, ObservationRequest::Independent),
        compile_prepared_observations(&plans, &operators, ObservationRequest::Independent),
    ]
    .iter()
    .map(|graph| {
        graph.program_nodes[0][2]
            .as_ref()
            .map_or(Cow::Borrowed(&[][..]), |output| {
                owned(
                    graph.nodes[output.0]
                        .contract
                        .output_kinds
                        .clone()
                        .unwrap_or_default(),
                )
            })
    })
    .collect();
    Ok(Laid::Borrowed {
        cells_kept: computed.cells == [CellValue::Number(Integer(7))]
            && native.cells == [CellValue::Number(Integer(17))],
        allocations_kept: allocations(&computed, &native) == before,
        prepared_kept: operators.rules[0][1]
            .output_kinds
            .as_ref()
            .map(|kinds| kinds.as_ptr())
            .ok()
            == prepared_kinds,
        outputs,
    })
}

/// Prepares the row's request and reads its contracts, compiled nodes or program.
fn subject(request: Input) -> Result<Value, Failure> {
    match request {
        Request::NativeSteps => Ok(Laid::StepKinds(
            contracts(&[native_plan(false)])?.rules[0]
                .iter()
                .map(|contract| owned(contract.output_kinds.clone().unwrap_or_default()))
                .collect(),
        )),
        Request::CrossFileFrontier => {
            let plan = native_plan(true);
            let schema = contracts(std::slice::from_ref(&plan))?.rules[0][1]
                .schema
                .clone()
                .ok()
                .map(owned);
            let request = prepared(plan)?;
            Ok(Laid::Frontier {
                schema,
                region: matches!(
                    request.program().rules()[0].frontier[0].as_ref().ok(),
                    Some(InputSource::Region(_))
                ),
            })
        }
        Request::StepPublication => {
            let request = prepared(native_plan(false))?;
            let program = request.program();
            let origins = &program.rules()[0].original_steps;
            let region =
                |at: usize| -> Result<_, OperatingError> {
                    Ok(program.logical_regions()
                        [origins[at].as_ref().map_err(Clone::clone)?.logical.0])
                };
            let (first, last) = (region(0)?, region(1)?);
            Ok(Laid::Publications {
                distinct: first != last,
                consumers: (
                    program.regions()[first.0].consumers().0,
                    program.regions()[last.0].consumers().0,
                ),
            })
        }
        Request::Extended => {
            let plans = [extended_plan()];
            let operators = contracts_with(&plans, &|_| &[CellKind::Symbol, CellKind::File])?;
            let graph =
                compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
            Ok(Laid::Metadata {
                preparations: operators.preparations.0,
                schemas: graph.program_nodes[0]
                    .iter()
                    .flatten()
                    .map(|node| {
                        owned(
                            graph.nodes[node.0]
                                .contract
                                .output_kinds
                                .clone()
                                .unwrap_or_default(),
                        )
                    })
                    .collect(),
            })
        }
        Request::Ordered => {
            let plans = [ordered_plan()];
            let probes = std::cell::RefCell::new(Vec::new());
            let metadata = |relation| {
                probes.borrow_mut().push(relation);
                &[CellKind::Symbol, CellKind::File][..]
            };
            let operators = contracts_with(&plans, &metadata)?;
            let graph =
                compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
            // Actual missing-provider refusal is exercised by the facade runtime service_test's
            // missing-demanded-right-source case.
            Ok(Laid::Ordered {
                probes: Cow::Owned(probes.take()),
                inputs: Cow::Owned(graph.nodes[0].inputs.clone()),
            })
        }
        Request::DeepWalk => {
            let plans = [deep_walk_plan()];
            let operators = contracts(&plans)?;
            let graph =
                compile_prepared_observations(&plans, &operators, ObservationRequest::Independent);
            Ok(Laid::DeepWalk(
                graph.program_nodes[0][0].as_ref().ok().and_then(|output| {
                    graph.nodes[output.0]
                        .contract
                        .output_kinds
                        .clone()
                        .ok()
                        .map(owned)
                }),
            ))
        }
        Request::Borrowed => borrowed(),
        Request::InvalidFrontier => {
            let request = prepared(plan(
                vec![],
                vec![
                    PlanInput::Relation(Relation::Function),
                    PlanInput::Step(StepId(99)),
                ],
            ))?;
            let frontier = &request.program().rules()[0].frontier;
            Ok(Laid::InvalidFrontier {
                first: match frontier[0].as_ref().ok() {
                    Some(InputSource::Relation(relation)) => Some(*relation),
                    _ => None,
                },
                second: frontier[1].clone().err(),
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
