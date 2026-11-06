//! Each family leaves through the production entry that acquires it; every exit ends with the
//! owner generation retired and `(required, optional, unique) == (0, 0, 0)` asserted here.
//! `RawKeyOrder` needs a `KernelPermit`, so its exits are the program-level `raw_scope` cases.
use super::request::{observed, request};
use super::{numbers, SmallTree, small_tree, SMALL_SOURCE};
use crate::application::file_dag::{
    closure_partitions::PartitionedClosure,
    derived::owner::{
        ViewContext,
        acquisition::admission::service::AcquiredView,
        fixtures::graph::GraphFixture,
        index::service::LIMIT,
        selection::fixtures::SelectionFixture,
    },
    relations::RelationOwner,
    testing::program_derived_port,
    workspace::{InputAdmissions, InvocationWorkspace, ViewInputs},
};
use crate::domain::{
    derived_views::types::PreparedViewUse,
    operating::{
        errors::OperatingError,
        types::{AfterTheFiles, OperatorContext},
    },
    operation_schema::types::InputShape,
    operator::contract::views::BorrowedRows,
    stored_graph::types::GraphWalk,
    stored_selection::{fixtures::program, types::Selector},
    table_cells::types::{TreeForm, TreeRows},
    table_model::types::{Rows, SelectionIdentity},
    table_scratch::types::OperatorScratch,
    table_texts::types::Texts,
};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::{
        service::to_bits,
        types::{CellKind, ColumnsView, FactColumn, FileId, Integer, NodeId, Relation, Value},
    },
    terms::{
        service::term_symbol,
        types::{DeclarationKind, NameText, Symbol},
    },
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{
        ColumnIndex, Comparison, Input, LiteralText, Operand, ParameterId, QueryLiteral, Test,
    },
    query_bindings::types::ParameterBinding,
};
use std::{rc::Rc, sync::Arc};

/// How the last consumer of a family leaves through its production entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum Exit {
    /// The view is acquired, read and dropped.
    Succeeds,
    /// The demand is withdrawn after the request is prepared and before the view is read.
    Cancelled,
    /// The production entry refuses; nothing acquired may stay charged.
    Errors,
}
/// The production entry a family is acquired through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum Entry {
    /// `InputAdmissions::{acquire_derived, try_native_lookup}` over an invocation workspace.
    Workspace,
    /// The `PartitionedClosure` cursor of `closure_partitions`.
    Partition,
    /// `DerivedOwner::select` over a stored relation.
    Selection,
    /// `acquire_graph` and `acquire_graph_domain` over an admitted graph input.
    Graph,
    /// `finite_raw` and `with_raw_order`, which require a `KernelPermit`.
    Permit,
}
/// Exhaustive on purpose: a new family must name its production entry before this file compiles.
pub(in crate::application::file_dag) fn entry(kind: DerivedKind) -> Entry {
    match kind {
        DerivedKind::RawKeyOrder => Entry::Permit,
        DerivedKind::Equality
        | DerivedKind::CycleAdjacency
        | DerivedKind::OrderedKeys
        | DerivedKind::OrderedDfs
        | DerivedKind::NativeLookup => Entry::Workspace,
        DerivedKind::PartitionOrder | DerivedKind::ClosureAdjacency => Entry::Partition,
        DerivedKind::PredicateMatches
        | DerivedKind::CandidatePostings
        | DerivedKind::DistinctRepresentatives => Entry::Selection,
        DerivedKind::Adjacency | DerivedKind::Membership => Entry::Graph,
    }
}
/// Drive one exit of `kind` through its production entry. `RawKeyOrder` is not driven here.
pub(in crate::application::file_dag) fn leave(
    kind: DerivedKind,
    exit: Exit,
) -> Result<DerivedKind, OperatingError> {
    match entry(kind) {
        Entry::Workspace => workspace(kind, exit),
        Entry::Partition => partition(kind, exit),
        Entry::Selection => selection(kind, exit),
        Entry::Graph => graph(kind, exit),
        Entry::Permit => Err(OperatingError::WrongOperation),
    }
}
fn released(kind: DerivedKind, exit: Exit, ledger: (usize, usize, usize)) {
    assert_eq!(
        ledger,
        (0, 0, 0),
        "{kind:?} {exit:?}: every lease retires with the owner generation"
    );
}

fn workspace(kind: DerivedKind, exit: Exit) -> Result<DerivedKind, OperatingError> {
    let owner = Rc::new(RelationOwner::new(true, program_derived_port()));
    let result = through_workspace(kind, exit, &owner);
    owner.release_population();
    released(kind, exit, owner.derived.publication_ownership_bytes());
    result
}
fn through_workspace(
    kind: DerivedKind,
    exit: Exit,
    owner: &Rc<RelationOwner>,
) -> Result<DerivedKind, OperatingError> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut table = numbers();
    let native = kind == DerivedKind::NativeLookup;
    if native {
        table.kinds[0] = CellKind::Node;
        table.cells[0] = Value::Node(NodeId(0));
        table.cells[4] = Value::Node(NodeId(1));
    }
    let first = owner.computed(table)?;
    let first_held = owner.resolve(first.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let tree = TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    };
    let tree_rows = Rows::Tree(tree);
    let second = if native {
        owner.source(Relation::SyntaxTree, &tree_rows)?
    } else {
        owner.computed(numbers())?
    };
    let second_held;
    let second_rows = if native {
        BorrowedRows::Tree(tree)
    } else {
        second_held = owner.resolve(second.as_ref(), &|_| Err(OperatingError::NotRead))?;
        BorrowedRows::from(&second_held)
    };
    let rows = [BorrowedRows::from(&first_held), second_rows];
    let pair = matches!(
        kind,
        DerivedKind::OrderedKeys | DerivedKind::OrderedDfs | DerivedKind::NativeLookup
    );
    let len = if pair { 2 } else { 1 };
    let shapes = [
        InputShape {
            kinds: rows[0].kinds(),
            tree: None,
        },
        InputShape {
            kinds: rows[1].kinds(),
            tree: native.then_some(TreeForm::Elements),
        },
    ];
    let plan = Arc::new(owner.derived.prepare_view(&request(kind), &shapes[..len])?);
    let use_ = PreparedViewUse {
        plan,
        inputs: [0, 1],
        relations: [None, None],
        len: u8::try_from(len).map_err(|_| OperatingError::ShapeMismatch)?,
    };
    let mut published = vec![first.clone()];
    if pair {
        published.push(second.clone());
    }
    let mut workspace = InvocationWorkspace::new(Rc::clone(owner));
    workspace.capture_admitted(published);
    let admissions = workspace.inputs(&rows[..len]);
    match exit {
        Exit::Cancelled => {
            drop(admissions);
            workspace.release_inputs();
            Ok(kind)
        }
        Exit::Succeeds => {
            let view = demand(kind, &admissions, &use_, &mut symbols, false)?;
            assert_eq!(observed(&view)?, kind, "{kind:?}: the leased view reports its family");
            Ok(kind)
        }
        Exit::Errors => {
            if !native {
                let wrong = demand(kind, &admissions, &use_, &mut symbols, true);
                assert!(
                    wrong.is_err(),
                    "{kind:?}: inputs of another family are refused by the owner"
                );
            }
            owner.release_population();
            demand(kind, &admissions, &use_, &mut symbols, false).map(|_| kind)
        }
    }
}
fn demand(
    kind: DerivedKind,
    admissions: &InputAdmissions<'_, '_>,
    use_: &PreparedViewUse,
    symbols: &mut Symbols,
    mismatched: bool,
) -> Result<AcquiredView, OperatingError> {
    let context = ViewContext::default();
    if mismatched {
        let wrong = if matches!(kind, DerivedKind::OrderedKeys | DerivedKind::OrderedDfs) {
            ViewInputs::Rows
        } else {
            ViewInputs::MembershipValues
        };
        return admissions.acquire_derived(use_, context, wrong);
    }
    match kind {
        DerivedKind::OrderedKeys => {
            let texts = Texts {
                symbols: &*symbols,
                source: SourceView(SMALL_SOURCE),
                paths: &[],
                arenas: &[],
            };
            admissions.acquire_derived(use_, context, ViewInputs::Text(&texts))
        }
        DerivedKind::OrderedDfs => {
            let mut query = (&mut *symbols).into();
            admissions.acquire_derived(
                use_,
                context,
                ViewInputs::Dfs {
                    symbols: &mut query,
                    source: SourceView(SMALL_SOURCE),
                    paths: &[],
                    arenas: &[],
                },
            )
        }
        DerivedKind::NativeLookup => admissions
            .try_native_lookup(use_)?
            .ok_or(OperatingError::WrongOperation),
        _ => admissions.acquire_derived(use_, context, ViewInputs::Rows),
    }
}

fn partition(kind: DerivedKind, exit: Exit) -> Result<DerivedKind, OperatingError> {
    let owner = Rc::new(RelationOwner::new(true, program_derived_port()));
    let result = through_partition(kind, exit, &owner);
    owner.release_population();
    released(kind, exit, owner.derived.publication_ownership_bytes());
    result
}
fn through_partition(
    kind: DerivedKind,
    exit: Exit,
    owner: &Rc<RelationOwner>,
) -> Result<DerivedKind, OperatingError> {
    let publication = owner.computed(numbers())?;
    let held = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let mut cursor = PartitionedClosure::prepare(
        &publication,
        BorrowedRows::from(&held),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        Rc::clone(owner),
    )?;
    match exit {
        Exit::Cancelled => Ok(kind),
        Exit::Succeeds => {
            assert!(cursor.advance()?, "{kind:?}: the first partition is built");
            Ok(kind)
        }
        Exit::Errors => {
            owner.release_population();
            cursor.advance().map(|_| kind)
        }
    }
}

fn graph(kind: DerivedKind, exit: Exit) -> Result<DerivedKind, OperatingError> {
    let fixture = GraphFixture::new(&[Relation::Function, Relation::Identifier], &[], LIMIT);
    let result = through_graph(kind, exit, &fixture);
    fixture.clear();
    let state = fixture.snapshot();
    released(
        kind,
        exit,
        (state.required_bytes, state.optional_bytes, state.unique_bytes),
    );
    result
}
fn through_graph(
    kind: DerivedKind,
    exit: Exit,
    fixture: &GraphFixture,
) -> Result<DerivedKind, OperatingError> {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![2, 3])];
    let rows = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let domain_columns = [FactColumn(vec![3]), FactColumn(vec![0])];
    let domain_rows = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &domain_columns,
    });
    let source = Input::Relation(Relation::Function);
    let edges = [ColumnIndex(0), ColumnIndex(1)];
    let graph = fixture
        .acquire(source, &rows, edges, &[])?
        .ok_or(OperatingError::WrongOperation)?;
    let membership = kind == DerivedKind::Membership;
    let domain = || {
        fixture
            .domain(
                &graph,
                Input::Relation(Relation::Identifier),
                &domain_rows,
                ColumnIndex(0),
            )?
            .ok_or(OperatingError::WrongOperation)
    };
    match exit {
        Exit::Cancelled => Ok(kind),
        Exit::Succeeds => {
            if membership {
                drop(domain()?);
            }
            Ok(kind)
        }
        Exit::Errors => {
            let held = if membership { Some(domain()?) } else { None };
            fixture.clear();
            let mut walk = GraphWalk::default();
            let refused = match &held {
                Some(domain) => fixture
                    .nearest(
                        &graph,
                        domain,
                        0,
                        crate::domain::stored_graph::types::Orientation::Forward,
                        &mut vec![],
                        &mut walk,
                    )
                    .map(|_| ()),
                None => fixture
                    .reach(&graph, &[], Value::Number(Integer(1)), &mut walk)
                    .map(|_| ()),
            };
            refused.map(|()| kind)
        }
    }
}

/// A fixed set of integer rows for the distinct and predicate selectors.
const DISTINCT: [&[u64]; 3] = [&[7, 7, 7, 7, 7, 8], &[1, 2, 0, 7, 0, 1], &[10, 10, 20, 11, 20, 10]];
fn selection(kind: DerivedKind, exit: Exit) -> Result<DerivedKind, OperatingError> {
    if kind == DerivedKind::CandidatePostings {
        return fanout(exit);
    }
    let columns = DISTINCT.map(|column| FactColumn(column.to_vec()));
    let source = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
        columns: &columns,
    };
    let mut distinct = (*program(7)).clone();
    distinct.columns = vec![ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)];
    distinct.distinct = true;
    let distinct = Arc::new(distinct);
    let mut bag = (*distinct).clone();
    bag.distinct = false;
    let bag = Arc::new(bag);
    let chosen = if kind == DerivedKind::DistinctRepresentatives {
        &distinct
    } else {
        &bag
    };
    run_with(
        Selected {
            kind,
            relation: Relation::Function,
            active: &[Arc::clone(&distinct)],
            authored: vec![Arc::clone(&distinct), Arc::clone(&bag)],
            source,
            kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
            chosen,
            fanned: false,
        },
        exit,
        &mut Symbols::default(),
    )
}
/// The text-equality selectors of one relation share a single fanout scan.
fn fanout(exit: Exit) -> Result<DerivedKind, OperatingError> {
    let mut symbols = Symbols::default();
    let mut bits = vec![to_bits(Value::Symbol(term_symbol(DeclarationKind::Method))).0; 128];
    for row in [1, 4, 8] {
        bits[row] = to_bits(Value::Symbol(term_symbol(DeclarationKind::Class))).0;
    }
    bits[127] = to_bits(Value::Symbol(Symbol(u32::MAX))).0;
    for number in 0..7 {
        let word = intern_derived(
            &mut symbols,
            ByteSlice(b""),
            NameText(&format!("word{number}")),
        );
        bits[20 + number * 2] = to_bits(Value::Symbol(word)).0;
    }
    let columns = [FactColumn(bits)];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let selector = |text: String| {
        Arc::new(Selector {
            relation: Relation::Defines,
            identity: Arc::new(SelectionIdentity {
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
                    origin: None,
                }],
                bindings: Vec::new(),
            }),
            columns: vec![ColumnIndex(0), ColumnIndex(0)],
            distinct: false,
        })
    };
    let mut selectors = vec![selector("class".into())];
    selectors.extend((0..7).map(|number| selector(format!("word{number}"))));
    let chosen = Arc::clone(&selectors[0]);
    run_with(
        Selected {
            kind: DerivedKind::CandidatePostings,
            relation: Relation::Defines,
            active: &selectors,
            authored: selectors.clone(),
            source,
            kinds: &[CellKind::Symbol],
            chosen: &chosen,
            fanned: true,
        },
        exit,
        &mut symbols,
    )
}
/// A selector whose parameter is bound to a table: the comparison refuses with `WrongBinding`.
fn refusing(base: &Selector) -> Arc<Selector> {
    let mut bad = base.clone();
    let bind = Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(0)),
        right: Operand::Parameter(ParameterId(0)),
        origin: None,
    };
    bad.identity = Arc::new(SelectionIdentity {
        comparisons: vec![base.identity.comparisons[0].clone(), bind],
        bindings: vec![ParameterBinding::Tables(Vec::new())],
    });
    Arc::new(bad)
}
/// One selection demand: the selectors the request prepared and the one the consumer selects.
struct Selected<'a> {
    kind: DerivedKind,
    relation: Relation,
    active: &'a [Arc<Selector>],
    authored: Vec<Arc<Selector>>,
    source: ColumnsView<'a>,
    kinds: &'static [CellKind],
    chosen: &'a Arc<Selector>,
    fanned: bool,
}
fn run_with(
    Selected {
        kind,
        relation,
        active,
        mut authored,
        source,
        kinds,
        chosen,
        fanned,
    }: Selected<'_>,
    exit: Exit,
    symbols: &mut Symbols,
) -> Result<DerivedKind, OperatingError> {
    let error = refusing(chosen);
    authored.push(Arc::clone(&error));
    let fixture = SelectionFixture::new(&[relation], &[], active.to_vec(), authored, kinds)?;
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let result = {
        let mut context = OperatorContext {
            phase: AfterTheFiles::Before,
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut *symbols).into(),
            source: SourceView(b""),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        match exit {
            Exit::Cancelled => Ok(kind),
            Exit::Succeeds => {
                let mut every = active.to_vec();
                if !fanned {
                    every = vec![Arc::clone(chosen)];
                }
                for selector in &every {
                    assert!(
                        fixture.select(selector, source, &mut context)?.is_some(),
                        "{kind:?}: the selection is published"
                    );
                }
                if fanned {
                    assert_eq!(
                        fixture.snapshot(None).fanouts,
                        1,
                        "{kind:?}: the aliases share one physical scan"
                    );
                }
                Ok(kind)
            }
            Exit::Errors => fixture
                .select(&error, source, &mut context)
                .map(|_| kind),
        }
    };
    fixture.owner().release_population();
    released(
        kind,
        exit,
        fixture.owner().derived.publication_ownership_bytes(),
    );
    result
}
