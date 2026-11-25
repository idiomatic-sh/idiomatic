//! The reference-only verification of a bench's fixtures against Ruff.
use super::commands::reference;
use super::constants;
use super::locations::{default_scratch, directory};
use benchmark::domain::command_line::types::BenchmarkArgument;
use benchmark::inbound::use_cases::verify_reference_cases::interface::VerifyReferenceCasesUseCase as _;
use benchmark::inbound::use_cases::verify_rule_profiles::{
    interface::VerifyRuleProfilesUseCase as _, use_case::VerifyRuleProfilesUseCaseImpl,
};
use benchmark::outbound::linter::interface::LinterPort as _;
use benchmark::outbound::suite::{
    interface::SuitePort as _,
    types::{ArtifactPath, ArtifactText},
};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::path::{Path, PathBuf};

/// Validate authored fixture expectations against Ruff without loading a Ryo candidate.
pub(super) fn verify_reference_cases(
    mut args: impl Iterator<Item = BenchmarkArgument>,
) -> Result<
    benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount,
    Error,
> {
    let executable = PathBuf::from(
        args.next()
            .map(|argument| argument.0)
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_RUFF.into()))?,
    )
    .canonicalize()?;
    let source = args
        .next()
        .map_or_else(directory, |argument| PathBuf::from(argument.0));
    // Each rule is checked under its own settings profile, reached through the harness's one registrar.
    let suite = benchmark::outbound::suite::factory::local();
    VerifyRuleProfilesUseCaseImpl::new(
        &suite,
        &benchmark::outbound::inventory::factory::local(),
    )
    .verify_rule_profiles(
        &source,
        benchmark_contracts::domain::requests::types::ShardPartition::All,
        ArtifactText(constants::VERIFY_REFERENCE_SCRATCH),
        &mut |profile| {
            verify_reference_profile(&executable, profile).map(|count| CaseCount(count.0))
        },
    )
    .map(|count| benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount(count.0))
}

/// Validate the fixture expectations of the profile at `source` against the reference, without loading a candidate.
fn verify_reference_profile(
    executable: &Path,
    source: &Path,
) -> Result<
    benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount,
    Error,
> {
    let suite_source = benchmark::outbound::suite::factory::local();
    let execution = benchmark::outbound::execution::factory::process();
    let executable = executable.to_owned();
    let source = source.to_owned();
    let tool = suite_source.profile(ArtifactPath(&source))?;
    let expected = reference(executable.clone(), tool.clone())?.version_line(&tool);
    suite_source.verify_tool(ArtifactPath(&executable), &tool, &expected)?;
    let reference = reference(executable, tool)?;
    let scratch = default_scratch(
        &suite_source,
        ArtifactText(constants::VERIFY_REFERENCE_SCRATCH),
    )?;
    let root = suite_source.ensure_directory(ArtifactPath(scratch.path()))?;
    let count = benchmark::inbound::use_cases::verify_reference_cases::use_case::VerifyReferenceCasesUseCaseImpl::new(&suite_source, &benchmark::outbound::host::factory::local(), &benchmark::outbound::receipts::factory::local(benchmark::outbound::receipts::types::CaseReceipts::Unrecorded), &benchmark::outbound::context::factory::local(), &reference, &execution, &benchmark::outbound::inventory::factory::local(), &benchmark::outbound::evidence::factory::bmf(
        Box::new(std::io::sink()),
        Box::new(std::io::sink()),
    )).verify_reference_cases(
        &source.join(constants::RULES_DIRECTORY),
        &root,
    );
    drop(scratch);
    count
}
