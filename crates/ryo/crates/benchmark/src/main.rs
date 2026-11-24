//! Minimal process composition; contestant clocks start inside the direct observer.
//!
//! The collector runs a language bench: `bench`, with one argument, the bench directory
//! `languages/<lang>/benchmarks/linters/<linter>`. It refuses any count of processors but four, builds and seals the bench's
//! worker and itself under `<target>/benchmark/<lang>/<linter>/bench/`, registers every default from the bench's
//! `benchmark.toml` and `rules/<ID>/{query.ryoql,cases.toml}` and verifies each against its cases once, and only when every
//! default passed runs the worker's `measure-valid` over the language's projects, which writes the bench's one document. The shared domain initializes every default as FAIL; the application invokes the existing verification path for each implementation, retains native observations, and continues after missing or failed rules. A default the profile defers (its `[deferred]` table) and does not implement is DEFERRED with its capability as the reason and runs nothing; the run passes when no row is FAIL. The injected artifact boundary atomically publishes `summary.json` (`benchmark-inventory/v2`) and raw per-rule evidence. PASS plus FAIL plus DEFERRED always equals the supplied inventory length; no provider-specific count or admission list exists in the runner. Existing collector stdin measurement behavior is unchanged when no command is supplied.
//!
//! ## Explicit reference process failures
//!
//! An authored `reference-discrepancy.process` contract is tagged `kind = "process-failure"` and declares a positive numeric exit, whole stdout/stderr templates, and two distinct archived observations for each exact executable pin. Template parts are literal UTF8 bytes or declared input paths encoded as UTF8 bytes or JSON string content. Existing independent semantic evidence and source/settings/profile/output identities remain mandatory. Signals, spawn failures, deadline observers, unexpected decoded success, and any extra output remain failures. The candidate must still emit its exact intended diagnostics with complete execution accounting.
//!
//! Verification preserves raw bytes and invocation metadata beside the explicit receipt file in fresh receipt-scoped attempt directories. Unreferenced attempts and resume history remain immutable. Offline validation uses the same shared comparison:
//!
//! ```text
//! ryo-benchmark-collector validate-case-receipts registration.json shard.cases.jsonl
//! ```
//!
//! The command returns verified identities and explicit errors; process differences count within documented discrepancies and never as raw full-report equality. Authoring adapters delegate to this command using a pinned validator executable. They do not match process failures themselves.
//!
//! For authoring tools that delegate process-receipt checks, set both `RYO_BENCHMARK_RECEIPT_VALIDATOR` to the absolute path of this collector and `RYO_BENCHMARK_RECEIPT_VALIDATOR_SHA256` to the independently recorded SHA256 of its bytes. Missing or incorrect configuration fails receipt validation and is retained in the shard error; it never falls back to local matching. A shard with no receipt is unfinished and is rerun on resume.
use ryo_benchmark::domain::operations::types::{BenchDirectory, BenchOperation};
use ryo_benchmark::{
    inbound::use_cases::{
        collect::{
            constants as collect_constants, interface::CollectUseCase as _,
            use_case::CollectUseCaseImpl,
        },
        operate::{
            constants as operate_constants,
            interface::OperateUseCase as _,
            types::{OperationRequest, OperationVerdict},
            use_case::OperateUseCaseImpl,
        },
        validate_case_receipts::{
            constants as validate_constants, interface::ValidateCaseReceiptsUseCase as _,
            use_case::ValidateCaseReceiptsUseCaseImpl,
        },
    },
    outbound::{
        collection, execution,
        inventory::{self, interface::InventoryPort as _},
    },
};
use std::{
    io::{Read as _, Write as _},
    sync::Arc,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|argument| argument == validate_constants::COMMAND)
    {
        if arguments.len() != 3 {
            return Err(validate_constants::USAGE.into());
        }
        let store = inventory::factory::local();
        let validate = ValidateCaseReceiptsUseCaseImpl::new(
            Arc::new(ryo_benchmark::outbound::receipts::factory::local(
                ryo_benchmark::outbound::receipts::types::CaseReceipts::Unrecorded,
            )),
            Arc::new(inventory::factory::local()),
        );
        let audit = validate.validate_case_receipts(
            std::path::Path::new(&arguments[1]),
            std::path::Path::new(&arguments[2]),
        )?;
        std::io::stdout().write_all(&store.audit_report(&audit)?.0)?;
        return if audit.errors.is_empty() && audit.verified.len() == audit.case_count.0 {
            Ok(())
        } else {
            Err(validate_constants::FAILED.into())
        };
    }
    if let Some(operation) = arguments
        .first()
        .and_then(|argument| argument.to_str())
        .map(ryo_benchmark::domain::operations::types::OperationVerb)
        .and_then(ryo_benchmark::domain::operations::service::operation)
    {
        let profiling = operation == BenchOperation::Profile;
        let (bench, other) = match arguments.as_slice() {
            [_verb, bench] => (bench, None),
            [_verb, bench, other] if profiling => (bench, Some(other.into())),
            _ => return Err(operate_constants::USAGE.into()),
        };
        let executables = Arc::new(ryo_benchmark::outbound::executables::factory::cargo(
            operate_constants::BUILD_TOOL.into(),
            std::env::join_paths(
                // ignore: idiomatic.arch/no-env-reads - the reference tool is the first of its name in the starting build tool's toolchain, then on the search path, the operating system's convention for finding a command
                std::env::var_os(operate_constants::TOOLCHAIN_TOOL)
                    .map(std::path::PathBuf::from)
                    .and_then(|tool| tool.parent().map(std::path::Path::to_path_buf))
                    .into_iter()
                    .chain(std::env::split_paths(
                        &std::env::var_os(operate_constants::SEARCH_PATH).unwrap_or_default(),
                    )),
            )?,
        ));
        let bench_path = std::path::Path::new(bench);
        if profiling && ryo_benchmark::application::check_profile::registered(bench_path) {
            let target = ryo_benchmark::outbound::executables::interface::ExecutablesPort::target(
                executables.as_ref(),
            )?;
            ryo_benchmark::application::check_profile::profile(
                bench_path,
                &target,
                other.as_deref(),
                &mut std::io::stdout().lock(),
                &mut std::io::stderr().lock(),
            )?;
            return Ok(());
        }
        let operate = OperateUseCaseImpl::new(
            executables,
            Arc::new(ryo_benchmark::outbound::suite::factory::local()),
            Arc::new(inventory::factory::local()),
            Arc::new(ryo_benchmark::outbound::receipts::factory::local(
                ryo_benchmark::outbound::receipts::types::CaseReceipts::Unrecorded,
            )),
            Arc::new(execution::factory::bounded(std::time::Duration::from_secs(
                operate_constants::RULE_TIMEOUT_SECONDS,
            ))),
            Arc::new(ryo_benchmark::outbound::scheduler::factory::threads()),
            Arc::new(ryo_benchmark::outbound::host::factory::local()),
            // The worker's document is read whole and the operation's class kept; nothing is written to these streams.
            Arc::new(ryo_benchmark::outbound::evidence::factory::bmf(
                Box::new(std::io::sink()),
                Box::new(std::io::sink()),
            )),
        );
        let request = OperationRequest {
            operation,
            bench: BenchDirectory(bench.into()),
        };
        let mut out = std::io::stdout().lock();
        let mut log = std::io::stderr().lock();
        let report = if profiling {
            operate.profile(&request, other, &mut out, &mut log)?
        } else {
            operate.operate(&request, &mut out, &mut log)?
        };
        return if report.verdict == OperationVerdict::Passed {
            Ok(())
        } else {
            Err(operate_constants::FAILED.into())
        };
    }
    if !arguments.is_empty() {
        return Err(collect_constants::UNKNOWN_COMMAND.into());
    }
    let mut request = Vec::new();
    let _read = std::io::stdin().read_to_end(&mut request)?;
    let collect = CollectUseCaseImpl::new(
        Arc::new(collection::factory::json()),
        Arc::new(execution::factory::direct()),
    );
    let response = collect.collect(&collection::types::CollectorMessage(request))?;
    std::io::stdout().write_all(&response.0)?;
    Ok(())
}
