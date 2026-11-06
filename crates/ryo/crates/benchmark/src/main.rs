//! Minimal process composition; contestant clocks start inside the direct observer.
//!
//! For the shipped Rust recording benchmark, check `cargo metadata --locked --no-deps --format-version 1`: package `idiomatic-sh-ryo` has target `lint`, kind `bench`, source `cli/benches/lint.rs` (`harness = false`). Build it with `cargo bench --locked --no-run -p idiomatic-sh-ryo --bench lint`, seal the reported bench executable, then run that executable with `--bench --rounds 1`. The ordinary `ryo` CLI binary is a different target and does not accept those arguments.
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
    inbound::use_cases::collect::{interface::CollectUseCase as _, use_case::CollectUseCaseImpl},
    outbound::{collection, execution},
};
use std::{
    io::{Read as _, Write as _},
    sync::Arc,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|argument| argument == "validate-case-receipts")
    {
        use ryo_benchmark::inbound::use_cases::validate_case_receipts::{
            interface::ValidateCaseReceiptsUseCase as _, use_case::ValidateCaseReceiptsUseCaseImpl,
        };
        if arguments.len() != 3 {
            return Err("usage: validate-case-receipts <registration.json> <receipt.jsonl>".into());
        }
        let validate = ValidateCaseReceiptsUseCaseImpl::new(Arc::new(
            ryo_benchmark::outbound::inventory::factory::local(),
        ));
        let audit = validate.validate_case_receipts(
            std::path::Path::new(&arguments[1]),
            std::path::Path::new(&arguments[2]),
        )?;
        let mut result = serde_json::json!({
            "schema":"benchmark-case-receipt-audit/v1", "case_count":audit.case_count,
            "validated_case_ids":audit.verified.iter().map(|(rule,name)|serde_json::json!({"rule":rule,"name":name})).collect::<Vec<_>>(),
            "raw_exact":audit.raw_exact,"documented_discrepancies":audit.documented_discrepancies,"contract_only":audit.contract_only,"errors":audit.errors,
        });
        if !audit.metadata_verified.is_empty() {
            result["file_metadata_verified_cases"] = serde_json::json!(audit.metadata_verified.iter().map(|(rule,name,digest)|serde_json::json!({"rule":rule,"name":name,"sha256":digest.0})).collect::<Vec<_>>());
        }
        if !audit.tool_contract_differences.is_empty() {
            result["tool_contract_difference_case_ids"] = serde_json::json!(
                audit
                    .tool_contract_differences
                    .iter()
                    .map(|(rule, name)| serde_json::json!({"rule":rule,"name":name}))
                    .collect::<Vec<_>>()
            );
        }
        serde_json::to_writer(std::io::stdout(), &result)?;
        std::io::stdout().write_all(b"\n")?;
        return if audit.errors.is_empty() && audit.verified.len() == audit.case_count {
            Ok(())
        } else {
            Err("case receipts failed strict verification".into())
        };
    }
    if arguments
        .first()
        .is_some_and(|argument| argument == "verify-inventory")
    {
        use ryo_benchmark::inbound::use_cases::verify_inventory::{
            interface::VerifyInventoryUseCase as _, use_case::VerifyInventoryUseCaseImpl,
        };
        if arguments.len() != 4 {
            return Err(
                "usage: verify-inventory <registration.json> <output> <timeout-seconds>".into(),
            );
        }
        let seconds = arguments[3]
            .to_str()
            .ok_or("timeout is not UTF-8")?
            .parse::<f64>()?;
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err("timeout must be finite and positive".into());
        }
        let verify = VerifyInventoryUseCaseImpl::new(
            Arc::new(execution::factory::bounded(
                std::time::Duration::try_from_secs_f64(seconds)?,
            )),
            Arc::new(ryo_benchmark::outbound::inventory::factory::local()),
        );
        return if verify.verify_inventory(
            std::path::Path::new(&arguments[1]),
            std::path::Path::new(&arguments[2]),
        )? {
            Ok(())
        } else {
            Err("inventory contains failed rules; see summary.json".into())
        };
    }
    if !arguments.is_empty() {
        return Err("unknown benchmark command".into());
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
