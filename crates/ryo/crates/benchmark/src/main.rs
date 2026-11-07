//! Minimal process composition; contestant clocks start inside the direct observer.
//!
//! For the shipped recording benchmark, check `cargo metadata --locked --no-deps --format-version 1`: package `idiomatic-sh-ryo` has target `lint`, kind `bench`, source `cli/benches/lint.rs` (`harness = false`). Build it with `cargo bench --locked --no-run -p idiomatic-sh-ryo --bench lint`, seal the reported bench executable, then run that executable with `--bench --rounds 1`. The ordinary `ryo` CLI binary is a different target and does not accept those arguments.
//!
//! The existing `ryo-benchmark-collector` also accepts `verify-inventory <registration.json> <output> <timeout-seconds>`. A linter adapter derives that temporary registration from its pinned `benchmark.toml` and canonical `rules/<ID>/{query.ryoql,cases.toml}` convention, supplying native verifier arguments and exact case identities. The shared domain initializes every default as FAIL; the application invokes the existing verification path for each implementation, retains native observations, and continues after missing or failed rules. The injected artifact boundary atomically publishes `summary.json` and raw per-rule evidence. PASS plus FAIL always equals the supplied inventory length; no provider-specific count or admission list exists in the runner. Existing collector stdin measurement behavior is unchanged when no command is supplied.
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
use ryo_benchmark::{
    inbound::use_cases::{
        collect::{
            constants as collect_constants, interface::CollectUseCase as _,
            use_case::CollectUseCaseImpl,
        },
        validate_case_receipts::{
            constants as validate_constants, interface::ValidateCaseReceiptsUseCase as _,
            use_case::ValidateCaseReceiptsUseCaseImpl,
        },
        verify_inventory::{
            constants as inventory_constants, interface::VerifyInventoryUseCase as _,
            types::InventoryVerdict, use_case::VerifyInventoryUseCaseImpl,
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
        let validate = ValidateCaseReceiptsUseCaseImpl::new(Arc::new(inventory::factory::local()));
        let audit = validate.validate_case_receipts(
            std::path::Path::new(&arguments[1]),
            std::path::Path::new(&arguments[2]),
        )?;
        std::io::stdout().write_all(&store.audit_report(&audit)?)?;
        return if audit.errors.is_empty() && audit.verified.len() == audit.case_count.0 {
            Ok(())
        } else {
            Err(validate_constants::FAILED.into())
        };
    }
    if arguments
        .first()
        .is_some_and(|argument| argument == inventory_constants::COMMAND)
    {
        if arguments.len() != 4 {
            return Err(inventory_constants::USAGE.into());
        }
        let seconds = arguments[3]
            .to_str()
            .ok_or(inventory_constants::TIMEOUT_NOT_UTF8)?
            .parse::<f64>()?;
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err(inventory_constants::TIMEOUT_INVALID.into());
        }
        let verify = VerifyInventoryUseCaseImpl::new(
            Arc::new(execution::factory::bounded(
                std::time::Duration::try_from_secs_f64(seconds)?,
            )),
            Arc::new(inventory::factory::local()),
        );
        return if verify.verify_inventory(
            std::path::Path::new(&arguments[1]),
            std::path::Path::new(&arguments[2]),
        )? == InventoryVerdict::Passed
        {
            Ok(())
        } else {
            Err(inventory_constants::FAILED.into())
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
