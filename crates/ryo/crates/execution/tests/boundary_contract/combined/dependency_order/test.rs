//! Combined cases of dependency order: a producer written after its consumer runs first, and the consumer answers what the
//! producer's named output answered, its rows or its refusal. The refusal's answer is the interpreter's at checkpoint-350
//! (64a99968a1), run on the same input.
use super::super::fixtures::{COLLECTED, answered};
use crate::boundary_fixture::fixtures::{FileRun, corpus, engine, run_file, source};
use crate::file_intents::fixtures::{
    consumer_before_its_producer_request, consumer_of_a_capture_request,
};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;

/// Rules run in dependency order, not in the order they are written: the producer (rule 1) runs before its consumer (rule 0, a
/// bound input of the producer's named output), so the producer's answer comes first. The producer's emit (its select) is
/// sound and answers its row; its capture, which only its named output reads, names column 1 of a step that keeps one column,
/// so the output is refused, and the consumer bound to it answers that refusal, which stops the file there.
#[test]
fn a_producer_runs_before_the_consumer_written_before_it() {
    let program = engine()
        .prepare(consumer_before_its_producer_request())
        .expect("a consumer written before its producer");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let rules = run_file(
        &request,
        FileRun::of(&[COLLECTED, COLLECTED], &elements),
        &mut Symbols::default(),
        &mut facts,
    );
    request.finish();
    assert_eq!(
        answered(rules),
        "rule 1: complete Table { kinds: [Node], cells: [Node(NodeId(2))], count: RowCount(1) }; ran, 0 captures\n\
         rule 0: failed Table(NoSuchColumn { index: 1 }); unreached\n\
         file: unfinished\n"
    );
}

/// A consumer reads what its producer's capture answered: the producer captures the function names `ma{rest}` matches (the two
/// `main`, not `other`) and names their nodes; the consumer, written first, is bound to them and answers exactly those.
#[test]
fn a_consumer_reads_the_rows_its_producers_capture_answered() {
    let program = engine()
        .prepare(consumer_of_a_capture_request())
        .expect("a consumer of a sound capture");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let mut symbols = Symbols::default();
    let table = crate::boundary_fixture::fixtures::named_functions(&mut symbols);
    let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let rules = run_file(
        &request,
        FileRun {
            source: b"mainother",
            ..FileRun::of(&[COLLECTED, COLLECTED], &elements)
        },
        &mut symbols,
        &mut facts,
    );
    request.finish();
    assert_eq!(
        answered(rules),
        "rule 1: complete Table { kinds: [Node, Symbol], cells: [Node(NodeId(0)), Symbol(Symbol(0)), Node(NodeId(1)), \
         Symbol(Symbol(0)), Node(NodeId(2)), Symbol(Symbol(1))], count: RowCount(3) }; ran, 0 captures\n\
         rule 0: complete Table { kinds: [Node], cells: [Node(NodeId(0)), Node(NodeId(1))], count: RowCount(2) }; ran, 0 captures\n\
         file: finished\n"
    );
}
