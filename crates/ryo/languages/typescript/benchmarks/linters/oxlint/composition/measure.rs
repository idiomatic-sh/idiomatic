//! The measure commands: certify both contestants, then compare them on the pinned corpus.
use super::certification::{
    Certifying, MeasurePaths, StagedSuite, candidate_linter, cases_for, paths, stage,
};
use super::commands;
use super::constants;
use super::constants::UNEXPECTED_MEASUREMENT_ARGUMENT;
use super::locations::{default_scratch, directory};
use benchmark::domain::command_line::types::{
    BenchmarkArgument, Selection, measure_command::value::MeasureCommand,
};
use benchmark::domain::measurements::types::{
    LanguageTitle, certification_summary::value::CertificationSummary,
    corpus_shape::value::CorpusShape, measurement_environment::value::MeasurementEnvironment,
    rule_tally::value::RuleTally,
};
use benchmark::domain::suite::types::certificate::value::Certificate;
use benchmark::inbound::use_cases::admit_measurement::{
    interface::AdmitMeasurementUseCase as _, types::MeasurementAdmission,
    use_case::AdmitMeasurementUseCaseImpl,
};
use benchmark::inbound::use_cases::audit::interface::AuditUseCase as _;
use benchmark::inbound::use_cases::certify::{
    interface::CertifyUseCase as _, use_case::CertifyUseCaseImpl,
};
use benchmark::inbound::use_cases::measure_corpus::{
    interface::MeasureCorpusUseCase as _,
    types::{CorpusLanguage, CorpusMeasurement, ProfileLabel, SourceSuffix, ToolTitle},
    use_case::MeasureCorpusUseCaseImpl,
};
use benchmark::outbound::discovery::interface::DiscoveryPort;
use benchmark::outbound::evidence::interface::EvidencePort;
use benchmark::outbound::execution::interface::ExecutionPort;
use benchmark::outbound::linter::interface::LinterPort;
use benchmark::outbound::suite::{
    interface::SuitePort,
    types::{ArtifactPath, ArtifactText},
};
use benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use benchmark_contracts::domain::measurements::types::MeasurementMode;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::requests::constants::{DOCUMENT_ARGUMENT, VERIFICATION_ARGUMENT};
use benchmark_contracts::domain::workloads::types::Suite;
use std::{path::PathBuf, sync::Arc};

/// Certify both contestants, then measure every project of the corpus and publish or report the samples.
pub(super) fn measure(
    command: MeasureCommand<'_>,
    args: &mut dyn Iterator<Item = BenchmarkArgument>,
    observations: &mut dyn std::io::Write,
    suite_source: &Arc<dyn SuitePort>,
) -> Result<(), Error> {
    if cfg!(debug_assertions) {
        return Err(Error::Invalid(constants::RELEASE_REQUIRED.into()));
    }
    let MeasurePaths {
        executable,
        cache,
        collector_path,
    } = paths(args, suite_source)?;
    let tool = suite_source.profile(ArtifactPath(&directory()))?;
    // The bench verified the reference tool's pinned bytes and version line once, before any rule (`verify-tool`); the
    // registration pins its digest, which the inventory checks before and after every rule.
    let declared_inventory = tool.defaults.clone();
    let audit = benchmark::inbound::use_cases::audit::use_case::AuditUseCaseImpl::new(Arc::clone(
        &suite_source,
    ))
    .audit(&directory(), &tool)?;
    let authored_total = audit.authored_count();
    let selection_or_inventory = args.next().map(|argument| argument.0);
    let scope = command.scope();
    let Certifying {
        audit,
        cases: case_source,
    } = cases_for(
        scope,
        audit,
        selection_or_inventory.as_deref().map(Selection),
        &cache,
        suite_source,
    )?;
    let verification = args
        .next()
        .and_then(|argument| {
            argument
                .0
                .strip_prefix(VERIFICATION_ARGUMENT)
                .map(PathBuf::from)
        })
        .filter(|directory| !directory.as_os_str().is_empty())
        .ok_or_else(|| Error::Invalid(constants::EXPECTED_VERIFICATION.into()))?;
    let document = args
        .next()
        .and_then(|argument| {
            argument
                .0
                .strip_prefix(DOCUMENT_ARGUMENT)
                .map(PathBuf::from)
        })
        .filter(|file| !file.as_os_str().is_empty())
        .ok_or_else(|| Error::Invalid(constants::EXPECTED_DOCUMENT.into()))?;
    if args.next().is_some() {
        return Err(Error::Invalid(UNEXPECTED_MEASUREMENT_ARGUMENT.into()));
    }
    let verified = audit.authored().to_vec();
    let measurement_mode =
        AdmitMeasurementUseCaseImpl::new().admit_measurement(&MeasurementAdmission {
            scope,
            audit: &audit,
            tool: &tool,
            declared_inventory,
        })?;
    let tally = RuleTally::of(&tool, authored_total);
    let project_context_mode = tool.project_context_mode;
    let project_search_roots = tool.project_search_roots.clone();
    let declared_rule_context = tool.rule_context.clone();
    let staged = stage(suite_source, &cache, audit.authored())?;
    let configured_cohort = benchmark_contracts::domain::requests::types::CohortSelection::Explicit(
        staged.tool.defaults.clone(),
    );
    let candidate_executable = staged.scratch.join(constants::CANDIDATE_EXECUTABLE);
    suite_source.copy_file(
        ArtifactPath(&std::env::current_exe()?),
        ArtifactPath(&candidate_executable),
    )?;
    let candidate = candidate_linter(&staged, candidate_executable.clone());
    let StagedSuite {
        scratch: selected_scratch,
        profile: selected_profile,
        workload: frozen_workload,
        tool: staged_tool,
    } = staged;
    let selected_rules = selected_profile.join(constants::RULES_DIRECTORY);
    let discovery: Arc<dyn DiscoveryPort> =
        Arc::new(benchmark::outbound::discovery::factory::listing(
            executable.clone(),
            crate::adapter::LinterOxlintAdapter::discovery_arguments(&selected_scratch)?,
        ));
    let parser = staged_tool.parser_configuration.clone();
    let reference = Arc::new(commands::reference(executable.clone(), staged_tool));
    // Composition is not DRY: this port wiring repeats in each linter's composition by the owner's rule; only pure values are shared.
    let collector_settings =
        commands::collector(suite_source.as_ref(), &collector_path, &frozen_workload)?;
    let execution = commands::timed(collector_settings, observations)?;
    let case_scratch =
        default_scratch(suite_source.as_ref(), ArtifactText(constants::CASE_SCRATCH))?;
    let certificate = CertifyUseCaseImpl::new(
        Arc::<_>::clone(&suite_source),
        Arc::new(benchmark::outbound::host::factory::local()),
        Arc::new(benchmark::outbound::receipts::factory::local(
            benchmark::outbound::receipts::types::CaseReceipts::Unrecorded,
        )),
        Arc::new(benchmark::outbound::context::factory::local()),
        Arc::<super::super::adapter::LinterOxlintAdapter>::clone(&reference),
        Arc::<_>::clone(&candidate),
        Arc::<_>::clone(&execution),
        Arc::new(benchmark::outbound::inventory::factory::local()),
        Arc::new(benchmark::outbound::scheduler::factory::threads()),
    )
    .certify(
        &selected_profile,
        &frozen_workload,
        &executable,
        &candidate_executable,
        &selected_rules,
        case_scratch.path(),
        scope,
        &case_source,
    )?;
    drop(case_scratch);
    writeln!(
        observations,
        "{}",
        CertificationSummary {
            scope,
            tally: &tally,
            audit: &audit,
            certificate: &certificate,
            profile: &selected_profile,
        }
    )?;
    writeln!(observations, "{MeasurementEnvironment}")?;
    compare(
        Certified {
            reference,
            candidate,
            discovery,
            execution,
            certificate,
            cohort: configured_cohort,
            rule_context: declared_rule_context,
            search_roots: project_search_roots,
            context_mode: project_context_mode,
            mode: measurement_mode,
            cache,
            workload: frozen_workload,
            verification,
            verified,
            document,
            parser,
        },
        suite_source,
        observations,
    )
}

/// What certification leaves for the comparison: both contestants, the certificate they hold and where the corpus is measured.
struct Certified {
    reference: Arc<super::super::adapter::LinterOxlintAdapter>,
    candidate: Arc<dyn LinterPort>,
    discovery: Arc<dyn DiscoveryPort>,
    execution: Arc<dyn ExecutionPort>,
    certificate: Certificate,
    cohort: benchmark_contracts::domain::requests::types::CohortSelection,
    rule_context: benchmark_contracts::domain::workloads::types::RuleContextProfile,
    search_roots: benchmark_contracts::domain::requests::types::SearchRoots,
    context_mode: ProjectContextMode,
    mode: MeasurementMode,
    cache: PathBuf,
    workload: PathBuf,
    /// The inventory directory the bench's verification wrote.
    verification: std::path::PathBuf,
    /// The rules it verified.
    verified: Vec<ryo::linter::RuleId>,
    /// The file the bench's one document is written to.
    document: PathBuf,
    /// The parser configuration both contestants run with.
    parser: benchmark_contracts::domain::requests::types::ParserSelection,
}

/// Measure the corpus of a certified pair and publish or report the samples.
fn compare(
    certified: Certified,
    suite_source: &Arc<dyn SuitePort>,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let Certified {
        reference,
        candidate,
        discovery,
        execution,
        certificate,
        cohort,
        rule_context,
        search_roots,
        context_mode,
        mode,
        cache,
        workload,
        verification,
        verified,
        document,
        parser,
    } = certified;
    let suite: Suite = suite_source.projects(ArtifactPath(&workload))?;
    CorpusShape {
        language: &constants::TYPESCRIPT.into(),
        display: LanguageTitle(constants::LANGUAGE_DISPLAY),
    }
    .admit(&suite)?;
    let evidence: Arc<dyn EvidencePort> = Arc::new(
        benchmark::outbound::evidence::factory::bmf_document(document, Box::new(std::io::stderr())),
    );
    let suffixes = constants::SOURCE_SUFFIXES.map(SourceSuffix);
    let language = constants::TYPESCRIPT.into();
    let invalid_syntax = ryo::linter::RuleId::try_from(crate::constants::INVALID_SYNTAX)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let bench = directory();
    MeasureCorpusUseCaseImpl::new(
        Arc::<super::super::adapter::LinterOxlintAdapter>::clone(&reference),
        Arc::<_>::clone(&candidate),
        discovery,
        execution,
        Arc::<_>::clone(suite_source),
        Arc::new(benchmark::outbound::context::factory::local()),
        evidence,
        Arc::new(benchmark::outbound::workspace::factory::git(
            cache.join(constants::PROJECTS_DIRECTORY),
        )),
        Arc::new(benchmark::outbound::host::factory::local()),
        Arc::new(
            benchmark::outbound::engine::factory::local_with_frontend_diagnostic(
                ryo::linter::Languages(vec![ryo_typescript::linked()]),
                None,
                commands::frontend_diagnostic()?,
            ),
        ),
        Arc::new(benchmark::outbound::process_meter::factory::linux()),
    )
    .measure_corpus(
        &CorpusMeasurement {
            bench: &bench,
            projects: suite.projects,
            growth: benchmark::domain::growth::types::CorpusGrowth::Grown(
                benchmark::domain::growth::types::GrowthSource {
                    bench: &bench,
                    rules: &bench.join(constants::RULES_DIRECTORY),
                    table: &bench.join(constants::GROWTH_TABLE),
                    parser: &parser,
                },
            ),
            verification: benchmark::inbound::use_cases::measure_corpus::types::VerifiedInventory {
                directory: &verification,
                rules: &verified,
            },
            cache: &cache,
            frozen_workload: &workload,
            certificate: &certificate,
            mode: &mode,
            cohort: &cohort,
            rule_context: &rule_context,
            search_roots: &search_roots,
            context_mode,
            language: CorpusLanguage {
                id: &language,
                display: LanguageTitle(constants::LANGUAGE_DISPLAY),
                reference_display: ToolTitle(constants::REFERENCE_DISPLAY),
                suffixes: &suffixes,
                invalid_syntax: &invalid_syntax,
                valid_profile: ProfileLabel(constants::VALID_SOURCE_PROFILE),
            },
        },
        observations,
    )
}
