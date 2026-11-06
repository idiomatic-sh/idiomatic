//! The `closure` and `axis` calls, on query text read by the real syntax adapter.

use super::super::traversing::test::{Input, Value, Failure, subject};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::ArgumentFault;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;

/// The children of each function: one axis walks from a node to its children.
const CHILDREN: &str = "let f = select * from \"function\"; let kids = axis(from=f, relation=\"syntax-child\", direction=child, domain=\"syntax-tree\"); emit kids subject id key(related);";

/// The parent of each name: the same walk the other way.
const PARENTS: &str = "let names = select * from \"syntax-tree\" where kind = \"name\"; let owners = axis(from=names, relation=\"syntax-child\", direction=parent, domain=\"syntax-tree\"); emit owners subject id key(related);";

/// The token just before each name under its parent: a sibling walk over the tree's own edges.
const BEFORE_NAMES: &str = "let names = select * from \"syntax-tree\" where kind = \"name\"; let before = axis(from=names, relation=\"syntax-child\", direction=previous-sibling, domain=\"syntax-tree\"); emit before subject id key(related);";

/// A sibling walk over edges a query made, which keep no order of the children.
const SIBLINGS_OF_A_STEP: &str = "let edges = select * from \"syntax-child\"; let names = select * from \"syntax-tree\" where kind = \"name\"; let after = axis(from=names, relation=edges, direction=next-sibling, domain=\"syntax-tree\"); emit after subject id key(related);";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("closure_omitted"),
        word: Word::Succeeds,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child); emit c subject parent key(child);",
        expect: Expect::Value("select closure"),
    },
    Case {
        name: CaseName("closure_empty"),
        word: Word::Succeeds,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[]); emit c subject parent key(child);",
        expect: Expect::Value("select closure"),
    },
    Case {
        name: CaseName("closure_one_key"),
        word: Word::Succeeds,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[file]); emit c subject parent key(child);",
        expect: Expect::Value("select closure"),
    },
    Case {
        name: CaseName("closure_multiple_keys"),
        word: Word::Succeeds,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[file, ordinal]); emit c subject parent key(child);",
        expect: Expect::Value("select closure"),
    },
    Case {
        name: CaseName("closure_refuses_repeated_key"),
        word: Word::Refuses,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[file,file]); emit c subject parent key(child);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("closure_refuses_endpoint_key"),
        word: Word::Refuses,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[parent]); emit c subject parent key(child);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("closure_refuses_missing_key"),
        word: Word::Refuses,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[missing]); emit c subject parent key(child);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::UnknownColumn { .. }))
        }),
    },
    Case {
        name: CaseName("closure_refuses_scalar_keys"),
        word: Word::Refuses,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=file); emit c subject parent key(child);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::ExpectedNames,
                    ..
                }
            ))
        }),
    },
    Case {
        name: CaseName("closure_refuses_numeric_key"),
        word: Word::Refuses,
        input: || "let e = select * from \"syntax-child\"; let c = closure(edges=e, source=parent, destination=child, keys=[42]); emit c subject parent key(child);",
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::ExpectedNames,
                    ..
                }
            ))
        }),
    },
    Case {
        name: CaseName("a_walk_to_the_children_of_a_node_is_one_axis"),
        word: Word::Succeeds,
        input: || CHILDREN,
        expect: Expect::Value("select axis"),
    },
    Case {
        name: CaseName("a_walk_to_the_parent_of_a_node_is_one_axis"),
        word: Word::Succeeds,
        input: || PARENTS,
        expect: Expect::Value("select axis"),
    },
    Case {
        name: CaseName("a_sibling_of_the_tree_is_one_axis"),
        word: Word::Succeeds,
        input: || BEFORE_NAMES,
        expect: Expect::Value("select axis"),
    },
    Case {
        name: CaseName("a_sibling_walk_over_edges_a_query_made_is_refused"),
        word: Word::Refuses,
        input: || SIBLINGS_OF_A_STEP,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::BadArgument {
                    reason: ArgumentFault::SiblingOverTree,
                    ..
                }
            ))
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
