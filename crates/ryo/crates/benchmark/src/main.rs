//! Minimal process composition; contestant clocks start inside the direct observer.
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
