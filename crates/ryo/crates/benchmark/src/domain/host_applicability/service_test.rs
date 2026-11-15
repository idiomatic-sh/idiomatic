//! Applicability follows the host facts: WSL outside a container and non-Unix hosts are unreliable.
use super::service::permissions;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::hosts::types::{
    FilePermissions, HostFacts, HostFamily, HostIsolation, HostKernel,
};

type Input = HostFacts;
type Value = FilePermissions;
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a native Unix kernel honours permissions"),
        word: Word::Succeeds,
        input: || HostFacts { family: HostFamily::Unix, kernel: HostKernel::Native, isolation: HostIsolation::Bare },
        expect: Expect::Value(FilePermissions::Honoured),
    },
    Case {
        name: CaseName("a Microsoft kernel outside a container is unreliable"),
        word: Word::Succeeds,
        input: || HostFacts { family: HostFamily::Unix, kernel: HostKernel::Microsoft, isolation: HostIsolation::Bare },
        expect: Expect::Value(FilePermissions::Unreliable),
    },
    Case {
        name: CaseName("a Microsoft kernel inside a container honours permissions"),
        word: Word::Succeeds,
        input: || HostFacts { family: HostFamily::Unix, kernel: HostKernel::Microsoft, isolation: HostIsolation::Container },
        expect: Expect::Value(FilePermissions::Honoured),
    },
    Case {
        name: CaseName("a host without POSIX permissions is unreliable"),
        word: Word::Succeeds,
        input: || HostFacts { family: HostFamily::Other, kernel: HostKernel::Native, isolation: HostIsolation::Bare },
        expect: Expect::Value(FilePermissions::Unreliable),
    },
];

fn subject(facts: Input) -> Result<Value, Failure> {
    Ok(permissions(&facts))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
