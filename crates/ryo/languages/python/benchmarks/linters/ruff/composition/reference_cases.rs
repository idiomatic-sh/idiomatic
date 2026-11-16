//! The reference-only verification of a bench's fixtures against Ruff.
use super::commands::reference;
use super::constants;
use super::locations::{default_scratch, directory};
use ryo_benchmark::domain::command_line::types::BenchmarkArgument;
use ryo_benchmark::inbound::use_cases::verify_reference_cases::interface::VerifyReferenceCasesUseCase as _;
use ryo_benchmark::inbound::use_cases::verify_rule_profiles::{
    interface::VerifyRuleProfilesUseCase as _, use_case::VerifyRuleProfilesUseCaseImpl,
};
use ryo_benchmark::outbound::linter::interface::LinterPort as _;
use ryo_benchmark::outbound::suite::{
    interface::SuitePort as _,
    types::{ArtifactPath, ArtifactText},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::path::{Path, PathBuf};

/// Validate authored fixture expectations against Ruff without loading a Ryo candidate.
pub(super) fn verify_reference_cases(
    mut args: impl Iterator<Item = BenchmarkArgument>,
) -> Result<ryo_benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount, Error>{
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
    let suite = ryo_benchmark::outbound::suite::factory::local();
    VerifyRuleProfilesUseCaseImpl::new(
        &suite,
        &ryo_benchmark::outbound::inventory::factory::local(),
    )
    .verify_rule_profiles(
        &source,
        ryo_benchmark_contracts::domain::requests::types::ShardPartition::All,
        ArtifactText(constants::VERIFY_REFERENCE_SCRATCH),
        &mut |profile| {
            verify_reference_profile(&executable, profile).map(|count| CaseCount(count.0))
        },
    )
    .map(|count| ryo_benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount(count.0))
}

/// Validate the fixture expectations of the profile at `source` against the reference, without loading a candidate.
fn verify_reference_profile(
    executable: &Path,
    source: &Path,
) -> Result<ryo_benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount, Error>{
    let suite_source = ryo_benchmark::outbound::suite::factory::local();
    let execution = ryo_benchmark::outbound::execution::factory::process();
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
    let count = ryo_benchmark::inbound::use_cases::verify_reference_cases::use_case::VerifyReferenceCasesUseCaseImpl::new(&suite_source, &ryo_benchmark::outbound::host::factory::local(), &ryo_benchmark::outbound::receipts::factory::local(ryo_benchmark::outbound::receipts::types::CaseReceipts::Unrecorded), &ryo_benchmark::outbound::context::factory::local(), &reference, &execution, &ryo_benchmark::outbound::inventory::factory::local(), &ryo_benchmark::outbound::evidence::factory::bmf_to(None)?).verify_reference_cases(
        &source.join(constants::RULES_DIRECTORY),
        &root,
    );
    drop(scratch);
    count
}
