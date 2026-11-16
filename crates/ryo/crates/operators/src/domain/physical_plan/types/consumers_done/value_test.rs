//! A node made after some of its consumers were done waits only for the rest; what was done is forgotten with the node's
//! release, and a node never told of a consumer done waits for all of them. A reader that does not run is done with each of
//! its reads, unless nothing consumes it in the plan, when its sources never counted them.
use super::value::ConsumersDone;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::{PhysicalNode, PhysicalPlan};
use crate::domain::physical_plan::types::{
    EngineUses, NodeFlow, NodeReaders, PlanNodeId, RowSource,
};

/// What a case does before node 1, of three readers and two engine uses, is made.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Nothing.
    Nothing,
    /// One reader and one engine use of node 1 done.
    OneOfEach,
    /// One reader and one engine use of node 1 done, then node 1 released.
    Forgotten,
    /// Five readers and three engine uses of node 1 done: more than it has.
    MoreThanItHas,
    /// One reader and one engine use of node 0 done.
    AnotherNode,
    /// Node 2, which reads node 1 on both sides and is emitted, does not run.
    ReaderUnread,
    /// Node 2, which reads node 1 on both sides and which no node reads and no engine use keeps, does not run.
    UnconsumedReaderUnread,
}
type Input = Given;
/// The readers and engine uses node 1 still waits for once made.
type Value = (NodeReaders, EngineUses);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_waits_for_every_consumer_none_is_done"),
        word: Word::Succeeds,
        input: || Given::Nothing,
        expect: Expect::Value((NodeReaders(3), EngineUses(2))),
    },
    Case {
        name: CaseName("a_consumer_done_before_the_making_is_not_waited_for"),
        word: Word::Succeeds,
        input: || Given::OneOfEach,
        expect: Expect::Value((NodeReaders(2), EngineUses(1))),
    },
    Case {
        name: CaseName("a_released_node_forgets_what_was_done"),
        word: Word::Succeeds,
        input: || Given::Forgotten,
        expect: Expect::Value((NodeReaders(3), EngineUses(2))),
    },
    Case {
        name: CaseName("more_done_than_a_node_has_leaves_nothing_to_wait_for"),
        word: Word::Succeeds,
        input: || Given::MoreThanItHas,
        expect: Expect::Value((NodeReaders(0), EngineUses(0))),
    },
    Case {
        name: CaseName("a_reader_that_does_not_run_is_done_with_each_of_its_reads"),
        word: Word::Succeeds,
        input: || Given::ReaderUnread,
        expect: Expect::Value((NodeReaders(1), EngineUses(2))),
    },
    Case {
        name: CaseName("a_reader_no_consumer_keeps_retires_no_read_of_its_sources"),
        word: Word::Succeeds,
        input: || Given::UnconsumedReaderUnread,
        expect: Expect::Value((NodeReaders(3), EngineUses(2))),
    },
    Case {
        name: CaseName("what_another_node_is_done_with_is_its_own"),
        word: Word::Succeeds,
        input: || Given::AnotherNode,
        expect: Expect::Value((NodeReaders(3), EngineUses(2))),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    let (zero, one) = (PlanNodeId(0), PlanNodeId(1));
    let mut done = ConsumersDone::default();
    // A plan of three nodes, as the cases count them.
    // A plan of three nodes; node 2 reads node 1 on both sides, consumed by an engine use or by nothing as the case says.
    let reader = |engine: EngineUses| PhysicalNode {
        lowering: Lowering::Distinct {
            sources: [RowSource::Node(one), RowSource::Node(one)],
        },
        readers: NodeReaders(0),
        engine,
        flow: NodeFlow::Held,
    };
    let plain = || PhysicalNode {
        lowering: Lowering::Unlowered,
        readers: NodeReaders(3),
        engine: EngineUses(2),
        flow: NodeFlow::Held,
    };
    let consumed = if matches!(given, Given::UnconsumedReaderUnread) {
        EngineUses(0)
    } else {
        EngineUses(1)
    };
    let plan = PhysicalPlan {
        nodes: vec![plain(), plain(), reader(consumed)],
        ..PhysicalPlan::default()
    };
    let times = |count: usize, done: &mut ConsumersDone, node: PlanNodeId, engine: bool| {
        for _ in 0..count {
            if engine {
                done.engine_done(&plan, node);
            } else {
                done.reader_done(&plan, node);
            }
        }
    };
    match given {
        Given::Nothing => {}
        Given::OneOfEach | Given::Forgotten => {
            times(1, &mut done, one, false);
            times(1, &mut done, one, true);
            if matches!(given, Given::Forgotten) {
                done.forgotten(one);
            }
        }
        Given::MoreThanItHas => {
            times(5, &mut done, one, false);
            times(3, &mut done, one, true);
        }
        Given::ReaderUnread | Given::UnconsumedReaderUnread => done.unread(&plan, PlanNodeId(2)),
        Given::AnotherNode => {
            times(1, &mut done, zero, false);
            times(1, &mut done, zero, true);
        }
    }
    Ok((
        done.readers_left(one, NodeReaders(3)),
        done.engine_left(one, EngineUses(2)),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
