//! Minimal process composition; contestant clocks start inside the direct observer.
use ryo_benchmark::{
    inbound::use_cases::collect::{interface::CollectUseCase, use_case::CollectUseCaseImpl},
    outbound::{collection, execution},
};
use std::{
    io::{Read, Write},
    sync::Arc,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
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
