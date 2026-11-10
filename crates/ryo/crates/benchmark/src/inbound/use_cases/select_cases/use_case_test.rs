//! Each verification flag is read once into the choice it names; anything else is refused.
use super::{
    interface::SelectCasesUseCase as _,
    types::{CaseArgument, CaseRun},
    use_case::SelectCasesUseCaseImpl,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::receipts::types::CaseReceipts;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::ShardPartition;

type Input = &'static [&'static str];
/// What the flags chose: whether receipts are recorded and the shard's coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Chosen {
    recorded: bool,
    shard: Option<(usize, usize)>,
}
type Value = Chosen;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Invalid,
    Unexpected,
}
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no flag covers every case without recording receipts"),
        word: Word::Succeeds,
        input: || &[],
        expect: Expect::Value(Chosen {
            recorded: false,
            shard: None,
        }),
    },
    Case {
        name: CaseName("a shard flag names its zero-based coordinates"),
        word: Word::Succeeds,
        input: || &["--shard=1/3"],
        expect: Expect::Value(Chosen {
            recorded: false,
            shard: Some((1, 3)),
        }),
    },
    Case {
        name: CaseName("a receipt flag names where receipts are recorded"),
        word: Word::Succeeds,
        input: || &["--case-receipts=receipts"],
        expect: Expect::Value(Chosen {
            recorded: true,
            shard: None,
        }),
    },
    Case {
        name: CaseName("a repeated shard flag is refused"),
        word: Word::Refuses,
        input: || &["--shard=0/2", "--shard=1/2"],
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("an empty receipt destination is refused"),
        word: Word::Refuses,
        input: || &["--case-receipts="],
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a shard index outside its count is refused"),
        word: Word::Refuses,
        input: || &["--shard=2/2"],
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a shard flag without a count is refused"),
        word: Word::Refuses,
        input: || &["--shard=1"],
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("an unknown flag is refused"),
        word: Word::Refuses,
        input: || &["--unknown"],
        expect: Expect::Error(Refusal::Invalid),
    },
];

fn subject(arguments: Input) -> Result<Value, Failure> {
    let arguments = arguments
        .iter()
        .map(|argument| CaseArgument((*argument).to_owned()))
        .collect::<Vec<_>>();
    SelectCasesUseCaseImpl
        .select_cases(&arguments)
        .map(chosen)
        .map_err(|error| match error {
            ReportsError::Invalid(_) => Refusal::Invalid,
            ReportsError::Io(_) | ReportsError::Json(_) | ReportsError::Toml(_) => {
                Refusal::Unexpected
            }
        })
}

/// Reduce a run to what its flags chose.
fn chosen(run: CaseRun) -> Chosen {
    Chosen {
        recorded: matches!(run.receipts, CaseReceipts::Recorded(_)),
        shard: match run.shard {
            ShardPartition::All => None,
            ShardPartition::Shard(shard) => Some((shard.index().0, shard.count().0)),
        },
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
